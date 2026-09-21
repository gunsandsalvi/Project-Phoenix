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
    /// The one calendar, so an order's life is a DATE and never a count of weeks kept beside it.
    pub calendar: &'a crate::calendar::Calendar,
    pub books: &'a [BookDecl],
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

pub fn declared_market(books: &[BookDecl], subject: InstrumentId) -> Option<MarketId> {
    books
        .iter()
        .find(|book| book.subject == subject)
        .map(|book| book.market)
}

#[derive(Clone, Copy)]
struct CarrierBooking {
    carrier: PartyId,
    capacity: f64,
}

struct DispatchPlan {
    route: Option<crate::mechanisms::freight::Route>,
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
            portions: vec![(None, requested)],
        };
    }

    let route = crate::mechanisms::freight::Route { from, to };
    let mut available = Vec::new();
    let mut capacities = Vec::new();
    for row in 0..stores.parties.len() {
        let carrier = PartyId::at(row as u32);
        if !stores.parties.alive(carrier)
            || stores.parties.kind_of(carrier) != crate::assembly::kinds::CARRIER
            || stores.parties.region_of(carrier) != from
        {
            continue;
        }
        let mut capacity = 0.0;
        for holding in stores.register.of_holder(carrier) {
            let holding = crate::ids::HoldingId(*holding);
            let line = stores.register.instrument_of(holding);
            let Some(plant) = stores.registry.plant_of(line) else {
                continue;
            };
            capacity += crate::instruments::capacity(stores.register.lots(holding), &plant, week);
        }
        let used = stores.wire.dispatches.used(week, carrier);
        let room = if used < capacity {
            capacity - used
        } else {
            0.0
        };
        if room > 0.0 {
            available.push((carrier, room));
            capacities.push((carrier, capacity));
        }
    }
    let fitted = crate::mechanisms::freight::fit_dispatch(requested, &available)
        .into_iter()
        .map(|(carrier, units)| {
            let capacity = capacities
                .iter()
                .find(|(candidate, _)| *candidate == carrier)
                .map(|(_, capacity)| *capacity)
                .expect("a fitted carrier came from the capacity list");
            (Some(CarrierBooking { carrier, capacity }), units)
        })
        .collect();
    DispatchPlan {
        route: Some(route),
        portions: fitted,
    }
}

/// Run one book: ask, clear, print, settle.
pub fn run_book(
    book: &BookDecl,
    participants: &[&dyn Participant],
    books: &Books,
    stores: &mut Stores<'_>,
    week: u32,
    says: crate::ledger::Outcomes,
) -> Session {
    let mut posted: Vec<Order> = Vec::new();
    let mut asks = 0usize;
    // 3 C2, 22c2.3: WHAT THE PARTIES PULL, before anybody is asked for a new order.
    let mut pulled: Vec<(PartyId, crate::stores::RestingId)> = Vec::new();
    {
        let seen = Shown {
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
    };
    for (n, p) in participants.iter().enumerate() {
        for &who in books.who(n, book.market) {
            asks += 1;
            posted.extend(p.orders(&shown.view(who, week), book.market));
        }
    }
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
    if let Cleared::Cleared {
        price, ref fills, ..
    } = outcome
    {
        // The book printed, because real supply met real demand at this level.
        stores.prints.write(Print {
            instrument: book.subject,
            market: book.market,
            week,
            price,
            ccy: book.ccy,
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        });
        // BOND N9.b: THE PRICE IS QUOTED CLEAN AND WHAT SETTLES IS CLEAN PLUS ACCRUED. What the
        // seller earned on the coupon running now is the seller's; without it the coupon is a
        // windfall to whoever happens to hold the paper on the date. Read once for the book,
        // because every fill in it is on the same line.
        let today = stores.calendar.at(crate::calendar::Week(i64::from(week)));
        let accrued_per_unit = match stores.schedules.accruing(book.subject, today) {
            None => 0.0,
            Some(d) => {
                let issuer = stores.instruments.issuer_of(book.subject);
                // The same denominator the payment itself uses, so what the buyer pre-pays and what
                // it is paid on the date are the same number.
                let outstanding: f64 = stores
                    .register
                    .of_instrument(book.subject)
                    .iter()
                    .map(|r| crate::ids::HoldingId(*r))
                    .filter(|row| stores.register.holder_of(*row) != issuer)
                    .map(|row| stores.register.quantity(row))
                    .sum();
                match (stores.schedules.accrued(d, today), outstanding > 0.0) {
                    (Some(accrued), true) => accrued / outstanding,
                    _ => 0.0,
                }
            }
        };
        // Each trade is an instruction — the units one way, the money the other, together.
        for (buyer, seller, qty, at) in pair_up(fills) {
            let dispatch = dispatch_plan(stores, buyer, seller, book.subject, week, qty as f64);
            for (booking, portion) in dispatch.portions {
                // A fill of nothing, or one struck at nothing, is not a trade to settle.
                let (Some(moving), Some(paid)) = (Units::new(portion), Units::new(portion * at))
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
                if let (Some(booking), Some(on)) = (booking, dispatch.route) {
                    legs.push(Leg::Dispatch {
                        shipper: seller,
                        consignee: buyer,
                        owner: buyer,
                        carrier: booking.carrier,
                        instrument: book.subject,
                        from: on.from,
                        to: on.to,
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
                    },
                ) {
                    Outcome::Settled => {
                        settled += 1;
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
        assert_eq!(
            declared_market(&books, InstrumentId::at(7)),
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
                price: None,
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
