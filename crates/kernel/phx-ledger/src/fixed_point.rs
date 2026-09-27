use std::collections::{BTreeMap, BTreeSet, VecDeque};

use phx_core::calendar::Calendar;
use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, violation};
use phx_rand::Draws;
use phx_store::Backing;

use crate::algebra::Side;
use crate::books::Books;
use crate::cleared::{Losers, times};
use crate::due::DueLines;
use crate::dues::Found;
use crate::instruction::{AccountRef, LegKind, LegRec};
use crate::pooled::{PooledRow, RowOutcome, pooled};
use crate::stream::{DayRecords, Payment, Records};

/// Stage 7b's result: the payments that fail, by line and payee, and how many times the worklist took a party.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FixedPoint {
    pub failed: BTreeSet<(LineId, PartyId)>,
    /// The failed payments on the way through a bank that could not cover its net: the bank's removal takes them,
    /// whether or not their payer had failed them first, so none is its payer's to answer for.
    pub by_bank: BTreeSet<(LineId, PartyId)>,
    pub iterations: u64,
    /// Each payer's payments among the day's, by their places in 7a's list, in payment order.
    pub by_payer: BTreeMap<PartyId, Vec<usize>>,
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
    found: &'a mut Found,
    records: &'a mut Records,
    /// The day's payments, as 7a found them, and each payer's by its payment order: a payer's own payments are read
    /// from here whatever lists its lines keep, so a side that keeps none is reached too.
    made: &'a [Payment],
    by_payer: BTreeMap<PartyId, Vec<usize>>,
    /// The parties still short once their own payments failed, taken as banks that cannot cover their customers'
    /// payments once every payer is done; and the banks whose customers' payments have been removed.
    short_banks: BTreeSet<PartyId>,
    removed_banks: BTreeSet<PartyId>,
    failed: BTreeSet<(LineId, PartyId)>,
    by_bank: BTreeSet<(LineId, PartyId)>,
    queue: VecDeque<PartyId>,
    queued: BTreeSet<PartyId>,
    draws_of: &'a dyn Fn(LineId) -> Draws,
}

impl<B: Backing> Work<'_, B> {
    fn enqueue(&mut self, party: PartyId) {
        if self.queued.insert(party) {
            self.queue.push_back(party);
        }
    }

    /// A payment removed: its effects come off every account it touched, whose parties are taken again. A cleared
    /// line's payer failing, its members' dues are lost by claimant members drawn for them.
    fn fail(&mut self, p: &Payment) {
        if !self.failed.insert(p.key()) {
            return;
        }
        let legs = self.books.effects(p);
        for party in self.books.book(self.records, &legs, -1) {
            self.enqueue(party);
        }
        if p.cleared && p.payer == p.reckoned_on {
            self.lose(p.line, p.members, p.ccy);
        }
    }

    /// A cleared line's failed members grown by `members`: the claimant members drawn to lose them, from the same
    /// sequence of the line's stream, and each newly drawn claimant's credit taken off the accounts it reaches.
    #[clause("REP.23", "REP.31")]
    fn lose(&mut self, line: LineId, members: u32, ccy: Ccy) {
        let Some(day) = self.found.cleared.get(line) else {
            violation!(clause = "REP.23", "a cleared payment failed on a line never reckoned", line = line.get());
        };
        let claimants: Vec<(PartyId, u32, u32)> = if day.losers.is_none() {
            day.claimants.iter().map(|(p, (c, u))| (*p, *c, *u)).collect()
        } else {
            Vec::new()
        };
        let draws_of = self.draws_of;
        let Some(day) = self.found.cleared.get_mut(line) else { return };
        day.failed += u64::from(members);
        let failed = day.failed;
        let per = day.per_member;
        let more = day.losers.get_or_insert_with(|| Losers::new(&claimants, draws_of(line))).draw_to(failed);
        let kind = self.books.ledger.lines.kind_of(line);
        let levy = self
            .books
            .withholding
            .iter()
            .find(|w| w.kind == kind && w.ccy == ccy)
            .map(|w| (w.payee, w.on_payment(per)));
        for (claimant, k) in more {
            let tax = levy.map_or(0, |(_, t)| t);
            let (mut legs, _) = self.books.route(claimant, -times(per - tax, k), ccy);
            if let Some((payee, t)) = levy.filter(|(_, t)| *t > 0) {
                legs.extend(self.books.route(payee, -times(t, k), ccy).0);
            }
            for party in self.books.book(self.records, &legs, -1) {
                self.enqueue(party);
            }
        }
    }

    /// A party short of funds given the payments standing: it fails its own payments from its first unaffordable one
    /// on, a prefix of its payment order; if what it pays for others through its account still exceeds its funds, it
    /// is a bank that may not cover its net, answered once every payer is done.
    fn visit(&mut self, party: PartyId) {
        let Some(rec) = self.records.get(self.books.parties.row(party)).copied() else { return };
        if rec.standing() >= 0 {
            return;
        }
        let made = self.made;
        let own: Vec<(Payment, i128)> = self
            .by_payer
            .get(&party)
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter_map(|i| made.get(*i))
            .filter(|p| !self.failed.contains(&p.key()))
            .map(|p| (*p, draw(&self.books.effects(p), party, rec.account)))
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
        if self.records.get(self.books.parties.row(party)).is_some_and(|r| r.standing() < 0) {
            self.short_banks.insert(party);
        }
    }

    /// Every payment of a bank's customers whose money passes through it, the bank's to answer for even where its payer
    /// had failed it: those its paying customers make, and those its customers are paid but on a cleared line, whose
    /// credit only adds to the bank's reserves and stands. Its customers are found among the day's payments, not by the
    /// holders its lines list.
    #[clause("MON.5")]
    fn remove_customers(&mut self, bank: PartyId) {
        let issued: BTreeSet<LineId> = self
            .books
            .rows_of(bank)
            .into_iter()
            .filter(|(line, side)| *side == Side::Liability && self.books.ledger.lines.is_money(*line))
            .map(|(line, _)| line)
            .collect();
        let banked_here = |legs: &[LegRec], party: PartyId| {
            legs.iter().any(|l| {
                l.party == party
                    && matches!(l.kind, LegKind::Money)
                    && matches!(l.account, AccountRef::Line { line, side: Side::Asset } if issued.contains(&line))
            })
        };
        let made = self.made;
        for p in made {
            let legs = self.books.effects(p);
            if banked_here(&legs, p.payer) || (!p.cleared && banked_here(&legs, p.payee)) {
                self.by_bank.insert(p.key());
                self.fail(p);
            }
        }
    }
}

impl<B: Backing> Books<B> {
    /// Stage 7b: from every payment standing, the parties short of funds fail, until nothing changes. A party fails
    /// as a prefix of its payment order, which is monotone in its funds, and a removal takes again the parties whose
    /// accounts it touched, so the payments left are the greatest set that can settle given one another, rings
    /// included.
    #[clause("SET.6", "MON.5", "REP.9", "TIME.6")]
    pub(crate) fn fixed_point(
        &self,
        day: &mut DayRecords,
        due: &DueLines,
        today: Day,
        calendar: &Calendar,
        found: &mut Found,
        draws_of: &dyn Fn(LineId) -> Draws,
    ) -> FixedPoint {
        let mut short: Vec<PartyId> = day.records.each().filter(|(_, r)| r.standing() < 0).map(|(p, _)| p).collect();
        short.sort_unstable();
        let mut by_payer: BTreeMap<PartyId, Vec<usize>> = BTreeMap::new();
        for (i, p) in day.made.iter().enumerate() {
            by_payer.entry(p.payer).or_default().push(i);
        }
        for list in by_payer.values_mut() {
            list.sort_by_key(|i| day.made.get(*i).map(|p| (p.order, p.line, p.reckoned_on)));
        }
        let mut work = Work {
            books: self,
            found,
            records: &mut day.records,
            made: &day.made,
            by_payer,
            short_banks: BTreeSet::new(),
            removed_banks: BTreeSet::new(),
            failed: BTreeSet::new(),
            by_bank: BTreeSet::new(),
            queue: VecDeque::new(),
            queued: BTreeSet::new(),
            draws_of,
        };
        for holder in &day.moneyless {
            for row in self.due_rows_of(*holder, due) {
                if let Some(p) = self.payment(*holder, &row, today, calendar, work.found).filter(|p| p.moneyless) {
                    work.fail(&p);
                }
            }
        }
        for p in short {
            work.enqueue(p);
        }
        // Payers fail their own payments until none is short but by what it pays for others; then every bank still
        // short at that point loses its customers' payments at once, and the payers answer again. A payer's removal is
        // monotone and a bank's is read only once the payers are done, so the result is the same in any order.
        let mut iterations = 0;
        loop {
            while let Some(party) = work.queue.pop_front() {
                work.queued.remove(&party);
                iterations += 1;
                work.visit(party);
            }
            let banks: Vec<PartyId> = std::mem::take(&mut work.short_banks)
                .into_iter()
                .filter(|b| work.records.get(self.parties.row(*b)).is_some_and(|r| r.standing() < 0))
                .filter(|b| !work.removed_banks.contains(b))
                .collect();
            if banks.is_empty() {
                break;
            }
            for bank in banks {
                work.removed_banks.insert(bank);
                work.remove_customers(bank);
            }
        }
        FixedPoint { failed: work.failed, by_bank: work.by_bank, iterations, by_payer: work.by_payer }
    }
}
