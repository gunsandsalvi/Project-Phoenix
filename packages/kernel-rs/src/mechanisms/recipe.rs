//! THE RECIPE: fixed input quantities per unit of output, and the production decision that draws on
//! them. Last in the sequence, deliberately — changing input-output relationships moves EVERY
//! quantity in the model, so it lands against a stable measurement and the comparison across the
//! change is the whole point of it.
//!
//! @spec 37 A1 · 37 A2 · 37 A2.a · 37 A2.b · 37 A2.c · 37 A3 · 37 A4 · 37 B1 · 37 B1.a · 37 B1.b ·
//! @spec 37 B1.c · 37 B1.d · 37 B2 · 37 B3 · 37 B4 · 37 B5 · 37 B5.a · 37 B5.b · Law 2, Law 6,
//! @spec Law 8, Law 19 · Appendix B
//!
//! **A Leontief recipe, no substitution** (A2.a). Chosen deliberately, for one reason and with one
//! cost stated: fixed coefficients make an input shortage bite as a REAL PRODUCTION CONSTRAINT rather
//! than being smoothed away by a substitution elasticity nobody can observe — which is what makes a
//! supply shock transmit at all. The cost is that a firm facing an expensive input cannot economise
//! on it, so **substitution is a MISSING mechanism here, not an assumption away**: if a relative-price
//! response is wanted later it is a new mechanism, not a parameter.
//!
//! **A recipe is not a value share** (A2.b). Expressed as cost per unit of revenue, with the physical
//! draw computed as money needed divided by the input's price, **a price doubling halves the physical
//! draw** — the strongest substitution assumption there is, sitting exactly where the model chose no
//! substitution at all, and invisible because it reads as an ordinary units calculation. So `Recipe`
//! holds QUANTITIES per unit of output and there is no price anywhere in it.
//!
//! **The quantity is the OUTCOME** (B1). The firm decides from its reasons — expected demand, its
//! margin, its capacity, its inputs on hand, its labour — and what it makes is what those allow.
//! Utilisation is a read of the outcome against capacity and never an input to it (B1.d).
//!
//! **Not everything started is finished** (B4). Scrap is a loss of UNITS at the point they would have
//! been made, and what survives is dearer per unit because normal waste is absorbed into the cost of
//! the survivors. Law 6: yield is a fact about the line, and the survivors are what arithmetic leaves.

use crate::ids::{InstrumentId, PartyId};

/// A2: fixed input quantities **per unit of output**. A TECHNOLOGY primitive (Law 2) — a fact about
/// how the thing is made, imported from the real world, and not an equilibrium.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub makes: InstrumentId,
    /// A2.a: quantities, in the input's own physical unit (Law 8). No prices, no shares, no money.
    pub per_unit: Vec<(InstrumentId, f64)>,
    /// A2.c: plus labour, plus capital services. Both are draws per unit like any other.
    pub labour_per_unit: f64,
    pub capital_services_per_unit: f64,
    /// B4: not everything started is finished. The share of starts that survive — a fact about the
    /// line, never a target.
    pub yields: f64,
}

impl Recipe {
    pub fn new(
        makes: InstrumentId,
        per_unit: Vec<(InstrumentId, f64)>,
        labour_per_unit: f64,
        capital_services_per_unit: f64,
        yields: f64,
    ) -> Recipe {
        assert!(
            yields > 0.0 && yields <= 1.0,
            "37 B4: a line that yields {yields} of what it starts is not a line"
        );
        Recipe { makes, per_unit, labour_per_unit, capital_services_per_unit, yields }
    }

    /// B2: **production consumes the inputs it consumes** — the physical consequence of the decision,
    /// and the recipe says how much. Never a separately chosen number.
    ///
    /// The draw is against what is STARTED, because scrap consumed its inputs too.
    pub fn draws_for(&self, starts: f64) -> Vec<(InstrumentId, f64)> {
        self.per_unit.iter().map(|(what, per)| (*what, per * starts)).collect()
    }

    /// B4: what arrives at the end of the line.
    pub fn finishes(&self, starts: f64) -> f64 {
        starts * self.yields
    }
}

/// B1: the reasons a firm has. Each is a real state it can read; none of them is the quantity.
#[derive(Clone, Debug)]
pub struct Reasons {
    pub firm: PartyId,
    /// Its own outlook (§46), never a model forecast.
    pub expected_demand: f64,
    /// B1.a: capacity is one of the reasons, and **binding capacity is a real state**.
    pub capacity: f64,
    /// B1.b: inputs on hand, in the same units the recipe draws in. **A shortage is a real state that
    /// reaches the decision** — a constraint computed and read by nobody is not a constraint.
    pub on_hand: Vec<(InstrumentId, f64)>,
    /// B1.c: labour available, from the engagement rows.
    pub labour: f64,
}

/// What the decision came to, and **which reason bound** — so a reader can say why the line ran short
/// rather than inferring it. B1.a and B1.b are states, and a state nobody can name is not one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Demand,
    Capacity,
    Inputs,
    Labour,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decided {
    pub starts: f64,
    pub finishes: f64,
    pub bound: Bound,
}

/// B1: **the quantity is the OUTCOME.** It starts what its demand, capacity, inputs and labour all
/// allow, and the tightest of them is what bound it.
///
/// Law 6: none of these is a cap imposed on a number the firm would otherwise have made — each is a
/// quantity of a real thing, and you cannot draw an input you have not got. That is arithmetic
/// impossibility, which is the only kind of limit there is.
pub fn decide(r: &Recipe, reasons: &Reasons) -> Decided {
    // What it would need to start to meet what it expects to sell, given that some of it scraps.
    let wanted = reasons.expected_demand / r.yields;

    let mut allows = wanted;
    let mut bound = Bound::Demand;
    if reasons.capacity < allows {
        allows = reasons.capacity;
        bound = Bound::Capacity;
    }
    for (what, per) in &r.per_unit {
        // Appendix A: an input the firm has no row for is an input it HAS NONE OF. That is not a
        // missing number needing a default — the register answered, and the answer was nothing.
        let held_none_of_it = 0.0;
        let have = match reasons.on_hand.iter().find(|(input, _)| input == what) {
            Some((_, units)) => *units,
            None => held_none_of_it,
        };
        let from_this = have / per;
        if from_this < allows {
            allows = from_this;
            bound = Bound::Inputs;
        }
    }
    if r.labour_per_unit > 0.0 {
        let from_labour = reasons.labour / r.labour_per_unit;
        if from_labour < allows {
            allows = from_labour;
            bound = Bound::Labour;
        }
    }
    Decided { starts: allows, finishes: r.finishes(allows), bound }
}

/// B1.d: **utilisation is a read of the outcome against capacity, never an input to it.** `None`
/// where there is no capacity to read against.
pub fn utilisation(d: &Decided, capacity: f64) -> Option<f64> {
    if capacity <= 0.0 {
        return None;
    }
    Some(d.starts / capacity)
}

/// B3: **work in progress exists between input and output**, owned by somebody, and it carries what
/// it cost. It is a real thing with a holder, not a timing adjustment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorkInProgress {
    pub owner: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub cost_carried: f64,
}

/// B5: **unit cost equals inputs consumed plus wages plus a capital charge** — and B4: what survives
/// is dearer per unit than what was started, because normal waste is absorbed into the cost of the
/// survivors. That is a consequence of the division, not a markup anybody applied.
///
/// `None` where nothing finished: B5.a's **no units, no capitalised cost**. A period in which the
/// line started nothing capitalises nothing; the firm still incurs the cost and it is a period
/// expense, which is the caller's to book — this read will not invent a unit to hang it on.
pub fn unit_cost(inputs_consumed: f64, wages: f64, capital_charge: f64, finished: f64) -> Option<f64> {
    if finished <= 0.0 {
        return None;
    }
    Some((inputs_consumed + wages + capital_charge) / finished)
}

/// B5.b: **a throttled period is different.** A line's whole cost over a smaller batch IS a higher
/// unit cost, and that is what running a plant below its rate does. The read is the same read; what
/// changes is the batch.
pub fn throttled_cost(whole_line_cost: f64, batch: f64) -> Option<f64> {
    if batch <= 0.0 {
        return None;
    }
    Some(whole_line_cost / batch)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn good(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    /// Two units of input 1 and half a unit of input 2 make one unit of good 9, with 0.4 of labour
    /// and 0.1 of capital services, and nineteen starts in twenty survive.
    fn line() -> Recipe {
        Recipe::new(good(9), vec![(good(1), 2.0), (good(2), 0.5)], 0.4, 0.1, 0.95)
    }

    fn reasons(demand: f64, capacity: f64, input_1: f64, input_2: f64, labour: f64) -> Reasons {
        Reasons {
            firm: party(5),
            expected_demand: demand,
            capacity,
            on_hand: vec![(good(1), input_1), (good(2), input_2)],
            labour,
        }
    }

    #[test]
    fn the_recipe_holds_quantities_and_no_price_so_a_price_doubling_draws_the_same_units() {
        // A2.b: a recipe expressed as cost per unit of revenue means a price doubling HALVES the
        // physical draw — the strongest substitution assumption there is, sitting where the model
        // chose none, and invisible because it reads as an ordinary units calculation. There is no
        // price in `Recipe` to double.
        let drawn = line().draws_for(100.0);
        assert_eq!(drawn, vec![(good(1), 200.0), (good(2), 50.0)]);
    }

    #[test]
    fn an_input_shortage_bites_as_a_real_production_constraint() {
        // A2.a: fixed coefficients are what make a supply shock transmit at all. The firm wants
        // 1,000, has capacity for 1,000 and labour for 1,000, and has 300 of an input it needs two
        // of per unit — so it starts 150, and the reason is nameable.
        let d = decide(&line(), &reasons(950.0, 1_000.0, 300.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 150.0);
        assert_eq!(d.bound, Bound::Inputs);
        // A firm facing an expensive input cannot economise on it: substitution is MISSING here, not
        // assumed away, and nothing in this module quietly supplies it.
    }

    #[test]
    fn the_quantity_is_the_outcome_and_which_reason_bound_is_nameable() {
        // B1, B1.a, B1.b, B1.c: each reason is a real state that reaches the decision. A constraint
        // computed and read by nobody is not a constraint.
        let plenty = reasons(950.0, 10_000.0, 100_000.0, 100_000.0, 100_000.0);
        assert_eq!(decide(&line(), &plenty).bound, Bound::Demand);
        let cramped = Reasons { capacity: 400.0, ..plenty.clone() };
        assert_eq!(decide(&line(), &cramped).bound, Bound::Capacity);
        let short_handed = Reasons { labour: 80.0, ..plenty.clone() };
        let d = decide(&line(), &short_handed);
        assert_eq!(d.bound, Bound::Labour);
        assert_eq!(d.starts, 200.0);
    }

    #[test]
    fn an_input_the_firm_has_no_row_for_is_an_input_it_has_none_of() {
        // The register answered and the answer was nothing — which stops the line, as it should.
        let missing = Reasons {
            on_hand: vec![(good(1), 100_000.0)],
            ..reasons(950.0, 10_000.0, 0.0, 0.0, 100_000.0)
        };
        let d = decide(&line(), &missing);
        assert_eq!(d.starts, 0.0);
        assert_eq!(d.bound, Bound::Inputs);
    }

    #[test]
    fn scrap_is_a_loss_of_units_at_the_point_they_would_have_been_made() {
        // B4: what survives is dearer per unit than what was started, because normal waste is
        // absorbed into the cost of the survivors — a consequence of the division, not a markup.
        let r = line();
        let starts = 1_000.0;
        assert_eq!(r.finishes(starts), 950.0);
        // The draw is against what was STARTED, because the scrap consumed its inputs too.
        assert_eq!(r.draws_for(starts)[0].1, 2_000.0);
        let dearer = unit_cost(2_000.0, 400.0, 100.0, r.finishes(starts)).unwrap();
        let if_nothing_scrapped = unit_cost(2_000.0, 400.0, 100.0, starts).unwrap();
        assert!(dearer > if_nothing_scrapped);
    }

    #[test]
    fn a_period_that_started_nothing_capitalises_nothing() {
        // B5.a: the firm still incurs the cost, and it is a period expense. This read will not
        // invent a unit to hang it on.
        assert!(unit_cost(2_000.0, 400.0, 100.0, 0.0).is_none());
    }

    #[test]
    fn a_throttled_line_costs_more_per_unit_because_the_batch_is_smaller() {
        // B5.b: a line's whole cost over a smaller batch IS a higher unit cost, and that is what
        // running a plant below its rate does.
        let at_rate = throttled_cost(10_000.0, 1_000.0).unwrap();
        let throttled = throttled_cost(10_000.0, 250.0).unwrap();
        assert_eq!(at_rate, 10.0);
        assert_eq!(throttled, 40.0);
    }

    #[test]
    fn utilisation_is_read_from_the_outcome_and_never_put_into_it() {
        // B1.d. The decision above never consulted a utilisation figure; this is computed after it.
        let d = decide(&line(), &reasons(950.0, 2_000.0, 100_000.0, 100_000.0, 100_000.0));
        let u = utilisation(&d, 2_000.0).unwrap();
        assert!(u > 0.0 && u < 1.0);
        assert!(utilisation(&d, 0.0).is_none());
    }

    #[test]
    fn work_in_progress_is_owned_by_somebody_and_carries_what_it_cost() {
        // B3: a real thing with a holder, between input and output — not a timing adjustment.
        let wip = WorkInProgress { owner: party(5), what: good(9), units: 120.0, cost_carried: 960.0 };
        assert_eq!(wip.owner, party(5));
        assert!(wip.cost_carried > 0.0);
    }

    #[test]
    #[should_panic(expected = "is not a line")]
    fn a_line_that_yields_nothing_is_not_a_line() {
        Recipe::new(good(9), vec![(good(1), 2.0)], 0.4, 0.1, 0.0);
    }

    #[test]
    #[should_panic(expected = "is not a line")]
    fn a_line_that_yields_more_than_it_starts_is_not_a_line_either() {
        Recipe::new(good(9), vec![(good(1), 2.0)], 0.4, 0.1, 1.2);
    }
}
