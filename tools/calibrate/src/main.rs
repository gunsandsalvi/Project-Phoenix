//! WHAT THE RUNTIME COSTS, on this engine's real data shape at its real scale.
//!
//! It is a CALIBRATION and not a model of the world. It answers one question that cannot be
//! answered in TypeScript, because measuring a language's cost requires the other language: what
//! does the register's hot read cost when the same holdings, the same lots and the same query count
//! are laid out natively instead of as heap objects behind string-keyed hash maps?
//!
//! The shape and every count is taken from `npm run world 1` on the full world:
//!   10,318 parties · 16,750 instruments · 544,104 holdings · 657,785 lots
//!   23,304,012 quantity/free reads · 38,676,668 instrument reads · 544,104-row full traversals
//!
//! The TypeScript side of each comparison is a MEASURED self time from the engine's own CPU
//! profile, not a re-implementation here — so the two sides are the real thing against the real
//! thing, and the only difference is the language and the layout.

use std::collections::HashMap;
use std::time::Instant;

const PARTIES: u32 = 10_318;
const INSTRUMENTS: u32 = 16_750;
const HOLDINGS: usize = 544_104;
const LOTS: usize = 657_785;
const QUANTITY_READS: usize = 23_304_012;
const INSTRUMENT_READS: usize = 38_676_668;

/// A deterministic draw, so the shape is the same on every run and on every machine.
struct Draw(u64);
impl Draw {
    fn next(&mut self) -> u64 {
        // xorshift64*, enough for placing rows and picking queries.
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

/// The register, struct-of-arrays: a holding is a ROW and a lot is a slice of one flat column.
struct Register {
    party: Vec<u32>,
    instrument: Vec<u32>,
    lot_at: Vec<u32>,
    lot_len: Vec<u32>,
    lot_qty: Vec<f64>,
    /// (party, instrument) -> row. The one hash in the design, on a packed integer key.
    row_of: HashMap<u64, u32>,
}

const fn key(party: u32, instrument: u32) -> u64 {
    ((party as u64) << 32) | (instrument as u64)
}

fn build(draw: &mut Draw) -> Register {
    let mut r = Register {
        party: Vec::with_capacity(HOLDINGS),
        instrument: Vec::with_capacity(HOLDINGS),
        lot_at: Vec::with_capacity(HOLDINGS),
        lot_len: Vec::with_capacity(HOLDINGS),
        lot_qty: Vec::with_capacity(LOTS),
        row_of: HashMap::with_capacity(HOLDINGS * 2),
    };
    // 657,785 lots over 544,104 holdings is 1.209 each: most hold one, some hold several.
    let mut placed = 0usize;
    while r.party.len() < HOLDINGS {
        let p = draw.below(PARTIES);
        let i = draw.below(INSTRUMENTS);
        let k = key(p, i);
        if r.row_of.contains_key(&k) {
            continue;
        }
        let row = r.party.len() as u32;
        let lots = if placed + 4 < LOTS && draw.next() % 5 == 0 { 2 } else { 1 };
        r.lot_at.push(placed as u32);
        r.lot_len.push(lots);
        for _ in 0..lots {
            r.lot_qty.push(((draw.next() % 1_000_000) as f64) / 100.0);
            placed += 1;
        }
        r.party.push(p);
        r.instrument.push(i);
        r.row_of.insert(k, row);
    }
    while placed < LOTS {
        r.lot_qty.push(1.0);
        placed += 1;
    }
    r
}

impl Register {
    /// `Register.quantity(holder, instrument)`: find the row, sum its lots. The engine's hot read.
    #[inline]
    fn quantity(&self, party: u32, instrument: u32) -> f64 {
        match self.row_of.get(&key(party, instrument)) {
            None => 0.0,
            Some(&row) => {
                let at = self.lot_at[row as usize] as usize;
                let len = self.lot_len[row as usize] as usize;
                let mut held = 0.0;
                for q in &self.lot_qty[at..at + len] {
                    held += *q;
                }
                held
            }
        }
    }
}

fn main() {
    let mut draw = Draw(0x9E37_79B9_7F4A_7C15);
    let built = Instant::now();
    let reg = build(&mut draw);
    let build_ms = built.elapsed().as_secs_f64() * 1000.0;

    // 1. THE HOT READ. TypeScript, measured: `quantity` 1,216 ms + `free` 349 ms of self time
    //    over ops.holding = 23,304,012 calls, so 67.2 ns a call.
    let mut queries = Vec::with_capacity(QUANTITY_READS);
    for _ in 0..QUANTITY_READS {
        let row = (draw.next() as usize) % HOLDINGS;
        queries.push((reg.party[row], reg.instrument[row]));
    }
    let t = Instant::now();
    let mut sink = 0.0f64;
    for &(p, i) in &queries {
        sink += reg.quantity(p, i);
    }
    let read_ns = t.elapsed().as_secs_f64() * 1e9 / QUANTITY_READS as f64;

    // 2. A RECORD FETCHED BY ID. TypeScript, measured: `instruments.get` 1,847 ms over 38,676,668
    //    calls on a 16,750-entry Map<string, Instrument>, so 47.8 ns a call.
    let kinds: Vec<u8> = (0..INSTRUMENTS).map(|_| (draw.next() % 24) as u8).collect();
    let mut picks = Vec::with_capacity(INSTRUMENT_READS);
    for _ in 0..INSTRUMENT_READS {
        picks.push(draw.below(INSTRUMENTS));
    }
    let t = Instant::now();
    let mut kind_sink = 0u64;
    for &i in &picks {
        kind_sink += u64::from(kinds[i as usize]);
    }
    let fetch_ns = t.elapsed().as_secs_f64() * 1e9 / INSTRUMENT_READS as f64;

    // 3. THE FULL TRAVERSAL every audit family makes. TypeScript, measured: `allHoldings` 957 ms of
    //    self time building the list, and `ownership` alone is exactly 544,104 reads — one pass.
    let t = Instant::now();
    let mut rounds = 0;
    let mut walk_sink = 0.0f64;
    for _ in 0..10 {
        for row in 0..HOLDINGS {
            let at = reg.lot_at[row] as usize;
            let len = reg.lot_len[row] as usize;
            for q in &reg.lot_qty[at..at + len] {
                walk_sink += *q;
            }
        }
        rounds += 1;
    }
    let walk_ms = t.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);

    // 4. THE SAME READ WITH THE ROW ALREADY IN HAND — what the native design actually does, because
    //    an id that is a ROW INDEX is hashed once at the boundary and never again.
    let rows: Vec<u32> = queries.iter().map(|&(p, i)| reg.row_of[&key(p, i)]).collect();
    let t = Instant::now();
    let mut row_sink = 0.0f64;
    for &row in &rows {
        let at = reg.lot_at[row as usize] as usize;
        let len = reg.lot_len[row as usize] as usize;
        for q in &reg.lot_qty[at..at + len] {
            row_sink += *q;
        }
    }
    let row_ns = t.elapsed().as_secs_f64() * 1e9 / QUANTITY_READS as f64;

    // 5. AND WITH A CHEAP HASH, to say whether the hashing or the cache misses cost read 1.
    let mut table = vec![u32::MAX; (HOLDINGS * 2).next_power_of_two()];
    let mask = table.len() - 1;
    for row in 0..HOLDINGS {
        let mut at = (fx(key(reg.party[row], reg.instrument[row])) as usize) & mask;
        while table[at] != u32::MAX {
            at = (at + 1) & mask;
        }
        table[at] = row as u32;
    }
    let t = Instant::now();
    let mut fx_sink = 0.0f64;
    for &(p, i) in &queries {
        let mut at = (fx(key(p, i)) as usize) & mask;
        loop {
            let row = table[at];
            if row == u32::MAX {
                break;
            }
            if reg.party[row as usize] == p && reg.instrument[row as usize] == i {
                let lo = reg.lot_at[row as usize] as usize;
                let len = reg.lot_len[row as usize] as usize;
                for q in &reg.lot_qty[lo..lo + len] {
                    fx_sink += *q;
                }
                break;
            }
            at = (at + 1) & mask;
        }
    }
    let fx_ns = t.elapsed().as_secs_f64() * 1e9 / QUANTITY_READS as f64;

    println!("4. read by row     {row_ns:6.2} ns/op   over {QUANTITY_READS} ops   (TS measured 67.20)");
    println!("5. read cheap hash {fx_ns:6.2} ns/op   over {QUANTITY_READS} ops   (TS measured 67.20)");
    println!("built {HOLDINGS} holdings, {LOTS} lots in {build_ms:.0} ms");
    println!("1. hot read        {read_ns:6.2} ns/op   over {QUANTITY_READS} ops   (TS measured 67.20)");
    println!("2. record by id    {fetch_ns:6.2} ns/op   over {INSTRUMENT_READS} ops   (TS measured 47.80)");
    println!("3. full traversal  {walk_ms:6.2} ms/pass over {HOLDINGS} rows      (TS measured 95.70)");
    println!("checksums {sink:.0} {kind_sink} {walk_sink:.0} {row_sink:.0} {fx_sink:.0}");
}

/// A multiply-xor hash, which is what a native register would key a packed pair with.
#[inline]
const fn fx(k: u64) -> u64 {
    let h = k.wrapping_mul(0x517C_C1B7_2722_0A95);
    h ^ (h >> 32)
}
