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
use crate::instruments::Instruments;
use crate::journal::{Journal, Value};
use crate::ledger::{Cause, Delivery, Leg, Settlement};
use crate::parties::Parties;
use crate::params::Params;
use crate::prices::{Print, Prints};
use crate::register::{Lot, Register};
use crate::stores::{Agreements, Claims, DueId, Outlooks, Processes, Schedules};

/// Observer A1–A4: ONE PARTY'S own state and the public state. Built for a party, and there is no
/// door on it that takes another party's id.
pub struct ParticipantView<'a> {
    who: PartyId,
    register: &'a Register,
    prints: &'a Prints,
    journal: &'a Journal,
    params: &'a Params,
    period: u32,
    cash: Option<InstrumentId>,
    /// XI-10: **its own relations.** A mandate, an engagement, a policy is a fact about THIS party
    /// and nobody else's business, so it belongs in the view — and `of_party` is what keeps that
    /// true: there is no argument here that could make it somebody else's (Observer A4). A view
    /// built without it answers *no relations*, which is what a caller that has none can say.
    agreements: Option<&'a Agreements>,
}

impl<'a> ParticipantView<'a> {
    pub fn of(
        who: PartyId,
        register: &'a Register,
        prints: &'a Prints,
        journal: &'a Journal,
        params: &'a Params,
        period: u32,
        cash: Option<InstrumentId>,
    ) -> Self {
        Self { who, register, prints, journal, params, period, cash, agreements: None }
    }

    /// The same view, able to answer what this party has AGREED. It is a second constructor rather
    /// than an eighth argument because most callers have no relations to hand and saying `None`
    /// eight times is how a caller ends up passing the wrong one.
    pub fn knowing(mut self, agreements: &'a Agreements) -> Self {
        self.agreements = Some(agreements);
        self
    }

    /// XI-10: **its own live relations of one kind** — the mandate a pool is run under, the
    /// engagements an employer holds. A view with no agreements answers none, which is an answer: a
    /// caller that cannot see relations must not read that as a party having none, and the two are
    /// told apart by whoever built the view rather than here.
    pub fn agreed(&self, kind: u32) -> Vec<crate::stores::AgreementId> {
        let Some(all) = self.agreements else { return Vec::new() };
        all.of_party(self.who)
            .iter()
            .map(|r| crate::stores::AgreementId(*r))
            .filter(|a| all.live(*a) && all.kind_of(*a) == kind)
            .collect()
    }

    /// Money D2: **the account this party pays out of** — resolved from the banking lattice when the
    /// view was built (`ledger::account_of`), not handed to a participant as a declaration. A world
    /// where every bank issues its own deposits has no single "the cash", and a participant told
    /// which line to look at is a participant looking at somebody else's money.
    ///
    /// `Missing` is missing: a party that banks nowhere and issues nothing has no account.
    pub fn cash(&self) -> Option<InstrumentId> {
        self.cash
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

    /// Money D2, 22b.9a: **what it holds of its OWN account**, free of liens. A party that banks
    /// nowhere and issues nothing holds no money, which is nothing rather than zero of something.
    pub fn own_cash(&self) -> f64 {
        match self.cash {
            Some(line) => self.free(line),
            None => 0.0,
        }
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

/// **THE SECOND DOOR** (ARCHITECTURE 4.9b): what a module is given when its phase runs.
///
/// @spec ARCHITECTURE 4.9b · Law 4 · Law 10 · Law 19 · Appendix B
///
/// A participant is asked a question inside a book; a MECHANISM does the rest of what a system does
/// in a period — accruing, maturing, deciding, publishing. It reads the kernel's stores and **it
/// proposes**: it cannot write the register, a print or a weight, because settlement, the markets
/// and the cell events are the one writer of each (Law 4).
///
/// **Proposing rather than writing is what makes that true by the type** rather than by discipline.
/// A module hands back instructions and events; the kernel settles them after the phase returns, so
/// a module that wanted to move units without a counterparty would have to write a leg that has
/// one, and the wire refuses the rest.
pub struct MechanismContext<'a> {
    period: u32,
    parties: &'a Parties,
    instruments: &'a Instruments,
    register: &'a Register,
    prints: &'a Prints,
    journal: &'a Journal,
    params: &'a Params,
    agreements: &'a Agreements,
    schedules: &'a Schedules,
    outlooks: &'a Outlooks,
    processes: &'a Processes,
    claims: &'a Claims,
    standing: &'a crate::stores::Standing,
    making: &'a crate::stores::InProgress,
    registry: &'a crate::registry::Registry,
    wire: &'a Settlement,
    proposed: Vec<Proposed>,
    said: Vec<Saying>,
    formed: Vec<(PartyId, u32, f64)>,
    settled: Vec<DueId>,
    ceased: Vec<PartyId>,
    claimed: Vec<(PartyId, PartyId, f64, u32)>,
    repaid: Vec<(crate::stores::ClaimId, f64)>,
    started: Vec<(PartyId, InstrumentId, f64, f64, u32)>,
    finished: Vec<crate::stores::BatchId>,
    stood: Vec<(u32, PartyId, Vec<f64>)>,
    split: Vec<(PartyId, u32, crate::stores::AgreementId)>,
}

/// One thing a module asks the world to do. It is a two-sided instruction like any other (Law 5) and
/// it carries WHY, because an instruction nobody can read back is a state change with a date on it.
pub struct Proposed {
    pub legs: Vec<Leg>,
    pub cause: Cause,
    pub delivery: Delivery,
    pub why: &'static str,
}

/// Audit A2: something a module states happened, for whoever it happened to. A private event reaches
/// only its subjects; a public one is a fact about the world anybody may read (Observer A3).
pub struct Saying {
    pub kind: u32,
    pub subjects: Vec<u32>,
    pub data: Vec<(u32, Value)>,
    pub public: bool,
}

/// Every store a mechanism may read, handed over together — because a phase is given the world as it
/// stands, and a caller made to name ten is a caller that will one day name nine.
pub struct Stores<'a> {
    pub parties: &'a Parties,
    pub instruments: &'a Instruments,
    pub register: &'a Register,
    pub prints: &'a Prints,
    pub journal: &'a Journal,
    pub params: &'a Params,
    pub agreements: &'a Agreements,
    pub schedules: &'a Schedules,
    pub outlooks: &'a Outlooks,
    pub processes: &'a Processes,
    /// XI-8: who is owed what by a dead party, and at what rank.
    pub claims: &'a Claims,
    /// Money D1: **the wire IS the history.** A mechanism may READ what happened — what it itself
    /// delivered last period, what anybody delivered — and it may not write it: settlement stays the
    /// one writer. Without this a module that needs its own past has to infer it by subtraction,
    /// which is exactly what Law 19 forbids.
    pub wire: &'a Settlement,
    /// 21f: terms parties stand behind, and what is on the line.
    pub standing: &'a crate::stores::Standing,
    pub making: &'a crate::stores::InProgress,
    /// 21i: what the ids point at — a line's footprint, a region's country, a kind's profile. Read-only
    /// like every other store here: the registry is DATA (Law 15) and the assembly is its writer.
    pub registry: &'a crate::registry::Registry,
}

impl<'a> MechanismContext<'a> {
    pub fn of(period: u32, s: Stores<'a>) -> Self {
        Self {
            period,
            parties: s.parties,
            instruments: s.instruments,
            register: s.register,
            prints: s.prints,
            journal: s.journal,
            params: s.params,
            agreements: s.agreements,
            schedules: s.schedules,
            outlooks: s.outlooks,
            processes: s.processes,
            claims: s.claims,
            wire: s.wire,
            standing: s.standing,
            making: s.making,
            registry: s.registry,
            proposed: Vec::new(),
            said: Vec::new(),
            formed: Vec::new(),
            settled: Vec::new(),
            ceased: Vec::new(),
            claimed: Vec::new(),
            repaid: Vec::new(),
            started: Vec::new(),
            finished: Vec::new(),
            stood: Vec::new(),
            split: Vec::new(),
        }
    }

    /// 21f: what parties stand behind — a posting, a lending standard — and what is on the line.
    pub fn standing(&self) -> &crate::stores::Standing {
        self.standing
    }

    /// 21i, Law 15: the DATA every id points at — a line's footprint, a region's country. A module
    /// reads it and never writes it, which is what keeps a kind out of a mechanism.
    pub fn registry(&self) -> &crate::registry::Registry {
        self.registry
    }

    pub fn making(&self) -> &crate::stores::InProgress {
        self.making
    }

    pub fn period(&self) -> u32 {
        self.period
    }

    pub fn parties(&self) -> &Parties {
        self.parties
    }

    pub fn instruments(&self) -> &Instruments {
        self.instruments
    }

    pub fn register(&self) -> &Register {
        self.register
    }

    pub fn prints(&self) -> &Prints {
        self.prints
    }

    pub fn journal(&self) -> &Journal {
        self.journal
    }

    pub fn params(&self) -> &Params {
        self.params
    }

    /// XI-10, 17f: the relations this party is in — an engagement, a mortgage, a policy.
    pub fn agreements(&self) -> &Agreements {
        self.agreements
    }

    /// 5 D2: what each instrument owes and when.
    pub fn schedules(&self) -> &Schedules {
        self.schedules
    }

    /// §46: what each party expects. They disagree, and that is the point.
    pub fn outlooks(&self) -> &Outlooks {
        self.outlooks
    }

    /// What is in flight across periods.
    pub fn processes(&self) -> &Processes {
        self.processes
    }

    /// XI-8: who is owed what by an estate, to READ. A module asks for a claim through `claims()`;
    /// this is the other direction.
    pub fn claims(&self) -> &Claims {
        self.claims
    }

    /// Money D1: **what actually happened**, to read and never to write. A module that needs its own
    /// past reads it here rather than inferring it from a balance that moved (Law 19).
    pub fn wire(&self) -> &Settlement {
        self.wire
    }

    /// Law 5: what it asks the world to do. Both legs or it is not a flow.
    pub fn propose(&mut self, legs: Vec<Leg>, cause: Cause, delivery: Delivery, why: &'static str) {
        assert!(!why.is_empty(), "4.9b: an instruction with no reason is a state change with a date on it");
        self.proposed.push(Proposed { legs, cause, delivery, why });
    }

    /// Audit A2: something it states happened, for whoever it happened to. A private event reaches
    /// only its subjects — the journal is what keeps that true, not the caller's good manners.
    pub fn say(&mut self, kind: u32, subjects: &[u32], data: &[(u32, Value)], public: bool) {
        self.said.push(Saying {
            kind,
            subjects: subjects.to_vec(),
            data: data.to_vec(),
            public,
        });
    }

    /// §46 B1: an outlook it formed from ITS OWN history. The forming is the module's — how much
    /// weight to give the surprise is that party's own preference — and this records the answer.
    pub fn form(&mut self, party: PartyId, about: u32, level: f64) {
        self.formed.push((party, about, level));
    }

    /// A-20: what it paid off the schedule. Marked once the instruction it proposed has settled, so
    /// a payment that failed leaves the arrear standing rather than clearing it.
    pub fn settles(&mut self, due: DueId) {
        self.settled.push(due);
    }

    /// **XI-3: NOTHING IS IMMORTAL, and a thing that ends says when.** A module asks; the kernel
    /// writes, because `Parties` is the one writer of who is alive (Law 4). What becomes of what it
    /// held is not decided here — that is the estate's, and a party ceasing with holdings is a
    /// finding for whoever is looking, never a quantity this door disposes of.
    pub fn ceases(&mut self, who: PartyId) {
        self.ceased.push(who);
    }

    /// **XI-8, Money E1: what somebody is OWED by a party whose life has ended.** A module asks; the
    /// kernel writes, because `Claims` is the one writer of who is owed what by an estate.
    ///
    /// It is the door 21c exists for. An assessment on a dead party that reached the wire as a
    /// PAYMENT was the state jumping the queue ahead of the creditors the estate exists to pay; a
    /// claim queues, and the waterfall pays it where its rank puts it.
    pub fn is_owed(&mut self, on: PartyId, holder: PartyId, owed: f64, ranks: u32) {
        self.claimed.push((on, holder, owed, ranks));
    }

    /// **XI-8, 21.36: and what an estate PAID one.** A claim that is paid and not marked comes back
    /// whole next period and is paid again, for ever — which is the shape 21.36 measured on the old
    /// engine, where a dead firm's debt stood twice on the register and grew by every claim every
    /// period. `Claims::pays` is the one writer; a module says what it paid.
    pub fn pays(&mut self, claim: crate::stores::ClaimId, amount: f64) {
        self.repaid.push((claim, amount));
    }

    /// **§37 B3, 21f.3: a batch goes ON the line**, owned, carrying what it cost, ready in a later
    /// period. `InProgress` is the one writer; a module says what it started.
    pub fn starts(&mut self, owner: PartyId, what: InstrumentId, units: f64, cost: f64, ready: u32) {
        self.started.push((owner, what, units, cost, ready));
    }

    /// And a batch comes OFF it. The mark and the `Create` leg are one event with two sides (Law 5),
    /// so a module proposes both in the same phase and the kernel applies both.
    pub fn finishes(&mut self, batch: crate::stores::BatchId) {
        self.finished.push(batch);
    }

    /// **What this party stands behind**, until it withdraws it (21f.1, 21f.2): a posting, a lending
    /// standard. `Standing` is the one writer.
    pub fn now_stands(&mut self, kind: u32, who: PartyId, terms: Vec<f64>) {
        self.stood.push((kind, who, terms));
    }

    /// **XI-15, Labour A4.c: an event that applies to SOME of a cell splits it**, and the relationship
    /// that applies to them goes with them. A module names the members and the relation; the kernel
    /// makes the cell, moves their exact share of what the parent holds, carries their outlook, and
    /// re-points the agreement.
    ///
    /// The share is the kernel's arithmetic and not the module's, because it must be EXACT: a cell is
    /// homogeneous, so its holdings divide by its weight without remainder, and a module computing
    /// that division is a second writer of the one thing that keeps a cell from becoming an average.
    pub fn splits(&mut self, cell: PartyId, taking: u32, carrying: crate::stores::AgreementId) {
        self.split.push((cell, taking, carrying));
    }


    /// What the kernel applies once the phase returns.
    pub fn taken(self) -> Taken {
        Taken {
            proposed: self.proposed,
            said: self.said,
            formed: self.formed,
            settled: self.settled,
            ceased: self.ceased,
            claimed: self.claimed,
            repaid: self.repaid,
            started: self.started,
            finished: self.finished,
            stood: self.stood,
            split: self.split,
        }
    }
}

/// Everything one phase asked for, handed back for the kernel to apply. Nothing here has happened
/// yet — which is what makes settlement, the journal and the cell events the one writer of each.
pub struct Taken {
    pub proposed: Vec<Proposed>,
    pub said: Vec<Saying>,
    pub formed: Vec<(PartyId, u32, f64)>,
    pub settled: Vec<DueId>,
    /// XI-3: the parties whose life ended in this phase. What HAPPENS to what they held is the
    /// estate's; this records only that they have ceased, and the kernel is the one writer of it.
    pub ceased: Vec<PartyId>,
    /// XI-8: who is owed what by a dead party, and at what rank. A module asks; `Claims` is the
    /// one writer, for the reason every other store has one.
    pub claimed: Vec<(PartyId, PartyId, f64, u32)>,
    /// XI-8, 21.36: and what an estate actually PAID one, so the claim comes down. A claim that is
    /// paid and not marked is a claim that is paid again next period, for ever.
    pub repaid: Vec<(crate::stores::ClaimId, f64)>,
    /// §37 B3, 21f.3: batches that went ON the line this phase — owner, what, how much, what it
    /// cost, and the period it is ready.
    pub started: Vec<(PartyId, InstrumentId, f64, f64, u32)>,
    /// And the batches taken off it, whose `Create` legs are in `proposed` (Law 5: one event).
    pub finished: Vec<crate::stores::BatchId>,
    /// 21f.1, 21f.2: terms a party now stands behind. The kernel writes `Standing`.
    pub stood: Vec<(u32, PartyId, Vec<f64>)>,
    /// XI-15, 21h: cells that an event applies to part of — the parent, how many members it takes,
    /// and the relationship those members carry with them. `Parties` is the one writer of a weight.
    pub split: Vec<(PartyId, u32, crate::stores::AgreementId)>,
}

/// A system's own work in a period, as opposed to the questions its participants are asked in books.
///
/// Every module has one; most do something every period and some do something only on a date. A
/// mechanism that has nothing to do this period proposes nothing, which is an answer and not a gap.
pub trait Mechanism {
    fn run(&self, ctx: &mut MechanismContext<'_>);
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

        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1, None);
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

        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1, None);
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
            let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1, None);
            view.versions()
        };
        reg.credit(me, line, 5.0, 1.0, 1);
        let view = ParticipantView::of(me, &reg, &prints, &journal, &params, 1, None);
        // Law 18: a kept answer is checked against these, and they have moved, so it is recomputed.
        assert_ne!(view.versions(), before);
        // And the view reads the register LIVE: a party that traded mid-cycle is seen to have.
        assert_eq!(view.quantity(line), 15.0);
        let _ = Calendar::new(Day(0), 7, 3);
    }
}
