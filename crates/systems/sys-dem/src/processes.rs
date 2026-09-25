//! The processes on households' persons: death by the country's life table, the onset of lasting disability by age
//! and sex, and birthdays, which move the head and the children across the age classes the household holds them in.

use if_pop::consts::FIRST_BIRTH_YEAR;
use if_pop::{
    BIRTH_YEAR_AT, CLASSED_GROUPS, DISABLED, FEMALE, HEAD_AGE, HEALTH_AT, HOUSEHOLD, LIFE, LIFE_GROUPS, MALE, REGION,
    SEX_AT,
};
use phx_core::calendar::daycount::actual_days;
use phx_core::{CellView, Household, Person, PopProcess, Register, component, joint};
use phx_id::{CountryId, Date};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::from_i64;
use phx_rand::{Draws, open_unit};

use crate::Prims;
use crate::household::{Role, life, settle, succeed, take_role};
use crate::life::{dies_in_year, level_for, survivorship};
use crate::opening::value as read;

/// A key attribute's value, which a household's key always holds.
fn key(cell: &CellView<'_>, name: &str) -> u32 {
    let Some(v) = (cell.key)(name) else { violation!(clause = "REP.19", "a household key without an attribute") };
    v
}

fn country(cell: &CellView<'_>) -> usize {
    let Some(c) = (cell.country_of)(key(cell, REGION.name)) else {
        violation!(clause = "REP.19", "a household in a region of no country");
    };
    usize::from(c.get())
}

/// The age a person of a life value reaches in a year: the year less its birth year.
fn reached(year: i32, value: u32) -> i64 {
    i64::from(year) - i64::from(FIRST_BIRTH_YEAR) - i64::from(component(LIFE, value, BIRTH_YEAR_AT))
}

fn year_start(year: i32) -> Date {
    let Some(d) = Date::new(year, 1, 1) else { violation!(clause = "TIME.2", "a year with no first day") };
    d
}

/// The days of a date's year, and those from the date to its end, the date's own among them.
fn year_days(date: Date) -> (i64, i64) {
    let next = year_start(date.year() + 1);
    (actual_days(year_start(date.year()), next), actual_days(date, next))
}

fn days32(n: i64) -> u32 {
    let Ok(n) = u32::try_from(n) else { violation!(clause = "TIME.2", "a year of days beyond counting") };
    n
}

/// The age class an age falls in.
pub(crate) fn class(bounds: &[i64], age: i64) -> u32 {
    let Some(c) = bounds.partition_point(|b| *b <= age).checked_sub(1).and_then(|c| u32::try_from(c).ok()) else {
        violation!(clause = "REP.25", "an age below the first age class", age = age);
    };
    c
}

/// Each country's value by its identity, found once at binding.
fn per_country<T>(register: &Register, of: impl Fn(CountryId) -> T) -> Vec<T> {
    (0..register.countries())
        .map(|c| {
            let Ok(c) = u8::try_from(c) else { violation!(clause = "GEN.1", "a country beyond identities") };
            of(CountryId::new(c))
        })
        .collect()
}

fn of_country<'a, T>(list: &'a [T], cell: &CellView<'_>) -> &'a T {
    let Some(x) = list.get(country(cell)) else {
        violation!(clause = "POP.16", "a country the process was not bound for");
    };
    x
}

fn of_sex<T>(pair: &[T; 2], value: u32) -> &T {
    let sex = component(LIFE, value, SEX_AT);
    if sex != FEMALE && sex != MALE {
        violation!(clause = "POP.1", "a sex the tables do not hold", sex = sex);
    }
    let [women, men] = pair;
    if sex == MALE { men } else { women }
}

/// The age class of a person taking the head's place: a child's band is its class; an adult's age this day is the
/// age it reaches this year if its birthday, spread over the year's days, has passed, and a year less if not.
fn successor_class(bounds: &[i64], p: &Person, date: Date, d: &mut Draws) -> u32 {
    if let Role::Child(b) = Role::of(p.role) {
        return b;
    }
    let (days, left) = year_days(date);
    let passed = open_unit(d) < from_i64(days - left) / from_i64(days);
    class(bounds, reached(date.year(), life(p)) - i64::from(!passed))
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

/// Death: a person's chance of dying in the year by the country's life table at the ages its birth year spans in it,
/// compounding over the year's days. The dead leave the household; the partner, else the eldest adult, else the
/// eldest child takes a dead head's place; a household no one is left in ends.
#[clause("POP.3", "POP.16", "REP.25", "REP.26")]
#[derive(Debug)]
pub struct Mortality {
    prims: Prims,
    tables: Vec<[Vec<f64>; 2]>,
    bounds: Vec<i64>,
}

impl Mortality {
    #[must_use]
    pub fn new(prims: Prims) -> Mortality {
        Mortality { prims, tables: Vec::new(), bounds: Vec::new() }
    }
}

impl PopProcess for Mortality {
    fn bind(&mut self, register: &Register) {
        self.tables = per_country(register, |c| life_table(&self.prims, register, c));
        self.bounds = self.prims.age_classes.shared(register).bounds.to_vec();
    }
    fn hazard(&self) -> &'static str {
        crate::DEATH.name
    }
    fn kind(&self) -> &'static str {
        HOUSEHOLD
    }
    fn groups(&self) -> &'static [&'static str] {
        LIFE_GROUPS
    }
    fn rate(&self, _: &Register, cell: &CellView<'_>, _: &'static str, value: u32) -> f64 {
        let table = of_sex(of_country(&self.tables, cell), value);
        let Ok(age) = usize::try_from(reached(cell.date.year(), value)) else {
            violation!(clause = "REP.25", "a person born after the year it is read");
        };
        phx_core::annual_to_daily(dies_in_year(table, age), days32(year_days(cell.date).0))
    }
    fn changes_after(&self, date: Date) -> Option<Date> {
        Some(year_start(date.year() + 1))
    }
    fn outcome(&self, _: &Register, cell: &CellView<'_>, h: &mut Household, reached: &[usize], d: &mut Draws) {
        for i in reached {
            if let Some(p) = h.persons.get_mut(*i) {
                p.gone = true;
            }
        }
        succeed(h, |p| successor_class(&self.bounds, p, cell.date, d));
        settle(h);
    }
}

/// The onset of lasting disability: an able person's yearly hazard by age and sex from the country's table,
/// compounding over the year's days; the person becomes disabled in place.
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
    fn groups(&self) -> &'static [&'static str] {
        LIFE_GROUPS
    }
    fn rate(&self, register: &Register, cell: &CellView<'_>, _: &'static str, value: u32) -> f64 {
        if component(LIFE, value, HEALTH_AT) == DISABLED {
            return 0.0;
        }
        let Ok(c) = u8::try_from(country(cell)) else { violation!(clause = "GEN.1", "a country beyond identities") };
        let id = CountryId::new(c);
        let (table, decl) = (self.prims.onset.get(register, id), self.prims.onset.decl(register));
        let sex = *of_sex(&[i64::from(FEMALE), i64::from(MALE)], value);
        let hazard = read(table, decl, reached(cell.date.year(), value), sex);
        if hazard < 0.0 {
            violation!(clause = "POP.16", "a disability hazard below nought", country = c);
        }
        -libm::expm1(-hazard / from_i64(year_days(cell.date).0))
    }
    fn changes_after(&self, date: Date) -> Option<Date> {
        Some(year_start(date.year() + 1))
    }
    fn outcome(&self, _: &Register, _: &CellView<'_>, h: &mut Household, reached: &[usize], _: &mut Draws) {
        for i in reached {
            let Some(p) = h.persons.get_mut(*i) else { continue };
            let group = Role::of(p.role).life_group();
            let v = life(p);
            let disabled = joint(LIFE, &[component(LIFE, v, BIRTH_YEAR_AT), component(LIFE, v, SEX_AT), DISABLED]);
            for (g, x) in &mut p.values {
                if *g == group {
                    *x = disabled;
                }
            }
        }
    }
}

/// Birthdays: the head or a child whose age class changes this year, or a child who reaches the age of majority,
/// moves on a day spread evenly over the year, its chance each day one over the days left in the year; the head's
/// class is the key's, a child's its band, and a child of age becomes another adult.
#[clause("REP.25", "REP.26")]
#[derive(Debug)]
pub struct Birthdays {
    prims: Prims,
    bounds: Vec<i64>,
    majority: Vec<i64>,
}

impl Birthdays {
    #[must_use]
    pub fn new(prims: Prims) -> Birthdays {
        Birthdays { prims, bounds: Vec::new(), majority: Vec::new() }
    }

    /// Whether a person of a group, at a value, has a birthday this year that moves it and has not had it.
    fn pending(&self, cell: &CellView<'_>, group: &str, value: u32) -> bool {
        let age = reached(cell.date.year(), value);
        let (before, after) = (class(&self.bounds, age - 1), class(&self.bounds, age));
        match Role::of_group(group) {
            Role::Head => key(cell, HEAD_AGE.name) == before && after != before,
            Role::Child(b) => b == before && (after != before || age >= *of_country(&self.majority, cell)),
            Role::Partner | Role::Adult => false,
        }
    }
}

impl PopProcess for Birthdays {
    fn bind(&mut self, register: &Register) {
        self.bounds = self.prims.age_classes.shared(register).bounds.to_vec();
        self.majority = per_country(register, |c| {
            let Ok(m) = i64::try_from(self.prims.majority.get(register, c).get()) else {
                violation!(clause = "POP.16", "an age of majority beyond counting", country = c.get());
            };
            m
        });
    }
    fn hazard(&self) -> &'static str {
        crate::BIRTHDAY.name
    }
    fn kind(&self) -> &'static str {
        HOUSEHOLD
    }
    fn groups(&self) -> &'static [&'static str] {
        CLASSED_GROUPS
    }
    fn rate(&self, _: &Register, cell: &CellView<'_>, group: &'static str, value: u32) -> f64 {
        if self.pending(cell, group, value) { 1.0 / from_i64(year_days(cell.date).1) } else { 0.0 }
    }
    fn changes_after(&self, date: Date) -> Option<Date> {
        Date::new(date.year(), date.month() + 1, 1).or_else(|| Date::new(date.year() + 1, 1, 1))
    }
    fn outcome(&self, _: &Register, cell: &CellView<'_>, h: &mut Household, reached_at: &[usize], _: &mut Draws) {
        let majority = *of_country(&self.majority, cell);
        for i in reached_at {
            let Some(p) = h.persons.get_mut(*i) else { continue };
            let age = reached(cell.date.year(), life(p));
            let class = class(&self.bounds, age);
            match Role::of(p.role) {
                Role::Head => h.set_attr(HEAD_AGE.name, class),
                Role::Child(_) if age >= majority => take_role(p, Role::Adult),
                Role::Child(_) => take_role(p, Role::Child(class)),
                Role::Partner | Role::Adult => {
                    violation!(clause = "REP.25", "a birthday for a person whose class the household does not hold")
                }
            }
        }
        settle(h);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Date;

    use super::{class, year_days};

    #[test]
    fn classes_and_days_of_the_year() {
        let bounds = [0, 6, 12, 18, 35];
        assert_eq!([0, 5, 6, 17, 18, 90].map(|a| class(&bounds, a)), [0, 0, 1, 2, 3, 4]);
        assert_eq!(year_days(Date::new(2028, 12, 31).unwrap()), (366, 1));
        assert_eq!(year_days(Date::new(2027, 1, 1).unwrap()), (365, 365));
    }
}
