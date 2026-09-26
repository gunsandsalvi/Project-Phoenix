//! A country's labour law and its technology of search, compiled once from the register.

use phx_num::Missing;

/// What a country's labour law, its trade's conventions and its technology of search set, compiled once: the hours of
/// a full-time job a week; the notice and the severance a year of service in days; the mean monthly wage of a job at
/// the opening, and the least monthly wage a full-time job may pay, where the law sets one; the ratio between neighbouring wage points; the days a vacancy stands before
/// its employer raises its offer; the applications a searcher sends a week and the chance an employer sees one; the
/// weight of the wage in a searcher's choice; the reservation as a share of the searcher's last wage; the weeks in a
/// month; the years of a start band; the months of age each sex's pension begins at; the months between an employer's
/// reviews of its wages; and the price level an employee expects at the next review over today's.
#[derive(Clone, Debug, PartialEq)]
pub struct Law {
    pub full_time_hours: u32,
    pub notice_days: u32,
    pub severance_days_a_year: u32,
    pub mean_monthly: f64,
    pub minimum_monthly: Missing<f64>,
    pub point_ratio: f64,
    pub patience_days: u32,
    pub applications_a_week: f64,
    pub seen_chance: f64,
    pub wage_weight: f64,
    pub reservation_share: f64,
    pub weeks_a_month: f64,
    pub band_years: u32,
    /// Each occupation family's least skill level, by the family's place.
    pub occupation_skill: Vec<u32>,
    /// The skill level each education value gives, by the value's place.
    pub education_skill: Vec<u32>,
    pub pension_months: Vec<i64>,
    pub review_months: u32,
    pub price_outlook: f64,
}
