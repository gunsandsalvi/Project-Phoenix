//! `-F terms`: the design point's distinct terms shapes interned, each held by a few contracts; each day's interns —
//! most of them shapes already held — and as many holds released, then the close retiring the shapes held by none.

use std::collections::BTreeMap;

use phx_num::Missing;
use phx_rand::draws::Draws;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, InternId, Interner, Reuse, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, index, slots, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the interner is measured under.
pub const BASE: &str = "terms";

/// One intern in this many is a shape no contract holds yet: the bench's reading of "most interns are hits".
const NEW_IN: u64 = 16;

/// A shape's encoding is this many bytes at least, and up to twice as many: a packed rate, tenor, schedule and flags.
const SHAPE_BYTES: u64 = 8;

/// The contracts that hold a shape at the fill: one to this many.
const HOLDERS: u64 = 4;

/// The interner, its shapes held now, one entry a hold, the streams the days draw from, and the day it is.
#[derive(Debug, Default)]
pub struct Terms {
    interner: Option<Interner>,
    shapes: Vec<InternId>,
    holds: Vec<InternId>,
    streams: Option<Streams>,
    today: u32,
    retired: u64,
}

impl Terms {
    /// The shapes the last close retired, the same for any workers.
    #[must_use]
    pub fn retired(&self) -> u64 {
        self.retired
    }
}

/// A new shape's bytes, drawn.
fn shape(d: &mut Draws) -> Result<Vec<u8>, FinError> {
    let len = index(SHAPE_BYTES + below_u64(d, SHAPE_BYTES + 1))?;
    let mut out = Vec::with_capacity(len);
    while out.len() < len {
        out.extend_from_slice(&below_u64(d, u64::MAX).to_le_bytes());
    }
    out.truncate(len);
    Ok(out)
}

impl FinBase for Terms {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] terms` shapes interned, each held by one to four contracts, in an interner with room for them and every
    /// measured day's new shapes.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let terms = count(&design.store, "terms", "store")?;
        let new: u64 =
            DayType::ALL.iter().filter_map(|d| design.day(*d)?.get("interns")).map(|n| n.div_ceil(NEW_IN)).sum();
        let max = slots(terms + new)?;
        let bytes = index((terms + new) * SHAPE_BYTES * 2)?;
        let mut interner =
            Interner::new(&mut AddressSpace::empty(), (max, bytes), Reuse::AfterClose).map_err(FinError)?;
        let mut d = streams.draws(BASE, 0, 0);
        for _ in 0..terms {
            let id = interner.intern(&shape(&mut d)?);
            if interner.count(id) == Missing::Present(1) {
                self.shapes.push(id);
            }
            self.holds.push(id);
            for _ in 0..below_u64(&mut d, HOLDERS) {
                interner.hold(id);
                self.holds.push(id);
            }
        }
        let _ = interner.close_day();
        (self.interner, self.streams) = (Some(interner), Some(*streams));
        Ok(Filled { rows: terms })
    }

    /// The day's interns, one in sixteen a new shape and the rest a held shape drawn, then as many holds released, each
    /// read of a shape's bytes, and the close.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(interner), Some(streams)) = (self.interner.as_mut(), self.streams) else {
            return Err(FinError("the terms measured before their fill".to_owned()));
        };
        // A day that opens no contract, as the counts may say of a closed day, interns nothing.
        let Some(&interns) = counts.get("interns") else { return Ok(()) };
        self.today += 1;
        let mut d = streams.draws(BASE, u64::from(self.today), day_of(day)?);
        // The day's shapes staged end to end as their contracts' applies hand them over: new ones drawn, held ones
        // copied; the stage's end interns them in order.
        let (mut bytes, mut ends) = (Vec::new(), Vec::with_capacity(index(interns)?));
        for i in 0..interns {
            if i % NEW_IN == 0 {
                bytes.extend_from_slice(&shape(&mut d)?);
            } else {
                let at = index(below_u64(&mut d, wide(self.shapes.len())))?;
                let id = self.shapes.get(at).copied().ok_or_else(|| FinError("no shape held".to_owned()))?;
                let Missing::Present(v) = interner.get(id) else {
                    return Err(FinError("a held shape with no value".to_owned()));
                };
                bytes.extend_from_slice(v);
            }
            ends.push(slots(wide(bytes.len()))?);
        }
        let mut found = vec![Missing::Absent; ends.len()];
        m.read(BASE, "intern", interns, || interner.intern_many((&bytes, &ends), &mut found));
        let got = found
            .iter()
            .map(|f| match f {
                Missing::Present(id) => Ok(*id),
                Missing::Absent => Err(FinError("a staged shape not interned".to_owned())),
            })
            .collect::<Result<Vec<InternId>, FinError>>()?;
        let _ = m.read(BASE, "get", interns, || {
            got.iter().filter(|id| matches!(interner.get(**id), Missing::Present(_))).count()
        });
        for id in &got {
            if interner.count(*id) == Missing::Present(1) {
                self.shapes.push(*id);
            }
        }
        self.holds.extend_from_slice(&got);
        // The contracts that ended today, as many as opened, each releasing its shape.
        let mut ended = Vec::with_capacity(got.len());
        for _ in 0..interns {
            let at = index(below_u64(&mut d, wide(self.holds.len())))?;
            ended.push(self.holds.swap_remove(at));
        }
        m.read(BASE, "release", interns, || {
            for id in &ended {
                interner.release(*id);
            }
        });
        let listed = wide(ended.len());
        self.retired = m.read(BASE, "close", listed, || interner.close_day());
        let interner = &*interner;
        self.shapes.retain(|id| interner.count(*id) != Missing::Absent);
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.interner.as_ref().map_or(0, StoreStats::bytes), resident: 0 }
    }

    /// The interner's bytes a shape held, rows, index, slots and values together.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let Some(interner) = self.interner.as_ref() else { return Vec::new() };
        let (Ok(bytes), Ok(live)) =
            (interner.bytes().to_string().parse::<f64>(), interner.live().to_string().parse::<f64>())
        else {
            return Vec::new();
        };
        vec![("bytes_per_id", bytes / live)]
    }
}
