//! SRV, services and distribution: how buyers weigh sellers' posted prices and distances in choosing among the
//! sellers in their reach, and a sale's retail margin.

use phx_core::{Declarations, StreamDef, System, declare_prim, declare_stream};
use phx_macros::clause;
use phx_num::{Count, Fixed};

declare_stream! { pub TasteStream = "SRV.taste" { family: World, purpose: Meeting, keyed: false, clause: "REP.22" } }
declare_stream! { pub LotStream = "SRV.capacity_lot" { family: World, purpose: Meeting, keyed: false, clause: "REP.22" } }

declare_prim! {
    /// How much a buyer weighs a seller's price in choosing among sellers: its value falls by this for each unit of the
    /// log of the price, against a standard Gumbel taste.
    pub PRICE_WEIGHT = "SRV.price_weight" { kind: Preference, value: Fixed { exp: 3 }, clause: "SRV.4", scope: Shared }
}

declare_prim! {
    /// How much a buyer weighs a seller's distance in choosing among sellers: its value falls by this for each km.
    pub DISTANCE_WEIGHT = "SRV.distance_weight" {
        kind: Preference, value: Fixed { exp: 3 }, clause: "SRV.4", scope: Shared
    }
}

declare_prim! {
    /// How far a buyer goes to shop, in metres of path between its zone and the seller's: the sellers in its reach.
    pub REACH = "SRV.reach" { kind: Technology, value: Count, clause: "SRV.9", scope: Shared }
}

/// Services and distribution.
#[derive(Debug)]
pub struct Srv;

impl System for Srv {
    const CODE: &'static str = "SRV";

    fn declare(d: &mut Declarations) {
        let _ = d.prim::<Fixed<3>>(&PRICE_WEIGHT);
        let _ = d.prim::<Fixed<3>>(&DISTANCE_WEIGHT);
        let _ = d.prim::<Count>(&REACH);
        d.stream(TasteStream::DECL);
        d.stream(LotStream::DECL);
    }
}

/// A sale's retail margin, what the seller keeps of its price at the till: the price less what the good cost it at
/// wholesale, the freight to bring it to the outlet and the consumption tax paid at the till. The price is the
/// seller's own; the margin is read from it, never applied to a factory price.
#[clause("SRV.6", "SRV.7", "SRV.8")]
#[must_use]
pub fn margin(till: i64, wholesale: i64, freight: i64, tax: i64) -> i64 {
    till - wholesale - freight - tax
}

#[cfg(test)]
mod tests {
    use super::margin;

    #[test]
    fn retail_price_components() {
        let (wholesale, freight, tax) = (600, 45, 170);
        let till = 1_020;
        let m = margin(till, wholesale, freight, tax);
        assert_eq!(wholesale + freight + tax + m, till, "the components add up to the price at the till");
        assert_eq!(margin(700, 600, 45, 170), -115, "a price below its costs leaves a loss, never a floor");
    }
}
