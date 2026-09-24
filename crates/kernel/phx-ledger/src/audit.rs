use phx_core::{FindingOwner, declare_family};
use phx_id::{InstrumentId, LineId};
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::algebra::Side;
use crate::holder::{HolderArenas, HolderKeys};
use crate::holding::holding;
use crate::instrument::Instruments;
use crate::line::Lines;
use crate::rows::rows;

declare_family! { pub OWNERSHIP = "REG.ownership" { mode: Rolling { cycle_days: 30 }, clause: "REG.13" } }
declare_family! { pub CONTRACTS = "REG.contracts" { mode: Rolling { cycle_days: 30 }, clause: "REG.14" } }
declare_family! { pub MONEY = "MON.money" { mode: Rolling { cycle_days: 30 }, clause: "MON.7" } }
declare_family! { pub FLOWS = "SET.flows" { mode: Incremental, clause: "SET.9" } }
declare_family! { pub UNITS = "NUM.units" { mode: Incremental, clause: "NUM.5" } }

/// What a family found wrong with one instrument or line: whose it is, by how much, and what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    pub owner: FindingOwner,
    pub size: i128,
    pub detail: String,
}

/// A holder table by its place among the holder tables, which a holder key carries.
fn table<'a>(tables: &[&'a dyn HolderArenas], keys: HolderKeys, key: u32) -> Result<(&'a dyn HolderArenas, u16), u16> {
    let (place, _) = keys.split(key);
    match tables.get(usize::from(place)) {
        Some(t) => Ok((*t, place)),
        None => Err(place),
    }
}

/// Holdings of an instrument, summed over its holder list, against its issued amount; a holder on the list
/// without a holding, or on a table the world does not keep, is a gap of its own.
#[clause("REG.13")]
pub fn ownership<B: Backing>(instruments: &Instruments<B>, tables: &[&dyn HolderArenas], id: InstrumentId) -> Vec<Gap> {
    let owner = FindingOwner::Instrument(id);
    let keys = instruments.keys();
    let mut gaps = Vec::new();
    let mut held = 0_i128;
    for key in instruments.holders(id) {
        let (t, place) = match table(tables, keys, key) {
            Ok(found) => found,
            Err(place) => {
                let detail =
                    format!("instrument {}: a holder on table {place}, which the world does not keep", id.get());
                gaps.push(Gap { owner, size: 1, detail });
                continue;
            }
        };
        let (_, slot) = keys.split(key);
        match holding(t, slot, id) {
            Missing::Present(h) => held += i128::from(h.quantity.raw()),
            Missing::Absent => {
                let detail =
                    format!("instrument {}: holder {} of table {place} listed without a holding", id.get(), slot.get());
                gaps.push(Gap { owner, size: 1, detail });
            }
        }
    }
    let issued = i128::from(instruments.get(id).issued.n());
    if held != issued {
        let detail = format!("instrument {}: holdings of {held} against {issued} issued", id.get());
        gaps.push(Gap { owner, size: held - issued, detail });
    }
    gaps
}

/// A line's two sides against each other, and each side that keeps a holder list against its holders' rows.
#[clause("REG.14", "REP.31")]
pub fn contracts<B: Backing>(lines: &Lines<B>, tables: &[&dyn HolderArenas], line: LineId) -> Vec<Gap> {
    let owner = FindingOwner::Line(line);
    let keys = lines.keys();
    let mut gaps = Vec::new();
    let (asset, liability) = (lines.side_count(line, Side::Asset), lines.side_count(line, Side::Liability));
    if asset != liability {
        let detail =
            format!("line {}: {asset} on its asset side against {liability} on its liability side", line.get());
        gaps.push(Gap { owner, size: i128::from(asset) - i128::from(liability), detail });
    }
    let mut counted = [0_i128; 2];
    for key in lines.holders(line) {
        let Ok((t, _)) = table(tables, keys, key) else {
            let detail = format!("line {}: a holder on a table the world does not keep", line.get());
            gaps.push(Gap { owner, size: 1, detail });
            continue;
        };
        let (_, slot) = keys.split(key);
        for r in rows(t, slot).iter().filter(|r| r.row.line == line) {
            let [a, l] = &mut counted;
            let side = match r.side() {
                Side::Asset => a,
                Side::Liability => l,
            };
            *side += i128::from(r.row.count);
        }
    }
    let [asset_rows, liability_rows] = counted;
    for (side, kept, from_rows) in [(Side::Asset, asset, asset_rows), (Side::Liability, liability, liability_rows)] {
        if lines.listed_side(line, side) && i128::from(kept) != from_rows {
            let detail = format!("line {}: {side:?} side kept at {kept} against {from_rows} in its rows", line.get());
            gaps.push(Gap { owner, size: i128::from(kept) - from_rows, detail });
        }
    }
    gaps
}

/// A money line's balances: the holders' on its asset side and its issuer's on its liability side sum to nothing, so
/// what the issuer records as its money liability is what its holders hold. Both sides of a money line keep
/// holder lists; a line kind without them is not money.
#[clause("MON.7", "MON.11")]
pub fn money_line<B: Backing>(lines: &Lines<B>, tables: &[&dyn HolderArenas], line: LineId) -> Vec<Gap> {
    let owner = FindingOwner::Line(line);
    let keys = lines.keys();
    let mut gaps = Vec::new();
    if !lines.listed_side(line, Side::Asset) || !lines.listed_side(line, Side::Liability) {
        let detail = format!("line {}: a money line whose sides are not both listed", line.get());
        return vec![Gap { owner, size: 1, detail }];
    }
    let mut sum = 0_i128;
    for key in lines.holders(line) {
        let Ok((t, _)) = table(tables, keys, key) else {
            let detail = format!("line {}: a holder on a table the world does not keep", line.get());
            gaps.push(Gap { owner, size: 1, detail });
            continue;
        };
        let (_, slot) = keys.split(key);
        for r in rows(t, slot).iter().filter(|r| r.row.line == line) {
            match r.optional.balance {
                Missing::Present(b) => sum += i128::from(b),
                Missing::Absent => {
                    let detail = format!("line {}: a money row without a balance", line.get());
                    gaps.push(Gap { owner, size: 1, detail });
                }
            }
        }
    }
    if sum != 0 {
        let detail = format!("line {}: its holders' and its issuer's balances differ by {sum}", line.get());
        gaps.push(Gap { owner, size: sum, detail });
    }
    gaps
}
