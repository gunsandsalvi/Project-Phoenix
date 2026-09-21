//! THE TRANSMISSION JOINT: a financial price changes, somebody's cost of capital changes, a real
//! decision changes, output changes with a lag.
//!
//! @spec XI-4 · Banks Lending C, D · 22 B · 46 A1 · Law 3, Law 4, Law 6, Law 19

use crate::calendar::Convention;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};

/// Joint one: what this bank's money costs IT.
#[derive(Clone, Copy, Debug)]
pub struct Funding {
    /// Its own mix, in the money it lends in.
    pub deposits: (f64, f64),
    pub wholesale: (f64, f64),
    pub capital: (f64, f64),
}

impl Funding {
    /// The one writer of what this bank's money costs it.
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

/// Joint one: what a bank charges a borrower, built from its own economics.
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

/// What the firm's capital costs it, AT THE MARGIN, NOW.
pub fn at_the_margin(debt_now: f64, equity_now: f64, debt_share: f64) -> f64 {
    assert!(
        (0.0..=1.0).contains(&debt_share),
        "XI-4: a mix of {debt_share} debt is not a mix"
    );
    debt_now * debt_share + equity_now * (1.0 - debt_share)
}

fn later_debt_price(
    current: Option<(InstrumentId, f64)>,
    candidate: InstrumentId,
    yield_now: f64,
) -> Option<(InstrumentId, f64)> {
    match current {
        Some((line, value)) if line.0 > candidate.0 => Some((line, value)),
        _ => Some((candidate, yield_now)),
    }
}

fn later_equity_price(
    current: Option<(InstrumentId, f64)>,
    candidate: InstrumentId,
    earnings_yield: f64,
) -> Option<(InstrumentId, f64)> {
    match current {
        Some((line, value)) if line.0 > candidate.0 => Some((line, value)),
        _ => Some((candidate, earnings_yield)),
    }
}

/// WHAT A COMPANY'S CAPITAL COSTS IT, AT THE MARGIN, NOW.
pub struct CostOfCapital {
    pub kind: u32,
    pub accounts: u32,
    pub at_income: u32,
    pub at_shares: u32,
}

impl Mechanism for CostOfCapital {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::instruments::Class;
        let today = ctx.today();

        // What each company last published, and over how many shares.
        let mut published: std::collections::HashMap<u32, (f64, f64)> =
            std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.accounts) {
            if let (Some(&who), Some(Value::Num(income)), Some(Value::Num(shares))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, self.at_income),
                ctx.journal().says(row, self.at_shares),
            ) {
                published.insert(who, (income, shares));
            }
        }

        let mut costs: Vec<(PartyId, f64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            let mut marginal_debt: Option<(InstrumentId, f64)> = None;
            let mut marginal_equity: Option<(InstrumentId, f64)> = None;
            let mut debt_value = 0.0;
            let mut equity_value = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let Some(print) = ctx.prints().of_line(what, ctx.week()) else {
                    continue;
                };
                match ctx.instruments().class_of(what) {
                    // 5: the yield derives FROM the price, which is the direction Law 3 requires —
                    // what the paper crossed at against what it repays.
                    Class::Claim => {
                        debt_value += print.price * ctx.register().held_total(what).0;
                        let Some(matures) = ctx.instruments().matures_on(what) else {
                            continue;
                        };
                        // A unit of a claim repays one of par, and what the holder waits is from
                        // TODAY to maturity — a yield over the whole life of a line priced this
                        // week is a rate for a wait nobody is doing.
                        if let Some(y) = crate::instruments::yield_to(
                            print.price,
                            1.0,
                            today,
                            matures,
                            Convention::Actual365,
                        ) {
                            marginal_debt = later_debt_price(marginal_debt, what, y);
                        }
                    }
                    // And the cost of equity is the EARNINGS YIELD — what it published over what a
                    // share last cost.
                    Class::Share => {
                        equity_value += print.price * ctx.register().held_total(what).0;
                        if let Some(&(income, shares)) = published.get(&row) {
                            if shares > 0.0 && print.price > 0.0 {
                                marginal_equity = later_equity_price(
                                    marginal_equity,
                                    what,
                                    income / shares / print.price,
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
            let (Some((_, debt_now)), Some((_, equity_now))) = (marginal_debt, marginal_equity)
            else {
                continue;
            };
            let total = debt_value + equity_value;
            if total <= 0.0 {
                continue;
            }
            let debt_share = debt_value / total;
            costs.push((who, at_the_margin(debt_now, equity_now, debt_share)));
        }

        for (who, cost) in costs {
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(cost))], true);
        }
    }
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
        let cheap = funding(0.01).blended().unwrap();
        let dear = funding(0.04).blended().unwrap();
        assert!(dear > cheap);
        let a = Priced {
            cost_of_funds: cheap,
            expected_loss: 0.02,
            capital_charge: 0.01,
            operating: 0.005,
        };
        let b = Priced {
            cost_of_funds: dear,
            ..a
        };
        assert!(b.rate() > a.rate());
        // And the terms are named, so a reader can say WHICH moved.
        let moved = (b.rate() - a.rate()) - (dear - cheap);
        let dust =
            10.0 * f64::EPSILON * (a.rate().abs() + b.rate().abs() + dear.abs() + cheap.abs());
        assert!(moved.abs() <= dust, "moved by {moved} against dust {dust}");
    }

    #[test]
    fn a_bank_that_funds_with_nothing_has_no_cost_of_funds_rather_than_a_free_one() {
        let empty = Funding {
            deposits: (0.01, 0.0),
            wholesale: (0.05, 0.0),
            capital: (0.12, 0.0),
        };
        // Answering zero would say it funds for free, and somebody would price a loan off that.
        assert!(empty.blended().is_none());
    }

    #[test]
    fn the_cost_of_capital_is_what_the_markets_say_now_and_not_the_old_coupon() {
        // The average coupon on debt already outstanding is a price struck in the past, and it
        // cannot transmit anything that has happened since.
        let before = at_the_margin(0.04, 0.10, 0.6);
        let after = at_the_margin(0.07, 0.10, 0.6);
        assert!(after > before);
        assert!((after - before - 0.018).abs() <= crate::num::dust(3, &[after, before, 0.018]));
    }

    #[test]
    fn the_newest_cleared_debt_line_is_the_marginal_debt_price() {
        let older = later_debt_price(None, InstrumentId::at(3), 0.04);
        let newest = later_debt_price(older, InstrumentId::at(8), 0.07);
        assert_eq!(newest, Some((InstrumentId::at(8), 0.07)));
        assert_eq!(later_debt_price(newest, InstrumentId::at(2), 0.02), newest);
    }

    #[test]
    fn the_newest_cleared_share_line_is_the_marginal_equity_price() {
        let older = later_equity_price(None, InstrumentId::at(5), 0.09);
        let newest = later_equity_price(older, InstrumentId::at(9), 0.12);
        assert_eq!(newest, Some((InstrumentId::at(9), 0.12)));
        assert_eq!(
            later_equity_price(newest, InstrumentId::at(1), 0.04),
            newest
        );
    }

    #[test]
    #[should_panic(expected = "is not a mix")]
    fn a_mix_is_a_share_of_the_whole() {
        at_the_margin(0.04, 0.10, 1.4);
    }
}
