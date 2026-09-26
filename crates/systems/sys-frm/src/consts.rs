/// Percent in a whole, for derived values published in percent.
pub const PERCENT: f64 = 100.0;
/// People in a million, for the firms the promotion rank admits per million people.
pub const MILLION: u64 = 1_000_000;
/// The opening draws' purposes within a country's stratum: the firms' sizes, their sites and their industries.
pub const SIZES: u32 = 0;
/// The draws of the firms' sites.
pub const SITES: u32 = 1;
/// The draws of the firms' industries.
pub const INDUSTRIES: u32 = 2;
/// The purposes a country's stratum holds.
pub const PURPOSES: u32 = 3;
/// The small firms' opening draws' purposes within a country's stratum: their regions, banks and sizes, their
/// deposits' and debt's shares, and their industries.
pub const CLASSES: u32 = 0;
/// The draws of the small firms' shares of deposits and debt.
pub const AMOUNTS: u32 = 1;
/// The draws of the small firms' industries.
pub const SMALL_INDUSTRIES: u32 = 2;
/// The purposes a country's small-firm stratum holds.
pub const SMALL_PURPOSES: u32 = 3;
/// The persons a small firm can employ: far beyond the smallest firm the individuals' rank admits, which a world whose
/// rank reaches past it stops at as a capacity reached.
pub const MOST_SMALL_EMPLOYED: u32 = 1 << 16;
/// A decade's ratio, from one power of ten to the next, over which a trade's price points repeat.
pub const DECADE: f64 = 10.0;
/// A decade's ratio as a whole number, for points moved between decades exactly.
pub const TEN: i32 = 10;
