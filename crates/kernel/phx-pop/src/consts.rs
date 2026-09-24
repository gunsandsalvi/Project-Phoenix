/// Words of a key record: 32 bytes, room for every key attribute a kind declares.
pub const KEY_WORDS: usize = 4;
/// Bits of a key record, which a radix sort of records passes over; the words above, counted in bits.
pub const KEY_BITS: u32 = u64::BITS * 4;

/// Steps packed into one word of a landing key's input: four sixteen-bit steps to a word.
pub const STEPS_PER_WORD: usize = 4;

/// Leading positions whose totals the hot record carries.
pub const HOT_LEAD: usize = 3;
/// Leading positions whose steps the hot record carries.
pub const HOT_STEPS: usize = 8;
/// Joint values up to which a profile group is kept as a dense histogram: a byte per value, where the sparse form
/// takes about two per value held, so up to sixteen values dense is no dearer than eight held sparsely.
pub const DENSE_MAX_VALUES: u32 = 16;
/// The high bit of a varint byte: more bytes follow.
pub const VARINT_MORE: u8 = 0x80;
/// Bits of a number each varint byte carries.
pub const VARINT_BITS: u32 = 7;

#[cfg(test)]
mod tests {
    #[test]
    fn a_records_bits_are_its_words() {
        let word_bits = usize::try_from(u64::BITS).unwrap();
        assert_eq!(usize::try_from(super::KEY_BITS).unwrap(), super::KEY_WORDS * word_bits);
    }
}
