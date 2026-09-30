//! What the day changed, with no clearing pass: a row marked by writing today into a stamp, or a bit under a word
//! stamped with today; "touched today" read by iterating the set words only.

use phx_id::Slot;
use phx_macros::opening;
use phx_num::violation;

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::convert::{to_u64, to_usize};
use crate::stats::StoreStats;

/// Rows a word of bits covers, and words a summary word covers.
const WORD: u32 = u64::BITS;

/// The stamp of a word or row no day has marked: a day no run reaches, only ever compared with today.
const NEVER: u32 = u32::MAX;

/// The rows a chunk of the bits covers, whose words and summary word one writer holds.
pub const EPOCH_CHUNK_ROWS: u32 = WORD * WORD;

/// A bit a row under a day a word of 64, and a summary bit a word under a day a summary word: a mark whose word's day
/// is not today clears the word first, so nothing is ever cleared by a pass, and a day's marks are read by visiting
/// only the words their summary marks.
#[derive(Debug)]
pub struct EpochBits<B: Backing = SystemBacking> {
    bits: Column<u64, B>,
    days: Column<u32, B>,
    summary: Column<u64, B>,
    summary_days: Column<u32, B>,
}

/// One chunk of the bits: 4 096 rows' words and their summary word, held by the one writer of those rows.
#[derive(Debug)]
pub struct EpochChunk<'a> {
    first: u32,
    bits: &'a mut [u64],
    days: &'a mut [u32],
    summary: &'a mut u64,
    summary_day: &'a mut u32,
}

/// The mark: a word not yet stamped today is cleared and stamped, and its summary bit set — once a word a day — then
/// the row's bit set.
fn mark_in(
    (bits, days): (&mut [u64], &mut [u32]),
    (summary, summary_day): (&mut u64, &mut u32),
    at: usize,
    today: u32,
) {
    let (word, bit) = (at / to_usize(WORD), at % to_usize(WORD));
    let (Some(w), Some(d)) = (bits.get_mut(word), days.get_mut(word)) else {
        violation!(clause = "SET.12", "a mark past its chunk's rows", at = at);
    };
    if *d != today {
        (*w, *d) = (0, today);
        if *summary_day != today {
            (*summary, *summary_day) = (0, today);
        }
        *summary |= 1 << (word % to_usize(WORD));
    }
    *w |= 1 << bit;
}

impl EpochChunk<'_> {
    /// The chunk's first row.
    #[must_use]
    pub fn first(&self) -> u32 {
        self.first
    }

    /// Marks a row of the chunk as touched today.
    pub fn mark(&mut self, slot: Slot, today: u32) {
        let Some(at) = slot.get().checked_sub(self.first).filter(|at| *at < EPOCH_CHUNK_ROWS) else {
            violation!(clause = "SET.12", "a mark outside its writer's chunk", slot = slot.get());
        };
        mark_in((self.bits, self.days), (self.summary, self.summary_day), to_usize(at), today);
    }
}

impl<B: Backing> EpochBits<B> {
    /// The bits of at most `rows` rows, every word stamped with no day's marks.
    #[opening]
    pub fn new(space: &mut AddressSpace, rows: u32) -> EpochBits<B> {
        let words = rows.div_ceil(WORD);
        let summaries = words.div_ceil(WORD);
        let chunk = crate::consts::SUMTREE_ROWS_PER_CHUNK;
        let mut e = EpochBits {
            bits: Column::new(space, summaries * WORD, chunk),
            days: Column::new(space, summaries * WORD, chunk),
            summary: Column::new(space, summaries, chunk),
            summary_days: Column::new(space, summaries, chunk),
        };
        // Day stamps begin at no day, so the first mark of any day, the opening's included, clears its word.
        for _ in 0..summaries * WORD {
            e.bits.push(0);
            e.days.push(NEVER);
        }
        for _ in 0..summaries {
            e.summary.push(0);
            e.summary_days.push(NEVER);
        }
        e
    }

    /// Marks a row as touched today: one compare and one bit set once its word is today's.
    pub fn mark(&mut self, slot: Slot, today: u32) {
        let word = to_usize(slot.get() / WORD);
        let (Some(w), Some(d)) = (self.bits.slice_mut().get_mut(word), self.days.slice_mut().get_mut(word)) else {
            violation!(clause = "SET.12", "a mark past the bits' rows", slot = slot.get());
        };
        if *d != today {
            (*w, *d) = (0, today);
            let s = word / to_usize(WORD);
            let (Some(summary), Some(summary_day)) =
                (self.summary.slice_mut().get_mut(s), self.summary_days.slice_mut().get_mut(s))
            else {
                violation!(clause = "SET.12", "a mark past the bits' rows", slot = slot.get());
            };
            if *summary_day != today {
                (*summary, *summary_day) = (0, today);
            }
            *summary |= 1 << (word % to_usize(WORD));
        }
        *w |= 1 << (slot.get() % WORD);
    }

    /// The bits in chunks of 4 096 rows, each for the one writer of those rows, so marks need no atomics.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = EpochChunk<'_>> {
        let (bits, days) = (self.bits.slice_mut(), self.days.slice_mut());
        let (summary, summary_days) = (self.summary.slice_mut(), self.summary_days.slice_mut());
        (0_u32..)
            .zip(bits.chunks_mut(to_usize(WORD)).zip(days.chunks_mut(to_usize(WORD))))
            .zip(summary.iter_mut().zip(summary_days.iter_mut()))
            .map(|((s, (bits, days)), (summary, summary_day))| EpochChunk {
                first: s * EPOCH_CHUNK_ROWS,
                bits,
                days,
                summary,
                summary_day,
            })
    }

    /// Whether a row was marked today.
    #[must_use]
    pub fn is_marked(&self, slot: Slot, today: u32) -> bool {
        let word = to_usize(slot.get() / WORD);
        self.days.slice().get(word) == Some(&today)
            && self.bits.slice().get(word).is_some_and(|w| w & (1 << (slot.get() % WORD)) != 0)
    }

    /// Calls `each` on every word marked today, in slot order — its first row and its bits — visiting only the words
    /// their summary marks: a reader handles a word's rows in its own loop.
    pub fn for_each_marked_word(&self, today: u32, mut each: impl FnMut(Slot, u64)) {
        let (bits, days) = (self.bits.slice(), self.days.slice());
        for (s, (summary, day)) in (0_u32..).zip(self.summary.slice().iter().zip(self.summary_days.slice())) {
            if *day != today {
                continue;
            }
            let mut words = *summary;
            while words != 0 {
                let w = s * WORD + words.trailing_zeros();
                words &= words - 1;
                let at = to_usize(w);
                if let (Some(&word), Some(&d)) = (bits.get(at), days.get(at))
                    && d == today
                {
                    each(Slot::new(w * WORD), word);
                }
            }
        }
    }

    /// Calls `each` on every row marked today, in slot order, visiting only the words their summary marks.
    pub fn for_each_marked(&self, today: u32, mut each: impl FnMut(Slot)) {
        let (bits, days) = (self.bits.slice(), self.days.slice());
        for (s, (summary, day)) in (0_u32..).zip(self.summary.slice().iter().zip(self.summary_days.slice())) {
            if *day != today {
                continue;
            }
            let mut words = *summary;
            while words != 0 {
                let w = s * WORD + words.trailing_zeros();
                words &= words - 1;
                let at = to_usize(w);
                if days.get(at) != Some(&today) {
                    continue;
                }
                let Some(&word) = bits.get(at) else {
                    violation!(clause = "SET.12", "a summary naming a word past the bits", word = w);
                };
                let mut rows = word;
                while rows != 0 {
                    each(Slot::new(w * WORD + rows.trailing_zeros()));
                    rows &= rows - 1;
                }
            }
        }
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.bits.bytes_committed()
            + self.days.bytes_committed()
            + self.summary.bytes_committed()
            + self.summary_days.bytes_committed()
    }
}

impl<B: Backing> StoreStats for EpochBits<B> {
    /// The rows the bits cover.
    fn rows_live(&self) -> u64 {
        to_u64(self.bits.len()) * u64::from(WORD)
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

/// A row's last day, for readers that ask whether a row was touched today — a per-day cache's stamp, a perishable
/// capacity's day of use: the test is "stamp is today", and no reset ever runs.
#[derive(Debug)]
pub struct DayStamps<B: Backing = SystemBacking> {
    days: Column<u32, B>,
}

impl<B: Backing> DayStamps<B> {
    /// Stamps for at most `rows` rows, none stamped.
    #[opening]
    pub fn new(space: &mut AddressSpace, rows: u32) -> DayStamps<B> {
        let mut days = Column::new(space, rows, crate::consts::SUMTREE_ROWS_PER_CHUNK);
        for _ in 0..rows {
            days.push(NEVER);
        }
        DayStamps { days }
    }

    /// Stamps a row with today.
    pub fn stamp(&mut self, slot: Slot, today: u32) {
        match self.days.slice_mut().get_mut(to_usize(slot.get())) {
            Some(d) => *d = today,
            None => violation!(clause = "SET.12", "a stamp past its rows", slot = slot.get()),
        }
    }

    /// Whether a row was stamped today.
    #[must_use]
    pub fn is_today(&self, slot: Slot, today: u32) -> bool {
        self.days.slice().get(to_usize(slot.get())) == Some(&today)
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.days.bytes_committed()
    }
}

impl<B: Backing> StoreStats for DayStamps<B> {
    fn rows_live(&self) -> u64 {
        to_u64(self.days.len())
    }

    fn rows_ever(&self) -> u64 {
        self.rows_live()
    }

    fn bytes(&self) -> u64 {
        to_u64(self.bytes_committed())
    }
}

#[cfg(test)]
#[path = "epoch_tests.rs"]
mod tests;
