//! SPOT FOREIGN EXCHANGE: an exchange of two amounts in two currencies, both legs settling, at a
//! rate that CLEARS — and no party turns one money into another by itself.
//!
//! @spec 12 A1 · 12 A2 · 12 A3 · 12 B1 · 12 B2 · 12 B3 · 12 B4 · 12 B5 · 12 B5.a · 12 B6 · 12 C1 ·
//! @spec 12 C2 · 12 C3 · 12 C4 · 12 C5 · 12 C6 · 12 D1 · 12 D2 · 12 D3 · 12 D4 · 12 D5 · 12 E1 ·
//! @spec 12 E2 · 12 E3 · 12 E4 · 12 F1 · 12 F1.a · 12 F1.b · XI-12 · Law 3, Law 5, Law 6, Law 19

use crate::ids::{CurrencyCode, InstrumentId, PartyId};
use crate::journal::Value;
use crate::ledger::{account_of, Cause, Delivery, Leg, Receipt, Units};
use crate::module::{Mechanism, MechanismContext};

/// An exchange of two amounts in two currencies, both legs settling — with both parties on it,
/// because E1 says somebody took the other side.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Trade {
    pub buyer: PartyId,
    pub seller: PartyId,
    pub buying: CurrencyCode,
    pub paying_with: CurrencyCode,
    pub bought: f64,
    pub paid: f64,
}

impl Trade {
    /// The rate, quoted one way with its inverse implied — one fact, read from either end.
    pub fn rate(&self) -> Option<f64> {
        if self.bought <= 0.0 {
            return None;
        }
        Some(self.paid / self.bought)
    }
}

/// Participants post schedules in rate space.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Reason {
    /// It owes a currency it does not have — an importer, a foreign-currency borrower, an investor
    /// settling a foreign purchase.
    OwesIt,
    /// It has a currency it does not want — an exporter, a coupon received abroad.
    HasIt,
    /// An investor changing its portfolio's currency mix, for yield or for risk.
    Allocating,
    /// A hedger closing an exposure it took on for another reason.
    Hedging,
    /// A dealer, whose reason is spread and inventory.
    Dealer,
    /// The central bank, for a stated policy reason, with a size and a limit — never as the
    /// residual.
    CentralBank { limit: f64 },
}

#[derive(Clone, Copy, Debug)]
pub struct Posted {
    pub who: PartyId,
    pub reason: Reason,
    /// Positive to buy the base currency, negative to sell it.
    pub quantity: f64,
    /// The worst rate this participant will take.
    pub rate: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cleared {
    pub trades: Vec<(PartyId, PartyId, f64, f64)>,
    /// One rate is in force for the period, and both valuation and settlement use it.
    pub rate: Option<f64>,
    pub unfilled: f64,
}

/// The rate clears where the two sides meet, and imbalance moves it: persistent demand for a
/// currency at the old rate means the old rate was wrong.
pub fn clearing(posted: &[Posted]) -> Cleared {
    let mut buying: Vec<&Posted> = posted.iter().filter(|p| p.quantity > 0.0).collect();
    let mut selling: Vec<&Posted> = posted.iter().filter(|p| p.quantity < 0.0).collect();
    buying.sort_by(|a, b| b.rate.total_cmp(&a.rate));
    selling.sort_by(|a, b| a.rate.total_cmp(&b.rate));

    let mut trades = Vec::new();
    let mut rate = None;
    let mut left: Vec<f64> = selling.iter().map(|s| -s.quantity).collect();
    let mut unfilled = 0.0;

    for b in &buying {
        let mut wants = b.quantity;
        for (at, s) in selling.iter().enumerate() {
            if wants <= 0.0 || left[at] <= 0.0 || s.rate > b.rate || s.who == b.who {
                continue;
            }
            let size = size_of(s, left[at]);
            let taken = if size < wants { size } else { wants };
            if taken <= 0.0 {
                continue;
            }
            trades.push((b.who, s.who, taken, s.rate));
            rate = Some(s.rate);
            left[at] -= taken;
            wants -= taken;
        }
        unfilled += wants;
    }
    Cleared {
        trades,
        rate,
        unfilled,
    }
}

/// What a participant will actually do, which is not always what it posted.
fn size_of(p: &Posted, remaining: f64) -> f64 {
    match p.reason {
        Reason::CentralBank { limit } => {
            if limit < remaining {
                limit
            } else {
                remaining
            }
        }
        _ => remaining,
    }
}

/// The dealer is left with the other side — a real open position in a real currency — and what it
/// does not square it CARRIES, which revalues.
#[derive(Clone, Copy, Debug)]
pub struct Inventory {
    pub dealer: PartyId,
    pub ccy: CurrencyCode,
    pub units: f64,
    pub at_cost: f64,
    /// What it will carry.
    pub limit: f64,
}

impl Inventory {
    /// Whether this desk is still quoting.
    pub fn will_quote(&self, more: f64) -> bool {
        (self.units + more).abs() <= self.limit
    }

    /// The carried position revalues, and that is the risk it is paid the spread for.
    pub fn revalued(&self, rate_now: f64) -> f64 {
        self.units * rate_now - self.at_cost
    }
}

/// Squaring is a trade with a counterparty, not a disappearance.
pub fn square(inv: &Inventory, with: PartyId, units: f64, at_rate: f64) -> Trade {
    assert!(
        with != inv.dealer,
        "12 D2: a dealer cannot square against itself"
    );
    Trade {
        buyer: with,
        seller: inv.dealer,
        buying: inv.ccy,
        paying_with: inv.ccy,
        bought: units,
        paid: units * at_rate,
    }
}

/// A cross is either traded or derived, and if both they must agree — or somebody is arbitraging.
pub fn inconsistency(direct: f64, through_a_vehicle: f64) -> f64 {
    direct - through_a_vehicle
}

/// A party's currency position after the market is exactly what it held plus what it traded, and NO
/// LEG LANDED CONVERTED.
pub fn position_after(who: PartyId, held_before: f64, ccy: CurrencyCode, trades: &[Trade]) -> f64 {
    let mut held = held_before;
    for t in trades {
        if t.buyer == who && t.buying == ccy {
            held += t.bought;
        }
        if t.seller == who && t.buying == ccy {
            held -= t.bought;
        }
        if t.buyer == who && t.paying_with == ccy {
            held -= t.paid;
        }
        if t.seller == who && t.paying_with == ccy {
            held += t.paid;
        }
    }
    held
}

/// Dealer positions and client positions sum to zero in every currency, because every trade has two
/// sides.
pub fn two_sided(trades: &[Trade]) {
    for t in trades {
        assert!(
            t.buyer != t.seller,
            "12 E1: a party cannot turn one money into another by itself"
        );
    }
}

/// A purchase settles in the seller's money.
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode {
    sellers_money
}

/// A CURRENCY PAIR CLEARS FROM REAL REASONS.
pub struct SpotFx {
    pub kind: u32,
}

impl Mechanism for SpotFx {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let to = ctx.closes_week();

        // Who OWES a money, and who HAS one.
        let mut owes: std::collections::HashMap<(u32, u32), f64> = std::collections::HashMap::new();
        let mut has: std::collections::HashMap<(u32, u32), (f64, InstrumentId)> =
            std::collections::HashMap::new();
        for row in 0..ctx.parties().len() as u32 {
            let who = PartyId(row);
            if !ctx.parties().alive(who) {
                continue;
            }
            // What money it banks in.
            let Some(mine) = account_of(ctx.parties(), ctx.instruments(), who) else {
                continue;
            };
            let my_ccy = ctx.instruments().ccy_of(mine).0;
            for &d in ctx.schedules().of_payer(who) {
                let d = crate::stores::DueId(d);
                if ctx.schedules().paid(d) || ctx.schedules().due_week(d) > to {
                    continue;
                }
                let owed_in = ctx.schedules().ccy(d).0;
                if owed_in == my_ccy {
                    continue;
                }
                // It owes a currency it has not got.
                *owes.entry((row, owed_in)).or_insert(0.0) += ctx.schedules().amount(d);
            }
            // And what it holds of a money that is not the one it banks in.
            for &held in ctx.register().of_holder(who) {
                let held = crate::ids::HoldingId(held);
                let line = ctx.register().instrument_of(held);
                if ctx.instruments().class_of(line) != crate::instruments::Class::Money {
                    continue;
                }
                let ccy = ctx.instruments().ccy_of(line).0;
                if ccy == my_ccy {
                    continue;
                }
                let entry = has.entry((row, ccy)).or_insert((0.0, line));
                entry.0 += ctx.register().quantity(held);
            }
        }
        // A foreign balance first performs that party's own obligation. Only the residual is an
        // order: otherwise one party can appear on both sides and manufacture an FX trade with
        // itself.
        let positions: Vec<((u32, u32), f64, f64)> = owes
            .iter()
            .filter_map(|(key, owed)| has.get(key).map(|(held, _)| (*key, *owed, *held)))
            .collect();
        for (key, owed, held) in positions {
            if owed > held {
                owes.insert(key, owed - held);
                has.remove(&key);
            } else if held > owed {
                owes.remove(&key);
                if let Some((amount, _)) = has.get_mut(&key) {
                    *amount = held - owed;
                }
            } else {
                owes.remove(&key);
                has.remove(&key);
            }
        }
        if owes.is_empty() || has.is_empty() {
            return;
        }

        // One book per currency being bought.
        let mut pairs: std::collections::HashMap<u32, Vec<Posted>> =
            std::collections::HashMap::new();
        for (&(who, ccy), &amount) in &owes {
            // The worst rate it will take.
            let Some(line) = has.iter().find_map(|(&(seller, offered), &(_, line))| {
                (seller != who && offered == ccy).then_some(line)
            }) else {
                continue;
            };
            let Some(rate) = ctx.prints().latest(line, ctx.period()).map(|p| p.price) else {
                continue;
            };
            pairs.entry(ccy).or_default().push(Posted {
                who: PartyId(who),
                reason: Reason::OwesIt,
                quantity: amount,
                rate,
            });
        }
        for (&(who, ccy), &(amount, line)) in &has {
            let Some(rate) = ctx.prints().latest(line, ctx.period()).map(|p| p.price) else {
                continue;
            };
            pairs.entry(ccy).or_default().push(Posted {
                who: PartyId(who),
                reason: Reason::HasIt,
                quantity: -amount,
                rate,
            });
        }

        let mut done: Vec<(u32, f64, usize, f64)> = Vec::new();
        let mut exchanges = Vec::new();
        for (&ccy, posted) in &pairs {
            let cleared = clearing(posted);
            // A pair nobody traded has NO rate.
            let Some(rate) = cleared.rate else { continue };
            for &(buyer, seller, bought, at) in &cleared.trades {
                let Some(&(_, bought_money)) = has.get(&(seller.0, ccy)) else {
                    continue;
                };
                let Some(paid_money) = account_of(ctx.parties(), ctx.instruments(), buyer) else {
                    continue;
                };
                exchanges.push((buyer, seller, bought_money, paid_money, bought, bought * at));
            }
            done.push((ccy, rate, cleared.trades.len(), cleared.unfilled));
        }

        for (buyer, seller, bought_money, paid_money, bought, paid) in exchanges {
            let (Some(bought), Some(paid)) = (Units::new(bought), Units::new(paid)) else {
                continue;
            };
            ctx.propose(
                vec![
                    Leg::Money {
                        from: seller,
                        to: buyer,
                        instrument: bought_money,
                        amount: bought,
                        receipt: Receipt::Fx,
                    },
                    Leg::Money {
                        from: buyer,
                        to: seller,
                        instrument: paid_money,
                        amount: paid,
                        receipt: Receipt::Fx,
                    },
                ],
                Cause::Trade,
                Delivery::Nothing,
                "a cleared spot-FX trade exchanges both named monies atomically",
            );
        }

        for (ccy, rate, trades, unfilled) in done {
            // One rate in force for the period, published — both valuation and settlement use it, so
            // it is a fact about the world and not one party's read.
            ctx.say(
                self.kind,
                &[],
                &[
                    (0, Value::Num(rate)),
                    (1, Value::Num(trades as f64)),
                    (2, Value::Num(unfilled)),
                ],
                true,
            );
            let _ = ccy;
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

    fn bid(who: u32, quantity: f64, rate: f64) -> Posted {
        Posted {
            who: party(who),
            reason: Reason::OwesIt,
            quantity,
            rate,
        }
    }

    fn ask(who: u32, quantity: f64, rate: f64) -> Posted {
        Posted {
            who: party(who),
            reason: Reason::HasIt,
            quantity: -quantity,
            rate,
        }
    }

    #[test]
    fn a_pair_nobody_traded_has_no_rate_and_nothing_invents_one() {
        // Not purchasing-power parity, not a differential applied to a level, not a path.
        let apart = [bid(1, 100.0, 1.20), ask(2, 100.0, 1.40)];
        let c = clearing(&apart);
        assert!(c.rate.is_none());
        assert!(c.trades.is_empty());
        assert_eq!(c.unfilled, 100.0);
    }

    #[test]
    fn one_party_cannot_clear_against_its_own_currency_order() {
        let crossed = clearing(&[bid(1, 100.0, 1.30), ask(1, 100.0, 1.20)]);
        assert!(crossed.trades.is_empty());
        assert!(crossed.rate.is_none());
        assert_eq!(crossed.unfilled, 100.0);
    }

    #[test]
    fn imbalance_moves_the_rate() {
        // Persistent demand for a currency at the old rate means the old rate was wrong.
        let thin = [bid(1, 60.0, 1.40), ask(2, 100.0, 1.20), ask(3, 100.0, 1.35)];
        let heavy = [
            bid(1, 160.0, 1.40),
            ask(2, 100.0, 1.20),
            ask(3, 100.0, 1.35),
        ];
        assert_eq!(clearing(&thin).rate, Some(1.20));
        assert_eq!(clearing(&heavy).rate, Some(1.35));
    }

    #[test]
    fn the_central_bank_participates_with_a_size_and_a_limit_and_is_never_the_residual() {
        // It is one schedule among others.
        let posted = [
            bid(1, 500.0, 1.40),
            Posted {
                who: party(9),
                reason: Reason::CentralBank { limit: 120.0 },
                quantity: -500.0,
                rate: 1.30,
            },
        ];
        let c = clearing(&posted);
        assert_eq!(c.trades.len(), 1);
        assert_eq!(c.trades[0].2, 120.0);
        assert_eq!(c.unfilled, 380.0);
    }

    #[test]
    fn a_dealer_is_left_with_the_other_side_and_squaring_names_a_counterparty() {
        // Squaring is a trade, not a disappearance — the position moves to somebody named.
        let inv = Inventory {
            dealer: party(5),
            ccy: ccy(2),
            units: 400.0,
            at_cost: 480.0,
            limit: 1_000.0,
        };
        let t = square(&inv, party(6), 400.0, 1.25);
        assert_eq!(t.seller, party(5));
        assert_eq!(t.buyer, party(6));
        assert_eq!(t.paid, 500.0);
    }

    #[test]
    fn what_a_dealer_carries_revalues_and_that_is_what_it_is_paid_the_spread_for() {
        let inv = Inventory {
            dealer: party(5),
            ccy: ccy(2),
            units: 400.0,
            at_cost: 480.0,
            limit: 1_000.0,
        };
        assert!(inv.revalued(1.25) > 0.0);
        assert!(inv.revalued(1.10) < 0.0);
    }

    #[test]
    fn a_dealer_at_its_limit_stops_quoting_rather_than_absorbing_more() {
        // It is NOT obliged to take whatever arrives, and a limit that never binds is not a limit.
        let inv = Inventory {
            dealer: party(5),
            ccy: ccy(2),
            units: 900.0,
            at_cost: 1_080.0,
            limit: 1_000.0,
        };
        assert!(inv.will_quote(50.0));
        assert!(!inv.will_quote(500.0));
    }

    #[test]
    fn no_leg_lands_converted_and_a_position_is_what_was_held_plus_what_was_traded() {
        // Both legs settle, and the party that bought one money gave up the other to a named
        // counterparty.
        let t = Trade {
            buyer: party(1),
            seller: party(2),
            buying: ccy(2),
            paying_with: ccy(1),
            bought: 100.0,
            paid: 125.0,
        };
        assert_eq!(position_after(party(1), 0.0, ccy(2), &[t]), 100.0);
        assert_eq!(position_after(party(1), 500.0, ccy(1), &[t]), 375.0);
        assert_eq!(position_after(party(2), 0.0, ccy(2), &[t]), -100.0);
        assert_eq!(position_after(party(2), 0.0, ccy(1), &[t]), 125.0);
        // And the two sides sum to zero in each currency, because every trade has two of them.
        let in_two = position_after(party(1), 0.0, ccy(2), &[t])
            + position_after(party(2), 0.0, ccy(2), &[t]);
        assert_eq!(in_two, 0.0);
        two_sided(&[t]);
    }

    #[test]
    fn the_rate_is_one_fact_read_from_either_end() {
        // Quoted one way with its inverse implied.
        let t = Trade {
            buyer: party(1),
            seller: party(2),
            buying: ccy(2),
            paying_with: ccy(1),
            bought: 100.0,
            paid: 125.0,
        };
        assert_eq!(t.rate(), Some(1.25));
        let nothing = Trade { bought: 0.0, ..t };
        assert!(nothing.rate().is_none());
    }

    #[test]
    fn a_cross_that_disagrees_with_the_direct_route_is_measured_and_not_corrected() {
        assert!(inconsistency(1.25, 1.24).abs() > 0.0);
        assert_eq!(inconsistency(1.25, 1.25), 0.0);
    }

    #[test]
    fn a_purchase_settles_in_the_sellers_money() {
        // One purchase, one rule, owned in one place.
        assert_eq!(settles_in(ccy(2)), ccy(2));
    }

    #[test]
    #[should_panic(expected = "by itself")]
    fn a_party_cannot_turn_one_money_into_another_by_itself() {
        // Converting inside a trade means the buyer is never short and no order is ever placed — the
        // currency demand the trade should have created disappears.
        let alone = Trade {
            buyer: party(1),
            seller: party(1),
            buying: ccy(2),
            paying_with: ccy(1),
            bought: 100.0,
            paid: 125.0,
        };
        two_sided(&[alone]);
    }

    #[test]
    #[should_panic(expected = "square against itself")]
    fn a_dealer_cannot_square_against_itself() {
        let inv = Inventory {
            dealer: party(5),
            ccy: ccy(2),
            units: 400.0,
            at_cost: 480.0,
            limit: 1_000.0,
        };
        square(&inv, party(5), 400.0, 1.25);
    }
}
