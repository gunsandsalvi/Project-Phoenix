//! The retail market's kind, what a buyer wants there and what a sale pays; the meeting itself is `meet`.

use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::market::MarketDecl;

/// A retail market kind as its system declares it: its market, an instance per product; the kinds that sell in it;
/// the facts naming what a seller sells and the price it posts; the primitives weighing price and distance and
/// bounding how far a buyer reaches; the stream buyers' tastes are drawn from; and the reason a purchase settles
/// under.
#[clause("SRV.1", "SRV.2", "SRV.9", "MKT.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetailKind {
    pub market: MarketDecl,
    pub sellers: &'static [&'static str],
    pub sells: &'static str,
    pub price: &'static str,
    /// A seller's units made a day, what it can serve of a product delivered as it is made: its fact, read at its
    /// fixed point.
    pub capacity: phx_core::ItemDecl,
    /// The way a seller makes its product by.
    pub way: &'static str,
    /// A seller's units a day its plant allows, where its plant limits it.
    pub plant: &'static str,
    pub price_weight: &'static str,
    pub distance_weight: &'static str,
    pub reach: &'static str,
    pub tastes: &'static str,
    pub reason: &'static str,
}

/// What a buyer wants: units it needs, money it spends, or units it needs at no more than a price for a lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Want {
    Units(i64),
    Money(i64),
    UpTo(i64, i64),
}

/// How a buyer weighs a seller: per unit of the log of its price, and per km of distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weights {
    pub price: f64,
    pub distance: f64,
}

/// What `units` of a good cost at `price` a lot of `lot` units, rounded once, as the sale settles.
#[must_use]
pub fn paid(units: i64, price: i64, lot: i64) -> i64 {
    let amount =
        phx_num::round::div_round(i128::from(units) * i128::from(price), i128::from(lot), phx_num::Round::HalfEven);
    let Ok(amount) = i64::try_from(amount) else {
        capacity_exceeded!("a trade's money", i64::MAX, units);
    };
    amount
}

/// Whole units a buyer wants at a price posted for a lot: its units, none above its limit, or the units its money
/// buys.
pub(crate) fn units_wanted(want: Want, lot: i64, price: i64) -> i64 {
    match want {
        Want::Units(q) => q,
        Want::UpTo(q, limit) => {
            if price <= limit {
                q
            } else {
                0
            }
        }
        // A word's product divides in a word; only a larger one needs two.
        Want::Money(m) if price > 0 => match m.checked_mul(lot) {
            Some(p) => p / price,
            None => match i64::try_from(i128::from(m) * i128::from(lot) / i128::from(price)) {
                Ok(q) => q,
                Err(_) => capacity_exceeded!("units a buyer's money buys", i64::MAX, m),
            },
        },
        Want::Money(_) => 0,
    }
}

/// What a buyer still wants after buying `units` at `price` a lot.
pub(crate) fn less(want: Want, lot: i64, units: i64, price: i64) -> Want {
    match want {
        Want::Units(q) => Want::Units(q - units),
        Want::Money(m) => Want::Money(m - paid(units, price, lot)),
        Want::UpTo(q, limit) => Want::UpTo(q - units, limit),
    }
}

/// Each seller's chance of a buyer's choice where every seller stands equally far from it: the logit's over the
/// price term, `exp(−w·ln price) ÷ Σ`, a seller with no price weighing nothing.
#[clause("SRV.4")]
#[must_use]
pub fn price_chances(prices: &[i64], weight: f64) -> Vec<f64> {
    let v: Vec<Option<f64>> =
        prices.iter().map(|p| (*p > 0).then(|| -weight * libm::log(phx_rand::float::from_i64(*p)))).collect();
    let Some(best) = v.iter().flatten().copied().reduce(|a, b| if b > a { b } else { a }) else {
        return vec![0.0; prices.len()];
    };
    let e: Vec<f64> = v.iter().map(|x| x.map_or(0.0, |x| libm::exp(x - best))).collect();
    let whole: f64 = e.iter().sum();
    e.iter().map(|x| x / whole).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn price_chances_follow_the_logit() {
        let c = super::price_chances(&[100, 200, 0], 1.0);
        assert!((c[0] - 2.0 / 3.0).abs() < 1e-12 && (c[1] - 1.0 / 3.0).abs() < 1e-12, "{c:?}");
        assert!(c[2] == 0.0, "no price weighs nothing");
        assert_eq!(super::price_chances(&[0], 1.0), vec![0.0]);
    }

    #[test]
    fn a_limit_buys_only_at_or_below_it() {
        use super::{Want, less, units_wanted};
        assert_eq!(units_wanted(Want::UpTo(5, 200), 10, 200), 5);
        assert_eq!(units_wanted(Want::UpTo(5, 200), 10, 201), 0);
        assert_eq!(less(Want::UpTo(5, 200), 10, 3, 150), Want::UpTo(2, 200));
    }
}
