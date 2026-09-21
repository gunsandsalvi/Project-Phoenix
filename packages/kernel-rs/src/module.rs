//! The three doors a module reaches the kernel through, and nothing else (ARCHITECTURE 4.9b).

use crate::calendar::Week;
use crate::ids::{HoldingId, InstrumentId, MarketId, PartyId, VenueId};
use crate::instruments::Instruments;
use crate::journal::{Journal, Value};
use crate::ledger::{Cause, Delivery, Leg, Settlement};
use crate::params::Params;
use crate::parties::Parties;
use crate::prices::{Print, Prints};
use crate::register::{Lot, Register};
use crate::stores::{Agreements, Claims, DueId, Outlooks, Processes, Schedules};

/// Whether an event reaches this party: it is public, or the party is one of its subjects. Nothing
/// else can see it.
pub fn reaches(public: bool, subjects: &[u32], me: PartyId) -> bool {
    public || subjects.contains(&me.0)
}

/// ONE PARTY'S own state and the public state. It borrows the stores rather than copying them, so
/// what it answers is live: a party that traded in an earlier book is seen to have.
pub struct ParticipantView<'a> {
    who: PartyId,
    register: &'a Register,
    instruments: &'a Instruments,
    prints: &'a Prints,
    journal: &'a Journal,
    params: &'a Params,
    week: u32,
    cash: Option<InstrumentId>,
    /// Its own relations.
    agreements: Option<&'a Agreements>,
    /// What it owes and is owed, by date.
    schedules: Option<&'a Schedules>,
    /// 3 C2, 22c.2: what it is already standing behind in a venue.
    resting: Option<&'a crate::stores::Resting>,
    /// What it has in flight — its OWN.
    processes: Option<&'a Processes>,
    /// Durable terms this party stands behind, including its own mandates.
    standing: Option<&'a crate::stores::Standing>,
    outlooks: &'a Outlooks,
    outlook_memory: f64,
    population_weight: u32,
    household_keeps: Option<f64>,
    /// THE ONE CALENDAR, so a party reads what week it is rather than multiplying out its own.
    calendar: &'a crate::calendar::Calendar,
    books: &'a [crate::session::BookDecl],
}

/// The shared stores needed to form one participant's basic view.
pub struct ViewInputs<'a> {
    pub register: &'a Register,
    pub instruments: &'a Instruments,
    pub prints: &'a Prints,
    pub journal: &'a Journal,
    pub params: &'a Params,
    pub week: u32,
    pub cash: Option<InstrumentId>,
    pub calendar: &'a crate::calendar::Calendar,
    pub outlooks: &'a Outlooks,
    pub outlook_memory: f64,
    pub population_weight: u32,
    pub household_keeps: Option<f64>,
    pub books: &'a [crate::session::BookDecl],
}

impl<'a> ParticipantView<'a> {
    pub fn of(who: PartyId, inputs: ViewInputs<'a>) -> Self {
        Self {
            who,
            register: inputs.register,
            instruments: inputs.instruments,
            prints: inputs.prints,
            journal: inputs.journal,
            params: inputs.params,
            week: inputs.week,
            cash: inputs.cash,
            calendar: inputs.calendar,
            agreements: None,
            schedules: None,
            resting: None,
            processes: None,
            standing: None,
            outlooks: inputs.outlooks,
            outlook_memory: inputs.outlook_memory,
            population_weight: inputs.population_weight,
            household_keeps: inputs.household_keeps,
            books: inputs.books,
        }
    }

    /// This party's own expectation, never a global forecast or another party's view.
    pub fn outlook(&self, about: u32) -> Option<f64> {
        self.outlooks.of(self.who, about)
    }

    pub fn price_outlook(&self, line: InstrumentId) -> Option<f64> {
        self.outlook(crate::stores::about::price_of(line))
    }

    pub fn subject_of(&self, market: MarketId) -> Option<InstrumentId> {
        crate::session::declared_subject(self.books, market)
    }

    pub fn market_of(&self, subject: InstrumentId) -> Option<MarketId> {
        crate::session::declared_market(self.books, subject)
    }

    pub fn venue_of(&self, market: MarketId) -> Option<crate::protocols::Venue> {
        crate::session::declared_venue(self.books, market)
    }

    pub fn confidence(&self, about: u32) -> Option<f64> {
        self.outlooks
            .confidence(self.who, about, self.outlook_memory)
    }

    pub fn household_keeps(&self) -> Option<f64> {
        self.household_keeps
    }

    /// Number of identical members represented by this party. Named parties have weight one.
    pub fn population_weight(&self) -> u32 {
        self.population_weight
    }

    /// The same view, able to answer what falls due for it and to it.
    pub fn owing(mut self, schedules: &'a Schedules) -> Self {
        self.schedules = Some(schedules);
        self
    }

    /// 3 C2, 22c.2: and able to answer what it already has RESTING in a venue.
    pub fn resting_in(mut self, resting: &'a crate::stores::Resting) -> Self {
        self.resting = Some(resting);
        self
    }

    /// And able to answer what it has in flight.
    pub fn afoot(mut self, processes: &'a Processes) -> Self {
        self.processes = Some(processes);
        self
    }

    /// Make durable party-owned mandates visible to this party. The lookup is deliberately scoped
    /// to `self`: a participant cannot inspect another party's private mandate through this door.
    pub fn standing_by(mut self, standing: &'a crate::stores::Standing) -> Self {
        self.standing = Some(standing);
        self
    }

    pub fn own_mandate(&self, kind: u32) -> Option<&[f64]> {
        let all = self.standing?;
        let row = all.of_party_about(self.who, self.who, kind)?;
        Some(all.terms(row))
    }

    /// How much this party has been put in a workout for, and zero where it is in none — which is
    /// not a party with a workout of nothing, because a workout of nothing is never opened.
    pub fn in_a_flotation(&self) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|r| crate::stores::ProcessId(*r))
            .filter(|p| !all.done(*p) && all.kind_of(*p) == crate::stores::afoot::FLOTATION)
            .map(|p| all.size(p))
            .sum()
    }

    pub fn flotation_lines(&self) -> Vec<InstrumentId> {
        let Some(all) = self.processes else {
            return Vec::new();
        };
        all.of_owner(self.who)
            .iter()
            .map(|row| crate::stores::ProcessId(*row))
            .filter(|process| {
                !all.done(*process) && all.kind_of(*process) == crate::stores::afoot::FLOTATION
            })
            .filter_map(|process| all.subject(process))
            .collect()
    }

    pub fn flotation_on(&self, line: InstrumentId) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|row| crate::stores::ProcessId(*row))
            .filter(|process| {
                !all.done(*process)
                    && all.kind_of(*process) == crate::stores::afoot::FLOTATION
                    && all.subject(*process) == Some(line)
            })
            .map(|process| all.size(process))
            .sum()
    }

    /// How much money this firm has committed to a capital programme, and zero where it has none
    /// afoot.
    pub fn in_a_programme(&self) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|r| crate::stores::ProcessId(*r))
            .filter(|p| !all.done(*p) && all.kind_of(*p) == crate::stores::afoot::CAPITAL_PROGRAMME)
            .map(|p| all.size(p))
            .sum()
    }

    /// The named capital lines this party is currently authorised to buy. A programme without a
    /// subject cannot become an order for an arbitrary asset.
    pub fn programme_markets(&self) -> Vec<crate::ids::MarketId> {
        let Some(all) = self.processes else {
            return Vec::new();
        };
        all.of_owner(self.who)
            .iter()
            .map(|row| crate::stores::ProcessId(*row))
            .filter(|process| {
                !all.done(*process)
                    && all.kind_of(*process) == crate::stores::afoot::CAPITAL_PROGRAMME
            })
            .filter_map(|process| all.subject(process).and_then(|line| self.market_of(line)))
            .collect()
    }

    /// The money still committed to this particular capital line.
    pub fn programme_on(&self, line: InstrumentId) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|row| crate::stores::ProcessId(*row))
            .filter(|process| {
                !all.done(*process)
                    && all.kind_of(*process) == crate::stores::afoot::CAPITAL_PROGRAMME
                    && all.subject(*process) == Some(line)
            })
            .map(|process| all.size(process))
            .sum()
    }

    pub fn in_a_workout(&self) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|r| crate::stores::ProcessId(*r))
            .filter(|p| !all.done(*p) && all.kind_of(*p) == crate::stores::afoot::WORKOUT)
            .map(|p| all.size(p))
            .sum()
    }

    /// The units this party is required to sell from this particular line. Workouts without a
    /// subject are legacy monetary requirements and cannot be turned into an arbitrary asset sale.
    pub fn workout_on(&self, line: InstrumentId) -> f64 {
        let Some(all) = self.processes else {
            return 0.0;
        };
        all.of_owner(self.who)
            .iter()
            .map(|r| crate::stores::ProcessId(*r))
            .filter(|p| {
                !all.done(*p)
                    && all.kind_of(*p) == crate::stores::afoot::WORKOUT
                    && all.subject(*p) == Some(line)
            })
            .map(|p| all.size(p))
            .sum()
    }

    pub fn workout_lines(&self) -> Vec<InstrumentId> {
        let Some(all) = self.processes else {
            return Vec::new();
        };
        let mut lines: Vec<InstrumentId> = all
            .of_owner(self.who)
            .iter()
            .map(|r| crate::stores::ProcessId(*r))
            .filter(|p| !all.done(*p) && all.kind_of(*p) == crate::stores::afoot::WORKOUT)
            .filter_map(|p| all.subject(p))
            .collect();
        lines.sort_by_key(|line| line.0);
        lines.dedup();
        lines
    }

    /// 3 C2, 22c.2: WHAT THIS PARTY IS ALREADY STANDING BEHIND, in one venue, as a count of pieces
    /// on each side.
    pub fn resting(&self, venue: MarketId) -> (i64, i64) {
        let Some(all) = self.resting else {
            return (0, 0);
        };
        let mut buying = 0i64;
        let mut selling = 0i64;
        for o in all.of_party(self.who) {
            if all.venue_of(o) != venue.0 {
                continue;
            }
            if all.buying(o) {
                buying += all.left(o);
            } else {
                selling += all.left(o);
            }
        }
        (buying, selling)
    }

    /// 3 C2, 22c2.3: its own orders in one venue, one by one, so a party that wants to withdraw one
    /// can name it.
    pub fn standing(&self, venue: MarketId) -> Vec<crate::stores::RestingId> {
        let Some(all) = self.resting else {
            return Vec::new();
        };
        all.of_party(self.who)
            .into_iter()
            .filter(|o| all.venue_of(*o) == venue.0)
            .collect()
    }

    pub fn left_of(&self, o: crate::stores::RestingId) -> i64 {
        match self.resting {
            Some(all) if all.owner(o) == self.who => all.left(o),
            _ => 0,
        }
    }

    /// The same view, able to answer what this party has AGREED.
    pub fn knowing(mut self, agreements: &'a Agreements) -> Self {
        self.agreements = Some(agreements);
        self
    }

    /// Its own live relations of one kind — the mandate a pool is run under, the engagements an
    /// employer holds.
    pub fn agreed(&self, kind: u32) -> Vec<crate::stores::AgreementId> {
        let Some(all) = self.agreements else {
            return Vec::new();
        };
        all.of_party(self.who)
            .iter()
            .map(|r| crate::stores::AgreementId(*r))
            .filter(|a| all.live(*a) && all.kind_of(*a) == kind)
            .collect()
    }

    /// The account this party pays out of — resolved from the banking lattice when the view was
    /// built (`ledger::account_of`), not handed to a participant as a declaration.
    pub fn cash(&self) -> Option<InstrumentId> {
        self.cash
    }

    /// Whose view this is.
    pub fn self_id(&self) -> PartyId {
        self.who
    }

    /// Lines this party issued. A participant may discover its own newly created paper without a
    /// wiring-time copy of instrument ids that goes stale as soon as another dated line is issued.
    pub fn own_issues(&self) -> impl Iterator<Item = InstrumentId> + '_ {
        self.instruments
            .of_issuer(self.who)
            .iter()
            .map(|row| InstrumentId::at(*row))
    }

    pub fn week(&self) -> u32 {
        self.week
    }

    /// THE CURRENT WEEK. A week settles once (Money G1), so it is the only day a party
    /// deciding in it has.
    pub fn today(&self) -> crate::calendar::Week {
        self.calendar
            .at(crate::calendar::Week(i64::from(self.week)))
    }

    /// And the current week it covers, so what a party must find this week is read at both ends.
    pub fn current_week(&self) -> crate::calendar::Week {
        crate::calendar::Week(i64::from(self.week))
    }

    pub fn params(&self) -> &Params {
        self.params
    }

    /// Its OWN holdings, as rows.
    pub fn holdings(&self) -> impl Iterator<Item = HoldingId> + '_ {
        self.register
            .of_holder(self.who)
            .iter()
            .map(|&row| HoldingId(row))
    }

    /// What IT holds of a line. There is no holder argument, so reading another party's book is not
    /// something a module can do by accident.
    pub fn quantity(&self, instrument: InstrumentId) -> f64 {
        self.register
            .quantity(self.register.row(self.who, instrument))
    }

    /// What it holds of its OWN account, free of liens.
    pub fn own_cash(&self) -> f64 {
        match self.cash {
            Some(line) => self.free(line),
            None => 0.0,
        }
    }

    /// The live desk capital visible from this party's own settled holdings and issued liabilities.
    pub fn own_booked_equity(&self) -> Option<f64> {
        let mut assets = 0.0;
        for row in self.register.of_holder(self.who) {
            assets += crate::instruments::carrying_value(
                HoldingId(*row),
                self.register,
                self.instruments,
                self.prints,
                self.week,
            )?;
        }
        let liabilities =
            crate::instruments::owed_by(self.who, self.instruments, |line| {
                match self.instruments.class_of(line) {
                    crate::instruments::Class::Money | crate::instruments::Class::Claim => {
                        let (held, _) = self.register.held_total(line);
                        held - self.register.quantity(self.register.row(self.who, line))
                    }
                    _ => 0.0,
                }
            });
        Some(assets - liabilities)
    }

    /// And what of it is not encumbered.
    pub fn free(&self, instrument: InstrumentId) -> f64 {
        self.register.free(self.register.row(self.who, instrument))
    }

    pub fn lots(&self, instrument: InstrumentId) -> &[Lot] {
        self.register.lots(self.register.row(self.who, instrument))
    }

    /// Which line one of its own holdings is of.
    pub fn line_of(&self, row: HoldingId) -> InstrumentId {
        self.register.instrument_of(row)
    }

    /// What a BOOK printed is public — anybody may read it, which is what a price is for.
    pub fn print(&self, instrument: InstrumentId) -> Option<Print> {
        self.prints.latest(instrument, self.week)
    }

    pub fn public_event(&self, row: u32) -> bool {
        reaches(
            self.journal.is_public(row),
            self.journal.subjects_of(row),
            self.who,
        )
    }

    /// Whether a recorded event of this kind reaches this party. Participants may react to public
    /// history without receiving a door that can inspect or mutate the journal wholesale.
    pub fn has_event(&self, kind: u32) -> bool {
        self.journal
            .of_kind(kind)
            .iter()
            .any(|row| self.public_event(*row))
    }

    /// XI-9, 5 D2, 21j.1: WHAT THIS PARTY MUST FIND BY A DATE.
    pub fn owes_by(&self, day: Week) -> f64 {
        let Some(all) = self.schedules else {
            return 0.0;
        };
        all.of_payer(self.who)
            .iter()
            .map(|r| crate::stores::DueId(*r))
            .filter(|d| !all.paid(*d) && all.due(*d) <= day)
            .map(|d| all.amount(d))
            .sum()
    }

    /// And what it expects to RECEIVE by then — read off the lines it holds, because whoever holds a
    /// line is who is owed (Appendix B: no liability without a beneficiary, and never a second list
    /// of who is owed what).
    pub fn owed_to_it_by(&self, day: Week) -> f64 {
        let Some(all) = self.schedules else {
            return 0.0;
        };
        all.falling_to(self.who, Week(0), day, self.register, self.instruments)
    }

    /// The versions of what this view reads, for a caller keeping an answer across a walk.
    pub fn versions(&self) -> (u64, u64) {
        (self.register.version(), self.prints.version())
    }
}

/// A participant's reason to be in a market, evaluated per party with only that party's own view — a
/// schedule cannot be written against something the party may not see.
pub trait Participant {
    /// 3 C2, 22c2.3: WHAT THIS PARTY PULLS.
    fn pulls(&self, _view: &ParticipantView<'_>, _m: MarketId) -> Vec<crate::stores::RestingId> {
        Vec::new()
    }

    fn party_kind(&self) -> u32;

    /// Ordinary discretion belongs to living parties of this declaration's kind. A court- or
    /// regulator-directed participant can override this selection without making the party alive.
    fn eligible_parties(&self, parties: &crate::parties::Parties) -> Vec<PartyId> {
        parties
            .of_kind(self.party_kind())
            .iter()
            .map(|row| PartyId::at(*row))
            .filter(|party| parties.alive(*party))
            .collect()
    }

    /// WHICH BOOKS THIS PARTY COULD BE IN AT ALL THIS PERIOD.
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

/// THE SECOND DOOR (ARCHITECTURE 4.9b): what a module is given when its phase runs.
///
/// @spec ARCHITECTURE 4.9b · Law 4 · Law 10 · Law 19 · Appendix B
pub struct MechanismContext<'a> {
    week: u32,
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
    books: &'a [crate::session::BookDecl],
    sessions: &'a [crate::session::Session],
    calendar: &'a crate::calendar::Calendar,
    /// Which stage it is running in, so the rules of that stage are rules rather than placement.
    at: u32,
    wire: &'a Settlement,
    proposed: Vec<Proposed>,
    owing: Vec<Obligation>,
    said: Vec<Saying>,
    formed: Vec<(PartyId, u32, f64)>,
    observed: Vec<(PartyId, u32, f64, f64)>,
    ceased: Vec<crate::mechanisms::mortality::Ceased>,
    claimed: Vec<(PartyId, PartyId, f64, u32)>,
    repaid: Vec<(crate::stores::ClaimId, f64)>,
    lost: Vec<(crate::stores::ClaimId, f64)>,
    started: Vec<(PartyId, InstrumentId, f64, f64, u32)>,
    finished: Vec<crate::stores::BatchId>,
    stood: Vec<(u32, PartyId, PartyId, Vec<f64>)>,
    split: Vec<(
        PartyId,
        std::num::NonZeroU32,
        crate::stores::AgreementId,
        crate::parties::LatticeKey,
    )>,
    transitioned: Vec<(PartyId, crate::parties::LatticeKey)>,
    issued: Vec<Brings>,
    agreed: Vec<Agrees>,
    contracted: Vec<ContractObligation>,
    ended: Vec<crate::stores::AgreementId>,
    opened: Vec<Opens>,
    closed: Vec<crate::stores::ProcessId>,
    on_terms: Vec<(crate::ledger::QueueId, crate::calendar::Week)>,
}

/// A MODULE ASKS FOR AN OBLIGATION TO COME INTO EXISTENCE.
pub struct Brings {
    pub issuer: PartyId,
    /// Recipient of the newly issued units. `None` leaves them with the issuer for a later auction;
    /// a directly originated claim names its lender here.
    pub initial_holder: Option<PartyId>,
    /// Typed bilateral terms; absent for non-loan issuance.
    pub loan_terms: Option<crate::instruments::LoanTerms>,
    /// Cash bid by the initial holder for one loan unit.
    pub issue_price: Option<f64>,
    pub ccy: crate::ids::CurrencyCode,
    pub class: crate::instruments::Class,
    pub unit: crate::ids::UnitId,
    /// A TERM, fixed for the life of the instrument.
    pub coupon: Option<f64>,
    pub matures: Option<crate::calendar::Week>,
    /// Bond N6: how often it pays and how interest accrues between payments. The kernel generates
    /// the schedule from these and the issuer never writes one, because what a piece of paper owes
    /// is the contract's arithmetic and not each issuer's copy of it.
    pub pays: crate::instruments::PaymentFrequency,
    pub convention: crate::calendar::Convention,
    /// How many units of it come into existence on the issuer's own book.
    pub units: f64,
    /// Whether a book opens for it, and under which rule.
    pub book: Option<crate::protocols::Venue>,
}

impl Brings {
    /// Originate a loan as a transferable claim issued by the borrower and held by its lender.
    pub fn loan_claim(
        borrower: PartyId,
        lender: PartyId,
        ccy: crate::ids::CurrencyCode,
        terms: crate::instruments::LoanTerms,
        issue_price: f64,
    ) -> Self {
        assert!(
            borrower != lender,
            "Money E1: a borrower cannot lend to itself"
        );
        assert!(issue_price.is_finite() && issue_price > 0.0 && issue_price <= terms.amount);
        Self {
            issuer: borrower,
            initial_holder: Some(lender),
            loan_terms: Some(terms),
            issue_price: Some(issue_price),
            ccy,
            class: crate::instruments::Class::Claim,
            unit: crate::ids::UnitId::at(0),
            coupon: None,
            matures: None,
            pays: crate::instruments::PaymentFrequency::AtMaturity,
            convention: crate::calendar::Convention::Actual365,
            units: 1.0,
            book: None,
        }
    }
}

/// A relation a module asks the kernel to strike.
pub struct Agrees {
    pub kind: u32,
    pub one: PartyId,
    pub other: PartyId,
    /// What was agreed, in the order that kind declares.
    pub terms: crate::stores::AgreementTerms,
    /// `Missing` where it runs until somebody ends it, which is not the same as ending today.
    pub until: Option<crate::calendar::Week>,
}

/// A bilateral or instrument obligation proposed to the schedule's single writer.
pub struct Obligation {
    pub on: crate::stores::Owed,
    pub owed_by: PartyId,
    pub ccy: crate::ids::CurrencyCode,
    pub payment: crate::stores::Payment,
    pub agreement: Option<crate::stores::AgreementId>,
}

/// One newly struck agreement and the payments it creates at inception.
pub struct ContractObligation {
    pub agreement: Agrees,
    pub owed_by: PartyId,
    pub ccy: crate::ids::CurrencyCode,
    pub payments: Vec<crate::stores::Payment>,
}

/// Something a module puts in flight, with an owner and an end.
pub struct Opens {
    pub kind: u32,
    pub owner: PartyId,
    /// The line this process acts on, when the process is instrument-specific.
    pub subject: Option<InstrumentId>,
    /// The causal door through which it opened, when the process has one.
    pub door: Option<u32>,
    /// The week it is due to close.
    pub closes: Option<u32>,
    pub size: f64,
}

/// One thing a module asks the world to do.
pub struct Proposed {
    pub legs: Vec<Leg>,
    pub cause: Cause,
    pub delivery: Delivery,
    pub why: &'static str,
    pub due: Option<DueId>,
}

/// Something a module states happened, for whoever it happened to.
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
    /// Who is owed what by a dead party, and at what rank.
    pub claims: &'a Claims,
    pub wire: &'a Settlement,
    /// Terms parties stand behind, and what is on the line.
    pub standing: &'a crate::stores::Standing,
    pub making: &'a crate::stores::InProgress,
    /// What the ids point at — a line's footprint, a region's country, a kind's profile.
    pub registry: &'a crate::registry::Registry,
    pub books: &'a [crate::session::BookDecl],
    /// Completed book sessions, including non-clearing outcomes.
    pub sessions: &'a [crate::session::Session],
    /// THE ONE CALENDAR. A mechanism that multiplies a week by a day length has built a second
    /// one, and the two agree only while nobody moves the epoch (Money G3, G3.c).
    pub calendar: &'a crate::calendar::Calendar,
}

impl<'a> MechanismContext<'a> {
    pub fn of(week: u32, at: u32, s: Stores<'a>) -> Self {
        Self {
            week,
            at,
            calendar: s.calendar,
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
            books: s.books,
            sessions: s.sessions,
            proposed: Vec::new(),
            owing: Vec::new(),
            said: Vec::new(),
            formed: Vec::new(),
            observed: Vec::new(),
            ceased: Vec::new(),
            claimed: Vec::new(),
            repaid: Vec::new(),
            lost: Vec::new(),
            started: Vec::new(),
            finished: Vec::new(),
            stood: Vec::new(),
            split: Vec::new(),
            transitioned: Vec::new(),
            issued: Vec::new(),
            agreed: Vec::new(),
            contracted: Vec::new(),
            ended: Vec::new(),
            opened: Vec::new(),
            closed: Vec::new(),
            on_terms: Vec::new(),
        }
    }

    /// What parties stand behind — a posting, a lending standard — and what is on the line.
    pub fn standing(&self) -> &crate::stores::Standing {
        self.standing
    }

    /// Completed books are facts after the books stage, including books that did not clear.
    pub fn sessions(&self) -> &[crate::session::Session] {
        self.sessions
    }

    /// The DATA every id points at — a line's footprint, a region's country.
    pub fn registry(&self) -> &crate::registry::Registry {
        self.registry
    }

    pub fn making(&self) -> &crate::stores::InProgress {
        self.making
    }

    pub fn week(&self) -> u32 {
        self.week
    }

    /// THE CURRENT WEEK, from the one calendar. A week is the minimal indivisible unit
    /// of time (Money G1), so this is the only day a mechanism running in it has.
    pub fn today(&self) -> crate::calendar::Week {
        self.calendar
            .at(crate::calendar::Week(i64::from(self.week)))
    }

    /// And the current week it covers, so "what falls due this week" is a read of the calendar at
    /// both ends rather than a day length added to the first.
    pub fn current_week(&self) -> crate::calendar::Week {
        crate::calendar::Week(i64::from(self.week))
    }

    /// The one calendar, for placing a date the week does not itself name.
    pub fn calendar(&self) -> &crate::calendar::Calendar {
        self.calendar
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

    pub fn subject_of(&self, market: MarketId) -> Option<InstrumentId> {
        crate::session::declared_subject(self.books, market)
    }

    pub fn journal(&self) -> &Journal {
        self.journal
    }

    pub fn params(&self) -> &Params {
        self.params
    }

    /// The relations this party is in — an engagement, a mortgage, a policy.
    pub fn agreements(&self) -> &Agreements {
        self.agreements
    }

    /// What each instrument owes and when.
    pub fn schedules(&self) -> &Schedules {
        self.schedules
    }

    /// What each party expects.
    pub fn outlooks(&self) -> &Outlooks {
        self.outlooks
    }

    /// What is in flight across weeks.
    pub fn processes(&self) -> &Processes {
        self.processes
    }

    /// Who is owed what by an estate, to READ.
    pub fn claims(&self) -> &Claims {
        self.claims
    }

    /// What actually happened, to read and never to write.
    pub fn wire(&self) -> &Settlement {
        self.wire
    }

    /// What it asks the world to do.
    pub fn propose(&mut self, legs: Vec<Leg>, cause: Cause, delivery: Delivery, why: &'static str) {
        assert!(
            !why.is_empty(),
            "4.9b: an instruction with no reason is a state change with a date on it"
        );
        // Money G1.c: NOTHING IS CALLED AND PAID IN THE SAME PERIOD. A stage that decides what the
        // week's judgement implies decides it for the week AFTER, so what it has is an
        // obligation to write and not an instruction to settle.
        assert!(
            self.at != crate::world::SCHEDULED,
            "Money G1.c: a call settled in the week it was made — {why}. What a scheduling stage \
             produces is a payment that FALLS DUE, through `owes`"
        );
        self.proposed.push(Proposed {
            legs,
            cause,
            delivery,
            why,
            due: None,
        });
    }

    /// Propose performance of one existing contractual due.
    pub fn propose_due(
        &mut self,
        due: DueId,
        legs: Vec<Leg>,
        cause: Cause,
        delivery: Delivery,
        why: &'static str,
    ) {
        assert!(!why.is_empty(), "4.9b: a due instruction needs a reason");
        self.proposed.push(Proposed {
            legs,
            cause,
            delivery,
            why,
            due: Some(due),
        });
    }

    /// A bilateral due created by an agreement that is already live.
    pub fn owes_under(
        &mut self,
        agreement: crate::stores::AgreementId,
        payee: PartyId,
        owed_by: PartyId,
        ccy: crate::ids::CurrencyCode,
        payment: crate::stores::Payment,
    ) {
        self.owing.push(Obligation {
            on: crate::stores::Owed::To(payee),
            owed_by,
            ccy,
            payment,
            agreement: Some(agreement),
        });
    }

    /// A statutory due has a named payer and payee but no bilateral agreement.
    pub fn owes(
        &mut self,
        payee: PartyId,
        owed_by: PartyId,
        ccy: crate::ids::CurrencyCode,
        payment: crate::stores::Payment,
    ) {
        self.owing.push(Obligation {
            on: crate::stores::Owed::To(payee),
            owed_by,
            ccy,
            payment,
            agreement: None,
        });
    }

    /// Something it states happened, for whoever it happened to.
    pub fn say(&mut self, kind: u32, subjects: &[u32], data: &[(u32, Value)], public: bool) {
        self.said.push(Saying {
            kind,
            subjects: subjects.to_vec(),
            data: data.to_vec(),
            public,
        });
    }

    /// An outlook it formed from ITS OWN history.
    pub fn form(&mut self, party: PartyId, about: u32, level: f64) {
        self.formed.push((party, about, level));
    }

    /// A lagged result observed by one party, with that party's entry-time memory horizon.
    pub fn observe(&mut self, party: PartyId, about: u32, observed: f64) {
        self.observed
            .push((party, about, observed, self.parties.outlook_memory(party)));
    }

    /// NOTHING IS IMMORTAL, and a thing that ends says when.
    pub fn ceases(&mut self, event: crate::mechanisms::mortality::Ceased) {
        self.ceased.push(event);
    }

    /// An orderly wind-up uses the same typed cessation handoff without making one mechanism
    /// depend on another mechanism's implementation.
    pub fn winds_up(&mut self, who: PartyId, says: u32, at_trigger: u32, at_destination: u32) {
        use crate::mechanisms::mortality::{Ceased, Destination, Trigger};
        let event = Ceased {
            who,
            why: Trigger::WoundUp,
            to: Destination::Estate,
            week: self.week,
        };
        self.ceased.push(event);
        self.say(
            says,
            &[who.0],
            &[
                (
                    at_trigger,
                    Value::Num(crate::mechanisms::mortality::trigger_code(event.why)),
                ),
                (
                    at_destination,
                    Value::Num(crate::mechanisms::mortality::destination_code(event.to)),
                ),
            ],
            true,
        );
    }

    /// What somebody is OWED by a party whose life has ended.
    pub fn is_owed(&mut self, on: PartyId, holder: PartyId, owed: f64, ranks: u32) {
        self.claimed.push((on, holder, owed, ranks));
    }

    /// And what an estate PAID one.
    pub fn pays(&mut self, claim: crate::stores::ClaimId, amount: f64) {
        self.repaid.push((claim, amount));
    }

    pub fn loses(&mut self, claim: crate::stores::ClaimId, amount: f64) {
        self.lost.push((claim, amount));
    }

    /// Final distribution is over: extinguish exactly what remains rather than leaving a live
    /// residual claim on a destination with no property left to realise.
    pub fn extinguishes(&mut self, claim: crate::stores::ClaimId) {
        let residual = self.claims.outstanding(claim);
        if residual > 0.0 {
            self.lost.push((claim, residual));
        }
    }

    /// A batch goes ON the line, owned, carrying what it cost, ready in a later week.
    pub fn starts(
        &mut self,
        owner: PartyId,
        what: InstrumentId,
        units: f64,
        cost: f64,
        ready: u32,
    ) {
        self.started.push((owner, what, units, cost, ready));
    }

    /// And a batch comes OFF it.
    pub fn finishes(&mut self, batch: crate::stores::BatchId) {
        self.finished.push(batch);
    }

    /// What this party stands behind, until it withdraws it: a posting, a lending standard.
    pub fn now_stands(&mut self, kind: u32, who: PartyId, about: PartyId, terms: Vec<f64>) {
        self.stood.push((kind, who, about, terms));
    }

    /// Bring an obligation into existence.
    pub fn brings(&mut self, what: Brings) {
        self.issued.push(what);
    }

    /// Strike a relation.
    pub fn agrees(&mut self, what: Agrees) {
        self.agreed.push(what);
    }

    pub fn contracts(&mut self, what: ContractObligation) {
        assert!(
            !what.payments.is_empty(),
            "a contract with no performance is not an obligation"
        );
        self.contracted.push(what);
    }

    /// And it ends, and the ending is recorded.
    pub fn ends(&mut self, a: crate::stores::AgreementId) {
        self.ended.push(a);
    }

    /// Put something in flight, with an owner and an end.
    pub fn opens(&mut self, what: Opens) {
        self.opened.push(what);
    }

    /// And finish one.
    pub fn closes(&mut self, p: crate::stores::ProcessId) {
        self.closed.push(p);
    }

    /// The seller agreed to WAIT, and the payment it was waiting on becomes terms.
    pub fn waits_for(&mut self, q: crate::ledger::QueueId, until: crate::calendar::Week) {
        self.on_terms.push((q, until));
    }

    /// An event that applies to SOME of a cell splits it, and the relationship that applies to them
    /// goes with them.
    pub fn splits(
        &mut self,
        cell: PartyId,
        taking: std::num::NonZeroU32,
        carrying: crate::stores::AgreementId,
        destination: crate::parties::LatticeKey,
    ) {
        self.split.push((cell, taking, carrying, destination));
    }

    /// Move a whole population cell to another coordinate on its lattice.
    pub fn transitions(&mut self, cell: PartyId, destination: crate::parties::LatticeKey) {
        self.transitioned.push((cell, destination));
    }

    /// What the kernel applies once the phase returns.
    pub fn taken(self) -> Taken {
        Taken {
            proposed: self.proposed,
            owing: self.owing,
            said: self.said,
            formed: self.formed,
            observed: self.observed,
            ceased: self.ceased,
            claimed: self.claimed,
            repaid: self.repaid,
            lost: self.lost,
            started: self.started,
            finished: self.finished,
            stood: self.stood,
            agreed: self.agreed,
            contracted: self.contracted,
            ended: self.ended,
            opened: self.opened,
            closed: self.closed,
            on_terms: self.on_terms,
            split: self.split,
            transitioned: self.transitioned,
            issued: self.issued,
        }
    }
}

impl Taken {
    /// WHETHER THIS MECHANISM DECIDED ANYTHING, READ OFF WHAT IT ASKED FOR.
    pub fn decided(&self) -> bool {
        // Destructured with no `..`, so an eighteenth kind of ask FAILS TO COMPILE until it is
        // accounted for here.
        let Taken {
            // Saying is the one that does NOT count: it is the count.
            said: _,
            proposed,
            owing,
            formed,
            ceased,
            claimed,
            repaid,
            lost,
            started,
            finished,
            stood,
            agreed,
            contracted,
            ended,
            opened,
            closed,
            on_terms,
            split,
            transitioned,
            issued,
            observed,
        } = self;
        !proposed.is_empty()
            || !owing.is_empty()
            || !formed.is_empty()
            || !ceased.is_empty()
            || !claimed.is_empty()
            || !repaid.is_empty()
            || !lost.is_empty()
            || !started.is_empty()
            || !finished.is_empty()
            || !stood.is_empty()
            || !agreed.is_empty()
            || !contracted.is_empty()
            || !ended.is_empty()
            || !opened.is_empty()
            || !closed.is_empty()
            || !on_terms.is_empty()
            || !split.is_empty()
            || !transitioned.is_empty()
            || !issued.is_empty()
            || !observed.is_empty()
    }
}

/// Everything one phase asked for, handed back for the kernel to apply.
pub struct Taken {
    pub proposed: Vec<Proposed>,
    /// Obligations struck: what falls due, on what or to whom, in what money.
    pub owing: Vec<Obligation>,
    pub said: Vec<Saying>,
    pub formed: Vec<(PartyId, u32, f64)>,
    pub observed: Vec<(PartyId, u32, f64, f64)>,
    /// The parties whose life ended in this phase.
    pub ceased: Vec<crate::mechanisms::mortality::Ceased>,
    /// Who is owed what by a dead party, and at what rank.
    pub claimed: Vec<(PartyId, PartyId, f64, u32)>,
    /// And what an estate actually PAID one, so the claim comes down.
    pub repaid: Vec<(crate::stores::ClaimId, f64)>,
    /// Unpaid balances finalised as losses on their named holders.
    pub lost: Vec<(crate::stores::ClaimId, f64)>,
    /// Batches that went ON the line this phase — owner, what, how much, what it cost, and the
    /// week it is ready.
    pub started: Vec<(PartyId, InstrumentId, f64, f64, u32)>,
    /// And the batches taken off it, whose `Create` legs are in `proposed` (Law 5: one event).
    pub finished: Vec<crate::stores::BatchId>,
    /// Terms a party now stands behind.
    pub stood: Vec<(u32, PartyId, PartyId, Vec<f64>)>,
    /// Cells that an event applies to part of — the parent, how many members it takes, and the
    /// relationship those members carry with them.
    pub split: Vec<(
        PartyId,
        std::num::NonZeroU32,
        crate::stores::AgreementId,
        crate::parties::LatticeKey,
    )>,
    /// Whole-cell lattice transitions.
    pub transitioned: Vec<(PartyId, crate::parties::LatticeKey)>,
    /// Obligations a module asked to bring into existence.
    pub issued: Vec<Brings>,
    /// Relations struck, and relations ended.
    pub agreed: Vec<Agrees>,
    pub contracted: Vec<ContractObligation>,
    pub ended: Vec<crate::stores::AgreementId>,
    /// Processes opened, and processes finished.
    pub opened: Vec<Opens>,
    pub closed: Vec<crate::stores::ProcessId>,
    /// Queued payments a seller agreed to wait for, replaced by terms.
    pub on_terms: Vec<(crate::ledger::QueueId, crate::calendar::Week)>,
}

/// A system's own work in a week, as opposed to the questions its participants are asked in books.
pub trait Mechanism {
    fn run(&self, ctx: &mut MechanismContext<'_>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_event_reaches_its_subjects_and_the_public_record_and_nobody_else() {
        let me = PartyId::at(0);
        let rival = PartyId::at(1);
        assert!(reaches(false, &[me.0], me));
        assert!(!reaches(false, &[rival.0], me));
        assert!(reaches(true, &[], me));
        assert!(reaches(true, &[rival.0], me));
    }
}
