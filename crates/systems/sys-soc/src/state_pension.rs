//! The state pension at the opening: each adult who has reached the country's pension age for its sex draws the state
//! pension at its sex's coverage — the rule's flat amount, the replacement rate of the labour share's mean wage per
//! worker — paid monthly by the treasury, a person's row on the country's state pension line.

use phx_core::register::values::Table1;
use phx_core::{OpeningCountry, Prim, Register, declare_stream};
use phx_ledger::opening::whole;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::open_unit;

use crate::consts::{AGE_PARTS, SHARE_PARTS};

declare_stream! { pub PensionStream = "SOC.opening_pensions" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_stream! {
    /// Whether a person who retires is among those its country's state pension covers, drawn once for each person.
    pub CoveredStream = "SOC.pension_covered" { family: World, purpose: Occasion, keyed: true, clause: "SOC.3" }
}

/// A country's state pension as a person who retires claims it.
///
/// # Errors
/// A primitive missing or of another shape.
#[clause("SOC.3", "GEN.2")]
pub fn law(register: &Register, c: &OpeningCountry) -> Result<if_state::kinds::PensionLaw, String> {
    let at = |id: &str, parts: f64| -> Result<[f64; 2], String> {
        let table = register.table1_in(id, c.id)?;
        let mut out = [0.0; 2];
        for (slot, sex) in out.iter_mut().zip([if_pop::FEMALE, if_pop::MALE]) {
            *slot = phx_rand::float::from_i64(table.at(i64::from(sex)).map_err(|e| format!("{e:?}"))?) / parts;
        }
        Ok(out)
    };
    Ok(if_state::kinds::PensionLaw {
        age: at(crate::PENSION_AGE.id, AGE_PARTS)?,
        replacement: at(crate::REPLACEMENT.id, SHARE_PARTS)?,
        coverage: at(crate::COVERAGE.id, SHARE_PARTS)?,
    })
}

/// The state pension, which the core binds.
pub const PENSIONS: if_state::kinds::PensionKind = if_state::kinds::PensionKind { law };

/// Social protection's draw of the state pensions in payment.
#[derive(Debug)]
pub struct StatePension {
    pub age: Prim<Table1>,
    pub replacement: Prim<Table1>,
    pub coverage: Prim<Table1>,
}

/// One country's state pension as its pensioners draw it, read from the register alone: by sex, the age it starts
/// at, the share covered and the month's amount, a replacement of the mean wage.
#[derive(Clone, Copy, Debug)]
pub struct Pensions {
    date: phx_id::Date,
    age: [f64; 2],
    coverage: [f64; 2],
    pub amount: [i64; 2],
}

impl StatePension {
    /// The state pension's draw with its primitives found in the register.
    ///
    /// # Errors
    /// A primitive the draw reads that the register does not hold as declared.
    pub fn of(register: &Register) -> Result<StatePension, String> {
        Ok(StatePension {
            age: register.handle(&crate::PENSION_AGE)?,
            replacement: register.handle(&crate::REPLACEMENT)?,
            coverage: register.handle(&crate::COVERAGE)?,
        })
    }

    /// A country's state pension as its pensioners draw it, on the opening's date.
    #[clause("GEN.2", "SOC.3")]
    #[must_use]
    pub fn pensions(&self, register: &Register, date: phx_id::Date, c: &OpeningCountry) -> Pensions {
        let wage = phx_ledger::opening::mean_wage(c);
        Pensions {
            date,
            age: by_sex(self.age.get(register, c.id), AGE_PARTS),
            coverage: by_sex(self.coverage.get(register, c.id), SHARE_PARTS),
            amount: by_sex(self.replacement.get(register, c.id), SHARE_PARTS).map(|rate| whole(rate * wage)),
        }
    }
}

impl Pensions {
    /// The adults of a household drawn in payment of the state pension: each past its sex's pension age and covered
    /// at its sex's share; each with its place and the pension's sex.
    #[clause("GEN.2", "SOC.3")]
    #[must_use]
    pub fn draw(&self, household: &phx_core::Household, d: &mut phx_rand::Draws) -> Vec<(usize, usize)> {
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        let mut out = Vec::new();
        for (place, p) in household.persons.iter().enumerate().filter(|(_, p)| adult_roles.contains(&p.role)) {
            let covered = open_unit(d);
            let Some(sex) = p.attr(if_pop::SEX.name) else { violation!(clause = "REP.26", "a person with no sex") };
            let age = phx_rand::float::from_i64(p.age_on(self.date));
            let at = match sex {
                if_pop::FEMALE => 0,
                if_pop::MALE => 1,
                _ => violation!(clause = "REP.26", "a sex beyond the two", sex = sex),
            };
            let (Some(start), Some(share)) = (self.age.get(at), self.coverage.get(at)) else {
                violation!(clause = "GEN.2", "a sex the state pension does not hold");
            };
            if age < *start || covered >= *share {
                continue;
            }
            out.push((place, at));
        }
        out
    }
}

fn by_sex(table: &Table1, parts: f64) -> [f64; 2] {
    [if_pop::FEMALE, if_pop::MALE].map(|sex| {
        let Ok(v) = table.at(i64::from(sex)) else { violation!(clause = "GEN.2", "no value for a sex", sex = sex) };
        phx_rand::float::from_i64(v) / parts
    })
}
