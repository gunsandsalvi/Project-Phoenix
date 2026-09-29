//! Jobs on the core. Each job the opening drew for a household's person is dealt to one of the core's firms in
//! its region: a region's jobs of an occupation are shared over its firms by the hours their output takes of that
//! occupation, and dealt to them in an order drawn by lot. Each job is a contract from its firm to the household,
//! naming the person by its identity and paying the job's wage on its dates. Public administration's share of each
//! occupation's jobs, and every job whose occupation no firm's way in its region takes, is its country's public
//! agency's. The employed are the persons drawn employed, so the staff is exactly the employed the population gives.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::{OpeningCountry, Register, StreamDecl, Streams, opening_subject};
use phx_id::{Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::{floor_to_i64, from_i64, len_u64};

use crate::consts::firm::{JOBS_PURPOSE, OUTPUT, PRODUCT, PRODUCTIVITY, PRODUCTIVITY_ONE, PURPOSES, REGION};
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, CORE_WHEEL_DAYS};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due, WAGE};
use crate::opening::economy::table;

/// A job as dealt: its household, its person, its wage, where its schedule is, its region and occupation.
#[derive(Clone, Copy, Debug)]
struct Job {
    household: PartyKey,
    person: u64,
    amount: i64,
    schedule: u32,
    nth: u32,
    region: u32,
    occupation: u32,
}

impl Job {
    /// The job as a contract from its employer to its household, naming its person.
    fn due(&self, employer: PartyKey) -> Due {
        Due {
            ends: [employer, self.household],
            amount: self.amount,
            nth: self.nth,
            schedule: self.schedule,
            person: self.person,
            arrears: 0,
        }
    }
}

/// A job's next date, read from its schedule.
fn first_date(family: &DatedFamily, calendar: &Calendar, j: &Job) -> Option<Day> {
    family.schedules.get(usize::try_from(j.schedule).unwrap_or(usize::MAX)).map(|s| s.0.nth(calendar, j.nth))
}

/// Each region's firms, each with its slot and its hours a year of each occupation.
type ByRegion = BTreeMap<u32, Vec<(Slot, Vec<f64>)>>;

/// What the jobs' opening reads besides the core.
#[derive(Debug)]
pub struct JobsOpening<'a> {
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub calendar: &'a Calendar,
    pub today: Day,
    pub streams: &'a Streams,
    pub stream: &'a StreamDecl,
}

/// `n` split over weights by largest remainder, each part its weight's share rounded down and the rest one each to
/// the largest remainders, ties to the earlier; nothing where no weight is.
#[must_use]
pub fn deal(n: u64, weights: &[f64]) -> Vec<u64> {
    let whole: f64 = weights.iter().sum();
    if whole <= 0.0 {
        return vec![0; weights.len()];
    }
    let quotas: Vec<f64> = weights.iter().map(|w| phx_rand::float::from_u64(n) * w / whole).collect();
    let mut parts: Vec<u64> =
        quotas.iter().map(|q| floor_to_i64(q.floor()).and_then(|v| u64::try_from(v).ok()).unwrap_or(0)).collect();
    let given: u64 = parts.iter().sum();
    let mut order: Vec<usize> = (0..quotas.len()).collect();
    let rest = |i: &usize| quotas.get(*i).map_or(0.0, |q| q - q.floor());
    order.sort_by(|a, b| rest(b).total_cmp(&rest(a)).then(a.cmp(b)));
    for i in order.into_iter().take(usize::try_from(n - given).unwrap_or(usize::MAX)) {
        if let Some(p) = parts.get_mut(i) {
            *p += 1;
        }
    }
    parts
}

impl Core {
    /// The jobs the opening drew, each with its schedule in the family: its country's monthly dates and its class.
    fn drawn_jobs(&mut self, o: &JobsOpening<'_>, family: &mut DatedFamily) -> Result<Vec<Job>, String> {
        let date = o.calendar.date(o.today);
        let mut jobs = Vec::with_capacity(self.drawn.jobs.len());
        for j in std::mem::take(&mut self.drawn.jobs) {
            let Some(c) = o.countries.iter().find(|c| c.id.get() == j.country) else {
                return Err(format!("a job drawn in country {}, which the world does not hold", j.country));
            };
            let schedule = family.schedule_of((c, date), j.class, None);
            let [occupation, _, _] = j.class;
            jobs.push(Job {
                household: j.household,
                person: j.person,
                amount: j.amount,
                schedule,
                nth: 1,
                region: j.region,
                occupation,
            });
        }
        Ok(jobs)
    }

    /// Each firm's hours a year of each occupation its output takes: its output times its way's hours of the
    /// occupation a unit, fewer by its productivity's factor; by region, each firm with its slot.
    fn firm_hours(&self, o: &JobsOpening<'_>, firm: usize) -> Result<ByRegion, String> {
        let mut by_region: ByRegion = BTreeMap::new();
        let Some(store) = self.kinds.get(firm) else { return Ok(by_region) };
        let mut ways: BTreeMap<u8, Vec<Vec<f64>>> = BTreeMap::new();
        for c in o.countries {
            ways.insert(c.id.get(), table(o.register, "TEC.labour", c.id)?.0);
        }
        let country_of =
            |r: u32| o.countries.iter().find(|c| c.regions.iter().any(|(x, _)| *x == r)).map(|c| c.id.get());
        for slot in store.parties.live_slots() {
            let rec = store.record(slot);
            let read = |i: usize| match rec.get(i).map(|w| w.get()) {
                Some(phx_num::Missing::Present(v)) => v,
                _ => violation!(clause = "FRM.23", "a firm missing a word of its record", slot = slot.get()),
            };
            let (product, region) = (read(PRODUCT), u32::try_from(read(REGION)).unwrap_or(u32::MAX));
            let productivity = from_i64(read(PRODUCTIVITY)) / PRODUCTIVITY_ONE;
            let output = from_i64(read(OUTPUT));
            let Some(labour) = country_of(region).and_then(|c| ways.get(&c)) else {
                return Err(format!("a firm's region {region} in no country"));
            };
            let p = usize::try_from(product).unwrap_or(usize::MAX);
            let hours = labour
                .iter()
                .map(|occ| {
                    output * sys_frm::rules::way::own_hours(occ.get(p).copied().unwrap_or(f64::NAN), productivity)
                })
                .collect();
            by_region.entry(region).or_default().push((slot, hours));
        }
        Ok(by_region)
    }

    /// A family of jobs paid by one kind of employer to households.
    fn job_family(&mut self, name: &'static str, employer: usize, household: usize, today: Day) -> DatedFamily {
        DatedFamily {
            name,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(employer), kind_number(household)], [AGENT_ROWS, AGENT_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), CORE_WHEEL_DAYS),
            ),
            reason: WAGE,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            moves: crate::core_day::LoanMoves::default(),
            lost: 0,
        }
    }

    /// The jobs on the core: each region's jobs of an occupation, in an order drawn by lot, public administration's
    /// share of them its country's public agency's and the rest dealt over its firms by the hours their output takes
    /// of it — all the agency's where no firm's way takes the occupation — each a contract from its employer to the
    /// household naming the person. Returns the jobs the agencies hold.
    ///
    /// # Errors
    /// A primitive the dealing reads that the register does not hold, or a country with no agency to employ.
    #[clause("LAB.1", "REP.40", "GEN.2", "PTY.3", "SOC.2")]
    pub fn open_jobs(&mut self, o: &JobsOpening<'_>) -> Result<u64, String> {
        let (Some(firm), Some(household)) =
            (self.names.iter().position(|n| *n == "firm"), self.names.iter().position(|n| *n == "household"))
        else {
            return Ok(0);
        };
        let Some(agency) = self.names.iter().position(|n| *n == "agency") else {
            return Err("no agency kind to employ the state's staff".to_owned());
        };
        let mut family = self.job_family("LAB.employment", firm, household, o.today);
        let mut public = self.job_family("LAB.public_employment", agency, household, o.today);
        let jobs = self.drawn_jobs(o, &mut family)?;
        public.schedules.clone_from(&family.schedules);
        public.classes.clone_from(&family.classes);
        public.terms.clone_from(&family.terms);
        let firms = self.firm_hours(o, firm)?;
        let mut groups: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        for (i, j) in jobs.iter().enumerate() {
            groups.entry((j.region, j.occupation)).or_default().push(i);
        }
        let mut public_jobs = 0_u64;
        for ((region, occupation), mut members) in groups {
            let here = firms.get(&region).map_or(&[][..], Vec::as_slice);
            let o_at = usize::try_from(occupation).unwrap_or(usize::MAX);
            let weights: Vec<f64> = here.iter().map(|(_, h)| h.get(o_at).copied().unwrap_or(0.0)).collect();
            let Some(country) = o.countries.iter().find(|c| c.regions.iter().any(|(x, _)| *x == region)) else {
                return Err(format!("jobs in region {region}, in no country"));
            };
            let Some(Some(state)) = self.agencies.get(usize::from(country.id.get())).copied() else {
                return Err(format!("country {}: no agency to employ the state's staff", country.id.get()));
            };
            let shares = sys_soc::public_staff_shares(o.register, country.id)?;
            let Some(share) = shares.get(usize::try_from(occupation).unwrap_or(usize::MAX)).copied() else {
                return Err(format!("country {}: no public staff share for occupation {occupation}", country.id.get()));
            };
            let mut lot = o.streams.open(
                o.stream,
                opening_subject(
                    u32::from(country.id.get()) * PURPOSES + JOBS_PURPOSE,
                    region * if_labour::consts::OCCUPATIONS + occupation,
                ),
                Day::new(0),
                0,
            );
            // An order drawn by lot, so no household's place in the books decides its employer.
            for i in (1..members.len()).rev() {
                let j = phx_rand::float::index(phx_rand::below_u64(&mut lot, len_u64(i + 1)));
                members.swap(i, j);
            }
            let firms_take = weights.iter().any(|w| *w > 0.0);
            let n = len_u64(members.len());
            let public_n = if firms_take {
                u64::try_from(phx_ledger::opening::whole(share * phx_rand::float::from_u64(n))).unwrap_or(0)
            } else {
                n
            };
            let mut next = members.into_iter();
            for job in next.by_ref().take(usize::try_from(public_n).unwrap_or(usize::MAX)) {
                let Some(j) = jobs.get(job) else { continue };
                let _ = public.store.open(j.due(state), first_date(&public, o.calendar, j));
                public_jobs += 1;
            }
            let Some(rest) = n.checked_sub(public_n) else {
                violation!(clause = "SOC.2", "more public jobs than jobs", region = region);
            };
            let parts = deal(rest, &weights);
            for ((slot, _), n) in here.iter().zip(parts) {
                for job in next.by_ref().take(usize::try_from(n).unwrap_or(usize::MAX)) {
                    let Some(j) = jobs.get(job) else { continue };
                    let first = first_date(&family, o.calendar, j);
                    let _ = family.store.open(j.due(PartyKey::new(kind_number(firm), *slot)), first);
                }
            }
        }
        self.families.push(family);
        self.families.push(public);
        Ok(public_jobs)
    }
}

#[path = "core_jobs_tests.rs"]
mod tests;
