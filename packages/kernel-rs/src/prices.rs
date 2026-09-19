//! Prices: `(market, instrument, period)` prints with provenance.

use crate::ids::{CurrencyCode, InstrumentId, MarketId};
use std::collections::HashMap;

/// WHAT THE LEVEL IS — money per unit, or a rate.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuotedAs {
    Money,
    Rate,
}

/// Where a print came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Provenance {
    /// Real supply met real demand in this book, this period.
    Cleared,
    /// The book ran and nothing crossed, so it carries its last level and says so.
    Carried,
    /// The world opened at a stated level, which is a primitive and dies at the seed.
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

    /// The last print at or before `up_to`, or none.
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

    /// Read a print the way its book quotes it, and refuse where it quotes the other.
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

    /// How many lines carry a print a reader in this period can see — walked, never stored.
    pub fn that_printed(&self, lines: usize, up_to: u32) -> usize {
        (0..lines)
            .filter(|i| self.latest(InstrumentId::at(*i as u32), up_to).is_some())
            .count()
    }
}

// `latest` answers `Option<Print>` and hands back the print as written, so a caller cannot take a
// price that is not there and a stale one carries its own period. That is the type, and it needs no
// test.

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
    #[should_panic(expected = "prints a RATE")]
    fn a_rate_is_not_read_as_money() {
        let mut r = print(1, 5, 0.04);
        r.quoted_as = QuotedAs::Rate;
        Prints::money(&r, "what a unit costs");
    }
}
