//! The places of an employment contract's class (`Terms::class`), each a whole value: contracts differing in any are
//! on different lines.

/// The occupation family, by its ISCO-08 major group.
pub const OCCUPATION: usize = 0;
/// The skill level the job is held at, by ISCO-08's four.
pub const SKILL: usize = 1;
/// The hours a week the contract buys.
pub const HOURS: usize = 2;
/// The days of notice the employer gives before a layoff takes effect.
pub const NOTICE: usize = 3;
/// The days of wages owed as severance for each whole year of service.
pub const SEVERANCE: usize = 4;
/// The region the job is in.
pub const REGION: usize = 5;
/// The first year of the band the contracts began in.
pub const BAND: usize = 6;
/// The places a class holds.
pub const PLACES: usize = 7;

/// A person's labour state when not searching, whether in work or out of the labour force. Employment is a person's
/// attachments on employment lines, never its state.
pub const NOT_SEARCHING: u32 = 0;
/// A person's labour state when searching for work, in work or out of it.
pub const SEARCHING: u32 = 1;
/// A person's labour state once retired.
pub const RETIRED: u32 = 2;
/// The states a person can be in.
pub const STATES: u32 = 3;
/// The occupation value of a person who has never worked, after the ten major groups.
pub const NO_OCCUPATION: u32 = 10;
/// The occupation values a person can hold.
pub const OCCUPATIONS: u32 = 11;
/// The wage points a person's last wage can be recorded at, the last for none.
pub const WAGE_POINTS: u32 = 128;
/// The last wage point of a person never counted in work.
pub const NO_POINT: u32 = WAGE_POINTS - 1;
