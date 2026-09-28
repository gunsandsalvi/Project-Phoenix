//! The core's day while the port runs beside the books: each family's contracts due today make their flows, a
//! contract's next date read from its schedule; and each currency's flows settle over the core's accounts on its
//! country's business days, and are committed to what the accounts have pending on its closed days.

use phx_core::calendar::Calendar;
use phx_core::calendar::period::ScheduleDates;
use phx_core::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};
use phx_core::settle::Settle;
use phx_core::store::{Family, books, deposits_of};
use phx_core::{StreamDecl, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::{Subject, SubjectTag};
use phx_store::{Row, SystemBacking};

use crate::core::Core;

/// A dated contract on the core: its payer and payee, the amount each date pays, which date of its schedule comes
/// next, its schedule among its family's, and the payee's person it is.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct Due {
    pub ends: [PartyKey; 2],
    pub amount: i64,
    pub nth: u32,
    pub schedule: u32,
    /// The person of the payee's household the contract is its, by its place there.
    pub person: u32,
    pub pad: u32,
}

impl Row for Due {
    fn ends(&self) -> [PartyKey; 2] {
        self.ends
    }

    fn set_end(&mut self, side: usize, party: PartyKey) {
        if let Some(e) = self.ends.get_mut(side) {
            *e = party;
        }
    }
}

/// The reasons the core's flows are made for, by their code.
pub const PENSION: u8 = 1;

/// A family of dated contracts: its store, the reason its flows carry, and the schedules its contracts' dates are
/// read from, each with its currency and the payment order its payer gives the family's flows.
#[derive(Debug)]
pub struct DatedFamily {
    pub name: &'static str,
    pub store: Family<Due, SystemBacking>,
    pub reason: u8,
    pub schedules: Vec<(ScheduleDates, u8, u8)>,
}

/// What the core's day did: the flows made, settled, failed and committed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoreDay {
    pub day: Day,
    pub flows: u64,
    pub settled: u64,
    pub failed: u64,
    pub committed: u64,
}

/// The day's working state, kept across days so a day allocates nothing once the heaviest has sized it.
#[derive(Debug, Default)]
pub struct Work {
    pub flows: FlowBufs,
    pub settle: Settle,
    pub due: Vec<u32>,
}

impl DatedFamily {
    /// The contracts due today made flows, each rescheduled at its next date.
    #[clause("SET.4", "TIME.4")]
    fn dues(&mut self, day: Day, calendar: &Calendar, due: &mut Vec<u32>, out: &mut Vec<Flow>) -> u64 {
        self.store.wheel.take(day, due, None);
        let mut made = 0;
        for edge in due.iter().copied() {
            let slot = Slot::new(edge);
            if !self.store.edges.is_open(slot) {
                continue;
            }
            let Some(row) = self.store.edges.rows_mut().get_mut(usize::try_from(edge).unwrap_or(usize::MAX)) else {
                continue;
            };
            let Some((dates, ccy, order)) = self.schedules.get(usize::try_from(row.schedule).unwrap_or(usize::MAX))
            else {
                violation!(clause = "TIME.4", "a contract with no schedule", edge = edge);
            };
            out.push(Flow {
                payer: row.ends[0],
                payee: row.ends[1],
                amount: row.amount,
                source: edge,
                denomination: Denom::money(*ccy),
                reason: self.reason,
                order: *order,
            });
            made += 1;
            row.nth += 1;
            let next = dates.nth(calendar, row.nth);
            if next <= day {
                violation!(clause = "TIME.4", "a contract's next date not after today", edge = edge);
            }
            self.store.wheel.schedule(edge, next);
        }
        made
    }
}

impl Core {
    /// Runs the core's day: every family's dues made flows, then each currency's flows settled on its country's
    /// business day or committed on its closed day.
    #[clause("SET.4", "SET.6", "MON.5")]
    pub fn run_day(&mut self, day: Day, calendar: &Calendar, streams: &Streams, order: &StreamDecl) -> CoreDay {
        let mut work = std::mem::take(&mut self.work);
        work.flows.reset(1);
        let mut record = CoreDay { day, flows: 0, settled: 0, failed: 0, committed: 0 };
        for family in &mut self.families {
            if let Some(buf) = work.flows.chunks_mut().first_mut() {
                record.flows += family.dues(day, calendar, &mut work.due, buf);
            }
        }
        let high: Vec<u32> = self.kinds.iter().map(|k| k.parties.high_water()).collect();
        let ranges = Ranges::new(self.range_bits, &high);
        let banks = self.bank_kind.map_or(0, |b| {
            usize::try_from(self.kinds.get(usize::from(b)).map_or(0, |k| k.parties.high_water())).unwrap_or(0)
        });
        let mut deposits = deposits_of(self.kinds.iter(), banks);
        let closed = vec![false; banks];
        for (country, issuer) in self.issuers.iter().enumerate() {
            let Ok(ccy) = u8::try_from(country) else { continue };
            work.flows.group(None, &ranges, Denom::money(ccy));
            let grouped = Grouped::new(&[&work.flows], &ranges);
            let mut b = books(&mut self.kinds, self.bank_kind.unwrap_or(u8::MAX), (&mut deposits, &closed), *issuer);
            if calendar.is_business(CountryId::new(ccy), day) {
                let lot = |p: PartyKey| {
                    streams.open(
                        order,
                        Subject::new(SubjectTag::Party, u64::from(p.word())),
                        day,
                        SubStep::S7b.ordinal(),
                    )
                };
                let out = work.settle.settle(None, &grouped, &ranges, &mut b, &lot);
                record.settled += out.settled;
                record.failed += phx_rand::float::len_u64(out.failed.len());
            } else {
                work.settle.commit(None, &grouped, &ranges, &mut b);
                record.committed += phx_rand::float::len_u64(grouped.end());
            }
        }
        self.work = work;
        for k in &mut self.kinds {
            k.parties.close_day();
        }
        for f in &mut self.families {
            f.store.edges.close_day();
        }
        self.days.push(record);
        record
    }
}
