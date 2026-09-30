//! `-F streams`: the day's draws as Philox blocks — a cursor per subject at the day's slot, one block each, as a
//! retail want draws its choice — and as batches of four subjects' first blocks made side by side, as a meeting's
//! round draws its buyers' tastes.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_rand::{Draws, Seed, SlotOrdinal, StreamFamily, Subject, SubjectTag, family_key};

use crate::design::Design;
use crate::fill::Streams as FillStreams;
use crate::kept::{count, day_of};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the streams are measured under.
pub const BASE: &str = "streams";

/// Subjects a batch draws for side by side.
const LANES: u64 = 4;

/// The slot the measured draws are made in: the retail meeting's.
const SLOT: u32 = 6;

/// The design point's seed, and what the days drew, folded so the draws are not optimised away.
#[derive(Debug, Default)]
pub struct StreamsBase {
    seed: u64,
    folded: u64,
}

impl FinBase for StreamsBase {
    fn name(&self) -> &'static str {
        BASE
    }

    /// Nothing to fill: a stream keeps no state.
    fn fill(&mut self, design: &Design, _streams: &FillStreams) -> Result<Filled, FinError> {
        self.seed = design.point.seed;
        Ok(Filled { rows: 0 })
    }

    /// A block for each of the day's `retail` wants, each its own subject's cursor, then as many draws in batches of
    /// four subjects.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let wants = count(counts, "retail", "day")?;
        let key = family_key(Seed::new(self.seed), StreamFamily::World, "fin.streams");
        let (today, slot) = (day_of(day)?, SlotOrdinal::new(SLOT));
        let blocks = m.read(BASE, "block", wants, || {
            let mut fold = 0_u64;
            for s in 0..wants {
                let mut d = Draws::at(key, Subject::new(SubjectTag::Party, s), today, slot);
                fold ^= d.next_u64() ^ d.next_u64();
            }
            black_box(fold)
        });
        let batches = wants / LANES;
        let x4 = m.read(BASE, "x4", batches, || {
            let mut fold = 0_u64;
            for b in 0..batches {
                let subjects = [0, 1, 2, 3].map(|l| Subject::new(SubjectTag::Party, b * LANES + l));
                for mut d in Draws::x4(key, subjects, today, slot.get(), 0) {
                    fold ^= d.next_u64() ^ d.next_u64();
                }
            }
            black_box(fold)
        });
        self.folded ^= blocks ^ x4;
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: 0, resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
}
