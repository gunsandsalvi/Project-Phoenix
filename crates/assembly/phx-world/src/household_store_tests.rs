//! The household's words over a hand-begun household or two: each of its old attributes and positions a handle of its
//! own, read back in the units the rules take, the stance and the trying flag written in their bits alone.
#![cfg(test)]

use phx_id::Day;
use phx_num::Missing;
use phx_pop::directory::Directory;
use phx_store::AddressSpace;

use super::{HouseholdOpening, HouseholdStore, words};

const HOUSEHOLDS: u8 = 0;
/// The run's first month, counted from the calendar's year zero.
const FIRST: i64 = 24_301;

fn fixture() -> (Directory, HouseholdStore) {
    let mut space = AddressSpace::empty();
    let dir = Directory::new(&mut space, &[16], 16, (Day::new(100), 730));
    let store = HouseholdStore::new(&mut space, HOUSEHOLDS, 16, (vec![7, 9], FIRST));
    (dir, store)
}

fn opening() -> HouseholdOpening {
    HouseholdOpening {
        zone: 1,
        preference: sys_hh::preference_type(3, 5).unwrap(),
        stance: 2,
        window: Missing::Present(4),
        income: 3_100_000,
    }
}

#[test]
fn household_words_map_to_handles() {
    let w = words();
    let places = [
        w.residence.read().place(),
        w.states.read().place(),
        w.preference.read().place(),
        w.flags.read().place(),
        w.received.read().place(),
        w.after.read().place(),
        w.income.read().place(),
        w.looked.read().place(),
        w.window.read().place(),
        w.ideal.read().place(),
    ];
    let mut sorted = places.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), places.len(), "every old word has one handle, none two: {places:?}");
    let (mut dir, mut store) = fixture();
    let r = dir.begin(HOUSEHOLDS);
    store.begin(&dir, r, &opening());
    let v = store.view(r.slot()).unwrap();
    assert_eq!((v.zone(), v.region(), v.stance(), v.types()), (Some(1), Some(9), Some(2), Some((3, 5))));
    assert_eq!((v.trying(), v.ideal(), v.window()), (Some(false), Missing::Absent, Missing::Present(4)));
    assert_eq!((v.income(), v.received(), v.after(), v.looked()), (Some(3_100_000), None, None, None));
}

#[test]
fn bits_written_alone() {
    let (mut dir, mut store) = fixture();
    let r = dir.begin(HOUSEHOLDS);
    store.begin(&dir, r, &opening());
    store.set_trying(r.slot(), true);
    store.set_stance(r.slot(), 3);
    store.set_looked(r.slot(), FIRST + 14);
    store.set_ideal(r.slot(), Missing::Present(2));
    let v = store.view(r.slot()).unwrap();
    assert_eq!(
        (v.trying(), v.stance(), v.looked(), v.ideal()),
        (Some(true), Some(3), Some(FIRST + 14), Missing::Present(2))
    );
    store.set_trying(r.slot(), false);
    assert_eq!(store.view(r.slot()).unwrap().stance(), Some(3), "a flag leaves the stance as it was");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.set_stance(r.slot(), 8))).is_err());
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| store.set_looked(r.slot(), FIRST - 1))).is_err());
}
