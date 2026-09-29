//! Labour on the core, a round a day. Each firm decides on its production schedule, from its price, its planned
//! output and its staff, the vacancies it posts and withdraws by the labour kind's rule; its hours a unit of each
//! occupation are its way's, fewer by its productivity, at its country's level in the occupation — the hours the
//! opening's staff in it give over the hours the ways ask of it at the opening's output — so the opening's staff is
//! what its output needs, occupation by occupation. Each searching person applies to the
//! vacancies in its reach; the next day each employer meets its applicants and offers its jobs; the day after each
//! person answers, and each hire is a contract from the firm to the household naming the person. Once a review
//! period each employer's pay round offers its contracts new wages, which its employees accept, counter, leave for
//! search, or take while applying on from their jobs; a hire of one who holds a job moves it job to job.

use std::collections::{BTreeMap, BTreeSet};

use if_labour::class;
use if_labour::decisions::{AcceptIn, AnswerIn, Need, PostIn, ReviewIn, SelectIn};
use if_labour::kind::LabourKind;
use if_labour::law::Law;
use phx_core::calendar::Calendar;
use phx_core::calendar::period::Period;
use phx_core::wheel::DueWheel;
use phx_core::{OpeningCountry, Register, Streams, SubStep};
use phx_id::{CountryId, Day, PartyKey, Slot};
use phx_macros::clause;
use phx_market::hiring::{Application, Seeker, Standing, Vacancy, answer, search, select};
use phx_num::{Missing, violation};
use phx_pop::person::{pack, unpack};
use phx_rand::float::{from_i64, from_u64, len_u64};
use phx_rand::{Draws, Subject, SubjectTag};

use crate::consts::firm::{OUTPUT, PRICE, PRODUCT, PRODUCTIVITY, PRODUCTIVITY_ONE, REGION};
use crate::consts::{CORE_WHEEL_DAYS, DAYS_A_WEEK, DAYS_A_YEAR, LEAST_MATCH_DAYS, PERCENT, WEEKS_A_YEAR};
use crate::core::{Core, kind_number};
use crate::core_day::Due;
use crate::opening::economy::table;

/// A posted vacancy's own record beside the kernel's: its identity, its point, when it was first posted and when its
/// point was last set, and its country.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Posting {
    pub id: u64,
    pub point: i64,
    pub first: Day,
    pub set: Day,
    pub country: u8,
}

/// What a day's round did.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LabourDay {
    pub day: u32,
    pub employers: u64,
    pub posted: u64,
    pub withdrawn: u64,
    /// The jobs laid off with notice, and the separations whose notice ran.
    pub layoffs_wanted: u64,
    pub separated: u64,
    pub searchers: u64,
    pub applications: u64,
    pub offers: u64,
    /// The offers accepted, and the contracts they made: an acceptance whose person left first makes none.
    pub acceptances: u64,
    pub hires: u64,
    /// The jobs open at the round's close, and the persons searching then.
    pub open: u64,
    pub searching: u64,
    /// The contracts whose pay round came, those raised and cut, the employees who applied on from their job at it,
    /// the hires of employees, who quit their job for the offer, and those who quit at their round for search.
    pub reviewed: u64,
    pub raised: u64,
    pub cut: u64,
    pub searching_on: u64,
    pub job_to_job: u64,
    pub quits: u64,
}

/// A contract's offer at its pay round, waiting for its employee's answer: the contract, its household and person, its
/// country, its point, the point offered and the most the job's month pays, the employee's reservation, and the
/// search it sees the vacancies by.
#[derive(Clone, Copy, Debug)]
pub struct Offered {
    pub family: usize,
    pub edge: Slot,
    pub country: u8,
    pub current: i64,
    pub offer: i64,
    pub revenue: i64,
    pub reservation: f64,
    pub seeker: Seeker,
}

/// A day's pay rounds: the contracts reviewed, those raised and cut, the employees whose counter the work could not
/// pay, who took the offer and applied to the vacancies they saw, and those offered less than they work for, who quit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reviews {
    pub reviewed: u64,
    pub raised: u64,
    pub cut: u64,
    pub searching_on: u64,
    pub quits: u64,
}

/// Labour's state on the core, kept from day to day.
#[derive(Debug, Default)]
pub struct CoreLabour {
    /// Each country's law, and its ways' hours of each occupation a unit of each product, by country index.
    pub laws: Vec<Law>,
    pub ways: Vec<Vec<Vec<f64>>>,
    /// Each country's level in each occupation: its opening staff's hours in it over the hours its ways ask of it at
    /// the opening's output; none where the ways ask none.
    pub level: Vec<Vec<f64>>,
    /// What financing an hour's wage until the output sells costs a year, by country: its lending rate.
    pub financing: Vec<f64>,
    pub vacancies: Vec<Vacancy>,
    pub postings: Vec<Posting>,
    pub next_vacancy: u64,
    pub applications: Vec<Application>,
    pub offers: Vec<Application>,
    /// The persons searching, by household and identity.
    pub searching: BTreeSet<(PartyKey, u64)>,
    pub employers: Option<DueWheel>,
    /// Each employer's last fill in an occupation: its point and the days it stood.
    pub fills: BTreeMap<(PartyKey, u32), (i64, u32)>,
    pub production_days: u32,
    /// The employment contracts under notice, and each separation: the contract, its person, the day its notice has
    /// run by, and its country.
    pub noticed: BTreeSet<u32>,
    /// The firms whose production schedule came today, whose price reviews follow.
    pub due_today: Vec<u32>,
    pub separations: Vec<(u32, u64, Day, u8)>,
    /// Each employer's next pay round.
    pub reviews: BTreeMap<PartyKey, Day>,
    /// Today's pay rounds, the applications employees made from their jobs at them, and the day's hires of
    /// employees.
    pub reviewing: Reviews,
    pub offered: Vec<Offered>,
    pub on_the_job: Vec<Application>,
    pub job_to_job: u64,
    pub days: Vec<LabourDay>,
}

/// What the labour round reads of the world besides the core.
pub struct LabourCtx<'a> {
    pub register: &'a Register,
    pub calendar: &'a Calendar,
    pub streams: &'a Streams,
    pub kind: &'a LabourKind,
    /// Each region's country, by region.
    pub regions: &'a [CountryId],
}

impl std::fmt::Debug for LabourCtx<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LabourCtx").field("regions", &self.regions.len()).finish_non_exhaustive()
    }
}

/// A job as its employer's staff lists it: its occupation, weekly hours and monthly wage.
type Job = (u32, u32, i64);

/// A firm's record words the round reads.
#[derive(Clone, Copy, Debug)]
struct Firm {
    product: usize,
    region: u32,
    productivity: f64,
    price: i64,
    output: f64,
}

/// A day `n` days after another.
fn after(day: Day, n: u64) -> Day {
    match u32::try_from(n).ok().and_then(|n| day.get().checked_add(n)) {
        Some(d) => Day::new(d),
        None => phx_num::capacity_exceeded!("day count", u32::MAX, n),
    }
}

fn at_country<T>(v: &[T], c: u8) -> &T {
    v.get(usize::from(c)).unwrap_or_else(|| violation!(clause = "LAB.16", "a country with no labour law", country = c))
}

impl LabourCtx<'_> {
    fn country_of(&self, region: u32) -> u8 {
        match self.regions.get(usize::try_from(region).unwrap_or(usize::MAX)) {
            Some(c) => c.get(),
            None => violation!(clause = "GEO.3", "a region in no country", region = region),
        }
    }

    fn draws(&self, name: &str, subject: Subject, day: Day) -> Draws {
        let Some(stream) = self.streams.named(name) else {
            violation!(clause = "CHN.1", "labour drawing from a stream never declared");
        };
        self.streams.open(&stream, subject, day, SubStep::S5c.ordinal())
    }

    fn wage_at(&self, law: &Law, point: i64) -> f64 {
        (self.kind.wage_at)(law, point)
    }
}

impl Core {
    /// The employment family's place among the core's families.
    fn employment(&self) -> Option<usize> {
        self.families.iter().position(|f| f.name == "LAB.employment")
    }

    fn firm_record(&self, firm: usize, slot: Slot) -> Option<Firm> {
        let rec = self.kinds.get(firm)?.record(slot);
        let read = |i: usize| match rec.get(i).map(|w| w.get()) {
            Some(Missing::Present(v)) => Some(v),
            _ => None,
        };
        Some(Firm {
            product: usize::try_from(read(PRODUCT)?).ok()?,
            region: u32::try_from(read(REGION)?).ok()?,
            productivity: from_i64(read(PRODUCTIVITY)?) / PRODUCTIVITY_ONE,
            price: read(PRICE)?,
            output: from_i64(read(OUTPUT)?),
        })
    }

    /// The sales a day a firm expects, as its record holds them.
    fn expected_of(&self, firm: usize, slot: Slot) -> Option<f64> {
        match self.kinds.get(firm)?.record(slot).get(crate::consts::firm::EXPECTED).map(|w| w.get()) {
            Some(Missing::Present(v)) => Some(from_i64(v) / crate::consts::firm::PART_ONE),
            _ => None,
        }
    }

    /// A firm's staff not under notice: each job's occupation and weekly hours, and its monthly wage.
    fn staff_of(&self, family: usize, firm: PartyKey) -> Vec<(u32, u32, i64)> {
        let Some(f) = self.families.get(family) else { return Vec::new() };
        f.store
            .of(0, firm.slot())
            .filter(|e| !self.labour.noticed.contains(&e.get()))
            .filter_map(|e| {
                let row = f.store.edges.row(e)?;
                let [occupation, hours, _] = *f.classes.get(usize::try_from(row.schedule).ok()?)?;
                Some((occupation, hours, row.amount))
            })
            .collect()
    }

    /// Labour opened on the core: each country's law and ways, its level from the opening's staff, what financing
    /// costs it, every searching person, and every firm's production schedule begun at a phase drawn for it.
    ///
    /// # Errors
    /// A primitive the round reads that the register does not hold, or a law that does not compile.
    #[clause("LAB.4", "LAB.16", "GEN.2")]
    pub fn open_labour(&mut self, ctx: &LabourCtx<'_>, countries: &[OpeningCountry], today: Day) -> Result<(), String> {
        let (Some(firm), Some(family)) = (self.names.iter().position(|n| *n == "firm"), self.employment()) else {
            return Ok(());
        };
        let mut labour = CoreLabour::default();
        let mut by_id: BTreeMap<u8, (Law, Vec<Vec<f64>>, f64)> = BTreeMap::new();
        for c in countries {
            let law = (ctx.kind.law)(ctx.register, c)?;
            let ways = table(ctx.register, "TEC.labour", c.id)?.0;
            let Some(rate) = c.derived("GEN.lending_rate") else {
                return Err(format!("country {}: no lending rate", c.id.get()));
            };
            by_id.insert(c.id.get(), (law, ways, rate / PERCENT));
        }
        for (_, (law, ways, rate)) in by_id {
            labour.laws.push(law);
            labour.ways.push(ways);
            labour.financing.push(rate);
        }
        let occupations = usize::try_from(class::NO_OCCUPATION).unwrap_or(0);
        let zeros = vec![vec![0.0; occupations]; labour.laws.len()];
        let (mut staff_hours, mut way_hours) = (zeros.clone(), zeros);
        let slots: Vec<Slot> = self.kinds.get(firm).map(|k| k.parties.live_slots().collect()).unwrap_or_default();
        for slot in &slots {
            let Some(f) = self.firm_record(firm, *slot) else { continue };
            let c = usize::from(ctx.country_of(f.region));
            let key = PartyKey::new(kind_number(firm), *slot);
            for (o, h, _) in self.staff_of(family, key) {
                if let Some(s) =
                    staff_hours.get_mut(c).and_then(|r| r.get_mut(usize::try_from(o).unwrap_or(usize::MAX)))
                {
                    *s += f64::from(h) * WEEKS_A_YEAR;
                }
            }
            let (Some(ways), Some(row)) = (labour.ways.get(c), way_hours.get_mut(c)) else { continue };
            for (w, occ) in row.iter_mut().zip(ways) {
                *w += f.output
                    * sys_frm::rules::way::own_hours(occ.get(f.product).copied().unwrap_or(0.0), f.productivity);
            }
        }
        labour.level = staff_hours
            .iter()
            .zip(&way_hours)
            .map(|(s, w)| s.iter().zip(w).map(|(s, w)| if *w > 0.0 { s / w } else { 0.0 }).collect())
            .collect();
        labour.production_days =
            u32::try_from(ctx.register.count("FRM.production_days")?).map_err(|e| e.to_string())?;
        let mut wheel = DueWheel::new(today.succ(), CORE_WHEEL_DAYS);
        for slot in slots {
            let Some(id) = self.kinds.get(firm).and_then(|k| k.parties.id(slot)) else { continue };
            let mut d = ctx.draws(ctx.kind.review_stream, Subject::new(SubjectTag::Party, id.get()), today);
            let phase = phx_rand::below_u64(&mut d, u64::from(labour.production_days));
            let first = after(today.succ(), phase);
            wheel.schedule(slot.get(), first);
        }
        labour.employers = Some(wheel);
        labour.searching = self.searching_persons(ctx);
        self.labour = labour;
        Ok(())
    }

    /// Every person whose labour state is searching.
    fn searching_persons(&self, ctx: &LabourCtx<'_>) -> BTreeSet<(PartyKey, u64)> {
        let mut out = BTreeSet::new();
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.as_ref())
        else {
            return out;
        };
        let (Some(store), Some(Some(persons))) = (self.kinds.get(place), self.persons.get(place)) else { return out };
        for slot in store.parties.live_slots() {
            for p in persons.of(slot) {
                if unpack(decl, p.word).attr(ctx.kind.state) == Some(class::SEARCHING) {
                    out.insert((PartyKey::new(kind_number(place), slot), p.id));
                }
            }
        }
        out
    }

    /// The day's round: yesterday's offers answered, yesterday's applications met and offered, the employers due
    /// today deciding, and today's searchers applying.
    #[clause("LAB.4", "LAB.5", "LAB.7", "LAB.8")]
    pub fn labour_day(&mut self, ctx: &LabourCtx<'_>, day: Day) -> LabourDay {
        let mut record = LabourDay { day: day.get(), ..LabourDay::default() };
        if self.labour.employers.is_none() {
            return record;
        }
        record.separated = self.separate(ctx, day);
        (record.acceptances, record.hires) = self.answer_offers(ctx, day);
        record.offers = self.select_applicants(ctx, day);
        let (posted, withdrawn, layoffs, employers) = self.post(ctx, day);
        self.answer_reviews(ctx, day);
        (record.posted, record.withdrawn, record.layoffs_wanted, record.employers) =
            (posted, withdrawn, layoffs, employers);
        (record.searchers, record.applications) = self.search_round(ctx, day);
        let r = std::mem::take(&mut self.labour.reviewing);
        (record.reviewed, record.raised, record.cut, record.searching_on) =
            (r.reviewed, r.raised, r.cut, r.searching_on);
        record.job_to_job = std::mem::take(&mut self.labour.job_to_job);
        record.quits = r.quits;
        self.keep_vacancies();
        record.open = self.labour.vacancies.iter().map(|v| u64::from(v.open)).sum();
        record.searching = len_u64(self.labour.searching.len());
        self.labour.days.push(record);
        record
    }

    /// The employers whose schedule came due decide and are booked again a production period on.
    fn post(&mut self, ctx: &LabourCtx<'_>, day: Day) -> (u64, u64, u64, u64) {
        let (Some(firm), Some(family)) = (self.names.iter().position(|n| *n == "firm"), self.employment()) else {
            return (0, 0, 0, 0);
        };
        let mut due = Vec::new();
        if let Some(w) = self.labour.employers.as_mut() {
            w.take(day, &mut due, None);
        }
        self.labour.due_today.clone_from(&due);
        let mut totals = (0, 0, 0, len_u64(due.len()));
        // Each employer's vacancies by their places, found in one pass, so a decision reads only its own.
        let mut mine: BTreeMap<PartyKey, Vec<usize>> = BTreeMap::new();
        for (i, v) in self.labour.vacancies.iter().enumerate() {
            mine.entry(v.employer).or_default().push(i);
        }
        for s in due {
            let slot = Slot::new(s);
            if self.kinds.get(firm).is_none_or(|k| k.parties.id(slot).is_none()) {
                continue;
            }
            let own = mine.remove(&PartyKey::new(kind_number(firm), slot)).unwrap_or_default();
            let (p, w, l) = self.post_one(ctx, day, (firm, family), (slot, &own));
            totals.0 += p;
            totals.1 += w;
            totals.2 += l;
            let next = after(day, u64::from(self.labour.production_days));
            if let Some(w) = self.labour.employers.as_mut() {
                w.schedule(s, next);
            }
        }
        totals
    }

    /// The point an employer offers in an occupation: by the labour kind's arithmetic, from its last fill there and
    /// the days it stood, its staff's wage there, and the mean wage; never below the law's least.
    #[clause("LAB.4", "LAB.11", "REP.34")]
    fn offer_point(
        &self,
        ctx: &LabourCtx<'_>,
        law: &Law,
        (employer, occupation): (PartyKey, u32),
        staff: &[(u32, u32, i64)],
    ) -> i64 {
        let fill = self.labour.fills.get(&(employer, occupation)).copied();
        let mine: Vec<i64> = staff.iter().filter(|(o, _, _)| *o == occupation).map(|(_, _, a)| *a).collect();
        let theirs = if mine.is_empty() {
            Missing::Absent
        } else {
            let mean = mine.iter().map(|a| from_i64(*a)).sum::<f64>() / from_u64(len_u64(mine.len()));
            (ctx.kind.point_near)(law, mean).map_or(Missing::Absent, Missing::Present)
        };
        let Some(mean) = (ctx.kind.point_near)(law, law.mean_monthly) else {
            violation!(clause = "REP.34", "a mean wage beyond the wage points");
        };
        let point = (ctx.kind.adapt)(fill.map_or(Missing::Absent, Missing::Present), theirs, mean, LEAST_MATCH_DAYS);
        match law.minimum_monthly {
            Missing::Present(m) => match (ctx.kind.least_point)(law, m) {
                Some(least) if point < least => least,
                Some(_) => point,
                None => violation!(clause = "LAB.11", "a minimum wage beyond the wage points"),
            },
            Missing::Absent => point,
        }
    }

    /// One employer's decision on its production schedule, applied: its stale vacancies raised a point, its needs in
    /// each occupation read, and the vacancies the rule posts and withdraws; the jobs it would lay off counted.
    #[clause("LAB.4", "LAB.11", "FRM.7")]
    fn post_one(
        &mut self,
        ctx: &LabourCtx<'_>,
        day: Day,
        (firm, family): (usize, usize),
        (slot, own): (Slot, &[usize]),
    ) -> (u64, u64, u64) {
        let Some(f) = self.firm_record(firm, slot) else { return (0, 0, 0) };
        let key = PartyKey::new(kind_number(firm), slot);
        let c = ctx.country_of(f.region);
        let law = at_country(&self.labour.laws, c).clone();
        self.raise_stale(ctx, day, own, &law);
        let staff = self.staff_of(family, key);
        let ways = at_country(&self.labour.ways, c);
        let level = at_country(&self.labour.level, c).clone();
        let lot = sys_frm::FilingPrims::lot(ctx.register, u16::try_from(f.product).unwrap_or(u16::MAX));
        let lead = ctx
            .register
            .table1("TEC.lead_time")
            .ok()
            .and_then(|t| t.at(i64::try_from(f.product).ok()?).ok())
            .map_or(0.0, from_i64);
        let financing = at_country(&self.labour.financing, c) * lead / DAYS_A_YEAR;
        let full = f64::from(law.full_time_hours);
        let job_hours = full / DAYS_A_WEEK;
        let mut needs = Vec::new();
        for occupation in 0..class::NO_OCCUPATION {
            let way = ways.get(usize::try_from(occupation).unwrap_or(usize::MAX)).and_then(|o| o.get(f.product));
            let at = level.get(usize::try_from(occupation).unwrap_or(usize::MAX)).copied().unwrap_or(0.0);
            let hours_a_unit = at * sys_frm::rules::way::own_hours(way.copied().unwrap_or(0.0), f.productivity);
            let held: f64 =
                staff.iter().filter(|(o, _, _)| *o == occupation).map(|(_, h, _)| f64::from(*h) / DAYS_A_WEEK).sum();
            if hours_a_unit <= 0.0 && held <= 0.0 {
                continue;
            }
            let open: f64 = own
                .iter()
                .filter_map(|i| self.labour.vacancies.get(*i))
                .filter(|v| v.occupation == occupation)
                .map(|v| f64::from(v.open) * job_hours)
                .sum();
            let point = self.offer_point(ctx, &law, (key, occupation), &staff);
            let wage_hour = ctx.wage_at(&law, point) / (law.weeks_a_month * full);
            needs.push(Need { occupation, hours_a_unit, staff_hours: held, open_hours: open, job_hours, wage_hour });
        }
        let minimum_hour = match law.minimum_monthly {
            Missing::Present(m) => m / (law.weeks_a_month * full),
            Missing::Absent => 0.0,
        };
        // What a unit leaves over its cost of making it at the price the firm expects.
        if let Some(cost) = self.unit_cost_of(ctx.regions, firm, slot) {
            let margin = self.price_expected(firm, slot, lot) - cost;
            self.review_wages(ctx, day, (family, key, &law), (margin, &needs, &staff, f.region));
        }
        let Some(units_a_day) = self.expected_of(firm, slot) else { return (0, 0, 0) };
        let input = PostIn { price: from_i64(f.price) / lot, units_a_day, financing, minimum_hour, needs };
        let out = (ctx.kind.post)(&input);
        let mut posted = 0;
        for &(occupation, open) in &out.post {
            let Some(skill) = usize::try_from(occupation).ok().and_then(|o| law.occupation_skill.get(o)).copied()
            else {
                continue;
            };
            if open == 0 {
                continue;
            }
            let point = self.offer_point(ctx, &law, (key, occupation), &staff);
            self.labour.vacancies.push(Vacancy {
                employer: key,
                region: f.region,
                occupation,
                skill,
                wage: ctx.wage_at(&law, point),
                open,
            });
            self.labour.postings.push(Posting {
                id: self.labour.next_vacancy,
                point,
                first: day,
                set: day,
                country: c,
            });
            self.labour.next_vacancy += 1;
            posted += u64::from(open);
        }
        let mut withdrawn = 0;
        for &(occupation, jobs) in &out.withdraw {
            let mut left = jobs;
            for i in own.iter().rev() {
                let Some(v) = self.labour.vacancies.get_mut(*i).filter(|v| v.occupation == occupation) else {
                    continue;
                };
                let take = if v.open < left { v.open } else { left };
                v.open -= take;
                left -= take;
                withdrawn += u64::from(take);
            }
        }
        let layoffs = self.lay_off(ctx, day, (family, key, c, &law), &out.layoff);
        (posted, withdrawn, layoffs)
    }

    /// The price a unit a firm expects its product to sell for: its stance's outlook of its product's mark in its
    /// region, or, before the mark has printed there, its own price.
    fn price_expected(&self, firm: usize, slot: Slot, lot: f64) -> f64 {
        use crate::consts::firm::{MEMORY, STANCE};
        let Some(store) = self.kinds.get(firm) else { return 0.0 };
        let rec = store.record(slot);
        let read = |i: usize| match rec.get(i).map(|w| w.get()) {
            Some(Missing::Present(v)) => usize::try_from(v).ok(),
            _ => None,
        };
        let (Some(product), Some(region), Some(memory), Some(stance), Some(price)) =
            (read(PRODUCT), read(REGION), read(MEMORY), read(STANCE), read(PRICE))
        else {
            return 0.0;
        };
        let series = (u16::try_from(product).unwrap_or(u16::MAX), u32::try_from(region).unwrap_or(u32::MAX));
        match self.goods.outlooks.outlook(series, memory, stance) {
            Missing::Present(mark) => mark / lot,
            Missing::Absent => from_u64(u64::try_from(price).unwrap_or(0)) / lot,
        }
    }

    /// Whether an employer's pay round is due today, its next set a review period on; an employer seen the first time
    /// draws the day in its first period its round falls on, so pay rounds are staggered.
    fn review_due(&mut self, ctx: &LabourCtx<'_>, day: Day, employer: PartyKey, law: &Law) -> bool {
        let Some(period) = u16::try_from(law.review_months).ok().and_then(Period::months) else {
            violation!(clause = "LAB.17", "a review period beyond a period", months = law.review_months);
        };
        match self.labour.reviews.get(&employer).copied() {
            Some(due) if due <= day => {
                let mut next = ctx.calendar.plus(due, period);
                while next <= day {
                    next = ctx.calendar.plus(next, period);
                }
                self.labour.reviews.insert(employer, next);
                true
            }
            Some(_) => false,
            None => {
                let end = ctx.calendar.plus(day, period);
                let Some(days) = ctx.calendar.days_between(day, end) else { return false };
                let mut d =
                    ctx.draws(ctx.kind.review_stream, Subject::new(SubjectTag::Party, u64::from(employer.word())), day);
                // The first round falls on one of the period's days after today, so a phase is never nought.
                let phase = phx_rand::below_u64(&mut d, u64::from(days)) + 1;
                self.labour.reviews.insert(employer, after(day, phase));
                false
            }
        }
    }

    /// A person's whole years on a day, where its household still holds it.
    fn age_of(&self, (household, person): (PartyKey, u64), date: phx_id::Date) -> Option<u32> {
        let place = self.names.iter().position(|n| *n == "household")?;
        let decl = self.household_decl.as_ref()?;
        let ps = self.persons.get(place)?.as_ref()?;
        let at = ps.place_of(household.slot(), person)?;
        let word = ps.of(household.slot()).nth(at)?.word;
        u32::try_from(unpack(decl, word).age_on(date)).ok()
    }

    /// An employer's pay round, when due: each of its contracts not under notice offered the lesser of the point
    /// nearest the most the job's month pays — its wage and what a unit leaves over its cost at the price the firm
    /// expects, for each unit a month of the job's hours makes, the way's other hours and inputs being its cost's
    /// already — and the point its fills show the market pays, never below the law's least; its employee answers with
    /// the day's others.
    #[clause("LAB.17", "LAB.11")]
    fn review_wages(
        &mut self,
        ctx: &LabourCtx<'_>,
        day: Day,
        (family, key, law): (usize, PartyKey, &Law),
        (margin, needs, staff, region): (f64, &[Need], &[Job], u32),
    ) {
        if staff.is_empty() || !self.review_due(ctx, day, key, law) {
            return;
        }
        let Some(f) = self.families.get(family) else { return };
        let contracts: Vec<(Slot, Due)> = f
            .store
            .of(0, key.slot())
            .filter(|e| !self.labour.noticed.contains(&e.get()))
            .filter_map(|e| Some((e, f.store.edges.row(e)?)))
            .collect();
        let full = f64::from(law.full_time_hours);
        for (edge, row) in contracts {
            let Some([occupation, hours, _]) =
                self.families.get(family).and_then(|f| f.classes.get(usize::try_from(row.schedule).ok()?)).copied()
            else {
                continue;
            };
            let Some(current) = (ctx.kind.point_near)(law, from_i64(row.amount)) else { continue };
            let Some(hours_a_unit) =
                needs.iter().find(|n| n.occupation == occupation).map(|n| n.hours_a_unit).filter(|h| *h > 0.0)
            else {
                continue;
            };
            let month = f64::from(hours) * law.weeks_a_month;
            // A job whose month pays nothing is the layoffs' to answer, not the pay round's.
            let Some(revenue) = (ctx.kind.point_near)(law, from_i64(row.amount) + margin * month / hours_a_unit) else {
                continue;
            };
            let least = match law.minimum_monthly {
                Missing::Present(m) => match (ctx.kind.least_point)(law, m * f64::from(hours) / full) {
                    Some(p) => Missing::Present(p),
                    None => violation!(clause = "LAB.11", "a minimum wage beyond the wage points"),
                },
                Missing::Absent => Missing::Absent,
            };
            let market = self.offer_point(ctx, law, (key, occupation), staff);
            let offer = (ctx.kind.review)(&ReviewIn { current, revenue, market, least });
            let reservation = law.reservation_share * ctx.wage_at(law, current);
            let Some(skill) = usize::try_from(occupation).ok().and_then(|o| law.occupation_skill.get(o)).copied()
            else {
                continue;
            };
            let Some(experience) = self.age_of((row.ends[1], row.person), ctx.calendar.date(day)) else { continue };
            // An employee moves only for more than its job pays.
            let seeker = Seeker {
                household: row.ends[1],
                person: row.person,
                subject: row.person,
                region,
                occupation,
                skill,
                experience,
                reservation: ctx.wage_at(law, current),
            };
            self.labour.offered.push(Offered {
                family,
                edge,
                country: ctx.country_of(region),
                current,
                offer,
                revenue,
                reservation,
                seeker,
            });
            self.labour.reviewing.reviewed += 1;
        }
    }

    /// The day's pay rounds answered: each employee offered sees the vacancies a week's search of its own draws, in
    /// one search of its country's, and answers from its reservation, the best of them and the prices it expects by
    /// the next round; the two conclude, or, where the work cannot pay its counter, it quits for search if the offer
    /// is below its reservation, and otherwise works on at the offer and applies to the vacancies it saw, quitting when
    /// one of them offers it the job.
    #[clause("LAB.17", "LAB.5", "LAB.6", "LAB.8")]
    fn answer_reviews(&mut self, ctx: &LabourCtx<'_>, day: Day) {
        let offered = std::mem::take(&mut self.labour.offered);
        if offered.is_empty() {
            return;
        }
        let standing = Standing::new(&self.labour.vacancies);
        let mut seen: BTreeMap<(PartyKey, u64), Vec<Application>> = BTreeMap::new();
        for (c, law) in self.labour.laws.iter().enumerate() {
            let mine: Vec<Seeker> = offered.iter().filter(|o| usize::from(o.country) == c).map(|o| o.seeker).collect();
            if mine.is_empty() {
                continue;
            }
            let draws = |subject: u64| ctx.draws(ctx.kind.taste_stream, Subject::new(SubjectTag::Party, subject), day);
            for a in search(
                None,
                (&self.labour.vacancies, &standing),
                &mine,
                (law.wage_weight, law.applications_a_week),
                &draws,
            ) {
                seen.entry((a.seeker.household, a.seeker.person)).or_default().push(a);
            }
        }
        for o in offered {
            let law = at_country(&self.labour.laws, o.country).clone();
            let apps = seen.remove(&(o.seeker.household, o.seeker.person)).unwrap_or_default();
            let best = apps
                .iter()
                .filter_map(|a| self.labour.vacancies.get(usize::try_from(a.vacancy).ok()?).map(|v| v.wage))
                .reduce(|a, b| if b > a { b } else { a })
                .map_or(Missing::Absent, Missing::Present);
            let input = AnswerIn {
                offer: o.offer,
                reservation: o.reservation,
                best,
                outlook: self.price_outlook(o.seeker.household, o.country, law.review_months),
                ratio: law.point_ratio,
            };
            let answer = (ctx.kind.answer.rule)(&input);
            let concluded = match (ctx.kind.conclude)(o.offer, answer, o.revenue) {
                Missing::Present(point) => point,
                // An offer below what it works for is one it leaves for search; above it, it works on at the offer
                // and applies to the vacancies it saw.
                Missing::Absent if ctx.wage_at(&law, o.offer) < o.reservation => {
                    let amount = self.families.get_mut(o.family).and_then(|f| {
                        let row = f.store.edges.row(o.edge).filter(|_| f.store.edges.is_open(o.edge))?;
                        f.store.close(o.edge);
                        Some(row.amount)
                    });
                    if let Some(amount) = amount {
                        self.searches_again(ctx, (o.seeker.household, o.seeker.person), amount, &law);
                        self.labour.reviewing.quits += 1;
                    }
                    continue;
                }
                Missing::Absent => {
                    self.labour.on_the_job.extend(apps);
                    self.labour.reviewing.searching_on += 1;
                    o.offer
                }
            };
            if concluded == o.current {
                continue;
            }
            let amount = phx_ledger::opening::whole(ctx.wage_at(&law, concluded));
            // A contract that ended since its offer, by its person's leaving, is not paid anew.
            let Some(r) = self.families.get_mut(o.family).and_then(|f| {
                let open = f.store.edges.is_open(o.edge);
                f.store.edges.rows_mut().get_mut(usize::try_from(o.edge.get()).unwrap_or(usize::MAX)).filter(|_| open)
            }) else {
                continue;
            };
            r.amount = amount;
            self.set_last_point(ctx, (o.seeker.household, o.seeker.person), concluded);
            if concluded > o.current {
                self.labour.reviewing.raised += 1;
            } else {
                self.labour.reviewing.cut += 1;
            }
        }
    }

    /// A person's last wage point, as its pay round set it.
    fn set_last_point(&mut self, ctx: &LabourCtx<'_>, (household, person): (PartyKey, u64), point: i64) {
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.clone())
        else {
            return;
        };
        let Some(Some(ps)) = self.persons.get_mut(place) else { return };
        let Some(at) = ps.place_of(household.slot(), person) else { return };
        let Some(word) = ps.of(household.slot()).nth(at).map(|x| x.word) else { return };
        let mut p = unpack(&decl, word);
        p.set_attr(ctx.kind.last_point, u32::try_from(point).unwrap_or(class::NO_POINT));
        ps.set_word(&mut self.space, household.slot(), at, pack(&decl, &p));
    }

    /// An employer's vacancies that stood past the law's patience raised a point.
    fn raise_stale(&mut self, ctx: &LabourCtx<'_>, day: Day, own: &[usize], law: &Law) {
        for i in own {
            let (Some(v), Some(p)) = (self.labour.vacancies.get_mut(*i), self.labour.postings.get_mut(*i)) else {
                continue;
            };
            if ctx.calendar.days_between(p.set, day).is_some_and(|d| d > law.patience_days) {
                p.point += 1;
                p.set = day;
                v.wage = ctx.wage_at(law, p.point);
            }
        }
    }

    /// Today's searchers, each with what the round reads of it: a searching person of a live household not waiting on
    /// an offer; one no longer searching or no longer held leaves the searchers.
    fn seekers(&mut self, ctx: &LabourCtx<'_>, day: Day) -> Vec<(u8, Seeker)> {
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.clone())
        else {
            return Vec::new();
        };
        let date = ctx.calendar.date(day);
        let waiting: BTreeSet<u64> = self.labour.offers.iter().map(|o| o.seeker.person).collect();
        let mut out = Vec::new();
        let mut gone = Vec::new();
        for &(household, person) in &self.labour.searching {
            let held = self.persons.get(place).and_then(Option::as_ref).and_then(|ps| {
                let at = ps.place_of(household.slot(), person)?;
                ps.of(household.slot()).nth(at)
            });
            let Some(h) =
                held.filter(|_| self.kinds.get(place).is_some_and(|k| k.parties.id(household.slot()).is_some()))
            else {
                gone.push((household, person));
                continue;
            };
            let p = unpack(&decl, h.word);
            if p.attr(ctx.kind.state) != Some(class::SEARCHING) {
                gone.push((household, person));
                continue;
            }
            if waiting.contains(&person) {
                continue;
            }
            let region = match decl.sited_by {
                Missing::Present(i) => {
                    self.kinds.get(place).and_then(|k| k.record(household.slot()).get(i).map(|w| w.get()))
                }
                Missing::Absent => None,
            };
            let Some(Missing::Present(region)) = region else { continue };
            let Ok(region) = u32::try_from(region) else { continue };
            let c = ctx.country_of(region);
            let law = at_country(&self.labour.laws, c);
            let skill =
                p.attr(ctx.kind.education).and_then(|e| law.education_skill.get(usize::try_from(e).ok()?)).copied();
            let (Some(skill), Some(occupation), Some(last)) =
                (skill, p.attr(ctx.kind.occupation), p.attr(ctx.kind.last_point))
            else {
                continue;
            };
            out.push((
                c,
                Seeker {
                    household,
                    person,
                    subject: person,
                    region,
                    occupation,
                    skill,
                    experience: u32::try_from(p.age_on(date)).unwrap_or(0),
                    reservation: law.reservation_share * ctx.wage_at(law, i64::from(last)),
                },
            ));
        }
        for g in gone {
            self.labour.searching.remove(&g);
        }
        out
    }

    /// Today's searchers apply, each country's by its law's rate and wage weight.
    #[clause("LAB.5", "LAB.8", "REP.22")]
    fn search_round(&mut self, ctx: &LabourCtx<'_>, day: Day) -> (u64, u64) {
        let seekers = self.seekers(ctx, day);
        let standing = Standing::new(&self.labour.vacancies);
        let mut sent = Vec::new();
        for (c, law) in self.labour.laws.iter().enumerate() {
            let mine: Vec<Seeker> = seekers.iter().filter(|(k, _)| usize::from(*k) == c).map(|(_, s)| *s).collect();
            if mine.is_empty() {
                continue;
            }
            let draws = |subject: u64| ctx.draws(ctx.kind.taste_stream, Subject::new(SubjectTag::Party, subject), day);
            sent.extend(search(
                None,
                (&self.labour.vacancies, &standing),
                &mine,
                (law.wage_weight, law.applications_a_week / DAYS_A_WEEK),
                &draws,
            ));
        }
        sent.append(&mut self.labour.on_the_job);
        let n = len_u64(sent.len());
        self.labour.applications = sent;
        (len_u64(seekers.len()), n)
    }

    /// Yesterday's applications met at the law's chance, each vacancy's met applicants handed to the labour kind's
    /// selection with their lots; returns the offers made.
    #[clause("LAB.7", "LAB.8")]
    fn select_applicants(&mut self, ctx: &LabourCtx<'_>, day: Day) -> u64 {
        let apps = std::mem::take(&mut self.labour.applications);
        let postings = self.labour.postings.clone();
        let laws = self.labour.laws.clone();
        let met = |a: &Application| {
            let Some(p) = postings.get(usize::try_from(a.vacancy).unwrap_or(usize::MAX)) else { return false };
            let mut d = ctx.draws(ctx.kind.meeting_stream, Subject::new(SubjectTag::Party, a.seeker.person), day);
            phx_rand::open_unit(&mut d) < at_country(&laws, p.country).seen_chance
        };
        let lots = |v: u32| {
            let id = postings.get(usize::try_from(v).unwrap_or(usize::MAX)).map_or(0, |p| p.id);
            ctx.draws(ctx.kind.lot_stream, Subject::new(SubjectTag::Market, id), day)
        };
        let choose = |applicants: &[phx_market::hiring::Applicant], open: u32| {
            let applicants = applicants
                .iter()
                .map(|a| if_labour::decisions::Applicant { skill: a.skill, experience: a.experience, lot: a.lot })
                .collect();
            (ctx.kind.select)(&SelectIn { applicants, open })
        };
        let offers = select(&mut self.labour.vacancies, &apps, met, (lots, choose));
        let n = len_u64(offers.len());
        self.labour.offers = offers;
        n
    }

    /// Yesterday's offers answered: each person takes the best-paid it accepts, by the labour kind's rule with its
    /// taste for the match; each hire is a contract from the firm to the household naming the person, at the
    /// vacancy's wage on its country's monthly dates, and the person no longer searches.
    #[clause("LAB.5", "LAB.8", "LAB.1")]
    fn answer_offers(&mut self, ctx: &LabourCtx<'_>, day: Day) -> (u64, u64) {
        let offers = std::mem::take(&mut self.labour.offers);
        let laws = self.labour.laws.clone();
        let postings = self.labour.postings.clone();
        let accepts = |o: &Application, v: &Vacancy| {
            let Some(p) = postings.get(usize::try_from(o.vacancy).unwrap_or(usize::MAX)) else { return false };
            let law = at_country(&laws, p.country);
            let mut d = ctx.draws(ctx.kind.taste_stream, Subject::new(SubjectTag::Party, o.seeker.person), day);
            let taste = phx_rand::gumbel(&mut d, 0.0, 1.0) - phx_rand::gumbel(&mut d, 0.0, 1.0);
            (ctx.kind.accept.rule)(&AcceptIn {
                wage: v.wage,
                reservation: o.seeker.reservation,
                taste,
                wage_weight: law.wage_weight,
            })
        };
        let hires = answer(&mut self.labour.vacancies, &offers, accepts);
        let accepted = len_u64(hires.len());
        let mut n = 0;
        for h in hires {
            if self.hire(ctx, day, &h) {
                n += 1;
            }
        }
        (accepted, n)
    }

    /// A hire made a contract, and its person's state, occupation and last point written.
    fn hire(&mut self, ctx: &LabourCtx<'_>, day: Day, hired: &Application) -> bool {
        let (Some(family), Some(place), Some(decl)) =
            (self.employment(), self.names.iter().position(|n| *n == "household"), self.household_decl.clone())
        else {
            return false;
        };
        let (Some(v), Some(p)) = (
            self.labour.vacancies.get(usize::try_from(hired.vacancy).unwrap_or(usize::MAX)).copied(),
            self.labour.postings.get(usize::try_from(hired.vacancy).unwrap_or(usize::MAX)).copied(),
        ) else {
            return false;
        };
        let household = hired.seeker.household;
        let Some(ps) = self.persons.get(place).and_then(Option::as_ref) else { return false };
        let Some(at) = ps.place_of(household.slot(), hired.seeker.person) else {
            // The person left before its answer took effect; the job returns to its vacancy.
            if let Some(v) = self.labour.vacancies.get_mut(usize::try_from(hired.vacancy).unwrap_or(usize::MAX)) {
                v.open += 1;
            }
            return false;
        };
        let law = at_country(&self.labour.laws, p.country).clone();
        let Some(word) = ps.of(household.slot()).nth(at).map(|x| x.word) else { return false };
        let Some(f) = self.families.get_mut(family) else { return false };
        let year = u32::try_from(ctx.calendar.date(day).year()).unwrap_or(0);
        let class = [v.occupation, law.full_time_hours, year];
        let found = f.schedules.iter().zip(&f.classes).position(|(sc, k)| sc.1 == p.country && *k == class);
        let schedule = if let Some(at_schedule) = found {
            at_schedule
        } else {
            let Some(same) = f.schedules.iter().find(|sc| sc.1 == p.country).copied() else { return false };
            f.schedules.push(same);
            f.classes.push(class);
            f.schedules.len() - 1
        };
        let Some((dates, _, _)) = f.schedules.get(schedule).copied() else { return false };
        let mut nth = 0_u32;
        while dates.nth(ctx.calendar, nth) <= day {
            nth += 1;
        }
        let due = Due {
            ends: [v.employer, household],
            amount: phx_ledger::opening::whole(v.wage),
            nth,
            schedule: u32::try_from(schedule)
                .unwrap_or_else(|_| violation!(clause = "TIME.4", "more schedules than a contract can name")),
            person: hired.seeker.person,
            arrears: 0,
        };
        // An employee hired quits the job it holds for this one.
        let held: Vec<Slot> = f
            .store
            .of(1, household.slot())
            .filter(|e| f.store.edges.row(*e).is_some_and(|r| r.person == hired.seeker.person))
            .collect();
        if !held.is_empty() {
            self.labour.job_to_job += 1;
        }
        for e in held {
            f.store.close(e);
            self.labour.noticed.remove(&e.get());
        }
        let _ = f.store.open(due, Some(dates.nth(ctx.calendar, nth)));
        let mut person = unpack(&decl, word);
        person.set_attr(ctx.kind.state, class::NOT_SEARCHING);
        person.set_attr(ctx.kind.occupation, v.occupation);
        person.set_attr(ctx.kind.last_point, u32::try_from(p.point).unwrap_or(class::NO_POINT));
        let packed = pack(&decl, &person);
        if let Some(Some(ps)) = self.persons.get_mut(place) {
            ps.set_word(&mut self.space, household.slot(), at, packed);
        }
        self.touched.insert(household.slot().get());
        self.labour.searching.remove(&(household, hired.seeker.person));
        self.end_benefit(household, hired.seeker.person);
        let stood = ctx.calendar.days_between(p.first, day).unwrap_or(0);
        self.labour.fills.insert((v.employer, v.occupation), (p.point, stood));
        true
    }

    /// Jobs of each occupation laid off with notice: the firm's jobs there not already under notice drawn by lot, each
    /// separation taking effect on the first business day the law's notice has run by.
    #[clause("LAB.4", "LAB.16")]
    fn lay_off(
        &mut self,
        ctx: &LabourCtx<'_>,
        day: Day,
        (family, firm, country, law): (usize, PartyKey, u8, &Law),
        layoffs: &[(u32, u32)],
    ) -> u64 {
        if layoffs.iter().all(|(_, n)| *n == 0) {
            return 0;
        }
        let Some(period) = u16::try_from(law.notice_days).ok().and_then(phx_core::calendar::period::Period::days)
        else {
            violation!(clause = "LAB.16", "a notice beyond a period", days = law.notice_days);
        };
        let effective = ctx.calendar.on_or_after(CountryId::new(country), ctx.calendar.plus(day, period));
        let mut d = ctx.draws(ctx.kind.layoff_stream, Subject::new(SubjectTag::Party, u64::from(firm.word())), day);
        let mut given = 0;
        for &(occupation, count) in layoffs {
            let Some(f) = self.families.get(family) else { break };
            let mut jobs: Vec<(u32, u64)> = f
                .store
                .of(0, firm.slot())
                .filter(|e| !self.labour.noticed.contains(&e.get()))
                .filter_map(|e| {
                    let row = f.store.edges.row(e)?;
                    let k = f.classes.get(usize::try_from(row.schedule).ok()?)?;
                    (k.first() == Some(&occupation)).then_some((e.get(), row.person))
                })
                .collect();
            for _ in 0..count {
                if jobs.is_empty() {
                    break;
                }
                let at = phx_rand::float::index(phx_rand::below_u64(&mut d, len_u64(jobs.len())));
                let (edge, person) = jobs.swap_remove(at);
                self.labour.noticed.insert(edge);
                self.labour.separations.push((edge, person, effective, country));
                given += 1;
            }
        }
        given
    }

    /// The separations whose notice has run: each job's contract closed, the severance the law owes for its years
    /// paid by the firm to the household, and its person searching again.
    #[clause("LAB.4", "LAB.5", "LAB.16")]
    fn separate(&mut self, ctx: &LabourCtx<'_>, day: Day) -> u64 {
        let Some(family) = self.employment() else { return 0 };
        let (due, rest): (Vec<_>, Vec<_>) =
            std::mem::take(&mut self.labour.separations).into_iter().partition(|(_, _, d, _)| *d <= day);
        self.labour.separations = rest;
        let year = ctx.calendar.date(day).year();
        let mut n = 0;
        for (edge, person, _, country) in due {
            self.labour.noticed.remove(&edge);
            let slot = Slot::new(edge);
            let Some(f) = self.families.get_mut(family) else { break };
            // A contract closed since its notice, by its person's leaving, is not separated again.
            let Some(row) = f.store.edges.row(slot).filter(|r| f.store.edges.is_open(slot) && r.person == person)
            else {
                continue;
            };
            let band =
                f.classes.get(usize::try_from(row.schedule).unwrap_or(usize::MAX)).and_then(|k| k.get(2)).copied();
            f.store.close(slot);
            let law = at_country(&self.labour.laws, country).clone();
            let years = band.map_or(0, |b| i64::from(year) - i64::from(b));
            let owed = phx_ledger::opening::whole((ctx.kind.owed)(
                &law,
                from_i64(row.amount),
                law.severance_days_a_year,
                years,
                1,
            ));
            if owed > 0 {
                self.pending.push(phx_core::flows::Flow {
                    payer: row.ends[0],
                    payee: row.ends[1],
                    amount: owed,
                    source: edge,
                    denomination: phx_core::flows::Denom::money(country),
                    reason: crate::core_day::SEVERANCE,
                    order: 0,
                });
            }
            self.searches_again(ctx, (row.ends[1], person), row.amount, &law);
            self.claim_benefit(ctx, day, (row.ends[1], person), (row.amount, country), &law);
            n += 1;
        }
        n
    }

    /// A person whose job ended searching again, its last point its job's.
    fn searches_again(&mut self, ctx: &LabourCtx<'_>, (household, person): (PartyKey, u64), amount: i64, law: &Law) {
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.clone())
        else {
            return;
        };
        let Some(Some(ps)) = self.persons.get_mut(place) else { return };
        let Some(at) = ps.place_of(household.slot(), person) else { return };
        let Some(word) = ps.of(household.slot()).nth(at).map(|x| x.word) else { return };
        let mut p = unpack(&decl, word);
        p.set_attr(ctx.kind.state, class::SEARCHING);
        if let Some(point) = (ctx.kind.point_near)(law, from_i64(amount)).and_then(|x| u32::try_from(x).ok()) {
            p.set_attr(ctx.kind.last_point, point);
        }
        ps.set_word(&mut self.space, household.slot(), at, pack(&decl, &p));
        self.touched.insert(household.slot().get());
        self.labour.searching.insert((household, person));
    }

    /// A person who lost its job claims its country's benefit where what it pays over its months is worth the hours
    /// claiming takes: a contract from the treasury paying the benefit's share of the wage it lost monthly from the
    /// next month for the benefit's months.
    #[clause("SOC.3", "SOC.7")]
    fn claim_benefit(
        &mut self,
        ctx: &LabourCtx<'_>,
        day: Day,
        (household, person): (PartyKey, u64),
        (wage, country): (i64, u8),
        law: &Law,
    ) {
        let (Some(Some(benefit)), Some(claim)) =
            (self.state.benefit.get(usize::from(country)).copied(), self.state.claim)
        else {
            return;
        };
        let Some(Some(treasury)) = self.treasuries.get(usize::from(country)).copied() else { return };
        let Some(family) = self.families.iter().position(|f| f.name == "SOC.benefit") else { return };
        let monthly = benefit.replacement * from_i64(wage);
        let hour = from_i64(wage) / (law.weeks_a_month * f64::from(law.full_time_hours));
        let input = if_state::kinds::ClaimIn {
            monthly,
            months: f64::from(benefit.months),
            claiming_cost: benefit.claim_hours * hour,
        };
        if !claim(&input) {
            return;
        }
        let date = ctx.calendar.date(day);
        let Some(f) = self.families.get_mut(family) else { return };
        let dates = phx_ledger::opening::monthly(date, CountryId::new(country));
        f.schedules.push((dates, country, 0));
        f.classes.push([0, 0, 0]);
        f.terms.push(None);
        f.ends_after.push(Some(benefit.months));
        let schedule = u32::try_from(f.schedules.len() - 1).unwrap_or(u32::MAX);
        let due = Due {
            ends: [treasury, household],
            amount: phx_ledger::opening::whole(monthly),
            nth: 1,
            schedule,
            person,
            arrears: 0,
        };
        let _ = f.store.open(due, Some(dates.nth(ctx.calendar, 1)));
    }

    /// A person hired leaves the benefit.
    fn end_benefit(&mut self, household: PartyKey, person: u64) {
        for family in self.families.iter_mut().filter(|f| f.name == "SOC.benefit") {
            let Some(side) = family.store.kinds.iter().position(|k| *k == household.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, household.slot()).collect();
            for e in mine {
                if family.store.edges.row(e).is_some_and(|r| r.person == person) {
                    family.store.close(e);
                }
            }
        }
    }

    /// A person who retired leaves its jobs and the searchers.
    #[clause("LAB.1", "LAB.6")]
    pub(crate) fn leave_jobs(&mut self, household: PartyKey, person: u64) {
        for family in self.families.iter_mut().filter(|f| f.name.starts_with("LAB.")) {
            let Some(side) = family.store.kinds.iter().position(|k| *k == household.kind()) else { continue };
            let mine: Vec<Slot> = family.store.of(side, household.slot()).collect();
            for e in mine {
                if family.store.edges.row(e).is_some_and(|r| r.person == person) {
                    family.store.close(e);
                }
            }
        }
        self.labour.searching.remove(&(household, person));
    }

    /// Vacancies with no job open and none held by an offer or an application let go, the rest's places renumbered.
    fn keep_vacancies(&mut self) {
        let held: BTreeSet<u32> =
            self.labour.offers.iter().chain(&self.labour.applications).map(|a| a.vacancy).collect();
        let mut map = vec![u32::MAX; self.labour.vacancies.len()];
        let mut kept_v = Vec::new();
        let mut kept_p = Vec::new();
        for (i, (v, p)) in self.labour.vacancies.iter().zip(&self.labour.postings).enumerate() {
            let at = u32::try_from(i).unwrap_or(u32::MAX);
            if v.open > 0 || held.contains(&at) {
                if let Some(m) = map.get_mut(i) {
                    *m = u32::try_from(kept_v.len()).unwrap_or(u32::MAX);
                }
                kept_v.push(*v);
                kept_p.push(*p);
            }
        }
        for a in self.labour.offers.iter_mut().chain(self.labour.applications.iter_mut()) {
            a.vacancy = map.get(usize::try_from(a.vacancy).unwrap_or(usize::MAX)).copied().unwrap_or(u32::MAX);
        }
        self.labour.vacancies = kept_v;
        self.labour.postings = kept_p;
    }
}
