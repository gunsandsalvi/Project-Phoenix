#![expect(unsafe_code, reason = "a global allocator's interface is unsafe; each call is passed to the system's own")]
//! The bench's allocator: the system's, counting every allocation's call and bytes, so a span's allocations are read
//! beside its time. Only the bench's build installs it; the world's has no counter and no cost.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::Ordering;

use crate::trace;

/// The system allocator, counted; the bench's binary installs it as its global allocator.
#[derive(Debug, Default)]
pub struct CountingAlloc;

/// One allocation's call and bytes counted.
fn count(bytes: usize) {
    let c = trace::counted();
    c.alloc_calls.fetch_add(1, Ordering::Relaxed);
    c.alloc_bytes.fetch_add(u64::try_from(bytes).unwrap_or(u64::MAX), Ordering::Relaxed);
}

// SAFETY: every call is passed to the system allocator unchanged; the counters only add.
unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: the caller's layout, passed on as the contract of `alloc` requires.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: the pointer and layout the caller had from `alloc`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count(new_size);
        // SAFETY: as `dealloc` and `alloc`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Allocations counted so far, calls and bytes; nothing is counted unless the counting allocator is installed.
#[must_use]
pub fn allocated() -> (u64, u64) {
    let c = trace::counted();
    (c.alloc_calls.load(Ordering::Relaxed), c.alloc_bytes.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use std::alloc::{GlobalAlloc, Layout};

    use super::{CountingAlloc, allocated};

    #[test]
    fn alloc_counter_counts() {
        let layout = Layout::new::<u64>();
        let (calls, bytes) = allocated();
        // SAFETY: each block is allocated with `layout` and freed once with the same layout.
        let blocks: Vec<*mut u8> = (0..100).map(|_| unsafe { CountingAlloc.alloc(layout) }).collect();
        let (after_calls, after_bytes) = allocated();
        for b in blocks {
            // SAFETY: as above.
            unsafe { CountingAlloc.dealloc(b, layout) };
        }
        assert_eq!((after_calls - calls, after_bytes - bytes), (100, 800), "n boxes give n calls");
    }
}
