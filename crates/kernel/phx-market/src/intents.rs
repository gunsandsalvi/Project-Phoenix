//! What a handler asks of a market: an order for its own row's party, in a declared market kind over a good at the
//! row's place or another it buys at, admitted into the day's book at 5d.

use phx_core::IntentDef;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{Missing, PriceRaw, capacity_exceeded};

use crate::order::{Side, Step};

/// An order as a handler asks it: its row, its market kind's code, the product and grade class it is over, the place
/// whose market it is posted at where that is not the row's own, its side and its steps.
#[clause("MKT.16", "MKT.17", "FRT.5")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderIntent {
    pub row: Slot,
    pub kind: u64,
    pub product: u16,
    pub grade: u8,
    pub at: Missing<u32>,
    pub side: Side,
    pub steps: Vec<Step>,
}

const SELL: u64 = 1;

/// A place as an intent's word carries it: none, or the zone with the bit above a zone's set.
fn place_word(at: Missing<u32>) -> u64 {
    match at {
        Missing::Present(z) => PLACED | u64::from(z),
        Missing::Absent => 0,
    }
}

/// The place an intent's word carries, or none when the word is no place's.
fn word_place(word: u64) -> Option<Missing<u32>> {
    match word {
        0 => Some(Missing::Absent),
        w if w & PLACED != 0 => u32::try_from(w & !PLACED).ok().map(Missing::Present),
        _ => None,
    }
}

/// The bit a word sets when it carries a place.
const PLACED: u64 = 1 << u32::BITS;

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
            place_word(self.at),
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
        let [row, kind, good, at, side, n, rest @ ..] = words else { return None };
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
            at: word_place(*at)?,
            side,
            steps,
        })
    }
}

/// A shipper's want to carry goods as a handler asks it: its row, the carriage kind's code, the product and grade
/// class of the goods, the place they are at where that is not the row's own, the units, the zone they go to, and the
/// mode.
#[clause("FRT.5", "FRT.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipIntent {
    pub row: Slot,
    pub kind: u64,
    pub product: u16,
    pub grade: u8,
    pub from: Missing<u32>,
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
            place_word(self.from),
            self.qty.cast_unsigned(),
            (u64::from(self.to) << u16::BITS) | u64::from(self.mode),
        ]);
    }
}

impl ShipIntent {
    /// The intent its words encode, or none when they are not a shipper's want.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<ShipIntent> {
        let [row, kind, good, from, qty, route] = words else { return None };
        Some(ShipIntent {
            row: Slot::new(u32::try_from(*row).ok()?),
            kind: *kind,
            product: u16::try_from(good >> u8::BITS).ok()?,
            grade: u8::try_from(good & u64::from(u8::MAX)).ok()?,
            from: word_place(*from)?,
            qty: qty.cast_signed(),
            to: u32::try_from(route >> u16::BITS).ok()?,
            mode: u16::try_from(route & u64::from(u16::MAX)).ok()?,
        })
    }
}

/// An owner's investment as a handler asks it: its row, the kind of plant, the product that kind is bought as, and
/// the units it buys, from a named producer at its place, in the stages it is built in.
#[clause("CAP.5", "CAP.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvestIntent {
    pub row: Slot,
    pub kind: u8,
    pub product: u16,
    pub units: i64,
    pub stages: u32,
}

impl IntentDef for InvestIntent {
    const NAME: &'static str = "CAP.invest";

    fn encode(&self, out: &mut Vec<u64>) {
        out.extend([
            u64::from(self.row.get()),
            u64::from(self.kind),
            u64::from(self.product),
            self.units.cast_unsigned(),
            u64::from(self.stages),
        ]);
    }
}

impl InvestIntent {
    /// The intent its words encode, or none when they are not an investment.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<InvestIntent> {
        let [row, kind, product, units, stages] = words else { return None };
        Some(InvestIntent {
            row: Slot::new(u32::try_from(*row).ok()?),
            kind: u8::try_from(*kind).ok()?,
            product: u16::try_from(*product).ok()?,
            units: units.cast_signed(),
            stages: u32::try_from(*stages).ok()?,
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
            at: phx_num::Missing::Present(12),
            side: Side::Sell,
            steps: vec![
                Step { limit: PriceRaw::from_raw(-5), qty: 40 },
                Step { limit: PriceRaw::from_raw(90), qty: 7 },
            ],
        };
        let mut words = Vec::new();
        o.encode(&mut words);
        assert_eq!(OrderIntent::decode(&words), Some(o.clone()));
        assert_eq!(OrderIntent::decode(words.get(..4).unwrap_or(&[])), None);
        let home = OrderIntent { at: phx_num::Missing::Absent, ..o };
        let mut words = Vec::new();
        home.encode(&mut words);
        assert_eq!(OrderIntent::decode(&words), Some(home));
    }

    #[test]
    fn a_consignment_survives_its_words() {
        for from in [phx_num::Missing::Absent, phx_num::Missing::Present(0), phx_num::Missing::Present(u32::MAX)] {
            let s = super::ShipIntent {
                row: Slot::new(9),
                kind: 5,
                product: 2,
                grade: 1,
                from,
                qty: 400,
                to: 77_000,
                mode: 1,
            };
            let mut words = Vec::new();
            s.encode(&mut words);
            assert_eq!(super::ShipIntent::decode(&words), Some(s));
        }
    }
}
