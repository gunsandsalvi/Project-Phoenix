//! HEDGE FUNDS: leverage is **a fact about a loan**, everything marks at cleared prices, **and the
//! fund can fail.**
//!
//! @spec 28 A1 · 28 A2 · 28 A3 · 28 A4 · 28 A5 · 28 B1 · 28 B1.a · 28 B2 · 28 B3 · 28 B4 · 28 B5 ·
//! @spec 28 C1 · 28 C2 · 28 C3 · 28 C4 · 28 D1 · 28 D2 · 28 D3 · 28 D4 · 28 D4.a · 28 D5 · 28 D5.a ·
//! @spec 28 D6 · 28 D7 · 28 E1 · 28 E2 · 28 E3 · XI-2 · Law 3, Law 5, Law 6, Law 19
//!
//! **No leverage without a lender** (E1, B1.a): leverage is a fact about a LOAN and never a property of
//! the fund, so `Borrowing` names its lender and `leverage` reads the loans.
//!
//! **No position that does not mark** (E2): a fund carrying an unmarked position has hidden its loss.
//! `mark` answers `None` for a position with no cleared price and the caller cannot pretend otherwise.
//!
//! **No fund that cannot fail** (E3): a vehicle that absorbs losses indefinitely is the buyer of last
//! resort under another name. `Failed` is reachable, and its broker eats the shortfall while its
//! investors lose their capital (D6).
//!
//! **The doom loop is emergent, never a contagion coefficient** (D4.a): a loss reduces equity, so with
//! fixed borrowing leverage RISES (D1); the lender calls margin (D2); meeting it requires SELLING at
//! market prices, **which moves prices** (D3); and the move hits other holders of the same positions
//! (D4). `spiral` walks exactly those steps and nothing else, so the chain is traceable (D7).
//!
//! **It will be the buyer when others are forced sellers, IF it has capacity** (C2) — which is what
//! makes it liquidity, and what makes its absence matter when everything is short at once.
//!
//! **Gross exposure, net exposure and equity are three different reads, and all three are needed**
//! (B5): one of them alone hides either the hedging or the size.

use crate::ids::{InstrumentId, PartyId};

/// A1, A2: **a named party whose investor capital is EQUITY** — the investors bear the result, and they
/// hold a share count (XI-15's redeemable claim).
#[derive(Clone, Debug)]
pub struct Fund {
    pub who: PartyId,
    /// A3: **a manager is a separate party**, earning a fee on assets and a share of the gains.
    pub manager: PartyId,
    /// A5: **everything is marked to market at cleared prices**, so its equity moves continuously.
    pub positions: Vec<Position>,
    /// B1: what it borrowed, and from whom.
    pub borrowings: Vec<Borrowing>,
    pub cash: f64,
    pub shares: f64,
}

/// A4, C1, C3: **a mandate that is wide** — long, short, levered, in many markets — and positions taken
/// for REASONS: a relative-value view, a directional view, a liquidity view.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position {
    pub what: InstrumentId,
    /// Negative is short, **which requires a borrow** (C3, §14).
    pub units: f64,
    pub cost: f64,
    /// E2: **no position that does not mark.** `None` where nothing cleared — a fund carrying an
    /// unmarked position has hidden its loss, and this refuses to let it.
    pub price: Option<f64>,
}

impl Position {
    pub fn value(&self) -> Option<f64> {
        Some(self.units * self.price?)
    }
}

/// B1, B1.a, E1: **leverage is a fact about a loan, from a NAMED lender** — never a property of the
/// fund. B2, B3: it also levers through derivatives, where the notional exceeds the margin, and through
/// repo against what it holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Borrowing {
    pub from: PartyId,
    pub amount: f64,
    /// B4: **the amount available is the LENDER'S decision, and it changes.**
    pub available: f64,
    pub kind: Levered,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Levered {
    Margin,
    Derivative,
    Repo,
}

impl Fund {
    /// A5, E2: what it is worth, at cleared prices. `None` where any position has no price — the whole
    /// mark is unavailable rather than partly invented.
    pub fn equity(&self) -> Option<f64> {
        let mut held = self.cash;
        for p in &self.positions {
            held += p.value()?;
        }
        Some(held - self.borrowings.iter().map(|b| b.amount).sum::<f64>())
    }

    /// B5: **gross exposure, net exposure and equity are three different reads, and all three are
    /// needed.** Gross alone hides the hedging; net alone hides the size.
    pub fn gross(&self) -> Option<f64> {
        let mut total = 0.0;
        for p in &self.positions {
            total += p.value()?.abs();
        }
        Some(total)
    }

    pub fn net(&self) -> Option<f64> {
        let mut total = 0.0;
        for p in &self.positions {
            total += p.value()?;
        }
        Some(total)
    }

    /// B1.a: read from the loans, against the equity. `None` where the equity is gone — which is not
    /// zero leverage, it is a fund that has failed.
    pub fn leverage(&self) -> Option<f64> {
        let equity = self.equity()?;
        if equity <= 0.0 {
            return None;
        }
        Some(self.borrowings.iter().map(|b| b.amount).sum::<f64>() / equity)
    }
}

/// D1: **a loss reduces equity, and with fixed borrowing LEVERAGE RISES.** The first step of the loop,
/// and the reason the rest follows.
pub fn after_a_loss(f: &Fund, marked_down_by: f64) -> Option<f64> {
    let equity = f.equity()? - marked_down_by;
    if equity <= 0.0 {
        return None;
    }
    Some(f.borrowings.iter().map(|b| b.amount).sum::<f64>() / equity)
}

/// D2, D3: **the lender calls margin, and meeting it requires selling at market prices — which moves
/// prices.** D4, D4.a: the move hits other holders of the same positions, who may be levered too, and
/// the loop must be **emergent from these steps, never a contagion coefficient**.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub called: f64,
    pub sold: f64,
    /// What the selling did to the price — the market's depth answering the size, not a parameter.
    pub moved_price_by: f64,
    /// D4, D7: who else is now short, by name. The chain is traceable party by party.
    pub reaches: Vec<(PartyId, f64)>,
}

pub fn spiral(f: &Fund, requirement: f64, depth: f64, others: &[(PartyId, f64, f64)]) -> Option<Round> {
    assert!(depth > 0.0, "28 D3: a sale into a market with no depth has no price to move");
    let equity = f.equity()?;
    let called = requirement - equity;
    if called <= 0.0 {
        return None;
    }
    // D3: it sells to meet the call — it cannot sell more than it holds, which is arithmetic.
    let holds = f.gross()?;
    let sold = if holds < called { holds } else { called };
    let moved = sold / depth;
    // D4: and the move hits everybody holding the same positions, levered or not.
    let reaches = others
        .iter()
        .filter_map(|(who, holding, their_equity)| {
            let hit = holding * moved;
            if hit > *their_equity {
                Some((*who, hit - their_equity))
            } else {
                None
            }
        })
        .collect();
    Some(Round { called, sold, moved_price_by: moved, reaches })
}

/// C2: **it will be the buyer when others are forced sellers, IF it has capacity** — which is what makes
/// it liquidity. `None` is the case that matters: everybody short at once, and nobody to buy (XI-2).
pub fn will_buy(f: &Fund, offered: f64, its_view_says_yes: bool) -> Option<f64> {
    if !its_view_says_yes {
        return None;
    }
    let room: f64 = f.borrowings.iter().map(|b| b.available - b.amount).sum::<f64>() + f.cash;
    if room <= 0.0 {
        return None;
    }
    Some(if room < offered { room } else { offered })
}

/// D5, D5.a: **investor redemptions arrive at the same time, for the same reason** — and a gate or
/// notice period **delays** it, which is a real contractual term with real consequences, not a refusal.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Redemption {
    Paid { to: PartyId, amount: f64 },
    /// D5.a: delayed to a stated period. The claim is not extinguished; it is queued.
    Gated { to: PartyId, amount: f64, until_period: u32 },
}

pub fn redeem(f: &Fund, holder: PartyId, shares: f64, gate_until: Option<u32>) -> Option<Redemption> {
    let equity = f.equity()?;
    if f.shares <= 0.0 {
        return None;
    }
    let amount = equity * shares / f.shares;
    Some(match gate_until {
        Some(until_period) => Redemption::Gated { to: holder, amount, until_period },
        None => Redemption::Paid { to: holder, amount },
    })
}

/// D6, E3: **the fund can fail, and then its broker eats the shortfall and its investors lose their
/// capital.** A vehicle that absorbs losses indefinitely is the buyer of last resort under another name.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Failed {
    pub fund: PartyId,
    /// What the broker could not recover — its loss, on its own capital (§15 D2).
    pub broker_eats: f64,
    /// And what the investors lost, which is all of it.
    pub investors_lose: f64,
}

pub fn fails(f: &Fund, collateral_fetched: f64) -> Failed {
    let lent: f64 = f.borrowings.iter().map(|b| b.amount).sum();
    let short = lent - collateral_fetched;
    Failed {
        fund: f.who,
        broker_eats: if short > 0.0 { short } else { 0.0 },
        investors_lose: match f.equity() {
            Some(e) if e > 0.0 => e,
            // Equity already gone, or unmarkable: there was nothing left for them either way.
            _ => 0.0,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn fund() -> Fund {
        Fund {
            who: party(60),
            manager: party(61),
            positions: vec![
                Position { what: InstrumentId::at(1), units: 1_000.0, cost: 900.0, price: Some(1.0) },
                Position { what: InstrumentId::at(2), units: -400.0, cost: -380.0, price: Some(1.0) },
            ],
            borrowings: vec![Borrowing { from: party(80), amount: 500.0, available: 900.0, kind: Levered::Margin }],
            cash: 200.0,
            shares: 100.0,
        }
    }

    #[test]
    fn leverage_is_a_fact_about_a_loan_and_the_lender_is_named() {
        // B1.a, E1: never a property of the fund.
        let f = fund();
        assert_eq!(f.borrowings[0].from, party(80));
        assert_eq!(f.equity(), Some(300.0));
        assert!((f.leverage().unwrap() - 500.0 / 300.0).abs() <= crate::num::dust(2, &[500.0, 300.0]));
    }

    #[test]
    fn a_position_that_does_not_mark_makes_the_whole_fund_unmarkable() {
        // E2: a fund carrying an unmarked position has hidden its loss, and this refuses to let it
        // hide — the equity is unavailable rather than partly invented.
        let mut f = fund();
        f.positions[0].price = None;
        assert!(f.equity().is_none());
        assert!(f.gross().is_none());
        assert!(f.leverage().is_none());
    }

    #[test]
    fn gross_net_and_equity_are_three_different_reads() {
        // B5: gross alone hides the hedging; net alone hides the size.
        let f = fund();
        assert_eq!(f.gross(), Some(1_400.0));
        assert_eq!(f.net(), Some(600.0));
        assert_eq!(f.equity(), Some(300.0));
    }

    #[test]
    fn a_loss_raises_leverage_because_the_borrowing_did_not_move() {
        // D1: the first step of the loop, and the reason the rest follows.
        let f = fund();
        let before = f.leverage().unwrap();
        let after = after_a_loss(&f, 150.0).unwrap();
        assert!(after > before);
        // And a loss past the equity leaves no leverage to read — the fund is gone.
        assert!(after_a_loss(&f, 400.0).is_none());
    }

    #[test]
    fn the_spiral_is_emergent_from_the_call_the_sale_and_the_price_move() {
        // D2, D3, D4, D4.a, D7: never a contagion coefficient — and the chain is traceable party by
        // party.
        let f = fund();
        let others = [(party(62), 5_000.0, 200.0), (party(63), 100.0, 5_000.0)];
        let r = spiral(&f, 900.0, 10_000.0, &others).unwrap();
        assert_eq!(r.called, 600.0);
        assert_eq!(r.sold, 600.0);
        assert!(r.moved_price_by > 0.0);
        // The levered holder of the same position is now short; the unlevered one is not.
        assert_eq!(r.reaches.len(), 1);
        assert_eq!(r.reaches[0].0, party(62));
        // A fund inside its requirement is not called at all.
        assert!(spiral(&f, 100.0, 10_000.0, &others).is_none());
    }

    #[test]
    fn it_buys_when_others_are_forced_sellers_only_if_it_has_capacity() {
        // C2, XI-2: which is what makes it liquidity — and the case that matters is when it has none.
        let f = fund();
        assert_eq!(will_buy(&f, 300.0, true), Some(300.0));
        assert_eq!(will_buy(&f, 5_000.0, true), Some(600.0));
        // Its own view says no: it is a participant, not a mechanism.
        assert!(will_buy(&f, 300.0, false).is_none());
        // Drawn to its line and out of cash, it cannot be the buyer however attractive the price.
        let tapped = Fund {
            cash: 0.0,
            borrowings: vec![Borrowing { from: party(80), amount: 900.0, available: 900.0, kind: Levered::Margin }],
            ..fund()
        };
        assert!(will_buy(&tapped, 300.0, true).is_none());
    }

    #[test]
    fn a_gate_delays_a_redemption_and_does_not_extinguish_it() {
        // D5, D5.a: a real contractual term with real consequences.
        let f = fund();
        assert_eq!(redeem(&f, party(70), 10.0, None), Some(Redemption::Paid { to: party(70), amount: 30.0 }));
        assert_eq!(
            redeem(&f, party(70), 10.0, Some(14)),
            Some(Redemption::Gated { to: party(70), amount: 30.0, until_period: 14 })
        );
    }

    #[test]
    fn the_fund_can_fail_and_the_broker_eats_the_shortfall() {
        // D6, E3: a vehicle that absorbs losses indefinitely is the buyer of last resort under another
        // name.
        let f = fund();
        let clean = fails(&f, 800.0);
        assert_eq!(clean.broker_eats, 0.0);
        let messy = fails(&f, 320.0);
        assert_eq!(messy.broker_eats, 180.0);
        assert_eq!(messy.investors_lose, 300.0);
    }

    #[test]
    #[should_panic(expected = "no price to move")]
    fn a_sale_into_a_market_with_no_depth_has_no_price_to_move() {
        spiral(&fund(), 900.0, 0.0, &[]);
    }
}
