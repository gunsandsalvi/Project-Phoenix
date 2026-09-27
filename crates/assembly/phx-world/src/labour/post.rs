//! The employers' decisions on their production schedule: in each occupation its work needs, the vacancies to post,
//! the offers to raise and the jobs to lay off, as the labour kind's rule decides from the employer's price, planned
//! output, staff and vacancies.

use if_labour::class;
use if_labour::decisions::{Need, PostIn, PostOut};
use if_labour::law::Law;
use phx_core::FactStore;
use phx_id::{CountryId, Day, LineId, PartyId, Slot};
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use super::book::{Separation, Vacancy};
use crate::goods::Rows;
use crate::world::World;

/// An employer's staff on one employment line: the line, its occupation, its weekly hours, its band and its members.
#[derive(Clone, Copy, Debug)]
pub(super) struct Staff {
    pub line: LineId,
    pub occupation: u32,
    pub hours: u32,
    pub band: u32,
    pub members: u32,
}

/// What an employer's decision reads of its own facts.
#[derive(Clone, Copy, Debug)]
struct Firm {
    product: i64,
    price: f64,
    units_a_day: f64,
    hurdle: f64,
    hours_a_unit: f64,
}

/// The fact names an employer's decision reads.
const PRODUCT: &str = "FRM.product";
const PRICE: &str = "FRM.price";
const OUTPUT: &str = "FRM.output_rate";
const HURDLE: &str = "FRM.required_return";
const FIRM_HOURS: &str = "FRM.hours_a_unit";
/// The technology an employer's work is read from: hours by occupation a unit, and the days a unit takes.
const HOURS_A_UNIT: &str = "TEC.labour";
const LEAD_TIME: &str = "TEC.lead_time";

impl World {
    /// An employer's rows a visit saw, kept for the day's round when the visit is one on which employers decide.
    pub(crate) fn labour_after(&mut self, handler: &str, rows: Rows, slots: &[Slot]) {
        let Some(kind) = self.labour.kind else { return };
        if kind.employer_visits.contains(&handler) {
            self.labour.employers_due.extend(slots.iter().map(|s| (rows, *s)));
        }
    }

    /// The employers whose schedule came due today decide, each on the staff it keeps once the notices it gave run.
    pub(super) fn post(&mut self, day: Day) {
        let mut noticed: std::collections::BTreeMap<(PartyId, LineId), u32> = std::collections::BTreeMap::new();
        for s in &self.labour.book.separations {
            *noticed.entry((s.employer, s.line)).or_insert(0) += s.count;
        }
        for (rows, slot) in std::mem::take(&mut self.labour.employers_due) {
            self.post_one(day, rows, slot, &noticed);
        }
    }

    /// An employer's facts its decision reads, none when any is missing: it decides nothing before its opening
    /// accounts give it a price and a planned output.
    fn firm_facts(&mut self, rows: Rows, slot: Slot) -> Option<Firm> {
        let first = self.books.parties.first_cell_place();
        let store: &mut dyn FactStore = if rows.individuals {
            self.books.parties.table_mut(rows.place)
        } else {
            let k = usize::from(rows.place.checked_sub(first)?);
            Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, k)
        };
        let mut read = |f| match store.read(f, slot) {
            Missing::Present(v) => Some(v),
            Missing::Absent => None,
        };
        let (product, price, output, hurdle) = (read(PRODUCT)?, read(PRICE)?, read(OUTPUT)?, read(HURDLE)?);
        let hours = read(FIRM_HOURS)?;
        let scale = |exp: u8| (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
        let lot = phx_rand::float::from_i64(self.goods_frame.base(u16::try_from(product).ok()?));
        // A posted price is for a lot of the product; the work is weighed by what a unit fetches.
        Some(Firm {
            product,
            price: phx_rand::float::from_i64(price) / lot,
            units_a_day: phx_rand::float::from_i64(output),
            hurdle: phx_rand::float::from_i64(hurdle) / scale(crate::consts::HURDLE_EXP),
            hours_a_unit: phx_rand::float::from_i64(hours) / scale(crate::consts::HOURS_EXP),
        })
    }

    /// An employer's staff on employment lines, a twin's members each, less those under notice.
    fn staff(
        &self,
        party: PartyId,
        twins: u32,
        noticed: &std::collections::BTreeMap<(PartyId, LineId), u32>,
    ) -> Vec<Staff> {
        let Some(kind) = self.labour.kind else { return Vec::new() };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let (place, slot) = self.books.parties.row(party);
        phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Liability && self.books.ledger.lines.kind_of(r.row.line) == k)
            .filter_map(|r| {
                let terms = self.books.ledger.terms.get(self.books.ledger.lines.terms(r.row.line));
                let at = |i| terms.class.get(i).copied();
                Some(Staff {
                    line: r.row.line,
                    occupation: at(class::OCCUPATION)?,
                    hours: at(class::HOURS)?,
                    band: at(class::BAND)?,
                    // Notices beyond the row's members leave it none: those gone since left before theirs ran.
                    members: noticed
                        .get(&(party, r.row.line))
                        .map_or(Some(r.row.count), |n| r.row.count.checked_sub(*n))
                        .map_or(0, |kept| kept / twins),
                })
            })
            .collect()
    }

    /// The point an employer offers in an occupation: its last fill's, a point lower when that fill came in the
    /// least a match takes; else its newest staff's there; else the mean wage for the hours; never below the law's
    /// least.
    #[clause("LAB.4", "LAB.11", "REP.34")]
    pub(super) fn offer_point(
        &self,
        law: &Law,
        (employer, occupation): (PartyId, u32),
        staff: &[Staff],
        hours: u32,
    ) -> i64 {
        let Some(kind) = self.labour.kind else {
            violation!(clause = "REP.34", "a wage offered in a world with no labour");
        };
        let fill = self.labour.book.fills.iter().find(|f| f.employer == employer && f.occupation == occupation);
        let newest =
            staff.iter().filter(|s| s.occupation == occupation).reduce(|a, b| if b.band > a.band { b } else { a });
        let share = f64::from(hours) / f64::from(law.full_time_hours);
        let Some(mean) = (kind.point_near)(law, law.mean_monthly * share) else {
            violation!(clause = "REP.34", "a mean wage beyond the wage points");
        };
        let point = (kind.adapt)(
            fill.map_or(Missing::Absent, |f| Missing::Present((f.point, f.days))),
            newest.map_or(Missing::Absent, |s| Missing::Present(i64::from(self.line_point(s.line)))),
            mean,
            crate::consts::LEAST_MATCH_DAYS,
        );
        match law.minimum_monthly {
            Missing::Present(m) => match self.labour.kind.and_then(|k| (k.least_point)(law, m * share)) {
                Some(least) if point < least => least,
                Some(_) => point,
                None => violation!(clause = "LAB.11", "a minimum wage beyond the wage points"),
            },
            Missing::Absent => point,
        }
    }

    /// One employer's decision on its production schedule, applied.
    #[clause("LAB.4", "LAB.11", "FRM.7")]
    fn post_one(
        &mut self,
        day: Day,
        rows: Rows,
        slot: Slot,
        noticed: &std::collections::BTreeMap<(PartyId, LineId), u32>,
    ) {
        let Some(kind) = self.labour.kind else { return };
        let Some(row) = self.goods_row(rows, slot) else { return };
        let Some(firm) = self.firm_facts(rows, slot) else { return };
        let (Missing::Present(country), Missing::Present(region)) =
            (self.geo().zone_country(row.zone), self.geo().zone_region(row.zone))
        else {
            return;
        };
        let law = super::law_of(&self.labour.laws, country).clone();
        let Ok(twins) = u32::try_from(row.twins) else { return };
        let staff = self.staff(row.party, twins, noticed);
        self.raise_stale(day, row.party, &law);
        let Ok(hours_a_unit) = self.register.table2_in(HOURS_A_UNIT, country).cloned() else { return };
        let lead = self.register.table1(LEAD_TIME).ok().and_then(|t| t.at(firm.product).ok());
        let Some(lead) = lead else { return };
        let financing = firm.hurdle * phx_rand::float::from_i64(lead) / crate::consts::DAYS_A_YEAR;
        let job_hours = f64::from(law.full_time_hours) / crate::consts::DAYS_A_WEEK;
        let Ok(phx_core::ValueType::Table2 { exp, .. }) = self.register.decl_by_id(HOURS_A_UNIT).map(|d| d.value)
        else {
            return;
        };
        let per_unit = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
        // The way gives the occupations' mix; the firm's own hours a unit, its productivity, give their sum.
        let way_hours =
            |o: u32| hours_a_unit.at(i64::from(o), firm.product).ok().map(|h| phx_rand::float::from_i64(h) / per_unit);
        let way_total: f64 = (0..class::NO_OCCUPATION).filter_map(way_hours).sum();
        if way_total <= 0.0 {
            return;
        }
        let mut needs = Vec::new();
        for occupation in 0..class::NO_OCCUPATION {
            let Some(h) = way_hours(occupation) else { continue };
            let hours_a_unit = firm.hours_a_unit * h / way_total;
            let held: f64 = staff
                .iter()
                .filter(|s| s.occupation == occupation)
                .map(|s| f64::from(s.members) * f64::from(s.hours) / crate::consts::DAYS_A_WEEK)
                .sum();
            if hours_a_unit <= 0.0 && held <= 0.0 {
                continue;
            }
            let book = &self.labour.book;
            let open: f64 = book
                .of_employer(row.party)
                .iter()
                .filter_map(|id| book.vacancy(*id))
                .filter(|v| v.occupation == occupation)
                .map(|v| f64::from(v.open / twins) * f64::from(v.hours) / crate::consts::DAYS_A_WEEK)
                .sum();
            let point = self.offer_point(&law, (row.party, occupation), &staff, law.full_time_hours);
            let wage_hour = self.wage_at(&law, point) / (law.weeks_a_month * f64::from(law.full_time_hours));
            needs.push(Need { occupation, hours_a_unit, staff_hours: held, open_hours: open, job_hours, wage_hour });
        }
        let minimum_hour = match law.minimum_monthly {
            Missing::Present(m) => m / (law.weeks_a_month * f64::from(law.full_time_hours)),
            Missing::Absent => 0.0,
        };
        self.review(day, (row.party, country, twins), (&law, firm.price), &staff, &needs);
        let input = PostIn { price: firm.price, units_a_day: firm.units_a_day, financing, minimum_hour, needs };
        let out = (kind.post)(&input);
        self.apply_post(day, (row.party, country, region, twins), &law, &staff, &out);
    }

    /// The vacancies of an employer that stood past its patience raised a point.
    fn raise_stale(&mut self, day: Day, employer: PartyId, law: &Law) {
        let ids = self.labour.book.of_employer(employer).to_vec();
        for id in ids {
            let Some(set) = self.labour.book.vacancy(id).map(|v| v.set) else { continue };
            if self.calendar.days_between(set, day).is_some_and(|d| d > law.patience_days)
                && let Some(v) = self.labour.book.vacancy_mut(id)
            {
                v.point += 1;
                v.set = day;
            }
        }
    }

    /// An employer's decision applied: its vacancies posted and withdrawn, and its layoffs given notice.
    fn apply_post(
        &mut self,
        day: Day,
        (employer, country, region, twins): (PartyId, CountryId, u32, u32),
        law: &Law,
        staff: &[Staff],
        out: &PostOut,
    ) {
        let Some(kind) = self.labour.kind else { return };
        let Some(stream) = self.streams.named(kind.vacancy_stream) else { return };
        let mut d = self.streams.open(
            &stream,
            Subject::new(SubjectTag::Party, employer.get()),
            day,
            phx_core::SubStep::S5c.ordinal(),
        );
        // An employer of one twin posts and withdraws jobs in the whole agents that fill them, the searchers' unit;
        // an agent's own twins make its jobs whole already.
        let unit = self.population.representation.multiplicity;
        let mut in_agents =
            |jobs: u32| if twins > 1 { jobs * twins } else { whole_agents(jobs, (unit, u32::MAX), &mut d) };
        for &(occupation, jobs) in &out.post {
            let open = in_agents(jobs);
            if open == 0 {
                continue;
            }
            let point = self.offer_point(law, (employer, occupation), staff, law.full_time_hours);
            let skill = usize::try_from(occupation).ok().and_then(|o| law.occupation_skill.get(o)).copied();
            let Some(skill) = skill else { continue };
            let id = self.labour.book.next;
            self.labour.book.next += 1;
            self.labour.book.post(Vacancy {
                id,
                employer,
                country: country.get(),
                region,
                occupation,
                skill,
                hours: law.full_time_hours,
                point,
                open,
                first: day,
                set: day,
            });
            self.labour.day.posted += u64::from(open);
        }
        for &(occupation, jobs) in &out.withdraw {
            let mut left = in_agents(jobs);
            let ids = self.labour.book.of_employer(employer).to_vec();
            for id in ids.into_iter().rev() {
                let Some(v) = self.labour.book.vacancy_mut(id).filter(|v| v.occupation == occupation) else { continue };
                let take = if v.open < left { v.open } else { left };
                v.open -= take;
                left -= take;
            }
        }
        for &(occupation, jobs) in &out.layoff {
            self.lay_off(day, (employer, country), staff, (occupation, jobs * twins), twins);
        }
    }

    /// Jobs of an occupation laid off with notice: the lines they leave drawn by their members, each separation taking
    /// effect on the first business day the notice has run by.
    fn lay_off(
        &mut self,
        day: Day,
        (employer, country): (PartyId, CountryId),
        staff: &[Staff],
        (occupation, count): (u32, u32),
        twins: u32,
    ) {
        let Some(kind) = self.labour.kind else { return };
        let law = super::law_of(&self.labour.laws, country);
        let Some(period) = u16::try_from(law.notice_days).ok().and_then(phx_core::calendar::period::Period::days)
        else {
            violation!(clause = "LAB.16", "a notice beyond a period", days = law.notice_days);
        };
        let effective = self.calendar.on_or_after(country, self.calendar.plus(day, period));
        let mut lines: Vec<(LineId, u32)> =
            staff.iter().filter(|s| s.occupation == occupation).map(|s| (s.line, s.members * twins)).collect();
        let Some(stream) = self.streams.named(kind.layoff_stream) else { return };
        let mut d = self.streams.open(
            &stream,
            Subject::new(SubjectTag::Party, employer.get()),
            day,
            phx_core::SubStep::S5c.ordinal(),
        );
        let unit = self.population.representation.multiplicity;
        let mut left = count;
        while left > 0 && !lines.is_empty() {
            let total: u64 = lines.iter().map(|(_, m)| u64::from(*m)).sum();
            if total == 0 {
                break;
            }
            let mut at = phx_rand::uniform::below_u64(&mut d, total);
            let Some(i) = lines.iter().position(|(_, m)| {
                if at < u64::from(*m) {
                    true
                } else {
                    at -= u64::from(*m);
                    false
                }
            }) else {
                break;
            };
            let (line, members) = lines.swap_remove(i);
            let take = if members < left { members } else { left };
            left -= take;
            let take = whole_agents(take, (unit, members), &mut d);
            if take == 0 {
                continue;
            }
            self.labour.book.separations.push(Separation {
                employer,
                country: country.get(),
                line,
                count: take,
                effective,
            });
            self.labour.day.layoffs += u64::from(take);
        }
    }
}

/// The jobs a line loses as whole agents: its workers are agents whose twins act alike, so an employer that is one
/// party lays off whole agents, one more than the whole it asked for with the chance of what is left over, and never
/// more than the line holds.
fn whole_agents(count: u32, (unit, members): (u32, u32), d: &mut phx_rand::Draws) -> u32 {
    if unit <= 1 {
        return count;
    }
    let (whole, over) = (count / unit, count % unit);
    let one_more = over > 0 && phx_rand::uniform::below_u64(d, u64::from(unit)) < u64::from(over);
    let take = (whole + u32::from(one_more)) * unit;
    let most = members - members % unit;
    if take < most { take } else { most }
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::whole_agents;

    fn draws(n: u64) -> Draws {
        Draws::new(stream_key(Seed::new(3), "LAB.layoffs"), Subject::new(SubjectTag::Party, n), 0, 0)
    }

    #[test]
    fn layoffs_come_in_whole_agents_unbiased() {
        assert_eq!(whole_agents(3_400, (1_700, 5_100), &mut draws(1)), 3_400, "whole agents stay whole");
        assert_eq!(whole_agents(80, (1, 200), &mut draws(1)), 80, "agents of one twin take any count");
        let (n, mut laid) = (4_000_u64, 0_u64);
        for i in 0..n {
            let take = whole_agents(170, (1_700, 5_100), &mut draws(i));
            assert!(take == 0 || take == 1_700, "none or one whole agent: {take}");
            laid += u64::from(take);
        }
        // A tenth of an agent asked for lays one off a tenth of the time.
        let mean = phx_rand::float::from_u64(laid) / phx_rand::float::from_u64(n);
        assert!((mean - 170.0).abs() < 20.0, "unbiased: {mean}");
        assert_eq!(whole_agents(9_000, (1_700, 3_400), &mut draws(2)), 3_400, "never more than the line holds");
    }
}
