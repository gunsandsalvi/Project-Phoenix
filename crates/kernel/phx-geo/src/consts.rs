/// How finely the A* bound allows for rounding: a leg of at least a tile's 10 km loses at most half a metre, a part in
/// twenty thousand, so a path loses less than a part in ten thousand of its plane length.
pub const ROUNDING_PARTS: u64 = 10_000;

/// The improved fade's fifth-power coefficient, of `6t⁵ − 15t⁴ + 10t³` (Perlin 2002, "Improving noise").
pub const FADE_QUINTIC: f64 = 6.0;
/// The improved fade's fourth-power coefficient.
pub const FADE_QUARTIC: f64 = 15.0;
/// The improved fade's cube coefficient.
pub const FADE_CUBIC: f64 = 10.0;

/// A half: a tile's centre within it, and the map's centre across it.
pub const HALF: f64 = 0.5;

/// Parts per thousand, in which slopes and positions across the map are read.
pub const PER_MILLE: u64 = 1_000;

/// A whole, in percent.
pub const WHOLE_PERCENT: u64 = 100;
