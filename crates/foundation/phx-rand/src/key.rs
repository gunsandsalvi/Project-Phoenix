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
    /// A joint value of a population kind's profile group, as an event names the members a hit reached.
    ProfileValue,
    /// A person, by its own identity, apart from the household it belongs to.
    Person,
}

const TAGS: [SubjectTag; 13] = [
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
    SubjectTag::ProfileValue,
    SubjectTag::Person,
];

/// A tagged identity: four bits of tag above sixty bits of identity.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Subject(u64);

impl Subject {
    #[inline]
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

#[inline]
fn halves(v: u64) -> [u32; 2] {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = v.to_le_bytes();
    [u32::from_le_bytes([b0, b1, b2, b3]), u32::from_le_bytes([b4, b5, b6, b7])]
}

/// Whose draws a stream's are: the world's, the observer's or the player's advice's. The family is mixed into the
/// key, so streams of two families never share one whatever their names; the world's word is nothing, so its keys are
/// its name and the seed alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StreamFamily {
    World,
    Observer,
    Advice,
}

impl StreamFamily {
    const fn word(self) -> u32 {
        match self {
            StreamFamily::World => 0,
            StreamFamily::Observer => 1,
            StreamFamily::Advice => 2,
        }
    }
}

/// A stream's key: its family, its name and the seed mixed by one Philox block, so adding a stream changes no other.
#[clause("CHN.1")]
pub fn family_key(seed: Seed, family: StreamFamily, name: &str) -> StreamKey {
    let [fnv_lo, fnv_hi] = halves(fnv1a64(name.as_bytes()));
    let [seed_lo, seed_hi] = halves(seed.0);
    let [w0, w1, _, _] = philox([fnv_lo, fnv_hi, seed_lo, seed_hi ^ family.word()], STREAM_KEY_KEY);
    StreamKey([w0, w1])
}

/// A world stream's key.
#[clause("CHN.1")]
pub fn stream_key(seed: Seed, name: &str) -> StreamKey {
    family_key(seed, StreamFamily::World, name)
}

/// The counter of one block of one address: the subject, the day, and the slot's ordinal above the block index.
#[clause("CHN.6")]
#[must_use]
#[inline]
pub fn counter(subject: Subject, day: u32, substep: u8, block: u32) -> [u32; 4] {
    let [lo, hi] = halves(subject.0);
    [lo, hi, day, (u32::from(substep) << SUBSTEP_SHIFT) | block]
}

#[cfg(test)]
#[path = "key_tests.rs"]
mod tests;
