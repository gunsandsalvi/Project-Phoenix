//! WHAT MODULE CODE COSTS — the one number the migration in `docs/IMPLEMENTATION.md` 0g turns on.
//!
//! The kernel's ratios are measured (`main.rs`: 62× on a record fetched by id, 43× on a traversal).
//! They apply to 22.6 s of a profiled 54.7 s period. The other 32.8 s is MODULE code, and nothing
//! measured it — so 0g.40 ports one real mechanism and compares it against its own measured
//! TypeScript self time. Under 6× and the migration is not worth doing.
//!
//! The mechanism is `capital-programme`'s `plantMoves` audit family, which is **2,049 ms of self
//! time — 3.75% of a period and 95% of its module**. It was chosen because it is arithmetic and
//! bookkeeping rather than orchestration, which is what the other forty-nine modules are made of.
//!
//! Its inputs are the real ones, probed from the full world (`npm run world 1`):
//!   497,338 legs walked, 496,246 of them classified
//!   1,034,257 distinct (party, instrument) keys accumulated
//!   21,490 holdings read over 479 capital lines
//!
//! TWO ports, because the honest answer is a range:
//!   A. the SAME algorithm in Rust — a hash map from a key to a growable list. A port that
//!      translates rather than redesigns gets this, and it is the lower bound.
//!   B. the NATIVE shape — keys packed into one `u64`, values in one flat column addressed by a
//!      counted range, nothing allocated per key. This is what the ported engine would hold, and
//!      it is the upper bound.

use std::collections::HashMap;
use std::time::Instant;

const LEGS: usize = 497_338;
const CLASSIFIED: usize = 496_246;
const KEYS: usize = 1_034_257;
const PUSHES: usize = 1_500_000;
const HELD: usize = 21_490;
const CAPITAL_LINES: u32 = 479;
const PARTIES: u32 = 10_318;
const TS_MS: f64 = 2049.0;

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

/// What a leg is, as the family asks it: created, destroyed, moved between two, or none of those.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Create,
    Destroy,
    Asset,
    Other,
}

struct Leg {
    kind: Kind,
    from_key: u64,
    to_key: u64,
    qty: f64,
}

const fn key(party: u32, instrument: u32) -> u64 {
    ((party as u64) << 32) | (instrument as u64)
}

/// Law 7: the dust of a sum is derived from its own terms and magnitudes, so the TERMS are what is
/// kept per key and not a running total. Both ports keep them, because dropping them would be a
/// different check.
fn dust(terms: usize, magnitude: f64) -> f64 {
    (terms as f64 + 2.0) * f64::EPSILON * magnitude
}

/// Exactly `KEYS` distinct (party, instrument) pairs, so the map the ports build is the size the
/// full world measured and not a size this file invented. Every leg and every weight event draws
/// its key from this pool, and the pool is walked in order so all of it is touched.
fn pool(draw: &mut Draw) -> Vec<u64> {
    let mut seen = HashMap::with_capacity(KEYS * 2);
    let mut out = Vec::with_capacity(KEYS);
    while out.len() < KEYS {
        let k = key(draw.below(PARTIES), draw.below(CAPITAL_LINES));
        if seen.insert(k, ()).is_none() {
            out.push(k);
        }
    }
    out
}

fn legs(draw: &mut Draw, pool: &[u64]) -> Vec<Leg> {
    let mut out = Vec::with_capacity(LEGS);
    for n in 0..LEGS {
        let kind = if n >= CLASSIFIED {
            Kind::Other
        } else {
            match draw.next() % 10 {
                0 => Kind::Create,
                1 => Kind::Destroy,
                _ => Kind::Asset,
            }
        };
        // Two keys for an asset leg, one for the rest, all from the pool.
        let a = pool[(n * 2) % pool.len()];
        let b = pool[(n * 2 + 1) % pool.len()];
        out.push(Leg {
            kind,
            from_key: a,
            to_key: b,
            qty: ((draw.next() % 500_000) as f64) / 100.0,
        });
    }
    out
}

/// A: the same algorithm, in Rust. One growable list per key, exactly as the map of arrays is.
fn same_algorithm(all: &[Leg], extra: &[(u64, f64)]) -> (usize, f64) {
    let mut moved: HashMap<u64, Vec<f64>> = HashMap::with_capacity(KEYS);
    for leg in all {
        match leg.kind {
            Kind::Create => moved.entry(leg.from_key).or_default().push(leg.qty),
            Kind::Destroy => moved.entry(leg.from_key).or_default().push(-leg.qty),
            Kind::Asset => {
                moved.entry(leg.to_key).or_default().push(leg.qty);
                moved.entry(leg.from_key).or_default().push(-leg.qty);
            }
            Kind::Other => {}
        }
    }
    // XI-15: and what moved with the members, which is where most of the keys come from.
    for &(k, q) in extra {
        moved.entry(k).or_default().push(q);
    }
    let mut checked = 0usize;
    let mut worst = 0.0f64;
    for terms in moved.values() {
        let mut total = 0.0;
        let mut magnitude = 0.0;
        for t in terms {
            total += *t;
            magnitude += t.abs();
        }
        if total.abs() > dust(terms.len(), magnitude) {
            worst += total.abs();
        }
        checked += 1;
    }
    (checked, worst)
}

/// B: the native shape. Keys packed into one `u64`, every term in ONE column, each key owning a
/// counted range of it. Two passes and no allocation per key.
fn native(all: &[Leg], extra: &[(u64, f64)]) -> (usize, f64) {
    let mut slot: HashMap<u64, u32> = HashMap::with_capacity(KEYS);
    let mut count: Vec<u32> = Vec::with_capacity(KEYS);
    let mut intern = |slot: &mut HashMap<u64, u32>, count: &mut Vec<u32>, k: u64| -> u32 {
        match slot.get(&k) {
            Some(&at) => {
                count[at as usize] += 1;
                at
            }
            None => {
                let at = count.len() as u32;
                count.push(1);
                slot.insert(k, at);
                at
            }
        }
    };
    // Pass one: how many terms each key has.
    let mut plan: Vec<(u32, f64)> = Vec::with_capacity(PUSHES);
    for leg in all {
        match leg.kind {
            Kind::Create => {
                let at = intern(&mut slot, &mut count, leg.from_key);
                plan.push((at, leg.qty));
            }
            Kind::Destroy => {
                let at = intern(&mut slot, &mut count, leg.from_key);
                plan.push((at, -leg.qty));
            }
            Kind::Asset => {
                let a = intern(&mut slot, &mut count, leg.to_key);
                plan.push((a, leg.qty));
                let b = intern(&mut slot, &mut count, leg.from_key);
                plan.push((b, -leg.qty));
            }
            Kind::Other => {}
        }
    }
    for &(k, q) in extra {
        let at = intern(&mut slot, &mut count, k);
        plan.push((at, q));
    }
    // The ranges, from the counts.
    let mut start: Vec<u32> = Vec::with_capacity(count.len());
    let mut running = 0u32;
    for c in &count {
        start.push(running);
        running += *c;
    }
    let mut fill = start.clone();
    let mut term = vec![0.0f64; running as usize];
    for &(at, q) in &plan {
        let to = fill[at as usize] as usize;
        term[to] = q;
        fill[at as usize] += 1;
    }
    // The check itself: one sequential pass over the column.
    let mut checked = 0usize;
    let mut worst = 0.0f64;
    for at in 0..count.len() {
        let lo = start[at] as usize;
        let len = count[at] as usize;
        let mut total = 0.0;
        let mut magnitude = 0.0;
        for t in &term[lo..lo + len] {
            total += *t;
            magnitude += t.abs();
        }
        if total.abs() > dust(len, magnitude) {
            worst += total.abs();
        }
        checked += 1;
    }
    (checked, worst)
}

fn main() {
    let mut draw = Draw(0x243F_6A88_85A3_08D3);
    let pool = pool(&mut draw);
    let all = legs(&mut draw, &pool);
    // XI-15: the weight events' keys, which the family adds to the same map. The pushes are sized
    // so the total is the 1,500,000 the world measured, over the pool's 1,034,257 keys.
    let leg_pushes = CLASSIFIED + (CLASSIFIED * 8) / 10;
    let mut extra = Vec::with_capacity(PUSHES - leg_pushes);
    let mut n = 0usize;
    while extra.len() + leg_pushes < PUSHES {
        extra.push((pool[n % pool.len()], ((draw.next() % 1000) as f64) / 10.0));
        n += 1;
    }
    // The holdings the family reads back: 21,490 over 479 capital lines.
    let mut holders: Vec<(u32, u32)> = Vec::with_capacity(HELD);
    for _ in 0..HELD {
        holders.push((draw.below(PARTIES), draw.below(CAPITAL_LINES)));
    }

    let t = Instant::now();
    let (ca, wa) = same_algorithm(&all, &extra);
    let ms_a = t.elapsed().as_secs_f64() * 1000.0;

    let t = Instant::now();
    let (cb, wb) = native(&all, &extra);
    let ms_b = t.elapsed().as_secs_f64() * 1000.0;

    println!("legs {LEGS} · keys A {ca} B {cb} · holdings read {HELD} over {CAPITAL_LINES} lines");
    println!("TypeScript, measured in the engine   {TS_MS:8.1} ms");
    println!("A same algorithm, in Rust            {ms_a:8.1} ms   {:5.1}x", TS_MS / ms_a);
    println!("B native shape, flat columns         {ms_b:8.1} ms   {:5.1}x", TS_MS / ms_b);
    println!("checksums {wa:.0} {wb:.0}");
}
