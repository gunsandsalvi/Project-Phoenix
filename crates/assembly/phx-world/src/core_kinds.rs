//! What the core reads of each kind of party from its declarations — its legal form's features and owners and its
//! place — bound once at assembly and again at load, so no mechanism tells one kind from another by its name.

use phx_core::kinds::{Feature, Owners};
use phx_core::{KindDecl, LegalForm, Place};
use phx_macros::{clause, opening};
use phx_num::{MaybeI64, Missing, violation};

use crate::consts::stats::MONEY_CLASSES;

/// Each kind's declared traits, and each country's heirless destination's kind.
pub type Bound = (Vec<KindTraits>, Vec<u8>);

/// The build's and the law's declarations the core reads by reference — the household kind, each kind's traits and
/// each country's heirless destination's kind — bound at assembly and again at load; a save never holds them.
#[derive(Clone, Debug, Default)]
pub struct Declared {
    pub household: Option<phx_pop::kind::PopKindDecl>,
    pub kinds: Vec<KindTraits>,
    pub heirless: Vec<u8>,
}

/// What the forms say a party may hold that is money.
const MONEY: &str = "money";

/// A kind's declared traits: whether its owners hold its equity, whether it holds money (it may and issues none),
/// whether it takes deposits, who owns it, the money stock's class its deposits count in, and its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindTraits {
    pub has_owners: bool,
    pub holds_money: bool,
    pub takes_deposits: bool,
    pub owners: Owners,
    pub money_class: usize,
    pub place: Place,
}

/// Each kind's traits from its legal form, in the kinds' order: the money stock's classes are the deposit-taking
/// forms' reserves, then the holder forms' deposits in their published order, all others in the last.
///
/// # Errors
/// A kind whose legal form the forms do not declare.
#[opening]
#[clause("PTY.4", "PTY.5", "MON.9")]
pub fn traits(kinds: &[KindDecl], forms: &[LegalForm], holders: &[&str]) -> Result<Vec<KindTraits>, String> {
    kinds
        .iter()
        .map(|k| {
            let Some(form) = forms.iter().find(|f| f.name == k.legal_form) else {
                return Err(format!(
                    "kind `{}` takes the legal form `{}`, which the law does not declare",
                    k.name, k.legal_form
                ));
            };
            let takes_deposits = form.has(Feature::TakesDeposits);
            let money_class = if takes_deposits {
                0
            } else {
                holders.iter().position(|h| *h == form.name).map_or(MONEY_CLASSES - 1, |at| at + 1)
            };
            Ok(KindTraits {
                has_owners: form.has(Feature::HasOwners),
                holds_money: form.may_hold.iter().any(|h| h == MONEY) && !form.has(Feature::IssuesCurrency),
                takes_deposits,
                owners: form.owners,
                money_class,
                place: k.place,
            })
        })
        .collect()
}

/// A kind's traits by its place among the kinds; a place beyond them stops the run.
#[clause("PTY.4")]
pub(crate) fn of(traits: &[KindTraits], kind: usize) -> &KindTraits {
    match traits.get(kind) {
        Some(t) => t,
        None => violation!(clause = "PTY.4", "a kind with no declared traits", kind = kind),
    }
}

/// The country a party is of, read from where its kind declares its place: a region word through the regions, the
/// region its population declaration sites it by, the country word, or its site's tile through the map.
#[clause("PTY.5")]
pub(crate) fn country_by_place(
    place: Place,
    record: &[MaybeI64],
    (sited_by, regions): (Missing<usize>, &[phx_id::CountryId]),
    tile_country: impl Fn(u32) -> Option<usize>,
) -> Option<usize> {
    let word = |at: usize| match record.get(at)?.get() {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    };
    let region = |at: usize| regions.get(usize::try_from(word(at)?).ok()?).map(|c| usize::from(c.get()));
    match place {
        Place::Region { word: at } => region(usize::from(at)),
        Place::Sited => match sited_by {
            Missing::Present(at) => region(at),
            Missing::Absent => None,
        },
        Place::Country { word: at } => usize::try_from(word(usize::from(at))?).ok(),
        Place::Site { word: at } => tile_country(u32::try_from(word(usize::from(at))?).ok()?),
    }
}

/// The party a country's inheritance law passes an estate with no heir to: of its institutions, the one of the kind
/// the law names.
#[must_use]
pub(crate) fn heirless_party(institutions: &[Option<phx_id::PartyKey>], kind: u8) -> Option<phx_id::PartyKey> {
    institutions.iter().flatten().copied().find(|p| p.kind() == kind)
}

#[cfg(test)]
#[path = "core_kinds_tests.rs"]
mod tests;
