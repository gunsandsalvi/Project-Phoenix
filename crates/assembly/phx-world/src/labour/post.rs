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
}

/// The fact names an employer's decision reads.
const PRODUCT: &str = "FRM.industry";
const PRICE: &str = "FRM.price";
const OUTPUT: &str = "FRM.output_rate";
const HURDLE: &str = "FRM.required_return";
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

    /// The employers whose schedule came due today decide.
    pub(super) fn post(&mut self, day: Day) {
        for (rows, slot) in std::mem::take(&mut self.labour.employers_due) {
            self.post_one(day, rows, slot);
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
        let scale = |exp: u8| (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
        Some(Firm {
            product,
            price: phx_rand::float::from_i64(price),
            units_a_day: phx_rand::float::from_i64(output),
            hurdle: phx_rand::float::from_i64(hurdle) / scale(crate::consts::HURDLE_EXP),
        })
    }

    /// An employer's staff on employment lines, a twin's members each.
    fn staff(&self, party: PartyId, twins: u32) -> Vec<Staff> {
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
                    members: r.row.count / twins,
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
    fn post_one(&mut self, day: Day, rows: Rows, slot: Slot) {
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
        let staff = self.staff(row.party, twins);
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
        let mut needs = Vec::new();
        for occupation in 0..class::NO_OCCUPATION {
            let Ok(h) = hours_a_unit.at(i64::from(occupation), firm.product) else { continue };
            let hours_a_unit = phx_rand::float::from_i64(h) / per_unit;
            let held: f64 = staff
                .iter()
                .filter(|s| s.occupation == occupation)
                .map(|s| f64::from(s.members) * f64::from(s.hours) / crate::consts::DAYS_A_WEEK)
                .sum();
            if hours_a_unit <= 0.0 && held <= 0.0 {
                continue;
            }
            let open: f64 = self
                .labour
                .book
                .vacancies
                .iter()
                .filter(|v| v.employer == row.party && v.occupation == occupation)
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
        for v in self.labour.book.vacancies.iter_mut().filter(|v| v.employer == employer) {
            if self.calendar.days_between(v.set, day).is_some_and(|d| d > law.patience_days) {
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
        for &(occupation, jobs) in &out.post {
            let point = self.offer_point(law, (employer, occupation), staff, law.full_time_hours);
            let skill = usize::try_from(occupation).ok().and_then(|o| law.occupation_skill.get(o)).copied();
            let Some(skill) = skill else { continue };
            let id = self.labour.book.next;
            self.labour.book.next += 1;
            self.labour.book.vacancies.push(Vacancy {
                id,
                employer,
                country: country.get(),
                region,
                occupation,
                skill,
                hours: law.full_time_hours,
                point,
                open: jobs * twins,
                first: day,
                set: day,
            });
            self.labour.day.posted += u64::from(jobs);
        }
        for &(occupation, jobs) in &out.withdraw {
            let mut left = jobs * twins;
            for v in self
                .labour
                .book
                .vacancies
                .iter_mut()
                .rev()
                .filter(|v| v.employer == employer && v.occupation == occupation)
            {
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
