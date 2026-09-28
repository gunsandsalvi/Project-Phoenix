//! A firm's production on its schedule: how much of its product to make over the period, within what its staff can
//! make and its stocks of inputs allow; what it offers other firms of what it then holds; and what it buys of the
//! inputs it will need, goods at the goods markets and services at retail, where a service is made as it is sold.

use phx_core::Register;
use phx_ledger::instruction::name_code;
use phx_num::Missing;
use phx_rand::float::{from_i64, from_u64};

use crate::consts::{PER_UNIT_SCALE, PPM};

/// A product as production and its trade read it: whether it can be held, the units its price is posted for, and
/// the market it is traded in between firms.
#[derive(Clone, Debug, PartialEq)]
pub struct Traded {
    pub storable: bool,
    pub lot: i64,
    pub market: u64,
}

/// A way as its user reads it: the product it makes, each input's units per unit started, its yield in millionths,
/// and the days a unit takes.
#[derive(Clone, Debug, PartialEq)]
pub struct WayRow {
    pub product: u16,
    pub inputs: Vec<(u16, i64)>,
    pub yield_ppm: i64,
    pub lead: f64,
}

/// What production reads, compiled once: each product's trade, and each way by its identity, each country's ways in
/// the products' order after the country before's, as the technology registers them.
#[derive(Clone, Debug, PartialEq)]
pub struct Plant {
    pub products: Vec<Traded>,
    pub ways: Vec<WayRow>,
    pub adjustment_days: f64,
}

impl Plant {
    /// # Errors
    /// A product in an undeclared unit, or ways' tables unread.
    pub fn compile(register: &Register, countries: usize, adjustment: u64) -> Result<Plant, String> {
        let entries = register.products("TEC.products")?;
        let standardised = register.table1("GDS.standardised")?;
        let mut products = Vec::with_capacity(entries.len());
        for (i, e) in (0_i64..).zip(entries) {
            let Missing::Present(unit) = register.units().named(&e.unit) else {
                return Err(format!("product `{}` in an undeclared unit", e.name));
            };
            let exp = register.units().decl(unit).map_or(0, |d| d.price_exp);
            let lot = (0..exp).try_fold(1_i64, |l, _| l.checked_mul(i64::from(crate::consts::TEN)));
            let Some(lot) = lot else { return Err(format!("`{}`'s price places beyond a quantity", e.unit)) };
            let kind = if standardised.at(i).is_ok_and(|v| v == 1) { "GDS.commodities" } else { "GDS.between_firms" };
            products.push(Traded { storable: e.storable, lot, market: name_code(kind) });
        }
        let (lead, yields) = (register.table1("TEC.lead_time")?, register.table1("TEC.yield")?);
        let mut ways = Vec::new();
        for c in 0..countries {
            let id = phx_id::CountryId::new(u8::try_from(c).map_err(|e| e.to_string())?);
            let inputs = register.table2_in("TEC.inputs", id)?;
            for j in 0..entries.len() {
                let col = i64::try_from(j).map_err(|e| e.to_string())?;
                let used = inputs
                    .rows()
                    .iter()
                    .filter_map(|r| {
                        let v = inputs.at(*r, col).ok()?;
                        (v != 0).then(|| u16::try_from(*r).ok().map(|q| (q, v)))?
                    })
                    .collect();
                ways.push(WayRow {
                    product: u16::try_from(j).map_err(|e| e.to_string())?,
                    inputs: used,
                    yield_ppm: yields.at(col).map_err(|_| format!("no yield of product {j}"))?,
                    lead: lead.at(col).map_or(0.0, from_i64),
                });
            }
        }
        Ok(Plant { products, ways, adjustment_days: from_u64(adjustment) })
    }
}

/// What starting enough to finish `finished` units takes of an input stated per unit: its units rounded up.
#[must_use]
pub fn takes(way: &WayRow, finished: i64, per: i64) -> i64 {
    if way.yield_ppm <= 0 {
        return 0;
    }
    let up = |a: i128, b: i128| phx_num::div_round(a, b, phx_num::Round::Ceil);
    let started = up(i128::from(finished) * i128::from(PPM), i128::from(way.yield_ppm));
    let took = up(started * i128::from(per), i128::from(PER_UNIT_SCALE));
    i64::try_from(took).unwrap_or(i64::MAX)
}

/// The most a way can finish from the units of each input held: the least over its storable inputs, each the most
/// finished whose take the holding covers, found by halving since a take rounds up.
#[must_use]
pub fn most_from(way: &WayRow, held: &dyn Fn(u16) -> i64, storable: &dyn Fn(u16) -> bool) -> Option<i64> {
    let mut most: Option<i64> = None;
    for (q, per) in way.inputs.iter().filter(|(q, _)| storable(*q)) {
        if *per <= 0 {
            continue;
        }
        let have = held(*q);
        let bound = i128::from(have) * i128::from(PER_UNIT_SCALE) / i128::from(*per) + 1;
        let (mut low, mut high) = (0_i64, i64::try_from(bound).unwrap_or(i64::MAX));
        while low < high {
            let mid = low + (high - low) / 2 + (high - low) % 2;
            if takes(way, mid, *per) <= have { low = mid } else { high = mid - 1 }
        }
        most = Some(most.map_or(low, |m| if low < m { low } else { m }));
    }
    most
}
