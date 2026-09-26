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
/// The filed accounts' opening draws' purposes within a country's stratum: the firms' products, their required
/// returns and their methods.
pub const PRODUCTS_PURPOSE: u32 = 0;
/// The draws of the firms' required returns.
pub const RETURNS: u32 = 1;
/// The draws of the firms' methods.
pub const METHODS: u32 = 2;
/// The purposes a country's filed-accounts stratum holds.
pub const FILED_PURPOSES: u32 = 3;
/// Days in a week, over which the law's weekly hours are worked.
pub const DAYS_A_WEEK: f64 = 7.0;
/// Days in a year, on average over the calendar's cycle of leap years.
pub const DAYS_A_YEAR: f64 = 365.25;
/// Months in a year.
pub const MONTHS_A_YEAR: f64 = 12.0;
/// A fact held at six places, as the markup and the required return are.
pub const FIXED_SCALE: f64 = 1_000_000.0;
/// A way's yield is stated in parts per million of what it starts.
pub const PPM: i64 = 1_000_000;
/// A way's inputs are stated a unit to nine places.
pub const PER_UNIT_SCALE: i64 = 1_000_000_000;
