//! A day's buffers keep their pages across days, grow in whole pages, stop the run past their capacity, and join in
//! (chunk, handler) order whoever filled them.
#![cfg(test)]

use super::{DayBuf, DayBufs};
use crate::backing::{AddressSpace, HeapBacking};

const PAGE: usize = 4096;
type Heap = HeapBacking<PAGE>;

fn buf(capacity: usize) -> DayBuf<u64, Heap> {
    DayBuf::new(&mut AddressSpace::empty(), "test.buffer", capacity)
}

#[test]
fn clear_keeps_pages() {
    let mut b = buf(10_000);
    b.extend(&[7; 1000]);
    let pages = b.bytes_committed();
    b.clear();
    assert_eq!((b.len(), b.bytes_committed()), (0, pages));
    b.push(3);
    assert_eq!(b.as_slice(), [3]);
}

#[test]
fn growth_in_whole_pages() {
    let mut b = buf(10_000);
    b.push(1);
    assert_eq!(b.bytes_committed(), PAGE);
    b.extend(&[1; 600]);
    assert_eq!(b.bytes_committed() % PAGE, 0);
    assert!(b.bytes_committed() >= 601 * size_of::<u64>());
}

#[test]
fn second_heavy_day_maps_no_page() {
    let mut b = buf(100_000);
    for (day, kind, n) in [(0, 2, 50_000), (1, 0, 9_000), (2, 2, 50_000)] {
        b.clear();
        let before = b.bytes_committed();
        for i in 0..n {
            b.push(i);
        }
        b.mark(kind);
        if day == 2 {
            assert_eq!(b.bytes_committed(), before, "the second heavy day commits no page");
        }
    }
    assert_eq!(b.highs(), [9_000, 0, 50_000, 0]);
}

#[test]
fn push_past_capacity_stops() {
    let mut b = buf(3);
    b.extend(&[1, 2, 3]);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.push(4)));
    let payload = caught.expect_err("a push past capacity stops the run");
    let what = payload.downcast_ref::<phx_num::violation::CapacityExceeded>().map(|c| c.what);
    assert_eq!(what, Some("test.buffer"), "the stop names the buffer");
}

/// A worker's chunks, each with its handlers' buffers.
type Held<'a> = Vec<(usize, &'a mut [DayBuf<u64, Heap>])>;

/// Each chunk's handlers write their items as a worker holding the chunk would, whichever worker it is.
fn filled(chunks: usize, handlers: usize, workers: usize) -> Vec<u64> {
    let mut bufs: DayBufs<u64, Heap> = DayBufs::new(&mut AddressSpace::empty(), "test.intents", (chunks, handlers), 64);
    let mut by_worker: Vec<Held<'_>> = (0..workers).map(|_| Vec::new()).collect();
    for (chunk, row) in bufs.chunks_mut().enumerate() {
        if let Some(w) = by_worker.get_mut((chunk * 7 + 3) % workers) {
            w.push((chunk, row));
        }
    }
    for held in by_worker.iter_mut().rev() {
        for (chunk, row) in held.iter_mut() {
            for (handler, b) in row.iter_mut().enumerate() {
                for i in 0..(*chunk + handler) % 4 {
                    b.push(u64::try_from(*chunk * 100 + handler * 10 + i).unwrap());
                }
            }
        }
    }
    let mut out: DayBuf<u64, Heap> = DayBuf::new(&mut AddressSpace::empty(), "test.joined", 10_000);
    bufs.join(&mut out);
    out.as_slice().to_vec()
}

#[test]
fn join_same_for_any_workers() {
    let one = filled(9, 3, 1);
    for workers in 2..=8 {
        assert_eq!(filled(9, 3, workers), one);
    }
}

#[test]
fn join_order_is_chunk_handler() {
    let joined = filled(3, 2, 2);
    let mut sorted = joined.clone();
    sorted.sort_unstable();
    assert_eq!(joined, sorted, "chunk before handler before the handler's own order");
    assert_eq!(joined.first(), Some(&10), "chunk 0's first handler wrote nothing, its second one item");
}
