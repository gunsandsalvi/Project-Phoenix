use phx_id::{Day, InstrumentId, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, UnitId, capacity_exceeded, violation};

use crate::algebra::Side;
use crate::covered::Covered;
use crate::line::NewRow;

/// An instruction's identity: its day in the high word and its place in that day's gather order below, so no two
/// instructions of the run share one.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstructionId(u64);

impl InstructionId {
    pub fn new(day: Day, seq: u32) -> InstructionId {
        InstructionId((u64::from(day.get()) << u32::BITS) | u64::from(seq))
    }

    #[must_use]
    pub fn get(self) -> u64 {
        self.0
    }
}

/// How a change of money shows in its party's accounts, as the reason declares it for each side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Revenue,
    Expense,
    Asset,
    Liability,
    Equity,
}

/// A reason an instruction is made, as the system that makes it declares it: its place in the declared payment order
/// within a sub-step, and its accounting effect on the side that pays and the side that receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReasonDecl {
    pub name: &'static str,
    pub order: u8,
    pub paid: Effect,
    pub received: Effect,
}

/// A declared reason.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReasonId(u16);

/// Every declared reason.
#[derive(Clone, Debug, Default)]
pub struct Reasons {
    decls: Vec<ReasonDecl>,
}

impl Reasons {
    pub fn declare(&mut self, decl: ReasonDecl) -> ReasonId {
        let Ok(index) = u16::try_from(self.decls.len()) else {
            capacity_exceeded!("reasons", u16::MAX, self.decls.len());
        };
        self.decls.push(decl);
        ReasonId(index)
    }

    #[must_use]
    pub fn get(&self, id: ReasonId) -> ReasonDecl {
        let Some(d) = self.decls.get(usize::from(id.0)) else {
            violation!(clause = "SET.1", "an instruction of an undeclared reason", reason = id.0);
        };
        *d
    }
}

/// What a leg moves: a row on one side of a line, units of an instrument, or a named unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccountRef {
    Line { line: LineId, side: Side },
    Instrument(InstrumentId),
    Unit(u64),
}

/// What a leg is counted in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Denom {
    Ccy(Ccy),
    Unit(UnitId),
}

/// What a row leg does to its party's row on a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOp {
    /// Adds the leg's quantity to the row's balance.
    Adjust,
    /// Opens the row; the leg's quantity is its member count.
    Open(NewRow),
    /// Retires the row; the leg's quantity is its member count.
    Close,
}

/// What accounts for a transformation's units: the way that produced them, the deposit they were taken from, or the
/// purchase, storage or hazard event that used them up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Way(u32),
    Deposit(u32),
    Purchase(u64),
    Storage(u64),
    Hazard(u64),
}

/// How a leg settles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegKind {
    /// Money moved on a money line's row.
    Money,
    /// Units of an instrument; a holder's units received come as a lot at `cost`, and the issuer's legs issue or
    /// retire units.
    Units { cost: i64 },
    /// A row opened, retired or adjusted on a line.
    Row(RowOp),
    /// Units made or used up, with what accounts for them; these legs alone need no other side.
    Transformation(Source),
    /// A balance or holding written by the opening before day one, naming the opening identity it served.
    OpeningWrite { identity: u64 },
}

/// One leg: its party, what it moves, by how much (received positive, given negative), in what, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegRec {
    pub party: PartyId,
    pub account: AccountRef,
    pub qty: i64,
    pub denom: Denom,
    pub kind: LegKind,
}

impl LegRec {
    /// Whether the leg is one of a pair: every leg but a transformation's or an opening write's.
    #[must_use]
    pub fn paired(&self) -> bool {
        !matches!(self.kind, LegKind::Transformation(_) | LegKind::OpeningWrite { .. })
    }

    /// Whether the leg moves money.
    #[must_use]
    pub fn moves_money(&self) -> bool {
        matches!(self.denom, Denom::Ccy(_)) && !matches!(self.kind, LegKind::OpeningWrite { .. })
    }
}

/// The row of a contract whose due an instruction pays: its line, and the side its payer holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueRow {
    pub line: LineId,
    pub side: Side,
}

/// A numbered instruction: its reason, its trade and settlement days, its legs, the contract row whose due it pays,
/// if any, and the covers of held units its trade's offer placed, released when it settles or fails.
#[clause("SET.1", "SET.2")]
#[derive(Debug)]
pub struct Instruction {
    pub id: InstructionId,
    pub reason: ReasonId,
    pub trade_day: Day,
    pub settle_day: Day,
    pub legs: Vec<LegRec>,
    pub pays: Missing<DueRow>,
    pub covers: Vec<Covered>,
}
