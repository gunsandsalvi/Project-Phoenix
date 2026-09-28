/// Bits of a network node's or edge's key within an arc's key, leaving room for the arc's kind and its step's rank.
pub const NODE_BITS: u32 = 60;

/// An offer step's arc, from the source into its node.
pub const TAG_OFFER: u128 = 0;
/// A bid step's arc, from its node to the sink.
pub const TAG_BID: u128 = 1;
/// A capacity through a node, from where its offers arrive to where its bids leave.
pub const TAG_NODE: u128 = 2;
/// An edge between two nodes.
pub const TAG_EDGE: u128 = 3;
/// The return from the sink to the source.
pub const TAG_RETURN: u128 = 4;

/// Where an arc's kind sits in its key, above its step's rank.
pub const TAG_SHIFT: u32 = 64;
/// Where the node's or edge's key sits in an arc's key, above the kind's three bits.
pub const KEY_SHIFT: u32 = 67;
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
