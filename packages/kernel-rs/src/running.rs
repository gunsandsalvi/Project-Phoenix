//! WHAT EACH SYSTEM DOES IN A PERIOD.
//!
//! @spec ARCHITECTURE 4.9b · Law 4, Law 5, Law 10, Law 15, Law 19 · Appendix B

use crate::calendar::Day;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::Class;
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{about, afoot, agreed, Owing};

/// WHAT FALLS DUE IS PAID, OR IT IS AN ARREAR.
pub struct Servicing {
    /// One calendar: how many days a period is, so "falls due this period" is a read of dates
    /// (Calendar A1).
    pub days_per_period: i64,
}

impl Mechanism for Servicing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let from = crate::calendar::Day(ctx.period() as i64 * self.days_per_period);
        let to = crate::calendar::Day(from.0 + self.days_per_period - 1);
        let mut paying: Vec<(PartyId, InstrumentId, Vec<(PartyId, f64)>, Receipt, crate::stores::DueId)> =
            Vec::new();
        for due in ctx.schedules().falling(from, to) {
            let line = ctx.schedules().instrument_of(due);
            let owes = ctx.schedules().owed_by(due);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), owes) else {
                // The payer has no account to pay from: there is nothing to propose, and inventing
                // one would be inventing a counterparty.
                continue;
            };
            // Register E1, A2.a, Appendix B #10: EVERY holder is owed, in proportion to what it
            // holds.
            let owed: Vec<(PartyId, f64)> = ctx
                .register()
                .of_instrument(line)
                .iter()
                .map(|r| {
                    let row = crate::ids::HoldingId(*r);
                    (ctx.register().holder_of(row), ctx.register().quantity(row))
                })
                .filter(|(who, units)| *who != owes && *units > 0.0)
                .collect();
            let outstanding: f64 = owed.iter().map(|(_, units)| units).sum();
            if outstanding <= 0.0 {
                // Nobody but the issuer holds it.
                continue;
            }
            // A payment on a line is per unit of par, and each holder is paid for the units it
            // holds.
            let per_unit = ctx.schedules().amount(due) / outstanding;
            let receipt = match ctx.schedules().of(due) {
                Owing::Interest => Receipt::Interest,
                Owing::Principal => Receipt::Principal,
                Owing::Premium | Owing::Rent => Receipt::Transfer,
            };
            let legs: Vec<(PartyId, f64)> =
                owed.into_iter().map(|(who, units)| (who, per_unit * units)).collect();
            paying.push((owes, money, legs, receipt, due));
        }
        for (from_whom, money, owed, receipt, due) in paying {
            // One obligation, one instruction.
            let legs: Vec<Leg> = owed
                .into_iter()
                .filter_map(|(to_whom, amount)| {
                    // A holder owed nothing is not paid nothing; it is not paid.
                    Some(Leg::Money {
                        from: from_whom,
                        to: to_whom,
                        instrument: money,
                        amount: crate::ledger::Units::new(amount)?,
                        receipt,
                    })
                })
                .collect();
            ctx.propose(
                legs,
                Cause::Payment,
                Delivery::Nothing,
                "what fell due on the schedule this period",
            );
            ctx.settles(due);
        }
    }
}

/// A PARTY SHORT OF MONEY BRINGS PAPER.
pub struct Funding {
    /// WHOSE paper this is, and over what horizon.
    pub of_kinds: &'static [u32],
    /// How far ahead this system's shortfall is read, in days — and from how far ahead.
    pub after: &'static str,
    pub horizon: &'static str,
    /// One calendar: how long a period is, so the window is read from DATES.
    pub days_per_period: i64,
    /// How long the paper runs.
    pub tenor: &'static str,
    /// The coupon the paper carries, as a term.
    pub coupon: &'static str,
    /// The buffer the issuer keeps back.
    pub buffer: &'static str,
    pub says: u32,
}

impl Mechanism for Funding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::instruments::Class;
        let now = ctx.period();
        let from = crate::calendar::Day(now as i64 * self.days_per_period);
        // The window is read from DATES.
        let to = crate::calendar::Day(from.0 + ctx.params().days(self.horizon) as i64 - 1);
        let opens = crate::calendar::Day(from.0 + ctx.params().days(self.after) as i64);
        let periods = ctx.params().periods(self.tenor);
        let coupon = ctx.params().per_annum(self.coupon);
        let buffer = ctx.params().amount(self.buffer, crate::params::Denomination::Money);

        let mut bringing: Vec<(PartyId, crate::ids::CurrencyCode, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // The profile answers, and a kind with none is a kind nobody has said this of — which is
            // missing rather than a no.
            let kind = ctx.parties().kind_of(who);
            if !self.of_kinds.contains(&kind) {
                continue;
            }
            // And the profile still answers whether a kind issues paper at all — a kind with none is
            // a kind nobody has said this of, which is missing rather than a no.
            match ctx.registry().profile(kind) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            // Its own position.
            let owes: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .filter(|d| ctx.schedules().due(*d) >= opens)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let cash = ctx.register().quantity(ctx.register().row(who, money));
            // Outlays against what it has, plus what it needs to get back to its own buffer.
            let short = crate::mechanisms::treasury::must_raise(owes, 0.0, cash, buffer);
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // It matures on a DATE, so the maturity wall is spread by the dates and not by a
            // count of periods.
            let matures = crate::calendar::Day(from.0 + (periods as i64) * self.days_per_period);
            // And it owes its coupon and its principal, written down at issue.
            let years = (matures.0 - from.0) as f64 / 365.0;
            ctx.brings(crate::module::Brings {
                issuer: who,
                ccy,
                class: Class::Claim,
                unit: crate::ids::UnitId::at(0),
                coupon: Some(coupon),
                matures: Some(matures),
                units: short,
                // A treasury auction is a CALL — a sealed cross at one level, which is what an
                // auction IS.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
                owing: vec![
                    (matures, short * coupon * years, crate::stores::Owing::Interest),
                    (matures, short, crate::stores::Owing::Principal),
                ],
            });
            ctx.say(self.says, &[who.0], &[(0, Value::Num(short))], true);
        }
    }
}

/// WHAT A FIRM MAKES, AND THE PLANT IT MAKES IT WITH.
#[derive(Clone)]
pub struct Makes {
    pub line: crate::mechanisms::recipe::Line,
    /// What THIS line's plant is.
    pub plant: InstrumentId,
    pub plant_is: crate::mechanisms::capital_programme::Plant,
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

pub struct Making {
    pub makes: Vec<Makes>,
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
        use crate::mechanisms::capital_programme::{capacity, charge, upkeep, Vintage};
        use crate::mechanisms::goods::{take, Lot};
        use crate::mechanisms::recipe::{decide, picks, unit_cost, Reasons};

        let now = ctx.period();
        // THE READ PASS.
        let mut runs: Vec<Ran> = Vec::new();

        // 21i, 33 A4: how built-up each place is — one walk over the register a period, never a
        // stored aggregate.
        let built = crate::places::built_up(ctx.parties(), ctx.register(), ctx.registry());
        let crowds_at = ctx.params().square_km(self.crowds_at);

        for m in &self.makes {
            for &plant_row in ctx.register().of_instrument(m.plant) {
                let plant_row = HoldingId(plant_row);
                let maker = ctx.register().holder_of(plant_row);
                if !ctx.parties().alive(maker) {
                    continue;
                }

                // A vintage IS a lot on the register, so capacity and the period's charge are
                // reads over the lots and nothing stores either.
                let stock: Vec<Vintage> = ctx
                    .register()
                    .lots(plant_row)
                    .iter()
                    .map(|l| Vintage { units: l.qty, cost_per_unit: l.basis_per_unit, in_service: l.acquired })
                    .collect();
                let can_make = capacity(&stock, &m.plant_is, now);
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
                let keeping: f64 = stock.iter().map(|v| upkeep(v, &m.plant_is, now) + charge(v, &m.plant_is, now)).sum();
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
                let Some((way, _)) = picks(&m.line, &priced, an_hour, a_service) else {
                    continue;
                };
                // The same line, run where this much already stands.
                let crowding = match ctx.registry().footprint_of(m.line.makes) {
                    Some(_) => crate::places::crowding(
                        crate::places::standing_in(&built, ctx.parties().region_of(maker)),
                        crowds_at,
                    ),
                    None => 1.0,
                };
                // ONE writer of the scaling.
                let way = &way.where_it_stands(crowding);

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
                        on_shelf: ctx.register().quantity(ctx.register().row(maker, m.line.makes)),
                        cover: ctx.params().ratio(self.cover),
                    },
                );
                if d.starts <= 0.0 {
                    continue;
                }

                // What the draw costs, at the lots' own basis and in the order settlement will draw
                // them in — so what this books and what leaves cannot disagree.
                let draws = way.draws_for(d.starts);
                let mut inputs_cost = 0.0;
                for (what, units) in &draws {
                    let held: Vec<Lot> = ctx
                        .register()
                        .lots(ctx.register().row(maker, *what))
                        .iter()
                        .map(|l| Lot { units: l.qty, cost_per_unit: l.basis_per_unit, acquired: l.acquired })
                        .collect();
                    inputs_cost += take(&held, *units, self.flow).cost;
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
                    makes: m.line.makes,
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

/// A SYSTEM THAT READS WHAT THE BOOKS PRODUCED.
pub struct Reads {
    pub kind: u32,
    pub what: Counts,
}

/// Which read a `Reads` system publishes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Counts {
    /// How many lines printed this period — what a benchmark is a read over.
    LinesThatPrinted,
    /// How many parties are alive — what the liveness family is a read over.
    PartiesAlive,
    /// How much is outstanding on every schedule — the credit stock.
    CreditOutstanding,
    /// How many relations are live — engagements, policies, tenancies.
    AgreementsLive,
}

impl Mechanism for Reads {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let n = match self.what {
            Counts::LinesThatPrinted => (0..ctx.instruments().len())
                .filter(|i| ctx.prints().latest(InstrumentId::at(*i as u32), ctx.period()).is_some())
                .count() as f64,
            Counts::PartiesAlive => (0..ctx.parties().len())
                .filter(|p| ctx.parties().alive(PartyId::at(*p as u32)))
                .count() as f64,
            Counts::CreditOutstanding => (0..ctx.instruments().len())
                .map(|i| ctx.schedules().outstanding(InstrumentId::at(i as u32)))
                .sum(),
            Counts::AgreementsLive => (0..ctx.agreements().len())
                .filter(|a| ctx.agreements().live(crate::stores::AgreementId(*a as u32)))
                .count() as f64,
        };
        // A read over what the books produced is PUBLIC.
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}

/// WHAT EACH ISSUER OWES ITS HOLDERS, published once a period.
pub struct Owed {
    pub kind: u32,
}

impl Mechanism for Owed {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut owed: Vec<(u32, f64)> = Vec::new();
        for i in 0..ctx.instruments().len() {
            let line = InstrumentId::at(i as u32);
            if ctx.instruments().class_of(line) != Class::Money {
                continue;
            }
            let issuer = ctx.instruments().issuer_of(line);
            let (held, _) = ctx.register().held_total(line);
            let outstanding = held - ctx.register().quantity(ctx.register().row(issuer, line));
            if outstanding > 0.0 {
                owed.push((issuer.0, outstanding));
            }
        }
        for (issuer, amount) in owed {
            ctx.say(self.kind, &[issuer], &[(0, Value::Num(amount))], true);
        }
    }
}

/// WHAT A COMPANY'S CAPITAL COSTS IT, AT THE MARGIN, NOW.
pub struct CostOfCapital {
    pub kind: u32,
    pub accounts: u32,
    pub at_income: u32,
    pub at_shares: u32,
    /// The mix it would raise at.
    pub debt_share: &'static str,
}

impl Mechanism for CostOfCapital {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::instruments::Class;
        let debt_share = ctx.params().ratio(self.debt_share);

        // What each company last published, and over how many shares.
        let mut published: std::collections::HashMap<u32, (f64, f64)> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.accounts) {
            if let (Some(&who), Some(Value::Num(income)), Some(Value::Num(shares))) = (
                ctx.journal().subjects_of(row).first(),
                ctx.journal().says(row, self.at_income),
                ctx.journal().says(row, self.at_shares),
            ) {
                published.insert(who, (income, shares));
            }
        }

        let mut costs: Vec<(PartyId, f64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            let mut debt_now: Option<f64> = None;
            let mut equity_now: Option<f64> = None;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let Some(print) = ctx.prints().latest(what, ctx.period()) else { continue };
                match ctx.instruments().class_of(what) {
                    // 5: the yield derives FROM the price, which is the direction Law 3 requires —
                    // what the paper crossed at against what it repays.
                    Class::Claim => {
                        let Some(matures) = ctx.instruments().matures_on(what) else { continue };
                        let paper = crate::mechanisms::short_term_debt::Paper {
                            issuer: who,
                            face: 1.0,
                            price: print.price,
                            issued: Day(0),
                            matures,
                        };
                        if let Some(y) = paper.yield_on(crate::mechanisms::short_term_debt::Convention::Actual365) {
                            debt_now = Some(y);
                        }
                    }
                    // And the cost of equity is the EARNINGS YIELD — what it published over what a
                    // share last cost.
                    Class::Share => {
                        if let Some(&(income, shares)) = published.get(&row) {
                            if shares > 0.0 && print.price > 0.0 {
                                equity_now = Some(income / shares / print.price);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let (Some(debt_now), Some(equity_now)) = (debt_now, equity_now) else { continue };
            costs.push((who, crate::mechanisms::cost_of_capital::at_the_margin(debt_now, equity_now, debt_share)));
        }

        for (who, cost) in costs {
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(cost))], true);
        }
    }
}

/// A FIRM DECIDES TO INVEST, AND THE COMPARISON IS THE MECHANISM.
pub struct Building {
    pub kind: u32,
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
        let horizon = ctx.params().periods(self.horizon);
        let hurdle = ctx.params().ratio(self.hurdle);
        let crowds_at = ctx.params().square_km(self.crowds_at);
        let takes = ctx.params().periods(self.takes) as u32;

        // What each company's capital costs it, most recently published.
        let mut costs: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.costs) {
            if let (Some(&who), Some(Value::Num(cost))) =
                (ctx.journal().subjects_of(row).first(), ctx.journal().says(row, 0))
            {
                costs.insert(who, cost);
            }
        }
        if costs.is_empty() {
            return;
        }
        // How built-up each place is.
        let built = crate::places::built_up(ctx.parties(), ctx.register(), ctx.registry());

        let mut opening: Vec<(PartyId, f64)> = Vec::new();
        for (&who, &cost_of_capital) in &costs {
            let firm = PartyId(who);
            if !ctx.parties().alive(firm) {
                continue;
            }
            if ctx.processes().running(afoot::CAPITAL_PROGRAMME).iter().any(|p| ctx.processes().owner(*p) == firm) {
                continue;
            }
            // Its own outlook, and a firm with none has nothing to expect.
            let Some(sells) = ctx.outlooks().of(firm, crate::stores::about::HOW_MUCH_IT_SELLS) else {
                continue;
            };
            let Some(price) = ctx.outlooks().of(firm, crate::stores::about::WHAT_IT_SELLS_FOR) else {
                continue;
            };
            // What the ground it stands on does to a build.
            let where_it_is = ctx.parties().region_of(firm);
            let crowding = crate::places::crowding(
                crate::places::standing_in(&built, where_it_is),
                crowds_at,
            );
            let project = crate::mechanisms::capital_programme::Project {
                returns_per_period: sells * price,
                costs: sells * price * crowding,
                horizon,
                hurdle,
            };
            if !crate::mechanisms::capital_programme::worth_doing(&project, cost_of_capital) {
                continue;
            }
            opening.push((firm, project.costs));
        }

        for (firm, commits) in opening {
            ctx.opens(crate::module::Opens {
                kind: afoot::CAPITAL_PROGRAMME,
                owner: firm,
                closes: Some(ctx.period() + takes),
                size: commits,
            });
            ctx.say(self.kind, &[firm.0], &[(0, Value::Num(commits))], true);
        }
    }
}
