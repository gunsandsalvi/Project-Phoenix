//! The register: who holds what, with lots and liens; both directions indexed; holdings sum to
//! issued.

use crate::ids::{HoldingId, InstrumentId, PartyId};
use std::collections::HashMap;

/// Units carry the basis they were acquired at, so a disposal has a gain to book.
#[derive(Clone, Copy)]
pub struct Lot {
    pub qty: f64,
    pub basis_per_unit: f64,
    pub acquired: u32,
}

/// Units somebody else has a claim over.
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
    // One row per holding.
    holder: Vec<u32>,
    instrument: Vec<u32>,
    lot_at: Vec<u32>,
    lot_len: Vec<u32>,
    lien_at: Vec<u32>,
    lien_len: Vec<u32>,
    /// What a MONEY account holds, and nothing else's total.
    total: Vec<f64>,
    /// MONEY IS ONE OF ITSELF, so its account is a TOTAL and has no lots to draw.
    total_only: Vec<bool>,

    lots: Vec<Lot>,
    liens: Vec<Lien>,

    /// (holder, instrument) -> row.
    row_of: HashMap<u64, u32>,
    /// Both directions indexed: the rows of one holder, and the rows of one line.
    by_holder: HashMap<u32, Vec<u32>>,
    by_instrument: HashMap<u32, Vec<u32>>,

    /// The write count a reader keys a kept answer on.
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

    /// The row for this pair, or `HoldingId::NONE`.
    #[inline]
    pub fn row(&self, holder: PartyId, instrument: InstrumentId) -> HoldingId {
        match self.row_of.get(&key(holder, instrument)) {
            Some(&row) => HoldingId(row),
            None => HoldingId::NONE,
        }
    }

    /// What the register holds is whole pieces, so what it reads back is a count of them.
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

    /// What is not encumbered.
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

    /// The lots of one holding, in the order they were acquired.
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

    /// Every holding, as rows.
    #[inline]
    pub fn all(&self) -> impl Iterator<Item = HoldingId> + '_ {
        (0..self.holder.len() as u32).map(HoldingId)
    }

    /// The rows of one holder, and of one line.
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

    /// The sum of what is held of a line, over its own index, with the dust of the walk.
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


    /// The row for this pair, opened if there is none.
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

    /// Units arrive with the basis they cost.
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
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        if at + len == self.lots.len() {
            self.lots.push(Lot { qty, basis_per_unit, acquired: period });
        } else {
            // Somebody else's lots are in the way: move this row's to the end of the column and grow
            // there.
            let mine: Vec<Lot> = self.lots[at..at + len].to_vec();
            self.lot_at[row.row()] = self.lots.len() as u32;
            self.lots.extend_from_slice(&mine);
            self.lots.push(Lot { qty, basis_per_unit, acquired: period });
        }
        self.lot_len[row.row()] += 1;
        self.writes += 1;
        row
    }

    /// Units leave OLDEST FIRST, and what they cost goes with them.
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

    /// Money is one of itself, so its account is a total and has no lots to draw.
    pub fn money_delta(&mut self, holder: PartyId, instrument: InstrumentId, delta: f64) -> f64 {
        let row = self.open(holder, instrument);
        self.total_only[row.row()] = true;
        self.total[row.row()] += delta;
        self.writes += 1;
        self.total[row.row()]
    }

    /// A claim over units, which refuses their move rather than adjusting it.
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

    /// A lien comes off the way it went on, and releasing one that is not there throws.
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{InstrumentId, PartyId};

    #[test]
    fn a_holdings_quantity_is_its_lots_and_cannot_drift_from_them() {
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

        // The one row that answers from a total, and it is not an exception — a money account has no
        // lots, so the total is a copy of nothing.
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
        // And the row answers 30 because the lots say 30.
        assert_eq!(reg.lots(row).iter().map(|l| l.qty).sum::<f64>(), 30.0);
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
        // The (24, 96) world stopped on exactly this, and stopping is right — a return that releases
        // a lien already gone means the row it thinks it is on is not the row it is on.
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
        // It is not clamped to what is there.
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

/// Banks Lending D1, XI-1: WHAT A CLAIM IS, as a status that is WRITTEN rather than inferred.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standing {
    Performing,
    /// A payment was missed.
    NonPerforming { since: u32 },
    /// The holder has written down what it believes it will not get.
    Impaired { since: u32 },
    /// It is gone from the book, on a date, and whatever was seized is a separate holding.
    WrittenOff { on: u32 },
}
