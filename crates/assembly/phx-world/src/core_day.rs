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
/// next, its schedule among its family's, and the payee's person it is, by the person's identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct Due {
    pub ends: [PartyKey; 2],
    pub amount: i64,
    pub nth: u32,
    pub schedule: u32,
    pub person: u64,
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

pub use crate::consts::reason::{ESTATE, PENSION, WAGE};

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
    /// The estates settled and ended.
    pub estates: u64,
    /// The money family's breaks found after settlement: a bank owing other than its customers hold, money made or
    /// lost among the parties, or reserves moving other than by what customers paid those held at the issuer.
    pub breaks: u64,
    /// The core's day's time, its chance and settlement together, by the run's clock.
    pub ns: u64,
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
    /// An estate begun with a household's money at its bank, to settle on its country's next business day.
    #[clause("PTY.9")]
    pub(crate) fn open_estate(&mut self, (bank, money): (u32, i64), (country, day): (CountryId, Day)) {
        let Some(place) = self.names.iter().position(|n| *n == phx_core::ESTATE_KIND.name) else {
            violation!(clause = "PTY.9", "an estate with no kind to hold it");
        };
        let id = phx_id::PartyId::new(self.next_id);
        self.next_id += 1;
        let Some(store) = self.kinds.get_mut(place) else { return };
        let party = store.begin(id, &[], Some(phx_core::store::Opening { bank, balance: money }));
        let key = PartyKey::new(u8::try_from(place).unwrap_or(u8::MAX), party.slot());
        let at = self.keys.partition_point(|(i, _)| *i < id);
        self.keys.insert(at, (id, key));
        self.estates.push((key, country, day));
    }

    /// Each estate opened before today, on its country's business day, pays what it holds to the party the law names
    /// where no heir is drawn — its country's treasury — and is ended after the day's settlement.
    #[clause("PTY.9", "POP.15")]
    fn estates_pay(&mut self, day: Day, calendar: &Calendar, out: &mut Vec<Flow>) -> Vec<PartyKey> {
        let mut settling = Vec::new();
        let treasury = self.names.iter().position(|n| *n == crate::consts::HEIRLESS_DESTINATION);
        for (estate, country, opened) in &self.estates {
            if *opened >= day || !calendar.is_business(*country, day) {
                continue;
            }
            let Some(to) = treasury.and_then(|t| {
                self.treasuries
                    .get(usize::from(country.get()))
                    .copied()
                    .flatten()
                    .filter(|k| usize::from(k.kind()) == t)
            }) else {
                violation!(
                    clause = "POP.15",
                    "an estate's country with no heirless destination",
                    country = country.get()
                );
            };
            let held = self
                .kinds
                .get(usize::from(estate.kind()))
                .and_then(|k| k.accounts.as_ref())
                .and_then(|a| Some(a.balance.get(estate.slot())? + a.pending.get(estate.slot())?));
            if let Some(amount) = held.filter(|m| *m > 0) {
                out.push(Flow {
                    payer: *estate,
                    payee: to,
                    amount,
                    source: estate.slot().get(),
                    denomination: Denom::money(country.get()),
                    reason: ESTATE,
                    order: 0,
                });
            }
            settling.push(*estate);
        }
        settling
    }

    /// The money every party but the banks holds, and the banks' reserves with every account held at the issuer:
    /// the first changes only by the issuer's own flows, the second only by what the issuer pays or is paid.
    #[must_use]
    pub fn money_totals(&self) -> (i128, i128) {
        let (mut parties, mut at_issuer) = (0_i128, 0_i128);
        for (k, store) in self.kinds.iter().enumerate() {
            let Some(a) = store.accounts.as_ref() else { continue };
            let bank = self.bank_kind.is_some_and(|b| usize::from(b) == k);
            for ((b, m), p) in a.bank.slice().iter().zip(a.balance.slice()).zip(a.pending.slice()) {
                let held = i128::from(*m) + i128::from(*p);
                if !bank {
                    parties += held;
                }
                if bank || *b == phx_core::settle::AT_ISSUER {
                    at_issuer += held;
                }
            }
        }
        (parties, at_issuer)
    }

    /// The money family on the core after the day's settlement, reading only: each bank owes what its customers hold,
    /// and neither total moved, the issuer making no flow on the core yet.
    #[clause("MON.5", "N1")]
    fn money_breaks(&self, before: (i128, i128), deposits: &mut [i64]) -> u64 {
        let owed = deposits_of(self.kinds.iter(), deposits.len());
        let banks = u64::try_from(owed.iter().zip(deposits.iter()).filter(|(a, b)| a != b).count()).unwrap_or(u64::MAX);
        let (parties, at_issuer) = self.money_totals();
        banks + u64::from(parties != before.0) + u64::from(at_issuer != before.1)
    }

    /// Runs the core's day: every family's dues made flows, then each currency's flows settled on its country's
    /// business day or committed on its closed day.
    #[clause("SET.4", "SET.6", "MON.5")]
    pub fn run_day(&mut self, day: Day, calendar: &Calendar, streams: &Streams, order: &StreamDecl) -> CoreDay {
        let mut work = std::mem::take(&mut self.work);
        work.flows.reset(1);
        let mut record = CoreDay { day, flows: 0, settled: 0, failed: 0, committed: 0, estates: 0, breaks: 0, ns: 0 };
        let before = self.money_totals();
        for family in &mut self.families {
            if let Some(buf) = work.flows.chunks_mut().first_mut() {
                record.flows += family.dues(day, calendar, &mut work.due, buf);
            }
        }
        let mut settling = Vec::new();
        if let Some(buf) = work.flows.chunks_mut().first_mut() {
            let before = buf.len();
            settling = self.estates_pay(day, calendar, buf);
            record.flows += phx_rand::float::len_u64(buf.len() - before);
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
        record.breaks += self.money_breaks(before, &mut deposits);
        for estate in settling {
            let empty = self.kinds.get(usize::from(estate.kind())).and_then(|k| k.accounts.as_ref()).is_some_and(|a| {
                a.balance.get(estate.slot()).unwrap_or(0) == 0 && a.pending.get(estate.slot()).unwrap_or(0) == 0
            });
            if empty {
                self.estates.retain(|(e, _, _)| *e != estate);
                if let Some(k) = self.kinds.get_mut(usize::from(estate.kind()))
                    && let Some(r) = k.parties.at(estate.slot())
                {
                    k.parties.end(r);
                }
                record.estates += 1;
            }
        }
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
