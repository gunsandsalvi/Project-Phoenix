//! Goods and the rights to extract: each good an instrument per (product, grade class, zone), issued the first time
//! something names it, so only goods made or held somewhere exist; and each deposit's right to extract, an instrument
//! of one unit over its tile.

use std::collections::BTreeMap;

use phx_id::{Day, InstrumentId, PartyId, ZoneId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_store::Backing;

use crate::instrument::{InstrumentFamily, Instruments, NewInstrument};

/// A good: its product by place in the declared list, its grade class and the zone it stands in.
#[clause("GDS.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, phx_macros::Saved)]
pub struct GoodKey {
    pub product: u16,
    pub grade: u8,
    pub zone: ZoneId,
}

const GRADE_SHIFT: u32 = u32::BITS;
const PRODUCT_SHIFT: u32 = GRADE_SHIFT + u8::BITS;

impl GoodKey {
    /// The key as one number, a market's subject.
    #[must_use]
    pub fn code(self) -> u64 {
        (u64::from(self.product) << PRODUCT_SHIFT) | (u64::from(self.grade) << GRADE_SHIFT) | u64::from(self.zone.get())
    }

    /// The key a code names.
    #[must_use]
    pub fn from_code(code: u64) -> GoodKey {
        let (Ok(product), Ok(grade), Ok(zone)) = (
            u16::try_from(code >> PRODUCT_SHIFT),
            u8::try_from((code >> GRADE_SHIFT) & u64::from(u8::MAX)),
            u32::try_from(code & u64::from(u32::MAX)),
        ) else {
            violation!(clause = "GDS.1", "a good's code beyond its key", code = code);
        };
        GoodKey { product, grade, zone: ZoneId::new(zone) }
    }
}

/// Goods in transit: their owner, the carrier they are pledged to by the lien, the good they left and the good they
/// arrive as, how many units, and the day they left and the day they arrive.
#[clause("FRT.3", "FRT.6", "FRT.9")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Shipment {
    pub owner: PartyId,
    pub carrier: PartyId,
    pub from: InstrumentId,
    pub to: GoodKey,
    pub qty: i64,
    pub lien: crate::lien::LienKey,
    pub left: Day,
    pub arrives: Day,
}

/// Every good issued and every deposit's right, each both ways; the units of each good each party has delivered by
/// trades settled since the world opened, which its sales are read from; and the shipments in transit, by the day
/// they arrive and their number.
#[clause("GDS.1", "GDS.3", "FRM.13", "FRT.3")]
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Goods {
    by_key: BTreeMap<GoodKey, InstrumentId>,
    keys: BTreeMap<InstrumentId, GoodKey>,
    rights: BTreeMap<u32, InstrumentId>,
    deposits: BTreeMap<InstrumentId, u32>,
    delivered: BTreeMap<(PartyId, InstrumentId), i64>,
    transit: BTreeMap<(Day, u64), Shipment>,
    shipped: u64,
}

impl Goods {
    /// A good's instrument, if it has been issued.
    pub fn of(&self, key: GoodKey) -> Missing<InstrumentId> {
        self.by_key.get(&key).copied().map_or(Missing::Absent, Missing::Present)
    }

    /// The good an instrument is, if it is one.
    pub fn key(&self, id: InstrumentId) -> Missing<GoodKey> {
        self.keys.get(&id).copied().map_or(Missing::Absent, Missing::Present)
    }

    /// A good's instrument, issued as a real asset with no issuer the first time it is named.
    pub fn issue<B: Backing>(
        &mut self,
        instruments: &mut Instruments<B>,
        key: GoodKey,
        new: NewInstrument,
    ) -> InstrumentId {
        if let Some(id) = self.by_key.get(&key) {
            return *id;
        }
        if new.family != InstrumentFamily::RealAsset {
            violation!(clause = "GDS.2", "a good issued as other than a real asset", product = key.product);
        }
        let id = instruments.issue(new);
        self.by_key.insert(key, id);
        self.keys.insert(id, key);
        id
    }

    /// A deposit's right to extract, by the deposit's place in the map's list.
    pub fn right(&self, deposit: u32) -> Missing<InstrumentId> {
        self.rights.get(&deposit).copied().map_or(Missing::Absent, Missing::Present)
    }

    /// The deposit a right is over, if the instrument is one.
    pub fn deposit(&self, id: InstrumentId) -> Missing<u32> {
        self.deposits.get(&id).copied().map_or(Missing::Absent, Missing::Present)
    }

    /// A deposit's right issued as a real asset with no issuer, once.
    #[clause("GDS.3")]
    pub fn issue_right<B: Backing>(
        &mut self,
        instruments: &mut Instruments<B>,
        deposit: u32,
        new: NewInstrument,
    ) -> InstrumentId {
        if self.rights.contains_key(&deposit) {
            violation!(clause = "GDS.3", "a deposit's right issued twice", deposit = deposit);
        }
        if new.family != InstrumentFamily::RealAsset {
            violation!(clause = "GDS.3", "a right issued as other than a real asset", deposit = deposit);
        }
        let id = instruments.issue(new);
        self.rights.insert(deposit, id);
        self.deposits.insert(id, deposit);
        id
    }

    /// Units of a good a party delivered by a settled trade.
    pub fn deliver(&mut self, seller: PartyId, good: InstrumentId, qty: i64) {
        let d = self.delivered.entry((seller, good)).or_insert(0);
        let Some(next) = d.checked_add(qty) else {
            phx_num::capacity_exceeded!("units delivered", i64::MAX, qty);
        };
        *d = next;
    }

    /// Each good a party has delivered, with the units, in the goods' order.
    pub fn delivered(&self, party: PartyId) -> impl Iterator<Item = (InstrumentId, i64)> + '_ {
        let lo = (party, InstrumentId::new(0));
        let hi = (party, InstrumentId::new(u32::MAX));
        self.delivered.range(lo..=hi).map(|((_, g), q)| (*g, *q))
    }

    /// A shipment set on its way; its number, one more than the last.
    #[clause("FRT.3")]
    pub fn dispatch(&mut self, s: Shipment) -> u64 {
        let Some(n) = self.shipped.checked_add(1) else {
            phx_num::capacity_exceeded!("shipments", u64::MAX, self.shipped);
        };
        self.shipped = n;
        self.transit.insert((s.arrives, n), s);
        n
    }

    /// The shipments arriving by `day`, taken out of transit, in the order they arrive and were numbered.
    pub fn arriving(&mut self, day: Day) -> Vec<(u64, Shipment)> {
        let later = self.transit.split_off(&(day.succ(), 0));
        let due = std::mem::replace(&mut self.transit, later);
        due.into_iter().map(|((_, n), s)| (n, s)).collect()
    }

    /// Every shipment in transit, with its number.
    pub fn in_transit(&self) -> impl Iterator<Item = (u64, &Shipment)> + '_ {
        self.transit.iter().map(|((_, n), s)| (*n, s))
    }

    /// Every good issued, by key.
    pub fn iter(&self) -> impl Iterator<Item = (GoodKey, InstrumentId)> + '_ {
        self.by_key.iter().map(|(k, i)| (*k, *i))
    }

    /// Every right issued, by deposit.
    pub fn rights(&self) -> impl Iterator<Item = (u32, InstrumentId)> + '_ {
        self.rights.iter().map(|(d, i)| (*d, *i))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::ZoneId;

    use super::GoodKey;

    #[test]
    fn a_key_survives_its_code() {
        let key = GoodKey { product: 9, grade: 3, zone: ZoneId::new(1_999) };
        assert_eq!(GoodKey::from_code(key.code()), key);
        let other = GoodKey { grade: 4, ..key };
        assert_ne!(other.code(), key.code());
    }
}
