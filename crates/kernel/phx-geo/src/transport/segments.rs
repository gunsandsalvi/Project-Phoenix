//! The network's segments: one 32-byte row each, between two zones, of a mode, with its length, what it carries a day,
//! its condition, whether it is closed and until when, and the named unit it is held as. A segment opened or removed
//! is logged, so what is derived from the segments is brought up to date in the same apply.

use phx_id::Day;
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};

use crate::consts::{SEGMENT_CLOSED, SEGMENT_HELD, SEGMENT_REMOVED};

/// A segment as declared: its zones, its mode, its length in metres, what it carries a day in its mode's unit, its
/// condition class and the named unit it is held as, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SegmentDecl {
    pub from: u16,
    pub to: u16,
    pub mode: u8,
    pub metres: u32,
    pub capacity: u32,
    pub condition: u8,
    pub unit: Missing<u32>,
}

/// A segment's row; its last eight bytes are kept for the free-flow time a mode's declared speed gives it.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct SegmentRow {
    from: u16,
    to: u16,
    mode: u8,
    condition: u8,
    flags: u16,
    metres: u32,
    capacity: u32,
    closed_until: u32,
    unit: u32,
    reserved: [u32; 2],
}

const _: () = assert!(size_of::<SegmentRow>() == size_of::<[u128; 2]>(), "a segment row is thirty-two bytes");

impl SegmentRow {
    fn flag(self, bit: u16) -> bool {
        self.flags & bit != 0
    }

    #[must_use]
    pub fn ends(self) -> (u16, u16) {
        (self.from, self.to)
    }

    #[must_use]
    pub fn mode(self) -> u8 {
        self.mode
    }

    #[must_use]
    pub fn metres(self) -> u32 {
        self.metres
    }

    #[must_use]
    pub fn capacity(self) -> u32 {
        self.capacity
    }

    #[must_use]
    pub fn condition(self) -> u8 {
        self.condition
    }

    /// The named unit the segment is held as, its holder its owner.
    pub fn unit(self) -> Missing<u32> {
        if self.flag(SEGMENT_HELD) { Missing::Present(self.unit) } else { Missing::Absent }
    }

    /// Whether the segment carries on a day: not removed, and not closed or closed only until an earlier day.
    #[must_use]
    pub fn open_on(self, day: Day) -> bool {
        !self.flag(SEGMENT_REMOVED) && (!self.flag(SEGMENT_CLOSED) || day.get() >= self.closed_until)
    }
}

/// A segment's identity, its row's place; never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SegmentId(u32);

impl SegmentId {
    /// The segment a route's run names.
    #[must_use]
    pub const fn new(id: u32) -> SegmentId {
        SegmentId(id)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// What happened to a segment that what is derived from the segments' lines must follow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Opened,
    Removed,
}

/// Every segment, by identity, and the openings and removals not yet taken by what follows them.
#[clause("GEO.4")]
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Segments {
    rows: Vec<SegmentRow>,
    #[saved(skip, rebuild = Segments::unlogged)]
    changes: Vec<(SegmentId, Change)>,
}

fn at(id: SegmentId) -> usize {
    match usize::try_from(id.0) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("segments", usize::MAX, id.0),
    }
}

impl Segments {
    /// A segment opened, its opening logged.
    pub fn open(&mut self, d: SegmentDecl) -> SegmentId {
        let id = match u32::try_from(self.rows.len()) {
            Ok(i) => SegmentId(i),
            Err(_) => capacity_exceeded!("segments", u32::MAX, self.rows.len()),
        };
        let (unit, held) = match d.unit {
            Missing::Present(u) => (u, SEGMENT_HELD),
            Missing::Absent => (0, 0),
        };
        self.rows.push(SegmentRow {
            from: d.from,
            to: d.to,
            mode: d.mode,
            condition: d.condition,
            flags: held,
            metres: d.metres,
            capacity: d.capacity,
            closed_until: 0,
            unit,
            reserved: [0; 2],
        });
        self.changes.push((id, Change::Opened));
        id
    }

    /// A loaded network's log, empty: every change was taken in the apply that made it.
    fn unlogged(&mut self) -> u64 {
        self.changes.clear();
        0
    }

    fn row_mut(&mut self, id: SegmentId) -> &mut SegmentRow {
        match self.rows.get_mut(at(id)) {
            Some(r) => r,
            None => violation!(clause = "GEO.4", "a segment never opened", segment = id.0),
        }
    }

    /// A segment's row, removed or not.
    #[must_use]
    pub fn row(&self, id: SegmentId) -> SegmentRow {
        match self.rows.get(at(id)) {
            Some(r) => *r,
            None => violation!(clause = "GEO.4", "a segment never opened", segment = id.0),
        }
    }

    /// A segment removed, its row kept, its removal logged.
    pub fn remove(&mut self, id: SegmentId) {
        self.row_mut(id).flags |= SEGMENT_REMOVED;
        self.changes.push((id, Change::Removed));
    }

    /// A segment closed until a day, the first it carries again.
    #[clause("GEO.8", "FRT.7")]
    pub fn close(&mut self, id: SegmentId, until: Day) {
        let r = self.row_mut(id);
        r.flags |= SEGMENT_CLOSED;
        r.closed_until = until.get();
    }

    /// A closed segment opened again at once.
    pub fn reopen(&mut self, id: SegmentId) {
        let r = self.row_mut(id);
        r.flags &= !SEGMENT_CLOSED;
        r.closed_until = 0;
    }

    /// Every segment's identity and row, in identity order.
    pub fn iter(&self) -> impl Iterator<Item = (SegmentId, SegmentRow)> + '_ {
        (0_u32..).zip(&self.rows).map(|(i, r)| (SegmentId(i), *r))
    }

    /// The openings and removals since the last taken, handed to what follows them and forgotten.
    pub fn take_changes(&mut self, into: &mut Vec<(SegmentId, Change)>) {
        into.append(&mut self.changes);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Its rows held in no more room than they take, once the opening has made them.
    pub fn settle(&mut self) {
        self.rows.shrink_to_fit();
    }

    /// The bytes its rows hold.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.rows.capacity() * size_of::<SegmentRow>()
    }
}
