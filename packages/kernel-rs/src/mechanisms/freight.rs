//! FREIGHT AND LOGISTICS: moving a quantity from one place to another over a time, at a price that
//! clears per route — and no instantaneous, costless transport.
//!
//! @spec 38 A1 · 38 A2 · 38 A3 · 38 A3.a · 38 A4 · 38 B1 · 38 B2 · 38 B2.a · 38 B3 · 38 B4 · 38 C1 ·
//! @spec 38 C1.a · 38 C2 · 38 C3 · 38 D1 · 38 D2 · 38 D3 · 38 D3.a · 38 D4 · 38 D5 · 38 D6 · 38 E1 ·
//! @spec 38 E2 · 38 E3 · 21 A1.a · Law 3, Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::geography::RouteId;
use crate::ids::{InstrumentId, MarketId, PartyId};
use crate::journal::Value;
use crate::ledger::{Cause, Delivery, Leg};
use crate::module::{Mechanism, MechanismContext};
use crate::module::{Participant, ParticipantView};
use std::collections::HashMap;

/// One quantity physically admitted to a carrier's finite week capacity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Dispatch {
    pub week: u32,
    pub shipper: PartyId,
    pub consignee: PartyId,
    /// Legal title while the goods are between the two regions.
    pub owner: PartyId,
    pub carrier: PartyId,
    pub what: InstrumentId,
    pub on: RouteId,
    pub units: f64,
    /// Delivery cannot precede this week.
    pub arrives: u32,
}

/// The durable capacity ledger. Capacity is shared by every route a carrier serves in a week, so
/// two books cannot each consume the same ship or truck.
#[derive(Default)]
pub struct Dispatches {
    rows: Vec<Dispatch>,
    used: HashMap<u64, f64>,
    outcomes: Vec<Option<DeliveryOutcome>>,
}

/// The legal result of carriage. Title changes only in `Delivered`; carrier failure leaves it with
/// the transit owner and names the party against whom the shipper has its carriage remedy.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeliveryOutcome {
    Delivered {
        title_to: PartyId,
    },
    CarrierFailed {
        title_stays_with: PartyId,
        claim_on: PartyId,
    },
}

fn capacity_key(week: u32, carrier: PartyId) -> u64 {
    (u64::from(week) << 32) | u64::from(carrier.0)
}

/// Fill a requested dispatch in carrier order, never assigning more than each carrier has left.
pub fn fit_dispatch(requested: f64, available: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    assert!(
        requested >= 0.0,
        "38 D6: dispatch demand cannot be negative"
    );
    let mut left = requested;
    let mut out = Vec::new();
    for &(carrier, room) in available {
        assert!(room >= 0.0, "38 E2: carrier room cannot be negative");
        if left <= 0.0 || room <= 0.0 {
            continue;
        }
        let moved = if room < left { room } else { left };
        out.push((carrier, moved));
        left -= moved;
    }
    out
}

impl Dispatches {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn used(&self, week: u32, carrier: PartyId) -> f64 {
        match self.used.get(&capacity_key(week, carrier)) {
            Some(units) => *units,
            None => 0.0,
        }
    }

    pub fn record(&mut self, dispatch: Dispatch) {
        assert!(dispatch.units > 0.0, "38 E2: a dispatch must move units");
        assert!(
            dispatch.owner.some(),
            "38 E3: goods in transit need an owner"
        );
        assert!(
            dispatch.arrives > dispatch.week,
            "38 E1: transport cannot be instantaneous"
        );
        *self
            .used
            .entry(capacity_key(dispatch.week, dispatch.carrier))
            .or_default() += dispatch.units;
        self.rows.push(dispatch);
        self.outcomes.push(None);
    }

    pub fn in_period(&self, week: u32) -> impl Iterator<Item = &Dispatch> {
        self.rows
            .iter()
            .filter(move |dispatch| dispatch.week == week)
    }

    pub fn in_transit(&self, week: u32) -> impl Iterator<Item = &Dispatch> {
        self.rows
            .iter()
            .enumerate()
            .filter_map(move |(row, dispatch)| {
                (self.outcomes[row].is_none() && dispatch.week <= week && week < dispatch.arrives)
                    .then_some(dispatch)
            })
    }

    /// Settle every delivery which has reached its arrival week. The caller supplies liveness from
    /// the party store, making carrier failure an explicit legal result rather than a lost row.
    pub fn settle_arrivals(
        &mut self,
        week: u32,
        carrier_alive: impl Fn(PartyId) -> bool,
    ) -> Vec<(Dispatch, DeliveryOutcome)> {
        let mut settled = Vec::new();
        for (row, dispatch) in self.rows.iter().copied().enumerate() {
            if self.outcomes[row].is_some() || dispatch.arrives > week {
                continue;
            }
            let outcome = if carrier_alive(dispatch.carrier) {
                DeliveryOutcome::Delivered {
                    title_to: dispatch.consignee,
                }
            } else {
                DeliveryOutcome::CarrierFailed {
                    title_stays_with: dispatch.owner,
                    claim_on: dispatch.carrier,
                }
            };
            self.outcomes[row] = Some(outcome);
            settled.push((dispatch, outcome));
        }
        settled
    }

    pub fn outcome(&self, row: usize) -> Option<DeliveryOutcome> {
        self.outcomes.get(row).copied().flatten()
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
    Some(if room < wants_to_move {
        room
    } else {
        wants_to_move
    })
}

/// What goods under carriage tie up while they move — a real asset on a real balance sheet for as
/// long as the transit lasts.
pub fn working_capital(units: f64, at_cost: f64) -> f64 {
    units * at_cost
}

/// Freight demand equals the volume actually moving between locations, READ from the shipments —
/// never a separate series.
pub fn demand_on(route: RouteId, shipments: &[crate::geography::Shipment]) -> f64 {
    shipments
        .iter()
        .filter(|s| s.route == route)
        .map(|s| s.units)
        .sum()
}

/// 38 A1, A4: WHAT A CARRIER SELLS is carriage on a ROUTE — one unit of it moves one unit of goods
/// over that route this week. It is the same service whoever performs it, so the route has one line
/// and one book and not one per carrier (38 A4: routes are distinct, carriers are not).
///
/// A carrier makes its week's capacity out of the plant it owns, at what running that plant costs
/// it, and whatever it does not sell PERISHES: a week's room on a ship that sailed empty is gone,
/// which is why the price is as inelastic as B2.a says.
pub struct Sells {
    /// The carriage line each route's room is made of, declared with the network.
    pub on: Vec<(RouteId, InstrumentId)>,
    pub says: u32,
}

impl Mechanism for Sells {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut made: Vec<(PartyId, InstrumentId, f64, f64)> = Vec::new();
        let mut gone: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
        for (route, room) in &self.on {
            // 38 B8: the room on a route is what the vehicles standing at ITS origin can move.
            let Some(origin) = ctx
                .geography()
                .ends_of(*route)
                .and_then(|(from, _)| ctx.geography().tile_of(from))
            else {
                continue;
            };
            let standing: Vec<crate::geography::Vehicle> =
                ctx.geography().vehicles_at(origin).copied().collect();
            for vehicle in standing {
                // Who owns it is the register's answer — the vehicle keeps none of its own.
                let Some(owner) = ctx
                    .register()
                    .of_instrument(vehicle.line)
                    .iter()
                    .map(|row| ctx.register().holder_of(crate::ids::HoldingId(*row)))
                    .find(|holder| holder.some())
                else {
                    continue;
                };
                if !ctx.parties().alive(owner) {
                    continue;
                }
                // Last week's room is gone whether it sailed or not.
                let left = ctx.register().quantity(ctx.register().row(owner, *room));
                if left > 0.0 {
                    gone.push((owner, *room, left));
                }
                // What it can move is its own line's declared capacity over the units of it that
                // are in service, and what that costs is what SAILING costs — each vehicle its own,
                // which is what gives the route a supply schedule instead of one number.
                let (Some(plant), Some(running)) = (
                    ctx.registry().plant_of(vehicle.line),
                    ctx.registry().running_of(vehicle.line),
                ) else {
                    continue;
                };
                let holding = ctx.register().row(owner, vehicle.line);
                let carries =
                    crate::instruments::capacity(ctx.register().lots(holding), &plant, ctx.week());
                if carries <= 0.0 {
                    continue;
                }
                made.push((owner, *room, carries, running));
            }
        }
        for (who, line, units) in gone {
            let Some(qty) = crate::ledger::Units::new(units) else {
                continue;
            };
            ctx.propose(
                vec![Leg::Destroy {
                    party: who,
                    instrument: line,
                    qty,
                    why: crate::ledger::Gone::Perished,
                }],
                Cause::Production,
                Delivery::Nothing,
                "room nobody bought, on a week that has gone",
            );
        }
        for (who, line, units, cost_per_unit) in made {
            let Some(qty) = crate::ledger::Units::new(units) else {
                continue;
            };
            ctx.carries(who, line, crate::register::Carrying::Cost);
            ctx.propose(
                vec![Leg::Create {
                    party: who,
                    instrument: line,
                    qty,
                    cost_per_unit,
                }],
                Cause::Production,
                Delivery::Nothing,
                "the week's room one vehicle standing at this route's origin can move",
            );
            ctx.say(
                self.says,
                &[who.0],
                &[(0, Value::Num(f64::from(line.0))), (1, Value::Num(units))],
                true,
            );
        }
    }
}

/// 38 C1, C2: WHO WANTS THE ROOM. Demand is DERIVED — it exists because somebody is trading goods,
/// and a seller that cannot deliver cannot sell. So what a move is worth to a shipper is its own
/// margin on what it expects to move, and that margin is also what caps the price: past it, holding
/// the goods or not trading at all is the better answer, which is the substitution C2 names.
pub struct Ships {
    /// The carriage line on each route, and the route it is on.
    pub on: Vec<(RouteId, InstrumentId)>,
}

impl Ships {
    /// The best margin it has on anything it holds — what not being able to deliver would cost it.
    fn worth_moving(view: &ParticipantView<'_>) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for holding in view.holdings() {
            let line = view.line_of(holding);
            let units = view.free(line);
            let (Some(fetches), Some(cost)) = (
                view.price_outlook(line),
                view.lots(line).first().map(|lot| lot.basis_per_unit),
            ) else {
                continue;
            };
            let margin = fetches - cost;
            if units <= 0.0 || margin <= 0.0 || best.is_some_and(|(had, _)| had >= margin) {
                continue;
            }
            best = Some((margin, units));
        }
        best
    }
}

impl Participant for Ships {
    /// Room is bought to be used this week, and what it cost is what the move cost.
    fn carries(
        &self,
        _view: &ParticipantView<'_>,
        _m: MarketId,
    ) -> Option<crate::register::Carrying> {
        Some(crate::register::Carrying::Cost)
    }

    fn party_kind(&self) -> u32 {
        kinds::FIRM
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // It ships OUT of where it stands, so a route starting anywhere else is not its to book.
        let Some(here) = view.place() else {
            return Vec::new();
        };
        self.on
            .iter()
            .filter(|(route, _)| matches!(view.route_ends(*route), Some((from, _)) if from == here))
            .filter_map(|(_, line)| view.market_of(*line))
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some((margin, holds)) = Self::worth_moving(view) else {
            return Vec::new();
        };
        // 38 C1: it books what it expects to move, and a party with no expectation of selling has
        // no reason to book anything at all.
        let expects = match view.outlook(crate::stores::about::HOW_MUCH_IT_SELLS) {
            Some(units) if units > 0.0 => units,
            _ => return Vec::new(),
        };
        let wants = if holds < expects { holds } else { expects };
        let (bidding, _) = view.resting(m);
        let pieces = whole_pieces(wants) - bidding;
        if pieces <= 0 {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Buy,
            price: Some(margin),
            qty: pieces,
        }]
    }
}

/// A QUAY'S OWNER EARNS WHAT A BERTH CLEARS AT.
pub struct OffersItsRoom {
    /// The carriage lines it may have made room on.
    pub lines: Vec<InstrumentId>,
}

impl Participant for OffersItsRoom {
    /// A carrier sells the room it made and acquires nothing in this book.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        None
    }

    fn party_kind(&self) -> u32 {
        kinds::CARRIER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines
            .iter()
            .filter(|l| view.quantity(**l) > 0.0)
            .filter_map(|line| view.market_of(*line))
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        if view.free(line) <= 0.0 {
            return Vec::new();
        }
        let (_, offering) = view.resting(m);
        let mut already = offering;
        // 38 B3, B7: each vehicle's room is its own lot at what SAILING costs that vehicle, so a
        // carrier with a cheap ship and a dear one offers at two levels — which is the route's
        // supply schedule and not one number repeated.
        let mut orders = Vec::new();
        for lot in view.lots(line) {
            let pieces = whole_pieces(lot.qty) - already;
            already = if already > whole_pieces(lot.qty) {
                already - whole_pieces(lot.qty)
            } else {
                0
            };
            if pieces <= 0 || lot.basis_per_unit <= 0.0 {
                continue;
            }
            orders.push(Order {
                party: view.self_id(),
                side: Side::Sell,
                price: Some(lot.basis_per_unit),
                qty: pieces,
            });
        }
        orders
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn route() -> RouteId {
        RouteId::at(0)
    }

    fn other() -> RouteId {
        RouteId::at(1)
    }

    fn shipment(owner: u32, carrier: u32, on: RouteId, units: f64) -> crate::geography::Shipment {
        crate::geography::Shipment {
            id: crate::geography::ShipmentId::at(owner),
            goods: InstrumentId::at(1),
            units,
            owner: party(owner),
            carrier: party(carrier),
            route: on,
            destination: crate::geography::SiteId::at(0),
            dispatched: crate::calendar::Week(1),
            expected_arrival: crate::calendar::Week(2),
            promised_arrival: crate::calendar::Week(2),
            state: crate::geography::ShipmentState::InTransit,
            settled_freight: 0.0,
            settled_tolls: 0.0,
            settled_handling: 0.0,
        }
    }

    #[test]
    fn dispatch_never_exceeds_the_carriers_remaining_capacity() {
        let fitted = fit_dispatch(900.0, &[(party(90), 400.0), (party(91), 300.0)]);
        assert_eq!(fitted, vec![(party(90), 400.0), (party(91), 300.0)]);
        assert_eq!(fitted.iter().map(|(_, units)| units).sum::<f64>(), 700.0);
    }

    #[test]
    fn arrival_transfers_title_but_carrier_failure_retains_it_and_names_the_claim() {
        let dispatch = Dispatch {
            week: 4,
            shipper: party(1),
            consignee: party(2),
            owner: party(1),
            carrier: party(90),
            what: InstrumentId::at(7),
            on: route(),
            units: 3.0,
            arrives: 6,
        };
        let mut delivered = Dispatches::new();
        delivered.record(dispatch);
        assert!(delivered.settle_arrivals(5, |_| true).is_empty());
        assert_eq!(
            delivered.settle_arrivals(6, |_| true)[0].1,
            DeliveryOutcome::Delivered { title_to: party(2) }
        );

        let mut failed = Dispatches::new();
        failed.record(dispatch);
        assert_eq!(
            failed.settle_arrivals(6, |_| false)[0].1,
            DeliveryOutcome::CarrierFailed {
                title_stays_with: party(1),
                claim_on: party(90),
            }
        );
    }

    #[test]
    fn a_dispatch_carries_its_title_holder_until_arrival() {
        let dispatch = Dispatch {
            week: 4,
            shipper: party(20),
            consignee: party(21),
            owner: party(21),
            carrier: party(90),
            what: InstrumentId::at(3),
            on: route(),
            units: 10.0,
            arrives: 6,
        };
        assert_eq!(dispatch.owner, party(21));
        assert!(dispatch.week < dispatch.arrives);
    }

    #[test]
    fn goods_in_transit_are_owned_by_somebody_and_tie_up_working_capital() {
        // A real asset on a real balance sheet, for as long as the transit lasts.
        let s = shipment(20, 90, route(), 100.0);
        assert_eq!(working_capital(s.units, 12.0), 1_200.0);
        assert!(!s.destination_inventory());
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
            shipment(20, 90, route(), 100.0),
            shipment(21, 91, route(), 50.0),
            shipment(22, 92, other(), 900.0),
        ];
        assert_eq!(demand_on(route(), &shipments), 150.0);
        assert_eq!(demand_on(other(), &shipments), 900.0);
    }
}
