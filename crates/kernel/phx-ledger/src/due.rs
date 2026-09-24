use phx_core::calendar::Calendar;
use phx_id::{Day, LineId};
use phx_macros::clause;
use phx_num::Missing;
use phx_store::Backing;

use crate::apply::Ledger;

/// The lines whose dues fall on a day: a bit per line, cache-resident, and the lines themselves by identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DueLines {
    bits: Vec<u64>,
    lines: Vec<LineId>,
}

/// A line's word of the bitmap and its bit in the word.
fn place(line: LineId) -> (usize, u32) {
    let Ok(word) = usize::try_from(line.get() / u64::BITS) else {
        phx_num::capacity_exceeded!("lines for their bits", usize::MAX, line.get());
    };
    (word, line.get() % u64::BITS)
}

/// Words of the bitmap for a count of lines.
fn words(lines: usize) -> usize {
    let Ok(bits) = usize::try_from(u64::BITS) else {
        phx_num::capacity_exceeded!("bits of a word", usize::MAX, u64::BITS);
    };
    lines.div_ceil(bits)
}

impl DueLines {
    /// Whether a line falls due today.
    #[must_use]
    pub fn is_due(&self, line: LineId) -> bool {
        let (word, bit) = place(line);
        self.bits.get(word).is_some_and(|w| w >> bit & 1 == 1)
    }

    /// Today's due lines, by identity.
    pub fn lines(&self) -> &[LineId] {
        &self.lines
    }
}

impl<B: Backing> Ledger<B> {
    /// Sub-step 1b: every line whose next due day is today is marked, and moved on to its schedule's next date — or
    /// marked spent after its last — so the day's stream reads advanced dates and each line's index of the date that
    /// fell due today.
    #[clause("REG.5")]
    pub fn mark_due(&mut self, day: Day, calendar: &Calendar) -> DueLines {
        let lines = self.lines.falling(day);
        let mut bits = vec![0_u64; words(self.lines.len())];
        for line in &lines {
            let schedule = self.terms.get(self.lines.terms(*line)).schedule;
            let k = self.lines.fallen(*line) + 1;
            let spent = matches!(schedule.count, Missing::Present(n) if k >= n);
            let next = if spent { Missing::Absent } else { Missing::Present(schedule.dates.nth(calendar, k + 1)) };
            self.lines.fall(*line, k, next);
            let (word, bit) = place(*line);
            if let Some(w) = bits.get_mut(word) {
                *w |= 1 << bit;
            }
        }
        DueLines { bits, lines }
    }
}
