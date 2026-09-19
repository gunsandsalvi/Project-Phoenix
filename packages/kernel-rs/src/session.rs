//! A BOOK, RUN: the participants that named it are asked, one solver clears what they posted, and
//! the fills settle as instructions (Clearing B2, C1, C4, XI-5).
//!
//! **Trades are instructions.** A fill is not a state change — it is a pair of legs that go over
//! the wire like everything else, so delivery-versus-payment holds for a cleared book exactly as it
//! does for a payment, and a book whose trades could not settle prints nothing.
//!
//! **Who is asked is settled ONCE A CYCLE, not once a book.** `Books` asks every party of a kind
//! which books it could be in at all and inverts the answer — the port of the kernel's `asked`
//! index. In TypeScript the door was optional and absent meant every book of the kind, so the
//! quadratic was the default; here `Participant::markets` is required and `everyone` is the
//! explicit way to say a book is open to all.

use crate::clearing::{Fill, Order, Outcome as Cleared, Side};
use crate::protocols::Venue;
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::journal::Journal;
use crate::instruments::Instruments;
use crate::ledger::{account_of, Cause, Instruction, Leg, Outcome, Receipt, Settlement, Settling};
use crate::module::{Participant, ParticipantView};
use crate::params::Params;
use crate::parties::Parties;
use crate::prices::{Print, Prints, Provenance, QuotedAs};
use crate::register::Register;
use crate::stores::{Agreements, Schedules};
use std::collections::HashMap;

/// Clearing B2: which parties could be in which books at all, this cycle. One question per party
/// per declaration, inverted — never one question per party per book.
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

/// **What a participant may be shown**: the stores a view is built from, together.
///
/// One struct because a view is built from all of them at once, and a caller made to name six is a
/// caller that will one day name five and not notice. Every field is a READ — a participant never
/// writes a store (ARCHITECTURE 4.9b).
pub struct Shown<'a> {
    pub parties: &'a Parties,
    pub instruments: &'a Instruments,
    pub register: &'a Register,
    pub prints: &'a Prints,
    pub journal: &'a Journal,
    pub params: &'a Params,
    /// XI-10: its own relations. A mandate, an engagement, a policy is a fact about THIS party, so
    /// a participant may read its own and no other's (Observer A4).
    pub agreements: &'a Agreements,
    /// XI-9, 21j.1: and what falls due for it and to it, so a party deciding about money it has to
    /// find can see the money it has to find.
    pub schedules: &'a Schedules,
    /// 3 C2, 22c.2: **its OWN resting orders.** A party that could not see what it already has in
    /// a venue would re-enter it every session and stand behind twice what it meant to (Observer A4).
    pub resting: &'a crate::stores::Resting,
    /// XI-2, 22i.3: what each party has in flight, so one put in a workout can see that it is.
    pub processes: &'a crate::stores::Processes,
}

impl<'a> Shown<'a> {
    /// One party's view of it, with its own account resolved from the banking lattice (22b.9a).
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
    /// Money D2: the payment system reads it to settle a payment across two banks — and since 0l.2
    /// it WRITES the issued amount there too, because a trade's legs reach `Settling` (Register B1).
    pub instruments: &'a mut Instruments,
    pub register: &'a mut Register,
    pub prints: &'a mut Prints,
    pub journal: &'a mut Journal,
    pub wire: &'a mut Settlement,
    pub params: &'a Params,
    /// XI-10: the relations a participant may read its OWN of.
    pub agreements: &'a Agreements,
    /// XI-9, 21j.1: and what falls due, so a participant can see the money it has to find.
    pub schedules: &'a Schedules,
    /// **3 C2, 22c.2: the standing book.** Every session opens with what was already resting, and
    /// what an arriving order did not fill stays. It is written here because a match CONSUMES a
    /// resting order and the session is what matched it — the same reason settlement writes the
    /// register (Law 4).
    pub resting: &'a mut crate::stores::Resting,
    /// XI-2, 22i.3: what is in flight, read by a forced seller.
    pub processes: &'a crate::stores::Processes,
    /// **G3.a, 22c2.2: the one calendar**, so an order's life is a DATE and never a count of periods
    /// kept beside it. The assembled world had no calendar at all — it counted periods — which is
    /// why nothing in it could expire.
    pub calendar: &'a crate::calendar::Calendar,
}

pub struct BookDecl {
    pub market: MarketId,
    /// What the book delivers. A book with no subject delivers nothing anybody holds (a pair).
    pub subject: InstrumentId,
    /// **The money of this book is a CURRENCY, not one bank's deposits** (22b.9a). A book used to
    /// name one instrument, and with one deposit line per bank (Money D2) that shut every customer
    /// of every other bank out of the market entirely — three quarters of the cells in the first
    /// warm-up were in no book at all. Each side pays out of ITS OWN account, which settlement
    /// resolves from the banking lattice exactly as it resolves the payee's (`ledger::account_of`).
    pub ccy: CurrencyCode,
    /// **3 A1, 22c.1: WHAT KIND OF PLACE THIS IS**, declared by whoever opened it — its rule, its
    /// protocol, what a buyer can see of it and how long an order stands in it. There was one
    /// microstructure — a weekly uniform-price call auction — for bread, labour, loans, shares and
    /// freight alike, and a Walrasian auctioneer for bread is the one intermediary that never
    /// existed (Law 1). The kernel dispatches on this and never on what is being traded (Law 15).
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
    // **3 C2, 22c2.3: WHAT THE PARTIES PULL, before anybody is asked for a new order.** An order
    // rested until somebody took it away and nothing ever did, so a seller whose stock had perished
    // went on standing behind units it had not got. The party decides and the kernel applies it —
    // `cancels` refuses anybody but the owner, and a book that pulled orders on a party's behalf
    // would be deciding for it.
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
    // **3 C2, 22c.2: every session opens with the standing book.** Nothing rested between sessions
    // before, which is the whole of why `noDemand` (7,903) dwarfed `noOverlap` (323): the two sides
    // were not failing to agree on a price, **they were failing to be in the room in the same week.**
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
        // Law 3: the book printed, because real supply met real demand at this level.
        stores.prints.write(Print {
            instrument: book.subject,
            market: book.market,
            period,
            price,
            ccy: book.ccy,
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        });
        // XI-5: each trade is an instruction — the units one way, the money the other, together.
        for (buyer, seller, qty, at) in pair_up(fills) {
            let legs = [
                Leg::Asset {
                    from: seller,
                    to: buyer,
                    instrument: book.subject,
                    qty: qty as f64,
                    price_per_unit: Some(at),
                },
                Leg::Money {
                    from: buyer,
                    to: seller,
                    // 22b.9a: the buyer pays out of its own account. A buyer with no account cannot
                    // be in a book at all, and `markets` is where that is decided — so this is
                    // unreachable rather than a case to handle quietly.
                    instrument: match account_of(stores.parties, stores.instruments, buyer) {
                        Some(line) => line,
                        None => panic!("Money D2: {} won a fill in a book and has no account to pay from", buyer.0),
                    },
                    amount: qty as f64 * at,
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
                // C4.b: a trade that did not settle is a recorded state, and the book still
                // printed — what cleared, cleared. Nothing is unwound and nothing is invented.
                _ => failed += 1,
            }
        }
    }
    // **3 C2, 22c.2: what a match consumed, and what did not fill RESTS.** A resting order that was
    // partly taken is a smaller order and not a filled one, and an arriving order nobody met stays
    // in the venue until its owner pulls it or its own date expires it — which is what stops a
    // market having no memory from one week to the next.
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
    // **3 C2, 22c.2: AND WHAT ARRIVED AND DID NOT FILL RESTS.** In a shop the ask is still on the
    // shelf next week; on an exchange the bid is still in the book. In a CALL it is gone, because a
    // sealed cross is an event and the event is over — which is why a treasury whose auction failed
    // must come back rather than find its bid still standing.
    if book.venue.protocol.rests() {
        // 22c2.2: the day it stands to, taken from the calendar at the moment it is entered.
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

/// C3: who trades with whom. The solver says how much each side got at the level; this walks the
/// two sides together so every piece bought has a named piece sold against it. **Nobody is left
/// holding a fill with no counterparty** — that is the residual with no holder (Appendix B).
/// **22c.1: the pairs carry THEIR OWN price, not the session's.** A call auction crosses everybody
/// at one level, so its fills all carry it — but a posted market and a resting book do not have one
/// level at all: each trade happened at what that seller was standing behind. Settling every trade
/// at the session's last print would be inventing a price for the ones that did not happen at it
/// (Law 3), which is what this returned before there was more than one protocol.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clearing::PriceRule;

    /// A fixture that strikes no relation. A view over it answers "no relations", which is what a
    /// And rests nothing: a fixture for a venue with no memory, which is what every venue was.
    fn nothing_resting() -> crate::stores::Resting {
        crate::stores::Resting::new()
    }

    /// And nothing in flight: a view over it answers "no workout", which is what a party in none says.
    fn nothing_afoot() -> crate::stores::Processes {
        crate::stores::Processes::new()
    }
    /// party with none says — and is a different answer from a view that cannot see them at all.
    fn no_relations() -> Agreements {
        Agreements::new()
    }

    /// And owes nothing on a schedule: a view over it answers "nothing falls due", which is what a
    /// party that owes nothing says.
    fn nothing_due() -> Schedules {
        Schedules::new()
    }

    /// G3: the one calendar these tests run against — a seven-day period from day zero.
    fn weekly() -> crate::calendar::Calendar {
        crate::calendar::Calendar::new(crate::calendar::Day(0), 7, 3)
    }

    /// A sealed cross: nothing rests in it, so it declares no life for an order.
    fn a_call() -> Venue {
        Venue {
            rule: PriceRule::SellersCompete,
            protocol: crate::protocols::Protocol::Call,
            seen_by: 1,
            stands_for: None,
        }
    }
    use crate::instruments::Class;
    use crate::parties::Representation;
    use crate::register::Register;

    const SELLER_KIND: u32 = 0;
    const BUYER_KIND: u32 = 1;

    struct Sells {
        market: MarketId,
        subject: InstrumentId,
        at: f64,
    }
    impl Participant for Sells {
        fn party_kind(&self) -> u32 {
            SELLER_KIND
        }
        fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
            // Law 19: it names the book of what it HOLDS — the same read `orders` answers out of.
            if view.free(self.subject) > 0.0 {
                vec![self.market]
            } else {
                vec![]
            }
        }
        fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
            let held = view.free(self.subject);
            if held <= 0.0 {
                return vec![];
            }
            vec![Order { party: view.self_id(), side: Side::Sell, price: Some(self.at), qty: held as i64 }]
        }
    }

    struct Buys {
        market: MarketId,
        cash: InstrumentId,
        at: f64,
        want: i64,
    }
    impl Participant for Buys {
        fn party_kind(&self) -> u32 {
            BUYER_KIND
        }
        fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
            if view.quantity(self.cash) > 0.0 {
                vec![self.market]
            } else {
                vec![]
            }
        }
        fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
            vec![Order { party: view.self_id(), side: Side::Buy, price: Some(self.at), qty: self.want }]
        }
    }

    #[test]
    fn a_book_asks_clears_prints_and_settles_and_every_piece_has_two_sides() {
        let mut parties = Parties::new();
        let bank = parties.add(9, crate::ids::RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        let seller = parties.add(SELLER_KIND, crate::ids::RegionId::at(0), bank, Representation::Named, 1, u32::MAX);
        let buyer = parties.add(BUYER_KIND, crate::ids::RegionId::at(0), bank, Representation::Named, 1, u32::MAX);

        // Money D2: the cash line is the bank's money and both sides bank there, so this trade does
        // not cross two banks.
        let mut instruments = Instruments::new();
        instruments.issue(bank, CurrencyCode::at(0), Class::Money, crate::ids::UnitId::at(0), None, None);

        let mut register = Register::new();
        let mut prints = Prints::new();
        let mut journal = Journal::new();
        let mut wire = Settlement::new(6);
        let params = Params::new(100.0, 60.0);
        let says = crate::ledger::Outcomes::declared(&mut journal);

        let cash = InstrumentId::at(0);
        let grain = InstrumentId::at(1);
        let market = MarketId::at(1);
        register.credit(seller, grain, 60.0, 1.0, 0);
        register.money_delta(buyer, cash, 10_000.0);

        let sells = Sells { market, subject: grain, at: 4.0 };
        let buys = Buys { market, cash, at: 5.0, want: 40 };
        let participants: Vec<&dyn Participant> = vec![&sells, &buys];

        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due(), resting: &nothing_resting(), processes: &nothing_afoot() }, 1);
        // Two parties, each asked ONCE which books it could be in — not once per book.
        assert_eq!(books.narrows, 2);

        let mut stores = Stores {
            parties: &parties,
            instruments: &mut instruments,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
            agreements: &no_relations(),
            schedules: &nothing_due(),
            resting: &mut nothing_resting(),
            processes: &nothing_afoot(),
            calendar: &weekly(),
        };
        let book = BookDecl { market, subject: grain, ccy: CurrencyCode::at(0), venue: a_call() };
        let s = run_book(&book, &participants, &books, &mut stores, 1, says);

        assert_eq!(s.asks, 2);
        assert_eq!(s.orders, 2);
        assert_eq!(s.settled, 1);
        assert_eq!(s.failed, 0);
        match s.outcome {
            Cleared::Cleared { price, volume, .. } => {
                // Sellers compete: the level is the seller's, and 40 is what crossed.
                assert_eq!(price, 4.0);
                assert_eq!(volume, 40);
            }
            other => panic!("{other:?}"),
        }
        // The book printed, and the print is what cleared.
        assert_eq!(prints.latest(grain, 1).unwrap().price, 4.0);
        // Delivery-versus-payment: the units went one way and the money the other, together.
        assert_eq!(register.quantity(register.row(buyer, grain)), 40.0);
        assert_eq!(register.quantity(register.row(seller, grain)), 20.0);
        assert_eq!(register.quantity(register.row(buyer, cash)), 10_000.0 - 160.0);
        assert_eq!(register.quantity(register.row(seller, cash)), 160.0);
    }

    #[test]
    fn a_book_that_did_not_cross_prints_nothing() {
        let mut parties = Parties::new();
        let bank = parties.add(9, crate::ids::RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        let seller = parties.add(SELLER_KIND, crate::ids::RegionId::at(0), bank, Representation::Named, 1, u32::MAX);
        let buyer = parties.add(BUYER_KIND, crate::ids::RegionId::at(0), bank, Representation::Named, 1, u32::MAX);

        // Money D2: the cash line is the bank's money and both sides bank there, so this trade does
        // not cross two banks.
        let mut instruments = Instruments::new();
        instruments.issue(bank, CurrencyCode::at(0), Class::Money, crate::ids::UnitId::at(0), None, None);

        let mut register = Register::new();
        let mut prints = Prints::new();
        let mut journal = Journal::new();
        let mut wire = Settlement::new(6);
        let params = Params::new(100.0, 60.0);
        let says = crate::ledger::Outcomes::declared(&mut journal);
        let cash = InstrumentId::at(0);
        let grain = InstrumentId::at(1);
        let market = MarketId::at(1);
        register.credit(seller, grain, 60.0, 1.0, 0);
        register.money_delta(buyer, cash, 10_000.0);

        // The seller wants 9, the buyer will pay 4. Nothing crosses.
        let sells = Sells { market, subject: grain, at: 9.0 };
        let buys = Buys { market, cash, at: 4.0, want: 40 };
        let participants: Vec<&dyn Participant> = vec![&sells, &buys];
        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due(), resting: &nothing_resting(), processes: &nothing_afoot() }, 1);
        let mut stores = Stores {
            parties: &parties,
            instruments: &mut instruments,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
            agreements: &no_relations(),
            schedules: &nothing_due(),
            resting: &mut nothing_resting(),
            processes: &nothing_afoot(),
            calendar: &weekly(),
        };
        let book = BookDecl { market, subject: grain, ccy: CurrencyCode::at(0), venue: a_call() };
        let s = run_book(&book, &participants, &books, &mut stores, 1, says);
        assert!(matches!(s.outcome, Cleared::NoOverlap { .. }));
        assert_eq!(s.settled, 0);
        // Law 3: a bracket is not a price, so the book printed NOTHING.
        assert!(prints.latest(grain, 1).is_none());
        // And nothing moved.
        assert_eq!(register.quantity(register.row(seller, grain)), 60.0);
    }

    #[test]
    fn a_party_that_could_not_be_in_the_book_is_never_asked() {
        let mut parties = Parties::new();
        let bank = parties.add(9, crate::ids::RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
        // A seller holding nothing: its own door says it is in no book, so it is never asked.
        let _empty = parties.add(SELLER_KIND, crate::ids::RegionId::at(0), bank, Representation::Named, 1, u32::MAX);
        let register = Register::new();
        let instruments = Instruments::new();
        let prints = Prints::new();
        let journal = Journal::new();
        let params = Params::new(100.0, 60.0);
        let sells = Sells { market: MarketId::at(1), subject: InstrumentId::at(1), at: 4.0 };
        let participants: Vec<&dyn Participant> = vec![&sells];
        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due(), resting: &nothing_resting(), processes: &nothing_afoot() }, 1);
        assert_eq!(books.narrows, 1, "asked once about itself");
        assert!(books.who(0, MarketId::at(1)).is_empty(), "and named no book");
    }
}
