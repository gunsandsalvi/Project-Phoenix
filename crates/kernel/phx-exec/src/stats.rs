//! The stores' own statistics as the counters read them: each store sampled at every simulated month's end, and its
//! growth over the whole years the samples cover.

pub use phx_store::StoreStats;

use crate::consts::MONTHS_A_YEAR;

/// One store's statistics at one month's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sample {
    pub rows_live: u64,
    pub rows_ever: u64,
    pub bytes: u64,
}

impl Sample {
    #[must_use]
    pub fn of(store: &dyn StoreStats) -> Sample {
        Sample { rows_live: store.rows_live(), rows_ever: store.rows_ever(), bytes: store.bytes() }
    }
}

/// Rows a store gains a simulated year, from its samples at each month's end, the first at the run's start: read only
/// over whole years, since a month's or a first day's growth is not a year's; none before a year is sampled.
#[must_use]
pub fn growth_per_year(samples: &[u64]) -> Option<i128> {
    let years = samples.len().checked_sub(1)? / MONTHS_A_YEAR;
    let last = samples.get(years.checked_mul(MONTHS_A_YEAR)?)?;
    let first = samples.first()?;
    let years = i128::try_from(years).ok().filter(|y| *y > 0)?;
    Some((i128::from(*last) - i128::from(*first)) / years)
}

#[cfg(test)]
mod tests {
    use super::growth_per_year;

    #[test]
    fn growth_per_year_from_monthly_samples() {
        let months = |n: u64| (0..=n).map(|m| 1_000 + 50 * m).collect::<Vec<_>>();
        assert_eq!(growth_per_year(&months(11)), None, "eleven months are not a year");
        assert_eq!(growth_per_year(&months(12)), Some(600));
        assert_eq!(growth_per_year(&months(30)), Some(600), "the months past the last whole year are not read");
        let shrinking: Vec<u64> = (0..=24).map(|m| 10_000 - 10 * m).collect();
        assert_eq!(growth_per_year(&shrinking), Some(-120));
        assert_eq!(growth_per_year(&[]), None);
    }
}
