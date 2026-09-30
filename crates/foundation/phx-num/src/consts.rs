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

/// Counts a large split's selection keeps a pass, one a digit of eight bits: they fit the stack, and a few passes
/// find the cut among any number of claimants.
pub const SELECT_BUCKETS: usize = 1 << 8;

/// Claimants whose remainders a split holds on the stack while it selects among them: 64 fit half a kilobyte.
pub const SMALL_SPLIT: usize = 64;

/// A split among this few selects on a stack of their size, which the common splits — a household's members, a joint
/// holding, a small estate — fit without clearing the larger one.
pub const SMALL_SPLIT_FEW: usize = 8;

/// A unit identity's width: 16.7 M units, room for the finished world's goods and capital classes at every zone, its
/// instruments and special units, with two years' growth.
pub const UNIT_ID_BITS: u32 = 24;
