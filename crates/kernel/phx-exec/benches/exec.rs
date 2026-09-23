#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::needless_pass_by_value,
    reason = "gungraun's harness prints, exits and passes setups by value"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_exec::{IntentBuf, KeyedReduce, gather, mix64, radix_sort};

/// Kernels run on the calling thread: a pool's idle workers would add instructions that depend on timing.
fn random_pairs(n: u64) -> Vec<(u64, u32)> {
    (0..n).map(|i| (mix64(i), u32::try_from(i % 1_000_000).unwrap_or(0))).collect()
}

fn keyed_chunks() -> Vec<Vec<(u64, u64)>> {
    (0..8_u64).map(|c| (0..512).map(|i| (mix64(c * 512 + i) % 1000, i)).collect()).collect()
}

fn filled_intents() -> IntentBuf<u64> {
    let mut buf = IntentBuf::new(2);
    buf.reset(16);
    for (c, handlers) in (0_u64..).zip(buf.chunks_mut()) {
        for (h, b) in (0_u64..).zip(handlers.iter_mut()) {
            b.extend((0..128).map(|i| c * 1000 + h * 100 + i));
        }
    }
    buf
}

#[library_benchmark]
#[bench::fresh(random_pairs(4096))]
fn ir_radix_sort(mut pairs: Vec<(u64, u32)>) -> usize {
    radix_sort(None, &mut pairs, &mut Vec::new());
    black_box(pairs.len())
}

#[library_benchmark]
#[bench::fresh(keyed_chunks())]
fn ir_keyed_reduce(chunks: Vec<Vec<(u64, u64)>>) -> usize {
    let out = KeyedReduce::run(None, &chunks, |acc: &mut u64, v| *acc += v);
    black_box(out.shards.len())
}

#[library_benchmark]
#[bench::fresh(filled_intents())]
fn ir_gather(intents: IntentBuf<u64>) -> usize {
    let mut out = Vec::new();
    gather(None, &intents, &mut out);
    black_box(out.len())
}

library_benchmark_group!(name = exec, benchmarks = [ir_radix_sort, ir_keyed_reduce, ir_gather]);

main!(library_benchmark_groups = exec);
