//! A kind's store over a few hand-begun parties: absent words reading `Missing`, sentinels refused, stale references
//! stopped, a reused slot written in full, the byte maps' sizes, an overfull group and a second writer refused, an
//! index following its word, chunks writing their own slots, and the store round-tripping with its index kept again.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::{Day, PartyRef, Slot};
use phx_num::Missing;
use phx_store::index_decl::{IndexDecl, Keys, Mode};
use phx_store::{AddressSpace, DayBuf, HeapBacking};

use super::{Attr, AttrW, KindStore, Opening};
use crate::directory::Directory;
use crate::layout::{Extra, FIRM, HOUSEHOLD, IntTy, Layout, WordDecl, used, width};

type Heap = HeapBacking<4096>;

const HOUSEHOLDS: u8 = 0;

fn fixture() -> (Directory<Heap>, KindStore<Heap>, Layout) {
    let mut space = AddressSpace::empty();
    let dir = Directory::new(&mut space, &[64], 16, (Day::new(0), 100));
    let layout = Layout::compile(&HOUSEHOLD, &[]).unwrap();
    let store = KindStore::new(&mut space, HOUSEHOLDS, &layout, 64);
    (dir, store, layout)
}

fn residence(layout: &mut Layout) -> AttrW<u32> {
    layout.writer::<u32>("residence", 0, "K-32").unwrap()
}

fn begun(dir: &mut Directory<Heap>, store: &mut KindStore<Heap>, opening: &[Opening]) -> PartyRef {
    let r = dir.begin(HOUSEHOLDS);
    store.begin(dir, r, opening);
    r
}

fn refused(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn absent_reads_missing() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let a = begun(&mut dir, &mut store, &[]);
    let b = begun(&mut dir, &mut store, &[Opening::of(w, 7)]);
    assert_eq!(store.get(&dir, a, w.read()), Missing::Absent, "a party with nothing reads its word missing");
    assert_eq!(store.get(&dir, b, w.read()), Missing::Present(7));
    let positions: Attr<i64> = layout.attr("positions", 2).unwrap();
    assert_eq!(store.gather(&dir, b, 0).get(positions), Missing::Absent);
    let flags: Attr<u8> = layout.attr("flags", 0).unwrap();
    assert_eq!(store.get(&dir, a, flags), Missing::Present(0), "a word never absent opens at nought");
    store.set(&dir, a, w, Missing::Present(3));
    store.set(&dir, b, w, Missing::Absent);
    assert_eq!((store.get(&dir, a, w.read()), store.get(&dir, b, w.read())), (Missing::Present(3), Missing::Absent));
}

#[test]
fn sentinel_write_stops() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let a = begun(&mut dir, &mut store, &[]);
    assert!(refused(|| store.set(&dir, a, w, Missing::Present(u32::MAX))), "the sentinel is no value");
    assert!(refused(|| {
        let _ = Opening::of(w, u32::MAX);
    }));
    let flags = layout.writer::<u8>("flags", 0, "K-32").unwrap();
    assert!(refused(|| store.set(&dir, a, flags, Missing::Absent)), "a word never absent takes no absence");
}

#[test]
fn stale_reference_refused() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let old = begun(&mut dir, &mut store, &[Opening::of(w, 1)]);
    store.end(&dir, old);
    dir.end(old, Day::new(1), Missing::Absent);
    assert_eq!(store.get(&dir, old, w.read()), Missing::Present(1), "an ended party reads until its slot is reused");
    let _ = dir.close_day(Day::new(1));
    let new = begun(&mut dir, &mut store, &[]);
    assert_eq!(new.slot(), old.slot());
    assert!(
        refused(|| {
            let _ = store.get(&dir, old, w.read());
        }),
        "the old reference names the slot's past party"
    );
    assert!(refused(|| store.set(&dir, old, w, Missing::Present(2))));
    let foreign = PartyRef::new(HOUSEHOLDS + 1, 0, Slot::new(0));
    assert!(
        refused(|| {
            let _ = store.get(&dir, foreign, w.read());
        }),
        "a reference of another kind"
    );
}

#[test]
fn reused_slot_written_in_full() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let pos = layout.writer::<i64>("positions", 1, "K-32").unwrap();
    let old = begun(&mut dir, &mut store, &[Opening::of(w, 9), Opening::of(pos, -40)]);
    store.set(&dir, old, layout.writer::<u8>("flags", 0, "K-32").unwrap(), Missing::Present(5));
    dir.end(old, Day::new(1), Missing::Absent);
    let _ = dir.close_day(Day::new(1));
    let new = begun(&mut dir, &mut store, &[]);
    let blanks = layout.blanks();
    for (g, blank) in (0_u8..).zip(&blanks) {
        let row: Vec<u8> = store.scan(g).next().map(|(_, r)| r.bytes.to_vec()).unwrap();
        assert_eq!(&row, blank, "every byte of group {g} is the new party's blank");
    }
    assert_eq!(store.get(&dir, new, pos.read()), Missing::Absent);
}

#[test]
fn household_layout_is_174_bytes() {
    assert_eq!(width(&HOUSEHOLD), 174);
    assert_eq!(HOUSEHOLD.groups.first().map(|g| g.width), Some(128));
    let reserve: u16 = HOUSEHOLD.groups.iter().map(|g| g.width - used(g)).sum();
    assert_eq!(reserve, 16, "the reserve later steps fill");
    let l = Layout::compile(&HOUSEHOLD, &[]).unwrap();
    assert_eq!(l.blanks().iter().map(Vec::len).sum::<usize>(), 174);
}

#[test]
fn firm_layout_is_520_bytes() {
    assert_eq!(width(&FIRM), 520);
    let reserve: u16 = FIRM.groups.iter().map(|g| g.width - used(g)).sum();
    assert_eq!(reserve, 39);
}

#[test]
fn firm_hot_is_192_bytes() {
    let hot = FIRM.groups.first().unwrap();
    assert_eq!((hot.name, hot.width, hot.width - used(hot)), ("hot", 192, 22));
}

#[test]
fn overfull_group_refused() {
    let wide = |name, count| Extra {
        group: "warm",
        word: WordDecl { name, ty: IntTy::U16, count, absent: false, writer: "S2.109" },
    };
    assert!(Layout::compile(&HOUSEHOLD, &[wide("fits", 3)]).is_ok(), "six bytes fit the warm reserve");
    let err = Layout::compile(&HOUSEHOLD, &[wide("too_wide", 4)]).unwrap_err();
    assert!(err.contains("household") && err.contains("warm") && err.contains("48"), "{err}");
    assert!(Layout::compile(&HOUSEHOLD, &[Extra { group: "attic", ..wide("x", 1) }]).is_err());
    assert!(Layout::compile(&HOUSEHOLD, &[wide("flags", 1)]).is_err(), "a word declared twice");
}

#[test]
fn one_writer_per_attribute() {
    let mut l = Layout::compile(&HOUSEHOLD, &[]).unwrap();
    assert!(l.writer::<u32>("residence", 0, "K-60").is_err(), "a base other than the word's writer");
    assert!(l.writer::<u32>("residence", 0, "K-32").is_ok());
    assert!(l.writer::<u32>("residence", 0, "K-32").is_err(), "the word's one writer already handed out");
    assert!(l.attr::<u32>("residence", 0).is_ok(), "reads are anyone's");
    assert!(l.attr::<u16>("residence", 0).is_err(), "a word read as another type");
    assert!(l.attr::<i64>("positions", 3).is_err(), "past the word's count");
}

fn by_residence() -> IndexDecl {
    IndexDecl {
        name: "households by residence",
        member_table: "household",
        key: "residence",
        predicate: Missing::Absent,
        mode: Mode::LazyCounted,
        keys: Keys::Dense(16),
        entries: 64,
        clause: "PTY.5",
    }
}

fn members(store: &KindStore<Heap>, dir: &Directory<Heap>, key: u64, a: Attr<u32>) -> Vec<u32> {
    let mut out: DayBuf<u32, Heap> = DayBuf::new(&mut AddressSpace::empty(), "test", 64);
    let valid = |m: u32| {
        dir.at(HOUSEHOLDS, Slot::new(m))
            .is_some_and(|r| store.get(dir, r, a) == Missing::Present(u32::try_from(key).unwrap()))
    };
    let _ = store.index(0).unwrap().members(key, valid, &mut out);
    out.as_slice().to_vec()
}

#[test]
fn index_instance_follows_set() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let _ = store.keep_index(&mut AddressSpace::empty(), w.read(), by_residence(), std::iter::empty()).unwrap();
    let a = begun(&mut dir, &mut store, &[Opening::of(w, 3)]);
    let b = begun(&mut dir, &mut store, &[Opening::of(w, 3)]);
    assert_eq!(members(&store, &dir, 3, w.read()), [0, 1]);
    store.set(&dir, a, w, Missing::Present(5));
    assert_eq!(members(&store, &dir, 3, w.read()), [1]);
    assert_eq!(members(&store, &dir, 5, w.read()), [0]);
    let counts = |s: &KindStore<Heap>| (s.index(0).unwrap().count(3), s.index(0).unwrap().count(5));
    assert_eq!(counts(&store), (1, 1));
    store.end(&dir, b);
    dir.end(b, Day::new(1), Missing::Absent);
    assert_eq!(counts(&store), (0, 1), "an ended party leaves its key");
    let mut chunk = store.chunks_mut(&dir, 16).next().unwrap();
    assert!(refused(|| chunk.set(a, w, Missing::Present(7))), "an indexed word moves only through set");
}

#[test]
fn chunk_writes_disjoint() {
    let (mut dir, mut store, mut layout) = fixture();
    let pos = layout.writer::<i64>("positions", 0, "K-32").unwrap();
    let parties: Vec<PartyRef> = (0..40).map(|_| begun(&mut dir, &mut store, &[])).collect();
    let mut seen = Vec::new();
    for mut chunk in store.chunks_mut(&dir, 16) {
        let (first, n) = chunk.slots();
        seen.push((first.get(), n));
        for r in parties.iter().filter(|r| (first.get()..first.get() + n).contains(&r.slot().get())) {
            chunk.set(*r, pos, Missing::Present(i64::from(r.slot().get())));
        }
        let outside = parties.iter().find(|r| r.slot().get() >= first.get() + n).copied();
        if let Some(o) = outside {
            assert!(refused(|| chunk.set(o, pos, Missing::Present(0))), "a slot of another chunk");
        }
    }
    assert_eq!(seen, [(0, 16), (16, 16), (32, 8)]);
    for r in &parties {
        assert_eq!(store.get(&dir, *r, pos.read()), Missing::Present(i64::from(r.slot().get())));
    }
}

#[test]
fn empty_kind_reads_nothing() {
    let (dir, mut store, layout) = fixture();
    assert_eq!((store.rows(), store.scan(0).count(), store.chunks_mut(&dir, 16).count()), (0, 0, 0));
    let nobody = PartyRef::new(HOUSEHOLDS, 0, Slot::new(0));
    assert!(refused(|| {
        let _ = store.get(&dir, nobody, layout.attr::<u32>("residence", 0).unwrap());
    }));
}

#[test]
fn save_round_trip_rebuilds_indexes() {
    let (mut dir, mut store, mut layout) = fixture();
    let w = residence(&mut layout);
    let _ = store.keep_index(&mut AddressSpace::empty(), w.read(), by_residence(), std::iter::empty()).unwrap();
    let parties: Vec<PartyRef> = (0..10_u32).map(|i| begun(&mut dir, &mut store, &[Opening::of(w, i % 3)])).collect();
    store.set(&dir, parties[4], w, Missing::Present(2));
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&store, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let mut back: KindStore<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    let _ = back.keep_index(&mut AddressSpace::empty(), w.read(), by_residence(), dir.live_slots(HOUSEHOLDS)).unwrap();
    for key in 0..3 {
        assert_eq!(members(&back, &dir, key, w.read()), members(&store, &dir, key, w.read()), "key {key}");
        assert_eq!(back.index(0).unwrap().count(key), store.index(0).unwrap().count(key));
    }
    for r in &parties {
        assert_eq!(back.get(&dir, *r, w.read()), store.get(&dir, *r, w.read()));
    }
}
