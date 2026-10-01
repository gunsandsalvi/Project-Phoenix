//! Kinds and families are bound by their declarations, never by a name's prefix, and bind the same however often.
#![cfg(test)]

use phx_id::Day;
use phx_store::AddressSpace;

use super::Bound;
use crate::consts::AGENT_ROWS_PER_CHUNK;
use crate::consts::families::{EMPLOYMENT, FIRM_LOANS, PUBLIC_EMPLOYMENT};
use crate::core_day::DatedFamily;

/// A family opened under a name, between parties of two kinds.
fn family(name: &'static str, kinds: [u8; 2]) -> DatedFamily {
    let rows = AGENT_ROWS_PER_CHUNK;
    crate::core_taxes::collectors_family(&mut AddressSpace::empty(), (name, kinds), ([rows, rows], rows), Day::new(0))
}

/// The fixtures' kinds, each at the place its families' sides name.
fn names() -> Vec<&'static str> {
    [sys_cb::CENTRAL_BANK, sys_cb::TREASURY, sys_bnk::BANK, sys_frm::FIRM, phx_core::ESTATE_KIND]
        .iter()
        .chain(&[sys_dem::HOUSEHOLD_KIND, sys_soc::AGENCY])
        .map(|k| k.name)
        .collect()
}

fn opened() -> Vec<DatedFamily> {
    vec![
        family(EMPLOYMENT, [3, 5]),
        family(FIRM_LOANS, [3, 2]),
        family(PUBLIC_EMPLOYMENT, [6, 5]),
        family("LAB.other", [3, 5]),
    ]
}

#[test]
fn jobs_families_by_flag_not_prefix() {
    let b = Bound::of(&names(), &opened());
    assert_eq!(b.jobs, [true, false, true, false], "a family named like labour's is no job unless declared one");
}

#[test]
fn debt_families_exclude_jobs() {
    let b = Bound::of(&names(), &opened());
    let debts: Vec<usize> = (0..b.jobs.len()).filter(|i| !b.is_jobs(*i)).collect();
    assert_eq!(debts, [1, 3]);
}

#[test]
fn employer_family_by_kind() {
    let fams = opened();
    let b = Bound::of(&names(), &fams);
    let agency = u8::try_from(b.kinds.agency.unwrap()).unwrap();
    let of = fams.iter().enumerate().position(|(i, f)| b.is_jobs(i) && f.store.kinds.first() == Some(&agency));
    assert_eq!(of, Some(2));
    assert_eq!(
        (b.families.employment, b.families.public_employment, b.families.firm_loans),
        (Some(0), Some(2), Some(1))
    );
}

#[test]
fn route_to_unbound_kind_refused() {
    let without_agency: Vec<&str> = names().into_iter().filter(|n| *n != sys_soc::AGENCY.name).collect();
    let b = Bound::of(&without_agency, &[]);
    assert_eq!(b.kinds.agency, None, "a world with no agency binds none");
    assert_eq!(b.families.employment, None);
    assert!(std::panic::catch_unwind(|| b.is_jobs(0)).is_err(), "a family the core never opened");
}

#[test]
fn bound_rebuilt_equal() {
    assert_eq!(Bound::of(&names(), &opened()), Bound::of(&names(), &opened()));
}

#[test]
fn bound_independent_of_declaration_lookup_order() {
    let mut reversed = names();
    reversed.reverse();
    let (b, r) = (Bound::of(&names(), &[]), Bound::of(&reversed, &[]));
    let last = names().len() - 1;
    assert_eq!(r.kinds.firm, b.kinds.firm.map(|f| last - f), "each kind bound to its own place");
    assert_eq!(r.kinds.household, b.kinds.household.map(|h| last - h));
}
