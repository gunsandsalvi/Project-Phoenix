use std::collections::BTreeMap;

use phx_id::{Day, MarketId, PartyId};
use phx_ledger::commitment::CommitmentId;
use phx_macros::clause;
use phx_num::{Ccy, Missing, PriceRaw, UnitId, capacity_exceeded, violation};

use crate::failure::MarketFailure;
use crate::market::Form;

/// One match: a buyer and a seller, the quantity that changes hands at a price, and the commitment of the seller's
/// the purchase draws, when it draws one.
#[clause("MKT.11", "MKT.13")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Match {
    pub buyer: PartyId,
    pub seller: PartyId,
    pub qty: i64,
    pub price: PriceRaw,
    pub draws: Missing<CommitmentId>,
}

/// A match set's identity on the tape.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct MatchSetId(u32);

impl MatchSetId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A print's identity on the tape.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct PrintId(u32);

impl PrintId {
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A meeting's matches, as the market recorded them: every print names one, and every trade a print stands for is
/// in it.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct MatchSet {
    pub market: MarketId,
    pub day: Day,
    pub matches: Vec<Match>,
}

/// A formed price with its market, day, unit, currency, the quantity it traded and its form, and the match set it
/// came out of. Only a meeting of this crate makes one, and only from the matches it recorded.
#[clause("MKT.2", "MKT.14")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Print {
    market: MarketId,
    day: Day,
    unit: UnitId,
    ccy: Ccy,
    quantity: i64,
    price: PriceRaw,
    form: Form,
    matches: MatchSetId,
}

impl Print {
    pub fn market(&self) -> MarketId {
        self.market
    }

    pub fn day(&self) -> Day {
        self.day
    }

    pub fn unit(&self) -> UnitId {
        self.unit
    }

    pub fn ccy(&self) -> Ccy {
        self.ccy
    }

    #[must_use]
    pub fn quantity(&self) -> i64 {
        self.quantity
    }

    pub fn price(&self) -> PriceRaw {
        self.price
    }

    #[must_use]
    pub fn form(&self) -> Form {
        self.form
    }

    pub fn matches(&self) -> MatchSetId {
        self.matches
    }
}

/// A fixing: the day's trades of a dealer or bilateral market taken by a named publisher's declared method, labelled
/// as a fixing and never a print, with the match sets it rests on.
#[clause("MKT.12")]
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Fixing {
    pub market: MarketId,
    pub day: Day,
    pub price: PriceRaw,
    pub publisher: &'static str,
    pub method: &'static str,
    pub rests_on: Vec<MatchSetId>,
}

/// Where a market's mark came from: its print, or its fixing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum MarkSource {
    Print(PrintId),
    Fixing(usize),
    /// The opening's present value of the market's latest print, which the snapshot dates.
    Snapshot,
}

/// The day's mark of a market, which every holder whose carrying basis marks to it reads.
#[clause("MKT.12")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Mark {
    pub market: MarketId,
    pub day: Day,
    pub price: PriceRaw,
    pub source: MarkSource,
}

fn next_index(len: usize) -> u32 {
    let Ok(i) = u32::try_from(len) else {
        capacity_exceeded!("entries of the tape", u32::MAX, len);
    };
    i
}

/// The public tape: every match set, print, fixing, mark and failure, and each market's last print.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tape {
    sets: Vec<MatchSet>,
    prints: Vec<Print>,
    fixings: Vec<Fixing>,
    failures: Vec<MarketFailure>,
    marks: BTreeMap<MarketId, Mark>,
    last: BTreeMap<MarketId, PrintId>,
    last_failed: BTreeMap<MarketId, usize>,
}

/// The tape saved as it was published, and its last print and last failure per market rebuilt on load.
impl phx_store::Saved for Tape {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        self.sets.save(w);
        self.prints.save(w);
        self.fixings.save(w);
        self.failures.save(w);
        self.marks.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Tape, phx_store::LoadError> {
        let mut tape = Tape {
            sets: phx_store::Saved::load(r)?,
            prints: phx_store::Saved::load(r)?,
            fixings: phx_store::Saved::load(r)?,
            failures: phx_store::Saved::load(r)?,
            marks: phx_store::Saved::load(r)?,
            last: BTreeMap::new(),
            last_failed: BTreeMap::new(),
        };
        for (i, p) in tape.prints.iter().enumerate() {
            tape.last.insert(p.market, PrintId(phx_store::narrow(i, "a print's place on the tape")?));
        }
        for (i, f) in tape.failures.iter().enumerate() {
            tape.last_failed.insert(f.market, i);
        }
        Ok(tape)
    }
}

impl Tape {
    /// A meeting's matches recorded with no print of their own: the trades of a dealer or bilateral market, which a
    /// fixing may later rest on.
    pub fn record(&mut self, market: MarketId, day: Day, matches: Vec<Match>) -> MatchSetId {
        let set = MatchSetId(next_index(self.sets.len()));
        self.sets.push(MatchSet { market, day, matches });
        set
    }

    /// A market's mark for the day, from its print or a fixing, or the opening's snapshot of a market not yet printed.
    pub fn mark(&mut self, mark: Mark) {
        let known = match mark.source {
            MarkSource::Print(id) => {
                self.print_of(id).is_some_and(|p| p.market == mark.market && p.price == mark.price)
            }
            MarkSource::Fixing(i) => {
                self.fixings.get(i).is_some_and(|f| f.market == mark.market && f.price == mark.price)
            }
            // A snapshot stands for a market's print before the world's first, never after it.
            MarkSource::Snapshot => matches!(self.last_print(mark.market), Missing::Absent),
        };
        if !known {
            violation!(clause = "MKT.12", "a mark from no print or fixing of its market", market = mark.market.get());
        }
        self.marks.insert(mark.market, mark);
    }

    /// A fixing published, returned by its index for the mark that reads it.
    pub fn fix(&mut self, fixing: Fixing) -> usize {
        self.fixings.push(fixing);
        self.fixings.len() - 1
    }

    pub fn fail(&mut self, failure: MarketFailure) {
        self.last_failed.insert(failure.market, self.failures.len());
        self.failures.push(failure);
    }

    /// Whether a market's last meeting found no seller: it failed so after its last print, or with none printed.
    #[must_use]
    pub fn unsold(&self, market: MarketId) -> bool {
        let Some(f) = self.last_failed.get(&market).and_then(|i| self.failures.get(*i)) else { return false };
        let after = match self.last_print(market) {
            Missing::Present(p) => f.day > p.day,
            Missing::Absent => true,
        };
        after && f.kind == crate::failure::FailureKind::NoSeller
    }

    #[must_use]
    pub fn print_of(&self, id: PrintId) -> Option<&Print> {
        self.prints.get(usize::try_from(id.0).ok()?)
    }

    #[must_use]
    pub fn set_of(&self, id: MatchSetId) -> Option<&MatchSet> {
        self.sets.get(usize::try_from(id.0).ok()?)
    }

    /// A market's last print, which shows its age on a day it formed none.
    pub fn last_print(&self, market: MarketId) -> Missing<&Print> {
        match self.last.get(&market).and_then(|id| self.print_of(*id)) {
            Some(p) => Missing::Present(p),
            None => Missing::Absent,
        }
    }

    /// Each market's last print, in the markets' order.
    pub fn last_prints(&self) -> impl Iterator<Item = (MarketId, &Print)> + '_ {
        self.last.iter().filter_map(|(m, id)| self.print_of(*id).map(|p| (*m, p)))
    }

    pub fn mark_of(&self, market: MarketId) -> Missing<Mark> {
        match self.marks.get(&market) {
            Some(m) => Missing::Present(*m),
            None => Missing::Absent,
        }
    }

    #[must_use]
    pub fn sets(&self) -> &[MatchSet] {
        &self.sets
    }

    #[must_use]
    pub fn prints(&self) -> &[Print] {
        &self.prints
    }

    #[must_use]
    pub fn fixings(&self) -> &[Fixing] {
        &self.fixings
    }

    #[must_use]
    pub fn failures(&self) -> &[MarketFailure] {
        &self.failures
    }

    #[must_use]
    pub fn marks(&self) -> &BTreeMap<MarketId, Mark> {
        &self.marks
    }
}
