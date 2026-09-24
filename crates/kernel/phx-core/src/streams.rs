use phx_id::Day;
use phx_macros::clause;
use phx_num::violation;
use phx_rand::key::fnv1a64;
use phx_rand::{Draws, Seed, StreamKey, Subject, stream_key};

use crate::consts::{KEYED_ORDINAL, OPENING_ORDINAL_BASE};
use crate::substep::SubStep;

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

/// A named stream: one process's draws.
#[clause("CHN.1")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamDecl {
    pub name: &'static str,
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
#[clause("CHN.1", "CHN.6")]
#[derive(Debug)]
pub struct Streams {
    entries: Vec<Entry>,
}

impl Streams {
    /// The streams of a run.
    ///
    /// # Errors
    /// A name declared twice, or two names whose FNV-1a hashes collide.
    pub fn new(seed: Seed, decls: &[StreamDecl]) -> Result<Streams, Vec<String>> {
        let mut entries: Vec<Entry> = decls.iter().map(|d| Entry { decl: *d, key: stream_key(seed, d.name) }).collect();
        entries.sort_unstable_by_key(|e| e.decl.name);
        let mut errors = Vec::new();
        for pair in entries.windows(2) {
            if let [a, b] = pair
                && a.decl.name == b.decl.name
            {
                errors.push(format!("stream `{}` declared twice", a.decl.name));
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
        if errors.is_empty() { Ok(Streams { entries }) } else { Err(errors) }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn entry(&self, decl: &StreamDecl) -> Entry {
        let found =
            self.entries.binary_search_by_key(&decl.name, |e| e.decl.name).ok().and_then(|i| self.entries.get(i));
        let Some(e) = found.filter(|e| e.decl == *decl) else {
            violation!(clause = "CHN.1", "a stream the run did not declare");
        };
        *e
    }

    /// The draws of a stream for a subject at a day's sub-step or an opening phase's ordinal: the one way to make
    /// draws.
    #[clause("CHN.1")]
    #[must_use]
    pub fn open(&self, decl: &StreamDecl, subject: Subject, day: Day, ordinal: u8) -> Draws {
        let e = self.entry(decl);
        if e.decl.keyed {
            violation!(clause = "CHN.6", "a keyed stream opened by day");
        }
        Draws::new(e.key, subject, day.get(), ordinal)
    }

    /// A keyed stream's draws for a subject, the same whenever they are read.
    #[must_use]
    pub fn open_keyed(&self, decl: &StreamDecl, subject: Subject) -> Draws {
        let e = self.entry(decl);
        if !e.decl.keyed {
            violation!(clause = "CHN.6", "a stream drawn by day opened as keyed");
        }
        Draws::new(e.key, subject, 0, KEYED_ORDINAL)
    }

    /// A keyed stream's draws for a subject and one of the things it is drawn for, by that thing's place: a cell's
    /// phase in each of its schedules is its own, and the same whenever read.
    #[must_use]
    pub fn open_keyed_at(&self, decl: &StreamDecl, subject: Subject, place: u32) -> Draws {
        let e = self.entry(decl);
        if !e.decl.keyed {
            violation!(clause = "CHN.6", "a stream drawn by day opened as keyed");
        }
        Draws::new(e.key, subject, place, KEYED_ORDINAL)
    }
}

/// A stream opened for the observer that is not the observer's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NotObserver;

/// The observer's draws: only streams of the observer's purpose, so looking draws nothing the world draws.
#[clause("Law 17")]
#[derive(Debug)]
pub struct ObserverDraws<'a> {
    streams: &'a Streams,
}

impl<'a> ObserverDraws<'a> {
    #[must_use]
    pub fn new(streams: &'a Streams) -> ObserverDraws<'a> {
        ObserverDraws { streams }
    }

    /// # Errors
    /// When the stream is not the observer's.
    pub fn open(&self, decl: &StreamDecl, subject: Subject, day: Day) -> Result<Draws, NotObserver> {
        if decl.purpose != Purpose::Observer {
            return Err(NotObserver);
        }
        Ok(self.streams.open(decl, subject, day, SubStep::S10e.ordinal()))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_rand::{Seed, Subject, SubjectTag, open_unit, stream_key};

    use super::{NotObserver, ObserverDraws, Purpose, StreamDecl, Streams};

    const fn stream(name: &'static str, purpose: Purpose) -> StreamDecl {
        StreamDecl { name, purpose, keyed: false, clause: "CHN.3" }
    }

    #[test]
    fn stream_names_unique_and_fnv_distinct() {
        let seed = Seed::new(1);
        assert!(
            Streams::new(seed, &[stream("DEM.mortality", Purpose::Mortality), stream("DEM.illness", Purpose::Illness)])
                .is_ok()
        );
        let twice = [stream("DEM.mortality", Purpose::Mortality), stream("DEM.mortality", Purpose::Mortality)];
        assert!(Streams::new(seed, &twice).is_err());
    }

    #[test]
    fn adding_a_stream_changes_no_other_key() {
        let seed = Seed::new(7);
        let names = ["DEM.mortality", "DEM.illness", "GEO.weather"];
        let before: Vec<_> = names.iter().map(|n| stream_key(seed, n)).collect();
        let decls: Vec<StreamDecl> =
            names.iter().chain(&["TEC.discovery"]).map(|n| stream(n, Purpose::Discovery)).collect();
        let with_one_more = Streams::new(seed, &decls).unwrap();
        let subject = Subject::new(SubjectTag::Party, 5);
        for (d, key) in decls.iter().zip(before) {
            let mut a = with_one_more.open(d, subject, Day::new(3), 4);
            let mut b = phx_rand::Draws::new(key, subject, 3, 4);
            assert_eq!(open_unit(&mut a).to_bits(), open_unit(&mut b).to_bits(), "{}", d.name);
        }
    }

    #[test]
    fn observer_draws_refuse_world_streams() {
        let tracer = stream("OBS.tracer", Purpose::Observer);
        let mortality = stream("DEM.mortality", Purpose::Mortality);
        let streams = Streams::new(Seed::new(1), &[tracer, mortality]).unwrap();
        let observer = ObserverDraws::new(&streams);
        let subject = Subject::new(SubjectTag::Party, 1);
        assert!(observer.open(&tracer, subject, Day::new(1)).is_ok());
        assert_eq!(observer.open(&mortality, subject, Day::new(1)).err(), Some(NotObserver));
    }

    #[test]
    fn keyed_streams_are_the_same_whenever_read() {
        let phase =
            StreamDecl { name: "TIME.schedule_phase", purpose: Purpose::SchedulePhase, keyed: true, clause: "TIME.5" };
        let streams = Streams::new(Seed::new(2), &[phase]).unwrap();
        let subject = Subject::new(SubjectTag::Party, 9);
        let (mut a, mut b) = (streams.open_keyed(&phase, subject), streams.open_keyed(&phase, subject));
        assert_eq!(open_unit(&mut a).to_bits(), open_unit(&mut b).to_bits());
        let third = open_unit(&mut streams.open_keyed_at(&phase, subject, 3)).to_bits();
        assert_eq!(third, open_unit(&mut streams.open_keyed_at(&phase, subject, 3)).to_bits());
        assert_ne!(third, open_unit(&mut streams.open_keyed_at(&phase, subject, 4)).to_bits(), "each place its own");
        assert!(std::panic::catch_unwind(|| streams.open(&phase, subject, Day::new(1), 0)).is_err());
    }
}
