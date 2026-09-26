//! Wage points: the round monthly wages the trade offers and pays at, each the ratio between neighbouring points to
//! the point's power.

use if_labour::law::Law;
use phx_macros::clause;

/// The monthly wage at a point.
#[clause("REP.34")]
#[must_use]
pub fn wage_at(law: &Law, point: i64) -> f64 {
    libm::pow(law.point_ratio, phx_rand::float::from_i64(point))
}

/// The point nearest a monthly wage; none for a wage no point reaches.
#[clause("REP.34")]
#[must_use]
pub fn point_near(law: &Law, wage: f64) -> Option<i64> {
    phx_rand::float::floor_to_i64(libm::rint(libm::log(wage) / libm::log(law.point_ratio)))
}

/// The least point that pays at least a monthly wage.
#[clause("REP.34", "LAB.11")]
#[must_use]
pub fn least_point(law: &Law, wage: f64) -> Option<i64> {
    phx_rand::float::floor_to_i64(libm::ceil(libm::log(wage) / libm::log(law.point_ratio)))
}

#[cfg(test)]
mod tests {
    use if_labour::law::Law;
    use phx_num::Missing;

    use super::{least_point, point_near, wage_at};

    fn law() -> Law {
        Law {
            full_time_hours: 40,
            notice_days: 30,
            severance_days_a_year: 7,
            mean_monthly: 300_000.0,
            minimum_monthly: Missing::Absent,
            point_ratio: 1.25,
            patience_days: 19,
            applications_a_week: 2.0,
            seen_chance: 0.04,
            wage_weight: 4.0,
            reservation_share: 0.9,
            weeks_a_month: 4.35,
            band_years: 5,
            occupation_skill: Vec::new(),
            education_skill: Vec::new(),
            pension_months: Vec::new(),
        }
    }

    #[test]
    fn points_round_trip_and_the_least_pays_enough() {
        let l = law();
        let p = point_near(&l, 300_000.0).expect("a point");
        assert_eq!(p, 57, "1.25 to the 57th is about 330 000, the nearest point");
        assert!((wage_at(&l, p) / 300_000.0 - 1.0).abs() < 0.125, "within half a step");
        let least = least_point(&l, 300_000.0).expect("a point");
        assert!(wage_at(&l, least) >= 300_000.0 && wage_at(&l, least - 1) < 300_000.0);
    }
}
