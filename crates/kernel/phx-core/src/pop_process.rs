//! A process that acts on the persons of a population's agents: a person's daily chance of a hit, read by the
//! kernel's draw, and its outcome on the household the hit reached, made explicit by the kernel.

use phx_id::{CountryId, Date, PartyId};
use phx_macros::clause;

use crate::register::Register;

/// An agent as a process reads it: its kind, its party, the value of each of its attributes by name, the country each
/// region lies in, and the day.
pub struct AgentView<'a> {
    pub kind: &'static str,
    pub party: PartyId,
    pub attr: &'a dyn Fn(&str) -> Option<u32>,
    pub country_of: &'a dyn Fn(u32) -> Option<CountryId>,
    pub date: Date,
}

impl core::fmt::Debug for AgentView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AgentView").field("kind", &self.kind).field("party", &self.party).finish_non_exhaustive()
    }
}

/// A person held in its household: its role, its birth date, its value of each of its kind's person attributes by
/// name, and whether it has gone — died or left — which keeps its place so the persons a hit reached keep theirs.
#[clause("REP.26", "REP.25")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Person {
    pub role: &'static str,
    pub born: Date,
    pub attrs: Vec<(&'static str, u32)>,
    pub gone: bool,
}

impl Person {
    /// The person's value of an attribute its kind declares.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<u32> {
        self.attrs.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }

    /// An attribute set to a value.
    pub fn set_attr(&mut self, name: &str, value: u32) {
        let Some((_, v)) = self.attrs.iter_mut().find(|(n, _)| *n == name) else {
            phx_num::violation!(clause = "REP.26", "a person given an attribute its kind does not declare");
        };
        *v = value;
    }

    /// An attribute given its first value, as the opening's draws give a person the attributes other systems than
    /// its composer declare; one it already holds is set.
    pub fn put_attr(&mut self, name: &'static str, value: u32) {
        match self.attrs.iter_mut().find(|(n, _)| *n == name) {
            Some((_, v)) => *v = value,
            None => self.attrs.push((name, value)),
        }
    }

    /// The whole years the person has lived on a date: one more on each birthday, a birthday on the 29th of February
    /// falling on the 1st of March in a common year.
    #[clause("REP.25")]
    #[must_use]
    pub fn age_on(&self, date: Date) -> i64 {
        let before = (date.month(), date.day()) < (self.born.month(), self.born.day());
        i64::from(date.year()) - i64::from(self.born.year()) - i64::from(before)
    }

    /// The person's birthday in a year, a birthday on the 29th of February falling on the 1st of March in a common
    /// year.
    pub fn birthday_in(&self, year: i32) -> Date {
        let (month, day) = crate::consts::LEAP_BIRTHDAY_IN_COMMON_YEAR;
        Date::new(year, self.born.month(), self.born.day()).or_else(|| Date::new(year, month, day)).unwrap_or_else(
            || {
                phx_num::violation!(clause = "TIME.2", "a birthday on no day of its year");
            },
        )
    }

    /// The person's first birthday after a date.
    pub fn next_birthday(&self, date: Date) -> Date {
        let this_year = self.birthday_in(date.year());
        if this_year > date { this_year } else { self.birthday_in(date.year() + 1) }
    }
}

/// A household made explicit for the day's outcomes: its attributes by name and its persons. An outcome changes it;
/// the kernel then writes it back to its agent, or ends the agent when no one is left.
#[clause("REP.26", "REP.41")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Household {
    pub attrs: Vec<(&'static str, u32)>,
    pub persons: Vec<Person>,
}

impl Household {
    /// An attribute's value; every household holds each of its kind's.
    #[must_use]
    pub fn attr(&self, name: &str) -> u32 {
        let Some((_, v)) = self.attrs.iter().find(|(n, _)| *n == name) else {
            phx_num::violation!(clause = "REP.41", "a household read by an attribute its kind does not hold");
        };
        *v
    }

    /// An attribute set to a value.
    pub fn set_attr(&mut self, name: &str, value: u32) {
        let Some((_, v)) = self.attrs.iter_mut().find(|(n, _)| *n == name) else {
            phx_num::violation!(clause = "REP.41", "a household given an attribute its kind does not hold");
        };
        *v = value;
    }

    /// The persons still in the household.
    pub fn present(&self) -> impl Iterator<Item = (usize, &Person)> {
        self.persons.iter().enumerate().filter(|(_, p)| !p.gone)
    }
}

/// A process on a population kind's persons, declared by the system that owns its outcome. The hazard it answers
/// names its rate table and stream; the process reads that table here, for a person on a day.
#[clause("CHN.2", "REP.7")]
pub trait PopProcess: Send + Sync {
    /// What its rate reads of the register, found once when the world binds it, before any day; it holds nothing
    /// else, so its rate is a pure read.
    fn bind(&mut self, register: &Register);
    /// The hazard it answers, as declared.
    fn hazard(&self) -> &'static str;
    /// The population kind whose persons it acts on.
    fn kind(&self) -> &'static str;
    /// A person's daily chance of a hit on the agent's day, read from the agent and the person.
    fn rate(&self, register: &Register, agent: &AgentView<'_>, person: &Person) -> f64;
    /// The first day after `date` on which the person's rate may change though its agent does not change.
    fn changes_after(&self, person: &Person, date: Date) -> Option<Date>;
    /// What the persons it reached, by their places in the household, do to their household; what is left to chance
    /// is drawn from `draws`, the process's own for the agent and the day.
    fn outcome(
        &self,
        register: &Register,
        agent: &AgentView<'_>,
        household: &mut Household,
        reached: &[usize],
        draws: &mut phx_rand::Draws,
    );
}

#[cfg(test)]
mod tests {
    use phx_id::Date;

    use super::Person;

    fn born(y: i32, m: u8, d: u8) -> Person {
        Person { role: "head", born: Date::new(y, m, d).unwrap(), attrs: Vec::new(), gone: false }
    }

    #[test]
    fn age_moves_on_the_birthday() {
        let p = born(1980, 6, 15);
        assert_eq!(p.age_on(Date::new(2025, 6, 14).unwrap()), 44);
        assert_eq!(p.age_on(Date::new(2025, 6, 15).unwrap()), 45);
        assert_eq!(p.next_birthday(Date::new(2025, 6, 15).unwrap()), Date::new(2026, 6, 15).unwrap());
        assert_eq!(p.next_birthday(Date::new(2025, 1, 1).unwrap()), Date::new(2025, 6, 15).unwrap());
    }

    #[test]
    fn a_leap_day_birthday_falls_on_the_first_of_march_in_a_common_year() {
        let p = born(2000, 2, 29);
        assert_eq!(p.age_on(Date::new(2025, 2, 28).unwrap()), 24);
        assert_eq!(p.age_on(Date::new(2025, 3, 1).unwrap()), 25);
        assert_eq!(p.next_birthday(Date::new(2025, 1, 1).unwrap()), Date::new(2025, 3, 1).unwrap());
        assert_eq!(p.next_birthday(Date::new(2027, 12, 31).unwrap()), Date::new(2028, 2, 29).unwrap());
    }
}
