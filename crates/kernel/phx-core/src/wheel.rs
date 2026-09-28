//! The due wheel: every dated contract waits in the bucket of its next due day, so a day takes the contracts due on
//! it and nothing else. A contract whose due moves or that closes is not taken out: its stale entry is skipped by its
//! reader, which checks the contract's own next due against the day.

use phx_id::Day;
use phx_num::violation;

/// Buckets for the days of a horizon, reused round the wheel, and the entries due beyond it, which enter a bucket as
/// the wheel comes within reach of their day.
#[derive(Debug)]
pub struct DueWheel {
    first: Day,
    buckets: Vec<Vec<u32>>,
    far: Vec<(Day, u32)>,
    /// The earliest day in `far`, so a day scans it only when an entry comes within reach.
    far_first: Option<Day>,
    /// The sort's pairs and scratch, kept so a day's take allocates nothing once the heaviest has sized them.
    pairs: Vec<(u32, u32)>,
    scratch: Vec<(u32, u32)>,
}

impl DueWheel {
    /// A wheel whose first day is `first`, with a bucket for each of `horizon` days.
    #[must_use]
    pub fn new(first: Day, horizon: u32) -> DueWheel {
        if horizon == 0 {
            violation!(clause = "TIME.6", "a due wheel with no days");
        }
        DueWheel {
            first,
            buckets: (0..horizon).map(|_| Vec::new()).collect(),
            far: Vec::new(),
            far_first: None,
            pairs: Vec::new(),
            scratch: Vec::new(),
        }
    }

    pub fn first(&self) -> Day {
        self.first
    }

    fn horizon(&self) -> u32 {
        u32::try_from(self.buckets.len()).unwrap_or(u32::MAX)
    }

    fn bucket(&mut self, day: Day) -> &mut Vec<u32> {
        let h = self.horizon();
        let i = usize::try_from(day.get() % h).unwrap_or(0);
        let Some(b) = self.buckets.get_mut(i) else {
            violation!(clause = "TIME.6", "a due wheel bucket outside the wheel", day = day.get());
        };
        b
    }

    /// Puts a contract in the bucket of its due day, on or after the wheel's first day.
    pub fn schedule(&mut self, edge: u32, day: Day) {
        if day < self.first {
            violation!(
                clause = "TIME.3",
                "a due scheduled before the wheel's day",
                day = day.get(),
                first = self.first.get()
            );
        }
        if day.get() - self.first.get() < self.horizon() {
            self.bucket(day).push(edge);
        } else {
            self.far.push((day, edge));
            if self.far_first.is_none_or(|f| day < f) {
                self.far_first = Some(day);
            }
        }
    }

    /// Takes the contracts due on the wheel's first day into `out`, sorted — by radix on the pool when one is given —
    /// and turns the wheel to the next day.
    pub fn take(&mut self, day: Day, out: &mut Vec<u32>, pool: Option<&phx_exec::Pool>) {
        if day != self.first {
            violation!(clause = "TIME.6", "a due wheel turned out of order", day = day.get(), first = self.first.get());
        }
        out.clear();
        std::mem::swap(out, self.bucket(day));
        if pool.is_some() {
            self.pairs.clear();
            self.pairs.extend(out.iter().map(|e| (*e, 0)));
            phx_exec::radix_sort(pool, &mut self.pairs, &mut self.scratch);
            out.clear();
            out.extend(self.pairs.iter().map(|(e, _)| *e));
        } else {
            out.sort_unstable();
        }
        self.first = day.succ();
        let reach = self.first.get() + self.horizon() - 1;
        if self.far_first.is_some_and(|f| f.get() <= reach) {
            let far = std::mem::take(&mut self.far);
            self.far_first = None;
            for (d, edge) in far {
                self.schedule(edge, d);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;

    use super::DueWheel;

    #[test]
    fn wheel_moves_edge_to_next_due() {
        let mut w = DueWheel::new(Day::new(10), 4);
        w.schedule(7, Day::new(11));
        w.schedule(3, Day::new(11));
        w.schedule(9, Day::new(30));
        let mut out = Vec::new();
        w.take(Day::new(10), &mut out, None);
        assert!(out.is_empty());
        w.take(Day::new(11), &mut out, None);
        assert_eq!(out, vec![3, 7], "sorted, whatever the order scheduled");
        // Each due contract moves to its next date, as a monthly one does.
        for e in out.clone() {
            w.schedule(e, Day::new(14));
        }
        for d in 12..14 {
            w.take(Day::new(d), &mut out, None);
            assert!(out.is_empty());
        }
        w.take(Day::new(14), &mut out, None);
        assert_eq!(out, vec![3, 7], "the bucket reused round the wheel");
        for d in 15..30 {
            w.take(Day::new(d), &mut out, None);
            assert!(out.is_empty(), "day {d}");
        }
        w.take(Day::new(30), &mut out, None);
        assert_eq!(out, vec![9], "a due beyond the horizon enters when the wheel reaches it");
    }
}
