//! THE MARKET SESSIONS, at the world's scale, against the TypeScript engine's measured cost.
//!
//! A period runs **1,546 books**, asks **920,404 participant questions** and gets back the orders
//! that clear them. TypeScript's `runOne` — the ask, the clear and the settle for one book — is
//! **5,453 ms inclusive, 9.97% of a period**, of which the SOLVER is 38 ms: the cost is the asking.
//!
//! Every figure here is a median of five, because a cold run is not a measurement (0g.43).

use phoenix_kernel::clearing::{Order, PriceRule, Side};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, MarketId, PartyId, RegionId};
use phoenix_kernel::journal::Journal;
use phoenix_kernel::ledger::Settlement;
use phoenix_kernel::module::{Participant, ParticipantView};
use phoenix_kernel::params::Params;
use phoenix_kernel::parties::{Parties, Representation};
use phoenix_kernel::prices::Prints;
use phoenix_kernel::register::Register;
use phoenix_kernel::session::{BookDecl, Books, Stores, run_book};
use std::time::Instant;

const PARTIES: u32 = 10_318;
const BOOKS: usize = 1_546;
const HOLDINGS: usize = 544_104;
/// TypeScript, measured: `runOne` inclusive over a period, and the questions it asked.
const TS_MS: f64 = 5453.0;
const TS_ASKS: usize = 920_404;

const SELLER: u32 = 0;
const BUYER: u32 = 1;
const CASH: InstrumentId = InstrumentId(0);
/// How many books a buyer has a reason to be in, so the ask count is the world's 920,404.
const WANTS_PER_BUYER: u32 = 125;

struct Draw(u64);
impl Draw {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

/// Law 19: it names the books of what it HOLDS — the same read its orders answer out of, so a book
/// it names and a book it posts in cannot disagree.
struct Sells;
impl Participant for Sells {
    fn party_kind(&self) -> u32 {
        SELLER
    }
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // Law 19, Law 18: it names the books of what it HOLDS, off the register's by-holder index
        // — not by asking every book in the world whether it is in it.
        let mut out = Vec::new();
        for row in view.holdings() {
            let line = view.line_of(row);
            if line.0 >= 1 && line.0 <= BOOKS as u32 {
                out.push(MarketId::at(line.0));
            }
        }
        out
    }
    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let held = view.free(InstrumentId::at(m.0));
        if held <= 0.0 {
            return vec![];
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(4.0), qty: held as i64 }]
    }
}

struct Buys;
impl Participant for Buys {
    fn party_kind(&self) -> u32 {
        BUYER
    }
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // A buyer names the books it has a REASON to be in, which is what the world's participants
        // do: 920,404 asks over 1,546 books is 595 a book, not one per party per book. A first
        // version of this named EVERY book for every buyer and asked 8,509,599 questions — the
        // quadratic, built into the bench that was meant to measure it.
        if view.quantity(CASH) <= 0.0 {
            return vec![];
        }
        let me = view.self_id().0;
        (0..WANTS_PER_BUYER)
            .map(|n| MarketId::at(1 + (me.wrapping_mul(2_654_435_761).wrapping_add(n)) % BOOKS as u32))
            .collect()
    }
    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(5.0), qty: 2 }]
    }
}

fn main() {
    let mut draw = Draw(0x5EED_0F00_D1CE_B00C);
    let mut parties = Parties::new();
    let bank = parties.add(9, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
    // Half sell, half buy, at the world's party count.
    for n in 0..PARTIES {
        let kind = if n.is_multiple_of(2) { SELLER } else { BUYER };
        parties.add(kind, RegionId::at(0), bank, Representation::Named, 1, u32::MAX);
    }

    let mut register = Register::new();
    let mut prints = Prints::new();
    let mut journal = Journal::new();
    let mut wire = Settlement::new();
    let params = Params::new(100.0, 60.0);
    let ok = journal.kinds.declare("instruction.settled");
    let no = journal.kinds.declare("instruction.failed");

    for row in 0..parties.len() as u32 {
        register.money_delta(PartyId::at(row), CASH, 1_000_000.0);
    }
    // 544,104 holdings over the books, held by the sellers.
    while register.rows() < HOLDINGS {
        let p = PartyId::at(1 + draw.below(PARTIES));
        if parties.kind_of(p) != SELLER {
            continue;
        }
        let i = InstrumentId::at(1 + draw.below(BOOKS as u32));
        if register.row(p, i).some() {
            continue;
        }
        register.credit(p, i, 100.0, 1.0, 0);
    }

    let sells = Sells;
    let buys = Buys;
    let participants: Vec<&dyn Participant> = vec![&sells, &buys];

    let t = Instant::now();
    let books = Books::index(&participants, &parties, &register, &prints, &journal, &params, 1);
    let index_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mut asks = 0usize;
    let mut orders = 0usize;
    let mut settled = 0usize;
    let mut cleared = 0usize;
    {
        let mut stores = Stores {
            parties: &parties,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
        };
        for n in 1..=BOOKS as u32 {
            let book = BookDecl {
                market: MarketId::at(n),
                subject: InstrumentId::at(n),
                ccy: CurrencyCode::at(0),
                cash: CASH,
                rule: PriceRule::SellersCompete,
            };
            let s = run_book(&book, &participants, &books, &mut stores, 1, ok, no);
            asks += s.asks;
            orders += s.orders;
            settled += s.settled;
            if matches!(s.outcome, phoenix_kernel::clearing::Outcome::Cleared { .. }) {
                cleared += 1;
            }
        }
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0;

    println!("{BOOKS} books · {} parties · {} holdings", parties.len(), register.rows());
    println!("  narrowing index  {index_ms:7.1} ms  {} questions, one per party per declaration", books.narrows);
    println!("  sessions         {ms:7.1} ms  {asks} asks -> {orders} orders, {cleared} books cleared, {settled} trades settled");
    println!();
    println!("TypeScript `runOne`, measured  {TS_MS:8.1} ms over {TS_ASKS} asks");
    println!("this session loop              {:8.1} ms   {:5.1}x", ms + index_ms, TS_MS / (ms + index_ms));
    println!("per ask                        {:8.0} ns", (ms + index_ms) * 1e6 / asks as f64);
}
