use libm::exp;
use phx_core::{OpeningCtx, Purpose, Register, StreamDecl};
use phx_id::TileId;
use phx_macros::clause;
use phx_num::{Fixed, Round};
use phx_rand::{Subject, SubjectTag, accept, normal};

use crate::climate::{cell1, cell2, exp_of, scaled};
use crate::generate::Map;
use crate::prims::{DEPOSIT_DENSITY, GRADE_MU, GRADE_SIGMA, GeoPrims, QUANTITY_MU, QUANTITY_SIGMA};

/// The stream deposits are drawn from, one opening per land tile.
pub const DEPOSITS_STREAM: StreamDecl =
    StreamDecl { name: "GEO.deposits", purpose: Purpose::Opening, keyed: false, clause: "GEO.6" };

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

/// Units taken from a finite deposit, written by GEO alone: what was extracted rises and what remains falls by them.
///
/// # Errors
/// When the deposit holds fewer than are taken, which leaves it as it was.
#[clause("GEO.12", "GDS.12")]
pub fn extract(table: &mut phx_core::FactColumns, row: phx_id::Slot, qty: i64) -> Result<(), String> {
    use phx_core::{FactDef, FactStore};

    use crate::audit::facts::{Extracted, Remaining};
    let (phx_num::Missing::Present(extracted), phx_num::Missing::Present(remaining)) =
        (table.value(Extracted::ITEM.name, row), table.value(Remaining::ITEM.name, row))
    else {
        return Err(format!("deposit row {} keeps no quantities", row.get()));
    };
    if qty < 0 || qty > remaining {
        return Err(format!("{qty} taken from deposit row {} holding {remaining}", row.get()));
    }
    table.write(Extracted::ITEM.name, row, extracted + qty);
    table.write(Remaining::ITEM.name, row, remaining - qty);
    Ok(())
}
