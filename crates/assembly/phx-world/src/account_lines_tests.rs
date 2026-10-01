//! The lines' arithmetic over two hand-begun banks: an amount entered on a line moves its owner's equity account by
//! its sign, the other party's not at all, and a line past an i64 stops the run.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::Day;
use phx_pop::directory::Directory;
use phx_pop::kinds::KindStore;
use phx_pop::layout::{BANK, Layout};
use phx_store::{AddressSpace, SystemBacking};

use super::{BookWords, Line, added};

#[test]
fn income_lines_i64_refuse_overflow() {
    assert_eq!(added(i64::MAX - 1, 1), i64::MAX);
    assert_eq!(added(-5, -7), -12);
    assert!(catch_unwind(|| added(i64::MAX, 1)).is_err(), "a line past an i64 stops the run");
    assert!(catch_unwind(|| added(i64::MIN, -1)).is_err());
}

#[test]
fn recognise_adds_to_owner_lines() {
    let mut space = AddressSpace::empty();
    let mut dir: Directory<SystemBacking> = Directory::new(&mut space, &[4], 4, (Day::new(0), 30));
    let mut layout = Layout::compile(&BANK, &[]).unwrap();
    let books = BookWords::bind(&mut layout);
    let mut store = KindStore::new(&mut space, 0, &layout, 4);
    let (a, b) = (dir.begin(0), dir.begin(0));
    store.begin(&dir, a, &[]);
    store.begin(&dir, b, &[]);
    books.open(&mut store, a.slot(), 1_000);
    books.open(&mut store, b.slot(), 500);
    books.add(&mut store, a.slot(), Line::Revenue, 300);
    books.add(&mut store, a.slot(), Line::Wages, 120);
    books.add(&mut store, a.slot(), Line::InterestReceived, 7);
    let lines = books.lines(&store, a.slot()).unwrap();
    assert_eq!((lines.get(Line::Revenue), lines.get(Line::Wages)), (300, 120));
    assert_eq!(books.equity(&store, a.slot()), Some(1_187), "300 + 7 in, 120 out");
    assert_eq!(books.equity(&store, b.slot()), Some(500), "the other bank's lines untouched");
    books.add(&mut store, b.slot(), Line::Revenue, i64::MAX - 1);
    assert!(catch_unwind(AssertUnwindSafe(|| books.add(&mut store, b.slot(), Line::Revenue, 2))).is_err());
}
