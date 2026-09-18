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

use crate::clearing::{clear, Fill, Order, Outcome as Cleared, PriceRule, Side};
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
    /// Money D2: the payment system reads it to settle a payment across two banks (`Settling`).
    pub instruments: &'a Instruments,
    pub register: &'a mut Register,
    pub prints: &'a mut Prints,
    pub journal: &'a mut Journal,
    pub wire: &'a mut Settlement,
    pub params: &'a Params,
    /// XI-10: the relations a participant may read its OWN of.
    pub agreements: &'a Agreements,
    /// XI-9, 21j.1: and what falls due, so a participant can see the money it has to find.
    pub schedules: &'a Schedules,
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
    pub rule: PriceRule,
}

/// Run one book: ask, clear, print, settle.
#[allow(clippy::too_many_arguments)]
pub fn run_book(
    book: &BookDecl,
    participants: &[&dyn Participant],
    books: &Books,
    stores: &mut Stores<'_>,
    period: u32,
    settled_kind: u32,
    failed_kind: u32,
    realised_kind: u32,
) -> Session {
    let mut posted: Vec<Order> = Vec::new();
    let mut asks = 0usize;
    let shown = Shown {
        parties: stores.parties,
        instruments: stores.instruments,
        register: stores.register,
        prints: stores.prints,
        journal: stores.journal,
        params: stores.params,
        agreements: stores.agreements,
        schedules: stores.schedules,
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
    let outcome = clear(&posted, book.rule, false);

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
        for (buyer, seller, qty) in pair_up(fills) {
            let legs = [
                Leg::Asset {
                    from: seller,
                    to: buyer,
                    instrument: book.subject,
                    qty: qty as f64,
                    price_per_unit: Some(price),
                },
                Leg::Money {
                    from: buyer,
                    to: seller,
                    ccy: book.ccy,
                    // 22b.9a: the buyer pays out of its own account. A buyer with no account cannot
                    // be in a book at all, and `markets` is where that is decided — so this is
                    // unreachable rather than a case to handle quietly.
                    instrument: match account_of(stores.parties, stores.instruments, buyer) {
                        Some(line) => line,
                        None => panic!("Money D2: {} won a fill in a book and has no account to pay from", buyer.0),
                    },
                    amount: qty as f64 * price,
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
                    realised: realised_kind,
                },
                settled_kind,
                failed_kind,
            ) {
                Outcome::Settled => settled += 1,
                // C4.b: a trade that did not settle is a recorded state, and the book still
                // printed — what cleared, cleared. Nothing is unwound and nothing is invented.
                _ => failed += 1,
            }
        }
    }
    Session { outcome, asks, orders, settled, failed }
}

/// C3: who trades with whom. The solver says how much each side got at the level; this walks the
/// two sides together so every piece bought has a named piece sold against it. **Nobody is left
/// holding a fill with no counterparty** — that is the residual with no holder (Appendix B).
fn pair_up(fills: &[Fill]) -> Vec<(PartyId, PartyId, i64)> {
    let mut buys: Vec<(PartyId, i64)> =
        fills.iter().filter(|f| f.side == Side::Buy).map(|f| (f.party, f.qty)).collect();
    let mut sells: Vec<(PartyId, i64)> =
        fills.iter().filter(|f| f.side == Side::Sell).map(|f| (f.party, f.qty)).collect();
    let mut out = Vec::new();
    let mut b = 0usize;
    let mut s = 0usize;
    while b < buys.len() && s < sells.len() {
        let take = if buys[b].1 < sells[s].1 { buys[b].1 } else { sells[s].1 };
        if take > 0 {
            out.push((buys[b].0, sells[s].0, take));
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

    /// A fixture that strikes no relation. A view over it answers "no relations", which is what a
    /// party with none says — and is a different answer from a view that cannot see them at all.
    fn no_relations() -> Agreements {
        Agreements::new()
    }

    /// And owes nothing on a schedule: a view over it answers "nothing falls due", which is what a
    /// party that owes nothing says.
    fn nothing_due() -> Schedules {
        Schedules::new()
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
        let mut wire = Settlement::new();
        let params = Params::new(100.0, 60.0);
        let ok = journal.kinds.declare("instruction.settled");
        let no = journal.kinds.declare("instruction.failed");

        let cash = InstrumentId::at(0);
        let grain = InstrumentId::at(1);
        let market = MarketId::at(1);
        register.credit(seller, grain, 60.0, 1.0, 0);
        register.money_delta(buyer, cash, 10_000.0);

        let sells = Sells { market, subject: grain, at: 4.0 };
        let buys = Buys { market, cash, at: 5.0, want: 40 };
        let participants: Vec<&dyn Participant> = vec![&sells, &buys];

        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due() }, 1);
        // Two parties, each asked ONCE which books it could be in — not once per book.
        assert_eq!(books.narrows, 2);

        let mut stores = Stores {
            parties: &parties,
            instruments: &instruments,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
            agreements: &no_relations(),
            schedules: &nothing_due(),
        };
        let book = BookDecl { market, subject: grain, ccy: CurrencyCode::at(0), rule: PriceRule::SellersCompete };
        let s = run_book(&book, &participants, &books, &mut stores, 1, ok, no, 0);

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
        let mut wire = Settlement::new();
        let params = Params::new(100.0, 60.0);
        let ok = journal.kinds.declare("instruction.settled");
        let no = journal.kinds.declare("instruction.failed");
        let cash = InstrumentId::at(0);
        let grain = InstrumentId::at(1);
        let market = MarketId::at(1);
        register.credit(seller, grain, 60.0, 1.0, 0);
        register.money_delta(buyer, cash, 10_000.0);

        // The seller wants 9, the buyer will pay 4. Nothing crosses.
        let sells = Sells { market, subject: grain, at: 9.0 };
        let buys = Buys { market, cash, at: 4.0, want: 40 };
        let participants: Vec<&dyn Participant> = vec![&sells, &buys];
        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due() }, 1);
        let mut stores = Stores {
            parties: &parties,
            instruments: &instruments,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
            agreements: &no_relations(),
            schedules: &nothing_due(),
        };
        let book = BookDecl { market, subject: grain, ccy: CurrencyCode::at(0), rule: PriceRule::SellersCompete };
        let s = run_book(&book, &participants, &books, &mut stores, 1, ok, no, 0);
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
        let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, agreements: &no_relations(), schedules: &nothing_due() }, 1);
        assert_eq!(books.narrows, 1, "asked once about itself");
        assert!(books.who(0, MarketId::at(1)).is_empty(), "and named no book");
    }
}
