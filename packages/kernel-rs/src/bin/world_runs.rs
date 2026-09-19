//! THE WHOLE MACHINE, AT THE SIZE IT IS JUDGED ON, RUNNING.

use phoenix_kernel::assembly::{kinds, System, World};
use phoenix_kernel::calendar::Day;
use phoenix_kernel::clearing::PriceRule;
use phoenix_kernel::ledger::{Cause, Leg};
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use phoenix_kernel::instruments::Class;
use phoenix_kernel::params::Kind as ParamKind;
use phoenix_kernel::parties::Representation;
use phoenix_kernel::protocols::{Protocol, Venue};
use phoenix_kernel::registry::{Banks, KindProfile};
use phoenix_kernel::mechanisms::capital_programme::Plant;
use phoenix_kernel::mechanisms::recipe::{Line, Recipe};
use phoenix_kernel::registry::tracks;
use phoenix_kernel::running::Makes;
use phoenix_kernel::stores::{about as running_about, afoot, agreed};
use phoenix_kernel::stores::Owing;
use phoenix_kernel::systems::{all, book_of, declare, Wiring};
use std::time::Instant;

/// The counts the engine is judged on — `world_at_scale`'s, so the two are comparable.
const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const BOOKS: u32 = 1_546;
/// The judged holdings count.
const HOLDINGS: usize = 544_104;
/// One calendar: a 7-day period (Calendar A1).
const WEEK: i64 = 7;
/// How many periods to run.
const PERIODS: u32 = 4;
/// How many places besides the first.
const PLACES: u32 = 6;

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

    let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);

    let usd = w.registry.currency(cb);
    let us = w.registry.country(usd);
    // 21i, 40 A1.a: several places, because one place cannot be more built-up than another.
    let home = w.registry.region(us);
    let places: Vec<RegionId> = (0..PLACES).map(|_| w.registry.region(us)).collect();
    assert_eq!(home, RegionId::at(0), "this world's first region is row 0, as the central bank's is");
    assert_eq!(w.registry.currency_of(home), usd, "Seed B3: the region determines its money");
    // Two units, because one grid for everything is 21.37's defect.
    let _fine = w.registry.unit(1_000_000.0);
    let _whole = w.registry.unit(1.0);
    for kind in kinds::ALL {
        let p = match kind {
            // And whether a kind funds a shortfall by BRINGING PAPER.
            kinds::CENTRAL_BANK => {
                KindProfile { issues_money: true, banks: Banks::Nowhere, issues_paper: false }
            }
            kinds::BANK => {
                KindProfile { issues_money: true, banks: Banks::AtTheCentralBank, issues_paper: true }
            }
            kinds::TREASURY => {
                KindProfile { issues_money: false, banks: Banks::AtTheCentralBank, issues_paper: true }
            }
            kinds::FIRM => {
                KindProfile { issues_money: false, banks: Banks::AtACommercialBank, issues_paper: true }
            }
            _ => KindProfile { issues_money: false, banks: Banks::AtACommercialBank, issues_paper: false },
        };
        w.registry.profile_for(kind, p);
    }
    // The central bank's money exists before anybody banks at it.
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

    // Everybody else banks somewhere, because a party with no account holds no money.
    let rest = [
        kinds::FIRM, kinds::HOUSEHOLD, kinds::FUND, kinds::INSURER, kinds::DEALER,
        kinds::CARRIER, kinds::SMALL_FIRM, kinds::ASSESSOR, kinds::STOCKIST,
    ];
    let mut everyone: Vec<PartyId> = vec![cb, treasury];
    everyone.extend(banks.iter().copied());
    while (w.parties.len() as u32) < PARTIES {
        let kind = rest[(w.parties.len()) % rest.len()];
        let at = w.parties.len() % banks.len();
        let cell = kind == kinds::HOUSEHOLD;
        let who = w.admit(
            kind,
            // Somewhere in particular.
            places[w.parties.len() % places.len()],
            banks[at],
            if cell { Representation::Cell } else { Representation::Named },
            if cell { 200 + draw.below(1_800) as u32 } else { 1 },
            0,
        );
        everyone.push(who);
    }

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

    // ── Indices: a country's, built from named lines, with a COUNT of each ───────────── One equity
    // index per country, and this world has one country.
    let in_it: Vec<(InstrumentId, f64)> = lines.iter().take(8).map(|l| (*l, 100.0)).collect();
    let equity_index = w.registry.index(tracks::EQUITY, us, &in_it);
    assert_eq!(w.registry.indices_in(us), vec![equity_index]);

    for (n, who) in everyone.iter().enumerate() {
        let at = n % deposits.len();
        w.register.money_delta(*who, deposits[at], draw.spread(10_000.0));
    }
    for b in &banks {
        w.register.money_delta(*b, reserves, draw.spread(200_000.0));
    }
    // The holdings the judged period walks.
    while w.register.rows() < HOLDINGS {
        let holder = everyone[draw.below(everyone.len() as u64) as usize];
        let line = InstrumentId::at(1 + draw.below(INSTRUMENTS as u64 - 1) as u32);
        if w.instruments.class_of(line) == Class::Money {
            continue;
        }
        w.register.credit(holder, line, draw.spread(500.0), 1.0, 0);
    }

    // Every claim owes something on a day, so there is something to fall behind on.
    for (line, issuer) in &claims {
        let holder = banks[draw.below(banks.len() as u64) as usize];
        w.register.credit(holder, *line, draw.spread(1_000.0), 1.0, 0);
        for k in 0..PERIODS as i64 {
            w.schedules.owes(*line, *issuer, Day(k * WEEK + draw.below(WEEK as u64) as i64), draw.spread(20.0), Owing::Interest);
        }
    }

    let firms: Vec<u32> = w.parties.of_kind(kinds::FIRM).to_vec();
    let cells: Vec<u32> = w.parties.of_kind(kinds::HOUSEHOLD).to_vec();
    for (n, c) in cells.iter().enumerate() {
        let employer = PartyId(firms[n % firms.len()]);
        // An engagement is for a HEADCOUNT, and a firm does not employ every member of a household
        // cell.
        let of_them = w.parties.weight(PartyId(*c));
        let heads = 1 + draw.below(u64::from(of_them)) as u32;
        w.agreements.strike(
            agreed::ENGAGEMENT,
            employer,
            PartyId(*c),
            &[draw.spread(40.0), draw.spread(35.0), f64::from(heads)],
            Day(-365),
            None,
        );
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

    // ── The venues ────────────────────────────────────────────────────────────────────── A
    // protocol per venue, and they differ.
    for line in &lines {
        // And how long an order stands there, which is the venue's own convention and the difference
        // between a book and a ratchet.
        let (protocol, rule, stands_for) = match w.instruments.class_of(*line) {
            Class::Good => (Protocol::Posted, PriceRule::SellersCompete, Some(1)),
            Class::Share => (Protocol::Book, PriceRule::BuyersCompete, Some(4)),
            _ => (Protocol::Call, PriceRule::SellersCompete, None),
        };
        // How much of a shop's market a buyer sees.
        w.open_book(
            book_of(*line),
            *line,
            CurrencyCode::at(0),
            Venue { rule, protocol, seen_by: 5, stands_for },
        );
    }

    // And THE SEED DECLARES what the rest of its lines are.
    let traded: std::collections::HashSet<u32> = lines.iter().map(|l| l.0).collect();
    for row in 0..w.instruments.len() as u32 {
        let line = InstrumentId(row);
        if !traded.contains(&row) && w.instruments.class_of(line) != Class::Money {
            w.instruments.carried_at_cost(line);
        }
    }

    // How the goods of this world are made.
    let goods: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Good).take(8).copied().collect();
    let plants: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Plant).take(8).copied().collect();

    for (n, plant) in plants.iter().enumerate() {
        // Arbitrary like everything else in this world: a range of footprints, so a place
        // fills at a rate that depends on what was built there rather than on how many things.
        w.registry.stands_on(*plant, 0.4 + draw.spread(1.6) * (1 + n % 3) as f64);
    }
    // RESIDENTIAL and COMMERCIAL: goods lines that are buildings rather than things.
    let buildings: Vec<InstrumentId> = goods.iter().rev().take(3).copied().collect();
    for (n, b) in buildings.iter().enumerate() {
        w.registry.stands_on(*b, 0.02 + 0.03 * n as f64);
    }

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

    // A maker is whoever holds the plant, so a world where the plant landed on parties
    // that employ nobody is a world that makes nothing.
    for (n, m) in makes.iter().enumerate() {
        let maker = PartyId(firms[n % firms.len()]);
        w.register.credit(maker, m.plant, 3.0, 1_000.0, 0);
        for (input, _) in &m.line.ways[0].per_unit {
            w.register.credit(maker, *input, draw.spread(4_000.0), 0.5, 0);
        }
        // And a view of its own demand, which in a seeded world is what its own past sales gave it.
        w.outlooks.form(maker, running_about::HOW_MUCH_IT_SELLS, draw.spread(300.0), 0);
    }

    let wiring = Wiring {
        makes,
        lines: lines.iter().take(8).copied().collect(),
        overnight: None,
        paper: None,
        days_per_period: WEEK,
    };
    // Every behaviour-shaping number this world acts on, declared before anything reads one.
    declare(&mut w.params);
    let wired = all(&wiring, &mut w.journal);
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
    // The honest measure of how much ontology is missing.
    let homeless = w.nouns.homeless();
    println!("         {} nouns with no kernel home:", homeless.len());
    for (name, item) in &homeless {
        println!("           {name} (item {item})");
    }
    // Of the declared numbers, how many are a CLAIM ABOUT THE ANSWER rather than a primitive.
    let shapes = w.params.shapes();
    println!("         {} of {} declared numbers are shapes:", shapes.len(), w.params.len());
    for (id, kind) in &shapes {
        match kind {
            ParamKind::Placeholder { mechanism, item } => {
                println!("           {id} — stands in for {mechanism} (item {item})")
            }
            _ => println!("           {id}"),
        }
    }

    let mut worst = 0.0f64;
    // What each period's mechanisms decided, kept so the census below can ask over the run.
    let mut every_period_decided: Vec<Vec<u32>> = Vec::new();
    for period in 1..=PERIODS {
        let began = Instant::now();
        let did = w.step(&systems);
        every_period_decided.push(did.decided.clone());
        let ms = began.elapsed().as_secs_f64() * 1_000.0;
        if ms > worst {
            worst = ms;
        }
        // How many production runs the world actually made.
        let made = w
            .wire
            .in_period(period)
            .filter(|n| {
                w.wire.cause_of(*n) == Cause::Production
                    && w.wire.legs_of(*n).iter().any(|l| matches!(l, Leg::Create { .. }))
            })
            .count();
        // The population, as a READ over the cells — never a number anybody keeps (Small-Business
        // Pools E4: a constant population is the defect).
        let (mut cells, mut people) = (0usize, 0u64);
        for row in 0..w.parties.len() as u32 {
            let who = PartyId::at(row);
            if w.parties.alive(who) && w.parties.representation_of(who) == Representation::Cell {
                cells += 1;
                people += u64::from(w.parties.weight(who));
            }
        }
        // WHAT THE AUDIT FOUND, BY FAMILY.
        let mut audited: Vec<String> = Vec::new();
        for r in &did.audit {
            audited.push(if r.built {
                format!("{} {}", r.family.name(), r.violations.len())
            } else {
                // And what it is waiting for, which the contributor slot carries for an unbuilt
                // family.
                format!("{} not-built ({})", r.family.name(), r.contributors.join(", "))
            });
        }
        audited.sort();

        // 3 C2, 22c.2: how many orders are STANDING.
        let standing = (0..w.resting.len() as u32)
            .map(phoenix_kernel::stores::RestingId)
            .filter(|o| w.resting.live(*o))
            .count();
        let brought = w.instruments.len();
        // How built-up the places are, as a read over the register.
        let built = phoenix_kernel::places::built_up(&w.parties, &w.register, &w.registry);
        let emptiest = built.iter().copied().fold(f64::INFINITY, f64::min);
        let fullest = built.iter().copied().fold(0.0f64, f64::max);
        // What became of THIS PERIOD's short payments.
        let (waiting, taken, late) = did.queue;
        println!(
            "period {period}  {ms:8.1} ms  — {} phases ran · {} asks · {} books cleared · {} trades · {} made · {} events · {} outlooks · {cells} cells of {people} · built {emptiest:.0}–{fullest:.0} km² · {brought} lines · {standing} resting",
            did.ran, did.asks, did.books_cleared, did.trades, made, did.events, w.outlooks.len(),
        );
        println!(
            "           {} in flight closed · queue, of this period's short payments: {waiting} still waiting · {taken} went through after waiting ({} of them in a ring) · {late} ran out of days",
            did.closed, did.unwound,
        );
        println!("           audit: {}", audited.join(" · "));
        for r in &did.audit {
            for v in r.violations.iter().take(3) {
                println!("             [{}] {} {} {} — {}", v.spec, v.owner, v.size, v.unit, v.message);
            }
        }
    }

    // THE THIRD REGISTER, AND IT IS A READ OF THE RUN.
    let mut decided: std::collections::HashSet<u32> = std::collections::HashSet::new();
    for slots in &every_period_decided {
        decided.extend(slots.iter().copied());
    }
    let counting: Vec<&'static str> = wired
        .iter()
        .filter(|s| s.mechanism.is_some() && !decided.contains(&s.slot))
        .map(|s| s.name)
        .collect();
    println!();
    println!(
        "         {} of {} wired systems only counted, over {} periods:",
        counting.len(),
        wired.iter().filter(|s| s.mechanism.is_some()).count(),
        PERIODS,
    );
    println!("           {}", counting.join(" "));

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
