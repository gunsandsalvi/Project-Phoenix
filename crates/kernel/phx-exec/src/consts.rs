/// Shards of a keyed reduction, as bits of the mixed key: 256 shards keep each shard's sort within a core's L2 cache
/// at the design point's sizes and give far more shards than workers, so uneven shards still balance.
pub const KEYED_SHARD_BITS: u32 = 8;
/// See `KEYED_SHARD_BITS`.
pub const KEYED_SHARDS: usize = 1 << KEYED_SHARD_BITS;

/// Radix digit width: 11 bits give 2 048 buckets, whose counts fit L1, and six passes over a 64-bit key.
pub const RADIX_BITS: u32 = 11;

/// Buckets of one radix digit.
pub const RADIX_BUCKETS: usize = 1 << RADIX_BITS;

/// Elements per parallel piece of a radix pass: 64 Ki pairs of up to 24 bytes stay within a core's L2 cache while it
/// counts and scatters them, and the pieces depend only on the input's length.
pub const RADIX_CHUNK: usize = 1 << 16;

/// Fewer pairs than this sort by a stable comparison sort: a radix pass's 2 048 buckets cost more to set up than
/// comparing so few, and the stable result is the same.
pub const RADIX_MIN: usize = 1 << 11;

/// Gathers below this many intents copy on the calling thread: a dispatch costs more than copying them.
pub const GATHER_SERIAL_BELOW: usize = 1 << 16;

/// Declared cost an agenda unit accumulates before it may close, in the cost units handlers declare per row (about a
/// nanosecond of phone core time): 200 µs of work amortises a dispatch several times over.
pub const CHUNK_COST: u64 = 200_000;

/// A core is used when its capacity is at least half the largest core's: fast and medium cores, not the little ones,
/// which would hold up every barrier.
pub const LITTLE_CORE_SHARE_NUM: u32 = 1;
/// See `LITTLE_CORE_SHARE_NUM`.
pub const LITTLE_CORE_SHARE_DEN: u32 = 2;

/// Rounds an idle worker spins after a dispatch before parking, so back-to-back sub-steps skip a wake-up: about
/// 50 µs on a phone core at a few nanoseconds a round; the bench measures the barrier it buys.
pub const SPIN_ROUNDS: u64 = 16_384;

/// The `SplitMix64` finaliser: the shift and multiplier constants of Steele, Lea and Flood (2014).
pub const MIX_SHIFTS: [u32; 3] = [30, 27, 31];
/// See `MIX_SHIFTS`.
pub const MIX_MULTIPLIERS: [u64; 2] = [0xbf58_476d_1ce4_e5b9, 0x94d0_49bb_1331_11eb];

/// Philox blocks in one unit of the probe's compute work: enough that counting units costs nothing beside them.
pub const PROBE_UNIT_BLOCKS: u32 = 256;

/// How far ahead the probe's prefetching gather asks for rows: 16 misses in flight cover a phone's memory latency.
pub const PREFETCH_DISTANCE: u64 = 16;

/// Nanoseconds in a second, for rates per second.
pub const NS_PER_S: u64 = 1_000_000_000;

/// Bytes each worker fills or sweeps as one piece of a probe region.
pub const PROBE_PIECE_BYTES: usize = 1 << 20;

/// Where Linux publishes each core's relative capacity, `cpu<N>` in the middle.
pub const CAPACITY_PATH: [&str; 2] = ["/sys/devices/system/cpu/cpu", "/cpu_capacity"];

/// Largest core number the affinity mask is read for: Linux's default `CPU_SETSIZE`.
pub const MAX_CPUS: usize = 1024;
