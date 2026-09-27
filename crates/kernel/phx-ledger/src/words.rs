use phx_num::violation;
use phx_store::{Pod, as_bytes, from_bytes};

/// How many arena words a stored type takes; every type kept in an arena is a whole number of words.
#[must_use]
pub const fn words_of<T: Pod>() -> usize {
    size_of::<T>() / size_of::<u64>()
}

/// A stored value's words, built where the caller stands rather than on the heap, since every write to a holder's
/// lists builds one.
#[derive(Clone, Copy, Debug)]
pub struct Words {
    buf: [u64; crate::consts::RECORD_WORDS],
    len: usize,
}

impl core::ops::Deref for Words {
    type Target = [u64];
    fn deref(&self) -> &[u64] {
        self.buf.get(..self.len).unwrap_or(&[])
    }
}

/// A stored value as arena words, little-endian as saves and hashes read them.
#[must_use]
pub fn to_words<T: Pod>(value: &T) -> Words {
    const {
        assert!(size_of::<T>() <= crate::consts::RECORD_WORDS * size_of::<u64>(), "a record wider than its room");
    };
    let mut out = Words { buf: [0; crate::consts::RECORD_WORDS], len: words_of::<T>() };
    for (w, c) in out.buf.iter_mut().zip(as_bytes(core::slice::from_ref(value)).chunks(size_of::<u64>())) {
        let mut b = [0_u8; size_of::<u64>()];
        b.iter_mut().zip(c).for_each(|(a, x)| *a = *x);
        *w = u64::from_le_bytes(b);
    }
    out
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
