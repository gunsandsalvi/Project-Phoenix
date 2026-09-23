use phx_id::TileId;
use phx_macros::clause;

/// A segment of the transport or power network between two tiles, of a declared kind, with its capacity in the kind's
/// units a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub from: TileId,
    pub to: TileId,
    pub kind: u16,
    pub capacity: u64,
}

/// The network infrastructure forms: owned capital with a path and a capacity. It holds no segment until capital
/// builds one.
#[clause("GEO.4")]
#[derive(Debug, Default)]
pub struct Network {
    pub segments: Vec<Segment>,
}
