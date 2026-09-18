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
use crate::ids::{CurrencyCode, InstrumentId, MarketId};
use crate::instruments::{Class, Instruments};
use crate::journal::Journal;
use crate::ledger::Settlement;
use crate::module::{Participant, ParticipantView};
use crate::params::Params;
use crate::parties::Parties;
use crate::prices::Prints;
use crate::register::Register;
use crate::session::{run_book, BookDecl, Books, Stores};
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
}

impl World {
    /// The world before anything has happened to it. The chronicle is what fills it (22b).
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

    /// One period: the books run, the fills settle, and what happened is returned. Law 19: the count
    /// is read off what the session did, never kept beside it.
    pub fn step(&mut self, systems: &[&dyn System]) -> Stepped {
        self.period += 1;
        let participants: Vec<&dyn Participant> =
            systems.iter().flat_map(|s| s.participants()).collect();
        if participants.is_empty() || self.books.is_empty() {
            return Stepped::default();
        }
        let books = Books::index(
            &participants,
            &self.parties,
            &self.register,
            &self.prints,
            &self.journal,
            &self.params,
            self.period,
        );
        let mut out = Stepped { narrows: books.narrows, ..Stepped::default() };
        let events_before = self.journal.len();
        for book in &self.books {
            let mut stores = Stores {
                parties: &self.parties,
                instruments: &self.instruments,
                register: &mut self.register,
                prints: &mut self.prints,
                journal: &mut self.journal,
                wire: &mut self.wire,
                params: &self.params,
            };
            let session = run_book(
                book,
                &participants,
                &books,
                &mut stores,
                self.period,
                self.settled_kind,
                self.failed_kind,
            );
            out.asks += session.asks;
            out.trades += session.settled;
            // Clearing C4: a book that had something cross is one that cleared; one that did not
            // prints nothing, and counting it would be a session that never happened.
            if matches!(session.outcome, crate::clearing::Outcome::Cleared { .. }) {
                out.books_cleared += 1;
            }
        }
        out.events = self.journal.len() - events_before;
        out
    }

    /// A book, declared. The subject is what it delivers and the cash is what buyers pay with — an
    /// instrument like any other (Money A2.b).
    pub fn open_book(&mut self, market: MarketId, subject: InstrumentId, ccy: CurrencyCode, cash: InstrumentId, rule: PriceRule) {
        assert!(
            self.instruments.class_of(cash) == Class::Money,
            "Money A2.b: a book's cash leg must be money somebody issued"
        );
        self.books.push(BookDecl { market, subject, ccy, cash, rule });
    }
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
    fn a_books_cash_leg_must_be_money_somebody_issued() {
        // Money A2.b, Appendix B: no money without an issuer, and a book paid for in something that
        // is not money is a conversion at the ledger boundary.
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let cash = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let share = w.instruments.issue(cb, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.open_book(MarketId::at(0), share, CurrencyCode::at(0), cash, PriceRule::SellersCompete);
        assert_eq!(w.books.len(), 1);
    }

    #[test]
    #[should_panic(expected = "must be money somebody issued")]
    fn a_book_paid_for_in_something_that_is_not_money_is_refused() {
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, crate::ids::RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let share = w.instruments.issue(cb, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.open_book(MarketId::at(0), share, CurrencyCode::at(0), share, PriceRule::SellersCompete);
    }
}
