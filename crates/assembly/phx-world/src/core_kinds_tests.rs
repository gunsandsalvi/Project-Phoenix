//! Each kind's traits are read from its legal form and its place from its declaration; the core's kinds are the ones
//! the build declares.
#![cfg(test)]

use phx_core::kinds::{Feature, Owners};
use phx_core::{KindDecl, LegalForm, Place};
use phx_id::{CountryId, PartyKey, Slot};
use phx_num::Missing;

use super::{country_by_place, heirless_party, traits};
use crate::consts::stats::MONEY_CLASSES;

fn form(name: &str, may_hold: &[&str], features: &[Feature], owners: Owners) -> LegalForm {
    LegalForm {
        name: name.to_owned(),
        may_hold: may_hold.iter().map(|h| (*h).to_owned()).collect(),
        features: features.to_vec(),
        endings: vec!["dissolution".to_owned()],
        owners,
        offices: Vec::new(),
    }
}

/// Today's forms, as the law declares their features and owners.
fn forms() -> Vec<LegalForm> {
    use Feature::{HasOwners, IssuesCurrency, LimitedLiability, SeparateParty, TakesDeposits};
    vec![
        form("central bank", &["money", "loans"], &[SeparateParty, IssuesCurrency], Owners::State),
        form("treasury", &["money"], &[SeparateParty], Owners::State),
        form(
            "bank",
            &["money", "loans"],
            &[SeparateParty, LimitedLiability, TakesDeposits, HasOwners],
            Owners::Shareholders,
        ),
        form("company", &["money", "stocks"], &[SeparateParty, LimitedLiability, HasOwners], Owners::Shareholders),
        form("estate", &["money"], &[SeparateParty], Owners::HeirsAndCreditors),
        form("household", &["money", "dwellings"], &[], Owners::Members),
        form("natural person", &[], &[], Owners::Nobody),
        form("public agency", &["money", "plant"], &[SeparateParty], Owners::State),
    ]
}

const HOLDERS: [&str; 2] = ["household", "company"];

/// The build's kinds, in a fixed order of the fixture's own, each reserving one row.
const TODAYS: [KindDecl; 7] = [
    sys_cb::CENTRAL_BANK,
    sys_cb::TREASURY,
    sys_bnk::BANK,
    sys_frm::FIRM,
    phx_core::ESTATE_KIND,
    sys_dem::HOUSEHOLD_KIND,
    sys_soc::AGENCY,
];

fn of_kinds() -> Vec<super::KindTraits> {
    traits(&TODAYS.map(|k| (k, 1)), &forms(), &HOLDERS).unwrap()
}

#[test]
fn equity_account_by_has_owners() {
    let owned: Vec<bool> = of_kinds().iter().map(|t| t.has_owners).collect();
    assert_eq!(owned, [false, false, true, true, false, false, false], "the bank and the firm");
}

#[test]
fn accounts_opened_by_may_hold() {
    let held = |may_hold: &[&str]| {
        let k = KindDecl { name: "x", legal_form: "x", place: Place::Site, store: "institutions", clause: "PTY.4" };
        traits(&[(k, 1)], &[form("x", may_hold, &[], Owners::State)], &HOLDERS).unwrap()[0].holds_money
    };
    assert!(held(&["money", "plant"]));
    assert!(!held(&["plant", "stocks"]), "a form that may hold no money opens no account");
}

#[test]
fn holds_money_unless_issuer() {
    let holds: Vec<bool> = of_kinds().iter().map(|t| t.holds_money).collect();
    assert_eq!(holds, [false, true, true, true, true, true, true], "every kind but the issuer of the currency");
}

#[test]
fn issuer_class_by_form() {
    let t = of_kinds();
    let state: Vec<bool> = t.iter().map(|t| t.owners == Owners::State).collect();
    assert_eq!(state, [true, true, false, false, false, false, true]);
    let deposits: Vec<bool> = t.iter().map(|t| t.takes_deposits).collect();
    assert_eq!(deposits, [false, false, true, false, false, false, false]);
}

#[test]
fn money_stock_class_by_form() {
    let classes: Vec<usize> = of_kinds().iter().map(|t| t.money_class).collect();
    let other = MONEY_CLASSES - 1;
    assert_eq!(classes, [other, other, 0, 2, other, 1, other], "reserves, then households' and firms' deposits");
}

#[test]
fn form_outside_holder_classes_counts_other() {
    let k = KindDecl { name: "fund", legal_form: "fund", place: Place::Site, store: "institutions", clause: "PTY.4" };
    let fund = traits(&[(k, 1)], &[form("fund", &["money"], &[], Owners::Shareholders)], &HOLDERS).unwrap();
    assert_eq!(fund[0].money_class, MONEY_CLASSES - 1);
    let unknown = KindDecl { legal_form: "church", ..k };
    assert!(traits(&[(unknown, 1)], &forms(), &HOLDERS).is_err(), "a form the law does not declare");
}

#[test]
fn unbanked_household_bank_missing() {
    assert_eq!(super::banked(phx_core::settle::AT_ISSUER), Missing::Absent, "an account at the issuer banks nowhere");
    assert_eq!(super::banked(4), Missing::Present(4));
}

#[test]
fn country_by_declared_place() {
    let regions = [CountryId::new(0), CountryId::new(2)];
    let tile = |t: u32| (t == 7).then_some(1);
    let read = |place, at| country_by_place(place, Some(at), &regions, tile);
    assert_eq!(read(Place::Region, 1), Some(2), "region 1 lies in country 2");
    assert_eq!(read(Place::Zone, 1), None, "a zone is its store's to read");
    assert_eq!(read(Place::Country, 1), Some(1));
    assert_eq!(read(Place::Site, 7), Some(1), "tile 7 lies in country 1");
    assert_eq!(country_by_place(Place::Country, None, &regions, tile), None, "a party placed nowhere");
}

#[test]
fn heirless_to_the_declared_institution() {
    let key = |kind: u8| PartyKey::new(kind, Slot::new(0));
    let institutions = [Some(key(0)), Some(key(1)), None];
    assert_eq!(heirless_party(&institutions, 1), Some(key(1)));
    assert_eq!(heirless_party(&institutions, 6), None, "a country with no such institution has no destination");
}

#[test]
fn stores_built_from_catalogue_order() {
    let mut d = phx_core::Declarations::new();
    let _ = crate::compile::KernelPrims::declare(&mut d);
    for system in crate::systems::SYSTEMS {
        phx_core::declare_entry(&system(), &mut d);
    }
    let codes = phx_core::catalogue::FamilyCodes::default();
    let catalogue = crate::registry::world_catalogue(&d, &forms(), &codes).unwrap();
    let mut declared: Vec<&str> = d.kinds.iter().map(|(_, k)| k.name).collect();
    declared.sort_unstable();
    let compiled: Vec<&str> = catalogue.names.kinds.iter().map(AsRef::as_ref).collect();
    assert_eq!(compiled, declared, "every kind the build declares, in name order, whatever the registration order");
    let rows = |store: &str| phx_core::capacity::table().find(|c| c.store == store).unwrap().rows;
    for (h, name) in catalogue.kinds().zip(&catalogue.names.kinds) {
        let (_, decl) = d.kinds.iter().find(|(_, k)| k.name == &**name).unwrap();
        assert_eq!(catalogue.kind(h).rows, rows(decl.store), "`{name}` reserves its own store's capacity");
    }
}

#[test]
fn place_store_keeps_declared_place() {
    use phx_id::Day;
    use phx_pop::directory::Directory;

    use crate::place_store::PlaceStore;

    // Each place a kind declares keeps its party's word at the party's slot; a zone is its own store's.
    let mut space = phx_store::AddressSpace::empty();
    let rows = crate::consts::KIND_ROWS_PER_CHUNK;
    let mut dir: Directory = Directory::new(&mut space, &[rows, rows, rows], rows, (Day::new(0), 2));
    assert!(PlaceStore::new(&mut space, 0, Place::Zone, rows).is_none());
    for (kind, (place, at)) in (0_u8..).zip([(Place::Site, 70_000), (Place::Region, 3), (Place::Country, 2)]) {
        let mut store = PlaceStore::new(&mut space, kind, place, rows).unwrap();
        let (a, b) = (dir.begin(kind), dir.begin(kind));
        store.begin(&dir, a, at);
        assert_eq!((store.place(), store.at(a.slot()), store.at(b.slot())), (place, Some(at), None));
    }
    let mut countries = PlaceStore::new(&mut space, 2, Place::Country, rows).unwrap();
    let r = dir.begin(2);
    let past = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| countries.begin(&dir, r, 256)));
    assert!(past.is_err(), "a country past its word stops the run");
}

#[test]
fn reference_names_party_without_table() {
    use phx_id::Day;
    use phx_num::Missing;
    use phx_pop::directory::{Directory, Resolved};

    use crate::place_store::PlaceStore;

    // An estate's party, begun at the directory's slot and ended into its estate, is named by its reference alone.
    let mut space = phx_store::AddressSpace::empty();
    let rows = crate::consts::KIND_ROWS_PER_CHUNK;
    let mut dir: Directory = Directory::new(&mut space, &[rows, rows], rows, (Day::new(0), 2));
    let mut firms = PlaceStore::new(&mut space, 0, Place::Region, rows).unwrap();
    let firm = dir.begin(0);
    firms.begin(&dir, firm, 7);
    let estate = dir.begin(1);
    assert_eq!(dir.resolve(firm), Resolved::Live(firm.slot()));
    dir.end(firm, Day::new(1), Missing::Present(estate));
    assert_eq!(dir.resolve(firm), Resolved::Ended { day: Day::new(1), successor: Missing::Present(estate) });
    let _ = dir.close_day(Day::new(1));
    let again = dir.begin(0);
    firms.begin(&dir, again, 8);
    assert_eq!(again.slot(), firm.slot(), "the slot is handed out again after the day closes");
    assert_eq!(firms.at(again.slot()), Some(8), "the party begun again takes its own place");
    assert_ne!(again, firm, "at its next generation, so the old reference still names the ended party");
    assert_eq!(dir.follow(firm), (estate, Resolved::Live(estate.slot())));
}
