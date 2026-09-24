use phx_core::ReadTrace;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, phx_macros::Saved)]
pub struct Open {
    pub stream: &'static str,
    pub subject: u64,
    pub substep: u8,
}

/// What `read-trace` found over the run: reads of undeclared facts, reads of a later write, and streams opened twice
/// for one subject in one sub-step.
#[clause("TIME.10", "CHN.6")]
#[derive(Debug, Default, phx_macros::Saved)]
pub struct TraceLog {
    total: ReadTrace,
    opens: Vec<Open>,
}

impl TraceLog {
    pub fn opened(&mut self, open: Open) {
        self.opens.push(open);
    }

    /// Sorts the day's opens and counts each repeated (stream, subject, sub-step), then clears them; with what the
    /// tables found of the day's reads, what the day found joins the run's total.
    pub fn close_day(&mut self, reads: ReadTrace) -> ReadTrace {
        self.opens.sort_unstable();
        let repeats = self.opens.windows(2).filter(|w| w.first() == w.last()).count();
        self.opens.clear();
        let today = ReadTrace {
            undeclared_reads: reads.undeclared_reads,
            later_writes: reads.later_writes,
            duplicate_opens: reads.duplicate_opens + repeats,
        };
        self.total.undeclared_reads += today.undeclared_reads;
        self.total.later_writes += today.later_writes;
        self.total.duplicate_opens += today.duplicate_opens;
        today
    }

    #[must_use]
    pub fn total(&self) -> ReadTrace {
        self.total
    }

    #[must_use]
    pub fn clean(&self) -> bool {
        self.total == ReadTrace::default()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;

    use phx_core::ReadTrace;

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
        assert_eq!(log.close_day(ReadTrace::default()).duplicate_opens, 0);
        log.opened(open);
        log.opened(open);
        let reads = ReadTrace { undeclared_reads: 2, later_writes: 1, duplicate_opens: 0 };
        assert_eq!(log.close_day(reads), ReadTrace { undeclared_reads: 2, later_writes: 1, duplicate_opens: 1 });
        assert_eq!((log.total().duplicate_opens, log.total().undeclared_reads, log.clean()), (1, 2, false));
    }
}
