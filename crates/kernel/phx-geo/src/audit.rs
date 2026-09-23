use std::sync::Arc;

use phx_core::{
    AuditFamily, FactColumns, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, InjectTarget, Unit,
    declare_family,
};
use phx_id::{CountryId, Slot};
use phx_macros::clause;
use phx_num::Missing;

use crate::state::GeoState;

declare_family! { pub PLACES = "GEO.places" { mode: Rolling { cycle_days: 30 }, clause: "GEO.11" } }
declare_family! { pub DEPOSITS = "GEO.deposits" { mode: Rolling { cycle_days: 30 }, clause: "GEO.12" } }

/// The table finite deposits are kept in, one row each, and its facts.
pub const DEPOSIT_TABLE: &str = "deposit";

fn finding(decl: FamilyDecl, ctx: &FamilyCtx<'_>, owner: FindingOwner, size: i128, detail: String) -> Finding {
    Finding { family: decl.name, clause: decl.clause, owner, size, unit: Unit::Count, day: ctx.day(), detail }
}

/// Every land tile's zone lies in a region of a country the world has, and every region has land: one slice of the
/// tiles a day, and the regions whole.
#[clause("GEO.11")]
#[derive(Debug)]
pub struct Places {
    pub geo: Arc<GeoState>,
    pub countries: usize,
}

impl AuditFamily for Places {
    fn decl(&self) -> FamilyDecl {
        PLACES
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let map = &self.geo.map;
        let span = ctx.rolling(map.tiles.len());
        let mut rows = 0;
        for i in span.iter() {
            let Some(tile) = map.tiles.get(i) else { continue };
            rows += 1;
            if !tile.is_land() {
                continue;
            }
            let country = tile
                .zone()
                .and_then(|z| map.zones.get(usize::try_from(z.get()).ok()?))
                .and_then(|z| map.regions.get(usize::from(z.region.get())))
                .map(|g| g.country);
            if country.is_none_or(|c| usize::from(c.get()) >= self.countries) {
                let t = map.grid.tile(i);
                let detail = format!("land tile {} lies in no country of the world", t.get());
                findings.record(finding(PLACES, ctx, FindingOwner::Tile(t), 1, detail));
            }
        }
        for (r, region) in map.regions.iter().enumerate() {
            rows += 1;
            let has_land = map.zones.iter().any(|z| usize::from(z.region.get()) == r && z.tiles > 0);
            if !has_land {
                let owner = FindingOwner::Country(CountryId::new(region.country.get()));
                findings.record(finding(PLACES, ctx, owner, 1, format!("region {r} has no land")));
            }
        }
        rows
    }

    fn inject(&self, _: &mut dyn InjectTarget) -> Result<(), String> {
        Err("the map is fixed at the opening and no party has a site a save could move".to_owned())
    }
}

/// For every finite deposit, extracted plus remaining equals what it held at the opening: one slice of the deposits a
/// day.
#[clause("GEO.12")]
#[derive(Debug)]
pub struct Deposits;

/// The deposit table's facts.
pub mod facts {
    phx_core::declare_fact! {
        pub Opening = "GEO.deposit_opening" {
            value: Qty, kinds: ["deposit"], writer: "GEO", audience: Public, repr: Position, clause: "GEO.12",
        }
    }
    phx_core::declare_fact! {
        pub Extracted = "GEO.deposit_extracted" {
            value: Qty, kinds: ["deposit"], writer: "GEO", audience: Public, repr: Position, clause: "GEO.12",
        }
    }
    phx_core::declare_fact! {
        pub Remaining = "GEO.deposit_remaining" {
            value: Qty, kinds: ["deposit"], writer: "GEO", audience: Public, repr: Position, clause: "GEO.12",
        }
    }
}

use phx_core::FactDef;

fn balance(t: &FactColumns, slot: Slot) -> Result<i128, String> {
    let read = |fact: &str| match t.value(fact, slot) {
        Missing::Present(v) => Ok(i128::from(v)),
        Missing::Absent => Err(format!("deposit {} has no `{fact}`", slot.get())),
    };
    let opening = read(facts::Opening::ITEM.name)?;
    Ok(read(facts::Extracted::ITEM.name)? + read(facts::Remaining::ITEM.name)? - opening)
}

impl AuditFamily for Deposits {
    fn decl(&self) -> FamilyDecl {
        DEPOSITS
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let t = ctx.table(DEPOSIT_TABLE);
        let span = ctx.rolling(usize::try_from(t.rows()).unwrap_or(usize::MAX));
        let mut rows = 0;
        for i in span.iter() {
            let Ok(row) = u32::try_from(i) else { continue };
            rows += 1;
            let slot = Slot::new(row);
            match balance(t, slot) {
                Ok(0) => {}
                Ok(gap) => {
                    let detail = format!("deposit {row}: extracted and remaining differ from its opening by {gap}");
                    findings.record(finding(DEPOSITS, ctx, FindingOwner::Run, gap, detail));
                }
                Err(detail) => findings.record(finding(DEPOSITS, ctx, FindingOwner::Run, 1, detail)),
            }
        }
        rows
    }

    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let (slot, name) = (Slot::new(0), facts::Remaining::ITEM.name);
        let Missing::Present(v) = target.fact(DEPOSIT_TABLE, name, slot) else {
            return Err("the save keeps no finite deposit".to_owned());
        };
        target.set_fact(DEPOSIT_TABLE, name, slot, v + 1)
    }
}
