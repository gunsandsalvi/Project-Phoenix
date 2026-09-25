/// Percent in a whole, for derived values published in percent.
pub const PERCENT: f64 = 100.0;
/// People in a million, for the firms the promotion rank admits per million people.
pub const MILLION: u64 = 1_000_000;
/// The opening draws' purposes within a country's stratum: the firms' sizes and their sites.
pub const SIZES: u32 = 0;
pub const SITES: u32 = 1;
pub const PURPOSES: u32 = 2;
/// The small firms' opening draws' purposes within a country's stratum: their regions, banks and sizes, and their
/// deposits' and debt's shares.
pub const CLASSES: u32 = 0;
pub const AMOUNTS: u32 = 1;
pub const SMALL_PURPOSES: u32 = 2;
/// The persons a small firm can employ: far beyond the smallest firm the individuals' rank admits, which a world whose
/// rank reaches past it stops at as a capacity reached.
pub const MOST_SMALL_EMPLOYED: u32 = 1 << 16;
