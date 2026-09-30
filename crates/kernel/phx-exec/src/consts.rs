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

/// Declared cost a plan's chunk accumulates before it may close, in the cost units handlers declare per row (about a
/// nanosecond of phone core time): 200 µs of work amortises a dispatch several times over.
pub const CHUNK_COST: u64 = 200_000;

/// A core is used when its capacity is at least half the largest core's: fast and medium cores, not the little ones,
/// which would hold up every barrier.
pub const LITTLE_CORE_SHARE_NUM: u32 = 1;
/// See `LITTLE_CORE_SHARE_NUM`.
pub const LITTLE_CORE_SHARE_DEN: u32 = 2;

/// Microseconds an idle worker spins after a dispatch before it parks, so back-to-back dispatches skip a wake-up
/// without a worker burning a core between sub-steps.
pub const SPIN_US: u64 = 20;

/// Rounds of spinning timed once at a pool's start to learn what a microsecond of spin is on its cores.
pub const SPIN_CALIBRATION_ROUNDS: u64 = 1 << 16;

/// The most workers a pool runs: the phone's performance cores and a margin. A plan's chunks never depend on it but
/// are at least four for each of them, so any count of workers up to it finds work to balance.
pub const POOL_MAX_WORKERS: u32 = 8;

/// A plan has at least this many chunks for each of the most workers, where its rows allow.
pub const CHUNKS_PER_MAX_WORKER: u32 = 4;

/// A plan whose declared cost is below this many cost units (about 100 µs of phone core time) runs on the calling
/// thread, over the same chunks in order: a dispatch would cost more than it saves.
pub const INLINE_BELOW: u64 = 100_000;

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

/// Microseconds a second.
pub const US_PER_S: u64 = 1_000_000;

/// Bytes each worker fills or sweeps as one piece of a probe region.
pub const PROBE_PIECE_BYTES: usize = 1 << 20;

/// Where Linux publishes each core's relative capacity, `cpu<N>` in the middle.
pub const CAPACITY_PATH: [&str; 2] = ["/sys/devices/system/cpu/cpu", "/cpu_capacity"];

/// Largest core number the affinity mask is read for: Linux's default `CPU_SETSIZE`.
pub const MAX_CPUS: usize = 1024;
/// Bytes in a kibibyte, the unit the system reports peak resident memory in.
pub const KIB: u64 = 1024;
/// The kinds a party may be of: as many as its key's kind bits can name.
pub const KINDS: usize = 1 << phx_id::consts::KEY_KIND_BITS;
/// Months in a year: the stores are sampled at each month's end and their growth read over whole years.
pub const MONTHS_A_YEAR: usize = 12;
/// Bytes of the region the run's gather probe reads: well past every cache of a phone, so each row is a miss.
pub const PROBE_GATHER_BYTES: u64 = 64 << 20;
/// Random rows each worker reads in the run's gather probe: about 25 ms of misses, twice over.
pub const PROBE_GATHER_READS: u64 = 1 << 18;

/// An item's scatter and sweep in a partitioned apply, in a plan's cost units: about 4 ns of phone core time.
pub const APPLY_ITEM_COST: u64 = 4;

/// A store's rows a partitioned apply's range holds, as a shift: 4 096 accounts, about 80 KB, an L2's share.
pub const APPLY_RANGE_SHIFT: u32 = 12;
