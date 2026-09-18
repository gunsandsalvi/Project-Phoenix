//! THE SYSTEMS, WIRED. Every spec system this world has, in one list, with the participants that
//! have a reason to post and the phases that say when they run.
//!
//! @spec 3 B2 · 37 C1 · 39 · 11 B1 · 10 B1 · 7 D1 · 40 B1 · 26 A2 · ARCHITECTURE 4.9b · Law 3,
//! @spec Law 6, Law 15, Law 19 · Appendix B
//!
//! **A participant posts out of its own state and nothing else.** Every `orders` below reads the
//! party's own holdings, its own money and the public prints — `ParticipantView` has no door onto
//! anybody else's book, so a schedule written against a rival's position cannot be written (Observer
//! A4).
//!
//! **Most asks post nothing, and that is the world's shape.** A session asks every party of a kind
//! whether it has an order and the answer is usually no: a firm sells what it made, not what it could
//! imagine making; a household buys what it can fund. A world where every asked party posts is
//! measuring a different economy.
//!
//! **The systems that post and the systems that read.** Twenty-odd of the forty-seven have a reason
//! to be in a book. The rest — the benchmarks, the ratings, the observer surface, reporting, the
//! second opinion, the audit families — are READS over what the books produced, and handing them a
//! schedule would be inventing demand nobody has (Appendix B: no demand added to clear).

use crate::assembly::{kinds, phase, System, AT_MARKETS, AT_REVALUATION};
use crate::clearing::{Order, Side};
use crate::ids::{InstrumentId, MarketId};
use crate::ids::Names;
use crate::module::{Mechanism, Participant, ParticipantView};
use crate::running::{afoot, Closing, Counts, Forming, Owed, Reads, Reporting, Servicing, Wages};
use crate::world::{Anchor, PhaseDecl};

/// The books this world opens, by subject. A participant names a book off its OWN rows (Law 19), so
/// the market id and the instrument it delivers are one number — the book for a line IS that line's
/// id, which is what lets `markets()` be a walk over holdings rather than a question per book.
/// Law 10: the kernel's three MOMENTS own the first three declaration names (corporate actions,
/// markets, revaluation), so a module's slot starts after them. A slot of 0 would not be a new
/// phase — it would be the corporate-actions moment, declared twice.
pub const FIRST_SLOT: u32 = 3;

#[inline]
pub fn book_of(line: InstrumentId) -> MarketId {
    MarketId::at(line.0)
}

#[inline]
pub fn line_of(book: MarketId) -> InstrumentId {
    InstrumentId::at(book.0)
}

/// §37 C1: **sellers offer quantities.** A firm posts what it actually holds, at what it will take —
/// its reservation, from its own cost. It does not post what it could make.
pub struct GoodsSellers {
    /// §37 B1: what a unit cost it. Its own, and two firms differ (§32 A3).
    pub will_take: f64,
}

impl Participant for GoodsSellers {
    fn party_kind(&self) -> u32 {
        kinds::FIRM
    }

    /// Law 19: off its OWN rows. A firm is in the book for a line because it HOLDS that line.
    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        view.holdings()
            .filter(|row| view.quantity(view.line_of(*row)) > 0.0)
            .map(|row| book_of(view.line_of(row)))
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        // Clearing C1, Law 8: **it offers what it holds IN WHOLE PIECES.** A holding is a quantity in
        // its own unit and an order is a count of pieces, so a firm left with part of a loaf has
        // something and has nothing to sell — and an order for none of it is not an order. The first
        // warm-up in which every firm could reach the book is where this turned up (22b.9a).
        let pieces = view.free(line_of(m)) as i64;
        if pieces <= 0 {
            return Vec::new();
        }
        // §37 B1: it produces — and sells — because the price covers its cost. Law 6: it is not made
        // to sell below that; a book that will not reach it simply does not clear for this seller.
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(self.will_take), qty: pieces }]
    }
}

/// §41 C1, §37 C3: **households buy because they need the thing**, and what they can spend is what
/// they have (C1.d: a household that cannot borrow spends what it has, whatever it wants).
pub struct HouseholdBuyers {
    /// §41 C1: what it will pay, from its own outlook of what the thing is worth to it.
    pub will_pay: f64,
    /// The lines a household consumes. Registry data (Law 15): not a branch on a kind.
    pub basket: Vec<InstrumentId>,
}

impl Participant for HouseholdBuyers {
    fn party_kind(&self) -> u32 {
        kinds::HOUSEHOLD
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // Law 6: a household with no money is in no book. That is not a rule about households — it
        // is what having nothing to pay with means.
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.basket.iter().map(|line| book_of(*line)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        if money <= 0.0 || self.will_pay <= 0.0 {
            return Vec::new();
        }
        // §41 C1.d: it bids for what it can actually fund. A bid it cannot pay for is not a bid.
        let affordable = (money / self.will_pay) as i64;
        if affordable <= 0 {
            return Vec::new();
        }
        let _ = m;
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(self.will_pay), qty: affordable }]
    }
}

/// §11 B1: **every bank posts a schedule out of its own position.** Who ends up lending and who ends
/// up borrowing is the OUTCOME — never a rule that surplus banks lend.
pub struct MoneyMarketBanks {
    /// §11 A2.a: its own buffer preference, derived from its own liabilities — not a stated ratio.
    pub buffer: f64,
    /// §11 B2: what it will take to lend its own money out, and what it will pay to borrow.
    pub lends_at: f64,
    pub borrows_at: f64,
    /// The overnight book. `Missing` where this world has no overnight line yet — a bank with
    /// nowhere to lend posts nowhere, which is an absence and not a market it sits out of.
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
        // §11 A3: the need is knowable only AFTER the period's flows — this reads the position the
        // flows actually left, not an opening balance.
        let need = self.buffer - reserves;
        if need > 0.0 {
            // Short: it bids for money, at what it will pay.
            return vec![Order { party: view.self_id(), side: Side::Buy, price: Some(self.borrows_at), qty: need as i64 }];
        }
        let spare = -need;
        if spare <= 0.0 {
            return Vec::new();
        }
        // Long: it offers what it has over its own buffer, at its own rate. §11 B1: whether it ends
        // up lending is the book's answer, not this schedule's.
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(self.lends_at), qty: spare as i64 }]
    }
}

/// §26 A2: **a dealer quotes a price at which it will buy and a price at which it will sell**, and it
/// is willing to do either. §26 C2: its inventory SKEWS the quote — long already means it bids lower
/// and offers lower, which is how a desk mean-reverts its book without anyone telling it to.
pub struct Dealers {
    /// §26 C1: the quote comes from the desk's own state. This is its mid before the skew.
    pub around: f64,
    /// §26 C3, C5: the width it needs, from what carrying the position costs it.
    pub width: f64,
    /// §26 D1: what it will carry. When the limit binds it stops quoting rather than absorbing more.
    pub limit: f64,
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
        // §26 D1: at its limit it stops quoting. A limit that never binds is not a limit.
        if held.abs() >= self.limit {
            return Vec::new();
        }
        // §26 C2: long already means it bids lower AND offers lower. The skew is the inventory
        // against the limit — a fact about its own book, not a stated rule.
        let skew = self.width * (held / self.limit);
        let bid = self.around - self.width - skew;
        let ask = self.around + self.width - skew;
        // Clearing C1: in whole pieces, and an order for none of them is not an order — a desk one
        // half-piece from its limit has room for nothing.
        let room = (self.limit - held) as i64;
        let long = held as i64;
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

/// §13 C1.a: **a fund must buy something with the cash**, per its mandate — which is why a flow into
/// a fund becomes a purchase of what the mandate allows, and why a fund is a transmission channel.
pub struct FundMandates {
    /// §13 A4: **the mandate is a real constraint on what it buys**, not a label.
    pub may_hold: Vec<InstrumentId>,
    pub will_pay: f64,
}

impl Participant for FundMandates {
    fn party_kind(&self) -> u32 {
        kinds::FUND
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.may_hold.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        // §13 A4: a line outside the mandate is one it cannot buy, whatever it is worth.
        let line = line_of(m);
        if !self.may_hold.contains(&line) {
            return Vec::new();
        }
        let money = view.own_cash();
        let affordable = (money / self.will_pay) as i64;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(self.will_pay), qty: affordable }]
    }
}

/// §27 C2.a: **a structural buyer of long bonds** — a one-way demand that exists whatever the price,
/// because its liabilities are long and its assets are not. A real force in that market, and not a
/// preference.
pub struct InsurerMatching {
    pub long_lines: Vec<InstrumentId>,
    pub will_pay: f64,
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

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        let affordable = (money / self.will_pay) as i64;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(self.will_pay), qty: affordable }]
    }
}

/// §30 D2: **the treasury issues into a market that must clear**, choosing the size and the tenor —
/// never the price. It is a seller of its own paper.
pub struct TreasuryIssues {
    /// `Missing` where the treasury has no line to auction in this world.
    pub paper: Option<InstrumentId>,
    /// §30 D2.a: the lowest price it will accept. Below that it pulls the auction (D5).
    pub will_accept: f64,
    pub size: f64,
}

impl Participant for TreasuryIssues {
    fn party_kind(&self) -> u32 {
        kinds::TREASURY
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.paper.map(book_of).into_iter().collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        if self.size <= 0.0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(self.will_accept), qty: self.size as i64 }]
    }
}

/// One system, its name, its phases and whoever it puts in a book. Most rows are one line, which is
/// the point: **adding a system is a row.**
pub struct Wired {
    pub name: &'static str,
    /// Law 10: its own declaration slot, so two systems cannot declare the same phase (Law 4).
    pub slot: u32,
    pub at: u32,
    pub anchor_after: bool,
    pub participant: Option<Box<dyn Participant>>,
    /// ARCHITECTURE 4.9b: its own work in the period, if it has any of its own. A system that only
    /// reads what the books produced has none, and that is an answer rather than a gap.
    pub mechanism: Option<Box<dyn Mechanism>>,
}

impl Wired {
    /// Law 4, Law 10: its declaration slot. `all` assigns these by position; a caller wiring a
    /// handful of systems itself says which slot each one is, because two systems sharing one is
    /// the same phase declared twice.
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

/// A system that reads rather than posts. **Not a gap**: the benchmarks, the ratings, the observer
/// surface, reporting and the second opinion are reads over what the books produced, and a schedule
/// for one of them would be demand nobody has.
pub fn reads(name: &'static str, at: u32) -> Wired {
    // The slot is assigned by `all`, which is the one place that knows the order.
    Wired { name, slot: 0, at, anchor_after: true, participant: None, mechanism: None }
}

/// A system that has its OWN WORK in the period and no reason to be in a book: it reads the world
/// through the second door and proposes. Most of the forty-seven are this.
pub fn works(name: &'static str, at: u32, mechanism: Box<dyn Mechanism>) -> Wired {
    Wired { name, slot: 0, at, anchor_after: true, participant: None, mechanism: Some(mechanism) }
}

/// A system that posts. The participant is its reason to be in a book (Clearing B2).
pub fn posts(name: &'static str, at: u32, participant: Box<dyn Participant>) -> Wired {
    Wired { name, slot: 0, at, anchor_after: false, participant: Some(participant), mechanism: None }
}

/// **The lines each system needs, NAMED.** It was one `cash` handed to everybody, which is what
/// 22b.9a found: with a deposit line per bank there is no such thing as "the cash", and a system
/// told which line to look at is a system looking at somebody else's money. A party's own account is
/// `ParticipantView::own_cash`; what is left here is what each system TRADES.
pub struct Wiring {
    /// What households consume.
    pub basket: Vec<InstrumentId>,
    /// What funds, insurers and dealers may hold.
    pub lines: Vec<InstrumentId>,
    /// §11: the overnight book's subject. `Missing` where no such line exists yet — a money market
    /// with nothing to lend is not a money market lending nothing.
    pub overnight: Option<InstrumentId>,
    /// §30 D2: what the treasury auctions.
    pub paper: Option<InstrumentId>,
    /// Calendar A1: how many days a period is, so "what falls due this period" is a read of dates.
    pub days_per_period: i64,
}

/// **Every system this world has, and every one of them RUNS.** The forty-seven of Part XIII, each
/// wired once: the ones with a reason to post carry a participant, and **all of them carry a
/// mechanism** — the thing that system does in a period, through the second door.
///
/// It takes the journal's kinds because a system that publishes has to publish under a name, and
/// naming it here is what makes an event traceable to the system that said it (Law 9).
///
/// **A system whose state the world does not hold yet proposes nothing**, and that is an answer: the
/// wiring is complete, so it becomes live the moment the world holds something. What it is NOT is a
/// row that the loop never reaches, which is what 43 of these were.
pub fn all(w: &Wiring, kinds: &mut Names) -> Vec<Wired> {
    // One event kind per system that publishes a read. Law 4: declared once, here, where the list is.
    let mut says = |name: &str| kinds.declare(name);
    let mut rows = vec![
        // ── The real economy: what is made, what it costs to move, and who buys it ──────────────
        {
            // §37 both posts and works: a firm offers what it holds, and the stock that does not
            // survive the period leaves at what it cost (37 E4).
            let mut goods = posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: 1.0 }));
            goods.mechanism = Some(Box::new(crate::mechanisms::goods::Perishing { share: 0.01 }));
            goods
        },
        {
            // §41 both posts and works: a cell bids for what it can fund, and forms its outlook from
            // the prices its own lines printed at (§46).
            let mut households =
                posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: 1.2, basket: w.basket.clone() }));
            households.mechanism = Some(Box::new(Forming { memory: 0.3 }));
            households
        },
        works("recipe", AT_MARKETS, Box::new(Reads { kind: says("recipe.lines"), what: Counts::LinesThatPrinted })),
        works("firms", AT_REVALUATION, Box::new(Reporting { kind: says("firm.result") })),
        works("employment", AT_CORPORATE_ACTIONS_SLOT, Box::new(Wages)),
        works("freight", AT_MARKETS, Box::new(Reads { kind: says("freight.carriage"), what: Counts::AgreementsLive })),
        works("commodities", AT_MARKETS, Box::new(Reads { kind: says("commodities.lines"), what: Counts::LinesThatPrinted })),
        works("housing", AT_MARKETS, Box::new(Closing { kind: afoot::FORECLOSURE, says: says("housing.foreclosed") })),
        works("trade_credit", AT_MARKETS, Box::new(Reads { kind: says("trade_credit.out"), what: Counts::AgreementsLive })),
        works("small_business", AT_MARKETS, Box::new(Reads { kind: says("small_business.credit"), what: Counts::CreditOutstanding })),
        // ── Money, the banks and the sovereign ──────────────────────────────────────────────────
        {
            let mut mm = posts(
                "money_market",
                AT_MARKETS,
                Box::new(MoneyMarketBanks { buffer: 100.0, lends_at: 1.0, borrows_at: 1.0, book: w.overnight.map(book_of) }),
            );
            mm.mechanism = Some(Box::new(Reads { kind: says("money_market.credit"), what: Counts::CreditOutstanding }));
            mm
        },
        {
            let mut t = posts("treasury", AT_MARKETS, Box::new(TreasuryIssues { paper: w.paper, will_accept: 0.98, size: 0.0 }));
            t.mechanism = Some(Box::new(Reads { kind: says("treasury.outstanding"), what: Counts::CreditOutstanding }));
            t
        },
        // Money A1, 5 A4: every asset is somebody's liability, published party by party.
        works("money", AT_CORPORATE_ACTIONS_SLOT, Box::new(Owed { kind: says("money.owed") })),
        works("sovereign", AT_MARKETS, Box::new(Reads { kind: says("sovereign.lines"), what: Counts::LinesThatPrinted })),
        works("bank_capital", AT_REVALUATION, Box::new(Reads { kind: says("bank_capital.alive"), what: Counts::PartiesAlive })),
        works("bank_funding", AT_REVALUATION, Box::new(Reads { kind: says("bank_funding.credit"), what: Counts::CreditOutstanding })),
        // §6, XI-9: THE ONE THE WHOLE CREDIT SIDE RESTS ON — what falls due is paid, or it is an arrear.
        works("lending", AT_CORPORATE_ACTIONS_SLOT, Box::new(Servicing { days_per_period: w.days_per_period })),
        works("capital_programme", AT_MARKETS, Box::new(Closing { kind: afoot::CAPITAL_PROGRAMME, says: says("plant.built") })),
        works("cost_of_capital", AT_REVALUATION, Box::new(Reads { kind: says("cost_of_capital.lines"), what: Counts::LinesThatPrinted })),
        works("short_term_debt", AT_MARKETS, Box::new(Reads { kind: says("short_term_debt.out"), what: Counts::CreditOutstanding })),
        works("corporate_credit", AT_MARKETS, Box::new(Reads { kind: says("corporate_credit.out"), what: Counts::CreditOutstanding })),
        // ── The holders ─────────────────────────────────────────────────────────────────────────
        {
            let mut f = posts("funds", AT_MARKETS, Box::new(FundMandates { may_hold: w.lines.clone(), will_pay: 1.1 }));
            f.mechanism = Some(Box::new(Reads { kind: says("funds.mandates"), what: Counts::AgreementsLive }));
            f
        },
        {
            let mut i = posts("insurers", AT_MARKETS, Box::new(InsurerMatching { long_lines: w.lines.clone(), will_pay: 1.05 }));
            i.mechanism = Some(Box::new(Reads { kind: says("insurers.policies"), what: Counts::AgreementsLive }));
            i
        },
        {
            let mut d = posts("dealing", AT_MARKETS, Box::new(Dealers { around: 1.0, width: 0.02, limit: 1_000.0, lines: w.lines.clone() }));
            d.mechanism = Some(Box::new(Reads { kind: says("dealing.lines"), what: Counts::LinesThatPrinted }));
            d
        },
        works("hedge_funds", AT_MARKETS, Box::new(Reads { kind: says("hedge_funds.alive"), what: Counts::PartiesAlive })),
        works("private_equity", AT_MARKETS, Box::new(Closing { kind: afoot::TAKEOVER, says: says("takeover.closed") })),
        works("prime_brokerage", AT_REVALUATION, Box::new(Reads { kind: says("prime_brokerage.books"), what: Counts::AgreementsLive })),
        works("redeemable", AT_MARKETS, Box::new(Reads { kind: says("redeemable.subscriptions"), what: Counts::AgreementsLive })),
        works("equity", AT_MARKETS, Box::new(Closing { kind: afoot::FLOTATION, says: says("equity.floated") })),
        works("securities_lending", AT_MARKETS, Box::new(Reads { kind: says("securities_lending.loans"), what: Counts::AgreementsLive })),
        works("securitisation", AT_MARKETS, Box::new(Closing { kind: afoot::SECURITISATION, says: says("pool.closed") })),
        // ── The instrument families that settle against what the books printed ──────────────────
        works("derivative_layer", AT_REVALUATION, Box::new(Reads { kind: says("derivative_layer.open"), what: Counts::AgreementsLive })),
        works("cds", AT_MARKETS, Box::new(Reads { kind: says("cds.open"), what: Counts::AgreementsLive })),
        works("irs", AT_MARKETS, Box::new(Servicing { days_per_period: w.days_per_period })),
        works("fx_forwards", AT_MARKETS, Box::new(Reads { kind: says("fx_forwards.open"), what: Counts::AgreementsLive })),
        works("spot_fx", AT_MARKETS, Box::new(Reads { kind: says("spot_fx.lines"), what: Counts::LinesThatPrinted })),
        works("currency", AT_MARKETS, Box::new(Owed { kind: says("currency.owed") })),
        works("cross_border", AT_REVALUATION, Box::new(Reads { kind: says("cross_border.lines"), what: Counts::LinesThatPrinted })),
        // ── The reads over what the books produced ──────────────────────────────────────────────
        works("benchmarks", AT_REVALUATION, Box::new(Reads { kind: says("benchmarks.printed"), what: Counts::LinesThatPrinted })),
        works("ratings", AT_REVALUATION, Box::new(Reads { kind: says("ratings.obligors"), what: Counts::PartiesAlive })),
        works("reporting", AT_REVALUATION, Box::new(Reporting { kind: says("reporting.result") })),
        works("second_opinion", AT_REVALUATION, Box::new(Reads { kind: says("second_opinion.lines"), what: Counts::LinesThatPrinted })),
        works("observer", AT_REVALUATION, Box::new(Reads { kind: says("observer.alive"), what: Counts::PartiesAlive })),
        // §46: every deciding party forms its own outlook from its own history. They disagree.
        works("expectations", AT_REVALUATION, Box::new(Forming { memory: 0.3 })),
        // ── The events that end things ──────────────────────────────────────────────────────────
        works("loss", AT_REVALUATION, Box::new(Reads { kind: says("loss.alive"), what: Counts::PartiesAlive })),
        works("forced_sale", AT_MARKETS, Box::new(Closing { kind: afoot::WORKOUT, says: says("workout.closed") })),
        works("mortality", AT_REVALUATION, Box::new(Reads { kind: says("mortality.alive"), what: Counts::PartiesAlive })),
        works("estate", AT_REVALUATION, Box::new(Reads { kind: says("estate.open"), what: Counts::AgreementsLive })),
        works("control", AT_MARKETS, Box::new(Closing { kind: afoot::BUY_BACK, says: says("buy_back.closed") })),
        works("polity", AT_REVALUATION, Box::new(Closing { kind: afoot::ELECTION, says: says("election.called") })),
    ];
    // Law 10, Law 4: each declaration gets its OWN slot, so no two systems declare the same phase.
    // The order of this list is the order they were wired in, and it is the only thing that decides it.
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
    ///
    /// **Everybody banks somewhere** (Money D2, 22b.9a): a party's account is the money ITS BANK
    /// issues, so a fixture whose parties bank nowhere is a fixture where nobody holds any money at
    /// all — which is what this one was until the book stopped naming a cash line for them.
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
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let bank = w.parties.add(kinds::BANK, RegionId::at(0), cb, Representation::Named, 1, 0);
        let reserves = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let cash = w.instruments.issue(bank, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let bread = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), bank, Representation::Named, 1, 0);
        let household = w.parties.add(kinds::HOUSEHOLD, RegionId::at(0), bank, Representation::Cell, 500, 0);
        w.register.credit(firm, bread, 400.0, 0.9, 0);
        w.register.money_delta(household, cash, 600.0);
        w.open_book(book_of(bread), bread, CurrencyCode::at(0), PriceRule::SellersCompete);
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
    }

    #[test]
    fn a_firm_posts_what_it_holds_and_a_household_bids_what_it_can_fund() {
        // §37 C1, §41 C1.d: the seller offers units it HAS; the buyer bids for what it can pay for.
        let s = world();
        let sellers = GoodsSellers { will_take: 1.0 };
        let buyers = HouseholdBuyers { will_pay: 1.2, basket: vec![s.bread] };
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
        // 600 of money at 1.2 apiece is 500 units it can actually pay for.
        assert_eq!(bids[0].qty, 500);
    }

    #[test]
    fn a_household_with_no_money_is_in_no_book() {
        // Law 6: not a rule about households — it is what having nothing to pay with means.
        let s = world();
        let buyers = HouseholdBuyers { will_pay: 1.2, basket: vec![s.bread] };
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
            posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: 1.0 })).slotted(FIRST_SLOT),
            posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: 1.2, basket: vec![bread] })).slotted(FIRST_SLOT + 1),
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
        // §26 C2: long already means it bids lower AND offers lower — how a desk mean-reverts its
        // book without anyone telling it to, and why order flow moves prices.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let dealer = s.w.parties.add(kinds::DEALER, RegionId::at(0), s.bank, Representation::Named, 1, 0);
        s.w.register.money_delta(dealer, cash, 5_000.0);
        let d = Dealers { around: 1.0, width: 0.02, limit: 1_000.0, lines: vec![bread] };

        let flat = seen(&s, dealer);
        let quoted_flat = d.orders(&flat, book_of(bread));
        let bid_flat = quoted_flat.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();

        s.w.register.credit(dealer, bread, 500.0, 1.0, 0);
        let long = seen(&s, dealer);
        let quoted_long = d.orders(&long, book_of(bread));
        let bid_long = quoted_long.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();
        assert!(bid_long < bid_flat);
        // And it is offering as well as bidding, which is what makes it two-sided (§26 A2).
        assert!(quoted_long.iter().any(|o| o.side == Side::Sell));
    }

    #[test]
    fn a_dealer_at_its_limit_stops_quoting_rather_than_absorbing_more() {
        // §26 D1: a limit that never binds is not a limit.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let dealer = s.w.parties.add(kinds::DEALER, RegionId::at(0), s.bank, Representation::Named, 1, 0);
        s.w.register.money_delta(dealer, cash, 5_000.0);
        s.w.register.credit(dealer, bread, 1_200.0, 1.0, 0);
        let d = Dealers { around: 1.0, width: 0.02, limit: 1_000.0, lines: vec![bread] };
        let view = seen(&s, dealer);
        assert!(d.orders(&view, book_of(bread)).is_empty());
    }

    #[test]
    fn a_bank_short_of_its_own_buffer_bids_and_one_over_it_offers() {
        // §11 B1: who lends and who borrows is the OUTCOME of the schedules, never a rule that
        // surplus banks lend. Two banks, the same book, opposite sides — from their own positions.
        let mut s = world();
        // Money D2: a bank banks at the central bank, so what it settles in is RESERVES — which is
        // what the overnight market is a market in.
        let reserves = s.reserves;
        let short = s.w.parties.add(kinds::BANK, RegionId::at(0), s.cb, Representation::Named, 1, 0);
        let flush = s.w.parties.add(kinds::BANK, RegionId::at(0), s.cb, Representation::Named, 1, 0);
        s.w.register.money_delta(short, reserves, 40.0);
        s.w.register.money_delta(flush, reserves, 900.0);
        let book = book_of(reserves);
        let m = MoneyMarketBanks { buffer: 100.0, lends_at: 1.0, borrows_at: 1.0, book: Some(book) };

        assert_eq!(m.orders(&seen(&s, short), book)[0].side, Side::Buy);
        assert_eq!(m.orders(&seen(&s, flush), book)[0].side, Side::Sell);
    }

    #[test]
    fn a_fund_cannot_buy_outside_its_mandate() {
        // §13 A4: the mandate is a real constraint on what it buys, not a label.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let fund = s.w.parties.add(kinds::FUND, RegionId::at(0), s.bank, Representation::Named, 1, 0);
        s.w.register.money_delta(fund, cash, 1_000.0);
        let allowed = FundMandates { may_hold: vec![bread], will_pay: 1.1 };
        let forbidden = FundMandates { may_hold: Vec::new(), will_pay: 1.1 };
        let view = seen(&s, fund);
        assert!(!allowed.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.markets(&view).is_empty());
    }

    #[test]
    fn every_system_of_the_world_is_wired_exactly_once() {
        // ARCHITECTURE 4.9b: adding a system is a row, and a module not in this list does not run.
        let mut kinds = Names::new();
        let all = all(
            &Wiring {
                basket: vec![InstrumentId::at(1)],
                lines: vec![InstrumentId::at(1)],
                overnight: Some(InstrumentId::at(2)),
                paper: Some(InstrumentId::at(3)),
                days_per_period: 7,
            },
            &mut kinds,
        );
        let mut names: Vec<&str> = all.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(names.len(), before, "a system wired twice would run twice");
        // Fifty rows for the spec's forty-seven systems, and the difference is not slack: §37 is two
        // modules (the recipe that makes a thing and the market that sells it), §20 and §21 are one
        // (`commodities`), and XI-7 shares `benchmarks` with §22. The count that matters is that
        // every ported module is here — a module not in this list does not run.
        assert_eq!(before, 50, "every ported module is wired, and nothing is wired twice");
        // And every declaration has its own slot, or two of them would be the same phase (Law 4).
        let mut slots: Vec<u32> = all.iter().map(|s| s.slot).collect();
        slots.sort_unstable();
        let filled = slots.len();
        slots.dedup();
        assert_eq!(slots.len(), filled, "two systems sharing a declaration slot is one phase, twice");
    }

    #[test]
    fn a_system_that_reads_posts_nothing() {
        // Appendix B: no demand added to clear. The benchmarks and the ratings read what the books
        // produced; handing them a schedule would be inventing demand nobody has.
        let r = reads("benchmarks", AT_REVALUATION);
        assert!(r.participants().is_empty());
        assert_eq!(r.phases().len(), 1);
    }

    #[test]
    fn every_wired_system_carries_a_mechanism_and_none_is_dead() {
        // **The thing that was not true.** `wire_up` collected fifty phase declarations and sealed
        // the order, and `World::step` ran books only — so forty-three of these rows did literally
        // nothing and the world reported as wired. A row with no mechanism and no participant is a
        // row the loop never reaches, and this is what says so.
        let mut kinds = Names::new();
        let all = all(
            &Wiring {
                basket: vec![InstrumentId::at(1)],
                lines: vec![InstrumentId::at(1)],
                overnight: Some(InstrumentId::at(2)),
                paper: Some(InstrumentId::at(3)),
                days_per_period: 7,
            },
            &mut kinds,
        );
        for row in &all {
            assert!(
                row.mechanism.is_some(),
                "{} is wired and has no work to do in a period",
                row.name
            );
        }
    }

    #[test]
    fn a_period_of_the_wired_world_runs_every_one_of_them() {
        // ARCHITECTURE 4.9b end to end: fifty systems, fifty phases, and `ran` counts the ones that
        // actually ran. A world of declarations cannot report as a world that works.
        let mut s = world();
        let bread = s.bread;
        // Law 4: the kinds they publish under are declared on THE WORLD'S OWN journal. A second
        // register of names would give every event an id that means something else there.
        let wired = all(
            &Wiring {
                basket: vec![bread],
                lines: vec![bread],
                overnight: None,
                paper: None,
                days_per_period: 7,
            },
            &mut s.w.journal.kinds,
        );
        let systems: Vec<&dyn System> = wired.iter().map(|w| w as &dyn System).collect();
        s.w.wire_up(&systems);
        let did = s.w.step(&systems);
        assert_eq!(did.ran, 50, "every wired system ran its phase");
    }
}
