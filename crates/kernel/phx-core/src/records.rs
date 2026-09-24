use phx_id::{Day, PartyId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{capacity_exceeded, violation};
use phx_store::consts::DEFAULT_ROWS_PER_CHUNK;
use phx_store::{AddressSpace, Backing, ChunkArena, Column, ListRef, LogicalHasher, SystemBacking};

use crate::calendar::Calendar;
use crate::calendar::period::Period;
use crate::facts::{Audience, Lag};
use crate::substep::SubStep;

/// A kind of public or scoped record: who may read its entries, how long they are kept, and who writes them.
#[clause("OBS.1", "PTY.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordKindDecl {
    pub name: &'static str,
    pub audience: Audience,
    pub horizon: Lag,
    pub writer: &'static str,
    pub clause: &'static str,
}

fn period(lag: Lag) -> Period {
    let p = match lag {
        Lag::Days(n) => Period::days(n),
        Lag::Months(n) => Period::months(n),
    };
    let Some(p) = p else {
        violation!(clause = "OBS.1", "a lag of no length");
    };
    p
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct RecordRow {
    subject: u64,
    day: u32,
    kind: u16,
    substep: u8,
    pad: u8,
    payload: ListRef,
    pad_word: u32,
}

/// Who reads, and when: its party, its kind, and the day and sub-step it reads in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reader {
    pub party: PartyId,
    pub kind: &'static str,
    pub day: Day,
    pub substep: SubStep,
}

/// An entry as a reader sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordEntry {
    pub subject: PartyId,
    pub day: Day,
    pub substep: u8,
    pub payload: Vec<u64>,
}

/// An entry's subject, and the day and sub-step ordinal that wrote it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordStamp {
    pub subject: PartyId,
    pub day: Day,
    pub substep: u8,
}

/// The world's records, dated by day and sub-step, each readable only by its audience and only after it was written.
/// They are world state: hashed and saved.
#[clause("OBS.1", "TIME.10")]
#[derive(Debug)]
pub struct RecordStore<B: Backing = SystemBacking> {
    kinds: Vec<RecordKindDecl>,
    rows: Column<RecordRow, B>,
    arena: ChunkArena<B>,
}

impl<B: Backing> RecordStore<B> {
    /// The record kinds the store keeps, as the build declared them.
    #[must_use]
    pub fn kinds(&self) -> &[RecordKindDecl] {
        &self.kinds
    }

    /// The store for a save: its kinds' names, which a load checks against the build's, its rows and its arena.
    pub fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        let names: Vec<&'static str> = self.kinds.iter().map(|k| k.name).collect();
        names.save(w);
        self.rows.save(w);
        self.arena.save(w);
    }

    /// The store read back, over the kinds the build declares, which must be those it was saved with.
    ///
    /// # Errors
    /// When the store is damaged or its kinds are not the build's.
    pub fn load_from(
        r: &mut phx_store::Reader<'_>,
        kinds: Vec<RecordKindDecl>,
    ) -> Result<RecordStore<B>, phx_store::LoadError> {
        use phx_store::Saved as _;
        let names: Vec<&'static str> = phx_store::Saved::load(r)?;
        if names.len() != kinds.len() || names.iter().zip(&kinds).any(|(n, k)| *n != k.name) {
            return Err(phx_store::LoadError::Invalid("record kinds other than the build's".to_owned()));
        }
        Ok(RecordStore { kinds, rows: Column::load(r)?, arena: ChunkArena::load(r)? })
    }

    pub fn new(space: &mut AddressSpace, kinds: Vec<RecordKindDecl>, max: u32, arena_words: u32) -> RecordStore<B> {
        RecordStore {
            kinds,
            rows: Column::new(space, max, DEFAULT_ROWS_PER_CHUNK),
            arena: ChunkArena::new(space, arena_words),
        }
    }

    fn kind_index(&self, name: &str) -> u16 {
        let found = self.kinds.iter().position(|k| k.name == name).and_then(|i| u16::try_from(i).ok());
        let Some(i) = found else {
            violation!(clause = "OBS.1", "a record kind never declared");
        };
        i
    }

    /// Writes an entry about a party, at the day and sub-step it is written in.
    pub fn write(&mut self, kind: &str, subject: PartyId, day: Day, substep: SubStep, payload: &[u64]) {
        let kind = self.kind_index(kind);
        let mut list = ListRef::EMPTY;
        self.arena.append(&mut list, payload);
        let row = RecordRow {
            subject: subject.get(),
            day: day.get(),
            kind,
            substep: substep.ordinal(),
            pad: 0,
            payload: list,
            pad_word: 0,
        };
        self.rows.push(row);
    }

    /// Whether a reader may see an entry: dated before its own sub-step, of an audience that includes it, and past its
    /// lag if it becomes public after one.
    fn visible(&self, row: &RecordRow, reader: &Reader, calendar: &Calendar) -> bool {
        let Some(decl) = self.kinds.get(usize::from(row.kind)) else {
            violation!(clause = "OBS.1", "a record of a kind never declared", kind = row.kind);
        };
        let before = (row.day, row.substep) < (reader.day.get(), reader.substep.ordinal());
        before
            && match decl.audience {
                Audience::Party => row.subject == reader.party.get(),
                Audience::Authority(kind) => reader.kind == kind || row.subject == reader.party.get(),
                Audience::Public => true,
                Audience::PublicAfter(lag) => {
                    row.subject == reader.party.get() || calendar.plus(Day::new(row.day), period(lag)) <= reader.day
                }
            }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Feeds every entry to the world's hash, its payload read through its reference.
    pub fn hash_into(&self, h: &mut LogicalHasher) {
        for row in self.rows.slice() {
            h.u64(row.subject);
            h.u64(u64::from(row.day));
            h.u64(u64::from(row.kind));
            h.u64(u64::from(row.substep));
            h.list(&self.arena, row.payload);
        }
    }

    /// The entry at a place in the order written.
    #[must_use]
    pub fn stamp(&self, index: usize) -> Option<RecordStamp> {
        let row = u32::try_from(index).ok().and_then(|i| self.rows.get(Slot::new(i)))?;
        Some(RecordStamp { subject: PartyId::new(row.subject), day: Day::new(row.day), substep: row.substep })
    }

    /// The day and sub-step ordinal of every entry, in the order written.
    #[must_use]
    pub fn dates(&self) -> Vec<(Day, u8)> {
        self.rows.slice().iter().map(|r| (Day::new(r.day), r.substep)).collect()
    }

    /// The entries of a kind the reader may see, in the order written.
    pub fn read(&self, kind: &str, reader: &Reader, calendar: &Calendar) -> Vec<RecordEntry> {
        let kind = self.kind_index(kind);
        let Ok(n) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("records", u32::MAX, self.rows.len());
        };
        (0..n)
            .filter_map(|i| self.rows.get(Slot::new(i)))
            .filter(|r| r.kind == kind && self.visible(r, reader, calendar))
            .map(|r| RecordEntry {
                subject: PartyId::new(r.subject),
                day: Day::new(r.day),
                substep: r.substep,
                payload: self.arena.read(r.payload).to_vec(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Date, PartyId};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Reader, RecordKindDecl, RecordStore};
    use crate::calendar::testing::calendar;
    use crate::facts::{Audience, Lag};
    use crate::substep::SubStep;

    #[test]
    fn records_respect_audience_lag_and_substep() {
        let cal = calendar(2020);
        let day = |m, d| cal.day(Date::new(2025, m, d).unwrap()).unwrap();
        let kind = |name, audience| RecordKindDecl {
            name,
            audience,
            horizon: Lag::Months(12),
            writer: "BNK",
            clause: "OBS.1",
        };
        let kinds = vec![
            kind("BNK.report", Audience::PublicAfter(Lag::Days(30))),
            kind("BNK.supervisory", Audience::Authority("supervisor")),
            kind("BNK.print", Audience::Public),
        ];
        let mut space = AddressSpace::empty();
        let mut store: RecordStore<HeapBacking<4096>> = RecordStore::new(&mut space, kinds, 64, 1 << 12);
        let bank = PartyId::new(3);
        store.write("BNK.report", bank, day(1, 10), SubStep::S9d, &[42]);
        store.write("BNK.supervisory", bank, day(1, 10), SubStep::S9c, &[7]);
        store.write("BNK.print", bank, day(1, 10), SubStep::S6b, &[1]);
        let reader = |kind, m, d, substep| Reader { party: PartyId::new(8), kind, day: day(m, d), substep };
        let household = reader("household", 1, 10, SubStep::S6a);
        assert!(store.read("BNK.print", &household, &cal).is_empty(), "written later in the same day");
        assert_eq!(store.read("BNK.print", &reader("household", 1, 10, SubStep::S6c), &cal).len(), 1);
        assert!(store.read("BNK.report", &reader("household", 2, 8, SubStep::S1a), &cal).is_empty(), "within its lag");
        assert_eq!(store.read("BNK.report", &reader("household", 2, 9, SubStep::S1a), &cal)[0].payload, vec![42]);
        assert!(store.read("BNK.supervisory", &reader("household", 3, 1, SubStep::S1a), &cal).is_empty());
        assert_eq!(store.read("BNK.supervisory", &reader("supervisor", 1, 10, SubStep::S9d), &cal).len(), 1);
    }
}
