//! CROSS-BORDER: two named parties in different regions, and the world is closed — so summing all
//! regions gives zero in every category.
//!
//! @spec 43 A1 · 43 A2 · 43 A2.a · 43 A3 · 43 A4 · 43 B1 · 43 B2 · 43 B3 · 43 B3.a · 43 B4 · 43 C1 ·
//! @spec 43 C2 · 43 C2.a · 43 C3 · 43 C4 · 43 C5 · 43 D1 · 43 D2 · 43 D3 · 43 D3.a · 43 D4 · 43 D4.a ·
//! @spec 43 D5 · 43 D6 · 43 E1 · 43 E2 · 43 E3 · 43 E4 · 43 F1 · 43 F2 · XI-12 · Law 5, Law 19

use crate::ids::{CurrencyCode, PartyId, RegionId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};

/// Two named parties in DIFFERENT regions, in one of two currencies or a third — and the
/// counterparty is foreign, which is a real credit and legal difference.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Flow {
    pub from: PartyId,
    pub from_region: RegionId,
    pub to: PartyId,
    pub to_region: RegionId,
    pub amount: f64,
    /// Somebody decided which money this is in, and A2.a: whoever is not in it has an exposure.
    pub invoiced_in: CurrencyCode,
    pub entry: Entry,
}

/// The current account is goods and services plus INCOME flows — C5: coupons, dividends, profits —
/// and D2: the financial account is the other side, the net acquisition of foreign claims.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Entry {
    /// A firm buys from or sells to a firm in another region, and the goods MOVE.
    Goods,
    Services,
    /// Income across the border — coupons, dividends, profits.
    Income,
    /// A claim acquired or given up — a foreign asset, an issue in a foreign currency, a direct
    /// investment that buys a firm outright.
    Claim,
}

impl Flow {
    pub fn is_cross_border(&self) -> bool {
        self.from_region != self.to_region
    }
}

/// A region's current account is a READ — computed from the flows that actually crossed, party by
/// party.
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

/// The financial account is the other side — the net acquisition of foreign claims.
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

/// The two sum to zero for each region, as a CONSEQUENCE of every transaction having two sides — and
/// a residual that has to be plugged is a transaction that lost a leg.
pub fn imbalance(region: RegionId, flows: &[Flow], terms: usize) -> Option<f64> {
    let current = current_account(region, flows);
    let financial = financial_account(region, flows);
    let residual = current - financial;
    if residual.abs() <= crate::num::dust(terms, &[current, financial]) {
        return None;
    }
    Some(residual)
}

/// Summing all regions gives zero in every category, BECAUSE THE WORLD IS CLOSED.
pub fn world_closes(regions: &[RegionId], flows: &[Flow], terms: usize) -> Option<f64> {
    let total: f64 = regions.iter().map(|r| current_account(*r, flows)).sum();
    if total.abs() <= crate::num::dust(terms, &[total.abs(), flows.iter().map(|f| f.amount.abs()).sum()]) {
        return None;
    }
    Some(total)
}

/// One region's exports are another's imports, UNIT FOR UNIT AND PARTY TO PARTY.
pub fn exports_to(from: RegionId, to: RegionId, flows: &[Flow]) -> f64 {
    flows
        .iter()
        .filter(|f| f.from_region == from && f.to_region == to && f.entry == Entry::Goods)
        .map(|f| f.amount)
        .sum()
}

/// Whoever is not in the invoice currency has an FX exposure, which it can hedge or CARRY — and for
/// a foreign-currency borrower that is a real solvency risk a rate move triggers, not a translation
/// adjustment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Exposure {
    pub who: PartyId,
    pub in_currency: CurrencyCode,
    pub amount: f64,
    pub hedged: f64,
}

impl Exposure {
    pub fn carried(&self) -> f64 {
        self.amount - self.hedged
    }

    /// What a rate move does to what this party owes, in its own money.
    pub fn on_a_rate_move(&self, rate_before: f64, rate_now: f64) -> f64 {
        self.carried() * (rate_now - rate_before)
    }
}

/// The price the buyer pays in its own money depends on the exchange rate, so a rate move changes
/// what it buys — the expenditure-switching channel, and it must be a CONSEQUENCE of the buyer's own
/// decision rather than an elasticity applied to a series.
pub fn in_buyers_money(price_abroad: f64, rate: f64) -> f64 {
    price_abroad * rate
}

/// A bank funds in one currency and lends in another, and it must SQUARE that — the gap is a real
/// position, and leaving it open is a decision.
pub fn currency_gap(funded_in: f64, lent_in: f64) -> f64 {
    lent_in - funded_in
}

/// A deficit region must be financed by somebody who CHOOSES to finance it, at a price — and a
/// persistent one-way flow financed by the banking system is a real phenomenon.
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
        // Nobody would finance the rest at a price the region would pay.
        return None;
    }
    Some(taken)
}

/// The accumulated position is a stock of claims held by NAMED parties that revalues when the rate
/// moves — never a regional aggregate that moves on its own.
pub fn revalued(held: &[(PartyId, f64)], rate_before: f64, rate_now: f64) -> Vec<(PartyId, f64)> {
    held.iter()
        .map(|(who, amount)| (*who, amount * (rate_now - rate_before)))
        .collect()
}

/// A default must reach foreign holders IN PROPORTION, like any other.
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


/// A REGION'S ACCOUNTS ARE A READ OF WHAT ACTUALLY CROSSED.
pub struct CrossBorder {
    pub kind: u32,
    pub at_current: u32,
    pub at_financial: u32,
}

impl Mechanism for CrossBorder {
    fn run(&self, ctx: &mut MechanismContext<'_>) {

        // The flows that actually crossed, this period, off the wire's own legs.
        let mut flows: Vec<Flow> = Vec::new();
        for n in ctx.wire().in_period(ctx.period()) {
            if ctx.wire().outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in ctx.wire().legs_of(n) {
                let crate::ledger::Leg::Money { from, to, instrument, amount, receipt } = *leg else {
                    continue;
                };
                if from == to || !ctx.parties().alive(from) || !ctx.parties().alive(to) {
                    continue;
                }
                let from_region = ctx.parties().region_of(from);
                let to_region = ctx.parties().region_of(to);
                if from_region == to_region {
                    continue;
                }
                // What the payment was FOR decides which account it lands in.
                let entry = match receipt {
                    crate::ledger::Receipt::Sale => Entry::Goods,
                    crate::ledger::Receipt::Wage | crate::ledger::Receipt::Tax => Entry::Services,
                    crate::ledger::Receipt::Interest | crate::ledger::Receipt::Dividend => Entry::Income,
                    crate::ledger::Receipt::Principal | crate::ledger::Receipt::Transfer => Entry::Claim,
                };
                flows.push(Flow {
                    from,
                    from_region,
                    to,
                    to_region,
                    amount: amount.get(),
                    // What money this is, read off the instrument that IS it.
                    invoiced_in: ctx.instruments().ccy_of(instrument),
                    entry,
                });
            }
        }
        if flows.is_empty() {
            return;
        }

        let mut places: Vec<u32> = flows.iter().flat_map(|f| [f.from_region.0, f.to_region.0]).collect();
        places.sort_unstable();
        places.dedup();
        let regions: Vec<crate::ids::RegionId> = places.iter().map(|r| crate::ids::RegionId::at(*r)).collect();

        let mut read: Vec<(u32, f64, f64, Option<f64>)> = Vec::new();
        for &at in &regions {
            read.push((
                at.0,
                current_account(at, &flows),
                financial_account(at, &flows),
                // The two are the same flows read twice, so this is the discrepancy and it is
                // REPORTED with a size rather than asserted away (Law 7's dust, not a band).
                imbalance(at, &flows, 2),
            ));
        }
        // And the world closes.
        let closes = world_closes(&regions, &flows, regions.len());

        for (at, current, financial, imbalance) in read {
            let mut data = vec![
                (0, Value::Num(f64::from(at))),
                (self.at_current, Value::Num(current)),
                (self.at_financial, Value::Num(financial)),
            ];
            if let Some(off) = imbalance {
                data.push((1, Value::Num(off)));
            }
            ctx.say(self.kind, &[], &data, true);
        }
        if let Some(off) = closes {
            // A world that does not close has a flow with one side in it somewhere, and that is a
            // finding with a size — never something to net away.
            ctx.say(self.kind, &[], &[(1, Value::Num(off))], true);
        }
    }
}

// Twenty-six tests here built a central bank, a bank and two customers to ask what a module
// already answers over values, or what the audit answers over the real world.
//
// The schedules — what falls due is paid to whoever holds the line, pro rata, an issuer holding its
// own line owes nothing, a short issuer fails the whole coupon — are the wire's, and the Flows
// family measures two-sidedness over 1.9M events a period. The waterfall is `estate::waterfall`'s
// and has eight tests of its own; the pool's pro rata is `funds::pro_rata`'s with nineteen; what a
// line makes and what it cost are `recipe`'s and `goods`' with thirty-three between them; the
// outlook that moves by a party's own memory is `expectations`' with six; the wage on a standing
// engagement is `employment`'s with fourteen.
//
// None of those needs a world, and this file is going: 0m2 moves every impl here into its system's
// module, where its arithmetic already lives.

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
        // No netting into a regional aggregate — the parties are named on every leg.
        assert_eq!(current_account(home(), &world()), 300.0);
        assert_eq!(current_account(abroad(), &world()), -300.0);
    }

    #[test]
    fn the_two_accounts_sum_to_zero_because_every_transaction_has_two_sides() {
        // A residual that has to be plugged is a transaction that lost a leg.
        assert!(imbalance(home(), &world(), 4).is_none());
        // Drop the financing leg and the residual appears — which is the missing leg, visible.
        let lost_a_leg: Vec<Flow> = world().into_iter().filter(|f| f.entry != Entry::Claim).collect();
        assert_eq!(imbalance(home(), &lost_a_leg, 4), Some(300.0));
    }

    #[test]
    fn summing_all_regions_gives_zero_because_the_world_is_closed() {
        // The check that actually catches a missing leg.
        assert!(world_closes(&[home(), abroad()], &world(), 4).is_none());
        // A flow that arrived in a region nobody counted leaves the world open.
        let mut leaking = world();
        leaking.push(flow(10, home(), 30, RegionId::at(3), 250.0, Entry::Goods));
        assert_eq!(world_closes(&[home(), abroad()], &leaking, 4), Some(-250.0));
    }

    #[test]
    fn one_regions_exports_are_anothers_imports_unit_for_unit() {
        // Read from the flows themselves, not from two aggregates that might disagree.
        assert_eq!(exports_to(abroad(), home(), &world()), 900.0);
        assert_eq!(exports_to(home(), abroad(), &world()), 500.0);
    }

    #[test]
    fn whoever_is_not_in_the_invoice_currency_carries_an_exposure_that_a_rate_move_triggers() {
        // A real solvency risk, not a translation adjustment.
        let open = Exposure { who: party(10), in_currency: CurrencyCode::at(2), amount: 1_000.0, hedged: 0.0 };
        let covered = Exposure { hedged: 900.0, ..open };
        assert_eq!(open.carried(), 1_000.0);
        assert_eq!(covered.carried(), 100.0);
        assert!(open.on_a_rate_move(1.0, 1.2).abs() > covered.on_a_rate_move(1.0, 1.2).abs());
    }

    #[test]
    fn a_rate_move_changes_what_the_buyer_pays_in_its_own_money() {
        // The expenditure-switching channel, as a consequence of the buyer's own decision.
        assert!(in_buyers_money(100.0, 1.4) > in_buyers_money(100.0, 1.1));
    }

    #[test]
    fn a_bank_that_funds_in_one_money_and_lends_in_another_has_a_gap_to_square() {
        // The gap is a real position, and leaving it open is a decision.
        assert_eq!(currency_gap(800.0, 1_000.0), 200.0);
        assert_eq!(currency_gap(1_000.0, 1_000.0), 0.0);
    }

    #[test]
    fn a_deficit_is_financed_by_somebody_who_chooses_to_at_a_price_or_not_at_all() {
        // The outcome that makes the price mean something.
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
        // Never a regional aggregate that moves on its own.
        let held = [(party(10), 1_000.0), (party(11), 400.0)];
        let moved = revalued(&held, 1.0, 1.1);
        assert_eq!(moved[0].0, party(10));
        assert!((moved[0].1 - 100.0).abs() <= crate::num::dust(2, &[1_000.0, 100.0]));
    }

    #[test]
    fn a_default_reaches_foreign_holders_in_proportion_like_any_other() {
        // Where a holder lives does not enter the walk.
        let holders = [(party(10), home(), 600.0), (party(20), abroad(), 400.0)];
        let hit = default_reaches(1_000.0, &holders);
        assert_eq!(hit[0], (party(10), 600.0));
        assert_eq!(hit[1], (party(20), 400.0));
        assert!(default_reaches(1_000.0, &[]).is_empty());
    }

    #[test]
    fn a_purely_domestic_flow_is_not_cross_border_at_all() {
        // No region that is a closed box — and a flow inside one is not this system's business.
        let domestic = flow(10, home(), 11, home(), 100.0, Entry::Goods);
        assert!(!domestic.is_cross_border());
        assert_eq!(current_account(home(), &[domestic]), 0.0);
    }
}
