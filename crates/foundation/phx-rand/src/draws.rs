use phx_macros::clause;
use phx_num::capacity_exceeded;

use crate::consts::{BLOCK_WORDS, BLOCKS_PER_ADDRESS};
use crate::key::{StreamKey, Subject, counter};
use crate::philox::philox;

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
    pub fn new(key: StreamKey, subject: Subject, day: u32, substep: u8) -> Draws {
        let base = counter(subject, day, substep, 0);
        Draws { key: key.words(), base, next_block: 0, buf: [0; BLOCK_WORDS], pos: BLOCK_WORDS }
    }

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
}
