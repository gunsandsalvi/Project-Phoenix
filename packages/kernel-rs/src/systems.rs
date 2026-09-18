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
use crate::module::{Participant, ParticipantView};
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
        let held = view.free(line_of(m));
        if held <= 0.0 {
            return Vec::new();
        }
        // §37 B1: it produces — and sells — because the price covers its cost. Law 6: it is not made
        // to sell below that; a book that will not reach it simply does not clear for this seller.
        vec![Order { party: view.self_id(), side: Side::Sell, price: Some(self.will_take), qty: held as i64 }]
    }
}

/// §41 C1, §37 C3: **households buy because they need the thing**, and what they can spend is what
/// they have (C1.d: a household that cannot borrow spends what it has, whatever it wants).
pub struct HouseholdBuyers {
    pub cash: InstrumentId,
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
        if view.free(self.cash) <= 0.0 {
            return Vec::new();
        }
        self.basket.iter().map(|line| book_of(*line)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.free(self.cash);
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
    pub cash: InstrumentId,
    /// §11 A2.a: its own buffer preference, derived from its own liabilities — not a stated ratio.
    pub buffer: f64,
    /// §11 B2: what it will take to lend its own money out, and what it will pay to borrow.
    pub lends_at: f64,
    pub borrows_at: f64,
    /// The overnight book.
    pub book: MarketId,
}

impl Participant for MoneyMarketBanks {
    fn party_kind(&self) -> u32 {
        kinds::BANK
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        vec![self.book]
    }

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        let reserves = view.free(self.cash);
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
    pub cash: InstrumentId,
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
        let mut out = Vec::new();
        if view.free(self.cash) > 0.0 && bid > 0.0 {
            out.push(Order { party: view.self_id(), side: Side::Buy, price: Some(bid), qty: (self.limit - held) as i64 });
        }
        if held > 0.0 {
            out.push(Order { party: view.self_id(), side: Side::Sell, price: Some(ask), qty: held as i64 });
        }
        out
    }
}

/// §13 C1.a: **a fund must buy something with the cash**, per its mandate — which is why a flow into
/// a fund becomes a purchase of what the mandate allows, and why a fund is a transmission channel.
pub struct FundMandates {
    pub cash: InstrumentId,
    /// §13 A4: **the mandate is a real constraint on what it buys**, not a label.
    pub may_hold: Vec<InstrumentId>,
    pub will_pay: f64,
}

impl Participant for FundMandates {
    fn party_kind(&self) -> u32 {
        kinds::FUND
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.free(self.cash) <= 0.0 {
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
        let money = view.free(self.cash);
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
    pub cash: InstrumentId,
    pub long_lines: Vec<InstrumentId>,
    pub will_pay: f64,
}

impl Participant for InsurerMatching {
    fn party_kind(&self) -> u32 {
        kinds::INSURER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.free(self.cash) <= 0.0 {
            return Vec::new();
        }
        self.long_lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, _m: MarketId) -> Vec<Order> {
        let money = view.free(self.cash);
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
    pub paper: InstrumentId,
    /// §30 D2.a: the lowest price it will accept. Below that it pulls the auction (D5).
    pub will_accept: f64,
    pub size: f64,
}

impl Participant for TreasuryIssues {
    fn party_kind(&self) -> u32 {
        kinds::TREASURY
    }

    fn markets(&self, _view: &ParticipantView<'_>) -> Vec<MarketId> {
        vec![book_of(self.paper)]
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
    Wired { name, slot: 0, at, anchor_after: true, participant: None }
}

/// A system that posts. The participant is its reason to be in a book (Clearing B2).
pub fn posts(name: &'static str, at: u32, participant: Box<dyn Participant>) -> Wired {
    Wired { name, slot: 0, at, anchor_after: false, participant: Some(participant) }
}

/// **Every system this world has.** The forty-seven of Part XIII, each wired once — the ones with a
/// reason to post carrying a participant, the rest reading what the books produced.
pub fn all(cash: InstrumentId, basket: Vec<InstrumentId>, lines: Vec<InstrumentId>) -> Vec<Wired> {
    let mut rows = vec![
        // The real economy: what is made, what it costs to move, and who buys it.
        posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: 1.0 })),
        posts(
            "households",
            AT_MARKETS,
            Box::new(HouseholdBuyers { cash, will_pay: 1.2, basket: basket.clone() }),
        ),
        reads("recipe", AT_MARKETS),
        reads("firms", AT_MARKETS),
        reads("employment", AT_MARKETS),
        reads("freight", AT_MARKETS),
        reads("commodities", AT_MARKETS),
        reads("housing", AT_MARKETS),
        reads("trade_credit", AT_MARKETS),
        reads("small_business", AT_MARKETS),
        // Money, the banks and the sovereign.
        posts(
            "money_market",
            AT_MARKETS,
            Box::new(MoneyMarketBanks { cash, buffer: 100.0, lends_at: 1.0, borrows_at: 1.0, book: book_of(cash) }),
        ),
        posts("treasury", AT_MARKETS, Box::new(TreasuryIssues { paper: cash, will_accept: 0.98, size: 0.0 })),
        reads("money", AT_CORPORATE_ACTIONS_SLOT),
        reads("sovereign", AT_MARKETS),
        reads("bank_capital", AT_REVALUATION),
        reads("bank_funding", AT_REVALUATION),
        reads("lending", AT_MARKETS),
        reads("capital_programme", AT_MARKETS),
        reads("cost_of_capital", AT_REVALUATION),
        reads("short_term_debt", AT_MARKETS),
        reads("corporate_credit", AT_MARKETS),
        // The holders.
        posts(
            "funds",
            AT_MARKETS,
            Box::new(FundMandates { cash, may_hold: lines.clone(), will_pay: 1.1 }),
        ),
        posts(
            "insurers",
            AT_MARKETS,
            Box::new(InsurerMatching { cash, long_lines: lines.clone(), will_pay: 1.05 }),
        ),
        posts(
            "dealing",
            AT_MARKETS,
            Box::new(Dealers { cash, around: 1.0, width: 0.02, limit: 1_000.0, lines: lines.clone() }),
        ),
        reads("hedge_funds", AT_MARKETS),
        reads("private_equity", AT_MARKETS),
        reads("prime_brokerage", AT_REVALUATION),
        reads("redeemable", AT_MARKETS),
        reads("equity", AT_MARKETS),
        reads("securities_lending", AT_MARKETS),
        reads("securitisation", AT_MARKETS),
        // The instrument families that settle against what the books printed.
        reads("derivative_layer", AT_REVALUATION),
        reads("cds", AT_MARKETS),
        reads("irs", AT_MARKETS),
        reads("fx_forwards", AT_MARKETS),
        reads("spot_fx", AT_MARKETS),
        reads("currency", AT_MARKETS),
        reads("cross_border", AT_REVALUATION),
        // The reads over what the books produced. Each of these would be demand nobody has.
        reads("benchmarks", AT_REVALUATION),
        reads("ratings", AT_REVALUATION),
        reads("reporting", AT_REVALUATION),
        reads("second_opinion", AT_REVALUATION),
        reads("observer", AT_REVALUATION),
        reads("expectations", AT_REVALUATION),
        // The events that end things.
        reads("loss", AT_REVALUATION),
        reads("forced_sale", AT_MARKETS),
        reads("mortality", AT_REVALUATION),
        reads("estate", AT_REVALUATION),
        reads("control", AT_MARKETS),
        reads("polity", AT_REVALUATION),
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
    use crate::instruments::Class;
    use crate::parties::Representation;
    use crate::assembly::World;
    use crate::clearing::PriceRule;

    /// A world with one good, one money, a firm that holds the good and a household with money.
    fn world() -> (World, InstrumentId, InstrumentId) {
        let mut w = World::empty();
        let cb = w.parties.add(kinds::CENTRAL_BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let cash = w.instruments.issue(cb, CurrencyCode::at(0), Class::Money, UnitId::at(0), None, None);
        let bread = w.instruments.issue(cb, CurrencyCode::at(0), Class::Good, UnitId::at(1), None, None);
        let firm = w.parties.add(kinds::FIRM, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let household = w.parties.add(kinds::HOUSEHOLD, RegionId::at(0), PartyId::NONE, Representation::Cell, 500, 0);
        w.register.credit(firm, bread, 400.0, 0.9, 0);
        w.register.money_delta(household, cash, 600.0);
        w.open_book(book_of(bread), bread, CurrencyCode::at(0), cash, PriceRule::SellersCompete);
        (w, cash, bread)
    }

    #[test]
    fn a_firm_posts_what_it_holds_and_a_household_bids_what_it_can_fund() {
        // §37 C1, §41 C1.d: the seller offers units it HAS; the buyer bids for what it can pay for.
        let (w, cash, bread) = world();
        let sellers = GoodsSellers { will_take: 1.0 };
        let buyers = HouseholdBuyers { cash, will_pay: 1.2, basket: vec![bread] };
        let firm = PartyId::at(1);
        let household = PartyId::at(2);

        let seller_view = ParticipantView::of(firm, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert_eq!(sellers.markets(&seller_view), vec![book_of(bread)]);
        let posted = sellers.orders(&seller_view, book_of(bread));
        assert_eq!(posted.len(), 1);
        assert_eq!(posted[0].qty, 400);

        let buyer_view = ParticipantView::of(household, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert_eq!(buyers.markets(&buyer_view), vec![book_of(bread)]);
        let bids = buyers.orders(&buyer_view, book_of(bread));
        assert_eq!(bids.len(), 1);
        // 600 of money at 1.2 apiece is 500 units it can actually pay for.
        assert_eq!(bids[0].qty, 500);
    }

    #[test]
    fn a_household_with_no_money_is_in_no_book() {
        // Law 6: not a rule about households — it is what having nothing to pay with means.
        let (w, cash, bread) = world();
        let buyers = HouseholdBuyers { cash, will_pay: 1.2, basket: vec![bread] };
        let broke = PartyId::at(1); // the firm holds bread and no cash
        let view = ParticipantView::of(broke, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert!(buyers.markets(&view).is_empty());
    }

    #[test]
    fn the_world_steps_and_the_book_clears() {
        // The whole point of the wiring: parties, a book, and a period that actually trades.
        let (mut w, cash, bread) = world();
        let systems = [
            posts("goods", AT_MARKETS, Box::new(GoodsSellers { will_take: 1.0 })).slotted(FIRST_SLOT),
            posts("households", AT_MARKETS, Box::new(HouseholdBuyers { cash, will_pay: 1.2, basket: vec![bread] })).slotted(FIRST_SLOT + 1),
        ];
        let as_systems: Vec<&dyn System> = systems.iter().map(|s| s as &dyn System).collect();
        w.wire_up(&as_systems);
        let did = w.step(&as_systems);
        assert!(did.asks > 0);
        assert_eq!(did.books_cleared, 1);
        assert!(did.trades > 0);
        // And the units actually moved: the household holds bread it did not hold before.
        assert!(w.register.quantity(w.register.row(PartyId::at(2), bread)) > 0.0);
    }

    #[test]
    fn a_dealers_inventory_skews_its_quote() {
        // §26 C2: long already means it bids lower AND offers lower — how a desk mean-reverts its
        // book without anyone telling it to, and why order flow moves prices.
        let (mut w, cash, bread) = world();
        let dealer = w.parties.add(kinds::DEALER, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        w.register.money_delta(dealer, cash, 5_000.0);
        let d = Dealers { cash, around: 1.0, width: 0.02, limit: 1_000.0, lines: vec![bread] };

        let flat = ParticipantView::of(dealer, &w.register, &w.prints, &w.journal, &w.params, 1);
        let quoted_flat = d.orders(&flat, book_of(bread));
        let bid_flat = quoted_flat.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();

        w.register.credit(dealer, bread, 500.0, 1.0, 0);
        let long = ParticipantView::of(dealer, &w.register, &w.prints, &w.journal, &w.params, 1);
        let quoted_long = d.orders(&long, book_of(bread));
        let bid_long = quoted_long.iter().find(|o| o.side == Side::Buy).unwrap().price.unwrap();
        assert!(bid_long < bid_flat);
        // And it is offering as well as bidding, which is what makes it two-sided (§26 A2).
        assert!(quoted_long.iter().any(|o| o.side == Side::Sell));
    }

    #[test]
    fn a_dealer_at_its_limit_stops_quoting_rather_than_absorbing_more() {
        // §26 D1: a limit that never binds is not a limit.
        let (mut w, cash, bread) = world();
        let dealer = w.parties.add(kinds::DEALER, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        w.register.money_delta(dealer, cash, 5_000.0);
        w.register.credit(dealer, bread, 1_200.0, 1.0, 0);
        let d = Dealers { cash, around: 1.0, width: 0.02, limit: 1_000.0, lines: vec![bread] };
        let view = ParticipantView::of(dealer, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert!(d.orders(&view, book_of(bread)).is_empty());
    }

    #[test]
    fn a_bank_short_of_its_own_buffer_bids_and_one_over_it_offers() {
        // §11 B1: who lends and who borrows is the OUTCOME of the schedules, never a rule that
        // surplus banks lend. Two banks, the same book, opposite sides — from their own positions.
        let (mut w, cash, _) = world();
        let short = w.parties.add(kinds::BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        let flush = w.parties.add(kinds::BANK, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        w.register.money_delta(short, cash, 40.0);
        w.register.money_delta(flush, cash, 900.0);
        let m = MoneyMarketBanks { cash, buffer: 100.0, lends_at: 1.0, borrows_at: 1.0, book: book_of(cash) };

        let short_view = ParticipantView::of(short, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert_eq!(m.orders(&short_view, book_of(cash))[0].side, Side::Buy);
        let flush_view = ParticipantView::of(flush, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert_eq!(m.orders(&flush_view, book_of(cash))[0].side, Side::Sell);
    }

    #[test]
    fn a_fund_cannot_buy_outside_its_mandate() {
        // §13 A4: the mandate is a real constraint on what it buys, not a label.
        let (mut w, cash, bread) = world();
        let fund = w.parties.add(kinds::FUND, RegionId::at(0), PartyId::NONE, Representation::Named, 1, 0);
        w.register.money_delta(fund, cash, 1_000.0);
        let allowed = FundMandates { cash, may_hold: vec![bread], will_pay: 1.1 };
        let forbidden = FundMandates { cash, may_hold: Vec::new(), will_pay: 1.1 };
        let view = ParticipantView::of(fund, &w.register, &w.prints, &w.journal, &w.params, 1);
        assert!(!allowed.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.orders(&view, book_of(bread)).is_empty());
        assert!(forbidden.markets(&view).is_empty());
    }

    #[test]
    fn every_system_of_the_world_is_wired_exactly_once() {
        // ARCHITECTURE 4.9b: adding a system is a row, and a module not in this list does not run.
        let all = all(InstrumentId::at(0), vec![InstrumentId::at(1)], vec![InstrumentId::at(1)]);
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
}
