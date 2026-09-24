use phx_core::{Declarations, Prim, Table1, Table2, declare_prim};
use phx_num::{Count, Fixed};

declare_prim! {
    /// A tile's side, in metres.
    pub TILE_M = "GEO.tile_m" { kind: Resolution, value: Count, clause: "GEO.18", scope: Shared }
}

declare_prim! {
    /// The share of the map's tiles under the sea, which sets the sea level.
    pub SEA_SHARE = "GEO.sea_share" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// Cells across the coarsest octave of the height noise.
    pub BASE_CELLS = "GEO.base_cells" {
        kind: Shape, value: Count, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// Octaves of the height noise, each twice as fine as the one before.
    pub OCTAVES = "GEO.octaves" {
        kind: Shape, value: Count, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// Each octave's weight as a share of the one before: the terrain's roughness.
    pub ROUGHNESS = "GEO.roughness" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// How fast the height falls away from the map's centre, which puts the sea at its edges.
    pub FALLOFF = "GEO.falloff" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// Relief cells across a tile: the relief is generated, eroded and measured this much finer than the tiles.
    pub RELIEF_CELLS = "GEO.relief_cells_per_tile" { kind: Resolution, value: Count, clause: "GEO.18", scope: Shared }
}

declare_prim! {
    /// Tectonic plates the relief is raised from.
    pub PLATES = "GEO.plates" {
        kind: Shape, value: Count, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// The width of a plate boundary's influence, as a share of the map's side: mountain belts and crust blends.
    pub PLATE_BELT = "GEO.plate_belt" {
        kind: Shape, value: Fixed { exp: 3 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// How much the plates' crust weighs in the relief against the fractal noise.
    pub PLATE_WEIGHT = "GEO.plate_weight" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// How much converging plates raise their mountain belts.
    pub MOUNTAIN_WEIGHT = "GEO.mountain_weight" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// How far the plane is warped before the relief is read, as a share of the map's side: bays, capes and bends.
    pub WARP = "GEO.warp" {
        kind: Shape, value: Fixed { exp: 3 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// Passes of fluvial erosion over the relief.
    pub EROSION_PASSES = "GEO.erosion_passes" {
        kind: Shape, value: Count, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// The stream-power law's rate, per pass, in the relief's own units.
    pub EROSION_RATE = "GEO.erosion_rate" {
        kind: Shape, value: Fixed { exp: 5 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// The stream-power law's exponent of the area draining through a cell.
    pub AREA_EXPONENT = "GEO.erosion_area_exponent" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "GEO.10", scope: Shared,
        shape: standing("the ground is given: plate tectonics, erosion and the making of relief are outside the world")
    }
}

declare_prim! {
    /// The land's heights in metres at each part per thousand of its area, lowest first, measured over the analogue
    /// region.
    pub LAND_HEIGHTS = "GEO.land_heights" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GEO.10", scope: Shared
    }
}

declare_prim! {
    /// The sea's heights in metres (below zero) at each part per thousand of its area, deepest first.
    pub SEA_DEPTHS = "GEO.sea_depths" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GEO.10", scope: Shared
    }
}

declare_prim! {
    /// The tiles a tile's drainage must gather for it to carry a river.
    pub RIVER_TILES = "GEO.river_tiles" { kind: Resolution, value: Count, clause: "GEO.7", scope: Shared }
}

declare_prim! {
    /// The relief in metres that doubles a step's cost to a growing country, region or zone.
    pub RUGGED_M = "GEO.rugged_m" { kind: Technology, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// The metres of travel a river crossing adds to a growing country, region or zone.
    pub RIVER_CROSSING_M = "GEO.river_crossing_m" { kind: Technology, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// How far, in parts per thousand, a country's land may stray from its share, and a region from its country's
    /// mean.
    pub SHARE_TOLERANCE = "GEO.share_tolerance_per_mille" {
        kind: Shape, value: Count, clause: "GEO.3", scope: Shared,
        shape: standing("a construction condition: regions of like size, as GEO.3 asks, and countries holding their shares of the land, which no mechanism of the world adjusts")
    }
}

declare_prim! {
    /// The most rounds in which a partition's handicaps are moved toward its parts' targets.
    pub PARTITION_ROUNDS = "GEO.partition_rounds" { kind: Resolution, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// Zones across the three countries, shared among them in their population shares.
    pub ZONES = "GEO.zones" { kind: Resolution, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// The fewest tiles a zone holds.
    pub ZONE_MIN_TILES = "GEO.zone_min_tiles" { kind: Resolution, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// The most tiles a zone holds.
    pub ZONE_MAX_TILES = "GEO.zone_max_tiles" { kind: Resolution, value: Count, clause: "GEO.3", scope: Shared }
}

declare_prim! {
    /// The least share of a country's land, in percent, that lies on the mainland.
    pub MAINLAND_FLOOR = "GEO.mainland_floor_percent" {
        kind: Shape, value: Count, clause: "GEO.10", scope: Shared,
        shape: standing("a construction condition: a country scattered over islands has no land market or road across it, and no mechanism joins them")
    }
}

declare_prim! {
    /// The latitude of the map's south edge, in degrees north, on the owner's projection.
    pub SOUTH_LATITUDE = "GEO.south_latitude" {
        kind: Endowment, value: Fixed { exp: 1 }, clause: "GEO.18", scope: Shared
    }
}

declare_prim! {
    /// Metres of the Earth's surface in a degree of latitude, by which the map's height spans its latitudes.
    pub METRES_PER_DEGREE = "GEO.metres_per_degree" {
        kind: Endowment, value: Count, clause: "GEO.18", scope: Shared
    }
}

declare_prim! {
    /// Each terrain class's highest elevation in metres, by class; the last class takes every tile the others leave.
    pub TERRAIN_ELEVATION = "GEO.terrain_max_elevation" {
        kind: Shape, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GEO.1", scope: Shared,
        shape: standing("terrain classes are how the relief is read by exposure and deposits; no mechanism classifies ground")
    }
}

declare_prim! {
    /// Each terrain class's greatest relief within a tile, the range of its heights in metres, by class.
    pub TERRAIN_RELIEF = "GEO.terrain_max_relief" {
        kind: Shape, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GEO.1", scope: Shared,
        shape: standing("terrain classes are how the relief is read by exposure and deposits; no mechanism classifies ground")
    }
}

declare_prim! {
    /// A lowland tile's climate class, by its latitude in degrees and its distance to the sea in kilometres.
    pub CLIMATE_LOWLAND = "GEO.climate_lowland" {
        kind: Endowment, value: Table2 { row_exp: 1, column_exp: 0, exp: 0 }, clause: "GEO.7", scope: Shared
    }
}

declare_prim! {
    /// The elevation in metres above which a tile's climate is the highland class, by latitude in degrees.
    pub HIGHLAND_ELEVATION = "GEO.highland_elevation" {
        kind: Endowment, value: Table1 { axis_exp: 1, exp: 0 }, clause: "GEO.7", scope: Shared
    }
}

declare_prim! {
    /// The highland climate class, by latitude in degrees.
    pub HIGHLAND_CLASS = "GEO.highland_class" {
        kind: Endowment, value: Table1 { axis_exp: 1, exp: 0 }, clause: "GEO.7", scope: Shared
    }
}

declare_prim! {
    /// The distance to the sea, in metres, within which a tile is exposed as a coastal one.
    pub COAST_M = "GEO.coast_m" { kind: Technology, value: Count, clause: "GEO.7", scope: Shared }
}

declare_prim! {
    /// The day's mean temperature in °C, by climate class and month.
    pub TEMP_MEAN = "GEO.temperature_mean" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 1 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The day's temperature's standard deviation in °C, by climate class and month.
    pub TEMP_SD = "GEO.temperature_sd" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 1 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The share of dry days, by climate class and month.
    pub DRY_SHARE = "GEO.dry_share" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 3 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The shape of a wet day's rainfall, a gamma, by climate class and month.
    pub RAIN_SHAPE = "GEO.rain_shape" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 2 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The scale of a wet day's rainfall in millimetres, by climate class and month.
    pub RAIN_SCALE = "GEO.rain_scale" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 1 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The shape of the day's mean wind speed, a Weibull, by climate class and month.
    pub WIND_SHAPE = "GEO.wind_shape" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 2 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The scale of the day's mean wind speed in metres a second, by climate class and month.
    pub WIND_SCALE = "GEO.wind_scale" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 1 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The first shape of the day's sunshine, its clear-sky index (the irradiance that reached the ground as a share of
    /// a cloudless day's), a beta, by climate class and month.
    pub SUN_A = "GEO.sunshine_a" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 2 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// The second shape of the day's clear-sky index, by climate class and month.
    pub SUN_B = "GEO.sunshine_b" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 2 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// Each weather variable's day-to-day persistence, the latent's autocorrelation, by climate class and variable in
    /// the order temperature, rain, wind, sunshine.
    pub PERSISTENCE = "GEO.persistence" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 3 }, clause: "CHN.3", scope: Shared
    }
}

declare_prim! {
    /// Each resource's chance that a tile holds a deposit of it, by resource and terrain class.
    pub DEPOSIT_DENSITY = "GEO.deposit_density" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 4 }, clause: "GEO.6", scope: Shared
    }
}

declare_prim! {
    /// The mean of a deposit's log grade, by resource.
    pub GRADE_MU = "GEO.deposit_grade_mu" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GEO.6", scope: Shared
    }
}

declare_prim! {
    /// The standard deviation of a deposit's log grade, by resource.
    pub GRADE_SIGMA = "GEO.deposit_grade_sigma" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GEO.6", scope: Shared
    }
}

declare_prim! {
    /// The mean of a finite deposit's log opening quantity in the resource's units, by resource.
    pub QUANTITY_MU = "GEO.deposit_quantity_mu" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GEO.6", scope: Shared
    }
}

declare_prim! {
    /// The standard deviation of a finite deposit's log opening quantity, by resource.
    pub QUANTITY_SIGMA = "GEO.deposit_quantity_sigma" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GEO.6", scope: Shared
    }
}

declare_prim! {
    /// Whether a resource's deposits are unbounded, one, or finite, zero, by resource.
    pub UNBOUNDED = "GEO.deposit_unbounded" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 0 }, clause: "GEO.6", scope: Shared
    }
}

/// Declares a hazard's seven tables: its exposure class inland, on the coast and on a river by terrain and climate
/// class, and by
/// exposure class its yearly chance of starting on a tile, its chance of spreading to a neighbour, and the two shapes
/// of the share of what stands there it destroys.
macro_rules! hazard_prims {
    ($inland:ident = $i:literal, $coastal:ident = $c:literal, $river:ident = $v:literal, $rate:ident = $r:literal,
     $spread:ident = $s:literal, $sa:ident = $a:literal, $sb:ident = $b:literal) => {
        declare_prim! {
            /// The hazard's exposure class of an inland tile, by terrain class and climate class.
            pub $inland = $i {
                kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 0 }, clause: "GEO.7", scope: Shared
            }
        }
        declare_prim! {
            /// The hazard's exposure class of a coastal tile, by terrain class and climate class.
            pub $coastal = $c {
                kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 0 }, clause: "GEO.7", scope: Shared
            }
        }
        declare_prim! {
            /// The hazard's exposure class of a tile a river runs through, by terrain class and climate class.
            pub $river = $v {
                kind: Technology, value: Table2 { row_exp: 0, column_exp: 0, exp: 0 }, clause: "GEO.7", scope: Shared
            }
        }
        declare_prim! {
            /// The hazard's yearly chance of starting on a tile, by exposure class.
            pub $rate = $r {
                kind: Technology, value: Table1 { axis_exp: 0, exp: 8 }, clause: "CHN.2", scope: Shared
            }
        }
        declare_prim! {
            /// The hazard's chance of spreading to a neighbour, by the neighbour's exposure class.
            pub $spread = $s {
                kind: Technology, value: Table1 { axis_exp: 0, exp: 3 }, clause: "GEO.8", scope: Shared
            }
        }
        declare_prim! {
            /// The first shape of the share of what stands on a struck tile the hazard destroys, by exposure class.
            pub $sa = $a {
                kind: Technology, value: Table1 { axis_exp: 0, exp: 2 }, clause: "GEO.8", scope: Shared
            }
        }
        declare_prim! {
            /// The second shape of the share destroyed, by exposure class.
            pub $sb = $b {
                kind: Technology, value: Table1 { axis_exp: 0, exp: 2 }, clause: "GEO.8", scope: Shared
            }
        }
    };
}

hazard_prims!(
    FLOOD_INLAND = "GEO.flood_exposure_inland",
    FLOOD_COASTAL = "GEO.flood_exposure_coastal",
    FLOOD_RIVER = "GEO.flood_exposure_river",
    FLOOD_RATE = "GEO.flood_rate",
    FLOOD_SPREAD = "GEO.flood_spread",
    FLOOD_SEVERITY_A = "GEO.flood_severity_a",
    FLOOD_SEVERITY_B = "GEO.flood_severity_b"
);
hazard_prims!(
    STORM_INLAND = "GEO.storm_exposure_inland",
    STORM_COASTAL = "GEO.storm_exposure_coastal",
    STORM_RIVER = "GEO.storm_exposure_river",
    STORM_RATE = "GEO.storm_rate",
    STORM_SPREAD = "GEO.storm_spread",
    STORM_SEVERITY_A = "GEO.storm_severity_a",
    STORM_SEVERITY_B = "GEO.storm_severity_b"
);
hazard_prims!(
    QUAKE_INLAND = "GEO.earthquake_exposure_inland",
    QUAKE_COASTAL = "GEO.earthquake_exposure_coastal",
    QUAKE_RIVER = "GEO.earthquake_exposure_river",
    QUAKE_RATE = "GEO.earthquake_rate",
    QUAKE_SPREAD = "GEO.earthquake_spread",
    QUAKE_SEVERITY_A = "GEO.earthquake_severity_a",
    QUAKE_SEVERITY_B = "GEO.earthquake_severity_b"
);
hazard_prims!(
    DROUGHT_INLAND = "GEO.drought_exposure_inland",
    DROUGHT_COASTAL = "GEO.drought_exposure_coastal",
    DROUGHT_RIVER = "GEO.drought_exposure_river",
    DROUGHT_RATE = "GEO.drought_rate",
    DROUGHT_SPREAD = "GEO.drought_spread",
    DROUGHT_SEVERITY_A = "GEO.drought_severity_a",
    DROUGHT_SEVERITY_B = "GEO.drought_severity_b"
);

/// A hazard's tables, by handle.
#[derive(Clone, Copy, Debug)]
pub struct HazardPrims {
    pub inland: Prim<Table2>,
    pub coastal: Prim<Table2>,
    pub river: Prim<Table2>,
    pub rate: Prim<Table1>,
    pub spread: Prim<Table1>,
    pub severity_a: Prim<Table1>,
    pub severity_b: Prim<Table1>,
}

/// The weather's tables, by handle.
#[derive(Clone, Copy, Debug)]
pub struct WeatherPrims {
    pub temp_mean: Prim<Table2>,
    pub temp_sd: Prim<Table2>,
    pub dry_share: Prim<Table2>,
    pub rain_shape: Prim<Table2>,
    pub rain_scale: Prim<Table2>,
    pub wind_shape: Prim<Table2>,
    pub wind_scale: Prim<Table2>,
    pub sun_a: Prim<Table2>,
    pub sun_b: Prim<Table2>,
    pub persistence: Prim<Table2>,
}

/// The deposits' tables, by handle.
#[derive(Clone, Copy, Debug)]
pub struct DepositPrims {
    pub density: Prim<Table2>,
    pub grade_mu: Prim<Table1>,
    pub grade_sigma: Prim<Table1>,
    pub quantity_mu: Prim<Table1>,
    pub quantity_sigma: Prim<Table1>,
    pub unbounded: Prim<Table1>,
}

/// Every GEO primitive, by handle, declared with the kernel's before any system's.
#[derive(Clone, Debug)]
pub struct GeoPrims {
    pub tile_m: Prim<Count>,
    pub sea_share: Prim<Fixed<2>>,
    pub base_cells: Prim<Count>,
    pub octaves: Prim<Count>,
    pub roughness: Prim<Fixed<2>>,
    pub falloff: Prim<Fixed<2>>,
    pub relief_cells: Prim<Count>,
    pub plates: Prim<Count>,
    pub plate_belt: Prim<Fixed<3>>,
    pub plate_weight: Prim<Fixed<2>>,
    pub mountain_weight: Prim<Fixed<2>>,
    pub warp: Prim<Fixed<3>>,
    pub erosion_passes: Prim<Count>,
    pub erosion_rate: Prim<Fixed<5>>,
    pub area_exponent: Prim<Fixed<2>>,
    pub land_heights: Prim<Table1>,
    pub sea_depths: Prim<Table1>,
    pub river_tiles: Prim<Count>,
    pub rugged_m: Prim<Count>,
    pub river_crossing_m: Prim<Count>,
    pub share_tolerance: Prim<Count>,
    pub partition_rounds: Prim<Count>,
    pub zones: Prim<Count>,
    pub zone_min_tiles: Prim<Count>,
    pub zone_max_tiles: Prim<Count>,
    pub mainland_floor: Prim<Count>,
    pub south_latitude: Prim<Fixed<1>>,
    pub metres_per_degree: Prim<Count>,
    pub terrain_elevation: Prim<Table1>,
    pub terrain_relief: Prim<Table1>,
    pub climate_lowland: Prim<Table2>,
    pub highland_elevation: Prim<Table1>,
    pub highland_class: Prim<Table1>,
    pub coast_m: Prim<Count>,
    pub weather: WeatherPrims,
    pub deposits: DepositPrims,
    pub hazards: Vec<HazardPrims>,
}

impl GeoPrims {
    pub fn declare(d: &mut Declarations) -> GeoPrims {
        let hazards = crate::hazards::HAZARDS
            .iter()
            .map(|h| {
                let [inland, coastal, river, rate, spread, severity_a, severity_b] = h.tables;
                HazardPrims {
                    inland: d.prim(inland),
                    coastal: d.prim(coastal),
                    river: d.prim(river),
                    rate: d.prim(rate),
                    spread: d.prim(spread),
                    severity_a: d.prim(severity_a),
                    severity_b: d.prim(severity_b),
                }
            })
            .collect();
        GeoPrims {
            tile_m: d.prim(&TILE_M),
            sea_share: d.prim(&SEA_SHARE),
            base_cells: d.prim(&BASE_CELLS),
            octaves: d.prim(&OCTAVES),
            roughness: d.prim(&ROUGHNESS),
            falloff: d.prim(&FALLOFF),
            relief_cells: d.prim(&RELIEF_CELLS),
            plates: d.prim(&PLATES),
            plate_belt: d.prim(&PLATE_BELT),
            plate_weight: d.prim(&PLATE_WEIGHT),
            mountain_weight: d.prim(&MOUNTAIN_WEIGHT),
            warp: d.prim(&WARP),
            erosion_passes: d.prim(&EROSION_PASSES),
            erosion_rate: d.prim(&EROSION_RATE),
            area_exponent: d.prim(&AREA_EXPONENT),
            land_heights: d.prim(&LAND_HEIGHTS),
            sea_depths: d.prim(&SEA_DEPTHS),
            river_tiles: d.prim(&RIVER_TILES),
            rugged_m: d.prim(&RUGGED_M),
            river_crossing_m: d.prim(&RIVER_CROSSING_M),
            share_tolerance: d.prim(&SHARE_TOLERANCE),
            partition_rounds: d.prim(&PARTITION_ROUNDS),
            zones: d.prim(&ZONES),
            zone_min_tiles: d.prim(&ZONE_MIN_TILES),
            zone_max_tiles: d.prim(&ZONE_MAX_TILES),
            mainland_floor: d.prim(&MAINLAND_FLOOR),
            south_latitude: d.prim(&SOUTH_LATITUDE),
            metres_per_degree: d.prim(&METRES_PER_DEGREE),
            terrain_elevation: d.prim(&TERRAIN_ELEVATION),
            terrain_relief: d.prim(&TERRAIN_RELIEF),
            climate_lowland: d.prim(&CLIMATE_LOWLAND),
            highland_elevation: d.prim(&HIGHLAND_ELEVATION),
            highland_class: d.prim(&HIGHLAND_CLASS),
            coast_m: d.prim(&COAST_M),
            weather: WeatherPrims {
                temp_mean: d.prim(&TEMP_MEAN),
                temp_sd: d.prim(&TEMP_SD),
                dry_share: d.prim(&DRY_SHARE),
                rain_shape: d.prim(&RAIN_SHAPE),
                rain_scale: d.prim(&RAIN_SCALE),
                wind_shape: d.prim(&WIND_SHAPE),
                wind_scale: d.prim(&WIND_SCALE),
                sun_a: d.prim(&SUN_A),
                sun_b: d.prim(&SUN_B),
                persistence: d.prim(&PERSISTENCE),
            },
            deposits: DepositPrims {
                density: d.prim(&DEPOSIT_DENSITY),
                grade_mu: d.prim(&GRADE_MU),
                grade_sigma: d.prim(&GRADE_SIGMA),
                quantity_mu: d.prim(&QUANTITY_MU),
                quantity_sigma: d.prim(&QUANTITY_SIGMA),
                unbounded: d.prim(&UNBOUNDED),
            },
            hazards,
        }
    }
}
