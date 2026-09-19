//! COMMODITIES, SPOT AND FUTURES: a grade at a location, an inventory that carries across periods,
//! and a curve whose two states are not symmetric.
//!
//! @spec 21 A1 · 21 A1.a · 21 A2 · 21 A3 · 21 A4 · 21 B1 · 21 B1.a · 21 B2 · 21 B2.a · 21 B3 ·
//! @spec 21 B4 · 21 C1 · 21 C2 · 21 C3 · 21 C4 · 21 D1 · 21 D2 · 21 D2.a · 21 D3 · 21 D4 · 21 D5 ·
//! @spec 21 F1 · 21 F2 · 21 F3 · 20 A1.a · 20 A1.b · 20 A1.c · 20 A1.d · 20 A2 · 20 A3 · 20 A4 ·
//! @spec 20 B1 · 20 B2 · 20 B3 · 20 B3.a · 20 B4 · 20 C1 · 20 C1.a · 20 C1.b · 20 C2 · 20 C3 ·
//! @spec 20 C4 · 20 C4.a · 20 D1 · 20 D2 · 20 D3 · 20 D4 · 20 E1 · 20 E2 · 20 E3 · Law 3, Law 6,
//! @spec Law 8, Law 19 · Appendix B

use crate::ids::{PartyId, RegionId};

/// 21 A1, A1.a: a standardised, fungible unit — a grade, at a location, in a quantity unit. Location
/// is part of the IDENTITY: the same grade in two places is two prices, and the difference is
/// transport.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Grade {
    pub what: u32,
    pub at: RegionId,
}

/// 21 A4: the stock is finite and observable — inventory is a real number held by real parties, at
/// real locations.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Inventory {
    pub held_by: PartyId,
    pub of: Grade,
    pub units: f64,
}

impl Inventory {
    /// 21 D2.a: it carries across periods, moved by production and consumption. 21 F1, F2: units
    /// cannot be conjured and the stock never goes negative — `None` is the refusal, and it is
    /// arithmetic impossibility rather than a clamp.
    pub fn draw(&self, units: f64) -> Option<Inventory> {
        if units > self.units {
            return None;
        }
        Some(Inventory { units: self.units - units, ..*self })
    }

    pub fn add(&self, units: f64) -> Inventory {
        assert!(units >= 0.0, "21 F2: adding negative units to a stock is a withdrawal wearing a disguise");
        Inventory { units: self.units + units, ..*self }
    }
}

/// 21 B1, B1.a: a producer produces at a cost, and it produces because the price covers it — and
/// costs differ across producers, so the supply schedule is a CONSEQUENCE of the cost distribution,
/// never a curve written down.
#[derive(Clone, Copy, Debug)]
pub struct Producer {
    pub who: PartyId,
    pub cost_per_unit: f64,
    /// 21 B2: capacity is fixed in the short run and changes only through investment, which takes
    /// time — which is why supply is inelastic on the horizon that matters.
    pub capacity: f64,
}

/// The supply that shows up at a price: every producer whose cost it covers, at its own capacity. A
/// walk over the cost distribution — there is no schedule to look up.
pub fn supply_at(price: f64, producers: &[Producer]) -> f64 {
    producers
        .iter()
        .filter(|p| price >= p.cost_per_unit)
        .map(|p| p.capacity)
        .sum()
}

/// 21 B3: a disruption is a real loss of UNITS at the point they would have been made — not a
/// multiplier on a price.
pub fn disrupted(p: &Producer, units_lost: f64) -> Producer {
    assert!(units_lost <= p.capacity, "21 B3: a disruption cannot lose more units than the line could make");
    Producer { capacity: p.capacity - units_lost, ..*p }
}

/// 21 D1, D2: the price clears, per grade and location, and inventory is the buffer: when demand
/// exceeds production stocks fall, and when stocks approach zero the price has nothing left to
/// ration with.
#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    pub price: Option<f64>,
    pub traded: f64,
    pub from_inventory: f64,
    pub unmet: f64,
}

/// 21 C4, D2: with both sides inelastic, small imbalances produce large price moves — a consequence
/// to be measured, not a volatility parameter. The price here is the marginal producer's cost when
/// production covers demand, and it is what the last unit of inventory fetched when it does not.
pub fn clearing(demand: f64, producers: &[Producer], stock: f64, bids: &[f64]) -> Cleared {
    let mut ordered: Vec<&Producer> = producers.iter().collect();
    ordered.sort_by(|a, b| a.cost_per_unit.total_cmp(&b.cost_per_unit));

    let mut left = demand;
    let mut price = None;
    for p in &ordered {
        if left <= 0.0 {
            break;
        }
        let taken = if p.capacity < left { p.capacity } else { left };
        if taken > 0.0 {
            price = Some(p.cost_per_unit);
            left -= taken;
        }
    }
    let produced = demand - left;
    // What production did not cover comes out of the stock, and when the stock runs out the
    // remaining demand is simply unmet — there is nothing left to ration with.
    let from_inventory = if stock < left { stock } else { left };
    let unmet = left - from_inventory;
    if from_inventory > 0.0 {
        // The bids that reached the last unit of stock are what it fetched — scarcity prices it,
        // not a formula.
        let mut sorted: Vec<f64> = bids.to_vec();
        sorted.sort_by(|a, b| b.total_cmp(a));
        let at = (produced + from_inventory) as usize;
        price = sorted.get(at.saturating_sub(1)).copied().or(price);
    }
    Cleared { price, traded: produced + from_inventory, from_inventory, unmet }
}

/// 21 D3: storage costs money and the cost is paid to somebody who owns the storage.
pub fn storage_fee(units: f64, per_unit: f64, to: PartyId) -> (PartyId, f64) {
    (to, units * per_unit)
}

/// 21 D5: produced plus opening inventory equals consumed plus closing inventory, per commodity and
/// location, exactly. A VERIFY on Law 7's derived dust; `None` when it holds, and the discrepancy
/// when it does not — units that appeared or vanished.
pub fn units_balance(produced: f64, opening: f64, consumed: f64, closing: f64, terms: usize) -> Option<f64> {
    let off = (produced + opening) - (consumed + closing);
    if off.abs() <= crate::num::dust(terms, &[produced, opening, consumed, closing]) {
        return None;
    }
    Some(off)
}

/// 20 A1.a, A1.d, A2: a stated grade at a stated delivery location, a fixed quantity per contract —
/// so size is in CONTRACTS, not money — and standardisation means the delivery terms are part of the
/// instrument.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Future {
    pub on: Grade,
    pub units_per_contract: f64,
    pub contracts: f64,
    /// 20 A1.c: the futures price, CLEARED.
    pub price: f64,
    pub expires_in_years: f64,
    /// 20 D2: physical delivery must be possible for at least some participants, or convergence has
    /// no mechanism behind it.
    pub deliverable: bool,
}

/// 20 E1: no futures price without a physical market underneath it. A futures curve on a commodity
/// that is never actually traded prices itself.
pub fn may_list(spot_traded: bool, deliverable: bool) -> bool {
    spot_traded && deliverable
}

/// 20 C1, C1.a: contango is bounded above by what it costs to buy, store and finance — past that the
/// arbitrageur takes it. This is the level at which that trade opens, which is a consequence of real
/// costs and not a ceiling anybody imposed.
pub fn full_carry(spot: f64, storage_per_year: f64, financing: f64, years: f64) -> f64 {
    spot * (1.0 + financing * years) + storage_per_year * years
}

/// 20 C1.b: backwardation is unbounded below, because you cannot store a shortage. The asymmetry
/// stated as a read: how far a curve sits above full carry (positive: the arbitrage is open) or
/// below spot (negative: physical tightness, and nothing bounds it).
pub fn against_carry(futures: f64, spot: f64, storage_per_year: f64, financing: f64, years: f64) -> f64 {
    futures - full_carry(spot, storage_per_year, financing, years)
}

/// 20 B4: the arbitrageur between the future and the physical can only act if it can actually store
/// and finance. `None` without storage or funding — and then the curve has nothing tying it to the
/// physical world.
pub fn arbitrages(over_carry: f64, storage_free: f64, can_finance: f64) -> Option<f64> {
    if over_carry <= 0.0 {
        // Below full carry there is nothing to take; below SPOT there is nothing to take either,
        // because you cannot store a shortage.
        return None;
    }
    let room = if storage_free < can_finance { storage_free } else { can_finance };
    if room <= 0.0 {
        return None;
    }
    Some(room)
}

/// 20 C2, C3: the curve carries information about physical tightness, and inventory is the state
/// variable it reads. Low inventories imply backwardation as a CONSEQUENCE of C1.b, never as a rule
/// — so this is a read of the two, published together.
pub fn tightness(stock: f64, consumed_per_period: f64) -> Option<f64> {
    if consumed_per_period <= 0.0 {
        return None;
    }
    Some(stock / consumed_per_period)
}

/// 20 C4, C4.a: convergence is a consequence of deliverability, not an enforced boundary condition.
/// Nothing drives the price; this says whether the mechanism exists at all.
pub fn can_converge(f: &Future) -> bool {
    f.deliverable
}

/// 20 B3.a, E3: the roll has a cost or a gain determined by the curve — which is most of an
/// investor's return and is not a fee — and no roll is free. It lands in the roller's P&L.
pub fn roll(out_of: f64, into: f64, contracts: f64, units_per_contract: f64) -> f64 {
    (out_of - into) * contracts * units_per_contract
}

/// 20 D3: a party that cannot take delivery must close or roll before expiry — a real forced trade
/// at a known time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AtExpiry {
    /// 20 D1: it delivers.
    Delivers,
    /// 20 D4: or cash-settles against an observed, CLEARED spot price.
    CashSettles,
    /// 20 D3: or it must have closed or rolled before now.
    MustCloseOrRoll,
}

pub fn at_expiry(f: &Future, can_take_delivery: bool, spot_cleared: Option<f64>) -> AtExpiry {
    if can_take_delivery && f.deliverable {
        return AtExpiry::Delivers;
    }
    match spot_cleared {
        Some(_) => AtExpiry::CashSettles,
        // Cash settlement is against an OBSERVED price. With no print there is nothing to settle
        // against, and the position has to have been closed.
        None => AtExpiry::MustCloseOrRoll,
    }
}

/// 20 E2: no unlimited open interest against finite deliverable supply without the squeeze that
/// implies. How many times the deliverable stock the open interest is — a read, and a large one is
/// the squeeze being visible rather than prevented.
pub fn open_interest_against_supply(contracts: f64, units_per_contract: f64, deliverable_stock: f64) -> Option<f64> {
    if deliverable_stock <= 0.0 {
        return None;
    }
    Some(contracts * units_per_contract / deliverable_stock)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn here() -> Grade {
        Grade { what: 1, at: RegionId::at(1) }
    }

    fn there() -> Grade {
        Grade { what: 1, at: RegionId::at(2) }
    }

    fn producers() -> Vec<Producer> {
        vec![
            Producer { who: party(1), cost_per_unit: 40.0, capacity: 500.0 },
            Producer { who: party(2), cost_per_unit: 55.0, capacity: 300.0 },
            Producer { who: party(3), cost_per_unit: 80.0, capacity: 400.0 },
        ]
    }

    #[test]
    fn the_same_grade_in_two_places_is_two_prices() {
        // 21 A1.a: location is part of the identity, and the difference is transport.
        assert_ne!(here(), there());
    }

    #[test]
    fn the_supply_schedule_is_a_consequence_of_the_cost_distribution() {
        // 21 B1.a: never a curve written down. A higher price brings out the dearer producer.
        assert_eq!(supply_at(45.0, &producers()), 500.0);
        assert_eq!(supply_at(60.0, &producers()), 800.0);
        assert_eq!(supply_at(90.0, &producers()), 1_200.0);
        assert_eq!(supply_at(20.0, &producers()), 0.0);
    }

    #[test]
    fn a_disruption_is_a_real_loss_of_units_and_not_a_multiplier_on_a_price() {
        // 21 B3.
        let hit = disrupted(&producers()[0], 200.0);
        assert_eq!(hit.capacity, 300.0);
        assert_eq!(hit.cost_per_unit, 40.0);
    }

    #[test]
    fn inventory_never_goes_negative_and_units_cannot_be_conjured() {
        // 21 F1, F2, Law 6: the refusal is arithmetic impossibility, not a clamp.
        let stock = Inventory { held_by: party(5), of: here(), units: 100.0 };
        assert_eq!(stock.draw(40.0).unwrap().units, 60.0);
        assert!(stock.draw(140.0).is_none());
        assert_eq!(stock.add(50.0).units, 150.0);
    }

    #[test]
    fn inventory_is_the_buffer_and_when_it_runs_out_demand_goes_unmet() {
        // 21 D2: when stocks approach zero the price has nothing left to ration with.
        let c = clearing(1_000.0, &producers(), 0.0, &[]);
        assert_eq!(c.traded, 1_000.0);
        assert_eq!(c.unmet, 0.0);
        assert_eq!(c.price, Some(80.0));

        let short = clearing(1_500.0, &producers(), 100.0, &[200.0, 190.0, 180.0]);
        assert_eq!(short.from_inventory, 100.0);
        assert_eq!(short.unmet, 200.0);
        assert!(short.price.is_some());
    }

    #[test]
    fn small_imbalances_produce_large_price_moves_when_both_sides_are_inelastic() {
        // 21 C4: a consequence to be measured, not a volatility parameter. One more unit of demand
        // than the cheap producers can make reaches the dear one, and the price jumps 45%.
        let within = clearing(800.0, &producers(), 0.0, &[]).price.unwrap();
        let over = clearing(801.0, &producers(), 0.0, &[]).price.unwrap();
        assert_eq!(within, 55.0);
        assert_eq!(over, 80.0);
    }

    #[test]
    fn produced_plus_opening_equals_consumed_plus_closing() {
        // 21 D5: exactly, per commodity and location — and the discrepancy is units that appeared
        // or vanished.
        assert!(units_balance(1_000.0, 200.0, 900.0, 300.0, 4).is_none());
        assert_eq!(units_balance(1_000.0, 200.0, 900.0, 250.0, 4), Some(50.0));
    }

    #[test]
    fn storage_costs_money_and_it_is_paid_to_somebody() {
        // 21 D3, Law 5.
        let (owner, paid) = storage_fee(100.0, 0.4, party(70));
        assert_eq!(owner, party(70));
        assert_eq!(paid, 40.0);
    }

    #[test]
    fn contango_is_bounded_by_the_arbitrage_and_backwardation_is_not_bounded_at_all() {
        // 20 C1.a, C1.b: the asymmetry is real, and it is why the two states are not symmetric. The
        // ceiling is a trade somebody performs; there is no floor anywhere in this module.
        let spot = 100.0;
        let carry = full_carry(spot, 4.0, 0.05, 1.0);
        assert!(carry > spot);
        // Above full carry the arbitrage is open, and somebody with storage and funding takes it.
        assert!(arbitrages(against_carry(carry + 5.0, spot, 4.0, 0.05, 1.0), 10_000.0, 10_000.0).is_some());
        // Far below spot there is nothing to take: you cannot store a shortage.
        assert!(arbitrages(against_carry(60.0, spot, 4.0, 0.05, 1.0), 10_000.0, 10_000.0).is_none());
    }

    #[test]
    fn without_storage_or_funding_there_is_no_arbitrageur_and_the_curve_floats_free() {
        // 20 B4: without a storable stock there is no such participant, and the curve has nothing
        // tying it to the physical world.
        let over = against_carry(120.0, 100.0, 4.0, 0.05, 1.0);
        assert!(arbitrages(over, 0.0, 10_000.0).is_none());
        assert!(arbitrages(over, 10_000.0, 0.0).is_none());
    }

    #[test]
    fn the_curve_reads_inventory_and_low_stocks_are_a_consequence_not_a_rule() {
        // 20 C2, C3: inventory is the state variable the curve reads.
        let ample = tightness(5_000.0, 500.0).unwrap();
        let scarce = tightness(200.0, 500.0).unwrap();
        assert!(ample > scarce);
        assert!(tightness(200.0, 0.0).is_none());
    }

    #[test]
    fn convergence_is_a_consequence_of_deliverability_and_is_never_enforced() {
        // 20 C4.a, D2: nothing here drives the price to spot. What this says is whether the
        // mechanism that would exists at all.
        let real = Future { on: here(), units_per_contract: 100.0, contracts: 10.0, price: 104.0, expires_in_years: 0.25, deliverable: true };
        assert!(can_converge(&real));
        let paper = Future { deliverable: false, ..real };
        assert!(!can_converge(&paper));
        // 20 E1: and a future on a commodity nobody trades physically prices itself.
        assert!(may_list(true, true));
        assert!(!may_list(false, true));
        assert!(!may_list(true, false));
    }

    #[test]
    fn no_roll_is_free_and_the_curve_decides_whether_it_costs_or_pays() {
        // 20 B3.a, E3: most of an investor's return, and it lands in its P&L.
        let in_contango = roll(100.0, 104.0, 10.0, 100.0);
        let in_backwardation = roll(100.0, 96.0, 10.0, 100.0);
        assert!(in_contango < 0.0);
        assert!(in_backwardation > 0.0);
    }

    #[test]
    fn a_party_that_cannot_take_delivery_must_close_or_roll() {
        // 20 D1, D3, D4: all three are real, and cash settlement needs an OBSERVED price.
        let f = Future { on: here(), units_per_contract: 100.0, contracts: 10.0, price: 104.0, expires_in_years: 0.0, deliverable: true };
        assert_eq!(at_expiry(&f, true, Some(101.0)), AtExpiry::Delivers);
        assert_eq!(at_expiry(&f, false, Some(101.0)), AtExpiry::CashSettles);
        assert_eq!(at_expiry(&f, false, None), AtExpiry::MustCloseOrRoll);
    }

    #[test]
    fn open_interest_against_finite_deliverable_supply_is_visible() {
        // 20 E2: no unlimited open interest without the squeeze that implies — so the ratio is a
        // read, and a large one is the squeeze being visible rather than prevented.
        assert_eq!(open_interest_against_supply(100.0, 100.0, 10_000.0), Some(1.0));
        assert_eq!(open_interest_against_supply(400.0, 100.0, 10_000.0), Some(4.0));
        assert!(open_interest_against_supply(100.0, 100.0, 0.0).is_none());
    }

    #[test]
    #[should_panic(expected = "more units than the line could make")]
    fn a_disruption_cannot_lose_more_than_the_line_could_make() {
        disrupted(&producers()[0], 900.0);
    }
}
