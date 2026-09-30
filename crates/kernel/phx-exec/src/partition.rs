#![expect(unsafe_code, reason = "each piece writes its disjoint positions of one output in parallel")]

//! A stable partition of many buffers' items into buckets: the day's flows grouped by the range of parties that owns
//! each, so each range's worker reads only its own. Pieces are fixed by the inputs' lengths, counted and scattered in
//! parallel into disjoint positions, so the result is the same whatever the workers.

use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use phx_num::violation;

use crate::consts::RADIX_CHUNK;
use crate::pool::{Pool, each, map};
use crate::unwind::Payload;

/// Two slices of one length, their items at each index handed out once, to whichever thread asks next: a dispatch's
/// chunks, each writing only the items at its own index. The cursor gives each index once, so each pair of
/// references it returns is the only one to its items.
pub(crate) struct Cursor<'a, A, B> {
    left: *mut A,
    right: *mut B,
    len: usize,
    taken: AtomicUsize,
    slices: PhantomData<(&'a mut [A], &'a mut [B])>,
}

// SAFETY: an item is reached only through its index, which the cursor hands to one thread; the items are `Send`.
unsafe impl<A: Send, B: Send> Sync for Cursor<'_, A, B> {}

impl<'a, A> Cursor<'a, A, ()> {
    /// A cursor over one slice, its other side holding nothing.
    pub(crate) fn one(items: &'a mut [A]) -> Cursor<'a, A, ()> {
        // Zero-sized items need no storage: a dangling, aligned pointer reaches every index.
        let right = std::ptr::NonNull::<()>::dangling().as_ptr();
        Cursor { left: items.as_mut_ptr(), right, len: items.len(), taken: AtomicUsize::new(0), slices: PhantomData }
    }
}

impl<'a, A, B> Cursor<'a, A, B> {
    /// A cursor over two slices of one length, borrowed mutably for as long as it lives.
    pub(crate) fn new(left: &'a mut [A], right: &'a mut [B]) -> Cursor<'a, A, B> {
        if left.len() != right.len() {
            violation!(clause = "TIME.6", "a cursor over slices of two lengths", left = left.len());
        }
        let len = left.len();
        Cursor {
            left: left.as_mut_ptr(),
            right: right.as_mut_ptr(),
            len,
            taken: AtomicUsize::new(0),
            slices: PhantomData,
        }
    }

    /// The next index not yet handed out and its items, or none once every index has been.
    pub(crate) fn next(&self) -> Option<(usize, &'a mut A, &'a mut B)> {
        let at = self.taken.fetch_add(1, Ordering::Relaxed);
        if at >= self.len {
            return None;
        }
        // SAFETY: `at` is below both slices' length and handed out once, so these are the only references to its
        // items, and the slices stay borrowed mutably for `'a`.
        Some(unsafe { (at, &mut *self.left.add(at), &mut *self.right.add(at)) })
    }
}

/// A dispatch's state its workers share: whether it has closed, the workers inside it, and the first panic any run
/// raised.
struct Latch {
    closed: AtomicBool,
    active: AtomicUsize,
    panic: AtomicPtr<Payload>,
}

impl Latch {
    /// Keeps the first panic raised; a later one is dropped, as the run stops with the first.
    fn hold(&self, payload: Payload) {
        let boxed = crate::unwind::held(payload);
        if self.panic.compare_exchange(std::ptr::null_mut(), boxed, Ordering::AcqRel, Ordering::Acquire).is_err() {
            // SAFETY: `boxed` came from `Box::into_raw` just above and was never shared.
            drop(unsafe { Box::from_raw(boxed) });
        }
    }
}

/// The work a dispatch hands its workers: where it lies and the function that runs it, followed only while the
/// dispatch is open.
struct Work {
    data: *const (),
    call: unsafe fn(*const ()),
}

// SAFETY: the work is `Sync`, and a worker follows the pointer only between counting itself in and out of a dispatch
// that was open when it counted in, while its caller waits.
unsafe impl Send for Work {}
// SAFETY: as above.
unsafe impl Sync for Work {}

/// Runs the work `data` points to.
///
/// # Safety
/// `data` points to an `F` alive for the call.
unsafe fn call<F: Fn() + Sync>(data: *const ()) {
    // SAFETY: the caller's promise.
    unsafe { (*data.cast::<F>())() }
}

/// `each` run on the calling thread and on whichever workers of `threads` wake while there is work, returning once no
/// run is left inside it. The caller works beside the workers, closes the dispatch once `each` returns to it — the
/// work is then all taken — and spins for the runs still inside rather than sleeping on a lock, whose waking costs a
/// dispatch tens of microseconds; a worker that wakes after the close leaves without running. A panic in any run is
/// raised in the caller once no run is inside. Each worker runs `after` once out of the dispatch: the pool's spin.
pub(crate) fn run_everywhere<F: Fn() + Sync>(
    threads: &rayon_core::ThreadPool,
    each: &F,
    after: impl Fn() + Send + Sync + 'static,
) {
    let latch = Arc::new(Latch {
        closed: AtomicBool::new(false),
        active: AtomicUsize::new(0),
        panic: AtomicPtr::new(std::ptr::null_mut()),
    });
    // The work is kept as a pointer with no lifetime; the caller does not return, and so `each` stays borrowed, until
    // the dispatch has closed and no worker is inside it.
    let work = Work { data: std::ptr::from_ref(each).cast::<()>(), call: call::<F> };
    let theirs = Arc::clone(&latch);
    threads.spawn_broadcast(move |_| {
        let work = &work;
        theirs.active.fetch_add(1, Ordering::SeqCst);
        if !theirs.closed.load(Ordering::SeqCst) {
            // SAFETY: the dispatch was open after this worker counted itself in, so its caller is still waiting and
            // the work it points to is alive.
            let run = || unsafe { (work.call)(work.data) };
            if let Err(payload) = catch_unwind(AssertUnwindSafe(run)) {
                theirs.hold(payload);
            }
        }
        theirs.active.fetch_sub(1, Ordering::SeqCst);
        after();
    });
    let mine = catch_unwind(AssertUnwindSafe(each));
    latch.closed.store(true, Ordering::SeqCst);
    while latch.active.load(Ordering::SeqCst) > 0 {
        std::hint::spin_loop();
    }
    if let Err(payload) = mine {
        resume_unwind(payload);
    }
    let raised = latch.panic.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !raised.is_null() {
        // SAFETY: a non-null pointer on the latch came from `Box::into_raw` in `hold` and is taken once, here.
        resume_unwind(*unsafe { Box::from_raw(raised) });
    }
}

/// Items grouped by bucket, each bucket's items in input order, with where each bucket starts; kept across days so
/// a day partitions without allocating once the heaviest day has sized it.
#[derive(Debug)]
pub struct Partitioned<T> {
    pub items: Vec<T>,
    pub starts: Vec<usize>,
    /// Each piece's count in each bucket, then each piece's first and next place in each bucket, pieces × buckets.
    counts: Vec<usize>,
    places: Vec<usize>,
}

impl<T> Default for Partitioned<T> {
    fn default() -> Partitioned<T> {
        Partitioned { items: Vec::new(), starts: Vec::new(), counts: Vec::new(), places: Vec::new() }
    }
}

impl<T> Partitioned<T> {
    /// The bytes it holds: its items' room and the scatter's counts and places.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.items.capacity() * size_of::<T>() + self.scatter_bytes()
    }

    /// The bytes of the scatter's own counts, places and starts, beside the items it places.
    #[must_use]
    pub fn scatter_bytes(&self) -> usize {
        (self.starts.capacity() + self.counts.capacity() + self.places.capacity()) * size_of::<usize>()
    }

    /// The items of one bucket.
    #[must_use]
    pub fn bucket(&self, b: usize) -> &[T] {
        match (self.starts.get(b), self.starts.get(b + 1)) {
            (Some(from), Some(to)) => self.items.get(*from..*to).unwrap_or(&[]),
            _ => &[],
        }
    }
}

/// A pointer shared by the pieces, each writing only the positions its counts gave it.
struct Out<T>(*mut MaybeUninit<T>);

// SAFETY: the pieces write disjoint positions of one allocation that outlives the scatter.
unsafe impl<T: Send> Send for Out<T> {}
// SAFETY: as above; no position is read during the scatter.
unsafe impl<T: Send> Sync for Out<T> {}

/// A piece of the inputs the scatter counts and places as one: `len` items from `input`'s `offset`th on, running on
/// into the inputs after, so the scatter's counts follow the items and not how many inputs hold them.
#[derive(Clone, Copy, Debug)]
struct Piece {
    input: usize,
    offset: usize,
    len: usize,
}

impl Piece {
    /// The piece's items, in the inputs' order.
    fn items<'a, T>(self, inputs: &'a [&'a [T]]) -> impl Iterator<Item = &'a T> {
        let first = inputs.get(self.input).and_then(|i| i.get(self.offset..)).unwrap_or(&[]);
        let rest = inputs.get(self.input + 1..).unwrap_or(&[]);
        std::iter::once(first).chain(rest.iter().copied()).flatten().take(self.len)
    }
}

/// The inputs cut into pieces of `RADIX_CHUNK` items, the last the rest.
struct Pieces<'a, T> {
    inputs: &'a [&'a [T]],
    input: usize,
    offset: usize,
}

impl<T> Iterator for Pieces<'_, T> {
    type Item = Piece;

    fn next(&mut self) -> Option<Piece> {
        let (input, offset) = (self.input, self.offset);
        let mut len = 0;
        while len < RADIX_CHUNK {
            let Some(here) = self.inputs.get(self.input).map(|i| i.len() - self.offset) else { break };
            let take = if here < RADIX_CHUNK - len { here } else { RADIX_CHUNK - len };
            len += take;
            if take == here {
                (self.input, self.offset) = (self.input + 1, 0);
            } else {
                self.offset += take;
            }
        }
        (len > 0).then_some(Piece { input, offset, len })
    }
}

/// Partitions every input's items into `buckets` by `key`, stably: bucket by bucket, and within a bucket in the order
/// of the inputs and of their items.
pub fn partition_into<T: Copy + Send + Sync>(
    pool: Option<&Pool>,
    inputs: &[&[T]],
    buckets: usize,
    key: impl Fn(&T) -> usize + Sync,
    out: &mut Partitioned<T>,
) {
    partition_map_into(pool, inputs, buckets, key, |t| *t, out);
}

/// As `partition_into`, each item stored as what `map` makes of it: a narrower record where a bucket's reader needs
/// only part of the item.
pub fn partition_map_into<T: Sync, U: Copy + Send + Sync>(
    pool: Option<&Pool>,
    inputs: &[&[T]],
    buckets: usize,
    key: impl Fn(&T) -> usize + Sync,
    map_item: impl Fn(&T) -> U + Sync,
    out: &mut Partitioned<U>,
) {
    if buckets == 0 {
        violation!(clause = "TIME.6", "a partition into no buckets");
    }
    let pieces: Vec<Piece> = Pieces { inputs, input: 0, offset: 0 }.collect();
    let cells = pieces.len() * buckets;
    out.counts.clear();
    out.counts.resize(cells, 0);
    each(pool, out.counts.chunks_mut(buckets).zip(&pieces), |(c, piece)| {
        for item in piece.items(inputs) {
            let Some(n) = c.get_mut(key(item)) else {
                violation!(clause = "TIME.6", "an item keyed outside the buckets", buckets = buckets);
            };
            *n += 1;
        }
    });
    // Each piece's first place in each bucket: the bucket's start plus the earlier pieces' counts in it.
    out.starts.clear();
    out.starts.push(0);
    out.places.clear();
    out.places.resize(cells, 0);
    let mut at = 0_usize;
    for b in 0..buckets {
        for p in 0..pieces.len() {
            if let Some(o) = out.places.get_mut(p * buckets + b) {
                *o = at;
            }
            at += out.counts.get(p * buckets + b).copied().unwrap_or(0);
        }
        out.starts.push(at);
    }
    out.items.clear();
    out.items.reserve(at);
    let dst = Out(out.items.spare_capacity_mut().as_mut_ptr());
    let dst = &dst;
    let counts = &out.counts;
    // Each piece writes only below its own end in each bucket, which the counts give: a key that answers differently
    // the second time stops the run rather than writing another piece's places or past the reservation.
    let whole: Vec<bool> = map(pool, pieces.len(), |p| {
        let (Some(&piece), Some(first), Some(count)) =
            (pieces.get(p), out.places.get(p * buckets..(p + 1) * buckets), counts.get(p * buckets..(p + 1) * buckets))
        else {
            return false;
        };
        let mut next = first.to_vec();
        for item in piece.items(inputs) {
            let b = key(item);
            let (Some(pos), Some(start), Some(n)) = (next.get_mut(b), first.get(b), count.get(b)) else {
                violation!(clause = "TIME.6", "an item keyed outside the buckets", buckets = buckets);
            };
            if *pos >= start + n {
                violation!(clause = "TIME.6", "a partition's key answered differently the second time", bucket = b);
            }
            // SAFETY: `pos` lies in this piece's own places of bucket `b`, `start .. start + n`, below `at`, the
            // reserved length; the pieces' places split each bucket by their counts, so no other piece writes it.
            unsafe { dst.0.add(*pos).write(MaybeUninit::new(map_item(item))) };
            *pos += 1;
        }
        next.iter().zip(first.iter().zip(count)).all(|(x, (f, n))| *x == f + n)
    });
    if !whole.iter().all(|w| *w) {
        violation!(clause = "TIME.6", "a partition's piece left places of its buckets unwritten");
    }
    // SAFETY: every piece wrote each of its places once, the checks above hold, and the places of all pieces and
    // buckets cover every position below `at`.
    unsafe { out.items.set_len(at) };
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{Partitioned, partition_into};
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn flows_partition_is_order_free() {
        let mut d = Draws::new(stream_key(Seed::new(3), "partition"), Subject::new(SubjectTag::World, 0), 0, 0);
        let inputs: Vec<Vec<(u64, u32)>> = (0..5)
            .map(|b| {
                (0..below_u64(&mut d, 150_000))
                    .map(|i| (below_u64(&mut d, 1 << 20), b * 1_000_000 + u32::try_from(i).unwrap()))
                    .collect()
            })
            .collect();
        let slices: Vec<&[(u64, u32)]> = inputs.iter().map(Vec::as_slice).collect();
        let buckets = 37;
        let key = |x: &(u64, u32)| usize::try_from(x.0 % 37).unwrap();
        let mut expected: Vec<(u64, u32)> = Vec::new();
        for b in 0..buckets {
            expected.extend(inputs.iter().flatten().filter(|x| key(x) == b));
        }
        for workers in [1, 4] {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let mut out = Partitioned::default();
            partition_into(Some(&pool), &slices, buckets, key, &mut out);
            assert_eq!(out.items, expected, "stable and the same for {workers} workers");
            assert_eq!(out.bucket(3).len(), expected.iter().filter(|x| key(x) == 3).count());
        }
        let mut serial = Partitioned::default();
        partition_into(None, &slices, buckets, key, &mut serial);
        assert_eq!(serial.items, expected);
    }
}
