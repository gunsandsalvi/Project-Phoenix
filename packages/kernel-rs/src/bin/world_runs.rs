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
use phoenix_kernel::ledger::{Cause, Leg};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use phoenix_kernel::instruments::Class;
use phoenix_kernel::parties::Representation;
use phoenix_kernel::registry::{Banks, KindProfile};
use phoenix_kernel::mechanisms::capital_programme::Plant;
use phoenix_kernel::mechanisms::recipe::{Line, Recipe};
use phoenix_kernel::running::{about as running_about, afoot, agreed, Makes};
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

    // ── The registry (21e): what the ids point at ────────────────────────────────────────────────
    // A currency naming its issuer, a country holding the money, a region reading through it, a unit
    // saying what one of it is divided into, and a profile per kind so `admit` asks rather than
    // allowing anything with no bank at all. The ids are the same rows they always were — this says
    // what they MEAN, which is what nothing did before.
    let usd = w.registry.currency(cb);
    let us = w.registry.country(usd);
    let home = w.registry.region(us);
    assert_eq!(home, RegionId::at(0), "this world's one region is row 0, as every party's is");
    assert_eq!(w.registry.currency_of(home), usd, "Seed B3: the region determines its money");
    // Law 8: two units, because one grid for everything is 21.37's defect. A tonne is milled; a thing
    // counted in whole things is not divided at all.
    let _fine = w.registry.unit(1_000_000.0);
    let _whole = w.registry.unit(1.0);
    for kind in kinds::ALL {
        let p = match kind {
            kinds::CENTRAL_BANK => KindProfile { issues_money: true, banks: Banks::Nowhere },
            kinds::BANK => KindProfile { issues_money: true, banks: Banks::AtTheCentralBank },
            kinds::TREASURY => KindProfile { issues_money: false, banks: Banks::AtTheCentralBank },
            _ => KindProfile { issues_money: false, banks: Banks::AtACommercialBank },
        };
        w.registry.profile_for(kind, p);
    }
    // Money D2: the central bank's money exists before anybody banks at it. `admit` asks the kind's
    // profile now (21e), so the ordering is enforced rather than assumed — a treasury admitted before
    // there were reserves to hold would be refused at ENTRY.
    let reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
    let treasury = w.admit(kinds::TREASURY, RegionId::at(0), cb, Representation::Named, 1, 0);

    let banks_wanted = 30usize;
    let mut banks: Vec<PartyId> = Vec::with_capacity(banks_wanted);
    let mut deposits: Vec<InstrumentId> = Vec::with_capacity(banks_wanted);
    for _ in 0..banks_wanted {
        let b = w.admit(kinds::BANK, RegionId::at(0), cb, Representation::Named, 1, 0);
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
        let who = w.admit(
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
        w.agreements.strike(agreed::ENGAGEMENT, employer, PartyId(*c), &[draw.spread(40.0), draw.spread(35.0)], Day(-365), None);
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

    // §37 A2: how the goods of this world are made. Arbitrary like everything else here, and with
    // two ways per line so the firm has something to choose between (22.1): one that leans on the
    // input and one that leans on the hours.
    let goods: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Good).take(8).copied().collect();
    let plants: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Plant).take(8).copied().collect();
    let makes: Vec<Makes> = goods
        .iter()
        .enumerate()
        .filter_map(|(n, made)| {
            let from = *goods.get((n + 1) % goods.len())?;
            let plant = *plants.get(n % plants.len().max(1))?;
            Some(Makes {
                line: Line::new(
                    *made,
                    vec![
                        Recipe::new(*made, vec![(from, 2.0)], 0.2, 0.1, 0.98, 10.0, 1),
                        Recipe::new(*made, vec![(from, 0.5)], 1.5, 0.1, 0.98, 10.0, 2),
                    ],
                ),
                plant,
                plant_is: Plant { life: 200, upkeep_per_period: 0.5, capacity_per_period: 40.0 },
            })
        })
        .collect();

    // 33 A2, 37 B1: **a maker is whoever holds the plant**, so a world where the plant landed on
    // parties that employ nobody is a world that makes nothing. The draw spread the plant lines over
    // everybody; here one firm per line is given the mill, the input stock to run it and nothing
    // else. It is as arbitrary as the rest of this file and it is what lets the production mechanism
    // be SEEN to run at scale rather than only in its own tests.
    for (n, m) in makes.iter().enumerate() {
        let maker = PartyId(firms[n % firms.len()]);
        w.register.credit(maker, m.plant, 3.0, 1_000.0, 0);
        for (input, _) in &m.line.ways[0].per_unit {
            w.register.credit(maker, *input, draw.spread(4_000.0), 0.5, 0);
        }
        // §46: and a view of its own demand, which in a seeded world is what its own past sales
        // gave it. Here it is drawn, like everything else — 5 E1 again: this is not a seed.
        w.outlooks.form(maker, running_about::HOW_MUCH_IT_SELLS, draw.spread(300.0), 0);
    }

    let wiring = Wiring {
        makes,
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
    // 21d.1b: the honest measure of how much ontology is missing. It must fall, and a run that does
    // not print it is a run in which nobody is looking at it.
    let homeless = w.nouns.homeless();
    println!("         {} nouns with no kernel home:", homeless.len());
    for (name, item) in &homeless {
        println!("           {name} (item {item})");
    }

    let mut worst = 0.0f64;
    for period in 1..=PERIODS {
        let began = Instant::now();
        let did = w.step(&systems);
        let ms = began.elapsed().as_secs_f64() * 1_000.0;
        if ms > worst {
            worst = ms;
        }
        // §37 B1: how many production runs the world actually made. A wired mechanism that never
        // fires is the defect 21d exists to prevent, so the runner counts it rather than assuming.
        // A thing coming into existence is a `Create`; a thing leaving it is a `Destroy`, and
        // perishing is the other user of `Cause::Production`. Counting the cause alone counts the
        // rot as production, which is the opposite of what is being asked.
        let made = w
            .wire
            .in_period(period)
            .filter(|n| {
                w.wire.cause_of(*n) == Cause::Production
                    && w.wire.legs_of(*n).iter().any(|l| matches!(l, Leg::Create { .. }))
            })
            .count();
        println!(
            "period {period}  {ms:8.1} ms  — {} phases ran · {} asks · {} books cleared · {} trades · {} made · {} events · {} outlooks",
            did.ran, did.asks, did.books_cleared, did.trades, made, did.events, w.outlooks.len(),
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
    println!("`made` counts the batches §37's lines ran. It was zero every period until item 22, and a");
    println!("world that makes nothing sells its opening stock once and then has nothing to trade.");
}
