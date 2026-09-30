//! Marks read back as the day's set, in slot order, with no clearing pass between days.
#![cfg(test)]

use phx_id::Slot;

use super::{DayStamps, EPOCH_CHUNK_ROWS, EpochBits};
use crate::backing::{AddressSpace, HeapBacking};

type Heap = HeapBacking<4096>;

fn bits(rows: u32) -> EpochBits<Heap> {
    EpochBits::new(&mut AddressSpace::empty(), rows)
}

fn marked(e: &EpochBits<Heap>, today: u32) -> Vec<u32> {
    let mut out = Vec::new();
    e.for_each_marked(today, |s| out.push(s.get()));
    out
}

#[test]
fn mark_then_iterate() {
    let mut e = bits(20_000);
    for s in [17, 5, 4_100, 64, 19_999] {
        e.mark(Slot::new(s), 7);
    }
    assert_eq!(marked(&e, 7), [5, 17, 64, 4_100, 19_999]);
    assert!(e.is_marked(Slot::new(64), 7));
    assert!(!e.is_marked(Slot::new(65), 7));
}

#[test]
fn lazy_clear_on_new_day() {
    let mut e = bits(10_000);
    e.mark(Slot::new(3), 1);
    e.mark(Slot::new(9_000), 1);
    e.mark(Slot::new(4), 2);
    assert_eq!(marked(&e, 2), [4], "yesterday's marks are gone without a pass");
    assert!(!e.is_marked(Slot::new(9_000), 2));
}

#[test]
fn double_mark_is_one() {
    let mut e = bits(100);
    e.mark(Slot::new(8), 3);
    e.mark(Slot::new(8), 3);
    assert_eq!(marked(&e, 3), [8]);
}

#[test]
fn dense_day_iterates_every_word_once() {
    let mut e = bits(3 * EPOCH_CHUNK_ROWS);
    for s in 0..3 * EPOCH_CHUNK_ROWS {
        e.mark(Slot::new(s), 4);
    }
    let all = marked(&e, 4);
    assert_eq!(all.len(), 3 * 4_096);
    assert!(all.windows(2).all(|w| w[0] + 1 == w[1]), "every row once, in slot order");
}

#[test]
fn iteration_same_for_any_workers() {
    let rows = 5 * EPOCH_CHUNK_ROWS;
    let marks: Vec<u32> = (0..3_000_u32).map(|i| i * 6_571 % rows).collect();
    let mut one = bits(rows);
    for m in &marks {
        one.mark(Slot::new(*m), 9);
    }
    // Each chunk's writer marks its own rows, whichever worker it is and in whatever order the chunks run.
    let mut split = bits(rows);
    let mut chunks: Vec<_> = split.chunks_mut().collect();
    chunks.reverse();
    for chunk in &mut chunks {
        for m in marks.iter().rev() {
            if m / EPOCH_CHUNK_ROWS == chunk.first() / EPOCH_CHUNK_ROWS {
                chunk.mark(Slot::new(*m), 9);
            }
        }
    }
    drop(chunks);
    assert_eq!(marked(&split, 9), marked(&one, 9));
}

#[test]
fn stamp_is_today_test() {
    let mut s: DayStamps<Heap> = DayStamps::new(&mut AddressSpace::empty(), 10);
    assert!(!s.is_today(Slot::new(3), 0), "a row never stamped is no day's");
    s.stamp(Slot::new(3), 5);
    assert!(s.is_today(Slot::new(3), 5));
    assert!(!s.is_today(Slot::new(3), 6), "tomorrow it is not, with no reset");
}

#[test]
fn load_starts_clean() {
    // Nothing of the bits is saved: a load makes them new, and its first day reads only its own marks.
    let mut fresh = bits(1_000);
    assert!(marked(&fresh, 12).is_empty());
    fresh.mark(Slot::new(2), 12);
    assert_eq!(marked(&fresh, 12), [2]);
}

#[test]
fn opening_marks_read_by_first_audit() {
    let mut e = bits(1_000);
    let opening = 0;
    for s in [1, 500, 999] {
        e.mark(Slot::new(s), opening);
    }
    assert_eq!(marked(&e, opening), [1, 500, 999], "the opening's writes, on day zero, are the first audit's set");
}

#[test]
fn words_carry_the_rows_marked() {
    let mut e = bits(10_000);
    for s in [3, 5, 64, 9_000] {
        e.mark(Slot::new(s), 1);
    }
    let mut words = Vec::new();
    e.for_each_marked_word(1, |first, w| words.push((first.get(), w)));
    assert_eq!(words, [(0, (1 << 3) | (1 << 5)), (64, 1), (8_960, 1 << 40)]);
}
