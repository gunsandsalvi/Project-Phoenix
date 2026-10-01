//! Each country's statistics agency on the core: the month's records — every sale at the meetings by what it was
//! for, the wages settled — summed per country, and at the month's end its series computed from them and each
//! published on its release day, after its law's lag: the consumer and producer price indices, chained over the
//! products sold in both months at their unit values; the labour force survey from the persons' states; the money
//! stock from the accounts; and the national accounts by production, by expenditure and by income, with the
//! discrepancy between expenditure and income. The consumer index's change reaches the households' outlooks on the
//! day it is published.

use std::collections::BTreeMap;

use if_state::stats::{ACCOUNTS, CPI, IndexKind, LABOUR_FORCE, LIFE_TABLE, MONEY, PPI, Priced, Release, StaLaw};
use phx_core::calendar::Calendar;
use phx_id::{Day, PartyKey};
use phx_macros::clause;
use phx_num::violation;

use crate::core::Core;

/// A life table's events: a death, and an onset of disability.
pub(crate) const DEATH: u8 = 0;
pub(crate) const ONSET: u8 = 1;

/// What a sale at a meeting was for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) enum Purchase {
    /// Bought at retail, by a household or the state.
    Final,
    /// A firm's inputs.
    Inputs,
    /// A firm's fixed investment.
    Investment,
}

/// A month's records of one country: each product's retail purchases by households and each product's sales between
/// firms, as money paid and units; spending by households, the state and on investment; inputs bought; wages paid,
/// and of them those the state's agencies paid. Every sale is a firm's, so the firms' sales are the final purchases
/// and the inputs together; the agencies' output is valued at its cost.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub(crate) struct Month {
    retail: BTreeMap<u16, (i128, i128)>,
    producer: BTreeMap<u16, (i128, i128)>,
    consumption: i128,
    government: i128,
    investment: i128,
    inputs: i128,
    wages: i128,
    public: i128,
    /// The month's deaths and onsets by age class, health before and event.
    vital: BTreeMap<(u32, u32, u8), u64>,
}

impl Month {
    /// What the firms sold at every meeting.
    fn sales(&self) -> i128 {
        self.consumption + self.government + self.investment + self.inputs
    }
}

/// The agencies: each country's law and index base, the month recorded so far and the one before, each index's
/// level, the releases waiting for their day and those published.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct CoreStats {
    laws: Vec<StaLaw>,
    #[saved(skip)]
    pub(crate) index: Option<IndexKind>,
    /// The rate of events over the exposure they happened in, and the age classes' lower bounds.
    #[saved(skip)]
    pub(crate) rate: Option<Rate>,
    classes: Vec<i64>,
    period: Option<u32>,
    months: Vec<Month>,
    before: Vec<Month>,
    /// Each country's consumer and producer index levels.
    levels: Vec<[f64; 2]>,
    waiting: Vec<Release>,
    pub published: Vec<Release>,
    /// The households' outlooks of the published series, and each country's consumer index as last published.
    pub outlooks: crate::core_outlooks::Outlooks,
    last_cpi: BTreeMap<u8, i64>,
}

/// The consumer index's key among the households' public series in a country.
#[must_use]
pub fn cpi_series(country: u8) -> (u16, u32) {
    (u16::try_from(CPI).unwrap_or(u16::MAX), u32::from(country))
}

impl CoreStats {
    /// The day a country's series for a period falls due under its law, where the law publishes it.
    #[must_use]
    pub fn due(&self, calendar: &Calendar, (country, series, period): (u8, usize, u32)) -> Option<Day> {
        let schedule = self.laws.get(usize::from(country))?.series.get(series)?;
        release_day(calendar, (period, country), schedule)
    }
}

/// A rate a year from events over person-days exposed.
pub type Rate = fn(f64, f64) -> Option<f64>;

/// The months from the calendar's year nought a day falls in.
#[must_use]
pub fn period_of(calendar: &Calendar, day: Day) -> u32 {
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
    pub(crate) fn open_stats(
        &mut self,
        (laws, rate, classes): (Vec<StaLaw>, Option<Rate>, Vec<i64>),
        (index, bases): (Option<IndexKind>, Vec<f64>),
    ) {
        let n = laws.len();
        self.stats = CoreStats {
            laws,
            index,
            rate,
            classes,
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
        let household = self.bound.kinds.household.and_then(|k| u8::try_from(k).ok());
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

    /// A death or an onset recorded in its country's month by the person's age class and health.
    pub(crate) fn record_vital(&mut self, country: u8, (age, health): (i64, u32), event: u8) {
        let class = u32::try_from(self.stats.classes.iter().filter(|b| **b <= age).count()).unwrap_or(u32::MAX);
        if let Some(m) = self.stats.months.get_mut(usize::from(country)) {
            *m.vital.entry((class, health, event)).or_insert(0) += 1;
        }
    }

    /// Wages settled today, recorded in their country's month, with those the state's agencies paid.
    pub(crate) fn record_wages(&mut self, ccy: u8, (amount, public): (i64, i64)) {
        if let Some(m) = self.stats.months.get_mut(usize::from(ccy)) {
            m.wages += i128::from(amount);
            m.public += i128::from(public);
        }
    }

    /// The day's statistics: at a new month, the month past computed and queued for each series' release day; and
    /// every release whose day has come published.
    #[clause("STA.1", "STA.2", "STA.3", "STA.4", "IDX.3", "IDX.4", "VAL.5", "VAL.23")]
    pub(crate) fn stats_day(
        &mut self,
        day: Day,
        (calendar, places): (&Calendar, Places<'_>),
        types: Option<&phx_val::types::Types>,
    ) {
        let period = period_of(calendar, day);
        match self.stats.period {
            None => self.stats.period = Some(period),
            Some(p) if p != period => {
                self.close_month(p, calendar, places);
                self.stats.period = Some(period);
            }
            Some(_) => {}
        }
        let (due, later): (Vec<Release>, Vec<Release>) =
            std::mem::take(&mut self.stats.waiting).into_iter().partition(|r| r.published <= day);
        self.stats.waiting = later;
        // The households' methods read the consumer index's change on the day it is published, and no sooner.
        for r in &due {
            let Some(level) = r.values.first().copied() else { continue };
            if usize::from(r.series) != CPI {
                continue;
            }
            if let (Some(before), Some(p)) = (self.stats.last_cpi.insert(r.country, level), types)
                && before > 0
            {
                let change = phx_rand::float::from_i64(level) / phx_rand::float::from_i64(before) - 1.0;
                self.stats.outlooks.print(cpi_series(r.country), change, (day, calendar.date(day).year(), None), p);
            }
        }
        self.stats.published.extend(due);
    }

    /// A month closed: each country's series computed from its records and queued for publication.
    fn close_month(&mut self, period: u32, calendar: &Calendar, (regions, geo): Places<'_>) {
        let labour = self.labour_force(regions);
        let money = self.money_stock((regions, geo));
        let exposed = self.exposed(regions, period);
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
            // Output by production is the firms' sales less the inputs they bought, and the agencies' staff; by
            // expenditure, the final purchases, the state's valued with its agencies' staff; by income, the wages paid
            // and the surplus the firms' sales left over their inputs and their own wages.
            let production = m.sales() - m.inputs + m.public;
            let expenditure = m.consumption + m.government + m.public + m.investment;
            let surplus = m.sales() - m.inputs - (m.wages - m.public);
            let income = m.wages + surplus;
            let whole =
                |x: i128| i64::try_from(x).unwrap_or_else(|_| phx_num::capacity_exceeded!("a total", i64::MAX, 0));
            let life = self.life_table(&m.vital, exposed.get(c));
            let values: [(usize, Option<Vec<i64>>); 6] = [
                (CPI, levels.map(|l| vec![at_places(l[0], if_state::stats::PLACES[CPI])])),
                (PPI, levels.map(|l| vec![at_places(l[1], if_state::stats::PLACES[PPI])])),
                (LABOUR_FORCE, labour.get(c).map(|l| l.to_vec())),
                (MONEY, money.get(c).cloned()),
                (LIFE_TABLE, life),
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

    /// A country's period life table for the month: for each age class and health exposed, its deaths and, for the
    /// able, its onsets, each with its person-days exposed and its rate a year.
    fn life_table(
        &self,
        vital: &BTreeMap<(u32, u32, u8), u64>,
        exposed: Option<&BTreeMap<(u32, u32), u64>>,
    ) -> Option<Vec<i64>> {
        let (rate, exposed) = (self.stats.rate?, exposed?);
        let mut out = Vec::new();
        for ((class, health), days) in exposed {
            let events: &[u8] = if *health == if_pop::ABLE { &[DEATH, ONSET] } else { &[DEATH] };
            for event in events {
                let n = vital.get(&(*class, *health, *event)).copied().unwrap_or(0);
                let r = rate(phx_rand::float::from_u64(n), phx_rand::float::from_u64(*days))?;
                let at = |v: u64| i64::try_from(v).unwrap_or(i64::MAX);
                out.extend([
                    i64::from(*class),
                    i64::from(*health),
                    i64::from(*event),
                    at(n),
                    at(*days),
                    at_places(r, if_state::stats::PLACES[LIFE_TABLE]),
                ]);
            }
        }
        Some(out)
    }

    /// Each country's person-days exposed over a month by age class and health: the persons at its close, over the
    /// month's days.
    fn exposed(&self, regions: &[phx_id::CountryId], period: u32) -> Vec<BTreeMap<(u32, u32), u64>> {
        let mut out = vec![BTreeMap::new(); self.stats.laws.len()];
        let Some(place) = self.bound.kinds.household else {
            return out;
        };
        let Some((year, month)) = month_of(period) else { return out };
        let (Some(date), Some(days)) = (phx_id::Date::new(year, month, 1), phx_id::Date::days_in_month(year, month))
        else {
            return out;
        };
        let days = u64::from(days);
        for slot in self.directory.live_slots(crate::core::kind_number(place)) {
            let Some(region) = self.household_region(slot).and_then(|r| usize::try_from(r).ok()) else { continue };
            let Some(table) = regions.get(region).and_then(|c| out.get_mut(usize::from(c.get()))) else { continue };
            for (_, person) in self.members(slot) {
                let age = person.age_on(date);
                let class = u32::try_from(self.stats.classes.iter().filter(|b| **b <= age).count()).unwrap_or(u32::MAX);
                let health = person.get(if_pop::HEALTH.field);
                *table.entry((class, health)).or_insert(0) += days;
            }
        }
        out
    }

    /// Each country's labour force at the day: its persons employed (holding a job or working as owners), unemployed and searching, and the
    /// adults out of the labour force.
    fn labour_force(&self, regions: &[phx_id::CountryId]) -> Vec<[i64; if_state::stats::LABOUR_STATES]> {
        let mut out = vec![[0_i64; if_state::stats::LABOUR_STATES]; self.stats.laws.len()];
        let Some(place) = self.bound.kinds.household else {
            return out;
        };
        let employed = self.employed_ids();
        for slot in self.directory.live_slots(crate::core::kind_number(place)) {
            let Some(region) = self.household_region(slot).and_then(|r| usize::try_from(r).ok()) else { continue };
            let Some(row) = regions.get(region).and_then(|c| out.get_mut(usize::from(c.get()))) else { continue };
            for (r, person) in self.members(slot) {
                if person.get(phx_core::person_word::ROLE) == if_pop::CHILD.value {
                    continue;
                }
                let state = person.get(sys_lab::STATE.field);
                let at = if employed.binary_search(&r.word()).is_ok() {
                    0
                } else if state == if_labour::class::SEARCHING {
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

    /// The persons holding a job or working in a firm they own, by identity, sorted.
    fn employed_ids(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self
            .families
            .iter()
            .filter(|f| f.reason == crate::consts::reason::WAGE)
            .flat_map(|f| f.store.edges.open_slots().filter_map(|e| f.store.edges.row(e)).map(|r| r.person))
            .chain(self.owners.works_at.keys().map(|(_, person)| *person))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Each country's money stock at the day: the banks' reserves, and deposits held by households, by firms and by
    /// every other holder.
    fn money_stock(&self, (regions, geo): (&[phx_id::CountryId], &phx_geo::GeoState)) -> Vec<Vec<i64>> {
        let mut out = vec![vec![0_i64; crate::consts::stats::MONEY_CLASSES]; self.stats.laws.len()];
        for (k, store) in self.kinds.iter().enumerate() {
            let Some(a) = store.accounts.as_ref() else { continue };
            let class = crate::core_kinds::of(&self.declared.kinds, k).money_class;
            for slot in self.directory.live_slots(crate::core::kind_number(k)) {
                let party = PartyKey::new(crate::core::kind_number(k), slot);
                let Some(country) = self.country_of_party(party, (regions, geo)) else {
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

    /// The country a party is of, read from where its kind declares its place.
    fn country_of_party(
        &self,
        party: PartyKey,
        (regions, geo): (&[phx_id::CountryId], &phx_geo::GeoState),
    ) -> Option<usize> {
        let k = usize::from(party.kind());
        let tile_country = |tile: u32| match geo.zone_of(phx_id::TileId::new(tile)) {
            phx_num::Missing::Present(zone) => match geo.zone_country(zone) {
                phx_num::Missing::Present(c) => Some(usize::from(c.get())),
                phx_num::Missing::Absent => None,
            },
            phx_num::Missing::Absent => None,
        };
        let place = crate::core_kinds::of(&self.declared.kinds, k).place;
        if place == phx_core::Place::Zone {
            let region = self.zoned_region(party)?;
            return regions.get(usize::try_from(region).ok()?).map(|c| usize::from(c.get()));
        }
        let at = self.places.get(k)?.as_ref()?.at(party.slot());
        crate::core_kinds::country_by_place(place, at, regions, tile_country)
    }
}

/// Where the core's parties are: each region's country, and the map their sites' tiles lie on.
pub(crate) type Places<'a> = (&'a [phx_id::CountryId], &'a phx_geo::GeoState);

/// A period's year and month.
#[must_use]
pub fn month_of(period: u32) -> Option<(i32, u8)> {
    let per_year = u32::from(phx_core::consts::MONTHS_PER_YEAR);
    let (year, month) = (i32::try_from(period / per_year).ok()?, u8::try_from(period % per_year).ok()?);
    if month == 0 { Some((year - 1, u8::try_from(per_year).ok()?)) } else { Some((year, month)) }
}

/// The day a period's series is released in its country: the business day of its schedule, counted in the month its
/// law's lag after the period's month.
#[must_use]
pub fn release_day(
    calendar: &Calendar,
    (period, country): (u32, u8),
    schedule: &if_state::stats::Schedule,
) -> Option<Day> {
    let (year, month) = month_of(period + 1 + schedule.lag)?;
    let country = phx_id::CountryId::new(country);
    let mut day = calendar.on_or_after(country, calendar.day(phx_id::Date::new(year, month, 1)?)?);
    for _ in 1..schedule.day {
        day = calendar.next_business(country, day);
    }
    Some(day)
}
