//! The offices institutions decide through: each a 16-byte row naming its institution, its kind among its legal form's
//! declared offices and its holder — the person who owns and manages it, or the appointment contract that names one —
//! or none while it is empty. An institution's offices lie in one block, opened at its founding in its form's declared
//! order and found from the first row its record keeps, so the office of a kind is the first plus the kind's offset.

use phx_id::{ContractLink, Day, PartyKey, PartyRef, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, Column, StoreStats, SystemBacking};

use crate::consts::{OFFICE_APPOINTED, OFFICE_FILLING, OFFICE_ROWS_PER_CHUNK, OFFICE_TERM_BOUND};
use crate::directory::Directory;

/// A word that holds nothing: an empty office's holder, an appointed office's start, a freed row's institution.
const NONE: u32 = u32::MAX;

/// An office's row: its institution, its kind, its flags, its holder (a person's slot, or an appointment's link where
/// the appointed flag is set) and the day an owner took it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct OfficeRow {
    institution: u32,
    kind: u16,
    flags: u16,
    holder: u32,
    since: u32,
}

/// An office: its row among the offices.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OfficeRef(Slot);

impl OfficeRef {
    pub const fn row(self) -> Slot {
        self.0
    }

    /// An office from the row its institution's record keeps.
    pub const fn at(row: Slot) -> OfficeRef {
        OfficeRef(row)
    }
}

/// Who holds an office: the person who owns and manages its institution, the appointment contract that names its
/// holder, whose person end the decision core reads, or no one while it is empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Owner(PartyRef),
    Appointment(ContractLink),
    Vacant,
}

/// The offices: their rows, the blocks freed for reuse by their length, and the persons' kind an owner is of.
#[clause("PTY.16", "MND.16")]
#[derive(Debug, phx_macros::Saved)]
pub struct Offices<B: Backing = SystemBacking> {
    rows: Column<OfficeRow, B>,
    /// Blocks freed by institutions that ended, each its first row and its length, for an institution founded with as
    /// many offices to take.
    free: Vec<(u32, u32)>,
    person_kind: u8,
}

fn whole(n: usize) -> u32 {
    u32::try_from(n).unwrap_or_else(|_| capacity_exceeded!("offices", u32::MAX, n))
}

impl<B: Backing> Offices<B> {
    /// Room for `capacity` offices, holders of the persons' kind.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, person_kind: u8, capacity: u32) -> Offices<B> {
        Offices { rows: Column::new(space, capacity, OFFICE_ROWS_PER_CHUNK), free: Vec::new(), person_kind }
    }

    fn row(&self, o: OfficeRef) -> OfficeRow {
        match self.rows.get(o.0) {
            Some(r) if r.institution != NONE => r,
            _ => violation!(clause = "PTY.16", "an office no institution holds", row = o.0.get()),
        }
    }

    fn write(&mut self, o: OfficeRef, f: impl FnOnce(&mut OfficeRow)) {
        let mut r = self.row(o);
        f(&mut r);
        self.rows.set(o.0, r);
    }

    /// An institution's offices opened at its founding, each empty, in its legal form's declared order: a freed block
    /// as long where one waits, else new rows. None for a form that declares no office.
    #[clause("PTY.16")]
    pub fn open(&mut self, institution: PartyKey, kinds: &[u16]) -> Missing<OfficeRef> {
        if kinds.is_empty() {
            return Missing::Absent;
        }
        let len = whole(kinds.len());
        let first = if let Some(i) = self.free.iter().rposition(|(_, l)| *l == len) {
            self.free.swap_remove(i).0
        } else {
            let first = whole(self.rows.len());
            for _ in kinds {
                self.rows.push(OfficeRow { institution: NONE, kind: 0, flags: 0, holder: NONE, since: NONE });
            }
            first
        };
        for (at, kind) in (first..).zip(kinds) {
            let row = OfficeRow { institution: institution.word(), kind: *kind, flags: 0, holder: NONE, since: NONE };
            self.rows.set(Slot::new(at), row);
        }
        Missing::Present(OfficeRef(Slot::new(first)))
    }

    /// An institution's office of a kind: its first plus the kind's offset among its form's offices. A row of another
    /// institution or kind stops the run.
    pub fn office(&self, first: OfficeRef, (offset, kind): (u16, u16), institution: PartyKey) -> OfficeRef {
        let Some(at) = first.0.get().checked_add(u32::from(offset)) else {
            capacity_exceeded!("offices", u32::MAX, first.0.get());
        };
        let o = OfficeRef(Slot::new(at));
        let r = self.row(o);
        if r.institution != institution.word() || r.kind != kind {
            violation!(clause = "PTY.16", "an office read past its institution's block", row = at);
        }
        o
    }

    /// Every office of the block an institution's first office begins, in its form's order.
    pub fn block_of(&self, first: OfficeRef) -> std::iter::Map<std::ops::Range<u32>, fn(u32) -> OfficeRef> {
        let (start, len) = self.block(first);
        (start..start + len).map(|at| OfficeRef(Slot::new(at)))
    }

    /// The block an institution's first office begins: its rows while they name the institution.
    fn block(&self, first: OfficeRef) -> (u32, u32) {
        let institution = self.row(first).institution;
        let start = first.0.get();
        let len = (start..whole(self.rows.len()))
            .take_while(|at| self.rows.get(Slot::new(*at)).is_some_and(|r| r.institution == institution))
            .count();
        (start, whole(len))
    }

    /// An office's holder, an owner's reference read from its slot's generation: its slot is never another's while it
    /// holds the office, a holder's death vacating it the same day.
    pub fn holder<D: Backing>(&self, o: OfficeRef, dir: &Directory<D>) -> Holder {
        let r = self.row(o);
        if r.holder == NONE {
            return Holder::Vacant;
        }
        if r.flags & OFFICE_APPOINTED != 0 {
            return Holder::Appointment(ContractLink::from_word(r.holder));
        }
        match dir.reference(self.person_kind, Slot::new(r.holder)) {
            Some(p) => Holder::Owner(p),
            None => violation!(clause = "PTY.17", "an office held by a person never begun", row = o.0.get()),
        }
    }

    /// The day an owner took an office; none for one empty or held by appointment, whose start is its contract's.
    pub fn since(&self, o: OfficeRef) -> Missing<Day> {
        match self.row(o).since {
            NONE => Missing::Absent,
            d => Missing::Present(Day::new(d)),
        }
    }

    /// Whether a fill is under way, and whether its holder serves a term.
    #[must_use]
    pub fn flags(&self, o: OfficeRef) -> (bool, bool) {
        let f = self.row(o).flags;
        (f & OFFICE_FILLING != 0, f & OFFICE_TERM_BOUND != 0)
    }

    /// How many offices someone holds, read once a run for its report.
    #[must_use]
    pub fn held(&self) -> u64 {
        let held = (0..whole(self.rows.len()))
            .filter(|at| self.rows.get(Slot::new(*at)).is_some_and(|r| r.institution != NONE && r.holder != NONE))
            .count();
        u64::from(whole(held))
    }

    /// A fill begun: the office stays empty, or with its holder, until it is filled.
    pub fn begin_filling(&mut self, o: OfficeRef, term_bound: bool) {
        self.write(o, |r| {
            r.flags |= OFFICE_FILLING;
            r.flags = if term_bound { r.flags | OFFICE_TERM_BOUND } else { r.flags & !OFFICE_TERM_BOUND };
        });
    }

    /// The person who owns and manages the institution takes the office, under its ownership, on a day.
    #[clause("PTY.16")]
    pub fn fill_owned(&mut self, o: OfficeRef, person: PartyRef, day: Day) {
        if person.kind() != self.person_kind {
            violation!(clause = "PTY.16", "an office owned by a party not a person", party = person.word());
        }
        self.write(o, |r| {
            (r.holder, r.since) = (person.slot().get(), day.get());
            r.flags &= !(OFFICE_FILLING | OFFICE_APPOINTED);
        });
    }

    /// The office held by the person an appointment contract names; its start is the contract's.
    #[clause("PTY.16")]
    pub fn fill_appointed(&mut self, o: OfficeRef, contract: ContractLink) {
        self.write(o, |r| {
            (r.holder, r.since) = (contract.word(), NONE);
            r.flags = (r.flags & !OFFICE_FILLING) | OFFICE_APPOINTED;
        });
    }

    /// The office left empty.
    pub fn vacate(&mut self, o: OfficeRef) {
        self.write(o, |r| {
            (r.holder, r.since) = (NONE, NONE);
            r.flags &= !OFFICE_APPOINTED;
        });
    }

    /// A holder's offices left empty the same day it dies, each found through its appointments and its holdings.
    #[clause("PTY.17")]
    pub fn vacate_all(&mut self, offices: impl IntoIterator<Item = OfficeRef>) {
        for o in offices {
            self.vacate(o);
        }
    }

    /// An office of a new kind added to an institution's block, as a firm enters a line: the block moved, whole, to
    /// rows that hold one more. The block's new first row, for the record to keep.
    pub fn grow(&mut self, first: OfficeRef, kind: u16) -> OfficeRef {
        let (start, len) = self.block(first);
        let institution = self.row(first).institution;
        let new = whole(self.rows.len());
        for at in start..start + len {
            let Some(r) = self.rows.get(Slot::new(at)) else { continue };
            self.rows.push(r);
        }
        self.rows.push(OfficeRow { institution, kind, flags: 0, holder: NONE, since: NONE });
        self.free_block(start, len);
        OfficeRef(Slot::new(new))
    }

    /// An institution's block freed as it ends, its offices vacated with it, for one founded with as many to take.
    pub fn close(&mut self, first: OfficeRef) {
        let (start, len) = self.block(first);
        self.free_block(start, len);
    }

    fn free_block(&mut self, start: u32, len: u32) {
        for at in start..start + len {
            self.rows.set(Slot::new(at), OfficeRow { institution: NONE, kind: 0, flags: 0, holder: NONE, since: NONE });
        }
        self.free.push((start, len));
    }
}

/// An institution's founding preferences, its record's type index into its country's declared sets; an institution
/// with none was founded without its draw, which stops the run.
#[clause("MND.16", "FRM.1")]
#[must_use]
pub fn founding(types: Missing<u8>, institution: PartyKey) -> u8 {
    match types {
        Missing::Present(t) => t,
        Missing::Absent => {
            violation!(clause = "MND.16", "an institution with no founding preferences", party = institution.word())
        }
    }
}

/// The offices' rows are those ever opened; the freed ones wait for a founding.
impl<B: Backing> StoreStats for Offices<B> {
    fn rows_live(&self) -> u64 {
        let freed: u64 = self.free.iter().map(|(_, l)| u64::from(*l)).sum();
        self.rows_ever() - freed
    }

    fn rows_ever(&self) -> u64 {
        u64::from(whole(self.rows.len()))
    }

    fn bytes(&self) -> u64 {
        u64::try_from(self.rows.bytes_committed()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "offices_tests.rs"]
mod tests;
