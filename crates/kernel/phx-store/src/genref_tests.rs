//! Slots return first in first out after the close, and a reference names its row only while the row lives at the
//! generation it was taken at.
#![cfg(test)]

use phx_id::Slot;

use super::{GenRef, Generations, ShortRef, next};
use crate::backing::{AddressSpace, HeapBacking};
use crate::save::Saved;
use crate::table::SlotAlloc;

type Heap = HeapBacking<4096>;

#[derive(Debug)]
struct Rows;

fn table(max: u32, bits: u32) -> (SlotAlloc<Heap>, Generations<Rows, Heap>) {
    let mut space = AddressSpace::empty();
    let slots = SlotAlloc::new(&mut space, max);
    (slots, Generations::new(&mut space, max, 64, bits))
}

fn taken(s: &mut SlotAlloc<Heap>, n: usize) -> Vec<u32> {
    (0..n).map(|_| s.alloc().get()).collect()
}

#[test]
fn fifo_reuse_after_close() {
    let (mut s, _) = table(100, u32::BITS);
    let _ = taken(&mut s, 8);
    for n in [6, 2] {
        s.release(Slot::new(n));
    }
    s.close_day();
    for n in [5, 0] {
        s.release(Slot::new(n));
    }
    s.close_day();
    assert_eq!(taken(&mut s, 5), [2, 6, 0, 5, 8], "the first day's releases first, each day's ascending");
}

#[test]
fn no_reuse_within_the_day() {
    let (mut s, _) = table(100, u32::BITS);
    let _ = taken(&mut s, 3);
    s.release(Slot::new(1));
    assert_eq!(s.alloc().get(), 3, "a slot released today waits for the close");
}

#[test]
fn stale_ref_refused_after_reuse() {
    let (mut s, mut g) = table(100, u32::BITS);
    let first = g.alloc(&mut s);
    assert_eq!(g.resolve(&s, first), Some(first.slot()));
    s.release(first.slot());
    assert_eq!(g.resolve(&s, first), None, "an ended row is refused");
    s.close_day();
    let second = g.alloc(&mut s);
    assert_eq!((second.slot(), second.generation()), (first.slot(), 1));
    assert_eq!(g.resolve(&s, first), None, "the old reference is refused once the slot holds another");
    assert_eq!(g.resolve(&s, second), Some(second.slot()));
}

#[test]
fn party_generation_wrap_stops() {
    let last = u32::MAX >> (u32::BITS - phx_id::consts::GENERATION_BITS);
    assert_eq!(next(last - 1, last), Some(last));
    assert_eq!(next(last, last), None, "a party's generation never wraps");
    let (mut s, mut g) = table(4, 2);
    for _ in 0..4 {
        let r = g.alloc(&mut s);
        s.release(r.slot());
        s.close_day();
    }
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| g.alloc(&mut s)));
    assert!(caught.is_err(), "a fifth row in a slot of two-bit generations stops the run");
}

#[test]
fn short_ref_modular_compare() {
    let r: ShortRef<Rows> = ShortRef::new(7, Slot::new(0x00AB_CDEF), 1);
    assert_eq!((r.family(), r.slot(), r.generation()), (7, Slot::new(0x00AB_CDEF), 1));
    let live = [u64::MAX];
    let at = |g: u32| move |_: Slot| Some(g.to_le_bytes()[0]);
    let near: ShortRef<Rows> = ShortRef::new(0, Slot::new(3), 1);
    assert_eq!(near.resolve(&live, at(1)), Some(Slot::new(3)));
    assert_eq!(near.resolve(&live, at(2)), None, "the slot turned once");
    assert_eq!(near.resolve(&live, at(257)), Some(Slot::new(3)), "256 turns alias, which the re-read rule excludes");
    assert_eq!(near.resolve(&[0], at(1)), None, "an ended row");
    assert!(std::panic::catch_unwind(|| ShortRef::<Rows>::new(0, Slot::new(1 << 24), 0)).is_err());
}

#[test]
fn close_day_cost_follows_releases() {
    let max = 1 << 22;
    let (mut s, _) = table(max, u32::BITS);
    let ended: u32 = 115_000;
    let _ = taken(&mut s, 200_000);
    let before = s.bytes_committed();
    for n in 0..ended {
        s.release(Slot::new(n * 2 % 200_000 + n * 2 / 200_000));
    }
    s.close_day();
    let ring = s.bytes_committed() - before;
    let bound = (usize::try_from(ended).unwrap() * 2).next_power_of_two() * size_of::<u32>();
    assert!(ring <= bound, "the ring holds {ring} bytes, following the day's {ended} releases, not {max} slots");
    s.close_day();
    assert_eq!(s.bytes_committed() - before, ring, "a close with no releases touches nothing");
}

#[test]
fn fifo_order_survives_save() {
    let (mut s, mut g) = table(100, u32::BITS);
    let refs: Vec<GenRef<Rows>> = (0..10).map(|_| g.alloc(&mut s)).collect();
    for r in refs.iter().step_by(3) {
        s.release(r.slot());
    }
    s.close_day();
    let _ = s.alloc();
    s.release(Slot::new(1));
    let mut bytes = Vec::new();
    {
        let mut w = crate::save::Writer::new(&mut bytes).unwrap();
        s.save(&mut w);
        g.save(&mut w);
        let _ = w.finish().unwrap();
    }
    let mut source = bytes.as_slice();
    let mut reader = crate::save::Reader::new(&mut source).unwrap();
    let mut loaded: SlotAlloc<Heap> = SlotAlloc::load(&mut reader).unwrap();
    let mut gens: Generations<Rows, Heap> = Generations::load(&mut reader).unwrap();
    s.close_day();
    loaded.close_day();
    let a: Vec<GenRef<Rows>> = (0..6).map(|_| g.alloc(&mut s)).collect();
    let b: Vec<GenRef<Rows>> = (0..6).map(|_| gens.alloc(&mut loaded)).collect();
    assert_eq!(a, b, "a restored table hands out the same slots at the same generations");
}

#[test]
fn slots_same_for_any_workers() {
    // Allocations are applied in range order whatever worker computed each range, so the slots follow only the
    // requests' order, however the day's requests are split into ranges.
    let requests: Vec<bool> = (0..64_u32).map(|i| i % 3 != 0).collect();
    let run = |ranges: usize| {
        let (mut s, mut g) = table(100, u32::BITS);
        let _ = (0..20).map(|_| g.alloc(&mut s)).collect::<Vec<_>>();
        for n in [4, 17, 9] {
            s.release(Slot::new(n));
        }
        s.close_day();
        let mut out = Vec::new();
        for range in requests.chunks(requests.len().div_ceil(ranges)) {
            for keep in range {
                let r = g.alloc(&mut s);
                if !keep {
                    s.release(r.slot());
                }
                out.push(r);
            }
        }
        out
    };
    let one = run(1);
    assert_eq!(one[..3].iter().map(|r| r.slot().get()).collect::<Vec<_>>(), [4, 9, 17]);
    assert_eq!(one, run(4));
    assert_eq!(one, run(7));
}
