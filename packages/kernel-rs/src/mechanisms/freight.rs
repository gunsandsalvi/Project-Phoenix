//! FREIGHT AND LOGISTICS: moving a quantity from one place to another over a time, at a price that
//! clears per route — and no instantaneous, costless transport.
//!
//! @spec 38 A1 · 38 A2 · 38 A3 · 38 A3.a · 38 A4 · 38 B1 · 38 B2 · 38 B2.a · 38 B3 · 38 B4 · 38 C1 ·
//! @spec 38 C1.a · 38 C2 · 38 C3 · 38 D1 · 38 D2 · 38 D3 · 38 D3.a · 38 D4 · 38 D5 · 38 D6 · 38 E1 ·
//! @spec 38 E2 · 38 E3 · 21 A1.a · Law 3, Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::RegionId;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{book_of, line_of, InstrumentId, MarketId, PartyId};
use crate::module::{Participant, ParticipantView};
use crate::params::Denomination;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};

/// A4, 21 A1.a: the price is per unit per route, and routes are DISTINCT — capacity on one is not
/// capacity on another, which is why the same commodity has two prices in two places.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Route {
    pub from: RegionId,
    pub to: RegionId,
}

/// A carrier owns capital — ships, trucks, planes, warehouses, with their own lives — and capacity
/// is FIXED in the short run and expensive and slow to add, which is why the freight price is
/// extremely inelastic.
#[derive(Clone, Copy, Debug)]
pub struct Carrier {
    pub who: PartyId,
    pub on: Route,
    /// No capacity without a carrier that owns it.
    pub units_per_period: f64,
    /// An operating cost — fuel, labour, and the capital charge.
    pub cost_per_unit: f64,
    /// The transit time is a real lag between a purchase and a delivery.
    pub periods_in_transit: u32,
}

impl Carrier {
    /// A disruption is a REAL REDUCTION IN UNITS MOVED — not a multiplier on a price.
    pub fn disrupted(&self, units_lost: f64) -> Carrier {
        assert!(
            units_lost <= self.units_per_period,
            "38 B4: a disruption cannot lose more capacity than the route had"
        );
        Carrier { units_per_period: self.units_per_period - units_lost, ..*self }
    }
}

/// Bought by a NAMED shipper from a NAMED carrier, at a price, in a currency — and the demand exists
/// because somebody is trading goods.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Booking {
    pub shipper: PartyId,
    pub on: Route,
    pub units: f64,
    /// The most this shipper will pay, which comes from what the move is worth to it.
    pub will_pay: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    pub moved: Vec<(PartyId, PartyId, f64, f64)>,
    /// The price that cleared on this route.
    pub price: Option<f64>,
    /// Capacity rations quantity, not only price — what did not move, because there was no room for
    /// it at any price.
    pub turned_away: Vec<(PartyId, f64)>,
}

/// It clears per route.
pub fn clearing(bookings: &[Booking], carriers: &[Carrier], on: Route) -> Cleared {
    let mut wanting: Vec<&Booking> = bookings.iter().filter(|b| b.on == on).collect();
    let mut sailing: Vec<&Carrier> = carriers.iter().filter(|c| c.on == on).collect();
    wanting.sort_by(|a, b| b.will_pay.total_cmp(&a.will_pay));
    sailing.sort_by(|a, b| a.cost_per_unit.total_cmp(&b.cost_per_unit));

    let mut left: Vec<f64> = sailing.iter().map(|c| c.units_per_period).collect();
    let mut moved = Vec::new();
    let mut price = None;
    let mut turned_away = Vec::new();

    for b in &wanting {
        let mut wants = b.units;
        for (at, c) in sailing.iter().enumerate() {
            if wants <= 0.0 || left[at] <= 0.0 || c.cost_per_unit > b.will_pay {
                continue;
            }
            let taken = if left[at] < wants { left[at] } else { wants };
            moved.push((b.shipper, c.who, taken, c.cost_per_unit));
            price = Some(c.cost_per_unit);
            left[at] -= taken;
            wants -= taken;
        }
        if wants > 0.0 {
            turned_away.push((b.shipper, wants));
        }
    }
    Cleared { moved, price, turned_away }
}

/// Goods in transit are owned by SOMEBODY, not yet where they are going — a real asset on a real
/// balance sheet and a real use of working capital.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Shipment {
    pub owner: PartyId,
    pub carrier: PartyId,
    pub on: Route,
    pub units: f64,
    pub at_cost: f64,
    pub arrives_in: u32,
}

impl Shipment {
    /// What it ties up while it moves.
    pub fn working_capital(&self) -> f64 {
        self.at_cost * self.units
    }

    pub fn arrived(&self, periods_passed: u32) -> bool {
        periods_passed >= self.arrives_in
    }
}

/// A shipper can NOT SHIP — hold the goods, source locally, or not trade at all.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shipper {
    Ships,
    Holds,
    SourcesLocally,
    DoesNotTrade,
}

pub fn decides(delivered_cost: f64, local_price: f64, worth_holding: bool) -> Shipper {
    if delivered_cost > local_price {
        return Shipper::SourcesLocally;
    }
    if worth_holding {
        return Shipper::Holds;
    }
    Shipper::Ships
}

/// The freight cost is part of the DELIVERED price of the good, so it flows into what the buyer
/// actually pays (§37 D4's landed cost).
pub fn delivered(ex_works: f64, freight: f64, duty: f64) -> f64 {
    ex_works + freight + duty
}

/// Freight is the mechanism behind the location basis — the same commodity priced differently in two
/// places — and D5: the gap should track the freight price on the route.
pub fn location_basis(price_there: f64, price_here: f64) -> f64 {
    price_there - price_here
}

/// The arbitrage that bounds the basis is SOMEBODY ACTUALLY SHIPPING, with capacity and cost.
pub fn arbitrages(basis: f64, route_price: f64, room: f64, wants_to_move: f64) -> Option<f64> {
    if basis <= route_price || room <= 0.0 {
        return None;
    }
    Some(if room < wants_to_move { room } else { wants_to_move })
}

/// Freight demand equals the volume actually moving between locations, READ from the shipments —
/// never a separate series.
pub fn demand_on(route: Route, shipments: &[Shipment]) -> f64 {
    shipments.iter().filter(|s| s.on == route).map(|s| s.units).sum()
}


/// WHAT IS UNDER CARRIAGE. A count of every live relation in the world stands in for it: §39 has no
/// carriage row of its own, so this says more than it knows and its own item is what narrows it.
pub struct Carriage {
    pub kind: u32,
}

impl Mechanism for Carriage {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let n = ctx.agreements().live_now() as f64;
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}


/// A QUAY'S OWNER EARNS WHAT A BERTH CLEARS AT.
pub struct LetsItsPlant {
    /// The lines whose USE it lets.
    pub lines: Vec<InstrumentId>,
    /// What keeping the plant costs its owner for a period, whether or not it is used.
    pub upkeep: &'static str,
}

impl Participant for LetsItsPlant {
    fn party_kind(&self) -> u32 {
        kinds::CARRIER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().filter(|l| view.quantity(**l) > 0.0).map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let line = line_of(m);
        let held = view.free(line);
        if held <= 0.0 {
            return Vec::new();
        }
        // It is not made to let below what standing there costs it.
        let upkeep = view.params().amount(self.upkeep, Denomination::Money);
        let (_, offering) = view.resting(m);
        let pieces = whole_pieces(held) - offering;
        if pieces <= 0 || upkeep <= 0.0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(upkeep), qty: pieces }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn route() -> Route {
        Route { from: RegionId::at(1), to: RegionId::at(2) }
    }

    fn other() -> Route {
        Route { from: RegionId::at(1), to: RegionId::at(3) }
    }

    fn carriers() -> Vec<Carrier> {
        vec![
            Carrier { who: party(90), on: route(), units_per_period: 400.0, cost_per_unit: 3.0, periods_in_transit: 2 },
            Carrier { who: party(91), on: route(), units_per_period: 300.0, cost_per_unit: 5.0, periods_in_transit: 1 },
            Carrier { who: party(92), on: other(), units_per_period: 900.0, cost_per_unit: 1.0, periods_in_transit: 1 },
        ]
    }

    #[test]
    fn capacity_on_one_route_is_not_capacity_on_another() {
        // A4, 21 A1.a: routes are distinct, which is why the same commodity has two prices in two
        // places.
        let bookings = [Booking { shipper: party(20), on: route(), units: 900.0, will_pay: 9.0 }];
        let c = clearing(&bookings, &carriers(), route());
        assert_eq!(c.moved.len(), 2);
        assert_eq!(c.turned_away, vec![(party(20), 200.0)]);
    }

    #[test]
    fn capacity_rations_quantity_and_not_only_price() {
        // A route's fill must be able to turn somebody away, and no price conjures a ship.
        let desperate = [Booking { shipper: party(20), on: route(), units: 5_000.0, will_pay: 900.0 }];
        let c = clearing(&desperate, &carriers(), route());
        let moved: f64 = c.moved.iter().map(|m| m.2).sum();
        assert_eq!(moved, 700.0);
        assert_eq!(c.turned_away, vec![(party(20), 4_300.0)]);
    }

    #[test]
    fn a_shipper_that_will_not_pay_the_cost_does_not_sail() {
        // The carrier will not sail below its operating cost, and nothing clears.
        let mean = [Booking { shipper: party(20), on: route(), units: 100.0, will_pay: 1.0 }];
        let c = clearing(&mean, &carriers(), route());
        assert!(c.price.is_none());
        assert!(c.moved.is_empty());
        assert_eq!(c.turned_away, vec![(party(20), 100.0)]);
    }

    #[test]
    fn a_disruption_is_a_real_reduction_in_units_moved() {
        // Not a multiplier on a price.
        let hit: Vec<Carrier> = carriers()
            .iter()
            .map(|c| if c.who == party(90) { c.disrupted(350.0) } else { *c })
            .collect();
        let bookings = [Booking { shipper: party(20), on: route(), units: 200.0, will_pay: 9.0 }];
        assert_eq!(clearing(&bookings, &carriers(), route()).price, Some(3.0));
        assert_eq!(clearing(&bookings, &hit, route()).price, Some(5.0));
    }

    #[test]
    fn goods_in_transit_are_owned_by_somebody_and_tie_up_working_capital() {
        // A real asset on a real balance sheet, for as long as the transit lasts.
        let s = Shipment { owner: party(20), carrier: party(90), on: route(), units: 100.0, at_cost: 12.0, arrives_in: 2 };
        assert_eq!(s.working_capital(), 1_200.0);
        assert!(!s.arrived(1));
        assert!(s.arrived(2));
    }

    #[test]
    fn a_shipper_can_decline_to_ship_at_all() {
        // Hold the goods, source locally, or not trade — real decisions, and the reason freight
        // demand is not simply whatever was produced.
        assert_eq!(decides(14.0, 11.0, false), Shipper::SourcesLocally);
        assert_eq!(decides(9.0, 11.0, true), Shipper::Holds);
        assert_eq!(decides(9.0, 11.0, false), Shipper::Ships);
    }

    #[test]
    fn the_freight_is_part_of_the_delivered_price() {
        // It flows into what the buyer actually pays.
        assert_eq!(delivered(900.0, 60.0, 40.0), 1_000.0);
    }

    #[test]
    fn the_basis_is_bounded_by_somebody_actually_shipping_and_stands_when_nobody_can() {
        // The arbitrage needs capacity and cost, and without them the gap persists — which is the
        // finding, not a defect to correct.
        let basis = location_basis(19.0, 11.0);
        assert!(arbitrages(basis, 3.0, 400.0, 1_000.0) == Some(400.0));
        // No room on the route: the gap stands.
        assert!(arbitrages(basis, 3.0, 0.0, 1_000.0).is_none());
        // And a gap that does not cover the freight is not worth moving.
        assert!(arbitrages(location_basis(12.0, 11.0), 3.0, 400.0, 1_000.0).is_none());
    }

    #[test]
    fn freight_demand_is_read_from_the_shipments_that_actually_move() {
        // Never a separate series.
        let shipments = [
            Shipment { owner: party(20), carrier: party(90), on: route(), units: 100.0, at_cost: 12.0, arrives_in: 2 },
            Shipment { owner: party(21), carrier: party(91), on: route(), units: 50.0, at_cost: 12.0, arrives_in: 1 },
            Shipment { owner: party(22), carrier: party(92), on: other(), units: 900.0, at_cost: 4.0, arrives_in: 1 },
        ];
        assert_eq!(demand_on(route(), &shipments), 150.0);
        assert_eq!(demand_on(other(), &shipments), 900.0);
    }

    #[test]
    fn transport_is_never_instantaneous_or_costless() {
        // That would collapse every location into one, and with it the basis, the arbitrage and the
        // working capital in transit.
        for c in carriers() {
            assert!(c.cost_per_unit > 0.0);
            assert!(c.periods_in_transit > 0);
        }
    }

    #[test]
    #[should_panic(expected = "more capacity than the route had")]
    fn a_disruption_cannot_lose_more_capacity_than_the_route_had() {
        carriers()[0].disrupted(9_000.0);
    }
}
