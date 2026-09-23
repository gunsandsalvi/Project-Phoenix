use phx_id::Day;
use phx_macros::{Pod, clause};
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::Subject;
use phx_store::{AddressSpace, Backing, ChunkArena, Column, ListRef, SystemBacking};

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
    pub public: bool,
    pub develops_from: Missing<u64>,
}

/// Every occurrence, recorded before any party reacts to it, with identities in the order written.
#[clause("CHN.4", "OBS.3")]
#[derive(Debug)]
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
            public: u8::from(e.public),
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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
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

    use super::{EventStore, NewEvent};
    use crate::substep::SubStep;

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
            public: true,
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
}
