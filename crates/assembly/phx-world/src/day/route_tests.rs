//! The routes over the stage table: every call made once a day, business or not, in the table's order; day zero
//! routes stage 5 alone; a heavy day routes the business day's slots.
#![cfg(test)]

use phx_core::stages::{DAY_TABLE, Mode, compile};

use super::{Call, acts, calls};

/// The calls a day makes, in order, for its walk of the table.
fn made(mode: Mode, any_business: bool) -> Vec<Call> {
    let table = compile(&DAY_TABLE).unwrap();
    let walked: Vec<_> = table.walk(mode, any_business).map(|s| s.slot).collect();
    walked.into_iter().flat_map(|s| calls(s).iter().copied()).filter(|c| acts(*c, any_business)).collect()
}

#[test]
fn routes_cover_every_running_slot() {
    use Call::*;
    let every = [Weather, Rates, Hazards, Windows, Labour, Goods, Freight, Settle, Publish, Audit, Statistics, Player];
    assert_eq!(made(Mode::Ordinary, true), every, "a business day makes every call once, in the table's order");
    for (slot, routed) in super::ROUTES.iter().copied() {
        assert!(
            DAY_TABLE.iter().any(|s| s.slot == slot && !s.runs.is_empty()),
            "{slot:?} routes calls but runs nothing"
        );
        assert!(!routed.is_empty());
    }
}

#[test]
fn nb_day_runs_nb_slots() {
    use Call::*;
    let closed =
        [Weather, Rates, Hazards, Windows, Labour, Goods, Freight, SettleClosed, Publish, Audit, Statistics, Player];
    assert_eq!(made(Mode::Ordinary, false), closed, "settlement records the closed day's commitments at 6d");
}

#[test]
fn day_zero_routes_stage_five() {
    assert_eq!(made(Mode::DayZero, true), [Call::Windows, Call::Labour]);
}

#[test]
fn heavy_day_routes_same_slots() {
    assert_eq!(made(Mode::Settling, true), made(Mode::Ordinary, true));
}
