//! The day's use of the network: each segment's load today, the sum of the flows of every (mode, origin, destination)
//! pair whose route crosses it, stamped by day so a day that put nothing on it reads none with no pass to clear it;
//! the segments the sums carried past their capacity admitted by lot; and a pair's time read at the loads all of the
//! day's flows put on its legs.

use phx_id::Day;
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::{Draws, multivariate_hypergeometric};

use super::routes::Route;
use super::segments::{SegmentId, SegmentRow, Segments};

/// A day no run reaches, held by a segment no day has loaded.
const NEVER: u32 = u32::MAX;

/// A pair's flow today: its mode, its origin and destination zones, and what it carries in its mode's unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PairFlow {
    pub mode: u8,
    pub from: u16,
    pub to: u16,
    pub count: u32,
}

/// A segment's load and the day it was put on.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct Load {
    day: u32,
    amount: u32,
}

const UNLOADED: Load = Load { day: NEVER, amount: 0 };

/// Each segment's load today and the day's buffers: the segments loaded today, those carried past their capacity,
/// the flows crossing them, and what each flow was admitted.
#[clause("GEO.20", "GEO.13")]
#[derive(Clone, Debug, Default)]
pub struct SegmentUse {
    loads: Vec<Load>,
    day: u32,
    flows: usize,
    touched: Vec<u32>,
    over: Vec<u32>,
    /// A bit a segment, set while the day's admission runs for those past their capacity.
    marked: Vec<u64>,
    crossing: Vec<(u32, u32)>,
    admitted: Vec<u32>,
    counts: Vec<u64>,
    drawn: Vec<u64>,
}

fn at(n: u32) -> usize {
    match usize::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("the day's use", usize::MAX, n),
    }
}

fn bit(seg: u32) -> (usize, u64) {
    (at(seg / u64::BITS), 1 << (seg % u64::BITS))
}

fn flip(marks: &mut [u64], seg: u32) {
    let (w, b) = bit(seg);
    if let Some(word) = marks.get_mut(w) {
        *word ^= b;
    }
}

fn is_set(marks: &[u64], seg: u32) -> bool {
    let (w, b) = bit(seg);
    marks.get(w).is_some_and(|word| word & b != 0)
}

fn wide(n: usize) -> u64 {
    match u64::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("the day's use", u64::MAX, n),
    }
}

fn narrow(n: usize) -> u32 {
    match u32::try_from(n) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("the day's use", u32::MAX, n),
    }
}

fn route_of<'r>(route: &impl Fn(&PairFlow) -> Missing<Route<'r>>, f: &PairFlow) -> Route<'r> {
    match route(f) {
        Missing::Present(r) => r,
        Missing::Absent => {
            violation!(clause = "GEO.20", "a flow over a pair no route joins", mode = f.mode, from = f.from, to = f.to)
        }
    }
}

impl SegmentUse {
    /// The loads a segment of the network holds today: none on a day that put nothing on it.
    #[must_use]
    pub fn load(&self, id: SegmentId, day: Day) -> u32 {
        match self.loads.get(at(id.get())) {
            Some(l) if l.day == day.get() => l.amount,
            _ => 0,
        }
    }

    /// The day's flows added to the segments of their routes; the items added. A day's flows are added once, all
    /// together, so its loads are every flow's before anything reads them.
    #[clause("GEO.20")]
    pub fn add_flows<'r>(
        &mut self,
        segments: &Segments,
        flows: &[PairFlow],
        route: impl Fn(&PairFlow) -> Missing<Route<'r>>,
        day: Day,
    ) -> u64 {
        if self.day == day.get() && self.flows != 0 {
            violation!(clause = "GEO.20", "a day's flows added twice", day = day.get());
        }
        if self.loads.len() < segments.len() {
            self.loads.resize(segments.len(), UNLOADED);
        }
        (self.day, self.flows) = (day.get(), flows.len());
        self.touched.clear();
        self.admitted.clear();
        self.admitted.extend(flows.iter().map(|f| f.count));
        let mut items = 0_u64;
        for f in flows {
            let r = route_of(&route, f);
            for seg in r.segments {
                let Some(l) = self.loads.get_mut(at(*seg)) else {
                    violation!(clause = "GEO.20", "a route over a segment never opened", segment = *seg);
                };
                if l.day != day.get() {
                    *l = Load { day: day.get(), amount: 0 };
                    self.touched.push(*seg);
                }
                let Some(after) = l.amount.checked_add(f.count) else {
                    capacity_exceeded!("a segment's load", u32::MAX, u64::from(l.amount) + u64::from(f.count));
                };
                l.amount = after;
            }
            items += wide(r.segments.len());
        }
        self.over.clear();
        for seg in &self.touched {
            let capacity = segments.row(SegmentId::new(*seg)).capacity();
            if self.loads.get(at(*seg)).is_some_and(|l| l.amount > capacity) {
                self.over.push(*seg);
            }
        }
        items
    }

    /// The segments the day's flows loaded, and those they carried past their capacity.
    #[must_use]
    pub fn loaded(&self) -> (usize, usize) {
        (self.touched.len(), self.over.len())
    }

    /// The segments the day's flows carried past their capacity, in identity order, each admitting its capacity
    /// across the flows still admitted over it by one draw without replacement; a flow's refused part leaves every
    /// leg of its route before the next segment is read. Returns the items read.
    #[clause("GEO.13", "REP.22")]
    pub fn admit<'r>(
        &mut self,
        segments: &Segments,
        flows: &[PairFlow],
        route: impl Fn(&PairFlow) -> Missing<Route<'r>>,
        (draws, day): (&mut Draws, Day),
    ) -> u64 {
        let added = if self.day == day.get() { self.flows } else { 0 };
        if flows.len() != added {
            violation!(clause = "GEO.13", "admission over other flows than the day's", flows = flows.len());
        }
        if self.over.is_empty() {
            return 0;
        }
        self.over.sort_unstable();
        let words = self.loads.len().div_ceil(at(u64::BITS));
        if self.marked.len() < words {
            self.marked.resize(words, 0);
        }
        for seg in &self.over {
            flip(&mut self.marked, *seg);
        }
        self.crossing.clear();
        let mut items = 0_u64;
        for (i, f) in flows.iter().enumerate() {
            let r = route_of(&route, f);
            for seg in r.segments {
                if is_set(&self.marked, *seg) {
                    self.crossing.push((*seg, narrow(i)));
                }
            }
            items += wide(r.segments.len());
        }
        for seg in &self.over {
            flip(&mut self.marked, *seg);
        }
        self.crossing.sort_unstable();
        let mut start = 0;
        for seg in &self.over {
            let Some(rest) = self.crossing.get(start..) else {
                violation!(clause = "GEO.13", "a crossing past the day's", segment = *seg);
            };
            let (crossing, end) = rest.split_at(rest.partition_point(|(o, _)| o == seg));
            start = self.crossing.len() - end.len();
            let capacity = segments.row(SegmentId::new(*seg)).capacity();
            let load = self.load(SegmentId::new(*seg), day);
            if load <= capacity {
                continue;
            }
            self.counts.clear();
            for (_, p) in crossing {
                let Some(a) = self.admitted.get(at(*p)) else {
                    violation!(clause = "GEO.13", "a crossing flow past the day's", flow = *p);
                };
                self.counts.push(u64::from(*a));
            }
            if self.counts.iter().sum::<u64>() != u64::from(load) {
                violation!(clause = "GEO.13", "a load other than its flows' sum", segment = *seg, load = load);
            }
            self.drawn.clear();
            self.drawn.resize(self.counts.len(), 0);
            multivariate_hypergeometric(draws, &self.counts, u64::from(capacity), &mut self.drawn);
            for ((_, p), kept) in crossing.iter().zip(&self.drawn) {
                let (Some(a), Some(f)) = (self.admitted.get_mut(at(*p)), flows.get(at(*p))) else { continue };
                let Ok(kept) = u32::try_from(*kept) else {
                    violation!(clause = "CHN.2", "a draw past its count", flow = *p);
                };
                let Some(refused) = a.checked_sub(kept) else {
                    violation!(clause = "CHN.2", "a draw past its count", flow = *p);
                };
                *a = kept;
                if refused == 0 {
                    continue;
                }
                for leg in route_of(&route, f).segments {
                    let Some(l) = self.loads.get_mut(at(*leg)).filter(|l| l.day == day.get()) else {
                        violation!(clause = "GEO.13", "a refused flow off a leg it never loaded", segment = *leg);
                    };
                    let Some(left) = l.amount.checked_sub(refused) else {
                        violation!(clause = "GEO.13", "a leg's load below its flows'", segment = *leg);
                    };
                    l.amount = left;
                    items += 1;
                }
            }
        }
        items
    }

    /// What each of the day's flows was admitted, in the order they were added: its count where no segment it
    /// crosses was carried past its capacity.
    #[must_use]
    pub fn admitted(&self) -> &[u32] {
        &self.admitted
    }

    /// A route's time at the day's loads: each leg's time at its load by the mode's curve, summed.
    #[clause("GEO.20")]
    pub fn time_at(
        &self,
        segments: &Segments,
        route: Route<'_>,
        day: Day,
        curve: impl Fn(SegmentRow, u32) -> u64,
    ) -> u64 {
        let mut total = 0_u64;
        for seg in route.segments {
            let id = SegmentId::new(*seg);
            let Some(sum) = total.checked_add(curve(segments.row(id), self.load(id, day))) else {
                capacity_exceeded!("a route's time", u64::MAX, total);
            };
            total = sum;
        }
        total
    }

    /// The bytes its loads and the day's buffers hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.loads.capacity() * size_of::<Load>()
            + (self.touched.capacity() + self.over.capacity() + self.admitted.capacity()) * size_of::<u32>()
            + self.crossing.capacity() * size_of::<(u32, u32)>()
            + (self.counts.capacity() + self.drawn.capacity() + self.marked.capacity()) * size_of::<u64>()
    }
}

impl phx_store::StoreStats for SegmentUse {
    fn rows_live(&self) -> u64 {
        wide(self.loads.len())
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        wide(SegmentUse::bytes(self))
    }
}

#[cfg(test)]
#[path = "day_use_tests.rs"]
mod tests;
