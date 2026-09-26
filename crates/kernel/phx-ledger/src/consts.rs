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
/// The rows a chunk holds: enough holders to fill much of an individual's chunk arena with their dated rows, so the
/// full load's million holders reserve tens of arenas, not hundreds.
pub const SYNTHETIC_ROWS_PER_CHUNK: u32 = 1 << 15;
/// The calendar's epoch year.
pub const SYNTHETIC_EPOCH_YEAR: i32 = 1950;
/// The calendar's first year.
pub const SYNTHETIC_WINDOW_YEAR: i32 = 2020;
/// The business days of a week, each with its own weekly rows.
pub const WEEKDAYS: u32 = 5;
/// The months of a year.
pub const MONTHS_A_YEAR: u8 = 12;
/// Rows on means-of-payment lines a holder's summary keeps: a household's deposit and banknotes, a bank's reserves
/// and the deposits it issues; a holder with more has its rows read.
pub const MONEY_ROWS_KEPT: usize = 4;
/// The shards 7a's heads are read in: fixed, never the number of workers, so the stream's result is the same on any
/// pool; many to a worker, so the pool balances shards of uneven work.
pub const STREAM_SHARDS: usize = 64;
/// The shards of 7a read at once, so no more than a wave's payments wait to be booked; fixed, as the shards are.
pub const STREAM_WAVE: usize = 8;
/// The shards 7c routes the day's payments in, read a wave at a time: finer than 7a's, as a route is wider than the
/// payment it moves, so a wave holds a thirty-second of the day.
pub const ROUTE_SHARDS: usize = 256;
/// The shards the day's records are dealt to, a run of slots at a time, so the pool folds each shard's bookings on
/// one worker; fixed, never the number of workers, so the records are the same on any pool.
pub const RECORD_SHARDS: usize = 64;
/// The bits of a run of slots dealt to one records shard: runs of a thousand slots keep a shard's records dense.
pub const RECORD_RUN_BITS: u32 = 10;

/// The words of a transformation's leg in a handler's intent: its product and grade, its quantity, its source's tag
/// and value, and its cost.
pub const LEG_WORDS: usize = 5;
/// A leg's source tagged as the way that made it.
pub const WAY: u64 = 0;
/// A leg's source tagged as the deposit it was taken from.
pub const DEPOSIT: u64 = 1;
/// A leg's source tagged as the purchase that used it up.
pub const PURCHASE: u64 = 2;
/// A leg's source tagged as spoiling in stock.
pub const SPOILAGE: u64 = 3;
/// A leg's source tagged as the hazard that destroyed it.
pub const HAZARD: u64 = 4;
/// A leg's source tagged as the chain it wore along.
pub const WEAR: u64 = 5;
/// A leg's source tagged as the shipment that carried it between places.
pub const CARRIED: u64 = 6;
