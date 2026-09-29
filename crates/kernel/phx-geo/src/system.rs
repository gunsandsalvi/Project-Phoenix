use phx_core::{Declarations, StreamDef, System};

use crate::catastrophe::CatastropheStream;
use crate::deposits::DEPOSITS_STREAM;
use crate::hazards::HAZARDS;
use crate::state::MAP_STREAM;
use crate::weather::{VARIABLES, WeatherStream};

/// The map, its weather and its catastrophes. Its primitives are declared with the kernel's, since the opening reads
/// them before any system runs.
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
    }
}
