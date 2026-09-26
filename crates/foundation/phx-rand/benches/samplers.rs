#![expect(clippy::disallowed_macros, clippy::disallowed_methods, reason = "gungraun's harness prints and exits")]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_num::Missing;
use phx_rand::{
    AliasTable, Draws, Seed, Subject, SubjectTag, accept, below_u64, beta, binomial, binomial_at_least_one,
    binomials_joint_at_least_one, gamma, geometric, hypergeometric, multinomial, multivariate_hypergeometric, normal,
    open_unit, philox, philox_x4, pick_without_replacement, stream_key,
};

fn draws() -> Draws {
    Draws::new(stream_key(Seed::new(7), "bench"), Subject::new(SubjectTag::Party, 42), 3, 5)
}

fn table() -> (Draws, AliasTable) {
    (draws(), AliasTable::new(&[3.0, 0.5, 1.0, 6.0, 0.25, 9.0, 2.0, 1.0]))
}

#[library_benchmark]
fn ir_philox() -> [u32; 4] {
    black_box(philox(black_box([1, 2, 3, 4]), black_box([5, 6])))
}

#[library_benchmark]
fn ir_philox_x4() -> [[u32; 4]; 4] {
    black_box(philox_x4(black_box([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 16]]), black_box([5, 6])))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_open_unit(mut d: Draws) -> f64 {
    black_box(open_unit(&mut d))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_below_u64(mut d: Draws) -> u64 {
    black_box(below_u64(&mut d, black_box(1_000_003)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_binomial_inversion(mut d: Draws) -> u64 {
    black_box(binomial(&mut d, black_box(40), black_box(0.05)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_binomial_btpe(mut d: Draws) -> u64 {
    black_box(binomial(&mut d, black_box(1_000), black_box(0.5)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_binomial_at_least_one(mut d: Draws) -> u64 {
    black_box(binomial_at_least_one(&mut d, black_box(12), black_box(0.01)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_joint_at_least_one(mut d: Draws) -> [u64; 3] {
    let mut out = [0; 3];
    binomials_joint_at_least_one(&mut d, black_box(&[3, 5, 2]), black_box(&[0.01, 0.2, 0.05]), &mut out);
    black_box(out)
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_multinomial(mut d: Draws) -> [u64; 5] {
    let mut out = [0; 5];
    multinomial(&mut d, black_box(37), black_box(&[0.1, 0.05, 0.25, 0.4, 0.2]), &mut out);
    black_box(out)
}

#[library_benchmark]
#[bench::fresh(table())]
fn ir_alias_draw(setup: (Draws, AliasTable)) -> usize {
    let (mut d, t) = setup;
    black_box(t.draw(&mut d))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_hypergeometric_hin(mut d: Draws) -> u64 {
    black_box(hypergeometric(&mut d, black_box(50), black_box(20), black_box(10)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_hypergeometric_hrua(mut d: Draws) -> u64 {
    black_box(hypergeometric(&mut d, black_box(1_000), black_box(300), black_box(100)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_multivariate_hypergeometric(mut d: Draws) -> [u64; 5] {
    let mut out = [0; 5];
    multivariate_hypergeometric(&mut d, black_box(&[5, 0, 17, 3, 40]), black_box(12), &mut out);
    black_box(out)
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_pick_without_replacement(mut d: Draws) -> Vec<u64> {
    let counts: Vec<u64> = (0..64).map(|i| 1 + i % 7).collect();
    let mut out = vec![0; 64];
    pick_without_replacement(&mut d, black_box(&counts), black_box(8), (&mut out, &mut phx_rand::Fenwick::default()));
    black_box(out)
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_geometric(mut d: Draws) -> Missing<u64> {
    black_box(geometric(&mut d, black_box(0.01)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_accept(mut d: Draws) -> bool {
    black_box(accept(&mut d, black_box(0.3)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_normal(mut d: Draws) -> f64 {
    black_box(normal(&mut d))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_gamma(mut d: Draws) -> f64 {
    black_box(gamma(&mut d, black_box(3.0), black_box(1.5)))
}

#[library_benchmark]
#[bench::fresh(draws())]
fn ir_beta(mut d: Draws) -> f64 {
    black_box(beta(&mut d, black_box(2.0), black_box(5.0)))
}

library_benchmark_group!(
    name = samplers,
    benchmarks = [
        ir_philox,
        ir_philox_x4,
        ir_open_unit,
        ir_below_u64,
        ir_binomial_inversion,
        ir_binomial_btpe,
        ir_binomial_at_least_one,
        ir_joint_at_least_one,
        ir_multinomial,
        ir_alias_draw,
        ir_hypergeometric_hin,
        ir_hypergeometric_hrua,
        ir_multivariate_hypergeometric,
        ir_pick_without_replacement,
        ir_geometric,
        ir_accept,
        ir_normal,
        ir_gamma,
        ir_beta
    ]
);

main!(library_benchmark_groups = samplers);
