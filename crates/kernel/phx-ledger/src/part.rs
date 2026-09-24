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

/// One part's rows leaving a cell: the members leaving each line side, and the rounding their shares take.
pub type RowPlan<'a> = (&'a [((LineId, Side), RowShare)], Round);

/// Several parts' rows as one per line side, so parts joining one cell together attach each line once: members and
/// words add; rows of other payment records, price points or arrears never merge.
#[clause("REP.8", "REP.14")]
#[must_use]
pub fn merge_rows(mut rows: Vec<DetachedRow>) -> Vec<DetachedRow> {
    // Rows of one line side add, which no order changes, so the sort need not keep their order.
    rows.sort_unstable_by_key(|d| (d.line(), d.side() == Side::Liability));
    let mut out: Vec<DetachedRow> = Vec::with_capacity(rows.len());
    for d in rows {
        match out.last_mut() {
            Some(last) if (last.line(), last.side()) == (d.line(), d.side()) => {
                let line = d.line();
                if (last.row.record, last.row.point, last.arrears_since) != (d.row.record, d.row.point, d.arrears_since)
                {
                    violation!(clause = "REP.8", "rows of other records, points or arrears merged", line = line.get());
                }
                let Some(count) = last.row.count.checked_add(d.row.count) else {
                    capacity_exceeded!("members of a row", u32::MAX, last.row.count);
                };
                last.row.count = count;
                last.optional = Optional {
                    balance: sum(last.optional.balance, d.optional.balance, line),
                    pending: sum(last.optional.pending, d.optional.pending, line),
                    amount: sum(last.optional.amount, d.optional.amount, line),
                };
            }
            _ => out.push(d),
        }
    }
    out
}

impl DetachedRow {
    /// A row as the opening places it on a cell: its members, their role within the cell and its words, with a clean
    /// record, for data that stands in for an opening's.
    #[must_use]
    pub fn opening(line: LineId, side: Side, within: u8, count: u32, optional: Optional) -> DetachedRow {
        let row = RelRow { line, count, record: 0, point: 0, role: rows::role(side, within), flags: optional.flags() };
        DetachedRow { row, optional, arrears_since: Missing::Absent }
    }

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
        self.detach_view(arenas, table, holder, view, share, rounding)
    }

    /// Members leave several of a cell's rows at once, each row read once: the rows are taken last first, so a row
    /// that leaves whole moves none of those still to come.
    #[clause("REP.8", "REP.9", "REP.14")]
    pub fn detach_rows(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        plan: &[((LineId, Side), RowShare)],
        rounding: Round,
    ) -> Vec<DetachedRow> {
        let views = rows::rows(arenas, holder);
        let mut found: Vec<(usize, RowView, RowShare)> = Vec::with_capacity(plan.len());
        for (i, ((line, side), share)) in plan.iter().enumerate() {
            let Some(view) = views.iter().find(|r| r.row.line == *line && r.side() == *side) else {
                violation!(clause = "REG.14", "a row left that its holder does not have", line = line.get());
            };
            found.push((i, *view, *share));
        }
        found.sort_unstable_by_key(|(_, view, _)| core::cmp::Reverse(view.at));
        let mut out: Vec<(usize, DetachedRow)> = found
            .into_iter()
            .map(|(i, view, share)| (i, self.detach_view(arenas, table, holder, view, share, rounding)))
            .collect();
        out.sort_unstable_by_key(|(i, _)| *i);
        out.into_iter().map(|(_, d)| d).collect()
    }

    /// Members leave a cell's rows for several parts in turn, as `detach_rows` for each plan in order would leave them,
    /// with the rows read once and each written once: every share is taken from the row as the plans before it left
    /// it, the rows staying are rewritten at the end and the rows left whole are taken out last first.
    #[clause("REP.8", "REP.9", "REP.14")]
    pub fn detach_rows_batch(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        plans: &[RowPlan<'_>],
    ) -> Vec<Vec<DetachedRow>> {
        let party = arenas.party(holder);
        // Each row as the plans so far left it, whether its words changed, and whether it left whole.
        let mut views: Vec<(RowView, bool, bool)> =
            rows::rows(arenas, holder).into_iter().map(|v| (v, false, false)).collect();
        let mut out = Vec::with_capacity(plans.len());
        for (plan, rounding) in plans {
            let mut detached = Vec::with_capacity(plan.len());
            for ((line, side), share) in *plan {
                let Some((view, changed, gone)) =
                    views.iter_mut().find(|(v, _, gone)| !*gone && v.row.line == *line && v.side() == *side)
                else {
                    violation!(clause = "REG.14", "a row left that its holder does not have", line = line.get());
                };
                let of = view.row.count;
                if share.count == 0 || share.count > of {
                    violation!(
                        clause = "REP.9",
                        "members leaving a row that does not hold them",
                        leaving = share.count,
                        of = of
                    );
                }
                let since = match self.arrears.since(ArrearsKey::new(*line, *side, party)) {
                    Some(d) => Missing::Present(d),
                    None => Missing::Absent,
                };
                if share.count == of {
                    *gone = true;
                    detached.push(DetachedRow { row: view.row, optional: view.optional, arrears_since: since });
                    continue;
                }
                let (bl, bs) = word_share(view.optional.balance, share.own_balance, *share, of, *rounding);
                let (pl, ps) = word_share(view.optional.pending, 0, *share, of, *rounding);
                let (al, a_s) = word_share(view.optional.amount, 0, *share, of, *rounding);
                let mut leaving = view.row;
                leaving.count = share.count;
                view.row.count = of - share.count;
                view.optional = Optional { balance: bs, pending: ps, amount: a_s };
                *changed = true;
                let optional = Optional { balance: bl, pending: pl, amount: al };
                detached.push(DetachedRow { row: leaving, optional, arrears_since: since });
            }
            out.push(detached);
        }
        for (view, _, _) in views.iter().filter(|(_, changed, gone)| *changed && !*gone) {
            rows::rewrite(arenas, holder, view, view.optional);
        }
        let mut gone: Vec<RowView> = views.into_iter().filter(|(_, _, gone)| *gone).map(|(v, _, _)| v).collect();
        gone.sort_unstable_by_key(|v| core::cmp::Reverse(v.at));
        for v in gone {
            let (line, side) = (v.row.line, v.side());
            let _ = self.lines.unplace_row(arenas, table, holder, line, side);
            self.arrears.remove(ArrearsKey::new(line, side, party));
        }
        out
    }

    fn detach_view(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        view: RowView,
        share: RowShare,
        rounding: Round,
    ) -> DetachedRow {
        let (line, side) = (view.row.line, view.side());
        let of = view.row.count;
        if share.count == 0 || share.count > of {
            violation!(
                clause = "REP.9",
                "members leaving a row that does not hold them",
                leaving = share.count,
                of = of
            );
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
    /// line's side counts do not move. Returns whether the cell entered the line's list.
    #[clause("REP.8", "REP.14")]
    pub fn attach_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        detached: DetachedRow,
    ) -> bool {
        let (line, side) = (detached.line(), detached.side());
        let found = rows::iter(arenas, holder).find(|r| r.row.line == line && r.side() == side);
        self.attach_view(arenas, table, holder, found, detached)
    }

    /// Several of a part's rows join a cell, its rows read once: rows it holds take their counts and words in place,
    /// then the rows new to it are placed. Returns how many lines' lists it entered.
    #[clause("REP.8", "REP.14")]
    pub fn attach_rows(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        detached: Vec<DetachedRow>,
    ) -> u32 {
        let views = rows::rows(arenas, holder);
        let mut new = Vec::new();
        for d in detached {
            match views.iter().find(|r| r.row.line == d.line() && r.side() == d.side()) {
                // A row held takes its members in place, its width unchanged, so the views stay where they were.
                Some(view) => {
                    let _ = self.attach_view(arenas, table, holder, Some(*view), d);
                }
                None => new.push(d),
            }
        }
        let mut entered = 0;
        for d in new {
            if self.attach_view(arenas, table, holder, None, d) {
                entered += 1;
            }
        }
        entered
    }

    fn attach_view(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        found: Option<RowView>,
        detached: DetachedRow,
    ) -> bool {
        let (line, side) = (detached.line(), detached.side());
        let key = ArrearsKey::new(line, side, arenas.party(holder));
        let Some(mut view) = found else {
            let entered = self.lines.place_row(arenas, table, holder, detached.row, detached.optional);
            if let Missing::Present(d) = detached.arrears_since {
                self.arrears.begin(key, d);
            }
            return entered;
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
        false
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
    /// instrument, the cell entering the instrument's list. Returns whether it entered the list.
    #[clause("REP.8", "REP.14")]
    pub fn attach_holding(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        part: CellHolding,
    ) -> bool {
        let all = cell_holdings(arenas, holder);
        let Some((i, h)) = all.iter().copied().enumerate().find(|(_, h)| h.instrument == part.instrument) else {
            arenas.append(holder, ListKind::Holdings, &to_words(&part));
            self.instruments.enlist(table, holder, part.instrument);
            return true;
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
        false
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
        l.lines.add_row(
            &mut cells,
            1,
            a,
            line,
            NewRow { side: Side::Liability, within: 0, count: 10, point: 0, optional },
        );
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
        let mut part: DetachedRow = l.detach_row(
            &mut cells,
            1,
            a,
            (line, Side::Liability),
            RowShare { count: 1, own_balance: 0 },
            Round::Floor,
        );
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

    #[test]
    fn merge_rows_adds_by_line_side_and_keeps_records_apart() {
        use phx_id::LineId;

        use crate::rows::Optional;

        let row = |line: u32, side, count, balance: i64| {
            let optional = Optional { balance: Missing::Present(balance), ..Optional::NONE };
            DetachedRow::opening(LineId::new(line), side, 0, count, optional)
        };
        let merged = super::merge_rows(vec![
            row(4, Side::Liability, 3, 300),
            row(2, Side::Asset, 5, 50),
            row(4, Side::Liability, 7, 700),
            row(4, Side::Asset, 1, 10),
        ]);
        let got: Vec<(u32, Side, u32, Missing<i64>)> =
            merged.iter().map(|d| (d.line().get(), d.side(), d.row.count, d.optional.balance)).collect();
        assert_eq!(
            got,
            [
                (2, Side::Asset, 5, Missing::Present(50)),
                (4, Side::Asset, 1, Missing::Present(10)),
                (4, Side::Liability, 10, Missing::Present(1_000)),
            ]
        );
        let mut late = row(4, Side::Liability, 2, 20);
        late.arrears_since = Missing::Present(phx_id::Day::new(9));
        let refused = std::panic::catch_unwind(|| super::merge_rows(vec![row(4, Side::Liability, 3, 300), late]));
        assert!(refused.is_err(), "rows in arrears since other days never merge");
    }
}
