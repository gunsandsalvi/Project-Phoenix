use phx_id::{RegionId, TileId, ZoneId};
use phx_macros::{Pod, clause};

/// A tile's surface.
pub const LAND: u8 = 1;
pub const WATER: u8 = 0;

/// The zone field of a tile with none: water.
const NO_ZONE: u32 = u32::MAX;

/// One tile, 12 bytes: its elevation in metres, surface, terrain and climate classes, and its zone; its coordinates
/// derive from its identity, and its region and country are read through its zone.
#[clause("GEO.1")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Tile {
    pub elevation_m: i16,
    pub surface: u8,
    pub terrain: u8,
    pub climate: u8,
    pad: [u8; 3],
    zone: u32,
}

impl Tile {
    #[must_use]
    pub fn new(elevation_m: i16, surface: u8, terrain: u8, climate: u8, zone: Option<ZoneId>) -> Tile {
        Tile { elevation_m, surface, terrain, climate, pad: [0; 3], zone: zone.map_or(NO_ZONE, ZoneId::get) }
    }

    #[must_use]
    pub fn zone(&self) -> Option<ZoneId> {
        (self.zone != NO_ZONE).then(|| ZoneId::new(self.zone))
    }

    #[must_use]
    pub fn is_land(&self) -> bool {
        self.surface == LAND
    }
}

/// A zone: the region it lies in, and its centroid, the tile of least summed distance to its others.
#[clause("GEO.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Zone {
    pub region: RegionId,
    pub centroid: TileId,
    pub tiles: u32,
}

/// A region: the country it lies in.
#[clause("GEO.3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Region {
    pub country: phx_id::CountryId,
}

#[cfg(test)]
mod tests {
    use phx_id::ZoneId;

    use super::{LAND, Tile};

    #[test]
    fn tile_is_twelve_bytes() {
        assert_eq!(size_of::<Tile>(), 12);
        let t = Tile::new(-40, LAND, 2, 1, Some(ZoneId::new(7)));
        assert_eq!((t.zone(), t.is_land(), Tile::new(0, 0, 0, 0, None).zone()), (Some(ZoneId::new(7)), true, None));
    }
}
