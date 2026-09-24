#![cfg(test)]

use phx_core::{
    Address, Audience, Calendar, Concerns, DayMessages, Directory, EventStore, Findings, InjectTarget, Lag, Message,
    NewEvent, ReadTrace, RecordKindDecl, RecordStore, Register, RegisterBuilder, SubStep,
};
use phx_id::{Date, Day, PartyId, RowRef, Slot, TableId};
use phx_num::Missing;
use phx_rand::{Subject, SubjectTag};
use phx_store::AddressSpace;

use crate::runner::{Audit, CloseInputs};
use crate::{NAMES, TIME, kernel_families};

const KIND: RecordKindDecl = RecordKindDecl {
    name: "DEM.births",
    audience: Audience::Public,
    horizon: Lag::Months(12),
    writer: "DEM",
    clause: "POP.1",
};

/// Values handed to the audit alone: a directory, records, events and the day's messages, never a world.
struct Hand {
    day: Day,
    register: Register,
    calendar: Calendar,
    directory: Directory,
    records: RecordStore,
    events: EventStore,
    messages: DayMessages,
    trace: Option<ReadTrace>,
}

impl Hand {
    fn new() -> Hand {
        let mut space = AddressSpace::empty();
        Hand {
            day: Day::new(100),
            register: RegisterBuilder::default().build(&[], 0).unwrap(),
            calendar: Calendar::new(Date::new(1950, 1, 1).unwrap(), Vec::new(), 1950).unwrap(),
            directory: Directory::new(),
            records: RecordStore::new(&mut space, vec![KIND], 64, 1 << 12),
            events: EventStore::new(&mut space, 64, 16, 1 << 12),
            messages: DayMessages::default(),
            trace: None,
        }
    }

    fn party(&mut self) -> PartyId {
        let slot = Slot::new(u32::try_from(self.directory.next()).unwrap());
        self.directory.begin(RowRef { table: TableId::new(0), slot })
    }

    fn record(&mut self, subject: PartyId) {
        self.records.write(KIND.name, subject, self.day, SubStep::S5c, &[]);
    }

    /// The families' findings at a close, by family, in order.
    fn audit(&self) -> Vec<&'static str> {
        let mut audit = Audit::new(kernel_families()).unwrap();
        let mut findings = Findings::default();
        let close = CloseInputs {
            day: self.day,
            register: &self.register,
            directory: &self.directory,
            calendar: &self.calendar,
            records: &self.records,
            events: &self.events,
            messages: &self.messages,
            tables: &[],
            trace: self.trace,
            books: &NoBooks,
            markets: &NoMarkets,
            accounts: &NoAccounts,
        };
        let record = audit.close(close, &mut findings);
        assert_eq!((record.families, record.findings), (2, findings.len()));
        findings.all().iter().map(|f| f.family).collect()
    }
}

/// Books with nothing in them, for a close whose families read none.
/// Accounts with no party.
#[derive(Debug)]
struct NoAccounts;

impl phx_core::AccountsAudit for NoAccounts {
    fn parties(&self) -> usize {
        0
    }
    fn equity(&self, _: usize) -> Vec<phx_core::Gap> {
        Vec::new()
    }
    fn claims(&self) -> (u64, Vec<phx_core::Gap>) {
        (0, Vec::new())
    }
    fn periods(&self, _: phx_id::Day) -> (u64, Vec<phx_core::Gap>) {
        (0, Vec::new())
    }
}

/// A tape with nothing on it.
#[derive(Debug)]
struct NoMarkets;

impl phx_core::MarketsAudit for NoMarkets {
    fn prices(&self, _: phx_id::Day) -> (u64, Vec<phx_core::Gap>) {
        (0, Vec::new())
    }
}

#[derive(Debug)]
struct NoBooks;

impl phx_core::BooksAudit for NoBooks {
    fn instruments(&self) -> usize {
        0
    }
    fn lines(&self) -> usize {
        0
    }
    fn ownership(&self, _: usize) -> Vec<phx_core::Gap> {
        Vec::new()
    }
    fn contracts(&self, _: usize) -> Vec<phx_core::Gap> {
        Vec::new()
    }
    fn money(&self, _: usize) -> Vec<phx_core::Gap> {
        Vec::new()
    }
    fn position(&self, _: PartyId, _: u64) -> i64 {
        0
    }
}

impl InjectTarget for Hand {
    fn day(&self) -> Day {
        self.day
    }

    fn unhanded_party(&self) -> PartyId {
        PartyId::new(self.directory.next())
    }

    fn live_party(&self) -> Option<PartyId> {
        (1..self.directory.next())
            .map(PartyId::new)
            .find(|p| matches!(self.directory.resolve(*p), phx_core::Resolved::Live(..)))
    }

    fn add_record(&mut self, subject: PartyId, day: Day, substep: SubStep) -> Result<(), String> {
        self.records.write(KIND.name, subject, day, substep, &[]);
        Ok(())
    }

    fn fact(&self, _: &str, _: &'static str, _: phx_id::Slot) -> phx_num::Missing<i64> {
        phx_num::Missing::Absent
    }

    fn set_fact(&mut self, table: &str, _: &'static str, _: phx_id::Slot, _: i64) -> Result<(), String> {
        Err(format!("the hand keeps no table `{table}`"))
    }
}

#[test]
fn names_refuses_dangling() {
    let mut hand = Hand::new();
    let alive = hand.party();
    hand.record(alive);
    assert!(hand.audit().is_empty());
    hand.record(PartyId::new(99));
    let subjects = [Subject::new(SubjectTag::Party, 77), Subject::new(SubjectTag::Tile, 5)];
    let event = NewEvent {
        day: hand.day,
        substep: SubStep::S3a,
        kind: 0,
        subjects: &subjects,
        details: &[],
        public: true,
        develops_from: Missing::Absent,
    };
    hand.events.record(event);
    let message = Message::new(
        0,
        Address::party(alive),
        Address::cell(PartyId::new(55), 3),
        hand.day,
        hand.day,
        Concerns::Nothing,
    );
    hand.messages.push(message);
    assert_eq!(hand.audit(), [NAMES.name; 3], "a record, an event and a message each name a party that never was");
}

#[test]
fn names_follows_successors() {
    let mut hand = Hand::new();
    let (heir, estate, gone) = (hand.party(), hand.party(), hand.party());
    let died = hand.party();
    for p in [died, estate, gone] {
        hand.directory.retain(p);
        hand.record(p);
    }
    hand.directory.end(died, hand.day, Missing::Present(heir));
    hand.directory.end(estate, hand.day, Missing::Absent);
    assert!(hand.audit().is_empty(), "a successor that lives and an estate that distributed both answer");
    hand.directory.end(gone, hand.day, Missing::Absent);
    hand.directory.release(gone);
    assert_eq!(
        hand.audit(),
        [NAMES.name],
        "a party nothing names any longer is forgotten, and a record naming it dangles"
    );
}

#[test]
fn time_refuses_what_the_day_did_not_write() {
    let mut hand = Hand::new();
    let p = hand.party();
    hand.records.write(KIND.name, p, hand.day, SubStep::S10e, &[]);
    hand.records.write(KIND.name, p, hand.day.succ(), SubStep::S1a, &[]);
    hand.trace = Some(ReadTrace { undeclared_reads: 0, later_writes: 2, duplicate_opens: 0 });
    assert_eq!(hand.audit(), [TIME.name; 3]);
}

#[test]
fn injections_light_their_family_alone() {
    for family in kernel_families() {
        let mut hand = Hand::new();
        let p = hand.party();
        hand.record(p);
        assert!(hand.audit().is_empty());
        family.inject(&mut hand).unwrap();
        assert_eq!(hand.audit(), [family.decl().name], "{} lit alone", family.decl().name);
    }
}

#[test]
fn a_family_declared_twice_is_refused() {
    let mut families = kernel_families();
    families.extend(kernel_families());
    assert_eq!(Audit::new(families).unwrap_err().len(), 2);
}

mod digests {
    use phx_core::LegDigest;
    use phx_id::PartyId;

    use crate::records::{Digests, Gap};

    fn leg(party: u64, account: u64, qty: i64, before: i64, money: bool) -> LegDigest {
        LegDigest { party: PartyId::new(party), account, denom: 0, qty, before, paired: true, money }
    }

    #[test]
    fn records_find_what_the_books_do_not_hold() {
        let mut d = Digests::default();
        d.record(1, leg(4, 8, -200, 1_000, true));
        d.record(1, leg(5, 8, 200, 1_000, true));
        d.record(2, leg(4, 8, -50, 800, true));
        d.record(2, leg(6, 16, 50, 0, false));
        assert!(d.flow_gaps().is_empty(), "each instruction has both its sides");
        assert_eq!(d.money_gaps(), vec![Gap::Money { instruction: 2, ccy: 0, sum: -50 }], "money gone to a loan row");
        let books = |p: PartyId, _: u64| match p.get() {
            4 => 750,
            5 => 1_200,
            _ => 49,
        };
        let gaps = d.unit_gaps(&books);
        assert_eq!(gaps, vec![Gap::Units { party: PartyId::new(6), account: 16, expected: 50, held: 49 }]);
    }
}
