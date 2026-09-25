//! Liveness and drift, read from the observer's records alone: whether a read never stops rising, whether the reads
//! stood still from one day to the next, and how far each opening distribution has moved from the world's own.

use phx_id::Day;
use phx_macros::clause;
use phx_num::Missing;

use crate::reads::Series;
use crate::view::View;

/// Whether a series rose on every day it was read, over at least two rises: a quantity with nothing yet holding it.
#[clause("N2")]
#[must_use]
pub fn rises_throughout(values: &[(Day, i128)]) -> bool {
    values.len() > 2 && values.windows(2).all(|w| matches!(w, [(_, a), (_, b)] if b > a))
}

/// The first day on which every series read held the value it held the day before, where one did: the world at a
/// fixed point, if nothing moves from then on.
#[clause("N2")]
pub fn still_from(series: &[Series]) -> Missing<Day> {
    let mut days: Vec<Day> = series.iter().flat_map(|s| s.values.iter().map(|(d, _)| *d)).collect();
    days.sort_unstable();
    days.dedup();
    let value = |s: &Series, d: Day| s.values.iter().find(|(at, _)| *at == d).map(|(_, v)| *v);
    let mut still = Missing::Absent;
    for pair in days.windows(2) {
        let [before, day] = pair else { continue };
        let unmoved = series.iter().all(|s| match (value(s, *before), value(s, *day)) {
            (Some(a), Some(b)) => a == b,
            _ => true,
        });
        still = match (unmoved, still) {
            (true, Missing::Absent) => Missing::Present(*day),
            (true, at) => at,
            (false, _) => Missing::Absent,
        };
    }
    still
}

/// One opening distribution's distance from the world's own at a later close.
#[derive(Clone, Debug, PartialEq)]
pub struct Drift {
    pub id: String,
    pub day: Day,
    pub distance: Missing<f64>,
}

/// Each histogram of the opening's view, its distance from the same histogram at a later view.
#[clause("GEN.8")]
#[must_use]
pub fn drift(opening: &View, later: &View) -> Vec<Drift> {
    opening
        .histograms
        .iter()
        .map(|(id, h)| {
            let distance = later
                .histograms
                .iter()
                .find(|(other, _)| other == id)
                .map_or(Missing::Absent, |(_, now)| h.distance(now));
            Drift { id: id.clone(), day: later.day, distance }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;

    use super::{rises_throughout, still_from};
    use crate::reads::Series;

    fn series(values: &[i128]) -> Series {
        let values = values.iter().zip(1_u32..).map(|(v, d)| (Day::new(d), *v)).collect();
        Series { id: "s".to_owned(), unit: "u".to_owned(), values }
    }

    #[test]
    fn a_read_rising_every_day_is_found() {
        assert!(rises_throughout(&series(&[1, 2, 5, 9]).values));
        assert!(!rises_throughout(&series(&[1, 2, 2, 9]).values), "a day it held still");
        assert!(!rises_throughout(&series(&[1, 2]).values), "one rise is no trend");
    }

    #[test]
    fn the_reads_standing_still_to_the_end_are_found() {
        let moving = [series(&[1, 2, 2, 2]), series(&[5, 5, 5, 5])];
        assert_eq!(still_from(&moving), Missing::Present(Day::new(3)), "unmoved from the third day on");
        let alive = [series(&[1, 1, 1, 2]), series(&[5, 5, 5, 5])];
        assert_eq!(still_from(&alive), Missing::Absent, "one read moved on the last day");
    }
}
