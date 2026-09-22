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

/// THE UNITS OF A LINE THAT ARE SOMEBODY ELSE'S — what an issuer owes on, which is not what it
/// holds of its own paper.
pub fn outstanding_of(register: &Register, instruments: &Instruments, line: InstrumentId) -> f64 {
    let issuer = instruments.issuer_of(line);
    register
        .of_instrument(line)
        .iter()
        .map(|row| HoldingId(*row))
        .filter(|row| register.holder_of(*row) != issuer)
        .map(|row| register.quantity(row))
        .sum()
}

/// WHAT ONE PROMISED PAYMENT IS WORTH to somebody who requires `requires` a year for waiting and
/// holds `chance` of a view that this name pays what it says.
///
/// A party that holds no view of the name reckons on the contract as written: not a default, but a
/// party with less to go on bidding more than one that knows better. A return that turns waiting
/// into nothing is not a valuation anybody can make, and is refused rather than bounded.
pub fn worth_of(promised: f64, chance: Option<f64>, requires: f64, waiting: f64) -> Option<f64> {
    let discount = 1.0 + requires * waiting;
    if discount <= 0.0 || !discount.is_finite() {
        return None;
    }
    let expects = match chance {
        Some(chance) => promised * chance,
        None => promised,
    };
    Some(expects / discount)
}

/// NET ASSET VALUE PER SHARE, and the one writer of it. A pool with no shares has no NAV — not a
/// zero, which is a price somebody could have paid.
pub fn nav(assets_at_market: f64, liabilities: f64, shares: f64) -> Option<f64> {
    if shares <= 0.0 {
        return None;
    }
    Some((assets_at_market - liabilities) / shares)
}

/// Shares issued at NAV. A pool that cannot say what a share is worth cannot sell one.
pub fn subscribe(cash: f64, nav: Option<f64>) -> Option<f64> {
    assert!(
        cash > 0.0,
        "13 C1: a subscription of {cash} is not a subscription"
    );
    match nav {
        Some(nav) if nav > 0.0 => Some(cash / nav),
        _ => None,
    }
}

/// What a redemption needs and where it comes from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Meeting {
    /// What the holder is owed at NAV.
    pub owed: f64,
    /// Taken from what the pool already holds.
    pub from_buffer: f64,
    /// And this is the forced sale.
    pub must_sell: f64,
}

/// Takes shares back and pays cash at NAV, finding the cash from the buffer or by selling.
pub fn meet(shares: f64, nav: f64, buffer: f64) -> Meeting {
    assert!(
        shares > 0.0,
        "13 C2: a redemption of {shares} shares is not a redemption"
    );
    let owed = shares * nav;
    let from_buffer = if buffer >= owed { owed } else { buffer };
    Meeting {
        owed,
        from_buffer,
        must_sell: owed - from_buffer,
    }
}

/// DEBT SERVICE: a fixed claim ahead of the owners, interest AND principal. Every borrower owes
/// one, so the reads over it — coverage against cash, burden against income — are each their own
/// system's, and the thing they read is not.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Service {
    pub interest: f64,
    pub principal: f64,
}

impl Service {
    pub fn total(&self) -> f64 {
        self.interest + self.principal
    }
}

/// WHAT A BORROWER MUST RAISE: the gap it cannot meet, plus what restocks its own buffer.
///
/// Raising the gap is what pays the gap, so the balance ends where it started and the restock is
/// measured against THAT, not against a balance the gap has been taken out of as well. The gap is
/// the borrower's own — a treasury's is outlays less receipts, a firm's is what falls due — and
/// only the buffer rule is shared.
pub fn must_raise(gap: f64, cash: f64, buffer: f64) -> f64 {
    let restock = buffer - cash;
    match restock > 0.0 {
        true => gap + restock,
        false => gap,
    }
}

/// WHAT ONE UNIT OF A LINE HAS ACCRUED, and the one writer of it.
///
/// A coupon accrues over the interval the payment covers. A line with no coupon accretes its own
/// discount instead — par against the price it FIRST cleared at, over the life it first cleared
/// for — because the return on a bill is the discount and nothing else is paid on the way.
pub fn accrued_per_unit(
    register: &Register,
    schedules: &Schedules,
    instruments: &Instruments,
    prints: &Prints,
    line: InstrumentId,
    on: Week,
) -> Option<f64> {
    let outstanding = outstanding_of(register, instruments, line);
    if outstanding <= 0.0 {
        return None;
    }
    if let Some(due) = schedules.accruing(line, on) {
        return schedules.accrued(due, on).map(|it| it / outstanding);
    }
    let matures = instruments.matures_on(line)?;
    if instruments.coupon_of(line).is_some_and(|rate| rate > 0.0) {
        return None;
    }
    let struck = prints.first_of_line(line)?;
    let from = Week(i64::from(struck.week));
    // Past its maturity a bill has nothing left to accrete: what it owes is the principal, and the
    // question is about a different thing.
    if on <= from || matures <= from || on > matures {
        return None;
    }
    let par = schedules
        .of_instrument(line)
        .iter()
        .map(|row| DueId(*row))
        .find(|due| schedules.of(*due) == crate::stores::Owing::Principal)
        .map(|due| schedules.amount(due) / outstanding)?;
    Some(crate::instruments::accrued(
        from,
        matures,
        par - struck.price,
        on,
    ))
}

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
    parties: &'a crate::parties::Parties,
    registry: &'a crate::registry::Registry,
    geography: &'a crate::geography::Geography,
    /// 46 F3: what each family of thing is worth, asked of the system that owns the family.
    valuers: &'a [Box<dyn Valuer>],
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
    /// 21 A1.a: so a view can mark its own holdings at the price of the place it stands in.
    pub parties: &'a crate::parties::Parties,
    pub registry: &'a crate::registry::Registry,
    pub valuers: &'a [Box<dyn Valuer>],
    /// 49 C4: where this party stands, and what joins it to anywhere else.
    pub geography: &'a crate::geography::Geography,
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
            geography: inputs.geography,
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
            parties: inputs.parties,
            registry: inputs.registry,
            valuers: inputs.valuers,
        }
    }

    /// This party's own expectation, never a global forecast or another party's view.
    pub fn outlook(&self, about: u32) -> Option<f64> {
        self.outlooks.of(self.who, about)
    }

    pub fn price_outlook(&self, line: InstrumentId) -> Option<f64> {
        self.outlook(crate::stores::about::price_of(line))
    }

    /// WHAT THIS PARTY MAKES OF PAPER IT HAS NO HISTORY OF.
    ///
    /// An outlook is formed from a party's own history of a line, and a line brought this week has
    /// none — so a first bid has to come from the names the party DOES have a view of: the issuer's
    /// other paper first, because that prices the name, then paper that comes back when this does.
    /// It is this party's own view either way, so two bidders still disagree.
    pub fn values(&self, line: InstrumentId) -> Option<f64> {
        match self.price_outlook(line) {
            Some(level) => Some(level),
            None => self.values_unseen(line),
        }
    }

    pub fn registry(&self) -> &crate::registry::Registry {
        self.registry
    }

    /// Where this party stands, as a place on the network — the one a route can start from.
    pub fn place(&self) -> Option<crate::geography::SiteId> {
        self.geography.site_of(self.who)
    }

    /// Where a route runs from and to, so a party can tell which ones are its own to use.
    pub fn route_ends(
        &self,
        route: crate::geography::RouteId,
    ) -> Option<(crate::geography::SiteId, crate::geography::SiteId)> {
        self.geography.ends_of(route)
    }

    /// 38 D3: the place a route delivers into, so a shipper can read what a line fetches there.
    pub fn destination_of(
        &self,
        route: crate::geography::RouteId,
    ) -> Option<crate::ids::RegionId> {
        let (_, to) = self.geography.ends_of(route)?;
        self.geography.region_of(self.geography.tile_of(to)?)
    }

    /// The book for this line in ANOTHER place — where a party that ships there sells delivered.
    pub fn market_at(
        &self,
        subject: InstrumentId,
        at: crate::ids::RegionId,
    ) -> Option<MarketId> {
        crate::session::book_here(self.books, subject, at)
    }

    fn values_unseen(&self, line: InstrumentId) -> Option<f64> {
        let issuer = self.instruments.issuer_of(line);
        if let Some(level) = self
            .instruments
            .of_issuer(issuer)
            .iter()
            .map(|row| InstrumentId::at(*row))
            .filter(|other| *other != line)
            .find_map(|other| self.price_outlook(other))
        {
            return Some(level);
        }
        if let Some(worth) = self
            .valuers
            .iter()
            .find(|it| it.family() == self.instruments.class_of(line))
            .and_then(|it| it.value(self, line))
        {
            return Some(worth);
        }
        if let Some(back) = self.instruments.matures_on(line) {
            if let Some(level) = self
                .outlooks
                .of_party(self.who)
                .iter()
                .filter_map(|row| {
                    let other = crate::stores::about::line_of(self.outlooks.about_at(*row))?;
                    (self.instruments.matures_on(other) == Some(back))
                        .then(|| self.outlooks.level_at(*row))
                })
                .next()
            {
                return Some(level);
            }
        }
        self.fair_value(line)
    }

    /// WHAT THIS PARTY RECKONS ONE UNIT IS WORTH, from the contract and its own two numbers: what
    /// it thinks this name pays back, and what it requires for waiting.
    ///
    /// This is a REASON, not a price. The price is what clears when reckonings like it meet an
    /// offer, and two parties with different views of the name or different costs of credit reckon
    /// differently — which is what gives a primary book two sides on the day a name first issues,
    /// before anything of its has ever printed.
    pub fn fair_value(&self, line: InstrumentId) -> Option<f64> {
        let schedules = self.schedules?;
        // What it requires for waiting is what credit costs IT, out of its own history.
        let requires = self.outlook(crate::stores::about::WHAT_CREDIT_COSTS)?;
        let view = self.own_view_of(self.instruments.issuer_of(line));
        let units = outstanding_of(self.register, self.instruments, line);
        if units <= 0.0 {
            return None;
        }
        let today = self.today();
        let mut worth = 0.0;
        for row in schedules.of_instrument(line) {
            let due = DueId(*row);
            if schedules.paid(due) || schedules.due(due) <= today {
                continue;
            }
            // Actual/365, the count this world's curves are read on, and here the investor's own:
            // the line does not carry the convention it was struck under.
            let waiting =
                crate::calendar::Convention::Actual365.year_fraction(today, schedules.due(due));
            let Some(part) = worth_of(schedules.amount(due), view, requires, waiting) else {
                return None;
            };
            worth += part;
        }
        (worth > 0.0).then_some(worth / units)
    }

    /// Its OWN view of whether a name pays, which it holds or does not.
    pub fn own_view_of(&self, borrower: PartyId) -> Option<f64> {
        let all = self.standing?;
        let row = all.of_party_about(self.who, borrower, crate::stores::standing::OWN_VIEW)?;
        all.terms(row).first().copied()
    }

    pub fn subject_of(&self, market: MarketId) -> Option<InstrumentId> {
        crate::session::declared_subject(self.books, market)
    }

    /// 21 A1.a: the book for this line WHERE THIS PARTY IS. A good has one per place, so the first
    /// declared would put every party in one region's book whatever region it stood in.
    pub fn market_of(&self, subject: InstrumentId) -> Option<MarketId> {
        crate::session::book_here(self.books, subject, self.parties.region_of(self.who))
    }

    pub fn venue_of(&self, market: MarketId) -> Option<crate::protocols::Venue> {
        crate::session::declared_venue(self.books, market)
    }

    /// EVERY BOOK THAT IS OPEN, so a party can find the paper somebody brought this week. A
    /// participant wired to a fixed list of lines cannot bid for an issue that did not exist when
    /// the world was assembled, and an auction whose buyers cannot see it has a seller and nobody.
    pub fn open_books(&self) -> impl Iterator<Item = (MarketId, InstrumentId)> + '_ {
        self.books.iter().map(|book| (book.market, book.subject))
    }

    /// 21 A1.a: what a line last printed WHERE THESE UNITS ARE. A holder marks at the price of the
    /// place it is in, and a line with one book everywhere answers the same in every place.
    pub fn print_here(&self, line: InstrumentId, at: crate::ids::RegionId) -> Option<Print> {
        self.prints.latest(
            crate::session::book_here(self.books, line, at)?,
            line,
            self.week,
        )
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

    /// The terms this party stands behind toward NOBODY IN PARTICULAR — a posted rate, which is
    /// one-sided and held until it changes.
    pub fn own_posted(&self, kind: u32) -> Option<&[f64]> {
        let all = self.standing?;
        let row = all.of_party_about(self.who, PartyId::NONE, kind)?;
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
                &crate::prices::Marks {
                    prints: self.prints,
                    books: self.books,
                    parties: self.parties,
                },
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
    /// When a line pays its principal back, which is what a lender is asking before it lends.
    pub fn matures_on(&self, line: InstrumentId) -> Option<crate::calendar::Week> {
        self.instruments.matures_on(line)
    }

    /// What backs a unit of it, for paper that is secured — a term of the paper, so any lender
    /// reads the same pledge.
    pub fn secured_by(&self, line: InstrumentId) -> Option<crate::instruments::Pledged> {
        self.instruments.collateral_of(line)
    }

    /// Whose paper it is — the name a lender is taking, not an anonymous unit.
    pub fn issuer_of(&self, line: InstrumentId) -> PartyId {
        self.instruments.issuer_of(line)
    }

    pub fn print(&self, instrument: InstrumentId) -> Option<Print> {
        self.prints.of_line(instrument, self.week)
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

    /// XI-6: what this party would hold what it buys here FOR, asked of every party in the book
    /// because the declaration belongs to the acquisition a fill would be. `None` where this
    /// party does not acquire here at all — and settlement then refuses units it never declared,
    /// rather than the book inventing a treatment for it.
    fn carries(
        &self,
        view: &ParticipantView<'_>,
        market: MarketId,
    ) -> Option<crate::register::Carrying>;
}

/// The same door for a VENUE, where what is struck is not the transfer of an instrument.
/// WHAT A FAMILY OF THING IS WORTH, answered by the system that owns that family.
///
/// 46 F3: the comparison is one mechanism and the TERMS are the thing's own — a claim's dated
/// payments, a plant's capacity and life, a share's residual, a dwelling's rent. The kernel asks
/// the family rather than holding a formula that would have to be true of all of them, which is a
/// decision taken at an average.
///
/// A valuer is given the party's view and must not ask it for `values` of the same line: it is the
/// answer that read is looking for.
pub trait Valuer {
    /// The class of thing this values. One family, one answer.
    fn family(&self) -> crate::instruments::Class;
    fn value(&self, view: &ParticipantView<'_>, line: InstrumentId) -> Option<f64>;
}

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
    equity: &'a crate::stores::Equity,
    standing: &'a crate::stores::Standing,
    making: &'a crate::stores::InProgress,
    registry: &'a crate::registry::Registry,
    geography: &'a crate::geography::Geography,
    books: &'a [crate::session::BookDecl],
    sessions: &'a [crate::session::Session],
    calendar: &'a crate::calendar::Calendar,
    /// Which stage it is running in, so the rules of that stage are rules rather than placement.
    at: u32,
    wire: &'a Settlement,
    kernel_says: crate::ledger::Outcomes,
    proposed: Vec<Proposed>,
    owing: Vec<Obligation>,
    said: Vec<Saying>,
    formed: Vec<(PartyId, u32, f64)>,
    observed: Vec<(PartyId, u32, f64, f64)>,
    ceased: Vec<crate::mechanisms::mortality::Ceased>,
    delivered: Vec<(
        u32,
        crate::mechanisms::freight::DeliveryOutcome,
        crate::geography::VehicleId,
        crate::geography::SiteId,
    )>,
    extracted: Vec<(crate::geography::TileId, InstrumentId, f64)>,
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
    carried: Vec<(PartyId, InstrumentId, crate::register::Carrying)>,
}

/// A MODULE ASKS FOR AN OBLIGATION TO COME INTO EXISTENCE.
pub struct Brings {
    pub issuer: PartyId,
    /// Recipient of the newly issued units. `None` leaves them with the issuer for a later auction;
    /// a directly originated claim names its lender here.
    pub initial_holder: Option<PartyId>,
    /// Typed bilateral terms; absent for non-loan issuance.
    pub loan_terms: Option<crate::instruments::LoanTerms>,
    /// 11 B3: what backs a unit of it, for paper that is secured rather than bilateral.
    pub secured_by: Option<crate::instruments::Pledged>,
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
    /// XI-6: what the initial holder holds it FOR. Paper an issuer has not sold yet is its own
    /// position like any other, and settlement refuses units nobody has declared.
    pub carried_as: crate::register::Carrying,
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
            secured_by: None,
            issue_price: Some(issue_price),
            ccy,
            class: crate::instruments::Class::Claim,
            unit: crate::ids::UnitId::at(0),
            coupon: None,
            matures: None,
            pays: crate::instruments::PaymentFrequency::AtMaturity,
            convention: crate::calendar::Convention::Actual365,
            // A lender that originated a claim holds it to its end at what it lent.
            carried_as: crate::register::Carrying::Cost,
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
    /// Audit B5: each party's equity account, as the movements that made it.
    pub equity: &'a crate::stores::Equity,
    pub wire: &'a Settlement,
    /// The kinds the KERNEL itself says under.
    pub kernel_says: crate::ledger::Outcomes,
    /// Terms parties stand behind, and what is on the line.
    pub standing: &'a crate::stores::Standing,
    pub making: &'a crate::stores::InProgress,
    /// What the ids point at — a line's footprint, a unit's divisor, a kind's profile.
    pub registry: &'a crate::registry::Registry,
    /// Where everything is: the ground, its jurisdictions, its sites and what moves between them.
    pub geography: &'a crate::geography::Geography,
    pub books: &'a [crate::session::BookDecl],
    /// 46 F3: what each family of thing is worth, answered by the system that owns it.
    pub valuers: &'a [Box<dyn Valuer>],
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
            equity: s.equity,
            wire: s.wire,
            kernel_says: s.kernel_says,
            standing: s.standing,
            making: s.making,
            registry: s.registry,
            geography: s.geography,
            books: s.books,
            sessions: s.sessions,
            proposed: Vec::new(),
            owing: Vec::new(),
            said: Vec::new(),
            formed: Vec::new(),
            observed: Vec::new(),
            ceased: Vec::new(),
            delivered: Vec::new(),
            extracted: Vec::new(),
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
            carried: Vec::new(),
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
    /// 49 C2, Seed B3: what a place is paid in — the ground says whose country it is and the
    /// registry says what that country's money is. Neither store answers it alone.
    pub fn currency_of(&self, region: crate::ids::RegionId) -> Option<crate::ids::CurrencyCode> {
        let country = self.geography.country_of(region)?;
        Some(self.registry.currency_of_country(country))
    }

    /// 49 C4: what a PARTY is paid in — read through the site it stands on, not off a column.
    pub fn money_of(&self, who: PartyId) -> Option<crate::ids::CurrencyCode> {
        let (_, region) = self.geography.where_is(who)?;
        self.currency_of(region)
    }

    /// 49 A: where everything is.
    pub fn geography(&self) -> &crate::geography::Geography {
        self.geography
    }

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

    /// 21 A1.a: what a line last printed WHERE THESE UNITS ARE — the answer a holder marks at,
    /// because a good is its sub-unit and a market in it is (region, sub-unit). A line with one
    /// book everywhere answers the same in every place.
    pub fn print_here(
        &self,
        line: InstrumentId,
        at: crate::ids::RegionId,
    ) -> Option<crate::prices::Print> {
        self.prints.latest(
            crate::session::book_here(self.books, line, at)?,
            line,
            self.week,
        )
    }

    /// The same read, for the value functions that take a row and must mark it where it is.
    pub fn marks(&self) -> crate::prices::Marks<'_> {
        crate::prices::Marks {
            prints: self.prints,
            books: self.books,
            parties: self.parties,
        }
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
    /// Audit B5: what each party's account has been moved by, which is what a published result
    /// reads rather than the residual it is supposed to be compared with.
    pub fn equity(&self) -> &crate::stores::Equity {
        self.equity
    }

    pub fn claims(&self) -> &Claims {
        self.claims
    }

    /// What actually happened, to read and never to write.
    pub fn wire(&self) -> &Settlement {
        self.wire
    }

    /// The kinds the KERNEL says its own events under, so a system reads a settlement outcome by
    /// its declared type rather than by a name it spells for itself.
    pub fn kernel_says(&self) -> crate::ledger::Outcomes {
        self.kernel_says
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
        self.not_in_the_week_it_was_decided(why);
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
        self.not_in_the_week_it_was_decided(why);
        self.proposed.push(Proposed {
            legs,
            cause,
            delivery,
            why,
            due: Some(due),
        });
    }

    /// Money G1.c: NOTHING IS CALLED AND PAID IN THE SAME WEEK. Slot h decides what the week's
    /// judgement implies for the week AFTER, so what it has is an obligation to write and never an
    /// instruction to settle — and a due already struck is no exception, because performing it here
    /// is the same week's cash.
    fn not_in_the_week_it_was_decided(&self, why: &'static str) {
        assert!(
            self.at != crate::world::H,
            "Money G1.c: a call settled in the week it was made — {why}. What slot h produces is a \
             payment that FALLS DUE, through `owes`"
        );
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

    /// 49 G4, 38 B5.b: A SHIPMENT ARRIVED — what became of it, and the vehicle that carried it is
    /// now where it delivered. The store of dispatches is on the wire and a position is on the
    /// ground, so a system says what happened and the kernel writes it.
    pub fn delivers(
        &mut self,
        row: u32,
        outcome: crate::mechanisms::freight::DeliveryOutcome,
        aboard: crate::geography::VehicleId,
        at: crate::geography::SiteId,
    ) {
        self.delivered.push((row, outcome, aboard, at));
    }

    /// 49 I3: what a run took OUT OF THE GROUND. What is extracted leaves the tile and does not
    /// come back.
    pub fn extracts(&mut self, tile: crate::geography::TileId, of: InstrumentId, units: f64) {
        self.extracted.push((tile, of, units));
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

    /// What this holder says it is acquiring a position FOR (XI-6). Said before the units arrive,
    /// because settlement refuses units into a position nobody has declared.
    pub fn carries(&mut self, who: PartyId, line: InstrumentId, as_: crate::register::Carrying) {
        self.carried.push((who, line, as_));
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
            delivered: self.delivered,
            extracted: self.extracted,
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
            carried: self.carried,
        }
    }
}

impl Taken {
    /// WHETHER THIS MECHANISM DECIDED ANYTHING, READ OFF WHAT IT ASKED FOR.
    pub fn decided(&self) -> bool {
        // Destructured with no `..`, so a new kind of ask FAILS TO COMPILE until it is accounted
        // for here.
        let Taken {
            // Saying is the one that does NOT count: it is the count.
            said: _,
            proposed,
            delivered,
            extracted,
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
            carried,
        } = self;
        !proposed.is_empty()
            || !owing.is_empty()
            || !formed.is_empty()
            || !ceased.is_empty()
            || !delivered.is_empty()
            || !extracted.is_empty()
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
            || !carried.is_empty()
    }
}

/// Everything one phase asked for, handed back for the kernel to apply.
pub struct Taken {
    pub proposed: Vec<Proposed>,
    /// 49 G4: each shipment that arrived, what became of it, and where its vehicle now is.
    pub delivered: Vec<(
        u32,
        crate::mechanisms::freight::DeliveryOutcome,
        crate::geography::VehicleId,
        crate::geography::SiteId,
    )>,
    /// 49 I3: what left the ground, tile by tile.
    pub extracted: Vec<(crate::geography::TileId, InstrumentId, f64)>,
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
    /// What a holder said it holds a position FOR, before anything is credited to it.
    pub carried: Vec<(PartyId, InstrumentId, crate::register::Carrying)>,
}

/// A system's own work in a week, as opposed to the questions its participants are asked in books.
pub trait Mechanism {
    fn run(&self, ctx: &mut MechanismContext<'_>);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_institutions_reckon_the_same_paper_differently_and_that_is_the_market() {
        // A hundred back in a year. The one that requires more for waiting pays less for it.
        let patient = worth_of(100.0, None, 0.02, 1.0).unwrap();
        let demanding = worth_of(100.0, None, 0.10, 1.0).unwrap();
        assert!(patient > demanding);
        // And one that holds a view of the name marks what it is promised down by it, so it bids
        // under the party that holds no view at all.
        let cautious = worth_of(100.0, Some(0.9), 0.02, 1.0).unwrap();
        assert!(cautious < patient);
        // Nothing here needs a print: it is the contract and the party's own two numbers.
        assert_eq!(worth_of(100.0, None, 0.0, 1.0), Some(100.0));
        // A return that turns waiting into nothing is refused rather than bounded.
        assert_eq!(worth_of(100.0, None, -1.0, 1.0), None);
        assert_eq!(worth_of(100.0, None, -2.0, 1.0), None);
    }

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
