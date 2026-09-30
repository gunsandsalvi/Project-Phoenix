use std::fmt;

use phx_core::{Declarations, ItemDecl, check_claims};
use phx_id::SystemCode;
use phx_macros::clause;
use phx_rand::float::from_i64;

/// Every refusal of an assembly, reported at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyErrors(pub Vec<String>);

impl fmt::Display for AssemblyErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for e in &self.0 {
            writeln!(f, "{e}")?;
        }
        Ok(())
    }
}

/// The refusals the declarations and the interfaces' items decide together: each item written and claimed once by a
/// registered system, and whatever the declarations refuse on their own.
#[clause("TIME.6", "Law 4")]
#[must_use]
pub fn refusals(d: &Declarations, items: &[ItemDecl], registered: &[SystemCode]) -> Vec<String> {
    let mut errors = d.refusals();
    match d.claims() {
        Ok(claims) => {
            if let Err(e) = check_claims(items, &claims, registered) {
                errors.extend(e);
            }
        }
        Err(e) => errors.push(e),
    }
    errors
}

/// Each product's lead time in days, compiled once from the technology's table: a product it lacks refuses the world.
///
/// # Errors
/// The first product the table holds no lead for.
pub(crate) fn leads(products: usize, lead: &dyn Fn(i64) -> Option<i64>) -> Result<Vec<f64>, String> {
    (0_i64..)
        .take(products)
        .map(|p| lead(p).map(from_i64).ok_or_else(|| format!("TEC.lead_time: product {p} has none")))
        .collect()
}

/// A product's opening price in a country: none refuses the world.
///
/// # Errors
/// A country or product the opening's prices do not hold.
pub(crate) fn opening_price(prices: &[Vec<f64>], (country, product): (usize, u16)) -> Result<f64, String> {
    prices
        .get(country)
        .and_then(|p| p.get(usize::from(product)))
        .copied()
        .ok_or_else(|| format!("GDS.opening_price: country {country} has no price for product {product}"))
}

/// A country's lending rate, the rate its firms finance their making at: none refuses the world.
///
/// # Errors
/// A country the drawn rates do not hold.
pub(crate) fn lending_rate(rate: Option<f64>, country: u8) -> Result<f64, String> {
    rate.ok_or_else(|| format!("GEN.lending_rate: country {country} has none"))
}

#[cfg(test)]
#[path = "refusals_tests.rs"]
mod values;

#[cfg(test)]
mod tests {
    use phx_core::{
        Audience, Declarations, FactDecl, FactType, ItemDecl, ItemKind, Purpose, ReprClass, StreamDecl, SystemEntry,
        Writer, declare_entry,
    };
    use phx_id::SystemCode;
    use phx_num::Missing;

    use super::refusals;

    fn declare(d: &mut Declarations) {
        d.stream(StreamDecl {
            name: "HH.taste",
            family: phx_core::StreamFamily::World,
            purpose: Purpose::Taste,
            keyed: false,
            clause: "CHN.3",
        });
        d.stream(StreamDecl {
            name: "HH.taste",
            family: phx_core::StreamFamily::World,
            purpose: Purpose::Taste,
            keyed: false,
            clause: "CHN.3",
        });
    }

    #[test]
    fn refusals_are_complete() {
        let mut d = Declarations::new();
        declare_entry(&SystemEntry { code: "HH", declare }, &mut d);
        let fact = FactDecl {
            value: FactType::Money,
            unit: Missing::Absent,
            kinds: &["household"],
            audience: Audience::Party,
            repr: ReprClass::Position,
        };
        let items =
            [ItemDecl { name: "HH.cash", kind: ItemKind::Fact(fact), writer: Writer::System("HH"), clause: "HH.1" }];
        let errors = refusals(&d, &items, &[SystemCode::new("HH").unwrap()]);
        let has = |text: &str| errors.iter().any(|e| e.contains(text));
        assert!(has("declared twice"), "a stream twice: {errors:?}");
        assert!(has("claimed by []"), "a fact with no claim");
    }
}
