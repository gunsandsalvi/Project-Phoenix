/// A person word's bits for its birth date: the calendar's civil day serial, in two's complement, so persons born
/// before the world's epoch are held exactly.
pub const BIRTH_BITS: u32 = 32;
/// A person word's bits for its role, after its birth date: room for eight roles.
pub const ROLE_BITS: u32 = 3;
/// A person word's bits for its kind's person attributes, after its role.
pub const PERSON_ATTR_BITS: u32 = u64::BITS - BIRTH_BITS - ROLE_BITS;
/// A tombstone's packed reference's bits below its day byte: kind, slot and generation.
pub const TOMB_KEY_BITS: u32 = 56;
/// The share of the main tombstone run the recent one may reach before they are merged: a sixteenth, so a day's merge
/// is rare and its cost amortised over the days that fill it.
pub const TOMB_MERGE_SHARE: usize = 16;
/// The main tombstone run's keys a fence entry stands for: a block of a kilobyte, so a lookup is a search of the fence,
/// which stays in cache, then of one block.
pub const TOMB_FENCE: usize = 64;
