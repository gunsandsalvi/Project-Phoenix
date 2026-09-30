//! Each kind's declared place is where today's records hold it, and a place a record does not hold is refused.
#![cfg(test)]

use phx_core::{Declarations, KindDecl, Place};

use super::misplaced_kinds;
use crate::consts::firm::{RECORD, REGION};

#[test]
fn declared_places_of_todays_kinds() {
    let word = |w: usize| u16::try_from(w).unwrap();
    assert_eq!(sys_frm::FIRM.place, Place::Region { word: word(REGION) });
    assert_eq!(sys_dem::HOUSEHOLD_KIND.place, Place::Sited);
    assert_eq!(phx_core::ESTATE_KIND.place, Place::Country { word: 0 });
    for site in [sys_cb::CENTRAL_BANK, sys_cb::TREASURY, sys_bnk::BANK, sys_soc::AGENCY] {
        assert_eq!(site.place, Place::Site { word: 0 }, "{} is begun with its site's tile first", site.name);
    }
    let mut d = Declarations::new();
    for k in [sys_frm::FIRM, sys_cb::CENTRAL_BANK, phx_core::ESTATE_KIND] {
        d.kind(k);
    }
    assert!(misplaced_kinds(&d, &[]).is_empty());
    let beyond = KindDecl { place: Place::Region { word: word(RECORD) }, ..sys_frm::FIRM };
    let mut d = Declarations::new();
    d.kind(beyond);
    d.kind(sys_dem::HOUSEHOLD_KIND);
    assert_eq!(misplaced_kinds(&d, &[]).len(), 2, "a word beyond the firm's record; a household no declaration sites");
}
