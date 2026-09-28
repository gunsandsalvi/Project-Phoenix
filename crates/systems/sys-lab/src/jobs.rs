//! The households' jobs and their persons' labour state at the opening. Each adult is employed at the country's
//! employment rate, and an employee at its sex's share of employees among the employed. An employee's job has an
//! occupation drawn by its sex among those its skill reaches, part-time hours at its sex's share, the law's notice and
//! severance, its household's region and the band its tenure, drawn from the tenure shares, began in. Its wage is its
//! hours times the mean wage the labour share gives an employee's mean hours, times its household's income as a
//! multiple of the mean, on the nearest wage point. A job is a row on the employment line of its class and point. Once every household is drawn, each region and
//! occupation's jobs are apportioned over the region's firms by their headcounts and the share of the occupation in
//! their product's work, and dealt to the lines in an order drawn by lot. Of those not employed, the
//! unemployed search, at the rate that makes their share of the labour force the country's; an adult past its
//! pension's age not employed is retired.

use if_labour::class::{NO_OCCUPATION, NOT_SEARCHING, PLACES, RETIRED, SEARCHING};
use if_labour::law::Law;
use phx_core::register::values::{Table1, Table2};
use phx_core::{OpeningCountry, Prim, Register, declare_stream};
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::opening::whole;
use phx_ledger::rows::BALANCE;
use phx_macros::clause;
use phx_num::{Count, violation};
use phx_rand::{Draws, open_unit};

use crate::consts::{EMPLOYEES, SHARE_PARTS};

declare_stream! { pub JobsStream = "LAB.opening_jobs" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// Employment: the employer owes the wage to the employee, each job a member; many employers and many employees on
/// a line, so it records no pairing.
pub const EMPLOYMENT: LineKindDecl = LineKindDecl {
    name: if_labour::consts::EMPLOYMENT_LINE,
    asset: SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name],
        exclusive: true,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &["firm", "small_firm", phx_core::ESTATE_KIND.name],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    dated: true,
    transfer_requesters: &["LAB"],
};

/// Labour's draw of the households' jobs.
#[derive(Debug)]
pub struct Jobs {
    pub status: Prim<Table2>,
    pub occupation: Prim<Table2>,
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
    employees: [f64; 2],
    part_time: [f64; 2],
    part_time_hours: u32,
    searching: [f64; 2],
    occupations: Vec<[f64; 2]>,
    tenure: Vec<(i64, f64)>,
    wage: f64,
    /// An employee's mean weekly hours, over the sexes' shares of employees and their part-time shares.
    mean_hours: f64,
    date: phx_id::Date,
}

/// An adult's labour as drawn: its place in its household, its state, the occupation recorded on it, the wage point
/// full time would pay it, and its job where it is an employee.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub place: usize,
    pub state: u32,
    pub occupation: u32,
    pub last: u32,
    pub job: Option<DrawnJob>,
}

/// An employee's job as drawn: its wage point and its class — occupation, skill, hours, notice, severance, region
/// and the band its tenure began in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawnJob {
    pub point: i64,
    pub class: Vec<u32>,
}

impl Jobs {
    /// Labour's draw with its primitives found in the register.
    ///
    /// # Errors
    /// A primitive the draw reads that the register does not hold as declared.
    pub fn of(register: &Register) -> Result<Jobs, String> {
        Ok(Jobs {
            status: register.handle(&crate::STATUS)?,
            occupation: register.handle(&crate::OCCUPATION)?,
            by_age: register.handle(&crate::EMPLOYMENT_BY_AGE)?,
            unemployment: register.handle(&crate::UNEMPLOYMENT)?,
            part_time: register.handle(&crate::PART_TIME)?,
            part_time_hours: register.handle(&crate::PART_TIME_HOURS)?,
            tenure: register.handle(&crate::TENURE)?,
        })
    }

    /// A country's labour as its households draw it, on the opening's date.
    #[clause("GEN.2", "LAB.1")]
    #[must_use]
    pub fn rule(&self, register: &Register, date: phx_id::Date, c: &OpeningCountry) -> Rule {
        let status = self.status.get(register, c.id);
        let employees = [if_pop::FEMALE, if_pop::MALE].map(|sex| {
            let Ok(v) = status.at(EMPLOYEES, i64::from(sex)) else {
                violation!(clause = "GEN.2", "no share of employees for a sex", sex = sex);
            };
            share(v)
        });
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
        let shares = self.occupation.get(register, c.id);
        let occupations = (0..i64::from(NO_OCCUPATION))
            .map(|o| {
                [if_pop::FEMALE, if_pop::MALE].map(|sex| match shares.at(o, i64::from(sex)) {
                    Ok(v) => share(v),
                    Err(_) => violation!(clause = "GEN.2", "no share for an occupation", occupation = o),
                })
            })
            .collect();
        let t = self.tenure.get(register, c.id);
        let tenure = t.axis().iter().copied().zip(t.values().iter().map(|v| share(*v))).collect();
        let Ok(part_time_hours) = u32::try_from(self.part_time_hours.get(register, c.id).get()) else {
            violation!(clause = "LAB.1", "part-time hours beyond a week's", country = c.id.get())
        };
        let of_employees: f64 = employees.iter().sum();
        let mean_hours: f64 = employees
            .iter()
            .zip(&part_time)
            .map(|(e, p)| {
                e / of_employees * (p * f64::from(part_time_hours) + (1.0 - p) * f64::from(law.full_time_hours))
            })
            .sum();
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
        Rule {
            law,
            employed,
            by_age,
            employees,
            part_time,
            part_time_hours,
            searching,
            occupations,
            tenure,
            wage: phx_ledger::opening::mean_wage(c),
            mean_hours,
            date,
        }
    }
}

impl Rule {
    /// The wage point nearest a month's wage.
    fn point_of(&self, wage: f64) -> u32 {
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

    /// An occupation drawn by sex among those the skill reaches.
    fn occupation(&self, sex: usize, skill: u32, d: &mut Draws) -> u32 {
        let open: Vec<(u32, f64)> = (0_u32..)
            .zip(&self.occupations)
            .filter(|(o, _)| {
                let least = usize::try_from(*o).ok().and_then(|o| self.law.occupation_skill.get(o));
                least.is_some_and(|l| *l <= skill)
            })
            .filter_map(|(o, s)| s.get(sex).map(|v| (o, *v)))
            .collect();
        let total: f64 = open.iter().map(|(_, s)| s).sum();
        let mut u = open_unit(d) * total;
        for (o, s) in &open {
            if u < *s {
                return *o;
            }
            u -= s;
        }
        open.last().map_or(NO_OCCUPATION, |(o, _)| *o)
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
    /// and sex's ratio to it, an employee at its sex's share, with its job's occupation, hours, wage point and class;
    /// of those not employed, searching at the rate that makes the unemployed the country's share, or retired past
    /// the pension's age. A week's hour of the household's work pays the mean wage over an employee's mean hours at
    /// its income's multiple.
    #[clause("GEN.2", "LAB.1", "REP.34", "PTY.3")]
    #[must_use]
    pub fn draw(&self, household: &phx_core::Household, income: f64, d: &mut Draws) -> Vec<Drawn> {
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        let region = household.attr(if_pop::REGION.name);
        let hourly = self.wage * income / self.mean_hours;
        let last = self.point_of(hourly * f64::from(self.law.full_time_hours));
        let mut out = Vec::new();
        for (place, p) in household.persons.iter().enumerate().filter(|(_, p)| adult_roles.contains(&p.role)) {
            let Some(sex) = p.attr(if_pop::SEX.name) else { violation!(clause = "REP.26", "a person with no sex") };
            let s = usize::try_from(sex).unwrap_or(usize::MAX);
            let (Some(employee), Some(part_time), Some(searching), Some(months)) = (
                self.employees.get(s).copied(),
                self.part_time.get(s).copied(),
                self.searching.get(s).copied(),
                self.law.pension_months.get(s).copied(),
            ) else {
                violation!(clause = "REP.26", "a sex beyond the two", sex = sex);
            };
            let education = p.attr(if_pop::EDUCATION.name).and_then(|e| usize::try_from(e).ok());
            let Some(skill) = education.and_then(|e| self.law.education_skill.get(e)).copied() else {
                violation!(clause = "LAB.3", "an education that gives no skill level");
            };
            let age = p.age_on(self.date);
            let retired = age * crate::consts::MONTHS_A_YEAR >= months;
            let Some(ratio) = self.by_age.iter().rev().find(|(first, _)| *first <= age).and_then(|(_, r)| r.get(s))
            else {
                violation!(clause = "GEN.2", "an adult younger than the employment bands", age = age);
            };
            let (works, as_employee, looks) = (open_unit(d), open_unit(d), open_unit(d));
            let occupation = self.occupation(s, skill, d);
            if works >= self.employed * ratio {
                let state = if retired {
                    RETIRED
                } else if looks < searching {
                    SEARCHING
                } else {
                    NOT_SEARCHING
                };
                let known = if state == SEARCHING { occupation } else { NO_OCCUPATION };
                out.push(Drawn { place, state, occupation: known, last, job: None });
                continue;
            }
            let state = if retired { RETIRED } else { NOT_SEARCHING };
            if as_employee >= employee {
                out.push(Drawn { place, state, occupation, last, job: None });
                continue;
            }
            let hours = if open_unit(d) < part_time { self.part_time_hours } else { self.law.full_time_hours };
            let point = i64::from(self.point_of(hourly * f64::from(hours)));
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
            out.push(Drawn { place, state, occupation, last, job: Some(DrawnJob { point, class }) });
        }
        out
    }
}
