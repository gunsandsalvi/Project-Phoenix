//! The capital programme's audit contribution: PLANT MOVES ONLY FOR A REASON.
//!
//! @spec Capital Programme A6.b, Capital Programme D1, Law 5, Law 19

use crate::audit::{Contribution, Family, Sources, Violation, Visit};
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::Leg;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::afoot;
use std::collections::HashMap;

/// MISSING IS MISSING — and these two are not missing, they are NOTHING, which is an answer.
#[inline]
fn held_nothing_then(before: &HashMap<u64, f64>, k: u64) -> f64 {
    match before.get(&k) {
        Some(&q) => q,
        None => 0.0,
    }
}

/// And what the legs accounted for where none of them named this holding: nothing, over no terms.
#[inline]
fn legs_said_nothing(moved: &HashMap<u64, (f64, f64, u32)>, k: u64) -> (f64, f64, u32) {
    match moved.get(&k) {
        Some(&seen) => seen,
        None => (0.0, 0.0, 0),
    }
}

#[inline]
const fn key(party: PartyId, instrument: InstrumentId) -> u64 {
    ((party.0 as u64) << 32) | (instrument.0 as u64)
}

/// What the legs say moved, per holding, kept as TERMS rather than a running total — Law 7's dust is
/// derived from the terms and a total alone cannot produce it.
#[derive(Default)]
pub struct PlantMoves {
    capital: Vec<bool>,
    /// What the legs accounted for, this week.
    moved: HashMap<u64, (f64, f64, u32)>,
    /// What the register holds now, for the lines that are capital.
    held: HashMap<u64, f64>,
    /// And what it held last week, which is what a change is measured against.
    before: HashMap<u64, f64>,
    /// Whether the two weeks are consecutive; without that there is nothing to compare.
    comparable: bool,
    last_period: Option<u32>,
    found: Vec<Violation>,
}

impl PlantMoves {
    /// The lines this module says are capital, by row.
    pub fn over(capital: Vec<bool>) -> Self {
        Self {
            capital,
            ..Default::default()
        }
    }

    #[inline]
    fn is_capital(&self, i: InstrumentId) -> bool {
        matches!(self.capital.get(i.row()), Some(true))
    }

    #[inline]
    fn account(&mut self, party: PartyId, instrument: InstrumentId, qty: f64) {
        if !self.is_capital(instrument) {
            return;
        }
        let e = self
            .moved
            .entry(key(party, instrument))
            .or_insert((0.0, 0.0, 0));
        e.0 += qty;
        e.1 += qty.abs();
        e.2 += 1;
    }
}

impl Contribution for PlantMoves {
    fn family(&self) -> Family {
        Family::Units
    }
    fn contributor(&self) -> &'static str {
        "capital-programme"
    }

    /// Its own pass over its own source: the legs of this week, and what each says moved.
    fn before(&mut self, from: &Sources<'_>) {
        self.moved.clear();
        self.comparable = self.last_period == from.week.checked_sub(1);
        self.before = std::mem::take(&mut self.held);
        for n in from.wire.in_period(from.week) {
            for leg in from.wire.legs_of(n) {
                match *leg {
                    // Goods B, E4: a thing coming into existence or leaving it.
                    Leg::Create {
                        party,
                        instrument,
                        qty,
                        ..
                    } => self.account(party, instrument, qty.get()),
                    Leg::Destroy {
                        party,
                        instrument,
                        qty,
                        ..
                    } => self.account(party, instrument, -qty.get()),
                    // And a move between two holders is two sides of one fact.
                    Leg::Asset {
                        from: seller,
                        to: buyer,
                        instrument,
                        qty,
                        ..
                    } => {
                        self.account(buyer, instrument, qty.get());
                        self.account(seller, instrument, -qty.get());
                    }
                    // Minting is money, and money is not a thing this family counts the units of.
                    Leg::Money { .. }
                    | Leg::Mint { .. }
                    | Leg::Pledge { .. }
                    | Leg::Depreciate { .. }
                    | Leg::Dispatch { .. } => {}
                }
            }
        }
    }

    /// The shared walk: what the register holds of every capital line, now.
    fn visit(&mut self, at: &Visit<'_>) {
        let instrument = at.register.instrument_of(at.row);
        if !self.is_capital(instrument) {
            return;
        }
        let holder = at.register.holder_of(at.row);
        self.held
            .insert(key(holder, instrument), at.register.quantity(at.row));
    }

    fn finish(&mut self, week: u32) -> Vec<Violation> {
        if self.comparable {
            // What moved, against what the legs say moved.
            for (&k, &now) in &self.held {
                let was = held_nothing_then(&self.before, k);
                let change = now - was;
                let (accounted, magnitude, terms) = legs_said_nothing(&self.moved, k);
                // The dust of THIS comparison, from its own terms and magnitudes.
                let dust = (terms as f64 + 2.0)
                    * f64::EPSILON
                    * (magnitude + change.abs() + now.abs() + was.abs());
                if (change - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Capital Programme A6.b",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: change - accounted,
                        unit: "units",
                        week,
                        message: format!(
                            "plant moved by {change} and its legs account for {accounted}"
                        ),
                    });
                }
            }
            // A holding that went to NOTHING still has to have a leg behind it.
            for (&k, &was) in &self.before {
                if self.held.contains_key(&k) {
                    continue;
                }
                let (accounted, magnitude, terms) = legs_said_nothing(&self.moved, k);
                let dust = (terms as f64 + 2.0) * f64::EPSILON * (magnitude + was.abs());
                if (-was - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Capital Programme A6.b",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: -was - accounted,
                        unit: "units",
                        week,
                        message: format!(
                            "plant left at {was} and its legs account for {accounted}"
                        ),
                    });
                }
            }
        }
        self.last_period = Some(week);
        std::mem::take(&mut self.found)
    }
}

// The three family fixtures are gone for the reason the rest of the audit's are: arranging plant
// that moved with no leg behind it, and checking the family says so, proves the arrangement. The
// family runs over the real world every week and reports an owner and a size.
//
// What is left is the arithmetic, and it always was values in and values out.

// And the party that posts for it.

/// AND IT BUYS THE PLANT.
pub struct Builder {
    pub of_kind: u32,
}

impl crate::module::Participant for Builder {
    /// Plant is bought to be used, not traded: it is carried at what it cost and written down as
    /// it is consumed.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        Some(crate::register::Carrying::Cost)
    }

    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        view.programme_markets()
    }

    fn orders(
        &self,
        view: &crate::module::ParticipantView<'_>,
        m: crate::ids::MarketId,
    ) -> Vec<crate::clearing::Order> {
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        let commits = view.programme_on(line);
        if commits <= 0.0 {
            return Vec::new();
        }
        // 46 F2, 33 A5: the most it will pay for a unit of plant is what that plant is worth to
        // it. The last print is what somebody else paid, and a firm that buys at it has not asked
        // whether the thing earns its price.
        let Some(worth) = view.values(line) else {
            return Vec::new();
        };
        if worth <= 0.0 {
            return Vec::new();
        }
        // It cannot commit more money than it has.
        let can_pay = view.own_cash();
        let money = if commits < can_pay { commits } else { can_pay };
        let units = crate::clearing::whole_pieces(money / worth);
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Buy,
            price: Some(worth),
            qty: units,
        }]
    }
}

// WHETHER A PROJECT IS WORTH DOING IS THE FIRM'S DECISION, so it is made here. What the firm's
// capital costs it is another system's and arrives as a public event on the journal, which is
// where this reads it: a fact with a place is read rather than recomputed.

/// Joint two: a PROJECT, with a return and a hurdle.
#[derive(Clone, Copy, Debug)]
pub struct Project {
    /// What it expects to get, per week, from its own outlook — never a model forecast.
    pub returns_per_period: f64,
    pub costs: f64,
    /// The management's own patience, in weeks.
    pub horizon: f64,
    /// And its own risk aversion, above the cost of capital.
    pub hurdle: f64,
}

/// Joint two: it invests when it expects the return to exceed its cost of capital.
pub fn worth_doing(p: &Project, cost_of_capital: f64) -> bool {
    if p.costs <= 0.0 {
        return false;
    }
    let over_the_horizon = p.returns_per_period * p.horizon;
    let expected = (over_the_horizon - p.costs) / p.costs / p.horizon;
    expected > cost_of_capital + p.hurdle
}

/// A FIRM DECIDES TO INVEST, AND THE COMPARISON IS THE MECHANISM.
pub struct Building {
    pub kind: u32,
    pub at_funding: u32,
    /// What its capital costs it, published by the cost-of-capital row.
    pub costs: u32,
    /// The management's own patience and its own risk aversion above the cost of capital.
    pub horizon: &'static str,
    pub hurdle: &'static str,
    /// 21i, 33 A4: the standing area at which building draws twice what it does on empty ground.
    pub crowds_at: &'static str,
    /// How long a programme runs before the plant is in service.
    pub takes: &'static str,
}

impl Mechanism for Building {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let horizon = ctx.params().weeks(self.horizon);
        let hurdle = ctx.params().ratio(self.hurdle);
        let crowds_at = ctx.params().square_km(self.crowds_at);
        let takes = ctx.params().weeks(self.takes) as u32;

        // What each company's capital costs it, most recently published.
        let mut costs: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.costs) {
            if let (Some(&who), Some(Value::Num(cost))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, 0),
            ) {
                costs.insert(who, cost);
            }
        }
        if costs.is_empty() {
            return;
        }
        // How built-up each place is.
        let built = crate::places::built_up(
            ctx.parties(),
            ctx.register(),
            ctx.registry(),
            ctx.geography(),
        );

        let mut opening: Vec<(PartyId, InstrumentId, f64, f64)> = Vec::new();
        for (&who, &cost_of_capital) in &costs {
            let firm = PartyId(who);
            if !ctx.parties().alive(firm) {
                continue;
            }
            if ctx
                .processes()
                .running(afoot::CAPITAL_PROGRAMME)
                .iter()
                .any(|p| ctx.processes().owner(*p) == firm)
            {
                continue;
            }
            // Its own outlook, and a firm with none has nothing to expect.
            let Some(sells) = ctx
                .outlooks()
                .of(firm, crate::stores::about::HOW_MUCH_IT_SELLS)
            else {
                continue;
            };
            // A project needs the price of one named output. Unlike prices cannot be averaged into
            // a unitless "price outlook"; until the recipe names its output explicitly, ambiguity
            // means there is no lawful investment decision.
            let prices: Vec<(InstrumentId, f64)> = ctx
                .register()
                .of_holder(firm)
                .iter()
                .filter_map(|row| {
                    let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                    ctx.outlooks()
                        .of(firm, crate::stores::about::price_of(line))
                        .map(|price| (line, price))
                })
                .collect();
            let [(output, price)] = prices.as_slice() else {
                continue;
            };
            let Some(plant) = ctx.registry().made_with(*output) else {
                continue;
            };
            // What the ground it stands on does to a build.
            let where_it_is = ctx.parties().region_of(firm);
            let crowding =
                crate::places::crowding(crate::places::standing_in(&built, where_it_is), crowds_at);
            let project = Project {
                returns_per_period: sells * *price,
                costs: sells * *price * crowding,
                horizon,
                hurdle,
            };
            if !worth_doing(&project, cost_of_capital) {
                continue;
            }
            let Some(money) = crate::ledger::account_of(ctx.parties(), ctx.instruments(), firm)
            else {
                continue;
            };
            let cash = ctx.register().quantity(ctx.register().row(firm, money));
            let funding = if project.costs > cash {
                project.costs - cash
            } else {
                0.0
            };
            opening.push((firm, plant, project.costs, funding));
        }

        for (firm, plant, commits, funding) in opening {
            ctx.opens(crate::module::Opens {
                kind: afoot::CAPITAL_PROGRAMME,
                owner: firm,
                subject: Some(plant),
                door: None,
                closes: Some(ctx.week() + takes),
                size: commits,
            });
            ctx.say(
                self.kind,
                &[firm.0],
                &[
                    (0, Value::Num(commits)),
                    (self.at_funding, Value::Num(funding)),
                ],
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::{capacity, charge, in_service, net, upkeep, worn};
    use crate::register::Lot;
    use crate::registry::Plant;

    fn mill() -> Plant {
        Plant {
            life: 5,
            upkeep_per_period: 3.0,
            capacity_per_period: 100.0,
        }
    }

    fn bought(units: f64, cost: f64, when: u32) -> Lot {
        Lot {
            qty: units,
            basis_per_unit: cost,
            acquired: when,
        }
    }

    #[test]
    fn the_charge_is_against_the_stock_and_not_against_revenue() {
        // One schedule, charged in both places.
        let p = mill();
        let one = charge(&bought(10.0, 500.0, 0), &p, 1);
        let two = charge(&bought(20.0, 500.0, 0), &p, 1);
        assert_eq!(two, one * 2.0);
        // And the stock it is charged against falls by exactly what was charged.
        let v = bought(10.0, 500.0, 0);
        assert_eq!(net(&v, &p, 0) - net(&v, &p, 1), charge(&v, &p, 1));
    }

    #[test]
    fn a_vintage_leaves_when_fully_worn_and_the_charge_stops_with_it() {
        // The charge stops when the plant is gone.
        let p = mill();
        let v = bought(10.0, 500.0, 0);
        assert!(in_service(&v, &p, 4));
        assert!(!in_service(&v, &p, 5));
        assert_eq!(charge(&v, &p, 5), 0.0);
        assert_eq!(net(&v, &p, 5), 0.0);
        assert_eq!(worn(&v, &p, 5), 10.0 * 500.0);
        // And nothing keeps paying to keep a plant that is gone.
        assert_eq!(upkeep(&v, &p, 5), 0.0);
    }

    #[test]
    fn upkeep_is_owed_whether_or_not_the_line_runs_and_that_is_what_idle_capacity_costs() {
        // Costs no batch absorbed are week costs.
        let p = mill();
        let v = bought(10.0, 500.0, 0);
        assert_eq!(upkeep(&v, &p, 1), 30.0);
        assert_eq!(upkeep(&v, &p, 3), 30.0);
        // It is not depreciation, and the two are never summed: one is cash paid to somebody, the
        // other is the plant wearing out, and they are different sizes for the same vintage.
        assert!(upkeep(&v, &p, 1) != charge(&v, &p, 1));
    }

    #[test]
    fn capacity_is_a_function_of_the_stock_and_the_worn_out_vintages_are_not_in_it() {
        // A world producing more than its capital allows has capacity from nowhere.
        let p = mill();
        let stock = [bought(10.0, 500.0, 0), bought(4.0, 600.0, 3)];
        // There is a lag between the spend and the capacity.
        assert_eq!(capacity(&stock, &p, 1), 1_000.0);
        assert_eq!(capacity(&stock, &p, 3), 1_400.0);
        // By week 5 the first vintage is worn out and only the second is making anything.
        assert_eq!(capacity(&stock, &p, 5), 400.0);
    }

    #[test]
    fn a_project_is_done_when_the_return_clears_the_cost_and_the_hurdle_and_not_otherwise() {
        let p = Project {
            returns_per_period: 30.0,
            costs: 100.0,
            horizon: 10.0,
            hurdle: 0.02,
        };
        // 200 over 10 weeks on 100 is 20% a week; it clears a 6% cost plus a 2% hurdle.
        assert!(worth_doing(&p, 0.06));
        // The same project does not clear a cost of capital of 25%.
        assert!(!worth_doing(&p, 0.25));
        // A project that does not clear is not done SMALLER.
        let marginal = Project {
            returns_per_period: 10.5,
            ..p
        };
        assert!(!worth_doing(&marginal, 0.06));
    }

    #[test]
    fn the_hurdle_and_the_horizon_are_the_managements_own() {
        // Read off its risk aversion and its patience.
        let patient = Project {
            returns_per_period: 12.0,
            costs: 100.0,
            horizon: 20.0,
            hurdle: 0.01,
        };
        let impatient = Project {
            horizon: 3.0,
            hurdle: 0.10,
            ..patient
        };
        assert!(worth_doing(&patient, 0.05));
        assert!(!worth_doing(&impatient, 0.05));
    }
}
