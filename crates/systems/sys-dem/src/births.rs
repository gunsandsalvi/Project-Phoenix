//! Births: a household decides on each of its head's birthdays whether to try for a child, drawing its ideal number
//! of children at its first decision; while it tries, each woman of its couple conceives by her age's chance a cycle,
//! and a conception is a birth, a child of the household in school.

use if_pop::fertility::{IDEAL, IDEAL_UNDRAWN, NOT_TRYING, TRYING, TRYING_FOR_CHILD};
use if_pop::{ABLE, CHILD, EDUCATION, EDUCATION_UNRECORDED, FEMALE, HEAD, HEALTH, MALE, PARTNER, SEX};
use phx_core::register::values::{Family, Table1, TypeSet, draw_type};
use phx_core::{AgentView, FactDef, Household, Person, PopProcess, Register};
use phx_id::Date;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::Draws;
use phx_rand::float::from_i64;

use crate::Prims;
use crate::fertility::{ChildIn, Scale, logistic, tries};
use crate::processes::{age, of_country, per_country, sex};

/// The decision to try for a child, taken on the head's birthday: the household's inputs read from its persons, its
/// outlook of its income and its ideal, drawn from its country's preference at its first decision and held.
#[clause("POP.10", "POP.2", "NUM.4")]
#[derive(Debug)]
pub struct Fertility {
    prims: Prims,
    scale: Option<Scale>,
    spread: f64,
    ideals: Vec<TypeSet>,
}

impl Fertility {
    #[must_use]
    pub fn new(prims: Prims) -> Fertility {
        Fertility { prims, scale: None, spread: 0.0, ideals: Vec::new() }
    }

    fn scale(&self) -> Scale {
        let Some(s) = self.scale else { violation!(clause = "POP.16", "a fertility decision before its binding") };
        s
    }

    /// The household's ideal, drawn at its first decision.
    fn ideal(&self, agent: &AgentView<'_>, h: &mut Household, d: &mut Draws) -> u32 {
        let held = h.attr(IDEAL.name);
        if held != IDEAL_UNDRAWN {
            return held - 1;
        }
        let set = of_country(&self.ideals, agent);
        let id = draw_type(set, d);
        let drawn = set.types().iter().find(|t| t.id == id).map(|t| t.value);
        let Some(ideal) = drawn.and_then(|n| u32::try_from(n).ok()) else {
            violation!(clause = "POP.16", "an ideal number of children beyond counting");
        };
        if ideal + 1 >= IDEAL.values {
            phx_num::capacity_exceeded!("a household's ideal number of children", IDEAL.values - 1, ideal);
        }
        h.set_attr(IDEAL.name, ideal + 1);
        ideal
    }
}

impl PopProcess for Fertility {
    fn bind(&mut self, register: &Register) {
        let t = self.prims.equivalence.shared(register);
        let hundredths = |i: usize| t.values().get(i).map(|v| from_i64(*v) / crate::consts::PERCENT);
        let (Some(adult), Some(child)) = (hundredths(0), hundredths(1)) else {
            violation!(clause = "POP.16", "an equivalence scale without an adult's and a child's weight");
        };
        self.scale = Some(Scale { adult, child });
        self.spread = self.prims.taste_spread.shared(register).to_f64();
        self.ideals = per_country(register, |c| {
            let distribution = self.prims.ideal_children.get(register, c);
            let Family::Discrete { values, .. } = &distribution.family else {
                violation!(
                    clause = "POP.16",
                    "an ideal number of children not declared by its shares",
                    country = c.get()
                );
            };
            let set =
                u16::try_from(values.len()).map_err(|e| e.to_string()).and_then(|n| TypeSet::build(distribution, n));
            set.unwrap_or_else(|_| {
                violation!(clause = "NUM.4", "an ideal number of children's shares refused", country = c.get())
            })
        });
    }
    fn hazard(&self) -> &'static str {
        crate::OCCASION.name
    }
    fn kind(&self) -> &'static str {
        if_pop::HOUSEHOLD
    }
    /// Certain on the head's birthday, and nothing on any other day.
    fn rate(&self, _: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        let birthday = p.role == HEAD.name && p.birthday_in(agent.date.year()) == agent.date;
        if birthday { 1.0 } else { 0.0 }
    }
    /// The head's next birthday; on the birthday itself the decision is certain, so no change is read.
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        (p.role == HEAD.name).then(|| p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, agent: &AgentView<'_>, h: &mut Household, _: &[usize], d: &mut Draws) {
        let ideal = self.ideal(agent, h, d);
        let taste = self.spread * logistic(phx_rand::open_unit(d));
        let income = match h.position(<if_pop::facts::Income as FactDef>::ITEM.name) {
            Missing::Present(y) => Some(from_i64(y)),
            Missing::Absent => None,
        };
        let (mut adults, mut children, mut youngest) = (0_u32, 0_u32, None::<i64>);
        for (_, p) in h.present() {
            if p.role == CHILD.name {
                children += 1;
                let a = p.age_on(agent.date);
                youngest = Some(youngest.map_or(a, |y| if a < y { a } else { y }));
            } else {
                adults += 1;
            }
        }
        let decided = ChildIn { income, adults, children, youngest, ideal, taste };
        h.set_attr(TRYING.name, if tries(&decided, self.scale()) { TRYING_FOR_CHILD } else { NOT_TRYING });
    }
}

/// Conception: while its household tries, a woman of its couple conceives by her age's chance a cycle, compounded
/// over the cycle's days. The conception is a birth: a child of the household, its sex drawn by the sex ratio at
/// birth, able and in school; the household decides afresh at its next occasion.
#[clause("POP.5", "CHN.3", "POP.16", "REP.25", "REP.26")]
#[derive(Debug)]
pub struct Conception {
    prims: Prims,
    table: Option<Table1>,
    cycle: f64,
    male_share: Vec<f64>,
}

impl Conception {
    #[must_use]
    pub fn new(prims: Prims) -> Conception {
        Conception { prims, table: None, cycle: 0.0, male_share: Vec::new() }
    }
}

/// Whether a person is a woman of the household's couple.
fn at_risk(p: &Person) -> bool {
    (p.role == HEAD.name || p.role == PARTNER.name) && sex(p) == FEMALE
}

impl PopProcess for Conception {
    fn bind(&mut self, register: &Register) {
        self.table = Some(self.prims.fecundability.shared(register).clone());
        self.cycle = phx_rand::float::from_u64(self.prims.cycle_days.shared(register).get());
        self.male_share = per_country(register, |c| {
            let ratio = self.prims.sex_ratio.get(register, c).to_f64();
            ratio / (crate::consts::PERCENT + ratio)
        });
    }
    fn hazard(&self) -> &'static str {
        crate::CONCEPTION.name
    }
    fn kind(&self) -> &'static str {
        if_pop::HOUSEHOLD
    }
    fn rate(&self, _: &Register, agent: &AgentView<'_>, p: &Person) -> f64 {
        if (agent.attr)(TRYING.name) != Some(TRYING_FOR_CHILD) || !at_risk(p) {
            return 0.0;
        }
        let Some(table) = &self.table else { violation!(clause = "POP.16", "a conception before its binding") };
        let Ok(a) = i64::try_from(age(p, agent.date)) else { violation!(clause = "REP.25", "an age beyond counting") };
        let Ok(parts) = table.at(a) else {
            violation!(clause = "POP.16", "a woman's age outside the fecundability table", age = a);
        };
        let chance = from_i64(parts) / crate::consts::MILLION;
        -libm::expm1(libm::log1p(-chance) / self.cycle)
    }
    fn changes_after(&self, p: &Person, date: Date) -> Option<Date> {
        at_risk(p).then(|| p.next_birthday(date))
    }
    fn outcome(&self, _: &Register, agent: &AgentView<'_>, h: &mut Household, reached: &[usize], d: &mut Draws) {
        let male_share = *of_country(&self.male_share, agent);
        for _ in reached {
            let s = if phx_rand::open_unit(d) < male_share { MALE } else { FEMALE };
            h.persons.push(Person {
                role: CHILD.name,
                born: agent.date,
                attrs: vec![(SEX.name, s), (HEALTH.name, ABLE), (EDUCATION.name, EDUCATION_UNRECORDED)],
                gone: false,
            });
        }
        h.set_attr(TRYING.name, NOT_TRYING);
    }
}
