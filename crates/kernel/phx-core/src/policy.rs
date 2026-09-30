//! Dated policy schedules: every POLICY primitive its owner may change during the run — a tax rate, a policy rate, a
//! haircut, a capital rule — held as its opening value and its dated changes, each announced on one day and in force
//! from a later one, saved with the world and read in force on a day from a cache the day's open advances.

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::marker::PhantomData;

use phx_id::{CountryId, Day, PartyRef};
use phx_macros::clause;
use phx_num::violation;
use phx_store::{Saved, StoreStats};

use crate::calendar::Calendar;
use crate::register::values::PrimType;
use crate::register::{Prim, PrimKind, Register};

/// A schedule's handle, bound once at assembly: an index its rules read the value in force by.
#[derive(Debug)]
pub struct PolicyH<T> {
    index: u32,
    marker: PhantomData<fn() -> T>,
}

impl<T> Clone for PolicyH<T> {
    fn clone(&self) -> PolicyH<T> {
        *self
    }
}

impl<T> Copy for PolicyH<T> {}

impl<T> PartialEq for PolicyH<T> {
    fn eq(&self, other: &PolicyH<T>) -> bool {
        self.index == other.index
    }
}

impl<T> Eq for PolicyH<T> {}

/// A change of a policy: in force from `effective`, announced on `announced` by `owner`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct PolicyEntry<T: Saved> {
    pub effective: Day,
    pub announced: Day,
    pub owner: PartyRef,
    pub value: T,
}

/// Why an announcement is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnounceRefused {
    /// In force before the next business day of the owner's country after it is announced.
    TooSoon { earliest: Day },
    /// Announced by a party other than the schedule's owner.
    NotOwner,
}

/// Every dated policy schedule of one value type: each schedule's opening value, owner and country, its changes in one
/// column ordered by schedule then effective day, and the values in force on the day last opened, with the schedules
/// whose next change is still to come ordered by its day.
#[clause("NUM.3", "POL.7", "TIME.7")]
#[derive(Debug, phx_macros::Saved)]
pub struct PolicyBook<T: Saved + Copy> {
    opening: Vec<T>,
    owner: Vec<PartyRef>,
    country: Vec<CountryId>,
    entries: Vec<PolicyEntry<T>>,
    /// Where each schedule's changes begin in `entries`, and one past the last schedule's end.
    starts: Vec<u32>,
    opened: Day,
    #[saved(skip, rebuild = PolicyBook::rebuild_cache)]
    cache: PolicyCache<T>,
}

/// The values in force on the day last opened, one a schedule, and the schedules whose next change is still to come,
/// by its day: derived from the schedules.
#[derive(Debug)]
struct PolicyCache<T> {
    current: Vec<T>,
    pending: BinaryHeap<Reverse<(Day, u32)>>,
}

impl<T> Default for PolicyCache<T> {
    #[phx_macros::opening]
    fn default() -> PolicyCache<T> {
        PolicyCache { current: Vec::new(), pending: BinaryHeap::new() }
    }
}

impl<T: Saved + Copy> Default for PolicyBook<T> {
    #[phx_macros::opening]
    fn default() -> PolicyBook<T> {
        PolicyBook {
            opening: Vec::new(),
            owner: Vec::new(),
            country: Vec::new(),
            entries: Vec::new(),
            starts: vec![0],
            opened: Day::new(0),
            cache: PolicyCache::default(),
        }
    }
}

/// Two books are equal when their schedules are: the cache is derived from them.
impl<T: Saved + Copy + PartialEq> PartialEq for PolicyBook<T> {
    fn eq(&self, other: &PolicyBook<T>) -> bool {
        (&self.opening, &self.owner, &self.country, &self.entries, &self.starts, self.opened)
            == (&other.opening, &other.owner, &other.country, &other.entries, &other.starts, other.opened)
    }
}

fn place(n: usize) -> u32 {
    match u32::try_from(n) {
        Ok(i) => i,
        Err(_) => phx_num::capacity_exceeded!("policy entries", u32::MAX, n),
    }
}

fn wide(n: usize) -> u64 {
    match u64::try_from(n) {
        Ok(w) => w,
        Err(_) => phx_num::capacity_exceeded!("policy entries", u64::MAX, n),
    }
}

fn at(i: u32) -> usize {
    match usize::try_from(i) {
        Ok(i) => i,
        Err(_) => phx_num::capacity_exceeded!("policy entries", usize::MAX, i),
    }
}

impl<T: Saved + Copy> PolicyBook<T> {
    /// A schedule of a POLICY primitive for a country, its opening value the register's and its owner the party that
    /// holds the primitive's decision there, bound once as the world is assembled.
    ///
    /// # Errors
    /// A primitive that is not a policy, or whose identity names no owning system.
    #[phx_macros::opening]
    pub fn bind<P>(
        &mut self,
        (prim, register): (Prim<P>, &Register),
        country: CountryId,
        owner: PartyRef,
    ) -> Result<PolicyH<T>, String>
    where
        P: PrimType,
        for<'a> P::Read<'a>: Into<T>,
    {
        let decl = prim.decl(register);
        let _owner_system = decl.owner()?;
        if decl.kind != PrimKind::Policy {
            return Err(format!("`{}` is not a policy, and has no schedule", decl.id));
        }
        Ok(self.open(prim.get(register, country).into(), country, owner))
    }

    /// A schedule of an opening value its owner may change.
    #[phx_macros::opening]
    pub fn open(&mut self, opening: T, country: CountryId, owner: PartyRef) -> PolicyH<T> {
        let index = place(self.opening.len());
        self.opening.push(opening);
        self.owner.push(owner);
        self.country.push(country);
        self.cache.current.push(opening);
        let Some(&end) = self.starts.last() else {
            violation!(clause = "NUM.3", "a policy book with no end to its schedules");
        };
        self.starts.push(end);
        PolicyH { index, marker: PhantomData }
    }

    fn span(&self, h: PolicyH<T>) -> std::ops::Range<usize> {
        let i = at(h.index);
        match (self.starts.get(i), self.starts.get(i + 1)) {
            (Some(&from), Some(&to)) => at(from)..at(to),
            _ => violation!(clause = "NUM.3", "a policy handle bound to no schedule", index = h.index),
        }
    }

    /// A schedule's changes, by effective day.
    #[must_use]
    pub fn entries(&self, h: PolicyH<T>) -> &[PolicyEntry<T>] {
        let span = self.span(h);
        match self.entries.get(span) {
            Some(e) => e,
            None => violation!(clause = "NUM.3", "a schedule's changes past the column", index = h.index),
        }
    }

    /// Records a change the owner announces on `announced`, in force from `effective`, at least the next business day
    /// of the owner's country after it.
    ///
    /// # Errors
    /// A writer other than the schedule's owner, or a change in force too soon.
    pub fn announce(
        &mut self,
        (h, writer): (PolicyH<T>, PartyRef),
        calendar: &Calendar,
        (announced, effective, value): (Day, Day, T),
    ) -> Result<(), AnnounceRefused> {
        let i = at(h.index);
        let (Some(&owner), Some(&country)) = (self.owner.get(i), self.country.get(i)) else {
            violation!(clause = "NUM.3", "a policy handle bound to no schedule", index = h.index);
        };
        if writer != owner {
            return Err(AnnounceRefused::NotOwner);
        }
        let earliest = calendar.next_business(country, announced);
        if effective < earliest {
            return Err(AnnounceRefused::TooSoon { earliest });
        }
        let span = self.span(h);
        let within = self.entries(h).partition_point(|x| x.effective <= effective);
        self.entries.insert(span.start + within, PolicyEntry { effective, announced, owner: writer, value });
        for s in self.starts.iter_mut().skip(i + 1) {
            *s += 1;
        }
        if effective > self.opened {
            self.cache.pending.push(Reverse((effective, h.index)));
        }
        Ok(())
    }

    /// The value in force on `day`: the last change effective on or before it, or the opening value.
    #[must_use]
    pub fn in_force(&self, h: PolicyH<T>, day: Day) -> T {
        let entries = self.entries(h);
        let n = entries.partition_point(|e| e.effective <= day);
        match n.checked_sub(1).and_then(|k| entries.get(k)) {
            Some(e) => e.value,
            None => match self.opening.get(at(h.index)) {
                Some(v) => *v,
                None => violation!(clause = "NUM.3", "a policy handle bound to no schedule", index = h.index),
            },
        }
    }

    /// The changes a party could know on `day`, those announced by then, with their effective days.
    pub fn announced_by(&self, h: PolicyH<T>, day: Day) -> impl Iterator<Item = &PolicyEntry<T>> {
        self.entries(h).iter().filter(move |e| e.announced <= day)
    }

    /// The values a period from `from` until `until` spans, each with its own days, from its first until the next's:
    /// a change in force within the period splits it there, and changes in force on one day count as the last.
    #[must_use]
    pub fn segments(&self, h: PolicyH<T>, (from, until): (Day, Day)) -> Segments<'_, T> {
        let entries = self.entries(h);
        let next = entries.partition_point(|e| e.effective <= from);
        Segments { book: self, h, entries, at: next, start: from, until }
    }

    /// Opens a day: every schedule whose change takes effect by it moves to its value in force; the others are not
    /// read.
    pub fn open_day(&mut self, day: Day) {
        self.opened = day;
        while let Some(&Reverse((effective, index))) = self.cache.pending.peek() {
            if effective > day {
                break;
            }
            let _ = self.cache.pending.pop();
            let h = PolicyH { index, marker: PhantomData };
            let value = self.in_force(h, day);
            if let Some(c) = self.cache.current.get_mut(at(index)) {
                *c = value;
            }
        }
    }

    /// The value in force on the day last opened, by handle: one indexed read.
    #[must_use]
    pub fn read(&self, h: PolicyH<T>) -> T {
        match self.cache.current.get(at(h.index)) {
            Some(v) => *v,
            None => violation!(clause = "NUM.3", "a policy handle bound to no schedule", index = h.index),
        }
    }

    /// An owner that ended: its schedules pass to its successor, which alone may change them from then on.
    pub fn succeed(&mut self, ended: PartyRef, successor: PartyRef) -> u64 {
        let mut moved = 0;
        for o in &mut self.owner {
            if *o == ended {
                *o = successor;
                moved += 1;
            }
        }
        moved
    }

    /// The values in force on the day last opened and the changes still to come, rebuilt from the schedules after a
    /// load.
    fn rebuild_cache(&mut self) -> u64 {
        let schedules = self.opening.len();
        self.cache.current.clear();
        self.cache.pending.clear();
        for i in 0..schedules {
            let h = PolicyH { index: place(i), marker: PhantomData };
            let value = self.in_force(h, self.opened);
            self.cache.current.push(value);
            if let Some(next) = self.entries(h).iter().find(|e| e.effective > self.opened) {
                self.cache.pending.push(Reverse((next.effective, h.index)));
            }
        }
        wide(schedules)
    }
}

/// A period's segments, each `(first, until, value)`: its days from `first` until the next segment's first, the value
/// in force throughout.
#[derive(Debug)]
pub struct Segments<'a, T: Saved + Copy> {
    book: &'a PolicyBook<T>,
    h: PolicyH<T>,
    entries: &'a [PolicyEntry<T>],
    at: usize,
    start: Day,
    until: Day,
}

impl<T: Saved + Copy> Iterator for Segments<'_, T> {
    type Item = (Day, Day, T);

    fn next(&mut self) -> Option<(Day, Day, T)> {
        if self.start >= self.until {
            return None;
        }
        // The next change that moves the value, past those in force on the segment's own first day.
        while self.entries.get(self.at).is_some_and(|e| e.effective <= self.start) {
            self.at += 1;
        }
        let end = match self.entries.get(self.at) {
            Some(e) if e.effective < self.until => e.effective,
            _ => self.until,
        };
        let segment = (self.start, end, self.book.in_force(self.h, self.start));
        self.start = end;
        Some(segment)
    }
}

impl<T: Saved + Copy> StoreStats for PolicyBook<T> {
    fn rows_live(&self) -> u64 {
        wide(self.entries.len())
    }

    fn rows_ever(&self) -> u64 {
        wide(self.entries.len())
    }

    fn bytes(&self) -> u64 {
        let per_schedule = size_of::<T>() * 2 + size_of::<PartyRef>() + size_of::<CountryId>() + size_of::<u32>();
        wide(self.entries.capacity() * size_of::<PolicyEntry<T>>() + self.opening.capacity() * per_schedule)
    }
}

#[cfg(test)]
#[path = "policy_tests.rs"]
mod tests;
