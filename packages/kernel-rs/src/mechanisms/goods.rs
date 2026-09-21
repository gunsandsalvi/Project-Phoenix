//! THE GOODS: how a thing is MADE — fixed input quantities per unit of output and the production
//! decision that draws on them — and how it is SOLD, where a price clears per (good, market,
//! period), unsold output stays with the seller, and what is held is held as LOTS with what each
//! cost.
//!
//! @spec 37 A1 · 37 A2 · 37 A2.a · 37 A2.b · 37 A2.c · 37 A3 · 37 A4 · 37 B1 · 37 B1.a · 37 B1.b ·
//! @spec 37 B1.c · 37 B1.d · 37 B2 · 37 B3 · 37 B4 · 37 B5 · 37 B5.a · 37 B5.b · 37 C1 · 37 C2 ·
//! @spec 37 C3 · 37 C4 · 37 C5 · 37 C6 · 37 D1 · 37 D2 · 37 D3 · 37 D4 · 37 D5 · 37 E1 · 37 E2 ·
//! @spec 37 E2.a · 37 E2.b · 37 E2.c · 37 E3 · 37 E4 · 37 E4.a · 37 E5 · 37 F1 · 37 F4 · 37 F5 ·
//! @spec 37 F5.a · 37 F5.b · XI-12 · Law 2, Law 3, Law 4, Law 5, Law 6, Law 8, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::ids::CurrencyCode;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{InstrumentId, MarketId, PartyId};
use crate::module::{Participant, ParticipantView};
use crate::params::Denomination;
use crate::instruments::{capacity, settled_charge as wears, upkeep, Class};
use crate::ledger::{Cause, Delivery, Gone, Leg};
use crate::module::{Mechanism, MechanismContext};
use crate::register::Lot;
use crate::registry::Way;
use crate::stores::{about, agreed};

fn installed(acquired: u32, construction_began: Option<u32>) -> bool {
    match construction_began {
        Some(began) => acquired < began,
        None => true,
    }
}


/// Production consumes the inputs it consumes — the physical consequence of the decision, and the
/// way says how much.
pub fn draws_for(r: &Way, starts: f64) -> Vec<(InstrumentId, f64)> {
    r.per_unit.iter().map(|(what, per)| (*what, per * starts)).collect()
}

/// What arrives at the end of the line.
pub fn finishes(r: &Way, starts: f64) -> f64 {
    starts * r.yields
}

/// 21i, 33 A4: THE SAME LINE, RUN WHERE THIS MUCH ALREADY STANDS.
pub fn where_it_stands(r: &Way, crowding: f64) -> Way {
    assert!(
        crowding >= 1.0,
        "21i: building on {crowding} of what it takes on empty ground is a place that pays you to build"
    );
    Way {
        per_unit: r.per_unit.iter().map(|(what, per)| (*what, per * crowding)).collect(),
        labour_per_unit: r.labour_per_unit * crowding,
        capital_services_per_unit: r.capital_services_per_unit * crowding,
        yields: r.yields,
        batch: r.batch,
        periods_to_make: r.periods_to_make,
    }
}

/// The reasons a firm has.
#[derive(Clone, Debug)]
pub struct Reasons {
    pub firm: PartyId,
    /// Its own outlook, never a model forecast.
    pub expected_demand: f64,
    /// Capacity is one of the reasons, and binding capacity is a real state.
    pub capacity: f64,
    /// Inputs on hand, in the same units the recipe draws in.
    pub on_hand: Vec<(InstrumentId, f64)>,
    /// Labour available, from the engagement rows.
    pub labour: f64,
    /// 37 B1, 22c.3: WHAT IT ALREADY HAS ON THE SHELF.
    pub on_shelf: f64,
    /// 37 B1, 22c.3: how much cover it wants, as a multiple of what it expects to sell.
    pub cover: f64,
}

/// What the decision came to, and which reason bound — so a reader can say why the line ran short
/// rather than inferring it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bound {
    Demand,
    Capacity,
    Inputs,
    Labour,
    /// What the firm could otherwise have run does not reach one batch of the line, so the line does
    /// not run.
    Batch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decided {
    pub starts: f64,
    pub finishes: f64,
    pub bound: Bound,
}

/// What one way costs THIS firm to make one unit that survives, at the prices it can see.
pub fn costs(r: &Way, priced: &impl Fn(InstrumentId) -> Option<f64>, wage: f64, capital_service: f64) -> Option<f64> {
    let mut inputs = 0.0;
    for (what, per) in &r.per_unit {
        inputs += per * priced(*what)?;
    }
    let per_start = inputs + r.labour_per_unit * wage + r.capital_services_per_unit * capital_service;
    Some(per_start / r.yields)
}

/// The firm picks the way that costs IT least, at the prices IT is facing.
pub fn picks<'a>(
    ways: &'a [Way],
    priced: &impl Fn(InstrumentId) -> Option<f64>,
    wage: f64,
    capital_service: f64,
) -> Option<(&'a Way, f64)> {
    let mut best: Option<(&Way, f64)> = None;
    for w in ways {
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

/// The quantity is the OUTCOME.
pub fn decide(r: &Way, reasons: &Reasons) -> Decided {
    // What it would need to start to reach the shelf it wants, given that some of it scraps.
    let target = reasons.expected_demand * (1.0 + reasons.cover);
    let wanted = (target - reasons.on_shelf) / r.yields;

    let mut allows = wanted;
    let mut bound = Bound::Demand;
    if r.capital_services_per_unit > 0.0 {
        let from_capital = reasons.capacity / r.capital_services_per_unit;
        if from_capital < allows {
            allows = from_capital;
            bound = Bound::Capacity;
        }
    }
    for (what, per) in &r.per_unit {
        // An input the firm has no row for is an input it HAS NONE OF.
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
    // The line runs in whole batches.
    let batches = (allows / r.batch).floor();
    let in_batches = batches * r.batch;
    if in_batches < allows {
        bound = if batches <= 0.0 { Bound::Batch } else { bound };
        allows = in_batches;
    }
    Decided { starts: allows, finishes: finishes(r, allows), bound }
}

/// Utilisation is a read of the outcome against capacity, never an input to it.
pub fn utilisation(d: &Decided, capital_services_per_unit: f64, capacity: f64) -> Option<f64> {
    if capacity <= 0.0 {
        return None;
    }
    Some(d.starts * capital_services_per_unit / capacity)
}

/// Work in progress exists between input and output, owned by somebody, and it carries what it cost.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct WorkInProgress {
    pub owner: PartyId,
    pub what: InstrumentId,
    pub units: f64,
    pub cost_carried: f64,
}

/// Unit cost equals inputs consumed plus wages plus a capital charge — and B4: what survives is
/// dearer per unit than what was started, because normal waste is absorbed into the cost of the
/// survivors.
pub fn unit_cost(inputs_consumed: f64, wages: f64, capital_charge: f64, finished: f64) -> Option<f64> {
    if finished <= 0.0 {
        return None;
    }
    Some((inputs_consumed + wages + capital_charge) / finished)
}

/// A throttled period is different.
pub fn throttled_cost(whole_line_cost: f64, batch: f64) -> Option<f64> {
    if batch <= 0.0 {
        return None;
    }
    Some(whole_line_cost / batch)
}


/// A seller offering a quantity, and a buyer posting the most it will pay.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Offer {
    pub seller: PartyId,
    pub units: f64,
    /// What it will not go below.
    pub reservation: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Posted {
    pub buyer: PartyId,
    pub units: f64,
    /// The most it will pay.
    pub most: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    /// The print, stored so next period can re-mark against it.
    pub print: Option<f64>,
    pub traded: f64,
    /// Unsold output stays with the seller.
    pub unsold: f64,
    /// What each buyer got when demand exceeded supply.
    pub to: Vec<(PartyId, f64)>,
}

/// The cross.
pub fn clearing(posted: &[Posted], offers: &[Offer]) -> Cleared {
    let mut bids: Vec<&Posted> = posted.iter().collect();
    let mut asks: Vec<&Offer> = offers.iter().collect();
    bids.sort_by(|a, b| b.most.total_cmp(&a.most));
    asks.sort_by(|a, b| a.reservation.total_cmp(&b.reservation));

    let supply: f64 = asks.iter().map(|a| a.units).sum();
    let mut left = supply;
    let mut print = None;
    let mut to: Vec<(PartyId, f64)> = Vec::new();
    let mut at = 0usize;
    while at < bids.len() && left > 0.0 {
        let most = bids[at].most;
        // Everyone bidding this much is one tie and they share pro rata.
        let mut tie: Vec<&Posted> = Vec::new();
        while at < bids.len() && bids[at].most == most {
            tie.push(bids[at]);
            at += 1;
        }
        // A bid below what any remaining seller will take does not trade, and nor does anything
        // behind it.
        let cheapest_left = asks
            .iter()
            .find(|a| a.units > 0.0)
            .map(|a| a.reservation);
        match cheapest_left {
            Some(lowest) if most < lowest => break,
            None => break,
            _ => {}
        }
        let wanted: f64 = tie.iter().map(|b| b.units).sum();
        let taken = if wanted < left { wanted } else { left };
        for b in tie {
            let got = taken * b.units / wanted;
            if got > 0.0 {
                to.push((b.buyer, got));
                print = Some(most);
            }
        }
        left -= taken;
    }
    Cleared { print, traded: supply - left, unsold: left, to }
}

/// The price is in the seller's currency, and a foreign buyer buys that money from somebody.
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode {
    sellers_money
}

/// Moving goods takes time and costs money, a carrier is a named party that earns the freight, and
/// landed cost is ex-works plus freight plus duty.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Consignment {
    pub what: InstrumentId,
    pub units: f64,
    pub carrier: PartyId,
    /// Whose book it is on while it is in transit.
    pub owned_in_transit_by: PartyId,
    pub ex_works: f64,
    pub freight: f64,
    pub duty: f64,
    pub periods_in_transit: u32,
}

impl Consignment {
    pub fn landed_cost(&self) -> f64 {
        self.ex_works + self.freight + self.duty
    }
}

/// Cost flows first-in-first-out or by weighted average; last-in-first-out is not permitted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CostFlow {
    FirstInFirstOut,
    WeightedAverage,
}

/// Whether this holder's inventory IS its position.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CarriesAtFairValue(pub bool);

/// What left the stock and what it cost, under the stated flow.
#[derive(Clone, Debug, PartialEq)]
pub struct Consumed {
    pub units: f64,
    pub cost: f64,
    pub left: Vec<Lot>,
}

/// You cannot take out more units than are there, and the answer is what there was — arithmetic, not
/// a clamp.
pub fn take(lots: &[Lot], units: f64, flow: CostFlow) -> Consumed {
    let held: f64 = lots.iter().map(|l| l.qty).sum();
    let taking = if units < held { units } else { held };
    match flow {
        CostFlow::WeightedAverage => {
            if held <= 0.0 {
                return Consumed { units: 0.0, cost: 0.0, left: lots.to_vec() };
            }
            let value: f64 = lots.iter().map(|l| l.qty * l.basis_per_unit).sum();
            let per_unit = value / held;
            // The pooled lot carries the earliest acquisition, because that is when the stock the
            // pool is made of started being held.
            let mut earliest = lots[0].acquired;
            for l in lots {
                if l.acquired < earliest {
                    earliest = l.acquired;
                }
            }
            let left = vec![Lot { qty: held - taking, basis_per_unit: per_unit, acquired: earliest }];
            Consumed { units: taking, cost: taking * per_unit, left }
        }
        CostFlow::FirstInFirstOut => {
            let mut ordered: Vec<Lot> = lots.to_vec();
            ordered.sort_by_key(|l| l.acquired);
            let mut want = taking;
            let mut cost = 0.0;
            let mut left: Vec<Lot> = Vec::new();
            for lot in ordered {
                if want <= 0.0 {
                    left.push(lot);
                    continue;
                }
                if lot.qty <= want {
                    cost += lot.qty * lot.basis_per_unit;
                    want -= lot.qty;
                } else {
                    cost += want * lot.basis_per_unit;
                    left.push(Lot { qty: lot.qty - want, ..lot });
                    want = 0.0;
                }
            }
            Consumed { units: taking, cost, left }
        }
    }
}

/// The lower of cost and net realisable value, and the write-down is a CHARGE TO INCOME in the
/// period it happens — an event with a date, a size and an income line.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Carried {
    pub per_unit: f64,
    /// What goes to income this period.
    pub to_income: f64,
}

pub fn carry(lot: &Lot, net_realisable: f64, holder: CarriesAtFairValue) -> Carried {
    let CarriesAtFairValue(at_fair_value) = holder;
    if at_fair_value {
        return Carried {
            per_unit: net_realisable,
            to_income: (net_realisable - lot.basis_per_unit) * lot.qty,
        };
    }
    if net_realisable < lot.basis_per_unit {
        return Carried {
            per_unit: net_realisable,
            to_income: (net_realisable - lot.basis_per_unit) * lot.qty,
        };
    }
    // At or above cost: carried at cost, and nothing reaches income.
    Carried { per_unit: lot.basis_per_unit, to_income: 0.0 }
}

/// Spoilage, obsolescence and shrinkage remove units without a sale, at the lot's own cost per unit,
/// recorded so the units identity can see them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Perished {
    pub units: f64,
    pub at_cost: f64,
}

pub fn perish(lot: &Lot, share_that_perishes: f64) -> Perished {
    assert!(
        (0.0..=1.0).contains(&share_that_perishes),
        "37 E4: {share_that_perishes} of a lot perishing is not a share of it"
    );
    let units = lot.qty * share_that_perishes;
    Perished { units, at_cost: units * lot.basis_per_unit }
}

fn spoiled_inventory(
    party: PartyId,
    instrument: InstrumentId,
    lots: &[Lot],
    share: f64,
) -> Option<Leg> {
    let units: f64 = lots.iter().map(|lot| perish(lot, share).units).sum();
    let qty = crate::ledger::Units::new(units)?;
    Some(Leg::Destroy { party, instrument, qty, why: Gone::Perished })
}

/// The OTHER thing — cash, paid to a named storer (Law 5: two sides).
pub fn storage_fee(units: f64, per_unit: f64, to: PartyId) -> (PartyId, f64) {
    (to, units * per_unit)
}

/// The income statement charges what it SOLD, not what it drew.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Charged {
    /// What the units that left cost, per E5.
    pub cost_of_goods_sold: f64,
    /// What no batch absorbed.
    pub period_cost: f64,
}

pub fn charge(sold: &Consumed, line_cost: f64, absorbed_into_batches: f64) -> Charged {
    Charged {
        cost_of_goods_sold: sold.cost,
        period_cost: line_cost - absorbed_into_batches,
    }
}

/// Allocate one period charge over services actually consumed, without allowing several products
/// that share a plant to absorb the same charge again.
fn absorb_period_charge(charge: f64, service_capacity: f64, services_used: f64, already: f64) -> f64 {
    assert!(charge >= 0.0 && service_capacity > 0.0 && services_used >= 0.0 && already >= 0.0);
    let available = charge - already;
    if available <= 0.0 {
        return 0.0;
    }
    let allocated = charge * services_used / service_capacity;
    if allocated < available { allocated } else { available }
}

/// Maintenance is bought from the plant's named producer; work performed internally creates no
/// bilateral payable because payer and payee would be the same party.
fn upkeep_due(owner: PartyId, supplier: PartyId, amount: f64) -> Option<(PartyId, PartyId, f64)> {
    match amount > 0.0 && owner != supplier {
        true => Some((supplier, owner, amount)),
        false => None,
    }
}

/// WHAT §37 DOES IN A PERIOD, through the second door (ARCHITECTURE 4.9b).
pub struct Perishing {
    /// The share of a lot that does not survive the period.
    pub share: &'static str,
}

impl Mechanism for Perishing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The READ pass first, then the proposals.
        let share = ctx.params().ratio(self.share);
        let mut gone_from: Vec<Leg> = Vec::new();
        for row in ctx.register().all() {
            let line = ctx.register().instrument_of(row);
            if ctx.instruments().class_of(line) != Class::Good {
                continue;
            }
            let held = ctx.register().lots(row);
            if held.is_empty() {
                continue;
            }
            if let Some(gone) = spoiled_inventory(ctx.register().holder_of(row), line, held, share) {
                gone_from.push(gone);
            }
        }
        for gone in gone_from {
            ctx.propose(
                vec![gone],
                Cause::Production,
                Delivery::Nothing,
                "the share of the stock that did not survive the period",
            );
        }
    }
}


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

/// Turn one completed batch into producer inventory at the cost the batch carried.
fn completed_output(maker: PartyId, makes: InstrumentId, units: f64, cost: f64) -> Option<Leg> {
    let qty = crate::ledger::Units::new(units)?;
    Some(Leg::Create { party: maker, instrument: makes, qty, cost_per_unit: cost / units })
}

pub struct Making {
    /// The flow, declared once and applied consistently — and it is the order settlement itself
    /// draws lots in, so the cost this books and the units that leave cannot disagree.
    pub flow: CostFlow,
    /// The id of the standing area at which a build draws twice, read through `params`.
    pub crowds_at: &'static str,
    /// How much COVER a firm wants on its shelf, as a multiple of what it expects to sell.
    pub cover: &'static str,
}

impl Mechanism for Making {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::ids::HoldingId;

        let now = ctx.period();
        // THE READ PASS.
        let mut runs: Vec<Ran> = Vec::new();
        let mut depreciation: Vec<(PartyId, InstrumentId, f64)> = Vec::new();
        let mut depreciated = std::collections::HashSet::new();
        let mut depreciation_in_batches = std::collections::HashMap::<u32, f64>::new();
        let mut upkeep_dues: Vec<(PartyId, PartyId, crate::ids::CurrencyCode, f64)> = Vec::new();
        let mut maintained = std::collections::HashSet::new();

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
                let mut construction_began: Option<u32> = None;
                for process in ctx.processes().running(crate::stores::afoot::CAPITAL_PROGRAMME) {
                    if ctx.processes().owner(process) != maker
                        || ctx.processes().subject(process) != Some(*plant)
                    {
                        continue;
                    }
                    let began = ctx.processes().began(process);
                    construction_began = match construction_began {
                        Some(earlier) if earlier < began => Some(earlier),
                        _ => Some(began),
                    };
                }
                let stock = ctx
                    .register()
                    .lots(plant_row)
                    .iter()
                    .filter(|lot| installed(lot.acquired, construction_began))
                    .copied()
                    .collect::<Vec<_>>();
                let charge: f64 = stock.iter().map(|v| wears(v, plant_is, now)).sum();
                if charge > 0.0 && depreciated.insert(plant_row.0) {
                    depreciation.push((maker, *plant, charge));
                }
                let keeping: f64 = stock.iter().map(|v| upkeep(v, plant_is, now)).sum();
                if maintained.insert(plant_row.0) {
                    let supplier = ctx.instruments().issuer_of(*plant);
                    if let Some((payee, payer, amount)) = upkeep_due(maker, supplier, keeping) {
                        let ccy = ctx.registry().currency_of(ctx.parties().region_of(maker));
                        upkeep_dues.push((payee, payer, ccy, amount));
                    }
                }
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
                    let crate::stores::AgreementTerms::Engagement { wage_per_person, hours_per_person, heads } = ctx.agreements().terms(a) else { continue };
                    wage_bill += wage_per_person * f64::from(*heads);
                    hours += hours_per_person * f64::from(*heads);
                }
                if hours <= 0.0 {
                    continue;
                }
                let an_hour = wage_bill / hours;

                // B5, 33 A3: upkeep is priced over the services the plant can supply. Depreciation
                // is allocated separately below so two products sharing this plant cannot absorb
                // the same period charge twice.
                let a_service = (keeping + charge) / can_make;

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
                let services_used = d.starts * way.capital_services_per_unit;
                let already = match depreciation_in_batches.get(&plant_row.0) {
                    Some(amount) => *amount,
                    None => 0.0,
                };
                let absorbed = absorb_period_charge(charge, can_make, services_used, already);
                let capital = services_used * keeping / can_make + absorbed;
                // No units, no capitalised cost.
                if unit_cost(inputs_cost, wages, capital, d.finishes).is_none() {
                    continue;
                }
                depreciation_in_batches.insert(plant_row.0, already + absorbed);
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

        for (party, instrument, amount) in depreciation {
            ctx.propose(
                vec![Leg::Depreciate { party, instrument, amount }],
                Cause::Production,
                Delivery::Nothing,
                "the period's plant depreciation reduced its carrying basis",
            );
        }
        let from = ctx.today();
        let due = ctx.calendar().start_of(crate::calendar::Period(now + 1));
        for (payee, payer, ccy, amount) in upkeep_dues {
            ctx.owes(
                payee,
                payer,
                ccy,
                crate::stores::Payment { from, due, amount, of: crate::stores::Owing::Purchase },
            );
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
            // A batch that made nothing is not a batch that came into existence. What does come
            // off is credited to its producer at the cost the batch carried.
            let Some(output) = completed_output(maker, makes, units, cost) else { continue };
            ctx.propose(
                vec![output],
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


/// Sellers offer quantities.
pub struct GoodsSellers {
    /// What another period on the shelf costs it, as a share of what the units cost.
    pub holding_costs: &'static str,
    /// Whether a good is an input is the HOLDER's question, not the good's.
    pub keeps: Vec<(InstrumentId, Vec<InstrumentId>)>,
}

impl Participant for GoodsSellers {
    fn party_kind(&self) -> u32 {
        kinds::FIRM
    }

    /// 3 C2, 22c2.3: it pulls what it can no longer deliver.
    fn pulls(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<crate::stores::RestingId> {
        let Some(line) = view.subject_of(m) else { return Vec::new() };
        let (_, standing) = view.resting(m);
        let have = whole_pieces(view.free(line));
        if standing <= have {
            return Vec::new();
        }
        let mut over = standing - have;
        let mut pulling = Vec::new();
        for o in view.standing(m) {
            if over <= 0 {
                break;
            }
            let left = view.left_of(o);
            if left <= 0 {
                continue;
            }
            pulling.push(o);
            over -= left;
        }
        pulling
    }

    /// Off its OWN rows.
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        let mine: Vec<InstrumentId> = self
            .keeps
            .iter()
            .filter(|(plant, _)| view.quantity(*plant) > 0.0)
            .flat_map(|(_, inputs)| inputs.iter().copied())
            .collect();
        view.holdings()
            .map(|row| view.line_of(row))
            .filter(|line| view.quantity(*line) > 0.0 && !mine.contains(line))
            .filter_map(|line| view.market_of(line))
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else { return Vec::new() };
        // It offers what it holds IN WHOLE PIECES.
        let (_, already) = view.resting(m);
        let pieces = whole_pieces(view.free(line)) - already;
        if pieces <= 0 {
            return Vec::new();
        }
        // THE ASK IS A PRICE AND IT ANSWERS THE SHELF.
        let lots = view.lots(line);
        let units: f64 = lots.iter().map(|l| l.qty).sum();
        if units <= 0.0 {
            return Vec::new();
        }
        let cost = lots.iter().map(|l| l.qty * l.basis_per_unit).sum::<f64>() / units;
        let holding = view.params().ratio(self.holding_costs);
        let Some(expected) = view.price_outlook(line) else {
            return Vec::new();
        };
        let reservation = expected - cost * holding;
        // A price of nothing or less is not a price this seller can post: below that it would rather
        // let the stock perish than pay somebody to take it.
        if reservation <= 0.0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(reservation), qty: pieces }]
    }
}


/// SOMEBODY WHOSE BUSINESS IS TO HOLD THE STOCK.
pub struct Stockist {
    /// What it will carry.
    pub lines: Vec<InstrumentId>,
    /// What a period of holding costs it, as a share of what the units cost: the room, the spoilage
    /// and the money tied up.
    pub carrying: &'static str,
    /// What it will hold of one line.
    pub limit: &'static str,
}

impl Participant for Stockist {
    fn party_kind(&self) -> u32 {
        kinds::STOCKIST
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().filter_map(|line| view.market_of(*line)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else { return Vec::new() };
        let carrying = view.params().ratio(self.carrying);
        let limit = view.params().amount(self.limit, Denomination::Money);
        let (bidding, offering) = view.resting(m);
        let mut out = Vec::new();

        // THE SELL SIDE: what it cost, plus what carrying it has actually cost.
        let lots = view.lots(line);
        let held: f64 = lots.iter().map(|l| l.qty).sum();
        if held > 0.0 {
            let asking: f64 = lots
                .iter()
                .map(|l| {
                    let periods = f64::from(view.period().saturating_sub(l.acquired));
                    l.qty * l.basis_per_unit * (1.0 + carrying * periods)
                })
                .sum::<f64>()
                / held;
            let pieces = whole_pieces(view.free(line)) - offering;
            if pieces > 0 && asking > 0.0 {
                out.push(Order { party: view.self_id(), side: Side::Sell, price: Some(asking), qty: pieces });
            }
        }

        // THE BUY SIDE: it buys at what it expects to sell for, less what it will cost to carry.
        if let Some(print) = view.print(line) {
            let bid = print.price * (1.0 - carrying);
            // It will not carry more than its limit.
            let room = whole_pieces(limit - held) - bidding;
            let affordable = whole_pieces(view.own_cash() / bid);
            let wants = if room < affordable { room } else { affordable };
            if bid > 0.0 && wants > 0 {
                out.push(Order { party: view.self_id(), side: Side::Buy, price: Some(bid), qty: wants });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn lots() -> Vec<Lot> {
        vec![
            Lot { qty: 100.0, basis_per_unit: 4.0, acquired: 1 },
            Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 },
        ]
    }

    #[test]
    fn construction_output_is_not_installed_until_its_process_closes() {
        assert!(installed(2, None));
        assert!(installed(2, Some(3)));
        assert!(!installed(3, Some(3)));
        assert!(!installed(4, Some(3)));
    }

    #[test]
    fn unsold_output_stays_with_the_seller() {
        // Illiquidity in goods is unsold stock, and there is no buyer of last resort.
        let offers = [Offer { seller: party(1), units: 500.0, reservation: 10.0 }];
        let posted = [Posted { buyer: party(20), units: 120.0, most: 12.0 }];
        let c = clearing(&posted, &offers);
        assert_eq!(c.traded, 120.0);
        assert_eq!(c.unsold, 380.0);
        assert_eq!(c.print, Some(12.0));
    }

    #[test]
    fn a_book_where_no_bid_reaches_a_reservation_prints_nothing() {
        // Nothing is added to make it clear, and no price is invented.
        let offers = [Offer { seller: party(1), units: 500.0, reservation: 20.0 }];
        let posted = [Posted { buyer: party(20), units: 120.0, most: 12.0 }];
        let c = clearing(&posted, &offers);
        assert!(c.print.is_none());
        assert_eq!(c.traded, 0.0);
        assert_eq!(c.unsold, 500.0);
    }

    #[test]
    fn rationing_is_one_stated_rule_and_it_is_pro_rata_within_the_marginal_price() {
        // Demand exceeds supply and the rule is stated once, not per market.
        let offers = [Offer { seller: party(1), units: 90.0, reservation: 5.0 }];
        let posted = [
            Posted { buyer: party(20), units: 60.0, most: 9.0 },
            Posted { buyer: party(21), units: 120.0, most: 9.0 },
        ];
        let c = clearing(&posted, &offers);
        assert_eq!(c.traded, 90.0);
        assert_eq!(c.to[0], (party(20), 30.0));
        assert_eq!(c.to[1], (party(21), 60.0));
    }

    #[test]
    fn a_higher_bid_is_filled_before_a_lower_one() {
        // Buyers are heterogeneous and bid for their own reasons; the book sorts them.
        let offers = [Offer { seller: party(1), units: 100.0, reservation: 5.0 }];
        let posted = [
            Posted { buyer: party(20), units: 80.0, most: 6.0 },
            Posted { buyer: party(21), units: 80.0, most: 11.0 },
        ];
        let c = clearing(&posted, &offers);
        assert_eq!(c.to[0], (party(21), 80.0));
        assert_eq!(c.to[1], (party(20), 20.0));
        // The marginal buyer's bid is the print.
        assert_eq!(c.print, Some(6.0));
    }

    #[test]
    fn cost_flows_first_in_first_out_and_the_older_lot_goes_first() {
        // And the choice changes reported profit and the carrying value in opposite directions when
        // prices move, which is why it is a real decision.
        let fifo = take(&lots(), 120.0, CostFlow::FirstInFirstOut);
        assert_eq!(fifo.cost, 100.0 * 4.0 + 20.0 * 7.0);
        let average = take(&lots(), 120.0, CostFlow::WeightedAverage);
        assert_eq!(average.cost, 120.0 * 5.5);
        // Profit and carrying value move opposite ways: the cheaper charge leaves dearer stock.
        let fifo_left: f64 = fifo.left.iter().map(|l| l.qty * l.basis_per_unit).sum();
        let average_left: f64 = average.left.iter().map(|l| l.qty * l.basis_per_unit).sum();
        assert!(fifo.cost < average.cost);
        assert!(fifo_left > average_left);
    }

    #[test]
    fn taking_more_units_than_are_there_takes_what_there_was() {
        // Arithmetic, not a clamp — there is no such thing as negative inventory.
        let all = take(&lots(), 500.0, CostFlow::FirstInFirstOut);
        assert_eq!(all.units, 200.0);
        assert_eq!(all.cost, 1_100.0);
        assert!(all.left.is_empty());
    }

    #[test]
    fn inventory_is_written_down_when_the_market_falls_below_cost_and_the_charge_is_an_event() {
        // The write-down is a charge to income in the period it happens, with a size.
        let lot = Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 };
        let down = carry(&lot, 5.0, CarriesAtFairValue(false));
        assert_eq!(down.per_unit, 5.0);
        assert_eq!(down.to_income, -200.0);
    }

    #[test]
    fn inventory_is_never_marked_up_above_cost_for_a_holder_that_is_not_a_broker_dealer() {
        // Marking it up invents profit the firm has not earned.
        let lot = Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 };
        let up = carry(&lot, 11.0, CarriesAtFairValue(false));
        assert_eq!(up.per_unit, 7.0);
        assert_eq!(up.to_income, 0.0);
    }

    #[test]
    fn a_commodity_broker_dealer_carries_at_fair_value_through_income_in_both_directions() {
        // The exception is real and narrow — for it the inventory IS the position.
        let lot = Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 };
        let up = carry(&lot, 11.0, CarriesAtFairValue(true));
        assert_eq!(up.per_unit, 11.0);
        assert_eq!(up.to_income, 400.0);
        let down = carry(&lot, 5.0, CarriesAtFairValue(true));
        assert_eq!(down.to_income, -200.0);
    }

    #[test]
    fn a_storage_fee_and_a_spoilage_rate_are_two_different_things() {
        // One is cash paid to whoever stores the goods, the other is units that perish.
        let lot = Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 };
        let gone = perish(&lot, 0.05);
        assert_eq!(gone.units, 5.0);
        assert_eq!(gone.at_cost, 35.0);
        let (storer, fee) = storage_fee(lot.qty, 0.2, party(70));
        assert_eq!(storer, party(70));
        assert_eq!(fee, 20.0);
    }

    #[test]
    fn spoilage_destroys_physical_units_from_their_inventory_owner() {
        assert_eq!(
            spoiled_inventory(party(7), good(3), &[Lot { qty: 20.0, basis_per_unit: 4.0, acquired: 1 }], 0.25),
            Some(Leg::Destroy {
                party: party(7),
                instrument: good(3),
                qty: crate::ledger::Units::new(5.0).unwrap(),
                why: Gone::Perished,
            })
        );
    }

    #[test]
    fn what_no_batch_absorbed_is_a_period_cost_and_is_not_also_in_the_stock() {
        // One cost in two places is counted twice.
        let sold = take(&lots(), 120.0, CostFlow::FirstInFirstOut);
        let charged = charge(&sold, 1_000.0, 600.0);
        assert_eq!(charged.cost_of_goods_sold, sold.cost);
        assert_eq!(charged.period_cost, 400.0);
        // A line that absorbed everything it spent charges nothing extra this period.
        assert_eq!(charge(&sold, 1_000.0, 1_000.0).period_cost, 0.0);
    }

    #[test]
    fn products_sharing_plant_cannot_absorb_the_same_depreciation_twice() {
        let first = absorb_period_charge(100.0, 1_000.0, 600.0, 0.0);
        let second = absorb_period_charge(100.0, 1_000.0, 600.0, first);

        assert_eq!(first, 60.0);
        assert_eq!(second, 40.0);
        assert_eq!(first + second, 100.0);
    }

    #[test]
    fn upkeep_names_the_external_plant_supplier() {
        assert_eq!(upkeep_due(party(1), party(2), 30.0), Some((party(2), party(1), 30.0)));
        assert_eq!(upkeep_due(party(1), party(1), 30.0), None);
    }

    #[test]
    fn completed_output_enters_the_producers_inventory_at_batch_cost() {
        assert_eq!(
            completed_output(party(4), good(9), 20.0, 150.0),
            Some(Leg::Create {
                party: party(4),
                instrument: good(9),
                qty: crate::ledger::Units::new(20.0).unwrap(),
                cost_per_unit: 7.5,
            })
        );
    }

    #[test]
    fn a_consignment_is_owned_while_it_moves_and_its_landed_cost_names_its_three_parts() {
        // The carrier is a named party that earns the freight, and goods in transit sit on
        // somebody's book.
        let c = Consignment {
            what: InstrumentId::at(9),
            units: 100.0,
            carrier: party(80),
            owned_in_transit_by: party(1),
            ex_works: 900.0,
            freight: 60.0,
            duty: 40.0,
            periods_in_transit: 2,
        };
        assert_eq!(c.landed_cost(), 1_000.0);
        assert_eq!(c.owned_in_transit_by, party(1));
        assert!(c.periods_in_transit > 0);
    }

    #[test]
    fn the_price_is_in_the_sellers_money() {
        // A foreign buyer converts by BUYING that money from somebody, which is an order with a
        // counterparty — not a conversion inside the trade.
        let sellers = CurrencyCode::at(2);
        assert_eq!(settles_in(sellers), sellers);
    }

    #[test]
    #[should_panic(expected = "is not a share of it")]
    fn more_than_a_lot_cannot_perish() {
        perish(&Lot { qty: 100.0, basis_per_unit: 7.0, acquired: 2 }, 1.4);
    }

    use crate::num::dust;

    fn good(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    /// Two units of input 1 and half a unit of input 2 make one unit of good 9, with 0.4 of labour
    /// and 0.1 of capital services, and nineteen starts in twenty survive.
    fn way(per_unit: Vec<(InstrumentId, f64)>, labour: f64, yields: f64, batch: f64) -> Way {
        Way {
            per_unit,
            labour_per_unit: labour,
            capital_services_per_unit: 0.1,
            yields,
            batch,
            periods_to_make: 1,
        }
    }

    fn line() -> Way {
        way(vec![(good(1), 2.0), (good(2), 0.5)], 0.4, 0.95, 1.0)
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
        let drawn = draws_for(&line(), 100.0);
        assert_eq!(drawn, vec![(good(1), 200.0), (good(2), 50.0)]);
    }

    #[test]
    fn an_input_shortage_bites_as_a_real_production_constraint() {
        // Fixed coefficients are what make a supply shock transmit at all.
        let d = decide(&line(), &reasons(950.0, 1_000.0, 300.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 150.0);
        assert_eq!(d.bound, Bound::Inputs);
        // A firm facing an expensive input cannot economise on it: substitution is MISSING here, not
        // assumed away, and nothing in this module quietly supplies it.
    }

    #[test]
    fn the_quantity_is_the_outcome_and_which_reason_bound_is_nameable() {
        // Each reason is a real state that reaches the decision.
        let plenty = reasons(950.0, 10_000.0, 100_000.0, 100_000.0, 100_000.0);
        assert_eq!(decide(&line(), &plenty).bound, Bound::Demand);
        let cramped = Reasons { capacity: 40.0, ..plenty.clone() };
        let constrained = decide(&line(), &cramped);
        assert_eq!(constrained.bound, Bound::Capacity);
        assert_eq!(constrained.starts * line().capital_services_per_unit, cramped.capacity);
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
        // What survives is dearer per unit than what was started, because normal waste is absorbed
        // into the cost of the survivors — a consequence of the division, not a markup.
        let r = line();
        let starts = 1_000.0;
        assert_eq!(finishes(&r, starts), 950.0);
        // The draw is against what was STARTED, because the scrap consumed its inputs too.
        assert_eq!(draws_for(&r, starts)[0].1, 2_000.0);
        let dearer = unit_cost(2_000.0, 400.0, 100.0, finishes(&r, starts)).unwrap();
        let if_nothing_scrapped = unit_cost(2_000.0, 400.0, 100.0, starts).unwrap();
        assert!(dearer > if_nothing_scrapped);
    }

    #[test]
    fn a_period_that_started_nothing_capitalises_nothing() {
        // The firm still incurs the cost, and it is a period expense.
        assert!(unit_cost(2_000.0, 400.0, 100.0, 0.0).is_none());
    }

    #[test]
    fn a_throttled_line_costs_more_per_unit_because_the_batch_is_smaller() {
        // A line's whole cost over a smaller batch IS a higher unit cost, and that is what running a
        // plant below its rate does.
        let at_rate = throttled_cost(10_000.0, 1_000.0).unwrap();
        let throttled = throttled_cost(10_000.0, 250.0).unwrap();
        assert_eq!(at_rate, 10.0);
        assert_eq!(throttled, 40.0);
    }

    #[test]
    fn utilisation_is_read_from_the_outcome_and_never_put_into_it() {
        // The decision above never consulted a utilisation figure; this is computed after it.
        let d = decide(&line(), &reasons(950.0, 2_000.0, 100_000.0, 100_000.0, 100_000.0));
        let u = utilisation(&d, line().capital_services_per_unit, 2_000.0).unwrap();
        assert!(u > 0.0 && u < 1.0);
        assert!(utilisation(&d, line().capital_services_per_unit, 0.0).is_none());
    }

    #[test]
    fn work_in_progress_is_owned_by_somebody_and_carries_what_it_cost() {
        // A real thing with a holder, between input and output — not a timing adjustment.
        let wip = WorkInProgress { owner: party(5), what: good(9), units: 120.0, cost_carried: 960.0 };
        assert_eq!(wip.owner, party(5));
        assert!(wip.cost_carried > 0.0);
    }

    #[test]
    fn a_line_that_yields_nothing_or_more_than_it_starts_is_not_a_line() {
        // What the registry refuses to write down, so no such way reaches a decision.
        assert!(!way(vec![(good(1), 2.0)], 0.4, 0.0, 1.0).runnable());
        assert!(!way(vec![(good(1), 2.0)], 0.4, 1.2, 1.0).runnable());
        assert!(!way(vec![(good(1), 2.0)], 0.4, 0.95, 0.0).runnable());
        assert!(line().runnable());
    }

    /// The same good, two ways: one that draws a lot of input 1 and little labour, one the reverse.
    fn two_ways() -> Vec<Way> {
        vec![
            way(vec![(good(1), 4.0), (good(2), 0.5)], 0.1, 0.95, 1.0),
            way(vec![(good(1), 1.0), (good(2), 0.5)], 2.0, 0.95, 1.0),
        ]
    }

    #[test]
    fn which_way_is_better_is_a_fact_about_prices_and_not_about_the_line() {
        // Two firms facing different prices pick differently, and the same firm picks differently
        // when a price moves.
        let line = two_ways();
        let dear_input = |i: InstrumentId| if i == good(1) { Some(10.0) } else { Some(1.0) };
        let cheap_input = |i: InstrumentId| if i == good(1) { Some(0.5) } else { Some(1.0) };

        // Input 1 at 10 and an hour at 1: the input-heavy way costs 40.6 a unit, the labour-heavy
        // one 12.2.
        let (picked, _) = picks(&line, &dear_input, 1.0, 1.0).unwrap();
        assert_eq!(picked.labour_per_unit, 2.0);

        // The same line, the same firm, input 1 now at 0.5: the first way wins on the same read.
        let (picked, _) = picks(&line, &cheap_input, 1.0, 1.0).unwrap();
        assert_eq!(picked.labour_per_unit, 0.1);

        // And nothing about the LINE changed between those two reads.
        assert_eq!(line.len(), 2);
    }

    #[test]
    fn a_way_the_firm_cannot_price_is_a_way_it_cannot_choose() {
        // An unpriced input is MISSING, not free.
        let line = two_ways();
        let only_input_two = |i: InstrumentId| if i == good(2) { Some(1.0) } else { None };
        assert!(costs(&line[0], &only_input_two, 1.0, 1.0).is_none());
        assert!(picks(&line, &only_input_two, 1.0, 1.0).is_none());
    }

    #[test]
    fn the_cost_a_firm_reads_is_the_cost_of_a_unit_that_survives() {
        // Normal waste is absorbed into the cost of the survivors, and the division is where that
        // happens.
        let ways = two_ways();
        let r = &ways[0];
        let all_at_one = |_: InstrumentId| Some(1.0);
        let c = costs(r, &all_at_one, 1.0, 1.0).unwrap();
        assert!((c - 4.7 / 0.95).abs() < dust(4, &[4.7, 0.95]));
        // Scrapping less makes the same physical draw cheaper per unit sold, with no markup moved.
        let kinder = way(r.per_unit.clone(), 0.1, 1.0, 1.0);
        assert!(costs(&kinder, &all_at_one, 1.0, 1.0).unwrap() < c);
    }

    /// The same line, but it can only be run fifty units at a time.
    fn in_fifties() -> Way {
        way(vec![(good(1), 2.0), (good(2), 0.5)], 0.4, 0.95, 50.0)
    }

    #[test]
    fn the_line_runs_in_whole_batches_and_the_remainder_was_never_producible() {
        // Half a furnace charge is not a smaller run, it is nothing.
        let d = decide(&in_fifties(), &reasons(950.0, 1_000.0, 320.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 150.0);
        assert_eq!(d.bound, Bound::Inputs);
        // The 10 units of input beyond the third batch are not clipped off a quantity the firm would
        // have made — they were never a producible run, and they are still on its books.
    }

    #[test]
    fn a_firm_whose_reasons_do_not_reach_one_batch_does_not_run_the_line_at_all() {
        // It is not a small production decision, it is the absence of one — and the reader can name
        // it, which is the whole point of `Bound`.
        let d = decide(&in_fifties(), &reasons(950.0, 1_000.0, 80.0, 10_000.0, 10_000.0));
        assert_eq!(d.starts, 0.0);
        assert_eq!(d.finishes, 0.0);
        assert_eq!(d.bound, Bound::Batch);
    }

    #[test]
    fn a_line_run_at_one_batch_where_it_could_run_ten_costs_ten_times_as_much_a_unit() {
        // B5.b and F5.a together: this is operating leverage.
        let at_ten = throttled_cost(10_000.0, 500.0).unwrap();
        let at_one = throttled_cost(10_000.0, 50.0).unwrap();
        assert_eq!(at_one / at_ten, 10.0);
    }
}
