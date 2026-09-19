//! M&A AND CORPORATE CONTROL: an acquirer, a target, and a price per share the target's owners
//! accept — or refuse.
//!
//! @spec 35 A1 · 35 A2 · 35 A3 · 35 A3.a · 35 A3.b · 35 A4 · 35 A5 · 35 B1 · 35 B2 · 35 B2.a ·
//! @spec 35 B3 · 35 B4 · 35 B5 · 35 C1 · 35 C2 · 35 C3 · 35 D1 · 35 D2 · 35 D3 · 35 D4 · 35 D5 ·
//! @spec 35 E1 · 35 E2 · 35 E3 · XI-4 · Law 2, Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::calendar::Day;
use crate::ids::PartyId;

/// Cash, shares, or both — and the choice matters. A3.a: cash needs funding and increases the
/// acquirer's leverage.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Consideration {
    pub cash_per_share: f64,
    pub shares_per_share: f64,
}

impl Consideration {
    /// The price per share, in the acquirer's money, at what its own shares are worth now — a
    /// cleared price, never a book value.
    pub fn per_share(&self, acquirers_share_price: f64) -> f64 {
        self.cash_per_share + self.shares_per_share * acquirers_share_price
    }
}

/// The acquirer's own valuation — the target's expected earnings discounted at the acquirer's own
/// hurdle. Formed by its management, and different from the next acquirer's.
pub fn worth_to(expected_earnings: f64, own_hurdle: f64) -> Option<f64> {
    if own_hurdle <= 0.0 {
        // An acquirer with no hurdle values every target at infinity, which is not a valuation.
        return None;
    }
    Some(expected_earnings / own_hurdle)
}

/// A named acquirer, a named target, a price, and the funding it has arranged — because the credit
/// market decides which deals happen.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bid {
    pub acquirer: PartyId,
    pub target: PartyId,
    pub offering: Consideration,
    /// What its lenders committed. A bid it cannot fund is not a bid.
    pub funded: f64,
    pub on: Day,
}

/// The premium is what it must pay to get the owners to sell, so it is an outcome of what they would
/// accept and not a stated percentage. `None` where the target has no price to be at a premium to
/// (Law 3: there is nothing to measure against).
pub fn premium(offered_per_share: f64, market_price: Option<f64>) -> Option<f64> {
    let market = market_price?;
    if market <= 0.0 {
        return None;
    }
    Some(offered_per_share / market - 1.0)
}

/// One dispersed owner, with its own valuation of holding on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Owner {
    pub who: PartyId,
    pub units: f64,
    /// What holding is worth to THIS owner. Its own, and it is why some accept and some do not.
    pub holding_is_worth: f64,
}

/// Management may resist, and its interests differ from the owners' — which is the corporate-control
/// problem and the reason takeovers discipline firms at all. Resistance is an act by a named party,
/// with a size: how much of the price it can put between the bid and the owners.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Resistance {
    pub by: PartyId,
    /// What defending costs the bidder, per share. It comes out of what the owners would receive,
    /// which is exactly why their interests and management's differ.
    pub costs_the_bidder: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// Ownership transfers in the register and the target's shareholders are PAID.
    Accepted { accepting: Vec<PartyId>, units: f64, paid: f64 },
    /// A target's owners can refuse, and a bid can fail — a real outcome with consequences for both
    /// prices.
    Refused { accepting_units: f64, needed: f64 },
    /// The credit market decides which deals happen.
    Unfunded { short_by: f64 },
}

/// Each owner decides individually and the outcome is the aggregate of those decisions. There is no
/// vote at an average and no representative holder (Appendix B: no decision at an average, no
/// representative agent where decisions are thresholds).
pub fn tender(
    bid: &Bid,
    acquirers_share_price: f64,
    owners: &[Owner],
    needed_units: f64,
    resistance: Option<Resistance>,
) -> Outcome {
    let gross = bid.offering.per_share(acquirers_share_price);
    // What management's defence costs comes out of the price the owners see.
    let reaching_owners = match resistance {
        Some(r) => gross - r.costs_the_bidder,
        None => gross,
    };

    let mut accepting: Vec<PartyId> = Vec::new();
    let mut units = 0.0;
    for o in owners {
        // The price beats holding, on THEIR OWN valuation.
        if reaching_owners > o.holding_is_worth {
            accepting.push(o.who);
            units += o.units;
        }
    }
    if units < needed_units {
        return Outcome::Refused { accepting_units: units, needed: needed_units };
    }
    let owed = units * gross;
    let cash_owed = units * bid.offering.cash_per_share;
    if cash_owed > bid.funded {
        return Outcome::Unfunded { short_by: cash_owed - bid.funded };
    }
    Outcome::Accepted { accepting, units, paid: owed }
}

/// A competing bidder can appear, and then the price is contested, which is the auction working. The
/// owners see both and take the better one; a bid that nobody beats is not a proof that it was the
/// right price, only that nobody came.
pub fn contested(bids: &[Bid], acquirers_share_prices: &[f64]) -> Option<usize> {
    assert!(
        bids.len() == acquirers_share_prices.len(),
        "35 B4: a bid without its bidder's share price cannot be compared"
    );
    let mut best: Option<(usize, f64)> = None;
    for (at, bid) in bids.iter().enumerate() {
        let offered = bid.offering.per_share(acquirers_share_prices[at]);
        match best {
            Some((_, so_far)) if offered <= so_far => {}
            _ => best = Some((at, offered)),
        }
    }
    best.map(|(at, _)| at)
}

/// The target's debt does not disappear. Three honest answers, and no fourth.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Debt {
    /// Paid off at the deal, with money that came from somewhere named.
    Repaid,
    /// Carried onto the combined balance sheet. D4: obligations are assumed, not written off.
    Assumed,
    /// A change-of-control term fired and it fell due — which is a funding problem for the acquirer
    /// on the day, and part of what B3 decides.
    Triggered,
}

/// After it, the two balance sheets combine and the combined firm is one party.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Combined {
    pub acquirer_flows: f64,
    pub target_flows: f64,
    /// What the acquirer CLAIMED it could change — carried separately, never added into the flows.
    /// D1's "whether that materialises is measurable" is then a subtraction anybody can do, and a
    /// headcount saving stays a claim until a separation event exists for it.
    pub claimed: f64,
}

impl Combined {
    /// The sum. The claim is not in it.
    pub fn flows(&self) -> f64 {
        self.acquirer_flows + self.target_flows
    }

    /// What actually turned up against what was claimed. A VERIFY — it measures and repairs nothing.
    pub fn materialised(&self, observed_flows: f64) -> f64 {
        observed_flows - self.flows() - self.claimed
    }
}

/// The acquirer's leverage is higher if it paid cash, and its credit is reassessed — which is why
/// its bonds can fall on the day its shares rise, and both are correct. A read of what it borrowed
/// against what it had.
pub fn leverage_after(debt_before: f64, borrowed_for_it: f64, equity: f64) -> Option<f64> {
    if equity <= 0.0 {
        return None;
    }
    Some((debt_before + borrowed_for_it) / equity)
}

/// The money paid to target shareholders equals what the acquirer and its lenders put up, exactly,
/// and it lands in named accounts. A VERIFY on Law 7's derived dust; no acquisition without payment.
pub fn payment_conserves(paid_to_owners: f64, acquirer_put_up: f64, lenders_put_up: f64) -> bool {
    let residual = paid_to_owners - (acquirer_put_up + lenders_put_up);
    residual.abs() <= crate::num::dust(3, &[paid_to_owners, acquirer_put_up, lenders_put_up])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn cash_bid(per_share: f64, funded: f64) -> Bid {
        Bid {
            acquirer: party(1),
            target: party(9),
            offering: Consideration { cash_per_share: per_share, shares_per_share: 0.0 },
            funded,
            on: Day(30),
        }
    }

    fn owners() -> Vec<Owner> {
        vec![
            Owner { who: party(20), units: 400.0, holding_is_worth: 9.0 },
            Owner { who: party(21), units: 400.0, holding_is_worth: 11.0 },
            Owner { who: party(22), units: 200.0, holding_is_worth: 14.0 },
        ]
    }

    #[test]
    fn two_acquirers_value_the_same_target_differently_because_the_hurdle_is_their_own() {
        // The intent is the acquirer's own valuation, formed by its management — never a screening
        // threshold. This is also what makes a contested auction possible.
        let patient = worth_to(100.0, 0.08).unwrap();
        let demanding = worth_to(100.0, 0.15).unwrap();
        assert!(patient > demanding);
        assert!(worth_to(100.0, 0.0).is_none());
    }

    #[test]
    fn each_dispersed_owner_decides_individually_and_the_outcome_is_the_aggregate() {
        // No vote at an average and no representative holder. At 12 a share the two owners who
        // value holding below that accept, and the third does not.
        let out = tender(&cash_bid(12.0, 100_000.0), 0.0, &owners(), 700.0, None);
        match out {
            Outcome::Accepted { accepting, units, paid } => {
                assert_eq!(accepting, vec![party(20), party(21)]);
                assert_eq!(units, 800.0);
                assert_eq!(paid, 9_600.0);
            }
            other => panic!("expected an acceptance, got {other:?}"),
        }
    }

    #[test]
    fn a_bid_can_fail_because_the_owners_refuse() {
        // A real outcome with consequences for both prices. Law 6: nothing forces it through.
        let out = tender(&cash_bid(10.0, 100_000.0), 0.0, &owners(), 700.0, None);
        assert_eq!(out, Outcome::Refused { accepting_units: 400.0, needed: 700.0 });
    }

    #[test]
    fn the_credit_market_decides_which_deals_happen() {
        // The owners accepted and the deal still does not happen, because the acquirer cannot fund
        // what it promised.
        let out = tender(&cash_bid(12.0, 5_000.0), 0.0, &owners(), 700.0, None);
        assert_eq!(out, Outcome::Unfunded { short_by: 4_600.0 });
    }

    #[test]
    fn management_can_resist_and_its_interests_differ_from_the_owners() {
        // The corporate-control problem, and the reason takeovers discipline firms at all. The
        // defence costs the bidder, the price reaching the owners falls, and a deal they would have
        let bid = cash_bid(12.0, 100_000.0);
        let unopposed = tender(&bid, 0.0, &owners(), 700.0, None);
        assert!(matches!(unopposed, Outcome::Accepted { .. }));
        let defended = Resistance { by: party(9), costs_the_bidder: 2.0 };
        let opposed = tender(&bid, 0.0, &owners(), 700.0, Some(defended));
        assert_eq!(opposed, Outcome::Refused { accepting_units: 400.0, needed: 700.0 });
    }

    #[test]
    fn paying_in_shares_is_priced_at_what_the_acquirers_shares_are_worth_now() {
        // Shares dilute the acquirer's existing owners, which is a real cost to them — and what the
        // target's owners receive depends on a cleared price, not on a book value.
        let in_shares = Bid {
            offering: Consideration { cash_per_share: 2.0, shares_per_share: 0.25 },
            ..cash_bid(0.0, 100_000.0)
        };
        assert_eq!(in_shares.offering.per_share(40.0), 12.0);
        assert_eq!(in_shares.offering.per_share(20.0), 7.0);
        // The same bid, after the acquirer's shares fall, no longer clears the owners' valuations.
        assert!(matches!(
            tender(&in_shares, 40.0, &owners(), 700.0, None),
            Outcome::Accepted { .. }
        ));
        assert!(matches!(
            tender(&in_shares, 20.0, &owners(), 700.0, None),
            Outcome::Refused { .. }
        ));
    }

    #[test]
    fn a_competing_bidder_contests_the_price_which_is_the_auction_working() {
        let first = cash_bid(12.0, 100_000.0);
        let rival = Bid { acquirer: party(2), ..cash_bid(13.5, 100_000.0) };
        assert_eq!(contested(&[first, rival], &[0.0, 0.0]), Some(1));
        assert!(contested(&[], &[]).is_none());
    }

    #[test]
    fn the_premium_is_measured_against_a_price_and_is_missing_without_one() {
        // It is what the bidder must pay to get the owners to sell — an outcome, not a stated
        // percentage — and there is nothing to measure it against where the target has no price.
        let paid = premium(12.0, Some(10.0)).unwrap();
        // Derived dust, never a band and never a float written out to its last digit.
        assert!((paid - 0.2).abs() <= crate::num::dust(2, &[12.0 / 10.0, 1.0]));
        assert!(premium(12.0, None).is_none());
    }

    #[test]
    fn no_synergy_is_assumed_into_the_cash_flows() {
        // The claim is carried separately, so whether it materialised is a subtraction anybody can
        // do. A headcount saving stays a claim until a separation event exists for it.
        let c = Combined { acquirer_flows: 500.0, target_flows: 300.0, claimed: 120.0 };
        assert_eq!(c.flows(), 800.0);
        // It delivered nothing of what was claimed.
        assert_eq!(c.materialised(800.0), -120.0);
        // It delivered all of it.
        assert_eq!(c.materialised(920.0), 0.0);
    }

    #[test]
    fn the_targets_debt_is_repaid_assumed_or_triggered_and_never_gone() {
        // An acquired firm is not a dead firm. There is no fourth answer to write.
        for state in [Debt::Repaid, Debt::Assumed, Debt::Triggered] {
            assert!(matches!(state, Debt::Repaid | Debt::Assumed | Debt::Triggered));
        }
    }

    #[test]
    fn an_acquirer_that_paid_cash_is_more_levered_afterwards() {
        // Its credit is reassessed, which is why its bonds can fall on the day its shares rise and
        // both are correct.
        let before = leverage_after(1_000.0, 0.0, 2_000.0).unwrap();
        let after = leverage_after(1_000.0, 900.0, 2_000.0).unwrap();
        assert!(after > before);
        assert!(leverage_after(1_000.0, 900.0, 0.0).is_none());
    }

    #[test]
    fn the_money_paid_equals_what_the_acquirer_and_its_lenders_put_up() {
        // No acquisition without payment, and it lands in named accounts. A VERIFY on Law 7 dust,
        // which answers false rather than balancing itself.
        assert!(payment_conserves(9_600.0, 2_600.0, 7_000.0));
        assert!(!payment_conserves(9_600.0, 2_600.0, 6_000.0));
    }

    #[test]
    #[should_panic(expected = "cannot be compared")]
    fn a_bid_without_its_bidders_share_price_cannot_be_compared() {
        contested(&[cash_bid(12.0, 1.0)], &[]);
    }
}
