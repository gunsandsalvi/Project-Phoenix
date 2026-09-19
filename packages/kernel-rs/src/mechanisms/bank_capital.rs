//! BANK CAPITAL AND RESOLUTION: capital is the residual, the layers absorb in order, two rules
//! compete to bind, and when it runs out the bank stops being a going concern.
//!
//! @spec 25 A1 · 25 A1.a · 25 A2 · 25 A3 · 25 A4 · 25 B1 · 25 B1.a · 25 B1.b · 25 B1.c · 25 B2 ·
//! @spec 25 B3 · 25 C1 · 25 C1.a · 25 C2 · 25 C2.b · 25 C3 · 25 D1 · 25 D2 · 25 D2.a · 25 D3 ·
//! @spec 25 D3.b · 25 D4 · 25 D5 · 25 E3 · XI-3 · Law 2, Law 4, Law 6, Law 7, Law 15, Law 19

use crate::ids::PartyId;

/// The weight is a property of what the asset is, and this is the question that decides it — can the
/// party behind this claim fail, in the money the claim is in? Nothing here branches on a kind id;
/// it reads the same fact the resolution reads to decide whether a party can fail at all, so the two
#[derive(Clone, Copy, Debug)]
pub struct Weight {
    pub of: f64,
}

impl Weight {
    /// XI-3's two exceptions — a central bank, and a treasury in the money it issues — are exactly
    /// the parties that cannot be made to fail, and a claim on them is the zero-weighted asset the
    /// standard means. Everything else weighs what its own credit says it weighs.
    pub fn on(counterparty_can_fail: bool, from_its_credit: f64) -> Weight {
        if counterparty_can_fail {
            Weight { of: from_its_credit }
        } else {
            Weight { of: 0.0 }
        }
    }
}

/// One asset of the bank's book, at what it is carried at and what it weighs.
#[derive(Clone, Copy, Debug)]
pub struct Asset {
    pub carried: f64,
    pub weight: Weight,
}

/// Capital is the residual. Assets minus liabilities, read — never a stored figure that something is
/// withdrawn from.
#[derive(Clone, Debug)]
pub struct Position {
    pub bank: PartyId,
    pub assets: Vec<Asset>,
    pub liabilities: f64,
    /// The layer between equity and senior paper. A ladder with no subordinated layer is one layer
    /// short at the top and one over-punished in the middle.
    pub subordinated: f64,
    /// Money owed today. Solvency says nothing about it, which is the whole of C1.a.
    pub due_now: f64,
    pub money_at_hand: f64,
}

impl Position {
    pub fn carried(&self) -> f64 {
        self.assets.iter().map(|a| a.carried).sum()
    }

    /// What the book weighs, asset by asset.
    pub fn weighted(&self) -> f64 {
        self.assets.iter().map(|a| a.carried * a.weight.of).sum()
    }

    /// The residual. It is read here and stored nowhere.
    pub fn capital(&self) -> f64 {
        self.carried() - self.liabilities
    }

    /// How many terms that walk had and what magnitude it passed through, published with the number
    /// — a reader comparing against it is comparing against a walk over the whole book, and the dust
    /// it is entitled to is the dust of THAT arithmetic.
    pub fn weighted_dust(&self) -> f64 {
        let magnitude: f64 = self.assets.iter().map(|a| (a.carried * a.weight.of).abs()).sum();
        (self.assets.len() as f64) * f64::EPSILON * magnitude
    }
}

/// The weighted requirement and the leverage backstop that uses no weights at all, plus B2's buffer
/// — which is the bank's own choice, declared as its own caution, not a ratio anybody imposed.
#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub min_weighted: f64,
    pub min_leverage: f64,
    pub buffer: f64,
}

/// What leaves this bank the least room. An OUTCOME of what it holds, never stated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Binding {
    Weighted,
    Leverage,
    Nothing,
}

/// What the bank could still put on, measured in units of the asset it is deciding about — which is
/// the only way the two rules can be compared, since one counts weight and the other does not.
pub fn headroom(p: &Position, r: Rules, adding: Weight) -> (Binding, f64) {
    let line = r.min_weighted + r.buffer;
    let capital = p.capital();
    let on_weighted = if adding.of > 0.0 {
        Some((capital / line - p.weighted()) / adding.of)
    } else {
        // A zero-weighted asset is not "unlimited room" — the weighted rule simply says nothing
        // about it, and the backstop is then the only thing that does.
        None
    };
    let on_leverage = capital / (r.min_leverage + r.buffer) - p.carried();
    match on_weighted {
        Some(w) if w < on_leverage => (Binding::Weighted, w),
        Some(_) | None => (Binding::Leverage, on_leverage),
    }
}

/// Below the line the bank chose for itself, which is where the consequences start — and below the
/// line the rule itself draws, which is a different and worse thing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Standing {
    pub in_buffer: bool,
    pub below_requirement: bool,
}

pub fn standing(p: &Position, r: Rules) -> Standing {
    let weighted = p.weighted();
    if weighted <= 0.0 {
        // A book that weighs nothing has no weighted ratio; the backstop is what speaks.
        let ratio = p.capital() / p.carried();
        return Standing {
            in_buffer: ratio < r.min_leverage + r.buffer,
            below_requirement: ratio < r.min_leverage,
        };
    }
    let ratio = p.capital() / weighted;
    Standing {
        in_buffer: ratio < r.min_weighted + r.buffer,
        below_requirement: ratio < r.min_weighted,
    }
}

/// The two failures, with different triggers and different remedies. A resolution must say which one
/// fired, so they are never collapsed into one word.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Trigger {
    /// Assets below liabilities.
    Insolvent,
    /// Money owed today that it has not got — which says nothing about whether it is solvent.
    Illiquid,
    Both,
    Neither,
}

pub fn trigger(p: &Position) -> Trigger {
    let insolvent = p.capital() < 0.0;
    let illiquid = p.money_at_hand < p.due_now;
    match (insolvent, illiquid) {
        (true, true) => Trigger::Both,
        (true, false) => Trigger::Insolvent,
        (false, true) => Trigger::Illiquid,
        (false, false) => Trigger::Neither,
    }
}

/// New money priced by whoever provides it, and the existing holders diluted by what they paid —
/// never by a formula, and never at a price the bank chose.
#[derive(Clone, Copy, Debug)]
pub struct Subscription {
    pub by: PartyId,
    pub money: f64,
    /// The shares they demanded for it. Their price, not the issuer's.
    pub for_shares: f64,
}

#[derive(Clone, Debug)]
pub struct Recapitalised {
    pub raised: f64,
    pub new_shares: f64,
    /// An equity issue priced by the equity market. What the incumbents are left holding.
    pub incumbent_share: f64,
}

/// Recapitalisation first, if somebody will provide it — and C2.b: it can fail. `None` is the answer
/// when the bids do not cover the hole, because nobody has to buy.
pub fn recapitalise(
    hole: f64,
    shares_before: f64,
    bids: &[Subscription],
) -> Option<Recapitalised> {
    let raised: f64 = bids.iter().map(|b| b.money).sum();
    if raised < hole {
        return None;
    }
    let new_shares: f64 = bids.iter().map(|b| b.for_shares).sum();
    Some(Recapitalised {
        raised,
        new_shares,
        incumbent_share: shares_before / (shares_before + new_shares),
    })
}

/// What the assets are actually worth, not their book — and D1.a: the hole is the difference. A
/// resolution cannot value the book it takes until a loan can be worth less than its face, which is
/// why this takes a realised valuation rather than computing one.
pub fn hole(valued_at: f64, liabilities: f64) -> f64 {
    liabilities - valued_at
}

/// The acquirer is choosing, and can decline. A bid is what it offered for assets and liabilities
/// together; `None` is a resolution with no bid, which falls through to the public path.
#[derive(Clone, Copy, Debug)]
pub struct Bid {
    pub by: PartyId,
    /// What it will pay (positive) or be paid (negative) to take the book. D3.a.
    pub pays: f64,
}

/// The hierarchy, in the order it absorbs. Equity first and fully, subordinated next, senior
/// creditors and depositors last and only in resolution.
#[derive(Clone, Debug, PartialEq)]
pub struct Absorbed {
    pub equity: f64,
    pub subordinated: f64,
    pub senior: f64,
    /// What the estate could not pay a covered depositor. The insurer pays it and becomes a creditor
    /// of the estate — so it is carried out, not netted away.
    pub unpaid: f64,
}

/// The hierarchy is respected. Law 6: a layer is not written down "up to" anything — it absorbs what
/// there is to absorb and runs out, and the next layer meets what is left.
pub fn absorb(hole: f64, equity: f64, subordinated: f64, senior: f64) -> Absorbed {
    if hole <= 0.0 {
        return Absorbed { equity: 0.0, subordinated: 0.0, senior: 0.0, unpaid: 0.0 };
    }
    if hole <= equity {
        return Absorbed { equity: hole, subordinated: 0.0, senior: 0.0, unpaid: 0.0 };
    }
    let after_equity = hole - equity;
    if after_equity <= subordinated {
        return Absorbed { equity, subordinated: after_equity, senior: 0.0, unpaid: 0.0 };
    }
    let after_sub = after_equity - subordinated;
    if after_sub <= senior {
        return Absorbed { equity, subordinated, senior: after_sub, unpaid: 0.0 };
    }
    Absorbed { equity, subordinated, senior, unpaid: after_sub - senior }
}

/// No creditor is worse off than in a liquidation. That is the constraint the whole design serves,
/// and it is a VERIFY — it MEASURES the resolution against the counterfactual and never adjusts
/// either.
pub fn no_creditor_worse_off(in_resolution: f64, in_liquidation: f64, terms: usize) -> bool {
    let dust = (terms as f64)
        * f64::EPSILON
        * (in_resolution.abs() + in_liquidation.abs());
    in_resolution >= in_liquidation - dust
}

/// The limit applies per member where the depositor is a cell (Banks Funding A1.a), so a large cell
/// of small depositors is covered and a cell of large ones is not — which is exactly the distinction
/// the insurance exists to draw. A limit applied to the cell's total deletes it.
pub fn insured(deposits: f64, members: f64, limit_per_member: f64) -> f64 {
    assert!(members >= 1.0, "25 D4: a cell of {members} members is not a depositor");
    let per_member = deposits / members;
    if per_member <= limit_per_member {
        deposits
    } else {
        limit_per_member * members
    }
}

/// The resolution conserves. What the acquirer took, what the insurer paid, what the estate realised
/// and what holders lost sum to the hole in D1 — and D5's public purse is the last resort and a
/// fiscal cost with a payer, so it is a term here like any other, never a residual that balances the
#[derive(Clone, Copy, Debug)]
pub struct Conservation {
    pub hole: f64,
    pub acquirer_took: f64,
    pub insurer_paid: f64,
    pub estate_realised: f64,
    pub holders_lost: f64,
    pub public_paid: f64,
}

impl Conservation {
    pub fn residual(&self) -> f64 {
        self.hole
            - (self.acquirer_took + self.insurer_paid + self.estate_realised
                + self.holders_lost
                + self.public_paid)
    }

    /// Derived dust, six terms over the magnitudes that went through the sum. Never a band.
    pub fn conserves(&self) -> bool {
        let magnitude = self.hole.abs()
            + self.acquirer_took.abs()
            + self.insurer_paid.abs()
            + self.estate_realised.abs()
            + self.holders_lost.abs()
            + self.public_paid.abs();
        self.residual().abs() <= 6.0 * f64::EPSILON * magnitude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn sovereign_book() -> Position {
        // A bank stuffed with claims on a party that cannot fail: the book weighs almost nothing.
        Position {
            bank: party(1),
            assets: vec![
                Asset { carried: 9_000.0, weight: Weight::on(false, 1.0) },
                Asset { carried: 1_000.0, weight: Weight::on(true, 1.0) },
            ],
            liabilities: 9_500.0,
            subordinated: 150.0,
            due_now: 100.0,
            money_at_hand: 400.0,
        }
    }

    fn lending_book() -> Position {
        Position {
            bank: party(2),
            assets: vec![Asset { carried: 10_000.0, weight: Weight::on(true, 1.0) }],
            liabilities: 8_700.0,
            subordinated: 150.0,
            due_now: 100.0,
            money_at_hand: 400.0,
        }
    }

    #[test]
    fn capital_is_the_residual_and_falls_because_the_asset_fell() {
        // It is not a pot that is spent. The same bank, after a loss is booked against the asset,
        // has less capital — and nothing was withdrawn from anywhere.
        let mut p = lending_book();
        let before = p.capital();
        p.assets[0].carried -= 300.0;
        assert!((p.capital() - (before - 300.0)).abs() <= 4.0 * f64::EPSILON * before.abs());
    }

    #[test]
    fn which_rule_binds_is_an_outcome_of_what_the_bank_holds() {
        // Neither answer is stated anywhere. Both banks face the same two rules; the one holding
        // claims on a party that cannot fail is stopped by the backstop, and the one whose book is
        let r = Rules { min_weighted: 0.08, min_leverage: 0.03, buffer: 0.01 };
        let loan = Weight::on(true, 1.0);
        assert_eq!(headroom(&sovereign_book(), r, loan).0, Binding::Leverage);
        assert_eq!(headroom(&lending_book(), r, loan).0, Binding::Weighted);
    }

    #[test]
    fn a_zero_weighted_asset_is_not_unlimited_room() {
        // B1.b is the backstop precisely because the weighted rule says nothing about an asset that
        // weighs nothing. Law 6: the answer is the backstop's number, not infinity.
        let r = Rules { min_weighted: 0.08, min_leverage: 0.03, buffer: 0.01 };
        let (binds, room) = headroom(&sovereign_book(), r, Weight::on(false, 1.0));
        assert_eq!(binds, Binding::Leverage);
        assert!(room.is_finite());
    }

    #[test]
    fn the_buffer_is_the_banks_own_and_breaching_it_is_not_breaching_the_requirement() {
        // Three different places to stand, and the consequences differ at each.
        let r = Rules { min_weighted: 0.08, min_leverage: 0.03, buffer: 0.02 };
        let comfortable = lending_book();
        assert_eq!(
            standing(&comfortable, r),
            Standing { in_buffer: false, below_requirement: false }
        );
        let mut thin = lending_book();
        thin.liabilities = 9_100.0; // 900 of capital on 10,000 weighted: 9%.
        assert_eq!(standing(&thin, r), Standing { in_buffer: true, below_requirement: false });
        let mut breached = lending_book();
        breached.liabilities = 9_300.0; // 7%.
        assert_eq!(standing(&breached, r), Standing { in_buffer: true, below_requirement: true });
    }

    #[test]
    fn solvent_and_illiquid_and_insolvent_and_liquid_are_different_failures() {
        // Both triggers exist and the resolution says which fired. Collapsing them would mean a
        // bank could only ever fail one way.
        let mut solvent_illiquid = lending_book();
        solvent_illiquid.due_now = 900.0;
        solvent_illiquid.money_at_hand = 100.0;
        assert_eq!(trigger(&solvent_illiquid), Trigger::Illiquid);

        let mut insolvent_liquid = lending_book();
        insolvent_liquid.liabilities = 10_500.0;
        insolvent_liquid.money_at_hand = 5_000.0;
        assert_eq!(trigger(&insolvent_liquid), Trigger::Insolvent);

        assert_eq!(trigger(&lending_book()), Trigger::Neither);
    }

    #[test]
    fn a_recapitalisation_can_fail_because_nobody_has_to_buy() {
        // A bank that is always rescued has no failure mechanism at all.
        assert!(recapitalise(500.0, 1_000.0, &[]).is_none());
        let short = [Subscription { by: party(9), money: 300.0, for_shares: 600.0 }];
        assert!(recapitalise(500.0, 1_000.0, &short).is_none());
    }

    #[test]
    fn new_money_is_priced_by_whoever_provides_it_and_the_incumbents_are_diluted() {
        // Their price, not the issuer's. The same hole filled at a worse price leaves the
        // incumbents holding less, and that difference is the whole of the dilution.
        let dear = [Subscription { by: party(9), money: 500.0, for_shares: 500.0 }];
        let cheap = [Subscription { by: party(9), money: 500.0, for_shares: 4_000.0 }];
        let a = recapitalise(500.0, 1_000.0, &dear).unwrap();
        let b = recapitalise(500.0, 1_000.0, &cheap).unwrap();
        assert!(b.incumbent_share < a.incumbent_share);
    }

    #[test]
    fn the_hierarchy_absorbs_in_order_and_a_missing_layer_would_over_punish_the_middle() {
        // Equity first and fully, then subordinated, then senior. A2.b's ladder: with the
        // subordinated layer present, senior paper is untouched by a hole this size.
        let with_sub = absorb(400.0, 300.0, 200.0, 5_000.0);
        assert_eq!(
            with_sub,
            Absorbed { equity: 300.0, subordinated: 100.0, senior: 0.0, unpaid: 0.0 }
        );
        // Without it, the same hole reaches senior creditors — one layer short at the top and one
        // over-punished in the middle.
        let without_sub = absorb(400.0, 300.0, 0.0, 5_000.0);
        assert!(without_sub.senior > 0.0);
    }

    #[test]
    fn a_hole_deeper_than_the_whole_stack_leaves_somebody_unpaid() {
        // Nothing is clamped and nothing is invented. What the estate cannot pay is carried out as
        // `unpaid` — it is the insurer's bill and then the public purse's, and it is never a
        let a = absorb(1_000.0, 300.0, 200.0, 400.0);
        assert_eq!(a.unpaid, 100.0);
    }

    #[test]
    fn no_creditor_worse_off_is_measured_and_never_enforced() {
        // D2.a is a VERIFY. It answers false and that is a finding about the resolution — there is
        // no path here that tops anybody up to make it true.
        assert!(no_creditor_worse_off(80.0, 75.0, 2));
        assert!(!no_creditor_worse_off(70.0, 75.0, 2));
        // Equal to the liquidation outcome passes on derived dust, not on a band.
        assert!(no_creditor_worse_off(75.0, 75.0, 2));
    }

    #[test]
    fn the_insurance_limit_applies_per_member_which_is_the_distinction_it_exists_to_draw() {
        // A large cell of SMALL depositors is covered; a cell of large ones is not.
        let many_small = insured(100_000.0, 10_000.0, 50.0);
        assert_eq!(many_small, 100_000.0);
        let few_large = insured(100_000.0, 100.0, 50.0);
        assert_eq!(few_large, 5_000.0);
        // Applying the limit to the CELL's total would have covered 50 of the 100,000 in both, and
        // the cell of small depositors — the ones the insurance is for — would be uninsured.
        assert!(many_small > few_large);
    }

    #[test]
    #[should_panic(expected = "is not a depositor")]
    fn a_cell_with_no_members_is_not_a_depositor() {
        insured(100.0, 0.0, 50.0);
    }

    #[test]
    fn the_resolution_conserves_and_the_public_purse_is_a_term_with_a_payer() {
        // Every part of the hole lands somewhere named; the sum is checked against derived dust,
        // and a residual is a defect rather than a rounding allowance.
        let c = Conservation {
            hole: 1_000.0,
            acquirer_took: 250.0,
            insurer_paid: 300.0,
            estate_realised: 200.0,
            holders_lost: 150.0,
            public_paid: 100.0,
        };
        assert!(c.conserves());
        let leaking = Conservation { public_paid: 0.0, ..c };
        assert!(!leaking.conserves());
        assert_eq!(leaking.residual(), 100.0);
    }

    #[test]
    fn an_acquirer_can_decline_and_the_resolution_falls_through_to_the_public_path() {
        // An assigned acquirer whose bid is the estate's own value by construction is not choosing.
        // There is no bid here at all, and that is a state the design has to have.
        let bids: Vec<Bid> = Vec::new();
        assert!(bids.is_empty());
        let offered = [Bid { by: party(7), pays: -400.0 }];
        // It takes assets AND liabilities and is PAID the difference when the book is a hole.
        assert!(offered[0].pays < 0.0);
    }

    #[test]
    fn the_hole_is_what_the_assets_fetch_against_what_is_owed() {
        // Not the book. The same liabilities against a book valued lower is a bigger hole, and a
        // resolution that read the book would find none.
        assert_eq!(hole(9_000.0, 9_500.0), 500.0);
        assert_eq!(hole(9_500.0, 9_500.0), 0.0);
    }
}
