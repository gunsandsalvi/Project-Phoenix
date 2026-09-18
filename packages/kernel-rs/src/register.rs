//! The register: who holds what, with lots and liens; both directions indexed; holdings sum to
//! issued (Register A1–B4).
//!
//! A holding is a ROW and its lots are a counted slice of one flat column. That is the whole of the
//! difference from the TypeScript store, and it is what the calibration measured: a traversal of
//! 544,104 holdings is **95.70 ms** there and **2.23 ms** here, and the hot read with the row in
//! hand is **67.20 ns** against **10.62 ns** (`tools/calibrate`).
//!
//! Law 4: settlement is the ONE writer. The reads are open to anybody; `debit`, `credit` and
//! `move_money` are reached only through the wire.

use crate::ids::{HoldingId, InstrumentId, PartyId, NONE};
use std::collections::HashMap;

/// Register D1: units carry the basis they were acquired at, so a disposal has a gain to book.
#[derive(Clone, Copy)]
pub struct Lot {
    pub qty: f64,
    pub basis_per_unit: f64,
    pub acquired: u32,
}

/// Register C3: units somebody else has a claim over. Moving them is refused, not adjusted (Law 6).
#[derive(Clone, Copy)]
pub struct Lien {
    pub to: PartyId,
    pub qty: f64,
}

/// What a debit drew, with the basis each parcel carried — settlement's own read for the gain.
#[derive(Clone, Copy)]
pub struct Drawn {
    pub qty: f64,
    pub basis_per_unit: f64,
    pub acquired: u32,
}

#[derive(Default)]
pub struct Register {
    // One row per holding. Columns, not records: a traversal touches only what it reads.
    holder: Vec<u32>,
    instrument: Vec<u32>,
    lot_at: Vec<u32>,
    lot_len: Vec<u32>,
    lien_at: Vec<u32>,
    lien_len: Vec<u32>,
    /// Law 18: the holding's own total, maintained by the one writer, so a read is not a sum.
    /// It is NOT a stored aggregate in Appendix B's sense — that forbids a stored TOTAL beside the
    /// units it is derived from across parties. This is the row's own quantity, written by the same
    /// instruction that writes the lots, and the audit still sums the lots to check it (Law 19).
    held: Vec<f64>,

    lots: Vec<Lot>,
    liens: Vec<Lien>,

    /// (holder, instrument) -> row. The one hash in the design, on a packed integer key, asked at
    /// a boundary. A caller in a loop holds the `HoldingId`.
    row_of: HashMap<u64, u32>,
    /// Both directions indexed (Register A3): the rows of one holder, and the rows of one line.
    by_holder: HashMap<u32, Vec<u32>>,
    by_instrument: HashMap<u32, Vec<u32>>,

    /// 0g.5: the write count a reader keys a kept answer on.
    writes: u64,
}

#[inline]
const fn key(holder: PartyId, instrument: InstrumentId) -> u64 {
    ((holder.0 as u64) << 32) | (instrument.0 as u64)
}

impl Register {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(&self) -> u64 {
        self.writes
    }

    pub fn rows(&self) -> usize {
        self.holder.len()
    }

    /// The row for this pair, or `HoldingId::NONE`. One hash; hold the answer across a loop.
    #[inline]
    pub fn row(&self, holder: PartyId, instrument: InstrumentId) -> HoldingId {
        match self.row_of.get(&key(holder, instrument)) {
            Some(&row) => HoldingId(row),
            None => HoldingId::NONE,
        }
    }

    /// Law 8: what the register holds is whole pieces, so what it reads back is a count of them.
    /// A pair nobody holds holds nothing — which is an answer, not a missing number.
    #[inline]
    pub fn quantity(&self, row: HoldingId) -> f64 {
        if row.some() {
            self.held[row.row()]
        } else {
            0.0
        }
    }

    /// Register C3: what is not encumbered. The liens are a slice, summed where it is asked.
    #[inline]
    pub fn free(&self, row: HoldingId) -> f64 {
        if !row.some() {
            return 0.0;
        }
        let at = self.lien_at[row.row()] as usize;
        let len = self.lien_len[row.row()] as usize;
        let mut pledged = 0.0;
        for l in &self.liens[at..at + len] {
            pledged += l.qty;
        }
        self.held[row.row()] - pledged
    }

    /// The lots of one holding, in the order they were acquired (Register D1).
    #[inline]
    pub fn lots(&self, row: HoldingId) -> &[Lot] {
        if !row.some() {
            return &[];
        }
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        &self.lots[at..at + len]
    }

    pub fn holder_of(&self, row: HoldingId) -> PartyId {
        PartyId(self.holder[row.row()])
    }

    pub fn instrument_of(&self, row: HoldingId) -> InstrumentId {
        InstrumentId(self.instrument[row.row()])
    }

    /// Every holding, as rows. It is a RANGE and not a list: the TypeScript `allHoldings` built
    /// 544,104 objects on every call and ten readers a period asked for it (0g.31).
    #[inline]
    pub fn all(&self) -> impl Iterator<Item = HoldingId> + '_ {
        (0..self.holder.len() as u32).map(HoldingId)
    }

    /// Register A3: the rows of one holder, and of one line. Both indexed, neither derived.
    pub fn of_holder(&self, holder: PartyId) -> &[u32] {
        match self.by_holder.get(&holder.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    pub fn of_instrument(&self, instrument: InstrumentId) -> &[u32] {
        match self.by_instrument.get(&instrument.0) {
            Some(rows) => rows,
            None => &[],
        }
    }

    /// B2: the sum of what is held of a line, over its own index, with the dust of the walk (Law 7).
    pub fn held_total(&self, instrument: InstrumentId) -> (f64, f64) {
        let mut total = 0.0;
        let mut magnitude = 0.0;
        let mut terms = 0usize;
        for &row in self.of_instrument(instrument) {
            let q = self.held[row as usize];
            total += q;
            magnitude += q.abs();
            terms += 1;
        }
        (total, (terms as f64 + 2.0) * f64::EPSILON * magnitude)
    }

    // ---- the writes: settlement's, and nobody else's (Law 4) ------------------------------------

    /// The row for this pair, opened if there is none. A holding without a holder or an issuer is
    /// what Appendix B forbids; both are named here and neither can be absent.
    fn open(&mut self, holder: PartyId, instrument: InstrumentId) -> HoldingId {
        assert!(holder.some(), "Appendix B: no holding without a holder");
        assert!(instrument.some(), "Appendix B: no holding without an issuer");
        let k = key(holder, instrument);
        if let Some(&row) = self.row_of.get(&k) {
            return HoldingId(row);
        }
        let row = self.holder.len() as u32;
        self.holder.push(holder.0);
        self.instrument.push(instrument.0);
        self.lot_at.push(self.lots.len() as u32);
        self.lot_len.push(0);
        self.lien_at.push(self.liens.len() as u32);
        self.lien_len.push(0);
        self.held.push(0.0);
        self.row_of.insert(k, row);
        self.by_holder.entry(holder.0).or_default().push(row);
        self.by_instrument.entry(instrument.0).or_default().push(row);
        HoldingId(row)
    }

    /// Register C1: units arrive with the basis they cost. A credit moves a positive quantity.
    pub fn credit(
        &mut self,
        holder: PartyId,
        instrument: InstrumentId,
        qty: f64,
        basis_per_unit: f64,
        period: u32,
    ) -> HoldingId {
        assert!(qty > 0.0, "Register C1: a credit moves a positive quantity");
        let row = self.open(holder, instrument);
        // A lot is appended, so a holding's lots are contiguous only until it is written again.
        // The column is compacted when a row's lots are next read past its end (below).
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        if at + len == self.lots.len() {
            self.lots.push(Lot { qty, basis_per_unit, acquired: period });
        } else {
            // Somebody else's lots are in the way: move this row's to the end of the column and
            // grow there. The old slice is left behind and is never read again.
            let mine: Vec<Lot> = self.lots[at..at + len].to_vec();
            self.lot_at[row.row()] = self.lots.len() as u32;
            self.lots.extend_from_slice(&mine);
            self.lots.push(Lot { qty, basis_per_unit, acquired: period });
        }
        self.lot_len[row.row()] += 1;
        self.held[row.row()] += qty;
        self.writes += 1;
        row
    }

    /// Register C4, D2: units leave OLDEST FIRST, and what they cost goes with them. A debit of
    /// more than is free is refused — arithmetic impossibility, never a clamp (Law 6).
    pub fn debit(&mut self, row: HoldingId, qty: f64) -> Vec<Drawn> {
        assert!(qty > 0.0, "Register C4: a debit moves a positive quantity");
        assert!(row.some(), "Register C4: nothing is held of this");
        let free = self.free(row);
        assert!(
            qty <= free,
            "Register C4: {qty} asked of {free} that is free — a short needs a borrow"
        );
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        let mut left = qty;
        let mut drawn = Vec::new();
        let mut first_live = 0usize;
        for i in 0..len {
            if left <= 0.0 {
                break;
            }
            let lot = self.lots[at + i];
            let take = if lot.qty <= left { lot.qty } else { left };
            drawn.push(Drawn {
                qty: take,
                basis_per_unit: lot.basis_per_unit,
                acquired: lot.acquired,
            });
            self.lots[at + i].qty -= take;
            left -= take;
            if self.lots[at + i].qty <= 0.0 {
                first_live = i + 1;
            }
        }
        // Emptied lots at the head are dropped by moving the row's start, not by shifting anything.
        if first_live > 0 {
            self.lot_at[row.row()] += first_live as u32;
            self.lot_len[row.row()] -= first_live as u32;
        }
        self.held[row.row()] -= qty;
        self.writes += 1;
        drawn
    }

    /// Money D2: money is one of itself, so its account is a total and has no lots to draw.
    pub fn money_delta(&mut self, holder: PartyId, instrument: InstrumentId, delta: f64) -> f64 {
        let row = self.open(holder, instrument);
        self.held[row.row()] += delta;
        self.writes += 1;
        self.held[row.row()]
    }

    /// Register C3: a claim over units, which refuses their move rather than adjusting it.
    pub fn pledge(&mut self, holder: PartyId, instrument: InstrumentId, to: PartyId, qty: f64) {
        let row = self.open(holder, instrument);
        let at = self.lien_at[row.row()] as usize;
        let len = self.lien_len[row.row()] as usize;
        if at + len == self.liens.len() {
            self.liens.push(Lien { to, qty });
        } else {
            let mine: Vec<Lien> = self.liens[at..at + len].to_vec();
            self.lien_at[row.row()] = self.liens.len() as u32;
            self.liens.extend_from_slice(&mine);
            self.liens.push(Lien { to, qty });
        }
        self.lien_len[row.row()] += 1;
        self.writes += 1;
    }
}

/// Law 19, Audit B2: the audit's own walk — it SUMS THE LOTS and compares them with the row's
/// quantity, which is the one place the maintained total is checked against what it is a total of.
/// A family reading `quantity` and calling that a check would be reading the answer (Audit C3).
pub fn lots_against_quantity(reg: &Register) -> Vec<(HoldingId, f64)> {
    let mut out = Vec::new();
    for row in reg.all() {
        let mut summed = 0.0;
        let mut magnitude = 0.0;
        let lots = reg.lots(row);
        for l in lots {
            summed += l.qty;
            magnitude += l.qty.abs();
        }
        let held = reg.quantity(row);
        let dust = (lots.len() as f64 + 2.0) * f64::EPSILON * (magnitude + held.abs());
        if (summed - held).abs() > dust {
            out.push((row, summed - held));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{InstrumentId, PartyId};

    #[test]
    fn units_arrive_with_their_basis_and_leave_oldest_first() {
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        reg.credit(p, i, 100.0, 1.5, 1);
        reg.credit(p, i, 50.0, 2.5, 2);
        let row = reg.row(p, i);
        assert_eq!(reg.quantity(row), 150.0);
        let drawn = reg.debit(row, 120.0);
        // Oldest first: 100 at 1.5, then 20 at 2.5.
        assert_eq!(drawn.len(), 2);
        assert_eq!(drawn[0].basis_per_unit, 1.5);
        assert_eq!(drawn[0].qty, 100.0);
        assert_eq!(drawn[1].basis_per_unit, 2.5);
        assert_eq!(drawn[1].qty, 20.0);
        assert_eq!(reg.quantity(row), 30.0);
        assert!(lots_against_quantity(&reg).is_empty());
    }

    #[test]
    #[should_panic(expected = "a short needs a borrow")]
    fn encumbered_units_do_not_move() {
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        let to = PartyId::at(1);
        reg.credit(p, i, 100.0, 1.0, 1);
        reg.pledge(p, i, to, 80.0);
        let row = reg.row(p, i);
        assert_eq!(reg.free(row), 20.0);
        reg.debit(row, 50.0);
    }

    #[test]
    fn both_directions_are_indexed_and_holdings_sum_to_issued() {
        let mut reg = Register::new();
        let i = InstrumentId::at(7);
        for p in 0..5u32 {
            reg.credit(PartyId::at(p), i, 10.0 * f64::from(p + 1), 1.0, 1);
        }
        assert_eq!(reg.of_instrument(i).len(), 5);
        assert_eq!(reg.of_holder(PartyId::at(3)).len(), 1);
        let (total, dust) = reg.held_total(i);
        assert!((total - 150.0).abs() <= dust);
    }
}
