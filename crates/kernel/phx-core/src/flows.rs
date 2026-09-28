//! Flows: every movement of money or of held units the day makes, each naming both its parties, appended by the
//! chunk that makes it and gathered in chunk order, so the day's flows are the same whatever the workers.

use phx_id::PartyKey;
use phx_macros::clause;
use phx_num::violation;

use crate::consts::UNITS_BIT;

/// What a flow moves: money in a currency, or units of a declared unit. The top bit tells them apart, so money and
/// units are never netted together.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

/// One movement: from the payer to the payee, an amount of a denomination, for a reason, at the payer's payment
/// order for the reason, from a source the reason names (a contract, a match, a transformation).
#[clause("Law 5", "SET.4", "MON.5")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
/// allocating once the heaviest day has sized each buffer.
#[derive(Debug, Default)]
pub struct FlowBufs {
    chunks: Vec<Vec<Flow>>,
    used: usize,
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

/// What a payee's range needs of a flow: whom it credits and by how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Credit {
    pub payee: PartyKey,
    pub amount: i64,
}

/// One denomination's flows of the day grouped by the range of their payer, whole, since a payer's failures follow
/// its flows' order, and by the range of their payee as credits; kept across days, so each range's parties are netted
/// by one worker reading only its own.
#[clause("SET.4", "SET.6")]
#[derive(Debug, Default)]
pub struct Grouped {
    pub by_payer: phx_exec::partition::Partitioned<Flow>,
    pub by_payee: phx_exec::partition::Partitioned<Credit>,
}

impl Grouped {
    /// Groups every flow of `denom` in the buffers given, in their order, by payer range and by payee range; flows of
    /// any other denomination fall in a last bucket of their own, which no range reads.
    pub fn group(&mut self, pool: Option<&phx_exec::Pool>, bufs: &[&FlowBufs], ranges: &Ranges, denom: Denom) {
        let slices: Vec<&[Flow]> = bufs.iter().flat_map(|b| b.slices()).collect();
        let n = ranges.count();
        let at = |f: &Flow, p: PartyKey| if f.denomination == denom { ranges.of(p) } else { n };
        phx_exec::partition::partition_into(pool, &slices, n + 1, |f| at(f, f.payer), &mut self.by_payer);
        phx_exec::partition::partition_map_into(
            pool,
            &slices,
            n + 1,
            |f| at(f, f.payee),
            |f| Credit { payee: f.payee, amount: f.amount },
            &mut self.by_payee,
        );
    }

    /// A range's net per party, credits less debits, added to `money`, one entry a slot of the range; returns the
    /// parties of the range left below nothing. A flow naming a party outside the range's slots stops the run: its
    /// other side would move alone.
    pub fn apply(&self, range: usize, ranges: &Ranges, money: &mut [i64]) -> u64 {
        let (_, first) = ranges.start(range);
        let at = |p: PartyKey| usize::try_from(p.slot().get() - first).unwrap_or(usize::MAX);
        let add = |p: PartyKey, v: i64, money: &mut [i64]| {
            let Some(m) = money.get_mut(at(p)) else {
                violation!(
                    clause = "Law 5",
                    "a flow names a party its range's money does not hold",
                    slot = p.slot().get()
                );
            };
            *m += v;
        };
        for c in self.by_payee.bucket(range) {
            add(c.payee, c.amount, money);
        }
        for f in self.by_payer.bucket(range) {
            add(f.payer, -f.amount, money);
        }
        // One pass over the range's money, which a worker holds in cache, counts each party once.
        u64::try_from(money.iter().filter(|m| **m < 0).count()).unwrap_or(u64::MAX)
    }

    /// A range's net per party, credits less debits, into `out`, one entry a slot of the range.
    pub fn net(&self, range: usize, ranges: &Ranges, out: &mut [i64]) {
        out.fill(0);
        let _ = self.apply(range, ranges, out);
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
        let mut g = Grouped::default();
        g.group(None, &[&bufs], &ranges, Denom::money(0));
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
        let nets = |workers: Option<usize>| {
            let pool = workers.map(|w| phx_exec::Pool::new(&phx_exec::PoolSpec::unpinned(w)).unwrap());
            let mut g = Grouped::default();
            g.group(pool.as_ref(), &[&bufs], &ranges, Denom::money(0));
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
