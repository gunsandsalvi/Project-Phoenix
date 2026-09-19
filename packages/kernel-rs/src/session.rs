//! A BOOK, RUN: the participants that named it are asked, one solver clears what they posted, and
//! the fills settle as instructions.

use crate::clearing::{Fill, Order, Outcome as Cleared, Side};
use crate::protocols::Venue;
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::journal::Journal;
use crate::instruments::Instruments;
use crate::ledger::{account_of, Cause, Instruction, Leg, Outcome, Receipt, Settlement, Settling, Units};
use crate::module::{Participant, ParticipantView};
use crate::params::Params;
use crate::parties::Parties;
use crate::prices::{Print, Prints, Provenance, QuotedAs};
use crate::register::Register;
use crate::stores::{Agreements, Schedules};
use std::collections::HashMap;

/// Which parties could be in which books at all, this cycle.
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
    /// Its own relations.
    pub agreements: &'a Agreements,
    /// And what falls due for it and to it, so a party deciding about money it has to find can see
    /// the money it has to find.
    pub schedules: &'a Schedules,
    /// 3 C2, 22c.2: its OWN resting orders.
    pub resting: &'a crate::stores::Resting,
    /// What each party has in flight, so one put in a workout can see that it is.
    pub processes: &'a crate::stores::Processes,
}

impl<'a> Shown<'a> {
    /// One party's view of it, with its own account resolved from the banking lattice.
    pub fn view(&self, who: PartyId, period: u32) -> ParticipantView<'_> {
        ParticipantView::of(
            who,
            self.register,
            self.prints,
            self.journal,
            self.params,
            period,
            account_of(self.parties, self.instruments, who),
        )
        .knowing(self.agreements)
        .owing(self.schedules)
        .resting_in(self.resting)
        .afoot(self.processes)
    }
}

impl Books {
    /// Ask every party of each declaration's kind which books it could be in, and invert it.
    pub fn index(participants: &[&dyn Participant], shown: &Shown<'_>, period: u32) -> Self {
        let mut books = Self::default();
        for (n, p) in participants.iter().enumerate() {
            for &row in shown.parties.of_kind(p.party_kind()) {
                let who = PartyId(row);
                if !shown.parties.alive(who) {
                    continue;
                }
                let view = shown.view(who, period);
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
    pub outcome: Cleared,
    pub asks: usize,
    pub orders: usize,
    pub settled: usize,
    pub failed: usize,
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
    /// The relations a participant may read its OWN of.
    pub agreements: &'a Agreements,
    /// And what falls due, so a participant can see the money it has to find.
    pub schedules: &'a Schedules,
    /// 3 C2, 22c.2: the standing book.
    pub resting: &'a mut crate::stores::Resting,
    /// What is in flight, read by a forced seller.
    pub processes: &'a crate::stores::Processes,
    /// The one calendar, so an order's life is a DATE and never a count of periods kept beside it.
    pub calendar: &'a crate::calendar::Calendar,
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

/// Run one book: ask, clear, print, settle.
pub fn run_book(
    book: &BookDecl,
    participants: &[&dyn Participant],
    books: &Books,
    stores: &mut Stores<'_>,
    period: u32,
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
            agreements: stores.agreements,
            schedules: stores.schedules,
            resting: stores.resting,
            processes: stores.processes,
        };
        for (n, p) in participants.iter().enumerate() {
            for &who in books.who(n, book.market) {
                if !stores.parties.alive(who) {
                    continue;
                }
                for o in p.pulls(&seen.view(who, period), book.market) {
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
        agreements: stores.agreements,
        resting: stores.resting,
        schedules: stores.schedules,
        processes: stores.processes,
    };
    for (n, p) in participants.iter().enumerate() {
        for &who in books.who(n, book.market) {
            if !stores.parties.alive(who) {
                continue;
            }
            asks += 1;
            posted.extend(p.orders(&shown.view(who, period), book.market));
        }
    }
    let orders = posted.len();
    // 3 C2, 22c.2: every session opens with the standing book.
    let standing: Vec<crate::stores::RestingId> = stores.resting.at(book.market.0);
    let resting: Vec<Order> = standing
        .iter()
        .map(|o| Order {
            party: stores.resting.owner(*o),
            side: if stores.resting.buying(*o) { Side::Buy } else { Side::Sell },
            price: stores.resting.level(*o),
            qty: stores.resting.left(*o),
        })
        .collect();
    let outcome =
        crate::protocols::run(book.venue.protocol, &resting, &posted, book.venue.rule, book.venue.seen_by);

    let mut settled = 0usize;
    let mut failed = 0usize;
    if let Cleared::Cleared { price, ref fills, .. } = outcome {
        // The book printed, because real supply met real demand at this level.
        stores.prints.write(Print {
            instrument: book.subject,
            market: book.market,
            period,
            price,
            ccy: book.ccy,
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        });
        // Each trade is an instruction — the units one way, the money the other, together.
        for (buyer, seller, qty, at) in pair_up(fills) {
            // A fill of nothing, or one struck at nothing, is not a trade to settle.
            let (Some(moving), Some(paid)) =
                (Units::new(qty as f64), Units::new(qty as f64 * at))
            else {
                continue;
            };
            let legs = [
                Leg::Asset {
                    from: seller,
                    to: buyer,
                    instrument: book.subject,
                    qty: moving,
                    price_per_unit: Some(at),
                },
                Leg::Money {
                    from: buyer,
                    to: seller,
                    // The buyer pays out of its own account.
                    instrument: match account_of(stores.parties, stores.instruments, buyer) {
                        Some(line) => line,
                        None => panic!("Money D2: {} won a fill in a book and has no account to pay from", buyer.0),
                    },
                    amount: paid,
                    receipt: Receipt::Sale,
                },
            ];
            match stores.wire.settle(
                &Instruction::against_payment(&legs, Cause::Trade),
                period,
                &mut Settling {
                    register: stores.register,
                    journal: stores.journal,
                    parties: stores.parties,
                    instruments: stores.instruments,
                    calendar: stores.calendar,
                    says,
                },
            ) {
                Outcome::Settled => settled += 1,
                // A trade that did not settle is a recorded state, and the book still printed — what
                // cleared, cleared.
                _ => failed += 1,
            }
        }
    }
    // 3 C2, 22c.2: what a match consumed, and what did not fill RESTS.
    let mut took: Vec<(PartyId, bool, i64)> = Vec::new();
    if let Cleared::Cleared { ref fills, .. } = outcome {
        for f in fills {
            let buying = f.side == Side::Buy;
            match took.iter_mut().find(|(p, b, _)| *p == f.party && *b == buying) {
                Some((_, _, q)) => *q += f.qty,
                None => took.push((f.party, buying, f.qty)),
            }
        }
        // What was already resting is consumed first: it was there before the arriving order was.
        for o in &standing {
            let owner = stores.resting.owner(*o);
            let buying = stores.resting.buying(*o);
            let Some(at) = took.iter().position(|(p, b, _)| *p == owner && *b == buying) else {
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
        let until = book.venue.until(stores.calendar, period);
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
                period,
                until,
                book.subject.0,
            );
        }
    }
    Session { outcome, asks, orders, settled, failed }
}

/// Who trades with whom.
fn pair_up(fills: &[Fill]) -> Vec<(PartyId, PartyId, i64, f64)> {
    let mut buys: Vec<(PartyId, i64, f64)> =
        fills.iter().filter(|f| f.side == Side::Buy).map(|f| (f.party, f.qty, f.price)).collect();
    let mut sells: Vec<(PartyId, i64, f64)> =
        fills.iter().filter(|f| f.side == Side::Sell).map(|f| (f.party, f.qty, f.price)).collect();
    let mut out = Vec::new();
    let mut b = 0usize;
    let mut s = 0usize;
    while b < buys.len() && s < sells.len() {
        let take = if buys[b].1 < sells[s].1 { buys[b].1 } else { sells[s].1 };
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

// The period loop has no test, because `world:runs` IS the test and it is not one: it steps the
// assembled world four periods over 51 systems and 54 phases, asks 58,354 times, clears books,
// settles the trades and prints what happened. The three fixtures here built two participants and
// one book to ask whether a book asks, clears, prints and settles, and whether a party that could
// not be in the book is asked — which the real loop answers every run, at every scale, with
// nothing arranged.
