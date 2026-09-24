use std::collections::BTreeMap;

use phx_id::{InstrumentId, PartyId};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

/// Where a lien is kept: the holder whose units it marks, the instrument, and its place among that holding's liens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct LienKey {
    pub holder: PartyId,
    pub instrument: InstrumentId,
    pub seq: u32,
}

/// Units of a holding pledged to a named party; a re-pledge names the lien whose units it pledges on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lien {
    pub key: LienKey,
    pub units: i64,
    pub to: PartyId,
    pub chain: Missing<LienKey>,
}

/// Every lien, keyed by holder, instrument and sequence, so a holding's pledged total is one range read.
#[clause("REG.2", "REG.16")]
#[derive(Clone, Debug, Default)]
pub struct Liens {
    liens: BTreeMap<LienKey, Lien>,
    next: BTreeMap<(PartyId, InstrumentId), u32>,
}

fn range(holder: PartyId, instrument: InstrumentId) -> core::ops::RangeInclusive<LienKey> {
    LienKey { holder, instrument, seq: 0 }..=LienKey { holder, instrument, seq: u32::MAX }
}

impl Liens {
    /// The units of a holding under liens of its holder's own; re-pledges pledge those same units again down a
    /// chain and are not counted twice.
    #[must_use]
    pub fn pledged(&self, holder: PartyId, instrument: InstrumentId) -> i64 {
        self.liens
            .range(range(holder, instrument))
            .filter(|(_, l)| l.chain == Missing::Absent)
            .map(|(_, l)| l.units)
            .sum()
    }

    fn seq(&mut self, holder: PartyId, instrument: InstrumentId) -> u32 {
        let next = self.next.entry((holder, instrument)).or_insert(0);
        let seq = *next;
        let Some(after) = next.checked_add(1) else {
            capacity_exceeded!("liens on one holding", u32::MAX, seq);
        };
        *next = after;
        seq
    }

    /// Units of a holding pledged to a party; `bound` is what else holds the holding's units (offers they cover).
    /// Pledging more than its free units stops the run: no unit is pledged twice.
    pub fn pledge(
        &mut self,
        holder: PartyId,
        instrument: InstrumentId,
        units: i64,
        to: PartyId,
        held: i64,
        bound: i64,
    ) -> LienKey {
        let free = held - self.pledged(holder, instrument) - bound;
        if units <= 0 || units > free {
            violation!(clause = "REG.16", "units pledged that are not free", units = units, free = free);
        }
        let key = LienKey { holder, instrument, seq: self.seq(holder, instrument) };
        self.liens.insert(key, Lien { key, units, to, chain: Missing::Absent });
        key
    }

    /// Pledged units pledged on by their lienholder to another party, as a chain back to the first lien; no more than
    /// the lien holds and has not already pledged on.
    pub fn repledge(&mut self, on: LienKey, units: i64, to: PartyId) -> LienKey {
        let Some(first) = self.liens.get(&on).copied() else {
            violation!(clause = "REG.2", "a re-pledge of a lien that does not exist", seq = on.seq);
        };
        let onward: i64 = self.liens.values().filter(|l| l.chain == Missing::Present(on)).map(|l| l.units).sum();
        if units <= 0 || units > first.units - onward {
            violation!(
                clause = "REG.16",
                "units re-pledged beyond their lien",
                units = units,
                free = first.units - onward
            );
        }
        let key = LienKey { holder: on.holder, instrument: on.instrument, seq: self.seq(on.holder, on.instrument) };
        self.liens.insert(key, Lien { key, units, to, chain: Missing::Present(on) });
        key
    }

    /// A lien released; one with re-pledges on it stops the run, as those must be released first.
    pub fn release(&mut self, key: LienKey) -> Lien {
        if self.liens.values().any(|l| l.chain == Missing::Present(key)) {
            violation!(clause = "REG.2", "a lien released with re-pledges still on it", seq = key.seq);
        }
        let Some(lien) = self.liens.remove(&key) else {
            violation!(clause = "REG.2", "a lien released that does not exist", seq = key.seq);
        };
        lien
    }

    /// The liens on a holding, in order.
    pub fn on(&self, holder: PartyId, instrument: InstrumentId) -> impl Iterator<Item = &Lien> {
        self.liens.range(range(holder, instrument)).map(|(_, l)| l)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.liens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.liens.is_empty()
    }
}
