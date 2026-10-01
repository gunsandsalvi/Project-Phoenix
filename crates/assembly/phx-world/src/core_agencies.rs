//! The public agencies on the core. Each country's agency produces its public administration: its staff are the
//! public administration's share of each occupation's jobs at the opening, its purchases the state's final uses, and it
//! is funded by its treasury as it pays. Its head keeps its staff: each business day it posts what it lacks of its
//! opening staff by region and occupation, as far as its appropriation for wages — what its opening staff were paid a
//! month, a placeholder naming POL until the budget votes it — pays.

use std::collections::BTreeMap;

use if_labour::law::Law;
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_market::hiring::Vacancy;

use crate::core::Core;
use crate::core_labour::{LabourCtx, Posting, at_country};

/// The public administration's contracts.
pub const PUBLIC: &str = crate::consts::families::PUBLIC_EMPLOYMENT;

/// An agency's day: its staff and wage bill a month, its appropriation for wages and what it posted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct AgencyDay {
    pub day: u32,
    pub country: u8,
    pub staff: u64,
    pub bill: i64,
    pub budget: i64,
    pub posted: u64,
}

/// The agencies' days; the staff each keeps by region and occupation and its appropriation for wages a month are its
/// kind's store's.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Agencies {
    pub days: Vec<AgencyDay>,
}

impl Core {
    /// The public administration's jobs, each with its agency, region, occupation and monthly wage: every agency's,
    /// or one's.
    fn posts_held(&self, only: Option<PartyKey>) -> Vec<(PartyKey, u32, u32, i64)> {
        let Some(family) = self.bound.families.public_employment.and_then(|i| self.families.get(i)) else {
            return Vec::new();
        };
        let edges: Vec<Slot> = match only {
            Some(agency) => family.store.of(0, agency.slot()).collect(),
            None => family.store.edges.open_slots().collect(),
        };
        let mut out = Vec::with_capacity(edges.len());
        for edge in edges {
            let Some(row) = family.store.edges.row(edge) else { continue };
            let Some([occupation, _, _]) =
                family.classes.get(usize::try_from(row.schedule).unwrap_or(usize::MAX)).copied()
            else {
                continue;
            };
            if let Some(region) = self.household_region(row.ends[1].slot()) {
                out.push((row.ends[0], region, occupation, row.amount));
            }
        }
        out
    }

    /// Each agency's staff and wages at the opening, which it keeps.
    #[clause("SOC.2", "SOC.8")]
    pub fn open_agencies(&mut self) {
        let posts = self.posts_held(None);
        let Some(store) = self.agency_store.as_mut() else { return };
        for (agency, region, occupation, amount) in posts {
            store.keep_post(agency.slot(), (region, occupation), amount);
        }
        self.agencies_kept = Agencies::default();
    }

    /// Each agency of a country on its business day posts what it lacks of the staff it keeps, its head's decision
    /// within its appropriation; its vacancies standing past its country's patience are raised a point, as any
    /// employer's. Returns the jobs posted.
    #[clause("SOC.8", "LAB.4", "MND.20")]
    pub(crate) fn post_agencies(&mut self, ctx: &LabourCtx<'_>, day: Day) -> u64 {
        let Some(family) = self.bound.families.public_employment else { return 0 };
        let staffing = self.point(|p| p.staff, &sys_soc::points::STAFF);
        let mut posted = 0;
        for (c, agency) in self.agencies.clone().into_iter().enumerate() {
            let (Some(agency), Ok(ccy)) = (agency, u8::try_from(c)) else { continue };
            if !ctx.calendar.is_business(CountryId::new(ccy), day) {
                continue;
            }
            let law = at_country(&self.labour.laws, ccy).clone();
            let own: Vec<usize> = (0..self.labour.vacancies.len())
                .filter(|i| self.labour.vacancies.get(*i).is_some_and(|v| v.employer == agency && v.open > 0))
                .collect();
            self.raise_stale(ctx, day, &own, &law);
            let staff = self.staff_of(family, agency);
            let means = crate::core_labour::staff_means(&staff);
            let mut have: BTreeMap<(u32, u32), u32> = BTreeMap::new();
            for (_, region, occupation, _) in self.posts_held(Some(agency)) {
                *have.entry((region, occupation)).or_insert(0) += 1;
            }
            let mut bill: f64 = staff.iter().map(|(_, _, a)| phx_rand::float::from_i64(*a)).sum();
            for i in &own {
                if let Some(v) = self.labour.vacancies.get(*i) {
                    *have.entry((v.region, v.occupation)).or_insert(0) += v.open;
                    bill += v.wage * f64::from(v.open);
                }
            }
            let Some(record) = self.agency_store.as_ref().and_then(|a| a.staffing(agency.slot())) else { continue };
            let budget = record.budget();
            let mut gaps = Vec::new();
            for (region, occupation, target) in record.targets() {
                let held = have.get(&(region, occupation)).copied().unwrap_or(0);
                if held < target {
                    let point = self.offer_point(ctx, &law, (agency, occupation), &means);
                    gaps.push(sys_soc::points::Gap {
                        region,
                        occupation,
                        jobs: target - held,
                        wage: ctx.wage_at(&law, point),
                    });
                }
            }
            let posts = self.decide(staffing, agency, |_| sys_soc::points::StaffIn {
                gaps,
                budget: phx_rand::float::from_i64(budget),
                bill,
            });
            for (region, occupation, jobs) in posts {
                posted += u64::from(jobs);
                self.post_vacancy(ctx, (agency, ccy, &law), (region, occupation, jobs), (day, &means));
            }
            let bill_now: i64 = staff.iter().map(|(_, _, a)| *a).sum();
            self.agencies_kept.days.push(AgencyDay {
                day: day.get(),
                country: ccy,
                staff: phx_rand::float::len_u64(staff.len()),
                bill: bill_now,
                budget,
                posted,
            });
        }
        posted
    }

    /// A vacancy posted at the point its fills show the market pays.
    fn post_vacancy(
        &mut self,
        ctx: &LabourCtx<'_>,
        (employer, country, law): (PartyKey, u8, &Law),
        (region, occupation, open): (u32, u32, u32),
        (day, means): (Day, &crate::core_labour::StaffMeans),
    ) {
        let Some(skill) = usize::try_from(occupation).ok().and_then(|o| law.occupation_skill.get(o)).copied() else {
            return;
        };
        let point = self.offer_point(ctx, law, (employer, occupation), means);
        self.labour.vacancies.push(Vacancy {
            employer,
            region,
            occupation,
            skill,
            wage: ctx.wage_at(law, point),
            open,
        });
        self.labour.postings.push(Posting { id: self.labour.next_vacancy, point, first: day, set: day, country });
        self.labour.next_vacancy += 1;
    }

    /// The contracts' family an employer's jobs are in: a firm's, or the state's agency's.
    pub(crate) fn employer_family(&self, employer: PartyKey) -> Option<usize> {
        self.families
            .iter()
            .enumerate()
            .position(|(i, f)| self.bound.is_jobs(i) && f.store.kinds.first() == Some(&employer.kind()))
    }

    /// Every job a person holds, in any employer's family, closed: it takes another.
    pub(crate) fn quit_jobs(&mut self, household: PartyKey, person: u64) -> u64 {
        let mut n = 0;
        let jobs = &self.bound;
        for (_, f) in self.families.iter_mut().enumerate().filter(|(i, _)| jobs.is_jobs(*i)) {
            let held: Vec<Slot> = f
                .store
                .of(1, household.slot())
                .filter(|e| f.store.edges.row(*e).is_some_and(|r| r.person == person))
                .collect();
            for e in held {
                f.store.close(e);
                self.labour.noticed.remove(&e.get());
                n += 1;
            }
        }
        n
    }
}
