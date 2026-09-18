//! The register: who holds what, with lots and liens; both directions indexed; holdings sum to
//! issued (Register A1–B4).
//!
//! A holding is a ROW and its lots are a counted slice of one flat column. That is the whole of the
//! difference from the TypeScript store, and it is what the calibration measured: a traversal of
//! 544,104 holdings is **95.70 ms** there and **2.75 ms** here, and the hot read with the row in
//! hand is **67.20 ns** against **19.47 ns** (`register-at-scale`).
//!
//! **Those two numbers were 0.39 ms and 3.99 ns until 22e2**, when the running total beside the lots
//! was deleted and a holding's quantity became a READ of them (Law 19). The read costs five times
//! what a tally cost and the traversal seven times, and the assembled world's period is unchanged
//! at ~350 ms — because the register read was never the period's bottleneck. Law 18 is satisfied
//! the way it asks to be: behaviour identical, correctness better, the cost measured rather than
//! assumed, and no cache added for a cost nothing pays.
//!
//! Law 4: settlement is the ONE writer. The reads are open to anybody; `debit`, `credit` and
//! `move_money` are reached only through the wire.

use crate::ids::{HoldingId, InstrumentId, PartyId};
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
    /// **Money D2: what a MONEY account holds, and nothing else's total.** A money account has no
    /// lots — one unit of it is every other unit — so there is nothing to sum and the total IS the
    /// holding. `money_delta` is its one writer, and for any row that carries lots this column is
    /// not read at all.
    ///
    /// **It used to be every row's total, and that was two writers of one quantity** (Law 4, 22e2).
    /// The old comment argued it was not a stored aggregate in Appendix B's sense — and the defect
    /// was not that: it was Law 19. `quantity()` answered from a tally kept BESIDE the lots rather
    /// than from the lots, so float addition over many periods drifted the two apart, and the
    /// ownership family — which reads the lots and compares them with the total — found exactly that
    /// on the audit's first period in the loop: *lots sum to 27.143341836734685 and the row holds
    /// 27.143341836734628*. The fix is the deletion: there is one writer of a holding's quantity
    /// because there is only one place it is written down.
    total: Vec<f64>,
    /// Money D2: MONEY IS ONE OF ITSELF, so its account is a TOTAL and has no lots to draw. A row
    /// is one or the other and the register says which, because a family that summed the lots of a
    /// money account would report every account in the world as a violation — which is exactly what
    /// the first end-to-end period did, 10,318 times.
    total_only: Vec<bool>,

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
    /// **Law 19, 22e2: it is READ from the lots, which are the source.** A holding's quantity used
    /// to be a running total `credit` added to and `debit` subtracted from while the lots carried
    /// the same number — two writers, and they drifted.
    ///
    /// Money D2 is the one row that answers from a total, and that is not an exception: a money
    /// account has no lots, so the total is not a copy of anything.
    #[inline]
    pub fn quantity(&self, row: HoldingId) -> f64 {
        if !row.some() {
            return 0.0;
        }
        if self.total_only[row.row()] {
            return self.total[row.row()];
        }
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        self.lots[at..at + len].iter().map(|l| l.qty).sum()
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
        self.quantity(row) - pledged
    }

    /// The lots of one holding, in the order they were acquired (Register D1).
    /// Money D2: whether this row is a TOTAL with no lots, or a holding that carries them.
    #[inline]
    pub fn is_total(&self, row: HoldingId) -> bool {
        row.some() && self.total_only[row.row()]
    }

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
            let q = self.quantity(HoldingId(row));
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
        self.total.push(0.0);
        self.total_only.push(false);
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
        self.writes += 1;
        drawn
    }

    /// Money D2: money is one of itself, so its account is a total and has no lots to draw.
    pub fn money_delta(&mut self, holder: PartyId, instrument: InstrumentId, delta: f64) -> f64 {
        let row = self.open(holder, instrument);
        self.total_only[row.row()] = true;
        self.total[row.row()] += delta;
        self.writes += 1;
        self.total[row.row()]
    }

    /// Register C3: a claim over units, which refuses their move rather than adjusting it.
    ///
    /// **One lien per (holding, holder)** (Law 4, 21.131.BF4): pledging again to the same party adds
    /// to the claim that party already has rather than writing a second row beside it. Two rows with
    /// the same holder would be two answers to *what does this party have a claim over*, and a
    /// release that matched by holder would find whichever came first — which is BF4's defect exactly
    /// (two identical rows are indistinguishable, so a pairing by value picks the wrong one). It was
    /// found by re-reading BF4 against `release`, which this session had written an hour earlier.
    pub fn pledge(&mut self, holder: PartyId, instrument: InstrumentId, to: PartyId, qty: f64) {
        assert!(to.some(), "Register C3: a lien is held BY somebody");
        let row = self.open(holder, instrument);
        let at = self.lien_at[row.row()] as usize;
        let len = self.lien_len[row.row()] as usize;
        if let Some(i) = self.liens[at..at + len].iter().position(|l| l.to == to) {
            self.liens[at + i].qty += qty;
            self.writes += 1;
            return;
        }
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

    /// Register C3, D5, 21.115: **a lien comes off the way it went on, and releasing one that is not
    /// there throws.** There was no release at all until now, which meant that in this world units
    /// pledged were pledged for ever: a securities loan could be returned, a repo could mature and a
    /// margin call could be reversed, and the claim over the units stayed on the row. `free` would
    /// answer for ever with the encumbrance of a relation that had ended.
    ///
    /// Law 6: it does not clamp a release to what is there. Releasing more than was pledged, or
    /// releasing to a party that holds no lien on this row, is not a smaller release — it is somebody
    /// reading the wrong row, and the citation says which.
    pub fn release(&mut self, holder: PartyId, instrument: InstrumentId, to: PartyId, qty: f64) {
        assert!(to.some(), "Register C3: a lien is held BY somebody, so a release names them");
        let row = self.row(holder, instrument);
        assert!(row.some(), "Register D5: nothing is pledged on a holding that does not exist");
        let at = self.lien_at[row.row()] as usize;
        let len = self.lien_len[row.row()] as usize;
        let found = self.liens[at..at + len].iter().position(|l| l.to == to);
        let i = match found {
            Some(i) => at + i,
            None => panic!("Register D5: there is no lien to release on this holding for that party"),
        };
        let have = self.liens[i].qty;
        assert!(
            qty <= have,
            "Register D5: releasing {qty} against a lien of {have} is a read of the wrong row"
        );
        self.liens[i].qty = have - qty;
        // A lien of nothing is not a lien, so the row stops carrying it (Law 2: missing is missing).
        if self.liens[i].qty == 0.0 {
            self.liens[i].to = PartyId::NONE;
        }
        self.writes += 1;
    }
}

/// Law 19, Audit B2: the audit's own walk — it SUMS THE LOTS and compares them with the row's
/// quantity, which is the one place the maintained total is checked against what it is a total of.
/// A family reading `quantity` and calling that a check would be reading the answer (Audit C3).
pub fn lots_against_quantity(reg: &Register) -> Vec<(HoldingId, f64)> {
    let mut out = Vec::new();
    for row in reg.all() {
        if reg.is_total(row) {
            continue;
        }
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
    fn a_holdings_quantity_is_its_lots_and_cannot_drift_from_them() {
        // **Law 4, Law 19, 22e2: ONE WRITER of a holding's quantity.** It was a running total that
        // `credit` added to and `debit` subtracted from while the lots carried the same number, and
        // over enough float arithmetic the two drifted apart — which is what the ownership family
        // found on the audit's first period in the loop.
        //
        // The drift cannot be asserted away with a band (Law 7). It is unsayable now, because there
        // is only one place the number is written down: whatever the lots say, the quantity IS.
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        // Amounts that do not land on a power of two, credited and drawn many times over — the shape
        // the arbitrary world produces and the shape that drifted.
        for n in 1..200u32 {
            reg.credit(p, i, 1.0 / 3.0 + f64::from(n) / 7.0, 0.5, n);
            let row = reg.row(p, i);
            if n % 3 == 0 {
                reg.debit(row, reg.quantity(row) / 11.0);
            }
        }
        let row = reg.row(p, i);
        let summed: f64 = reg.lots(row).iter().map(|l| l.qty).sum();
        assert_eq!(reg.quantity(row), summed, "the quantity IS the lots, not a tally beside them");

        // Money D2: the one row that answers from a total, and it is not an exception — a money
        // account has no lots, so the total is a copy of nothing.
        let cash = InstrumentId::at(1);
        reg.money_delta(p, cash, 900.0);
        reg.money_delta(p, cash, -250.0);
        let account = reg.row(p, cash);
        assert!(reg.is_total(account));
        assert_eq!(reg.quantity(account), 650.0);
        assert!(reg.lots(account).is_empty());
    }

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
    fn a_lien_comes_off_the_way_it_went_on() {
        // 21.115, Register C3/D5: there was no release at all, so units pledged were pledged for
        // ever — a securities loan could be returned and the claim over the units stayed on the row.
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        let to = PartyId::at(1);
        reg.credit(p, i, 100.0, 1.0, 1);
        reg.pledge(p, i, to, 80.0);
        let row = reg.row(p, i);
        assert_eq!(reg.free(row), 20.0);

        // Part of it back: the rest of the claim stands.
        reg.release(p, i, to, 30.0);
        assert_eq!(reg.free(row), 50.0);
        // And the whole of the rest: the units are the holder's again and they move.
        reg.release(p, i, to, 50.0);
        assert_eq!(reg.free(row), 100.0);
        assert_eq!(reg.debit(row, 100.0).len(), 1);
    }

    #[test]
    fn one_holder_has_one_lien_on_one_holding_however_often_it_is_pledged_to() {
        // Law 4, 21.131.BF4: two rows with the same holder are two answers to "what does this party
        // have a claim over", and a release that matched by holder would find whichever came first.
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        let to = PartyId::at(1);
        reg.credit(p, i, 100.0, 1.0, 1);
        reg.pledge(p, i, to, 30.0);
        reg.pledge(p, i, to, 20.0);
        let row = reg.row(p, i);
        assert_eq!(reg.free(row), 50.0, "the two pledges are one claim of fifty");
        // And the whole of it comes off in one release, which it could not if there were two rows.
        reg.release(p, i, to, 50.0);
        assert_eq!(reg.free(row), 100.0);
    }

    #[test]
    #[should_panic(expected = "no lien to release")]
    fn releasing_a_lien_that_is_not_there_is_a_read_of_the_wrong_row() {
        // 21.115: the (24, 96) world stopped on exactly this, and stopping is right — a return that
        // releases a lien already gone means the row it thinks it is on is not the row it is on.
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        reg.credit(p, i, 100.0, 1.0, 1);
        reg.pledge(p, i, PartyId::at(1), 80.0);
        reg.release(p, i, PartyId::at(2), 10.0);
    }

    #[test]
    #[should_panic(expected = "a read of the wrong row")]
    fn releasing_more_than_was_pledged_is_not_a_smaller_release() {
        // Law 6: it is not clamped to what is there.
        let mut reg = Register::new();
        let p = PartyId::at(0);
        let i = InstrumentId::at(0);
        let to = PartyId::at(1);
        reg.credit(p, i, 100.0, 1.0, 1);
        reg.pledge(p, i, to, 40.0);
        reg.release(p, i, to, 60.0);
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

/// Banks Lending D1, XI-1: **WHAT A CLAIM IS**, as a status that is WRITTEN rather than inferred.
/// Each step is a crossing with a date, and a claim does not slide between them by arithmetic.
///
/// It lives in the KERNEL and not in a module, because more than one module reads it — the lender
/// writes it, a pool reads it, a credit-default swap triggers on it, a resolution values a book by
/// it — and a fact two modules share is the kernel's, never one module's for another to import
/// (Law 15). `phoenix-check` is what said so: `lending` reached into `loss` for this, and the
/// module that does that has made the other one part of its own contract.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standing {
    Performing,
    /// A payment was missed. The claim is still whole; what changed is what is known about it.
    NonPerforming { since: u32 },
    /// The holder has written down what it believes it will not get. Banks Lending D2: a charge to
    /// income that is VISIBLE, never a reserve absorbing things quietly.
    Impaired { since: u32 },
    /// It is gone from the book, on a date, and whatever was seized is a separate holding.
    WrittenOff { on: u32 },
}
