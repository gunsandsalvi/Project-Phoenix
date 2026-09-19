//! WHAT EACH SYSTEM DOES IN A PERIOD.
//!
//! @spec ARCHITECTURE 4.9b · Law 4, Law 5, Law 10, Law 15, Law 19 · Appendix B

use crate::ids::{InstrumentId, PartyId};
use crate::ledger::{Cause, Delivery, Leg};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{about, agreed};

/// THE FIRM PRODUCES.
struct Ran {
    maker: PartyId,
    makes: InstrumentId,
    draws: Vec<(InstrumentId, f64)>,
    finished: f64,
    /// The period it comes off the line.
    ready: u32,
    /// What went in — inputs at their own basis, wages, the capital charge.
    cost: f64,
}

pub struct Making {
    /// The flow, declared once and applied consistently — and it is the order settlement itself
    /// draws lots in, so the cost this books and the units that leave cannot disagree.
    pub flow: crate::mechanisms::goods::CostFlow,
    /// The id of the standing area at which a build draws twice, read through `params`.
    pub crowds_at: &'static str,
    /// How much COVER a firm wants on its shelf, as a multiple of what it expects to sell.
    pub cover: &'static str,
}

impl Mechanism for Making {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::ids::HoldingId;
        use crate::instruments::{capacity, charge, upkeep};
        use crate::mechanisms::goods::take;
        use crate::mechanisms::recipe::{decide, draws_for, picks, unit_cost, where_it_stands, Reasons};

        let now = ctx.period();
        // THE READ PASS.
        let mut runs: Vec<Ran> = Vec::new();

        // 21i, 33 A4: how built-up each place is — one walk over the register a period, never a
        // stored aggregate.
        let built = crate::places::built_up(ctx.parties(), ctx.register(), ctx.registry());
        let crowds_at = ctx.params().square_km(self.crowds_at);

        // How this world makes what it makes, off the registry — the one place that data lives.
        let makes: Vec<(InstrumentId, Vec<crate::registry::Way>, InstrumentId, crate::registry::Plant)> =
            ctx.registry()
                .made()
                .iter()
                .filter_map(|line| {
                    let plant = ctx.registry().made_with(*line)?;
                    let plant_is = ctx.registry().plant_of(plant)?;
                    Some((*line, ctx.registry().ways_of(*line).to_vec(), plant, plant_is))
                })
                .collect();

        for (makes_line, ways, plant, plant_is) in &makes {
            for &plant_row in ctx.register().of_instrument(*plant) {
                let plant_row = HoldingId(plant_row);
                let maker = ctx.register().holder_of(plant_row);
                if !ctx.parties().alive(maker) {
                    continue;
                }

                // A vintage IS a lot on the register, so capacity and the period's charge are
                // reads over the lots and nothing stores either.
                let stock = ctx.register().lots(plant_row).to_vec();
                let can_make = capacity(&stock, plant_is, now);
                if can_make <= 0.0 {
                    continue;
                }

                // Its own outlook, and NOT a model forecast.
                let Some(expects) = ctx.outlooks().of(maker, about::HOW_MUCH_IT_SELLS) else {
                    continue;
                };

                // The hours its engagements give it, and what an hour of them costs.
                let mut hours = 0.0;
                let mut wage_bill = 0.0;
                for a in ctx.agreements().of_party(maker) {
                    let a = crate::stores::AgreementId(*a);
                    if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::ENGAGEMENT {
                        continue;
                    }
                    let (employer, _) = ctx.agreements().between(a);
                    if employer != maker {
                        continue;
                    }
                    let terms = ctx.agreements().terms(a);
                    match (terms.first(), terms.get(1), terms.get(2)) {
                        // A wage and an hour are PER PERSON, so the line gets the headcount's worth
                        // of both.
                        (Some(w), Some(h), Some(heads)) => {
                            wage_bill += w * heads;
                            hours += h * heads;
                        }
                        // An engagement that does not say how long it is for, or for how many, buys
                        // no hours.
                        _ => continue,
                    }
                }
                if hours <= 0.0 {
                    continue;
                }
                let an_hour = wage_bill / hours;

                // B5, 33 A3: what a unit of capital service costs — the plant's own upkeep and its
                // own depreciation, over what the plant can make.
                let keeping: f64 = stock.iter().map(|v| upkeep(v, plant_is, now) + charge(v, plant_is, now)).sum();
                let a_service = keeping / can_make;

                let priced = |i: InstrumentId| {
                    let row = ctx.register().row(maker, i);
                    let lots = ctx.register().lots(row);
                    let units: f64 = lots.iter().map(|l| l.qty).sum();
                    if units > 0.0 {
                        let value: f64 = lots.iter().map(|l| l.qty * l.basis_per_unit).sum();
                        return Some(value / units);
                    }
                    ctx.prints().latest(i, now).map(|p| p.price)
                };
                let Some((way, _)) = picks(ways, &priced, an_hour, a_service) else {
                    continue;
                };
                // The same line, run where this much already stands.
                let crowding = match ctx.registry().footprint_of(*makes_line) {
                    Some(_) => crate::places::crowding(
                        crate::places::standing_in(&built, ctx.parties().region_of(maker)),
                        crowds_at,
                    ),
                    None => 1.0,
                };
                // ONE writer of the scaling.
                let way = &where_it_stands(way, crowding);

                // What it holds of each input, off its own rows.
                let on_hand: Vec<(InstrumentId, f64)> = way
                    .per_unit
                    .iter()
                    .map(|(what, _)| (*what, ctx.register().quantity(ctx.register().row(maker, *what))))
                    .collect();

                let d = decide(
                    way,
                    &Reasons {
                        firm: maker,
                        expected_demand: expects,
                        capacity: can_make,
                        on_hand,
                        labour: hours,
                        // What it already has of what it makes, off its own rows.
                        on_shelf: ctx.register().quantity(ctx.register().row(maker, *makes_line)),
                        cover: ctx.params().ratio(self.cover),
                    },
                );
                if d.starts <= 0.0 {
                    continue;
                }

                // What the draw costs, at the lots' own basis and in the order settlement will draw
                // them in — so what this books and what leaves cannot disagree.
                let draws = draws_for(way, d.starts);
                let mut inputs_cost = 0.0;
                for (what, units) in &draws {
                    let held = ctx.register().lots(ctx.register().row(maker, *what));
                    inputs_cost += take(held, *units, self.flow).cost;
                }
                let wages = d.starts * way.labour_per_unit * an_hour;
                let capital = d.starts * way.capital_services_per_unit * a_service;
                // No units, no capitalised cost.
                if unit_cost(inputs_cost, wages, capital, d.finishes).is_none() {
                    continue;
                }
                // What goes ON the line now, and when it comes off.
                runs.push(Ran {
                    maker,
                    makes: *makes_line,
                    draws,
                    finished: d.finishes,
                    ready: now + way.periods_to_make,
                    cost: inputs_cost + wages + capital,
                });
            }
        }

        // WHAT COMES OFF THE LINE.
        let due: Vec<(crate::stores::BatchId, PartyId, InstrumentId, f64, f64)> = ctx
            .making()
            .ready_in(now)
            .into_iter()
            .map(|b| {
                let m = ctx.making();
                (b, m.owner_of(b), m.what(b), m.units(b), m.cost_carried(b))
            })
            .collect();
        for (batch, maker, makes, units, cost) in due {
            if !ctx.parties().alive(maker) {
                continue;
            }
            // What a unit cost is what went in over what came out — the cost the batch carried.
            let per_unit = cost / units;
            // A batch that made nothing is not a batch that came into existence.
            let Some(made) = crate::ledger::Units::new(units) else { continue };
            ctx.propose(
                vec![Leg::Create { party: maker, instrument: makes, qty: made, cost_per_unit: per_unit }],
                Cause::Production,
                Delivery::Nothing,
                "the batches that came off the line this period",
            );
            ctx.finishes(batch);
        }

        // THE STARTS.
        for Ran { maker, makes, draws, finished, ready, cost } in runs {
            let legs: Vec<Leg> = draws
                .iter()
                .filter_map(|(what, qty)| {
                    Some(Leg::Destroy {
                        party: maker,
                        instrument: *what,
                        qty: crate::ledger::Units::new(*qty)?,
                        why: crate::ledger::Gone::Consumed,
                    })
                })
                .collect();
            ctx.propose(legs, Cause::Production, Delivery::Nothing, "the inputs the line drew this period");
            ctx.starts(maker, makes, finished, cost, ready);
        }
    }
}

