//! Labour in the world: the book of vacancies, applications, offers, hires and separations; the day's rounds at 5c,
//! a round a day; the hires and separations of 4a; and severance paid at 7c. The rules are the labour kind's; the
//! kernel builds each decision's input from the world and applies what it decides.

pub mod book;
mod jobs;
mod post;
mod round;

pub use book::LabourDay;
pub(crate) use book::{Labour, LabourBook};

use if_labour::kind::LabourKind;
use if_labour::law::Law;
use phx_core::{Declarations, OpeningCountry, Register};
use phx_num::violation;

/// The labour kind the systems declare, with each country's law; none when no system declares one. More than one
/// is refused.
pub(crate) fn bind(
    d: &Declarations,
    register: &Register,
    countries: &[OpeningCountry],
    book: LabourBook,
) -> Result<Labour, Vec<String>> {
    let kinds: Vec<LabourKind> =
        d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<LabourKind>()).copied().collect();
    let [kind] = kinds.as_slice() else {
        return if kinds.is_empty() {
            Ok(Labour { book, ..Labour::none() })
        } else {
            Err(vec!["more than one labour kind, where the employment line is one".to_owned()])
        };
    };
    let mut errors = Vec::new();
    let laws: Vec<Law> = countries
        .iter()
        .filter_map(|c| match (kind.law)(register, c) {
            Ok(l) => Some(l),
            Err(e) => {
                errors.push(format!("labour's law in country {}: {e}", c.id.get()));
                None
            }
        })
        .collect();
    if errors.is_empty() { Ok(Labour { book, ..Labour::of(kind, laws) }) } else { Err(errors) }
}

/// A country's law by its identity, which the kind's compile holds for every country.
pub(crate) fn law_of(laws: &[Law], country: phx_id::CountryId) -> &Law {
    let Some(l) = laws.get(usize::from(country.get())) else {
        violation!(clause = "LAB.16", "a country with no labour law", country = country.get());
    };
    l
}
