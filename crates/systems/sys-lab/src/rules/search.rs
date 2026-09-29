//! A searcher's applications.

use if_labour::decisions::SearchIn;
use phx_macros::clause;

/// The vacancies a searcher applies to: drawn one by one without replacement in proportion to their pulls, the logit
/// over the weight of their wage's log with a standard Gumbel taste for each.
#[clause("LAB.5", "LAB.8", "REP.22")]
#[must_use]
pub fn search(i: &SearchIn) -> Vec<u32> {
    phx_market::hiring::pick(&i.reach, &i.draws)
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::SearchIn;

    use super::search;

    #[test]
    fn applications_are_drawn_in_proportion_to_their_pulls() {
        let reach = vec![(4, 1.0), (7, 3.0)];
        assert_eq!(
            search(&SearchIn { reach: reach.clone(), draws: vec![0.5] }),
            vec![7],
            "the draw falls in the pull of 3"
        );
        assert_eq!(search(&SearchIn { reach: reach.clone(), draws: vec![0.1, 0.5] }), vec![4, 7], "then the one left");
        assert_eq!(search(&SearchIn { reach, draws: vec![0.9, 0.9, 0.9] }).len(), 2, "no more than it weighs");
        assert!(search(&SearchIn { reach: Vec::new(), draws: vec![0.5] }).is_empty(), "nothing in reach");
    }
}
