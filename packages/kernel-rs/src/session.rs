//! A BOOK, RUN: the participants that named it are asked, one solver clears what they posted, and
//! the fills settle as instructions.

use crate::clearing::{Fill, Order, Outcome as Cleared, Side};
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::instruments::Instruments;
use crate::journal::Journal;
use crate::ledger::{
    account_of, Cause, Instruction, Leg, Outcome, Receipt, Settlement, Settling, Units,
};
use crate::module::{Participant, ParticipantView, ViewInputs};
use crate::params::Params;
use crate::parties::Parties;
use crate::prices::{Print, Prints, Provenance, QuotedAs};
use crate::protocols::Venue;
use crate::register::Register;
use crate::stores::{Agreements, Outlooks, Schedules};
use std::collections::HashMap;

/// Which parties could be in which books at all, this week.
#[derive(Default)]
pub struct Books {
    asked: HashMap<u64, Vec<PartyId>>,
    /// How many questions this cost, so the quadratic cannot hide (`work.ts`'s lesson).
    pub narrows: usize,
}

#[inline]
const fn slot(decl: usize, market: MarketId) -> u64 {
    ((decl as u64) << 32) | (market.0 as u64)
}

/// What a participant may be shown: the stores a view is built from, together.
pub struct Shown<'a> {
    pub parties: &'a Parties,
    pub instruments: &'a Instruments,
    pub register: &'a Register,
    pub prints: &'a Prints,
    pub journal: &'a Journal,
    pub params: &'a Params,
    pub outlooks: &'a Outlooks,
    /// Its own relations.
    pub agreements: &'a Agreements,
    /// And what falls due for it and to it, so a party deciding about money it has to find can see
    /// the money it has to find.
    pub schedules: &'a Schedules,
    /// 3 C2, 22c.2: its OWN resting orders.
    pub resting: &'a crate::stores::Resting,
    /// What each party has in flight, so one put in a workout can see that it is.
    pub processes: &'a crate::stores::Processes,
    /// Durable mandates and standards held by the party being shown.
    pub standing: &'a crate::stores::Standing,
    /// The one calendar, so a party reads what week it is.
    pub calendar: &'a crate::calendar::Calendar,
    pub books: &'a [BookDecl],
    pub registry: &'a crate::registry::Registry,
    /// 46 F3: what each family of thing is worth, answered by the system that owns it.
    pub valuers: &'a [Box<dyn crate::module::Valuer>],
    /// 49 C4: where each party stands, and what joins it to anywhere else.
    pub geography: &'a crate::geography::Geography,
}

impl<'a> Shown<'a> {
    /// One party's view of it, with its own account resolved from the banking lattice.
    pub fn view(&self, who: PartyId, week: u32) -> ParticipantView<'_> {
        ParticipantView::of(
            who,
            ViewInputs {
                register: self.register,
                instruments: self.instruments,
                prints: self.prints,
                journal: self.journal,
                params: self.params,
                week,
                cash: account_of(self.parties, self.instruments, who),
                calendar: self.calendar,
                outlooks: self.outlooks,
                outlook_memory: self.parties.outlook_memory(who),
                population_weight: self.parties.weight(who),
                household_keeps: self.parties.household_keeps(who),
                books: self.books,
                parties: self.parties,
                registry: self.registry,
                valuers: self.valuers,
                geography: self.geography,
            },
        )
        .knowing(self.agreements)
        .owing(self.schedules)
        .resting_in(self.resting)
        .afoot(self.processes)
        .standing_by(self.standing)
    }
}

impl Books {
    /// Ask every party of each declaration's kind which books it could be in, and invert it.
    pub fn index(participants: &[&dyn Participant], shown: &Shown<'_>, week: u32) -> Self {
        let mut books = Self::default();
        for (n, p) in participants.iter().enumerate() {
            for who in p.eligible_parties(shown.parties) {
                let view = shown.view(who, week);
                books.narrows += 1;
                for m in p.markets(&view) {
                    books.asked.entry(slot(n, m)).or_default().push(who);
                }
            }
        }
        books
    }

    fn who(&self, decl: usize, market: MarketId) -> &[PartyId] {
        match self.asked.get(&slot(decl, market)) {
            Some(rows) => rows,
            None => &[],
        }
    }
}

/// What one book did, and what it cost to find out.
pub struct Session {
    pub market: MarketId,
    pub subject: InstrumentId,
    pub week: u32,
    /// The orders submitted in this session. Retaining them is what distinguishes an auction that
    /// was never attempted from one that was attempted and only partly taken.
    pub submitted: Vec<Order>,
    pub outcome: Cleared,
    pub asks: usize,
    pub orders: usize,
    pub settled: usize,
    pub failed: usize,
    /// Reconciliation totals from settlement outcomes, never from proposed fills.
    pub bought: f64,
    pub sold: f64,
    pub cash_paid: f64,
    pub cash_received: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct AuctionProgress {
    pub asked: i64,
    pub filled: i64,
    pub proceeds: f64,
}

impl Session {
    /// What one seller actually offered and sold. This remains readable after the book closes,
    /// including zero-demand and partial-auction outcomes.
    pub fn auction_of(&self, seller: PartyId) -> Option<AuctionProgress> {
        let asked: i64 = self
            .submitted
            .iter()
            .filter(|order| order.party == seller && order.side == Side::Sell)
            .map(|order| order.qty)
            .sum();
        if asked == 0 {
            return None;
        }
        let (filled, proceeds) = match &self.outcome {
            Cleared::Cleared { fills, .. } => fills
                .iter()
                .filter(|fill| fill.party == seller && fill.side == Side::Sell)
                .fold((0, 0.0), |(units, money), fill| {
                    (units + fill.qty, money + fill.qty as f64 * fill.price)
                }),
            Cleared::NoDemand | Cleared::NoSupply | Cleared::NoOverlap { .. } => (0, 0.0),
        };
        Some(AuctionProgress {
            asked,
            filled,
            proceeds,
        })
    }
}

/// The kernel's stores, handed to a session together because a book touches all of them.
pub struct Stores<'a> {
    pub parties: &'a Parties,
    /// The payment system reads it to settle a payment across two banks — and since 0l.2 it WRITES
    /// the issued amount there too, because a trade's legs reach `Settling`.
    pub instruments: &'a mut Instruments,
    pub register: &'a mut Register,
    pub prints: &'a mut Prints,
    pub journal: &'a mut Journal,
    pub wire: &'a mut Settlement,
    pub params: &'a Params,
    pub outlooks: &'a Outlooks,
    /// The relations a participant may read its OWN of.
    pub agreements: &'a Agreements,
    /// And what falls due, so a participant can see the money it has to find.
    pub schedules: &'a Schedules,
    /// 3 C2, 22c.2: the standing book.
    pub resting: &'a mut crate::stores::Resting,
    /// What is in flight, read by a forced seller.
    pub processes: &'a mut crate::stores::Processes,
    /// Durable party-owned mandates and standards.
    pub standing: &'a crate::stores::Standing,
    /// Carrier technology and party kinds used to derive physical dispatch capacity.
    pub registry: &'a crate::registry::Registry,
    /// Where places are and what joins them, so a dispatch names a route rather than two regions.
    pub geography: &'a crate::geography::Geography,
    /// The one calendar, so an order's life is a DATE and never a count of weeks kept beside it.
    pub calendar: &'a crate::calendar::Calendar,
    pub books: &'a [BookDecl],
    /// 46 F3: what each family of thing is worth, answered by the system that owns it.
    pub valuers: &'a [Box<dyn crate::module::Valuer>],
    /// Audit B5: settlement moves it as each fill's legs apply.
    pub equity: &'a mut crate::stores::Equity,
}

/// Apply settled plant consideration to the matching programme, in opening order. A different
/// buyer or line cannot finish it, and consideration beyond the commitment is ignored.
fn fulfil_programmes(
    processes: &mut crate::stores::Processes,
    buyer: PartyId,
    line: InstrumentId,
    mut invested: f64,
) {
    let programmes = processes
        .running(crate::stores::afoot::CAPITAL_PROGRAMME)
        .into_iter()
        .filter(|process| {
            processes.owner(*process) == buyer && processes.subject(*process) == Some(line)
        })
        .collect::<Vec<_>>();
    for process in programmes {
        if invested <= 0.0 {
            break;
        }
        let remaining = processes.size(process);
        if remaining <= 0.0 {
            continue;
        }
        let applied = if invested < remaining {
            invested
        } else {
            remaining
        };
        processes.funds(process, applied);
        invested -= applied;
    }
}

pub struct BookDecl {
    pub market: MarketId,
    /// What the book delivers.
    pub subject: InstrumentId,
    /// 21 A1.a, Keys: WHERE this book is. A good is its sub-unit and a market in it is
    /// (region, sub-unit), so the same grade in two places is two books and two prices. A line
    /// whose identity does not include a place — paper, a share, a currency — is `None` and has
    /// one book everywhere.
    pub at: Option<crate::ids::RegionId>,
    /// The money of this book is a CURRENCY, not one bank's deposits.
    pub ccy: CurrencyCode,
    /// 3 A1, 22c.1: WHAT KIND OF PLACE THIS IS, declared by whoever opened it — its rule, its
    /// protocol, what a buyer can see of it and how long an order stands in it.
    pub venue: Venue,
}

pub fn declared_subject(books: &[BookDecl], market: MarketId) -> Option<InstrumentId> {
    books
        .iter()
        .find(|book| book.market == market)
        .map(|book| book.subject)
}

/// 21 A1.a: THE BOOK FOR THESE UNITS WHERE THEY ARE. A line whose identity includes a place has a
/// book in each one; a line whose does not has one book, and every place reads that same one.
pub fn book_here(
    books: &[BookDecl],
    subject: InstrumentId,
    at: crate::ids::RegionId,
) -> Option<MarketId> {
    books
        .iter()
        .find(|book| book.subject == subject && book.at == Some(at))
        .or_else(|| {
            books
                .iter()
                .find(|book| book.subject == subject && book.at.is_none())
        })
        .map(|book| book.market)
}

/// What kind of place this is, so a participant posts what the venue admits rather than what
/// another venue would have taken.
pub fn declared_venue(books: &[BookDecl], market: MarketId) -> Option<crate::protocols::Venue> {
    books
        .iter()
        .find(|book| book.market == market)
        .map(|book| book.venue)
}

#[derive(Clone, Copy)]
struct CarrierBooking {
    carrier: PartyId,
    aboard: crate::geography::VehicleId,
    capacity: f64,
    /// 49 F4: the weeks THIS vehicle takes over this route. A slow one arrives later.
    transit: u32,
}

struct DispatchPlan {
    route: Option<crate::geography::RouteId>,
    /// 38 A1: the carriage line that route's room is made of.
    carriage: Option<InstrumentId>,
    portions: Vec<(Option<CarrierBooking>, f64)>,
}

fn dispatch_plan(
    stores: &Stores<'_>,
    buyer: PartyId,
    seller: PartyId,
    subject: InstrumentId,
    week: u32,
    requested: f64,
) -> DispatchPlan {
    let from = stores.parties.region_of(seller);
    let to = stores.parties.region_of(buyer);
    if stores.instruments.class_of(subject) != crate::instruments::Class::Good || from == to {
        return DispatchPlan {
            route: None,
            carriage: None,
            portions: vec![(None, requested)],
        };
    }

    // 49 G1: the route is the one laid over the network, never a pair of regions named here.
    let Some(route) = stores
        .geography
        .place_of(from)
        .zip(stores.geography.place_of(to))
        .and_then(|(origin, destination)| stores.geography.route_between(origin, destination))
    else {
        return DispatchPlan {
            route: None,
            carriage: None,
            portions: vec![(None, requested)],
        };
    };
    // 38 B8: what can carry it is the VEHICLES standing where it is — not a carrier's holdings
    // summed from wherever they happen to be.
    let Some(origin) = stores
        .geography
        .place_of(from)
        .and_then(|site| stores.geography.tile_of(site))
    else {
        return DispatchPlan {
            route: None,
            carriage: None,
            portions: vec![(None, requested)],
        };
    };
    // 49 F4: the transit is the ROUTE'S length against what each vehicle covers in a week, so an
    // unmeasured route carries nothing rather than everything arriving next week.
    let Some(length) = stores.geography.length_of(route) else {
        return DispatchPlan {
            route: None,
            carriage: None,
            portions: vec![(None, requested)],
        };
    };
    let loading = stores.params.weeks("freight.loading_weeks");
    let standing: Vec<crate::geography::Vehicle> =
        stores.geography.vehicles_at(origin).copied().collect();
    let mut available = Vec::new();
    let mut capacities = Vec::new();
    for vehicle in standing {
        let Some(carrier) = stores
            .register
            .of_instrument(vehicle.line)
            .iter()
            .map(|row| stores.register.holder_of(crate::ids::HoldingId(*row)))
            .find(|holder| holder.some())
        else {
            continue;
        };
        if !stores.parties.alive(carrier) {
            continue;
        }
        let Some(plant) = stores.registry.plant_of(vehicle.line) else {
            continue;
        };
        let Some(speed) = stores.registry.speed_of(vehicle.line) else {
            continue;
        };
        let holding = stores.register.row(carrier, vehicle.line);
        let capacity = crate::instruments::capacity(stores.register.lots(holding), &plant, week);
        let used = stores.wire.dispatches.used(week, vehicle.id);
        let room = if used < capacity {
            capacity - used
        } else {
            0.0
        };
        if room > 0.0 {
            available.push((vehicle.id, carrier, room));
            capacities.push((
                vehicle.id,
                capacity,
                crate::mechanisms::freight::arrives_in(length.0, speed, loading),
            ));
        }
    }
    let fitted = crate::mechanisms::freight::fit_dispatch(
        requested,
        &available
            .iter()
            .map(|(id, _, room)| (*id, *room))
            .collect::<Vec<_>>(),
    )
    .into_iter()
    .map(|(aboard, units)| {
        let (capacity, transit) = capacities
            .iter()
            .find(|(candidate, _, _)| *candidate == aboard)
            .map(|(_, capacity, transit)| (*capacity, *transit))
            .expect("a fitted vehicle came from the capacity list");
        let carrier = available
            .iter()
            .find(|(candidate, _, _)| *candidate == aboard)
            .map(|(_, carrier, _)| *carrier)
            .expect("a fitted vehicle came from the available list");
        (
            Some(CarrierBooking {
                carrier,
                aboard,
                capacity,
                transit,
            }),
            units,
        )
    })
    .collect();
    DispatchPlan {
        route: Some(route),
        carriage: stores
            .registry
            .carriage()
            .iter()
            .find(|(on, _)| *on == route)
            .map(|(_, line)| *line),
        portions: fitted,
    }
}

/// Put a buyer's own declaration on the position it is about to acquire. A buyer that was not
/// asked this week — one whose order has been resting since a week it was — declared then, and
/// settlement refuses units into a position nobody ever declared.
fn said_for(
    register: &mut crate::register::Register,
    declared: &[(PartyId, crate::register::Carrying)],
    who: PartyId,
    line: InstrumentId,
) {
    if let Some((_, as_)) = declared.iter().find(|(party, _)| *party == who) {
        register.carry(who, line, *as_);
    }
}

/// Money G2.e: WHAT A BOOK WAS HANDED once every party had formed its view — the orders posted into
/// it, and what each poster said it would hold what it bought here FOR. The book clears a stage
/// later, so this is what survives between the two.
pub struct Posted {
    pub market: MarketId,
    pub orders: Vec<Order>,
    pub declared: Vec<(PartyId, crate::register::Carrying)>,
    pub asks: usize,
}

/// Ask one book's parties what they want (Money G2.e). Nothing clears here: a party that reads a
/// price this stage produced would be reading the answer to the question it is being asked.
pub fn ask_book(
    book: &BookDecl,
    participants: &[&dyn Participant],
    books: &Books,
    stores: &mut Stores<'_>,
    week: u32,
) -> Posted {
    let mut posted: Vec<Order> = Vec::new();
    let mut asks = 0usize;
    // 3 C2, 22c2.3: WHAT THE PARTIES PULL, before anybody is asked for a new order.
    let mut pulled: Vec<(PartyId, crate::stores::RestingId)> = Vec::new();
    {
        let seen = Shown {
            geography: stores.geography,
            parties: stores.parties,
            instruments: stores.instruments,
            register: stores.register,
            prints: stores.prints,
            journal: stores.journal,
            params: stores.params,
            outlooks: stores.outlooks,
            agreements: stores.agreements,
            schedules: stores.schedules,
            resting: stores.resting,
            processes: &*stores.processes,
            standing: stores.standing,
            calendar: stores.calendar,
            books: stores.books,
            registry: stores.registry,
            valuers: stores.valuers,
        };
        for (n, p) in participants.iter().enumerate() {
            for &who in books.who(n, book.market) {
                for o in p.pulls(&seen.view(who, week), book.market) {
                    pulled.push((who, o));
                }
            }
        }
    }
    for (who, o) in pulled {
        stores.resting.cancels(o, who);
    }

    // And the book the rest of the session reads is the one the pulls left behind.
    let shown = Shown {
        geography: stores.geography,
        parties: stores.parties,
        instruments: stores.instruments,
        register: stores.register,
        prints: stores.prints,
        journal: stores.journal,
        params: stores.params,
        outlooks: stores.outlooks,
        agreements: stores.agreements,
        resting: stores.resting,
        schedules: stores.schedules,
        processes: &*stores.processes,
        standing: stores.standing,
        calendar: stores.calendar,
        books: stores.books,
        registry: stores.registry,
        valuers: stores.valuers,
    };
    // XI-6: and what each of them would hold what it buys here FOR, asked with the order because
    // the declaration belongs to the acquisition a fill would be.
    let mut declared: Vec<(PartyId, crate::register::Carrying)> = Vec::new();
    for (n, p) in participants.iter().enumerate() {
        for &who in books.who(n, book.market) {
            asks += 1;
            let view = shown.view(who, week);
            posted.extend(p.orders(&view, book.market));
            if let Some(as_) = p.carries(&view, book.market) {
                match declared.iter().find(|(party, _)| *party == who) {
                    // Law 4: one position, one treatment. Two systems that both acquire for a
                    // party here and disagree would leave the register holding whichever of them
                    // happened to be asked first.
                    Some((_, said)) => assert!(
                        *said == as_,
                        "XI-6: party {} is told it holds market {} as {:?} and as {:?}",
                        who.0,
                        book.market.0,
                        said,
                        as_
                    ),
                    None => declared.push((who, as_)),
                }
            }
        }
    }
    Posted {
        market: book.market,
        orders: posted,
        declared,
        asks,
    }
}

/// Run one book: clear what was posted into it, print, settle.
pub fn run_book(
    book: &BookDecl,
    said: &Posted,
    stores: &mut Stores<'_>,
    week: u32,
    says: crate::ledger::Outcomes,
) -> Session {
    let posted = said.orders.clone();
    let declared = &said.declared;
    let asks = said.asks;
    let orders = posted.len();
    // 3 C2, 22c.2: every session opens with the standing book.
    let standing: Vec<crate::stores::RestingId> = stores.resting.at(book.market.0);
    let resting: Vec<Order> = standing
        .iter()
        .map(|o| Order {
            party: stores.resting.owner(*o),
            side: if stores.resting.buying(*o) {
                Side::Buy
            } else {
                Side::Sell
            },
            price: stores.resting.level(*o),
            qty: stores.resting.left(*o),
        })
        .collect();
    let outcome = crate::protocols::run(
        book.venue.protocol,
        &resting,
        &posted,
        book.venue.rule,
        book.venue.seen_by,
    );

    let mut settled = 0usize;
    let mut failed = 0usize;
    let mut bought = 0.0;
    let mut sold = 0.0;
    let mut cash_paid = 0.0;
    let mut cash_received = 0.0;
    if let Cleared::Cleared {
        price, ref fills, ..
    } = outcome
    {
        // BOND N9.b: THE PRICE IS QUOTED CLEAN AND WHAT SETTLES IS CLEAN PLUS ACCRUED. What the
        // seller earned on the coupon running now is the seller's; without it the coupon is a
        // windfall to whoever happens to hold the paper on the date. Read once for the book,
        // because every fill in it is on the same line.
        let today = stores.calendar.at(crate::calendar::Week(i64::from(week)));
        let accruing = crate::module::accrued_per_unit(
            stores.register,
            stores.schedules,
            stores.instruments,
            stores.prints,
            book.subject,
            today,
        );
        // A line with nothing accruing adds nothing to the clean price.
        let accrued_per_unit = match accruing {
            Some(per_unit) => per_unit,
            None => 0.0,
        };
        let paired = pair_up(fills);
        // A primary equity offering is all-or-nothing.  The book supplies the cleared price and
        // named subscribers; only settlement may create the subscribed units, in the very same
        // instruction that pays their cash to the issuer.
        let flotation = paired.first().and_then(|(_, seller, _, _)| {
            stores
                .processes
                .running(crate::stores::afoot::FLOTATION)
                .into_iter()
                .find(|process| {
                    stores.processes.owner(*process) == *seller
                        && stores.processes.subject(*process) == Some(book.subject)
                })
        });
        if let Some(process) = flotation {
            let issuer = stores.processes.owner(process);
            let subscriptions = paired
                .iter()
                .filter(|(_, seller, _, _)| *seller == issuer)
                .map(
                    |(buyer, _, qty, _)| crate::mechanisms::equity::Subscription {
                        investor: *buyer,
                        shares: *qty as f64,
                    },
                )
                .collect::<Vec<_>>();
            if let Some(offering) = crate::mechanisms::equity::offering(
                stores.processes.size(process),
                price,
                &subscriptions,
            ) {
                let mut legs = Vec::with_capacity(offering.allocations.len() * 2);
                for allocation in &offering.allocations {
                    let Some(qty) = Units::new(allocation.shares) else {
                        continue;
                    };
                    let Some(amount) = Units::new(allocation.shares * offering.price) else {
                        continue;
                    };
                    let account =
                        account_of(stores.parties, stores.instruments, allocation.investor)
                            .expect("Money D2: an equity subscriber needs an account");
                    said_for(stores.register, declared, allocation.investor, book.subject);
                    legs.push(Leg::Create {
                        party: allocation.investor,
                        instrument: book.subject,
                        qty,
                        cost_per_unit: offering.price,
                    });
                    legs.push(Leg::Money {
                        from: allocation.investor,
                        to: issuer,
                        instrument: account,
                        amount,
                        // A subscription is capital paid IN, and it is the one receipt the payee
                        // keeps: what the investor handed over came back as the shares it bought.
                        receipt: Receipt::Capital,
                    });
                }
                match stores.wire.settle(
                    &Instruction::against_payment(&legs, Cause::CorporateAction),
                    week,
                    &mut Settling {
                        register: stores.register,
                        journal: stores.journal,
                        parties: stores.parties,
                        instruments: stores.instruments,
                        calendar: stores.calendar,
                        says,
                        equity: stores.equity,
                    },
                ) {
                    Outcome::Settled => {
                        settled += 1;
                        let units: f64 = offering.allocations.iter().map(|a| a.shares).sum();
                        bought += units;
                        sold += units;
                        cash_paid += offering.raised;
                        cash_received += offering.raised;
                        stores.processes.fulfils(process, offering.raised);
                    }
                    _ => failed += 1,
                }
            }
        } else {
            // Each ordinary trade is an instruction — the units one way, the money the other, together.
            for (buyer, seller, qty, at) in paired {
                let dispatch = dispatch_plan(stores, buyer, seller, book.subject, week, qty as f64);
                for (booking, portion) in dispatch.portions {
                    // A fill of nothing, or one struck at nothing, is not a trade to settle.
                    let (Some(moving), Some(paid)) =
                        (Units::new(portion), Units::new(portion * at))
                    else {
                        continue;
                    };
                    // The buyer pays out of its own account.
                    let account = match account_of(stores.parties, stores.instruments, buyer) {
                        Some(line) => line,
                        None => panic!(
                            "Money D2: {} won a fill in a book and has no account to pay from",
                            buyer.0
                        ),
                    };
                    said_for(stores.register, declared, buyer, book.subject);
                    let mut legs = vec![
                        Leg::Asset {
                            from: seller,
                            to: buyer,
                            instrument: book.subject,
                            // The basis is what it paid for the PAPER: accrued is interest pre-paid, not
                            // part of what the position cost.
                            qty: moving,
                            price_per_unit: Some(at),
                        },
                        Leg::Money {
                            from: buyer,
                            to: seller,
                            instrument: account,
                            amount: paid,
                            receipt: Receipt::Sale,
                        },
                    ];
                    // A separate leg, because it is interest and not the price — and it says so.
                    if let Some(accrued) = Units::new(portion * accrued_per_unit) {
                        legs.push(Leg::Money {
                            from: buyer,
                            to: seller,
                            instrument: account,
                            amount: accrued,
                            receipt: Receipt::Interest,
                        });
                    }
                    // Secured paper carries its collateral as a TERM, so a fill encumbers that
                    // much of it in the same instruction: the lender's lien and the money move
                    // together or neither does.
                    if let Some(pledge) = stores.instruments.collateral_of(book.subject) {
                        if let Some(units) = Units::new(portion * pledge.per_unit) {
                            legs.push(Leg::Pledge {
                                holder: seller,
                                instrument: pledge.line,
                                to: buyer,
                                qty: units,
                            });
                        }
                    }
                    if let (Some(booking), Some(on), Some(carriage)) =
                        (booking, dispatch.route, dispatch.carriage)
                    {
                        legs.push(Leg::Dispatch {
                            shipper: seller,
                            consignee: buyer,
                            owner: buyer,
                            carrier: booking.carrier,
                            aboard: booking.aboard,
                            instrument: book.subject,
                            on,
                            carriage,
                            transit: booking.transit,
                            qty: moving,
                            carrier_capacity: booking.capacity,
                        });
                    }
                    match stores.wire.settle(
                        &Instruction::against_payment(&legs, Cause::Trade),
                        week,
                        &mut Settling {
                            register: stores.register,
                            journal: stores.journal,
                            parties: stores.parties,
                            instruments: stores.instruments,
                            calendar: stores.calendar,
                            says,
                            equity: stores.equity,
                        },
                    ) {
                        Outcome::Settled => {
                            settled += 1;
                            bought += portion;
                            sold += portion;
                            cash_paid += paid.get();
                            cash_received += paid.get();
                            // A capital programme is denominated in money. Only a settled purchase of its
                            // named plant reduces the commitment; a failed fill or another asset cannot
                            // complete it.
                            fulfil_programmes(stores.processes, buyer, book.subject, paid.get());
                            let flotations = stores
                                .processes
                                .running(crate::stores::afoot::FLOTATION)
                                .into_iter()
                                .filter(|process| {
                                    stores.processes.owner(*process) == seller
                                        && stores.processes.subject(*process) == Some(book.subject)
                                })
                                .collect::<Vec<_>>();
                            let mut issued = portion;
                            for process in flotations {
                                if issued <= 0.0 {
                                    break;
                                }
                                let applied = if issued < stores.processes.size(process) {
                                    issued
                                } else {
                                    stores.processes.size(process)
                                };
                                stores.processes.fulfils(process, applied);
                                issued -= applied;
                            }
                            let processes = stores
                                .processes
                                .running(crate::stores::afoot::WORKOUT)
                                .into_iter()
                                .filter(|process| {
                                    stores.processes.owner(*process) == seller
                                        && stores.processes.subject(*process) == Some(book.subject)
                                })
                                .collect::<Vec<_>>();
                            let mut left = portion;
                            for process in processes {
                                if left <= 0.0 {
                                    break;
                                }
                                let remaining = stores.processes.size(process);
                                let applied = if left < remaining { left } else { remaining };
                                let realised = paid.get() * applied / portion;
                                stores.processes.realises(process, applied, realised);
                                left -= applied;
                            }
                        }
                        // A trade that did not settle is a recorded state, and the book still printed — what
                        // cleared, cleared.
                        _ => failed += 1,
                    }
                }
            }
        }
    }
    // A clearing level becomes the public mark only when at least one matched transfer became
    // final.  Otherwise the last public level is carried with provenance that exposes its age.
    if settled > 0 {
        if let Cleared::Cleared { price, .. } = outcome {
            stores.prints.write(Print {
                instrument: book.subject,
                market: book.market,
                week,
                // What crossed, crossed here and now.
                struck: week,
                price,
                ccy: book.ccy,
                quoted_as: QuotedAs::Money,
                provenance: Provenance::Cleared,
            });
        }
    } else if let Some(previous) =
        stores
            .prints
            .latest(book.market, book.subject, week.saturating_sub(1))
    {
        // The level is the one it already had, so it keeps the week that level was struck in and
        // the carry is visible as the distance between the two.
        stores.prints.write(Print {
            week,
            provenance: Provenance::Carried,
            ..previous
        });
    }
    // 3 C2, 22c.2: what a match consumed, and what did not fill RESTS.
    let mut took: Vec<(PartyId, bool, i64)> = Vec::new();
    if let Cleared::Cleared { ref fills, .. } = outcome {
        for f in fills {
            let buying = f.side == Side::Buy;
            match took
                .iter_mut()
                .find(|(p, b, _)| *p == f.party && *b == buying)
            {
                Some((_, _, q)) => *q += f.qty,
                None => took.push((f.party, buying, f.qty)),
            }
        }
        // What was already resting is consumed first: it was there before the arriving order was.
        for o in &standing {
            let owner = stores.resting.owner(*o);
            let buying = stores.resting.buying(*o);
            let Some(at) = took
                .iter()
                .position(|(p, b, _)| *p == owner && *b == buying)
            else {
                continue;
            };
            let left = stores.resting.left(*o);
            let fill = if took[at].2 < left { took[at].2 } else { left };
            if fill > 0 {
                stores.resting.took(*o, fill);
                took[at].2 -= fill;
            }
        }
    }
    // 3 C2, 22c.2: AND WHAT ARRIVED AND DID NOT FILL RESTS.
    if book.venue.protocol.rests() {
        // The day it stands to, taken from the calendar at the moment it is entered.
        let until = book.venue.until(stores.calendar, week);
        for o in &posted {
            let filled = took
                .iter()
                .find(|(p, b, _)| *p == o.party && *b == (o.side == Side::Buy))
                .map_or(0, |(_, _, q)| *q);
            let left = o.qty - filled;
            if left <= 0 {
                continue;
            }
            stores.resting.enters(
                o.party,
                book.market.0,
                o.side == Side::Buy,
                o.price,
                left,
                week,
                until,
                book.subject.0,
            );
        }
    }
    Session {
        market: book.market,
        subject: book.subject,
        week,
        submitted: posted,
        outcome,
        asks,
        orders,
        settled,
        failed,
        bought,
        sold,
        cash_paid,
        cash_received,
    }
}

/// Who trades with whom.
fn pair_up(fills: &[Fill]) -> Vec<(PartyId, PartyId, i64, f64)> {
    let mut buys: Vec<(PartyId, i64, f64)> = fills
        .iter()
        .filter(|f| f.side == Side::Buy)
        .map(|f| (f.party, f.qty, f.price))
        .collect();
    let mut sells: Vec<(PartyId, i64, f64)> = fills
        .iter()
        .filter(|f| f.side == Side::Sell)
        .map(|f| (f.party, f.qty, f.price))
        .collect();
    let mut out = Vec::new();
    let mut b = 0usize;
    let mut s = 0usize;
    while b < buys.len() && s < sells.len() {
        let take = if buys[b].1 < sells[s].1 {
            buys[b].1
        } else {
            sells[s].1
        };
        if take > 0 {
            // The two sides of one trade agree on its price by construction — the protocol wrote
            // both legs of it — so the buyer's is the trade's, and a disagreement is unsayable.
            out.push((buys[b].0, sells[s].0, take, buys[b].2));
        }
        buys[b].1 -= take;
        sells[s].1 -= take;
        if buys[b].1 == 0 {
            b += 1;
        }
        if sells[s].1 == 0 {
            s += 1;
        }
    }
    out
}

// The week loop has no test, because `world:runs` IS the test and it is not one: it steps the
// assembled world four weeks over 51 systems and 54 phases, asks 58,354 times, clears books,
// settles the trades and prints what happened. The three fixtures here built two participants and
// one book to ask whether a book asks, clears, prints and settles, and whether a party that could
// not be in the book is asked — which the real loop answers every run, at every scale, with
// nothing arranged.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declaration_resolves_ids_that_do_not_share_a_row() {
        let books = [BookDecl {
            market: MarketId::at(41),
            subject: InstrumentId::at(7),
            at: None,
            ccy: CurrencyCode::at(2),
            venue: Venue {
                rule: crate::clearing::PriceRule::SellersCompete,
                protocol: crate::protocols::Protocol::Call,
                seen_by: 1,
                stands_for: None,
            },
        }];
        assert_eq!(
            declared_subject(&books, MarketId::at(41)),
            Some(InstrumentId::at(7))
        );
        // An unplaced line has one book, and every place reads that same one.
        assert_eq!(
            book_here(&books, InstrumentId::at(7), crate::ids::RegionId::at(3)),
            Some(MarketId::at(41))
        );
        assert_eq!(declared_subject(&books, MarketId::at(7)), None);
    }

    #[test]
    fn a_partial_auction_retains_what_was_asked_filled_and_raised() {
        let seller = PartyId::at(3);
        let session = Session {
            market: MarketId::at(4),
            subject: InstrumentId::at(7),
            week: 9,
            submitted: vec![Order {
                party: seller,
                side: Side::Sell,
                price: Some(0.95),
                qty: 100,
            }],
            outcome: Cleared::Cleared {
                price: 0.95,
                volume: 40,
                fills: vec![
                    Fill {
                        party: seller,
                        side: Side::Sell,
                        qty: 40,
                        price: 0.95,
                    },
                    Fill {
                        party: PartyId::at(8),
                        side: Side::Buy,
                        qty: 40,
                        price: 0.95,
                    },
                ],
                rationed: crate::clearing::Rationed::Sell,
                demand_at_price: 40,
                supply_at_price: 100,
            },
            asks: 2,
            orders: 2,
            settled: 1,
            failed: 0,
            bought: 40.0,
            sold: 40.0,
            cash_paid: 38.0,
            cash_received: 38.0,
        };

        assert_eq!(
            session.auction_of(seller),
            Some(AuctionProgress {
                asked: 100,
                filled: 40,
                proceeds: 38.0
            })
        );
    }

    #[test]
    fn only_settled_consideration_for_the_named_plant_completes_a_programme() {
        let buyer = PartyId::at(3);
        let plant = InstrumentId::at(8);
        let other = InstrumentId::at(9);
        let mut processes = crate::stores::Processes::new();
        let programme = processes.begin_for(
            crate::stores::afoot::CAPITAL_PROGRAMME,
            buyer,
            1,
            Some(4),
            100.0,
            crate::stores::ProcessTarget {
                door: None,
                subject: Some(plant),
            },
        );
        fulfil_programmes(&mut processes, buyer, other, 100.0);
        assert_eq!(processes.size(programme), 100.0);
        fulfil_programmes(&mut processes, buyer, plant, 60.0);
        assert_eq!(processes.size(programme), 40.0);
        fulfil_programmes(&mut processes, buyer, plant, 50.0);
        assert_eq!(processes.size(programme), 0.0);
        assert!(!processes.done(programme));
        processes.finish(programme);
        assert!(processes.done(programme));
    }
}
