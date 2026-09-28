//! Firms on the core, of one kind: each country's firms making each product in each region, counted from the persons
//! the product's output employs by its way and the firms each person employed makes, each drawing its productivity
//! and its site. Nothing about a firm's size is drawn: its output, staff, plant and stocks follow from the demand its
//! price wins.

use phx_core::store::{KindStore, Opening};
use phx_core::{OpeningCountry, Register, StreamDecl, Streams, opening_subject};
use phx_id::{PartyId, PartyKey, TileId};
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{MaybeI64, violation};
use phx_rand::float::{floor_to_i64, from_u64, len_u64};
use phx_rand::{below_u64, normal};
use phx_store::SystemBacking;

use crate::consts::firm::{PRODUCTIVITY_ONE, PRODUCTIVITY_PURPOSE, PURPOSES, RECORD, SITE_PURPOSE};
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, WEEKS_A_YEAR};
use crate::core::Core;
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

/// The persons employed making each product a year: its output in units — its share of GDP at its opening price and
/// price level — times the hours its way asks of every occupation a unit, over a full-time year.
///
/// # Errors
/// A primitive the count reads that the register does not hold.
#[clause("GEN.2", "TEC.1")]
pub fn employed_by_product(register: &Register, c: &OpeningCountry) -> Result<Vec<f64>, String> {
    let output = row(register, "GEN.output", c)?;
    let price: Vec<f64> = row(register, "GDS.opening_price", c)?
        .iter()
        .zip(row(register, "GDS.price_level", c)?)
        .map(|(a, b)| a * b)
        .collect();
    let (labour, _) = table(register, "TEC.labour", c.id)?;
    let full_time = register.count_in("LAB.full_time_hours", c.id).map(|h| from_u64(h) * WEEKS_A_YEAR)?;
    Ok(price
        .iter()
        .enumerate()
        .map(|(p, unit_price)| {
            let units = output.get(p).copied().unwrap_or(f64::NAN) * c.gdp / unit_price;
            let hours: f64 = labour.iter().map(|occ| occ.get(p).copied().unwrap_or(f64::NAN)).sum::<f64>() * units;
            hours / full_time
        })
        .collect())
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

/// The country's firms by product and region: the persons it employs — its people from 15 at its employment rate —
/// times the firms a person employed makes, shared over the products by the persons each product's output employs by
/// its way, and each product's firms over the regions by their persons.
///
/// # Errors
/// A primitive the count reads that the register does not hold.
#[clause("GEN.2", "REP.40", "FRM.23")]
pub fn cells(register: &Register, c: &OpeningCountry, persons_by_region: &[(u32, u64)]) -> Result<Vec<Cell>, String> {
    let by_way = employed_by_product(register, c)?;
    let density = register.fixed_in("FRM.firms_per_employed", c.id)?;
    let firms = phx_ledger::opening::employed(c) * density;
    let Some(total) = floor_to_i64(firms.round()).and_then(|f| u64::try_from(f).ok()) else {
        return Err(format!("country {}: firms beyond counting", c.id.get()));
    };
    let mut weights = Vec::with_capacity(by_way.len());
    for (p, e) in by_way.iter().enumerate() {
        let Some(w) = floor_to_i64(e.round()).and_then(|v| u64::try_from(v).ok()) else {
            return Err(format!("country {}: product {p} employs no count of persons", c.id.get()));
        };
        weights.push(w);
    }
    let regions: Vec<u64> = persons_by_region.iter().map(|(_, n)| *n).collect();
    let mut out = Vec::new();
    for (product, n) in apportion(total, &weights).into_iter().enumerate() {
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

impl Core {
    /// The firms drawn on the core, of one kind: the mirrored firms of both of the books' tables let go, and every
    /// country's firms begun by product and region, each with its productivity drawn from its group's spread around
    /// one, its site drawn among its region's land, and an account at a bank drawn by the banks' deposits; each
    /// country's firms' deposits split evenly over them until their output is known.
    ///
    /// # Errors
    /// A primitive the count reads that the register does not hold.
    #[clause("GEN.2", "FRM.23", "FRM.2", "PTY.5")]
    pub fn open_firms(
        &mut self,
        register: &Register,
        (countries, sheets): (&[OpeningCountry], &[crate::opening::sheet::Sheet]),
        (streams, stream): (&Streams, &StreamDecl),
    ) -> Result<(), String> {
        let Some(firm) = self.names.iter().position(|n| *n == "firm") else { return Ok(()) };
        for name in ["firm", "small_firm"] {
            let Some(k) = self.names.iter().position(|n| *n == name) else { continue };
            self.keys.retain(|(_, key)| usize::from(key.kind()) != k);
            if let Some(store) = self.kinds.get_mut(k) {
                let live: Vec<_> = store.parties.live_slots().filter_map(|s| store.parties.at(s)).collect();
                for r in live {
                    store.parties.end(r);
                }
                store.parties.close_day();
            }
        }
        let mut store: KindStore<SystemBacking> =
            KindStore::new(&mut self.space, crate::core::kind_number(firm), AGENT_ROWS, AGENT_ROWS_PER_CHUNK, RECORD)
                .with_accounts(&mut self.space, AGENT_ROWS, AGENT_ROWS_PER_CHUNK);
        for (c, sheet) in countries.iter().zip(sheets) {
            let persons = self.persons_by_region(c);
            let cells = cells(register, c, &persons)?;
            let spread = register.fixed_in("GEN.productivity_spread", c.id)?;
            let deposits = phx_ledger::opening::whole(
                sheet.at(crate::consts::sheet::DEPOSITS, crate::consts::sheet::FIRMS) * c.gdp,
            );
            let count: u64 = cells.iter().map(|x| x.firms).sum();
            let banks = self.banks_of.get(usize::from(c.id.get())).cloned().unwrap_or_default();
            let bank_weight: u64 = banks.iter().map(|(_, w)| *w).sum();
            if bank_weight == 0 {
                violation!(
                    clause = "BNK.1",
                    "a country's firms with no bank to hold their deposits",
                    country = c.id.get()
                );
            }
            let (mut left, mut whole) = (deposits, count);
            let mut ordinal = 0_u32;
            for cell in &cells {
                let Some((_, tiles)) = c.regions.iter().find(|(r, _)| *r == cell.region) else { continue };
                for _ in 0..cell.firms {
                    let subject = |purpose: u32| opening_subject(u32::from(c.id.get()) * PURPOSES + purpose, ordinal);
                    let mut d = streams.open(stream, subject(PRODUCTIVITY_PURPOSE), phx_id::Day::new(0), 0);
                    let productivity = floor_to_i64((normal(&mut d) * spread * PRODUCTIVITY_ONE).round())
                        .unwrap_or_else(|| violation!(clause = "FRM.2", "a productivity beyond a word"));
                    let mut at = streams.open(stream, subject(SITE_PURPOSE), phx_id::Day::new(0), 0);
                    let site: TileId = match tiles.get(phx_rand::float::index(below_u64(&mut at, len_u64(tiles.len()))))
                    {
                        Some(t) => *t,
                        None => violation!(clause = "PTY.5", "a firm's region with no land", region = cell.region),
                    };
                    let bank = pick(&banks, below_u64(&mut at, bank_weight));
                    let (share, rest) = split_total(left, 1, whole, Round::HalfEven);
                    (left, whole) = (rest, whole - 1);
                    let record = [
                        MaybeI64::present(i64::from(cell.product)),
                        MaybeI64::present(i64::from(cell.region)),
                        MaybeI64::present(i64::from(site.get())),
                        MaybeI64::present(productivity),
                    ];
                    let id = PartyId::new(self.next_id);
                    self.next_id += 1;
                    let party = store.begin(id, &record, Some(Opening { bank, balance: share }));
                    let key = PartyKey::new(crate::core::kind_number(firm), party.slot());
                    let at_key = self.keys.partition_point(|(i, _)| *i < id);
                    self.keys.insert(at_key, (id, key));
                    ordinal += 1;
                }
            }
        }
        if let Some(k) = self.kinds.get_mut(firm) {
            *k = store;
        }
        // The small firms' table is kept, empty and holding no money, so the kinds keep their numbers.
        if let Some(k) = self.names.iter().position(|n| *n == "small_firm")
            && let Some(store) = self.kinds.get_mut(k)
        {
            *store = KindStore::new(&mut self.space, crate::core::kind_number(k), AGENT_ROWS, AGENT_ROWS_PER_CHUNK, 1);
        }
        Ok(())
    }

    /// The persons the core's households of a country hold, by region, in the country's regions' order.
    fn persons_by_region(&self, c: &OpeningCountry) -> Vec<(u32, u64)> {
        let mut out: Vec<(u32, u64)> = c.regions.iter().map(|(r, _)| (*r, 0)).collect();
        let Some(place) = self.names.iter().position(|n| *n == "household") else { return out };
        let (Some(store), Some(Some(persons)), Some(decl)) =
            (self.kinds.get(place), self.persons.get(place), self.household_decl.as_ref())
        else {
            return out;
        };
        let phx_num::Missing::Present(region_at) = decl.sited_by else { return out };
        for slot in store.parties.live_slots() {
            let Some(r) = store.record(slot).get(region_at).and_then(|w| match w.get() {
                phx_num::Missing::Present(v) => u32::try_from(v).ok(),
                phx_num::Missing::Absent => None,
            }) else {
                continue;
            };
            if let Some((_, n)) = out.iter_mut().find(|(x, _)| *x == r) {
                *n += len_u64(persons.of(slot).len());
            }
        }
        out
    }
}
