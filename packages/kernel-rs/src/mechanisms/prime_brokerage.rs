//! PRIME BROKERAGE: the client's leverage is a loan from a named lender, the margin requirement is a
//! decision by the broker, and the available line is never floored at zero.
//!
//! @spec 15 A1 · 15 A2 · 15 A3 · 15 A4 · 15 B1 · 15 B1.a · 15 B2 · 15 B3 · 15 B4 · 15 B5 · 15 C1 ·
//! @spec 15 C1.a · 15 C1.b · 15 C2 · 15 C3 · 15 C3.a · 15 C3.b · 15 C4 · 15 C4.a · 15 C5 · 15 D1 ·
//! @spec 15 D2 · 15 D3 · 15 D4 · 15 E1 · 15 E2 · 15 E3 · 15 E4 · XI-2 · Law 5, Law 6, Law 19

use crate::assembly::kinds;
use crate::ids::{InstrumentId, PartyId};
use crate::journal::Value;
use crate::module::{Mechanism, MechanismContext};
use crate::stores::{afoot, agreed};

/// A named bank and a named client, where the broker holds the client's assets and knows the whole
/// position it holds — that knowledge is what lets it lend against them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Account {
    pub broker: PartyId,
    pub client: PartyId,
    /// What this broker holds for the client, at market.
    pub assets: f64,
    /// The client's leverage is a loan from a NAMED lender, not a property of the client.
    pub lent: f64,
    /// The short side is financed too — proceeds of a short are held, and stock is borrowed.
    pub short_proceeds_held: f64,
    pub stock_borrowed: f64,
    /// The broker has an exposure per client, and it should know it — and no unlimited exposure: a
    /// broker with no limit is a synthetic counterparty.
    pub limit: f64,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FinancedPosition {
    pub account: Account,
    pub position: InstrumentId,
    pub collateral: InstrumentId,
    pub collateral_units: f64,
}

impl FinancedPosition {
    pub fn new(
        account: Account,
        position: InstrumentId,
        collateral: InstrumentId,
        collateral_units: f64,
    ) -> Option<Self> {
        if collateral_units <= 0.0 || position == collateral {
            return None;
        }
        Some(Self {
            account,
            position,
            collateral,
            collateral_units,
        })
    }
}

impl Account {
    /// The client's leverage is a read of borrowed against equity, and it must equal what the broker
    /// has lent.
    pub fn leverage(&self) -> Option<f64> {
        let equity = self.assets - self.lent;
        if equity <= 0.0 {
            return None;
        }
        Some(self.lent / equity)
    }

    /// What this broker is exposed to.
    pub fn exposure(&self) -> f64 {
        self.lent + self.stock_borrowed - self.short_proceeds_held
    }

    /// And whether that is within the limit it set.
    pub fn within_limit(&self) -> bool {
        self.exposure() <= self.limit
    }
}

/// The broker sets a margin requirement on the whole portfolio, from its own view of the risk — a
/// DECISION by the broker, not a formula the client can rely on.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// What the broker thinks the book could move, this week.
    pub move_it_expects: f64,
    /// What it adds when it likes what it sees less — worse markets, worse client, worse own
    /// position.
    pub add_for_the_client: f64,
}

/// The requirement, accounting for offsetting positions — so a hedged book requires less than the
/// sum of its legs.
pub fn requirement(long: f64, short: f64, v: &View) -> f64 {
    // The legs that offset each other are not risk the broker is carrying; what is left over is.
    let net = (long - short).abs();
    let gross = long + short;
    // Some of the gross remains a requirement even when the book nets: the two legs can move apart.
    let unhedged = net + (gross - net) * v.add_for_the_client;
    unhedged * v.move_it_expects
}

/// The requirement is remeasured as prices move, and a shortfall is a margin call.
pub fn headroom(a: &Account, required: f64) -> f64 {
    (a.assets - a.lent) - required
}

/// Real money, from the client's account, now — or the client is liquidated.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Call {
    /// A met call MOVES CASH, from the client to the broker.
    Met {
        from: PartyId,
        to: PartyId,
        amount: f64,
    },
    /// To meet it, the client may have to SELL, into a market that must clear.
    MustSell {
        who: PartyId,
        raising: f64,
    },
    /// It failed to meet the call, and the broker closes the positions.
    Liquidated {
        who: PartyId,
        short_by: f64,
    },
    None,
}

pub fn call(a: &Account, required: f64, client_cash: f64, client_can_sell: f64) -> Call {
    let short = -headroom(a, required);
    if short <= 0.0 {
        return Call::None;
    }
    if client_cash >= short {
        return Call::Met {
            from: a.client,
            to: a.broker,
            amount: short,
        };
    }
    if client_cash + client_can_sell >= short {
        return Call::MustSell {
            who: a.client,
            raising: short - client_cash,
        };
    }
    Call::Liquidated {
        who: a.client,
        short_by: short - client_cash - client_can_sell,
    }
}

/// The broker closes the positions, selling collateral at MARKET prices, and the proceeds may be
/// less than the loan — the shortfall is the broker's loss, hitting its capital.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Liquidation {
    pub broker: PartyId,
    pub fetched: f64,
    pub loan_was: f64,
}

impl Liquidation {
    /// The loss lands on the broker's capital.
    pub fn loss(&self) -> f64 {
        self.loan_was - self.fetched
    }
}

/// The liquidation is a real sale into a real market, so it MOVES PRICES, which can margin-call
/// other clients.
pub fn reaches(sold: f64, depth: f64, others: &[Account], v: &View) -> Vec<(PartyId, f64)> {
    assert!(
        depth > 0.0,
        "15 D3: a sale into a market with no depth has no price to move"
    );
    // What the sale did to the price, and therefore to every other client's assets.
    let moved = sold / depth;
    others
        .iter()
        .filter_map(|o| {
            let marked_down = Account {
                assets: o.assets * (1.0 - moved),
                ..*o
            };
            let required = requirement(marked_down.assets, 0.0, v);
            let short = -headroom(&marked_down, required);
            if short > 0.0 {
                Some((o.client, short))
            } else {
                None
            }
        })
        .collect()
}

/// The client can have more than one broker, and then no broker sees the whole position — a real and
/// material blind spot.
pub fn concentration_seen_by(
    broker: PartyId,
    in_one_name: &[(PartyId, f64)],
    market_depth: f64,
) -> Option<f64> {
    if market_depth <= 0.0 {
        return None;
    }
    let mine = in_one_name.iter().find(|(b, _)| *b == broker)?.1;
    Some(mine / market_depth)
}

/// The number nobody in the world holds: it needs every broker's row at once, and no participant has
/// them.
pub fn true_concentration(in_one_name: &[(PartyId, f64)], market_depth: f64) -> Option<f64> {
    if market_depth <= 0.0 {
        return None;
    }
    Some(in_one_name.iter().map(|(_, units)| units).sum::<f64>() / market_depth)
}

/// What one broker computes for its own book.
pub fn leverage_seen_by(broker: PartyId, accounts: &[Account]) -> Option<f64> {
    accounts.iter().find(|a| a.broker == broker)?.leverage()
}

/// The broker earns from financing spread, stock-borrow fees and commissions, and that income is a
/// reason for it to take the risk — and its balance sheet GROWS by the loan, which consumes its
/// capital and its liquidity.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Earns {
    pub financing: f64,
    pub stock_borrow: f64,
    pub commissions: f64,
}

pub fn earns(
    a: &Account,
    lends_at: f64,
    own_cost_of_funds: f64,
    borrow_fee: f64,
    commission: f64,
) -> Earns {
    assert!(
        lends_at > own_cost_of_funds,
        "15 B2: a loan at or below the broker's own cost of funds earns it nothing to be at risk for"
    );
    Earns {
        financing: a.lent * (lends_at - own_cost_of_funds),
        stock_borrow: a.stock_borrowed * borrow_fee,
        commissions: commission,
    }
}

/// A BROKER LENDS TO A NAMED CLIENT, AND SETS WHAT IT REQUIRES.
pub struct Broking {
    pub kind: u32,
    /// What the broker thinks the book could move this week.
    pub could_move: &'static str,
    /// What one broker will be exposed to one client for.
    pub limit: &'static str,
}

impl Mechanism for Broking {
    fn run(&self, ctx: &mut MechanismContext<'_>) {
        let could_move = ctx.params().ratio(self.could_move);
        let limit = ctx
            .params()
            .amount(self.limit, crate::params::Denomination::Money);

        let brokers: Vec<PartyId> = ctx
            .parties()
            .of_kind(kinds::DEALER)
            .iter()
            .map(|p| PartyId(*p))
            .filter(|p| ctx.parties().alive(*p))
            .collect();
        if brokers.is_empty() {
            return;
        }

        let mut opening: Vec<(PartyId, PartyId, f64, f64)> = Vec::new();
        let mut calling: Vec<(PartyId, PartyId, InstrumentId, f64)> = Vec::new();
        for (n, &client) in ctx.parties().of_kind(kinds::FUND).iter().enumerate() {
            let client = PartyId(client);
            if !ctx.parties().alive(client) {
                continue;
            }
            // One broker per client here — a client with two brokers is real and is §12 E3's, which
            // needs each to see only its own book.
            let broker = brokers[n % brokers.len()];
            // The broker sets a margin requirement on the whole portfolio, from its own view of the
            // risk — so a portfolio it cannot value is one it cannot margin.
            let mut assets = 0.0;
            let mut priced = true;
            for &row in ctx.register().of_holder(client) {
                let row = crate::ids::HoldingId(row);
                let line = ctx.register().instrument_of(row);
                // Money is not collateral a broker margins; it is what the margin is paid in.
                if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                    continue;
                }
                match crate::instruments::worth(
                    row,
                    ctx.register(),
                    ctx.instruments(),
                    &ctx.marks(),
                    ctx.week(),
                ) {
                    Some(value) => assets += value,
                    None => priced = false,
                }
            }
            if !priced || assets <= 0.0 {
                continue;
            }
            // What this broker has already lent it, read off the relation it holds.
            let held = ctx
                .agreements()
                .of_party(client)
                .iter()
                .map(|a| crate::stores::AgreementId(*a))
                .find(|a| {
                    ctx.agreements().live(*a)
                        && ctx.agreements().kind_of(*a) == agreed::PRIME_BROKERAGE
                });
            // What this broker has already lent it.
            let lent = match held {
                Some(a) => match ctx.agreements().terms(a) {
                    crate::stores::AgreementTerms::PrimeBrokerage { lent, .. } => *lent,
                    _ => continue,
                },
                None => 0.0,
            };
            let account = Account {
                broker,
                client,
                assets,
                lent,
                short_proceeds_held: 0.0,
                stock_borrowed: 0.0,
                limit,
            };
            // The requirement, from the broker's OWN view of what the book could move.
            let view = View {
                move_it_expects: could_move,
                add_for_the_client: could_move,
            };
            let required = requirement(assets, 0.0, &view);
            if held.is_none() {
                opening.push((broker, client, lent, limit));
            }
            // And where the account is short of what the broker requires, it CALLS.
            let headroom = headroom(&account, required);
            if headroom < 0.0 {
                let mut remaining = -headroom;
                for row in ctx.register().of_holder(client) {
                    let holding = crate::ids::HoldingId(*row);
                    let line = ctx.register().instrument_of(holding);
                    if ctx.instruments().class_of(line) == crate::instruments::Class::Money {
                        continue;
                    }
                    let Some(price) = ctx
                        .prints()
                        .of_line(line, ctx.week())
                        .map(|print| print.price)
                    else {
                        continue;
                    };
                    if price <= 0.0 || remaining <= 0.0 {
                        continue;
                    }
                    let free = ctx.register().free(holding);
                    let needed = remaining / price;
                    let units = if free < needed { free } else { needed };
                    if units > 0.0 {
                        calling.push((broker, client, line, units));
                        remaining -= units * price;
                    }
                }
            }
        }

        for (broker, client, lent, limit) in opening {
            // A loan from a NAMED lender.
            ctx.agrees(crate::module::Agrees {
                kind: agreed::PRIME_BROKERAGE,
                one: broker,
                other: client,
                terms: crate::stores::AgreementTerms::PrimeBrokerage { lent, limit },
                until: None,
            });
            ctx.say(
                self.kind,
                &[broker.0, client.0],
                &[(0, Value::Num(limit))],
                false,
            );
        }
        for (broker, client, line, units) in calling {
            // A margin call the client cannot meet from cash is the first of the four doors, and it
            // is a WORKOUT — the client must find the money or sell.
            ctx.opens(crate::module::Opens {
                kind: afoot::WORKOUT,
                owner: client,
                subject: Some(line),
                door: Some(crate::stores::WorkoutDoor::MarginCall as u32),
                closes: Some(ctx.week() + 1),
                size: units,
            });
            ctx.say(
                self.kind,
                &[broker.0, client.0, line.0],
                &[(0, Value::Num(-units))],
                false,
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

    fn account(broker: u32, client: u32, assets: f64, lent: f64) -> Account {
        Account {
            broker: party(broker),
            client: party(client),
            assets,
            lent,
            short_proceeds_held: 0.0,
            stock_borrowed: 0.0,
            limit: 10_000.0,
        }
    }

    fn view() -> View {
        View {
            move_it_expects: 0.10,
            add_for_the_client: 0.2,
        }
    }

    #[test]
    fn the_clients_leverage_is_a_loan_from_a_named_lender() {
        // Not a property of the client — a read of borrowed against equity, equal to what the broker
        // has lent.
        let a = account(80, 50, 1_000.0, 600.0);
        assert_eq!(a.leverage(), Some(1.5));
        // A client with no equity left is not at zero leverage; it is gone.
        assert!(account(80, 50, 600.0, 600.0).leverage().is_none());
    }

    #[test]
    fn a_hedged_book_requires_less_than_the_sum_of_its_legs() {
        // Offsets are accounted for, which is what makes the requirement a view of the portfolio
        // rather than a sum of tickets.
        let hedged = requirement(1_000.0, 900.0, &view());
        let directional = requirement(1_000.0, 0.0, &view());
        assert!(hedged < directional);
    }

    #[test]
    fn the_requirement_rises_when_the_broker_likes_what_it_sees_less() {
        // A decision by the broker, not a formula the client can rely on — and a stated constant
        // could not rise when it matters, which deletes the procyclicality.
        let calm = requirement(1_000.0, 900.0, &view());
        let worried = requirement(
            1_000.0,
            900.0,
            &View {
                move_it_expects: 0.25,
                add_for_the_client: 0.6,
            },
        );
        assert!(worried > calm);
    }

    #[test]
    fn the_available_line_is_never_floored_at_zero() {
        // A client drawn past its line is OVER the line, and the shortfall is what forces the sale.
        let a = account(80, 50, 1_000.0, 900.0);
        let over = headroom(&a, 400.0);
        assert!(over < 0.0);
        assert_eq!(over, -300.0);
    }

    #[test]
    fn an_unmet_call_has_a_consequence_and_a_met_one_moves_cash() {
        // No margin that is only a number.
        let a = account(80, 50, 1_000.0, 900.0);
        assert_eq!(
            call(&a, 400.0, 500.0, 0.0),
            Call::Met {
                from: party(50),
                to: party(80),
                amount: 300.0
            }
        );
        // To meet it, the client may have to SELL into a market that must clear.
        assert_eq!(
            call(&a, 400.0, 100.0, 500.0),
            Call::MustSell {
                who: party(50),
                raising: 200.0
            }
        );
        // And a client that can do neither is liquidated.
        assert_eq!(
            call(&a, 400.0, 0.0, 50.0),
            Call::Liquidated {
                who: party(50),
                short_by: 250.0
            }
        );
        // A client inside its requirement is called for nothing.
        assert_eq!(call(&a, 50.0, 0.0, 0.0), Call::None);
    }

    #[test]
    fn the_shortfall_after_a_liquidation_hits_the_brokers_capital() {
        // The proceeds may be less than the loan — concentrated collateral is worth less in
        // liquidation than it is marked at, which is why what it fetched is not a calculation.
        let clean = Liquidation {
            broker: party(80),
            fetched: 950.0,
            loan_was: 900.0,
        };
        assert!(clean.loss() < 0.0);
        let concentrated = Liquidation {
            broker: party(80),
            fetched: 600.0,
            loan_was: 900.0,
        };
        assert_eq!(concentrated.loss(), 300.0);
    }

    #[test]
    fn a_liquidation_moves_prices_and_margin_calls_other_clients() {
        // The chain from one fund to one bank to other funds is traceable party by party — a loss
        // that stops at the fund is a broker that was never really lending.
        let others = [
            account(80, 51, 1_000.0, 880.0),
            account(80, 52, 1_000.0, 100.0),
        ];
        let hit = reaches(500.0, 5_000.0, &others, &view());
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].0, party(51));
        assert!(hit[0].1 > 0.0);
        // A small sale into a deep market reaches nobody.
        assert!(reaches(10.0, 500_000.0, &others, &view()).is_empty());
    }

    #[test]
    fn no_broker_sees_the_whole_position_and_each_underestimates_what_it_would_take_to_sell() {
        let in_one_name = [
            (party(80), 400_000.0),
            (party(81), 500_000.0),
            (party(82), 600_000.0),
        ];
        let depth = 1_000_000.0;
        let seen = concentration_seen_by(party(80), &in_one_name, depth).unwrap();
        let whole = true_concentration(&in_one_name, depth).unwrap();
        assert!(seen < 1.0);
        assert!(whole > 1.0);
        for (broker, _) in in_one_name {
            assert!(concentration_seen_by(broker, &in_one_name, depth).unwrap() < whole);
        }
        assert!(concentration_seen_by(party(99), &in_one_name, depth).is_none());
        assert!(true_concentration(&in_one_name, 0.0).is_none());

        // And each broker's leverage read is right about its own rows: the aggregate lies BETWEEN
        // them, which is why the blind spot is concentration and not this number.
        let spread = [
            account(80, 50, 1_000.0, 600.0),
            account(81, 50, 1_000.0, 800.0),
        ];
        let one = leverage_seen_by(party(80), &spread).unwrap();
        let other = leverage_seen_by(party(81), &spread).unwrap();
        assert!(one < other);
        assert!(leverage_seen_by(party(99), &spread).is_none());
    }

    #[test]
    fn a_broker_has_a_limit_per_client_and_it_binds() {
        // A broker with no limit per client is a synthetic counterparty.
        let inside = account(80, 50, 1_000.0, 600.0);
        assert!(inside.within_limit());
        let over = Account {
            lent: 40_000.0,
            ..inside
        };
        assert!(!over.within_limit());
    }

    #[test]
    fn the_broker_earns_the_spread_the_borrow_fee_and_the_commission() {
        // The income is the reason it takes the risk, and its balance sheet grows by the loan.
        let a = Account {
            stock_borrowed: 500.0,
            ..account(80, 50, 1_000.0, 600.0)
        };
        let e = earns(&a, 0.06, 0.04, 0.01, 12.0);
        // 600 × 0.02 is not 12 in binary, so this is asserted against its dust.
        assert!((e.financing - 12.0).abs() <= crate::num::dust(2, &[600.0, 12.0]));
        assert_eq!(e.stock_borrow, 5.0);
        assert_eq!(e.commissions, 12.0);
    }

    #[test]
    #[should_panic(expected = "earns it nothing to be at risk for")]
    fn a_loan_at_the_brokers_own_cost_of_funds_is_refused() {
        earns(&account(80, 50, 1_000.0, 600.0), 0.04, 0.04, 0.01, 12.0);
    }

    #[test]
    #[should_panic(expected = "no price to move")]
    fn a_sale_into_a_market_with_no_depth_has_no_price_to_move() {
        reaches(500.0, 0.0, &[], &view());
    }
}
