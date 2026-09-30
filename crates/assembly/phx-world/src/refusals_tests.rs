//! The opening's primitives are there for everything the world holds, or the world is refused naming what is missing;
//! a zero the dataset writes is a value.
#![cfg(test)]

use crate::consts::final_use::TAXED;
use crate::consts::firm::PUBLIC_ADMINISTRATION;
use crate::core_goods::CoreGoods;
use crate::opening::economy::{Flows, entry, shape_breaks};

use super::{leads, lending_rate, opening_price};

/// Flows of the shape the opening reads: every activity through public administration, every final use through
/// those bought at a sale, each part of value added.
fn shaped() -> Flows {
    let (n, finals) = (PUBLIC_ADMINISTRATION + 1, TAXED + 1);
    Flows {
        output: vec![1.0; n],
        inputs: vec![vec![0.0; n]; n],
        taxes: vec![0.0; n + finals],
        added: vec![vec![0.5, 0.5]; n],
        finals: vec![vec![0.25; finals]; n],
    }
}

#[test]
fn missing_lead_refused() {
    let refused = leads(3, &|p| (p != 1).then_some(5));
    assert_eq!(refused, Err("TEC.lead_time: product 1 has none".to_owned()));
    assert_eq!(leads(2, &|p| Some(p + 3)), Ok(vec![3.0, 4.0]));
}

#[test]
fn missing_rate_refused() {
    assert_eq!(lending_rate(None, 2), Err("GEN.lending_rate: country 2 has none".to_owned()));
    assert_eq!(lending_rate(Some(0.04), 2), Ok(0.04));
}

#[test]
fn short_accounts_row_refused() {
    assert_eq!(shape_breaks(&shaped()), Vec::<String>::new());
    let mut short = shaped();
    short.inputs[PUBLIC_ADMINISTRATION].pop();
    let breaks = shape_breaks(&short);
    assert_eq!(
        breaks,
        vec![format!("TEC.inputs: row {PUBLIC_ADMINISTRATION} has {PUBLIC_ADMINISTRATION} columns for 22")]
    );
    let few = Flows { finals: vec![vec![0.25; TAXED]; PUBLIC_ADMINISTRATION + 1], ..shaped() };
    assert!(shape_breaks(&few).iter().any(|b| b.starts_with("GEN.final_weights")), "a final use the sale reads");
}

#[test]
fn missing_opening_price_refused() {
    let prices = vec![vec![2.0, 3.0]];
    assert_eq!(opening_price(&prices, (0, 1)), Ok(3.0));
    assert!(opening_price(&prices, (0, 2)).is_err(), "a product the country has no price for");
    assert!(opening_price(&prices, (1, 0)).is_err(), "a country the prices do not hold");
}

#[test]
fn lead_read_beyond_products_stops() {
    let goods = CoreGoods { lead: vec![2.0], ..CoreGoods::default() };
    assert_eq!(goods.lead_of(0).to_bits(), 2.0_f64.to_bits());
    assert!(std::panic::catch_unwind(|| goods.lead_of(1)).is_err());
}

#[test]
fn zero_in_dataset_is_read_as_zero() {
    assert_eq!(leads(2, &|_| Some(0)), Ok(vec![0.0, 0.0]));
    assert_eq!(opening_price(&[vec![0.0]], (0, 0)), Ok(0.0));
    assert_eq!(entry(&[0.0], 0).to_bits(), 0.0_f64.to_bits());
}

#[test]
fn no_maker_reads_no_lead() {
    // A lead is the product's, whoever makes it: compiled for every product, and read only by a firm that makes it.
    let compiled = leads(3, &|p| Some(p)).unwrap();
    assert_eq!(compiled.len(), 3);
    let goods = CoreGoods { lead: compiled, ..CoreGoods::default() };
    assert_eq!(goods.lead_of(2).to_bits(), 2.0_f64.to_bits());
}
