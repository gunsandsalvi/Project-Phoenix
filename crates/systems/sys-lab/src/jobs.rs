//! The households' jobs and their persons' labour state at the opening. Each adult is employed at the country's
//! employment rate. What it does is what the country's output asks: the hours the activities ask of each occupation,
//! as employees' or as the self-employed's, are shared over the sexes by each sex's share of the occupation, and each
//! sex's employed fill them by rank — ordered by the skill their education gives, least first, against the occupations
//! ordered by the skill each asks — so the most schooled fill the most skilled work, and where schooling is short of
//! what the work asks, the work is learnt by doing it. A person's occupation and whether it runs its own business
//! are drawn by the chances its sex and skill have there. An employee's job has part-time hours at its sex's share, the law's notice and severance, its
//! household's region and the band its tenure, drawn from the tenure shares, began in. Its wage is not drawn: it
//! follows from its employer's activity once each region and occupation's jobs are dealt over the region's employers
//! by the hours their output takes of the occupation, in an order drawn by lot. Of those not employed, the unemployed
//! search, for an occupation drawn as an employee's, at the rate that makes their share of the labour force the
//! country's; an adult past its pension's age not employed is retired.

use std::collections::BTreeMap;

use if_labour::class::{NO_OCCUPATION, NOT_SEARCHING, PLACES, RETIRED, SEARCHING};
use if_labour::law::Law;
use phx_core::register::values::{Table1, Table2};
use phx_core::{OpeningCountry, Prim, Register, declare_stream};
use phx_ledger::opening::whole;
use phx_macros::clause;
use phx_num::{Count, violation};
use phx_rand::{Draws, open_unit};

use crate::consts::SHARE_PARTS;

declare_stream! { pub JobsStream = "LAB.opening_jobs" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// Where the hours an occupation is asked for as employees' are, among its asked hours.
pub const AS_EMPLOYEE: usize = 0;
/// Where the hours an occupation is asked for as the self-employed's are, among its asked hours.
pub const AS_OWNER: usize = 1;

/// Labour's draw of the households' jobs.
#[derive(Debug)]
pub struct Jobs {
    pub women: Prim<Table1>,
    pub by_age: Prim<Table2>,
    pub unemployment: Prim<Table1>,
    pub part_time: Prim<Table1>,
    pub part_time_hours: Prim<Count>,
    pub tenure: Prim<Table1>,
}

/// A share of a published table, as a probability.
fn share(v: i64) -> f64 {
    phx_rand::float::from_i64(v) / SHARE_PARTS
}

/// A value of a table of one axis by sex.
fn by_sex(t: &Table1, sex: u32) -> f64 {
    let Ok(v) = t.at(i64::from(sex)) else {
        violation!(clause = "GEN.2", "a table by sex with no value for a sex", sex = sex);
    };
    share(v)
}

/// One country's labour as its households draw it, read from the register alone.
#[derive(Clone, Debug)]
pub struct Rule {
    law: Law,
    employed: f64,
    /// Each ten-year band's first age and each sex's employment rate there over the country's.
    by_age: Vec<(i64, [f64; 2])>,
    part_time: [f64; 2],
    part_time_hours: u32,
    searching: [f64; 2],
    /// Each occupation's share of its employed of each sex.
    sexes: Vec<[f64; 2]>,
    /// Each occupation's hours a year the country's output asks, as employees' and as the self-employed's.
    asked: Vec<[f64; 2]>,
    /// For each sex, each skill's chance of each occupation as an employee and running its own business; empty until
    /// the country's households are matched.
    by_skill: [BTreeMap<u32, Vec<[f64; 2]>>; 2],
    tenure: Vec<(i64, f64)>,
    date: phx_id::Date,
}

/// Each skill's chance of each occupation, as an employee and running its own business: the persons of each skill
/// (`supply`), least first, fill the persons each occupation asks (`asked`, with the skill it asks), scaled to all of
/// them, the occupations taken by the skill they ask, least first; a skill's persons filling a skill of work are spread
/// over its occupations and ways by what each asks.
#[clause("GEN.2", "LAB.3")]
#[must_use]
pub fn by_rank(supply: &BTreeMap<u32, f64>, asked: &[(u32, [f64; 2])]) -> BTreeMap<u32, Vec<[f64; 2]>> {
    let supplied: f64 = supply.values().sum();
    let mut levels: BTreeMap<u32, f64> = BTreeMap::new();
    for (level, ways) in asked {
        *levels.entry(*level).or_insert(0.0) += ways.iter().sum::<f64>();
    }
    let wanted: f64 = levels.values().sum();
    let mut out: BTreeMap<u32, Vec<[f64; 2]>> = BTreeMap::new();
    if !(supplied > 0.0 && wanted > 0.0) {
        return out;
    }
    let mut open = levels.iter().map(|(l, w)| (*l, *w, w * supplied / wanted));
    let mut at = open.next();
    for (skill, persons) in supply {
        let chances = out.entry(*skill).or_insert_with(|| vec![[0.0; 2]; asked.len()]);
        let mut left = *persons;
        while left > 0.0 {
            let Some((level, of_level, room)) = at else { break };
            let taken = if left < room { left } else { room };
            for ((l, ways), c) in asked.iter().zip(chances.iter_mut()) {
                if *l == level {
                    for (c, w) in c.iter_mut().zip(ways) {
                        *c += taken * w / of_level / persons;
                    }
                }
            }
            left -= taken;
            at = if taken < room { Some((level, of_level, room - taken)) } else { open.next() };
        }
    }
    out
}

/// An adult's labour as drawn: its place in its household, its state, the occupation recorded on it, and its job's
/// class where it is an employee — occupation, skill, hours, notice, severance, region and the band its tenure began
/// in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub place: usize,
    pub state: u32,
    pub occupation: u32,
    pub job: Option<Vec<u32>>,
}

impl Jobs {
    /// Labour's draw with its primitives found in the register.
    ///
    /// # Errors
    /// A primitive the draw reads that the register does not hold as declared.
    pub fn of(register: &Register) -> Result<Jobs, String> {
        Ok(Jobs {
            women: register.handle(&crate::WOMEN)?,
            by_age: register.handle(&crate::EMPLOYMENT_BY_AGE)?,
            unemployment: register.handle(&crate::UNEMPLOYMENT)?,
            part_time: register.handle(&crate::PART_TIME)?,
            part_time_hours: register.handle(&crate::PART_TIME_HOURS)?,
            tenure: register.handle(&crate::TENURE)?,
        })
    }

    /// A country's labour as its households draw it, on the opening's date, from each occupation's hours a year its
    /// output asks as employees' and as the self-employed's.
    #[clause("GEN.2", "LAB.1")]
    #[must_use]
    pub fn rule(&self, register: &Register, date: phx_id::Date, c: &OpeningCountry, asked: Vec<[f64; 2]>) -> Rule {
        let Ok(law) = crate::law::law(register, c) else {
            violation!(clause = "LAB.16", "a country's labour law the register does not hold", country = c.id.get());
        };
        let d = |name| phx_ledger::opening::derived(c, name);
        let employed = d("GEN.employment_rate") / crate::consts::PERCENT;
        let unemployment = self.unemployment.get(register, c.id);
        // The unemployed as a share of those not employed: u·E / ((1 − u)(1 − E)), the labour force being E / (1 − u).
        let searching = [if_pop::FEMALE, if_pop::MALE].map(|sex| {
            let u = by_sex(unemployment, sex);
            u * employed / ((1.0 - u) * (1.0 - employed))
        });
        let part = self.part_time.get(register, c.id);
        let part_time = [if_pop::FEMALE, if_pop::MALE].map(|sex| by_sex(part, sex));
        let women = self.women.get(register, c.id);
        let sexes: Vec<[f64; 2]> = (0..i64::from(NO_OCCUPATION))
            .map(|o| match women.at(o) {
                Ok(v) => {
                    let w = share(v);
                    let mut by = [0.0; 2];
                    for (sex, part) in [(if_pop::FEMALE, w), (if_pop::MALE, 1.0 - w)] {
                        if let Some(b) = by.get_mut(usize::try_from(sex).unwrap_or(usize::MAX)) {
                            *b = part;
                        }
                    }
                    by
                }
                Err(_) => violation!(clause = "GEN.2", "no women's share for an occupation", occupation = o),
            })
            .collect();
        if asked.len() != sexes.len() {
            violation!(clause = "GEN.2", "hours asked of other occupations than the persons'", country = c.id.get());
        }
        let t = self.tenure.get(register, c.id);
        let tenure = t.axis().iter().copied().zip(t.values().iter().map(|v| share(*v))).collect();
        let Ok(part_time_hours) = u32::try_from(self.part_time_hours.get(register, c.id).get()) else {
            violation!(clause = "LAB.1", "part-time hours beyond a week's", country = c.id.get())
        };
        let t = self.by_age.get(register, c.id);
        let by_age = t
            .rows()
            .iter()
            .map(|a| {
                (
                    *a,
                    [if_pop::FEMALE, if_pop::MALE].map(|sex| match t.at(*a, i64::from(sex)) {
                        Ok(v) => share(v),
                        Err(_) => violation!(clause = "GEN.2", "no employment ratio for an age and sex", age = *a),
                    }),
                )
            })
            .collect();
        let by_skill = [BTreeMap::new(), BTreeMap::new()];
        Rule { law, employed, by_age, part_time, part_time_hours, searching, sexes, asked, by_skill, tenure, date }
    }
}

impl Rule {
    /// The wage point nearest a month's wage.
    #[must_use]
    pub fn point_of(&self, wage: f64) -> u32 {
        let Some(point) = phx_rand::float::floor_to_i64(libm::rint(libm::log(wage) / libm::log(self.law.point_ratio)))
        else {
            violation!(clause = "REP.34", "a wage beyond the wage points");
        };
        let Some(point) = u32::try_from(point).ok().filter(|p| *p < if_labour::class::WAGE_POINTS) else {
            violation!(clause = "REP.34", "a wage point beyond those a person can record", point = point);
        };
        point
    }

    /// A wage point's month's wage in whole smallest units.
    #[must_use]
    pub fn wage_at(&self, point: i64) -> i64 {
        whole(libm::pow(self.law.point_ratio, phx_rand::float::from_i64(point)))
    }

    /// An adult's sex, the skill its education gives and its chance of being employed: the country's rate times its
    /// age band's and sex's ratio to it; none for a person who is not an adult.
    fn adult(&self, p: &phx_core::Person) -> Option<(usize, u32, f64)> {
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        if !adult_roles.contains(&p.role) {
            return None;
        }
        let Some(sex) = p.attr(if_pop::SEX.name) else { violation!(clause = "REP.26", "a person with no sex") };
        let s = usize::try_from(sex).unwrap_or(usize::MAX);
        let education = p.attr(if_pop::EDUCATION.name).and_then(|e| usize::try_from(e).ok());
        let Some(skill) = education.and_then(|e| self.law.education_skill.get(e)).copied() else {
            violation!(clause = "LAB.3", "an education that gives no skill level");
        };
        let age = p.age_on(self.date);
        let Some(ratio) = self.by_age.iter().rev().find(|(first, _)| *first <= age).and_then(|(_, r)| r.get(s)) else {
            violation!(clause = "GEN.2", "an adult younger than the employment bands", age = age);
        };
        Some((s, skill, self.employed * ratio))
    }

    /// The country's households matched to what its output asks: each sex's employed by skill — each adult counted
    /// at its chance of being employed — filling by rank the hours asked of each occupation, shared over the sexes by
    /// each sex's share of it.
    #[clause("GEN.2", "LAB.3")]
    pub fn couple<'h>(&mut self, households: impl Iterator<Item = &'h phx_core::Household>) {
        let mut supply: [BTreeMap<u32, f64>; 2] = [BTreeMap::new(), BTreeMap::new()];
        for p in households.flat_map(|h| h.persons.iter()) {
            let Some((sex, skill, chance)) = self.adult(p) else { continue };
            // A uniform draw falls below a chance past one always.
            let chance = if chance < 1.0 { chance } else { 1.0 };
            if let Some(by) = supply.get_mut(sex) {
                *by.entry(skill).or_insert(0.0) += chance;
            }
        }
        for (sex, (by, of_sex)) in supply.iter().zip(self.by_skill.iter_mut()).enumerate() {
            let asked: Vec<(u32, [f64; 2])> = self
                .asked
                .iter()
                .zip(&self.sexes)
                .enumerate()
                .map(|(o, (ways, sexes))| {
                    let Some(level) = self.law.occupation_skill.get(o).copied() else {
                        violation!(clause = "LAB.3", "an occupation that asks no skill level", occupation = o);
                    };
                    let part = sexes.get(sex).copied().unwrap_or(f64::NAN);
                    (level, ways.map(|w| w * part))
                })
                .collect();
            *of_sex = by_rank(by, &asked);
        }
    }

    /// An occupation and the way it is worked — of `ways`, as an employee or running one's own business — drawn by
    /// the chances the country's match gives the sex and skill.
    fn occupation(&self, sex: usize, skill: u32, ways: &[usize], d: &mut Draws) -> (u32, usize) {
        let Some(chances) = self.by_skill.get(sex).and_then(|m| m.get(&skill)) else {
            violation!(clause = "GEN.2", "labour drawn for a sex and skill the match does not hold", skill = skill);
        };
        let open: Vec<(u32, usize, f64)> = (0_u32..)
            .zip(chances)
            .flat_map(|(o, c)| ways.iter().filter_map(move |w| c.get(*w).map(|v| (o, *w, *v))))
            .collect();
        let total: f64 = open.iter().map(|(_, _, s)| s).sum();
        if total <= 0.0 {
            violation!(clause = "GEN.2", "no occupation is asked of a skill", skill = skill);
        }
        let mut u = open_unit(d) * total;
        for (o, w, s) in &open {
            if u < *s {
                return (*o, *w);
            }
            u -= s;
        }
        open.iter().rev().find(|(_, _, s)| *s > 0.0).map_or((NO_OCCUPATION, AS_EMPLOYEE), |(o, w, _)| (*o, *w))
    }

    /// The first year of the band a job began in, its tenure drawn from the shares by band.
    fn band(&self, d: &mut Draws) -> u32 {
        let total: f64 = self.tenure.iter().map(|(_, s)| s).sum();
        let mut u = open_unit(d) * total;
        let mut years = 0_i64;
        for (i, (from, s)) in self.tenure.iter().enumerate() {
            let Some((to, _)) = self.tenure.get(i + 1) else { break };
            if u < *s {
                let within = u / s;
                let Some(into) = phx_rand::float::floor_to_i64(within * phx_rand::float::from_i64(*to - from)) else {
                    violation!(clause = "GEN.2", "a tenure beyond its band", band = *from);
                };
                years = from + into;
                break;
            }
            u -= s;
        }
        let began = i64::from(self.date.year()) - years;
        let band = i64::from(self.law.band_years);
        let Ok(first) = u32::try_from(began - began.rem_euclid(band)) else {
            violation!(clause = "REP.3", "a start band before the calendar's years", year = began);
        };
        first
    }

    /// Each adult's labour in a household drawn at the opening: employed at the country's rate times its age band's
    /// and sex's ratio to it, its occupation and whether it is an employee drawn from what the output asks, with its
    /// job's hours and class; of those not employed, searching at the rate that makes the unemployed the country's
    /// share, or retired past the pension's age.
    #[clause("GEN.2", "LAB.1", "PTY.3")]
    #[must_use]
    pub fn draw(&self, household: &phx_core::Household, d: &mut Draws) -> Vec<Drawn> {
        let region = household.attr(if_pop::REGION.name);
        let mut out = Vec::new();
        for (place, p) in household.persons.iter().enumerate() {
            let Some((s, skill, chance)) = self.adult(p) else { continue };
            let (Some(part_time), Some(searching), Some(months)) = (
                self.part_time.get(s).copied(),
                self.searching.get(s).copied(),
                self.law.pension_months.get(s).copied(),
            ) else {
                violation!(clause = "REP.26", "a sex beyond the two", sex = s);
            };
            let retired = p.age_on(self.date) * crate::consts::MONTHS_A_YEAR >= months;
            let (works, looks) = (open_unit(d), open_unit(d));
            if works >= chance {
                let state = if retired {
                    RETIRED
                } else if looks < searching {
                    SEARCHING
                } else {
                    NOT_SEARCHING
                };
                let known = if state == SEARCHING {
                    self.occupation(s, skill, &[AS_EMPLOYEE, AS_OWNER], d).0
                } else {
                    NO_OCCUPATION
                };
                out.push(Drawn { place, state, occupation: known, job: None });
                continue;
            }
            let state = if retired { RETIRED } else { NOT_SEARCHING };
            let (occupation, way) = self.occupation(s, skill, &[AS_EMPLOYEE, AS_OWNER], d);
            if way == AS_OWNER {
                out.push(Drawn { place, state, occupation, job: None });
                continue;
            }
            let hours = if open_unit(d) < part_time { self.part_time_hours } else { self.law.full_time_hours };
            let mut class = vec![0; PLACES];
            let places = [
                (if_labour::class::OCCUPATION, occupation),
                (if_labour::class::SKILL, skill),
                (if_labour::class::HOURS, hours),
                (if_labour::class::NOTICE, self.law.notice_days),
                (if_labour::class::SEVERANCE, self.law.severance_days_a_year),
                (if_labour::class::REGION, region),
                (if_labour::class::BAND, self.band(d)),
            ];
            for (i, v) in places {
                if let Some(c) = class.get_mut(i) {
                    *c = v;
                }
            }
            out.push(Drawn { place, state, occupation, job: Some(class) });
        }
        out
    }
}

#[path = "jobs_tests.rs"]
mod tests;
