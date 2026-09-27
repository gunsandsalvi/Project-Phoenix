//! The observer's constants.

/// One agent in this many, by its party's identity, is sampled for the members' moves: a sample fixed by who the agents
/// are, so no draw picks it, and large enough at the play resolution to read a share within a point.
pub const MOBILITY_SAMPLE: u64 = 64;
