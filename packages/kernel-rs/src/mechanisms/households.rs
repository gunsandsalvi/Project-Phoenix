//! HOUSEHOLDS: a set of CELLS, each a named party with a weight, deciding one at a time and summed
//! weighted — never a representative agent and never a decision at an average.
//!
//! @spec 41 A1 · 41 A2 · 41 A2.a · 41 A2.b · 41 A2.c · 41 A2.d · 41 A2.e · 41 A2.f · 41 A2.g ·
//! @spec 41 A3 · 41 B1 · 41 B2 · 41 B3 · 41 B3.a · 41 B5 · 41 C1 · 41 C1.a · 41 C1.b · 41 C1.c ·
//! @spec 41 C1.d · 41 C2 · 41 C5 · 41 D1 · 41 D3 · 41 D4 · 41 D5 · 41 D5.a · 41 D6 · 41 E3 ·
//! @spec 41 E3.a · 41 E4 · 41 E4.a · 41 E5 · 41 F1.a · 41 F1.b · 41 F2 · XI-15 · XI-16 · Law 2,
//! @spec Law 4, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{InstrumentId, MarketId, PartyId};
use crate::module::{Participant, ParticipantView};
use crate::params::Denomination;
use crate::calendar::Day;

/// One POSSIBLE household with a multiplicity — never the average of a group.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
    pub who: PartyId,
    pub weight: f64,
    /// The cohort is a KEY dimension of the cell, so ageing splits at its boundary.
    pub born: Day,
    /// Its employment state, read from the engagement rows.
    pub without_work: bool,
    /// Holdings are TOTALS for the cell.
    pub deposits: f64,
    pub securities: f64,
    pub fund_shares: f64,
    pub housing: f64,
    pub mortgage: f64,
    pub consumer_credit: f64,
}

impl Cell {
    /// Per-member is a READ, computed when asked, and never a second stored number.
    pub fn per_member(&self, total: f64) -> Option<f64> {
        if self.weight <= 0.0 {
            return None;
        }
        Some(total / self.weight)
    }

    /// What it owns — deposits, securities held directly, fund shares, housing.
    pub fn assets(&self) -> f64 {
        self.deposits + self.securities + self.fund_shares + self.housing
    }

    /// And what it owes, which is somebody's asset.
    pub fn liabilities(&self) -> f64 {
        self.mortgage + self.consumer_credit
    }

    /// Net worth is a read and never a stored number.
    pub fn net_worth(&self) -> f64 {
        self.assets() - self.liabilities()
    }
}

/// What a cell was actually PAID this period, from named payers.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Received {
    pub wages: f64,
    pub transfers: f64,
    /// Dividends, interest, coupons — cash that arrived.
    pub investment: f64,
    /// Income is taxed, and the tax is remitted by somebody.
    pub tax: f64,
}

impl Received {
    pub fn after_tax(&self) -> f64 {
        self.wages + self.transfers + self.investment - self.tax
    }
}

/// Sector income is the sum of what households were actually paid, never an accounting identity
/// solved for.
pub fn sector_income(cells: &[(Cell, Received)]) -> f64 {
    cells.iter().map(|(c, r)| r.after_tax() * c.weight).sum()
}

/// The reasons a cell has for how much to spend.
#[derive(Clone, Copy, Debug)]
pub struct Spending {
    pub income_now: f64,
    /// Wealth, which is why an asset price matters to demand.
    pub wealth: f64,
    /// Its own outlook — how much of its income it expects to keep having.
    pub expects_to_keep: f64,
    /// A household that cannot borrow spends what it has, whatever it wants.
    pub can_borrow: f64,
}

/// The decision, per cell.
pub fn spends(s: &Spending, out_of_income: f64, out_of_wealth: f64) -> f64 {
    let wants = s.income_now * s.expects_to_keep * out_of_income + s.wealth * out_of_wealth;
    let has = s.income_now + s.can_borrow;
    if wants < has {
        wants
    } else {
        has
    }
}

/// What is left over.
pub fn saves(income_now: f64, spent: f64) -> f64 {
    income_now - spent
}

/// Consumption is the sum of what households actually bought, weighted per cell.
pub fn sector_consumption(spent: &[(Cell, f64)]) -> f64 {
    spent.iter().map(|(c, s)| s * c.weight).sum()
}

/// Where a saver's money goes, ranked by what this cell wants from it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Where {
    Deposit,
    MoneyFund,
    BillsDirectly,
}

pub fn prefers(deposit_pays: f64, fund_pays: f64, bills_pay: f64, wants_it_liquid: bool) -> Where {
    if wants_it_liquid {
        // Liquidity is one of D5's reasons, and for a cell that needs it the rate is not the whole
        // of the decision.
        return Where::Deposit;
    }
    if bills_pay > fund_pays && bills_pay > deposit_pays {
        Where::BillsDirectly
    } else if fund_pays > deposit_pays {
        Where::MoneyFund
    } else {
        Where::Deposit
    }
}

/// It services the debt out of income, and interest plus principal — the distinction matters,
/// because only one of them reduces what is owed.
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

/// The debt-service burden is a read of the service against income, and it can become unpayable.
pub fn burden(s: &Service, income_after_tax: f64) -> Option<f64> {
    if income_after_tax <= 0.0 {
        return None;
    }
    Some(s.total() / income_after_tax)
}

/// The default depends on the DISTRIBUTION, not the mean.
pub fn defaults(s: &Service, income_after_tax: f64, liquid: f64) -> bool {
    s.total() > income_after_tax + liquid
}

/// The measurement that proves the representation is a distribution.
pub fn crossings(cells: &[(Cell, Service, f64)]) -> f64 {
    cells
        .iter()
        .filter(|(_, s, income)| defaults(s, *income, 0.0))
        .map(|(c, _, _)| c.weight)
        .sum()
}

/// The weighted mean of a per-cell quantity — a READ, published beside `crossings` so A2.g's
/// comparison can be made.
pub fn weighted_mean(cells: &[(Cell, f64)]) -> Option<f64> {
    let weight: f64 = cells.iter().map(|(c, _)| c.weight).sum();
    if weight <= 0.0 {
        return None;
    }
    Some(cells.iter().map(|(c, x)| x * c.weight).sum::<f64>() / weight)
}

/// Ageing is a split at the cohort boundary, by date.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Split {
    pub stays: f64,
    pub crosses: f64,
}

pub fn age_at_boundary(cell: &Cell, crossing: f64) -> Split {
    assert!(
        crossing >= 0.0 && crossing <= cell.weight,
        "XI-15: {crossing} of a cell of {} cannot cross a boundary",
        cell.weight
    );
    Split { stays: cell.weight - crossing, crosses: crossing }
}

/// Wealth transfers on dissolution, and it goes somewhere NAMED — somebody inherits it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Inherited {
    pub from: PartyId,
    pub to: PartyId,
    pub amount: f64,
    pub on: Day,
}


/// Households buy because they need the thing, and what they can spend is what they have (C1.d: a
/// household that cannot borrow spends what it has, whatever it wants).
pub struct HouseholdBuyers {
    /// The money it keeps back.
    pub keeps: &'static str,
    /// The lines a household consumes.
    pub basket: Vec<InstrumentId>,
}

impl Participant for HouseholdBuyers {
    fn party_kind(&self) -> u32 {
        kinds::HOUSEHOLD
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // A household with no money is in no book.
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.basket.iter().filter_map(|line| view.market_of(*line)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        if money <= 0.0 {
            return Vec::new();
        }
        // WHAT IT WILL PAY IS A PRICE.
        let Some(limit) = view.subject_of(m).and_then(|line| view.price_outlook(line)) else {
            return Vec::new();
        };
        if limit <= 0.0 {
            return Vec::new();
        }
        // IT SPENDS OUT OF ITS WEALTH, NOT JUST ITS INCOME — but it keeps a buffer, and what it
        // keeps is its own (a PREFERENCE, dispersed like any other).
        let keeps = view.params().amount(self.keeps, Denomination::Money);
        let spendable = money - keeps;
        if spendable <= 0.0 {
            return Vec::new();
        }
        // It bids for what it can actually fund — and 22c.2: less what it is already bidding for
        // here, because a resting bid is money it has committed once already.
        let (already, _) = view.resting(m);
        let affordable = whole_pieces(spendable / limit) - already;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(limit), qty: affordable }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn cell(who: u32, weight: f64, deposits: f64, housing: f64, mortgage: f64) -> Cell {
        Cell {
            who: party(who),
            weight,
            born: Day(-10_000),
            without_work: false,
            deposits,
            securities: 0.0,
            fund_shares: 0.0,
            housing,
            mortgage,
            consumer_credit: 0.0,
        }
    }

    #[test]
    fn net_worth_is_a_read_and_a_revaluation_is_not_income() {
        // There is no stored net worth to disagree with the holdings, and a house price moving
        // changes what it owns without anybody being paid anything.
        let mut c = cell(10, 1_000.0, 200.0, 800.0, 600.0);
        assert_eq!(c.net_worth(), 400.0);
        c.housing = 1_000.0;
        assert_eq!(c.net_worth(), 600.0);
        let received = Received { wages: 0.0, transfers: 0.0, investment: 0.0, tax: 0.0 };
        assert_eq!(received.after_tax(), 0.0);
    }

    #[test]
    fn per_member_is_a_read_of_a_total_and_a_cell_of_nobody_has_none() {
        // Holdings are TOTALS and per-member is computed when asked — never a second number.
        let c = cell(10, 500.0, 5_000.0, 0.0, 0.0);
        assert_eq!(c.per_member(c.deposits), Some(10.0));
        let empty = Cell { weight: 0.0, ..c };
        assert!(empty.per_member(empty.deposits).is_none());
    }

    #[test]
    fn income_the_household_did_not_receive_is_not_income() {
        // Retained earnings raise the value of what it owns and reach it on sale or distribution.
        let paid = Received { wages: 400.0, transfers: 50.0, investment: 30.0, tax: 90.0 };
        assert_eq!(paid.after_tax(), 390.0);
    }

    #[test]
    fn sector_income_is_the_sum_of_what_households_were_actually_paid() {
        // Never an accounting identity solved for.
        let a = (cell(10, 1_000.0, 0.0, 0.0, 0.0), Received { wages: 400.0, transfers: 0.0, investment: 0.0, tax: 80.0 });
        let b = (cell(11, 200.0, 0.0, 0.0, 0.0), Received { wages: 1_500.0, transfers: 0.0, investment: 200.0, tax: 500.0 });
        assert_eq!(sector_income(&[a, b]), 320.0 * 1_000.0 + 1_200.0 * 200.0);
    }

    #[test]
    fn a_household_that_cannot_borrow_spends_what_it_has_whatever_it_wants() {
        // This is not a clamp on a desire — it is not having the money.
        let liquid = Spending { income_now: 100.0, wealth: 10_000.0, expects_to_keep: 1.0, can_borrow: 500.0 };
        let dry = Spending { can_borrow: 0.0, ..liquid };
        assert!(spends(&liquid, 0.9, 0.05) > spends(&dry, 0.9, 0.05));
        assert_eq!(spends(&dry, 0.9, 0.05), 100.0);
    }

    #[test]
    fn wealth_matters_to_demand_which_is_why_an_asset_price_reaches_consumption() {
        // The same income, a different balance sheet, a different decision.
        let poorer = Spending { income_now: 500.0, wealth: 1_000.0, expects_to_keep: 1.0, can_borrow: 10_000.0 };
        let richer = Spending { wealth: 40_000.0, ..poorer };
        assert!(spends(&richer, 0.8, 0.05) > spends(&poorer, 0.8, 0.05));
    }

    #[test]
    fn the_same_aggregate_income_produces_different_demand_depending_on_who_has_it() {
        // The sector's number is Σ f(xᵢ)·wᵢ and never f(Σ xᵢ·wᵢ).
        let sector = |income_many: f64, income_few: f64| {
            let many = Spending { income_now: income_many, wealth: 0.0, expects_to_keep: 1.0, can_borrow: 0.0 };
            let few = Spending { income_now: income_few, wealth: 0.0, expects_to_keep: 1.0, can_borrow: 0.0 };
            sector_consumption(&[
                (cell(10, 900.0, 0.0, 0.0, 0.0), spends(&many, 0.95, 0.0)),
                (cell(11, 100.0, 0.0, 0.0, 0.0), spends(&few, 0.50, 0.0)),
            ])
        };
        // Both worlds pay the same weighted total — 900×100 + 100×100 and 900×90 + 100×190 are both
        // 100,000 — and they differ only in WHO got it.
        let even = sector(100.0, 100.0);
        let tilted_to_the_wealthy = sector(90.0, 190.0);
        assert!(even > tilted_to_the_wealthy);
    }

    #[test]
    fn a_mean_preserving_spread_raises_the_count_of_crossings_while_the_mean_does_not_move() {
        // The measurement that proves the representation is a distribution and not an average
        // wearing a distribution's clothes.
        let service = Service { interest: 20.0, principal: 80.0 };
        let tight = vec![
            (cell(10, 500.0, 0.0, 0.0, 0.0), service, 110.0),
            (cell(11, 500.0, 0.0, 0.0, 0.0), service, 130.0),
        ];
        let spread = vec![
            (cell(10, 500.0, 0.0, 0.0, 0.0), service, 60.0),
            (cell(11, 500.0, 0.0, 0.0, 0.0), service, 180.0),
        ];
        // The weighted mean income is 120 in both.
        let mean_of = |v: &Vec<(Cell, Service, f64)>| {
            weighted_mean(&v.iter().map(|(c, _, i)| (*c, *i)).collect::<Vec<_>>()).unwrap()
        };
        assert_eq!(mean_of(&tight), mean_of(&spread));
        // And the crossings are not.
        assert_eq!(crossings(&tight), 0.0);
        assert_eq!(crossings(&spread), 500.0);
    }

    #[test]
    fn the_debt_service_burden_is_a_read_and_can_become_unpayable() {
        // Interest plus principal, and only one of them reduces what is owed.
        let s = Service { interest: 30.0, principal: 70.0 };
        assert_eq!(s.total(), 100.0);
        assert_eq!(burden(&s, 400.0), Some(0.25));
        // No income to read it against is the worst case, not a zero burden.
        assert!(burden(&s, 0.0).is_none());
        assert!(defaults(&s, 60.0, 20.0));
        assert!(!defaults(&s, 60.0, 50.0));
    }

    #[test]
    fn the_saver_chooses_between_a_deposit_a_money_fund_and_bills_which_is_how_a_rate_reaches_it() {
        // Fund shares issued pro rata and never chosen means the substitution never happens.
        assert_eq!(prefers(0.01, 0.03, 0.035, false), Where::BillsDirectly);
        assert_eq!(prefers(0.01, 0.03, 0.02, false), Where::MoneyFund);
        assert_eq!(prefers(0.04, 0.03, 0.02, false), Where::Deposit);
        // And liquidity is one of D5's reasons, so the rate is not the whole decision.
        assert_eq!(prefers(0.01, 0.03, 0.035, true), Where::Deposit);
    }

    #[test]
    fn ageing_is_an_exact_split_at_the_cohort_boundary() {
        // A cell whose members straddle a boundary is an average of two cohorts, which A2.d forbids.
        let c = cell(10, 1_000.0, 0.0, 0.0, 0.0);
        let s = age_at_boundary(&c, 240.0);
        assert_eq!(s.stays, 760.0);
        assert_eq!(s.crosses, 240.0);
        assert_eq!(s.stays + s.crosses, c.weight);
    }

    #[test]
    #[should_panic(expected = "cannot cross a boundary")]
    fn more_members_than_a_cell_has_cannot_cross() {
        age_at_boundary(&cell(10, 1_000.0, 0.0, 0.0, 0.0), 1_400.0);
    }

    #[test]
    fn wealth_on_dissolution_goes_somewhere_named() {
        // A dissolution with no heir is a residual with no holder.
        let i = Inherited { from: party(10), to: party(11), amount: 4_000.0, on: Day(500) };
        assert_ne!(i.from, i.to);
        assert!(i.amount > 0.0);
    }
}
