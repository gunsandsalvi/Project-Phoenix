//! THE TREASURY: a named party that pays out of a balance, raises before it spends, and has no
//! overdraft at the central bank.
//!
//! @spec 30 A1 · 30 A1.a · 30 A2 · 30 A3 · 30 A3.a · 30 B1 · 30 B2 · 30 B3 · 30 B3.a · 30 B4 · 30 C1 ·
//! @spec 30 C1.a · 30 C2 · 30 C3 · 30 D1 · 30 D2 · 30 D2.a · 30 D3 · 30 D3.a · 30 D4 · 30 D4.a ·
//! @spec 30 D4.b · 30 D5 · 30 D5.a · 30 D6 · 30 E1 · 30 E2 · 30 E3 · 30 E4 · 30 F1 · 30 F2 · 30 F3 ·
//! @spec XI-9 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::calendar::{Convention, Day};
use crate::ids::CurrencyCode;
use crate::instruments::Class;
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::Owing;
use crate::ids::PartyId;

/// A named party with an account like any other — it pays out of a balance, and the balance can run
/// low — in its region's currency, with a balance sheet whose equity is negative and that is normal;
/// the number is still a read.
#[derive(Clone, Debug)]
pub struct Treasury {
    pub who: PartyId,
    pub money: CurrencyCode,
    pub cash: f64,
    /// Debt outstanding is READ from the register — the bonds themselves, not a running total.
    pub bonds: Vec<Bond>,
    /// A cash buffer, because the alternative to a buffer is dependence on every single auction.
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

    /// Equity is a read, and it being negative is normal rather than an error.
    pub fn equity(&self, owns: f64) -> f64 {
        self.cash + owns - self.debt_outstanding()
    }

    /// Interest is an outlay, and it is the sum of what its OWN BONDS pay, read from the register —
    /// never a stated service cost.
    pub fn interest(&self) -> f64 {
        self.bonds.iter().map(|b| b.face * b.coupon).sum()
    }

    /// It knows its maturity profile, so a wall is foreseeable and pre-funded.
    pub fn maturing_by(&self, when: Day) -> f64 {
        self.bonds.iter().filter(|b| b.matures <= when).map(|b| b.face).sum()
    }
}

/// It spends on named things — transfers to households, purchases of goods, wages — and the causes
/// VARY: the cycle, unemployment, policy.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Outlay {
    pub to: PartyId,
    pub amount: f64,
    pub because: Cause,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cause {
    /// Transfers that rise when more people are out of work.
    Unemployment,
    /// The programme's own size and composition, which the polity sets.
    Programme,
    Wages,
    /// Maturing debt must be repaid in full, in cash, on its date, and it is the largest of them.
    Redemption,
}

/// A downturn raises outlays while lowering receipts, which is why the constraint bites when it
/// does.
pub fn outlays(programme: f64, out_of_work: f64, per_head: f64, wages: f64, maturing: f64) -> f64 {
    programme + out_of_work * per_head + wages + maturing
}

/// Taxes levied on real bases, paid by NAMED PAYERS out of their accounts — a real flow both ways.
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

/// Receipts follow the economy — they fall when income and spending fall — because they are the sum
/// of what named payers actually paid.
pub fn receipts(collected: &[Collected]) -> f64 {
    collected.iter().map(|c| c.amount()).sum()
}

/// Outlays minus receipts is what must be raised, AND IT MUST BE RAISED BEFORE IT IS SPENT.
pub fn must_raise(outlays: f64, receipts: f64, cash: f64, buffer: f64) -> f64 {
    let gap = outlays - receipts;
    // It raises the gap AND what it needs to get back to its own buffer — the buffer is the reason
    // it is not dependent on every single auction. Raising the gap is what pays the gap, so the
    // balance ends where it started and the restock is measured against THAT, not against a balance
    // the gap has been taken out of as well.
    let restock = buffer - cash;
    match restock > 0.0 {
        true => gap + restock,
        false => gap,
    }
}

/// It issues into a market that must clear, at whatever price the buyers are willing to pay — the
/// treasury chooses the SIZE and the TENOR, not the price — and an auction can fail: nobody is
/// obliged to bid, and no participant absorbs the unsold.
#[derive(Clone, Debug, PartialEq)]
pub enum Auction {
    Cleared { raised: f64, at_price: f64, to: Vec<(PartyId, f64)> },
    /// It failed, and the treasury must handle the consequence — from its buffer, or by cutting what
    /// it does.
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
        // Nothing absorbs the rest.
        return Auction::Failed { raised, short_by: size - raised };
    }
    Auction::Cleared { raised, at_price, to }
}

/// There is no central-bank overdraft.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Paid {
    Settled { to: PartyId, amount: f64 },
    /// The balance ran low.
    CannotPay { short_by: f64 },
}

pub fn pay(cash: f64, o: &Outlay) -> Paid {
    if cash < o.amount {
        return Paid::CannotPay { short_by: o.amount - cash };
    }
    Paid::Settled { to: o.to, amount: o.amount }
}

/// The central bank may hold sovereign debt BOUGHT IN THE MARKET for a policy reason — which is a
/// purchase with a seller on the other side, not a line of credit.
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

/// The treasury chooses the maturity mix, and the choice has a trade-off: short is cheaper on the
/// curve and rolls more often, long costs more and locks it in.
pub fn rollover_exposure(t: &Treasury, within: Day) -> Option<f64> {
    let outstanding = t.debt_outstanding();
    if outstanding <= 0.0 {
        return None;
    }
    Some(t.maturing_by(within) / outstanding)
}

/// The cost of its debt is a CONSEQUENCE of what it has issued and at what prices — so heavier
/// issuance into the same demand shows up in the clearing price.
pub fn cost_of_issuing(cleared_at: f64, face: f64) -> Option<f64> {
    if cleared_at <= 0.0 {
        return None;
    }
    Some(face / cleared_at - 1.0)
}

/// Interest paid is income to HOLDERS, most of whom are domestic — a real flow with two sides, not a
/// line in a statement.
pub fn interest_reaches(t: &Treasury, holders: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let face: f64 = holders.iter().map(|(_, f)| f).sum();
    if face <= 0.0 {
        return Vec::new();
    }
    let paying = t.interest();
    holders.iter().map(|(who, f)| (*who, paying * f / face)).collect()
}

/// The debt outstanding is the accumulated deficit plus rollovers, READ FROM THE REGISTER — and a
/// difference between the two is a finding, not a plug.
pub fn debt_reconciles(read_from_register: f64, accumulated_deficit: f64, terms: usize) -> Option<f64> {
    let off = read_from_register - accumulated_deficit;
    if off.abs() <= crate::num::dust(terms, &[read_from_register, accumulated_deficit]) {
        return None;
    }
    Some(off)
}

/// THE SOVEREIGN BRINGS ITS PAPER, because what it must raise it must raise before it spends.

pub struct Funding {
    /// WHOSE paper this is, and over what horizon.
    pub of_kinds: &'static [u32],
    /// How far ahead this system's shortfall is read, in days — and from how far ahead.
    pub after: &'static str,
    pub horizon: &'static str,
    /// One calendar: how long a period is, so the window is read from DATES.
    pub days_per_period: i64,
    /// How long the paper runs.
    pub tenor: &'static str,
    /// The coupon the paper carries, as a term.
    pub coupon: &'static str,
    /// The buffer the issuer keeps back.
    pub buffer: &'static str,
    pub says: u32,
}

impl Mechanism for Funding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let from = Day(ctx.period() as i64 * self.days_per_period);
        // The window is read from DATES.
        let to = Day(from.0 + ctx.params().days(self.horizon) as i64 - 1);
        let opens = Day(from.0 + ctx.params().days(self.after) as i64);
        let periods = ctx.params().periods(self.tenor);
        let coupon = ctx.params().per_annum(self.coupon);
        let buffer = ctx.params().amount(self.buffer, crate::params::Denomination::Money);

        let mut bringing: Vec<(PartyId, CurrencyCode, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            let kind = ctx.parties().kind_of(who);
            if !ctx.parties().alive(who) || !self.of_kinds.contains(&kind) {
                continue;
            }
            // The profile answers whether a kind issues paper at all, and a kind with none is a kind
            // nobody has said this of — which is missing rather than a no.
            match ctx.registry().profile(kind) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else { continue };
            // Its own position: what falls due in the window, against what it holds.
            let owes = ctx.schedules().falling_for(who, opens, to);
            let cash = ctx.register().quantity(ctx.register().row(who, money));
            let short = must_raise(owes, 0.0, cash, buffer);
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // It matures on a DATE, so the maturity wall is spread by the dates and not by a count
            // of periods.
            let matures = Day(from.0 + (periods as i64) * self.days_per_period);
            // And it owes its coupon and its principal, written down at issue.
            let years = Convention::Actual365.year_fraction(from, matures);
            ctx.brings(crate::module::Brings {
                issuer: who,
                ccy,
                class: Class::Claim,
                unit: crate::ids::UnitId::at(0),
                coupon: Some(coupon),
                matures: Some(matures),
                units: short,
                // An auction is a CALL — a sealed cross at one level, which is what an auction IS.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
                owing: vec![
                    (matures, short * coupon * years, Owing::Interest),
                    (matures, short, Owing::Principal),
                ],
            });
            ctx.say(self.says, &[who.0], &[(0, Value::Num(short))], true);
        }
    }
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
        // The treasury pays out of its balance or it does not pay, and that refusal is the funding
        // constraint.
        let t = treasury();
        let small = Outlay { to: party(20), amount: 300.0, because: Cause::Programme };
        assert_eq!(pay(t.cash, &small), Paid::Settled { to: party(20), amount: 300.0 });
        let large = Outlay { to: party(20), amount: 900.0, because: Cause::Programme };
        assert_eq!(pay(t.cash, &large), Paid::CannotPay { short_by: 400.0 });
    }

    #[test]
    fn the_central_bank_may_buy_in_the_market_which_is_not_a_line_of_credit() {
        // A purchase with a seller on the other side.
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
        // No forced buyer.
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
        // Heavier issuance into the same demand shows up in the clearing price.
        let book = [(party(40), 400.0, 0.995), (party(41), 400.0, 0.98), (party(42), 400.0, 0.95)];
        let small = issue(400.0, &book, 0.90);
        let large = issue(1_200.0, &book, 0.90);
        match (small, large) {
            (Auction::Cleared { at_price: tight, .. }, Auction::Cleared { at_price: wide, .. }) => {
                assert!(tight > wide);
                // And that price is what the debt then costs.
                assert!(cost_of_issuing(wide, 1.0).unwrap() > cost_of_issuing(tight, 1.0).unwrap());
            }
            other => panic!("expected two cleared auctions, got {other:?}"),
        }
        // A price below what it will accept is a bid it does not take.
        assert!(matches!(issue(1_200.0, &book, 0.99), Auction::Failed { .. }));
    }

    #[test]
    fn a_downturn_raises_outlays_while_lowering_receipts() {
        // Which is the whole reason the constraint bites when it does.
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
        // Never a rate applied to an aggregate, and the tax is a real flow both ways.
        let collected = [
            Collected { from: party(20), base: 5_000.0, at_rate: 0.2 },
            Collected { from: party(21), base: 2_000.0, at_rate: 0.3 },
        ];
        assert_eq!(receipts(&collected), 1_600.0);
        assert_eq!(collected[0].from, party(20));
    }

    #[test]
    fn it_raises_the_gap_and_what_it_needs_to_get_back_to_its_own_buffer() {
        // The buffer is why it is not dependent on every single auction, so a balance below it is
        // raised back up on top of the gap.
        assert_eq!(must_raise(1_000.0, 800.0, 100.0, 400.0), 500.0);
        // A balance already at or above the buffer raises the gap and no more — raising the gap is
        // what pays it, so the balance is where it was and there is nothing to restock.
        assert_eq!(must_raise(1_000.0, 800.0, 500.0, 400.0), 200.0);
        assert_eq!(must_raise(1_000.0, 800.0, 5_000.0, 400.0), 200.0);
        // And receipts beyond the outlays still leave a thin balance to raise for.
        assert_eq!(must_raise(800.0, 1_000.0, 100.0, 400.0), 100.0);
    }

    #[test]
    fn the_interest_outlay_is_read_from_its_own_bonds_and_reaches_the_holders() {
        // The sum of what its own bonds pay — and it is income to named holders.
        let t = treasury();
        assert_eq!(t.interest(), 130.0);
        let holders = [(party(40), 1_000.0), (party(41), 2_000.0)];
        let reaching = interest_reaches(&t, &holders);
        assert!((reaching[0].1 - 130.0 / 3.0).abs() <= crate::num::dust(2, &[130.0, 3_000.0]));
        assert!(interest_reaches(&t, &[]).is_empty());
    }

    #[test]
    fn a_wall_is_foreseeable_because_it_knows_its_own_maturity_profile() {
        // And the maturity mix is a choice with a trade-off.
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
        assert!(treasury().equity(200.0) < 0.0);
    }

    #[test]
    fn the_debt_outstanding_is_read_from_the_register_and_a_difference_is_a_finding() {
        // Never a plug.
        let t = treasury();
        assert_eq!(t.debt_outstanding(), 3_000.0);
        assert!(debt_reconciles(3_000.0, 3_000.0, 2).is_none());
        assert_eq!(debt_reconciles(3_000.0, 2_800.0, 2), Some(200.0));
    }
}
