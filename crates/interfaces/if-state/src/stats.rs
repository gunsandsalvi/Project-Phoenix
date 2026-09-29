//! The statistics agency's kinds: the series it publishes, each country's calendar, samples and revisions, and the
//! rules its indices and rates are computed by.

use phx_core::{OpeningCountry, Register};
use phx_id::Day;

pub use crate::consts::{
    ACCOUNTS, CPI, HOLDER_CLASSES, LABOUR_FORCE, LABOUR_STATES, LIFE_ENTRY, LIFE_TABLE, MONEY, NAMES, PLACES, PPI,
    SERIES,
};

/// A published statistic: its series, country and period (months from the epoch's), its vintage (nought the first
/// release, one its revision), the day it was published, and its values at the series' places.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Release {
    pub series: u8,
    pub country: u8,
    pub period: u32,
    pub vintage: u8,
    pub published: Day,
    pub values: Vec<i64>,
}

/// When and from how much a series is published: the business day of the month it is released on, the months after
/// its period's end that month is, and the share of its records or households the agency samples.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Schedule {
    pub day: u32,
    pub lag: u32,
    pub sample: f64,
}

/// A country's statistics law: each series' schedule, the share of the sample's returns in by a first release, and
/// the months after it the revision, from every return, comes.
#[derive(Clone, Debug, PartialEq, phx_macros::Saved)]
pub struct StaLaw {
    pub series: Vec<Schedule>,
    pub early: f64,
    pub revision: u32,
}

/// The release of a series and country a reader may read on a day, among those published.
pub type Latest = for<'a> fn(&'a [Release], (u8, u8), Day) -> Option<&'a Release>;

/// The statistics agency: each country's law; the rate of events over the exposure they happened in; the release a
/// reader may read on a day; the market kinds its consumer and producer indices sample; the age classes it reports
/// by; and the streams its samples and their returns are drawn from.
#[derive(Clone, Copy, Debug)]
pub struct StaKind {
    pub law: fn(&Register, &OpeningCountry) -> Result<StaLaw, String>,
    pub rate: fn(f64, f64) -> Option<f64>,
    pub latest: Latest,
    pub consumer: &'static [&'static str],
    pub producer: &'static [&'static str],
    pub classes: &'static str,
    pub sample: phx_core::StreamDecl,
    pub returns: phx_core::StreamDecl,
}

/// One constituent of an index's link: its price in the period before and in this one, and its weight, its share of
/// the period before's sampled spending.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Priced {
    pub before: f64,
    pub now: f64,
    pub weight: f64,
}

/// The price indices' publisher: each country's base level, and the chain's link over the constituents priced in
/// both periods.
#[derive(Clone, Copy, Debug)]
pub struct IndexKind {
    pub base: fn(&Register, &OpeningCountry) -> Result<f64, String>,
    pub link: fn(&[Priced]) -> Option<f64>,
}
