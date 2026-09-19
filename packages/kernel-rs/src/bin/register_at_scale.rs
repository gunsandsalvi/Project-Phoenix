//! THIS register, at the world's scale, against the TypeScript engine's measured self times.
//!
//! `tools/calibrate` measured a prototype; this measures the real store in `src/register.rs`, so
//! the ratio 0g.41 is gated on is the one the ported engine will actually have.
//!
//! The shape is the full world's, from `npm run world 1`: 10,318 parties, 16,750 instruments,
//! 544,104 holdings in 657,785 lots, 23,304,012 quantity reads a period, ~10 full traversals.

use phoenix_kernel::ids::{InstrumentId, PartyId};
use phoenix_kernel::register::Register;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
const LOTS: usize = 657_785;
const READS: usize = 23_304_012;

/// TypeScript, measured in the engine's own CPU profile.
const TS_READ_NS: f64 = 67.20;
const TS_WALK_MS: f64 = 95.70;

struct Draw(u64);
impl Draw {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u32) -> u32 {
        (self.next() % u64::from(n)) as u32
    }
}

fn main() {
    let mut draw = Draw(0x9E37_79B9_7F4A_7C15);
    let mut reg = Register::new();
    let began = Instant::now();
    let mut pairs = Vec::with_capacity(HOLDINGS);
    let mut lots_placed = 0usize;
    while reg.rows() < HOLDINGS {
        let p = PartyId::at(draw.below(PARTIES));
        let i = InstrumentId::at(draw.below(INSTRUMENTS));
        if reg.row(p, i).some() {
            continue;
        }
        reg.credit(p, i, ((draw.next() % 1_000_000) as f64) / 100.0, 1.0, 1);
        lots_placed += 1;
        // 657,785 lots over 544,104 holdings is 1.209 each.
        if lots_placed + 4 < LOTS && draw.next().is_multiple_of(5) {
            reg.credit(p, i, ((draw.next() % 1_000_000) as f64) / 100.0, 2.0, 2);
            lots_placed += 1;
        }
        pairs.push((p, i));
    }
    let build_ms = began.elapsed().as_secs_f64() * 1000.0;

    // The hot read, with the row in hand — which is what a caller in a loop holds.
    let rows: Vec<_> = pairs.iter().map(|&(p, i)| reg.row(p, i)).collect();
    let mut picks = Vec::with_capacity(READS);
    for _ in 0..READS {
        picks.push(rows[(draw.next() as usize) % rows.len()]);
    }
    let t = Instant::now();
    let mut sink = 0.0f64;
    for &row in &picks {
        sink += reg.quantity(row);
    }
    let read_ns = t.elapsed().as_secs_f64() * 1e9 / READS as f64;

    // A full traversal, the way an audit family makes one.
    let t = Instant::now();
    let mut passes = 0;
    let mut walk = 0.0f64;
    for _ in 0..10 {
        for row in reg.all() {
            walk += reg.quantity(row);
        }
        passes += 1;
    }
    let walk_ms = t.elapsed().as_secs_f64() * 1000.0 / f64::from(passes);

    // The third timing here was `lots_against_quantity`, and it timed a check that could not
    // fail — `quantity()` re-derives from the very lots it summed. Both are gone, and a benchmark
    // asserting against a check the world never ran went with them.
    println!("built {} holdings, {lots_placed} lots in {build_ms:.0} ms", reg.rows());
    println!(
        "hot read       {read_ns:6.2} ns/op   over {READS} ops    TS {TS_READ_NS:.2}   {:5.1}x",
        TS_READ_NS / read_ns
    );
    println!(
        "full traversal {walk_ms:6.2} ms/pass over {} rows       TS {TS_WALK_MS:.2}   {:5.1}x",
        reg.rows(),
        TS_WALK_MS / walk_ms
    );
    println!("checksum {sink:.0} {walk:.0}");
}
