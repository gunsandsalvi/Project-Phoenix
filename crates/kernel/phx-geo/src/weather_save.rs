//! The weather's store saved as its ring's live chunks and every region's latents, and compared row by row.

use phx_num::MaybeI64;
use phx_store::{Backing, HorizonRing, LoadError, Reader, Saved, Writer};

use crate::weather::{VARIABLES, WeatherStore};

impl<B: Backing> Saved for WeatherStore<B> {
    /// The ring's live chunks and every region's latents; the day's rows are the day's.
    fn save(&self, w: &mut Writer<'_>) {
        self.ring.save(w);
        w.count(self.latents.len());
        for l in &self.latents {
            w.rows(l, phx_store::Transform::Plain);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<WeatherStore<B>, LoadError> {
        let ring = HorizonRing::load(r)?;
        let regions = r.count()?;
        let mut latents = Vec::with_capacity(regions);
        for _ in 0..regions {
            let row: Vec<MaybeI64> = r.rows(phx_store::Transform::Plain)?;
            let Ok(row) = <[MaybeI64; VARIABLES.len()]>::try_from(row) else {
                return Err(LoadError::Invalid("a region's latents not one a variable".to_owned()));
            };
            latents.push(row);
        }
        Ok(WeatherStore { ring, latents, today: Vec::with_capacity(regions) })
    }
}

impl<B: Backing> PartialEq for WeatherStore<B> {
    /// Two stores alike in their latents and every row their rings hold.
    fn eq(&self, other: &WeatherStore<B>) -> bool {
        let rows = |s: &WeatherStore<B>| {
            let mut out = Vec::new();
            s.ring.range((0, u32::MAX), |_, d, r| out.extend(d.iter().copied().zip(r.iter().copied())));
            out
        };
        self.latents == other.latents && rows(self) == rows(other)
    }
}
