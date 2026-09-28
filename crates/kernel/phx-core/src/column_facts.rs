//! A handler's facts over its chunk's column slices, in the order its declaration lists them: a read by a fact's place
//! in that list is an indexed load, and a read by name finds the place first.

use phx_id::Slot;
use phx_macros::clause;
use phx_num::{MaybeI64, Missing, violation};

use crate::handler::FactStore;

/// A fact's column over one chunk: read only, or written by the handler that owns the chunk.
#[derive(Debug)]
pub enum FactSlice<'a> {
    Read(&'a [MaybeI64]),
    Write(&'a mut [MaybeI64]),
}

/// One chunk's declared facts: the chunk's first slot and, for each fact, its column's rows in the chunk.
#[clause("TIME.6")]
#[derive(Debug)]
pub struct ColumnFacts<'a> {
    first: u32,
    facts: Vec<(&'static str, FactSlice<'a>)>,
}

impl<'a> ColumnFacts<'a> {
    /// The facts of the chunk whose first slot is `first`, each with its column's rows in the chunk.
    #[must_use]
    pub fn new(first: Slot, facts: Vec<(&'static str, FactSlice<'a>)>) -> ColumnFacts<'a> {
        ColumnFacts { first: first.get(), facts }
    }

    fn row(&self, slot: Slot) -> usize {
        let Some(row) = slot.get().checked_sub(self.first) else {
            violation!(clause = "TIME.6", "a handler read a row before its chunk", slot = slot.get());
        };
        usize::try_from(row).unwrap_or(usize::MAX)
    }

    fn place(&self, fact: &'static str) -> usize {
        // Names are compared by address first: a handler's declared facts are the same statics it reads by.
        let Some(i) = self.facts.iter().position(|(f, _)| std::ptr::eq(*f, fact) || *f == fact) else {
            violation!(clause = "TIME.6", "a handler read a fact its chunk was not given");
        };
        i
    }

    /// The row's value of the fact at `place` in the declaration's list.
    pub fn read_at(&self, place: usize, slot: Slot) -> Missing<i64> {
        let row = self.row(slot);
        let rows: &[MaybeI64] = match self.facts.get(place) {
            Some((_, FactSlice::Read(r))) => r,
            Some((_, FactSlice::Write(w))) => w,
            None => violation!(clause = "TIME.6", "a handler read a fact its chunk was not given", place = place),
        };
        match rows.get(row) {
            Some(v) => v.get(),
            None => violation!(clause = "TIME.6", "a handler read a row past its chunk", slot = slot.get()),
        }
    }

    /// Writes the row's value of the fact at `place`, which the handler writes.
    pub fn write_at(&mut self, place: usize, slot: Slot, value: i64) {
        let row = self.row(slot);
        let Some((_, FactSlice::Write(rows))) = self.facts.get_mut(place) else {
            violation!(clause = "TIME.6", "a handler wrote a fact it only reads", slot = slot.get());
        };
        let Some(cell) = rows.get_mut(row) else {
            violation!(clause = "TIME.6", "a handler wrote a row past its chunk", slot = slot.get());
        };
        *cell = MaybeI64::present(value);
    }
}

impl FactStore for ColumnFacts<'_> {
    fn read(&mut self, fact: &'static str, slot: Slot) -> Missing<i64> {
        let place = self.place(fact);
        self.read_at(place, slot)
    }

    fn write(&mut self, fact: &'static str, slot: Slot, value: i64) {
        let place = self.place(fact);
        self.write_at(place, slot, value);
    }
}

/// A handler's facts over its chunk's party records, row-major: each party's facts one record of `stride` words,
/// a fact at its declared offset in it, so a visit to a sparse row reads one or two cache lines rather than one a
/// fact. The handler owns its chunk's records; `writable` lists the offsets its declaration lets it write.
#[clause("TIME.6", "REP.41")]
#[derive(Debug)]
pub struct RecordFacts<'a> {
    first: u32,
    stride: usize,
    records: &'a mut [MaybeI64],
    layout: Layout<'a>,
}

/// A handler's declared facts in a kind's records, in the declaration's order: each fact's name, its offset in a
/// record, and whether the handler writes it.
#[derive(Debug, Clone, Copy)]
pub struct Layout<'a> {
    pub names: &'a [&'static str],
    pub offsets: &'a [usize],
    pub writable: &'a [bool],
}

impl<'a> RecordFacts<'a> {
    /// The records of the chunk whose first slot is `first`, `stride` words a party, read by the handler's layout.
    #[must_use]
    pub fn new(first: Slot, stride: usize, records: &'a mut [MaybeI64], layout: Layout<'a>) -> RecordFacts<'a> {
        RecordFacts { first: first.get(), stride, records, layout }
    }

    fn cell(&self, place: usize, slot: Slot) -> usize {
        let (Some(row), Some(off)) = (slot.get().checked_sub(self.first), self.layout.offsets.get(place)) else {
            violation!(clause = "TIME.6", "a handler read a row or fact outside its chunk", slot = slot.get());
        };
        usize::try_from(row).unwrap_or(usize::MAX) * self.stride + off
    }

    /// The row's value of the fact declared at `place`.
    pub fn read_at(&self, place: usize, slot: Slot) -> Missing<i64> {
        match self.records.get(self.cell(place, slot)) {
            Some(v) => v.get(),
            None => violation!(clause = "TIME.6", "a handler read a row past its chunk", slot = slot.get()),
        }
    }

    /// Writes the row's value of the fact declared at `place`, which the handler writes.
    pub fn write_at(&mut self, place: usize, slot: Slot, value: i64) {
        if !self.layout.writable.get(place).copied().unwrap_or(false) {
            violation!(clause = "TIME.6", "a handler wrote a fact it only reads", slot = slot.get());
        }
        let at = self.cell(place, slot);
        let Some(cell) = self.records.get_mut(at) else {
            violation!(clause = "TIME.6", "a handler wrote a row past its chunk", slot = slot.get());
        };
        *cell = MaybeI64::present(value);
    }
}

impl FactStore for RecordFacts<'_> {
    fn read(&mut self, fact: &'static str, slot: Slot) -> Missing<i64> {
        let Some(place) = self.layout.names.iter().position(|f| std::ptr::eq(*f, fact) || *f == fact) else {
            violation!(clause = "TIME.6", "a handler read a fact its layout does not declare", slot = slot.get());
        };
        self.read_at(place, slot)
    }

    fn write(&mut self, fact: &'static str, slot: Slot, value: i64) {
        let Some(place) = self.layout.names.iter().position(|f| std::ptr::eq(*f, fact) || *f == fact) else {
            violation!(clause = "TIME.6", "a handler wrote a fact its layout does not declare", slot = slot.get());
        };
        self.write_at(place, slot, value);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Slot;
    use phx_num::{MaybeI64, Missing};

    use super::{ColumnFacts, FactSlice};
    use crate::handler::FactStore;

    const INCOME: &str = "HH.income";
    const AFTER: &str = "HH.after";

    #[test]
    fn handler_context_reads_sentinel_as_absent() {
        let income = [MaybeI64::present(5), MaybeI64::ABSENT];
        let mut after = [MaybeI64::ABSENT, MaybeI64::ABSENT];
        let mut facts = ColumnFacts::new(
            Slot::new(8),
            vec![(INCOME, FactSlice::Read(&income)), (AFTER, FactSlice::Write(&mut after))],
        );
        assert_eq!(facts.read(INCOME, Slot::new(8)), Missing::Present(5));
        assert_eq!(facts.read(INCOME, Slot::new(9)), Missing::Absent, "the sentinel reads as absent");
        facts.write(AFTER, Slot::new(9), 12);
        assert_eq!(facts.read(AFTER, Slot::new(9)), Missing::Present(12), "a row reads back its own write");
        drop(facts);
        assert_eq!(after[1], MaybeI64::present(12));
        let refused = std::panic::catch_unwind(|| {
            let income = [MaybeI64::ABSENT];
            let mut f = ColumnFacts::new(Slot::new(0), vec![(INCOME, FactSlice::Read(&income))]);
            f.write(INCOME, Slot::new(0), 1);
        });
        assert!(refused.is_err(), "a fact only read is never written");
    }

    #[test]
    fn record_facts_read_their_offsets_and_refuse_unwritable() {
        let mut records = [MaybeI64::present(1), MaybeI64::present(2), MaybeI64::ABSENT, MaybeI64::present(4)];
        let layout = super::Layout { names: &[INCOME, AFTER], offsets: &[1, 0], writable: &[false, true] };
        let mut f = super::RecordFacts::new(Slot::new(10), 2, &mut records, layout);
        assert_eq!(f.read_at(0, Slot::new(10)), Missing::Present(2));
        assert_eq!(f.read_at(1, Slot::new(11)), Missing::Absent);
        f.write_at(1, Slot::new(11), 9);
        assert_eq!(f.read(AFTER, Slot::new(11)), Missing::Present(9), "read by name as the context reads");
        assert!(
            std::panic::catch_unwind(move || {
                let mut r = [MaybeI64::ABSENT; 2];
                let layout = super::Layout { names: &[INCOME, AFTER], offsets: &[1, 0], writable: &[false, true] };
            let mut g = super::RecordFacts::new(Slot::new(0), 2, &mut r, layout);
                g.write_at(0, Slot::new(0), 1);
            })
            .is_err()
        );
    }
}
