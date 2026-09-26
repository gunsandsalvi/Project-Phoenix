//! Credit in the world: the book of applications and quotes and each bank's learning; the day's round at 5c, a step
//! of each part a day; the loans written at 7c, each creating the deposit it lends; and the banks' monthly reviews.
//! The rules are the credit kind's; the kernel builds each decision's input from the world and applies what it
//! decides.

pub mod book;
mod round;

pub(crate) use book::{Credit, CreditBook};
pub use book::{CreditDay, Written};

use if_credit::kind::CreditKind;
use if_credit::law::Law;
use phx_core::{Declarations, OpeningCountry, Register};
use phx_num::violation;

/// The credit kind the systems declare, with each country's law; none when no system declares one. More than one
/// is refused.
pub(crate) fn bind(
    d: &Declarations,
    register: &Register,
    countries: &[OpeningCountry],
    book: CreditBook,
) -> Result<Credit, Vec<String>> {
    let kinds: Vec<CreditKind> =
        d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<CreditKind>()).copied().collect();
    let [kind] = kinds.as_slice() else {
        return if kinds.is_empty() {
            Ok(Credit { book, ..Credit::default() })
        } else {
            Err(vec!["more than one credit kind, where the firms' loan line is one".to_owned()])
        };
    };
    let mut errors = Vec::new();
    let laws: Vec<Law> = countries
        .iter()
        .filter_map(|c| match (kind.law)(register, c) {
            Ok(l) => Some(l),
            Err(e) => {
                errors.push(format!("credit's law in country {}: {e}", c.id.get()));
                None
            }
        })
        .collect();
    if errors.is_empty() { Ok(Credit { kind: Some(*kind), laws, book, ..Credit::default() }) } else { Err(errors) }
}

/// A country's law by its identity, which the kind's compile holds for every country.
pub(crate) fn law_of(laws: &[Law], country: phx_id::CountryId) -> &Law {
    let Some(l) = laws.get(usize::from(country.get())) else {
        violation!(clause = "BNK.16", "a country with no lending law", country = country.get());
    };
    l
}
