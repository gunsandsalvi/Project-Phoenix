//! Jobs on the core. Each job the books' opening drew for a household's person is dealt to one of the core's firms in
//! its region: a region's jobs of an occupation are shared over its firms by the hours their output takes of that
//! occupation, and dealt to them in an order drawn by lot. Each job is a contract from its firm to the household,
//! naming the person by its identity and paying the job's wage on its dates. A job whose occupation no firm's way in
//! its region takes — the armed forces' — is the state's, a contract from its country's treasury until the public
//! agencies take their staff. The employed are the persons the books drew employed, so the staff is exactly the
//! employed the population gives.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::{OpeningCountry, Register, StreamDecl, Streams, opening_subject};
use phx_id::{Day, LineId, PartyKey, Slot};
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::violation;
use phx_pop::population::Population;
use phx_rand::float::{floor_to_i64, from_i64, len_u64};
use phx_store::SystemBacking;

use crate::consts::firm::{JOBS_PURPOSE, OUTPUT, PRODUCT, PRODUCTIVITY, PRODUCTIVITY_ONE, PURPOSES, REGION};
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, CORE_WHEEL_DAYS};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due, WAGE};
use crate::opening::economy::table;

/// A job read from the books: its household, its person, its wage, where its schedule is, its region and occupation.
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
    pub books: &'a Books,
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
    /// The jobs the books hold on employment lines, each read once a line: its wage, its schedule and next date, its
    /// region and occupation.
    fn read_jobs(&self, o: &JobsOpening<'_>, family: &mut DatedFamily, household: usize) -> Vec<Job> {
        let Some(pop_at) = household.checked_sub(usize::from(self.first_agents)) else { return Vec::new() };
        let ledger = &o.books.ledger;
        let table = Population::table::<SystemBacking>(o.books.parties.cells(), pop_at);
        let mut lines: BTreeMap<LineId, (i64, u32, u32, u32, u32)> = BTreeMap::new();
        let mut jobs = Vec::new();
        for slot in table.slots() {
            let Some(key) = self.key(table.party(slot)) else { continue };
            for word in table.attachments(slot) {
                let a = phx_pop::person::Attachment::unpack(*word);
                if ledger.lines.kind_name(a.line) != if_labour::consts::EMPLOYMENT_LINE {
                    continue;
                }
                let read = *lines.entry(a.line).or_insert_with(|| {
                    let terms = ledger.terms.get(ledger.lines.terms(a.line));
                    let Some(amount) = terms.legs.iter().find_map(|l| match l {
                        phx_ledger::algebra::Leg::FixedAmount(m) => Some(m.amt()),
                        _ => None,
                    }) else {
                        violation!(clause = "LAB.1", "a job with no wage", line = a.line.get());
                    };
                    let (Some(occupation), Some(region)) = (
                        terms.class.get(if_labour::consts::OCCUPATION).copied(),
                        terms.class.get(if_labour::consts::REGION).copied(),
                    ) else {
                        violation!(clause = "LAB.1", "a job with no occupation or region", line = a.line.get());
                    };
                    let (dates, next) = (terms.schedule.dates, ledger.lines.next_due(a.line));
                    let mut nth = 0_u32;
                    while dates.nth(o.calendar, nth) < next {
                        nth += 1;
                    }
                    let schedule = u32::try_from(family.schedules.len())
                        .unwrap_or_else(|_| violation!(clause = "TIME.4", "more schedules than a contract can name"));
                    family.schedules.push((dates, terms.ccy.index(), terms.payment_order.0));
                    (amount, schedule, nth, occupation, region)
                });
                let (amount, schedule, nth, occupation, region) = read;
                let phx_pop::person::Holder::Person(place) = a.holder else {
                    violation!(clause = "LAB.1", "a job held by a household, not a person", line = a.line.get());
                };
                let Some(person) = self.person_at(key, place) else {
                    violation!(clause = "REP.26", "a job held by a person its household does not hold");
                };
                jobs.push(Job { household: key, person, amount, schedule, nth, region, occupation });
            }
        }
        jobs
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
        }
    }

    /// The jobs on the core: each region's jobs of an occupation dealt over its firms by the hours their output takes
    /// of it, in an order drawn by lot, each a contract from its firm to the household naming the person; those no
    /// firm's way in their region takes, from their country's treasury. Returns the jobs the state holds.
    ///
    /// # Errors
    /// A primitive the dealing reads that the register does not hold, or a country with no treasury to employ.
    #[clause("LAB.1", "REP.40", "GEN.2", "PTY.3", "SOC.2")]
    pub fn open_jobs(&mut self, o: &JobsOpening<'_>) -> Result<u64, String> {
        let (Some(firm), Some(household)) =
            (self.names.iter().position(|n| *n == "firm"), self.names.iter().position(|n| *n == "household"))
        else {
            return Ok(0);
        };
        let Some(treasury) = self.names.iter().position(|n| *n == "treasury") else {
            return Err("no treasury kind to employ the state's staff".to_owned());
        };
        let mut family = self.job_family("LAB.employment", firm, household, o.today);
        let mut public = self.job_family("LAB.public_employment", treasury, household, o.today);
        let jobs = self.read_jobs(o, &mut family, household);
        public.schedules.clone_from(&family.schedules);
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
            let parts = deal(len_u64(members.len()), &weights);
            let Some(country) = o.countries.iter().find(|c| c.regions.iter().any(|(x, _)| *x == region)) else {
                return Err(format!("jobs in region {region}, in no country"));
            };
            if parts.iter().sum::<u64>() == 0 {
                let Some(Some(state)) = self.treasuries.get(usize::from(country.id.get())).copied() else {
                    return Err(format!("country {}: no treasury to employ the state's staff", country.id.get()));
                };
                for job in members {
                    let Some(j) = jobs.get(job) else { continue };
                    let _ = public.store.open(j.due(state), first_date(&public, o.calendar, j));
                    public_jobs += 1;
                }
                continue;
            }
            let country = u32::from(country.id.get());
            let mut lot = o.streams.open(
                o.stream,
                opening_subject(
                    country * PURPOSES + JOBS_PURPOSE,
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
            let mut next = members.into_iter();
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
