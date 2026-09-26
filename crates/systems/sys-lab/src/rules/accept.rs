//! A searcher's answer to an offer.

use if_labour::decisions::AcceptIn;
use phx_macros::clause;

/// Whether a searcher accepts an offer: when the weight of its wage's log over its reservation's, with the match's
/// quality drawn as its taste, is more than nothing. A searcher that reserves nothing takes any offer.
#[clause("LAB.5", "LAB.8", "REP.22")]
#[must_use]
pub fn accept(i: &AcceptIn) -> bool {
    if i.reservation <= 0.0 {
        return true;
    }
    i.wage > 0.0 && i.wage_weight * libm::log(i.wage / i.reservation) + i.taste > 0.0
}

#[cfg(test)]
mod tests {
    use if_labour::decisions::AcceptIn;

    use super::accept;

    #[test]
    fn reservation_components() {
        let offer = AcceptIn { wage: 1_100.0, reservation: 1_000.0, taste: 0.0, wage_weight: 2.0 };
        assert!(accept(&offer), "above the reservation with a neutral match");
        assert!(!accept(&AcceptIn { taste: -0.5, ..offer }), "a poor match outweighs a small gain");
        assert!(accept(&AcceptIn { wage: 950.0, taste: 0.2, ..offer }), "a good match makes up a small loss");
        assert!(accept(&AcceptIn { reservation: 0.0, taste: -9.0, ..offer }), "reserving nothing, anything");
    }
}
