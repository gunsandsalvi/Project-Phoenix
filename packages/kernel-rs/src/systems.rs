//! THE SYSTEMS, WIRED. Every spec system this world has, in one list, with the participants that
//! have a reason to post and the phases that say when they run.
//!
//! @spec 3 B2 · 37 C1 · 39 · 11 B1 · 10 B1 · 7 D1 · 40 B1 · 26 A2 · ARCHITECTURE 4.9b · Law 3,
//! @spec Law 6, Law 15, Law 19 · Appendix B

use crate::assembly::{kinds, phase, System, AT_JUDGED, AT_OWED, AT_SCHEDULED, AT_VIEWS, AT_WORK};
use crate::ids::book_of;
use crate::ids::InstrumentId;
use crate::mechanisms::bank_capital::BankCapital;
use crate::mechanisms::bank_funding::BankFunding;
use crate::mechanisms::benchmarks::Fixes;
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
use crate::mechanisms::freight::Carriage;
use crate::mechanisms::freight::LetsItsPlant;
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
use crate::mechanisms::money_market::Credit;
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
use crate::mechanisms::sovereign::Sovereign;
use crate::mechanisms::spot_fx::SpotFx;
use crate::mechanisms::trade_credit::TradeCredit;
use crate::mechanisms::treasury::Funding;
use crate::mechanisms::treasury::TreasuryIssues;
use crate::module::{Mechanism, Participant};
use crate::params::{Denomination, Dimension, Kind, Owner, ParamDecl, Params};
use crate::registry::Registry;
use crate::world::PhaseDecl;

/// The nine stages own the first nine slots, so a system's own starts after them.
pub const FIRST_SLOT: u32 = 9;

/// One system, its name, its phases and whoever it puts in a book.
pub struct Wired {
    pub name: &'static str,
    /// Its own declaration slot, so two systems cannot declare the same phase.
    pub slot: u32,
    /// Which of the nine stages its MECHANISM runs in. A participant is not placed by this: it is
    /// collected for the books and posts at VIEWS whatever the row says, because posting is the
    /// market door rather than a place in the period.
    pub at: u32,
    pub participant: Option<Box<dyn Participant>>,
    /// ARCHITECTURE 4.9b: its own work in the period, if it has any of its own.
    pub mechanism: Option<Box<dyn Mechanism>>,
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

    fn phases(&self) -> Vec<PhaseDecl> {
        // The phase's name is its own declaration slot; the owner is the system itself.
        vec![phase(self.slot, self.slot, self.at)]
    }

    fn participants(&self) -> Vec<&dyn Participant> {
        match &self.participant {
            Some(p) => vec![p.as_ref()],
            None => Vec::new(),
        }
    }
}

/// A system that reads rather than posts.
pub fn reads(name: &'static str, at: u32) -> Wired {
    // The slot is assigned by `all`, which is the one place that knows the order.
    Wired {
        name,
        slot: 0,
        at,
        participant: None,
        mechanism: None,
        audits: Vec::new(),
    }
}

/// A system that has its OWN WORK in the period and no reason to be in a book: it reads the world
/// through the second door and proposes.
pub fn works(name: &'static str, at: u32, mechanism: Box<dyn Mechanism>) -> Wired {
    Wired {
        name,
        slot: 0,
        at,
        participant: None,
        mechanism: Some(mechanism),
        audits: Vec::new(),
    }
}

/// A system that puts somebody in a book. Its participant posts at VIEWS; the stage is where its
/// own work runs, and a row that only posts has none.
pub fn posts(name: &'static str, at: u32, participant: Box<dyn Participant>) -> Wired {
    Wired {
        name,
        slot: 0,
        at,
        participant: Some(participant),
        mechanism: None,
        audits: Vec::new(),
    }
}

/// The lines each system needs, NAMED. How a good is MADE is not here: that is data, and it lives
/// in the registry with everything else the ids point at.
pub struct Wiring {
    /// What funds, insurers and dealers may hold.
    pub lines: Vec<InstrumentId>,
    /// The overnight book's subject.
    pub overnight: Option<InstrumentId>,
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

/// Every system this world has, and every one of them RUNS.
pub fn declare(p: &mut Params) {
    let mut say = |id: &str,
                   value: f64,
                   unit: &str,
                   dimension: Dimension,
                   kind: Kind,
                   owner: Owner,
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
        45.0,
        "days after the books close",
        Dimension::Days,
        Kind::Technology,
        Owner::StandardSetter,
        "the days between a company's quarter-end and the day its accounts are published",
    );
    // How long a holder has to sell what its mandate no longer lets it hold.
    say(
        "funding.now",
        0.0,
        "days ahead",
        Dimension::Days,
        Kind::Resolution,
        Owner::Model,
        "the near end of a funding window, which is today",
    );
    say(
        "funding.this_period",
        7.0,
        "days ahead",
        Dimension::Days,
        Kind::Resolution,
        Owner::Model,
        "the days a working-capital shortfall is read over, which is one period",
    );
    say(
        "funding.this_year",
        365.0,
        "days ahead",
        Dimension::Days,
        Kind::Resolution,
        Owner::Model,
        "the days a long-term shortfall is read over, which is a year",
    );
    // Commercial paper's own convention, which is what makes it a different
    // instrument from a bond rather than the same one with a different number in it.
    say(
        "paper.tenor",
        3.0,
        "months the paper runs",
        Dimension::Months,
        Kind::Technology,
        Owner::StandardSetter,
        "how long the commercial paper a borrower brings runs for, advanced as MONTHS of calendar",
    );
    say(
        "firm.buffer",
        10.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "the cash a borrower that is not the state keeps back beyond what falls due",
    );
    // The seller's own limits, which is what makes terms a decision rather than a rule.
    say(
        "trade_credit.will_carry",
        500.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "how much a seller will have out to one buyer at once before it stops offering terms",
    );
    say(
        "trade_credit.will_wait",
        30.0,
        "days",
        Dimension::Days,
        Kind::Preference,
        Owner::Model,
        "how long a seller will wait to be paid",
    );
    // How many shares a line comes into existence with.
    say(
        "lender.hurdle",
        0.05,
        "per unit lent",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the return a lender wants on what it puts out, which its standard is read against",
    );
    say(
        "bank.min_weighted",
        0.08,
        "capital per unit of weighted assets",
        Dimension::Ratio,
        Kind::Policy,
        Owner::Parliament,
        "the capital a bank must hold against its risk-weighted assets",
    );
    say(
        "bank.min_leverage",
        0.03,
        "capital per unit of assets",
        Dimension::Ratio,
        Kind::Policy,
        Owner::Parliament,
        "the backstop: capital against total assets, whatever they weigh",
    );
    say("bank.buffer", 0.025, "capital per unit above the requirement", Dimension::Ratio, Kind::Policy, Owner::Parliament,
        "the buffer a bank is expected to keep above its requirement, inside which there are consequences short of a breach");
    // The weight schedule is the regulator's, and it is two numbers rather than one per rung.
    say(
        "bank.on_the_best",
        1.01,
        "per unit carried",
        Dimension::Ratio,
        Kind::Policy,
        Owner::Parliament,
        "what a bank must hold against a claim on the best credit there is, per unit it carries",
    );
    say(
        "bank.per_notch",
        1.07,
        "per unit carried, per notch",
        Dimension::Ratio,
        Kind::Policy,
        Owner::Parliament,
        "how much more a bank must hold for each notch further down the scale a name is graded",
    );
    say(
        "bank.ungraded_at",
        7.0,
        "notches below the best",
        Dimension::Count,
        Kind::Policy,
        Owner::Parliament,
        "where on the scale a bank must weight a name nobody has graded",
    );
    // And a house's own scale, from which its twenty-two band edges fall out.
    say(
        "ratings.best_carries",
        0.25,
        "strain",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the strain a name at the top of a house's scale already carries",
    );
    say(
        "ratings.per_notch",
        0.25,
        "strain per notch",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the strain one notch of a house's scale is worth",
    );
    say(
        "ratings.without_a_record",
        3.0,
        "notches",
        Dimension::Count,
        Kind::Preference,
        Owner::Model,
        "the notches a house marks down a name it has no history for",
    );
    say(
        "ratings.record_after",
        8.0,
        "periods",
        Dimension::Periods,
        Kind::Preference,
        Owner::Model,
        "how long a name must have existed before a house reads its numbers as a record",
    );
    // The mix a company would raise at.
    say(
        "fund.draws",
        0.05,
        "per unit uncalled",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the share of an uncalled commitment a fund draws in one period",
    );
    say(
        "small_business.reaches",
        5000.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Technology,
        Owner::StandardSetter,
        "the size at which a borrower can reach the bond market instead of a bank",
    );
    say(
        "acquirer.hurdle",
        0.1,
        "per unit paid",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the return an acquirer wants on what it pays for a company",
    );
    say(
        "control.needs",
        0.5,
        "per unit outstanding",
        Dimension::Ratio,
        Kind::Technology,
        Owner::StandardSetter,
        "how much of the shares it does not already hold a tender must reach",
    );
    say(
        "pool.junior",
        0.1,
        "per unit of the pool",
        Dimension::Ratio,
        Kind::Technology,
        Owner::StandardSetter,
        "the share of a pool that stands in front of its senior note",
    );
    say(
        "pool.pools",
        0.2,
        "per unit of its loan book",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "how much of its loan book a bank pools at once",
    );
    say(
        "dwelling.upkeep",
        0.02,
        "money per dwelling per period",
        Dimension::PricePerUnit,
        Kind::Technology,
        Owner::StandardSetter,
        "what keeping one dwelling in repair costs its owner each period",
    );
    say(
        "household.will_spend",
        0.5,
        "per unit of its money",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the share of what it holds a household will put towards a roof",
    );
    say(
        "storage.per_unit",
        0.01,
        "money per unit per period",
        Dimension::PricePerUnit,
        Kind::Technology,
        Owner::StandardSetter,
        "what holding one unit of a physical good for one period costs",
    );
    say(
        "subscribe.commits",
        0.1,
        "per unit of spare cash",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "the share of its spare money a holder commits to one pool",
    );
    // The broker's own view and its own limit.
    say(
        "broker.could_move",
        0.2,
        "per unit of the book",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "what a broker thinks a client's book could move against it in a period",
    );
    say(
        "broker.limit",
        100000.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "what one broker will be exposed to one client for",
    );
    say(
        "forward.tenor",
        90.0,
        "days",
        Dimension::Days,
        Kind::Technology,
        Owner::StandardSetter,
        "how far out a currency forward is struck",
    );
    say(
        "protection.tenor",
        5.0,
        "years",
        Dimension::Years,
        Kind::Technology,
        Owner::StandardSetter,
        "how long a protection contract runs",
    );
    say(
        "observer.lag",
        2.0,
        "periods",
        Dimension::Periods,
        Kind::Technology,
        Owner::StandardSetter,
        "the periods between what a statistic is about and the period it is published in",
    );
    say(
        "invest.horizon",
        20.0,
        "periods",
        Dimension::Periods,
        Kind::Preference,
        Owner::Model,
        "how many periods of return a management counts when it weighs a project",
    );
    say(
        "invest.hurdle",
        0.02,
        "per unit above the cost of capital",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "what a management wants above its cost of capital before it commits",
    );
    say(
        "invest.takes",
        3.0,
        "periods",
        Dimension::Periods,
        Kind::Technology,
        Owner::StandardSetter,
        "the periods a capital programme runs before the plant is in service",
    );
    say(
        "equity.takes",
        4.0,
        "periods",
        Dimension::Periods,
        Kind::Technology,
        Owner::StandardSetter,
        "the periods a flotation stands before it is over, one way or the other",
    );
    say(
        "parliament.seats",
        100.0,
        "seats",
        Dimension::Count,
        Kind::Policy,
        Owner::Constitution,
        "how many seats the parliament of a country has",
    );
    say(
        "parliament.term",
        1460.0,
        "days",
        Dimension::Days,
        Kind::Policy,
        Owner::Constitution,
        "the days between elections, placed by DATE and never a count of periods",
    );
    say(
        "election.takes",
        1.0,
        "periods",
        Dimension::Periods,
        Kind::Technology,
        Owner::StandardSetter,
        "the periods between an election being called and its result being known",
    );
    say(
        "workout.within",
        2.0,
        "periods",
        Dimension::Periods,
        Kind::Technology,
        Owner::StandardSetter,
        "the periods a holder has to sell a line its mandate no longer lets it hold",
    );
    say(
        "loss.impair_after",
        2.0,
        "periods non-performing",
        Dimension::Periods,
        Kind::Policy,
        Owner::StandardSetter,
        "how long a finally failed claim remains non-performing before impairment",
    );
    say(
        "loss.write_off_after",
        2.0,
        "periods impaired",
        Dimension::Periods,
        Kind::Policy,
        Owner::StandardSetter,
        "how long an impaired claim remains unresolved before write-off",
    );
    say("building.crowds_at", 60.0, "square km standing", Dimension::SquareKm, Kind::Technology, Owner::Model,
        "the ground already covered in a place at which building there draws twice what it does on empty ground");
    // 37 B1, 22c.3: how much cover a firm wants on its shelf.
    say(
        "firm.cover",
        0.5,
        "multiple of what it expects to sell",
        Dimension::Ratio,
        Kind::Preference,
        Owner::Model,
        "how much stock a firm wants on the shelf beyond the week it expects to sell",
    );
    // 37 C1, 22c.3: what another period on the shelf costs the holder, as a share of what the units
    // cost it — the storage, the spoilage and the money tied up.
    say("plant.upkeep", 0.5, "money", Dimension::Amount(Denomination::Money), Kind::Technology, Owner::Model,
        "what keeping a plant costs its owner a period whether or not anybody books it — 33 A4.b's fixed cost");
    say(
        "stockist.limit",
        50.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "the most a stockist will carry of one line — without one it is the buyer of last resort",
    );
    say("goods.seller.holding_costs", 0.03, "share of what the units cost, a period", Dimension::Ratio, Kind::Technology, Owner::Model,
        "what it costs to keep a unit another period: the room it takes, what spoils and the money in it");
    // A fact about the thing, not about who holds it.
    say(
        "goods.perishes",
        0.01,
        "share of a lot a period",
        Dimension::Ratio,
        Kind::Technology,
        Owner::Model,
        "the share of a lot that does not survive the period",
    );
    // The money a household keeps back.
    say("household.keeps", 1.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the balance a household holds on to rather than spends, which is why its money is not a trend");
    // A bank's own liquidity buffer, and what it lends and borrows at overnight.
    say(
        "money_market.buffer",
        1.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "the balance a bank keeps back before it lends overnight",
    );
    say(
        "money_market.lends_at",
        1.0,
        "per annum",
        Dimension::PerAnnum,
        Kind::Preference,
        Owner::Model,
        "the rate a bank will lend overnight at",
    );
    say(
        "money_market.borrows_at",
        1.0,
        "per annum",
        Dimension::PerAnnum,
        Kind::Preference,
        Owner::Model,
        "the rate a bank will borrow overnight at",
    );
    say(
        "central_bank.facility_advance",
        0.8,
        "share of collateral market value",
        Dimension::Ratio,
        Kind::Policy,
        Owner::CentralBank,
        "the share of eligible collateral value the central bank advances at its standing facility",
    );
    say(
        "central_bank.facility_penalty",
        0.02,
        "per annum over the market",
        Dimension::PerAnnum,
        Kind::Policy,
        Owner::CentralBank,
        "the standing facility penalty over the observed money-market rate",
    );
    // A desk's own inventory constraint. Its price and width are decisions, not parameters.
    say(
        "dealer.limit",
        10.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Model,
        "the most a desk will hold of one line",
    );
    // The shape is dead.
    say(
        "treasury.buffer",
        200.0,
        "money",
        Dimension::Amount(Denomination::Money),
        Kind::Preference,
        Owner::Parliament,
        "the balance the treasury keeps back, which is why one failed auction is not a default",
    );
    // 5 C3.a, 21j.1a: the tenor and the coupon paper is BROUGHT at.
    say(
        "funding.tenor",
        60.0,
        "months the paper runs",
        Dimension::Months,
        Kind::Technology,
        Owner::StandardSetter,
        "how long the paper an issuer brings runs for, advanced as MONTHS of calendar — the tenor \
         its market quotes, and what makes a five-year line five years rather than 260 weeks",
    );
}

pub fn all(w: &Wiring, r: &Registry, journal: &mut crate::journal::Journal) -> Vec<Wired> {
    let keys_of = |j: &mut crate::journal::Journal, name: &str| j.keys_named.declare(name);
    let at_equity = keys_of(journal, "accounts.equity");
    let at_income = keys_of(journal, "accounts.income");
    let at_firm_revenue = keys_of(journal, "firm.revenue");
    let at_firm_costs = keys_of(journal, "firm.costs");
    let at_firm_cash = keys_of(journal, "firm.operating_cash");
    let at_programme_funding = keys_of(journal, "programme.funding");
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
    let kinds = &mut journal.kinds;
    // The kind the accounts are published under, read back by whatever reads them — the grades do.
    let kinds_row_accounts = kinds.declare("accounts.published");
    // A bank short of capital says so, and the equity row acts on it — one writer of a company's
    // shares, two reasons to issue them.
    let kinds_row_short_of_capital = kinds.declare("bank.short_of_capital");
    // The benchmark fixing, read by whatever a market rate reaches.
    let kinds_row_fixing = kinds.declare("benchmarks.fixing");
    // What a company's capital costs it, read by whatever a hurdle reaches.
    let kinds_row_costs = kinds.declare("capital.costs");
    // The rate a pair cleared at, read by the forward that is a rate forward OF it.
    let kinds_row_spot = kinds.declare("spot.rate");
    let kinds_row_loss_crossed = kinds.declare("claim.crossed");
    let kinds_row_dissolved = kinds.declare("household.dissolved");
    let kinds_row_past_waterfall = kinds.declare("clearing.waterfall.exhausted");
    let kinds_row_mortality_failed = kinds.declare("mortality.failed");
    let kinds_row_funding_failed = kinds.declare("bank.funding.failed");
    let kinds_row_facility_drawn = kinds.declare("bank.facility.drawn");
    let kinds_row_firm_result = kinds.declare("firm.result");
    let kinds_row_programme = kinds.declare("plant.built");
    // One event kind per system that publishes a read.
    let mut says = |name: &str| kinds.declare(name);
    let mut rows = vec![
        {
            // §37 both posts and works: the stock that does not survive the period leaves at what
            // it cost, and then a firm offers what is left of what it holds.
            let mut goods = posts(
                "goods",
                AT_WORK,
                Box::new(GoodsSellers {
                    holding_costs: "goods.seller.holding_costs",
                    keeps: keeps(r),
                }),
            );
            goods.mechanism = Some(Box::new(crate::mechanisms::goods::Perishing {
                share: "goods.perishes",
            }));
            goods
        },
        // A cell bids for what it can fund. Its outlook is the `expectations` row's, once, at VIEWS:
        // §46's memory is ONE preference over a party's own history, and applied twice a period it
        // is a different preference.
        posts(
            "households",
            AT_VIEWS,
            Box::new(HouseholdBuyers {
                keeps: "household.keeps",
                basket: basket(r),
            }),
        ),
        // THE ONE SYSTEM THAT MAKES ANYTHING.
        works(
            "recipe",
            AT_WORK,
            Box::new(Making {
                flow: CostFlow::FirstInFirstOut,
                crowds_at: "building.crowds_at",
                cover: "firm.cover",
            }),
        ),
        works(
            "firms",
            AT_JUDGED,
            Box::new(Reporting {
                kind: kinds_row_firm_result,
                at_revenue: at_firm_revenue,
                at_costs: at_firm_costs,
                at_cash: at_firm_cash,
                at_equity,
            }),
        ),
        works("employment", AT_WORK, Box::new(Wages)),
        // A quay's owner earns what a berth clears at.
        {
            // A quay's owner earns what a berth clears at.
            let mut f = posts(
                "freight",
                AT_WORK,
                Box::new(LetsItsPlant {
                    lines: plants(r),
                    upkeep: "plant.upkeep",
                }),
            );
            f.mechanism = Some(Box::new(Carriage {
                kind: says("freight.carriage"),
            }));
            f
        },
        // And stock is TIGHT or it is not, and storing it costs money to somebody.
        works(
            "commodities",
            AT_WORK,
            Box::new(Storing {
                kind: says("stock.tightness"),
                per_unit: "storage.per_unit",
            }),
        ),
        // Somebody whose business is to hold the stock.
        posts(
            "stockists",
            AT_VIEWS,
            Box::new(Stockist {
                lines: basket(r),
                carrying: "goods.seller.holding_costs",
                limit: "stockist.limit",
            }),
        ),
        // And dwellings are LET and SOLD, and both prices clear.
        works(
            "housing",
            AT_WORK,
            Box::new(Housing {
                kind: says("dwelling.sold"),
                lets: says("dwelling.let"),
                upkeep: "dwelling.upkeep",
                will_spend: "household.will_spend",
            }),
        ),
        // And a seller that has delivered and not been paid OFFERS TERMS.
        works(
            "trade_credit",
            AT_WORK,
            Box::new(TradeCredit {
                kind: says("invoice.struck"),
                will_carry: "trade_credit.will_carry",
                will_wait: "trade_credit.will_wait",
            }),
        ),
        // And the tier too small for the bond market is READ.
        works(
            "small_business",
            AT_WORK,
            Box::new(SmallBusiness {
                kind: says("small_business.state"),
                reaches_the_bond_market_at: "small_business.reaches",
            }),
        ),
        {
            let mut mm = posts(
                "money_market",
                AT_JUDGED,
                Box::new(MoneyMarketBanks {
                    buffer: "money_market.buffer",
                    lends_at: "money_market.lends_at",
                    borrows_at: "money_market.borrows_at",
                    book: w.overnight.map(book_of),
                }),
            );
            mm.mechanism = Some(Box::new(Credit {
                kind: says("money_market.credit"),
            }));
            mm
        },
        {
            // It BRINGS the paper in the week's work and AUCTIONS it when the books clear, because
            // a bill has to exist before anybody bids for it.
            let mut t = posts(
                "treasury",
                AT_WORK,
                Box::new(TreasuryIssues {
                    paper: w.paper,
                    buffer: "treasury.buffer",
                }),
            );
            t.mechanism = Some(Box::new(Funding {
                // The SOVEREIGN's own paper, and nobody else's.
                of_kinds: &[kinds::TREASURY],
                after: "funding.now",
                horizon: "funding.this_period",
                tenor: "funding.tenor",
                buffer: "treasury.buffer",
                says: says("funding.brought"),
            }));
            t
        },
        // Money A1, 5 A4: every asset is somebody's liability, published party by party.
        works(
            "money",
            AT_JUDGED,
            Box::new(crate::mechanisms::money::Owed {
                kind: says("money.owed"),
            }),
        ),
        // And a treasury HANDLES being short.
        works(
            "sovereign",
            AT_JUDGED,
            Box::new(Sovereign {
                kind: says("sovereign.shortfall"),
            }),
        ),
        // And a bank READS its own capital.
        works(
            "bank_capital",
            AT_JUDGED,
            Box::new(BankCapital {
                kind: says("bank.capital"),
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
        ),
        // And a bank SETS the rate it pays on deposits.
        works(
            "bank_funding",
            AT_JUDGED,
            Box::new(BankFunding {
                kind: says("deposit.rate"),
                failed: kinds_row_funding_failed,
                at_short: at_funding_short,
                buffer: "money_market.buffer",
                facility_advance: "central_bank.facility_advance",
                facility_penalty: "central_bank.facility_penalty",
                facility_drawn: kinds_row_facility_drawn,
                at_rate: at_funding_rate,
                fixing: kinds_row_fixing,
            }),
        ),
        // THE ONE THE WHOLE CREDIT SIDE RESTS ON — what falls due is paid, or it is an arrear.
        works("lending", AT_OWED, Box::new(Servicing)),
        {
            // Audit C3, 33 A6.b, 22e: PLANT MOVES ONLY FOR A REASON, and this is the one module
            // family this world has.
            let capital = capital(r);
            // And a firm DECIDES to invest.
            let mut cp = works(
                "capital_programme",
                AT_WORK,
                Box::new(Building {
                    kind: kinds_row_programme,
                    at_funding: at_programme_funding,
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
            AT_JUDGED,
            Box::new(CostOfCapital {
                kind: kinds_row_costs,
                accounts: kinds_row_accounts,
                at_income,
                at_shares,
            }),
        ),
        // A borrower short over the WEEK brings commercial paper.
        works(
            "short_term_debt",
            AT_WORK,
            Box::new(BringsPaper {
                of_kinds: &[kinds::BANK, kinds::FIRM, kinds::SMALL_FIRM],
                after: "funding.now",
                horizon: "funding.this_period",
                tenor: "paper.tenor",
                buffer: "firm.buffer",
                says: says("paper.brought"),
                programme: Some(kinds_row_programme),
                at_programme_funding: Some(at_programme_funding),
            }),
        ),
        // And a borrower short over the YEAR brings a bond.
        works(
            "corporate_credit",
            AT_WORK,
            Box::new(BringsBond {
                of_kinds: &[kinds::FIRM, kinds::BANK],
                after: "funding.this_period",
                horizon: "funding.this_year",
                tenor: "funding.tenor",
                buffer: "firm.buffer",
                says: says("bond.brought"),
            }),
        ),
        {
            let mut f = posts(
                "funds",
                AT_VIEWS,
                Box::new(FundMandates {
                    may_hold: w.lines.clone(),
                }),
            );
            // A pool whose manager died posts nothing and winds up.
            f.mechanism = Some(Box::new(Winding {
                says: says("fund.orphaned"),
                ceased: kinds_row_mortality_failed,
                at_trigger: at_failure_trigger,
                at_destination: at_failure_destination,
            }));
            f
        },
        {
            let mut i = posts(
                "insurers",
                AT_WORK,
                Box::new(InsurerMatching {
                    long_lines: w.lines.clone(),
                }),
            );
            i.mechanism = Some(Box::new(Policies {
                kind: says("insurers.policies"),
            }));
            i
        },
        {
            let mut d = posts(
                "dealing",
                AT_JUDGED,
                Box::new(Dealers {
                    limit: "dealer.limit",
                    lines: w.lines.clone(),
                }),
            );
            d.mechanism = Some(Box::new(Lines {
                kind: says("dealing.lines"),
            }));
            d
        },
        {
            // And a fund is the BUYER when others are forced sellers.
            let mut h = works(
                "hedge_funds",
                AT_JUDGED,
                Box::new(Levering {
                    kind: says("fund.marked"),
                    at_equity,
                }),
            );
            h.participant = Some(Box::new(Liquidity {
                of_kind: kinds::FUND,
            }));
            h
        },
        // And a fund CALLS its commitments.
        works(
            "private_equity",
            AT_SCHEDULED,
            Box::new(Calling {
                kind: says("commitment.called"),
                draws: "fund.draws",
            }),
        ),
        // And a broker LENDS to a named client and sets what it requires.
        works(
            "prime_brokerage",
            AT_JUDGED,
            Box::new(Broking {
                kind: says("broker.account"),
                could_move: "broker.could_move",
                limit: "broker.limit",
            }),
        ),
        // And a pool publishes its NAV, and a holder subscribes at it.
        works(
            "redeemable",
            AT_JUDGED,
            Box::new(Subscribing {
                kind: says("pool.nav"),
                commits: "subscribe.commits",
            }),
        ),
        {
            // And a company FLOATS.
            let mut e = works(
                "equity",
                AT_WORK,
                Box::new(Floating {
                    kind: says("equity.floated"),
                    short_of_capital: kinds_row_short_of_capital,
                    at_short: at_ratio,
                    takes: "equity.takes",
                }),
            );
            e.participant = Some(Box::new(Flotation {
                of_kind: kinds::FIRM,
            }));
            e
        },
        // And stock is LENT, at a fee that clears.
        works(
            "securities_lending",
            AT_WORK,
            Box::new(StockLending {
                kind: says("stock.lent"),
            }),
        ),
        // And a bank POOLS loans and cuts notes against them.
        works(
            "securitisation",
            AT_WORK,
            Box::new(Securitising {
                kind: says("pool.cut"),
                junior: "pool.junior",
                pools: "pool.pools",
            }),
        ),
        // ── The instrument families that settle against what the books printed ──────────────────
        // And a position MARKS, and an offset does not remove it.
        works(
            "derivative_layer",
            AT_JUDGED,
            Box::new(Derivatives {
                kind: says("position.marked"),
                at_mark: at_the_mark,
            }),
        ),
        // And protection CLEARS between two parties who disagree.
        works(
            "cds",
            AT_OWED,
            Box::new(Protection {
                kind: says("protection.struck"),
                tenor: "protection.tenor",
            }),
        ),
        // And a forward is STRUCK.
        works(
            "fx_forwards",
            AT_JUDGED,
            Box::new(FxForwards {
                kind: says("forward.struck"),
                spot: kinds_row_spot,
                fixing: kinds_row_fixing,
                tenor: "forward.tenor",
            }),
        ),
        // And a currency pair CLEARS from real reasons.
        works(
            "spot_fx",
            AT_JUDGED,
            Box::new(SpotFx {
                kind: kinds_row_spot,
            }),
        ),
        // And a region's accounts are a READ of what actually crossed.
        works(
            "cross_border",
            AT_JUDGED,
            Box::new(CrossBorder {
                kind: says("region.accounts"),
                at_current: keys_of_current,
                at_financial: keys_of_financial,
            }),
        ),
        works(
            "benchmarks",
            AT_JUDGED,
            Box::new(Fixes {
                on: w.overnight.map(book_of),
                says: kinds_row_fixing,
            }),
        ),
        // And every house grades every name it can read.
        works(
            "ratings",
            AT_JUDGED,
            Box::new(Grading {
                kind: says("ratings.action"),
                accounts: kinds_row_accounts,
                at_income,
                best_carries: "ratings.best_carries",
                per_notch: "ratings.per_notch",
                without_a_record: "ratings.without_a_record",
                record_after: "ratings.record_after",
            }),
        ),
        // And the accounts are PUBLISHED.
        works(
            "reporting",
            AT_JUDGED,
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
        works(
            "second_opinion",
            AT_JUDGED,
            Box::new(SecondOpinion {
                kind: says("lender.view"),
            }),
        ),
        // And the observer publishes a statistic — LATE, and revised.
        works(
            "observer",
            AT_JUDGED,
            Box::new(Observing {
                kind: says("statistic.published"),
                at_about,
                at_value,
                at_revised,
                lag: "observer.lag",
            }),
        ),
        // Every deciding party forms its own outlook from its own history.
        // THE ONE PLACE AN OUTLOOK IS FORMED, before anybody posts with it.
        works(
            "expectations",
            AT_VIEWS,
            Box::new(Forming {
                firm_result: kinds_row_firm_result,
                at_cash: at_firm_cash,
            }),
        ),
        // ── The events that end things ──────────────────────────────────────────────────────────
        // And a loss is an EVENT.
        works(
            "loss",
            AT_OWED,
            Box::new(Losses {
                kind: kinds_row_loss_crossed,
                loss_kind: says("claim.loss.realised"),
                at_standing,
                at_loss,
                impair_after: "loss.impair_after",
                write_off_after: "loss.write_off_after",
            }),
        ),
        {
            // And the workout is OPENED.
            let mut f = works(
                "forced_sale",
                AT_SCHEDULED,
                Box::new(ForcedSelling {
                    kind: says("workout.opened"),
                    within: "workout.within",
                }),
            );
            f.participant = Some(Box::new(ForcedSeller {
                kind: says("workout.sold"),
                of_kind: kinds::FUND,
            }));
            f
        },
        // A party whose liabilities exceed its assets CEASES.
        works(
            "mortality",
            AT_OWED,
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
        works(
            "estate",
            AT_OWED,
            Box::new(Ranked {
                says: says("estate.paid"),
            }),
        ),
        // §35, §29 B: and a company is BID FOR, and the owners decide.
        works(
            "control",
            AT_SCHEDULED,
            Box::new(Control {
                kind: says("control.tender"),
                hurdle: "acquirer.hurdle",
                needs: "control.needs",
            }),
        ),
        // And the term RUNS OUT.
        works(
            "polity",
            AT_SCHEDULED,
            Box::new(Elections {
                kind: says("election.called"),
                term: "parliament.term",
                takes: "election.takes",
            }),
        ),
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
    fn a_system_that_reads_posts_nothing() {
        // No demand added to clear: a read is not a party with a reason to be in a book.
        let r = reads("benchmarks", AT_JUDGED);
        assert!(r.participants().is_empty());
        assert_eq!(r.phases().len(), 1);
    }
}
