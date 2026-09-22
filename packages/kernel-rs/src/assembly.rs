//! THE ASSEMBLY: the systems, wired into one world that steps.
//!
//! @spec ARCHITECTURE 4.9b · 1 G3 · 3 B2 · 4 · Law 4, Law 10, Law 15, Law 19

use crate::clearing::{Order, Side};
use crate::ids::{CurrencyCode, InstrumentId, MarketId, PartyId};
use crate::instruments::{Class, Instruments};
use crate::journal::Journal;
use crate::ledger::{Instruction, Settlement, Settling};
use crate::module::{Mechanism, MechanismContext, Participant, ParticipantView, Stores as Reads};
use crate::nouns::{NounDecl, Nouns, Sort};
use crate::params::{Dimension, Kind, Owner, ParamDecl, Params};
use crate::parties::Parties;
use crate::prices::Prints;
use crate::register::Register;
use crate::registry::{Banks, Registry};
use crate::session::{run_book, BookDecl, Books, Shown, Stores};
use crate::stores::{Agreements, Claims, InProgress, Outlooks, Processes, Schedules, Standing};
use crate::world::{PhaseDecl, Phases, Produces, A1, A2, B1, E2, F, I1, I2, KERNEL};

/// The party kinds this world has.
pub mod kinds {
    pub const HOUSEHOLD: u32 = 0;
    pub const FIRM: u32 = 1;
    pub const BANK: u32 = 2;
    pub const FUND: u32 = 3;
    pub const INSURER: u32 = 4;
    pub const DEALER: u32 = 5;
    pub const TREASURY: u32 = 6;
    pub const CENTRAL_BANK: u32 = 7;
    pub const CARRIER: u32 = 8;
    pub const SMALL_FIRM: u32 = 9;
    pub const ASSESSOR: u32 = 10;
    /// 37 C3, 22c.4: SOMEBODY WHOSE BUSINESS IS TO HOLD THE STOCK.
    pub const STOCKIST: u32 = 11;
    pub const ALL: [u32; 12] = [
        HOUSEHOLD,
        FIRM,
        BANK,
        FUND,
        INSURER,
        DEALER,
        TREASURY,
        CENTRAL_BANK,
        CARRIER,
        SMALL_FIRM,
        ASSESSOR,
        STOCKIST,
    ];
}

/// One spec system, wired.
pub trait System {
    /// The spec system this is, for the record and for the census.
    fn name(&self) -> &'static str;

    /// Where its work falls in the week.
    fn phases(&self) -> Vec<PhaseDecl> {
        Vec::new()
    }

    /// Who it puts into books, if anybody.
    fn participants(&self) -> Vec<&dyn Participant> {
        Vec::new()
    }

    /// 46 F3: what its own family of thing is worth, where this system owns a family.
    fn valuers(&self) -> Vec<Box<dyn crate::module::Valuer>> {
        Vec::new()
    }

    /// Its own work in the week (ARCHITECTURE 4.9b, the second door): accruing, maturing,
    /// deciding, publishing.
    fn mechanism(&self) -> Option<&dyn Mechanism> {
        None
    }

    /// What this system contributes to the audit.
    fn audits(&self) -> Vec<Box<dyn crate::audit::Contribution>> {
        Vec::new()
    }
}

/// Every kernel store, declared for what it is.
fn declared() -> Nouns {
    let mut n = Nouns::new();
    let mut at_home = |name: &str, holds: &str, why: &str| {
        n.declare(NounDecl {
            name: name.to_string(),
            sort: Sort::Noun { home: None },
            holds: holds.to_string(),
            why: why.to_string(),
        });
    };
    at_home(
        "equity",
        "each party's equity account, as the movements that made it",
        "Audit B5: the residual has to have something independent to be equal to",
    );
    at_home(
        "geography",
        "one tiled surface, its jurisdictions, sites, network capital, routes and cargo",
        "49 A1: distance is a fact about where things are, and a model that carries a copy of it beside the map has two maps",
    );
    at_home(
        "parties",
        "who exists, named or a cell",
        "XI-15: a party is a fact about the world",
    );
    at_home(
        "instruments",
        "every priced thing, with its issuer",
        "Money A1: no instrument without an issuer",
    );
    at_home(
        "register",
        "who holds what, with lots and liens",
        "Register A1: ownership is the world's",
    );
    at_home(
        "prints",
        "what each book printed, with provenance",
        "Law 3: a price is a fact somebody cleared",
    );
    at_home(
        "journal",
        "every event, with its subjects",
        "Audit A2: what happened is not a module's scratch",
    );
    at_home(
        "wire",
        "every instruction ever applied",
        "Money D1: the wire IS the history",
    );
    at_home(
        "agreements",
        "relations: engagement, mortgage, policy, contract",
        "XI-10, 17f: a relation is not an instrument and not an event",
    );
    at_home(
        "schedules",
        "what each instrument owes, and when",
        "5 D2: a claim with no schedule is one nobody can fall behind on",
    );
    at_home(
        "outlooks",
        "what each party expects, formed from its own history",
        "§46: no global expectation; they disagree and it is load-bearing",
    );
    at_home(
        "processes",
        "what is in flight across weeks, with an owner",
        "XI-3: a process with no end is one nobody has to finish",
    );
    at_home(
        "claims",
        "who is owed what by a dead party, and at what rank",
        "XI-8, Appendix B: no liability without beneficiaries, and an estate pays in rank order",
    );
    // The four the registry now holds.
    at_home(
        "registry.currencies",
        "each money and the party whose liability it is",
        "Money A2: money is somebody's liability, and a CurrencyCode named nobody",
    );
    at_home("registry.places", "countries and the regions in them", "Seed B3: a country has the money and a region is a place, so currency_of(region) reads through the country");
    at_home("registry.units", "each unit and what one of it is divided into", "Law 8: the unit is part of the number, and one grid for everything made a dwelling divisible");
    at_home("registry.kind_profiles", "what varies by party kind, behind a dispatch the kernel reads", "Law 15: a world whose kinds have no profiles has nowhere to put what varies, so the pressure to branch never goes away");
    // The three a module named and no store kept.
    at_home("standing", "terms a party stands behind: a posting, a lending standard", "XI-10, Housing C5: a posting is HELD by an employer, which is what lets it be withdrawn; a standard is a decision that persists and that a borrower meets or does not");
    at_home(
        "making",
        "what is between input and output, owned, carrying what it cost",
        "37 B3: work in progress is a real thing with a holder, not a timing adjustment",
    );
    // It is the JOURNAL's, because a realised gain is an event rather than a thing anybody holds —
    // it happens at the moment the units leave, to a named party, for an amount.
    at_home("registry.indices", "each index, the country whose it is, and the lines it is built from with a COUNT of each", "Indices D1, 22 D5: an index is a COUNTRY's and it is ONE system; the level is never stored, it is computed from the constituents when asked");
    // The wire is what HAPPENED; this is what is still trying to.
    at_home("settlement.queue", "payments that could not be made yet, with the day each is late on", "XI-9: a gridlock is a timing failure and not a default, so a payment that cannot be made yet waits rather than becoming an arrear");

    // The kernel's own journal kinds. A module's are declared where the module is wired.
    at_home(
        "instruction.settled",
        "every instruction that applied, with its legs",
        "Money D1: the wire IS the history, and an outcome that is not recorded is one nobody can act on",
    );
    at_home(
        "instruction.failed",
        "every instruction that was refused, and why",
        "XI-5: a fail is a recorded state and never a silence",
    );
    at_home(
        "instruction.queued",
        "the payments waiting, and the day each is late on",
        "XI-9: a gridlock is a timing failure and not a default, so waiting is a state of its own",
    );
    at_home(
        "process.closed",
        "the processes that ended, and when",
        "XI-3: a process with no end is one nobody has to finish",
    );
    at_home(
        "disposal.realised",
        "what each disposal realised against the basis its lots carried",
        "Law 19: settlement is the only place that holds the price and the basis at once, so anywhere else would re-derive one of them",
    );
    at_home(
        "interest.accrued",
        "what each line has accrued to date, and what one unit of it carries",
        "Money G2.b: what accrues accrues before what falls due is paid, and it is worked out once rather than by every system that owes",
    );
    at_home(
        "population.moved",
        "each weight that moved, what it was and what it became",
        "XI-15: a weight moves by one of five events, each with a cause and a date, and this is the date",
    );
    // What a party stands behind until it withdraws it, one kind at a time.
    at_home(
        "standing.posting",
        "the open positions an employer holds",
        "XI-10, 39 B: a posting is HELD, which is what lets it be withdrawn",
    );
    at_home(
        "standing.lending_standard",
        "what each lender is currently lending at, and to whom it will",
        "12 A: a standard is a decision that persists, and a borrower meets it or does not",
    );
    at_home(
        "standing.grade",
        "the grade an assessor currently holds on a name, and what it was before",
        "21 A4, A6: two houses hold two rows on one name and may disagree, and a move is a restatement beside what was said before",
    );
    at_home(
        "standing.deposit_rate",
        "the rate each bank pays on deposits",
        "25 B: a depositor moves for a rate, so the rate has to be a thing the bank stands behind",
    );
    at_home(
        "standing.own_view",
        "each lender's own view of a borrower",
        "12 B: a lender that borrows somebody else's opinion has not formed one",
    );
    at_home(
        "standing.estimate",
        "what each bank expects a named company to report",
        "48 C1, C3: a consensus computed from a list that lives for one call cannot hold the disagreement it is the mean of",
    );
    at_home(
        "standing.redemption_gate",
        "which funds are currently gating redemptions",
        "13 D: a gate is a stated position holders act on, and it is withdrawn the same way",
    );
    at_home(
        "standing.capital_distribution",
        "whether each bank may currently distribute capital",
        "28 D: a restriction on distributions is a position somebody stands behind, and it binds until it is lifted",
    );
    // The rest of the kernel's own stores.
    at_home(
        "params",
        "every behaviour-shaping number, with its kind, unit and owner",
        "XI-14: the engine reads numbers only through the register that declares them",
    );
    at_home(
        "registry",
        "what the ids point at",
        "Law 15: the kernel asks a kind's profile rather than branching on its id",
    );
    at_home(
        "nouns",
        "what every store is: a noun, a working store or physics",
        "Law 15: a store nobody declared is a fact of the world nobody owns",
    );
    at_home(
        "phases",
        "the week's order, as data",
        "Money G2: the causal order inside a period is fixed, and a fixed order is a thing a reader can see",
    );
    at_home(
        "audit",
        "the families, and what each found",
        "Audit C2: the same invariants every period, so the period a violation first appears is known",
    );
    at_home(
        "valuers",
        "what each family of thing is worth, answered by the system that owns the family",
        "46 F3: the comparison is one mechanism and the terms are the thing's own, so a formula true of every family would be a decision taken at an average",
    );
    at_home(
        "resting",
        "the orders standing between sessions",
        "3 C2: a book opens with what was left standing in it, and an order that expired says so",
    );
    at_home(
        "books",
        "every market, its subject, its money and its venue",
        "3 A1: a place to trade is declared by whoever opened it, with its rule and its protocol",
    );
    at_home(
        "sessions",
        "every book that ran, including the ones that did not clear",
        "Clearing D: a book that printed nothing is a fact about the week, not an absence of one",
    );
    at_home(
        "calendar",
        "the one mapping from a week to a date",
        "Money G3: one calendar, and every periodicity placed on it by date",
    );
    at_home(
        "week",
        "where the world is",
        "Money G1: a period is the unit of causation, so which one it is has to be one number",
    );
    at_home(
        "says",
        "the journal kinds the kernel itself says under",
        "Audit A2: what the kernel did is not the kernel's scratch, so it is said under a named kind",
    );
    at_home(
        "config",
        "the seed and the resolutions a world is built from, before any number is declared",
        "Seed A5, Audit D3: a run is reproducible from a seed value, so the seed is part of what the world IS",
    );

    // AND WHAT HAS NO HOME. `home` names the store that will hold it, which is what makes the
    // count a worklist rather than a shrug.
    let mut homeless = |name: &str, home: &str, holds: &str, why: &str| {
        n.declare(NounDecl {
            name: name.to_string(),
            sort: Sort::Noun {
                home: Some(home.to_string()),
            },
            holds: holds.to_string(),
            why: why.to_string(),
        });
    };

    homeless(
        "control.resistance",
        "standing",
        "what a target's management is doing to defend it, and what that costs the bidder",
        "35 C3: management may resist and its interests differ from the owners', which is the corporate-control problem — and nothing in this world resists, so a tender meets only the owners' own valuations",
    );
    homeless(
        "derivatives.collateral",
        "register",
        "what is posted against a derivative position, by whom, and what it is worth now",
        "16 C1, Appendix B: no margin that is only a number. A position marks and nothing is posted against it, so a counterparty exposure has nothing standing behind it",
    );
    homeless(
        "agreements.states",
        "agreements",
        "whether a relation is live, breached, cured, discharged or terminated",
        "`Agreements` is live or ended, so this world records neither a breach nor a cure — and a payer given time on an arrear has nowhere to land",
    );

    n
}

/// Immutable construction inputs for one reproducible run.
///
/// This is not a second parameter register. It contains only the seed and kernel resolutions needed
/// before `Params` exists; behavioural numbers remain declared in `Params`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunConfig {
    pub seed: u64,
    pub payment_wait_weeks: u32,
    pub money_pieces_per_unit: f64,
    pub time_pieces_per_unit: f64,
    /// 49 A1: the grid the physical world is drawn on — how many tiles across and down, and how
    /// wide one is. It is a RESOLUTION like the other two: distance is what decides, and the grid
    /// it is measured on must not.
    pub tiles_east: u32,
    pub tiles_north: u32,
    pub tile_side_km: f64,
    /// 49 A2: the elevation below which a tile is water. Physics, and the one thing here nobody
    /// decides.
    pub sea_level_m: f64,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            seed: 0x9E37_79B9_7F4A_7C15,
            payment_wait_weeks: 1,
            money_pieces_per_unit: 100.0,
            time_pieces_per_unit: 60.0,
            tiles_east: 16,
            tiles_north: 16,
            tile_side_km: 50.0,
            sea_level_m: 0.0,
        }
    }
}

impl RunConfig {
    fn validate(self) {
        assert!(
            self.payment_wait_weeks > 0,
            "Money G1: a payment that may wait no week does not wait"
        );
        assert!(
            self.money_pieces_per_unit.is_finite() && self.money_pieces_per_unit > 0.0,
            "Law 6: the money resolution must be finite and positive"
        );
        assert!(
            self.time_pieces_per_unit.is_finite() && self.time_pieces_per_unit > 0.0,
            "Law 6: the time resolution must be finite and positive"
        );
        assert!(
            self.tiles_east > 0 && self.tiles_north > 0,
            "49 A1: a surface with no tiles is nowhere"
        );
        assert!(
            self.tile_side_km.is_finite() && self.tile_side_km > 0.0,
            "Law 6: the spatial resolution must be finite and positive"
        );
        assert!(
            self.sea_level_m.is_finite(),
            "Law 6: a sea level of {} is not a level",
            self.sea_level_m
        );
    }
}

/// The stores, one of each, owned by the kernel.
pub struct World {
    /// The immutable inputs that identify this run.
    pub config: RunConfig,
    pub parties: Parties,
    pub instruments: Instruments,
    pub register: Register,
    pub prints: Prints,
    pub journal: Journal,
    pub wire: Settlement,
    pub params: Params,
    /// The relations.
    pub agreements: Agreements,
    /// What each instrument owes and when.
    pub schedules: Schedules,
    /// What each deciding party expects, formed from its own history.
    pub outlooks: Outlooks,
    /// Whatever is in flight across weeks with an owner and an end.
    pub processes: Processes,
    /// Who is owed what by a party whose life has ended, and at what rank.
    pub claims: Claims,
    /// Audit B5: each party's equity account, as the movements that made it.
    pub equity: crate::stores::Equity,
    /// 49 A: the physical world — one tiled surface, its jurisdictions, sites, network and cargo.
    pub geography: crate::geography::Geography,
    /// ARCHITECTURE 4.10, 21e: what the ids point at — each money's issuer, each country's money and
    /// each region's country, each unit's subdivision, and a profile per party kind.
    pub registry: Registry,
    /// Terms a named party stands behind until it withdraws them — a posting, a lending standard.
    pub standing: Standing,
    /// What is between input and output, owned by somebody, carrying what it cost.
    pub making: InProgress,
    /// The ontology register.
    pub nouns: Nouns,
    pub phases: Phases,
    /// The families, assembled at `wire_up` and run every week.
    pub audit: crate::audit::Audit,
    /// 46 F3: what each family of thing is worth, asked of the system that owns the family.
    pub valuers: Vec<Box<dyn crate::module::Valuer>>,
    /// 3 C2, 22c.2: the standing book — orders that rest between sessions.
    pub resting: crate::stores::Resting,
    pub books: Vec<BookDecl>,
    /// Every completed book session, including failed and partially filled auctions.
    pub sessions: Vec<crate::session::Session>,
    pub calendar: crate::calendar::Calendar,
    pub week: u32,
    /// The journal kinds an instruction's outcome is said under — settled, failed, queued, and what
    /// a disposal realised.
    pub says: crate::ledger::Outcomes,
}

/// What one week did.
#[derive(Debug, Default)]
pub struct Stepped {
    /// What the audit found in this week, by family.
    pub audit: Vec<crate::audit::Report>,
    pub asks: usize,
    pub narrows: usize,
    pub books_cleared: usize,
    pub trades: usize,
    pub events: usize,
    /// How many systems' phases actually ran.
    pub ran: usize,
    /// How many queued payments the gridlock pass settled, in cycles nobody in them could have paid
    /// alone.
    pub unwound: usize,
    /// What became of THIS week's short payments — still waiting, went through after waiting, ran
    /// out of days.
    pub queue: (usize, usize, usize),
    /// How many processes reached their end this week.
    pub closed: usize,
    /// The declaration slots whose mechanism DECIDED something this week — read off what it asked
    /// for (`Taken::decided`), never declared by the mechanism itself.
    pub decided: Vec<u32>,
}

impl World {
    /// The world before anything has happened to it.
    pub fn empty() -> World {
        Self::with_config(RunConfig::default())
    }

    /// The world before anything has happened, under an explicit reproducibility contract.
    pub fn with_config(config: RunConfig) -> World {
        Self::with_parameters(config, |_| {})
    }

    /// Construct a world and its one parameter register from the same explicit run input.
    pub fn with_parameters(config: RunConfig, declare: impl FnOnce(&mut Params)) -> World {
        config.validate();
        let mut params = Params::new(config.money_pieces_per_unit, config.time_pieces_per_unit);
        params.declare(ParamDecl {
            id: "outlook.memory.from".to_string(),
            value: 2.0,
            unit: "weeks".to_string(),
            dimension: Dimension::Weeks,
            kind: Kind::Preference,
            owner: None,
            why: "the shortest memory an entering party may draw".to_string(),
        });
        params.declare(ParamDecl {
            id: "outlook.memory.to".to_string(),
            value: 10.0,
            unit: "weeks".to_string(),
            dimension: Dimension::Weeks,
            kind: Kind::Preference,
            owner: None,
            why: "the exclusive upper bound of an entering party's memory draw".to_string(),
        });
        params.declare(ParamDecl {
            id: "deposit.insurance.limit_per_member".to_string(),
            value: 50.0,
            unit: "price".to_string(),
            dimension: Dimension::Price,
            kind: Kind::Policy,
            owner: Some(Owner::Parliament),
            why: "the insured balance transferred per member when a bank enters resolution"
                .to_string(),
        });
        declare(&mut params);
        let mut journal = Journal::new();
        // Settled, failed, QUEUED — a payment waiting for the money to arrive is neither of the
        // other two — and what a disposal realised.
        let says = crate::ledger::Outcomes::declared(&mut journal);
        World {
            config,
            parties: Parties::with_seed_and_memory(
                config.seed,
                params.weeks("outlook.memory.from"),
                params.weeks("outlook.memory.to"),
            ),
            instruments: Instruments::new(),
            register: Register::new(),
            prints: Prints::new(),
            journal,
            // How many days a payment may wait here before it is late.
            wire: Settlement::new(config.payment_wait_weeks),
            params,
            agreements: Agreements::new(),
            schedules: Schedules::new(),
            outlooks: Outlooks::new(),
            processes: Processes::new(),
            claims: Claims::new(),
            equity: crate::stores::Equity::new(),
            geography: crate::geography::Geography::generate(
                crate::geography::GenerationShape {
                    width: config.tiles_east,
                    height: config.tiles_north,
                    tile_side: crate::geography::Kilometres(config.tile_side_km),
                    sea_level: crate::geography::Metres(config.sea_level_m),
                },
                crate::geography::Provenance {
                    run_seed: config.seed,
                    substream: "geography".to_string(),
                    algorithm_version: 1,
                    projection: "equirectangular".to_string(),
                },
            )
            .expect("49 A1: the run's own grid is a surface"),
            registry: Registry::new(),
            standing: Standing::new(),
            making: InProgress::new(),
            nouns: declared(),
            audit: crate::audit::Audit::new(),
            resting: crate::stores::Resting::new(),
            phases: Phases::new(),
            books: Vec::new(),
            valuers: Vec::new(),
            sessions: Vec::new(),
            // One configured week and epoch; settlement still happens once per week.
            calendar: crate::calendar::Calendar::new(),
            week: 0,
            says,
        }
    }

    /// Every system's phases, in one order, sealed — a phase that reads a not-yet-produced print
    /// throws rather than reading a stale one.
    pub fn wire_up(&mut self, systems: &[&dyn System]) {
        if self.params.declared("household.keeps.from")
            && self.params.declared("household.keeps.to")
            && self.params.declared("household.will_spend.from")
            && self.params.declared("household.will_spend.to")
        {
            let from = self
                .params
                .amount("household.keeps.from", crate::params::Denomination::Money);
            let to = self
                .params
                .amount("household.keeps.to", crate::params::Denomination::Money);
            let spend_from = self.params.ratio("household.will_spend.from");
            let spend_to = self.params.ratio("household.will_spend.to");
            for row in self.parties.of_kind(kinds::HOUSEHOLD).to_vec() {
                let household = PartyId::at(row);
                if self.parties.household_keeps(household).is_none() {
                    self.parties
                        .assign_household_preferences(household, from, to, spend_from, spend_to);
                }
            }
        }
        // A system wired twice would run twice, and one that neither works nor posts is a
        // declaration and nothing else. Both are caught here because this is the one door.
        let mut named: Vec<&str> = systems.iter().map(|s| s.name()).collect();
        named.sort_unstable();
        let wired = named.len();
        named.dedup();
        assert_eq!(
            wired,
            named.len(),
            "ARCHITECTURE 4.9b: a system is wired once"
        );
        for s in systems {
            assert!(
                s.mechanism().is_some() || !s.participants().is_empty(),
                "ARCHITECTURE 4.9b: {} is wired and neither works nor posts",
                s.name()
            );
            for phase in s.phases() {
                self.phases.add(phase);
            }
        }
        self.phases.seal();
        self.every_store_is_declared();
        // The audit is assembled here, with the phases.
        let mut contributions: Vec<Box<dyn crate::audit::Contribution>> = vec![
            Box::<crate::audit::ATotalCarriesNoLots>::default(),
            Box::<crate::audit::NoCollateralCountedTwice>::default(),
            Box::<crate::audit::CellWeightsConserve>::default(),
            Box::<crate::audit::CellOwnedUnitsReachLiveRows>::default(),
            Box::<crate::audit::CellAgreementsReachLiveRows>::default(),
            Box::<crate::audit::CellHoldingsDivideByWeight>::default(),
            Box::<crate::audit::HoldersAgainstIssued>::default(),
            Box::<crate::audit::MarketValuesExist>::default(),
            Box::<crate::audit::AccountsBalance>::default(),
            Box::<crate::audit::MoneyIsConserved>::default(),
            Box::<crate::audit::FlowsAreComplete>::default(),
            Box::<crate::audit::ScheduleOutcomesMatch>::default(),
            Box::<crate::audit::NamesResolve>::default(),
            Box::<crate::audit::CrossMarketValues>::default(),
            Box::<crate::audit::ClearingReconciles>::default(),
            Box::<crate::audit::BilateralDerivativesAreZeroSum>::default(),
            Box::<crate::audit::MarketDecisionLiveness>::default(),
        ];
        contributions.extend(crate::geography::contributions());
        // 46 F3: and what each family is worth, asked of the system that owns the family.
        self.valuers = systems.iter().flat_map(|s| s.valuers()).collect();
        for s in systems {
            contributions.extend(s.audits());
        }
        self.audit = crate::audit::Audit::over(contributions);
    }

    /// ONE PERIOD, IN ONE PASS OVER THE THIRTY-ONE SLOTS. A slot marker does the kernel's own work
    /// of that slot; everything between two markers is the modules declared in the earlier one.
    pub fn step(&mut self, systems: &[&dyn System]) -> Stepped {
        self.week += 1;
        // And the parties store knows what week it is, so a party entering in it is stamped with
        // it.
        self.parties.opened(self.week);
        let participants: Vec<&dyn Participant> =
            systems.iter().flat_map(|s| s.participants()).collect();
        let mut out = Stepped::default();
        let mut posted: Vec<crate::session::Posted> = Vec::new();
        let events_before = self.journal.len();
        let today = self
            .calendar
            .at(crate::calendar::Week(i64::from(self.week)));
        let order: Vec<(u32, u32, u32)> = self
            .phases
            .order()
            .iter()
            .map(|p| (p.owner, p.name, p.at))
            .collect();
        let by_slot = slots(systems);

        for (owner, name, at) in &order {
            match (*owner, *name) {
                (KERNEL, A1) => self.expires(today, &mut out),
                (KERNEL, A2) => self.gives_up(today),
                (KERNEL, B1) => self.accrues(today),
                // e2 asks, f clears: what a party posted is what the book it posted into gets, and
                // it gets it a slot later.
                (KERNEL, E2) => posted = self.collect_orders(&participants, &mut out),
                (KERNEL, F) => out.trades += self.run_books(&posted, &mut out),
                (KERNEL, I1) => self.unwinds(&mut out),
                (KERNEL, I2) => self.audits(&mut out),
                (KERNEL, _) => {}
                _ => out.ran += self.run_phase(*owner, *at, &by_slot, systems, &mut out),
            }
        }

        // And what became of the payments that were short in it.
        let closes = crate::calendar::Week(i64::from(self.week));
        out.queue = self.wire.queue.between(today, closes);
        out.events = self.journal.len() - events_before;
        out
    }

    /// SLOT a1 — what stood to a past week expires, and what was in flight whose time has come
    /// closes (Money G2.a).
    fn expires(&mut self, today: crate::calendar::Week, out: &mut Stepped) {
        // 3 C2, G3.a, 22c2.2: an offer that stood to a past week goes before anything reads a book.
        self.resting.expire(today);
        self.agreements.expire(today);
        // AND WHATEVER IS IN FLIGHT CLOSES WHEN ITS PERIOD COMES — here, because every reader of
        // what is afoot asks whether one is running and every one of them runs from WORK on.
        let closing: Vec<crate::stores::ProcessId> = (0..self.processes.len() as u32)
            .map(crate::stores::ProcessId)
            .filter(|p| !self.processes.done(*p))
            .filter(|p| self.processes.completion_met(*p, self.week))
            .collect();
        out.closed = closing.len();
        for p in closing {
            let (owner, size) = (self.processes.owner(p), self.processes.size(p));
            self.processes.finish(p);
            // What closed, whose it was and how big it was.
            self.journal.say(
                self.week,
                self.says.closed,
                &[owner.0],
                &[(0, crate::journal::Value::Num(size))],
                true,
            );
        }
    }

    /// SLOT a2 — a payment past its deadline is GIVEN UP, which is a recorded state and never a
    /// silence (Money G2.a).
    fn gives_up(&mut self, today: crate::calendar::Week) {
        self.wire.give_up(
            today,
            self.week,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &mut self.instruments,
                calendar: &self.calendar,
                says: self.says,
                equity: &mut self.equity,
            },
        );
        self.apply_due_updates();
    }

    /// SLOT b1 — WHAT ACCRUES ACCRUES (Money G2.b), before what falls due is paid.
    ///
    /// One pass, so the accrual on a coupon, a loan, a premium, a fee and a bill's own discount is
    /// worked out in one place and every sheet reads the same number. It writes nothing: what a
    /// holder is owed is `accrued_to` and what an issuer owes is `accrued_by`, both reads over the
    /// schedules, and this says for the week what those reads would answer.
    fn accrues(&mut self, today: crate::calendar::Week) {
        for row in 0..self.instruments.len() {
            let line = InstrumentId(row as u32);
            let Some(per_unit) = crate::module::accrued_per_unit(
                &self.register,
                &self.schedules,
                &self.instruments,
                &self.prints,
                line,
                today,
            ) else {
                continue;
            };
            let outstanding =
                crate::module::outstanding_of(&self.register, &self.instruments, line);
            self.journal.say(
                self.week,
                self.says.accrued,
                &[line.0, self.instruments.issuer_of(line).0],
                &[
                    (0, crate::journal::Value::Num(per_unit)),
                    (1, crate::journal::Value::Num(per_unit * outstanding)),
                ],
                true,
            );
        }
    }

    /// SLOT i1 — the gridlock pass over every payment the week holds, so a ring that can settle
    /// together does (Money G2.i).
    fn unwinds(&mut self, out: &mut Stepped) {
        out.unwound = self.wire.unwind(
            self.week,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &mut self.instruments,
                calendar: &self.calendar,
                says: self.says,
                equity: &mut self.equity,
            },
        );
        self.apply_due_updates();
    }

    /// SLOT i2 — the audit, over what the week actually left behind (Audit C1).
    fn audits(&mut self, out: &mut Stepped) {
        // EVERY FAMILY, EVERY PERIOD, over the one traversal it was built for.
        out.audit = self.audit.run(&crate::audit::Sources {
            wire: &self.wire,
            register: &self.register,
            instruments: &self.instruments,
            parties: &self.parties,
            week: self.week,
            prints: Some(&self.prints),
            claims: Some(&self.claims),
            schedules: Some(&self.schedules),
            agreements: Some(&self.agreements),
            sessions: Some(&self.sessions),
            equity: Some(&self.equity),
            geography: Some(&self.geography),
        });
    }

    /// Law 15: the ontology register is a RULE and not a list, so every store the world keeps is
    /// passed through it once the systems are wired — and a store nobody declared throws here,
    /// before a week has run, rather than being discovered in a reader's arithmetic.
    fn every_store_is_declared(&self) {
        // Destructured with no `..`, so a new store fails to compile until it is named below and
        // declared in `declared()`.
        let World {
            parties: _,
            instruments: _,
            register: _,
            prints: _,
            journal: _,
            wire: _,
            params: _,
            agreements: _,
            schedules: _,
            outlooks: _,
            processes: _,
            claims: _,
            equity: _,
            geography: _,
            registry: _,
            standing: _,
            making: _,
            nouns: _,
            phases: _,
            audit: _,
            resting: _,
            books: _,
            valuers: _,
            sessions: _,
            calendar: _,
            week: _,
            says: _,
            config: _,
        } = self;
        for store in [
            "parties",
            "instruments",
            "register",
            "prints",
            "journal",
            "wire",
            "params",
            "agreements",
            "schedules",
            "outlooks",
            "processes",
            "claims",
            "equity",
            "geography",
            "registry",
            "standing",
            "making",
            "nouns",
            "phases",
            "audit",
            "valuers",
            "resting",
            "books",
            "sessions",
            "calendar",
            "week",
            "says",
            "config",
        ] {
            self.nouns.sort_of(store);
        }
        // And the two places a module keeps state without a store of its own.
        for (_, kind) in crate::stores::standing::ALL {
            self.nouns.sort_of(kind);
        }
        for row in 0..self.journal.kinds.len() as u32 {
            self.nouns.sort_of(self.journal.kinds.name(row));
        }
    }

    /// One system's phase: its mechanism reads the stores, proposes, and the kernel settles.
    fn split_cell(
        &mut self,
        parent: PartyId,
        taking: std::num::NonZeroU32,
        carrying: crate::stores::AgreementId,
        destination: crate::parties::LatticeKey,
    ) {
        let had = self.parties.weight(parent);
        let share = f64::from(taking.get()) / f64::from(had);
        // XI-15: a weight moves by a named event, and the event has a dated row of its own that the
        // population's history points at — so a reader asking why a cell is this size has somewhere
        // to look.
        let said = self.journal.say(
            self.week,
            self.says.population,
            &[parent.0],
            &[
                (0, crate::journal::Value::Num(f64::from(had))),
                (1, crate::journal::Value::Num(f64::from(taking.get()))),
            ],
            true,
        );
        let child = self.parties.split(parent, taking, destination, said);

        let mut legs: Vec<crate::ledger::Leg> = Vec::new();
        // A claim over units belongs to the units, so the members who leave take their share of it
        // with them: it comes off the parent before they move and goes on the child after.
        let mut claims: Vec<(PartyId, crate::ids::InstrumentId, f64)> = Vec::new();
        let inherited: Vec<u32> = self.register.of_holder(parent).to_vec();
        for row in inherited {
            let row = crate::ids::HoldingId(row);
            let line = self.register.instrument_of(row);
            // Their share of everything they held, encumbered or not — a share of nothing is not
            // a leg.
            let Some(theirs) = crate::ledger::Units::new(self.register.quantity(row) * share)
            else {
                continue;
            };
            let theirs_of: Vec<(PartyId, f64)> = self
                .register
                .liens(row)
                .iter()
                .map(|lien| (lien.to, lien.qty * share))
                .collect();
            for (to, qty) in theirs_of {
                self.register.release(parent, line, to, qty);
                claims.push((to, line, qty));
            }
            legs.push(if self.instruments.class_of(line) == Class::Money {
                crate::ledger::Leg::Money {
                    from: parent,
                    to: child,
                    instrument: line,
                    amount: theirs,
                    receipt: crate::ledger::Receipt::Transfer,
                }
            } else {
                self.register
                    .carry(child, line, declared_on(&self.register, row));
                crate::ledger::Leg::Asset {
                    from: parent,
                    to: child,
                    instrument: line,
                    qty: theirs,
                    // No price.
                    price_per_unit: None,
                }
            });
        }
        if !legs.is_empty() {
            let instruction = Instruction {
                legs: &legs,
                cause: crate::ledger::Cause::CorporateAction,
                delivery: crate::ledger::Delivery::Free,
                due: None,
            };
            self.wire.settle(
                &instruction,
                self.week,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                    equity: &mut self.equity,
                },
            );
        }

        // And the claims land on the units they are over, now that they are there.
        for (to, line, qty) in claims {
            self.register.pledge(child, line, to, qty);
        }

        // One group, one history.
        for row in self.outlooks.of_party(parent).to_vec() {
            let about = self.outlooks.about_at(row);
            let level = self.outlooks.level_at(row);
            self.outlooks.form(child, about, level, self.week);
        }

        // And the relationship that applies to them goes with them.
        self.agreements.moves(carrying, parent, child);
    }

    /// AN OBLIGATION COMES INTO EXISTENCE.
    fn brought(&mut self, what: crate::module::Brings) {
        let initial_holder = what.initial_holder.unwrap_or(what.issuer);
        let line = self.instruments.issue(
            what.issuer,
            what.ccy,
            what.class,
            what.unit,
            what.coupon,
            what.matures,
        );
        if let Some(terms) = what.loan_terms {
            self.instruments.records_negotiated_loan(line, terms);
        }
        // Settlement is the one writer of the register, so units arrive over the wire like
        // everything else — and what the holder says it holds them FOR is said before they do.
        self.register.carry(initial_holder, line, what.carried_as);
        if let Some(units) = crate::ledger::Units::new(what.units) {
            let issue_basis = match (what.initial_holder, what.issue_price) {
                (_, Some(price)) => price,
                (None, None) => 0.0,
                (Some(_), None) => {
                    panic!("direct issuance to a holder must carry that holder's bid")
                }
            };
            let legs = [crate::ledger::Leg::Create {
                party: initial_holder,
                instrument: line,
                qty: units,
                cost_per_unit: issue_basis,
            }];
            let instruction = Instruction {
                legs: &legs,
                cause: crate::ledger::Cause::CorporateAction,
                delivery: crate::ledger::Delivery::Nothing,
                due: None,
            };
            self.wire.settle(
                &instruction,
                self.week,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                    equity: &mut self.equity,
                },
            );
        }
        // A book for it, if it is paper anybody else may bid for.
        if let Some(venue) = what.book {
            self.open_book(crate::ids::book_of(line), line, what.ccy, venue);
        }
        // And what it owes, generated from its own terms — Bond N6, and the one writer of a
        // schedule row, so no issuer carries a copy of the contract's arithmetic.
        if let Some(matures) = what.matures {
            let issued_on = self
                .calendar
                .at(crate::calendar::Week(i64::from(self.week)));
            // Kept as an explicit contract branch: repository law checks forbid a numeric fallback
            // because most missing terms must remain missing. N5.c is the declared exception: no
            // coupon means a zero-coupon bill, while principal still exists.
            #[allow(clippy::manual_unwrap_or)]
            let coupon = match what.coupon {
                Some(rate) => rate,
                None => 0.0,
            };
            for payment in crate::instruments::schedule_of(
                &self.calendar,
                issued_on,
                matures,
                what.units,
                coupon,
                what.pays,
                what.convention,
            ) {
                self.schedules.owes(
                    crate::stores::Owed::On(line),
                    what.issuer,
                    what.ccy,
                    payment,
                );
            }
        }
        if let Some(terms) = what.loan_terms {
            let issued_on = self
                .calendar
                .at(crate::calendar::Week(i64::from(self.week)));
            let due = self
                .calendar
                .at(crate::calendar::Week(i64::from(self.week + terms.tenor)));
            self.schedules.owes(
                crate::stores::Owed::On(line),
                what.issuer,
                what.ccy,
                crate::stores::Payment {
                    from: issued_on,
                    due,
                    amount: terms.amount,
                    of: crate::stores::Owing::Principal,
                },
            );
        }
    }

    fn run_phase(
        &mut self,
        owner: u32,
        at_stage: u32,
        by_slot: &[usize],
        systems: &[&dyn System],
        out: &mut Stepped,
    ) -> usize {
        let at = match by_slot.get(owner as usize) {
            Some(at) if *at < systems.len() => *at,
            // The three kernel moments own slots nothing declares a mechanism for.
            _ => return 0,
        };
        let m = match systems[at].mechanism() {
            Some(m) => m,
            None => return 0,
        };
        let mut ctx = MechanismContext::of(
            self.week,
            at_stage,
            Reads {
                valuers: &self.valuers,
                claims: &self.claims,
                equity: &self.equity,
                geography: &self.geography,
                parties: &self.parties,
                instruments: &mut self.instruments,
                register: &self.register,
                prints: &self.prints,
                journal: &self.journal,
                params: &self.params,
                outlooks: &self.outlooks,
                agreements: &self.agreements,
                schedules: &self.schedules,
                processes: &self.processes,
                wire: &self.wire,
                standing: &self.standing,
                making: &self.making,
                registry: &self.registry,
                calendar: &self.calendar,
                books: &self.books,
                sessions: &self.sessions,
            },
        );
        m.run(&mut ctx);
        let asked = ctx.taken();
        // What it asked for is what it did, and this is the only place both are in hand at once.
        if asked.decided() {
            out.decided.push(owner);
        }

        // The cell events come first, because they change WHO the parties are.
        for (parent, taking, carrying, destination) in asked.split {
            self.split_cell(parent, taking, carrying, destination);
        }
        for (cell, destination) in asked.transitioned {
            self.parties.transition(cell, destination);
        }
        // And obligations that have come into existence.
        for what in asked.issued {
            self.brought(what);
        }
        // And the bilateral ones it struck, which fall due like any other.
        for owing in asked.owing {
            let crate::stores::Owed::To(payee) = owing.on else {
                panic!("a contractual bilateral due must name its payee");
            };
            match owing.agreement {
                Some(agreement) => {
                    let parties = self.agreements.between(agreement);
                    assert!(
                        (parties.0 == owing.owed_by && parties.1 == payee)
                            || (parties.1 == owing.owed_by && parties.0 == payee),
                        "Law 5: a contractual due must remain between its agreement's parties"
                    );
                    self.schedules.owes_under(
                        agreement,
                        payee,
                        owing.owed_by,
                        owing.ccy,
                        owing.payment,
                    );
                }
                None => {
                    self.schedules
                        .owes(owing.on, owing.owed_by, owing.ccy, owing.payment);
                }
            }
        }
        // And relations struck and processes opened.
        let today = self
            .calendar
            .at(crate::calendar::Week(i64::from(self.week)));
        for a in asked.agreed {
            self.agreements
                .strike(a.kind, a.one, a.other, a.terms, today, a.until);
        }
        for contract in asked.contracted {
            let payee = if contract.owed_by == contract.agreement.one {
                contract.agreement.other
            } else {
                assert_eq!(
                    contract.owed_by, contract.agreement.other,
                    "Law 5: a new contractual due must be owed by one of its parties"
                );
                contract.agreement.one
            };
            let agreement = self.agreements.strike(
                contract.agreement.kind,
                contract.agreement.one,
                contract.agreement.other,
                contract.agreement.terms,
                today,
                contract.agreement.until,
            );
            for payment in contract.payments {
                self.schedules.owes_under(
                    agreement,
                    payee,
                    contract.owed_by,
                    contract.ccy,
                    payment,
                );
            }
        }
        for o in asked.opened {
            self.processes.begin_for(
                o.kind,
                o.owner,
                self.week,
                o.closes,
                o.size,
                crate::stores::ProcessTarget {
                    door: o.door,
                    subject: o.subject,
                },
            );
        }

        // XI-6: what a holder said it holds a position FOR, before settlement puts units in it.
        for (who, line, as_) in asked.carried {
            self.register.carry(who, line, as_);
        }

        // Settlement is the one writer of the register.
        for p in asked.proposed {
            let instruction = Instruction {
                legs: &p.legs,
                cause: p.cause,
                delivery: p.delivery,
                due: p.due,
            };
            self.wire.settle(
                &instruction,
                self.week,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                    equity: &mut self.equity,
                },
            );
        }
        self.apply_due_updates();
        // And what it said happened, for whoever it happened to.
        for s in asked.said {
            self.journal
                .say(self.week, s.kind, &s.subjects, &s.data, s.public);
        }
        // The outlooks it formed from its parties' own histories.
        for (who, about, level) in asked.formed {
            self.outlooks.form(who, about, level, self.week);
        }
        for (who, about, observed, memory) in asked.observed {
            self.outlooks
                .observe(who, about, observed, memory, self.week);
        }
        // And what ended — after the legs, because the last payment a relation owed is made under it
        // and not after it.
        for a in asked.ended {
            self.agreements.end(a, today);
        }
        for p in asked.closed {
            self.processes.finish(p);
        }
        // And the payments a seller agreed to wait for.
        for (q, until) in asked.on_terms {
            self.wire.queue.given_time(q, until);
        }
        // And whose life ended.
        for event in asked.ceased {
            let authority = self.destination_authority(event);
            // Open the legal identity before property is handed to an heir or put through a
            // workout: an asset must never lose its ordinary owner before its destination exists.
            self.parties.open_destination_for(
                event.who,
                event.to,
                authority,
                event.week,
                Some(crate::mechanisms::mortality::trigger_id(event.why)),
            );
            self.transfer_insured_deposits(event);
            self.convert_dues_to_claims(event.who);
            self.assert_liabilities_have_destination(event.who);
            self.transfer_to_heir(event, authority);
            self.open_liquidation(event);
            // Liquidation creates processes, so include those new workouts in the handoff.
            self.assign_employment(event.who, event.to, authority);
            self.assign_relationships(event.who, event.to, authority);
            self.assert_assets_have_destination(event, authority);
            self.parties.cease(event.who);
        }
        // And who is owed what by an estate.
        for (on, holder, owed, ranks) in asked.claimed {
            self.claims.against(on, holder, owed, ranks);
        }
        // What went ON the line and what came OFF it.
        for (owner, what, units, cost, ready) in asked.started {
            self.making
                .starts(owner, what, units, cost, self.week, ready);
        }
        for batch in asked.finished {
            self.making.finishes(batch);
        }
        // And what a party now stands behind.
        for (kind, who, about, terms) in asked.stood {
            let was = self.standing.of_party_about(who, about, kind);
            match was {
                Some(s) => {
                    self.standing.restates(s, &terms, self.week);
                }
                None => {
                    self.standing.stands(kind, who, about, &terms, self.week);
                }
            }
        }
        // And what came off one.
        for (claim, amount) in asked.repaid {
            self.claims.pays(claim, amount);
        }
        for (claim, amount) in asked.lost {
            // Audit B5, G2.b: what an exhausted estate did not pay lands on the named holder that
            // was owed it, in the week it was given up on.
            self.equity.moves(
                self.claims.holder_of(claim),
                -amount,
                crate::stores::Moved::Landed,
                self.week,
            );
            self.claims.loses(claim, amount);
        }
        1
    }

    fn apply_due_updates(&mut self) {
        for update in self.wire.take_due_updates() {
            let agreement = self.schedules.agreement(update.due);
            let before = self.schedules.state(update.due);
            self.schedules.apply(update);
            let after = self.schedules.state(update.due);
            if let Some(agreement) = agreement {
                match after {
                    crate::stores::DueState::Failed { on, .. } => {
                        self.agreements.records_performance(
                            agreement,
                            crate::stores::AgreementPerformance::Breached {
                                due: update.due,
                                on,
                            },
                        )
                    }
                    crate::stores::DueState::Settled { on }
                        if matches!(before, crate::stores::DueState::Failed { .. }) =>
                    {
                        self.agreements.records_performance(
                            agreement,
                            crate::stores::AgreementPerformance::Cured {
                                due: update.due,
                                on,
                            },
                        );
                    }
                    _ => {}
                }
                let all_paid = self
                    .schedules
                    .of_agreement(agreement)
                    .iter()
                    .all(|row| self.schedules.paid(crate::stores::DueId(*row)));
                if let crate::stores::DueState::Settled { on } = after {
                    let final_cure = matches!(before, crate::stores::DueState::Failed { .. });
                    let reached_end = self
                        .agreements
                        .until(agreement)
                        .is_some_and(|until| until <= on);
                    if all_paid && (final_cure || reached_end) {
                        self.agreements.discharge(agreement, on);
                    }
                }
            }
        }
    }

    /// Existing obligations survive cessation as claims on the same legal identity's estate or
    /// resolution state. Instrument dues follow title in the register at the instant authority
    /// transfers; bilateral dues retain their named beneficiary.
    fn convert_dues_to_claims(&mut self, estate: PartyId) {
        let dues = self.schedules.of_payer(estate).to_vec();
        for row in dues {
            let due = crate::stores::DueId(row);
            if self.schedules.paid(due) || self.schedules.claimed(due) {
                continue;
            }
            let outstanding = self.schedules.amount(due) - self.schedules.recovered(due);
            if outstanding <= 0.0 {
                continue;
            }
            let rank = match self.schedules.of(due) {
                crate::stores::Owing::Tax => 1,
                crate::stores::Owing::Wage => 1,
                crate::stores::Owing::Premium
                | crate::stores::Owing::Rent
                | crate::stores::Owing::Purchase => 3,
                crate::stores::Owing::Interest
                | crate::stores::Owing::Principal
                | crate::stores::Owing::Transfer
                | crate::stores::Owing::Call => 2,
            };
            let beneficiaries: Vec<(PartyId, f64)> = match self.schedules.on(due) {
                crate::stores::Owed::To(holder) if holder != estate => vec![(holder, outstanding)],
                crate::stores::Owed::To(_) => Vec::new(),
                crate::stores::Owed::On(line) => {
                    let held: Vec<(PartyId, f64)> = self
                        .register
                        .of_instrument(line)
                        .iter()
                        .map(|row| crate::ids::HoldingId(*row))
                        .map(|holding| {
                            (
                                self.register.holder_of(holding),
                                self.register.quantity(holding),
                            )
                        })
                        .filter(|(holder, quantity)| *holder != estate && *quantity > 0.0)
                        .collect();
                    let total: f64 = held.iter().map(|(_, quantity)| quantity).sum();
                    if total <= 0.0 {
                        Vec::new()
                    } else {
                        held.into_iter()
                            .map(|(holder, quantity)| (holder, outstanding * quantity / total))
                            .collect()
                    }
                }
            };
            for (holder, owed) in beneficiaries {
                self.claims.against(estate, holder, owed, rank);
            }
            // A liability without an external beneficiary is cancelled into the destination,
            // rather than left live on a party whose ordinary life is about to end.
            self.schedules.claim(due);
        }

        // A bank deposit is a liability even though it is represented by the depositor's holding
        // of the bank's money rather than by a future schedule row. It therefore enters resolution
        // as a preferential claim on the issuing bank, once, at its face amount.
        for row in 0..self.instruments.len() as u32 {
            let line = InstrumentId::at(row);
            if self.instruments.issuer_of(line) != estate
                || self.instruments.class_of(line) != Class::Money
            {
                continue;
            }
            for holding in self.register.of_instrument(line).to_vec() {
                let holding = crate::ids::HoldingId(holding);
                let holder = self.register.holder_of(holding);
                let owed = self.register.quantity(holding);
                if holder != estate && owed > 0.0 {
                    self.claims.against(estate, holder, owed, 1);
                }
            }
        }
    }

    fn transfer_insured_deposits(&mut self, event: crate::mechanisms::mortality::Ceased) {
        if event.to != crate::parties::Destination::Resolution
            || self.parties.kind_of(event.who) != kinds::BANK
        {
            return;
        }
        let region = self.parties.region_of(event.who);
        let successor = self
            .parties
            .of_kind(kinds::BANK)
            .iter()
            .map(|row| PartyId::at(*row))
            .find(|bank| {
                *bank != event.who
                    && self.parties.alive(*bank)
                    && self.parties.region_of(*bank) == region
            });
        let Some(successor) = successor else { return };
        let Some(old_money) = self.instruments.money_issued_by(event.who) else {
            return;
        };
        let new_money = self
            .instruments
            .money_issued_by(successor)
            .expect("a successor bank must issue the deposit money it assumes");
        let limit = self.params.price("deposit.insurance.limit_per_member");
        for row in self.register.of_instrument(old_money).to_vec() {
            let holding = crate::ids::HoldingId(row);
            let depositor = self.register.holder_of(holding);
            // Reserve and settlement holdings of the same money are not customer deposits and do
            // not move the holder's banking relationship to the resolution successor.
            if depositor == event.who || self.parties.bank_of(depositor) != event.who {
                continue;
            }
            let balance = self.register.quantity(holding);
            let covered = crate::mechanisms::bank_capital::insured(
                balance,
                f64::from(self.parties.weight(depositor)),
                limit,
            );
            let Some(amount) = crate::ledger::Units::new(covered) else {
                continue;
            };
            self.parties.moves_bank(depositor, event.who, successor);
            let legs = [
                crate::ledger::Leg::Mint {
                    issuer: successor,
                    money: new_money,
                    amount,
                },
                crate::ledger::Leg::Money {
                    from: successor,
                    to: depositor,
                    instrument: new_money,
                    amount,
                    receipt: crate::ledger::Receipt::Transfer,
                },
                crate::ledger::Leg::Destroy {
                    party: depositor,
                    instrument: old_money,
                    qty: amount,
                    why: crate::ledger::Gone::Redeemed,
                },
            ];
            let outcome = self.wire.settle(
                &Instruction::against_payment(&legs, crate::ledger::Cause::CorporateAction),
                event.week,
                &mut Settling {
                    register: &mut self.register,
                    journal: &mut self.journal,
                    parties: &self.parties,
                    instruments: &mut self.instruments,
                    calendar: &self.calendar,
                    says: self.says,
                    equity: &mut self.equity,
                },
            );
            assert_eq!(
                outcome,
                crate::ledger::Outcome::Settled,
                "an insured deposit transfer must settle atomically"
            );
        }
    }

    fn assert_liabilities_have_destination(&self, estate: PartyId) {
        for row in self.schedules.of_payer(estate) {
            let due = crate::stores::DueId(*row);
            assert!(
                self.schedules.paid(due) || self.schedules.claimed(due),
                "every outstanding due must enter the legal destination"
            );
        }
        for row in 0..self.instruments.len() as u32 {
            let line = InstrumentId::at(row);
            if self.instruments.issuer_of(line) != estate
                || self.instruments.class_of(line) != Class::Money
            {
                continue;
            }
            for row in self.register.of_instrument(line) {
                let holding = crate::ids::HoldingId(*row);
                let holder = self.register.holder_of(holding);
                let owed = self.register.quantity(holding);
                if holder == estate || owed <= 0.0 {
                    continue;
                }
                let routed = self.claims.on_estate(estate).iter().any(|row| {
                    let claim = crate::stores::ClaimId(*row);
                    let scale = if owed >= 1.0 { owed } else { 1.0 };
                    self.claims.holder_of(claim) == holder
                        && self.claims.ranks(claim) == 1
                        && (self.claims.owed(claim) - owed).abs() <= f64::EPSILON * scale
                });
                assert!(
                    routed,
                    "every deposit liability must become a destination claim"
                );
            }
        }
    }

    fn open_liquidation(&mut self, event: crate::mechanisms::mortality::Ceased) {
        use crate::mechanisms::forced_sale::Door;
        use crate::parties::Destination;
        let door = match event.to {
            Destination::Estate => Some(Door::Estate),
            Destination::Resolution => Some(Door::Resolution),
            Destination::Heir => None,
        };
        let Some(door) = door else { return };
        let positions: Vec<(InstrumentId, f64)> = self
            .register
            .of_holder(event.who)
            .iter()
            .map(|row| crate::ids::HoldingId(*row))
            .filter(|holding| {
                self.instruments
                    .class_of(self.register.instrument_of(*holding))
                    != Class::Money
            })
            .filter_map(|holding| {
                let quantity = self.register.free(holding);
                (quantity > 0.0).then_some((self.register.instrument_of(holding), quantity))
            })
            .collect();
        for (line, size) in positions {
            self.processes.begin_for(
                crate::stores::afoot::WORKOUT,
                event.who,
                event.week,
                None,
                size,
                crate::stores::ProcessTarget {
                    door: Some(door as u32),
                    subject: Some(line),
                },
            );
        }
    }

    fn assert_assets_have_destination(
        &self,
        event: crate::mechanisms::mortality::Ceased,
        authority: PartyId,
    ) {
        assert_eq!(self.parties.destination_of(event.who), Some(event.to));
        assert_eq!(self.parties.authority_of(event.who), Some(authority));
        for row in self.register.of_holder(event.who) {
            let holding = crate::ids::HoldingId(*row);
            let line = self.register.instrument_of(holding);
            let free = self.register.free(holding);
            if free <= 0.0 || self.instruments.class_of(line) == Class::Money {
                continue;
            }
            match event.to {
                crate::parties::Destination::Heir => {
                    assert_eq!(
                        free, 0.0,
                        "a free inherited asset must reach its named heir"
                    );
                }
                crate::parties::Destination::Estate | crate::parties::Destination::Resolution => {
                    let routed = self.processes.of_owner(event.who).iter().any(|row| {
                        let process = crate::stores::ProcessId(*row);
                        !self.processes.done(process)
                            && self.processes.subject(process) == Some(line)
                            && self.processes.destination(process) == Some(event.to)
                    });
                    assert!(
                        routed,
                        "every free non-money asset needs a destination workout"
                    );
                }
            }
        }
    }

    fn assign_relationships(
        &mut self,
        party: PartyId,
        to: crate::parties::Destination,
        authority: PartyId,
    ) {
        let agreements = self.agreements.of_party(party).to_vec();
        for row in agreements {
            let agreement = crate::stores::AgreementId(row);
            if self.agreements.kind_of(agreement) == crate::stores::agreed::ENGAGEMENT {
                continue;
            }
            if self.agreements.live(agreement) && self.agreements.destination(agreement).is_none() {
                self.agreements.enters_destination(agreement, to);
                if authority != party {
                    self.agreements.moves(agreement, party, authority);
                }
            }
            if self.agreements.live(agreement) {
                assert_eq!(self.agreements.destination(agreement), Some(to));
                let (one, other) = self.agreements.between(agreement);
                assert!(
                    one == authority || other == authority,
                    "a live contract must name its destination authority"
                );
            }
        }
        let processes = self.processes.of_owner(party).to_vec();
        for row in processes {
            let process = crate::stores::ProcessId(row);
            if !self.processes.done(process) && self.processes.destination(process).is_none() {
                self.processes.enters_destination(process, to);
                if authority != party {
                    self.processes.moves(process, authority);
                }
            }
        }
    }

    fn assign_employment(
        &mut self,
        party: PartyId,
        to: crate::parties::Destination,
        authority: PartyId,
    ) {
        let agreements = self.agreements.of_party(party).to_vec();
        for row in agreements {
            let agreement = crate::stores::AgreementId(row);
            if self.agreements.kind_of(agreement) != crate::stores::agreed::ENGAGEMENT
                || !self.agreements.live(agreement)
            {
                continue;
            }
            if self.agreements.destination(agreement).is_none() {
                self.agreements.enters_destination(agreement, to);
                if authority != party {
                    self.agreements.moves(agreement, party, authority);
                }
            }
            assert_eq!(self.agreements.destination(agreement), Some(to));
            let (one, other) = self.agreements.between(agreement);
            assert!(
                one == authority || other == authority,
                "a live employment agreement must name its destination authority"
            );
        }
    }

    fn destination_authority(&self, event: crate::mechanisms::mortality::Ceased) -> PartyId {
        if event.to != crate::parties::Destination::Heir {
            return event.who;
        }
        let kind = self.parties.kind_of(event.who);
        let region = self.parties.region_of(event.who);
        self.parties
            .of_kind(kind)
            .iter()
            .map(|row| PartyId::at(*row))
            .find(|party| {
                *party != event.who
                    && self.parties.alive(*party)
                    && self.parties.region_of(*party) == region
            })
            .expect("a household heir path requires a named living cell in its region")
    }

    fn transfer_to_heir(&mut self, event: crate::mechanisms::mortality::Ceased, heir: PartyId) {
        if event.to != crate::parties::Destination::Heir {
            return;
        }
        let mut legs = Vec::new();
        for row in self.register.of_holder(event.who).to_vec() {
            let holding = crate::ids::HoldingId(row);
            let line = self.register.instrument_of(holding);
            let Some(amount) = crate::ledger::Units::new(self.register.free(holding)) else {
                continue;
            };
            if self.instruments.class_of(line) == Class::Money {
                legs.push(crate::ledger::Leg::Money {
                    from: event.who,
                    to: heir,
                    instrument: line,
                    amount,
                    receipt: crate::ledger::Receipt::Transfer,
                });
            } else {
                // The heir's position is its own, so what it has already said about this line
                // stands and arriving units do not restate it.
                if self
                    .register
                    .carrying(self.register.row(heir, line))
                    .is_none()
                {
                    self.register
                        .carry(heir, line, declared_on(&self.register, holding));
                }
                legs.push(crate::ledger::Leg::Asset {
                    from: event.who,
                    to: heir,
                    instrument: line,
                    qty: amount,
                    price_per_unit: None,
                });
            }
        }
        if legs.is_empty() {
            return;
        }
        let instruction = Instruction {
            legs: &legs,
            cause: crate::ledger::Cause::CorporateAction,
            delivery: crate::ledger::Delivery::Free,
            due: None,
        };
        self.wire.settle(
            &instruction,
            event.week,
            &mut Settling {
                register: &mut self.register,
                journal: &mut self.journal,
                parties: &self.parties,
                instruments: &mut self.instruments,
                calendar: &self.calendar,
                says: self.says,
                equity: &mut self.equity,
            },
        );
    }

    /// The markets moment: every declared book, asked once.
    fn collect_orders(
        &mut self,
        participants: &[&dyn Participant],
        out: &mut Stepped,
    ) -> Vec<crate::session::Posted> {
        if participants.is_empty() || self.books.is_empty() {
            return Vec::new();
        }
        let books = Books::index(
            participants,
            &Shown {
                registry: &self.registry,
                valuers: &self.valuers,
                parties: &self.parties,
                instruments: &mut self.instruments,
                register: &self.register,
                prints: &self.prints,
                journal: &self.journal,
                params: &self.params,
                outlooks: &self.outlooks,
                agreements: &self.agreements,
                schedules: &self.schedules,
                resting: &self.resting,
                processes: &self.processes,
                standing: &self.standing,
                calendar: &self.calendar,
                books: &self.books,
            },
            self.week,
        );
        out.narrows += books.narrows;
        let mut posted = Vec::with_capacity(self.books.len());
        for book in &self.books {
            let mut stores = Stores {
                valuers: &self.valuers,
                parties: &self.parties,
                instruments: &mut self.instruments,
                register: &mut self.register,
                prints: &mut self.prints,
                journal: &mut self.journal,
                wire: &mut self.wire,
                params: &self.params,
                outlooks: &self.outlooks,
                agreements: &self.agreements,
                schedules: &self.schedules,
                resting: &mut self.resting,
                processes: &mut self.processes,
                standing: &self.standing,
                registry: &self.registry,
                calendar: &self.calendar,
                books: &self.books,
                equity: &mut self.equity,
            };
            let said = crate::session::ask_book(book, participants, &books, &mut stores, self.week);
            out.asks += said.asks;
            posted.push(said);
        }
        posted
    }

    /// STAGE f — the books clear, once, on what was posted into them (Money G2.f).
    fn run_books(&mut self, posted: &[crate::session::Posted], out: &mut Stepped) -> usize {
        let mut traded = 0usize;
        for (book, said) in self.books.iter().zip(posted) {
            let mut stores = Stores {
                valuers: &self.valuers,
                parties: &self.parties,
                instruments: &mut self.instruments,
                register: &mut self.register,
                prints: &mut self.prints,
                journal: &mut self.journal,
                wire: &mut self.wire,
                params: &self.params,
                outlooks: &self.outlooks,
                agreements: &self.agreements,
                schedules: &self.schedules,
                resting: &mut self.resting,
                processes: &mut self.processes,
                standing: &self.standing,
                registry: &self.registry,
                calendar: &self.calendar,
                books: &self.books,
                equity: &mut self.equity,
            };
            let session = run_book(book, said, &mut stores, self.week, self.says);
            traded += session.settled;
            // A book that had something cross is one that cleared; one that did not prints nothing,
            // and counting it would be a session that never happened.
            if matches!(&session.outcome, crate::clearing::Outcome::Cleared { .. }) {
                out.books_cleared += 1;
            }
            self.sessions.push(session);
        }
        traded
    }

    /// 49 C1: a country is ground with a money. The registry writes what it is paid in and
    /// geography writes where it is, so neither holds the other's fact.
    pub fn admit_country(&mut self, ccy: crate::ids::CurrencyCode) -> crate::ids::CountryId {
        let country = self.registry.country(ccy);
        self.geography.declare_country(country);
        country
    }

    /// 49 C2: and a region is ground under it — the first land nobody has claimed.
    pub fn admit_region(&mut self, country: crate::ids::CountryId) -> crate::ids::RegionId {
        let tile = self
            .geography
            .unclaimed_land()
            .expect("49 C2: a region needs ground nobody has claimed");
        self.geography
            .region(country, tile)
            .expect("49 C2: unclaimed land takes a region")
    }

    /// A PARTY IS ADMITTED TO A WORLD, AND ITS BANK HAS TO ISSUE MONEY.
    pub fn admit<K: Into<crate::parties::LatticeKey>>(
        &mut self,
        kind: u32,
        region: crate::ids::RegionId,
        bank: PartyId,
        representation: crate::parties::Representation,
        key: K,
    ) -> PartyId {
        let key = key.into();
        assert!(
            kind != kinds::HOUSEHOLD || matches!(&key, crate::parties::LatticeKey::Household(_)),
            "XI-15: a household cell needs a household lattice coordinate"
        );
        assert!(
            kind != kinds::SMALL_FIRM
                || matches!(&key, crate::parties::LatticeKey::SmallBusiness(_)),
            "XI-15: an SME cell needs a small-business lattice coordinate"
        );
        // The kernel asks the kind's PROFILE.
        match self.registry.profile(kind).map(|p| p.banks) {
            Some(Banks::Nowhere) => assert!(
                !bank.some(),
                "Money D2: a party of kind {kind} issues the money others settle in, so it banks nowhere"
            ),
            Some(_) => assert!(
                bank.some() && self.instruments.money_issued_by(bank).is_some(),
                "Money D2: party of kind {kind} banks at {}, which issues no money — it would hold nothing it could pay with",
                bank.0
            ),
            None => assert!(
                !bank.some() || self.instruments.money_issued_by(bank).is_some(),
                "Money D2: party of kind {kind} banks at {}, which issues no money — it would hold nothing it could pay with",
                bank.0
            ),
        }
        // XI-15: a weight arrives by ENTRY, and an entry names the row that says it happened —
        // otherwise a population is a number somebody set.
        let party = match (representation, &key) {
            (crate::parties::Representation::Cell(weight), _) => {
                let said = self.journal.say(
                    self.week,
                    self.says.population,
                    &[],
                    &[(0, crate::journal::Value::Num(f64::from(weight.get())))],
                    true,
                );
                match key.clone() {
                    crate::parties::LatticeKey::Household(at) => self
                        .parties
                        .enter_household(kind, region, bank, weight, at, said),
                    crate::parties::LatticeKey::SmallBusiness(at) => self
                        .parties
                        .enter_small_business(kind, region, bank, weight, at, said),
                    crate::parties::LatticeKey::Named(_) => {
                        panic!("XI-15: a population cell needs a declared lattice key")
                    }
                }
            }
            _ => self.parties.add(kind, region, bank, representation, key),
        };
        // 49 C4: and it stands somewhere exact, so its country is read through the ground.
        self.geography
            .stand(party, region, crate::geography::SiteKind::Party)
            .expect("49 C4: a party is admitted onto ground its region holds");
        if kind == kinds::HOUSEHOLD
            && self.params.declared("household.keeps.from")
            && self.params.declared("household.keeps.to")
            && self.params.declared("household.will_spend.from")
            && self.params.declared("household.will_spend.to")
        {
            let from = self
                .params
                .amount("household.keeps.from", crate::params::Denomination::Money);
            let to = self
                .params
                .amount("household.keeps.to", crate::params::Denomination::Money);
            let spend_from = self.params.ratio("household.will_spend.from");
            let spend_to = self.params.ratio("household.will_spend.to");
            self.parties
                .assign_household_preferences(party, from, to, spend_from, spend_to);
        }
        party
    }

    pub fn open_book(
        &mut self,
        market: MarketId,
        subject: InstrumentId,
        ccy: CurrencyCode,
        venue: crate::protocols::Venue,
    ) {
        // A book names a CURRENCY and each side pays out of its own account, so there is no cash
        // line to check the class of.
        assert!(
            self.instruments.class_of(subject) != Class::Money,
            "Clearing B1: a book's subject is what it delivers, and money is not delivered in a book"
        );
        assert!(
            !self.books.iter().any(|book| book.market == market),
            "Law 4: market {} is declared twice",
            market.0
        );
        assert!(
            !self.books.iter().any(|book| book.subject == subject),
            "Law 19: instrument {} already has a book",
            subject.0
        );
        self.books.push(BookDecl {
            market,
            subject,
            ccy,
            venue,
        });
    }
}

/// Which system owns each declaration slot, so a phase in the order can be traced back to the system
/// that declared it.
/// What a position that is being handed on was declared as. A holding with units was declared
/// when it was acquired, so an absence here is a position the register should have refused.
fn declared_on(
    register: &crate::register::Register,
    row: crate::ids::HoldingId,
) -> crate::register::Carrying {
    register
        .carrying(row)
        .expect("XI-6: a position holding units said what it was held FOR when it was acquired")
}

fn slots(systems: &[&dyn System]) -> Vec<usize> {
    let mut by_slot = Vec::new();
    for (at, s) in systems.iter().enumerate() {
        for p in s.phases() {
            while by_slot.len() <= p.owner as usize {
                by_slot.push(usize::MAX);
            }
            by_slot[p.owner as usize] = at;
        }
    }
    by_slot
}

/// A phase: where it runs, what it needs of the week it runs in, and what it puts into that week.
pub fn phase(name: u32, owner: u32, at: u32, needs: &[Produces], makes: &[Produces]) -> PhaseDecl {
    PhaseDecl {
        name,
        owner,
        at,
        reads: needs.to_vec(),
        writes: makes.to_vec(),
    }
}

/// The thirty-one slots, re-exported so a system says which one it runs in without importing the
/// world. A system names a SLOT, because a stage is a group of them and "somewhere in b" is not an
/// order.
pub const AT_B2: u32 = crate::world::B2;
pub const AT_B3: u32 = crate::world::B3;
pub const AT_B4: u32 = crate::world::B4;
pub const AT_B5: u32 = crate::world::B5;
pub const AT_D1: u32 = crate::world::D1;
pub const AT_D3: u32 = crate::world::D3;
pub const AT_D4: u32 = crate::world::D4;
pub const AT_D5: u32 = crate::world::D5;
pub const AT_D6: u32 = crate::world::D6;
pub const AT_E1: u32 = crate::world::E1;
pub const AT_G1: u32 = crate::world::G1;
pub const AT_G2: u32 = crate::world::G2;
pub const AT_G3: u32 = crate::world::G3;
pub const AT_G4: u32 = crate::world::G4;
pub const AT_G5: u32 = crate::world::G5;
pub const AT_G6: u32 = crate::world::G6;
pub const AT_G7: u32 = crate::world::G7;
pub const AT_H: u32 = crate::world::H;

/// What a party will pay or take for what it already holds, read from its own book.
pub fn holds_of(view: &ParticipantView<'_>, subject: InstrumentId) -> f64 {
    view.free(subject)
}

/// Its own money, for what it can actually fund — Law 6: a bid it cannot pay for is not a bid.
pub fn can_fund(view: &ParticipantView<'_>, cash: InstrumentId) -> f64 {
    view.free(cash)
}

/// A seller's order, from what it holds and what it will take.
pub fn offer(view: &ParticipantView<'_>, subject: InstrumentId, at_least: f64) -> Option<Order> {
    let units = holds_of(view, subject);
    if units <= 0.0 {
        return None;
    }
    Some(Order {
        party: view.self_id(),
        side: Side::Sell,
        price: Some(at_least),
        qty: units as i64,
    })
}

/// A buyer's order, from what it can fund and what it will pay.
pub fn bid(view: &ParticipantView<'_>, cash: InstrumentId, at_most: f64) -> Option<Order> {
    if at_most <= 0.0 {
        return None;
    }
    let money = can_fund(view, cash);
    let affordable = (money / at_most) as i64;
    if affordable <= 0 {
        return None;
    }
    Some(Order {
        party: view.self_id(),
        side: Side::Buy,
        price: Some(at_most),
        qty: affordable,
    })
}

// THE ASSEMBLED WORLD HAS NO FIXTURE, because `world:runs` steps the real one four weeks every
// time it is run: 51 systems, 54 phases, 58,354 asks, 1.9M events, and the audit reporting by
// family with an owner and a size. Sixteen tests here built three or four parties and asked
// whether a module can strike a relation, whether a world with no books steps, whether a party
// short of money brings paper, whether a wired system's proposal is settled — each of which the
// assembled world answers at scale, arranged by nobody.
//
// What the kernel REFUSES it refuses at the site: a party admitted at a bank that issues no money,
// a book that delivers money, a phase that anchors to nothing, a system wired twice, a system that
// neither works nor posts. Each panics with its clause and none needs a world to state.
//
// The homeless-noun count is a CENSUS, printed by `world:runs` every run. The fixture that guarded
// it had become a list of items already closed — `registry.currencies has a home now` — which is
// history, and the live claim is the one in CLAUDE.md: a count of zero would be the measure
// switched off.
//
// What is left here that a family should ask instead: a party whose liabilities exceed its assets
// ceases, and the one exception is a consequence rather than a rule. The Accounts family asks it
// of every party every week.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_records_the_seed_and_kernel_resolutions() {
        let config = RunConfig {
            seed: 42,
            payment_wait_weeks: 3,
            money_pieces_per_unit: 1_000.0,
            time_pieces_per_unit: 4.0,
            ..RunConfig::default()
        };
        let world = World::with_parameters(config, |params| {
            params.declare(crate::params::ParamDecl {
                id: "test.preference".to_string(),
                value: 0.25,
                unit: "share".to_string(),
                dimension: crate::params::Dimension::Ratio,
                kind: crate::params::Kind::Preference,
                owner: None,
                why: "exercise the configured parameter set".to_string(),
            });
        });

        assert_eq!(world.config, config);
        assert_eq!(world.wire.waits_for(), 3);
        assert_eq!(world.params.ratio("test.preference"), 0.25);
    }

    #[test]
    fn construction_refuses_an_invalid_resolution_before_state_exists() {
        let config = RunConfig {
            money_pieces_per_unit: f64::NAN,
            ..RunConfig::default()
        };
        assert!(std::panic::catch_unwind(|| World::with_config(config)).is_err());
    }

    #[test]
    fn cessation_opens_the_legal_destination_before_routing_each_asset() {
        use crate::parties::{Destination, Representation};

        let mut world = World::with_config(RunConfig::default());
        let estate = world.parties.add(
            kinds::FIRM,
            crate::ids::RegionId::at(0),
            PartyId::NONE,
            Representation::Named,
            1,
        );
        let asset = world.instruments.issue(
            estate,
            CurrencyCode::at(0),
            Class::Share,
            crate::ids::UnitId::at(0),
            None,
            None,
        );
        world
            .register
            .carry(estate, asset, crate::register::Carrying::Cost);
        world.register.credit(estate, asset, 3.0, 12.0, 0);
        let event = crate::mechanisms::mortality::Ceased {
            who: estate,
            why: crate::mechanisms::mortality::Trigger::LiabilitiesExceedAssets,
            to: Destination::Estate,
            week: 4,
        };

        world
            .parties
            .open_destination_for(estate, event.to, estate, event.week, None);
        world.open_liquidation(event);
        world.assign_relationships(estate, event.to, estate);
        world.assert_assets_have_destination(event, estate);

        let workout = crate::stores::ProcessId(world.processes.of_owner(estate)[0]);
        assert_eq!(world.processes.subject(workout), Some(asset));
        assert_eq!(
            world.processes.destination(workout),
            Some(Destination::Estate)
        );
        world.parties.cease(estate);
        assert!(!world.parties.alive(estate));
    }

    #[test]
    fn cessation_converts_each_deposit_liability_into_a_destination_claim() {
        use crate::parties::Representation;

        let mut world = World::with_config(RunConfig::default());
        let bank = world.parties.add(
            kinds::BANK,
            crate::ids::RegionId::at(0),
            PartyId::NONE,
            Representation::Named,
            1,
        );
        let depositor = world.parties.add(
            kinds::FIRM,
            crate::ids::RegionId::at(0),
            bank,
            Representation::Named,
            2,
        );
        let deposit = world.instruments.issue(
            bank,
            CurrencyCode::at(0),
            Class::Money,
            crate::ids::UnitId::at(0),
            None,
            None,
        );
        world
            .register
            .carry(depositor, deposit, crate::register::Carrying::Cost);
        world.register.credit(depositor, deposit, 75.0, 1.0, 0);

        world.convert_dues_to_claims(bank);
        world.assert_liabilities_have_destination(bank);

        let claim = crate::stores::ClaimId(world.claims.on_estate(bank)[0]);
        assert_eq!(world.claims.holder_of(claim), depositor);
        assert_eq!(world.claims.owed(claim), 75.0);
        assert_eq!(world.claims.ranks(claim), 1);
    }

    #[test]
    fn cessation_moves_employment_to_the_named_destination_authority() {
        use crate::parties::Destination;

        let mut world = World::with_config(RunConfig::default());
        let employer = PartyId::at(1);
        let worker = PartyId::at(2);
        let heir = PartyId::at(3);
        let engagement = world.agreements.strike(
            crate::stores::agreed::ENGAGEMENT,
            employer,
            worker,
            crate::stores::AgreementTerms::Engagement {
                wage_per_person: 40.0,
                hours_per_person: 8.0,
                heads: 1,
            },
            crate::calendar::Week(0),
            None,
        );

        world.assign_employment(employer, Destination::Heir, heir);

        assert_eq!(
            world.agreements.destination(engagement),
            Some(Destination::Heir)
        );
        assert_eq!(world.agreements.between(engagement), (heir, worker));
    }

    #[test]
    fn cessation_moves_every_other_live_contract_to_the_destination_authority() {
        use crate::parties::Destination;

        let mut world = World::with_config(RunConfig::default());
        let borrower = PartyId::at(1);
        let lender = PartyId::at(2);
        let successor = PartyId::at(3);
        let mortgage = world.agreements.strike(
            crate::stores::agreed::MORTGAGE,
            borrower,
            lender,
            crate::stores::AgreementTerms::Mortgage {
                purchase_price: 250.0,
                deposit_share: 0.2,
            },
            crate::calendar::Week(0),
            None,
        );

        world.assign_relationships(lender, Destination::Resolution, successor);

        assert_eq!(
            world.agreements.destination(mortgage),
            Some(Destination::Resolution)
        );
        assert_eq!(world.agreements.between(mortgage), (borrower, successor));
    }

    #[test]
    fn resolution_moves_the_insured_deposit_to_a_live_successor_bank() {
        use crate::parties::{Destination, Representation};

        let mut world = World::with_config(RunConfig::default());
        let failed = world.parties.add(
            kinds::BANK,
            crate::ids::RegionId::at(0),
            PartyId::NONE,
            Representation::Named,
            1,
        );
        let successor = world.parties.add(
            kinds::BANK,
            crate::ids::RegionId::at(0),
            PartyId::NONE,
            Representation::Named,
            2,
        );
        let depositor = world.parties.add(
            kinds::FIRM,
            crate::ids::RegionId::at(0),
            failed,
            Representation::Named,
            3,
        );
        let old_money = world.instruments.issue(
            failed,
            CurrencyCode::at(0),
            Class::Money,
            crate::ids::UnitId::at(0),
            None,
            None,
        );
        let new_money = world.instruments.issue(
            successor,
            CurrencyCode::at(0),
            Class::Money,
            crate::ids::UnitId::at(0),
            None,
            None,
        );
        let amount = crate::ledger::Units::new(40.0).unwrap();
        let legs = [
            crate::ledger::Leg::Mint {
                issuer: failed,
                money: old_money,
                amount,
            },
            crate::ledger::Leg::Money {
                from: failed,
                to: depositor,
                instrument: old_money,
                amount,
                receipt: crate::ledger::Receipt::Transfer,
            },
        ];
        world.wire.settle(
            &Instruction::plain(&legs, crate::ledger::Cause::CorporateAction),
            0,
            &mut Settling {
                register: &mut world.register,
                journal: &mut world.journal,
                parties: &world.parties,
                instruments: &mut world.instruments,
                calendar: &world.calendar,
                says: world.says,
                equity: &mut world.equity,
            },
        );

        world.transfer_insured_deposits(crate::mechanisms::mortality::Ceased {
            who: failed,
            why: crate::mechanisms::mortality::Trigger::CapitalGone,
            to: Destination::Resolution,
            week: 1,
        });

        assert_eq!(world.parties.bank_of(depositor), successor);
        assert_eq!(
            world
                .register
                .quantity(world.register.row(depositor, old_money)),
            0.0
        );
        assert_eq!(
            world
                .register
                .quantity(world.register.row(depositor, new_money)),
            40.0
        );
    }

    #[test]
    fn zero_coupon_paper_still_owes_its_principal() {
        let mut world = World::with_config(RunConfig::default());
        world.brought(crate::module::Brings {
            issuer: PartyId::at(0),
            initial_holder: Some(PartyId::at(2)),
            loan_terms: None,
            issue_price: Some(100.0),
            ccy: crate::ids::CurrencyCode::at(0),
            class: Class::Claim,
            unit: crate::ids::UnitId::at(0),
            coupon: None,
            matures: Some(crate::calendar::Week(365)),
            pays: crate::instruments::PaymentFrequency::AtMaturity,
            convention: crate::calendar::Convention::Actual365,
            units: 100.0,
            carried_as: crate::register::Carrying::Cost,
            book: None,
        });

        assert_eq!(world.schedules.len(), 1);
        let claim = InstrumentId::at(0);
        assert_eq!(
            world
                .register
                .quantity(world.register.row(PartyId::at(2), claim)),
            100.0
        );
        assert_eq!(world.instruments.issuer_of(claim), PartyId::at(0));
        let due = crate::stores::DueId(0);
        assert_eq!(world.schedules.of(due), crate::stores::Owing::Principal);
        assert_eq!(world.schedules.amount(due), 100.0);
    }

    #[test]
    fn a_bilateral_loans_negotiated_terms_are_stored_on_its_instrument_row() {
        let mut world = World::with_config(RunConfig::default());
        world.brought(crate::module::Brings::loan_claim(
            PartyId::at(0),
            PartyId::at(2),
            crate::ids::CurrencyCode::at(0),
            crate::instruments::LoanTerms {
                amount: 275.0,
                tenor: 104,
                covenant: crate::instruments::LoanCovenant::LoanToValue { maximum: 0.8 },
                collateral: Some(InstrumentId::at(7)),
            },
            240.0,
        ));

        let loan = InstrumentId::at(0);
        assert_eq!(world.instruments.negotiated_amount_of(loan), Some(275.0));
        assert_eq!(world.instruments.negotiated_tenor_of(loan), Some(104));
        assert_eq!(
            world.instruments.negotiated_covenant_of(loan),
            Some(crate::instruments::LoanCovenant::LoanToValue { maximum: 0.8 })
        );
        assert_eq!(
            world.instruments.collateral_of(loan),
            Some(InstrumentId::at(7))
        );
        assert_eq!(world.instruments.issued_of(loan), 1.0);
        assert_eq!(
            world
                .register
                .quantity(world.register.row(PartyId::at(2), loan)),
            1.0
        );
        assert_eq!(
            world
                .register
                .lots(world.register.row(PartyId::at(2), loan))[0]
                .basis_per_unit,
            240.0
        );
        assert_eq!(world.schedules.len(), 1);
        let due = crate::stores::DueId(0);
        assert_eq!(world.schedules.on(due), crate::stores::Owed::On(loan));
        assert_eq!(world.schedules.amount(due), 275.0);
        assert_eq!(
            world.schedules.due(due),
            world.calendar.at(crate::calendar::Week(104))
        );
    }

    #[test]
    fn fixed_coupon_paper_generates_its_dated_schedule_from_issued_terms() {
        let mut world = World::with_config(RunConfig::default());
        let issued = world.calendar.at(crate::calendar::Week(0));
        world.brought(crate::module::Brings {
            issuer: PartyId::at(0),
            initial_holder: None,
            loan_terms: None,
            issue_price: None,
            ccy: crate::ids::CurrencyCode::at(0),
            class: Class::Claim,
            unit: crate::ids::UnitId::at(0),
            coupon: Some(0.04),
            matures: Some(issued.after(52)),
            pays: crate::instruments::PaymentFrequency::SemiAnnual,
            convention: crate::calendar::Convention::Actual365,
            units: 100.0,
            carried_as: crate::register::Carrying::Cost,
            book: None,
        });

        let interest = (0..world.schedules.len())
            .map(|row| crate::stores::DueId(row as u32))
            .filter(|due| world.schedules.of(*due) == crate::stores::Owing::Interest)
            .collect::<Vec<_>>();
        assert_eq!(interest.len(), 2);
        assert_eq!(world.schedules.from(interest[0]), issued);
        assert_eq!(world.schedules.due(interest[0]), issued.after(26));
        assert_eq!(world.schedules.due(interest[1]), issued.after(52));
        assert_eq!(
            (0..world.schedules.len())
                .map(|row| crate::stores::DueId(row as u32))
                .filter(|due| world.schedules.of(*due) == crate::stores::Owing::Principal)
                .count(),
            1
        );
    }
}
