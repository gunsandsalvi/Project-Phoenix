use phx_core::{Declarations, FactDef, HandlerTable, ItemDecl, StreamDef, System};

use crate::audit::facts::{Extracted, Opening, Remaining};
use crate::catastrophe::{CatastropheStream, Catastrophes};
use crate::deposits::DEPOSITS_STREAM;
use crate::hazards::HAZARDS;
use crate::state::MAP_STREAM;
use crate::weather::{LatentRain, LatentSunshine, LatentTemperature, LatentWind, VARIABLES, Weather, WeatherStream};

/// GEO's interface items: the regions' weather latents and the deposits' quantities.
pub const ITEMS: &[ItemDecl] = &[
    LatentTemperature::ITEM,
    LatentRain::ITEM,
    LatentWind::ITEM,
    LatentSunshine::ITEM,
    Opening::ITEM,
    Extracted::ITEM,
    Remaining::ITEM,
];

/// The map, its weather and its catastrophes. Its primitives are declared with the kernel's, since the opening reads
/// them before any system runs; its families are built with the map they read.
#[derive(Debug)]
pub struct Geo;

impl System for Geo {
    const CODE: &'static str = "GEO";

    fn declare(d: &mut Declarations) {
        for s in [MAP_STREAM, DEPOSITS_STREAM, WeatherStream::DECL, CatastropheStream::DECL] {
            d.stream(s);
        }
        for v in &VARIABLES {
            d.event(v.event);
        }
        for h in HAZARDS {
            d.hazard(h.decl);
            d.event(h.event);
        }
        for item in ITEMS {
            d.claim(item.name);
        }
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<Weather>();
        h.add::<Catastrophes>();
    }
}

/// The tables GEO keeps: a row per region for its weather, a row per country for its catastrophes, and a row per
/// finite deposit, its opening quantity written, nothing yet extracted and all of it remaining.
#[must_use]
pub fn tables(geo: &crate::GeoState, countries: u32) -> Vec<phx_core::KernelTable> {
    let facts_of = |table: &str| -> Vec<&'static str> {
        ITEMS
            .iter()
            .filter(|i| matches!(i.kind, phx_core::ItemKind::Fact(f) if f.kinds.contains(&table)))
            .map(|i| i.name)
            .collect()
    };
    let rows = |n: usize| {
        let Ok(r) = u32::try_from(n) else {
            phx_num::capacity_exceeded!("table rows", u32::MAX, n);
        };
        r
    };
    let finite: Vec<u64> = geo
        .deposits
        .iter()
        .filter_map(|d| match d.opening {
            crate::deposits::Opening::Finite(q) => Some(q),
            crate::deposits::Opening::Unbounded => None,
        })
        .collect();
    let mut deposits = phx_core::FactColumns::new(rows(finite.len()), &facts_of(crate::audit::DEPOSIT_TABLE));
    for (row, q) in (0_u32..).zip(&finite) {
        let Ok(q) = i64::try_from(*q) else {
            phx_num::capacity_exceeded!("a deposit's quantity", i64::MAX, *q);
        };
        let slot = phx_id::Slot::new(row);
        phx_core::FactStore::write(&mut deposits, Opening::ITEM.name, slot, q);
        phx_core::FactStore::write(&mut deposits, Extracted::ITEM.name, slot, 0);
        phx_core::FactStore::write(&mut deposits, Remaining::ITEM.name, slot, q);
    }
    vec![
        phx_core::KernelTable {
            name: "region",
            columns: phx_core::FactColumns::new(rows(geo.map.regions.len()), &facts_of("region")),
        },
        phx_core::KernelTable { name: "country", columns: phx_core::FactColumns::new(countries, &facts_of("country")) },
        phx_core::KernelTable { name: crate::audit::DEPOSIT_TABLE, columns: deposits },
    ]
}
