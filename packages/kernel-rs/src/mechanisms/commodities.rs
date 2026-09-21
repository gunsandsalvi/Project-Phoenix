//! COMMODITIES, SPOT AND FUTURES: a grade at a location, an inventory that carries across weeks,
//! and a curve whose two states are not symmetric.
//!
//! @spec 21 A1 · 21 A1.a · 21 A2 · 21 A3 · 21 A4 · 21 B1 · 21 B1.a · 21 B2 · 21 B2.a · 21 B3 ·
//! @spec 21 B4 · 21 C1 · 21 C2 · 21 C3 · 21 C4 · 21 D1 · 21 D2 · 21 D2.a · 21 D3 · 21 D4 · 21 D5 ·
//! @spec 21 F1 · 21 F2 · 21 F3 · 20 A1.a · 20 A1.b · 20 A1.c · 20 A1.d · 20 A2 · 20 A3 · 20 A4 ·
//! @spec 20 B1 · 20 B2 · 20 B3 · 20 B3.a · 20 B4 · 20 C1 · 20 C1.a · 20 C1.b · 20 C2 · 20 C3 ·
//! @spec 20 C4 · 20 C4.a · 20 D1 · 20 D2 · 20 D3 · 20 D4 · 20 E1 · 20 E2 · 20 E3 · Law 3, Law 6,
//! @spec Law 8, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::audit::{Contribution, Family, Sources, Violation, Visit};
use crate::ids::{InstrumentId, MarketId, PartyId, RegionId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use std::collections::HashMap;

/// A standardised, fungible unit — a grade, at a location, in a quantity unit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Grade {
    pub what: u32,
    pub at: RegionId,
}

/// The stock is finite and observable — inventory is a real number held by real parties, at
/// real locations.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Inventory {
    pub held_by: PartyId,
    pub of: Grade,
    pub units: f64,
}

impl Inventory {
    /// It carries across weeks, moved by production and consumption.
    pub fn draw(&self, units: f64) -> Option<Inventory> {
        if units > self.units {
            return None;
        }
        Some(Inventory {
            units: self.units - units,
            ..*self
        })
    }

    pub fn add(&self, units: f64) -> Inventory {
        assert!(
            units >= 0.0,
            "21 F2: adding negative units to a stock is a withdrawal wearing a disguise"
        );
        Inventory {
            units: self.units + units,
            ..*self
        }
    }
}

/// A producer produces at a cost, and it produces because the price covers it — and
/// costs differ across producers, so the supply schedule is a CONSEQUENCE of the cost distribution,
/// never a curve written down.
#[derive(Clone, Copy, Debug)]
pub struct Producer {
    pub who: PartyId,
    pub cost_per_unit: f64,
    /// Capacity is fixed in the short run and changes only through investment, which takes
    /// time — which is why supply is inelastic on the horizon that matters.
    pub capacity: f64,
}

/// The supply that shows up at a price: every producer whose cost it covers, at its own capacity.
pub fn supply_at(price: f64, producers: &[Producer]) -> f64 {
    producers
        .iter()
        .filter(|p| price >= p.cost_per_unit)
        .map(|p| p.capacity)
        .sum()
}

/// A disruption is a real loss of UNITS at the point they would have been made — not a
/// multiplier on a price.
pub fn disrupted(p: &Producer, units_lost: f64) -> Producer {
    assert!(
        units_lost <= p.capacity,
        "21 B3: a disruption cannot lose more units than the line could make"
    );
    Producer {
        capacity: p.capacity - units_lost,
        ..*p
    }
}

/// The price clears, per grade and location, and inventory is the buffer: when demand
/// exceeds production stocks fall, and when stocks approach zero the price has nothing left to
/// ration with.
#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    pub price: Option<f64>,
    pub traded: f64,
    pub from_inventory: f64,
    pub unmet: f64,
}

/// With both sides inelastic, small imbalances produce large price moves — a consequence
/// to be measured, not a volatility parameter.
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
        // The bids that reached the last unit of stock are what it fetched — scarcity prices it, not
        // a formula.
        let mut sorted: Vec<f64> = bids.to_vec();
        sorted.sort_by(|a, b| b.total_cmp(a));
        let at = (produced + from_inventory) as usize;
        price = sorted.get(at.saturating_sub(1)).copied().or(price);
    }
    Cleared {
        price,
        traded: produced + from_inventory,
        from_inventory,
        unmet,
    }
}

/// Storage costs money and the cost is paid to somebody who owns the storage.
pub fn storage_fee(units: f64, per_unit: f64, to: PartyId) -> (PartyId, f64) {
    (to, units * per_unit)
}

/// Produced plus opening inventory equals consumed plus closing inventory, per commodity and
/// location, exactly.
pub fn units_balance(
    produced: f64,
    opening: f64,
    consumed: f64,
    closing: f64,
    terms: usize,
) -> Option<f64> {
    let off = (produced + opening) - (consumed + closing);
    if off.abs() <= crate::num::dust(terms, &[produced, opening, consumed, closing]) {
        return None;
    }
    Some(off)
}

/// A stated grade at a stated delivery location, a fixed quantity per contract —
/// so size is in CONTRACTS, not money — and standardisation means the delivery terms are part of the
/// instrument.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Future {
    pub on: Grade,
    pub units_per_contract: f64,
    pub contracts: f64,
    /// The futures price, CLEARED.
    pub price: f64,
    pub expires_in_years: f64,
    /// Physical delivery must be possible for at least some participants, or convergence has
    /// no mechanism behind it.
    pub deliverable: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FuturesOrder {
    pub book: MarketId,
    pub trader: PartyId,
    pub contracts: f64,
    pub limit: f64,
}

pub fn submit(order: FuturesOrder, declared_book: MarketId) -> Option<FuturesOrder> {
    if order.book != declared_book || order.contracts == 0.0 || !order.limit.is_finite() {
        return None;
    }
    Some(order)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FuturesMargin {
    pub from: PartyId,
    pub to: PartyId,
    pub amount: f64,
}

pub fn variation_margin(
    from: PartyId,
    to: PartyId,
    previous: f64,
    current: f64,
    f: &Future,
) -> Option<FuturesMargin> {
    let amount = (current - previous).abs() * f.contracts.abs() * f.units_per_contract;
    if amount == 0.0 {
        return None;
    }
    Some(FuturesMargin { from, to, amount })
}

/// No futures price without a physical market underneath it.
pub fn may_list(spot_traded: bool, deliverable: bool) -> bool {
    spot_traded && deliverable
}

/// Contango is bounded above by what it costs to buy, store and finance — past that the
/// arbitrageur takes it.
pub fn full_carry(spot: f64, storage_per_year: f64, financing: f64, years: f64) -> f64 {
    spot * (1.0 + financing * years) + storage_per_year * years
}

/// Backwardation is unbounded below, because you cannot store a shortage.
pub fn against_carry(
    futures: f64,
    spot: f64,
    storage_per_year: f64,
    financing: f64,
    years: f64,
) -> f64 {
    futures - full_carry(spot, storage_per_year, financing, years)
}

/// The arbitrageur between the future and the physical can only act if it can actually store
/// and finance.
pub fn arbitrages(over_carry: f64, storage_free: f64, can_finance: f64) -> Option<f64> {
    if over_carry <= 0.0 {
        // Below full carry there is nothing to take; below SPOT there is nothing to take either,
        // because you cannot store a shortage.
        return None;
    }
    let room = if storage_free < can_finance {
        storage_free
    } else {
        can_finance
    };
    if room <= 0.0 {
        return None;
    }
    Some(room)
}

/// The curve carries information about physical tightness, and inventory is the state
/// variable it reads.
pub fn tightness(stock: f64, consumed_per_period: f64) -> Option<f64> {
    if consumed_per_period <= 0.0 {
        return None;
    }
    Some(stock / consumed_per_period)
}

/// Convergence is a consequence of deliverability, not an enforced boundary condition.
pub fn can_converge(f: &Future) -> bool {
    f.deliverable
}

/// The roll has a cost or a gain determined by the curve — which is most of an
/// investor's return and is not a fee — and no roll is free.
pub fn roll(out_of: f64, into: f64, contracts: f64, units_per_contract: f64) -> f64 {
    (out_of - into) * contracts * units_per_contract
}

/// A party that cannot take delivery must close or roll before expiry — a real forced trade
/// at a known time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AtExpiry {
    Delivers,
    /// Or cash-settles against an observed, CLEARED spot price.
    CashSettles,
    /// Or it must have closed or rolled before now.
    MustCloseOrRoll,
}

pub fn at_expiry(f: &Future, can_take_delivery: bool, spot_cleared: Option<f64>) -> AtExpiry {
    if can_take_delivery && f.deliverable {
        return AtExpiry::Delivers;
    }
    match spot_cleared {
        Some(_) => AtExpiry::CashSettles,
        // Cash settlement is against an OBSERVED price.
        None => AtExpiry::MustCloseOrRoll,
    }
}

/// No unlimited open interest against finite deliverable supply without the squeeze that
/// implies.
pub fn open_interest_against_supply(
    contracts: f64,
    units_per_contract: f64,
    deliverable_stock: f64,
) -> Option<f64> {
    if deliverable_stock <= 0.0 {
        return None;
    }
    Some(contracts * units_per_contract / deliverable_stock)
}

/// STOCK IS TIGHT OR IT IS NOT, AND STORING IT COSTS MONEY TO SOMEBODY.
pub struct Storing {
    pub kind: u32,
    pub at_line: u32,
    pub at_value: u32,
    /// What a week of storage costs, per unit.
    pub per_unit: &'static str,
}

impl Mechanism for Storing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let per_unit = ctx.params().price_per_unit(self.per_unit);

        // Who holds the storage, by place.
        let mut warehouses: std::collections::HashMap<u32, PartyId> =
            std::collections::HashMap::new();
        for &keeper in ctx.parties().of_kind(kinds::STOCKIST) {
            let who = PartyId(keeper);
            if ctx.parties().alive(who) {
                warehouses
                    .entry(ctx.parties().region_of(who).0)
                    .or_insert(who);
            }
        }
        if warehouses.is_empty() {
            return;
        }

        // What was consumed, by line.
        let mut consumed_of: HashMap<u32, f64> = HashMap::new();
        let mut produced_by: HashMap<u32, Vec<Producer>> = HashMap::new();
        for n in ctx.wire().in_period(ctx.week()) {
            for leg in ctx.wire().legs_of(n) {
                match *leg {
                    crate::ledger::Leg::Create {
                        party,
                        instrument,
                        qty,
                        cost_per_unit,
                    } if ctx.instruments().class_of(instrument)
                        == crate::instruments::Class::Good =>
                    {
                        produced_by.entry(instrument.0).or_default().push(Producer {
                            who: party,
                            cost_per_unit,
                            capacity: qty.get(),
                        });
                    }
                    crate::ledger::Leg::Destroy {
                        instrument, qty, ..
                    } if ctx.instruments().class_of(instrument)
                        == crate::instruments::Class::Good =>
                    {
                        *consumed_of.entry(instrument.0).or_insert(0.0) += qty.get();
                    }
                    _ => {}
                }
            }
        }

        let mut charging: Vec<(PartyId, PartyId, f64)> = Vec::new();
        let mut tight: Vec<(u32, f64)> = Vec::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            if ctx.instruments().class_of(line) != crate::instruments::Class::Good {
                continue;
            }
            let (held, _) = ctx.register().held_total(line);
            if held <= 0.0 {
                continue;
            }
            // A line with no `Destroy` leg this week had none consumed — the walk above saw every
            // leg, so its absence is the answer and not a number standing in for one.
            let consumed = match consumed_of.get(&row) {
                Some(&units) => units,
                None => 0.0,
            };
            if let Some(t) = tightness(held, consumed) {
                tight.push((row, t));
            }
            // The supply observation is derived from named production rows and the price this
            // line's own book printed. It is not a second schedule or a price-setting formula.
            if let (Some(print), Some(producers)) =
                (ctx.prints().latest(line, ctx.week()), produced_by.get(&row))
            {
                let available = supply_at(print.price, producers);
                ctx.say(
                    self.kind,
                    &[row],
                    &[
                        (self.at_line, Value::Num(f64::from(row))),
                        (self.at_value, Value::Num(available)),
                    ],
                    true,
                );
            }
            // And everybody holding it pays for the storage, to the keeper of its own place.
            for &holding in ctx.register().of_instrument(line) {
                let holding = crate::ids::HoldingId(holding);
                let holder = ctx.register().holder_of(holding);
                if !ctx.parties().alive(holder) {
                    continue;
                }
                let Some(&keeper) = warehouses.get(&ctx.parties().region_of(holder).0) else {
                    continue;
                };
                if keeper == holder {
                    continue;
                }
                let units = ctx.register().quantity(holding);
                let (to, fee) = storage_fee(units, per_unit, keeper);
                if fee > 0.0 {
                    charging.push((holder, to, fee));
                }
            }
        }

        for (line, t) in tight {
            // The measure of scarcity, published.
            ctx.say(
                self.kind,
                &[line],
                &[
                    (self.at_line, Value::Num(f64::from(line))),
                    (self.at_value, Value::Num(t)),
                ],
                true,
            );
        }
        for (holder, keeper, fee) in charging {
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), holder) else {
                continue;
            };
            // A fee of nothing is not charged.
            let Some(fee) = crate::ledger::Units::new(fee) else {
                continue;
            };
            // Two named sides, in the same pass.
            ctx.propose(
                vec![crate::ledger::Leg::Money {
                    from: holder,
                    to: keeper,
                    instrument: money,
                    amount: fee,
                    receipt: crate::ledger::Receipt::Sale,
                }],
                crate::ledger::Cause::Payment,
                crate::ledger::Delivery::Nothing,
                "21 D3: storage costs money, and it is paid to whoever owns the storage",
            );
        }
    }
}

/// Physical goods reconcile from the wire rather than from a parallel inventory ledger. Transfers
/// cancel in aggregate; only creation and destruction can change the stock of a line.
#[derive(Default)]
pub struct CommodityUnits {
    opening: HashMap<u32, f64>,
    closing: HashMap<u32, f64>,
    produced: HashMap<u32, f64>,
    consumed: HashMap<u32, f64>,
    goods: Vec<bool>,
    last_week: Option<u32>,
    found: Vec<Violation>,
}

impl CommodityUnits {
    pub fn over(goods: Vec<bool>) -> Self {
        Self {
            goods,
            ..Self::default()
        }
    }

    fn is_good(&self, line: InstrumentId) -> bool {
        matches!(self.goods.get(line.row()), Some(true))
    }
}

impl Contribution for CommodityUnits {
    fn family(&self) -> Family {
        Family::Units
    }

    fn contributor(&self) -> &'static str {
        "commodities-spot"
    }

    fn before(&mut self, from: &Sources<'_>) {
        self.found.clear();
        self.produced.clear();
        self.consumed.clear();
        self.opening = std::mem::take(&mut self.closing);
        for instruction in from.wire.in_period(from.week) {
            for leg in from.wire.legs_of(instruction) {
                match *leg {
                    crate::ledger::Leg::Create {
                        instrument, qty, ..
                    } if self.is_good(instrument) => {
                        *self.produced.entry(instrument.0).or_insert(0.0) += qty.get();
                    }
                    crate::ledger::Leg::Destroy {
                        instrument, qty, ..
                    } if self.is_good(instrument) => {
                        *self.consumed.entry(instrument.0).or_insert(0.0) += qty.get();
                    }
                    _ => {}
                }
            }
        }
    }

    fn visit(&mut self, at: &Visit<'_>) {
        let line = at.register.instrument_of(at.row);
        if self.is_good(line) {
            *self.closing.entry(line.0).or_insert(0.0) += at.register.quantity(at.row);
        }
    }

    fn finish(&mut self, week: u32) -> Vec<Violation> {
        if self.last_week == week.checked_sub(1) {
            for row in 0..self.goods.len() as u32 {
                let line = InstrumentId::at(row);
                if !self.is_good(line) {
                    continue;
                }
                let opening = match self.opening.get(&row) {
                    Some(&units) => units,
                    None => 0.0,
                };
                let produced = match self.produced.get(&row) {
                    Some(&units) => units,
                    None => 0.0,
                };
                let consumed = match self.consumed.get(&row) {
                    Some(&units) => units,
                    None => 0.0,
                };
                let closing = match self.closing.get(&row) {
                    Some(&units) => units,
                    None => 0.0,
                };
                if let Some(off) = units_balance(produced, opening, consumed, closing, 4) {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Commodities Spot D5",
                        owner: format!("commodity {}", line.0),
                        size: off,
                        unit: "physical units",
                        week,
                        message: format!(
                            "produced {produced} plus opening {opening} does not equal consumed {consumed} plus closing {closing}"
                        ),
                    });
                }
            }
        }
        self.last_week = Some(week);
        std::mem::take(&mut self.found)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn here() -> Grade {
        Grade {
            what: 1,
            at: RegionId::at(1),
        }
    }

    fn there() -> Grade {
        Grade {
            what: 1,
            at: RegionId::at(2),
        }
    }

    fn producers() -> Vec<Producer> {
        vec![
            Producer {
                who: party(1),
                cost_per_unit: 40.0,
                capacity: 500.0,
            },
            Producer {
                who: party(2),
                cost_per_unit: 55.0,
                capacity: 300.0,
            },
            Producer {
                who: party(3),
                cost_per_unit: 80.0,
                capacity: 400.0,
            },
        ]
    }

    #[test]
    fn the_same_grade_in_two_places_is_two_prices() {
        // Location is part of the identity, and the difference is transport.
        assert_ne!(here(), there());
    }

    #[test]
    fn the_supply_schedule_is_a_consequence_of_the_cost_distribution() {
        // Never a curve written down.
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
        let stock = Inventory {
            held_by: party(5),
            of: here(),
            units: 100.0,
        };
        assert_eq!(stock.draw(40.0).unwrap().units, 60.0);
        assert!(stock.draw(140.0).is_none());
        assert_eq!(stock.add(50.0).units, 150.0);
    }

    #[test]
    fn inventory_is_the_buffer_and_when_it_runs_out_demand_goes_unmet() {
        // When stocks approach zero the price has nothing left to ration with.
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
        // A consequence to be measured, not a volatility parameter.
        let within = clearing(800.0, &producers(), 0.0, &[]).price.unwrap();
        let over = clearing(801.0, &producers(), 0.0, &[]).price.unwrap();
        assert_eq!(within, 55.0);
        assert_eq!(over, 80.0);
    }

    #[test]
    fn produced_plus_opening_equals_consumed_plus_closing() {
        // Exactly, per commodity and location — and the discrepancy is units that appeared or
        // vanished.
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
        // The asymmetry is real, and it is why the two states are not symmetric.
        let spot = 100.0;
        let carry = full_carry(spot, 4.0, 0.05, 1.0);
        assert!(carry > spot);
        // Above full carry the arbitrage is open, and somebody with storage and funding takes it.
        assert!(arbitrages(
            against_carry(carry + 5.0, spot, 4.0, 0.05, 1.0),
            10_000.0,
            10_000.0
        )
        .is_some());
        // Far below spot there is nothing to take: you cannot store a shortage.
        assert!(arbitrages(
            against_carry(60.0, spot, 4.0, 0.05, 1.0),
            10_000.0,
            10_000.0
        )
        .is_none());
    }

    #[test]
    fn without_storage_or_funding_there_is_no_arbitrageur_and_the_curve_floats_free() {
        // Without a storable stock there is no such participant, and the curve has nothing
        // tying it to the physical world.
        let over = against_carry(120.0, 100.0, 4.0, 0.05, 1.0);
        assert!(arbitrages(over, 0.0, 10_000.0).is_none());
        assert!(arbitrages(over, 10_000.0, 0.0).is_none());
    }

    #[test]
    fn the_curve_reads_inventory_and_low_stocks_are_a_consequence_not_a_rule() {
        // Inventory is the state variable the curve reads.
        let ample = tightness(5_000.0, 500.0).unwrap();
        let scarce = tightness(200.0, 500.0).unwrap();
        assert!(ample > scarce);
        assert!(tightness(200.0, 0.0).is_none());
    }

    #[test]
    fn convergence_is_a_consequence_of_deliverability_and_is_never_enforced() {
        // Nothing here drives the price to spot.
        let real = Future {
            on: here(),
            units_per_contract: 100.0,
            contracts: 10.0,
            price: 104.0,
            expires_in_years: 0.25,
            deliverable: true,
        };
        assert!(can_converge(&real));
        let paper = Future {
            deliverable: false,
            ..real
        };
        assert!(!can_converge(&paper));
        // And a future on a commodity nobody trades physically prices itself.
        assert!(may_list(true, true));
        assert!(!may_list(false, true));
        assert!(!may_list(true, false));
    }

    #[test]
    fn no_roll_is_free_and_the_curve_decides_whether_it_costs_or_pays() {
        // Most of an investor's return, and it lands in its P&L.
        let in_contango = roll(100.0, 104.0, 10.0, 100.0);
        let in_backwardation = roll(100.0, 96.0, 10.0, 100.0);
        assert!(in_contango < 0.0);
        assert!(in_backwardation > 0.0);
    }

    #[test]
    fn a_party_that_cannot_take_delivery_must_close_or_roll() {
        // All three are real, and cash settlement needs an OBSERVED price.
        let f = Future {
            on: here(),
            units_per_contract: 100.0,
            contracts: 10.0,
            price: 104.0,
            expires_in_years: 0.0,
            deliverable: true,
        };
        assert_eq!(at_expiry(&f, true, Some(101.0)), AtExpiry::Delivers);
        assert_eq!(at_expiry(&f, false, Some(101.0)), AtExpiry::CashSettles);
        assert_eq!(at_expiry(&f, false, None), AtExpiry::MustCloseOrRoll);
    }

    #[test]
    fn open_interest_against_finite_deliverable_supply_is_visible() {
        // No unlimited open interest without the squeeze that implies — so the ratio is a
        // read, and a large one is the squeeze being visible rather than prevented.
        assert_eq!(
            open_interest_against_supply(100.0, 100.0, 10_000.0),
            Some(1.0)
        );
        assert_eq!(
            open_interest_against_supply(400.0, 100.0, 10_000.0),
            Some(4.0)
        );
        assert!(open_interest_against_supply(100.0, 100.0, 0.0).is_none());
    }

    #[test]
    #[should_panic(expected = "more units than the line could make")]
    fn a_disruption_cannot_lose_more_than_the_line_could_make() {
        disrupted(&producers()[0], 900.0);
    }
}
