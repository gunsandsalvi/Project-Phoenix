use phx_id::Day;
use phx_macros::clause;

use crate::consts::TRACE_PERIOD;

/// Whether a chunk is read-traced on a day: the first chunk of each (handler, table) in the run, and the chunks
/// whose index is congruent to the day.
#[must_use]
pub fn traced(chunk: u32, first_of_its_pair: bool, day: Day) -> bool {
    first_of_its_pair || chunk % TRACE_PERIOD == day.get() % TRACE_PERIOD
}

/// A stream opened for a subject at a sub-step, as the trace records it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Open {
    pub stream: &'static str,
    pub subject: u64,
    pub substep: u8,
}

/// What `read-trace` found: reads of undeclared facts, reads of a later write, and streams opened twice for one
/// subject in one sub-step.
#[clause("TIME.10", "CHN.6")]
#[derive(Debug, Default)]
pub struct TraceLog {
    pub undeclared_reads: usize,
    pub later_writes: usize,
    pub duplicate_opens: usize,
    opens: Vec<Open>,
}

impl TraceLog {
    pub fn opened(&mut self, open: Open) {
        self.opens.push(open);
    }

    /// Sorts the day's opens and counts each repeated (stream, subject, sub-step), then clears them.
    pub fn close_day(&mut self) {
        self.opens.sort_unstable();
        let repeats = self.opens.windows(2).filter(|w| w.first() == w.last()).count();
        self.duplicate_opens += repeats;
        self.opens.clear();
    }

    #[must_use]
    pub fn clean(&self) -> bool {
        self.undeclared_reads == 0 && self.later_writes == 0 && self.duplicate_opens == 0
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;

    use super::{Open, TraceLog, traced};

    #[test]
    fn trace_sampling_is_deterministic() {
        let day = Day::new(130);
        let chosen: Vec<u32> = (0..200).filter(|c| traced(*c, false, day)).collect();
        assert_eq!(chosen, vec![2, 66, 130, 194]);
        assert!(traced(7, true, day));
        assert_eq!(chosen, (0..200).filter(|c| traced(*c, false, day)).collect::<Vec<_>>());
    }

    #[test]
    fn duplicate_opens_are_found() {
        let mut log = TraceLog::default();
        let open = Open { stream: "DEM.mortality", subject: 4, substep: 11 };
        log.opened(open);
        log.opened(Open { substep: 12, ..open });
        log.close_day();
        assert!(log.clean());
        log.opened(open);
        log.opened(open);
        log.close_day();
        assert_eq!(log.duplicate_opens, 1);
    }
}
