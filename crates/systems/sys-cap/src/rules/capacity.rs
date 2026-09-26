//! The output a year a holder's plant allows a way: the scarcest kind's.

use phx_macros::clause;
use phx_num::{Missing, violation};

/// The output a year a way can make from the plant a holder has: for each kind the way needs, the efficient units of
/// it held (each class's units times its efficiency) over the way's plant of it per unit of output a year, and the
/// least of these. `Absent` for a way that needs no plant, which plant does not limit.
#[clause("CAP.9", "CAP.1")]
pub fn capacity(efficient_held: &[f64], needs: &[(usize, f64)]) -> Missing<f64> {
    let mut least = Missing::Absent;
    for (kind, per_unit) in needs.iter().filter(|(_, per_unit)| *per_unit > 0.0) {
        let Some(held) = efficient_held.get(*kind).copied() else {
            violation!(clause = "CAP.1", "a way's plant of a kind its holder's plant is not read for", kind = *kind);
        };
        let allows = held / per_unit;
        least = match least {
            Missing::Present(l) if l <= allows => Missing::Present(l),
            _ => Missing::Present(allows),
        };
    }
    least
}

/// The efficient units of a kind: each class's units times the efficiency its units work at.
#[clause("CAP.1", "REP.24")]
#[must_use]
pub fn efficient_units(units: &[i64], efficiencies: &[f64]) -> f64 {
    units.iter().zip(efficiencies).map(|(u, e)| phx_rand::float::from_i64(*u) * e).sum()
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::{capacity, efficient_units};

    #[test]
    fn capacity_is_the_scarcest_kind() {
        let held = [1_000.0, 50.0, 0.0];
        assert_eq!(capacity(&held, &[(0, 10.0), (1, 1.0)]), Missing::Present(50.0), "the second kind binds");
        assert_eq!(capacity(&held, &[(0, 10.0)]), Missing::Present(100.0));
        assert_eq!(capacity(&held, &[(0, 10.0), (2, 1.0)]), Missing::Present(0.0), "a needed kind not held stops it");
        assert_eq!(capacity(&held, &[(2, 0.0)]), Missing::Absent, "no plant needed, no limit from plant");
        assert!((efficient_units(&[100, 50], &[0.9, 0.5]) - 115.0).abs() < 1e-9);
    }
}
