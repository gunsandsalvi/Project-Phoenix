//! The processes on households' persons: death by the country's life table at the person's exact age, and the onset
//! of lasting disability by age and sex.

use if_pop::{DISABLED, FEMALE, HEALTH, HOUSEHOLD, MALE, SEX};
use phx_core::calendar::daycount::actual_days;
use phx_core::register::values::Table1;
use phx_core::{AgentView, Household, Person, PopProcess, Register};
use phx_id::{CountryId, Date};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;
use phx_rand::float::from_i64;

use crate::Prims;
use crate::household::succeed;
use crate::life::{dies_at_age, level_for, survivorship};
use crate::opening::value as read;

pub(crate) fn country(agent: &AgentView<'_>) -> usize {
    let Some(c) = (agent.country_of)(agent.state.region) else {
        violation!(clause = "REP.41", "a household in a region of no country");
    };
    usize::from(c.get())
}

/// Each country's value by its identity, found once at binding.
pub(crate) fn per_country<T>(register: &Register, of: impl Fn(CountryId) -> T) -> Vec<T> {
    (0..register.countries())
        .map(|c| {
            let Ok(c) = u8::try_from(c) else { violation!(clause = "GEN.1", "a country beyond identities") };
            of(CountryId::new(c))
        })
        .collect()
}

pub(crate) fn of_country<'a, T>(list: &'a [T], agent: &AgentView<'_>) -> &'a T {
    let Some(x) = list.get(country(agent)) else {
        violation!(clause = "POP.16", "a country the process was not bound for");
    };
    x
}

pub(crate) fn sex(p: &Person) -> u32 {
    let s = p.get(SEX.field);
    if s != FEMALE && s != MALE {
        violation!(clause = "POP.1", "a person of a sex the tables do not hold");
    }
    s
}

pub(crate) fn age(p: &Person, date: Date) -> usize {
    let Ok(a) = usize::try_from(p.age_on(date)) else {
        violation!(clause = "REP.25", "a person read before it was born");
    };
    a
}

/// The days of the person's present year of age: from its last birthday to its next.
fn year_of_age(p: &Person, date: Date) -> i64 {
    let next = p.next_birthday(date);
    actual_days(p.birthday_in(next.year() - 1), next)
}

/// A country's survivorship from birth to each whole age, women's and men's: the standard at the level that gives
/// its life expectancy, the sexes weighted by their shares of births.
fn life_table(p: &Prims, register: &Register, id: CountryId) -> [Vec<f64>; 2] {
    let (standard, decl) = (p.survival.get(register, id), p.survival.decl(register));
    let [f, m] = [FEMALE, MALE]
        .map(|s| standard.rows().iter().map(|a| read(standard, decl, *a, i64::from(s))).collect::<Vec<f64>>());
    let ratio = p.sex_ratio.get(register, id).to_f64();
    let male_share = ratio / (crate::consts::PERCENT + ratio);
    let level = level_for([&f, &m], male_share, p.life_expectancy.get(register, id).to_f64());
    [survivorship(&f, level), survivorship(&m, level)]
}

/// Death: a person's chance of dying before its next birthday by the country's life table at its exact age, split
/// between the able and the disabled so the disabled's hazard is the declared multiple of the able's and the two,
/// weighted by the chance of disability at that age, make the table's; compounding over the days of that year of age.
/// The dead leave the household; the partner, else the eldest adult, else the eldest child takes a dead head's place;
/// a household no one is left in ends.
#[clause("POP.3", "POP.16", "REP.25", "REP.26")]
#[derive(Debug)]
pub struct Mortality {
    prims: Prims,
    tables: Vec<[Vec<f64>; 2]>,
    disabled: Vec<Vec<[f64; 2]>>,
    ratio: Option<Table1>,
}

impl Mortality {
    #[must_use]
    pub fn new(prims: Prims) -> Mortality {
        Mortality { prims, tables: Vec::new(), disabled: Vec::new(), ratio: None }
    }

    /// The disabled's hazard over the able's at an age.
    fn ratio(&self, a: usize) -> f64 {
        let Some(table) = &self.ratio else { violation!(clause = "POP.3", "a death before its binding") };
        let at = i64::try_from(a).ok().and_then(|a| table.at(a).ok());
        let Some(raw) = at else { violation!(clause = "POP.3", "an age outside the disabled's mortality", age = a) };
        from_i64(raw) / crate::consts::PERCENT
    }
}

impl PopProcess for Mortality {
    fn bind(&mut self, register: &Register) {
        self.tables = per_country(register, |c| life_table(&self.prims, register, c));
        self.disabled = per_country(register, |c| {
            let Some(oldest) = self.prims.age_standard.get(register, c).rows().last().copied() else {
                violation!(clause = "GEN.2", "an age standard of no ages", country = c.get());
            };
            crate::opening::disabled(&self.prims, register, c, oldest)
        });
        self.ratio = Some(self.prims.disabled_mortality.shared(register).clone());
    }
    fn hazard(&self) -> &'static str {
        crate::DEATH.name
    }
    fn kind(&self) -> &'static str {
        HOUSEHOLD
    }
    fn rate(&self, _: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        let [women, men] = of_country(&self.tables, agent);
        let table = if sex(p) == MALE { men } else { women };
        let Ok(days) = u32::try_from(year_of_age(p, agent.date)) else {
            violation!(clause = "TIME.2", "a year of age beyond counting");
        };
        let a = age(p, agent.date);
        let chances = of_country(&self.disabled, agent);
        // The table's last age is open: those older read it, as they read the life table's last year.
        let open = chances.len().checked_sub(1).filter(|last| a > *last);
        let Some(share) = chances.get(open.unwrap_or(a)).and_then(|s| s.get(usize::from(sex(p) == MALE))) else {
            violation!(clause = "POP.3", "a disability table of no ages", age = a);
        };
        let ratio = self.ratio(a);
        let able = -libm::log1p(-dies_at_age(table, a)) / (1.0 + share * (ratio - 1.0));
        let hazard = if p.get(HEALTH.field) == DISABLED { ratio * able } else { able };
        phx_core::annual_to_daily(-libm::expm1(-hazard), days)
    }
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        Some(p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, _: &AgentView<'_>, h: &mut Household, reached: &[usize], _: &mut Draws) {
        for i in reached {
            if let Some(p) = h.persons.get_mut(*i) {
                p.gone = true;
            }
        }
        succeed(h);
    }
}

/// The onset of lasting disability: an able person's yearly hazard at its exact age and sex from the country's
/// table, compounding over the days of that year of age; the person becomes disabled in place.
#[clause("POP.4", "POP.16")]
#[derive(Debug)]
pub struct Onset {
    pub prims: Prims,
}

impl PopProcess for Onset {
    fn bind(&mut self, _: &Register) {}
    fn hazard(&self) -> &'static str {
        crate::ONSET.name
    }
    fn kind(&self) -> &'static str {
        HOUSEHOLD
    }
    fn rate(&self, register: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        if p.get(HEALTH.field) == DISABLED {
            return 0.0;
        }
        let Ok(c) = u8::try_from(country(agent)) else { violation!(clause = "GEN.1", "a country beyond identities") };
        let id = CountryId::new(c);
        let (table, decl) = (self.prims.onset.get(register, id), self.prims.onset.decl(register));
        let Ok(a) = i64::try_from(age(p, agent.date)) else { violation!(clause = "REP.25", "an age beyond counting") };
        let hazard = read(table, decl, a, i64::from(sex(p)));
        if hazard < 0.0 {
            violation!(clause = "POP.16", "a disability hazard below nought", country = c);
        }
        -libm::expm1(-hazard / from_i64(year_of_age(p, agent.date)))
    }
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        Some(p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, _: &AgentView<'_>, h: &mut Household, reached: &[usize], _: &mut Draws) {
        for i in reached {
            if let Some(p) = h.persons.get_mut(*i) {
                p.set(HEALTH.field, DISABLED);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::Person;
    use phx_id::Date;

    use super::year_of_age;

    #[test]
    fn a_year_of_age_runs_from_birthday_to_birthday() {
        let p = Person::of(phx_core::person_word::PersonWord::new(Date::new(1990, 3, 10).unwrap(), 0));
        assert_eq!(year_of_age(&p, Date::new(2027, 6, 1).unwrap()), 366, "10 March 2027 to 10 March 2028");
        assert_eq!(year_of_age(&p, Date::new(2026, 1, 1).unwrap()), 365);
    }
}
