//! The directory's tombstones: the main run and the recent one, each sorted by packed reference, the main one found
//! through a fence of every few keys that stays in cache; and the day's endings, sorted as a phase of endings ends. The
//! day's close sorts the day's into the recent run, and once the recent run passes a share of the main one, merges it
//! in place from the back and drops what ended before the horizon.

use phx_macros::Pod;
use phx_num::violation;

use crate::consts::{TOMB_FENCE, TOMB_KEY_BITS, TOMB_MERGE_SHARE};

const KEY_MASK: u64 = (1 << TOMB_KEY_BITS) - 1;

/// A party that ended: its packed reference with the high byte of its end day's offset from the run's first day, and
/// its successor's packed reference (or none) with the low byte.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Tomb {
    pub(crate) ended: u64,
    pub(crate) successor: u64,
}

impl Tomb {
    pub(crate) fn key(self) -> u64 {
        self.ended & KEY_MASK
    }

    /// The days from the run's first to the day it ended.
    pub(crate) fn offset(self) -> u32 {
        u32::from(u16::from_be_bytes([self.ended.to_be_bytes()[0], self.successor.to_be_bytes()[0]]))
    }
}

/// The tombstones kept: the main run, its fence, the recent run, and the day's endings, sorted up to `sorted`.
#[derive(Debug, Default)]
pub(crate) struct Tombs {
    pub(crate) main: Vec<Tomb>,
    fences: Vec<u64>,
    pub(crate) recent: Vec<Tomb>,
    pub(crate) today: Vec<Tomb>,
    sorted: usize,
}

fn search(run: &[Tomb], key: u64) -> Option<usize> {
    run.binary_search_by_key(&key, |t| t.key()).ok()
}

/// A sorted run widened in place by another, merged from the back so nothing moves twice.
pub(crate) fn merge_in(into: &mut Vec<Tomb>, newer: &[Tomb]) {
    let mut kept = into.len();
    into.extend_from_slice(newer);
    let mut taken = newer.len();
    let mut at = into.len();
    while taken > 0 {
        at -= 1;
        let (Some(&old), Some(&new)) = (kept.checked_sub(1).and_then(|i| into.get(i)), newer.get(taken - 1)) else {
            // The older run is spent: the rest of the newer one lies before every older key.
            if let (Some(slot), Some(&new)) = (into.get_mut(at), newer.get(taken - 1)) {
                *slot = new;
            }
            taken -= 1;
            continue;
        };
        let next = if old.key() > new.key() {
            kept -= 1;
            old
        } else {
            taken -= 1;
            new
        };
        if let Some(slot) = into.get_mut(at) {
            *slot = next;
        }
    }
}

impl Tombs {
    /// A tombstone kept from a saved run: the main run, its fence laid again.
    pub(crate) fn from_main(main: Vec<Tomb>) -> Tombs {
        let mut t = Tombs { main, ..Tombs::default() };
        t.fence();
        t
    }

    fn fence(&mut self) {
        self.fences.clear();
        self.fences.extend(self.main.iter().step_by(TOMB_FENCE).map(|t| t.key()));
    }

    /// A party's ending, among the day's.
    pub(crate) fn push(&mut self, t: Tomb) {
        self.today.push(t);
    }

    /// The day's endings sorted, as a phase of them ends, so the reads that follow search them.
    pub(crate) fn sort_today(&mut self) {
        if self.sorted < self.today.len() {
            self.today.sort_unstable_by_key(|t| t.key());
            self.sorted = self.today.len();
        }
    }

    /// Where a key's tombstone lies: the day's (sorted part searched, the rest read through), the recent run, or the
    /// main run's block its fence names.
    fn locate(&self, key: u64) -> Option<(Run, usize)> {
        let Some((sorted, rest)) = self.today.split_at_checked(self.sorted) else {
            violation!(clause = "PTY.13", "the day's endings sorted past their length", sorted = self.sorted);
        };
        if let Some(i) = search(sorted, key) {
            return Some((Run::Today, i));
        }
        if let Some(i) = rest.iter().position(|t| t.key() == key) {
            return Some((Run::Today, self.sorted + i));
        }
        if let Some(i) = search(&self.recent, key) {
            return Some((Run::Recent, i));
        }
        let block = self.fences.partition_point(|f| *f <= key).checked_sub(1)?;
        let start = block * TOMB_FENCE;
        let end = if start + TOMB_FENCE < self.main.len() { start + TOMB_FENCE } else { self.main.len() };
        search(self.main.get(start..end)?, key).map(|i| (Run::Main, start + i))
    }

    pub(crate) fn find(&self, key: u64) -> Option<Tomb> {
        let (run, i) = self.locate(key)?;
        self.run(run).get(i).copied()
    }

    pub(crate) fn find_mut(&mut self, key: u64) -> Option<&mut Tomb> {
        let (run, i) = self.locate(key)?;
        match run {
            Run::Today => self.today.get_mut(i),
            Run::Recent => self.recent.get_mut(i),
            Run::Main => self.main.get_mut(i),
        }
    }

    fn run(&self, run: Run) -> &[Tomb] {
        match run {
            Run::Today => &self.today,
            Run::Recent => &self.recent,
            Run::Main => &self.main,
        }
    }

    /// The day's close: its endings sorted into the recent run; once the recent run passes its share of the main one,
    /// both merged into the main one in place, keeping only what `kept` admits. The tombstones the merge read.
    pub(crate) fn close(&mut self, kept: impl Fn(&Tomb) -> bool) -> usize {
        self.sort_today();
        merge_in(&mut self.recent, &self.today);
        self.today.clear();
        self.sorted = 0;
        if self.recent.len() * TOMB_MERGE_SHARE <= self.main.len() {
            return 0;
        }
        let read = self.main.len() + self.recent.len();
        merge_in(&mut self.main, &self.recent);
        self.recent.clear();
        self.main.retain(kept);
        self.fence();
        if self.main.windows(2).any(|w| matches!(w, [a, b] if a.key() >= b.key())) {
            violation!(clause = "PTY.13", "a party with two tombstones");
        }
        read
    }

    pub(crate) fn len(&self) -> usize {
        self.main.len() + self.recent.len() + self.today.len()
    }

    pub(crate) fn bytes(&self) -> usize {
        (self.main.capacity() + self.recent.capacity() + self.today.capacity()) * size_of::<Tomb>()
            + self.fences.capacity() * size_of::<u64>()
    }
}

/// Which of the runs a tombstone lies in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Run {
    Today,
    Recent,
    Main,
}
