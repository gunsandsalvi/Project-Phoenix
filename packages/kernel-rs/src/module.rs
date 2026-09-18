//! The three doors a module reaches the kernel through, and nothing else (ARCHITECTURE 4.9b).
//!
//! A module is one spec system, instrument family or seed. It declares its kinds and profiles, its
//! units, its params, its phases, its participants, its audit contributions and its seed
//! contribution — and it **never imports another module**, never writes the register, a print or a
//! weight. Settlement, the markets and the cell events are the one writer of each (Law 4).
//!
//! **Observer A4: a participant sees its OWN state and the public state, and that is a fact about
//! the type here rather than a discipline.** A `ParticipantView` is built for one party and has no
//! way to name another: `holdings()` walks that party's rows, `quantity` takes an instrument and
//! not a holder. A module cannot read a rival's book by accident, and the test below is what says
//! so — where TypeScript could only pass a context and hope.

use crate::ids::{HoldingId, InstrumentId, MarketId, PartyId, VenueId};
use crate::journal::Journal;
use crate::params::Params;
use crate::prices::{Print, Prints};
use crate::register::{Lot, Register};

/// Observer A1–A4: ONE PARTY'S own state and the public state. Built for a party, and there is no
/// door on it that takes another party's id.
pub struct ParticipantView<'a> {
    who: PartyId,
    register: &'a Register,
    prints: &'a Prints,
    journal: &'a Journal,
    params: &'a Params,
    period: u32,
}

impl<'a> ParticipantView<'a> {
    pub fn of(
        who: PartyId,
        register: &'a Register,
        prints: &'a Prints,
        journal: &'a Journal,
        params: &'a Params,
        period: u32,
    ) -> Self {
        Self { who, register, prints, journal, params, period }
    }

    /// Whose view this is. It is a read, and the only party this view can be about.
    pub fn self_id(&self) -> PartyId {
        self.who
    }

    pub fn period(&self) -> u32 {
        self.period
    }

    pub fn params(&self) -> &Params {
        self.params
    }

    /// A2: its OWN holdings, as rows. There is no argument that could make this somebody else's.
    pub fn holdings(&self) -> impl Iterator<Item = HoldingId> + '_ {
        self.register.of_holder(self.who).iter().map(|&row| HoldingId(row))
    }

    /// Law 8: what IT holds of a line, in whole pieces. The holder is not a parameter.
    pub fn quantity(&self, instrument: InstrumentId) -> f64 {
        self.register.quantity(self.register.row(self.who, instrument))
    }

    /// Register C3: and what of it is not encumbered.
    pub fn free(&self, instrument: InstrumentId) -> f64 {
        self.register.free(self.register.row(self.who, instrument))
    }

    pub fn lots(&self, instrument: InstrumentId) -> &[Lot] {
        self.register.lots(self.register.row(self.who, instrument))
    }

    /// Which line one of its own holdings is of. A view walks its rows and reads the line off
    /// them; asking every line in the world whether it holds one is the walk this replaces.
    pub fn line_of(&self, row: HoldingId) -> InstrumentId {
        self.register.instrument_of(row)
    }

    /// A3: what a BOOK printed is public — anybody may read it, which is what a price is for.
    pub fn print(&self, instrument: InstrumentId) -> Option<Print> {
        self.prints.latest(instrument, self.period)
    }

    /// A3: and the public record. A private event reaches only its subjects, and this is the door
    /// that keeps that true rather than a convention.
    pub fn public_event(&self, row: u32) -> bool {
        self.journal.is_public(row)
            || self.journal.subjects_of(row).contains(&self.who.0)
    }

    /// Law 18: the versions of what this view reads, for a caller keeping an answer across a walk.
    /// They are not facts about the world and no decision may be taken from them.
    pub fn versions(&self) -> (u64, u64) {
        (self.register.version(), self.prints.version())
    }
}

/// Clearing B2: a participant's reason to be in a market, evaluated per party with only that
/// party's own view — a schedule cannot be written against something the party may not see.
pub trait Participant {
    /// Which kind of party is asked.
    fn party_kind(&self) -> u32;

    /// Law 18, Clearing B2: WHICH BOOKS THIS PARTY COULD BE IN AT ALL THIS CYCLE.
    ///
    /// **It is required.** In TypeScript it was optional and absent meant every book of the kind,
    /// so the quadratic was the DEFAULT and nothing said which declarations were taking it — which
    /// is what `work.ts` had to be built to find out. A book open to everybody says so through
    /// `everyone` below, which is one question about the BOOK rather than one per party.
    ///
    /// It is a READ and never a second copy (Law 19): a participant answers out of the same thing
    /// its `orders` answers out of, so a book it names here and a book it posts in cannot disagree.
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId>;

    /// A book every party of the kind is asked about, whatever `markets` said — a fact about the
    /// BOOK and not about any party, settled once for the book.
    fn everyone(&self, _market: MarketId) -> bool {
        false
    }

    fn orders(&self, view: &ParticipantView<'_>, market: MarketId) -> Vec<crate::clearing::Order>;
}

/// The same door for a VENUE, where what is struck is not the transfer of an instrument.
pub trait VenueParticipant {
    fn party_kind(&self) -> u32;
    /// Required, for the reason `Participant::markets` is.
    fn venues(&self, view: &ParticipantView<'_>) -> Vec<VenueId>;
    fn orders(&self, view: &ParticipantView<'_>, venue: VenueId) -> Vec<crate::clearing::Order>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::{Calendar, Day};
    use crate::ids::CurrencyCode;
    use crate::prices::{Provenance, QuotedAs};

    fn world() -> (Register, Prints, Journal, Params) {
        (Register::new(), Prints::new(), Journal::new(), Params::new(100.0, 60.0))
    }

    #[test]
    fn a_participant_sees_its_own_book_and_has_no_door_onto_anothers() {
        let (mut reg, prints, journal, params) = world();
        let me = PartyId::at(0);
        let rival = PartyId::at(1);
        let line = InstrumentId::at(3);
        reg.credit(me, line, 10.0, 1.0, 1);
        reg.credit(rival, line, 999.0, 1.0, 1);

        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1);
        assert_eq!(view.quantity(line), 10.0);
        assert_eq!(view.holdings().count(), 1);
        // Observer A4: the rival holds 999 of the same line and this view cannot say so. There is
        // no argument to pass — `quantity` takes an instrument, never a holder — so reading
        // another party's book is not something a module can do by accident.
        let mine: Vec<f64> = view.holdings().map(|row| reg.quantity(row)).collect();
        assert_eq!(mine, vec![10.0]);
    }

    #[test]
    fn a_print_is_public_and_a_private_event_reaches_only_its_subjects() {
        let (reg, mut prints, mut journal, params) = world();
        let me = PartyId::at(0);
        let line = InstrumentId::at(3);
        prints.write(Print {
            instrument: line,
            market: MarketId::at(3),
            period: 1,
            price: 12.5,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        });
        let kind = journal.kinds.declare("bank.refused");
        let mine = journal.say(1, 0, kind, &[me.0], &[], false);
        let theirs = journal.say(1, 0, kind, &[PartyId::at(1).0], &[], false);
        let open = journal.say(1, 0, kind, &[], &[], true);

        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1);
        // A3: what a book printed is public — that is what a price is for.
        assert_eq!(view.print(line).unwrap().price, 12.5);
        // A private event reaches its subjects and the public record, and nobody else.
        assert!(view.public_event(mine));
        assert!(!view.public_event(theirs));
        assert!(view.public_event(open));
    }

    #[test]
    fn a_view_reads_the_world_live_and_says_when_what_it_read_has_moved() {
        let (mut reg, prints, journal, params) = world();
        let me = PartyId::at(0);
        let line = InstrumentId::at(3);
        reg.credit(me, line, 10.0, 1.0, 1);
        let before = {
            let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1);
            view.versions()
        };
        reg.credit(me, line, 5.0, 1.0, 1);
        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1);
        // Law 18: a kept answer is checked against these, and they have moved, so it is recomputed.
        assert_ne!(view.versions(), before);
        // And the view reads the register LIVE: a party that traded mid-cycle is seen to have.
        assert_eq!(view.quantity(line), 15.0);
        let _ = Calendar::new(Day(0), 7, 3);
    }
}
