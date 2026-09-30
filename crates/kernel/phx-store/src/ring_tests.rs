//! Dated rows read back by day and by subject while their horizon keeps them, and dropped whole chunks at a time once
//! it has passed.
#![cfg(test)]

use phx_num::Missing;

use super::HorizonRing;
use crate::backing::{AddressSpace, HeapBacking};
use crate::daybuf::DayBuf;
use crate::index::Index;
use crate::index_decl::{IndexDecl, Keys, Mode};
use crate::roundtrip::roundtrip;
use crate::stats::StoreStats;

type Heap = HeapBacking<4096>;

/// A row: its subject and a value.
type Rec = [u64; 2];

fn ring(chunk_rows: u32, max_chunks: u32, horizon: u32) -> HorizonRing<Rec, Heap> {
    HorizonRing::new(&mut AddressSpace::empty(), (chunk_rows, max_chunks), horizon).unwrap()
}

fn in_range(r: &HorizonRing<Rec, Heap>, days: (u32, u32)) -> Vec<(u64, u32, Rec)> {
    let mut out = Vec::new();
    r.range(days, |first, d, rows| out.extend((first..).zip(d.iter().zip(rows)).map(|(k, (d, row))| (k, *d, *row))));
    out
}

fn subject(row: &Rec) -> u64 {
    row[0]
}

fn subjects() -> Index<Heap> {
    let decl = IndexDecl {
        name: "records by subject",
        member_table: "records",
        key: "subject",
        predicate: Missing::Absent,
        mode: Mode::Lazy,
        keys: Keys::Dense(8),
        entries: 4_096,
        clause: "SET.13",
    };
    Index::new(&mut AddressSpace::empty(), decl).unwrap()
}

fn of(r: &HorizonRing<Rec, Heap>, ix: &Index<Heap>, s: u64) -> (Vec<u32>, u32) {
    let mut out = DayBuf::new(&mut AddressSpace::empty(), "test.walk", 4_096);
    let dropped = r.of_subject(ix, s, subject, &mut out);
    (out.as_slice().to_vec(), dropped)
}

#[test]
fn append_then_range() {
    let mut r = ring(4, 16, 30);
    assert_eq!(r.append(1, &[[1, 10], [2, 11]]), 0);
    assert_eq!(r.append(1, &[[3, 12]]), 2);
    assert_eq!(r.append(3, &[[1, 13], [2, 14]]), 3, "a day that fits no longer begins a chunk");
    assert_eq!(r.append(5, &[[4, 15]]), 5);
    assert_eq!(r.chunks_live(), 2, "day 5's row fits day 3's chunk");
    assert_eq!(in_range(&r, (2, 4)), vec![(3, 3, [1, 13]), (4, 3, [2, 14])]);
    assert_eq!(in_range(&r, (1, 5)).len(), 6);
    assert_eq!(in_range(&r, (6, 9)), vec![]);
    assert_eq!(r.row(2), Missing::Present((1, [3, 12])));
    assert_eq!(r.row(6), Missing::Absent);
}

#[test]
fn append_earlier_day_refused() {
    let mut r = ring(4, 4, 30);
    let _ = r.append(5, &[[1, 1]]);
    let _ = r.append(5, &[[1, 2]]);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.append(4, &[[1, 3]])));
    assert!(caught.is_err(), "a day before the last appended stops the run");
}

#[test]
fn prune_drops_whole_chunks() {
    let mut r = ring(4, 12, 10);
    for day in 0..6 {
        let _ = r.append(day, &[[1, u64::from(day)], [2, u64::from(day)], [3, u64::from(day)]]);
    }
    // Days 0..=5, three rows a day, a chunk a day.
    assert_eq!(r.prune(10), 0, "day zero is within ten days of day ten");
    assert_eq!(r.prune(12), 6, "days 0 and 1 end before day 2");
    assert_eq!(r.ordinals(), (6, 18));
    assert_eq!(r.row(5), Missing::Absent);
    assert_eq!(r.row(6), Missing::Present((2, [1, 2])));
    // The pruned chunks are taken again, so the ring never needs more than it keeps.
    for day in 6..30 {
        let _ = r.append(day, &[[1, 0]; 3]);
        let _ = r.prune(day);
    }
    assert_eq!(r.chunks_live(), 11);
}

#[test]
fn rows_bounded_by_horizon() {
    let horizon = 30;
    let mut r = ring(64, 64, horizon);
    for day in 0..730 {
        let _ = r.append(day, &[[1, 0]; 17]);
        if day % 10 == 0 {
            let _ = r.prune(day);
        }
    }
    let _ = r.prune(729);
    let (base, next) = r.ordinals();
    let kept_days = u64::from(horizon) + 1;
    assert!(next - base >= kept_days * 17, "every row within the horizon kept");
    assert!(next - base <= kept_days * 17 + 64, "no more than a chunk's rows past it: {}", next - base);
    assert_eq!(r.rows_live(), next - base);
}

#[test]
fn floor_raises_horizon_from_its_day() {
    let mut r = ring(2, 64, 5);
    r.set_floor(20, 15);
    assert_eq!(r.horizon_on(19), 5);
    assert_eq!(r.horizon_on(20), 15);
    r.set_floor(40, 3);
    assert_eq!(r.horizon_on(40), 5, "a floor below the ring's own horizon leaves it");
    for day in 0..20 {
        let _ = r.append(day, &[[1, 0]]);
    }
    let _ = r.prune(18);
    assert_eq!(r.ordinals().0, 12, "days 0..=11 lie before day 18 less 5");
    let _ = r.prune(24);
    assert_eq!(r.ordinals().0, 12, "from day 20 the floor keeps fifteen days");
}

#[test]
fn day_zero_holds_opening_values() {
    let mut r = ring(4, 4, 365);
    let _ = r.append(0, &[[1, 100], [2, 200]]);
    assert_eq!(in_range(&r, (0, 0)), vec![(0, 0, [1, 100]), (1, 0, [2, 200])]);
    assert_eq!(r.prune(0), 0);
    assert_eq!(r.prune(365), 0, "the opening's values are kept their horizon");
    assert_eq!(r.prune(366), 2);
}

#[test]
fn subject_index_skips_pruned() {
    let mut r = ring(2, 16, 3);
    let mut ix = subjects();
    for day in 0..10 {
        let first = r.append(day, &[[u64::from(day % 2), u64::from(day)]]);
        assert_eq!(r.index_since(first, &mut ix, subject), 1);
    }
    let _ = r.prune(9);
    assert_eq!(r.ordinals().0, 6, "days 0..=5 lie before day 9 less 3, in chunks of two days");
    assert_eq!(of(&r, &ix, 0), (vec![6, 8], 3), "a pruned entry is dropped by the walk");
    r.compact_subject(&mut ix, 0, subject, &mut DayBuf::new(&mut AddressSpace::empty(), "test.scratch", 4_096));
    assert_eq!(of(&r, &ix, 0), (vec![6, 8], 0), "a compacted key holds its live rows alone");
    assert_eq!(of(&r, &ix, 1), (vec![7, 9], 3));
}

#[test]
fn large_day_spans_chunks() {
    let mut r = ring(8, 16, 30);
    let _ = r.append(1, &[[1, 0]; 3]);
    let rows: Vec<Rec> = (0..30).map(|i| [2, i]).collect();
    assert_eq!(r.append(2, &rows), 3);
    assert_eq!(r.chunks_live(), 5, "a day of more rows than a chunk fills the last chunk's room, then whole chunks");
    let read = in_range(&r, (2, 2));
    assert_eq!(read.len(), 30);
    assert!(read.iter().zip(3_u64..).all(|((k, _, row), want)| *k == want && row[1] == want - 3));
    assert_eq!(r.prune(32), 0, "day 1's rows are kept with day 2's, in the chunk they share");
    assert_eq!(r.prune(33), 33);
}

/// An owner's ring and its subject index, rebuilt from the ring's live rows after a load.
#[derive(Debug, phx_macros::Saved)]
struct Records {
    ring: HorizonRing<Rec, Heap>,
    #[saved(skip, rebuild = Records::index_subjects)]
    by_subject: Option<Index<Heap>>,
}

impl Records {
    fn index_subjects(&mut self) -> u64 {
        let mut ix = subjects();
        let rows = self.ring.index_since(self.ring.ordinals().0, &mut ix, subject);
        self.by_subject = Some(ix);
        rows
    }

    fn walks(&self) -> Vec<Vec<u32>> {
        (0..8).map(|s| self.by_subject.as_ref().map_or_else(Vec::new, |ix| of(&self.ring, ix, s).0)).collect()
    }
}

impl PartialEq for Records {
    /// Two owners are equal when their rings are and every subject's walk reads the same rows.
    fn eq(&self, other: &Records) -> bool {
        self.ring == other.ring && self.walks() == other.walks()
    }
}

#[test]
fn rebuild_equals_incremental() {
    let mut kept = Records { ring: ring(16, 32, 40), by_subject: Some(subjects()) };
    for day in 0..200_u32 {
        let rows: Vec<Rec> = (0..day % 7).map(|i| [u64::from((day + i) % 8), u64::from(day)]).collect();
        let first = kept.ring.append(day, &rows);
        let _ = kept.ring.index_since(first, kept.by_subject.as_mut().unwrap(), subject);
        if day % 10 == 0 {
            let _ = kept.ring.prune(day);
        }
    }
    kept.ring.set_floor(150, 60);
    let live = kept.ring.rows_live();
    let (back, rebuilt) = roundtrip(&kept).unwrap();
    assert_eq!(rebuilt.stores(), [("Records.by_subject", live)]);
    assert_eq!(back.ring.ordinals(), kept.ring.ordinals());
    assert_eq!(in_range(&back.ring, (0, 200)), in_range(&kept.ring, (0, 200)));
}
