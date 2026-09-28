use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::consts::{BLOCK_WORDS, BLOCKS_PER_ADDRESS, LANES};
use crate::key::{StreamKey, Subject, counter};
use crate::philox::{philox, philox_x4};

/// The cursor over one address (stream, subject, day, sub-step): every sampler draws from it, so a result depends
/// only on the address and never on the order subjects are processed.
#[clause("CHN.6")]
#[derive(Clone, Debug)]
pub struct Draws {
    key: [u32; 2],
    /// The address's first block's counter, built once: a block's counter adds its index to the last word.
    base: [u32; 4],
    next_block: u32,
    buf: [u32; BLOCK_WORDS],
    pos: usize,
}

impl Draws {
    #[must_use]
    #[inline]
    pub fn new(key: StreamKey, subject: Subject, day: u32, substep: u8) -> Draws {
        let base = counter(subject, day, substep, 0);
        Draws { key: key.words(), base, next_block: 0, buf: [0; BLOCK_WORDS], pos: BLOCK_WORDS }
    }

    /// The cursor from a block of its address on: the draws of a purpose that takes one block a turn, as a buyer's
    /// choice in each round of a meeting does, start at that turn's block.
    #[must_use]
    #[inline]
    pub fn from_block(key: StreamKey, subject: Subject, day: u32, substep: u8, block: u32) -> Draws {
        let mut d = Draws::new(key, subject, day, substep);
        d.next_block = block;
        d
    }

    /// Four addresses' cursors from the same block, as `from_block` gives each, their first blocks made together so
    /// four independent Philox chains run side by side.
    #[must_use]
    #[inline]
    pub fn x4(key: StreamKey, subjects: [Subject; LANES], day: u32, substep: u8, block: u32) -> [Draws; LANES] {
        if block >= BLOCKS_PER_ADDRESS {
            capacity_exceeded!("blocks per draw address", BLOCKS_PER_ADDRESS, block);
        }
        let mut ds = subjects.map(|s| Draws::from_block(key, s, day, substep, block));
        let ctrs = ds.each_ref().map(|d| {
            let [c0, c1, c2, c3] = d.base;
            [c0, c1, c2, c3 | block]
        });
        for (d, b) in ds.iter_mut().zip(philox_x4(ctrs, key.words())) {
            d.buf = b;
            d.next_block = block + 1;
            d.pos = 0;
        }
        ds
    }

    #[inline]
    fn refill(&mut self) {
        if self.next_block >= BLOCKS_PER_ADDRESS {
            capacity_exceeded!("blocks per draw address", BLOCKS_PER_ADDRESS, self.next_block);
        }
        let [c0, c1, c2, c3] = self.base;
        self.buf = philox([c0, c1, c2, c3 | self.next_block], self.key);
        self.next_block += 1;
        self.pos = 0;
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        if self.pos >= BLOCK_WORDS {
            self.refill();
        }
        let [w0, w1, w2, w3] = self.buf;
        let word = match self.pos {
            0 => w0,
            1 => w1,
            2 => w2,
            _ => w3,
        };
        self.pos += 1;
        word
    }

    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        let low = self.next_u32();
        let high = self.next_u32();
        (u64::from(high) << u32::BITS) | u64::from(low)
    }
}

#[cfg(test)]
mod tests {
    use super::Draws;
    use crate::key::{Seed, Subject, SubjectTag, stream_key};

    #[test]
    fn same_address_same_draw() {
        let key = stream_key(Seed::new(1), "DEM.mortality");
        let s = Subject::new(SubjectTag::Party, 12);
        let mut a = Draws::new(key, s, 100, 3);
        let mut b = Draws::new(key, s, 100, 3);
        let first: Vec<u64> = (0..9).map(|_| a.next_u64()).collect();
        let second: Vec<u64> = (0..9).map(|_| b.next_u64()).collect();
        assert_eq!(first, second);
        let mut c = Draws::new(key, s, 101, 3);
        assert_ne!(first[0], c.next_u64());
    }

    #[test]
    fn four_at_once_are_each_from_its_block() {
        let key = stream_key(Seed::new(1), "SRV.taste");
        let subjects = [3, 9, 27, 81].map(|i| Subject::new(SubjectTag::Party, i));
        for block in [0, 1, 7] {
            let mut four = Draws::x4(key, subjects, 40, 6, block);
            for (d, s) in four.iter_mut().zip(subjects) {
                let mut one = Draws::from_block(key, s, 40, 6, block);
                let (a, b): (Vec<u64>, Vec<u64>) =
                    ((0..5).map(|_| d.next_u64()).collect(), (0..5).map(|_| one.next_u64()).collect());
                assert_eq!(a, b);
            }
        }
        let mut whole = Draws::new(key, subjects[0], 40, 6);
        let _ = (whole.next_u64(), whole.next_u64());
        assert_eq!(whole.next_u64(), Draws::from_block(key, subjects[0], 40, 6, 1).next_u64(), "a block is two words");
    }
}
