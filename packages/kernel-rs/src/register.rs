//! The register: who holds what, with lots and liens; both directions indexed; holdings sum to
//! issued.

use crate::ids::{HoldingId, InstrumentId, PartyId};
use std::collections::HashMap;

/// Units carry the basis they were acquired at, so a disposal has a gain to book. A parcel of units
/// held at what it cost, on the date it arrived, is ONE thing: inventory, plant and everything else
/// held in lots is this row and not a second book beside it.
#[derive(Clone, Copy, PartialEq, Debug)]
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

/// UNITS LEAVE OLDEST FIRST, and each parcel keeps the basis it arrived with — which is what gives
/// a disposal a gain to book. Answers what was drawn, and how many lots at the head are now empty.
pub fn draw(lots: &mut [Lot], qty: f64) -> (Vec<Drawn>, usize) {
    let mut left = qty;
    let mut drawn = Vec::new();
    let mut first_live = 0usize;
    for (i, lot) in lots.iter_mut().enumerate() {
        if left <= 0.0 {
            break;
        }
        let take = if lot.qty <= left { lot.qty } else { left };
        drawn.push(Drawn {
            qty: take,
            basis_per_unit: lot.basis_per_unit,
            acquired: lot.acquired,
        });
        lot.qty -= take;
        left -= take;
        if lot.qty <= 0.0 {
            first_live = i + 1;
        }
    }
    (drawn, first_live)
}

/// Reduce a lot position's carrying basis pro rata without changing its units or vintage dates.
fn reduce_basis(lots: &mut [Lot], amount: f64) {
    assert!(
        amount.is_finite() && amount > 0.0,
        "Capital Programme D1: depreciation must be positive and finite"
    );
    let carrying: f64 = lots.iter().map(|lot| lot.qty * lot.basis_per_unit).sum();
    assert!(
        carrying > 0.0,
        "Capital Programme D1: depreciation needs a positive carrying basis"
    );
    assert!(
        amount <= carrying,
        "Capital Programme D1: depreciation exceeds carrying basis"
    );
    let factor = (carrying - amount) / carrying;
    for lot in lots {
        lot.basis_per_unit *= factor;
    }
}

/// What a debit drew, with the basis each parcel carried — settlement's own read for the gain.
#[derive(Clone, Copy)]
pub struct Drawn {
    pub qty: f64,
    pub basis_per_unit: f64,
    pub acquired: u32,
}

/// How one holder carries one position. The treatment belongs to the position, so two holders may
/// account for the same instrument differently.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Carrying {
    Market,
    Cost,
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
    carrying: Vec<Carrying>,

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

    pub fn pledged(&self, row: HoldingId) -> f64 {
        self.quantity(row) - self.free(row)
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

    /// Reduce the carrying basis of an existing lot position without moving its units or resetting
    /// its acquisition date. Settlement is the sole caller because depreciation is a booked event.
    pub(crate) fn depreciate(&mut self, row: HoldingId, amount: f64) {
        assert!(
            row.some() && !self.total_only[row.row()],
            "Capital Programme D1: depreciation needs a lot position"
        );
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        reduce_basis(&mut self.lots[at..at + len], amount);
        self.writes += 1;
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

    /// Voting power is the settled share quantity held by each holder of record.
    pub fn votes_of_record(&self, share: InstrumentId, issuer: PartyId) -> Vec<(PartyId, f64)> {
        self.of_instrument(share)
            .iter()
            .filter_map(|row| {
                let holding = HoldingId(*row);
                let holder = self.holder_of(holding);
                let units = self.quantity(holding);
                (holder != issuer && units > 0.0).then_some((holder, units))
            })
            .collect()
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
        assert!(
            instrument.some(),
            "Appendix B: no holding without an issuer"
        );
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
        self.carrying.push(Carrying::Cost);
        self.row_of.insert(k, row);
        self.by_holder.entry(holder.0).or_default().push(row);
        self.by_instrument
            .entry(instrument.0)
            .or_default()
            .push(row);
        HoldingId(row)
    }

    /// Declare one holder's treatment. Before acquisition this opens a zero position so settlement
    /// subsequently writes into the already-declared row.
    pub fn carry(&mut self, holder: PartyId, instrument: InstrumentId, as_: Carrying) -> HoldingId {
        let row = self.open(holder, instrument);
        self.carrying[row.row()] = as_;
        row
    }

    #[inline]
    pub fn carrying(&self, row: HoldingId) -> Carrying {
        self.carrying[row.row()]
    }

    /// Units arrive with the basis they cost.
    pub fn credit(
        &mut self,
        holder: PartyId,
        instrument: InstrumentId,
        qty: f64,
        basis_per_unit: f64,
        week: u32,
    ) -> HoldingId {
        assert!(qty > 0.0, "Register C1: a credit moves a positive quantity");
        let row = self.open(holder, instrument);
        // A lot is appended, so a holding's lots are contiguous only until it is written again.
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        if at + len == self.lots.len() {
            self.lots.push(Lot {
                qty,
                basis_per_unit,
                acquired: week,
            });
        } else {
            // Somebody else's lots are in the way: move this row's to the end of the column and grow
            // there.
            let mine: Vec<Lot> = self.lots[at..at + len].to_vec();
            self.lot_at[row.row()] = self.lots.len() as u32;
            self.lots.extend_from_slice(&mine);
            self.lots.push(Lot {
                qty,
                basis_per_unit,
                acquired: week,
            });
        }
        self.lot_len[row.row()] += 1;
        self.writes += 1;
        row
    }

    /// Units leave OLDEST FIRST, and what they cost goes with them.
    pub(crate) fn debit(&mut self, row: HoldingId, qty: f64) -> Vec<Drawn> {
        assert!(qty > 0.0, "Register C4: a debit moves a positive quantity");
        assert!(row.some(), "Register C4: nothing is held of this");
        let free = self.free(row);
        assert!(
            qty <= free,
            "Register C4: {qty} asked of {free} that is free — a short needs a borrow"
        );
        let at = self.lot_at[row.row()] as usize;
        let len = self.lot_len[row.row()] as usize;
        let (drawn, first_live) = draw(&mut self.lots[at..at + len], qty);
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
        assert!(
            to.some(),
            "Register C3: a lien is held BY somebody, so a release names them"
        );
        let row = self.row(holder, instrument);
        assert!(
            row.some(),
            "Register D5: nothing is pledged on a holding that does not exist"
        );
        let at = self.lien_at[row.row()] as usize;
        let len = self.lien_len[row.row()] as usize;
        let found = self.liens[at..at + len].iter().position(|l| l.to == to);
        let i = match found {
            Some(i) => at + i,
            None => {
                panic!("Register D5: there is no lien to release on this holding for that party")
            }
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

// A holding's quantity IS its lots, so asserting that it equals their sum compares a function with
// a copy of itself. The lien arithmetic, the index and the refusals — a short with no borrow, a
// lien released that is not there, a release larger than the lien — all relate an argument to what
// the store already holds, so each panics at its site and none of them has a test.
//
// What IS arithmetic is the draw, and it is a pure function now.

#[cfg(test)]
mod tests {
    use super::*;

    fn lot(qty: f64, basis: f64, at: u32) -> Lot {
        Lot {
            qty,
            basis_per_unit: basis,
            acquired: at,
        }
    }

    #[test]
    fn units_leave_oldest_first_and_each_parcel_keeps_the_basis_it_arrived_with() {
        let mut lots = [lot(100.0, 1.5, 1), lot(50.0, 2.5, 2)];
        let (drawn, empty) = draw(&mut lots, 120.0);
        assert_eq!(drawn.len(), 2);
        assert_eq!((drawn[0].qty, drawn[0].basis_per_unit), (100.0, 1.5));
        assert_eq!((drawn[1].qty, drawn[1].basis_per_unit), (20.0, 2.5));
        // The first lot is spent, so the row starts one later; 30 is left at 2.5.
        assert_eq!(empty, 1);
        assert_eq!(lots.iter().map(|l| l.qty).sum::<f64>(), 30.0);
    }

    #[test]
    fn a_draw_that_one_lot_covers_leaves_that_lot_standing() {
        let mut lots = [lot(100.0, 1.5, 1)];
        let (drawn, empty) = draw(&mut lots, 40.0);
        assert_eq!(drawn.len(), 1);
        assert_eq!(drawn[0].qty, 40.0);
        assert_eq!(empty, 0, "a lot with units left in it is not empty");
        assert_eq!(lots[0].qty, 60.0);
    }

    #[test]
    fn drawing_the_whole_of_a_holding_empties_every_lot() {
        let mut lots = [lot(10.0, 1.0, 1), lot(20.0, 2.0, 2), lot(30.0, 3.0, 3)];
        let (drawn, empty) = draw(&mut lots, 60.0);
        assert_eq!(drawn.len(), 3);
        assert_eq!(empty, 3);
        assert_eq!(lots.iter().map(|l| l.qty).sum::<f64>(), 0.0);
    }

    #[test]
    fn depreciation_reduces_basis_without_replacing_the_plant_lots() {
        let mut lots = [lot(2.0, 100.0, 3), lot(1.0, 200.0, 7)];

        reduce_basis(&mut lots, 100.0);

        assert_eq!(lots, [lot(2.0, 75.0, 3), lot(1.0, 150.0, 7)]);
    }

    #[test]
    fn voting_power_is_the_settled_share_quantity_of_each_holder_of_record() {
        let issuer = PartyId::at(1);
        let share = InstrumentId::at(4);
        let mut register = Register::default();
        register.credit(PartyId::at(2), share, 30.0, 8.0, 0);
        register.credit(PartyId::at(3), share, 70.0, 8.0, 0);
        assert_eq!(
            register.votes_of_record(share, issuer),
            vec![(PartyId::at(2), 30.0), (PartyId::at(3), 70.0)]
        );
    }

    #[test]
    fn pledged_collateral_reconciles_with_total_and_free_units() {
        let holder = PartyId::at(2);
        let line = InstrumentId::at(5);
        let mut register = Register::default();
        let row = register.credit(holder, line, 100.0, 4.0, 0);
        register.pledge(holder, line, PartyId::at(9), 35.0);
        assert_eq!(register.free(row), 65.0);
        assert_eq!(register.pledged(row), 35.0);
        assert_eq!(
            register.free(row) + register.pledged(row),
            register.quantity(row)
        );
    }
}

/// Banks Lending D1, XI-1: WHAT A CLAIM IS, as a status that is WRITTEN rather than inferred.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Standing {
    Performing,
    /// A payment was missed.
    NonPerforming {
        since: u32,
    },
    /// The holder has written down what it believes it will not get.
    Impaired {
        since: u32,
    },
    /// It is gone from the book, on a date, and whatever was seized is a separate holding.
    WrittenOff {
        on: u32,
    },
}
