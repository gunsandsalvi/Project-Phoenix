//! Quantities and unit identities over hand-given values: units never mixed, overflow refused, and the identity's
//! width held.
#![cfg(test)]

use super::{Qty, QtyRaw, UnitId};
use crate::price::PriceRaw;
use crate::violation::CapacityExceeded;
use crate::violation::testing::violated_clause;

#[test]
fn qty_refuses_mixed_units_and_overflow() {
    let (a, b) = (UnitId::new(1), UnitId::new(2));
    assert_eq!(Qty::new(3, a) + Qty::new(4, a), Qty::new(7, a));
    assert_eq!(violated_clause(|| Qty::new(3, a) + Qty::new(4, b)), "NUM.5");
    assert_eq!(violated_clause(|| Qty::new(i64::MAX, a) + Qty::new(1, a)), "Law 7");
    assert_eq!(Qty::matched(Qty::new(5, a), Qty::new(3, a)), Qty::new(3, a));
    assert_eq!(violated_clause(|| Qty::matched(Qty::new(5, a), Qty::new(3, b))), "NUM.5");
}

#[test]
fn unit_id_width_stops() {
    assert_eq!(UnitId::new((1 << 24) - 1).index(), (1 << 24) - 1);
    let Err(payload) = std::panic::catch_unwind(|| UnitId::new(1 << 24)) else { panic!("2^24 accepted") };
    let c = payload.downcast_ref::<CapacityExceeded>().expect("a capacity payload");
    assert_eq!((c.declared, c.needed), (1 << 24, (1 << 24) + 1));
}

#[test]
fn qty_column_form_has_no_padding() {
    // The column forms are single words, and a unit identity one 32-bit word, so a row laid of them has no padding
    // where its fields are ordered widest first.
    assert_eq!(size_of::<UnitId>(), size_of::<u32>());
    assert_eq!(size_of::<QtyRaw>(), size_of::<i64>());
    assert_eq!(size_of::<PriceRaw>(), size_of::<i64>());
    assert_eq!(size_of::<(QtyRaw, UnitId, UnitId)>(), size_of::<QtyRaw>() + 2 * size_of::<UnitId>());
}
