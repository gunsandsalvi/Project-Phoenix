//! Freight in the world: a shipper's want to carry goods admitted at 5d, the goods covered; at 6a each origin and
//! mode's carriers found and the carriage meeting held over what each segment can still carry today; at 6d each
//! booking's freight owed the carrier, settled in stage 7, when the goods are pledged to the carrier and set on their
//! way; at 5a of their day, arrivals: the lien released and the goods moved from where they left to where they
//! arrive, carrying their cost.

use std::collections::{BTreeMap, BTreeSet};

use phx_core::{Declarations, FactStore, Register, SubStep};
use phx_geo::network::Route;
use phx_id::{Day, InstrumentId, MarketId, PartyId, ZoneId};
use phx_ledger::apply::ApplyAt;
use phx_ledger::covered::Covered;
use phx_ledger::goods::{GoodKey, Shipment};
use phx_ledger::instruction::{AccountRef, Denom, Instruction, LegKind, LegRec, Source};
use phx_macros::clause;
use phx_market::carriage::{Carrier, Consignment, FreightKind, FreightTech};
use phx_market::intents::ShipIntent;
use phx_num::{Missing, PriceRaw, Qty, capacity_exceeded, violation};
use phx_pop::population::Population;
use phx_rand::{Subject, SubjectTag};
use phx_store::SystemBacking;

use crate::goods::Rows;
use crate::world::World;

/// A carriage kind as the world meets it: its place among the market kinds, its declaration, the tables of the kinds
/// that carry, and its technology.
#[derive(Clone, Debug)]
pub(crate) struct FreightBound {
    pub kind: u16,
    pub decl: FreightKind,
    pub carriers: Vec<(u16, bool)>,
    pub tech: FreightTech,
}

/// A shipper's want admitted for the day: its market, the shipper, the good it holds where it stands and the units,
/// covered until booked, the zone they go to, the mode and route, and their weight in kilograms.
#[derive(Debug)]
pub(crate) struct Ship {
    pub market: MarketId,
    pub party: PartyId,
    pub good: InstrumentId,
    pub qty: i64,
    pub cover: Covered,
    pub to: ZoneId,
    pub mode: u16,
    pub route: Route,
    pub kg: i64,
}

/// A booking of the day's meetings: the want, the carrier and its price for a lot of carriage, and the lots of
/// carriage the trip takes.
#[derive(Debug)]
pub(crate) struct Booked {
    pub ship: Ship,
    pub carrier: PartyId,
    pub price: PriceRaw,
    pub lots: i64,
}

/// A carrier's row as read: its rows and slot, its mode, the product it sells and its posted price.
type CarrierRead = (Rows, phx_id::Slot, u16, Missing<i64>, Missing<i64>);

/// Every carriage kind the systems declare, bound to its market kind, its carriers' tables and its technology; every
/// refusal at once.
pub(crate) fn bind(
    d: &Declarations,
    kinds: &phx_market::instances::Kinds,
    register: &Register,
) -> Result<Vec<FreightBound>, Vec<String>> {
    let (mut out, mut errors) = (Vec::new(), Vec::new());
    for (system, k) in &d.markets {
        let Some(decl) = k.downcast_ref::<FreightKind>() else { continue };
        let Missing::Present(kind) = kinds.kind(phx_ledger::instruction::name_code(decl.market.key.kind)) else {
            errors.push(format!("{system}'s carriage kind `{}` is no declared market kind", decl.market.key.kind));
            continue;
        };
        let mut carriers = Vec::new();
        for c in decl.carriers {
            if let Some(p) = crate::retail::place_of(d, c) {
                carriers.push(p);
            } else {
                errors.push(format!("`{}` carries by kind `{c}`, which the world does not keep", decl.market.name));
            }
        }
        match (decl.tech)(register) {
            Ok(tech) => out.push(FreightBound { kind, decl: *decl, carriers, tech }),
            Err(e) => errors.push(format!("{system}'s freight: {e}")),
        }
    }
    // A shipment names no kind, so its arrival reads the one kind's reasons.
    if out.len() > 1 {
        errors.push("more than one carriage kind, where a shipment names none".to_owned());
    }
    if errors.is_empty() { Ok(out) } else { Err(errors) }
}

/// A value of the technology's table by its place, which the kind's compile checked.
fn of<T: Copy>(table: &[T], at: usize) -> T {
    let Some(v) = table.get(at).copied() else {
        violation!(clause = "FRT.12", "freight's technology has no value there", place = at);
    };
    v
}

impl World {
    /// A shipper's want of carriage admitted: goods it holds where it stands, free of other claims and covered until
    /// booked, to another zone its mode's segments reach. A want that fails any of these is refused and counted.
    #[clause("FRT.5", "FRT.2", "FRT.11")]
    pub(crate) fn admit_ship(&mut self, step: SubStep, rows: Rows, s: &ShipIntent) {
        if step.ordinal() > SubStep::S5d.ordinal() {
            violation!(clause = "TIME.6", "carriage asked after the day's orders were admitted", step = step.ordinal());
        }
        let Some(row) = self.goods_row(rows, s.row) else { return };
        let Missing::Present(kind) = self.market_kinds.kind(s.kind) else {
            violation!(clause = "MKT.1", "carriage in a market kind never declared", party = row.party.get());
        };
        let Some(bound) = self.trade.freight.iter().find(|f| f.kind == kind) else {
            violation!(
                clause = "FRT.6",
                "carriage asked in a market that is no carriage market",
                party = row.party.get()
            );
        };
        let to = ZoneId::new(s.to);
        let route = self.geo().network.route(s.mode, row.zone, to);
        let good = self.books.ledger.goods.of(GoodKey { product: s.product, grade: s.grade, zone: row.zone });
        let (Some(route), Missing::Present(good)) = (route, good) else {
            self.market_day.tally.refused += 1;
            return;
        };
        if s.qty <= 0 || to == row.zone {
            self.market_day.tally.refused += 1;
            return;
        }
        let Some(qty) = s.qty.checked_mul(row.twins) else {
            capacity_exceeded!("units carried for every twin", i64::MAX, s.qty);
        };
        let units_a_tonne = of(&bound.tech.units_a_tonne, usize::from(s.product));
        let Some(kg) =
            phx_rand::float::floor_to_i64(phx_rand::float::from_i64(qty) * crate::consts::KG_A_TONNE / units_a_tonne)
        else {
            capacity_exceeded!("a consignment's kilograms", i64::MAX, qty);
        };
        let market = self.market_kinds.instance(
            &mut self.markets.made,
            kind,
            (u64::from(row.zone.get()) << u16::BITS) | u64::from(s.mode),
        );
        let (place, slot) = self.books.parties.row(row.party);
        let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, good) {
            Missing::Present(h) => h.quantity.raw(),
            Missing::Absent => 0,
        };
        let pledged = self.books.ledger.liens.pledged(row.party, good);
        let unit = self.books.ledger.instruments.get(good).unit;
        let Ok(cover) = self.books.ledger.covers.cover(row.party, good, Qty::new(qty, unit), held, pledged) else {
            self.market_day.tally.refused += 1;
            return;
        };
        self.market_day.ships.push(Ship { market, party: row.party, good, qty, cover, to, mode: s.mode, route, kg });
        self.market_day.tally.orders += 1;
    }

    /// The carriers of a kind standing at the origins and modes the day's shippers leave by, in one pass over them:
    /// each with its posted price for a lot of carriage, the product it sells, and the room its vehicles have a day
    /// in kilogram-km.
    fn carriers(
        &mut self,
        bound: &FreightBound,
        wanted: &BTreeSet<(ZoneId, u16)>,
    ) -> BTreeMap<(ZoneId, u16), Vec<(Carrier, u16)>> {
        let first = self.books.parties.first_cell_place();
        let mut read: Vec<CarrierRead> = Vec::new();
        for &(place, individuals) in &bound.carriers {
            let agents = || {
                let Some(k) = place.checked_sub(first) else {
                    violation!(clause = "REP.2", "an agent kind's table before the first agents'", place = place);
                };
                usize::from(k)
            };
            let slots: Vec<phx_id::Slot> = if individuals {
                self.books.parties.table(place).slots().collect()
            } else {
                Population::table::<SystemBacking>(self.books.parties.cells(), agents()).slots().collect()
            };
            let store: &mut dyn FactStore = if individuals {
                let t = self.books.parties.table_mut(place);
                t.trace(None);
                t
            } else {
                let t = Population::table_mut::<SystemBacking>(self.books.parties.cells_mut().0, agents());
                t.trace(None);
                t
            };
            for slot in slots {
                let Missing::Present(mode) = store.read(bound.decl.mode, slot) else { continue };
                let Ok(mode) = u16::try_from(mode) else {
                    violation!(clause = "FRT.1", "a carrier's mode beyond the modes' places", value = mode);
                };
                let (sells, price) = (store.read(bound.decl.sells, slot), store.read(bound.decl.price, slot));
                read.push((Rows { place, individuals }, slot, mode, sells, price));
            }
        }
        let mut out: BTreeMap<(ZoneId, u16), Vec<(Carrier, u16)>> = BTreeMap::new();
        for (rows, slot, mode, sells, price) in read {
            let Some(row) = self.goods_row(rows, slot) else { continue };
            if !wanted.contains(&(row.zone, mode)) {
                continue;
            }
            let (Missing::Present(sells), Missing::Present(price)) = (sells, price) else { continue };
            let Ok(sells) = u16::try_from(sells) else {
                violation!(clause = "TEC.4", "a carrier's product beyond the products' places", value = sells);
            };
            let vehicles = self.vehicles(row.party, bound.tech.vehicles);
            let per_unit = of(&bound.tech.tonne_km, usize::from(mode));
            let Some(room) = phx_rand::float::floor_to_i64(
                phx_rand::float::from_i64(vehicles) * per_unit * crate::consts::KG_A_TONNE,
            ) else {
                capacity_exceeded!("a carrier's room", i64::MAX, vehicles);
            };
            if price > 0 && room > 0 {
                let carrier = Carrier { carrier: row.party, price: PriceRaw::from_raw(price), room };
                out.entry((row.zone, mode)).or_default().push((carrier, sells));
            }
        }
        out
    }

    /// The units of vehicles a party holds: its holdings in every class of the chains tagged as vehicles.
    fn vehicles(&self, party: PartyId, tag: u32) -> i64 {
        let (place, slot) = self.books.parties.row(party);
        let arenas = self.books.parties.holder(place);
        self.books
            .ledger
            .chains
            .iter()
            .filter(|c| c.tag == tag)
            .flat_map(|c| c.classes.iter())
            .map(|id| match phx_ledger::holding::holding(arenas, slot, *id) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            })
            .sum()
    }

    /// What each segment can carry today in kilograms: its capacity, or nothing where a catastrophe struck either
    /// end's zone today.
    #[clause("GEO.13", "FRT.7")]
    fn segments_left(&self, day: Day) -> Vec<i64> {
        let geo = self.geo();
        let closed: BTreeSet<ZoneId> = self
            .struck_today(day)
            .into_iter()
            .filter_map(|(_, tile, _)| match geo.zone_of(tile) {
                Missing::Present(z) => Some(z),
                Missing::Absent => None,
            })
            .collect();
        geo.network
            .segments
            .iter()
            .map(|s| {
                if closed.contains(&s.from) || closed.contains(&s.to) {
                    return 0;
                }
                let Some(kg) =
                    i64::try_from(s.tonnes).ok().and_then(|t| t.checked_mul(crate::consts::KG_A_TONNE_WHOLE))
                else {
                    capacity_exceeded!("a segment's kilograms", i64::MAX, s.tonnes);
                };
                kg
            })
            .collect()
    }

    /// 6a: every origin and mode with shippers today met: its carriers, the consignments in an order drawn by lot,
    /// each at the cheapest carrier with room, the segments' capacity binding. A want not booked has its goods'
    /// cover released and is counted.
    #[clause("FRT.6", "FRT.7", "FRT.9", "GEO.13")]
    pub(crate) fn freight_meet(&mut self, day: Day) {
        let ships = std::mem::take(&mut self.market_day.ships);
        if ships.is_empty() {
            return;
        }
        let mut left = self.segments_left(day);
        let mut by_market: BTreeMap<MarketId, Vec<Ship>> = BTreeMap::new();
        for s in ships {
            by_market.entry(s.market).or_default().push(s);
        }
        for bound in self.trade.freight.clone() {
            let wanted: BTreeSet<(ZoneId, u16)> = by_market
                .values()
                .flatten()
                .filter(|s| self.market_kinds.decl(&self.markets.made, s.market).key.kind == bound.decl.market.key.kind)
                .filter_map(|s| s.route.segments.first().map(|_| (self.ship_origin(s), s.mode)))
                .collect();
            let mut carriers = self.carriers(&bound, &wanted);
            let markets: Vec<MarketId> = by_market
                .keys()
                .copied()
                .filter(|m| self.market_kinds.decl(&self.markets.made, *m).key.kind == bound.decl.market.key.kind)
                .collect();
            for market in markets {
                let Some(ships) = by_market.remove(&market) else { continue };
                let Some(origin) = ships.first().map(|s| (self.ship_origin(s), s.mode)) else { continue };
                let here = carriers.remove(&origin).unwrap_or_default();
                self.meet_origin(day, market, &bound, ships, &here, &mut left);
            }
        }
    }

    /// Where a want's goods leave from: the zone of the good it holds.
    fn ship_origin(&self, s: &Ship) -> ZoneId {
        self.good_key(s.good, s.party).zone
    }

    /// The key of a good carried, which every good has.
    fn good_key(&self, good: InstrumentId, party: PartyId) -> GoodKey {
        let Missing::Present(key) = self.books.ledger.goods.key(good) else {
            violation!(clause = "GDS.1", "carriage of no good", party = party.get());
        };
        key
    }

    /// One origin and mode's meeting, its bookings kept for 6d.
    fn meet_origin(
        &mut self,
        day: Day,
        market: MarketId,
        bound: &FreightBound,
        ships: Vec<Ship>,
        here: &[(Carrier, u16)],
        left: &mut [i64],
    ) {
        let decl = self.market_kinds.decl(&self.markets.made, market);
        let Some(stream) = self.streams.named(decl.stream) else {
            violation!(clause = "CHN.1", "a market drawing from a stream never declared", market = market.get());
        };
        let subject = Subject::new(SubjectTag::Market, u64::from(market.get()));
        let mut draws = self.streams.open(&stream, subject, day, SubStep::S6a.ordinal());
        let consignments: Vec<Consignment> = ships
            .iter()
            .map(|s| {
                let km = phx_rand::float::from_u64(s.route.metres) / crate::consts::METRES_PER_KM;
                let Some(need) = phx_rand::float::floor_to_i64(phx_rand::float::from_i64(s.kg) * km * 2.0) else {
                    capacity_exceeded!("a consignment's kilogram-km", i64::MAX, s.kg);
                };
                Consignment { shipper: s.party, need, load: s.kg, segments: s.route.segments.clone() }
            })
            .collect();
        let offers: Vec<Carrier> = here.iter().map(|(c, _)| *c).collect();
        let outcome = phx_market::carriage::carriage(&offers, &consignments, left, &mut draws);
        let t = &mut self.market_day.tally;
        t.refused_bookings += phx_rand::float::len_u64(outcome.no_room.len() + outcome.over_capacity.len());
        let mut booked: BTreeMap<usize, usize> = outcome.booked.into_iter().collect();
        for (i, ship) in ships.into_iter().enumerate() {
            let Some(k) = booked.remove(&i) else {
                self.books.ledger.covers.release(ship.cover);
                continue;
            };
            let Some((carrier, sells)) = here.get(k).copied() else { continue };
            let per = of(&bound.tech.carriage_a_tonne_km, usize::from(ship.mode));
            let tonne_km = phx_rand::float::from_i64(ship.kg) / crate::consts::KG_A_TONNE
                * (phx_rand::float::from_u64(ship.route.metres) / crate::consts::METRES_PER_KM);
            let base = phx_rand::float::from_i64(self.goods_frame.base(sells));
            // A carrier charges whole lots of its carriage, as its price is posted for a lot.
            let exact = tonne_km * per / base;
            let Some(whole) = phx_rand::float::floor_to_i64(exact) else {
                capacity_exceeded!("a trip's lots of carriage", i64::MAX, ship.kg);
            };
            let lots = if phx_rand::float::from_i64(whole) < exact { whole + 1 } else { whole };
            self.market_day.booked.push(Booked { ship, carrier: carrier.carrier, price: carrier.price, lots });
        }
    }

    /// 6d: each booking's freight owed its carrier, due today under the kind's reason: the shipper's money for the
    /// trip's lots at the carrier's price.
    #[clause("FRT.6", "SET.1")]
    pub(crate) fn freight_trade(&mut self, day: Day) {
        for b in std::mem::take(&mut self.market_day.booked) {
            let decl = self.market_kinds.decl(&self.markets.made, b.ship.market);
            let Some(bound) = self.trade.freight.iter().find(|f| f.decl.market.key.kind == decl.key.kind) else {
                continue;
            };
            let code = phx_ledger::instruction::name_code(bound.decl.paid);
            let Missing::Present(reason) = self.books.ledger.reasons.coded(code) else {
                violation!(clause = "SET.1", "freight under a reason never declared", market = b.ship.market.get());
            };
            let Some(amount) = b.lots.checked_mul(b.price.raw()) else {
                capacity_exceeded!("a trip's freight", i64::MAX, b.lots);
            };
            let ccy = self.books.ledger.instruments.get(b.ship.good).ccy;
            let mut legs = Vec::new();
            self.books.pay_into(b.ship.party, b.carrier, (amount, ccy), &mut legs);
            let instruction = Instruction {
                id: self.books.ledger.next_id(day),
                reason,
                trade_day: day,
                settle_day: day,
                legs,
                pays: Missing::Absent,
                covers: Vec::new(),
            };
            self.market_day.freight.push((instruction, b));
        }
    }

    /// Stage 7, after the trades: each freight paid, all or none; a paid one pledges the goods to the carrier and sets
    /// them on their way, due after the route's run at the mode's speed and loading at each end; an unpaid one releases
    /// the goods and is counted.
    #[clause("FRT.3", "FRT.6", "FRT.11")]
    pub(crate) fn freight_settle(&mut self, day: Day, step: SubStep) {
        let freight = std::mem::take(&mut self.market_day.freight);
        if freight.is_empty() {
            return;
        }
        let (list, bookings): (Vec<Instruction>, Vec<Booked>) = freight.into_iter().unzip();
        let books = &mut self.books;
        let results = books.ledger.settle(&mut books.parties, ApplyAt::Day(step), list, self.audit.stream());
        for (r, b) in results.into_iter().zip(bookings) {
            self.books.ledger.covers.release(b.ship.cover);
            if r.is_err() {
                self.market_day.tally.refused_bookings += 1;
                continue;
            }
            let Some(bound) = self.trade.freight.iter().find(|f| f.kind == self.market_kind_of(b.ship.market)) else {
                continue;
            };
            let speed = of(&bound.tech.metres_a_day, usize::from(b.ship.mode));
            let loading = of(&bound.tech.loading_days, usize::from(b.ship.mode));
            let days = phx_market::carriage::transit_days(b.ship.route.metres, speed, loading);
            let to = GoodKey { zone: b.ship.to, ..self.good_key(b.ship.good, b.ship.party) };
            let (place, slot) = self.books.parties.row(b.ship.party);
            let held = match phx_ledger::holding::holding(self.books.parties.holder(place), slot, b.ship.good) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            };
            let bound_units = self.books.ledger.covers.committed(b.ship.party, b.ship.good);
            let lien =
                self.books.ledger.liens.pledge(b.ship.party, b.ship.good, b.ship.qty, b.carrier, held, bound_units);
            let Some(period) = u16::try_from(days).ok().and_then(phx_core::calendar::period::Period::days) else {
                capacity_exceeded!("a shipment's days", u16::MAX, days);
            };
            let arrives = self.calendar.plus(day, period);
            let shipment = Shipment {
                owner: b.ship.party,
                carrier: b.carrier,
                from: b.ship.good,
                to,
                qty: b.ship.qty,
                lien,
                left: day,
                arrives,
            };
            let _ = self.books.ledger.goods.dispatch(shipment);
            self.market_day.tally.shipments += 1;
        }
    }

    /// The kind a market is an instance of.
    fn market_kind_of(&self, market: MarketId) -> u16 {
        let decl = self.market_kinds.decl(&self.markets.made, market);
        match self.market_kinds.kind(phx_ledger::instruction::name_code(decl.key.kind)) {
            Missing::Present(k) => k,
            Missing::Absent => violation!(clause = "MKT.1", "a market of no declared kind", market = market.get()),
        }
    }

    /// 5a: the day's arrivals, in the order they arrive and were numbered: each lien released and the goods moved,
    /// the owner's units used up where they left under the kind's `shipped` and made where they arrive under its
    /// `arrived`, at the cost the units leaving carried.
    #[clause("FRT.3", "FRT.6", "FRT.8", "GDS.10")]
    pub(crate) fn arrivals(&mut self, day: Day) {
        let due = self.books.ledger.goods.arriving(day);
        let Some(bound) = self.trade.freight.first().cloned() else {
            if !due.is_empty() {
                violation!(clause = "FRT.3", "shipments in a world that declares no carriage");
            }
            return;
        };
        let reason =
            |w: &World, name: &str| match w.books.ledger.reasons.coded(phx_ledger::instruction::name_code(name)) {
                Missing::Present(r) => r,
                Missing::Absent => violation!(clause = "SET.1", "an arrival under a reason never declared"),
            };
        let (shipped, arrived) = (reason(self, bound.decl.shipped), reason(self, bound.decl.arrived));
        for (n, s) in due {
            let _ = self.books.ledger.liens.release(s.lien);
            let (place, slot) = self.books.parties.row(s.owner);
            let basis = |w: &World| match phx_ledger::holding::basis(w.books.parties.holder(place), slot, s.from) {
                Missing::Present(b) => b,
                Missing::Absent => 0,
            };
            let before = basis(self);
            let unit = self.books.ledger.instruments.get(s.from).unit;
            let leaving = LegRec {
                party: s.owner,
                account: AccountRef::Instrument(s.from),
                qty: -s.qty,
                denom: Denom::Unit(unit),
                kind: LegKind::Transformation { source: Source::Carried(n), cost: 0 },
            };
            self.apply_carried(day, shipped, leaving);
            let cost = before - basis(self);
            let to = self.good(s.to);
            let arriving = LegRec {
                party: s.owner,
                account: AccountRef::Instrument(to),
                qty: s.qty,
                denom: Denom::Unit(unit),
                kind: LegKind::Transformation { source: Source::Carried(n), cost },
            };
            self.apply_carried(day, arrived, arriving);
            self.market_day.tally.arrivals += 1;
        }
    }

    /// One leg of an arrival applied alone; goods pledged and set on their way are still the owner's, so it cannot
    /// fail.
    fn apply_carried(&mut self, day: Day, reason: phx_ledger::instruction::ReasonId, leg: LegRec) {
        let instruction = Instruction {
            id: self.books.ledger.next_id(day),
            reason,
            trade_day: day,
            settle_day: day,
            legs: vec![leg],
            pays: Missing::Absent,
            covers: Vec::new(),
        };
        if let Err(f) = self.books.apply(ApplyAt::Day(SubStep::S5a), instruction, self.audit.stream()) {
            violation!(clause = "FRT.3", "goods in transit their owner no longer holds", party = f.party.get());
        }
    }
}
