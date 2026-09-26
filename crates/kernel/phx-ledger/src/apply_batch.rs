use std::collections::{BTreeMap, BTreeSet};

use phx_core::calendar::Calendar;
use phx_core::{AuditStream, SubStep};
use phx_id::{Day, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::{ApplyAt, Located};
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

/// One shard of 7c's payments before it is folded: where the payments held pending (true) or failed lie in the day's
/// payments, in the stream's order, and what the settled ones add, each put with the shard of the sum it adds to.
#[derive(Default)]
struct Routed {
    unsettled: Vec<(usize, bool)>,
    settled: u64,
    given: Vec<Vec<crate::stream::Booking>>,
    nets: Vec<Vec<(NetKey, i64)>>,
    earned: Vec<Vec<(PartyId, i128)>>,
    crossing: BTreeMap<(PartyId, u8), i128>,
}

/// A keyed value put with its shard.
fn push<T>(buckets: &mut [Vec<T>], shard: usize, value: T) {
    let Some(bucket) = buckets.get_mut(shard) else {
        phx_num::capacity_exceeded!("keyed shards", buckets.len(), shard);
    };
    bucket.push(value);
}

/// The day's buffers, kept on the books from one stage 7 to the next and emptied, never freed, so a day maps no new
/// pages once the heaviest day has sized them.
#[derive(Debug, Default)]
pub(crate) struct DayBuffers {
    records: Records,
    given: Records,
    made: Vec<Payment>,
    scanned: Vec<(u16, Slot)>,
    nets: phx_core::KernelMap<NetKey, i128>,
    net_list: Vec<(NetKey, i128)>,
}

impl DayBuffers {
    /// The day's stream buffers, emptied, handed to 7a.
    fn stream_buffers(&mut self) -> DayRecords {
        let mut records = std::mem::take(&mut self.records);
        records.clear();
        let mut made = std::mem::take(&mut self.made);
        made.clear();
        let mut scanned = std::mem::take(&mut self.scanned);
        scanned.clear();
        DayRecords { records, scanned, made, ..DayRecords::default() }
    }

    /// The day's buffers taken back at the close of stage 7.
    fn keep(&mut self, streamed: DayRecords, g: Gathered, nets: phx_core::KernelMap<NetKey, i128>) {
        self.records = streamed.records;
        self.made = streamed.made;
        self.scanned = streamed.scanned;
        self.given = g.given;
        self.net_list = g.nets;
        self.nets = nets;
    }
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
    fn settles_at(&self, party: PartyId, ccy: Ccy) -> PartyId {
        match self.money_row(party, ccy) {
            Missing::Present(line) if !self.ledger.lines.is_reserves(line) => self.owed_by(line),
            _ => party,
        }
    }

    /// A party's reserves in a currency, when it holds them.
    fn reserves_of(&self, party: PartyId, ccy: Ccy) -> Option<i128> {
        let Missing::Present(line) = self.money_row(party, ccy) else { return None };
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
    fn runs_broken(&self, due: &DueLines, day: Day, scanned: &[(u16, Slot)]) -> (u64, u64, usize) {
        let phase = day.get() % RUN_SAMPLE_PERIOD;
        // Only today's slice of holders is sampled, so only the scanned among them are kept, sorted to be found.
        let mut sampled: Vec<(u16, Slot)> =
            scanned.iter().copied().filter(|(_, s)| s.get() % RUN_SAMPLE_PERIOD == phase).collect();
        sampled.sort_unstable();
        let (mut read, mut broken) = (0_u64, 0_u64);
        for place in self.parties.places() {
            let table = self.parties.holder(place);
            for slot in phx_store::table::live_every(table.live_words(), phase, RUN_SAMPLE_PERIOD) {
                let t = runs::truth(table, slot, day, due, &self.ledger.lines);
                read += 1;
                if !t.holds || (t.due > 0 && sampled.binary_search(&(place, slot)).is_err()) {
                    broken += 1;
                }
            }
        }
        (read, broken, sampled.capacity())
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
    /// fixed point failed is recorded, and one that settles adds its interest to its parties' income, its legs to the
    /// nets and its bookings to the records given the settled payments. Each wave of shards is routed on the pool and
    /// what it adds folded a shard of each sum to a worker; the payments left or failed are recorded in order after.
    fn gather(
        &mut self,
        streamed: &DayRecords,
        fixed: &FixedPoint,
        today: Today<'_>,
        closed: &Closed,
        found: &mut Found,
    ) -> (Gathered, phx_core::KernelMap<NetKey, i128>)
    where
        B: Sync,
    {
        let (_, day, _) = today;
        let mut given = std::mem::take(&mut self.buffers.given);
        given.clear();
        let mut net_list = std::mem::take(&mut self.buffers.net_list);
        net_list.clear();
        let mut g = Gathered { given, nets: net_list, ..Gathered::default() };
        let mut nets = std::mem::take(&mut self.buffers.nets);
        nets.clear();
        let each = streamed.made.len().div_ceil(crate::consts::ROUTE_SHARDS);
        // Waves past the last payment hold nothing, so a small day pays for its payments, not for the shards.
        for wave in (0..crate::consts::ROUTE_SHARDS)
            .step_by(crate::consts::STREAM_WAVE)
            .take_while(|w| w * each < streamed.made.len())
        {
            let found_now: &Found = found;
            let mut routed = phx_exec::pool::map(self.pool.as_deref(), crate::consts::STREAM_WAVE, |i| {
                let at_most = |a: usize, b: usize| if a < b { a } else { b };
                let from = at_most((wave + i) * each, streamed.made.len());
                let to = at_most(from + each, streamed.made.len());
                self.route_shard(streamed.made.get(from..to).unwrap_or(&[]), from, fixed, (closed, found_now))
            });
            let pool = self.pool.as_deref();
            let bookings: Vec<_> = routed.iter_mut().map(|r| std::mem::take(&mut r.given)).collect();
            g.given.fold(pool, &bookings, &|party, account| self.record_of(party, account));
            let legs: Vec<_> = routed.iter_mut().map(|r| std::mem::take(&mut r.nets)).collect();
            nets.fold(pool, &legs, || 0, |q, leg| *q += i128::from(*leg));
            let earned: Vec<_> = routed.iter_mut().map(|r| std::mem::take(&mut r.earned)).collect();
            self.ledger.day.earned.fold(pool, &earned, || 0, |v, add| *v += add);
            for r in routed {
                g.settled += r.settled;
                for (key, q) in r.crossing {
                    *g.crossing.entry(key).or_insert(0) += q;
                }
                for (at, pending) in r.unsettled {
                    let Some(p) = streamed.made.get(at).and_then(|made| after_losers(made, found)) else {
                        violation!(clause = "SET.6", "an unsettled payment the day did not make", at = at);
                    };
                    if pending {
                        self.record_due(&p, DueOutcome::Pending);
                        continue;
                    }
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
                }
            }
        }
        g.nets.extend(nets.drain_sorted());
        (g, nets)
    }

    /// One shard of 7c's payments routed: those held pending or failed kept in order, and what the settled ones add,
    /// each put with the shard of the sum it adds to.
    fn route_shard(
        &self,
        made: &[Payment],
        first: usize,
        fixed: &FixedPoint,
        (closed, found): (&Closed, &Found),
    ) -> Routed {
        if made.is_empty() {
            return Routed::default();
        }
        let mut r = Routed {
            given: crate::stream::booking_buckets(),
            nets: phx_core::KernelMap::<NetKey, i128>::buckets(),
            earned: phx_core::KernelMap::<PartyId, i128>::buckets(),
            ..Routed::default()
        };
        for (at, p) in (first..).zip(made).filter_map(|(at, made)| after_losers(made, found).map(|p| (at, p))) {
            let route = self.effects(&p);
            if closed.holds(p.payer, &crate::stream::issuers(&route)) {
                r.unsettled.push((at, true));
                continue;
            }
            if fixed.failed.contains(&p.key()) {
                r.unsettled.push((at, false));
                continue;
            }
            r.settled += 1;
            let interest = i128::from(p.amount - p.principal);
            for (party, v) in [(p.payee, interest), (p.payer, -interest)] {
                push(&mut r.earned, phx_core::KernelMap::<PartyId, i128>::shard_of(party), (party, v));
            }
            self.bookings(&route, &mut r.given);
            let (from, to) = (self.settles_at(p.payer, p.ccy), self.settles_at(p.payee, p.ccy));
            if from != to {
                *r.crossing.entry((from, p.ccy.index())).or_insert(0) -= i128::from(p.amount);
                *r.crossing.entry((to, p.ccy.index())).or_insert(0) += i128::from(p.amount);
            }
            for leg in route {
                let AccountRef::Line { line, side } = leg.account else {
                    violation!(clause = "SET.1", "a payment's leg on no line", party = leg.party.get());
                };
                let is_row = matches!(leg.kind, LegKind::Row(_));
                let key = NetKey { line, party: leg.party, side, row: is_row };
                push(&mut r.nets, phx_core::KernelMap::<NetKey, i128>::shard_of(key), (key, leg.qty));
                if !is_row && side == Side::Asset && self.ledger.lines.is_reserves(line) {
                    let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
                    let _ = r.crossing.entry((leg.party, ccy.index())).or_insert(0);
                }
            }
        }
        r
    }

    /// An instruction's legs read on the pool in fixed shards before anything moves, as `apply` reads them: each leg's
    /// party located, or the first leg whose party has ended, then what each leg draws.
    fn read_legs(&self, legs: &[LegRec]) -> crate::apply::LegReads
    where
        B: Sync,
    {
        let each = legs.len().div_ceil(crate::consts::STREAM_SHARDS);
        let chunk = |i: usize| {
            let from = if i * each < legs.len() { i * each } else { legs.len() };
            let to = if from + each < legs.len() { from + each } else { legs.len() };
            (from, legs.get(from..to).unwrap_or(&[]))
        };
        let located = phx_exec::pool::map(self.pool.as_deref(), crate::consts::STREAM_SHARDS, |i| {
            chunk(i).1.iter().map(|l| crate::apply::Holders::locate(&self.parties, l.party)).collect::<Vec<_>>()
        });
        let mut at = Vec::with_capacity(legs.len());
        for (n, l) in located.into_iter().flatten().enumerate() {
            match l {
                Located::Live { party, table, slot } => at.push(crate::apply::At { party, table, slot }),
                Located::Ended => return Err(n),
            }
        }
        let opened = crate::apply::opened(legs);
        let located = &at;
        let drawn = phx_exec::pool::map(self.pool.as_deref(), crate::consts::STREAM_SHARDS, |i| {
            let (from, of) = chunk(i);
            of.iter()
                .zip(located.get(from..).unwrap_or(&[]))
                .map(|(leg, a)| self.ledger.leg_draw(self.parties.holder(a.table), *a, leg, &opened))
                .collect::<Vec<_>>()
        });
        Ok((at, drawn.concat()))
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
    fn apply_nets(&mut self, nets: &[(NetKey, i128)], day: Day, audit: &mut dyn AuditStream)
    where
        B: Sync,
    {
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
            let reads = self.read_legs(&instruction.legs);
            let applied =
                self.ledger.apply_read(&mut self.parties, ApplyAt::Day(SubStep::S7c), instruction, reads, audit);
            if let Err(f) = applied {
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
    ) -> DaySettlement
    where
        B: Sync,
    {
        let mut found = Found::default();
        let heads = self.ledger.lines.take_heads(day);
        let buffers = self.buffers.stream_buffers();
        let mut streamed = self.stream(&heads, buffers, due, (day, calendar), closed, &mut found);
        let fixed = self.fixed_point(&mut streamed, due, day, calendar, &mut found, draws_of);
        let (runs_read, runs_broken, sampled) = self.runs_broken(due, day, &streamed.scanned);
        let (g, nets) = self.gather(&streamed, &fixed, (due, day, calendar), closed, &mut found);
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
            g.crossing.keys().map(|&(p, c)| ((p, c), self.reserves_of(p, Ccy::new(c)))).collect();
        let before = self.balances(&g.nets);
        // Every day buffer by the room it holds, grown or not: what the phone must find free at the day's peak.
        let buffer_bytes = bytes::<(u32, Option<Record>)>(streamed.records.capacity() + g.given.capacity())
            + bytes::<(PartyId, u16, Slot)>(streamed.records.touched() + g.given.touched())
            + bytes::<(u16, Slot)>(streamed.scanned.capacity() + sampled)
            + bytes::<Payment>(streamed.made.capacity())
            + bytes::<(NetKey, i128)>(g.nets.capacity())
            + bytes::<Option<i64>>(before.capacity())
            + bytes::<((PartyId, u8), i128)>(g.crossing.len() + reserves_before.capacity())
            + bytes::<(LineId, PartyId)>(fixed.failed.len() + fixed.by_bank.len())
            + bytes::<PartyId>(g.failed_payers.len() + streamed.moneyless.len())
            + bytes::<u8>(found.bytes() + self.ledger.day.bytes());
        self.apply_nets(&g.nets, day, audit);
        let nets_missed = self.nets_missed(&g.nets, &before);
        let reserves_missed = count(
            reserves_before
                .iter()
                .filter(|(key, was)| match (was, self.reserves_of(key.0, Ccy::new(key.1))) {
                    (Some(was), Some(now)) => g.crossing.get(key).is_none_or(|net| now - was != *net),
                    (None, None) => false,
                    _ => true,
                })
                .count(),
        );
        if !closed.is_empty() {
            self.hold_all(&streamed, (due, day, calendar), closed, &mut found);
        }
        self.reheads(&streamed.scanned);
        let settled = DaySettlement {
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
        };
        self.buffers.keep(streamed, g, nets);
        settled
    }

    /// Each scanned holder's head rewritten: a wave of fixed shards' next days read on the pool, then the heads written
    /// and filed in the stream's order; a holder with a spent row has its rows moved on the calling thread.
    fn reheads(&mut self, scanned: &[(u16, Slot)])
    where
        B: Sync,
    {
        let at_most = |a: usize, b: usize| if a < b { a } else { b };
        let each = scanned.len().div_ceil(crate::consts::STREAM_SHARDS);
        for wave in (0..crate::consts::STREAM_SHARDS)
            .step_by(crate::consts::STREAM_WAVE)
            .take_while(|w| w * each < scanned.len())
        {
            let read = phx_exec::pool::map(self.pool.as_deref(), crate::consts::STREAM_WAVE, |i| {
                let from = at_most((wave + i) * each, scanned.len());
                let to = at_most(from + each, scanned.len());
                let lines = &self.ledger.lines;
                scanned
                    .get(from..to)
                    .unwrap_or(&[])
                    .iter()
                    .map(|&(place, slot)| runs::next_head(self.parties.holder(place), slot, lines))
                    .collect::<Vec<_>>()
            });
            let from = at_most(wave * each, scanned.len());
            for (&(place, slot), next) in scanned.get(from..).unwrap_or(&[]).iter().zip(read.into_iter().flatten()) {
                let arenas = crate::apply::Holders::arenas(&mut self.parties, place);
                match next {
                    Some(least) => {
                        let mut head = arenas.run_head(slot);
                        if let Some(day) = least {
                            head.next_due = day;
                            arenas.set_run_head(slot, head);
                            self.ledger.lines.file_head(place, slot, day);
                        }
                    }
                    None => runs::rehead(arenas, place, slot, &mut self.ledger.lines),
                }
            }
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
            let legs = self.effects(&p);
            let Missing::Present(account) = self.money_row(payer, p.ccy) else { return false };
            let rec = given.get(self.parties.row(payer)).copied().unwrap_or_else(|| self.record_of(payer, account));
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
                let route = self.effects(&p);
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
            let Some(view) = crate::rows::find(arenas, slot, line, Side::Asset) else {
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

    /// The balances of the money accounts a day's nets move, read before they apply, on the pool in fixed shards.
    fn balances(&self, nets: &[(NetKey, i128)]) -> Vec<Option<i64>>
    where
        B: Sync,
    {
        let mut out: Vec<Option<i64>> = vec![None; nets.len()];
        let each = nets.len().div_ceil(crate::consts::STREAM_SHARDS);
        if each > 0 {
            let shards = nets.chunks(each).zip(out.chunks_mut(each));
            phx_exec::pool::each(self.pool.as_deref(), shards, |(keys, read)| {
                for ((k, _), b) in keys.iter().zip(read.iter_mut()) {
                    *b = self.balance_of(k);
                }
            });
        }
        out
    }

    /// The money nets whose account did not move by the net, read against the balances before the apply, on the pool
    /// by fixed shards; no balances after are held, since each is compared as it is read.
    fn nets_missed(&self, nets: &[(NetKey, i128)], before: &[Option<i64>]) -> u64
    where
        B: Sync,
    {
        let each = nets.len().div_ceil(crate::consts::STREAM_SHARDS);
        if each == 0 {
            return 0;
        }
        let mut missed: Vec<usize> = vec![0; nets.len().div_ceil(each)];
        let shards = nets.chunks(each).zip(before.chunks(each)).zip(missed.iter_mut());
        phx_exec::pool::each(self.pool.as_deref(), shards, |((keys, was), n)| {
            *n = keys
                .iter()
                .zip(was)
                .filter(|((k, q), b)| {
                    !k.row && self.balance_of(k).zip(**b).is_none_or(|(a, b)| i128::from(a) - i128::from(b) != *q)
                })
                .count();
        });
        count(missed.iter().sum::<usize>())
    }

    /// The balance of the money account a net moves; none for a net on a contract's rows.
    fn balance_of(&self, k: &NetKey) -> Option<i64> {
        if k.row {
            return None;
        }
        let (place, slot) = self.parties.row(k.party);
        match crate::rows::find(self.parties.holder(place), slot, k.line, k.side).map(|r| r.optional.balance) {
            Some(Missing::Present(b)) => Some(b),
            _ => None,
        }
    }
}
