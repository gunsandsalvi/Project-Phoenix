//! CROSS-BORDER: two named parties in different regions, and **the world is closed** — so summing all
//! regions gives zero in every category.
//!
//! @spec 43 A1 · 43 A2 · 43 A2.a · 43 A3 · 43 A4 · 43 B1 · 43 B2 · 43 B3 · 43 B3.a · 43 B4 · 43 C1 ·
//! @spec 43 C2 · 43 C2.a · 43 C3 · 43 C4 · 43 C5 · 43 D1 · 43 D2 · 43 D3 · 43 D3.a · 43 D4 · 43 D4.a ·
//! @spec 43 D5 · 43 D6 · 43 E1 · 43 E2 · 43 E3 · 43 E4 · 43 F1 · 43 F2 · XI-12 · Law 5, Law 19
//!
//! **No netting of cross-border flows into a regional aggregate** (F2): the parties are named on every
//! leg, and an aggregate is a READ over those legs rather than a thing anybody writes. `current_account`
//! walks the flows; there is no door here that takes a region and a number.
//!
//! **A residual that has to be plugged is a transaction that lost a leg** (D3.a). `imbalance` reports it
//! and repairs nothing — and **summing all regions gives zero because the world is closed** (D6), which
//! is the check that actually catches a missing leg.
//!
//! **A deficit region must be financed by somebody who CHOOSES to finance it, at a price** (D4) — so
//! `financed_by` names the financier and can answer `None`, which is a real outcome and the reason the
//! price exists.
//!
//! **Whoever is not in the invoice currency has an FX exposure, which it can hedge or CARRY** (A2.a) —
//! and a foreign-currency borrower's exposure is **a real solvency risk that a rate move triggers, not a
//! translation adjustment** (C2.a).
//!
//! **No region that is a closed box** (F1): if every party trades only domestically, every node here is
//! decoration.

use crate::ids::{CurrencyCode, PartyId, RegionId};

/// A1, A2, A4: **two named parties in DIFFERENT regions**, in one of two currencies or a third — and
/// **the counterparty is foreign, which is a real credit and legal difference.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flow {
    pub from: PartyId,
    pub from_region: RegionId,
    pub to: PartyId,
    pub to_region: RegionId,
    pub amount: f64,
    /// A2: somebody decided which money this is in, and A2.a: whoever is not in it has an exposure.
    pub invoiced_in: CurrencyCode,
    pub entry: Entry,
}

/// D1: **the current account is goods and services plus INCOME flows** — C5: coupons, dividends,
/// profits — and D2: the financial account is the other side, the net acquisition of foreign claims.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// B1, B2: a firm buys from or sells to a firm in another region, and the goods MOVE.
    Goods,
    Services,
    /// C5: income across the border — coupons, dividends, profits.
    Income,
    /// C1, C2, C4: a claim acquired or given up — a foreign asset, an issue in a foreign currency, a
    /// direct investment that buys a firm outright.
    Claim,
}

impl Flow {
    pub fn is_cross_border(&self) -> bool {
        self.from_region != self.to_region
    }
}

/// D1: **a region's current account is a READ** — computed from the flows that actually crossed, party
/// by party (Law 19, F2). Positive is a surplus.
pub fn current_account(region: RegionId, flows: &[Flow]) -> f64 {
    flows
        .iter()
        .filter(|f| f.is_cross_border())
        .filter(|f| matches!(f.entry, Entry::Goods | Entry::Services | Entry::Income))
        .map(|f| {
            if f.to_region == region {
                // Money arriving for goods this region sold.
                f.amount
            } else if f.from_region == region {
                -f.amount
            } else {
                0.0
            }
        })
        .sum()
}

/// D2: **the financial account is the other side** — the net acquisition of foreign claims.
pub fn financial_account(region: RegionId, flows: &[Flow]) -> f64 {
    flows
        .iter()
        .filter(|f| f.is_cross_border() && f.entry == Entry::Claim)
        .map(|f| {
            if f.from_region == region {
                f.amount
            } else if f.to_region == region {
                -f.amount
            } else {
                0.0
            }
        })
        .sum()
}

/// D3, D3.a: **the two sum to zero for each region, as a CONSEQUENCE of every transaction having two
/// sides** — and **a residual that has to be plugged is a transaction that lost a leg.** `None` when it
/// balances; the residual when it does not, and nothing repairs it.
pub fn imbalance(region: RegionId, flows: &[Flow], terms: usize) -> Option<f64> {
    let current = current_account(region, flows);
    let financial = financial_account(region, flows);
    let residual = current - financial;
    if residual.abs() <= crate::num::dust(terms, &[current, financial]) {
        return None;
    }
    Some(residual)
}

/// D6: **summing all regions gives zero in every category, BECAUSE THE WORLD IS CLOSED.** The check
/// that actually catches a missing leg — every flow that left somewhere arrived somewhere.
pub fn world_closes(regions: &[RegionId], flows: &[Flow], terms: usize) -> Option<f64> {
    let total: f64 = regions.iter().map(|r| current_account(*r, flows)).sum();
    if total.abs() <= crate::num::dust(terms, &[total.abs(), flows.iter().map(|f| f.amount.abs()).sum()]) {
        return None;
    }
    Some(total)
}

/// B4: **one region's exports are another's imports, UNIT FOR UNIT AND PARTY TO PARTY.** The read that
/// says so, from the flows themselves rather than from two aggregates.
pub fn exports_to(from: RegionId, to: RegionId, flows: &[Flow]) -> f64 {
    flows
        .iter()
        .filter(|f| f.from_region == from && f.to_region == to && f.entry == Entry::Goods)
        .map(|f| f.amount)
        .sum()
}

/// A2.a, C2, C2.a: **whoever is not in the invoice currency has an FX exposure, which it can hedge or
/// CARRY** — and for a foreign-currency borrower that is **a real solvency risk a rate move triggers,
/// not a translation adjustment.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Exposure {
    pub who: PartyId,
    pub in_currency: CurrencyCode,
    pub amount: f64,
    /// A2.a: hedged, or carried. Both are decisions with costs.
    pub hedged: f64,
}

impl Exposure {
    pub fn carried(&self) -> f64 {
        self.amount - self.hedged
    }

    /// C2.a: what a rate move does to what this party owes, in its own money. A solvency event, not a
    /// translation line.
    pub fn on_a_rate_move(&self, rate_before: f64, rate_now: f64) -> f64 {
        self.carried() * (rate_now - rate_before)
    }
}

/// B3, B3.a: **the price the buyer pays in its own money depends on the exchange rate**, so a rate move
/// changes what it buys — **the expenditure-switching channel, and it must be a CONSEQUENCE of the
/// buyer's own decision** rather than an elasticity applied to a series.
pub fn in_buyers_money(price_abroad: f64, rate: f64) -> f64 {
    price_abroad * rate
}

/// C3: **a bank funds in one currency and lends in another, and it must SQUARE that** — the gap is a
/// real position, and leaving it open is a decision.
pub fn currency_gap(funded_in: f64, lent_in: f64) -> f64 {
    lent_in - funded_in
}

/// D4, D4.a: **a deficit region must be financed by somebody who CHOOSES to finance it, at a price** —
/// and a persistent one-way flow financed by the banking system is a real phenomenon. `None` is nobody
/// choosing to, which is the outcome that makes the price mean something.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Finances {
    pub who: PartyId,
    pub amount: f64,
    pub at_price: f64,
}

pub fn financed_by(deficit: f64, offers: &[(PartyId, f64, f64)], region_will_pay: f64) -> Option<Vec<Finances>> {
    if deficit >= 0.0 {
        return None;
    }
    let needs = -deficit;
    let mut ordered: Vec<&(PartyId, f64, f64)> = offers.iter().collect();
    ordered.sort_by(|a, b| a.2.total_cmp(&b.2));
    let mut taken = Vec::new();
    let mut raised = 0.0;
    for (who, size, price) in ordered {
        if raised >= needs || *price > region_will_pay {
            break;
        }
        let wants = needs - raised;
        let amount = if *size < wants { *size } else { wants };
        taken.push(Finances { who: *who, amount, at_price: *price });
        raised += amount;
    }
    if raised < needs {
        // Nobody would finance the rest at a price the region would pay. A real outcome.
        return None;
    }
    Some(taken)
}

/// D5: **the accumulated position is a stock of claims held by NAMED parties that revalues** when the
/// rate moves — never a regional aggregate that moves on its own (F2).
pub fn revalued(held: &[(PartyId, f64)], rate_before: f64, rate_now: f64) -> Vec<(PartyId, f64)> {
    held.iter()
        .map(|(who, amount)| (*who, amount * (rate_now - rate_before)))
        .collect()
}

/// E3: **a default must reach foreign holders IN PROPORTION, like any other.** No domestic preference
/// anywhere: the walk is over holders, and where they live does not enter it.
pub fn default_reaches(loss: f64, holders: &[(PartyId, RegionId, f64)]) -> Vec<(PartyId, f64)> {
    let units: f64 = holders.iter().map(|(_, _, u)| u).sum();
    if units <= 0.0 {
        return Vec::new();
    }
    holders
        .iter()
        .map(|(who, _, u)| (*who, loss * u / units))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn home() -> RegionId {
        RegionId::at(1)
    }

    fn abroad() -> RegionId {
        RegionId::at(2)
    }

    fn flow(from: u32, from_r: RegionId, to: u32, to_r: RegionId, amount: f64, entry: Entry) -> Flow {
        Flow {
            from: party(from),
            from_region: from_r,
            to: party(to),
            to_region: to_r,
            amount,
            invoiced_in: CurrencyCode::at(1),
            entry,
        }
    }

    fn world() -> Vec<Flow> {
        vec![
            // Home sells goods abroad, and is paid.
            flow(20, abroad(), 10, home(), 900.0, Entry::Goods),
            // Home buys goods from abroad.
            flow(10, home(), 20, abroad(), 500.0, Entry::Goods),
            // And pays a coupon to a foreign holder.
            flow(10, home(), 21, abroad(), 100.0, Entry::Income),
            // Home acquires a foreign claim with the difference.
            flow(10, home(), 20, abroad(), 300.0, Entry::Claim),
        ]
    }

    #[test]
    fn the_current_account_is_read_from_the_flows_party_by_party() {
        // D1, F2: no netting into a regional aggregate — the parties are named on every leg.
        assert_eq!(current_account(home(), &world()), 300.0);
        assert_eq!(current_account(abroad(), &world()), -300.0);
    }

    #[test]
    fn the_two_accounts_sum_to_zero_because_every_transaction_has_two_sides() {
        // D3, D3.a: a residual that has to be plugged is a transaction that lost a leg.
        assert!(imbalance(home(), &world(), 4).is_none());
        // Drop the financing leg and the residual appears — which is the missing leg, visible.
        let lost_a_leg: Vec<Flow> = world().into_iter().filter(|f| f.entry != Entry::Claim).collect();
        assert_eq!(imbalance(home(), &lost_a_leg, 4), Some(300.0));
    }

    #[test]
    fn summing_all_regions_gives_zero_because_the_world_is_closed() {
        // D6: the check that actually catches a missing leg.
        assert!(world_closes(&[home(), abroad()], &world(), 4).is_none());
        // A flow that arrived in a region nobody counted leaves the world open.
        let mut leaking = world();
        leaking.push(flow(10, home(), 30, RegionId::at(3), 250.0, Entry::Goods));
        assert_eq!(world_closes(&[home(), abroad()], &leaking, 4), Some(-250.0));
    }

    #[test]
    fn one_regions_exports_are_anothers_imports_unit_for_unit() {
        // B4: read from the flows themselves, not from two aggregates that might disagree.
        assert_eq!(exports_to(abroad(), home(), &world()), 900.0);
        assert_eq!(exports_to(home(), abroad(), &world()), 500.0);
    }

    #[test]
    fn whoever_is_not_in_the_invoice_currency_carries_an_exposure_that_a_rate_move_triggers() {
        // A2.a, C2, C2.a: a real solvency risk, not a translation adjustment.
        let open = Exposure { who: party(10), in_currency: CurrencyCode::at(2), amount: 1_000.0, hedged: 0.0 };
        let covered = Exposure { hedged: 900.0, ..open };
        assert_eq!(open.carried(), 1_000.0);
        assert_eq!(covered.carried(), 100.0);
        assert!(open.on_a_rate_move(1.0, 1.2).abs() > covered.on_a_rate_move(1.0, 1.2).abs());
    }

    #[test]
    fn a_rate_move_changes_what_the_buyer_pays_in_its_own_money() {
        // B3, B3.a: the expenditure-switching channel, as a consequence of the buyer's own decision.
        assert!(in_buyers_money(100.0, 1.4) > in_buyers_money(100.0, 1.1));
    }

    #[test]
    fn a_bank_that_funds_in_one_money_and_lends_in_another_has_a_gap_to_square() {
        // C3: the gap is a real position, and leaving it open is a decision.
        assert_eq!(currency_gap(800.0, 1_000.0), 200.0);
        assert_eq!(currency_gap(1_000.0, 1_000.0), 0.0);
    }

    #[test]
    fn a_deficit_is_financed_by_somebody_who_chooses_to_at_a_price_or_not_at_all() {
        // D4, D4.a: the outcome that makes the price mean something.
        let offers = [(party(40), 200.0, 0.03), (party(41), 400.0, 0.05)];
        let financed = financed_by(-500.0, &offers, 0.06).unwrap();
        assert_eq!(financed[0].who, party(40));
        assert_eq!(financed[1].amount, 300.0);
        // Nobody will finance it at a price the region will pay.
        assert!(financed_by(-500.0, &offers, 0.01).is_none());
        // And a surplus region is not being financed at all.
        assert!(financed_by(300.0, &offers, 0.06).is_none());
    }

    #[test]
    fn the_accumulated_position_revalues_for_the_named_parties_that_hold_it() {
        // D5, F2: never a regional aggregate that moves on its own.
        let held = [(party(10), 1_000.0), (party(11), 400.0)];
        let moved = revalued(&held, 1.0, 1.1);
        assert_eq!(moved[0].0, party(10));
        assert!((moved[0].1 - 100.0).abs() <= crate::num::dust(2, &[1_000.0, 100.0]));
    }

    #[test]
    fn a_default_reaches_foreign_holders_in_proportion_like_any_other() {
        // E3: where a holder lives does not enter the walk.
        let holders = [(party(10), home(), 600.0), (party(20), abroad(), 400.0)];
        let hit = default_reaches(1_000.0, &holders);
        assert_eq!(hit[0], (party(10), 600.0));
        assert_eq!(hit[1], (party(20), 400.0));
        assert!(default_reaches(1_000.0, &[]).is_empty());
    }

    #[test]
    fn a_purely_domestic_flow_is_not_cross_border_at_all() {
        // F1: no region that is a closed box — and a flow inside one is not this system's business.
        let domestic = flow(10, home(), 11, home(), 100.0, Entry::Goods);
        assert!(!domestic.is_cross_border());
        assert_eq!(current_account(home(), &[domestic]), 0.0);
    }
}
