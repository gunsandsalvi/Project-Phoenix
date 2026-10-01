//! A kind's store: each group a column of fixed-width rows by slot, at the slot the directory gave its party, its words
//! read and written in place through handles a layout compiled. A read or write first checks the reference against the
//! directory's generation for the slot, so a stale reference stops the run; an absent word reads `Missing`.

use std::marker::PhantomData;

use phx_id::{PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::index::Index;
use phx_store::index_decl::IndexDecl;
use phx_store::{AddressSpace, Backing, Column, StoreStats, SystemBacking};

use crate::consts::{KIND_GROUPS, KIND_INDEXES};
use crate::directory::Directory;
use crate::layout::{IntTy, Layout};

/// An integer a word holds, read and written little-endian, with the sentinel an absent one holds.
pub trait Word: Copy + PartialEq + std::fmt::Debug {
    const TY: IntTy;
    const ABSENT: Self;
    fn read(bytes: &[u8]) -> Option<Self>;
    fn write(self, bytes: &mut [u8]);
    /// The word as an index's key; a negative one is no key.
    fn key(self) -> Option<u64>;
}

macro_rules! word {
    ($($t:ty => $ty:ident, $absent:expr);* $(;)?) => {
        $(impl Word for $t {
            const TY: IntTy = IntTy::$ty;
            const ABSENT: $t = $absent;
            #[inline]
            fn read(bytes: &[u8]) -> Option<$t> {
                bytes.try_into().ok().map(<$t>::from_le_bytes)
            }
            #[inline]
            fn write(self, bytes: &mut [u8]) {
                bytes.copy_from_slice(&self.to_le_bytes());
            }
            fn key(self) -> Option<u64> {
                u64::try_from(self).ok()
            }
        })*
    };
}

word!(u8 => U8, u8::MAX; u16 => U16, u16::MAX; u32 => U32, u32::MAX; u64 => U64, u64::MAX; i32 => I32, i32::MIN;
    i64 => I64, i64::MIN);

/// A word's read handle: its group, its offset in the group's row, and whether it may be absent.
#[derive(Debug)]
pub struct Attr<T> {
    group: u8,
    offset: u16,
    absent: bool,
    ty: PhantomData<T>,
}

impl<T> Clone for Attr<T> {
    fn clone(&self) -> Attr<T> {
        *self
    }
}

impl<T> Copy for Attr<T> {}

impl<T> PartialEq for Attr<T> {
    fn eq(&self, o: &Attr<T>) -> bool {
        (self.group, self.offset) == (o.group, o.offset)
    }
}

impl<T: Word> Attr<T> {
    pub(crate) fn new(group: u8, offset: u16, absent: bool) -> Attr<T> {
        Attr { group, offset, absent, ty: PhantomData }
    }

    /// Where the word lies: its group and its offset in the group's row.
    #[must_use]
    pub fn place(self) -> (u8, u16) {
        (self.group, self.offset)
    }

    fn span(self) -> std::ops::Range<usize> {
        let at = usize::from(self.offset);
        at..at + usize::from(T::TY.bytes())
    }

    /// The word in a group's row, `Missing` where it holds its sentinel.
    #[inline]
    pub(crate) fn read_in(self, row: &[u8]) -> Missing<T> {
        let Some(v) = row.get(self.span()).and_then(T::read) else { past_row(self.offset) };
        if self.absent && v == T::ABSENT { Missing::Absent } else { Missing::Present(v) }
    }

    pub(crate) fn write_in(self, row: &mut [u8], v: Missing<T>) {
        let value = match v {
            Missing::Present(x) if x == T::ABSENT && self.absent => {
                violation!(clause = "NUM.8", "an attribute's sentinel written as its value", offset = self.offset)
            }
            Missing::Present(x) => x,
            Missing::Absent if self.absent => T::ABSENT,
            Missing::Absent => violation!(clause = "NUM.8", "absent written to a word that is never absent"),
        };
        match row.get_mut(self.span()) {
            Some(bytes) => value.write(bytes),
            None => violation!(clause = "REP.1", "a word past its group's row", offset = self.offset),
        }
    }
}

/// A word's write handle, handed once to the base that declared it.
#[derive(Debug)]
pub struct AttrW<T>(pub(crate) Attr<T>);

impl<T> Clone for AttrW<T> {
    fn clone(&self) -> AttrW<T> {
        *self
    }
}

impl<T> Copy for AttrW<T> {}

impl<T> AttrW<T> {
    /// The handle that reads what this one writes.
    #[must_use]
    pub fn read(self) -> Attr<T> {
        self.0
    }
}

/// A word an opening writes at `begin`, its value as the word's own type.
#[derive(Clone, Copy, Debug)]
pub struct Opening {
    group: u8,
    offset: u16,
    bytes: [u8; 8],
    width: u8,
}

impl Opening {
    #[must_use]
    pub fn of<T: Word>(a: AttrW<T>, v: T) -> Opening {
        let mut bytes = [0; 8];
        let span = 0..usize::from(T::TY.bytes());
        if let Some(b) = bytes.get_mut(span) {
            v.write(b);
        }
        if a.0.absent && v == T::ABSENT {
            violation!(clause = "NUM.8", "an attribute's sentinel written as its opening", offset = a.0.offset);
        }
        Opening { group: a.0.group, offset: a.0.offset, bytes, width: u8::try_from(T::TY.bytes()).unwrap_or(u8::MAX) }
    }
}

/// An index a kind keeps on one of its words: the word's group and offset, and the instance.
#[derive(Debug)]
pub(crate) struct Keyed<B: Backing> {
    group: u8,
    offset: u16,
    ty: IntTy,
    absent: bool,
    index: Index<B>,
}

impl<B: Backing> Keyed<B> {
    fn key_in(&self, row: &[u8]) -> Option<u64> {
        let at = usize::from(self.offset);
        let bytes = row.get(at..at + usize::from(self.ty.bytes()))?;
        macro_rules! key {
            ($t:ty) => {{
                let v = <$t>::read(bytes)?;
                (!(self.absent && v == <$t>::ABSENT)).then(|| v.key()).flatten()
            }};
        }
        match self.ty {
            IntTy::U8 => key!(u8),
            IntTy::U16 => key!(u16),
            IntTy::U32 => key!(u32),
            IntTy::U64 => key!(u64),
            IntTy::I32 => key!(i32),
            IntTy::I64 => key!(i64),
        }
    }
}

/// A kind's store: its kind's number, each group's width and blank row, each group's rows by slot, the slots ever
/// written, and the indexes kept on its words.
#[clause("PTY.5", "REP.1")]
#[derive(Debug)]
pub struct KindStore<B: Backing = SystemBacking> {
    pub(crate) kind: u8,
    pub(crate) widths: Vec<u16>,
    pub(crate) blanks: Vec<u8>,
    pub(crate) groups: Vec<Column<u8, B>>,
    pub(crate) rows: u32,
    pub(crate) keyed: Vec<Keyed<B>>,
}

fn at(slot: Slot, width: u16) -> usize {
    let (s, w) = (usize::try_from(slot.get()).unwrap_or(usize::MAX), usize::from(width));
    s.checked_mul(w).unwrap_or_else(|| capacity_exceeded!("a kind's row bytes", usize::MAX, s))
}

impl<B: Backing> KindStore<B> {
    /// An empty store of a kind's layout, room for `capacity` parties, each group's column reserved at capacity.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, layout: &Layout, capacity: u32) -> KindStore<B> {
        let groups = layout
            .widths
            .iter()
            .map(|w| {
                let Some(bytes) = capacity.checked_mul(u32::from(*w)) else {
                    capacity_exceeded!("a kind's group bytes", u32::MAX, u64::from(capacity) * u64::from(*w));
                };
                Column::new(space, bytes, crate::consts::KIND_CHUNK_BYTES)
            })
            .collect();
        KindStore {
            kind,
            widths: layout.widths.clone(),
            blanks: layout.blanks().concat(),
            groups,
            rows: 0,
            keyed: Vec::new(),
        }
    }

    /// An index kept on a word from now on: every live slot `live` names placed at its key, in slot order, as a load
    /// rebuilds it.
    ///
    /// # Errors
    /// An instance its declaration refuses.
    #[phx_macros::opening]
    pub fn keep_index<T: Word>(
        &mut self,
        space: &mut AddressSpace,
        a: Attr<T>,
        decl: IndexDecl,
        live: impl Iterator<Item = Slot>,
    ) -> Result<usize, String> {
        if self.keyed.len() >= KIND_INDEXES {
            return Err(format!("a kind keeps at most {KIND_INDEXES} indexes on its words"));
        }
        let mut k =
            Keyed { group: a.group, offset: a.offset, ty: T::TY, absent: a.absent, index: Index::new(space, decl)? };
        for slot in live {
            if let Some(key) = self.row(a.group, slot).and_then(|r| k.key_in(r)) {
                k.index.insert(key, slot.get());
            }
        }
        k.index.settle();
        self.keyed.push(k);
        Ok(self.keyed.len() - 1)
    }

    /// The `i`th index kept.
    #[must_use]
    pub fn index(&self, i: usize) -> Option<&Index<B>> {
        self.keyed.get(i).map(|k| &k.index)
    }

    fn row(&self, group: u8, slot: Slot) -> Option<&[u8]> {
        let g = usize::from(group);
        let w = *self.widths.get(g)?;
        let start = at(slot, w);
        self.groups.get(g)?.slice().get(start..start + usize::from(w))
    }

    fn row_mut(&mut self, group: u8, slot: Slot) -> &mut [u8] {
        let g = usize::from(group);
        let (Some(w), Some(col)) = (self.widths.get(g).copied(), self.groups.get_mut(g)) else {
            violation!(clause = "REP.1", "a group the kind does not keep", group = group);
        };
        let start = at(slot, w);
        match col.slice_mut().get_mut(start..start + usize::from(w)) {
            Some(r) => r,
            None => violation!(clause = "REP.1", "a row past the slots begun", slot = slot.get()),
        }
    }

    /// The slot a reference names, which the directory's generation for it must match: a stale one stops the run.
    #[inline]
    fn check<D: Backing>(&self, dir: &Directory<D>, r: PartyRef) -> Slot {
        if r.kind() != self.kind || dir.reference(self.kind, r.slot()) != Some(r) || r.slot().get() >= self.rows {
            violation!(clause = "PTY.10", "a stale or foreign reference read through a kind's store", party = r.word());
        }
        r.slot()
    }

    /// A party the directory began, every group's row written in full: its blank, then the opening's words.
    #[clause("PTY.9", "REP.1")]
    pub fn begin<D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, opening: &[Opening]) {
        if r.kind() != self.kind || dir.reference(self.kind, r.slot()) != Some(r) {
            violation!(clause = "PTY.1", "a party begun that the directory did not hand out", party = r.word());
        }
        let slot = r.slot();
        match slot.get().cmp(&self.rows) {
            std::cmp::Ordering::Equal => {
                let mut blanks = self.blanks.as_slice();
                for (col, w) in self.groups.iter_mut().zip(&self.widths) {
                    let (blank, rest) = blanks.split_at(usize::from(*w));
                    col.extend(blank);
                    blanks = rest;
                }
                self.rows += 1;
            }
            std::cmp::Ordering::Less => {
                let mut blanks = self.blanks.as_slice();
                for (col, w) in self.groups.iter_mut().zip(&self.widths) {
                    let (blank, rest) = blanks.split_at(usize::from(*w));
                    blanks = rest;
                    let start = at(slot, *w);
                    match col.slice_mut().get_mut(start..start + usize::from(*w)) {
                        Some(row) => row.copy_from_slice(blank),
                        None => violation!(clause = "REP.1", "a row past the slots begun", slot = slot.get()),
                    }
                }
            }
            std::cmp::Ordering::Greater => {
                violation!(clause = "SET.12", "a party begun past the next slot", slot = slot.get(), rows = self.rows);
            }
        }
        for o in opening {
            let at = usize::from(o.offset);
            let w = usize::from(o.width);
            let row = self.row_mut(o.group, slot);
            match (row.get_mut(at..at + w), o.bytes.get(..w)) {
                (Some(dst), Some(src)) => dst.copy_from_slice(src),
                _ => violation!(clause = "REP.1", "an opening word past its group's row", offset = o.offset),
            }
        }
        for i in 0..self.keyed.len() {
            let key = self.keyed.get(i).and_then(|k| self.row(k.group, slot).and_then(|r| k.key_in(r)));
            if let (Some(key), Some(k)) = (key, self.keyed.get_mut(i)) {
                k.index.insert(key, slot.get());
            }
        }
    }

    /// A party ended: its row stays until its slot's next `begin`; it leaves every index it stood at.
    pub fn end<D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef) {
        let slot = self.check(dir, r);
        for i in 0..self.keyed.len() {
            let key = self.keyed.get(i).and_then(|k| self.row(k.group, slot).and_then(|r| k.key_in(r)));
            if let (Some(key), Some(k)) = (key, self.keyed.get_mut(i)) {
                k.index.left(key);
            }
        }
    }

    /// A word of a party.
    #[inline]
    pub fn get<T: Word, D: Backing>(&self, dir: &Directory<D>, r: PartyRef, a: Attr<T>) -> Missing<T> {
        let slot = self.check(dir, r);
        match self.row(a.group, slot) {
            Some(row) => a.read_in(row),
            None => violation!(clause = "REP.1", "a row past the slots begun", slot = slot.get()),
        }
    }

    /// A party's row of one group, gathered once for the reads that follow.
    #[inline]
    #[must_use]
    pub fn gather<D: Backing>(&self, dir: &Directory<D>, r: PartyRef, group: u8) -> Row<'_> {
        let slot = self.check(dir, r);
        match self.row(group, slot) {
            Some(bytes) => Row { group, bytes },
            None => violation!(clause = "REP.1", "a group the kind does not keep", group = group),
        }
    }

    /// The row of one group at a slot, as the day names its parties by their place: the slot's party now, live or
    /// ended this day, its slot not handed out again before the day closes. None past the slots begun.
    #[inline]
    #[must_use]
    pub fn gather_at(&self, slot: Slot, group: u8) -> Option<Row<'_>> {
        self.row(group, slot).map(|bytes| Row { group, bytes })
    }

    /// A word of a party written, every index on the word moved with it.
    pub fn set<T: Word, D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, a: AttrW<T>, v: Missing<T>) {
        let slot = self.check(dir, r);
        self.set_at(slot, a, v);
    }

    /// A word of the party at a slot written, as the day names its parties by their place; a slot past those begun
    /// stops the run.
    pub fn set_at<T: Word>(&mut self, slot: Slot, a: AttrW<T>, v: Missing<T>) {
        let a = a.0;
        let mut moved = [None; KIND_INDEXES];
        for (k, m) in self.keyed.iter().zip(&mut moved).filter(|(k, _)| (k.group, k.offset) == (a.group, a.offset)) {
            *m = Some(self.row(k.group, slot).and_then(|r| k.key_in(r)));
        }
        a.write_in(self.row_mut(a.group, slot), v);
        for (i, before) in moved.iter().enumerate().filter_map(|(i, m)| m.map(|b| (i, b))) {
            let after = self.keyed.get(i).and_then(|k| self.row(k.group, slot).and_then(|r| k.key_in(r)));
            let Some(k) = self.keyed.get_mut(i) else { continue };
            if before == after {
                continue;
            }
            if let Some(b) = before {
                k.index.left(b);
            }
            if let Some(n) = after {
                k.index.insert(n, slot.get());
            }
        }
    }

    /// One group's rows of every slot ever begun, in slot order, for a scan that reads a few words of each.
    pub fn scan(&self, group: u8) -> impl Iterator<Item = (Slot, Row<'_>)> + '_ {
        let (Some(w), Some(col)) = (self.widths.get(usize::from(group)), self.groups.get(usize::from(group))) else {
            violation!(clause = "REP.1", "a group the kind does not keep", group = group);
        };
        let (w, bytes) = (usize::from(*w), col.slice());
        (0_u32..).zip(bytes.chunks_exact(w)).map(move |(s, b)| (Slot::new(s), Row { group, bytes: b }))
    }

    /// The store split into chunks of `slots` slots, each owning its slots' rows of every group alone, for writes on
    /// the pool; an indexed word is written only through `set`.
    pub fn chunks_mut<'a, D: Backing>(&'a mut self, dir: &'a Directory<D>, slots: u32) -> Chunks<'a, D> {
        let mut rest: [&'a mut [u8]; KIND_GROUPS] = Default::default();
        for (r, col) in rest.iter_mut().zip(&mut self.groups) {
            *r = col.slice_mut();
        }
        let mut widths = [0; KIND_GROUPS];
        for (w, x) in widths.iter_mut().zip(&self.widths) {
            *w = *x;
        }
        let mut keyed = [None; KIND_INDEXES];
        for (k, x) in keyed.iter_mut().zip(&self.keyed) {
            *k = Some((x.group, x.offset));
        }
        Chunks { dir, kind: self.kind, rest, widths, keyed, slots, cursor: 0, rows: self.rows }
    }

    /// The kind its parties are of.
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// The slots ever begun.
    #[must_use]
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Each group's bytes a party.
    #[must_use]
    pub fn widths(&self) -> &[u16] {
        &self.widths
    }

    /// One group's bytes committed; none for a group the kind does not keep.
    #[must_use]
    pub fn group_bytes(&self, group: u8) -> Option<usize> {
        self.groups.get(usize::from(group)).map(Column::bytes_committed)
    }
}

/// A party's row of one group, its words read by handle.
#[derive(Clone, Copy, Debug)]
pub struct Row<'a> {
    group: u8,
    bytes: &'a [u8],
}

impl Row<'_> {
    #[inline]
    pub fn get<T: Word>(&self, a: Attr<T>) -> Missing<T> {
        if a.group != self.group {
            other_group(a.group);
        }
        a.read_in(self.bytes)
    }
}

/// A read past its row, kept out of line so the reads that never meet it stay small.
#[cold]
#[inline(never)]
fn past_row(offset: u16) -> ! {
    violation!(clause = "REP.1", "a word past its group's row", offset = offset)
}

#[cold]
#[inline(never)]
fn other_group(group: u8) -> ! {
    violation!(clause = "REP.1", "a word read from another group's row", group = group)
}

/// The store's rows handed out a chunk at a time.
#[derive(Debug)]
pub struct Chunks<'a, D: Backing> {
    dir: &'a Directory<D>,
    kind: u8,
    rest: [&'a mut [u8]; KIND_GROUPS],
    widths: [u16; KIND_GROUPS],
    keyed: [Option<(u8, u16)>; KIND_INDEXES],
    slots: u32,
    cursor: u32,
    rows: u32,
}

impl<'a, D: Backing> Iterator for Chunks<'a, D> {
    type Item = KindChunk<'a, D>;

    fn next(&mut self) -> Option<KindChunk<'a, D>> {
        if self.cursor >= self.rows {
            return None;
        }
        let first = self.cursor;
        let left = self.rows - first;
        let n = if left < self.slots { left } else { self.slots };
        self.cursor += n;
        let mut rows: [&'a mut [u8]; KIND_GROUPS] = Default::default();
        for ((r, rest), w) in rows.iter_mut().zip(&mut self.rest).zip(self.widths) {
            let take = usize::try_from(n).unwrap_or(usize::MAX) * usize::from(w);
            let (mine, after) = std::mem::take(rest).split_at_mut(take);
            (*r, *rest) = (mine, after);
        }
        Some(KindChunk { dir: self.dir, kind: self.kind, first, n, rows, widths: self.widths, keyed: self.keyed })
    }
}

/// One chunk's rows of every group, which one worker owns: it reads and writes its own slots' words alone.
#[derive(Debug)]
pub struct KindChunk<'a, D: Backing> {
    dir: &'a Directory<D>,
    kind: u8,
    first: u32,
    n: u32,
    rows: [&'a mut [u8]; KIND_GROUPS],
    widths: [u16; KIND_GROUPS],
    keyed: [Option<(u8, u16)>; KIND_INDEXES],
}

impl<D: Backing> KindChunk<'_, D> {
    /// The chunk's first slot and its slots.
    pub fn slots(&self) -> (Slot, u32) {
        (Slot::new(self.first), self.n)
    }

    fn row(&mut self, r: PartyRef, group: u8) -> &mut [u8] {
        let local = r.slot().get().checked_sub(self.first).filter(|s| *s < self.n);
        let (Some(local), true) = (local, r.kind() == self.kind && self.dir.reference(self.kind, r.slot()) == Some(r))
        else {
            violation!(clause = "PTY.10", "a chunk's write to a party it does not own", party = r.word());
        };
        let g = usize::from(group);
        let Some(w) = self.widths.get(g).map(|w| usize::from(*w)) else {
            violation!(clause = "REP.1", "a group the kind does not keep", group = group);
        };
        let start = usize::try_from(local).unwrap_or(usize::MAX) * w;
        match self.rows.get_mut(g).and_then(|b| b.get_mut(start..start + w)) {
            Some(row) => row,
            None => violation!(clause = "REP.1", "a group the kind does not keep", group = group),
        }
    }

    pub fn get<T: Word>(&mut self, r: PartyRef, a: Attr<T>) -> Missing<T> {
        a.read_in(self.row(r, a.group))
    }

    /// A word of one of the chunk's own parties written; an indexed word is refused, its index moving only by `set`.
    pub fn set<T: Word>(&mut self, r: PartyRef, a: AttrW<T>, v: Missing<T>) {
        if self.keyed.contains(&Some((a.0.group, a.0.offset))) {
            violation!(clause = "SET.12", "an indexed word written from a chunk", offset = a.0.offset);
        }
        a.0.write_in(self.row(r, a.0.group), v);
    }
}

/// A kind's rows are the slots it has ever begun; which are live is the directory's.
impl<B: Backing> StoreStats for KindStore<B> {
    fn rows_live(&self) -> u64 {
        u64::from(self.rows)
    }

    fn rows_ever(&self) -> u64 {
        u64::from(self.rows)
    }

    fn bytes(&self) -> u64 {
        let own: usize = self.groups.iter().map(Column::bytes_committed).sum::<usize>()
            + self.keyed.iter().map(|k| k.index.bytes_committed()).sum::<usize>();
        u64::try_from(own).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
