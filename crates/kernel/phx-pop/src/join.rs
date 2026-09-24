use phx_core::Weight;
use phx_id::Slot;
use phx_ledger::apply::Ledger;
use phx_ledger::part::merge_rows;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::Backing;

use crate::part::{Part, PartId};
use crate::table::CellTable;

/// A part's landing in a cell, the one way a cell's totals change at 10b: weights, totals, profile counts, review
/// exposures, rows by line and role, and holdings at pooled cost add. The check has passed, so its rates, attention,
/// records and steps are the cell's.
#[clause("REP.8", "REP.14", "SET.1")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Landing {
    pub part: PartId,
    pub target: Slot,
}

/// What a landing did, for the representation's costs: the rows it joined, the holder lists it changed, and per
/// position the dispersion it erased, in the position's unit.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Joined {
    pub rows: u32,
    pub holder_list_changes: u32,
    pub erased: Vec<f64>,
}

/// The dispersion two groups' joining erases about one position: the members of each at their own mean, `a` holding
/// `ta` in all and `b` holding `tb`, lose `a·b ÷ (a + b) × (ta ÷ a − tb ÷ b)²` of squared difference from the mean.
#[clause("REP.15")]
#[must_use]
pub fn erased(a: Weight, ta: i64, b: Weight, tb: i64) -> f64 {
    let (a, b) = (f64::from(a.get()), f64::from(b.get()));
    let gap = phx_rand::float::from_i64(ta) / a - phx_rand::float::from_i64(tb) / b;
    a * b / (a + b) * gap * gap
}

fn add(a: i64, b: i64) -> i64 {
    let Some(s) = a.checked_add(b) else {
        violation!(clause = "Law 7", "a landing's total overflows", total = a, joining = b);
    };
    s
}

/// Applies a landing: the part joins the cell whole.
#[clause("REP.8", "REP.14", "REP.15")]
pub fn join<B: Backing, L: Backing>(
    ledger: &mut Ledger<L>,
    table: &mut CellTable<B>,
    place: u16,
    landing: &Landing,
    part: Part,
) -> Joined {
    if landing.part != part.id {
        violation!(clause = "REP.8", "a landing applied to another part than its own", origin = part.id.origin.get());
    }
    join_batch(ledger, table, place, landing.target, vec![part])
}

/// Parts bound for one cell join it together, in the order given, leaving the cell as joining them one by one would:
/// weights, totals and exposures add; the profile is moved in one pass; the parts' rows are merged per line side and
/// attached once, each line's holder list changing at most once; and each part's erased dispersion is taken against
/// the cell as the parts before it left it.
#[clause("REP.8", "REP.14", "REP.15")]
pub fn join_batch<B: Backing, L: Backing>(
    ledger: &mut Ledger<L>,
    table: &mut CellTable<B>,
    place: u16,
    target: Slot,
    parts: Vec<Part>,
) -> Joined {
    let mut weight = table.weight(target);
    let mut held: Vec<i64> = (0..table.positions()).map(|i| table.position(target, i)).collect();
    let mut exposures: Vec<Missing<i64>> = (0..table.review_kinds()).map(|j| table.exposure(target, j)).collect();
    let mut done = Joined { erased: vec![0.0; held.len()], ..Joined::default() };
    let mut deltas: Vec<(usize, u32, i64)> = Vec::new();
    let mut rows = Vec::new();
    let mut holdings = Vec::new();
    let groups = table.profile_layout().groups.len();
    for part in parts {
        if part.positions.len() != held.len() {
            violation!(clause = "REP.20", "a part of other positions than its cell's", part = part.positions.len());
        }
        for ((h, joining), e) in held.iter_mut().zip(&part.positions).zip(done.erased.iter_mut()) {
            *e += erased(part.weight, *joining, weight, *h);
            *h = add(*h, *joining);
        }
        for (j, (e, joining)) in exposures.iter_mut().zip(&part.exposures).enumerate() {
            *e = match (*e, joining) {
                (Missing::Present(e), Missing::Present(x)) => Missing::Present(add(e, *x)),
                (Missing::Absent, Missing::Absent) => Missing::Absent,
                _ => violation!(clause = "REP.21", "a landing between exposures kept and absent", review = j),
            };
        }
        for g in 0..groups {
            deltas.extend(part.profile.held(g).iter().map(|(v, n)| (g, *v, i64::from(*n))));
        }
        let Some(joined) = weight.get().checked_add(part.weight.get()) else {
            capacity_exceeded!("members of a cell", u32::MAX, weight.get());
        };
        weight = Weight::new(joined);
        let Some(n) = u32::try_from(part.rows.len()).ok().and_then(|n| done.rows.checked_add(n)) else {
            capacity_exceeded!("rows of a batch", u32::MAX, done.rows);
        };
        done.rows = n;
        rows.extend(part.rows);
        holdings.extend(part.holdings);
    }
    for (i, h) in held.into_iter().enumerate() {
        table.set_position(target, i, h);
    }
    for (j, e) in exposures.into_iter().enumerate() {
        if let Missing::Present(x) = e {
            table.set_exposure(target, j, x);
        }
    }
    table.shift_profile(target, &crate::profile::net(deltas));
    done.holder_list_changes += ledger.attach_rows(table, place, target, merge_rows(rows));
    for holding in holdings {
        if ledger.attach_holding(table, place, target, holding) {
            done.holder_list_changes += 1;
        }
    }
    table.set_weight(target, weight);
    done
}
