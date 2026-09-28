/// Points each income shock's distribution is cut into, equally likely: enough for the buffer's target to three digits.
pub const SHOCK_POINTS: usize = 7;
/// Points on the grid of assets the consumption rule is solved over.
pub const GRID_POINTS: usize = 80;
/// The most assets the grid reaches, in years of permanent income: far beyond any buffer the rule targets.
pub const GRID_TOP: f64 = 40.0;
/// The change in consumption between iterations below which the rule has converged, in years of income.
pub const TOLERANCE: f64 = 1e-9;
/// The most iterations the rule's solution may take before the model is refused as not converging.
pub const MOST_ITERATIONS: usize = 5_000;
/// The least cash on hand the target is searched above, in years of income.
pub const TARGET_LOW: f64 = 1e-6;
/// Halvings of the target's interval: enough for a double's precision.
pub const HALVINGS: usize = 100;
/// The step either side of the target over which the propensity to consume is read, in years of income.
pub const SLOPE_STEP: f64 = 1e-4;
/// Halvings of the unit interval when inverting the normal distribution.
pub const INVERSE_STEPS: usize = 80;
/// The widest standard normal value searched when inverting its distribution.
pub const NORMAL_EDGE: f64 = 12.0;
/// The standard normal's mass below its mean.
pub const HALF: f64 = 0.5;
/// Days of a year, over which a year's income is spent between decisions.
pub const DAYS_A_YEAR: f64 = 365.2425;
/// Months of a year, to which a month's income is grossed.
pub const MONTHS_A_YEAR: f64 = 12.0;
/// See `MONTHS_A_YEAR`, for counting months.
pub const MONTHS_A_YEAR_COUNT: i64 = 12;
/// Parts of a whole a budget share is written in.
pub const SHARE_PARTS: f64 = 1_000_000.0;
