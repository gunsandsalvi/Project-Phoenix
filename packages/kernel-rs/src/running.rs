//! WHAT EACH SYSTEM DOES IN A PERIOD.
//!
//! @spec ARCHITECTURE 4.9b · Law 4, Law 5, Law 10, Law 15, Law 19 · Appendix B
//!
//! `systems.rs` is the list; this is what the rows on it DO when their phase runs. It sits beside the
//! assembly rather than inside `mechanisms/` for one reason: it names every module, and a module may
//! never import another module (Law 15). The adapter layer is the assembly's, not any module's.
//!
//! **Every mechanism here reads the world through the second door and proposes.** None of them
//! invents a number at the boundary: where the world holds nothing for a system yet, it proposes
//! nothing, which is an answer and not a gap — and it becomes live the moment the world holds
//! something. That is what makes this wiring rather than a fixture.
//!
//! **The shape is always the same**: the READ pass first, over the kernel's own stores, then the
//! proposals. A module reads and proposes; it cannot do both at once, which is the borrow checker
//! saying what Law 4 already says.

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::instruments::{equity, Class};
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::stores::Owing;

/// The agreement kinds this world has. **Registry data** (Law 15): `Agreements` holds a kind id and
/// never knows what an engagement is, and a mechanism asks for its own kind's rows.
pub mod agreed {
    /// XI-10, §39: an employer and a worker. **Its terms are `[wage per period, hours per period]`,
    /// and this is the one place that convention is stated** (Law 4): `Wages` pays the first and
    /// `Making` draws on the second, and a reader who wants to know what a term means comes here
    /// rather than to whichever mechanism happened to be open.
    pub const ENGAGEMENT: u32 = 0;
    pub const MORTGAGE: u32 = 1;
    pub const POLICY: u32 = 2;
    pub const SUPPLY: u32 = 3;
    pub const TENANCY: u32 = 4;
    pub const MANDATE: u32 = 5;
    pub const SUBSCRIPTION: u32 = 6;
    pub const PRIME_BROKERAGE: u32 = 7;
    pub const SECURITIES_LOAN: u32 = 8;
    pub const DERIVATIVE: u32 = 9;
    pub const CARRIAGE: u32 = 10;
    pub const TRADE_CREDIT: u32 = 11;
}

/// The processes this world runs. Same rule: data, not a branch.
pub mod afoot {
    pub const CAPITAL_PROGRAMME: u32 = 0;
    pub const FORECLOSURE: u32 = 1;
    pub const BUY_BACK: u32 = 2;
    pub const ELECTION: u32 = 3;
    pub const WORKOUT: u32 = 4;
    pub const FLOTATION: u32 = 5;
    pub const TAKEOVER: u32 = 6;
    pub const SECURITISATION: u32 = 7;
}

/// What a party's outlook is ABOUT. §46: the subject is whatever the asking module declared, and two
/// parties holding different numbers about the same subject is the point.
pub mod about {
    pub const WHAT_IT_SELLS_FOR: u32 = 0;
    pub const WHAT_IT_KEEPS_EARNING: u32 = 1;
    pub const WHAT_CREDIT_COSTS: u32 = 2;
    pub const WHAT_A_HOUSE_IS_WORTH: u32 = 3;
    pub const WHETHER_IT_IS_PAID_BACK: u32 = 4;
    /// 37 B1: **how much it expects to sell** — a quantity, and a different fact from the price it
    /// expects to get. It is the firm's own, formed from what it actually delivered (§46), and it is
    /// the first of the production decision's reasons.
    pub const HOW_MUCH_IT_SELLS: u32 = 5;
}

/// **§6, §7, XI-9: WHAT FALLS DUE IS PAID, OR IT IS AN ARREAR.**
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
        let mut paying: Vec<(PartyId, PartyId, InstrumentId, f64, Receipt, crate::stores::DueId)> = Vec::new();
        for due in ctx.schedules().falling(from, to) {
            let line = ctx.schedules().instrument_of(due);
            let owes = ctx.schedules().owed_by(due);
            // Appendix B: no liability without a beneficiary. Whoever HOLDS the line is owed, which
            // the register says — never a second list of who is owed what.
            let holders = ctx.register().of_instrument(line);
            let owed_to = holders
                .iter()
                .map(|r| ctx.register().holder_of(crate::ids::HoldingId(*r)))
                .find(|h| *h != owes);
            let (to_whom, money) = match (owed_to, account_of(ctx.parties(), ctx.instruments(), owes)) {
                (Some(w), Some(m)) => (w, m),
                // Nobody holds it, or the payer has no account to pay from: there is nothing to
                // propose, and inventing either would be inventing a counterparty.
                _ => continue,
            };
            let receipt = match ctx.schedules().of(due) {
                Owing::Interest => Receipt::Interest,
                Owing::Principal => Receipt::Principal,
                Owing::Premium | Owing::Rent => Receipt::Transfer,
            };
            paying.push((owes, to_whom, money, ctx.schedules().amount(due), receipt, due));
        }
        for (from_whom, to_whom, money, amount, receipt, due) in paying {
            ctx.propose(
                vec![Leg::Money {
                    from: from_whom,
                    to: to_whom,
                    ccy: ctx.instruments().ccy_of(money),
                    instrument: money,
                    amount,
                    receipt,
                }],
                Cause::Payment,
                Delivery::Nothing,
                "what fell due on the schedule this period",
            );
            ctx.settles(due);
        }
    }
}

/// **§39, XI-10: AN ENGAGEMENT IS A RELATION, AND A WAGE IS WHAT IT PAYS.**
///
/// The employment module's `Engagement` had nowhere to live, so nobody was ever paid by one. It lives
/// in `Agreements` now: an employer, a worker, a wage as its first term, a start and an end.
pub struct Wages;

impl Mechanism for Wages {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut owed: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        for row in ctx.agreements().of_kind(agreed::ENGAGEMENT) {
            let a = crate::stores::AgreementId(*row);
            if !ctx.agreements().live(a) {
                continue;
            }
            let (employer, worker) = ctx.agreements().between(a);
            let wage = match ctx.agreements().terms(a).first() {
                Some(w) => *w,
                // An engagement with no wage is a relationship nobody agreed the terms of.
                None => continue,
            };
            if let Some(money) = account_of(ctx.parties(), ctx.instruments(), employer) {
                owed.push((employer, worker, money, wage));
            }
        }
        for (employer, worker, money, wage) in owed {
            ctx.propose(
                vec![Leg::Money {
                    from: employer,
                    to: worker,
                    ccy: ctx.instruments().ccy_of(money),
                    instrument: money,
                    amount: wage,
                    receipt: Receipt::Wage,
                }],
                Cause::Payment,
                Delivery::Nothing,
                "the week's wages on a standing engagement",
            );
        }
    }
}

/// **§37: WHAT A FIRM MAKES, AND THE PLANT IT MAKES IT WITH.** Registry data (Law 15), one row per
/// good this world knows how to produce.
///
/// **Who makes it is not declared here, and that is the point.** A maker is whoever holds the plant —
/// so production follows the capital rather than the party kind, entry is a firm buying plant, and
/// exit is a firm selling it. A mechanism that asked a party what kind it was would be Law 15's
/// defect and would also be wrong: a bank that bought a mill makes flour.
#[derive(Clone)]
pub struct Makes {
    pub line: crate::mechanisms::recipe::Line,
    /// A4.a: what THIS line's plant is. Capital is specific in kind (33 A4).
    pub plant: InstrumentId,
    pub plant_is: crate::mechanisms::capital_programme::Plant,
}

/// **§37 A2, B1–B5: THE FIRM PRODUCES.**
///
/// The one thing in this world that makes anything, and the read pass is the firm's reasons: what it
/// expects to sell (its own outlook, §46), the capacity of the plant it holds (33 A2), the inputs on
/// its own register rows (B1.b), and the hours its engagements give it (B1.c). It picks the way of
/// making the good that costs IT least at the prices IT can see (A2), runs the line in whole batches
/// (B5.b), and what comes out is the OUTCOME of those reasons and never a quantity anybody chose.
///
/// **The proposal is one instruction.** The inputs are destroyed as consumed and the output is
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
    per_unit: f64,
}

pub struct Making {
    pub makes: Vec<Makes>,
    /// E5: the flow, declared once and applied consistently — and it is the order settlement itself
    /// draws lots in, so the cost this books and the units that leave cannot disagree (Law 4).
    pub flow: crate::mechanisms::goods::CostFlow,
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

                // B1: its own outlook, and NOT a model forecast (§46). A firm with no view of what it
                // sells has no reason to start a line, and that is missing rather than nothing.
                let Some(expects) = ctx.outlooks().of(maker, about::HOW_MUCH_IT_SELLS) else {
                    continue;
                };

                // B1.c: the hours its engagements give it, and what an hour of them costs. The terms
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
                    match (terms.first(), terms.get(1)) {
                        (Some(w), Some(h)) => {
                            wage_bill += w;
                            hours += h;
                        }
                        // An engagement that does not say how long it is for buys no hours.
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

                // A2, B5: it picks the way that costs IT least — and **what an input costs IT is
                // what it paid for the stock it holds**, off its own lots, because that is the stock
                // the batch will actually consume and the number `unit_cost` will book. Where it
                // holds none of an input it would have to buy it, and what it would pay is the
                // market's print (Law 3, never a number this module made up).
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

                // B1.b: what it holds of each input, off its own rows. An input it has no row for is
                // one it has none of, and `decide` is where that stops the line.
                let on_hand: Vec<(InstrumentId, f64)> = way
                    .per_unit
                    .iter()
                    .map(|(what, _)| (*what, ctx.register().quantity(ctx.register().row(maker, *what))))
                    .collect();

                let d = decide(
                    way,
                    &Reasons { firm: maker, expected_demand: expects, capacity: can_make, on_hand, labour: hours },
                );
                if d.starts <= 0.0 {
                    continue;
                }

                // B2, E5: what the draw costs, at the lots' own basis and in the order settlement
                // will draw them in — so what this books and what leaves cannot disagree (Law 4).
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
                let Some(per_unit) = unit_cost(inputs_cost, wages, capital, d.finishes) else {
                    continue;
                };
                runs.push(Ran { maker, makes: m.line.makes, draws, finished: d.finishes, per_unit });
            }
        }

        // THE PROPOSALS. B2 and B3 in ONE instruction: a world where the inputs went and the output
        // did not arrive is a world that ate them.
        for Ran { maker, makes, draws, finished, per_unit } in runs {
            let mut legs: Vec<Leg> = draws
                .iter()
                .map(|(what, qty)| Leg::Destroy {
                    party: maker,
                    instrument: *what,
                    qty: *qty,
                    why: crate::ledger::Gone::Consumed,
                })
                .collect();
            legs.push(Leg::Create { party: maker, instrument: makes, qty: finished, cost_per_unit: per_unit });
            ctx.propose(legs, Cause::Production, Delivery::Nothing, "the batches the line ran this period");
        }
    }
}


/// **§46, XI-16: EVERY DECIDING PARTY FORMS ITS OWN OUTLOOK FROM ITS OWN HISTORY.**
///
/// One PREFERENCE — how much weight it gives the surprise — and no global expectation anywhere. What
/// it forms its outlook ABOUT is what it can see: the price its own lines last printed at.
///
/// **Outlooks disagree because the parties see different things**, which is what gives a market two
/// sides. A world where everybody expected the same would trade once and stop (§46 A3).
pub struct Forming {
    /// §46: the memory — how much of the new observation displaces the old. The one primitive here.
    pub memory: f64,
}

impl Mechanism for Forming {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        assert!(self.memory > 0.0 && self.memory <= 1.0, "§46: a memory outside its own range is not one");
        let mut formed: Vec<(PartyId, u32, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            if !ctx.parties().alive(who) {
                continue;
            }
            // Law 19: it looks at ITS OWN rows and the prints those lines actually made. A party
            // that holds nothing has seen nothing and forms nothing — which is not an outlook of
            // zero (Appendix A).
            let mut seen = 0.0;
            let mut lines = 0.0;
            for row in ctx.register().of_holder(who) {
                let line = ctx.register().instrument_of(crate::ids::HoldingId(*row));
                if let Some(print) = ctx.prints().latest(line, ctx.period()) {
                    seen += print.price;
                    lines += 1.0;
                }
            }
            if lines <= 0.0 {
                continue;
            }
            let now = seen / lines;
            let was = ctx.outlooks().of(who, about::WHAT_IT_SELLS_FOR);
            // §46 B1: adaptive. The first observation IS the outlook; after that the surprise moves
            // it by the party's own memory.
            let level = match was {
                Some(old) => old + self.memory * (now - old),
                None => now,
            };
            formed.push((who, about::WHAT_IT_SELLS_FOR, level));
        }

        // 37 B1, §46: **and how much it expects to sell**, which is a different fact from the price
        // and is the first reason the production decision has. It is formed the same adaptive way,
        // from the one source that is not an inference: what it actually DELIVERED last period, read
        // off the wire (Money D1, Law 19). A firm that has never delivered has no view of its demand
        // and forms none — which is missing, not a demand of zero.
        let mut delivered: Vec<(PartyId, f64)> = Vec::new();
        for n in ctx.wire().in_period(ctx.period()) {
            for leg in ctx.wire().legs_of(n) {
                if let Leg::Asset { from, qty, .. } = *leg {
                    match delivered.iter_mut().find(|(who, _)| *who == from) {
                        Some((_, units)) => *units += qty,
                        None => delivered.push((from, qty)),
                    }
                }
            }
        }
        for (who, units) in delivered {
            if !ctx.parties().alive(who) {
                continue;
            }
            let level = match ctx.outlooks().of(who, about::HOW_MUCH_IT_SELLS) {
                Some(old) => old + self.memory * (units - old),
                None => units,
            };
            formed.push((who, about::HOW_MUCH_IT_SELLS, level));
        }

        for (who, subject, level) in formed {
            ctx.form(who, subject, level);
        }
    }
}

/// **§32: A FIRM'S RESULT IS PUBLISHED, and it is a read of what actually happened to it.**
///
/// Law 19: revenue, cost and what it is worth are read off the register and the wire — never a
/// running total a module kept beside them.
pub struct Reporting {
    /// The event kind this publishes under, declared by the assembly.
    pub kind: u32,
}

impl Mechanism for Reporting {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut said: Vec<(u32, f64)> = Vec::new();
        for f in ctx.parties().of_kind(kinds::FIRM) {
            let who = PartyId(*f);
            if !ctx.parties().alive(who) {
                continue;
            }
            said.push((*f, equity(who, ctx.register(), ctx.instruments())));
        }
        for (who, worth) in said {
            // Observer A3: a firm's own result reaches its own subjects. What it publishes to the
            // world is §48's, and it is not this.
            ctx.say(self.kind, &[who], &[(0, Value::Num(worth))], false);
        }
    }
}

/// **XI-3: NOTHING IS IMMORTAL, and a process is not either.**
///
/// Whatever is in flight closes when its period comes. A process with no end is one nobody has to
/// finish, which is how a world accumulates things that never resolve.
pub struct Closing {
    pub kind: u32,
    /// What it says when one closes.
    pub says: u32,
}

impl Mechanism for Closing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut done: Vec<(u32, f64)> = Vec::new();
        for p in ctx.processes().running(self.kind) {
            if matches!(ctx.processes().closes(p), Some(when) if when <= ctx.period()) {
                done.push((ctx.processes().owner(p).0, ctx.processes().size(p)));
            }
        }
        for (owner, size) in done {
            ctx.say(self.says, &[owner], &[(0, Value::Num(size))], true);
        }
    }
}

/// **A SYSTEM THAT READS WHAT THE BOOKS PRODUCED.**
///
/// Benchmarks, ratings, the observer surface, the second opinion: they publish a read over what
/// already happened and propose nothing. That is not a stub — **giving them a schedule would be
/// inventing demand nobody has** (Appendix B) — and the count it publishes is what makes it visible
/// that it ran.
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
        // Observer A3: a read over what the books produced is PUBLIC. That is what a benchmark is.
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}

/// **Money A1, Money D2: WHAT EACH ISSUER OWES ITS HOLDERS**, published once a period.
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

#[cfg(test)]
mod tests {
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
        (w, bank, firm, worker, cash)
    }

    fn ran(w: &mut World, m: &dyn Mechanism) {
        let mut ctx = MechanismContext::of(
            w.period,
            crate::module::Stores {
                parties: &w.parties,
                instruments: &w.instruments,
                register: &w.register,
                prints: &w.prints,
                journal: &w.journal,
                params: &w.params,
                agreements: &w.agreements,
                schedules: &w.schedules,
                outlooks: &w.outlooks,
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
                    instruments: &w.instruments,
                },
                w.settled_kind,
                w.failed_kind,
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
    }

    #[test]
    fn what_falls_due_this_period_is_paid_to_whoever_holds_the_line() {
        // §6, XI-9: the mechanism the whole credit side rests on. The beneficiary is read off the
        // register — never a second list of who is owed what (Appendix B).
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
        // §39, XI-10: employment is a relation, and the wage is what it pays. An engagement that
        // ended is still readable and pays nothing, which is what ending means.
        let (mut w, _bank, firm, worker, cash) = world();
        w.register.money_delta(firm, cash, 500.0);
        let hired = w.agreements.strike(agreed::ENGAGEMENT, firm, worker, &[40.0], Day(-100), None);

        w.period = 1;
        ran(&mut w, &Wages);
        assert_eq!(w.register.quantity(w.register.row(worker, cash)), 40.0);

        w.agreements.end(hired);
        ran(&mut w, &Wages);
        assert_eq!(w.register.quantity(w.register.row(worker, cash)), 40.0, "an ended engagement pays nothing");
    }

    #[test]
    fn two_parties_seeing_different_prices_form_different_outlooks() {
        // §46 A3: the disagreement is LOAD-BEARING. A world where everybody expected the same would
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
        ran(&mut w, &Forming { memory: 0.5 });
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), Some(3.0));
        assert_eq!(w.outlooks.of(worker, about::WHAT_IT_SELLS_FOR), Some(9.0));
        assert_eq!(w.outlooks.spread_on(about::WHAT_IT_SELLS_FOR).len(), 2);
    }

    #[test]
    fn a_party_that_has_seen_nothing_forms_nothing_and_that_is_not_zero() {
        // Appendix A: missing is missing. An outlook of zero is an expectation.
        let (mut w, _bank, firm, _worker, _cash) = world();
        w.period = 1;
        ran(&mut w, &Forming { memory: 0.5 });
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), None);
    }

    #[test]
    fn the_second_observation_moves_the_outlook_by_the_partys_own_memory() {
        // §46 B1: adaptive, from its OWN history. The first observation IS the outlook.
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
            ran(&mut w, &Forming { memory: 0.5 });
        }
        // 4 first, then half the way from 4 to 8.
        assert_eq!(w.outlooks.of(firm, about::WHAT_IT_SELLS_FOR), Some(6.0));
    }

    #[test]
    fn a_read_over_what_the_books_produced_publishes_and_proposes_nothing() {
        // Appendix B: giving a benchmark a schedule would be inventing demand nobody has. What it
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
        // XI-3: nothing is immortal, a process included.
        let (mut w, _bank, firm, _worker, _cash) = world();
        let says = w.journal.kinds.declare("programme.closed");
        w.processes.begin(afoot::CAPITAL_PROGRAMME, firm, 1, Some(3), 500.0);
        w.period = 2;
        ran(&mut w, &Closing { kind: afoot::CAPITAL_PROGRAMME, says });
        assert_eq!(w.journal.in_period(2).count(), 0);
        w.period = 3;
        ran(&mut w, &Closing { kind: afoot::CAPITAL_PROGRAMME, says });
        assert_eq!(w.journal.in_period(3).count(), 1);
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
        // `[wage, hours]`, the convention `agreed::ENGAGEMENT` states.
        w.agreements.strike(agreed::ENGAGEMENT, firm, worker, &[80.0, 40.0], Day(-7), None);
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
        }
    }

    #[test]
    fn a_firm_with_plant_inputs_hours_and_a_view_of_its_demand_actually_makes_something() {
        // 37 B1, B2, B4: the inputs go, the output arrives, and both are in ONE instruction — a
        // world where the inputs went and the output did not is a world that ate them.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0)]);

        let flour_before = w.register.quantity(w.register.row(firm, flour));
        ran(&mut w, &making(&line, mill));

        // It wanted 200/0.98 = 204 starts, had capacity for 300 and flour for 450, so the batch
        // rounded it to 200 — and 196 loaves came out of 400 sacks.
        let made = w.register.quantity(w.register.row(firm, bread));
        assert_eq!(made, 196.0);
        assert_eq!(flour_before - w.register.quantity(w.register.row(firm, flour)), 400.0);
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
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0)]);
        ran(&mut w, &making(&line, mill));

        let lots = w.register.lots(w.register.row(firm, bread));
        assert_eq!(lots.len(), 1);
        // 400 sacks at 0.5 is 200; 20 hours at 2 an hour is 40; the capital service is the mill's
        // own keep over what it can make. All of it over 196 loaves.
        assert!(lots[0].basis_per_unit > (200.0 + 40.0) / 196.0);
    }

    #[test]
    fn a_firm_with_no_view_of_its_own_demand_has_no_reason_to_start_a_line() {
        // B1, §46: expected demand is the first reason, and it is the firm's OWN. Missing is missing
        // (Appendix A) — a firm that has never sold anything does not produce as if it expected zero,
        // it does not produce at all, and the difference is that it starts the moment it sells once.
        use crate::mechanisms::recipe::{Line, Recipe};
        let (mut w, firm, flour, bread, mill) = a_mill();
        w.period = 1;
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0)]);
        ran(&mut w, &making(&line, mill));
        assert_eq!(w.register.quantity(w.register.row(firm, bread)), 0.0);

        // And it starts the moment it has one.
        w.outlooks.form(firm, about::HOW_MUCH_IT_SELLS, 200.0, 1);
        ran(&mut w, &making(&line, mill));
        assert!(w.register.quantity(w.register.row(firm, bread)) > 0.0);
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
        w.agreements.strike(agreed::ENGAGEMENT, bank, cb, &[80.0, 40.0], Day(-7), None);
        w.outlooks.form(bank, about::HOW_MUCH_IT_SELLS, 100.0, 1);
        let line = Line::new(bread, vec![Recipe::new(bread, vec![(flour, 2.0)], 0.1, 0.05, 0.98, 10.0)]);
        ran(&mut w, &making(&line, mill));
        assert!(w.register.quantity(w.register.row(bank, bread)) > 0.0);
    }
}
