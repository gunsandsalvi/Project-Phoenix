//! An agency's staffing record over a hand-begun agency: a cell for every region and occupation the record holds,
//! as many occupations as the labour law's, a cell with no post reading none, and posts and wages kept by their
//! writer alone.
#![cfg(test)]

use phx_id::Day;
use phx_num::Missing;
use phx_pop::consts::{AGENCY_OCCUPATIONS, AGENCY_REGIONS};
use phx_pop::directory::Directory;
use phx_store::{AddressSpace, SystemBacking};

use super::{AgencyStore, cell, compiled};

#[test]
fn agency_targets_array_declared() {
    assert_eq!(u32::from(AGENCY_OCCUPATIONS), if_labour::consts::OCCUPATIONS, "the labour law's occupations");
    let (_, w) = compiled();
    assert_eq!(w.targets.len(), usize::from(AGENCY_REGIONS) * usize::from(AGENCY_OCCUPATIONS));
    let mut places: Vec<(u8, u16)> = w.targets.iter().map(|a| a.read().place()).collect();
    places.sort_unstable();
    places.dedup();
    assert_eq!(places.len(), w.targets.len(), "every cell its own word");
    assert_eq!((cell(0, 0), cell(1, 0), cell(2, 3)), (0, 11, 25), "region-major");
    let mut space = AddressSpace::empty();
    let mut dir: Directory<SystemBacking> = Directory::new(&mut space, &[4], 4, (Day::new(0), 30));
    let mut agencies = AgencyStore::new(&mut space, 0, 4);
    let agency = dir.begin(0);
    agencies.begin(&dir, agency);
    let s = agencies.staffing(agency.slot()).unwrap();
    assert_eq!((s.target(3, 2), s.budget()), (Missing::Absent, 0), "no post kept before the opening's");
    agencies.keep_post(agency.slot(), (3, 2), 4_000);
    agencies.keep_post(agency.slot(), (3, 2), 4_500);
    agencies.keep_post(agency.slot(), (5, 0), 3_000);
    let s = agencies.staffing(agency.slot()).unwrap();
    assert_eq!((s.target(3, 2), s.target(5, 0), s.budget()), (Missing::Present(2), Missing::Present(1), 11_500));
    assert_eq!(s.targets().collect::<Vec<_>>(), [(3, 2, 2), (5, 0, 1)]);
    assert!(std::panic::catch_unwind(|| cell(u32::from(AGENCY_REGIONS), 0)).is_err(), "a region past the record");
}
