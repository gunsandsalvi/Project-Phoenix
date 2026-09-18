//! THE ASSEMBLY: the systems, wired into one world that steps.
//!
//! @spec ARCHITECTURE 4.9b · 1 G3 · 3 B2 · 4 · Law 4, Law 10, Law 15, Law 19
//!
//! A module reaches the kernel through three doors and nothing else: `ParticipantView`,
//! `MechanismContext`, `SeedContext`. This is where the doors are hung — every system declares its
//! kinds, its phases and its participants here, and **the kernel asks a kind's profile and never
//! branches on its id** (Law 15).
//!
//! **Adding a system is one row in `systems()`.** That is the whole of what "wired" means: a module
//! that is not in that list does not run, and one that is runs exactly like every other.
//!
//! **What is a participant and what is not.** A system appears in a book only if it has a REASON to
//! post there (Clearing B2). The systems that do not — the benchmarks, the observer surface, the
//! ratings, the audit families — are reads over what the books produced, and giving them a schedule
//! would be inventing demand nobody has (Appendix B: no demand added to clear).

use crate::clearing::{Order, PriceRule, Side};
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::instruments::{Class, Instruments};
use crate::journal::Journal;
use crate::ledger::{Instruction, Settlement, Settling};
use crate::module::{Mechanism, MechanismContext, Participant, ParticipantView, Stores as Reads};
use crate::params::Params;
use crate::parties::Parties;
use crate::nouns::{NounDecl, Nouns, Sort};
use crate::prices::Prints;
use crate::stores::{Agreements, Claims, Outlooks, Processes, Schedules};
use crate::register::Register;
use crate::session::{run_book, BookDecl, Books, Shown, Stores};
use crate::world::{Anchor, PhaseDecl, Phases, CORPORATE_ACTIONS, MARKETS, REVALUATION};

/// The party kinds this world has. **Registry data, declared once** — a mechanism asks a kind's
/// profile and never tests an id (Law 15), and these exist so the assembly can say which parties a
/// participant is asked about.
pub mod kinds {
    pub const HOUSEHOLD: u32 = 0;
    pub const FIRM: u32 = 1;
    pub const BANK: u32 = 2;
    pub const FUND: u32 = 3;
    pub const INSURER: u32 = 4;
    pub const DEALER: u32 = 5;
    pub const TREASURY: u32 = 6;
    pub const CENTRAL_BANK: u32 = 7;
    pub const CARRIER: u32 = 8;
    pub const SMALL_FIRM: u32 = 9;
    pub const ASSESSOR: u32 = 10;
    pub const ALL: [u32; 11] = [
        HOUSEHOLD, FIRM, BANK, FUND, INSURER, DEALER, TREASURY, CENTRAL_BANK, CARRIER, SMALL_FIRM,
        ASSESSOR,
    ];
}

/// One spec system, wired. It declares what it adds to the world and nothing more — the kernel owns
/// every store, and a module never writes one (ARCHITECTURE 4.9b).
pub trait System {
    /// The spec system this is, for the record and for the census.
    fn name(&self) -> &'static str;

    /// Law 10: where its work falls in the period. Anchored to one of the three moments or to
    /// another declaration, never floating.
    fn phases(&self) -> Vec<PhaseDecl> {
        Vec::new()
    }

    /// Clearing B2: who it puts into books, if anybody. A system with no reason to post has none,
    /// and that is not a gap — it is a read over what the books produced.
    fn participants(&self) -> Vec<&dyn Participant> {
        Vec::new()
    }

    /// **Its own work in the period** (ARCHITECTURE 4.9b, the second door): accruing, maturing,
    /// deciding, publishing. It reads the stores and PROPOSES; the kernel settles what it proposed
    /// once the phase returns, so settlement stays the one writer of the register (Law 4).
    ///
    /// A system with nothing to do in a period proposes nothing, which is an answer and not a gap.
    fn mechanism(&self) -> Option<&dyn Mechanism> {
        None
    }
}

/// **Every kernel store, declared for what it is.** The register exists to be asked what is still
/// HOMELESS — a fact about the world with no kernel store to live in — and the count is the honest
/// measure of how much ontology is missing. It must fall; it is not a number to tolerate.
fn declared() -> Nouns {
    let mut n = Nouns::new();
    let mut at_home = |name: &str, holds: &str, why: &str| {
        n.declare(NounDecl {
            name: name.to_string(),
            sort: Sort::Noun { home: None },
            holds: holds.to_string(),
            why: why.to_string(),
        });
    };
    at_home("parties", "who exists, named or a cell", "XI-15: a party is a fact about the world");
    at_home("instruments", "every priced thing, with its issuer", "Money A1: no instrument without an issuer");
    at_home("register", "who holds what, with lots and liens", "Register A1: ownership is the world's");
    at_home("prints", "what each book printed, with provenance", "Law 3: a price is a fact somebody cleared");
    at_home("journal", "every event, with its subjects", "Audit A2: what happened is not a module's scratch");
    at_home("wire", "every instruction ever applied", "Money D1: the wire IS the history");
    at_home("agreements", "relations: engagement, mortgage, policy, contract", "XI-10, 17f: a relation is not an instrument and not an event");
    at_home("schedules", "what each instrument owes, and when", "5 D2: a claim with no schedule is one nobody can fall behind on");
    at_home("outlooks", "what each party expects, formed from its own history", "§46: no global expectation; they disagree and it is load-bearing");
    at_home("processes", "what is in flight across periods, with an owner", "XI-3: a process with no end is one nobody has to finish");
    at_home("claims", "who is owed what by a dead party, and at what rank", "XI-8, Appendix B: no liability without beneficiaries, and an estate pays in rank order");

    // **AND WHAT HAS NO HOME.** The count of these is the honest measure of how much ontology is
    // missing, and it must fall. Each names the plan item that gives it one; a noun whose item does
    // not exist is a noun nobody has agreed to build, and the register refuses to let that be silent.
    let mut homeless = |name: &str, item: &str, holds: &str, why: &str| {
        n.declare(NounDecl {
            name: name.to_string(),
            sort: Sort::Noun { home: Some(item.to_string()) },
            holds: holds.to_string(),
            why: why.to_string(),
        });
    };

    // **THERE IS NO REGISTRY** (ARCHITECTURE 4.10: *all data lives in the registry*). `CurrencyCode`,
    // `UnitId` and `RegionId` are bare row ids with nothing behind them, and the party kinds are
    // integer constants with no profile — so four facts the specification states outright are, in
    // this engine, not stated anywhere.
    homeless(
        "registry.currencies",
        "21e",
        "each money and the central bank whose liability it is",
        "Money A2, Currency A2: money is issued by a named issuer, and a CurrencyCode names nobody",
    );
    homeless(
        "registry.places",
        "21e",
        "countries and the regions in them: a country has the money, a region is a place",
        "Seed B3: each party is placed in a region, and the region determines its money — and a RegionId determines nothing",
    );
    homeless(
        "registry.units",
        "21e",
        "each unit of measure and what it is divided into",
        "Law 8: the unit is part of the number, and a UnitId carries no unit",
    );
    homeless(
        "registry.kind_profiles",
        "21e",
        "what varies by party kind, behind a dispatch table",
        "Law 15: kind-varying behaviour lives in a profile, and the kinds here are integers a mechanism could branch on",
    );

    // Three facts a module names, which persist between periods, and which no store keeps.
    homeless(
        "employment.postings",
        "21f",
        "an open position an employer holds, at the wage it offers",
        "XI-10, §39 B: every posting is a bid, and it is something an employer HOLDS — which is what lets it be withdrawn",
    );
    homeless(
        "lending.standards",
        "21f",
        "the standard a lender is currently lending at",
        "Housing C5, Banks Lending: a lender's standard is a decision that persists and that every borrower meets or does not",
    );
    homeless(
        "recipe.work_in_progress",
        "21f",
        "what is between input and output, owned by somebody, carrying what it cost",
        "37 B3: work in progress is a real thing with a holder, not a timing adjustment",
    );
    n
}

/// The stores, one of each, owned by the kernel. **Two engines is Law 4's defect at the largest
/// possible scale**, and two of any store is the same defect one level down — so there is one.
pub struct World {
    pub parties: Parties,
    pub instruments: Instruments,
    pub register: Register,
    pub prints: Prints,
    pub journal: Journal,
    pub wire: Settlement,
    pub params: Params,
    /// XI-10, 17f: the relations. An engagement, a mortgage, a policy, a supply contract.
    pub agreements: Agreements,
    /// 5 D2: what each instrument owes and when.
    pub schedules: Schedules,
    /// §46: what each deciding party expects, formed from its own history. They disagree.
    pub outlooks: Outlooks,
    /// Whatever is in flight across periods with an owner and an end.
    pub processes: Processes,
    /// XI-8: who is owed what by a party whose life has ended, and at what rank.
    pub claims: Claims,
    /// **The ontology register.** Every store declares itself, and its count of HOMELESS nouns is
    /// the honest measure of how much ontology is missing. It was written and wired to nothing, so
    /// the count was zero by never having been asked (21d.1).
    pub nouns: Nouns,
    pub phases: Phases,
    pub books: Vec<BookDecl>,
    pub period: u32,
    pub settled_kind: u32,
    pub failed_kind: u32,
}

/// What one period did. Printed rather than asserted (`check:opens`): the census is a read, and a
/// number nobody looks at is not a check.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stepped {
    pub asks: usize,
    pub narrows: usize,
    pub books_cleared: usize,
    pub trades: usize,
    pub events: usize,
    /// How many systems' phases actually ran. A world where this is zero is a world of declarations.
    pub ran: usize,
}

impl World {
    /// The world before anything has happened to it. **Every store declares itself here**, so the
    /// ontology register's count of what is still homeless is true rather than zero-by-omission.
    pub fn empty() -> World {
        let mut journal = Journal::new();
        let settled_kind = journal.kinds.declare("instruction.settled");
        let failed_kind = journal.kinds.declare("instruction.failed");
        World {
            parties: Parties::new(),
            instruments: Instruments::new(),
            register: Register::new(),
            prints: Prints::new(),
            journal,
            wire: Settlement::new(),
            params: Params::new(100.0, 60.0),
            agreements: Agreements::new(),
            schedules: Schedules::new(),
            outlooks: Outlooks::new(),
            processes: Processes::new(),
            claims: Claims::new(),
            nouns: declared(),
            phases: Phases::new(),
            books: Vec::new(),
            period: 0,
            settled_kind,
            failed_kind,
        }
    }

    /// Law 10: every system's phases, in one order, sealed — **a phase that reads a not-yet-produced
    /// print throws** rather than reading a stale one.
    pub fn wire_up(&mut self, systems: &[&dyn System]) {
        for s in systems {
            for phase in s.phases() {
                self.phases.add(phase);
            }
        }
        self.phases.seal();
    }

    /// One period: **the phases run in their declared order, the books run at the markets moment,
    /// and what each proposed is settled.** Law 19: the count is read off what happened, never kept
    /// beside it.
    ///
    /// **The phase pass is what makes a wired module a running one.** Before it, `wire_up` collected
    /// every declaration, sealed the order and the loop then ran books only — so a system without a
    /// participant did nothing at all, and the ordering Law 10 is about was validated and ignored.
    pub fn step(&mut self, systems: &[&dyn System]) -> Stepped {
        self.period += 1;
        let participants: Vec<&dyn Participant> =
            systems.iter().flat_map(|s| s.participants()).collect();
        let mut out = Stepped::default();
        let events_before = self.journal.len();

        // Law 10: in order, and the markets moment is where the books run. A phase anchored before
        // markets sees the world the last period left; one anchored after sees this period's prints.
        let order: Vec<(u32, bool)> = self
            .phases
            .order()
            .iter()
            .map(|p| (p.owner, matches!(p.anchor, Anchor::After(MARKETS) | Anchor::Before(REVALUATION))))
            .collect();
        let by_slot = slots(systems);

        for (owner, after_markets) in order.iter().filter(|(_, after)| !after) {
            out.ran += self.run_phase(*owner, &by_slot, systems);
            let _ = after_markets;
        }
        out.trades += self.run_books(&participants, &mut out);
        for (owner, _) in order.iter().filter(|(_, after)| *after) {
            out.ran += self.run_phase(*owner, &by_slot, systems);
        }

        out.events = self.journal.len() - events_before;
        out
    }

    /// One system's phase: its mechanism reads the stores, proposes, and the kernel settles.
    fn run_phase(&mut self, owner: u32, by_slot: &[usize], systems: &[&dyn System]) -> usize {
        let at = match by_slot.get(owner as usize) {
            Some(at) if *at < systems.len() => *at,
            // The three kernel moments own slots nothing declares a mechanism for.
            _ => return 0,
        };
        let m = match systems[at].mechanism() {
            Some(m) => m,
            None => return 0,
        };
        let mut ctx = MechanismContext::of(
            self.period,
            Reads {
                claims: &self.claims,
                parties: &self.parties,
                instruments: &self.instruments,
                register: &self.register,
                prints: &self.prints,
                journal: &self.journal,
                params: &self.params,
                agreements: &self.agreements,
                schedules: &self.schedules,
                outlooks: &self.outlooks,
                processes: &self.processes,
                wire: &self.wire,
            },
        );
        m.run(&mut ctx);
        let asked = ctx.taken();

        // **Settlement is the one writer of the register** (Law 4). What a phase asked for happens
        // here or not at all, and a refusal leaves the world as it was — a module cannot move units
        // by wanting to.
        for p in asked.proposed {
            let instruction = Instruction { legs: &p.legs, cause: p.cause, delivery: p.delivery };
            self.wire.settle(
                &instruction,
                self.period,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &self.instruments,
                },
                self.settled_kind,
                self.failed_kind,
            );
        }
        // Audit A2: and what it said happened, for whoever it happened to.
        for s in asked.said {
            self.journal.say(self.period, 0, s.kind, &s.subjects, &s.data, s.public);
        }
        // §46: the outlooks it formed from its parties' own histories.
        for (who, about, level) in asked.formed {
            self.outlooks.form(who, about, level, self.period);
        }
        // A-20: and what it paid off a schedule. A payment that failed leaves the arrear standing,
        // because the mark is the module's claim and the wire above is what makes it true or not.
        for due in asked.settled {
            self.schedules.settle(due);
        }
        // XI-3: and whose life ended. `Parties` is the one writer of who is alive; a module asks.
        for who in asked.ceased {
            self.parties.cease(who);
        }
        // XI-8: and who is owed what by an estate. A claim QUEUES where its rank puts it; a payment
        // would be the claimant jumping ahead of the creditors the estate exists to pay (21c).
        for (on, holder, owed, ranks) in asked.claimed {
            self.claims.against(on, holder, owed, ranks);
        }
        1
    }

    /// The markets moment: every declared book, asked once.
    fn run_books(&mut self, participants: &[&dyn Participant], out: &mut Stepped) -> usize {
        if participants.is_empty() || self.books.is_empty() {
            return 0;
        }
        let books = Books::index(
            participants,
            &Shown {
                parties: &self.parties,
                instruments: &self.instruments,
                register: &self.register,
                prints: &self.prints,
                journal: &self.journal,
                params: &self.params,
                agreements: &self.agreements,
            },
            self.period,
        );
        out.narrows += books.narrows;
        let mut traded = 0usize;
        for book in &self.books {
            let mut stores = Stores {
                parties: &self.parties,
                instruments: &self.instruments,
                register: &mut self.register,
                prints: &mut self.prints,
                journal: &mut self.journal,
                wire: &mut self.wire,
                params: &self.params,
                agreements: &self.agreements,
            };
            let session = run_book(
                book,
                participants,
                &books,
                &mut stores,
                self.period,
                self.settled_kind,
                self.failed_kind,
            );
            out.asks += session.asks;
            traded += session.settled;
            // Clearing C4: a book that had something cross is one that cleared; one that did not
            // prints nothing, and counting it would be a session that never happened.
            if matches!(session.outcome, crate::clearing::Outcome::Cleared { .. }) {
                out.books_cleared += 1;
            }
        }
        traded
    }

    /// **Money D2, 21.12: A PARTY IS ADMITTED TO A WORLD, AND ITS BANK HAS TO ISSUE MONEY.**
    ///
    /// `Parties::add` cannot ask: it is the one writer of who exists and it has no instruments to
    /// look at. So a party banked at a bank that issues nothing was admitted silently, held no money
    /// it could pay with, and **found out at its first payment** — where `across` panics that the
    /// payment has nowhere to land. 21.12 and `WK13` are the same finding: the throw is in the right
    /// place for the payment and two hundred periods too late for the party.
    ///
    /// A world is where both are known, so this is where it is asked. A party that banks NOWHERE is
    /// admitted and is not an error: that is what a central bank does, and it issues its own.
    pub fn admit(
        &mut self,
        kind: u32,
        region: crate::ids::RegionId,
        bank: PartyId,
        representation: crate::parties::Representation,
        weight: u32,
        key: u32,
    ) -> PartyId {
        assert!(
            !bank.some() || self.instruments.money_issued_by(bank).is_some(),
            "Money D2: party of kind {kind} banks at {}, which issues no money — it would hold nothing it could pay with",
            bank.0
        );
        self.parties.add(kind, region, bank, representation, weight, key)
    }

    /// A book, declared. The subject is what it delivers; its money is a CURRENCY, and each side pays
    /// out of its own account (22b.9a).
    pub fn open_book(&mut self, market: MarketId, subject: InstrumentId, ccy: CurrencyCode, rule: PriceRule) {
        // 22b.9a: a book names a CURRENCY and each side pays out of its own account, so there is no
        // cash line to check the class of. What it does have to be is a thing somebody can deliver:
        // a book whose subject is money is a book for swapping a deposit for itself.
        assert!(
            self.instruments.class_of(subject) != Class::Money,
            "Clearing B1: a book's subject is what it delivers, and money is not delivered in a book"
        );
        self.books.push(BookDecl { market, subject, ccy, rule });
    }
}

/// Law 10: which system owns each declaration slot, so a phase in the order can be traced back to
/// the system that declared it. The slot IS the index plus the three kernel moments, and this reads
/// it rather than keeping a second map beside it (Law 19).
fn slots(systems: &[&dyn System]) -> Vec<usize> {
    let mut by_slot = Vec::new();
    for (at, s) in systems.iter().enumerate() {
        for p in s.phases() {
            while by_slot.len() <= p.owner as usize {
                by_slot.push(usize::MAX);
            }
            by_slot[p.owner as usize] = at;
        }
    }
    by_slot
}

/// A phase, declared without ceremony: most systems have one and it anchors to a moment.
pub fn phase(name: u32, owner: u32, anchor: Anchor) -> PhaseDecl {
    PhaseDecl { name, owner, anchor, reads: Vec::new(), writes: Vec::new() }
}

/// The moments, re-exported so a system says where it runs without importing the world.
pub const AT_CORPORATE_ACTIONS: u32 = CORPORATE_ACTIONS;
pub const AT_MARKETS: u32 = MARKETS;
pub const AT_REVALUATION: u32 = REVALUATION;

/// What a party will pay or take for what it already holds, read from its own book. Every
/// participant below is built out of this: **a schedule comes from the party's own state** (Clearing
/// B2), and there is no door here onto anybody else's.
pub fn holds_of(view: &ParticipantView<'_>, subject: InstrumentId) -> f64 {
    view.free(subject)
}

/// Its own money, for what it can actually fund — Law 6: a bid it cannot pay for is not a bid.
pub fn can_fund(view: &ParticipantView<'_>, cash: InstrumentId) -> f64 {
    view.free(cash)
}

/// A seller's order, from what it holds and what it will take. `None` where it holds nothing: a
/// schedule over no units is not a schedule.
pub fn offer(view: &ParticipantView<'_>, subject: InstrumentId, at_least: f64) -> Option<Order> {
    let units = holds_of(view, subject);
    if units <= 0.0 {
        return None;
    }
    Some(Order { party: view.self_id(), side: Side::Sell, price: Some(at_least), qty: units as i64 })
}

/// A buyer's order, from what it can fund and what it will pay. `None` where it can fund nothing.
pub fn bid(view: &ParticipantView<'_>, cash: InstrumentId, at_most: f64) -> Option<Order> {
    if at_most <= 0.0 {
        return None;
    }
    let money = can_fund(view, cash);
    let affordable = (money / at_most) as i64;
    if affordable <= 0 {
        return None;
    }
    Some(Order { party: view.self_id(), side: Side::Buy, price: Some(at_most), qty: affordable })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{PartyId, UnitId};
    use crate::ledger::{Cause, Leg};
    use crate::parties::Representation;

    struct Quiet;
    impl System for Quiet {
        fn name(&self) -> &'static str {
            "quiet"
        }
    }

    #[test]
    fn a_system_with_no_reason_to_post_has_no_participants_and_that_is_not_a_gap() {
        // Clearing B2, Appendix B: giving a read a schedule would be inventing demand nobody has.
        let q = Quiet;
        assert!(q.participants().is_empty());
        assert!(q.phases().is_empty());
    }

    #[test]
    fn a_world_with_no_books_steps_and_does_nothing() {
        // The honest empty case: the period advances and nothing trades, because there is nothing to
        // trade in. It does not throw and it does not invent a session.
        let mut w = World::empty();
        let q = Quiet;
        let systems: Vec<&dyn System> = vec![&q];
        w.wire_up(&systems);
        let did = w.step(&systems);
        assert_eq!(w.period, 1);
        assert_eq!(did, Stepped::default());
    }

    #[test]
    fn a_book_delivers_something_that_is_not_money() {
        // 22b.9a: a book names a CURRENCY and each side pays out of its own account, so there is no
        // cash line to check. What it must have is something to DELIVER.
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let share = w.instruments.issue(cb, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.open_book(MarketId::at(0), share, CurrencyCode::at(0), PriceRule::SellersCompete);
        assert_eq!(w.books.len(), 1);
    }

    #[test]
    #[should_panic(expected = "money is not delivered in a book")]
    fn a_book_that_delivers_money_is_refused() {
        // Swapping a deposit for a deposit at a cleared price is a book for swapping money for
        // itself. The FX pair that looks like this is a pair, and it is declared as one.
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let cash = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        w.open_book(MarketId::at(0), cash, CurrencyCode::at(0), PriceRule::SellersCompete);
    }

    /// A system whose phase mints a little of its own money and pays it away — the smallest thing a
    /// mechanism can do that touches the register through the wire.
    struct Pays {
        who: PartyId,
        to: PartyId,
        money: InstrumentId,
    }

    impl Mechanism for Pays {
        fn run(&self, ctx: &mut MechanismContext<'_>) {
            ctx.propose(
                vec![Leg::Mint { issuer: self.who, ccy: CurrencyCode::at(0), money: self.money, amount: 10.0 }],
                Cause::Payment,
                crate::ledger::Delivery::Nothing,
                "it created its own money",
            );
            ctx.propose(
                vec![Leg::Money {
                    from: self.who,
                    to: self.to,
                    ccy: CurrencyCode::at(0),
                    instrument: self.money,
                    amount: 4.0,
                    receipt: crate::ledger::Receipt::Transfer,
                }],
                Cause::Payment,
                crate::ledger::Delivery::Nothing,
                "and paid some of it away",
            );
        }
    }

    struct Runs {
        pays: Pays,
    }

    impl System for Runs {
        fn name(&self) -> &'static str {
            "runs"
        }
        fn phases(&self) -> Vec<PhaseDecl> {
            vec![phase(3, 3, Anchor::Before(MARKETS))]
        }
        fn mechanism(&self) -> Option<&dyn Mechanism> {
            Some(&self.pays)
        }
    }

    #[test]
    fn a_wired_system_actually_runs_and_what_it_proposes_is_settled() {
        // ARCHITECTURE 4.9b: the second door. Before the phase pass, `wire_up` collected every
        // declaration and sealed the order and `step` ran books only — so a system without a
        // participant did NOTHING, and fifty of them were listed as wired. This is what makes a
        // wired module a running one.
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let them = w.parties.add(kinds::BANK, crate::ids::RegionId::at(0), cb, Representation::Named, 1, 0);
        let money = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let runs = Runs { pays: Pays { who: cb, to: them, money } };
        let systems: Vec<&dyn System> = vec![&runs];
        w.wire_up(&systems);

        let did = w.step(&systems);
        assert_eq!(did.ran, 1, "the phase ran");
        // Law 4: the module wrote nothing — settlement did, and the register says so.
        assert_eq!(w.register.quantity(w.register.row(them, money)), 4.0);
        assert_eq!(w.register.quantity(w.register.row(cb, money)), 6.0);
    }

    #[test]
    fn a_system_with_no_mechanism_runs_nothing_and_that_is_an_answer() {
        // A read over what the books produced is not a phase that does nothing wrong — it is a
        // system with nothing of its own to do, and the count says so rather than hiding it.
        let mut w = World::empty();
        let q = Quiet;
        let systems: Vec<&dyn System> = vec![&q];
        w.wire_up(&systems);
        assert_eq!(w.step(&systems).ran, 0);
    }

    #[test]
    fn the_ontology_register_names_what_is_still_homeless_rather_than_reporting_zero() {
        // 21d.1b: the register was wired at 21d and every one of the kernel's own stores declared
        // itself — so the count was zero, and zero because nothing had declared a noun it had no
        // home for. A measure that can only report "nothing missing" is not a measure.
        let w = World::empty();
        let homeless = w.nouns.homeless();
        assert!(!homeless.is_empty(), "a count of zero here is the measure switched off");

        // **There is no registry** (ARCHITECTURE 4.10), and that is four of them: a CurrencyCode
        // names no issuer, a RegionId determines no money, a UnitId carries no unit, and a party
        // kind is an integer with no profile.
        let named: Vec<&str> = homeless.iter().map(|(n, _)| *n).collect();
        assert!(named.contains(&"registry.currencies"));
        assert!(named.contains(&"registry.places"));
        assert!(named.contains(&"registry.units"));
        assert!(named.contains(&"registry.kind_profiles"));

        // And every one names the item that gives it a home. A noun whose item nobody has written
        // is a noun nobody has agreed to build.
        for (name, item) in &homeless {
            assert!(!item.is_empty(), "{name} names no item");
        }
    }

    #[test]
    fn a_store_the_kernel_owns_is_not_homeless_and_a_fact_with_nowhere_to_live_is() {
        // The distinction the register exists to hold. `agreements` had no home before 21d and has
        // one now; `employment.postings` is the same shape of fact and still has none.
        let w = World::empty();
        let named: Vec<&str> = w.nouns.homeless().iter().map(|(n, _)| *n).collect();
        assert!(!named.contains(&"agreements"), "it got a home at 21d");
        assert!(!named.contains(&"claims"), "it got one at 21c");
        assert!(named.contains(&"employment.postings"), "an employer holds it and nothing keeps it");
    }

    #[test]
    #[should_panic(expected = "issues no money")]
    fn a_party_admitted_at_a_bank_that_issues_nothing_is_refused_at_entry() {
        // Money D2, 21.12: it was admitted silently, held nothing it could pay with, and found out
        // at its FIRST PAYMENT, where `across` panics that the payment has nowhere to land. The
        // throw was in the right place for the payment and two hundred periods too late for the
        // party. `Parties::add` cannot ask — it has no instruments to look at — so a WORLD does.
        let mut w = World::empty();
        let not_a_bank = w.parties.add(kinds::FIRM, crate::ids::RegionId::at(0), PartyId::NONE, crate::parties::Representation::Named, 1, 0);
        w.admit(kinds::HOUSEHOLD, crate::ids::RegionId::at(0), not_a_bank, crate::parties::Representation::Named, 1, 0);
    }

    #[test]
    fn a_party_that_banks_nowhere_is_admitted_because_that_is_what_a_central_bank_does() {
        // The absence is not the error: a central bank banks nowhere and issues its own money.
        let mut w = World::empty();
        let cb = w.admit(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, crate::parties::Representation::Named, 1, 0);
        assert!(w.parties.alive(cb));

        // And once it issues one, a party may bank at it.
        w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, crate::ids::UnitId::at(0), None, None);
        let t = w.admit(kinds::TREASURY, crate::ids::RegionId::at(0), cb, crate::parties::Representation::Named, 1, 0);
        assert!(w.parties.alive(t));
    }
}
