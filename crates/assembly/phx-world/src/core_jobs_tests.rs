//! The jobs' deal: largest remainder over the firms' hours, whole and in proportion.
#![cfg(test)]

use super::deal;

const JOBS: u64 = 10;
const LOPSIDED: [f64; 3] = [1.0, 1.0, 1.0];

#[test]
fn every_job_is_dealt_in_proportion() {
    assert_eq!(deal(JOBS, &[3.0, 1.0, 1.0]), vec![6, 2, 2]);
    assert_eq!(deal(JOBS, &LOPSIDED), vec![4, 3, 3], "the remainder to the earlier on a tie");
    assert_eq!(deal(JOBS, &[0.0, 0.0]), vec![0, 0], "no hours, no jobs");
    assert_eq!(deal(JOBS, &LOPSIDED).iter().sum::<u64>(), JOBS);
}

#[test]
fn wages_share_each_activitys_compensation_by_pay() {
    use std::collections::BTreeMap;

    use super::{activity_rates, occupation_wages, wage_in};
    let compensation = [900.0, 400.0];
    let pay = [1.0, 2.0];
    let hours = BTreeMap::from([((0, 0), 100.0), ((0, 1), 100.0), ((1, 1), 50.0)]);
    let rates = activity_rates(&compensation, &pay, &hours);
    let at = |k| wage_in(&rates, &pay, k).unwrap_or(f64::NAN);
    assert!((at((0, 0)) - 3.0).abs() < 1e-12, "900 over 100 hours at one and 100 at two");
    assert!((at((0, 1)) - 6.0).abs() < 1e-12);
    assert!((at((1, 1)) - 8.0).abs() < 1e-12);
    assert!((at((1, 0)) - 4.0).abs() < 1e-12, "an occupation the activity has no job of, at its pay");
    let paid: f64 = hours.iter().map(|(k, h)| at(*k) * h).sum();
    assert!((paid - 1300.0).abs() < 1e-9, "the wages are the compensation");
    let by = occupation_wages(&rates, &pay, &hours);
    assert!((by[&1] - (6.0 * 100.0 + 8.0 * 50.0) / 150.0).abs() < 1e-12);
    assert!(
        activity_rates(&compensation, &pay, &BTreeMap::from([((2, 0), 10.0)])).is_empty(),
        "no compensation, no rate"
    );
}
