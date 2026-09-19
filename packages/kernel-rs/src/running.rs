//! WHAT EACH SYSTEM DOES IN A PERIOD.
//!
//! @spec ARCHITECTURE 4.9b · Law 4, Law 5, Law 10, Law 15, Law 19 · Appendix B

use crate::calendar::Day;
use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::{equity, Class};
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{about, afoot, agreed, standing, Owing};

// THE FIVE KIND COLUMNS THAT LIVED HERE ARE IN THE KERNEL NOW. `agreed`, `standing`, `afoot` and
// `about` are in `stores.rs`, beside the stores whose kind columns they name; `tracks` is in

/// WHAT FALLS DUE IS PAID, OR IT IS AN ARREAR.
pub struct Servicing {
    /// One calendar: how many days a period is, so "falls due this period" is a read of dates
    /// (Calendar A1). A TECHNOLOGY.
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
            // holds. This took `of_instrument(line)`, found the first row that was not the issuer,
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
                // Nobody but the issuer holds it. Nothing falls due to anybody, which is an answer
                // about who is owed rather than a payment to invent a payee for.
                continue;
            }
            // A payment on a line is per unit of par, and each holder is paid for the units it
            // holds. The parts sum to the whole by construction because the denominator is the sum
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
            // One obligation, one instruction. Every holder's leg stands or falls with the rest,
            // because an issuer short of its coupon fails the coupon and not nineteen twentieths of
            let legs: Vec<Leg> = owed
                .into_iter()
                .map(|(to_whom, amount)| Leg::Money {
                    from: from_whom,
                    to: to_whom,
                    instrument: money,
                    amount,
                    receipt,
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
    /// How far ahead this system's shortfall is read, in days — and from how far ahead. A firm short
    /// over the year is short over the week too, so the windows do not overlap: two systems reading
    /// the same due date would bring two instruments for one shortfall.
    pub after: &'static str,
    pub horizon: &'static str,
    /// One calendar: how long a period is, so the window is read from DATES.
    pub days_per_period: i64,
    /// 5 C3.a: how long the paper runs. A market CONVENTION about the tenor it brings, declared as a
    /// technology and read through `params` — not a choice this mechanism makes for anybody.
    pub tenor: &'static str,
    /// The coupon the paper carries, as a term (5 C4.b). It is a term and not a price: what the
    /// paper is WORTH is what the auction crosses at.
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
        // The window is read from DATES. What falls due inside it is what this system funds.
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
            // The profile answers, and a kind with none is a kind nobody has said this of — which
            // is missing rather than a no.
            let kind = ctx.parties().kind_of(who);
            if !self.of_kinds.contains(&kind) {
                continue;
            }
            // And the profile still answers whether a kind issues paper at all — a kind with none
            // is a kind nobody has said this of, which is missing rather than a no.
            match ctx.registry().profile(kind) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            // Its own position. What falls due on it, what falls due to it, what it has.
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
            // Outlays against what it has, plus what it needs to get back to its own buffer. A
            // party short of nothing brings nothing — not a floor under the size, the absence of a
            let short = crate::mechanisms::treasury::must_raise(owes, 0.0, cash, buffer);
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // 5 C3.a: it matures on a DATE, so the maturity wall is spread by the dates and not by
            // a count of periods.
            let matures = crate::calendar::Day(from.0 + (periods as i64) * self.days_per_period);
            // 5 D2: and it owes its coupon and its principal, written down at issue. The coupon is
            // the annual rate over the years the paper runs, from the dates.
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
                // auction IS. The `seen_by` is not read by a call and says so with one, and nothing
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


/// THE FLOATING BENCHMARK IS A TRANSACTED RATE, OR IT IS NOTHING.
pub struct Fixes {
    /// The overnight book. `Missing` where this world has no overnight line — and then there is
    /// nothing to fix on, which is an answer.
    pub on: Option<crate::ids::MarketId>,
    pub says: u32,
}

impl Mechanism for Fixes {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let Some(book) = self.on else { return };
        let line = crate::ids::InstrumentId::at(book.0);
        let Some(print) = ctx.prints().latest(line, ctx.period()) else { return };
        // Only a CLEARED print is a fixing. A carried or seeded one is refused here, which is the
        // whole of what "a transacted rate" means.
        let Some(fixing) = crate::mechanisms::benchmarks::fix(&print) else { return };
        ctx.say(
            self.says,
            &[],
            &[(0, Value::Num(fixing.rate)), (1, Value::Num(f64::from(fixing.period)))],
            true,
        );
    }
}


/// WHAT A FIRM MAKES, AND THE PLANT IT MAKES IT WITH. Registry data, one row per good this world
/// knows how to produce.
#[derive(Clone)]
pub struct Makes {
    pub line: crate::mechanisms::recipe::Line,
    /// What THIS line's plant is. Capital is specific in kind (33 A4).
    pub plant: InstrumentId,
    pub plant_is: crate::mechanisms::capital_programme::Plant,
}

/// THE FIRM PRODUCES.
struct Ran {
    maker: PartyId,
    makes: InstrumentId,
    draws: Vec<(InstrumentId, f64)>,
    finished: f64,
    /// The period it comes off the line. The inputs go now; the output arrives then.
    ready: u32,
    /// What went in — inputs at their own basis, wages, the capital charge. The batch carries it and
    /// the unit cost is struck from it when the batch comes off, so what a unit cost is what went
    /// into THAT batch (Law 4: one cost, in one place).
    cost: f64,
}

pub struct Making {
    pub makes: Vec<Makes>,
    /// The flow, declared once and applied consistently — and it is the order settlement itself
    /// draws lots in, so the cost this books and the units that leave cannot disagree.
    pub flow: crate::mechanisms::goods::CostFlow,
    /// The id of the standing area at which a build draws twice, read through `params`.
    pub crowds_at: &'static str,
    /// How much COVER a firm wants on its shelf, as a multiple of what it expects to sell. A
    /// PREFERENCE, read through `params`.
    pub cover: &'static str,
}

impl Mechanism for Making {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::ids::HoldingId;
        use crate::mechanisms::capital_programme::{capacity, charge, upkeep, Vintage};
        use crate::mechanisms::goods::{take, Lot};
        use crate::mechanisms::recipe::{decide, picks, unit_cost, Reasons};

        let now = ctx.period();
        // THE READ PASS. Nothing below writes, and nothing above proposes.
        let mut runs: Vec<Ran> = Vec::new();

        // 21i, 33 A4: how built-up each place is — one walk over the register a period, never a
        // stored aggregate. A line that builds a STRUCTURE draws more where more already stands,
        let built = crate::places::built_up(ctx.parties(), ctx.register(), ctx.registry());
        let crowds_at = ctx.params().square_km(self.crowds_at);

        for m in &self.makes {
            for &plant_row in ctx.register().of_instrument(m.plant) {
                let plant_row = HoldingId(plant_row);
                let maker = ctx.register().holder_of(plant_row);
                if !ctx.parties().alive(maker) {
                    continue;
                }

                // 33 A6: a vintage IS a lot on the register, so capacity and the period's charge
                // are reads over the lots and nothing stores either.
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

                // Its own outlook, and NOT a model forecast. A firm with no view of what it sells
                // has no reason to start a line, and that is missing rather than nothing.
                let Some(expects) = ctx.outlooks().of(maker, about::HOW_MUCH_IT_SELLS) else {
                    continue;
                };

                // The hours its engagements give it, and what an hour of them costs. The terms are
                // `[wage, hours]` — the convention `agreed::ENGAGEMENT` states.
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
                        // of both. `Wages` was told this and `Making` was not, which left one term
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
                // own depreciation, over what the plant can make. Both are owed whether the line
                let keeping: f64 = stock.iter().map(|v| upkeep(v, &m.plant_is, now) + charge(v, &m.plant_is, now)).sum();
                let a_service = keeping / can_make;

                // It picks the way that costs IT least — and what an input costs IT is what it paid
                // for the stock it holds, off its own lots, because that is the stock the batch
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
                // The same line, run where this much already stands. Crowding scales every way of
                // making the line alike, so it cannot change which way is cheapest — that is why it
                let crowding = match ctx.registry().footprint_of(m.line.makes) {
                    Some(_) => crate::places::crowding(
                        crate::places::standing_in(&built, ctx.parties().region_of(maker)),
                        crowds_at,
                    ),
                    None => 1.0,
                };
                // ONE writer of the scaling. What the firm can afford to start, what leaves its
                // rows and what the batch cost all come off this one object, so the decision and
                let way = &way.where_it_stands(crowding);

                // What it holds of each input, off its own rows. An input it has no row for is one
                // it has none of, and `decide` is where that stops the line.
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
                // No units, no capitalised cost. A run that finishes nothing capitalises nothing,
                // and the cost it incurred is a period expense rather than a batch — which is the
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

        // WHAT COMES OFF THE LINE. B3, 21f.3: the batches whose time is up, started in an earlier
        // period and carrying what they cost then.
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
            ctx.propose(
                vec![Leg::Create { party: maker, instrument: makes, qty: units, cost_per_unit: per_unit }],
                Cause::Production,
                Delivery::Nothing,
                "the batches that came off the line this period",
            );
            ctx.finishes(batch);
        }

        // THE STARTS. B2: production consumes the inputs it consumes, NOW — and B3 puts what they
        // became on the line, owned, carrying what it cost, until it is ready.
        for Ran { maker, makes, draws, finished, ready, cost } in runs {
            let legs: Vec<Leg> = draws
                .iter()
                .map(|(what, qty)| Leg::Destroy {
                    party: maker,
                    instrument: *what,
                    qty: *qty,
                    why: crate::ledger::Gone::Consumed,
                })
                .collect();
            ctx.propose(legs, Cause::Production, Delivery::Nothing, "the inputs the line drew this period");
            ctx.starts(maker, makes, finished, cost, ready);
        }
    }
}


/// A POOL WHOSE MANAGER DIED WINDS UP THROUGH THE MACHINERY IT ALREADY HAS.
pub struct Winding {
    /// The kind it publishes under, so a reader can see a pool lose its manager.
    pub says: u32,
}

impl Mechanism for Winding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::funds::{pro_rata, run_as, Run};

        let mut paying: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        let mut ending: Vec<PartyId> = Vec::new();
        let mut orphaned: Vec<PartyId> = Vec::new();

        for p in ctx.parties().of_kind(kinds::FUND) {
            let pool = PartyId::at(*p);
            if !ctx.parties().alive(pool) {
                continue;
            }
            // Whether anybody decides for it is a read of the RELATIONS, never a flag on the pool
            // that somebody has to remember to clear.
            let live = ctx
                .agreements()
                .of_party(pool)
                .iter()
                .map(|r| crate::stores::AgreementId(*r))
                .filter(|a| ctx.agreements().live(*a) && ctx.agreements().kind_of(*a) == agreed::MANDATE)
                .count();
            if run_as(live) == Run::Mandated {
                continue;
            }
            orphaned.push(pool);

            // What it has raised is what there is to pay with — its own account, and nothing else.
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), pool) else { continue };
            let cash = ctx.register().quantity(ctx.register().row(pool, money));

            // A holder of its shares has a redeemable claim, and a share count is what makes a
            // claim redeemable. Which line that is, is a walk over the instruments filtered by
            let Some(shares) = (0..ctx.instruments().len())
                .map(|r| InstrumentId::at(r as u32))
                .find(|i| ctx.instruments().issuer_of(*i) == pool && ctx.instruments().class_of(*i) == Class::Share)
            else {
                continue;
            };
            let (outstanding, _) = ctx.register().held_total(shares);
            let held_by_it = ctx.register().quantity(ctx.register().row(pool, shares));
            let out = outstanding - held_by_it;

            let still_holds: f64 = ctx
                .register()
                .of_holder(pool)
                .iter()
                .map(|r| crate::ids::HoldingId(*r))
                .filter(|r| {
                    let line = ctx.register().instrument_of(*r);
                    line != money && line != shares
                })
                .map(|r| ctx.register().quantity(r))
                .sum();

            if out <= 0.0 {
                // Nothing held and nobody owed is a pool that has ended. Law 6: nothing here ends
                // it on a schedule — the wind-up takes as long as the selling takes.
                if still_holds <= 0.0 {
                    ending.push(pool);
                }
                continue;
            }
            if cash <= 0.0 {
                continue;
            }
            for row in ctx.register().of_instrument(shares) {
                let row = crate::ids::HoldingId(*row);
                let holder = ctx.register().holder_of(row);
                if holder == pool {
                    continue;
                }
                let Some(share) = pro_rata(cash, ctx.register().quantity(row), out) else { continue };
                if share > 0.0 {
                    paying.push((pool, holder, money, share));
                }
            }
        }

        for pool in orphaned {
            ctx.say(self.says, &[pool.0], &[], true);
        }
        for (pool, holder, money, amount) in paying {
            ctx.propose(
                vec![Leg::Money {
                    from: pool,
                    to: holder,
                    instrument: money,
                    amount,
                    receipt: Receipt::Principal,
                }],
                Cause::CorporateAction,
                Delivery::Nothing,
                "a winding pool paying its holders pro rata on what it raised",
            );
        }
        for pool in ending {
            ctx.ceases(pool);
        }
    }
}

/// AN ESTATE PAYS ITS CLAIMANTS IN RANK ORDER, AND THE STATE IS ONE OF THEM.
pub struct Ranked {
    pub says: u32,
}

impl Mechanism for Ranked {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::estate::{waterfall, Claim};

        let mut paying: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        let mut told: Vec<(PartyId, f64)> = Vec::new();
        let mut marking: Vec<(crate::stores::ClaimId, f64)> = Vec::new();

        for p in 0..ctx.parties().len() {
            let estate = PartyId::at(p as u32);
            // An estate is what is left of a party whose life has ended. Nothing here asks what
            // KIND of party it was — a dead bank and a dead baker pay the same way.
            if ctx.parties().alive(estate) {
                continue;
            }
            let rows = ctx.claims().on_estate(estate);
            if rows.is_empty() {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), estate) else { continue };
            let has = ctx.register().quantity(ctx.register().row(estate, money));
            if has <= 0.0 {
                continue;
            }
            let live: Vec<crate::stores::ClaimId> = rows
                .iter()
                .map(|r| crate::stores::ClaimId(*r))
                .filter(|c| ctx.claims().outstanding(*c) > 0.0)
                .collect();
            let claims: Vec<Claim> = live
                .iter()
                .map(|c| Claim {
                    holder: ctx.claims().holder_of(*c),
                    owed: ctx.claims().outstanding(*c),
                    ranks: rank_of(ctx.claims().ranks(*c)),
                })
                .collect();
            if claims.is_empty() {
                continue;
            }
            let mut out = 0.0;
            // The waterfall answers in the order it was asked, so each result is THIS claim's and
            // `marking` carries the id. 21.36: a claim paid and not marked comes back whole next
            for (c, p) in live.iter().zip(waterfall(has, &claims)) {
                if p.paid > 0.0 {
                    paying.push((estate, p.holder, money, p.paid));
                    marking.push((*c, p.paid));
                    out += p.paid;
                }
            }
            told.push((estate, out));
        }

        for (estate, out) in told {
            ctx.say(self.says, &[estate.0], &[(0, Value::Num(out))], true);
        }
        for (estate, holder, money, amount) in paying {
            ctx.propose(
                vec![Leg::Money {
                    from: estate,
                    to: holder,
                    instrument: money,
                    amount,
                    receipt: Receipt::Principal,
                }],
                Cause::CorporateAction,
                Delivery::Nothing,
                "an estate paying a ranked claimant out of what it has",
            );
        }
        for (claim, amount) in marking {
            ctx.pays(claim, amount);
        }
    }
}

/// The rank a stored claim stands at. `Claims` holds a number and does not know what it means; this
/// is where the number becomes the law's ordering, in one place.
fn rank_of(stored: u32) -> crate::mechanisms::estate::Rank {
    use crate::mechanisms::estate::Rank;
    match stored {
        0 => Rank::Secured,
        1 => Rank::Preferential,
        2 => Rank::Senior,
        3 => Rank::Trade,
        4 => Rank::Subordinated,
        // Equity is last, which is what makes it equity — and what an unrecognised rank is, is last
        // too: a claimant nobody can place does not get in ahead of one somebody can.
        _ => Rank::Equity,
    }
}



/// A SYSTEM THAT READS WHAT THE BOOKS PRODUCED.
pub struct Reads {
    pub kind: u32,
    /// What it counts. A read over the world's own stores, named so a reader knows which.
    pub what: Counts,
}

/// Which read a `Reads` system publishes. Law 15: data, not a branch in the mechanism.
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
        // A read over what the books produced is PUBLIC. That is what a benchmark is.
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

/// A PUBLIC COMPANY PUBLISHES WHAT ITS OWN BOOKS PRODUCED.
pub struct Publishes {
    /// The event kind the accounts are published under.
    pub kind: u32,
    /// Key rows for the figures, so a reader takes them by name rather than by position.
    pub at_equity: u32,
    pub at_income: u32,
    pub at_shares: u32,
    /// Which fiscal close a report is FOR. A figure with no period is one nobody can restate against
    /// or compare with the next.
    pub at_closed: u32,
    pub days_per_period: i64,
    /// How many days after the books close the report comes out. A TECHNOLOGY.
    pub asymmetry: &'static str,
    /// The weight a bank puts on what it already thought against what it has just seen. ONE
    /// PREFERENCE, and it is what makes two banks' estimates of one name differ.
    pub memory: &'static str,
}

impl Mechanism for Publishes {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let days = self.days_per_period;
        let today = Day(i64::from(ctx.period()) * days);
        let asymmetry = ctx.params().days(self.asymmetry) as i64;

        // What each company last published, read off the journal's own rows — one pass, not one
        // walk of the world's history per company (Law 19: the read replaces the walk).
        let mut last: std::collections::HashMap<u32, (u32, f64)> = std::collections::HashMap::new();
        // And which fiscal close each report was ABOUT, so a quarter is published once and a
        // restatement is a different act.
        let mut reported: std::collections::HashSet<(u32, i64)> = std::collections::HashSet::new();
        for &row in ctx.journal().of_kind(self.kind) {
            let when = ctx.journal().period_of(row);
            if let (Some(&who), Some(Value::Num(equity))) =
                (ctx.journal().subjects_of(row).first(), ctx.journal().says(row, self.at_equity))
            {
                last.insert(who, (when, equity));
                if let Some(Value::Num(about)) = ctx.journal().says(row, self.at_closed) {
                    reported.insert((who, about as i64));
                }
            }
        }

        let mut out: Vec<(u32, f64, Option<f64>, f64, i64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Listed, and held by outsiders. Both are reads of the register.
            let mut listed = 0.0;
            let mut outsiders = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let share = InstrumentId::at(line);
                if ctx.instruments().class_of(share) != Class::Share {
                    continue;
                }
                let (held, _) = ctx.register().held_total(share);
                listed += held;
                outsiders += held - ctx.register().quantity(ctx.register().row(who, share));
            }
            if !crate::mechanisms::reporting::reports(listed > 0.0, outsiders) {
                continue;
            }
            // THE FISCAL PERIOD IS A QUARTER, placed by DATE from the day this company started —
            // three months of calendar, which is a whole number of periods only by accident. It is
            let born = Day(i64::from(ctx.parties().since(who)) * days);
            let mut opens = born;
            let mut closes = Day(born.plus_months(3).0 - 1);
            // The LAST quarter whose report is due. A company reports every quarter, so the one to
            // publish is the most recent closed one it has not published yet.
            while Day(closes.plus_months(3).0).0 + asymmetry <= today.0 {
                opens = Day(closes.0 + 1);
                closes = Day(opens.plus_months(3).0 - 1);
            }
            if closes.0 >= today.0 {
                continue;
            }
            let fiscal = crate::mechanisms::reporting::Fiscal::new(opens, closes, Day(closes.0 + asymmetry));
            if today < fiscal.published {
                continue;
            }
            // And it publishes each quarter ONCE. A report already out for this close is not
            // republished; a restatement is a different act and nothing restates yet.
            if reported.contains(&(row, fiscal.closes.0)) {
                continue;
            }
            let now = equity(who, ctx.register(), ctx.instruments(), ctx.claims());
            // Income is the MOVEMENT against what it last published. A first report has no prior
            // close and so publishes no income — missing is missing.
            let income = last.get(&row).map(|&(_, was)| now - was);
            out.push((row, now, income, listed, fiscal.closes.0));
        }
        // And the banks that cover a name estimate what it will report. Coverage is uneven and how
        // many cover a name is an OUTCOME: a bank estimates the names it can SEE, which is the ones
        let memory = ctx.params().ratio(self.memory);
        let mut estimating: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (who, worth, _, _, _) in &out {
            let company = PartyId(*who);
            for &line in ctx.instruments().of_issuer(company) {
                for &row in ctx.register().of_instrument(InstrumentId::at(line)) {
                    let row = crate::ids::HoldingId(row);
                    let bank = ctx.register().holder_of(row);
                    if bank == company
                        || ctx.parties().kind_of(bank) != kinds::BANK
                        || ctx.register().quantity(row) <= 0.0
                    {
                        continue;
                    }
                    // What it has seen is what the company has PUBLISHED, and it may not be handed
                    // the answer — so what it holds now is weighted against what this report is
                    let held = match ctx.standing().of_party_about(bank, company, standing::ESTIMATE) {
                        Some(st) => ctx.standing().terms(st)[0],
                        // A bank that has seen nothing of a name has no estimate of it and is not
                        // covering it — its first is what the first report it saw said.
                        None => *worth,
                    };
                    estimating.push((bank, company, held * memory + *worth * (1.0 - memory)));
                }
            }
        }
        for (bank, company, figure) in estimating {
            // It HOLDS it, so it can be shown to have been wrong — and F1's surprise is the report
            // against what was standing when it arrived.
            ctx.now_stands(standing::ESTIMATE, bank, company, vec![figure]);
        }

        for (who, worth, income, shares, closed) in out {
            let mut data = vec![
                (self.at_equity, Value::Num(worth)),
                (self.at_shares, Value::Num(shares)),
                // WHICH fiscal close this is the report for. A figure with no period is a figure
                // nobody can restate or compare.
                (self.at_closed, Value::Num(closed as f64)),
            ];
            if let Some(earned) = income {
                data.push((self.at_income, Value::Num(earned)));
            }
            // Published, which is what makes it something anybody else may read.
            ctx.say(self.kind, &[who], &data, true);
        }
    }
}


/// EVERY HOUSE GRADES EVERY NAME IT CAN READ, AND THEY DISAGREE.
pub struct Grading {
    /// The event kind a rating action is published under.
    pub kind: u32,
    /// §48's published accounts, which is what the coverage and the trend are read from.
    pub accounts: u32,
    pub at_income: u32,
    pub days_per_period: i64,
}

impl Mechanism for Grading {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let houses: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::ASSESSOR)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if houses.is_empty() {
            return;
        }
        // What each name last published, and what it published before that — the trend. One pass
        // over the accounts rather than a walk per name per house.
        let mut last: std::collections::HashMap<u32, (f64, Option<f64>)> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.accounts) {
            if let (Some(&who), Some(Value::Num(income))) =
                (ctx.journal().subjects_of(row).first(), ctx.journal().says(row, self.at_income))
            {
                let was = last.get(&who).map(|(now, _)| *now);
                last.insert(who, (income, was));
            }
        }

        let mut actions: Vec<(PartyId, PartyId, crate::mechanisms::ratings::Grade)> = Vec::new();
        for (&who, &(income, before)) in &last {
            let of = PartyId(who);
            if !ctx.parties().alive(of) {
                continue;
            }
            // Leverage is what it owes against what it holds. Both are reads.
            let owes: f64 = ctx
                .instruments()
                .of_issuer(of)
                .iter()
                .map(|i| ctx.schedules().outstanding(InstrumentId::at(*i)))
                .sum();
            let holds = equity(of, ctx.register(), ctx.instruments(), ctx.claims());
            let state = crate::mechanisms::ratings::State {
                leverage: owes / holds,
                // Coverage is what it earns against what it owes. An issuer that owes nothing is
                // covered by arithmetic and not by a bound.
                coverage: if owes > 0.0 { income / owes } else { f64::INFINITY },
                cash: ctx.register().quantity(
                    ctx.register().row(of, match crate::ledger::account_of(ctx.parties(), ctx.instruments(), of) {
                        Some(cash) => cash,
                        None => continue,
                    }),
                ),
                size: holds,
                age_periods: ctx.parties().age(of, ctx.period()),
                // And the TREND — this year's published income against last year's. A name with one
                // report has no trend, and no trend is not a trend of zero.
                trend: match before {
                    Some(was) if was != 0.0 => (income - was) / was.abs(),
                    _ => 0.0,
                },
            };
            let grade = crate::mechanisms::ratings::grade_from(&state);
            for &by in &houses {
                actions.push((by, of, grade));
            }
        }

        for (by, of, grade) in actions {
            // It is STICKY. A house that already says this about this name says nothing; a grade
            // republished every period is not a rating action and would make A6's record of what a
            let held = ctx
                .standing()
                .of_party_about(by, of, crate::stores::standing::GRADE)
                .map(|s| ctx.standing().terms(s)[0]);
            if matches!(held, Some(rank) if rank == grade.rank()) {
                continue;
            }
            // The probability of failing and, SEPARATELY, the loss given it. Both are the house's
            // own view and both are stood behind with the grade.
            ctx.now_stands(
                crate::stores::standing::GRADE,
                by,
                of,
                vec![grade.rank(), 0.0, 0.0],
            );
            ctx.say(self.kind, &[by.0, of.0], &[(0, Value::Num(grade.rank()))], true);
        }
    }
}

/// A DOWNGRADE PAST A MANDATE'S BOUNDARY IS A FORCED SALE BY EVERY HOLDER BOUND BY IT, ON THE SAME
/// DATE.
pub struct ForcedSelling {
    /// What it says when a holder is put in a workout.
    pub kind: u32,
    /// How many periods a holder has to sell what its mandate no longer lets it hold. A TECHNOLOGY:
    /// how long a breach may stand before it is a breach nobody is curing.
    pub within: &'static str,
}

impl Mechanism for ForcedSelling {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let within = ctx.params().periods(self.within) as u32;
        // What each issuer is graded at now. The WORST grade any house holds on it, because a
        // mandate that let a holder pick the kindest house would not bind on anything.
        let mut worst: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let s = crate::stores::StandingId(row);
            if !ctx.standing().live(s) || ctx.standing().kind_of(s) != standing::GRADE {
                continue;
            }
            let about = ctx.standing().about(s).0;
            let rank = ctx.standing().terms(s)[0];
            worst.entry(about).and_modify(|r| if rank > *r { *r = rank }).or_insert(rank);
        }
        if worst.is_empty() {
            return;
        }

        let mut breached: Vec<(PartyId, f64)> = Vec::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::MANDATE {
                continue;
            }
            let floor = match ctx.agreements().terms(a).first() {
                Some(&floor) => floor,
                // A mandate with no floor restricts no grade. That is an answer about that mandate
                // and not a reason to invent one.
                None => continue,
            };
            // The pool is the side the mandate is over; the manager is the other. The pool is what
            // HOLDS, so it is the side whose register rows this reads.
            let (one, other) = ctx.agreements().between(a);
            for pool in [one, other] {
                if !ctx.parties().alive(pool) {
                    continue;
                }
                let mut must_sell = 0.0;
                for row in ctx.register().of_holder(pool) {
                    let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                    let issuer = ctx.instruments().issuer_of(line);
                    // Through the floor, and only through it. A grade at the floor is one the
                    // mandate still allows.
                    if matches!(worst.get(&issuer.0), Some(&rank) if rank > floor) {
                        must_sell += ctx.register().quantity(crate::ids::HoldingId(*row));
                    }
                }
                if must_sell > 0.0 {
                    breached.push((pool, must_sell));
                }
            }
        }

        for (pool, units) in breached {
            // It is already in one, and a second workout for the same breach would be the same
            // requirement counted twice.
            if ctx.processes().running(afoot::WORKOUT).iter().any(|p| ctx.processes().owner(*p) == pool) {
                continue;
            }
            ctx.opens(crate::module::Opens {
                kind: afoot::WORKOUT,
                owner: pool,
                closes: Some(ctx.period() + within),
                size: units,
            });
            ctx.say(self.kind, &[pool.0], &[(0, Value::Num(units))], true);
        }
    }
}

/// AND IT STANDS IN THE MARKET WITH A SIZE AND NO LEVEL.
pub struct ForcedSeller {
    pub kind: u32,
    /// The kinds of party that can be put in a workout. Registry data handed in, never a branch.
    pub of_kind: u32,
}

impl crate::module::Participant for ForcedSeller {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        // It sells what it HOLDS, off its own rows — never by asking every book in the world.
        if view.in_a_workout() == 0.0 {
            return Vec::new();
        }
        view.holdings().map(|row| crate::systems::book_of(view.line_of(row))).collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let must = view.in_a_workout();
        if must <= 0.0 {
            return Vec::new();
        }
        let line = crate::systems::line_of(m);
        let held = view.free(line);
        // It cannot sell more than it holds, which is arithmetic about a holding and not a cap on a
        // number.
        let units = crate::clearing::whole_pieces(crate::mechanisms::forced_sale::sells(held, must));
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Sell,
            // NO LEVEL. This is the whole of Clearing C3 and XI-2 in one field.
            price: None,
            qty: units,
        }]
    }
}


/// XI-1, Banks Lending D1, D2, 22i.5: A LOSS IS AN EVENT, NOT A RATE — and this world had none.
pub struct Losses {
    pub kind: u32,
    pub at_standing: u32,
    pub days_per_period: i64,
}

impl Mechanism for Losses {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::register::Standing;
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        // What each claim's standing IS: the last crossing said about it. One pass over this kind's
        // own rows, never a walk of the world's history per claim.
        let mut was: std::collections::HashMap<(u32, u32), Standing> = std::collections::HashMap::new();
        for &row in ctx.journal().of_kind(self.kind) {
            let subjects = ctx.journal().subjects_of(row);
            if let ([borrower, claim], Some(Value::Num(rank))) =
                (subjects, ctx.journal().says(row, self.at_standing))
            {
                let when = ctx.journal().period_of(row);
                let standing = match rank as i64 {
                    0 => Standing::Performing,
                    1 => Standing::NonPerforming { since: when },
                    2 => Standing::Impaired { since: when },
                    _ => Standing::WrittenOff { on: when },
                };
                was.insert((*borrower, *claim), standing);
            }
        }

        // What fell due on each borrower, per claim. A claim with nothing due this period is one
        // nobody could have missed a payment on.
        let mut fell: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            for &due in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(due);
                if ctx.schedules().paid(d) || ctx.schedules().due(d) > to || ctx.schedules().due(d) < from {
                    continue;
                }
                *fell.entry((row, ctx.schedules().instrument_of(d).0)).or_insert(0.0) += ctx.schedules().amount(d);
            }
        }

        let mut crossings: Vec<(u32, u32, f64)> = Vec::new();
        for (&(borrower, claim), &owed) in &fell {
            let who = PartyId(borrower);
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let could_pay = ctx.register().quantity(ctx.register().row(who, money));
            let standing = *was.get(&(borrower, claim)).unwrap_or(&Standing::Performing);
            // The only tolerance is the dust of the two numbers, never a grace band.
            let dust = crate::num::dust(2, &[owed, could_pay]);
            let Some(crossed) = crate::mechanisms::loss::crossed(
                who,
                InstrumentId::at(claim),
                standing,
                could_pay,
                owed,
                ctx.period(),
                dust,
            ) else {
                continue;
            };
            let rank = match crossed.now {
                Standing::Performing => 0.0,
                Standing::NonPerforming { .. } => 1.0,
                Standing::Impaired { .. } => 2.0,
                Standing::WrittenOff { .. } => 3.0,
            };
            crossings.push((borrower, claim, rank));
        }

        for (borrower, claim, rank) in crossings {
            // A charge that is VISIBLE, never a reserve absorbing things quietly. It names the
            // borrower and the claim, so a holder can find its own.
            ctx.say(self.kind, &[borrower, claim], &[(self.at_standing, Value::Num(rank))], true);
        }
    }
}

/// A SELLER THAT HAS DELIVERED AND NOT BEEN PAID OFFERS TERMS.
pub struct TradeCredit {
    pub kind: u32,
    /// How much a seller will have out to ONE buyer at once. A PREFERENCE — its own limit.
    pub will_carry: &'static str,
    /// And how long it will wait. Its own, and it shortens when the seller is worried.
    pub will_wait: &'static str,
    pub days_per_period: i64,
}

impl Mechanism for TradeCredit {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let will_carry = ctx.params().amount(self.will_carry, crate::params::Denomination::Money);
        let will_wait = ctx.params().days(self.will_wait) as i64;
        let today = Day(i64::from(ctx.period()) * self.days_per_period);

        // What each seller already has out to each buyer, read off the relations it holds.
        let mut out: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::TRADE_CREDIT {
                continue;
            }
            let (seller, buyer) = ctx.agreements().between(a);
            if let Some(&amount) = ctx.agreements().terms(a).first() {
                *out.entry((seller.0, buyer.0)).or_insert(0.0) += amount;
            }
        }

        // The payments that are short, and who was to be paid by them.
        let mut offering: Vec<(crate::ledger::QueueId, PartyId, PartyId, f64)> = Vec::new();
        // How many sellers each payment owes. One payment can owe several — a coupon owes every
        // holder of the line — and a payment cannot half-wait, so the count is what says whether
        let mut payees: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
        for row in 0..ctx.wire().queue.len() as u32 {
            let q = crate::ledger::QueueId(row);
            if ctx.wire().queue.state_of(q) != crate::ledger::Waiting::Queued {
                continue;
            }
            let buyer = ctx.wire().queue.payer_of(q);
            let mut owed: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
            for leg in ctx.wire().queue.legs_of(q) {
                if let crate::ledger::Leg::Money { from, to, amount, .. } = *leg {
                    if from != to {
                        *owed.entry(to.0).or_insert(0.0) += amount;
                    }
                }
            }
            payees.insert(row, owed.len());
            for (seller, amount) in owed {
                let seller = PartyId(seller);
                if !ctx.parties().alive(seller) || !ctx.parties().alive(buyer) {
                    continue;
                }
                offering.push((q, seller, buyer, amount));
            }
        }

        let mut struck: Vec<(crate::ledger::QueueId, PartyId, PartyId, f64, Day)> = Vec::new();
        for (q, seller, buyer, amount) in offering {
            let already = *out.get(&(seller.0, buyer.0)).unwrap_or(&0.0);
            let view = crate::mechanisms::trade_credit::View {
                of: buyer,
                will_carry,
                will_wait_days: will_wait,
            };
            // `None` is a REFUSAL, and a refusal is a decision. The buyer's payment then runs out
            // of days as an arrear, which is what a seller that will not wait means.
            let Some(terms) =
                crate::mechanisms::trade_credit::offer(seller, buyer, amount, today, &view, already)
            else {
                continue;
            };
            *out.entry((seller.0, buyer.0)).or_insert(0.0) += amount;
            struck.push((q, seller, buyer, terms.amount, terms.due));
        }

        // Each seller decides for itself, and the PAYMENT is one.
        let mut waiting: std::collections::HashMap<u32, (usize, Day)> = std::collections::HashMap::new();
        for (q, seller, buyer, amount, due) in struck {
            // The terms are the relation — what is owed and when. The DUE DATE is what makes the
            // goods and the money two different moments (E1: a sale that settles instantly by
            ctx.agrees(crate::module::Agrees {
                kind: agreed::TRADE_CREDIT,
                one: seller,
                other: buyer,
                terms: vec![amount, due.0 as f64],
                until: Some(due),
            });
            ctx.say(self.kind, &[seller.0, buyer.0], &[(0, Value::Num(amount))], true);
            let at = waiting.entry(q.0).or_insert((0, due));
            at.0 += 1;
            if due.0 < at.1.0 {
                at.1 = due;
            }
        }
        // Sorted, because a `HashMap`'s own order would move the same payments on different days
        // between two runs of one world.
        let mut moves: Vec<(u32, Day)> = waiting
            .into_iter()
            .filter(|(row, (agreed, _))| payees.get(row) == Some(agreed))
            .map(|(row, (_, until))| (row, until))
            .collect();
        moves.sort_by_key(|(row, _)| *row);
        for (row, until) in moves {
            let q = crate::ledger::QueueId(row);
            // Terms that end sooner than the payment's own day are not time given, so nothing moves
            // and the payment keeps the day it had.
            if until.0 > ctx.wire().queue.late_after(q).0 {
                ctx.waits_for(q, until);
            }
        }
    }
}

/// A COMPANY FLOATS — and no company in this world had ever had shares.
pub struct Floating {
    pub kind: u32,
    /// What a bank says when it is below its capital requirement. A recapitalisation IS an equity
    /// issue, so it comes through this door and not a second one.
    pub short_of_capital: u32,
    /// How many shares a line comes into existence with. A TECHNOLOGY of the market: the count is a
    /// convention and what a share is WORTH is what the book crosses at.
    pub shares: &'static str,
    /// How long the flotation runs before it is over, one way or the other.
    pub takes: &'static str,
}

impl Mechanism for Floating {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::instruments::Class;
        let shares = ctx.params().count(self.shares);
        let takes = ctx.params().periods(self.takes) as u32;

        // The banks that said they are short of capital this period.
        let mut must_raise: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &row in ctx.journal().of_kind(self.short_of_capital) {
            if ctx.journal().period_of(row) == ctx.period() {
                if let Some(&who) = ctx.journal().subjects_of(row).first() {
                    must_raise.insert(who);
                }
            }
        }

        let mut floating: Vec<(PartyId, crate::ids::CurrencyCode)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // The profile answers whether this kind brings paper at all. A kind that does not is a
            // kind with no credit market to be turned down by.
            match ctx.registry().profile(ctx.parties().kind_of(who)) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            let mut listed = false;
            let mut unsold = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                match ctx.instruments().class_of(what) {
                    Class::Share => listed = true,
                    // What it brought and is still holding: paper nobody bought.
                    Class::Claim => {
                        unsold += ctx.register().quantity(ctx.register().row(who, what));
                    }
                    _ => {}
                }
            }
            // Two reasons to sell ownership, and a company already listed has neither. The lenders
            // would not take it (unsold paper), or it is a bank below its requirement and must
            if listed || (unsold <= 0.0 && !must_raise.contains(&row)) {
                continue;
            }
            if ctx.processes().running(afoot::FLOTATION).iter().any(|p| ctx.processes().owner(*p) == who) {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            floating.push((who, ctx.instruments().ccy_of(money)));
        }

        for (who, ccy) in floating {
            ctx.brings(crate::module::Brings {
                issuer: who,
                ccy,
                class: Class::Share,
                // Counted in SHARES, a unit that is not money and is not divided.
                unit: crate::ids::UnitId::at(0),
                // A share is not a claim: it carries no coupon and never matures (5 C4.b).
                coupon: None,
                matures: None,
                units: shares,
                // Shares trade on an EXCHANGE — orders rest and are matched as they arrive, priced
                // at the level the resting side was standing at.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Book,
                    seen_by: 1,
                    stands_for: Some(4),
                }),
                // A share owes nothing on a date. What it gets is the residual, and only if there
                // is one.
                owing: Vec::new(),
            });
            ctx.opens(crate::module::Opens {
                kind: afoot::FLOTATION,
                owner: who,
                closes: Some(ctx.period() + takes),
                size: shares,
            });
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(shares))], true);
        }
    }
}

/// AND IT OFFERS THEM, AT NO LEVEL.
pub struct Flotation {
    pub of_kind: u32,
}

impl crate::module::Participant for Flotation {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        if view.in_a_flotation() <= 0.0 {
            return Vec::new();
        }
        view.holdings().map(|row| crate::systems::book_of(view.line_of(row))).collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let shares = view.in_a_flotation();
        if shares <= 0.0 {
            return Vec::new();
        }
        let _ = m;
        // IT OFFERS NOTHING, AND THAT IS A STOPPED MECHANISM RATHER THAN A DECISION.
        Vec::new()
    }
}

/// §31 A1, B1, B3, C1, C1.a, 40 C5, 22i.8: A BANK READS ITS OWN CAPITAL AND ACTS ON IT.
pub struct BankCapital {
    pub kind: u32,
    /// What it says when it is below its requirement and has to raise. `Floating` reads it.
    pub short_by: u32,
    pub at_ratio: u32,
    /// The requirement, the backstop and the buffer. POLICY — the regulation's, and the one place
    /// they are stated.
    pub min_weighted: &'static str,
    pub min_leverage: &'static str,
    pub buffer: &'static str,
    /// 40 C5: the return a lender wants on what it puts out. Its own hurdle.
    pub hurdle: &'static str,
    pub days_per_period: i64,
}

impl Mechanism for BankCapital {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::bank_capital::{standing, Asset, Position, Rules, Weight};
        let rules = Rules {
            min_weighted: ctx.params().ratio(self.min_weighted),
            min_leverage: ctx.params().ratio(self.min_leverage),
            buffer: ctx.params().ratio(self.buffer),
        };
        let hurdle = ctx.params().ratio(self.hurdle);
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        // What each name is graded at — the WORST any house holds on it, because a bank that could
        // pick the kindest house would weigh its book by choosing its assessor.
        let mut worst: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let s = crate::stores::StandingId(row);
            if !ctx.standing().live(s) || ctx.standing().kind_of(s) != standing::GRADE {
                continue;
            }
            let rank = ctx.standing().terms(s)[0];
            worst
                .entry(ctx.standing().about(s).0)
                .and_modify(|r| if rank > *r { *r = rank })
                .or_insert(rank);
        }

        let mut acted: Vec<(PartyId, f64, bool, f64)> = Vec::new();
        for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What it holds, at what it is carried at, and what each weighs.
            let mut assets: Vec<Asset> = Vec::new();
            let mut money_at_hand = 0.0;
            for &row in ctx.register().of_holder(who) {
                let row = crate::ids::HoldingId(row);
                let line = ctx.register().instrument_of(row);
                let carried = ctx.register().quantity(row);
                if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                    money_at_hand += carried;
                }
                let issuer = ctx.instruments().issuer_of(line);
                // XI-3's two exceptions are exactly the parties that cannot be made to fail, and a
                // claim on one of them is the zero-weighted asset the standard means.
                let kind = ctx.parties().kind_of(issuer);
                let can_fail = kind != kinds::CENTRAL_BANK && kind != kinds::TREASURY;
                // A name nobody has graded weighs what an ungraded name weighs, which is what the
                // scale's bottom is for — not nothing, and not a number invented here.
                let grade = crate::mechanisms::ratings::Grade::at_rank(
                    *worst.get(&issuer.0).unwrap_or(&crate::mechanisms::ratings::Grade::Substantial.rank()),
                )
                .unwrap_or(crate::mechanisms::ratings::Grade::Substantial);
                assets.push(Asset {
                    carried,
                    weight: Weight::on(can_fail, crate::mechanisms::ratings::haircut(grade, 1.0) - 1.0),
                });
            }
            // And what it OWES — the money it issued that others hold, plus what falls due on it.
            let mut liabilities = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let (held, _) = ctx.register().held_total(what);
                liabilities += held - ctx.register().quantity(ctx.register().row(who, what));
            }
            let due_now: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            let position = Position {
                bank: who,
                assets,
                liabilities,
                // The layer between equity and senior paper. Nothing in this world issues one yet,
                // so it is none — which is a fact about the world and not a number chosen here.
                subordinated: 0.0,
                due_now,
                money_at_hand,
            };
            if position.carried() <= 0.0 {
                continue;
            }
            let how = standing(&position, rules);
            let ratio = position.capital() / position.carried();
            // 40 C5: what it is lending at now. A lender with no room asks for more of the price up
            // front, and both terms move together because both are read from the same position.
            let headroom = position.capital() / (rules.min_leverage + rules.buffer) - position.carried();
            acted.push((who, ratio, how.below_requirement, headroom));
        }

        for (who, ratio, below, headroom) in acted {
            // The standing is PUBLIC. A capital position nobody could read is one no depositor, no
            // lender and no assessor could act on.
            ctx.say(self.kind, &[who.0], &[(self.at_ratio, Value::Num(ratio))], true);
            // 40 C5, C5.a: the standard it is lending at, which is a READ of what it already
            // measures — the strain on its own book, its hurdle and its headroom — and never a
            if ratio > 0.0 && headroom > 0.0 {
                let standard = crate::mechanisms::housing::standard(1.0 / ratio, headroom, hurdle);
                ctx.now_stands(
                    standing::LENDING_STANDARD,
                    who,
                    PartyId::NONE,
                    vec![standard.income_multiple, standard.deposit_share],
                );
            }
            // And a bank below its requirement must RAISE. What it says here is what it is short
            // of, and `Floating` is the one writer of a company's shares — a recapitalisation IS an
            if below {
                ctx.say(self.short_by, &[who.0], &[(self.at_ratio, Value::Num(-headroom))], true);
            }
        }
    }
}

/// WHAT A COMPANY'S CAPITAL COSTS IT, AT THE MARGIN, NOW.
pub struct CostOfCapital {
    pub kind: u32,
    pub accounts: u32,
    pub at_income: u32,
    pub at_shares: u32,
    /// The mix it would raise at. A PREFERENCE — the management's own, and theirs.
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
                    // share last cost. A price is never turned into a return the other way.
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

/// A BANK SETS THE RATE IT PAYS ON DEPOSITS.
pub struct BankFunding {
    pub kind: u32,
    /// The benchmark fixing, which is what a money fund would earn. A CLEARED print or nothing
    /// (`Fixes` refuses a carried one), so a bank with no fixing to read sets no rate.
    pub fixing: u32,
    pub days_per_period: i64,
}

impl Mechanism for BankFunding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::bank_funding::{blended, will_pay_on_deposits, Funding, Source};
        // The last fixing. A rate nobody transacted is not a benchmark, so there may be none.
        let mut money_fund_yield: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.fixing) {
            if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                money_fund_yield = Some(rate);
            }
        }
        let Some(money_fund_yield) = money_fund_yield else { return };

        let mut set: Vec<(PartyId, f64)> = Vec::new();
        for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Its OWN mix, read off what it has issued and what it pays on each.
            let mut mix: Vec<Source> = Vec::new();
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let (held, _) = ctx.register().held_total(what);
                let outstanding = held - ctx.register().quantity(ctx.register().row(who, what));
                if outstanding <= 0.0 {
                    continue;
                }
                match ctx.instruments().class_of(what) {
                    crate::instruments::Class::Money => mix.push(Source {
                        kind: Funding::Deposits(crate::mechanisms::bank_funding::Class::Retail),
                        amount: outstanding,
                        // What it is paying now is what it last stood behind, and nothing where it
                        // has never set one — a bank that has not set a rate is not paying zero.
                        rate: match ctx.standing().of_party_about(who, PartyId::NONE, standing::DEPOSIT_RATE) {
                            Some(s) => ctx.standing().terms(s)[0],
                            None => continue,
                        },
                    }),
                    // Short, and it ROLLS — which is where a funding squeeze bites. Its rate is the
                    // coupon it promised, which is a TERM and not a price (5 C4.b).
                    crate::instruments::Class::Claim => mix.push(Source {
                        kind: Funding::Wholesale,
                        amount: outstanding,
                        rate: match ctx.instruments().coupon_of(what) {
                            Some(c) => c,
                            None => continue,
                        },
                    }),
                    _ => {}
                }
            }
            // `None` where it funds with nothing — answering zero would say it funds free.
            let own_wholesale_cost = match blended(&mix) {
                Some(cost) => cost,
                // A bank that has never funded wholesale has its own cost to find, and the
                // benchmark is the only thing it can read. That is a real starting position.
                None => money_fund_yield,
            };
            set.push((who, will_pay_on_deposits(own_wholesale_cost, money_fund_yield)));
        }

        for (who, rate) in set {
            // A POSTED rate — depositors respond to it, so it is one-sided terms the bank stands
            // behind until it changes them, and what it was paying stays readable beside it.
            ctx.now_stands(standing::DEPOSIT_RATE, who, PartyId::NONE, vec![rate]);
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(rate))], true);
        }
    }
}

/// A FIRM DECIDES TO INVEST, AND THE COMPARISON IS THE MECHANISM.
pub struct Building {
    pub kind: u32,
    /// What its capital costs it, published by the cost-of-capital row.
    pub costs: u32,
    /// The management's own patience and its own risk aversion above the cost of capital.
    /// PREFERENCES, and theirs.
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
        // How built-up each place is. One read over the register for the whole world, not one per
        // firm.
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
            // Its own outlook, and a firm with none has nothing to expect. Missing is missing: a
            // firm that has formed no view does not invest on a view somebody else has.
            let Some(sells) = ctx.outlooks().of(firm, crate::stores::about::HOW_MUCH_IT_SELLS) else {
                continue;
            };
            let Some(price) = ctx.outlooks().of(firm, crate::stores::about::WHAT_IT_SELLS_FOR) else {
                continue;
            };
            // What the ground it stands on does to a build. A firm in a crowded place commits more
            // money for the same plant.
            let where_it_is = ctx.parties().region_of(firm);
            let crowding = crate::places::crowding(
                crate::places::standing_in(&built, where_it_is),
                crowds_at,
            );
            let project = crate::mechanisms::cost_of_capital::Project {
                returns_per_period: sells * price,
                costs: sells * price * crowding,
                horizon,
                hurdle,
            };
            if !crate::mechanisms::cost_of_capital::worth_doing(&project, cost_of_capital) {
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

/// AND IT BUYS THE PLANT.
pub struct Builder {
    pub of_kind: u32,
}

impl crate::module::Participant for Builder {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        if view.in_a_programme() <= 0.0 {
            return Vec::new();
        }
        // It bids in the books of what it already holds — the lines it knows how to use. A firm
        // that bid in every book in the world would be a buyer of things it has no plant for.
        view.holdings().map(|row| crate::systems::book_of(view.line_of(row))).collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let commits = view.in_a_programme();
        if commits <= 0.0 {
            return Vec::new();
        }
        let line = crate::systems::line_of(m);
        // It bids against what the book last PRINTED, because its limit is money and an order is
        // pieces. A line with no print has nothing to bid against.
        let Some(print) = view.print(line) else { return Vec::new() };
        if print.price <= 0.0 {
            return Vec::new();
        }
        // It cannot commit more money than it has. Arithmetic about its own account.
        let can_pay = view.own_cash();
        let money = if commits < can_pay { commits } else { can_pay };
        let units = crate::clearing::whole_pieces(money / print.price);
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Buy,
            price: Some(print.price),
            qty: units,
        }]
    }
}



/// A CURRENCY PAIR CLEARS FROM REAL REASONS.
pub struct SpotFx {
    pub kind: u32,
    pub days_per_period: i64,
}

impl Mechanism for SpotFx {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::spot_fx::{clearing, Posted, Reason};
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        // Who OWES a money, and who HAS one. Both from what the party is, not from a side anybody
        // gave it.
        let mut owes: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        let mut has: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What money it banks in. A party with no account is in no pair.
            let Some(mine) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let my_ccy = ctx.instruments().ccy_of(mine).0;
            for &d in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(d);
                if ctx.schedules().paid(d) || ctx.schedules().due(d) > to {
                    continue;
                }
                let line = ctx.schedules().instrument_of(d);
                let owed_in = ctx.instruments().ccy_of(line).0;
                if owed_in == my_ccy {
                    continue;
                }
                // It owes a currency it has not got. That is a reason, and it is the reason.
                *owes.entry((row, owed_in)).or_insert(0.0) += ctx.schedules().amount(d);
            }
            // And what it holds of a money that is not the one it banks in.
            for &held in ctx.register().of_holder(who) {
                let held = crate::ids::HoldingId(held);
                let line = ctx.register().instrument_of(held);
                if ctx.instruments().class_of(line) != crate::instruments::Class::Money {
                    continue;
                }
                let ccy = ctx.instruments().ccy_of(line).0;
                if ccy == my_ccy {
                    continue;
                }
                *has.entry((row, ccy)).or_insert(0.0) += ctx.register().quantity(held);
            }
        }
        if owes.is_empty() || has.is_empty() {
            return;
        }

        // One book per currency being bought. The rate is that currency against the money the other
        // side is paying with, which is what "a pair" means.
        let mut pairs: std::collections::HashMap<u32, Vec<Posted>> = std::collections::HashMap::new();
        for (&(who, ccy), &amount) in &owes {
            // The worst rate it will take. A party that MUST have the money will pay what the
            // market asks — it has an obligation, not a view — so its reservation is what the pair
            let Some(rate) = ctx.prints().latest(InstrumentId::at(ccy), ctx.period()).map(|p| p.price) else {
                continue;
            };
            pairs.entry(ccy).or_default().push(Posted { who: PartyId(who), reason: Reason::OwesIt, quantity: amount, rate });
        }
        for (&(who, ccy), &amount) in &has {
            let Some(rate) = ctx.prints().latest(InstrumentId::at(ccy), ctx.period()).map(|p| p.price) else {
                continue;
            };
            pairs.entry(ccy).or_default().push(Posted { who: PartyId(who), reason: Reason::HasIt, quantity: -amount, rate });
        }

        let mut done: Vec<(u32, f64, usize, f64)> = Vec::new();
        for (&ccy, posted) in &pairs {
            let cleared = clearing(posted);
            // A pair nobody traded has NO rate. Nothing is carried forward and nothing is invented
            // to give it one.
            let Some(rate) = cleared.rate else { continue };
            done.push((ccy, rate, cleared.trades.len(), cleared.unfilled));
        }

        for (ccy, rate, trades, unfilled) in done {
            // One rate in force for the period, published — both valuation and settlement use it,
            // so it is a fact about the world and not one party's read.
            ctx.say(
                self.kind,
                &[],
                &[(0, Value::Num(rate)), (1, Value::Num(trades as f64)), (2, Value::Num(unfilled))],
                true,
            );
            let _ = ccy;
        }
    }
}

/// A FORWARD IS STRUCK, AND THE BASIS IS WHAT IT DEVIATES BY.
pub struct FxForwards {
    pub kind: u32,
    pub spot: u32,
    pub fixing: u32,
    /// How far out the forward is struck. A market CONVENTION.
    pub tenor: &'static str,
    pub days_per_period: i64,
}

impl Mechanism for FxForwards {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The tenor is DAYS and the year fraction is read from the dates, never the other way round
        // — a forward "of a quarter" is ninety days and the calendar says what that is as a year.
        let days = ctx.params().days(self.tenor) as i64;
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let matures = Day(from.0 + days);
        let tenor = days as f64 / 365.0;

        // The rate the pair last cleared at. A pair with no rate has no forward, because there is
        // nothing for the forward to be a rate FORWARD of.
        let mut spot: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.spot) {
            if ctx.journal().period_of(row) == ctx.period() {
                if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                    spot = Some(rate);
                }
            }
        }
        let Some(spot) = spot else { return };
        // And what the two moneys fund at. The fixing is the only transacted rate this world has,
        // so both legs read it until each currency has its own.
        let mut funding: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.fixing) {
            if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                funding = Some(rate);
            }
        }
        let Some(funding) = funding else { return };

        // Where it would sit if the arbitrage were free. THE CHECK, not the price.
        let parity = crate::mechanisms::fx_forwards::parity(spot, funding, funding, tenor);
        let mut hedging: Vec<(PartyId, u32, f64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(mine) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let my_ccy = ctx.instruments().ccy_of(mine).0;
            for &d in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(d);
                // Beyond this period: what falls due now is a SPOT problem and is bought spot.
                if ctx.schedules().paid(d) || ctx.schedules().due(d) <= Day(from.0 + self.days_per_period) {
                    continue;
                }
                let owed_in = ctx.instruments().ccy_of(ctx.schedules().instrument_of(d)).0;
                if owed_in == my_ccy {
                    continue;
                }
                hedging.push((who, owed_in, ctx.schedules().amount(d)));
            }
        }
        if hedging.len() < 2 {
            // A hedge needs a counterparty holding the other side. One party wanting one is not a
            // market, and nothing is invented to be the other side of it.
            return;
        }

        let mut struck: Vec<(PartyId, PartyId, f64, f64)> = Vec::new();
        // The two ends of the book: whoever needs the most and whoever needs the least are the two
        // sides, and the rate is what they cross at.
        hedging.sort_by(|a, b| b.2.total_cmp(&a.2));
        let (buyer, _, size) = hedging[0];
        let (seller, _, _) = hedging[hedging.len() - 1];
        if buyer != seller {
            struck.push((buyer, seller, parity, size));
        }

        for (buyer, seller, rate, size) in struck {
            // The basis is the deviation, and it is a real price paid by whoever needs the money.
            // Where the forward crosses at parity the basis is nothing, which is what a market with
            let basis = rate - parity;
            ctx.agrees(crate::module::Agrees {
                kind: agreed::DERIVATIVE,
                one: buyer,
                other: seller,
                terms: vec![rate, size, tenor],
                until: Some(matures),
            });
            ctx.say(
                self.kind,
                &[buyer.0, seller.0],
                &[(0, Value::Num(rate)), (1, Value::Num(basis))],
                true,
            );
        }
    }
}

/// A POOL PUBLISHES ITS NAV, AND A HOLDER SUBSCRIBES AT IT.
pub struct Subscribing {
    pub kind: u32,
    /// How much of its spare money a holder will put into one pool. A PREFERENCE.
    pub commits: &'static str,
}

impl Mechanism for Subscribing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let commits = ctx.params().ratio(self.commits);

        let mut navs: Vec<(PartyId, f64, f64)> = Vec::new();
        for &pool in ctx.parties().of_kind(kinds::FUND) {
            let who = PartyId(pool);
            if !ctx.parties().alive(who) {
                continue;
            }
            // At cleared prices, and there is one read of that in the engine.
            let Some(at_market) =
                crate::instruments::book_value(who, ctx.register(), ctx.instruments(), ctx.prints(), ctx.period())
            else {
                continue;
            };
            // Its shares are what it has already sold, which is what its subscriptions say.
            let mut shares = 0.0;
            let mut owed = 0.0;
            for &a in ctx.agreements().of_party(who) {
                let a = crate::stores::AgreementId(a);
                if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::SUBSCRIPTION {
                    continue;
                }
                if let [held, _] = ctx.agreements().terms(a) {
                    shares += held;
                }
            }
            let due: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d))
                .map(|d| ctx.schedules().amount(d))
                .sum();
            owed += due;
            let book = crate::mechanisms::redeemable::Book {
                assets_at_market: at_market,
                liabilities: owed,
                shares,
            };
            // `None` where there are no shares. A pool with none has no per-share value, and a
            // first subscription therefore buys at what the pool is worth per share it is about to
            let nav = match book.nav() {
                Some(nav) => nav,
                None if at_market > 0.0 => at_market,
                None => continue,
            };
            if nav <= 0.0 {
                continue;
            }
            navs.push((who, nav, shares));
        }

        let mut subscribing: Vec<(PartyId, PartyId, f64, f64)> = Vec::new();
        for (pool, nav, _) in &navs {
            // The NAV is published. It is a fact about the pool that anybody may read, which is
            // what makes a subscription possible at all.
            ctx.say(self.kind, &[pool.0], &[(0, Value::Num(*nav))], true);
        }
        for (pool, nav, _) in &navs {
            for &holder in ctx.parties().of_kind(kinds::INSURER) {
                let holder = PartyId(holder);
                if !ctx.parties().alive(holder) {
                    continue;
                }
                let Some(money) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
                let cash = ctx.register().quantity(ctx.register().row(holder, money));
                // It subscribes with cash it has. `None` where it has none — a subscription by a
                // holder with no money is a share issued against nothing.
                let Some(shares) = crate::mechanisms::redeemable::subscribe(cash * commits, Some(*nav))
                else {
                    continue;
                };
                if shares <= 0.0 {
                    continue;
                }
                subscribing.push((*pool, holder, shares, shares * nav));
                break;
            }
        }

        for (pool, holder, shares, paid) in subscribing {
            // Cash one way and shares the other, in the same pass. The shares are the RELATION —
            // §13 A2's liability denominated in shares is exactly this row.
            let Some(from) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
            ctx.propose(
                vec![crate::ledger::Leg::Money {
                    from: holder,
                    to: pool,
                    instrument: from,
                    amount: paid,
                    receipt: crate::ledger::Receipt::Transfer,
                }],
                crate::ledger::Cause::CorporateAction,
                crate::ledger::Delivery::Nothing,
                "13 C1: a subscription gives the pool cash and the holder shares at NAV",
            );
            ctx.agrees(crate::module::Agrees {
                kind: agreed::SUBSCRIPTION,
                one: pool,
                other: holder,
                terms: vec![shares, paid],
                until: None,
            });
        }
    }
}

/// A BROKER LENDS TO A NAMED CLIENT, AND SETS WHAT IT REQUIRES.
pub struct Broking {
    pub kind: u32,
    /// What the broker thinks the book could move this period. Its own view, and it is what the
    /// requirement is made of.
    pub could_move: &'static str,
    /// What one broker will be exposed to one client for. Its own limit.
    pub limit: &'static str,
}

impl Mechanism for Broking {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::prime_brokerage::{requirement, Account, View};
        let could_move = ctx.params().ratio(self.could_move);
        let limit = ctx.params().amount(self.limit, crate::params::Denomination::Money);

        let brokers: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::DEALER)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if brokers.is_empty() {
            return;
        }

        let mut opening: Vec<(PartyId, PartyId, f64, f64)> = Vec::new();
        let mut calling: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (n, &client) in ctx.parties().of_kind(kinds::FUND).iter().enumerate() {
            let client = PartyId(client);
            if !ctx.parties().alive(client) {
                continue;
            }
            // One broker per client here — a client with two brokers is real and is §12 E3's, which
            // needs each to see only its own book. This world gives each client one.
            let broker = brokers[n % brokers.len()];
            // The broker sets a margin requirement on the whole portfolio, from its own view of the
            // risk — so a portfolio it cannot value is one it cannot margin. A requirement struck
            let mut assets = 0.0;
            let mut priced = true;
            for &row in ctx.register().of_holder(client) {
                let row = crate::ids::HoldingId(row);
                let line = ctx.register().instrument_of(row);
                // Money is not collateral a broker margins; it is what the margin is paid in.
                if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                    continue;
                }
                match crate::instruments::worth(
                    row,
                    ctx.register(),
                    ctx.instruments(),
                    ctx.prints(),
                    ctx.period(),
                ) {
                    Some(value) => assets += value,
                    None => priced = false,
                }
            }
            if !priced || assets <= 0.0 {
                continue;
            }
            // What this broker has already lent it, read off the relation it holds.
            let held = ctx
                .agreements()
                .of_party(client)
                .iter()
                .map(|a| crate::stores::AgreementId(*a))
                .find(|a| {
                    ctx.agreements().live(*a)
                        && ctx.agreements().kind_of(*a) == agreed::PRIME_BROKERAGE
                });
            // What this broker has already lent it. A client with no account has borrowed nothing
            // from it — that is the absence of a loan, not a loan of nothing — and an account whose
            let lent = match held.map(|a| ctx.agreements().terms(a).to_vec()) {
                Some(terms) => match terms.first() {
                    Some(&lent) => lent,
                    None => 0.0,
                },
                None => 0.0,
            };
            let account = Account {
                broker,
                client,
                assets,
                lent,
                short_proceeds_held: 0.0,
                stock_borrowed: 0.0,
                limit,
            };
            // The requirement, from the broker's OWN view of what the book could move.
            let view = View { move_it_expects: could_move, add_for_the_client: could_move };
            let required = requirement(assets, 0.0, &view);
            if held.is_none() {
                opening.push((broker, client, lent, limit));
            }
            // And where the account is short of what the broker requires, it CALLS. That is a real
            // demand on a named client for real money.
            let headroom = crate::mechanisms::prime_brokerage::headroom(&account, required);
            if headroom < 0.0 {
                calling.push((broker, client, -headroom));
            }
        }

        for (broker, client, lent, limit) in opening {
            // A loan from a NAMED lender. Terms `[lent, limit]`, and the limit is the broker's own
            // — E4's *no unlimited exposure* is this number existing at all.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::PRIME_BROKERAGE,
                one: broker,
                other: client,
                terms: vec![lent, limit],
                until: None,
            });
            ctx.say(self.kind, &[broker.0, client.0], &[(0, Value::Num(limit))], false);
        }
        for (broker, client, short) in calling {
            // A margin call the client cannot meet from cash is the first of the four doors, and it
            // is a WORKOUT — the client must find the money or sell.
            ctx.opens(crate::module::Opens {
                kind: afoot::WORKOUT,
                owner: client,
                closes: Some(ctx.period() + 1),
                size: short,
            });
            ctx.say(self.kind, &[broker.0, client.0], &[(0, Value::Num(-short))], false);
        }
    }
}

/// A FUND IS THE BUYER WHEN OTHERS ARE FORCED SELLERS.
pub struct Liquidity {
    pub of_kind: u32,
}

impl crate::module::Participant for Liquidity {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        // IF it has capacity. A fund with no room is not a buyer of anything, and that is the case
        // XI-2 turns on.
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        // The lines it knows — its own rows — never every book in the world.
        view.holdings().map(|row| crate::systems::book_of(view.line_of(row))).collect()
    }

    fn orders(&self, view: &crate::module::ParticipantView<'_>, m: crate::ids::MarketId) -> Vec<crate::clearing::Order> {
        let room = view.own_cash();
        if room <= 0.0 {
            return Vec::new();
        }
        let line = crate::systems::line_of(m);
        // No position that does not mark. A line nothing cleared is one it will not take on,
        // because it could not say afterwards what it was worth.
        let Some(print) = view.print(line) else { return Vec::new() };
        if print.price <= 0.0 {
            return Vec::new();
        }
        let units = crate::clearing::whole_pieces(room / print.price);
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Buy,
            // It bids at what the line last cleared at: it is buying from somebody who must sell,
            // and what it pays is what the book crosses at.
            price: Some(print.price),
            qty: units,
        }]
    }
}

/// STOCK IS LENT, AND THE FEE CLEARS.
pub struct StockLending {
    pub kind: u32,
    /// How much of what it holds a lender will put out at once. Its own limit, and it may be none.
    pub will_lend: &'static str,
}

impl Mechanism for StockLending {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::securities_lending::{clearing, Willing};
        let will_lend = ctx.params().ratio(self.will_lend);

        // The views held on each name, so the keenest short and the calmest holder are found rather
        // than assigned.
        let mut views: std::collections::HashMap<u32, Vec<(PartyId, f64)>> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let st = crate::stores::StandingId(row);
            if !ctx.standing().live(st) || ctx.standing().kind_of(st) != standing::OWN_VIEW {
                continue;
            }
            views
                .entry(ctx.standing().about(st).0)
                .or_default()
                .push((ctx.standing().held_by(st), ctx.standing().terms(st)[0]));
        }

        let mut struck: Vec<(PartyId, PartyId, InstrumentId, f64, f64)> = Vec::new();
        for (&on, holders) in &views {
            if holders.len() < 2 {
                continue;
            }
            let mut by_view = holders.clone();
            by_view.sort_by(|a, b| a.1.total_cmp(&b.1));
            let (borrower, worst) = by_view[by_view.len() - 1];
            let (_, best) = by_view[0];
            if worst <= best {
                continue;
            }
            for &line in ctx.instruments().of_issuer(PartyId(on)) {
                let what = InstrumentId::at(line);
                // The pool is what holders will actually lend, which is their own limit on their
                // own holding and never all of it.
                let mut pool: Vec<Willing> = Vec::new();
                for &row in ctx.register().of_instrument(what) {
                    let row = crate::ids::HoldingId(row);
                    let holder = ctx.register().holder_of(row);
                    if holder == borrower || holder == PartyId(on) {
                        continue;
                    }
                    let holds = ctx.register().free(row);
                    if holds <= 0.0 {
                        continue;
                    }
                    pool.push(Willing { holder, holds, will_lend: holds * will_lend });
                }
                if pool.is_empty() {
                    continue;
                }
                // The fee CLEARS. What the borrower wants against what the pool will lend, and the
                // schedules are what each holder posted.
                let wants = pool.iter().map(|w| w.will_lend).sum::<f64>();
                let schedules: Vec<(PartyId, f64, f64)> =
                    pool.iter().map(|w| (w.holder, w.will_lend, worst - best)).collect();
                let Some(fee) = clearing(wants, &pool, &schedules) else { continue };
                let lender = pool[0].holder;
                struck.push((lender, borrower, what, pool[0].will_lend, fee));
                break;
            }
        }

        for (lender, borrower, what, units, fee) in struck {
            // A loan of stock is a RELATION — terms `[the line, the units, the fee]` — and C1's
            // collateral, worth more than the loan, is what the two sides then post against it.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::SECURITIES_LOAN,
                one: lender,
                other: borrower,
                terms: vec![f64::from(what.0), units, fee],
                until: None,
            });
            ctx.say(self.kind, &[lender.0, borrower.0], &[(0, Value::Num(fee))], true);
        }
    }
}


/// WHAT A TREASURY DOES WHEN THE MONEY IS NOT THERE.
pub struct Sovereign {
    pub kind: u32,
    pub days_per_period: i64,
}

impl Mechanism for Sovereign {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::sovereign::{handle, Programme, Shortfall};
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        let mut handled: Vec<(PartyId, f64, f64)> = Vec::new();
        for &state in ctx.parties().of_kind(kinds::TREASURY) {
            let who = PartyId(state);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What falls due on paper already issued, and what it has committed to pay out. Both
            // are reads of its own schedule.
            let redemptions: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            // The buffer is a real holding of real money and not a line in a plan.
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let buffer = ctx.register().quantity(ctx.register().row(who, money));
            let programme = Programme { redemptions, outlays: 0.0, buffer };
            let short = programme.to_raise();
            // What it can defer is what it has queued and not yet made good: a payment already
            // waiting is one somebody is already not being paid, and deferring it further is the
            let deferrable: f64 = (0..ctx.wire().queue.len() as u32)
                .map(crate::ledger::QueueId)
                .filter(|q| ctx.wire().queue.state_of(*q) == crate::ledger::Waiting::Queued)
                .filter(|q| ctx.wire().queue.payer_of(*q) == who)
                .map(|q| {
                    ctx.wire()
                        .queue
                        .legs_of(q)
                        .iter()
                        .filter_map(|l| match *l {
                            crate::ledger::Leg::Money { from, to, amount, .. } if from != to => Some(amount),
                            _ => None,
                        })
                        .sum::<f64>()
                })
                .sum();
            let (what, size) = match handle(short, buffer, deferrable) {
                // A treasury whose buffer covers the period raises nothing, which is an answer.
                Shortfall::None => continue,
                Shortfall::FromTheBuffer { drawn } => (0.0, drawn),
                Shortfall::DeferAnOutlay { deferred } => (1.0, deferred),
                Shortfall::ComeBackToTheMarket { still_short } => (2.0, still_short),
            };
            handled.push((who, what, size));
        }

        for (who, what, size) in handled {
            // Each is a real act and it is SAID, because a shortfall handled silently is the
            // overdraft this clause exists to refuse — it would make being short cost nothing.
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(what)), (1, Value::Num(size))], true);
        }
    }
}

/// STOCK IS TIGHT OR IT IS NOT, AND STORING IT COSTS MONEY TO SOMEBODY.
pub struct Storing {
    pub kind: u32,
    /// What a period of storage costs, per unit. A TECHNOLOGY: a fact about warehouses.
    pub per_unit: &'static str,
}

impl Mechanism for Storing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let per_unit = ctx.params().price_per_unit(self.per_unit);

        // Who holds the storage, by place. A region with none has nobody to pay.
        let mut warehouses: std::collections::HashMap<u32, PartyId> = std::collections::HashMap::new();
        for &keeper in ctx.parties().of_kind(kinds::STOCKIST) {
            let who = PartyId(keeper);
            if ctx.parties().alive(who) {
                warehouses.entry(ctx.parties().region_of(who).0).or_insert(who);
            }
        }
        if warehouses.is_empty() {
            return;
        }

        // What was consumed, by line. ONE walk of the period's legs — Law 18: the traversal is free
        // to change and the mechanism is not, and a walk per line over half a million legs is the
        let mut consumed_of: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for n in ctx.wire().in_period(ctx.period()) {
            for leg in ctx.wire().legs_of(n) {
                if let crate::ledger::Leg::Destroy { instrument, qty, .. } = *leg {
                    *consumed_of.entry(instrument.0).or_insert(0.0) += qty;
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
            // A line with no `Destroy` leg this period had none consumed — the walk above saw every
            // leg, so its absence is the answer and not a number standing in for one.
            let consumed = match consumed_of.get(&row) {
                Some(&units) => units,
                None => 0.0,
            };
            if let Some(t) = crate::mechanisms::commodities::tightness(held, consumed) {
                tight.push((row, t));
            }
            // And everybody holding it pays for the storage, to the keeper of its own place.
            for &holding in ctx.register().of_instrument(line) {
                let holding = crate::ids::HoldingId(holding);
                let holder = ctx.register().holder_of(holding);
                if !ctx.parties().alive(holder) {
                    continue;
                }
                let Some(&keeper) = warehouses.get(&ctx.parties().region_of(holder).0) else { continue };
                if keeper == holder {
                    continue;
                }
                let units = ctx.register().quantity(holding);
                let (to, fee) = crate::mechanisms::commodities::storage_fee(units, per_unit, keeper);
                if fee > 0.0 {
                    charging.push((holder, to, fee));
                }
            }
        }

        for (line, t) in tight {
            // The measure of scarcity, published. It is what a price has left to ration with, and a
            // world that could not say it could not tell a squeeze from a glut.
            ctx.say(self.kind, &[], &[(0, Value::Num(f64::from(line))), (1, Value::Num(t))], true);
        }
        for (holder, keeper, fee) in charging {
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
            // Two named sides, in the same pass. A storage cost that came off a number without
            // reaching anybody would be the one-sided flow Law 5 is about.
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

/// DWELLINGS ARE LET AND SOLD, AND BOTH PRICES CLEAR.
pub struct Housing {
    pub kind: u32,
    pub lets: u32,
    /// What a dwelling costs its owner to keep, per period.
    pub upkeep: &'static str,
    /// What share of its money a household will put towards a roof. A PREFERENCE.
    pub will_spend: &'static str,
}

impl Mechanism for Housing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::housing::{can_bid, clearing, Bid, Offer, Standard};
        let will_spend = ctx.params().ratio(self.will_spend);
        let _upkeep = ctx.params().price_per_unit(self.upkeep);

        // The standard each lender is currently lending at. The KEENEST is what a buyer faces,
        // because a buyer takes the best offer it can find and lenders compete for it.
        let mut keenest: Option<Standard> = None;
        for row in 0..ctx.standing().len() as u32 {
            let st = crate::stores::StandingId(row);
            if !ctx.standing().live(st) || ctx.standing().kind_of(st) != standing::LENDING_STANDARD {
                continue;
            }
            let terms = ctx.standing().terms(st);
            let here = Standard { income_multiple: terms[0], deposit_share: terms[1] };
            keenest = Some(match keenest {
                Some(best) if best.income_multiple >= here.income_multiple => best,
                _ => here,
            });
        }
        // A buyer with no lender cannot bid. Missing is missing — it does not bid cash it has not
        // got, and it does not bid on a standard nobody is offering.
        let Some(standard) = keenest else { return };

        // The dwellings, where they are, and who lives in them. A dwelling is a structure its
        // holder holds; the occupier is its holder until a tenancy says otherwise.
        let mut offers: Vec<Offer> = Vec::new();
        let mut spare: Vec<(PartyId, crate::ids::RegionId)> = Vec::new();
        for row in 0..ctx.instruments().len() as u32 {
            let line = InstrumentId::at(row);
            if !crate::places::is_a_structure(ctx.registry(), line) {
                continue;
            }
            for &holding in ctx.register().of_instrument(line) {
                let holding = crate::ids::HoldingId(holding);
                let owner = ctx.register().holder_of(holding);
                if !ctx.parties().alive(owner) || ctx.register().free(holding) <= 0.0 {
                    continue;
                }
                let at = ctx.parties().region_of(owner);
                // It will not sell below what it owes or what a dwelling costs to build there,
                // whichever is more — and the build cost is higher where more already stands. What
                let Some(print) = ctx.prints().latest(line, ctx.period()) else { continue };
                let owed: f64 = ctx
                    .schedules()
                    .of_payer(owner)
                    .iter()
                    .map(|r| crate::stores::DueId(*r))
                    .filter(|d| !ctx.schedules().paid(*d))
                    .map(|d| ctx.schedules().amount(d))
                    .sum();
                offers.push(Offer::reserving(owner, at, owed, print.price));
                // And an owner holding more than one has a roof to let.
                if ctx.register().quantity(holding) > 1.0 {
                    spare.push((owner, at));
                }
            }
        }
        if offers.is_empty() {
            return;
        }

        // What each household can fund. Its income is what it has been paid — read off the wire —
        // and its deposit is what it holds.
        let mut bids: Vec<Bid> = Vec::new();
        let mut renting: Vec<(PartyId, crate::ids::RegionId, f64)> = Vec::new();
        for &household in ctx.parties().of_kind(kinds::HOUSEHOLD) {
            let who = PartyId(household);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let deposit = ctx.register().quantity(ctx.register().row(who, money)) * will_spend;
            if deposit <= 0.0 {
                continue;
            }
            // What it expects to earn is its OWN outlook, and a household that has formed none bids
            // what it holds — not an income of zero, which would be a number nobody has, but a bid
            let at = ctx.parties().region_of(who);
            let bidding = match ctx.outlooks().of(who, crate::stores::about::WHAT_IT_KEEPS_EARNING) {
                Some(income) => can_bid(income, deposit, &standard),
                None => deposit,
            };
            bids.push(Bid { buyer: who, at, bidding });
            renting.push((who, at, deposit));
        }

        // Per LOCATION. There is no single housing market, so each place clears on its own and a
        // print in one says nothing about another.
        let mut sold: Vec<(PartyId, PartyId, f64)> = Vec::new();
        let mut printed: Vec<(u32, f64)> = Vec::new();
        let mut places: Vec<u32> = offers.iter().map(|o| o.at.0).collect();
        places.sort_unstable();
        places.dedup();
        for place in places {
            let at = crate::ids::RegionId::at(place);
            let cleared = clearing(&bids, &offers, at);
            if let Some(print) = cleared.print {
                printed.push((place, print));
            }
            sold.extend(cleared.trades);
        }

        // And the letting session — a spare roof, and a household without one.
        let mut let_to: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for (owner, at) in spare {
            let Some((tenant, _, can_pay)) = renting.iter().copied().find(|(_, where_it_is, _)| *where_it_is == at)
            else {
                continue;
            };
            if tenant == owner {
                continue;
            }
            // The rent is what this session crossed at — what the tenant will pay against a roof
            // that is standing empty, and an owner that will not let at it keeps it empty.
            let_to.push((owner, tenant, can_pay));
        }

        for (seller, buyer, price) in sold {
            // A loan from a NAMED lender, secured on the house. What the buyer cannot find itself
            // it borrows, and E3's *no mortgage without a lender's balance sheet* is the lender
            ctx.agrees(crate::module::Agrees {
                kind: agreed::MORTGAGE,
                one: seller,
                other: buyer,
                terms: vec![price, standard.deposit_share],
                until: None,
            });
            ctx.say(self.kind, &[seller.0, buyer.0], &[(0, Value::Num(price))], true);
        }
        for (owner, tenant, rent) in let_to {
            // A tenancy is a relation, and the rent is its term. It is what the occupier pays the
            // owner for the shelter it consumes — two named parties, and never imputed.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::TENANCY,
                one: owner,
                other: tenant,
                terms: vec![rent],
                until: None,
            });
            ctx.say(self.lets, &[owner.0, tenant.0], &[(0, Value::Num(rent))], true);
        }
        for (place, print) in printed {
            ctx.say(self.kind, &[], &[(0, Value::Num(f64::from(place))), (1, Value::Num(print))], true);
        }
    }
}


/// A COMPANY IS BID FOR, AND THE OWNERS DECIDE.
pub struct Control {
    pub kind: u32,
    /// What an acquirer wants on what it buys. Its own hurdle, and a PREFERENCE.
    pub hurdle: &'static str,
    /// The share of a company somebody must hold to control it, read off the outstanding count
    /// rather than declared — this is only how much of the rest a bid must reach.
    pub needs: &'static str,
}

impl Mechanism for Control {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::control::{tender, Bid, Consideration, Outcome, Owner};
        let hurdle = ctx.params().ratio(self.hurdle);
        let needs = ctx.params().ratio(self.needs);

        // THE ACQUIRER'S OWN VALUATION, of the target's EXPECTED earnings. It does not wait for a
        // published report and must not: a target being bid for OPENS ITS BOOKS to the bidder —
        let mut earned: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for n in ctx.wire().in_period(ctx.period()) {
            if ctx.wire().outcome_of(n) != crate::ledger::Outcome::Settled {
                continue;
            }
            for leg in ctx.wire().legs_of(n) {
                if let crate::ledger::Leg::Money { from, to, amount, receipt, .. } = *leg {
                    if from == to {
                        continue;
                    }
                    // What a company EARNS is what it sells, less what it pays for what it uses. A
                    // receipt says which a payment is, so this reads the leg's own word for it
                    match receipt {
                        crate::ledger::Receipt::Sale => {
                            *earned.entry(to.0).or_insert(0.0) += amount;
                            *earned.entry(from.0).or_insert(0.0) -= amount;
                        }
                        crate::ledger::Receipt::Wage | crate::ledger::Receipt::Tax => {
                            *earned.entry(from.0).or_insert(0.0) -= amount;
                        }
                        _ => {}
                    }
                }
            }
        }
        if earned.is_empty() {
            return;
        }

        let mut bidding: Vec<(PartyId, PartyId, f64, f64, f64)> = Vec::new();
        for (&target, &income) in &earned {
            let company = PartyId(target);
            if !ctx.parties().alive(company) {
                continue;
            }
            // Its shares, and who holds them. A company with none cannot be bid for.
            let Some(share) = ctx
                .instruments()
                .of_issuer(company)
                .iter()
                .map(|l| InstrumentId::at(*l))
                .find(|l| ctx.instruments().class_of(*l) == crate::instruments::Class::Share)
            else {
                continue;
            };
            let Some(print) = ctx.prints().latest(share, ctx.period()) else { continue };
            // The acquirer's OWN valuation. Whoever already holds the most of it is who has a
            // reason to take the rest — and what it is worth to that holder is what its own hurdle
            let mut owners: Vec<Owner> = Vec::new();
            let mut outstanding = 0.0;
            for &row in ctx.register().of_instrument(share) {
                let row = crate::ids::HoldingId(row);
                let who = ctx.register().holder_of(row);
                let units = ctx.register().quantity(row);
                if who == company || units <= 0.0 {
                    continue;
                }
                outstanding += units;
                // What holding is worth to THIS owner — what the market last printed, which is what
                // it could get for it now. Its own, and it is why some accept and some do not.
                owners.push(Owner { who, units, holding_is_worth: print.price });
            }
            if owners.len() < 2 || outstanding <= 0.0 {
                continue;
            }
            owners.sort_by(|a, b| b.units.total_cmp(&a.units));
            let acquirer = owners[0].who;
            let Some(worth) = crate::mechanisms::control::worth_to(income, hurdle) else { continue };
            let per_share = worth / outstanding;
            if per_share <= print.price {
                // It will not pay a premium it does not think is there. A bid below the market is
                // not a bid, and nothing is adjusted to make one.
                continue;
            }
            bidding.push((acquirer, company, per_share, outstanding * needs, print.price));
        }

        let mut done: Vec<(PartyId, PartyId, f64, f64, bool)> = Vec::new();
        for (acquirer, company, per_share, needed, market) in bidding {
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), acquirer) else { continue };
            let funded = ctx.register().quantity(ctx.register().row(acquirer, money));
            let Some(share) = ctx
                .instruments()
                .of_issuer(company)
                .iter()
                .map(|l| InstrumentId::at(*l))
                .find(|l| ctx.instruments().class_of(*l) == crate::instruments::Class::Share)
            else {
                continue;
            };
            let mut owners: Vec<Owner> = Vec::new();
            for &row in ctx.register().of_instrument(share) {
                let row = crate::ids::HoldingId(row);
                let who = ctx.register().holder_of(row);
                let units = ctx.register().quantity(row);
                if who == company || who == acquirer || units <= 0.0 {
                    continue;
                }
                owners.push(Owner { who, units, holding_is_worth: market });
            }
            let bid = Bid {
                acquirer,
                target: company,
                // What its lenders committed. A bid it cannot fund is not a bid, and here what it
                // can fund is what it holds — the credit market's half is §29 B2.b's.
                offering: Consideration { cash_per_share: per_share, shares_per_share: 0.0 },
                funded,
                on: Day(0),
            };
            // Management may resist, and its interests differ from the owners'. Nothing in this
            // world resists yet, which is an absence and not a defence of nothing.
            match tender(&bid, 0.0, &owners, needed, None) {
                Outcome::Accepted { units, paid, .. } => done.push((acquirer, company, units, paid, true)),
                Outcome::Refused { accepting_units, .. } => {
                    done.push((acquirer, company, accepting_units, 0.0, false))
                }
                Outcome::Unfunded { short_by } => done.push((acquirer, company, 0.0, short_by, false)),
            }
        }

        for (acquirer, company, units, paid, took) in done {
            if took {
                // §29 B: a takeover is a thing that runs and closes, so it is opened like one.
                ctx.opens(crate::module::Opens {
                    kind: afoot::TAKEOVER,
                    owner: acquirer,
                    closes: Some(ctx.period() + 1),
                    size: units,
                });
            }
            // A bid that nobody beats is not a proof that it was the right price, only that nobody
            // came — so what happened is said either way.
            ctx.say(
                self.kind,
                &[acquirer.0, company.0],
                &[(0, Value::Num(units)), (1, Value::Num(paid))],
                true,
            );
        }
    }
}

/// A DERIVATIVE POSITION IS MARKED, AND AN OFFSET DOES NOT REMOVE IT.
pub struct Derivatives {
    pub kind: u32,
    pub at_mark: u32,
}

impl Mechanism for Derivatives {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::derivative_layer::Position;

        let mut marked: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::DERIVATIVE {
                continue;
            }
            let (one, other) = ctx.agreements().between(a);
            let terms = ctx.agreements().terms(a);
            let [struck_at, notional, years] = match terms {
                [a, b, c] => [*a, *b, *c],
                _ => continue,
            };
            // It was agreed at a CLEARED price and it marks against one — never against a price
            // this world does not clear. What it is worth now is what that name's protection would
            let on = InstrumentId::at(struck_at as u32);
            let mark = match ctx.prints().latest(on, ctx.period()) {
                Some(print) => (print.price - struck_at) * notional,
                // A contract on something nothing has cleared does not mark, and a position that
                // does not mark is one whose holder has hidden its loss. It is said with no mark
                None => continue,
            };
            let position = Position {
                a: one,
                b: other,
                notional,
                struck_at,
                mark,
                years_left: years,
                cleared_at_house: None,
            };
            // One number, two reads. The other side's is its negative, and it is read rather than
            // written a second time.
            if let Some(to_one) = position.mark_to(one) {
                marked.push((one, other, to_one));
            }
        }

        for (one, other, mark) in marked {
            // Per counterparty. A party's exposure to one name is not reduced by an offsetting
            // contract with another, so this is said naming both — netting them across
            ctx.say(self.kind, &[one.0, other.0], &[(self.at_mark, Value::Num(mark))], false);
        }
    }
}

/// THE TIER THAT IS TOO SMALL FOR THE BOND MARKET.
pub struct SmallBusiness {
    pub kind: u32,
    /// The size at which a borrower reaches the bond market. A TECHNOLOGY of the market: below it
    /// the issue costs more than it raises.
    pub reaches_the_bond_market_at: &'static str,
    pub days_per_period: i64,
}

impl Mechanism for SmallBusiness {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::small_business::Cell;
        let reaches = ctx.params().amount(self.reaches_the_bond_market_at, crate::params::Denomination::Money);
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        let mut tier: Vec<(PartyId, f64, bool)> = Vec::new();
        for &small in ctx.parties().of_kind(kinds::SMALL_FIRM) {
            let who = PartyId(small);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Observable characteristics, every one of them a read.
            let size = equity(who, ctx.register(), ctx.instruments(), ctx.claims());
            let owes: f64 = ctx
                .instruments()
                .of_issuer(who)
                .iter()
                .map(|i| ctx.schedules().outstanding(InstrumentId::at(*i)))
                .sum();
            let cell = Cell {
                who,
                weight: f64::from(ctx.parties().weight(who)),
                size,
                region: ctx.parties().region_of(who),
                leverage: if size > 0.0 { owes / size } else { f64::INFINITY },
                // Coverage is what it earns against what it owes, and a borrower with no published
                // earnings has none that anybody can read.
                coverage: 0.0,
                bank_dependent: true,
            };
            // It outgrew bank-dependence — large enough to reach the bond market. That is a fact
            // about its size against a market convention, and never a label anybody set.
            let dependent = !cell.outgrew(reaches);
            // And whether THIS borrower can meet what falls due. One borrower's own arithmetic,
            // never a band's average.
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            let cash = ctx.register().quantity(ctx.register().row(who, money));
            let due: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            tier.push((who, cell.leverage, dependent && cash < due));
        }

        for (who, leverage, cannot_meet) in tier {
            // Which borrowers have no alternative, and which of those cannot meet what falls due —
            // the two facts a credit tightening needs to bite here first and hardest.
            ctx.say(
                self.kind,
                &[who.0],
                &[(0, Value::Num(leverage)), (1, Value::Flag(cannot_meet))],
                true,
            );
        }
    }
}

/// A REGION'S ACCOUNTS ARE A READ OF WHAT ACTUALLY CROSSED.
pub struct CrossBorder {
    pub kind: u32,
    pub at_current: u32,
    pub at_financial: u32,
}

impl Mechanism for CrossBorder {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::cross_border::{Entry, Flow};

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
                // What the payment was FOR decides which account it lands in. It is read off the
                // leg's own receipt, which the writer had to state — never guessed from the
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
                    amount,
                    // What money this is, read off the instrument that IS it. The leg used to carry
                    // a second copy and nothing validated it, so these accounts were the only
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
                crate::mechanisms::cross_border::current_account(at, &flows),
                crate::mechanisms::cross_border::financial_account(at, &flows),
                // The two are the same flows read twice, so this is the discrepancy and it is
                // REPORTED with a size rather than asserted away (Law 7's dust, not a band).
                crate::mechanisms::cross_border::imbalance(at, &flows, 2),
            ));
        }
        // And the world closes. Every region's surplus is somebody's deficit, so the sum over all
        // of them is dust or it is a finding about a flow with one side.
        let closes = crate::mechanisms::cross_border::world_closes(&regions, &flows, regions.len());

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

/// A FUND CALLS ITS COMMITMENTS, AND THE INVESTOR MUST HAVE THE MONEY.
pub struct Calling {
    pub kind: u32,
    /// What share of an uncalled commitment a fund draws at once. Its own.
    pub draws: &'static str,
}

impl Mechanism for Calling {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let draws = ctx.params().ratio(self.draws);

        let mut calling: Vec<(PartyId, PartyId, f64)> = Vec::new();
        for row in 0..ctx.agreements().len() as u32 {
            let a = crate::stores::AgreementId(row);
            if !ctx.agreements().live(a) || ctx.agreements().kind_of(a) != agreed::SUBSCRIPTION {
                continue;
            }
            let (fund, investor) = ctx.agreements().between(a);
            if !ctx.parties().alive(fund) || !ctx.parties().alive(investor) {
                continue;
            }
            // Pro rata on what is UNCALLED. A commitment fully drawn has nothing left to call,
            // which is an answer and not a call of nothing.
            let Some(&committed) = ctx.agreements().terms(a).first() else { continue };
            let owed = committed * draws;
            if owed <= 0.0 {
                continue;
            }
            calling.push((fund, investor, owed));
        }

        for (fund, investor, owed) in calling {
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), investor) else { continue };
            // The investor must hold liquidity against a call it did not choose the timing of. If
            // it has not got it the payment waits in the queue like any other and can run out of
            ctx.propose(
                vec![crate::ledger::Leg::Money {
                    from: investor,
                    to: fund,
                    instrument: money,
                    amount: owed,
                    receipt: crate::ledger::Receipt::Transfer,
                }],
                crate::ledger::Cause::Payment,
                crate::ledger::Delivery::Nothing,
                "29 A2: a call is pro rata on uncalled commitments, and it is real money",
            );
            ctx.say(self.kind, &[fund.0, investor.0], &[(0, Value::Num(owed))], false);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mechanisms::employment::Wages;
    use crate::mechanisms::expectations::Forming;
    use super::*;
    use crate::assembly::World;
    use crate::calendar::Day;
    use crate::ids::{CurrencyCode, RegionId, UnitId};
    use crate::parties::Representation;

    /// A world with a central bank, a bank, and two customers of it.
    fn world() -> (World, PartyId, PartyId, PartyId, InstrumentId) {
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 1, 0);
        let _reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let cash = w.instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        let worker = w.parties.add(kinds::HOUSEHOLD, RegionId::at(0), bank, Representation::Cell, 100, 0);
        // A mechanism holds the ID of the number it acts on, so a test that runs one declares the
        // number first — half, here, because half the way is easy to check by eye.
        w.params.declare(crate::params::ParamDecl {
            id: MEMORY.to_string(),
            value: 0.5,
            unit: "weight on what just happened".to_string(),
            dimension: crate::params::Dimension::Ratio,
            kind: crate::params::Kind::Preference,
            owner: crate::params::Owner::Model,
            why: "what this test forms outlooks with".to_string(),
        });
        (w, bank, firm, worker, cash)
    }

    const MEMORY: &str = "test.outlook.memory";
    /// The standing area at which a build draws twice, for the tests that run a line.
    const CROWDS_AT: &str = "test.building.crowds_at";
    /// How much cover a firm wants on its shelf, for the tests that run a line.
    const COVER: &str = "test.firm.cover";

    /// A mechanism holds the ID of the number it acts on, so a world that runs one declares it. Ten
    /// square km, because a place carrying ten is then exactly twice as dear to build in.
    fn declare_crowding(w: &mut World) {
        w.params.declare(crate::params::ParamDecl {
            id: CROWDS_AT.to_string(),
            value: 10.0,
            unit: "square km standing".to_string(),
            dimension: crate::params::Dimension::SquareKm,
            kind: crate::params::Kind::Technology,
            owner: crate::params::Owner::Model,
            why: "what this test builds against".to_string(),
        });
    }

    fn ran(w: &mut World, m: &dyn Mechanism) {
        let mut ctx = MechanismContext::of(
            w.period,
            crate::module::Stores {
                claims: &w.claims,
                parties: &w.parties,
                instruments: &mut w.instruments,
                register: &w.register,
                prints: &w.prints,
                journal: &w.journal,
                params: &w.params,
                agreements: &w.agreements,
                schedules: &w.schedules,
                outlooks: &w.outlooks,
                standing: &w.standing,
                making: &w.making,
                registry: &w.registry,
                processes: &w.processes,
                wire: &w.wire,
            },
        );
        m.run(&mut ctx);
        let asked = ctx.taken();
        for p in asked.proposed {
            let ins = crate::ledger::Instruction { legs: &p.legs, cause: p.cause, delivery: p.delivery };
            w.wire.settle(
                &ins,
                w.period,
                &mut crate::ledger::Settling {
                    register: &mut w.register,
                    journal: &mut w.journal,
                    parties: &w.parties,
                    instruments: &mut w.instruments,
                    calendar: &w.calendar,
                    says: w.says,
                },
            );
        }
        for s in asked.said {
            w.journal.say(w.period, 0, s.kind, &s.subjects, &s.data, s.public);
        }
        for (who, subject, level) in asked.formed {
            w.outlooks.form(who, subject, level, w.period);
        }
        for due in asked.settled {
            w.schedules.settle(due);
        }
        for who in asked.ceased {
            w.parties.cease(who);
        }
        for (on, holder, owed, ranks) in asked.claimed {
            w.claims.against(on, holder, owed, ranks);
        }
        for (claim, amount) in asked.repaid {
            w.claims.pays(claim, amount);
        }
        for (owner, what, units, cost, ready) in asked.started {
            w.making.starts(owner, what, units, cost, w.period, ready);
        }
        for batch in asked.finished {
            w.making.finishes(batch);
        }
        for (kind, who, about, terms) in asked.stood {
            w.standing.stands(kind, who, about, &terms, w.period);
        }
    }

    #[test]
    fn what_falls_due_this_period_is_paid_to_whoever_holds_the_line() {
        // The mechanism the whole credit side rests on. The beneficiary is read off the register —
        // never a second list of who is owed what.
        let (mut w, bank, firm, _worker, cash) = world();
        let loan = w.instruments.issue(firm, CurrencyCode::at(0), Class::Claim, UnitId::at(0), Some(0.04), Some(Day(700)));
        w.register.credit(bank, loan, 1_000.0, 1.0, 0);
        w.register.money_delta(firm, cash, 500.0);
        let due = w.schedules.owes(loan, firm, Day(3), 40.0, Owing::Interest);

        w.period = 0;
        ran(&mut w, &Servicing { days_per_period: 7 });

        assert_eq!(w.register.quantity(w.register.row(firm, cash)), 460.0);
        assert_eq!(w.register.quantity(w.register.row(bank, cash)), 40.0);
        assert!(w.schedules.paid(due), "and the schedule knows it was paid");
        assert_eq!(w.schedules.outstanding(loan), 0.0);
    }

    #[test]
    fn a_coupon_reaches_every_holder_in_proportion_to_what_it_holds() {
        // Register E1, A2.a, Appendix B #10. This paid the FIRST row that was not the issuer, in
        // full: a line held by three parties paid all of it to one of them, and what the other two
        let (mut w, bank, firm, worker, cash) = world();
        let other = w.parties.add(kinds::BANK, RegionId::at(0), bank, Representation::Named, 1, 0);
        let loan = w.instruments.issue(firm, CurrencyCode::at(0), Class::Claim, UnitId::at(0), Some(0.04), Some(Day(700)));
        // 500 + 300 + 200 = 1,000 units outstanding, and 100 falling due on them.
        w.register.credit(bank, loan, 500.0, 1.0, 0);
        w.register.credit(other, loan, 300.0, 1.0, 0);
        w.register.credit(worker, loan, 200.0, 1.0, 0);
        w.register.money_delta(firm, cash, 500.0);
        let due = w.schedules.owes(loan, firm, Day(3), 100.0, Owing::Interest);

        w.period = 0;
        ran(&mut w, &Servicing { days_per_period: 7 });

        assert_eq!(w.register.quantity(w.register.row(bank, cash)), 50.0);
        assert_eq!(w.register.quantity(w.register.row(other, cash)), 30.0);
        assert_eq!(w.register.quantity(w.register.row(worker, cash)), 20.0);
        // And the issuer paid the whole of it, once — both sides of one obligation.
        assert_eq!(w.register.quantity(w.register.row(firm, cash)), 400.0);
        assert!(w.schedules.paid(due));
    }

    #[test]
    fn a_line_the_issuer_holds_itself_owes_nothing_to_anybody() {
        // Its own line on its own book is not a debt to itself, which is what "issued and
        // outstanding" means. Paying itself a coupon would move money in a circle and mark the
        let (mut w, _bank, firm, _worker, cash) = world();
        let loan = w.instruments.issue(firm, CurrencyCode::at(0), Class::Claim, UnitId::at(0), Some(0.04), Some(Day(700)));
        w.register.credit(firm, loan, 1_000.0, 1.0, 0);
        w.register.money_delta(firm, cash, 500.0);
        w.schedules.owes(loan, firm, Day(3), 40.0, Owing::Interest);

        w.period = 0;
        ran(&mut w, &Servicing { days_per_period: 7 });
        assert_eq!(w.register.quantity(w.register.row(firm, cash)), 500.0);
        assert_eq!(w.schedules.outstanding(loan), 40.0, "it still falls due on whoever buys it");
    }

    #[test]
    fn an_issuer_short_of_its_coupon_fails_the_whole_of_it_and_not_part() {
        // XI-5, 5 D2: one obligation, one instruction. A pass that paid some holders and not others
        // would make `settles(due)` a lie either way, and the arrear it left would be for an amount
        let (mut w, bank, firm, worker, cash) = world();
        let loan = w.instruments.issue(firm, CurrencyCode::at(0), Class::Claim, UnitId::at(0), Some(0.04), Some(Day(700)));
        w.register.credit(bank, loan, 500.0, 1.0, 0);
        w.register.credit(worker, loan, 500.0, 1.0, 0);
        // Enough for one holder's half and not for both.
        w.register.money_delta(firm, cash, 60.0);
        w.schedules.owes(loan, firm, Day(3), 100.0, Owing::Interest);

        w.period = 0;
        ran(&mut w, &Servicing { days_per_period: 7 });
        assert_eq!(w.register.quantity(w.register.row(bank, cash)), 0.0);
        assert_eq!(w.register.quantity(w.register.row(worker, cash)), 0.0);
        assert_eq!(w.register.quantity(w.register.row(firm, cash)), 60.0, "XI-5: nothing moved");
    }

    #[test]
    fn what_falls_due_later_is_not_paid_now() {
        // Calendar A1: "this period" is a read of dates, so a payment due next week stays due.
        let (mut w, bank, firm, _worker, cash) = world();
        let loan = w.instruments.issue(firm, CurrencyCode::at(0), Class::Claim, UnitId::at(0), Some(0.04), Some(Day(700)));
        w.register.credit(bank, loan, 1_000.0, 1.0, 0);
        w.register.money_delta(firm, cash, 500.0);
        w.schedules.owes(loan, firm, Day(30), 40.0, Owing::Interest);

        w.period = 0;
        ran(&mut w, &Servicing { days_per_period: 7 });
        assert_eq!(w.register.quantity(w.register.row(firm, cash)), 500.0);
        assert_eq!(w.schedules.outstanding(loan), 40.0);
    }

    #[test]
    fn an_engagement_pays_its_wage_and_an_ended_one_does_not() {
        // Employment is a relation, and the wage is what it pays. An engagement that ended is still
        // readable and pays nothing, which is what ending means.
        let (mut w, _bank, firm, worker, cash) = world();
        w.register.money_delta(firm, cash, 9_000.0);
        // The worker is a CELL of a hundred, the engagement is for all hundred, and a wage is per
        // person — so what the firm owes is a hundred wages. Paying one was the defect: a firm
        let of_them = f64::from(w.parties.weight(worker));
        let hired = w.agreements.strike(agreed::ENGAGEMENT, firm, worker, &[40.0, 35.0, of_them], Day(-100), None);

        w.period = 1;
        ran(&mut w, &Wages);
        assert_eq!(w.register.quantity(w.register.row(worker, cash)), 40.0 * of_them);

        w.agreements.end(hired);
        ran(&mut w, &Wages);
        assert_eq!(
            w.register.quantity(w.register.row(worker, cash)),
            40.0 * of_them,
            "an ended engagement pays nothing"
        );
    }

    #[test]
    fn two_parties_seeing_different_prices_form_different_outlooks() {
        // The disagreement is LOAD-BEARING. A world where everybody expected the same would trade
        // once and stop.
        let (mut w, _bank, firm, worker, _cash) = world();
        let one = w.instruments.issue(firm, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let two = w.instruments.issue(firm, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        w.register.credit(firm, one, 10.0, 1.0, 0);
        w.register.credit(worker, two, 10.0, 1.0, 0);
        w.prints.write(crate::prices::Print {
            instrument: one,
            market: crate::ids::MarketId::at(1),
            period: 1,
            price: 3.0,
            ccy: CurrencyCode::at(0),
            quoted_as: crate::prices::QuotedAs::Money,
            provenance: crate::prices::Provenance::Cleared,
        });
        w.prints.write(crate::prices::Print {
            instrument: two,
            market: crate::ids::MarketId::at(2),
            period: 1,
            price: 9.0,
            ccy: CurrencyCode::at(0),
            quoted_as: crate::prices::QuotedAs::Money,
            provenance: crate::prices::Provenance::Cleared,
        });

        w.period = 1;
        ran(&mut w, &Forming { memory: MEMORY });
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), Some(3.0));
        assert_eq!(w.outlooks.of(worker, about::WHAT_IT_SELLS_FOR), Some(9.0));
        assert_eq!(w.outlooks.spread_on(about::WHAT_IT_SELLS_FOR).len(), 2);
    }

    #[test]
    fn a_party_that_has_seen_nothing_forms_nothing_and_that_is_not_zero() {
        // Missing is missing. An outlook of zero is an expectation.
        let (mut w, _bank, firm, _worker, _cash) = world();
        w.period = 1;
        ran(&mut w, &Forming { memory: MEMORY });
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), None);
    }

    #[test]
    fn the_second_observation_moves_the_outlook_by_the_partys_own_memory() {
        // Adaptive, from its OWN history. The first observation IS the outlook.
        let (mut w, _bank, firm, _worker, _cash) = world();
        let line = w.instruments.issue(firm, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        w.register.credit(firm, line, 10.0, 1.0, 0);
        for (period, price) in [(1u32, 4.0), (2, 8.0)] {
            w.prints.write(crate::prices::Print {
                instrument: line,
                market: crate::ids::MarketId::at(1),
                period,
                price,
                ccy: CurrencyCode::at(0),
                quoted_as: crate::prices::QuotedAs::Money,
                provenance: crate::prices::Provenance::Cleared,
            });
            w.period = period;
            ran(&mut w, &Forming { memory: MEMORY });
        }
        // 4 first, then half the way from 4 to 8.
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), Some(6.0));
    }

    #[test]
    fn a_read_over_what_the_books_produced_publishes_and_proposes_nothing() {
        // Giving a benchmark a schedule would be inventing demand nobody has. What it does is
        // publish a read, and the count is what shows it ran.
        let (mut w, _bank, firm, worker, _cash) = world();
        let kind = w.journal.kinds.declare("benchmark.count");
        let before = w.register.version();
        w.period = 1;
        ran(&mut w, &Reads { kind, what: Counts::PartiesAlive });
        assert_eq!(w.register.version(), before, "a read moves nothing");
        let rows: Vec<u32> = w.journal.in_period(1).collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(w.journal.says(rows[0], 0), Some(Value::Num(4.0)));
        let _ = (firm, worker);
    }

    #[test]
    fn a_process_closes_when_its_period_comes_and_not_before() {
        // Nothing is immortal, a process included — and the KERNEL is what closes one.
        let (mut w, _bank, firm, _worker, _cash) = world();
        let systems: Vec<&dyn crate::assembly::System> = Vec::new();
        w.wire_up(&systems);
        w.processes.begin(afoot::CAPITAL_PROGRAMME, firm, 1, Some(3), 500.0);
        let p = crate::stores::ProcessId(0);
        w.period = 1;
        assert_eq!(w.step(&systems).closed, 0, "its period has not come");
        assert!(!w.processes.done(p));
        w.period = 2;
        assert_eq!(w.step(&systems).closed, 1, "and now it has");
        assert!(w.processes.done(p));
        // And it does not close twice: a process that ended is ended.
        assert_eq!(w.step(&systems).closed, 0);
    }

    /// A mill that holds its plant, its input and one worker — the smallest world that can make
    /// anything.
    fn a_mill() -> (World, PartyId, InstrumentId, InstrumentId, InstrumentId) {
        let (mut w, bank, firm, worker, _cash) = world();
        let flour = w.instruments.issue(firm, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let bread = w.instruments.issue(firm, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let mill = w.instruments.issue(bank, CurrencyCode::at(0), Class::Plant, UnitId::at(0), None, None);
        w.register.credit(firm, mill, 2.0, 1_000.0, 0);
        w.register.credit(firm, flour, 900.0, 0.5, 0);
        // `[wage per person, hours per person, headcount]`, the convention `agreed::ENGAGEMENT`
        // states. The worker is a cell of a hundred, so the hours it gives the line are a hundred
        let of_them = f64::from(w.parties.weight(worker));
        w.agreements.strike(agreed::ENGAGEMENT, firm, worker, &[80.0, 40.0, of_them], Day(-7), None);
        declare_crowding(&mut w);
        w.params.declare(crate::params::ParamDecl {
            id: COVER.to_string(),
            value: 0.0,
            unit: "multiple of what it expects to sell".to_string(),
            dimension: crate::params::Dimension::Ratio,
            kind: crate::params::Kind::Preference,
            owner: crate::params::Owner::Model,
            why: "these cases are about the OTHER reasons binding, so the shelf it wants is none".to_string(),
        });
        (w, firm, flour, bread, mill)
    }

    fn making(line: &crate::mechanisms::recipe::Line, plant: InstrumentId) -> Making {
        Making {
            makes: vec![Makes {
                line: line.clone(),
                plant,
                plant_is: crate::mechanisms::capital_programme::Plant {
                    life: 100,
                    upkeep_per_period: 1.0,
                    capacity_per_period: 150.0,
                },
            }],
            flow: crate::mechanisms::goods::CostFlow::FirstInFirstOut,
            crowds_at: CROWDS_AT,
            cover: COVER,
        }
    }

    #[test]
    fn a_firm_with_plant_inputs_hours_and_a_view_of_its_demand_actually_makes_something() {
        // 37 B1, B2, B3, B4: the inputs go NOW and the output arrives when the line is done — and
        // in between there is work in progress, owned, carrying what it cost.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);

        let flour_before = w.register.quantity(w.register.row(firm, flour));
        ran(&mut w, &making(&line, mill));

        // It wanted 200/0.98 = 204 starts, had capacity for 300 and flour for 450, so the batch
        // rounded it to 200. The 400 sacks have gone and the 196 loaves are ON THE LINE.
        assert_eq!(flour_before - w.register.quantity(w.register.row(firm, flour)), 400.0);
        assert_eq!(w.register.quantity(w.register.row(firm, bread)), 0.0, "it is not made yet");
        let on_the_line = w.making.held_by(firm);
        assert_eq!(on_the_line.len(), 1);
        assert_eq!(w.making.units(on_the_line[0]), 196.0);
        assert!(w.making.cost_carried(on_the_line[0]) > 0.0, "B3: it carries what it cost");

        // And the period it is ready, it comes off and becomes a thing the firm holds.
        w.period = 2;
        ran(&mut w, &making(&line, mill));
        assert_eq!(w.register.quantity(w.register.row(firm, bread)), 196.0);
    }

    #[test]
    fn what_it_made_carries_what_it_cost_and_the_cost_includes_the_plant_nobody_could_switch_off() {
        // B5, 33 A3: inputs at their own lot basis, plus the wages the hours cost, plus the
        // period's upkeep and depreciation on the plant that ran. B4: over what FINISHED, so the
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));
        // The cost is carried by the batch on the line, and it is what the lot is struck at when
        // the batch comes off — so what a unit cost is what went into IT, not what things cost
        w.period = 2;
        ran(&mut w, &making(&line, mill));

        let lots = w.register.lots(w.register.row(firm, bread));
        assert_eq!(lots.len(), 1);
        // 400 sacks at 0.5 is 200; 20 hours at 2 an hour is 40; the capital service is the mill's
        // own keep over what it can make. All of it over 196 loaves.
        assert!(lots[0].basis_per_unit > (200.0 + 40.0) / 196.0);
    }

    #[test]
    fn a_firm_with_no_view_of_its_own_demand_has_no_reason_to_start_a_line() {
        // Expected demand is the first reason, and it is the firm's OWN. Missing is missing — a
        // firm that has never sold anything does not produce as if it expected zero, it does not
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));
        assert!(w.making.held_by(firm).is_empty(), "it started nothing");

        // And it starts the moment it has one — which is a batch on the line, not a loaf.
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        ran(&mut w, &making(&line, mill));
        assert_eq!(w.making.held_by(firm).len(), 1);
    }

    #[test]
    fn the_same_build_draws_more_where_more_already_stands() {
        // 21i, 33 A4: congestion, not scarcity. Two identical firms build the same structure, one
        // on empty ground and one where the declared doubling area already stands.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, _firm, flour, _bread, mill) = a_mill();
        let cb = PartyId::at(0);
        let bank = PartyId::at(1);

        // Two places. A structure line, so the registry says it stands on something at all.
        let usd = w.registry.currency(cb);
        let country = w.registry.country(usd);
        let empty = w.registry.region(country);
        let crowded = w.registry.region(country);
        let shed = w.instruments.issue(bank, CurrencyCode::at(0), Class::Plant, UnitId::at(0), None, None);
        w.registry.stands_on(shed, 1.0);

        // Two builders with the same plant, the same input, the same hours and the same outlook —
        // alike in everything but where they are.
        let mut builder = |at| {
            let who = w.parties.add(kinds::FIRM, at, bank, Representation::Named, 1, 0);
            w.register.credit(who, mill, 2.0, 1_000.0, 0);
            w.register.credit(who, flour, 900.0, 0.5, 0);
            w.agreements.strike(agreed::ENGAGEMENT, who, cb, &[80.0, 40.0, 1.0], Day(-7), None);
            w.outlooks.form(who, about::HOW_MUCH_IT_SELLS, 100.0, 1);
            who
        };
        let on_empty = builder(empty);
        let on_crowded = builder(crowded);
        // Ten square km already standing where the second one builds, which is exactly the declared
        // doubling area — so its draw is twice, and a reader can check that by eye.
        let squatter = w.parties.add(kinds::FIRM, crowded, bank, Representation::Named, 1, 0);
        w.register.credit(squatter, shed, 10.0, 1.0, 0);

        w.period = 1;
        let line = Line::new(shed, vec![Recipe::new(shed, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));

        // Both built — congestion prices, it does not refuse.
        let drew = |w: &World, who: PartyId| 900.0 - w.register.quantity(w.register.row(who, flour));
        let easy = drew(&w, on_empty);
        let dear = drew(&w, on_crowded);
        assert!(easy > 0.0 && dear > 0.0, "a crowded place is dearer to build in, not closed");
        // Per unit of output, the crowded build drew twice the flour.
        let made_easy = w.making.held_by(on_empty).len();
        let made_dear = w.making.held_by(on_crowded).len();
        assert_eq!((made_easy, made_dear), (1, 1), "both started a batch");
        let units = |w: &World, who: PartyId| {
            let b = w.making.held_by(who)[0];
            w.making.units(b)
        };
        let per_unit_easy = easy / units(&w, on_empty);
        let per_unit_dear = dear / units(&w, on_crowded);
        let dust = crate::num::dust(4, &[per_unit_easy, per_unit_dear]);
        assert!(
            (per_unit_dear - 2.0 * per_unit_easy).abs() <= dust,
            "{per_unit_dear} against twice {per_unit_easy}"
        );
    }

    #[test]
    fn flour_is_milled_alike_everywhere() {
        // A line that stands nowhere is built the same wherever it is — and that is an ANSWER, not
        // a default: being a structure is a footprint in the registry, and flour has none.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        let cb = PartyId::at(0);
        let bank = PartyId::at(1);
        let usd = w.registry.currency(cb);
        let country = w.registry.country(usd);
        let crowded = w.registry.region(country);
        let shed = w.instruments.issue(bank, CurrencyCode::at(0), Class::Plant, UnitId::at(0), None, None);
        w.registry.stands_on(shed, 1.0);
        let squatter = w.parties.add(kinds::FIRM, crowded, bank, Representation::Named, 1, 0);
        w.register.credit(squatter, shed, 400.0, 1.0, 0);

        let baker = w.parties.add(kinds::FIRM, crowded, bank, Representation::Named, 1, 0);
        w.register.credit(baker, mill, 2.0, 1_000.0, 0);
        w.register.credit(baker, flour, 900.0, 0.5, 0);
        w.agreements.strike(agreed::ENGAGEMENT, baker, cb, &[80.0, 40.0, 1.0], Day(-7), None);
        w.outlooks.form(baker, about::HOW_MUCH_IT_SELLS, 100.0, 1);
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 100.0, 1);

        w.period = 1;
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));

        let drew = |w: &World, who: PartyId| 900.0 - w.register.quantity(w.register.row(who, flour));
        // Four hundred square km standing over the baker, and it draws exactly what the one on open
        // ground draws, because bread is not a building.
        assert_eq!(drew(&w, baker), drew(&w, firm));
    }

    #[test]
    fn production_follows_the_plant_and_not_the_party_kind() {
        // Law 15, 33 A2: a maker is whoever holds the plant. A bank that bought a mill makes flour,
        // and nothing here asks a party what it is.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, _firm, flour, bread, mill) = a_mill();
        let cb = PartyId::at(0);
        let bank = PartyId::at(1);
        w.period = 1;
        w.register.credit(bank, mill, 1.0, 1_000.0, 0);
        w.register.credit(bank, flour, 500.0, 0.5, 0);
        // `[wage per person, hours per person, headcount]` — the central bank is one named party,
        // so the headcount is one.
        w.agreements.strike(agreed::ENGAGEMENT, bank, cb, &[80.0, 40.0, 1.0], Day(-7), None);
        w.outlooks.form(bank, about::HOW_MUCH_IT_SELLS, 100.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));
        assert_eq!(w.making.held_by(bank).len(), 1, "the bank that bought a mill is making bread");
        w.period = 2;
        ran(&mut w, &making(&line, mill));
        assert!(w.register.quantity(w.register.row(bank, bread)) > 0.0);
    }

    /// A pool with a manager, a book of shares two households hold, and money it raised.
    fn a_pool() -> (World, PartyId, PartyId, InstrumentId, InstrumentId, PartyId, PartyId) {
        let (mut w, bank, manager, saver, cash) = world();
        let pool = w.parties.add(kinds::FUND, RegionId::at(0), bank, Representation::Named, 1, 0);
        let other = w.parties.add(kinds::HOUSEHOLD, RegionId::at(0), bank, Representation::Cell, 100, 0);
        let shares = w.instruments.issue(pool, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.register.credit(saver, shares, 300.0, 1.0, 0);
        w.register.credit(other, shares, 700.0, 1.0, 0);
        w.register.money_delta(pool, cash, 900.0);
        let mandate = w.agreements.strike(agreed::MANDATE, manager, pool, &[], Day(-30), None);
        let _ = mandate;
        (w, pool, manager, cash, shares, saver, other)
    }

    fn mandate_of(w: &World, pool: PartyId) -> crate::stores::AgreementId {
        let row = w
            .agreements
            .of_party(pool)
            .iter()
            .copied()
            .find(|r| w.agreements.kind_of(crate::stores::AgreementId(*r)) == agreed::MANDATE)
            .expect("the fixture struck one");
        crate::stores::AgreementId(row)
    }

    #[test]
    fn a_pool_under_a_live_mandate_is_left_alone() {
        // This mechanism is about the absence of a manager and does nothing while there is one.
        let (mut w, pool, _m, cash, _s, saver, _o) = a_pool();
        w.period = 1;
        let says = w.journal.kinds.declare("fund.orphaned");
        ran(&mut w, &Winding { says });
        assert_eq!(w.register.quantity(w.register.row(pool, cash)), 900.0);
        assert_eq!(w.register.quantity(w.register.row(saver, cash)), 0.0);
        assert!(w.parties.alive(pool));
    }

    #[test]
    fn a_manager_that_dies_leaves_its_pool_paying_its_holders_pro_rata_on_what_it_raised() {
        // The holders' claim is redeemable, so what the pool raised goes back to them in proportion
        // — 300 shares of 1,000 is 270 of 900, and it is the fund's own money moving over the
        let (mut w, pool, _m, cash, _s, saver, other) = a_pool();
        w.period = 1;
        let ended = mandate_of(&w, pool);
        w.agreements.end(ended);
        let says = w.journal.kinds.declare("fund.orphaned");
        ran(&mut w, &Winding { says });

        assert_eq!(w.register.quantity(w.register.row(saver, cash)), 270.0);
        assert_eq!(w.register.quantity(w.register.row(other, cash)), 630.0);
        // Every piece of it has a holder. The pool paid out what it had, and no more.
        assert_eq!(w.register.quantity(w.register.row(pool, cash)), 0.0);
    }

    #[test]
    fn a_pool_that_has_paid_everybody_and_holds_nothing_has_ended() {
        // Nothing is immortal, and a thing that ends says when. Law 6: it is not a schedule — the
        // pool ends because there is nothing left, which is arithmetic about what it holds.
        let (mut w, pool, _m, _c, shares, saver, other) = a_pool();
        w.period = 1;
        let ended = mandate_of(&w, pool);
        w.agreements.end(ended);
        // The holders have been paid and handed their shares back.
        w.register.debit(w.register.row(saver, shares), 300.0);
        w.register.debit(w.register.row(other, shares), 700.0);
        let says = w.journal.kinds.declare("fund.orphaned");
        ran(&mut w, &Winding { says });
        assert!(!w.parties.alive(pool));
    }

    #[test]
    fn a_pool_still_holding_something_is_not_wound_up_however_long_it_takes() {
        // No forced buyer. A book that will not take its stock leaves it unsold, and the pool is
        // still there next period — the absence of a buyer showing up as a duration.
        let (mut w, pool, _m, _c, shares, saver, other) = a_pool();
        let unsold = w.instruments.issue(PartyId::at(1), CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        w.register.credit(pool, unsold, 240.0, 1.0, 0);
        w.period = 1;
        let ended = mandate_of(&w, pool);
        w.agreements.end(ended);
        w.register.debit(w.register.row(saver, shares), 300.0);
        w.register.debit(w.register.row(other, shares), 700.0);
        let says = w.journal.kinds.declare("fund.orphaned");
        ran(&mut w, &Winding { says });
        assert!(w.parties.alive(pool), "it still holds 240 of something nobody bought");
    }

    #[test]
    fn an_estate_pays_its_claimants_in_rank_order_and_the_state_is_one_of_them() {
        // The estate paid the treasury 388 directly and the audit said the treasury had no claim on
        // it. Now the state is a CLAIMANT, standing behind the secured creditor, and it gets what
        use crate::mechanisms::estate::Rank;
        let (mut w, bank, secured, treasury, cash) = world();
        let estate = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        w.register.money_delta(estate, cash, 400.0);
        w.parties.cease(estate);
        w.claims.against(estate, secured, 100.0, Rank::Secured as u32);
        w.claims.against(estate, treasury, 388.0, Rank::Preferential as u32);
        w.period = 1;

        let says = w.journal.kinds.declare("estate.paid");
        ran(&mut w, &Ranked { says });

        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 100.0);
        assert_eq!(w.register.quantity(w.register.row(treasury, cash)), 300.0);
        // The state is paid what there is, and what there is ran out. Nothing was clamped and
        // nothing was topped up — the estate had 400 and 400 left it.
        assert_eq!(w.register.quantity(w.register.row(estate, cash)), 0.0);
    }

    #[test]
    fn a_live_party_is_not_an_estate_and_nothing_here_touches_it() {
        // An estate is what is left of a party whose life has ended. A claim against a party that
        // is still alive is not paid by this mechanism — it is the party's own to pay.
        use crate::mechanisms::estate::Rank;
        let (mut w, bank, secured, _t, cash) = world();
        let alive = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        w.register.money_delta(alive, cash, 400.0);
        w.claims.against(alive, secured, 100.0, Rank::Secured as u32);
        w.period = 1;
        let says = w.journal.kinds.declare("estate.paid");
        ran(&mut w, &Ranked { says });
        assert_eq!(w.register.quantity(w.register.row(alive, cash)), 400.0);
    }

    #[test]
    fn an_estate_with_nothing_pays_nobody_rather_than_paying_them_a_share_of_nothing() {
        // Appendix A and Law 6 together: there is nothing to divide, and an estate that paid out of
        // an empty account would be inventing the money it paid with.
        use crate::mechanisms::estate::Rank;
        let (mut w, bank, secured, treasury, cash) = world();
        let estate = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        w.parties.cease(estate);
        w.claims.against(estate, secured, 100.0, Rank::Secured as u32);
        w.period = 1;
        let says = w.journal.kinds.declare("estate.paid");
        ran(&mut w, &Ranked { says });
        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 0.0);
        assert_eq!(w.register.quantity(w.register.row(treasury, cash)), 0.0);
    }

    #[test]
    fn a_claim_an_estate_has_paid_does_not_come_back_next_period() {
        // A claim paid must be marked paid: reading `outstanding` without marking pays every claim
        // in full again every period.
        use crate::mechanisms::estate::Rank;
        let (mut w, bank, secured, _t, cash) = world();
        let estate = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        w.register.money_delta(estate, cash, 400.0);
        w.parties.cease(estate);
        w.claims.against(estate, secured, 100.0, Rank::Secured as u32);
        let says = w.journal.kinds.declare("estate.paid");

        w.period = 1;
        ran(&mut w, &Ranked { says });
        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 100.0);

        // The estate still has 300 and the claimant has been paid. Next period it is owed NOTHING.
        w.period = 2;
        ran(&mut w, &Ranked { says });
        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 100.0, "it was paid once");
        assert_eq!(w.register.quantity(w.register.row(estate, cash)), 300.0);
    }

    #[test]
    fn a_claim_paid_in_part_comes_back_for_the_rest_and_no_more() {
        // What a claimant did not get is a LOSS on a named holder, and it stands until the estate
        // has something to pay it with. The estate has 40 against a claim of 100.
        use crate::mechanisms::estate::Rank;
        let (mut w, bank, secured, _t, cash) = world();
        let estate = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        w.register.money_delta(estate, cash, 40.0);
        w.parties.cease(estate);
        let c = w.claims.against(estate, secured, 100.0, Rank::Secured as u32);
        let says = w.journal.kinds.declare("estate.paid");

        w.period = 1;
        ran(&mut w, &Ranked { says });
        assert_eq!(w.claims.outstanding(c), 60.0);

        // The estate realises something more, and the rest of the claim is paid — and only the
        // rest.
        w.register.money_delta(estate, cash, 100.0);
        w.period = 2;
        ran(&mut w, &Ranked { says });
        assert_eq!(w.claims.outstanding(c), 0.0);
        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 100.0);
        assert_eq!(w.register.quantity(w.register.row(estate, cash)), 40.0);
    }
}
