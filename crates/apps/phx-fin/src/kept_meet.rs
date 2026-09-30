//! Today's posted-price meeting (`phx_market::meet`): the design point's stalls spread over its products and zones,
//! and a day's retail wants met product by product, each buyer choosing among the stalls of its own zone.

use std::collections::BTreeMap;

use phx_exec::Pool;
use phx_id::{PartyKey, Slot};
use phx_market::meet::{Buyer, Meeting, Place, Stall, Tastes, meet};
use phx_market::retail::{Want, Weights};
use phx_rand::uniform::{below_u64, open_unit};
use phx_rand::{Draws, Subject, SubjectTag};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{BASE, count, day_of, index, slots, whole, wide};
use crate::measure::Measures;
use crate::{DayType, FinError};

/// The kinds of the fill: the buyers' and the sellers'.
const BUYERS: u8 = 1;
const SELLERS: u8 = 2;
/// How a buyer weighs a stall, as the meeting's own arithmetic is checked at: the log price's weight, a km's.
const WEIGHTS: Weights = Weights { price: 2.0, distance: 0.05 };
/// A stall posts a price of up to this many cents a unit and holds up to this many units, so some sell out and their
/// buyers choose again; a buyer wants up to this many units; a stall is up to this many km from its zone's buyers.
const PRICE_CENTS: u64 = 10_000;
const STALL_UNITS: u64 = 40;
const WANT_UNITS: u64 = 3;
const ZONE_KM: f64 = 5.0;

/// The stalls by product, each zone's stalls of each product, and each day type's buyers by product.
#[derive(Debug, Default)]
pub struct Meet {
    stalls: Vec<Vec<Stall>>,
    places: Vec<Vec<Place>>,
    buyers: BTreeMap<DayType, Vec<Vec<Buyer>>>,
    meeting: Meeting,
    zones: u64,
    streams: Option<Streams>,
    /// The last day's wants that found no stall, and its sales.
    pub unserved: u64,
    pub sales: u64,
}

impl Meet {
    /// `[store] stalls` over `[store] products`, each in a zone drawn for it with a price and units drawn for it.
    ///
    /// # Errors
    /// A design point without the counts.
    pub fn fill(&mut self, design: &Design, streams: &Streams) -> Result<u64, FinError> {
        let stalls = count(&design.store, "stalls", "store")?;
        let products = count(&design.store, "products", "store")?;
        let zones = count(&design.store, "zones", "store")?;
        let mut d = streams.draws("kept.meet", 0, 0);
        self.stalls = vec![Vec::new(); index(products)?];
        self.places = vec![vec![Place::default(); index(zones)?]; index(products)?];
        for s in 0..stalls {
            let product = index(s % products)?;
            let zone = index(below_u64(&mut d, zones))?;
            let (Some(list), Some(places)) = (self.stalls.get_mut(product), self.places.get_mut(product)) else {
                continue;
            };
            let at = slots(wide(list.len()))?;
            list.push(Stall {
                seller: PartyKey::new(SELLERS, Slot::new(slots(s)?)),
                price: whole(below_u64(&mut d, PRICE_CENTS) + 1)?,
                units: whole(below_u64(&mut d, STALL_UNITS) + 1)?,
            });
            if let Some(place) = places.get_mut(zone) {
                place.near.push((at, open_unit(&mut d) * ZONE_KM));
            }
        }
        (self.zones, self.streams) = (zones, Some(*streams));
        Ok(stalls)
    }

    fn make(&self, day: DayType, wants: u64) -> Result<Vec<Vec<Buyer>>, FinError> {
        let streams = self.streams.ok_or_else(|| FinError("buyers made before the fill".to_owned()))?;
        let products = u64::try_from(self.stalls.len()).map_err(|e| FinError(e.to_string()))?;
        let mut d = streams.draws("kept.meet", 1, day_of(day)?);
        let mut by_product = vec![Vec::new(); self.stalls.len()];
        for w in 0..wants {
            if let Some(list) = by_product.get_mut(index(w % products)?) {
                list.push(Buyer {
                    party: PartyKey::new(BUYERS, Slot::new(slots(w / products)?)),
                    subject: w,
                    want: Want::Units(whole(below_u64(&mut d, WANT_UNITS) + 1)?),
                    place: slots(below_u64(&mut d, self.zones))?,
                });
            }
        }
        Ok(by_product)
    }

    /// A day's `retail` wants met, product by product.
    ///
    /// # Errors
    /// A day without `retail`.
    pub fn day(
        &mut self,
        day: DayType,
        counts: &BTreeMap<String, u64>,
        m: &mut Measures<'_>,
        pool: Option<&Pool>,
    ) -> Result<(), FinError> {
        let wants = count(counts, "retail", "day")?;
        if !self.buyers.contains_key(&day) {
            let made = self.make(day, wants)?;
            self.buyers.insert(day, made);
        }
        let streams = self.streams.ok_or_else(|| FinError("a meeting before the fill".to_owned()))?;
        let (taste, lot) = (streams.key("kept.taste"), streams.key("kept.lot"));
        // The fill meets one day, so a seller's round takes the draws' day, and no two rounds share a draw.
        let lots = move |s: PartyKey, round: u32| {
            Draws::new(lot, Subject::new(SubjectTag::Party, u64::from(s.word())), round, 0)
        };
        let tastes = Tastes { key: taste, day: 0, substep: 0 };
        let (mut unserved, mut sales) = (0, 0);
        let buyers = self.buyers.get(&day).map_or(&[][..], Vec::as_slice);
        let meeting = &mut self.meeting;
        for ((stalls, places), buyers) in self.stalls.iter().zip(&self.places).zip(buyers) {
            let n = u64::try_from(buyers.len()).map_err(|e| FinError(e.to_string()))?;
            m.read(BASE, "purchase", n, || meet(meeting, pool, (stalls, places, buyers), (1, WEIGHTS), tastes, &lots));
            unserved += wide(meeting.unserved.len());
            sales += wide(meeting.sales().count());
        }
        (self.unserved, self.sales) = (unserved, sales);
        Ok(())
    }

    /// A digest of the stalls, the same for any workers.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let words = self.stalls.iter().flatten().flat_map(|s| [u64::from(s.seller.word()), s.price.cast_unsigned()]);
        words.fold(0, |a, w| phx_exec::mix::mix64(a ^ w))
    }

    /// The bytes the stalls and each zone's reach hold.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        let stalls: usize = self.stalls.iter().map(Vec::len).sum();
        let near: usize = self.places.iter().flatten().map(|p| p.near.len()).sum();
        wide(stalls * size_of::<Stall>() + near * size_of::<(u32, f64)>())
    }
}
