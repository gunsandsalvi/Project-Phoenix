//! Windowed groups: a kind's columns that exist only inside a dated window, such as a campaign's voting intentions. A
//! window covers a run of slots — a country's parties, begun region by region — and reserves its rows in address
//! space when it opens, committing pages only as far as it has been written; outside an open window, or on a party not
//! live, its words read `Missing`; at its close its pages are returned.

use phx_id::{Day, PartyRef, Slot};
use phx_macros::{clause, opening};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, Column, StoreStats, SystemBacking};

use crate::directory::Directory;
use crate::kinds::{Attr, AttrW, Word};
use crate::layout::{GroupDecl, KindMap, Layout};

/// A windowed group's layout, whose handles read and write its windows alone.
#[derive(Clone, Debug)]
pub struct WindowLayout(Layout);

/// A windowed word's read handle.
#[derive(Debug)]
pub struct WAttr<T>(Attr<T>);

/// A windowed word's write handle, handed once to the base that declared it.
#[derive(Debug)]
pub struct WAttrW<T>(AttrW<T>);

impl<T> Clone for WAttr<T> {
    fn clone(&self) -> WAttr<T> {
        *self
    }
}

impl<T> Copy for WAttr<T> {}

impl<T> Clone for WAttrW<T> {
    fn clone(&self) -> WAttrW<T> {
        *self
    }
}

impl<T> Copy for WAttrW<T> {}

impl<T> WAttrW<T> {
    #[must_use]
    pub fn read(self) -> WAttr<T> {
        WAttr(self.0.read())
    }
}

impl WindowLayout {
    /// A kind's windowed group: its words, as a map of one group.
    ///
    /// # Errors
    /// As `Layout::compile`.
    #[opening]
    pub fn compile(kind: &'static str, group: &'static [GroupDecl]) -> Result<WindowLayout, String> {
        if group.len() != 1 {
            return Err(format!("kind `{kind}`: a windowed group is one group, not {}", group.len()));
        }
        Layout::compile(&KindMap { kind, groups: group }, &[]).map(WindowLayout)
    }

    /// # Errors
    /// As `Layout::attr`.
    #[opening]
    pub fn attr<T: Word>(&self, name: &str, i: u16) -> Result<WAttr<T>, String> {
        self.0.attr(name, i).map(WAttr)
    }

    /// # Errors
    /// As `Layout::writer`.
    #[opening]
    pub fn writer<T: Word>(&mut self, name: &str, i: u16, base: &str) -> Result<WAttrW<T>, String> {
        self.0.writer(name, i, base).map(WAttrW)
    }
}

/// One open window: what it covers (its subject, such as a country, and its run of slots), the days it opened and
/// closes, its rows from the run's first slot, and how many of them have been written into.
#[derive(Debug)]
pub(crate) struct Window<B: Backing> {
    pub(crate) subject: u32,
    pub(crate) first: u32,
    pub(crate) slots: u32,
    pub(crate) opened: Day,
    pub(crate) closes: Day,
    pub(crate) rows: Column<u8, B>,
    pub(crate) filled: u32,
}

/// A kind's windowed group: its kind's number, its row's width and blank, and its windows open now.
#[clause("REP.1")]
#[derive(Debug)]
pub struct WindowedGroup<B: Backing = SystemBacking> {
    pub(crate) kind: u8,
    pub(crate) width: u16,
    pub(crate) blank: Vec<u8>,
    pub(crate) windows: Vec<Window<B>>,
}

impl<B: Backing> WindowedGroup<B> {
    /// A kind's windowed group with none open, room for `subjects` windows at once.
    #[must_use]
    #[opening]
    pub fn new(kind: u8, layout: &WindowLayout, subjects: usize) -> WindowedGroup<B> {
        let width = layout.0.widths.first().copied().unwrap_or_else(|| {
            violation!(clause = "REP.1", "a windowed group of no row", kind = kind);
        });
        let blank = layout.0.blanks().concat();
        WindowedGroup { kind, width, blank, windows: Vec::with_capacity(subjects) }
    }

    /// A window opened for a subject over a run of slots, from its open day to its close day: its rows reserved in
    /// address space, none committed until written. A subject's second window, or more windows than the group was
    /// built for, stops the run.
    #[clause("REP.1")]
    pub fn open(
        &mut self,
        space: &mut AddressSpace,
        subject: u32,
        (first, slots): (Slot, u32),
        (opened, closes): (Day, Day),
    ) {
        if self.windows.iter().any(|w| w.subject == subject) {
            violation!(clause = "SET.12", "a second window open for one subject", subject = subject);
        }
        if self.windows.len() == self.windows.capacity() {
            capacity_exceeded!("windows open at once", self.windows.capacity(), self.windows.len() + 1);
        }
        let Some(bytes) = slots.checked_mul(u32::from(self.width)) else {
            capacity_exceeded!("a window's bytes", u32::MAX, u64::from(slots) * u64::from(self.width));
        };
        let rows = Column::new(space, bytes, crate::consts::KIND_CHUNK_BYTES);
        self.windows.push(Window { subject, first: first.get(), slots, opened, closes, rows, filled: 0 });
    }

    /// A subject's window closed: its pages returned and its reservation released. A subject with none open stops the
    /// run.
    pub fn close(&mut self, subject: u32) {
        let Some(at) = self.windows.iter().position(|w| w.subject == subject) else {
            violation!(clause = "SET.12", "a window closed that is not open", subject = subject);
        };
        drop(self.windows.swap_remove(at));
    }

    /// The subjects whose windows close by a day, for the close to call `close` on.
    pub fn closing(&self, today: Day) -> impl Iterator<Item = u32> + '_ {
        self.windows.iter().filter(move |w| w.closes <= today).map(|w| w.subject)
    }

    /// The window covering a slot, and the slot's place in it.
    fn covering(&self, slot: Slot) -> Option<(usize, u32)> {
        self.windows.iter().enumerate().find_map(|(i, w)| {
            let local = slot.get().checked_sub(w.first).filter(|l| *l < w.slots)?;
            Some((i, local))
        })
    }

    fn offset(&self, local: u32) -> usize {
        let (l, w) = (usize::try_from(local).unwrap_or(usize::MAX), usize::from(self.width));
        l.checked_mul(w).unwrap_or_else(|| capacity_exceeded!("a window's row bytes", usize::MAX, l))
    }

    /// A word of a party: `Missing` where no window covers its slot, where the party is not live — ended, or its slot
    /// since taken by another — and where the window has not written it.
    pub fn get<T: Word, D: Backing>(&self, dir: &Directory<D>, r: PartyRef, a: WAttr<T>) -> Missing<T> {
        let Some((i, local)) = self.covering(r.slot()) else { return Missing::Absent };
        if r.kind() != self.kind || dir.at(self.kind, r.slot()) != Some(r) {
            return Missing::Absent;
        }
        let Some(w) = self.windows.get(i).filter(|w| local < w.filled) else { return Missing::Absent };
        let start = self.offset(local);
        match w.rows.slice().get(start..start + usize::from(self.width)) {
            Some(row) => a.0.read_in(row),
            None => violation!(clause = "REP.1", "a window's row past its written rows", slot = r.slot().get()),
        }
    }

    /// A word of a live party written inside a window covering it, committing the window's rows as far as it; a
    /// write outside every window, or to a party not live, stops the run.
    pub fn set<T: Word, D: Backing>(&mut self, dir: &Directory<D>, r: PartyRef, a: WAttrW<T>, v: Missing<T>) {
        let Some((at, local)) = self.covering(r.slot()) else {
            violation!(clause = "SET.12", "a windowed word written outside its window", slot = r.slot().get());
        };
        if r.kind() != self.kind || dir.at(self.kind, r.slot()) != Some(r) {
            violation!(clause = "PTY.10", "a windowed word written for a party not live", party = r.word());
        }
        let start = self.offset(local);
        let width = usize::from(self.width);
        let Some(window) = self.windows.get_mut(at) else { return };
        while window.filled <= local {
            window.rows.extend(&self.blank);
            window.filled += 1;
        }
        match window.rows.slice_mut().get_mut(start..start + width) {
            Some(row) => a.0.read().write_in(row, v),
            None => violation!(clause = "REP.1", "a window's row past its written rows", slot = r.slot().get()),
        }
    }

    /// A party begun in a slot an open window has written: its row blanked, so it reads nothing of the slot's last
    /// party.
    pub fn begun(&mut self, r: PartyRef) {
        let Some((i, local)) = self.covering(r.slot()) else { return };
        let start = self.offset(local);
        let width = usize::from(self.width);
        let Some(w) = self.windows.get_mut(i).filter(|w| local < w.filled) else { return };
        if let Some(row) = w.rows.slice_mut().get_mut(start..start + width) {
            row.copy_from_slice(&self.blank);
        }
    }

    /// The windows open now.
    #[must_use]
    pub fn open_windows(&self) -> usize {
        self.windows.len()
    }
}

/// A windowed group's rows are its open windows' rows written into.
impl<B: Backing> StoreStats for WindowedGroup<B> {
    fn rows_live(&self) -> u64 {
        self.windows.iter().map(|w| u64::from(w.filled)).sum()
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        let committed: usize = self.windows.iter().map(|w| w.rows.bytes_committed()).sum();
        u64::try_from(committed).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "windowed_tests.rs"]
mod tests;
