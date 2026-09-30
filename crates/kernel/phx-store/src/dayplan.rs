//! The day plan: each day buffer's life on the day's slots, and its place in one region, so buffers whose lives never
//! meet share pages and the day's resident buffers are its largest live set, not the sum of them.

use phx_macros::{clause, opening};
use phx_num::violation;

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::convert::to_u64;
use crate::region::Region;
use crate::stats::StoreStats;

/// The slots a buffer lives over: first filled in `fill`, released after `release`, both places on the day's slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Life {
    pub fill: u16,
    pub release: u16,
}

impl Life {
    /// Whether two lives share a slot.
    #[must_use]
    pub fn meets(self, other: Life) -> bool {
        self.fill <= other.release && other.fill <= self.release
    }

    fn slots(self) -> u16 {
        self.release - self.fill
    }
}

/// A buffer's bytes at the heaviest day: its own, or what the day's largest live set leaves beside the buffers that
/// share its slots, for the one buffer a plan sizes to reach its maximum and never pass it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    Bytes(u64),
    Rest,
}

/// A buffer's declaration: its name, its size and its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufDecl {
    pub name: &'static str,
    pub size: Size,
    pub life: Life,
}

/// A buffer placed: its place among the declarations, its offset and bytes in the region, and its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub decl: usize,
    pub offset: u64,
    pub bytes: u64,
    pub life: Life,
}

/// The plan: every buffer placed, and the region's extent that holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayPlan {
    placed: Vec<Placed>,
    extent: u64,
}

/// The bytes live at each slot, over `slots` slots.
#[opening]
fn live_by_slot(lives: impl Iterator<Item = (Life, u64)>, slots: u16) -> Vec<u64> {
    let mut live = vec![0; usize::from(slots)];
    for (life, bytes) in lives {
        for at in life.fill..=life.release {
            if let Some(cell) = live.get_mut(usize::from(at)) {
                *cell += bytes;
            }
        }
    }
    live
}

/// The largest of the figures, or none of none.
fn largest(figures: &[u64]) -> u64 {
    figures.iter().fold(0, |a, f| if *f > a { *f } else { a })
}

impl DayPlan {
    /// Every buffer placed at the lowest offset where it meets no placed buffer whose life meets its own, the longest
    /// lives first, then the largest, then in declaration order: a plan of the declarations alone.
    ///
    /// # Errors
    /// A life that ends before it begins or past the day's slots, and more than one buffer sized by the rest.
    #[opening]
    #[clause("TIME.10")]
    pub fn plan(decls: &[BufDecl], slots: u16) -> Result<DayPlan, String> {
        for d in decls {
            if d.life.fill > d.life.release || d.life.release >= slots {
                return Err(format!("the buffer `{}` has no life within the day's {slots} slots", d.name));
            }
        }
        let rests: Vec<usize> =
            (0..decls.len()).filter(|i| decls.get(*i).is_some_and(|d| d.size == Size::Rest)).collect();
        if rests.len() > 1 {
            return Err("more than one buffer sized by what the day's maximum leaves".to_owned());
        }
        let fixed = |d: &BufDecl| match d.size {
            Size::Bytes(b) => Some((d.life, b)),
            Size::Rest => None,
        };
        let live = live_by_slot(decls.iter().filter_map(fixed), slots);
        let peak = largest(&live);
        let rest_bytes = |d: &BufDecl| {
            let beside: Vec<u64> =
                (d.life.fill..=d.life.release).filter_map(|at| live.get(usize::from(at)).copied()).collect();
            peak - largest(&beside)
        };
        let bytes_of = |d: &BufDecl| match d.size {
            Size::Bytes(b) => b,
            Size::Rest => rest_bytes(d),
        };
        let mut order: Vec<(usize, &BufDecl)> = decls.iter().enumerate().collect();
        order.sort_by_key(|(i, d)| (std::cmp::Reverse(d.life.slots()), std::cmp::Reverse(bytes_of(d)), *i));
        let mut placed: Vec<Placed> = Vec::with_capacity(decls.len());
        for (i, d) in order {
            let bytes = bytes_of(d);
            let mut taken: Vec<(u64, u64)> =
                placed.iter().filter(|p| p.life.meets(d.life)).map(|p| (p.offset, p.offset + p.bytes)).collect();
            taken.sort_unstable();
            let mut offset = 0;
            for (start, end) in taken {
                if start >= offset + bytes {
                    break;
                }
                if end > offset {
                    offset = end;
                }
            }
            placed.push(Placed { decl: i, offset, bytes, life: d.life });
        }
        placed.sort_by_key(|p| p.decl);
        let extent = placed.iter().fold(0, |a, p| if p.offset + p.bytes > a { p.offset + p.bytes } else { a });
        Ok(DayPlan { placed, extent })
    }

    /// The bytes the region holds to place every buffer.
    #[must_use]
    pub fn extent(&self) -> u64 {
        self.extent
    }

    /// The largest bytes live at any slot.
    #[opening]
    #[must_use]
    pub fn peak_live(&self, slots: u16) -> u64 {
        largest(&live_by_slot(self.placed.iter().map(|p| (p.life, p.bytes)), slots))
    }

    /// Each buffer placed, in declaration order.
    #[must_use]
    pub fn placed(&self) -> &[Placed] {
        &self.placed
    }

    /// Refuses a buffer read outside its life: after its release its pages are another buffer's.
    #[clause("TIME.10")]
    pub fn check_read(&self, decl: usize, slot: u16) {
        if cfg!(debug_assertions)
            && let Some(p) = self.placed.get(decl)
            && !(p.life.fill..=p.life.release).contains(&slot)
        {
            violation!(clause = "TIME.10", "a day buffer read outside its life", buffer = decl, slot = slot);
        }
    }
}

/// The region the plan's buffers share, reserved at its extent and committed as its lanes are first written.
#[derive(Debug)]
pub struct DayRegion<B: Backing = SystemBacking> {
    plan: DayPlan,
    words: Region<u64, B>,
}

/// Bytes to the words that hold them.
fn words(bytes: u64) -> usize {
    usize::try_from(bytes.div_ceil(u64::from(u64::BITS / u8::BITS)))
        .unwrap_or_else(|_| phx_num::capacity_exceeded!("a day region's words", usize::MAX, bytes))
}

impl<B: Backing> DayRegion<B> {
    #[opening]
    pub fn new(space: &mut AddressSpace, plan: DayPlan) -> DayRegion<B> {
        let words = Region::reserve(space, words(plan.extent));
        DayRegion { plan, words }
    }

    #[must_use]
    pub fn plan(&self) -> &DayPlan {
        &self.plan
    }

    /// A buffer's words at a slot of its life, `len` of them, to be written in place.
    pub fn lane_mut(&mut self, decl: usize, slot: u16, len: usize) -> &mut [u64] {
        self.plan.check_read(decl, slot);
        let Some(p) = self.plan.placed.get(decl).copied() else {
            violation!(clause = "TIME.10", "a day buffer the plan does not place", buffer = decl);
        };
        let (start, room) = (words(p.offset), words(p.bytes));
        if len > room {
            violation!(clause = "TIME.10", "a day buffer written past its planned bytes", buffer = decl, len = len);
        }
        self.words.ensure(start + len);
        match self.words.slice_mut(start + len).get_mut(start..) {
            Some(lane) => lane,
            None => violation!(clause = "SET.12", "a lane past its region", buffer = decl),
        }
    }

    /// The bytes a buffer's lane holds committed: where its bytes meet the region's committed prefix.
    #[must_use]
    pub fn lane_bytes(&self, decl: usize) -> u64 {
        let committed = to_u64(self.words.bytes_committed());
        let Some(p) = self.plan.placed.get(decl) else {
            violation!(clause = "TIME.10", "a day buffer the plan does not place", buffer = decl);
        };
        if committed <= p.offset {
            0
        } else if committed < p.offset + p.bytes {
            committed - p.offset
        } else {
            p.bytes
        }
    }
}

impl<B: Backing> StoreStats for DayRegion<B> {
    fn rows_live(&self) -> u64 {
        to_u64(self.plan.placed.len())
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        to_u64(self.words.bytes_committed())
    }
}

#[cfg(test)]
#[path = "dayplan_tests.rs"]
mod tests;
