use phx_macros::clause;
use phx_rand::{Draws, Subject};

use crate::streams::{OpeningPhase, Purpose, StreamDecl, Streams};

/// The opening's context: its phase, and draws at the phase's own ordinal.
#[derive(Debug)]
pub struct OpeningCtx<'a> {
    streams: &'a Streams,
    phase: OpeningPhase,
}

impl<'a> OpeningCtx<'a> {
    #[must_use]
    pub fn new(streams: &'a Streams, phase: OpeningPhase) -> OpeningCtx<'a> {
        OpeningCtx { streams, phase }
    }

    #[must_use]
    pub fn phase(&self) -> OpeningPhase {
        self.phase
    }

    /// The draws of an opening stream for a subject, on the opening's day.
    #[must_use]
    pub fn draws(&self, stream: &StreamDecl, subject: Subject) -> Draws {
        if stream.purpose == Purpose::Observer {
            phx_num::violation!(clause = "Law 17", "the opening drawing from the observer's stream");
        }
        self.streams.open(stream, subject, phx_id::Day::new(0), self.phase.ordinal())
    }
}

/// A system's part of the opening: its phase, what it reads and writes, which sides it draws and which it derives.
#[clause("GEN.3")]
pub trait Contribution: Send + Sync {
    fn name(&self) -> &'static str;
    fn phase(&self) -> OpeningPhase;
    fn reads(&self) -> &'static [&'static str];
    fn writes(&self) -> &'static [&'static str];
    fn drawn(&self) -> &'static [&'static str];
    fn derived(&self) -> &'static [&'static str];
    fn contribute(&self, ctx: &mut OpeningCtx<'_>);
}
