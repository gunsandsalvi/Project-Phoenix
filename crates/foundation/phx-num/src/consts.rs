/// Rates are fractions scaled by 10^12, so a basis point of a basis point is still eight digits above the unit.
pub const RATE_SCALE: i128 = 1_000_000_000_000;

/// The column marker of an absent value: the one `i64` no present value may take, so absence is never a number.
pub const ABSENT_I64: i64 = i64::MIN;

/// A price exponent above 18 would make 10^exp overflow `i64`, the width of a raw price.
pub const MAX_PRICE_EXP: u8 = 18;

/// Prices, positions and rates are decimal, so their scales are powers of ten.
pub const DECIMAL_BASE: i128 = 10;

/// A violation carries at most eight keys, enough to name the parties, amounts and units of any impossible state.
pub const MAX_VIOLATION_KEYS: usize = 8;

/// Ten as a float, for scaling a fixed-point value to and from `f64` inside pure functions.
pub const DECIMAL_BASE_F64: f64 = 10.0;
