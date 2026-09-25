/// Address space all stores together may reserve: 64 GiB, a sixth of a 39-bit phone address space, leaving the rest to
/// the system and the application.
pub const VA_BUDGET: usize = 1 << 36;

/// Page size of the heap backing, which stands in for the system's pages under Miri and in tests; phones use 16 KiB.
pub const HEAP_PAGE: usize = 1 << 14;

/// Rows per chunk when a table declares none: 4 096 rows keep a chunk's hot columns within a core's L2 cache and give
/// a few hundred chunks to share among the pool's threads.
pub const DEFAULT_ROWS_PER_CHUNK: u32 = 1 << 12;

/// Words a chunk's arena reserves when its table declares none: 32 MiB of address space, 1 024 words per row of a
/// full chunk. Reservation costs no memory until committed.
pub const ARENA_RESERVED_WORDS: u32 = 1 << 22;

/// Words a chunk's arena of individuals reserves: 128 MiB of address space, 4 096 words per row of a full chunk, since
/// an individual — a firm, a bank — holds a row on each of the many lines of the households' terms it is party to, one
/// per wage or rent point. Their tables are few chunks, so the reservation stays small against the budget.
pub const INDIVIDUAL_ARENA_WORDS: u32 = 1 << 24;

/// A relocated list's capacity grows by 5/4 of the length it needs, so repeated appends relocate a logarithmic number
/// of times while the slack stays near an eighth on average.
pub const ARENA_GROWTH_NUM: u64 = 5;
/// See `ARENA_GROWTH_NUM`.
pub const ARENA_GROWTH_DEN: u64 = 4;

/// Capacities are rounded up to four words, 32 bytes, the widest variable-stride record.
pub const ARENA_ROUND_WORDS: u32 = 4;

/// An arena is compacted when its dead words pass an eighth of its used words; with growth slack this keeps the
/// arena's waste within the memory budget's 15%.
pub const ARENA_DEAD_NUM: u64 = 1;
/// See `ARENA_DEAD_NUM`.
pub const ARENA_DEAD_DEN: u64 = 8;

/// A cell list reference's length and capacity are 16 bits; this length marks a list whose reference lives in the
/// arena's overflow map instead.
pub const CELL_LIST_SENTINEL: u16 = u16::MAX;

/// Entries in one block of a block list: 16 slots of 4 bytes fill one 64-byte cache line.
pub const BLOCK_ENTRIES: usize = 16;

/// A block holds at least half its entries unless it is its list's only block.
pub const BLOCK_HALF: usize = BLOCK_ENTRIES / 2;

/// Deepest block tree: 16-way nodes at least half full hold 8^12 ≈ 6.9·10^10 entries, beyond any list of the world.
pub const BLOCK_TREE_DEPTH: usize = 12;

/// Elements per encoding block: 1 024 values share one bit width, small enough that an outlier widens few values.
pub const ENCODE_BLOCK: usize = 1 << 10;

/// Bytes of transformed data per zstd frame: 1 MiB frames compress well and let a save's columns be written in
/// parallel.
pub const FRAME_BYTES: usize = 1 << 20;

/// Bytes a save's reader takes at a time for a length it was given: 1 MiB, so that a damaged length runs out of
/// store long before it runs out of memory.
pub const SAVE_READ_STEP: usize = 1 << 20;

/// The bytes of one frame of a framed save: a frame is compressed on its own, so a store's frames compress at once on
/// many workers; a mebibyte keeps each frame's matches long and a wave's frames small beside the world.
pub const SAVE_FRAME_BYTES: usize = 1 << 20;
/// The frames of a framed save compressed at once: enough for every worker the phone runs, twice over.
pub const SAVE_FRAME_WAVE: usize = 16;

/// zstd's fastest level: saves stop the world, and the transforms already remove most redundancy.
pub const ZSTD_LEVEL: i32 = 1;

/// "PXCL" read as a little-endian word: the first bytes of every encoded column.
pub const ENCODE_MAGIC: u32 = 0x4C43_5850;

/// The encoding's format; a change of layout is a new version, and a reader refuses versions it does not know.
pub const ENCODE_VERSION: u16 = 1;

/// Rounds of the content hash: two per message word and four to finalise, the published `SipHash-2-4`.
pub const SIP_C_ROUNDS: usize = 2;
/// See `SIP_C_ROUNDS`.
pub const SIP_D_ROUNDS: usize = 4;
/// The content hash's initial state words, the ASCII of "somepseudorandomlygeneratedbytes".
pub const SIP_INIT: [u64; 4] =
    [0x736f_6d65_7073_6575, 0x646f_7261_6e64_6f6d, 0x6c79_6765_6e65_7261, 0x7465_6462_7974_6573];
/// The 128-bit variant's tweaks: v1 at the start and before the second half, v2 before the first half.
pub const SIP_TWEAK_128: u64 = 0xee;
/// See `SIP_TWEAK_128`.
pub const SIP_TWEAK_SECOND: u64 = 0xdd;
/// The hash round's rotations, in the order the round applies them.
pub const SIP_ROT: [u32; 6] = [13, 32, 16, 21, 17, 32];
/// The final word carries the message length in its top byte.
pub const SIP_LEN_SHIFT: u32 = 56;
