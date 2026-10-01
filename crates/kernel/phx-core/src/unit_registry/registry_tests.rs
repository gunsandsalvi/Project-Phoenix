//! The registry over hand-issued keys: one id a key, absent keys missing, ids in issue order and never reused, the
//! index rebuilt as issued, and each grade's content read by its grade.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::Day;
use phx_num::{Fixed, Missing, UnitId};

use super::{CapitalClass, Content, GradeContents, UnitKey, UnitRegistry, UnitTraits};
use crate::catalogue::{Declared, GradeH, ProductDecl, ProductH, compile};

const KEPT: UnitTraits = UnitTraits { price_exp: 2, storable: true, perishable: false, spoilage: 0 };

fn good(product: u16, grade: u8, zone: u16) -> UnitKey {
    UnitKey::Good { product: ProductH::new(product), grade, zone }
}

fn class(age: u8) -> CapitalClass {
    CapitalClass { kind: 7, size: 1, quality: 2, condition: 0, age }
}

/// Goods, capital classes, an instrument, a special unit and a right, in that order.
fn keys() -> Vec<UnitKey> {
    vec![
        good(0, 0, 0),
        good(0, 1, 0),
        good(0, 0, 1),
        good(3, 0, 0),
        UnitKey::Capital { class: class(0), zone: 0 },
        UnitKey::Capital { class: class(1), zone: 0 },
        UnitKey::Instrument { id: 40 },
        UnitKey::Special { kind: 2, place: 5 },
        UnitKey::Right { deposit: 9 },
    ]
}

fn issued(keys: &[UnitKey]) -> UnitRegistry {
    let mut r = UnitRegistry::with_capacity(64);
    for k in keys {
        let _ = r.issue(*k, KEPT);
    }
    r
}

#[test]
fn row_bytes_are_the_stores() {
    let bytes = u32::try_from(size_of::<super::UnitRow>()).unwrap();
    assert_eq!(bytes, crate::consts::UNIT_ROW_BYTES, "the capacity table counts the row the registry keeps");
}

#[test]
fn issue_twice_same_id() {
    let mut r = issued(&keys());
    for (at, k) in keys().into_iter().enumerate() {
        let id = UnitId::new(u32::try_from(at).unwrap());
        assert_eq!(r.issue(k, KEPT), id, "{k:?} keeps its id");
        assert_eq!(r.row(id).key(), k, "the row says what the unit is");
    }
    assert_eq!(r.len(), keys().len());
    let other = UnitTraits { perishable: true, ..KEPT };
    assert!(catch_unwind(AssertUnwindSafe(|| r.issue(good(0, 0, 0), other))).is_err(), "a key's traits are one fact");
}

#[test]
fn absent_key_is_missing() {
    let r = issued(&keys());
    assert_eq!(r.find(good(1, 0, 0)), Missing::Absent, "a product never named at a zone has no unit");
    assert_eq!(r.find(good(0, 0, 2)), Missing::Absent);
    assert_eq!(r.find(UnitKey::Instrument { id: 41 }), Missing::Absent);
    assert_eq!(r.find(good(0, 0, 1)), Missing::Present(UnitId::new(2)));
}

#[test]
fn issue_past_capacity_stops() {
    let mut r = UnitRegistry::with_capacity(2);
    let _ = (r.issue(good(0, 0, 0), KEPT), r.issue(good(0, 0, 1), KEPT));
    assert!(catch_unwind(AssertUnwindSafe(|| r.issue(good(0, 0, 2), KEPT))).is_err());
    assert!(catch_unwind(|| UnitId::new(1 << phx_num::consts::UNIT_ID_BITS)).is_err(), "the 2^24th id stops");
}

#[test]
fn age_class_in_capital_key() {
    let r = issued(&keys());
    let (young, older) =
        (r.find(UnitKey::Capital { class: class(0), zone: 0 }), r.find(UnitKey::Capital { class: class(1), zone: 0 }));
    assert_ne!(young, older, "two age classes of one kind, bands and condition are two units");
    let Missing::Present(older) = older else { panic!("issued") };
    assert_eq!(r.row(older).key(), UnitKey::Capital { class: class(1), zone: 0 }, "the age class reads back");
    let widest = CapitalClass { kind: u8::MAX, size: 63, quality: 63, condition: 63, age: 63 };
    let r = issued(&[UnitKey::Capital { class: widest, zone: 3 }]);
    assert_eq!(r.row(UnitId::new(0)).key(), UnitKey::Capital { class: widest, zone: 3 });
    let past = CapitalClass { age: 64, ..widest };
    assert!(catch_unwind(|| issued(&[UnitKey::Capital { class: past, zone: 0 }])).is_err(), "a band past its bits");
}

#[test]
fn ids_same_for_any_workers() {
    // Ids follow the order of issue alone: the same keys issued in the same order give the same rows and index,
    // whatever made them, and another order gives other ids.
    let (a, b) = (issued(&keys()), issued(&keys()));
    assert_eq!((&a.rows, &a.slots), (&b.rows, &b.slots));
    let mut reversed = keys();
    reversed.reverse();
    let c = issued(&reversed);
    assert_eq!(c.find(good(0, 0, 0)), Missing::Present(UnitId::new(8)));
}

#[test]
fn index_rebuild_equals() {
    let mut r = issued(&keys());
    r.retire(UnitId::new(6), Day::new(30));
    let _ = r.issue(UnitKey::Instrument { id: 40 }, KEPT);
    let slots = r.slots.clone();
    r.slots.clear();
    r.reindex();
    assert_eq!(r.slots, slots, "the index rebuilt from the rows is the one kept as they were issued");
}

#[test]
fn retired_row_still_reads() {
    let mut r = issued(&keys());
    let bill = UnitId::new(6);
    r.retire(bill, Day::new(30));
    assert_eq!(r.row(bill).retired(), Missing::Present(Day::new(30)));
    assert_eq!(r.row(bill).key(), UnitKey::Instrument { id: 40 }, "a retired unit still says what it was");
    assert_eq!(r.find(UnitKey::Instrument { id: 40 }), Missing::Absent);
    let again = r.issue(UnitKey::Instrument { id: 40 }, KEPT);
    assert_eq!(again, UnitId::new(9), "named again, a key takes a new id; the old is never reused");
    assert!(catch_unwind(AssertUnwindSafe(|| r.retire(bill, Day::new(31)))).is_err(), "retired once");
}

#[test]
fn content_by_grade() {
    let products =
        [ProductDecl { system: "GDS", name: "ore", grades: 2 }, ProductDecl { system: "GDS", name: "coal", grades: 1 }];
    let catalogue = compile(&Declared { products: &products, ..Declared::default() }).unwrap();
    let mut contents = GradeContents::new(&catalogue);
    let ore = GradeH { product: ProductH::new(1), grade: 1 };
    let content = Content { unit: UnitId::new(4), per_unit: Fixed::from_raw(350_000) };
    contents.declare(ore, content);
    assert_eq!(contents.content(ore), Missing::Present(content));
    assert_eq!(contents.content(GradeH { grade: 0, ..ore }), Missing::Absent, "a grade that declares none");
    let coal = GradeH { product: ProductH::new(0), grade: 1 };
    assert!(catch_unwind(|| contents.content(coal)).is_err(), "coal has one grade");
    assert!(catch_unwind(AssertUnwindSafe(|| contents.declare(ore, content))).is_err(), "declared once");
}
