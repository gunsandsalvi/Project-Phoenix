use phx_core::{ActsOn, DrawScheme, EventKindDecl, HazardDecl, PrimDecl, RateFn};

use crate::prims::{
    DROUGHT_COASTAL, DROUGHT_INLAND, DROUGHT_RATE, DROUGHT_RIVER, DROUGHT_SEVERITY_A, DROUGHT_SEVERITY_B,
    DROUGHT_SPREAD, FLOOD_COASTAL, FLOOD_INLAND, FLOOD_RATE, FLOOD_RIVER, FLOOD_SEVERITY_A, FLOOD_SEVERITY_B,
    FLOOD_SPREAD, QUAKE_COASTAL, QUAKE_INLAND, QUAKE_RATE, QUAKE_RIVER, QUAKE_SEVERITY_A, QUAKE_SEVERITY_B,
    QUAKE_SPREAD, STORM_COASTAL, STORM_INLAND, STORM_RATE, STORM_RIVER, STORM_SEVERITY_A, STORM_SEVERITY_B,
    STORM_SPREAD,
};

/// The stream catastrophes are drawn from, one opening per country and day.
pub const CATASTROPHE_STREAM: &str = "GEO.catastrophe";

/// A natural hazard on tiles: its declaration, the event an occurrence records, and its tables in the order
/// inland, coastal and river exposure, rate, spread and the two severity shapes.
#[derive(Clone, Copy, Debug)]
pub struct HazardSpec {
    pub decl: HazardDecl,
    pub event: EventKindDecl,
    pub tables: [&'static PrimDecl; 7],
}

const fn tile_hazard(name: &'static str, rate: &'static str, source: &'static str) -> HazardDecl {
    HazardDecl {
        name,
        acts_on: ActsOn::Tile,
        rate: RateFn { table: rate, axes: &["GEO.exposure"], changes: &[] },
        outcome: name,
        scheme: DrawScheme::Daily,
        stream: CATASTROPHE_STREAM,
        clause: "GEO.8",
        source,
    }
}

const fn catastrophe(name: &'static str) -> EventKindDecl {
    EventKindDecl { name, size_unit: "permille destroyed", clause: "GEO.8" }
}

/// Every natural hazard the map has, each drawn per tile from its exposure; a new one is a new entry here and its
/// tables in the data.
pub const HAZARDS: &[HazardSpec] = &[
    HazardSpec {
        decl: tile_hazard("GEO.flood", "GEO.flood_rate", "GEO.toml"),
        event: catastrophe("GEO.flood"),
        tables: [
            &FLOOD_INLAND,
            &FLOOD_COASTAL,
            &FLOOD_RIVER,
            &FLOOD_RATE,
            &FLOOD_SPREAD,
            &FLOOD_SEVERITY_A,
            &FLOOD_SEVERITY_B,
        ],
    },
    HazardSpec {
        decl: tile_hazard("GEO.storm", "GEO.storm_rate", "GEO.toml"),
        event: catastrophe("GEO.storm"),
        tables: [
            &STORM_INLAND,
            &STORM_COASTAL,
            &STORM_RIVER,
            &STORM_RATE,
            &STORM_SPREAD,
            &STORM_SEVERITY_A,
            &STORM_SEVERITY_B,
        ],
    },
    HazardSpec {
        decl: tile_hazard("GEO.earthquake", "GEO.earthquake_rate", "GEO.toml"),
        event: catastrophe("GEO.earthquake"),
        tables: [
            &QUAKE_INLAND,
            &QUAKE_COASTAL,
            &QUAKE_RIVER,
            &QUAKE_RATE,
            &QUAKE_SPREAD,
            &QUAKE_SEVERITY_A,
            &QUAKE_SEVERITY_B,
        ],
    },
    HazardSpec {
        decl: tile_hazard("GEO.drought", "GEO.drought_rate", "GEO.toml"),
        event: catastrophe("GEO.drought"),
        tables: [
            &DROUGHT_INLAND,
            &DROUGHT_COASTAL,
            &DROUGHT_RIVER,
            &DROUGHT_RATE,
            &DROUGHT_SPREAD,
            &DROUGHT_SEVERITY_A,
            &DROUGHT_SEVERITY_B,
        ],
    },
];
