//! SECURITIES LENDING: legal title moves and the economics do not — and no short without a
//! borrow.
//!
//! @spec 14 A1 · 14 A2 · 14 A3 · 14 A4 · 14 A5 · 14 A5.a · 14 A5.b · 14 B1 · 14 B2 · 14 B2.a ·
//! @spec 14 B3 · 14 B4 · 14 C1 · 14 C2 · 14 C2.a · 14 C3 · 14 C4 · 14 C5 · 14 D1 · 14 D2 · 14 D2.a ·
//! @spec 14 D3 · 14 E1 · 14 E2 · 14 E3 · XI-2 · Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! The manufactured payment is the defining property. Title passes, so the ISSUER pays the
//! registered holder — the borrower — and the borrower passes it on. Without it the property is
//! inverted and the lender pays a fee to lose its income, so `manufactured` is what the borrower
//! owes and it is a real flow between two named parties.
//!
//! No short without a borrow: a negative position nobody lent is an invented security. The
//! lendable pool is a read of who actually holds the paper and is willing, and it caps how large a
//! short can get — which is arithmetic about a finite quantity, not a bound anybody chose.
//!
//! No collateral that is not held: posted collateral leaves the poster's free balance and
//! cannot be counted as available by both sides. Cash collateral is reinvested and that
//! reinvestment is a position with its own risk — this is where a lending programme actually loses
//! money.
//!
//! Re-pledging makes one security back a CHAIN, and the chain must be traceable, because it is how
//! a single default reaches parties that never traded with the defaulter.
//!
//! No free borrow: a fee of zero is a cleared price only if somebody posted it, so `Loan`
//! carries the fee that cleared and `clearing` is where one comes from.

use crate::ids::{InstrumentId, PartyId};

/// The lender delivers the security and the borrower delivers collateral, and legal title
/// passes — the borrower can sell what it borrowed, which is the entire point.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Loan {
    pub lender: PartyId,
    pub borrower: PartyId,
    /// An agent may sit in the middle and take part of the fee.
    pub agent: Option<PartyId>,
    pub what: InstrumentId,
    pub units: f64,
    /// The fee, and it is a price that cleared.
    pub fee: f64,
    /// What was posted, worth more than the loan.
    pub collateral: Collateral,
}

/// The collateral. Cash or other securities — and cash collateral is REINVESTED by the
/// lender, which is a position with its own risk.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Collateral {
    pub value: f64,
    pub is_cash: bool,
    /// Posted collateral leaves the poster's free balance. It cannot be counted as available
    /// by both sides, so the encumbrance is on the row.
    pub encumbered_to: PartyId,
}

impl Loan {
    /// The collateral is worth more than the loan — a haircut — because the lender must be
    /// able to sell it and be whole. Collateral exactly equal to the loan, re-marked to the same
    /// price, means the gap between two marks is covered by nothing.
    pub fn margin_over(&self, security_worth: f64) -> f64 {
        self.collateral.value - security_worth
    }

    /// The fee, or — where the collateral is cash — the same number seen from the other
    /// side, as a REBATE on that cash. Two forms, one price.
    pub fn rebate(&self, cash_rate: f64) -> Option<f64> {
        if !self.collateral.is_cash {
            return None;
        }
        Some(cash_rate - self.fee)
    }
}

/// The economics stay with the lender. The issuer pays the registered holder — the borrower —
/// and the borrower passes it on, so the lender's cash flows are unchanged. Without this the lender
/// pays a fee to lose its income.
pub fn manufactured(l: &Loan, paid_per_unit: f64) -> (PartyId, PartyId, f64) {
    (l.borrower, l.lender, paid_per_unit * l.units)
}

/// The lendable pool is a read of who actually holds the security and is willing, within
/// a mandate, against acceptable collateral, with a limit. Law 19: a walk over holders, never a stated
/// availability.
#[derive(Clone, Copy, Debug)]
pub struct Willing {
    pub holder: PartyId,
    pub holds: f64,
    /// Its own limit on how much of that it will lend. Its own, and it may be none.
    pub will_lend: f64,
}

pub fn lendable(pool: &[Willing]) -> f64 {
    pool.iter()
        .map(|w| if w.will_lend < w.holds { w.will_lend } else { w.holds })
        .sum()
}

/// No short without a borrow, and the pool caps how large a short can get — which is a
/// real constraint and arithmetic about a finite quantity, not a bound anybody chose. `None` is a
/// short that cannot be opened at all.
pub fn can_short(wants: f64, pool: &[Willing], already_lent: f64) -> Option<f64> {
    let free = lendable(pool) - already_lent;
    if free <= 0.0 {
        return None;
    }
    Some(if free < wants { free } else { wants })
}

/// The fee clears — scarce paper is expensive to borrow, abundant paper is cheap. E3: a fee
/// of zero is a cleared price only if somebody posted it, so `None` means nobody did and there is no
/// borrow rather than a free one.
pub fn clearing(demand: f64, pool: &[Willing], schedules: &[(PartyId, f64, f64)]) -> Option<f64> {
    let free = lendable(pool);
    if free <= 0.0 || demand <= 0.0 {
        return None;
    }
    // Lenders in order of the fee they will take; the one that lends the marginal unit sets it.
    let mut posted: Vec<&(PartyId, f64, f64)> = schedules.iter().collect();
    posted.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut left = demand;
    let mut fee = None;
    for (_, units, at) in posted {
        if left <= 0.0 {
            break;
        }
        let taken = if *units < left { *units } else { left };
        if taken > 0.0 {
            fee = Some(*at);
            left -= taken;
        }
    }
    fee
}

/// Both sides are marked every period — when the borrowed security rises, the borrower
/// posts more collateral, and the margin flow is real money moving between two named parties.
pub fn margin_call(l: &Loan, security_worth_now: f64, haircut: f64) -> Option<(PartyId, PartyId, f64)> {
    let wanted = security_worth_now * haircut;
    let short = wanted - l.collateral.value;
    if short > 0.0 {
        return Some((l.borrower, l.lender, short));
    }
    if short < 0.0 {
        // And back the other way when it falls: the lender returns what it no longer needs.
        return Some((l.lender, l.borrower, -short));
    }
    None
}

/// Cash collateral is reinvested, and this is where a lending programme actually loses money.
/// The reinvestment is a position with its own risk, held by the lender — not a balance that sits
/// still.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reinvested {
    pub by: PartyId,
    pub into: InstrumentId,
    pub at_cost: f64,
}

impl Reinvested {
    pub fn worth_now(&self, marked_at: f64) -> f64 {
        marked_at - self.at_cost
    }
}

/// Re-pledging means the same security backs a CHAIN of obligations, and the chain must be
/// traceable — it is how a single default reaches parties that never traded with the defaulter.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Pledge {
    pub from: PartyId,
    pub to: PartyId,
    pub what: InstrumentId,
    pub units: f64,
}

/// Who a default reaches, following the chain from the party that failed. Every party named, which is
/// what "traceable" means.
pub fn chain_from(failed: PartyId, pledges: &[Pledge]) -> Vec<PartyId> {
    let mut reached = Vec::new();
    let mut walking = vec![failed];
    let mut steps = 0usize;
    while let Some(at) = walking.pop() {
        steps += 1;
        assert!(steps <= pledges.len() + 1, "14 C5: a re-pledge chain cannot revisit more links than exist");
        for p in pledges.iter().filter(|p| p.from == at) {
            reached.push(p.to);
            walking.push(p.to);
        }
    }
    reached
}

/// The borrower can fail to return, and then the lender keeps the collateral and buys the
/// security back in the market, at whatever it costs. A failed return TERMINATES.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FailedReturn {
    pub lender: PartyId,
    pub kept: f64,
    /// What buying it back actually cost — a real order in a real book, at whatever it cleared.
    pub bought_back_for: f64,
}

impl FailedReturn {
    /// The loss is what it cost less what the collateral covered, and it lands on the lender.
    pub fn loss(&self) -> f64 {
        self.bought_back_for - self.kept
    }
}

/// A squeeze — shorts must buy, the lendable pool is small, the fee and the price both
/// rise. A CONSEQUENCE of the pool and the collateral, to be measured, never a scripted event: this
/// reads how much of the pool the shorts already hold.
pub fn tightness(shorted: f64, pool: &[Willing]) -> Option<f64> {
    let free = lendable(pool);
    if free <= 0.0 {
        return None;
    }
    Some(shorted / free)
}

/// A recall forces the borrower to find the security elsewhere or close its short. Both are
/// real acts, and which one happens depends on whether anybody else will lend.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OnRecall {
    FoundElsewhere,
    MustClose,
}

pub fn recall(units: f64, pool: &[Willing], already_lent: f64) -> OnRecall {
    match can_short(units, pool, already_lent) {
        Some(found) if found >= units => OnRecall::FoundElsewhere,
        _ => OnRecall::MustClose,
    }
}

/// No double-counting the loaned security. The lender's economic exposure and the borrower's
/// legal title are two reads of ONE security, and holdings must still sum to issued. This is the read
/// that says which is which — there is no second unit anywhere.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Holds {
    /// Legal title — the registered holder, who receives from the issuer and passes it on.
    Title,
    /// The economics — unchanged cash flows, via the manufactured payment.
    Economics,
}

pub fn who_holds(l: &Loan, party: PartyId) -> Option<Holds> {
    if party == l.borrower {
        return Some(Holds::Title);
    }
    if party == l.lender {
        return Some(Holds::Economics);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn paper() -> InstrumentId {
        InstrumentId::at(7)
    }

    fn loan(cash: bool) -> Loan {
        Loan {
            lender: party(1),
            borrower: party(2),
            agent: Some(party(3)),
            what: paper(),
            units: 1_000.0,
            fee: 0.004,
            collateral: Collateral { value: 10_200.0, is_cash: cash, encumbered_to: party(1) },
        }
    }

    fn pool() -> Vec<Willing> {
        vec![
            Willing { holder: party(10), holds: 5_000.0, will_lend: 2_000.0 },
            Willing { holder: party(11), holds: 1_000.0, will_lend: 4_000.0 },
            Willing { holder: party(12), holds: 9_000.0, will_lend: 0.0 },
        ]
    }

    #[test]
    fn title_passes_and_the_economics_stay_with_the_lender() {
        // Two reads of ONE security. Without the manufactured payment the defining
        // property is inverted and the lender pays a fee to lose its income.
        let l = loan(false);
        assert_eq!(who_holds(&l, l.borrower), Some(Holds::Title));
        assert_eq!(who_holds(&l, l.lender), Some(Holds::Economics));
        assert!(who_holds(&l, party(99)).is_none());
        let (from, to, amount) = manufactured(&l, 0.30);
        assert_eq!(from, l.borrower);
        assert_eq!(to, l.lender);
        assert_eq!(amount, 300.0);
    }

    #[test]
    fn the_lendable_pool_is_a_read_of_who_holds_it_and_is_willing() {
        // Within a mandate, with a limit — and a holder willing to lend more than it holds
        // still lends only what it has.
        assert_eq!(lendable(&pool()), 3_000.0);
    }

    #[test]
    fn there_is_no_short_without_a_borrow_and_the_pool_caps_how_large_one_can_get() {
        // A negative position nobody lent is an invented security, and the limit here is
        // arithmetic about a finite quantity rather than a bound anybody chose.
        assert_eq!(can_short(1_000.0, &pool(), 0.0), Some(1_000.0));
        assert_eq!(can_short(9_000.0, &pool(), 0.0), Some(3_000.0));
        assert!(can_short(500.0, &pool(), 3_000.0).is_none());
    }

    #[test]
    fn scarce_paper_is_expensive_to_borrow_and_a_free_borrow_needs_somebody_to_post_it() {
        // The fee clears. Demand that reaches the dearer lender prints the dearer fee.
        let schedules = [(party(10), 500.0, 0.002), (party(11), 2_500.0, 0.030)];
        assert_eq!(clearing(400.0, &pool(), &schedules), Some(0.002));
        assert_eq!(clearing(2_000.0, &pool(), &schedules), Some(0.030));
        // Nobody willing means no borrow, rather than a free one.
        let none = [Willing { holder: party(12), holds: 9_000.0, will_lend: 0.0 }];
        assert!(clearing(400.0, &none, &schedules).is_none());
    }

    #[test]
    fn the_fee_and_the_rebate_are_one_price_seen_from_two_sides() {
        // When the collateral is cash the price is expressed as a rebate on that cash.
        let cash = loan(true);
        assert_eq!(cash.rebate(0.05), Some(0.046));
        // Non-cash collateral has no rebate — it has a fee, which is the same number the other way.
        assert!(loan(false).rebate(0.05).is_none());
    }

    #[test]
    fn the_collateral_is_worth_more_than_the_loan() {
        // Collateral exactly equal to the loan, re-marked to the same price, means the gap
        // between two marks is covered by nothing.
        assert!(loan(false).margin_over(10_000.0) > 0.0);
        assert_eq!(loan(false).margin_over(10_200.0), 0.0);
    }

    #[test]
    fn a_rise_in_the_borrowed_security_moves_real_money_between_two_named_parties() {
        // Both sides are marked every period, and the flow has two ends.
        let l = loan(false);
        let up = margin_call(&l, 11_000.0, 1.02).unwrap();
        assert_eq!(up.0, l.borrower);
        assert_eq!(up.1, l.lender);
        assert!(up.2 > 0.0);
        let down = margin_call(&l, 9_000.0, 1.02).unwrap();
        assert_eq!(down.0, l.lender);
        assert_eq!(down.1, l.borrower);
        // And a loan already margined to exactly its haircut calls for nothing: there is no flow to
        // make, and inventing a zero one would be a leg with no reason behind it.
        assert!(margin_call(&l, 10_000.0, 1.02).is_none());
    }

    #[test]
    fn posted_collateral_is_encumbered_to_one_party_and_not_available_to_both() {
        // It leaves the poster's free balance.
        let l = loan(true);
        assert_eq!(l.collateral.encumbered_to, l.lender);
    }

    #[test]
    fn reinvested_cash_collateral_is_where_a_lending_programme_loses_money() {
        // A position with its own risk, held by the lender — not a balance that sits still.
        let r = Reinvested { by: party(1), into: InstrumentId::at(20), at_cost: 10_200.0 };
        assert!(r.worth_now(10_400.0) > 0.0);
        assert!(r.worth_now(9_600.0) < 0.0);
    }

    #[test]
    fn a_re_pledge_chain_reaches_parties_that_never_traded_with_the_defaulter() {
        // Which is the whole reason it must be traceable.
        let pledges = [
            Pledge { from: party(2), to: party(4), what: paper(), units: 1_000.0 },
            Pledge { from: party(4), to: party(5), what: paper(), units: 1_000.0 },
            Pledge { from: party(8), to: party(9), what: paper(), units: 500.0 },
        ];
        let reached = chain_from(party(2), &pledges);
        assert_eq!(reached, vec![party(4), party(5)]);
    }

    #[test]
    fn a_failed_return_terminates_and_the_buy_back_costs_whatever_it_costs() {
        // The lender keeps the collateral and buys the security back at the market price.
        let cheap = FailedReturn { lender: party(1), kept: 10_200.0, bought_back_for: 9_800.0 };
        assert!(cheap.loss() < 0.0);
        let squeezed = FailedReturn { lender: party(1), kept: 10_200.0, bought_back_for: 14_000.0 };
        assert!(squeezed.loss() > 0.0);
    }

    #[test]
    fn a_squeeze_is_measured_from_the_pool_and_never_scripted() {
        // A consequence of B4 and C. More shorted against the same pool is tighter.
        let easy = tightness(500.0, &pool()).unwrap();
        let tight = tightness(2_900.0, &pool()).unwrap();
        assert!(tight > easy);
        let none = [Willing { holder: party(12), holds: 9_000.0, will_lend: 0.0 }];
        assert!(tightness(500.0, &none).is_none());
    }

    #[test]
    fn a_recall_is_met_elsewhere_or_forces_the_short_to_close() {
        // Both are real acts, and which happens depends on whether anybody else will lend.
        assert_eq!(recall(500.0, &pool(), 0.0), OnRecall::FoundElsewhere);
        assert_eq!(recall(500.0, &pool(), 3_000.0), OnRecall::MustClose);
    }
}
