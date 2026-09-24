#![expect(unsafe_code, reason = "a region hands out typed views of the committed part of its reservation")]

use std::marker::PhantomData;

use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::pod::Pod;

/// Typed elements in reserved address space: pages are committed as the region grows, and the elements never move.
#[derive(Debug)]
pub struct Region<T: Pod, B: Backing = SystemBacking> {
    backing: B,
    capacity: usize,
    committed: usize,
    marker: PhantomData<T>,
}

impl<T: Pod, B: Backing> Region<T, B> {
    const SIZE: usize = {
        assert!(size_of::<T>() > 0, "a stored type has a size");
        size_of::<T>()
    };

    pub fn reserve(space: &mut AddressSpace, capacity: usize) -> Region<T, B> {
        let Some(bytes) = capacity.checked_mul(Self::SIZE) else {
            capacity_exceeded!("region elements", usize::MAX / Self::SIZE, capacity);
        };
        let backing: B = space.reserve(bytes);
        if align_of::<T>() > backing.page() {
            violation!(clause = "SET.12", "a stored type aligned wider than a page", align = align_of::<T>());
        }
        Region { backing, capacity, committed: 0, marker: PhantomData }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.committed
    }

    /// Elements readable without committing more.
    #[must_use]
    pub fn committed_len(&self) -> usize {
        self.committed / Self::SIZE
    }

    fn page_end(&self, n: usize) -> usize {
        let page = self.backing.page();
        (n * Self::SIZE).div_ceil(page) * page
    }

    /// Commits pages until `n` elements are writable; more than the reservation holds stops the run.
    pub fn ensure(&mut self, n: usize) {
        if n > self.capacity {
            capacity_exceeded!("region elements", self.capacity, n);
        }
        if n <= self.committed_len() {
            return;
        }
        let end = self.page_end(n);
        self.backing.commit(self.committed..end);
        self.committed = end;
    }

    /// Returns to the system every page wholly above the first `n` elements; they read as zero if committed again.
    pub fn release_above(&mut self, n: usize) {
        let keep = self.page_end(n);
        if keep < self.committed {
            self.backing.decommit(keep..self.committed);
            self.committed = keep;
        }
    }

    fn check(&self, n: usize) {
        if n > self.committed_len() {
            violation!(clause = "SET.12", "a view past a region's committed pages", n = n, committed = self.committed);
        }
    }

    /// The first `n` elements, which must be committed.
    #[must_use]
    pub fn slice(&self, n: usize) -> &[T] {
        self.check(n);
        // SAFETY: the first `n` elements are committed, so valid for reads; the base is page-aligned, which covers
        // `T`'s alignment (checked at reservation); committed bytes are zero or written as `T`, and `T` accepts
        // every bit pattern.
        unsafe { std::slice::from_raw_parts(self.backing.base().as_ptr().cast::<T>(), n) }
    }

    /// The first `n` elements, mutably; they must be committed.
    pub fn slice_mut(&mut self, n: usize) -> &mut [T] {
        self.check(n);
        // SAFETY: as for `slice`; `&mut self` makes this the only view of the region while it lives.
        unsafe { std::slice::from_raw_parts_mut(self.backing.base().as_ptr().cast::<T>(), n) }
    }
}

impl<T: Pod, B: Backing> Region<T, B> {
    /// The region's capacity and its first `n` elements, for a save; the rest of its reservation is empty.
    pub(crate) fn save_prefix(&self, n: usize, w: &mut crate::save::Writer<'_>, transform: crate::Transform) {
        w.count(self.capacity);
        w.count(n);
        if n > 0 {
            w.rows(self.slice(n), transform);
        }
    }

    /// A region read back as `save_prefix` wrote it, reserved again in the reader's space: its capacity and
    /// its first elements.
    pub(crate) fn load_prefix(
        r: &mut crate::save::Reader<'_>,
        transform: crate::Transform,
    ) -> Result<(Region<T, B>, usize), crate::save::LoadError> {
        let cap = crate::save::capacity::<T>(r)?;
        let n = r.count()?;
        crate::save::within(n, cap, "a region's elements")?;
        let mut region = Region::reserve(r.space(), cap);
        if n > 0 {
            region.ensure(n);
            r.rows_into(transform, region.slice_mut(n))?;
        }
        Ok((region, n))
    }
}

#[cfg(test)]
mod tests {
    use super::Region;
    use crate::backing::{AddressSpace, HeapBacking};

    #[test]
    fn region_commits_by_page_and_releases_to_zero() {
        let mut space = AddressSpace::empty();
        let mut r: Region<u64, HeapBacking<4096>> = Region::reserve(&mut space, 4096);
        assert_eq!(r.committed_len(), 0);
        r.ensure(10);
        assert_eq!((r.committed_len(), r.bytes_committed()), (512, 4096));
        r.slice_mut(512).iter_mut().for_each(|x| *x = 7);
        r.ensure(1000);
        r.release_above(100);
        assert_eq!(r.committed_len(), 512);
        r.ensure(1000);
        assert_eq!((r.slice(1000)[511], r.slice(1000)[512]), (7, 0));
    }
}
