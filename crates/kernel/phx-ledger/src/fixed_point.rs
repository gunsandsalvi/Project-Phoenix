use std::collections::{BTreeMap, BTreeSet, VecDeque};

use phx_core::calendar::Calendar;
use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_store::Backing;

use crate::algebra::Side;
use crate::books::Books;
use crate::due::DueLines;
use crate::dues::Found;
use crate::instruction::{AccountRef, LegKind, LegRec};
use crate::pooled::{PooledRow, RowOutcome, pooled};
use crate::stream::{DayRecords, Payment, Record};

/// Stage 7b's result: the payments that fail, by line and payee, and how many times the worklist took a party.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixedPoint {
    pub failed: BTreeSet<(LineId, PartyId)>,
    /// The failed payments removed because a bank on their way could not cover its net, not for their payer.
    pub by_bank: BTreeSet<(LineId, PartyId)>,
    pub iterations: u64,
}

/// What a payment draws on a party's account: its legs taking from the account the party holds.
pub(crate) fn draw(legs: &[LegRec], party: PartyId, account: LineId) -> i128 {
    legs.iter()
        .filter(|l| l.party == party && matches!(l.kind, LegKind::Money))
        .filter(|l| matches!(l.account, AccountRef::Line { line, side: Side::Asset } if line == account))
        .map(|l| -i128::from(l.qty))
        .filter(|q| *q > 0)
        .sum()
}

struct Work<'a, B: Backing> {
    books: &'a Books<B>,
    due: &'a DueLines,
    day: Day,
    calendar: &'a Calendar,
    found: &'a mut Found,
    records: &'a mut BTreeMap<PartyId, Record>,
    failed: BTreeSet<(LineId, PartyId)>,
    by_bank: BTreeSet<(LineId, PartyId)>,
    queue: VecDeque<PartyId>,
    queued: BTreeSet<PartyId>,
}

impl<B: Backing> Work<'_, B> {
    fn enqueue(&mut self, party: PartyId) {
        if self.queued.insert(party) {
            self.queue.push_back(party);
        }
    }

    /// A payment removed: its effects come off every account it touched, whose parties are taken again.
    fn fail(&mut self, p: &Payment) {
        if !self.failed.insert(p.key()) {
            return;
        }
        let legs = self.books.effects(p, self.found);
        for party in self.books.book(self.records, &legs, -1) {
            self.enqueue(party);
        }
    }

    /// A party short of funds given the payments standing: it fails its own payments from its first unaffordable one
    /// on, a prefix of its payment order; if what it pays for others through its account still exceeds its funds, it
    /// is a bank that cannot cover its net, and its customers' payments through it are removed.
    fn visit(&mut self, party: PartyId) {
        let Some(rec) = self.records.get(&party).copied() else { return };
        if rec.standing() >= 0 {
            return;
        }
        let all = self.books.payments_of(party, self.due, self.day, self.calendar, self.found);
        let own: Vec<(Payment, i128)> = all
            .iter()
            .filter(|p| p.payer == party && !self.failed.contains(&p.key()))
            .map(|p| (*p, draw(&self.books.effects(p, self.found), party, rec.account)))
            .collect();
        let own_draw: i128 = own.iter().map(|(_, d)| d).sum();
        let for_others = rec.debit - own_draw;
        let rows: Vec<PooledRow> = own
            .iter()
            .map(|(_, d)| {
                let Ok(per_member) = i64::try_from(*d) else {
                    phx_num::capacity_exceeded!("a payment's draw", i64::MAX, 0);
                };
                PooledRow { per_member, reached: 1, position: 0, moves: 0 }
            })
            .collect();
        let outcomes = pooled(rec.funds + rec.credit - for_others, 1, &rows, &[]);
        for ((p, _), o) in own.iter().zip(outcomes) {
            if o == RowOutcome::Fails {
                self.fail(p);
            }
        }
        if self.records.get(&party).is_some_and(|r| r.standing() < 0) {
            self.remove_customers(party, rec.account);
        }
    }

    /// Every standing payment of a bank's customers whose legs pass through the bank's account.
    #[clause("MON.5")]
    fn remove_customers(&mut self, bank: PartyId, account: LineId) {
        let issued: Vec<LineId> = self
            .books
            .rows_of(bank)
            .into_iter()
            .filter(|(line, side)| *side == Side::Liability && self.books.ledger.lines.is_money(*line))
            .map(|(line, _)| line)
            .collect();
        for line in issued {
            for customer in self.books.side_holders(line, Side::Asset) {
                for p in self.books.payments_of(customer, self.due, self.day, self.calendar, self.found) {
                    let legs = self.books.effects(&p, self.found);
                    if draw(&legs, bank, account) > 0 || legs.iter().any(|l| l.party == bank) {
                        if !self.failed.contains(&p.key()) {
                            self.by_bank.insert(p.key());
                        }
                        self.fail(&p);
                    }
                }
            }
        }
    }
}

impl<B: Backing> Books<B> {
    /// Stage 7b: from every payment standing, the parties short of funds fail, until nothing changes. A party fails
    /// as a prefix of its payment order, which is monotone in its funds, and a removal takes again the parties whose
    /// accounts it touched, so the payments left are the greatest set that can settle given one another, rings
    /// included.
    #[clause("SET.6", "MON.5", "REP.8", "TIME.6")]
    pub(crate) fn fixed_point(
        &self,
        day: &mut DayRecords,
        due: &DueLines,
        today: Day,
        calendar: &Calendar,
        found: &mut Found,
    ) -> FixedPoint {
        let short: Vec<PartyId> = day.records.iter().filter(|(_, r)| r.standing() < 0).map(|(p, _)| *p).collect();
        let mut work = Work {
            books: self,
            due,
            day: today,
            calendar,
            found,
            records: &mut day.records,
            failed: BTreeSet::new(),
            by_bank: BTreeSet::new(),
            queue: VecDeque::new(),
            queued: BTreeSet::new(),
        };
        for p in short {
            work.enqueue(p);
        }
        let mut iterations = 0;
        while let Some(party) = work.queue.pop_front() {
            work.queued.remove(&party);
            iterations += 1;
            work.visit(party);
        }
        FixedPoint { failed: work.failed, by_bank: work.by_bank, iterations }
    }
}
