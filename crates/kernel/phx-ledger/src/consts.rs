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
