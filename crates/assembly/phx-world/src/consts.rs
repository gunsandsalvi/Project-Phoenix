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
/// A whole, in the hundredths a share of two decimals is written in.
pub const SHARE_WHOLE: u64 = 100;
/// The widening sweeps a kind may take in one day while its cells exceed its budget: the heavy day's budget holds two.
pub const SWEEPS_PER_DAY: u32 = 2;
/// Bits of a promoted member's draw subject that number the members rising from one cell together, below its origin's
/// identity: room for a million at once.
pub const PROMOTION_SEQ_BITS: u32 = 20;

/// Rows each kind table of individuals reserves: room for a kind's individuals across the three countries, beyond the
/// firms the promotion rank admits, in address space committed only as rows are written.
pub const KIND_ROWS: u32 = 1 << 18;
/// Rows of a kind table per chunk, as the population's tables chunk theirs.
pub const KIND_ROWS_PER_CHUNK: u32 = 1 << 12;
/// Rows each population kind's cell table reserves: room for several times the cells the design point carries, in
/// address space committed only as rows are written.
pub const CELL_ROWS: u32 = 1 << 23;
/// Rows of a cell table per chunk.
pub const CELL_ROWS_PER_CHUNK: u32 = 1 << 12;
/// Instruments the books reserve room for.
pub const INSTRUMENTS: u32 = 1 << 20;
/// Lines the books reserve room for: the individuals' contracts now, and the cells' from the population's steps.
pub const LINES: u32 = 1 << 22;
/// Instruments and lines per chunk of their columns.
pub const BOOK_ROWS_PER_CHUNK: u32 = 1 << 14;
/// Blocks of each holder-list pool.
pub const HOLDER_BLOCKS: u32 = 1 << 20;

/// A save's format: a change of what a store holds or how it is written is a new format, and a load refuses others.
pub const SAVE_FORMAT: u32 = 2;
/// The file every save writes last, which makes it complete.
pub const SAVE_MANIFEST: &str = "manifest.json";
/// The suffix a save's directory carries until it is complete.
pub const SAVE_PARTIAL: &str = ".partial";
