//! Firms on the core, of one kind: each country's firms making each product in each region, counted from the persons
//! it employs and the firms each person employed makes, each drawing its productivity and its site, and posting its
//! day-zero price from its own cost. Nothing about a firm's size is drawn: its output is its product's in the accounts
//! shared by the demand its price wins, and its staff, plant and stocks follow from that output.

use phx_core::store::Opening;
use phx_core::{OpeningCountry, Register, StreamDecl, WorldStreams, opening_subject};
use phx_id::TileId;
use phx_macros::{clause, opening};
use phx_num::round::{Round, split_total};
use phx_num::violation;
use phx_rand::float::{floor_to_i64, from_i64, from_u64, len_u64};
use phx_rand::{below_u64, normal};

use crate::consts::firm::{
    COMPENSATION, MANAGEMENT_PURPOSE, PRODUCTIVITY_ONE, PRODUCTIVITY_PURPOSE, PUBLIC_ADMINISTRATION, PURPOSES,
    SITE_PURPOSE,
};
use crate::core::Core;
use crate::firm_store::{FirmOpening, FirmStore};
use crate::opening::economy::table;

/// A country's firms of one product in one region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub product: u32,
    pub region: u32,
    pub firms: u64,
}

/// A register table's first row, a table of one row, or a table by country's row read whole.
fn row(register: &Register, id: &str, c: &OpeningCountry) -> Result<Vec<f64>, String> {
    table(register, id, c.id)?.0.into_iter().next().ok_or_else(|| format!("`{id}` holds no row"))
}

/// A country's products at the opening, a unit's each: its price, what its way's inputs cost at those prices and what
/// its labour costs at the accounts' split of what it adds; and the units a year the accounts give its output.
#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub price: Vec<f64>,
    pub materials: Vec<f64>,
    pub labour: Vec<f64>,
    pub units: Vec<f64>,
}

impl Snapshot {
    /// A unit's price at a productivity: the product's markup over its cost in the accounts, over the firm's own cost,
    /// its labour's hours fewer by its productivity's factor.
    #[clause("FRM.5", "FRM.14", "GEN.13")]
    #[must_use]
    pub fn price_at(&self, product: usize, productivity: f64) -> f64 {
        let at = |v: &[f64]| v.get(product).copied().unwrap_or(f64::NAN);
        sys_frm::rules::price::day_zero(at(&self.price), at(&self.materials), at(&self.labour), productivity)
    }
}

/// A country's products at the opening, read from its accounts, its ways and its prices.
///
/// # Errors
/// A primitive the snapshot reads that the register does not hold, or a product the accounts leave unpriced or
/// costing nothing.
#[clause("GEN.2", "GEN.5", "TEC.1")]
pub fn snapshot(register: &Register, c: &OpeningCountry) -> Result<Snapshot, String> {
    let (accounts, _) = crate::opening::economy::accounts(register, c.id)?;
    let output = accounts.output;
    let price: Vec<f64> = row(register, "GDS.opening_price", c)?
        .iter()
        .zip(row(register, "GDS.price_level", c)?)
        .map(|(a, b)| a * b)
        .collect();
    let (inputs, _) = table(register, "TEC.inputs", c.id)?;
    let added = accounts.added;
    let (mut materials, mut labour, mut units) = (Vec::new(), Vec::new(), Vec::new());
    for (p, unit_price) in price.iter().enumerate() {
        let share = output.get(p).copied().unwrap_or(f64::NAN);
        let m: f64 = inputs.iter().zip(&price).map(|(row, q)| row.get(p).copied().unwrap_or(f64::NAN) * q).sum();
        let wages = added.get(p).and_then(|r| r.get(COMPENSATION)).copied().unwrap_or(f64::NAN);
        let l = wages / share * unit_price;
        let u = share * c.gdp / unit_price;
        if !(m.is_finite() && l.is_finite() && u.is_finite() && m + l > 0.0) {
            return Err(format!("country {}: product {p} unpriced or costing nothing in the accounts", c.id.get()));
        }
        materials.push(m);
        labour.push(l);
        units.push(u);
    }
    Ok(Snapshot { price, materials, labour, units })
}

/// `total` split over `weights` exactly, each part in proportion to its weight.
fn apportion(total: u64, weights: &[u64]) -> Vec<u64> {
    let mut whole: u64 = weights.iter().sum();
    let Ok(mut left) = i64::try_from(total) else {
        violation!(clause = "REP.9", "a count beyond an amount", total = total);
    };
    weights
        .iter()
        .map(|w| {
            if whole == 0 {
                return 0;
            }
            let (part, rest) = split_total(left, *w, whole, Round::HalfEven);
            (left, whole) = (rest, whole - w);
            u64::try_from(part).unwrap_or_else(|_| violation!(clause = "REP.9", "a share below nothing", part = part))
        })
        .collect()
}

/// An amount split over weights exactly, each part in proportion to its weight; nothing where no weight is.
pub(crate) fn apportion_amount(total: i64, weights: &[u64]) -> Vec<i64> {
    let (mut left, mut whole): (i64, u64) = (total, weights.iter().sum());
    weights
        .iter()
        .map(|w| {
            if whole == 0 {
                return 0;
            }
            let (part, rest) = split_total(left, *w, whole, Round::HalfEven);
            (left, whole) = (rest, whole - w);
            part
        })
        .collect()
}

/// The country's firms by product and region: the persons each product employs — the country's employed shared by
/// the hours its output asks — times the firms a person employed making it makes, and each product's firms over the
/// regions by their persons.
///
/// # Errors
/// A primitive the count reads that the register does not hold.
#[clause("GEN.2", "REP.40", "FRM.23")]
pub fn cells(register: &Register, c: &OpeningCountry, persons_by_region: &[(u32, u64)]) -> Result<Vec<Cell>, String> {
    let asked = crate::opening::asked::Asked::of(register, c)?;
    let density = row(register, "FRM.firms_per_employed", c)?;
    let regions: Vec<u64> = persons_by_region.iter().map(|(_, n)| *n).collect();
    let mut out = Vec::new();
    // Public administration's persons are the agency's, which is no firm.
    for (product, employed) in
        asked.persons(phx_ledger::opening::employed(c)).into_iter().filter(|(a, _)| *a != PUBLIC_ADMINISTRATION)
    {
        let Some(per) = density.get(product) else {
            return Err(format!("country {}: product {product} with no firms per person employed", c.id.get()));
        };
        let Some(n) = floor_to_i64((employed * per).round()).and_then(|f| u64::try_from(f).ok()) else {
            return Err(format!("country {}: product {product}'s firms beyond counting", c.id.get()));
        };
        let Ok(product) = u32::try_from(product) else {
            return Err(format!("country {}: products beyond counting", c.id.get()));
        };
        for ((region, _), firms) in persons_by_region.iter().zip(apportion(n, &regions)) {
            if firms > 0 {
                out.push(Cell { product, region: *region, firms });
            }
        }
    }
    Ok(out)
}

/// The bank a draw below the banks' total weight falls on.
fn pick(banks: &[(u32, u64)], draw: u64) -> u32 {
    let mut below = 0_u64;
    for (bank, w) in banks {
        below += w;
        if draw < below {
            return *bank;
        }
    }
    violation!(clause = "BNK.1", "a draw beyond the banks' weights", draw = draw)
}

/// A firm drawn at the opening before it is begun: its cell, site, productivity, bank, posted price and output.
#[derive(Clone, Copy, Debug)]
struct Draft {
    product: u32,
    region: u32,
    site: TileId,
    productivity: i64,
    bank: u32,
    price: i64,
    output: f64,
    /// Its preferences drawn at its founding, which its offices read while no one holds them.
    prefs: phx_core::Prefs,
}

/// What the firms' opening reads besides the core: the register, the countries with their sheets, the stream its
/// draws come from, and the management whose price points a firm posts on.
#[derive(Debug)]
pub struct FirmsOpening<'a> {
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub sheets: &'a [crate::opening::sheet::Sheet],
    pub streams: &'a WorldStreams,
    pub stream: &'a StreamDecl,
    pub management: &'a sys_frm::decide::Management,
    pub today: phx_id::Day,
    /// The map, whose zones the firms' sites lie in.
    pub geo: &'a phx_geo::GeoState,
}

/// The firms' mean hours a unit, as a factor of their way's, at a shift of their log productivities: each firm's
/// factor weighed by the chance the logit gives its price in its cell, each cell by its share of the product's units.
fn mean_hours(cells: &[(f64, Vec<f64>)], price_at: &dyn Fn(f64) -> f64, weight: f64, shift: f64) -> f64 {
    let mut total = 0.0;
    for (share, logs) in cells {
        let prices: Vec<f64> = logs.iter().map(|l| price_at(l + shift)).collect();
        let chances = phx_market::retail::chances_at(&prices, weight);
        let hours: f64 =
            chances.iter().zip(logs).map(|(c, l)| c * sys_frm::rules::way::own_hours(1.0, l + shift)).sum();
        total += share * hours;
    }
    total
}

/// The shift of a product's firms' log productivities at which their mean hours a unit, weighed by the output their
/// prices win, are their way's — the way's hours being the average firm's, as the accounts measure them: a bracket
/// widened by doubling, then halved until its halves are the same number.
#[clause("GEN.4", "FRM.2", "TEC.1")]
#[must_use]
pub fn centring(cells: &[(f64, Vec<f64>)], price_at: &dyn Fn(f64) -> f64, weight: f64) -> f64 {
    let at = |shift: f64| mean_hours(cells, price_at, weight, shift);
    let (mut lo, mut hi) = (-1.0, 1.0);
    while at(lo) < 1.0 {
        lo *= 2.0;
    }
    while at(hi) > 1.0 {
        hi *= 2.0;
    }
    loop {
        let mid = f64::midpoint(lo, hi);
        if !(mid > lo && mid < hi) {
            return mid;
        }
        if at(mid) > 1.0 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
}

/// Each cell's output a year shared over its firms by the logit's chance its price wins: the product's units in the
/// accounts over the regions its firms are in by their persons, and a region's over its firms by their prices.
#[clause("GEN.2", "FRM.2", "SRV.4")]
fn share_output(drafts: &mut [Draft], snap: &Snapshot, persons: &[(u32, u64)], price_weight: f64) {
    let persons_of = |r: u32| persons.iter().find(|(x, _)| *x == r).map_or(0.0, |(_, n)| from_u64(*n));
    let mut start = 0;
    while start < drafts.len() {
        let product = drafts.get(start).map_or(u32::MAX, |d| d.product);
        let end = start + drafts.iter().skip(start).take_while(|d| d.product == product).count();
        let Some(of_product) = drafts.get_mut(start..end) else { break };
        let mut regions: Vec<u32> = of_product.iter().map(|d| d.region).collect();
        regions.dedup();
        let persons_in: f64 = regions.iter().map(|r| persons_of(*r)).sum();
        let units = snap.units.get(phx_rand::float::index(u64::from(product))).copied().unwrap_or(f64::NAN);
        for region in regions {
            let cell: Vec<&mut Draft> = of_product.iter_mut().filter(|d| d.region == region).collect();
            let prices: Vec<i64> = cell.iter().map(|d| d.price).collect();
            let of_cell = units * persons_of(region) / persons_in;
            for (d, chance) in cell.into_iter().zip(phx_market::retail::price_chances(&prices, price_weight)) {
                d.output = of_cell * chance;
            }
        }
        start = end;
    }
}

impl Core {
    /// The firms drawn on the core, of one kind: every country's firms begun by product and region, each with its
    /// productivity drawn from its group's spread around one, its site drawn among its region's land, its management's
    /// memory type drawn by the types' shares, an account at a
    /// bank drawn by the banks' deposits, its day-zero price posted from its own cost and its output the demand that
    /// price wins; each country's firms' deposits shared over them by their turnover.
    ///
    /// # Errors
    /// A primitive the opening reads that the register does not hold, or a product the accounts leave unpriced.
    #[clause("GEN.2", "GEN.13", "FRM.23", "FRM.2", "PTY.5")]
    #[opening]
    pub fn open_firms(&mut self, o: &FirmsOpening<'_>) -> Result<(), String> {
        let Some(firm) = self.bound.kinds.firm else { return Ok(()) };
        let price_weight = o.register.fixed("SRV.price_weight")?;
        let kind = crate::core::kind_number(firm);
        let held = (crate::household_store::zone_regions(o.geo), o.management.points.clone(), o.today);
        let capacity = self.kind_rows(firm);
        self.firms = Some(FirmStore::new(&mut self.space, kind, capacity, held));
        for (c, sheet) in o.countries.iter().zip(o.sheets) {
            let persons = self.persons_by_region(c);
            let snap = snapshot(o.register, c)?;
            let mut drafts = self.drafts(o, c, &snap, &persons)?;
            share_output(&mut drafts, &snap, &persons, price_weight);
            let deposits = phx_ledger::opening::whole(
                sheet.at(crate::consts::sheet::DEPOSITS, crate::consts::sheet::FIRMS) * c.gdp,
            );
            let mut turnover = Vec::with_capacity(drafts.len());
            for d in &drafts {
                let lot = sys_frm::FilingPrims::lot(o.register, u16::try_from(d.product).unwrap_or(u16::MAX));
                let Some(t) =
                    floor_to_i64((from_i64(d.price) / lot * d.output).round()).and_then(|t| u64::try_from(t).ok())
                else {
                    return Err(format!("country {}: a firm's turnover beyond an amount", c.id.get()));
                };
                turnover.push(t);
            }
            let shares = self.apportion(("firms' deposits", c.id.get()), deposits, &turnover);
            for (d, share) in drafts.iter().zip(shares) {
                let Some(output) = floor_to_i64(d.output.round()) else {
                    return Err(format!("country {}: a firm's output beyond a count", c.id.get()));
                };
                let phx_num::Missing::Present(zone) = o.geo.zone_of(d.site) else {
                    return Err(format!("country {}: a firm sited on no zone's land", c.id.get()));
                };
                let (Ok(product), Ok(zone)) = (u16::try_from(d.product), u16::try_from(zone.get())) else {
                    return Err(format!("country {}: a firm's product or zone beyond its word", c.id.get()));
                };
                let key = self.begin_party(firm, None, Some(Opening { bank: d.bank, balance: share }));
                let Some(r) = self.reference(key) else {
                    violation!(clause = "PTY.1", "a firm begun the directory does not name", slot = key.slot().get());
                };
                let opening = FirmOpening {
                    product,
                    site: d.site.get(),
                    zone,
                    productivity: from_i64(d.productivity) / PRODUCTIVITY_ONE,
                    price: d.price,
                    output: from_i64(output),
                    markup: phx_num::Missing::Absent,
                    expected: phx_num::Missing::Present(from_i64(output) / crate::consts::DAYS_A_YEAR),
                    opened: o.today,
                };
                if let Some(fs) = self.firms.as_mut() {
                    fs.begin(&self.directory, r, &opening);
                }
                self.found(key, d.prefs);
            }
        }
        Ok(())
    }

    /// A country's firms drawn, cell by cell in product order: each one's productivity and site from its own subjects,
    /// its site among its region's own tiles, a region with no land stopping the run, its bank by the banks' deposits,
    /// and its day-zero price posted at the point nearest its own cost's.
    #[clause("FRM.2", "FRM.5", "GEN.13", "PTY.5", "REP.34", "GEO.11")]
    #[opening]
    fn drafts(
        &self,
        o: &FirmsOpening<'_>,
        c: &OpeningCountry,
        snap: &Snapshot,
        persons: &[(u32, u64)],
    ) -> Result<Vec<Draft>, String> {
        let cells = cells(o.register, c, persons)?;
        let spread = o.register.fixed_in("GEN.productivity_spread", c.id)?;
        let banks = self.banks_of.get(usize::from(c.id.get())).cloned().unwrap_or_default();
        let bank_weight: u64 = banks.iter().map(|(_, w)| *w).sum();
        if bank_weight == 0 {
            violation!(clause = "BNK.1", "a country's firms with no bank to hold their deposits", country = c.id.get());
        }
        let (mut out, mut logs) = (Vec::new(), Vec::new());
        let mut ordinal = 0_u32;
        for cell in &cells {
            let Some((_, tiles)) = c.regions.iter().find(|(r, _)| *r == cell.region) else { continue };
            for _ in 0..cell.firms {
                let subject = |purpose: u32| opening_subject(u32::from(c.id.get()) * PURPOSES + purpose, ordinal);
                let mut d = o.streams.open(o.stream, subject(PRODUCTIVITY_PURPOSE), phx_id::Day::new(0), 0);
                logs.push(normal(&mut d) * spread);
                let mut at = o.streams.open(o.stream, subject(SITE_PURPOSE), phx_id::Day::new(0), 0);
                let site: TileId = match tiles.get(phx_rand::float::index(below_u64(&mut at, len_u64(tiles.len())))) {
                    Some(t) => *t,
                    None => violation!(clause = "PTY.5", "a firm's region with no land", region = cell.region),
                };
                let bank = pick(&banks, below_u64(&mut at, bank_weight));
                let mut m = o.streams.open(o.stream, subject(MANAGEMENT_PURPOSE), phx_id::Day::new(0), 0);
                let memory = phx_core::register::values::draw_type(&o.management.types.memory, &mut m).get();
                let switching = phx_core::register::values::draw_type(&o.management.types.switching, &mut m).get();
                // With no heuristic scored yet nothing favours one, so its first stance is its taste's alone.
                let menu = len_u64(phx_val::heuristic::MENU.len());
                let stance = u16::try_from(below_u64(&mut m, menu)).unwrap_or(u16::MAX);
                let required = o.management.required_return.draw(&mut m);
                let prefs = phx_core::Prefs {
                    memory: phx_num::Missing::Present(memory),
                    switching: phx_num::Missing::Present(switching),
                    stance: phx_num::Missing::Present(stance),
                    required_return: phx_num::Missing::Present(required),
                    management: phx_num::Missing::Present(0),
                    window: phx_num::Missing::Absent,
                };
                out.push(Draft {
                    product: cell.product,
                    region: cell.region,
                    site,
                    productivity: 0,
                    bank,
                    price: 0,
                    output: 0.0,
                    prefs,
                });
                ordinal += 1;
            }
        }
        self.price_drafts(o, (c, snap, persons), &mut out, &logs)?;
        Ok(out)
    }

    /// Each product's drafts centred and priced: their log productivities shifted so that their mean hours a unit,
    /// weighed by the output their prices win, are the way's, and each one's day-zero price posted at the point
    /// nearest its own cost's.
    #[clause("FRM.2", "FRM.5", "GEN.13", "GEN.4", "REP.34")]
    #[opening]
    fn price_drafts(
        &self,
        o: &FirmsOpening<'_>,
        (c, snap, persons): (&OpeningCountry, &Snapshot, &[(u32, u64)]),
        drafts: &mut [Draft],
        logs: &[f64],
    ) -> Result<(), String> {
        let weight = o.register.fixed("SRV.price_weight")?;
        let pricing = self.bind(&sys_frm::points::DAY_ZERO_PRICE);
        let persons_of = |r: u32| persons.iter().find(|(x, _)| *x == r).map_or(0.0, |(_, n)| from_u64(*n));
        let mut start = 0;
        while start < drafts.len() {
            let product = drafts.get(start).map_or(u32::MAX, |d| d.product);
            let end = start + drafts.iter().skip(start).take_while(|d| d.product == product).count();
            let (Some(of_product), Some(of_logs)) = (drafts.get_mut(start..end), logs.get(start..end)) else { break };
            let p = phx_rand::float::index(u64::from(product));
            let mut regions: Vec<u32> = of_product.iter().map(|d| d.region).collect();
            regions.dedup();
            let persons_in: f64 = regions.iter().map(|r| persons_of(*r)).sum();
            let cells: Vec<(f64, Vec<f64>)> = regions
                .iter()
                .map(|r| {
                    let in_cell = of_product.iter().zip(of_logs).filter(|(d, _)| d.region == *r).map(|(_, l)| *l);
                    (persons_of(*r) / persons_in, in_cell.collect())
                })
                .collect();
            let shift = centring(&cells, &|log| snap.price_at(p, log), weight);
            let lot = sys_frm::FilingPrims::lot(o.register, u16::try_from(product).unwrap_or(u16::MAX));
            for (d, log) in of_product.iter_mut().zip(of_logs) {
                let log = log + shift;
                d.productivity = floor_to_i64((log * PRODUCTIVITY_ONE).round())
                    .unwrap_or_else(|| violation!(clause = "FRM.2", "a productivity beyond a word"));
                let wanted = lot * snap.price_at(p, log);
                let price = self.decide_founding(pricing, &d.prefs, |_| sys_frm::rules::review::DayZeroIn {
                    points: o.management.points_near(wanted),
                    wanted,
                });
                let Some(price) = price else {
                    return Err(format!("country {}: product {product} priced at no point", c.id.get()));
                };
                d.price = price;
            }
            start = end;
        }
        Ok(())
    }

    /// The persons the core's households of a country hold, by region, in the country's regions' order.
    fn persons_by_region(&self, c: &OpeningCountry) -> Vec<(u32, u64)> {
        let mut out: Vec<(u32, u64)> = c.regions.iter().map(|(r, _)| (*r, 0)).collect();
        let Some(place) = self.bound.kinds.household else { return out };
        for slot in self.directory.live_slots(crate::core::kind_number(place)) {
            let Some(r) = self.household_region(slot) else { continue };
            if let Some((_, n)) = out.iter_mut().find(|(x, _)| *x == r) {
                *n += len_u64(self.persons_in(phx_id::PartyKey::new(crate::core::kind_number(place), slot)));
            }
        }
        out
    }
}

#[path = "core_firms_tests.rs"]
mod tests;
