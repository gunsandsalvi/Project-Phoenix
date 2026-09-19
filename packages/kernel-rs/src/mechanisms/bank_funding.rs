//! BANKS — FUNDING AND LIQUIDITY: deposits are not one thing, the cost of funds is the bank's own,
//! and a model with one deposit type cannot have a run.
//!
//! @spec 24 A1 · 24 A1.a · 24 A1.b · 24 A1.c · 24 A1.d · 24 A2 · 24 A2.a · 24 A3 · 24 A4 · 24 A5 ·
//! @spec 24 B1 · 24 B1.a · 24 B1.b · 24 B2 · 24 B2.a · 24 B2.b · 24 B3 · 24 C1 · 24 C1.a · 24 C2 ·
//! @spec 24 C2.a · 24 C3 · 24 C3.a · 24 C4 · 24 D1 · 24 D2 · 24 D3 · 24 D4 · 24 D4.a · 24 D5 · 24 D6 ·
//! @spec 24 D6.a · 24 E1 · 24 E2 · 24 E2.a · 24 E3 · 24 E3.a · 24 E4 · 24 E4.a · 24 E5 · 24 F1 ·
//! @spec 24 F2 · 24 F3 · XI-15 · XI-2 · Law 4, Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::standing;

/// Deposits are not one thing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    /// Many, small, sticky, and insured up to a limit.
    Retail,
    /// Fewer, larger, operational — a firm banks where it transacts.
    Corporate,
    /// Few, very large, and RATE-SENSITIVE.
    Wholesale,
}

/// A deposit line by class, as a read of who actually banks there.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Line {
    pub depositor: PartyId,
    pub class: Class,
    pub balance: f64,
    /// How many real depositors this line is.
    pub members: f64,
    /// The rate the BANK sets on it, which depositors respond to.
    pub rate: f64,
}

impl Line {
    /// The insured amount is `weight × min(member balance, limit)`, which is EXACT because the cell
    /// is homogeneous — and that is what makes the break in the run loop real rather than notional.
    pub fn insured(&self, limit_per_member: f64) -> f64 {
        if self.class != Class::Retail || self.members <= 0.0 {
            return 0.0;
        }
        let per_member = self.balance / self.members;
        let covered = if per_member < limit_per_member { per_member } else { limit_per_member };
        self.members * covered
    }

    /// How much of this line leaves when the depositors see something.
    pub fn leaves(&self, on_signals: f64, limit_per_member: f64, runs: Eagerness) -> f64 {
        let exposed = self.balance - self.insured(limit_per_member);
        let going = exposed * runs.of(self.class) * on_signals;
        // They cannot take more than they have.
        if going < self.balance {
            going
        } else {
            self.balance
        }
    }
}

/// HOW FAST EACH KIND OF DEPOSITOR RUNS. Wholesale money is watching and goes first, corporate
/// money is slower, and insured retail is slowest of all — three PREFERENCES of the depositor, and
/// whoever wires this supplies them rather than the branch that reads them carrying them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Eagerness {
    pub wholesale: f64,
    pub corporate: f64,
    pub retail: f64,
}

impl Eagerness {
    fn of(self, class: Class) -> f64 {
        match class {
            Class::Wholesale => self.wholesale,
            Class::Corporate => self.corporate,
            Class::Retail => self.retail,
        }
    }
}

/// Each source has a price, the prices differ, and the mix is a DECISION.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Funding {
    Deposits(Class),
    /// Short, and it ROLLS — which is where a funding squeeze bites.
    Wholesale,
    /// Equity and subordinated debt, which do not run.
    Capital,
    /// The central bank, on the corridor's terms.
    CentralBank,
}

/// The bank pays a rate on each source, and it is a real payment to a real holder.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Source {
    pub kind: Funding,
    pub amount: f64,
    pub rate: f64,
}

/// The blended cost of funds is a READ across the mix — the bank's own, which is what lets a funding
/// condition reach a borrower at all.
pub fn blended(mix: &[Source]) -> Option<f64> {
    let size: f64 = mix.iter().map(|s| s.amount).sum();
    if size <= 0.0 {
        return None;
    }
    Some(mix.iter().map(|s| s.rate * s.amount).sum::<f64>() / size)
}

/// A deposit rate the bank SETS — bounded above by the cheaper of its own wholesale cost and the
/// money fund's yield, on the contested share of its base.
pub fn will_pay_on_deposits(own_wholesale_cost: f64, money_fund_yield: f64) -> f64 {
    if own_wholesale_cost < money_fund_yield {
        own_wholesale_cost
    } else {
        money_fund_yield
    }
}

/// Net interest margin is what it earns minus its cost of funds, and it can be NEGATIVE.
pub fn net_interest_margin(earned: f64, mix: &[Source]) -> Option<f64> {
    Some(earned - blended(mix)?)
}

/// Liquid assets — reserves, and securities it can sell or pledge — differ in how fast and how
/// surely they convert: a haircut and a market depth.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Liquid {
    pub value: f64,
    /// What it fetches when sold in a hurry, as a fraction of its mark.
    pub converts_at: f64,
    /// And how much of it the market can take this period.
    pub depth: f64,
}

impl Liquid {
    /// What this asset would actually raise, now.
    pub fn raises(&self) -> f64 {
        let sellable = if self.value < self.depth { self.value } else { self.depth };
        sellable * self.converts_at
    }
}

/// A buffer preference derived from its OWN liabilities, not a stated ratio — a bank funded by
/// wholesale money needs more than one funded by insured retail, and that is the whole of A1.d
/// showing up as a number.
pub fn buffer_wanted(lines: &[Line], limit_per_member: f64, on_signals: f64, runs: Eagerness) -> f64 {
    lines.iter().map(|l| l.leaves(on_signals, limit_per_member, runs)).sum()
}

/// Maturity transformation is the business — it funds long assets with short liabilities, and that
/// gap is why it earns anything.
pub fn transformation(asset_years: f64, liability_years: f64) -> f64 {
    asset_years - liability_years
}

/// What a bank that is short actually does, in order, each a real act with a counterparty.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Short {
    BorrowsInTheMarket { amount: f64 },
    /// Sells or pledges liquid assets — a real order in a real book.
    SellsLiquid { raising: f64 },
    /// Bids up for deposits, and pays for them.
    BidsForDeposits { paying: f64 },
    /// It stops lending and lets the book run off — this is the credit crunch, a funding problem
    /// transmitted into the credit decision.
    StopsLending { by: f64 },
    /// The facility, collateralised and at a penalty.
    DrawsTheWindow { amount: f64 },
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

/// They leave because they observe something, and what they observe must be OBSERVABLE — a capital
/// ratio, a facility draw, a rate paid up, a rating action, a run of periods ending short.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    pub capital_ratio_published: f64,
    pub drew_the_window: bool,
    pub paid_up_for_deposits: bool,
    pub downgraded: bool,
    pub periods_ending_short: u32,
}

impl Observed {
    /// How many things a depositor can actually see.
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

/// The deposit leaves WITH THE RESERVES BEHIND IT, so the bank is shorter at the next close — and
/// that is the loop.
pub fn after_outflow(reserves: f64, left: f64) -> f64 {
    reserves - left
}

/// Assets equal liabilities plus equity, in the bank's own money, every period.
pub fn balances(assets: f64, liabilities: f64, equity: f64, terms: usize) -> Option<f64> {
    let off = assets - (liabilities + equity);
    if off.abs() <= crate::num::dust(terms, &[assets, liabilities, equity]) {
        return None;
    }
    Some(off)
}


/// A BANK SETS THE RATE IT PAYS ON DEPOSITS.
pub struct BankFunding {
    pub kind: u32,
    /// The benchmark fixing, which is what a money fund would earn.
    pub fixing: u32,
    pub days_per_period: i64,
}

impl Mechanism for BankFunding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The last fixing.
        let mut money_fund_yield: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.fixing) {
            if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                money_fund_yield = Some(rate);
            }
        }
        let Some(money_fund_yield) = money_fund_yield else { return };

        let mut set: Vec<(PartyId, f64)> = Vec::new();
        for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Its OWN mix, read off what it has issued and what it pays on each.
            let mut mix: Vec<Source> = Vec::new();
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let (held, _) = ctx.register().held_total(what);
                let outstanding = held - ctx.register().quantity(ctx.register().row(who, what));
                if outstanding <= 0.0 {
                    continue;
                }
                match ctx.instruments().class_of(what) {
                    crate::instruments::Class::Money => mix.push(Source {
                        kind: Funding::Deposits(Class::Retail),
                        amount: outstanding,
                        // What it is paying now is what it last stood behind, and nothing where it
                        // has never set one — a bank that has not set a rate is not paying zero.
                        rate: match ctx.standing().of_party_about(who, PartyId::NONE, standing::DEPOSIT_RATE) {
                            Some(s) => ctx.standing().terms(s)[0],
                            None => continue,
                        },
                    }),
                    // Short, and it ROLLS — which is where a funding squeeze bites.
                    crate::instruments::Class::Claim => mix.push(Source {
                        kind: Funding::Wholesale,
                        amount: outstanding,
                        rate: match ctx.instruments().coupon_of(what) {
                            Some(c) => c,
                            None => continue,
                        },
                    }),
                    _ => {}
                }
            }
            // `None` where it funds with nothing — answering zero would say it funds free.
            let own_wholesale_cost = match blended(&mix) {
                Some(cost) => cost,
                // A bank that has never funded wholesale has its own cost to find, and the benchmark
                // is the only thing it can read.
                None => money_fund_yield,
            };
            set.push((who, will_pay_on_deposits(own_wholesale_cost, money_fund_yield)));
        }

        for (who, rate) in set {
            // A POSTED rate — depositors respond to it, so it is one-sided terms the bank stands
            // behind until it changes them, and what it was paying stays readable beside it.
            ctx.now_stands(standing::DEPOSIT_RATE, who, PartyId::NONE, vec![rate]);
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(rate))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tests' own depositors, and this is the one place these three are written.
    fn runs() -> Eagerness {
        Eagerness { wholesale: 1.0, corporate: 0.4, retail: 0.15 }
    }

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
        // Stickiness differs by class and it is the whole of liquidity risk.
        let signals = 1.0;
        let retail = line(50, Class::Retail, 80_000.0, 10_000.0).leaves(signals, 50.0, runs());
        let corporate = line(51, Class::Corporate, 80_000.0, 40.0).leaves(signals, 50.0, runs());
        let wholesale = line(52, Class::Wholesale, 80_000.0, 4.0).leaves(signals, 50.0, runs());
        assert!(wholesale > corporate);
        assert!(corporate > retail);
    }

    #[test]
    fn the_insured_amount_is_exact_because_the_cell_is_homogeneous() {
        // Weight × min(member balance, limit).
        let small = line(50, Class::Retail, 100_000.0, 10_000.0);
        assert_eq!(small.insured(50.0), 100_000.0);
        let large = line(50, Class::Retail, 100_000.0, 100.0);
        assert_eq!(large.insured(50.0), 5_000.0);
        // And insurance does not reach wholesale at all.
        assert_eq!(line(52, Class::Wholesale, 100_000.0, 4.0).insured(50.0), 0.0);
    }

    #[test]
    fn a_run_is_a_wholesale_phenomenon_first() {
        // Deposit insurance breaks the loop for retail and not for wholesale.
        let leaving: Vec<f64> = book().iter().map(|l| l.leaves(1.0, 50.0, runs())).collect();
        // The fully insured retail line barely moves; the wholesale line goes entirely.
        assert_eq!(leaving[0], 0.0);
        assert_eq!(leaving[2], 80_000.0);
    }

    #[test]
    fn the_buffer_is_derived_from_its_own_liabilities_and_not_from_a_ratio() {
        // A bank funded by wholesale money needs more than one funded by insured retail.
        let wholesale_funded = [line(52, Class::Wholesale, 200_000.0, 4.0)];
        let retail_funded = [line(50, Class::Retail, 200_000.0, 20_000.0)];
        assert!(buffer_wanted(&wholesale_funded, 50.0, 1.0, runs()) > buffer_wanted(&retail_funded, 50.0, 1.0, runs()));
    }

    #[test]
    fn the_blended_cost_is_the_banks_own_and_a_bank_funding_with_nothing_has_none() {
        // A bank with no cost of funds prices every loan as if it funded at the policy rate whatever
        // its own position, and then no funding condition can reach a borrower.
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
        // And the margin can be negative.
        assert!(net_interest_margin(0.02, &dear).unwrap() < 0.0);
    }

    #[test]
    fn the_deposit_rate_is_set_against_the_cheaper_of_wholesale_and_the_money_fund() {
        // Past that point the bank would rather fund wholesale — a decision, not a rule.
        assert_eq!(will_pay_on_deposits(0.045, 0.030), 0.030);
        assert_eq!(will_pay_on_deposits(0.020, 0.030), 0.020);
    }

    #[test]
    fn a_liquid_asset_raises_what_the_market_can_take_at_what_it_converts_at() {
        // They differ in how fast and how surely they convert.
        let deep = Liquid { value: 10_000.0, converts_at: 0.99, depth: 50_000.0 };
        let thin = Liquid { value: 10_000.0, converts_at: 0.80, depth: 2_000.0 };
        assert!(deep.raises() > thin.raises());
        assert_eq!(thin.raises(), 1_600.0);
    }

    #[test]
    fn a_short_bank_works_through_its_options_and_can_still_fail_to_fund() {
        // Failure to fund is REACHABLE, which is what makes the buffer worth holding.
        let liquid = [Liquid { value: 5_000.0, converts_at: 0.9, depth: 5_000.0 }];
        assert_eq!(when_short(1_000.0, 4_000.0, &liquid, 0.0, 0.0, 0.0), Short::BorrowsInTheMarket { amount: 1_000.0 });
        assert_eq!(when_short(5_000.0, 1_000.0, &liquid, 0.0, 0.0, 0.0), Short::SellsLiquid { raising: 4_000.0 });
        assert!(matches!(when_short(7_000.0, 1_000.0, &liquid, 2_000.0, 0.0, 0.0), Short::BidsForDeposits { .. }));
        assert!(matches!(when_short(9_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 0.0), Short::DrawsTheWindow { .. }));
        // The credit crunch — it stops lending and lets the book run off.
        assert!(matches!(when_short(12_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0), Short::StopsLending { .. }));
        // And past all of that it cannot fund itself.
        assert!(matches!(when_short(99_000.0, 1_000.0, &liquid, 2_000.0, 3_000.0, 5_000.0), Short::CannotFund { .. }));
    }

    #[test]
    fn what_depositors_observe_is_observable_and_the_loop_reinforces() {
        // The deposit leaves WITH THE RESERVES BEHIND IT, so the bank is shorter at the next close —
        // and more signals mean more leaves.
        let quiet = Observed { capital_ratio_published: 0.14, drew_the_window: false, paid_up_for_deposits: false, downgraded: false, periods_ending_short: 0 };
        let visible = Observed { capital_ratio_published: 0.06, drew_the_window: true, paid_up_for_deposits: true, downgraded: true, periods_ending_short: 2 };
        assert_eq!(quiet.signals(0.10), 0.0);
        assert_eq!(visible.signals(0.10), 6.0);
        let leaving = buffer_wanted(&book(), 50.0, visible.signals(0.10), runs());
        assert!(leaving > buffer_wanted(&book(), 50.0, quiet.signals(0.10), runs()));
        // And the reserves go with them.
        assert!(after_outflow(200_000.0, leaving) < 200_000.0);
    }

    #[test]
    fn maturity_transformation_is_the_business_and_a_bank_with_none_is_not_a_bank() {
        assert!(transformation(7.0, 0.5) > 0.0);
        assert_eq!(transformation(0.5, 0.5), 0.0);
    }

    #[test]
    fn the_balance_sheet_balances_or_the_discrepancy_is_reported() {
        // A VERIFY on derived dust, repairing nothing.
        assert!(balances(10_000.0, 9_000.0, 1_000.0, 3).is_none());
        assert_eq!(balances(10_000.0, 9_000.0, 800.0, 3), Some(200.0));
    }
}
