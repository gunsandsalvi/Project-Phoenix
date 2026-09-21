//! THE MARKET SESSIONS, at the world's scale, against the TypeScript engine's measured cost.

use phoenix_kernel::clearing::{Order, PriceRule, Side};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, MarketId, PartyId, RegionId, UnitId};
use phoenix_kernel::instruments::{Class, Instruments};
use phoenix_kernel::journal::Journal;
use phoenix_kernel::ledger::Settlement;
use phoenix_kernel::module::{Participant, ParticipantView};
use phoenix_kernel::params::Params;
use phoenix_kernel::parties::{Parties, Representation};
use phoenix_kernel::prices::Prints;
use phoenix_kernel::register::Register;
use phoenix_kernel::session::{run_book, BookDecl, Books, Shown, Stores};
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

/// It names the books of what it HOLDS — the same read its orders answer out of, so a book it names
/// and a book it posts in cannot disagree.
struct Sells;
impl Participant for Sells {
    fn party_kind(&self) -> u32 {
        SELLER
    }
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // It names the books of what it HOLDS, off the register's by-holder index — not by asking
        // every book in the world whether it is in it.
        let mut out = Vec::new();
        for row in view.holdings() {
            let line = view.line_of(row);
            if line.0 >= 1 && line.0 <= BOOKS as u32 {
                if let Some(market) = view.market_of(line) { out.push(market); }
            }
        }
        out
    }
    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let Some(line) = view.subject_of(m) else { return Vec::new() };
        let held = view.free(line);
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
        // do: 920,404 asks over 1,546 books is 595 a book, not one per party per book.
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
    let bank = parties.add(9, RegionId::at(0), PartyId::at(0), Representation::Named, u32::MAX);
    // Half sell, half buy, at the world's party count.
    for n in 0..PARTIES {
        let kind = if n.is_multiple_of(2) { SELLER } else { BUYER };
        parties.add(kind, RegionId::at(0), bank, Representation::Named, u32::MAX);
    }

    // The cash line is the BANK'S money, and every party above banks there — so no payment here
    // crosses two banks and the interbank leg is not in this measurement.
    let mut instruments = Instruments::new();
    instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);

    let mut register = Register::new();
    let mut prints = Prints::new();
    let mut journal = Journal::new();
    let mut wire = Settlement::new(1);
    let params = Params::new(100.0, 60.0);
    // Predates the relations store and strikes none: a view over it answers "no relations".
    let bench_agreements = phoenix_kernel::stores::Agreements::new();
    let bench_outlooks = phoenix_kernel::stores::Outlooks::new();
    // And owes nothing on a schedule: a view over it answers "nothing falls due".
    let bench_schedules = phoenix_kernel::stores::Schedules::new();
    // And nothing rests in it: a bench measures one session, not a market with a memory.
    let mut bench_resting = phoenix_kernel::stores::Resting::new();
    let bench_calendar = phoenix_kernel::calendar::Calendar::new(phoenix_kernel::calendar::Day(0), 7);
    // And nothing in flight: a bench measures a session, not a world with workouts in it.
    let mut nothing_afoot = phoenix_kernel::stores::Processes::new();
    let bench_standing = phoenix_kernel::stores::Standing::new();
    let says = phoenix_kernel::ledger::Outcomes::declared(&mut journal);

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
    let declared = (1..=BOOKS as u32).map(|n| BookDecl { market: MarketId::at(n), subject: InstrumentId::at(n), ccy: CurrencyCode::at(0), venue: phoenix_kernel::protocols::Venue { rule: PriceRule::SellersCompete, protocol: phoenix_kernel::protocols::Protocol::Call, seen_by: 1, stands_for: None } }).collect::<Vec<_>>();

    let t = Instant::now();
    let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params, outlooks: &bench_outlooks, agreements: &bench_agreements, schedules: &bench_schedules, resting: &bench_resting, processes: &nothing_afoot, standing: &bench_standing, calendar: &bench_calendar, books: &declared }, 1);
    let index_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mut asks = 0usize;
    let mut orders = 0usize;
    let mut settled = 0usize;
    let mut cleared = 0usize;
    {
        let mut stores = Stores {
            parties: &parties,
            instruments: &mut instruments,
            register: &mut register,
            prints: &mut prints,
            journal: &mut journal,
            wire: &mut wire,
            params: &params,
            outlooks: &bench_outlooks,
            agreements: &bench_agreements,
            schedules: &bench_schedules,
            resting: &mut bench_resting,
            processes: &mut nothing_afoot,
            standing: &bench_standing,
            calendar: &bench_calendar,
            books: &declared,
        };
        for n in 1..=BOOKS as u32 {
            let book = BookDecl {
                market: MarketId::at(n),
                subject: InstrumentId::at(n),
                ccy: CurrencyCode::at(0),
                // The bench measures the CALL solver, which is what it always measured.
                venue: phoenix_kernel::protocols::Venue {
                    rule: PriceRule::SellersCompete,
                    protocol: phoenix_kernel::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                },
            };
            let s = run_book(&book, &participants, &books, &mut stores, 1, says);
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
