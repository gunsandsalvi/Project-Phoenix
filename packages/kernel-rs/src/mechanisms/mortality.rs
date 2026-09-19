//! NOTHING IS IMMORTAL — and the one exception is a consequence, not an omission.
//!
//! @spec XI-3 · 31 A1.a, 31 E4 · Households F1, F2 · XI-8 · Appendix B · Law 1

use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::ids::{CurrencyCode, PartyId};

/// Why this party failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trigger {
    /// A firm: it cannot pay.
    CouldNotPay,
    /// A firm, a fund, an insurer: what it owes is more than what it has.
    LiabilitiesExceedAssets,
    /// A bank: it cannot fund itself — which is a different failure from having no capital, and a
    /// bank can meet either one first.
    CouldNotFundItself,
    CapitalGone,
    /// A clearing house: it ran past the end of its waterfall.
    PastTheWaterfall,
    /// A sovereign: it will not or cannot pay, in a money it cannot create.
    WillNotOrCannotPay,
    /// A household cell: it dissolved.
    Dissolved,
}

/// No death without a destination.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    /// An estate opens, its assets are sold into real markets and its claims are ranked.
    Estate(PartyId),
    /// A household cell's wealth transfers to a NAMED heir cell, never to nobody.
    Heir(PartyId),
    /// A bank: resolution — a valuation, a bail-in hierarchy, an acquirer or a public path.
    Resolution(PartyId),
}

/// What happens when a party fails.
#[derive(Clone, Copy, Debug)]
pub struct Ceased {
    pub who: PartyId,
    pub why: Trigger,
    pub to: Destination,
    pub period: u32,
}

/// A central bank cannot cease in its own money.
pub fn can_cease(is_central_bank: bool, owed_in: CurrencyCode, issues: CurrencyCode) -> bool {
    !(is_central_bank && owed_in == issues)
}

/// And it can still make a loss.
#[derive(Clone, Copy, Debug)]
pub struct CentralBankLoss {
    pub equity_after: f64,
    /// What the treasury may have to make good.
    pub deferred: f64,
    pub remitted: f64,
}

impl CentralBankLoss {
    /// A bank in loss remits NOTHING.
    pub fn is_consistent(&self) -> bool {
        self.equity_after >= 0.0 || (self.remitted == 0.0 && self.deferred > 0.0)
    }
}

// XI-3 RUNS HERE.

/// NOTHING IS IMMORTAL — and nothing in this world had ever died.
pub struct Failing {
    pub says: u32,
}

impl Mechanism for Failing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut gone: Vec<(PartyId, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // It cannot run out of what it alone issues.
            let kind = ctx.parties().kind_of(who);
            if matches!(
                ctx.registry().profile(kind),
                Some(profile) if profile.banks == crate::registry::Banks::Nowhere
            ) {
                continue;
            }
            let worth = crate::instruments::equity(who, ctx.register(), ctx.instruments(), ctx.claims());
            if worth >= 0.0 {
                continue;
            }
            gone.push((who, worth));
        }
        for (who, worth) in gone {
            // What it HELD is the estate's, and this records only that its life ended — the estate
            // machinery is what pays its claimants in rank order.
            ctx.ceases(who);
            ctx.say(self.says, &[who.0], &[(0, Value::Num(worth))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_fails_its_own_way_and_a_bank_has_two_doors() {
        // A bank fails on liquidity OR on solvency, and they are different failures — a single
        // "insolvent" flag would erase which one happened.
        let liquidity = Ceased {
            who: PartyId::at(3),
            why: Trigger::CouldNotFundItself,
            to: Destination::Resolution(PartyId::at(0)),
            period: 40,
        };
        let solvency = Ceased { why: Trigger::CapitalGone, ..liquidity };
        assert_ne!(liquidity.why, solvency.why);
    }

    #[test]
    fn there_is_no_death_without_a_destination() {
        // Every variant names somebody.
        let c = Ceased {
            who: PartyId::at(9),
            why: Trigger::Dissolved,
            to: Destination::Heir(PartyId::at(10)),
            period: 12,
        };
        match c.to {
            Destination::Heir(to) | Destination::Estate(to) | Destination::Resolution(to) => {
                assert!(to.some(), "somebody receives it");
            }
        }
    }

    #[test]
    fn a_central_bank_cannot_cease_in_its_own_money_and_can_in_any_other() {
        let usd = CurrencyCode::at(0);
        let eur = CurrencyCode::at(1);
        // It can never run out of what it alone issues, which is why a corridor works.
        assert!(!can_cease(true, usd, usd));
        // And it is bounded to THAT money: short of one it does not issue, it is a party like any
        // other.
        assert!(can_cease(true, eur, usd));
        // Everybody else can cease in anything.
        assert!(can_cease(false, usd, usd));
    }

    #[test]
    fn a_central_bank_in_loss_remits_nothing_and_the_deferred_asset_is_a_row() {
        // The loss is REAL.
        let in_loss = CentralBankLoss { equity_after: -400.0, deferred: 400.0, remitted: 0.0 };
        assert!(in_loss.is_consistent());
        // Remitting out of a loss is the interest round-trip in a different hat.
        let flattering = CentralBankLoss { equity_after: -400.0, deferred: 0.0, remitted: 120.0 };
        assert!(!flattering.is_consistent());
    }
}
