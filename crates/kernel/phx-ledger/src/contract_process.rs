use std::collections::BTreeMap;

use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::{Holders, Ledger, Located};
use crate::fails::Fail;
use crate::rows::{PaymentRecord, rows};

/// A contract row in arrears: its line, its side, and the party holding it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
#[derive(Clone, Debug, Default)]
pub struct Arrears {
    since: BTreeMap<ArrearsKey, Day>,
}

impl Arrears {
    #[must_use]
    pub fn since(&self, key: ArrearsKey) -> Option<Day> {
        self.since.get(&key).copied()
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
    #[clause("SET.3", "SET.16")]
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

    /// Arrears cured when a row's dues are paid up: it leaves the index, and its days in arrears return to nothing.
    pub fn cure(&mut self, holders: &mut dyn Holders, line: LineId, side: Side, party: PartyId) {
        let key = ArrearsKey { line, side: side_code(side), party };
        if self.arrears.since.remove(&key).is_none() {
            violation!(clause = "SET.3", "arrears cured on a row that had none", line = line.get());
        }
        let Located::Live { table, slot, .. } = holders.locate(party) else { return };
        let arenas = holders.arenas(table);
        let Some(view) = rows(arenas, slot).into_iter().find(|r| r.row.line == line && r.side() == side) else {
            violation!(clause = "SET.3", "arrears cured on a row its party does not have", line = line.get());
        };
        let record = PaymentRecord { arrears_days: 0, missed: view.record().missed };
        self.lines.set_record(arenas, slot, line, side, record);
    }

    fn write_record(&mut self, holders: &mut dyn Holders, key: ArrearsKey, today: Day, missed_one: bool) {
        let Some(since) = self.arrears.since(key) else { return };
        let Located::Live { table, slot, .. } = holders.locate(key.party) else {
            self.arrears.since.remove(&key);
            return;
        };
        let side = side_of(key.side);
        let arenas = holders.arenas(table);
        let Some(view) = rows(arenas, slot).into_iter().find(|r| r.row.line == key.line && r.side() == side) else {
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
        self.lines.set_record(arenas, slot, key.line, side, record);
    }
}
