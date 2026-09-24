use std::collections::BTreeMap;

use phx_id::{InstrumentId, PartyId};
use phx_macros::clause;
use phx_num::{Qty, violation};

/// Free units covering an open offer, held or borrowed from a named lender. Only the ledger builds one, and building
/// it places a commitment on those units until its order lapses or its trade settles, so no unit covers two offers.
#[clause("REG.10", "REG.16")]
#[must_use]
#[derive(Debug, PartialEq, Eq)]
pub struct Covered {
    holder: PartyId,
    instrument: InstrumentId,
    qty: Qty,
}

impl Covered {
    pub fn qty(&self) -> Qty {
        self.qty
    }

    pub fn holder(&self) -> PartyId {
        self.holder
    }

    pub fn instrument(&self) -> InstrumentId {
        self.instrument
    }

    /// The cover given up, as what it covered.
    fn into_parts(self) -> (PartyId, InstrumentId, Qty) {
        (self.holder, self.instrument, self.qty)
    }
}

/// An offer the units cannot cover: how many were free.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uncovered {
    pub free: i64,
}

/// The units each holding has committed to open offers.
#[derive(Clone, Debug, Default)]
pub struct Covers {
    committed: BTreeMap<(PartyId, InstrumentId), i64>,
}

impl Covers {
    /// The units of a holding committed to open offers.
    #[must_use]
    pub fn committed(&self, holder: PartyId, instrument: InstrumentId) -> i64 {
        self.committed.get(&(holder, instrument)).copied().unwrap_or(0)
    }

    /// Units covering an offer, from those held or borrowed that are neither pledged nor already covering one.
    ///
    /// # Errors
    /// `Uncovered` when fewer units are free than the offer names.
    pub fn cover(
        &mut self,
        holder: PartyId,
        instrument: InstrumentId,
        qty: Qty,
        available: i64,
        pledged: i64,
    ) -> Result<Covered, Uncovered> {
        let free = available - pledged - self.committed(holder, instrument);
        if qty.n() <= 0 || qty.n() > free {
            return Err(Uncovered { free });
        }
        *self.committed.entry((holder, instrument)).or_insert(0) += qty.n();
        Ok(Covered { holder, instrument, qty })
    }

    /// The commitment released, when the offer's order lapses or its trade settles.
    pub fn release(&mut self, covered: Covered) {
        let (holder, instrument, qty) = covered.into_parts();
        let key = (holder, instrument);
        let Some(c) = self.committed.get_mut(&key) else {
            violation!(clause = "REG.10", "a cover released that was never placed", units = qty.n());
        };
        *c -= qty.n();
        if *c == 0 {
            self.committed.remove(&key);
        }
    }
}
