//! Retirement as a person's process: on the first birthday an adult not yet retired has reached the age its
//! country's pension begins at, it decides whether to retire; one who does leaves its jobs at the next day's start of
//! work.

use if_labour::class::RETIRED;
use if_labour::decisions::RetireIn;
use phx_core::{AgentView, Household, Person, PopProcess, Register};
use phx_id::{CountryId, Date};
use phx_num::violation;
use phx_rand::Draws;

/// A person's age in months at its last birthday, as the birthdays that bring it to decide read it.
fn age_months(p: &Person, on: Date) -> i64 {
    p.age_on(on) * crate::consts::MONTHS_A_YEAR
}

/// Retirement on the household's persons.
#[derive(Debug, Default)]
pub struct Retirement {
    /// Each country's pension age in months, by sex.
    pension: Vec<Vec<i64>>,
}

impl Retirement {
    fn pension_months(&self, agent: &AgentView<'_>, p: &Person) -> Option<i64> {
        let region = (agent.attr)(if_pop::REGION.name)?;
        let country: CountryId = (agent.country_of)(region)?;
        let sex = usize::try_from(p.attr(if_pop::SEX.name)?).ok()?;
        self.pension.get(usize::from(country.get()))?.get(sex).copied()
    }

    fn deciding(&self, agent: &AgentView<'_>, p: &Person, on: Date) -> bool {
        let adult = p.role != if_pop::CHILD.name;
        let retired = p.attr(crate::STATE.name) == Some(RETIRED);
        let reached = self.pension_months(agent, p).is_some_and(|m| age_months(p, on) >= m);
        adult && !retired && reached
    }
}

impl PopProcess for Retirement {
    fn bind(&mut self, register: &Register) {
        self.pension = (0..register.countries())
            .map(|c| {
                let Ok(c) = u8::try_from(c) else {
                    violation!(clause = "GEN.1", "more countries than an identity holds", countries = c);
                };
                let c = CountryId::new(c);
                match crate::law::pension_months(register, c) {
                    Ok(m) => m,
                    Err(_) => {
                        violation!(clause = "LAB.6", "a pension age the register does not hold", country = c.get())
                    }
                }
            })
            .collect();
    }
    fn hazard(&self) -> &'static str {
        crate::RETIREMENT.name
    }
    fn kind(&self) -> &'static str {
        if_pop::HOUSEHOLD
    }
    /// Certain once an adult not yet retired has reached its pension's age, and nothing before.
    fn rate(&self, _: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        if self.deciding(agent, p, agent.date) { 1.0 } else { 0.0 }
    }
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        (p.attr(crate::STATE.name) != Some(RETIRED)).then(|| p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, agent: &AgentView<'_>, h: &mut Household, reached: &[usize], _: &mut Draws) {
        for i in reached {
            let Some(p) = h.persons.get(*i) else { continue };
            let months = age_months(p, agent.date);
            let Some(pension) = self.pension_months(agent, p) else { continue };
            let input = RetireIn { age_months: months, pension_months: pension };
            if crate::rules::retire::retire(&input)
                && let Some(p) = h.persons.get_mut(*i)
            {
                p.set_attr(crate::STATE.name, RETIRED);
            }
        }
    }
}
