use phx_core::kind_tables::ListKind;
use phx_id::{Day, InstrumentId, LineId, Slot};
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Amount, Missing, QtyRaw, capacity_exceeded, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::Ledger;
use crate::contract_process::ArrearsKey;
use crate::holder::HolderArenas;
use crate::holding::CellHolding;
use crate::rows::{self, Optional, RelRow, RowView};
use crate::words::{from_words, to_words, words_of};

/// A row a part carries between its split and its landing: its members' count and their share of the row's words,
/// with the payment record they share and the day its arrears began, if they are in arrears. Its members stay on the
/// line, so the line's side counts never move with it.
#[clause("REP.8", "REP.14")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetachedRow {
    pub row: RelRow,
    pub optional: Optional,
    pub arrears_since: Missing<Day>,
}

impl DetachedRow {
    pub fn line(&self) -> LineId {
        self.row.line
    }

    #[must_use]
    pub fn side(&self) -> Side {
        rows::side_of(self.row.role)
    }
}

/// Members leaving a row: how many, and an amount of its balance that is theirs alone — what a flow that reached only
/// them moved — which leaves whole with them before the rest is shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowShare {
    pub count: u32,
    pub own_balance: i64,
}

fn word_share(total: Missing<i64>, own: i64, share: RowShare, of: u32, r: Round) -> (Missing<i64>, Missing<i64>) {
    let Missing::Present(t) = total else {
        if own != 0 {
            violation!(clause = "REP.9", "an amount of their own leaving a row that keeps no balance", own = own);
        }
        return (Missing::Absent, Missing::Absent);
    };
    let Some(shared) = t.checked_sub(own) else {
        violation!(clause = "Law 7", "a row's shared balance overflows", total = t, own = own);
    };
    let (leaving, staying) = split_total(shared, u64::from(share.count), u64::from(of), r);
    let Some(leaving) = leaving.checked_add(own) else {
        violation!(clause = "Law 7", "a leaving share overflows", leaving = leaving, own = own);
    };
    (Missing::Present(leaving), Missing::Present(staying))
}

fn sum(held: Missing<i64>, joining: Missing<i64>, line: LineId) -> Missing<i64> {
    match (held, joining) {
        (Missing::Present(x), Missing::Present(y)) => {
            let Some(total) = x.checked_add(y) else {
                violation!(clause = "Law 7", "a joined row's word overflows", line = line.get());
            };
            Missing::Present(total)
        }
        (Missing::Absent, Missing::Absent) => Missing::Absent,
        _ => violation!(clause = "REP.3", "rows of one line side joined with other optional words", line = line.get()),
    }
}

const HOLDING: usize = words_of::<CellHolding>();

/// A cell's holdings, in the order of its list.
#[must_use]
pub fn cell_holdings(arenas: &dyn HolderArenas, holder: Slot) -> Vec<CellHolding> {
    arenas.read(holder, ListKind::Holdings).as_chunks::<HOLDING>().0.iter().map(|w| from_words(w)).collect()
}

impl<B: Backing> Ledger<B> {
    /// Members leave a cell's row for a part, taking their share of each of its words by the rounding given, the rest
    /// staying; all of its members leaving take the row itself, and the cell leaves the line's list if it was its last
    /// listed row there. The members stay on the line, so its side counts do not move and no total moves.
    #[clause("REP.8", "REP.9", "REP.14")]
    pub fn detach_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        (line, side): (LineId, Side),
        share: RowShare,
        rounding: Round,
    ) -> DetachedRow {
        let view: RowView = crate::line::Lines::<B>::find(arenas, holder, line, side);
        let of = view.row.count;
        if share.count == 0 || share.count > of {
            violation!(clause = "REP.9", "members leaving a row that does not hold them", leaving = share.count, of = of);
        }
        let key = ArrearsKey::new(line, side, arenas.party(holder));
        let since = match self.arrears.since(key) {
            Some(d) => Missing::Present(d),
            None => Missing::Absent,
        };
        if share.count == of {
            // All its members leaving take the row whole, their own amount within its balance.
            let whole = self.lines.unplace_row(arenas, table, holder, line, side);
            self.arrears.remove(key);
            return DetachedRow { row: whole.row, optional: whole.optional, arrears_since: since };
        }
        let (bl, bs) = word_share(view.optional.balance, share.own_balance, share, of, rounding);
        let (pl, ps) = word_share(view.optional.pending, 0, share, of, rounding);
        let (al, a_s) = word_share(view.optional.amount, 0, share, of, rounding);
        let mut stay = view;
        stay.row.count = of - share.count;
        let staying = Optional { balance: bs, pending: ps, amount: a_s };
        rows::rewrite(arenas, holder, &stay, staying);
        let mut leaving = view.row;
        leaving.count = share.count;
        DetachedRow { row: leaving, optional: Optional { balance: bl, pending: pl, amount: al }, arrears_since: since }
    }

    /// A part's row joins a cell: added to the cell's row on the same line side, whose payment record it must share,
    /// or placed as the cell's first row there, the cell entering the line's list with its first listed row. The
    /// line's side counts do not move.
    #[clause("REP.8", "REP.14")]
    pub fn attach_row(&mut self, arenas: &mut dyn HolderArenas, table: u16, holder: Slot, detached: DetachedRow) {
        let (line, side) = (detached.line(), detached.side());
        let key = ArrearsKey::new(line, side, arenas.party(holder));
        let found = rows::iter(arenas, holder).find(|r| r.row.line == line && r.side() == side);
        let Some(mut view) = found else {
            self.lines.place_row(arenas, table, holder, detached.row, detached.optional);
            if let Missing::Present(d) = detached.arrears_since {
                self.arrears.begin(key, d);
            }
            return;
        };
        if view.row.record != detached.row.record || view.row.point != detached.row.point {
            violation!(clause = "REP.8", "rows of other payment records or points joined", line = line.get());
        }
        let since = match self.arrears.since(key) {
            Some(d) => Missing::Present(d),
            None => Missing::Absent,
        };
        if since != detached.arrears_since {
            violation!(clause = "REP.8", "rows in arrears since other days joined", line = line.get());
        }
        let Some(count) = view.row.count.checked_add(detached.row.count) else {
            capacity_exceeded!("members of a row", u32::MAX, view.row.count);
        };
        let joined = Optional {
            balance: sum(view.optional.balance, detached.optional.balance, line),
            pending: sum(view.optional.pending, detached.optional.pending, line),
            amount: sum(view.optional.amount, detached.optional.amount, line),
        };
        view.row.count = count;
        rows::rewrite(arenas, holder, &view, joined);
    }

    /// Members leave a cell's holding for a part: `count` of the members holding it, taking their share of its units
    /// and of its pooled cost; all of them leaving take the holding, and the cell leaves the instrument's list.
    #[clause("REP.8", "REP.9", "REP.14")]
    pub fn detach_holding(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        instrument: InstrumentId,
        count: u32,
        rounding: Round,
    ) -> CellHolding {
        let all = cell_holdings(arenas, holder);
        let Some((i, h)) = all.iter().copied().enumerate().find(|(_, h)| h.instrument == instrument) else {
            violation!(clause = "REG.16", "a part leaving a holding its cell does not have", id = instrument.get());
        };
        if count == 0 || count > h.count {
            violation!(clause = "REP.9", "members leaving a holding that does not hold them", leaving = count);
        }
        if count == h.count {
            arenas.remove(holder, ListKind::Holdings, i * HOLDING, HOLDING);
            self.instruments.unlist(table, holder, instrument);
            return h;
        }
        let (q_leave, q_stay) = split_total(h.quantity.raw(), u64::from(count), u64::from(h.count), rounding);
        let (c_leave, c_stay) = split_total(h.pooled_cost.raw(), u64::from(count), u64::from(h.count), rounding);
        let staying = CellHolding {
            instrument,
            count: h.count - count,
            quantity: QtyRaw::from_raw(q_stay),
            pooled_cost: Amount::from_raw(c_stay),
        };
        arenas.overwrite(holder, ListKind::Holdings, i * HOLDING, &to_words(&staying));
        CellHolding { instrument, count, quantity: QtyRaw::from_raw(q_leave), pooled_cost: Amount::from_raw(c_leave) }
    }

    /// A part's holding joins a cell's: members, units and pooled cost add, or it becomes the cell's holding of the
    /// instrument, the cell entering the instrument's list.
    #[clause("REP.8", "REP.14")]
    pub fn attach_holding(&mut self, arenas: &mut dyn HolderArenas, table: u16, holder: Slot, part: CellHolding) {
        let all = cell_holdings(arenas, holder);
        let Some((i, h)) = all.iter().copied().enumerate().find(|(_, h)| h.instrument == part.instrument) else {
            arenas.append(holder, ListKind::Holdings, &to_words(&part));
            self.instruments.enlist(table, holder, part.instrument);
            return;
        };
        let add = |a: i64, b: i64| {
            let Some(s) = a.checked_add(b) else {
                violation!(clause = "Law 7", "a joined holding overflows", id = part.instrument.get());
            };
            s
        };
        let Some(count) = h.count.checked_add(part.count) else {
            capacity_exceeded!("members holding an instrument", u32::MAX, h.count);
        };
        let joined = CellHolding {
            instrument: h.instrument,
            count,
            quantity: QtyRaw::from_raw(add(h.quantity.raw(), part.quantity.raw())),
            pooled_cost: Amount::from_raw(add(h.pooled_cost.raw(), part.pooled_cost.raw())),
        };
        arenas.overwrite(holder, ListKind::Holdings, i * HOLDING, &to_words(&joined));
    }
}

#[cfg(test)]
mod tests {
    use phx_num::round::Round;
    use phx_num::{Amount, Ccy, Missing, QtyRaw, UnitId};
    use phx_store::AddressSpace;

    use super::{DetachedRow, RowShare, cell_holdings};
    use crate::algebra::Side;
    use crate::apply::Ledger;
    use crate::holder::HolderKeys;
    use crate::holding::CellHolding;
    use crate::instrument::{InstrumentFamily, Instruments, NewInstrument};
    use crate::line::{LineKindDecl, Lines, NewRow, SideDecl};
    use crate::rows::{BALANCE, Optional, PaymentRecord, rows};
    use crate::terms::TermsId;
    use crate::tests::{Heap, Table, holder, table};

    const LOAN: LineKindDecl = LineKindDecl {
        name: "loan",
        asset: SideDecl { holder_kinds: &["bank"], words: 0, holder_list: true },
        liability: SideDecl { holder_kinds: &["household"], words: BALANCE, holder_list: true },
        transfer_requesters: &["BNK"],
        dated: false,
    };

    fn ledger(space: &mut AddressSpace) -> Ledger<Heap> {
        let keys = HolderKeys::new(2);
        Ledger::new(Instruments::new(space, 16, 16, 16, keys), Lines::new(space, 16, 16, 16, keys))
    }

    fn balance(t: &Table, s: phx_id::Slot) -> Vec<(u32, Missing<i64>)> {
        rows(t, s).iter().map(|r| (r.row.count, r.optional.balance)).collect()
    }

    #[test]
    fn rows_leave_and_join_without_moving_a_total() {
        let mut space = AddressSpace::empty();
        let mut cells = table(&mut space, "household", 1);
        let (a, b) = (holder(&mut space, &mut cells, 1), holder(&mut space, &mut cells, 2));
        let mut l = ledger(&mut space);
        let kind = l.lines.declare_money(LOAN).index();
        let line = l.lines.open(kind, TermsId::new(0), Missing::Absent);
        let optional = Optional { balance: Missing::Present(1_001), ..Optional::NONE };
        l.lines.add_row(&mut cells, 1, a, line, NewRow { side: Side::Liability, within: 0, count: 10, point: 0, optional });
        let at = (line, Side::Liability);
        let three = l.detach_row(&mut cells, 1, a, at, RowShare { count: 3, own_balance: 0 }, Round::HalfEven);
        assert_eq!((three.row.count, three.optional.balance), (3, Missing::Present(300)), "3 × 1001 ÷ 10 = 300.3");
        assert_eq!(balance(&cells, a), [(7, Missing::Present(701))], "the rest stays");
        assert_eq!(l.lines.side_count(line, Side::Liability), 10, "leavers stay on the line");
        l.attach_row(&mut cells, 1, b, three);
        assert_eq!(l.lines.holders(line).count(), 2, "the first row there enters the list");
        let own = RowShare { count: 2, own_balance: -50 };
        let two = l.detach_row(&mut cells, 1, a, at, own, Round::HalfEven);
        assert_eq!(two.optional.balance, Missing::Present(-50 + 215), "their own −50, then 2 × 751 ÷ 7 = 214.57");
        let rest = l.detach_row(&mut cells, 1, a, at, RowShare { count: 5, own_balance: 0 }, Round::HalfEven);
        assert!(rows(&cells, a).is_empty(), "all its members take the row");
        assert_eq!(l.lines.holders(line).count(), 1, "and its holder leaves the list");
        l.attach_row(&mut cells, 1, b, two);
        l.attach_row(&mut cells, 1, b, rest);
        assert_eq!(balance(&cells, b), [(10, Missing::Present(1_001))]);
        assert_eq!(l.lines.side_count(line, Side::Liability), 10);
    }

    #[test]
    fn rows_of_other_records_never_join() {
        let mut space = AddressSpace::empty();
        let mut cells = table(&mut space, "household", 1);
        let (a, b) = (holder(&mut space, &mut cells, 1), holder(&mut space, &mut cells, 2));
        let mut l = ledger(&mut space);
        let kind = l.lines.declare_money(LOAN).index();
        let line = l.lines.open(kind, TermsId::new(0), Missing::Absent);
        let optional = Optional { balance: Missing::Present(100), ..Optional::NONE };
        for h in [a, b] {
            let new = NewRow { side: Side::Liability, within: 0, count: 4, point: 0, optional };
            l.lines.add_row(&mut cells, 1, h, line, new);
        }
        let mut part: DetachedRow =
            l.detach_row(&mut cells, 1, a, (line, Side::Liability), RowShare { count: 1, own_balance: 0 }, Round::Floor);
        part.row.record = PaymentRecord { arrears_days: 3, missed: 1 }.packed();
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| l.attach_row(&mut cells, 1, b, part)));
        let payload = caught.expect_err("a record is kept, never averaged");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REP.8"));
    }

    #[test]
    fn holdings_leave_and_join_at_pooled_cost() {
        let mut space = AddressSpace::empty();
        let mut cells = table(&mut space, "household", 1);
        let (a, b) = (holder(&mut space, &mut cells, 1), holder(&mut space, &mut cells, 2));
        let mut l = ledger(&mut space);
        let fund = l.instruments.issue(NewInstrument {
            family: InstrumentFamily::FundUnit,
            issuer: Missing::Present(phx_id::PartyId::new(9)),
            unit: UnitId::new(0),
            ccy: Ccy::new(0),
            terms: TermsId::new(0),
        });
        let h = |count, q, c| CellHolding {
            instrument: fund,
            count,
            quantity: QtyRaw::from_raw(q),
            pooled_cost: Amount::from_raw(c),
        };
        l.attach_holding(&mut cells, 1, a, h(6, 600, 1_205));
        assert_eq!(l.instruments.holders(fund).count(), 1);
        let part = l.detach_holding(&mut cells, 1, a, fund, 2, Round::HalfEven);
        assert_eq!(part, h(2, 200, 402), "a third of the units and of the pooled cost, 401.67 rounded");
        assert_eq!(cell_holdings(&cells, a), [h(4, 400, 803)]);
        l.attach_holding(&mut cells, 1, b, part);
        let rest = l.detach_holding(&mut cells, 1, a, fund, 4, Round::HalfEven);
        assert!(cell_holdings(&cells, a).is_empty());
        l.attach_holding(&mut cells, 1, b, rest);
        assert_eq!(cell_holdings(&cells, b), [h(6, 600, 1_205)]);
        assert_eq!(l.instruments.holders(fund).count(), 1, "the holder list follows the holding");
    }
}
