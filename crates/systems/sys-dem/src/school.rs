//! Leaving school, until education is built: a child leaves on the birthday its country's education law lets it, and
//! becomes another adult of its household, out of the labour force until its first decision to search.

use if_pop::{ADULT, CHILD};
use phx_core::{AgentView, Household, Person, PopProcess, Register};
use phx_id::Date;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::Draws;

use crate::Prims;
use crate::processes::{of_country, per_country};

/// A child leaves school on the birthday its country's school-leaving age falls on.
#[clause("REP.25", "REP.26", "POP.16")]
#[derive(Debug)]
pub struct LeavingSchool {
    prims: Prims,
    leaving: Vec<i64>,
}

impl LeavingSchool {
    #[must_use]
    pub fn new(prims: Prims) -> LeavingSchool {
        LeavingSchool { prims, leaving: Vec::new() }
    }
}

impl PopProcess for LeavingSchool {
    fn bind(&mut self, register: &Register) {
        self.leaving = per_country(register, |c| {
            let Ok(m) = i64::try_from(self.prims.school_leaving.get(register, c).get()) else {
                violation!(clause = "POP.16", "a school-leaving age beyond counting", country = c.get());
            };
            m
        });
    }
    fn hazard(&self) -> &'static str {
        crate::BIRTHDAY.name
    }
    fn kind(&self) -> &'static str {
        if_pop::HOUSEHOLD
    }
    /// Certain on the day a child reaches the school-leaving age, and nothing on any other.
    fn rate(&self, _: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        let leaves = p.role == CHILD.name && p.age_on(agent.date) >= *of_country(&self.leaving, agent);
        if leaves { 1.0 } else { 0.0 }
    }
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        (p.role == CHILD.name).then(|| p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, _: &AgentView<'_>, h: &mut Household, reached: &[usize], _: &mut Draws) {
        for i in reached {
            if let Some(p) = h.persons.get_mut(*i).filter(|p| p.role == CHILD.name) {
                p.role = ADULT.name;
            }
        }
    }
}
