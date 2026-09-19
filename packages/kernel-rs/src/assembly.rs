//! THE ASSEMBLY: the systems, wired into one world that steps.
//!
//! @spec ARCHITECTURE 4.9b · 1 G3 · 3 B2 · 4 · Law 4, Law 10, Law 15, Law 19

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

/// The party kinds this world has.
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
    /// 37 C3, 22c.4: SOMEBODY WHOSE BUSINESS IS TO HOLD THE STOCK.
    pub const STOCKIST: u32 = 11;
    pub const ALL: [u32; 12] = [
        HOUSEHOLD, FIRM, BANK, FUND, INSURER, DEALER, TREASURY, CENTRAL_BANK, CARRIER, SMALL_FIRM,
        ASSESSOR, STOCKIST,
    ];
}

/// One spec system, wired.
pub trait System {
    /// The spec system this is, for the record and for the census.
    fn name(&self) -> &'static str;

    /// Where its work falls in the period.
    fn phases(&self) -> Vec<PhaseDecl> {
        Vec::new()
    }

    /// Who it puts into books, if anybody.
    fn participants(&self) -> Vec<&dyn Participant> {
        Vec::new()
    }

    /// Its own work in the period (ARCHITECTURE 4.9b, the second door): accruing, maturing,
    /// deciding, publishing.
    fn mechanism(&self) -> Option<&dyn Mechanism> {
        None
    }

    /// What this system contributes to the audit.
    fn audits(&self) -> Vec<Box<dyn crate::audit::Contribution>> {
        Vec::new()
    }
}

/// Every kernel store, declared for what it is.
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
    // The four the registry now holds.
    at_home("registry.currencies", "each money and the party whose liability it is", "Money A2: money is somebody's liability, and a CurrencyCode named nobody");
    at_home("registry.places", "countries and the regions in them", "Seed B3, 13c.1: a country has the money and a region is a place, so currency_of(region) reads through the country");
    at_home("registry.units", "each unit and what one of it is divided into", "Law 8: the unit is part of the number, and one grid for everything made a dwelling divisible");
    at_home("registry.kind_profiles", "what varies by party kind, behind a dispatch the kernel reads", "Law 15: a world whose kinds have no profiles has nowhere to put what varies, so the pressure to branch never goes away");
    // The three a module named and no store kept.
    at_home("standing", "terms a party stands behind: a posting, a lending standard", "XI-10, Housing C5: a posting is HELD by an employer, which is what lets it be withdrawn; a standard is a decision that persists and that a borrower meets or does not");
    at_home("making", "what is between input and output, owned, carrying what it cost", "37 B3: work in progress is a real thing with a holder, not a timing adjustment");
    // It is the JOURNAL's, because a realised gain is an event rather than a thing anybody holds —
    // it happens at the moment the units leave, to a named party, for an amount.
    at_home("registry.indices", "each index, the country whose it is, and the lines it is built from with a COUNT of each", "Indices D1, 22 D5: an index is a COUNTRY's and it is ONE system; the level is never stored, it is computed from the constituents when asked");
    at_home("ratings.grades", "the grade an assessor currently holds on an issuer, and what it was before", "21 A4, A6: two houses hold two rows on one name and may disagree, and a move is a `restates`, so what a house said before stays readable beside what it says now");
    at_home("reporting.estimates", "what each bank expects a named company to report", "48 C1, C3: `consensus` was computed from a list that existed for one call, so the disagreement C3 is about could not survive the call that measured it");
    at_home("reporting.accounts", "the accounts a party has PUBLISHED, as at a date — an EVENT in the journal, on a day, to a named company, public", "48 A1, 22i.1: a covenant is tested against published accounts and a bid is formed from them, and `journal.of_kind(accounts.published)` is that read. What was missing was never a store — it was a mechanism");
    at_home("settlement.realised", "what each disposal realised against the basis its lots carried", "Law 19: settlement is the only place that holds the price and the basis at once, so anywhere else would re-derive one of them");
    // The wire is what HAPPENED; this is what is still trying to.
    at_home("settlement.queue", "payments that could not be made yet, with the day each is late on", "XI-9, 22d: a gridlock is a timing failure and not a default, and this world turned every one of them into an arrear the instant it was tried");

    // AND WHAT HAS NO HOME.
    let mut homeless = |name: &str, item: &str, holds: &str, why: &str| {
        n.declare(NounDecl {
            name: name.to_string(),
            sort: Sort::Noun { home: Some(item.to_string()) },
            holds: holds.to_string(),
            why: why.to_string(),
        });
    };

    // AND WHAT STILL HAS NO HOME.
    homeless(
        "control.resistance",
        "23.1",
        "what a target's management is doing to defend it, and what that costs the bidder",
        "35 C3: management may resist and its interests differ from the owners', which is the corporate-control problem — and nothing in this world resists, so a tender meets only the owners' own valuations",
    );
    homeless(
        "derivatives.collateral",
        "23.1",
        "what is posted against a derivative position, by whom, and what it is worth now",
        "16 C1, Appendix B: no margin that is only a number. A position marks (22i.19) and nothing is posted against it, so a counterparty exposure has nothing standing behind it",
    );
    homeless(
        "agreements.states",
        "21.62",
        "whether a relation is live, breached, cured, discharged or terminated",
        "17.7: `Agreements` is live or ended, so this world records neither a breach nor a cure — and a payer given time on an arrear has nowhere to land",
    );

    n
}

/// The stores, one of each, owned by the kernel.
pub struct World {
    pub parties: Parties,
    pub instruments: Instruments,
    pub register: Register,
    pub prints: Prints,
    pub journal: Journal,
    pub wire: Settlement,
    pub params: Params,
    /// The relations.
    pub agreements: Agreements,
    /// What each instrument owes and when.
    pub schedules: Schedules,
    /// What each deciding party expects, formed from its own history.
    pub outlooks: Outlooks,
    /// Whatever is in flight across periods with an owner and an end.
    pub processes: Processes,
    /// Who is owed what by a party whose life has ended, and at what rank.
    pub claims: Claims,
    /// ARCHITECTURE 4.10, 21e: what the ids point at — each money's issuer, each country's money and
    /// each region's country, each unit's subdivision, and a profile per party kind.
    pub registry: Registry,
    /// Terms a named party stands behind until it withdraws them — a posting, a lending standard.
    pub standing: Standing,
    /// What is between input and output, owned by somebody, carrying what it cost.
    pub making: InProgress,
    /// The ontology register.
    pub nouns: Nouns,
    pub phases: Phases,
    /// The families, assembled at `wire_up` and run every period.
    pub audit: crate::audit::Audit,
    /// 3 C2, 22c.2: the standing book — orders that rest between sessions.
    pub resting: crate::stores::Resting,
    pub books: Vec<BookDecl>,
    pub calendar: crate::calendar::Calendar,
    pub period: u32,
    /// The journal kinds an instruction's outcome is said under — settled, failed, queued, and what
    /// a disposal realised.
    pub says: crate::ledger::Outcomes,
}

/// What one period did.
#[derive(Debug, Default)]
pub struct Stepped {
    /// What the audit found in this period, by family.
    pub audit: Vec<crate::audit::Report>,
    pub asks: usize,
    pub narrows: usize,
    pub books_cleared: usize,
    pub trades: usize,
    pub events: usize,
    /// How many systems' phases actually ran.
    pub ran: usize,
    /// How many queued payments the gridlock pass settled, in cycles nobody in them could have paid
    /// alone.
    pub unwound: usize,
    /// What became of THIS period's short payments — still waiting, went through after waiting, ran
    /// out of days.
    pub queue: (usize, usize, usize),
    /// How many processes reached their end this period.
    pub closed: usize,
    /// The declaration slots whose mechanism DECIDED something this period — read off what it asked
    /// for (`Taken::decided`), never declared by the mechanism itself.
    pub decided: Vec<u32>,
}

impl World {
    /// The world before anything has happened to it.
    pub fn empty() -> World {
        let mut journal = Journal::new();
        // Settled, failed, QUEUED (22d.1 — a payment waiting for the money to arrive, which is
        // neither of the other two) and what a disposal realised.
        let says = crate::ledger::Outcomes::declared(&mut journal);
        World {
            parties: Parties::new(),
            instruments: Instruments::new(),
            register: Register::new(),
            prints: Prints::new(),
            journal,
            // How many days a payment may wait here before it is late.
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
            // The period is 7 days with 3 settlement cycles in it, and the world opened on
            // 2000-01-01 (`calendar::Day`'s epoch).
            calendar: crate::calendar::Calendar::new(crate::calendar::Day(0), 7, 3),
            period: 0,
            says,
        }
    }

    /// Every system's phases, in one order, sealed — a phase that reads a not-yet-produced print
    /// throws rather than reading a stale one.
    pub fn wire_up(&mut self, systems: &[&dyn System]) {
        // A system wired twice would run twice, and one that neither works nor posts is a
        // declaration and nothing else. Both are caught here because this is the one door.
        let mut named: Vec<&str> = systems.iter().map(|s| s.name()).collect();
        named.sort_unstable();
        let wired = named.len();
        named.dedup();
        assert_eq!(wired, named.len(), "ARCHITECTURE 4.9b: a system is wired once");
        for s in systems {
            assert!(
                s.mechanism().is_some() || !s.participants().is_empty(),
                "ARCHITECTURE 4.9b: {} is wired and neither works nor posts",
                s.name()
            );
            for phase in s.phases() {
                self.phases.add(phase);
            }
        }
        self.phases.seal();
        // The audit is assembled here, with the phases.
        let mut contributions: Vec<Box<dyn crate::audit::Contribution>> = vec![
            Box::<crate::audit::ATotalCarriesNoLots>::default(),
            Box::<crate::audit::NoCollateralCountedTwice>::default(),
            Box::<crate::audit::HoldersAgainstIssued>::default(),
            Box::<crate::audit::MoneyIsConserved>::default(),
            Box::<crate::audit::FlowsAreComplete>::default(),
            Box::<crate::audit::NamesResolve>::default(),
        ];
        for s in systems {
            contributions.extend(s.audits());
        }
        self.audit = crate::audit::Audit::over(contributions);
    }

    /// One period: the phases run in their declared order, the books run at the markets moment, and
    /// what each proposed is settled.
    pub fn step(&mut self, systems: &[&dyn System]) -> Stepped {
        self.period += 1;
        // And the parties store knows what period it is, so a party entering in it is stamped with
        // it.
        self.parties.opened(self.period);
        let participants: Vec<&dyn Participant> =
            systems.iter().flat_map(|s| s.participants()).collect();
        let mut out = Stepped::default();
        let events_before = self.journal.len();

        // 3 C2, G3.a, 22c2.2: the calendar expires what stood to yesterday, before anything reads a
        // book.
        let today = self.calendar.start_of(crate::calendar::Period(self.period));
        self.resting.expire(today);

        // And the payments that ran out of days.
        self.wire.give_up(
            today,
            self.period,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &mut self.instruments,
                calendar: &self.calendar,
                says: self.says,
            },
        );

        // In order, and the markets moment is where the books run.
        let order: Vec<(u32, bool)> = self
            .phases
            .order()
            .iter()
            .map(|p| (p.owner, matches!(p.anchor, Anchor::After(MARKETS) | Anchor::Before(REVALUATION))))
            .collect();
        let by_slot = slots(systems);

        for (owner, after_markets) in order.iter().filter(|(_, after)| !after) {
            out.ran += self.run_phase(*owner, &by_slot, systems, &mut out);
            let _ = after_markets;
        }
        out.trades += self.run_books(&participants, &mut out);
        for (owner, _) in order.iter().filter(|(_, after)| *after) {
            out.ran += self.run_phase(*owner, &by_slot, systems, &mut out);
        }

        // AND WHATEVER IS IN FLIGHT CLOSES WHEN ITS PERIOD COMES.
        let closing: Vec<crate::stores::ProcessId> = (0..self.processes.len() as u32)
            .map(crate::stores::ProcessId)
            .filter(|p| !self.processes.done(*p))
            .filter(|p| matches!(self.processes.closes(*p), Some(when) if when <= self.period))
            .collect();
        out.closed = closing.len();
        for p in closing {
            let (owner, size) = (self.processes.owner(p), self.processes.size(p));
            self.processes.finish(p);
            // What closed, whose it was and how big it was.
            self.journal.say(
                self.period,
                0,
                self.says.closed,
                &[owner.0],
                &[(0, crate::journal::Value::Num(size))],
                true,
            );
        }

        // And the gridlock pass, once, with every payment of the period in.
        out.unwound = self.wire.unwind(
            self.period,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &mut self.instruments,
                calendar: &self.calendar,
                says: self.says,
            },
        );

        // And what became of the payments that were short in it.
        let closes = crate::calendar::Day(self.calendar.start_of(crate::calendar::Period(self.period + 1)).0 - 1);
        out.queue = self.wire.queue.between(today, closes);

        out.events = self.journal.len() - events_before;
        // EVERY FAMILY, EVERY PERIOD, over the one traversal it was built for.
        out.audit = self.audit.run(&crate::audit::Sources {
            wire: &self.wire,
            register: &self.register,
            instruments: &self.instruments,
            parties: &self.parties,
            period: self.period,
        });
        out
    }

    /// One system's phase: its mechanism reads the stores, proposes, and the kernel settles.
    fn split_cell(&mut self, parent: PartyId, taking: std::num::NonZeroU32, carrying: crate::stores::AgreementId) {
        let had = self.parties.weight(parent);
        let share = f64::from(taking.get()) / f64::from(had);
        let child = self.parties.split(parent, taking);

        let mut legs: Vec<crate::ledger::Leg> = Vec::new();
        for row in self.register.of_holder(parent) {
            let row = crate::ids::HoldingId(*row);
            let line = self.register.instrument_of(row);
            // What is pledged does not move, so the members take their share of what is free.
            // What is pledged does not move, and a share of nothing is not a leg.
            let Some(theirs) = crate::ledger::Units::new(self.register.free(row) * share) else {
                continue;
            };
            legs.push(if self.instruments.class_of(line) == Class::Money {
                crate::ledger::Leg::Money {
                    from: parent,
                    to: child,
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
                    // No price.
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
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                },
            );
        }

        // One group, one history.
        for row in self.outlooks.of_party(parent).to_vec() {
            let about = self.outlooks.about_at(row);
            let level = self.outlooks.level_at(row);
            self.outlooks.form(child, about, level, self.period);
        }

        // And the relationship that applies to them goes with them.
        self.agreements.moves(carrying, parent, child);
    }

    /// AN OBLIGATION COMES INTO EXISTENCE.
    fn brought(&mut self, what: crate::module::Brings) {
        let line = self.instruments.issue(
            what.issuer,
            what.ccy,
            what.class,
            what.unit,
            what.coupon,
            what.matures,
        );
        // Settlement is the one writer of the register, so units arrive over the wire like
        // everything else.
        if let Some(units) = crate::ledger::Units::new(what.units) {
            let legs = [crate::ledger::Leg::Create {
                party: what.issuer,
                instrument: line,
                qty: units,
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
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                },
            );
        }
        // A book for it, if it is paper anybody else may bid for.
        match what.book {
            Some(venue) => self.open_book(crate::systems::book_of(line), line, what.ccy, venue),
            None => self.instruments.carried_at_cost(line),
        }
        // And what it owes, by date.
        for (due, amount, of) in what.owing {
            self.schedules.owes(line, what.issuer, due, amount, of);
        }
    }

    fn run_phase(&mut self, owner: u32, by_slot: &[usize], systems: &[&dyn System], out: &mut Stepped) -> usize {
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
                instruments: &mut self.instruments,
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
        // What it asked for is what it did, and this is the only place both are in hand at once.
        if asked.decided() {
            out.decided.push(owner);
        }

        // The cell events come first, because they change WHO the parties are.
        for (parent, taking, carrying) in asked.split {
            self.split_cell(parent, taking, carrying);
        }
        // And obligations that have come into existence.
        for what in asked.issued {
            self.brought(what);
        }
        // And relations struck and processes opened.
        let today = self.calendar.start_of(crate::calendar::Period(self.period));
        for a in asked.agreed {
            self.agreements.strike(a.kind, a.one, a.other, &a.terms, today, a.until);
        }
        for o in asked.opened {
            self.processes.begin(o.kind, o.owner, self.period, o.closes, o.size);
        }

        // Settlement is the one writer of the register.
        for p in asked.proposed {
            let instruction = Instruction { legs: &p.legs, cause: p.cause, delivery: p.delivery };
            self.wire.settle(
                &instruction,
                self.period,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                },
            );
        }
        // And what it said happened, for whoever it happened to.
        for s in asked.said {
            self.journal.say(self.period, 0, s.kind, &s.subjects, &s.data, s.public);
        }
        // The outlooks it formed from its parties' own histories.
        for (who, about, level) in asked.formed {
            self.outlooks.form(who, about, level, self.period);
        }
        // A-20: and what it paid off a schedule.
        for due in asked.settled {
            self.schedules.settle(due);
        }
        // And what ended — after the legs, because the last payment a relation owed is made under it
        // and not after it.
        for a in asked.ended {
            self.agreements.end(a);
        }
        for p in asked.closed {
            self.processes.finish(p);
        }
        // And the payments a seller agreed to wait for.
        for (q, until) in asked.on_terms {
            self.wire.queue.given_time(q, until);
        }
        // And whose life ended.
        for who in asked.ceased {
            self.parties.cease(who);
        }
        // And who is owed what by an estate.
        for (on, holder, owed, ranks) in asked.claimed {
            self.claims.against(on, holder, owed, ranks);
        }
        // What went ON the line and what came OFF it.
        for (owner, what, units, cost, ready) in asked.started {
            self.making.starts(owner, what, units, cost, self.period, ready);
        }
        for batch in asked.finished {
            self.making.finishes(batch);
        }
        // And what a party now stands behind.
        for (kind, who, about, terms) in asked.stood {
            let was = self.standing.of_party_about(who, about, kind);
            match was {
                Some(s) => {
                    self.standing.restates(s, &terms, self.period);
                }
                None => {
                    self.standing.stands(kind, who, about, &terms, self.period);
                }
            }
        }
        // And what came off one.
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
                instruments: &mut self.instruments,
                register: &self.register,
                prints: &self.prints,
                journal: &self.journal,
                params: &self.params,
                agreements: &self.agreements,
                schedules: &self.schedules,
                resting: &self.resting,
                processes: &self.processes,
            },
            self.period,
        );
        out.narrows += books.narrows;
        let mut traded = 0usize;
        for book in &self.books {
            let mut stores = Stores {
                parties: &self.parties,
                instruments: &mut self.instruments,
                register: &mut self.register,
                prints: &mut self.prints,
                journal: &mut self.journal,
                wire: &mut self.wire,
                params: &self.params,
                agreements: &self.agreements,
                schedules: &self.schedules,
                resting: &mut self.resting,
                processes: &self.processes,
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
            // A book that had something cross is one that cleared; one that did not prints nothing,
            // and counting it would be a session that never happened.
            if matches!(session.outcome, crate::clearing::Outcome::Cleared { .. }) {
                out.books_cleared += 1;
            }
        }
        traded
    }

    /// A PARTY IS ADMITTED TO A WORLD, AND ITS BANK HAS TO ISSUE MONEY.
    pub fn admit(
        &mut self,
        kind: u32,
        region: crate::ids::RegionId,
        bank: PartyId,
        representation: crate::parties::Representation,
        key: u32,
    ) -> PartyId {
        // The kernel asks the kind's PROFILE.
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
        self.parties.add(kind, region, bank, representation, key)
    }

    pub fn open_book(&mut self, market: MarketId, subject: InstrumentId, ccy: CurrencyCode, venue: crate::protocols::Venue) {
        // A book names a CURRENCY and each side pays out of its own account, so there is no cash
        // line to check the class of.
        assert!(
            self.instruments.class_of(subject) != Class::Money,
            "Clearing B1: a book's subject is what it delivers, and money is not delivered in a book"
        );
        self.books.push(BookDecl { market, subject, ccy, venue });
    }
}

/// Which system owns each declaration slot, so a phase in the order can be traced back to the system
/// that declared it.
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

/// What a party will pay or take for what it already holds, read from its own book.
pub fn holds_of(view: &ParticipantView<'_>, subject: InstrumentId) -> f64 {
    view.free(subject)
}

/// Its own money, for what it can actually fund — Law 6: a bid it cannot pay for is not a bid.
pub fn can_fund(view: &ParticipantView<'_>, cash: InstrumentId) -> f64 {
    view.free(cash)
}

/// A seller's order, from what it holds and what it will take.
pub fn offer(view: &ParticipantView<'_>, subject: InstrumentId, at_least: f64) -> Option<Order> {
    let units = holds_of(view, subject);
    if units <= 0.0 {
        return None;
    }
    Some(Order { party: view.self_id(), side: Side::Sell, price: Some(at_least), qty: units as i64 })
}

/// A buyer's order, from what it can fund and what it will pay.
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

// THE ASSEMBLED WORLD HAS NO FIXTURE, because `world:runs` steps the real one four periods every
// time it is run: 51 systems, 54 phases, 58,354 asks, 1.9M events, and the audit reporting by
// family with an owner and a size. Sixteen tests here built three or four parties and asked
// whether a module can strike a relation, whether a world with no books steps, whether a party
// short of money brings paper, whether a wired system's proposal is settled — each of which the
// assembled world answers at scale, arranged by nobody.
//
// What the kernel REFUSES it refuses at the site: a party admitted at a bank that issues no money,
// a book that delivers money, a phase that anchors to nothing, a system wired twice, a system that
// neither works nor posts. Each panics with its clause and none needs a world to state.
//
// The homeless-noun count is a CENSUS, printed by `world:runs` every run. The fixture that guarded
// it had become a list of items already closed — `registry.currencies has a home now` — which is
// history, and the live claim is the one in CLAUDE.md: a count of zero would be the measure
// switched off.
//
// What is left here that a family should ask instead: a party whose liabilities exceed its assets
// ceases, and the one exception is a consequence rather than a rule. That is 0n.5's, where the
// Accounts family asks it of every party every period.
