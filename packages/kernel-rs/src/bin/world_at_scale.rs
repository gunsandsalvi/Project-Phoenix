//! A WHOLE PERIOD OF THE WORLD, at every count the real one has. **This is the number the
//! migration is judged on.**
//!
//! From `npm run world 1` on the TypeScript engine, which takes **42.1 s** (median of three):
//!   10,318 parties · 16,750 instruments · **544,104 holdings** · 1,546 books
//!   **920,404 participant questions** · **48,828 instructions carrying 501,044 legs**
//!   **178,604 events** · an audit over every holding
//!
//! Every figure printed here is a median of five runs, because a cold run is not a measurement in
//! either direction (0g.43, learned by publishing two that were).
//!
//! What it does NOT have is forty-six of the forty-seven modules. The sessions ask a real
//! participant and the audit runs a real family, so the SHAPE of a period is here and its content
//! is one module deep. That is stated rather than buried: the honest reading is the floor plus what
//! 0g.40 measured the module block at, not this number on its own.

use phoenix_kernel::audit::{ATotalCarriesNoLots, Audit, LotsAgainstQuantity, NoCollateralCountedTwice};
use phoenix_kernel::calendar::{Calendar, Day};
use phoenix_kernel::clearing::{Order, PriceRule, Side};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, MarketId, PartyId, RegionId, UnitId};
use phoenix_kernel::journal::{Journal, Value};
use phoenix_kernel::ledger::{Cause, Instruction, Leg, Receipt, Settlement, Settling};
use phoenix_kernel::instruments::{Class, Instruments};
use phoenix_kernel::mechanisms::capital_programme::PlantMoves;
use phoenix_kernel::module::{Participant, ParticipantView};
use phoenix_kernel::params::Params;
use phoenix_kernel::parties::{Parties, Representation};
use phoenix_kernel::prices::Prints;
use phoenix_kernel::register::Register;
use phoenix_kernel::session::{run_book, BookDecl, Books, Shown, Stores};
use phoenix_kernel::world::Clock;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
const BOOKS: usize = 1_546;
const INSTRUCTIONS: usize = 48_828;
const LEGS: usize = 501_044;
const EVENTS: usize = 178_604;
/// How many books a party has a reason to be in, sized so the asks come to the world's 920,404.
const REASONS: u32 = 169;
const TS_MS: f64 = 42_100.0;

const SELLER: u32 = 0;
const BUYER: u32 = 1;
const CASH: InstrumentId = InstrumentId(0);

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

struct Sells;
impl Participant for Sells {
    fn party_kind(&self) -> u32 {
        SELLER
    }
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // Law 19: off its OWN rows, never by asking every book in the world.
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
        // It named this book because it holds the line; whether it SELLS this period is a second
        // decision, and most periods it does not.
        if !(view.self_id().0 ^ m.0).is_multiple_of(5) {
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
        if view.quantity(CASH) <= 0.0 {
            return vec![];
        }
        let me = view.self_id().0;
        (0..REASONS)
            .map(|n| MarketId::at(1 + (me.wrapping_mul(2_654_435_761).wrapping_add(n)) % BOOKS as u32))
            .collect()
    }
    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        // **Most asks post NOTHING**, which is the world's own shape: a session asks every party
        // of a kind whether it has an order in it and the answer is almost always no. A bench
        // where every asked party posts settles nineteen times the world's instructions and is
        // measuring a different economy.
        if !(view.self_id().0 ^ m.0).is_multiple_of(7) {
            return vec![];
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(5.0), qty: 40 }]
    }
}

fn main() {
    let mut draw = Draw(0xC0FF_EE15_600D_1DEA);

    // ---- assembly ---------------------------------------------------------------------------
    let t = Instant::now();
    let mut parties = Parties::new();
    let bank = parties.add(9, RegionId::at(0), PartyId::at(0), Representation::Named, 1, u32::MAX);
    for n in 0..PARTIES {
        let kind = if n.is_multiple_of(2) { SELLER } else { BUYER };
        parties.add(kind, RegionId::at(0), bank, Representation::Named, 1, u32::MAX);
    }
    // Money D2: the cash line is the BANK'S money and every party banks there, so no payment here
    // crosses two banks. The interbank leg is not in this measurement, and that is stated.
    let mut instruments = Instruments::new();
    instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);

    let mut register = Register::new();
    let mut prints = Prints::new();
    let mut journal = Journal::new();
    let mut wire = Settlement::new();
    let params = Params::new(100.0, 60.0);
    let mut clock = Clock::new(Calendar::new(Day(0), 7, 3));
    let ok = journal.kinds.declare("instruction.settled");
    let no = journal.kinds.declare("instruction.failed");
    let said = journal.kinds.declare("module.said");
    let amount = journal.keys_named.declare("amount");

    for row in 0..parties.len() as u32 {
        register.money_delta(PartyId::at(row), CASH, 100_000_000.0);
    }
    let mut plant: Vec<(u32, u32)> = Vec::new();
    while register.rows() < HOLDINGS {
        let p = PartyId::at(1 + draw.below(PARTIES));
        if parties.kind_of(p) != SELLER {
            continue;
        }
        let i = InstrumentId::at(1 + draw.below(INSTRUMENTS - 1));
        if register.row(p, i).some() {
            continue;
        }
        register.credit(p, i, 1_000.0, 1.0, 0);
        if i.0 <= 479 {
            plant.push((p.0, i.0));
        }
    }
    let mut capital = vec![false; INSTRUMENTS as usize];
    for line in capital.iter_mut().take(480).skip(1) {
        *line = true;
    }
    let mut audit = Audit::new();
    audit.add(Box::<LotsAgainstQuantity>::default());
    audit.add(Box::<NoCollateralCountedTwice>::default());
    audit.add(Box::<ATotalCarriesNoLots>::default());
    audit.add(Box::new(PlantMoves::over(capital)));
    audit.run(&register, &wire, 0);
    let assembly_ms = t.elapsed().as_secs_f64() * 1000.0;

    let sells = Sells;
    let buys = Buys;
    let participants: Vec<&dyn Participant> = vec![&sells, &buys];

    // ---- THE PERIOD -------------------------------------------------------------------------
    let began = Instant::now();
    clock.step();
    let period = clock.period.0;

    let t = Instant::now();
    let books = Books::index(&participants, &Shown { parties: &parties, instruments: &instruments, register: &register, prints: &prints, journal: &journal, params: &params }, period);
    let index_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let mut asks = 0usize;
    let mut trades = 0usize;
    let mut cleared = 0usize;
    {
        let mut stores = Stores {
            parties: &parties,
            instruments: &instruments,
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
                rule: PriceRule::SellersCompete,
            };
            let s = run_book(&book, &participants, &books, &mut stores, period, ok, no);
            asks += s.asks;
            trades += s.settled;
            if matches!(s.outcome, phoenix_kernel::clearing::Outcome::Cleared { .. }) {
                cleared += 1;
            }
        }
    }
    let sessions_ms = t.elapsed().as_secs_f64() * 1000.0;

    // What the rest of a period settles that is not a trade: wages, coupons, taxes, estates.
    let t = Instant::now();
    let mut legs_left = LEGS.saturating_sub(trades * 2);
    while wire.in_period(period).len() < INSTRUCTIONS && legs_left > 20 {
        let here = (2 + (draw.next() % 18) as usize).min(legs_left);
        let mut legs: Vec<Leg> = Vec::with_capacity(here);
        for _ in 0..here {
            legs.push(Leg::Money {
                from: PartyId::at(1 + draw.below(PARTIES)),
                to: PartyId::at(1 + draw.below(PARTIES)),
                ccy: CurrencyCode::at(0),
                instrument: CASH,
                amount: ((draw.next() % 10_000) as f64) / 100.0,
                receipt: Receipt::Wage,
            });
            legs_left -= 1;
        }
        wire.settle(&Instruction::plain(&legs, Cause::Payment), period, &mut Settling { register: &mut register, journal: &mut journal, parties: &parties, instruments: &instruments }, ok, no);
    }
    let wire_ms = t.elapsed().as_secs_f64() * 1000.0;

    // And what the modules say about themselves, to the period's event count.
    let t = Instant::now();
    while journal.len() < EVENTS {
        journal.say(period, 0, said, &[draw.below(PARTIES)], &[(amount, Value::Num(1.0))], true);
    }
    let journal_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let reports = audit.run(&register, &wire, period);
    let audit_ms = t.elapsed().as_secs_f64() * 1000.0;
    let found: usize = reports.iter().map(|r| r.violations.len()).sum();

    let period_ms = began.elapsed().as_secs_f64() * 1000.0;

    println!("assembly {assembly_ms:.0} ms — {} parties, {} holdings", parties.len(), register.rows());
    println!("  narrowing  {index_ms:7.1} ms   {} questions", books.narrows);
    println!("  sessions   {sessions_ms:7.1} ms   {asks} asks (920404 in the world), {cleared} books cleared, {trades} trades");
    println!("  wire       {wire_ms:7.1} ms   {} instructions ({INSTRUCTIONS} in the world), {} legs ({LEGS} in the world)", wire.in_period(period).len(), LEGS - legs_left);
    println!("  journal    {journal_ms:7.1} ms   {} events", journal.len());
    println!("  audit      {audit_ms:7.1} ms   {} holdings, 4 families, {found} violations", register.rows());
    println!("  PERIOD     {period_ms:7.1} ms");
    println!();
    println!("TypeScript, measured   {TS_MS:9.1} ms   (42.1 s, median of three)");
    println!("this world             {period_ms:9.1} ms   {:5.0}x", TS_MS / period_ms);
    println!();
    println!("ONE module of forty-seven is in this. The honest reading is this floor plus the rest");
    println!("of the module block, which 0g.40 measured at 10.9x ported — not this figure alone.");
}
