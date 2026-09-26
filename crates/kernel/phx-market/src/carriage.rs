//! The carriage meeting at an origin and a mode: shippers in an order drawn by lot, each at the cheapest carrier with
//! room for its consignment, equal prices by lot; a consignment whose route crosses a segment already carrying its
//! day's capacity is refused, never repriced.

use phx_id::PartyId;
use phx_macros::clause;
use phx_num::{PriceRaw, capacity_exceeded};
use phx_rand::Draws;
use phx_rand::uniform::below_u64;

use crate::market::MarketDecl;

/// Freight's technology as its system compiles it from the register: the chain its vehicles are held in; by mode,
/// the tonne-km a unit of vehicles carries a day, the metres it runs a day, the days loading takes at each end and the
/// units of carriage a tonne-km takes; and by product, the units of a good in a tonne.
#[clause("FRT.1", "FRT.12")]
#[derive(Clone, Debug, PartialEq)]
pub struct FreightTech {
    pub vehicles: u32,
    pub tonne_km: Vec<f64>,
    pub metres_a_day: Vec<u64>,
    pub loading_days: Vec<u32>,
    pub carriage_a_tonne_km: Vec<f64>,
    pub units_a_tonne: Vec<f64>,
}

/// A carriage market kind as its system declares it: its market, an instance per origin zone and mode; the kinds that
/// carry; the facts naming a carrier's mode, the product it sells and its posted price for a lot of it; its
/// technology; and the reasons freight is paid, goods leave and goods arrive under.
#[clause("FRT.1", "FRT.4", "FRT.6", "FRT.12")]
#[derive(Clone, Copy, Debug)]
pub struct FreightKind {
    pub market: MarketDecl,
    pub carriers: &'static [&'static str],
    pub mode: &'static str,
    pub sells: &'static str,
    pub price: &'static str,
    pub tech: fn(&phx_core::Register) -> Result<FreightTech, String>,
    pub paid: &'static str,
    pub shipped: &'static str,
    pub arrived: &'static str,
}

/// A carrier's offer at the origin: its posted price and the room its vehicles have left today.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Carrier {
    pub carrier: PartyId,
    pub price: PriceRaw,
    pub room: i64,
}

/// A shipper's consignment: the room it takes of a carrier's vehicles, the load it puts on each segment of its route,
/// and the route's segments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Consignment {
    pub shipper: PartyId,
    pub need: i64,
    pub load: i64,
    pub segments: Vec<usize>,
}

/// The meeting's outcome: each consignment booked with its carrier, those no carrier had room for, and those a full
/// segment refused.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CarriageDay {
    pub booked: Vec<(usize, usize)>,
    pub no_room: Vec<usize>,
    pub over_capacity: Vec<usize>,
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

/// The carriage meeting over the carriers at an origin and the consignments leaving it by their mode, with what each
/// segment can still carry today in `left`, which the bookings take.
#[clause("FRT.6", "FRT.7", "FRT.9", "GEO.13")]
pub fn carriage(
    carriers: &[Carrier],
    consignments: &[Consignment],
    left: &mut [i64],
    draws: &mut Draws,
) -> CarriageDay {
    let mut room: Vec<i64> = carriers.iter().map(|c| c.room).collect();
    let mut order: Vec<usize> = (0..consignments.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, lot(draws, i + 1));
    }
    let mut day = CarriageDay::default();
    for i in order {
        let Some(c) = consignments.get(i) else { continue };
        let fits = c.segments.iter().all(|s| left.get(*s).is_some_and(|l| *l >= c.load));
        if !fits {
            day.over_capacity.push(i);
            continue;
        }
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
        for s in &c.segments {
            if let Some(l) = left.get_mut(*s) {
                *l -= c.load;
            }
        }
        day.booked.push((i, k));
    }
    day
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::PriceRaw;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Carrier, Consignment, carriage};

    fn draws() -> Draws {
        Draws::new(stream_key(Seed::new(3), "FRT.capacity_lot"), Subject::new(SubjectTag::Market, 1), 0, 0)
    }

    fn consignment(shipper: u64, load: i64, segments: Vec<usize>) -> Consignment {
        Consignment { shipper: PartyId::new(shipper), need: load * 2, load, segments }
    }

    #[test]
    fn arrival_day_from_route_and_speed() {
        assert_eq!(super::transit_days(450_000, 600_000, 0), 1, "part of a day's run is a day");
        assert_eq!(super::transit_days(1_300_000, 600_000, 1), 5, "three days' run and a day loading at each end");
        assert_eq!(super::transit_days(600_000, 600_000, 0), 1);
    }

    #[test]
    fn route_capacity_binds_by_lot() {
        let carriers = [
            Carrier { carrier: PartyId::new(1), price: PriceRaw::from_raw(9), room: 1_000 },
            Carrier { carrier: PartyId::new(2), price: PriceRaw::from_raw(5), room: 30 },
        ];
        let consignments: Vec<Consignment> = (0..6).map(|s| consignment(10 + s, 10, vec![0, 1])).collect();
        let mut left = vec![1_000, 40];
        let day = carriage(&carriers, &consignments, &mut left, &mut draws());
        assert_eq!(day.booked.len(), 4, "the second segment carries four loads a day");
        assert_eq!(day.over_capacity.len(), 2, "the rest are refused, never repriced");
        assert_eq!(left, vec![960, 0]);
        assert_eq!(day.booked.iter().filter(|(_, k)| *k == 1).count(), 1, "the cheap carrier's room takes one trip");
        let none = carriage(&carriers[1..], &[consignment(99, 100, vec![0])], &mut [1_000], &mut draws());
        assert_eq!(none.no_room, vec![0], "no room without a vehicle");
    }
}
