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

use crate::clearing::{Order, Side};
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::instruments::{Class, Instruments};
use crate::journal::Journal;
use crate::ledger::{Instruction, Settlement, Settling};
use crate::module::{Mechanism, MechanismContext, Participant, ParticipantView, Stores as Reads};
use crate::params::Params;
use crate::parties::Parties;
use crate::nouns::{NounDecl, Nouns, Sort};
use crate::prices::Prints;
use crate::stores::{Agreements, Claims, InProgress, Outlooks, Processes, Schedules, Standing};
use crate::register::Register;
use crate::registry::{Banks, Registry};
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
    /// **37 C3, 22c.4: SOMEBODY WHOSE BUSINESS IS TO HOLD THE STOCK.** No party's business was, so a
    /// good went from the firm that made it straight to the household that ate it, in one week, or it
    /// sat on the maker's own shelf and perished. The missing intermediary of Law 1, and what makes a
    /// consumer price index possible at all (21.84).
    pub const STOCKIST: u32 = 11;
    pub const ALL: [u32; 12] = [
        HOUSEHOLD, FIRM, BANK, FUND, INSURER, DEALER, TREASURY, CENTRAL_BANK, CARRIER, SMALL_FIRM,
        ASSESSOR, STOCKIST,
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

    /// **Audit C3, 22e: what this system contributes to the audit.** A family's independence is
    /// about the SOURCE it reads, not about who wrote it — so a module states its own check and the
    /// kernel runs it on the one traversal, beside the kernel's own and beside every other module's.
    ///
    /// A system with none contributes none, and that is not the same as a family being green: the
    /// families nobody contributed to are declared NOT BUILT at assembly (`Audit::over`).
    fn audits(&self) -> Vec<Box<dyn crate::audit::Contribution>> {
        Vec::new()
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
    // 21e: the four the registry now holds. Each was a bare row id naming nothing, and the count of
    // homeless nouns falls by four because they are here rather than because nobody asked.
    at_home("registry.currencies", "each money and the party whose liability it is", "Money A2: money is somebody's liability, and a CurrencyCode named nobody");
    at_home("registry.places", "countries and the regions in them", "Seed B3, 13c.1: a country has the money and a region is a place, so currency_of(region) reads through the country");
    at_home("registry.units", "each unit and what one of it is divided into", "Law 8: the unit is part of the number, and one grid for everything made a dwelling divisible");
    at_home("registry.kind_profiles", "what varies by party kind, behind a dispatch the kernel reads", "Law 15: a world whose kinds have no profiles has nowhere to put what varies, so the pressure to branch never goes away");
    // 21f: the three a module named and no store kept. Two were one shape — terms a party stands
    // behind, one-sided, which is what an agreement is not (Law 5).
    at_home("standing", "terms a party stands behind: a posting, a lending standard", "XI-10, Housing C5: a posting is HELD by an employer, which is what lets it be withdrawn; a standard is a decision that persists and that a borrower meets or does not");
    at_home("making", "what is between input and output, owned, carrying what it cost", "37 B3: work in progress is a real thing with a holder, not a timing adjustment");
    // 21.112: it is the JOURNAL's, because a realised gain is an event rather than a thing anybody
    // holds — it happens at the moment the units leave, to a named party, for an amount.
    at_home("registry.indices", "each index, the country whose it is, and the lines it is built from with a COUNT of each", "Indices D1, 22 D5: an index is a COUNTRY's and it is ONE system; the level is never stored, it is computed from the constituents when asked");
    at_home("reporting.accounts", "the accounts a party has PUBLISHED, as at a date — an EVENT in the journal, on a day, to a named company, public", "48 A1, 22i.1: a covenant is tested against published accounts and a bid is formed from them, and `journal.of_kind(accounts.published)` is that read. What was missing was never a store — it was a mechanism");
    at_home("settlement.realised", "what each disposal realised against the basis its lots carried", "Law 19: settlement is the only place that holds the price and the basis at once, so anywhere else would re-derive one of them");
    // 22d.1: the wire is what HAPPENED; this is what is still trying to. It is settlement's because
    // settlement is what decides an instruction cannot go through, and a queue written anywhere else
    // would be a second writer of that decision (Law 4).
    at_home("settlement.queue", "payments that could not be made yet, with the day each is late on", "XI-9, 22d: a gridlock is a timing failure and not a default, and this world turned every one of them into an arrear the instant it was tried");

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

    // **AND WHAT STILL HAS NO HOME.** The registry's four went home at 21e and the three a module
    // kept went home at 21f — and a count of zero would be the measure switched off again (21d.1b),
    // because zero here would mean "nothing anybody has DECLARED is homeless" rather than "nothing is
    // missing". What is left was found by the re-read of item 21 and names the item that will
    // give it a home. The count must fall and must never rise; it falls by being built, not by
    // nobody asking.
    // 22i.1: **and it went home, to the JOURNAL.** A set of published accounts is an EVENT — it
    // happens on a day, to a named company, and it is public — so it belongs where every other public
    // fact about this world belongs, and a separate store for it would be a second history (Law 4).
    // `Publishes` writes it; `journal.of_kind("accounts.published")` is the read a covenant and a bid
    // take. What was missing was never a store, it was a mechanism.
    // **AND WHAT THE 22i RE-READ FOUND, declared as it was found.** `reporting.accounts` went home to
    // the journal at 22i.1, and a count of zero would be the measure switched off — zero means
    // nothing anybody DECLARED is homeless, not that nothing is missing. Each of these is a fact a
    // built mechanism produces and no store keeps.
    homeless(
        "ratings.grades",
        "22i.2",
        "the grade an assessor currently holds on an issuer, and what it was before",
        "21 A4, A6: `ratings::reassess` says when a HELD grade moves, and nothing holds one — so two houses cannot disagree about a name and no grade can be shown to have been wrong",
    );
    homeless(
        "reporting.estimates",
        "22i.3",
        "what each bank expects a named company to report, and the surprise when it does",
        "48 C1, F1: `reporting::estimate` and `settle` are built over a slice of estimates nobody keeps, so the consensus is computed from a list that exists for one call and the surprise has nothing to settle against",
    );
    homeless(
        "agreements.states",
        "21.62",
        "whether a relation is live, breached, cured, discharged or terminated",
        "17.7: `Agreements` is live or ended, so this world records neither a breach nor a cure — and a payer given time on an arrear has nowhere to land",
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
    /// ARCHITECTURE 4.10, 21e: **what the ids point at** — each money's issuer, each country's money
    /// and each region's country, each unit's subdivision, and a profile per party kind. They were
    /// bare row numbers naming nothing, which is four of the ontology register's homeless nouns.
    pub registry: Registry,
    /// 21f: terms a named party stands behind until it withdraws them — a posting, a lending
    /// standard. The one-sided twin of `agreements`, which always has two sides (Law 5).
    pub standing: Standing,
    /// §37 B3, 21f: what is between input and output, owned by somebody, carrying what it cost.
    pub making: InProgress,
    /// **The ontology register.** Every store declares itself, and its count of HOMELESS nouns is
    /// the honest measure of how much ontology is missing. It was written and wired to nothing, so
    /// the count was zero by never having been asked (21d.1).
    pub nouns: Nouns,
    pub phases: Phases,
    /// Audit C1, 22e: **the families, assembled at `wire_up` and run every period.** It was built,
    /// tested and never reached by the loop.
    pub audit: crate::audit::Audit,
    /// 3 C2, 22c.2: the standing book — orders that rest between sessions.
    pub resting: crate::stores::Resting,
    pub books: Vec<BookDecl>,
    /// **G3, 22c2.2: THE ONE CALENDAR.** The assembled world counted periods and had no calendar in
    /// it at all, so nothing it held could be placed by date — which is why every order in every
    /// book rested for ever. A periodicity is placed by date (G3.a) and this is where the dates
    /// come from.
    pub calendar: crate::calendar::Calendar,
    pub period: u32,
    /// 22d.1: the journal kinds an instruction's outcome is said under — settled, failed, queued,
    /// and what a disposal realised. They were four loose fields handed to `settle` three at a time.
    pub says: crate::ledger::Outcomes,
}

/// What one period did. Printed rather than asserted (`check:opens`): the census is a read, and a
/// number nobody looks at is not a check.
#[derive(Debug, Default)]
pub struct Stepped {
    /// **Audit C1, A2, E2, 22e.1: what the audit found in this period, by family.** It is carried
    /// here rather than printed and dropped, because a run that breaks an identity has to be able to
    /// say so in the line it prints — and because a caller that wants to know whether the world it
    /// just stepped is sound should not have to run the audit a second time (Law 19).
    ///
    /// **An unbuilt family is in this list too**, reporting `built: false` and no violations. A world
    /// that assembled no family must not read as a world with no violations; that lie is what
    /// `Audit::report_card` refuses to let a reader tell by counting.
    pub audit: Vec<crate::audit::Report>,
    pub asks: usize,
    pub narrows: usize,
    pub books_cleared: usize,
    pub trades: usize,
    pub events: usize,
    /// How many systems' phases actually ran. A world where this is zero is a world of declarations.
    pub ran: usize,
    /// **22d.2: how many queued payments the gridlock pass settled**, in cycles nobody in them
    /// could have paid alone. Every one is a default this world would otherwise have invented.
    pub unwound: usize,
    /// **22d.3: what became of THIS period's short payments** — still waiting, went through after
    /// waiting, ran out of days. A read, published, causing nothing (`ledger::Queue::between`). The
    /// middle number is the measure: every one of those was an arrear before the queue existed.
    pub queue: (usize, usize, usize),
}

impl World {
    /// The world before anything has happened to it. **Every store declares itself here**, so the
    /// ontology register's count of what is still homeless is true rather than zero-by-omission.
    pub fn empty() -> World {
        let mut journal = Journal::new();
        // Settled, failed, QUEUED (22d.1 — a payment waiting for the money to arrive, which is
        // neither of the other two) and what a disposal realised (21.112).
        let says = crate::ledger::Outcomes::declared(&mut journal);
        World {
            parties: Parties::new(),
            instruments: Instruments::new(),
            register: Register::new(),
            prints: Prints::new(),
            journal,
            // **22d.1: how many days a payment may wait here before it is late.** A TECHNOLOGY of
            // the payment system, stated where the system is built, in days — six, so a payment
            // tried on the first day of a period is late when the next period opens and one tried
            // late in a period still has the days it has left. Nothing about it is a choice any
            // party makes.
            wire: Settlement::new(6),
            params: Params::new(100.0, 60.0),
            agreements: Agreements::new(),
            schedules: Schedules::new(),
            outlooks: Outlooks::new(),
            processes: Processes::new(),
            claims: Claims::new(),
            registry: Registry::new(),
            standing: Standing::new(),
            making: InProgress::new(),
            nouns: declared(),
            audit: crate::audit::Audit::new(),
            resting: crate::stores::Resting::new(),
            phases: Phases::new(),
            books: Vec::new(),
            // G3: the period is 7 days with 3 settlement cycles in it, and the world opened on
            // 2000-01-01 (`calendar::Day`'s epoch). A RESOLUTION, tested by invariance.
            calendar: crate::calendar::Calendar::new(crate::calendar::Day(0), 7, 3),
            period: 0,
            says,
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
        // **Audit C1, C3, E2, 22e: the audit is assembled here, with the phases.** The kernel's own
        // checks, plus whatever each module contributes, plus NOT BUILT for every family nobody
        // contributed to — because a family that is missing has to SAY it is missing.
        let mut contributions: Vec<Box<dyn crate::audit::Contribution>> = vec![
            Box::<crate::audit::LotsAgainstQuantity>::default(),
            Box::<crate::audit::ATotalCarriesNoLots>::default(),
            Box::<crate::audit::NoCollateralCountedTwice>::default(),
        ];
        for s in systems {
            contributions.extend(s.audits());
        }
        self.audit = crate::audit::Audit::over(contributions);
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
        // 22i.1: and the parties store knows what period it is, so a party entering in it is
        // stamped with it. A party does not choose when it was born.
        self.parties.opened(self.period);
        let participants: Vec<&dyn Participant> =
            systems.iter().flat_map(|s| s.participants()).collect();
        let mut out = Stepped::default();
        let events_before = self.journal.len();

        // **3 C2, G3.a, 22c2.2: the calendar expires what stood to yesterday, before anything reads
        // a book.** The period opens with the orders that are still good and no others; an order
        // whose day has passed is not one somebody has to be asked about.
        let today = self.calendar.start_of(crate::calendar::Period(self.period));
        self.resting.expire(today);

        // **XI-9, 22d.1: and the payments that ran out of days.** A queued payment is not an arrear
        // while it still has days to wait; when it has none left it becomes the failure this world
        // used to record the instant the payer was short. The arrear is recorded on the wire, where
        // every other outcome is.
        // 22d.3 reads how many, off the rows, with the rest of what became of them.
        self.wire.give_up(
            today,
            self.period,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &self.instruments,
                calendar: &self.calendar,
                says: self.says,
            },
        );

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

        // **XI-9, 22d.2: and the gridlock pass, once, with every payment of the period in.** The
        // retry unwinds a chain as the money arrives; a CYCLE has no outside to arrive from, and
        // this is the one moment the whole queue is visible at once. It runs before the audit
        // because a cycle it settles is not a violation of anything.
        out.unwound = self.wire.unwind(
            self.period,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &self.instruments,
                calendar: &self.calendar,
                says: self.says,
            },
        );

        // 22d.3: and what became of the payments that were short in it. Read off the rows at the
        // end, when the retries and the gridlock pass have both had their turn.
        // The LAST day of this period, not the first of the next: a payment that finishes on the
        // boundary belongs to one of them and not to both.
        let closes = crate::calendar::Day(self.calendar.start_of(crate::calendar::Period(self.period + 1)).0 - 1);
        out.queue = self.wire.queue.between(today, closes);

        out.events = self.journal.len() - events_before;
        // **Audit C1, 22e.1: EVERY FAMILY, EVERY PERIOD, over the one traversal it was built for.**
        // `audit.rs` and its families were built and tested and `grep audit assembly.rs` returned one
        // hit, in a comment — so the assembled world had stepped in every run since the port with no
        // family ever visiting it, which is 21d's defect one register up.
        //
        // It runs LAST, over what the period actually left behind. It never repairs: what it finds is
        // carried out for whoever is reading, and a violation is a finding about a mechanism rather
        // than a licence to adjust the number.
        out.audit = self.audit.run(&self.register, &self.wire, self.period);
        out
    }

    /// One system's phase: its mechanism reads the stores, proposes, and the kernel settles.
    /// **XI-15: an event applying to SOME members makes them a new cell with the same state, and the
    /// division is EXACT.** A cell is homogeneous — every member holds the cell's holdings divided by
    /// its weight — so the departing members' share is `taking / had` of every row, with no
    /// remainder and no rounding to choose. That is why the arithmetic is here and not in a module:
    /// the split is the one place a cell could quietly become an average, and it has one writer.
    ///
    /// What they hold moves over the ORDINARY WIRE, as legs, because a split is a movement of units
    /// between two named parties like any other (Law 5). Their outlook goes with them: one group has
    /// one history, so a cell split off its parent does not start out expecting nothing (21.20).
    fn split_cell(&mut self, parent: PartyId, taking: u32, carrying: crate::stores::AgreementId) {
        let had = self.parties.weight(parent);
        let share = f64::from(taking) / f64::from(had);
        let child = self.parties.split(parent, taking);

        let mut legs: Vec<crate::ledger::Leg> = Vec::new();
        for row in self.register.of_holder(parent) {
            let row = crate::ids::HoldingId(*row);
            let line = self.register.instrument_of(row);
            // Register C3: what is pledged does not move, so the members take their share of what is
            // free. A lien is a claim against the party that gave it, and it stays with that party.
            let free = self.register.free(row);
            if free <= 0.0 {
                continue;
            }
            let theirs = free * share;
            legs.push(if self.instruments.class_of(line) == Class::Money {
                crate::ledger::Leg::Money {
                    from: parent,
                    to: child,
                    ccy: self.instruments.ccy_of(line),
                    instrument: line,
                    amount: theirs,
                    receipt: crate::ledger::Receipt::Transfer,
                }
            } else {
                crate::ledger::Leg::Asset {
                    from: parent,
                    to: child,
                    instrument: line,
                    qty: theirs,
                    // Law 19: no price. The units did not change hands at one — they are the same
                    // members' holdings, carried at what they cost, and a price here would print a
                    // realised gain on a group that sold nothing (21.112).
                    price_per_unit: None,
                }
            });
        }
        if !legs.is_empty() {
            let instruction = Instruction {
                legs: &legs,
                cause: crate::ledger::Cause::CorporateAction,
                delivery: crate::ledger::Delivery::Free,
            };
            self.wire.settle(
                &instruction,
                self.period,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                },
            );
        }

        // 21.20: one group, one history. The child's outlook is the parent's, because its members
        // lived the parent's history — an empty book would say these people have seen nothing.
        for row in self.outlooks.of_party(parent).to_vec() {
            let about = self.outlooks.about_at(row);
            let level = self.outlooks.level_at(row);
            self.outlooks.form(child, about, level, self.period);
        }

        // Labour A4.c: and the relationship that applies to them goes with them.
        self.agreements.moves(carrying, parent, child);
    }

    /// **21j.1a, 21.139: AN OBLIGATION COMES INTO EXISTENCE.** The line is issued, the issuer holds
    /// what it brought, a book opens if the paper is traded, and what it owes is written down. One
    /// act, four stores, one writer of each (Law 4) — and the whole of what *bringing paper* means.
    ///
    /// The units come onto the issuer's own book at NO COST. It did not buy them: a promise is not a
    /// thing you pay for, and what the issuer OWES is what others come to hold of it (5 A4), which
    /// starts the moment it sells one. Its own paper on its own book nets to nothing, which is
    /// exactly what *issued and outstanding* means.
    fn brought(&mut self, what: crate::module::Brings) {
        let line = self.instruments.issue(
            what.issuer,
            what.ccy,
            what.class,
            what.unit,
            what.coupon,
            what.matures,
        );
        // Settlement is the one writer of the register, so the units arrive over the wire like
        // everything else — one-sided, because nobody is on the other end of a promise being made,
        // which is the same shape as a harvest (`Leg::Create`).
        if what.units > 0.0 {
            let legs = [crate::ledger::Leg::Create {
                party: what.issuer,
                instrument: line,
                qty: what.units,
                cost_per_unit: 0.0,
            }];
            let instruction = Instruction {
                legs: &legs,
                cause: crate::ledger::Cause::CorporateAction,
                delivery: crate::ledger::Delivery::Nothing,
            };
            self.wire.settle(
                &instruction,
                self.period,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                },
            );
        }
        // Clearing B1: a book for it, if it is paper anybody else may bid for. A loan row is the
        // lender's and nobody bids for it, which is an answer rather than a missing book.
        if let Some(venue) = what.book {
            self.open_book(crate::systems::book_of(line), line, what.ccy, venue);
        }
        // 5 D2: and what it owes, by date. A claim with terms and no schedule is a claim nobody can
        // fall behind on.
        for (due, amount, of) in what.owing {
            self.schedules.owes(line, what.issuer, due, amount, of);
        }
    }

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
                standing: &self.standing,
                making: &self.making,
                registry: &self.registry,
            },
        );
        m.run(&mut ctx);
        let asked = ctx.taken();

        // **XI-15: the cell events come first, because they change WHO the parties are.** Everything
        // else in a phase is about parties, and a leg naming a cell that is about to be split in two
        // has named a party whose weight is no longer what the module read.
        for (parent, taking, carrying) in asked.split {
            self.split_cell(parent, taking, carrying);
        }
        // **21j.1a: and obligations that have come into existence.** Before the legs, because a leg
        // that sells what was just brought names a line that has to be there first.
        for what in asked.issued {
            self.brought(what);
        }
        // **XI-10, XI-3, 22i.0: and relations struck and processes opened.** Before the legs for the
        // same reason: a leg that pays rent performs a tenancy, and the tenancy has to exist first.
        // `Agreements` and `Processes` are the one writer of each; a module asks.
        let today = self.calendar.start_of(crate::calendar::Period(self.period));
        for a in asked.agreed {
            self.agreements.strike(a.kind, a.one, a.other, &a.terms, today, a.until);
        }
        for o in asked.opened {
            self.processes.begin(o.kind, o.owner, self.period, o.closes, o.size);
        }

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
                    calendar: &self.calendar,
                    says: self.says,
                },
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
        // 17f, XI-3, 22i.0: and what ended — after the legs, because the last payment a relation
        // owed is made under it and not after it.
        for a in asked.ended {
            self.agreements.end(a);
        }
        for p in asked.closed {
            self.processes.finish(p);
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
        // §37 B3, 21f.3: what went ON the line and what came OFF it. The `Create` legs for what came
        // off are in `proposed` and settled above, so the mark and the leg are one event (Law 5).
        for (owner, what, units, cost, ready) in asked.started {
            self.making.starts(owner, what, units, cost, self.period, ready);
        }
        for batch in asked.finished {
            self.making.finishes(batch);
        }
        // 21f.1, 21f.2: and what a party now stands behind. A party that already stands behind terms
        // of this kind RESTATES them, so what it was standing behind stays readable beside what it is
        // (Housing C5: a tightening is only visible against what it was).
        for (kind, who, terms) in asked.stood {
            let was = self
                .standing
                .of_party(who)
                .iter()
                .map(|r| crate::stores::StandingId(*r))
                .find(|s| self.standing.live(*s) && self.standing.kind_of(*s) == kind);
            match was {
                Some(s) => {
                    self.standing.restates(s, &terms, self.period);
                }
                None => {
                    self.standing.stands(kind, who, &terms, self.period);
                }
            }
        }
        // XI-8, 21.36: and what came off one. A claim paid and not marked is paid again next period.
        for (claim, amount) in asked.repaid {
            self.claims.pays(claim, amount);
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
                schedules: &self.schedules,
                resting: &self.resting,
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
                schedules: &self.schedules,
                resting: &mut self.resting,
                calendar: &self.calendar,
            };
            let session = run_book(
                book,
                participants,
                &books,
                &mut stores,
                self.period,
                self.says,
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
        // Law 15, 21e: **the kernel asks the kind's PROFILE.** This used to read *a party banks at
        // somebody who issues money, OR at nobody at all* — a blanket escape, because the rule it
        // wanted (a central bank banks nowhere, a treasury at its central bank, everybody else at a
        // commercial bank) is a fact about the KIND and had nowhere to be written. So any party could
        // be admitted with no bank, and Money D2 was enforced only for the ones that happened to name
        // one. A kind with no profile is still admitted the old way: `Missing` is missing, and a world
        // that has declared no profiles is not one this can speak for.
        match self.registry.profile(kind).map(|p| p.banks) {
            Some(Banks::Nowhere) => assert!(
                !bank.some(),
                "Money D2: a party of kind {kind} issues the money others settle in, so it banks nowhere"
            ),
            Some(_) => assert!(
                bank.some() && self.instruments.money_issued_by(bank).is_some(),
                "Money D2: party of kind {kind} banks at {}, which issues no money — it would hold nothing it could pay with",
                bank.0
            ),
            None => assert!(
                !bank.some() || self.instruments.money_issued_by(bank).is_some(),
                "Money D2: party of kind {kind} banks at {}, which issues no money — it would hold nothing it could pay with",
                bank.0
            ),
        }
        self.parties.add(kind, region, bank, representation, weight, key)
    }

    /// A book, declared. The subject is what it delivers; its money is a CURRENCY, and each side pays
    /// out of its own account (22b.9a).
    pub fn open_book(&mut self, market: MarketId, subject: InstrumentId, ccy: CurrencyCode, venue: crate::protocols::Venue) {
        // 22b.9a: a book names a CURRENCY and each side pays out of its own account, so there is no
        // cash line to check the class of. What it does have to be is a thing somebody can deliver:
        // a book whose subject is money is a book for swapping a deposit for itself.
        assert!(
            self.instruments.class_of(subject) != Class::Money,
            "Clearing B1: a book's subject is what it delivers, and money is not delivered in a book"
        );
        self.books.push(BookDecl { market, subject, ccy, venue });
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
    use crate::clearing::PriceRule;
    use crate::ids::{PartyId, UnitId};
    use crate::ledger::{Cause, Leg};
    use crate::parties::Representation;

    struct Quiet;
    impl System for Quiet {
        fn name(&self) -> &'static str {
            "quiet"
        }
    }

    /// 22i.0: a module that strikes a relation and puts something in flight — the two acts no module
    /// could perform, which is why thirty of fifty-one systems could only count.
    struct Relates {
        kind: u32,
        one: PartyId,
        other: PartyId,
    }
    impl crate::module::Mechanism for Relates {
        fn run(&self, ctx: &mut crate::module::MechanismContext<'_>) {
            if ctx.period() > 1 {
                // It ends what it struck and closes what it opened, so both halves are exercised.
                for a in ctx.agreements().of_kind(self.kind).to_vec() {
                    ctx.ends(crate::stores::AgreementId(a));
                }
                for p in ctx.processes().running(self.kind) {
                    ctx.closes(p);
                }
                return;
            }
            ctx.agrees(crate::module::Agrees {
                kind: self.kind,
                one: self.one,
                other: self.other,
                terms: vec![7.0],
                until: None,
            });
            ctx.opens(crate::module::Opens { kind: self.kind, owner: self.one, closes: Some(9), size: 3.0 });
        }
    }
    struct Relating(Relates);
    impl System for Relating {
        fn name(&self) -> &'static str {
            "relating"
        }
        fn phases(&self) -> Vec<PhaseDecl> {
            vec![phase(900, 0, Anchor::After(MARKETS))]
        }
        fn mechanism(&self) -> Option<&dyn crate::module::Mechanism> {
            Some(&self.0)
        }
    }

    #[test]
    fn a_module_can_strike_a_relation_and_put_something_in_flight() {
        // **XI-10, XI-3, 22i.0: the two doors that were missing.** `Agreements::strike` and
        // `Processes::begin` had no caller outside tests, so every system whose whole content is a
        // relation counted the relations the assembly drew, and seven systems were a CLOSER for a
        // process nothing opens.
        let mut w = World::empty();
        let bank = w.parties.add(kinds::BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let firm = w.parties.add(kinds::FIRM, crate::ids::RegionId::at(0), bank, Representation::Named, 1, 0);
        let kind = 3u32;
        let r = Relating(Relates { kind, one: bank, other: firm });
        let systems: Vec<&dyn System> = vec![&r];
        w.wire_up(&systems);

        w.step(&systems);
        assert_eq!(w.agreements.len(), 1);
        let a = crate::stores::AgreementId(0);
        assert_eq!(w.agreements.between(a), (bank, firm));
        assert_eq!(w.agreements.terms(a), [7.0]);
        // G3.a: the kernel placed it on the calendar; the module said no day at all.
        assert_eq!(w.agreements.from(a), w.calendar.start_of(crate::calendar::Period(1)));
        assert!(w.agreements.live(a));
        assert_eq!(w.processes.len(), 1);
        let p = crate::stores::ProcessId(0);
        assert_eq!(w.processes.owner(p), bank);
        assert_eq!(w.processes.size(p), 3.0);
        assert!(!w.processes.done(p));

        // And both end, which is the half that makes them not immortal (XI-3).
        w.step(&systems);
        assert!(!w.agreements.live(a));
        assert!(w.processes.done(p));
    }

    #[test]
    fn a_system_with_no_reason_to_post_has_no_participants_and_that_is_not_a_gap() {
        // Clearing B2, Appendix B: giving a read a schedule would be inventing demand nobody has.
        let q = Quiet;
        assert!(q.participants().is_empty());
        assert!(q.phases().is_empty());
    }

    #[test]
    fn the_period_opens_with_the_orders_that_are_still_good() {
        // **3 C2, G3.a, 22c2.2: the calendar expires what stood to yesterday.** Every order in this
        // world rested for ever because the assembled world had no calendar in it to place a date
        // against — 16,869 resting after one period and 33,069 after four, with books cleared
        // falling from five to one.
        let mut w = World::empty();
        let who = w.parties.add(kinds::FIRM, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        // Period 0 is days 0..6, so an order standing to day 6 is good for period 0 and no longer.
        let week = w.resting.enters(who, 3, false, Some(2.0), 100, 0, Some(crate::calendar::Day(6)), 0);
        let forever = w.resting.enters(who, 3, false, Some(2.0), 100, 0, None, 0);
        let q = Quiet;
        let systems: Vec<&dyn System> = vec![&q];
        w.wire_up(&systems);
        w.step(&systems);
        assert!(!w.resting.live(week), "period 1 opens on day 7 and its day has passed");
        assert!(w.resting.live(forever), "a venue that declared no life for it did not end it");
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
        assert_eq!((did.asks, did.books_cleared, did.trades, did.events, did.ran), (0, 0, 0, 0, 0));
        // **Audit E2, 22e.2: and it was AUDITED.** A world that assembled no family must not read as
        // a world with no violations, so every family is in the report — the three the kernel builds
        // saying nothing is wrong, and the seven nobody has built saying they are NOT BUILT.
        assert_eq!(did.audit.len(), crate::audit::Family::ALL.len());
        assert!(did.audit.iter().all(|r| r.violations.is_empty()));
        // TWO built families, not three contributions: `LotsAgainstQuantity` and
        // `NoCollateralCountedTwice` both contribute to OWNERSHIP and are one report between them.
        assert_eq!(did.audit.iter().filter(|r| r.built).count(), 2);
        assert!(
            did.audit.iter().any(|r| !r.built && r.family == crate::audit::Family::Flows),
            "a family nobody built says so, and is never absent from the report"
        );
    }

    /// A system that does nothing but run `Wages`, so the split reaches the kernel the way any
    /// module's does. A test names no party (`docs/CLAUDE.md`): it asks the world for a cell.
    struct Employs;
    impl System for Employs {
        fn name(&self) -> &'static str {
            "employment"
        }
        fn phases(&self) -> Vec<PhaseDecl> {
            vec![phase(3, 3, Anchor::After(CORPORATE_ACTIONS))]
        }
        fn mechanism(&self) -> Option<&dyn Mechanism> {
            Some(&crate::running::Wages)
        }
    }

    #[test]
    fn an_engagement_for_part_of_a_cell_splits_it_and_the_two_halves_hold_what_the_one_held() {
        // **XI-15, Labour A4.c, 21h.** A firm employs some of a household cell. Those members become
        // a cell of their own carrying the engagement and EXACTLY their share of what the parent
        // held — not a fraction anybody rounded, because identical members divide without remainder.
        // The population does not change: a split divides a group, it does not create one.
        let mut w = World::empty();
        let region = crate::ids::RegionId::at(0);
        let cb = w.parties.add(kinds::CENTRAL_BANK, region, PartyId::NONE, Representation::Named, 1, 0);
        let bank = w.parties.add(kinds::BANK, region, cb, Representation::Named, 1, 0);
        let _reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let cash = w.instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let bread = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let firm = w.parties.add(kinds::FIRM, region, bank, Representation::Named, 1, 0);
        let cell = w.parties.add(kinds::HOUSEHOLD, region, bank, Representation::Cell, 1_000, 7);
        w.register.money_delta(firm, cash, 50_000.0);
        w.register.money_delta(cell, cash, 4_000.0);
        w.register.credit(cell, bread, 800.0, 0.5, 0);
        // §46: and a view of its own, so the split can be asked whether the history went with it.
        w.outlooks.form(cell, crate::running::about::WHAT_IT_SELLS_FOR, 3.0, 0);
        w.agreements.strike(
            crate::running::agreed::ENGAGEMENT,
            firm,
            cell,
            &[2.0, 35.0, 250.0],
            crate::calendar::Day(-100),
            None,
        );

        let e = Employs;
        let systems: Vec<&dyn System> = vec![&e];
        w.wire_up(&systems);
        w.step(&systems);

        let part = PartyId::at(w.parties.len() as u32 - 1);
        assert_eq!(w.parties.weight(cell), 750);
        assert_eq!(w.parties.weight(part), 250);
        // A quarter of the people took a quarter of each holding, exactly.
        assert_eq!(w.register.quantity(w.register.row(part, cash)), 1_000.0);
        assert_eq!(w.register.quantity(w.register.row(cell, cash)), 3_000.0);
        assert_eq!(w.register.quantity(w.register.row(part, bread)), 200.0);
        assert_eq!(w.register.quantity(w.register.row(cell, bread)), 600.0);
        // Register D2: and at what the units cost, because nothing was sold.
        assert_eq!(w.register.lots(w.register.row(part, bread))[0].basis_per_unit, 0.5);
        // 21.20: one group, one history. A cell split off its parent has seen what its parent saw.
        assert_eq!(w.outlooks.of(part, crate::running::about::WHAT_IT_SELLS_FOR), Some(3.0));
        // Labour A4.c: and the relationship went with the people it is a relationship with.
        assert!(w.agreements.of_party(part).len() == 1 && w.agreements.of_party(cell).is_empty());
    }

    /// A system that does nothing but fund whoever is short, so the door reaches the kernel the way
    /// any module's does.
    struct Funds;
    impl System for Funds {
        fn name(&self) -> &'static str {
            "treasury"
        }
        fn phases(&self) -> Vec<PhaseDecl> {
            vec![phase(3, 3, Anchor::After(CORPORATE_ACTIONS))]
        }
        fn mechanism(&self) -> Option<&dyn Mechanism> {
            Some(&FUNDING)
        }
    }
    static FUNDING: crate::running::Funding = crate::running::Funding {
        days_per_period: 7,
        tenor: "test.funding.tenor",
        coupon: "test.funding.coupon",
        buffer: "test.funding.buffer",
        says: 0,
    };

    #[test]
    fn a_party_short_of_money_brings_paper_that_did_not_exist_before() {
        // **21j.1a, 21.139: NOTHING IN THIS WORLD COULD ISSUE AN INSTRUMENT.** The instrument table
        // was whatever the assembly built and it never changed while the world ran, so no treasury
        // could auction a bill it had not got. Here one reads what falls due on it, finds it short,
        // and the line, the units, the book and the schedule all come into existence together.
        let mut w = World::empty();
        let region = crate::ids::RegionId::at(0);
        let cb = w.parties.add(kinds::CENTRAL_BANK, region, PartyId::NONE, Representation::Named, 1, 0);
        w.registry.profile_for(
            kinds::CENTRAL_BANK,
            crate::registry::KindProfile { issues_money: true, banks: Banks::Nowhere, issues_paper: false },
        );
        w.registry.profile_for(
            kinds::TREASURY,
            crate::registry::KindProfile {
                issues_money: false,
                banks: Banks::AtTheCentralBank,
                issues_paper: true,
            },
        );
        let reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let treasury = w.admit(kinds::TREASURY, region, cb, Representation::Named, 1, 0);
        // It has a little money and owes a lot this period, so it is short.
        w.register.money_delta(treasury, reserves, 100.0);
        let old = w.instruments.issue(treasury, CurrencyCode::at(0), Class::Claim, UnitId::at(0), None, Some(crate::calendar::Day(7)));
        w.schedules.owes(old, treasury, crate::calendar::Day(3), 900.0, crate::stores::Owing::Principal);
        for (id, value, dimension) in [
            ("test.funding.tenor", 26.0, crate::params::Dimension::Periods),
            ("test.funding.coupon", 0.04, crate::params::Dimension::PerAnnum),
            ("test.funding.buffer", 2.0, crate::params::Dimension::Amount(crate::params::Denomination::Money)),
        ] {
            w.params.declare(crate::params::ParamDecl {
                id: id.to_string(),
                value,
                unit: "stated".to_string(),
                dimension,
                kind: crate::params::Kind::Technology,
                owner: crate::params::Owner::StandardSetter,
                why: "what this test funds against".to_string(),
            });
        }

        let lines_before = w.instruments.len();
        let books_before = w.books.len();
        let f = Funds;
        let systems: Vec<&dyn System> = vec![&f];
        w.wire_up(&systems);
        w.step(&systems);

        // A line that did not exist, a book for it, and what it owes — one act.
        assert_eq!(w.instruments.len(), lines_before + 1, "it brought paper");
        assert_eq!(w.books.len(), books_before + 1, "and a book opened for it");
        let brought = InstrumentId::at(lines_before as u32);
        assert_eq!(w.instruments.issuer_of(brought), treasury);
        // 5 A4: its own paper on its own book. It holds what it brought and owes nothing yet —
        // what it owes starts when somebody else comes to hold one.
        assert!(w.register.quantity(w.register.row(treasury, brought)) > 0.0);
        // 5 D2: and the coupon and the principal are written down at issue.
        assert_eq!(w.schedules.of_instrument(brought).len(), 2);
        assert!(w.schedules.outstanding(brought) > 0.0);
    }

    struct Fails;
    impl System for Fails {
        fn name(&self) -> &'static str {
            "mortality"
        }
        fn phases(&self) -> Vec<PhaseDecl> {
            vec![phase(3, 3, Anchor::After(REVALUATION))]
        }
        fn mechanism(&self) -> Option<&dyn Mechanism> {
            Some(&FAILING)
        }
    }
    static FAILING: crate::running::Failing = crate::running::Failing { says: 0 };

    #[test]
    fn a_party_whose_liabilities_exceed_its_assets_ceases_and_the_one_exception_is_a_consequence() {
        // **XI-3, Appendix B: nothing is immortal — and nothing in this world had ever died.** The
        // mortality row counted who was alive, which is the opposite of the read it is for.
        let mut w = World::empty();
        let region = crate::ids::RegionId::at(0);
        let cb = w.parties.add(kinds::CENTRAL_BANK, region, PartyId::NONE, Representation::Named, 1, 0);
        w.registry.profile_for(
            kinds::CENTRAL_BANK,
            crate::registry::KindProfile { issues_money: true, banks: Banks::Nowhere, issues_paper: false },
        );
        w.registry.profile_for(
            kinds::FIRM,
            crate::registry::KindProfile {
                issues_money: false,
                banks: Banks::AtACommercialBank,
                issues_paper: true,
            },
        );
        let _reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let broke = w.parties.add(kinds::FIRM, region, cb, Representation::Named, 1, 0);
        let sound = w.parties.add(kinds::FIRM, region, cb, Representation::Named, 1, 0);
        let holder = w.parties.add(kinds::FIRM, region, cb, Representation::Named, 1, 0);

        // 5 A4: what it owes is what OTHERS hold of what it issued. One firm's paper is out and it
        // holds nothing; the other holds something and has issued nothing.
        let paper = w.instruments.issue(broke, CurrencyCode::at(0), Class::Claim, UnitId::at(0), None, None);
        w.register.credit(holder, paper, 500.0, 1.0, 0);
        let good = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        w.register.credit(sound, good, 10.0, 3.0, 0);

        let f = Fails;
        let systems: Vec<&dyn System> = vec![&f];
        w.wire_up(&systems);
        w.step(&systems);

        assert!(!w.parties.alive(broke), "what it owes is more than what it has");
        assert!(w.parties.alive(sound), "and a firm that owes nothing does not die of it");
        // §31 A1.a: the exception is a CONSEQUENCE of what it issues, not a rule relaxed for it —
        // the central bank has issued money everybody holds and cannot run out of what it creates.
        assert!(w.parties.alive(cb), "a central bank cannot fail in its own money");
    }

    #[test]
    fn a_participant_can_see_what_falls_due_on_it() {
        // **XI-9, 21j.1.** A view could read what a party held and not what it owed, so nothing in
        // this world had a funding constraint that bound. Observer A4: its OWN obligations, and what
        // falls due to it read off the lines it holds.
        let mut w = World::empty();
        let region = crate::ids::RegionId::at(0);
        let cb = w.parties.add(kinds::CENTRAL_BANK, region, PartyId::NONE, Representation::Named, 1, 0);
        let borrower = w.parties.add(kinds::FIRM, region, cb, Representation::Named, 1, 0);
        let lender = w.parties.add(kinds::BANK, region, cb, Representation::Named, 1, 0);
        let loan = w.instruments.issue(borrower, CurrencyCode::at(0), Class::Claim, UnitId::at(0), None, None);
        w.register.credit(lender, loan, 1.0, 1_000.0, 0);
        w.schedules.owes(loan, borrower, crate::calendar::Day(5), 40.0, crate::stores::Owing::Interest);
        w.schedules.owes(loan, borrower, crate::calendar::Day(99), 1_000.0, crate::stores::Owing::Principal);

        let seen = |who: PartyId| {
            ParticipantView::of(who, &w.register, &w.prints, &w.journal, &w.params, 1, None)
                .owing(&w.schedules)
        };
        // The borrower owes the coupon by day 5 and everything by day 99.
        assert_eq!(seen(borrower).owes_by(crate::calendar::Day(5)), 40.0);
        assert_eq!(seen(borrower).owes_by(crate::calendar::Day(99)), 1_040.0);
        // And the lender, which HOLDS the loan, is owed exactly the same — one fact, read from the
        // two ends, and never a second list of who is owed what.
        assert_eq!(seen(lender).owed_to_it_by(crate::calendar::Day(99)), 1_040.0);
        // Neither is owed what it owes itself.
        assert_eq!(seen(borrower).owed_to_it_by(crate::calendar::Day(99)), 0.0);
        // A view built without the schedules answers nothing, which a caller must not read as a
        // party owing nothing.
        let blind = ParticipantView::of(borrower, &w.register, &w.prints, &w.journal, &w.params, 1, None);
        assert_eq!(blind.owes_by(crate::calendar::Day(99)), 0.0);
    }

    #[test]
    fn a_book_delivers_something_that_is_not_money() {
        // 22b.9a: a book names a CURRENCY and each side pays out of its own account, so there is no
        // cash line to check. What it must have is something to DELIVER.
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let share = w.instruments.issue(cb, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.open_book(MarketId::at(0), share, CurrencyCode::at(0), a_call());
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
        w.open_book(MarketId::at(0), cash, CurrencyCode::at(0), a_call());
    }

    /// A sealed cross: nothing rests in it, so it declares no life for an order.
    fn a_call() -> crate::protocols::Venue {
        crate::protocols::Venue {
            rule: PriceRule::SellersCompete,
            protocol: crate::protocols::Protocol::Call,
            seen_by: 1,
            stands_for: None,
        }
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

        // **The measure's whole point is that the count FALLS**, so this asserts what has gone home
        // rather than what has not. The registry's four went at 21e; the three a module kept and no
        // store held went at 21f — a posting and a lending standard into `standing`, work in progress
        // into `making`.
        let named: Vec<&str> = homeless.iter().map(|(n, _)| *n).collect();
        for gone in [
            "registry.currencies",
            "registry.places",
            "registry.units",
            "registry.kind_profiles",
            "employment.postings",
            "lending.standards",
            "recipe.work_in_progress",
        ] {
            assert!(!named.contains(&gone), "{gone} has a home now");
        }
        // And what is left is what the re-read of item 21 found and nothing yet holds. A count of
        // zero would be this measure switched off, not a world with nothing missing.
        assert!(!named.contains(&"registry.indices"), "it got a home at 21.116");
        assert!(!named.contains(&"settlement.realised"), "it got a home at 21.112");
        assert!(!named.contains(&"reporting.accounts"), "it got a home at 22i.1, in the journal");

        // And every one names the item that gives it a home. A noun whose item nobody has written
        // is a noun nobody has agreed to build.
        for (name, item) in &homeless {
            assert!(!item.is_empty(), "{name} names no item");
        }
    }

    #[test]
    fn a_store_the_kernel_owns_is_not_homeless_and_a_fact_with_nowhere_to_live_is() {
        // The distinction the register exists to hold, and it is the same distinction wherever the
        // line currently falls: `agreements` had no home before 21d, `standing` none before 21f, and
        // `settlement.realised` has none yet — a gain that exists on the register and in no read.
        let w = World::empty();
        let named: Vec<&str> = w.nouns.homeless().iter().map(|(n, _)| *n).collect();
        assert!(!named.contains(&"agreements"), "it got a home at 21d");
        assert!(!named.contains(&"claims"), "it got one at 21c");
        assert!(!named.contains(&"standing"), "it got one at 21f");
        assert!(!named.contains(&"reporting.accounts"), "it went home to the journal at 22i.1");
        // And what the 22i re-read found in its place. The count falls by what goes home and rises
        // by what a re-read finds, which is the whole of how this measure stays honest.
        assert!(named.contains(&"ratings.grades"), "nothing holds a grade, so two houses cannot disagree");
        assert!(named.contains(&"agreements.states"), "a relation is live or ended and never breached");
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
