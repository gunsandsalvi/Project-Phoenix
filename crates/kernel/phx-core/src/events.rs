use phx_id::Day;
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::Subject;
use phx_store::{AddressSpace, Backing, ChunkArena, Column, ListRef, LogicalHasher, SystemBacking};

use crate::substep::SubStep;

/// A kind of event, and the unit its sizes are in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventKindDecl {
    pub name: &'static str,
    pub size_unit: &'static str,
    pub clause: &'static str,
}

/// One event: when, what kind, whether it is public, what it develops from, and where its subjects and their sizes
/// lie in the store's arena.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct EventRow {
    id: u64,
    day: u32,
    kind: u16,
    substep: u8,
    public: u8,
    subjects: ListRef,
    details: ListRef,
    develops_from: u64,
}

const NONE: u64 = u64::MAX;

/// An event as read back: its subjects, and each detail's subject and size in the kind's unit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: u64,
    pub day: Day,
    pub substep: u8,
    pub kind: u16,
    pub public: bool,
    pub develops_from: Missing<u64>,
    pub subjects: Vec<u64>,
    pub details: Vec<(u64, i64)>,
}

/// What an event records when it is written, in the apply of the sub-step that drew it.
#[derive(Clone, Copy, Debug)]
pub struct NewEvent<'a> {
    pub day: Day,
    pub substep: SubStep,
    pub kind: u16,
    pub subjects: &'a [Subject],
    pub details: &'a [(Subject, i64)],
    pub develops_from: Missing<u64>,
}

/// An event drawn by a handler, recorded at its sub-step's apply point: its kind's place among the declared kinds,
/// which the handler's system resolves at assembly, its subjects, and each detail's subject and size. Whether it
/// becomes public is the declared rule's to say at the close, never the handler's.
#[clause("CHN.4")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventIntent {
    pub kind: u16,
    pub subjects: Vec<Subject>,
    pub details: Vec<(Subject, i64)>,
}

impl crate::handler::IntentDef for EventIntent {
    const NAME: &'static str = "CHN.event";

    fn encode(&self, out: &mut Vec<u64>) {
        out.push(u64::from(self.kind));
        let Ok(count) = u64::try_from(self.subjects.len()) else {
            capacity_exceeded!("event subjects", u64::MAX, self.subjects.len());
        };
        out.push(count);
        out.extend(self.subjects.iter().map(|s| s.raw()));
        out.extend(self.details.iter().flat_map(|(s, size)| [s.raw(), size.cast_unsigned()]));
    }
}

impl EventIntent {
    /// The intent its words encode, or none when they are not an event's.
    #[must_use]
    pub fn decode(words: &[u64]) -> Option<EventIntent> {
        let [kind, count, rest @ ..] = words else { return None };
        let count = usize::try_from(*count).ok()?;
        let (subjects, details) = (rest.get(..count)?, rest.get(count..)?);
        if details.len() % 2 != 0 {
            return None;
        }
        Some(EventIntent {
            kind: u16::try_from(*kind).ok()?,
            subjects: subjects.iter().map(|w| Subject::from_raw(*w)).collect::<Option<_>>()?,
            details: details
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[s, size]| Subject::from_raw(*s).map(|s| (s, size.cast_signed())))
                .collect::<Option<_>>()?,
        })
    }
}

/// Every occurrence, recorded before any party reacts to it, with identities in the order written.
#[clause("CHN.4", "OBS.3")]
#[derive(Debug, phx_macros::Saved)]
pub struct EventStore<B: Backing = SystemBacking> {
    rows: Column<EventRow, B>,
    arena: ChunkArena<B>,
}

impl<B: Backing> EventStore<B> {
    pub fn new(space: &mut AddressSpace, max: u32, rows_per_chunk: u32, arena_words: u32) -> EventStore<B> {
        EventStore { rows: Column::new(space, max, rows_per_chunk), arena: ChunkArena::new(space, arena_words) }
    }

    /// Writes an event and returns its identity.
    pub fn record(&mut self, e: NewEvent<'_>) -> u64 {
        let Ok(id) = u64::try_from(self.rows.len() + 1) else {
            capacity_exceeded!("events", u64::MAX, self.rows.len());
        };
        let develops_from = match e.develops_from {
            Missing::Present(earlier) if earlier < id => earlier,
            Missing::Present(later) => {
                violation!(clause = "CHN.4", "an event developing from one not yet written", from = later)
            }
            Missing::Absent => NONE,
        };
        let mut subjects = ListRef::EMPTY;
        let words: Vec<u64> = e.subjects.iter().map(|s| s.raw()).collect();
        self.arena.append(&mut subjects, &words);
        let mut details = ListRef::EMPTY;
        let words: Vec<u64> = e.details.iter().flat_map(|(s, size)| [s.raw(), size.cast_unsigned()]).collect();
        self.arena.append(&mut details, &words);
        self.rows.push(EventRow {
            id,
            day: e.day.get(),
            kind: e.kind,
            substep: e.substep.ordinal(),
            public: 0,
            subjects,
            details,
            develops_from,
        });
        id
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Makes public, by the declared rule, each event dated `from` or later that is not yet public, and returns how
    /// many it made. The rule reads each event alone, so an event read twice is judged the same way both times.
    #[clause("OBS.3", "OBS.5")]
    pub fn publish(&mut self, from: Day, rule: &dyn crate::extensions::PublicEventRule) -> u64 {
        let first = self.rows.slice().partition_point(|r| r.day < from.get());
        let mut made = 0;
        for i in first..self.rows.len() {
            let Some(slot) = u32::try_from(i).ok().map(phx_id::Slot::new) else {
                capacity_exceeded!("events", u32::MAX, i);
            };
            let Some(mut row) = self.rows.get(slot).filter(|r| r.public == 0) else { continue };
            if rule.is_public(&self.get(row.id)) {
                row.public = 1;
                self.rows.set(slot, row);
                made += 1;
            }
        }
        made
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Feeds every event to the world's hash, its lists read through their references.
    pub fn hash_into(&self, h: &mut LogicalHasher) {
        for row in self.rows.slice() {
            h.u64(row.id);
            h.u64(u64::from(row.day));
            h.u64(u64::from(row.kind));
            h.u64(u64::from(row.substep));
            h.u64(u64::from(row.public));
            h.u64(row.develops_from);
            h.list(&self.arena, row.subjects);
            h.list(&self.arena, row.details);
        }
    }

    #[must_use]
    pub fn get(&self, id: u64) -> Event {
        let row =
            id.checked_sub(1).and_then(|i| u32::try_from(i).ok()).and_then(|i| self.rows.get(phx_id::Slot::new(i)));
        let Some(r) = row else {
            violation!(clause = "CHN.4", "an event that was never written", id = id);
        };
        let details = self.arena.read(r.details);
        Event {
            id: r.id,
            day: Day::new(r.day),
            substep: r.substep,
            kind: r.kind,
            public: r.public != 0,
            develops_from: if r.develops_from == NONE { Missing::Absent } else { Missing::Present(r.develops_from) },
            subjects: self.arena.read(r.subjects).to_vec(),
            details: details.as_chunks::<2>().0.iter().map(|[s, size]| (*s, size.cast_signed())).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;
    use phx_rand::{Subject, SubjectTag};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{EventIntent, EventStore, NewEvent};
    use crate::handler::IntentDef;
    use crate::substep::SubStep;

    #[test]
    fn event_intents_decode_what_they_encode() {
        let tile = |i| Subject::new(SubjectTag::Tile, i);
        let intent = EventIntent { kind: 3, subjects: vec![tile(8), tile(9)], details: vec![(tile(9), -40)] };
        let mut words = Vec::new();
        intent.encode(&mut words);
        assert_eq!(EventIntent::decode(&words), Some(intent));
        assert_eq!(EventIntent::decode(words.get(..words.len() - 1).unwrap()), None, "a detail cut short");
        assert_eq!(EventIntent::decode(&[3, 2, 7]), None, "a subject count beyond the words");
    }

    #[test]
    fn events_keep_subjects_and_sizes() {
        let mut space = AddressSpace::empty();
        let mut store: EventStore<HeapBacking<4096>> = EventStore::new(&mut space, 64, 8, 1 << 12);
        let (tile_a, tile_b) = (Subject::new(SubjectTag::Tile, 3), Subject::new(SubjectTag::Tile, 4));
        let flood = NewEvent {
            day: Day::new(9),
            substep: SubStep::S3a,
            kind: 2,
            subjects: &[tile_a, tile_b],
            details: &[(tile_a, 70), (tile_b, -5)],
            develops_from: Missing::Absent,
        };
        let first = store.record(flood);
        let second = store.record(NewEvent { develops_from: Missing::Present(first), ..flood });
        let e = store.get(second);
        assert_eq!((e.develops_from, e.subjects.len(), e.details[1].1, e.substep), (Missing::Present(1), 2, -5, 9));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                store.record(NewEvent { develops_from: Missing::Present(9), ..flood })
            }))
            .is_err()
        );
    }

    struct Large;

    impl crate::extensions::PublicEventRule for Large {
        fn is_public(&self, event: &super::Event) -> bool {
            event.details.iter().any(|(_, s)| *s > 10)
        }
    }

    #[test]
    fn the_rule_publishes_from_a_day_on_and_never_unpublishes() {
        let mut space = AddressSpace::empty();
        let mut store: EventStore<HeapBacking<4096>> = EventStore::new(&mut space, 64, 8, 1 << 12);
        let tile = Subject::new(SubjectTag::Tile, 3);
        let (large, small) = ([(tile, 70)], [(tile, 5)]);
        let new = |day, details| NewEvent {
            day: Day::new(day),
            substep: SubStep::S3a,
            kind: 0,
            subjects: &[],
            details,
            develops_from: Missing::Absent,
        };
        let first = store.record(new(1, &large));
        let (second, third) = (store.record(new(2, &small)), store.record(new(2, &large)));
        assert_eq!(store.publish(Day::new(2), &Large), 1, "an event before the day is left as it is");
        assert_eq!(store.publish(Day::new(1), &Large), 1);
        assert_eq!(store.publish(Day::new(1), &Large), 0, "a public event stays public and is not counted again");
        assert_eq!([first, second, third].map(|id| store.get(id).public), [true, false, true]);
    }
}
