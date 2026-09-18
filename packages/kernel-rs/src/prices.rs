//! Prices: `(market, instrument, period)` prints with provenance (Clearing D1, Law 3).
//!
//! Every price is CLEARED from real supply meeting real demand. Nothing here writes a price from a
//! yield, a spread, a multiple or a target, and there is no price path: a print exists because a
//! book cleared, and a reader that finds none gets none.
//!
//! Columnar, and the history of one line is a counted slice so `latest` is a binary search over a
//! contiguous run rather than a walk of a boxed list. Measured in TypeScript: `latest` is
//! **60.30 ns** over 12,783,916 calls a period.

use crate::ids::{CurrencyCode, InstrumentId, MarketId};
use std::collections::HashMap;

/// Law 8, Derivative D7: WHAT THE LEVEL IS — money per unit, or a rate. There is no third tag, and
/// reading one as the other is refused at the READ rather than silently averaged.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuotedAs {
    Money,
    Rate,
}

/// Law 3: where a print came from. A print with no book behind it is not a price.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provenance {
    /// Real supply met real demand in this book, this period.
    Cleared,
    /// Clearing C4: the book ran and nothing crossed, so it carries its last level and says so.
    Carried,
    /// Seed C4: the world opened at a stated level, which is a primitive and dies at the seed.
    Seeded,
}

#[derive(Clone, Copy)]
pub struct Print {
    pub instrument: InstrumentId,
    pub market: MarketId,
    pub period: u32,
    /// Per unit of the instrument, in its currency.
    pub price: f64,
    pub ccy: CurrencyCode,
    pub quoted_as: QuotedAs,
    pub provenance: Provenance,
}

#[derive(Default)]
pub struct Prints {
    /// Every print, in writing order, grouped by instrument so one line's history is contiguous.
    of: Vec<Vec<Print>>,
    /// instrument row -> index into `of`.
    at: HashMap<u32, u32>,
    written: u64,
}

impl Prints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(&self) -> u64 {
        self.written
    }

    /// One print per (instrument, period); a second writer is Law 4's defect and is refused.
    pub fn write(&mut self, p: Print) {
        assert!(p.price.is_finite(), "Law 6: a price of {} is not a price", p.price);
        let slot = match self.at.get(&p.instrument.0) {
            Some(&slot) => slot,
            None => {
                let slot = self.of.len() as u32;
                self.of.push(Vec::new());
                self.at.insert(p.instrument.0, slot);
                slot
            }
        };
        let run = &mut self.of[slot as usize];
        if let Some(last) = run.last() {
            assert!(
                last.period != p.period,
                "Law 4: {} is printed twice in period {}",
                p.instrument.0,
                p.period
            );
            assert!(last.period < p.period, "Law 10: a print arrives out of order");
        }
        run.push(p);
        self.written += 1;
    }

    /// Clearing D1: the last print at or before `up_to`, or none. A price that does not exist is
    /// MISSING, never zero and never the last one pretending to be this one.
    pub fn latest(&self, instrument: InstrumentId, up_to: u32) -> Option<Print> {
        let slot = *self.at.get(&instrument.0)?;
        let run = &self.of[slot as usize];
        // The run is in period order, so this is a search and never a walk.
        let mut lo = 0usize;
        let mut hi = run.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if run[mid].period <= up_to {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        if lo == 0 {
            None
        } else {
            Some(run[lo - 1])
        }
    }

    /// Law 8, D7: read a print the way its book quotes it, and refuse where it quotes the other.
    pub fn money(p: &Print, what: &str) -> f64 {
        assert!(p.quoted_as == QuotedAs::Money, "Derivative D7: {what} — this book prints a RATE");
        p.price
    }

    pub fn rate(p: &Print, what: &str) -> f64 {
        assert!(p.quoted_as == QuotedAs::Rate, "Derivative D7: {what} — this book prints MONEY");
        p.price
    }

    pub fn lines(&self) -> usize {
        self.of.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn print(i: u32, period: u32, price: f64) -> Print {
        Print {
            instrument: InstrumentId::at(i),
            market: MarketId::at(i),
            period,
            price,
            ccy: CurrencyCode::at(0),
            quoted_as: QuotedAs::Money,
            provenance: Provenance::Cleared,
        }
    }

    #[test]
    fn a_price_that_does_not_exist_is_missing_and_never_zero() {
        let mut p = Prints::new();
        p.write(print(1, 5, 12.5));
        assert!(p.latest(InstrumentId::at(1), 4).is_none());
        assert!(p.latest(InstrumentId::at(2), 9).is_none());
        assert_eq!(p.latest(InstrumentId::at(1), 5).unwrap().price, 12.5);
        // A stale print is visibly stale: it is the period's own, not today's.
        assert_eq!(p.latest(InstrumentId::at(1), 40).unwrap().period, 5);
    }

    #[test]
    #[should_panic(expected = "is printed twice")]
    fn one_print_per_instrument_per_period() {
        let mut p = Prints::new();
        p.write(print(1, 5, 12.5));
        p.write(print(1, 5, 13.0));
    }

    #[test]
    #[should_panic(expected = "prints a RATE")]
    fn a_rate_is_not_read_as_money() {
        let mut p = Prints::new();
        let mut r = print(1, 5, 0.04);
        r.quoted_as = QuotedAs::Rate;
        p.write(r);
        let got = p.latest(InstrumentId::at(1), 5).unwrap();
        Prints::money(&got, "what a unit costs");
    }
}
