//! THE CURRENCY LAYER: every pair clears on the flow that actually crosses it, and triangular
//! consistency is an OUTCOME that bounded arbitrageurs enforce — or, at their limits, fail to.
//!
//! @spec XI-12 · XI-7 · Law 3, Law 4, Law 5, Law 6, Law 19 · Appendix B

use crate::calendar::Period;
use crate::ids::{CurrencyCode, PartyId};

/// A book.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Pair {
    pub base: CurrencyCode,
    pub quote: CurrencyCode,
}

impl Pair {
    pub fn of(base: CurrencyCode, quote: CurrencyCode) -> Pair {
        assert!(base != quote, "XI-12: {base:?} against itself is not a pair");
        Pair { base, quote }
    }

    pub fn other_way(self) -> Pair {
        Pair { base: self.quote, quote: self.base }
    }
}

/// What a pair's own flow cleared at, in the period it cleared in.
#[derive(Clone, Copy, Debug)]
pub struct Crossed {
    pub pair: Pair,
    pub at: f64,
    pub period: Period,
    /// The flow that actually crossed this pair.
    pub on_flow: f64,
}

/// The prints, one per pair that cleared.
#[derive(Default)]
pub struct Rates {
    prints: Vec<Crossed>,
}

impl Rates {
    pub fn new() -> Rates {
        Rates { prints: Vec::new() }
    }

    pub fn cleared(&mut self, print: Crossed) {
        assert!(print.on_flow > 0.0, "XI-12: a print off no flow is not a price (Law 3)");
        self.prints.push(print);
    }

    /// The rate for a pair, or `None`.
    pub fn of(&self, pair: Pair) -> Option<f64> {
        for p in self.prints.iter().rev() {
            if p.pair == pair {
                return Some(p.at);
            }
            if p.pair == pair.other_way() {
                return Some(1.0 / p.at);
            }
        }
        None
    }
}

/// Triangular consistency is an outcome.
pub fn gap(a_to_b: f64, b_to_c: f64, a_to_c: f64) -> f64 {
    a_to_b * b_to_c - a_to_c
}

/// An arbitrageur with a balance sheet.
#[derive(Clone, Copy, Debug)]
pub struct Arbitrageur {
    pub who: PartyId,
    /// What it can put to work, in the base money.
    pub capital: f64,
    /// What it pays to cross each leg.
    pub cost_per_leg: f64,
}

/// What this arbitrageur actually does about a gap: the size it can fund, or nothing.
pub fn arbitrage(a: &Arbitrageur, gap_per_unit: f64, available: f64) -> Option<f64> {
    let after_costs = gap_per_unit.abs() - 3.0 * a.cost_per_leg;
    if after_costs <= 0.0 {
        return None;
    }
    let size = if a.capital < available { a.capital } else { available };
    if size <= 0.0 {
        return None;
    }
    Some(size)
}

/// The parity point: the forward that carries the interest differential over the tenor.
pub fn covered(spot: f64, base_rate: f64, quote_rate: f64, year_fraction: f64) -> f64 {
    spot * (1.0 + quote_rate * year_fraction) / (1.0 + base_rate * year_fraction)
}

/// THE basis — what the cleared forward says against parity.
pub fn basis(forward_cleared: f64, spot: f64, base_rate: f64, quote_rate: f64, year_fraction: f64) -> f64 {
    assert!(year_fraction > 0.0, "XI-12: a basis over no time is not a rate (Law 8)");
    let parity = covered(spot, base_rate, quote_rate, year_fraction);
    (forward_cleared / parity - 1.0) / year_fraction
}

/// XI-12's one convention: a purchase settles in the seller's money.
pub fn settles_in(sellers_money: CurrencyCode) -> CurrencyCode {
    sellers_money
}

/// What a buyer short of the seller's money must do about it: buy it, in a currency book, with a
/// counterparty on the other side.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MustBuy {
    pub who: PartyId,
    pub pair: Pair,
    pub quantity: f64,
}

pub fn short_of(
    buyer: PartyId,
    owes: f64,
    in_money: CurrencyCode,
    has: f64,
    own_money: CurrencyCode,
) -> Option<MustBuy> {
    if in_money == own_money || has >= owes {
        return None;
    }
    Some(MustBuy { who: buyer, pair: Pair::of(own_money, in_money), quantity: owes - has })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ccy(n: u32) -> CurrencyCode {
        CurrencyCode::at(n)
    }

    fn period() -> Period {
        Period(1)
    }

    fn print(base: u32, quote: u32, at: f64) -> Crossed {
        Crossed { pair: Pair::of(ccy(base), ccy(quote)), at, period: period(), on_flow: 1_000.0 }
    }

    #[test]
    fn a_pair_nothing_crossed_has_no_rate_rather_than_a_triangulated_one() {
        // This is the whole item.
        let mut r = Rates::new();
        r.cleared(print(1, 2, 1.25));
        r.cleared(print(2, 3, 0.80));
        assert_eq!(r.of(Pair::of(ccy(1), ccy(2))), Some(1.25));
        assert!(r.of(Pair::of(ccy(1), ccy(3))).is_none());
    }

    #[test]
    fn the_two_directions_of_a_pair_are_one_print_read_from_either_end() {
        // One fact, one writer.
        let mut r = Rates::new();
        r.cleared(print(1, 2, 1.25));
        let back = r.of(Pair::of(ccy(2), ccy(1))).unwrap();
        assert!((back * 1.25 - 1.0).abs() <= 2.0 * f64::EPSILON);
    }

    #[test]
    fn a_print_off_no_flow_is_not_a_price() {
        // The assertion is the refusal; there is no path that records one.
        let mut r = Rates::new();
        let silent = Crossed { on_flow: 0.0, ..print(1, 2, 1.25) };
        assert!(std::panic::catch_unwind(move || {
            r.cleared(silent);
        })
        .is_err());
    }

    #[test]
    fn triangular_consistency_is_measured_and_a_gap_can_stand() {
        // An outcome that bounded arbitrageurs enforce — and may fail to enforce.
        let wide = gap(1.25, 0.80, 0.95);
        assert!(wide.abs() > 0.0);
        let a = Arbitrageur { who: PartyId::at(3), capital: 1_000_000.0, cost_per_leg: 0.002 };
        assert!(arbitrage(&a, wide, 500_000.0).is_some());
        let narrow = gap(1.25, 0.80, 1.0 - 0.0005);
        assert!(narrow.abs() > 0.0);
        assert!(arbitrage(&a, narrow, 500_000.0).is_none());
    }

    #[test]
    fn an_arbitrageur_does_only_what_its_own_capital_funds() {
        // No unlimited arbitrageur.
        let big = Arbitrageur { who: PartyId::at(3), capital: 1_000_000.0, cost_per_leg: 0.002 };
        let small = Arbitrageur { capital: 5_000.0, ..big };
        let g = gap(1.25, 0.80, 0.95);
        assert_eq!(arbitrage(&big, g, 500_000.0), Some(500_000.0));
        assert_eq!(arbitrage(&small, g, 500_000.0), Some(5_000.0));
    }

    #[test]
    fn the_forward_carries_the_interest_differential() {
        // A forward struck as spot moved by a basis, with no differential in it, is neither cleared
        // nor at parity.
        let spot = 1.25;
        let dear = covered(spot, 0.01, 0.05, 1.0);
        let cheap = covered(spot, 0.05, 0.01, 1.0);
        assert!(dear > spot);
        assert!(cheap < spot);
        // And with no differential the forward is the spot, which is the degenerate case and not the
        // general one.
        assert!((covered(spot, 0.03, 0.03, 1.0) - spot).abs() <= 4.0 * f64::EPSILON * spot);
    }

    #[test]
    fn there_is_one_basis_and_it_is_derived_from_the_cleared_forward() {
        // Read the source.
        let spot = 1.25;
        let parity = covered(spot, 0.01, 0.05, 0.5);
        assert!(basis(parity, spot, 0.01, 0.05, 0.5).abs() <= 8.0 * f64::EPSILON);
        let wide = basis(parity * 1.01, spot, 0.01, 0.05, 0.5);
        assert!(wide > 0.0);
    }

    #[test]
    fn a_basis_over_no_time_is_not_a_rate() {
        // The periodicity is part of the number.
        assert!(std::panic::catch_unwind(|| basis(1.26, 1.25, 0.01, 0.05, 0.0)).is_err());
    }

    #[test]
    fn a_purchase_settles_in_the_sellers_money_whoever_the_buyer_is() {
        // A convention that depends on WHO the buyer is means the same purchase lands the flow in
        // two different places.
        let sellers = ccy(2);
        assert_eq!(settles_in(sellers), sellers);
    }

    #[test]
    fn a_buyer_short_of_the_sellers_money_places_an_order_for_it() {
        // This order is what a conversion inside the trade deletes, and with it the currency demand
        // the purchase should have created.
        let buyer = PartyId::at(4);
        let must = short_of(buyer, 500.0, ccy(2), 120.0, ccy(1)).unwrap();
        assert_eq!(must, MustBuy { who: buyer, pair: Pair::of(ccy(1), ccy(2)), quantity: 380.0 });
        // A buyer paying in its own money is not short of anything, and a buyer that already holds
        // enough is not either.
        assert!(short_of(buyer, 500.0, ccy(1), 0.0, ccy(1)).is_none());
        assert!(short_of(buyer, 500.0, ccy(2), 600.0, ccy(1)).is_none());
    }
}
