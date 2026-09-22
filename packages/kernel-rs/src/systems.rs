//! THE SYSTEMS, WIRED. Every spec system this world has, in one list, with the participants that
//! have a reason to post and the phases that say when they run.
//!
//! @spec 3 B2 · 37 C1 · 39 · 11 B1 · 10 B1 · 7 D1 · 40 B1 · 26 A2 · ARCHITECTURE 4.9b · Law 3,
//! @spec Law 6, Law 15, Law 19 · Appendix B

use crate::assembly::{
    kinds, phase, System, AT_B2, AT_B3, AT_B4, AT_B5, AT_D1, AT_D2, AT_D3, AT_D4, AT_D5, AT_D6,
    AT_E1, AT_G1, AT_G2, AT_G3, AT_G4, AT_G5, AT_G6, AT_G7, AT_H,
};
use crate::ids::book_of;
use crate::ids::InstrumentId;
use crate::mechanisms::bank_capital::BankCapital;
use crate::mechanisms::bank_funding::BankFunding;
use crate::mechanisms::benchmarks::{Fixes, PublishedIndices};
use crate::mechanisms::capital_programme::{Builder, Building};
use crate::mechanisms::cds::Protection;
use crate::mechanisms::commodities::Storing;
use crate::mechanisms::control::Control;
use crate::mechanisms::corporate_credit::Brings as BringsBond;
use crate::mechanisms::cost_of_capital::CostOfCapital;
use crate::mechanisms::cross_border::CrossBorder;
use crate::mechanisms::dealing::Dealers;
use crate::mechanisms::dealing::Lines;
use crate::mechanisms::derivative_layer::Derivatives;
use crate::mechanisms::employment::Wages;
use crate::mechanisms::equity::{Floating, Flotation};
use crate::mechanisms::estate::Ranked;
use crate::mechanisms::expectations::Forming;
use crate::mechanisms::firms::Reporting;
use crate::mechanisms::forced_sale::ForcedSeller;
use crate::mechanisms::forced_sale::ForcedSelling;
use crate::mechanisms::freight::OffersItsRoom;
use crate::mechanisms::funds::FundMandates;
use crate::mechanisms::funds::Winding;
use crate::mechanisms::fx_forwards::FxForwards;
use crate::mechanisms::goods::CostFlow;
use crate::mechanisms::goods::Making;
use crate::mechanisms::goods::{GoodsSellers, Stockist};
use crate::mechanisms::hedge_funds::Levering;
use crate::mechanisms::hedge_funds::Liquidity;
use crate::mechanisms::households::HouseholdBuyers;
use crate::mechanisms::housing::Housing;
use crate::mechanisms::insurers::InsurerMatching;
use crate::mechanisms::insurers::Policies;
use crate::mechanisms::lending::Servicing;
use crate::mechanisms::loss::Losses;
use crate::mechanisms::money_market::Interbank;
use crate::mechanisms::money_market::MoneyMarketBanks;
use crate::mechanisms::mortality::Failing;
use crate::mechanisms::observer::Observing;
use crate::mechanisms::polity::Elections;
use crate::mechanisms::prime_brokerage::Broking;
use crate::mechanisms::private_equity::Calling;
use crate::mechanisms::ratings::Grading;
use crate::mechanisms::redeemable::Subscribing;
use crate::mechanisms::reporting::Publishes;
use crate::mechanisms::second_opinion::SecondOpinion;
use crate::mechanisms::securities_lending::StockLending;
use crate::mechanisms::securitisation::Securitising;
use crate::mechanisms::short_term_debt::Brings as BringsPaper;
use crate::mechanisms::small_business::SmallBusiness;
use crate::mechanisms::sovereign::{PrimaryDealers, Sovereign};
use crate::mechanisms::spot_fx::SpotFx;
use crate::mechanisms::trade_credit::TradeCredit;
use crate::mechanisms::treasury::Funding;
use crate::mechanisms::treasury::TreasuryIssues;
use crate::module::{Mechanism, Participant};
use crate::params::{Denomination, Dimension, Kind, Owner, ParamDecl, Params};
use crate::registry::Registry;
use crate::world::{PhaseDecl, Produces};

/// The week's own thirty-one slots are named first, so a system's phase is named after them.
pub const FIRST_SLOT: u32 = 31;

/// One system, its name, its phases and whoever it puts in a book.
pub struct Wired {
    pub name: &'static str,
    /// Its own declaration slot, so two systems cannot declare the same phase.
    pub slot: u32,
    /// Which of the nine stages its MECHANISM runs in. A participant is not placed by this: it is
    /// collected for the books and posts at VIEWS whatever the row says, because posting is the
    /// market door rather than a place in the week.
    pub at: u32,
    pub participant: Option<Box<dyn Participant>>,
    /// ARCHITECTURE 4.9b: its own work in the week, if it has any of its own.
    pub mechanism: Option<Box<dyn Mechanism>>,
    /// What its mechanism needs of the week it runs in, and what that mechanism says into it.
    pub reads: Vec<Produces>,
    pub writes: Vec<Produces>,
    /// 46 F3: what this system's own family of thing is worth, where it owns one.
    pub valuer: Option<Box<dyn Fn() -> Box<dyn crate::module::Valuer>>>,
    /// How this system builds its audit family, not a built one.
    pub audits: Vec<Box<dyn Fn() -> Box<dyn crate::audit::Contribution>>>,
}

impl Wired {
    /// Its declaration slot.
    pub fn slotted(mut self, n: u32) -> Wired {
        self.slot = n;
        self
    }
}

impl System for Wired {
    fn name(&self) -> &'static str {
        self.name
    }

    fn mechanism(&self) -> Option<&dyn Mechanism> {
        self.mechanism.as_deref()
    }

    fn audits(&self) -> Vec<Box<dyn crate::audit::Contribution>> {
        self.audits.iter().map(|make| make()).collect()
    }

    fn valuers(&self) -> Vec<Box<dyn crate::module::Valuer>> {
        self.valuer.iter().map(|make| make()).collect()
    }

    fn phases(&self) -> Vec<PhaseDecl> {
        // A phase runs a mechanism, so a row that has none declares none. Its participant is asked
        // at e2 by the kernel, and a stage it named there would be a stage nothing happened in.
        if self.mechanism.is_none() {
            assert!(
                self.reads.is_empty() && self.writes.is_empty(),
                "Law 4: {} produces nothing of the week and cannot declare what it hands it",
                self.name
            );
            return Vec::new();
        }
        // The phase's name is its own declaration slot; the owner is the system itself.
        vec![phase(
            self.slot,
            self.slot,
            self.at,
            &self.reads,
            &self.writes,
        )]
    }

    fn participants(&self) -> Vec<&dyn Participant> {
        match &self.participant {
            Some(p) => vec![p.as_ref()],
            None => Vec::new(),
        }
    }
}

/// A system that has its OWN WORK in the week and no reason to be in a book: it reads the world
/// through the second door and proposes.
pub fn works(
    name: &'static str,
    at: u32,
    needs: &[Produces],
    makes: &[Produces],
    mechanism: Box<dyn Mechanism>,
) -> Wired {
    Wired {
        name,
        slot: 0,
        at,
        participant: None,
        valuer: None,
        mechanism: Some(mechanism),
        reads: needs.to_vec(),
        writes: makes.to_vec(),
        audits: Vec::new(),
    }
}

/// A system that puts somebody in a book. Its participant posts at VIEWS; the stage is where its
/// own work runs, and a row that only posts has none — so what it declares is its MECHANISM's.
pub fn posts(name: &'static str, participant: Box<dyn Participant>) -> Wired {
    Wired {
        valuer: None,
        name,
        slot: 0,
        // Money G2.e: a party posts at e2 and nowhere else, so a row whose only act is to post has
        // no stage to declare. It runs no phase, and `phases` is where that is said.
        at: crate::world::VIEWS,
        participant: Some(participant),
        mechanism: None,
        reads: Vec::new(),
        writes: Vec::new(),
        audits: Vec::new(),
    }
}

/// The lines each system needs, NAMED. How a good is MADE is not here: that is data, and it lives
/// in the registry with everything else the ids point at.
pub struct Wiring {
    /// What funds, insurers and dealers may hold.
    pub lines: Vec<InstrumentId>,
    /// The weekly_funding book's subject.
    pub weekly_funding: Option<InstrumentId>,
    /// What the treasury auctions.
    pub paper: Option<InstrumentId>,
}

/// 37 A2, Law 4, Law 19: the basket is a READ of what the registry says is made.
fn basket(r: &Registry) -> Vec<InstrumentId> {
    r.made().to_vec()
}

/// Each plant with everything the ways of running it draw on — what a holder of that plant is
/// keeping rather than selling.
fn keeps(r: &Registry) -> Vec<(InstrumentId, Vec<InstrumentId>)> {
    r.made()
        .iter()
        .filter_map(|line| {
            let mut inputs: Vec<InstrumentId> = Vec::new();
            for way in r.ways_of(*line) {
                for (what, _) in &way.per_unit {
                    if !inputs.contains(what) {
                        inputs.push(*what);
                    }
                }
            }
            Some((r.made_with(*line)?, inputs))
        })
        .collect()
}

/// Audit C3, 33 A6.b, 22e: which lines the plant-moves family is about, by row.
/// 38 A4: the carriage line each route's room is made of — a good with a route behind it, declared
/// with the network and known by the unit it is measured in.
fn carriage(r: &Registry) -> Vec<InstrumentId> {
    r.carriage().iter().map(|(_, line)| *line).collect()
}

fn plants(r: &Registry) -> Vec<InstrumentId> {
    let mut out: Vec<InstrumentId> = Vec::new();
    for line in r.made() {
        match r.made_with(*line) {
            Some(plant) if !out.contains(&plant) => out.push(plant),
            _ => {}
        }
    }
    out
}

fn capital(r: &Registry) -> Vec<bool> {
    let mut is_capital: Vec<bool> = Vec::new();
    for plant in plants(r) {
        let row = plant.row();
        while is_capital.len() <= row {
            is_capital.push(false);
        }
        is_capital[row] = true;
    }
    is_capital
}

fn goods(r: &Registry) -> Vec<bool> {
    let mut is_good = Vec::new();
    for line in r.made() {
        while is_good.len() <= line.row() {
            is_good.push(false);
        }
        is_good[line.row()] = true;
    }
    is_good
}

/// Every system this world has, and every one of them RUNS.
pub fn declare(p: &mut Params) {
    let mut say = |id: &str,
                   value: f64,
                   unit: &str,
                   dimension: Dimension,
                   kind: Kind,
                   owner: Option<Owner>,
                   why: &str| {
        p.declare(ParamDecl {
            id: id.to_string(),
            value,
            unit: unit.to_string(),
            dimension,
            kind,
            owner,
            why: why.to_string(),
        });
    };

    // 21i, 33 A4: the one declared number congestion has.
    say(
        "reporting.asymmetry",
        7.0,
        "weeks after the books close",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how long after a company's quarter-end its accounts are published",
    );
    // How long a holder has to sell what its mandate no longer lets it hold.
    say(
        "funding.now",
        0.0,
        "weeks ahead",
        Dimension::Weeks,
        Kind::Resolution,
        None,
        "the near end of a funding window, which is today",
    );
    say(
        "funding.this_week",
        1.0,
        "weeks ahead",
        Dimension::Weeks,
        Kind::Resolution,
        None,
        "how far ahead a working-capital shortfall is read, which is one week",
    );
    say(
        "funding.this_year",
        52.0,
        "weeks ahead",
        Dimension::Weeks,
        Kind::Resolution,
        None,
        "how far ahead a long-term shortfall is read, which is a year",
    );
    // Commercial paper's own convention, which is what makes it a different
    // instrument from a bond rather than the same one with a different number in it.
    say(
        "paper.tenor",
        13.0,
        "weeks the paper runs",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how long the commercial paper a borrower brings runs for, expressed on the fixed weekly clock",
    );
    say(
        "firm.buffer",
        10.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a firm's own cash management, which reads what it owes and when against what it holds".to_string(),
        },
        None,
        "the cash a borrower that is not the state keeps back beyond what falls due",
    );
    // The seller's own limits, which is what makes terms a decision rather than a rule.
    say(
        "trade_credit.will_carry",
        500.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a seller's own view of each buyer, which is what decides how much it will have out to one at once".to_string(),
        },
        None,
        "how much a seller will have out to one buyer at once before it stops offering terms",
    );
    say(
        "trade_credit.will_wait",
        5.0,
        "weeks",
        Dimension::Weeks,
        Kind::Placeholder {
            mechanism: "the terms a seller and a buyer strike, which is a decision between them"
                .to_string(),
        },
        None,
        "how long a seller will wait to be paid",
    );
    // How many shares a line comes into existence with.
    say(
        "lender.hurdle",
        0.05,
        "per unit lent",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "a lender's own cost of funds, which is what a hurdle is read against"
                .to_string(),
        },
        None,
        "the return a lender wants on what it puts out, which its standard is read against",
    );
    say(
        "bank.min_weighted",
        0.08,
        "capital per unit of weighted assets",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the capital a bank must hold against its risk-weighted assets",
    );
    say(
        "bank.min_leverage",
        0.03,
        "capital per unit of assets",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the backstop: capital against total assets, whatever they weigh",
    );
    say("bank.buffer", 0.025, "capital per unit above the requirement", Dimension::Ratio, Kind::Policy, Some(Owner::Parliament),
        "the buffer a bank is expected to keep above its requirement, inside which there are consequences short of a breach");
    // The weight schedule is the regulator's, and it is two numbers rather than one per rung.
    say(
        "bank.on_the_best",
        1.01,
        "per unit carried",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "what a bank must hold against a claim on the best credit there is, per unit it carries",
    );
    say(
        "bank.per_notch",
        1.07,
        "per unit carried, per notch",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "how much more a bank must hold for each notch further down the scale a name is graded",
    );
    say(
        "bank.ungraded_at",
        7.0,
        "notches below the best",
        Dimension::Count,
        Kind::Policy,
        Some(Owner::Parliament),
        "where on the scale a bank must weight a name nobody has graded",
    );
    // And a house's own scale, from which its twenty-two band edges fall out.
    say(
        "ratings.best_carries",
        0.25,
        "strain",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism:
                "each house's own calibration of its scale, which is the opinion it is paid for"
                    .to_string(),
        },
        None,
        "the strain a name at the top of a house's scale already carries",
    );
    say(
        "ratings.per_notch",
        0.25,
        "strain per notch",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "what one notch of a house's own scale is worth to that house".to_string(),
        },
        None,
        "the strain one notch of a house's scale is worth",
    );
    say(
        "ratings.without_a_record",
        3.0,
        "notches",
        Dimension::Count,
        Kind::Placeholder {
            mechanism: "what a house does about a name it cannot read, which is its own judgement"
                .to_string(),
        },
        None,
        "the notches a house marks down a name it has no history for",
    );
    say(
        "ratings.record_after",
        8.0,
        "weeks",
        Dimension::Weeks,
        Kind::Placeholder {
            mechanism: "when a house decides a name has a record worth reading".to_string(),
        },
        None,
        "how long a name must have existed before a house reads its numbers as a record",
    );
    // The mix a company would raise at.
    say(
        "fund.draws",
        0.05,
        "per unit uncalled",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism:
                "a fund's own need for cash, which is what decides when it calls a commitment"
                    .to_string(),
        },
        None,
        "the share of an uncalled commitment a fund draws in one week",
    );
    say(
        "small_business.reaches",
        5000.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the size at which a borrower can reach the bond market instead of a bank",
    );
    say(
        "acquirer.hurdle",
        0.1,
        "per unit paid",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism:
                "an acquirer's own cost of capital, which is what it weighs a target against"
                    .to_string(),
        },
        None,
        "the return an acquirer wants on what it pays for a company",
    );
    say(
        "control.needs",
        0.5,
        "per unit outstanding",
        Dimension::Ratio,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how much of the shares it does not already hold a tender must reach",
    );
    say(
        "pool.junior",
        0.1,
        "per unit of the pool",
        Dimension::Ratio,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the share of a pool that stands in front of its senior note",
    );
    say(
        "pool.pools",
        0.2,
        "per unit of its loan book",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "how much of its book a bank pools, which follows from the capital the pooling would free".to_string(),
        },
        None,
        "how much of its loan book a bank pools at once",
    );
    say(
        "dwelling.upkeep",
        0.02,
        "money per dwelling per week",
        Dimension::PricePerUnit,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "what keeping one dwelling in repair costs its owner each week",
    );
    say(
        "household.will_spend.from",
        0.3,
        "per unit of its money",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "the spending decision a household makes from its income, its wealth, its own outlook and what it can borrow".to_string(),
        },
        None,
        "the lower bound of the entry-time household housing-budget distribution",
    );
    say(
        "household.will_spend.to",
        0.7,
        "per unit of its money",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "the spending decision a household makes from its income, its wealth, its own outlook and what it can borrow".to_string(),
        },
        None,
        "the exclusive upper bound of the entry-time household housing-budget distribution",
    );
    say(
        "storage.per_unit",
        0.01,
        "money per unit per week",
        Dimension::PricePerUnit,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "what holding one unit of a physical good for one week costs",
    );
    say(
        "subscribe.commits",
        0.1,
        "per unit of spare cash",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "a holder's own allocation, which is a decision about its own money"
                .to_string(),
        },
        None,
        "the share of its spare money a holder commits to one pool",
    );
    // The broker's own view and its own limit.
    say(
        "broker.could_move",
        0.2,
        "per unit of the book",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "initial margin sized from the underlying's own measured move, scaled by the notional and the life left, which is not a stated rate per class".to_string(),
        },
        None,
        "what a broker thinks a client's book could move against it in a week",
    );
    say(
        "broker.limit",
        100000.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a broker's own limit on one client, read from what that client's book could do to it".to_string(),
        },
        None,
        "what one broker will be exposed to one client for",
    );
    say(
        "forward.tenor",
        13.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how far out a currency forward is struck",
    );
    say(
        "protection.tenor",
        5.0,
        "years",
        Dimension::Years,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how long a protection contract runs",
    );
    say(
        "observer.lag",
        2.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the weeks between what a statistic is about and the week it is published in",
    );
    say(
        "invest.horizon",
        20.0,
        "weeks",
        Dimension::Weeks,
        Kind::Placeholder {
            mechanism: "the horizon a management counts, which is its own and differs between them"
                .to_string(),
        },
        None,
        "how many weeks of return a management counts when it weighs a project",
    );
    say(
        "invest.hurdle",
        0.02,
        "per unit above the cost of capital",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "what a management wants above its own cost of capital before it commits"
                .to_string(),
        },
        None,
        "what a management wants above its cost of capital before it commits",
    );
    say(
        "invest.takes",
        3.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the weeks a capital programme runs before the plant is in service",
    );
    say(
        "equity.takes",
        4.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the weeks a flotation stands before it is over, one way or the other",
    );
    say(
        "equity.payout",
        0.25,
        "ratio",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism:
                "each company's own distribution decision, which is a decision others react to"
                    .to_string(),
        },
        None,
        "the share of the prior settled operating cash result declared as a dividend",
    );
    say(
        "parliament.seats",
        100.0,
        "seats",
        Dimension::Count,
        Kind::Policy,
        Some(Owner::Constitution),
        "how many seats the parliament of a country has",
    );
    say(
        "parliament.term",
        209.0,
        "weeks",
        Dimension::Weeks,
        Kind::Policy,
        Some(Owner::Constitution),
        "how long a parliament sits before its term runs out",
    );
    say(
        "election.takes",
        1.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the weeks between an election being called and its result being known",
    );
    say(
        "workout.within",
        2.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the weeks a holder has to sell a line its mandate no longer lets it hold",
    );
    say(
        "loss.impair_after",
        2.0,
        "weeks non-performing",
        Dimension::Weeks,
        Kind::Policy,
        Some(Owner::StandardSetter),
        "how long a finally failed claim remains non-performing before impairment",
    );
    say(
        "loss.write_off_after",
        2.0,
        "weeks impaired",
        Dimension::Weeks,
        Kind::Policy,
        Some(Owner::StandardSetter),
        "how long an impaired claim remains unresolved before write-off",
    );
    say("building.crowds_at", 60.0, "square km standing", Dimension::SquareKm, Kind::Technology, None,
        "the ground already covered in a place at which building there draws twice what it does on empty ground");
    // 37 B1, 22c.3: how much cover a firm wants on its shelf.
    say(
        "firm.cover",
        0.5,
        "multiple of what it expects to sell",
        Dimension::Ratio,
        Kind::Placeholder {
            mechanism: "a firm's own inventory policy, formed from what it expects to sell"
                .to_string(),
        },
        None,
        "how much stock a firm wants on the shelf beyond the week it expects to sell",
    );
    // 37 C1, 22c.3: what another week on the shelf costs the holder, as a share of what the units
    // cost it — the storage, the spoilage and the money tied up.
    say("plant.upkeep", 0.5, "money", Dimension::Amount(Denomination::Money), Kind::Technology, None,
        "what keeping a plant costs its owner a week whether or not anybody books it — 33 A4.b's fixed cost");
    say(
        "stockist.limit",
        50.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a stockist's own limit on one line, read from what it can fund and what it can shift".to_string(),
        },
        None,
        "the most a stockist will carry of one line — without one it is the buyer of last resort",
    );
    say("goods.seller.holding_costs", 0.03, "share of what the units cost, a week", Dimension::Ratio, Kind::Technology, None,
        "what it costs to keep a unit another week: the room it takes, what spoils and the money in it");
    // A fact about the thing, not about who holds it.
    say(
        "goods.perishes",
        0.01,
        "share of a lot a week",
        Dimension::Ratio,
        Kind::Technology,
        None,
        "the share of a lot that does not survive the week",
    );
    // The money a household keeps back.
    say(
        "household.keeps.from",
        0.5,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "the buffer a household holds, which follows from its income, what it owes and whether it can borrow".to_string(),
        },
        None,
        "the lower bound of the entry-time household liquidity-buffer distribution",
    );
    say(
        "household.keeps.to",
        1.5,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "the buffer a household holds, which follows from its income, what it owes and whether it can borrow".to_string(),
        },
        None,
        "the exclusive upper bound of the entry-time household liquidity-buffer distribution",
    );
    // A bank's own liquidity buffer. What it lends and borrows at is its own cost of funds.
    say(
        "money_market.buffer",
        1.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a bank's own buffer, derived from how liquid its own liabilities are"
                .to_string(),
        },
        None,
        "the balance a bank keeps back before it lends weekly_funding",
    );
    // 11 B6: the other tenor beside the week. A market's standard term is a convention, and what
    // a bank does with it is its own decision.
    say(
        "money_market.term",
        13.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        None,
        "how long a TERM is in the interbank market, beside the week",
    );
    say(
        "central_bank.facility_advance",
        0.8,
        "share of collateral market value",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::CentralBank),
        "the share of eligible collateral value the central bank advances at its standing facility",
    );
    say(
        "central_bank.facility_penalty",
        0.02,
        "per annum over the market",
        Dimension::PerAnnum,
        Kind::Policy,
        Some(Owner::CentralBank),
        "the standing facility penalty over the observed money-market rate",
    );
    // A desk's own inventory constraint. Its price and width are decisions, not parameters.
    say(
        "dealer.limit",
        10.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Placeholder {
            mechanism: "a desk's own risk function, which is what sets a position limit per line and in aggregate".to_string(),
        },
        None,
        "the most a desk will hold of one line",
    );
    // The shape is dead.
    say(
        "treasury.buffer",
        200.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Some(Owner::Parliament),
        "the balance the treasury keeps back, which is why one failed auction is not a default",
    );
    say(
        "tax.income",
        0.2,
        "share of settled wage income",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the income-tax rate applied to each named recipient's settled wage income",
    );
    say(
        "tax.consumption",
        0.1,
        "share of settled taxable purchases",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the consumption-tax rate applied to each buyer's settled purchases",
    );
    say(
        "tax.corporate",
        0.2,
        "share of positive reported income",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the corporate-tax rate applied to each reporting entity's positive taxable result",
    );
    say(
        "tax.payroll",
        0.1,
        "share of settled wage payments",
        Dimension::Ratio,
        Kind::Policy,
        Some(Owner::Parliament),
        "the payroll-tax rate applied to the employer on each settled wage payment",
    );
    say(
        "labour.hours_per_person",
        35.0,
        "hours per person per week",
        Dimension::Amount(Denomination::Time),
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the standard hours carried by a newly struck employment agreement",
    );
    say(
        "freight.loading_weeks",
        1.0,
        "weeks",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "the time a vehicle spends being loaded and unloaded, on top of the time it spends moving. A week is the clock's finest grain (1 G3.b), so this is the least a load can take and be placed at all",
    );
    say(
        "labour.severance_periods",
        4.0,
        "weeks of wages",
        Dimension::Weeks,
        Kind::Policy,
        Some(Owner::Parliament),
        "the wage weeks an employer owes when it terminates an engagement",
    );
    say(
        "mortgage.tenor",
        260.0,
        "weeks",
        Dimension::Weeks,
        Kind::Placeholder {
            mechanism: "the term each mortgage is agreed on, between its lender and its borrower"
                .to_string(),
        },
        None,
        "the duration agreed for a newly originated mortgage claim",
    );
    say("sovereign.willingness", 1.0, "share", Dimension::Ratio, Kind::Preference, Some(Owner::Parliament),
        "the fiscal authority's opening willingness to honour a due; copied into its own durable mandate");
    // 5 C3.a, 21j.1a: the tenor and the coupon paper is BROUGHT at.
    say(
        "funding.tenor",
        260.0,
        "weeks the paper runs",
        Dimension::Weeks,
        Kind::Technology,
        Some(Owner::StandardSetter),
        "how long the paper an issuer brings runs for, expressed on the fixed weekly clock — the tenor \
         its market quotes, and what makes a five-year line five years rather than 260 weeks",
    );
}

pub fn all(
    w: &Wiring,
    r: &Registry,
    journal: &mut crate::journal::Journal,
    nouns: &mut crate::nouns::Nouns,
) -> Vec<Wired> {
    let keys_of = |j: &mut crate::journal::Journal, name: &str| j.keys_named.declare(name);
    let at_equity = keys_of(journal, "accounts.equity");
    let at_opening_equity = keys_of(journal, "accounts.opening_equity");
    let at_income = keys_of(journal, "accounts.income");
    let at_firm_revenue = keys_of(journal, "firm.revenue");
    let at_firm_costs = keys_of(journal, "firm.costs");
    let at_firm_fixed = keys_of(journal, "firm.fixed_costs");
    let at_firm_working_capital = keys_of(journal, "firm.working_capital");
    let at_firm_cash = keys_of(journal, "firm.operating_cash");
    let at_firm_banked = keys_of(journal, "firm.cash_after_working_capital");
    let at_firm_coverage = keys_of(journal, "firm.coverage");
    let at_firm_margin = keys_of(journal, "firm.margin");
    let at_firm_revenue_change = keys_of(journal, "firm.revenue_change");
    let at_programme_funding = keys_of(journal, "programme.funding");
    let at_programme_issue = keys_of(journal, "programme.issue");
    let at_shares = keys_of(journal, "accounts.shares");
    let at_closed = keys_of(journal, "accounts.closed");
    let at_standing = keys_of(journal, "claim.standing");
    let at_loss = keys_of(journal, "claim.loss");
    let at_failure_trigger = keys_of(journal, "mortality.trigger");
    let at_failure_destination = keys_of(journal, "mortality.destination");
    let at_ratio = keys_of(journal, "bank.ratio");
    let at_funding_short = keys_of(journal, "bank.funding.short");
    let at_funding_rate = keys_of(journal, "bank.funding.rate");
    let at_about = keys_of(journal, "statistic.about");
    let at_value = keys_of(journal, "statistic.value");
    let at_revised = keys_of(journal, "statistic.revised_from");
    let at_the_mark = keys_of(journal, "position.mark");
    let keys_of_current = keys_of(journal, "region.current_account");
    let keys_of_financial = keys_of(journal, "region.financial_account");
    let keys_of_valuation = keys_of(journal, "region.valuation_changes");
    let at_index = keys_of(journal, "index.id");
    let at_index_subject = keys_of(journal, "index.subject");
    let at_index_level = keys_of(journal, "index.level");
    let at_index_observed = keys_of(journal, "index.observed_week");
    let at_index_base = keys_of(journal, "index.base_week");
    let at_fx_base = keys_of(journal, "fx.base_currency");
    let at_fx_quote = keys_of(journal, "fx.quote_currency");
    let kinds = &mut journal.kinds;
    // ONE EVENT KIND PER SYSTEM THAT PUBLISHES A READ, and every one of them says what it holds and
    // why: a kind is a store a module keeps, and `wire_up` refuses one the register never heard of.
    let mut says = |name: &str, holds: &str, why: &str| {
        nouns.declare(crate::nouns::NounDecl {
            name: name.to_string(),
            sort: crate::nouns::Sort::Noun { home: None },
            holds: holds.to_string(),
            why: why.to_string(),
        });
        kinds.declare(name)
    };
    let kinds_row_accounts = says(
        "accounts.published",
        "what each company published, as at a date, and what it said",
        "48 A1: a covenant is tested against published accounts and a bid is formed from them, so what was said has to outlive the week that said it",
    );
    let kinds_row_short_of_capital = says(
        "bank.short_of_capital",
        "which banks said they are short of capital, and by how much",
        "28 C: a raise is called in the week the ratio was read and paid in the week after, so the shortfall has to survive the week between",
    );
    let kinds_row_fixing = says(
        "benchmarks.fixing",
        "the level each benchmark fixed at",
        "22 B: a benchmark is a read of what transacted, so the fixing is a record and never a posting",
    );
    let kinds_row_costs = says(
        "capital.costs",
        "what a company's own capital costs it",
        "27 A: a hurdle is the company's own cost of funds, and every investment decision reads it",
    );
    let kinds_row_spot = says(
        "spot.rate",
        "the rate each currency pair cleared at",
        "18 A: a pair has one cleared rate a week, and everything derived from it reads that one",
    );
    let kinds_row_loss_crossed = says(
        "claim.crossed",
        "the claims that crossed into arrears this week",
        "12 C: an impairment is an event on a date, and the date is the day the arrear crossed",
    );
    let kinds_row_dissolved = says(
        "household.dissolved",
        "the household cells whose life ended",
        "41 F1.b: dissolution is a weight event with a cause, and what it held has to reach a named heir",
    );
    let kinds_row_past_waterfall = says(
        "clearing.waterfall.exhausted",
        "the clearing houses whose waterfall ran out",
        "17 E: a house that has spent its waterfall has no resources left, which is the fact its members act on",
    );
    let kinds_row_mortality_failed = says(
        "mortality.failed",
        "the parties whose life ended, with the trigger and the destination",
        "XI-8: nothing is immortal, and no death is without a destination",
    );
    let kinds_row_funding_failed = says(
        "bank.funding.failed",
        "the banks that could not fund themselves this week",
        "25 D: illiquidity and insolvency are different failures, and a reader has to be able to tell them apart",
    );
    let kinds_row_facility_drawn = says(
        "bank.facility.drawn",
        "what each bank drew on a central-bank facility",
        "24 C: last-resort lending is a real loan on both balance sheets, and what was drawn is the record of it",
    );
    let kinds_row_firm_result = says(
        "firm.result",
        "what a firm's own week produced: revenue, costs, cash and equity",
        "34 B: a firm acts on its own result, and the result is a read of what happened to it rather than a sector average",
    );
    let kinds_row_programme = says(
        "plant.built",
        "what each capital programme committed to and what was delivered",
        "36 C: plant is built over weeks, so a programme is a process with an owner and an end",
    );
    let kinds_row_rent = says(
        "dwelling.let",
        "what each dwelling let for this week",
        "40 D: a rent is a cleared price, and the consumer basket weighs the ones let this week",
    );
    let sovereign_default_kind = says(
        "sovereign.default",
        "the sovereigns that missed, and on what",
        "8 F: a sovereign in its own money defaults by choosing to, and the choice is an event with a date",
    );
    let mut rows = vec![
        {
            // §37 both posts and works: the stock that does not survive the week leaves at what
            // it cost, and then a firm offers what is left of what it holds.
            let mut goods = works(
                "goods",
                AT_D1,
                &[],
                &[],
                Box::new(crate::mechanisms::goods::Perishing {
                    share: "goods.perishes",
                }),
            );
            goods.participant = Some(Box::new(GoodsSellers {
                holding_costs: "goods.seller.holding_costs",
                keeps: keeps(r),
            }));
            // 46 F3, 37 A2: a good is worth what it is used for, and the family says so.
            goods.valuer = Some(Box::new(|| {
                Box::new(crate::mechanisms::goods::GoodIsWorthWhatItMakes)
            }));
            goods
        },
        // A cell bids for what it can fund. Its outlook is the `expectations` row's, once, at VIEWS:
        // §46's memory is ONE preference over a party's own history, and applied twice a week it
        // is a different preference.
        posts(
            "households",
            Box::new(HouseholdBuyers { basket: basket(r) }),
        ),
        // THE ONE SYSTEM THAT MAKES ANYTHING: the lines draw at d1 and the batches come off at
        // d2, so what is made this week was not an input to what ran this week.
        works(
            "recipe",
            AT_D1,
            &[],
            &[],
            Box::new(Making {
                flow: CostFlow::FirstInFirstOut,
                crowds_at: "building.crowds_at",
                cover: "firm.cover",
            }),
        ),
        {
            // 37 E2, E3: stock is marked down where the market fell below what it cost, and the
            // write-down is an event with a date and a size.
            let wrote_down = says(
                "goods.written_down",
                "the stock each holder wrote down this week, and by how much",
                "37 E3: a holding loss is an event with a date, a size and an income line, and the asymmetry in E2 is the mechanism",
            );
            works(
                "goods_marking",
                AT_G1,
                &[],
                &[Produces(wrote_down)],
                Box::new(crate::mechanisms::goods::Marking {
                    kind: wrote_down,
                    at_line: 0,
                    at_charge: 1,
                }),
            )
        },
        {
            let disrupted = says(
                "goods.disrupted",
                "the units a producer lost on the line this week, and what they had cost it",
                "21 B3: a disruption is a real loss of units at the point they would have been made, never a multiplier on a price",
            );
            works(
                "finishing",
                AT_D2,
                &[],
                &[Produces(disrupted)],
                Box::new(crate::mechanisms::goods::Finishing { disrupted }),
            )
        },
        works(
            "firms",
            AT_G2,
            &[],
            &[Produces(kinds_row_firm_result)],
            Box::new(Reporting {
                kind: kinds_row_firm_result,
                at_revenue: at_firm_revenue,
                at_costs: at_firm_costs,
                at_fixed: at_firm_fixed,
                at_working_capital: at_firm_working_capital,
                at_operating_cash: at_firm_banked,
                at_coverage: at_firm_coverage,
                at_margin: at_firm_margin,
                at_revenue_change: at_firm_revenue_change,
                at_cash: at_firm_cash,
                at_equity,
                at_opening_equity,
            }),
        ),
        {
            let struck_wage = says(
                "labour.cleared",
                "the wage the week's labour book struck, and the places it filled",
                "39 D1: the wage is a price that clears between posted supply and posted demand, and the bid that took the last place is the print",
            );
            works(
                "employment",
                AT_D5,
                &[],
                &[Produces(struck_wage)],
                Box::new(Wages {
                    hours_per_person: "labour.hours_per_person",
                    severance_periods: "labour.severance_periods",
                    says: struck_wage,
                }),
            )
        },
        {
            // 38 B1, B2: a carrier makes the week's room out of the plant it owns, and what nobody
            // buys is gone with the week.
            let room = says(
                "freight.room",
                "the room each carrier made on each route this week",
                "38 A4, B2: capacity is per route and fixed in the short run, so what a carrier can move is a fact before anybody bids for it",
            );
            let mut f = works(
                "freight",
                AT_D4,
                &[],
                &[Produces(room)],
                Box::new(crate::mechanisms::freight::Sells {
                    on: r.carriage().to_vec(),
                    says: room,
                }),
            );
            f.participant = Some(Box::new(OffersItsRoom { lines: carriage(r) }));
            f
        },
        // 49 G4: and what was carried ARRIVES — title moves and the vehicle is where it delivered.
        {
            let landed = says(
                "freight.arrived",
                "what arrived this week, for whom, and whether the carrier delivered it",
                "49 G4: carrier failure and missed delivery are named outcomes, and goods that never arrive are goods nobody can sell",
            );
            let mut a = works(
                "arrivals",
                AT_D4,
                &[],
                &[Produces(landed)],
                Box::new(crate::mechanisms::freight::Arrives { says: landed }),
            );
            a.audits.push(Box::new(|| {
                Box::<crate::mechanisms::freight::CargoHasAnOwner>::default()
            }));
            a.audits.push(Box::new(|| {
                Box::<crate::mechanisms::freight::DeliveriesLandOnce>::default()
            }));
            a.audits.push(Box::new(|| {
                Box::<crate::mechanisms::freight::FreightIsPaidFor>::default()
            }));
            a
        },
        // 38 C1: and the other side of that book is whoever has goods to move. Demand is derived,
        // so it is the seller's own margin that bids and the seller's own outlook that sizes it.
        posts(
            "shippers",
            Box::new(crate::mechanisms::freight::Ships {
                on: r.carriage().to_vec(),
            }),
        ),
        // And stock is TIGHT or it is not, and storing it costs money to somebody.
        {
            let physical = goods(r);
            let tightness = says(
                "stock.tightness",
                "how tight each good's stock is where it is held",
                "37 C3: a shortage is a fact about a shelf, and it is what the party that has to decide what to make reads",
            );
            // 21 C: a commodity moves and is stored, which is d4's work and not the line's.
            let mut commodities = works(
                "commodities",
                AT_D4,
                &[],
                &[Produces(tightness)],
                Box::new(Storing {
                    kind: tightness,
                    at_line: at_about,
                    at_value,
                    per_unit: "storage.per_unit",
                }),
            );
            commodities.audits.push(Box::new(move || {
                Box::new(crate::mechanisms::commodities::CommodityUnits::over(
                    physical.clone(),
                ))
            }));
            commodities
        },
        // Somebody whose business is to hold the stock.
        posts(
            "stockists",
            Box::new(Stockist {
                lines: basket(r),
                carrying: "goods.seller.holding_costs",
                limit: "stockist.limit",
            }),
        ),
        // And dwellings are LET and SOLD, and both prices clear.
        {
            let sold = says(
                "dwelling.sold",
                "what each dwelling sold for",
                "40 C: a house price is a cleared price like any other, and a housing market with no prints has no wealth effect",
            );
            works(
                "housing",
                AT_D5,
                &[],
                &[Produces(sold), Produces(kinds_row_rent)],
                Box::new(Housing {
                    kind: sold,
                    lets: kinds_row_rent,
                    upkeep: "dwelling.upkeep",
                    tenor: "mortgage.tenor",
                }),
            )
        },
        // And a seller that has delivered and not been paid OFFERS TERMS.
        {
            let struck = says(
                "invoice.struck",
                "the terms a seller gave a buyer that has not paid",
                "44 A: trade credit is a real claim with a named payer and a date, not a timing adjustment",
            );
            works(
                "trade_credit",
                AT_D5,
                &[],
                &[Produces(struck)],
                Box::new(TradeCredit {
                    kind: struck,
                    will_carry: "trade_credit.will_carry",
                    will_wait: "trade_credit.will_wait",
                }),
            )
        },
        // And the tier too small for the bond market is READ.
        {
            let state = says(
                "small_business.state",
                "which small-business cells can reach the bond market",
                "42 A6: the tier too small for the bond market is a different borrower, and where the boundary falls is a read",
            );
            works(
                "small_business",
                AT_D5,
                &[],
                &[Produces(state)],
                Box::new(SmallBusiness {
                    kind: state,
                    reaches_the_bond_market_at: "small_business.reaches",
                }),
            )
        },
        {
            let brought_funding = says(
                "money_market.brought",
                "the paper each short bank brought to fund itself this week",
                "11 B1, 11 B2: a bank funds itself by borrowing on its own NAME, and one name's paper is not another's",
            );
            // 11 A3.a: the need is knowable only after the week's flows, so it is read at d6 —
            // the position they actually left it in — and the paper is in the book before e2.
            let mut mm = works(
                "money_market",
                AT_D6,
                &[],
                &[Produces(brought_funding)],
                Box::new(Interbank {
                    buffer: "money_market.buffer",
                    term: "money_market.term",
                    says: brought_funding,
                }),
            );
            mm.participant = Some(Box::new(MoneyMarketBanks {
                buffer: "money_market.buffer",
                facility_penalty: "central_bank.facility_penalty",
            }));
            mm
        },
        {
            // It BRINGS the paper in the week's work and AUCTIONS it when the books clear, because
            // a bill has to exist before anybody bids for it.
            let buffer_kind = says(
                "treasury.buffer.mandate",
                "the cash buffer a treasury says it will hold",
                "13 D: a buffer is a stated policy, and what holding it forces the treasury to do is an outcome",
            );
            let brought = says(
                "funding.brought",
                "the paper a treasury brought to the market this week",
                "XI-9: a sovereign funds itself by issuing, and the issue is what the auction runs on",
            );
            let mut t = works(
                "treasury",
                AT_D6,
                &[],
                &[Produces(brought)],
                Box::new(Funding {
                    // The SOVEREIGN's own paper, and nobody else's.
                    of_kinds: &[kinds::TREASURY],
                    after: "funding.now",
                    horizon: "funding.this_week",
                    tenor: "funding.tenor",
                    buffer: "treasury.buffer",
                    buffer_kind,
                    says: brought,
                    income_tax_rate: "tax.income",
                    consumption_tax_rate: "tax.consumption",
                    corporate_tax_rate: "tax.corporate",
                    payroll_tax_rate: "tax.payroll",
                    accounts_kind: kinds_row_accounts,
                    at_income,
                }),
            );
            t.participant = Some(Box::new(TreasuryIssues {
                paper: w.paper,
                buffer_kind,
                default_kind: sovereign_default_kind,
            }));
            t
        },
        // Money A1, 5 A4: every asset is somebody's liability, published party by party.
        {
            let owed = says(
                "money.owed",
                "what each issuer owes, party by party",
                "Money A1, 5 A4: every asset is somebody's liability, and it is published by name",
            );
            let stock = says(
                "money.stock",
                "how much money each issuer has out",
                "Money A1: the stock of a money is a read of what its issuer put out, and nobody keeps a second copy of it",
            );
            works(
                "money",
                AT_G1,
                &[],
                &[Produces(owed), Produces(stock)],
                Box::new(crate::mechanisms::money::Owed {
                    kind: owed,
                    stock_kind: stock,
                }),
            )
        },
        // And a treasury HANDLES being short.
        {
            let shortfall = says(
                "sovereign.shortfall",
                "what each sovereign is short by this week",
                "XI-9: a funding constraint bites when the money is not there, and the shortfall is what it has to act on",
            );
            let auction = says(
                "sovereign.auction.shortfall",
                "the auctions that did not raise what they needed",
                "XI-9: an auction that fails is a recorded outcome, because failing is what changes the next decision",
            );
            let exchange = says(
                "sovereign.exchange.holdouts",
                "who refused an exchange offer, and for how much",
                "8 G: a holdout is a named creditor with a claim, and a restructuring that assumes none is not one",
            );
            let mut sov = works(
                "sovereign",
                AT_G4,
                &[],
                &[
                    Produces(shortfall),
                    Produces(auction),
                    Produces(sovereign_default_kind),
                    Produces(exchange),
                ],
                Box::new(Sovereign {
                    kind: shortfall,
                    auction_kind: auction,
                    default_kind: sovereign_default_kind,
                    willingness_kind: says(
                "sovereign.willingness.mandate",
                "what each sovereign says it will pay",
                "8 F: willingness is a political position, stated, and a default is a choice taken against it",
            ),
                    willingness_decision_kind: says(
                "sovereign.willingness.decision",
                "what it decided when it could not pay everything",
                "8 F: the decision is the event, and a reader has to see which way it went",
            ),
                    at_willingness: 0,
                    exchange_kind: exchange,
                    initial_willingness: "sovereign.willingness",
                }),
            );
            // 8 C3: the obligation to bid is the sovereign's institution, and the desk it binds is
            // the same desk with the same limit.
            sov.participant = Some(Box::new(PrimaryDealers {
                paper: w.paper,
                limit: "dealer.limit",
            }));
            sov
        },
        // And a bank READS its own capital.
        {
            let capital = says(
                "bank.capital",
                "each bank's own reading of its capital",
                "28 A: a ratio is a read of the bank's own book, and the bank acts on its own reading of it",
            );
            works(
                "bank_capital",
                AT_G4,
                &[],
                &[Produces(capital), Produces(kinds_row_short_of_capital)],
                Box::new(BankCapital {
                    kind: capital,
                    short_by: kinds_row_short_of_capital,
                    at_ratio,
                    min_weighted: "bank.min_weighted",
                    min_leverage: "bank.min_leverage",
                    buffer: "bank.buffer",
                    hurdle: "lender.hurdle",
                    on_the_best: "bank.on_the_best",
                    per_notch: "bank.per_notch",
                    ungraded_at: "bank.ungraded_at",
                }),
            )
        },
        // And a bank SETS the rate it pays on deposits.
        {
            let deposit_rate = says(
                "deposit.rate",
                "the rate each bank pays on deposits",
                "25 B: a deposit rate is a bank's own decision, standing until it withdraws it",
            );
            works(
                "bank_funding",
                AT_G4,
                &[],
                &[
                    Produces(deposit_rate),
                    Produces(kinds_row_funding_failed),
                    Produces(kinds_row_facility_drawn),
                ],
                Box::new(BankFunding {
                    kind: deposit_rate,
                    failed: kinds_row_funding_failed,
                    at_short: at_funding_short,
                    facility_advance: "central_bank.facility_advance",
                    facility_penalty: "central_bank.facility_penalty",
                    facility_drawn: kinds_row_facility_drawn,
                    at_rate: at_funding_rate,
                    weekly_funding_fixing: kinds_row_fixing,
                }),
            )
        },
        // THE ONE THE WHOLE CREDIT SIDE RESTS ON — what falls due is paid, or it is an arrear.
        works("lending", AT_B2, &[], &[], Box::new(Servicing)),
        {
            // Audit C3, 33 A6.b, 22e: PLANT MOVES ONLY FOR A REASON, and this is the one module
            // family this world has.
            let capital = capital(r);
            // And a firm DECIDES to invest.
            let mut cp = works(
                "capital_programme",
                AT_D3,
                &[],
                &[Produces(kinds_row_programme)],
                Box::new(Building {
                    kind: kinds_row_programme,
                    at_funding: at_programme_funding,
                    at_issue: at_programme_issue,
                    costs: kinds_row_costs,
                    horizon: "invest.horizon",
                    hurdle: "invest.hurdle",
                    crowds_at: "building.crowds_at",
                    takes: "invest.takes",
                }),
            );
            cp.participant = Some(Box::new(Builder {
                of_kind: kinds::FIRM,
            }));
            // 46 F3, 33 A5: a plant is worth what it can produce, and the family says so.
            cp.valuer = Some(Box::new(|| {
                Box::new(crate::mechanisms::capital_programme::PlantIsWorthWhatItMakes)
            }));
            cp.audits.push(Box::new(move || {
                Box::new(crate::mechanisms::capital_programme::PlantMoves::over(
                    capital.clone(),
                ))
            }));
            cp
        },
        // And a company knows what its capital COSTS it.
        works(
            "cost_of_capital",
            AT_G3,
            &[],
            &[Produces(kinds_row_costs)],
            Box::new(CostOfCapital {
                kind: kinds_row_costs,
                accounts: kinds_row_accounts,
                at_income,
                at_shares,
            }),
        ),
        // A borrower short over the WEEK brings commercial paper. What it must fund includes the
        // plant this week's programme committed to, so it runs after it.
        {
            let brought = says(
                "paper.brought",
                "the commercial paper a borrower short over the week brought",
                "9 A: short funding is issued paper with a maturity, not a line somebody drew",
            );
            works(
                "short_term_debt",
                AT_D6,
                &[Produces(kinds_row_programme)],
                &[Produces(brought)],
                Box::new(BringsPaper {
                    of_kinds: &[kinds::BANK, kinds::FIRM, kinds::SMALL_FIRM],
                    after: "funding.now",
                    horizon: "funding.this_week",
                    tenor: "paper.tenor",
                    buffer: "firm.buffer",
                    says: brought,
                    programme: Some(kinds_row_programme),
                    at_programme_funding: Some(at_programme_funding),
                }),
            )
        },
        // And a borrower short over the YEAR brings a bond.
        {
            let brought = says(
                "bond.brought",
                "the bonds a borrower short over the year brought",
                "7 B: term funding is a bond with a coupon and a maturity, issued because a shortfall existed",
            );
            works(
                "corporate_credit",
                AT_D6,
                &[],
                &[Produces(brought)],
                Box::new(BringsBond {
                    of_kinds: &[kinds::FIRM, kinds::BANK],
                    after: "funding.this_week",
                    horizon: "funding.this_year",
                    tenor: "funding.tenor",
                    buffer: "firm.buffer",
                    says: brought,
                }),
            )
        },
        {
            let orphaned = says(
                "fund.orphaned",
                "the funds whose holders have no redemption route left",
                "13 E: a claim nobody can redeem is not redeemable, and that has to be visible to its holder",
            );
            let gate = says(
                "fund.redemption_gate",
                "which funds are gating redemptions",
                "13 D: a gate is a decision the fund takes, and its holders act on it",
            );
            // A pool whose manager died posts nothing and winds up.
            let mut f = works(
                "funds",
                AT_G4,
                &[],
                &[Produces(orphaned), Produces(gate)],
                Box::new(Winding {
                    says: orphaned,
                    gate_says: gate,
                    ceased: kinds_row_mortality_failed,
                    at_trigger: at_failure_trigger,
                    at_destination: at_failure_destination,
                }),
            );
            f.participant = Some(Box::new(FundMandates {
                may_hold: w.lines.clone(),
            }));
            f
        },
        {
            let policies = says(
                "insurers.policies",
                "the policies each insurer has written",
                "30 A: an insurer's liability is the policies it wrote, to named holders",
            );
            let mut i = works(
                "insurers",
                AT_D5,
                &[],
                &[Produces(policies)],
                Box::new(Policies { kind: policies }),
            );
            i.participant = Some(Box::new(InsurerMatching {
                long_lines: w.lines.clone(),
            }));
            i
        },
        {
            let lines = says(
                "dealing.lines",
                "the lines each desk is making a market in",
                "26 A: a desk quotes what it chooses to quote, and the inventory that results is its own",
            );
            let mut d = works(
                "dealing",
                AT_G4,
                &[],
                &[Produces(lines)],
                Box::new(Lines { kind: lines }),
            );
            d.participant = Some(Box::new(Dealers {
                limit: "dealer.limit",
                lines: w.lines.clone(),
            }));
            d
        },
        {
            // And a fund is the BUYER when others are forced sellers.
            let marked = says(
                "fund.marked",
                "what each fund's book is marked at, and what it could raise",
                "28 A5: a fund marks, so its equity moves with the print, which is what a redemption meets",
            );
            let mut h = works(
                "hedge_funds",
                AT_G4,
                &[],
                &[Produces(marked)],
                Box::new(Levering {
                    kind: marked,
                    at_equity,
                }),
            );
            h.participant = Some(Box::new(Liquidity {
                of_kind: kinds::FUND,
            }));
            h
        },
        // And a fund CALLS its commitments.
        {
            let called = says(
                "commitment.called",
                "the commitments each fund called from its investors",
                "29 B: a commitment is drawn when the fund needs it, which is a cash call with a date",
            );
            works(
                "private_equity",
                AT_H,
                &[],
                &[Produces(called)],
                Box::new(Calling {
                    kind: called,
                    draws: "fund.draws",
                }),
            )
        },
        // And a broker LENDS to a named client and sets what it requires.
        {
            let account = says(
                "broker.account",
                "what each client owes its broker, and what the broker requires",
                "31 A: a prime broker lends against a named client's position, and the requirement is the broker's own",
            );
            works(
                "prime_brokerage",
                AT_G4,
                &[],
                &[Produces(account)],
                Box::new(Broking {
                    kind: account,
                    could_move: "broker.could_move",
                    limit: "broker.limit",
                }),
            )
        },
        // And a pool publishes its NAV, and a holder subscribes at it.
        {
            let nav = says(
                "pool.nav",
                "the net asset value each pool published",
                "13 C: a subscription is at NAV, and the NAV is a read of what the pool holds",
            );
            works(
                "redeemable",
                AT_G4,
                &[],
                &[Produces(nav)],
                Box::new(Subscribing {
                    kind: nav,
                    commits: "subscribe.commits",
                }),
            )
        },
        {
            // And a company FLOATS. The shortfall it raises against was published a week ago
            // (Money G1.c), so this row needs nothing of the week it runs in.
            let floated = says(
                "equity.floated",
                "the shares a company brought to market and what it raised",
                "10 C: a flotation is a real offering with named subscribers, which either fills or does not",
            );
            let mut e = works(
                "equity",
                AT_D6,
                &[],
                &[Produces(floated)],
                Box::new(Floating {
                    kind: floated,
                    short_of_capital: kinds_row_short_of_capital,
                    at_short: at_ratio,
                    takes: "equity.takes",
                    firm_result: kinds_row_firm_result,
                    at_cash: at_firm_cash,
                    payout: "equity.payout",
                    programme: kinds_row_programme,
                    at_issue: at_programme_issue,
                }),
            );
            e.participant = Some(Box::new(Flotation {
                of_kind: kinds::FIRM,
            }));
            e
        },
        // And stock is LENT, at a fee that clears.
        {
            let lent = says(
                "stock.lent",
                "what was lent, to whom, and at what fee",
                "15 A: a short needs a borrow, and a borrow is a priced loan of a named line",
            );
            works(
                "securities_lending",
                AT_D5,
                &[],
                &[Produces(lent)],
                Box::new(StockLending { kind: lent }),
            )
        },
        // And a bank POOLS loans and cuts notes against them.
        {
            let cut = says(
                "pool.cut",
                "the notes a bank cut against a pool of loans",
                "46 A: a security is cut against named loans, and the junior note stands in front of the senior",
            );
            works(
                "securitisation",
                AT_D6,
                &[],
                &[Produces(cut)],
                Box::new(Securitising {
                    kind: cut,
                    junior: "pool.junior",
                    pools: "pool.pools",
                }),
            )
        },
        // ── The instrument families that settle against what the books printed ──────────────────
        // And a position MARKS, and an offset does not remove it.
        {
            let marked = says(
                "position.marked",
                "what each derivative position is worth now",
                "16 C: a position marks, and the mark is what margin is called against",
            );
            works(
                "derivative_layer",
                AT_G4,
                &[],
                &[Produces(marked)],
                Box::new(Derivatives {
                    kind: marked,
                    at_mark: at_the_mark,
                }),
            )
        },
        // And protection CLEARS between two parties who disagree.
        {
            let struck = says(
                "protection.struck",
                "the protection struck, on what name and at what spread",
                "19 A: protection is a contract between two parties who disagree about one name",
            );
            works(
                "cds",
                AT_B3,
                &[],
                &[Produces(struck)],
                Box::new(Protection {
                    kind: struck,
                    tenor: "protection.tenor",
                }),
            )
        },
        // And a currency pair CLEARS from real reasons.
        works(
            "spot_fx",
            AT_G3,
            &[],
            &[Produces(kinds_row_spot)],
            Box::new(SpotFx {
                kind: kinds_row_spot,
                at_base: at_fx_base,
                at_quote: at_fx_quote,
                reserve_mandates: Vec::new(),
            }),
        ),
        // And a forward is STRUCK — on the rate the pair cleared at THIS week, so it follows it.
        {
            let struck = says(
                "forward.struck",
                "the forwards struck, and on what spot",
                "20 A: a forward is struck on the rate the pair actually cleared at, never on a parity formula",
            );
            works(
                "fx_forwards",
                AT_G3,
                &[Produces(kinds_row_spot)],
                &[Produces(struck)],
                Box::new(FxForwards {
                    kind: struck,
                    spot: kinds_row_spot,
                    fixing: kinds_row_fixing,
                    tenor: "forward.tenor",
                }),
            )
        },
        // And a region's accounts are a READ of what actually crossed.
        {
            let accounts = says(
                "region.accounts",
                "what crossed each region's border, by account",
                "45 A: a region's accounts are a read of what actually crossed, never an exogenous series",
            );
            works(
                "cross_border",
                AT_G3,
                &[],
                &[Produces(accounts)],
                Box::new(CrossBorder {
                    kind: accounts,
                    at_current: keys_of_current,
                    at_financial: keys_of_financial,
                    at_valuation: keys_of_valuation,
                }),
            )
        },
        {
            let curve = says(
                "benchmarks.sovereign_curve",
                "the curve read off what sovereign paper transacted at",
                "22 C: a curve is a read of transacted prices, so a benchmark nobody traded does not exist",
            );
            works(
                "benchmarks",
                AT_G3,
                &[],
                &[Produces(kinds_row_fixing), Produces(curve)],
                Box::new(Fixes {
                    on: w.weekly_funding.map(book_of),
                    says: kinds_row_fixing,
                    sovereign_says: curve,
                }),
            )
        },
        {
            let observation = says(
                "index.observation",
                "what each index stood at, from its constituents",
                "23 D1: an index is a read of its constituents, and the level is never stored",
            );
            // 22 D4: it publishes every declared index, the consumer basket included, so there is
            // one writer of what an index stood at. The rents it weighs are struck this week.
            works(
                "indices",
                AT_G3,
                &[Produces(kinds_row_rent)],
                &[Produces(observation)],
                Box::new(PublishedIndices {
                    kind: observation,
                    at_index,
                    at_subject: at_index_subject,
                    at_level: at_index_level,
                    at_observed: at_index_observed,
                    at_base: at_index_base,
                    rent_kind: kinds_row_rent,
                    rent_key: 0,
                    rent_weight: 1.0,
                }),
            )
        },
        // And every house grades every name it can read.
        {
            let action = says(
                "ratings.action",
                "the grade each house now holds on a name, and what it was",
                "21 A4: two houses may disagree about one name, and a move is a restatement beside what was said before",
            );
            works(
                "ratings",
                AT_G6,
                &[],
                &[Produces(action)],
                Box::new(Grading {
                    kind: action,
                    accounts: kinds_row_accounts,
                    at_income,
                    best_carries: "ratings.best_carries",
                    per_notch: "ratings.per_notch",
                    without_a_record: "ratings.without_a_record",
                    record_after: "ratings.record_after",
                }),
            )
        },
        // And the accounts are PUBLISHED.
        works(
            "reporting",
            AT_G5,
            &[],
            &[Produces(kinds_row_accounts)],
            Box::new(Publishes {
                kind: kinds_row_accounts,
                at_equity,
                at_income,
                at_shares,
                at_closed,
                asymmetry: "reporting.asymmetry",
            }),
        ),
        // And every lender forms its OWN view of every borrower it holds.
        {
            let view = says(
                "lender.view",
                "each lender's own view of each borrower it holds",
                "12 B: a lender's opinion is its own, and a world with one opinion has no second side to a market",
            );
            works(
                "second_opinion",
                AT_G6,
                &[],
                &[Produces(view)],
                Box::new(SecondOpinion { kind: view }),
            )
        },
        // And the observer publishes a statistic — LATE, and revised.
        {
            let published = says(
                "statistic.published",
                "what the observer published, when, and what it revised",
                "Observer B: a statistic is late and revised, and the revision is part of the number",
            );
            works(
                "observer",
                AT_G7,
                &[],
                &[Produces(published)],
                Box::new(Observing {
                    kind: published,
                    at_about,
                    at_value,
                    at_revised,
                    lag: "observer.lag",
                }),
            )
        },
        // Every deciding party forms its own outlook from its own history.
        // THE ONE PLACE AN OUTLOOK IS FORMED, before anybody posts with it.
        works(
            "expectations",
            AT_E1,
            &[],
            &[],
            Box::new(Forming {
                firm_result: kinds_row_firm_result,
                at_cash: at_firm_cash,
            }),
        ),
        // ── The events that end things ──────────────────────────────────────────────────────────
        // And a loss is an EVENT.
        {
            let realised = says(
                "claim.loss.realised",
                "what each holder lost on a claim, and when",
                "12 C: a loss is an event with a date, a size and a named holder",
            );
            works(
                "loss",
                AT_B3,
                &[],
                &[Produces(kinds_row_loss_crossed), Produces(realised)],
                Box::new(Losses {
                    kind: kinds_row_loss_crossed,
                    loss_kind: realised,
                    at_standing,
                    at_loss,
                    impair_after: "loss.impair_after",
                    write_off_after: "loss.write_off_after",
                }),
            )
        },
        {
            // And the workout is OPENED.
            let opened = says(
                "workout.opened",
                "the workouts opened, and on what",
                "XI-8: a forced seller is a party in a workout, and a workout is a process with an end",
            );
            let mut f = works(
                "forced_sale",
                AT_H,
                &[],
                &[Produces(opened)],
                Box::new(ForcedSelling {
                    kind: opened,
                    within: "workout.within",
                }),
            );
            f.participant = Some(Box::new(ForcedSeller {
                kind: says(
                "workout.sold",
                "what a forced seller sold, and at what",
                "XI-8: what a forced sale fetched is a price like any other, and it is what the estate pays from",
            ),
                of_kind: kinds::FUND,
            }));
            f
        },
        // A party whose liabilities exceed its assets CEASES.
        works(
            "mortality",
            AT_B4,
            &[],
            &[Produces(kinds_row_mortality_failed)],
            Box::new(Failing {
                says: kinds_row_mortality_failed,
                loss_crossed: kinds_row_loss_crossed,
                at_standing,
                at_trigger: at_failure_trigger,
                at_destination: at_failure_destination,
                dissolved: kinds_row_dissolved,
                past_waterfall: kinds_row_past_waterfall,
                funding_failed: kinds_row_funding_failed,
            }),
        ),
        {
            let paid = says(
                "estate.paid",
                "what each estate paid, to whom, and at what rank",
                "XI-8: an estate pays in rank order, and what it did not pay is a named holder's loss",
            );
            works(
                "estate",
                AT_B5,
                &[],
                &[Produces(paid)],
                Box::new(Ranked { says: paid }),
            )
        },
        // §35, §29 B: and a company is BID FOR, and the owners decide.
        {
            let tender = says(
                "control.tender",
                "the tenders made for a company, and what the owners did",
                "35 B: control changes when the owners accept, and each one decides for itself",
            );
            works(
                "control",
                AT_H,
                &[],
                &[Produces(tender)],
                Box::new(Control {
                    kind: tender,
                    hurdle: "acquirer.hurdle",
                    needs: "control.needs",
                }),
            )
        },
        // And the term RUNS OUT.
        {
            let called = says(
                "election.called",
                "the elections called, and when they fall",
                "47 B: a term runs out on a date, and the election is what follows from it",
            );
            works(
                "polity",
                AT_H,
                &[],
                &[Produces(called)],
                Box::new(Elections {
                    kind: called,
                    term: "parliament.term",
                    takes: "election.takes",
                }),
            )
        },
    ];
    // Each declaration gets its OWN slot, so no two systems declare the same phase.
    for (n, row) in rows.iter_mut().enumerate() {
        row.slot = FIRST_SLOT + n as u32;
    }
    rows
}

// THE WIRING IS A CONTRACT NOW, not a test. `World::wire_up` is the one door a system comes in
// through, and it refuses a name wired twice and a row that neither works nor posts; `Phases::add`
// refuses a phase declared twice. Every world enforces all three on every run, where a test
// enforced them on one built for the purpose. How many systems there are is a CENSUS, printed by
// `world:runs` rather than asserted — a number nobody may change without seeing it change.
//
// What the participants here do is another matter, and it is 0m2.4's and 0r's. `Dealers::orders`
// computes a skewed bid inline and never calls `dealing::quote`, which is in the module and has
// its own logic-level tests; the same is true of the bank's buffer, the fund's mandate and the
// pool with no manager. The fixtures asserted the COPY. Moving each participant into its system's
// module deletes the copy, and then the claim — a dealer long bids lower, a limit that never binds
// is not a limit, who lends is an outcome and not a rule — is a test over values in that module.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_row_that_only_posts_declares_no_phase() {
        // Money G2.e: a participant is asked at e2 by the kernel, so a row whose only act is to
        // post has no stage of its own — and a phase that ran no mechanism ran nothing.
        let p = posts(
            "stockists",
            Box::new(Stockist {
                lines: Vec::new(),
                carrying: "goods.seller.holding_costs",
                limit: "stockist.limit",
            }),
        );
        assert_eq!(p.participants().len(), 1);
        assert!(p.phases().is_empty());
    }
}
