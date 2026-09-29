//! What the opening reads and writes with besides the core: amounts in whole smallest units, a country's currency,
//! its derived values and employed, its mean wage, monthly dates and plain terms.

use phx_core::OpeningCountry;
use phx_id::CountryId;
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};

/// The currency a country's central bank issues.
pub fn currency(country: CountryId) -> Ccy {
    Ccy::new(country.get())
}

/// An amount in whole smallest units, rounded to the nearest.
#[must_use]
pub fn whole(x: f64) -> i64 {
    let Some(n) = phx_rand::float::floor_to_i64(x.round()) else {
        violation!(clause = "MON.16", "an opening amount beyond whole smallest units");
    };
    n
}

/// A derived value the opening needs, which the country's group must report.
#[clause("GEN.15")]
#[must_use]
pub fn derived(country: &OpeningCountry, name: &str) -> f64 {
    let Some(v) = country.derived(name) else {
        violation!(
            clause = "GEN.15",
            "an opening reading a derived value its country's group does not report",
            country = country.id.get()
        );
    };
    v
}

/// The persons a country employs at the opening: its people from 15 at its employment rate.
#[clause("GEN.2")]
#[must_use]
pub fn employed(country: &OpeningCountry) -> f64 {
    let percent = phx_core::consts::PERCENT_F64;
    let adults = 1.0 - derived(country, "GEN.share_under_15") / percent;
    phx_rand::float::from_u64(country.people) * adults * derived(country, "GEN.employment_rate") / percent
}

/// The mean monthly wage of the employed at the opening: labour's share of the country's output over them, a
/// month's.
#[clause("GEN.2")]
#[must_use]
pub fn mean_wage(country: &OpeningCountry) -> f64 {
    let share = derived(country, "GEN.labour_share") / phx_core::consts::PERCENT_F64;
    share * country.gdp / employed(country) / f64::from(crate::consts::MONTHS_A_YEAR)
}

/// Monthly dates from an anchor, on the anchor's day of each month, a date on no business day moved to the next.
#[must_use]
pub fn monthly(anchor: phx_id::Date, country: CountryId) -> phx_core::calendar::period::ScheduleDates {
    let Some(months) = phx_core::calendar::period::Period::months(1) else {
        violation!(clause = "TIME.4", "a month that is no period");
    };
    phx_core::calendar::period::ScheduleDates {
        anchor,
        period: months,
        eom: phx_core::calendar::period::EndOfMonth::Plain,
        convention: phx_core::calendar::bizday::BusinessDayConvention::Following,
        country,
    }
}

/// Terms of plain legs on a schedule: senior, unsecured, paid first, never terminated or converted, in default at
/// the first payment missed, with no facility and no stay.
#[must_use]
pub fn plain_terms(
    ccy: Ccy,
    legs: Vec<crate::algebra::Leg>,
    schedule: crate::algebra::Schedule,
) -> crate::algebra::Terms {
    use crate::algebra::{DefaultDefinition, Facility, PaymentOrder, Seniority, Termination, Terms};
    Terms {
        ccy,
        legs,
        schedule,
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 0 },
        underlying: Missing::Absent,
        facility: Missing::<Facility>::Absent,
        stay: Missing::Absent,
        class: Vec::new(),
    }
}
