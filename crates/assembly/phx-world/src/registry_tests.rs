//! Each kind's declared place is where today's records hold it, and a place a record does not hold is refused.
#![cfg(test)]

use phx_core::{Declarations, KindDecl, Place};

use super::{heirless_refusals, misplaced_kinds};
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

#[test]
fn heirless_destination_read_per_country() {
    let kinds = ["treasury", "household", "firm"];
    assert!(heirless_refusals(&kinds, &[Some("treasury"), Some("treasury")]).is_empty());
}

#[test]
fn heirless_destination_missing_refused() {
    let refused = heirless_refusals(&["treasury"], &[Some("treasury"), None]);
    assert_eq!(refused, vec!["country 1: its law names no destination for an estate with no heir".to_owned()]);
}

#[test]
fn heirless_destination_unknown_kind_refused() {
    let refused = heirless_refusals(&["treasury"], &[Some("church")]);
    assert_eq!(refused.len(), 1);
    assert!(refused[0].contains("`church`"));
}

#[test]
fn todays_families_hold_declared_rows() {
    #[derive(serde::Deserialize)]
    struct File {
        family: Vec<phx_core::catalogue::FamilyCode>,
    }
    let file: File = toml::from_str(include_str!("../../../../data/shared/families.toml")).unwrap();
    let codes = phx_core::catalogue::FamilyCodes::new(file.family).unwrap();
    let declared = |name: &str| {
        codes.rows().iter().any(|r| r.name == name && r.status == phx_core::catalogue::FamilyStatus::Declared)
    };
    for name in crate::consts::families::ALL {
        assert!(declared(name), "`{name}` has no declared row");
    }
    let rows = codes.rows().iter().filter(|r| r.status == phx_core::catalogue::FamilyStatus::Declared).count();
    assert_eq!(rows, crate::consts::families::ALL.len(), "a declared row the world does not declare");
}
