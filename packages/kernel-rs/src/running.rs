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
}
