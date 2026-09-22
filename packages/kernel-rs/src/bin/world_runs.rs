//! THE WHOLE MACHINE, AT THE SIZE IT IS JUDGED ON, RUNNING.

use phoenix_kernel::assembly::{kinds, RunConfig, System, World};
use phoenix_kernel::calendar::Week;
use phoenix_kernel::clearing::PriceRule;
use phoenix_kernel::ids::book_of;
use phoenix_kernel::ids::{CurrencyCode, InstrumentId, PartyId, RegionId, UnitId};
use phoenix_kernel::instruments::Class;
use phoenix_kernel::ledger::{Cause, Leg};
use phoenix_kernel::params::Kind as ParamKind;
use phoenix_kernel::parties::Representation;
use phoenix_kernel::protocols::{Protocol, Venue};
use phoenix_kernel::registry::{
    Banks, Capitalisation, CreditQuality, Footprint, IndexScope, IndexSubject, KindProfile,
};
use phoenix_kernel::registry::{Plant, Way, Weighting};
use phoenix_kernel::stores::Owing;
use phoenix_kernel::stores::{about as running_about, afoot, agreed};
use phoenix_kernel::systems::{all, declare, Wiring};
use std::time::Instant;

/// The counts the engine is judged on — `world_at_scale`'s, so the two are comparable.
const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const BOOKS: u32 = 1_546;
/// The judged holdings count.
const HOLDINGS: usize = 544_104;
/// One calendar: a 7-day week (Calendar A1).
const WEEK: i64 = 7;
/// How many weeks to run.
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

/// XI-6: a seeded position is not one anybody acquired, so the seed says what it is held FOR. It
/// says what the holder's own kind would say, because the first fill it wins restates it — and a
/// line with no book has no price to mark to, whoever holds it.
fn seeded_as(
    w: &mut World,
    traded: &std::collections::HashSet<u32>,
    holder: PartyId,
    line: InstrumentId,
) {
    let marks = matches!(w.parties.kind_of(holder), kinds::DEALER | kinds::FUND)
        && traded.contains(&line.0);
    let as_ = if marks {
        phoenix_kernel::register::Carrying::Market
    } else {
        phoenix_kernel::register::Carrying::Cost
    };
    w.register.carry(holder, line, as_);
}

fn main() {
    let config = RunConfig::default();
    let mut draw = Draw(config.seed);
    let built = Instant::now();
    let mut w = World::with_parameters(config, declare);

    let cb = w.parties.add(
        kinds::CENTRAL_BANK,
        RegionId::at(0),
        PartyId::NONE,
        Representation::Named,
        0,
    );

    let usd = w.registry.currency(cb);
    let us = w.admit_country(usd);
    // 21i, 40 A1.a: several places, because one place cannot be more built-up than another.
    let home = w.admit_region(us);
    let places: Vec<RegionId> = (0..PLACES).map(|_| w.admit_region(us)).collect();
    assert_eq!(
        home,
        RegionId::at(0),
        "this world's first region is row 0, as the central bank's is"
    );
    assert_eq!(
        w.geography
            .country_of(home)
            .map(|c| w.registry.currency_of_country(c)),
        Some(usd),
        "Seed B3: the ground a region is on says whose money it is in"
    );
    // Two units, because one grid for everything is 21.37's defect.
    let _fine = w
        .registry
        .unit(std::num::NonZeroU32::new(1_000_000).unwrap());
    let _whole = w.registry.unit(std::num::NonZeroU32::new(1).unwrap());
    for kind in kinds::ALL {
        let failure = match kind {
            kinds::CENTRAL_BANK => phoenix_kernel::registry::FailureMode::Never,
            kinds::HOUSEHOLD => phoenix_kernel::registry::FailureMode::Household,
            kinds::BANK => phoenix_kernel::registry::FailureMode::Bank,
            kinds::TREASURY => phoenix_kernel::registry::FailureMode::Sovereign,
            kinds::FUND | kinds::INSURER => phoenix_kernel::registry::FailureMode::BalanceSheet,
            kinds::FIRM | kinds::SMALL_FIRM | kinds::CARRIER | kinds::DEALER | kinds::STOCKIST => {
                phoenix_kernel::registry::FailureMode::Operating
            }
            _ => phoenix_kernel::registry::FailureMode::Never,
        };
        let p = match kind {
            // And whether a kind funds a shortfall by BRINGING PAPER.
            kinds::CENTRAL_BANK => KindProfile {
                issues_money: true,
                banks: Banks::Nowhere,
                issues_paper: false,
                occupies_a_dwelling: false,
                failure,
            },
            kinds::BANK => KindProfile {
                issues_money: true,
                banks: Banks::AtTheCentralBank,
                issues_paper: true,
                occupies_a_dwelling: false,
                failure,
            },
            kinds::TREASURY => KindProfile {
                issues_money: false,
                banks: Banks::AtTheCentralBank,
                issues_paper: true,
                occupies_a_dwelling: false,
                failure,
            },
            kinds::FIRM => KindProfile {
                issues_money: false,
                banks: Banks::AtACommercialBank,
                issues_paper: true,
                occupies_a_dwelling: false,
                failure,
            },
            // A household is the only thing in this world that lives in a house.
            kinds::HOUSEHOLD => KindProfile {
                issues_money: false,
                banks: Banks::AtACommercialBank,
                issues_paper: false,
                occupies_a_dwelling: true,
                failure,
            },
            _ => KindProfile {
                issues_money: false,
                banks: Banks::AtACommercialBank,
                issues_paper: false,
                occupies_a_dwelling: false,
                failure,
            },
        };
        w.registry.profile_for(kind, p);
    }
    // The central bank's money exists before anybody banks at it.
    let reserves = w.instruments.issue(
        cb,
        CurrencyCode::at(0),
        Class::Money,
        UnitId::at(0),
        None,
        None,
    );
    let treasury = w.admit(
        kinds::TREASURY,
        RegionId::at(0),
        cb,
        Representation::Named,
        0,
    );

    let banks_wanted = 30usize;
    let mut banks: Vec<PartyId> = Vec::with_capacity(banks_wanted);
    let mut deposits: Vec<InstrumentId> = Vec::with_capacity(banks_wanted);
    for _ in 0..banks_wanted {
        let b = w.admit(kinds::BANK, RegionId::at(0), cb, Representation::Named, 0);
        deposits.push(w.instruments.issue(
            b,
            CurrencyCode::at(0),
            Class::Money,
            UnitId::at(0),
            None,
            None,
        ));
        banks.push(b);
    }

    // Everybody else banks somewhere, because a party with no account holds no money.
    let rest = [
        kinds::FIRM,
        kinds::HOUSEHOLD,
        kinds::FUND,
        kinds::INSURER,
        kinds::DEALER,
        kinds::CARRIER,
        kinds::SMALL_FIRM,
        kinds::ASSESSOR,
        kinds::STOCKIST,
    ];
    let mut everyone: Vec<PartyId> = vec![cb, treasury];
    everyone.extend(banks.iter().copied());
    while (w.parties.len() as u32) < PARTIES {
        let kind = rest[(w.parties.len()) % rest.len()];
        let at = w.parties.len() % banks.len();
        let cell = kind == kinds::HOUSEHOLD || kind == kinds::SMALL_FIRM;
        let key = if kind == kinds::HOUSEHOLD {
            phoenix_kernel::parties::LatticeKey::Household(phoenix_kernel::parties::HouseholdKey {
                age: draw.below(12) as u32,
                composition: draw.below(6) as u32,
                employment: draw.below(5) as u32,
                unemployed_since: 0,
                income: draw.below(10) as u32,
                tenure: draw.below(4) as u32,
                liquid_wealth: draw.below(10) as u32,
                debt_service: draw.below(6) as u32,
            })
        } else if kind == kinds::SMALL_FIRM {
            phoenix_kernel::parties::LatticeKey::SmallBusiness(
                phoenix_kernel::parties::SmallBusinessKey {
                    sector: draw.below(12) as u32,
                    age: draw.below(6) as u32,
                    size: draw.below(5) as u32,
                    productivity: draw.below(10) as u32,
                    leverage: draw.below(6) as u32,
                    coverage: draw.below(6) as u32,
                    credit_access: draw.below(4) as u32,
                },
            )
        } else {
            phoenix_kernel::parties::LatticeKey::Named(0)
        };
        let who = w.admit(
            kind,
            // Somewhere in particular.
            places[w.parties.len() % places.len()],
            banks[at],
            if cell {
                Representation::Cell(
                    std::num::NonZeroU32::new(200 + draw.below(1_800) as u32).unwrap(),
                )
            } else {
                Representation::Named
            },
            key,
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
            _ => (Class::Claim, UnitId::at(0), Some(0.04), Some(Week(3_650))),
        };
        let line = w
            .instruments
            .issue(issuer, CurrencyCode::at(0), class, unit, coupon, matures);
        if class == Class::Claim {
            claims.push((line, issuer));
        } else if lines.len() < BOOKS as usize {
            lines.push(line);
        }
    }

    // ── Indices: a country's and the world's, each a RULE over what qualifies ───────────
    let currency = IndexScope::Currency(w.registry.currency_of_country(us));
    // 22 A4: every index in this world is based on the week it opens in.
    let base = Week(0);
    // 22 A1, B2: each index states its subject, its scope and where its weights come from. What is
    // in it is whatever qualifies that week, so nothing here is a list of lines.
    let mut indices = Vec::new();
    for scope in [currency, IndexScope::Global] {
        for band in [
            Capitalisation::All,
            Capitalisation::Small,
            Capitalisation::Large,
        ] {
            indices.push(w.registry.index(
                IndexSubject::Equity(band),
                scope,
                base,
                Weighting::Capitalisation,
            ));
        }
    }
    for subject in [
        IndexSubject::FixedBond(CreditQuality::InvestmentGrade),
        IndexSubject::FixedBond(CreditQuality::HighYield),
        IndexSubject::Cds(CreditQuality::InvestmentGrade),
        IndexSubject::Cds(CreditQuality::HighYield),
        IndexSubject::TradableTermLoan(CreditQuality::HighYield),
    ] {
        indices.push(
            w.registry
                .index(subject, currency, base, Weighting::AmountOutstanding),
        );
    }
    for basket in [IndexSubject::ConsumerPrices, IndexSubject::ProducerPrices] {
        indices.push(w.registry.index(basket, currency, base, Weighting::Equal));
    }
    assert_eq!(
        w.registry.indices_in(us),
        indices
            .iter()
            .copied()
            .filter(|index| w.registry.index_scope(*index) == currency)
            .collect::<Vec<_>>()
    );

    for (n, who) in everyone.iter().enumerate() {
        let at = n % deposits.len();
        w.register
            .money_delta(*who, deposits[at], draw.spread(10_000.0));
    }
    for b in &banks {
        w.register.money_delta(*b, reserves, draw.spread(200_000.0));
    }
    // The holdings the judged week walks.
    let traded: std::collections::HashSet<u32> = lines.iter().map(|l| l.0).collect();
    while w.register.rows() < HOLDINGS {
        let holder = everyone[draw.below(everyone.len() as u64) as usize];
        let line = InstrumentId::at(1 + draw.below(INSTRUMENTS as u64 - 1) as u32);
        if w.instruments.class_of(line) == Class::Money {
            continue;
        }
        seeded_as(&mut w, &traded, holder, line);
        w.register.credit(holder, line, draw.spread(500.0), 1.0, 0);
    }

    // Every claim owes something on a day, so there is something to fall behind on.
    for (line, issuer) in &claims {
        let holder = banks[draw.below(banks.len() as u64) as usize];
        seeded_as(&mut w, &traded, holder, *line);
        w.register
            .credit(holder, *line, draw.spread(1_000.0), 1.0, 0);
        for k in 0..PERIODS as i64 {
            // A coupon covers the week it falls at the end of, because a coupon IS a week.
            let due = Week(k * WEEK + draw.below(WEEK as u64) as i64);
            w.schedules.owes(
                phoenix_kernel::stores::Owed::On(*line),
                *issuer,
                w.instruments.ccy_of(*line),
                phoenix_kernel::stores::Payment {
                    from: Week(due.0 - WEEK),
                    due,
                    amount: draw.spread(20.0),
                    of: Owing::Interest,
                },
            );
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
            phoenix_kernel::stores::AgreementTerms::Engagement {
                wage_per_person: draw.spread(40.0),
                hours_per_person: draw.spread(35.0),
                heads,
            },
            Week(-365),
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
        w.processes.begin(
            kind,
            PartyId(*f),
            0,
            Some(draw.below(PERIODS as u64) as u32),
            draw.spread(500.0),
        );
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
            Venue {
                rule,
                protocol,
                seen_by: 5,
                stands_for,
            },
        );
    }

    // How the goods of this world are made.
    let goods: Vec<InstrumentId> = lines
        .iter()
        .filter(|l| w.instruments.class_of(**l) == Class::Good)
        .take(8)
        .copied()
        .collect();
    let plants: Vec<InstrumentId> = lines
        .iter()
        .filter(|l| w.instruments.class_of(**l) == Class::Plant)
        .take(8)
        .copied()
        .collect();

    for (n, plant) in plants.iter().enumerate() {
        // Arbitrary like everything else in this world: a range of footprints, so a place
        // fills at a rate that depends on what was built there rather than on how many things.
        let ground = 0.4 + draw.spread(1.6) * (1 + n % 3) as f64;
        w.registry.stands_on(
            *plant,
            Footprint::new(ground).expect("a plant stands on ground"),
        );
    }
    // RESIDENTIAL and COMMERCIAL: goods lines that are buildings rather than things.
    let buildings: Vec<InstrumentId> = goods.iter().rev().take(3).copied().collect();
    for (n, b) in buildings.iter().enumerate() {
        let ground = 0.02 + 0.03 * n as f64;
        w.registry.stands_on(
            *b,
            Footprint::new(ground).expect("a dwelling stands on ground"),
        );
    }

    // ── The road network, and the routes over it ──────────────────────────────────────
    // A road is capital the treasury owns, with a life and an upkeep, and nothing can be carried
    // anywhere until it exists.
    let road = w.instruments.issue(
        treasury,
        CurrencyCode::at(0),
        Class::Plant,
        UnitId::at(0),
        None,
        None,
    );
    w.registry
        .stands_on(road, Footprint::new(0.1).expect("a road stands on ground"));
    let stretches = w.lay_roads(treasury, road, 2_000.0, 1_560, 12.0);
    let carrier_of_record = everyone
        .iter()
        .copied()
        .find(|p| w.parties.kind_of(*p) == kinds::CARRIER)
        .expect("a world with routes has somebody to move things over them");
    let routes = w.connect_places(
        carrier_of_record,
        CurrencyCode::at(0),
        UnitId::at(1),
        Venue {
            rule: PriceRule::BuyersCompete,
            protocol: Protocol::Posted,
            seen_by: 5,
            stands_for: Some(1),
        },
    );
    // 38 B5: and every carrier owns VEHICLES — each its own line, one unit of it, standing
    // somewhere. A sale of that unit is a sale of the ship.
    let mut fleet = 0;
    for &who in everyone.iter() {
        if w.parties.kind_of(who) != kinds::CARRIER || !w.parties.alive(who) {
            continue;
        }
        let Some(at) = w
            .geography
            .site_of(who)
            .and_then(|site| w.geography.tile_of(site))
        else {
            continue;
        };
        // Arbitrary like everything else here: a range of sizes and a range of running costs, so a
        // route has a supply schedule rather than one number repeated.
        for n in 0..3u32 {
            let line = w.instruments.issue(
                who,
                CurrencyCode::at(0),
                Class::Plant,
                UnitId::at(0),
                None,
                None,
            );
            w.registry.is_plant(
                line,
                Plant {
                    life: 520,
                    // What it costs to KEEP, whether or not it sails.
                    upkeep_per_period: 8.0 + draw.spread(4.0),
                    capacity_per_period: 400.0 + draw.spread(600.0) * f64::from(n + 1),
                },
            );
            // What it costs to MOVE one unit, which is a different number, and how far it gets in
            // a week — a slower vehicle takes longer over the same route.
            w.registry
                .travels(line, 0.5 + draw.spread(1.5), 40.0 + draw.spread(60.0));
            w.geography
                .add_vehicle(line, at)
                .expect("a vehicle stands on the ground its owner does");
            fleet += 1;
        }
    }
    assert!(
        fleet > 0,
        "38 B5: a world with carriers and no vehicles moves nothing"
    );
    assert!(
        stretches > 0 && routes > 0,
        "49 G1: {stretches} stretches joined {routes} pairs of places, so nothing can be carried"
    );

    // How each good is made, and with what — declared into the registry, which is where the ids
    // point at everything else this world knows.
    // 49 I6: the first two goods come OUT OF THE GROUND. Everything else is made from another
    // good, which is what a world with no extraction looks like.
    let commodities: Vec<InstrumentId> = goods.iter().take(2).copied().collect();
    for (n, made) in goods.iter().enumerate() {
        let Some(from) = goods.get((n + 1) % goods.len()).copied() else {
            continue;
        };
        let Some(plant) = plants.get(n % plants.len().max(1)).copied() else {
            continue;
        };
        let ways = match commodities.contains(made) {
            true => vec![Way {
                per_unit: Vec::new(),
                labour_per_unit: 0.6,
                capital_services_per_unit: 0.2,
                yields: 0.95,
                batch: 10.0,
                periods_to_make: 1,
                extractive: true,
            }],
            false => vec![
                Way {
                    per_unit: vec![(from, 2.0)],
                    labour_per_unit: 0.2,
                    capital_services_per_unit: 0.1,
                    yields: 0.98,
                    batch: 10.0,
                    periods_to_make: 1,
                    extractive: false,
                },
                Way {
                    per_unit: vec![(from, 0.5)],
                    labour_per_unit: 1.5,
                    capital_services_per_unit: 0.1,
                    yields: 0.98,
                    batch: 10.0,
                    periods_to_make: 2,
                    extractive: false,
                },
            ],
        };
        w.registry.made_by(*made, ways, plant);
    }
    for plant in &plants {
        w.registry.is_plant(
            *plant,
            Plant {
                life: 200,
                upkeep_per_period: 0.5,
                capacity_per_period: 40.0,
            },
        );
    }

    // A maker is whoever holds the plant, so a world where the plant landed on parties
    // that employ nobody is a world that makes nothing.
    for (n, made) in w.registry.made().to_vec().iter().enumerate() {
        let maker = PartyId(firms[n % firms.len()]);
        let Some(plant) = w.registry.made_with(*made) else {
            continue;
        };
        seeded_as(&mut w, &traded, maker, plant);
        w.register.credit(maker, plant, 3.0, 1_000.0, 0);
        let inputs: Vec<InstrumentId> = w.registry.ways_of(*made)[0]
            .per_unit
            .iter()
            .map(|(what, _)| *what)
            .collect();
        for input in inputs {
            seeded_as(&mut w, &traded, maker, input);
            w.register
                .credit(maker, input, draw.spread(4_000.0), 0.5, 0);
        }
        // And a view of its own demand, which in a seeded world is what its own past sales gave it.
        w.outlooks.form(
            maker,
            running_about::HOW_MUCH_IT_SELLS,
            draw.spread(300.0),
            0,
        );
    }

    // ── What the ground holds ─────────────────────────────────────────────────────────
    // 49 I6: NOT EVERYWHERE. A commodity sits under some tiles and not others, which is what gives
    // this world a location basis at all. Declared unbounded, so nothing depletes yet (49 I3) and
    // the finite half of I1 is in the type waiting for a world that states one.
    let mut deposits = 0usize;
    let mut rights = 0usize;
    for (n, commodity) in commodities.iter().enumerate() {
        let land: Vec<phoenix_kernel::geography::TileId> = w
            .geography
            .tiles()
            .iter()
            .filter(|t| t.surface == phoenix_kernel::geography::Surface::Land)
            .map(|t| t.id)
            .collect();
        for tile in land.iter().skip(n).step_by(3) {
            if w.geography
                .add_deposit(phoenix_kernel::geography::Deposit {
                    tile: *tile,
                    of: *commodity,
                    holds: phoenix_kernel::geography::Held::Unbounded,
                })
                .is_err()
            {
                continue;
            }
            deposits += 1;
            // 49 I4: the right to work it is a holding, over THIS tile and THIS deposit, and it is
            // one unit of its own line so a sale of it is an ordinary transfer.
            let Some(region) = w.geography.region_of(*tile) else {
                continue;
            };
            let Some(who) = w
                .parties
                .of_kind(kinds::FIRM)
                .iter()
                .map(|row| PartyId(*row))
                .find(|f| w.parties.region_of(*f) == region)
            else {
                continue;
            };
            let line = w.instruments.issue(
                who,
                CurrencyCode::at(0),
                Class::Plant,
                UnitId::at(0),
                None,
                None,
            );
            w.registry.right_over(line, *tile, *commodity);
            // A right is held to WORK, not to trade, so it is carried at what it cost.
            w.register
                .carry(who, line, phoenix_kernel::register::Carrying::Cost);
            w.register.credit(who, line, 1.0, 0.0, 0);
            rights += 1;
        }
    }
    assert!(
        deposits > 0 && rights > 0,
        "49 I6: {deposits} deposits and {rights} rights — a world with no ground to work extracts nothing"
    );

    // Audit B5: a world that opens holding things opens owing somebody the difference, so the seed
    // states each account and the week after is the first one where the two records can disagree.
    let opening: Vec<(PartyId, f64)> = (0..w.parties.len() as u32)
        .filter_map(|row| {
            let party = PartyId(row);
            phoenix_kernel::instruments::booked_equity(
                party,
                &w.register,
                &w.instruments,
                &w.prints,
                &w.claims,
                w.week,
            )
            .map(|equity| (party, equity))
        })
        .collect();
    for (party, equity) in opening {
        w.equity.moves(
            party,
            equity,
            phoenix_kernel::stores::Moved::Opening,
            w.week,
        );
    }

    let wiring = Wiring {
        lines: lines.iter().take(8).copied().collect(),
        weekly_funding: None,
        paper: None,
    };
    let wired = all(&wiring, &w.registry, &mut w.journal, &mut w.nouns);
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
    println!(
        "         {} of {} declared numbers are shapes:",
        shapes.len(),
        w.params.len()
    );
    for (id, kind) in &shapes {
        match kind {
            ParamKind::Placeholder { mechanism } => {
                println!("           {id} — stands in for {mechanism}")
            }
            _ => println!("           {id}"),
        }
    }

    let mut worst = 0.0f64;
    // What each week's mechanisms decided, kept so the census below can ask over the run.
    let mut every_period_decided: Vec<Vec<u32>> = Vec::new();
    for week in 1..=PERIODS {
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
            .in_period(week)
            .filter(|n| {
                w.wire.cause_of(*n) == Cause::Production
                    && w.wire
                        .legs_of(*n)
                        .iter()
                        .any(|l| matches!(l, Leg::Create { .. }))
            })
            .count();
        // The population, as a READ over the cells — never a number anybody keeps (Small-Business
        // Pools E4: a constant population is the defect).
        let (mut cells, mut people) = (0usize, 0u64);
        for row in 0..w.parties.len() as u32 {
            let who = PartyId::at(row);
            if w.parties.alive(who)
                && matches!(w.parties.representation_of(who), Representation::Cell(_))
            {
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
                format!(
                    "{} not-built ({})",
                    r.family.name(),
                    r.contributors.join(", ")
                )
            });
        }
        audited.sort();

        // 3 C2, 22c.2: how many orders are STANDING.
        let standing = (0..w.resting.len() as u32)
            .map(phoenix_kernel::stores::RestingId)
            .filter(|o| w.resting.live(*o))
            .count();
        let brought = w.instruments.len();
        // What the world OWES, by date. A world whose schedule does not grow with its paper is a
        // world whose paper owes nothing.
        let owed = w.schedules.len();
        // How built-up the places are, as a read over the register.
        let built =
            phoenix_kernel::places::built_up(&w.parties, &w.register, &w.registry, &w.geography);
        let emptiest = built.iter().copied().fold(f64::INFINITY, f64::min);
        let fullest = built.iter().copied().fold(0.0f64, f64::max);
        // What became of THIS PERIOD's short payments.
        let (waiting, taken, late) = did.queue;
        println!(
            "week {week}  {ms:8.1} ms  — {} phases ran · {} asks · {} of {books} books cleared · {} trades · {} made · {} events · {} outlooks · {cells} cells of {people} · built {emptiest:.0}–{fullest:.0} km² · {brought} lines · {owed} due · {standing} resting",
            did.ran, did.asks, did.books_cleared, did.trades, made, did.events, w.outlooks.len(),
            books = w.books.len(),
        );
        println!(
            "           {} in flight closed · queue, of this week's short payments: {waiting} still waiting · {taken} went through after waiting ({} of them in a ring) · {late} ran out of days",
            did.closed, did.unwound,
        );
        println!("           audit: {}", audited.join(" · "));
        for r in &did.audit {
            for v in r.violations.iter().take(3) {
                println!(
                    "             [{}] {} {} {} — {}",
                    v.spec, v.owner, v.size, v.unit, v.message
                );
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
        "         {} of {} wired systems only counted, over {} weeks:",
        counting.len(),
        wired.iter().filter(|s| s.mechanism.is_some()).count(),
        PERIODS,
    );
    println!("           {}", counting.join(" "));

    println!();
    println!("worst week {worst:.1} ms against the 3,000 ms the migration was judged on.");
    println!(
        "All {} wired systems ran every week, in {} declared phases (the week's own thirty-one own the rest).",
        wired.len(),
        w.phases.len(),
    );
    println!(
        "The world is ARBITRARY: nothing in it was cleared, decided or seeded, and the seeding"
    );
    println!(
        "replaces it (5 E1). What it proves is that the machine runs in full, and what that costs."
    );
    println!("`made` counts the batches §37's lines ran, and a world that makes nothing sells its");
    println!("opening stock once and then has nothing to trade.");
}
