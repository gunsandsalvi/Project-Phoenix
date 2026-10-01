//! The deposits register over hand-entered deposits: extracted and remaining summing to the opening, extraction past
//! what remains stopping, a deposit without end having no remaining figure, the grade falling as it is worked, the
//! same totals in any order, two extractors on one deposit in turn, and the rows round-tripping.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::TileId;
use phx_num::{Fixed, Missing, UnitId};

use super::{DepositId, Deposits, Extraction};

fn register() -> Deposits {
    let mut d = Deposits::default();
    let units = |i| (UnitId::new(i), UnitId::new(100 + i));
    let _ = d.open(TileId::new(3), units(0), Fixed::from_raw(1_200), Missing::Present(1_000));
    let _ = d.open(TileId::new(4), units(1), Fixed::from_raw(800), Missing::Absent);
    let _ = d.open(TileId::new(9), units(2), Fixed::from_raw(1_000), Missing::Present(50));
    d
}

fn take(deposit: u32, extractor: u32, units: u64) -> Extraction {
    Extraction { deposit: DepositId::new(deposit), extractor, units }
}

#[test]
fn identity_holds_after_batches() {
    let mut d = register();
    let _ = d.extract_batch(&[take(0, 1, 300), take(2, 1, 20)]);
    let _ = d.extract_batch(&[take(0, 2, 150), take(1, 2, 10_000)]);
    for id in [0, 2] {
        let row = d.row(DepositId::new(id));
        let (Missing::Present(opening), Missing::Present(left)) = (row.opening(), row.remaining()) else { panic!() };
        assert_eq!(row.extracted() + left, opening, "extracted and remaining are what it held");
    }
    assert_eq!(d.row(DepositId::new(0)).remaining(), Missing::Present(550));
}

#[test]
fn over_extraction_stops() {
    let mut d = register();
    assert!(catch_unwind(AssertUnwindSafe(|| d.extract_batch(&[take(2, 1, 51)]))).is_err(), "past what it holds");
    assert!(catch_unwind(AssertUnwindSafe(|| d.extract_batch(&[take(7, 1, 1)]))).is_err(), "where there is none");
}

#[test]
fn unbounded_has_no_remaining() {
    let mut d = register();
    let _ = d.extract_batch(&[take(1, 1, u64::from(u32::MAX))]);
    let row = d.row(DepositId::new(1));
    assert_eq!((row.opening(), row.remaining()), (Missing::Absent, Missing::Absent), "no number stands for its end");
    assert_eq!(row.extracted(), u64::from(u32::MAX), "what it gave is still counted");
}

#[test]
fn grade_falls_with_share_extracted() {
    let mut d = register();
    let rule = |opening: f64, share: f64| opening * libm::exp(-0.5 * share);
    let fresh = d.grade_now(DepositId::new(0), rule);
    let _ = d.extract_batch(&[take(0, 1, 500)]);
    let worked = d.grade_now(DepositId::new(0), rule);
    assert!((fresh - 1.2).abs() < 1e-12 && worked < fresh, "the richest part first");
    assert!((worked - 1.2 * libm::exp(-0.25)).abs() < 1e-12);
    let _ = d.extract_batch(&[take(1, 1, 500)]);
    assert!((d.grade_now(DepositId::new(1), rule) - 0.8).abs() < 1e-12, "a deposit without end keeps its grade");
}

#[test]
fn batch_same_for_any_workers() {
    // Totals are sums, so the order the day's makers handed their items in changes nothing.
    let items = [take(0, 1, 10), take(2, 2, 5), take(0, 3, 7), take(1, 4, 9)];
    let mut reversed = items;
    reversed.reverse();
    let (mut a, mut b) = (register(), register());
    let _ = (a.extract_batch(&items), b.extract_batch(&reversed));
    assert_eq!(a, b);
}

#[test]
fn two_extractors_one_deposit() {
    // Each reads what remains before the other's take is applied; the batch takes them in turn, and the second past
    // what the first left stops at its own item.
    let mut d = register();
    let _ = d.extract_batch(&[take(2, 1, 30), take(2, 2, 20)]);
    assert_eq!(d.row(DepositId::new(2)).remaining(), Missing::Present(0));
    let mut e = register();
    assert!(catch_unwind(AssertUnwindSafe(|| e.extract_batch(&[take(2, 1, 30), take(2, 2, 21)]))).is_err());
}

#[test]
fn deposits_roundtrip() {
    let mut d = register();
    let _ = d.extract_batch(&[take(0, 1, 300)]);
    let (back, _) = phx_store::roundtrip(&d).unwrap();
    assert_eq!(back, d);
    assert_eq!(size_of::<super::DepositRow>(), 32, "a deposit's row is thirty-two bytes");
}
