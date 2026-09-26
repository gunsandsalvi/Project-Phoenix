//! Freight: every shipment in transit pledged to its one carrier for its units and none overdue; freight rates against
//! the gaps between places, once goods are shipped.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Every shipment in transit is its owner's goods pledged by a standing lien to its carrier for its units, and
/// leaves after it left.
fn one_owner_one_carrier(w: Inspector<'_>) -> Outcome {
    let ledger = &w.books().ledger;
    for (n, s) in ledger.goods.in_transit() {
        let Some(lien) = ledger.liens.get(s.lien) else {
            return Outcome::Fail(format!("shipment {n} in transit with no lien standing"));
        };
        if lien.to != s.carrier || lien.units != s.qty || lien.key.holder != s.owner {
            return Outcome::Fail(format!("shipment {n}'s lien is not its owner's to its carrier for its units"));
        }
        if s.arrives <= s.left {
            return Outcome::Fail(format!("shipment {n} arrives the day it left or before"));
        }
    }
    Outcome::Pass
}

pub const LC_1_19: super::Check = live_check! {
    id: "LC-1-19",
    title: "FRT.9 and GEO.13: every shipment has one owner, one carrier and its goods pledged to it; no carrier books \
            beyond its vehicles' room and no segment beyond its capacity, which the carriage meeting refuses",
    from_step: "S1.07",
    check: one_owner_one_carrier,
};

fn no_freight_reads(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("freight rates are read against price gaps once goods are shipped (S1.15)")
}

pub const LC_1_20: super::Check = live_check! {
    id: "LC-1-20",
    title: "FRT.10: freight rates and price gaps between places are reported, and gaps track freight",
    from_step: "S1.07",
    check: no_freight_reads,
};

/// No shipment still in transit was due before today: each has arrived, its lien released.
fn none_overdue(w: Inspector<'_>) -> Outcome {
    let today = w.today();
    match w.books().ledger.goods.in_transit().find(|(_, s)| s.arrives < today) {
        Some((n, s)) => Outcome::Fail(format!("shipment {n} due on day {} still in transit", s.arrives.get())),
        None => Outcome::Pass,
    }
}

pub const LC_1_47: super::Check = live_check! {
    id: "LC-1-47",
    title: "every lien of goods in transit is released on arrival (FRT.6, FRT.8): no shipment stays in transit past \
            its day",
    from_step: "S1.07",
    check: none_overdue,
};
