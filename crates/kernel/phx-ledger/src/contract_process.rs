use std::collections::BTreeMap;

use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::{Holders, Ledger, Located};
use crate::fails::Fail;
use crate::line::Lines;
use crate::rows::{self, PaymentRecord};

/// A contract row in arrears: its line, its side, and the party holding it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct ArrearsKey {
    pub line: LineId,
    pub side: u8,
    pub party: PartyId,
}

fn side_code(side: Side) -> u8 {
    match side {
        Side::Asset => 0,
        Side::Liability => 1,
    }
}

fn side_of(code: u8) -> Side {
    if code == 0 { Side::Asset } else { Side::Liability }
}

/// The rows in arrears, each with the day its oldest unpaid due fell: the one fact the rows' days in arrears are read
/// from, so the process refreshes them without asking when each began.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub struct Arrears {
    since: BTreeMap<ArrearsKey, Day>,
}

impl ArrearsKey {
    #[must_use]
    pub fn new(line: LineId, side: Side, party: PartyId) -> ArrearsKey {
        ArrearsKey { line, side: side_code(side), party }
    }
}

impl Arrears {
    /// A row's arrears carried to another holder, begun on the day they began where the row was.
    pub(crate) fn begin(&mut self, key: ArrearsKey, since: Day) {
        if self.since.insert(key, since).is_some() {
            violation!(clause = "SET.3", "arrears begun twice on one row", line = key.line.get());
        }
    }

    /// A row's arrears leaving with it, if it had any.
    pub(crate) fn remove(&mut self, key: ArrearsKey) {
        self.since.remove(&key);
    }

    #[must_use]
    pub fn since(&self, key: ArrearsKey) -> Option<Day> {
        self.since.get(&key).copied()
    }

    /// Since when a party's row on a side of a line has been in arrears, if it is.
    #[must_use]
    pub fn of(&self, line: LineId, side: Side, party: PartyId) -> Option<Day> {
        self.since(ArrearsKey { line, side: side_code(side), party })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.since.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.since.is_empty()
    }
}

fn days(since: Day, today: Day) -> u16 {
    let Some(d) = today.get().checked_sub(since.get()) else {
        violation!(clause = "SET.3", "arrears that began after today", since = since.get(), today = today.get());
    };
    let Ok(d) = u16::try_from(d) else {
        capacity_exceeded!("days in arrears in a row's record", u16::MAX, d);
    };
    d
}

impl<B: Backing> Ledger<B> {
    /// Stage 2d: each line kind's generic contract process turns yesterday's fails on its lines into arrears — a
    /// payment missed on the row, arrears begun at the fail's due day if the row had none — and refreshes the days in
    /// arrears of every row still in arrears. A fail naming no contract row is a fail of a payment with no contract,
    /// whose party sees it and whose line has nothing to record.
    #[clause("SET.3", "SET.16", "TIME.7")]
    pub fn contract_process(&mut self, holders: &mut dyn Holders, fails: &[Fail], today: Day) {
        for fail in fails {
            let phx_num::Missing::Present(row) = fail.row else { continue };
            let key = ArrearsKey { line: row.line, side: side_code(row.side), party: fail.party };
            let begun = self.arrears.since.entry(key).or_insert(fail.due);
            if fail.due < *begun {
                *begun = fail.due;
            }
            self.write_record(holders, key, today, true);
        }
        let keys: Vec<ArrearsKey> = self.arrears.since.keys().copied().collect();
        for key in keys {
            self.write_record(holders, key, today, false);
        }
    }

    /// Today's fails on a cell's rows about to leave it, made arrears now: the members leaving failed with the cell,
    /// so the arrears and the missed payment go with them, where tomorrow's contract process would find the row gone.
    /// Every fail of a day comes from an instruction numbered that day, so the day numbered last is today.
    #[clause("SET.3", "REP.8")]
    pub(crate) fn arrears_before_leaving(
        &mut self,
        arenas: &mut dyn crate::holder::HolderArenas,
        holder: phx_id::Slot,
        party: PartyId,
        rows: &[(LineId, Side)],
    ) {
        let today = self.numbered.0;
        let found: Vec<(usize, Fail)> = self
            .day
            .fails
            .iter()
            .enumerate()
            .filter(|(i, f)| f.party == party && !self.day.arrears_taken.contains(i))
            .filter(|(_, f)| matches!(f.row, phx_num::Missing::Present(r) if rows.contains(&(r.line, r.side))))
            .map(|(i, f)| (i, *f))
            .collect();
        for (i, fail) in found {
            let phx_num::Missing::Present(row) = fail.row else { continue };
            let key = ArrearsKey { line: row.line, side: side_code(row.side), party };
            let begun = self.arrears.since.entry(key).or_insert(fail.due);
            if fail.due < *begun {
                *begun = fail.due;
            }
            let since = *begun;
            let Some(view) = rows::iter(arenas, holder).find(|r| r.row.line == row.line && r.side() == row.side) else {
                violation!(clause = "SET.3", "a fail on a row its cell does not have", line = row.line.get());
            };
            let mut record = view.record();
            let Some(m) = record.missed.checked_add(1) else {
                capacity_exceeded!("payments missed in a row's record", u16::MAX, record.missed);
            };
            record.missed = m;
            record.arrears_days = days(since, today);
            Lines::<B>::set_record(arenas, holder, row.line, row.side, record);
            self.day.arrears_taken.insert(i);
        }
    }

    /// Arrears cured when a row's dues are paid up: it leaves the index, and its days in arrears return to nothing.
    pub fn cure(&mut self, holders: &mut dyn Holders, line: LineId, side: Side, party: PartyId) {
        let key = ArrearsKey { line, side: side_code(side), party };
        if self.arrears.since.remove(&key).is_none() {
            violation!(clause = "SET.3", "arrears cured on a row that had none", line = line.get());
        }
        let Located::Live { table, slot, .. } = holders.locate(party) else { return };
        let arenas = holders.arenas(table);
        let Some(view) = rows::iter(arenas, slot).find(|r| r.row.line == line && r.side() == side) else {
            violation!(clause = "SET.3", "arrears cured on a row its party does not have", line = line.get());
        };
        let record = PaymentRecord { arrears_days: 0, missed: view.record().missed };
        Lines::<B>::set_record(arenas, slot, line, side, record);
    }

    fn write_record(&mut self, holders: &mut dyn Holders, key: ArrearsKey, today: Day, missed_one: bool) {
        let Some(since) = self.arrears.since(key) else { return };
        let Located::Live { table, slot, .. } = holders.locate(key.party) else {
            self.arrears.since.remove(&key);
            return;
        };
        let side = side_of(key.side);
        let arenas = holders.arenas(table);
        let Some(view) = rows::iter(arenas, slot).find(|r| r.row.line == key.line && r.side() == side) else {
            violation!(clause = "SET.3", "arrears on a row its party does not have", line = key.line.get());
        };
        let mut record = view.record();
        if missed_one {
            let Some(m) = record.missed.checked_add(1) else {
                capacity_exceeded!("payments missed in a row's record", u16::MAX, record.missed);
            };
            record.missed = m;
        }
        record.arrears_days = days(since, today);
        Lines::<B>::set_record(arenas, slot, key.line, side, record);
    }
}
