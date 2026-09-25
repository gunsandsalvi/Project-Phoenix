use phx_id::{Day, PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::calendar::Calendar;
use crate::columns::{FactColumns, KernelTable};
use crate::directory::Directory;
use crate::events::EventStore;
use crate::findings::Findings;
use crate::messages::DayMessages;
use crate::records::RecordStore;
use crate::register::Register;
use crate::substep::SubStep;
use crate::touched::TouchedRows;

/// How an audit family checks: on every apply; on the rows the day touched and wrote; or on those and, besides, one
/// slice a day of its whole domain, covering it in a cycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FamilyMode {
    Streaming,
    Incremental,
    Rolling { cycle_days: u16 },
}

/// An audit family: its name, owner, clause and mode, which every family states.
#[clause("N1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FamilyDecl {
    pub name: &'static str,
    pub owner: &'static str,
    pub clause: &'static str,
    pub mode: FamilyMode,
}

/// Where the audit reads what the day left behind: after the day's last write, before the read-only tracers and views.
pub const AUDIT_SUBSTEP: SubStep = SubStep::S10d;

/// A run of row indices, `start` included and `end` not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn iter(self) -> impl Iterator<Item = usize> {
        self.start..self.end
    }
}

/// The day's share of a rolling cycle over `len` rows in slot order: the cycle's days cut the rows into equal runs,
/// and each day takes its own.
#[must_use]
pub fn rolling_slice(len: usize, cycle_days: u16, day: Day) -> Span {
    if cycle_days == 0 {
        violation!(clause = "N1", "a rolling cycle of no days");
    }
    let cycle = usize::from(cycle_days);
    let Ok(k) = usize::try_from(day.get() % u32::from(cycle_days)) else {
        violation!(clause = "N1", "a cycle position beyond the machine's words");
    };
    Span { start: len * k / cycle, end: len * (k + 1) / cycle }
}

/// What `read-trace` found on a day, when the run traces reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct ReadTrace {
    pub undeclared_reads: usize,
    pub later_writes: usize,
    pub duplicate_opens: usize,
}

/// Something a family found wrong: whose it is, by how much and in what, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub owner: crate::findings::FindingOwner,
    pub size: i128,
    pub unit: crate::findings::Unit,
    pub detail: String,
}

/// The world's books as the audit reads them: its instruments and lines by index, each checked by the ledger that
/// keeps them, and what a party holds on an account. The ledger implements it, so the audit need not name the ledger.
pub trait BooksAudit: core::fmt::Debug {
    fn instruments(&self) -> usize;
    fn lines(&self) -> usize;
    /// An instrument's holdings against its issued amount.
    fn ownership(&self, instrument: usize) -> Vec<Gap>;
    /// A line's two sides against each other and against its holders' rows.
    fn contracts(&self, line: usize) -> Vec<Gap>;
    /// The money lines of a span of lines: each one's holders' balances against its issuer's, the span read together
    /// so a side of many small holders that keeps no list is summed in one pass; nothing for a line that is not money.
    fn money(&self, lines: core::ops::Range<usize>) -> Vec<Gap>;
    /// What a party holds on an account, by the account's code.
    fn position(&self, party: PartyId, account: u64) -> i64;
}

/// The markets' public tape as the audit reads it: the prints, marks and fixings the markets published on a day,
/// each checked against the match sets it came from. The market kernel implements it, so the audit need not name it.
pub trait MarketsAudit: core::fmt::Debug {
    /// What the day published and what it gets wrong: the entries checked, and each gap.
    fn prices(&self, day: Day) -> (u64, Vec<Gap>);
}

/// The parties' accounts as the audit reads them: each party's equity account against a read of its positions at
/// their carrying values, the receivables against the payables line by line, and each period's income against its
/// equity's change. The accounts kernel implements it over the books.
pub trait AccountsAudit: core::fmt::Debug {
    /// Parties with an equity account, by index.
    fn parties(&self) -> usize;
    /// One party's equity account against its positions, or its read as unreadable.
    fn equity(&self, party: usize) -> Vec<Gap>;
    /// The receivables against the payables, line by line: the lines checked, and each gap.
    fn claims(&self) -> (u64, Vec<Gap>);
    /// Each closed period's income against its equity's change: the parties checked, and each gap.
    fn periods(&self, day: Day) -> (u64, Vec<Gap>);
}

/// The population's agents as the audit reads them: each agent's multiplicity against its contracts, and each
/// population's multiplicities against it. The population kernel implements it over its agent tables.
pub trait AgentsAudit: core::fmt::Debug {
    /// Agents' slots over every population table, by index.
    fn agents(&self) -> usize;
    /// One agent's multiplicity and each of its rows' counts against it.
    fn agent(&self, at: usize) -> Vec<Gap>;
    /// Each population's agents' multiplicities against its population.
    fn populations(&self) -> Vec<Gap>;
}

/// The audit's own record of the day's settled legs, kept apart from the books they moved.
pub trait LegRecords: core::fmt::Debug {
    /// The instructions recorded today, each in each denomination it moved.
    fn instructions(&self) -> u64;
    /// The positions the day's legs moved.
    fn positions(&self) -> u64;
    /// Instructions whose paired legs do not sum to nothing in a denomination.
    fn flow_gaps(&self) -> Vec<Gap>;
    /// Instructions that changed a currency's money without its issuer's matching leg.
    fn money_gaps(&self) -> Vec<Gap>;
    /// Positions whose holding before the day's first leg and the day's legs do not make what the books hold.
    fn unit_gaps(&self, books: &dyn BooksAudit) -> Vec<Gap>;
}

/// Everything the audit reads at a close, as shared borrows: the world's stores, the rows the day touched, and the
/// records and events written since the last close.
#[derive(Clone, Copy, Debug)]
pub struct AuditInputs<'a> {
    pub day: Day,
    pub register: &'a Register,
    pub directory: &'a Directory,
    pub calendar: &'a Calendar,
    pub records: &'a RecordStore,
    pub events: &'a EventStore,
    pub messages: &'a DayMessages,
    pub tables: &'a [KernelTable],
    pub touched: &'a TouchedRows,
    pub trace: Option<ReadTrace>,
    pub new_records: Span,
    pub new_events: Span,
    pub books: &'a dyn BooksAudit,
    pub legs: &'a dyn LegRecords,
    pub markets: &'a dyn MarketsAudit,
    pub accounts: &'a dyn AccountsAudit,
    pub agents: &'a dyn AgentsAudit,
    /// Each system's own state, by its code, as its handlers read it.
    pub own: &'a [(&'static str, Box<dyn core::any::Any + Send + Sync>)],
}

/// What a family may read: only through `&self`, so checking changes nothing.
#[derive(Clone, Copy, Debug)]
pub struct FamilyCtx<'a> {
    inputs: AuditInputs<'a>,
    mode: FamilyMode,
}

impl<'a> FamilyCtx<'a> {
    #[must_use]
    pub fn new(inputs: AuditInputs<'a>, mode: FamilyMode) -> FamilyCtx<'a> {
        FamilyCtx { inputs, mode }
    }

    pub fn day(&self) -> Day {
        self.inputs.day
    }

    /// The sub-step the audit runs in; everything it reads was written before it.
    #[must_use]
    pub fn substep(&self) -> SubStep {
        AUDIT_SUBSTEP
    }

    #[must_use]
    pub fn register(&self) -> &Register {
        self.inputs.register
    }

    #[must_use]
    pub fn directory(&self) -> &Directory {
        self.inputs.directory
    }

    #[must_use]
    pub fn calendar(&self) -> &Calendar {
        self.inputs.calendar
    }

    #[must_use]
    pub fn records(&self) -> &RecordStore {
        self.inputs.records
    }

    #[must_use]
    pub fn events(&self) -> &EventStore {
        self.inputs.events
    }

    #[must_use]
    pub fn messages(&self) -> &DayMessages {
        self.inputs.messages
    }

    /// A table the kernel keeps, which the family must name rightly.
    #[must_use]
    pub fn table(&self, name: &str) -> &FactColumns {
        let Some(t) = self.inputs.tables.iter().find(|t| t.name == name) else {
            violation!(clause = "N1", "a family reading a table the world does not keep");
        };
        &t.columns
    }

    /// The rows of a table the day's applies touched, in slot order.
    pub fn touched(&self, table: TableId) -> impl Iterator<Item = Slot> + '_ {
        self.inputs.touched.rows(table)
    }

    /// The day's read trace, when the run traces reads.
    #[must_use]
    pub fn trace(&self) -> Option<ReadTrace> {
        self.inputs.trace
    }

    /// The records written since the last close.
    #[must_use]
    pub fn new_records(&self) -> Span {
        self.inputs.new_records
    }

    /// The events written since the last close.
    #[must_use]
    pub fn new_events(&self) -> Span {
        self.inputs.new_events
    }

    /// The world's books.
    #[must_use]
    pub fn books(&self) -> &dyn BooksAudit {
        self.inputs.books
    }

    /// A system's own state, as its handlers read it: the family of the system that keeps it reads it here.
    #[must_use]
    pub fn own<T: 'static>(&self, system: &str) -> Option<&T> {
        self.inputs.own.iter().find(|(code, _)| *code == system).and_then(|(_, s)| s.downcast_ref::<T>())
    }

    /// The parties' accounts.
    #[must_use]
    pub fn accounts(&self) -> &dyn AccountsAudit {
        self.inputs.accounts
    }

    /// The population's agents.
    #[must_use]
    pub fn agents(&self) -> &dyn AgentsAudit {
        self.inputs.agents
    }

    /// The markets' public tape.
    #[must_use]
    pub fn markets(&self) -> &dyn MarketsAudit {
        self.inputs.markets
    }

    /// The audit's own record of the day's settled legs.
    #[must_use]
    pub fn legs(&self) -> &dyn LegRecords {
        self.inputs.legs
    }

    /// The family's slice today of a domain of `len` rows; only a rolling family has one.
    #[must_use]
    pub fn rolling(&self, len: usize) -> Span {
        let FamilyMode::Rolling { cycle_days } = self.mode else {
            violation!(clause = "N1", "a rolling slice read by a family that does not roll");
        };
        rolling_slice(len, cycle_days, self.inputs.day)
    }
}

/// A save loaded apart for the audit alone, which one family's injection changes by one unit before the audit reads
/// it; never the world that runs.
pub trait InjectTarget {
    fn day(&self) -> Day;
    /// An identity the directory never handed out.
    fn unhanded_party(&self) -> PartyId;
    /// A party the directory holds live, if any.
    fn live_party(&self) -> Option<PartyId>;
    /// Adds an entry about a party to one of the save's record kinds, dated as given.
    ///
    /// # Errors
    /// When the save declares no record kind.
    fn add_record(&mut self, subject: PartyId, day: Day, substep: SubStep) -> Result<(), String>;
    /// A fact of a kernel table's row as the save holds it.
    fn fact(&self, table: &str, fact: &'static str, slot: Slot) -> Missing<i64>;
    /// Sets a fact of a kernel table's row in the save.
    ///
    /// # Errors
    /// When the save keeps no such table or row.
    fn set_fact(&mut self, table: &str, fact: &'static str, slot: Slot, value: i64) -> Result<(), String>;
    /// Records an event naming a subject, dated as given.
    ///
    /// # Errors
    /// When the save declares no event kind.
    fn add_event(&mut self, subject: phx_rand::Subject, day: Day, substep: SubStep) -> Result<(), String>;
    /// The save's books, as the ledger's type, which the ledger's families name.
    fn books(&mut self) -> &mut dyn core::any::Any;
    /// The save's markets, as their type, which the markets' family names.
    fn markets(&mut self) -> &mut dyn core::any::Any;
    /// The save's accounts, as their type, which the accounts' families name.
    fn accounts(&mut self) -> &mut dyn core::any::Any;
    /// The save's agent tables, as their type, which the population's families name.
    fn agents(&mut self) -> &mut dyn core::any::Any;
    /// A system's own state, which that system's families name.
    fn own(&mut self, system: &str) -> Option<&mut dyn core::any::Any>;
    /// The sink the audit reads the day's legs from, as if they had settled today.
    fn stream(&mut self) -> &mut dyn AuditStream;
}

/// An audit family: it reads the world through its context and writes only findings, never repairing, and says how
/// many rows it checked. Its injection is a discrepancy that it alone sees.
pub trait AuditFamily: Send + Sync {
    fn decl(&self) -> FamilyDecl;
    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64;
    /// # Errors
    /// When the save holds nothing the injection can change.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String>;
}

/// A settled leg as the audit keeps it, apart from the books it moved: whose, on which account and in which
/// denomination (each coded by the ledger), by how much, what it counts toward its instruction's balance (its quantity,
/// or a liability row's member count against the asset side's, as a line's sides open and close together), what the
/// account held before it, whether it is one of a pair, and whether it moved money on a money line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegDigest {
    pub party: PartyId,
    pub account: u64,
    pub denom: u32,
    pub qty: i64,
    pub flow: i64,
    pub before: i64,
    pub paired: bool,
    pub money: bool,
}

/// The sink the apply routine feeds as it applies, which the audit implements and the assembly injects, so no kernel
/// crate calls the audit above it.
pub trait AuditStream {
    fn applied(&mut self, instruction: u64);
    fn touched(&mut self, table: TableId, slot: Slot);
    /// A leg as it settles, for the families that check flows, money and units against the books.
    fn leg(&mut self, instruction: u64, leg: LegDigest);
}

#[cfg(test)]
mod tests {
    use phx_id::Day;

    use super::{Span, rolling_slice};

    #[test]
    fn rolling_slices_cover_everything() {
        for (len, cycle) in [(0, 3), (7, 3), (100, 30), (5, 30), (1000, 7)] {
            let mut covered = Vec::new();
            for d in 40..40 + u32::from(cycle) {
                covered.extend(rolling_slice(len, cycle, Day::new(d)).iter());
            }
            covered.sort_unstable();
            assert_eq!(covered, (0..len).collect::<Vec<_>>(), "{len} rows over {cycle} days");
        }
        assert_eq!(rolling_slice(10, 5, Day::new(7)), Span { start: 4, end: 6 });
    }
}
