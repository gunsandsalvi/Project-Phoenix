use std::collections::{BTreeMap, BTreeSet};

use phx_core::calendar::Calendar;
use phx_core::{AuditStream, SubStep};
use phx_id::{Day, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::ApplyAt;
use crate::books::Books;
use crate::check::FailCause;
use crate::consts::RUN_SAMPLE_PERIOD;
use crate::due::DueLines;
use crate::dues::Found;
use crate::effects::{DueOutcome, DueRec};
use crate::fails::Fail;
use crate::fixed_point::FixedPoint;
use crate::instruction::{AccountRef, Denom, DueRow, Instruction, LegKind, LegRec, RowOp};
use crate::pending::Closed;
use crate::runs;
use crate::stream::{DayRecords, Payment, Record, Records};

/// What a day's settlement came to, for the published measure and the live checks: the dues streamed and settled,
/// the fixed point's work, the gross paid, the bytes the day's buffers held, and the breaches found by recomputing
/// over the settled payments on the books as stage 7 found them, each zero when the settlement is right.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct DaySettlement {
    pub lines: u64,
    pub heads_read: u64,
    pub rows_scanned: u64,
    pub rows_due: u64,
    pub payments: u64,
    pub pending: u64,
    pub settled: u64,
    pub failed: u64,
    /// The claimant members drawn to lose on cleared lines whose payers failed.
    pub lost: u64,
    /// The claimant members drawn past the failed members, a unit being wider than what remained to draw.
    pub lost_past_failed: u64,
    pub iterations: u64,
    pub gross: i128,
    /// The closing ring: the parties whose settled payments their own funds could not cover without the day's
    /// credits, and the part of their payments those credits paid.
    pub ring_parties: u64,
    pub ring_value: i128,
    pub buffer_bytes: u64,
    /// Parties that could not pay what settled, given the other settled payments.
    pub unsound: u64,
    /// Parties whose payment failed though they could pay it, given the payments that settled.
    pub not_maximal: u64,
    /// Accounts whose movement is not the net of the payments applied to them.
    pub nets_missed: u64,
    /// Banks whose reserves did not move by the net of the settled payments between their customers, themselves
    /// among them, and parties that settle elsewhere.
    pub reserves_missed: u64,
    /// The holders whose runs were read in full today, and those whose run did not hold: a row that can still fall
    /// due outside its segment, a head later than the earliest, or a holder with a row due today not scanned.
    pub runs_read: u64,
    pub runs_broken: u64,
}

/// A net leg's key: the line and side it lies on, its party, and whether it moves the contract's rows rather than
/// money. Lines come first, so a line's legs are adjacent and each line is applied as its legs are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NetKey {
    line: LineId,
    party: PartyId,
    side: Side,
    row: bool,
}

impl core::hash::Hash for NetKey {
    fn hash<H: core::hash::Hasher>(&self, h: &mut H) {
        self.line.hash(h);
        self.party.hash(h);
        (self.side == Side::Asset).hash(h);
        self.row.hash(h);
    }
}

impl phx_core::MapKey for NetKey {
    fn key64(self) -> u64 {
        (u64::from(self.line.get()) << u32::BITS) ^ self.party.get()
    }
}

/// What the day's dues were due on: the lines, the day and the calendar that dates them.
type Today<'a> = (&'a DueLines, Day, &'a Calendar);

/// The day's payments gathered after the fixed point, per account and party and never per payment: the net of each
/// account and row, each party's record given the payments that settle, each bank's net across its customers, and
/// the payers that failed.
#[derive(Default)]
struct Gathered {
    nets: Vec<(NetKey, i128)>,
    given: Records,
    crossing: BTreeMap<(PartyId, u8), i128>,
    failed_payers: BTreeSet<PartyId>,
    settled: u64,
    failed: u64,
}

fn bytes<T>(n: usize) -> u64 {
    phx_rand::float::len_u64(n * std::mem::size_of::<T>())
}

fn count(n: usize) -> u64 {
    phx_rand::float::len_u64(n)
}

/// A payment the stream made as the fixed point left it: a cleared line's claimant is paid for its members not
/// drawn to lose, and a claimant all of whose members were drawn is paid nothing.
fn after_losers(p: &Payment, found: &Found) -> Option<Payment> {
    if !(p.cleared && p.payee == p.reckoned_on) {
        return Some(*p);
    }
    let Some(day) = found.cleared.get(p.line) else {
        violation!(clause = "REP.23", "a cleared payment on a line never reckoned", line = p.line.get());
    };
    let members = p.members - day.lost(p.reckoned_on);
    let amount = crate::cleared::times(day.per_member, members);
    (amount != 0).then_some(Payment { amount, members, ..*p })
}

impl<B: Backing> Books<B> {
    /// The party whose reserves a party's payments move: a party holding reserves itself, a depositor its bank, and
    /// a party with no account in the currency, which pays in its own money, itself.
    fn settles_at(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> PartyId {
        match self.money_row(party, ccy, found) {
            Missing::Present(line) if !self.ledger.lines.is_reserves(line) => self.owed_by(line, party, found),
            _ => party,
        }
    }

    /// A party's reserves in a currency, when it holds them.
    fn reserves_of(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> Option<i128> {
        let Missing::Present(line) = self.money_row(party, ccy, found) else { return None };
        if !self.ledger.lines.is_reserves(line) {
            return None;
        }
        match self.row_on_side(party, line, Side::Asset).map(|r| r.optional.balance) {
            Some(Missing::Present(b)) => Some(i128::from(b)),
            _ => None,
        }
    }

    /// How many of the sampled holders' runs do not hold against their rows, read before their heads are rewritten,
    /// and how many were read.
    fn runs_broken(&self, due: &DueLines, day: Day, scanned: &BTreeSet<(u16, Slot)>) -> (u64, u64) {
        let (mut read, mut broken) = (0_u64, 0_u64);
        for place in self.parties.places() {
            let table = self.parties.holder(place);
            for slot in phx_store::table::live_in(table.live_words())
                .filter(|s| s.get() % RUN_SAMPLE_PERIOD == day.get() % RUN_SAMPLE_PERIOD)
            {
                let t = runs::truth(table, slot, day, due, &self.ledger.lines);
                read += 1;
                if !t.holds || (t.due > 0 && !scanned.contains(&(place, slot))) {
                    broken += 1;
                }
            }
        }
        (read, broken)
    }

    /// A due that fell today recorded for the accounts: the interest in it, which the payee earned today, and the
    /// principal it repays, with what became of it.
    fn record_due(&mut self, p: &Payment, outcome: DueOutcome) {
        self.ledger.record_due(DueRec {
            line: p.line,
            payer: p.payer,
            payee: p.payee,
            interest: phx_num::Money::new(p.amount - p.principal, p.ccy),
            principal: phx_num::Money::new(p.principal, p.ccy),
            outcome,
        });
    }

    /// A payment the fixed point failed, recorded for the contract process.
    fn fail_payment(&mut self, p: &Payment, day: Day, cause: FailCause) {
        let reason = if p.principal != 0 { self.dues.principal } else { self.dues.payment };
        let instruction = self.ledger.next_id(day);
        self.ledger.record_fail(Fail {
            instruction,
            reason,
            cause,
            party: p.payer,
            due: day,
            row: Missing::Present(DueRow { line: p.line, side: Side::Liability }),
        });
    }

    /// The day's payments in the stream's order: one through a closed issuer is left for the pending pass, one the
    /// fixed point failed is recorded, and one that settles adds its legs to the nets.
    fn gather(
        &mut self,
        streamed: &DayRecords,
        fixed: &FixedPoint,
        today: Today<'_>,
        closed: &Closed,
        found: &mut Found,
    ) -> Gathered {
        let (_, day, _) = today;
        let mut g = Gathered::default();
        let mut nets: phx_core::KernelMap<NetKey, i128> = phx_core::KernelMap::new();
        for made in &streamed.made {
            let Some(p) = after_losers(made, found) else { continue };
            let route = self.effects(&p, found);
            if closed.holds(p.payer, &crate::stream::issuers(&route)) {
                self.record_due(&p, DueOutcome::Pending);
                continue;
            }
            if fixed.failed.contains(&p.key()) {
                g.failed += 1;
                if !fixed.by_bank.contains(&p.key()) && !p.moneyless {
                    g.failed_payers.insert(p.payer);
                }
                // A cleared line's claimant with no money loses its due against the top issuer, which owes no row.
                if !(p.cleared && p.payee == p.reckoned_on && p.moneyless) {
                    let cause = if p.moneyless {
                        FailCause::NoMoney
                    } else if fixed.by_bank.contains(&p.key()) {
                        FailCause::BankShort
                    } else {
                        FailCause::Funds
                    };
                    self.fail_payment(&p, day, cause);
                }
                self.record_due(&p, DueOutcome::Failed);
                continue;
            }
            g.settled += 1;
            self.record_due(&p, DueOutcome::Settled);
            let _ = self.book(&mut g.given, &route, 1);
            let (from, to) = (self.settles_at(p.payer, p.ccy, found), self.settles_at(p.payee, p.ccy, found));
            if from != to {
                *g.crossing.entry((from, p.ccy.index())).or_insert(0) -= i128::from(p.amount);
                *g.crossing.entry((to, p.ccy.index())).or_insert(0) += i128::from(p.amount);
            }
            for leg in route {
                let AccountRef::Line { line, side } = leg.account else {
                    violation!(clause = "SET.1", "a payment's leg on no line", party = leg.party.get());
                };
                let is_row = matches!(leg.kind, LegKind::Row(_));
                let key = NetKey { line, party: leg.party, side, row: is_row };
                match nets.get_mut(key) {
                    Some(q) => *q += i128::from(leg.qty),
                    None => {
                        let _ = nets.insert(key, i128::from(leg.qty));
                    }
                }
                if !is_row && side == Side::Asset && self.ledger.lines.is_reserves(line) {
                    let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
                    let _ = g.crossing.entry((leg.party, ccy.index())).or_insert(0);
                }
            }
        }
        g.nets = nets.drain_sorted();
        g
    }

    /// The dues the claimant members drawn on cleared lines lost, recorded as failed against the top issuer, which
    /// holds the failed payers' dues; the members drawn, and those drawn past the failed, which a claimant unit wider
    /// than what remained to draw leaves with the top issuer.
    #[clause("REP.23", "ACC.1")]
    fn record_lost(&mut self, found: &Found) -> (u64, u64) {
        let (mut members, mut past) = (0_u64, 0_u64);
        for (line, c) in found.cleared.sorted() {
            let Some(losers) = &c.losers else { continue };
            let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
            for (claimant, k) in losers.all_lost() {
                self.ledger.record_due(DueRec {
                    line,
                    payer: c.top,
                    payee: claimant,
                    interest: phx_num::Money::new(crate::cleared::times(c.per_member, k), ccy),
                    principal: phx_num::Money::new(0, ccy),
                    outcome: DueOutcome::Failed,
                });
            }
            members += losers.drawn();
            past += losers.drawn() - c.failed;
        }
        (members, past)
    }

    /// The nets applied, one instruction per line as its legs are read in order, so no batch of legs is held.
    fn apply_nets(&mut self, nets: &[(NetKey, i128)], day: Day, audit: &mut dyn AuditStream) {
        let mut legs: Vec<LegRec> = Vec::new();
        let mut entries = nets.iter().peekable();
        while let Some((NetKey { line, party, side, row: is_row }, q)) = entries.next() {
            let (line, party, side, is_row) = (*line, *party, *side, *is_row);
            if *q != 0 {
                let Ok(qty) = i64::try_from(*q) else {
                    phx_num::capacity_exceeded!("a day's net on one account", i64::MAX, 0);
                };
                let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
                let kind = if is_row { LegKind::Row(RowOp::Adjust) } else { LegKind::Money };
                legs.push(LegRec {
                    party,
                    account: AccountRef::Line { line, side },
                    qty,
                    denom: Denom::Ccy(ccy),
                    kind,
                });
            }
            if entries.peek().is_some_and(|(k, _)| k.line == line) || legs.is_empty() {
                continue;
            }
            let rows = legs.iter().any(|l| matches!(l.kind, LegKind::Row(_)));
            let instruction = Instruction {
                id: self.ledger.next_id(day),
                reason: if rows { self.dues.principal } else { self.dues.payment },
                trade_day: day,
                settle_day: day,
                legs: std::mem::take(&mut legs),
                pays: Missing::Absent,
                covers: Vec::new(),
            };
            if let Err(f) = self.ledger.apply(&mut self.parties, ApplyAt::Day(SubStep::S7c), instruction, audit) {
                violation!(
                    clause = "SET.6",
                    "a net the fixed point let stand would not settle",
                    line = line.get(),
                    party = f.party.get()
                );
            }
        }
    }

    /// Stage 7 over the books: the stream (7a), the fixed point (7b), and the apply (7c) of every surviving payment as
    /// one net leg per account and row, one instruction per line, so each account is checked alone and no order of
    /// application can fail what the fixed point let stand; reserves move once per bank by net. The payments that fail
    /// are recorded for the contract process, and each scanned holder's head is rewritten.
    #[clause("MON.5", "SET.4", "SET.5", "SET.6", "SET.10")]
    pub fn settle_day(
        &mut self,
        due: &DueLines,
        day: Day,
        calendar: &Calendar,
        closed: &Closed,
        draws_of: &dyn Fn(LineId) -> phx_rand::Draws,
        audit: &mut dyn AuditStream,
    ) -> DaySettlement {
        let mut found = Found::default();
        let mut streamed = self.stream(due, day, calendar, closed, &mut found);
        let fixed = self.fixed_point(&mut streamed, due, day, calendar, &mut found, draws_of);
        let scanned: BTreeSet<(u16, Slot)> = streamed.scanned.iter().copied().collect();
        let (runs_read, runs_broken) = self.runs_broken(due, day, &scanned);
        let g = self.gather(&streamed, &fixed, (due, day, calendar), closed, &mut found);
        let (lost, lost_past_failed) = self.record_lost(&found);
        let given = g.given.sorted();
        let unsound = count(given.iter().filter(|(_, r)| r.standing() < 0).count());
        let (ring_parties, ring_value) = given
            .iter()
            .map(|(_, r)| *r)
            .filter(|r| r.debit > r.funds)
            .fold((0_u64, 0_i128), |(n, v), r| (n + 1, v + r.debit - r.funds));
        let not_maximal = self.not_maximal(&g.failed_payers, &fixed, &g.given, (due, day, calendar), &mut found);
        let reserves_before: Vec<((PartyId, u8), Option<i128>)> =
            g.crossing.keys().map(|&(p, c)| ((p, c), self.reserves_of(p, Ccy::new(c), &mut found))).collect();
        let before = self.balances(&g.nets);
        let buffer_bytes = bytes::<(PartyId, Record)>(streamed.records.len() + g.given.len())
            + bytes::<(u16, Slot)>(streamed.scanned.len() + scanned.len())
            + bytes::<Payment>(streamed.made.len())
            + bytes::<(NetKey, i128)>(g.nets.len())
            + bytes::<Option<i128>>(g.nets.len() * 2)
            + bytes::<((PartyId, u8), i128)>(g.crossing.len() + reserves_before.len())
            + bytes::<(LineId, PartyId)>(fixed.failed.len() + fixed.by_bank.len());
        self.apply_nets(&g.nets, day, audit);
        let after = self.balances(&g.nets);
        let nets_missed = count(
            g.nets
                .iter()
                .zip(before.iter().zip(&after))
                .filter(|((k, q), (b, a))| !k.row && a.zip(**b).is_none_or(|(a, b)| a - b != *q))
                .count(),
        );
        let reserves_missed = count(
            reserves_before
                .iter()
                .filter(|(key, was)| match (was, self.reserves_of(key.0, Ccy::new(key.1), &mut found)) {
                    (Some(was), Some(now)) => g.crossing.get(key).is_none_or(|net| now - was != *net),
                    (None, None) => false,
                    _ => true,
                })
                .count(),
        );
        if !closed.is_empty() {
            self.hold_all(&streamed, (due, day, calendar), closed, &mut found);
        }
        for &(place, slot) in &streamed.scanned {
            runs::rehead(crate::apply::Holders::arenas(&mut self.parties, place), slot, &self.ledger.lines);
        }
        DaySettlement {
            lines: count(due.lines().len()),
            heads_read: streamed.heads_read,
            rows_scanned: streamed.rows_scanned,
            rows_due: streamed.rows_due,
            payments: streamed.payments,
            pending: streamed.pending,
            settled: g.settled,
            failed: g.failed,
            lost,
            lost_past_failed,
            iterations: fixed.iterations,
            gross: streamed.gross,
            ring_parties,
            ring_value,
            buffer_bytes,
            unsound,
            not_maximal,
            nets_missed,
            reserves_missed,
            runs_read,
            runs_broken,
        }
    }

    /// How many payers that failed were not short: at its first failed payment in its own order, what a payer held
    /// given the payments that settle could pay it. A payment removed because a bank on its way was short is the bank's.
    fn not_maximal(
        &self,
        payers: &BTreeSet<PartyId>,
        fixed: &FixedPoint,
        given: &Records,
        (due, day, calendar): Today<'_>,
        found: &mut Found,
    ) -> u64 {
        let short = |payer: PartyId, found: &mut Found| {
            let first = self
                .payments_of(payer, due, day, calendar, found)
                .into_iter()
                .find(|p| p.payer == payer && fixed.failed.contains(&p.key()) && !fixed.by_bank.contains(&p.key()));
            let Some(p) = first else { return false };
            let legs = self.effects(&p, found);
            let Missing::Present(account) = self.money_row(payer, p.ccy, found) else { return false };
            let rec = given.get(payer).copied().unwrap_or_else(|| self.record_of(payer, account));
            rec.standing() < crate::fixed_point::draw(&legs, payer, rec.account)
        };
        count(payers.iter().filter(|p| !short(**p, found)).count())
    }

    /// The pending pass, after the day's nets are applied, so what a party could draw on at 7a is what the fixed point
    /// and the verdicts read: each scanned holder's payments through a closed issuer held pending.
    fn hold_all(&mut self, streamed: &DayRecords, (due, day, calendar): Today<'_>, closed: &Closed, found: &mut Found) {
        for &(place, slot) in &streamed.scanned {
            let holder = self.parties.holder(place).party(slot);
            for row in self.due_rows_of(holder, due) {
                let Some(p) = self.payment(holder, &row, day, calendar, found) else { continue };
                let route = self.effects(&p, found);
                if closed.holds(p.payer, &crate::stream::issuers(&route)) {
                    self.hold_pending(&route);
                }
            }
        }
    }

    /// A payment held pending: its amount recorded as pending on each deposit row its money would move, which the
    /// payer's funds exclude and the payee's count for nothing until it settles or fails.
    #[clause("MON.5", "SET.2")]
    fn hold_pending(&mut self, route: &[LegRec]) {
        for leg in route.iter().filter(|l| matches!(l.kind, LegKind::Money)) {
            let AccountRef::Line { line, side: Side::Asset } = leg.account else { continue };
            let (place, slot) = self.parties.row(leg.party);
            let arenas = crate::apply::Holders::arenas(&mut self.parties, place);
            let Some(view) = crate::rows::iter(arenas, slot).find(|r| r.row.line == line && r.side() == Side::Asset)
            else {
                violation!(
                    clause = "MON.5",
                    "a pending amount on an account its party does not hold",
                    line = line.get()
                );
            };
            let Missing::Present(pending) = view.optional.pending else { continue };
            let mut optional = view.optional;
            optional.pending = Missing::Present(pending + leg.qty.abs());
            crate::line::Lines::<B>::set_words(arenas, slot, line, Side::Asset, optional);
        }
    }

    /// The balances of the money accounts a day's nets move, read before or after they apply.
    fn balances(&self, nets: &[(NetKey, i128)]) -> Vec<Option<i128>> {
        nets.iter()
            .map(|(k, _)| {
                if k.row {
                    return None;
                }
                let (place, slot) = self.parties.row(k.party);
                let row = crate::rows::iter(self.parties.holder(place), slot)
                    .find(|r| r.row.line == k.line && r.side() == k.side);
                match row.map(|r| r.optional.balance) {
                    Some(Missing::Present(b)) => Some(i128::from(b)),
                    _ => None,
                }
            })
            .collect()
    }
}
