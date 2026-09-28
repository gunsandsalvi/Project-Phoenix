//! FRT, freight: the carriage market at each origin and mode, carriers' room from their vehicles, the technology of
//! vehicles by mode and of goods' weight, the reasons freight is paid and goods leave and arrive under, and the
//! shipper's rule and visit, and each carrier's mode at the opening.

mod carriers;
mod consts;
pub mod ship;

use phx_core::register::values::Table1;
use phx_core::{
    Cadence, Contribution, DECLARATIONS, Declarations, FacetDecl, FactDef, HandlerDecl, HandlerTable, Opening,
    OpeningPhase, PositionDecl, Register, RunsOn, StreamDef, System, VisitDecl, declare_prim, declare_stream,
};
use phx_id::MarketId;
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_macros::clause;
use phx_market::carriage::{FreightKind, FreightTech};
use phx_market::market::{Form, MarketDecl, MarketKey, Ration};
use phx_num::{Count, Missing};

declare_stream! { pub LotStream = "FRT.capacity_lot" { purpose: Meeting, keyed: false, clause: "FRT.7" } }
declare_stream! { pub OpeningStream = "FRT.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub VisitStream = "FRT.visits" { purpose: Occasion, keyed: false, clause: "FRT.5" } }

declare_prim! {
    /// Each mode's share of the people carriage employs, by the mode's place, which the carriers' modes are drawn by.
    pub MODE_SHARE = "FRT.mode_share" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 1 }, clause: "GEN.2", scope: Shared
    }
}

declare_prim! {
    /// The product carriage is sold as, by its place among the products.
    pub CARRIAGE_PRODUCT = "FRT.carriage_product" { kind: Technology, value: Count, clause: "FRT.1", scope: Shared }
}

declare_prim! {
    /// Days between a shipper's decisions to carry its goods elsewhere.
    pub SHIPPING_DAYS = "FRT.shipping_days" { kind: Preference, value: Count, clause: "FRT.5", scope: Shared }
}

declare_prim! {
    /// The chain vehicles are held in: the place of transport equipment among the kinds of plant.
    pub VEHICLES = "FRT.vehicles" { kind: Technology, value: Count, clause: "FRT.1", scope: Shared }
}

declare_prim! {
    /// The tonne-km a unit of vehicles carries a day, by the mode's place.
    pub TONNE_KM = "FRT.tonne_km" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 6 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The metres a vehicle runs a day, by the mode's place.
    pub METRES_A_DAY = "FRT.metres_a_day" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The days loading takes at each end of a trip, by the mode's place.
    pub LOADING_DAYS = "FRT.loading_days" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The units of carriage a tonne-km takes, by the mode's place: what carrying it is worth at the opening's
    /// prices, in the carriage product's units.
    pub CARRIAGE_A_TONNE_KM = "FRT.carriage_a_tonne_km" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 2 }, clause: "FRT.12", scope: Shared
    }
}

declare_prim! {
    /// The units of a good in a tonne, by its product's place, for the storable products.
    pub UNITS_A_TONNE = "FRT.units_a_tonne" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "FRT.12", scope: Shared
    }
}

/// Freight paid for a trip: the shipper's outlay an expense, the carrier's receipt revenue.
pub const CARRIED: ReasonDecl = ReasonDecl {
    name: "FRT carried",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Revenue,
    held: Missing::Absent,
};

/// Goods leaving where they were for where they go, their cost going with them.
pub const SHIPPED: ReasonDecl = ReasonDecl {
    name: "FRT shipped",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Revenue,
    held: Missing::Present((Effect::Asset, Effect::Asset)),
};

/// Goods arriving where they go, at the cost they carried.
pub const ARRIVED: ReasonDecl = ReasonDecl {
    name: "FRT arrived",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Revenue,
    held: Missing::Present((Effect::Asset, Effect::Asset)),
};

/// The kind of large firms, which keep the carrier's mode as a fact.
const FIRM: &str = "firm";
/// The kind of small firms, whose agents keep it as a position.
const SMALL_FIRM: &str = "small_firm";
/// The kinds that carry.
pub const CARRIERS: &[&str] = &[FIRM, SMALL_FIRM];

/// A table of one axis over places in order, in the decimals its declaration gives.
fn places(register: &Register, decl: &phx_core::PrimDecl) -> Result<Vec<f64>, String> {
    let (id, phx_core::ValueType::Table1 { exp, .. }) = (decl.id, decl.value) else {
        return Err(format!("`{}` is declared as no table of one axis", decl.id));
    };
    let t = register.table1(id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err(format!("`{id}` is not by places in order"));
    }
    let scale = (0..exp).fold(1.0, |s, _| s * phx_core::consts::DECIMAL_BASE);
    Ok(t.values().iter().map(|v| phx_rand::float::from_i64(*v) / scale).collect())
}

/// Whole values of a table of one axis over places in order.
fn whole<T: TryFrom<i64>>(register: &Register, id: &str) -> Result<Vec<T>, String> {
    let t = register.table1(id)?;
    if t.axis().iter().zip(0_i64..).any(|(a, i)| *a != i) {
        return Err(format!("`{id}` is not by places in order"));
    }
    t.values().iter().map(|v| T::try_from(*v).map_err(|_| format!("`{id}` holds {v}, beyond its type"))).collect()
}

/// Freight's technology as the register holds it.
///
/// # Errors
/// When a table is missing or not by places in order, or a value is beyond its type.
pub fn tech(register: &Register) -> Result<FreightTech, String> {
    Ok(FreightTech {
        vehicles: u32::try_from(register.count(VEHICLES.id)?).map_err(|e| e.to_string())?,
        tonne_km: places(register, &TONNE_KM)?,
        metres_a_day: whole(register, METRES_A_DAY.id)?,
        loading_days: whole(register, LOADING_DAYS.id)?,
        carriage_a_tonne_km: places(register, &CARRIAGE_A_TONNE_KM)?,
        units_a_tonne: places(register, &UNITS_A_TONNE)?,
        carriage_product: u16::try_from(register.count(CARRIAGE_PRODUCT.id)?).map_err(|e| e.to_string())?,
    })
}

/// The carriage market, one instance an origin zone and mode, meeting on business days.
pub const CARRIAGE: FreightKind = FreightKind {
    market: MarketDecl {
        id: MarketId::new(0),
        name: "carriage",
        key: MarketKey { kind: "FRT.carriage", subject: 0 },
        form: Form::Posted,
        operator: "the carriers",
        meeting_days: "business days",
        settle_days: 0,
        participants: "firms",
        tick: 1,
        ties: &[],
        ration: Ration::ProRata,
        stream: LotStream::DECL.name,
        quantity_response: Missing::Absent,
        admission: Missing::Absent,
    },
    carriers: CARRIERS,
    mode: <if_firm::freight::Mode as FactDef>::ITEM.name,
    sells: <if_firm::known::Product as FactDef>::ITEM.name,
    price: <if_firm::facts::Price as FactDef>::ITEM.name,
    tech,
    paid: CARRIED.name,
    shipped: SHIPPED.name,
    arrived: ARRIVED.name,
};

/// The shipper's compiled reads: freight's technology and each product's lot.
fn compile(register: &Register) -> Result<ship::Own, String> {
    let lots = register
        .products("TEC.products")?
        .iter()
        .map(|e| match register.units().named(&e.unit) {
            Missing::Present(u) => {
                let exp = register
                    .units()
                    .decl(u)
                    .map(|d| d.price_exp)
                    .ok_or_else(|| format!("unit `{}` undeclared", e.unit))?;
                (0..exp)
                    .try_fold(1_i64, |b, _| b.checked_mul(consts::DECADE))
                    .ok_or_else(|| format!("product `{}`'s lot beyond a quantity", e.name))
            }
            Missing::Absent => Err(format!("product `{}` in an undeclared unit", e.name)),
        })
        .collect::<Result<Vec<i64>, String>>()?;
    let kind = phx_ledger::instruction::name_code(CARRIAGE.market.key.kind);
    Ok(ship::Own { tech: tech(register)?, lots, kind })
}

/// Whether a shipper books room: when what the goods fetch where they go, less what they fetch where they are,
/// exceeds what carrying them costs.
#[clause("FRT.5", "FRT.11")]
#[must_use]
pub fn books(there: i64, here: i64, freight: i64) -> bool {
    there - here > freight
}

/// Freight's declarations in the books: the reasons freight is paid and goods leave and arrive under.
#[clause("FRT.6")]
#[derive(Debug)]
pub struct Declared;

impl Contribution for Declared {
    fn name(&self) -> &'static str {
        "freight declarations"
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
        let reasons = &mut phx_ledger::books::of(opening).ledger.reasons;
        for r in [CARRIED, SHIPPED, ARRIVED] {
            let _ = reasons.declare(r);
        }
    }
}

/// Freight and logistics.
#[derive(Debug)]
pub struct Frt;

impl System for Frt {
    const CODE: &'static str = "FRT";

    fn declare(d: &mut Declarations) {
        let _ = d.prim::<Count>(&VEHICLES);
        for table in [&TONNE_KM, &METRES_A_DAY, &LOADING_DAYS, &CARRIAGE_A_TONNE_KM, &UNITS_A_TONNE] {
            let _ = d.prim::<Table1>(table);
        }
        d.stream(LotStream::DECL);
        let mode = <if_firm::freight::Mode as FactDef>::ITEM;
        d.claim(mode.name);
        d.facet(FacetDecl { fact: mode.name, kind: FIRM });
        d.pop_kind(SMALL_FIRM).position(PositionDecl {
            name: mode.name,
            clause: mode.clause,
            opening: phx_core::PositionOpening::Missing,
        });
        let share = d.prim::<Table1>(&MODE_SHARE);
        let product = d.prim::<Count>(&CARRIAGE_PRODUCT);
        let _ = d.prim::<Count>(&SHIPPING_DAYS);
        d.stream(OpeningStream::DECL);
        d.stream(VisitStream::DECL);
        d.contribution(Box::new(Declared));
        d.contribution(Box::new(carriers::Carriers { share, product }));
        d.market(Box::new(CARRIAGE));
        d.compile(Box::new(|register, _| Ok(Box::new(compile(register)?))));
        let cadence = Cadence::Schedule { days: SHIPPING_DAYS.id, runs_on: RunsOn::Business };
        for (handler, kind) in [(ship::ShipSmall::NAME, SMALL_FIRM), (ship::ShipLarge::NAME, FIRM)] {
            d.visit(VisitDecl { handler, kind, cadence, stream: VisitStream::DECL.name, wakes: &[], clause: "FRT.5" });
        }
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<ship::ShipSmall>();
        h.add::<ship::ShipLarge>();
    }
}

#[cfg(test)]
mod tests {
    use super::books;

    #[test]
    fn shipper_books_only_above_freight() {
        assert!(books(1_300, 1_000, 250), "a gap of 300 pays a freight of 250");
        assert!(!books(1_250, 1_000, 250), "a gap equal to the freight does not");
        assert!(!books(900, 1_000, 50), "goods dearer here stay");
    }
}
