/// Address space all stores together may reserve: 64 GiB, a sixth of a 39-bit phone address space, leaving the rest to
/// the system and the application.
pub const VA_BUDGET: usize = 1 << 36;

/// Page size of the heap backing, which stands in for the system's pages under Miri and in tests; phones use 16 KiB.
pub const HEAP_PAGE: usize = 1 << 14;

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

/// A short reference's link holds its family in the top byte and the row's slot in the 24 bits below, the most rows a
/// contract family holds.
pub const SHORT_LINK_SLOT_BITS: u32 = 24;

/// The kinds of day a day buffer records its longest length on: a business day, a day no market opens, a heavy day,
/// and the business day after closed days.
pub const DAY_KINDS: usize = 4;

/// A sum-tree's first class holds four members.
pub const SUMTREE_BASE_CAPACITY: usize = 4;

/// Sum-tree classes a doubling: four, so each class holds a quarter of the base's octave more than the one before.
pub const SUMTREE_STEPS: u32 = 4;

/// A sum-tree's length takes the low 24 bits of its header's word, its class the rest.
pub const SUMTREE_LEN_BITS: u32 = 24;

/// Sum-trees' columns chunk as the tables' do; no traversal walks them by chunk.
pub const SUMTREE_ROWS_PER_CHUNK: u32 = 1 << 12;

/// Sum-tree classes: four a doubling from the base until a class holds every length a header's 24 bits count.
pub const SUMTREE_CLASSES: usize = 88;

/// An index's blocks are sized for their entries at three quarters of a block, the fill a lazy list keeps between
/// compactions.
pub const INDEX_BLOCKS_PER_ENTRIES: (u32, u32) = (3, 4);

/// A sparse index's pairs come since its last merge are sorted apart and merged into its pairs in one pass when their
/// run fills: at least this many, so a merge's cost is shared by the pairs that filled it.
pub const INDEX_NEW_KEYS: u32 = 1 << 12;

/// A sparse index's run of new pairs holds a sixteenth of its entries, so a merge copies its pairs about once for each
/// sixteenth that comes.
pub const INDEX_NEW_SHARE: u32 = 16;

/// The floors a ring's owner may set on its horizon over a run: one a change of the policy that sets it.
pub const RING_FLOORS: u32 = 64;

/// The interner's hash key: fixed, so an interner's index depends on its values alone.
pub const INTERN_KEY: [u64; 2] = [0x5048_5820_494e_5445, 0x524e_4552_204b_4559];

/// An interned id's slot bits; its generation takes the rest of its word.
pub const INTERN_SLOT_BITS: u32 = 24;

/// An interner's index is kept at most two-thirds full: its cells are three for every two ids it may hold.
pub const INTERN_CELLS_PER_IDS: (u32, u32) = (3, 2);

/// An interner's values are compacted at a close once their dead bytes pass an eighth of the bytes in use.
pub const INTERN_DEAD_SHARE: usize = 8;

/// Values an interner looks up together, each phase of their lookups read across the group so the reads overlap.
pub const INTERN_GROUP: usize = 16;
