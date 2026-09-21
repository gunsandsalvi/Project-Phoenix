//! A REDEEMABLE CLAIM: a pool whose liability is its shares, and whose equity is zero by
//! construction.
//!
//! @spec 13 A1–A4, B1–B4, C1, C1.a, C2, C2.a, C2.b · XI-2 · Law 3, Law 4, Law 6, Law 7, Appendix B

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};

/// What the pool has and owes, at the prints it can see.
#[derive(Clone, Copy, Debug)]
pub struct Book {
    /// Marked at CLEARED prices.
    pub assets_at_market: f64,
    /// What it owes that is not its shares: fees accrued, a borrowing, a trade not yet settled.
    pub liabilities: f64,
    /// Counted in SHARES, which is what its liability is denominated in.
    pub shares: f64,
}

impl Book {
    /// NAV is a read, not a stored level.
    pub fn nav(&self) -> Option<f64> {
        if self.shares <= 0.0 {
            return None;
        }
        Some((self.assets_at_market - self.liabilities) / self.shares)
    }

    /// Equity is zero BY CONSTRUCTION — assets minus liabilities minus what the shares are worth.
    pub fn equity(&self) -> f64 {
        match self.nav() {
            None => self.assets_at_market - self.liabilities,
            Some(nav) => self.assets_at_market - self.liabilities - nav * self.shares,
        }
    }

    /// The dust of THAT walk, from its own terms and magnitudes.
    pub fn dust(&self) -> f64 {
        3.0 * f64::EPSILON
            * (self.assets_at_market.abs() + self.liabilities.abs() + self.shares.abs())
    }
}

/// A subscription gives the fund cash and the holder new shares AT NAV.
#[derive(Clone, Copy, Debug)]
pub struct Subscription {
    pub holder: PartyId,
    pub cash: f64,
    /// What it got, at the NAV that stood when it came in.
    pub shares: f64,
}

/// Shares issued at NAV.
pub fn subscribe(cash: f64, nav: Option<f64>) -> Option<f64> {
    assert!(cash > 0.0, "13 C1: a subscription of {cash} is not a subscription");
    match nav {
        Some(nav) if nav > 0.0 => Some(cash / nav),
        _ => None,
    }
}

/// What a redemption needs and where it comes from.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Meeting {
    /// What the holder is owed at NAV.
    pub owed: f64,
    /// Taken from what the pool already holds.
    pub from_buffer: f64,
    /// And this is the forced sale.
    pub must_sell: f64,
}

/// Takes shares back and pays cash at NAV, finding the cash from the buffer or by selling.
pub fn meet(shares: f64, nav: f64, buffer: f64) -> Meeting {
    assert!(shares > 0.0, "13 C2: a redemption of {shares} shares is not a redemption");
    let owed = shares * nav;
    let from_buffer = if buffer >= owed { owed } else { buffer };
    Meeting { owed, from_buffer, must_sell: owed - from_buffer }
}

/// The sum of holders' share value equals assets minus liabilities.
pub fn holders_against_the_book(book: &Book, holders: &[(PartyId, f64)]) -> (f64, f64) {
    let held: f64 = holders.iter().map(|(_, s)| *s).sum();
    let value = match book.nav() {
        None => 0.0,
        Some(nav) => nav * held,
    };
    (value - (book.assets_at_market - book.liabilities), book.dust())
}


/// A POOL PUBLISHES ITS NAV, AND A HOLDER SUBSCRIBES AT IT.
pub struct Subscribing {
    pub kind: u32,
    /// How much of its spare money a holder will put into one pool.
    pub commits: &'static str,
}

impl Mechanism for Subscribing {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let commits = ctx.params().ratio(self.commits);

        let mut navs: Vec<(PartyId, f64, f64)> = Vec::new();
        for &pool in ctx.parties().of_kind(kinds::FUND) {
            let who = PartyId(pool);
            if !ctx.parties().alive(who) {
                continue;
            }
            // At cleared prices, and there is one read of that in the engine.
            let Some(at_market) =
                crate::instruments::market_book_value(who, ctx.register(), ctx.instruments(), ctx.prints(), ctx.period())
            else {
                continue;
            };
            // Its shares are settled ownership in the register, never an agreement written before
            // the subscriber's cash instruction has succeeded.
            let share_line = (0..ctx.instruments().len())
                .map(|row| InstrumentId::at(row as u32))
                .find(|line| {
                    ctx.instruments().issuer_of(*line) == who
                        && ctx.instruments().class_of(*line) == crate::instruments::Class::Share
                });
            let shares = match share_line {
                Some(line) => ctx.register().held_total(line).0,
                None => 0.0,
            };
            let mut owed = 0.0;
            let due: f64 = ctx
                .schedules()
                .of_payer(who)
                .iter()
                .map(|r| crate::stores::DueId(*r))
                .filter(|d| !ctx.schedules().paid(*d))
                .map(|d| ctx.schedules().amount(d))
                .sum();
            owed += due;
            let book = Book {
                assets_at_market: at_market,
                liabilities: owed,
                shares,
            };
            // `None` where there are no shares.
            let nav = match book.nav() {
                Some(nav) => nav,
                None if at_market > 0.0 => at_market,
                None => continue,
            };
            if nav <= 0.0 {
                continue;
            }
            navs.push((who, nav, shares));
        }

        let mut subscribing: Vec<(PartyId, PartyId, f64, f64)> = Vec::new();
        for (pool, nav, _) in &navs {
            // The NAV is published.
            ctx.say(self.kind, &[pool.0], &[(0, Value::Num(*nav))], true);
        }
        for (pool, nav, _) in &navs {
            for &holder in ctx.parties().of_kind(kinds::INSURER) {
                let holder = PartyId(holder);
                if !ctx.parties().alive(holder) {
                    continue;
                }
                let Some(money) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
                let cash = ctx.register().quantity(ctx.register().row(holder, money));
                // It subscribes with cash it has.
                let Some(shares) = subscribe(cash * commits, Some(*nav))
                else {
                    continue;
                };
                if shares <= 0.0 {
                    continue;
                }
                subscribing.push((*pool, holder, shares, shares * nav));
                break;
            }
        }

        for (pool, holder, shares, paid) in subscribing {
            // Cash one way and shares the other, in the same pass.
            let Some(from) = account_of(ctx.parties(), ctx.instruments(), holder) else { continue };
            let Some(line) = (0..ctx.instruments().len())
                .map(|row| InstrumentId::at(row as u32))
                .find(|line| {
                    ctx.instruments().issuer_of(*line) == pool
                        && ctx.instruments().class_of(*line) == crate::instruments::Class::Share
                })
            else { continue };
            let Some(paid) = crate::ledger::Units::new(paid) else { continue };
            let Some(units) = crate::ledger::Units::new(shares) else { continue };
            ctx.propose(
                vec![
                    crate::ledger::Leg::Money {
                        from: holder,
                        to: pool,
                        instrument: from,
                        amount: paid,
                        receipt: crate::ledger::Receipt::Transfer,
                    },
                    crate::ledger::Leg::Create {
                        party: holder,
                        instrument: line,
                        qty: units,
                        cost_per_unit: paid.get() / units.get(),
                    },
                ],
                crate::ledger::Cause::CorporateAction,
                crate::ledger::Delivery::Nothing,
                "13 C1: a subscription gives the pool cash and the holder shares at NAV",
            );
        }
    }
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
        // Not an invariant the pool tries to hold — it is what a redeemable claim IS.
        assert!(b.equity().abs() <= b.dust(), "equity {} against dust {}", b.equity(), b.dust());
    }

    #[test]
    fn nav_is_missing_where_there_are_no_shares_and_never_zero() {
        // A pool with no shares has no per-share value.
        assert!(book(0.0, 0.0, 0.0).nav().is_none());
        assert!(subscribe(1_000.0, None).is_none());
    }

    #[test]
    fn a_stale_nav_is_a_real_transfer_and_not_something_to_smooth() {
        // The assets are worth 12,000 but the pool can only see prints worth 10,000.
        let stale = book(10_000.0, 0.0, 500.0);
        let got = subscribe(2_000.0, stale.nav()).unwrap();
        assert_eq!(got, 100.0, "it bought at 20 a share");
        let fresh = book(12_000.0, 0.0, 500.0);
        let would_have = subscribe(2_000.0, fresh.nav()).unwrap();
        assert!(got > would_have, "the stale print bought more shares than the true one would");
    }

    #[test]
    fn a_redemption_is_met_in_full_and_what_the_buffer_misses_is_sold_for() {
        // The buffer covers part; the rest is a FORCED SALE into a market that must clear.
        let m = meet(100.0, 20.0, 500.0);
        assert_eq!(m.owed, 2_000.0);
        assert_eq!(m.from_buffer, 500.0);
        assert_eq!(m.must_sell, 1_500.0);
        // The holder is not paid less because the pool was illiquid.
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
        // A register that says holders hold fewer shares than the pool issued is a real finding, and
        // it is REPORTED rather than corrected.
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
