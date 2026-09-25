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

/// The weight ladder's ratio, five quarters: a rung is the last one's weight and a quarter more.
pub const LADDER_NUM: u64 = 5;
/// See `LADDER_NUM`.
pub const LADDER_DEN: u64 = 4;

/// One unit of review exposure, −ln(1 − a) summed over days, in the fixed point a cell's exposure column keeps.
pub const EXPOSURE_ONE: f64 = 4_294_967_296.0;

/// The design point's cell a part is measured on: its relationship rows.
pub const DESIGN_ROWS: u32 = 40;
/// Its joint profile values held in each of its two roles.
pub const DESIGN_ENTRIES_PER_ROLE: u32 = 75;
/// Its positions.
pub const DESIGN_POSITIONS: usize = 30;
/// Its members.
pub const DESIGN_WEIGHT: u32 = 200;
/// The members holding each of its rows.
pub const DESIGN_ROW_MEMBERS: u32 = 50;
/// Each row's balance, in smallest units.
pub const DESIGN_ROW_BALANCE: i64 = 5_000;
/// Each position's total per member, a hundred of its scale.
pub const DESIGN_PER_MEMBER: i64 = 100;
/// A profile group's first two components' values: ten each.
pub const DESIGN_TEN: u32 = 10;
/// A profile group's third component's values, and the design point's compositions.
pub const DESIGN_THREE: u32 = 3;

/// Rows each kind's table in the population's agenda reserves: as many as a cell table does, in address space
/// committed only as rows are written.
pub const CELL_AGENDA_ROWS: u32 = 1 << 23;
/// Blocks of the population agenda's entries: room for every row's one entry, in address space committed as used.
pub const AGENDA_BLOCKS: u32 = 1 << 20;

/// The cells a landing key holds inline in the landing index before the rest spill past them.
pub const INDEX_INLINE: usize = 4;

/// A rank read's histogram bins: a per-member value's sign and bit length, sixty-five lengths either side of nought.
pub const RANK_BINS: usize = 130;

/// The rows per chunk of a synthetic population, so a thousand cells span chunks as the world's do.
pub const POPULATION_CHUNK: u32 = 256;
/// The positions a synthetic population's cells draw; the rest they hold alike.
pub const POPULATION_DRAWN: usize = 6;

/// One landing in this many is re-read after it joins, by its part's identity: enough over a run's landings to find a
/// join that loses a member or a unit or moves members across a kink, few enough to cost little of 10b.
pub const LANDING_SAMPLE: u64 = 64;

#[cfg(test)]
mod tests {
    #[test]
    fn a_records_bits_are_its_words() {
        let word_bits = usize::try_from(u64::BITS).unwrap();
        assert_eq!(usize::try_from(super::KEY_BITS).unwrap(), super::KEY_WORDS * word_bits);
    }
}
