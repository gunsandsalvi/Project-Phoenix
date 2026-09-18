//! A REDEEMABLE CLAIM: a pool whose liability is its shares, and whose equity is zero by
//! construction.
//!
//! @spec 13 A1–A4, B1–B4, C1, C1.a, C2, C2.a, C2.b · XI-2 · Law 3, Law 4, Law 6, Law 7, Appendix B
//!
//! **A3: its equity is zero by construction** — assets minus liabilities is zero, because the
//! liability IS the shares and the shares are a claim on the assets. That is not an invariant the
//! pool tries to hold; it is what a redeemable claim is, and `equity` is a READ that must come out
//! at zero within the dust of its own walk. A pool showing equity has a holder nobody named.
//!
//! **B1: NAV is (assets at market minus liabilities) over shares outstanding** — a read, taken at
//! CLEARED prices (B2). **B2.a: a stale price makes a stale NAV and somebody transacts on it**, and
//! that is a real transfer between the holder who came in and the ones already there. It is not a
//! defect to be smoothed away; smoothing it is what deletes the transfer.
//!
//! **C2.a, C2.b, XI-2: a redemption is met from the buffer OR BY SELLING**, and selling is a trade
//! into a market that must clear at whatever it clears. **A redemption rationed by the fund's cash,
//! with the unfilled part dropped, is not a redemption** (XI-2's second door) — so `meet` returns
//! what must be sold rather than quietly paying less.

use crate::ids::PartyId;

/// What the pool has and owes, at the prints it can see. Everything here is a READ of the register
/// and the price store — the pool stores no copy of its own assets (Law 19).
#[derive(Clone, Copy, Debug)]
pub struct Book {
    /// B2: marked at CLEARED prices. A price nobody cleared is not a mark.
    pub assets_at_market: f64,
    /// What it owes that is not its shares: fees accrued, a borrowing, a trade not yet settled.
    pub liabilities: f64,
    /// A2: counted in SHARES, which is what its liability is denominated in.
    pub shares: f64,
}

impl Book {
    /// B1: **NAV is a read, not a stored level.** Missing where there are no shares: a pool with
    /// none has no per-share value, and answering zero would be a number nobody could act on.
    pub fn nav(&self) -> Option<f64> {
        if self.shares <= 0.0 {
            return None;
        }
        Some((self.assets_at_market - self.liabilities) / self.shares)
    }

    /// A3: **equity is zero BY CONSTRUCTION** — assets minus liabilities minus what the shares are
    /// worth. It is returned rather than asserted so a family can report it with a size (Audit A2),
    /// because an audit that threw here would be repairing by stopping the world.
    pub fn equity(&self) -> f64 {
        match self.nav() {
            None => self.assets_at_market - self.liabilities,
            Some(nav) => self.assets_at_market - self.liabilities - nav * self.shares,
        }
    }

    /// Law 7: the dust of THAT walk, from its own terms and magnitudes. Never a percentage band.
    pub fn dust(&self) -> f64 {
        3.0 * f64::EPSILON
            * (self.assets_at_market.abs() + self.liabilities.abs() + self.shares.abs())
    }
}

/// C1: a subscription gives the fund cash and the holder new shares AT NAV. C1.a: and the fund must
/// then buy something with the cash, per its mandate — cash that sits is a mandate not being kept.
#[derive(Clone, Copy, Debug)]
pub struct Subscription {
    pub holder: PartyId,
    pub cash: f64,
    /// What it got, at the NAV that stood when it came in. B2.a: if that NAV was stale, this is
    /// where the transfer happened.
    pub shares: f64,
}

/// C1: shares issued at NAV. Missing where there is no NAV to issue at — a pool with no shares yet
/// is a pool whose first subscription sets the price by agreement, not by division.
pub fn subscribe(cash: f64, nav: Option<f64>) -> Option<f64> {
    assert!(cash > 0.0, "13 C1: a subscription of {cash} is not a subscription");
    match nav {
        Some(nav) if nav > 0.0 => Some(cash / nav),
        _ => None,
    }
}

/// C2, C2.a: what a redemption needs and where it comes from. **The whole of it is met** — from the
/// buffer, and by selling for the rest. XI-2: a redemption rationed by the fund's cash with the
/// unfilled part dropped is not a redemption.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Meeting {
    /// What the holder is owed at NAV.
    pub owed: f64,
    /// Taken from what the pool already holds.
    pub from_buffer: f64,
    /// C2.b: **and this is the forced sale.** A trade into a market that must clear, at whatever it
    /// clears — not a valuation, and not a number the pool chooses.
    pub must_sell: f64,
}

/// C2: takes shares back and pays cash at NAV, finding the cash from the buffer or by selling.
///
/// Law 6: the buffer is not "as much as it can" — what it does not cover is still owed and is sold
/// for. Nothing is clamped, and the holder is not paid less because the pool was illiquid.
pub fn meet(shares: f64, nav: f64, buffer: f64) -> Meeting {
    assert!(shares > 0.0, "13 C2: a redemption of {shares} shares is not a redemption");
    let owed = shares * nav;
    let from_buffer = if buffer >= owed { owed } else { buffer };
    Meeting { owed, from_buffer, must_sell: owed - from_buffer }
}

/// B4: **the sum of holders' share value equals assets minus liabilities.** A VERIFY, so it returns
/// the gap with its dust rather than enforcing anything — a family reports it, and never repairs it.
pub fn holders_against_the_book(book: &Book, holders: &[(PartyId, f64)]) -> (f64, f64) {
    let held: f64 = holders.iter().map(|(_, s)| *s).sum();
    let value = match book.nav() {
        None => 0.0,
        Some(nav) => nav * held,
    };
    (value - (book.assets_at_market - book.liabilities), book.dust())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn book(assets: f64, liabilities: f64, shares: f64) -> Book {
        Book { assets_at_market: assets, liabilities, shares }
    }

    #[test]
    fn equity_is_zero_by_construction_because_the_liability_is_the_shares() {
        let b = book(10_000.0, 250.0, 500.0);
        assert_eq!(b.nav(), Some(19.5));
        // A3: not an invariant the pool tries to hold — it is what a redeemable claim IS.
        assert!(b.equity().abs() <= b.dust(), "equity {} against dust {}", b.equity(), b.dust());
    }

    #[test]
    fn nav_is_missing_where_there_are_no_shares_and_never_zero() {
        // A pool with no shares has no per-share value. Answering zero would be a number nobody
        // could act on, and somebody would divide by it (Appendix A).
        assert!(book(0.0, 0.0, 0.0).nav().is_none());
        assert!(subscribe(1_000.0, None).is_none());
    }

    #[test]
    fn a_stale_nav_is_a_real_transfer_and_not_something_to_smooth() {
        // B2.a: the assets are worth 12,000 but the pool can only see prints worth 10,000. A holder
        // subscribing transacts on the stale NAV, and the difference goes to the holders already
        // there. Smoothing it is what deletes the transfer.
        let stale = book(10_000.0, 0.0, 500.0);
        let got = subscribe(2_000.0, stale.nav()).unwrap();
        assert_eq!(got, 100.0, "it bought at 20 a share");
        let fresh = book(12_000.0, 0.0, 500.0);
        let would_have = subscribe(2_000.0, fresh.nav()).unwrap();
        assert!(got > would_have, "the stale print bought more shares than the true one would");
    }

    #[test]
    fn a_redemption_is_met_in_full_and_what_the_buffer_misses_is_sold_for() {
        // C2.a, C2.b, XI-2: the buffer covers part; the rest is a FORCED SALE into a market that
        // must clear. A redemption rationed by the fund's cash is not a redemption.
        let m = meet(100.0, 20.0, 500.0);
        assert_eq!(m.owed, 2_000.0);
        assert_eq!(m.from_buffer, 500.0);
        assert_eq!(m.must_sell, 1_500.0);
        // Law 6: the holder is not paid less because the pool was illiquid.
        assert_eq!(m.from_buffer + m.must_sell, m.owed);
        // And a pool with cash to spare sells nothing.
        assert_eq!(meet(10.0, 20.0, 5_000.0).must_sell, 0.0);
    }

    #[test]
    fn the_holders_share_value_is_checked_against_the_book_and_never_enforced() {
        let b = book(10_000.0, 250.0, 500.0);
        let holders = [(PartyId::at(1), 300.0), (PartyId::at(2), 200.0)];
        let (gap, dust) = holders_against_the_book(&b, &holders);
        // B4 is a VERIFY: it returns a size a family can report with an owner, and repairs nothing.
        assert!(gap.abs() <= dust, "gap {gap} against dust {dust}");
        // A register that says holders hold fewer shares than the pool issued is a real finding,
        // and it is REPORTED rather than corrected.
        let short = [(PartyId::at(1), 300.0)];
        let (missing, _) = holders_against_the_book(&b, &short);
        assert!(missing < 0.0, "200 shares are unaccounted for and the read says so");
    }

    #[test]
    #[should_panic(expected = "is not a redemption")]
    fn a_redemption_of_no_shares_is_not_a_redemption() {
        meet(0.0, 20.0, 500.0);
    }
}
