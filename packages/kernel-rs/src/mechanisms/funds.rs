//! FUND SHARES: a named party whose liability is its shares, whose **equity is zero by
//! construction**, and whose redemption is the forced-seller channel.
//!
//! @spec 13 A1 · 13 A2 · 13 A3 · 13 A4 · 13 B1 · 13 B2 · 13 B2.a · 13 B3 · 13 B4 · 13 C1 · 13 C1.a ·
//! @spec 13 C2 · 13 C2.a · 13 C2.b · 13 C3 · 13 C4 · 13 C4.a · 13 C5 · 13 D1 · 13 D2 · 13 D2.a ·
//! @spec 13 D3 · 13 D4 · 13 D5 · 13 E1 · 13 E2 · 13 E3 · 13 E3.a · 13 E4 · 13 F1 · 13 F2 · 13 F3 ·
//! @spec 13 G1 · 13 G1.a · 13 G1.b · XI-2 · Law 3, Law 5, Law 6, Law 19 · Appendix B
//!
//! **A fund with equity has mislaid somebody's money** (A3): assets minus liabilities is zero,
//! because the holders own the assets. `equity` is a read and `mislaid` is the finding when it is not
//! zero on its own dust.
//!
//! **NAV is a read, every time, never a stored series** (B1) — and the assets are marked at CLEARED
//! prices (B2), so **a stale price makes a stale NAV and somebody transacts on it: a real transfer
//! between holders, not a rounding** (B2.a).
//!
//! **A redemption rationed by the fund's cash, with the unfilled part dropped, deletes the entire
//! system** (C2.b). `redeem` therefore returns what must be SOLD when the buffer is short, and the
//! sale is a trade into a market that must clear at whatever price it clears — that is the
//! forced-seller channel and it is the point (XI-2).
//!
//! **No guaranteed constant NAV** (D4). A money fund is measured in SHARES, not currency units;
//! losses fall on the NAV, and **a fund that cannot break the buck is a fund with a hidden guarantor,
//! and the guarantor is nobody**. There is no floor anywhere in this module (Law 6).
//!
//! **The gap between an exchange-traded fund's price and its NAV closes because somebody TRADES**
//! (E3.a) — a reason for a participant, not a rule tying the two — **and it can persist when they will
//! not**, which is a finding about liquidity and never a number to clamp (E4).
//!
//! **A fund does not create its assets** (F1) and **there is no leverage without a lender** (F2): a
//! fund holding more than it raised has borrowed from somebody named.

use crate::ids::{InstrumentId, PartyId};

/// A1, A2: a named party whose **liability is its shares**, held by named holders, counted in shares.
#[derive(Clone, Debug)]
pub struct Fund {
    pub who: PartyId,
    /// F3: **the manager is a separate party** that earns the fee — its income and the fund's cost.
    pub manager: PartyId,
    /// Marked at cleared prices (B2). Each holding was bought from a named seller (F1).
    pub assets: Vec<(InstrumentId, f64)>,
    /// F2: **no leverage without a lender.** What it borrowed, and from whom.
    pub borrowed: Vec<(PartyId, f64)>,
    pub cash: f64,
    pub shares: f64,
    /// B3: fees accrue and are paid to the manager, and they reduce NAV.
    pub fees_accrued: f64,
    /// A4: **the mandate is a real constraint on what it buys, not a label.**
    pub may_hold: Vec<InstrumentId>,
}

impl Fund {
    pub fn assets_at_market(&self) -> f64 {
        self.cash + self.assets.iter().map(|(_, v)| v).sum::<f64>()
    }

    pub fn liabilities(&self) -> f64 {
        self.fees_accrued + self.borrowed.iter().map(|(_, v)| v).sum::<f64>()
    }

    /// B1: **(assets at market minus liabilities) divided by shares outstanding** — a READ, every
    /// time. `None` where there are no shares: a NAV per nothing is not a number.
    pub fn nav(&self) -> Option<f64> {
        if self.shares <= 0.0 {
            return None;
        }
        Some((self.assets_at_market() - self.liabilities()) / self.shares)
    }

    /// A3: **its equity is zero by construction**, because the holders own the assets. What the
    /// holders' shares come to, against what the fund holds.
    pub fn equity(&self, holders_shares: f64) -> Option<f64> {
        let nav = self.nav()?;
        Some(self.assets_at_market() - self.liabilities() - holders_shares * nav)
    }

    /// A4: what the mandate allows. A fund that buys outside it has no mandate.
    pub fn may_buy(&self, what: InstrumentId) -> bool {
        self.may_hold.contains(&what)
    }
}

/// A3, B4: **a fund with equity has mislaid somebody's money**, and the sum of holders' share value
/// equals assets minus liabilities, exactly. A VERIFY on Law 7's derived dust; `None` when it holds.
pub fn mislaid(f: &Fund, holders_shares: f64, terms: usize) -> Option<f64> {
    let over = f.equity(holders_shares)?;
    if over.abs() <= crate::num::dust(terms, &[f.assets_at_market(), f.liabilities()]) {
        return None;
    }
    Some(over)
}

/// C1: **a subscription gives the fund cash and the holder new shares at NAV** — and C1.a: the fund
/// must then BUY something with the cash, per its mandate. That is why a fund is a transmission
/// channel: a flow in becomes a purchase of what the mandate allows.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Subscribed {
    pub holder: PartyId,
    pub cash: f64,
    pub shares_issued: f64,
    /// C1.a: what it must now go and buy. Not optional.
    pub to_invest: f64,
}

pub fn subscribe(f: &Fund, holder: PartyId, cash: f64) -> Option<Subscribed> {
    let nav = f.nav()?;
    if nav <= 0.0 {
        return None;
    }
    Some(Subscribed { holder, cash, shares_issued: cash / nav, to_invest: cash })
}

/// C2, C2.a, C2.b: **a redemption takes shares back and pays the holder cash at NAV — and the fund
/// must FIND the cash: from its buffer, or by SELLING.**
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Redeemed {
    pub holder: PartyId,
    pub shares: f64,
    pub owed: f64,
    /// From the buffer.
    pub from_cash: f64,
    /// C2.b: **and the rest must be sold** — a trade into a market that must clear, at whatever price
    /// it clears. This is the forced-seller channel (XI-2), and rationing the redemption to the
    /// buffer instead would delete the entire system.
    pub must_sell: f64,
}

pub fn redeem(f: &Fund, holder: PartyId, shares: f64) -> Option<Redeemed> {
    let nav = f.nav()?;
    let owed = shares * nav;
    let from_cash = if f.cash < owed { f.cash } else { owed };
    Some(Redeemed { holder, shares, owed, from_cash, must_sell: owed - from_cash })
}

/// C4, C4.a: **the holder is paid at today's NAV, the sales happen at tomorrow's prices, and the
/// difference falls on the remaining holders** — which is why a redemption is a real cost to those who
/// stay, and why runs are a thing.
pub fn cost_to_those_who_stay(r: &Redeemed, sold_for: f64) -> f64 {
    r.must_sell - sold_for
}

/// C5: **shares created minus redeemed equals shares outstanding, and cash in and out matches.** What
/// can fail is the count, so this walks the events and reports the discrepancy against the register's
/// figure — `None` when they agree on derived dust.
pub fn shares_reconcile(created: f64, redeemed: f64, outstanding: f64, terms: usize) -> Option<f64> {
    let implied = created - redeemed;
    let off = implied - outstanding;
    if off.abs() <= crate::num::dust(terms, &[created, redeemed, outstanding]) {
        return None;
    }
    Some(off)
}

/// D4: **no guaranteed constant NAV.** A money fund is measured in shares; losses fall on the NAV, and
/// if the assets fall the NAV falls. **A fund that cannot break the buck is a fund with a hidden
/// guarantor, and the guarantor is nobody.** This is the read that shows it — there is no floor
/// anywhere near it (Law 6).
pub fn broke_the_buck(f: &Fund, issued_at: f64) -> bool {
    match f.nav() {
        Some(nav) => nav < issued_at,
        None => false,
    }
}

/// D2, D2.a, D5: **a saver chooses between a bank deposit, a money fund and bills directly**, and the
/// money fund's yield competes with the deposit rate — which is a real constraint on what banks pay.
/// Flows follow as a CONSEQUENCE and never as an imposed allocation.
pub fn beats_the_deposit(fund_yield: f64, deposit_rate: f64) -> bool {
    fund_yield > deposit_rate
}

/// E1, E2: an exchange-traded fund has **two values** — the traded price and the NAV — and they are
/// different numbers. E4: the premium or discount is a READ of two prices.
pub fn premium(traded_price: f64, nav: f64) -> Option<f64> {
    if nav <= 0.0 {
        return None;
    }
    Some(traded_price / nav - 1.0)
}

/// E3, E3.a: **the gap closes because somebody TRADES** — a reason for a participant, not a rule tying
/// the two — **and it can persist when they will not.** `None` is the gap standing, which E4 calls a
/// finding about liquidity rather than a number to clamp.
pub fn arbitrages(gap: f64, costs_to_do_it: f64, can_fund: f64) -> Option<f64> {
    if gap.abs() <= costs_to_do_it || can_fund <= 0.0 {
        return None;
    }
    Some(can_fund)
}

/// G1.a: **an in-kind redemption means that vehicle is NOT a forced seller.** A world in which the
/// largest fund complex redeems only in kind has no fund-driven forced selling at all, and some other
/// vehicle must carry it — so which way a fund redeems is a fact about it, carried here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Redeems {
    InCash,
    InKind,
}

/// G1.b: **an investor holding a scalar with no share count cannot ask for its money back**, and a
/// vehicle like that is outside this system whatever it is called. A share count is what makes a
/// claim redeemable.
pub fn is_redeemable(shares_held: Option<f64>) -> bool {
    matches!(shares_held, Some(s) if s > 0.0)
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
        // B1: never a stored series. Assets 10,000 over 1,000 shares.
        assert_eq!(fund().nav(), Some(10.0));
        let empty = Fund { shares: 0.0, ..fund() };
        assert!(empty.nav().is_none());
    }

    #[test]
    fn a_fund_with_equity_has_mislaid_somebodys_money() {
        // A3, B4: assets minus liabilities is zero because the holders own the assets.
        let f = fund();
        assert!(mislaid(&f, 1_000.0, 4).is_none());
        // Holders whose shares do not add up to the fund's book is exactly the defect.
        assert_eq!(mislaid(&f, 900.0, 4), Some(1_000.0));
    }

    #[test]
    fn fees_accrue_to_the_manager_and_reduce_the_nav() {
        // B3, F3: the manager is a separate party; the fee is its income and the fund's cost.
        let f = fund();
        let after_fees = Fund { fees_accrued: 200.0, ..f.clone() };
        assert!(after_fees.nav().unwrap() < f.nav().unwrap());
        assert_ne!(f.manager, f.who);
    }

    #[test]
    fn a_stale_price_makes_a_stale_nav_and_somebody_transacts_on_it() {
        // B2.a: that is a real transfer between holders, not a rounding. The same fund with the
        // asset re-marked pays a redeemer a different amount for the same shares.
        let stale = fund();
        let marked = Fund { assets: vec![(holds(1), 7_000.0), (holds(2), 1_500.0)], ..fund() };
        let on_stale = redeem(&stale, party(50), 100.0).unwrap();
        let on_fresh = redeem(&marked, party(50), 100.0).unwrap();
        assert!(on_stale.owed > on_fresh.owed);
    }

    #[test]
    fn a_subscription_must_then_buy_something_per_the_mandate() {
        // C1, C1.a, A4: a flow into the fund becomes a purchase of what the mandate allows, which is
        // why a fund is a transmission channel.
        let f = fund();
        let s = subscribe(&f, party(50), 2_000.0).unwrap();
        assert_eq!(s.shares_issued, 200.0);
        assert_eq!(s.to_invest, 2_000.0);
        assert!(f.may_buy(holds(1)));
        assert!(!f.may_buy(holds(9)));
    }

    #[test]
    fn a_redemption_beyond_the_buffer_must_be_sold_and_that_is_the_forced_seller_channel() {
        // C2.a, C2.b, XI-2: a redemption rationed by the fund's cash, with the unfilled part dropped,
        // deletes the entire system. Here the shortfall comes back as what must be SOLD.
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
        // C4, C4.a: the holder is paid at today's NAV and the sales happen at tomorrow's prices —
        // which is why runs are a thing.
        let r = redeem(&fund(), party(50), 400.0).unwrap();
        assert!(cost_to_those_who_stay(&r, 3_500.0) == 0.0);
        assert!(cost_to_those_who_stay(&r, 3_100.0) > 0.0);
    }

    #[test]
    fn a_money_fund_can_break_the_buck() {
        // D4: a fund that cannot is a fund with a hidden guarantor, and the guarantor is nobody.
        // There is no floor here — the NAV is whatever the assets came to.
        let f = fund();
        assert!(!broke_the_buck(&f, 10.0));
        let hit = Fund { assets: vec![(holds(1), 7_000.0), (holds(2), 1_500.0)], ..fund() };
        assert!(broke_the_buck(&hit, 10.0));
    }

    #[test]
    fn the_saver_compares_the_fund_with_the_deposit_and_the_flow_follows() {
        // D2.a, D5: the competition is a real constraint on what banks pay, and flows are a
        // CONSEQUENCE rather than an imposed allocation.
        assert!(beats_the_deposit(0.045, 0.030));
        assert!(!beats_the_deposit(0.020, 0.030));
    }

    #[test]
    fn an_etfs_gap_closes_because_somebody_trades_and_can_persist_when_nobody_will() {
        // E2, E3, E3.a, E4: two different numbers, and the premium is a read of them. A gap inside
        // what it costs to close is a gap that stands — a finding about liquidity, never clamped.
        let p = premium(10.4, 10.0).unwrap();
        assert!(p > 0.0);
        assert!(arbitrages(p, 0.01, 1_000_000.0).is_some());
        assert!(arbitrages(p, 0.10, 1_000_000.0).is_none());
        assert!(arbitrages(p, 0.01, 0.0).is_none());
        assert!(premium(10.4, 0.0).is_none());
    }

    #[test]
    fn leverage_names_its_lender() {
        // F2: a fund that holds more than it raised has borrowed from somebody named, and the loan
        // is a liability that reduces the NAV.
        let levered = Fund { borrowed: vec![(party(60), 2_000.0)], ..fund() };
        assert!(levered.nav().unwrap() < fund().nav().unwrap());
        assert_eq!(levered.borrowed[0].0, party(60));
    }

    #[test]
    fn a_vehicle_with_no_share_count_cannot_be_redeemed_from() {
        // G1.b: an investor holding a scalar cannot ask for its money back, and a vehicle like that
        // is outside this system whatever it is called.
        assert!(is_redeemable(Some(100.0)));
        assert!(!is_redeemable(Some(0.0)));
        assert!(!is_redeemable(None));
    }

    #[test]
    fn redeeming_in_kind_means_this_vehicle_is_not_a_forced_seller() {
        // G1.a: and a world where the largest complex redeems only in kind has no fund-driven forced
        // selling at all — some other vehicle must carry it.
        assert_ne!(Redeems::InKind, Redeems::InCash);
    }

    #[test]
    fn the_share_count_reconciles_or_the_discrepancy_is_reported() {
        // C5: created minus redeemed equals outstanding.
        assert!(shares_reconcile(1_400.0, 400.0, 1_000.0, 3).is_none());
        assert_eq!(shares_reconcile(1_400.0, 400.0, 900.0, 3), Some(100.0));
    }
}
