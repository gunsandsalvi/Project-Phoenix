//! The carriage meeting at an origin and a mode: shippers in an order drawn by lot, each at the cheapest carrier with
//! room for its consignment, equal prices by lot; a consignment whose route crosses a segment already carrying its
//! day's capacity is refused, never repriced, and one no route serves fails.

use phx_geo::transport::{Route, SegmentUse, Segments};
use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded};
use phx_rand::Draws;
use phx_rand::uniform::below_u64;

/// Freight's technology as its system compiles it from the register: the chain its vehicles are held in; by mode,
/// the tonne-km a unit of vehicles carries a day, the metres it runs a day, the days loading takes at each end and the
/// units of carriage a tonne-km takes; by product, the units of a good in a tonne; and the product carriage is sold as.
#[clause("FRT.1", "FRT.12")]
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub struct FreightTech {
    pub vehicles: u32,
    pub tonne_km: Vec<f64>,
    pub metres_a_day: Vec<u64>,
    pub loading_days: Vec<u32>,
    pub carriage_a_tonne_km: Vec<f64>,
    pub units_a_tonne: Vec<f64>,
    pub carriage_product: u16,
}

/// The freight of a lot of a good carried `metres` by a mode: its tonnes times the km, in units of carriage, at the
/// carriage market's price for a lot of it.
#[clause("FRT.5", "FRT.12")]
#[must_use]
pub fn freight(
    tech: &FreightTech,
    (product, lot): (u16, i64),
    (mode, metres): (u16, u64),
    (price, carriage_lot): (i64, f64),
) -> Option<f64> {
    let units_a_tonne = tech.units_a_tonne.get(usize::from(product)).copied()?;
    let per_tonne_km = tech.carriage_a_tonne_km.get(usize::from(mode)).copied()?;
    let tonne_km =
        phx_rand::float::from_i64(lot) / units_a_tonne * phx_rand::float::from_u64(metres) / crate::consts::METRES_A_KM;
    Some(tonne_km * per_tonne_km / carriage_lot * phx_rand::float::from_i64(price))
}

/// A carrier's offer at the origin: its posted price and the room its vehicles have left today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carrier {
    pub carrier: PartyId,
    pub price: PriceRaw,
    pub room: i64,
}

/// A shipper's consignment: the room it takes of a carrier's vehicles, the load it puts on each segment of its route,
/// and the route the network gives it today, read where the network keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Consignment<'r> {
    pub shipper: PartyId,
    pub need: i64,
    pub load: i64,
    pub route: Missing<Route<'r>>,
}

/// The meeting's outcome: each consignment booked with its carrier, those no carrier had room for, those a full
/// segment refused, and those no route served.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CarriageDay {
    pub booked: Vec<(usize, usize)>,
    pub no_room: Vec<usize>,
    pub over_capacity: Vec<usize>,
    pub no_route: Vec<usize>,
}

/// The days a shipment takes over a route of `metres` at a mode's `metres_a_day`, the part of a day's run a whole day,
/// plus the loading days at each end; never none, since no transport is instantaneous.
#[clause("FRT.3", "FRT.11")]
#[must_use]
pub fn transit_days(metres: u64, metres_a_day: u64, loading_days: u32) -> u32 {
    let run = if metres_a_day == 0 {
        phx_num::violation!(clause = "FRT.12", "a mode that runs no distance a day");
    } else {
        metres.div_ceil(metres_a_day)
    };
    let Ok(run) = u32::try_from(run) else {
        capacity_exceeded!("a shipment's days", u32::MAX, run);
    };
    let days = run + 2 * loading_days;
    if days == 0 {
        phx_num::violation!(clause = "FRT.11", "a shipment that takes no time");
    }
    days
}

/// A place drawn by lot below `n`.
fn lot(draws: &mut Draws, n: usize) -> usize {
    let Ok(i) = usize::try_from(below_u64(draws, phx_rand::float::len_u64(n))) else {
        capacity_exceeded!("places drawn by lot", usize::MAX, n);
    };
    i
}

/// The carriage meeting over the carriers at an origin and the consignments leaving it by their mode, each booked on
/// its route's segments' loads today, which every meeting of the day shares.
#[clause("FRT.6", "FRT.7", "FRT.9", "GEO.13")]
pub fn carriage(
    carriers: &[Carrier],
    consignments: &[Consignment<'_>],
    (segments, used): (&Segments, &mut SegmentUse),
    (draws, today): (&mut Draws, Day),
) -> CarriageDay {
    let mut room: Vec<i64> = carriers.iter().map(|c| c.room).collect();
    let mut order: Vec<usize> = (0..consignments.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, lot(draws, i + 1));
    }
    let mut day = CarriageDay::default();
    for i in order {
        let Some(c) = consignments.get(i) else { continue };
        let Missing::Present(route) = c.route else {
            day.no_route.push(i);
            continue;
        };
        // A load past what a segment's count can hold is past every segment's capacity.
        let fits = u32::try_from(c.load).ok().filter(|load| used.room(segments, route, *load, today));
        let Some(load) = fits else {
            day.over_capacity.push(i);
            continue;
        };
        let open: Vec<usize> = (0..carriers.len()).filter(|k| room.get(*k).is_some_and(|r| *r >= c.need)).collect();
        let Some(low) = open
            .iter()
            .filter_map(|k| carriers.get(*k))
            .map(|k| k.price.raw())
            .reduce(|a, b| if b < a { b } else { a })
        else {
            day.no_room.push(i);
            continue;
        };
        let cheapest: Vec<usize> =
            open.into_iter().filter(|k| carriers.get(*k).is_some_and(|x| x.price.raw() == low)).collect();
        let Some(&k) = cheapest.get(lot(draws, cheapest.len())) else { continue };
        if let Some(r) = room.get_mut(k) {
            *r -= c.need;
        }
        used.book(segments, route, load, today);
        day.booked.push((i, k));
    }
    day
}

#[cfg(test)]
#[path = "carriage_tests.rs"]
mod tests;
