//! Flows: every movement of money or of held units the day makes, each naming both its parties, appended by the
//! chunk that makes it and gathered in chunk order, so the day's flows are the same whatever the workers.

use phx_id::PartyKey;
use phx_macros::clause;
use phx_num::violation;

use crate::consts::UNITS_BIT;

/// What a flow moves: money in a currency, or units of a declared unit. The top bit tells them apart, so money and
/// units are never netted together.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Saved)]
pub struct Denom(u16);

/// The bit a denomination of units carries: the top one, leaving 2^15 currencies and as many units.
const UNITS: u16 = 1 << UNITS_BIT;

impl Denom {
    /// Money in the currency of index `ccy`.
    pub fn money(ccy: u8) -> Denom {
        Denom(u16::from(ccy))
    }

    /// Units of the declared unit `unit`, below 2^15.
    pub fn units(unit: u16) -> Denom {
        if unit >= UNITS {
            phx_num::capacity_exceeded!("declared units a flow names", UNITS, unit);
        }
        Denom(UNITS | unit)
    }

    #[must_use]
    pub const fn is_money(self) -> bool {
        self.0 & UNITS == 0
    }

    /// The declared unit a denomination of units names; money names none, and asking for one stops the run.
    #[must_use]
    pub fn unit(self) -> u16 {
        if self.is_money() {
            violation!(clause = "Law 5", "a unit read from money", denomination = self.0);
        }
        self.0 & !UNITS
    }

    /// The currency a denomination of money names; units name none, and asking for one stops the run.
    #[must_use]
    pub fn ccy(self) -> u8 {
        match u8::try_from(self.0) {
            Ok(c) if self.is_money() => c,
            _ => violation!(clause = "Law 5", "a currency read from units", denomination = self.0),
        }
    }
}

/// One movement: from the payer to the payee, an amount of a denomination, for a reason, at the payer's payment
/// order for the reason, from a source the reason names (a contract, a match, a transformation).
#[clause("Law 5", "SET.1", "SET.4", "MON.5")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Flow {
    pub payer: PartyKey,
    pub payee: PartyKey,
    pub amount: i64,
    pub source: u32,
    pub denomination: Denom,
    pub reason: u8,
    pub order: u8,
}

/// The day's flows of one sub-step, a buffer per chunk that made any, kept across days so a day appends without
/// allocating once the heaviest day has sized each buffer. Grouped, each chunk's flows are put in the order of their
/// payers' ranges, in place, and their credits in the order of their payees'.
#[derive(Debug, Default)]
pub struct FlowBufs {
    chunks: Vec<Vec<Flow>>,
    used: usize,
    groups: Vec<ChunkGroups>,
}

/// A chunk's flows as grouped: where each payer range's flows begin among them, the last range holding the flows of
/// other denominations; and its credits by payee range and where each range's begin.
#[derive(Debug, Default)]
struct ChunkGroups {
    starts: Vec<usize>,
    credits: Vec<Credit>,
    credit_starts: Vec<usize>,
}

impl FlowBufs {
    /// Empties every buffer, keeping its capacity, for `chunks` chunks to fill.
    pub fn reset(&mut self, chunks: usize) {
        if self.chunks.len() < chunks {
            self.chunks.resize_with(chunks, Vec::new);
        }
        for c in &mut self.chunks {
            c.clear();
        }
        self.used = chunks;
    }

    /// Each chunk's buffer, to hand one to each chunk's worker.
    pub fn chunks_mut(&mut self) -> &mut [Vec<Flow>] {
        let used = self.used;
        self.chunks.get_mut(..used).unwrap_or_default()
    }

    /// The buffers in chunk order.
    pub fn slices(&self) -> impl Iterator<Item = &[Flow]> {
        self.chunks.iter().take(self.used).map(Vec::as_slice)
    }

    /// Flows held, across the buffers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.slices().map(<[Flow]>::len).sum()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Groups each chunk's flows of `denom` by their payers' ranges, in place, and makes their credits by their payees'
    /// ranges; a flow of another denomination goes to a last range of its own, which no range reads. Chunks are grouped
    /// on the pool, each by itself, so the result is the same whatever the workers; within a range a chunk's flows are
    /// in an order its own made, since a payer's ties are drawn by lot.
    #[clause("SET.4")]
    pub fn group(&mut self, pool: Option<&phx_exec::Pool>, ranges: &Ranges, denom: Denom) {
        let used = self.used;
        self.groups.resize_with(used, ChunkGroups::default);
        let n = ranges.count();
        let jobs: Vec<(&mut Vec<Flow>, &mut ChunkGroups)> =
            self.chunks.iter_mut().take(used).zip(self.groups.iter_mut()).collect();
        phx_exec::pool::each(pool, jobs, |(flows, g)| {
            let paying = |f: &Flow| if f.denomination == denom { ranges.of(f.payer) } else { n };
            counting_places(flows.iter().map(paying), n + 1, &mut g.starts);
            // In place, each flow swapped into its range's next free place until every place holds its range's.
            let mut next = g.starts.clone();
            for b in 0..=n {
                let end = g.starts.get(b + 1).copied().unwrap_or(0);
                while let Some(at) = next.get(b).copied().filter(|at| *at < end) {
                    let Some(to) = flows.get(at).map(&paying) else { break };
                    if to == b {
                        if let Some(x) = next.get_mut(b) {
                            *x += 1;
                        }
                    } else if let Some(there) = next.get_mut(to) {
                        flows.swap(at, *there);
                        *there += 1;
                    }
                }
            }
            // Credits only of the denomination's flows, which lead the chunk: no range reads another's.
            let settled = g.starts.get(n).copied().unwrap_or(0);
            let ours = flows.get(..settled).unwrap_or(&[]);
            let paid = |f: &Flow| ranges.of(f.payee);
            counting_places(ours.iter().map(paid), n, &mut g.credit_starts);
            g.credits.clear();
            g.credits.resize(ours.len(), Credit { payee: PartyKey::from_word(0), amount: 0, reason: 0 });
            let mut next = g.credit_starts.clone();
            for f in ours {
                if let Some(at) = next.get_mut(paid(f))
                    && let Some(cell) = g.credits.get_mut(*at)
                {
                    *cell = Credit { payee: f.payee, amount: f.amount, reason: f.reason };
                    *at += 1;
                }
            }
        });
    }

    /// A grouped chunk's flows of a payer range.
    fn payers(&self, chunk: usize, range: usize) -> &[Flow] {
        let (Some(flows), Some(g)) = (self.chunks.get(chunk), self.groups.get(chunk)) else { return &[] };
        match (g.starts.get(range), g.starts.get(range + 1)) {
            (Some(a), Some(b)) => flows.get(*a..*b).unwrap_or(&[]),
            _ => &[],
        }
    }

    /// A grouped chunk's credits of a payee range.
    fn credits(&self, chunk: usize, range: usize) -> &[Credit] {
        let Some(g) = self.groups.get(chunk) else { return &[] };
        match (g.credit_starts.get(range), g.credit_starts.get(range + 1)) {
            (Some(a), Some(b)) => g.credits.get(*a..*b).unwrap_or(&[]),
            _ => &[],
        }
    }
}

/// Where each of `buckets` begins when items fall in the buckets `keys` gives, with the end after the last.
fn counting_places(keys: impl Iterator<Item = usize>, buckets: usize, starts: &mut Vec<usize>) {
    starts.clear();
    starts.resize(buckets + 1, 0);
    for k in keys {
        let Some(c) = starts.get_mut(k + 1) else {
            violation!(clause = "SET.4", "a flow in a range beyond the day's", range = k);
        };
        *c += 1;
    }
    let mut sum = 0;
    for c in starts.iter_mut() {
        sum += *c;
        *c = sum;
    }
}

/// Where each kind's parties fall among the netting ranges: a range is `1 << range_bits` slots of one kind, and a
/// kind's ranges follow the kinds before it.
#[derive(Debug, Clone)]
pub struct Ranges {
    range_bits: u32,
    first: Vec<usize>,
    total: usize,
}

impl Ranges {
    /// Ranges of `1 << range_bits` slots over kinds whose tables hold `high[k]` slots.
    #[must_use]
    pub fn new(range_bits: u32, high: &[u32]) -> Ranges {
        let mut first = Vec::with_capacity(high.len());
        let mut total = 0_usize;
        for h in high {
            first.push(total);
            total += usize::try_from(h.div_ceil(1 << range_bits)).unwrap_or(0);
        }
        Ranges { range_bits, first, total }
    }

    /// The range a party falls in.
    #[must_use]
    pub fn of(&self, party: PartyKey) -> usize {
        let Some(base) = self.first.get(usize::from(party.kind())) else {
            phx_num::violation!(
                clause = "SET.4",
                "a flow names a kind the netting has no table of",
                kind = party.kind()
            );
        };
        base + usize::try_from(party.slot().get() >> self.range_bits).unwrap_or(0)
    }

    #[must_use]
    pub fn count(&self) -> usize {
        self.total
    }

    /// The ranges a kind's slots fall in, in slot order.
    #[must_use]
    pub fn of_kind(&self, kind: u8) -> std::ops::Range<usize> {
        let k = usize::from(kind);
        let from = self.first.get(k).copied().unwrap_or(self.total);
        let to = self.first.get(k + 1).copied().unwrap_or(self.total);
        from..to
    }

    /// The kind and first slot of a range.
    #[must_use]
    pub fn start(&self, range: usize) -> (u8, u32) {
        let kind = self.first.iter().rposition(|f| *f <= range).unwrap_or(0);
        let within = range - self.first.get(kind).copied().unwrap_or(0);
        (u8::try_from(kind).unwrap_or(u8::MAX), u32::try_from(within).unwrap_or(0) << self.range_bits)
    }

    #[must_use]
    pub fn range_bits(&self) -> u32 {
        self.range_bits
    }
}

/// What a payee's range needs of a flow: whom it credits, by how much, and for what reason, which says where the
/// credit is posted in the payee's accounts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credit {
    pub payee: PartyKey,
    pub amount: i64,
    pub reason: u8,
}

/// A range's piece of one chunk's grouped flows: its buffer, its chunk, and where its first flow falls in the
/// range's order.
#[derive(Clone, Copy, Debug)]
struct Piece {
    buf: usize,
    chunk: usize,
    at: usize,
}

/// One denomination's flows of the day as grouped in their buffers, read by range: each range's flows, whole, in the
/// order of the buffers and their chunks, since a payer's failures follow its flows' order; and its credits. Every
/// flow has a place in the day's order, range after range, which the settlement marks it by. Nothing is copied.
#[clause("SET.4", "SET.6")]
#[derive(Debug)]
pub struct Grouped<'a> {
    bufs: Vec<&'a FlowBufs>,
    /// Each range's pieces of flows and of credits, in order.
    payers: Vec<Vec<Piece>>,
    credits: Vec<Vec<(usize, usize)>>,
    /// Where each range's flows begin in the day's order, and the day's end after the last.
    first: Vec<usize>,
}

impl<'a> Grouped<'a> {
    /// The day's grouped buffers read by range; each buffer must have been grouped over the same ranges.
    #[must_use]
    pub fn new(bufs: &[&'a FlowBufs], ranges: &Ranges) -> Grouped<'a> {
        let n = ranges.count();
        let mut payers: Vec<Vec<Piece>> = (0..n).map(|_| Vec::new()).collect();
        let mut credits: Vec<Vec<(usize, usize)>> = (0..n).map(|_| Vec::new()).collect();
        let mut first = Vec::with_capacity(n + 1);
        let mut total = 0;
        for (r, (pieces, cpieces)) in payers.iter_mut().zip(credits.iter_mut()).enumerate() {
            first.push(total);
            let mut at = 0;
            for (b, buf) in bufs.iter().enumerate() {
                for chunk in 0..buf.used {
                    let len = buf.payers(chunk, r).len();
                    if len > 0 {
                        pieces.push(Piece { buf: b, chunk, at });
                        at += len;
                    }
                    if !buf.credits(chunk, r).is_empty() {
                        cpieces.push((b, chunk));
                    }
                }
            }
            total += at;
        }
        first.push(total);
        Grouped { bufs: bufs.to_vec(), payers, credits, first }
    }

    /// A range's flows, in order, a slice a chunk that made any.
    pub fn payer_slices(&self, range: usize) -> impl Iterator<Item = &'a [Flow]> + '_ {
        self.payers
            .get(range)
            .into_iter()
            .flatten()
            .map(move |p| self.bufs.get(p.buf).map_or(&[][..], |b| b.payers(p.chunk, range)))
    }

    /// A range's flows, in order.
    pub fn payers(&self, range: usize) -> impl Iterator<Item = &'a Flow> + '_ {
        self.payer_slices(range).flatten()
    }

    /// A range's credits, a slice a chunk that made any.
    pub fn credit_slices(&self, range: usize) -> impl Iterator<Item = &'a [Credit]> + '_ {
        self.credits
            .get(range)
            .into_iter()
            .flatten()
            .map(move |(b, c)| self.bufs.get(*b).map_or(&[][..], |buf| buf.credits(*c, range)))
    }

    /// A range's credits.
    pub fn credits(&self, range: usize) -> impl Iterator<Item = &'a Credit> + '_ {
        self.credit_slices(range).flatten()
    }

    /// Where a range's flows begin in the day's order.
    #[must_use]
    pub fn first(&self, range: usize) -> usize {
        self.first.get(range).copied().unwrap_or(0)
    }

    /// The day's flows of the denomination.
    #[must_use]
    pub fn end(&self) -> usize {
        self.first.last().copied().unwrap_or(0)
    }

    /// The flow at a place in the day's order.
    #[must_use]
    pub fn flow(&self, place: usize) -> Option<&'a Flow> {
        let r = self.first.partition_point(|f| *f <= place).checked_sub(1)?;
        let within = place - self.first(r);
        let pieces = self.payers.get(r)?;
        let p = pieces.get(pieces.partition_point(|p| p.at <= within).checked_sub(1)?)?;
        self.bufs.get(p.buf)?.payers(p.chunk, r).get(within - p.at)
    }

    /// A range's net per party, credits less debits, into `out`, one entry a slot of the range.
    pub fn net(&self, range: usize, ranges: &Ranges, out: &mut [i64]) {
        out.fill(0);
        let (_, first) = ranges.start(range);
        let mut add = |p: PartyKey, v: i64| {
            let at = usize::try_from(p.slot().get() - first).unwrap_or(usize::MAX);
            let Some(m) = out.get_mut(at) else {
                violation!(clause = "Law 5", "a flow names a party its range does not hold", slot = p.slot().get());
            };
            *m += v;
        };
        for c in self.credits(range) {
            add(c.payee, c.amount);
        }
        for f in self.payers(range) {
            add(f.payer, -f.amount);
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyKey, Slot};

    use super::{Denom, Flow, FlowBufs, Grouped, Ranges};

    fn flow(from: (u8, u32), to: (u8, u32), amount: i64) -> Flow {
        let key = |(k, s): (u8, u32)| PartyKey::new(k, Slot::new(s));
        Flow { payer: key(from), payee: key(to), amount, source: 0, denomination: Denom::money(0), reason: 0, order: 0 }
    }

    #[test]
    fn nets_are_credits_less_debits_per_party() {
        let ranges = Ranges::new(2, &[10, 3]);
        assert_eq!(ranges.count(), 4, "three ranges of four slots for ten, one for three");
        assert_eq!(ranges.start(3), (1, 0));
        let mut bufs = FlowBufs::default();
        bufs.reset(2);
        bufs.chunks_mut()[0].extend([flow((0, 1), (1, 2), 50), flow((0, 9), (0, 1), 7)]);
        bufs.chunks_mut()[1].push(flow((1, 2), (0, 9), 20));
        bufs.group(None, &ranges, Denom::money(0));
        let g = Grouped::new(&[&bufs], &ranges);
        assert_eq!(g.end(), 3);
        assert_eq!(g.flow(2).map(|f| f.amount), Some(20), "the third range's flow is the day's last");
        let mut out = [0_i64; 4];
        g.net(0, &ranges, &mut out);
        assert_eq!(out, [0, -43, 0, 0]);
        g.net(2, &ranges, &mut out);
        assert_eq!(out, [0, 13, 0, 0], "slot 9 is the second of the third range");
        g.net(3, &ranges, &mut out);
        assert_eq!(out, [0, 0, 30, 0]);
    }

    #[test]
    fn nets_are_the_same_for_any_workers_and_units_apart() {
        let ranges = Ranges::new(3, &[40, 20]);
        let made = || {
            let mut bufs = FlowBufs::default();
            bufs.reset(7);
            let mut i = 0_u32;
            for c in bufs.chunks_mut() {
                for _ in 0..500 {
                    i += 1;
                    let from = (u8::from(i.is_multiple_of(3)), (i * 7) % 20);
                    let to = (u8::from(i.is_multiple_of(5)), (i * 13) % 20);
                    let mut f = flow(from, to, i64::from(i % 97));
                    if i.is_multiple_of(11) {
                        f.denomination = Denom::units(3);
                    }
                    c.push(f);
                }
            }
            bufs
        };
        let nets = |workers: Option<usize>| {
            let pool = workers.map(|w| phx_exec::Pool::new(&phx_exec::PoolSpec::unpinned(w)).unwrap());
            let mut bufs = made();
            bufs.group(pool.as_ref(), &ranges, Denom::money(0));
            let g = Grouped::new(&[&bufs], &ranges);
            (0..ranges.count())
                .map(|r| {
                    let mut out = [0_i64; 8];
                    g.net(r, &ranges, &mut out);
                    out
                })
                .collect::<Vec<_>>()
        };
        let one = nets(None);
        assert_eq!(one, nets(Some(1)));
        assert_eq!(one, nets(Some(4)), "the same nets whatever the workers");
        let total: i64 = one.iter().flatten().sum();
        assert_eq!(total, 0, "money's nets sum to nothing once units are kept apart");
    }
}
