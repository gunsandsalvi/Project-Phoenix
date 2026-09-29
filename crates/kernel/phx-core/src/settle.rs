//! Settlement on the core: a currency's flows of the day netted per party by range, the parties the nets leave short
//! found, and their flows failed in payment order until every flow left can settle given the others; the nets are then
//! added to the accounts in place. A bank's reserves move by what its customers pay out of it and are paid into it.

use phx_exec::Pool;
use phx_id::{PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;

use crate::consts::BIT_WORD;
use crate::flows::{Flow, Grouped, Ranges};

/// The bank of an account held at the currency's issuer, whose payments move no bank's reserves.
pub const AT_ISSUER: u32 = u32::MAX;

/// One kind's accounts in one currency, a value a slot of the kind. For the kind of the banks, `balance` is each
/// bank's reserves at the issuer and `bank` is not read.
#[derive(Debug)]
pub struct Book<'a> {
    /// The bank each account is held at, by the bank's slot, or `AT_ISSUER`.
    pub bank: &'a [u32],
    pub balance: &'a mut [i64],
    /// Card payments of the closed days before, commitments that settle first on the next business day.
    pub pending: &'a mut [i64],
    /// What each account has paid through a closed bank, held until the bank's fate is known; never drawn on.
    pub held: &'a mut [i64],
    /// How far each account may fall below nothing, as its terms grant.
    pub facility: &'a [i64],
    /// Where each party's settled flows are posted, if the kind keeps lines of them.
    pub lines: Option<Lines<'a>>,
}

/// A kind's parties' lines of the cash their statements report: `width` lines a party, and the line a flow of each
/// reason adds its amount to when the party pays it and when the party is paid it. A reason with no line on a side
/// feeds none there.
#[clause("ACC.9")]
#[derive(Debug)]
pub struct Lines<'a> {
    pub amounts: &'a mut [i64],
    pub width: usize,
    pub paid: &'a [Option<usize>],
    pub received: &'a [Option<usize>],
}

impl Lines<'_> {
    fn post(amounts: &mut [i64], width: usize, at: usize, line: Option<usize>, amount: i64) {
        let Some(l) = line else { return };
        let Some(cell) = amounts.get_mut(at * width + l) else {
            violation!(clause = "ACC.9", "a posting past its party's lines", line = l);
        };
        *cell += amount;
    }
}

/// Every kind's accounts in one currency, the kind whose parties are the banks, the banks closed today, and the
/// currency's issuer, whose own payments are never short.
#[derive(Debug)]
pub struct Books<'a> {
    pub kinds: Vec<Option<Book<'a>>>,
    pub banks: u8,
    /// Each bank's deposits, what it owes its customers: moved by their nets, so it is always the sum of their
    /// accounts and what they have pending.
    pub deposits: &'a mut [i64],
    pub closed: &'a [bool],
    pub issuer: PartyKey,
}

/// Why a flow did not settle: its payer could not pay it, or it passed through a bank that could not cover its net,
/// which is the bank's to answer for and not the payer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Payer,
    Bank,
}

/// What a day's settlement did: the flows settled, those failed with why, those held through closed banks, and the
/// parties the worklist took.
#[derive(Debug, Default)]
pub struct Outcome {
    pub settled: u64,
    pub failed: Vec<(Flow, Cause)>,
    pub held: Vec<Flow>,
    pub visits: u64,
    /// The worklist's rounds: how deep the failures ran from party to party.
    pub rounds: u64,
    pub values: Values,
}

/// A day's settlement measured: the value of the flows settled; what the payers' nets drew, the value settlement
/// needed; and the closing ring — the parties whose settled payments exceeded what they could pay alone, so settled
/// only by what they were paid in the same settlement — with the part of their payments those receipts paid.
#[clause("SET.10")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Values {
    pub gross: i128,
    pub net: i128,
    pub ring: u64,
    pub ring_value: i128,
}

impl std::ops::AddAssign for Values {
    fn add_assign(&mut self, o: Values) {
        self.gross += o.gross;
        self.net += o.net;
        self.ring += o.ring;
        self.ring_value += o.ring_value;
    }
}

/// A range's flows by payer, in the order made: each flow's place among the grouped payers, and where each slot's
/// run begins. Built only for a range a short party falls in, once a day.
#[derive(Debug, Default)]
struct ByPayer {
    built: bool,
    /// Each flow's place within the range, by payer.
    flows: Vec<u32>,
    starts: Vec<u32>,
}

/// The settlement's working state, kept across days so a day allocates nothing once the heaviest has sized it.
#[derive(Debug, Default)]
pub struct Settle {
    /// Each kind's parties' credits less debits of the flows standing, a value a slot.
    net: Vec<Vec<i64>>,
    /// Each bank's customers' flows standing, paid into it less paid out of it: what its reserves move by.
    cust: Vec<i64>,
    /// The flows removed, held or failed, a bit a flow of the grouped payers.
    removed: Vec<u64>,
    /// Each kind's parties' settled payments, a value a slot, read for the closing ring.
    debits: Vec<Vec<i64>>,
    by_payer: Vec<ByPayer>,
}

impl<'a> Books<'a> {
    fn book(&self, kind: u8) -> &Book<'a> {
        let Some(Some(b)) = self.kinds.get(usize::from(kind)) else {
            violation!(clause = "Law 5", "a flow names a party of a kind that holds no money", kind = kind);
        };
        b
    }

    fn book_mut(&mut self, kind: u8) -> &mut Book<'a> {
        let Some(Some(b)) = self.kinds.get_mut(usize::from(kind)) else {
            violation!(clause = "Law 5", "a flow names a party of a kind that holds no money", kind = kind);
        };
        b
    }

    /// The bank a customer's payments pass through; none for a bank's own, the issuer's, or an account at the issuer.
    fn bank_of(&self, p: PartyKey) -> Option<u32> {
        if p == self.issuer || p.kind() == self.banks {
            return None;
        }
        let Some(b) = self.book(p.kind()).bank.get(slot(p)) else {
            violation!(clause = "Law 5", "a flow names an account its kind does not hold", slot = p.slot().get());
        };
        (*b != AT_ISSUER).then_some(*b)
    }

    /// The bank a flow's side passes through, a bank's own included: a closed one holds the flow.
    fn through(&self, p: PartyKey) -> Option<u32> {
        if p.kind() == self.banks { Some(p.slot().get()) } else { self.bank_of(p) }
    }

    fn is_closed(&self, bank: Option<u32>) -> bool {
        bank.is_some_and(|b| self.closed.get(to_usize(b)).copied().unwrap_or(false))
    }
}

impl Books<'_> {
    /// What each bank's customers hold with it, their pending included: what the bank owes them.
    #[must_use]
    pub fn owed(&self) -> Vec<i64> {
        let mut owed = vec![0_i64; self.deposits.len()];
        for (k, book) in self.kinds.iter().enumerate() {
            let Some(b) = book.as_ref().filter(|_| u8::try_from(k).ok() != Some(self.banks)) else { continue };
            for ((at, balance), pending) in b.bank.iter().zip(b.balance.iter()).zip(b.pending.iter()) {
                if let Some(o) = owed.get_mut(to_usize(*at)) {
                    *o += balance + pending;
                }
            }
        }
        owed
    }

    /// The banks whose deposits are other than their customers hold, with both: the money family's identity, which the
    /// audit reads and never repairs.
    #[clause("N1")]
    #[must_use]
    pub fn deposit_breaks(&self) -> Vec<(u32, i64, i64)> {
        self.owed()
            .into_iter()
            .zip(self.deposits.iter())
            .enumerate()
            .filter(|(_, (o, d))| o != *d)
            .map(|(b, (o, d))| (u32::try_from(b).unwrap_or(u32::MAX), o, *d))
            .collect()
    }
}

fn slot(p: PartyKey) -> usize {
    to_usize(p.slot().get())
}

fn to_usize(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

fn cell(v: &mut [i64], i: usize) -> &mut i64 {
    let Some(c) = v.get_mut(i) else {
        violation!(clause = "Law 5", "a flow names a party its kind's accounts do not hold", slot = i);
    };
    c
}

/// A range's pass: its commitments added, its flows netted into its parties' nets, the reserve moves of its customers'
/// flows by bank, and its parties the nets leave short, but for banks, which are read once every range is done.
struct RangeJob<'a> {
    range: usize,
    net: &'a mut [i64],
    lines: Option<LineChunk<'a>>,
    bank: &'a [u32],
    balance: &'a mut [i64],
    pending: &'a mut [i64],
    held: &'a [i64],
    facility: &'a [i64],
}

impl Settle {
    /// Settles the day's flows of one currency, grouped by range, against the books: commitments first, then flows
    /// through a closed bank held, then every short party's flows failed from its first unaffordable one in payment
    /// order — ties among one payer's flows of one order by the lot `lot` draws for it — and a bank still short once
    /// its customers are done losing every flow through it, until nothing changes. What is left is the greatest set
    /// of flows that can settle given one another, rings included, and it is added to the accounts.
    #[clause("SET.4", "SET.5", "SET.6", "MON.3", "MON.5", "MON.12", "MON.13", "TIME.6")]
    pub fn settle(
        &mut self,
        pool: Option<&Pool>,
        grouped: &Grouped<'_>,
        ranges: &Ranges,
        books: &mut Books<'_>,
        lot: &(impl Fn(PartyKey) -> Draws + Sync),
    ) -> Outcome {
        let mut out = Outcome::default();
        let shorts = self.net_all(pool, grouped, ranges, books, true);
        let end = grouped.end();
        self.removed.clear();
        self.removed.resize(end.div_ceil(BIT_WORD), 0);
        self.by_payer.iter_mut().for_each(|b| b.built = false);
        self.by_payer.resize_with(ranges.count(), ByPayer::default);
        let mut next: Vec<PartyKey> = Vec::new();
        if books.closed.iter().any(|c| *c) {
            let held = matching(pool, grouped, ranges, |f| {
                books.is_closed(books.through(f.payer)) || books.is_closed(books.through(f.payee))
            });
            for idx in held {
                if self.remove(books, grouped, idx, &mut next)
                    && let Some(f) = grouped.flow(idx)
                {
                    out.held.push(*f);
                }
            }
        }
        let mut current = shorts;
        current.extend(self.short_banks(books));
        current.extend(next.drain(..).filter(|p| self.is_short(books, *p)));
        current.sort_unstable();
        current.dedup();
        let mut removed_banks: Vec<u32> = Vec::new();
        loop {
            let mut still_short: Vec<u32> = Vec::new();
            while !current.is_empty() {
                out.rounds += 1;
                self.index(pool, grouped, ranges, &current);
                // Every short party of the round decides from the round's state, range by range on the pool; its
                // failures are then made in order. A failure only takes from others, so deciding on the round's state
                // fails no flow the greatest set keeps, and what one round misses the next finds.
                let mut groups: Vec<Vec<PartyKey>> = Vec::new();
                for p in std::mem::take(&mut current) {
                    match groups.last_mut() {
                        Some(g) if g.first().is_some_and(|q| ranges.of(*q) == ranges.of(p)) => g.push(p),
                        _ => groups.push(vec![p]),
                    }
                }
                out.visits += groups.iter().map(|g| u64::try_from(g.len()).unwrap_or(u64::MAX)).sum::<u64>();
                let this: &Settle = self;
                let decided = phx_exec::pool::map(pool, groups.len(), |i| {
                    groups.get(i).map_or_else(Vec::new, |g| {
                        g.iter().flat_map(|p| this.decide(*p, books, grouped, ranges, lot)).collect::<Vec<_>>()
                    })
                });
                let mut fails = Vec::new();
                for (idx, f) in decided.into_iter().flatten() {
                    if self.mark(idx) {
                        fails.push(f);
                        out.failed.push((f, Cause::Payer));
                    }
                }
                next.extend(self.take_off(pool, books, ranges, &fails));
                still_short.extend(
                    groups
                        .iter()
                        .flatten()
                        .filter(|p| p.kind() == books.banks && self.is_short(books, **p))
                        .map(|p| p.slot().get()),
                );
                current.extend(next.drain(..).filter(|p| self.is_short(books, *p)));
                current.sort_unstable();
                current.dedup();
            }
            // A bank is answered once every payer is done, so what its customers could not pay is off its net first.
            still_short.sort_unstable();
            still_short.dedup();
            let banks: Vec<u32> = still_short
                .into_iter()
                .filter(|b| !removed_banks.contains(b))
                .filter(|b| self.is_short(books, PartyKey::new(books.banks, Slot::new(*b))))
                .collect();
            if banks.is_empty() {
                break;
            }
            removed_banks.extend(&banks);
            let through = |p: PartyKey| books.bank_of(p).is_some_and(|b| banks.contains(&b));
            for idx in matching(pool, grouped, ranges, |f| through(f.payer) || through(f.payee)) {
                if self.remove(books, grouped, idx, &mut next)
                    && let Some(f) = grouped.flow(idx)
                {
                    out.failed.push((*f, Cause::Bank));
                }
            }
            current.extend(next.drain(..).filter(|p| self.is_short(books, *p)));
            current.sort_unstable();
            current.dedup();
        }
        out.values = self.values(pool, grouped, ranges, books);
        self.apply(pool, ranges, books);
        unsettled(books, &out);
        let removed: u64 = self.removed.iter().map(|w| u64::from(w.count_ones())).sum();
        out.settled = u64::try_from(end).unwrap_or(u64::MAX) - removed;
        out
    }

    /// A closed day's flows, commitments until the next business day: their nets added to what each account has
    /// pending, and each bank's customers' moves to its own, with nothing failed.
    #[clause("SET.2", "TIME.8")]
    pub fn commit(&mut self, pool: Option<&Pool>, grouped: &Grouped<'_>, ranges: &Ranges, books: &mut Books<'_>) {
        let _ = self.net_all(pool, grouped, ranges, books, false);
        for (d, c) in books.deposits.iter_mut().zip(&self.cust) {
            *d += c;
        }
        let (net, cust, banks) = (&self.net, &self.cust, books.banks);
        for (k, book) in books.kinds.iter_mut().enumerate() {
            let (Some(b), Some(n)) = (book.as_mut(), net.get(k)) else { continue };
            for (i, (p, v)) in b.pending.iter_mut().zip(n).enumerate() {
                let moved = if u8::try_from(k).ok() == Some(banks) { cust.get(i).copied().unwrap_or(0) } else { 0 };
                *p += *v + moved;
            }
        }
    }

    /// Every range's pass on the pool; returns the parties short, banks aside, in range order.
    fn net_all(
        &mut self,
        pool: Option<&Pool>,
        grouped: &Grouped<'_>,
        ranges: &Ranges,
        books: &mut Books<'_>,
        settle: bool,
    ) -> Vec<PartyKey> {
        let width = 1_usize << ranges.range_bits();
        let n_banks = books.kinds.get(usize::from(books.banks)).and_then(Option::as_ref).map_or(0, |b| b.balance.len());
        self.net.resize_with(books.kinds.len(), Vec::new);
        let mut jobs: Vec<RangeJob<'_>> = Vec::new();
        for (k, (book, net)) in books.kinds.iter_mut().zip(self.net.iter_mut()).enumerate() {
            let Some(b) = book.as_mut() else { continue };
            net.resize(b.balance.len(), 0);
            let kind = u8::try_from(k).unwrap_or(u8::MAX);
            let first = ranges.of_kind(kind).start;
            let banks = b.bank.chunks(width).map(Some).chain(std::iter::repeat(None));
            let mut lines: Vec<LineChunk<'_>> = match b.lines.as_mut() {
                Some(l) => {
                    let (w, paid, received) = (l.width, l.paid, l.received);
                    l.amounts.chunks_mut(width * w).map(|a| (a, w, paid, received)).collect()
                }
                None => Vec::new(),
            };
            lines.reverse();
            let parts = net
                .chunks_mut(width)
                .zip(b.balance.chunks_mut(width))
                .zip(b.pending.chunks_mut(width))
                .zip(b.held.chunks(width))
                .zip(b.facility.chunks(width))
                .zip(banks);
            for (i, (((((net, balance), pending), held), facility), bank)) in parts.enumerate() {
                jobs.push(RangeJob {
                    range: first + i,
                    net,
                    lines: lines.pop(),
                    bank: bank.unwrap_or(&[]),
                    balance,
                    pending,
                    held,
                    facility,
                });
            }
        }
        let (issuer, banks) = (books.issuer, books.banks);
        let done: Vec<(Vec<i64>, Vec<PartyKey>)> = match pool {
            Some(p) => p.map_items(jobs, |j| range_pass(j, grouped, ranges, (issuer, banks, n_banks), settle)),
            None => {
                jobs.into_iter().map(|j| range_pass(j, grouped, ranges, (issuer, banks, n_banks), settle)).collect()
            }
        };
        self.cust.clear();
        self.cust.resize(n_banks, 0);
        let mut shorts = Vec::new();
        for (cust, short) in done {
            for (c, v) in self.cust.iter_mut().zip(cust) {
                *c += v;
            }
            shorts.extend(short);
        }
        shorts
    }

    fn standing(&self, books: &Books<'_>, p: PartyKey) -> i64 {
        let b = books.book(p.kind());
        let i = slot(p);
        let (Some(balance), Some(held), Some(facility)) = (b.balance.get(i), b.held.get(i), b.facility.get(i)) else {
            violation!(clause = "Law 5", "a party its kind's accounts do not hold", slot = i);
        };
        let Some(net) = self.net.get(usize::from(p.kind())).and_then(|n| n.get(i)).copied() else {
            violation!(clause = "Law 5", "a party the day's nets do not hold", slot = i);
        };
        // Only a bank's reserves move by its customers' flows; any other party's by nothing.
        let reserves = if p.kind() == books.banks {
            let Some(moves) = self.cust.get(i).copied() else {
                violation!(clause = "MON.5", "a bank the day's reserve moves do not hold", bank = i);
            };
            moves
        } else {
            0
        };
        balance - held + facility + net + reserves
    }

    fn is_short(&self, books: &Books<'_>, p: PartyKey) -> bool {
        p != books.issuer && self.standing(books, p) < 0
    }

    fn short_banks(&self, books: &Books<'_>) -> Vec<PartyKey> {
        (0..self.cust.len())
            .filter_map(|b| u32::try_from(b).ok())
            .map(|b| PartyKey::new(books.banks, Slot::new(b)))
            .filter(|p| self.is_short(books, *p))
            .collect()
    }

    /// Adds (+1) or takes away (−1) a flow's moves: the payer's and the payee's nets, and the reserves of the banks
    /// its customers' sides pass through.
    fn effect(&mut self, books: &Books<'_>, f: &Flow, sign: i64) {
        let a = sign * f.amount;
        for (p, v) in [(f.payer, -a), (f.payee, a)] {
            if p != books.issuer {
                let Some(net) = self.net.get_mut(usize::from(p.kind())) else { continue };
                *cell(net, slot(p)) += v;
            }
            if let Some(b) = books.bank_of(p) {
                *cell(&mut self.cust, to_usize(b)) += v;
            }
        }
    }

    /// Marks a flow removed; false if it already was.
    fn mark(&mut self, idx: usize) -> bool {
        let (word, bit) = (idx / BIT_WORD, 1_u64 << (idx % BIT_WORD));
        let Some(w) = self.removed.get_mut(word) else { return false };
        if *w & bit != 0 {
            return false;
        }
        *w |= bit;
        true
    }

    /// Takes a round's failed flows off their payers' and payees' nets, range by range on the pool, and each
    /// customer's bank's moves with them; returns the parties the failures leave short, banks included.
    fn take_off(&mut self, pool: Option<&Pool>, books: &Books<'_>, ranges: &Ranges, fails: &[Flow]) -> Vec<PartyKey> {
        let n = ranges.count();
        let mut sides: Vec<Vec<(PartyKey, i64)>> = (0..n).map(|_| Vec::new()).collect();
        for f in fails {
            for (p, v) in [(f.payer, f.amount), (f.payee, -f.amount)] {
                if p != books.issuer
                    && let Some(side) = sides.get_mut(ranges.of(p))
                {
                    side.push((p, v));
                }
            }
        }
        let width = 1_usize << ranges.range_bits();
        let mut chunks: Vec<Option<&mut [i64]>> = (0..n).map(|_| None).collect();
        for (k, net) in self.net.iter_mut().enumerate() {
            let first = ranges.of_kind(u8::try_from(k).unwrap_or(u8::MAX)).start;
            for (i, c) in net.chunks_mut(width).enumerate() {
                if let Some(slot) = chunks.get_mut(first + i) {
                    *slot = Some(c);
                }
            }
        }
        let jobs: Vec<TakeOff<'_>> = chunks
            .into_iter()
            .zip(sides)
            .enumerate()
            .filter(|(_, (_, side))| !side.is_empty())
            .filter_map(|(r, (c, side))| c.map(|c| (r, c, side)))
            .collect();
        let n_banks = self.cust.len();
        let done = map_items(pool, jobs, |(r, net, side)| {
            let (_, first) = ranges.start(r);
            let mut cust = vec![0_i64; n_banks];
            let mut short = Vec::new();
            for (p, v) in side {
                let at = to_usize(p.slot().get() - first);
                *cell(net, at) += v;
                if let Some(b) = books.bank_of(p) {
                    *cell(&mut cust, to_usize(b)) += v;
                    if v < 0 {
                        short.push(PartyKey::new(books.banks, Slot::new(b)));
                    }
                }
                if v < 0 && p.kind() != books.banks && funds(books, p) + net.get(at).copied().unwrap_or(0) < 0 {
                    short.push(p);
                }
            }
            (cust, short)
        });
        let mut short = Vec::new();
        for (cust, s) in done {
            for (c, v) in self.cust.iter_mut().zip(cust) {
                *c += v;
            }
            short.extend(s);
        }
        // A bank a customer was paid less through is only a candidate; the round's caller keeps those still short.
        short
    }

    /// Removes a flow once; its payee, and the bank the payee is paid through, may be short for it.
    fn remove(&mut self, books: &Books<'_>, grouped: &Grouped<'_>, idx: usize, next: &mut Vec<PartyKey>) -> bool {
        if !self.mark(idx) {
            return false;
        }
        let Some(f) = grouped.flow(idx).copied() else { return false };
        self.effect(books, &f, -1);
        next.push(f.payee);
        if let Some(b) = books.bank_of(f.payee) {
            next.push(PartyKey::new(books.banks, Slot::new(b)));
        }
        true
    }

    fn is_removed(&self, idx: usize) -> bool {
        self.removed.get(idx / BIT_WORD).is_some_and(|w| w & (1 << (idx % BIT_WORD)) != 0)
    }

    /// The flows a short party fails: it pays its flows in payment order while its funds last, and the first it
    /// cannot pay fails with every one after it, so what fails is a suffix of its order and more funds never settle
    /// less.
    fn decide(
        &self,
        p: PartyKey,
        books: &Books<'_>,
        grouped: &Grouped<'_>,
        ranges: &Ranges,
        lot: &(impl Fn(PartyKey) -> Draws + Sync),
    ) -> Vec<(usize, Flow)> {
        if !self.is_short(books, p) {
            return Vec::new();
        }
        let standing: Vec<(usize, Flow)> =
            self.own(grouped, ranges, p, lot).into_iter().filter(|(i, _)| !self.is_removed(*i)).collect();
        let mut left = self.standing(books, p) + standing.iter().map(|(_, f)| f.amount).sum::<i64>();
        let mut failing = false;
        let mut fails = Vec::new();
        for (i, f) in standing {
            failing = failing || (f.amount > 0 && left < f.amount);
            if failing {
                fails.push((i, f));
            } else {
                left -= f.amount;
            }
        }
        fails
    }

    /// A payer's flows in payment order, each with its place: by order, then by the lot drawn for the payer, one draw
    /// a flow in the order the flows were made, so the same whenever it is read.
    fn own(
        &self,
        grouped: &Grouped<'_>,
        ranges: &Ranges,
        p: PartyKey,
        lot: &(impl Fn(PartyKey) -> Draws + Sync),
    ) -> Vec<(usize, Flow)> {
        let r = ranges.of(p);
        let (_, first) = ranges.start(r);
        let Some(by) = self.by_payer.get(r).filter(|b| b.built) else {
            violation!(clause = "TIME.6", "a short payer's range read before its flows were indexed", range = r);
        };
        let at = to_usize(p.slot().get() - first);
        let (Some(from), Some(to)) = (by.starts.get(at), by.starts.get(at + 1)) else { return Vec::new() };
        let start = grouped.first(r);
        let mut draws = lot(p);
        let mut keyed: Vec<(u8, u64, usize, Flow)> = by
            .flows
            .get(to_usize(*from)..to_usize(*to))
            .unwrap_or(&[])
            .iter()
            .map(|i| start + to_usize(*i))
            .filter_map(|i| grouped.flow(i).map(|f| (f.order, draws.next_u64(), i, *f)))
            .collect();
        keyed.sort_unstable_by_key(|(o, l, i, _)| (*o, *l, *i));
        keyed.into_iter().map(|(_, _, i, f)| (i, f)).collect()
    }

    /// Indexes by payer every range the round's short parties fall in and no earlier round indexed, on the pool.
    fn index(&mut self, pool: Option<&Pool>, grouped: &Grouped<'_>, ranges: &Ranges, want: &[PartyKey]) {
        let mut todo: Vec<usize> = want.iter().map(|p| ranges.of(*p)).collect();
        todo.dedup();
        let width = 1_usize << ranges.range_bits();
        let mut jobs: Vec<(usize, &mut ByPayer)> =
            self.by_payer.iter_mut().enumerate().filter(|(r, b)| !b.built && todo.binary_search(r).is_ok()).collect();
        jobs.sort_unstable_by_key(|(r, _)| *r);
        phx_exec::pool::each(pool, jobs, |(r, by)| {
            let (_, first) = ranges.start(r);
            let start = grouped.first(r);
            let count = grouped.first(r + 1) - start;
            if u32::try_from(count).is_err() {
                phx_num::capacity_exceeded!("a range's flows of a day", u32::MAX, count);
            }
            let at = |f: &Flow| to_usize(f.payer.slot().get() - first);
            // A counting sort over the range's slots: each slot's count, its run's start, then each flow placed.
            by.starts.clear();
            by.starts.resize(width + 1, 0);
            for f in grouped.payers(r) {
                let Some(c) = by.starts.get_mut(at(f) + 1) else {
                    violation!(clause = "SET.4", "a flow grouped in a range its payer is not in", range = r);
                };
                *c += 1;
            }
            let mut sum = 0;
            for c in &mut by.starts {
                sum += *c;
                *c = sum;
            }
            let mut next: Vec<u32> = by.starts.clone();
            by.flows.clear();
            by.flows.resize(count, 0);
            for (i, f) in (0_u32..).zip(grouped.payers(r)) {
                if let Some(n) = next.get_mut(at(f))
                    && let Some(slot) = by.flows.get_mut(to_usize(*n))
                {
                    *slot = i;
                    *n += 1;
                }
            }
            by.built = true;
        });
    }

    /// The day's settlement measured once the greatest set is found and before its nets are added, so each account
    /// still holds what it held before: range by range on the pool, each settled flow's value and its payer's
    /// payments, then each payer's own means against them.
    #[clause("SET.10")]
    fn values(&mut self, pool: Option<&Pool>, grouped: &Grouped<'_>, ranges: &Ranges, books: &Books<'_>) -> Values {
        let width = 1_usize << ranges.range_bits();
        self.debits.resize_with(books.kinds.len(), Vec::new);
        let mut jobs: Vec<(usize, &mut [i64], &[i64])> = Vec::new();
        for (k, (book, debits)) in books.kinds.iter().zip(self.debits.iter_mut()).enumerate() {
            let Some(b) = book.as_ref() else { continue };
            debits.clear();
            debits.resize(b.balance.len(), 0);
            let first = ranges.of_kind(u8::try_from(k).unwrap_or(u8::MAX)).start;
            let nets = self.net.get(k).map_or(&[][..], Vec::as_slice);
            let nets = nets.chunks(width).chain(std::iter::repeat(&[][..]));
            for (i, (d, n)) in debits.chunks_mut(width).zip(nets).enumerate() {
                jobs.push((first + i, d, n));
            }
        }
        let removed = &self.removed;
        let parts = map_items(pool, jobs, |(range, debits, nets)| {
            let (kind, first) = ranges.start(range);
            let mut v = Values::default();
            let base = grouped.first(range);
            for (i, f) in grouped.payers(range).enumerate() {
                let idx = base + i;
                if removed.get(idx / BIT_WORD).is_some_and(|w| w & (1 << (idx % BIT_WORD)) != 0) {
                    continue;
                }
                v.gross += i128::from(f.amount);
                if f.payer != books.issuer {
                    *cell(debits, to_usize(f.payer.slot().get() - first)) += f.amount;
                }
            }
            let book = books.book(kind);
            let start = to_usize(first);
            for (i, d) in debits.iter().enumerate().filter(|(_, d)| **d > 0) {
                let at = start + i;
                let (Some(balance), Some(held), Some(facility)) =
                    (book.balance.get(at), book.held.get(at), book.facility.get(at))
                else {
                    violation!(clause = "Law 5", "a payer its kind's accounts do not hold", slot = at);
                };
                let alone = balance - held + facility;
                if *d > alone {
                    v.ring += 1;
                    v.ring_value += i128::from(if alone > 0 { *d - alone } else { *d });
                }
            }
            v.net += nets.iter().filter(|n| **n < 0).map(|n| -i128::from(*n)).sum::<i128>();
            v
        });
        let mut total = Values::default();
        for v in parts {
            total += v;
        }
        total
    }

    /// Adds each party's net to its account, and each bank's customers' moves to its reserves.
    fn apply(&self, pool: Option<&Pool>, ranges: &Ranges, books: &mut Books<'_>) {
        for (d, c) in books.deposits.iter_mut().zip(&self.cust) {
            *d += c;
        }
        let width = 1_usize << ranges.range_bits();
        let banks = usize::from(books.banks);
        let mut jobs: Vec<(&mut [i64], &[i64], &[i64])> = Vec::new();
        for (k, (book, net)) in books.kinds.iter_mut().zip(&self.net).enumerate() {
            let Some(b) = book.as_mut() else { continue };
            let moves = if k == banks { self.cust.as_slice() } else { &[] };
            let moves = moves.chunks(width).chain(std::iter::repeat(&[][..]));
            for ((balance, net), m) in b.balance.chunks_mut(width).zip(net.chunks(width)).zip(moves) {
                jobs.push((balance, net, m));
            }
        }
        phx_exec::pool::each(pool, jobs, |(balance, net, moves)| {
            for (i, (a, n)) in balance.iter_mut().zip(net).enumerate() {
                *a += n + moves.get(i).copied().unwrap_or(0);
            }
        });
    }
}

/// What the flows that did not settle leave: a held flow holds back its amount from the next day on, since today its
/// money never left the account; and every one is taken back from both sides' lines, posted as it was netted.
fn unsettled(books: &mut Books<'_>, out: &Outcome) {
    for f in &out.held {
        *cell(books.book_mut(f.payer.kind()).held, slot(f.payer)) += f.amount;
    }
    for f in out.failed.iter().map(|(f, _)| f).chain(&out.held) {
        for (p, paying) in [(f.payer, true), (f.payee, false)] {
            if p == books.issuer {
                continue;
            }
            let Some(l) = books.book_mut(p.kind()).lines.as_mut() else { continue };
            let words = if paying { l.paid } else { l.received };
            let line = words.get(usize::from(f.reason)).copied().flatten();
            Lines::post(l.amounts, l.width, slot(p), line, -f.amount);
        }
    }
}

/// `f` of every item, on the pool when one is given and on the calling thread otherwise, results in the items' order.
fn map_items<I: Send, T: Send>(pool: Option<&Pool>, items: Vec<I>, f: impl Fn(I) -> T + Sync) -> Vec<T> {
    match pool {
        Some(p) => p.map_items(items, f),
        None => items.into_iter().map(f).collect(),
    }
}

/// A range's part in taking a round's failures off: the range, its parties' nets, and each failed flow's side there.
type TakeOff<'a> = (usize, &'a mut [i64], Vec<(PartyKey, i64)>);

/// What an account may draw on before the day's flows: its balance less what it holds, and its facility.
fn funds(books: &Books<'_>, p: PartyKey) -> i64 {
    let b = books.book(p.kind());
    let i = slot(p);
    match (b.balance.get(i), b.held.get(i), b.facility.get(i)) {
        (Some(balance), Some(held), Some(facility)) => balance - held + facility,
        _ => violation!(clause = "Law 5", "a party its kind's accounts do not hold", slot = i),
    }
}

/// A range's lines: its parties' amounts, the lines a party, and the line each reason posts to on each side.
type LineChunk<'a> = (&'a mut [i64], usize, &'a [Option<usize>], &'a [Option<usize>]);

/// One range's pass: see `RangeJob`.
fn range_pass(
    j: RangeJob<'_>,
    grouped: &Grouped<'_>,
    ranges: &Ranges,
    (issuer, banks, n_banks): (PartyKey, u8, usize),
    settle: bool,
) -> (Vec<i64>, Vec<PartyKey>) {
    let RangeJob { range, net, mut lines, bank, balance, pending, held, facility } = j;
    let (kind, first) = ranges.start(range);
    net.fill(0);
    if settle {
        for (b, p) in balance.iter_mut().zip(pending.iter_mut()) {
            *b += *p;
            *p = 0;
        }
    }
    let mut cust = vec![0_i64; n_banks];
    let at = |p: PartyKey| to_usize(p.slot().get() - first);
    let moved = |p: PartyKey, v: i64, net: &mut [i64], cust: &mut [i64]| {
        if p == issuer {
            return;
        }
        *cell(net, at(p)) += v;
        if kind == banks {
            return;
        }
        let Some(b) = bank.get(at(p)) else {
            violation!(clause = "Law 5", "a flow names an account its kind does not hold", slot = p.slot().get());
        };
        if *b != AT_ISSUER {
            *cell(cust, to_usize(*b)) += v;
        }
    };
    for credits in grouped.credit_slices(range) {
        for c in credits {
            moved(c.payee, c.amount, net, &mut cust);
            if let Some((amounts, w, _, received)) = lines.as_mut().filter(|_| c.payee != issuer) {
                let line = received.get(usize::from(c.reason)).copied().flatten();
                Lines::post(amounts, *w, at(c.payee), line, c.amount);
            }
        }
    }
    for flows in grouped.payer_slices(range) {
        for f in flows {
            moved(f.payer, -f.amount, net, &mut cust);
            if let Some((amounts, w, paid, _)) = lines.as_mut().filter(|_| f.payer != issuer) {
                let line = paid.get(usize::from(f.reason)).copied().flatten();
                Lines::post(amounts, *w, at(f.payer), line, f.amount);
            }
        }
    }
    let mut short = Vec::new();
    if settle && kind != banks {
        for (i, (((b, h), f), n)) in balance.iter().zip(held).zip(facility).zip(net.iter()).enumerate() {
            if b - h + f + n < 0 {
                let s = first + u32::try_from(i).unwrap_or(u32::MAX);
                let p = PartyKey::new(kind, Slot::new(s));
                if p != issuer {
                    short.push(p);
                }
            }
        }
    }
    (cust, short)
}

/// Every flow `hit` picks, by its place among the grouped payers, in order: one pass over each range on the pool.
fn matching(
    pool: Option<&Pool>,
    grouped: &Grouped<'_>,
    ranges: &Ranges,
    hit: impl Fn(&Flow) -> bool + Sync,
) -> Vec<usize> {
    let found = phx_exec::pool::map(pool, ranges.count(), |r| {
        let start = grouped.first(r);
        grouped.payers(r).enumerate().filter(|(_, f)| hit(f)).map(|(i, _)| start + i).collect::<Vec<usize>>()
    });
    found.concat()
}

#[path = "settle_tests.rs"]
mod tests;
