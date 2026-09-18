//! THE DERIVATIVE LAYER: the infrastructure every derivative class runs on — one contract on two
//! books, margin that is held rather than consumed, and a waterfall with an END.
//!
//! @spec 16 A1 · 16 A3 · 16 A4 · 16 B1 · 16 B2 · 16 B3 · 16 B3.a · 16 B4 · 16 C1 · 16 C1.a · 16 C2 ·
//! @spec 16 C2.a · 16 C3 · 16 C3.a · 16 C3.b · 16 C4 · 16 C4.a · 16 C4.b · 16 C4.c · 16 C4.d ·
//! @spec 16 C5 · 16 D1 · 16 D2 · 16 D2.a · 16 D2.b · 16 D2.c · 16 D3 · 16 D4 · 16 D4.a · 16 D5 ·
//! @spec 16 E1 · 16 E2 · 16 E3 · 16 E4 · 16 F1 · 16 F2 · 16 F3 · 16 F4 · 16 G1 · 16 G2 · 16 G3 ·
//! @spec 16 G4 · XI-2 · Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! **An offsetting trade with a different counterparty does not remove the first** (B3.a). The party
//! now has TWO contracts and two counterparty exposures: the market risk is flat while the credit risk
//! has doubled, and **collapsing them hides the thing that actually breaks**. `offset` therefore
//! returns a second position and never cancels the first.
//!
//! **No netting across counterparties** (G3, C1.a). Netting is per counterparty PAIR, which is why
//! gross notional and net exposure are orders of magnitude apart — and treating exposure to one as
//! offsetting exposure to another is how a book looks flat until one of them fails. `net_against`
//! takes a pair and there is no door here that takes a party alone.
//!
//! **The house is not a guarantor of last resort** (C5). Its resources are finite and enumerable, and
//! **running past the end of the waterfall is a real event with real consequences, not an
//! impossibility** — `waterfall` returns what was left UNFUNDED.
//!
//! **Posting margin is an asset swap, not an expense** (C3.a): the poster still owns it and gets it
//! back, but it is no longer free (D3). **Initial margin is sized from the risk of the position** —
//! the underlying's own measured move, scaled by notional and remaining life — and **not a stated rate
//! per class** (D1). It **rises when volatility rises, which is exactly when parties can least afford
//! it: procyclical by construction and a consequence to be measured** (D5).
//!
//! **A variation-margin payment has a cash test** (D2.c): a party that cannot pay is in the state
//! settlement describes, and it **does not silently become a borrowing**.

use crate::ids::PartyId;

/// A1, A3, B2: **one contract, recorded on both books** — the same obligation appearing twice, as an
/// asset and a liability, and the two are the same number read from two sides.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position {
    pub a: PartyId,
    pub b: PartyId,
    pub notional: f64,
    /// B1: agreed at a CLEARED price (G4: never against a price this world does not clear).
    pub struck_at: f64,
    /// Positive to `a`, negative to `b`. One number, two reads (A3).
    pub mark: f64,
    pub years_left: f64,
    pub cleared_at_house: Option<PartyId>,
}

impl Position {
    pub fn opened(terms: Position) -> Position {
        assert!(terms.a != terms.b, "16 G1: no position without a counterparty");
        terms
    }

    /// A3: what this contract is worth to one side. The other side's is its negative — the same
    /// number, not a second one.
    pub fn mark_to(&self, who: PartyId) -> Option<f64> {
        if who == self.a {
            return Some(self.mark);
        }
        if who == self.b {
            return Some(-self.mark);
        }
        None
    }
}

/// B3, B3.a: **an offsetting trade with a different counterparty does not remove the first.** The
/// party ends with two contracts and two counterparty exposures — flat market risk, doubled credit
/// risk — and this returns both, because collapsing them hides the thing that actually breaks.
pub fn offset(first: &Position, with: PartyId, at: f64) -> (Position, Position) {
    let second = Position::opened(Position {
        a: first.a,
        b: with,
        notional: first.notional,
        struck_at: at,
        mark: -first.mark,
        years_left: first.years_left,
        cleared_at_house: first.cleared_at_house,
    });
    (*first, second)
}

/// B4: **novation transfers a position to a new counterparty, with the old one's consent** — a real
/// change of who faces whom, not a bookkeeping edit.
pub fn novate(p: &Position, from: PartyId, to: PartyId, consented: bool) -> Option<Position> {
    if !consented {
        return None;
    }
    if from == p.a {
        return Some(Position::opened(Position { a: to, ..*p }));
    }
    if from == p.b {
        return Some(Position::opened(Position { b: to, ..*p }));
    }
    None
}

/// C1, C1.a, G3: **netting is per counterparty PAIR.** Exposure to one does not offset exposure to
/// another, and this takes the pair — there is no door that takes a party alone.
pub fn net_against(who: PartyId, counterparty: PartyId, book: &[Position]) -> f64 {
    book.iter()
        .filter(|p| (p.a == who && p.b == counterparty) || (p.b == who && p.a == counterparty))
        .filter_map(|p| p.mark_to(who))
        .sum()
}

/// C1.a: **gross notional and net exposure are orders of magnitude apart**, and both are reads over
/// the same book — never one inferred from the other.
pub fn gross_notional(who: PartyId, book: &[Position]) -> f64 {
    book.iter()
        .filter(|p| p.a == who || p.b == who)
        .map(|p| p.notional)
        .sum()
}

/// C2: **the house becomes buyer to the seller and seller to the buyer**, so each side faces it and
/// **no member pays another** — every leg is written as TWO, member to house and house to member, and
/// the house is flat on every leg by construction.
pub fn through_the_house(p: &Position, house: PartyId) -> (Position, Position) {
    assert!(p.cleared_at_house == Some(house), "16 C2: a contract cleared nowhere does not face a house");
    (
        Position::opened(Position { b: house, ..*p }),
        Position::opened(Position { a: house, ..*p }),
    )
}

/// C3: **a real entity with a balance sheet** — the margin it holds, a default fund its members paid
/// into, and its own capital, which is the residual: what it has retained beyond margin and fund.
#[derive(Clone, Debug)]
pub struct House {
    pub who: PartyId,
    /// C3.a: each member's margin is **its asset at the house**, not the house's money.
    pub margin_held: Vec<(PartyId, f64)>,
    pub fund: Vec<(PartyId, f64)>,
    pub own_capital: f64,
}

impl House {
    /// C3.b: **the default fund is sized cover-one** — enough to absorb the largest member's book,
    /// given that closing it takes several sessions and **the price move over that horizon scales
    /// with the square root of its length.**
    pub fn cover_one(&self, largest_book: f64, move_per_session: f64, sessions: f64) -> f64 {
        assert!(sessions > 0.0, "16 C3.b: a close-out over no sessions is instantaneous, which it is not");
        largest_book * move_per_session * sessions.sqrt()
    }

    /// C3.b: **contributions are pro rata to each member's margin, trued up every period**, and a
    /// member that leaves is refunded. `None` where nobody has posted anything.
    pub fn contribution_of(&self, member: PartyId, needed: f64) -> Option<f64> {
        let total: f64 = self.margin_held.iter().map(|(_, m)| m).sum();
        if total <= 0.0 {
            return None;
        }
        let mine = self.margin_held.iter().find(|(p, _)| *p == member)?.1;
        Some(needed * mine / total)
    }
}

/// C4.d: **every round of a waterfall is recorded and reportable**: who defaulted, the loss, what each
/// line paid, what was left unfunded.
#[derive(Clone, Debug, PartialEq)]
pub struct Waterfall {
    pub defaulter: PartyId,
    pub loss: f64,
    pub from_defaulters_margin: f64,
    pub from_defaulters_fund: f64,
    pub from_house_capital: f64,
    /// C4.a: **a member's loss can come from another member's default** — the mutualisation channel.
    /// Each survivor books its write-down against its equity: a real loss on a real sheet.
    pub from_survivors: Vec<(PartyId, f64)>,
    /// C5: **running past the end is a real event**, not an impossibility.
    pub unfunded: f64,
}

/// C4: **the stated default waterfall, in order.** Law 6: each line pays what it has and runs out;
/// nothing is clamped and nothing is topped up.
pub fn waterfall(h: &House, defaulter: PartyId, loss: f64) -> Waterfall {
    let margin = h.margin_held.iter().find(|(p, _)| *p == defaulter).map(|(_, m)| *m);
    let fund = h.fund.iter().find(|(p, _)| *p == defaulter).map(|(_, m)| *m);
    // A member with no row posted nothing. That is a read of the register, not a default value.
    let posted_no_margin = 0.0;
    let mut left = loss;

    let from_margin = take(&mut left, match margin { Some(m) => m, None => posted_no_margin });
    let from_fund = take(&mut left, match fund { Some(m) => m, None => posted_no_margin });
    let from_capital = take(&mut left, h.own_capital);

    let survivors: Vec<(PartyId, f64)> = h.fund.iter().filter(|(p, _)| *p != defaulter).copied().collect();
    let theirs: f64 = survivors.iter().map(|(_, c)| c).sum();
    let mut from_survivors = Vec::with_capacity(survivors.len());
    let owed_by_survivors = if left < theirs { left } else { theirs };
    for (who, contributed) in &survivors {
        if theirs > 0.0 {
            from_survivors.push((*who, owed_by_survivors * contributed / theirs));
        }
    }
    left -= owed_by_survivors;

    Waterfall {
        defaulter,
        loss,
        from_defaulters_margin: from_margin,
        from_defaulters_fund: from_fund,
        from_house_capital: from_capital,
        from_survivors,
        unfunded: left,
    }
}

/// One line of the waterfall paying what it has. Arithmetic: it runs out, and what remains passes on.
fn take(left: &mut f64, available: f64) -> f64 {
    let paid = if available < *left { available } else { *left };
    let paid = if paid > 0.0 { paid } else { 0.0 };
    *left -= paid;
    paid
}

/// C4.c: **what the defaulter's own money did not cover is the house's UNSECURED claim on the estate,
/// ranking with other unsecured claims. A close-out is not paid ahead of every ranked claim.**
pub fn claim_on_the_estate(w: &Waterfall) -> f64 {
    w.loss - w.from_defaulters_margin - w.from_defaulters_fund
}

/// D1: **initial margin is sized from the risk of the position** — the underlying's own MEASURED move,
/// scaled by the notional and the remaining life — and never a stated rate per class.
///
/// D5: it **rises when volatility rises, which is exactly when parties can least afford it.** That is
/// procyclical by construction and a consequence to be measured, not damped (Law 6).
pub fn initial_margin(notional: f64, measured_move: f64, years_left: f64) -> f64 {
    assert!(years_left > 0.0, "16 D1: margin on a position with no life left is margin on nothing");
    notional * measured_move * years_left.sqrt()
}

/// D2, D2.a: **variation margin is the change in the mark, paid in cash, every period** — real money
/// leaving one account and arriving in another, and the largest recurring flow this layer produces.
///
/// D2.c: **it has a cash test.** A party that cannot pay is in a recorded failure state; it does not
/// silently become a borrowing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Variation {
    Paid { from: PartyId, to: PartyId, amount: f64 },
    /// D2.c, D4: a margin call that is not met closes the position out (D4.a: and meeting it may force
    /// a sale — the same liquidity channel as a redemption, XI-2).
    CannotPay { who: PartyId, short_by: f64 },
    Nothing,
}

pub fn variation(p: &Position, mark_before: f64, payer_can_find: f64) -> Variation {
    let moved = p.mark - mark_before;
    // Law 7, 21.11: **whether a mark MOVED is a question about a subtraction**, so the answer is the
    // subtraction's own dust — two terms over the two magnitudes it was taken between — and not an
    // exact zero. `==` here called for a variation payment on the last bit of a float, which is a
    // real payment between two named parties for an amount that is an artefact of the arithmetic.
    if moved.abs() <= crate::num::dust(2, &[p.mark, mark_before]) {
        return Variation::Nothing;
    }
    let (from, to, amount) = if moved > 0.0 { (p.b, p.a, moved) } else { (p.a, p.b, -moved) };
    if payer_can_find < amount {
        return Variation::CannotPay { who: from, short_by: amount - payer_can_find };
    }
    Variation::Paid { from, to, amount }
}

/// E1: **a clearing member may carry no more margin at the houses than its own liquid cash could
/// re-margin over the close-out horizon** — a real limit, read once per period from the member's own
/// liquid assets net of what it has already committed.
pub fn admitted(liquid: f64, already_committed: f64, horizon_sessions: f64) -> f64 {
    assert!(horizon_sessions > 0.0, "16 E1: a close-out horizon of no sessions is not a horizon");
    (liquid - already_committed) / horizon_sessions.sqrt()
}

/// E2: **a contract is cut to the smaller of its two members' admitted shares — size, units and margin
/// together — or refused**, and the cut happens at the strike, in the same pass as the contract and
/// its margin leg. `None` is the refusal.
pub fn cut_to(wanted: f64, a_admits: f64, b_admits: f64) -> Option<f64> {
    let smaller = if a_admits < b_admits { a_admits } else { b_admits };
    if smaller <= 0.0 {
        return None;
    }
    Some(if smaller < wanted { smaller } else { wanted })
}

/// E4: **what the markets struck BEYOND what their members could margin is a measurable quantity.**
/// Non-zero means a market sized its demand to the wrong constraint. **Measure; do not raise the
/// limit.**
pub fn struck_beyond_capacity(struck: f64, admitted_total: f64) -> f64 {
    let over = struck - admitted_total;
    if over > 0.0 {
        over
    } else {
        0.0
    }
}

/// F2, F3: **the positions are closed out at a stated value and the in-the-money side has a claim on
/// the estate**; the loss is **the mark minus the collateral held**, and it lands on named survivors.
/// F4: the chain is traceable party by party — a default whose losses vanish is a layer that was never
/// really bilateral.
pub fn close_out(book: &[Position], failed: PartyId, collateral_held: &[(PartyId, f64)]) -> Vec<(PartyId, f64)> {
    let mut landed = Vec::new();
    for p in book.iter().filter(|p| p.a == failed || p.b == failed) {
        let survivor = if p.a == failed { p.b } else { p.a };
        let Some(owed) = p.mark_to(survivor) else { continue };
        if owed <= 0.0 {
            continue;
        }
        // What this survivor actually holds against the failed party: a SUM over its collateral rows.
        // A survivor with no rows has posted nothing, and the sum says so without a default being
        // chosen anywhere (G2: no exposure without margin, or a stated reason there is none).
        let covered: f64 = collateral_held
            .iter()
            .filter(|(who, _)| *who == survivor)
            .map(|(_, c)| c)
            .sum();
        landed.push((survivor, owed - covered));
    }
    landed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn position(a: u32, b: u32, mark: f64) -> Position {
        Position::opened(Position {
            a: party(a),
            b: party(b),
            notional: 1_000_000.0,
            struck_at: 0.03,
            mark,
            years_left: 5.0,
            cleared_at_house: None,
        })
    }

    fn house() -> House {
        House {
            who: party(90),
            margin_held: vec![(party(1), 400.0), (party(2), 600.0), (party(3), 1_000.0)],
            fund: vec![(party(1), 200.0), (party(2), 300.0), (party(3), 500.0)],
            own_capital: 250.0,
        }
    }

    #[test]
    fn one_contract_is_the_same_number_read_from_two_sides() {
        // A3, A4: marks sum to zero across the parties, per contract — because there is one number.
        let p = position(1, 2, 4_000.0);
        assert_eq!(p.mark_to(party(1)), Some(4_000.0));
        assert_eq!(p.mark_to(party(2)), Some(-4_000.0));
        assert!(p.mark_to(party(9)).is_none());
        assert_eq!(p.mark_to(party(1)).unwrap() + p.mark_to(party(2)).unwrap(), 0.0);
    }

    #[test]
    fn an_offsetting_trade_with_a_different_counterparty_leaves_two_exposures() {
        // B3.a: the market risk is flat while the credit risk has DOUBLED, and collapsing them hides
        // the thing that actually breaks.
        let first = position(1, 2, 4_000.0);
        let (kept, second) = offset(&first, party(5), 0.035);
        assert_eq!(kept, first);
        assert_eq!(second.b, party(5));
        // Flat to the market...
        assert_eq!(kept.mark_to(party(1)).unwrap() + second.mark_to(party(1)).unwrap(), 0.0);
        // ...and exposed to two names, which is what G3 refuses to net.
        let book = [kept, second];
        assert_eq!(net_against(party(1), party(2), &book), 4_000.0);
        assert_eq!(net_against(party(1), party(5), &book), -4_000.0);
        assert_eq!(gross_notional(party(1), &book), 2_000_000.0);
    }

    #[test]
    fn exposure_to_one_counterparty_does_not_offset_exposure_to_another() {
        // G3, C1.a: treating it as if it does is how a book looks flat until one of them fails.
        // There is no function here that takes a party alone and returns a net.
        let book = [position(1, 2, 4_000.0), position(1, 5, -4_000.0)];
        assert_eq!(net_against(party(1), party(2), &book), 4_000.0);
        assert_eq!(net_against(party(1), party(5), &book), -4_000.0);
    }

    #[test]
    fn novation_needs_the_old_counterpartys_consent_and_changes_who_faces_whom() {
        // B4.
        let p = position(1, 2, 4_000.0);
        assert!(novate(&p, party(2), party(6), false).is_none());
        let moved = novate(&p, party(2), party(6), true).unwrap();
        assert_eq!(moved.b, party(6));
        assert_eq!(moved.mark, p.mark);
        assert!(novate(&p, party(9), party(6), true).is_none());
    }

    #[test]
    fn clearing_writes_two_legs_and_the_house_is_flat_on_both() {
        // C2: no member pays another; every leg is member-to-house and house-to-member. C2.a: it
        // does not remove the risk, it CONCENTRATES it in a named party.
        let p = Position { cleared_at_house: Some(party(90)), ..position(1, 2, 4_000.0) };
        let (a_leg, b_leg) = through_the_house(&p, party(90));
        assert_eq!(a_leg.b, party(90));
        assert_eq!(b_leg.a, party(90));
        let flat = a_leg.mark_to(party(90)).unwrap() + b_leg.mark_to(party(90)).unwrap();
        assert_eq!(flat, 0.0);
    }

    #[test]
    fn the_waterfall_runs_in_order_and_can_run_past_its_end() {
        // C4, C5: the house is not a guarantor of last resort — its resources are finite and
        // enumerable, and running past the end is a real event with real consequences.
        let small = waterfall(&house(), party(3), 900.0);
        assert_eq!(small.from_defaulters_margin, 900.0);
        assert_eq!(small.unfunded, 0.0);

        let big = waterfall(&house(), party(3), 5_000.0);
        assert_eq!(big.from_defaulters_margin, 1_000.0);
        assert_eq!(big.from_defaulters_fund, 500.0);
        assert_eq!(big.from_house_capital, 250.0);
        // C4.a: the mutualisation channel — survivors pay pro rata to what they contributed.
        let mutualised: f64 = big.from_survivors.iter().map(|(_, x)| x).sum();
        assert_eq!(mutualised, 500.0);
        assert_eq!(big.from_survivors[0], (party(1), 200.0));
        assert_eq!(big.unfunded, 2_750.0);
    }

    #[test]
    fn what_the_defaulters_own_money_did_not_cover_is_an_unsecured_claim_on_the_estate() {
        // C4.c: a close-out is NOT paid ahead of every ranked claim.
        let w = waterfall(&house(), party(3), 5_000.0);
        assert_eq!(claim_on_the_estate(&w), 3_500.0);
    }

    #[test]
    fn the_default_fund_is_sized_cover_one_over_a_horizon_that_takes_sessions() {
        // C3.b: the price move over the horizon scales with the square root of its length, because
        // closing a defaulted book takes several sessions.
        let h = house();
        let one_session = h.cover_one(100_000.0, 0.02, 1.0);
        let five = h.cover_one(100_000.0, 0.02, 5.0);
        assert!(five > one_session);
        assert!(five < one_session * 5.0);
        // And contributions are pro rata to each member's margin.
        assert_eq!(h.contribution_of(party(3), 1_000.0), Some(500.0));
        assert!(h.contribution_of(party(99), 1_000.0).is_none());
    }

    #[test]
    fn initial_margin_is_sized_from_the_risk_and_rises_with_volatility() {
        // D1, D5: not a stated rate per class — and procyclical by construction, which is a
        // consequence to be measured rather than damped.
        let calm = initial_margin(1_000_000.0, 0.01, 5.0);
        let stressed = initial_margin(1_000_000.0, 0.04, 5.0);
        assert!(stressed > calm);
        // And it is smaller on a position with less life left.
        assert!(initial_margin(1_000_000.0, 0.01, 1.0) < calm);
    }

    #[test]
    fn variation_margin_is_cash_and_a_party_that_cannot_pay_is_in_a_recorded_state() {
        // D2, D2.a, D2.c: real money leaving one account and arriving in another — and it does NOT
        // silently become a borrowing.
        let p = position(1, 2, 4_000.0);
        match variation(&p, 1_000.0, 100_000.0) {
            Variation::Paid { from, to, amount } => {
                assert_eq!(from, party(2));
                assert_eq!(to, party(1));
                assert_eq!(amount, 3_000.0);
            }
            other => panic!("expected a payment, got {other:?}"),
        }
        assert_eq!(
            variation(&p, 1_000.0, 500.0),
            Variation::CannotPay { who: party(2), short_by: 2_500.0 }
        );
        assert_eq!(variation(&p, 4_000.0, 100_000.0), Variation::Nothing);
    }

    #[test]
    fn a_member_carries_no_more_than_its_own_liquid_cash_could_re_margin() {
        // E1, E2, E3: a real limit read from the member's own assets net of what it has committed,
        // and a contract is cut to the SMALLER of the two members' admitted shares, or refused.
        let deep = admitted(10_000_000.0, 0.0, 4.0);
        let shallow = admitted(200_000.0, 150_000.0, 4.0);
        assert!(deep > shallow);
        assert_eq!(cut_to(1_000_000.0, deep, shallow), Some(shallow));
        // E3: capacity is drawn down as it is consumed, so the second hedge sees less.
        let after_first = admitted(10_000_000.0, 9_900_000.0, 4.0);
        assert!(after_first < deep);
        // And a member with nothing admitted is refused rather than cut to nothing.
        assert!(cut_to(1_000_000.0, deep, 0.0).is_none());
    }

    #[test]
    fn what_was_struck_beyond_capacity_is_measured_and_the_limit_is_not_raised() {
        // E4: non-zero means a market sized its demand to the wrong constraint. Measure; do not
        // raise the limit.
        assert_eq!(struck_beyond_capacity(1_200_000.0, 1_000_000.0), 200_000.0);
        assert_eq!(struck_beyond_capacity(800_000.0, 1_000_000.0), 0.0);
    }

    #[test]
    fn a_default_lands_on_named_survivors_and_its_losses_do_not_vanish() {
        // F2, F3, F4: the loss is the mark minus the collateral held, traceable party by party. A
        // default whose losses vanish is a layer that was never really bilateral.
        let book = [position(1, 2, 4_000.0), position(3, 2, 9_000.0), position(4, 5, 1_000.0)];
        let held = [(party(1), 1_500.0)];
        let landed = close_out(&book, party(2), &held);
        assert_eq!(landed.len(), 2);
        assert_eq!(landed[0], (party(1), 2_500.0));
        // A survivor that posted no collateral has a row with none — a read, not a default.
        assert_eq!(landed[1], (party(3), 9_000.0));
    }

    #[test]
    #[should_panic(expected = "no position without a counterparty")]
    fn there_is_no_position_without_a_counterparty() {
        Position::opened(Position { a: party(1), b: party(1), ..position(1, 2, 0.0) });
    }

    #[test]
    #[should_panic(expected = "margin on nothing")]
    fn margin_on_a_position_with_no_life_left_is_margin_on_nothing() {
        initial_margin(1_000_000.0, 0.01, 0.0);
    }

    #[test]
    fn a_mark_that_moved_by_arithmetic_dust_calls_no_variation() {
        // Law 7, 21.11: `moved == 0.0` called for a variation payment on the last bit of a float —
        // a real payment between two named parties for an amount that is an artefact of the
        // subtraction. Whether a mark MOVED is a question about a subtraction, so the answer is the
        // subtraction's own dust.
        let big = 1_000_000.0;
        let p = position(1, 2, big);
        assert_eq!(variation(&p, big - f64::EPSILON, 1_000_000.0), Variation::Nothing);
        // And a move that is a move is still paid, in full and in the right direction.
        match variation(&p, big - 500.0, 1_000_000.0) {
            Variation::Paid { amount, .. } => assert_eq!(amount, 500.0),
            other => panic!("a mark that moved 500 pays it: {other:?}"),
        }
    }
}
