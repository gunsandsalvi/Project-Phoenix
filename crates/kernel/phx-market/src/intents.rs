//! What a handler asks of a market: an order for its own row's party, in a declared market kind over a good at the
//! row's place, admitted into the day's book at 5d.

use phx_core::IntentDef;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{PriceRaw, capacity_exceeded};

use crate::order::{Side, Step};

/// An order as a handler asks it: its row, its market kind's code, the product and grade class it is over, its side
/// and its steps.
#[clause("MKT.16", "MKT.17")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderIntent {
    pub row: Slot,
    pub kind: u64,
    pub product: u16,
    pub grade: u8,
    pub side: Side,
    pub steps: Vec<Step>,
}

const SELL: u64 = 1;

impl IntentDef for OrderIntent {
    const NAME: &'static str = "MKT.order";

    fn encode(&self, out: &mut Vec<u64>) {
        let Ok(n) = u64::try_from(self.steps.len()) else {
            capacity_exceeded!("an order's steps", u64::MAX, self.steps.len());
        };
        let side = match self.side {
            Side::Buy => 0,
            Side::Sell => SELL,
        };
        out.extend([
            u64::from(self.row.get()),
            self.kind,
            (u64::from(self.product) << u8::BITS) | u64::from(self.grade),
            side,
            n,
        ]);
        out.extend(self.steps.iter().flat_map(|s| [s.limit.raw().cast_unsigned(), s.qty.cast_unsigned()]));
    }
}

impl OrderIntent {
    /// The intent its words encode, or none when they are not an order's.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<OrderIntent> {
        let [row, kind, good, side, n, rest @ ..] = words else { return None };
        let n = usize::try_from(*n).ok()?;
        if rest.len() != n.checked_mul(2)? {
            return None;
        }
        let side = match *side {
            0 => Side::Buy,
            SELL => Side::Sell,
            _ => return None,
        };
        let steps = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|[limit, qty]| Some(Step { limit: PriceRaw::from_raw(limit.cast_signed()), qty: qty.cast_signed() }))
            .collect::<Option<Vec<Step>>>()?;
        Some(OrderIntent {
            row: Slot::new(u32::try_from(*row).ok()?),
            kind: *kind,
            product: u16::try_from(good >> u8::BITS).ok()?,
            grade: u8::try_from(good & u64::from(u8::MAX)).ok()?,
            side,
            steps,
        })
    }
}

/// A buyer's want of a product as a handler asks it: its row, the retail kind's code, the product, and what a twin
/// wants, units it needs or money it spends.
#[clause("SRV.4", "HH.5")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShopIntent {
    pub row: Slot,
    pub kind: u64,
    pub product: u16,
    pub want: crate::retail::Want,
}

const MONEY: u64 = 1;

impl IntentDef for ShopIntent {
    const NAME: &'static str = "SRV.shop";

    fn encode(&self, out: &mut Vec<u64>) {
        let (tag, amount) = match self.want {
            crate::retail::Want::Units(q) => (0, q),
            crate::retail::Want::Money(m) => (MONEY, m),
        };
        out.extend([u64::from(self.row.get()), self.kind, u64::from(self.product), tag, amount.cast_unsigned()]);
    }
}

impl ShopIntent {
    /// The intent its words encode, or none when they are not a buyer's want.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<ShopIntent> {
        let [row, kind, product, tag, amount] = words else { return None };
        let want = match *tag {
            0 => crate::retail::Want::Units(amount.cast_signed()),
            MONEY => crate::retail::Want::Money(amount.cast_signed()),
            _ => return None,
        };
        Some(ShopIntent {
            row: Slot::new(u32::try_from(*row).ok()?),
            kind: *kind,
            product: u16::try_from(*product).ok()?,
            want,
        })
    }
}

/// A shipper's want to carry goods as a handler asks it: its row, the carriage kind's code, the product and grade
/// class of the goods where the row stands, units a twin, the zone they go to, and the mode.
#[clause("FRT.5", "FRT.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipIntent {
    pub row: Slot,
    pub kind: u64,
    pub product: u16,
    pub grade: u8,
    pub qty: i64,
    pub to: u32,
    pub mode: u16,
}

impl IntentDef for ShipIntent {
    const NAME: &'static str = "FRT.ship";

    fn encode(&self, out: &mut Vec<u64>) {
        out.extend([
            u64::from(self.row.get()),
            self.kind,
            (u64::from(self.product) << u8::BITS) | u64::from(self.grade),
            self.qty.cast_unsigned(),
            (u64::from(self.to) << u16::BITS) | u64::from(self.mode),
        ]);
    }
}

impl ShipIntent {
    /// The intent its words encode, or none when they are not a shipper's want.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<ShipIntent> {
        let [row, kind, good, qty, route] = words else { return None };
        Some(ShipIntent {
            row: Slot::new(u32::try_from(*row).ok()?),
            kind: *kind,
            product: u16::try_from(good >> u8::BITS).ok()?,
            grade: u8::try_from(good & u64::from(u8::MAX)).ok()?,
            qty: qty.cast_signed(),
            to: u32::try_from(route >> u16::BITS).ok()?,
            mode: u16::try_from(route & u64::from(u16::MAX)).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use phx_core::IntentDef;
    use phx_id::Slot;
    use phx_num::PriceRaw;

    use super::OrderIntent;
    use crate::order::{Side, Step};

    #[test]
    fn an_order_survives_its_words() {
        let o = OrderIntent {
            row: Slot::new(3),
            kind: phx_ledger::instruction::name_code("GDS.commodities"),
            product: 2,
            grade: 1,
            side: Side::Sell,
            steps: vec![
                Step { limit: PriceRaw::from_raw(-5), qty: 40 },
                Step { limit: PriceRaw::from_raw(90), qty: 7 },
            ],
        };
        let mut words = Vec::new();
        o.encode(&mut words);
        assert_eq!(OrderIntent::decode(&words), Some(o));
        assert_eq!(OrderIntent::decode(words.get(..4).unwrap_or(&[])), None);
    }

    #[test]
    fn a_consignment_survives_its_words() {
        let s = super::ShipIntent { row: Slot::new(9), kind: 5, product: 2, grade: 1, qty: 400, to: 77_000, mode: 1 };
        let mut words = Vec::new();
        s.encode(&mut words);
        assert_eq!(super::ShipIntent::decode(&words), Some(s));
    }

    #[test]
    fn a_want_survives_its_words() {
        for want in [crate::retail::Want::Units(12), crate::retail::Want::Money(-1), crate::retail::Want::Money(900)] {
            let s = super::ShopIntent { row: Slot::new(4), kind: 77, product: 3, want };
            let mut words = Vec::new();
            s.encode(&mut words);
            assert_eq!(super::ShopIntent::decode(&words), Some(s));
        }
    }
}
