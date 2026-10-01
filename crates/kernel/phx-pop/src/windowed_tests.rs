//! A windowed group over a few hand-begun parties: read inside its window, `Missing` outside it and for a party
//! ended, a write outside stopping the run, a closed window holding no page, a save inside the window restoring it, and
//! a country with no campaign costing nothing.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::{Day, PartyRef, Slot};
use phx_num::Missing;
use phx_store::{AddressSpace, HeapBacking, StoreStats};

use super::{WAttrW, WindowLayout, WindowedGroup};
use crate::directory::Directory;
use crate::layout::{GroupDecl, IntTy, WordDecl};

type Heap = HeapBacking<4096>;

const PERSONS: u8 = 0;
const CAMPAIGN: &[GroupDecl] = &[GroupDecl {
    name: "campaign",
    width: 1,
    words: &[WordDecl { name: "intention", ty: IntTy::U8, count: 1, absent: true, writer: "S5.137" }],
}];
/// Two countries of eight persons each, their slots begun country by country.
const COUNTRY: u32 = 8;

fn fixture() -> (Directory<Heap>, WindowedGroup<Heap>, WAttrW<u8>, Vec<PartyRef>) {
    let mut dir = Directory::new(&mut AddressSpace::empty(), &[64], 16, (Day::new(0), 100));
    let mut layout = WindowLayout::compile("person", CAMPAIGN).unwrap();
    let w = layout.writer::<u8>("intention", 0, "S5.137").unwrap();
    let group = WindowedGroup::new(PERSONS, &layout, 3);
    let parties = (0..2 * COUNTRY).map(|_| dir.begin(PERSONS)).collect();
    (dir, group, w, parties)
}

fn open(group: &mut WindowedGroup<Heap>, country: u32) {
    group.open(
        &mut AddressSpace::empty(),
        country,
        (Slot::new(country * COUNTRY), COUNTRY),
        (Day::new(10), Day::new(40)),
    );
}

fn refused(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn read_inside_window() {
    let (dir, mut group, w, parties) = fixture();
    open(&mut group, 0);
    for (r, v) in parties.iter().take(4).zip(1..) {
        group.set(&dir, *r, w, Missing::Present(v));
    }
    let read: Vec<Missing<u8>> = parties.iter().take(5).map(|r| group.get(&dir, *r, w.read())).collect();
    let present = |v| Missing::Present(v);
    assert_eq!(read, [present(1), present(2), present(3), present(4), Missing::Absent], "unwritten reads missing");
    assert_eq!(group.rows_live(), 4, "rows written as far as the last");
}

#[test]
fn outside_window_reads_missing() {
    let (dir, mut group, w, parties) = fixture();
    let first = parties.first().copied().unwrap();
    assert_eq!(group.get(&dir, first, w.read()), Missing::Absent, "no window open");
    open(&mut group, 0);
    group.set(&dir, first, w, Missing::Present(3));
    let other = parties.get(9).copied().unwrap();
    assert_eq!(group.get(&dir, other, w.read()), Missing::Absent, "a slot another country holds");
    group.close(0);
    assert_eq!(group.get(&dir, first, w.read()), Missing::Absent, "the window closed");
}

#[test]
fn set_outside_window_stops() {
    let (dir, mut group, w, parties) = fixture();
    open(&mut group, 0);
    let other = parties.get(9).copied().unwrap();
    assert!(refused(|| group.set(&dir, other, w, Missing::Present(1))));
    assert!(refused(|| group.close(1)), "a window not open");
    assert!(refused(|| open(&mut group, 0)), "a second window for one country");
}

#[test]
fn close_decommits() {
    let (dir, mut group, w, parties) = fixture();
    open(&mut group, 0);
    open(&mut group, 1);
    for r in &parties {
        group.set(&dir, *r, w, Missing::Present(1));
    }
    assert!(group.bytes() > 0);
    assert_eq!(group.closing(Day::new(39)).count(), 0);
    let due: Vec<u32> = group.closing(Day::new(40)).collect();
    for c in due {
        group.close(c);
    }
    assert_eq!((group.bytes(), group.open_windows(), group.rows_live()), (0, 0, 0), "a closed window holds no page");
}

#[test]
fn save_inside_window_round_trip() {
    let (dir, mut group, w, parties) = fixture();
    open(&mut group, 1);
    for (r, v) in parties.iter().skip(8).zip(1..) {
        group.set(&dir, *r, w, Missing::Present(v));
    }
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&group, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let back: WindowedGroup<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    assert_eq!(back.open_windows(), 1);
    for r in &parties {
        assert_eq!(back.get(&dir, *r, w.read()), group.get(&dir, *r, w.read()));
    }
    assert_eq!(back.closing(Day::new(40)).collect::<Vec<_>>(), [1]);
}

#[test]
fn ended_party_reads_missing() {
    let (mut dir, mut group, w, parties) = fixture();
    open(&mut group, 0);
    let gone = parties.first().copied().unwrap();
    group.set(&dir, gone, w, Missing::Present(4));
    dir.end(gone, Day::new(12), Missing::Absent);
    assert_eq!(group.get(&dir, gone, w.read()), Missing::Absent, "an ended party reads nothing");
    assert!(refused(|| group.set(&dir, gone, w, Missing::Present(5))));
    // The slot is handed out again only after the day closes, at the ring's end: begin until it comes round.
    let _ = dir.close_day(Day::new(12));
    let new = loop {
        let r = dir.begin(PERSONS);
        group.begun(r);
        if r.slot() == gone.slot() {
            break r;
        }
    };
    assert_eq!(group.get(&dir, new, w.read()), Missing::Absent, "the slot's new party reads nothing of the old");
}

#[test]
fn no_window_no_bytes() {
    let (dir, group, w, parties) = fixture();
    assert_eq!((group.bytes(), group.open_windows()), (0, 0));
    assert!(parties.iter().all(|r| group.get(&dir, *r, w.read()) == Missing::Absent));
}
