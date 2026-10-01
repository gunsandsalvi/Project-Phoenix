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

/// A party reference's generation takes the 24 bits between its kind's byte and its slot's word.
pub const GENERATION_BITS: u32 = 24;
/// The generation's place in a party reference, above the slot's 32 bits.
pub const GENERATION_SHIFT: u32 = 32;
/// The kind's place in a party reference, its top byte.
pub const KIND_SHIFT: u32 = 56;
/// A party key's kind takes its top five bits, room for 32 kinds of party.
pub const KEY_KIND_BITS: u32 = 5;

/// The last kind a key holds, no table's: the side a transformation's flow names in place of a counterparty.
pub const NATURE_KIND: u8 = 31;
/// A party key's slot takes the other 27 bits: 134 million parties of one kind.
pub const KEY_SLOT_BITS: u32 = 27;

/// A contract link's family code, in bits: 255 families, the last code holdings'.
pub const FAMILY_BITS: u32 = 8;
/// A contract link's slot in its family's table, in bits: 16.7 M rows, past the largest family at the design point with
/// two years' growth.
pub const FAMILY_SLOT_BITS: u32 = 24;
/// The family code reserved for holdings, the highest the code holds.
pub const HOLDINGS_FAMILY: u8 = u8::MAX;
