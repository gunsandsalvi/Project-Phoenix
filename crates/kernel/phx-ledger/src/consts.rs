/// The most dues one contract can fall due for on one day: one per leg, and a contract composes a handful of legs, so
/// a buffer of this size on the stack holds any day's dues without allocating.
pub const DUES_PER_DAY: usize = 16;

/// The months of a year.
pub const MONTHS_A_YEAR: u8 = 12;
