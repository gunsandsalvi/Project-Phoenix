//! `-F epoch`: the design point's party slots under epoch bits, each day's `touched` parties marked three times — by
//! three applies, each in its target ranges' order — and the day's set iterated once, as the audit reads it.

use std::collections::BTreeMap;

use phx_id::Slot;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, EPOCH_CHUNK_ROWS, EpochBits, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, slots};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the epoch bits are measured under.
pub const BASE: &str = "epoch";

/// The applies that mark a touched party in a day.
const APPLIES: u64 = 3;

/// The bits, the parties they cover, the streams the days draw from, the day it is, and the last day's set.
#[derive(Debug, Default)]
pub struct Epoch {
    bits: Option<EpochBits>,
    parties: u64,
    streams: Option<Streams>,
    today: u32,
    set: u64,
}

impl Epoch {
    /// The rows the last day's iteration read, the same for any workers.
    #[must_use]
    pub fn set(&self) -> u64 {
        self.set
    }
}

impl FinBase for Epoch {
    fn name(&self) -> &'static str {
        BASE
    }

    /// Bits over `[store] directory_slots` party slots.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        self.parties = count(&design.store, "directory_slots", "store")?;
        self.bits = Some(EpochBits::new(&mut AddressSpace::empty(), slots(self.parties)?));
        self.streams = Some(*streams);
        Ok(Filled { rows: self.parties })
    }

    /// The day's touched parties drawn, marked by each apply in slot order, then the set iterated.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(bits), Some(streams)) = (self.bits.as_mut(), self.streams) else {
            return Err(FinError("the epoch bits measured before their fill".to_owned()));
        };
        // A day that touches nothing, as the counts may say of a closed day, marks nothing.
        let Some(&touched) = counts.get("touched") else { return Ok(()) };
        self.today += 1;
        let mut d = streams.draws(BASE, u64::from(self.today), day_of(day)?);
        let mut parties: Vec<Slot> =
            (0..touched).map(|_| slots(below_u64(&mut d, self.parties)).map(Slot::new)).collect::<Result<_, _>>()?;
        parties.sort_unstable();
        let today = self.today;
        // Each apply marks the rows of its target ranges through the chunk it holds, in slot order; the rows come to
        // it partitioned by range, as the apply's items do.
        let mut by_chunk: Vec<&[Slot]> = Vec::new();
        let mut rest = parties.as_slice();
        for chunk in bits.chunks_mut() {
            let take = rest.partition_point(|p| p.get() < chunk.first() + EPOCH_CHUNK_ROWS);
            let (mine, later) = rest.split_at(take);
            by_chunk.push(mine);
            rest = later;
        }
        m.read(BASE, "mark", touched * APPLIES, || {
            for _ in 0..APPLIES {
                for (mut chunk, mine) in bits.chunks_mut().zip(&by_chunk) {
                    for p in *mine {
                        chunk.mark(*p, today);
                    }
                }
            }
        });
        let bits = &*bits;
        let mut distinct = 0_u64;
        bits.for_each_marked_word(today, |_, w| distinct += u64::from(w.count_ones()));
        // The audit reads the day's set a word at a time, each word's rows in its own loop.
        self.set = m.read(BASE, "iter", distinct, || {
            let mut n = 0_u64;
            bits.for_each_marked_word(today, |_, w| n += u64::from(w.count_ones()));
            n
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.bits.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    /// The bits' bits a party, with their day words and summary.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let (Some(bits), Ok(parties)) = (self.bits.as_ref(), self.parties.to_string().parse::<f64>()) else {
            return Vec::new();
        };
        match bits.bytes().to_string().parse::<f64>() {
            Ok(bytes) => vec![("bits_per_row", bytes * 8.0 / parties)],
            Err(_) => Vec::new(),
        }
    }
}
