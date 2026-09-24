use phx_num::violation;
use phx_store::{Pod, as_bytes, from_bytes};

/// How many arena words a stored type takes; every type kept in an arena is a whole number of words.
#[must_use]
pub const fn words_of<T: Pod>() -> usize {
    size_of::<T>() / size_of::<u64>()
}

/// A stored value as arena words, little-endian as saves and hashes read them.
#[must_use]
pub fn to_words<T: Pod>(value: &T) -> Vec<u64> {
    as_bytes(core::slice::from_ref(value))
        .chunks(size_of::<u64>())
        .map(|c| {
            let mut w = [0_u8; size_of::<u64>()];
            w.iter_mut().zip(c).for_each(|(a, b)| *a = *b);
            u64::from_le_bytes(w)
        })
        .collect()
}

/// A stored value read back from arena words.
#[must_use]
pub fn from_words<T: Pod>(words: &[u64]) -> T {
    let bytes = as_bytes(words);
    let Some(v) = bytes.get(..size_of::<T>()).and_then(from_bytes::<T>) else {
        violation!(clause = "REG.14", "a holder's list read short of a whole record", words = words.len());
    };
    v
}
