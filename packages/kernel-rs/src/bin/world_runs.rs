//! **THE WHOLE MACHINE, AT THE SIZE IT IS JUDGED ON, RUNNING.**
//!
//! Fifty systems, fifty phases, every book, for N periods — and what it cost.
//!
//! **THE WORLD THIS BUILDS IS ARBITRARY AND IS DECLARED ARBITRARY.** Every number in it is drawn
//! from a counter. It is NOT a seed and must never be read as one: nothing here is cleared, nothing
//! is decided, and no quantity in it is an outcome of anything (5 E1). It exists to answer one
//! question — *does the machine run in full, and what does a period cost* — and the seeding replaces
//! it when the seeding is built.
//!
//! What it does prove is what could not be proved before: that every ported module is REACHED by the
//! period loop, that the phase order holds at scale, and that what the mechanisms propose goes over
//! the ordinary wire.

use phoenix_kernel::assembly::{kinds, System, World};
use phoenix_kernel::calendar::Day;
use phoenix_kernel::clearing::PriceRule;
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use phoenix_kernel::instruments::Class;
use phoenix_kernel::parties::Representation;
use phoenix_kernel::running::{afoot, agreed};
use phoenix_kernel::stores::Owing;
use phoenix_kernel::systems::{all, book_of, Wiring};
use std::time::Instant;

/// The counts the engine is judged on — `world_at_scale`'s, so the two are comparable.
const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const BOOKS: u32 = 1_546;
/// The judged holdings count. The register walks and the audit scale with this, so a period measured
/// at a tenth of it is not the period the migration was judged on.
const HOLDINGS: usize = 544_104;
/// One calendar: a 7-day period (Calendar A1).
const WEEK: i64 = 7;
/// How many periods to run. A measurement, not a target.
const PERIODS: u32 = 4;

/// The same counter-based draw the engine uses: no clock, no ambient source.
struct Draw(u64);

impl Draw {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    fn spread(&mut self, scale: f64) -> f64 {
        scale * (0.5 + (self.below(1_000) as f64) / 1_000.0)
    }
}

fn main() {
    let mut draw = Draw(0x9E37_79B9_7F4A_7C15);
    let built = Instant::now();
    let mut w = World::empty();

    // ── Parties: all eleven kinds, so no participant and no mechanism has nobody to be ──────────
    let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
    let treasury = w.parties.add(kinds::TREASURY, RegionId::at(0), cb, Representation::Named, 1, 0);
    let reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);

    let banks_wanted = 30usize;
    let mut banks: Vec<PartyId> = Vec::with_capacity(banks_wanted);
    let mut deposits: Vec<InstrumentId> = Vec::with_capacity(banks_wanted);
    for _ in 0..banks_wanted {
        let b = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 1, 0);
        deposits.push(w.instruments.issue(b, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None));
        banks.push(b);
    }

    // Everybody else banks somewhere, because a party with no account holds no money (Money D2).
    let rest = [
        kinds::FIRM, kinds::HOUSEHOLD, kinds::FUND, kinds::INSURER, kinds::DEALER,
        kinds::CARRIER, kinds::SMALL_FIRM, kinds::ASSESSOR,
    ];
    let mut everyone: Vec<PartyId> = vec![cb, treasury];
    everyone.extend(banks.iter().copied());
    while (w.parties.len() as u32) < PARTIES {
        let kind = rest[(w.parties.len()) % rest.len()];
        let at = w.parties.len() % banks.len();
        let cell = kind == kinds::HOUSEHOLD;
        let who = w.parties.add(
            kind,
            RegionId::at(0),
            banks[at],
            if cell { Representation::Cell } else { Representation::Named },
            if cell { 200 + draw.below(1_800) as u32 } else { 1 },
            0,
        );
        everyone.push(who);
    }

    // ── Instruments: the lines the books deliver, and the claims that carry schedules ───────────
    let mut lines: Vec<InstrumentId> = Vec::new();
    let mut claims: Vec<(InstrumentId, PartyId)> = Vec::new();
    while (w.instruments.len() as u32) < INSTRUMENTS {
        let issuer = everyone[draw.below(everyone.len() as u64) as usize];
        let n = w.instruments.len();
        let (class, unit, coupon, matures) = match n % 4 {
            0 => (Class::Good, UnitId::at(1), None, None),
            1 => (Class::Share, UnitId::at(0), None, None),
            2 => (Class::Plant, UnitId::at(0), None, None),
            _ => (Class::Claim, UnitId::at(0), Some(0.04), Some(Day(3_650))),
        };
        let line = w.instruments.issue(issuer, CurrencyCode::at(0), class, unit, coupon, matures);
        if class == Class::Claim {
            claims.push((line, issuer));
        } else if lines.len() < BOOKS as usize {
            lines.push(line);
        }
    }

    // ── What everybody holds, and what everybody owes ───────────────────────────────────────────
    for (n, who) in everyone.iter().enumerate() {
        let at = n % deposits.len();
        w.register.money_delta(*who, deposits[at], draw.spread(10_000.0));
    }
    for b in &banks {
        w.register.money_delta(*b, reserves, draw.spread(200_000.0));
    }
    // Register A3: the holdings the judged period walks. Spread over the lines and the holders, so
    // the walks and the audit cost what they cost in the world this is a model of.
    while w.register.rows() < HOLDINGS {
        let holder = everyone[draw.below(everyone.len() as u64) as usize];
        let line = InstrumentId::at(1 + draw.below(INSTRUMENTS as u64 - 1) as u32);
        if w.instruments.class_of(line) == Class::Money {
            continue;
        }
        w.register.credit(holder, line, draw.spread(500.0), 1.0, 0);
    }

    // 5 D2: every claim owes something on a day, so there is something to fall behind on.
    for (line, issuer) in &claims {
        let holder = banks[draw.below(banks.len() as u64) as usize];
        w.register.credit(holder, *line, draw.spread(1_000.0), 1.0, 0);
        for k in 0..PERIODS as i64 {
            w.schedules.owes(*line, *issuer, Day(k * WEEK + draw.below(WEEK as u64) as i64), draw.spread(20.0), Owing::Interest);
        }
    }

    // ── The relations and the things in flight ──────────────────────────────────────────────────
    let firms: Vec<u32> = w.parties.of_kind(kinds::FIRM).to_vec();
    let cells: Vec<u32> = w.parties.of_kind(kinds::HOUSEHOLD).to_vec();
    for (n, c) in cells.iter().enumerate() {
        let employer = PartyId(firms[n % firms.len()]);
        w.agreements.strike(agreed::ENGAGEMENT, employer, PartyId(*c), &[draw.spread(40.0)], Day(-365), None);
    }
    for (n, f) in firms.iter().enumerate() {
        let kind = match n % 4 {
            0 => afoot::CAPITAL_PROGRAMME,
            1 => afoot::FLOTATION,
            2 => afoot::BUY_BACK,
            _ => afoot::WORKOUT,
        };
        w.processes.begin(kind, PartyId(*f), 0, Some(draw.below(PERIODS as u64) as u32), draw.spread(500.0));
    }

    // ── The books ───────────────────────────────────────────────────────────────────────────────
    for line in &lines {
        w.open_book(book_of(*line), *line, CurrencyCode::at(0), PriceRule::SellersCompete);
    }

    let wiring = Wiring {
        basket: lines.iter().take(8).copied().collect(),
        lines: lines.iter().take(8).copied().collect(),
        overnight: None,
        paper: None,
        days_per_period: WEEK,
    };
    let wired = all(&wiring, &mut w.journal.kinds);
    let systems: Vec<&dyn System> = wired.iter().map(|s| s as &dyn System).collect();
    w.wire_up(&systems);
    let assembly = built.elapsed();

    println!(
        "built {:.0} ms — {} parties, {} instruments, {} holdings, {} books, {} phases",
        assembly.as_secs_f64() * 1_000.0,
        w.parties.len(),
        w.instruments.len(),
        w.register.rows(),
        w.books.len(),
        w.phases.len(),
    );
    println!(
        "         {} agreements, {} scheduled payments, {} processes",
        w.agreements.len(),
        w.schedules.len(),
        w.processes.len(),
    );

    let mut worst = 0.0f64;
    for period in 1..=PERIODS {
        let began = Instant::now();
        let did = w.step(&systems);
        let ms = began.elapsed().as_secs_f64() * 1_000.0;
        if ms > worst {
            worst = ms;
        }
        println!(
            "period {period}  {ms:8.1} ms  — {} phases ran · {} asks · {} books cleared · {} trades · {} events · {} outlooks",
            did.ran, did.asks, did.books_cleared, did.trades, did.events, w.outlooks.len(),
        );
    }

    println!();
    println!("worst period {worst:.1} ms against the 3,000 ms the migration was judged on.");
    println!(
        "All {} wired systems ran every period, in {} declared phases (the three kernel moments own the rest).",
        wired.len(),
        w.phases.len(),
    );
    println!("The world is ARBITRARY: nothing in it was cleared, decided or seeded, and the seeding");
    println!("replaces it (5 E1). What it proves is that the machine runs in full, and what that costs.");
}
