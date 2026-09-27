//! The statistics agencies in the world. Each day at the close an agency samples the day's sales at retail and at
//! the goods markets, and the births, deaths and onsets of disability in its panel of households as they happen; at
//! each month's end it counts its panel's persons by age class, health and labour state, and reads the banks' and the
//! central bank's reports of money. On its calendar's days it publishes each period's first release from the returns
//! in by then, and its revision from every return; a reader reads only what was published by its day.

use std::collections::BTreeMap;

use if_state::stats::{
    ACCOUNTS, CPI, HOLDER_CLASSES, IndexKind, LABOUR_FORCE, LABOUR_STATES, LIFE_TABLE, MONEY, PLACES, PPI, Priced,
    Release, SERIES, StaKind, StaLaw,
};
use phx_core::Household;
use phx_id::{CountryId, Date, Day, PartyId};
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::price::pow10;
use phx_num::{Missing, violation};
use phx_rand::float::{from_i64, from_u64};
use phx_rand::{Subject, SubjectTag};

use crate::world::World;

/// A product's sampled sales in a period: the value and quantity of the early returns, and of every return.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Sampled {
    pub early: (i128, i128),
    pub all: (i128, i128),
}

/// What an agency has collected for one period of one country: each index's sampled sales by product; the panel's
/// deaths and onsets by age class and health, early and in all; its persons by age class and health at the period's
/// first and last census, with the days between; its labour force at the last; and the money reported.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Collected {
    pub country: u8,
    pub period: u32,
    pub prices: BTreeMap<(u8, u16), Sampled>,
    pub vital: BTreeMap<(u8, u8, u8), (u64, u64)>,
    pub start: BTreeMap<(u8, u8), u64>,
    pub end: BTreeMap<(u8, u8), u64>,
    pub census_days: (u32, u32),
    pub labour: Vec<u64>,
    pub money: Vec<i128>,
    pub accounts: BTreeMap<u8, Sampled>,
    pub income: i128,
    pub stocks: (i128, i128),
}

/// The national accounts' records of firms' trades: what firms sold, what they bought of each other, what
/// households bought of them, the plant firms bought, and what everyone else bought of them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Output,
    Purchases,
    Consumption,
    Investment,
    Other,
}

/// The items in the order their records are kept.
const ITEMS: [Item; 5] = [Item::Output, Item::Purchases, Item::Consumption, Item::Investment, Item::Other];

impl Item {
    /// The item's key among a period's records: its place in their order.
    fn code(self) -> u8 {
        let Some(at) = ITEMS.iter().position(|i| *i == self).and_then(|i| u8::try_from(i).ok()) else {
            violation!(clause = "STA.3", "an item of the accounts kept in no order");
        };
        at
    }
}

/// Where a party stands in the national accounts: a firm, a household, or anything else.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sector {
    Firm,
    Household,
    Other,
}

/// What the agencies carry across days: what each has collected of each period not yet revised or still read by the
/// next period's link, the releases, and the tape's match sets read so far.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct StatsBook {
    pub collected: Vec<Collected>,
    pub releases: Vec<Release>,
    pub read: u32,
}

/// A death, or an onset of disability, in a panel household: its country, the person's age class and health, and
/// whether its return is in early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Vital {
    pub country: u8,
    pub class: u8,
    pub health: u8,
    pub onset: bool,
    pub early: bool,
}

/// The day's statistics: records sampled and releases published.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatsDay {
    pub sampled: u64,
    pub published: u64,
}

/// The agencies as the world keeps them: their kinds, each country's law and index base, and the age classes they
/// report by.
#[derive(Clone, Debug, Default)]
pub(crate) struct Stats {
    pub kind: Option<StaKind>,
    pub index: Option<IndexKind>,
    pub laws: Vec<Missing<(StaLaw, f64)>>,
    pub classes: Vec<i64>,
}

/// A month as a number: twelve for each year and one for each month before it.
pub(crate) fn month_of(date: Date) -> u32 {
    let months = i64::from(date.year()) * crate::consts::MONTHS + i64::from(date.month()) - 1;
    let Ok(m) = u32::try_from(months) else { violation!(clause = "TIME.2", "a month before the calendar's") };
    m
}

/// The first day of a month by its number.
fn first_of(month: u32) -> Date {
    let Ok(a_year) = u32::try_from(crate::consts::MONTHS) else { violation!(clause = "TIME.2", "a year of no months") };
    let (Ok(year), Ok(m)) = (i32::try_from(month / a_year), u8::try_from(month % a_year + 1)) else {
        violation!(clause = "TIME.2", "a month beyond the calendar's", month = month);
    };
    let Some(d) = Date::new(year, m, 1) else { violation!(clause = "TIME.2", "a month with no first day") };
    d
}

/// A person's age class, the last whose first age it has reached.
fn class_of(classes: &[i64], age: i64) -> Option<u8> {
    classes.partition_point(|c| *c <= age).checked_sub(1).and_then(|c| u8::try_from(c).ok())
}

/// Whether a uniform draw falls within a share.
fn within(u: f64, share: f64) -> bool {
    u < share
}

/// An `i128` as the nearest `f64`, from its halves.
fn from_i128(x: i128) -> f64 {
    let half = u32::BITS;
    let (Ok(high), Ok(low)) = (i64::try_from(x >> half), i64::try_from(x & i128::from(u32::MAX))) else {
        phx_num::capacity_exceeded!("a sum beyond reading", i64::MAX, 0);
    };
    from_i64(high) * from_u64(1_u64 << half) + from_i64(low)
}

/// A series' values' scale: ten to its places.
fn scale(series: usize) -> Option<f64> {
    PLACES.get(series).map(|p| from_i128(pow10(*p)))
}

/// The agencies bound: each country's law and base, and the age classes they report by.
///
/// # Errors
/// A country's law or base refused, or the age classes missing.
pub(crate) fn bind(
    d: &phx_core::Declarations,
    register: &phx_core::Register,
    countries: &[phx_core::OpeningCountry],
) -> Result<Stats, Vec<String>> {
    let kind = d.markets.iter().find_map(|(_, k)| k.downcast_ref::<StaKind>()).copied();
    let index = d.markets.iter().find_map(|(_, k)| k.downcast_ref::<IndexKind>()).copied();
    let mut errors = Vec::new();
    let laws = countries
        .iter()
        .map(|c| match (kind, index) {
            (Some(k), Some(i)) => match ((k.law)(register, c), (i.base)(register, c)) {
                (Ok(law), Ok(base)) => Missing::Present((law, base)),
                (Err(e), _) | (_, Err(e)) => {
                    errors.push(format!("the statistics law in country {}: {e}", c.id.get()));
                    Missing::Absent
                }
            },
            _ => Missing::Absent,
        })
        .collect();
    let classes = match kind {
        Some(k) => register.partition(k.classes).map_or_else(
            |e| {
                errors.push(e);
                Vec::new()
            },
            |p| p.bounds.to_vec(),
        ),
        None => Vec::new(),
    };
    if errors.is_empty() { Ok(Stats { kind, index, laws, classes }) } else { Err(errors) }
}

impl World {
    fn sta_law(&self, country: u8) -> Option<&(StaLaw, f64)> {
        match self.state.stats.laws.get(usize::from(country)) {
            Some(Missing::Present(l)) => Some(l),
            _ => None,
        }
    }

    /// Whether a household is in its country's panel for a series, and whether its return for a period is in early.
    fn panel(&self, party: PartyId, (country, series, period): (u8, usize, u32)) -> Option<bool> {
        let kind = self.state.stats.kind?;
        let (law, _) = self.sta_law(country)?;
        let share = law.series.get(series)?.sample;
        let subject = Subject::new(SubjectTag::Party, party.get());
        let place = u32::try_from(series).ok()?;
        let u = phx_rand::open_unit(&mut self.streams.open_keyed_at(&kind.sample, subject, place));
        if !within(u, share) {
            return None;
        }
        let e = phx_rand::open_unit(&mut self.streams.open_keyed_at(&kind.returns, subject, period));
        Some(within(e, law.early))
    }

    /// Whether a household is in its country's panel for the period life table, and whether its return for the
    /// month is in early.
    pub(crate) fn life_panel(&self, party: PartyId, country: Missing<CountryId>, date: Date) -> Option<bool> {
        let Missing::Present(c) = country else { return None };
        self.panel(party, (c.get(), LIFE_TABLE, month_of(date)))
    }

    /// The deaths and onsets of disability a panel household's outcomes made: each person gone, and each able person
    /// left disabled, by its age class and health before.
    #[clause("STA.1", "STA.2")]
    pub(crate) fn vital_of(
        &self,
        country: Missing<CountryId>,
        date: Date,
        early: bool,
        (before, after): (&[phx_core::Person], &Household),
    ) -> Vec<Vital> {
        let Missing::Present(c) = country else { return Vec::new() };
        let c = c.get();
        let mut out = Vec::new();
        for (p, now) in before.iter().zip(&after.persons).filter(|(p, _)| !p.gone) {
            let Some(class) = class_of(&self.state.stats.classes, p.age_on(date)) else { continue };
            let Some(health) = p.attr(if_pop::HEALTH.name).and_then(|h| u8::try_from(h).ok()) else { continue };
            let onset = now.attr(if_pop::HEALTH.name) == Some(if_pop::DISABLED) && u32::from(health) == if_pop::ABLE;
            if now.gone || onset {
                out.push(Vital { country: c, class, health, onset: !now.gone, early });
            }
        }
        out
    }

    /// A country's collection for a period, begun where none is.
    fn collecting(&mut self, country: u8, period: u32) -> &mut Collected {
        let book = &mut self.state.book.stats.collected;
        let found = book.iter().position(|c| (c.country, c.period) == (country, period));
        let at = if let Some(i) = found {
            i
        } else {
            book.push(Collected { country, period, ..Collected::default() });
            book.len() - 1
        };
        let Some(c) = book.get_mut(at) else { violation!(clause = "STA.2", "a collection lost as it was begun") };
        c
    }

    /// Vital events recorded against their period.
    pub(crate) fn record_vital(&mut self, date: Date, events: &[Vital], twins: u64) {
        let period = month_of(date);
        for v in events {
            let cell =
                self.collecting(v.country, period).vital.entry((v.class, v.health, u8::from(v.onset))).or_default();
            if v.early {
                cell.0 += twins;
            }
            cell.1 += twins;
        }
    }

    /// 10a: the day's sales sampled; at a month's last day its census and money counted; and each release due today
    /// published.
    #[clause("STA.1", "STA.2", "STA.4", "IDX.1", "IDX.3", "IDX.4", "MON.10")]
    pub(crate) fn stats_close(&mut self, day: Day) {
        let Some(kind) = self.state.stats.kind else { return };
        let date = self.calendar.date(day);
        self.sample_sales(kind, date);
        self.sample_accounts(kind, date);
        if self.state.book.stats.collected.iter().all(|c| c.census_days.0 == 0)
            || month_of(self.calendar.date(day.succ())) != month_of(date)
        {
            self.census(day);
        }
        self.publish(day);
    }

    /// The day's match sets read from the tape: each sale at a consumer or producer market sampled for its seller's
    /// country's index, early or late.
    fn sample_sales(&mut self, kind: StaKind, date: Date) {
        let sets = self.markets.tape.sets();
        let from = usize::try_from(self.state.book.stats.read).unwrap_or(usize::MAX);
        let mut sampled: Vec<(u8, u8, u16, i128, i128, bool)> = Vec::new();
        for (s, set) in sets.iter().enumerate().skip(from) {
            let decl = self.market_kinds.decl(&self.markets.made, set.market);
            let (series, product) = if kind.consumer.contains(&decl.key.kind) {
                (CPI, crate::retail::retail_of(decl.key.subject).map(|(p, _)| p))
            } else if kind.producer.contains(&decl.key.kind) {
                (PPI, Some(phx_ledger::goods::GoodKey::from_code(decl.key.subject).product))
            } else {
                continue;
            };
            let Some(product) = product else { continue };
            for (m, x) in set.matches.iter().enumerate() {
                let Missing::Present(country) = self.country_of_party(x.seller) else { continue };
                let c = country.get();
                let Some((law, _)) = self.sta_law(c) else { continue };
                let Some(schedule) = law.series.get(series) else { continue };
                // A sale is drawn by its match set and its place in it, the same whenever it is read.
                let (Ok(set_no), Ok(place), Ok(series_no)) = (u64::try_from(s), u32::try_from(m), u8::try_from(series))
                else {
                    continue;
                };
                let subject = Subject::new(SubjectTag::Market, set_no);
                let drawn = |decl: &phx_core::StreamDecl| {
                    phx_rand::open_unit(&mut self.streams.open_keyed_at(decl, subject, place))
                };
                if !within(drawn(&kind.sample), schedule.sample) {
                    continue;
                }
                let early = within(drawn(&kind.returns), law.early);
                let value = i128::from(x.qty) * i128::from(x.price.raw());
                sampled.push((c, series_no, product, value, i128::from(x.qty), early));
            }
        }
        self.state.book.stats.read = u32::try_from(sets.len()).unwrap_or(u32::MAX);
        let period = month_of(date);
        self.state.day.stats.sampled += phx_rand::float::len_u64(sampled.len());
        for (c, series, product, value, qty, early) in sampled {
            let cell = self.collecting(c, period).prices.entry((series, product)).or_default();
            if early {
                cell.early = (cell.early.0 + value, cell.early.1 + qty);
            }
            cell.all = (cell.all.0 + value, cell.all.1 + qty);
        }
    }

    /// A live party's sector and country, by its kind.
    fn sector_of(&self, party: PartyId) -> Option<(Sector, u8)> {
        let phx_core::Resolved::Live(live, _) = self.books.parties.directory().resolve(party) else { return None };
        let Missing::Present(country) = self.country_of_party(live) else { return None };
        let (place, _) = self.books.parties.row(live);
        let kind = self.books.parties.holder(place).kind();
        let sector = if kind == sys_frm::FIRM.name || kind == sys_frm::SMALL_FIRM.name {
            Sector::Firm
        } else if kind == if_pop::HOUSEHOLD {
            Sector::Household
        } else {
            Sector::Other
        };
        Some((sector, country.get()))
    }

    /// The day's settled money read as firms' records: each firm's receipt of revenue its sale, and what paid it a
    /// purchase by a firm, a household or anyone else, a firm's purchase of plant its investment; each instruction
    /// sampled for its seller's country, early or late. And each firm's and household's income, from its effects and
    /// the interest and wages it earned or paid, counted in full.
    #[clause("STA.3")]
    fn sample_accounts(&mut self, kind: StaKind, date: Date) {
        let book = self.books.ledger.day_book();
        let bought = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(sys_cap::BOUGHT.name));
        let worn = self.books.dues.worn;
        let mut by: BTreeMap<phx_ledger::instruction::InstructionId, Vec<(PartyId, i64, bool)>> = BTreeMap::new();
        let mut income: BTreeMap<u8, i128> = BTreeMap::new();
        // A party's sector is read once a day, however many effects it has.
        let mut sectors: BTreeMap<PartyId, Option<(Sector, u8)>> = BTreeMap::new();
        let mut sector = |w: &World, party: PartyId| *sectors.entry(party).or_insert_with(|| w.sector_of(party));
        for e in &book.effects {
            let Some((sector, c)) = sector(self, e.party) else { continue };
            let earns =
                matches!(e.effect, phx_ledger::instruction::Effect::Revenue | phx_ledger::instruction::Effect::Expense);
            // Wear is capital consumed, which the income measure counts gross of.
            if sector != Sector::Other && earns && e.reason != worn {
                *income.entry(c).or_insert(0) += i128::from(e.amount.amt());
            }
            if !e.held {
                let investment = bought == Missing::Present(e.reason);
                by.entry(e.instruction).or_default().push((e.party, e.amount.amt(), investment));
            }
        }
        for (party, amount) in book.earned.sorted() {
            if let Some((sector, c)) = sector(self, party)
                && sector != Sector::Other
            {
                *income.entry(c).or_insert(0) += *amount;
            }
        }
        let mut sampled: Vec<(u8, Item, i128, bool)> = Vec::new();
        for (id, legs) in by {
            let sold: Vec<(PartyId, i64)> = legs
                .iter()
                .filter(|(p, a, _)| *a > 0 && matches!(sector(self, *p), Some((Sector::Firm, _))))
                .map(|(p, a, _)| (*p, *a))
                .collect();
            let Some((seller, _)) = sold.first().copied() else { continue };
            let Some((_, c)) = sector(self, seller) else { continue };
            let Some((law, _)) = self.sta_law(c) else { continue };
            let Some(schedule) = law.series.get(ACCOUNTS) else { continue };
            let subject = Subject::new(SubjectTag::Party, id.get());
            let place = u32::try_from(ACCOUNTS).unwrap_or(u32::MAX);
            if !within(
                phx_rand::open_unit(&mut self.streams.open_keyed_at(&kind.sample, subject, place)),
                schedule.sample,
            ) {
                continue;
            }
            let early =
                within(phx_rand::open_unit(&mut self.streams.open_keyed_at(&kind.returns, subject, place)), law.early);
            let revenue: i128 = sold.iter().map(|(_, a)| i128::from(*a)).sum();
            sampled.push((c, Item::Output, revenue, early));
            // A seller's own payment within its sale is a levy on it, never a purchase.
            let sellers: Vec<PartyId> = sold.iter().map(|(p, _)| *p).collect();
            for (payer, amount, investment) in legs.iter().filter(|(p, a, _)| *a < 0 && !sellers.contains(p)) {
                let item = match sector(self, *payer).map(|(s, _)| s) {
                    Some(Sector::Firm) if *investment => Item::Investment,
                    Some(Sector::Firm) => Item::Purchases,
                    Some(Sector::Household) => Item::Consumption,
                    _ => Item::Other,
                };
                sampled.push((c, item, -i128::from(*amount), early));
            }
        }
        let period = month_of(date);
        for (c, item, value, early) in sampled {
            let cell = self.collecting(c, period).accounts.entry(item.code()).or_default();
            if early {
                cell.early = (cell.early.0 + value, cell.early.1 + 1);
            }
            cell.all = (cell.all.0 + value, cell.all.1 + 1);
        }
        for (c, v) in income {
            self.collecting(c, period).income += v;
        }
    }

    /// Each country's firms' stocks of goods at their cost, as their books hold them.
    fn stocks_held(&self) -> Vec<i128> {
        let mut out = vec![0_i128; self.state.stats.laws.len()];
        let ledger = &self.books.ledger;
        for place in self.books.parties.places() {
            let holder = self.books.parties.holder(place);
            if holder.kind() != sys_frm::FIRM.name && holder.kind() != sys_frm::SMALL_FIRM.name {
                continue;
            }
            for slot in phx_store::table::live_in(holder.live_words()) {
                let party = holder.party(slot);
                let Missing::Present(country) = self.country_of_party(party) else { continue };
                let Some(total) = out.get_mut(usize::from(country.get())) else { continue };
                for (instrument, cost) in phx_ledger::holding::bases(holder, slot) {
                    if matches!(ledger.goods.key(instrument), Missing::Present(_)) {
                        *total += i128::from(cost);
                    }
                }
            }
        }
        out
    }

    /// The census at a month's end, or on the first close: each country's panel's persons by age class and health,
    /// the end of this period's exposure and the start of the next's, its labour force, and the money reported.
    fn census(&mut self, day: Day) {
        let date = self.calendar.date(day);
        let period = month_of(date);
        let countries = self.state.stats.laws.len();
        let mut persons: Vec<BTreeMap<(u8, u8), u64>> = vec![BTreeMap::new(); countries];
        let mut labour: Vec<[u64; LABOUR_STATES]> = vec![[0; LABOUR_STATES]; countries];
        if let Some((k, kd)) = self.population.kinds.iter().enumerate().find(|(_, k)| k.decl.kind == if_pop::HOUSEHOLD)
        {
            let kd = &kd.decl;
            let table =
                phx_pop::population::Population::table::<phx_store::SystemBacking>(self.books.parties.cells(), k);
            let employment: std::collections::BTreeSet<phx_id::LineId> = self.labour.lines.values().copied().collect();
            let mut h = Household { attrs: Vec::new(), persons: Vec::new(), positions: Vec::new() };
            for slot in table.slots() {
                let party = table.party(slot);
                let Missing::Present(country) = self.country_of_party(party) else { continue };
                let c = country.get();
                let twins = u64::from(table.multiplicity(slot).get());
                let life = self.panel(party, (c, LIFE_TABLE, period)).is_some();
                let force = self.panel(party, (c, LABOUR_FORCE, period)).is_some();
                if !life && !force {
                    continue;
                }
                phx_pop::explicit::household_into(kd, table, slot, &mut h);
                let employed: std::collections::BTreeSet<usize> = table
                    .attachments(slot)
                    .iter()
                    .map(|w| phx_pop::person::Attachment::unpack(*w))
                    .filter(|a| employment.contains(&a.line))
                    .filter_map(|a| match a.holder {
                        phx_pop::person::Holder::Person(i) => Some(i),
                        phx_pop::person::Holder::Household => None,
                    })
                    .collect();
                for (i, p) in h.present() {
                    if life
                        && let (Some(class), Some(health)) = (
                            class_of(&self.state.stats.classes, p.age_on(date)),
                            p.attr(if_pop::HEALTH.name).and_then(|x| u8::try_from(x).ok()),
                        )
                        && let Some(cell) = persons.get_mut(usize::from(c))
                    {
                        *cell.entry((class, health)).or_insert(0) += twins;
                    }
                    if force
                        && p.role != if_pop::CHILD.name
                        && let Some(l) = labour.get_mut(usize::from(c))
                    {
                        let state = if employed.contains(&i) {
                            0
                        } else if self
                            .labour
                            .kind
                            .is_some_and(|l| p.attr(l.state) == Some(if_labour::consts::SEARCHING))
                        {
                            1
                        } else {
                            2
                        };
                        if let Some(n) = l.get_mut(state) {
                            *n += twins;
                        }
                    }
                }
            }
        }
        let money = self.money_reported();
        let stocks = self.stocks_held();
        let day_no = day.get();
        for c in 0..countries {
            let Ok(id) = u8::try_from(c) else { continue };
            if self.sta_law(id).is_none() {
                continue;
            }
            let counted = persons.get(c).cloned().unwrap_or_default();
            let this = self.collecting(id, period);
            let held = stocks.get(c).copied().unwrap_or(0);
            if this.census_days.0 == 0 {
                this.start = counted.clone();
                this.stocks.0 = held;
                this.census_days.0 = day_no;
            } else {
                this.stocks.1 = held;
                this.end = counted.clone();
                this.census_days.1 = day_no;
                this.labour = labour.get(c).map(|l| l.to_vec()).unwrap_or_default();
                this.money = money.get(c).cloned().unwrap_or_default();
                let next = self.collecting(id, period + 1);
                next.start = counted;
                next.stocks.0 = held;
                next.census_days.0 = day_no;
            }
        }
    }

    /// The money each country's banks and central bank report at a month's end: the banks' reserves, and the deposits
    /// held by each class of holder by its legal form, the others last.
    fn money_reported(&self) -> Vec<Vec<i128>> {
        let countries = self.state.stats.laws.len();
        let classes = HOLDER_CLASSES.len() + 1;
        let mut out = vec![vec![0_i128; classes + 1]; countries];
        let ccys: Vec<phx_num::Ccy> = (0..countries)
            .filter_map(|c| u8::try_from(c).ok())
            .map(|c| phx_ledger::opening::currency(CountryId::new(c)))
            .collect();
        let lines = &self.books.ledger.lines;
        let first = self.books.parties.first_cell_place();
        let individual: Vec<&'static str> = self.books.parties.kinds().collect();
        for place in self.books.parties.places() {
            let name = match place.checked_sub(first) {
                None => individual.get(usize::from(place)).copied(),
                Some(k) => self.population.kinds.get(usize::from(k)).map(|kd| kd.decl.kind),
            };
            let Some(name) = name else { continue };
            let class = match self.accounts.form_of(name) {
                Missing::Present(form) => {
                    HOLDER_CLASSES.iter().position(|f| *f == form).unwrap_or(HOLDER_CLASSES.len())
                }
                Missing::Absent => HOLDER_CLASSES.len(),
            };
            let holder = self.books.parties.holder(place);
            for slot in phx_store::table::live_in(holder.live_words()) {
                for r in phx_ledger::rows::iter(holder, slot) {
                    if r.side() != Side::Asset {
                        continue;
                    }
                    let (deposit, reserves) = (lines.is_deposit(r.row.line), lines.is_reserves(r.row.line));
                    if !deposit && !reserves {
                        continue;
                    }
                    let Missing::Present(balance) = r.optional.balance else { continue };
                    let ccy = self.books.ledger.terms.get(lines.terms(r.row.line)).ccy;
                    let Some(c) = ccys.iter().position(|x| *x == ccy) else { continue };
                    let Some(row) = out.get_mut(c) else { continue };
                    let at = if reserves { 0 } else { class + 1 };
                    if let Some(v) = row.get_mut(at) {
                        *v += i128::from(balance);
                    }
                }
            }
        }
        out
    }

    /// The day a period's release of a vintage falls on in a country: the business day of its month the calendar
    /// names, in the month the lag and, for a revision, the revision's months after it put it in.
    pub(crate) fn release_day(&self, country: u8, series: usize, (period, vintage): (u32, u8)) -> Option<Day> {
        let (law, _) = self.sta_law(country)?;
        let s = law.series.get(series)?;
        let month = period + 1 + s.lag + if vintage == 0 { 0 } else { law.revision };
        let mut day = self.calendar.day(first_of(month))?;
        let mut left = s.day;
        loop {
            if self.calendar.is_business(CountryId::new(country), day) {
                left = left.checked_sub(1)?;
                if left == 0 {
                    return Some(day);
                }
            }
            day = day.succ();
            if month_of(self.calendar.date(day)) != month {
                return None;
            }
        }
    }

    /// Each release due today published: a period's first release from its early returns and its revision from every
    /// return, each index chained on the level last published for the period before.
    fn publish(&mut self, day: Day) {
        let periods: Vec<(u8, u32)> = self.state.book.stats.collected.iter().map(|c| (c.country, c.period)).collect();
        for (country, period) in periods {
            for series in 0..SERIES {
                for vintage in [0_u8, 1] {
                    if self.release_day(country, series, (period, vintage)) != Some(day) {
                        continue;
                    }
                    if let Some(values) = self.estimate(country, series, (period, vintage)) {
                        let Ok(s) = u8::try_from(series) else { continue };
                        self.state.book.stats.releases.push(Release {
                            series: s,
                            country,
                            period,
                            vintage,
                            published: day,
                            values,
                        });
                        self.state.day.stats.published += 1;
                    }
                }
            }
        }
        // A period is let go once revised and once the next period's revision no longer chains on it.
        let latest: Vec<(u8, u32)> = self.state.book.stats.collected.iter().map(|c| (c.country, c.period)).collect();
        let kept: Vec<bool> = latest
            .iter()
            .map(|(c, p)| (0..SERIES).any(|s| self.release_day(*c, s, (p + 1, 1)).is_none_or(|d| d >= day)))
            .collect();
        let mut keep = kept.into_iter();
        self.state.book.stats.collected.retain(|_| keep.next().unwrap_or(true));
    }

    /// A release's values, or none where the period's records give none.
    fn estimate(&self, country: u8, series: usize, (period, vintage): (u32, u8)) -> Option<Vec<i64>> {
        let (law, base) = self.sta_law(country)?;
        let share = law.series.get(series)?.sample;
        let c = self.state.book.stats.collected.iter().find(|x| (x.country, x.period) == (country, period))?;
        let places = scale(series)?;
        let whole = |x: f64| phx_rand::float::floor_to_i64(f64::round(x * places));
        match series {
            CPI | PPI => {
                let level = self.level(country, series, (period, vintage), *base)?;
                Some(vec![whole(level)?])
            }
            LABOUR_FORCE => {
                if c.census_days.1 == 0 {
                    return None;
                }
                c.labour.iter().map(|n| whole(from_u64(*n) / share)).collect()
            }
            MONEY => {
                if c.money.is_empty() {
                    return None;
                }
                c.money.iter().map(|v| i64::try_from(*v).ok()).collect()
            }
            LIFE_TABLE => self.life_table(c, (share, vintage != 0)),
            ACCOUNTS => {
                if c.census_days.1 == 0 {
                    return None;
                }
                let of = |item: Item| {
                    c.accounts
                        .get(&item.code())
                        .map_or(0.0, |x| from_i128(if vintage == 0 { x.early.0 } else { x.all.0 }))
                };
                let returned = if vintage == 0 { share * law.early } else { share };
                let gross = |item: Item| of(item) / returned;
                let stocked = from_i128(c.stocks.1 - c.stocks.0);
                let production = gross(Item::Output) - gross(Item::Purchases) + stocked;
                let expenditure = gross(Item::Consumption) + gross(Item::Investment) + gross(Item::Other) + stocked;
                let income = from_i128(c.income);
                [production, expenditure, income, expenditure - income].into_iter().map(whole).collect()
            }
            _ => None,
        }
    }

    /// An index's level for a period: the base where no period before it was priced, else the level published for
    /// the period before chained by this period's link.
    fn level(&self, country: u8, series: usize, (period, vintage): (u32, u8), base: f64) -> Option<f64> {
        let index = self.state.stats.index?;
        let s = u8::try_from(series).ok()?;
        let prices = |p: u32, all: bool| -> Option<BTreeMap<u16, (f64, f64)>> {
            let c = self.state.book.stats.collected.iter().find(|x| (x.country, x.period) == (country, p))?;
            let out: BTreeMap<u16, (f64, f64)> = c
                .prices
                .iter()
                .filter(|((x, _), _)| *x == s)
                .filter_map(|((_, product), t)| {
                    let (value, qty) = if all { t.all } else { t.early };
                    (qty > 0).then(|| (*product, (from_i128(value) / from_i128(qty), from_i128(value))))
                })
                .collect();
            (!out.is_empty()).then_some(out)
        };
        let now = prices(period, vintage != 0)?;
        let before = period.checked_sub(1).and_then(|p| prices(p, true));
        let Some(before) = before else { return Some(base) };
        let spent: f64 = before.values().map(|(_, v)| v).sum();
        let constituents: Vec<Priced> = before
            .iter()
            .map(|(p, (price, value))| Priced {
                before: *price,
                now: now.get(p).map_or(0.0, |(x, _)| *x),
                weight: value / spent,
            })
            .collect();
        let link = (index.link)(&constituents)?;
        let latest = self.state.stats.kind?.latest;
        let last = latest(&self.state.book.stats.releases, (s, country), Day::new(u32::MAX))
            .filter(|r| r.period + 1 == period)?;
        Some(from_i64(*last.values.first()?) / scale(series)? * link)
    }

    /// The period life table: for each age class and health exposed in the period, the deaths, the person-days
    /// exposed and the rate a year; and for the able, the onsets with the same. Each estimate is the panel's count
    /// over its share and, for a first release, over the share of returns in. None before the period's last census.
    fn life_table(&self, c: &Collected, (share, all): (f64, bool)) -> Option<Vec<i64>> {
        let kind = self.state.stats.kind?;
        let (from, to) = c.census_days;
        if to == 0 {
            return None;
        }
        let days = from_u64(u64::from(to.checked_sub(from)?) + 1);
        let returned = if all { 1.0 } else { self.sta_law(c.country)?.0.early };
        let places = scale(LIFE_TABLE)?;
        let count = |x: f64| phx_rand::float::floor_to_i64(f64::round(x));
        let mut out = Vec::new();
        for class in 0..self.state.stats.classes.len() {
            let Ok(k) = u8::try_from(class) else { continue };
            for health in [if_pop::ABLE, if_pop::DISABLED] {
                let Ok(h) = u8::try_from(health) else { continue };
                let exposed = |m: &BTreeMap<(u8, u8), u64>| from_u64(m.get(&(k, h)).copied().unwrap_or(0));
                let exposure = f64::midpoint(exposed(&c.start), exposed(&c.end)) * days / share;
                for onset in [0_u8, 1] {
                    if onset == 1 && health != if_pop::ABLE {
                        continue;
                    }
                    let (early, every) = c.vital.get(&(k, h, onset)).copied().unwrap_or_default();
                    let events = from_u64(if all { every } else { early }) / share / returned;
                    // The rate is the one its published events and exposure give, so a reader can trace it; a cell
                    // nobody was exposed in has none, and is not published.
                    let (events, exposure) = (count(events)?, count(exposure)?);
                    let Some(rate) = (kind.rate)(from_i64(events), from_i64(exposure)) else { continue };
                    let rate = phx_rand::float::floor_to_i64(f64::round(rate * places))?;
                    out.extend([i64::from(k), i64::from(h), i64::from(onset), events, exposure, rate]);
                }
            }
        }
        Some(out)
    }
}
