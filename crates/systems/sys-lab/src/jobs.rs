//! The households' jobs and their persons' labour state at the opening. Each adult is employed at the country's
//! employment rate, and an employee at its sex's share of employees among the employed. An employee's job has an
//! occupation drawn by its sex among those its skill reaches, part-time hours at its sex's share, the law's notice and
//! severance, its household's region and the band its tenure, drawn from the tenure shares, began in. Its wage is the
//! mean wage the labour share gives times its household's income as a multiple of the mean, on the nearest wage
//! point. A job is a row on the employment line of its class and point. Once every household is drawn, each region and
//! occupation's jobs are apportioned over the region's firms by their headcounts and the share of the occupation in
//! their industry's work, and dealt to the lines in an order drawn by lot. Of those not employed, the
//! unemployed search, at the rate that makes their share of the labour force the country's; an adult past its
//! pension's age not employed is retired.

use std::collections::BTreeMap;

use if_labour::class::{NO_OCCUPATION, NOT_SEARCHING, PLACES, RETIRED, SEARCHING};
use if_labour::law::Law;
use phx_core::register::values::{Table1, Table2};
use phx_core::{OpeningCountry, OpeningCtx, Prim, Register, StreamDef, declare_stream};
use phx_id::{Day, PartyId};
use phx_ledger::algebra::{Leg, Schedule, Side};
use phx_ledger::attachments::{AttachmentDraw, Balance, CountryAttachments, Drawing, DrawnRow, Holder, Keys, LineSpec};
use phx_ledger::books::Books;
use phx_ledger::line::{LineKindDecl, SideDecl};
use phx_ledger::opening::{currency, key, whole};
use phx_ledger::rows::BALANCE;
use phx_ledger::terms::TermsId;
use phx_macros::clause;
use phx_num::{Count, Missing, Money, violation};
use phx_rand::{Draws, Subject, open_unit};

use crate::consts::{EMPLOYEES, SHARE_PARTS};

declare_stream! { pub JobsStream = "LAB.opening_jobs" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// Employment: the employer owes the wage to the employee, each job a member; many employers and many employees on
/// a line, so it records no pairing.
pub const EMPLOYMENT: LineKindDecl = LineKindDecl {
    name: "employment",
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

const FIRMS: &str = "FRM.firms";
const SMALL_FIRMS: &str = "FRM.small_firms";
const FIRM_REGIONS: &str = "FRM.firm_regions";
const SMALL_REGIONS: &str = "FRM.small_regions";
const FIRM_INDUSTRIES: &str = "FRM.firm_industries";
const SMALL_INDUSTRIES: &str = "FRM.small_industries";
/// The hours each occupation's work takes in each industry's way, which weigh the occupations a firm employs.
const HOURS_A_UNIT: &str = "TEC.labour";

/// A firm as the jobs' deal reads it: its party, its headcount, its region and its industry.
#[derive(Clone, Copy, Debug)]
struct Employer {
    party: PartyId,
    headcount: u64,
    region: u32,
    industry: i64,
}

/// Labour's draw of the households' jobs.
#[derive(Debug)]
pub struct Jobs {
    pub status: Prim<Table2>,
    pub occupation: Prim<Table2>,
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

/// One country's labour as the households draw it.
struct Country {
    law: Law,
    employed: f64,
    employees: [f64; 2],
    part_time: [f64; 2],
    part_time_hours: u32,
    searching: [f64; 2],
    occupations: Vec<[f64; 2]>,
    tenure: Vec<(i64, f64)>,
    wage: f64,
    kind: u16,
    schedule: Schedule,
    first: Missing<(Day, u32)>,
    date: phx_id::Date,
    ccy: phx_num::Ccy,
    terms: BTreeMap<(i64, Vec<u32>), TermsId>,
    firms: Vec<Employer>,
    /// Each occupation's share of each industry's hours, by (occupation, industry), in parts of a whole.
    shares: BTreeMap<(u32, i64), u64>,
    /// Each line's region, occupation and the jobs drawn on it.
    drawn: BTreeMap<TermsId, (u32, u32, u64)>,
    /// Each line's employers and their jobs, dealt once every household is drawn.
    dealt: Option<BTreeMap<TermsId, Vec<(PartyId, u64)>>>,
}

impl AttachmentDraw for Jobs {
    #[clause("GEN.2", "LAB.1", "PTY.3")]
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        (calendar, today): (&phx_core::Calendar, Day),
        c: &OpeningCountry,
    ) -> Box<dyn CountryAttachments> {
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
        let firms = employers(books, c.id);
        let Ok(hours) = register.table2_in(HOURS_A_UNIT, c.id) else {
            violation!(clause = "GEN.3", "labour's opening with no hours of work by occupation", country = c.id.get());
        };
        let shares = occupation_shares(hours);
        let date = calendar.date(today);
        let dates = phx_ledger::opening::monthly(date, c.id);
        let first = Missing::Present((dates.nth(calendar, 1), 1));
        Box::new(Country {
            law,
            employed,
            employees,
            part_time,
            part_time_hours: match u32::try_from(self.part_time_hours.get(register, c.id).get()) {
                Ok(h) => h,
                Err(_) => violation!(clause = "LAB.1", "part-time hours beyond a week's", country = c.id.get()),
            },
            searching,
            occupations,
            tenure,
            wage: crate::law::mean_wage(c),
            kind: books.ledger.lines.kind_index(EMPLOYMENT.name),
            schedule: Schedule { dates, count: Missing::Absent },
            first,
            date,
            ccy: currency(c.id),
            terms: BTreeMap::new(),
            firms,
            shares,
            drawn: BTreeMap::new(),
            dealt: None,
        })
    }
}

impl Country {
    /// The terms of a wage point and a class: its amount paid on each monthly date.
    fn terms(&mut self, books: &mut Books, point: i64, class: Vec<u32>) -> TermsId {
        if let Some(t) = self.terms.get(&(point, class.clone())) {
            return *t;
        }
        let amount = whole(libm::pow(self.law.point_ratio, phx_rand::float::from_i64(point)));
        let mut terms = phx_ledger::opening::plain_terms(
            self.ccy,
            vec![Leg::FixedAmount(Money::new(amount, self.ccy))],
            self.schedule,
        );
        terms.class.clone_from(&class);
        let t = books.ledger.terms.intern(terms);
        self.terms.insert((point, class), t);
        t
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
}

impl CountryAttachments for Country {
    #[clause("GEN.2", "LAB.1", "REP.34", "PTY.3")]
    fn draw(
        &mut self,
        books: &mut Books,
        h: Drawing<'_>,
        (ctx, subject): (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        keys: &mut Keys,
    ) {
        let mut d = ctx.draws(&JobsStream::DECL, subject);
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        let region = h.household.attr(if_pop::REGION.name);
        let wage = self.wage * h.income;
        let Some(point) = phx_rand::float::floor_to_i64(libm::rint(libm::log(wage) / libm::log(self.law.point_ratio)))
        else {
            violation!(clause = "REP.34", "a wage beyond the wage points");
        };
        let Some(last) = u32::try_from(point).ok().filter(|p| *p < if_labour::class::WAGE_POINTS) else {
            violation!(clause = "REP.34", "a wage point beyond those a person can record", point = point);
        };
        for (place, p) in h.household.persons.iter().enumerate().filter(|(_, p)| adult_roles.contains(&p.role)) {
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
            let retired = p.age_on(self.date) * crate::consts::MONTHS_A_YEAR >= months;
            let (works, as_employee, looks) = (open_unit(&mut d), open_unit(&mut d), open_unit(&mut d));
            let occupation = self.occupation(s, skill, &mut d);
            keys.persons.push((place, crate::LAST_POINT.name, last));
            if works >= self.employed {
                let state = if retired {
                    RETIRED
                } else if looks < searching {
                    SEARCHING
                } else {
                    NOT_SEARCHING
                };
                let known = if state == SEARCHING { occupation } else { NO_OCCUPATION };
                keys.persons.push((place, crate::STATE.name, state));
                keys.persons.push((place, crate::OCCUPATION_ATTR.name, known));
                continue;
            }
            keys.persons.push((place, crate::STATE.name, if retired { RETIRED } else { NOT_SEARCHING }));
            keys.persons.push((place, crate::OCCUPATION_ATTR.name, occupation));
            if as_employee >= employee {
                continue;
            }
            let hours = if open_unit(&mut d) < part_time { self.part_time_hours } else { self.law.full_time_hours };
            let mut class = vec![0; PLACES];
            let places = [
                (if_labour::class::OCCUPATION, occupation),
                (if_labour::class::SKILL, skill),
                (if_labour::class::HOURS, hours),
                (if_labour::class::NOTICE, self.law.notice_days),
                (if_labour::class::SEVERANCE, self.law.severance_days_a_year),
                (if_labour::class::REGION, region),
                (if_labour::class::BAND, self.band(&mut d)),
            ];
            for (i, v) in places {
                if let Some(c) = class.get_mut(i) {
                    *c = v;
                }
            }
            let terms = self.terms(books, point, class);
            self.drawn.entry(terms).or_insert((region, occupation, 0)).2 += 1;
            rows.push(DrawnRow {
                line: LineSpec { kind: self.kind, terms, counterparty: Missing::Absent, first: self.first },
                side: Side::Asset,
                holder: Holder::Person(place),
                balance: Balance::None,
            });
        }
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        Vec::new()
    }

    fn counterparties(&mut self, _: &Books, line: &LineSpec, lot: &mut Draws) -> Vec<(PartyId, u64)> {
        if line.kind != self.kind {
            return Vec::new();
        }
        let dealt = self.dealt.get_or_insert_with(|| deal(&self.drawn, (&self.firms, &self.shares), lot));
        dealt.get(&line.terms).cloned().unwrap_or_default()
    }
}

/// Every firm of a country, large and small, with its headcount, region and industry as the firms' opening drew them.
fn employers(books: &Books, country: phx_id::CountryId) -> Vec<Employer> {
    let list = |name: &str| {
        let Some(l) = books.drawn.get(&key(name, country)) else {
            violation!(clause = "GEN.3", "labour's opening reading firms not yet drawn", country = country.get());
        };
        l
    };
    let mut out = Vec::new();
    for (heads, regions, industries) in
        [(FIRMS, FIRM_REGIONS, FIRM_INDUSTRIES), (SMALL_FIRMS, SMALL_REGIONS, SMALL_INDUSTRIES)]
    {
        for (((party, headcount), (_, region)), (_, industry)) in
            list(heads).iter().zip(list(regions)).zip(list(industries))
        {
            let (Ok(region), Ok(industry)) = (u32::try_from(*region), i64::try_from(*industry)) else {
                violation!(clause = "GEN.3", "a firm drawn in no region or industry", party = party.get());
            };
            out.push(Employer { party: *party, headcount: *headcount, region, industry });
        }
    }
    out
}

/// Each occupation's share of each industry's hours of work, in parts of a whole.
fn occupation_shares(hours: &Table2) -> BTreeMap<(u32, i64), u64> {
    let mut out = BTreeMap::new();
    for industry in hours.columns() {
        let each: Vec<(i64, i64)> =
            hours.rows().iter().filter_map(|o| hours.at(*o, *industry).ok().map(|h| (*o, h))).collect();
        let total: i64 = each.iter().map(|(_, h)| h).sum();
        if total <= 0 {
            continue;
        }
        for (o, h) in each {
            let part = phx_rand::float::from_i64(h) / phx_rand::float::from_i64(total) * SHARE_PARTS;
            if let (Ok(o), Some(part)) = (u32::try_from(o), phx_rand::float::floor_to_u64(part)) {
                out.insert((o, *industry), part);
            }
        }
    }
    out
}

/// Each line's employers: each region and occupation's jobs apportioned over the region's firms by their headcounts
/// times the occupation's share of their industry's hours (over the region's firms by headcount where none employs
/// the occupation, and the country's where the region holds no firm), the firms taken in an order drawn by lot and
/// dealt to the lines in theirs, each line's jobs from the next firm with jobs left.
#[clause("GEN.4", "LAB.1")]
fn deal(
    drawn: &BTreeMap<TermsId, (u32, u32, u64)>,
    (firms, shares): (&[Employer], &BTreeMap<(u32, i64), u64>),
    lot: &mut Draws,
) -> BTreeMap<TermsId, Vec<(PartyId, u64)>> {
    let mut cells: BTreeMap<(u32, u32), Vec<(TermsId, u64)>> = BTreeMap::new();
    for (terms, (region, occupation, jobs)) in drawn {
        cells.entry((*region, *occupation)).or_default().push((*terms, *jobs));
    }
    let mut out = BTreeMap::new();
    for ((region, occupation), lines) in cells {
        let jobs: u64 = lines.iter().map(|(_, n)| n).sum();
        let weigh = |f: &Employer| f.headcount * shares.get(&(occupation, f.industry)).copied().unwrap_or(0);
        let mut weighed: Vec<(PartyId, u64)> =
            firms.iter().filter(|f| f.region == region).map(|f| (f.party, weigh(f))).filter(|(_, w)| *w > 0).collect();
        if weighed.is_empty() {
            weighed = firms.iter().filter(|f| f.region == region).map(|f| (f.party, f.headcount)).collect();
        }
        if weighed.is_empty() {
            weighed = firms.iter().map(|f| (f.party, f.headcount)).collect();
        }
        let weights: Vec<u64> = weighed.iter().map(|(_, w)| *w).collect();
        let quotas = phx_core::contribution::apportion(jobs, &weights, lot);
        let mut givers: Vec<(PartyId, u64)> =
            weighed.iter().zip(quotas).filter(|(_, q)| *q > 0).map(|((p, _), q)| (*p, q)).collect();
        for i in (1..givers.len()).rev() {
            let n = phx_rand::float::len_u64(i + 1);
            let Ok(j) = usize::try_from(phx_rand::below_u64(lot, n)) else {
                violation!(clause = "CHN.1", "a lot beyond the firms drawn among");
            };
            givers.swap(i, j);
        }
        let mut at = 0_usize;
        for (terms, mut need) in lines {
            let mut employers = Vec::new();
            while need > 0 {
                let Some((party, left)) = givers.get_mut(at) else {
                    violation!(clause = "GEN.4", "jobs dealt beyond the firms' apportioned shares", jobs = need);
                };
                let take = if *left < need { *left } else { need };
                employers.push((*party, take));
                *left -= take;
                need -= take;
                if *left == 0 {
                    at += 1;
                }
            }
            out.insert(terms, employers);
        }
    }
    out
}
