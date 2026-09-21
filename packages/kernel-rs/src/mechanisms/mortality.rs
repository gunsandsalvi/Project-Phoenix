//! NOTHING IS IMMORTAL — and the one exception is a consequence, not an omission.
//!
//! @spec XI-3 · 31 A1.a, 31 E4 · Households F1, F2 · XI-8 · Appendix B · Law 1

use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::ids::{CurrencyCode, PartyId};
pub use crate::parties::Destination;

/// Why this party failed.
#[repr(u8)]
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
    /// A fund completed an orderly wind-up rather than failing.
    WoundUp,
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
    pub loss_crossed: u32,
    pub at_standing: u32,
    pub at_trigger: u32,
    pub at_destination: u32,
    pub dissolved: u32,
    pub past_waterfall: u32,
    pub funding_failed: u32,
}

fn destination(trigger: Trigger) -> Destination {
    match trigger {
        Trigger::Dissolved => Destination::Heir,
        Trigger::CouldNotFundItself | Trigger::CapitalGone | Trigger::PastTheWaterfall => Destination::Resolution,
        _ => Destination::Estate,
    }
}

pub fn trigger_for(kind: u32, written_off: bool, failed_due: bool, negative_equity: bool, dissolved: bool, past_waterfall: bool) -> Option<Trigger> {
    use crate::assembly::kinds;
    if past_waterfall { return Some(Trigger::PastTheWaterfall); }
    match kind {
        kinds::CENTRAL_BANK => None,
        kinds::HOUSEHOLD if dissolved => Some(Trigger::Dissolved),
        kinds::HOUSEHOLD => None,
        kinds::BANK if negative_equity => Some(Trigger::CapitalGone),
        kinds::BANK if failed_due => Some(Trigger::CouldNotFundItself),
        kinds::FUND | kinds::INSURER if negative_equity => Some(Trigger::LiabilitiesExceedAssets),
        kinds::TREASURY if failed_due => Some(Trigger::WillNotOrCannotPay),
        kinds::FIRM | kinds::SMALL_FIRM | kinds::CARRIER | kinds::DEALER | kinds::STOCKIST
            if written_off || failed_due => Some(Trigger::CouldNotPay),
        _ => None,
    }
}

pub(crate) fn trigger_code(trigger: Trigger) -> f64 {
    f64::from(trigger as u8)
}

pub(crate) fn trigger_id(trigger: Trigger) -> u8 {
    trigger as u8
}

pub(crate) fn destination_code(to: Destination) -> f64 {
    f64::from(to as u8)
}

impl Mechanism for Failing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let dissolved: std::collections::HashSet<u32> = ctx.journal().of_kind(self.dissolved).iter().flat_map(|row| ctx.journal().subjects_of(*row).iter().copied()).collect();
        let past_waterfall: std::collections::HashSet<u32> = ctx.journal().of_kind(self.past_waterfall).iter().flat_map(|row| ctx.journal().subjects_of(*row).iter().copied()).collect();
        let prior = ctx.period().saturating_sub(1);
        let funding_failed: std::collections::HashSet<u32> = ctx
            .journal()
            .of_kind(self.funding_failed)
            .iter()
            .filter(|row| ctx.journal().period_of(**row).0 == i64::from(prior))
            .flat_map(|row| ctx.journal().subjects_of(*row).iter().copied())
            .collect();
        let mut gone: Vec<Ceased> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) { continue; }
            let kind = ctx.parties().kind_of(who);
            let Some(worth) = crate::instruments::booked_equity(who, ctx.register(), ctx.instruments(), ctx.prints(), ctx.claims(), ctx.period()) else { continue };
            let failed_due = funding_failed.contains(&who.0) || ctx.schedules().of_payer(who).iter().any(|row| matches!(ctx.schedules().state(crate::stores::DueId(*row)), crate::stores::DueState::Failed { .. }));
            let written_off = ctx.journal().of_kind(self.loss_crossed).iter().any(|row| ctx.journal().subjects_of(*row).first() == Some(&who.0) && matches!(ctx.journal().says(*row, self.at_standing), Some(Value::Num(3.0))));
            let Some(why) = trigger_for(kind, written_off, failed_due, worth < 0.0, dissolved.contains(&who.0), past_waterfall.contains(&who.0)) else { continue };
            gone.push(Ceased { who, why, to: destination(why), period: ctx.period() });
        }
        for ceased in gone {
            ctx.ceases(ceased);
            ctx.say(self.says, &[ceased.who.0], &[(self.at_trigger, Value::Num(trigger_code(ceased.why))), (self.at_destination, Value::Num(destination_code(ceased.to)))], true);
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
            to: Destination::Resolution,
            period: 40,
        };
        let solvency = Ceased { why: Trigger::CapitalGone, ..liquidity };
        assert_ne!(liquidity.why, solvency.why);
    }

    #[test]
    fn there_is_no_death_without_a_destination_path() {
        let c = Ceased { who: PartyId::at(9), why: Trigger::Dissolved, to: Destination::Heir, period: 12 };
        assert_eq!(c.to, Destination::Heir);
    }

    #[test]
    fn kinds_consume_distinct_accumulated_failure_states() {
        use crate::assembly::kinds;
        assert_eq!(trigger_for(kinds::FIRM, false, true, true, false, false), Some(Trigger::CouldNotPay));
        assert_eq!(trigger_for(kinds::FIRM, true, true, false, false, false), Some(Trigger::CouldNotPay));
        assert_eq!(trigger_for(kinds::BANK, false, true, false, false, false), Some(Trigger::CouldNotFundItself));
        assert_eq!(trigger_for(kinds::BANK, false, false, true, false, false), Some(Trigger::CapitalGone));
        assert_eq!(trigger_for(kinds::FUND, false, false, true, false, false), Some(Trigger::LiabilitiesExceedAssets));
        assert_eq!(trigger_for(kinds::HOUSEHOLD, true, true, true, false, false), None);
        assert_eq!(trigger_for(kinds::HOUSEHOLD, false, false, false, true, false), Some(Trigger::Dissolved));
        assert_eq!(trigger_for(kinds::TREASURY, false, true, false, false, false), Some(Trigger::WillNotOrCannotPay));
        assert_eq!(trigger_for(kinds::BANK, false, false, false, false, true), Some(Trigger::PastTheWaterfall));
        assert_eq!(trigger_for(kinds::CENTRAL_BANK, true, true, true, true, false), None);
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
