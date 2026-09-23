use phx_core::{PrimDecl, Register, Table1, Table2, ValueType};
use phx_macros::clause;
use phx_num::violation;

use crate::consts::{METRES_PER_KM, MONTHS, PER_MILLE};
use crate::generate::{ClimateInput, Map};
use crate::prims::{
    DRY_SHARE, GeoPrims, PERSISTENCE, RAIN_SCALE, RAIN_SHAPE, SUN_A, SUN_B, TEMP_MEAN, TEMP_SD, WIND_SCALE, WIND_SHAPE,
};

/// A declared table's value as a number, its decimals undone.
#[must_use]
pub fn scaled(raw: i64, exp: u32) -> f64 {
    phx_rand::float::from_i64(raw) / libm::pow(phx_core::consts::DECIMAL_RADIX_F64, f64::from(exp))
}

/// The decimals a table's values are declared to.
#[must_use]
pub fn exp_of(decl: &PrimDecl) -> u32 {
    match decl.value {
        ValueType::Table1 { exp, .. } | ValueType::Table2 { exp, .. } => u32::from(exp),
        _ => violation!(clause = "NUM.3", "a table read from a primitive that is not one"),
    }
}

/// A table read at a point its data must cover.
pub(crate) fn cell2(table: &Table2, row: i64, column: i64) -> i64 {
    let Ok(v) = table.at(row, column) else {
        violation!(clause = "NUM.3", "a table that does not cover the map", row = row, column = column);
    };
    v
}

pub(crate) fn cell1(table: &Table1, x: i64) -> i64 {
    let Ok(v) = table.at(x) else {
        violation!(clause = "NUM.3", "a climate table that does not cover a tile", at = x);
    };
    v
}

/// How a tile's climate class is read: its latitude on the owner's projection, from the map's south edge to its
/// north, the lowland class by latitude and distance to the sea, and the latitude's highland class above the
/// elevation it declares.
#[clause("GEO.7")]
#[derive(Clone, Debug)]
pub struct ClimateRule {
    south_tenths: i64,
    north_tenths: i64,
    lowland: Table2,
    highland: Table1,
    highland_class: Table1,
}

impl ClimateRule {
    #[must_use]
    pub fn read(p: &GeoPrims, r: &Register) -> ClimateRule {
        ClimateRule {
            south_tenths: p.south_latitude.shared(r).raw(),
            north_tenths: p.north_latitude.shared(r).raw(),
            lowland: p.climate_lowland.shared(r).clone(),
            highland: p.highland_elevation.shared(r).clone(),
            highland_class: p.highland_class.shared(r).clone(),
        }
    }

    /// The latitude, in tenths of a degree, at a position across the map from south to north.
    #[must_use]
    pub fn latitude_tenths(&self, north_permille: u32) -> i64 {
        let per_mille = i64::try_from(PER_MILLE).unwrap_or(i64::MAX);
        self.south_tenths + i64::from(north_permille) * (self.north_tenths - self.south_tenths) / per_mille
    }

    /// A tile's climate class.
    #[must_use]
    pub fn class(&self, input: ClimateInput) -> u8 {
        let latitude = self.latitude_tenths(input.north_permille);
        let class = if i64::from(input.elevation_m) > cell1(&self.highland, latitude) {
            cell1(&self.highland_class, latitude)
        } else {
            let km = i64::try_from(input.sea_distance_m / METRES_PER_KM).unwrap_or(i64::MAX);
            cell2(&self.lowland, latitude, km)
        };
        let Ok(c) = u8::try_from(class) else {
            violation!(clause = "NUM.3", "a climate class beyond a byte", class = class);
        };
        c
    }
}

/// A weather variable's marginal for one region and month.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Marginal {
    /// Temperature in °C.
    Normal { mean: f64, sd: f64 },
    /// Rainfall in millimetres: none on a dry day, a gamma on a wet one.
    WetGamma { dry: f64, shape: f64, scale: f64 },
    /// Mean wind speed in metres a second.
    Weibull { shape: f64, scale: f64 },
    /// Sunshine as the day's clear-sky index.
    Beta { a: f64, b: f64 },
}

impl Marginal {
    /// The marginal's mean and variance.
    #[must_use]
    pub fn moments(&self) -> (f64, f64) {
        match *self {
            Marginal::Normal { mean, sd } => (mean, sd * sd),
            Marginal::WetGamma { dry, shape, scale } => {
                let wet = 1.0 - dry;
                let mean = wet * shape * scale;
                (mean, wet * shape * scale * scale * (shape + 1.0) - mean * mean)
            }
            Marginal::Weibull { shape, scale } => {
                let first = libm::tgamma(1.0 + 1.0 / shape);
                (scale * first, scale * scale * (libm::tgamma(1.0 + 2.0 / shape) - first * first))
            }
            Marginal::Beta { a, b } => (a / (a + b), a * b / ((a + b) * (a + b) * (a + b + 1.0))),
        }
    }

    /// The value at a probability: the quantile of the declared marginal.
    #[must_use]
    pub fn quantile(&self, u: f64) -> f64 {
        match *self {
            Marginal::Normal { mean, sd } => mean + sd * phx_rand::normal_quantile(u),
            Marginal::WetGamma { dry, shape, scale } => {
                if u <= dry {
                    0.0
                } else {
                    phx_rand::gamma_quantile((u - dry) / (1.0 - dry), shape, scale)
                }
            }
            Marginal::Weibull { shape, scale } => phx_rand::weibull_quantile(u, shape, scale),
            Marginal::Beta { a, b } => phx_rand::beta_quantile(u, a, b),
        }
    }
}

/// A region's climate: each month's marginals, in the weather's variable order, and each variable's persistence.
#[clause("CHN.3")]
#[derive(Clone, Debug, PartialEq)]
pub struct RegionClimate {
    pub months: Vec<Vec<Marginal>>,
    pub persistence: Vec<f64>,
}

impl RegionClimate {
    /// A month's marginals, the month numbered from one.
    #[must_use]
    pub fn month(&self, month: u8) -> &[Marginal] {
        let Some(m) = usize::from(month).checked_sub(1).and_then(|i| self.months.get(i)) else {
            violation!(clause = "CHN.3", "a month the climate does not have", month = month);
        };
        m
    }
}

/// Each region's climate: its tiles' classes' parameters weighted by their area, which is their count.
#[clause("CHN.3")]
#[must_use]
pub fn regions(p: &GeoPrims, r: &Register, map: &Map) -> Vec<RegionClimate> {
    let w = &p.weather;
    let mut counts = vec![Vec::<u64>::new(); map.regions.len()];
    for tile in &map.tiles {
        let Some(zone) = tile.zone().and_then(|z| map.zones.get(usize::try_from(z.get()).ok()?)) else { continue };
        if let Some(c) = counts.get_mut(usize::from(zone.region.get())) {
            let class = usize::from(tile.climate);
            if c.len() <= class {
                c.resize(class + 1, 0);
            }
            if let Some(n) = c.get_mut(class) {
                *n += 1;
            }
        }
    }
    let weighted = |counts: &[u64], table: &Table2, decl: &PrimDecl, column: i64| -> f64 {
        let total: u64 = counts.iter().sum();
        let sum: f64 = counts
            .iter()
            .zip(0_i64..)
            .filter(|(n, _)| **n > 0)
            .map(|(n, class)| phx_rand::float::from_u64(*n) * scaled(cell2(table, class, column), exp_of(decl)))
            .sum();
        sum / phx_rand::float::from_u64(total)
    };
    counts
        .iter()
        .map(|c| {
            let months = (1..=MONTHS)
                .map(|m| {
                    let at = |t: &phx_core::Prim<Table2>, decl| weighted(c, t.shared(r), decl, m);
                    vec![
                        Marginal::Normal { mean: at(&w.temp_mean, &TEMP_MEAN), sd: at(&w.temp_sd, &TEMP_SD) },
                        Marginal::WetGamma {
                            dry: at(&w.dry_share, &DRY_SHARE),
                            shape: at(&w.rain_shape, &RAIN_SHAPE),
                            scale: at(&w.rain_scale, &RAIN_SCALE),
                        },
                        Marginal::Weibull {
                            shape: at(&w.wind_shape, &WIND_SHAPE),
                            scale: at(&w.wind_scale, &WIND_SCALE),
                        },
                        Marginal::Beta { a: at(&w.sun_a, &SUN_A), b: at(&w.sun_b, &SUN_B) },
                    ]
                })
                .collect();
            let persistence = (0..crate::weather::VARIABLES.len())
                .map(|v| weighted(c, w.persistence.shared(r), &PERSISTENCE, v_i64(v)))
                .collect();
            RegionClimate { months, persistence }
        })
        .collect()
}

fn v_i64(v: usize) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{Marginal, scaled};

    #[test]
    fn marginals_map_probabilities() {
        assert!((scaled(125, 1) - 12.5).abs() < 1e-12);
        assert_eq!(super::exp_of(&crate::prims::DRY_SHARE), 3);
        let rain = Marginal::WetGamma { dry: 0.6, shape: 0.8, scale: 6.0 };
        assert_eq!(rain.quantile(0.3).to_bits(), 0.0_f64.to_bits(), "a dry day");
        assert!(rain.quantile(0.9) > 0.0);
        let t = Marginal::Normal { mean: 10.0, sd: 3.0 };
        assert!((t.quantile(0.5) - 10.0).abs() < 1e-12);
        let s = Marginal::Beta { a: 2.0, b: 2.0 };
        assert!((s.quantile(0.5) - 0.5).abs() < 1e-12, "a symmetric beta's median");
    }
}
