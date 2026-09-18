//! NOTHING IS IMMORTAL — and the one exception is a consequence, not an omission.
//!
//! @spec XI-3 · 31 A1.a, 31 E4 · Households F1, F2 · XI-8 · Appendix B · Law 1
//!
//! Every kind of party in this world can cease to exist, each with its own trigger and its own
//! consequence: a firm that cannot pay or whose liabilities exceed its assets; a bank that cannot
//! fund itself **or** whose capital is gone; a fund whose equity is gone; an insurer whose assets
//! fall below the present value of its liabilities; a clearing house that runs past the end of its
//! waterfall; a sovereign that will not or cannot pay in a money it cannot create; a household cell
//! that dissolves.
//!
//! **The one exception is stated rather than left as an omission.** A central bank cannot cease in
//! its own money — §31 A1.a: it can never run out of what it alone issues, which is the whole
//! reason a corridor works. **It can still make a loss**, and the loss is real: it reduces its
//! equity, it is not remitted, and the deferred asset is a row the treasury may have to make good
//! (§31 E4). So immortality here is a CONSEQUENCE of what it issues, and it is bounded to that
//! money — not a party the rules were relaxed for.
//!
//! **Appendix B: no death without a destination.** `cease` returns where what it held goes, and
//! there is no variant that means "nowhere".

use crate::ids::{CurrencyCode, PartyId};

/// XI-3: why this party failed. Each kind fails its own way, and the trigger is named rather than
/// being a single "insolvent" flag that erases what actually happened.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trigger {
    /// A firm: it cannot pay.
    CouldNotPay,
    /// A firm, a fund, an insurer: what it owes is more than what it has.
    LiabilitiesExceedAssets,
    /// A bank: it cannot fund itself — which is a different failure from having no capital, and a
    /// bank can meet either one first (XI-3, Appendix B: liquidity AND solvency).
    CouldNotFundItself,
    /// A bank: its capital is gone.
    CapitalGone,
    /// A clearing house: it ran past the end of its waterfall. A real event with real
    /// consequences, not an impossibility.
    PastTheWaterfall,
    /// A sovereign: it will not or cannot pay, in a money it cannot create.
    WillNotOrCannotPay,
    /// A household cell: it dissolved (Households F1).
    Dissolved,
}

/// Appendix B, XI-8: **no death without a destination.** Where what it held goes — and every
/// variant names somebody, because there is no "nowhere" to put it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Destination {
    /// XI-8: an estate opens, its assets are sold into real markets and its claims are ranked.
    Estate(PartyId),
    /// Households F2: a household cell's wealth transfers to a NAMED heir cell, never to nobody.
    Heir(PartyId),
    /// A bank: resolution — a valuation, a bail-in hierarchy, an acquirer or a public path.
    Resolution(PartyId),
}

/// XI-3: what happens when a party fails. It is a pair — the trigger and the destination — because
/// either alone loses half of what the event is.
#[derive(Clone, Copy, Debug)]
pub struct Ceased {
    pub who: PartyId,
    pub why: Trigger,
    pub to: Destination,
    pub period: u32,
}

/// §31 A1.a: **a central bank cannot cease in its own money.** It is asked here rather than assumed,
/// so the exception is a read with a reason and not a gap in a match.
///
/// It is bounded to THAT money: a central bank short of a money it does not issue is a party like
/// any other, which is why the currency is an argument and not a property of the kind.
pub fn can_cease(is_central_bank: bool, owed_in: CurrencyCode, issues: CurrencyCode) -> bool {
    !(is_central_bank && owed_in == issues)
}

/// §31 E4: **and it can still make a loss.** The loss is real — it reduces equity, it is NOT
/// remitted, and what is left is a deferred asset the treasury may have to make good. A central
/// bank that booked no loss because it cannot fail would be immortality leaking out of its own
/// money into its accounts.
#[derive(Clone, Copy, Debug)]
pub struct CentralBankLoss {
    pub equity_after: f64,
    /// What the treasury may have to make good. It is a ROW somebody holds, not a number that
    /// disappeared into the fact that a central bank cannot run out.
    pub deferred: f64,
    pub remitted: f64,
}

impl CentralBankLoss {
    /// E4: a bank in loss remits NOTHING. Remitting out of a loss would be the interest
    /// round-trip XI-9 warns about, wearing a different hat.
    pub fn is_consistent(&self) -> bool {
        self.equity_after >= 0.0 || (self.remitted == 0.0 && self.deferred > 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_fails_its_own_way_and_a_bank_has_two_doors() {
        // XI-3, Appendix B: a bank fails on liquidity OR on solvency, and they are different
        // failures — a single "insolvent" flag would erase which one happened.
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
        // Appendix B: every variant names somebody. A household's wealth goes to a NAMED heir cell
        // (Households F2), never to nobody — and there is no variant that means "nowhere", so a
        // caller cannot write one.
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
        // §31 A1.a: it can never run out of what it alone issues, which is why a corridor works.
        assert!(!can_cease(true, usd, usd));
        // And it is bounded to THAT money: short of one it does not issue, it is a party like any
        // other. Appendix B's "no sovereign in foreign money" is the same point.
        assert!(can_cease(true, eur, usd));
        // Everybody else can cease in anything.
        assert!(can_cease(false, usd, usd));
    }

    #[test]
    fn a_central_bank_in_loss_remits_nothing_and_the_deferred_asset_is_a_row() {
        // §31 E4: the loss is REAL. Immortality is a consequence of what it issues and must not
        // leak into its accounts.
        let in_loss = CentralBankLoss { equity_after: -400.0, deferred: 400.0, remitted: 0.0 };
        assert!(in_loss.is_consistent());
        // Remitting out of a loss is the interest round-trip in a different hat.
        let flattering = CentralBankLoss { equity_after: -400.0, deferred: 0.0, remitted: 120.0 };
        assert!(!flattering.is_consistent());
    }
}
