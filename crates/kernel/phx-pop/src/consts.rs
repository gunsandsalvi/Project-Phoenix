/// Rows each kind's table in the population's agenda reserves: room for every agent at the smallest factor a build
/// sets, in address space committed only as rows are written.
pub const AGENT_AGENDA_ROWS: u32 = 1 << 25;
/// Blocks of the population agenda's entries: room for every agent's one entry, in address space committed as used.
pub const AGENDA_BLOCKS: u32 = 1 << 22;

/// A person word's bits for its birth date: the calendar's civil day serial, in two's complement, so persons born
/// before the world's epoch are held exactly.
pub const BIRTH_BITS: u32 = 32;
/// A person word's bits for its role, after its birth date: room for eight roles.
pub const ROLE_BITS: u32 = 3;
/// A person word's bits for its kind's person attributes, after its role.
pub const PERSON_ATTR_BITS: u32 = u64::BITS - BIRTH_BITS - ROLE_BITS;

/// An attachment word's bits for its line, from its lowest.
pub const LINE_BITS: u32 = 32;
/// The bit after the line: set for a liability side.
pub const SIDE_BIT: u32 = 32;
/// The place of the holder, after the side bit: a person's place in the household, or the household's own mark.
pub const HOLDER_SHIFT: u32 = 33;
/// An attachment's holder mark for the household itself, above every person's place.
pub const HOUSEHOLD_MARK: u64 = 0xFF;
/// The most persons a household holds, below its own mark.
pub const MOST_PERSONS: usize = 0xFF;
