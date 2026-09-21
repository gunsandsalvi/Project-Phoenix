//! SHORT-TERM DEBT: issued at a discount, redeemed at par — and it must be ROLLED, which is the
//! whole risk.
//!
//! @spec 9 A1.a · 9 A1.b · 9 A1.c · 9 A2 · 9 A2.a · 9 A3 · 9 B1 · 9 B2 · 9 B3 · 9 B3.a · 9 B3.b ·
//! @spec 9 B4 · 9 B5 · 9 C1 · 9 C2 · 9 C2.a · 9 C3 · 9 C4 · 9 D1 · 9 D2 · 9 D3 · 9 D4 · 9 E1 · 9 E2 ·
//! @spec 9 E3 · XI-2 · Law 3, Law 5, Law 6, Law 8, Law 19

use crate::calendar::{Convention, Week};
use crate::ids::{CurrencyCode, PartyId};
use crate::instruments::{Class, PaymentFrequency};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};

/// No coupon — issued at a discount, redeemed at par, and the discount is the whole return.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Paper {
    pub issuer: PartyId,
    pub face: f64,
    /// What it CLEARED at.
    pub price: f64,
    pub issued: Week,
    pub matures: Week,
}

impl Paper {
    pub fn days(&self) -> i64 {
        self.matures.0 - self.issued.0
    }

    /// The yield is DERIVED from price and days to maturity — this direction only.
    pub fn yield_on(&self, c: Convention) -> Option<f64> {
        crate::instruments::yield_to(self.price, self.face, self.issued, self.matures, c)
    }
}

/// There are types by issuer — the state, a bank, a firm — AND THE TYPE IS THE CREDIT.
#[derive(Clone, Copy, Debug)]
pub struct Limit {
    pub buyer: PartyId,
    pub on_issuer: PartyId,
    /// The limit is why a deteriorating issuer loses funding before it loses solvency.
    pub most: f64,
    pub already_holding: f64,
}

impl Limit {
    pub fn room(&self) -> f64 {
        self.most - self.already_holding
    }
}

/// A rollover is a new issue into a market that must clear.
#[derive(Clone, Debug, PartialEq)]
pub enum Rolled {
    /// The buyers took it, at what they would pay.
    Done {
        raised: f64,
        at_price: f64,
        from: Vec<(PartyId, f64)>,
    },
    /// Buyers decline, and the issuer must repay maturing paper out of cash it does not have — it
    /// must find the money somewhere (B4's backstop, or a sale, XI-2).
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
        let taken = if room_in_money < wanted {
            room_in_money
        } else {
            wanted
        };
        from.push((limit.buyer, taken));
        at_price = *price;
        raised += taken;
    }
    if raised < maturing {
        return Rolled::Declined {
            short_by: maturing - raised,
        };
    }
    Rolled::Done {
        raised,
        at_price,
        from,
    }
}

/// The issuer keeps a backstop — a committed bank line, a liquid buffer — and the backstop costs
/// money in every week it is not used.
pub fn wall(outstanding: &[Paper], within: Week) -> f64 {
    outstanding
        .iter()
        .filter(|p| p.matures <= within)
        .map(|p| p.face)
        .sum()
}

/// The buyer's reasons are yield against the alternatives — a deposit, a repo, a central bank
/// facility — and credit and liquidity, which makes this paper a real substitute for a deposit and
/// therefore one of the channels a policy rate travels down.
pub fn prefers_paper(
    paper_yield: f64,
    deposit_rate: f64,
    facility_rate: f64,
    for_the_credit: f64,
) -> bool {
    let best_alternative = if deposit_rate > facility_rate {
        deposit_rate
    } else {
        facility_rate
    };
    paper_yield - for_the_credit > best_alternative
}

/// A spread over the equivalent-tenor bill is a derived READ of two cleared prices, never a stored
/// number.
pub fn spread_over_bill(paper: &Paper, bill: &Paper, c: Convention) -> Option<f64> {
    Some(paper.yield_on(c)? - bill.yield_on(c)?)
}

/// It is collateral, with a haircut, which is a large part of why anyone holds it.
pub fn lends_against(p: &Paper, on_that_issuers_credit: f64) -> f64 {
    assert!(
        on_that_issuers_credit > 0.0,
        "9 D3: a haircut with no view of the issuer is one per type"
    );
    p.price / on_that_issuers_credit
}

/// No negative outstanding, and no maturity that passes without cash moving.
pub fn redeem(p: &Paper, held: f64, holder: PartyId) -> Option<(PartyId, PartyId, f64)> {
    if held <= 0.0 || held > p.face {
        return None;
    }
    Some((p.issuer, holder, held))
}

/// WHAT THIS BORROWER MUST RAISE. A PLACEHOLDER, and two things mark it as one. It reads the
/// borrower's receipts as nothing, which is a stated value for an outcome — what its customers
/// actually paid it. And the rule itself is the sovereign's, borrowed because §9 has no
/// funding decision of its own. Both die at 0r, which builds one.
fn must_raise(owes: f64, cash: f64, buffer: f64) -> f64 {
    let restock = buffer - cash;
    match restock > 0.0 {
        true => owes + restock,
        false => owes,
    }
}
/// A BORROWER SHORT OVER THE WEEK BRINGS COMMERCIAL PAPER.
///
pub struct Brings {
    /// WHOSE paper this is, and over what horizon.
    pub of_kinds: &'static [u32],
    /// How far ahead this system's shortfall is read, in days — and from how far ahead.
    pub after: &'static str,
    pub horizon: &'static str,
    /// How long the paper runs.
    pub tenor: &'static str,
    /// The coupon the paper carries, as a term.
    /// The buffer the issuer keeps back.
    pub buffer: &'static str,
    pub says: u32,
    /// A current capital programme's explicit external-funding requirement.
    pub programme: Option<u32>,
    pub at_programme_funding: Option<u32>,
}

impl Mechanism for Brings {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let from = ctx.today();
        // The window is read from DATES.
        let to = Week(from.0 + ctx.params().weeks(self.horizon) as i64 - 1);
        let opens = Week(from.0 + ctx.params().weeks(self.after) as i64);
        let tenor = ctx.params().weeks(self.tenor) as i64;
        let buffer = ctx
            .params()
            .amount(self.buffer, crate::params::Denomination::Money);
        let mut programme_need: std::collections::HashMap<u32, f64> =
            std::collections::HashMap::new();
        if let (Some(kind), Some(at)) = (self.programme, self.at_programme_funding) {
            for &row in ctx.journal().of_kind(kind) {
                if ctx.journal().period_of(row) != ctx.week() {
                    continue;
                }
                if let (Some(&who), Some(Value::Num(need))) = (
                    ctx.journal().subjects_of(row).first(),
                    ctx.journal().says(row, at),
                ) {
                    programme_need.insert(who, need);
                }
            }
        }

        let mut bringing: Vec<(PartyId, CurrencyCode, f64)> = Vec::new();
        for p in 0..ctx.parties().len() {
            let who = PartyId::at(p as u32);
            let kind = ctx.parties().kind_of(who);
            if !ctx.parties().alive(who) || !self.of_kinds.contains(&kind) {
                continue;
            }
            // The profile answers whether a kind issues paper at all, and a kind with none is a kind
            // nobody has said this of — which is missing rather than a no.
            match ctx.registry().profile(kind) {
                Some(profile) if profile.issues_paper => {}
                _ => continue,
            }
            let Some(money) = account_of(ctx.parties(), ctx.instruments(), who) else {
                continue;
            };
            // Its own position: what falls due in the window, against what it holds.
            let owes = ctx.schedules().falling_for(who, opens, to);
            let cash = ctx.register().quantity(ctx.register().row(who, money));
            let ordinary = must_raise(owes, cash, buffer);
            let programme = match programme_need.get(&who.0) {
                Some(need) => *need,
                None => 0.0,
            };
            let short = if ordinary > programme {
                ordinary
            } else {
                programme
            };
            if short <= 0.0 {
                continue;
            }
            bringing.push((who, ctx.instruments().ccy_of(money), short));
        }

        for (who, ccy, short) in bringing {
            // A tenor is a term of MONTHS, so the maturity is reached by advancing a date (Money
            // G3.a).
            let matures = from.after(u32::try_from(tenor).expect("non-negative tenor"));
            ctx.brings(crate::module::Brings {
                issuer: who,
                initial_holder: None,
                loan_terms: None,
                issue_price: None,
                ccy,
                class: Class::Claim,
                unit: crate::ids::UnitId::at(0),
                coupon: None,
                matures: Some(matures),
                // Paper this short pays once, at the end, on the money-market count — which is what
                // makes it a different instrument from a bond rather than the same one with a
                // different number in it.
                pays: PaymentFrequency::AtMaturity,
                convention: Convention::Actual360,
                units: short,
                // An auction is a CALL — a sealed cross at one level, which is what an auction IS.
                book: Some(crate::protocols::Venue {
                    rule: crate::clearing::PriceRule::BuyersCompete,
                    protocol: crate::protocols::Protocol::Call,
                    seen_by: 1,
                    stands_for: None,
                }),
            });
            ctx.say(self.says, &[who.0], &[(0, Value::Num(short))], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::Commitment;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn bill(price: f64, days: i64) -> Paper {
        Paper {
            issuer: party(9),
            face: 100.0,
            price,
            issued: Week(0),
            matures: Week(days),
        }
    }

    fn limit(buyer: u32, most: f64, holding: f64) -> Limit {
        Limit {
            buyer: party(buyer),
            on_issuer: party(9),
            most,
            already_holding: holding,
        }
    }

    #[test]
    fn the_discount_is_the_whole_return_and_the_yield_comes_from_the_price() {
        // Never the reverse — E2's discount computed from a curve nobody traded is Law 3's defect at
        // the short end, and there is no function here that does it.
        let cheap = bill(98.0, 90);
        let dear = bill(99.5, 90);
        assert!(
            cheap.yield_on(Convention::Actual360).unwrap()
                > dear.yield_on(Convention::Actual360).unwrap()
        );
    }

    #[test]
    fn the_convention_is_a_material_part_of_the_number_at_this_tenor() {
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
        // Paper that always rolls at a written rate is not debt — it is a permanent liability with a
        // coupon, and it removes the only risk the instrument has.
        let willing = [(limit(20, 800.0, 0.0), 99.0), (limit(21, 400.0, 0.0), 98.5)];
        match roll(1_000.0, &willing, 100.0) {
            Rolled::Done {
                raised,
                at_price,
                from,
            } => {
                assert_eq!(raised, 1_000.0);
                assert!(at_price <= 99.0);
                assert_eq!(from[0].0, party(20));
            }
            other => panic!("expected a completed roll, got {other:?}"),
        }
        // The buyers are at their limits, and the issuer must find the money somewhere.
        let full = [
            (limit(20, 800.0, 800.0), 99.0),
            (limit(21, 400.0, 400.0), 98.5),
        ];
        assert_eq!(
            roll(1_000.0, &full, 100.0),
            Rolled::Declined { short_by: 1_000.0 }
        );
    }

    #[test]
    fn a_partial_roll_leaves_the_issuer_short_by_the_difference() {
        // This is what a run is made of — and the shortfall is a real number it must find.
        let thin = [(limit(20, 300.0, 0.0), 99.0)];
        assert_eq!(
            roll(1_000.0, &thin, 100.0),
            Rolled::Declined { short_by: 703.0 }
        );
    }

    #[test]
    fn a_limit_per_issuer_is_why_funding_is_lost_before_solvency_is() {
        // The buyer stops before the issuer fails.
        let l = limit(20, 800.0, 750.0);
        assert_eq!(l.room(), 50.0);
        let at_the_limit = limit(20, 800.0, 800.0);
        assert_eq!(at_the_limit.room(), 0.0);
    }

    #[test]
    fn the_backstop_costs_money_in_every_period_it_is_not_used() {
        // A committed line with no commitment fee on undrawn headroom is a free option the lender
        // did not sell.
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
        assert!(b.live_on(Week(9_000)));
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
        let outstanding = [
            Paper {
                face: 500.0,
                matures: Week(30),
                ..bill(99.0, 30)
            },
            Paper {
                face: 700.0,
                matures: Week(35),
                ..bill(99.0, 35)
            },
            Paper {
                face: 300.0,
                matures: Week(200),
                ..bill(97.0, 200)
            },
        ];
        assert_eq!(wall(&outstanding, Week(40)), 1_200.0);
        assert_eq!(wall(&outstanding, Week(10)), 0.0);
    }

    #[test]
    fn the_buyer_compares_the_paper_with_its_alternatives() {
        // The bill yield moves when the policy rate does BECAUSE the buyers' alternative moved — not
        // because a rule ties them.
        assert!(prefers_paper(0.050, 0.030, 0.035, 0.002));
        // Raise the facility rate and the same paper stops being worth holding.
        assert!(!prefers_paper(0.050, 0.030, 0.060, 0.002));
        // And a credit it doubts has to pay more for the same decision.
        assert!(!prefers_paper(0.050, 0.030, 0.035, 0.030));
    }

    #[test]
    fn the_spread_over_the_bill_is_a_read_of_two_cleared_prices() {
        // Never a stored number, and both must have printed.
        let corporate = bill(97.5, 90);
        let govt = Paper {
            issuer: party(1),
            ..bill(99.2, 90)
        };
        assert!(spread_over_bill(&corporate, &govt, Convention::Actual360).unwrap() > 0.0);
        let unpriced = bill(0.0, 90);
        assert!(spread_over_bill(&unpriced, &govt, Convention::Actual360).is_none());
    }

    #[test]
    fn it_is_collateral_at_a_haircut_that_reads_the_issuers_credit() {
        assert!(lends_against(&bill(98.0, 90), 1.01) > lends_against(&bill(98.0, 90), 1.20));
    }

    #[test]
    fn no_maturity_passes_without_cash_moving() {
        // And there is no negative outstanding to redeem.
        let p = bill(98.0, 90);
        assert_eq!(
            redeem(&p, 100.0, party(20)),
            Some((party(9), party(20), 100.0))
        );
        assert!(redeem(&p, 0.0, party(20)).is_none());
        assert!(redeem(&p, 140.0, party(20)).is_none());
    }
}
