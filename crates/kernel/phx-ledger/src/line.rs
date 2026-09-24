use core::marker::PhantomData;

use phx_id::{Day, LineId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Ccy, Missing, Money, Qty, UnitId, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, BlockList, Column, SystemBacking};

use crate::algebra::Side;
use crate::holder::{HolderArenas, HolderKeys, HolderLists};
use crate::rows::{self, Optional, PaymentRecord, RelRow, RowView, role};
use crate::terms::TermsId;

/// One side of a line kind: the kinds of party that may hold it, the optional words its rows carry, and whether the
/// line keeps its holders in a list — a many-party retail side reached only on its dues keeps none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideDecl {
    pub holder_kinds: &'static [&'static str],
    pub words: u8,
    pub holder_list: bool,
}

/// A line kind as a system declares it: its two sides, and the systems that may request a transfer of its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineKindDecl {
    pub name: &'static str,
    pub asset: SideDecl,
    pub liability: SideDecl,
    pub transfer_requesters: &'static [&'static str],
}

impl LineKindDecl {
    #[must_use]
    pub fn side(&self, side: Side) -> &SideDecl {
        match side {
            Side::Asset => &self.asset,
            Side::Liability => &self.liability,
        }
    }
}

/// Line kinds assembly refuses: any no system may request a transfer of.
#[must_use]
pub fn refusals(kinds: &[LineKindDecl]) -> Vec<String> {
    kinds
        .iter()
        .filter(|k| k.transfer_requesters.is_empty())
        .map(|k| format!("line kind `{}` names no system that may request a transfer of its rows", k.name))
        .collect()
}

/// What a line kind's balance word is in: money, or a declared unit that is not money.
pub trait BalanceUnit {
    type Value;
    fn value(raw: i64, ccy: Ccy, unit: UnitId) -> Self::Value;
}

/// A balance in money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InMoney;

impl BalanceUnit for InMoney {
    type Value = Money;

    fn value(raw: i64, ccy: Ccy, _: UnitId) -> Money {
        Money::new(raw, ccy)
    }
}

/// A balance in a declared unit that is not money, such as a pension accrued per year: it becomes money only
/// through its line kind's declared conversions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InUnit;

impl BalanceUnit for InUnit {
    type Value = Qty;

    fn value(raw: i64, _: Ccy, unit: UnitId) -> Qty {
        Qty::new(raw, unit)
    }
}

/// A declared line kind, typed by its balance's unit, so a balance that is not money cannot be read as money.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineKind<U> {
    index: u16,
    unit: PhantomData<U>,
    balance_unit: UnitId,
}

impl<U: BalanceUnit> LineKind<U> {
    #[must_use]
    pub fn index(&self) -> u16 {
        self.index
    }

    /// A row's balance, in the kind's unit; a row without a balance word has none.
    pub fn balance(&self, row: &RowView, ccy: Ccy) -> Missing<U::Value> {
        match row.optional.balance {
            Missing::Present(raw) => Missing::Present(U::value(raw, ccy, self.balance_unit)),
            Missing::Absent => Missing::Absent,
        }
    }
}

/// A line's flag: its schedule's dates are spent.
const DONE: u16 = 1;

/// A line's stored row, 32 bytes: its kind and flags, its interned terms, its side counts kept as rows change, the
/// next day any of its dues can fall, and its holder list.
#[clause("REP.3", "REG.14")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct LineRow {
    kind: u16,
    flags: u16,
    terms: u32,
    side_counts: [u32; 2],
    next_due: Day,
    holders: BlockList,
}

/// A row to open on a line: its side and role within the party, its member count, its price point and its optional
/// words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewRow {
    pub side: Side,
    pub within: u8,
    pub count: u32,
    pub point: u16,
    pub optional: Optional,
}

/// Every line: one record of identical contracts each, its sides read from its holders' rows, and the holders
/// of its listed sides kept in its holder list.
#[clause("REP.3", "REG.8", "REG.14")]
#[derive(Debug)]
pub struct Lines<B: Backing = SystemBacking> {
    rows: Column<LineRow, B>,
    lists: HolderLists<B>,
    kinds: Vec<LineKindDecl>,
    deposits: Vec<u16>,
    reserves: Vec<u16>,
    money: Vec<u16>,
}

impl<B: Backing> Lines<B> {
    /// Room for at most `max` lines chunked by `per_chunk`, and `blocks` holder-list blocks in each pool.
    pub fn new(space: &mut AddressSpace, max: u32, per_chunk: u32, blocks: u32, keys: HolderKeys) -> Lines<B> {
        Lines {
            rows: Column::new(space, max, per_chunk),
            lists: HolderLists::new(space, blocks, keys),
            kinds: Vec::new(),
            deposits: Vec::new(),
            reserves: Vec::new(),
            money: Vec::new(),
        }
    }

    fn declare_kind(&mut self, decl: LineKindDecl) -> u16 {
        let Ok(index) = u16::try_from(self.kinds.len()) else {
            capacity_exceeded!("line kinds", u16::MAX, self.kinds.len());
        };
        self.kinds.push(decl);
        index
    }

    /// A line kind whose balance is money.
    pub fn declare_money(&mut self, decl: LineKindDecl) -> LineKind<InMoney> {
        LineKind { index: self.declare_kind(decl), unit: PhantomData, balance_unit: UnitId::new(0) }
    }

    /// Money its holders pay and are paid from: a central bank's or a bank's liability that settles payments.
    pub fn declare_means_of_payment(&mut self, decl: LineKindDecl) -> LineKind<InMoney> {
        let kind = self.declare_money(decl);
        self.money.push(kind.index());
        kind
    }

    /// A deposit kind: money whose holders pay and are paid from it.
    pub fn declare_deposits(&mut self, decl: LineKindDecl) -> LineKind<InMoney> {
        let kind = self.declare_means_of_payment(decl);
        self.deposits.push(kind.index());
        kind
    }

    /// Reserves: the money banks pay each other with.
    pub fn declare_reserves(&mut self, decl: LineKindDecl) -> LineKind<InMoney> {
        let kind = self.declare_means_of_payment(decl);
        self.reserves.push(kind.index());
        kind
    }

    /// The name of a line's kind.
    #[must_use]
    pub fn kind_name(&self, line: LineId) -> &'static str {
        self.kind(self.row(line).kind).name
    }

    /// A line's kind.
    #[must_use]
    pub fn kind_of(&self, line: LineId) -> u16 {
        self.row(line).kind
    }

    /// Whether a line's kind is a deposit kind.
    #[must_use]
    pub fn is_deposit(&self, line: LineId) -> bool {
        self.deposits.contains(&self.kind_of(line))
    }

    /// Whether a line's kind is reserves.
    #[must_use]
    pub fn is_reserves(&self, line: LineId) -> bool {
        self.reserves.contains(&self.kind_of(line))
    }

    /// Whether a line's kind is a means of payment: deposits, reserves or another account a central bank or bank keeps.
    #[must_use]
    pub fn is_money(&self, line: LineId) -> bool {
        self.money.contains(&self.kind_of(line))
    }

    /// Every line, by identity.
    pub fn ids(&self) -> impl Iterator<Item = LineId> + use<B> {
        let Ok(n) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("lines", u32::MAX, self.rows.len());
        };
        (0..n).map(LineId::new)
    }

    /// Whether every date of a line's schedule has passed, so nothing more falls due on it.
    #[must_use]
    pub fn done(&self, line: LineId) -> bool {
        self.row(line).flags & DONE != 0
    }

    /// A line's next due day after it paid: the next date of its schedule, or none when its dates are spent.
    pub fn advance(&mut self, line: LineId, next: Missing<Day>) {
        let mut row = self.row(line);
        match next {
            Missing::Present(d) => {
                if d <= row.next_due {
                    violation!(clause = "REG.5", "a line's next due day not after its last", line = line.get());
                }
                row.next_due = d;
            }
            Missing::Absent => row.flags |= DONE,
        }
        self.set(line, row);
    }

    /// A line kind whose balance is in a declared unit that is not money.
    pub fn declare_in_unit(&mut self, decl: LineKindDecl, unit: UnitId) -> LineKind<InUnit> {
        LineKind { index: self.declare_kind(decl), unit: PhantomData, balance_unit: unit }
    }

    fn kind(&self, index: u16) -> &LineKindDecl {
        let Some(k) = self.kinds.get(usize::from(index)) else {
            violation!(clause = "REG.8", "a line of an undeclared kind", kind = index);
        };
        k
    }

    fn row(&self, line: LineId) -> LineRow {
        let Some(row) = self.rows.get(Slot::new(line.get())) else {
            violation!(clause = "REG.14", "a line that was never opened", line = line.get());
        };
        row
    }

    fn set(&mut self, line: LineId, row: LineRow) {
        self.rows.set(Slot::new(line.get()), row);
    }

    /// A line opened with its kind and terms, whose holder of the interned terms it becomes; no rows yet.
    pub fn open(&mut self, kind: u16, terms: TermsId, next_due: Day) -> LineId {
        let _ = self.kind(kind);
        let Ok(id) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("lines", u32::MAX, self.rows.len());
        };
        self.rows.push(LineRow {
            kind,
            flags: 0,
            terms: terms.get(),
            side_counts: [0; 2],
            next_due,
            holders: BlockList::EMPTY,
        });
        LineId::new(id)
    }

    /// A line's terms.
    pub fn terms(&self, line: LineId) -> TermsId {
        TermsId::new(self.row(line).terms)
    }

    /// The members on a side of a line.
    #[must_use]
    pub fn side_count(&self, line: LineId, side: Side) -> u32 {
        let [asset, liability] = self.row(line).side_counts;
        match side {
            Side::Asset => asset,
            Side::Liability => liability,
        }
    }

    /// Whether a side of a line keeps its holders in the line's holder list.
    #[must_use]
    pub fn listed_side(&self, line: LineId, side: Side) -> bool {
        self.kind(self.row(line).kind).side(side).holder_list
    }

    /// The next day any of a line's dues can fall.
    pub fn next_due(&self, line: LineId) -> Day {
        self.row(line).next_due
    }

    /// A line's holders, as their keys, in order.
    pub fn holders(&self, line: LineId) -> impl Iterator<Item = u32> + '_ {
        self.lists.iter(line.get(), self.row(line).holders)
    }

    fn adjust(&mut self, line: LineId, side: Side, by: i64) {
        let mut row = self.row(line);
        let [asset, liability] = &mut row.side_counts;
        let count = match side {
            Side::Asset => asset,
            Side::Liability => liability,
        };
        let Some(next) = i64::from(*count).checked_add(by).and_then(|n| u32::try_from(n).ok()) else {
            violation!(clause = "REG.14", "a line's side count below nothing or beyond its width", line = line.get());
        };
        *count = next;
        self.set(line, row);
    }

    /// A holder's row opened on a line. A holder of a kind the side does not declare, or a row with other optional
    /// words than the side declares, is refused, and a holder keeps at most one row on each side of a line.
    pub(crate) fn add_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        line: LineId,
        new: NewRow,
    ) {
        let NewRow { side, within, count, point, optional } = new;
        let decl = *self.kind(self.row(line).kind).side(side);
        let kind = arenas.kind();
        if !decl.holder_kinds.contains(&kind) {
            violation!(clause = "REG.8", "a row of a holder kind its line side does not declare", line = line.get());
        }
        if optional.flags() != decl.words {
            violation!(clause = "REP.3", "a row's optional words other than its line side declares", line = line.get());
        }
        let rows_now = rows::rows(arenas, holder);
        if rows_now.iter().any(|r| r.row.line == line && r.side() == side) {
            violation!(clause = "REG.14", "a second row of one holder on one side of a line", line = line.get());
        }
        let listed_before = self.listed(line, &rows_now);
        let row = RelRow { line, count, record: 0, point, role: role(side, within), flags: 0 };
        rows::append(arenas, holder, row, optional);
        self.adjust(line, side, i64::from(count));
        if decl.holder_list && !listed_before {
            let mut r = self.row(line);
            self.lists.enter(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
    }

    /// Whether any of a holder's rows puts it on a line's holder list: a row on a side that keeps one.
    fn listed(&self, line: LineId, rows: &[RowView]) -> bool {
        let kind = self.kind(self.row(line).kind);
        rows.iter().any(|r| r.row.line == line && kind.side(r.side()).holder_list)
    }

    fn find(arenas: &dyn HolderArenas, holder: Slot, line: LineId, side: Side) -> RowView {
        let Some(view) = rows::iter(arenas, holder).find(|r| r.row.line == line && r.side() == side) else {
            violation!(clause = "REG.14", "a row read that its holder does not have", line = line.get());
        };
        view
    }

    /// A row's member count changed, the line's side count with it.
    pub(crate) fn set_count(
        &mut self,
        arenas: &mut dyn HolderArenas,
        holder: Slot,
        line: LineId,
        side: Side,
        count: u32,
    ) {
        let mut view = Self::find(arenas, holder, line, side);
        let by = i64::from(count) - i64::from(view.row.count);
        view.row.count = count;
        let optional = view.optional;
        rows::rewrite(arenas, holder, &view, optional);
        self.adjust(line, side, by);
    }

    /// A row's optional words written, where the row lies.
    pub(crate) fn set_words(arenas: &mut dyn HolderArenas, holder: Slot, line: LineId, side: Side, optional: Optional) {
        let view = Self::find(arenas, holder, line, side);
        rows::rewrite(arenas, holder, &view, optional);
    }

    /// A row's payment record written: its days in arrears and its payments missed.
    pub(crate) fn set_record(
        arenas: &mut dyn HolderArenas,
        holder: Slot,
        line: LineId,
        side: Side,
        record: PaymentRecord,
    ) {
        let mut view = Self::find(arenas, holder, line, side);
        view.row.record = record.packed();
        let optional = view.optional;
        rows::rewrite(arenas, holder, &view, optional);
    }

    /// A holder's row on a side of a line removed; the holder leaves the line's list with its last row on it.
    pub(crate) fn remove_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        line: LineId,
        side: Side,
    ) {
        let view = Self::find(arenas, holder, line, side);
        let listed_before = self.listed(line, &rows::rows(arenas, holder));
        rows::remove(arenas, holder, &view);
        self.adjust(line, side, -i64::from(view.row.count));
        if listed_before && !self.listed(line, &rows::rows(arenas, holder)) {
            let mut r = self.row(line);
            self.lists.leave(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
    }

    /// Each line's kind, flags, terms, side counts and next due day, in identity order; its holder list is an index
    /// of its holders' rows and stays out.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        for r in self.rows.slice() {
            let [a, b] = r.side_counts;
            for w in [u64::from(r.kind), u64::from(r.flags), u64::from(r.terms), u64::from(a), u64::from(b)] {
                h.u64(w);
            }
            h.u64(u64::from(r.next_due.get()));
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// The holder keys' split, for reading a holder list's entries.
    pub fn keys(&self) -> HolderKeys {
        self.lists.keys()
    }
}
