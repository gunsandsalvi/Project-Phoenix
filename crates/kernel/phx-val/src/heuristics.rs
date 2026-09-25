//! The heuristic menu, each rule a real decider uses, over the series as the party saw it published.

use phx_id::Day;
use phx_macros::clause;

/// Last outlook corrected toward what happened, at the party's own memory speed `lambda`.
#[clause("VAL.6")]
#[must_use]
pub fn adaptive(previous: f64, observed: f64, lambda: f64) -> f64 {
    previous + lambda * (observed - previous)
}

/// The recent change extrapolated by `gamma`.
#[clause("VAL.6")]
#[must_use]
pub fn trend(last: f64, before: f64, gamma: f64) -> f64 {
    last + gamma * (last - before)
}

/// A return toward a level the party has observed over a long window, or a published target, by `kappa`.
#[clause("VAL.6")]
#[must_use]
pub fn anchor(last: f64, level: f64, kappa: f64) -> f64 {
    last + kappa * (level - last)
}

/// A published, dated change: the level it sets from its effective day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Announced {
    pub effective: Day,
    pub level: f64,
}

/// The announced level for a horizon reaching its effective day, and `otherwise` for one ending before it.
#[clause("VAL.6")]
#[must_use]
pub fn announcement(otherwise: f64, announced: Announced, horizon_end: Day) -> f64 {
    if horizon_end >= announced.effective { announced.level } else { otherwise }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn adaptive_converges_to_constant() {
        let mut e = 0.0;
        for _ in 0..200 {
            e = adaptive(e, 3.0, 0.2);
        }
        assert!(close(e, 3.0));
        // Each step closes the same fraction of the gap.
        assert!(close(adaptive(1.0, 3.0, 0.25), 1.5));
    }

    #[test]
    fn trend_extrapolates() {
        assert!(close(trend(2.0, 1.0, 0.5), 2.5));
        assert!(close(trend(1.0, 2.0, 0.5), 0.5));
        assert!(close(trend(4.0, 4.0, 0.9), 4.0));
    }

    #[test]
    fn anchor_reverts() {
        assert!(close(anchor(5.0, 1.0, 0.25), 4.0));
        assert!(close(anchor(-1.0, 1.0, 0.5), 0.0));
        assert!(close(anchor(2.0, 2.0, 0.7), 2.0));
    }

    #[test]
    fn announcement_effective_day() {
        let a = Announced { effective: Day::new(30), level: 0.25 };
        assert!(close(announcement(0.2, a, Day::new(29)), 0.2));
        assert!(close(announcement(0.2, a, Day::new(30)), 0.25));
        assert!(close(announcement(0.2, a, Day::new(90)), 0.25));
    }
}
