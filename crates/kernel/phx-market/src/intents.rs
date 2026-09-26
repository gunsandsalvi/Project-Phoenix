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
}
