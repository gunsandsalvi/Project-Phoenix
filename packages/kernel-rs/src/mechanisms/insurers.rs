//! INSURERS AND PENSIONS: a real liability to named beneficiaries, discounted at a rate read from a
//! market — the beneficiary does not absorb the investment result.
//!
//! @spec 27 A1 · 27 A2 · 27 A2.a · 27 A2.b · 27 A3 · 27 A4 · 27 A4.a · 27 A4.b · 27 A4.c · 27 B1 ·
//! @spec 27 B2 · 27 B2.a · 27 B2.b · 27 B3 · 27 B4 · 27 C1 · 27 C2 · 27 C2.a · 27 C3 · 27 C4 ·
//! @spec 27 C5 · 27 D1 · 27 D2 · 27 D3 · 27 D4 · 27 D4.a · 27 D5 · 27 E1 · 27 E2 · 27 E3 · XI-2 ·
//! @spec Law 3, Law 5, Law 6, Law 19 · Appendix B

use crate::assembly::kinds;
use crate::clearing::{whole_pieces, Order, Side};
use crate::ids::{book_of, InstrumentId, MarketId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::module::{Participant, ParticipantView};

/// The liability has a schedule — how much is owed in each future period — and E1: somebody NAMED is
/// owed the money.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Owed {
    pub to: PartyId,
    pub amount: f64,
    pub in_years: f64,
}

/// A liability of the institution, not a fund share.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Bears {
    /// The institution bears the investment result.
    TheInstitution,
    /// The beneficiary does — and then it is a fund share, and §13 is where it lives.
    TheBeneficiary,
}

#[derive(Clone, Debug)]
pub struct Institution {
    pub who: PartyId,
    pub schedule: Vec<Owed>,
    pub bears: Bears,
    /// It invests the premiums, and the portfolio is a DECISION with reasons.
    pub assets: f64,
    /// What its surplus can stand behind.
    pub surplus: f64,
}

/// The present value depends on a discount rate READ FROM A MARKET.
pub fn present_value(schedule: &[Owed], rate: f64) -> f64 {
    assert!(
        rate > -1.0,
        "27 B2: a discount rate below -100% discounts a payment into a payment"
    );
    schedule
        .iter()
        .map(|o| o.amount / (1.0 + rate).powf(o.in_years))
        .sum()
}

impl Institution {
    /// Equity is assets minus liabilities, a READ, and it can go negative — a solvency event with
    /// consequences.
    pub fn equity(&self, rate: f64) -> f64 {
        self.assets - present_value(&self.schedule, rate)
    }

    /// Assets and liabilities do not match, and the mismatch is measurable in duration — the
    /// weighted time of what is owed, against the assets'.
    pub fn liability_duration(&self, rate: f64) -> Option<f64> {
        let pv = present_value(&self.schedule, rate);
        if pv <= 0.0 {
            return None;
        }
        let weighted: f64 = self
            .schedule
            .iter()
            .map(|o| o.in_years * o.amount / (1.0 + rate).powf(o.in_years))
            .sum();
        Some(weighted / pv)
    }
}

/// An insurer quotes a price for cover that answers ITS OWN losses and ITS OWN capital.
#[derive(Clone, Copy, Debug)]
pub struct Quote {
    pub by: PartyId,
    /// Its own experience, which moves toward what its periods actually cost it.
    pub expected_claims_per_unit: f64,
    pub capital_per_unit: f64,
    pub needs_on_capital: f64,
}

impl Quote {
    pub fn price(&self) -> f64 {
        self.expected_claims_per_unit + self.capital_per_unit * self.needs_on_capital
    }
}

/// A policy goes to the insurer that prices lower, subject to the cover that insurer's surplus can
/// stand behind.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Placed {
    With {
        insurer: PartyId,
        at_price: f64,
    },
    /// Nobody could stand behind it.
    Unplaced,
}

pub fn place(cover: f64, quotes: &[(Quote, f64)]) -> Placed {
    let mut best: Option<(PartyId, f64)> = None;
    for (q, surplus_backing) in quotes {
        if *surplus_backing < cover {
            // An insurer with no surplus writes nothing — this is that, as arithmetic.
            continue;
        }
        let price = q.price();
        match best {
            Some((_, so_far)) if price >= so_far => {}
            _ => best = Some((q.by, price)),
        }
    }
    match best {
        Some((insurer, at_price)) => Placed::With { insurer, at_price },
        None => Placed::Unplaced,
    }
}

/// Its experience moves toward what its periods actually cost it — adaptively, from its own history,
/// and never from a sector figure.
pub fn experience(held: f64, this_period_cost: f64, memory: f64) -> f64 {
    assert!(
        memory > 0.0 && memory < 1.0,
        "46 A2: a memory of {memory} is not a weighting"
    );
    held * memory + this_period_cost * (1.0 - memory)
}

/// A catastrophe is ONE EVENT hitting many policies at once, which is different from the average
/// being higher.
#[derive(Clone, Debug, PartialEq)]
pub struct Catastrophe {
    pub hit: Vec<(PartyId, f64)>,
}

impl Catastrophe {
    pub fn total(&self) -> f64 {
        self.hit.iter().map(|(_, c)| c).sum()
    }

    /// The thing an average cannot show: how much of the book one event touched.
    pub fn policies_hit(&self) -> usize {
        self.hit.len()
    }
}

/// The dominant reason is matching the schedule — long assets against long liabilities — so it is a
/// STRUCTURAL buyer of long bonds and long swaps, a one-way demand that exists whatever the price.
pub fn duration_gap(liability_duration: f64, asset_duration: f64, liabilities: f64) -> f64 {
    (liability_duration - asset_duration) * liabilities
}

/// The mismatch moves equity when rates move, in the opposite direction to a bank's.
pub fn on_a_rate_move(i: &Institution, rate_before: f64, rate_now: f64) -> f64 {
    i.equity(rate_now) - i.equity(rate_before)
}

/// A funding shortfall has consequences: the sponsor contributes, the fund de-risks, or benefits are
/// cut — each a real action by a NAMED party.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum OnShortfall {
    SponsorContributes {
        sponsor: PartyId,
        amount: f64,
    },
    DeRisks {
        selling: f64,
    },
    BenefitsCut {
        to: PartyId,
        by: f64,
    },
    /// Solvent: there is no shortfall to answer.
    Nothing,
}

pub fn shortfall(
    i: &Institution,
    rate: f64,
    sponsor: Option<PartyId>,
    sponsor_can_pay: f64,
) -> OnShortfall {
    let gap = -i.equity(rate);
    if gap <= 0.0 {
        return OnShortfall::Nothing;
    }
    match sponsor {
        Some(s) if sponsor_can_pay >= gap => OnShortfall::SponsorContributes {
            sponsor: s,
            amount: gap,
        },
        _ if i.assets > 0.0 => OnShortfall::DeRisks { selling: gap },
        _ => {
            // Nothing left to sell and nobody to ask: the promise itself is cut, and the beneficiary
            // is named because somebody bears it.
            let first = i.schedule.first().map(|o| o.to);
            match first {
                Some(to) => OnShortfall::BenefitsCut { to, by: gap },
                None => OnShortfall::Nothing,
            }
        }
    }
}

/// Hedging the gap costs money and creates margin calls, and a leveraged hedge turns a solvency
/// improvement into a LIQUIDITY requirement — the failure mode of the whole sector.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hedged {
    /// What the hedge did to the solvency position.
    pub equity_moved: f64,
    /// And what it demands in cash NOW, which the liability never does.
    pub cash_now: f64,
}

pub fn hedge(gap: f64, leverage: f64, rate_moved: f64) -> Hedged {
    assert!(leverage > 0.0, "27 D4: a hedge with no size is not a hedge");
    let notional = gap * leverage;
    Hedged {
        equity_moved: notional * rate_moved,
        cash_now: notional * rate_moved,
    }
}

/// It is a buyer of credit, and its mandate limits which credits — so a downgrade can FORCE a sale.
pub fn must_sell(holding: f64, still_eligible: bool) -> Option<f64> {
    if still_eligible {
        return None;
    }
    Some(holding)
}

/// It can hold illiquid assets because it does not face redemption the way a fund does — that is
/// what it is paid for.
pub fn illiquidity_premium(illiquid_yield: f64, liquid_yield: f64) -> f64 {
    illiquid_yield - liquid_yield
}

/// WHAT IS ON RISK. A count of every live relation in the world stands in for it: §29 writes no
/// policy row of its own, so this says more than it knows and its own item is what narrows it.
pub struct Policies {
    pub kind: u32,
}

impl Mechanism for Policies {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let n = ctx.agreements().live_now() as f64;
        ctx.say(self.kind, &[], &[(0, Value::Num(n))], true);
    }
}

/// A structural buyer of long bonds — a one-way demand that exists whatever the price, because its
/// liabilities are long and its assets are not.
pub struct InsurerMatching {
    pub long_lines: Vec<InstrumentId>,
}

impl Participant for InsurerMatching {
    fn party_kind(&self) -> u32 {
        kinds::INSURER
    }

    fn markets(&self, view: &ParticipantView<'_>) -> Vec<MarketId> {
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        self.long_lines.iter().map(|l| book_of(*l)).collect()
    }

    fn orders(&self, view: &ParticipantView<'_>, m: MarketId) -> Vec<Order> {
        let money = view.own_cash();
        let Some(will_pay) = view.price_outlook(crate::ids::line_of(m)) else {
            return Vec::new();
        };
        // Less what it is already bidding for here.
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

    fn pension() -> Institution {
        Institution {
            who: party(30),
            schedule: vec![
                Owed {
                    to: party(40),
                    amount: 1_000.0,
                    in_years: 5.0,
                },
                Owed {
                    to: party(41),
                    amount: 1_000.0,
                    in_years: 20.0,
                },
            ],
            bears: Bears::TheInstitution,
            assets: 1_400.0,
            surplus: 200.0,
        }
    }

    #[test]
    fn falling_rates_raise_the_liability_which_is_why_a_rate_move_is_a_solvency_event() {
        // A liability that is a cash balance never moves when rates move, and the sector's defining
        // risk disappears.
        let dear = present_value(&pension().schedule, 0.02);
        let cheap = present_value(&pension().schedule, 0.06);
        assert!(dear > cheap);
    }

    #[test]
    fn equity_is_a_read_and_it_can_go_negative() {
        // No solvency measured against a stored liability value — it is recomputed from the schedule
        // and the rate, every time.
        let p = pension();
        assert!(p.equity(0.06) > 0.0);
        assert!(p.equity(0.01) < 0.0);
    }

    #[test]
    fn the_institution_bears_the_investment_result_and_not_the_beneficiary() {
        // A sector that passes it straight through is a fund wearing an insurer's name.
        assert_eq!(pension().bears, Bears::TheInstitution);
        let unit_linked = Institution {
            bears: Bears::TheBeneficiary,
            ..pension()
        };
        assert_eq!(unit_linked.bears, Bears::TheBeneficiary);
    }

    #[test]
    fn the_liability_has_a_duration_and_this_sector_is_the_largest_holder_of_it() {
        // It must HAVE duration, and the mismatch is measurable.
        let d = pension().liability_duration(0.04).unwrap();
        assert!(d > 5.0 && d < 20.0);
        let nothing_owed = Institution {
            schedule: Vec::new(),
            ..pension()
        };
        assert!(nothing_owed.liability_duration(0.04).is_none());
        // And the gap is a one-way demand for long assets, which is a real force in that market.
        assert!(duration_gap(d, 4.0, 1_400.0) > 0.0);
    }

    #[test]
    fn a_policy_goes_to_the_insurer_that_prices_lower_and_cover_nobody_can_write_is_unplaced() {
        // Worse experience or dearer capital quotes higher — and an insurer with no surplus writes
        // nothing, losing book before it loses its licence.
        let cheap = Quote {
            by: party(1),
            expected_claims_per_unit: 0.04,
            capital_per_unit: 0.2,
            needs_on_capital: 0.10,
        };
        let dear = Quote {
            by: party(2),
            expected_claims_per_unit: 0.06,
            capital_per_unit: 0.2,
            needs_on_capital: 0.15,
        };
        assert!(dear.price() > cheap.price());
        assert_eq!(
            place(500.0, &[(cheap, 1_000.0), (dear, 1_000.0)]),
            Placed::With {
                insurer: party(1),
                at_price: cheap.price()
            }
        );
        // The cheaper insurer cannot stand behind this much, so the dearer one writes it.
        assert_eq!(
            place(900.0, &[(cheap, 500.0), (dear, 1_000.0)]),
            Placed::With {
                insurer: party(2),
                at_price: dear.price()
            }
        );
        // And cover nobody can stand behind is unplaced, paying no premium — not written dearer.
        assert_eq!(
            place(5_000.0, &[(cheap, 500.0), (dear, 1_000.0)]),
            Placed::Unplaced
        );
    }

    #[test]
    fn an_insurer_draws_its_claims_off_its_own_experience() {
        // Its own cover, its own experience, moving toward what its periods actually cost it.
        let held = 0.04;
        let after_a_bad_year = experience(held, 0.09, 0.7);
        assert!(after_a_bad_year > held);
        let after_a_quiet_one = experience(held, 0.01, 0.7);
        assert!(after_a_quiet_one < held);
    }

    #[test]
    fn a_catastrophe_is_one_event_hitting_many_policies_at_once() {
        // Which is different from the average being higher, and claims as a ratio of premium for
        // every policy have no representation for one.
        let c = Catastrophe {
            hit: vec![(party(50), 300.0), (party(51), 250.0), (party(52), 700.0)],
        };
        assert_eq!(c.policies_hit(), 3);
        assert_eq!(c.total(), 1_250.0);
    }

    #[test]
    fn a_shortfall_is_answered_by_a_named_party_and_never_by_nothing() {
        // The sponsor contributes, the fund de-risks, or benefits are cut — each a real action.
        let p = pension();
        assert_eq!(
            shortfall(&p, 0.06, Some(party(60)), 10_000.0),
            OnShortfall::Nothing
        );
        match shortfall(&p, 0.01, Some(party(60)), 10_000.0) {
            OnShortfall::SponsorContributes { sponsor, amount } => {
                assert_eq!(sponsor, party(60));
                assert!(amount > 0.0);
            }
            other => panic!("expected a contribution, got {other:?}"),
        }
        // No sponsor able to pay: it de-risks by selling.
        assert!(matches!(
            shortfall(&p, 0.01, None, 0.0),
            OnShortfall::DeRisks { .. }
        ));
        // Nothing left to sell and nobody to ask: the promise is cut, and the beneficiary is named.
        let empty = Institution {
            assets: 0.0,
            ..pension()
        };
        assert!(matches!(
            shortfall(&empty, 0.01, None, 0.0),
            OnShortfall::BenefitsCut { .. }
        ));
    }

    #[test]
    fn a_leveraged_hedge_turns_a_solvency_improvement_into_a_liquidity_requirement() {
        // The failure mode of the whole sector — and the asymmetry is that cash moves on the hedge
        // and not on the liability.
        let modest = hedge(1_000.0, 1.0, 0.02);
        let levered = hedge(1_000.0, 8.0, 0.02);
        assert!(levered.equity_moved > modest.equity_moved);
        assert!(levered.cash_now > modest.cash_now);
        // The liability moved too — and demanded no cash at all.
        let liability_moved = on_a_rate_move(&pension(), 0.04, 0.02);
        assert!(liability_moved < 0.0);
    }

    #[test]
    fn a_downgrade_can_force_a_sale() {
        // The mandate meeting a rating it may no longer hold.
        assert!(must_sell(5_000.0, true).is_none());
        assert_eq!(must_sell(5_000.0, false), Some(5_000.0));
    }

    #[test]
    fn it_is_paid_for_not_facing_redemption_the_way_a_fund_does() {
        // The premium is a read of two prices, never a stated bonus.
        assert!(illiquidity_premium(0.08, 0.05) > 0.0);
    }

    #[test]
    #[should_panic(expected = "discounts a payment into a payment")]
    fn a_rate_below_minus_one_hundred_per_cent_is_refused() {
        present_value(&pension().schedule, -1.5);
    }

    #[test]
    #[should_panic(expected = "is not a hedge")]
    fn a_hedge_with_no_size_is_not_a_hedge() {
        hedge(1_000.0, 0.0, 0.02);
    }
}
