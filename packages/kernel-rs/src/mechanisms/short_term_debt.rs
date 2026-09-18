//! SHORT-TERM DEBT: issued at a discount, redeemed at par — **and it must be ROLLED**, which is the
//! whole risk.
//!
//! @spec 9 A1.a · 9 A1.b · 9 A1.c · 9 A2 · 9 A2.a · 9 A3 · 9 B1 · 9 B2 · 9 B3 · 9 B3.a · 9 B3.b ·
//! @spec 9 B4 · 9 B5 · 9 C1 · 9 C2 · 9 C2.a · 9 C3 · 9 C4 · 9 D1 · 9 D2 · 9 D3 · 9 D4 · 9 E1 · 9 E2 ·
//! @spec 9 E3 · XI-2 · Law 3, Law 5, Law 6, Law 8, Law 19
//!
//! **No automatic roll** (E1). Paper that always rolls at a written rate is not debt; it is a permanent
//! liability with a coupon, and it **removes the only risk the instrument has**. `roll` is a new issue
//! into a market that must clear (B3.a) — the issuer is asking the market to lend again, **and it may
//! not** — so it returns `Declined`, and then the issuer must repay maturing paper out of cash it does
//! not have (B3.b).
//!
//! **The backstop costs money in every period it is not used** (B4): a committed line with no
//! commitment fee on undrawn headroom is **a free option the lender did not sell**.
//!
//! **The yield is derived from price and days to maturity** (A2), on a **stated day-count and quoting
//! convention, because at this tenor the convention is a material part of the number** (A2.a, Law 8).
//! There is no function here that turns a yield into a price: E2's discount computed from a curve
//! nobody traded is Law 3's defect at the short end.
//!
//! **A buyer has a limit per issuer, and the limit is why a deteriorating issuer loses funding BEFORE
//! it loses solvency** (C3).

use crate::calendar::Day;
use crate::ids::PartyId;

/// A1.a, A1.b, A1.c: **no coupon — issued at a discount, redeemed at par, and the discount is the
/// whole return.** Under a year, senior unsecured, and A1.d: no early-termination regime, because it
/// is too short to be worth an option.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paper {
    pub issuer: PartyId,
    pub face: f64,
    /// A2: what it CLEARED at. The yield comes from this, never the reverse.
    pub price: f64,
    pub issued: Day,
    pub matures: Day,
}

/// A2.a: **a stated day-count and quoting convention, because at this tenor the convention is a
/// material part of the number** (Law 8). Two conventions on one number would be Law 4's defect.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Convention {
    /// Money-market: actual days over 360.
    Actual360,
    /// Bond-equivalent: actual days over 365.
    Actual365,
}

impl Convention {
    pub fn year(&self) -> f64 {
        match self {
            Convention::Actual360 => 360.0,
            Convention::Actual365 => 365.0,
        }
    }
}

impl Paper {
    pub fn days(&self) -> i64 {
        self.matures.0 - self.issued.0
    }

    /// A2: **the yield is DERIVED from price and days to maturity** — this direction only. `None` on a
    /// price of nothing or a tenor of no days: a yield over no time is not a rate.
    pub fn yield_on(&self, c: Convention) -> Option<f64> {
        let days = self.days();
        if self.price <= 0.0 || days <= 0 {
            return None;
        }
        Some((self.face / self.price - 1.0) * c.year() / days as f64)
    }
}

/// A3: **there are types by issuer — the state, a bank, a firm — AND THE TYPE IS THE CREDIT.** It is
/// the issuer's own standing, carried on the paper's buyer-side view, not a class the mechanism
/// branches on (Law 15).
#[derive(Clone, Copy, Debug)]
pub struct Limit {
    pub buyer: PartyId,
    pub on_issuer: PartyId,
    /// C3: **the limit is why a deteriorating issuer loses funding before it loses solvency.**
    pub most: f64,
    pub already_holding: f64,
}

impl Limit {
    pub fn room(&self) -> f64 {
        self.most - self.already_holding
    }
}

/// B3, B3.a, B3.b, E1: **a rollover is a new issue into a market that must clear.** The issuer is
/// asking the market to lend again **and it may not** — so there is no automatic roll here, and a
/// decline is what a run is made of.
#[derive(Clone, Debug, PartialEq)]
pub enum Rolled {
    /// The buyers took it, at what they would pay.
    Done { raised: f64, at_price: f64, from: Vec<(PartyId, f64)> },
    /// B3.b: **buyers decline, and the issuer must repay maturing paper out of cash it does not
    /// have** — it must find the money somewhere (B4's backstop, or a sale, XI-2).
    Declined { short_by: f64 },
}

pub fn roll(maturing: f64, buyers: &[(Limit, f64)], face_per_unit: f64) -> Rolled {
    let mut raised = 0.0;
    let mut at_price = 0.0;
    let mut from = Vec::new();
    // Buyers in order of the price they will pay, best first.
    let mut ordered: Vec<&(Limit, f64)> = buyers.iter().collect();
    ordered.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (limit, price) in ordered {
        if raised >= maturing {
            break;
        }
        let room_in_money = limit.room() * price / face_per_unit;
        if room_in_money <= 0.0 {
            continue;
        }
        let wanted = maturing - raised;
        let taken = if room_in_money < wanted { room_in_money } else { wanted };
        from.push((limit.buyer, taken));
        at_price = *price;
        raised += taken;
    }
    if raised < maturing {
        return Rolled::Declined { short_by: maturing - raised };
    }
    Rolled::Done { raised, at_price, from }
}

/// B4: **the issuer keeps a backstop — a committed bank line, a liquid buffer — and the backstop costs
/// money in every period it is not used.** The line itself is `stores::Commitment`, which §7 C9 names
/// a facility and this clause names a backstop: one object, one representation (Law 4, 21.71).
/// B5: **the maturity profile of outstanding paper is a read, and a concentrated profile is a
/// foreseeable wall.** A walk over the rows (Law 19) — never a stated schedule.
pub fn wall(outstanding: &[Paper], within: Day) -> f64 {
    outstanding
        .iter()
        .filter(|p| p.matures <= within)
        .map(|p| p.face)
        .sum()
}

/// C2, C2.a: **the buyer's reasons are yield against the alternatives — a deposit, a repo, a central
/// bank facility — and credit and liquidity**, which makes this paper a real substitute for a deposit
/// and therefore one of the channels a policy rate travels down.
///
/// C4: when the policy rate moves the bill yield moves **because the buyers' alternative moved**, not
/// because a rule ties them — so this answers whether THIS buyer prefers the paper, and the market
/// yield is whatever their schedules clear at.
pub fn prefers_paper(paper_yield: f64, deposit_rate: f64, facility_rate: f64, for_the_credit: f64) -> bool {
    let best_alternative = if deposit_rate > facility_rate { deposit_rate } else { facility_rate };
    paper_yield - for_the_credit > best_alternative
}

/// D4: **a spread over the equivalent-tenor bill is a derived READ of two cleared prices, never a
/// stored number.** Both must have printed.
pub fn spread_over_bill(paper: &Paper, bill: &Paper, c: Convention) -> Option<f64> {
    Some(paper.yield_on(c)? - bill.yield_on(c)?)
}

/// D3: **it is collateral, with a haircut, which is a large part of why anyone holds it.** The haircut
/// reads the issuer's own credit, as everywhere else in this world.
pub fn lends_against(p: &Paper, on_that_issuers_credit: f64) -> f64 {
    assert!(on_that_issuers_credit > 0.0, "9 D3: a haircut with no view of the issuer is one per type");
    p.price / on_that_issuers_credit
}

/// E3: **no negative outstanding, and no maturity that passes without cash moving.** The redemption is
/// par, from the issuer to the holder, and it is refused rather than netted if the paper is not there.
pub fn redeem(p: &Paper, held: f64, holder: PartyId) -> Option<(PartyId, PartyId, f64)> {
    if held <= 0.0 || held > p.face {
        return None;
    }
    Some((p.issuer, holder, held))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::Commitment;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn bill(price: f64, days: i64) -> Paper {
        Paper { issuer: party(9), face: 100.0, price, issued: Day(0), matures: Day(days) }
    }

    fn limit(buyer: u32, most: f64, holding: f64) -> Limit {
        Limit { buyer: party(buyer), on_issuer: party(9), most, already_holding: holding }
    }

    #[test]
    fn the_discount_is_the_whole_return_and_the_yield_comes_from_the_price() {
        // A1.a, A2: never the reverse — E2's discount computed from a curve nobody traded is Law 3's
        // defect at the short end, and there is no function here that does it.
        let cheap = bill(98.0, 90);
        let dear = bill(99.5, 90);
        assert!(cheap.yield_on(Convention::Actual360).unwrap() > dear.yield_on(Convention::Actual360).unwrap());
    }

    #[test]
    fn the_convention_is_a_material_part_of_the_number_at_this_tenor() {
        // A2.a, Law 8.
        let b = bill(98.0, 90);
        let money_market = b.yield_on(Convention::Actual360).unwrap();
        let bond_equivalent = b.yield_on(Convention::Actual365).unwrap();
        assert!(bond_equivalent > money_market);
    }

    #[test]
    fn a_yield_over_no_time_or_no_price_is_not_a_rate() {
        assert!(bill(98.0, 0).yield_on(Convention::Actual360).is_none());
        assert!(bill(0.0, 90).yield_on(Convention::Actual360).is_none());
    }

    #[test]
    fn there_is_no_automatic_roll_and_buyers_can_decline() {
        // B3, B3.a, E1: paper that always rolls at a written rate is not debt — it is a permanent
        // liability with a coupon, and it removes the only risk the instrument has.
        let willing = [(limit(20, 800.0, 0.0), 99.0), (limit(21, 400.0, 0.0), 98.5)];
        match roll(1_000.0, &willing, 100.0) {
            Rolled::Done { raised, at_price, from } => {
                assert_eq!(raised, 1_000.0);
                assert!(at_price <= 99.0);
                assert_eq!(from[0].0, party(20));
            }
            other => panic!("expected a completed roll, got {other:?}"),
        }
        // B3.b: the buyers are at their limits, and the issuer must find the money somewhere.
        let full = [(limit(20, 800.0, 800.0), 99.0), (limit(21, 400.0, 400.0), 98.5)];
        assert_eq!(roll(1_000.0, &full, 100.0), Rolled::Declined { short_by: 1_000.0 });
    }

    #[test]
    fn a_partial_roll_leaves_the_issuer_short_by_the_difference() {
        // B3.b: this is what a run is made of — and the shortfall is a real number it must find.
        let thin = [(limit(20, 300.0, 0.0), 99.0)];
        assert_eq!(roll(1_000.0, &thin, 100.0), Rolled::Declined { short_by: 703.0 });
    }

    #[test]
    fn a_limit_per_issuer_is_why_funding_is_lost_before_solvency_is() {
        // C3: the buyer stops before the issuer fails.
        let l = limit(20, 800.0, 750.0);
        assert_eq!(l.room(), 50.0);
        let at_the_limit = limit(20, 800.0, 800.0);
        assert_eq!(at_the_limit.room(), 0.0);
    }

    #[test]
    fn the_backstop_costs_money_in_every_period_it_is_not_used() {
        // B4: a committed line with no commitment fee on undrawn headroom is a free option the lender
        // did not sell.
        // 21.71: and it is the SAME object §7 C9 calls a facility, so it is read from one place.
        let b = Commitment {
            lender: party(70),
            borrower: party(9),
            limit: 1_000.0,
            drawn: 200.0,
            margin: 0.02,
            fee_on_undrawn: 0.005,
            until: None,
        };
        let (lender, fee) = b.costs();
        assert_eq!(lender, party(70));
        assert_eq!(fee, 4.0);
        // A backstop STANDS — `until` is Missing — and it is still standing whenever it is asked.
        assert!(b.live_on(Day(9_000)));
    }

    #[test]
    #[should_panic(expected = "free option the lender did not sell")]
    fn a_committed_line_with_no_fee_is_refused() {
        let free = Commitment {
            lender: party(70),
            borrower: party(9),
            limit: 1_000.0,
            drawn: 0.0,
            margin: 0.02,
            fee_on_undrawn: 0.0,
            until: None,
        };
        free.costs();
    }

    #[test]
    fn the_maturity_profile_is_a_read_and_a_concentrated_one_is_a_wall() {
        // B5, Law 19.
        let outstanding = [
            Paper { face: 500.0, matures: Day(30), ..bill(99.0, 30) },
            Paper { face: 700.0, matures: Day(35), ..bill(99.0, 35) },
            Paper { face: 300.0, matures: Day(200), ..bill(97.0, 200) },
        ];
        assert_eq!(wall(&outstanding, Day(40)), 1_200.0);
        assert_eq!(wall(&outstanding, Day(10)), 0.0);
    }

    #[test]
    fn the_buyer_compares_the_paper_with_its_alternatives() {
        // C2, C2.a, C4: the bill yield moves when the policy rate does BECAUSE the buyers' alternative
        // moved — not because a rule ties them.
        assert!(prefers_paper(0.050, 0.030, 0.035, 0.002));
        // Raise the facility rate and the same paper stops being worth holding.
        assert!(!prefers_paper(0.050, 0.030, 0.060, 0.002));
        // And a credit it doubts has to pay more for the same decision.
        assert!(!prefers_paper(0.050, 0.030, 0.035, 0.030));
    }

    #[test]
    fn the_spread_over_the_bill_is_a_read_of_two_cleared_prices() {
        // D4: never a stored number, and both must have printed.
        let corporate = bill(97.5, 90);
        let govt = Paper { issuer: party(1), ..bill(99.2, 90) };
        assert!(spread_over_bill(&corporate, &govt, Convention::Actual360).unwrap() > 0.0);
        let unpriced = bill(0.0, 90);
        assert!(spread_over_bill(&unpriced, &govt, Convention::Actual360).is_none());
    }

    #[test]
    fn it_is_collateral_at_a_haircut_that_reads_the_issuers_credit() {
        // D3.
        assert!(lends_against(&bill(98.0, 90), 1.01) > lends_against(&bill(98.0, 90), 1.20));
    }

    #[test]
    fn no_maturity_passes_without_cash_moving() {
        // E3: and there is no negative outstanding to redeem.
        let p = bill(98.0, 90);
        assert_eq!(redeem(&p, 100.0, party(20)), Some((party(9), party(20), 100.0)));
        assert!(redeem(&p, 0.0, party(20)).is_none());
        assert!(redeem(&p, 140.0, party(20)).is_none());
    }
}
