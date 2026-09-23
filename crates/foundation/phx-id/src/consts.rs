/// Party identities stay below 2^60, the identity bits a random draw's subject carries.
pub const PARTY_ID_BITS: u32 = 60;

/// A system's code is two to four capital letters.
pub const SYSTEM_CODE_MAX: usize = 4;

/// The Gregorian calendar repeats every 400 years (Hinnant, "chrono-Compatible Low-Level Date Algorithms").
pub const YEARS_PER_ERA: i64 = 400;
/// Days in a 400-year era: 400·365 + 97 leap days.
pub const DAYS_PER_ERA: i64 = 146_097;
/// Days in a common year.
pub const DAYS_PER_YEAR: i64 = 365;
/// A leap year every fourth year...
pub const LEAP_EVERY: i64 = 4;
/// ...except every hundredth...
pub const CENTURY: i64 = 100;
/// Days in four common years: dividing by it counts the leap days already passed in an era.
pub const DAYS_PER_LEAP_CYCLE: i64 = 1_460;
/// Days in a century whose first year is common: dividing by it counts the century years that skip their leap day.
pub const DAYS_PER_CENTURY: i64 = 36_524;
/// Days from 0000-03-01, where Hinnant's shifted years begin, to 1970-01-01, serial zero.
pub const SERIAL_SHIFT: i64 = 719_468;
/// Hinnant's month mapping: day-of-year = (153·m' + 2)/5 for the March-based month m'.
pub const MONTH_SPAN_NUM: i64 = 153;
/// See `MONTH_SPAN_NUM`.
pub const MONTH_SPAN_DEN: i64 = 5;
/// March is the first month of Hinnant's shifted year, so February's leap day falls at its end.
pub const MARCH: i64 = 3;
/// A March-based month below 10 is March to December; from 10 it is January or February of the next year.
pub const MARCH_BASED_JANUARY: i64 = 10;
/// Months in a year.
pub const MONTHS: i64 = 12;
/// The longest month.
pub const LONGEST_MONTH: u8 = 31;
/// The months of 30 days: April, June, September, November.
pub const THIRTY_DAY_MONTHS: [u8; 4] = [4, 6, 9, 11];
/// February's days in a leap year.
pub const LEAP_FEBRUARY: u8 = 29;
/// February's days in a common year.
pub const COMMON_FEBRUARY: u8 = 28;
/// The months of 30 days hold one day fewer than the longest.
pub const SHORT_MONTH: u8 = 30;
/// Days in a week.
pub const DAYS_PER_WEEK: i64 = 7;
/// 1970-01-01, serial zero, was a Thursday, day three of a week counted from Monday as day zero.
pub const SERIAL_ZERO_WEEKDAY: i64 = 3;
