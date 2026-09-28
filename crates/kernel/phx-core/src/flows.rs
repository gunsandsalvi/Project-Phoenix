//! Flows: every movement of money or of held units the day makes, each naming both its parties, appended by the
//! chunk that makes it and gathered in chunk order, so the day's flows are the same whatever the workers.

use phx_id::PartyKey;

/// One movement: from the payer to the payee, an amount in a currency or a unit, for a reason, at the payer's
/// payment order for the reason, from a source the reason names (a contract, a match, a transformation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flow {
    pub payer: PartyKey,
    pub payee: PartyKey,
    pub amount: i64,
    pub source: u32,
    pub reason: u16,
    /// The currency's index for money, or the unit's for goods and holdings, by `kinds`.
    pub denomination: u8,
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

/// The day's flows grouped by the range of their payer, whole, since a payer's failures follow its flows' order, and
/// by the range of their payee as credits; kept across days, so each range's parties are netted by one worker reading
/// only its own.
#[derive(Debug, Default)]
pub struct Grouped {
    pub by_payer: phx_exec::partition::Partitioned<Flow>,
    pub by_payee: phx_exec::partition::Partitioned<Credit>,
}

impl Grouped {
    /// Groups every flow of the buffers given, in their order, by payer range and by payee range.
    pub fn group(&mut self, pool: Option<&phx_exec::Pool>, bufs: &[&FlowBufs], ranges: &Ranges) {
        let slices: Vec<&[Flow]> = bufs.iter().flat_map(|b| b.slices()).collect();
        let n = ranges.count();
        phx_exec::partition::partition_into(pool, &slices, n, |f| ranges.of(f.payer), &mut self.by_payer);
        phx_exec::partition::partition_map_into(
            pool,
            &slices,
            n,
            |f| ranges.of(f.payee),
            |f| Credit { payee: f.payee, amount: f.amount },
            &mut self.by_payee,
        );
    }

    /// A range's net per party, credits less debits, added to `money`, one entry a slot of the range; returns the
    /// payers the day's debits leave below nothing.
    pub fn apply(&self, range: usize, ranges: &Ranges, money: &mut [i64]) -> u64 {
        let (_, first) = ranges.start(range);
        let at = |p: PartyKey| usize::try_from(p.slot().get() - first).unwrap_or(usize::MAX);
        for c in self.by_payee.bucket(range) {
            if let Some(v) = money.get_mut(at(c.payee)) {
                *v += c.amount;
            }
        }
        for f in self.by_payer.bucket(range) {
            if let Some(v) = money.get_mut(at(f.payer)) {
                *v -= f.amount;
            }
        }
        let mut short = 0_u64;
        let mut last = None;
        for f in self.by_payer.bucket(range) {
            if last != Some(f.payer) && money.get(at(f.payer)).is_some_and(|m| *m < 0) {
                short += 1;
            }
            last = Some(f.payer);
        }
        short
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

    use super::{Flow, FlowBufs, Grouped, Ranges};

    fn flow(from: (u8, u32), to: (u8, u32), amount: i64) -> Flow {
        let key = |(k, s): (u8, u32)| PartyKey::new(k, Slot::new(s));
        Flow { payer: key(from), payee: key(to), amount, source: 0, reason: 0, denomination: 0, order: 0 }
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
        g.group(None, &[&bufs], &ranges);
        let mut out = [0_i64; 4];
        g.net(0, &ranges, &mut out);
        assert_eq!(out, [0, -43, 0, 0]);
        g.net(2, &ranges, &mut out);
        assert_eq!(out, [0, 13, 0, 0], "slot 9 is the second of the third range");
        g.net(3, &ranges, &mut out);
        assert_eq!(out, [0, 0, 30, 0]);
    }
}
