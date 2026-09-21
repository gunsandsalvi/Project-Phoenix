//! Prices: `(market, instrument, week)` prints with provenance.

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
    /// Real supply met real demand in this book, this week.
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
    pub week: u32,
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

    /// The key is (instrument, week): a print carries the market it came from and two markets in
    /// one line cannot both print in one week, which is refused here rather than resolved.
    pub fn write(&mut self, p: Print) {
        assert!(
            p.price.is_finite(),
            "Law 6: a price of {} is not a price",
            p.price
        );
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
                last.week != p.week,
                "Law 4: {} is printed twice in week {}",
                p.instrument.0,
                p.week
            );
            assert!(last.week < p.week, "Law 10: a print arrives out of order");
        }
        run.push(p);
        self.written += 1;
    }

    /// The last print at or before `up_to`, or none.
    pub fn latest(&self, instrument: InstrumentId, up_to: u32) -> Option<Print> {
        let slot = *self.at.get(&instrument.0)?;
        let run = &self.of[slot as usize];
        // The run is in week order, so this is a search and never a walk.
        let mut lo = 0usize;
        let mut hi = run.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if run[mid].week <= up_to {
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
        assert!(
            p.quoted_as == QuotedAs::Money,
            "Derivative D7: {what} — this book prints a RATE"
        );
        p.price
    }

    pub fn rate(p: &Print, what: &str) -> f64 {
        assert!(
            p.quoted_as == QuotedAs::Rate,
            "Derivative D7: {what} — this book prints MONEY"
        );
        p.price
    }

    pub fn lines(&self) -> usize {
        self.of.len()
    }

    /// How many lines carry a print a reader in this week can see — walked, never stored.
    pub fn that_printed(&self, lines: usize, up_to: u32) -> usize {
        (0..lines)
            .filter(|i| self.latest(InstrumentId::at(*i as u32), up_to).is_some())
            .count()
    }
}

// `latest` answers `Option<Print>` and hands back the print as written, so a caller cannot take a
// price that is not there. A print older than the week asked for still comes back, carrying the
// week it was struck in, and the caller decides what to do about that.

#[cfg(test)]
mod tests {
    use super::*;

    fn print(i: u32, week: u32, price: f64) -> Print {
        Print {
            instrument: InstrumentId::at(i),
            market: MarketId::at(i),
            week,
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
