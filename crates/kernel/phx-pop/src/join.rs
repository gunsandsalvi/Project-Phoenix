use phx_core::Weight;
use phx_id::Slot;
use phx_ledger::apply::Ledger;
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
    let target = landing.target;
    let weight = table.weight(target);
    let mut done = Joined::default();
    for (i, joining) in part.positions.iter().enumerate() {
        let held = table.position(target, i);
        done.erased.push(erased(part.weight, *joining, weight, held));
        table.set_position(target, i, add(held, *joining));
    }
    let layout = table.profile_layout().clone();
    let mut profile = table.profile(target);
    for g in 0..layout.groups.len() {
        for (v, n) in part.profile.held(g) {
            profile.add(&layout, g, *v, *n);
        }
    }
    table.set_profile(target, &profile);
    for (j, joining) in part.exposures.iter().enumerate() {
        match (table.exposure(target, j), joining) {
            (Missing::Present(e), Missing::Present(x)) => table.set_exposure(target, j, add(e, *x)),
            (Missing::Absent, Missing::Absent) => {}
            _ => violation!(clause = "REP.21", "a landing between exposures kept and absent", review = j),
        }
    }
    for row in part.rows {
        done.rows += 1;
        if ledger.attach_row(table, place, target, row) {
            done.holder_list_changes += 1;
        }
    }
    for holding in part.holdings {
        if ledger.attach_holding(table, place, target, holding) {
            done.holder_list_changes += 1;
        }
    }
    let Some(joined) = weight.get().checked_add(part.weight.get()) else {
        capacity_exceeded!("members of a cell", u32::MAX, weight.get());
    };
    table.set_weight(target, Weight::new(joined));
    done
}
