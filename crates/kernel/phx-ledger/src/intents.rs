//! What a handler asks of the ledger: units of goods made or used up by its own row's party, by a reason it declared,
//! applied by the one apply routine at the handler's next apply point. A handler moves no money; money moves only by
//! the trades markets make and the dues contracts make.

use phx_core::IntentDef;
use phx_id::Slot;
use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::consts::{DEPOSIT, HAZARD, LEG_WORDS, PURCHASE, SPOILAGE, WAY, WEAR};
use crate::instruction::Source;

/// Units of one good made (positive) or used up (negative), with what accounts for them and, for units made, the
/// cost their lot carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Made {
    pub product: u16,
    pub grade: u8,
    pub qty: i64,
    pub source: Source,
    pub cost: i64,
}

/// A transformation of a row's goods: its reason's code, and its legs, each at the place the kernel reads for its
/// row or, for units taken from a deposit, at the deposit's.
#[clause("SET.9", "GDS.2")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transform {
    pub row: Slot,
    pub reason: u64,
    pub legs: Vec<Made>,
}

fn source_words(s: Source) -> (u64, u64) {
    match s {
        Source::Way(w) => (WAY, u64::from(w)),
        Source::Deposit(d) => (DEPOSIT, u64::from(d)),
        Source::Purchase(p) => (PURCHASE, p),
        Source::Spoilage(k) => (SPOILAGE, k),
        Source::Hazard(e) => (HAZARD, e),
        Source::Wear(c) => (WEAR, u64::from(c)),
    }
}

fn source_of(tag: u64, value: u64) -> Option<Source> {
    Some(match tag {
        WAY => Source::Way(u32::try_from(value).ok()?),
        DEPOSIT => Source::Deposit(u32::try_from(value).ok()?),
        PURCHASE => Source::Purchase(value),
        SPOILAGE => Source::Spoilage(value),
        HAZARD => Source::Hazard(value),
        WEAR => Source::Wear(u32::try_from(value).ok()?),
        _ => return None,
    })
}

impl IntentDef for Transform {
    const NAME: &'static str = "SET.transform";

    fn encode(&self, out: &mut Vec<u64>) {
        let Ok(n) = u64::try_from(self.legs.len()) else {
            capacity_exceeded!("a transformation's legs", u64::MAX, self.legs.len());
        };
        out.extend([u64::from(self.row.get()), self.reason, n]);
        for leg in &self.legs {
            let (tag, value) = source_words(leg.source);
            out.extend([
                (u64::from(leg.product) << u8::BITS) | u64::from(leg.grade),
                leg.qty.cast_unsigned(),
                tag,
                value,
                leg.cost.cast_unsigned(),
            ]);
        }
    }
}

impl Transform {
    /// The intent its words encode, or none when they are not a transformation's.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<Transform> {
        let [row, reason, n, rest @ ..] = words else { return None };
        let n = usize::try_from(*n).ok()?;
        if rest.len() != n.checked_mul(LEG_WORDS)? {
            return None;
        }
        let legs = rest
            .as_chunks::<LEG_WORDS>()
            .0
            .iter()
            .map(|[good, qty, tag, value, cost]| {
                Some(Made {
                    product: u16::try_from(good >> u8::BITS).ok()?,
                    grade: u8::try_from(good & u64::from(u8::MAX)).ok()?,
                    qty: qty.cast_signed(),
                    source: source_of(*tag, *value)?,
                    cost: cost.cast_signed(),
                })
            })
            .collect::<Option<Vec<Made>>>()?;
        Some(Transform { row: Slot::new(u32::try_from(*row).ok()?), reason: *reason, legs })
    }
}

#[cfg(test)]
mod tests {
    use phx_core::IntentDef;
    use phx_id::Slot;

    use super::{Made, Transform};
    use crate::instruction::{Source, name_code};

    #[test]
    fn a_transformation_survives_its_words() {
        let t = Transform {
            row: Slot::new(41),
            reason: name_code("GDS spoiled"),
            legs: vec![
                Made { product: 4, grade: 2, qty: 900, source: Source::Deposit(77), cost: 0 },
                Made { product: 0, grade: 0, qty: -12, source: Source::Spoilage(7), cost: 0 },
            ],
        };
        let mut words = Vec::new();
        t.encode(&mut words);
        assert_eq!(Transform::decode(&words), Some(t));
        assert_eq!(Transform::decode(words.get(..words.len() - 1).unwrap_or(&[])), None, "a leg cut short");
    }
}
