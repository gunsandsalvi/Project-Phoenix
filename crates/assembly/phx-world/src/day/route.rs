//! The routing table: each slot of the day and the calls into the core that fill it, in the order the slot makes them;
//! a slot the table lists with no call runs nothing yet.

use phx_core::slots::DaySlot as S;

/// A call the day makes into the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Call {
    Weather,
    /// The rates the day's hazards are drawn at, measured before they are drawn.
    Rates,
    Hazards,
    /// The first day of a year's refresh of the households' windows.
    Windows,
    Labour,
    Goods,
    Freight,
    /// Settlement on a day no country does business: dues taken and what is bought recorded as commitments.
    SettleClosed,
    Settle,
    Publish,
    Audit,
    Statistics,
    /// The record of which of the player's decisions came today.
    Player,
}

/// The slots that make calls, each with its calls in order.
pub(crate) const ROUTES: &[(S, &[Call])] = &[
    (S::S3a, &[Call::Weather]),
    (S::S3b, &[Call::Rates, Call::Hazards]),
    (S::S5a, &[Call::Windows]),
    (S::S5b, &[Call::Labour]),
    (S::S6a, &[Call::Goods]),
    (S::S6b, &[Call::Freight]),
    (S::S6d, &[Call::SettleClosed]),
    (S::S7a, &[Call::Settle]),
    (S::S10a, &[Call::Publish]),
    (S::S10c, &[Call::Audit]),
    (S::S10d, &[Call::Statistics, Call::Player]),
];

/// A slot's calls, none for a slot nothing fills yet.
pub(crate) fn calls(slot: S) -> &'static [Call] {
    ROUTES.iter().find(|(s, _)| *s == slot).map_or(&[], |(_, c)| c)
}

/// Whether a call does anything on a day, which only a closed day's settlement asks.
pub(crate) fn acts(call: Call, any_business: bool) -> bool {
    call != Call::SettleClosed || !any_business
}

#[cfg(test)]
#[path = "route_tests.rs"]
mod tests;
