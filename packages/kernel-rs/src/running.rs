//! WHAT EACH SYSTEM DOES IN A PERIOD.
//!
//! @spec ARCHITECTURE 4.9b · Law 4, Law 5, Law 10, Law 15, Law 19 · Appendix B
//!
//! `systems.rs` is the list; this is what the rows on it DO when their phase runs. It sits beside the
//! assembly rather than inside `mechanisms/` for one reason: it names every module, and a module may
//! never import another module. The adapter layer is the assembly's, not any module's.
//!
//! Every mechanism here reads the world through the second door and proposes. None of them
//! invents a number at the boundary: where the world holds nothing for a system yet, it proposes
//! nothing, which is an answer and not a gap — and it becomes live the moment the world holds
//! something. That is what makes this wiring rather than a fixture.
//!
//! The shape is always the same: the READ pass first, over the kernel's own stores, then the
//! proposals. A module reads and proposes; it cannot do both at once, which is the borrow checker
//! saying what Law 4 already says.

use crate::calendar::Day;
use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::{equity, Class};
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{about, afoot, agreed, standing, Owing};

// THE FIVE KIND COLUMNS THAT LIVED HERE ARE IN THE KERNEL NOW. `agreed`, `standing`,
// `afoot` and `about` are in `stores.rs`, beside the stores whose kind columns they name; `tracks`
// is in `registry.rs`, beside the indices. This file called them *registry data* in its own comment
// and held them anyway — so `module.rs`, the kernel's own door, reached into `crate::running::afoot`
// to ask what a process was, and the kernel depended on the layer that depends on it.

/// WHAT FALLS DUE IS PAID, OR IT IS AN ARREAR.
///
/// The one mechanism the whole credit side rests on, and the reason `Schedules` exists: a claim with
/// terms and no schedule is a claim nobody can fall behind on, which is how the old world's
/// maturities all arrived at once. Every payment falling in this period is proposed against whoever
/// holds the line — and the wire refuses the ones the payer cannot fund, which leaves the arrear
/// standing rather than clearing it (A-20).
///
/// It serves loans, bonds, premiums and rents alike, because what they have in common is a schedule.
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
            // holds. This took `of_instrument(line)`, found the first row that was not the
            // issuer, and paid it the whole amount — so a line held by twenty parties paid all of
            // it to whichever row the index returned first, and what the other nineteen were owed
            // and did not get was a residual with no holder. The comment above it already said
            // *"whoever HOLDS the line is owed, which the register says"*; a stale comment is a
            // defect and this one was describing the fix rather than the code.
            //
            // The issuer's own rows are not outstanding. A company does not pay itself a
            // coupon, and netting them off here is what "issued and outstanding" means —
            // the same netting `instruments::equity` does on the liability side.
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
            // A payment on a line is per unit of par, and each holder is paid for
            // the units it holds. The parts sum to the whole by construction because the
            // denominator is the sum of the very rows being paid (Law 19: no second tally).
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
            // One obligation, one instruction. Every holder's leg stands or falls
            // with the rest, because an issuer short of its coupon fails the coupon and not
            // nineteen twentieths of it — and `settles(due)` marks ONE schedule row, so a pass
            // that paid some holders and not others would make that mark a lie either way.
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
///
/// Nothing in this world could bring an obligation into existence, so the sovereign funding
/// constraint — sequencing step 3 — bound on nothing: the treasury auctioned a line the assembly had
/// made for it, in a size that was a literal. Now a party that is short reads what it is short of and
/// ISSUES a bill for it, which the market then takes or does not (D5: an auction can fail).
///
/// It never asks what a party is. Which kinds fund a shortfall this way is a PROFILE the
/// registry holds (`KindProfile::issues_paper`) — a treasury auctions, a firm brings a bond, a
/// household cannot — and this walks the parties whose profile says so.
pub struct Funding {
    /// WHOSE paper this is, and over what horizon.
    ///
    /// One `Funding` on the treasury row was bringing every kind's paper — a firm's bond issued by
    /// the system that funds the state, while `corporate_credit` and `short_term_debt` counted. Two
    /// writers of one fact, and the wrong one.
    ///
    /// The kinds are registry data handed in, never a branch; the HORIZON is what makes
    /// them different instruments rather than the same one twice. A party short against what falls
    /// due this week brings commercial paper; a party short against what falls due this year brings
    /// a bond. The tenor is a decision about a NEED and the need is the borrower's.
    pub of_kinds: &'static [u32],
    /// How far ahead this system's shortfall is read, in days — and from how far ahead. A firm
    /// short over the year is short over the week too, so the windows do not overlap: two systems
    /// reading the same due date would bring two instruments for one shortfall.
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
            // The profile answers, and a kind with none is a kind nobody has said this of —
            // which is missing rather than a no.
            let kind = ctx.parties().kind_of(who);
            if !self.of_kinds.contains(&kind) {
                continue;
            }
            // And the profile still answers whether a kind issues paper at all — a kind with
            // none is a kind nobody has said this of, which is missing rather than a no.
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
            // Outlays against what it has, plus what it needs to get back to its own buffer.
            // A party short of nothing brings nothing — not a floor under the size, the absence of a
            // reason to issue at all.
            let short = crate::mechanisms::treasury::must_raise(owes, 0.0, cash, buffer);
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // 5 C3.a: it matures on a DATE, so the maturity wall is spread by the dates and not by a
            // count of periods.
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
                // A treasury auction is a CALL — a sealed cross at one level, which is what
                // an auction IS. The `seen_by` is not read by a call and says so with one,
                // and nothing rests in a sealed cross, so it declares no life for an order.
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
///
/// The benchmarks row counted how many lines printed — an honest count, and not the read a benchmark
/// is for. A benchmark FIXES: it reads what the overnight book actually cleared at and
/// publishes that, and where the book ran and nothing crossed it publishes nothing, because a
/// carried price is not a rate anybody transacted at this period.
///
/// *no posted benchmark.* Publishing a carried number would be exactly that — the
/// corridor as decoration, with the money market's own price unused — so the absence is the mechanism
/// rather than a gap in it.
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
        // Only a CLEARED print is a fixing. A carried or seeded one is refused here, which is
        // the whole of what "a transacted rate" means.
        let Some(fixing) = crate::mechanisms::benchmarks::fix(&print) else { return };
        ctx.say(
            self.says,
            &[],
            &[(0, Value::Num(fixing.rate)), (1, Value::Num(f64::from(fixing.period)))],
            true,
        );
    }
}


/// WHAT A FIRM MAKES, AND THE PLANT IT MAKES IT WITH. Registry data, one row per
/// good this world knows how to produce.
///
/// Who makes it is not declared here, and that is the point. A maker is whoever holds the plant —
/// so production follows the capital rather than the party kind, entry is a firm buying plant, and
/// exit is a firm selling it. A mechanism that asked a party what kind it was would be Law 15's
/// defect and would also be wrong: a bank that bought a mill makes flour.
#[derive(Clone)]
pub struct Makes {
    pub line: crate::mechanisms::recipe::Line,
    /// What THIS line's plant is. Capital is specific in kind (33 A4).
    pub plant: InstrumentId,
    pub plant_is: crate::mechanisms::capital_programme::Plant,
}

/// THE FIRM PRODUCES.
///
/// The one thing in this world that makes anything, and the read pass is the firm's reasons: what it
/// expects to sell (its own outlook, §46), the capacity of the plant it holds (33 A2), the inputs on
/// its own register rows, and the hours its engagements give it. It picks the way of
/// making the good that costs IT least at the prices IT can see, runs the line in whole batches
/// , and what comes out is the OUTCOME of those reasons and never a quantity anybody chose.
///
/// The proposal is one instruction. The inputs are destroyed as consumed and the output is
/// created carrying what it cost — inputs at their own lot basis (E5, first in first out, the same
/// order settlement will draw them in), plus the wages the hours cost, plus the period's depreciation
/// on the plant that ran (33 A3). Both halves stand or fall together, because a world where the
/// inputs went and the output did not arrive is a world that ate them.
/// What one batch run came to, between the read pass and the proposal. It is named rather than a
/// tuple because five positional numbers about a production run is a thing a reader has to decode.
struct Ran {
    maker: PartyId,
    makes: InstrumentId,
    draws: Vec<(InstrumentId, f64)>,
    finished: f64,
    /// The period it comes off the line. The inputs go now; the output arrives then.
    ready: u32,
    /// What went in — inputs at their own basis, wages, the capital charge. The batch carries it
    /// and the unit cost is struck from it when the batch comes off, so what a unit cost is what went
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
    /// 37 B1, 22c.3: how much COVER a firm wants on its shelf, as a multiple of what it expects
    /// to sell. A PREFERENCE read through `params` — the desired buffer used to be exactly zero and
    /// nobody had chosen it.
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
        // stored aggregate. A line that builds a STRUCTURE draws more where more already
        // stands, and which lines those are is registry data, so an office block, a dwelling and a
        // works go through this one mechanism.
        let built = crate::places::built_up(ctx.parties(), ctx.register(), ctx.registry());
        let crowds_at = ctx.params().square_km(self.crowds_at);

        for m in &self.makes {
            for &plant_row in ctx.register().of_instrument(m.plant) {
                let plant_row = HoldingId(plant_row);
                let maker = ctx.register().holder_of(plant_row);
                if !ctx.parties().alive(maker) {
                    continue;
                }

                // 33 A6: a vintage IS a lot on the register, so capacity and the period's charge are
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

                // Its own outlook, and NOT a model forecast. A firm with no view of what it
                // sells has no reason to start a line, and that is missing rather than nothing.
                let Some(expects) = ctx.outlooks().of(maker, about::HOW_MUCH_IT_SELLS) else {
                    continue;
                };

                // The hours its engagements give it, and what an hour of them costs. The terms
                // are `[wage, hours]` — the convention `agreed::ENGAGEMENT` states.
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
                        // A wage and an hour are PER PERSON, so the line gets
                        // the headcount's worth of both. `Wages` was told this and `Making` was
                        // not, which left one term with two readings — Law 4 — and a firm employing
                        // two thousand people getting one person's hours.
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
                // own depreciation, over what the plant can make. Both are owed whether the line runs
                // or not, which is exactly why they land in the unit cost of what it does make.
                let keeping: f64 = stock.iter().map(|v| upkeep(v, &m.plant_is, now) + charge(v, &m.plant_is, now)).sum();
                let a_service = keeping / can_make;

                // It picks the way that costs IT least — and what an input costs IT is
                // what it paid for the stock it holds, off its own lots, because that is the stock
                // the batch will actually consume and the number `unit_cost` will book. Where it
                // holds none of an input it would have to buy it, and what it would pay is the
                // market's print.
                //
                // This is one rule, stated once, over two real situations — not two formulas for one
                // fact. Costing everything at the print was the earlier reading and it was wrong in a
                // way that mattered: a firm with a full yard could not cost the line it was standing
                // in, because the market for its input had not happened to clear.
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
                // The same line, run where this much already stands. Crowding scales every
                // way of making the line alike, so it cannot change which way is cheapest — that is
                // why it is applied AFTER the pick rather than to each way before it.
                //
                // A line that stands nowhere is built alike everywhere, and that is an ANSWER rather
                // than a default: flour is milled the same in an empty valley and in a city.
                let crowding = match ctx.registry().footprint_of(m.line.makes) {
                    Some(_) => crate::places::crowding(
                        crate::places::standing_in(&built, ctx.parties().region_of(maker)),
                        crowds_at,
                    ),
                    None => 1.0,
                };
                // ONE writer of the scaling. What the firm can afford to start, what leaves
                // its rows and what the batch cost all come off this one object, so the decision and
                // the draw cannot disagree about where the line is standing.
                let way = &way.where_it_stands(crowding);

                // What it holds of each input, off its own rows. An input it has no row for is
                // one it has none of, and `decide` is where that stops the line.
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

                // What the draw costs, at the lots' own basis and in the order settlement
                // will draw them in — so what this books and what leaves cannot disagree.
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
                // No units, no capitalised cost. A run that finishes nothing capitalises
                // nothing, and the cost it incurred is a period expense rather than a batch — which
                // is the caller's, and this will not invent a unit to hang it on. The read is asked
                // here, where the decision is, and the answer is thrown at the batch below (Law 4:
                // one cost in one place — `unit_cost` over what the batch carried).
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
        // period and carrying what they cost then. This runs BEFORE the starts below, because a line
        // that took a period delivers what it began before it begins anything else.
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
        // became on the line, owned, carrying what it cost, until it is ready. The two were one
        // instruction until the recipe had a lead time, which is why nothing was ever in progress.
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
///
/// 21b, measured on the old engine: a manager died, the succession rule ended every
/// commitment it ran, and its money fund was left ALIVE — holding a book, with households holding its
/// shares, and nobody deciding for it. It stayed that way for the rest of the run and the audit said
/// so every period. Which pools survived a manager's death was decided by which side of the row the
/// dead party was on, and by nothing about the pools.
///
/// There is no clause for it: F3 is about the FEE. So this is the mechanism that absence asks for,
/// and it invents nothing — the holders' claim is redeemable, so what the pool holds is sold in
/// the books it bought it in and the proceeds pay redemptions pro rata, period by period, until
/// nothing is left. No forced buyer: a book that will not take it leaves it unsold
/// and the wind-up takes another period. The selling is the pool's own, posted by the participant
/// side; what this does is pay out what the selling raised, and end the pool when there is nothing
/// left to pay with.
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
            // Whether anybody decides for it is a read of the RELATIONS, never a flag on the
            // pool that somebody has to remember to clear.
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
            // issuer — the walk 21.130 names, run here only for a pool that has actually lost its
            // manager, which is rare. It becomes a read the day `Instruments` is indexed by issuer.
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
                // Nothing held and nobody owed is a pool that has ended. Law 6: nothing here
                // ends it on a schedule — the wind-up takes as long as the selling takes.
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
///
/// 21c, measured on the old engine: an estate paid the treasury 388 pieces in period 5 and
/// 388 again in period 6, with a `tax` receipt, and the audit said *"paid 388 to treasury.us, who
/// has no claim on it"*. The estate was right to OWE it and the treasury was wrong to TAKE it.
///
/// What this does is the payout, and only the payout: it reads the claims standing against every
/// dead party, asks the waterfall what each gets out of what the estate actually has, and proposes
/// those payments. Nothing here decides a claim — a claimant gets in by being written one
/// through `ctx.claims()`, which is the door an assessment goes through, and the rank it stands at
/// is the law's.
///
/// A rank is not paid "up to" anything. It is paid what there is, and what there is runs out.
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
            // An estate is what is left of a party whose life has ended. Nothing here asks
            // what KIND of party it was — a dead bank and a dead baker pay the same way.
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
            // period and is paid again, which is how a dead firm's debt stood twice on the register.
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

/// The rank a stored claim stands at. `Claims` holds a number and does not know what it means
/// ; this is where the number becomes the law's ordering, in one place.
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
///
/// Benchmarks, ratings, the observer surface, the second opinion: they publish a read over what
/// already happened and propose nothing. That is not a stub — giving them a schedule would be
/// inventing demand nobody has — and the count it publishes is what makes it visible
/// that it ran.
///
/// This is exactly the shape the census counts — an honest count of something
/// real, no decision and no write. It used to say so by overriding `Mechanism::only_counts`, and
/// nothing else in the world overrode it, so the register read zero while four systems ran one of
/// these. It is read off `Taken::decided` now and nothing declares it.
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
///
/// 5 A4: every asset is somebody's liability, party by party. A world that could not say who owes
/// the money in it is a world with free money in it somewhere.
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
///
/// Nothing in this world published accounts, on any calendar. `reporting` said a firm's own equity
/// to its own subjects, which is a private result — and §48's PUBLISHED accounts, the thing a bid
/// values a company off (§35 `worthAt`) and a covenant is written against, were nobody's. That is
/// the deeper cause 21.76 names under three other findings.
///
/// Being public is a state read from the register, never a label: a company reports while
/// its shares are listed and held by OUTSIDERS, so a company whose outsiders sell stops reporting
/// without anybody relabelling it, and there is no kind of party that reports — this walks
/// every living party and asks the register.
///
/// The year is placed by DATE from the day the company started, which is why 22i.1 had
/// to give a party a birth period first. Year-ends are staggered because companies start on
/// different days, not because anything staggers them.
///
/// The report comes out after the books close, and in between the company knows its result
/// and nobody else does — the only real information asymmetry this world has.
///
/// Income is the equity account's MOVEMENT, read against what this company last published
/// — never a figure anybody chose. A first report has no prior close, so it publishes no income at
/// all: missing is missing, and a first annual report with no comparative is what that looks like.
pub struct Publishes {
    /// The event kind the accounts are published under.
    pub kind: u32,
    /// Key rows for the figures, so a reader takes them by name rather than by position.
    pub at_equity: u32,
    pub at_income: u32,
    pub at_shares: u32,
    /// Which fiscal close a report is FOR. A figure with no period is one nobody can
    /// restate against or compare with the next.
    pub at_closed: u32,
    pub days_per_period: i64,
    /// How many days after the books close the report comes out. A TECHNOLOGY.
    pub asymmetry: &'static str,
    /// The weight a bank puts on what it already thought against what it has just
    /// seen. ONE PREFERENCE, and it is what makes two banks' estimates of one name differ.
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
            // THE FISCAL PERIOD IS A QUARTER, placed by DATE from the day this
            // company started — three months of calendar, which is a whole number of periods only by
            // accident. It is walked by advancing the month, never by adding days: a quarter is not
            // ninety-one days and a year is not four of them.
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
            // Income is the MOVEMENT against what it last published. A first report has no
            // prior close and so publishes no income — missing is missing.
            let income = last.get(&row).map(|&(_, was)| now - was);
            out.push((row, now, income, listed, fiscal.closes.0));
        }
        // And the banks that cover a name estimate what it will report. Coverage is
        // uneven and how many cover a name is an OUTCOME: a bank estimates the names it can
        // SEE, which is the ones whose paper it holds, so a company nobody lends to is covered by
        // nobody. §46 A2: it is formed from what THIS bank has observed, weighted by its own memory,
        // so two banks with different histories of a name estimate differently.
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
                    // What it has seen is what the company has PUBLISHED, and it may not
                    // be handed the answer — so what it holds now is weighted against what this
                    // report is about to say, and never set to it.
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
            // It HOLDS it, so it can be shown to have been wrong — and F1's surprise is the
            // report against what was standing when it arrived.
            ctx.now_stands(standing::ESTIMATE, bank, company, vec![figure]);
        }

        for (who, worth, income, shares, closed) in out {
            let mut data = vec![
                (self.at_equity, Value::Num(worth)),
                (self.at_shares, Value::Num(shares)),
                // WHICH fiscal close this is the report for. A figure with no period is a
                // figure nobody can restate or compare.
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
///
/// This world had never published a grade. `grade_from` and `reassess` were built, tested and
/// unreached; the `ratings` row counted how many parties were alive. So the investment-grade index
/// was empty by construction, every claim took the worst weight wherever a grade was read, and A4's
/// two houses could not disagree about anything because neither said anything.
///
/// The state is READ, and A2.a's forbidden input cannot be supplied: there is no price here and
/// no spread. Leverage is what an issuer owes against what it holds; coverage is what it last
/// PUBLISHED against what falls due on it (which is why 22i.1 came first); age is how long it has
/// been going (22i.1 again); the trend is this year's published income against last year's.
///
/// A house holds its grade (`standing::GRADE`, `about` the issuer), so two houses hold two rows
/// on one name. A move is a `restates`, so what a house said before stays readable beside what it
/// says now — which is what lets a grade be shown to have been wrong.
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
        // What each name last published, and what it published before that — the trend. One
        // pass over the accounts rather than a walk per name per house.
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
                // And the TREND — this year's published income against last year's. A name with
                // one report has no trend, and no trend is not a trend of zero.
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
            // It is STICKY. A house that already says this about this name says nothing;
            // a grade republished every period is not a rating action and would make A6's record of
            // what a house got wrong unreadable.
            let held = ctx
                .standing()
                .of_party_about(by, of, crate::stores::standing::GRADE)
                .map(|s| ctx.standing().terms(s)[0]);
            if matches!(held, Some(rank) if rank == grade.rank()) {
                continue;
            }
            // The probability of failing and, SEPARATELY, the loss given it. Both are the
            // house's own view and both are stood behind with the grade.
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

/// A DOWNGRADE PAST A MANDATE'S BOUNDARY IS A FORCED SALE BY EVERY
/// HOLDER BOUND BY IT, ON THE SAME DATE.
///
/// The `forced_sale` row was a CLOSER for a process nothing opened, so XI-2 — a sequencing step —
/// had never happened in this world. What it was waiting on was a grade, and 22i.2 published one.
///
/// This is the most mechanical and most synchronised of XI-2's four doors, and the only one whose
/// inputs exist: a pool is run under a mandate with a floor (`agreed::MANDATE`'s terms), an
/// assessor holds a grade on each issuer (`standing::GRADE`), and when the grade falls through the
/// floor every bound holder must sell what it holds of that issuer's lines — whatever it is worth.
///
/// It opens the workout and does not sell: a forced seller posts a size and NO level (Clearing
/// C3), which is an order in a book and therefore a participant's act, not a mechanism's. What is
/// opened here is the requirement; `ForcedSeller` is what stands in the market with it.
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
        // What each issuer is graded at now. The WORST grade any house holds on it, because
        // a mandate that let a holder pick the kindest house would not bind on anything.
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
            // It is already in one, and a second workout for the same breach would be the
            // same requirement counted twice.
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
///
/// A forced sale with a reservation price is a sale that can decline, and then the channel XI-2 is
/// about is closed: the sale must be struck at whatever the other side posted, which is what makes
/// it move the price and what makes the move reach holders that did nothing.
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
        // It cannot sell more than it holds, which is arithmetic about a holding and not a
        // cap on a number.
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
///
/// The `loss` row counted how many parties were alive. So nothing in this world ever crossed a
/// threshold, nothing was ever non-performing, and XI-1 — a sequencing step — had never happened:
/// there was no borrower, nothing to distribute, nothing to seize and nothing to disagree with.
///
/// The test is applied to ONE borrower, never to a band's average. A cell is one
/// borrower here: it stands for a population whose members share a key, and applying the test to a
/// mean would mean a mean-preserving spread caused no defaults at all — exactly backwards, because
/// widening dispersion at constant mean is what a downturn does.
///
/// The crossing is an EVENT WITH A DATE, which is what XI-1 calls it, so the journal is where it
/// lives: what a claim's standing is, is the last crossing said about it. There is no second store
/// holding a status beside the events that changed it.
///
/// Nothing is drawn and there is no probability. What is compared is what the borrower HAS
/// against what FELL DUE: it either could pay or it could not.
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

        // What fell due on each borrower, per claim. A claim with nothing due this period is
        // one nobody could have missed a payment on.
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
            // A charge that is VISIBLE, never a reserve absorbing things quietly. It names
            // the borrower and the claim, so a holder can find its own.
            ctx.say(self.kind, &[borrower, claim], &[(self.at_standing, Value::Num(rank))], true);
        }
    }
}

/// A SELLER THAT HAS DELIVERED AND NOT BEEN PAID OFFERS TERMS.
///
/// `trade_credit` counted live agreements and wrote none, so no invoice existed in this world at
/// all — the tier §42 A4 calls *the tier that lives on it* used nothing, because there was nothing
/// to use.
///
/// The trigger is 22d's queue. A payment that is short is sitting in it with a day it is late
/// on; the seller can wait, and a seller that waits has made a loan. That is what trade credit IS
/// (C1: the goods move at one time and the money at another), and it is a DECISION the seller takes
/// per buyer on that buyer's condition — a refusal is an outcome, and the buyer's payment then
/// runs out of days as an arrear like any other.
///
/// The seller's limit is its own: how much it will have out to one name at once, and how
/// long it will wait. Terms that are a formula cannot tighten, which is what D4 is about — the
/// anticipation of failure making suppliers withdraw terms and starving a firm of working capital
/// faster than any lender could.
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
        // holder of the line — and a payment cannot half-wait, so the count is what says
        // whether ALL of them agreed to wait.
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
            // `None` is a REFUSAL, and a refusal is a decision. The buyer's payment then
            // runs out of days as an arrear, which is what a seller that will not wait means.
            let Some(terms) =
                crate::mechanisms::trade_credit::offer(seller, buyer, amount, today, &view, already)
            else {
                continue;
            };
            *out.entry((seller.0, buyer.0)).or_insert(0.0) += amount;
            struck.push((q, seller, buyer, terms.amount, terms.due));
        }

        // Each seller decides for itself, and the PAYMENT is one.
        //
        // A payment owing several sellers cannot half-wait — it is one instruction with one day —
        // so the terms each seller struck are its own relation, and the payment's day moves ONCE:
        // to the earliest day any of them agreed, and only where every payee agreed. A seller that
        // refused still wants paying on the day it was owed and is not made to wait because the
        // others were willing (B5: a refusal is a decision).
        //
        // It used to move the day once PER SELLER, which nothing noticed while every queued payment
        // had exactly one payee. 0k.5 gave a coupon one leg per holder and the second call stopped
        // the world in period 2.
        let mut waiting: std::collections::HashMap<u32, (usize, Day)> = std::collections::HashMap::new();
        for (q, seller, buyer, amount, due) in struck {
            // The terms are the relation — what is owed and when. The DUE DATE is what makes the
            // goods and the money two different moments (E1: a sale that settles instantly by
            // construction deletes all of this). The money owed stays in the queue: the invoice
            // and the payment would otherwise be one debt in two places, so what the terms
            // change is WHEN, and the seller's waiting is the payment's day moving out.
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
            // Terms that end sooner than the payment's own day are not time given, so nothing
            // moves and the payment keeps the day it had.
            if until.0 > ctx.wire().queue.late_after(q).0 {
                ctx.waits_for(q, until);
            }
        }
    }
}

/// A COMPANY FLOATS — and no company in this world had ever had shares.
///
/// The `equity` row was a CLOSER for a flotation nothing opened. So §10's line did not exist for any
/// firm, which is why nothing could be valued (§35 `worthAt` reads a share price), nothing could be
/// taken over (§29 B), and §48's accounts were published by nobody: being public is *shares listed
/// and held by outsiders*, and there were no shares.
///
/// A company floats when the credit market has not taken it. That is a read and not a rule: it
/// brought paper and is still holding it, so nobody bought the paper — equity is what a
/// borrower the lenders will not take has left. A firm whose paper sold has no reason to sell its
/// ownership and does not.
///
/// A share count changes only by a NAMED EVENT, and this is one — the line comes into
/// existence with the shares it issues, through the same door every other instrument does.
pub struct Floating {
    pub kind: u32,
    /// What a bank says when it is below its capital requirement. A recapitalisation
    /// IS an equity issue, so it comes through this door and not a second one.
    pub short_of_capital: u32,
    /// How many shares a line comes into existence with. A TECHNOLOGY of the market: the
    /// count is a convention and what a share is WORTH is what the book crosses at.
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
            // The profile answers whether this kind brings paper at all. A kind that does
            // not is a kind with no credit market to be turned down by.
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
            // Two reasons to sell ownership, and a company already listed has neither. The
            // lenders would not take it (unsold paper), or it is a bank below its requirement and
            // must raise.
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
                // Shares trade on an EXCHANGE — orders rest and are matched as
                // they arrive, priced at the level the resting side was standing at.
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
///
/// A company selling its own new shares offers them and the BOOK says what they are worth. That is
/// what a flotation is — investors bid, the price clears, and the company finds out what its
/// ownership fetches.
///
/// It posted a book-value reservation and that was invented. Book equity is what a company's
/// holdings cost, at cost; it is not what the company is worth and nothing in §10 says a floating
/// company will not sell below it. A reservation nobody derived is a floor under a price
/// wearing a seller's clothes, and it made the flotation clear at an accounting figure rather than
/// at what anybody would pay.
///
/// A flotation that finds no bid does not clear, which is a real outcome and is why `closes` exists.
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
        //
        // A company floating its shares posts an ask at NO LEVEL — the book says what its ownership
        // is worth and the company does not — and posting one at its book value was an invented
        // floor under a price that nothing derived. Both of those are settled.
        //
        // What is not settled is that a level-less ask in a `Book` venue reaches `prices.rs` as
        // MINUS INFINITY: `protocols::level_of` uses ±∞ to mean *takes what the book gives* and
        // nothing stops that sentinel becoming the print. It stops the world in period 1, and it is
        // Until it is fixed the line is brought and nobody is asked to bid for it — which is
        // a flotation that has not been offered, and says so, rather than one cleared at an
        // accounting figure.
        Vec::new()
    }
}

/// §31 A1, B1, B3, C1, C1.a, 40 C5, 22i.8: A BANK READS ITS OWN CAPITAL AND ACTS ON IT.
///
/// The `bank_capital` row counted how many parties were alive. So no bank in this world had a
/// capital position at all: §31's ladder was never read, the two failures C1.a insists are different
/// were never told apart, and 21.40's lending standard — *the standard a lender is currently lending
/// at* — had no lender computing one, because no lender decided anything.
///
/// Capital is the RESIDUAL: what it holds against what it owes, read, never a stored figure
/// something is withdrawn from. The weights come from the GRADES: a claim on a party
/// that cannot fail weighs nothing and everything else weighs what its own credit says it weighs —
/// which is why this had to wait for a house to publish a grade.
///
/// And a bank below its requirement has to RAISE, which it says and `Floating` acts on: a
/// recapitalisation IS an equity issue, and one company's shares have one writer. C2 also
/// says it can FAIL — nobody has to buy — which is why this publishes a need rather than an outcome.
pub struct BankCapital {
    pub kind: u32,
    /// What it says when it is below its requirement and has to raise. `Floating` reads it.
    pub short_by: u32,
    pub at_ratio: u32,
    /// The requirement, the backstop and the buffer. POLICY — the regulation's, and the one
    /// place they are stated.
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

        // What each name is graded at — the WORST any house holds on it, because a bank
        // that could pick the kindest house would weigh its book by choosing its assessor.
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
                // The layer between equity and senior paper. Nothing in this world issues one
                // yet, so it is none — which is a fact about the world and not a number chosen here.
                subordinated: 0.0,
                due_now,
                money_at_hand,
            };
            if position.carried() <= 0.0 {
                continue;
            }
            let how = standing(&position, rules);
            let ratio = position.capital() / position.carried();
            // 40 C5: what it is lending at now. A lender with no room asks for more of the price
            // up front, and both terms move together because both are read from the same position.
            let headroom = position.capital() / (rules.min_leverage + rules.buffer) - position.carried();
            acted.push((who, ratio, how.below_requirement, headroom));
        }

        for (who, ratio, below, headroom) in acted {
            // The standing is PUBLIC. A capital position nobody could read is one no depositor,
            // no lender and no assessor could act on.
            ctx.say(self.kind, &[who.0], &[(self.at_ratio, Value::Num(ratio))], true);
            // 40 C5, C5.a: the standard it is lending at, which is a READ of what it already
            // measures — the strain on its own book, its hurdle and its headroom — and never a
            // constant. A lender already stretched, or with little room to put more on, asks for
            // more of the price up front and lends a smaller multiple of income; both move together
            // because both come from the same position. A constant here means only the rate
            // channel loops, and it is C5.b's loop that is the housing cycle.
            //
            // The strain is its leverage: what it carries against what it has. It has that whether
            // or not it has written a mortgage, which is why this does not wait for a book.
            if ratio > 0.0 && headroom > 0.0 {
                let standard = crate::mechanisms::housing::standard(1.0 / ratio, headroom, hurdle);
                ctx.now_stands(
                    standing::LENDING_STANDARD,
                    who,
                    PartyId::NONE,
                    vec![standard.income_multiple, standard.deposit_share],
                );
            }
            // And a bank below its requirement must RAISE. What it says here is what it is
            // short of, and `Floating` is the one writer of a company's shares — a
            // recapitalisation IS an equity issue, and a second mechanism bringing a share line
            // would be two answers to *where did this company's shares come from*.
            if below {
                ctx.say(self.short_by, &[who.0], &[(self.at_ratio, Value::Num(-headroom))], true);
            }
        }
    }
}

/// WHAT A COMPANY'S CAPITAL COSTS IT, AT THE MARGIN, NOW.
///
/// The `cost_of_capital` row counted how many lines printed. So XI-4 — a sequencing step — had never
/// been taken: no company in this world knew what its money cost, and with no cost of capital there
/// is no hurdle, and with no hurdle no financial price can reach a real decision (`worth_doing`).
///
/// At the MARGIN and NOW — weighted by what it would raise, at what the markets say today, never
/// the average coupon on debt already outstanding, which is a price struck in the past and cannot
/// transmit anything that has happened since.
///
/// Both halves are DERIVED FROM CLEARED PRICES, in the direction Law 3 requires. The cost of
/// debt is the yield its own paper last crossed at; the cost of equity is the earnings yield on its
/// own share price — what it last published over what the market last paid for a share
/// . Neither existed before this item, which is why this row could only count.
///
/// `Missing` is missing: a company whose paper has never printed has no cost of debt, and a
/// company with no published income has no cost of equity. It stands behind nothing rather than
/// standing behind a number nobody struck.
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
                    // 5: the yield derives FROM the price, which is the direction Law 3 requires
                    // — what the paper crossed at against what it repays.
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
                    // And the cost of equity is the EARNINGS YIELD — what it published over
                    // what a share last cost. A price is never turned into a return the other way.
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
///
/// The `bank_funding` row counted the credit outstanding. So no bank in this world set a deposit
/// rate, no depositor had anything to respond to, and B1's *a real payment to a real holder* had no
/// rate to be of.
///
/// The mix is its own and the blend is a READ across it, which is what lets a funding
/// condition reach a borrower at all. What it will pay is bounded above by the cheaper of its own
/// wholesale cost and what a money fund yields — and that is not a bound anybody imposed:
/// past that point the bank would rather fund wholesale, which is a decision.
pub struct BankFunding {
    pub kind: u32,
    /// The benchmark fixing, which is what a money fund would earn. A CLEARED print or
    /// nothing (`Fixes` refuses a carried one), so a bank with no fixing to read sets no rate.
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
                    // Short, and it ROLLS — which is where a funding squeeze bites. Its rate is
                    // the coupon it promised, which is a TERM and not a price (5 C4.b).
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
            // A POSTED rate — depositors respond to it, so it is one-sided terms the bank
            // stands behind until it changes them, and what it was paying stays readable beside it.
            ctx.now_stands(standing::DEPOSIT_RATE, who, PartyId::NONE, vec![rate]);
            ctx.say(self.kind, &[who.0], &[(0, Value::Num(rate))], true);
        }
    }
}

/// A FIRM DECIDES TO INVEST, AND THE COMPARISON IS THE MECHANISM.
///
/// The `capital_programme` row was a CLOSER for a programme nothing opened, so no firm in this world
/// had ever decided to build anything — and §33's whole content is that decision.
///
/// It invests when it expects the return to exceed its cost of capital (`worth_doing`). A rate
/// applied to revenue, however many multipliers are attached, is this joint deleted, and then no
/// financial price can reach a real decision. The cost of capital is the one 22i.9 publishes, off
/// its own cleared prices; the return is its OWN outlook and never a model forecast.
///
/// What it commits is money, and the plant arrives by PURCHASE. A programme that conjured plant
/// would be a firm building with nothing — it bids for capital lines in their books like any other
/// buyer, and what it pays is what those books cross at.
///
/// And building where it is already built-up costs more. The congestion is a read over the
/// register, so a firm in a crowded place commits more money for the same plant, which is what makes
/// location decide anything at all.
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
        // How built-up each place is. One read over the register for the whole world, not one
        // per firm.
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
            // Its own outlook, and a firm with none has nothing to expect. Missing is
            // missing: a firm that has formed no view does not invest on a view somebody else has.
            let Some(sells) = ctx.outlooks().of(firm, crate::stores::about::HOW_MUCH_IT_SELLS) else {
                continue;
            };
            let Some(price) = ctx.outlooks().of(firm, crate::stores::about::WHAT_IT_SELLS_FOR) else {
                continue;
            };
            // What the ground it stands on does to a build. A firm in a crowded place commits
            // more money for the same plant.
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
///
/// A firm with a programme afoot bids for capital lines in their own books, up to what it committed.
/// Nothing is conjured: what the plant costs is what the book crosses at, and a programme whose bids
/// do not fill is a programme that did not build — which is a real outcome and is why `closes` exists.
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
        // It bids against what the book last PRINTED, because its limit is money and an
        // order is pieces. A line with no print has nothing to bid against.
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
///
/// The `spot_fx` row counted how many lines printed. So no pair in this world had ever had a rate,
/// and XI-12 — a sequencing step — had never been taken.
///
/// Every participant is here for a reason it HAS, never a side the mechanism
/// assigned: a party that owes a money it has not got must buy it, and a party holding a money it
/// has no use for will sell it. Both are reads of the register against the schedules, so the book is
/// made of obligations rather than of postings somebody invented.
///
/// One rate is in force for the period and both valuation and settlement use it, and where
/// nothing crossed there is NO rate — a pair nobody traded has none. Nothing is added to
/// make the book balance: the unfilled side stays unfilled, which is what C4's *imbalance moves it*
/// is about.
pub struct SpotFx {
    pub kind: u32,
    pub days_per_period: i64,
}

impl Mechanism for SpotFx {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::spot_fx::{clearing, Posted, Reason};
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let to = Day(from.0 + self.days_per_period - 1);

        // Who OWES a money, and who HAS one. Both from what the party is, not from a side
        // anybody gave it.
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

        // One book per currency being bought. The rate is that currency against the money the
        // other side is paying with, which is what "a pair" means.
        let mut pairs: std::collections::HashMap<u32, Vec<Posted>> = std::collections::HashMap::new();
        for (&(who, ccy), &amount) in &owes {
            // The worst rate it will take. A party that MUST have the money will pay what the
            // market asks — it has an obligation, not a view — so its reservation is what the pair
            // last printed, and where it has never printed there is nothing to post against.
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
            // A pair nobody traded has NO rate. Nothing is carried forward and nothing
            // is invented to give it one.
            let Some(rate) = cleared.rate else { continue };
            done.push((ccy, rate, cleared.trades.len(), cleared.unfilled));
        }

        for (ccy, rate, trades, unfilled) in done {
            // One rate in force for the period, published — both valuation and settlement use
            // it, so it is a fact about the world and not one party's read.
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
///
/// The `fx_forwards` row counted live agreements. So nothing in this world had ever hedged a
/// currency, and B3's cross-currency basis — a real price paid by whoever needs the money — had
/// nothing to be a deviation from.
///
/// It is cleared from what participants will do, not struck off a formula. The parity
/// rate is the CHECK and not the price: it says where the forward would sit if the
/// arbitrage were free, and the distance between the two is the basis, which is what somebody is
/// actually paying to get the money it needs.
///
/// A party that owes a money it has not got, at a date beyond this period, is who hedges. That
/// is the reason, and it is read off its own schedule.
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
        // The tenor is DAYS and the year fraction is read from the dates, never the
        // other way round — a forward "of a quarter" is ninety days and the calendar says what that
        // is as a year.
        let days = ctx.params().days(self.tenor) as i64;
        let from = Day(i64::from(ctx.period()) * self.days_per_period);
        let matures = Day(from.0 + days);
        let tenor = days as f64 / 365.0;

        // The rate the pair last cleared at. A pair with no rate has no forward, because
        // there is nothing for the forward to be a rate FORWARD of.
        let mut spot: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.spot) {
            if ctx.journal().period_of(row) == ctx.period() {
                if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                    spot = Some(rate);
                }
            }
        }
        let Some(spot) = spot else { return };
        // And what the two moneys fund at. The fixing is the only transacted rate this world
        // has, so both legs read it until each currency has its own.
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
            // A hedge needs a counterparty holding the other side. One party wanting one is not
            // a market, and nothing is invented to be the other side of it.
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
            // The basis is the deviation, and it is a real price paid by whoever needs the
            // money. Where the forward crosses at parity the basis is nothing, which is what a
            // market with free arbitrage looks like — and it is a MEASUREMENT here, not a target.
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
///
/// The `redeemable` row counted live agreements. So no pool in this world had a net asset value,
/// nobody could subscribe to one, and §13's whole shape — a liability denominated in shares whose
/// value is a READ of what the assets cleared at — did not exist.
///
/// NAV is a read, not a stored level, and it is marked at CLEARED prices: a price
/// nobody cleared is not a mark, so an asset the market has not touched is carried at what it cost
/// and says so. A pool with no shares has no per-share value at all — missing, not zero.
///
/// A subscription gives the pool cash and the holder shares AT NAV, and it is a RELATION:
/// the pool's liability is denominated in shares, and the agreement is where those shares are. C1.a
/// says the pool must then buy something with the cash, which is its mandate's business and not this
/// mechanism's — cash that sits is a mandate not being kept, and that shows up as a read.
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
            //
            // A fund that cannot value its book publishes no NAV (Appendix A: whoever asked
            // must handle an unpriced instrument). A line with a book that has never crossed is
            // unpriced — not zero, and not what it cost — so a NAV struck over it would be a number
            // that looks like the whole book and is not, and every subscription and redemption
            // priced off it would be struck at a fiction.
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
            // `None` where there are no shares. A pool with none has no per-share value, and
            // a first subscription therefore buys at what the pool is worth per share it is about to
            // create — which is the assets themselves where it holds any, and nothing where it does
            // not. A pool with neither has nothing to sell.
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
                // It subscribes with cash it has. `None` where it has none — a subscription
                // by a holder with no money is a share issued against nothing.
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
            // Cash one way and shares the other, in the same pass. The shares are the
            // RELATION — §13 A2's liability denominated in shares is exactly this row.
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

/// A BROKER LENDS TO A NAMED CLIENT, AND SETS WHAT IT
/// REQUIRES.
///
/// The `prime_brokerage` row counted live agreements. So no client in this world was levered by a
/// named lender: B1.a's *the client's leverage is a loan from a NAMED lender, not a property of the
/// client* had nothing to be true of, and §14's funds could not borrow at all.
///
/// The requirement is a DECISION by the broker, from its own view of the risk, never a
/// formula the client can rely on — which is what lets it RISE, and the rise is the procyclicality
/// XI-2 is about.
///
/// And no unlimited exposure: a broker with no limit is a synthetic counterparty. The limit
/// is its own and it binds; a limit that never binds is not a limit.
pub struct Broking {
    pub kind: u32,
    /// What the broker thinks the book could move this period. Its own view, and it is
    /// what the requirement is made of.
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
            // One broker per client here — a client with two brokers is real and is §12 E3's,
            // which needs each to see only its own book. This world gives each client one.
            let broker = brokers[n % brokers.len()];
            // The broker sets a margin requirement on the whole portfolio, from its own view
            // of the risk — so a portfolio it cannot value is one it cannot margin. A requirement
            // struck over the lines it could price and silently missing the rest would be a number
            // that looks like the whole book, and the client would be lent against
            // collateral nobody valued.
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
            // What this broker has already lent it. A client with no account has borrowed
            // nothing from it — that is the absence of a loan, not a loan of nothing — and an
            // account whose terms say nothing is one nobody has written terms for yet.
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
            // And where the account is short of what the broker requires, it CALLS. That is a
            // real demand on a named client for real money.
            let headroom = crate::mechanisms::prime_brokerage::headroom(&account, required);
            if headroom < 0.0 {
                calling.push((broker, client, -headroom));
            }
        }

        for (broker, client, lent, limit) in opening {
            // A loan from a NAMED lender. Terms `[lent, limit]`, and the limit is the
            // broker's own — E4's *no unlimited exposure* is this number existing at all.
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
            // A margin call the client cannot meet from cash is the first of the four doors,
            // and it is a WORKOUT — the client must find the money or sell.
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
///
/// The `hedge_funds` row counted how many parties were alive. So the one thing §14 C2 says a fund IS
/// — the other side of a forced sale, if it has capacity — had never happened, and XI-2's channel
/// had nobody at the end of it.
///
/// `None` is the case that matters: everybody short at once and nobody to buy. A fund only bids
/// what its borrowing room and its cash allow, so a world where every fund is out of room is a world
/// where a forced sale finds no bid — which is the contagion, and it is arithmetic here rather than
/// a rule anybody wrote.
///
/// And it bids because it DISAGREES: its own view of the name is better than the
/// worst view held, which is 22i.11's disagreement doing the work it exists for. A fund that agreed
/// with the seller would have no reason to take the other side.
pub struct Liquidity {
    pub of_kind: u32,
}

impl crate::module::Participant for Liquidity {
    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        // IF it has capacity. A fund with no room is not a buyer of anything, and that is
        // the case XI-2 turns on.
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
///
/// The `securities_lending` row counted live agreements. So nothing in this world had ever been
/// borrowed, which means nothing could be SHORTED — Appendix B's *no short without a borrow* held
/// only because no short was possible at all.
///
/// The fee is a price: scarce paper is expensive to borrow and abundant paper is
/// cheap, and it clears between what holders will lend and what borrowers want. A fee of zero is a
/// cleared price only if somebody posted it — `None` means nobody did and there is no borrow
/// rather than a free one.
///
/// Who wants to borrow is who disagrees: the party holding the WORST view of a
/// name wants to be short it, and the parties holding it are who can lend. Two different numbers on
/// one name is the whole reason either side is there.
pub struct StockLending {
    pub kind: u32,
    /// How much of what it holds a lender will put out at once. Its own limit, and it may
    /// be none.
    pub will_lend: &'static str,
}

impl Mechanism for StockLending {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::securities_lending::{clearing, Willing};
        let will_lend = ctx.params().ratio(self.will_lend);

        // The views held on each name, so the keenest short and the calmest holder are found
        // rather than assigned.
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
                // The pool is what holders will actually lend, which is their own limit on
                // their own holding and never all of it.
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
                // The fee CLEARS. What the borrower wants against what the pool will
                // lend, and the schedules are what each holder posted.
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
            // A loan of stock is a RELATION — terms `[the line, the units, the fee]` — and
            // C1's collateral, worth more than the loan, is what the two sides then post against it.
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
///
/// The `sovereign` row counted how many lines printed. So the sovereign funding constraint — step 3
/// of the sequencing — bound on nothing: a treasury short of money simply failed a payment, and
/// XI-9's three real acts, each with a consequence, had never happened.
///
/// It is a READ of the state, not a policy — which of the three is available is arithmetic about
/// what it holds and what it owes. The buffer is what the buffer is for, and it is smaller
/// afterwards. An outlay deferred is somebody not paid, and that is an EVENT with a named
/// counterparty, never a number quietly reduced. Past both, it goes back to the market, and a failed
/// auction has cost something — which is what makes its result carry information rather than being
/// decorative.
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
            // act XI-9 names rather than a new one.
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
///
/// The `commodities` row counted how many lines printed. So this world had no measure of scarcity at
/// all — D2's *when stocks approach zero the price has nothing left to ration with* had no stocks to
/// be about — and D3's storage cost, which is a real payment to a real owner of real storage, was
/// paid by nobody to nobody.
///
/// Tightness is a READ: stock against what is consumed in a period, and `Missing` where
/// nothing is consumed — a ratio over no consumption is not a ratio, and answering zero
/// would say the world is awash when in fact nobody has asked it.
///
/// And the fee is TWO-SIDED: whoever holds the stock pays whoever holds the storage.
/// A region with no stockist has nobody to pay, so nothing is charged — which is an answer about
/// that place and not a fee waived.
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

        // What was consumed, by line. ONE walk of the period's legs — Law 18: the traversal
        // is free to change and the mechanism is not, and a walk per line over half a million legs
        // is the same answer at sixteen thousand times the cost.
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
            // The measure of scarcity, published. It is what a price has left to ration with,
            // and a world that could not say it could not tell a squeeze from a glut.
            ctx.say(self.kind, &[], &[(0, Value::Num(f64::from(line))), (1, Value::Num(t))], true);
        }
        for (holder, keeper, fee) in charging {
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
            // Two named sides, in the same pass. A storage cost that came off a number
            // without reaching anybody would be the one-sided flow Law 5 is about.
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
///
/// The `housing` row was a CLOSER for a foreclosure nothing opened. So no dwelling in this world had
/// ever been offered, no rent had ever been set, no household had ever compared owning with
/// renting, and the consumer basket could not include the rent Housing D3 calls a large
/// component of it — because there was no rent to include.
///
/// A rent is a real payment between two named parties, and an owner-occupier pays
/// itself nothing: imputing a rent would be a flow with one side. So the letting session is between
/// owners with a spare dwelling and households without one, and what the roof costs is what that
/// session crossed at.
///
/// A buyer bids what it can FUND, which is its income and its deposit against the standard
/// its lender is currently lending at — the standard 22i.8 stands behind. A seller's reservation
/// is what it owes or what the dwelling cost to build, whichever is more: below that it
/// refuses, and a refusal is an outcome.
///
/// A dwelling is INDIVISIBLE: a trade is one dwelling and there is no partial fill. What did
/// not sell is not a residual — it stays with its owner, and B4's falling market is volumes
/// collapsing before prices do.
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
        // A buyer with no lender cannot bid. Missing is missing — it does not bid cash it
        // has not got, and it does not bid on a standard nobody is offering.
        let Some(standard) = keenest else { return };

        // The dwellings, where they are, and who lives in them. A dwelling is a structure
        // its holder holds; the occupier is its holder until a tenancy says otherwise.
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
                // It will not sell below what it owes or what a dwelling costs to build
                // there, whichever is more — and the build cost is higher where more already
                // stands. What the market last printed is what building it draws.
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

        // What each household can fund. Its income is what it has been paid — read off the wire
        // — and its deposit is what it holds.
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
            // What it expects to earn is its OWN outlook, and a household that has formed
            // none bids what it holds — not an income of zero, which would be a number nobody has
            // , but a bid with no borrowing behind it, which is what having no view of
            // your income means when a lender asks.
            let at = ctx.parties().region_of(who);
            let bidding = match ctx.outlooks().of(who, crate::stores::about::WHAT_IT_KEEPS_EARNING) {
                Some(income) => can_bid(income, deposit, &standard),
                None => deposit,
            };
            bids.push(Bid { buyer: who, at, bidding });
            renting.push((who, at, deposit));
        }

        // Per LOCATION. There is no single housing market, so each place clears on
        // its own and a print in one says nothing about another.
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
            // The rent is what this session crossed at — what the tenant will pay against a
            // roof that is standing empty, and an owner that will not let at it keeps it empty.
            let_to.push((owner, tenant, can_pay));
        }

        for (seller, buyer, price) in sold {
            // A loan from a NAMED lender, secured on the house. What the buyer cannot find
            // itself it borrows, and E3's *no mortgage without a lender's balance sheet* is the
            // lender being on the row.
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
            // A tenancy is a relation, and the rent is its term. It is what the occupier
            // pays the owner for the shelter it consumes — two named parties, and never imputed.
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
///
/// The `control` row was a CLOSER for a buy-back nothing opened, so §35 and §29 B were a thousand
/// lines nothing had ever exercised. What it waited on was a company with SHARES, which
/// 22i.7 built.
///
/// It does NOT wait for a published report. A target being bid for opens its books to
/// the bidder — that is what due diligence is — so the acquirer forms its expectation from what the
/// target is EARNING, read off the wire in any week. Gating this on §48's quarterly calendar made a
/// company unvaluable between its quarters and for the whole of its first one.
///
/// The acquirer's valuation is its OWN: the target's expected earnings discounted at its own
/// hurdle, so the next acquirer's is different — which is what makes a contest possible at all.
///
/// The owners decide, each on its own valuation: the price beats holding, on THEIR
/// valuation, or they keep the shares. A tender that does not reach the units it needs is REFUSED,
/// and that is a real outcome — a bid nobody accepts is not a bid that happened at a lower price.
///
/// And the premium is an OUTCOME, not a stated percentage: what it must pay to get them to
/// sell, measured against what the market last printed.
pub struct Control {
    pub kind: u32,
    /// What an acquirer wants on what it buys. Its own hurdle, and a PREFERENCE.
    pub hurdle: &'static str,
    /// The share of a company somebody must hold to control it, read off the outstanding
    /// count rather than declared — this is only how much of the rest a bid must reach.
    pub needs: &'static str,
}

impl Mechanism for Control {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        use crate::mechanisms::control::{tender, Bid, Consideration, Outcome, Owner};
        let hurdle = ctx.params().ratio(self.hurdle);
        let needs = ctx.params().ratio(self.needs);

        // THE ACQUIRER'S OWN VALUATION, of the target's EXPECTED earnings. It does not
        // wait for a published report and must not: a target being bid for OPENS ITS BOOKS to the
        // bidder — that is what due diligence IS — and gating on §48's calendar made a company
        // unvaluable between its quarters and for its whole first quarter.
        //
        // What an acquirer expects is what the target is EARNING: the money it has taken in against
        // the money it has paid out, over the period, read off the wire. That is a read of what
        // settled (§48 A2's rule, applied to a private look rather than a public report), and it is
        // available in any week rather than four times a year.
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
                    // What a company EARNS is what it sells, less what it pays for what it uses.
                    // A receipt says which a payment is, so this reads the leg's own word
                    // for it rather than guessing from the parties.
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
            // The acquirer's OWN valuation. Whoever already holds the most of it is who has
            // a reason to take the rest — and what it is worth to that holder is what its own
            // hurdle says, never what the market says.
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
                // What holding is worth to THIS owner — what the market last printed, which
                // is what it could get for it now. Its own, and it is why some accept and some do not.
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
                // It will not pay a premium it does not think is there. A bid below the market
                // is not a bid, and nothing is adjusted to make one.
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
                // What its lenders committed. A bid it cannot fund is not a bid, and here
                // what it can fund is what it holds — the credit market's half is §29 B2.b's.
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
            // A bid that nobody beats is not a proof that it was the right price, only that
            // nobody came — so what happened is said either way.
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
///
/// The `derivative_layer` row counted live agreements. So §16's one indispensable fact — that a
/// party which has hedged is FLAT on the market and DOUBLED on the credit — had nothing to be true
/// of, and a world that netted them would hide the thing that actually breaks.
///
/// One number, two reads: what a contract is worth to one side is the negative of what it
/// is worth to the other. Not two numbers that might drift apart — the same number, read from either
/// end.
///
/// And the exposure is PER COUNTERPARTY: a party with a bought contract and a sold one
/// against different names has two exposures, and netting them across counterparties is exactly what
/// Appendix B forbids.
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
            // It was agreed at a CLEARED price and it marks against one — never against a
            // price this world does not clear. What it is worth now is what that name's protection
            // would cost now, against what this contract struck at.
            let on = InstrumentId::at(struck_at as u32);
            let mark = match ctx.prints().latest(on, ctx.period()) {
                Some(print) => (print.price - struck_at) * notional,
                // A contract on something nothing has cleared does not mark, and a position that
                // does not mark is one whose holder has hidden its loss. It is said with
                // no mark rather than marked at zero.
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
            // One number, two reads. The other side's is its negative, and it is read rather
            // than written a second time.
            if let Some(to_one) = position.mark_to(one) {
                marked.push((one, other, to_one));
            }
        }

        for (one, other, mark) in marked {
            // Per counterparty. A party's exposure to one name is not reduced by an
            // offsetting contract with another, so this is said naming both — netting them across
            // counterparties is what Appendix B forbids, and a single number per party would be it.
            ctx.say(self.kind, &[one.0, other.0], &[(self.at_mark, Value::Num(mark))], false);
        }
    }
}

/// THE TIER THAT IS TOO SMALL FOR THE BOND MARKET.
///
/// The `small_business` row counted the credit outstanding. So A5's *bank-dependent — too small for
/// the bond market* was a sentence about nobody: nothing in this world knew which borrowers had no
/// alternative, which is why a credit tightening could not bite here first and hardest.
///
/// It is a READ of observable characteristics: size, region, leverage, coverage — and
/// bank-dependence is size against the point where the bond market starts, never a label. A cell
/// that outgrows it is PROMOTED, which is a weight event and not a relabelling.
///
/// And the default is one borrower's arithmetic: a threshold this cell crosses or does
/// not, never an average. A test applied to a band's mean means a mean-preserving spread causes no
/// defaults at all, which is exactly backwards.
pub struct SmallBusiness {
    pub kind: u32,
    /// The size at which a borrower reaches the bond market. A TECHNOLOGY of the market:
    /// below it the issue costs more than it raises.
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
                // Coverage is what it earns against what it owes, and a borrower with no
                // published earnings has none that anybody can read.
                coverage: 0.0,
                bank_dependent: true,
            };
            // It outgrew bank-dependence — large enough to reach the bond market. That is
            // a fact about its size against a market convention, and never a label anybody set.
            let dependent = !cell.outgrew(reaches);
            // And whether THIS borrower can meet what falls due. One borrower's own
            // arithmetic, never a band's average.
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
            // Which borrowers have no alternative, and which of those cannot meet what falls
            // due — the two facts a credit tightening needs to bite here first and hardest.
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
///
/// The `cross_border` row counted how many lines printed. So no region in this world had a current
/// account, a financial account or an imbalance — and F2's *the world closes*, the check that every
/// region's surplus is somebody's deficit, had nothing to check.
///
/// Every flow is read off the WIRE, party by party: who paid whom, where each of
/// them is, and what the payment was FOR. Nothing is a tally kept beside the instructions, and no
/// series is exogenous — Appendix B forbids an imposed trade or capital-flow series precisely
/// because it would make this read decorative.
///
/// The two accounts are the same flows read twice, which is why the imbalance is derived and
/// can be reported with a size rather than asserted away.
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
                // What the payment was FOR decides which account it lands in. It is
                // read off the leg's own receipt, which the writer had to state — never
                // guessed from the parties.
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
                    // What money this is, read off the instrument that IS it. The
                    // leg used to carry a second copy and nothing validated it, so these accounts
                    // were the only reader of a field the wire never checked.
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
        // And the world closes. Every region's surplus is somebody's deficit, so the sum
        // over all of them is dust or it is a finding about a flow with one side.
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

/// A FUND CALLS ITS COMMITMENTS, AND THE INVESTOR MUST HAVE THE
/// MONEY.
///
/// The `private_equity` row was a CLOSER for a takeover nothing opened. §29's own half — the fund,
/// its commitments and the call — had never run either: A2.a's *the investor must hold liquidity
/// against calls it did not choose the timing of* had no call to be against.
///
/// A call is pro rata on UNCALLED commitments, and it is a real demand on a named investor
/// for real money. The fund has a LIFE: it invests, it holds, it exits and it winds up —
/// nothing here is immortal.
///
/// And whether the deal happens is the credit market's: the buyout half is `control`'s,
/// which 22i.18 built. This is the money behind it.
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
            // The investor must hold liquidity against a call it did not choose the timing
            // of. If it has not got it the payment waits in the queue like any other and can run
            // out of days — which is the consequence that makes a commitment mean something.
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
        // A mechanism holds the ID of the number it acts on, so a test that runs one declares
        // the number first — half, here, because half the way is easy to check by eye.
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

    /// A mechanism holds the ID of the number it acts on, so a world that runs one declares
    /// it. Ten square km, because a place carrying ten is then exactly twice as dear to build in.
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
        // The mechanism the whole credit side rests on. The beneficiary is read off the
        // register — never a second list of who is owed what.
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
        // Register E1, A2.a, Appendix B #10. This paid the FIRST row that was not the issuer,
        // in full: a line held by three parties paid all of it to one of them, and what the other
        // two were owed and did not get was a residual with no holder. `Servicing` runs the whole
        // credit side — every loan, bond, premium and rent in the world went this way.
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
        // schedule discharged.
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
        // XI-5, 5 D2: one obligation, one instruction. A pass that paid some holders and not
        // others would make `settles(due)` a lie either way, and the arrear it left would be for
        // an amount nobody could name.
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
        // Employment is a relation, and the wage is what it pays. An engagement that
        // ended is still readable and pays nothing, which is what ending means.
        let (mut w, _bank, firm, worker, cash) = world();
        w.register.money_delta(firm, cash, 9_000.0);
        // The worker is a CELL of a hundred, the engagement is for all hundred,
        // and a wage is per person — so what the firm owes is a hundred wages. Paying one was the
        // defect: a firm employing two thousand people paid forty.
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
        // The disagreement is LOAD-BEARING. A world where everybody expected the same would
        // trade once and stop.
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
        // Giving a benchmark a schedule would be inventing demand nobody has. What it
        // does is publish a read, and the count is what shows it ran.
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
        // Nothing is immortal, a process included — and the KERNEL is what closes
        // one. Seven systems used to be a `Closing`, a closer for a process nothing opened; 22i
        // gave every one of them an opener, which left the closing itself with seven writers of one
        // rule. It is the kernel's now, once, for every kind.
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
        // people's — which is the half of 21h `Making` had not been told.
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
        // 37 B1, B2, B3, B4: the inputs go NOW and the output arrives when the line is done — and in
        // between there is work in progress, owned, carrying what it cost.
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
        // B5, 33 A3: inputs at their own lot basis, plus the wages the hours cost, plus the period's
        // upkeep and depreciation on the plant that ran. B4: over what FINISHED, so the scrap is
        // absorbed into the survivors.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)]);
        ran(&mut w, &making(&line, mill));
        // The cost is carried by the batch on the line, and it is what the lot is struck at when
        // the batch comes off — so what a unit cost is what went into IT, not what things cost then.
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
        // Expected demand is the first reason, and it is the firm's OWN. Missing is missing
        //  — a firm that has never sold anything does not produce as if it expected zero,
        // it does not produce at all, and the difference is that it starts the moment it sells once.
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
        // 21i, 33 A4: congestion, not scarcity. Two identical firms build the same structure,
        // one on empty ground and one where the declared doubling area already stands. Nothing is
        // refused and nothing is capped — the crowded one simply draws twice as much of everything
        // for the same output, which is deeper foundations and more hours on a tight site.
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
        // The holders' claim is redeemable, so what the pool raised goes back to them in
        // proportion — 300 shares of 1,000 is 270 of 900, and it is the fund's own money moving over
        // the ordinary wire, not a transfer somebody arranged.
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
        // Nothing is immortal, and a thing that ends says when. Law 6: it is not a schedule —
        // the pool ends because there is nothing left, which is arithmetic about what it holds.
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
        // No forced buyer. A book that will not take its stock leaves it unsold,
        // and the pool is still there next period — the absence of a buyer showing up as a duration.
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
        // The estate paid the treasury 388 directly and the audit said the treasury had
        // no claim on it. Now the state is a CLAIMANT, standing behind the secured creditor, and it
        // gets what is left of its rank rather than what it asked for.
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
        // An estate is what is left of a party whose life has ended. A claim against a party
        // that is still alive is not paid by this mechanism — it is the party's own to pay.
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
        // 21.36, and it was a defect this engine had for three commits. `Ranked` read
        // `outstanding` and marked nothing, so every claim was paid IN FULL AGAIN every period —
        // the shape 21.36 measured on the old engine, where a dead firm's debt stood twice on the
        // register and grew by every claim every period until a test timed out.
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
        // What a claimant did not get is a LOSS on a named holder, and it stands until the
        // estate has something to pay it with. The estate has 40 against a claim of 100.
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

        // The estate realises something more, and the rest of the claim is paid — and only the rest.
        w.register.money_delta(estate, cash, 100.0);
        w.period = 2;
        ran(&mut w, &Ranked { says });
        assert_eq!(w.claims.outstanding(c), 0.0);
        assert_eq!(w.register.quantity(w.register.row(secured, cash)), 100.0);
        assert_eq!(w.register.quantity(w.register.row(estate, cash)), 40.0);
    }
}
