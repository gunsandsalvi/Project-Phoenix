//! PRIVATE EQUITY: capital is committed, not paid, the debt is the TARGET's, and an unlisted mark is
//! not a cleared price.
//!
//! @spec 29 A1 · 29 A2 · 29 A2.a · 29 A2.b · 29 A3 · 29 A4 · 29 A5 · 29 B1 · 29 B2 · 29 B2.a ·
//! @spec 29 B2.b · 29 B3 · 29 B4 · 29 B5 · 29 C1 · 29 C2 · 29 C3 · 29 C4 · 29 C5 · 29 C5.a · 29 D1 ·
//! @spec 29 D2 · 29 D3 · XI-3 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::ids::PartyId;

/// A fund with committed capital from named investors — committed, not paid: it is CALLED when a
/// deal needs it.
#[derive(Clone, Debug)]
pub struct Fund {
    pub who: PartyId,
    /// A manager earning a fee on COMMITTED capital and a share of the gains.
    pub manager: PartyId,
    pub commitments: Vec<Commitment>,
    /// The fund has a LIFE: it invests, it holds, it exits, and it winds up.
    pub winds_up_in_periods: u32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Commitment {
    pub investor: PartyId,
    pub committed: f64,
    pub called_so_far: f64,
}

impl Commitment {
    pub fn uncalled(&self) -> f64 {
        self.committed - self.called_so_far
    }
}

/// The call, pro rata on uncalled commitments. A2.a: the investor must hold liquidity against calls
/// it did not choose the timing of.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Called {
    pub from: PartyId,
    pub owed: f64,
}

pub fn call(f: &Fund, needs: f64) -> Option<Vec<Called>> {
    let uncalled: f64 = f.commitments.iter().map(|c| c.uncalled()).sum();
    if uncalled < needs {
        // The fund cannot call what nobody committed. It is short, and the deal is the smaller for
        // it.
        return None;
    }
    Some(
        f.commitments
            .iter()
            .filter(|c| c.uncalled() > 0.0)
            .map(|c| Called { from: c.investor, owed: needs * c.uncalled() / uncalled })
            .collect(),
    )
}

/// A call bounded by the investor's spare cash is not an obligation. The investor committed; it must
/// find the money — from cash, by selling, or by borrowing — and failing that it is in default on
/// its commitment, which is a real state with real consequences.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Met {
    FromCash { amount: f64 },
    BySelling { amount: f64 },
    ByBorrowing { from: PartyId, amount: f64 },
    /// Not a smaller call: a default on the commitment.
    Defaulted { short_by: f64 },
}

pub fn meet(c: &Called, cash: f64, can_sell: f64, lender: Option<PartyId>, will_lend: f64) -> Met {
    if cash >= c.owed {
        return Met::FromCash { amount: c.owed };
    }
    if cash + can_sell >= c.owed {
        return Met::BySelling { amount: c.owed - cash };
    }
    let so_far = cash + can_sell;
    match lender {
        Some(from) if so_far + will_lend >= c.owed => Met::ByBorrowing { from, amount: c.owed - so_far },
        _ => Met::Defaulted { short_by: c.owed - so_far - will_lend },
    }
}

/// The acquired firm is held in a NAMED vehicle, each a party with its own balance sheet — and most
/// of the price is debt raised against the TARGET itself.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Buyout {
    pub vehicle: PartyId,
    pub target: PartyId,
    pub price: f64,
    /// The target's liability, not the fund's.
    pub debt_on_the_target: f64,
    /// The equity cheque is the rest, funded by the capital call.
    pub equity_cheque: f64,
    pub lender: PartyId,
}

/// The deal only happens if lenders will lend, at a price — the credit market decides which deals
/// happen. `None` is the deal that does not.
pub fn buy(
    vehicle: PartyId,
    target: PartyId,
    price: f64,
    lenders_will_lend: f64,
    lender: PartyId,
    equity_available: f64,
) -> Option<Buyout> {
    let cheque = price - lenders_will_lend;
    if cheque > equity_available {
        return None;
    }
    Some(Buyout { vehicle, target, price, debt_on_the_target: lenders_will_lend, equity_cheque: cheque, lender })
}

/// The sources and uses of a deal must balance EXACTLY, and the money must come out of named
/// accounts. A VERIFY on Law 7's derived dust: `None` when it balances, the discrepancy when it does
/// not.
pub fn sources_and_uses(b: &Buyout, terms: usize) -> Option<f64> {
    let sources = b.debt_on_the_target + b.equity_cheque;
    let off = sources - b.price;
    if off.abs() <= crate::num::dust(terms, &[sources, b.price]) {
        return None;
    }
    Some(off)
}

/// The target's balance sheet is transformed at the moment of purchase: leverage up, and the service
/// that comes with it.
pub fn transformed(b: &Buyout) -> (f64, f64) {
    // The new owner's equity is its cheque and the debt is on the company, which now services it.
    // What the target's equity WAS does not survive the purchase, so it is not an input here.
    (b.equity_cheque, b.debt_on_the_target)
}

/// The firm operates and services its debt out of cash flow, and the higher leverage makes that
/// binding — coverage is a read that can fall below one.
pub fn coverage(operating_cash: f64, interest: f64, principal: f64) -> Option<f64> {
    let service = interest + principal;
    if service <= 0.0 {
        return None;
    }
    Some(operating_cash / service)
}

/// It can recapitalise — raise more debt to pay itself a distribution — which is a real cash
/// movement to a named holder and leaves the company with more debt than before.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Recapitalised {
    pub to: PartyId,
    pub distribution: f64,
    pub debt_now: f64,
}

pub fn recapitalise(b: &Buyout, raised: f64, to: PartyId) -> Recapitalised {
    Recapitalised { to, distribution: raised, debt_now: b.debt_on_the_target + raised }
}

/// It can fail: the leverage makes default a real outcome, the loss falls on the lenders and the
/// equity is wiped — and the failed buyout kills the COMPANY, not the fund.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Failure {
    pub company_ceases: PartyId,
    pub lender_loses: f64,
    pub fund_loses: f64,
    /// The fund itself survives: the debt was never its liability.
    pub fund_survives: bool,
}

pub fn fails(b: &Buyout, estate_fetched: f64) -> Failure {
    let short = b.debt_on_the_target - estate_fetched;
    Failure {
        company_ceases: b.target,
        lender_loses: if short > 0.0 { short } else { 0.0 },
        fund_loses: b.equity_cheque,
        fund_survives: true,
    }
}

/// The holding has a value that is not a market price — no clearing, so it is a MARK — and an
/// unlisted mark must never be treated as a cleared price. A separate type is how that is kept true:
/// nothing that takes a print will accept one of these.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Mark {
    pub by: PartyId,
    pub value: f64,
    pub period: u32,
}

/// It sells — to another fund, to a corporate buyer, or to the public market — and the exit produces
/// a CLEARED PRICE, which is the first real price the holding has had. The proceeds are distributed
/// to the investors, in cash, into their accounts.
#[derive(Clone, Debug, PartialEq)]
pub struct Exit {
    pub cleared_at: f64,
    pub distributed: Vec<(PartyId, f64)>,
}

pub fn exit(f: &Fund, cleared_at: f64, held_by_fund: f64) -> Exit {
    let proceeds = cleared_at * held_by_fund;
    let called: f64 = f.commitments.iter().map(|c| c.called_so_far).sum();
    let distributed = if called > 0.0 {
        f.commitments
            .iter()
            .map(|c| (c.investor, proceeds * c.called_so_far / called))
            .collect()
    } else {
        Vec::new()
    };
    Exit { cleared_at, distributed }
}

/// What the mark said against what the exit cleared at. The first real price the holding has had,
/// and the difference is the measure of what the mark was worth.
pub fn mark_against_exit(m: &Mark, e: &Exit, held: f64) -> f64 {
    e.cleared_at * held - m.value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn fund() -> Fund {
        Fund {
            who: party(65),
            manager: party(66),
            commitments: vec![
                Commitment { investor: party(70), committed: 6_000.0, called_so_far: 1_000.0 },
                Commitment { investor: party(71), committed: 4_000.0, called_so_far: 1_000.0 },
            ],
            winds_up_in_periods: 400,
        }
    }

    #[test]
    fn capital_is_committed_not_paid_and_the_call_is_pro_rata_on_what_is_uncalled() {
        // The investor must hold liquidity against calls it did not choose the timing of.
        let called = call(&fund(), 800.0).unwrap();
        assert_eq!(called[0].from, party(70));
        assert!((called[0].owed - 500.0).abs() <= crate::num::dust(2, &[800.0, 5_000.0]));
        assert!((called[1].owed - 300.0).abs() <= crate::num::dust(2, &[800.0, 3_000.0]));
        // A fund cannot call what nobody committed.
        assert!(call(&fund(), 50_000.0).is_none());
    }

    #[test]
    fn a_call_bounded_by_the_investors_spare_cash_is_not_an_obligation() {
        // The investor committed. It must find the money — and failing that it is in DEFAULT on its
        // commitment, not the beneficiary of a smaller call.
        let c = Called { from: party(70), owed: 500.0 };
        assert_eq!(meet(&c, 900.0, 0.0, None, 0.0), Met::FromCash { amount: 500.0 });
        assert_eq!(meet(&c, 100.0, 900.0, None, 0.0), Met::BySelling { amount: 400.0 });
        assert_eq!(
            meet(&c, 100.0, 0.0, Some(party(80)), 900.0),
            Met::ByBorrowing { from: party(80), amount: 400.0 }
        );
        assert_eq!(meet(&c, 100.0, 50.0, None, 0.0), Met::Defaulted { short_by: 350.0 });
    }

    #[test]
    fn the_credit_market_decides_which_deals_happen() {
        // The deal only happens if lenders will lend, at a price.
        let done = buy(party(67), party(9), 10_000.0, 7_000.0, party(80), 5_000.0).unwrap();
        assert_eq!(done.debt_on_the_target, 7_000.0);
        assert_eq!(done.equity_cheque, 3_000.0);
        // Lenders pull back and the equity cannot cover the gap: no deal.
        assert!(buy(party(67), party(9), 10_000.0, 2_000.0, party(80), 5_000.0).is_none());
    }

    #[test]
    fn the_sources_and_uses_balance_exactly() {
        // And the money comes out of named accounts.
        let b = buy(party(67), party(9), 10_000.0, 7_000.0, party(80), 5_000.0).unwrap();
        assert!(sources_and_uses(&b, 3).is_none());
        let leaking = Buyout { equity_cheque: 2_500.0, ..b };
        assert_eq!(sources_and_uses(&leaking, 3), Some(-500.0));
    }

    #[test]
    fn the_debt_is_the_targets_so_a_failed_buyout_kills_the_company_and_not_the_fund() {
        let b = buy(party(67), party(9), 10_000.0, 7_000.0, party(80), 5_000.0).unwrap();
        let f = fails(&b, 4_000.0);
        assert_eq!(f.company_ceases, party(9));
        assert_eq!(f.lender_loses, 3_000.0);
        assert_eq!(f.fund_loses, 3_000.0);
        assert!(f.fund_survives);
    }

    #[test]
    fn the_balance_sheet_is_transformed_at_the_moment_of_purchase_and_the_service_binds() {
        // Leverage up, and coverage can fall below one.
        let b = buy(party(67), party(9), 10_000.0, 7_000.0, party(80), 5_000.0).unwrap();
        let (equity, debt) = transformed(&b);
        assert_eq!(equity, 3_000.0);
        assert_eq!(debt, 7_000.0);
        assert!(coverage(500.0, 400.0, 300.0).unwrap() < 1.0);
        assert!(coverage(500.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn a_dividend_recapitalisation_pays_the_owner_and_leaves_more_debt_behind() {
        // A real cash movement to a named holder.
        let b = buy(party(67), party(9), 10_000.0, 7_000.0, party(80), 5_000.0).unwrap();
        let r = recapitalise(&b, 2_000.0, party(65));
        assert_eq!(r.to, party(65));
        assert_eq!(r.distribution, 2_000.0);
        assert_eq!(r.debt_now, 9_000.0);
    }

    #[test]
    fn an_unlisted_mark_is_not_a_cleared_price_and_the_exit_is_the_first_real_one() {
        // A separate type is how that is kept true, and the exit is what tests the mark.
        let m = Mark { by: party(66), value: 12_000.0, period: 40 };
        let e = exit(&fund(), 1.0, 9_000.0);
        assert_eq!(e.cleared_at, 1.0);
        assert_eq!(mark_against_exit(&m, &e, 9_000.0), -3_000.0);
    }

    #[test]
    fn the_proceeds_are_distributed_in_cash_to_the_investors_that_paid_in() {
        // Into their accounts, pro rata on what was actually called.
        let e = exit(&fund(), 2.0, 5_000.0);
        assert_eq!(e.distributed.len(), 2);
        assert_eq!(e.distributed[0], (party(70), 5_000.0));
        assert_eq!(e.distributed[1], (party(71), 5_000.0));
        // A fund that never called anything has nothing to distribute.
        let unfunded = Fund {
            commitments: vec![Commitment { investor: party(70), committed: 6_000.0, called_so_far: 0.0 }],
            ..fund()
        };
        assert!(exit(&unfunded, 2.0, 5_000.0).distributed.is_empty());
    }
}
