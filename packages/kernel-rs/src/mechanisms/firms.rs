//! FIRM FUNDAMENTALS: a named party with a book, whose profit is a residual, whose equity is a read,
//! and which can run out of cash two different ways.
//!
//! @spec 32 A1 · 32 A2 · 32 A3 · 32 A4 · 32 B1 · 32 B1.a · 32 B1.b · 32 B2 · 32 B3 · 32 B4 ·
//! @spec 32 B4.a · 32 B4.b · 32 B5 · 32 B6 · 32 C1 · 32 C2 · 32 C3 · 32 C4 · 32 C4.a · 32 C4.b ·
//! @spec 32 D1 · 32 D2 · 32 D3 · 32 D4 · 32 D5 · 32 E4 · 32 E4.a · 32 E5 · 32 E7 · 32 F1 · 32 F2 ·
//! @spec 32 F3 · XI-4 · 46 C2 · Law 2, Law 3, Law 4, Law 6, Law 19 · Appendix B
//!
//! **The margin is a read, never a target the revenue was fitted to** (B4.a). A cost line struck at
//! the seed as the gap to a chosen margin, and then applied as a fixed share of revenue for ever,
//! makes that margin an ATTRACTOR and makes the cost base perfectly variable by construction. So
//! `Costs` is a list of named lines with real payees (B4.b) and `margin` divides two reads — there is
//! no share-of-revenue anywhere, and nothing in this module can produce a cost from a revenue.
//!
//! **Fixed and variable costs differ, which is why margin moves more than revenue** (B5). Operating
//! leverage is a CONSEQUENCE of the cost structure and not a coefficient: the test halves output and
//! watches the margin fall further than the revenue did, with nothing multiplying anything.
//!
//! **Profit and cash are different numbers, and the difference is where firms die** (C4.a). Working
//! capital is a real use of cash — inventory bought and not yet sold, invoices sent and not yet paid
//! — and `cash_from_operations` subtracts the change in it from profit. A profitable firm with a
//! growing invoice book runs out of money, which is the case the whole section exists for.
//!
//! **Receivables are the sum of the actual invoice book, not a ratio of revenue** (C4.b). Two
//! representations of one thing, with the decision reading the stated one, is Law 4's defect at the
//! point it matters most: `receivables` walks the invoices.
//!
//! **It can fail two ways** (D4): no cash to pay something due, or liabilities exceeding assets. They
//! are different and a firm can be either without the other — `Failing` keeps them apart, and F3's
//! firm that cannot run out of cash is not writable here because the balance is a real quantity.

use crate::assembly::kinds;
use crate::instruments::equity;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::calendar::Day;
use crate::ids::{CurrencyCode, PartyId, RegionId};

/// A1, A2: a named party with an account, in a region — **the region fixes its money** — and with the
/// dispersion A3 calls the reason markets exist among firms.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Firm {
    pub who: PartyId,
    pub at: RegionId,
    /// A2: fixed by the region. A firm does not choose what money it keeps its books in.
    pub money: CurrencyCode,
}

/// B1: **revenue is quantity sold times price achieved, from named buyers** — a consequence of a
/// market, never a growth rate applied to last period (B1.a). Each row is a sale that happened.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sale {
    pub to: PartyId,
    pub units: f64,
    pub at_price: f64,
}

pub fn revenue(sales: &[Sale]) -> f64 {
    sales.iter().map(|s| s.units * s.at_price).sum()
}

/// B4.b: **every cost line is a named line with a real payee** (F1: no cost without one). B5: and it
/// is either fixed or variable, which is a fact about the line, not a split anybody assumed.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CostLine {
    pub payee: PartyId,
    pub amount: f64,
    /// B5: does this line move with what the firm made? Fixed lines are what make operating leverage
    /// a consequence rather than a coefficient.
    pub varies_with_output: bool,
}

pub fn costs(lines: &[CostLine]) -> f64 {
    lines.iter().map(|l| l.amount).sum()
}

/// B5: what happens to the cost base when output changes. The variable lines scale and the fixed ones
/// do not — which is the whole of operating leverage, computed rather than parameterised.
pub fn costs_at(lines: &[CostLine], output_now: f64, output_then: f64) -> f64 {
    assert!(output_then > 0.0, "32 B5: a cost base with no output behind it cannot be rescaled");
    lines
        .iter()
        .map(|l| if l.varies_with_output { l.amount * output_now / output_then } else { l.amount })
        .sum()
}

/// B4: **operating profit is the residual** of revenue minus input costs minus labour, **and it can be
/// negative.** F2: there is no path here that produces it from a series.
pub fn operating_profit(revenue: f64, input_costs: f64, labour: f64) -> f64 {
    revenue - input_costs - labour
}

/// B4.a: **the margin is a READ.** `None` on no revenue: a margin over nothing is not zero, and a firm
/// that sold nothing has no margin to report.
pub fn margin(profit: f64, revenue: f64) -> Option<f64> {
    if revenue <= 0.0 {
        return None;
    }
    Some(profit / revenue)
}

/// C4.b: **an invoice**, which is where a receivable actually lives. The book is the sum of these and
/// never a ratio of revenue.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Invoice {
    pub counterparty: PartyId,
    pub amount: f64,
    pub due: Day,
}

pub fn receivables(book: &[Invoice]) -> f64 {
    book.iter().map(|i| i.amount).sum()
}

/// C1, C2, C3: **equity is the read**, assets minus liabilities, **and it can be negative.**
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

    /// C3, A4: the owners' claim is the residual, and it can be negative.
    pub fn equity(&self) -> f64 {
        self.assets() - self.liabilities()
    }

    /// C4: **working capital is a real use of cash** — inventory bought and not yet sold, invoices
    /// sent and not yet paid, less what the firm itself has not yet paid.
    pub fn working_capital(&self) -> f64 {
        receivables(&self.receivable_book) + self.inventory - receivables(&self.payable_book)
    }
}

/// C4.a: **profit and cash are different numbers, and the difference is where firms die.** Growth in
/// working capital consumes cash the profit statement never mentions.
pub fn cash_from_operations(profit: f64, working_capital_now: f64, working_capital_before: f64) -> f64 {
    profit - (working_capital_now - working_capital_before)
}

/// D2: **debt service is a fixed claim ahead of the owners**: interest AND principal.
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

/// D3: **coverage is a read** of operating cash against debt service, and it is what lenders look at.
/// `None` where nothing is due — a firm with no debt has no coverage ratio, not an infinite one.
pub fn coverage(operating_cash: f64, s: &Service) -> Option<f64> {
    if s.total() <= 0.0 {
        return None;
    }
    Some(operating_cash / s.total())
}

/// D4: **it can fail two ways, and a firm can be either without the other.**
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Failing {
    /// D1, D5: no cash to pay something due. The balance hit zero and something was owed.
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

/// E4, E4.a: **the leverage target is the management's own** — the lender's covenant line moderated by
/// the management's risk aversion, approached at its own horizon. A management above its target pays
/// down toward it whatever debt costs.
#[derive(Clone, Copy, Debug)]
pub struct LeverageTarget {
    /// What the lender's covenant allows. Not the firm's choice.
    pub covenant: f64,
    /// The management's own caution below it. Theirs, and a PREFERENCE (Law 2).
    pub caution: f64,
}

impl LeverageTarget {
    pub fn at(&self) -> f64 {
        self.covenant - self.caution
    }
}

/// E4, E4.a: how to fund itself — retained cash, debt, or new equity — **and the money raised is
/// raised into an actual investment programme. A firm with no programme raises nothing.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Funds {
    Nothing,
    FromCash(f64),
    Borrow(f64),
    Issue(f64),
}

/// The choice depends on what each costs — and on where the firm's leverage stands against the
/// management's own target. Law 6: a firm that needs nothing raises nothing, and that is `Nothing`,
/// not a zero raise.
pub fn funds(programme: f64, cash_spare: f64, leverage_now: f64, target: &LeverageTarget, debt_costs: f64, equity_costs: f64) -> Funds {
    if programme <= 0.0 {
        // E4.a: a firm with no programme raises nothing, whatever the markets are offering.
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

/// E5: **real cash to owners.** A distribution is a payment to named holders, and it leaves the
/// balance — which is why a firm short of cash cannot make one (D1).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Distribution {
    pub to_holders_of: PartyId,
    pub cash: f64,
}

/// E7, §46 C2: **the management publishes an expectation** — its own adaptive read of its own
/// earnings — **and is then judged against it.** It is never a choice among written phrases, and the
/// figure it publishes is the one its own decisions read (§48 B4).
pub fn expects(last_seen: f64, held_before: f64, memory: f64) -> f64 {
    assert!(memory > 0.0 && memory < 1.0, "46 C2: a memory of {memory} is not a weighting");
    held_before * memory + last_seen * (1.0 - memory)
}

/// B6: **every cost is somebody's income and every revenue is somebody's outlay, party by party.** A
/// VERIFY on derived dust (Law 7): it measures and repairs nothing.
pub fn two_sided(costs_booked: f64, received_by_payees: f64, terms: usize) -> bool {
    (costs_booked - received_by_payees).abs()
        <= crate::num::dust(terms, &[costs_booked, received_by_payees])
}

// **§5 RUNS HERE** (0m2.1). `Reporting` was in `running.rs`, apart from the firm's own accounts.

/// **§32: A FIRM'S RESULT IS PUBLISHED, and it is a read of what actually happened to it.**
///
/// Law 19: revenue, cost and what it is worth are read off the register and the wire — never a
/// running total a module kept beside them.
pub struct Reporting {
    /// The event kind this publishes under, declared by the assembly.
    pub kind: u32,
}

impl Mechanism for Reporting {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut said: Vec<(u32, f64)> = Vec::new();
        for f in ctx.parties().of_kind(kinds::FIRM) {
            let who = PartyId(*f);
            if !ctx.parties().alive(who) {
                continue;
            }
            said.push((*f, equity(who, ctx.register(), ctx.instruments(), ctx.claims())));
        }
        for (who, worth) in said {
            // Observer A3: a firm's own result reaches its own subjects. What it publishes to the
            // world is §48's, and it is not this.
            ctx.say(self.kind, &[who], &[(0, Value::Num(worth))], false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
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
        // B1, B1.a, F1: no revenue without a buyer. Each row is a sale that happened, at a price
        // achieved — there is nothing here to apply a rate to.
        let sales = [
            Sale { to: party(20), units: 100.0, at_price: 9.0 },
            Sale { to: party(21), units: 50.0, at_price: 11.0 },
        ];
        assert_eq!(revenue(&sales), 1_450.0);
    }

    #[test]
    fn the_margin_is_a_read_and_a_firm_that_sold_nothing_has_none() {
        // B4.a: never a target the revenue was fitted to. A cost line struck as the gap to a chosen
        // margin would make that margin an attractor; nothing here can produce a cost from revenue.
        let p = operating_profit(1_450.0, 600.0, 400.0);
        assert_eq!(p, 450.0);
        let m = margin(p, 1_450.0).unwrap();
        assert!(m > 0.3 && m < 0.32);
        assert!(margin(-50.0, 0.0).is_none());
    }

    #[test]
    fn operating_profit_can_be_negative() {
        // B4: it is a residual, not a quantity with a sign anybody chose.
        assert_eq!(operating_profit(800.0, 600.0, 400.0), -200.0);
    }

    #[test]
    fn margin_moves_more_than_revenue_because_some_costs_do_not_move() {
        // B5: operating leverage is a CONSEQUENCE of the cost structure, not a coefficient. Halve
        // the output and the fixed line stays where it is; the margin falls further than the top
        // line did, and nothing multiplied anything.
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
        // C4.b: two representations of one thing, with the decision reading the stated one, is Law
        // 4's defect at the point it matters most. There is only the book.
        assert_eq!(receivables(&book().receivable_book), 600.0);
    }

    #[test]
    fn equity_is_the_read_and_it_can_be_negative() {
        // C3. Assets 3,400 against liabilities 2,350.
        let b = book();
        assert_eq!(b.equity(), 1_050.0);
        let sunk = Book { fixed_capital: 200.0, ..b };
        assert!(sunk.equity() < 0.0);
    }

    #[test]
    fn a_profitable_firm_with_a_growing_invoice_book_runs_out_of_cash() {
        // C4.a: profit and cash are different numbers, and the difference is where firms die. This
        // firm earned 450 and consumed 700 of working capital, so it went backwards in cash.
        let b = book();
        let before = 300.0;
        let cash = cash_from_operations(450.0, b.working_capital(), before);
        assert_eq!(b.working_capital(), 850.0);
        assert!(cash < 0.0);
    }

    #[test]
    fn a_firm_can_be_out_of_cash_without_being_insolvent_and_the_other_way_round() {
        // D4: they are different failures, and a firm can be either without the other.
        let solvent_but_dry = book();
        assert_eq!(failing(&solvent_but_dry, 900.0), Failing::OutOfCash);
        assert_eq!(failing(&solvent_but_dry, 100.0), Failing::Neither);
        // Plenty of money in the account and a bond stack far beyond what the book is worth.
        let insolvent_with_cash = Book { cash: 5_000.0, fixed_capital: 0.0, bonds: 9_000.0, ..book() };
        assert_eq!(failing(&insolvent_with_cash, 100.0), Failing::Insolvent);
    }

    #[test]
    fn coverage_is_a_read_and_a_firm_with_no_debt_has_none_rather_than_an_infinite_one() {
        // D3: what lenders look at. Appendix A: missing is missing.
        let s = Service { interest: 60.0, principal: 140.0 };
        assert_eq!(coverage(400.0, &s), Some(2.0));
        assert!(coverage(400.0, &Service { interest: 0.0, principal: 0.0 }).is_none());
    }

    #[test]
    fn a_firm_with_no_programme_raises_nothing_whatever_the_markets_are_offering() {
        // E4.a: the money raised is raised INTO an actual investment programme.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(0.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Nothing);
    }

    #[test]
    fn a_management_above_its_own_target_does_not_borrow_whatever_debt_costs() {
        // E4.a: the target is the covenant line moderated by the management's own risk aversion, and
        // it is theirs. Debt is cheap in both of these; only the leverage differs.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(500.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Borrow(500.0));
        assert_eq!(funds(500.0, 0.0, 3.5, &t, 0.03, 0.10), Funds::Issue(500.0));
        // And it spends its own cash before raising anything at all.
        assert_eq!(funds(500.0, 900.0, 1.0, &t, 0.03, 0.10), Funds::FromCash(500.0));
    }

    #[test]
    fn the_funding_choice_depends_on_what_each_costs() {
        // E4: and this is XI-4's joint — a financial price changes, the firm's choice changes.
        let t = LeverageTarget { covenant: 4.0, caution: 1.0 };
        assert_eq!(funds(500.0, 0.0, 1.0, &t, 0.12, 0.10), Funds::Issue(500.0));
    }

    #[test]
    fn the_management_forms_its_expectation_adaptively_from_its_own_earnings() {
        // E7, §46 C2: its own adaptive read, never a choice among written phrases — and the figure
        // it publishes is the one its own decisions read.
        let held = expects(600.0, 400.0, 0.7);
        assert!(held > 400.0 && held < 600.0);
        // A surprise moves it, and a repeat of the same number moves it less each time.
        let again = expects(600.0, held, 0.7);
        assert!(again > held);
        assert!(again - held < held - 400.0);
    }

    #[test]
    fn every_cost_is_somebodys_income() {
        // B6: party by party, on derived dust — a VERIFY that answers false rather than balancing.
        assert!(two_sided(1_000.0, 1_000.0, 2));
        assert!(!two_sided(1_000.0, 940.0, 2));
    }

    #[test]
    #[should_panic(expected = "cannot be rescaled")]
    fn a_cost_base_with_no_output_behind_it_cannot_be_rescaled() {
        costs_at(&lines(), 0.5, 0.0);
    }
}
