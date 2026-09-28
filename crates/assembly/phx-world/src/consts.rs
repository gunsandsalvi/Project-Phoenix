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

/// Rows each kind table of individuals reserves: room for a kind's individuals across the three countries, beyond the
/// firms the promotion rank admits, in address space committed only as rows are written.
pub const KIND_ROWS: u32 = 1 << 18;
/// Rows of a kind table per chunk, as the population's tables chunk theirs.
pub const KIND_ROWS_PER_CHUNK: u32 = 1 << 12;
/// Rows each population kind's agent table reserves, in address space committed only as rows are written: some
/// thirty times the design point's households, under a million agents.
pub const AGENT_ROWS: u32 = 1 << 25;
/// Rows of an agent table per chunk.
pub const AGENT_ROWS_PER_CHUNK: u32 = 1 << 12;
/// Instruments the books reserve room for.
pub const INSTRUMENTS: u32 = 1 << 20;
/// Lines the books reserve room for: the individuals' contracts, and the agents' by their terms.
pub const LINES: u32 = 1 << 22;
/// Instruments and lines per chunk of their columns.
pub const BOOK_ROWS_PER_CHUNK: u32 = 1 << 14;
/// Blocks of each holder-list pool.
pub const HOLDER_BLOCKS: u32 = 1 << 20;

/// A save's format: a change of what a store holds or how it is written is a new format, and a load refuses others.
pub const SAVE_FORMAT: u32 = 11;
/// The file every save writes last, which makes it complete.
pub const SAVE_MANIFEST: &str = "manifest.json";
/// The suffix a save's directory carries until it is complete.
pub const SAVE_PARTIAL: &str = ".partial";
/// One agent in this many, by its identity's mix, is measured for the realised rates: enough for a year's sampling
/// error to be small against the rates' own, few enough to cost a small share of 3b.
pub const RATE_SAMPLE: u64 = 64;

/// The kind of party the law names to take what an estate leaves where no heir is drawn: the treasury, in every
/// opening country, until the inheritance law's destinations are declared per country.
pub const HEIRLESS_DESTINATION: &str = "treasury";

/// The population kind the player's party is drawn from at the opening: its household.
pub const PLAYER_KIND: &str = "household";
/// The shards 3b's agents are followed in, and how many are read at once: fixed, never the number of workers, so the
/// day's result is the same on any pool, and only a wave's draws wait to be written.
pub const GATHER_SHARDS: usize = 64;
/// See `GATHER_SHARDS`.
pub const GATHER_WAVE: usize = 8;
/// The shards 6a's sellers' stalls are read in: fixed, never the number of workers, as `GATHER_SHARDS` are.
pub const STALL_SHARDS: usize = 64;
/// The shards a visit's rows run in, and the fewest rows a visit shards: fixed, never the number of workers, as
/// `GATHER_SHARDS` are; a visit of fewer rows runs as one.
pub const VISIT_SHARDS: usize = 32;
/// See `VISIT_SHARDS`.
pub const VISIT_SHARD_ROWS: usize = 256;
/// The shards 5c's searchers are read in: fixed, never the number of workers, as `GATHER_SHARDS` are.
pub const SEARCH_SHARDS: usize = 64;
/// Billionths in a whole, for a review's daily chance as a position holds it.
pub const BILLION: f64 = 1e9;

/// Ten, the base a unit's price exponent counts powers of.
pub const DECADE: i64 = 10;

/// A half, added before a floor to round to the nearest.
pub const HALF: f64 = 0.5;

/// Metres in a kilometre, the unit buyers weigh a seller's distance in.
pub const METRES_PER_KM: f64 = 1_000.0;
/// Kilograms in a tonne, the unit goods are weighed in to fill vehicles and segments.
pub const KG_A_TONNE: f64 = 1_000.0;
/// Kilograms in a tonne, whole, as segments' capacities are counted.
pub const KG_A_TONNE_WHOLE: i64 = 1_000;
/// Days of a week, over which weekly hours and applications are spread.
pub const DAYS_A_WEEK: f64 = 7.0;
/// The days the least match takes: a vacancy posted today is applied to tomorrow, offered the day after and
/// accepted the day after that; one filled so fast was offered too high.
pub const LEAST_MATCH_DAYS: u32 = 3;
/// The Gregorian calendar's mean days a year, over which a yearly return is spread.
pub const DAYS_A_YEAR: f64 = 365.2425;
/// The places of decimals the required return a firm holds is written to.
pub const HURDLE_EXP: u8 = 6;
/// The decimal places of a firm's hours a unit.
pub const HOURS_EXP: u8 = 6;
/// Months of a year, for a loan's term in years and a month's share of a loan-year.
pub const MONTHS_A_YEAR: f64 = 12.0;
/// Months of a year, whole, for numbering months across years.
pub const MONTHS: i64 = 12;
/// Weeks of a year, for a bill's term in years.
pub const WEEKS_A_YEAR: f64 = DAYS_A_YEAR / DAYS_A_WEEK;
/// A rate of one in phx-num's rate scale, for a yearly rate as the ledger holds it.
pub const RATE_ONE: f64 = 1_000_000_000_000.0;
/// The opening balance sheet's sectors worth nothing beyond their equity, by their column: firms, banks and the central
/// bank (households are 0, the government 4).
pub const SECTORS_WORTH_NOTHING: [usize; 3] = [1, 2, 3];
/// The balance sheet's sectors by their column, and its instruments and closing real assets by their row, as the
/// dataset's derivation lays them out.
/// The kinds of party on the core, in their order, and each one's place: those sited by a tile, then the households.
pub mod kinds {
    pub const KINDS: [&str; 6] = ["central_bank", "treasury", "bank", "firm", phx_core::ESTATE_KIND.name, "household"];
    pub const CENTRAL_BANK: usize = 0;
    pub const TREASURY: usize = 1;
    pub const BANK: usize = 2;
    pub const FIRM: usize = 3;
    pub const ESTATE: usize = 4;
    pub const HOUSEHOLD: usize = 5;
}
pub mod sheet {
    pub const HOUSEHOLDS: usize = 0;
    pub const FIRMS: usize = 1;
    pub const BANKS: usize = 2;
    pub const CENTRAL_BANK: usize = 3;
    pub const GOVERNMENT: usize = 4;
    pub const CURRENCY: usize = 0;
    pub const DEPOSITS: usize = 1;
    pub const LOANS_TO_HOUSEHOLDS: usize = 2;
    pub const LOANS_TO_FIRMS: usize = 3;
    pub const FIRMS_BONDS: usize = 4;
    pub const GOVERNMENT_PAPER: usize = 5;
    pub const RESERVES: usize = 6;
    pub const CENTRAL_BANK_LOANS: usize = 7;
    pub const BANKS_BONDS: usize = 8;
    pub const BANKS_EQUITY: usize = 9;
    pub const FIRMS_EQUITY: usize = 10;
    pub const INSTRUMENTS: usize = 11;
    pub const SECTORS: usize = 5;
}
/// A percentage's whole.
pub const PERCENT: f64 = 100.0;
/// The core's ranges of slots settlement nets by, 2^bits slots each.
pub const CORE_RANGE_BITS: u32 = 12;
/// The days the core's due wheels hold before their far list.
pub const CORE_WHEEL_DAYS: u32 = 64;
/// A core firm's record: its product, its region, the tile it is sited on, its productivity, the log of its factor
/// over its way's, in billionths, its posted price of a lot, its output a year in units, its markup over its unit cost
/// and the sales a day it expects in millionths, the units it sold since its last review and that review's day; the
/// purposes its opening
/// draws and its jobs' dealing are keyed by; and the column of the value added's parts that is labour's.
pub mod firm {
    pub const RECORD: usize = 10;
    pub const PRODUCT: usize = 0;
    pub const REGION: usize = 1;
    pub const SITE: usize = 2;
    pub const PRODUCTIVITY: usize = 3;
    pub const PRICE: usize = 4;
    pub const OUTPUT: usize = 5;
    pub const MARKUP: usize = 6;
    pub const EXPECTED: usize = 7;
    pub const SOLD: usize = 8;
    pub const REVIEWED: usize = 9;
    pub const PART_ONE: f64 = 1_000_000.0;
    pub const PRODUCTIVITY_ONE: f64 = 1_000_000_000.0;
    pub const PURPOSES: u32 = 4;
    pub const PRODUCTIVITY_PURPOSE: u32 = 0;
    pub const SITE_PURPOSE: u32 = 1;
    pub const JOBS_PURPOSE: u32 = 2;
    pub const LOANS_PURPOSE: u32 = 3;
    pub const COMPENSATION: usize = 0;
}
/// The reasons the core's flows are made for, by their code.
pub mod reason {
    pub const PENSION: u8 = 1;
    pub const ESTATE: u8 = 2;
    pub const WAGE: u8 = 3;
    pub const SEVERANCE: u8 = 4;
    pub const SOLD: u8 = 5;
    pub const MADE: u8 = 6;
    pub const USED: u8 = 7;
    pub const DELIVERED: u8 = 8;
    pub const REPAID: u8 = 9;
    pub const TAXED: u8 = 10;
    pub const BENEFIT: u8 = 11;
    pub const LENT: u8 = 12;
    /// The reasons a day's record counts failed flows by: every reason above.
    pub const REASONS: usize = 13;
}
/// The bits a draw's subject gives a region beside its buyer, and a meeting's round beside its seller.
pub mod draws {
    pub const REGION_BITS: u32 = 16;
    pub const ROUND_BITS: u32 = 20;
}
