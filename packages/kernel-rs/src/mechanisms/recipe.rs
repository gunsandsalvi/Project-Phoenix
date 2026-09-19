//! THE RECIPE: fixed input quantities per unit of output, and the production decision that draws on
//! them. Last in the sequence, deliberately — changing input-output relationships moves EVERY
//! quantity in the model, so it lands against a stable measurement and the comparison across the
//! change is the whole point of it.
//!
//! @spec 37 A1 · 37 A2 · 37 A2.a · 37 A2.b · 37 A2.c · 37 A3 · 37 A4 · 37 B1 · 37 B1.a · 37 B1.b ·
//! @spec 37 B1.c · 37 B1.d · 37 B2 · 37 B3 · 37 B4 · 37 B5 · 37 B5.a · 37 B5.b · Law 2, Law 6,
//! @spec Law 8, Law 19 · Appendix B
//!
//! A Leontief recipe, no substitution. Chosen deliberately, for one reason and with one
//! cost stated: fixed coefficients make an input shortage bite as a REAL PRODUCTION CONSTRAINT rather
//! than being smoothed away by a substitution elasticity nobody can observe — which is what makes a
//! supply shock transmit at all. The cost is that a firm facing an expensive input cannot economise
//! on it, so substitution is a MISSING mechanism here, not an assumption away: if a relative-price
//! response is wanted later it is a new mechanism, not a parameter.
//!
//! A recipe is not a value share. Expressed as cost per unit of revenue, with the physical
//! draw computed as money needed divided by the input's price, a price doubling halves the physical
//! draw — the strongest substitution assumption there is, sitting exactly where the model chose no
//! substitution at all, and invisible because it reads as an ordinary units calculation. So `Recipe`
//! holds QUANTITIES per unit of output and there is no price anywhere in it.
//!
//! The quantity is the OUTCOME. The firm decides from its reasons — expected demand, its
//! margin, its capacity, its inputs on hand, its labour — and what it makes is what those allow.
//! Utilisation is a read of the outcome against capacity and never an input to it.
//!
//! Not everything started is finished. Scrap is a loss of UNITS at the point they would have
//! been made, and what survives is dearer per unit because normal waste is absorbed into the cost of
//! the survivors. Law 6: yield is a fact about the line, and the survivors are what arithmetic leaves.

use crate::ids::{InstrumentId, PartyId};

/// Fixed input quantities per unit of output. A TECHNOLOGY primitive — a fact about
/// how the thing is made, imported from the real world, and not an equilibrium.
#[derive(Clone, Debug)]
pub struct Recipe {
    pub makes: InstrumentId,
    /// Quantities, in the input's own physical unit. No prices, no shares, no money.
    pub per_unit: Vec<(InstrumentId, f64)>,
    /// Plus labour, plus capital services. Both are draws per unit like any other.
    pub labour_per_unit: f64,
    pub capital_services_per_unit: f64,
    /// Not everything started is finished. The share of starts that survive — a fact about the
    /// line, never a target.
    pub yields: f64,
    /// The smallest run of the line — a furnace charge, a print run, a shift. A TECHNOLOGY
    /// primitive: a fact about how the thing is made, and the reason a small order is not simply a
    /// small run. A line whose batch is one unit is a line with no batch, which is a real kind of
    /// line and is said by declaring one rather than by leaving it out.
    pub batch: f64,
    /// How long the line takes, in periods. A TECHNOLOGY primitive like the batch, and the
    /// reason work in progress exists at all: between the input going in and the output coming out
    /// there is a thing, owned by somebody, carrying what it cost. A line that takes one
    /// period still has an in-between — it started last period and finishes this one — and a line
    /// that took none would be a line where nothing is ever being made.
    pub periods_to_make: u32,
}

impl Recipe {
    pub fn new(
        makes: InstrumentId,
        per_unit: Vec<(InstrumentId, f64)>,
        labour_per_unit: f64,
        capital_services_per_unit: f64,
        yields: f64,
        batch: f64,
        periods_to_make: u32,
    ) -> Recipe {
        assert!(
            yields > 0.0 && yields <= 1.0,
            "37 B4: a line that yields {yields} of what it starts is not a line"
        );
        assert!(batch > 0.0, "37 B5.b: a line whose smallest run is {batch} cannot be run at all");
        assert!(
            periods_to_make > 0,
            "37 B3: a line that takes no time has nothing between its input and its output"
        );
        Recipe { makes, per_unit, labour_per_unit, capital_services_per_unit, yields, batch, periods_to_make }
    }

    /// Production consumes the inputs it consumes — the physical consequence of the decision,
    /// and the recipe says how much. Never a separately chosen number.
    ///
    /// The draw is against what is STARTED, because scrap consumed its inputs too.
    pub fn draws_for(&self, starts: f64) -> Vec<(InstrumentId, f64)> {
        self.per_unit.iter().map(|(what, per)| (*what, per * starts)).collect()
    }

    /// What arrives at the end of the line.
    pub fn finishes(&self, starts: f64) -> f64 {
        starts * self.yields
    }

    /// 21i, 33 A4: THE SAME LINE, RUN WHERE THIS MUCH ALREADY STANDS. Building a structure
    /// somewhere already built-up draws more of everything — deeper foundations, longer haulage, more
    /// hours on a constrained site — so the recipe's requirement is a function of the place rather
    /// than a constant.
    ///
    /// It is ONE writer of the scaling. Every read downstream — what the firm can afford
    /// to start, what actually leaves its rows, what the batch cost — comes off this one object, so
    /// the decision and the draw cannot disagree about where the line is standing.
    ///
    /// What it does NOT touch is the yield, the batch or the lead time: a crowded site does not spoil
    /// more of what it makes, does not change the smallest run of the line, and is a claim about cost
    /// rather than about time. And it sets no money number at all — what the extra draw COSTS is
    /// whatever those inputs cleared at in their own books.
    ///
    /// A factor of one is the same recipe, which is what a line that stands nowhere gets: flour is
    /// milled alike everywhere.
    pub fn where_it_stands(&self, crowding: f64) -> Recipe {
        assert!(
            crowding >= 1.0,
            "21i: building on {crowding} of what it takes on empty ground is a place that pays you to build"
        );
        Recipe {
            makes: self.makes,
            per_unit: self.per_unit.iter().map(|(what, per)| (*what, per * crowding)).collect(),
            labour_per_unit: self.labour_per_unit * crowding,
            capital_services_per_unit: self.capital_services_per_unit * crowding,
            yields: self.yields,
            batch: self.batch,
            periods_to_make: self.periods_to_make,
        }
    }
}

/// The reasons a firm has. Each is a real state it can read; none of them is the quantity.
#[derive(Clone, Debug)]
pub struct Reasons {
    pub firm: PartyId,
    /// Its own outlook, never a model forecast.
    pub expected_demand: f64,
    /// Capacity is one of the reasons, and binding capacity is a real state.
    pub capacity: f64,
    /// Inputs on hand, in the same units the recipe draws in. A shortage is a real state that
    /// reaches the decision — a constraint computed and read by nobody is not a constraint.
    pub on_hand: Vec<(InstrumentId, f64)>,
    /// Labour available, from the engagement rows.
    pub labour: f64,
    /// 37 B1, 22c.3: WHAT IT ALREADY HAS ON THE SHELF. A firm that cannot see its own unsold
    /// stock decides as if every period started empty.
    pub on_shelf: f64,
    /// 37 B1, 22c.3: how much cover it wants, as a multiple of what it expects to sell. A
    /// PREFERENCE, its own and dispersed — a firm that wants two weeks' cover is not a firm
    /// that wants none, and the difference is a real thing about how it runs.
    ///
    /// The desired buffer used to be exactly ZERO and nothing said so: `wanted = expected / yields`
    /// is a stock-adjustment rule whose target stock is nothing at all. A firm holding anything above
    /// what it expected to sell therefore started nothing, for ever — `batch 0, bound demand`, while
    /// 1,648 lots perished on its own shelf.
    pub cover: f64,
}

/// What the decision came to, and which reason bound — so a reader can say why the line ran short
/// rather than inferring it. B1.a and B1.b are states, and a state nobody can name is not one.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Demand,
    Capacity,
    Inputs,
    Labour,
    /// What the firm could otherwise have run does not reach one batch of the line, so the
    /// line does not run. It is not a small production decision; it is the absence of one.
    Batch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decided {
    pub starts: f64,
    pub finishes: f64,
    pub bound: Bound,
}

/// A line may be made more than one way. Two ways of making the same good are two
/// TECHNOLOGIES, not a parameter — one may be labour-heavy and cheap on inputs, another the reverse,
/// and which is better is not a fact about the line at all. It is a fact about the PRICES the firm
/// is facing, and those differ between firms and between periods, which is why the choice belongs to
/// the firm and not to this declaration.
#[derive(Clone, Debug)]
pub struct Line {
    pub makes: InstrumentId,
    /// Each way is a full Leontief recipe in its own right. There is no blending of two ways
    /// into one, because blending IS substitution, and the model chose none.
    pub ways: Vec<Recipe>,
}

impl Line {
    pub fn new(makes: InstrumentId, ways: Vec<Recipe>) -> Line {
        assert!(!ways.is_empty(), "37 A2: a line with no way of making it is not a line");
        for w in &ways {
            assert!(
                w.makes == makes,
                "Law 4: a way of making something else is not a way of making this line"
            );
        }
        Line { makes, ways }
    }
}

/// What one way costs THIS firm to make one unit that survives, at the prices it can see.
///
/// Inputs consumed plus wages plus a capital charge, divided by what finishes rather than by
/// what starts — because normal waste is absorbed into the cost of the survivors. The division
/// is where that happens; nobody applies a markup for it.
///
/// `None` where an input has never printed. A firm cannot cost a way it cannot price, and a way
/// it cannot cost is one it cannot choose. Appendix A: that is missing, not zero — reading an
/// unpriced input as free would make the way with the most unpriced inputs look cheapest, which is
/// the defect exactly inverted.
pub fn costs(r: &Recipe, priced: &impl Fn(InstrumentId) -> Option<f64>, wage: f64, capital_service: f64) -> Option<f64> {
    let mut inputs = 0.0;
    for (what, per) in &r.per_unit {
        match priced(*what) {
            Some(price) => inputs += per * price,
            None => return None,
        }
    }
    let per_start = inputs + r.labour_per_unit * wage + r.capital_services_per_unit * capital_service;
    Some(per_start / r.yields)
}

/// The firm picks the way that costs IT least, at the prices IT is facing.
///
/// This is not a rule about which way is better — there is no such fact. Two firms looking at
/// different prices pick differently, and a firm picks differently when a price moves, which is the
/// whole reason a line has more than one way of being made. What is declared is the ways; which one
/// runs is an OUTCOME.
///
/// A tie goes to the way declared first, and that is arbitrary rather than economic: two ways that
/// cost the same are two ways that cost the same, and inventing a preference between them would be a
/// decision nobody made. `None` where the firm can price no way at all.
pub fn picks<'a>(
    line: &'a Line,
    priced: &impl Fn(InstrumentId) -> Option<f64>,
    wage: f64,
    capital_service: f64,
) -> Option<(&'a Recipe, f64)> {
    let mut best: Option<(&Recipe, f64)> = None;
    for w in &line.ways {
        let Some(c) = costs(w, priced, wage, capital_service) else {
            continue;
        };
        match best {
            Some((_, so_far)) if c >= so_far => {}
            _ => best = Some((w, c)),
        }
    }
    best
}

/// The quantity is the OUTCOME. It starts what its demand, capacity, inputs and labour all
/// allow, and the tightest of them is what bound it.
///
/// None of these is a cap imposed on a number the firm would otherwise have made — each is a
/// quantity of a real thing, and you cannot draw an input you have not got. That is arithmetic
/// impossibility, which is the only kind of limit there is.
pub fn decide(r: &Recipe, reasons: &Reasons) -> Decided {
    // What it would need to start to reach the shelf it wants, given that some of it scraps.
    //
    // The target is what it expects to sell PLUS the cover it wants, less what it already
    // has. A firm whose shelf is already above that starts nothing — which is a real decision about
    // a real stock, where before it was a firm with a desired buffer of zero that nobody had chosen.
    //
    // `wanted` can come out negative and nothing floors it. It means the firm is over-stocked
    // and there is no run of the line that would help; the batch arithmetic below turns it into
    // starting nothing, which is what being over-stocked DOES.
    let target = reasons.expected_demand * (1.0 + reasons.cover);
    let wanted = (target - reasons.on_shelf) / r.yields;

    let mut allows = wanted;
    let mut bound = Bound::Demand;
    if reasons.capacity < allows {
        allows = reasons.capacity;
        bound = Bound::Capacity;
    }
    for (what, per) in &r.per_unit {
        // An input the firm has no row for is an input it HAS NONE OF. That is not a
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
    // The line runs in whole batches. A batch is a fact about the line — a furnace charge,
    // a print run, a shift — and half of one is not a smaller run of it, it is nothing. So what the
    // reasons allow is turned into whole batches, and a firm whose reasons do not reach one batch
    // STARTS NOTHING and says so.
    //
    // This is not a floor under the quantity. A floor would raise a number to meet a
    // threshold; this lowers it to what the line can actually be run at, and the remainder is not
    // clipped away — it was never a producible quantity. It is also where operating leverage comes
    // from: the plant's upkeep is owed over whatever the batch made, so a line running one batch
    // where it could run ten costs ten times as much a unit (`throttled_cost`).
    let batches = (allows / r.batch).floor();
    let in_batches = batches * r.batch;
    if in_batches < allows {
        bound = if batches <= 0.0 { Bound::Batch } else { bound };
        allows = in_batches;
    }
    Decided { starts: allows, finishes: r.finishes(allows), bound }
}

/// Utilisation is a read of the outcome against capacity, never an input to it. `None`
/// where there is no capacity to read against.
pub fn utilisation(d: &Decided, capacity: f64) -> Option<f64> {
    if capacity <= 0.0 {
        return None;
    }
    Some(d.starts / capacity)
}

/// Work in progress exists between input and output, owned by somebody, and it carries what
/// it cost. It is a real thing with a holder, not a timing adjustment.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorkInProgress {
    pub owner: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub cost_carried: f64,
}

/// Unit cost equals inputs consumed plus wages plus a capital charge — and B4: what survives
/// is dearer per unit than what was started, because normal waste is absorbed into the cost of the
/// survivors. That is a consequence of the division, not a markup anybody applied.
///
/// `None` where nothing finished: B5.a's no units, no capitalised cost. A period in which the
/// line started nothing capitalises nothing; the firm still incurs the cost and it is a period
/// expense, which is the caller's to book — this read will not invent a unit to hang it on.
pub fn unit_cost(inputs_consumed: f64, wages: f64, capital_charge: f64, finished: f64) -> Option<f64> {
    if finished <= 0.0 {
        return None;
    }
    Some((inputs_consumed + wages + capital_charge) / finished)
}

/// A throttled period is different. A line's whole cost over a smaller batch IS a higher
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
    use crate::num::dust;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn good(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    /// Two units of input 1 and half a unit of input 2 make one unit of good 9, with 0.4 of labour
    /// and 0.1 of capital services, and nineteen starts in twenty survive.
    fn line() -> Recipe {
        Recipe::new(good(9), vec![(good(1), 2.0), (good(2), 0.5)], 0.4, 0.1, 0.95, 1.0, 1)
    }

    fn reasons(demand: f64, capacity: f64, input_1: f64, input_2: f64, labour: f64) -> Reasons {
        Reasons {
            firm: party(5),
            expected_demand: demand,
            capacity,
            on_hand: vec![(good(1), input_1), (good(2), input_2)],
            // The existing cases are about the OTHER reasons binding, so this firm starts from an
            // empty shelf and wants no cover — which is what they were written against.
            on_shelf: 0.0,
            cover: 0.0,
            labour,
        }
    }

    #[test]
    fn the_recipe_holds_quantities_and_no_price_so_a_price_doubling_draws_the_same_units() {
        // A recipe expressed as cost per unit of revenue means a price doubling HALVES the
        // physical draw — the strongest substitution assumption there is, sitting where the model
        // chose none, and invisible because it reads as an ordinary units calculation. There is no
        // price in `Recipe` to double.
        let drawn = line().draws_for(100.0);
        assert_eq!(drawn, vec![(good(1), 200.0), (good(2), 50.0)]);
    }

    #[test]
    fn an_input_shortage_bites_as_a_real_production_constraint() {
        // Fixed coefficients are what make a supply shock transmit at all. The firm wants
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
        // Each reason is a real state that reaches the decision. A constraint
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
        // What survives is dearer per unit than what was started, because normal waste is
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
        // The firm still incurs the cost, and it is a period expense. This read will not
        // invent a unit to hang it on.
        assert!(unit_cost(2_000.0, 400.0, 100.0, 0.0).is_none());
    }

    #[test]
    fn a_throttled_line_costs_more_per_unit_because_the_batch_is_smaller() {
        // A line's whole cost over a smaller batch IS a higher unit cost, and that is what
        // running a plant below its rate does.
        let at_rate = throttled_cost(10_000.0, 1_000.0).unwrap();
        let throttled = throttled_cost(10_000.0, 250.0).unwrap();
        assert_eq!(at_rate, 10.0);
        assert_eq!(throttled, 40.0);
    }

    #[test]
    fn utilisation_is_read_from_the_outcome_and_never_put_into_it() {
        // The decision above never consulted a utilisation figure; this is computed after it.
        let d = decide(&line(), &reasons(950.0, 2_000.0, 100_000.0, 100_000.0, 100_000.0));
        let u = utilisation(&d, 2_000.0).unwrap();
        assert!(u > 0.0 && u < 1.0);
        assert!(utilisation(&d, 0.0).is_none());
    }

    #[test]
    fn work_in_progress_is_owned_by_somebody_and_carries_what_it_cost() {
        // A real thing with a holder, between input and output — not a timing adjustment.
        let wip = WorkInProgress { owner: party(5), what: good(9), units: 120.0, cost_carried: 960.0 };
        assert_eq!(wip.owner, party(5));
        assert!(wip.cost_carried > 0.0);
    }

    #[test]
    #[should_panic(expected = "is not a line")]
    fn a_line_that_yields_nothing_is_not_a_line() {
        Recipe::new(good(9), vec![(good(1), 2.0)], 0.4, 0.1, 0.0, 1.0, 1);
    }

    #[test]
    #[should_panic(expected = "is not a line")]
    fn a_line_that_yields_more_than_it_starts_is_not_a_line_either() {
        Recipe::new(good(9), vec![(good(1), 2.0)], 0.4, 0.1, 1.2, 1.0, 1);
    }

    /// The same good, two ways: one that draws a lot of input 1 and little labour, one the reverse.
    fn two_ways() -> Line {
        Line::new(
            good(9),
            vec![
                Recipe::new(good(9), vec![(good(1), 4.0), (good(2), 0.5)], 0.1, 0.1, 0.95, 1.0, 1),
                Recipe::new(good(9), vec![(good(1), 1.0), (good(2), 0.5)], 2.0, 0.1, 0.95, 1.0, 1),
            ],
        )
    }

    #[test]
    fn which_way_is_better_is_a_fact_about_prices_and_not_about_the_line() {
        // Two firms facing different prices pick differently, and the same firm picks
        // differently when a price moves. That is why the ways are declared and the choice is not.
        let line = two_ways();
        let dear_input = |i: InstrumentId| if i == good(1) { Some(10.0) } else { Some(1.0) };
        let cheap_input = |i: InstrumentId| if i == good(1) { Some(0.5) } else { Some(1.0) };

        // Input 1 at 10 and an hour at 1: the input-heavy way costs 40.6 a unit, the labour-heavy
        // one 12.2. The firm takes the second.
        let (way, _) = picks(&line, &dear_input, 1.0, 1.0).unwrap();
        assert_eq!(way.labour_per_unit, 2.0);

        // The same line, the same firm, input 1 now at 0.5: the first way wins on the same read.
        let (way, _) = picks(&line, &cheap_input, 1.0, 1.0).unwrap();
        assert_eq!(way.labour_per_unit, 0.1);

        // And nothing about the LINE changed between those two reads.
        assert_eq!(line.ways.len(), 2);
    }

    #[test]
    fn a_way_the_firm_cannot_price_is_a_way_it_cannot_choose() {
        // An unpriced input is MISSING, not free. Read as zero it would make the way
        // with the most unpriced inputs look cheapest — the defect exactly inverted.
        let line = two_ways();
        let only_input_two = |i: InstrumentId| if i == good(2) { Some(1.0) } else { None };
        assert!(costs(&line.ways[0], &only_input_two, 1.0, 1.0).is_none());
        assert!(picks(&line, &only_input_two, 1.0, 1.0).is_none());
    }

    #[test]
    fn the_cost_a_firm_reads_is_the_cost_of_a_unit_that_survives() {
        // Normal waste is absorbed into the cost of the survivors, and the division is
        // where that happens. Four units of input 1 at 1, half of input 2 at 1, a tenth of an hour
        // at 1 and a tenth of capital at 1 is 4.7 a start — and 4.7/0.95 a survivor.
        let r = &two_ways().ways[0];
        let all_at_one = |_: InstrumentId| Some(1.0);
        let c = costs(r, &all_at_one, 1.0, 1.0).unwrap();
        assert!((c - 4.7 / 0.95).abs() < dust(4, &[4.7, 0.95]));
        // Scrapping less makes the same physical draw cheaper per unit sold, with no markup moved.
        let kinder = Recipe::new(good(9), r.per_unit.clone(), 0.1, 0.1, 1.0, 1.0, 1);
        assert!(costs(&kinder, &all_at_one, 1.0, 1.0).unwrap() < c);
    }

    #[test]
    #[should_panic(expected = "not a way of making this line")]
    fn a_way_of_making_something_else_is_not_one_of_this_line_s_ways() {
        // One representation per real thing. A line's ways all make the line.
        Line::new(good(9), vec![Recipe::new(good(8), vec![(good(1), 1.0)], 0.1, 0.1, 1.0, 1.0, 1)]);
    }

    /// The same line, but it can only be run fifty units at a time.
    fn in_fifties() -> Recipe {
        Recipe::new(good(9), vec![(good(1), 2.0), (good(2), 0.5)], 0.4, 0.1, 0.95, 50.0, 1)
    }

    #[test]
    fn the_line_runs_in_whole_batches_and_the_remainder_was_never_producible() {
        // Half a furnace charge is not a smaller run, it is nothing. The firm has inputs for
        // 150 and could sell more, so it runs three batches of fifty and the reasons still say
        // inputs — the batch shaped the quantity but the shortage is what bound it.
        let d = decide(&in_fifties(), &reasons(950.0, 1_000.0, 320.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 150.0);
        assert_eq!(d.bound, Bound::Inputs);
        // The 10 units of input beyond the third batch are not clipped off a quantity the
        // firm would have made — they were never a producible run, and they are still on its books.
    }

    #[test]
    fn a_firm_whose_reasons_do_not_reach_one_batch_does_not_run_the_line_at_all() {
        // It is not a small production decision, it is the absence of one — and the reader
        // can name it, which is the whole point of `Bound`.
        let d = decide(&in_fifties(), &reasons(950.0, 1_000.0, 80.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 0.0);
        assert_eq!(d.finishes, 0.0);
        assert_eq!(d.bound, Bound::Batch);
    }

    #[test]
    fn a_line_run_at_one_batch_where_it_could_run_ten_costs_ten_times_as_much_a_unit() {
        // B5.b and F5.a together: this is operating leverage. The plant's upkeep is owed over
        // whatever the batch made, so the same whole-line cost over a tenth of the output is ten
        // times the unit cost — and nobody applied a markup to make it so.
        let at_ten = throttled_cost(10_000.0, 500.0).unwrap();
        let at_one = throttled_cost(10_000.0, 50.0).unwrap();
        assert_eq!(at_one / at_ten, 10.0);
    }

    #[test]
    #[should_panic(expected = "cannot be run at all")]
    fn a_line_with_no_smallest_run_is_not_a_line() {
        Recipe::new(good(9), vec![(good(1), 2.0)], 0.4, 0.1, 0.95, 0.0, 1);
    }
}
