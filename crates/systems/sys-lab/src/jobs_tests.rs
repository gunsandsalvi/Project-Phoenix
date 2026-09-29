//! The rank match written out by hand: three skills filling three levels of work.
#![cfg(test)]

use std::collections::BTreeMap;

use super::by_rank;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

/// 60 persons of skill 1, 30 of 2 and 10 of 4 against work asking 10 of level 1, 80 of level 2 (a quarter of it the
/// self-employed's) and 10 of level 4.
#[test]
fn the_least_skilled_fill_the_least_skilled_work_first() {
    let supply = BTreeMap::from([(1, 60.0), (2, 30.0), (4, 10.0)]);
    let asked = [(1, [10.0, 0.0]), (2, [60.0, 20.0]), (4, [10.0, 0.0])];
    let m = by_rank(&supply, &asked);
    let at = |skill: u32, o: usize, w: usize| m[&skill][o][w];
    assert!(close(at(1, 0, 0), 1.0 / 6.0) && close(at(1, 1, 0), 0.625) && close(at(1, 1, 1), 50.0 / 240.0));
    assert!(close(at(2, 1, 0), 0.75) && close(at(2, 1, 1), 0.25) && close(at(2, 0, 0), 0.0));
    assert!(close(at(4, 2, 0), 1.0) && close(at(4, 1, 0), 0.0));
}

#[test]
fn asked_hours_are_scaled_to_the_persons() {
    let supply = BTreeMap::from([(1, 60.0), (2, 30.0), (4, 10.0)]);
    let halved = by_rank(&supply, &[(1, [10.0, 0.0]), (2, [60.0, 20.0]), (4, [10.0, 0.0])]);
    let doubled = by_rank(&supply, &[(1, [40.0, 0.0]), (2, [240.0, 80.0]), (4, [40.0, 0.0])]);
    for (a, b) in halved.values().flatten().flatten().zip(doubled.values().flatten().flatten()) {
        assert!(close(*a, *b));
    }
}

#[test]
fn each_skills_chances_sum_to_one() {
    let supply = BTreeMap::from([(1, 5.0), (2, 70.0), (4, 25.0)]);
    let m = by_rank(&supply, &[(1, [30.0, 0.0]), (2, [20.0, 20.0]), (3, [10.0, 0.0]), (4, [20.0, 0.0])]);
    for chances in m.values() {
        assert!(close(chances.iter().flatten().sum(), 1.0));
    }
}

#[test]
fn nothing_matched_where_nothing_is_asked() {
    assert!(by_rank(&BTreeMap::from([(1, 5.0)]), &[(1, [0.0, 0.0])]).is_empty());
}
