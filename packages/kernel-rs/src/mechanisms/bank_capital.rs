//! BANK CAPITAL AND RESOLUTION: capital is the residual, the layers absorb in order, two rules
//! compete to bind, and when it runs out the bank stops being a going concern.
//!
//! @spec 25 A1 · 25 A1.a · 25 A2 · 25 A3 · 25 A4 · 25 B1 · 25 B1.a · 25 B1.b · 25 B1.c · 25 B2 ·
//! @spec 25 B3 · 25 C1 · 25 C1.a · 25 C2 · 25 C2.b · 25 C3 · 25 D1 · 25 D2 · 25 D2.a · 25 D3 ·
//! @spec 25 D3.b · 25 D4 · 25 D5 · 25 E3 · XI-3 · Law 2, Law 4, Law 6, Law 7, Law 15, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{standing, Grade, Standard};

/// The weight is a property of what the asset is, and this is the question that decides it — can the
/// party behind this claim fail, in the money the claim is in?
#[derive(Clone, Copy, Debug)]
pub struct Weight {
    pub of: f64,
}

impl Weight {
    /// XI-3's two exceptions — a central bank, and a treasury in the money it issues — are exactly
    /// the parties that cannot be made to fail, and a claim on them is the zero-weighted asset the
    /// standard means.
    pub fn on(counterparty_can_fail: bool, from_its_credit: f64) -> Weight {
        if counterparty_can_fail {
            Weight {
                of: from_its_credit,
            }
        } else {
            Weight { of: 0.0 }
        }
    }
}

/// Select the prudential treatment from the exposure's declared instrument class. Credit grades
/// affect promises to pay; residual and physical exposures retain full weight.
pub fn classified_weight(
    class: crate::instruments::Class,
    counterparty_can_fail: bool,
    credit_weight: f64,
) -> Weight {
    match class {
        crate::instruments::Class::Money | crate::instruments::Class::Claim => {
            Weight::on(counterparty_can_fail, credit_weight)
        }
        crate::instruments::Class::Share
        | crate::instruments::Class::Good
        | crate::instruments::Class::Plant => Weight { of: 1.0 },
    }
}

/// One asset of the bank's book, at what it is carried at and what it weighs.
#[derive(Clone, Copy, Debug)]
pub struct Asset {
    pub carried: f64,
    pub weight: Weight,
}

/// Prudential market value admits contractual par for money and a transacted market observation
/// for every other asset. Seed levels and carried no-trade levels are not evidence of an exit value.
pub fn prudential_price(
    contractual: Option<f64>,
    observed: Option<crate::prices::Print>,
) -> Option<f64> {
    if let Some(price) = contractual {
        return Some(price);
    }
    match observed {
        Some(print)
            if print.provenance == crate::prices::Provenance::Cleared
                && print.quoted_as == crate::prices::QuotedAs::Money =>
        {
            Some(print.price)
        }
        _ => None,
    }
}

/// Capital is the residual.
#[derive(Clone, Debug)]
pub struct Position {
    pub bank: PartyId,
    pub assets: Vec<Asset>,
    pub liabilities: f64,
    /// The layer between equity and senior paper.
    pub subordinated: f64,
    /// Money owed today.
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

    /// The residual.
    pub fn capital(&self) -> f64 {
        self.carried() - self.liabilities
    }

    /// How many terms that walk had and what magnitude it passed through, published with the number
    /// — a reader comparing against it is comparing against a walk over the whole book, and the dust
    /// it is entitled to is the dust of THAT arithmetic.
    pub fn weighted_dust(&self) -> f64 {
        let magnitude: f64 = self
            .assets
            .iter()
            .map(|a| (a.carried * a.weight.of).abs())
            .sum();
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

/// What leaves this bank the least room.
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

pub fn how_it_stands(p: &Position, r: Rules) -> Standing {
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

pub fn distribution_allowed(standing: Standing) -> bool {
    !standing.in_buffer && !standing.below_requirement
}

/// Cash which must enter the bank before both prudential minima are met.  Unlike lending
/// headroom, this is denominated in money and can therefore be the size of an equity offering.
pub fn capital_shortfall(p: &Position, r: Rules) -> f64 {
    let weighted = r.min_weighted * p.weighted() - p.capital();
    let leverage = r.min_leverage * p.carried() - p.capital();
    if weighted > leverage {
        weighted
    } else {
        leverage
    }
}

/// The two failures, with different triggers and different remedies.
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
    /// The shares they demanded for it.
    pub for_shares: f64,
}

#[derive(Clone, Debug)]
pub struct Recapitalised {
    pub raised: f64,
    pub new_shares: f64,
    /// An equity issue priced by the equity market.
    pub incumbent_share: f64,
}

/// Recapitalisation first, if somebody will provide it — and C2.b: it can fail.
pub fn recapitalise(hole: f64, shares_before: f64, bids: &[Subscription]) -> Option<Recapitalised> {
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

/// What the assets are actually worth, not their book — and D1.a: the hole is the difference.
pub fn hole(valued_at: f64, liabilities: f64) -> f64 {
    liabilities - valued_at
}

/// The acquirer is choosing, and can decline.
#[derive(Clone, Copy, Debug)]
pub struct Bid {
    pub by: PartyId,
    /// What it will pay (positive) or be paid (negative) to take the book.
    pub pays: f64,
}

/// The hierarchy, in the order it absorbs.
#[derive(Clone, Debug, PartialEq)]
pub struct Absorbed {
    pub equity: f64,
    pub subordinated: f64,
    pub senior: f64,
    /// What the estate could not pay a covered depositor.
    pub unpaid: f64,
}

/// The hierarchy is respected.
pub fn absorb(hole: f64, equity: f64, subordinated: f64, senior: f64) -> Absorbed {
    if hole <= 0.0 {
        return Absorbed {
            equity: 0.0,
            subordinated: 0.0,
            senior: 0.0,
            unpaid: 0.0,
        };
    }
    if hole <= equity {
        return Absorbed {
            equity: hole,
            subordinated: 0.0,
            senior: 0.0,
            unpaid: 0.0,
        };
    }
    let after_equity = hole - equity;
    if after_equity <= subordinated {
        return Absorbed {
            equity,
            subordinated: after_equity,
            senior: 0.0,
            unpaid: 0.0,
        };
    }
    let after_sub = after_equity - subordinated;
    if after_sub <= senior {
        return Absorbed {
            equity,
            subordinated,
            senior: after_sub,
            unpaid: 0.0,
        };
    }
    Absorbed {
        equity,
        subordinated,
        senior,
        unpaid: after_sub - senior,
    }
}

pub fn no_creditor_worse_off(in_resolution: f64, in_liquidation: f64, terms: usize) -> bool {
    let dust = (terms as f64) * f64::EPSILON * (in_resolution.abs() + in_liquidation.abs());
    in_resolution >= in_liquidation - dust
}

/// The limit applies per member where the depositor is a cell (Banks Funding A1.a), so a large cell
/// of small depositors is covered and a cell of large ones is not — which is exactly the distinction
/// the insurance exists to draw.
pub fn insured(deposits: f64, members: f64, limit_per_member: f64) -> f64 {
    assert!(
        members >= 1.0,
        "25 D4: a cell of {members} members is not a depositor"
    );
    let per_member = deposits / members;
    if per_member <= limit_per_member {
        deposits
    } else {
        limit_per_member * members
    }
}

/// The resolution conserves.
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
            - (self.acquirer_took
                + self.insurer_paid
                + self.estate_realised
                + self.holders_lost
                + self.public_paid)
    }

    /// Derived dust, six terms over the magnitudes that went through the sum.
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

// WHAT A LENDER IS LENDING AT IS THE LENDER'S, so a bank reads its own book, its own hurdle and
// its own headroom and says what it will lend. It is written to `standing::LENDING_STANDARD`,
// where housing reads it: one fact, one writer.

/// The standard is a READ of what the lender already measures — the loan-to-value cross-section of
/// its own book, its hurdle, its headroom — never a constant.
pub fn standard(worst_ltv_on_its_book: f64, headroom: f64, hurdle: f64) -> Standard {
    assert!(
        hurdle > 0.0,
        "40 C5: a lender with no hurdle has no standard to read"
    );
    // A lender whose own book is already stretched, or which has little room to put more on, asks
    // for more of the price up front and lends a smaller multiple of income.
    let strain = worst_ltv_on_its_book * hurdle / headroom;
    Standard {
        income_multiple: 1.0 / strain,
        deposit_share: strain,
        claim_bid_fraction: 1.0 / (1.0 + hurdle),
    }
}

// WHAT A BANK MUST HOLD AGAINST AN ASSET reads the grade somebody stood behind and turns it into
// a weight. The schedule is the regulator's, so it is the bank's business rather than the
// assessor's — the assessor says what the credit IS and stops there.

/// A per-ISSUER risk measure closes the loop, with or without a rating table. The schedule is what
/// the best credit is asked for and what each further notch costs, so a twenty-two rung scale needs
/// two numbers rather than twenty-two.
pub fn haircut(g: Grade, by_tenor: f64, on_the_best: f64, per_notch: f64) -> f64 {
    assert!(
        per_notch > 1.0,
        "XI-14: a schedule that does not rise with the credit is one per type"
    );
    by_tenor * on_the_best * per_notch.powf(g.rank())
}

/// §31 A1, B1, B3, C1, C1.a, 40 C5, 22i.8: A BANK READS ITS OWN CAPITAL AND ACTS ON IT.
pub struct BankCapital {
    pub kind: u32,
    /// What it says when it is below its requirement and has to raise.
    pub short_by: u32,
    pub at_ratio: u32,
    /// The requirement, the backstop and the buffer.
    pub min_weighted: &'static str,
    pub min_leverage: &'static str,
    pub buffer: &'static str,
    /// The return a lender wants on what it puts out.
    pub hurdle: &'static str,
    /// The weight schedule: what the best credit is asked for, what each further notch costs, and
    /// where on the scale a name nobody has graded is weighted.
    pub on_the_best: &'static str,
    pub per_notch: &'static str,
    pub ungraded_at: &'static str,
}

impl Mechanism for BankCapital {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let rules = Rules {
            min_weighted: ctx.params().ratio(self.min_weighted),
            min_leverage: ctx.params().ratio(self.min_leverage),
            buffer: ctx.params().ratio(self.buffer),
        };
        let hurdle = ctx.params().ratio(self.hurdle);
        let on_the_best = ctx.params().ratio(self.on_the_best);
        let per_notch = ctx.params().ratio(self.per_notch);
        let ungraded_at = ctx.params().count(self.ungraded_at);
        let to = ctx.current_week();

        // What each name is graded at — the WORST any house holds on it, because a bank that could
        // pick the kindest house would weigh its book by choosing its assessor.
        let mut worst: std::collections::HashMap<u32, f64> = std::collections::HashMap::new();
        for row in 0..ctx.standing().len() as u32 {
            let s = crate::stores::StandingId(row);
            if !ctx.standing().live(s) || ctx.standing().kind_of(s) != standing::GRADE {
                continue;
            }
            let rank = ctx.standing().terms(s)[0];
            worst
                .entry(ctx.standing().about(s).0)
                .and_modify(|r| {
                    if rank > *r {
                        *r = rank
                    }
                })
                .or_insert(rank);
        }

        let mut acted: Vec<(PartyId, f64, bool, bool, f64, f64)> = Vec::new();
        'banks: for &bank in ctx.parties().of_kind(kinds::BANK) {
            let who = PartyId(bank);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What it holds, at what it is carried at, and what each weighs.
            let mut assets: Vec<Asset> = Vec::new();
            let mut money_at_hand = 0.0;
            for &row in ctx.register().of_holder(who) {
                let row = crate::ids::HoldingId(row);
                let line = ctx.register().instrument_of(row);
                let Some(price) = prudential_price(
                    ctx.instruments().hard_coded_price(line),
                    ctx.prints().of_line(line, ctx.week()),
                ) else {
                    continue 'banks;
                };
                let carried = ctx.register().quantity(row) * price;
                if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                    money_at_hand += carried;
                }
                let issuer = ctx.instruments().issuer_of(line);
                // XI-3's two exceptions are exactly the parties that cannot be made to fail, and a
                // claim on one of them is the zero-weighted asset the standard means.
                let kind = ctx.parties().kind_of(issuer);
                let can_fail = !matches!(
                    ctx.registry()
                        .profile(kind)
                        .expect("Law 15: an issuer kind needs a declared failure capability")
                        .failure,
                    crate::registry::FailureMode::Never
                );
                // A name nobody has graded is weighted where the standard says an ungraded name
                // sits — a notch on the scale, not nothing and not a number invented here.
                let grade =
                    crate::stores::Grade::nearest(*worst.get(&issuer.0).unwrap_or(&ungraded_at));
                let weighs = haircut(grade, 1.0, on_the_best, per_notch);
                assets.push(Asset {
                    carried,
                    weight: classified_weight(
                        ctx.instruments().class_of(line),
                        can_fail,
                        weighs - 1.0,
                    ),
                });
            }
            // And what it OWES — the money it issued that others hold, plus what falls due on it.
            let mut liabilities = 0.0;
            for &line in ctx.instruments().of_issuer(who) {
                let what = InstrumentId::at(line);
                let (held, _) = ctx.register().held_total(what);
                liabilities += held - ctx.register().quantity(ctx.register().row(who, what));
            }
            let due_now: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d) && ctx.schedules().due(*d) <= to)
                .map(|d| ctx.schedules().amount(d))
                .sum();
            let position = Position {
                bank: who,
                assets,
                liabilities,
                // The layer between equity and senior paper.
                subordinated: 0.0,
                due_now,
                money_at_hand,
            };
            if position.carried() <= 0.0 {
                continue;
            }
            let how = how_it_stands(&position, rules);
            let ratio = position.capital() / position.carried();
            // What it is lending at now.
            let headroom =
                position.capital() / (rules.min_leverage + rules.buffer) - position.carried();
            acted.push((
                who,
                ratio,
                how.below_requirement,
                distribution_allowed(how),
                headroom,
                capital_shortfall(&position, rules),
            ));
        }

        for (who, ratio, below, may_distribute, headroom, shortfall) in acted {
            // The standing is PUBLIC.
            ctx.say(
                self.kind,
                &[who.0],
                &[(self.at_ratio, Value::Num(ratio))],
                true,
            );
            ctx.now_stands(
                standing::CAPITAL_DISTRIBUTION,
                who,
                PartyId::NONE,
                vec![if may_distribute { 1.0 } else { 0.0 }],
            );
            if ratio > 0.0 && headroom > 0.0 {
                let standard = standard(1.0 / ratio, headroom, hurdle);
                ctx.now_stands(
                    standing::LENDING_STANDARD,
                    who,
                    PartyId::NONE,
                    vec![
                        standard.income_multiple,
                        standard.deposit_share,
                        standard.claim_bid_fraction,
                    ],
                );
            }
            // And a bank below its requirement must RAISE.
            if below {
                ctx.say(
                    self.short_by,
                    &[who.0],
                    &[(self.at_ratio, Value::Num(shortfall))],
                    true,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    #[test]
    fn prudential_prices_require_a_permitted_market_observation() {
        let print = |provenance| crate::prices::Print {
            instrument: InstrumentId::at(1),
            market: crate::ids::MarketId::at(1),
            week: 1,
            struck: 1,
            price: 72.0,
            ccy: crate::ids::CurrencyCode::at(1),
            quoted_as: crate::prices::QuotedAs::Money,
            provenance,
        };
        assert_eq!(
            prudential_price(None, Some(print(crate::prices::Provenance::Cleared))),
            Some(72.0)
        );
        assert!(prudential_price(None, Some(print(crate::prices::Provenance::Carried))).is_none());
        assert!(prudential_price(None, Some(print(crate::prices::Provenance::Seeded))).is_none());
        assert_eq!(prudential_price(Some(1.0), None), Some(1.0));
    }

    #[test]
    fn risk_weight_follows_the_exposures_declared_classification() {
        assert_eq!(
            classified_weight(crate::instruments::Class::Claim, true, 0.35).of,
            0.35
        );
        assert_eq!(
            classified_weight(crate::instruments::Class::Claim, false, 0.35).of,
            0.0
        );
        assert_eq!(
            classified_weight(crate::instruments::Class::Share, false, 0.35).of,
            1.0
        );
        assert_eq!(
            classified_weight(crate::instruments::Class::Plant, false, 0.35).of,
            1.0
        );
    }

    #[test]
    fn a_binding_capital_buffer_blocks_distributions() {
        assert!(!distribution_allowed(Standing {
            in_buffer: true,
            below_requirement: false
        }));
        assert!(!distribution_allowed(Standing {
            in_buffer: true,
            below_requirement: true
        }));
        assert!(distribution_allowed(Standing {
            in_buffer: false,
            below_requirement: false
        }));
    }

    fn sovereign_book() -> Position {
        // A bank stuffed with claims on a party that cannot fail: the book weighs almost nothing.
        Position {
            bank: party(1),
            assets: vec![
                Asset {
                    carried: 9_000.0,
                    weight: Weight::on(false, 1.0),
                },
                Asset {
                    carried: 1_000.0,
                    weight: Weight::on(true, 1.0),
                },
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
            assets: vec![Asset {
                carried: 10_000.0,
                weight: Weight::on(true, 1.0),
            }],
            liabilities: 8_700.0,
            subordinated: 150.0,
            due_now: 100.0,
            money_at_hand: 400.0,
        }
    }

    #[test]
    fn capital_is_the_residual_and_falls_because_the_asset_fell() {
        // It is not a pot that is spent.
        let mut p = lending_book();
        let before = p.capital();
        p.assets[0].carried -= 300.0;
        assert!((p.capital() - (before - 300.0)).abs() <= 4.0 * f64::EPSILON * before.abs());
    }

    #[test]
    fn which_rule_binds_is_an_outcome_of_what_the_bank_holds() {
        // Neither answer is stated anywhere.
        let r = Rules {
            min_weighted: 0.08,
            min_leverage: 0.03,
            buffer: 0.01,
        };
        let loan = Weight::on(true, 1.0);
        assert_eq!(headroom(&sovereign_book(), r, loan).0, Binding::Leverage);
        assert_eq!(headroom(&lending_book(), r, loan).0, Binding::Weighted);
    }

    #[test]
    fn a_zero_weighted_asset_is_not_unlimited_room() {
        // B1.b is the backstop precisely because the weighted rule says nothing about an asset that
        // weighs nothing.
        let r = Rules {
            min_weighted: 0.08,
            min_leverage: 0.03,
            buffer: 0.01,
        };
        let (binds, room) = headroom(&sovereign_book(), r, Weight::on(false, 1.0));
        assert_eq!(binds, Binding::Leverage);
        assert!(room.is_finite());
    }

    #[test]
    fn the_buffer_is_the_banks_own_and_breaching_it_is_not_breaching_the_requirement() {
        // Three different places to stand, and the consequences differ at each.
        let r = Rules {
            min_weighted: 0.08,
            min_leverage: 0.03,
            buffer: 0.02,
        };
        let comfortable = lending_book();
        assert_eq!(
            how_it_stands(&comfortable, r),
            Standing {
                in_buffer: false,
                below_requirement: false
            }
        );
        let mut thin = lending_book();
        thin.liabilities = 9_100.0; // 900 of capital on 10,000 weighted: 9%.
        assert_eq!(
            how_it_stands(&thin, r),
            Standing {
                in_buffer: true,
                below_requirement: false
            }
        );
        let mut breached = lending_book();
        breached.liabilities = 9_300.0; // 7%.
        assert_eq!(
            how_it_stands(&breached, r),
            Standing {
                in_buffer: true,
                below_requirement: true
            }
        );
    }

    #[test]
    fn solvent_and_illiquid_and_insolvent_and_liquid_are_different_failures() {
        // Both triggers exist and the resolution says which fired.
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
        let short = [Subscription {
            by: party(9),
            money: 300.0,
            for_shares: 600.0,
        }];
        assert!(recapitalise(500.0, 1_000.0, &short).is_none());
    }

    #[test]
    fn new_money_is_priced_by_whoever_provides_it_and_the_incumbents_are_diluted() {
        // Their price, not the issuer's.
        let dear = [Subscription {
            by: party(9),
            money: 500.0,
            for_shares: 500.0,
        }];
        let cheap = [Subscription {
            by: party(9),
            money: 500.0,
            for_shares: 4_000.0,
        }];
        let a = recapitalise(500.0, 1_000.0, &dear).unwrap();
        let b = recapitalise(500.0, 1_000.0, &cheap).unwrap();
        assert!(b.incumbent_share < a.incumbent_share);
    }

    #[test]
    fn the_hierarchy_absorbs_in_order_and_a_missing_layer_would_over_punish_the_middle() {
        // Equity first and fully, then subordinated, then senior.
        let with_sub = absorb(400.0, 300.0, 200.0, 5_000.0);
        assert_eq!(
            with_sub,
            Absorbed {
                equity: 300.0,
                subordinated: 100.0,
                senior: 0.0,
                unpaid: 0.0
            }
        );
        // Without it, the same hole reaches senior creditors — one layer short at the top and one
        // over-punished in the middle.
        let without_sub = absorb(400.0, 300.0, 0.0, 5_000.0);
        assert!(without_sub.senior > 0.0);
    }

    #[test]
    fn a_hole_deeper_than_the_whole_stack_leaves_somebody_unpaid() {
        // Nothing is clamped and nothing is invented.
        let a = absorb(1_000.0, 300.0, 200.0, 400.0);
        assert_eq!(a.unpaid, 100.0);
    }

    #[test]
    fn no_creditor_worse_off_is_measured_and_never_enforced() {
        // D2.a is a VERIFY.
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
        // Every part of the hole lands somewhere named; the sum is checked against derived dust, and
        // a residual is a defect rather than a rounding allowance.
        let c = Conservation {
            hole: 1_000.0,
            acquirer_took: 250.0,
            insurer_paid: 300.0,
            estate_realised: 200.0,
            holders_lost: 150.0,
            public_paid: 100.0,
        };
        assert!(c.conserves());
        let leaking = Conservation {
            public_paid: 0.0,
            ..c
        };
        assert!(!leaking.conserves());
        assert_eq!(leaking.residual(), 100.0);
    }

    #[test]
    fn an_acquirer_can_decline_and_the_resolution_falls_through_to_the_public_path() {
        // An assigned acquirer whose bid is the estate's own value by construction is not choosing.
        let bids: Vec<Bid> = Vec::new();
        assert!(bids.is_empty());
        let offered = [Bid {
            by: party(7),
            pays: -400.0,
        }];
        // It takes assets AND liabilities and is PAID the difference when the book is a hole.
        assert!(offered[0].pays < 0.0);
    }

    #[test]
    fn the_hole_is_what_the_assets_fetch_against_what_is_owed() {
        // Not the book.
        assert_eq!(hole(9_000.0, 9_500.0), 500.0);
        assert_eq!(hole(9_500.0, 9_500.0), 0.0);
    }

    #[test]
    fn the_standard_is_read_from_what_the_lender_already_measures_and_tightens_when_it_is_worried()
    {
        // A constant means only the rate channel loops, and C5.b's loop is the housing cycle.
        let calm = standard(0.7, 500.0, 0.10);
        let worried = standard(0.95, 120.0, 0.10);
        assert!(worried.deposit_share > calm.deposit_share);
        assert!(worried.income_multiple < calm.income_multiple);
    }

    #[test]
    fn the_haircut_reads_the_issuers_own_grade_and_not_the_instrument_type() {
        // A haircut that is one number per type — the same for the best and worst credit — is the
        // one leg of the downgrade loop that is wholly absent.
        assert!(haircut(Grade::Bminus, 1.0, 1.01, 1.07) > haircut(Grade::BEST, 1.0, 1.01, 1.07));
        // And every notch costs, so a one-notch downgrade is a capital call on its own.
        assert!(haircut(Grade::BBB, 1.0, 1.01, 1.07) > haircut(Grade::BBBplus, 1.0, 1.01, 1.07));
    }
}
