//! HOUSEHOLDS: a set of CELLS, each a named party with a weight, deciding one at a time and summed
//! weighted — never a representative agent and never a decision at an average.
//!
//! @spec 41 A1 · 41 A2 · 41 A2.a · 41 A2.b · 41 A2.c · 41 A2.d · 41 A2.e · 41 A2.f · 41 A2.g ·
//! @spec 41 A3 · 41 B1 · 41 B2 · 41 B3 · 41 B3.a · 41 B5 · 41 C1 · 41 C1.a · 41 C1.b · 41 C1.c ·
//! @spec 41 C1.d · 41 C2 · 41 C5 · 41 D1 · 41 D3 · 41 D4 · 41 D5 · 41 D5.a · 41 D6 · 41 E3 ·
//! @spec 41 E3.a · 41 E4 · 41 E4.a · 41 E5 · 41 F1.a · 41 F1.b · 41 F2 · XI-15 · XI-16 · Law 2,
//! @spec Law 4, Law 6, Law 19 · Appendix B
//!
//! **The heterogeneity is load-bearing** (A2). Every decision that matters here is a THRESHOLD, and
//! **a mean-preserving spread must be able to cause defaults** — with one agent it cannot, and a
//! default test applied to a band's mean is the same defect one level down. `crossings` is A2.g's
//! measurement: spread the cells about an unchanged weighted mean and the count of crossings RISES.
//! That is what proves the representation is a distribution and not an average in a distribution's
//! clothes.
//!
//! **No decision evaluated at an average** (A2.f). The sector's numbers are `Σ f(xᵢ)·wᵢ` and never
//! `f(Σ xᵢ·wᵢ)`, so every read here evaluates per cell first and weights afterwards — there is no
//! door in this module that takes a sector total and returns a decision.
//!
//! **Income the household did not RECEIVE is not income** (B3.a). Retained earnings raise the value
//! of what it owns and reach it on sale or distribution; `income` sums cash that arrived from named
//! payers, and a revaluation is not in it (D4).
//!
//! **Net worth is a read** (D3), and the portfolio choice between a deposit, a money fund and bills
//! directly is a real substitution — **it is how a policy rate reaches a saver** (D5.a). Fund shares
//! issued pro rata and never chosen means the substitution never happens, so `prefers` returns the
//! cell's own ranking rather than a share anybody allotted.
//!
//! **Ageing is a split at the cohort boundary, by date** (F1.a): when the calendar carries some of a
//! cell's members across it, those members become a cell in the next cohort and the split is EXACT. A
//! cell whose members straddle a boundary is an average of two cohorts, which A2.d forbids.

use crate::calendar::Day;
use crate::ids::PartyId;

/// A2.e, XI-15: **one POSSIBLE household with a multiplicity** — never the average of a group. A
/// named party with an account, a register of holdings, and a weight that is an integer count of how
/// many real households it is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
    pub who: PartyId,
    /// XI-15: a weight is a COUNT. Holdings are totals; per-member is a read.
    pub weight: f64,
    /// A2.b, F1.a: the cohort is a KEY dimension of the cell, so ageing splits at its boundary.
    pub born: Day,
    /// A2.c: its employment state, read from the engagement rows (XI-10).
    pub without_work: bool,
    /// Holdings are TOTALS for the cell (XI-15).
    pub deposits: f64,
    pub securities: f64,
    pub fund_shares: f64,
    pub housing: f64,
    pub mortgage: f64,
    pub consumer_credit: f64,
}

impl Cell {
    /// XI-15: per-member is a READ, computed when asked, and never a second stored number (Law 4).
    pub fn per_member(&self, total: f64) -> Option<f64> {
        if self.weight <= 0.0 {
            return None;
        }
        Some(total / self.weight)
    }

    /// D1: what it owns — deposits, securities held directly, fund shares, housing. Each is a real
    /// claim on a named issuer held in a register; the household sector holds a real book and is not
    /// a residual holder of what nobody else took (D1.a, D6).
    pub fn assets(&self) -> f64 {
        self.deposits + self.securities + self.fund_shares + self.housing
    }

    /// D2: and what it owes, which is somebody's asset.
    pub fn liabilities(&self) -> f64 {
        self.mortgage + self.consumer_credit
    }

    /// D3: **net worth is a read and never a stored number.** D4: it revalues when prices move, and
    /// that revaluation is not income.
    pub fn net_worth(&self) -> f64 {
        self.assets() - self.liabilities()
    }
}

/// B1–B4: what a cell was actually PAID this period, from named payers. B3.a: income it did not
/// receive is not income — there is no field here for earnings retained by something it owns.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Received {
    pub wages: f64,
    pub transfers: f64,
    /// Dividends, interest, coupons — cash that arrived.
    pub investment: f64,
    /// B4: income is taxed, and the tax is remitted by somebody.
    pub tax: f64,
}

impl Received {
    pub fn after_tax(&self) -> f64 {
        self.wages + self.transfers + self.investment - self.tax
    }
}

/// B5: **sector income is the sum of what households were actually paid**, never an accounting
/// identity solved for. `Σ f(xᵢ)·wᵢ` — per cell, then weighted.
pub fn sector_income(cells: &[(Cell, Received)]) -> f64 {
    cells.iter().map(|(c, r)| r.after_tax() * c.weight).sum()
}

/// C1: the reasons a cell has for how much to spend. Each is its own; C1.c's expectations and
/// confidence are the CELL's (§46 C1), never a published aggregate.
#[derive(Clone, Copy, Debug)]
pub struct Spending {
    pub income_now: f64,
    /// C1.b: wealth, which is why an asset price matters to demand.
    pub wealth: f64,
    /// C1.c: its own outlook — how much of its income it expects to keep having.
    pub expects_to_keep: f64,
    /// C1.d: **a household that cannot borrow spends what it has, whatever it wants.**
    pub can_borrow: f64,
}

/// C1: **the decision, per cell.** Law 6: a cell that cannot fund what it would like does not spend
/// it — that is not a clamp, it is not having the money. C2: the residual is saving, and saving is a
/// flow into what it owns.
pub fn spends(s: &Spending, out_of_income: f64, out_of_wealth: f64) -> f64 {
    let wants = s.income_now * s.expects_to_keep * out_of_income + s.wealth * out_of_wealth;
    let has = s.income_now + s.can_borrow;
    if wants < has {
        wants
    } else {
        has
    }
}

/// C2: what is left over. It is the residual of a decision, not a rate anybody set.
pub fn saves(income_now: f64, spent: f64) -> f64 {
    income_now - spent
}

/// C5: **consumption is the sum of what households actually bought**, weighted per cell.
pub fn sector_consumption(spent: &[(Cell, f64)]) -> f64 {
    spent.iter().map(|(c, s)| s * c.weight).sum()
}

/// D5, D5.a: where a saver's money goes, ranked by what this cell wants from it. **The choice between
/// a deposit, a money fund and bills directly is a real substitution and it is how a policy rate
/// reaches a saver** — fund shares issued pro rata and never chosen means the substitution never
/// happens, so this returns the cell's own ranking and allots nothing.
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

/// E3, E3.a: it services the debt out of income, and **interest plus principal** — the distinction
/// matters, because only one of them reduces what is owed.
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

/// E5: **the debt-service burden is a read** of the service against income, **and it can become
/// unpayable**. `None` where there is no income to read it against — which is itself the worst case
/// and not a zero burden.
pub fn burden(s: &Service, income_after_tax: f64) -> Option<f64> {
    if income_after_tax <= 0.0 {
        return None;
    }
    Some(s.total() / income_after_tax)
}

/// E4: **the default depends on the DISTRIBUTION, not the mean.** One cell, its own income, its own
/// service: it crosses or it does not.
pub fn defaults(s: &Service, income_after_tax: f64, liquid: f64) -> bool {
    s.total() > income_after_tax + liquid
}

/// A2.g: **the measurement that proves the representation is a distribution.** How many of these
/// cells cross, weighted — and A2.d's point is that this number moves under a mean-preserving spread
/// while the weighted mean does not.
pub fn crossings(cells: &[(Cell, Service, f64)]) -> f64 {
    cells
        .iter()
        .filter(|(_, s, income)| defaults(s, *income, 0.0))
        .map(|(c, _, _)| c.weight)
        .sum()
}

/// The weighted mean of a per-cell quantity — a READ, published beside `crossings` so A2.g's
/// comparison can be made. Nothing decides on it (A2.f).
pub fn weighted_mean(cells: &[(Cell, f64)]) -> Option<f64> {
    let weight: f64 = cells.iter().map(|(c, _)| c.weight).sum();
    if weight <= 0.0 {
        return None;
    }
    Some(cells.iter().map(|(c, x)| x * c.weight).sum::<f64>() / weight)
}

/// F1.a: **ageing is a split at the cohort boundary, by date.** The members the calendar carried
/// across become a cell in the next cohort, and the split is EXACT — a cell whose members straddle a
/// boundary is an average of two cohorts.
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

/// F2: **wealth transfers on dissolution, and it goes somewhere NAMED** — somebody inherits it. A
/// dissolution with no heir is a residual with no holder (Appendix B).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Inherited {
    pub from: PartyId,
    pub to: PartyId,
    pub amount: f64,
    pub on: Day,
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
        // D3, D4: there is no stored net worth to disagree with the holdings, and a house price
        // moving changes what it owns without anybody being paid anything.
        let mut c = cell(10, 1_000.0, 200.0, 800.0, 600.0);
        assert_eq!(c.net_worth(), 400.0);
        c.housing = 1_000.0;
        assert_eq!(c.net_worth(), 600.0);
        let received = Received { wages: 0.0, transfers: 0.0, investment: 0.0, tax: 0.0 };
        assert_eq!(received.after_tax(), 0.0);
    }

    #[test]
    fn per_member_is_a_read_of_a_total_and_a_cell_of_nobody_has_none() {
        // XI-15: holdings are TOTALS and per-member is computed when asked — never a second number.
        let c = cell(10, 500.0, 5_000.0, 0.0, 0.0);
        assert_eq!(c.per_member(c.deposits), Some(10.0));
        let empty = Cell { weight: 0.0, ..c };
        assert!(empty.per_member(empty.deposits).is_none());
    }

    #[test]
    fn income_the_household_did_not_receive_is_not_income() {
        // B3.a: retained earnings raise the value of what it owns and reach it on sale or
        // distribution. There is no field here to put them in.
        let paid = Received { wages: 400.0, transfers: 50.0, investment: 30.0, tax: 90.0 };
        assert_eq!(paid.after_tax(), 390.0);
    }

    #[test]
    fn sector_income_is_the_sum_of_what_households_were_actually_paid() {
        // B5: never an accounting identity solved for. Per cell, then weighted.
        let a = (cell(10, 1_000.0, 0.0, 0.0, 0.0), Received { wages: 400.0, transfers: 0.0, investment: 0.0, tax: 80.0 });
        let b = (cell(11, 200.0, 0.0, 0.0, 0.0), Received { wages: 1_500.0, transfers: 0.0, investment: 200.0, tax: 500.0 });
        assert_eq!(sector_income(&[a, b]), 320.0 * 1_000.0 + 1_200.0 * 200.0);
    }

    #[test]
    fn a_household_that_cannot_borrow_spends_what_it_has_whatever_it_wants() {
        // C1.d, Law 6: this is not a clamp on a desire — it is not having the money.
        let liquid = Spending { income_now: 100.0, wealth: 10_000.0, expects_to_keep: 1.0, can_borrow: 500.0 };
        let dry = Spending { can_borrow: 0.0, ..liquid };
        assert!(spends(&liquid, 0.9, 0.05) > spends(&dry, 0.9, 0.05));
        assert_eq!(spends(&dry, 0.9, 0.05), 100.0);
    }

    #[test]
    fn wealth_matters_to_demand_which_is_why_an_asset_price_reaches_consumption() {
        // C1.b. The same income, a different balance sheet, a different decision.
        let poorer = Spending { income_now: 500.0, wealth: 1_000.0, expects_to_keep: 1.0, can_borrow: 10_000.0 };
        let richer = Spending { wealth: 40_000.0, ..poorer };
        assert!(spends(&richer, 0.8, 0.05) > spends(&poorer, 0.8, 0.05));
    }

    #[test]
    fn the_same_aggregate_income_produces_different_demand_depending_on_who_has_it() {
        // A2.a, A2.f: the sector's number is Σ f(xᵢ)·wᵢ and never f(Σ xᵢ·wᵢ). Two distributions with
        // the same weighted income spend differently, which is the whole reason for cells.
        //
        // The two cells have their OWN propensities — A2.a says the propensity to consume differs,
        // and it is a preference of each cell, not a sector coefficient. The many-membered cell
        // spends almost all of what arrives; the small wealthy one spends half.
        let sector = |income_many: f64, income_few: f64| {
            let many = Spending { income_now: income_many, wealth: 0.0, expects_to_keep: 1.0, can_borrow: 0.0 };
            let few = Spending { income_now: income_few, wealth: 0.0, expects_to_keep: 1.0, can_borrow: 0.0 };
            sector_consumption(&[
                (cell(10, 900.0, 0.0, 0.0, 0.0), spends(&many, 0.95, 0.0)),
                (cell(11, 100.0, 0.0, 0.0, 0.0), spends(&few, 0.50, 0.0)),
            ])
        };
        // Both worlds pay the same weighted total — 900×100 + 100×100 and 900×90 + 100×190 are both
        // 100,000 — and they differ only in WHO got it. The sector's demand is not the same, and a
        // representative agent could not tell these two worlds apart at all.
        let even = sector(100.0, 100.0);
        let tilted_to_the_wealthy = sector(90.0, 190.0);
        assert!(even > tilted_to_the_wealthy);
    }

    #[test]
    fn a_mean_preserving_spread_raises_the_count_of_crossings_while_the_mean_does_not_move() {
        // A2.g: the measurement that proves the representation is a distribution and not an average
        // wearing a distribution's clothes. A2.d: with one agent it cannot happen at all.
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
        // E5, E3.a: interest plus principal, and only one of them reduces what is owed.
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
        // D5.a: fund shares issued pro rata and never chosen means the substitution never happens.
        assert_eq!(prefers(0.01, 0.03, 0.035, false), Where::BillsDirectly);
        assert_eq!(prefers(0.01, 0.03, 0.02, false), Where::MoneyFund);
        assert_eq!(prefers(0.04, 0.03, 0.02, false), Where::Deposit);
        // And liquidity is one of D5's reasons, so the rate is not the whole decision.
        assert_eq!(prefers(0.01, 0.03, 0.035, true), Where::Deposit);
    }

    #[test]
    fn ageing_is_an_exact_split_at_the_cohort_boundary() {
        // F1.a: a cell whose members straddle a boundary is an average of two cohorts, which A2.d
        // forbids. The split conserves the weight exactly — a weight is a count.
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
        // F2, Appendix B: a dissolution with no heir is a residual with no holder.
        let i = Inherited { from: party(10), to: party(11), amount: 4_000.0, on: Day(500) };
        assert_ne!(i.from, i.to);
        assert!(i.amount > 0.0);
    }
}
