//! FIRM FUNDAMENTALS: a named party with a book, whose profit is a residual, whose equity is a read,
//! and which can run out of cash two different ways.
//!
//! @spec 32 A1 · 32 A2 · 32 A3 · 32 A4 · 32 B1 · 32 B1.a · 32 B1.b · 32 B2 · 32 B3 · 32 B4 ·
//! @spec 32 B4.a · 32 B4.b · 32 B5 · 32 B6 · 32 C1 · 32 C2 · 32 C3 · 32 C4 · 32 C4.a · 32 C4.b ·
//! @spec 32 D1 · 32 D2 · 32 D3 · 32 D4 · 32 D5 · 32 E4 · 32 E4.a · 32 E5 · 32 E7 · 32 F1 · 32 F2 ·
//! @spec 32 F3 · XI-4 · 46 C2 · Law 2, Law 3, Law 4, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::instruments::booked_equity;
use crate::journal::Value;
use crate::ledger::{Leg, Outcome, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::calendar::Day;
use crate::ids::{CurrencyCode, PartyId, RegionId};

/// A named party with an account, in a region — the region fixes its money — and with the dispersion
/// A3 calls the reason markets exist among firms.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Firm {
    pub who: PartyId,
    pub at: RegionId,
    /// Fixed by the region.
    pub money: CurrencyCode,
}

/// Revenue is quantity sold times price achieved, from named buyers — a consequence of a market,
/// never a growth rate applied to last period.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sale {
    pub to: PartyId,
    pub units: f64,
    pub at_price: f64,
}

pub fn revenue(sales: &[Sale]) -> f64 {
    sales.iter().map(|s| s.units * s.at_price).sum()
}

/// Every cost line is a named line with a real payee (F1: no cost without one).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CostLine {
    pub payee: PartyId,
    pub amount: f64,
    /// Does this line move with what the firm made?
    pub varies_with_output: bool,
}

pub fn costs(lines: &[CostLine]) -> f64 {
    lines.iter().map(|l| l.amount).sum()
}

/// What happens to the cost base when output changes.
pub fn costs_at(lines: &[CostLine], output_now: f64, output_then: f64) -> f64 {
    assert!(output_then > 0.0, "32 B5: a cost base with no output behind it cannot be rescaled");
    lines
        .iter()
        .map(|l| if l.varies_with_output { l.amount * output_now / output_then } else { l.amount })
        .sum()
}

/// Operating profit is the residual of revenue minus input costs minus labour, and it can be
/// negative.
pub fn operating_profit(revenue: f64, input_costs: f64, labour: f64) -> f64 {
    revenue - input_costs - labour
}

pub fn margin(profit: f64, revenue: f64) -> Option<f64> {
    if revenue <= 0.0 {
        return None;
    }
    Some(profit / revenue)
}

/// An invoice, which is where a receivable actually lives.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Invoice {
    pub counterparty: PartyId,
    pub amount: f64,
    pub due: Day,
}

pub fn receivables(book: &[Invoice]) -> f64 {
    book.iter().map(|i| i.amount).sum()
}

/// Equity is the read, assets minus liabilities, and it can be negative.
#[derive(Clone, Debug)]
pub struct Book {
    pub cash: f64,
    pub receivable_book: Vec<Invoice>,
    pub inventory: f64,
    pub fixed_capital: f64,
    pub payable_book: Vec<Invoice>,
    pub bank_debt: f64,
    pub bonds: f64,
}

impl Book {
    pub fn assets(&self) -> f64 {
        self.cash + receivables(&self.receivable_book) + self.inventory + self.fixed_capital
    }

    pub fn liabilities(&self) -> f64 {
        receivables(&self.payable_book) + self.bank_debt + self.bonds
    }

    /// The owners' claim is the residual, and it can be negative.
    pub fn equity(&self) -> f64 {
        self.assets() - self.liabilities()
    }

    /// Working capital is a real use of cash — inventory bought and not yet sold, invoices sent and
    /// not yet paid, less what the firm itself has not yet paid.
    pub fn working_capital(&self) -> f64 {
        receivables(&self.receivable_book) + self.inventory - receivables(&self.payable_book)
    }
}

/// Profit and cash are different numbers, and the difference is where firms die.
pub fn cash_from_operations(profit: f64, working_capital_now: f64, working_capital_before: f64) -> f64 {
    profit - (working_capital_now - working_capital_before)
}

/// Debt service is a fixed claim ahead of the owners: interest AND principal.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Service {
    pub interest: f64,
    pub principal: f64,
}

impl Service {
    pub fn total(&self) -> f64 {
        self.interest + self.principal
    }
}

/// Coverage is a read of operating cash against debt service, and it is what lenders look at.
pub fn coverage(operating_cash: f64, s: &Service) -> Option<f64> {
    if s.total() <= 0.0 {
        return None;
    }
    Some(operating_cash / s.total())
}

/// It can fail two ways, and a firm can be either without the other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Failing {
    /// No cash to pay something due.
    OutOfCash,
    /// Liabilities exceeding assets.
    Insolvent,
    Both,
    Neither,
}

pub fn failing(b: &Book, due_now: f64) -> Failing {
    let no_cash = b.cash < due_now;
    let insolvent = b.equity() < 0.0;
    match (no_cash, insolvent) {
        (true, true) => Failing::Both,
        (true, false) => Failing::OutOfCash,
        (false, true) => Failing::Insolvent,
        (false, false) => Failing::Neither,
    }
}

/// The leverage target is the management's own — the lender's covenant line moderated by the
/// management's risk aversion, approached at its own horizon.
#[derive(Clone, Copy, Debug)]
pub struct LeverageTarget {
    /// What the lender's covenant allows.
    pub covenant: f64,
    /// The management's own caution below it.
    pub caution: f64,
}

impl LeverageTarget {
    pub fn at(&self) -> f64 {
        self.covenant - self.caution
    }
}

/// How to fund itself — retained cash, debt, or new equity — and the money raised is raised into an
/// actual investment programme.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Funds {
    Nothing,
    FromCash(f64),
    Borrow(f64),
    Issue(f64),
}

/// The choice depends on what each costs — and on where the firm's leverage stands against the
/// management's own target.
pub fn funds(programme: f64, cash_spare: f64, leverage_now: f64, target: &LeverageTarget, debt_costs: f64, equity_costs: f64) -> Funds {
    if programme <= 0.0 {
        // A firm with no programme raises nothing, whatever the markets are offering.
        return Funds::Nothing;
    }
    if cash_spare >= programme {
        return Funds::FromCash(programme);
    }
    if leverage_now >= target.at() {
        // Above its own target: it does not borrow more, whatever debt costs.
        return Funds::Issue(programme - cash_spare);
    }
    if debt_costs < equity_costs {
        Funds::Borrow(programme - cash_spare)
    } else {
        Funds::Issue(programme - cash_spare)
    }
}

/// Real cash to owners.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Distribution {
    pub to_holders_of: PartyId,
    pub cash: f64,
}

/// The management publishes an expectation — its own adaptive read of its own earnings — and is then
/// judged against it.
pub fn expects(last_seen: f64, held_before: f64, memory: f64) -> f64 {
    assert!(memory > 0.0 && memory < 1.0, "46 C2: a memory of {memory} is not a weighting");
    held_before * memory + last_seen * (1.0 - memory)
}

/// Every cost is somebody's income and every revenue is somebody's outlay, party by party.
pub fn two_sided(costs_booked: f64, received_by_payees: f64, terms: usize) -> bool {
    (costs_booked - received_by_payees).abs()
        <= crate::num::dust(terms, &[costs_booked, received_by_payees])
}

/// The operating result read from settled cash legs. Financing principal and transfers are not
/// sales, and principal repayment is not an input cost, so neither can inflate or depress profit.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct OperatingFlows {
    pub revenue: f64,
    pub costs: f64,
}

impl OperatingFlows {
    pub fn cash(self) -> f64 {
        operating_profit(self.revenue, self.costs, 0.0)
    }

    pub fn reads(&mut self, who: PartyId, leg: Leg) {
        if let Leg::Money { from, to, amount, receipt, .. } = leg {
            if to == who && receipt == Receipt::Sale {
                self.revenue += amount.get();
            }
            if from == who && matches!(receipt, Receipt::Sale | Receipt::Wage) {
                self.costs += amount.get();
            }
        }
    }
}

// §5 RUNS HERE.

/// A FIRM'S RESULT IS PUBLISHED, and it is a read of what actually happened to it.
pub struct Reporting {
    /// The event kind this publishes under, declared by the assembly.
    pub kind: u32,
    pub at_revenue: u32,
    pub at_costs: u32,
    pub at_cash: u32,
    pub at_equity: u32,
    pub at_opening_equity: u32,
}

impl Mechanism for Reporting {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut said: Vec<(u32, f64, OperatingFlows)> = Vec::new();
        for f in ctx.parties().of_kind(kinds::FIRM) {
            let who = PartyId(*f);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(worth) = booked_equity(who, ctx.register(), ctx.instruments(), ctx.prints(), ctx.claims(), ctx.period()) else { continue };
            let mut flows = OperatingFlows::default();
            for instruction in ctx.wire().in_period(ctx.period()) {
                if ctx.wire().outcome_of(instruction) != Outcome::Settled {
                    continue;
                }
                for leg in ctx.wire().legs_of(instruction) {
                    flows.reads(who, *leg);
                }
            }
            said.push((*f, worth, flows));
        }
        for (who, worth, flows) in said {
            let opening_equity = match ctx.parties().opening_equity_of(PartyId(who)) {
                Some(equity) => equity,
                None => worth,
            };
            // A firm's result is private now, but remains a typed observation consumed by its own
            // outlook next period. Public accounts remain the creditors' and owners' legal read.
            ctx.say(
                self.kind,
                &[who],
                &[
                    (self.at_revenue, Value::Num(flows.revenue)),
                    (self.at_costs, Value::Num(flows.costs)),
                    (self.at_cash, Value::Num(flows.cash())),
                    (self.at_equity, Value::Num(worth)),
                    (self.at_opening_equity, Value::Num(opening_equity)),
                ],
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn the_operating_result_reads_settled_sales_and_costs_not_financing() {
        let firm = party(4);
        let customer = party(5);
        let lender = party(6);
        let money = crate::ids::InstrumentId::at(1);
        let mut flows = OperatingFlows::default();
        flows.reads(
            firm,
            Leg::Money {
                from: customer,
                to: firm,
                instrument: money,
                amount: crate::ledger::Units::new(120.0).expect("a sale has proceeds"),
                receipt: Receipt::Sale,
            },
        );
        flows.reads(
            firm,
            Leg::Money {
                from: firm,
                to: customer,
                instrument: money,
                amount: crate::ledger::Units::new(70.0).expect("an input has a cost"),
                receipt: Receipt::Sale,
            },
        );
        flows.reads(
            firm,
            Leg::Money {
                from: lender,
                to: firm,
                instrument: money,
                amount: crate::ledger::Units::new(500.0).expect("the loan has principal"),
                receipt: Receipt::Principal,
            },
        );
        assert_eq!(flows, OperatingFlows { revenue: 120.0, costs: 70.0 });
        assert_eq!(flows.cash(), 50.0);
    }

    fn invoice(counterparty: u32, amount: f64) -> Invoice {
        Invoice { counterparty: party(counterparty), amount, due: Day(30) }
    }

    fn book() -> Book {
        Book {
            cash: 300.0,
            receivable_book: vec![invoice(20, 400.0), invoice(21, 200.0)],
            inventory: 500.0,
            fixed_capital: 2_000.0,
            payable_book: vec![invoice(30, 250.0)],
            bank_debt: 900.0,
            bonds: 1_200.0,
        }
    }

    fn lines() -> Vec<CostLine> {
        vec![
            CostLine { payee: party(30), amount: 600.0, varies_with_output: true },
            CostLine { payee: party(31), amount: 400.0, varies_with_output: false },
        ]
    }

    #[test]
    fn revenue_comes_from_named_buyers_and_never_from_a_growth_rate() {
        // No revenue without a buyer.
        let sales = [
            Sale { to: party(20), units: 100.0, at_price: 9.0 },
            Sale { to: party(21), units: 50.0, at_price: 11.0 },
        ];
        assert_eq!(revenue(&sales), 1_450.0);
    }

    #[test]
    fn the_margin_is_a_read_and_a_firm_that_sold_nothing_has_none() {
        // Never a target the revenue was fitted to.
        let p = operating_profit(1_450.0, 600.0, 400.0);
        assert_eq!(p, 450.0);
        let m = margin(p, 1_450.0).unwrap();
        assert!(m > 0.3 && m < 0.32);
        assert!(margin(-50.0, 0.0).is_none());
    }

    #[test]
    fn operating_profit_can_be_negative() {
        // It is a residual, not a quantity with a sign anybody chose.
        assert_eq!(operating_profit(800.0, 600.0, 400.0), -200.0);
    }

    #[test]
    fn margin_moves_more_than_revenue_because_some_costs_do_not_move() {
        // Operating leverage is a CONSEQUENCE of the cost structure, not a coefficient.
        let full_revenue = 1_450.0;
        let half_revenue = full_revenue / 2.0;
        let full = margin(full_revenue - costs(&lines()), full_revenue).unwrap();
        let half = margin(half_revenue - costs_at(&lines(), 0.5, 1.0), half_revenue).unwrap();
        assert!(half < full);
        // The revenue halved; the margin fell by more than half of itself.
        assert!(full - half > full * 0.5);
    }

    #[test]
    fn receivables_are_the_sum_of_the_invoice_book_and_not_a_ratio_of_revenue() {
        // Two representations of one thing, with the decision reading the stated one, is Law 4's
        // defect at the point it matters most.
        assert_eq!(receivables(&book().receivable_book), 600.0);
    }

    #[test]
    fn equity_is_the_read_and_it_can_be_negative() {
        // Assets 3,400 against liabilities 2,350.
        let b = book();
        assert_eq!(b.equity(), 1_050.0);
        let sunk = Book { fixed_capital: 200.0, ..b };
        assert!(sunk.equity() < 0.0);
    }

    #[test]
    fn a_profitable_firm_with_a_growing_invoice_book_runs_out_of_cash() {
        // Profit and cash are different numbers, and the difference is where firms die.
        let b = book();
        let before = 300.0;
        let cash = cash_from_operations(450.0, b.working_capital(), before);
        assert_eq!(b.working_capital(), 850.0);
        assert!(cash < 0.0);
    }

    #[test]
    fn a_firm_can_be_out_of_cash_without_being_insolvent_and_the_other_way_round() {
        // They are different failures, and a firm can be either without the other.
        let solvent_but_dry = book();
        assert_eq!(failing(&solvent_but_dry, 900.0), Failing::OutOfCash);
        assert_eq!(failing(&solvent_but_dry, 100.0), Failing::Neither);
        // Plenty of money in the account and a bond stack far beyond what the book is worth.
        let insolvent_with_cash = Book { cash: 5_000.0, fixed_capital: 0.0, bonds: 9_000.0, ..book() };
        assert_eq!(failing(&insolvent_with_cash, 100.0), Failing::Insolvent);
    }

    #[test]
    fn coverage_is_a_read_and_a_firm_with_no_debt_has_none_rather_than_an_infinite_one() {
        // What lenders look at.
        let s = Service { interest: 60.0, principal: 140.0 };
        assert_eq!(coverage(400.0, &s), Some(2.0));
        assert!(coverage(400.0, &Service { interest: 0.0, principal: 0.0 }).is_none());
    }

    #[test]
    fn a_firm_with_no_programme_raises_nothing_whatever_the_markets_are_offering() {
        // The money raised is raised INTO an actual investment programme.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(0.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Nothing);
    }

    #[test]
    fn a_management_above_its_own_target_does_not_borrow_whatever_debt_costs() {
        // The target is the covenant line moderated by the management's own risk aversion, and it is
        // theirs.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(500.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Borrow(500.0));
        assert_eq!(funds(500.0, 0.0, 3.5, &t, 0.03, 0.10), Funds::Issue(500.0));
        // And it spends its own cash before raising anything at all.
        assert_eq!(funds(500.0, 900.0, 1.0, &t, 0.03, 0.10), Funds::FromCash(500.0));
    }

    #[test]
    fn the_funding_choice_depends_on_what_each_costs() {
        // And this is XI-4's joint — a financial price changes, the firm's choice changes.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(500.0, 0.0, 1.0, &t, 0.12, 0.10), Funds::Issue(500.0));
    }

    #[test]
    fn the_management_forms_its_expectation_adaptively_from_its_own_earnings() {
        // Its own adaptive read, never a choice among written phrases — and the figure it publishes
        // is the one its own decisions read.
        let held = expects(600.0, 400.0, 0.7);
        assert!(held > 400.0 && held < 600.0);
        // A surprise moves it, and a repeat of the same number moves it less each time.
        let again = expects(600.0, held, 0.7);
        assert!(again > held);
        assert!(again - held < held - 400.0);
    }

    #[test]
    fn every_cost_is_somebodys_income() {
        // Party by party, on derived dust — a VERIFY that answers false rather than balancing.
        assert!(two_sided(1_000.0, 1_000.0, 2));
        assert!(!two_sided(1_000.0, 940.0, 2));
    }

    #[test]
    #[should_panic(expected = "cannot be rescaled")]
    fn a_cost_base_with_no_output_behind_it_cannot_be_rescaled() {
        costs_at(&lines(), 0.5, 0.0);
    }
}
