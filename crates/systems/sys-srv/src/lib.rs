//! SRV, services and distribution: the retail market every product meets its buyers in, sellers' posted prices
//! against buyers' choices among the sellers in their reach, and the reason a purchase at the till settles under.

use phx_core::{
    Contribution, DECLARATIONS, Declarations, FactDef, HandlerTable, Opening, OpeningPhase, StreamDef, System,
    declare_prim, declare_stream,
};
use phx_id::MarketId;
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_macros::clause;
use phx_market::market::{Form, MarketDecl, MarketKey, Ration};
use phx_market::retail::RetailKind;
use phx_num::{Count, Fixed, Missing};

declare_stream! { pub TasteStream = "SRV.taste" { purpose: Meeting, keyed: false, clause: "REP.22" } }
declare_stream! { pub LotStream = "SRV.capacity_lot" { purpose: Meeting, keyed: false, clause: "REP.22" } }

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

/// A purchase at the till: the buyer's outlay an expense, the seller's receipt revenue, and the cost the seller's
/// units used up carry the cost of what it sold.
pub const SOLD: ReasonDecl = ReasonDecl {
    name: "SRV sold",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Revenue,
    held: Missing::Present((Effect::Expense, Effect::Asset)),
};

/// The kinds that sell at retail.
pub const SELLERS: &[&str] = &["firm", "small_firm"];

/// The retail market, one instance a product, meeting every day its sellers open.
pub const RETAIL: RetailKind = RetailKind {
    market: MarketDecl {
        id: MarketId::new(0),
        name: "retail",
        key: MarketKey { kind: "SRV.retail", subject: 0 },
        form: Form::Posted,
        operator: "the sellers",
        meeting_days: "every day",
        settle_days: 0,
        participants: "households and firms",
        tick: 1,
        ties: &[],
        ration: Ration::ProRata,
        stream: LotStream::DECL.name,
        quantity_response: Missing::Absent,
        admission: Missing::Absent,
    },
    sellers: SELLERS,
    sells: <if_firm::known::Industry as FactDef>::ITEM.name,
    price: <if_firm::facts::Price as FactDef>::ITEM.name,
    price_weight: PRICE_WEIGHT.id,
    distance_weight: DISTANCE_WEIGHT.id,
    reach: REACH.id,
    tastes: TasteStream::DECL.name,
    reason: SOLD.name,
};

/// The retail declarations in the books: the reason purchases settle under.
#[clause("SRV.6")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "retail declarations"
    }
    fn phase(&self) -> OpeningPhase {
        DECLARATIONS
    }
    fn reads(&self) -> &'static [&'static str] {
        &[]
    }
    fn writes(&self) -> &'static [&'static str] {
        &[]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &[]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let _ = phx_ledger::books::of(opening).ledger.reasons.declare(SOLD);
    }
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
        d.contribution(Box::new(Declared));
        d.market(Box::new(RETAIL));
    }

    fn handlers(_: &mut HandlerTable) {}
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
