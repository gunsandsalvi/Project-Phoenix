//! Each kind's declared place is where its records or its store hold it, and each country's law names a destination
//! for an estate with no heir.
#![cfg(test)]

use phx_core::Place;

use super::heirless_refusals;

#[test]
fn declared_places_of_todays_kinds() {
    assert_eq!(sys_frm::FIRM.place, Place::Zone, "a firm's zone is a word of its store");
    assert_eq!(sys_dem::HOUSEHOLD_KIND.place, Place::Zone, "a household's zone is a word of its store");
    assert_eq!(phx_core::ESTATE_KIND.place, Place::Country { word: 0 });
    for site in [sys_cb::CENTRAL_BANK, sys_cb::TREASURY, sys_bnk::BANK, sys_soc::AGENCY] {
        assert_eq!(site.place, Place::Site { word: 0 }, "{} is begun with its site's tile first", site.name);
    }
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
