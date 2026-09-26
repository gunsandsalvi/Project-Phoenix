//! Each country's labour law and technology of search, compiled from the register and the opening's country.

use if_labour::law::Law;
use phx_core::{OpeningCountry, Register};
use phx_id::CountryId;
use phx_macros::clause;
use phx_num::Missing;

use crate::consts::{DAYS_A_WEEK, DAYS_A_YEAR, MONTHS_PER_YEAR, PERCENT};

/// A table of one axis over places in order, whole.
fn places(register: &Register, id: &str, country: CountryId) -> Result<Vec<u32>, String> {
    let t = register.table1_in(id, country)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err(format!("`{id}` is not by places in order"));
    }
    t.values().iter().map(|v| u32::try_from(*v).map_err(|_| format!("`{id}` holds {v}, no place"))).collect()
}

fn whole(register: &Register, id: &str, country: CountryId) -> Result<u32, String> {
    u32::try_from(register.count_in(id, country)?).map_err(|e| format!("`{id}`: {e}"))
}

/// The mean monthly wage of a job at the opening: the labour share of the country's output over its employed, a
/// month's.
#[clause("GEN.2", "LAB.1")]
#[must_use]
pub fn mean_wage(c: &OpeningCountry) -> f64 {
    let d = |name| phx_ledger::opening::derived(c, name);
    let adults = 1.0 - d("GEN.share_under_15") / PERCENT;
    let workers = phx_rand::float::from_u64(c.people) * adults * d("GEN.employment_rate") / PERCENT;
    d("GEN.labour_share") / PERCENT * c.gdp / workers / MONTHS_PER_YEAR
}

/// The months of age each sex's state pension begins at, by the sex's place, from its law in years.
pub(crate) fn pension_months(register: &Register, country: CountryId) -> Result<Vec<i64>, String> {
    let t = register.table1_in(PENSION_AGE, country)?;
    let phx_core::ValueType::Table1 { exp, .. } = register.decl_by_id(PENSION_AGE)?.value else {
        return Err(format!("`{PENSION_AGE}` is no table of one axis"));
    };
    let years = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
    t.values()
        .iter()
        .map(|v| {
            phx_rand::float::floor_to_i64(libm::rint(phx_rand::float::from_i64(*v) / years * MONTHS_PER_YEAR))
                .ok_or_else(|| format!("`{PENSION_AGE}` holds {v}, beyond an age"))
        })
        .collect()
}

/// The law's pension age by sex, which retirement reads until households weigh it.
const PENSION_AGE: &str = "SOC.pension_age";

/// A country's labour law and technology of search.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("LAB.11", "LAB.16")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<Law, String> {
    let id = c.id;
    let share = register.fixed_in(crate::MINIMUM_SHARE.id, id)?;
    Ok(Law {
        full_time_hours: whole(register, crate::FULL_TIME_HOURS.id, id)?,
        notice_days: whole(register, crate::NOTICE_DAYS.id, id)?,
        severance_days_a_year: whole(register, crate::SEVERANCE_DAYS.id, id)?,
        mean_monthly: mean_wage(c),
        // A law that sets no minimum declares none, as no share of the mean wage.
        minimum_monthly: if share > 0.0 { Missing::Present(share * mean_wage(c)) } else { Missing::Absent },
        point_ratio: register.fixed(crate::WAGE_POINT_RATIO.id)?,
        patience_days: whole(register, crate::PATIENCE_DAYS.id, id)?,
        applications_a_week: register.fixed(crate::APPLICATIONS.id)?,
        seen_chance: register.fixed(crate::SEEN_CHANCE.id)?,
        wage_weight: register.fixed(crate::WAGE_WEIGHT.id)?,
        reservation_share: register.fixed(crate::RESERVATION_SHARE.id)?,
        weeks_a_month: DAYS_A_YEAR / DAYS_A_WEEK / MONTHS_PER_YEAR,
        band_years: whole(register, crate::BAND_YEARS.id, id)?,
        occupation_skill: places(register, crate::OCCUPATION_SKILL.id, id)?,
        education_skill: places(register, crate::EDUCATION_SKILL.id, id)?,
        pension_months: pension_months(register, id)?,
    })
}
