//! FIRM FUNDAMENTALS: a named party with a book, whose profit is a residual, whose equity is a read,
//! and which can run out of cash two different ways.
//!
//! @spec 32 A1 · 32 A2 · 32 A3 · 32 A4 · 32 B1 · 32 B1.a · 32 B1.b · 32 B2 · 32 B3 · 32 B4 ·
//! @spec 32 B4.a · 32 B4.b · 32 B5 · 32 B6 · 32 C1 · 32 C2 · 32 C3 · 32 C4 · 32 C4.a · 32 C4.b ·
//! @spec 32 D1 · 32 D2 · 32 D3 · 32 D4 · 32 D5 · 32 E4 · 32 E4.a · 32 E5 · 32 E7 · 32 F1 · 32 F2 ·
//! @spec 32 F3 · XI-4 · 46 C2 · Law 2, Law 3, Law 4, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::ids::PartyId;

/// The whole invoice book is what is owed, so the window the payee read takes is every week this
/// world could still be owed in rather than a horizon anybody chose.
const FAR: i64 = 10_000;
use crate::instruments::booked_equity;
use crate::journal::Value;
use crate::ledger::{Leg, Outcome, Receipt};
use crate::module::{Mechanism, MechanismContext, Service};

/// Revenue is quantity sold times price achieved, from named buyers — a consequence of a market,
/// never a growth rate applied to last week.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sale {
    pub to: PartyId,
    pub units: f64,
    pub at_price: f64,
}

pub fn revenue(sales: &[Sale]) -> f64 {
    sales.iter().map(|s| s.units * s.at_price).sum()
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

/// 32 C4: WORKING CAPITAL IS A REAL USE OF CASH — stock bought and not yet sold, and invoices sent
/// and not yet paid, less what the firm itself has not yet paid. Every part is the sum of actual
/// rows, never a ratio of revenue.
pub fn working_capital(stock: f64, owed_to_it: f64, owed_by_it: f64) -> f64 {
    stock + owed_to_it - owed_by_it
}

/// 32 C4.a: profit and cash are different numbers, and the difference is where firms die — what a
/// week's result tied up rather than banked.
pub fn cash_from_operations(profit: f64, working_capital_now: f64, before: f64) -> f64 {
    profit - (working_capital_now - before)
}

/// Coverage is a read of operating cash against debt service, and it is what lenders look at.
pub fn coverage(operating_cash: f64, s: &Service) -> Option<f64> {
    if s.total() <= 0.0 {
        return None;
    }
    Some(operating_cash / s.total())
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
pub fn funds(
    programme: f64,
    cash_spare: f64,
    leverage_now: f64,
    target: &LeverageTarget,
    debt_costs: f64,
    equity_costs: f64,
) -> Funds {
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
    assert!(
        memory > 0.0 && memory < 1.0,
        "46 C2: a memory of {memory} is not a weighting"
    );
    held_before * memory + last_seen * (1.0 - memory)
}

/// 37 F5, F5.a: THE OPERATING RESULT IS WHAT IT SOLD. Revenue is recognised when the units are
/// delivered, and the charge against it is what those units cost — so a firm that produces and does
/// not sell carries the cost in its stock instead of charging it, which is what absorption means.
/// What it paid for an input is not a cost; it is stock it now holds.
///
/// Financing principal and transfers are not sales, so neither can inflate or depress profit.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct OperatingFlows {
    pub revenue: f64,
    /// 32 B2: what the units it SOLD had cost it. This is the part that moves with output.
    pub cost_of_sales: f64,
    /// 32 B3: headcount times wage, paid to named households. No batch absorbed it.
    pub labour: f64,
    /// 32 B2, B5: money it paid out and got no units back for — upkeep, storage, rent. Fixed in
    /// the week, which is why its margin moves more than its revenue does.
    pub period: f64,
}

impl OperatingFlows {
    /// 32 B4: the residual of revenue less what it bought less what it paid for labour, and it can
    /// be negative.
    pub fn cash(self) -> f64 {
        operating_profit(self.revenue, self.cost_of_sales + self.period, self.labour)
    }

    /// 32 B5: what does NOT move with what it made this week. Operating leverage is the ratio of
    /// this to the rest, and it is a consequence of the cost structure rather than a coefficient.
    pub fn fixed(self) -> f64 {
        self.labour + self.period
    }

    /// WHAT AN INSTRUCTION DID TO THIS FIRM. Money out against units in is stock it now holds and
    /// not a cost; money out against nothing is a period cost, which is what absorption means.
    pub fn reads(&mut self, who: PartyId, legs: &[Leg]) {
        let took_units_in = legs.iter().any(|leg| match leg {
            Leg::Asset { to, .. } => *to == who,
            _ => false,
        });
        for leg in legs {
            let Leg::Money {
                from,
                amount,
                receipt,
                ..
            } = leg
            else {
                continue;
            };
            if *from != who {
                continue;
            }
            match receipt {
                Receipt::Wage => self.labour += amount.get(),
                Receipt::Sale if !took_units_in => self.period += amount.get(),
                Receipt::Sale
                | Receipt::Interest
                | Receipt::Dividend
                | Receipt::Transfer
                | Receipt::Tax
                | Receipt::Principal
                | Receipt::Capital
                | Receipt::Fx => {}
            }
        }
    }

    /// What a disposal fetched and what the units that left cost, from the one pass that holds the
    /// price and the basis at once.
    pub fn sold(&mut self, proceeds: f64, cost: f64) {
        self.revenue += proceeds;
        self.cost_of_sales += cost;
    }
}

// §5 RUNS HERE.

/// A FIRM'S RESULT IS PUBLISHED, and it is a read of what actually happened to it.
pub struct Reporting {
    /// The event kind this publishes under, declared by the assembly.
    pub kind: u32,
    pub at_revenue: u32,
    pub at_costs: u32,
    /// 32 B5: what did not move with what it made, so operating leverage is readable.
    pub at_fixed: u32,
    /// 32 C4: what the week's stock and invoice books tied up.
    pub at_working_capital: u32,
    /// 32 C4.a: and what that left of the week's profit.
    pub at_operating_cash: u32,
    pub at_cash: u32,
    pub at_equity: u32,
    pub at_opening_equity: u32,
}

impl Mechanism for Reporting {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut said: Vec<(u32, f64, OperatingFlows, Option<f64>)> = Vec::new();
        for f in ctx.parties().of_kind(kinds::FIRM) {
            let who = PartyId(*f);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(worth) = booked_equity(
                who,
                ctx.register(),
                ctx.instruments(),
                &ctx.marks(),
                ctx.claims(),
                ctx.week(),
            ) else {
                continue;
            };
            let mut flows = OperatingFlows::default();
            for instruction in ctx.wire().in_period(ctx.week()) {
                if ctx.wire().outcome_of(instruction) != Outcome::Settled {
                    continue;
                }
                flows.reads(who, ctx.wire().legs_of(instruction));
            }
            // What it sold, and what those units cost — said by settlement, never re-derived here.
            let realised = ctx.kernel_says().realised;
            for row in ctx.journal().of_kind(realised) {
                if ctx.journal().period_of(*row) != ctx.week()
                    || ctx.journal().subjects_of(*row).first() != Some(&who.0)
                {
                    continue;
                }
                if let (Some(Value::Num(proceeds)), Some(Value::Num(cost))) =
                    (ctx.journal().says(*row, 2), ctx.journal().says(*row, 3))
                {
                    flows.sold(proceeds, cost);
                }
            }
            // 32 C1, C2, C4, C4.b: the balance sheet is a READ. What it holds is its own register
            // rows at what each is carried at; what it owes is the unpaid dues against it, summed
            // from the actual rows and never from a share of revenue.
            // A lot nobody can value leaves the whole read MISSING: stock that is not priced is
            // not stock worth nothing.
            let mut stock = Some(0.0);
            for &row in ctx.register().of_holder(who) {
                let row = crate::ids::HoldingId(row);
                if ctx
                    .instruments()
                    .class_of(ctx.register().instrument_of(row))
                    != crate::instruments::Class::Good
                {
                    continue;
                }
                let lot = crate::instruments::carrying_value(
                    row,
                    ctx.register(),
                    ctx.instruments(),
                    &ctx.marks(),
                    ctx.week(),
                );
                stock = stock.zip(lot).map(|(so_far, lot)| so_far + lot);
            }
            let owed_by_it: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d))
                .map(|d| ctx.schedules().amount(d))
                .sum();
            let owed_to_it = ctx.schedules().falling_to(
                who,
                ctx.calendar().at(crate::calendar::Week(0)),
                ctx.calendar()
                    .at(crate::calendar::Week(i64::from(ctx.week()) + FAR)),
                ctx.register(),
                ctx.instruments(),
            );
            said.push((
                *f,
                worth,
                flows,
                stock.map(|stock| working_capital(stock, owed_to_it, owed_by_it)),
            ));
        }
        for (who, worth, flows, working_capital) in said {
            // Its own previous publication, which is a public record and not a number kept aside.
            let last_week = ctx
                .journal()
                .of_kind(self.kind)
                .iter()
                .filter(|row| {
                    ctx.journal().period_of(**row) + 1 == ctx.week()
                        && ctx.journal().subjects_of(**row).first() == Some(&who)
                })
                .find_map(
                    |row| match ctx.journal().says(*row, self.at_working_capital) {
                        Some(Value::Num(tied_up)) => Some(tied_up),
                        _ => None,
                    },
                );
            let opening_equity = match ctx.equity().opening_of(PartyId(who)) {
                Some(equity) => equity,
                None => worth,
            };
            // A firm's result is private now, but remains a typed observation consumed by its own
            // outlook next week. Public accounts remain the creditors' and owners' legal read.
            let mut terms = vec![
                (self.at_revenue, Value::Num(flows.revenue)),
                (
                    self.at_costs,
                    Value::Num(flows.cost_of_sales + flows.labour + flows.period),
                ),
                (self.at_fixed, Value::Num(flows.fixed())),
                (self.at_cash, Value::Num(flows.cash())),
                (self.at_equity, Value::Num(worth)),
                (self.at_opening_equity, Value::Num(opening_equity)),
            ];
            if let Some(tied_up) = working_capital {
                terms.push((self.at_working_capital, Value::Num(tied_up)));
                // 32 C4.a: what the week EARNED against what it banked. The difference is what
                // stock and unpaid invoices took, read off what this firm published last week.
                if let Some(before) = last_week {
                    terms.push((
                        self.at_operating_cash,
                        Value::Num(cash_from_operations(flows.cash(), tied_up, before)),
                    ));
                }
            }
            ctx.say(self.kind, &[who], &terms, false);
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
    fn the_result_is_what_it_sold_and_not_what_it_paid() {
        let firm = party(4);
        let customer = party(5);
        let lender = party(6);
        let money = crate::ids::InstrumentId::at(1);
        let paid = |from, to, amount, receipt| Leg::Money {
            from,
            to,
            instrument: money,
            amount: crate::ledger::Units::new(amount).expect("a leg moves something"),
            receipt,
        };
        let mut flows = OperatingFlows::default();
        // 37 F5.a: what it paid for an input is stock it now holds, not a cost of this week.
        let units = |to| Leg::Asset {
            from: customer,
            to,
            instrument: crate::ids::InstrumentId::at(2),
            qty: crate::ledger::Units::new(10.0).expect("a leg moves something"),
            price_per_unit: Some(7.0),
        };
        flows.reads(
            firm,
            &[paid(firm, customer, 70.0, Receipt::Sale), units(firm)],
        );
        // Financing principal is not revenue.
        flows.reads(firm, &[paid(lender, firm, 500.0, Receipt::Principal)]);
        // An idle line's payroll is a period cost, because no batch absorbed it.
        flows.reads(firm, &[paid(firm, customer, 30.0, Receipt::Wage)]);
        // 32 B2, B5: and money out with NOTHING coming back is a period cost too — upkeep, rent,
        // storage. Invisible before, which is what made a firm's cost base look all variable.
        flows.reads(firm, &[paid(firm, customer, 12.0, Receipt::Sale)]);
        assert_eq!(
            flows,
            OperatingFlows {
                revenue: 0.0,
                cost_of_sales: 0.0,
                labour: 30.0,
                period: 12.0,
            }
        );
        // And the sale: recognised on delivery, charged with what the units that left cost.
        flows.sold(120.0, 70.0);
        assert_eq!(
            flows,
            OperatingFlows {
                revenue: 120.0,
                cost_of_sales: 70.0,
                labour: 30.0,
                period: 12.0,
            }
        );
        assert_eq!(flows.cash(), 8.0);
        // 32 B5: and what of that did not move with what it sold.
        assert_eq!(flows.fixed(), 42.0);
    }

    #[test]
    fn revenue_comes_from_named_buyers_and_never_from_a_growth_rate() {
        // No revenue without a buyer.
        let sales = [
            Sale {
                to: party(20),
                units: 100.0,
                at_price: 9.0,
            },
            Sale {
                to: party(21),
                units: 50.0,
                at_price: 11.0,
            },
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
        // 32 B5: operating leverage is a CONSEQUENCE of the cost structure, not a coefficient.
        // The same firm, selling half as much: only what the units cost it moves with output.
        let busy = OperatingFlows {
            revenue: 1_450.0,
            cost_of_sales: 600.0,
            labour: 300.0,
            period: 100.0,
        };
        let slack = OperatingFlows {
            revenue: 725.0,
            cost_of_sales: 300.0,
            ..busy
        };
        let full = margin(busy.cash(), busy.revenue).unwrap();
        let half = margin(slack.cash(), slack.revenue).unwrap();
        assert!(half < full);
        // The revenue halved; the margin fell by more than half of itself.
        assert!(full - half > full * 0.5);
        // And what did not move is what made it so.
        assert_eq!(busy.fixed(), slack.fixed());
    }

    #[test]
    fn a_profitable_firm_with_a_growing_invoice_book_runs_out_of_cash() {
        // 32 C4, C4.a: what stock and unpaid invoices tie up is a real use of cash, so profit and
        // cash are different numbers — and the difference is where firms die.
        let tied_up = working_capital(500.0, 600.0, 250.0);
        assert_eq!(tied_up, 850.0);
        assert!(cash_from_operations(450.0, tied_up, 300.0) < 0.0);
        // And a firm that collected what it was owed banks the profit instead.
        assert_eq!(cash_from_operations(450.0, 300.0, 300.0), 450.0);
    }

    #[test]
    fn coverage_is_a_read_and_a_firm_with_no_debt_has_none_rather_than_an_infinite_one() {
        // What lenders look at.
        let s = Service {
            interest: 60.0,
            principal: 140.0,
        };
        assert_eq!(coverage(400.0, &s), Some(2.0));
        assert!(coverage(
            400.0,
            &Service {
                interest: 0.0,
                principal: 0.0
            }
        )
        .is_none());
    }

    #[test]
    fn a_firm_with_no_programme_raises_nothing_whatever_the_markets_are_offering() {
        // The money raised is raised INTO an actual investment programme.
        let t = LeverageTarget {
            covenant: 4.0,
            caution: 1.0,
        };
        assert_eq!(funds(0.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Nothing);
    }

    #[test]
    fn a_management_above_its_own_target_does_not_borrow_whatever_debt_costs() {
        // The target is the covenant line moderated by the management's own risk aversion, and it is
        // theirs.
        let t = LeverageTarget {
            covenant: 4.0,
            caution: 1.0,
        };
        assert_eq!(funds(500.0, 0.0, 1.0, &t, 0.03, 0.10), Funds::Borrow(500.0));
        assert_eq!(funds(500.0, 0.0, 3.5, &t, 0.03, 0.10), Funds::Issue(500.0));
        // And it spends its own cash before raising anything at all.
        assert_eq!(
            funds(500.0, 900.0, 1.0, &t, 0.03, 0.10),
            Funds::FromCash(500.0)
        );
    }

    #[test]
    fn the_funding_choice_depends_on_what_each_costs() {
        // And this is XI-4's joint — a financial price changes, the firm's choice changes.
        let t = LeverageTarget {
            covenant: 4.0,
            caution: 1.0,
        };
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
}
