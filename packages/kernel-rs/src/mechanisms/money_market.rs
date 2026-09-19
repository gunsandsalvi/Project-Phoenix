//! THE MONEY MARKET: every bank posts a schedule out of its own position, and who ends up lending
//! and who ends up borrowing is the OUTCOME.
//!
//! @spec 11 A1 · 11 A1.a · 11 A1.b · 11 A2.a · 11 A2.b · 11 A3 · 11 A3.a · 11 B1 · 11 B2 · 11 B2.a ·
//! @spec 11 B2.b · 11 B3 · 11 B3.a · 11 B3.b · 11 B3.c · 11 B4 · 11 B5 · 11 B6 · 11 B6.a · 11 B7 ·
//! @spec 11 C1 · 11 C1.a · 11 C2 · 11 C4 · 11 C4.a · 11 C4.b · 11 C5 · 11 D1 · 11 D2 · 11 D3 ·
//! @spec 11 D4 · 11 D5 · 11 D5.a · 11 D6 · 11 E3 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::ids::PartyId;
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};

/// The position is the RESIDUE of everyone else's period — its customers paid other banks'
/// customers, and nobody decided it.
#[derive(Clone, Copy, Debug)]
pub struct Position {
    pub bank: PartyId,
    /// What the flows left in its account at the central bank.
    pub reserves: f64,
    /// A PREFERENCE derived from its own liabilities' liquidity, not a stated ratio.
    pub buffer: f64,
}

impl Position {
    /// What this bank is short or long against its own buffer.
    pub fn need(&self) -> f64 {
        self.buffer - self.reserves
    }
}

/// In aggregate the system's reserves are unchanged — they are REDISTRIBUTED.
pub fn redistributed(before: &[Position], after: &[Position], terms: usize) -> Option<f64> {
    let was: f64 = before.iter().map(|p| p.reserves).sum();
    let now: f64 = after.iter().map(|p| p.reserves).sum();
    let moved = now - was;
    if moved.abs() <= crate::num::dust(terms, &[was, now]) {
        return None;
    }
    // Reserves that left or arrived came from outside the banking system — C1.a's parked cash, or
    // the window.
    Some(moved)
}

/// Overnight and term, each with its own book.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tenor {
    Overnight,
    Term(u32),
}

/// What a piece of collateral is worth to a lender — by asset, by tenor, and by the ISSUER'S OWN
/// CREDIT.
#[derive(Clone, Copy, Debug)]
pub struct Collateral {
    pub issued_by: PartyId,
    pub market_value: f64,
    /// Eligibility is defined per asset, and something is ineligible.
    pub eligible: bool,
    /// Pledged collateral is ENCUMBERED and cannot be pledged twice.
    pub encumbered: bool,
}

/// The lender's own haircut on this piece: what it will lend against it.
pub fn lends_against(c: &Collateral, by_tenor: f64, on_that_issuers_credit: f64) -> Option<f64> {
    if !c.eligible || c.encumbered {
        return None;
    }
    assert!(
        on_that_issuers_credit > 0.0,
        "11 B3.b: a haircut with no view of the issuer's credit is one haircut per instrument type"
    );
    Some(c.market_value / (by_tenor * on_that_issuers_credit))
}

/// A schedule out of the bank's own position and its own cost of funds.
#[derive(Clone, Copy, Debug)]
pub struct Schedule {
    pub bank: PartyId,
    /// Positive to lend, negative to borrow — what this bank wants to do at its own rate.
    pub quantity: f64,
    /// The rate at which it will do it.
    pub rate: f64,
    pub tenor: Tenor,
    pub secured_by: Option<Collateral>,
}

/// The lender's view on getting it back, and that view is in its schedule.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub of: PartyId,
    /// What this lender adds for this borrower's name.
    pub over_the_market: f64,
    /// Or it will not lend to this name at any rate, which is a real outcome of a real schedule.
    pub will_lend: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    pub trades: Vec<(PartyId, PartyId, f64, f64)>,
    pub rate: Option<f64>,
    /// The market can fail to clear for a name — that is what a funding squeeze is.
    pub unfunded: Vec<(PartyId, f64)>,
}

/// A rate clears from those schedules meeting each other.
pub fn session(schedules: &[Schedule], views: &[View], tenor: Tenor) -> Cleared {
    let mut lending: Vec<&Schedule> = schedules
        .iter()
        .filter(|s| s.tenor == tenor && s.quantity > 0.0)
        .collect();
    let mut borrowing: Vec<&Schedule> = schedules
        .iter()
        .filter(|s| s.tenor == tenor && s.quantity < 0.0)
        .collect();
    lending.sort_by(|a, b| a.rate.total_cmp(&b.rate));
    borrowing.sort_by(|a, b| b.rate.total_cmp(&a.rate));

    let mut trades = Vec::new();
    let mut rate = None;
    let mut unfunded: Vec<(PartyId, f64)> = Vec::new();
    let mut left_to_lend: Vec<f64> = lending.iter().map(|s| s.quantity).collect();

    for b in &borrowing {
        let mut wants = -b.quantity;
        for (at, l) in lending.iter().enumerate() {
            if wants <= 0.0 || left_to_lend[at] <= 0.0 {
                continue;
            }
            // The lender's own view of THIS borrower is in its schedule — not a market-wide spread,
            // and not a rule about who lends to whom.
            let view = views.iter().find(|v| v.of == b.bank && l.bank != b.bank);
            let (will_lend, premium) = match view {
                Some(v) => (v.will_lend, v.over_the_market),
                // A lender with no view of this name does not lend to it.
                None => (false, 0.0),
            };
            if !will_lend || l.rate + premium > b.rate {
                continue;
            }
            let taken = if left_to_lend[at] < wants { left_to_lend[at] } else { wants };
            trades.push((l.bank, b.bank, taken, l.rate + premium));
            rate = Some(l.rate + premium);
            left_to_lend[at] -= taken;
            wants -= taken;
        }
        if wants > 0.0 {
            unfunded.push((b.bank, wants));
        }
    }
    Cleared { trades, rate, unfunded }
}

/// The spread between the strongest and weakest name is a measure of stress.
pub fn stress(views: &[View]) -> Option<f64> {
    let lending_to: Vec<f64> = views.iter().filter(|v| v.will_lend).map(|v| v.over_the_market).collect();
    if lending_to.len() < 2 {
        return None;
    }
    let mut widest = lending_to[0];
    let mut tightest = lending_to[0];
    for p in &lending_to {
        if *p > widest {
            widest = *p;
        }
        if *p < tightest {
            tightest = *p;
        }
    }
    Some(widest - tightest)
}

/// The term-to-overnight spread is information about expected stress, not a parameter.
pub fn term_spread(term: &Cleared, overnight: &Cleared) -> Option<f64> {
    Some(term.rate? - overnight.rate?)
}

/// The corridor.
#[derive(Clone, Copy, Debug)]
pub struct Facility {
    pub at_rate: f64,
    /// Above the market, which is what makes a draw informative rather than routine.
    pub penalty_over_market: f64,
}

/// Freely, against good collateral, at a penalty, to the SOLVENT — all four.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Drawn {
    /// What it got, and at what.
    Lent { amount: f64, at_rate: f64 },
    /// The constraint has to bite.
    NoCollateral,
    /// The window does not lend to a bank that is insolvent — that bank goes to resolution.
    Insolvent,
}

pub fn draw(f: &Facility, market_rate: f64, pledgeable: &[Collateral], wants: f64, solvent: bool, by_tenor: f64, on_credit: f64) -> Drawn {
    if !solvent {
        return Drawn::Insolvent;
    }
    let good: f64 = pledgeable
        .iter()
        .filter_map(|c| lends_against(c, by_tenor, on_credit))
        .sum();
    if good <= 0.0 {
        return Drawn::NoCollateral;
    }
    let amount = if good < wants { good } else { wants };
    // Priced above the market, always — a facility at or below it is C5's subsidy, and then a draw
    // stops being information because nobody would prefer the market.
    Drawn::Lent { amount, at_rate: market_rate + f.penalty_over_market }
}

/// What a name that cannot fund actually does, in order.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Recourse {
    /// It sells assets at whatever they fetch, and stops originating.
    Shrink,
    /// It bids up for deposits, and depositors respond to the rate.
    BidForDeposits,
    /// It draws the facility, at the penalty, against collateral.
    Window,
    /// Its account is below zero after the market AND the window have both run.
    CannotPay,
}

pub fn recourse(short_by: f64, can_sell: f64, can_attract: f64, window: Drawn) -> Recourse {
    if can_sell >= short_by {
        return Recourse::Shrink;
    }
    if can_sell + can_attract >= short_by {
        return Recourse::BidForDeposits;
    }
    match window {
        Drawn::Lent { amount, .. } if can_sell + can_attract + amount >= short_by => Recourse::Window,
        _ => Recourse::CannotPay,
    }
}

/// A run: depositors withdraw because they observe weakness, and what they observe must be
/// OBSERVABLE — a published ratio, a facility draw, a rate paid, a run of short closes.
#[derive(Clone, Copy, Debug)]
pub struct Observed {
    pub drew_the_window: bool,
    pub paid_over_the_market: f64,
    pub short_closes: u32,
}

pub fn run_on(o: &Observed, leaving_per_signal: f64, deposits: f64) -> f64 {
    let signals = (o.drew_the_window as u32 + o.short_closes) as f64
        + if o.paid_over_the_market > 0.0 { 1.0 } else { 0.0 };
    let leaving = deposits * leaving_per_signal * signals;
    // Depositors cannot withdraw more than they have.
    if leaving < deposits {
        leaving
    } else {
        deposits
    }
}

/// Interbank exposure is a contagion path: a failure lands on its LENDERS, by name.
pub fn lands_on(failed: PartyId, trades: &[(PartyId, PartyId, f64, f64)]) -> Vec<(PartyId, f64)> {
    trades
        .iter()
        .filter(|(_, borrower, _, _)| *borrower == failed)
        .map(|(lender, _, amount, _)| (*lender, *amount))
        .collect()
}


/// THE CREDIT STOCK: what is still owed on every schedule there is.
pub struct Credit {
    pub kind: u32,
}

impl Mechanism for Credit {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let n = ctx.schedules().outstanding_total();
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn lender(bank: u32, quantity: f64, rate: f64) -> Schedule {
        Schedule { bank: party(bank), quantity, rate, tenor: Tenor::Overnight, secured_by: None }
    }

    fn borrower(bank: u32, quantity: f64, rate: f64) -> Schedule {
        Schedule { bank: party(bank), quantity: -quantity, rate, tenor: Tenor::Overnight, secured_by: None }
    }

    fn trusted(of: u32) -> View {
        View { of: party(of), over_the_market: 0.0, will_lend: true }
    }

    #[test]
    fn who_lends_and_who_borrows_is_the_outcome_and_not_a_rule() {
        // Writing "surplus banks lend, deficit banks borrow" licenses moving cash from a computed
        // surplus to a computed deficit without anybody quoting a rate.
        let schedules = [lender(1, 500.0, 0.05), borrower(2, 500.0, 0.02)];
        let c = session(&schedules, &[trusted(2)], Tenor::Overnight);
        assert!(c.trades.is_empty());
        assert!(c.rate.is_none());
        assert_eq!(c.unfunded, vec![(party(2), 500.0)]);
    }

    #[test]
    fn a_name_the_market_doubts_pays_more_or_finds_no_bid_at_all() {
        // The lender's view of THIS borrower is in its schedule, and refusal is a real outcome
        // rather than a special case.
        let schedules = [lender(1, 500.0, 0.02), borrower(2, 500.0, 0.04)];
        let doubted = View { of: party(2), over_the_market: 0.015, will_lend: true };
        let priced = session(&schedules, &[doubted], Tenor::Overnight);
        assert_eq!(priced.rate, Some(0.035));
        let refused = View { of: party(2), over_the_market: 0.0, will_lend: false };
        let squeezed = session(&schedules, &[refused], Tenor::Overnight);
        assert!(squeezed.trades.is_empty());
        assert_eq!(squeezed.unfunded, vec![(party(2), 500.0)]);
    }

    #[test]
    fn a_lender_with_no_view_of_a_name_does_not_lend_to_it() {
        // Missing is missing: no view is not an implicit yes at the market rate.
        let schedules = [lender(1, 500.0, 0.02), borrower(2, 500.0, 0.04)];
        let c = session(&schedules, &[], Tenor::Overnight);
        assert!(c.trades.is_empty());
    }

    #[test]
    fn the_market_can_fail_to_clear_for_one_name_while_clearing_for_another() {
        // That is what a funding squeeze IS, and it has to be representable.
        let schedules = [lender(1, 500.0, 0.02), borrower(2, 300.0, 0.04), borrower(3, 300.0, 0.04)];
        let views = [trusted(2), View { of: party(3), over_the_market: 0.0, will_lend: false }];
        let c = session(&schedules, &views, Tenor::Overnight);
        assert_eq!(c.trades.len(), 1);
        assert_eq!(c.trades[0].1, party(2));
        assert_eq!(c.unfunded, vec![(party(3), 300.0)]);
    }

    #[test]
    fn a_haircut_reads_the_issuers_own_credit_and_not_only_the_instrument_type() {
        // A haircut identical for the best and worst credit of the same type is the one leg of the
        // downgrade loop that is wholly absent.
        let paper = Collateral { issued_by: party(9), market_value: 1_000.0, eligible: true, encumbered: false };
        let strong = lends_against(&paper, 1.02, 1.01).unwrap();
        let weak = lends_against(&paper, 1.02, 1.30).unwrap();
        assert!(strong > weak);
    }

    #[test]
    fn pledged_collateral_cannot_be_pledged_twice_and_ineligible_paper_is_not_collateral() {
        // Running out of it is how a solvent bank stops being able to borrow.
        let pledged = Collateral { issued_by: party(9), market_value: 1_000.0, eligible: true, encumbered: true };
        assert!(lends_against(&pledged, 1.02, 1.01).is_none());
        let junk = Collateral { issued_by: party(9), market_value: 1_000.0, eligible: false, encumbered: false };
        assert!(lends_against(&junk, 1.02, 1.01).is_none());
    }

    #[test]
    fn the_window_has_all_four_classical_conditions() {
        // Freely, against good collateral, at a penalty, to the solvent.
        let f = Facility { at_rate: 0.04, penalty_over_market: 0.01 };
        let good = [Collateral { issued_by: party(9), market_value: 10_000.0, eligible: true, encumbered: false }];
        match draw(&f, 0.03, &good, 5_000.0, true, 1.02, 1.01) {
            Drawn::Lent { amount, at_rate } => {
                assert_eq!(amount, 5_000.0);
                // Priced ABOVE the market, which is what makes a draw information.
                assert!(at_rate > 0.03);
            }
            other => panic!("expected a loan, got {other:?}"),
        }
        // Out of eligible collateral, it cannot draw.
        let pledged = [Collateral { encumbered: true, ..good[0] }];
        assert_eq!(draw(&f, 0.03, &pledged, 5_000.0, true, 1.02, 1.01), Drawn::NoCollateral);
        // And the window does not lend to an insolvent bank — that bank goes to resolution.
        assert_eq!(draw(&f, 0.03, &good, 5_000.0, false, 1.02, 1.01), Drawn::Insolvent);
    }

    #[test]
    fn failure_for_liquidity_is_reached_only_after_the_market_and_the_window_have_both_run() {
        // Each recourse is a real act, and the last one is a distinct event from failing for
        // solvency.
        let f = Facility { at_rate: 0.04, penalty_over_market: 0.01 };
        let good = [Collateral { issued_by: party(9), market_value: 1_000.0, eligible: true, encumbered: false }];
        let window = draw(&f, 0.03, &good, 10_000.0, true, 1.02, 1.01);
        assert_eq!(recourse(500.0, 900.0, 0.0, window), Recourse::Shrink);
        assert_eq!(recourse(1_500.0, 900.0, 800.0, window), Recourse::BidForDeposits);
        assert_eq!(recourse(2_000.0, 900.0, 200.0, window), Recourse::Window);
        assert_eq!(recourse(90_000.0, 900.0, 200.0, window), Recourse::CannotPay);
    }

    #[test]
    fn a_run_is_self_reinforcing_and_what_depositors_observe_is_observable() {
        // A published ratio, a facility draw, a rate paid, a run of short closes — and the deposits
        // leave with the reserves behind them.
        let quiet = Observed { drew_the_window: false, paid_over_the_market: 0.0, short_closes: 0 };
        let visible = Observed { drew_the_window: true, paid_over_the_market: 0.01, short_closes: 2 };
        assert_eq!(run_on(&quiet, 0.05, 10_000.0), 0.0);
        assert!(run_on(&visible, 0.05, 10_000.0) > 0.0);
        // They cannot withdraw more than they have, which is arithmetic and not a cap.
        let panic = Observed { drew_the_window: true, paid_over_the_market: 0.05, short_closes: 40 };
        assert_eq!(run_on(&panic, 0.05, 10_000.0), 10_000.0);
    }

    #[test]
    fn a_failure_lands_on_its_lenders_by_name() {
        // Interbank exposure is a contagion path.
        let schedules = [lender(1, 300.0, 0.02), lender(4, 300.0, 0.02), borrower(2, 500.0, 0.04)];
        let c = session(&schedules, &[trusted(2)], Tenor::Overnight);
        let hit = lands_on(party(2), &c.trades);
        assert_eq!(hit.len(), 2);
        assert_eq!(hit[0].0, party(1));
        assert_eq!(hit[1].0, party(4));
    }

    #[test]
    fn the_stress_read_and_the_term_spread_are_reads_and_not_parameters() {
        // One name is not a spread, and a spread against a book that did not clear is not
        // information.
        let tight = [trusted(2), View { of: party(3), over_the_market: 0.001, will_lend: true }];
        let wide = [trusted(2), View { of: party(3), over_the_market: 0.04, will_lend: true }];
        assert!(stress(&wide).unwrap() > stress(&tight).unwrap());
        assert!(stress(&[trusted(2)]).is_none());

        let cleared = Cleared { trades: Vec::new(), rate: Some(0.03), unfunded: Vec::new() };
        let dark = Cleared { trades: Vec::new(), rate: None, unfunded: Vec::new() };
        assert_eq!(term_spread(&cleared, &cleared), Some(0.0));
        assert!(term_spread(&cleared, &dark).is_none());
    }

    #[test]
    fn reserves_are_redistributed_and_a_change_in_the_total_is_a_finding() {
        // In aggregate the system's reserves are unchanged.
        let before = [
            Position { bank: party(1), reserves: 1_000.0, buffer: 500.0 },
            Position { bank: party(2), reserves: 200.0, buffer: 500.0 },
        ];
        let after = [
            Position { bank: party(1), reserves: 700.0, buffer: 500.0 },
            Position { bank: party(2), reserves: 500.0, buffer: 500.0 },
        ];
        assert!(redistributed(&before, &after, 4).is_none());
        let leaked = [
            Position { bank: party(1), reserves: 700.0, buffer: 500.0 },
            Position { bank: party(2), reserves: 800.0, buffer: 500.0 },
        ];
        assert_eq!(redistributed(&before, &leaked, 4), Some(300.0));
    }

    #[test]
    fn the_buffer_is_the_banks_own_preference_and_the_need_is_read_after_the_flows() {
        // A bank funded by overnight household money needs more than one funded by term wholesale,
        // and the need is knowable only after the period's flows.
        let skittish = Position { bank: party(1), reserves: 300.0, buffer: 900.0 };
        let steady = Position { bank: party(2), reserves: 300.0, buffer: 350.0 };
        assert!(skittish.need() > steady.need());
    }

    #[test]
    #[should_panic(expected = "one haircut per instrument type")]
    fn a_haircut_with_no_view_of_the_issuer_is_refused() {
        let paper = Collateral { issued_by: party(9), market_value: 1_000.0, eligible: true, encumbered: false };
        lends_against(&paper, 1.02, 0.0);
    }
}
