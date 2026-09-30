//! The harness's draws: one stream a base, `fin.<base>`, keyed by the design point's seed, so a base's fill is the
//! same on every run and every machine.

use phx_rand::draws::Draws;
use phx_rand::key::{Seed, StreamKey, Subject, SubjectTag, stream_key};

/// The streams a fill draws from.
#[derive(Debug, Clone, Copy)]
pub struct Streams {
    seed: u64,
}

impl Streams {
    #[must_use]
    pub fn new(seed: u64) -> Streams {
        Streams { seed }
    }

    /// A base's stream.
    pub fn key(&self, base: &str) -> StreamKey {
        stream_key(Seed::new(self.seed), &format!("fin.{base}"))
    }

    /// A row's draws on a day, from one base's stream.
    #[must_use]
    pub fn draws(&self, base: &str, row: u64, day: u32) -> Draws {
        Draws::new(self.key(base), Subject::new(SubjectTag::World, row), day, 0)
    }
}
