//! Jobs on the core. Each job the opening drew for a household's person is dealt to one of the core's firms in
//! its region: a region's jobs of an occupation are shared over its firms by the hours their output takes of that
//! occupation, and dealt to them in an order drawn by lot. Each job is a contract from its firm to the household,
//! naming the person by its identity and paying the job's wage on its dates. Public administration's share of each
//! occupation's jobs — its hours of the occupation's employees' hours the country's output asks — and every job whose
//! occupation no firm's way in its region takes, is its country's public agency's. The employed are the persons drawn employed, so the staff is exactly the employed the population gives.
//! A wage is not drawn: an activity's compensation in the country's accounts is shared over the hours its jobs work,
//! each occupation's hour paid in proportion to its pay, so the wages are the accounts' and a firm's cost of a unit
//! is its way's at its productivity.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::{OpeningCountry, Register, StreamDecl, WorldStreams, opening_subject};
use phx_id::{Day, PartyKey, Slot};
use phx_macros::clause;
use phx_num::violation;
use phx_rand::float::{floor_to_i64, len_u64};

use crate::consts::firm::{COMPENSATION, JOBS_PURPOSE, PUBLIC_ADMINISTRATION, PURPOSES};
use crate::consts::{AGENT_ROWS_PER_CHUNK, MONTHS_A_YEAR, WEEKS_A_YEAR};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due, WAGE};
use crate::opening::asked::Asked;
use crate::opening::economy::table;
use phx_core::capacity::{AGENT_ROWS, WHEEL_DAYS};

/// A job as drawn: its household, its person, where its schedule is, its region, occupation, weekly hours and
/// country.
#[derive(Clone, Copy, Debug)]
struct Job {
    household: PartyKey,
    person: u64,
    schedule: u32,
    nth: u32,
    region: u32,
    occupation: u32,
    hours: u32,
    country: u8,
}

impl Job {
    /// The job as a contract from its employer to its household at a month's wage, naming its person.
    fn due(&self, employer: PartyKey, amount: i64) -> Due {
        Due {
            ends: [employer, self.household],
            amount,
            nth: self.nth,
            schedule: self.schedule,
            person: self.person,
            arrears: 0,
        }
    }
}

/// A job dealt to its employer: the job's place among those drawn, its employer and the activity it works in.
type Placed = (usize, PartyKey, usize);

/// Each activity's compensation a year over the hours its jobs work a year, each hour weighed by its occupation's pay:
/// an occupation's wage an hour there is this times its pay. None for an activity no job works in, or whose jobs' pay
/// weighs nothing.
#[clause("GEN.2", "GEN.4", "LAB.1")]
#[must_use]
pub fn activity_rates(compensation: &[f64], pay: &[f64], hours: &BTreeMap<(usize, u32), f64>) -> BTreeMap<usize, f64> {
    let mut weighed: BTreeMap<usize, f64> = BTreeMap::new();
    for ((a, o), h) in hours {
        if let Some(p) = usize::try_from(*o).ok().and_then(|o| pay.get(o)) {
            *weighed.entry(*a).or_insert(0.0) += h * p;
        }
    }
    weighed.into_iter().filter(|(_, w)| *w > 0.0).filter_map(|(a, w)| Some((a, compensation.get(a)? / w))).collect()
}

/// An occupation's wage an hour in an activity, from the activity's rate and the occupation's pay.
#[must_use]
pub fn wage_in(rates: &BTreeMap<usize, f64>, pay: &[f64], (activity, occupation): (usize, u32)) -> Option<f64> {
    Some(rates.get(&activity)? * pay.get(usize::try_from(occupation).ok()?)?)
}

/// Each occupation's wage an hour over every activity, weighed by the hours each works of it.
#[must_use]
pub fn occupation_wages(
    rates: &BTreeMap<usize, f64>,
    pay: &[f64],
    hours: &BTreeMap<(usize, u32), f64>,
) -> BTreeMap<u32, f64> {
    let mut sums: BTreeMap<u32, (f64, f64)> = BTreeMap::new();
    for (k, h) in hours {
        if let Some(r) = wage_in(rates, pay, *k) {
            let e = sums.entry(k.1).or_insert((0.0, 0.0));
            (e.0, e.1) = (e.0 + r * h, e.1 + h);
        }
    }
    sums.into_iter().filter(|(_, (_, h))| *h > 0.0).map(|(o, (paid, h))| (o, paid / h)).collect()
}

/// The wage point nearest a month's wage at an hourly rate and weekly hours.
pub(crate) fn month_point(law: &if_labour::law::Law, hourly: f64, hours: u32) -> Option<i64> {
    sys_lab::wages::point_near(law, hourly * f64::from(hours) * WEEKS_A_YEAR / MONTHS_A_YEAR)
}

/// Each firm's need of an occupation's hours net of its working owners': the hours its output takes, scaled so
/// that all the firms' take what their owners and the jobs to deal work, less its owners' own; none where its owners
/// meet its need.
#[must_use]
pub fn net_needs(asked: &[f64], owners: &[f64], jobs: f64) -> Vec<f64> {
    let whole: f64 = asked.iter().sum();
    if whole <= 0.0 {
        return vec![0.0; asked.len()];
    }
    let scale = (jobs + owners.iter().sum::<f64>()) / whole;
    asked
        .iter()
        .zip(owners)
        .map(|(a, o)| {
            let net = scale * a - o;
            if net > 0.0 { net } else { 0.0 }
        })
        .collect()
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
    pub streams: &'a WorldStreams,
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
            let [occupation, hours, _] = j.class;
            jobs.push(Job {
                household: j.household,
                person: j.person,
                schedule,
                nth: 1,
                region: j.region,
                occupation,
                hours,
                country: j.country,
            });
        }
        Ok(jobs)
    }

    /// Each firm's hours a year of each occupation its output takes: its output times its way's hours of the
    /// occupation a unit, fewer by its productivity's factor; by region, each firm with its slot.
    pub(crate) fn firm_hours(&self, o: &JobsOpening<'_>, firm: usize) -> Result<ByRegion, String> {
        let mut by_region: ByRegion = BTreeMap::new();
        let mut ways: BTreeMap<u8, Vec<Vec<f64>>> = BTreeMap::new();
        for c in o.countries {
            ways.insert(c.id.get(), table(o.register, "TEC.labour", c.id)?.0);
        }
        let country_of =
            |r: u32| o.countries.iter().find(|c| c.regions.iter().any(|(x, _)| *x == r)).map(|c| c.id.get());
        for slot in self.directory.live_slots(crate::core::kind_number(firm)) {
            let Some(v) = self.firm_view(slot) else {
                violation!(clause = "FRM.23", "a live firm with no rows", slot = slot.get());
            };
            let (Some(product), Some(region), Some(productivity), Some(output)) =
                (v.product(), v.region(), v.productivity(), v.output())
            else {
                violation!(clause = "FRM.23", "a firm missing a word of its own", slot = slot.get());
            };
            let Some(labour) = country_of(region).and_then(|c| ways.get(&c)) else {
                return Err(format!("a firm's region {region} in no country"));
            };
            let p = usize::from(product);
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
        let rows = [self.kind_rows(employer), self.kind_rows(household)];
        DatedFamily {
            name,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(employer), kind_number(household)], rows),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), WHEEL_DAYS),
            ),
            reason: WAGE,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            alike: BTreeMap::new(),
            alike_upto: 0,
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
        let (Some(firm), Some(household)) = (self.bound.kinds.firm, self.bound.kinds.household) else {
            return Ok(0);
        };
        let Some(agency) = self.bound.kinds.agency else {
            return Err("no agency kind to employ the state's staff".to_owned());
        };
        let mut family = self.job_family(crate::consts::families::EMPLOYMENT, firm, household, o.today);
        let mut public = self.job_family(crate::consts::families::PUBLIC_EMPLOYMENT, agency, household, o.today);
        let jobs = self.drawn_jobs(o, &mut family)?;
        public.schedules.clone_from(&family.schedules);
        public.classes.clone_from(&family.classes);
        public.terms.clone_from(&family.terms);
        let firms = self.firm_hours(o, firm)?;
        let mut groups: BTreeMap<(u32, u32), Vec<usize>> = BTreeMap::new();
        for (i, j) in jobs.iter().enumerate() {
            groups.entry((j.region, j.occupation)).or_default().push(i);
        }
        let mut public_share: BTreeMap<u8, Vec<Option<f64>>> = BTreeMap::new();
        for c in o.countries {
            public_share.insert(c.id.get(), Asked::of(o.register, c)?.employees_share(PUBLIC_ADMINISTRATION));
        }
        let mut public_jobs = 0_u64;
        let mut placed: Vec<Placed> = Vec::new();
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
            let Some(Some(share)) = public_share.get(&country.id.get()).and_then(|s| s.get(o_at)).copied() else {
                violation!(
                    clause = "GEN.2",
                    "jobs of an occupation no activity asks employees of",
                    occupation = occupation
                );
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
                placed.push((job, state, PUBLIC_ADMINISTRATION));
                public_jobs += 1;
            }
            let Some(rest) = n.checked_sub(public_n) else {
                violation!(clause = "SOC.2", "more public jobs than jobs", region = region);
            };
            let left: Vec<usize> = next.collect();
            let hours: f64 = left.iter().filter_map(|j| jobs.get(*j)).map(|j| f64::from(j.hours) * WEEKS_A_YEAR).sum();
            let owners: Vec<f64> = here
                .iter()
                .map(|(slot, _)| self.owner_hours(PartyKey::new(kind_number(firm), *slot), occupation))
                .collect();
            let parts = deal(rest, &net_needs(&weights, &owners, hours));
            let mut next = left.into_iter();
            for ((slot, _), n) in here.iter().zip(parts) {
                let Some(product) = self.firm_view(*slot).and_then(|v| v.product()).map(usize::from) else {
                    violation!(clause = "FRM.23", "a firm with no product", slot = slot.get());
                };
                let employer = PartyKey::new(kind_number(firm), *slot);
                for job in next.by_ref().take(usize::try_from(n).unwrap_or(usize::MAX)) {
                    placed.push((job, employer, product));
                }
            }
        }
        self.pay_jobs(o, &jobs, &placed, (&mut family, &mut public))?;
        self.add_family(family);
        self.add_family(public);
        Ok(public_jobs)
    }

    /// Each dealt job opened at its wage: each country's activities' compensation shared over their jobs' hours by
    /// their occupations' pay, a month's wage on the nearest wage point; its household's income a year raised by it,
    /// and its person's last wage point set to it. An adult with an occupation and no job — searching, or working in
    /// the firm it owns — is given its occupation's wage over every activity, full time, as its last point.
    #[clause("GEN.2", "GEN.4", "LAB.1", "REP.34")]
    fn pay_jobs(
        &mut self,
        o: &JobsOpening<'_>,
        jobs: &[Job],
        placed: &[Placed],
        (family, public): (&mut DatedFamily, &mut DatedFamily),
    ) -> Result<(), String> {
        let year_hours = |j: &Job| f64::from(j.hours) * WEEKS_A_YEAR;
        let mut by_occupation: BTreeMap<u8, BTreeMap<u32, f64>> = BTreeMap::new();
        for c in o.countries {
            let id = c.id.get();
            let mut hours: BTreeMap<(usize, u32), f64> = BTreeMap::new();
            for (job, _, activity) in placed {
                let Some(j) = jobs.get(*job).filter(|j| j.country == id) else { continue };
                *hours.entry((*activity, j.occupation)).or_insert(0.0) += year_hours(j);
            }
            let compensation: Vec<f64> = crate::opening::economy::accounts(o.register, c.id)?
                .0
                .added
                .iter()
                .map(|r| r.get(COMPENSATION).copied().unwrap_or(f64::NAN) * c.gdp)
                .collect();
            let pay = table(o.register, "GEN.occupation_pay", c.id)?.0.into_iter().next().unwrap_or_default();
            let rates = activity_rates(&compensation, &pay, &hours);
            by_occupation.insert(id, occupation_wages(&rates, &pay, &hours));
            for (a, r) in rates {
                self.drawn.rates.insert((id, a), r);
            }
            self.drawn.pay.insert(id, pay);
        }
        let laws: BTreeMap<u8, if_labour::law::Law> = o
            .countries
            .iter()
            .map(|c| sys_lab::law::law(o.register, c).map(|l| (c.id.get(), l)))
            .collect::<Result<_, _>>()?;
        let months: BTreeMap<u8, i64> = o
            .countries
            .iter()
            .map(|c| {
                let dates = phx_ledger::opening::monthly(o.calendar.date(o.today), c.id);
                (c.id.get(), crate::core_open::months_a_year(o.calendar, o.today, dates))
            })
            .collect();
        for (job, employer, activity) in placed {
            let Some(j) = jobs.get(*job) else { continue };
            let (Some(law), Some(m)) = (laws.get(&j.country), months.get(&j.country)) else { continue };
            let Some(hourly) = self.drawn.wage_in((j.country, *activity), j.occupation) else {
                return Err(format!("country {}: activity {activity}'s jobs with no compensation", j.country));
            };
            let Some(point) = month_point(law, hourly, j.hours) else {
                violation!(clause = "REP.34", "a wage beyond the wage points", activity = *activity);
            };
            let amount = phx_ledger::opening::whole(sys_lab::wages::wage_at(law, point));
            let book = if *activity == PUBLIC_ADMINISTRATION { &mut *public } else { &mut *family };
            let first = first_date(book, o.calendar, j);
            let _ = book.store.open(j.due(*employer, amount), first);
            self.add_income(j.household, amount * m);
            self.put_last_point((j.household, j.person), point);
        }
        let idle = std::mem::take(&mut self.drawn.idle);
        for i in &idle {
            let (Some(of_occupation), Some(law)) = (by_occupation.get(&i.country), laws.get(&i.country)) else {
                continue;
            };
            let Some(hourly) = of_occupation.get(&i.occupation).copied() else { continue };
            if let Some(point) = month_point(law, hourly, law.full_time_hours) {
                self.put_last_point((i.household, i.person), point);
            }
        }
        Ok(())
    }

    /// A household's outlook of its income a year; none where it holds none.
    pub(crate) fn income_of(&self, household: PartyKey) -> Option<i64> {
        self.household_of(household)?.income()
    }

    /// A household's income a year raised by an amount.
    fn add_income(&mut self, household: PartyKey, amount: i64) {
        let Some(held) = self.income_of(household) else {
            violation!(clause = "GEN.2", "a household with no income to add a wage to", slot = household.slot().get());
        };
        self.household_write(|hs| hs.set_income(household.slot(), held + amount));
    }

    /// A person's last wage point written to it.
    pub(crate) fn put_last_point(&mut self, (household, person): (PartyKey, u64), point: i64) {
        let Some(decl) = self.declared.household.clone() else { return };
        let Some(Some(ps)) = self.persons.get_mut(usize::from(household.kind())) else { return };
        let Some(at) = ps.place_of(household.slot(), person) else { return };
        let Some(word) = ps.of(household.slot()).nth(at).map(|x| x.word) else { return };
        let mut p = phx_pop::person::unpack(&decl, word);
        p.set_attr(sys_lab::LAST_POINT.name, u32::try_from(point).unwrap_or(if_labour::class::NO_POINT));
        ps.set_word(&mut self.space, household.slot(), at, phx_pop::person::pack(&decl, &p));
    }
}

#[path = "core_jobs_tests.rs"]
mod tests;
