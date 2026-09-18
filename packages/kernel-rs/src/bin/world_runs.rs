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
use phoenix_kernel::params::Kind as ParamKind;
use phoenix_kernel::parties::Representation;
use phoenix_kernel::protocols::{Protocol, Venue};
use phoenix_kernel::registry::{Banks, KindProfile};
use phoenix_kernel::mechanisms::capital_programme::Plant;
use phoenix_kernel::mechanisms::recipe::{Line, Recipe};
use phoenix_kernel::running::{about as running_about, afoot, agreed, tracks, Makes};
use phoenix_kernel::stores::Owing;
use phoenix_kernel::systems::{all, book_of, declare, Wiring};
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
/// 21i: how many places besides the first. A world of one region has nowhere to be more built-up
/// than anywhere else, so nothing a firm decides can turn on where it is.
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

    // ── Parties: all eleven kinds, so no participant and no mechanism has nobody to be ──────────
    let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);

    // ── The registry (21e): what the ids point at ────────────────────────────────────────────────
    // A currency naming its issuer, a country holding the money, a region reading through it, a unit
    // saying what one of it is divided into, and a profile per kind so `admit` asks rather than
    // allowing anything with no bank at all. The ids are the same rows they always were — this says
    // what they MEAN, which is what nothing did before.
    let usd = w.registry.currency(cb);
    let us = w.registry.country(usd);
    // 21i, 40 A1.a: **several places, because one place cannot be more built-up than another.** A
    // world with a single region has nowhere for a firm to prefer, so building there costs what
    // building anywhere costs and location does no work at all.
    let home = w.registry.region(us);
    let places: Vec<RegionId> = (0..PLACES).map(|_| w.registry.region(us)).collect();
    assert_eq!(home, RegionId::at(0), "this world's first region is row 0, as the central bank's is");
    assert_eq!(w.registry.currency_of(home), usd, "Seed B3: the region determines its money");
    // Law 8: two units, because one grid for everything is 21.37's defect. A tonne is milled; a thing
    // counted in whole things is not divided at all.
    let _fine = w.registry.unit(1_000_000.0);
    let _whole = w.registry.unit(1.0);
    for kind in kinds::ALL {
        let p = match kind {
            // 21j.1a: and whether a kind funds a shortfall by BRINGING PAPER. A treasury auctions a
            // bill and a firm brings a bond; a central bank issues the money instead, and a
            // household does neither.
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
            // 21i: somewhere in particular. Everybody in one region is a world where no place can be
            // more built-up than another, so the congestion has nothing to be about.
            places[w.parties.len() % places.len()],
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

    // ── Indices (21.116): a country's, built from named lines, with a COUNT of each ─────────────
    // Indices D1: one equity index per country, and this world has one country. The level is not
    // declared and never is — it is computed from the constituents' own prints when asked.
    let in_it: Vec<(InstrumentId, f64)> = lines.iter().take(8).map(|l| (*l, 100.0)).collect();
    let equity_index = w.registry.index(tracks::EQUITY, us, &in_it);
    assert_eq!(w.registry.indices_in(us), vec![equity_index]);

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
        // Labour A4.b, XI-15: an engagement is for a HEADCOUNT, and a firm does not employ every
        // member of a household cell. Where it employs some of them the cell splits (A4.c), which is
        // this world's one partial event and the only thing that moves a weight. The count is drawn
        // like everything else here — it is a world that is arbitrary and says so (5 E1).
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

    // ── The venues (22c.1) ──────────────────────────────────────────────────────────────────────
    // **A protocol per venue, and they differ.** There was one microstructure — a weekly
    // uniform-price call auction — for bread, labour, loans, shares and freight alike, and a
    // Walrasian auctioneer for bread is the one intermediary that never existed (Law 1).
    //
    // A GOOD is bought in a shop: a seller stands behind an ask and a buyer takes the best it saw.
    // A SHARE trades on an exchange: orders rest and are matched as they arrive. Everything else is
    // a call — an auction or a fixing, which is what a sealed cross at one level IS.
    for line in &lines {
        // **22c2.2: and how long an order stands there**, which is the venue's own convention and
        // the difference between a book and a ratchet. A shop's ask is good for the week and is
        // reposted; an exchange's order is good for about a month; a call's orders do not rest at
        // all, so it declares no life for one.
        let (protocol, rule, stands_for) = match w.instruments.class_of(*line) {
            Class::Good => (Protocol::Posted, PriceRule::SellersCompete, Some(1)),
            Class::Share => (Protocol::Book, PriceRule::BuyersCompete, Some(4)),
            _ => (Protocol::Call, PriceRule::SellersCompete, None),
        };
        // How much of a shop's market a buyer sees. A TECHNOLOGY: search is costly, and a buyer that
        // saw every seller would be a buyer in a call auction wearing a shop's clothes.
        w.open_book(
            book_of(*line),
            *line,
            CurrencyCode::at(0),
            Venue { rule, protocol, seen_by: 5, stands_for },
        );
    }

    // §37 A2: how the goods of this world are made. Arbitrary like everything else here, and with
    // two ways per line so the firm has something to choose between (22.1): one that leans on the
    // input and one that leans on the hours.
    let goods: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Good).take(8).copied().collect();
    let plants: Vec<InstrumentId> =
        lines.iter().filter(|l| w.instruments.class_of(**l) == Class::Plant).take(8).copied().collect();

    // ── 21i: which of these lines are STRUCTURES, and what one of each stands on ─────────────────
    // Commercial, residential and industrial go through one mechanism because the only thing that
    // tells them apart here is a number in the registry (Law 15). A works covers a lot of ground; a
    // dwelling covers a little; a tonne of flour covers none, and that is why it has no row.
    for (n, plant) in plants.iter().enumerate() {
        // Arbitrary like everything else in this world (5 E1): a range of footprints, so a place
        // fills at a rate that depends on what was built there rather than on how many things.
        w.registry.stands_on(*plant, 0.4 + draw.spread(1.6) * (1 + n % 3) as f64);
    }
    // RESIDENTIAL and COMMERCIAL: goods lines that are buildings rather than things. A dwelling is a
    // GOOD in this kernel and a works is PLANT, and both are structures — which is exactly why being
    // a structure is registry data and not a class (40 A1: durable, immovable, indivisible).
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
    // XI-14, 21g: every behaviour-shaping number this world acts on, declared before anything
    // reads one. A participant holds the ID; the value lives here and nowhere else.
    declare(&mut w.params);
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
    // Law 2, 21g.2: of the declared numbers, how many are a CLAIM ABOUT THE ANSWER rather than a
    // primitive. This count must fall, and a run that does not print it is a run in which nobody is
    // looking at it — the same argument as the homeless nouns above.
    // 21j.4: **the third register.** How many wired systems do nothing but publish a count — an
    // honest count of something real, and never the read the system is FOR. It must fall, and a run
    // that does not print it is a run in which nobody is looking at it (21d.1b's argument again).
    let counting: Vec<&'static str> = wired
        .iter()
        .filter(|s| s.participant.is_none() && s.mechanism.as_ref().is_some_and(|m| m.only_counts()))
        .map(|s| s.name)
        .collect();
    println!("         {} of {} wired systems only count:", counting.len(), wired.len());
    println!("           {}", counting.join(" "));

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
        // XI-15, 21h: **the population, as a READ over the cells** — never a number anybody keeps
        // (Small-Business Pools E4: a constant population is the defect). `cells` is how many rows
        // stand for a group and `people` is what they stand for; both move only by the five events,
        // and until item 21h neither had ever moved at all.
        let (mut cells, mut people) = (0usize, 0u64);
        for row in 0..w.parties.len() as u32 {
            let who = PartyId::at(row);
            if w.parties.alive(who) && w.parties.representation_of(who) == Representation::Cell {
                cells += 1;
                people += u64::from(w.parties.weight(who));
            }
        }
        // **Audit A2, E2, 22e.3: WHAT THE AUDIT FOUND, BY FAMILY.** A number nobody reads is not a
        // check, and until this item the audit was not even run — so the assembled world had stepped
        // in every run since the port with no family ever visiting it.
        //
        // An unbuilt family says NOT BUILT and is never counted as clean: a world that assembled no
        // family must not read as a world with no violations.
        let mut audited: Vec<String> = Vec::new();
        for r in &did.audit {
            audited.push(if r.built {
                format!("{} {}", r.family.name(), r.violations.len())
            } else {
                format!("{} not-built", r.family.name())
            });
        }
        audited.sort();

        // **3 C2, 22c.2: how many orders are STANDING.** Nothing rested between sessions at all, and
        // a market with no memory reports thin demand that is an artefact of its protocol rather
        // than a fact about anybody's willingness to buy.
        let standing = (0..w.resting.len() as u32)
            .map(phoenix_kernel::stores::RestingId)
            .filter(|o| w.resting.live(*o))
            .count();
        // 21j.1a: **how many obligations came into existence**, which was zero in every period of
        // every world until the door existed — no firm brought paper, no treasury auctioned a bill it
        // had not got, no pool cut a note. It is counted off the wire rather than assumed (Law 19).
        let brought = w.instruments.len();
        // 21i: **how built-up the places are**, as a read over the register. The SPREAD is what
        // matters: a world where every place carries the same is a world where location decides
        // nothing, so the two ends are printed rather than a total nobody can act on.
        let built = phoenix_kernel::places::built_up(&w.parties, &w.register, &w.registry);
        let emptiest = built.iter().copied().fold(f64::INFINITY, f64::min);
        let fullest = built.iter().copied().fold(0.0f64, f64::max);
        // **XI-9, 22d.1: and what the PAYMENT QUEUE holds.** A payment waiting is not a payment
        // that failed, and a world with no queue turned every gridlock into an arrear on the spot.
        // `taken` is the measure that matters: those are the defaults this world was inventing.
        let (waiting, taken, late) = w.wire.queue.census();
        println!(
            "period {period}  {ms:8.1} ms  — {} phases ran · {} asks · {} books cleared · {} trades · {} made · {} events · {} outlooks · {cells} cells of {people} · built {emptiest:.0}–{fullest:.0} km² · {brought} lines · {standing} resting",
            did.ran, did.asks, did.books_cleared, did.trades, made, did.events, w.outlooks.len(),
        );
        println!(
            "           queue: {waiting} waiting · {taken} went through on a retry · {late} ran out of days ({} of them this period)",
            did.gave_up,
        );
        println!("           audit: {}", audited.join(" · "));
        // Audit A2: a violation names its OWNER, its SIZE, its period and the clause it is about —
        // a finding with no size cannot be ranked and one with no owner cannot be chased, so the
        // count above is never the whole of what is printed.
        for r in &did.audit {
            for v in r.violations.iter().take(3) {
                println!("             [{}] {} {} {} — {}", v.spec, v.owner, v.size, v.unit, v.message);
            }
        }
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
