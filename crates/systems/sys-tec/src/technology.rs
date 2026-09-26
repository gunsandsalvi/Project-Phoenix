//! The technology compiled from the register at assembly: the products, every country's opening ways, one per
//! product, and the public ways of each country's industries, which every firm of the industry knows.

use if_base::{CapKind, IndustryId, OccFamily, PerUnit, ProductId, Products, Way, WayId, WaySetId};
use phx_core::Register;
use phx_core::register::values::{Table1, Table2};
use phx_id::CountryId;
use phx_macros::clause;
use phx_num::{Missing, QtyRaw};

use crate::prims::TecPrims;
use crate::sets::WaySets;

/// The products, the ways, and each country's public set per industry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Technology {
    pub products: Products,
    ways: Vec<Way>,
    /// Each country's public set, by industry.
    public: Vec<Vec<WaySetId>>,
    pub sets: WaySets,
}

impl Technology {
    /// The technology the register declares for `countries` countries.
    ///
    /// # Errors
    /// Products the register refuses, and a table whose points are not the products in order.
    #[clause("TEC.2", "TEC.3", "TEC.4", "TEC.13")]
    pub fn compile(p: &TecPrims, register: &Register, countries: usize) -> Result<Technology, String> {
        let products = crate::products::build(p.products.shared(register), register.units())?;
        let shared = |t: &Table1, what: &str| per_product(t, &products, what);
        let draws = shared(p.deposit_draw.shared(register), "TEC.deposit_draw")?;
        let lead = shared(p.lead_time.shared(register), "TEC.lead_time")?;
        let yields = shared(p.yield_ppm.shared(register), "TEC.yield")?;
        let batches = shared(p.batch.shared(register), "TEC.batch")?;
        let mut tech = Technology { products, ..Technology::default() };
        for c in 0..countries {
            let country = CountryId::new(u8::try_from(c).map_err(|_| format!("{countries} countries"))?);
            let inputs = p.inputs.get(register, country);
            let labour = p.labour.get(register, country);
            let capital = p.capital.get(register, country);
            let land = per_product(p.land.get(register, country), &tech.products, "TEC.land")?;
            for t in [inputs, labour, capital] {
                columns_are_products(t, &tech.products)?;
            }
            let mut by_industry: Vec<Vec<WayId>> = vec![Vec::new(); tech.products.industries.len()];
            for decl in &tech.products.decls {
                let j = usize::from(decl.id.index());
                let at = |v: &[i64]| v.get(j).copied().ok_or_else(|| format!("no value for product {j}"));
                let deposit = match decl.extracts {
                    Missing::Present(r) => Missing::Present((r, PerUnit::from_raw(at(&draws)?))),
                    Missing::Absent => Missing::Absent,
                };
                let way = Way {
                    product: decl.id,
                    inputs: column(inputs, j, |i| u16::try_from(i).ok().map(ProductId::new))?,
                    labour: column(labour, j, |i| u8::try_from(i).ok().map(OccFamily))?,
                    capital: column(capital, j, |i| u8::try_from(i).ok().map(CapKind))?,
                    land: PerUnit::from_raw(at(&land)?),
                    deposit,
                    lead_time_days: u16::try_from(at(&lead)?).map_err(|_| format!("product {j}'s lead time"))?,
                    batch: QtyRaw::from_raw(at(&batches)?),
                    yield_ppm: match u32::try_from(at(&yields)?) {
                        Ok(y) if y > 0 => y,
                        _ => return Err(format!("product {j}'s way finishes nothing it starts")),
                    },
                    by_products: Box::new([]),
                };
                let id = WayId::new(u32::try_from(tech.ways.len()).map_err(|_| "more ways than an identity holds")?);
                tech.ways.push(way);
                if let Some(ways) = by_industry.get_mut(usize::from(decl.industry.index())) {
                    ways.push(id);
                }
            }
            let sets = by_industry.iter().map(|w| tech.sets.intern(w)).collect();
            tech.public.push(sets);
        }
        Ok(tech)
    }

    #[must_use]
    pub fn way(&self, id: WayId) -> Option<&Way> {
        self.ways.get(usize::try_from(id.index()).ok()?)
    }

    /// The ways every firm of a country's industry knows.
    #[must_use]
    pub fn public(&self, country: CountryId, industry: IndustryId) -> Option<WaySetId> {
        self.public.get(usize::from(country.get()))?.get(usize::from(industry.index())).copied()
    }

    /// The ways registered.
    #[must_use]
    pub fn ways(&self) -> usize {
        self.ways.len()
    }
}

/// A table over the products as one value per product, in the products' order.
fn per_product(t: &Table1, products: &Products, what: &str) -> Result<Vec<i64>, String> {
    if !points_are_products(t.axis(), products) {
        return Err(format!("{what} is not a table over the products in order"));
    }
    Ok(t.values().to_vec())
}

fn points_are_products(points: &[i64], products: &Products) -> bool {
    points.len() == products.decls.len() && points.iter().zip(0_i64..).all(|(p, i)| *p == i)
}

fn columns_are_products(t: &Table2, products: &Products) -> Result<(), String> {
    if points_are_products(t.columns(), products) {
        Ok(())
    } else {
        Err("a way table's columns are not the products in order".to_owned())
    }
}

/// A table's column as the non-zero quantities per unit of its rows: what a way states, and nothing it does not use.
fn column<K>(t: &Table2, j: usize, key: impl Fn(i64) -> Option<K>) -> Result<Box<[(K, PerUnit)]>, String> {
    let column = i64::try_from(j).map_err(|_| format!("product {j}"))?;
    let mut out = Vec::new();
    for row in t.rows() {
        let v = t.at(*row, column).map_err(|_| format!("no value at row {row}, product {j}"))?;
        if v < 0 {
            return Err(format!("a way of product {j} states less than nothing of row {row}"));
        }
        if v != 0 {
            out.push((key(*row).ok_or_else(|| format!("row {row} names nothing"))?, PerUnit::from_raw(v)));
        }
    }
    Ok(out.into_boxed_slice())
}
