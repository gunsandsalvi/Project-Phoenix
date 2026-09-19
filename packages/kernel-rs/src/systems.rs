//! THE SYSTEMS, WIRED. Every spec system this world has, in one list, with the participants that
//! have a reason to post and the phases that say when they run.
//!
//! @spec 3 B2 · 37 C1 · 39 · 11 B1 · 10 B1 · 7 D1 · 40 B1 · 26 A2 · ARCHITECTURE 4.9b · Law 3,
//! @spec Law 6, Law 15, Law 19 · Appendix B

use crate::assembly::{kinds, phase, System, AT_MARKETS, AT_REVALUATION};
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{InstrumentId, MarketId};
use crate::module::{Mechanism, Participant, ParticipantView};
use crate::params::{Denomination, Dimension, Kind, Owner, ParamDecl, Params};
use crate::mechanisms::cds::Protection;
use crate::mechanisms::employment::Wages;
use crate::mechanisms::expectations::Forming;
use crate::mechanisms::firms::Reporting;
use crate::mechanisms::mortality::Failing;
use crate::mechanisms::polity::Elections;
use crate::mechanisms::hedge_funds::Levering;
use crate::mechanisms::observer::Observing;
use crate::mechanisms::second_opinion::SecondOpinion;
use crate::mechanisms::securitisation::Securitising;
use crate::mechanisms::funds::{run_as, Run};
use crate::mechanisms::goods::CostFlow;
use crate::stores::agreed;
use crate::running::{BankCapital, BankFunding, Broking, Calling, Builder, Building, Control, CostOfCapital, Counts, CrossBorder, Derivatives, Fixes, Floating, Flotation, ForcedSeller, ForcedSelling, Funding, FxForwards, Grading, Housing, Liquidity, Losses, Makes, Making, Owed, Publishes, Ranked, Reads, SmallBusiness, Servicing, Sovereign, SpotFx, Storing, StockLending, Subscribing, TradeCredit, Winding};
use crate::world::{Anchor, PhaseDecl};

/// The books this world opens, by subject.
pub const FIRST_SLOT: u32 = 3;

#[inline]
pub fn book_of(line: InstrumentId) -> MarketId {
    MarketId::at(line.0)
}

#[inline]
pub fn line_of(book: MarketId) -> InstrumentId {
    InstrumentId::at(book.0)
}

/// Sellers offer quantities.
pub struct GoodsSellers {
    /// The id of what it will take, read through `params`.
    pub will_take: &'static str,
    /// What another period on the shelf costs it, as a share of what the units cost.
    pub holding_costs: &'static str,
    /// Whether a good is an input is the HOLDER's question, not the good's.
    pub keeps: Vec<(InstrumentId, Vec<InstrumentId>)>,
}

impl Participant for GoodsSellers {
    fn party_kind(&self) -> u32 {
        kinds::FIRM
    }

    /// 3 C2, 22c2.3: it pulls what it can no longer deliver.
    fn pulls(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<crate::stores::RestingId> {
        let (_, standing) = view.resting(m);
        let have = whole_pieces(view.free(line_of(m)));
        if standing <= have {
            return Vec::new();
        }
        let mut over = standing - have;
        let mut pulling = Vec::new();
        for o in view.standing(m) {
            if over <= 0 {
                break;
            }
            let left = view.left_of(o);
            if left <= 0 {
                continue;
            }
            pulling.push(o);
            over -= left;
        }
        pulling
    }

    /// Off its OWN rows.
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        let mine: Vec<InstrumentId> = self
            .keeps
            .iter()
            .filter(|(plant, _)| view.quantity(*plant) > 0.0)
            .flat_map(|(_, inputs)| inputs.iter().copied())
            .collect();
        view.holdings()
            .map(|row| view.line_of(row))
            .filter(|line| view.quantity(*line) > 0.0 && !mine.contains(line))
            .map(book_of)
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        // It offers what it holds IN WHOLE PIECES.
        let (_, already) = view.resting(m);
        let pieces = whole_pieces(view.free(line_of(m))) - already;
        if pieces <= 0 {
            return Vec::new();
        }
        // THE ASK IS A PRICE AND IT ANSWERS THE SHELF.
        let lots = view.lots(line_of(m));
        let units: f64 = lots.iter().map(|l| l.qty).sum();
        if units <= 0.0 {
            return Vec::new();
        }
        let cost = lots.iter().map(|l| l.qty * l.basis_per_unit).sum::<f64>() / units;
        let holding = view.params().ratio(self.holding_costs);
        let will_take = view.params().ratio(self.will_take);
        let reservation = cost * will_take - cost * holding;
        // A price of nothing or less is not a price this seller can post: below that it would rather
        // let the stock perish than pay somebody to take it.
        if reservation <= 0.0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(reservation), qty: pieces }]
    }
}

/// Households buy because they need the thing, and what they can spend is what they have (C1.d: a
/// household that cannot borrow spends what it has, whatever it wants).
pub struct HouseholdBuyers {
    /// The id of what it will pay, read through `params`.
    pub will_pay: &'static str,
    /// The money it keeps back.
    pub keeps: &'static str,
    /// The lines a household consumes.
    pub basket: Vec<InstrumentId>,
}

impl Participant for HouseholdBuyers {
    fn party_kind(&self) -> u32 {
        kinds::HOUSEHOLD
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // A household with no money is in no book.
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.basket.iter().map(|line| book_of(*line)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        if money <= 0.0 {
            return Vec::new();
        }
        // WHAT IT WILL PAY IS A PRICE.
        let Some(print) = view.print(line_of(m)) else { return Vec::new() };
        let limit = print.price * view.params().ratio(self.will_pay);
        if limit <= 0.0 {
            return Vec::new();
        }
        // IT SPENDS OUT OF ITS WEALTH, NOT JUST ITS INCOME — but it keeps a buffer, and what it
        // keeps is its own (a PREFERENCE, dispersed like any other).
        let keeps = view.params().amount(self.keeps, Denomination::Money);
        let spendable = money - keeps;
        if spendable <= 0.0 {
            return Vec::new();
        }
        // It bids for what it can actually fund — and 22c.2: less what it is already bidding for
        // here, because a resting bid is money it has committed once already.
        let (already, _) = view.resting(m);
        let affordable = whole_pieces(spendable / limit) - already;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(limit), qty: affordable }]
    }
}

/// SOMEBODY WHOSE BUSINESS IS TO HOLD THE STOCK.
pub struct Stockist {
    /// What it will carry.
    pub lines: Vec<InstrumentId>,
    /// What a period of holding costs it, as a share of what the units cost: the room, the spoilage
    /// and the money tied up.
    pub carrying: &'static str,
    /// What it will hold of one line.
    pub limit: &'static str,
}

impl Participant for Stockist {
    fn party_kind(&self) -> u32 {
        kinds::STOCKIST
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let line = line_of(m);
        let carrying = view.params().ratio(self.carrying);
        let limit = view.params().amount(self.limit, Denomination::Money);
        let (bidding, offering) = view.resting(m);
        let mut out = Vec::new();

        // THE SELL SIDE: what it cost, plus what carrying it has actually cost.
        let lots = view.lots(line);
        let held: f64 = lots.iter().map(|l| l.qty).sum();
        if held > 0.0 {
            let asking: f64 = lots
                .iter()
                .map(|l| {
                    let periods = f64::from(view.period().saturating_sub(l.acquired));
                    l.qty * l.basis_per_unit * (1.0 + carrying * periods)
                })
                .sum::<f64>()
                / held;
            let pieces = whole_pieces(view.free(line)) - offering;
            if pieces > 0 && asking > 0.0 {
                out.push(Order { party: view.self_id(), side: Side::Sell, price: Some(asking), qty: pieces });
            }
        }

        // THE BUY SIDE: it buys at what it expects to sell for, less what it will cost to carry.
        if let Some(print) = view.print(line) {
            let bid = print.price * (1.0 - carrying);
            // It will not carry more than its limit.
            let room = whole_pieces(limit - held) - bidding;
            let affordable = whole_pieces(view.own_cash() / bid);
            let wants = if room < affordable { room } else { affordable };
            if bid > 0.0 && wants > 0 {
                out.push(Order { party: view.self_id(), side: Side::Buy, price: Some(bid), qty: wants });
            }
        }
        out
    }
}

/// A QUAY'S OWNER EARNS WHAT A BERTH CLEARS AT.
pub struct LetsItsPlant {
    /// The lines whose USE it lets.
    pub lines: Vec<InstrumentId>,
    /// What keeping the plant costs its owner for a period, whether or not it is used.
    pub upkeep: &'static str,
}

impl Participant for LetsItsPlant {
    fn party_kind(&self) -> u32 {
        kinds::CARRIER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().filter(|l| view.quantity(**l) > 0.0).map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let line = line_of(m);
        let held = view.free(line);
        if held <= 0.0 {
            return Vec::new();
        }
        // It is not made to let below what standing there costs it.
        let upkeep = view.params().amount(self.upkeep, Denomination::Money);
        let (_, offering) = view.resting(m);
        let pieces = whole_pieces(held) - offering;
        if pieces <= 0 || upkeep <= 0.0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(upkeep), qty: pieces }]
    }
}

/// Every bank posts a schedule out of its own position.
pub struct MoneyMarketBanks {
    /// Its own buffer preference, derived from its own liabilities — not a stated ratio.
    pub buffer: &'static str,
    /// What it will take to lend its own money out, and what it will pay to borrow.
    pub lends_at: &'static str,
    pub borrows_at: &'static str,
    pub book: Option<MarketId>,
}

impl Participant for MoneyMarketBanks {
    fn party_kind(&self) -> u32 {
        kinds::BANK
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.book.into_iter().collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        let reserves = view.own_cash();
        // The need is knowable only AFTER the period's flows — this reads the position the flows
        // actually left, not an opening balance.
        let need = view.params().amount(self.buffer, Denomination::Money) - reserves;
        if need > 0.0 {
            // Short: it bids for money, at what it will pay.
            let borrows_at = view.params().per_annum(self.borrows_at);
            return vec![Order { party: view.self_id(), side: Side::Buy, price: Some(borrows_at), qty: whole_pieces(need) }];
        }
        let spare = -need;
        if spare <= 0.0 {
            return Vec::new();
        }
        // Long: it offers what it has over its own buffer, at its own rate.
        let lends_at = view.params().per_annum(self.lends_at);
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(lends_at), qty: whole_pieces(spare) }]
    }
}

/// A dealer quotes a price at which it will buy and a price at which it will sell, and it is willing
/// to do either.
pub struct Dealers {
    /// The quote comes from the desk's own state.
    pub around: &'static str,
    /// The width it needs, from what carrying the position costs it.
    pub width: &'static str,
    /// What it will carry.
    pub limit: &'static str,
    pub lines: Vec<InstrumentId>,
}

impl Participant for Dealers {
    fn party_kind(&self) -> u32 {
        kinds::DEALER
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let line = line_of(m);
        let held = view.quantity(line);
        let limit = view.params().amount(self.limit, Denomination::Money);
        let width = view.params().ratio(self.width);
        let around = view.params().ratio(self.around);
        // At its limit it stops quoting.
        if held.abs() >= limit {
            return Vec::new();
        }
        // Long already means it bids lower AND offers lower.
        let skew = width * (held / limit);
        let bid = around - width - skew;
        let ask = around + width - skew;
        // In whole pieces, and an order for none of them is not an order — a desk one half-piece
        // from its limit has room for nothing.
        let (bidding, offering) = view.resting(m);
        let room = whole_pieces(limit - held) - bidding;
        let long = whole_pieces(held) - offering;
        let mut out = Vec::new();
        if view.own_cash() > 0.0 && bid > 0.0 && room > 0 {
            out.push(Order { party: view.self_id(), side: Side::Buy, price: Some(bid), qty: room });
        }
        if long > 0 {
            out.push(Order { party: view.self_id(), side: Side::Sell, price: Some(ask), qty: long });
        }
        out
    }
}

/// A fund must buy something with the cash, per its mandate — which is why a flow into a fund
/// becomes a purchase of what the mandate allows, and why a fund is a transmission channel.
pub struct FundMandates {
    /// The mandate is a real constraint on what it buys, not a label.
    pub may_hold: Vec<InstrumentId>,
    pub will_pay: &'static str,
}

impl Participant for FundMandates {
    fn party_kind(&self) -> u32 {
        kinds::FUND
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // A pool under nobody's mandate posts nothing, because a schedule is somebody's and there is
        // nobody here whose view the order would be.
        if run_as(view.agreed(agreed::MANDATE).len()) == Run::Orphaned {
            return Vec::new();
        }
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.may_hold.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        if run_as(view.agreed(agreed::MANDATE).len()) == Run::Orphaned {
            return Vec::new();
        }
        // A line outside the mandate is one it cannot buy, whatever it is worth.
        let line = line_of(m);
        if !self.may_hold.contains(&line) {
            return Vec::new();
        }
        let money = view.own_cash();
        let will_pay = view.params().ratio(self.will_pay);
        // Less what it is already bidding for here, or it commits the same money twice.
        let (already, _) = view.resting(m);
        let affordable = whole_pieces(money / will_pay) - already;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(will_pay), qty: affordable }]
    }
}

/// A structural buyer of long bonds — a one-way demand that exists whatever the price, because its
/// liabilities are long and its assets are not.
pub struct InsurerMatching {
    pub long_lines: Vec<InstrumentId>,
    pub will_pay: &'static str,
}

impl Participant for InsurerMatching {
    fn party_kind(&self) -> u32 {
        kinds::INSURER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.long_lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        let will_pay = view.params().ratio(self.will_pay);
        // Less what it is already bidding for here.
        let (already, _) = view.resting(m);
        let affordable = whole_pieces(money / will_pay) - already;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(will_pay), qty: affordable }]
    }
}

/// The treasury issues into a market that must clear, choosing the size and the tenor — never the
/// price.
pub struct TreasuryIssues {
    /// `Missing` where the treasury has no line to auction in this world.
    pub paper: Option<InstrumentId>,
    /// The lowest price it will accept.
    pub will_accept: &'static str,
    /// Its own buffer — the reason it is not dependent on every single auction.
    pub buffer: &'static str,
    /// One calendar: how many days a period is, so *what falls due this period* is a read of DATES.
    pub days_per_period: i64,
}

impl Participant for TreasuryIssues {
    fn party_kind(&self) -> u32 {
        kinds::TREASURY
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.paper.map(book_of).into_iter().collect()
    }

    /// IT AUCTIONS WHAT IT IS SHORT OF.
    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        // This period, by DATE.
        let from = crate::calendar::Day(view.period() as i64 * self.days_per_period);
        let to = crate::calendar::Day(from.0 + self.days_per_period - 1);
        let _ = from;
        let outlays = view.owes_by(to);
        // Receipts are what named payers actually owe it — read off the lines it holds, not a rate
        // applied to an aggregate.
        let receipts = view.owed_to_it_by(to);
        let buffer = view.params().amount(self.buffer, Denomination::Money);
        let size = crate::mechanisms::treasury::must_raise(outlays, receipts, view.own_cash(), buffer);
        // A treasury that is short of nothing does not auction.
        if size <= 0.0 {
            return Vec::new();
        }
        let will_accept = view.params().price(self.will_accept);
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(will_accept), qty: whole_pieces(size) }]
    }
}

/// One system, its name, its phases and whoever it puts in a book.
pub struct Wired {
    pub name: &'static str,
    /// Its own declaration slot, so two systems cannot declare the same phase.
    pub slot: u32,
    pub at: u32,
    pub anchor_after: bool,
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
        let anchor = if self.anchor_after { Anchor::After(self.at) } else { Anchor::Before(self.at) };
        // The phase's name is its own declaration slot; the owner is the system itself.
        vec![phase(self.slot, self.slot, anchor)]
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
    Wired { name, slot: 0, at, anchor_after: true, participant: None, mechanism: None, audits: Vec::new() }
}

/// A system that has its OWN WORK in the period and no reason to be in a book: it reads the world
/// through the second door and proposes.
pub fn works(name: &'static str, at: u32, mechanism: Box<dyn Mechanism>) -> Wired {
    Wired { name, slot: 0, at, anchor_after: true, participant: None, mechanism: Some(mechanism), audits: Vec::new() }
}

pub fn posts(name: &'static str, at: u32, participant: Box<dyn Participant>) -> Wired {
    Wired { name, slot: 0, at, anchor_after: false, participant: Some(participant), mechanism: None, audits: Vec::new() }
}

/// The lines each system needs, NAMED.
pub struct Wiring {
    /// How every good in this world is made, one row per line, with the plant it is made
    /// with.
    pub makes: Vec<Makes>,
    /// What funds, insurers and dealers may hold.
    pub lines: Vec<InstrumentId>,
    /// The overnight book's subject.
    pub overnight: Option<InstrumentId>,
    /// What the treasury auctions.
    pub paper: Option<InstrumentId>,
    /// Calendar A1: how many days a period is, so "what falls due this period" is a read of dates.
    pub days_per_period: i64,
}

impl Wiring {
    /// 37 A2, Law 4, Law 19: the basket is a READ of what the recipes make.
    pub fn basket(&self) -> Vec<InstrumentId> {
        self.makes.iter().map(|m| m.line.makes).collect()
    }

    /// Each plant with everything the ways of running it draw on — what a holder of that
    /// plant is keeping rather than selling.
    pub fn keeps(&self) -> Vec<(InstrumentId, Vec<InstrumentId>)> {
        self.makes
            .iter()
            .map(|m| {
                let mut inputs: Vec<InstrumentId> = Vec::new();
                for way in &m.line.ways {
                    for (what, _) in &way.per_unit {
                        if !inputs.contains(what) {
                            inputs.push(*what);
                        }
                    }
                }
                (m.plant, inputs)
            })
            .collect()
    }

    /// Audit C3, 33 A6.b, 22e: which lines the plant-moves family is about, by row.
    pub fn plants(&self) -> Vec<InstrumentId> {
        let mut out: Vec<InstrumentId> = Vec::new();
        for m in &self.makes {
            if !out.contains(&m.plant) {
                out.push(m.plant);
            }
        }
        out
    }

    pub fn capital(&self) -> Vec<bool> {
        let mut is_capital: Vec<bool> = Vec::new();
        for m in &self.makes {
            let row = m.plant.row();
            while is_capital.len() <= row {
                is_capital.push(false);
            }
            is_capital[row] = true;
        }
        is_capital
    }
}

/// Every system this world has, and every one of them RUNS.
pub fn declare(p: &mut Params) {
    let mut say = |id: &str, value: f64, unit: &str, dimension: Dimension, kind: Kind, owner: Owner, why: &str| {
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
    say("reporting.asymmetry", 45.0, "days after the books close", Dimension::Days, Kind::Technology, Owner::StandardSetter,
        "the days between a company's quarter-end and the day its accounts are published");
    // How long a holder has to sell what its mandate no longer lets it hold.
    say("funding.now", 0.0, "days ahead", Dimension::Days, Kind::Resolution, Owner::Model,
        "the near end of a funding window, which is today");
    say("funding.this_period", 7.0, "days ahead", Dimension::Days, Kind::Resolution, Owner::Model,
        "the days a working-capital shortfall is read over, which is one period");
    say("funding.this_year", 365.0, "days ahead", Dimension::Days, Kind::Resolution, Owner::Model,
        "the days a long-term shortfall is read over, which is a year");
    // Commercial paper's own convention, which is what makes it a different
    // instrument from a bond rather than the same one with a different number in it.
    say("paper.tenor", 13.0, "periods the paper runs", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "how long the commercial paper a borrower brings runs for");
    say("paper.coupon", 0.03, "per annum", Dimension::PerAnnum, Kind::Technology, Owner::StandardSetter,
        "the coupon commercial paper carries as a TERM, fixed for its life");
    say("firm.buffer", 10.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the cash a borrower that is not the state keeps back beyond what falls due");
    // The seller's own limits, which is what makes terms a decision rather than a rule.
    say("trade_credit.will_carry", 500.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "how much a seller will have out to one buyer at once before it stops offering terms");
    say("trade_credit.will_wait", 30.0, "days", Dimension::Days, Kind::Preference, Owner::Model,
        "how long a seller will wait to be paid");
    // How many shares a line comes into existence with.
    say("lender.hurdle", 0.05, "per unit lent", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the return a lender wants on what it puts out, which its standard is read against");
    say("bank.min_weighted", 0.08, "capital per unit of weighted assets", Dimension::Ratio, Kind::Policy, Owner::Parliament,
        "the capital a bank must hold against its risk-weighted assets");
    say("bank.min_leverage", 0.03, "capital per unit of assets", Dimension::Ratio, Kind::Policy, Owner::Parliament,
        "the backstop: capital against total assets, whatever they weigh");
    say("bank.buffer", 0.025, "capital per unit above the requirement", Dimension::Ratio, Kind::Policy, Owner::Parliament,
        "the buffer a bank is expected to keep above its requirement, inside which there are consequences short of a breach");
    // The mix a company would raise at.
    say("fund.draws", 0.05, "per unit uncalled", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the share of an uncalled commitment a fund draws in one period");
    say("small_business.reaches", 5000.0, "money", Dimension::Amount(Denomination::Money), Kind::Technology, Owner::StandardSetter,
        "the size at which a borrower can reach the bond market instead of a bank");
    say("acquirer.hurdle", 0.1, "per unit paid", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the return an acquirer wants on what it pays for a company");
    say("control.needs", 0.5, "per unit outstanding", Dimension::Ratio, Kind::Technology, Owner::StandardSetter,
        "how much of the shares it does not already hold a tender must reach");
    say("pool.junior", 0.1, "per unit of the pool", Dimension::Ratio, Kind::Technology, Owner::StandardSetter,
        "the share of a pool that stands in front of its senior note");
    say("pool.pools", 0.2, "per unit of its loan book", Dimension::Ratio, Kind::Preference, Owner::Model,
        "how much of its loan book a bank pools at once");
    say("dwelling.upkeep", 0.02, "money per dwelling per period", Dimension::PricePerUnit, Kind::Technology, Owner::StandardSetter,
        "what keeping one dwelling in repair costs its owner each period");
    say("household.will_spend", 0.5, "per unit of its money", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the share of what it holds a household will put towards a roof");
    say("storage.per_unit", 0.01, "money per unit per period", Dimension::PricePerUnit, Kind::Technology, Owner::StandardSetter,
        "what holding one unit of a physical good for one period costs");
    say("lending.will_lend", 0.3, "per unit held", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the share of a holding a lender will have out on loan at once");
    say("subscribe.commits", 0.1, "per unit of spare cash", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the share of its spare money a holder commits to one pool");
    // The broker's own view and its own limit.
    say("broker.could_move", 0.2, "per unit of the book", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a broker thinks a client's book could move against it in a period");
    say("broker.limit", 100000.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "what one broker will be exposed to one client for");
    say("forward.tenor", 90.0, "days", Dimension::Days, Kind::Technology, Owner::StandardSetter,
        "how far out a currency forward is struck");
    say("protection.tenor", 5.0, "years", Dimension::Years, Kind::Technology, Owner::StandardSetter,
        "how long a protection contract runs");
    say("observer.lag", 2.0, "periods", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "the periods between what a statistic is about and the period it is published in");
    say("invest.horizon", 20.0, "periods", Dimension::Periods, Kind::Preference, Owner::Model,
        "how many periods of return a management counts when it weighs a project");
    say("invest.hurdle", 0.02, "per unit above the cost of capital", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a management wants above its cost of capital before it commits");
    say("invest.takes", 3.0, "periods", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "the periods a capital programme runs before the plant is in service");
    say("capital.debt_share", 0.6, "debt per unit raised", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the share of debt in the money a company would raise at the margin");
    say("equity.shares", 1000.0, "shares", Dimension::Count, Kind::Technology, Owner::StandardSetter,
        "how many shares a company's line comes into existence with when it floats");
    say("equity.takes", 4.0, "periods", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "the periods a flotation stands before it is over, one way or the other");
    say("parliament.seats", 100.0, "seats", Dimension::Count, Kind::Policy, Owner::Constitution,
        "how many seats the parliament of a country has");
    say("parliament.term", 1460.0, "days", Dimension::Days, Kind::Policy, Owner::Constitution,
        "the days between elections, placed by DATE and never a count of periods");
    say("election.takes", 1.0, "periods", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "the periods between an election being called and its result being known");
    say("workout.within", 2.0, "periods", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "the periods a holder has to sell a line its mandate no longer lets it hold");
    say("building.crowds_at", 60.0, "square km standing", Dimension::SquareKm, Kind::Technology, Owner::Model,
        "the ground already covered in a place at which building there draws twice what it does on empty ground");
    // 37 B1, 22c.3: how much cover a firm wants on its shelf.
    say("firm.cover", 0.5, "multiple of what it expects to sell", Dimension::Ratio, Kind::Preference, Owner::Model,
        "how much stock a firm wants on the shelf beyond the week it expects to sell");
    // 37 C1, 22c.3: what another period on the shelf costs the holder, as a share of what the units
    // cost it — the storage, the spoilage and the money tied up.
    say("plant.upkeep", 0.5, "money", Dimension::Amount(Denomination::Money), Kind::Technology, Owner::Model,
        "what keeping a plant costs its owner a period whether or not anybody books it — 33 A4.b's fixed cost");
    say("stockist.limit", 50.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the most a stockist will carry of one line — without one it is the buyer of last resort");
    say("goods.seller.holding_costs", 0.03, "share of what the units cost, a period", Dimension::Ratio, Kind::Technology, Owner::Model,
        "what it costs to keep a unit another period: the room it takes, what spoils and the money in it");
    // A fact about the thing, not about who holds it.
    say("goods.perishes", 0.01, "share of a lot a period", Dimension::Ratio, Kind::Technology, Owner::Model,
        "the share of a lot that does not survive the period");
    // A seller's own reservation, and a buyer's own limit.
    say("goods.seller.will_take", 1.0, "multiple of what it cost", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the least a holder will take for what it holds, against what the units cost it");
    say("household.will_pay", 1.2, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the most a cell will pay for what it buys, against what the book last printed");
    // The money a household keeps back.
    say("household.keeps", 1.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the balance a household holds on to rather than spends, which is why its money is not a trend");
    say("fund.will_pay", 1.1, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a mandate will pay for a line it may hold");
    say("insurer.will_pay", 1.05, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a long-dated matcher will pay for a line that matches its liabilities");
    // The ONE preference expectations are allowed, and it was written twice.
    say("outlook.memory", 0.3, "weight on what just happened", Dimension::Ratio, Kind::Preference, Owner::Model,
        "how fast a party corrects its outlook towards what happened — the one preference §46 has");
    // A bank's own liquidity buffer, and what it lends and borrows at overnight.
    say("money_market.buffer", 1.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the balance a bank keeps back before it lends overnight");
    say("money_market.lends_at", 1.0, "per annum", Dimension::PerAnnum, Kind::Preference, Owner::Model,
        "the rate a bank will lend overnight at");
    say("money_market.borrows_at", 1.0, "per annum", Dimension::PerAnnum, Kind::Preference, Owner::Model,
        "the rate a bank will borrow overnight at");
    // A desk's own quote.
    say("dealer.around", 1.0, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "where a desk centres its quote, against what the book last printed");
    say("dealer.width", 0.02, "share of the mid", Dimension::Ratio, Kind::Preference, Owner::Model,
        "half the spread a desk quotes, which is what it charges for standing there");
    say("dealer.limit", 10.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the most a desk will hold of one line");
    // The lowest price a treasury will accept before it pulls the auction.
    say("treasury.will_accept", 0.98, "price per unit of par", Dimension::Price, Kind::Policy, Owner::Parliament,
        "the lowest price the treasury will accept before it pulls the auction");
    // The shape is dead.
    say("treasury.buffer", 200.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Parliament,
        "the balance the treasury keeps back, which is why one failed auction is not a default");
    // 5 C3.a, 21j.1a: the tenor and the coupon paper is BROUGHT at.
    say("funding.tenor", 26.0, "periods the paper runs", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "how long the paper an issuer brings runs for, which is the convention its market has");
    say("funding.coupon", 0.04, "per annum", Dimension::PerAnnum, Kind::Technology, Owner::StandardSetter,
        "the coupon the paper carries as a TERM, fixed for its life — never what it is worth");
}

pub fn all(w: &Wiring, journal: &mut crate::journal::Journal) -> Vec<Wired> {
    let keys_of = |j: &mut crate::journal::Journal, name: &str| j.keys_named.declare(name);
    let at_equity = keys_of(journal, "accounts.equity");
    let at_income = keys_of(journal, "accounts.income");
    let at_shares = keys_of(journal, "accounts.shares");
    let at_closed = keys_of(journal, "accounts.closed");
    let at_standing = keys_of(journal, "claim.standing");
    let at_ratio = keys_of(journal, "bank.ratio");
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
    // One event kind per system that publishes a read.
    let mut says = |name: &str| kinds.declare(name);
    let mut rows = vec![
        {
            // §37 both posts and works: a firm offers what it holds, and the stock that does not
            // Survive the period leaves at what it cost.
            let mut goods = posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: "goods.seller.will_take", holding_costs: "goods.seller.holding_costs", keeps: w.keeps() }));
            goods.mechanism = Some(Box::new(crate::mechanisms::goods::Perishing { share: "goods.perishes" }));
            goods
        },
        {
            // A cell bids for what it can fund, and forms its outlook from the prices its own lines
            // printed at.
            let mut households =
                posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: "household.will_pay", keeps: "household.keeps", basket: w.basket() }));
            households.mechanism = Some(Box::new(Forming { memory: "outlook.memory" }));
            households
        },
        // THE ONE SYSTEM THAT MAKES ANYTHING.
        works("recipe", AT_CORPORATE_ACTIONS_SLOT, Box::new(Making { makes: w.makes.clone(), flow: CostFlow::FirstInFirstOut, crowds_at: "building.crowds_at", cover: "firm.cover" })),
        works("firms", AT_REVALUATION, Box::new(Reporting { kind: says("firm.result") })),
        works("employment", AT_CORPORATE_ACTIONS_SLOT, Box::new(Wages)),
        // A quay's owner earns what a berth clears at.
        {
            // A quay's owner earns what a berth clears at.
            let mut f = posts("freight", AT_MARKETS, Box::new(LetsItsPlant { lines: w.plants(), upkeep: "plant.upkeep" }));
            f.mechanism = Some(Box::new(Reads { kind: says("freight.carriage"), what: Counts::AgreementsLive }));
            f
        },
        // And stock is TIGHT or it is not, and storing it costs money to somebody.
        works("commodities", AT_REVALUATION, Box::new(Storing {
            kind: says("stock.tightness"),
            per_unit: "storage.per_unit",
        })),
        // Somebody whose business is to hold the stock.
        posts("stockists", AT_MARKETS, Box::new(Stockist { lines: w.basket(), carrying: "goods.seller.holding_costs", limit: "stockist.limit" })),
        // And dwellings are LET and SOLD, and both prices clear.
        works("housing", AT_REVALUATION, Box::new(Housing {
            kind: says("dwelling.sold"),
            lets: says("dwelling.let"),
            upkeep: "dwelling.upkeep",
            will_spend: "household.will_spend",
        })),
        // And a seller that has delivered and not been paid OFFERS TERMS.
        works("trade_credit", AT_REVALUATION, Box::new(TradeCredit {
            kind: says("invoice.struck"),
            will_carry: "trade_credit.will_carry",
            will_wait: "trade_credit.will_wait",
            days_per_period: w.days_per_period,
        })),
        // And the tier too small for the bond market is READ.
        works("small_business", AT_REVALUATION, Box::new(SmallBusiness {
            kind: says("small_business.state"),
            reaches_the_bond_market_at: "small_business.reaches",
            days_per_period: w.days_per_period,
        })),
        {
            let mut mm = posts(
                "money_market",
                AT_MARKETS,
                Box::new(MoneyMarketBanks { buffer: "money_market.buffer", lends_at: "money_market.lends_at", borrows_at: "money_market.borrows_at", book: w.overnight.map(book_of) }),
            );
            mm.mechanism = Some(Box::new(Reads { kind: says("money_market.credit"), what: Counts::CreditOutstanding }));
            mm
        },
        {
            // It BRINGS the paper at corporate actions and AUCTIONS it at the markets — two moments
            // in order, because a bill has to exist before anybody bids for it.
            let mut t = posts("treasury", AT_MARKETS, Box::new(TreasuryIssues { paper: w.paper, will_accept: "treasury.will_accept", buffer: "treasury.buffer", days_per_period: w.days_per_period }));
            t.mechanism = Some(Box::new(Funding {
                // The SOVEREIGN's own paper, and nobody else's.
                of_kinds: &[kinds::TREASURY],
                after: "funding.now",
                horizon: "funding.this_period",
                days_per_period: w.days_per_period,
                tenor: "funding.tenor",
                coupon: "funding.coupon",
                buffer: "treasury.buffer",
                says: says("funding.brought"),
            }));
            t
        },
        // Money A1, 5 A4: every asset is somebody's liability, published party by party.
        works("money", AT_CORPORATE_ACTIONS_SLOT, Box::new(Owed { kind: says("money.owed") })),
        // And a treasury HANDLES being short.
        works("sovereign", AT_REVALUATION, Box::new(Sovereign {
            kind: says("sovereign.shortfall"),
            days_per_period: w.days_per_period,
        })),
        // And a bank READS its own capital.
        works("bank_capital", AT_REVALUATION, Box::new(BankCapital {
            kind: says("bank.capital"),
            short_by: kinds_row_short_of_capital,
            at_ratio,
            min_weighted: "bank.min_weighted",
            min_leverage: "bank.min_leverage",
            buffer: "bank.buffer",
            hurdle: "lender.hurdle",
            days_per_period: w.days_per_period,
        })),
        // And a bank SETS the rate it pays on deposits.
        works("bank_funding", AT_REVALUATION, Box::new(BankFunding {
            kind: says("deposit.rate"),
            fixing: kinds_row_fixing,
            days_per_period: w.days_per_period,
        })),
        // THE ONE THE WHOLE CREDIT SIDE RESTS ON — what falls due is paid, or it is an arrear.
        works("lending", AT_CORPORATE_ACTIONS_SLOT, Box::new(Servicing { days_per_period: w.days_per_period })),
        {
            // Audit C3, 33 A6.b, 22e: PLANT MOVES ONLY FOR A REASON, and this is the one module
            // family this world has.
            let capital = w.capital();
            // And a firm DECIDES to invest.
            let mut cp = works("capital_programme", AT_CORPORATE_ACTIONS_SLOT, Box::new(Building {
                kind: says("plant.built"),
                costs: kinds_row_costs,
                horizon: "invest.horizon",
                hurdle: "invest.hurdle",
                crowds_at: "building.crowds_at",
                takes: "invest.takes",
            }));
            cp.participant = Some(Box::new(Builder { of_kind: kinds::FIRM }));
            cp.audits.push(Box::new(move || {
                Box::new(crate::mechanisms::capital_programme::PlantMoves::over(capital.clone()))
            }));
            cp
        },
        // And a company knows what its capital COSTS it.
        works("cost_of_capital", AT_REVALUATION, Box::new(CostOfCapital {
            kind: kinds_row_costs,
            accounts: kinds_row_accounts,
            at_income,
            at_shares,
            debt_share: "capital.debt_share",
        })),
        // A borrower short over the WEEK brings commercial paper.
        works("short_term_debt", AT_CORPORATE_ACTIONS_SLOT, Box::new(Funding {
            of_kinds: &[kinds::BANK, kinds::FIRM, kinds::SMALL_FIRM],
            after: "funding.now",
            horizon: "funding.this_period",
            days_per_period: w.days_per_period,
            tenor: "paper.tenor",
            coupon: "paper.coupon",
            buffer: "firm.buffer",
            says: says("paper.brought"),
        })),
        // And a borrower short over the YEAR brings a bond.
        works("corporate_credit", AT_CORPORATE_ACTIONS_SLOT, Box::new(Funding {
            of_kinds: &[kinds::FIRM, kinds::BANK],
            after: "funding.this_period",
            horizon: "funding.this_year",
            days_per_period: w.days_per_period,
            tenor: "funding.tenor",
            coupon: "funding.coupon",
            buffer: "firm.buffer",
            says: says("bond.brought"),
        })),
        {
            let mut f = posts("funds", AT_MARKETS, Box::new(FundMandates { may_hold: w.lines.clone(), will_pay: "fund.will_pay" }));
            // A pool whose manager died posts nothing and winds up.
            f.mechanism = Some(Box::new(Winding { says: says("fund.orphaned") }));
            f
        },
        {
            let mut i = posts("insurers", AT_MARKETS, Box::new(InsurerMatching { long_lines: w.lines.clone(), will_pay: "insurer.will_pay" }));
            i.mechanism = Some(Box::new(Reads { kind: says("insurers.policies"), what: Counts::AgreementsLive }));
            i
        },
        {
            let mut d = posts("dealing", AT_MARKETS, Box::new(Dealers { around: "dealer.around", width: "dealer.width", limit: "dealer.limit", lines: w.lines.clone() }));
            d.mechanism = Some(Box::new(Reads { kind: says("dealing.lines"), what: Counts::LinesThatPrinted }));
            d
        },
        {
            // And a fund is the BUYER when others are forced sellers.
            let mut h = works("hedge_funds", AT_REVALUATION, Box::new(Levering {
                kind: says("fund.marked"),
                at_equity,
            }));
            h.participant = Some(Box::new(Liquidity { of_kind: kinds::FUND }));
            h
        },
        // And a fund CALLS its commitments.
        works("private_equity", AT_CORPORATE_ACTIONS_SLOT, Box::new(Calling {
            kind: says("commitment.called"),
            draws: "fund.draws",
        })),
        // And a broker LENDS to a named client and sets what it requires.
        works("prime_brokerage", AT_REVALUATION, Box::new(Broking {
            kind: says("broker.account"),
            could_move: "broker.could_move",
            limit: "broker.limit",
        })),
        // And a pool publishes its NAV, and a holder subscribes at it.
        works("redeemable", AT_REVALUATION, Box::new(Subscribing {
            kind: says("pool.nav"),
            commits: "subscribe.commits",
        })),
        {
            // And a company FLOATS.
            let mut e = works("equity", AT_CORPORATE_ACTIONS_SLOT, Box::new(Floating {
                kind: says("equity.floated"),
                short_of_capital: kinds_row_short_of_capital,
                shares: "equity.shares",
                takes: "equity.takes",
            }));
            e.participant = Some(Box::new(Flotation { of_kind: kinds::FIRM }));
            e
        },
        // And stock is LENT, at a fee that clears.
        works("securities_lending", AT_MARKETS, Box::new(StockLending {
            kind: says("stock.lent"),
            will_lend: "lending.will_lend",
        })),
        // And a bank POOLS loans and cuts notes against them.
        works("securitisation", AT_CORPORATE_ACTIONS_SLOT, Box::new(Securitising {
            kind: says("pool.cut"),
            junior: "pool.junior",
            pools: "pool.pools",
        })),
        // ── The instrument families that settle against what the books printed ──────────────────
        // And a position MARKS, and an offset does not remove it.
        works("derivative_layer", AT_REVALUATION, Box::new(Derivatives {
            kind: says("position.marked"),
            at_mark: at_the_mark,
        })),
        // And protection CLEARS between two parties who disagree.
        works("cds", AT_MARKETS, Box::new(Protection {
            kind: says("protection.struck"),
            tenor: "protection.tenor",
        })),
        works("irs", AT_MARKETS, Box::new(Servicing { days_per_period: w.days_per_period })),
        // And a forward is STRUCK.
        works("fx_forwards", AT_MARKETS, Box::new(FxForwards {
            kind: says("forward.struck"),
            spot: kinds_row_spot,
            fixing: kinds_row_fixing,
            tenor: "forward.tenor",
            days_per_period: w.days_per_period,
        })),
        // And a currency pair CLEARS from real reasons.
        works("spot_fx", AT_MARKETS, Box::new(SpotFx {
            kind: kinds_row_spot,
            days_per_period: w.days_per_period,
        })),
        works("currency", AT_MARKETS, Box::new(Owed { kind: says("currency.owed") })),
        // And a region's accounts are a READ of what actually crossed.
        works("cross_border", AT_REVALUATION, Box::new(CrossBorder {
            kind: says("region.accounts"),
            at_current: keys_of_current,
            at_financial: keys_of_financial,
        })),
        works("benchmarks", AT_REVALUATION, Box::new(Fixes { on: w.overnight.map(book_of), says: kinds_row_fixing })),
        // And every house grades every name it can read.
        works("ratings", AT_REVALUATION, Box::new(Grading {
            kind: says("ratings.action"),
            accounts: kinds_row_accounts,
            at_income,
            days_per_period: w.days_per_period,
        })),
        // And the accounts are PUBLISHED.
        works("reporting", AT_REVALUATION, Box::new(Publishes {
            kind: kinds_row_accounts,
            at_equity,
            at_income,
            at_shares,
            at_closed,
            days_per_period: w.days_per_period,
            asymmetry: "reporting.asymmetry",
            memory: "outlook.memory",
        })),
        // And every lender forms its OWN view of every borrower it holds.
        works("second_opinion", AT_REVALUATION, Box::new(SecondOpinion {
            kind: says("lender.view"),
            days_per_period: w.days_per_period,
        })),
        // And the observer publishes a statistic — LATE, and revised.
        works("observer", AT_REVALUATION, Box::new(Observing {
            kind: says("statistic.published"),
            at_about,
            at_value,
            at_revised,
            lag: "observer.lag",
        })),
        // Every deciding party forms its own outlook from its own history.
        works("expectations", AT_REVALUATION, Box::new(Forming { memory: "outlook.memory" })),
        // ── The events that end things ──────────────────────────────────────────────────────────
        // And a loss is an EVENT.
        works("loss", AT_REVALUATION, Box::new(Losses {
            kind: says("claim.crossed"),
            at_standing,
            days_per_period: w.days_per_period,
        })),
        {
            // And the workout is OPENED.
            let mut f = works("forced_sale", AT_MARKETS, Box::new(ForcedSelling {
                kind: says("workout.opened"),
                within: "workout.within",
            }));
            f.participant = Some(Box::new(ForcedSeller { kind: says("workout.sold"), of_kind: kinds::FUND }));
            f
        },
        // A party whose liabilities exceed its assets CEASES.
        works("mortality", AT_REVALUATION, Box::new(Failing { says: says("mortality.failed") })),
        works("estate", AT_CORPORATE_ACTIONS_SLOT, Box::new(Ranked { says: says("estate.paid") })),
        // §35, §29 B: and a company is BID FOR, and the owners decide.
        works("control", AT_REVALUATION, Box::new(Control {
            kind: says("control.tender"),
            hurdle: "acquirer.hurdle",
            needs: "control.needs",
        })),
        // And the term RUNS OUT.
        works("polity", AT_REVALUATION, Box::new(Elections {
            kind: says("election.called"),
            term: "parliament.term",
            takes: "election.takes",
            days_per_period: w.days_per_period,
        })),
    ];
    // Each declaration gets its OWN slot, so no two systems declare the same phase.
    for (n, row) in rows.iter_mut().enumerate() {
        row.slot = FIRST_SLOT + n as u32;
    }
    rows
}

/// The corporate-actions moment, named here so a system says where it runs without importing the
/// world's constants.
pub const AT_CORPORATE_ACTIONS_SLOT: u32 = crate::assembly::AT_CORPORATE_ACTIONS;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{CurrencyCode, PartyId, RegionId, UnitId};
    use crate::ledger::account_of;
    use crate::instruments::Class;
    use crate::parties::Representation;
    use crate::assembly::World;
    use crate::clearing::PriceRule;

    /// A world with one good, one money, a firm that holds the good and a household with money.
    struct Small {
        w: World,
        cash: InstrumentId,
        reserves: InstrumentId,
        bread: InstrumentId,
        cb: PartyId,
        bank: PartyId,
        firm: PartyId,
        household: PartyId,
    }

    fn world() -> Small {
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 0);
        let reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let cash = w.instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let bread = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 0);
        let household = w.parties.add(kinds::HOUSEHOLD, RegionId::at(0), bank, Representation::Cell(std::num::NonZeroU32::new(500).unwrap()), 0);
        w.register.credit(firm, bread, 400.0, 0.9, 0);
        w.register.money_delta(household, cash, 600.0);
        w.open_book(
            book_of(bread),
            bread,
            CurrencyCode::at(0),
            crate::protocols::Venue {
                rule: PriceRule::SellersCompete,
                protocol: crate::protocols::Protocol::Call,
                seen_by: 1,
                stands_for: None,
            },
        );
        // A buyer bids against what the book last PRINTED, because its limit is a multiple of a
        // price and not a price.
        w.prints.write(crate::prices::Print {
            instrument: bread,
            market: book_of(bread),
            period: 0,
            price: 1.0,
            ccy: CurrencyCode::at(0),
            quoted_as: crate::prices::QuotedAs::Money,
            provenance: crate::prices::Provenance::Cleared,
        });
        // The participants below hold IDs, so the world they are asked in has the numbers.
        declare(&mut w.params);
        Small { w, cash, reserves, bread, cb, bank, firm, household }
    }

    /// A view of one party in that world, with its own account resolved as the session resolves it.
    fn seen(s: &Small, who: PartyId) -> ParticipantView<'_> {
        ParticipantView::of(
            who,
            &s.w.register,
            &s.w.prints,
            &s.w.journal,
            &s.w.params,
            1,
            account_of(&s.w.parties, &s.w.instruments, who),
        )
        .knowing(&s.w.agreements)
    }

    #[test]
    fn a_firm_posts_what_it_holds_and_a_household_bids_what_it_can_fund() {
        // The seller offers units it HAS; the buyer bids for what it can pay for.
        let s = world();
        let sellers = GoodsSellers { will_take: "goods.seller.will_take", holding_costs: "goods.seller.holding_costs", keeps: Vec::new() };
        let buyers = HouseholdBuyers { will_pay: "household.will_pay", keeps: "household.keeps", basket: vec![s.bread] };
        let bread = s.bread;

        let seller_view = seen(&s, s.firm);
        assert_eq!(sellers.markets(&seller_view), vec![book_of(bread)]);
        let posted = sellers.orders(&seller_view, book_of(bread));
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].qty, 400);

        let buyer_view = seen(&s, s.household);
        assert_eq!(buyers.markets(&buyer_view), vec![book_of(bread)]);
        let bids = buyers.orders(&buyer_view, book_of(bread));
        assert_eq!(bids.len(), 1);
        // It bids against the PRINT and it keeps a buffer.
        assert_eq!(bids[0].price, Some(1.2));
        assert_eq!(bids[0].qty, 416);
    }

    #[test]
    fn a_household_with_no_money_is_in_no_book() {
        // Not a rule about households — it is what having nothing to pay with means.
        let s = world();
        let buyers = HouseholdBuyers { will_pay: "household.will_pay", keeps: "household.keeps", basket: vec![s.bread] };
        // The firm holds bread and has an account with nothing in it.
        let view = seen(&s, s.firm);
        assert!(buyers.markets(&view).is_empty());
    }

    #[test]
    fn the_world_steps_and_the_book_clears() {
        // The whole point of the wiring: parties, a book, and a period that actually trades.
        let mut s = world();
        let (bread, household) = (s.bread, s.household);
        let w = &mut s.w;
        let systems = [
            posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: "goods.seller.will_take", holding_costs: "goods.seller.holding_costs", keeps: Vec::new() })).slotted(FIRST_SLOT),
            posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: "household.will_pay", keeps: "household.keeps", basket: vec![bread] })).slotted(FIRST_SLOT + 1),
        ];
        let as_systems: Vec<&dyn System> = systems.iter().map(|s| s as &dyn System).collect();
        w.wire_up(&as_systems);
        let did = w.step(&as_systems);
        assert!(did.asks > 0);
        assert_eq!(did.books_cleared, 1);
        assert!(did.trades > 0);
        // And the units actually moved: the household holds bread it did not hold before.
        assert!(w.register.quantity(w.register.row(household, bread)) > 0.0);
    }

    #[test]
    fn a_dealers_inventory_skews_its_quote() {
        // Long already means it bids lower AND offers lower — how a desk mean-reverts its book
        // without anyone telling it to, and why order flow moves prices.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let dealer = s.w.parties.add(kinds::DEALER, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.register.money_delta(dealer, cash, 5_000.0);
        let d = Dealers { around: "dealer.around", width: "dealer.width", limit: "dealer.limit", lines: vec![bread] };

        let flat = seen(&s, dealer);
        let quoted_flat = d.orders(&flat, book_of(bread));
        let bid_flat = quoted_flat.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();

        s.w.register.credit(dealer, bread, 500.0, 1.0, 0);
        let long = seen(&s, dealer);
        let quoted_long = d.orders(&long, book_of(bread));
        let bid_long = quoted_long.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();
        assert!(bid_long < bid_flat);
        // And it is offering as well as bidding, which is what makes it two-sided.
        assert!(quoted_long.iter().any(|o| o.side == Side::Sell));
    }

    #[test]
    fn a_dealer_at_its_limit_stops_quoting_rather_than_absorbing_more() {
        // A limit that never binds is not a limit.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let dealer = s.w.parties.add(kinds::DEALER, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.register.money_delta(dealer, cash, 5_000.0);
        s.w.register.credit(dealer, bread, 1_200.0, 1.0, 0);
        let d = Dealers { around: "dealer.around", width: "dealer.width", limit: "dealer.limit", lines: vec![bread] };
        let view = seen(&s, dealer);
        assert!(d.orders(&view, book_of(bread)).is_empty());
    }

    #[test]
    fn a_bank_short_of_its_own_buffer_bids_and_one_over_it_offers() {
        // Who lends and who borrows is the OUTCOME of the schedules, never a rule that surplus banks
        // lend.
        let mut s = world();
        // A bank banks at the central bank, so what it settles in is RESERVES — which is what the
        // overnight market is a market in.
        let reserves = s.reserves;
        let short = s.w.parties.add(kinds::BANK, RegionId::at(0), s.cb, Representation::Named, 0);
        let flush = s.w.parties.add(kinds::BANK, RegionId::at(0), s.cb, Representation::Named, 0);
        s.w.register.money_delta(short, reserves, 40.0);
        s.w.register.money_delta(flush, reserves, 900.0);
        let book = book_of(reserves);
        let m = MoneyMarketBanks { buffer: "money_market.buffer", lends_at: "money_market.lends_at", borrows_at: "money_market.borrows_at", book: Some(book) };

        assert_eq!(m.orders(&seen(&s, short), book)[0].side, Side::Buy);
        assert_eq!(m.orders(&seen(&s, flush), book)[0].side, Side::Sell);
    }

    #[test]
    fn a_fund_cannot_buy_outside_its_mandate() {
        // The mandate is a real constraint on what it buys, not a label.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let fund = s.w.parties.add(kinds::FUND, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.register.money_delta(fund, cash, 1_000.0);
        // And somebody runs it.
        let manager = s.w.parties.add(kinds::FIRM, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.agreements.strike(agreed::MANDATE, manager, fund, &[], crate::calendar::Day(-7), None);
        let allowed = FundMandates { may_hold: vec![bread], will_pay: "fund.will_pay" };
        let forbidden = FundMandates { may_hold: Vec::new(), will_pay: "fund.will_pay" };
        let view = seen(&s, fund);
        assert!(!allowed.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.markets(&view).is_empty());
    }

    #[test]
    fn a_pool_under_nobody_s_mandate_posts_nothing_at_all() {
        // A schedule is somebody's.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let orphan = s.w.parties.add(kinds::FUND, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.register.money_delta(orphan, cash, 1_000.0);
        let pool = FundMandates { may_hold: vec![bread], will_pay: "fund.will_pay" };
        let view = seen(&s, orphan);
        assert!(pool.markets(&view).is_empty());
        assert!(pool.orders(&view, book_of(bread)).is_empty());

        // And it is not a rule against acting: give it a manager and it is in the book again.
        let manager = s.w.parties.add(kinds::FIRM, RegionId::at(0), s.bank, Representation::Named, 0);
        s.w.agreements.strike(agreed::MANDATE, manager, orphan, &[], crate::calendar::Day(-7), None);
        let view = seen(&s, orphan);
        assert_eq!(pool.markets(&view), vec![book_of(bread)]);
    }

    /// A line that makes one good out of another, with a mill to make it in.
    fn a_loaf(makes: InstrumentId, from: InstrumentId) -> Makes {
        Makes {
            line: crate::mechanisms::recipe::Line::new(
                makes,
                vec![crate::mechanisms::recipe::Recipe::new(makes, vec![(from, 2.0)], 0.1, 0.05, 0.98, 10.0, 1)],
            ),
            plant: InstrumentId::at(9),
            plant_is: crate::mechanisms::capital_programme::Plant {
                life: 100,
                upkeep_per_period: 1.0,
                capacity_per_period: 150.0,
            },
        }
    }

    #[test]
    fn every_system_of_the_world_is_wired_exactly_once() {
        // ARCHITECTURE 4.9b: adding a system is a row, and a module not in this list does not run.
        let mut journal = crate::journal::Journal::new();
        let all = all(
            &Wiring {
                makes: vec![a_loaf(InstrumentId::at(1), InstrumentId::at(4))],
                lines: vec![InstrumentId::at(1)],
                overnight: Some(InstrumentId::at(2)),
                paper: Some(InstrumentId::at(3)),
                days_per_period: 7,
            },
            &mut journal,
        );
        let mut names: Vec<&str> = all.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a system wired twice would run twice");
        assert_eq!(before, 51, "every ported module is wired, and nothing is wired twice");
        // And every declaration has its own slot, or two of them would be the same phase.
        let mut slots: Vec<u32> = all.iter().map(|s| s.slot).collect();
        slots.sort_unstable();
        let filled = slots.len();
        slots.dedup();
        assert_eq!(slots.len(), filled, "two systems sharing a declaration slot is one phase, twice");
    }

    #[test]
    fn a_system_that_reads_posts_nothing() {
        // No demand added to clear.
        let r = reads("benchmarks", AT_REVALUATION);
        assert!(r.participants().is_empty());
        assert_eq!(r.phases().len(), 1);
    }

    #[test]
    fn every_wired_system_carries_a_mechanism_and_none_is_dead() {
        // The thing that was not true.
        let mut journal = crate::journal::Journal::new();
        let all = all(
            &Wiring {
                makes: vec![a_loaf(InstrumentId::at(1), InstrumentId::at(4))],
                lines: vec![InstrumentId::at(1)],
                overnight: Some(InstrumentId::at(2)),
                paper: Some(InstrumentId::at(3)),
                days_per_period: 7,
            },
            &mut journal,
        );
        for row in &all {
            // A row with NEITHER is dead, which is what this exists to catch.
            assert!(
                row.mechanism.is_some() || row.participant.is_some(),
                "{} is wired and neither posts nor works — it is a declaration and nothing else",
                row.name
            );
        }
    }

    #[test]
    fn a_period_of_the_wired_world_runs_every_one_of_them() {
        // ARCHITECTURE 4.9b end to end: fifty systems, fifty phases, and `ran` counts the ones that
        // actually ran.
        let mut s = world();
        let bread = s.bread;
        // The kinds they publish under are declared on THE WORLD'S OWN journal.
        let wired = all(
            &Wiring {
                makes: vec![a_loaf(bread, InstrumentId::at(4))],
                lines: vec![bread],
                overnight: None,
                paper: None,
                days_per_period: 7,
            },
            &mut s.w.journal,
        );
        let systems: Vec<&dyn System> = wired.iter().map(|w| w as &dyn System).collect();
        s.w.wire_up(&systems);
        let did = s.w.step(&systems);
        assert_eq!(did.ran, 50, "every wired system ran its phase");
    }
}

#[cfg(test)]
mod publishing {
    use super::*;
    use crate::assembly::{kinds, System, World};
    use crate::ids::{CurrencyCode, PartyId, RegionId, UnitId};
    use crate::instruments::Class;
    use crate::journal::Value;
    use crate::parties::Representation;

    #[test]
    fn a_listed_company_publishes_its_accounts_on_its_own_quarter_end() {
        let mut w = World::empty();
        declare(&mut w.params);
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 0);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 0);
        let holder = w.parties.add(kinds::FUND, RegionId::at(0), bank, Representation::Named, 0);
        // Listed and held by an OUTSIDER.
        let share = w.instruments.issue(firm, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.register.credit(holder, share, 100.0, 2.0, 0);
        // Something for the accounts to be about.
        let plant = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(0), None, None);
        w.register.credit(firm, plant, 50.0, 4.0, 0);

        let wired = all(
            &Wiring {
                makes: Vec::new(),
                lines: Vec::new(),
                overnight: None,
                paper: None,
                days_per_period: 7,
            },
            &mut w.journal,
        );
        let only: Vec<&dyn System> = wired.iter().filter(|s| s.name == "reporting").map(|s| s as &dyn System).collect();
        w.wire_up(&only);
        let published = w.journal.kinds.row("accounts.published");
        let at_equity = w.journal.keys_named.row("accounts.equity");
        let at_income = w.journal.keys_named.row("accounts.income");

        // The fiscal period is a QUARTER placed by date.
        for _ in 0..19 {
            w.step(&only);
            assert!(w.journal.of_kind(published).is_empty(), "48 A4: the books have not closed");
        }
        w.step(&only);
        let out = w.journal.of_kind(published).to_vec();
        assert_eq!(out.len(), 1, "one company, one quarter-end, one report");
        assert_eq!(w.journal.subjects_of(out[0]), &[firm.0]);
        assert!(w.journal.is_public(out[0]), "48 A1: PUBLISHED is what makes it readable by anybody");
        assert_eq!(w.journal.says(out[0], at_equity), Some(Value::Num(200.0)));
        // A FIRST report has no prior close, so there is no income figure at all.
        assert_eq!(w.journal.says(out[0], at_income), None);

        // And it does not publish the same quarter twice — the next report is the next quarter's.
        w.step(&only);
        assert_eq!(w.journal.of_kind(published).len(), 1);
    }

    /// And a company whose outsiders sell stops reporting, without anybody relabelling it.
    #[test]
    fn a_company_nobody_outside_holds_publishes_nothing() {
        let mut w = World::empty();
        declare(&mut w.params);
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 0);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 0);
        let share = w.instruments.issue(firm, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        // Every share is the company's own: listed, and held by nobody else.
        w.register.credit(firm, share, 100.0, 1.0, 0);
        let wired = all(
            &Wiring { makes: Vec::new(), lines: Vec::new(), overnight: None, paper: None, days_per_period: 7 },
            &mut w.journal,
        );
        let only: Vec<&dyn System> = wired.iter().filter(|s| s.name == "reporting").map(|s| s as &dyn System).collect();
        w.wire_up(&only);
        let published = w.journal.kinds.row("accounts.published");
        for _ in 0..25 {
            w.step(&only);
        }
        assert!(w.journal.of_kind(published).is_empty());
    }
}

#[cfg(test)]
mod grading {
    use super::*;
    use crate::assembly::{kinds, System, World};
    use crate::ids::{CurrencyCode, PartyId, RegionId, UnitId};
    use crate::instruments::Class;
    use crate::parties::Representation;

    /// A house grades a name, holds the grade, and is STICKY.
    #[test]
    fn a_house_grades_every_name_that_has_published_and_holds_what_it_said() {
        let mut w = World::empty();
        declare(&mut w.params);
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 0);
        w.instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 0);
        let holder = w.parties.add(kinds::FUND, RegionId::at(0), bank, Representation::Named, 0);
        let one = w.parties.add(kinds::ASSESSOR, RegionId::at(0), bank, Representation::Named, 0);
        let other = w.parties.add(kinds::ASSESSOR, RegionId::at(0), bank, Representation::Named, 0);
        let share = w.instruments.issue(firm, CurrencyCode::at(0), Class::Share, UnitId::at(0), None, None);
        w.register.credit(holder, share, 100.0, 2.0, 0);
        let plant = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(0), None, None);
        w.register.credit(firm, plant, 50.0, 4.0, 0);

        let wired = all(
            &Wiring { makes: Vec::new(), lines: Vec::new(), overnight: None, paper: None, days_per_period: 7 },
            &mut w.journal,
        );
        let only: Vec<&dyn System> = wired
            .iter()
            .filter(|s| s.name == "reporting" || s.name == "ratings")
            .map(|s| s as &dyn System)
            .collect();
        w.wire_up(&only);
        let action = w.journal.kinds.row("ratings.action");

        // Nothing to grade until something has published: a grade formed off no accounts would be a
        // grade of a state nobody can read.
        for _ in 0..20 {
            w.step(&only);
        }
        assert!(
            w.journal.of_kind(action).is_empty(),
            "21 A2: ONE report carries no income (48 G2 has no prior close), so coverage cannot be \
             read and there is nothing to grade on — missing is missing"
        );
        // Its second QUARTER-end gives it an income figure, and that is the first thing gradeable.
        for _ in 0..14 {
            w.step(&only);
        }
        assert!(!w.journal.of_kind(action).is_empty(), "48 A1 published twice, so 21 A2 has a state to read");

        // The house HOLDS it, about that name, and it is one row per house.
        let held = w.standing.of_party_about(one, firm, crate::stores::standing::GRADE);
        assert!(held.is_some(), "21 A4: a grade a house does not hold is one it cannot be held to");
        assert!(w.standing.of_party_about(other, firm, crate::stores::standing::GRADE).is_some());
        assert!(
            w.standing.of_party_about(one, holder, crate::stores::standing::GRADE).is_none(),
            "a name that published nothing is not graded"
        );

        // And it is STICKY — a state that has not moved the grade is not a rating action.
        let was = w.journal.of_kind(action).len();
        w.step(&only);
        assert_eq!(w.journal.of_kind(action).len(), was, "21 A3: a grade republished is not a move");
    }
}
