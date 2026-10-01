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

/// Metres in a kilometre, the unit a climate table reads the distance to the sea in.
pub const METRES_PER_KM: u64 = 1_000;

/// Months in a year, the columns of a climate table.
pub const MONTHS: i64 = 12;

/// Tenths in a unit: temperature, rain and wind are recorded in tenths of their units.
pub const TENTHS: f64 = 10.0;

/// Parts per thousand, in which sunshine is recorded as its clear-sky index.
pub const PER_MILLE_F64: f64 = 1_000.0;

/// Months in a year, as a date numbers them.
pub const MONTHS_U8: u8 = 12;

/// Attempts at a map before the generator gives up: a map that fails its conditions this often is a finding about the
/// conditions, not a run to keep drawing.
pub const MAP_MAX_ATTEMPTS: u64 = 64;
/// A segment row's flags: closed until its day, removed, held as a named unit.
pub const SEGMENT_CLOSED: u16 = 1;
/// A removed segment.
pub const SEGMENT_REMOVED: u16 = 1 << 1;
/// A segment held as a named unit.
pub const SEGMENT_HELD: u16 = 1 << 2;
/// Why land is held apart from its tile, as a parcel's flags: owned apart, leased apart.
pub const PARCEL_OWNED_APART: u16 = 1;
pub const PARCEL_LEASED_APART: u16 = 1 << 1;
/// A tile's parcels read through in order rather than searched: a few cache lines, about a tile's parcels at the design
/// point.
pub const PARCEL_SHORT_RUN: usize = 32;
