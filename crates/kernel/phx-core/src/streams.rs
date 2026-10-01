use phx_id::Day;
use phx_macros::clause;
use phx_num::violation;
pub use phx_rand::StreamFamily;
use phx_rand::key::fnv1a64;
use phx_rand::{Draws, Seed, SlotOrdinal, StreamKey, Subject, family_key};

use crate::consts::{KEYED_ORDINAL, OPENING_ORDINAL_BASE};
use crate::slots::DaySlot;

/// What chance is for: the world's declared purposes, and none for an outcome.
#[clause("CHN.3", "CHN.5")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Purpose {
    Mortality,
    Illness,
    /// The days in a year on which persons reach an age.
    Birthday,
    Conception,
    Accident,
    Damage,
    ThirdPartyHarm,
    Catastrophe,
    EquipmentFailure,
    /// Research and imitation.
    Discovery,
    Meeting,
    Weather,
    TypeAtBirth,
    SchedulePhase,
    Occasion,
    Taste,
    Pairing,
    Sample,
    /// The world's and the map's opening draws.
    Opening,
    /// The draws that place tracers, which touch no state of the world.
    Observer,
    Lot,
}

/// A named stream: one process's draws, of one family — the world's, the observer's or the player's advice's — whose
/// code alone opens it.
#[clause("CHN.1", "REP.16")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamDecl {
    pub name: &'static str,
    pub family: StreamFamily,
    pub purpose: Purpose,
    /// Drawn from its name and subject alone, recomputed whenever read, as a schedule's phase is.
    pub keyed: bool,
    pub clause: &'static str,
}

/// A stream declared as a type, so a handler names the streams it draws from.
pub trait StreamDef {
    const DECL: StreamDecl;
}

/// A step of the opening, whose draws carry an ordinal of their own beyond the day's sub-steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpeningPhase(pub u8);

impl OpeningPhase {
    #[must_use]
    pub fn ordinal(self) -> u8 {
        let Some(o) = OPENING_ORDINAL_BASE.checked_add(self.0).filter(|o| *o < KEYED_ORDINAL) else {
            phx_num::capacity_exceeded!("opening phases", KEYED_ORDINAL - OPENING_ORDINAL_BASE, self.0);
        };
        o
    }
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    decl: StreamDecl,
    key: StreamKey,
}

/// The run's streams, each keyed by its name and the one seed.
#[clause("CHN.1", "CHN.6", "CHN.8")]
#[derive(Debug)]
pub struct WorldStreams {
    entries: Vec<Entry>,
}

impl WorldStreams {
    /// The streams of a run, made as the world is assembled, each keyed by its family, its name and the seed.
    ///
    /// # Errors
    /// A name declared twice, in one family or two; two names whose FNV-1a hashes collide; or the observer's purpose
    /// outside the observer's family, or its family for another purpose.
    #[phx_macros::opening]
    pub fn new(seed: Seed, decls: &[StreamDecl]) -> Result<WorldStreams, Vec<String>> {
        let mut entries: Vec<Entry> =
            decls.iter().map(|d| Entry { decl: *d, key: family_key(seed, d.family, d.name) }).collect();
        entries.sort_unstable_by_key(|e| e.decl.name);
        let mut errors = Vec::new();
        for pair in entries.windows(2) {
            if let [a, b] = pair
                && a.decl.name == b.decl.name
            {
                if a.decl.family == b.decl.family {
                    errors.push(format!("stream `{}` declared twice", a.decl.name));
                } else {
                    errors.push(format!("stream `{}` declared in two families", a.decl.name));
                }
            }
        }
        // The observer's purpose is the observer's family's, and only its.
        for e in &entries {
            if (e.decl.purpose == Purpose::Observer) != (e.decl.family == StreamFamily::Observer) {
                errors.push(format!(
                    "stream `{}` of the observer's purpose outside its family, or of its family for another purpose",
                    e.decl.name
                ));
            }
        }
        let mut hashes: Vec<(u64, &str)> =
            entries.iter().map(|e| (fnv1a64(e.decl.name.as_bytes()), e.decl.name)).collect();
        hashes.sort_unstable();
        for pair in hashes.windows(2) {
            if let [(ha, a), (hb, b)] = pair
                && ha == hb
                && a != b
            {
                errors.push(format!("streams `{a}` and `{b}` share a hash"));
            }
        }
        if errors.is_empty() { Ok(WorldStreams { entries }) } else { Err(errors) }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// A declared stream by its name, as a market's declaration names the stream its lots are drawn from.
    #[must_use]
    pub fn named(&self, name: &str) -> Option<StreamDecl> {
        self.entries.binary_search_by_key(&name, |e| e.decl.name).ok().and_then(|i| self.entries.get(i)).map(|e| e.decl)
    }

    /// A world stream's entry; another family's declaration stops the run, since the world never draws from it.
    fn world(&self, decl: &StreamDecl) -> Entry {
        if decl.family != StreamFamily::World {
            violation!(clause = "REP.16", "the world opening a stream of another family");
        }
        self.entry(decl)
    }

    fn entry(&self, decl: &StreamDecl) -> Entry {
        let found =
            self.entries.binary_search_by_key(&decl.name, |e| e.decl.name).ok().and_then(|i| self.entries.get(i));
        let Some(e) = found.filter(|e| e.decl == *decl) else {
            violation!(clause = "CHN.1", "a stream the run did not declare");
        };
        *e
    }

    /// A day stream's key, which a meeting draws each round's tastes from by its buyers' subjects.
    pub fn key(&self, decl: &StreamDecl) -> StreamKey {
        let e = self.world(decl);
        if e.decl.keyed {
            violation!(clause = "CHN.6", "a keyed stream drawn by day");
        }
        e.key
    }

    /// The draws of a stream for a subject at a day's sub-step or an opening phase's ordinal: the one way to make
    /// draws.
    #[clause("CHN.1")]
    #[must_use]
    pub fn open(&self, decl: &StreamDecl, subject: Subject, day: Day, ordinal: u8) -> Draws {
        let e = self.world(decl);
        Self::by_day(e, subject, day, ordinal)
    }

    /// The draws of a world stream for a subject in a slot of the day.
    #[clause("CHN.1", "CHN.6")]
    #[must_use]
    pub fn open_at(&self, decl: &StreamDecl, subject: Subject, day: Day, slot: SlotOrdinal) -> Draws {
        Self::by_day(self.world(decl), subject, day, slot.get())
    }

    fn by_day(e: Entry, subject: Subject, day: Day, ordinal: u8) -> Draws {
        if e.decl.keyed {
            violation!(clause = "CHN.6", "a keyed stream opened by day");
        }
        Draws::new(e.key, subject, day.get(), ordinal)
    }

    /// A keyed stream's draws for a subject, the same whenever they are read.
    #[must_use]
    pub fn open_keyed(&self, decl: &StreamDecl, subject: Subject) -> Draws {
        let e = self.world(decl);
        if !e.decl.keyed {
            violation!(clause = "CHN.6", "a stream drawn by day opened as keyed");
        }
        Draws::new(e.key, subject, 0, KEYED_ORDINAL)
    }

    /// A keyed stream's draws for a subject and one of the things it is drawn for, by that thing's place: a cell's
    /// phase in each of its schedules is its own, and the same whenever read.
    #[must_use]
    pub fn open_keyed_at(&self, decl: &StreamDecl, subject: Subject, place: u32) -> Draws {
        let e = self.world(decl);
        if !e.decl.keyed {
            violation!(clause = "CHN.6", "a stream drawn by day opened as keyed");
        }
        Draws::new(e.key, subject, place, KEYED_ORDINAL)
    }
}

/// A stream opened for the observer, or the player's advice, that is not of its family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotObserver;

/// The observer's draws: only streams of the observer's family, so looking draws nothing the world draws.
#[clause("Law 17", "REP.16")]
#[derive(Debug)]
pub struct ObserverDraws<'a> {
    streams: &'a WorldStreams,
}

impl<'a> ObserverDraws<'a> {
    #[must_use]
    pub fn new(streams: &'a WorldStreams) -> ObserverDraws<'a> {
        ObserverDraws { streams }
    }

    /// # Errors
    /// When the stream is not the observer's.
    pub fn open(&self, decl: &StreamDecl, subject: Subject, day: Day) -> Result<Draws, NotObserver> {
        if decl.family != StreamFamily::Observer {
            return Err(NotObserver);
        }
        Ok(WorldStreams::by_day(self.streams.entry(decl), subject, day, DaySlot::S10e.ordinal().get()))
    }
}

/// The player's advice's draws, and the player's own — which household the player takes: only streams of the
/// advice's family, so advising or choosing the player draws nothing the world draws and the world nothing of theirs.
#[clause("REP.16")]
#[derive(Debug)]
pub struct AdviceDraws<'a> {
    streams: &'a WorldStreams,
}

impl<'a> AdviceDraws<'a> {
    #[must_use]
    pub fn new(streams: &'a WorldStreams) -> AdviceDraws<'a> {
        AdviceDraws { streams }
    }

    /// # Errors
    /// When the stream is not the advice's.
    pub fn open(&self, decl: &StreamDecl, subject: Subject, day: Day, ordinal: u8) -> Result<Draws, NotObserver> {
        if decl.family != StreamFamily::Advice {
            return Err(NotObserver);
        }
        Ok(WorldStreams::by_day(self.streams.entry(decl), subject, day, ordinal))
    }
}

#[cfg(test)]
#[path = "streams_tests.rs"]
mod tests;
