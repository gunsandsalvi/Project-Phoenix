//! The weather's store over hand-built climates: a row a region a day, kept to the horizon, the first day drawn from
//! the stationary law, the day's footprint one buffer, the store round-tripping and carrying on alike, and each
//! region's day its own draws'.
#![cfg(test)]

use phx_core::{StreamDef, WorldStreams};
use phx_id::{Day, TileId};
use phx_num::Missing;
use phx_rand::{Draws, Seed, Subject, SubjectTag};
use phx_store::{AddressSpace, HeapBacking, StoreStats};

use super::{VARIABLES, WeatherStore, WeatherStream};
use crate::catastrophe::{Footprint, Struck};
use crate::climate::{Marginal, RegionClimate};

type Heap = HeapBacking<4096>;

const REGIONS: usize = 3;

fn climates() -> Vec<RegionClimate> {
    let month = vec![
        Marginal::Normal { mean: 12.0, sd: 4.0 },
        Marginal::WetGamma { dry: 0.6, shape: 0.8, scale: 6.0 },
        Marginal::Weibull { shape: 2.0, scale: 5.0 },
        Marginal::Beta { a: 2.0, b: 3.0 },
    ];
    (0..REGIONS).map(|_| RegionClimate { months: vec![month.clone(); 12], persistence: vec![0.7; 4] }).collect()
}

fn draws(region: usize, day: Day) -> Draws {
    let streams = WorldStreams::new(Seed::new(4), &[WeatherStream::DECL]).unwrap();
    let subject = Subject::new(SubjectTag::Region, u64::try_from(region).unwrap());
    streams.open(&WeatherStream::DECL, subject, day, 0)
}

fn store(horizon: u32) -> WeatherStore<Heap> {
    WeatherStore::new(&mut AddressSpace::empty(), REGIONS, horizon, 8).unwrap()
}

fn run(s: &mut WeatherStore<Heap>, days: std::ops::RangeInclusive<u32>) {
    let c = climates();
    for d in days {
        let day = Day::new(d);
        let _ = s.record_day(day, (&c, 1), |r| draws(r, day));
    }
}

#[test]
fn one_row_per_region_per_day() {
    let mut s = store(30);
    run(&mut s, 1..=5);
    assert_eq!(s.rows_live(), 15);
    for r in 0..3 {
        let mut days = Vec::new();
        s.range(r, (Day::new(1), Day::new(5)), |d, _| days.push(d.get()));
        assert_eq!(days, [1, 2, 3, 4, 5], "region {r}");
    }
    assert_eq!(s.on(1, Day::new(9)), Missing::Absent, "a day not yet drawn");
}

#[test]
fn weather_bounded_by_horizon() {
    let mut s = store(20);
    run(&mut s, 1..=200);
    assert!(s.rows_live() <= u64::try_from(REGIONS).unwrap() * (20 + 2 * 8), "the horizon and a chunk either side");
    assert_eq!(s.on(0, Day::new(100)), Missing::Absent, "past the horizon, dropped by whole chunks");
    assert!(matches!(s.on(0, Day::new(185)), Missing::Present(_)), "within it, kept");
}

#[test]
fn first_day_from_stationary() {
    // Before its first day a region has no latent; its first day's latent is the shock itself, the stationary law.
    let mut s = store(30);
    assert_eq!(s.latents(0), [Missing::Absent; VARIABLES.len()]);
    run(&mut s, 1..=1);
    let mut d = draws(0, Day::new(1));
    let first = phx_rand::normal(&mut d);
    let Missing::Present(z) = s.latents(0)[0] else { panic!("drawn") };
    assert!((z - first).abs() < 1e-8, "the first latent is its first draw, kept to nine places");
}

#[test]
fn large_footprint_one_buffer() {
    let mut f: Footprint<Heap> = Footprint::new(&mut AddressSpace::empty(), "GEO.footprint", 50_000);
    for t in 0..40_000 {
        f.push(Struck { tile: TileId::new(t), severity: t % 1_000 });
    }
    assert_eq!(f.len(), 40_000, "a strike over forty thousand tiles is one buffer");
    f.clear();
    assert!(f.is_empty(), "and the next day starts it empty, its pages kept");
}

#[test]
fn weather_roundtrip_continues() {
    let mut s = store(30);
    run(&mut s, 1..=10);
    let (mut back, _) = phx_store::roundtrip(&s).unwrap();
    assert_eq!(back, s, "the ring's rows and the latents");
    run(&mut s, 11..=12);
    run(&mut back, 11..=12);
    assert_eq!(back, s, "the next days' weather is the same");
}

#[test]
fn weather_same_for_any_workers() {
    // Each region draws on its own (region, day) stream, so a region's weather does not depend on the others drawn.
    let mut all = store(30);
    run(&mut all, 1..=3);
    let mut one: WeatherStore<Heap> = WeatherStore::new(&mut AddressSpace::empty(), 1, 30, 8).unwrap();
    let c = climates();
    for d in 1..=3 {
        let day = Day::new(d);
        let _ = one.record_day(day, (c.get(..1).unwrap(), 1), |_| draws(0, day));
    }
    assert_eq!(one.on(0, Day::new(3)), all.on(0, Day::new(3)));
}
