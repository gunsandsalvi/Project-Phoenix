//! The firm's words over a hand-begun firm or two: each of its old record words a handle of its own, read back in the
//! units the rules take, and a firm with no sales outlook reading none.
#![cfg(test)]

use phx_id::{Day, Slot};
use phx_num::Missing;
use phx_pop::directory::Directory;
use phx_store::AddressSpace;

use super::{FirmOpening, FirmStore, words};

const FIRMS: u8 = 0;

fn fixture() -> (Directory, FirmStore) {
    let mut space = AddressSpace::empty();
    let dir = Directory::new(&mut space, &[16], 16, (Day::new(100), 730));
    let store = FirmStore::new(&mut space, FIRMS, 16, (vec![7, 9], vec![100, 199, 499, 999], Day::new(100)));
    (dir, store)
}

fn opening(expected: Missing<f64>) -> FirmOpening {
    FirmOpening {
        product: 3,
        site: 4_242,
        zone: 1,
        productivity: -1.25,
        price: 49_900,
        output: 36_524.25,
        markup: Missing::Present(0.35),
        expected,
        opened: Day::new(102),
        founding: [2, 1, 4],
        stance: 3,
        head_office: Missing::Present(40),
    }
}

#[test]
fn firm_words_map_to_handles() {
    let w = words();
    let places = [
        w.product.read().place(),
        w.zone.read().place(),
        w.site.read().place(),
        w.productivity.read().place(),
        w.price.read().place(),
        w.rate.read().place(),
        w.markup.read().place(),
        w.expected.read().place(),
        w.width.read().place(),
        w.sold.read().place(),
        w.seen.read().place(),
        w.reviewed.read().place(),
    ];
    let mut sorted = places.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), places.len(), "every old word has one handle, none two: {places:?}");
    let (mut dir, mut store) = fixture();
    let r = dir.begin(FIRMS);
    store.begin(&dir, r, &opening(Missing::Present(100.0)));
    let v = store.view(r.slot()).unwrap();
    assert_eq!((v.product(), v.zone(), v.region(), v.site()), (Some(3), Some(1), Some(9), Some(4_242)));
    assert_eq!(v.price(), Some(49_900), "the price read back from its point's code");
    assert!((v.productivity().unwrap() + 1.25).abs() < 1e-8);
    assert!((v.output().unwrap() - 36_524.25).abs() < 1e-3, "the output a year read back from its rate a day");
    assert_eq!((v.markup(), v.expected(), v.sold(), v.seen_sold()), (Some(0.35), Some(100.0), Some(0), Some(0)));
    assert_eq!((v.sales_width(), v.reviewed()), (None, Some(Day::new(102))));
    store.set_reviewed(r.slot(), Day::new(130));
    store.set_price(r.slot(), 1_990);
    let v = store.view(r.slot()).unwrap();
    assert_eq!((v.reviewed(), v.price()), (Some(Day::new(130)), Some(1_990)));
    assert!(store.view(Slot::new(5)).is_none(), "no firm at a slot never begun");
}

/// A firm's founding types, stance and first office are its record's, and a reconsidered stance replaces its first.
#[test]
fn founding_and_offices_kept_in_the_record() {
    let (mut dir, mut store) = fixture();
    let r = dir.begin(FIRMS);
    store.begin(&dir, r, &opening(Missing::Absent));
    let v = store.view(r.slot()).unwrap();
    assert_eq!(v.founding(), [Missing::Present(2), Missing::Present(1), Missing::Present(4)]);
    assert_eq!((v.stance(), v.head_office()), (Missing::Present(3), Missing::Present(40)));
    store.set_stance(r.slot(), 1);
    assert_eq!(store.view(r.slot()).unwrap().stance(), Missing::Present(1));
    let s = dir.begin(FIRMS);
    store.begin(&dir, s, &FirmOpening { head_office: Missing::Absent, ..opening(Missing::Absent) });
    assert_eq!(store.view(s.slot()).unwrap().head_office(), Missing::Absent, "no office opened reads none");
}

#[test]
fn firm_without_outlook_decides_nothing() {
    let (mut dir, mut store) = fixture();
    let r = dir.begin(FIRMS);
    store.begin(&dir, r, &opening(Missing::Absent));
    let v = store.view(r.slot()).unwrap();
    assert_eq!((v.expected(), v.sales_width()), (None, None), "no outlook reads missing, never nought");
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.set_price(r.slot(), 1_500)));
    assert!(refused.is_err(), "a price that is no point is refused");
}

#[test]
fn working_bit_declared_in_flags() {
    let (mut dir, mut store) = fixture();
    let r = dir.begin(FIRMS);
    store.begin(&dir, r, &opening(Missing::Present(100.0)));
    assert_eq!(store.view(r.slot()).unwrap().extraction(), None, "undecided until its first review");
    store.set_extraction(r.slot(), Some(false));
    assert_eq!(store.view(r.slot()).unwrap().extraction(), Some(false));
    store.set_extraction(r.slot(), Some(true));
    assert_eq!(store.view(r.slot()).unwrap().extraction(), Some(true));
    store.set_extraction(r.slot(), None);
    assert_eq!(store.view(r.slot()).unwrap().extraction(), None, "a decision cleared as its rights pass");
}

#[test]
fn carrier_mode_word() {
    let (mut dir, mut store) = fixture();
    let (carrier, maker) = (dir.begin(FIRMS), dir.begin(FIRMS));
    store.begin(&dir, carrier, &opening(Missing::Present(100.0)));
    store.begin(&dir, maker, &opening(Missing::Present(100.0)));
    store.set_mode(carrier.slot(), 2);
    assert_eq!(store.view(carrier.slot()).unwrap().mode(), Some(2));
    assert_eq!(store.view(maker.slot()).unwrap().mode(), None, "a firm that carries nothing has no mode");
}
