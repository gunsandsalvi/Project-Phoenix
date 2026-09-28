//! The banks at the opening: how many each country holds and each one's share of their assets, from a Zipf law fitted
//! to the published concentration; each one's site; and a rate as the ledger holds it.

use phx_core::{OpeningCountry, StreamDef, opening_subject};
use phx_id::CountryId;
use phx_ledger::opening::{derived, whole};
use phx_macros::clause;
use phx_num::{Rate, RatePeriod, violation};

use crate::consts::{PERCENT, PURPOSES, RATE_ONE, SHARE_PARTS, SITES};
use crate::{OpeningStream, zipf};

/// The kind of a firm's term loan, which credit's kind names.
pub(crate) const LOAN_KIND: &str = "firm term loan";

fn subject(country: CountryId, purpose: u32, ordinal: u32) -> phx_rand::Subject {
    opening_subject(u32::from(country.get()) * PURPOSES + purpose, ordinal)
}
/// A yearly rate from a percentage.
pub fn rate(percent: f64) -> Rate {
    Rate::new(whole(percent / PERCENT * RATE_ONE), RatePeriod::Year)
}
/// A country's banks: how many, the exponent of the Zipf law fitted to the published concentration of the three and
/// the five largest, and each bank's share of the banks' assets in parts of a whole.
#[clause("GEN.2", "GEN.3")]
#[must_use]
pub fn bank_weights(c: &OpeningCountry) -> (u32, f64, Vec<u64>) {
    let c5 = derived(c, "GEN.bank_concentration5") / PERCENT;
    let c3 = derived(c, "GEN.bank_top3_of_top5") * c5;
    let (n, a) = zipf::fit(c3, c5);
    let weights = zipf::shares(n, a)
        .into_iter()
        .map(|share| {
            let Ok(weight) = u64::try_from(whole(share * SHARE_PARTS)) else {
                violation!(clause = "GEN.2", "a bank's share below nothing");
            };
            weight
        })
        .collect();
    (n, a, weights)
}
/// The site a country's bank is drawn at, by its place among the country's banks.
pub fn bank_site(ctx: &phx_core::OpeningCtx<'_>, c: &OpeningCountry, k: u32) -> phx_id::TileId {
    let mut draws = ctx.draws(&OpeningStream::DECL, subject(c.id, SITES, k));
    c.site(&mut draws)
}
