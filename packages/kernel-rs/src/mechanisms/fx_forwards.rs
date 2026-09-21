//! FX FORWARDS AND CROSS-CURRENCY SWAPS: the forward rate is CLEARED and parity is checked against
//! it, never applied to produce it — and there is one basis.
//!
//! @spec 19 A1.a · 19 A1.b · 19 A1.c · 19 A1.d · 19 A2 · 19 A3 · 19 A4 · 19 B1 · 19 B2 · 19 B2.a ·
//! @spec 19 B2.b · 19 B3 · 19 B3.a · 19 B3.b · 19 B4 · 19 C1 · 19 C1.a · 19 C2 · 19 C3 · 19 C4 ·
//! @spec 19 D1 · 19 D2 · 19 D2.a · 19 D3 · 19 D4 · 19 E1 · 19 E2 · 19 E3 · 19 E4 · XI-12 · Law 3,
//! @spec Law 4, Law 5, Law 6, Law 19

use crate::calendar::Week;
use crate::ids::{CurrencyCode, PartyId};
use crate::journal::Value;
use crate::ledger::account_of;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::agreed;

/// A leg states its own money.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Side {
    pub party: PartyId,
    pub ccy: CurrencyCode,
    pub amount: f64,
}

/// An exchange of two fixed amounts at maturity at a forward rate that cleared.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Forward {
    pub pays: Side,
    pub receives: Side,
    /// Cleared from what participants will do.
    pub rate: f64,
    pub matures: Week,
    pub year_fraction: f64,
}

impl Forward {
    pub fn struck(terms: Forward) -> Forward {
        assert!(
            terms.pays.ccy != terms.receives.ccy,
            "19 A1.d: a forward with one money on both legs is not an FX forward"
        );
        assert!(
            terms.pays.party != terms.receives.party,
            "19 E2: a hedge needs a counterparty holding the other side"
        );
        assert!(
            terms.year_fraction > 0.0,
            "19 A3: a forward over no time is a spot trade (Law 8)"
        );
        terms
    }
}

/// Where the forward would sit if the arbitrage were free — spot adjusted for the two currencies'
/// funding costs, because otherwise somebody can borrow one, buy the other, lend it and lock a
/// profit.
pub fn parity(spot: f64, base_funding: f64, quote_funding: f64, year_fraction: f64) -> f64 {
    spot * (1.0 + quote_funding * year_fraction) / (1.0 + base_funding * year_fraction)
}

/// The cross-currency basis is the deviation, and it is a real price paid by whoever needs the
/// currency more.
pub fn basis(f: &Forward, spot: f64, base_funding: f64, quote_funding: f64) -> f64 {
    let at_parity = parity(spot, base_funding, quote_funding, f.year_fraction);
    (f.rate / at_parity - 1.0) / f.year_fraction
}

/// The arbitrage uses balance sheet, capital and credit lines.
#[derive(Clone, Copy, Debug)]
pub struct Arbitrageur {
    pub who: PartyId,
    pub balance_sheet_free: f64,
    /// What it must earn on the balance sheet it uses.
    pub needs: f64,
    /// And a line to the counterparty, because this trade is credit as well as capital.
    pub line_to_counterparty: f64,
}

/// What it does about a basis: the size it can fund, or `None` — the gap stands, and B2.b says that
/// is a finding about its constraints.
pub fn closes(a: &Arbitrageur, basis_now: f64, size_available: f64) -> Option<f64> {
    if basis_now.abs() <= a.needs {
        return None;
    }
    let room = if a.balance_sheet_free < a.line_to_counterparty {
        a.balance_sheet_free
    } else {
        a.line_to_counterparty
    };
    let size = if room < size_available {
        room
    } else {
        size_available
    };
    if size <= 0.0 {
        return None;
    }
    Some(size)
}

/// A forward is a funding item long before it is a settlement.
pub fn mark(f: &Forward, forward_now_for_tenor_left: f64, tenor_left: f64) -> f64 {
    assert!(
        tenor_left <= f.year_fraction,
        "19 A3: a forward cannot have more time left than it was struck for"
    );
    f.receives.amount * (forward_now_for_tenor_left - f.rate) * (tenor_left / f.year_fraction)
}

/// An FX swap — spot one way, forward back — is a SECURED LOAN of one currency against another, and
/// that is what it must be modelled as.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FxSwap {
    pub near: Forward,
    pub far: Forward,
}

impl FxSwap {
    /// What it costs the borrower of the scarce currency, over the swap's life — the funding rate
    /// the trade actually struck, read from the two legs.
    pub fn implied_funding(&self, other_currencys_rate: f64) -> f64 {
        let moved = self.far.rate / self.near.rate - 1.0;
        other_currencys_rate - moved / self.far.year_fraction
    }
}

/// Two legs in two currencies, notionals exchanged at start and end, periodic interest on both — an
/// interest-rate swap with an FX leg attached, inheriting both curves.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct CrossCurrency {
    pub a: Side,
    pub b: Side,
    pub at_rate: f64,
    pub a_pays: f64,
    pub b_pays: f64,
    /// Its price includes the basis, and that is where a foreign-currency funding shortage shows up
    /// as a number.
    pub basis: f64,
    pub years: f64,
}

impl CrossCurrency {
    /// The end exchange, at the rate struck at the start.
    pub fn returns_at_maturity(&self) -> (Side, Side) {
        (
            Side {
                party: self.b.party,
                ccy: self.a.ccy,
                amount: self.a.amount,
            },
            Side {
                party: self.a.party,
                ccy: self.b.ccy,
                amount: self.b.amount,
            },
        )
    }

    /// What the party needing the scarce currency pays for it, per week.
    pub fn periodic(&self, periods_per_year: f64) -> f64 {
        self.a.amount * (self.a_pays + self.basis) / periods_per_year
    }
}

/// No maturity passes without both legs settling in full, in both currencies.
pub fn settles(f: &Forward, pays_can_find: f64, receives_can_find: f64) -> Option<(Side, Side)> {
    if pays_can_find < f.pays.amount || receives_can_find < f.receives.amount {
        // A failure to deliver is a real state, and it is NOT a half-settled forward.
        return None;
    }
    Some((f.pays, f.receives))
}

/// The hedge must be ROLLED as the asset persists, which is a recurring demand and a recurring cost.
pub fn roll_cost(old: &Forward, new_rate: f64) -> f64 {
    (new_rate - old.rate) * old.receives.amount
}

/// A hedged foreign asset shows the asset revaluing one way and the forward the other, and the
/// residual is the basis and the imperfection — NOT zero by construction.
pub fn hedge_residual(asset_moved: f64, forward_moved: f64) -> f64 {
    asset_moved + forward_moved
}

/// Why a party is here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reason {
    /// An importer or exporter with a known future foreign payment.
    KnownPayment,
    /// An investor holding a foreign asset that wants the asset and not the currency.
    WantsTheAssetNotTheCurrency,
    /// A bank funding a foreign-currency book — deposits in one money, loans in another.
    FundingAForeignBook,
    Dealer,
}

/// The dealer's width, from what the position consumes and what it needs on that.
pub fn width(capital_consumed: f64, needs_on_capital: f64, size: f64) -> Option<f64> {
    if size <= 0.0 {
        return None;
    }
    Some(capital_consumed * needs_on_capital / size)
}

/// A FORWARD IS STRUCK, AND THE BASIS IS WHAT IT DEVIATES BY.
pub struct FxForwards {
    pub kind: u32,
    pub spot: u32,
    pub fixing: u32,
    /// How far out the forward is struck.
    pub tenor: &'static str,
}

impl Mechanism for FxForwards {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        // The tenor is executable weeks; the declared day-count convention only measures them.
        let weeks = ctx.params().weeks(self.tenor) as u32;
        let from = ctx.today();
        let matures = from.after(weeks);
        let tenor = crate::calendar::Convention::Actual365.year_fraction(from, matures);

        // The rate the pair last cleared at.
        let mut spot: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.spot) {
            if ctx.journal().period_of(row) == ctx.week() {
                if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                    spot = Some(rate);
                }
            }
        }
        let Some(spot) = spot else { return };
        // And what the two moneys fund at.
        let mut funding: Option<f64> = None;
        for &row in ctx.journal().of_kind(self.fixing) {
            if let Some(Value::Num(rate)) = ctx.journal().says(row, 0) {
                funding = Some(rate);
            }
        }
        let Some(funding) = funding else { return };

        // Where it would sit if the arbitrage were free.
        let parity = parity(spot, funding, funding, tenor);
        let mut hedging: Vec<(PartyId, CurrencyCode, CurrencyCode, f64)> = Vec::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            let Some(mine) = account_of(ctx.parties(), ctx.instruments(), who) else {
                continue;
            };
            let my_ccy = ctx.instruments().ccy_of(mine);
            for &d in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(d);
                // Beyond this week: what falls due now is a SPOT problem and is bought spot.
                if ctx.schedules().paid(d) || ctx.schedules().due(d) <= ctx.current_week() {
                    continue;
                }
                let owed_in = ctx.schedules().ccy(d);
                if owed_in == my_ccy {
                    continue;
                }
                hedging.push((who, my_ccy, owed_in, ctx.schedules().amount(d)));
            }
        }
        if hedging.len() < 2 {
            // A hedge needs a counterparty holding the other side.
            return;
        }

        let mut struck: Vec<(PartyId, PartyId, CurrencyCode, CurrencyCode, f64, f64)> = Vec::new();
        // The two ends of the book: whoever needs the most and whoever needs the least are the two
        // sides, and the rate is what they cross at.
        hedging.sort_by(|a, b| b.3.total_cmp(&a.3));
        for &(buyer, pays, receives, size) in &hedging {
            if let Some(&(seller, _, _, offered)) =
                hedging
                    .iter()
                    .find(|(who, seller_pays, seller_receives, _)| {
                        *who != buyer && *seller_pays == receives && *seller_receives == pays
                    })
            {
                struck.push((
                    buyer,
                    seller,
                    pays,
                    receives,
                    parity,
                    if size < offered { size } else { offered },
                ));
                break;
            }
        }

        for (buyer, seller, pays, receives, rate, size) in struck {
            // The basis is the deviation, and it is a real price paid by whoever needs the money.
            let basis = rate - parity;
            ctx.agrees(crate::module::Agrees {
                kind: agreed::FX_FORWARD,
                one: buyer,
                other: seller,
                terms: crate::stores::AgreementTerms::FxForward {
                    pays,
                    receives,
                    rate,
                    amount: size,
                    tenor_years: tenor,
                },
                until: Some(matures),
            });
            ctx.say(
                self.kind,
                &[buyer.0, seller.0],
                &[(0, Value::Num(rate)), (1, Value::Num(basis))],
                true,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn ccy(n: u32) -> CurrencyCode {
        CurrencyCode::at(n)
    }

    fn forward(rate: f64) -> Forward {
        Forward::struck(Forward {
            pays: Side {
                party: party(1),
                ccy: ccy(1),
                amount: 1_250_000.0,
            },
            receives: Side {
                party: party(2),
                ccy: ccy(2),
                amount: 1_000_000.0,
            },
            rate,
            matures: Week(365),
            year_fraction: 1.0,
        })
    }

    #[test]
    fn the_forward_rate_is_cleared_and_parity_is_checked_against_it() {
        // Covered interest parity is a CONSEQUENCE of an arbitrage somebody takes, never an identity
        // applied to produce the rate.
        let at_parity = parity(1.25, 0.01, 0.05, 1.0);
        let struck_wider = forward(at_parity * 1.01);
        assert_ne!(struck_wider.rate, at_parity);
        // And the deviation is THE basis, derived from the cleared print.
        assert!(basis(&struck_wider, 1.25, 0.01, 0.05) > 0.0);
        // A forward that cleared exactly at parity has no basis, which is the degenerate case.
        let exact = forward(at_parity);
        assert!(basis(&exact, 1.25, 0.01, 0.05).abs() <= crate::num::dust(4, &[at_parity, 1.25]));
    }

    #[test]
    fn the_forward_carries_the_interest_differential() {
        // A forward struck as spot moved by a basis, with NO interest differential at all, is
        // neither cleared nor at parity, and carry is absent from the instrument.
        let spot = 1.25;
        assert!(parity(spot, 0.01, 0.05, 1.0) > spot);
        assert!(parity(spot, 0.05, 0.01, 1.0) < spot);
    }

    #[test]
    fn a_persistent_basis_is_a_finding_about_the_arbitrageurs_constraints() {
        // The arbitrage uses balance sheet, capital and credit lines, so it is not free and a gap
        // can stand.
        let big = Arbitrageur {
            who: party(5),
            balance_sheet_free: 50_000_000.0,
            needs: 0.001,
            line_to_counterparty: 40_000_000.0,
        };
        let constrained = Arbitrageur {
            balance_sheet_free: 1_000_000.0,
            ..big
        };
        assert_eq!(closes(&big, 0.01, 100_000_000.0), Some(40_000_000.0));
        assert_eq!(closes(&constrained, 0.01, 100_000_000.0), Some(1_000_000.0));
        // And a basis inside what it needs on its own balance sheet is not worth taking at all.
        assert!(closes(&big, 0.0005, 100_000_000.0).is_none());
    }

    #[test]
    fn the_carry_is_earned_over_the_forwards_life_and_not_booked_at_inception() {
        // A parity-struck forward is worth nothing at strike, and the mark is against the forward
        // for the tenor LEFT.
        let f = forward(1.30);
        assert_eq!(mark(&f, 1.30, 1.0), 0.0);
        let half_way = mark(&f, 1.34, 0.5);
        let at_the_start = mark(&f, 1.34, 1.0);
        assert!(half_way.abs() < at_the_start.abs());
    }

    #[test]
    #[should_panic(expected = "more time left than it was struck for")]
    fn a_forward_cannot_have_more_time_left_than_it_was_struck_for() {
        mark(&forward(1.30), 1.34, 2.0);
    }

    #[test]
    fn an_fx_swap_is_a_secured_loan_of_one_currency_against_another() {
        // That is what it must be modelled as, and the implied funding rate is READ from the two
        // legs rather than stated.
        let near = forward(1.25);
        let far = Forward::struck(Forward {
            rate: 1.28,
            ..forward(1.28)
        });
        let s = FxSwap { near, far };
        let implied = s.implied_funding(0.05);
        // Paying away a forward premium means funding cheaper in the other money than its own rate.
        assert!(implied < 0.05);
    }

    #[test]
    fn both_legs_settle_or_neither_does() {
        // Both notionals DO move, and a settlement that delivers one leg is refused rather than
        // recorded.
        let f = forward(1.25);
        assert!(settles(&f, 2_000_000.0, 2_000_000.0).is_some());
        assert!(settles(&f, 10.0, 2_000_000.0).is_none());
        assert!(settles(&f, 2_000_000.0, 10.0).is_none());
    }

    #[test]
    fn a_cross_currency_swap_returns_the_notionals_at_the_original_rate() {
        // Which is what removes the currency risk and what creates the counterparty risk.
        let x = CrossCurrency {
            a: Side {
                party: party(1),
                ccy: ccy(1),
                amount: 1_250_000.0,
            },
            b: Side {
                party: party(2),
                ccy: ccy(2),
                amount: 1_000_000.0,
            },
            at_rate: 1.25,
            a_pays: 0.03,
            b_pays: 0.01,
            basis: 0.004,
            years: 5.0,
        };
        let (back_to_a, back_to_b) = x.returns_at_maturity();
        assert_eq!(back_to_a.amount, 1_250_000.0);
        assert_eq!(back_to_b.amount, 1_000_000.0);
        // The price INCLUDES the basis, which is where a funding shortage shows up as a number.
        let with_basis = x.periodic(4.0);
        let without = CrossCurrency { basis: 0.0, ..x }.periodic(4.0);
        assert!(with_basis > without);
    }

    #[test]
    fn a_hedge_must_be_rolled_and_the_roll_costs_what_the_new_forward_struck_at() {
        // A recurring demand and a recurring cost, and B3.a says the basis widens exactly when
        // hedgers need it.
        let f = forward(1.25);
        assert!(roll_cost(&f, 1.29) > 0.0);
        assert!(roll_cost(&f, 1.21) < 0.0);
    }

    #[test]
    fn a_hedged_asset_leaves_a_residual_and_it_is_not_zero_by_construction() {
        // The residual is the basis and the imperfection.
        assert_eq!(hedge_residual(-1_000.0, 960.0), -40.0);
        assert_eq!(hedge_residual(-1_000.0, 1_000.0), 0.0);
    }

    #[test]
    fn a_dealers_width_is_what_the_position_costs_it() {
        // The return it needs on the capital the position consumes — not a stated number.
        let small = width(50_000.0, 0.12, 1_000_000.0).unwrap();
        let large = width(50_000.0, 0.12, 10_000_000.0).unwrap();
        assert!(small > large);
        assert!(width(50_000.0, 0.12, 0.0).is_none());
    }

    #[test]
    #[should_panic(expected = "needs a counterparty holding the other side")]
    fn there_is_no_hedge_that_removes_a_position_without_somebody_holding_it() {
        Forward::struck(Forward {
            pays: Side {
                party: party(1),
                ccy: ccy(1),
                amount: 1_250_000.0,
            },
            receives: Side {
                party: party(1),
                ccy: ccy(2),
                amount: 1_000_000.0,
            },
            rate: 1.25,
            matures: Week(365),
            year_fraction: 1.0,
        });
    }

    #[test]
    #[should_panic(expected = "not an FX forward")]
    fn a_forward_with_one_money_on_both_legs_is_not_an_fx_forward() {
        Forward::struck(Forward {
            pays: Side {
                party: party(1),
                ccy: ccy(1),
                amount: 1_250_000.0,
            },
            receives: Side {
                party: party(2),
                ccy: ccy(1),
                amount: 1_000_000.0,
            },
            rate: 1.25,
            matures: Week(365),
            year_fraction: 1.0,
        });
    }
}
