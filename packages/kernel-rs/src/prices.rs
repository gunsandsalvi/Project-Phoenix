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
    /// The week this print is OF.
    pub week: u32,
    /// The week the level it carries was struck in. A cleared print struck its own; a carried one
    /// keeps the week the level it repeats actually crossed, so `week - struck` is how stale a
    /// mark is and nobody has to walk the run to find out.
    pub struck: u32,
    /// Per unit of the instrument, in its currency.
    pub price: f64,
    pub ccy: CurrencyCode,
    pub quoted_as: QuotedAs,
    pub provenance: Provenance,
}

#[derive(Default)]
pub struct Prints {
    /// Every print, in writing order, grouped by (market, instrument) so one book's history in one
    /// line is contiguous.
    of: Vec<Vec<Print>>,
    /// (market row, instrument row) -> index into `of`.
    at: HashMap<(u32, u32), u32>,
    /// instrument row -> every slot that line has printed in, so the markets in a good are a read.
    in_line: HashMap<u32, Vec<u32>>,
    written: u64,
}

impl Prints {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(&self) -> u64 {
        self.written
    }

    /// The key is (market, instrument, week): the same grade in two places is two prices, and the
    /// difference between them is what it costs to move it.
    pub fn write(&mut self, p: Print) {
        assert!(
            p.price.is_finite(),
            "Law 6: a price of {} is not a price",
            p.price
        );
        assert!(
            p.struck <= p.week,
            "Law 8: a print cannot carry a level struck after it"
        );
        let key = (p.market.0, p.instrument.0);
        let slot = match self.at.get(&key) {
            Some(&slot) => slot,
            None => {
                let slot = self.of.len() as u32;
                self.of.push(Vec::new());
                self.at.insert(key, slot);
                self.in_line.entry(p.instrument.0).or_default().push(slot);
                slot
            }
        };
        let run = &mut self.of[slot as usize];
        if let Some(last) = run.last() {
            assert!(
                last.week != p.week,
                "Law 4: market {} prints {} twice in week {}",
                p.market.0,
                p.instrument.0,
                p.week
            );
            assert!(last.week < p.week, "Law 10: a print arrives out of order");
        }
        run.push(p);
        self.written += 1;
    }

    /// What this BOOK last printed at or before `up_to`, or none.
    pub fn latest(&self, market: MarketId, instrument: InstrumentId, up_to: u32) -> Option<Print> {
        let slot = *self.at.get(&(market.0, instrument.0))?;
        self.up_to(slot, up_to)
    }

    /// What this LINE last printed, for a reader holding units rather than a seat in a book.
    ///
    /// A good is its sub-unit and a market in it is (region, sub-unit), so a line that has printed
    /// in two places has no one answer: which one a holder marks at turns on where its units are,
    /// and a store that picked would be picking for it.
    pub fn of_line(&self, instrument: InstrumentId, up_to: u32) -> Option<Print> {
        let slots = self.in_line.get(&instrument.0)?;
        assert!(
            slots.len() < 2,
            "Appendix A: instrument {} has printed in {} markets — read the one its units are in",
            instrument.0,
            slots.len()
        );
        self.up_to(*slots.first()?, up_to)
    }

    /// THE PRICE A LINE FIRST CLEARED AT — what a discount is measured from, and the only print
    /// whose level is not a mark of something later.
    pub fn first_of_line(&self, instrument: InstrumentId) -> Option<Print> {
        let slots = self.in_line.get(&instrument.0)?;
        assert!(
            slots.len() < 2,
            "Appendix A: instrument {} has printed in {} markets — read the one its units are in",
            instrument.0,
            slots.len()
        );
        self.of[*slots.first()? as usize].first().copied()
    }

    /// The last print of a run at or before a week. The run is in week order, so this is a search
    /// and never a walk.
    fn up_to(&self, slot: u32, up_to: u32) -> Option<Print> {
        let run = &self.of[slot as usize];
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

    /// How many lines carry a print a reader in this week can see — walked over what was written,
    /// never over what might have been.
    pub fn that_printed(&self, up_to: u32) -> usize {
        self.in_line
            .iter()
            .filter(|(_, slots)| slots.iter().any(|slot| self.up_to(*slot, up_to).is_some()))
            .count()
    }
}

// `latest` and `of_line` answer `Option<Print>` and hand back the print as written, so a caller
// cannot take a price that is not there. A print older than the week asked for still comes back,
// carrying the week its level was struck in, and the caller decides what to do about that.

#[cfg(test)]
mod tests {
    use super::*;

    fn print(i: u32, week: u32, price: f64) -> Print {
        Print {
            instrument: InstrumentId::at(i),
            market: MarketId::at(i),
            week,
            struck: week,
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

    #[test]
    fn a_carried_print_says_how_old_the_level_it_repeats_is() {
        let mut carried = print(7, 6, 100.0);
        carried.struck = 3;
        carried.provenance = Provenance::Carried;
        // Three weeks stale, off the print itself and without a walk back through the run.
        assert_eq!(carried.week - carried.struck, 3);
        // And a print that cleared is not stale at all, whatever week reads it.
        assert_eq!(print(7, 6, 100.0).week - print(7, 6, 100.0).struck, 0);
    }
}
