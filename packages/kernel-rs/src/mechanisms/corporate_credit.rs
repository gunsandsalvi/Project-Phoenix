//! CORPORATE CREDIT: a promise brought by a named underwriter into a real book, priced where the
//! book fills — and the issuer can walk away.
//!
//! @spec 7 A2.b · 7 A2.c · 7 A3.a · 7 A3.b · 7 A4 · 7 A4.a · 7 A4.b · 7 B2 · 7 B2.a · 7 B2.b · 7 B3 ·
//! @spec 7 C1 · 7 C2 · 7 C2.a · 7 C2.b · 7 C3 · 7 C4 · 7 C5 · 7 C6 · 7 C7 · 7 C7.a · 7 C7.b · 7 C8 ·
//! @spec 7 C9 · 7 C10 · 7 C10.a · 7 C10.b · 7 C10.c · 7 C11 · 7 C11.a · 7 C11.b · 7 C11.c · 7 C11.d ·
//! @spec 7 C11.e · 7 D1 · 7 D2 · 7 D5 · 7 D7 · 7 D8 · 7 E3 · 7 E4 · 7 E4.a · 7 E5 · 7 E6 · 7 E6.b ·
//! @spec 7 F1 · 7 F2 · 7 F3 · 7 F4 · 7 F5 · 7 F6 · XI-2 · XI-13 · Law 3, Law 5, Law 6, Law 19

use crate::calendar::{Convention, Day};
use crate::ids::CurrencyCode;
use crate::instruments::Class;
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::Owing;
use crate::ids::{InstrumentId, PartyId};
use crate::stores::Commitment;

/// An indication is a SCHEDULE — a size at a level — and not a quantity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Indication {
    pub buyer: PartyId,
    pub size: f64,
    /// The worst level this buyer will take.
    pub at_spread: f64,
}

/// The basis, chosen by the issuer as a decision — two different products with different prices.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Basis {
    /// The bank is an AGENT.
    BestEffort,
    /// The bank UNDERWRITES — it commits to take what the book does not, and is paid for it.
    Backstopped,
}

/// A syndicate of NAMED banks, each taking a stated share of the underwriting risk and of the fee.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Member {
    pub bank: PartyId,
    pub share: f64,
    /// Its own limit, against its own capital.
    pub limit: f64,
}

/// The syndicate can be formed, or the deal is downsized or pulled — never one carried by a member
/// past its limit.
pub fn syndicate(size: f64, members: &[Member]) -> Option<Vec<(PartyId, f64)>> {
    let willing: f64 = members.iter().map(|m| m.limit).sum();
    if willing < size {
        return None;
    }
    let mut taken = Vec::with_capacity(members.len());
    for m in members {
        let theirs = size * m.share;
        if theirs > m.limit {
            // A member cannot be assigned what it did not agree to carry.
            return None;
        }
        taken.push((m.bank, theirs));
    }
    Some(taken)
}

/// It prices — one level struck at which the book fills — and the allocation is decided out of the
/// book.
#[derive(Clone, Debug, PartialEq)]
pub enum Brought {
    Priced {
        at_spread: f64,
        allocated: Vec<(PartyId, f64)>,
        /// Proceeds reach the issuer net of a fee that reaches the underwriter.
        to_issuer: f64,
        fee_to_underwriter: f64,
        /// What the book did not take.
        left_with_underwriter: f64,
        not_issued: f64,
    },
    /// A pulled deal never traded and never existed.
    Pulled,
}

/// The primary market, in one pass.
pub fn bring(
    size: f64,
    book: &[Indication],
    basis: Basis,
    fee_rate: f64,
    issuer_will_pay_up_to: f64,
    underwriter: PartyId,
) -> Brought {
    let mut ordered: Vec<&Indication> = book.iter().collect();
    ordered.sort_by(|a, b| a.at_spread.total_cmp(&b.at_spread));

    let mut filled = 0.0;
    let mut at_spread = 0.0;
    let mut allocated = Vec::new();
    for i in &ordered {
        if filled >= size {
            break;
        }
        let room = size - filled;
        let taken = if i.size < room { i.size } else { room };
        allocated.push((i.buyer, taken));
        at_spread = i.at_spread;
        filled += taken;
    }

    // The issuer's walk-away.
    if filled <= 0.0 || at_spread > issuer_will_pay_up_to {
        return Brought::Pulled;
    }

    let short = size - filled;
    let (left_with_underwriter, not_issued) = match basis {
        // The underwriter committed, so the remainder is its position.
        Basis::Backstopped => (short, 0.0),
        // The agent commits nothing, so the remainder is simply not issued.
        Basis::BestEffort => (0.0, short),
    };
    let issued = filled + left_with_underwriter;
    let fee = issued * fee_rate;
    assert!(underwriter.some(), "7 C1: a deal is brought by a NAMED underwriter");
    Brought::Priced {
        at_spread,
        allocated,
        to_issuer: issued - fee,
        fee_to_underwriter: fee,
        left_with_underwriter,
        not_issued,
    }
}

/// The backstop fee exceeds the best-effort fee for the same issuer and size, as a CONSEQUENCE of
/// the risk behind it (C7.b: a fee with no risk behind it is a transfer).
pub fn fee_gap(backstopped: f64, best_effort: f64) -> f64 {
    backstopped - best_effort
}

/// A tap does not create a second instrument.
pub fn tap(existing: Option<InstrumentId>, added_face: f64) -> Option<(InstrumentId, f64)> {
    let what = existing?;
    assert!(added_face > 0.0, "7 C8: a tap of no face is not a tap");
    Some((what, added_face))
}

/// A committed facility is one line per lender per borrower.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Draw {
    /// Against the existing line, at its own margin.
    OnExistingLine { at_margin: f64, amount: f64 },
    /// No line is live, so one opens at the margin the lender quotes NOW.
    OpensNewLine { at_margin: f64, amount: f64 },
    /// The line is live but has no room, and a draw beyond it is not a draw.
    NoRoom,
}

pub fn draw(live: Option<&Commitment>, wants: f64, quoted_now: f64) -> Draw {
    match live {
        Some(f) => {
            let room = f.undrawn();
            if room <= 0.0 {
                return Draw::NoRoom;
            }
            let amount = if room < wants { room } else { wants };
            Draw::OnExistingLine { at_margin: f.margin, amount }
        }
        None => Draw::OpensNewLine { at_margin: quoted_now, amount: wants },
    }
}

/// Default is a missed payment OR a breached covenant, and a covenant breach is an OBSERVABLE event.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Covenant {
    /// What the issuer promised about its own conduct: leverage, coverage, restricted payments.
    pub leverage_at_most: f64,
    pub coverage_at_least: f64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standing {
    Performing,
    /// Observable, and other participants react to it.
    Breached,
    /// A missed payment.
    Defaulted,
}

pub fn standing(c: &Covenant, leverage: f64, coverage: f64, paid_when_due: bool) -> Standing {
    if !paid_when_due {
        return Standing::Defaulted;
    }
    if leverage > c.leverage_at_most || coverage < c.coverage_at_least {
        return Standing::Breached;
    }
    Standing::Performing
}

/// A breach can be waived or cured, at a price, and that negotiation is real.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Waiver {
    pub fee: f64,
    pub margin_rises_by: f64,
}

pub fn waive(breached: Standing, lender_will: bool, terms: Waiver) -> Option<Waiver> {
    if breached != Standing::Breached || !lender_will {
        return None;
    }
    Some(terms)
}

/// The service is interest PLUS scheduled principal, both real payments — and coverage is a read,
/// and it can fall below one.
pub fn coverage(operating_cash: f64, interest: f64, scheduled_principal: f64) -> Option<f64> {
    let service = interest + scheduled_principal;
    if service <= 0.0 {
        return None;
    }
    Some(operating_cash / service)
}

/// Holders who want out and buyers who want in post schedules, and who trades is the outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct Traded {
    pub trades: Vec<(PartyId, PartyId, f64, f64)>,
    pub price: Option<f64>,
    /// A seller that finds no buyer keeps its paper.
    pub kept: Vec<(PartyId, f64)>,
}

pub fn secondary(sellers: &[(PartyId, f64, f64)], buyers: &[(PartyId, f64, f64)]) -> Traded {
    let mut asks: Vec<&(PartyId, f64, f64)> = sellers.iter().collect();
    let mut bids: Vec<&(PartyId, f64, f64)> = buyers.iter().collect();
    asks.sort_by(|a, b| a.2.total_cmp(&b.2));
    bids.sort_by(|a, b| b.2.total_cmp(&a.2));

    let mut trades = Vec::new();
    let mut price = None;
    let mut left: Vec<f64> = asks.iter().map(|a| a.1).collect();
    for b in &bids {
        let mut wants = b.1;
        for (at, a) in asks.iter().enumerate() {
            if wants <= 0.0 || left[at] <= 0.0 || a.2 > b.2 {
                continue;
            }
            let taken = if left[at] < wants { left[at] } else { wants };
            trades.push((a.0, b.0, taken, a.2));
            price = Some(a.2);
            left[at] -= taken;
            wants -= taken;
        }
    }
    let kept = asks
        .iter()
        .enumerate()
        .filter(|(at, _)| left[*at] > 0.0)
        .map(|(at, a)| (a.0, left[at]))
        .collect();
    Traded { trades, price, kept }
}

/// No derived measure may set the price.
pub fn spread_from(price: f64, risk_free_price: f64, years: f64) -> Option<f64> {
    if price <= 0.0 || risk_free_price <= 0.0 || years <= 0.0 {
        return None;
    }
    Some((risk_free_price / price).powf(1.0 / years) - 1.0)
}

/// Interest accrues to the holder of record, continuously, and accrued interest transfers with the
/// paper.
pub fn accrued(coupon: f64, face: f64, days_since_payment: i64, days_in_period: i64) -> f64 {
    assert!(days_in_period > 0, "7 F1: a coupon period of no days accrues nothing to anybody");
    face * coupon * days_since_payment as f64 / days_in_period as f64
}

/// The holder marks at the cleared price, its value is units times price, and the change in the mark
/// is P&L that reaches its income — realised on sale, unrealised while held, and the two are
/// distinguishable.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Marked {
    pub value: f64,
    pub unrealised: f64,
    pub realised: f64,
}

pub fn mark(units: f64, price_now: f64, cost: f64, units_sold: f64, sold_at: f64) -> Marked {
    let held = units - units_sold;
    Marked {
        value: held * price_now,
        unrealised: held * (price_now - cost),
        realised: units_sold * (sold_at - cost),
    }
}

/// A leveraged holder funds the position, and that funding can be WITHDRAWN — which forces a sale.
pub fn funding_withdrawn(position: f64, funded_by: f64, still_lent: f64) -> Option<f64> {
    if still_lent >= funded_by {
        return None;
    }
    let short = funded_by - still_lent;
    Some(if short < position { short } else { position })
}

/// Refinancing — a new issue whose proceeds retire an old one, at the market's price ON THE DAY,
/// which is how a rate rise reaches a firm that borrowed years ago.
pub fn refinanced_at(old_coupon: f64, new_market_spread: f64, risk_free_now: f64) -> f64 {
    let now = new_market_spread + risk_free_now;
    now - old_coupon
}


/// WHAT THIS BORROWER MUST RAISE. A PLACEHOLDER, and two things mark it as one. It reads the
/// borrower's receipts as nothing, which is a stated value for an outcome — what its customers
/// actually paid it. And the rule itself is the sovereign's, borrowed because corporate credit has no
/// funding decision of its own. Both die at 0r, which builds one.
fn must_raise(owes: f64, cash: f64, buffer: f64) -> f64 {
    let restock = buffer - (cash - owes);
    match restock > 0.0 {
        true => owes + restock,
        false => owes,
    }
}
/// A BORROWER SHORT OVER THE YEAR BRINGS A BOND.

pub struct Brings {
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

impl Mechanism for Brings {
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
            let short = must_raise(owes, cash, buffer);
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
    use crate::calendar::Day;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn book() -> Vec<Indication> {
        vec![
            Indication { buyer: party(20), size: 300.0, at_spread: 0.010 },
            Indication { buyer: party(21), size: 400.0, at_spread: 0.015 },
            Indication { buyer: party(22), size: 500.0, at_spread: 0.025 },
        ]
    }

    #[test]
    fn the_book_decides_where_the_deal_prices() {
        // The book's size and shape decide the level, and one level is struck at which it fills.
        let small = bring(500.0, &book(), Basis::Backstopped, 0.01, 0.05, party(80));
        let large = bring(1_100.0, &book(), Basis::Backstopped, 0.01, 0.05, party(80));
        match (small, large) {
            (Brought::Priced { at_spread: tight, .. }, Brought::Priced { at_spread: wide, .. }) => {
                assert_eq!(tight, 0.015);
                assert_eq!(wide, 0.025);
            }
            other => panic!("expected two priced deals, got {other:?}"),
        }
    }

    #[test]
    fn a_pulled_deal_never_traded_and_never_existed() {
        // The issuer's walk-away.
        assert_eq!(bring(1_100.0, &book(), Basis::Backstopped, 0.01, 0.012, party(80)), Brought::Pulled);
        // And a deal nobody indicated for is pulled too.
        assert_eq!(bring(500.0, &[], Basis::Backstopped, 0.01, 0.05, party(80)), Brought::Pulled);
    }

    #[test]
    fn best_effort_leaves_the_agent_holding_nothing_and_backstopped_leaves_it_holding_the_rest() {
        // No best-effort deal that leaves the agent holding paper, and no backstopped deal whose
        // underwriter does not.
        match bring(1_500.0, &book(), Basis::BestEffort, 0.005, 0.05, party(80)) {
            Brought::Priced { left_with_underwriter, not_issued, .. } => {
                assert_eq!(left_with_underwriter, 0.0);
                assert_eq!(not_issued, 300.0);
            }
            other => panic!("expected a priced deal, got {other:?}"),
        }
        match bring(1_500.0, &book(), Basis::Backstopped, 0.01, 0.05, party(80)) {
            Brought::Priced { left_with_underwriter, not_issued, .. } => {
                assert_eq!(left_with_underwriter, 300.0);
                assert_eq!(not_issued, 0.0);
            }
            other => panic!("expected a priced deal, got {other:?}"),
        }
    }

    #[test]
    fn the_proceeds_reach_the_issuer_net_of_a_fee_that_reaches_the_underwriter() {
        // A fee with no risk behind it is a transfer, so the backstop fee exceeds the best-effort
        // one — measured, never enforced.
        match bring(1_000.0, &book(), Basis::Backstopped, 0.01, 0.05, party(80)) {
            Brought::Priced { to_issuer, fee_to_underwriter, .. } => {
                assert_eq!(fee_to_underwriter, 10.0);
                assert_eq!(to_issuer, 990.0);
            }
            other => panic!("expected a priced deal, got {other:?}"),
        }
        assert!(fee_gap(0.010, 0.005) > 0.0);
    }

    #[test]
    fn a_deal_larger_than_the_willing_members_limits_fails_to_find_a_syndicate() {
        // An observable event with a named issuer, NOT a deal that silently shrinks to fit.
        let members = [
            Member { bank: party(80), share: 0.5, limit: 600.0 },
            Member { bank: party(81), share: 0.3, limit: 400.0 },
            Member { bank: party(82), share: 0.2, limit: 300.0 },
        ];
        let filled = syndicate(1_000.0, &members).unwrap();
        assert_eq!(filled[0], (party(80), 500.0));
        assert!(syndicate(5_000.0, &members).is_none());
    }

    #[test]
    fn no_syndicate_share_is_above_a_members_own_limit() {
        // The lead cannot lend a member capacity it does not have.
        let members = [
            Member { bank: party(80), share: 0.9, limit: 600.0 },
            Member { bank: party(81), share: 0.1, limit: 4_000.0 },
        ];
        assert!(syndicate(1_000.0, &members).is_none());
    }

    #[test]
    fn a_tap_does_not_create_a_second_instrument_and_a_debut_has_nothing_to_tap() {
        let existing = InstrumentId::at(5);
        assert_eq!(tap(Some(existing), 200.0), Some((existing, 200.0)));
        assert!(tap(None, 200.0).is_none());
    }

    #[test]
    fn a_draw_taps_the_line_at_the_margin_it_was_struck_at() {
        // A draw does not mint a facility per period, and a line with no room is not a draw.
        let line = Commitment {
            lender: party(70),
            borrower: party(9),
            limit: 1_000.0,
            drawn: 400.0,
            margin: 0.02,
            fee_on_undrawn: 0.005,
            until: Some(Day(900)),
        };
        assert_eq!(draw(Some(&line), 300.0, 0.09), Draw::OnExistingLine { at_margin: 0.02, amount: 300.0 });
        assert_eq!(draw(Some(&line), 900.0, 0.09), Draw::OnExistingLine { at_margin: 0.02, amount: 600.0 });
        let full = Commitment { drawn: 1_000.0, ..line };
        assert_eq!(draw(Some(&full), 100.0, 0.09), Draw::NoRoom);
        // And with no line live, one opens at what the lender quotes NOW.
        assert_eq!(draw(None, 300.0, 0.09), Draw::OpensNewLine { at_margin: 0.09, amount: 300.0 });
    }

    #[test]
    fn a_covenant_breach_is_observable_before_a_default_and_can_be_waived_at_a_price() {
        // Without covenants the only credit dynamic is binary, and an assessment has nothing to
        // update on between "paying" and "gone".
        let c = Covenant { leverage_at_most: 4.0, coverage_at_least: 2.0 };
        assert_eq!(standing(&c, 3.0, 3.0, true), Standing::Performing);
        assert_eq!(standing(&c, 5.0, 3.0, true), Standing::Breached);
        assert_eq!(standing(&c, 1.0, 9.0, false), Standing::Defaulted);
        let terms = Waiver { fee: 25.0, margin_rises_by: 0.01 };
        assert_eq!(waive(Standing::Breached, true, terms), Some(terms));
        // A lender that will not waive is a real outcome.
        assert!(waive(Standing::Breached, false, terms).is_none());
        assert!(waive(Standing::Performing, true, terms).is_none());
    }

    #[test]
    fn coverage_is_a_read_and_it_can_fall_below_one() {
        // The service is interest PLUS scheduled principal.
        assert_eq!(coverage(300.0, 100.0, 50.0), Some(2.0));
        assert!(coverage(100.0, 100.0, 50.0).unwrap() < 1.0);
        assert!(coverage(300.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn a_seller_that_finds_no_buyer_keeps_its_paper() {
        // Illiquidity is an unsold position, and there is no invisible bid.
        let sellers = [(party(30), 500.0, 0.03), (party(31), 500.0, 0.09)];
        let buyers = [(party(40), 400.0, 0.04)];
        let t = secondary(&sellers, &buyers);
        assert_eq!(t.trades.len(), 1);
        assert_eq!(t.price, Some(0.03));
        assert_eq!(t.kept, vec![(party(30), 100.0), (party(31), 500.0)]);
    }

    #[test]
    fn the_spread_is_read_from_the_price_and_never_into_it() {
        // No derived measure may set the price.
        let wide = spread_from(90.0, 100.0, 5.0).unwrap();
        let tight = spread_from(98.0, 100.0, 5.0).unwrap();
        assert!(wide > tight);
        assert!(spread_from(0.0, 100.0, 5.0).is_none());
        assert!(spread_from(90.0, 100.0, 0.0).is_none());
    }

    #[test]
    fn accrued_interest_transfers_with_the_paper() {
        // It accrues to the holder of record, and the buyer pays it to the seller.
        assert_eq!(accrued(0.05, 1_000.0, 90, 360), 12.5);
        assert_eq!(accrued(0.05, 1_000.0, 0, 360), 0.0);
    }

    #[test]
    fn realised_and_unrealised_p_and_l_are_distinguishable() {
        let m = mark(1_000.0, 1.05, 1.0, 200.0, 1.10);
        assert_eq!(m.value, 840.0);
        assert!((m.unrealised - 40.0).abs() <= crate::num::dust(3, &[840.0, 800.0]));
        assert!((m.realised - 20.0).abs() <= crate::num::dust(3, &[220.0, 200.0]));
    }

    #[test]
    fn withdrawn_funding_forces_a_sale() {
        // The link from the money market to this one.
        assert!(funding_withdrawn(1_000.0, 800.0, 800.0).is_none());
        assert_eq!(funding_withdrawn(1_000.0, 800.0, 500.0), Some(300.0));
        // It cannot force the sale of more than is held — arithmetic, not a clamp.
        assert_eq!(funding_withdrawn(200.0, 800.0, 0.0), Some(200.0));
    }

    #[test]
    fn refinancing_is_how_a_rate_rise_reaches_a_firm_that_borrowed_years_ago() {
        // At the market's price ON THE DAY.
        assert!(refinanced_at(0.03, 0.02, 0.05) > 0.0);
        assert!(refinanced_at(0.08, 0.02, 0.03) < 0.0);
    }

    #[test]
    #[should_panic(expected = "is not a tap")]
    fn a_tap_of_no_face_is_not_a_tap() {
        tap(Some(InstrumentId::at(5)), 0.0);
    }
}
