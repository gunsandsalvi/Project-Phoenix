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
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{InstrumentId, MarketId};
use crate::ids::Names;
use crate::module::{Mechanism, Participant, ParticipantView};
use crate::params::{Denomination, Dimension, Kind, Owner, ParamDecl, Params};
use crate::mechanisms::funds::{run_as, Run};
use crate::mechanisms::goods::CostFlow;
use crate::running::{afoot, agreed, Closing, Counts, Failing, Fixes, Forming, Funding, Makes, Making, Owed, Ranked, Reads, Reporting, Servicing, Wages, Winding};
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
    /// §37 B1, 21g: the id of what it will take, read through `params`. Its own, and two firms
    /// differ (§32 A3).
    pub will_take: &'static str,
    /// **33 A4.c: whether a good is an input is the HOLDER's question, not the good's.** What a
    /// party's own recipe consumes is an input to it and stock to everybody else — so a firm that can
    /// run a line does not offer the flour it is about to bake, and the same sacks in a merchant's
    /// yard are for sale. Each row is a plant and what the ways of running it draw on; holding the
    /// plant is what makes the question apply, and it is a read of the register (Law 19).
    pub keeps: Vec<(InstrumentId, Vec<InstrumentId>)>,
}

impl Participant for GoodsSellers {
    fn party_kind(&self) -> u32 {
        kinds::FIRM
    }

    /// Law 19: off its OWN rows. A firm is in the book for a line because it HOLDS that line — and
    /// not for a line its own plant is about to consume (33 A4.c).
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
        // Clearing C1, Law 8: **it offers what it holds IN WHOLE PIECES.** A holding is a quantity in
        // its own unit and an order is a count of pieces, so a firm left with part of a loaf has
        // something and has nothing to sell — and an order for none of it is not an order. The first
        // warm-up in which every firm could reach the book is where this turned up (22b.9a).
        let pieces = whole_pieces(view.free(line_of(m)));
        if pieces <= 0 {
            return Vec::new();
        }
        // §37 B1: it produces — and sells — because the price covers its cost. Law 6: it is not made
        // to sell below that; a book that will not reach it simply does not clear for this seller.
        let will_take = view.params().ratio(self.will_take);
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(will_take), qty: pieces }]
    }
}

/// §41 C1, §37 C3: **households buy because they need the thing**, and what they can spend is what
/// they have (C1.d: a household that cannot borrow spends what it has, whatever it wants).
pub struct HouseholdBuyers {
    /// §41 C1, 21g: the id of what it will pay, read through `params`.
    pub will_pay: &'static str,
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
        let will_pay = view.params().ratio(self.will_pay);
        if money <= 0.0 || will_pay <= 0.0 {
            return Vec::new();
        }
        // §41 C1.d: it bids for what it can actually fund. A bid it cannot pay for is not a bid.
        let affordable = whole_pieces(money / will_pay);
        if affordable <= 0 {
            return Vec::new();
        }
        let _ = m;
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(will_pay), qty: affordable }]
    }
}

/// §11 B1: **every bank posts a schedule out of its own position.** Who ends up lending and who ends
/// up borrowing is the OUTCOME — never a rule that surplus banks lend.
pub struct MoneyMarketBanks {
    /// §11 A2.a: its own buffer preference, derived from its own liabilities — not a stated ratio.
    pub buffer: &'static str,
    /// §11 B2: what it will take to lend its own money out, and what it will pay to borrow.
    pub lends_at: &'static str,
    pub borrows_at: &'static str,
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
        // Long: it offers what it has over its own buffer, at its own rate. §11 B1: whether it ends
        // up lending is the book's answer, not this schedule's.
        let lends_at = view.params().per_annum(self.lends_at);
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(lends_at), qty: whole_pieces(spare) }]
    }
}

/// §26 A2: **a dealer quotes a price at which it will buy and a price at which it will sell**, and it
/// is willing to do either. §26 C2: its inventory SKEWS the quote — long already means it bids lower
/// and offers lower, which is how a desk mean-reverts its book without anyone telling it to.
pub struct Dealers {
    /// §26 C1: the quote comes from the desk's own state. This is its mid before the skew.
    pub around: &'static str,
    /// §26 C3, C5: the width it needs, from what carrying the position costs it.
    pub width: &'static str,
    /// §26 D1: what it will carry. When the limit binds it stops quoting rather than absorbing more.
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
        // §26 D1: at its limit it stops quoting. A limit that never binds is not a limit.
        if held.abs() >= limit {
            return Vec::new();
        }
        // §26 C2: long already means it bids lower AND offers lower. The skew is the inventory
        // against the limit — a fact about its own book, not a stated rule.
        let skew = width * (held / limit);
        let bid = around - width - skew;
        let ask = around + width - skew;
        // Clearing C1: in whole pieces, and an order for none of them is not an order — a desk one
        // half-piece from its limit has room for nothing.
        let room = whole_pieces(limit - held);
        let long = whole_pieces(held);
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
    pub will_pay: &'static str,
}

impl Participant for FundMandates {
    fn party_kind(&self) -> u32 {
        kinds::FUND
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // **§13 F3, 21b.1: a pool under nobody's mandate posts nothing**, because a schedule is
        // somebody's (Clearing B2) and there is nobody here whose view the order would be. It is
        // not a rule against acting; it is arithmetic about who is there. What it holds is sold by
        // `running::Winding`, which is the pool's own selling and not a decision it took.
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
        // §13 A4: a line outside the mandate is one it cannot buy, whatever it is worth.
        let line = line_of(m);
        if !self.may_hold.contains(&line) {
            return Vec::new();
        }
        let money = view.own_cash();
        let will_pay = view.params().ratio(self.will_pay);
        let affordable = whole_pieces(money / will_pay);
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(will_pay), qty: affordable }]
    }
}

/// §27 C2.a: **a structural buyer of long bonds** — a one-way demand that exists whatever the price,
/// because its liabilities are long and its assets are not. A real force in that market, and not a
/// preference.
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

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        let will_pay = view.params().ratio(self.will_pay);
        let affordable = whole_pieces(money / will_pay);
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order { party: view.self_id(), side: Side::Buy, price: Some(will_pay), qty: affordable }]
    }
}

/// §30 D2: **the treasury issues into a market that must clear**, choosing the size and the tenor —
/// never the price. It is a seller of its own paper.
pub struct TreasuryIssues {
    /// `Missing` where the treasury has no line to auction in this world.
    pub paper: Option<InstrumentId>,
    /// §30 D2.a: the lowest price it will accept. Below that it pulls the auction (D5).
    pub will_accept: &'static str,
    /// §30 D4.b, 21j.2: **its own buffer** — the reason it is not dependent on every single auction.
    /// A PREFERENCE, and the only declared number the size has left in it.
    pub buffer: &'static str,
    /// One calendar (G3.a): how many days a period is, so *what falls due this period* is a read of
    /// DATES. A RESOLUTION, handed in like `Servicing`'s.
    pub days_per_period: i64,
}

impl Participant for TreasuryIssues {
    fn party_kind(&self) -> u32 {
        kinds::TREASURY
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        self.paper.map(book_of).into_iter().collect()
    }

    /// **§30 D1, D3, XI-9, 21j.2: IT AUCTIONS WHAT IT IS SHORT OF.** The size was a literal `0.0`
    /// written at the assembly site, so the treasury of this world auctioned nothing, ever, while
    /// `treasury::must_raise` sat in the module unread — a declared number of no kind at all beside
    /// a read that existed (Law 2, Law 19). It could not be fixed at the call site, because
    /// `must_raise` wants what falls due and no participant could see that until 21j.1.
    ///
    /// Now it is a READ of its own position: what it must find this period, what it expects to
    /// receive, what it has, and its own buffer. **The sovereign funding constraint binds on
    /// something for the first time** — sequencing step 3, and until now it bound on nothing.
    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        // G3.a: this period, by DATE. The window is the same one `Servicing` pays out of, so what
        // the treasury raises for and what it is asked for are the same obligations (Law 4).
        let from = crate::calendar::Day(view.period() as i64 * self.days_per_period);
        let to = crate::calendar::Day(from.0 + self.days_per_period - 1);
        let _ = from;
        let outlays = view.owes_by(to);
        // C2, C3: receipts are what named payers actually owe it — read off the lines it holds, not
        // a rate applied to an aggregate. A treasury nobody owes receives nothing, which is an
        // answer about this world rather than a number anybody set.
        let receipts = view.owed_to_it_by(to);
        let buffer = view.params().amount(self.buffer, Denomination::Money);
        let size = crate::mechanisms::treasury::must_raise(outlays, receipts, view.own_cash(), buffer);
        // D1: a treasury that is short of nothing does not auction. Law 6 — not a floor under the
        // size, but the absence of a reason to be in the book at all.
        if size <= 0.0 {
            return Vec::new();
        }
        let will_accept = view.params().price(self.will_accept);
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(will_accept), qty: whole_pieces(size) }]
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
    /// 37 A2: **how every good in this world is made**, one row per line, with the plant it is made
    /// with. It replaces the separate `basket` that stood here: what a household consumes is what
    /// somebody makes, and two lists of that are two writers of one fact (Law 4). A world whose
    /// basket named a good no recipe produced was a world asking for something nobody could supply,
    /// and the books that never cleared could not say which of the two lists was wrong.
    pub makes: Vec<Makes>,
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

impl Wiring {
    /// 37 A2, Law 4, Law 19: **the basket is a READ of what the recipes make.** It replaces the list
    /// that used to sit beside them, and the read is what makes "a household wants what somebody
    /// makes" true by construction rather than by two declarations agreeing.
    pub fn basket(&self) -> Vec<InstrumentId> {
        self.makes.iter().map(|m| m.line.makes).collect()
    }

    /// 33 A4.c: each plant with everything the ways of running it draw on — what a holder of that
    /// plant is keeping rather than selling. Another read of the same declaration (Law 4).
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
/// **XI-14, Law 2, 21g: every behaviour-shaping number this world acts on, declared.**
///
/// It is declared HERE, beside the wiring, for the reason the event kinds are: the assembly is what
/// knows the whole list, and a number declared where it is read is a number nobody can count. Each
/// participant and mechanism below holds the ID and reads the value through `params` — so Law 2's
/// question is answerable of every one of them, and `params.shapes()` is a count that can fall.
///
/// **These were bare `f64` fields a caller passed in.** Nothing asked what kind of number each was,
/// and two of them — the outlook's memory, declared at both `households` and `expectations` — were
/// the same number written twice, which is the two-writers defect Law 4 is about and which this makes
/// impossible: the register refuses a second declaration of one id.
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

    // **21i, 33 A4: the one declared number congestion has.** It is the standing area at which a
    // build draws TWICE, which is what gives it a meaning a reader can check rather than an
    // elasticity nobody can derive. A TECHNOLOGY: a fact about building, like a recipe's batch.
    //
    // Everything else in that mechanism is an OUTCOME — how built-up a place is, is a read over the
    // register, and what the extra draw costs is whatever its inputs cleared at (Law 3). No money
    // number is set anywhere in it, which is why this is a draw and not a price.
    say("building.crowds_at", 60.0, "square km standing", Dimension::SquareKm, Kind::Technology, Owner::Model,
        "the ground already covered in a place at which building there draws twice what it does on empty ground");
    // §37 E4: a fact about the thing, not about who holds it.
    say("goods.perishes", 0.01, "share of a lot a period", Dimension::Ratio, Kind::Technology, Owner::Model,
        "the share of a lot that does not survive the period");
    // Clearing C4.c: a seller's own reservation, and a buyer's own limit. Both are what the party
    // will do rather than what the market is, so both are PREFERENCES.
    say("goods.seller.will_take", 1.0, "multiple of what it cost", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the least a holder will take for what it holds, against what the units cost it");
    say("household.will_pay", 1.2, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "the most a cell will pay for what it buys, against what the book last printed");
    say("fund.will_pay", 1.1, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a mandate will pay for a line it may hold");
    say("insurer.will_pay", 1.05, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "what a long-dated matcher will pay for a line that matches its liabilities");
    // §46: the ONE preference expectations are allowed (XI-16), and it was written twice.
    say("outlook.memory", 0.3, "weight on what just happened", Dimension::Ratio, Kind::Preference, Owner::Model,
        "how fast a party corrects its outlook towards what happened — the one preference §46 has");
    // §11: a bank's own liquidity buffer, and what it lends and borrows at overnight.
    // Law 8: a declared AMOUNT is a NAMED amount of money and the read returns pieces, so the two
    // amounts below are stated in units where the literals they replace were piece counts. The same
    // quantity, said in the unit a person declares it in — and it now moves with `piece_shift`.
    say("money_market.buffer", 1.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the balance a bank keeps back before it lends overnight");
    say("money_market.lends_at", 1.0, "per annum", Dimension::PerAnnum, Kind::Preference, Owner::Model,
        "the rate a bank will lend overnight at");
    say("money_market.borrows_at", 1.0, "per annum", Dimension::PerAnnum, Kind::Preference, Owner::Model,
        "the rate a bank will borrow overnight at");
    // §10: a desk's own quote. Its width is what it charges for standing there.
    say("dealer.around", 1.0, "multiple of the last print", Dimension::Ratio, Kind::Preference, Owner::Model,
        "where a desk centres its quote, against what the book last printed");
    say("dealer.width", 0.02, "share of the mid", Dimension::Ratio, Kind::Preference, Owner::Model,
        "half the spread a desk quotes, which is what it charges for standing there");
    say("dealer.limit", 10.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Model,
        "the most a desk will hold of one line");
    // §30 D2.a: the lowest price a treasury will accept before it pulls the auction (D5).
    say("treasury.will_accept", 0.98, "price per unit of par", Dimension::Price, Kind::Policy, Owner::Parliament,
        "the lowest price the treasury will accept before it pulls the auction");
    // §30 D4.b, 21j.2: **the shape is dead.** What it auctions was a literal standing in for
    // `must_raise`, and 21j.1's door let the treasury read what it is short of — so the size is an
    // OUTCOME now and what is left declared is the BUFFER, which is a real preference: the reason a
    // treasury is not dependent on every single auction. The count of shapes falls to zero.
    say("treasury.buffer", 200.0, "money", Dimension::Amount(Denomination::Money), Kind::Preference, Owner::Parliament,
        "the balance the treasury keeps back, which is why one failed auction is not a default");
    // 5 C3.a, 21j.1a: the tenor and the coupon paper is BROUGHT at. Both are market conventions —
    // what the paper is WORTH is what the auction crosses at, and neither of these is that (Law 3).
    say("funding.tenor", 26.0, "periods the paper runs", Dimension::Periods, Kind::Technology, Owner::StandardSetter,
        "how long the paper an issuer brings runs for, which is the convention its market has");
    say("funding.coupon", 0.04, "per annum", Dimension::PerAnnum, Kind::Technology, Owner::StandardSetter,
        "the coupon the paper carries as a TERM, fixed for its life — never what it is worth");
}

pub fn all(w: &Wiring, kinds: &mut Names) -> Vec<Wired> {
    // One event kind per system that publishes a read. Law 4: declared once, here, where the list is.
    let mut says = |name: &str| kinds.declare(name);
    let mut rows = vec![
        // ── The real economy: what is made, what it costs to move, and who buys it ──────────────
        {
            // §37 both posts and works: a firm offers what it holds, and the stock that does not
            // survive the period leaves at what it cost (37 E4).
            let mut goods = posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: "goods.seller.will_take", keeps: w.keeps() }));
            goods.mechanism = Some(Box::new(crate::mechanisms::goods::Perishing { share: "goods.perishes" }));
            goods
        },
        {
            // §41 both posts and works: a cell bids for what it can fund, and forms its outlook from
            // the prices its own lines printed at (§46).
            let mut households =
                posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: "household.will_pay", basket: w.basket() }));
            households.mechanism = Some(Box::new(Forming { memory: "outlook.memory" }));
            households
        },
        // §37 A2, B1–B5: THE ONE SYSTEM THAT MAKES ANYTHING. It was a read of how many lines
        // printed, which is a system reporting on a world it takes no part in.
        works("recipe", AT_CORPORATE_ACTIONS_SLOT, Box::new(Making { makes: w.makes.clone(), flow: CostFlow::FirstInFirstOut, crowds_at: "building.crowds_at" })),
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
                Box::new(MoneyMarketBanks { buffer: "money_market.buffer", lends_at: "money_market.lends_at", borrows_at: "money_market.borrows_at", book: w.overnight.map(book_of) }),
            );
            mm.mechanism = Some(Box::new(Reads { kind: says("money_market.credit"), what: Counts::CreditOutstanding }));
            mm
        },
        {
            // §30 D1, D3, XI-9, 21j: it BRINGS the paper at corporate actions and AUCTIONS it at the
            // markets — two moments in order, because a bill has to exist before anybody bids for it.
            // The `Reads` row that counted credit outstanding is replaced rather than kept: a system
            // that funds itself is not also a system that counts (Law 4, 21j.4).
            let mut t = posts("treasury", AT_MARKETS, Box::new(TreasuryIssues { paper: w.paper, will_accept: "treasury.will_accept", buffer: "treasury.buffer", days_per_period: w.days_per_period }));
            t.mechanism = Some(Box::new(Funding {
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
            let mut f = posts("funds", AT_MARKETS, Box::new(FundMandates { may_hold: w.lines.clone(), will_pay: "fund.will_pay" }));
            // §13 F3, 21b: a pool whose manager died posts nothing and winds up. It was a count of
            // live mandates, which is a read that reports on a thing it does not do.
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
        // XI-7, 21j.3a: it FIXES on what the overnight book cleared at, and publishes nothing where
        // nothing crossed. A count of how many lines printed was never what a benchmark is for.
        works("benchmarks", AT_REVALUATION, Box::new(Fixes { on: w.overnight.map(book_of), says: says("benchmarks.fixing") })),
        works("ratings", AT_REVALUATION, Box::new(Reads { kind: says("ratings.obligors"), what: Counts::PartiesAlive })),
        works("reporting", AT_REVALUATION, Box::new(Reporting { kind: says("reporting.result") })),
        works("second_opinion", AT_REVALUATION, Box::new(Reads { kind: says("second_opinion.lines"), what: Counts::LinesThatPrinted })),
        works("observer", AT_REVALUATION, Box::new(Reads { kind: says("observer.alive"), what: Counts::PartiesAlive })),
        // §46: every deciding party forms its own outlook from its own history. They disagree.
        works("expectations", AT_REVALUATION, Box::new(Forming { memory: "outlook.memory" })),
        // ── The events that end things ──────────────────────────────────────────────────────────
        works("loss", AT_REVALUATION, Box::new(Reads { kind: says("loss.alive"), what: Counts::PartiesAlive })),
        works("forced_sale", AT_MARKETS, Box::new(Closing { kind: afoot::WORKOUT, says: says("workout.closed") })),
        // XI-3, 21j.3a: a party whose liabilities exceed its assets CEASES. Counting who was alive was
        // the opposite of the read this system is for, and nothing in this world had ever died.
        works("mortality", AT_REVALUATION, Box::new(Failing { says: says("mortality.failed") })),
        works("estate", AT_CORPORATE_ACTIONS_SLOT, Box::new(Ranked { says: says("estate.paid") })),
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
        // XI-14: the participants below hold IDs, so the world they are asked in has the numbers.
        // A scale model of the world runs the same declarations, not a second set (Law 4).
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
        // §37 C1, §41 C1.d: the seller offers units it HAS; the buyer bids for what it can pay for.
        let s = world();
        let sellers = GoodsSellers { will_take: "goods.seller.will_take", keeps: Vec::new() };
        let buyers = HouseholdBuyers { will_pay: "household.will_pay", basket: vec![s.bread] };
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
        let buyers = HouseholdBuyers { will_pay: "household.will_pay", basket: vec![s.bread] };
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
            posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: "goods.seller.will_take", keeps: Vec::new() })).slotted(FIRST_SLOT),
            posts("households", AT_MARKETS, Box::new(HouseholdBuyers { will_pay: "household.will_pay", basket: vec![bread] })).slotted(FIRST_SLOT + 1),
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
        let d = Dealers { around: "dealer.around", width: "dealer.width", limit: "dealer.limit", lines: vec![bread] };

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
        let d = Dealers { around: "dealer.around", width: "dealer.width", limit: "dealer.limit", lines: vec![bread] };
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
        let m = MoneyMarketBanks { buffer: "money_market.buffer", lends_at: "money_market.lends_at", borrows_at: "money_market.borrows_at", book: Some(book) };

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
        // F3: and somebody runs it. A pool under nobody's mandate is not a pool with a narrow
        // mandate — it has nobody whose view the order would be, which is the test below this one.
        let manager = s.w.parties.add(kinds::FIRM, RegionId::at(0), s.bank, Representation::Named, 1, 0);
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
        // §13 F3, 21b.1: a schedule is somebody's (Clearing B2). This pool has money, a mandate list
        // it could buy from, and nobody deciding for it — and the difference between it and the one
        // above is a RELATION, not a flag on the fund.
        let mut s = world();
        let (cash, bread) = (s.cash, s.bread);
        let orphan = s.w.parties.add(kinds::FUND, RegionId::at(0), s.bank, Representation::Named, 1, 0);
        s.w.register.money_delta(orphan, cash, 1_000.0);
        let pool = FundMandates { may_hold: vec![bread], will_pay: "fund.will_pay" };
        let view = seen(&s, orphan);
        assert!(pool.markets(&view).is_empty());
        assert!(pool.orders(&view, book_of(bread)).is_empty());

        // And it is not a rule against acting: give it a manager and it is in the book again.
        let manager = s.w.parties.add(kinds::FIRM, RegionId::at(0), s.bank, Representation::Named, 1, 0);
        s.w.agreements.strike(agreed::MANDATE, manager, orphan, &[], crate::calendar::Day(-7), None);
        let view = seen(&s, orphan);
        assert_eq!(pool.markets(&view), vec![book_of(bread)]);
    }

    /// A line that makes one good out of another, with a mill to make it in. The fixtures need one
    /// because the basket is now a READ of what the recipes make (22.3), so a world that makes
    /// nothing is a world whose households want nothing.
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
        let mut kinds = Names::new();
        let all = all(
            &Wiring {
                makes: vec![a_loaf(InstrumentId::at(1), InstrumentId::at(4))],
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
                makes: vec![a_loaf(InstrumentId::at(1), InstrumentId::at(4))],
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
                makes: vec![a_loaf(bread, InstrumentId::at(4))],
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
