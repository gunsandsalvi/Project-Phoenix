//! HEDGE FUNDS: leverage is a fact about a loan, everything marks at cleared prices, and the fund
//! can fail.
//!
//! @spec 28 A1 · 28 A2 · 28 A3 · 28 A4 · 28 A5 · 28 B1 · 28 B1.a · 28 B2 · 28 B3 · 28 B4 · 28 B5 ·
//! @spec 28 C1 · 28 C2 · 28 C3 · 28 C4 · 28 D1 · 28 D2 · 28 D3 · 28 D4 · 28 D4.a · 28 D5 · 28 D5.a ·
//! @spec 28 D6 · 28 D7 · 28 E1 · 28 E2 · 28 E3 · XI-2 · Law 3, Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::agreed;

/// A named party whose investor capital is EQUITY — the investors bear the result, and they hold a
/// share count (XI-15's redeemable claim).
#[derive(Clone, Debug)]
pub struct Fund {
    pub who: PartyId,
    /// A manager is a separate party, earning a fee on assets and a share of the gains.
    pub manager: PartyId,
    /// Everything is marked to market at cleared prices, so its equity moves continuously.
    pub positions: Vec<Position>,
    /// What it borrowed, and from whom.
    pub borrowings: Vec<Borrowing>,
    pub cash: f64,
    pub shares: f64,
}

/// A mandate that is wide — long, short, levered, in many markets — and positions taken for REASONS:
/// a relative-value view, a directional view, a liquidity view.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position {
    pub what: InstrumentId,
    /// Negative is short, which requires a borrow.
    pub units: f64,
    pub cost: f64,
    /// No position that does not mark.
    pub price: Option<f64>,
}

impl Position {
    pub fn value(&self) -> Option<f64> {
        Some(self.units * self.price?)
    }
}

/// Leverage is a fact about a loan, from a NAMED lender — never a property of the fund.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Borrowing {
    pub from: PartyId,
    pub amount: f64,
    /// The amount available is the LENDER'S decision, and it changes.
    pub available: f64,
    pub kind: Levered,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MarginFunding {
    pub lender: PartyId,
    pub fund: PartyId,
    pub amount: f64,
}

pub fn fund_margin(f: &Fund, called: f64) -> Option<Vec<MarginFunding>> {
    if called <= 0.0 {
        return None;
    }
    let available: f64 = f
        .borrowings
        .iter()
        .map(|b| b.available - b.amount)
        .filter(|room| *room > 0.0)
        .sum();
    if available < called {
        return None;
    }
    let mut left = called;
    let mut routed = Vec::new();
    for borrowing in &f.borrowings {
        let room = borrowing.available - borrowing.amount;
        if room <= 0.0 || left <= 0.0 {
            continue;
        }
        let amount = if room < left { room } else { left };
        routed.push(MarginFunding {
            lender: borrowing.from,
            fund: f.who,
            amount,
        });
        left -= amount;
    }
    Some(routed)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Levered {
    Margin,
    Derivative,
    Repo,
}

impl Fund {
    /// What it is worth, at cleared prices.
    pub fn equity(&self) -> Option<f64> {
        let mut held = self.cash;
        for p in &self.positions {
            held += p.value()?;
        }
        Some(held - self.borrowings.iter().map(|b| b.amount).sum::<f64>())
    }

    /// Gross exposure, net exposure and equity are three different reads, and all three are needed.
    pub fn gross(&self) -> Option<f64> {
        let mut total = 0.0;
        for p in &self.positions {
            total += p.value()?.abs();
        }
        Some(total)
    }

    pub fn net(&self) -> Option<f64> {
        let mut total = 0.0;
        for p in &self.positions {
            total += p.value()?;
        }
        Some(total)
    }

    /// Read from the loans, against the equity.
    pub fn leverage(&self) -> Option<f64> {
        let equity = self.equity()?;
        if equity <= 0.0 {
            return None;
        }
        Some(self.borrowings.iter().map(|b| b.amount).sum::<f64>() / equity)
    }
}

/// A loss reduces equity, and with fixed borrowing LEVERAGE RISES.
pub fn after_a_loss(f: &Fund, marked_down_by: f64) -> Option<f64> {
    let equity = f.equity()? - marked_down_by;
    if equity <= 0.0 {
        return None;
    }
    Some(f.borrowings.iter().map(|b| b.amount).sum::<f64>() / equity)
}

/// The lender calls margin, and meeting it requires selling at market prices — which moves prices.
#[derive(Clone, Debug, PartialEq)]
pub struct Round {
    pub called: f64,
    pub sold: f64,
    /// What the selling did to the price — the market's depth answering the size, not a parameter.
    pub moved_price_by: f64,
    /// Who else is now short, by name.
    pub reaches: Vec<(PartyId, f64)>,
}

pub fn spiral(
    f: &Fund,
    requirement: f64,
    depth: f64,
    others: &[(PartyId, f64, f64)],
) -> Option<Round> {
    assert!(
        depth > 0.0,
        "28 D3: a sale into a market with no depth has no price to move"
    );
    let equity = f.equity()?;
    let called = requirement - equity;
    if called <= 0.0 {
        return None;
    }
    // It sells to meet the call — it cannot sell more than it holds, which is arithmetic.
    let holds = f.gross()?;
    let sold = if holds < called { holds } else { called };
    let moved = sold / depth;
    // And the move hits everybody holding the same positions, levered or not.
    let reaches = others
        .iter()
        .filter_map(|(who, holding, their_equity)| {
            let hit = holding * moved;
            if hit > *their_equity {
                Some((*who, hit - their_equity))
            } else {
                None
            }
        })
        .collect();
    Some(Round {
        called,
        sold,
        moved_price_by: moved,
        reaches,
    })
}

/// It will be the buyer when others are forced sellers, IF it has capacity — which is what makes it
/// liquidity.
pub fn will_buy(f: &Fund, offered: f64, its_view_says_yes: bool) -> Option<f64> {
    if !its_view_says_yes {
        return None;
    }
    let room: f64 = f
        .borrowings
        .iter()
        .map(|b| b.available - b.amount)
        .sum::<f64>()
        + f.cash;
    if room <= 0.0 {
        return None;
    }
    Some(if room < offered { room } else { offered })
}

/// Investor redemptions arrive at the same time, for the same reason — and a gate or notice week
/// delays it, which is a real contractual term with real consequences, not a refusal.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Redemption {
    Paid {
        to: PartyId,
        amount: f64,
    },
    /// Delayed to a stated week.
    Gated {
        to: PartyId,
        amount: f64,
        until_period: u32,
    },
}

pub fn redeem(
    f: &Fund,
    holder: PartyId,
    shares: f64,
    gate_until: Option<u32>,
) -> Option<Redemption> {
    let equity = f.equity()?;
    if f.shares <= 0.0 {
        return None;
    }
    let amount = equity * shares / f.shares;
    Some(match gate_until {
        Some(until_period) => Redemption::Gated {
            to: holder,
            amount,
            until_period,
        },
        None => Redemption::Paid { to: holder, amount },
    })
}

/// The fund can fail, and then its broker eats the shortfall and its investors lose their capital.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Failed {
    pub fund: PartyId,
    /// What the broker could not recover — its loss, on its own capital.
    pub broker_eats: f64,
    pub investors_lose: f64,
}

pub fn fails(f: &Fund, collateral_fetched: f64) -> Failed {
    let lent: f64 = f.borrowings.iter().map(|b| b.amount).sum();
    let short = lent - collateral_fetched;
    Failed {
        fund: f.who,
        broker_eats: if short > 0.0 { short } else { 0.0 },
        investors_lose: match f.equity() {
            Some(e) if e > 0.0 => e,
            // Equity already gone, or unmarkable: there was nothing left for them either way.
            _ => 0.0,
        },
    }
}

// §14 RUNS HERE.

/// A FUND MARKS, AND ITS LEVERAGE IS A READ AGAINST WHAT IT BORROWED.
pub struct Levering {
    pub kind: u32,
    pub at_equity: u32,
}

impl Mechanism for Levering {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let mut marked: Vec<(PartyId, f64, Option<f64>)> = Vec::new();
        for &fund in ctx.parties().of_kind(kinds::FUND) {
            let who = PartyId(fund);
            if !ctx.parties().alive(who) {
                continue;
            }
            // A fund whose book cannot be valued does not lever against it.
            let Some(at_market) = crate::instruments::market_book_value(
                who,
                ctx.register(),
                ctx.instruments(),
                &ctx.marks(),
                ctx.week(),
            ) else {
                continue;
            };
            // What it borrowed, from the named lender that lent it.
            let lent: f64 = ctx
                .agreements()
                .of_party(who)
                .iter()
                .map(|a| crate::stores::AgreementId(*a))
                .filter(|a| {
                    ctx.agreements().live(*a)
                        && ctx.agreements().kind_of(*a) == agreed::PRIME_BROKERAGE
                })
                .filter_map(|a| match ctx.agreements().terms(a) {
                    crate::stores::AgreementTerms::PrimeBrokerage { limit, .. } => Some(*limit),
                    _ => None,
                })
                .sum();
            let equity = at_market - lent;
            // `None` where the client has no equity left — which is not zero leverage, it is a
            // client that is gone.
            let leverage = if equity > 0.0 {
                Some(lent / equity)
            } else {
                None
            };
            marked.push((who, equity, leverage));
        }

        for (who, equity, leverage) in marked {
            let mut data = vec![(self.at_equity, Value::Num(equity))];
            if let Some(l) = leverage {
                data.push((0, Value::Num(l)));
            }
            // What a fund is worth is its holders' business and its lender's, not the world's.
            ctx.say(self.kind, &[who.0], &data, false);
        }
    }
}

// And the party that posts for it.

/// A FUND IS THE BUYER WHEN OTHERS ARE FORCED SELLERS.
pub struct Liquidity {
    pub of_kind: u32,
}

impl crate::module::Participant for Liquidity {
    /// A fund that meets redemptions out of its book marks it: what it can raise is what it is
    /// worth today, not what it paid.
    fn carries(
        &self,
        _view: &crate::module::ParticipantView<'_>,
        _m: crate::ids::MarketId,
    ) -> Option<crate::register::Carrying> {
        Some(crate::register::Carrying::Market)
    }

    fn party_kind(&self) -> u32 {
        self.of_kind
    }

    fn markets(&self, view: &crate::module::ParticipantView<'_>) -> Vec<crate::ids::MarketId> {
        // IF it has capacity.
        if view.own_cash() <= 0.0 {
            return Vec::new();
        }
        // The lines it knows — its own rows — never every book in the world.
        view.holdings()
            .filter_map(|row| view.market_of(view.line_of(row)))
            .collect()
    }

    fn orders(
        &self,
        view: &crate::module::ParticipantView<'_>,
        m: crate::ids::MarketId,
    ) -> Vec<crate::clearing::Order> {
        let room = view.own_cash();
        if room <= 0.0 {
            return Vec::new();
        }
        let Some(line) = view.subject_of(m) else {
            return Vec::new();
        };
        // 46 F2: it buys from somebody who must sell, and what it will pay is what the line is
        // worth to IT. Bidding at the last print would be the book's own answer used as the reason
        // for its next one, and a fund with no value of its own has no reason to be here.
        let Some(worth) = view.values(line) else {
            return Vec::new();
        };
        if worth <= 0.0 {
            return Vec::new();
        }
        let units = crate::clearing::whole_pieces(room / worth);
        if units <= 0 {
            return Vec::new();
        }
        vec![crate::clearing::Order {
            party: view.self_id(),
            side: crate::clearing::Side::Buy,
            price: Some(worth),
            qty: units,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(n: u32) -> PartyId {
        PartyId::at(n)
    }

    fn fund() -> Fund {
        Fund {
            who: party(60),
            manager: party(61),
            positions: vec![
                Position {
                    what: InstrumentId::at(1),
                    units: 1_000.0,
                    cost: 900.0,
                    price: Some(1.0),
                },
                Position {
                    what: InstrumentId::at(2),
                    units: -400.0,
                    cost: -380.0,
                    price: Some(1.0),
                },
            ],
            borrowings: vec![Borrowing {
                from: party(80),
                amount: 500.0,
                available: 900.0,
                kind: Levered::Margin,
            }],
            cash: 200.0,
            shares: 100.0,
        }
    }

    #[test]
    fn leverage_is_a_fact_about_a_loan_and_the_lender_is_named() {
        // Never a property of the fund.
        let f = fund();
        assert_eq!(f.borrowings[0].from, party(80));
        assert_eq!(f.equity(), Some(300.0));
        assert!(
            (f.leverage().unwrap() - 500.0 / 300.0).abs() <= crate::num::dust(2, &[500.0, 300.0])
        );
    }

    #[test]
    fn a_position_that_does_not_mark_makes_the_whole_fund_unmarkable() {
        // A fund carrying an unmarked position has hidden its loss, and this refuses to let it hide
        // — the equity is unavailable rather than partly invented.
        let mut f = fund();
        f.positions[0].price = None;
        assert!(f.equity().is_none());
        assert!(f.gross().is_none());
        assert!(f.leverage().is_none());
    }

    #[test]
    fn gross_net_and_equity_are_three_different_reads() {
        // Gross alone hides the hedging; net alone hides the size.
        let f = fund();
        assert_eq!(f.gross(), Some(1_400.0));
        assert_eq!(f.net(), Some(600.0));
        assert_eq!(f.equity(), Some(300.0));
    }

    #[test]
    fn a_loss_raises_leverage_because_the_borrowing_did_not_move() {
        // The first step of the loop, and the reason the rest follows.
        let f = fund();
        let before = f.leverage().unwrap();
        let after = after_a_loss(&f, 150.0).unwrap();
        assert!(after > before);
        // And a loss past the equity leaves no leverage to read — the fund is gone.
        assert!(after_a_loss(&f, 400.0).is_none());
    }

    #[test]
    fn the_spiral_is_emergent_from_the_call_the_sale_and_the_price_move() {
        // Never a contagion coefficient — and the chain is traceable party by party.
        let f = fund();
        let others = [(party(62), 5_000.0, 200.0), (party(63), 100.0, 5_000.0)];
        let r = spiral(&f, 900.0, 10_000.0, &others).unwrap();
        assert_eq!(r.called, 600.0);
        assert_eq!(r.sold, 600.0);
        assert!(r.moved_price_by > 0.0);
        // The levered holder of the same position is now short; the unlevered one is not.
        assert_eq!(r.reaches.len(), 1);
        assert_eq!(r.reaches[0].0, party(62));
        // A fund inside its requirement is not called at all.
        assert!(spiral(&f, 100.0, 10_000.0, &others).is_none());
    }

    #[test]
    fn it_buys_when_others_are_forced_sellers_only_if_it_has_capacity() {
        // Which is what makes it liquidity — and the case that matters is when it has none.
        let f = fund();
        assert_eq!(will_buy(&f, 300.0, true), Some(300.0));
        assert_eq!(will_buy(&f, 5_000.0, true), Some(600.0));
        // Its own view says no: it is a participant, not a mechanism.
        assert!(will_buy(&f, 300.0, false).is_none());
        // Drawn to its line and out of cash, it cannot be the buyer however attractive the price.
        let tapped = Fund {
            cash: 0.0,
            borrowings: vec![Borrowing {
                from: party(80),
                amount: 900.0,
                available: 900.0,
                kind: Levered::Margin,
            }],
            ..fund()
        };
        assert!(will_buy(&tapped, 300.0, true).is_none());
    }

    #[test]
    fn a_gate_delays_a_redemption_and_does_not_extinguish_it() {
        // A real contractual term with real consequences.
        let f = fund();
        assert_eq!(
            redeem(&f, party(70), 10.0, None),
            Some(Redemption::Paid {
                to: party(70),
                amount: 30.0
            })
        );
        assert_eq!(
            redeem(&f, party(70), 10.0, Some(14)),
            Some(Redemption::Gated {
                to: party(70),
                amount: 30.0,
                until_period: 14
            })
        );
    }

    #[test]
    fn the_fund_can_fail_and_the_broker_eats_the_shortfall() {
        // A vehicle that absorbs losses indefinitely is the buyer of last resort under another name.
        let f = fund();
        let clean = fails(&f, 800.0);
        assert_eq!(clean.broker_eats, 0.0);
        let messy = fails(&f, 320.0);
        assert_eq!(messy.broker_eats, 180.0);
        assert_eq!(messy.investors_lose, 300.0);
    }

    #[test]
    #[should_panic(expected = "no price to move")]
    fn a_sale_into_a_market_with_no_depth_has_no_price_to_move() {
        spiral(&fund(), 900.0, 0.0, &[]);
    }
}
