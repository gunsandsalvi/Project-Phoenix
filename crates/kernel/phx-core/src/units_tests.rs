//! Capital's arithmetic over a holder or two: wear down a chain at the values' ratio with the service day kept, the
//! last condition leaving, the units conserved but for those that leave, and capacity from the conditions.
#![cfg(test)]

use phx_id::{Day, PartyKey, Slot};

use super::{Chain, Class, capacity, issue_chain, wear};
use crate::goods::{Held, NATURE, Stocks, find, nature_net, unit};
use crate::unit_registry::{UnitRegistry, UnitTraits};

fn firm(i: u32) -> PartyKey {
    PartyKey::new(2, Slot::new(i))
}

fn chain() -> Chain {
    Chain { leaving_per_year: 1.0, efficiency: vec![1.0, 0.8, 0.5], value: vec![1.0, 0.6, 0.3] }
}

fn class(condition: u8) -> Held {
    Held::Capital(Class { kind: 4, band: 0, condition, zone: 7 })
}

#[test]
fn wear_moves_units_down_the_chain_at_the_values_ratio() {
    let (mut stocks, mut ids) = (Stocks::default(), UnitRegistry::with_capacity(16));
    let units = issue_chain(&mut ids, Class { kind: 4, band: 0, condition: 0, zone: 7 }, 3);
    let (new, old) = (unit(&mut ids, class(0), UnitTraits::NONE), unit(&mut ids, class(2), UnitTraits::NONE));
    assert_eq!(units, vec![new, 1, old], "a chain's conditions issued together");
    stocks.receive(firm(1), new, (1_000, 100_000), Day::new(0));
    stocks.receive(firm(1), old, (100, 3_000), Day::new(0));
    let c = chain();
    let mut out = Vec::new();
    let open = stocks.totals();
    wear((&mut stocks, &ids), firm(1), |k| (k == 4).then_some(&c), (365, 365), 5, &mut out);
    // A year at one a year takes 1 − e^−1 of each class: 632 of the new, 63 of the oldest.
    let next = find(&ids, class(1)).unwrap();
    let held = |u| stocks.holding(firm(1), u).map(|h| (h.units, h.cost, h.day));
    assert_eq!(held(new), Some((368, 36_800, 0)));
    assert_eq!(held(next), Some((632, 37_920, 0)), "63 200 of cost at 0.6 over 1.0; the service day kept");
    assert_eq!(held(old), Some((37, 1_110, 0)), "the last condition's leave the chain");
    let legs: Vec<(PartyKey, PartyKey, u16, i64)> =
        out.iter().map(|f| (f.payer, f.payee, f.denomination.unit(), f.amount)).collect();
    assert!(legs.contains(&(firm(1), NATURE, new, 632)) && legs.contains(&(NATURE, firm(1), next, 632)));
    assert!(legs.contains(&(firm(1), NATURE, old, 63)) && legs.len() == 3, "a unit leaving has no arriving leg");
    let made = nature_net(&out);
    assert_eq!(crate::goods::breaks(&open, &made, &stocks.totals()), vec![], "the goods' identity holds for capital");
}

#[test]
fn capacity_reads_each_condition_at_its_efficiency() {
    let (mut stocks, mut ids) = (Stocks::default(), UnitRegistry::with_capacity(16));
    for (cond, units) in [(0, 10), (1, 10), (2, 4)] {
        let u = unit(&mut ids, class(cond), UnitTraits::NONE);
        stocks.receive(firm(1), u, (units, units * 100), Day::new(0));
    }
    let far = unit(&mut ids, Held::Capital(Class { kind: 4, band: 0, condition: 0, zone: 8 }), UnitTraits::NONE);
    stocks.receive(firm(1), far, (5, 500), Day::new(0));
    let c = chain();
    assert!((capacity(&stocks, &ids, firm(1), (4, Some(7)), &c) - 20.0).abs() < 1e-12, "10 + 8 + 2 here");
    assert!((capacity(&stocks, &ids, firm(1), (4, None), &c) - 25.0).abs() < 1e-12, "and 5 elsewhere");
    assert!(capacity(&stocks, &ids, firm(2), (4, None), &c).abs() < 1e-12, "none held, none to do");
}
