#![expect(unsafe_code, reason = "reserving, committing and releasing address space are system calls")]

use std::alloc::{self, Layout};
use std::ops::Range;
use std::ptr::NonNull;

use phx_num::violation::Key;
use phx_num::{capacity_exceeded, violation};

use crate::consts::{HEAP_PAGE, VA_BUDGET};

/// A reservation of address space whose pages are committed and returned as a store grows and shrinks.
///
/// # Safety
/// `base` never changes and is aligned to `page`; every committed byte is valid for reads and writes through it; a
/// byte reads as zero when first committed and when committed again after a decommit.
pub unsafe trait Backing: Send + Sync {
    /// Reserves at least `bytes`, rounded up to whole pages, none of them committed.
    fn reserve(bytes: usize) -> Self
    where
        Self: Sized;
    fn page(&self) -> usize;
    fn reserved(&self) -> usize;
    fn base(&self) -> NonNull<u8>;
    /// Makes whole pages readable and writable.
    fn commit(&mut self, range: Range<usize>);
    /// Returns whole pages to the system.
    fn decommit(&mut self, range: Range<usize>);
}

/// The backing the world uses: the system's pages, or the heap where there are none to map, as under Miri.
#[cfg(all(unix, not(miri)))]
pub type SystemBacking = MmapBacking;
#[cfg(any(miri, not(unix)))]
pub type SystemBacking = HeapBacking;

/// The address space every store reserves, held within its budget; reservations last the world's life.
#[derive(Debug)]
pub struct AddressSpace {
    reserved: usize,
}

impl AddressSpace {
    #[must_use]
    pub fn empty() -> AddressSpace {
        AddressSpace { reserved: 0 }
    }

    pub fn reserve<B: Backing>(&mut self, bytes: usize) -> B {
        let backing = B::reserve(bytes);
        let Some(total) = self.reserved.checked_add(backing.reserved()).filter(|t| *t <= VA_BUDGET) else {
            capacity_exceeded!(
                "reserved address space",
                VA_BUDGET,
                Key::key(self.reserved) + Key::key(backing.reserved())
            );
        };
        self.reserved = total;
        backing
    }

    #[must_use]
    pub fn reserved(&self) -> usize {
        self.reserved
    }
}

fn whole_pages(bytes: usize, page: usize) -> usize {
    let Some(rounded) = bytes.div_ceil(page).checked_mul(page) else {
        capacity_exceeded!("reserved address space", VA_BUDGET, bytes);
    };
    if rounded == 0 { page } else { rounded }
}

fn check_range(range: &Range<usize>, page: usize, reserved: usize) {
    let aligned = range.start.is_multiple_of(page) && range.end.is_multiple_of(page);
    if !aligned || range.start > range.end || range.end > reserved {
        violation!(
            clause = "SET.12",
            "a commit or decommit outside whole reserved pages",
            start = range.start,
            end = range.end,
            reserved = reserved
        );
    }
}

/// Pages mapped from the system: reserved without access or swap, committed by granting access, decommitted by
/// discarding their contents.
#[derive(Debug)]
pub struct MmapBacking {
    base: NonNull<u8>,
    len: usize,
    page: usize,
}

// SAFETY: the mapping is owned by this value alone and freed only when it drops; shared access is read-only
// through the typed views that borrow it.
unsafe impl Send for MmapBacking {}
// SAFETY: as above; `&MmapBacking` exposes no mutation.
unsafe impl Sync for MmapBacking {}

#[cfg(unix)]
fn system_page() -> usize {
    // SAFETY: sysconf reads a constant of the system and has no preconditions.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    let Some(page) = usize::try_from(page).ok().filter(|p| p.is_power_of_two()) else {
        violation!(clause = "SET.12", "the system reports no page size", page = page);
    };
    page
}

#[cfg(unix)]
// SAFETY: the mapping is private and anonymous, owned by the returned value; commit and decommit keep to whole
// reserved pages, and anonymous pages read as zero when first touched and after MADV_DONTNEED.
unsafe impl Backing for MmapBacking {
    fn reserve(bytes: usize) -> MmapBacking {
        let page = system_page();
        let len = whole_pages(bytes, page);
        let flags = libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE;
        // SAFETY: a fresh anonymous mapping with no address hint touches no existing memory.
        let ptr = unsafe { libc::mmap(std::ptr::null_mut(), len, libc::PROT_NONE, flags, -1, 0) };
        let Some(base) = NonNull::new(ptr.cast::<u8>()).filter(|_| ptr != libc::MAP_FAILED) else {
            capacity_exceeded!("reserved address space", VA_BUDGET, len);
        };
        MmapBacking { base, len, page }
    }

    fn page(&self) -> usize {
        self.page
    }

    fn reserved(&self) -> usize {
        self.len
    }

    fn base(&self) -> NonNull<u8> {
        self.base
    }

    fn commit(&mut self, range: Range<usize>) {
        check_range(&range, self.page, self.len);
        let len = range.end - range.start;
        // SAFETY: the range lies within this mapping, checked above.
        let status = unsafe {
            libc::mprotect(self.base.as_ptr().add(range.start).cast(), len, libc::PROT_READ | libc::PROT_WRITE)
        };
        if status != 0 {
            capacity_exceeded!("committed memory", self.len, range.end);
        }
    }

    fn decommit(&mut self, range: Range<usize>) {
        check_range(&range, self.page, self.len);
        let len = range.end - range.start;
        // SAFETY: the range lies within this mapping and no view of it outlives the `&mut self` of this call.
        let status = unsafe {
            let start = self.base.as_ptr().add(range.start).cast();
            libc::madvise(start, len, libc::MADV_DONTNEED) | libc::mprotect(start, len, libc::PROT_NONE)
        };
        if status != 0 {
            violation!(clause = "SET.12", "the system refused to release committed pages", start = range.start);
        }
    }
}

impl Drop for MmapBacking {
    fn drop(&mut self) {
        // SAFETY: the mapping was created by `reserve` with this base and length and is unmapped once, here.
        unsafe {
            libc::munmap(self.base.as_ptr().cast(), self.len);
        }
    }
}

/// A heap allocation standing in for mapped pages, where there are none to map (Miri, tests of other page sizes):
/// everything is allocated zeroed at once, and a decommit zeroes the range.
#[derive(Debug)]
pub struct HeapBacking<const PAGE: usize = HEAP_PAGE> {
    base: NonNull<u8>,
    layout: Layout,
}

// SAFETY: the allocation is owned by this value alone and freed only when it drops.
unsafe impl<const PAGE: usize> Send for HeapBacking<PAGE> {}
// SAFETY: as above; `&HeapBacking` exposes no mutation.
unsafe impl<const PAGE: usize> Sync for HeapBacking<PAGE> {}

// SAFETY: the allocation is zeroed, aligned to the page, owned by the returned value, and zeroed again on decommit.
unsafe impl<const PAGE: usize> Backing for HeapBacking<PAGE> {
    fn reserve(bytes: usize) -> HeapBacking<PAGE> {
        let len = whole_pages(bytes, PAGE);
        let Ok(layout) = Layout::from_size_align(len, PAGE) else {
            capacity_exceeded!("reserved address space", VA_BUDGET, len);
        };
        // SAFETY: the layout has a non-zero size.
        let ptr = unsafe { alloc::alloc_zeroed(layout) };
        let Some(base) = NonNull::new(ptr) else {
            capacity_exceeded!("committed memory", VA_BUDGET, len);
        };
        HeapBacking { base, layout }
    }

    fn page(&self) -> usize {
        PAGE
    }

    fn reserved(&self) -> usize {
        self.layout.size()
    }

    fn base(&self) -> NonNull<u8> {
        self.base
    }

    fn commit(&mut self, range: Range<usize>) {
        check_range(&range, PAGE, self.layout.size());
    }

    fn decommit(&mut self, range: Range<usize>) {
        check_range(&range, PAGE, self.layout.size());
        // SAFETY: the range lies within the allocation, checked above, and no view outlives `&mut self`.
        unsafe { self.base.as_ptr().add(range.start).write_bytes(0, range.end - range.start) };
    }
}

impl<const PAGE: usize> Drop for HeapBacking<PAGE> {
    fn drop(&mut self) {
        // SAFETY: allocated by `reserve` with this layout and freed once, here.
        unsafe { alloc::dealloc(self.base.as_ptr(), self.layout) };
    }
}

#[cfg(test)]
mod tests {
    use super::{AddressSpace, Backing, HeapBacking};

    fn write_read<B: Backing>(b: &mut B, at: usize, v: u8) -> u8 {
        let p = b.base().as_ptr();
        // SAFETY: the caller has committed the page holding `at`.
        unsafe {
            p.add(at).write(v);
            p.add(at).read()
        }
    }

    fn read<B: Backing>(b: &B, at: usize) -> u8 {
        // SAFETY: the caller has committed the page holding `at`.
        unsafe { b.base().as_ptr().add(at).read() }
    }

    fn commit_decommit<B: Backing>(b: &mut B) {
        let page = b.page();
        assert!(b.reserved() >= page * 4);
        b.commit(0..page * 2);
        assert_eq!(write_read(b, page + 3, 7), 7);
        b.decommit(page..page * 2);
        b.commit(page..page * 2);
        assert_eq!(read(b, page + 3), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri cannot call mmap")]
    fn mmap_backing_commit_decommit() {
        let mut space = AddressSpace::empty();
        let mut b: super::MmapBacking = space.reserve(1 << 20);
        commit_decommit(&mut b);
        assert_eq!(space.reserved(), b.reserved());
    }

    #[test]
    fn heap_backing_both_page_sizes() {
        let mut space = AddressSpace::empty();
        let mut small: HeapBacking<4096> = space.reserve(1 << 15);
        let mut large: HeapBacking<16384> = space.reserve(1 << 16);
        commit_decommit(&mut small);
        commit_decommit(&mut large);
        assert_eq!(space.reserved(), (1 << 15) + (1 << 16));
    }
}
