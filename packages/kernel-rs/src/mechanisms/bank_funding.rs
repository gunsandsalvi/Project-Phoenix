//! BANKS — FUNDING AND LIQUIDITY: deposits are not one thing, the cost of funds is the bank's own, and
//! **a model with one deposit type cannot have a run.**
//!
//! @spec 24 A1 · 24 A1.a · 24 A1.b · 24 A1.c · 24 A1.d · 24 A2 · 24 A2.a · 24 A3 · 24 A4 · 24 A5 ·
//! @spec 24 B1 · 24 B1.a · 24 B1.b · 24 B2 · 24 B2.a · 24 B2.b · 24 B3 · 24 C1 · 24 C1.a · 24 C2 ·
//! @spec 24 C2.a · 24 C3 · 24 C3.a · 24 C4 · 24 D1 · 24 D2 · 24 D3 · 24 D4 · 24 D4.a · 24 D5 · 24 D6 ·
//! @spec 24 D6.a · 24 E1 · 24 E2 · 24 E2.a · 24 E3 · 24 E3.a · 24 E4 · 24 E4.a · 24 E5 · 24 F1 ·
//! @spec 24 F2 · 24 F3 · XI-15 · XI-2 · Law 4, Law 5, Law 6, Law 19
//!
//! **Stickiness differs by class, and it is the whole of liquidity risk** (A1.d). `Class` is the
//! deposit's own, carried on the line; a world with one of them cannot have a run, because there is
//! nothing for the fast money to be faster than.
//!
//! **Deposit insurance breaks the loop for retail and not for wholesale** (E4), **which is why a run is
//! a wholesale phenomenon first** (E4.a). Where the depositor is a cell the insured amount is `weight ×
//! min(member balance, limit)` — **exact because the cell is homogeneous** (A1.a, XI-15), and that
//! exactness is what makes the break real rather than notional.
//!
//! **One rate per liability** (B2.b): a loan whose interest cost is computed one way for a margin
//! statistic and another way for the cash that leaves is two representations of one price. `Source`
//! carries the rate it pays and `blended` is the only reader of them.
//!
//! **A bank that has no cost of funds prices every loan as if it funded at the policy rate whatever its
//! own position** (B2.a) — and then no funding condition anywhere can reach a borrower.
//!
//! **There is no unbounded, uncollateralised, unpriced credit line that makes failure-to-fund
//! unreachable** (D6.a). A facility with none of the classical conditions does not bound anything: it
//! deletes the whole branch above it, and with it the reason the buffer exists.
//!
//! **The deposit leaves with the reserves behind it** (E3.a), so the bank is shorter at the next close
//! — which is the loop, and `after_outflow` is what makes it show.

use crate::ids::PartyId;

/// A1: **deposits are not one thing.** The class is a fact about who banks there, and it decides how
/// fast the money leaves (A1.d) and whether insurance reaches it (E4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// A1.a: many, small, sticky, and insured up to a limit.
    Retail,
    /// A1.b: fewer, larger, operational — a firm banks where it transacts.
    Corporate,
    /// A1.c: few, very large, and RATE-SENSITIVE. E1: these leave fastest.
    Wholesale,
}

/// F1: **a deposit line by class, as a read of who actually banks there.** The depositor may be a cell
/// (XI-15), and then the balance is a TOTAL and the per-member figure is a read.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Line {
    pub depositor: PartyId,
    pub class: Class,
    pub balance: f64,
    /// XI-15: how many real depositors this line is. A weight is a count.
    pub members: f64,
    /// B1.a: the rate the BANK sets on it, which depositors respond to.
    pub rate: f64,
}

impl Line {
    /// A1.a: **the insured amount is `weight × min(member balance, limit)`, which is EXACT because the
    /// cell is homogeneous** — and that is what makes the break in the run loop real rather than
    /// notional. Only retail is insured (E4).
    pub fn insured(&self, limit_per_member: f64) -> f64 {
        if self.class != Class::Retail || self.members <= 0.0 {
            return 0.0;
        }
        let per_member = self.balance / self.members;
        let covered = if per_member < limit_per_member { per_member } else { limit_per_member };
        self.members * covered
    }

    /// E1, A1.d: how much of this line leaves when the depositors see something. Wholesale goes first
    /// and furthest; insured retail has the least reason to move (E4.a).
    pub fn leaves(&self, on_signals: f64, limit_per_member: f64) -> f64 {
        let exposed = self.balance - self.insured(limit_per_member);
        let eagerness = match self.class {
            Class::Wholesale => 1.0,
            Class::Corporate => 0.4,
            Class::Retail => 0.15,
        };
        let going = exposed * eagerness * on_signals;
        // Law 6: they cannot take more than they have. Arithmetic, not a cap.
        if going < self.balance {
            going
        } else {
            self.balance
        }
    }
}

/// A2, A3, A4, A5: **each source has a price, the prices differ, and the mix is a DECISION.**
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Funding {
    Deposits(Class),
    /// A2, A2.a: short, and it ROLLS — which is where a funding squeeze bites.
    Wholesale,
    /// A3: equity and subordinated debt, which **do not run**.
    Capital,
    /// A4: the central bank, on the corridor's terms.
    CentralBank,
}

/// B1: **the bank pays a rate on each source, and it is a real payment to a real holder** (Law 5).
/// B2.b: one rate per liability — this is where it lives, and there is no second.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Source {
    pub kind: Funding,
    pub amount: f64,
    pub rate: f64,
}

/// B2: **the blended cost of funds is a READ across the mix** — the bank's own, which is what lets a
/// funding condition reach a borrower at all. `None` where it funds with nothing: answering zero would
/// say it funds for free.
pub fn blended(mix: &[Source]) -> Option<f64> {
    let size: f64 = mix.iter().map(|s| s.amount).sum();
    if size <= 0.0 {
        return None;
    }
    Some(mix.iter().map(|s| s.rate * s.amount).sum::<f64>() / size)
}

/// B1.a: **a deposit rate the bank SETS** — bounded above by the cheaper of its own wholesale cost and
/// the money fund's yield, **on the contested share of its base.** Not a bound anybody imposed: past
/// that point the bank would rather fund wholesale, which is a decision and not a rule.
pub fn will_pay_on_deposits(own_wholesale_cost: f64, money_fund_yield: f64) -> f64 {
    if own_wholesale_cost < money_fund_yield {
        own_wholesale_cost
    } else {
        money_fund_yield
    }
}

/// B3: **net interest margin is what it earns minus its cost of funds, and it can be NEGATIVE.**
pub fn net_interest_margin(earned: f64, mix: &[Source]) -> Option<f64> {
    Some(earned - blended(mix)?)
}

/// C1, C1.a: **liquid assets — reserves, and securities it can sell or pledge — differ in how fast and
/// how surely they convert: a haircut and a market depth.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Liquid {
    pub value: f64,
    /// What it fetches when sold in a hurry, as a fraction of its mark. A fact about the asset's
    /// market, not a rule.
    pub converts_at: f64,
    /// And how much of it the market can take this period.
    pub depth: f64,
}

impl Liquid {
    /// What this asset would actually raise, now. Limited by the market's depth, which is arithmetic
    /// about a real quantity.
    pub fn raises(&self) -> f64 {
        let sellable = if self.value < self.depth { self.value } else { self.depth };
        sellable * self.converts_at
    }
}

/// C2, C2.a: **a buffer preference derived from its OWN liabilities, not a stated ratio** — a bank
/// funded by wholesale money needs more than one funded by insured retail, and that is the whole of
/// A1.d showing up as a number.
pub fn buffer_wanted(lines: &[Line], limit_per_member: f64, on_signals: f64) -> f64 {
    lines.iter().map(|l| l.leaves(on_signals, limit_per_member)).sum()
}

/// C3, C3.a: **maturity transformation is the business** — it funds long assets with short liabilities,
/// and **that gap is why it earns anything. A bank with none is not a bank.**
pub fn transformation(asset_years: f64, liability_years: f64) -> f64 {
    asset_years - liability_years
}

/// D1–D6: **what a bank that is short actually does**, in order, each a real act with a counterparty.
/// D6: **it can fail to fund itself, and that is a distinct failure from insolvency.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Short {
    BorrowsInTheMarket { amount: f64 },
    /// D2: sells or pledges liquid assets — a real order in a real book (XI-2).
    SellsLiquid { raising: f64 },
    /// D3: bids up for deposits, and pays for them.
    BidsForDeposits { paying: f64 },
    /// D4, D4.a: **it stops lending and lets the book run off — this is the credit crunch**, a funding
    /// problem transmitted into the credit decision.
    StopsLending { by: f64 },
    /// D5: the facility, collateralised and at a penalty.
    DrawsTheWindow { amount: f64 },
    /// D6: it cannot fund itself. Distinct from insolvency, and reachable — which D6.a insists on.
    CannotFund { short_by: f64 },
}

pub fn when_short(
    short_by: f64,
    market_will_lend: f64,
    liquid: &[Liquid],
    deposits_biddable: f64,
    window: f64,
    book_that_can_run_off: f64,
) -> Short {
    if market_will_lend >= short_by {
        return Short::BorrowsInTheMarket { amount: short_by };
    }
    let from_sales: f64 = liquid.iter().map(|l| l.raises()).sum();
    if market_will_lend + from_sales >= short_by {
        return Short::SellsLiquid { raising: short_by - market_will_lend };
    }
    let so_far = market_will_lend + from_sales;
    if so_far + deposits_biddable >= short_by {
        return Short::BidsForDeposits { paying: short_by - so_far };
    }
    let so_far = so_far + deposits_biddable;
    if so_far + window >= short_by {
        return Short::DrawsTheWindow { amount: short_by - so_far };
    }
    let so_far = so_far + window;
    if so_far + book_that_can_run_off >= short_by {
        return Short::StopsLending { by: short_by - so_far };
    }
    Short::CannotFund { short_by: short_by - so_far - book_that_can_run_off }
}

/// E2, E2.a: **they leave because they observe something, and what they observe must be OBSERVABLE** —
/// a capital ratio, a facility draw, a rate paid up, a rating action, a run of periods ending short.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    pub capital_ratio_published: f64,
    pub drew_the_window: bool,
    pub paid_up_for_deposits: bool,
    pub downgraded: bool,
    pub periods_ending_short: u32,
}

impl Observed {
    /// How many things a depositor can actually see. Each is a published fact, and a run needs at
    /// least one of them — E5: a run at one bank is information about others through exactly these.
    pub fn signals(&self, capital_that_worries: f64) -> f64 {
        let mut n = self.periods_ending_short as f64;
        if self.capital_ratio_published < capital_that_worries {
            n += 1.0;
        }
        if self.drew_the_window {
            n += 1.0;
        }
        if self.paid_up_for_deposits {
            n += 1.0;
        }
        if self.downgraded {
            n += 1.0;
        }
        n
    }
}

/// E3, E3.a: **the deposit leaves WITH THE RESERVES BEHIND IT**, so the bank is shorter at the next
/// close — and that is the loop. F2: the reserve balance is one row, moved only by settlement legs,
/// never a mirrored copy (Law 4).
pub fn after_outflow(reserves: f64, left: f64) -> f64 {
    reserves - left
}

/// F3: **assets equal liabilities plus equity, in the bank's own money, every period.** A VERIFY on
/// Law 7's derived dust — it answers with the discrepancy and repairs nothing.
pub fn balances(assets: f64, liabilities: f64, equity: f64, terms: usize) -> Option<f64> {
    let off = assets - (liabilities + equity);
    if off.abs() <= crate::num::dust(terms, &[assets, liabilities, equity]) {
        return None;
    }
    Some(off)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn line(depositor: u32, class: Class, balance: f64, members: f64) -> Line {
        Line { depositor: party(depositor), class, balance, members, rate: 0.01 }
    }

    fn book() -> Vec<Line> {
        vec![
            line(50, Class::Retail, 100_000.0, 10_000.0),
            line(51, Class::Corporate, 60_000.0, 40.0),
            line(52, Class::Wholesale, 80_000.0, 4.0),
        ]
    }

    #[test]
    fn a_model_with_one_deposit_type_cannot_have_a_run() {
        // A1.d: stickiness differs by class and it is the whole of liquidity risk. The same balance
        // in three classes leaves at three speeds, and with one class there is nothing for the fast
        // money to be faster than.
        let signals = 1.0;
        let retail = line(50, Class::Retail, 80_000.0, 10_000.0).leaves(signals, 50.0);
        let corporate = line(51, Class::Corporate, 80_000.0, 40.0).leaves(signals, 50.0);
        let wholesale = line(52, Class::Wholesale, 80_000.0, 4.0).leaves(signals, 50.0);
        assert!(wholesale > corporate);
        assert!(corporate > retail);
    }

    #[test]
    fn the_insured_amount_is_exact_because_the_cell_is_homogeneous() {
        // A1.a, XI-15: weight × min(member balance, limit). Ten thousand members holding 10 each are
        // covered in full; four wholesale depositors holding 20,000 each are covered for nothing.
        let small = line(50, Class::Retail, 100_000.0, 10_000.0);
        assert_eq!(small.insured(50.0), 100_000.0);
        let large = line(50, Class::Retail, 100_000.0, 100.0);
        assert_eq!(large.insured(50.0), 5_000.0);
        // E4: and insurance does not reach wholesale at all.
        assert_eq!(line(52, Class::Wholesale, 100_000.0, 4.0).insured(50.0), 0.0);
    }

    #[test]
    fn a_run_is_a_wholesale_phenomenon_first() {
        // E4.a: deposit insurance breaks the loop for retail and not for wholesale.
        let leaving: Vec<f64> = book().iter().map(|l| l.leaves(1.0, 50.0)).collect();
        // The fully insured retail line barely moves; the wholesale line goes entirely.
        assert_eq!(leaving[0], 0.0);
        assert_eq!(leaving[2], 80_000.0);
    }

    #[test]
    fn the_buffer_is_derived_from_its_own_liabilities_and_not_from_a_ratio() {
        // C2.a: a bank funded by wholesale money needs more than one funded by insured retail.
        let wholesale_funded = [line(52, Class::Wholesale, 200_000.0, 4.0)];
        let retail_funded = [line(50, Class::Retail, 200_000.0, 20_000.0)];
        assert!(buffer_wanted(&wholesale_funded, 50.0, 1.0) > buffer_wanted(&retail_funded, 50.0, 1.0));
    }

    #[test]
    fn the_blended_cost_is_the_banks_own_and_a_bank_funding_with_nothing_has_none() {
        // B2, B2.a: a bank with no cost of funds prices every loan as if it funded at the policy rate
        // whatever its own position, and then no funding condition can reach a borrower.
        let cheap = [
            Source { kind: Funding::Deposits(Class::Retail), amount: 8_000.0, rate: 0.005 },
            Source { kind: Funding::Wholesale, amount: 2_000.0, rate: 0.04 },
        ];
        let dear = [
            Source { kind: Funding::Deposits(Class::Retail), amount: 2_000.0, rate: 0.005 },
            Source { kind: Funding::Wholesale, amount: 8_000.0, rate: 0.04 },
        ];
        assert!(blended(&dear).unwrap() > blended(&cheap).unwrap());
        assert!(blended(&[]).is_none());
        // B3: and the margin can be negative.
        assert!(net_interest_margin(0.02, &dear).unwrap() < 0.0);
    }

    #[test]
    fn the_deposit_rate_is_set_against_the_cheaper_of_wholesale_and_the_money_fund() {
        // B1.a: past that point the bank would rather fund wholesale — a decision, not a rule.
        assert_eq!(will_pay_on_deposits(0.045, 0.030), 0.030);
        assert_eq!(will_pay_on_deposits(0.020, 0.030), 0.020);
    }

    #[test]
    fn a_liquid_asset_raises_what_the_market_can_take_at_what_it_converts_at() {
        // C1.a: they differ in how fast and how surely they convert.
        let deep = Liquid { value: 10_000.0, converts_at: 0.99, depth: 50_000.0 };
        let thin = Liquid { value: 10_000.0, converts_at: 0.80, depth: 2_000.0 };
        assert!(deep.raises() > thin.raises());
        assert_eq!(thin.raises(), 1_600.0);
    }

    #[test]
    fn a_short_bank_works_through_its_options_and_can_still_fail_to_fund() {
        // D1–D6, D6.a: failure to fund is REACHABLE, which is what makes the buffer worth holding.
        let liquid = [Liquid { value: 5_000.0, converts_at: 0.9, depth: 5_000.0 }];
        assert_eq!(when_short(1_000.0, 4_000.0, &liquid, 0.0, 0.0, 0.0), Short::BorrowsInTheMarket { amount: 1_000.0 });
        assert_eq!(when_short(5_000.0, 1_000.0, &liquid, 0.0, 0.0, 0.0), Short::SellsLiquid { raising: 4_000.0 });
        assert!(matches!(when_short(7_000.0, 1_000.0, &liquid, 2_000.0, 0.0, 0.0), Short::BidsForDeposits { .. }));
        assert!(matches!(when_short(9_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 0.0), Short::DrawsTheWindow { .. }));
        // D4.a: the credit crunch — it stops lending and lets the book run off.
        assert!(matches!(when_short(12_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0), Short::StopsLending { .. }));
        // D6: and past all of that it cannot fund itself.
        assert!(matches!(when_short(99_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0), Short::CannotFund { .. }));
    }

    #[test]
    fn what_depositors_observe_is_observable_and_the_loop_reinforces() {
        // E2.a, E3.a: the deposit leaves WITH THE RESERVES BEHIND IT, so the bank is shorter at the
        // next close — and more signals mean more leaves.
        let quiet = Observed { capital_ratio_published: 0.14, drew_the_window: false, paid_up_for_deposits: false, downgraded: false, periods_ending_short: 0 };
        let visible = Observed { capital_ratio_published: 0.06, drew_the_window: true, paid_up_for_deposits: true, downgraded: true, periods_ending_short: 2 };
        assert_eq!(quiet.signals(0.10), 0.0);
        assert_eq!(visible.signals(0.10), 6.0);
        let leaving = buffer_wanted(&book(), 50.0, visible.signals(0.10));
        assert!(leaving > buffer_wanted(&book(), 50.0, quiet.signals(0.10)));
        // And the reserves go with them.
        assert!(after_outflow(200_000.0, leaving) < 200_000.0);
    }

    #[test]
    fn maturity_transformation_is_the_business_and_a_bank_with_none_is_not_a_bank() {
        // C3, C3.a.
        assert!(transformation(7.0, 0.5) > 0.0);
        assert_eq!(transformation(0.5, 0.5), 0.0);
    }

    #[test]
    fn the_balance_sheet_balances_or_the_discrepancy_is_reported() {
        // F3: a VERIFY on derived dust, repairing nothing.
        assert!(balances(10_000.0, 9_000.0, 1_000.0, 3).is_none());
        assert_eq!(balances(10_000.0, 9_000.0, 800.0, 3), Some(200.0));
    }
}
