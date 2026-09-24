use phx_core::Weight;
use phx_id::{PartyId, Slot};
use phx_ledger::holding::CellHolding;
use phx_ledger::part::DetachedRow;
use phx_macros::clause;
use phx_num::Missing;

use crate::key::KeyRecord;
use crate::profile::Profile;
use crate::table::Attention;

/// A part's identity for the day: the cell it left and its place among that cell's parts, in the order the cell's
/// handlers emitted them. It orders every part of the day and never enters the directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PartId {
    pub origin: PartyId,
    pub seq: u32,
}

/// Members split from a cell, carried in the day until they land at 10b: their weight, key and signature, their share
/// of the cell's totals, their profile counts, their rows and holdings, and the cell's per-member rates and attention
/// they took with them. `from` only locates the cell they left.
#[clause("REP.8", "REP.14")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub id: PartId,
    pub from: Slot,
    pub weight: Weight,
    pub key: KeyRecord,
    pub sig: Vec<u64>,
    pub positions: Vec<i64>,
    pub rates: Vec<Missing<i64>>,
    pub exposures: Vec<Missing<i64>>,
    pub attention: Vec<Missing<Attention>>,
    pub profile: Profile,
    pub rows: Vec<DetachedRow>,
    pub holdings: Vec<CellHolding>,
}
