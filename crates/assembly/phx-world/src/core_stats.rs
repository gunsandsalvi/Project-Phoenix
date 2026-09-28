//! Each country's statistics agency on the core: the month's records — every sale at the meetings by what it was
//! for, the wages settled — summed per country, and at the month's end its series computed from them and each
//! published on its release day, after its law's lag: the consumer and producer price indices, chained over the
//! products sold in both months at their unit values; the labour force survey from the persons' states; the money
//! stock from the accounts; and the national accounts by production, by expenditure and by income, with the
//! discrepancy between expenditure and income.

use std::collections::BTreeMap;

use if_state::stats::{ACCOUNTS, CPI, IndexKind, LABOUR_FORCE, MONEY, PPI, Priced, Release, StaLaw};
use phx_core::calendar::Calendar;
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::violation;

use crate::core::Core;

/// What a sale at a meeting was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Purchase {
    /// Bought at retail, by a household or the state.
    Final,
    /// A firm's inputs.
    Inputs,
    /// A firm's fixed investment.
    Investment,
}

/// A month's records of one country: each product's retail purchases by households and each product's sales between
/// firms, as money paid and units; spending by households, the state and on investment; inputs bought; wages paid.
/// Every sale is a firm's, so the firms' sales are the final purchases and the inputs together.
#[derive(Clone, Debug, Default)]
pub(crate) struct Month {
    retail: BTreeMap<u16, (i128, i128)>,
    producer: BTreeMap<u16, (i128, i128)>,
    consumption: i128,
    government: i128,
    investment: i128,
    inputs: i128,
    wages: i128,
}

impl Month {
    /// What the firms sold at every meeting.
    fn sales(&self) -> i128 {
        self.consumption + self.government + self.investment + self.inputs
    }
}

/// The agencies: each country's law and index base, the month recorded so far and the one before, each index's
/// level, the releases waiting for their day and those published.
#[derive(Debug, Default)]
pub struct CoreStats {
    laws: Vec<StaLaw>,
    index: Option<IndexKind>,
    period: Option<u32>,
    months: Vec<Month>,
    before: Vec<Month>,
    /// Each country's consumer and producer index levels.
    levels: Vec<[f64; 2]>,
    waiting: Vec<Release>,
    pub published: Vec<Release>,
}

/// The months from the calendar's year nought a day falls in.
fn period_of(calendar: &Calendar, day: Day) -> u32 {
    let date = calendar.date(day);
    let months = i64::from(date.year()) * i64::from(phx_core::consts::MONTHS_PER_YEAR) + i64::from(date.month());
    u32::try_from(months).unwrap_or_else(|_| violation!(clause = "STA.2", "a period before the calendar's year nought"))
}

/// A value at so many decimal places, whole.
fn at_places(x: f64, places: u8) -> i64 {
    let scale = (0..places).fold(1.0_f64, |s, _| s * phx_rand::float::from_u64(crate::consts::stats::TEN));
    phx_rand::float::floor_to_i64((x * scale).round()).unwrap_or_else(|| {
        phx_num::capacity_exceeded!("a published value", i64::MAX, 0);
    })
}

/// The chain's link over two months' records: each product's unit value in both, weighted by the month before's
/// spending on it.
fn link(index: &IndexKind, before: &BTreeMap<u16, (i128, i128)>, now: &BTreeMap<u16, (i128, i128)>) -> Option<f64> {
    let unit = |(paid, units): (i128, i128)| {
        (units > 0).then(|| phx_rand::float::from_i128(paid) / phx_rand::float::from_i128(units))
    };
    let priced: Vec<Priced> = before
        .iter()
        .filter_map(|(p, b)| {
            let n = now.get(p)?;
            Some(Priced { before: unit(*b)?, now: unit(*n)?, weight: phx_rand::float::from_i128(b.0) })
        })
        .collect();
    (index.link)(&priced)
}

impl Core {
    /// The agencies opened: each country's law and the indices' base levels.
    pub(crate) fn open_stats(&mut self, laws: Vec<StaLaw>, (index, bases): (Option<IndexKind>, Vec<f64>)) {
        let n = laws.len();
        self.stats = CoreStats {
            laws,
            index,
            months: vec![Month::default(); n],
            before: vec![Month::default(); n],
            levels: bases.into_iter().map(|b| [b, b]).collect(),
            ..CoreStats::default()
        };
    }

    /// A sale recorded in its seller's country's month.
    pub(crate) fn record_sale(
        &mut self,
        ccy: u8,
        (buyer_kind, purpose): (u8, Purchase),
        (product, paid, units): (u16, i64, i64),
    ) {
        let household = self.names.iter().position(|n| *n == "household").and_then(|k| u8::try_from(k).ok());
        let Some(m) = self.stats.months.get_mut(usize::from(ccy)) else { return };
        let (paid, units) = (i128::from(paid), i128::from(units));
        let add = |map: &mut BTreeMap<u16, (i128, i128)>| {
            let e = map.entry(product).or_insert((0, 0));
            (e.0, e.1) = (e.0 + paid, e.1 + units);
        };
        match purpose {
            Purchase::Final if Some(buyer_kind) == household => {
                m.consumption += paid;
                add(&mut m.retail);
            }
            Purchase::Final => m.government += paid,
            Purchase::Inputs => {
                m.inputs += paid;
                add(&mut m.producer);
            }
            Purchase::Investment => {
                m.investment += paid;
                add(&mut m.producer);
            }
        }
    }

    /// Wages settled today, recorded in their country's month.
    pub(crate) fn record_wages(&mut self, ccy: u8, amount: i64) {
        if let Some(m) = self.stats.months.get_mut(usize::from(ccy)) {
            m.wages += i128::from(amount);
        }
    }

    /// The day's statistics: at a new month, the month past computed and queued for each series' release day; and
    /// every release whose day has come published.
    #[clause("STA.1", "STA.2", "STA.3", "STA.4", "IDX.3", "IDX.4")]
    pub(crate) fn stats_day(&mut self, day: Day, calendar: &Calendar, regions: &[phx_id::CountryId]) {
        let period = period_of(calendar, day);
        match self.stats.period {
            None => self.stats.period = Some(period),
            Some(p) if p != period => {
                self.close_month(p, calendar, regions);
                self.stats.period = Some(period);
            }
            Some(_) => {}
        }
        let (due, later): (Vec<Release>, Vec<Release>) =
            std::mem::take(&mut self.stats.waiting).into_iter().partition(|r| r.published <= day);
        self.stats.waiting = later;
        self.stats.published.extend(due);
    }

    /// A month closed: each country's series computed from its records and queued for publication.
    fn close_month(&mut self, period: u32, calendar: &Calendar, regions: &[phx_id::CountryId]) {
        let labour = self.labour_force(regions);
        let money = self.money_stock(regions);
        let months = std::mem::take(&mut self.stats.months);
        let n = months.len();
        let before = std::mem::replace(&mut self.stats.before, months.clone());
        self.stats.months = vec![Month::default(); n];
        for (c, (m, b)) in months.iter().zip(&before).enumerate() {
            let Ok(country) = u8::try_from(c) else { continue };
            let Some(law) = self.stats.laws.get(c).cloned() else { continue };
            if let (Some(index), Some(levels)) = (self.stats.index, self.stats.levels.get_mut(c)) {
                for (level, (b, now)) in levels.iter_mut().zip([(&b.retail, &m.retail), (&b.producer, &m.producer)]) {
                    if let Some(l) = link(&index, b, now) {
                        *level *= l;
                    }
                }
            }
            let levels = self.stats.levels.get(c).copied();
            // Output by production is the firms' sales less the inputs they bought; by expenditure, the final
            // purchases; by income, the wages paid and the surplus the firms' sales left over their inputs and wages.
            let production = m.sales() - m.inputs;
            let expenditure = m.consumption + m.government + m.investment;
            let surplus = m.sales() - m.inputs - m.wages;
            let income = m.wages + surplus;
            let whole =
                |x: i128| i64::try_from(x).unwrap_or_else(|_| phx_num::capacity_exceeded!("a total", i64::MAX, 0));
            let values: [(usize, Option<Vec<i64>>); 5] = [
                (CPI, levels.map(|l| vec![at_places(l[0], if_state::stats::PLACES[CPI])])),
                (PPI, levels.map(|l| vec![at_places(l[1], if_state::stats::PLACES[PPI])])),
                (LABOUR_FORCE, labour.get(c).map(|l| l.to_vec())),
                (MONEY, money.get(c).cloned()),
                (
                    ACCOUNTS,
                    Some(vec![whole(production), whole(expenditure), whole(income), whole(expenditure - income)]),
                ),
            ];
            for (series, values) in values {
                let (Some(values), Some(schedule)) = (values, law.series.get(series)) else { continue };
                let Some(published) = release_day(calendar, (period, country), schedule) else { continue };
                self.stats.waiting.push(Release {
                    series: u8::try_from(series).unwrap_or(u8::MAX),
                    country,
                    period,
                    vintage: 0,
                    published,
                    values,
                });
            }
        }
    }

    /// Each country's labour force at the day: its persons employed (holding a job), unemployed and searching, and the
    /// adults out of the labour force.
    fn labour_force(&self, regions: &[phx_id::CountryId]) -> Vec<[i64; if_state::stats::LABOUR_STATES]> {
        let mut out = vec![[0_i64; if_state::stats::LABOUR_STATES]; self.stats.laws.len()];
        let (Some(place), Some(decl)) =
            (self.names.iter().position(|n| *n == "household"), self.household_decl.as_ref())
        else {
            return out;
        };
        let (Some(store), Some(Some(persons))) = (self.kinds.get(place), self.persons.get(place)) else { return out };
        let phx_num::Missing::Present(region_at) = decl.sited_by else { return out };
        let employed = self.employed_ids();
        for slot in store.parties.live_slots() {
            let Some(region) = store.record(slot).get(region_at).and_then(|w| match w.get() {
                phx_num::Missing::Present(v) => usize::try_from(v).ok(),
                phx_num::Missing::Absent => None,
            }) else {
                continue;
            };
            let Some(row) = regions.get(region).and_then(|c| out.get_mut(usize::from(c.get()))) else { continue };
            for p in persons.of(slot) {
                let person = phx_pop::person::unpack(decl, p.word);
                if person.role == if_pop::CHILD.name {
                    continue;
                }
                let state = person.attr(sys_lab::STATE.name);
                let at = if employed.binary_search(&p.id).is_ok() {
                    0
                } else if state == Some(if_labour::class::SEARCHING) {
                    1
                } else {
                    2
                };
                if let Some(n) = row.get_mut(at) {
                    *n += 1;
                }
            }
        }
        out
    }

    /// The persons holding a job, by identity, sorted.
    fn employed_ids(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .families
            .iter()
            .filter(|f| f.reason == crate::consts::reason::WAGE)
            .flat_map(|f| f.store.edges.open_slots().filter_map(|e| f.store.edges.row(e)).map(|r| r.person))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Each country's money stock at the day: the banks' reserves, and deposits held by households, by firms and by
    /// every other holder.
    fn money_stock(&self, regions: &[phx_id::CountryId]) -> Vec<Vec<i64>> {
        let mut out = vec![vec![0_i64; crate::consts::stats::MONEY_CLASSES]; self.stats.laws.len()];
        let at = |name: &str| self.names.iter().position(|n| *n == name);
        for (k, store) in self.kinds.iter().enumerate() {
            let Some(a) = store.accounts.as_ref() else { continue };
            let class = if Some(k) == self.bank_kind.map(usize::from) {
                0
            } else if Some(k) == at("household") {
                1
            } else if Some(k) == at("firm") {
                2
            } else {
                crate::consts::stats::MONEY_CLASSES - 1
            };
            for slot in store.parties.live_slots() {
                let Some(country) = self.country_of_party(PartyKey::new(crate::core::kind_number(k), slot), regions)
                else {
                    continue;
                };
                let (Some(b), Some(p)) = (a.balance.get(slot), a.pending.get(slot)) else { continue };
                if let Some(v) = out.get_mut(country).and_then(|v| v.get_mut(class)) {
                    *v += b + p;
                }
            }
        }
        out
    }

    /// The country a party is of: a household or a firm by its region, a bank, a treasury or an estate by the country
    /// that holds it.
    fn country_of_party(&self, party: PartyKey, regions: &[phx_id::CountryId]) -> Option<usize> {
        let k = usize::from(party.kind());
        let name = *self.names.get(k)?;
        let region = |at: usize| match self.kinds.get(k)?.record(party.slot()).get(at)?.get() {
            phx_num::Missing::Present(v) => regions.get(usize::try_from(v).ok()?).map(|c| usize::from(c.get())),
            phx_num::Missing::Absent => None,
        };
        match name {
            "household" => match self.household_decl.as_ref()?.sited_by {
                phx_num::Missing::Present(i) => region(i),
                phx_num::Missing::Absent => None,
            },
            "firm" => region(crate::consts::firm::REGION),
            "bank" => self.banks_of.iter().position(|b| b.iter().any(|(s, _)| *s == party.slot().get())),
            "treasury" => self.treasuries.iter().position(|t| *t == Some(party)),
            _ => self.estates.iter().find(|(e, _, _)| *e == party).map(|(_, c, _)| usize::from(c.get())),
        }
    }
}

/// The day a period's series is released in its country: the business day of its schedule, counted in the month its
/// law's lag after the period's month.
fn release_day(calendar: &Calendar, (period, country): (u32, u8), schedule: &if_state::stats::Schedule) -> Option<Day> {
    let months = period + 1 + schedule.lag;
    let per_year = u32::from(phx_core::consts::MONTHS_PER_YEAR);
    let (year, month) = (i32::try_from(months / per_year).ok()?, u8::try_from(months % per_year).ok()?);
    let (year, month) = if month == 0 { (year - 1, u8::try_from(per_year).ok()?) } else { (year, month) };
    let country = phx_id::CountryId::new(country);
    let mut day = calendar.on_or_after(country, calendar.day(phx_id::Date::new(year, month, 1)?)?);
    for _ in 1..schedule.day {
        day = calendar.next_business(country, day);
    }
    Some(day)
}
