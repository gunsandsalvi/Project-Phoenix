//! FUND SHARES: a named party whose liability is its shares, whose equity is zero by construction,
//! and whose redemption is the forced-seller channel.
//!
//! @spec 13 A1 · 13 A2 · 13 A3 · 13 A4 · 13 B1 · 13 B2 · 13 B2.a · 13 B3 · 13 B4 · 13 C1 · 13 C1.a ·
//! @spec 13 C2 · 13 C2.a · 13 C2.b · 13 C3 · 13 C4 · 13 C4.a · 13 C5 · 13 D1 · 13 D2 · 13 D2.a ·
//! @spec 13 D3 · 13 D4 · 13 D5 · 13 E1 · 13 E2 · 13 E3 · 13 E3.a · 13 E4 · 13 F1 · 13 F2 · 13 F3 ·
//! @spec 13 G1 · 13 G1.a · 13 G1.b · XI-2 · Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{InstrumentId, MarketId, PartyId};
use crate::instruments::Class;
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt};
use crate::module::{Mechanism, MechanismContext};
use crate::module::{Participant, ParticipantView};
use crate::stores::agreed;

/// A named party whose liability is its shares, held by named holders, counted in shares.
#[derive(Clone, Debug)]
pub struct Fund {
    pub who: PartyId,
    /// The manager is a separate party that earns the fee — its income and the fund's cost.
    pub manager: PartyId,
    /// Marked at cleared prices.
    pub assets: Vec<(InstrumentId, f64)>,
    /// No leverage without a lender.
    pub borrowed: Vec<(PartyId, f64)>,
    pub cash: f64,
    pub shares: f64,
    /// Fees accrue and are paid to the manager, and they reduce NAV.
    pub fees_accrued: f64,
    /// The mandate is a real constraint on what it buys, not a label.
    pub may_hold: Vec<InstrumentId>,
}

impl Fund {
    pub fn assets_at_market(&self) -> f64 {
        self.cash + self.assets.iter().map(|(_, v)| v).sum::<f64>()
    }

    pub fn liabilities(&self) -> f64 {
        self.fees_accrued + self.borrowed.iter().map(|(_, v)| v).sum::<f64>()
    }

    /// (assets at market minus liabilities) divided by shares outstanding — a READ, every time.
    pub fn nav(&self) -> Option<f64> {
        if self.shares <= 0.0 {
            return None;
        }
        Some((self.assets_at_market() - self.liabilities()) / self.shares)
    }

    /// Its equity is zero by construction, because the holders own the assets.
    pub fn equity(&self, holders_shares: f64) -> Option<f64> {
        let nav = self.nav()?;
        Some(self.assets_at_market() - self.liabilities() - holders_shares * nav)
    }

    /// What the mandate allows.
    pub fn may_buy(&self, what: InstrumentId) -> bool {
        self.may_hold.contains(&what)
    }
}

/// A fund with equity has mislaid somebody's money, and the sum of holders' share value equals
/// assets minus liabilities, exactly.
pub fn mislaid(f: &Fund, holders_shares: f64, terms: usize) -> Option<f64> {
    let over = f.equity(holders_shares)?;
    if over.abs() <= crate::num::dust(terms, &[f.assets_at_market(), f.liabilities()]) {
        return None;
    }
    Some(over)
}

/// A redemption takes shares back and pays the holder cash at NAV — and the fund must FIND the cash:
/// from its buffer, or by SELLING.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Redeemed {
    pub holder: PartyId,
    pub shares: f64,
    pub owed: f64,
    /// From the buffer.
    pub from_cash: f64,
    /// And the rest must be sold — a trade into a market that must clear, at whatever price it
    /// clears.
    pub must_sell: f64,
}

pub fn redeem(f: &Fund, holder: PartyId, shares: f64) -> Option<Redeemed> {
    let nav = f.nav()?;
    let owed = shares * nav;
    let from_cash = if f.cash < owed { f.cash } else { owed };
    Some(Redeemed {
        holder,
        shares,
        owed,
        from_cash,
        must_sell: owed - from_cash,
    })
}

/// The holder is paid at today's NAV, the sales happen at tomorrow's prices, and the difference
/// falls on the remaining holders — which is why a redemption is a real cost to those who stay, and
/// why runs are a thing.
pub fn cost_to_those_who_stay(r: &Redeemed, sold_for: f64) -> f64 {
    r.must_sell - sold_for
}

/// Shares created minus redeemed equals shares outstanding, and cash in and out matches.
pub fn shares_reconcile(
    created: f64,
    redeemed: f64,
    outstanding: f64,
    terms: usize,
) -> Option<f64> {
    let implied = created - redeemed;
    let off = implied - outstanding;
    if off.abs() <= crate::num::dust(terms, &[created, redeemed, outstanding]) {
        return None;
    }
    Some(off)
}

/// No guaranteed constant NAV.
pub fn broke_the_buck(f: &Fund, issued_at: f64) -> bool {
    match f.nav() {
        Some(nav) => nav < issued_at,
        None => false,
    }
}

/// A saver chooses between a bank deposit, a money fund and bills directly, and the money fund's
/// yield competes with the deposit rate — which is a real constraint on what banks pay.
pub fn beats_the_deposit(fund_yield: f64, deposit_rate: f64) -> bool {
    fund_yield > deposit_rate
}

/// An exchange-traded fund has two values — the traded price and the NAV — and they are different
/// numbers.
pub fn premium(traded_price: f64, nav: f64) -> Option<f64> {
    if nav <= 0.0 {
        return None;
    }
    Some(traded_price / nav - 1.0)
}

/// The gap closes because somebody TRADES — a reason for a participant, not a rule tying the two —
/// and it can persist when they will not.
pub fn arbitrages(gap: f64, costs_to_do_it: f64, can_fund: f64) -> Option<f64> {
    if gap.abs() <= costs_to_do_it || can_fund <= 0.0 {
        return None;
    }
    Some(can_fund)
}

/// An in-kind redemption means that vehicle is NOT a forced seller.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Redeems {
    InCash,
    InKind,
}

/// An investor holding a scalar with no share count cannot ask for its money back, and a vehicle
/// like that is outside this system whatever it is called.
pub fn is_redeemable(shares_held: Option<f64>) -> bool {
    matches!(shares_held, Some(s) if s > 0.0)
}

/// A POOL WHOSE MANAGER DIED.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Run {
    /// Somebody's mandate is live over it, and it decides as it always did.
    Mandated,
    /// Nobody's is.
    Orphaned,
}

/// How many live mandates a pool is under, and therefore whether anybody decides for it.
pub fn run_as(live_mandates: usize) -> Run {
    if live_mandates > 0 {
        Run::Mandated
    } else {
        Run::Orphaned
    }
}

/// What an orphaned pool sells this week — everything it holds that a book will take, at what a
/// book will pay.
pub fn winding_sale(held: f64) -> Option<f64> {
    if held > 0.0 {
        Some(held)
    } else {
        None
    }
}

/// And what each holder gets — pro rata on the shares they hold, out of what the pool has actually
/// raised.
pub fn pro_rata(cash: f64, shares_held: f64, shares_outstanding: f64) -> Option<f64> {
    if shares_outstanding <= 0.0 {
        return None;
    }
    Some(cash * shares_held / shares_outstanding)
}

/// And when it is over.
pub fn is_wound_up(holds: f64, shares_outstanding: f64) -> bool {
    holds <= 0.0 && shares_outstanding <= 0.0
}

/// A gate is a fund decision with a durable state, not an omitted or silently rationed payment.
pub fn redemption_gate(assets_still_to_sell: f64) -> bool {
    assets_still_to_sell > 0.0
}

/// A POOL WHOSE MANAGER DIED WINDS UP THROUGH THE MACHINERY IT ALREADY HAS.
pub struct Winding {
    /// The kind it publishes under, so a reader can see a pool lose its manager.
    pub says: u32,
    pub gate_says: u32,
    pub ceased: u32,
    pub at_trigger: u32,
    pub at_destination: u32,
}

impl Mechanism for Winding {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut paying: Vec<(PartyId, PartyId, InstrumentId, InstrumentId, f64, f64)> = Vec::new();
        let mut ending: Vec<PartyId> = Vec::new();
        let mut orphaned: Vec<PartyId> = Vec::new();

        let pools = ctx.parties().of_kind(kinds::FUND).to_vec();
        for p in pools {
            let pool = PartyId::at(p);
            if !ctx.parties().alive(pool) {
                continue;
            }
            // Whether anybody decides for it is a read of the RELATIONS, never a flag on the pool
            // that somebody has to remember to clear.
            let live = ctx
                .agreements()
                .of_party(pool)
                .iter()
                .map(|r| crate::stores::AgreementId(*r))
                .filter(|a| {
                    ctx.agreements().live(*a) && ctx.agreements().kind_of(*a) == agreed::MANDATE
                })
                .count();
            if run_as(live) == Run::Mandated {
                continue;
            }
            orphaned.push(pool);

            // What it has raised is what there is to pay with — its own account, and nothing else.
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), pool) else {
                continue;
            };
            let cash = ctx.register().quantity(ctx.register().row(pool, money));

            // A holder of its shares has a redeemable claim, and a share count is what makes a claim
            // redeemable.
            let Some(shares) = (0..ctx.instruments().len())
                .map(|r| InstrumentId::at(r as u32))
                .find(|i| {
                    ctx.instruments().issuer_of(*i) == pool
                        && ctx.instruments().class_of(*i) == Class::Share
                })
            else {
                continue;
            };
            let (outstanding, _) = ctx.register().held_total(shares);
            let held_by_it = ctx.register().quantity(ctx.register().row(pool, shares));
            let out = outstanding - held_by_it;

            let still_holds: f64 = ctx
                .register()
                .of_holder(pool)
                .iter()
                .map(|r| crate::ids::HoldingId(*r))
                .filter(|r| {
                    let line = ctx.register().instrument_of(*r);
                    line != money && line != shares
                })
                .map(|r| ctx.register().quantity(r))
                .sum();

            let gated = redemption_gate(still_holds);
            let recorded = ctx
                .standing()
                .of_party_about(pool, pool, crate::stores::standing::REDEMPTION_GATE)
                .and_then(|standing| ctx.standing().terms(standing).first().copied());
            let gate_term = if gated { 1.0 } else { 0.0 };
            if recorded != Some(gate_term) {
                ctx.now_stands(
                    crate::stores::standing::REDEMPTION_GATE,
                    pool,
                    pool,
                    vec![gate_term],
                );
                ctx.say(
                    self.gate_says,
                    &[pool.0],
                    &[(0, Value::Num(gate_term))],
                    true,
                );
            }

            if out <= 0.0 {
                // Nothing held and nobody owed is a pool that has ended.
                if still_holds <= 0.0 {
                    ending.push(pool);
                }
                continue;
            }
            // Redemptions wait until the pool has sold its assets. The cash read below is therefore
            // settled sale proceeds, not a promise based on a mark or a dropped unfunded balance.
            if gated || cash <= 0.0 {
                continue;
            }
            for row in ctx.register().of_instrument(shares) {
                let row = crate::ids::HoldingId(*row);
                let holder = ctx.register().holder_of(row);
                if holder == pool {
                    continue;
                }
                let Some(share) = pro_rata(cash, ctx.register().quantity(row), out) else {
                    continue;
                };
                if share > 0.0 {
                    paying.push((
                        pool,
                        holder,
                        money,
                        shares,
                        share,
                        ctx.register().quantity(row),
                    ));
                }
            }
        }

        for pool in orphaned {
            ctx.say(self.says, &[pool.0], &[], true);
        }
        for (pool, holder, money, shares, amount, units) in paying {
            let Some(amount) = crate::ledger::Units::new(amount) else {
                continue;
            };
            let Some(units) = crate::ledger::Units::new(units) else {
                continue;
            };
            ctx.propose(
                vec![
                    Leg::Money {
                        from: pool,
                        to: holder,
                        instrument: money,
                        amount,
                        receipt: Receipt::Principal,
                    },
                    Leg::Destroy {
                        party: holder,
                        instrument: shares,
                        qty: units,
                        why: crate::ledger::Gone::Redeemed,
                    },
                ],
                Cause::CorporateAction,
                Delivery::Nothing,
                "a winding pool redeeming shares only from its settled sale proceeds",
            );
        }
        for pool in ending {
            ctx.winds_up(pool, self.ceased, self.at_trigger, self.at_destination);
        }
    }
}

/// A fund must buy something with the cash, per its mandate — which is why a flow into a fund
/// becomes a purchase of what the mandate allows, and why a fund is a transmission channel.
pub struct FundMandates {
    /// The mandate is a real constraint on what it buys, not a label.
    pub may_hold: Vec<InstrumentId>,
}

impl Participant for FundMandates {
    /// A fund publishes a net asset value, so every position in it is marked.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        Some(crate::register::Carrying::Market)
    }

    fn party_kind(&self) -> u32 {
        kinds::FUND
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        // A pool under nobody's mandate posts nothing, because a schedule is somebody's and there is
        // nobody here whose view the order would be.
        if run_as(view.agreed(agreed::MANDATE).len()) == Run::Orphaned {
            return Vec::new();
        }
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.may_hold
            .iter()
            .filter_map(|line| view.market_of(*line))
            .collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        if run_as(view.agreed(agreed::MANDATE).len()) == Run::Orphaned {
            return Vec::new();
        }
        // A line outside the mandate is one it cannot buy, whatever it is worth.
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        if !self.may_hold.contains(&line) {
            return Vec::new();
        }
        let money = view.own_cash();
        let Some(will_pay) = view.subject_of(m).and_then(|line| view.values(line)) else {
            return Vec::new();
        };
        // Less what it is already bidding for here, or it commits the same money twice.
        let (already, _) = view.resting(m);
        let affordable = whole_pieces(money / will_pay) - already;
        if affordable <= 0 {
            return Vec::new();
        }
        vec![Order {
            party: view.self_id(),
            side: Side::Buy,
            price: Some(will_pay),
            qty: affordable,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn holds(n: u32) -> InstrumentId {
        InstrumentId::at(n)
    }

    fn fund() -> Fund {
        Fund {
            who: party(40),
            manager: party(41),
            assets: vec![(holds(1), 8_000.0), (holds(2), 1_500.0)],
            borrowed: Vec::new(),
            cash: 500.0,
            shares: 1_000.0,
            fees_accrued: 0.0,
            may_hold: vec![holds(1), holds(2)],
        }
    }

    #[test]
    fn nav_is_a_read_and_a_fund_with_no_shares_has_none() {
        // Never a stored series.
        assert_eq!(fund().nav(), Some(10.0));
        let empty = Fund {
            shares: 0.0,
            ..fund()
        };
        assert!(empty.nav().is_none());
    }

    #[test]
    fn a_fund_with_equity_has_mislaid_somebodys_money() {
        // Assets minus liabilities is zero because the holders own the assets.
        let f = fund();
        assert!(mislaid(&f, 1_000.0, 4).is_none());
        // Holders whose shares do not add up to the fund's book is exactly the defect.
        assert_eq!(mislaid(&f, 900.0, 4), Some(1_000.0));
    }

    #[test]
    fn fees_accrue_to_the_manager_and_reduce_the_nav() {
        // The manager is a separate party; the fee is its income and the fund's cost.
        let f = fund();
        let after_fees = Fund {
            fees_accrued: 200.0,
            ..f.clone()
        };
        assert!(after_fees.nav().unwrap() < f.nav().unwrap());
        assert_ne!(f.manager, f.who);
    }

    #[test]
    fn a_stale_price_makes_a_stale_nav_and_somebody_transacts_on_it() {
        // That is a real transfer between holders, not a rounding.
        let stale = fund();
        let marked = Fund {
            assets: vec![(holds(1), 7_000.0), (holds(2), 1_500.0)],
            ..fund()
        };
        let on_stale = redeem(&stale, party(50), 100.0).unwrap();
        let on_fresh = redeem(&marked, party(50), 100.0).unwrap();
        assert!(on_stale.owed > on_fresh.owed);
    }

    #[test]
    fn a_redemption_beyond_the_buffer_must_be_sold_and_that_is_the_forced_seller_channel() {
        // A redemption rationed by the fund's cash, with the unfilled part dropped, deletes the
        // entire system.
        let f = fund();
        let small = redeem(&f, party(50), 30.0).unwrap();
        assert_eq!(small.from_cash, 300.0);
        assert_eq!(small.must_sell, 0.0);
        let large = redeem(&f, party(50), 400.0).unwrap();
        assert_eq!(large.owed, 4_000.0);
        assert_eq!(large.from_cash, 500.0);
        assert_eq!(large.must_sell, 3_500.0);
    }

    #[test]
    fn the_cost_of_a_late_sale_lands_on_the_holders_who_stayed() {
        // The holder is paid at today's NAV and the sales happen at tomorrow's prices — which is why
        // runs are a thing.
        let r = redeem(&fund(), party(50), 400.0).unwrap();
        assert!(cost_to_those_who_stay(&r, 3_500.0) == 0.0);
        assert!(cost_to_those_who_stay(&r, 3_100.0) > 0.0);
    }

    #[test]
    fn a_money_fund_can_break_the_buck() {
        // A fund that cannot is a fund with a hidden guarantor, and the guarantor is nobody.
        let f = fund();
        assert!(!broke_the_buck(&f, 10.0));
        let hit = Fund {
            assets: vec![(holds(1), 7_000.0), (holds(2), 1_500.0)],
            ..fund()
        };
        assert!(broke_the_buck(&hit, 10.0));
    }

    #[test]
    fn the_saver_compares_the_fund_with_the_deposit_and_the_flow_follows() {
        // The competition is a real constraint on what banks pay, and flows are a CONSEQUENCE rather
        // than an imposed allocation.
        assert!(beats_the_deposit(0.045, 0.030));
        assert!(!beats_the_deposit(0.020, 0.030));
    }

    #[test]
    fn an_etfs_gap_closes_because_somebody_trades_and_can_persist_when_nobody_will() {
        // Two different numbers, and the premium is a read of them.
        let p = premium(10.4, 10.0).unwrap();
        assert!(p > 0.0);
        assert!(arbitrages(p, 0.01, 1_000_000.0).is_some());
        assert!(arbitrages(p, 0.10, 1_000_000.0).is_none());
        assert!(arbitrages(p, 0.01, 0.0).is_none());
        assert!(premium(10.4, 0.0).is_none());
    }

    #[test]
    fn leverage_names_its_lender() {
        // A fund that holds more than it raised has borrowed from somebody named, and the loan is a
        // liability that reduces the NAV.
        let levered = Fund {
            borrowed: vec![(party(60), 2_000.0)],
            ..fund()
        };
        assert!(levered.nav().unwrap() < fund().nav().unwrap());
        assert_eq!(levered.borrowed[0].0, party(60));
    }

    #[test]
    fn a_vehicle_with_no_share_count_cannot_be_redeemed_from() {
        // An investor holding a scalar cannot ask for its money back, and a vehicle like that is
        // outside this system whatever it is called.
        assert!(is_redeemable(Some(100.0)));
        assert!(!is_redeemable(Some(0.0)));
        assert!(!is_redeemable(None));
    }

    #[test]
    fn redeeming_in_kind_means_this_vehicle_is_not_a_forced_seller() {
        // And a world where the largest complex redeems only in kind has no fund-driven forced
        // selling at all — some other vehicle must carry it.
        assert_ne!(Redeems::InKind, Redeems::InCash);
    }

    #[test]
    fn the_share_count_reconciles_or_the_discrepancy_is_reported() {
        // Created minus redeemed equals outstanding.
        assert!(shares_reconcile(1_400.0, 400.0, 1_000.0, 3).is_none());
        assert_eq!(shares_reconcile(1_400.0, 400.0, 900.0, 3), Some(100.0));
    }

    #[test]
    fn a_pool_is_run_by_whoever_holds_a_live_mandate_over_it_and_by_nothing_else() {
        // Which pools survived a manager's death was decided by which side of the row the dead party
        // was on.
        assert_eq!(run_as(1), Run::Mandated);
        assert_eq!(run_as(3), Run::Mandated);
        assert_eq!(run_as(0), Run::Orphaned);
    }

    #[test]
    fn a_winding_pool_sells_what_it_holds_and_says_nothing_about_the_price() {
        // The selling is the fund's own, into the books it bought in.
        assert_eq!(winding_sale(240.0), Some(240.0));
        // A pool holding nothing has nothing to sell, which is not a sale of nothing.
        assert!(winding_sale(0.0).is_none());
    }

    #[test]
    fn what_each_holder_gets_is_its_share_of_what_the_pool_actually_raised() {
        // Not a promise of NAV.
        let raised = 900.0;
        let a = pro_rata(raised, 300.0, 1_000.0).unwrap();
        let b = pro_rata(raised, 700.0, 1_000.0).unwrap();
        assert_eq!(a, 270.0);
        assert_eq!(b, 630.0);
        // Every piece of it has a holder — what goes out is what came in.
        assert!((a + b - raised).abs() <= crate::num::dust(3, &[a, b, raised]));
    }

    #[test]
    fn a_redemption_gate_is_an_explicit_decision_that_opens_after_assets_are_sold() {
        assert!(redemption_gate(1.0));
        assert!(!redemption_gate(0.0));
    }

    #[test]
    fn a_pool_nobody_holds_pays_nobody_rather_than_paying_everything_to_nobody() {
        // Dividing by no shares is missing, not a payment.
        assert!(pro_rata(900.0, 0.0, 0.0).is_none());
    }

    #[test]
    fn the_wind_up_takes_as_long_as_the_selling_takes_and_ends_when_there_is_nothing_left() {
        // Nothing ends it on a schedule.
        assert!(!is_wound_up(240.0, 1_000.0));
        assert!(!is_wound_up(0.0, 1_000.0));
        assert!(!is_wound_up(240.0, 0.0));
        assert!(is_wound_up(0.0, 0.0));
    }
}
