/// Percent in a whole, for derived values published in percent and the sex ratio at birth, per hundred females.
pub const PERCENT: f64 = 100.0;
/// The first age of the working-age band the derived shares count, under 15 being the first band.
pub const WORKING_AGE: i64 = 15;
/// The first age of the old-age band the derived shares count.
pub const OLD_AGE: i64 = 65;
/// The derived shares' age bands: under 15, 15 to 64, 65 and over.
pub const BANDS: usize = 3;
/// The columns of the household types' members: a partner, children, an older relative, another adult.
pub const PARTNER_COLUMN: usize = 0;
/// The children column.
pub const CHILDREN_COLUMN: usize = 1;
/// The older relative column.
pub const OLDER: usize = 2;
/// The other adult column.
pub const OTHER: usize = 3;
/// How many columns the members table has.
pub const MEMBER_COLUMNS: usize = 4;
/// The types the partner gap is cut into to give each whole year of it a chance: fine enough that no year's chance
/// moves by more than a type's share.
pub const GAP_TYPES: u16 = 10_000;
