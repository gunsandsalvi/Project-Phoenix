//! THE TREASURY: a named party that pays out of a balance, raises **before** it spends, and **has no
//! overdraft at the central bank.**
//!
//! @spec 30 A1 · 30 A1.a · 30 A2 · 30 A3 · 30 A3.a · 30 B1 · 30 B2 · 30 B3 · 30 B3.a · 30 B4 · 30 C1 ·
//! @spec 30 C1.a · 30 C2 · 30 C3 · 30 D1 · 30 D2 · 30 D2.a · 30 D3 · 30 D3.a · 30 D4 · 30 D4.a ·
//! @spec 30 D4.b · 30 D5 · 30 D5.a · 30 D6 · 30 E1 · 30 E2 · 30 E3 · 30 E4 · 30 F1 · 30 F2 · 30 F3 ·
//! @spec XI-9 · Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! **There is no central-bank overdraft** (D3, XI-9). The treasury cannot draw on the central bank to
//! pay for anything: `pay` refuses when the balance is short, and that refusal is the funding constraint
//! that makes every other node here matter. The central bank may hold sovereign debt **bought in the
//! market** for a policy reason (D3.a) — which is a purchase with a seller, not a line of credit.
//!
//! **No forced buyer** (D5.a): nobody is obliged to bid, and no participant absorbs the unsold. **An
//! auction can FAIL** (D5), and the failure has consequences the treasury must then handle — which is
//! why the buffer (D4.b) and the forward-looking programme (D4) exist at all.
//!
//! **Outlays have causes that vary** (B3): a downturn raises them while lowering receipts (B3.a), **which
//! is the whole reason the constraint bites when it does.** They are not a constant anywhere here.
//!
//! **Receipts are the sum of what was actually collected from NAMED PAYERS, never a rate applied to an
//! aggregate** (C1.a, C3) — the base is somebody's income, and the tax is a real flow both ways.
//!
//! **The cost of its debt is a CONSEQUENCE** of what it has issued and at what prices (E3), so heavier
//! issuance into the same demand shows up in the clearing price (E4) and then in the interest outlay
//! (B2) — read from the bonds, never a stated service cost.

use crate::calendar::Day;
use crate::ids::{CurrencyCode, PartyId};

/// A1, A1.a, A2, A3: **a named party with an account like any other** — it pays out of a balance, **and
/// the balance can run low** — in its region's currency, with a balance sheet whose **equity is negative
/// and that is normal; the number is still a read** (A3.a).
#[derive(Clone, Debug)]
pub struct Treasury {
    pub who: PartyId,
    pub money: CurrencyCode,
    pub cash: f64,
    /// D6: debt outstanding is READ from the register — the bonds themselves, not a running total.
    pub bonds: Vec<Bond>,
    /// D4.b: **a cash buffer, because the alternative to a buffer is dependence on every single
    /// auction.**
    pub buffer: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bond {
    pub face: f64,
    pub coupon: f64,
    pub matures: Day,
}

impl Treasury {
    pub fn debt_outstanding(&self) -> f64 {
        self.bonds.iter().map(|b| b.face).sum()
    }

    /// A3.a: equity is a read, and it being negative is normal rather than an error.
    pub fn equity(&self, owns: f64) -> f64 {
        self.cash + owns - self.debt_outstanding()
    }

    /// B2: **interest is an outlay, and it is the sum of what its OWN BONDS pay, read from the register**
    /// (Law 19) — never a stated service cost.
    pub fn interest(&self) -> f64 {
        self.bonds.iter().map(|b| b.face * b.coupon).sum()
    }

    /// D4.a: **it knows its maturity profile, so a wall is foreseeable and pre-funded.**
    pub fn maturing_by(&self, when: Day) -> f64 {
        self.bonds.iter().filter(|b| b.matures <= when).map(|b| b.face).sum()
    }
}

/// B1, B3: **it spends on named things — transfers to households, purchases of goods, wages — and the
/// causes VARY**: the cycle, unemployment, policy. F1: spending is somebody's income.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Outlay {
    pub to: PartyId,
    pub amount: f64,
    pub because: Cause,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    /// B3: transfers that rise when more people are out of work.
    Unemployment,
    /// The programme's own size and composition, which the polity sets (§47).
    Programme,
    Wages,
    /// B4: **maturing debt must be repaid in full, in cash, on its date**, and it is the largest of them.
    Redemption,
}

/// B3, B3.a: **a downturn raises outlays while lowering receipts**, which is why the constraint bites
/// when it does. The cyclical part is read from how many are actually out of work, never from a rate.
pub fn outlays(programme: f64, out_of_work: f64, per_head: f64, wages: f64, maturing: f64) -> f64 {
    programme + out_of_work * per_head + wages + maturing
}

/// C1, C1.a, C3: **taxes levied on real bases, paid by NAMED PAYERS out of their accounts** — a real flow
/// both ways (Law 5). **Receipts are the sum of what was actually collected, never a rate applied to an
/// aggregate.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Collected {
    pub from: PartyId,
    pub base: f64,
    pub at_rate: f64,
}

impl Collected {
    pub fn amount(&self) -> f64 {
        self.base * self.at_rate
    }
}

/// C2, C3: **receipts follow the economy — they fall when income and spending fall** — because they are
/// the sum of what named payers actually paid (Law 19).
pub fn receipts(collected: &[Collected]) -> f64 {
    collected.iter().map(|c| c.amount()).sum()
}

/// D1, XI-9: **outlays minus receipts is what must be raised, AND IT MUST BE RAISED BEFORE IT IS
/// SPENT.** That ordering is the constraint; without it causation reverses and the auction carries no
/// information.
pub fn must_raise(outlays: f64, receipts: f64, cash: f64, buffer: f64) -> f64 {
    let gap = outlays - receipts;
    // It raises the gap AND what it needs to get back to its own buffer — the buffer is the reason it
    // is not dependent on every single auction (D4.b).
    let restock = buffer - (cash - gap);
    if restock > 0.0 {
        gap + restock
    } else {
        gap
    }
}

/// D2, D2.a, D5, D5.a: **it issues into a market that must clear, at whatever price the buyers are
/// willing to pay — the treasury chooses the SIZE and the TENOR, not the price** — and **an auction can
/// fail: nobody is obliged to bid, and no participant absorbs the unsold.**
#[derive(Clone, Debug, PartialEq)]
pub enum Auction {
    Cleared { raised: f64, at_price: f64, to: Vec<(PartyId, f64)> },
    /// D5: it failed, and the treasury must handle the consequence — from its buffer, or by cutting
    /// what it does.
    Failed { raised: f64, short_by: f64 },
}

pub fn issue(size: f64, bids: &[(PartyId, f64, f64)], will_accept_down_to: f64) -> Auction {
    let mut ordered: Vec<&(PartyId, f64, f64)> = bids.iter().collect();
    // Best price first: the treasury sells to whoever pays most.
    ordered.sort_by(|a, b| b.2.total_cmp(&a.2));
    let mut raised = 0.0;
    let mut at_price = 0.0;
    let mut to = Vec::new();
    for (who, amount, price) in ordered {
        if raised >= size || *price < will_accept_down_to {
            break;
        }
        let wants = size - raised;
        let taken = if *amount < wants { *amount } else { wants };
        to.push((*who, taken));
        at_price = *price;
        raised += taken;
    }
    if raised < size {
        // D5.a: nothing absorbs the rest. The shortfall is real and the treasury must deal with it.
        return Auction::Failed { raised, short_by: size - raised };
    }
    Auction::Cleared { raised, at_price, to }
}

/// D3, D3.a: **there is no central-bank overdraft.** The treasury pays out of its balance or it does not
/// pay — and that refusal is the funding constraint XI-9 is about.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Paid {
    Settled { to: PartyId, amount: f64 },
    /// A1.a: the balance ran low. There is no facility behind it.
    CannotPay { short_by: f64 },
}

pub fn pay(cash: f64, o: &Outlay) -> Paid {
    if cash < o.amount {
        return Paid::CannotPay { short_by: o.amount - cash };
    }
    Paid::Settled { to: o.to, amount: o.amount }
}

/// D3.a: **the central bank may hold sovereign debt BOUGHT IN THE MARKET for a policy reason** — which
/// is a purchase with a seller on the other side, not a line of credit. This is what distinguishes them:
/// a purchase names the party it bought from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BoughtInTheMarket {
    pub by: PartyId,
    pub from: PartyId,
    pub face: f64,
    pub at_price: f64,
}

pub fn central_bank_buys(by: PartyId, from: PartyId, face: f64, at_price: f64) -> BoughtInTheMarket {
    assert!(by != from, "30 D3: a central bank buying from the treasury directly is the overdraft again");
    BoughtInTheMarket { by, from, face, at_price }
}

/// E1: **the treasury chooses the maturity mix, and the choice has a trade-off**: short is cheaper on
/// the curve and rolls more often, long costs more and locks it in. A decision, not a schedule.
pub fn rollover_exposure(t: &Treasury, within: Day) -> Option<f64> {
    let outstanding = t.debt_outstanding();
    if outstanding <= 0.0 {
        return None;
    }
    Some(t.maturing_by(within) / outstanding)
}

/// E3, E4: **the cost of its debt is a CONSEQUENCE of what it has issued and at what prices** — so
/// heavier issuance into the same demand shows up in the clearing price. This is the read that connects
/// them: what the last auction cleared at, against what the book would have paid for less.
pub fn cost_of_issuing(cleared_at: f64, face: f64) -> Option<f64> {
    if cleared_at <= 0.0 {
        return None;
    }
    Some(face / cleared_at - 1.0)
}

/// F3: **interest paid is income to HOLDERS**, most of whom are domestic — a real flow with two sides
/// (Law 5), not a line in a statement.
pub fn interest_reaches(t: &Treasury, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let face: f64 = holders.iter().map(|(_, f)| f).sum();
    if face <= 0.0 {
        return Vec::new();
    }
    let paying = t.interest();
    holders.iter().map(|(who, f)| (*who, paying * f / face)).collect()
}

/// D6: **the debt outstanding is the accumulated deficit plus rollovers, READ FROM THE REGISTER** — and
/// a difference between the two is a finding, not a plug. `None` when they agree on derived dust.
pub fn debt_reconciles(read_from_register: f64, accumulated_deficit: f64, terms: usize) -> Option<f64> {
    let off = read_from_register - accumulated_deficit;
    if off.abs() <= crate::num::dust(terms, &[read_from_register, accumulated_deficit]) {
        return None;
    }
    Some(off)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn treasury() -> Treasury {
        Treasury {
            who: party(1),
            money: CurrencyCode::at(0),
            cash: 500.0,
            bonds: vec![
                Bond { face: 1_000.0, coupon: 0.03, matures: Day(100) },
                Bond { face: 2_000.0, coupon: 0.05, matures: Day(900) },
            ],
            buffer: 400.0,
        }
    }

    #[test]
    fn there_is_no_central_bank_overdraft() {
        // D3, A1.a, XI-9: the treasury pays out of its balance or it does not pay, and that refusal is
        // the funding constraint.
        let t = treasury();
        let small = Outlay { to: party(20), amount: 300.0, because: Cause::Programme };
        assert_eq!(pay(t.cash, &small), Paid::Settled { to: party(20), amount: 300.0 });
        let large = Outlay { to: party(20), amount: 900.0, because: Cause::Programme };
        assert_eq!(pay(t.cash, &large), Paid::CannotPay { short_by: 400.0 });
    }

    #[test]
    fn the_central_bank_may_buy_in_the_market_which_is_not_a_line_of_credit() {
        // D3.a: a purchase with a seller on the other side.
        let bought = central_bank_buys(party(2), party(30), 500.0, 0.98);
        assert_eq!(bought.from, party(30));
        assert_ne!(bought.by, bought.from);
    }

    #[test]
    #[should_panic(expected = "is the overdraft again")]
    fn a_central_bank_buying_from_the_treasury_directly_is_refused() {
        central_bank_buys(party(2), party(2), 500.0, 0.98);
    }

    #[test]
    fn an_auction_can_fail_and_nobody_absorbs_the_unsold() {
        // D5, D5.a: no forced buyer. The shortfall is real, and the treasury must deal with it.
        let thin = [(party(40), 300.0, 0.99), (party(41), 200.0, 0.97)];
        assert_eq!(issue(1_000.0, &thin, 0.95), Auction::Failed { raised: 500.0, short_by: 500.0 });
        let deep = [(party(40), 800.0, 0.99), (party(41), 600.0, 0.97)];
        match issue(1_000.0, &deep, 0.95) {
            Auction::Cleared { raised, at_price, to } => {
                assert_eq!(raised, 1_000.0);
                assert_eq!(at_price, 0.97);
                assert_eq!(to[0], (party(40), 800.0));
            }
            other => panic!("expected a cleared auction, got {other:?}"),
        }
    }

    #[test]
    fn the_treasury_chooses_the_size_and_the_tenor_and_not_the_price() {
        // D2.a, E4: heavier issuance into the same demand shows up in the clearing price.
        let book = [(party(40), 400.0, 0.995), (party(41), 400.0, 0.98), (party(42), 400.0, 0.95)];
        let small = issue(400.0, &book, 0.90);
        let large = issue(1_200.0, &book, 0.90);
        match (small, large) {
            (Auction::Cleared { at_price: tight, .. }, Auction::Cleared { at_price: wide, .. }) => {
                assert!(tight > wide);
                // E3: and that price is what the debt then costs.
                assert!(cost_of_issuing(wide, 1.0).unwrap() > cost_of_issuing(tight, 1.0).unwrap());
            }
            other => panic!("expected two cleared auctions, got {other:?}"),
        }
        // A price below what it will accept is a bid it does not take.
        assert!(matches!(issue(1_200.0, &book, 0.99), Auction::Failed { .. }));
    }

    #[test]
    fn a_downturn_raises_outlays_while_lowering_receipts() {
        // B3, B3.a, C2: which is the whole reason the constraint bites when it does. The cyclical part
        // is read from how many are actually out of work.
        let good_times = outlays(1_000.0, 50.0, 4.0, 300.0, 200.0);
        let bad_times = outlays(1_000.0, 400.0, 4.0, 300.0, 200.0);
        assert!(bad_times > good_times);
        let collected_well = [Collected { from: party(20), base: 5_000.0, at_rate: 0.2 }];
        let collected_badly = [Collected { from: party(20), base: 3_000.0, at_rate: 0.2 }];
        assert!(receipts(&collected_badly) < receipts(&collected_well));
        // And the amount to raise moves with both.
        assert!(must_raise(bad_times, receipts(&collected_badly), 500.0, 400.0)
            > must_raise(good_times, receipts(&collected_well), 500.0, 400.0));
    }

    #[test]
    fn receipts_are_the_sum_of_what_named_payers_actually_paid() {
        // C1.a, C3: never a rate applied to an aggregate, and the tax is a real flow both ways.
        let collected = [
            Collected { from: party(20), base: 5_000.0, at_rate: 0.2 },
            Collected { from: party(21), base: 2_000.0, at_rate: 0.3 },
        ];
        assert_eq!(receipts(&collected), 1_600.0);
        assert_eq!(collected[0].from, party(20));
    }

    #[test]
    fn it_raises_the_gap_and_what_it_needs_to_get_back_to_its_own_buffer() {
        // D1, D4.b: the buffer is why it is not dependent on every single auction.
        // A gap of 200 from a cash balance of 500 leaves 300, below the buffer of 400.
        assert_eq!(must_raise(1_000.0, 800.0, 500.0, 400.0), 300.0);
        // With plenty of cash it raises only the gap.
        assert_eq!(must_raise(1_000.0, 800.0, 5_000.0, 400.0), 200.0);
    }

    #[test]
    fn the_interest_outlay_is_read_from_its_own_bonds_and_reaches_the_holders() {
        // B2, F3, Law 19: the sum of what its own bonds pay — and it is income to named holders.
        let t = treasury();
        assert_eq!(t.interest(), 130.0);
        let holders = [(party(40), 1_000.0), (party(41), 2_000.0)];
        let reaching = interest_reaches(&t, &holders);
        assert!((reaching[0].1 - 130.0 / 3.0).abs() <= crate::num::dust(2, &[130.0, 3_000.0]));
        assert!(interest_reaches(&t, &[]).is_empty());
    }

    #[test]
    fn a_wall_is_foreseeable_because_it_knows_its_own_maturity_profile() {
        // D4, D4.a, E1: and the maturity mix is a choice with a trade-off.
        let t = treasury();
        assert_eq!(t.maturing_by(Day(200)), 1_000.0);
        assert_eq!(t.maturing_by(Day(50)), 0.0);
        let soon = rollover_exposure(&t, Day(200)).unwrap();
        assert!(soon > 0.0 && soon < 1.0);
        let debt_free = Treasury { bonds: Vec::new(), ..treasury() };
        assert!(rollover_exposure(&debt_free, Day(200)).is_none());
    }

    #[test]
    fn equity_is_a_read_and_being_negative_is_normal() {
        // A3.a.
        assert!(treasury().equity(200.0) < 0.0);
    }

    #[test]
    fn the_debt_outstanding_is_read_from_the_register_and_a_difference_is_a_finding() {
        // D6: never a plug.
        let t = treasury();
        assert_eq!(t.debt_outstanding(), 3_000.0);
        assert!(debt_reconciles(3_000.0, 3_000.0, 2).is_none());
        assert_eq!(debt_reconciles(3_000.0, 2_800.0, 2), Some(200.0));
    }
}
