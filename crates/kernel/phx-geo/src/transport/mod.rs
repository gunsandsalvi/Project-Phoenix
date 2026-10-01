//! The transport network: its segments, and each mode's routes between the regions' market zones, rebuilt from the
//! open segments whenever a segment of the mode opens, closes, reopens or is removed, and at load.

pub mod day_use;
pub mod routes;
pub mod segments;

use phx_id::Day;
use phx_macros::clause;
use phx_num::Missing;
use phx_store::StoreStats;

pub use day_use::{PairFlow, SegmentUse};
pub use routes::{Route, Routes};
pub use segments::{Change, SegmentDecl, SegmentId, SegmentRow, Segments};

/// The segments, the places routes join and the zones they lie among, the modes, the day the routes were last
/// computed for, and each mode's routes, derived and rebuilt at load.
#[clause("GEO.4", "FRT.2")]
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Transport {
    pub segments: Segments,
    places: Vec<u16>,
    zones: u32,
    modes: u8,
    computed: u32,
    #[saved(skip, rebuild = Transport::rebuild)]
    routes: Vec<Routes>,
}

impl Transport {
    /// The network over its segments, its routes computed for a day.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(segments: Segments, (places, zones): (&[u16], u32), modes: u8, day: Day) -> Transport {
        let mut t =
            Transport { segments, places: places.to_vec(), zones, modes, computed: day.get(), routes: Vec::new() };
        t.segments.settle();
        t.rebuild();
        t
    }

    /// Every mode's routes computed again over the segments open on the day last computed for.
    #[phx_macros::opening]
    fn rebuild(&mut self) -> u64 {
        let day = Day::new(self.computed);
        self.routes = (0..self.modes).map(|m| Routes::new(m, &self.places, self.zones)).collect();
        for r in &mut self.routes {
            r.admit_all(&self.segments);
            let _ = r.recompute(&self.segments, day);
        }
        u64::from(self.modes)
    }

    fn recompute(&mut self, mode: u8, day: Day) -> u64 {
        self.computed = day.get();
        match self.routes.get_mut(usize::from(mode)) {
            Some(r) => r.recompute(&self.segments, day),
            None => 0,
        }
    }

    /// A mode's route between two places, or `Missing` where no open path joins them.
    pub fn route(&self, mode: u8, from: u16, to: u16) -> Missing<Route<'_>> {
        match self.routes.get(usize::from(mode)) {
            Some(r) => r.route(from, to),
            None => phx_num::violation!(clause = "FRT.2", "a route of a mode the network has not", mode = mode),
        }
    }

    /// A segment closed until a day, its mode's routes computed again without it; the segment rows visited.
    #[clause("GEO.8", "FRT.7")]
    pub fn close(&mut self, id: SegmentId, until: Day, today: Day) -> u64 {
        self.segments.close(id, until);
        self.recompute(self.segments.row(id).mode(), today)
    }

    /// A closed segment opened again, its mode's routes computed again with it; the segment rows visited.
    pub fn reopen(&mut self, id: SegmentId, today: Day) -> u64 {
        self.segments.reopen(id);
        self.recompute(self.segments.row(id).mode(), today)
    }

    /// A segment opened, its mode's routes computed again with it.
    pub fn open(&mut self, d: SegmentDecl, today: Day) -> SegmentId {
        let id = self.segments.open(d);
        let row = self.segments.row(id);
        if let Some(r) = self.routes.get_mut(usize::from(d.mode)) {
            r.admit(id, row);
        }
        let _ = self.recompute(d.mode, today);
        id
    }

    /// A segment removed, its mode's routes computed again without it; the segment rows visited.
    pub fn remove(&mut self, id: SegmentId, today: Day) -> u64 {
        self.segments.remove(id);
        self.recompute(self.segments.row(id).mode(), today)
    }
}

impl StoreStats for Transport {
    fn rows_live(&self) -> u64 {
        u64::try_from(self.segments.len()).unwrap_or(u64::MAX)
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        let bytes = self.segments.bytes() + self.routes.iter().map(Routes::bytes).sum::<usize>();
        u64::try_from(bytes).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "transport_tests.rs"]
mod tests;
