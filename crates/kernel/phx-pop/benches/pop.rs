#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "gungraun's harness prints and exits; a setup that fails is a broken benchmark"
)]

//! A thousand members' steps on a partition of sixty-four boundaries, and a thousand landing keys over sixteen steps
//! and a word of signature: the reads a landing makes for every part.

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_core::register::values::Partition;
use phx_pop::key::KeyId;
use phx_pop::landing::landing_key;
use phx_pop::steps::{Step, StepTable};
use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

const MEMBERS: u64 = 1_000;
const BOUNDARIES: i64 = 64;
const STEPS: usize = 16;

fn draws() -> Draws {
    Draws::new(stream_key(Seed::new(1), "POP.bench"), Subject::new(SubjectTag::World, 0), 1, 0)
}

fn steps_setup() -> (StepTable, Vec<i64>) {
    let bounds: Vec<i64> = (0..BOUNDARIES).map(|i| i * i * 7 - 2_000).collect();
    let table = StepTable::new(&Partition { exp: 2, bounds: bounds.into() }).unwrap();
    let mut d = draws();
    let values = (0..MEMBERS).map(|_| i64::try_from(below_u64(&mut d, 30_000)).unwrap() - 3_000).collect();
    (table, values)
}

fn keys_setup() -> Vec<(KeyId, Vec<Step>, [u64; 1])> {
    let (table, values) = steps_setup();
    let mut d = draws();
    (0..MEMBERS)
        .map(|_| {
            let key = KeyId::new(u32::try_from(below_u64(&mut d, 5_000)).unwrap());
            let steps: Vec<Step> = (0..STEPS).map(|i| table.step_of(*values.get(i * 7).unwrap())).collect();
            (key, steps, [below_u64(&mut d, 64)])
        })
        .collect()
}

#[library_benchmark]
#[bench::thousand(setup = steps_setup)]
fn ir_step_of((table, values): (StepTable, Vec<i64>)) -> u64 {
    values.iter().map(|v| u64::from(black_box(&table).step_of(*v).get())).sum()
}

#[library_benchmark]
#[bench::thousand(setup = keys_setup)]
fn ir_landing_key(parts: Vec<(KeyId, Vec<Step>, [u64; 1])>) -> u64 {
    parts.into_iter().fold(0, |acc, (k, s, sig)| acc ^ landing_key(k, black_box(&s), &sig))
}

library_benchmark_group!(name = pop, benchmarks = [ir_step_of, ir_landing_key]);

main!(library_benchmark_groups = pop);
