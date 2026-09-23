/// Records the world's store reserves room for: sixteen million, beyond a decade's publications and reports, in
/// 512 MiB of address space, committed only as written.
pub const RECORD_ROWS: u32 = 1 << 24;
/// Events the store reserves room for, as records, in 768 MiB of address space.
pub const EVENT_ROWS: u32 = 1 << 24;
/// Words of the records' and events' arenas: 1 GiB of address space each, committed as written.
pub const STORE_ARENA_WORDS: u32 = 1 << 27;
/// One chunk in this many is read-traced each day, the chunk whose index is congruent to the day.
pub const TRACE_PERIOD: u32 = 64;
/// The world hash's key: any fixed value, so the same content always hashes the same.
pub const HASH_KEY: [u64; 2] = [0x5048_5820_574f_524c, 0x4420_4841_5348_2031];

/// The whole population, in the percent a setup's split is written in.
pub const WHOLE: u64 = 100;
