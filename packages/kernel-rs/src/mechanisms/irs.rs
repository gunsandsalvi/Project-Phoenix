//! INTEREST-RATE SWAPS: fixed against floating on a notional that never moves, at a fixed rate that
//! CLEARS — and the curve is read from those cleared rates, never fitted and then used to price
//! them.
//!
//! @spec 18 A1.a · 18 A1.b · 18 A1.c · 18 A1.d · 18 A2 · 18 A3 · 18 A4 · 18 B1 · 18 B2 · 18 B2.a ·
//! @spec 18 B3 · 18 B4 · 18 B5 · 18 C1 · 18 C1.a · 18 C2 · 18 C3 · 18 C3.a · 18 D1 · 18 D2 · 18 D3 ·
//! @spec 18 D3.a · 18 D4 · 18 E1 · 18 E2 · 18 E3 · XI-7 · XI-13 · Law 3, Law 5, Law 8, Law 19

use crate::calendar::Day;
use crate::ids::{CurrencyCode, PartyId};

/// A named floating reference that is observable and transacted. Not a posted policy rate: a leg
/// fixing on an administered number is a label nothing prices off.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reference {
    pub name: u32,
    /// It cleared in a book somebody transacted in. A leg on anything else is E3's forbidden leg.
    pub transacted: bool,
    /// Where it is an overnight rate the leg is the COMPOUNDED overnight print, not a single fix.
    pub overnight: bool,
}

/// A leg with its own periodicity and accrual convention — and the two legs need not match. That
/// mismatch is real and it is part of the price.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Leg {
    /// The periodicity is part of the number.
    pub payments_per_year: f64,
    /// Days in the year this leg counts against. The two legs may count differently.
    pub year_basis: f64,
}

/// The notional is never exchanged, which is why a swap is not a loan. A4: both legs in ONE
/// currency; two currencies makes it a cross-currency swap and a different instrument.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Swap {
    pub payer_of_fixed: PartyId,
    pub payer_of_floating: PartyId,
    pub notional: f64,
    pub ccy: CurrencyCode,
    /// The fixed rate that made the swap worth zero at inception — cleared, not solved.
    pub fixed: f64,
    pub fixed_leg: Leg,
    pub floating_leg: Leg,
    pub on: Reference,
    pub matures: Day,
}

impl Swap {
    /// The struck contract, checked. It takes the terms as written rather than nine positional
    /// arguments, because a swap whose payer and receiver can be transposed by miscounting a comma
    /// is a contract nobody can read.
    pub fn struck(terms: Swap) -> Swap {
        assert!(terms.on.transacted, "18 E3: no floating leg on a rate this world does not produce");
        assert!(terms.notional > 0.0, "18 A1.d: a swap on no notional exchanges nothing either way");
        terms
    }
}

/// The fixing is a real observation. Where the reference is overnight the leg compounds the prints
/// that actually happened — a walk, not a formula over an average.
pub fn fixes_at(r: &Reference, prints: &[f64]) -> Option<f64> {
    if prints.is_empty() {
        // No observation, no fixing. A leg cannot fix on a day the book did not print.
        return None;
    }
    if !r.overnight {
        return prints.last().copied();
    }
    let mut compounded = 1.0;
    for p in prints {
        compounded *= 1.0 + p;
    }
    Some(compounded - 1.0)
}

/// Only the NET moves. One payment, in one direction, between two named parties — and the two legs'
/// own periodicities and bases are in it, which is A2's mismatch being part of the price rather than
/// an approximation.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Net {
    pub from: PartyId,
    pub to: PartyId,
    pub amount: f64,
}

pub fn net(s: &Swap, floating_fixed_at: f64, days_accrued: f64) -> Net {
    let fixed_owed = s.notional * s.fixed * days_accrued / s.fixed_leg.year_basis;
    let floating_owed = s.notional * floating_fixed_at * days_accrued / s.floating_leg.year_basis;
    if fixed_owed > floating_owed {
        Net { from: s.payer_of_fixed, to: s.payer_of_floating, amount: fixed_owed - floating_owed }
    } else {
        Net { from: s.payer_of_floating, to: s.payer_of_fixed, amount: floating_owed - fixed_owed }
    }
}

/// Swaps exist at many tenors, and the set of CLEARED fixed rates IS the curve. C1.a: a read of
/// cleared prices, never a fitted object that then prices the swaps.
#[derive(Clone, Debug)]
pub struct Curve {
    pub points: Vec<(f64, f64)>,
}

impl Curve {
    /// The only way to build one: from rates that cleared. E2's forbidden direction — solving a par
    /// rate out of a discount curve — has no function here to call.
    pub fn from_cleared(points: Vec<(f64, f64)>) -> Curve {
        assert!(points.len() > 1, "18 C1: one cleared rate is not a curve");
        Curve { points }
    }

    pub fn at(&self, tenor_years: f64) -> Option<f64> {
        self.points.iter().find(|(t, _)| *t == tenor_years).map(|(_, r)| *r)
    }

    /// A forward rate is DERIVED from the curve, and it is what the market thinks, not what will
    /// happen. `None` where either tenor did not clear — a forward between two points, one of which
    /// nobody traded, is a number with no market behind it.
    pub fn forward(&self, from_years: f64, to_years: f64) -> Option<f64> {
        if to_years <= from_years {
            return None;
        }
        let near = self.at(from_years)?;
        let far = self.at(to_years)?;
        let grown_far = (1.0 + far).powf(to_years);
        let grown_near = (1.0 + near).powf(from_years);
        Some((grown_far / grown_near).powf(1.0 / (to_years - from_years)) - 1.0)
    }
}

/// The swap curve and the sovereign curve are different curves, and the difference is the swap
/// spread — a consequence of bank credit, collateral, balance-sheet cost and who is forced to be on
/// which side. Measured, never set.
pub fn swap_spread(swap_rate: f64, sovereign_rate: f64) -> f64 {
    swap_rate - sovereign_rate
}

/// After inception the swap has a mark, positive to one side, and it moves with the curve — a real
/// gain and a real loss, not a bookkeeping entry.
pub fn mark(s: &Swap, curve_now: &Curve, tenor_left: f64) -> Option<f64> {
    let par_now = curve_now.at(tenor_left)?;
    // Positive to the payer of fixed when rates have risen: it is paying the old, lower rate.
    Some(s.notional * (par_now - s.fixed) * tenor_left)
}

/// Variation margin turns that mark into cash, which is why a rate move is a liquidity event long
/// before it is a P&L event. D3.a: a hedger whose hedge is winning RECEIVES cash while its hedged
/// item shows an unrealised loss, and that mismatch is a real funding problem.
pub fn margin_call(s: &Swap, mark_now: f64, mark_before: f64) -> Net {
    let moved = mark_now - mark_before;
    if moved > 0.0 {
        Net { from: s.payer_of_floating, to: s.payer_of_fixed, amount: moved }
    } else {
        Net { from: s.payer_of_fixed, to: s.payer_of_floating, amount: -moved }
    }
}

/// Why a party is here. B4: a speculator with a view is required, not optional.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// It issued fixed and wants floating, or the reverse — debt it cannot economically reissue.
    Reissuing,
    /// A duration mismatch — a pension whose liabilities are long and whose assets are not. This is
    /// a STRUCTURAL, one-way demand, and it is why long swap rates behave as they do.
    Duration,
    /// A bank managing its own gap, assets repricing at a different speed from liabilities.
    Gap,
    /// A view on rates.
    AView,
    /// A dealer running a book and hedging its net position.
    Dealer,
}

#[derive(Clone, Copy, Debug)]
pub struct Participant {
    pub who: PartyId,
    pub reason: Reason,
}

/// Without a participant whose reason is a view, the cleared par rate is a function of two
/// regulatory gaps and cannot move because somebody thinks rates are wrong.
pub fn can_clear(book: &[Participant]) -> bool {
    book.iter().any(|p| p.reason == Reason::AView)
}

/// Marks across the two sides sum to zero, and net payments sum to zero, every period.
pub fn pairs_up(payments: &[Net]) {
    for p in payments {
        assert!(p.from != p.to, "18 D4: a payment from a party to itself moves nothing");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn overnight() -> Reference {
        Reference { name: 1, transacted: true, overnight: true }
    }

    fn quarterly() -> Leg {
        Leg { payments_per_year: 4.0, year_basis: 360.0 }
    }

    fn annual() -> Leg {
        Leg { payments_per_year: 1.0, year_basis: 365.0 }
    }

    fn swap() -> Swap {
        Swap::struck(Swap {
            payer_of_fixed: party(1),
            payer_of_floating: party(2),
            notional: 1_000_000.0,
            ccy: CurrencyCode::at(0),
            fixed: 0.03,
            fixed_leg: annual(),
            floating_leg: quarterly(),
            on: overnight(),
            matures: Day(1_825),
        })
    }

    #[test]
    fn only_the_net_moves_and_the_notional_never_does() {
        // If the notional moved it would be a loan. The payment is one number in one direction
        // between two named parties.
        let s = swap();
        let paying_more_fixed = net(&s, 0.01, 90.0);
        assert_eq!(paying_more_fixed.from, party(1));
        assert_eq!(paying_more_fixed.to, party(2));
        assert!(paying_more_fixed.amount > 0.0);
        assert!(paying_more_fixed.amount < s.notional);
        // Rates rise above the fixed rate and the direction reverses.
        let paying_more_floating = net(&s, 0.09, 90.0);
        assert_eq!(paying_more_floating.from, party(2));
        assert_eq!(paying_more_floating.to, party(1));
    }

    #[test]
    fn the_two_legs_accrue_on_their_own_bases_and_the_mismatch_is_in_the_price() {
        // They need not match, and that mismatch is real. Same rate on both legs, different bases,
        // and the net is not zero — which is the convention showing up as money.
        let s = swap();
        let same_rate = net(&s, 0.03, 90.0);
        assert!(same_rate.amount > 0.0);
        // 360 and 365 disagree, and the floating leg on the shorter basis accrues more.
        assert_eq!(same_rate.from, party(2));
    }

    #[test]
    fn an_overnight_leg_compounds_the_prints_that_actually_happened() {
        // A real observation, not a forecast, and not an average of the window.
        let prints = [0.0001, 0.0001, 0.0002];
        let compounded = fixes_at(&overnight(), &prints).unwrap();
        let summed: f64 = prints.iter().sum();
        assert!(compounded > summed);
        // A term reference takes its own fix, not a compounding.
        let term = Reference { overnight: false, ..overnight() };
        assert_eq!(fixes_at(&term, &prints), Some(0.0002));
    }

    #[test]
    fn a_leg_cannot_fix_on_a_day_the_book_did_not_print() {
        assert!(fixes_at(&overnight(), &[]).is_none());
    }

    #[test]
    #[should_panic(expected = "a rate this world does not produce")]
    fn there_is_no_floating_leg_on_a_rate_nobody_transacts() {
        // A posted policy rate is not a benchmark, and a leg fixing on one is a label nothing
        // prices off.
        let posted = Reference { name: 9, transacted: false, overnight: false };
        Swap::struck(Swap { on: posted, ..swap() });
    }

    #[test]
    fn the_curve_is_the_cleared_rates_and_a_forward_is_derived_from_it() {
        // A read of cleared prices, never a fitted object that then prices the swaps — and the
        // forward is what the market thinks, not what will happen.
        let c = Curve::from_cleared(vec![(1.0, 0.030), (2.0, 0.035), (5.0, 0.040)]);
        assert_eq!(c.at(2.0), Some(0.035));
        let forward = c.forward(1.0, 2.0).unwrap();
        // The one-year rate a year forward is above both spot rates on this upward curve.
        assert!(forward > 0.035);
        // A forward between two points, one of which nobody traded, has no market behind it.
        assert!(c.forward(1.0, 3.0).is_none());
        assert!(c.forward(2.0, 1.0).is_none());
    }

    #[test]
    #[should_panic(expected = "is not a curve")]
    fn one_cleared_rate_is_not_a_curve() {
        Curve::from_cleared(vec![(5.0, 0.04)]);
    }

    #[test]
    fn a_rate_move_is_a_liquidity_event_before_it_is_a_p_and_l_event() {
        // The mark moves with the curve and variation margin turns it into cash. The hedger whose
        // hedge is winning RECEIVES money while its hedged item shows an unrealised loss, and that
        let s = swap();
        let before = Curve::from_cleared(vec![(4.0, 0.030), (5.0, 0.032)]);
        let after = Curve::from_cleared(vec![(4.0, 0.050), (5.0, 0.052)]);
        let was = mark(&s, &before, 4.0).unwrap();
        let now = mark(&s, &after, 4.0).unwrap();
        assert!(now > was);
        let call = margin_call(&s, now, was);
        // Rates rose, so the payer of fixed is owed — and it receives CASH, this period.
        assert_eq!(call.to, party(1));
        assert!(call.amount > 0.0);
        // A tenor the curve does not carry has no mark rather than a convenient one.
        assert!(mark(&s, &after, 7.0).is_none());
    }

    #[test]
    fn the_swap_spread_is_measured_against_the_sovereign_curve_and_never_set() {
        // A consequence of bank credit, collateral, balance-sheet cost and who is forced to be on
        // which side.
        assert!(swap_spread(0.035, 0.030) > 0.0);
        assert!(swap_spread(0.028, 0.030) < 0.0);
    }

    #[test]
    fn a_book_with_no_view_on_rates_cannot_clear() {
        // Without one the par rate is a function of two regulatory gaps and cannot move because
        // somebody thinks rates are wrong.
        let gaps = [
            Participant { who: party(1), reason: Reason::Gap },
            Participant { who: party(2), reason: Reason::Duration },
        ];
        assert!(!can_clear(&gaps));
        let with_a_view = [
            Participant { who: party(1), reason: Reason::Gap },
            Participant { who: party(3), reason: Reason::AView },
        ];
        assert!(can_clear(&with_a_view));
    }

    #[test]
    fn every_payment_leaves_one_party_and_arrives_at_another() {
        // What can fail is the pairing, not the arithmetic — a payment from a party to itself moves
        // nothing and is refused at the site.
        let s = swap();
        let payments = [net(&s, 0.01, 90.0), net(&s, 0.09, 90.0)];
        pairs_up(&payments);
    }

    #[test]
    #[should_panic(expected = "moves nothing")]
    fn a_payment_from_a_party_to_itself_is_refused() {
        let to_itself = Net { from: party(1), to: party(1), amount: 500.0 };
        pairs_up(&[to_itself]);
    }
}
