//! FX FORWARDS AND CROSS-CURRENCY SWAPS: the forward rate is CLEARED and parity is checked against
//! it, never applied to produce it — and **there is one basis**.
//!
//! @spec 19 A1.a · 19 A1.b · 19 A1.c · 19 A1.d · 19 A2 · 19 A3 · 19 A4 · 19 B1 · 19 B2 · 19 B2.a ·
//! @spec 19 B2.b · 19 B3 · 19 B3.a · 19 B3.b · 19 B4 · 19 C1 · 19 C1.a · 19 C2 · 19 C3 · 19 C4 ·
//! @spec 19 D1 · 19 D2 · 19 D2.a · 19 D3 · 19 D4 · 19 E1 · 19 E2 · 19 E3 · 19 E4 · XI-12 · Law 3,
//! @spec Law 4, Law 5, Law 6, Law 19
//!
//! **No forward rate from a parity formula** (E1). `Forward` carries a rate that cleared, and
//! `parity` is a separate read the cleared rate is CHECKED against — B2.a: covered interest parity is
//! a consequence of an arbitrage somebody takes, never an identity applied to produce the rate. A
//! forward struck as spot moved by a basis, carrying no interest differential at all, is neither
//! cleared nor at parity, and carry is absent from the instrument.
//!
//! **There is one basis** (B3.b). A cleared funding basis and a second basis on a random walk, with
//! the second being the one participants see, trade and book P&L on, is Law 4's defect in the most
//! consequential possible place. `basis` derives it from the cleared forward against parity, and
//! nothing in this module stores a second one.
//!
//! **The arbitrage is not free** (B2.b): it uses balance sheet, capital and credit lines, so a
//! persistent basis is possible and is a finding about those constraints — `closes` answers with what
//! an arbitrageur's own limits let it do, and `None` when they let it do nothing.
//!
//! **The carry is earned over its life, not booked at inception** (A3): a parity-struck forward is
//! worth nothing at strike, and its mark is against the forward for the tenor LEFT.
//!
//! **An FX swap is a secured loan of one currency against another** (A4) — that is what it must be
//! modelled as, and the banks are its largest users: a world without it has no market in which they
//! can fund a foreign book.
//!
//! **No maturity passes without both legs settling in full, in both currencies** (E3, A1.b): both
//! notionals DO move, unlike a rate swap, and `settles` returns both legs or refuses.

use crate::calendar::Day;
use crate::ids::{CurrencyCode, PartyId};

/// A1.d: **a leg states its own money.** Two of them, one per leg, by definition.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Side {
    pub party: PartyId,
    pub ccy: CurrencyCode,
    pub amount: f64,
}

/// A1.b, A1.c: **an exchange of two fixed amounts at maturity** at a forward rate that cleared.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Forward {
    pub pays: Side,
    pub receives: Side,
    /// B1: **cleared from what participants will do.** Not struck off a formula (E1).
    pub rate: f64,
    pub matures: Day,
    pub year_fraction: f64,
}

impl Forward {
    pub fn struck(terms: Forward) -> Forward {
        assert!(
            terms.pays.ccy != terms.receives.ccy,
            "19 A1.d: a forward with one money on both legs is not an FX forward"
        );
        assert!(terms.pays.party != terms.receives.party, "19 E2: a hedge needs a counterparty holding the other side");
        assert!(terms.year_fraction > 0.0, "19 A3: a forward over no time is a spot trade (Law 8)");
        terms
    }
}

/// B2: **where the forward would sit if the arbitrage were free** — spot adjusted for the two
/// currencies' funding costs, because otherwise somebody can borrow one, buy the other, lend it and
/// lock a profit. This is the CHECK, not the price (E1, B2.a).
pub fn parity(spot: f64, base_funding: f64, quote_funding: f64, year_fraction: f64) -> f64 {
    spot * (1.0 + quote_funding * year_fraction) / (1.0 + base_funding * year_fraction)
}

/// B3: **the cross-currency basis is the deviation**, and it is a real price paid by whoever needs the
/// currency more. One basis, derived from the CLEARED forward against parity (Law 19, B3.b).
pub fn basis(f: &Forward, spot: f64, base_funding: f64, quote_funding: f64) -> f64 {
    let at_parity = parity(spot, base_funding, quote_funding, f.year_fraction);
    (f.rate / at_parity - 1.0) / f.year_fraction
}

/// B2.b: **the arbitrage uses balance sheet, capital and credit lines.** What this arbitrageur can
/// actually put on, which is why a persistent basis is possible and is a finding about these
/// constraints rather than about the market.
#[derive(Clone, Copy, Debug)]
pub struct Arbitrageur {
    pub who: PartyId,
    pub balance_sheet_free: f64,
    /// What it must earn on the balance sheet it uses. Its own.
    pub needs: f64,
    /// D3: and a line to the counterparty, because this trade is credit as well as capital.
    pub line_to_counterparty: f64,
}

/// What it does about a basis: the size it can fund, or `None` — the gap stands, and B2.b says that
/// is a finding about its constraints.
pub fn closes(a: &Arbitrageur, basis_now: f64, size_available: f64) -> Option<f64> {
    if basis_now.abs() <= a.needs {
        return None;
    }
    let room = if a.balance_sheet_free < a.line_to_counterparty {
        a.balance_sheet_free
    } else {
        a.line_to_counterparty
    };
    let size = if room < size_available { room } else { size_available };
    if size <= 0.0 {
        return None;
    }
    Some(size)
}

/// A3: **a forward is a funding item long before it is a settlement.** Its mark is against the forward
/// for the tenor LEFT, so a parity-struck forward is worth nothing at strike and **the carry is earned
/// over its life, not booked at inception**.
pub fn mark(f: &Forward, forward_now_for_tenor_left: f64, tenor_left: f64) -> f64 {
    assert!(
        tenor_left <= f.year_fraction,
        "19 A3: a forward cannot have more time left than it was struck for"
    );
    f.receives.amount * (forward_now_for_tenor_left - f.rate) * (tenor_left / f.year_fraction)
}

/// A4: **an FX swap — spot one way, forward back — is a SECURED LOAN of one currency against
/// another**, and that is what it must be modelled as. The near leg lends; the far leg returns it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FxSwap {
    pub near: Forward,
    pub far: Forward,
}

impl FxSwap {
    /// What it costs the borrower of the scarce currency, over the swap's life — the funding rate the
    /// trade actually struck, read from the two legs (Law 19).
    pub fn implied_funding(&self, other_currencys_rate: f64) -> f64 {
        let moved = self.far.rate / self.near.rate - 1.0;
        other_currencys_rate - moved / self.far.year_fraction
    }
}

/// C1, C1.a: **two legs in two currencies, notionals exchanged at start and end, periodic interest on
/// both** — an interest-rate swap with an FX leg attached, inheriting both curves. C3: **the notional
/// exchange at the end is at the ORIGINAL rate**, which is what removes the currency risk and what
/// creates the counterparty risk.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CrossCurrency {
    pub a: Side,
    pub b: Side,
    pub at_rate: f64,
    pub a_pays: f64,
    pub b_pays: f64,
    /// C4: **its price includes the basis**, and that is where a foreign-currency funding shortage
    /// shows up as a number.
    pub basis: f64,
    pub years: f64,
}

impl CrossCurrency {
    /// C3: the end exchange, at the rate struck at the start. Both legs, both currencies (E3).
    pub fn returns_at_maturity(&self) -> (Side, Side) {
        (
            Side { party: self.b.party, ccy: self.a.ccy, amount: self.a.amount },
            Side { party: self.a.party, ccy: self.b.ccy, amount: self.b.amount },
        )
    }

    /// C4: what the party needing the scarce currency pays for it, per period. The basis is IN the
    /// price, not beside it.
    pub fn periodic(&self, periods_per_year: f64) -> f64 {
        self.a.amount * (self.a_pays + self.basis) / periods_per_year
    }
}

/// E3: **no maturity passes without both legs settling in full, in both currencies.** Both notionals
/// move (A1.b), and a settlement that delivers one leg is refused rather than recorded.
pub fn settles(f: &Forward, pays_can_find: f64, receives_can_find: f64) -> Option<(Side, Side)> {
    if pays_can_find < f.pays.amount || receives_can_find < f.receives.amount {
        // A failure to deliver is a real state, and it is NOT a half-settled forward.
        return None;
    }
    Some((f.pays, f.receives))
}

/// D2.a: **the hedge must be ROLLED as the asset persists**, which is a recurring demand and a
/// recurring cost. The cost is what the new forward struck at, against what the old one did — never a
/// formula.
pub fn roll_cost(old: &Forward, new_rate: f64) -> f64 {
    (new_rate - old.rate) * old.receives.amount
}

/// E4: **a hedged foreign asset shows the asset revaluing one way and the forward the other, and the
/// residual is the basis and the imperfection — NOT zero by construction.** This returns that residual
/// so it can be looked at; a hedge that always nets to nothing is a hedge nobody modelled.
pub fn hedge_residual(asset_moved: f64, forward_moved: f64) -> f64 {
    asset_moved + forward_moved
}

/// Why a party is here. Each is a real reason, and D4's dealer quotes a width that is **what carrying
/// the position costs it — the return it needs on the capital the position consumes — not a stated
/// number.**
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// D1: an importer or exporter with a known future foreign payment.
    KnownPayment,
    /// D2: an investor holding a foreign asset that wants the asset and not the currency.
    WantsTheAssetNotTheCurrency,
    /// D3: a bank funding a foreign-currency book — deposits in one money, loans in another.
    FundingAForeignBook,
    Dealer,
}

/// D4: the dealer's width, from what the position consumes and what it needs on that. Never stated.
pub fn width(capital_consumed: f64, needs_on_capital: f64, size: f64) -> Option<f64> {
    if size <= 0.0 {
        return None;
    }
    Some(capital_consumed * needs_on_capital / size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn ccy(n: u32) -> CurrencyCode {
        CurrencyCode::at(n)
    }

    fn forward(rate: f64) -> Forward {
        Forward::struck(Forward {
            pays: Side { party: party(1), ccy: ccy(1), amount: 1_250_000.0 },
            receives: Side { party: party(2), ccy: ccy(2), amount: 1_000_000.0 },
            rate,
            matures: Day(365),
            year_fraction: 1.0,
        })
    }

    #[test]
    fn the_forward_rate_is_cleared_and_parity_is_checked_against_it() {
        // E1, B2.a: covered interest parity is a CONSEQUENCE of an arbitrage somebody takes, never an
        // identity applied to produce the rate. These are two different numbers, and the second does
        // not set the first.
        let at_parity = parity(1.25, 0.01, 0.05, 1.0);
        let struck_wider = forward(at_parity * 1.01);
        assert_ne!(struck_wider.rate, at_parity);
        // And the deviation is THE basis, derived from the cleared print (B3, B3.b).
        assert!(basis(&struck_wider, 1.25, 0.01, 0.05) > 0.0);
        // A forward that cleared exactly at parity has no basis, which is the degenerate case.
        let exact = forward(at_parity);
        assert!(basis(&exact, 1.25, 0.01, 0.05).abs() <= crate::num::dust(4, &[at_parity, 1.25]));
    }

    #[test]
    fn the_forward_carries_the_interest_differential() {
        // B2.a: a forward struck as spot moved by a basis, with NO interest differential at all, is
        // neither cleared nor at parity, and carry is absent from the instrument. The higher-rate
        // money is forward-weaker, and that is the carry.
        let spot = 1.25;
        assert!(parity(spot, 0.01, 0.05, 1.0) > spot);
        assert!(parity(spot, 0.05, 0.01, 1.0) < spot);
    }

    #[test]
    fn a_persistent_basis_is_a_finding_about_the_arbitrageurs_constraints() {
        // B2.b: the arbitrage uses balance sheet, capital and credit lines, so it is not free and a
        // gap can stand. Same basis, different constraints, different answers.
        let big = Arbitrageur { who: party(5), balance_sheet_free: 50_000_000.0, needs: 0.001, line_to_counterparty: 40_000_000.0 };
        let constrained = Arbitrageur { balance_sheet_free: 1_000_000.0, ..big };
        assert_eq!(closes(&big, 0.01, 100_000_000.0), Some(40_000_000.0));
        assert_eq!(closes(&constrained, 0.01, 100_000_000.0), Some(1_000_000.0));
        // And a basis inside what it needs on its own balance sheet is not worth taking at all.
        assert!(closes(&big, 0.0005, 100_000_000.0).is_none());
    }

    #[test]
    fn the_carry_is_earned_over_the_forwards_life_and_not_booked_at_inception() {
        // A3: a parity-struck forward is worth nothing at strike, and the mark is against the
        // forward for the tenor LEFT.
        let f = forward(1.30);
        assert_eq!(mark(&f, 1.30, 1.0), 0.0);
        let half_way = mark(&f, 1.34, 0.5);
        let at_the_start = mark(&f, 1.34, 1.0);
        assert!(half_way.abs() < at_the_start.abs());
    }

    #[test]
    #[should_panic(expected = "more time left than it was struck for")]
    fn a_forward_cannot_have_more_time_left_than_it_was_struck_for() {
        mark(&forward(1.30), 1.34, 2.0);
    }

    #[test]
    fn an_fx_swap_is_a_secured_loan_of_one_currency_against_another() {
        // A4: that is what it must be modelled as, and the implied funding rate is READ from the two
        // legs rather than stated.
        let near = forward(1.25);
        let far = Forward::struck(Forward { rate: 1.28, ..forward(1.28) });
        let s = FxSwap { near, far };
        let implied = s.implied_funding(0.05);
        // Paying away a forward premium means funding cheaper in the other money than its own rate.
        assert!(implied < 0.05);
    }

    #[test]
    fn both_legs_settle_or_neither_does() {
        // E3, A1.b: both notionals DO move, and a settlement that delivers one leg is refused rather
        // than recorded.
        let f = forward(1.25);
        assert!(settles(&f, 2_000_000.0, 2_000_000.0).is_some());
        assert!(settles(&f, 10.0, 2_000_000.0).is_none());
        assert!(settles(&f, 2_000_000.0, 10.0).is_none());
    }

    #[test]
    fn a_cross_currency_swap_returns_the_notionals_at_the_original_rate() {
        // C1, C3: which is what removes the currency risk and what creates the counterparty risk.
        let x = CrossCurrency {
            a: Side { party: party(1), ccy: ccy(1), amount: 1_250_000.0 },
            b: Side { party: party(2), ccy: ccy(2), amount: 1_000_000.0 },
            at_rate: 1.25,
            a_pays: 0.03,
            b_pays: 0.01,
            basis: 0.004,
            years: 5.0,
        };
        let (back_to_a, back_to_b) = x.returns_at_maturity();
        assert_eq!(back_to_a.amount, 1_250_000.0);
        assert_eq!(back_to_b.amount, 1_000_000.0);
        // C4: the price INCLUDES the basis, which is where a funding shortage shows up as a number.
        let with_basis = x.periodic(4.0);
        let without = CrossCurrency { basis: 0.0, ..x }.periodic(4.0);
        assert!(with_basis > without);
    }

    #[test]
    fn a_hedge_must_be_rolled_and_the_roll_costs_what_the_new_forward_struck_at() {
        // D2.a: a recurring demand and a recurring cost, and B3.a says the basis widens exactly when
        // hedgers need it.
        let f = forward(1.25);
        assert!(roll_cost(&f, 1.29) > 0.0);
        assert!(roll_cost(&f, 1.21) < 0.0);
    }

    #[test]
    fn a_hedged_asset_leaves_a_residual_and_it_is_not_zero_by_construction() {
        // E4: the residual is the basis and the imperfection. A hedge that always nets to nothing is
        // a hedge nobody modelled.
        assert_eq!(hedge_residual(-1_000.0, 960.0), -40.0);
        assert_eq!(hedge_residual(-1_000.0, 1_000.0), 0.0);
    }

    #[test]
    fn a_dealers_width_is_what_the_position_costs_it() {
        // D4: the return it needs on the capital the position consumes — not a stated number. A
        // bigger ticket over the same capital is a tighter width, which is why size matters.
        let small = width(50_000.0, 0.12, 1_000_000.0).unwrap();
        let large = width(50_000.0, 0.12, 10_000_000.0).unwrap();
        assert!(small > large);
        assert!(width(50_000.0, 0.12, 0.0).is_none());
    }

    #[test]
    #[should_panic(expected = "needs a counterparty holding the other side")]
    fn there_is_no_hedge_that_removes_a_position_without_somebody_holding_it() {
        // E2.
        Forward::struck(Forward {
            pays: Side { party: party(1), ccy: ccy(1), amount: 1_250_000.0 },
            receives: Side { party: party(1), ccy: ccy(2), amount: 1_000_000.0 },
            rate: 1.25,
            matures: Day(365),
            year_fraction: 1.0,
        });
    }

    #[test]
    #[should_panic(expected = "not an FX forward")]
    fn a_forward_with_one_money_on_both_legs_is_not_an_fx_forward() {
        Forward::struck(Forward {
            pays: Side { party: party(1), ccy: ccy(1), amount: 1_250_000.0 },
            receives: Side { party: party(2), ccy: ccy(1), amount: 1_000_000.0 },
            rate: 1.25,
            matures: Day(365),
            year_fraction: 1.0,
        });
    }
}
