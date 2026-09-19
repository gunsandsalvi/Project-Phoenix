//! The capital programme's audit contribution: PLANT MOVES ONLY FOR A REASON.
//!
//! @spec Capital Programme A6.b, Capital Programme D1, Law 5, Law 19

use crate::audit::{Contribution, Family, Sources, Violation, Visit};
use crate::ids::{InstrumentId, PartyId};
use crate::ledger::Leg;
use std::collections::HashMap;

/// MISSING IS MISSING — and these two are not missing, they are NOTHING, which is an answer. A
/// holding that was not on the register last period held none of the line; a holding no leg
/// mentioned had nothing accounted for.
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
    /// Which lines are capital. A fact about the INSTRUMENTS, declared by the module that owns them,
    /// and never a branch on a kind inside the check.
    capital: Vec<bool>,
    /// What the legs accounted for, this period.
    moved: HashMap<u64, (f64, f64, u32)>,
    /// What the register holds now, for the lines that are capital.
    held: HashMap<u64, f64>,
    /// And what it held last period, which is what a change is measured against.
    before: HashMap<u64, f64>,
    /// Whether the two periods are consecutive; without that there is nothing to compare.
    comparable: bool,
    last_period: Option<u32>,
    found: Vec<Violation>,
}

impl PlantMoves {
    /// The lines this module says are capital, by row. It is DATA in a registry, handed in, so the
    /// check never asks an instrument what kind it is.
    pub fn over(capital: Vec<bool>) -> Self {
        Self { capital, ..Default::default() }
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
        let e = self.moved.entry(key(party, instrument)).or_insert((0.0, 0.0, 0));
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

    /// Its own pass over its own source: the legs of this period, and what each says moved.
    fn before(&mut self, from: &Sources<'_>) {
        self.moved.clear();
        self.comparable = self.last_period == from.period.checked_sub(1);
        self.before = std::mem::take(&mut self.held);
        for n in from.wire.in_period(from.period) {
            for leg in from.wire.legs_of(n) {
                match *leg {
                    // Goods B, E4: a thing coming into existence or leaving it. ONE side.
                    Leg::Create { party, instrument, qty, .. } => self.account(party, instrument, qty),
                    Leg::Destroy { party, instrument, qty, .. } => self.account(party, instrument, -qty),
                    // And a move between two holders is two sides of one fact.
                    Leg::Asset { from: seller, to: buyer, instrument, qty, .. } => {
                        self.account(buyer, instrument, qty);
                        self.account(seller, instrument, -qty);
                    }
                    // Minting is money, and money is not a thing this family counts the units of.
                    Leg::Money { .. } | Leg::Mint { .. } | Leg::Pledge { .. } => {}
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
        self.held.insert(key(holder, instrument), at.register.quantity(at.row));
    }

    fn finish(&mut self, period: u32) -> Vec<Violation> {
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
                        period,
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
                let dust =
                    (terms as f64 + 2.0) * f64::EPSILON * (magnitude + was.abs());
                if (-was - accounted).abs() > dust {
                    self.found.push(Violation {
                        family: Family::Units,
                        spec: "Capital Programme A6.b",
                        owner: format!("{}/{}", (k >> 32) as u32, k as u32),
                        size: -was - accounted,
                        unit: "units",
                        period,
                        message: format!("plant left at {was} and its legs account for {accounted}"),
                    });
                }
            }
        }
        self.last_period = Some(period);
        std::mem::take(&mut self.found)
    }
}

/// The stock is a set of dated vintages, each with its own cost and its own service date — and a
/// vintage is a LOT ON THE REGISTER, not a second book kept beside it. The register already holds
/// units with a basis and a date; a parallel vintage store would be Law 4's defect, two writers for
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Vintage {
    pub units: f64,
    pub cost_per_unit: f64,
    /// Its own service date. C3's lag is between the spend and this.
    pub in_service: u32,
}

/// What a kind of plant IS — a TECHNOLOGY primitive about the capital good, declared once per line
/// and true for every holder of it. A4.a: what a firm's plant is made of is a property of its
/// industry, so this is DATA handed in, never a branch on a kind.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Plant {
    /// A useful life of its own, and the presence of a life is what makes a good a capital good. In
    /// periods.
    pub life: u32,
    /// What it costs to keep, every period, whether or not it runs. This is the fixed cost, and it
    /// is the whole of operating leverage: a firm with a large plant and a quiet line pays it
    /// anyway, and that is what makes idle capacity expensive (37 F5.a).
    pub upkeep_per_period: f64,
    /// Capacity is a function of the stock. What one unit of this plant can make in a period.
    pub capacity_per_period: f64,
}

/// One depreciation schedule, charged in both places — against profit and against the stock.
/// Straight line over the declared life: a convention, stated here as the one convention, not a
/// shape with a claim in it.
pub fn charge(v: &Vintage, p: &Plant, now: u32) -> f64 {
    if !in_service(v, p, now) {
        return 0.0;
    }
    v.units * v.cost_per_unit / (p.life as f64)
}

/// A vintage leaves the register when fully worn, so the charge stops when the plant is gone. This
/// is not a floor under the charge — it is the plant having been consumed, which is arithmetic about
/// a thing that ran out.
pub fn in_service(v: &Vintage, p: &Plant, now: u32) -> bool {
    now >= v.in_service && now - v.in_service < p.life
}

/// Accumulated depreciation is a READ over the vintages, never a stored balance.
pub fn worn(v: &Vintage, p: &Plant, now: u32) -> f64 {
    let periods = if now <= v.in_service {
        0
    } else if now - v.in_service > p.life {
        p.life
    } else {
        now - v.in_service
    };
    v.units * v.cost_per_unit * (periods as f64) / (p.life as f64)
}

/// And so is net book value. Gross is `units * cost_per_unit`, and nothing stores the difference.
pub fn net(v: &Vintage, p: &Plant, now: u32) -> f64 {
    v.units * v.cost_per_unit - worn(v, p, now)
}

/// What the firm pays this period to keep this vintage, whether or not the line runs. A vintage out
/// of service is a vintage nobody keeps.
pub fn upkeep(v: &Vintage, p: &Plant, now: u32) -> f64 {
    if !in_service(v, p, now) {
        return 0.0;
    }
    v.units * p.upkeep_per_period
}

/// Capacity is a function of the stock, summed over the vintages still in service. 37 B1.a reads
/// this as one of the firm's reasons; nothing writes it.
pub fn capacity(vintages: &[Vintage], p: &Plant, now: u32) -> f64 {
    vintages.iter().filter(|v| in_service(v, p, now)).map(|v| v.units * p.capacity_per_period).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::Audit;
    use crate::journal::Journal;
    use crate::ledger::{Cause, Instruction, Receipt, Settlement, Settling};
    use crate::instruments::Instruments;
    use crate::parties::Parties;
    use crate::register::Register;

    fn capital(lines: usize) -> Vec<bool> {
        vec![true; lines]
    }

    /// The stores the audit derives its answers from, gathered for a call. This family reads only
    /// the wire and the register; the other two are here because `Sources` is one shape for every
    /// family.
    fn over<'a>(register: &'a Register, wire: &'a Settlement, period: u32) -> Sources<'a> {
        static NOBODY: std::sync::OnceLock<(Instruments, Parties)> = std::sync::OnceLock::new();
        let (instruments, parties) = NOBODY.get_or_init(|| (Instruments::new(), Parties::new()));
        Sources { wire, register, instruments, parties, period }
    }

    #[test]
    fn plant_that_moved_with_a_leg_behind_it_is_not_a_violation() {
        let mut reg = Register::new();
        let mut j = Journal::new();
        let says = crate::ledger::Outcomes::declared(&mut j);
        let cal = crate::calendar::Calendar::new(crate::calendar::Day(0), 7, 3);
        let mut wire = Settlement::new(6);
        let a = PartyId::at(0);
        let b = PartyId::at(1);
        let plant = InstrumentId::at(1);
        reg.credit(a, plant, 100.0, 1.0, 0);

        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(capital(4))));
        // Period 1 establishes what is held; nothing is comparable yet.
        audit.run(&over(&reg, &wire, 1));

        // Period 2: it sells 40, and the leg says so.
        let legs = [Leg::Asset { from: a, to: b, instrument: plant, qty: 40.0, price_per_unit: Some(2.0) }];
        // No money leg here, so the payment system is never consulted: empty stores are the truth.
        let (ps, mut ins) = (Parties::new(), Instruments::new());
        wire.settle(
            &Instruction::free_of_payment(&legs, Cause::Trade),
            2,
            &mut Settling { register: &mut reg, journal: &mut j, parties: &ps, instruments: &mut ins, calendar: &cal, says },
        );
        let reports = audit.run(&over(&reg, &wire, 2));
        assert!(
            reports[0].violations.is_empty(),
            "{:?}",
            reports[0].violations.iter().map(|v| v.message.clone()).collect::<Vec<_>>()
        );
        let _ = Receipt::Sale;
    }

    #[test]
    fn plant_that_moved_with_no_leg_behind_it_has_nowhere_to_hide() {
        let mut reg = Register::new();
        let wire = Settlement::new(6);
        let a = PartyId::at(0);
        let plant = InstrumentId::at(1);
        reg.credit(a, plant, 100.0, 1.0, 0);

        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(capital(4))));
        audit.run(&over(&reg, &wire, 1));

        // Units appear with no instruction behind them — which is what this family exists to find.
        reg.credit(a, plant, 25.0, 1.0, 2);
        let reports = audit.run(&over(&reg, &wire, 2));
        assert_eq!(reports[0].violations.len(), 1);
        assert_eq!(reports[0].violations[0].size, 25.0);
        assert_eq!(reports[0].violations[0].spec, "Capital Programme A6.b");
        // And it never repairs: the units are still there.
        assert_eq!(reg.quantity(reg.row(a, plant)), 125.0);
    }

    #[test]
    fn a_line_that_is_not_capital_is_not_this_familys_business() {
        let mut reg = Register::new();
        let wire = Settlement::new(6);
        let a = PartyId::at(0);
        let share = InstrumentId::at(2);
        // Only row 1 is capital; row 2 is not.
        let mut which = vec![false; 4];
        which[1] = true;
        reg.credit(a, share, 100.0, 1.0, 0);
        let mut audit = Audit::new();
        audit.add(Box::new(PlantMoves::over(which)));
        audit.run(&over(&reg, &wire, 1));
        reg.credit(a, share, 50.0, 1.0, 2);
        let reports = audit.run(&over(&reg, &wire, 2));
        assert!(reports[0].violations.is_empty());
    }

    /// A mill: five periods of life, 3 a period to keep, and each unit makes 100 a period.
    fn mill() -> Plant {
        Plant { life: 5, upkeep_per_period: 3.0, capacity_per_period: 100.0 }
    }

    fn bought(units: f64, cost: f64, when: u32) -> Vintage {
        Vintage { units, cost_per_unit: cost, in_service: when }
    }

    #[test]
    fn the_charge_is_against_the_stock_and_not_against_revenue() {
        // One schedule, charged in both places. A charge struck as a share of revenue means a firm
        // that doubles its plant takes no extra charge — so doubling the plant here doubles it.
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
        // The charge stops when the plant is gone. Law 6: not a floor under the charge — the plant
        // was consumed, which is arithmetic about a thing that ran out.
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
        // 37 F5.a: costs no batch absorbed are period costs. Nothing in this read asks what the
        // line made, because the answer would not change what the plant costs to keep.
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
        // There is a lag between the spend and the capacity. The second vintage is bought and not
        // yet in service in period 1, and a plant that is not working makes nothing.
        assert_eq!(capacity(&stock, &p, 1), 1_000.0);
        assert_eq!(capacity(&stock, &p, 3), 1_400.0);
        // By period 5 the first vintage is worn out and only the second is making anything.
        assert_eq!(capacity(&stock, &p, 5), 400.0);
    }
}
