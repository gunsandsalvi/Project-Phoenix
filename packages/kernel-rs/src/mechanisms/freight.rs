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
    /// 38 B6: the vehicle it is ON, so what happens to that vehicle happens to this cargo.
    pub aboard: crate::geography::VehicleId,
    pub what: InstrumentId,
    pub on: RouteId,
    pub units: f64,
    /// 49 G3: the room the owner had actually BOUGHT and gave up to this move. It is what the
    /// carriage check measures against, and short of `units` means room nobody paid for.
    pub carriage_settled: f64,
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

/// The legal result of carriage. One named owner throughout, so what an outcome says is whether the
/// goods are where they were sent, or whom the owner has a remedy against instead.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DeliveryOutcome {
    /// 49 G2: title did not move — it is one named owner the whole way. What changed is that the
    /// goods are HERE, and can be used or sold at last.
    Arrived { owner: PartyId },
    /// 49 G4: and where the carrier did not survive the voyage, what the owner has instead of its
    /// goods is a claim on the carrier.
    CarrierFailed { owner: PartyId, claim_on: PartyId },
}

fn capacity_key(week: u32, aboard: crate::geography::VehicleId) -> u64 {
    (u64::from(week) << 32) | u64::from(aboard.0)
}

/// Fill a requested dispatch in carrier order, never assigning more than each carrier has left.
pub fn fit_dispatch(
    requested: f64,
    available: &[(crate::geography::VehicleId, f64)],
) -> Vec<(crate::geography::VehicleId, f64)> {
    assert!(
        requested >= 0.0,
        "38 D6: dispatch demand cannot be negative"
    );
    let mut left = requested;
    let mut out = Vec::new();
    for &(aboard, room) in available {
        assert!(room >= 0.0, "38 E2: a vehicle's room cannot be negative");
        if left <= 0.0 || room <= 0.0 {
            continue;
        }
        let moved = if room < left { room } else { left };
        out.push((aboard, moved));
        left -= moved;
    }
    out
}

impl Dispatches {
    pub fn new() -> Self {
        Self::default()
    }

    /// 38 B8: room is a VEHICLE'S, so what is already aboard one is not room on another.
    pub fn used(&self, week: u32, aboard: crate::geography::VehicleId) -> f64 {
        match self.used.get(&capacity_key(week, aboard)) {
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
            .entry(capacity_key(dispatch.week, dispatch.aboard))
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

    /// 49 G4: WHAT HAS REACHED ITS ARRIVAL WEEK and has no outcome yet — a read, so the system that
    /// owns carriage decides what happened rather than the store deciding for it.
    pub fn due_in(&self, week: u32) -> Vec<(u32, Dispatch)> {
        self.rows
            .iter()
            .copied()
            .enumerate()
            .filter(|(row, dispatch)| self.outcomes[*row].is_none() && dispatch.arrives <= week)
            .map(|(row, dispatch)| (row as u32, dispatch))
            .collect()
    }

    /// And what it decided, written once. A dispatch settles one way and never again.
    pub fn settled(&mut self, row: u32, outcome: DeliveryOutcome) -> Dispatch {
        let row = row as usize;
        assert!(
            self.outcomes[row].is_none(),
            "49 G5: a dispatch delivers once"
        );
        self.outcomes[row] = Some(outcome);
        self.rows[row]
    }

    pub fn outcome(&self, row: usize) -> Option<DeliveryOutcome> {
        self.outcomes.get(row).copied().flatten()
    }
}

/// 49 F4: WHEN IT GETS THERE. The route's own length over what the vehicle covers in a week, plus
/// the weeks spent loading it — landed on the first tick of the clock that is not before it, and
/// never the week it left, because nothing crosses any distance in no time (49 F6).
pub fn arrives_in(route_km: f64, km_per_week: f64, loading_weeks: f64) -> u32 {
    assert!(
        route_km > 0.0 && route_km.is_finite(),
        "49 F1: a route of no length is not a route"
    );
    assert!(
        km_per_week > 0.0 && km_per_week.is_finite(),
        "49 F6: a vehicle covering no ground never arrives"
    );
    assert!(
        loading_weeks >= 0.0 && loading_weeks.is_finite(),
        "49 F4: loading cannot take negative time"
    );
    let weeks = (route_km / km_per_week + loading_weeks).ceil() as u32;
    if weeks < 1 {
        1
    } else {
        weeks
    }
}

/// Freight is the mechanism behind the location basis — the same commodity priced differently in two
/// places — and D5: the gap should track the freight price on the route.
pub fn location_basis(price_there: f64, price_here: f64) -> f64 {
    price_there - price_here
}

/// Freight demand equals the volume actually moving between locations, READ from the shipments —
/// never a separate series.
pub fn demand_on(route: RouteId, moving: &[Dispatch]) -> f64 {
    moving
        .iter()
        .filter(|d| d.on == route)
        .map(|d| d.units)
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

/// 49 G4: WHAT BECAME OF A SHIPMENT, and the vehicle it was aboard is where it delivered.
///
/// A carrier that died in transit does not hand the goods over: title stays where it was and the
/// shipper has a claim on the carrier, which is a named outcome and not a lost row.
pub fn arrived(d: &Dispatch, carrier_alive: bool) -> DeliveryOutcome {
    match carrier_alive {
        true => DeliveryOutcome::Arrived { owner: d.owner },
        false => DeliveryOutcome::CarrierFailed {
            owner: d.owner,
            claim_on: d.carrier,
        },
    }
}

/// 49 G2: CARGO IN THE AIR HAS A LIVE OWNER AND A LIVE CARRIER, or somebody's goods are aboard
/// nobody's ship.
#[derive(Default)]
pub struct CargoHasAnOwner {
    found: Vec<crate::audit::Violation>,
}

impl crate::audit::Contribution for CargoHasAnOwner {
    fn family(&self) -> crate::audit::Family {
        crate::audit::Family::Ownership
    }

    fn contributor(&self) -> &'static str {
        "freight"
    }

    fn before(&mut self, from: &crate::audit::Sources<'_>) {
        let live =
            |who: PartyId| who.some() && who.row() < from.parties.len() && from.parties.alive(who);
        self.found = from
            .wire
            .dispatches
            .in_transit(from.week)
            .filter(|d| !live(d.owner) || !live(d.carrier))
            .map(|d| crate::audit::Violation {
                family: crate::audit::Family::Ownership,
                spec: "49 G2",
                owner: format!("{}/{}", d.owner.0, d.carrier.0),
                size: d.units,
                unit: "units in transit",
                week: from.week,
                message: "cargo is in the air with no live owner or no live carrier".to_string(),
            })
            .collect();
    }

    fn finish(&mut self, _period: u32) -> Vec<crate::audit::Violation> {
        std::mem::take(&mut self.found)
    }
}

/// 49 G3: THE ROOM A MOVE USED WAS BOUGHT. Carriage is an instrument a shipper holds because it
/// paid a carrier for it, and settlement consumes what the owner had. Room it did not have is room
/// nobody was paid for — carriage given away, and the goods carrying a cost that never happened.
#[derive(Default)]
pub struct FreightIsPaidFor {
    found: Vec<crate::audit::Violation>,
}

impl crate::audit::Contribution for FreightIsPaidFor {
    fn family(&self) -> crate::audit::Family {
        crate::audit::Family::Flows
    }

    fn contributor(&self) -> &'static str {
        "freight.carriage"
    }

    fn before(&mut self, from: &crate::audit::Sources<'_>) {
        self.found = from
            .wire
            .dispatches
            .in_period(from.week)
            .filter(|d| d.carriage_settled < d.units)
            .map(|d| crate::audit::Violation {
                family: crate::audit::Family::Flows,
                spec: "49 G3",
                owner: format!("{}/{}", d.owner.0, d.carrier.0),
                size: d.units - d.carriage_settled,
                unit: "units carried unpaid",
                week: from.week,
                message: "a dispatch used more room than its owner ever bought".to_string(),
            })
            .collect();
    }

    fn finish(&mut self, _period: u32) -> Vec<crate::audit::Violation> {
        std::mem::take(&mut self.found)
    }
}

/// 49 G4, F4: A SHIPMENT PAST ITS PROMISE IS DELIVERED OR FAILED, and never neither. Carriage runs
/// at d4 and the audit at i2, so anything still due by then is a delivery nobody decided.
#[derive(Default)]
pub struct DeliveriesLandOnce {
    found: Vec<crate::audit::Violation>,
}

impl crate::audit::Contribution for DeliveriesLandOnce {
    fn family(&self) -> crate::audit::Family {
        crate::audit::Family::Liveness
    }

    fn contributor(&self) -> &'static str {
        "freight.arrivals"
    }

    fn before(&mut self, from: &crate::audit::Sources<'_>) {
        self.found = from
            .wire
            .dispatches
            .due_in(from.week)
            .into_iter()
            .map(|(_, d)| crate::audit::Violation {
                family: crate::audit::Family::Liveness,
                spec: "49 G4",
                owner: format!("{}/{}", d.owner.0, d.carrier.0),
                size: f64::from(from.week - d.arrives),
                unit: "weeks overdue",
                week: from.week,
                message: "a shipment is past its promise and is neither delivered nor failed"
                    .to_string(),
            })
            .collect();
    }

    fn finish(&mut self, _period: u32) -> Vec<crate::audit::Violation> {
        std::mem::take(&mut self.found)
    }
}

/// 49 G4, 38 A3: WHAT IS IN TRANSIT ARRIVES. The vehicle that carried it ends the week where it
/// delivered — so it is there to be booked from next week, and an empty leg back is somebody's
/// problem.
pub struct Arrives {
    pub says: u32,
}

impl Mechanism for Arrives {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let week = ctx.week();
        let due: Vec<(u32, Dispatch)> = ctx.wire().dispatches.due_in(week);
        for (row, d) in due {
            let outcome = arrived(&d, ctx.parties().alive(d.carrier));
            let Some(destination) = ctx.geography().ends_of(d.on).map(|(_, to)| to) else {
                continue;
            };
            ctx.delivers(row, outcome, d.aboard, destination);
            ctx.say(
                self.says,
                &[d.owner.0, d.carrier.0],
                &[
                    (0, Value::Num(f64::from(d.what.0))),
                    (1, Value::Num(d.units)),
                    (
                        2,
                        Value::Num(f64::from(matches!(
                            outcome,
                            DeliveryOutcome::Arrived { .. }
                        ))),
                    ),
                ],
                true,
            );
        }
    }
}

/// 38 C1, C2, D3: WHO WANTS THE ROOM, AND WHAT IT DOES WITH IT. Demand is DERIVED — it exists
/// because somebody holds goods that fetch more somewhere else. A shipper bids for room on a route
/// out of where it stands what the line fetches at the far end over what it is worth here, and
/// past that holding them or not trading is the better answer, which is what caps the price. With
/// room bought it sells delivered in the far end's book, at what the goods are worth here plus
/// what the room cost it — so the gap between two places is bounded by somebody actually shipping.
pub struct Ships {
    /// The carriage line on each route, and the route it is on.
    pub on: Vec<(RouteId, InstrumentId)>,
}

impl Ships {
    /// The routes out of where it stands, the room on each, and the place each delivers into.
    fn out_of_here(
        &self,
        view: &ParticipantView<'_>,
    ) -> Vec<(InstrumentId, crate::ids::RegionId)> {
        let Some(here) = view.place() else {
            return Vec::new();
        };
        self.on
            .iter()
            .filter(|(route, _)| matches!(view.route_ends(*route), Some((from, _)) if from == here))
            .filter_map(|(route, room)| Some((*room, view.destination_of(*route)?)))
            .collect()
    }

    /// The widest gap it holds goods across: what they fetch there over what they are worth here.
    fn widest_gap(view: &ParticipantView<'_>, there: crate::ids::RegionId) -> Option<(f64, f64)> {
        let mut best: Option<(f64, f64)> = None;
        for holding in view.holdings() {
            let line = view.line_of(holding);
            let units = view.free(line);
            let (Some(fetches), Some(here)) = (view.print_here(line, there), view.values(line))
            else {
                continue;
            };
            let gap = location_basis(fetches.price, here);
            if units <= 0.0 || gap <= 0.0 || best.is_some_and(|(had, _)| had >= gap) {
                continue;
            }
            best = Some((gap, units));
        }
        best
    }
}

impl Participant for Ships {
    /// Room is bought to be used this week, and what it cost is what the move cost; goods are
    /// only ever sold in the far book.
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
        let mut out = Vec::new();
        for (room, there) in self.out_of_here(view) {
            out.extend(view.market_of(room));
            if view.free(room) <= 0.0 {
                continue;
            }
            for holding in view.holdings() {
                let line = view.line_of(holding);
                if line != room && view.free(line) > 0.0 {
                    out.extend(view.market_at(line, there));
                }
            }
        }
        out
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(subject) = view.subject_of(m) else {
            return Vec::new();
        };
        let routes = self.out_of_here(view);
        let (bidding, offering) = view.resting(m);

        // THE ROOM: as much as it expects to sell of what it holds, at the widest gap it holds.
        if let Some((_, there)) = routes.iter().find(|(room, _)| *room == subject) {
            let Some((gap, holds)) = Self::widest_gap(view, *there) else {
                return Vec::new();
            };
            // 38 C1: a party with no expectation of selling has no reason to book anything.
            let expects = match view.outlook(crate::stores::about::HOW_MUCH_IT_SELLS) {
                Some(units) if units > 0.0 => units,
                _ => return Vec::new(),
            };
            let wants = if holds < expects { holds } else { expects };
            let pieces = whole_pieces(wants) - bidding;
            if pieces <= 0 {
                return Vec::new();
            }
            return vec![Order {
                party: view.self_id(),
                side: Side::Buy,
                price: Some(gap),
                qty: pieces,
            }];
        }

        // THE GOODS, DELIVERED: no more than the room it holds on the route into this book's place.
        let Some((room, _)) = routes
            .iter()
            .find(|(_, there)| view.market_at(subject, *there) == Some(m))
        else {
            return Vec::new();
        };
        let lots = view.lots(*room);
        let room_held: f64 = lots.iter().map(|l| l.qty).sum();
        let (Some(here), true) = (view.values(subject), room_held > 0.0) else {
            return Vec::new();
        };
        let freight = lots.iter().map(|l| l.qty * l.basis_per_unit).sum::<f64>() / room_held;
        let can = if view.free(*room) < view.free(subject) {
            view.free(*room)
        } else {
            view.free(subject)
        };
        let pieces = whole_pieces(can) - offering;
        if pieces <= 0 {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Sell,
            price: Some(here + freight),
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

    fn ship(n: u32) -> crate::geography::VehicleId {
        crate::geography::VehicleId::at(n)
    }

    fn route() -> RouteId {
        RouteId::at(0)
    }

    fn other() -> RouteId {
        RouteId::at(1)
    }

    fn moving(owner: u32, carrier: u32, on: RouteId, units: f64) -> Dispatch {
        Dispatch {
            week: 1,
            shipper: party(owner),
            consignee: party(owner + 50),
            owner: party(owner),
            carrier: party(carrier),
            aboard: ship(carrier),
            what: InstrumentId::at(1),
            on,
            units,
            carriage_settled: units,
            arrives: 2,
        }
    }

    #[test]
    fn a_longer_route_and_a_slower_vehicle_both_take_longer() {
        // 49 F4: travel time plus loading, landed on the first tick that is not before it.
        assert_eq!(arrives_in(300.0, 100.0, 0.5), 4);
        assert_eq!(arrives_in(600.0, 100.0, 0.5), 7);
        // Same route, half the speed: twice as long.
        assert_eq!(arrives_in(300.0, 50.0, 0.0), 6);
        // And a route shorter than one week's travel still takes a week — nothing is instant.
        assert_eq!(arrives_in(1.0, 1_000.0, 0.0), 1);
    }

    #[test]
    #[should_panic(expected = "never arrives")]
    fn a_vehicle_that_covers_no_ground_never_arrives() {
        arrives_in(300.0, 0.0, 0.5);
    }

    #[test]
    fn dispatch_never_exceeds_what_the_vehicles_standing_there_can_carry() {
        let fitted = fit_dispatch(900.0, &[(ship(90), 400.0), (ship(91), 300.0)]);
        assert_eq!(fitted, vec![(ship(90), 400.0), (ship(91), 300.0)]);
        assert_eq!(fitted.iter().map(|(_, units)| units).sum::<f64>(), 700.0);
    }

    #[test]
    fn an_arrival_leaves_title_where_it_was_and_carrier_failure_names_the_claim() {
        let dispatch = Dispatch {
            week: 4,
            shipper: party(1),
            consignee: party(2),
            owner: party(1),
            carrier: party(90),
            aboard: ship(90),
            what: InstrumentId::at(7),
            on: route(),
            carriage_settled: 40.0,
            units: 3.0,
            arrives: 6,
        };
        let mut delivered = Dispatches::new();
        delivered.record(dispatch);
        assert!(delivered.due_in(5).is_empty());
        let (row, due) = delivered.due_in(6)[0];
        assert_eq!(
            arrived(&due, true),
            DeliveryOutcome::Arrived { owner: party(1) }
        );
        // A carrier that died in transit hands nothing over, and the shipper has a claim on it.
        assert_eq!(
            arrived(&due, false),
            DeliveryOutcome::CarrierFailed {
                owner: party(1),
                claim_on: party(90),
            }
        );
        // And once it has settled it is not due again.
        delivered.settled(row, arrived(&due, true));
        assert!(delivered.due_in(6).is_empty());
    }

    #[test]
    fn a_dispatch_carries_its_title_holder_until_arrival() {
        let dispatch = Dispatch {
            week: 4,
            shipper: party(20),
            consignee: party(21),
            owner: party(21),
            carrier: party(90),
            aboard: ship(90),
            what: InstrumentId::at(3),
            on: route(),
            units: 10.0,
            carriage_settled: 10.0,
            arrives: 6,
        };
        assert_eq!(dispatch.owner, party(21));
        assert!(dispatch.week < dispatch.arrives);
    }

    #[test]
    fn freight_demand_is_read_from_the_shipments_that_actually_move() {
        // Never a separate series.
        let shipments = [
            moving(20, 90, route(), 100.0),
            moving(21, 91, route(), 50.0),
            moving(22, 92, other(), 900.0),
        ];
        assert_eq!(demand_on(route(), &shipments), 150.0);
        assert_eq!(demand_on(other(), &shipments), 900.0);
    }
}
