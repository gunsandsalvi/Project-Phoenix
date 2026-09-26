//! A searcher's applications.

use if_labour::decisions::SearchIn;
use phx_macros::clause;

/// The vacancies a searcher applies to: among those paying more than its reservation, the ones it values most by the
/// weight of their wage's log and its taste for each, as many as it sends a round. With tastes drawn from a Gumbel,
/// taking the best is a draw without replacement from the logit over them.
#[clause("LAB.5", "LAB.8", "REP.22")]
#[must_use]
pub fn search(i: &SearchIn) -> Vec<u32> {
    let mut valued: Vec<(f64, u32)> = i
        .wages
        .iter()
        .zip(&i.tastes)
        .zip(0_u32..)
        .filter(|((w, _), _)| **w > i.reservation)
        .map(|((w, t), k)| (i.wage_weight * libm::log(*w) + t, k))
        .collect();
    valued.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    if let Ok(n) = usize::try_from(i.applications) {
        valued.truncate(n);
    }
    valued.into_iter().map(|(_, k)| k).collect()
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::SearchIn;

    use super::search;

    #[test]
    fn applications_go_to_the_best_valued_above_the_reservation() {
        let i = SearchIn {
            wages: vec![900.0, 1_500.0, 1_200.0, 2_000.0],
            tastes: vec![5.0, 0.0, 0.0, -0.2],
            reservation: 1_000.0,
            applications: 2,
            wage_weight: 1.0,
        };
        assert_eq!(search(&i), vec![3, 1], "the dearest two; the first pays less than the reservation");
        let taste = SearchIn { tastes: vec![0.0, 0.0, 1.0, 0.0], ..i.clone() };
        assert_eq!(search(&taste), vec![2, 3], "a strong enough taste outweighs a wage");
        assert!(search(&SearchIn { reservation: 5_000.0, ..i }).is_empty(), "nothing pays enough");
    }
}
