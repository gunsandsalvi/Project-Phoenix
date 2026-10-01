//! Freight on the core. Each firm selling carriage carries by one mode, drawn at the opening by the modes' shares of
//! carriage's employment. On its shipping schedule a firm holding its own product beyond what its cover asks weighs
//! carrying whole lots of it to each other region of its country: what a lot fetches there against here and the freight
//! out by each mode posted at its region that joins the two, the widest gap first. The carriage meeting at each origin
//! and mode books each consignment with the cheapest carrier with room for its trip out and back, refusing one whose
//! route crosses a segment already carrying its day's tonnes; the freight is paid with the day's flows, and once it
//! settles the goods are pledged to the carrier, on their way until their day, when they arrive at the cost they left
//! with.

use std::collections::BTreeMap;

use phx_core::flows::{Denom, Flow};
use phx_core::goods::{Bound, Carriage, Good, Held, Shipment, Shipments};
use phx_core::{StreamDef, SubStep, WorldStreams};
use phx_geo::GeoState;
use phx_id::{CountryId, Day, PartyKey};
use phx_macros::{clause, opening};
use phx_market::carriage::{Carrier, Consignment, FreightTech, carriage, freight, transit_days};
use phx_num::{Missing, PriceRaw, violation};
use phx_rand::float::{floor_to_i64, from_i64, from_u64};
use phx_rand::{Subject, SubjectTag};

use crate::consts::reason::{ARRIVED, CARRIED, SHIPPED};
use crate::core::Core;
use crate::core_accounts::Line;

/// A trip booked and awaiting its freight's payment: its shipper and carrier, the goods it leaves and arrives as,
/// their units, its day of arrival, the freight, the units of carriage it buys and its currency.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Booking {
    pub shipper: PartyKey,
    pub carrier: PartyKey,
    pub from: u16,
    pub to: u16,
    pub units: i64,
    pub arrives: u32,
    pub freight: i64,
    pub carriage: i64,
    pub ccy: u8,
}

/// What freight did on a day: the shippers who weighed carrying and those who chose to, the consignments booked, those
/// no carrier had room for and those a full segment refused, the trips whose freight failed, the units that left and
/// arrived, and the shipments on their way at the close.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct FreightDay {
    pub day: u32,
    pub weighed: u64,
    pub chose: u64,
    pub booked: u64,
    pub no_room: u64,
    pub over_capacity: u64,
    pub unpaid: u64,
    pub departed: i64,
    pub arrived: i64,
    pub on_the_way: u64,
}

/// Freight: its technology, each carrier's mode, each mode's route and its length between every two regions' market
/// zones,
/// the days between a shipper's decisions and the day freight opened, the shipments on their way, today's bookings, the
/// days' records.
#[derive(Debug, Default, phx_macros::Saved)]
pub struct Freight {
    tech: FreightTech,
    pub modes: BTreeMap<PartyKey, u16>,
    lengths: BTreeMap<(u16, u32, u32), u64>,
    routes: BTreeMap<(u16, u32, u32), Vec<usize>>,
    shipping_days: u32,
    began: u32,
    pub shipments: Option<Shipments>,
    bookings: Vec<Booking>,
    pub days: Vec<FreightDay>,
    today: FreightDay,
}

/// The day's carriage offers at each region and mode: each carrier, its posted price for a lot and its room today.
type Offers = BTreeMap<(u32, u16), Vec<(PartyKey, i64, i64)>>;

/// A consignment weighed at an origin: its shipper, the good it leaves as and arrives as, its units, the tonnes and
/// the trip's tonne-km, its route's segments and days, and the freight it would pay a lot.
struct Weighed {
    shipper: PartyKey,
    from: u16,
    to: u16,
    units: i64,
    tonnes: f64,
    km: f64,
    segments: Vec<usize>,
    days: u32,
}

impl Core {
    /// Freight opened: its technology, each carrier's mode drawn by the modes' shares of carriage's employment, and
    /// each mode's route lengths between the regions' market zones.
    ///
    /// # Errors
    /// Freight's tables unread.
    #[clause("FRT.1", "FRT.4", "GEN.3")]
    #[opening]
    pub(crate) fn open_freight(
        &mut self,
        (geo, register, streams): (&GeoState, &phx_core::Register, &WorldStreams),
        regions: &[CountryId],
        today: Day,
    ) -> Result<(), String> {
        let tech = sys_frt::tech(register)?;
        let shares = register.table1(sys_frt::MODE_SHARE.id)?.values().to_vec();
        let whole: i64 = shares.iter().sum();
        let shipping_days = u32::try_from(register.count(sys_frt::SHIPPING_DAYS.id)?).map_err(|e| e.to_string())?;
        let zones = geo.market_zones();
        let zone_of = |r: usize| match zones.get(r) {
            Some(Missing::Present(z)) => Some(*z),
            _ => None,
        };
        let all = geo.network.lengths();
        let mut lengths = BTreeMap::new();
        let mut routes = BTreeMap::new();
        for ((mode, from, to), metres) in &all {
            for a in (0..regions.len()).filter(|a| zone_of(*a) == Some(*from)) {
                for b in (0..regions.len()).filter(|b| zone_of(*b) == Some(*to)) {
                    let (Ok(ra), Ok(rb)) = (u32::try_from(a), u32::try_from(b)) else { continue };
                    let Some(route) = geo.network.route(*mode, *from, *to) else { continue };
                    lengths.insert((*mode, ra, rb), *metres);
                    routes.insert((*mode, ra, rb), route.segments);
                }
            }
        }
        let mut modes = BTreeMap::new();
        if let Some(firm) = self.bound.kinds.firm
            && whole > 0
        {
            for slot in self.firm_slots(firm) {
                let Some(f) = self.goods_firm(regions, firm, slot) else { continue };
                if f.product != tech.carriage_product {
                    continue;
                }
                let Some(id) = self.kinds.get(firm).and_then(|k| k.parties.id(slot)) else { continue };
                let subject = Subject::new(SubjectTag::Party, id.get());
                let mut d = streams.open(&sys_frt::OpeningStream::DECL, subject, today, 0);
                let at = phx_rand::uniform::below_u64(&mut d, whole.unsigned_abs());
                let mut sum = 0;
                for (mode, share) in (0_u16..).zip(&shares) {
                    sum += share.unsigned_abs();
                    if at < sum {
                        modes.insert(f.key, mode);
                        break;
                    }
                }
            }
        }
        self.freight = Freight {
            tech,
            modes,
            lengths,
            routes,
            shipping_days,
            began: today.get(),
            shipments: Some(Shipments::new(today.succ(), phx_core::capacity::WHEEL_DAYS)),
            ..Freight::default()
        };
        Ok(())
    }

    /// The day's arrivals: each shipment due today leaves its pledge where it left and is made where it arrives.
    #[clause("FRT.6", "FRT.8", "GDS.10")]
    pub(crate) fn arrive_shipments(&mut self, day: Day, moved: &mut Vec<Flow>) {
        let Some(shipments) = self.freight.shipments.as_mut() else { return };
        let before = moved.len();
        shipments.arrive(day, &mut self.goods.stocks, Carriage { shipped: SHIPPED, arrived: ARRIVED }, moved);
        let arrived: i64 = moved.iter().skip(before).filter(|f| f.reason == ARRIVED).map(|f| f.amount).sum();
        self.freight.today.arrived += arrived;
    }

    /// On its shipping schedule, each firm holding more of its own product than its cover asks weighs carrying whole
    /// lots of the rest to another region of its country; the day's carriage meetings book what the shippers chose.
    #[clause("FRT.5", "FRT.6", "FRT.7", "FRT.9", "GEO.13", "MND.20")]
    pub(crate) fn ship(&mut self, ctx: &crate::core_goods::GoodsCtx<'_>, geo: &GeoState, day: Day) {
        let days = self.freight.shipping_days;
        let Some(firm) = self.bound.kinds.firm else { return };
        if days == 0 {
            return;
        }
        let deciding = self.point(|p| p.ship, &sys_frt::points::SHIP);
        let offers = self.carriage_offers();
        let mut by_origin: BTreeMap<(u32, u16), Vec<Weighed>> = BTreeMap::new();
        for slot in self.firm_slots(firm) {
            if !(day.get() + slot.get()).is_multiple_of(days) {
                continue;
            }
            let Some(f) = self.goods_firm(ctx.regions, firm, slot) else { continue };
            let country = CountryId::new(u8::try_from(f.country).unwrap_or(u8::MAX));
            if !self.is_stored(f.product) || !ctx.calendar.is_business(country, day) {
                continue;
            }
            if let Some((mode, w)) = self.weigh(ctx, (&f, deciding), &offers) {
                by_origin.entry((f.region, mode)).or_default().push(w);
            }
        }
        for ((origin, mode), weighed) in by_origin {
            let Some(ccy) = country_of(ctx.regions, origin).and_then(|c| u8::try_from(c).ok()) else {
                continue;
            };
            let carriers = offers.get(&(origin, mode)).map_or(&[][..], Vec::as_slice);
            self.meet_carriage((ctx.streams, geo), (origin, mode, ccy), (carriers, &weighed), day);
        }
    }

    /// A shipper's best trip: the other region of its country, and the mode posted at its region that joins them,
    /// where a lot fetches most above its own price less the freight out, when its decision is to carry.
    fn weigh(
        &mut self,
        ctx: &crate::core_goods::GoodsCtx<'_>,
        (f, deciding): (&crate::core_goods::Firm, crate::core_decide::Bound<sys_frt::points::ShipIn, bool>),
        offers: &Offers,
    ) -> Option<(u16, Weighed)> {
        let lot = self.lot(f.product);
        let lot_units = floor_to_i64(lot)?;
        let expected =
            from_i64(self.record_word(usize::from(f.key.kind()), f.key.slot(), crate::consts::firm::EXPECTED)?)
                / crate::consts::firm::PART_ONE;
        let free = self.free_held(f.key, f.product, f.region);
        let spare = from_i64(free) - expected * self.goods.cover;
        let lots = floor_to_i64(spare / lot)?;
        if lots <= 0 {
            return None;
        }
        self.freight.today.weighed += 1;
        let here = from_i64(f.price);
        let carriage_lot = self.lot(self.freight.tech.carriage_product);
        let mut best: Option<(f64, u16, u32, f64, u64)> = None;
        for ((product, region), there) in self.goods.marks.range((f.product, 0)..=(f.product, u32::MAX)) {
            if *product != f.product || *region == f.region || country_of(ctx.regions, *region) != Some(f.country) {
                continue;
            }
            for ((_, mode), carriers) in offers.range((f.region, 0)..=(f.region, u16::MAX)) {
                let mode = *mode;
                let lowest = lowest_price(carriers);
                let (Some(metres), Some(price)) =
                    (self.freight.lengths.get(&(mode, f.region, *region)).copied(), lowest)
                else {
                    continue;
                };
                let Some(out) =
                    freight(&self.freight.tech, (f.product, lot_units), (mode, metres), (price, carriage_lot))
                else {
                    continue;
                };
                let gap = there - here - out;
                if best.is_none_or(|(g, ..)| gap > g) {
                    best = Some((gap, mode, *region, out, metres));
                }
            }
        }
        let (gap, mode, to_region, out, metres) = best?;
        let (there, freight_a_lot) = (gap + here + out, out);
        let input = sys_frt::points::ShipIn {
            there: floor_to_i64(there)?,
            here: floor_to_i64(here)?,
            freight: floor_to_i64(freight_a_lot)?,
        };
        if !self.decide(deciding, f.key, |_| input) {
            return None;
        }
        self.freight.today.chose += 1;
        let units = lots * lot_units;
        let units_a_tonne = self.freight.tech.units_a_tonne.get(usize::from(f.product)).copied()?;
        let route = self.route_of(mode, (f.region, to_region))?;
        let speed = self.freight.tech.metres_a_day.get(usize::from(mode)).copied()?;
        let loading = self.freight.tech.loading_days.get(usize::from(mode)).copied()?;
        let from = self.good_unit(f.product, f.region);
        let to = self.good_unit(f.product, to_region);
        Some((
            mode,
            Weighed {
                shipper: f.key,
                from,
                to,
                units,
                tonnes: from_i64(units) / units_a_tonne,
                km: from_u64(metres) / phx_market::consts::METRES_A_KM,
                segments: route,
                days: transit_days(metres, speed, loading),
            },
        ))
    }

    /// The carriage meeting at an origin and mode: its carriers' posted prices and room, the consignments leaving,
    /// each booked with the cheapest carrier with room, its freight owed and its goods committed.
    fn meet_carriage(
        &mut self,
        (streams, geo): (&WorldStreams, &GeoState),
        (origin, mode, ccy): (u32, u16, u8),
        (carriers, weighed): (&[(PartyKey, i64, i64)], &[Weighed]),
        day: Day,
    ) {
        let offers: Vec<Carrier> = carriers
            .iter()
            .filter_map(|(k, price, room)| {
                let id = self.kinds.get(usize::from(k.kind()))?.parties.id(k.slot())?;
                Some(Carrier { carrier: id, price: PriceRaw::from_raw(*price), room: *room })
            })
            .collect();
        let consignments: Vec<Consignment> = weighed
            .iter()
            .filter_map(|w| {
                let id = self.kinds.get(usize::from(w.shipper.kind()))?.parties.id(w.shipper.slot())?;
                let need = floor_to_i64((2.0 * w.tonnes * w.km).ceil())?;
                let load = floor_to_i64(w.tonnes.ceil())?;
                Some(Consignment { shipper: id, need, load, segments: w.segments.clone() })
            })
            .collect();
        let mut left: Vec<i64> =
            geo.network.segments.iter().map(|s| i64::try_from(s.tonnes).unwrap_or(i64::MAX)).collect();
        let subject = Subject::new(SubjectTag::Region, u64::from(origin) << u16::BITS | u64::from(mode));
        let Some(lot) = streams.named(sys_frt::LotStream::DECL.name) else {
            violation!(clause = "FRT.7", "the carriage lot's stream is not declared");
        };
        let mut d = streams.open(&lot, subject, day, SubStep::S6a.ordinal());
        let outcome = carriage(&offers, &consignments, &mut left, &mut d);
        self.freight.today.no_room += phx_rand::float::len_u64(outcome.no_room.len());
        self.freight.today.over_capacity += phx_rand::float::len_u64(outcome.over_capacity.len());
        let carriage_lot = self.lot(self.freight.tech.carriage_product);
        let Some(per_tonne_km) = self.freight.tech.carriage_a_tonne_km.get(usize::from(mode)).copied() else {
            violation!(clause = "FRT.7", "a mode with no carriage a tonne-km", mode = mode);
        };
        for (i, k) in outcome.booked {
            let (Some(w), Some((carrier, price, _))) = (weighed.get(i), carriers.get(k).copied()) else { continue };
            // Carriage is sold in whole units, so a trip buys the whole units its tonne-km take, priced as a sale is.
            let (Some(units), Some(lot)) =
                (floor_to_i64((w.tonnes * w.km * per_tonne_km).ceil()), floor_to_i64(carriage_lot))
            else {
                continue;
            };
            let owed = phx_market::retail::paid(units, price, lot);
            if self.goods.stocks.bind(w.shipper, w.from, w.units, (Bound::Free, Bound::Committed)).is_err() {
                continue;
            }
            self.pending.push(Flow {
                payer: w.shipper,
                payee: carrier,
                amount: owed,
                source: u32::try_from(self.freight.bookings.len()).unwrap_or(u32::MAX),
                denomination: Denom::money(ccy),
                reason: CARRIED,
                order: 0,
            });
            let arrives = day.get() + w.days;
            self.freight.bookings.push(Booking {
                shipper: w.shipper,
                carrier,
                from: w.from,
                to: w.to,
                units: w.units,
                arrives,
                freight: owed,
                carriage: units,
                ccy,
            });
            self.freight.today.booked += 1;
        }
    }

    /// After the day's settlement, each trip whose freight was paid departs, its goods pledged to its carrier until
    /// they arrive, the freight the carrier's revenue and the shipper's cost; a trip whose freight failed releases its
    /// goods. The day's record is kept.
    #[clause("FRT.6", "FRT.9", "ACC.13")]
    pub(crate) fn depart_shipments(&mut self, day: Day, failed: &[Flow]) {
        let mut unpaid: BTreeMap<(PartyKey, PartyKey, i64), u32> = BTreeMap::new();
        for f in failed.iter().filter(|f| f.reason == CARRIED && f.denomination.is_money()) {
            *unpaid.entry((f.payer, f.payee, f.amount)).or_insert(0) += 1;
        }
        for b in std::mem::take(&mut self.freight.bookings) {
            if let Some(n) = unpaid.get_mut(&(b.shipper, b.carrier, b.freight)).filter(|n| **n > 0) {
                *n -= 1;
                let _ = self.goods.stocks.bind(b.shipper, b.from, b.units, (Bound::Committed, Bound::Free));
                self.freight.today.unpaid += 1;
                continue;
            }
            let s = Shipment {
                owner: b.shipper,
                carrier: b.carrier,
                from: b.from,
                to: b.to,
                arrives: b.arrives,
                units: b.units,
            };
            let Some(shipments) = self.freight.shipments.as_mut() else { continue };
            if shipments.depart(&mut self.goods.stocks, s, (Bound::Committed, day)).is_err() {
                violation!(clause = "FRT.6", "a paid trip whose goods are gone", shipper = b.shipper.word());
            }
            self.freight.today.departed += b.units;
            self.recognise(b.carrier, Line::Revenue, b.freight);
            if self.accounts.opening.contains_key(b.carrier) {
                self.accounts.revenue += i128::from(b.freight);
            }
            self.recognise(b.shipper, Line::ServicesUsed, b.freight);
            let carriage = self.freight.tech.carriage_product;
            let purpose = (b.shipper.kind(), crate::core_stats::Purchase::Inputs);
            self.record_sale(b.ccy, purpose, (carriage, b.freight, b.carriage));
        }
        if let Some(shipments) = self.freight.shipments.as_mut() {
            shipments.close_day();
            self.freight.today.on_the_way = phx_rand::float::len_u64(shipments.on_the_way());
        }
        let mut record = std::mem::take(&mut self.freight.today);
        record.day = day.get();
        self.freight.days.push(record);
    }

    /// The basis between places against freight: for each storable product marked at two regions of a country, the
    /// gap between its marks a lot, with the least freight of a lot between them by a mode posted at either.
    #[clause("GDS.11", "FRT.10")]
    #[must_use]
    pub fn basis(&self, regions: &[CountryId]) -> Vec<(f64, f64)> {
        let carriage_lot = self.lot(self.freight.tech.carriage_product);
        let offers = self.carriage_offers();
        let lowest = |at: u32, mode: u16| lowest_price(offers.get(&(at, mode))?);
        let mut out = Vec::new();
        for ((product, from), here) in &self.goods.marks {
            if !self.is_stored(*product) {
                continue;
            }
            let Some(lot) = floor_to_i64(self.lot(*product)) else { continue };
            for ((p, to), there) in self.goods.marks.range((*product, 0)..=(*product, u32::MAX)) {
                if p != product || to <= from || country_of(regions, *to) != country_of(regions, *from) {
                    continue;
                }
                let least = [*from, *to]
                    .iter()
                    .flat_map(|at| offers.range((*at, 0)..=(*at, u16::MAX)).map(move |((_, m), _)| (*m, *at)))
                    .filter_map(|(mode, at)| {
                        let metres = self.freight.lengths.get(&(mode, *from, *to)).copied()?;
                        let price = lowest(at, mode)?;
                        freight(&self.freight.tech, (*product, lot), (mode, metres), (price, carriage_lot))
                    })
                    .reduce(|a, b| if b < a { b } else { a });
                if let Some(f) = least {
                    out.push(((there - here).abs(), f));
                }
            }
        }
        out
    }

    /// Each storable product's stock at each region against what its sellers there expect to sell: the days of sales
    /// their free units cover.
    #[clause("GDS.11")]
    #[must_use]
    pub fn cover_by_place(&self, regions: &[CountryId]) -> BTreeMap<(u16, u32), f64> {
        let mut held: BTreeMap<(u16, u32), (f64, f64)> = BTreeMap::new();
        let Some(firm) = self.bound.kinds.firm else { return BTreeMap::new() };
        for slot in self.firm_slots(firm) {
            let Some(f) = self.goods_firm(regions, firm, slot) else { continue };
            if !self.is_stored(f.product) {
                continue;
            }
            let Some(expected) = self.record_word(firm, slot, crate::consts::firm::EXPECTED) else { continue };
            let e = held.entry((f.product, f.region)).or_insert((0.0, 0.0));
            e.0 += from_i64(self.free_held(f.key, f.product, f.region));
            e.1 += from_i64(expected) / crate::consts::firm::PART_ONE;
        }
        held.into_iter().filter(|(_, (_, e))| *e > 0.0).map(|(k, (h, e))| (k, h / e)).collect()
    }

    /// The day's carriage offers by region and mode: each carrier with its posted price for a lot of carriage and its
    /// vehicles' room today in tonne-km, read once for the day's shippers and meetings.
    fn carriage_offers(&self) -> Offers {
        let mut out = Offers::new();
        let (Some(chain), Ok(kind)) = (
            self.plant.chains.get(usize::try_from(self.freight.tech.vehicles).unwrap_or(usize::MAX)),
            u16::try_from(self.freight.tech.vehicles),
        ) else {
            return out;
        };
        for (k, mode) in &self.freight.modes {
            let Some(a_day) = self.freight.tech.tonne_km.get(usize::from(*mode)).copied() else { continue };
            let at = self.record_word(usize::from(k.kind()), k.slot(), crate::consts::firm::REGION);
            let price = self.record_word(usize::from(k.kind()), k.slot(), crate::consts::firm::PRICE);
            let (Some(at), Some(price)) = (at.and_then(|r| u32::try_from(r).ok()), price) else { continue };
            let vehicles = phx_core::units::capacity(&self.goods.stocks, &self.goods.units, *k, (kind, None), chain);
            let Some(room) = floor_to_i64(vehicles * a_day) else { continue };
            out.entry((at, *mode)).or_default().push((*k, price, room));
        }
        out
    }

    /// A mode's route between two regions' market zones, by its segments.
    fn route_of(&self, mode: u16, (from, to): (u32, u32)) -> Option<Vec<usize>> {
        self.freight.routes.get(&(mode, from, to)).cloned()
    }

    /// A party's free units of a product at a region.
    fn free_held(&self, holder: PartyKey, product: u16, region: u32) -> i64 {
        phx_core::goods::find(&self.goods.units, Held::Good(Good { product, grade: 0, zone: region }))
            .and_then(|u| self.goods.stocks.holding(holder, u))
            .map_or(0, phx_core::goods::Holding::free)
    }

    /// A product's unit at a region, issued if new.
    fn good_unit(&mut self, product: u16, region: u32) -> u16 {
        let good = Held::Good(Good { product, grade: 0, zone: region });
        if let Some(u) = phx_core::goods::find(&self.goods.units, good) {
            return u;
        }
        let traits = self.good_traits(product);
        phx_core::goods::unit(&mut self.goods.units, good, traits)
    }
}

/// The lowest price carriage is posted at by the carriers with room.
fn lowest_price(carriers: &[(PartyKey, i64, i64)]) -> Option<i64> {
    carriers.iter().filter(|(_, _, room)| *room > 0).map(|(_, p, _)| *p).reduce(|a, b| if b < a { b } else { a })
}

/// The country a region lies in, by its place.
fn country_of(regions: &[CountryId], region: u32) -> Option<usize> {
    regions.get(usize::try_from(region).ok()?).map(|c| usize::from(c.get()))
}
