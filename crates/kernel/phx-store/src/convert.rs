use phx_num::capacity_exceeded;

/// A 32-bit count or offset as an index; every supported target's pointer is at least 32 bits wide.
pub(crate) fn to_usize(n: u32) -> usize {
    let Ok(i) = usize::try_from(n) else {
        capacity_exceeded!("index width", usize::MAX, n);
    };
    i
}

/// An index as a 32-bit count or offset, which every store's layout uses.
pub(crate) fn to_u32(n: usize) -> u32 {
    let Ok(v) = u32::try_from(n) else {
        capacity_exceeded!("32-bit offsets", u32::MAX, n);
    };
    v
}

/// An index as a 64-bit count, as saves and hashes write it.
pub(crate) fn to_u64(n: usize) -> u64 {
    let Ok(v) = u64::try_from(n) else {
        capacity_exceeded!("64-bit counts", u64::MAX, n);
    };
    v
}
