//! `-F stalls`, the sum-tree part: the design point's stalls in one sum-tree per (good, zone), each day's reprices and
//! sold-out removals set in place, and the day's retail wants drawing a stall in proportion to its weight.

use std::collections::BTreeMap;

use phx_num::Missing;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, StoreStats, SumTrees};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, index, slots, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the stalls' trees are measured under.
pub const BASE: &str = "stalls";

/// The base whose own figures the trees' finds and bytes are.
pub const TREES: &str = "sumtree";

/// A stall's weight is its price term quantised: below this many steps.
const WEIGHT_STEPS: u64 = 1 << 20;

/// Every stall's tree and place in it, the trees, and the draws of the last day that found a stall.
#[derive(Debug, Default)]
pub struct Stalls {
    at: Vec<(u32, usize)>,
    trees: Option<SumTrees>,
    streams: Option<Streams>,
    found: u64,
}

impl Stalls {
    /// The draws that found a stall on the last day, the same for any workers.
    #[must_use]
    pub fn found(&self) -> u64 {
        self.found
    }
}

impl FinBase for Stalls {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] stalls` stalls, each in the tree of a (good, zone) drawn for it among `[store] stall_keys`, at a drawn
    /// weight, the trees built one after another as a load's rebuild builds them from their owners' rows.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let stalls = count(&design.store, "stalls", "store")?;
        let keys = count(&design.store, "stall_keys", "store")?;
        // Room for every class a tree may reach: twice the stalls, as the classes double.
        let mut trees = SumTrees::new(&mut AddressSpace::empty(), slots(stalls * 2 + keys * 4)?, slots(keys)?);
        let mut d = streams.draws(BASE, 0, 0);
        let mut drawn: Vec<(u32, u64)> = (0..stalls)
            .map(|_| slots(below_u64(&mut d, keys)).map(|k| (k, 1 + below_u64(&mut d, WEIGHT_STEPS))))
            .collect::<Result<_, _>>()?;
        drawn.sort_by_key(|(k, _)| *k);
        self.at = Vec::with_capacity(index(stalls)?);
        let mut rows = drawn.iter().peekable();
        for key in 0..slots(keys)? {
            let tree = trees.make();
            while let Some((_, w)) = rows.next_if(|(k, _)| *k == key) {
                self.at.push((tree, trees.push(tree, *w)));
            }
        }
        (self.trees, self.streams) = (Some(trees), Some(*streams));
        Ok(Filled { rows: stalls })
    }

    /// The day's reprices and removals set at drawn stalls, then its retail wants each drawing a stall of a drawn
    /// (good, zone).
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(trees), Some(streams)) = (self.trees.as_mut(), self.streams) else {
            return Err(FinError("the stalls measured before their fill".to_owned()));
        };
        let mut d = streams.draws(BASE, 1, day_of(day)?);
        let stalls = wide(self.at.len());
        let mut sets: Vec<(u32, usize, u64)> = Vec::new();
        // A day that reprices or sells out nothing, as a closed day may not, sets nothing of that kind.
        for (key, weighs) in [("reprices", true), ("removals", false)] {
            let Some(&n) = counts.get(key) else { continue };
            for _ in 0..n {
                let Some(&(key, place)) = self.at.get(index(below_u64(&mut d, stalls))?) else { continue };
                sets.push((key, place, if weighs { 1 + below_u64(&mut d, WEIGHT_STEPS) } else { 0 }));
            }
        }
        // Each (good, zone) is one market key's job, whose reprices and sell-outs it sets in turn.
        sets.sort_unstable_by_key(|(key, place, _)| (*key, *place));
        m.read(BASE, "update", wide(sets.len()), || {
            for (key, place, w) in &sets {
                trees.set(*key, *place, *w);
            }
        });
        let Some(&retail) = counts.get("retail") else { return Ok(()) };
        let keys = u64::from(trees.trees());
        let mut wants: Vec<(u32, u64)> = (0..retail)
            .map(|_| slots(below_u64(&mut d, keys)).map(|k| (k, below_u64(&mut d, WEIGHT_STEPS))))
            .collect::<Result<_, _>>()?;
        // The day's wants are bucketed by (good, zone) before they meet, each bucket drawing from its own tree.
        wants.sort_unstable_by_key(|(key, _)| *key);
        let trees = &*trees;
        self.found = m.read(TREES, "find", retail, || {
            wants.iter().fold(0_u64, |n, (key, x)| {
                let total = trees.total(*key);
                if total == 0 { n } else { n + u64::from(matches!(trees.find(*key, x % total), Missing::Present(_))) }
            })
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.trees.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    /// The trees' bytes a weight: their extents over the stalls they hold; the headers are the trees' own, 16 bytes each.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        self.trees
            .as_ref()
            .and_then(|t| wide(t.weight_bytes()).checked_div(t.rows_live()))
            .and_then(|b| b.to_string().parse().ok())
            .map(|b| vec![("sumtree.bytes_per_weight", b)])
            .unwrap_or_default()
    }
}
