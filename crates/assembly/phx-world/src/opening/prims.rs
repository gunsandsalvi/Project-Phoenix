use phx_core::{Declarations, Prim};
use phx_macros::declare_prim;
use phx_num::{Count, Fixed};

declare_prim! {
    /// The world's population, fixed for the simulation, which a setup splits among the countries.
    pub POPULATION = "GEN.population" { kind: Endowment, value: Count, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// How many countries the world holds.
    pub COUNTRIES = "GEN.countries" { kind: Endowment, value: Count, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// How many regions the countries share, allotted by the population split.
    pub REGIONS = "GEN.regions" { kind: Endowment, value: Count, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// The fewest regions a country holds.
    pub REGIONS_FLOOR = "GEN.regions_floor" { kind: Endowment, value: Count, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// The map's land, in tiles, which the countries share in their population shares.
    pub LAND_TILES = "GEN.land_tiles" { kind: Endowment, value: Count, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// The least share of the population a setup may give a country.
    pub SHARE_FLOOR = "GEN.share_floor" { kind: Endowment, value: Fixed { exp: 2 }, clause: "GEN.14", scope: Shared }
}

declare_prim! {
    /// The greatest share of the population a setup may give a country.
    pub SHARE_CEILING = "GEN.share_ceiling" {
        kind: Endowment, value: Fixed { exp: 2 }, clause: "GEN.14", scope: Shared
    }
}

declare_prim! {
    /// How many simulated years the world settles by its own mechanisms before play.
    pub SETTLING_YEARS = "GEN.settling_years" { kind: Endowment, value: Count, clause: "GEN.6", scope: Shared }
}

/// The generator's constants, declared with the kernel's before any system's.
#[derive(Debug)]
pub struct GenPrims {
    pub population: Prim<Count>,
    pub countries: Prim<Count>,
    pub regions: Prim<Count>,
    pub regions_floor: Prim<Count>,
    pub land_tiles: Prim<Count>,
    pub share_floor: Prim<Fixed<2>>,
    pub share_ceiling: Prim<Fixed<2>>,
    pub settling_years: Prim<Count>,
}

impl GenPrims {
    pub fn declare(d: &mut Declarations) -> GenPrims {
        GenPrims {
            population: d.prim(&POPULATION),
            countries: d.prim(&COUNTRIES),
            regions: d.prim(&REGIONS),
            regions_floor: d.prim(&REGIONS_FLOOR),
            land_tiles: d.prim(&LAND_TILES),
            share_floor: d.prim(&SHARE_FLOOR),
            share_ceiling: d.prim(&SHARE_CEILING),
            settling_years: d.prim(&SETTLING_YEARS),
        }
    }
}
