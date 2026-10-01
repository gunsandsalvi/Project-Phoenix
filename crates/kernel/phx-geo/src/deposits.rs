use libm::exp;
use phx_core::{OpeningCtx, Purpose, Register, StreamDecl};
use phx_id::TileId;
use phx_macros::clause;
use phx_num::{Fixed, MaybeI64, Missing, Round, UnitId, capacity_exceeded, violation};
use phx_rand::{Subject, SubjectTag, accept, normal};

use crate::climate::{cell1, cell2, exp_of, scaled};
use crate::generate::Map;
use crate::prims::{DEPOSIT_DENSITY, GRADE_MU, GRADE_SIGMA, GeoPrims, QUANTITY_MU, QUANTITY_SIGMA};

/// The stream deposits are drawn from, one opening per land tile.
pub const DEPOSITS_STREAM: StreamDecl = StreamDecl {
    name: "GEO.deposits",
    family: phx_core::StreamFamily::World,
    purpose: Purpose::Opening,
    keyed: false,
    clause: "GEO.6",
};

/// How much a deposit held when the world opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Opening {
    /// A finite quantity in the resource's units.
    Finite(u64),
    Unbounded,
}

/// A deposit: its tile, its resource by place in the declared list, its grade and what it held at the opening. What
/// has been extracted is a fact of the deposit table.
#[clause("GEO.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Deposit {
    pub tile: TileId,
    pub resource: u16,
    pub grade: Fixed<3>,
    pub opening: Opening,
}

/// The resources the data declares, and the terrain classes, as table points.
fn resources(p: &GeoPrims, r: &Register) -> Vec<i64> {
    p.deposits.density.shared(r).rows().to_vec()
}

/// The refusal of a resource declared present on every terrain: what a place can extract is a fact of its ground.
#[clause("GEO.16")]
#[must_use]
pub fn everywhere(p: &GeoPrims, r: &Register) -> Vec<String> {
    let density = p.deposits.density.shared(r);
    let terrains = p.terrain_elevation.shared(r).axis();
    resources(p, r)
        .into_iter()
        .filter(|res| terrains.iter().all(|t| density.at(*res, *t).is_ok_and(|d| d > 0)))
        .map(|res| format!("resource {res} is declared present on every terrain"))
        .collect()
}

/// Every land tile's deposits, each resource drawn in the declared order from the tile's own draws: present with the
/// declared chance for its terrain, then its grade and, unless the resource is unbounded, its quantity, each
/// log-normal.
#[clause("GEO.6")]
#[must_use]
pub fn draw(p: &GeoPrims, r: &Register, map: &Map, ctx: &OpeningCtx<'_>) -> Vec<Deposit> {
    let dp = &p.deposits;
    let (density, unbounded) = (dp.density.shared(r), dp.unbounded.shared(r));
    let at = |t: &phx_core::Table1, decl, res| scaled(cell1(t, res), exp_of(decl));
    let mut out = Vec::new();
    for (index, tile) in map.tiles.iter().enumerate().filter(|(_, t)| t.is_land()) {
        let id = map.grid.tile(index);
        let mut d = ctx.draws(&DEPOSITS_STREAM, Subject::new(SubjectTag::Tile, u64::from(id.get())));
        for (res, resource) in resources(p, r).into_iter().zip(0_u16..) {
            let chance = scaled(cell2(density, res, i64::from(tile.terrain)), exp_of(&DEPOSIT_DENSITY));
            if !accept(&mut d, chance) {
                continue;
            }
            let log_grade = at(dp.grade_mu.shared(r), &GRADE_MU, res)
                + at(dp.grade_sigma.shared(r), &GRADE_SIGMA, res) * normal(&mut d);
            let Ok(grade) = Fixed::<3>::from_f64(exp(log_grade), Round::HalfEven) else {
                phx_num::violation!(clause = "GEO.6", "a deposit's grade beyond its width");
            };
            let opening = if cell1(unbounded, res) == 1 {
                Opening::Unbounded
            } else {
                let log_q = at(dp.quantity_mu.shared(r), &QUANTITY_MU, res)
                    + at(dp.quantity_sigma.shared(r), &QUANTITY_SIGMA, res) * normal(&mut d);
                let Some(q) = phx_rand::float::floor_to_u64(exp(log_q)) else {
                    phx_num::violation!(clause = "GEO.6", "a deposit's quantity beyond its width");
                };
                Opening::Finite(q)
            };
            out.push(Deposit { tile: id, resource, grade, opening });
        }
    }
    out
}

/// A deposit's row in the deposit table, which keeps a row for each finite deposit in the list's order; none for a
/// deposit without end, which nothing depletes.
pub fn row_of(deposits: &[Deposit], index: u32) -> phx_num::Missing<phx_id::Slot> {
    let Some(at) = usize::try_from(index).ok().filter(|i| *i < deposits.len()) else {
        phx_num::violation!(clause = "GDS.12", "a deposit the map does not hold", deposit = index);
    };
    match deposits.get(at).map(|d| d.opening) {
        Some(Opening::Finite(_)) => {
            let before = deposits.iter().take(at).filter(|d| matches!(d.opening, Opening::Finite(_))).count();
            let Ok(row) = u32::try_from(before) else {
                phx_num::capacity_exceeded!("deposit rows", u32::MAX, before);
            };
            phx_num::Missing::Present(phx_id::Slot::new(row))
        }
        _ => phx_num::Missing::Absent,
    }
}

/// A deposit's identity: its row's place in the register, never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DepositId(u32);

impl DepositId {
    #[must_use]
    pub const fn new(id: u32) -> DepositId {
        DepositId(id)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A deposit's row: its tile, the unit its resource is at its grade and zone, the unit its right to extract is, its
/// grade at the opening in thousandths, what it held then (absent for a deposit without end, never a large number in
/// its place), and what has been extracted from it since. What remains is a read.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct DepositRow {
    tile: TileId,
    resource: UnitId,
    right: UnitId,
    grade: i32,
    opening: MaybeI64,
    extracted: u64,
}

impl DepositRow {
    pub fn tile(&self) -> TileId {
        self.tile
    }

    pub fn resource(&self) -> UnitId {
        self.resource
    }

    pub fn right(&self) -> UnitId {
        self.right
    }

    pub fn grade(&self) -> Fixed<3> {
        Fixed::from_raw(i64::from(self.grade))
    }

    /// What it held at the opening; absent for a deposit without end.
    pub fn opening(&self) -> Missing<u64> {
        match self.opening.get() {
            Missing::Present(q) => Missing::Present(q.unsigned_abs()),
            Missing::Absent => Missing::Absent,
        }
    }

    #[must_use]
    pub fn extracted(&self) -> u64 {
        self.extracted
    }

    /// What remains: what it opened with less what has been extracted; absent for a deposit without end.
    pub fn remaining(&self) -> Missing<u64> {
        match self.opening() {
            Missing::Present(q) => match q.checked_sub(self.extracted) {
                Some(left) => Missing::Present(left),
                None => violation!(clause = "GEO.12", "a deposit extracted past what it held", opening = q),
            },
            Missing::Absent => Missing::Absent,
        }
    }
}

/// An extraction: the deposit it is taken from, the party that takes it, and the units it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extraction {
    pub deposit: DepositId,
    pub extractor: u32,
    pub units: u64,
}

/// Every deposit's row, by identity: its extracted total written only by the day's extractions.
#[clause("GEO.6", "GEO.9", "GEO.12")]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Deposits {
    rows: Vec<DepositRow>,
}

fn at(id: DepositId) -> usize {
    match usize::try_from(id.0) {
        Ok(i) => i,
        Err(_) => capacity_exceeded!("deposits", usize::MAX, id.0),
    }
}

impl Deposits {
    /// A deposit entered at the opening: its tile, its resource's and its right's units, its grade, and what it holds,
    /// absent for one without end.
    #[phx_macros::opening]
    pub fn open(
        &mut self,
        tile: TileId,
        (resource, right): (UnitId, UnitId),
        grade: Fixed<3>,
        opening: Missing<u64>,
    ) -> DepositId {
        let Ok(id) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("deposits", u32::MAX, self.rows.len());
        };
        let Ok(grade) = i32::try_from(grade.raw()) else {
            capacity_exceeded!("a deposit's grade", i32::MAX, grade.raw());
        };
        let opening = match opening {
            Missing::Present(q) => match i64::try_from(q) {
                Ok(q) => MaybeI64::present(q),
                Err(_) => capacity_exceeded!("a deposit's quantity", i64::MAX, q),
            },
            Missing::Absent => MaybeI64::ABSENT,
        };
        self.rows.push(DepositRow { tile, resource, right, grade, opening, extracted: 0 });
        DepositId(id)
    }

    /// A deposit's row.
    #[must_use]
    pub fn row(&self, id: DepositId) -> DepositRow {
        match self.rows.get(at(id)) {
            Some(r) => *r,
            None => violation!(clause = "GDS.12", "a deposit the map does not hold", deposit = id.0),
        }
    }

    /// The day's extractions, each a transformation's leg taking units from its deposit, in the order given: an
    /// extraction past what remains stops the run, the extractor having read what remains before it chose. The
    /// extracted totals are the only thing written; the rows read.
    #[clause("GEO.12", "GDS.12", "SET.9")]
    pub fn extract_batch(&mut self, items: &[Extraction]) -> u64 {
        for x in items {
            let Some(row) = self.rows.get_mut(at(x.deposit)) else {
                violation!(clause = "GDS.12", "an extraction where there is no deposit", deposit = x.deposit.0);
            };
            if let Missing::Present(left) = row.remaining()
                && x.units > left
            {
                violation!(clause = "GEO.12", "an extraction past what a deposit holds", deposit = x.deposit.0);
            }
            let Some(extracted) = row.extracted.checked_add(x.units) else {
                capacity_exceeded!("a deposit's extracted total", u64::MAX, row.extracted);
            };
            row.extracted = extracted;
        }
        u64::try_from(items.len()).unwrap_or(u64::MAX)
    }

    /// A deposit's grade as it is worked: its opening grade carried by the resource's rule over the share of it taken,
    /// the richest part first; a deposit without end keeps its grade.
    #[clause("GEO.9")]
    #[must_use]
    pub fn grade_now(&self, id: DepositId, rule: impl Fn(f64, f64) -> f64) -> f64 {
        let row = self.row(id);
        match row.opening() {
            Missing::Present(q) if q > 0 => {
                rule(row.grade().to_f64(), phx_rand::float::from_u64(row.extracted) / phx_rand::float::from_u64(q))
            }
            _ => row.grade().to_f64(),
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Its rows held in no more room than they take, once the opening has entered them.
    pub fn settle(&mut self) {
        self.rows.shrink_to_fit();
    }
}

impl phx_store::StoreStats for Deposits {
    fn rows_live(&self) -> u64 {
        u64::try_from(self.rows.len()).unwrap_or(u64::MAX)
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        u64::try_from(self.rows.capacity() * size_of::<DepositRow>()).unwrap_or(u64::MAX)
    }
}

impl phx_store::Saved for Deposits {
    /// The rows, extracted totals and all: what remains is read from them.
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        w.rows(&self.rows, phx_store::Transform::Plain);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Deposits, phx_store::LoadError> {
        Ok(Deposits { rows: r.rows(phx_store::Transform::Plain)? })
    }
}

#[cfg(test)]
#[path = "deposits_tests.rs"]
mod tests;
