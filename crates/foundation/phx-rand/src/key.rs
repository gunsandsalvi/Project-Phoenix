use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::consts::{FNV_OFFSET, FNV_PRIME, STREAM_KEY_KEY, SUBJECT_TAG_SHIFT, SUBSTEP_SHIFT};
use crate::philox::philox;

/// The one seed of a run.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed(u64);

impl Seed {
    pub const fn new(seed: u64) -> Seed {
        Seed(seed)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A stream's Philox key, a function of its name and the seed alone.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StreamKey([u32; 2]);

impl StreamKey {
    #[must_use]
    pub const fn words(self) -> [u32; 2] {
        self.0
    }
}

/// What a draw is about, so draws for different things never share a counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubjectTag {
    World,
    Party,
    Part,
    Line,
    Tile,
    Region,
    Zone,
    Country,
    Market,
    Instrument,
    /// A stratum and ordinal of an opening draw, before its party exists.
    Opening,
}

const TAGS: [SubjectTag; 11] = [
    SubjectTag::World,
    SubjectTag::Party,
    SubjectTag::Part,
    SubjectTag::Line,
    SubjectTag::Tile,
    SubjectTag::Region,
    SubjectTag::Zone,
    SubjectTag::Country,
    SubjectTag::Market,
    SubjectTag::Instrument,
    SubjectTag::Opening,
];

/// A tagged identity: four bits of tag above sixty bits of identity.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Subject(u64);

impl Subject {
    pub fn new(tag: SubjectTag, id: u64) -> Subject {
        let limit = 1_u64 << SUBJECT_TAG_SHIFT;
        if id >= limit {
            capacity_exceeded!("subject identity bits", limit, id);
        }
        let index = TAGS.iter().take_while(|t| **t != tag).fold(0_u64, |n, _| n + 1);
        Subject((index << SUBJECT_TAG_SHIFT) | id)
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }

    /// A subject read back from its raw word; `None` when the tag is none of the known ones.
    #[must_use]
    pub fn from_raw(raw: u64) -> Option<Subject> {
        let index = usize::try_from(raw >> SUBJECT_TAG_SHIFT).ok()?;
        TAGS.get(index).map(|_| Subject(raw))
    }

    #[must_use]
    pub fn tag(self) -> SubjectTag {
        let found = usize::try_from(self.0 >> SUBJECT_TAG_SHIFT).ok().and_then(|i| TAGS.get(i));
        let Some(tag) = found else {
            phx_num::violation!(clause = "CHN.2", "a subject with no tag", raw = self.0);
        };
        *tag
    }

    #[must_use]
    pub fn id(self) -> u64 {
        self.0 & ((1_u64 << SUBJECT_TAG_SHIFT) - 1)
    }
}

/// FNV-1a over a name's bytes, the first half of a stream's key.
#[must_use]
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET, |h, b| (h ^ u64::from(*b)).wrapping_mul(FNV_PRIME))
}

fn halves(v: u64) -> [u32; 2] {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = v.to_le_bytes();
    [u32::from_le_bytes([b0, b1, b2, b3]), u32::from_le_bytes([b4, b5, b6, b7])]
}

/// A stream's key: its name and the seed mixed by one Philox block, so adding a stream changes no other.
#[clause("CHN.1")]
pub fn stream_key(seed: Seed, name: &str) -> StreamKey {
    let [fnv_lo, fnv_hi] = halves(fnv1a64(name.as_bytes()));
    let [seed_lo, seed_hi] = halves(seed.0);
    let [w0, w1, _, _] = philox([fnv_lo, fnv_hi, seed_lo, seed_hi], STREAM_KEY_KEY);
    StreamKey([w0, w1])
}

/// The counter of one block of one address: the subject, the day, and the sub-step above the block index.
#[must_use]
pub fn counter(subject: Subject, day: u32, substep: u8, block: u32) -> [u32; 4] {
    let [lo, hi] = halves(subject.0);
    [lo, hi, day, (u32::from(substep) << SUBSTEP_SHIFT) | block]
}

#[cfg(test)]
mod tests {
    use super::{Seed, Subject, SubjectTag, counter, stream_key};

    #[test]
    fn stream_keys_independent_of_registration() {
        let seed = Seed::new(42);
        let in_order: Vec<_> = ["A", "B", "C"].iter().map(|n| stream_key(seed, n)).collect();
        let reordered: Vec<_> = ["C", "A", "B", "D"].iter().map(|n| stream_key(seed, n)).collect();
        assert_eq!(in_order, vec![reordered[1], reordered[2], reordered[0]]);
        assert_ne!(stream_key(seed, "A"), stream_key(Seed::new(43), "A"));
        assert_ne!(stream_key(seed, "A"), stream_key(seed, "B"));
    }

    #[test]
    fn subjects_read_back() {
        let s = Subject::new(SubjectTag::Party, 77);
        assert_eq!((Subject::from_raw(s.raw()), s.tag(), s.id()), (Some(s), SubjectTag::Party, 77));
        assert_eq!(Subject::from_raw(u64::MAX), None);
    }

    #[test]
    fn subjects_never_collide() {
        let party = Subject::new(SubjectTag::Party, 7);
        let line = Subject::new(SubjectTag::Line, 7);
        assert_ne!(counter(party, 3, 1, 0), counter(line, 3, 1, 0));
        let caught = std::panic::catch_unwind(|| Subject::new(SubjectTag::World, 1 << 60));
        assert!(caught.is_err());
    }

    #[test]
    fn substeps_never_collide() {
        let s = Subject::new(SubjectTag::Party, 9);
        assert_ne!(counter(s, 3, 1, 0), counter(s, 3, 2, 0));
        assert_ne!(counter(s, 3, 1, 0), counter(s, 4, 1, 0));
    }
}
