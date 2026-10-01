//! A process that acts on the persons of a population's agents: a person's daily chance of a hit, read by the
//! kernel's draw, and its outcome on the household the hit reached, made explicit by the kernel.

use phx_id::{CountryId, Date, PartyRef};
use phx_macros::clause;

use crate::person_word::{Field, PersonWord, ROLE};
use crate::register::Register;

/// What a household holds of its own that its processes read: the region it lives in, read through its zone; whether
/// its last decision was to try for a child; its ideal number of children, none until its first decision draws it; and
/// its outlook of its income a year, none before its first look.
#[clause("REP.41", "POP.2", "POP.10")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HouseholdState {
    pub region: u32,
    pub trying: bool,
    pub ideal: phx_num::Missing<u8>,
    pub income: phx_num::Missing<i64>,
}

impl HouseholdState {
    /// A household as it forms in a region: not trying for a child, its ideal not yet drawn, no outlook of its income.
    #[must_use]
    pub const fn formed(region: u32) -> HouseholdState {
        HouseholdState { region, trying: false, ideal: phx_num::Missing::Absent, income: phx_num::Missing::Absent }
    }
}

/// An agent as a process reads it: its kind, its party, its household's own state as the day found it, a person's
/// word as its kind begins one (every declared attribute at its initial value), the country each region lies in, the
/// day, and the decision core, which counts a decision the process takes by its name and says how it is taken: by the
/// rule at its decider's preferences, by the player's queued intent, or not that day.
pub struct AgentView<'a> {
    pub kind: &'static str,
    pub party: PartyRef,
    pub state: HouseholdState,
    pub blank: PersonWord,
    pub country_of: &'a dyn Fn(u32) -> Option<CountryId>,
    pub date: Date,
    pub decider: &'a dyn Fn(&str) -> crate::decisions::Say,
}

impl AgentView<'_> {
    /// A decision the process takes, through the decision core: as its decider says, none where the player keeps it
    /// and queued nothing.
    #[clause("MND.20", "OBS.4")]
    pub fn decide<I, O: crate::decisions::QueuedPayload>(
        &self,
        point: &crate::decisions::DecisionPointDecl<I, O>,
        input: impl FnOnce(&crate::decisions::Prefs) -> I,
    ) -> Option<O> {
        (self.decider)(point.name).take(point, input)
    }
}

impl core::fmt::Debug for AgentView<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AgentView").field("kind", &self.kind).field("party", &self.party).finish_non_exhaustive()
    }
}

/// A person held in its household: its word — its birth date, its role and its attributes, each a field — and whether
/// it has gone, died or left, which keeps its place so the persons a hit reached keep theirs.
#[clause("REP.26", "REP.25")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Person {
    pub word: PersonWord,
    pub gone: bool,
}

impl Person {
    /// A person present in its household.
    #[must_use]
    pub const fn of(word: PersonWord) -> Person {
        Person { word, gone: false }
    }

    /// Its role's value, as its kind declares the role.
    #[must_use]
    pub fn role(&self) -> u32 {
        self.word.get(ROLE)
    }

    #[must_use]
    pub fn get(&self, f: Field) -> u32 {
        self.word.get(f)
    }

    /// One field of its word written.
    pub fn set(&mut self, f: Field, v: u32) {
        self.word = self.word.with(f, v);
    }

    pub fn born(&self) -> Date {
        self.word.born()
    }

    /// The whole years the person has lived on a date: one more on each birthday, a birthday on the 29th of February
    /// falling on the 1st of March in a common year.
    #[clause("REP.25")]
    #[must_use]
    pub fn age_on(&self, date: Date) -> i64 {
        self.word.age_on(date)
    }

    /// The person's birthday in a year, a birthday on the 29th of February falling on the 1st of March in a common
    /// year.
    pub fn birthday_in(&self, year: i32) -> Date {
        let (month, day) = crate::consts::LEAP_BIRTHDAY_IN_COMMON_YEAR;
        let born = self.born();
        Date::new(year, born.month(), born.day()).or_else(|| Date::new(year, month, day)).unwrap_or_else(|| {
            phx_num::violation!(clause = "TIME.2", "a birthday on no day of its year");
        })
    }

    /// The person's first birthday after a date.
    pub fn next_birthday(&self, date: Date) -> Date {
        let this_year = self.birthday_in(date.year());
        if this_year > date { this_year } else { self.birthday_in(date.year() + 1) }
    }
}

/// The whole years lived on a date by one born on another: one more on each birthday, a birthday on the 29th of
/// February falling on the 1st of March in a common year.
#[clause("REP.25")]
#[must_use]
pub fn age_on(born: Date, date: Date) -> i64 {
    let before = (date.month(), date.day()) < (born.month(), born.day());
    i64::from(date.year()) - i64::from(born.year()) - i64::from(before)
}

/// A household made explicit for the day's outcomes: its own state and its persons, as read. An outcome changes
/// whether it tries for a child, its ideal and its persons; the kernel then writes them back, or ends the household when
/// no one is left. Its region and income are others' to write, so they are never written back.
#[clause("REP.26", "REP.41")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Household {
    pub state: HouseholdState,
    pub persons: Vec<Person>,
}

impl Household {
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
        Person::of(crate::person_word::PersonWord::new(Date::new(y, m, d).unwrap(), 0))
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
