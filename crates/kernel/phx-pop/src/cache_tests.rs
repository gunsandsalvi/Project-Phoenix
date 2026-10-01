//! The per-day cache over a few hand-begun firms: a miss computing and the next read hitting, a stamp of another day
//! computing again, two readers of one day reading one value, a loaded store's caches read as never on its next day,
//! and the write-backs applied by the chunk that owns each slot, alike for any chunking.
#![cfg(test)]

use std::cell::Cell;

use phx_id::{Day, PartyRef, Slot};
use phx_num::Missing;
use phx_store::{AddressSpace, DayBuf, HeapBacking};

use super::{CacheWrite, DayCached};
use crate::directory::Directory;
use crate::kinds::KindStore;
use crate::layout::{FIRM, Layout};

type Heap = HeapBacking<4096>;

const FIRMS: u8 = 0;
const HOT: u8 = 0;
/// The run's first day.
const FIRST: Day = Day::new(10);

struct Fixture {
    dir: Directory<Heap>,
    store: KindStore<Heap>,
    cache: DayCached<i64>,
    back: DayBuf<CacheWrite, Heap>,
    firms: Vec<PartyRef>,
}

fn fixture(n: usize) -> Fixture {
    let mut space = AddressSpace::empty();
    let mut dir = Directory::new(&mut space, &[16], 16, (FIRST, 100));
    let mut layout = Layout::compile(&FIRM, &[]).unwrap();
    let value = layout.writer::<i64>("unit_cost", 0, "K-35").unwrap();
    let stamp = layout.writer::<u16>("unit_cost_day", 0, "K-35").unwrap();
    let mut store = KindStore::new(&mut space, FIRMS, &layout, 16);
    let firms = (0..n)
        .map(|_| {
            let r = dir.begin(FIRMS);
            store.begin(&dir, r, &[]);
            r
        })
        .collect();
    let back = DayBuf::new(&mut space, "write-back", 16);
    Fixture { dir, store, cache: DayCached::new(value, stamp, FIRST), back, firms }
}

/// A firm's unit cost as a pure function of what it is handed, counting its calls.
fn cost((calls, base): (&Cell<u32>, i64)) -> i64 {
    calls.set(calls.get() + 1);
    base * 2
}

impl Fixture {
    fn read(&mut self, slot: Slot, today: Day, (calls, base): (&Cell<u32>, i64)) -> i64 {
        let row = self.store.gather_at(slot, HOT).unwrap();
        self.cache.get_or((&row, slot), today, &mut self.back, (cost, (calls, base)))
    }

    fn barrier(&mut self, today: Day) {
        let writes = self.back.as_slice().to_vec();
        self.cache.apply_all(&mut self.store, &self.dir, today, &writes);
        self.back.clear();
    }
}

#[test]
fn miss_computes_then_hits() {
    let mut f = fixture(1);
    let (calls, slot, today) = (Cell::new(0), f.firms[0].slot(), FIRST);
    assert_eq!(f.read(slot, today, (&calls, 21)), 42, "day zero computes on first use");
    assert_eq!((calls.get(), f.back.len()), (1, 1), "the miss recorded for its chunk");
    f.barrier(today);
    assert_eq!(f.read(slot, today, (&calls, 99)), 42, "the barrier's write read back");
    assert_eq!((calls.get(), f.back.len()), (1, 0), "a hit computes nothing and records nothing");
}

#[test]
fn stale_stamp_recomputes() {
    let mut f = fixture(1);
    let (calls, slot) = (Cell::new(0), f.firms[0].slot());
    f.read(slot, FIRST, (&calls, 21));
    f.barrier(FIRST);
    let tomorrow = FIRST.succ();
    assert_eq!(f.read(slot, tomorrow, (&calls, 5)), 10, "yesterday's value is no longer today's");
    assert_eq!(calls.get(), 2);
    let row = f.store.gather_at(slot, HOT).unwrap();
    assert_eq!(f.cache.get(&row, tomorrow), Missing::Absent, "unwritten before its barrier");
}

#[test]
fn second_read_hits() {
    let mut f = fixture(1);
    let (calls, slot) = (Cell::new(0), f.firms[0].slot());
    let first = f.cache.get_or_mut(&mut f.store, (slot, FIRST), (cost, (&calls, 4)));
    let second = f.cache.get_or_mut(&mut f.store, (slot, FIRST), (cost, (&calls, 9)));
    assert_eq!((first, second, calls.get()), (8, 8, 1), "two readers of one day read one value");
}

#[test]
fn load_clears_stamps() {
    let mut f = fixture(1);
    let (calls, slot) = (Cell::new(0), f.firms[0].slot());
    f.read(slot, FIRST, (&calls, 21));
    f.barrier(FIRST);
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&f.store, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let back: KindStore<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    let row = back.gather_at(slot, HOT).unwrap();
    assert_eq!(f.cache.get(&row, FIRST.succ()), Missing::Absent, "a load's next day finds no value computed");
}

#[test]
fn write_back_applied_by_owner_chunk() {
    let read_all = |chunk_slots: u32| {
        let mut f = fixture(5);
        let calls = Cell::new(0);
        for (i, r) in f.firms.clone().iter().enumerate().rev() {
            f.read(r.slot(), FIRST, (&calls, i64::try_from(i).unwrap() + 1));
        }
        f.read(f.firms[2].slot(), FIRST, (&calls, 3));
        let writes = f.back.as_slice().to_vec();
        for mut chunk in f.store.chunks_mut(&f.dir, chunk_slots) {
            f.cache.apply(&mut chunk, FIRST, &writes);
        }
        f.firms.iter().map(|r| f.cache.get(&f.store.gather_at(r.slot(), HOT).unwrap(), FIRST)).collect::<Vec<_>>()
    };
    let one = read_all(16);
    assert_eq!(one, (1..=5).map(|b| Missing::Present(b * 2)).collect::<Vec<_>>());
    assert_eq!(read_all(2), one, "the same values for any chunking");
    assert_eq!(read_all(1), one);
}

#[test]
fn words_of_two_groups_refused() {
    let mut layout = Layout::compile(&FIRM, &[]).unwrap();
    let value = layout.writer::<i64>("markup", 0, "K-32").unwrap();
    let stamp = layout.writer::<u16>("unit_cost_day", 0, "K-35").unwrap();
    assert!(std::panic::catch_unwind(|| DayCached::new(value, stamp, FIRST)).is_err());
}
