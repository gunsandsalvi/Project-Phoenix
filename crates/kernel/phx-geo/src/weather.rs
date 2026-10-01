use libm::{erfc, sqrt};
use phx_core::{EventKindDecl, declare_stream};
use phx_id::Day;
use phx_macros::{Pod, clause};
use phx_num::{Fixed, MaybeI64, Missing, Round, capacity_exceeded, violation};
use phx_rand::{Draws, normal};
use phx_store::{AddressSpace, Backing, HorizonRing, StoreStats, SystemBacking};

use core::f64::consts::SQRT_2;

use crate::consts::HALF;

declare_stream! { pub WeatherStream = "GEO.weather" { family: World, purpose: Weather, keyed: false, clause: "CHN.3" } }

/// A weather variable: the event its day's value is recorded as, and how many of the event's units make one of the
/// marginal's.
#[derive(Clone, Copy, Debug)]
pub struct Variable {
    pub event: EventKindDecl,
    pub units_per: f64,
}

/// The weather's variables, in the order the climate's marginals and persistence list them.
pub const VARIABLES: [Variable; 4] = [
    Variable {
        event: EventKindDecl { name: "GEO.temperature", size_unit: "0.1 degC", clause: "CHN.3" },
        units_per: crate::consts::TENTHS,
    },
    Variable {
        event: EventKindDecl { name: "GEO.rain", size_unit: "0.1 mm", clause: "CHN.3" },
        units_per: crate::consts::TENTHS,
    },
    Variable {
        event: EventKindDecl { name: "GEO.wind", size_unit: "0.1 m/s", clause: "CHN.3" },
        units_per: crate::consts::TENTHS,
    },
    Variable {
        event: EventKindDecl { name: "GEO.sunshine", size_unit: "permille of clear-sky irradiance", clause: "CHN.3" },
        units_per: crate::consts::PER_MILLE_F64,
    },
];

/// The standard normal's distribution function.
#[must_use]
pub fn normal_cdf(z: f64) -> f64 {
    HALF * erfc(-z / SQRT_2)
}

/// The next day's latent: the AR(1) step with the declared persistence, whose stationary law stays the standard
/// normal; a region with no latent yet starts from that law.
#[clause("CHN.3")]
#[must_use]
pub fn next_latent(latent: Missing<f64>, persistence: f64, shock: f64) -> f64 {
    match latent {
        Missing::Present(z) => persistence * z + sqrt(1.0 - persistence * persistence) * shock,
        Missing::Absent => shock,
    }
}

/// A region's latents, one a weather variable in `VARIABLES`' order; absent before its first day.
pub type Latents = [Missing<f64>; VARIABLES.len()];

/// A region's day of weather: each variable's latent moved on by its declared persistence from the region's own draws,
/// in `VARIABLES`' order, and its value through the month's marginal, in its event's units.
#[clause("CHN.3")]
pub fn region_day(climate: &crate::climate::RegionClimate, month: u8, latents: &mut Latents, d: &mut Draws) -> Latents {
    let marginals = climate.month(month);
    if marginals.len() != VARIABLES.len() || climate.persistence.len() != VARIABLES.len() {
        violation!(
            clause = "CHN.3",
            "a climate that does not declare every weather variable",
            declared = marginals.len()
        );
    }
    let mut out = [Missing::Absent; VARIABLES.len()];
    let steps = latents.iter_mut().zip(&climate.persistence).zip(marginals).zip(&VARIABLES).zip(out.iter_mut());
    for ((((latent, phi), marginal), var), o) in steps {
        let z = next_latent(*latent, *phi, normal(d));
        *latent = Missing::Present(z);
        *o = Missing::Present(marginal.quantile(normal_cdf(z)) * var.units_per);
    }
    out
}

/// A region's day of weather as the ring keeps it: the region and each variable's value in its event's units, whole,
/// in `VARIABLES`' order; the ring keeps its day.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct WeatherRow {
    region: u16,
    pad: u16,
    values: [i32; VARIABLES.len()],
}

impl WeatherRow {
    #[must_use]
    pub fn region(&self) -> u16 {
        self.region
    }

    #[must_use]
    pub fn values(&self) -> [i32; VARIABLES.len()] {
        self.values
    }
}

/// A latent as the store keeps it: a fixed-point number, a store holding no float.
type Latent = Fixed<9>;

/// The weather's history and state: each region's days of weather in a ring kept to its horizon, each region's
/// latents, the one piece of weather state carried from day to day, and the day's rows as they are drawn.
#[clause("CHN.3", "SET.13")]
#[derive(Debug)]
pub struct WeatherStore<B: Backing = SystemBacking> {
    pub(crate) ring: HorizonRing<WeatherRow, B>,
    pub(crate) latents: Vec<[MaybeI64; VARIABLES.len()]>,
    pub(crate) today: Vec<WeatherRow>,
}

fn latent_of(z: MaybeI64) -> Missing<f64> {
    match z.get() {
        Missing::Present(raw) => Missing::Present(Latent::from_raw(raw).to_f64()),
        Missing::Absent => Missing::Absent,
    }
}

fn whole(v: f64) -> i32 {
    match Fixed::<0>::from_f64(v, Round::HalfEven).map(|f| i32::try_from(f.raw())) {
        Ok(Ok(v)) => v,
        _ => violation!(clause = "CHN.3", "a weather value beyond its width"),
    }
}

impl<B: Backing> WeatherStore<B> {
    /// No weather yet for `regions` regions, kept `horizon` days in chunks of `chunk_days` days.
    ///
    /// # Errors
    /// A ring of no rows or past a slot's width.
    #[phx_macros::opening]
    pub fn new(
        space: &mut AddressSpace,
        regions: usize,
        horizon: u32,
        chunk_days: u32,
    ) -> Result<WeatherStore<B>, String> {
        let per_day = u32::try_from(regions).map_err(|e| e.to_string())?;
        let chunk_rows = per_day.checked_mul(chunk_days).ok_or("a weather chunk past a slot's width")?;
        // The horizon's chunks, one being filled and one the prune has yet to drop.
        let chunks = horizon.div_ceil(chunk_days) + 2;
        Ok(WeatherStore {
            ring: HorizonRing::new(space, (chunk_rows, chunks), horizon)?,
            latents: vec![[MaybeI64::ABSENT; VARIABLES.len()]; regions],
            today: Vec::with_capacity(regions),
        })
    }

    /// A day's weather in every region, in region order, each from its own draws: its latents moved on and its
    /// values appended to the ring, which then drops what lies past its horizon. The rows written.
    #[clause("CHN.3", "SET.13")]
    pub fn record_day(
        &mut self,
        day: Day,
        (climates, month): (&[crate::climate::RegionClimate], u8),
        mut draws: impl FnMut(usize) -> Draws,
    ) -> u64 {
        if climates.len() != self.latents.len() {
            violation!(clause = "CHN.3", "weather for other regions than the store's", regions = climates.len());
        }
        self.today.clear();
        for (r, (climate, stored)) in climates.iter().zip(self.latents.iter_mut()).enumerate() {
            let mut latents: Latents = [Missing::Absent; VARIABLES.len()];
            for (l, z) in latents.iter_mut().zip(stored.iter()) {
                *l = latent_of(*z);
            }
            let values = region_day(climate, month, &mut latents, &mut draws(r));
            for (z, l) in stored.iter_mut().zip(latents) {
                if let Missing::Present(l) = l {
                    let Ok(fixed) = Latent::from_f64(l, Round::HalfEven) else {
                        violation!(clause = "CHN.3", "a latent beyond its width");
                    };
                    *z = MaybeI64::present(fixed.raw());
                }
            }
            let mut row = WeatherRow { region: region(r), pad: 0, values: [0; VARIABLES.len()] };
            for (v, value) in row.values.iter_mut().zip(values) {
                let Missing::Present(value) = value else {
                    violation!(clause = "CHN.3", "a weather variable with no value", region = r);
                };
                *v = whole(value);
            }
            self.today.push(row);
        }
        let _ = self.ring.append(day.get(), &self.today);
        let _ = self.ring.prune(day.get());
        u64::try_from(self.today.len()).unwrap_or(u64::MAX)
    }

    /// A region's latents as the next day moves them on; absent before its first day.
    pub fn latents(&self, region: usize) -> Latents {
        let mut out = [Missing::Absent; VARIABLES.len()];
        if let Some(stored) = self.latents.get(region) {
            for (o, z) in out.iter_mut().zip(stored) {
                *o = latent_of(*z);
            }
        }
        out
    }

    /// A region's weather over a window of days, each day's values in `VARIABLES`' order, read from the ring.
    pub fn range(&self, region: u16, (from, to): (Day, Day), mut each: impl FnMut(Day, [i32; VARIABLES.len()])) {
        self.ring.range((from.get(), to.get()), |_, days, rows| {
            for (d, row) in days.iter().zip(rows).filter(|(_, row)| row.region == region) {
                each(Day::new(*d), row.values);
            }
        });
    }

    /// A region's weather on a day, or `Missing` for a day the ring does not hold.
    pub fn on(&self, region: u16, day: Day) -> Missing<[i32; VARIABLES.len()]> {
        let mut found = Missing::Absent;
        self.range(region, (day, day), |_, v| found = Missing::Present(v));
        found
    }
}

fn region(r: usize) -> u16 {
    match u16::try_from(r) {
        Ok(r) => r,
        Err(_) => capacity_exceeded!("weather regions", u16::MAX, r),
    }
}

impl<B: Backing> StoreStats for WeatherStore<B> {
    fn rows_live(&self) -> u64 {
        self.ring.rows_live()
    }

    fn rows_ever(&self) -> u64 {
        self.ring.rows_ever()
    }

    fn bytes(&self) -> u64 {
        let latents = self.latents.capacity() * size_of::<[MaybeI64; VARIABLES.len()]>();
        self.ring.bytes() + u64::try_from(latents).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
#[path = "weather_tests.rs"]
mod store_tests;

#[cfg(test)]
mod tests {
    use phx_core::{StreamDef, WorldStreams};
    use phx_id::Day;
    use phx_num::Missing;
    use phx_rand::{Seed, Subject, SubjectTag};

    use super::{WeatherStream, next_latent, normal_cdf};
    use crate::climate::Marginal;

    #[test]
    fn weather_marginals_and_persistence() {
        let streams = WorldStreams::new(Seed::new(3), &[WeatherStream::DECL]).unwrap();
        let mut d = streams.open(&WeatherStream::DECL, Subject::new(SubjectTag::Region, 0), Day::new(1), 0);
        let (phi, n) = (0.7, 100_000_u32);
        let count = f64::from(n);
        let marginal = Marginal::WetGamma { dry: 0.55, shape: 0.9, scale: 5.0 };
        let mut z = Missing::Absent;
        let (mut zs, mut dry) = (Vec::new(), 0_u32);
        for _ in 0..n {
            let next = next_latent(z, phi, phx_rand::normal(&mut d));
            if marginal.quantile(normal_cdf(next)).to_bits() == 0.0_f64.to_bits() {
                dry += 1;
            }
            zs.push(next);
            z = Missing::Present(next);
        }
        let mean = zs.iter().sum::<f64>() / count;
        let var = zs.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / count;
        let lag = zs.windows(2).map(|w| (w[0] - mean) * (w[1] - mean)).sum::<f64>() / (count - 1.0) / var;
        assert!(mean.abs() < 0.03 && (var - 1.0).abs() < 0.03, "the latent stays standard normal: {mean} {var}");
        assert!((lag - phi).abs() < 0.01, "persistence is the declared autocorrelation: {lag}");
        let share = f64::from(dry) / count;
        assert!((share - 0.55).abs() < 0.015, "dry days at their declared share: {share}");
    }
}
