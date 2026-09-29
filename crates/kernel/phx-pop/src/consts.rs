/// A person word's bits for its birth date: the calendar's civil day serial, in two's complement, so persons born
/// before the world's epoch are held exactly.
pub const BIRTH_BITS: u32 = 32;
/// A person word's bits for its role, after its birth date: room for eight roles.
pub const ROLE_BITS: u32 = 3;
/// A person word's bits for its kind's person attributes, after its role.
pub const PERSON_ATTR_BITS: u32 = u64::BITS - BIRTH_BITS - ROLE_BITS;
