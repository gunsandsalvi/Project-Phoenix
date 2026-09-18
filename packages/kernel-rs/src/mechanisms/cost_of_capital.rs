//! THE TRANSMISSION JOINT: a financial price changes, somebody's cost of capital changes, a real
//! decision changes, output changes with a lag.
//!
//! @spec XI-4 · Banks Lending C, D · 22 B · 46 A1 · Law 3, Law 4, Law 6, Law 19
//!
//! **This is the chain the whole model exists to have**, and XI-4 says it has three joints, each of
//! which can break it on its own. Two of them are here; the third — the lag from decision to output
//! — belongs to the firm's own production.
//!
//! **Joint one: a bank's cost of funds.** A bank prices a loan from ITS OWN economics — its blended
//! cost of funds across its own mix, the borrower's expected loss, the capital the loan consumes
//! times the return it needs on that, and an operating cost. **A bank with no cost-of-funds term
//! prices every loan as though it funded at the policy rate whatever its own position**, and then
//! nothing about that bank's funding condition can ever reach a borrower.
//!
//! **One rate per liability** is the corollary and it is Law 4: a liability whose interest cost is
//! computed one way for a reported margin and another way for the cash that actually leaves is two
//! prices for one thing, and the decision reads whichever it happens to reach. `Funding::blended`
//! is the one writer of what this bank's money costs it, and there is no second formula.
//!
//! **Joint two: investment as a project with a return and a hurdle.** A firm invests when it
//! expects the return to exceed its cost of capital. The cost comes from the markets — **what its
//! debt costs AT THE MARGIN, NOW, not the average coupon on debt already outstanding** — and what
//! its equity costs. The hurdle and the horizon are the management's own.
//!
//! *Investment as a rate on revenue, however many multipliers are attached, is this joint deleted.*

/// Joint one: what this bank's money costs IT. Every term is a fact about the bank's own book, so
/// two banks facing the same policy rate price the same loan differently — which is what lets a
/// funding condition reach a borrower at all.
#[derive(Clone, Copy, Debug)]
pub struct Funding {
    /// Its own mix, in the money it lends in. Each is (what it pays, how much of it there is).
    pub deposits: (f64, f64),
    pub wholesale: (f64, f64),
    pub capital: (f64, f64),
}

impl Funding {
    /// **The one writer of what this bank's money costs it.** Blended across its OWN mix — not the
    /// policy rate, and not an average of the market's.
    ///
    /// Missing where it funds with nothing: a bank with no liabilities has no cost of funds, and
    /// answering zero would say it funds for free (Appendix A).
    pub fn blended(&self) -> Option<f64> {
        let size = self.deposits.1 + self.wholesale.1 + self.capital.1;
        if size <= 0.0 {
            return None;
        }
        Some(
            (self.deposits.0 * self.deposits.1
                + self.wholesale.0 * self.wholesale.1
                + self.capital.0 * self.capital.1)
                / size,
        )
    }
}

/// Joint one: what a bank charges a borrower, built from its own economics. Each term is named, so
/// a reader can say which of them moved — a single number could not.
#[derive(Clone, Copy, Debug)]
pub struct Priced {
    pub cost_of_funds: f64,
    /// The BORROWER's expected loss, from this bank's own assessment of it (XI-13: the second
    /// opinion is the point, so it is the bank's and not a shared figure).
    pub expected_loss: f64,
    /// The capital this loan consumes times the return the bank needs on that capital.
    pub capital_charge: f64,
    pub operating: f64,
}

impl Priced {
    pub fn rate(&self) -> f64 {
        self.cost_of_funds + self.expected_loss + self.capital_charge + self.operating
    }
}

/// Joint two: a PROJECT, with a return and a hurdle. The return comes from expected demand and
/// price; the cost comes from the markets; the hurdle and the horizon are the management's own.
#[derive(Clone, Copy, Debug)]
pub struct Project {
    /// What it expects to get, per period, from its own outlook (§46 A1) — never a model forecast.
    pub returns_per_period: f64,
    pub costs: f64,
    /// The management's own patience, in periods. It is a PREFERENCE and it is theirs.
    pub horizon: f64,
    /// And its own risk aversion, above the cost of capital. Also theirs.
    pub hurdle: f64,
}

/// **What the firm's capital costs it, AT THE MARGIN, NOW.** Weighted by what it would raise, at
/// what the markets say today — not the average coupon on debt already outstanding, which is a
/// price struck in the past and cannot transmit anything that has happened since (Law 19).
pub fn at_the_margin(debt_now: f64, equity_now: f64, debt_share: f64) -> f64 {
    assert!(
        (0.0..=1.0).contains(&debt_share),
        "XI-4: a mix of {debt_share} debt is not a mix"
    );
    debt_now * debt_share + equity_now * (1.0 - debt_share)
}

/// Joint two: **it invests when it expects the return to exceed its cost of capital.** The
/// comparison IS the mechanism — a rate applied to revenue, however many multipliers are attached,
/// is this joint deleted, and then no financial price can reach a real decision.
///
/// Law 6: nothing is clamped. A project that does not clear the hurdle is not done smaller — it is
/// not done, and `false` is the answer.
pub fn worth_doing(p: &Project, cost_of_capital: f64) -> bool {
    if p.costs <= 0.0 {
        return false;
    }
    let over_the_horizon = p.returns_per_period * p.horizon;
    let expected = (over_the_horizon - p.costs) / p.costs / p.horizon;
    expected > cost_of_capital + p.hurdle
}

#[cfg(test)]
mod tests {
    use super::*;

    fn funding(deposit_rate: f64) -> Funding {
        Funding {
            deposits: (deposit_rate, 8_000.0),
            wholesale: (0.05, 1_500.0),
            capital: (0.12, 500.0),
        }
    }

    #[test]
    fn two_banks_at_the_same_policy_rate_price_a_loan_differently() {
        // Joint one: a bank with no cost-of-funds term prices every loan as though it funded at
        // the policy rate whatever its own position — and then nothing about its funding condition
        // can ever reach a borrower. These two face the same market and differ only in their own
        // deposit base, and the borrower pays the difference.
        let cheap = funding(0.01).blended().unwrap();
        let dear = funding(0.04).blended().unwrap();
        assert!(dear > cheap);
        let a = Priced { cost_of_funds: cheap, expected_loss: 0.02, capital_charge: 0.01, operating: 0.005 };
        let b = Priced { cost_of_funds: dear, ..a };
        assert!(b.rate() > a.rate());
        // And the terms are named, so a reader can say WHICH moved. A single number could not.
        //
        // Law 7: the tolerance is the DUST of this arithmetic — terms × ε × Σ|magnitudes| — and
        // never a band somebody chose. Four terms go into each rate, and comparing two of them by
        // exact equality is what asserting a float without its dust looks like.
        let moved = (b.rate() - a.rate()) - (dear - cheap);
        let dust = 10.0 * f64::EPSILON * (a.rate().abs() + b.rate().abs() + dear.abs() + cheap.abs());
        assert!(moved.abs() <= dust, "moved by {moved} against dust {dust}");
    }

    #[test]
    fn a_bank_that_funds_with_nothing_has_no_cost_of_funds_rather_than_a_free_one() {
        let empty = Funding { deposits: (0.01, 0.0), wholesale: (0.05, 0.0), capital: (0.12, 0.0) };
        // Appendix A: answering zero would say it funds for free, and somebody would price a loan
        // off that.
        assert!(empty.blended().is_none());
    }

    #[test]
    fn the_cost_of_capital_is_what_the_markets_say_now_and_not_the_old_coupon() {
        // XI-4, Law 19: the average coupon on debt already outstanding is a price struck in the
        // past, and it cannot transmit anything that has happened since. These are the same firm
        // before and after a repricing, and only the marginal figure moves.
        let before = at_the_margin(0.04, 0.10, 0.6);
        let after = at_the_margin(0.07, 0.10, 0.6);
        assert!(after > before);
        assert!((after - before - 0.018).abs() <= crate::num::dust(3, &[after, before, 0.018]));
    }

    #[test]
    fn a_project_is_done_when_the_return_clears_the_cost_and_the_hurdle_and_not_otherwise() {
        let p = Project { returns_per_period: 30.0, costs: 100.0, horizon: 10.0, hurdle: 0.02 };
        // 200 over 10 periods on 100 is 20% a period; it clears a 6% cost plus a 2% hurdle.
        assert!(worth_doing(&p, 0.06));
        // The same project does not clear a cost of capital of 25%.
        assert!(!worth_doing(&p, 0.25));
        // Law 6: a project that does not clear is not done SMALLER. It is not done.
        let marginal = Project { returns_per_period: 10.5, ..p };
        assert!(!worth_doing(&marginal, 0.06));
    }

    #[test]
    fn the_hurdle_and_the_horizon_are_the_managements_own() {
        // XI-4: read off its risk aversion and its patience. Two managements facing identical
        // markets and an identical project decide differently, which is what makes them decisions.
        let patient = Project { returns_per_period: 12.0, costs: 100.0, horizon: 20.0, hurdle: 0.01 };
        let impatient = Project { horizon: 3.0, hurdle: 0.10, ..patient };
        assert!(worth_doing(&patient, 0.05));
        assert!(!worth_doing(&impatient, 0.05));
    }

    #[test]
    #[should_panic(expected = "is not a mix")]
    fn a_mix_is_a_share_of_the_whole() {
        at_the_margin(0.04, 0.10, 1.4);
    }
}
