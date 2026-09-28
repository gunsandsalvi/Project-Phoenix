/// Percent in a whole, for derived values published in percent.
pub const PERCENT: f64 = 100.0;
/// The largest banks whose share of assets the first published concentration counts.
pub const TOP_THREE: u32 = 3;
/// The largest banks whose share the second published concentration counts.
pub const TOP_FIVE: u32 = 5;
/// Halvings of the Zipf exponent's interval when fitting it: enough for the exponent to a double's precision.
pub const FIT_STEPS: u32 = 64;
/// The widest Zipf exponent searched: beyond it the three largest banks hold everything to a double's precision.
pub const EXPONENT_CEILING: f64 = 64.0;
/// The most banks a country's concentration may imply before the fit is refused as no Zipf law.
pub const MOST_BANKS: u32 = 1 << 16;
/// Parts of a whole a bank's share is weighed in for apportionment.
pub const SHARE_PARTS: f64 = 1_000_000.0;
/// A rate of one, whole, in phx-num's rate scale.
pub const RATE_ONE: f64 = 1_000_000_000_000.0;
/// The opening draws' purpose of the banks' sites within a country's stratum.
pub const SITES: u32 = 0;
/// How many purposes a country's stratum holds.
pub const PURPOSES: u32 = 3;
/// Parts of a household's wealth or income, as a multiple of its country's median, it is weighed in for its share of
/// the households' balances.
pub const WEALTH_PARTS: f64 = 1_000_000.0;
