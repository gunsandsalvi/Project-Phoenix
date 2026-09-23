/// Years of business days each country's calendar keeps as a bitset from its first year; days beyond are computed
/// from the rules, and the window moves forward a year at each year's start. Sixty-four years cover every contract
/// date a run of a few decades reads, at 3 KiB per country.
pub const CALENDAR_WINDOW_YEARS: i32 = 64;

/// A year without 29 February, against which a fixed holiday is checked to exist every year.
pub const COMMON_YEAR: i32 = 2001;
/// A month holds each weekday at least four times, so the fourth exists every year and the fifth may not.
pub const MAX_NTH: i8 = 4;

/// Gregorian computus (Meeus, Jones and Butcher, 1876): the golden-number cycle.
pub const EASTER_GOLDEN: i32 = 19;
/// Years per century in the computus.
pub const EASTER_CENTURY: i32 = 100;
/// The computus' divisor of four, for leap years and the century's quarters.
pub const EASTER_FOUR: i32 = 4;
/// The lunar correction's numerator offset and divisor: (b + 8) / 25.
pub const EASTER_F: [i32; 2] = [8, 25];
/// The solar correction: (b − f + 1) / 3.
pub const EASTER_G_DIV: i32 = 3;
/// The epact: (19a + b − d − g + 15) mod 30.
pub const EASTER_H: [i32; 2] = [15, 30];
/// The weekday correction: (32 + 2e + 2i − h − k) mod 7.
pub const EASTER_L: [i32; 2] = [32, 7];
/// The final correction: (a + 11h + 22l) / 451.
pub const EASTER_M: [i32; 3] = [11, 22, 451];
/// The month and day: month = (h + l − 7m + 114) / 31, day = (h + l − 7m + 114) mod 31 + 1.
pub const EASTER_MONTH: [i32; 3] = [7, 114, 31];

/// Days of the day counts' conventional years.
pub const DAYS_360: i64 = 360;
/// See `DAYS_360`.
pub const DAYS_365: i64 = 365;
/// See `DAYS_360`.
pub const DAYS_366: i64 = 366;
/// A 30/360 convention's month.
pub const DAYS_30: i64 = 30;
/// The day of the month a 30/360 convention counts its month's last day as.
pub const DAY_30: u8 = 30;
/// A 30/360 convention's day of the month that counts as its thirtieth.
pub const DAY_31: u8 = 31;
/// Months in a year.
pub const MONTHS: i32 = 12;
/// Days in a week, for week periods.
pub const DAYS_PER_WEEK: u16 = 7;
/// The shortest month, so a phase within any month period is shorter than every instance of it.
pub const SHORTEST_MONTH_DAYS: u16 = 28;

/// Business days per word of a country's bitset.
pub const CALENDAR_WORD_BITS: i64 = 64;
/// Buckets of the agenda wheel's day level, and days of its block level's buckets: 1 024 days, 2.8 years.
pub const WHEEL_BITS: u32 = 10;
/// See `WHEEL_BITS`.
pub const WHEEL_BUCKETS: usize = 1 << WHEEL_BITS;
/// Reasons a table's rows may be on the agenda for; each is a `u32` day, so 16 fill one cache line.
pub const MAX_REASONS: usize = 16;

/// Parts per million, the unit of every share of a whole: a type set's shares sum to exactly this.
pub const PPM: u32 = 1_000_000;
/// Decimal places of a rate's fraction, as `Rate` holds it.
pub const RATE_EXP: u8 = 12;
/// Decimal places of a distribution's parameters in data: twelve keep any parameter written to six significant
/// figures exact across the magnitudes data uses.
pub const PARAM_EXP: u8 = 12;
/// `10^PARAM_EXP`, by which a parameter's integer is read as a number.
pub const PARAM_SCALE_F64: f64 = 1e12;
/// The base of decimal places.
pub const DECIMAL_RADIX_F64: f64 = 10.0;

/// The base of the register's decimal places, as an integer.
pub const DECIMAL_RADIX: i64 = 10;
/// Steps of a series, continued fraction or bisection before it stops: each converges to a double's precision in
/// well under a hundred steps for the parameters data declares, and a bisection halves its interval to below the
/// precision of any double within 200.
pub const NUMERIC_STEPS: u32 = 200;
/// The value a continued fraction's vanishing denominator is held at, far below any term it meets (Lentz's method).
pub const CF_TINY: f64 = 1e-300;
/// Months of a year and of a quarter, as primitives' periods state them.
pub const MONTHS_PER_YEAR: u16 = 12;
/// See `MONTHS_PER_YEAR`.
pub const MONTHS_PER_QUARTER: u16 = 3;
/// The kernel map's hash seed: any fixed value, since the map is read whole only sorted and its hash reaches no
/// outcome.
pub const MAP_SEED: u64 = 0x5048_5820_4d41_5021;
/// The draw address's ordinal of an opening phase: beyond every sub-step of a day, so opening draws share no address
/// with a day's.
pub const OPENING_ORDINAL_BASE: u8 = 64;
/// The draw address's ordinal of a keyed stream, beyond every day's and opening's ordinal.
pub const KEYED_ORDINAL: u8 = u8::MAX;

/// A whole, in percent, for the profile's percentages.
pub const PERCENT_F64: f64 = 100.0;
