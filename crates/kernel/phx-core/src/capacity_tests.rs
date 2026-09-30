//! The capacity table's own arithmetic: rows cover the design point with two years' growth, the reservations fit the
//! address space, the wheel reaches past a quarter, and the family code holds 255 codes with holdings'.
#![cfg(test)]

use phx_store::consts::VA_BUDGET;

use super::{AGENT_ROWS, Growth, LINK_BITS, family_code, slot_fits, table};
use crate::consts::{CHUNK_ROWS, GROWTH_DIVISOR, GROWTH_YEARS, HOLDINGS_CODE, LONGEST_QUARTER_DAYS, WHEEL_DAYS};

#[test]
fn capacity_rows_cover_growth() {
    for c in table() {
        assert!(c.rows >= c.design_rows + GROWTH_YEARS * c.growth_per_year, "{}", c.store);
        assert_eq!(c.rows % CHUNK_ROWS, 0, "{} rounds to whole chunks", c.store);
        assert!(
            c.rows - (c.design_rows + GROWTH_YEARS * c.growth_per_year) < CHUNK_ROWS,
            "{} no more than a chunk over",
            c.store
        );
    }
    let persons = table().find(|c| c.store == "persons").unwrap();
    assert_eq!(persons.rows, AGENT_ROWS);
    assert!(persons.growth_per_year > 0, "the persons grow");
    let zones = table().find(|c| c.store == "zones").unwrap();
    assert_eq!(zones.growth_per_year, 0, "the map's geometry is fixed");
    assert_eq!(super::capacity(&("x", 1_000_000, 8, Growth::Persons)).growth_per_year, 40_000);
}

#[test]
fn reservations_fit_the_address_space() {
    let reserved: u64 = table().map(|c| u64::from(c.rows) * u64::from(c.row_bytes) * 2).sum();
    assert!(reserved <= u64::try_from(VA_BUDGET).unwrap(), "{reserved} bytes reserved against {VA_BUDGET}");
}

#[test]
fn wheel_horizon_covers_a_quarter() {
    let (wheel, quarter) = std::hint::black_box((WHEEL_DAYS, LONGEST_QUARTER_DAYS));
    assert!(wheel >= quarter, "the capacity module also refuses this at compile time");
}

#[test]
fn family_code_holds_255() {
    assert_eq!(family_code(0), Some(0));
    assert_eq!(family_code(254), Some(254));
    assert_eq!(family_code(255), None, "255 is holdings'");
    assert_eq!(family_code(300), None);
    assert_eq!(HOLDINGS_CODE, u8::MAX);
    assert!(slot_fits((1 << 24) - 1) && !slot_fits(1 << 24));
    assert_eq!(std::hint::black_box(LINK_BITS), u32::BITS);
}

#[test]
fn contract_families_fit_the_link() {
    // The design point's contract families, each with two years' growth, within a link's slots, and their codes
    // below the one holdings keep.
    let design: toml::Table = include_str!("../../../../perf/design.toml").parse().unwrap();
    let families = design["store"]["contracts"].as_table().unwrap();
    assert!(families.len() < usize::from(HOLDINGS_CODE), "{} families", families.len());
    for (name, rows) in families {
        let rows = u32::try_from(rows.as_integer().unwrap()).unwrap();
        let grown = rows + GROWTH_YEARS * rows.div_ceil(GROWTH_DIVISOR);
        assert!(slot_fits(grown - 1), "{name}: {grown} rows past a link's slots");
    }
}
