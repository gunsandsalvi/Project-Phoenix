/// The most dues one contract can fall due for on one day: one per leg, and a contract composes a handful of legs, so
/// a buffer of this size on the stack holds any day's dues without allocating.
pub const DUES_PER_DAY: usize = 16;

/// Shards of the terms interner: one per worker the pool can run on the phone's eight fast and medium cores, so
/// parallel interning never contends for a shard.
pub const TERMS_SHARDS: u32 = 8;

/// Pools of holder-list blocks: one per worker the pool can run on the phone's eight fast and medium cores, so
/// parallel updates of holder lists never share a pool.
pub const HOLDER_SHARDS: u32 = 8;

/// One holder in this many has its run read in full against its rows each day, the holders whose slot is congruent
/// to the day, so every holder is read within this many days.
pub const RUN_SAMPLE_PERIOD: u32 = 64;
/// The optional words a relationship row can carry: its balance, its pending amount and its amount.
pub const ROW_OPTIONAL_WORDS: usize = 3;

/// The synthetic settlement's books: each holder's deposit, in the smallest units.
pub const SYNTHETIC_DEPOSIT: i64 = 100_000_000;
/// Each loan row's balance, in the smallest units.
pub const SYNTHETIC_LOAN: i64 = 1_000_000;
/// The loans' rate a year: twelve per cent, in the rate's raw scale.
pub const SYNTHETIC_RATE: i64 = 120_000_000_000;
/// The rows a chunk holds.
pub const SYNTHETIC_ROWS_PER_CHUNK: u32 = 1 << 12;
/// The calendar's epoch year.
pub const SYNTHETIC_EPOCH_YEAR: i32 = 1950;
/// The calendar's first year.
pub const SYNTHETIC_WINDOW_YEAR: i32 = 2020;
/// The business days of a week, each with its own weekly rows.
pub const WEEKDAYS: u32 = 5;
/// The months of a year.
pub const MONTHS_A_YEAR: u8 = 12;
