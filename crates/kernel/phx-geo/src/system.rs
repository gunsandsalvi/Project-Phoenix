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
