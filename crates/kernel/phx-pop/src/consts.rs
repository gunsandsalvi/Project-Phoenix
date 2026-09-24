/// Words of a key record: 32 bytes, room for every key attribute a kind declares.
pub const KEY_WORDS: usize = 4;
/// Bits of a key record, which a radix sort of records passes over; the words above, counted in bits.
pub const KEY_BITS: u32 = u64::BITS * 4;

/// Steps packed into one word of a landing key's input: four sixteen-bit steps to a word.
pub const STEPS_PER_WORD: usize = 4;

#[cfg(test)]
mod tests {
    #[test]
    fn a_records_bits_are_its_words() {
        let word_bits = usize::try_from(u64::BITS).unwrap();
        assert_eq!(usize::try_from(super::KEY_BITS).unwrap(), super::KEY_WORDS * word_bits);
    }
}
