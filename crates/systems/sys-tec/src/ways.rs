//! What a way takes and makes for a quantity started, with its rounding declared: inputs rounded up, so a way
//! never runs on less than it states, and output rounded down, so it never finishes more than its yield gives.

use if_base::consts::PER_UNIT_EXP;
use if_base::{PerUnit, ProductId, Way};
use phx_core::consts::PPM;
use phx_macros::clause;
use phx_num::consts::DECIMAL_BASE;
use phx_num::{QtyRaw, Round, div_round, violation};

fn narrow(raw: i128) -> QtyRaw {
    let Ok(raw) = i64::try_from(raw) else {
        violation!(clause = "NUM.6", "a way's quantity beyond a quantity's width");
    };
    QtyRaw::from_raw(raw)
}

/// What `started` units finish as.
#[clause("TEC.2", "TEC.9")]
pub fn finished(way: &Way, started: QtyRaw) -> QtyRaw {
    narrow(div_round(i128::from(started.raw()) * i128::from(way.yield_ppm), i128::from(PPM), Round::Floor))
}

/// The least started quantity that finishes `finished`: a start that finished nothing more is not a start.
#[clause("TEC.2", "TEC.9")]
pub fn started_for(way: &Way, finished: QtyRaw) -> QtyRaw {
    narrow(div_round(i128::from(finished.raw()) * i128::from(PPM), i128::from(way.yield_ppm), Round::Ceil))
}

/// What `started` units take of an input stated per unit.
#[clause("TEC.2", "TEC.9")]
pub fn takes(started: QtyRaw, per: PerUnit) -> QtyRaw {
    let scale = DECIMAL_BASE.pow(u32::from(PER_UNIT_EXP));
    narrow(div_round(i128::from(started.raw()) * i128::from(per.raw()), scale, Round::Ceil))
}

/// Each input `started` units take, in the way's order.
pub fn consumed(way: &Way, started: QtyRaw) -> impl Iterator<Item = (ProductId, QtyRaw)> + '_ {
    way.inputs.iter().map(move |(p, per)| (*p, takes(started, *per)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use phx_num::Missing;

    fn way(yield_ppm: u32) -> Way {
        Way {
            product: ProductId::new(0),
            inputs: vec![(ProductId::new(1), PerUnit::from_raw(250_000_000))].into_boxed_slice(),
            labour: Box::new([]),
            capital: Box::new([]),
            land: PerUnit::from_raw(0),
            deposit: Missing::Absent,
            lead_time_days: 1,
            batch: QtyRaw::from_raw(1),
            yield_ppm,
            by_products: Box::new([]),
        }
    }

    #[test]
    fn yield_applied_exactly() {
        assert_eq!(finished(&way(970_000), QtyRaw::from_raw(1_000)), QtyRaw::from_raw(970));
        // What does not come to a whole unit is not finished.
        assert_eq!(finished(&way(970_000), QtyRaw::from_raw(33)), QtyRaw::from_raw(32));
        assert_eq!(finished(&way(1_000_000), QtyRaw::from_raw(7)), QtyRaw::from_raw(7));
    }

    #[test]
    fn the_least_start_that_finishes() {
        let w = way(970_000);
        assert_eq!(started_for(&w, QtyRaw::from_raw(970)), QtyRaw::from_raw(1_000));
        assert_eq!(finished(&w, started_for(&w, QtyRaw::from_raw(32))), QtyRaw::from_raw(32));
        assert_eq!(started_for(&way(1_000_000), QtyRaw::from_raw(7)), QtyRaw::from_raw(7));
    }

    #[test]
    fn inputs_taken_whole_and_never_short() {
        let taken: Vec<(ProductId, QtyRaw)> = consumed(&way(1_000_000), QtyRaw::from_raw(10)).collect();
        assert_eq!(taken, vec![(ProductId::new(1), QtyRaw::from_raw(3))]);
        assert_eq!(takes(QtyRaw::from_raw(8), PerUnit::from_raw(250_000_000)), QtyRaw::from_raw(2));
    }
}
