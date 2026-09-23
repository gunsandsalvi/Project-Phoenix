use std::sync::Arc;

use libm::{erfc, sqrt};
use phx_core::{
    Ctx, EventIntent, EventKindDecl, FactDef, FactStore, Reads, Writes, declare_fact, declare_handler, declare_stream,
};
use phx_id::Slot;
use phx_macros::clause;
use phx_num::{Fixed, Missing, Round, violation};
use phx_rand::{Draws, Subject, SubjectTag, normal};

use core::f64::consts::SQRT_2;

use crate::climate::Marginal;
use crate::consts::HALF;
use crate::state::GeoState;

declare_stream! { pub WeatherStream = "GEO.weather" { purpose: Weather, keyed: false, clause: "CHN.3" } }

declare_fact! {
    pub LatentTemperature = "GEO.latent_temperature" {
        value: Fixed { exp: 6 }, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3",
    }
}

declare_fact! {
    pub LatentRain = "GEO.latent_rain" {
        value: Fixed { exp: 6 }, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3",
    }
}

declare_fact! {
    pub LatentWind = "GEO.latent_wind" {
        value: Fixed { exp: 6 }, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3",
    }
}

declare_fact! {
    pub LatentSunshine = "GEO.latent_sunshine" {
        value: Fixed { exp: 6 }, kinds: ["region"], writer: "GEO", audience: Public, repr: Position, clause: "CHN.3",
    }
}

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
        event: EventKindDecl { name: "GEO.sunshine", size_unit: "permille of daylight", clause: "CHN.3" },
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

/// A latent as its fact's fixed-point value, and back.
fn to_fact(z: f64) -> i64 {
    let Ok(f) = Fixed::<6>::from_f64(z, Round::HalfEven) else {
        violation!(clause = "CHN.3", "a weather latent beyond its width");
    };
    f.raw()
}

fn from_fact(v: Missing<i64>) -> Missing<f64> {
    match v {
        Missing::Present(raw) => Missing::Present(Fixed::<6>::from_raw(raw).to_f64()),
        Missing::Absent => Missing::Absent,
    }
}

/// A variable's day: its latent moved on, written, and its value through the region's marginal.
fn variable<F: FactDef, S: FactStore + ?Sized>(
    ctx: &mut Ctx<'_, Weather, S>,
    row: Slot,
    d: &mut Draws,
    persistence: f64,
    marginal: &Marginal,
) -> f64
where
    Weather: Reads<F> + Writes<F>,
{
    let z = next_latent(from_fact(ctx.read::<F>(row)), persistence, normal(d));
    ctx.write::<F>(row, to_fact(z));
    marginal.quantile(normal_cdf(z))
}

/// A region's day of weather: each variable's latent moved on from the region's own draws, mapped through the month's
/// marginal, and recorded as a public event naming the region.
#[clause("CHN.3")]
fn day<S: FactStore + ?Sized>(ctx: &mut Ctx<'_, Weather, S>, row: Slot) {
    let geo: &GeoState = ctx.own::<Arc<GeoState>>();
    let Some(climate) = geo.regions.get(usize::try_from(row.get()).unwrap_or(usize::MAX)) else {
        violation!(clause = "CHN.3", "a region the map does not have", region = row.get());
    };
    let month = climate.month(ctx.date().month());
    let region = Subject::new(SubjectTag::Region, u64::from(row.get()));
    let mut d = ctx.draws::<WeatherStream>(region);
    let pairs: Vec<(f64, &Marginal)> = climate.persistence.iter().copied().zip(month).collect();
    let [(pt, mt), (pr, mr), (pw, mw), (ps, ms)] = pairs.as_slice() else {
        violation!(clause = "CHN.3", "a climate that does not declare every weather variable", declared = pairs.len());
    };
    let values = [
        variable::<LatentTemperature, S>(ctx, row, &mut d, *pt, mt),
        variable::<LatentRain, S>(ctx, row, &mut d, *pr, mr),
        variable::<LatentWind, S>(ctx, row, &mut d, *pw, mw),
        variable::<LatentSunshine, S>(ctx, row, &mut d, *ps, ms),
    ];
    for ((v, var), kind) in values.iter().zip(&VARIABLES).zip(&geo.weather_kinds) {
        let Ok(size) = Fixed::<0>::from_f64(v * var.units_per, Round::HalfEven) else {
            violation!(clause = "CHN.3", "a weather value beyond its width");
        };
        ctx.emit(&EventIntent {
            kind: *kind,
            subjects: vec![region],
            details: vec![(region, size.raw())],
            public: true,
        });
    }
}

declare_handler! {
    pub Weather = "GEO.weather" {
        substep: S3a,
        table: "region",
        reads: [LatentTemperature, LatentRain, LatentWind, LatentSunshine],
        writes: [LatentTemperature, LatentRain, LatentWind, LatentSunshine],
        intents: [EventIntent],
        streams: [WeatherStream],
        clause: "CHN.3",
        body: day,
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{StreamDef, Streams};
    use phx_id::Day;
    use phx_num::Missing;
    use phx_rand::{Seed, Subject, SubjectTag};

    use super::{WeatherStream, next_latent, normal_cdf};
    use crate::climate::Marginal;

    #[test]
    fn weather_marginals_and_persistence() {
        let streams = Streams::new(Seed::new(3), &[WeatherStream::DECL]).unwrap();
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
