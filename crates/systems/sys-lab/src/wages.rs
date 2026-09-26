//! Wage points: the round monthly wages the trade offers and pays at, each the ratio between neighbouring points to
//! the point's power.

use if_labour::law::Law;
use phx_macros::clause;
use phx_num::Missing;

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

/// The point an employer offers for work of an occupation, from its own fill history: its last fill's point, a point
/// lower when that fill came as quickly as a match can; with no fill, its newest staff's; with neither, the point of
/// the mean wage.
#[clause("LAB.4", "REP.34")]
#[must_use]
pub fn adapt(fill: Missing<(i64, u32)>, newest: Missing<i64>, mean: i64, quickest: u32) -> i64 {
    match (fill, newest) {
        (Missing::Present((point, days)), _) if days <= quickest => point - 1,
        (Missing::Present((point, _)), _) | (Missing::Absent, Missing::Present(point)) => point,
        (Missing::Absent, Missing::Absent) => mean,
    }
}

/// The severance a separation owes: the days a year of service its terms name, for each whole year served, of the
/// line's daily wage, for each member leaving.
#[clause("LAB.12", "LAB.16")]
#[must_use]
pub fn severance(law: &Law, monthly: f64, days_a_year: u32, years: i64, members: u32) -> f64 {
    if years <= 0 {
        return 0.0;
    }
    let per_day = monthly / (law.weeks_a_month * crate::consts::DAYS_A_WEEK);
    per_day * f64::from(days_a_year) * phx_rand::float::from_i64(years) * f64::from(members)
}

#[cfg(test)]
mod tests {
    use if_labour::law::Law;
    use phx_num::Missing;

    use super::{adapt, least_point, point_near, severance, wage_at};

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
            review_months: 12,
            price_outlook: 1.0,
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

    #[test]
    fn wage_point_adapts_to_fill_history() {
        assert_eq!(adapt(Missing::Absent, Missing::Absent, 57, 3), 57, "no history: the mean wage's point");
        assert_eq!(adapt(Missing::Absent, Missing::Present(55), 57, 3), 55, "its newest staff's point");
        assert_eq!(adapt(Missing::Present((56, 40)), Missing::Present(55), 57, 3), 56, "its last fill's point");
        assert_eq!(adapt(Missing::Present((56, 3)), Missing::Present(55), 57, 3), 55, "a quick fill: a point lower");
    }

    #[test]
    fn severance_owed_by_terms() {
        let l = law();
        let month = l.weeks_a_month * 7.0 * 100.0;
        assert!((severance(&l, month, 7, 10, 2) - 14_000.0).abs() < 1e-6, "7 days a year, 10 years, two members");
        assert!(severance(&l, month, 7, 0, 2).abs() < f64::EPSILON, "no whole year served, none owed");
        assert!(severance(&l, month, 0, 10, 2).abs() < f64::EPSILON, "terms that name none owe none");
    }
}
