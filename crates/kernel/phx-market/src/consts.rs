/// Metres in a kilometre, freight being priced by the tonne-km.
pub const METRES_A_KM: f64 = 1_000.0;

/// Buyers whose choices one job of the meeting draws: enough that a job outweighs its dispatch, few enough that a day's
/// buyers spread over the workers.
pub const CHOICE_CHUNK: usize = 4_096;
/// Blocks of a buyer's stream a round of the meeting keeps for it: one a choice takes, and room for the rare redraw an
/// unbiased index makes, so no round reads another's.
pub const ROUND_BLOCKS: u32 = 4;
/// Stalls one job of the meeting serves.
pub const STALL_CHUNK: usize = 1_024;
/// Bits of a stall's place the meeting groups buyers by in one pass: 256 streams to write keep a pass sequential.
pub const STALL_DIGIT_BITS: u32 = 8;
/// Seekers whose applications one job of the labour round draws.
pub const SEARCH_CHUNK: usize = 4_096;
