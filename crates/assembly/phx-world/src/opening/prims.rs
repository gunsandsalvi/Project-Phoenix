use phx_core::register::values::{Table1, Table2};
use phx_core::{Declarations, JointProfile, Prim, Purpose, StreamDecl};
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

declare_prim! {
    /// A development level's joint profile of derived values, from its country group's published data; each country
    /// reads its level's.
    pub PROFILE = "GEN.profile" { kind: Endowment, value: Profile { exp: 6 }, clause: "GEN.12", scope: PerCountry }
}

declare_prim! {
    /// A currency's smallest units to the international dollar at the snapshot: the unit a country's amounts are
    /// counted in, each currency's unit worth a dollar on the opening day.
    pub UNITS_PER_DOLLAR = "GEN.units_per_dollar" { kind: Endowment, value: Count, clause: "MON.16", scope: PerCountry }
}

declare_prim! {
    /// What each activity uses of finance, real estate and public administration per unit of its output, and what those
    /// three use of every product, in value.
    pub SERVICE_INPUTS = "GEN.service_inputs" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// Taxes less subsidies on products per unit of each activity's output and of each final use's spending.
    pub PRODUCT_TAXES = "GEN.product_taxes" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// The shares of each activity's value added that are compensation and other taxes on production, the operating
    /// surplus and mixed income the rest.
    pub VALUE_ADDED_PARTS = "GEN.value_added_parts" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// Each final use's weight in GDP at purchasers' prices: households, collective consumption, fixed investment,
    /// changes in inventories.
    pub FINAL_WEIGHTS = "GEN.final_weights" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// What each final use spends on each activity, each use's parts summing to one.
    pub FINAL_COMPOSITION = "GEN.final_composition" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// The currency in circulation over GDP, households' part of it, households' and firms' parts of the deposits, the
    /// part of firms' debt borrowed from banks, and banks' part of the government's paper.
    pub HOLDINGS = "GEN.holdings" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// The real assets measured apart from plant, over GDP: firms' inventories and land, households' dwellings and
    /// land, the government's fixed assets.
    pub REAL_ASSETS = "GEN.real_assets" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// The spread of firms' log physical productivity around their industry's mean.
    pub PRODUCTIVITY_SPREAD = "GEN.productivity_spread" {
        kind: Endowment, value: Fixed { exp: 2 }, clause: "GEN.15", scope: PerCountry
    }
}

declare_prim! {
    /// Each occupation's mean earnings over all employees'.
    pub OCCUPATION_PAY = "GEN.occupation_pay" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 9 }, clause: "GEN.15", scope: PerCountry
    }
}

/// The stream a payer's ties among its flows of one payment order are drawn from at settlement.
pub const SETTLE_ORDER: StreamDecl =
    StreamDecl { name: "SET.order", purpose: Purpose::Lot, keyed: false, clause: "SET.6" };

/// The stream a new game's open choices, the regions' lot and the derived values are drawn from.
pub const SETUP_STREAM: StreamDecl =
    StreamDecl { name: "GEN.setup", purpose: Purpose::Opening, keyed: false, clause: "GEN.15" };

/// The stream the player's household is drawn from, and its members split out by.
pub const PLAYER_STREAM: StreamDecl =
    StreamDecl { name: "GEN.player", purpose: Purpose::Opening, keyed: false, clause: "OBS.4" };

/// The stream generated names are drawn from.
pub const NAMES_STREAM: StreamDecl =
    StreamDecl { name: "GEN.names", purpose: Purpose::Opening, keyed: false, clause: "GEN.14" };

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
    pub profile: Prim<JointProfile>,
    pub units_per_dollar: Prim<Count>,
    pub service_inputs: Prim<Table2>,
    pub product_taxes: Prim<Table1>,
    pub value_added_parts: Prim<Table2>,
    pub final_weights: Prim<Table1>,
    pub final_composition: Prim<Table2>,
    pub holdings: Prim<Table1>,
    pub real_assets: Prim<Table1>,
    pub productivity_spread: Prim<Fixed<2>>,
    pub occupation_pay: Prim<Table1>,
}

impl GenPrims {
    pub fn declare(d: &mut Declarations) -> GenPrims {
        d.stream(SETUP_STREAM);
        d.stream(SETTLE_ORDER);
        d.stream(NAMES_STREAM);
        d.stream(PLAYER_STREAM);
        GenPrims {
            population: d.prim(&POPULATION),
            countries: d.prim(&COUNTRIES),
            regions: d.prim(&REGIONS),
            regions_floor: d.prim(&REGIONS_FLOOR),
            land_tiles: d.prim(&LAND_TILES),
            share_floor: d.prim(&SHARE_FLOOR),
            share_ceiling: d.prim(&SHARE_CEILING),
            settling_years: d.prim(&SETTLING_YEARS),
            profile: d.prim(&PROFILE),
            units_per_dollar: d.prim(&UNITS_PER_DOLLAR),
            service_inputs: d.prim(&SERVICE_INPUTS),
            product_taxes: d.prim(&PRODUCT_TAXES),
            value_added_parts: d.prim(&VALUE_ADDED_PARTS),
            final_weights: d.prim(&FINAL_WEIGHTS),
            final_composition: d.prim(&FINAL_COMPOSITION),
            holdings: d.prim(&HOLDINGS),
            real_assets: d.prim(&REAL_ASSETS),
            productivity_spread: d.prim(&PRODUCTIVITY_SPREAD),
            occupation_pay: d.prim(&OCCUPATION_PAY),
        }
    }
}
