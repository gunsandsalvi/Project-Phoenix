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
