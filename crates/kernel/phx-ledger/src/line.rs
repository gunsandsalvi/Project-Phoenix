use core::marker::PhantomData;
use std::collections::BTreeMap;

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
    /// On a population's cells, the roles whose persons hold the side's rows, each person a member; none, the
    /// households.
    pub holder_roles: &'static [&'static str],
    /// Whether a holder holds at most one row of the kind on this side, as a person holds one job.
    pub exclusive: bool,
    /// Whether a holder's row counts a member for each of its counterparts, as an employer a job for each employee,
    /// so a cell's member holds any number; otherwise each member holds one, or one for each of its persons in
    /// `holder_roles`.
    pub many: bool,
}

/// A line kind as a system declares it: its two sides, and the systems that may request a transfer of its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineKindDecl {
    pub name: &'static str,
    pub asset: SideDecl,
    pub liability: SideDecl,
    pub transfer_requesters: &'static [&'static str],
    /// Whether its lines have dues on dates, so its rows sit in their holders' due-day runs.
    pub dated: bool,
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

fn usize_of(n: u32) -> usize {
    let Ok(u) = usize::try_from(n) else {
        capacity_exceeded!("words of a holder's run", usize::MAX, n);
    };
    u
}

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a holder's run", u32::MAX, n);
    };
    w
}

/// A line's flag: its schedule's dates are spent.
const DONE: u16 = 1;

/// A line's stored row, 36 bytes: its kind and flags, its interned terms, its side counts kept as rows change, the
/// next day any of its dues can fall and the index of its schedule's date that last fell due, so a due never searches
/// the schedule for its day, and its holder list.
#[clause("REP.3", "REG.14")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct LineRow {
    kind: u16,
    flags: u16,
    terms: u32,
    side_counts: [u32; 2],
    next_due: Day,
    fallen: u32,
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
    /// The lines by the day they next fall due; an entry whose line has since moved is passed over.
    wheel: BTreeMap<u32, Vec<u32>>,
    /// How many times each line side's members have changed, so what was read of a side is known still true; kept
    /// for the run alone, a load beginning it afresh with every read of a side.
    versions: BTreeMap<(u32, bool), u64>,
    /// The day of the latest 1b and the lines that fell due on it, so a row placed later that day on one of them
    /// falls due today; kept for the day alone, as the marks are made again each day.
    fell: (Day, std::collections::BTreeSet<u32>),
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
            wheel: BTreeMap::new(),
            versions: BTreeMap::new(),
            fell: (Day::new(0), std::collections::BTreeSet::new()),
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

    /// A line kind declared by name, as a contribution after the declarations reads it.
    #[must_use]
    pub fn kind_index(&self, name: &str) -> u16 {
        let Some(i) = self.kinds.iter().position(|k| k.name == name) else {
            violation!(clause = "REG.8", "a line kind read by a name never declared");
        };
        let Ok(index) = u16::try_from(i) else {
            capacity_exceeded!("line kinds", u16::MAX, i);
        };
        index
    }

    /// Every declared line kind's name, in the order declared.
    pub fn kind_names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.kinds.iter().map(|k| k.name)
    }

    /// A side of a line's kind, as its kind declares it.
    #[must_use]
    pub fn side_decl(&self, line: LineId, side: Side) -> &SideDecl {
        self.kind(self.row(line).kind).side(side)
    }

    /// A line's kind as its system declared it.
    #[must_use]
    pub fn decl(&self, line: LineId) -> &LineKindDecl {
        self.kind(self.row(line).kind)
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

    /// A line opened with its kind and terms, whose holder of the interned terms it becomes; no rows yet. `first` is
    /// the first day its dues fall and that date's index in its schedule; a line with no dates never falls due.
    pub fn open(&mut self, kind: u16, terms: TermsId, first: Missing<(Day, u32)>) -> LineId {
        let _ = self.kind(kind);
        let Ok(id) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("lines", u32::MAX, self.rows.len());
        };
        let (next_due, fallen, flags) = match first {
            Missing::Present((day, k)) => {
                let Some(before) = k.checked_sub(1) else {
                    violation!(
                        clause = "REG.5",
                        "a line's first due on its schedule's anchor, which is no date",
                        line = id
                    );
                };
                self.wheel.entry(day.get()).or_default().push(id);
                (day, before, 0)
            }
            Missing::Absent => (Day::new(0), 0, DONE),
        };
        self.rows.push(LineRow {
            kind,
            flags,
            terms: terms.get(),
            side_counts: [0; 2],
            next_due,
            fallen,
            holders: BlockList::EMPTY,
        });
        LineId::new(id)
    }

    /// The lines that fall due on a day, by identity; entries of earlier days, which no line can still owe, and of
    /// lines that have moved on are passed over.
    pub(crate) fn falling(&mut self, day: Day) -> Vec<LineId> {
        self.fell = (day, std::collections::BTreeSet::new());
        let mut out = Vec::new();
        while let Some(entry) = self.wheel.first_entry() {
            if *entry.key() > day.get() {
                break;
            }
            for id in entry.remove() {
                let row = self.row(LineId::new(id));
                if row.flags & DONE == 0 && row.next_due == day {
                    out.push(LineId::new(id));
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// The index of the schedule's date a line last fell due on.
    #[must_use]
    pub fn fallen(&self, line: LineId) -> u32 {
        self.row(line).fallen
    }

    /// A line fallen due on its `k`-th date, moved to its next: the next date's day, or none when its dates are spent.
    pub(crate) fn fall(&mut self, line: LineId, k: u32, next: Missing<Day>) {
        self.fell.1.insert(line.get());
        let mut row = self.row(line);
        if k != row.fallen + 1 {
            violation!(clause = "REG.5", "a line falling due out of its dates' order", line = line.get(), k = k);
        }
        row.fallen = k;
        match next {
            Missing::Present(d) => {
                if d <= row.next_due {
                    violation!(clause = "REG.5", "a line's next due day not after its last", line = line.get());
                }
                row.next_due = d;
                self.wheel.entry(d.get()).or_default().push(line.get());
            }
            Missing::Absent => row.flags |= DONE,
        }
        self.set(line, row);
    }

    /// Whether a line's kind has dues on dates.
    #[must_use]
    pub fn dated(&self, line: LineId) -> bool {
        self.kind(self.row(line).kind).dated
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

    pub(crate) fn adjust(&mut self, line: LineId, side: Side, by: i64) {
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
        self.moved(line, side);
    }

    /// A line side's members changed hands or counts, its total the same or not: what was read of it is stale.
    pub(crate) fn moved(&mut self, line: LineId, side: Side) {
        let changed = self.versions.entry((line.get(), side == Side::Asset)).or_insert(0);
        let Some(next) = changed.checked_add(1) else {
            capacity_exceeded!("changes of a line side", u64::MAX, *changed);
        };
        *changed = next;
    }

    /// How many times a line side's members have changed in this run; what was read of it at the same version is
    /// still true.
    #[must_use]
    pub fn side_version(&self, line: LineId, side: Side) -> u64 {
        self.versions.get(&(line.get(), side == Side::Asset)).copied().unwrap_or(0)
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
        let row = RelRow { line, count, record: 0, point, role: role(side, within), flags: 0 };
        self.place_row(arenas, table, holder, row, optional);
        self.adjust(line, side, i64::from(count));
    }

    /// A row put into its holder's rows as it is, into the due-day run if its line is dated, the holder entering the
    /// line's list with its first listed row there; the line's side counts are the caller's. Returns whether the holder
    /// entered the list.
    pub(crate) fn place_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        row: RelRow,
        optional: Optional,
    ) -> bool {
        let (line, side) = (row.line, rows::side_of(row.role));
        let decl = *self.kind(self.row(line).kind).side(side);
        let kind = arenas.kind();
        if !decl.holder_kinds.contains(&kind) {
            violation!(clause = "REG.8", "a row of a holder kind its line side does not declare", line = line.get());
        }
        if optional.flags() != decl.words {
            violation!(clause = "REP.3", "a row's optional words other than its line side declares", line = line.get());
        }
        // One pass over the holder's rows, which a firm holds on every wage and rent line: the check and the list.
        let mut listed_before = false;
        for r in rows::iter(arenas, holder).filter(|r| r.row.line == line) {
            if r.side() == side {
                violation!(clause = "REG.14", "a second row of one holder on one side of a line", line = line.get());
            }
            listed_before |= self.kind(self.row(line).kind).side(r.side()).holder_list;
        }
        if self.dated(line) {
            let mut head = arenas.run_head(holder);
            let at = usize_of(head.offset + head.len);
            let added = rows::insert(arenas, holder, at, row, optional);
            // A line that fell due today has moved on to its next date, but the row falls due today with it.
            let due = if self.fell.1.contains(&line.get()) { self.fell.0 } else { self.next_due(line) }.get();
            if head.len == 0 || due < head.next_due {
                head.next_due = due;
            }
            head.len += word32(added);
            arenas.set_run_head(holder, head);
        } else {
            rows::append(arenas, holder, row, optional);
        }
        self.moved(line, side);
        let enters = decl.holder_list && !listed_before;
        if enters {
            let mut r = self.row(line);
            self.lists.enter(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
        enters
    }

    pub(crate) fn find(arenas: &dyn HolderArenas, holder: Slot, line: LineId, side: Side) -> RowView {
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
        let view = self.unplace_row(arenas, table, holder, line, side);
        self.adjust(line, side, -i64::from(view.row.count));
    }

    /// A holder's row taken out of its rows and its due-day run, the holder leaving the line's list with its last
    /// listed row there; the line's side counts are the caller's. Returns the row as it was.
    pub(crate) fn unplace_row(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        line: LineId,
        side: Side,
    ) -> RowView {
        // One pass finds the row and whether the holder is listed on its line by another row there too.
        let listing = |s: Side| self.kind(self.row(line).kind).side(s).holder_list;
        let mut found = None;
        let mut listed_by_other = false;
        for r in rows::iter(arenas, holder).filter(|r| r.row.line == line) {
            if r.side() == side {
                found = Some(r);
            } else {
                listed_by_other |= listing(r.side());
            }
        }
        let Some(view) = found else {
            violation!(clause = "REG.14", "a row read that its holder does not have", line = line.get());
        };
        let leaves = listing(side) && !listed_by_other;
        rows::remove(arenas, holder, &view);
        self.moved(line, side);
        let mut head = arenas.run_head(holder);
        let (at, width) = (word32(view.at), word32(rows::words_of_row(&view)));
        if at < head.offset {
            head.offset -= width;
        } else if at < head.offset + head.len {
            head.len -= width;
        }
        arenas.set_run_head(holder, head);
        if leaves {
            let mut r = self.row(line);
            self.lists.leave(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
        view
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
            h.u64(u64::from(r.fallen));
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

/// A line kind's declaration and its money roles, which a load takes from the build's own declarations.
#[derive(Debug, Default)]
pub struct LineDecls {
    kinds: Vec<LineKindDecl>,
    deposits: Vec<u16>,
    reserves: Vec<u16>,
    money: Vec<u16>,
}

impl<B: Backing> Lines<B> {
    /// Another's declarations, copied onto lines that have none.
    pub(crate) fn copy_decls<C: Backing>(&mut self, from: &Lines<C>) {
        self.kinds.clone_from(&from.kinds);
        self.deposits.clone_from(&from.deposits);
        self.reserves.clone_from(&from.reserves);
        self.money.clone_from(&from.money);
    }

    /// The declarations made so far, taken for the lines a load reads back.
    pub(crate) fn take_decls(&mut self) -> LineDecls {
        LineDecls {
            kinds: core::mem::take(&mut self.kinds),
            deposits: core::mem::take(&mut self.deposits),
            reserves: core::mem::take(&mut self.reserves),
            money: core::mem::take(&mut self.money),
        }
    }

    /// The lines for a save: their kinds' names, which a load checks against the build's, their rows and the room
    /// their holder lists had; the holder lists and the due wheel are indexes and are rebuilt.
    pub(crate) fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        self.kind_names().collect::<Vec<_>>().save(w);
        self.rows.save(w);
        self.lists.blocks().save(w);
    }

    /// The lines read back over the build's declarations, with their holder lists empty for the books to rebuild and
    /// their due wheel rebuilt from each line's next due day.
    pub(crate) fn load_from(
        r: &mut phx_store::Reader<'_>,
        decls: LineDecls,
        keys: HolderKeys,
    ) -> Result<Lines<B>, phx_store::LoadError> {
        use phx_store::Saved as _;
        let names: Vec<&'static str> = phx_store::Saved::load(r)?;
        if names.len() != decls.kinds.len() || names.iter().zip(&decls.kinds).any(|(n, k)| *n != k.name) {
            return Err(phx_store::LoadError::Invalid("line kinds other than the build's".to_owned()));
        }
        let mut rows: Column<LineRow, B> = Column::load(r)?;
        let blocks = u32::load(r)?;
        let mut wheel: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (id, row) in rows.slice_mut().iter_mut().enumerate() {
            if usize::from(row.kind) >= decls.kinds.len() {
                return Err(phx_store::LoadError::Invalid("a line of a kind never declared".to_owned()));
            }
            row.holders = BlockList::EMPTY;
            if row.flags & DONE == 0 {
                wheel.entry(row.next_due.get()).or_default().push(phx_store::narrow(id, "a line's identity")?);
            }
        }
        let LineDecls { kinds, deposits, reserves, money } = decls;
        Ok(Lines {
            rows,
            lists: HolderLists::new(r.space(), blocks, keys),
            kinds,
            deposits,
            reserves,
            money,
            wheel,
            versions: BTreeMap::new(),
            fell: (Day::new(0), std::collections::BTreeSet::new()),
        })
    }

    /// A holder taken off a line's holder list as renumbering moves it, where a side it holds keeps one.
    pub(crate) fn delist(&mut self, table: u16, holder: Slot, line: LineId, sides: &[Side]) {
        let kind = self.kind(self.row(line).kind);
        if sides.iter().any(|s| kind.side(*s).holder_list) {
            let mut r = self.row(line);
            self.lists.leave(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
    }

    /// A holder put back on a line's holder list as a load rebuilds it, where a side it holds keeps one.
    pub(crate) fn relist(&mut self, table: u16, holder: Slot, line: LineId, sides: &[Side]) {
        let kind = self.kind(self.row(line).kind);
        if sides.iter().any(|s| kind.side(*s).holder_list) {
            let mut r = self.row(line);
            self.lists.enter(line.get(), &mut r.holders, table, holder);
            self.set(line, r);
        }
    }
}
