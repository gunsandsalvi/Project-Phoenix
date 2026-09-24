#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "gungraun's harness prints and exits; a setup that fails is a broken benchmark"
)]

//! A thousand members' steps on a partition of sixty-four boundaries, a thousand landing keys over sixteen steps and a
//! word of signature, the reads a landing makes for every part; a thousand candidate days of one cell of 170 members
//! in three values, the screen's work on every row the agenda brings to 3b; a part end to end at the design point,
//! split, its origin re-keyed, looked up, checked and joined; and a seller cell of fifty members' spread.

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_core::register::values::Partition;
use phx_core::{GroupDecl, KinkRegistry, PopEntry, PopItem, ProfileComponent, RoleDecl, Weight};
use phx_id::{Day, PartyId, Slot, TableId};
use phx_num::Missing;
use phx_pop::envelope::rung;
use phx_pop::key::KeyId;
use phx_pop::kind::PopKindDecl;
use phx_pop::landing::{land, landing_key};
use phx_pop::profile::Profile;
use phx_pop::screen::{Process, ScreenCounters, screen_candidate};
use phx_pop::seller_spread::spread;
use phx_pop::steps::{Step, StepTable};
use phx_pop::synthetic::{DesignPoint, NoKinks, design_point, levels};
use phx_pop::table::{CellTable, NewCell};
use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};
use phx_store::{AddressSpace, HeapBacking};

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

const AGE: &[ProfileComponent] = &[ProfileComponent { name: "age_band", values: 3 }];

fn cell_setup() -> (CellTable<HeapBacking>, Slot) {
    let entry = |item| PopEntry { system: "DEM", kind: "household", item };
    let entries = [
        entry(PopItem::Role(RoleDecl { name: "person", clause: "REP.26" })),
        entry(PopItem::ProfileGroup(GroupDecl { name: "age", role: "person", components: AGE, clause: "REP.32" })),
    ];
    let steps = |_: &'static str| StepTable::new(&Partition { exp: 0, bounds: [1].into() });
    let kind = PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap();
    let mut space = AddressSpace::empty();
    let mut table = CellTable::new(&mut space, &kind, TableId::new(1), 64, 8);
    let layout = table.profile_layout().clone();
    let mut p = Profile::empty(&layout);
    for (v, n) in [(0, 100), (1, 50), (2, 20)] {
        p.add(&layout, 0, v, n);
    }
    let new = NewCell {
        party: PartyId::new(1),
        created: Day::new(0),
        weight: Weight::new(170),
        key: KeyId::new(0),
        positions: &[],
        profile: &p,
    };
    let slot = table.add(&mut space, new, &kind, &[]);
    (table, slot)
}

#[library_benchmark]
#[bench::thousand(setup = cell_setup)]
fn ir_screen_candidate((table, slot): (CellTable<HeapBacking>, Slot)) -> u64 {
    let rates = [0.0004, 0.0012, 0.003];
    let rate = |v: u32, _: Day| *rates.get(usize::try_from(v).unwrap()).unwrap();
    let window = |_: &[u32], _: Day| (0.003, Missing::Absent);
    let process = Process { group: 0, rate: &rate, envelope: &window };
    let mut counters = ScreenCounters::default();
    let stream = stream_key(Seed::new(1), "DEM.illness");
    for day in 0..u32::try_from(MEMBERS).unwrap() {
        let mut d = Draws::new(stream, Subject::new(SubjectTag::Party, 1), day, 0);
        let _ = black_box(screen_candidate(&table, slot, &process, Day::new(day), rung(170), &mut d, &mut counters));
    }
    counters.candidates
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

fn design_setup() -> DesignPoint<HeapBacking> {
    design_point()
}

/// The design point's stores dropped outside the measurement, which counts the part alone.
fn discard(dp: DesignPoint<HeapBacking>) {
    drop(black_box(dp));
}

/// A part of `members` split, its origin re-keyed, and landed alone.
fn part_alone(mut dp: DesignPoint<HeapBacking>, members: u32) -> DesignPoint<HeapBacking> {
    let part = dp.part(0, members, &mut draws());
    let lv = levels();
    let origin = dp.origin;
    dp.table.rekey(origin, &dp.kind, &lv);
    let mut index = std::mem::take(&mut dp.index);
    let landed = land(&mut dp.tenb(&NoKinks, &lv), &mut index, vec![part]);
    black_box(landed.rows);
    dp.index = index;
    dp
}

#[library_benchmark]
#[bench::ten(args = (design_setup(),), teardown = discard)]
fn ir_part_end_to_end(dp: DesignPoint<HeapBacking>) -> DesignPoint<HeapBacking> {
    part_alone(dp, 10)
}

#[library_benchmark]
#[bench::one(args = (design_setup(),), teardown = discard)]
fn ir_part_lone(dp: DesignPoint<HeapBacking>) -> DesignPoint<HeapBacking> {
    part_alone(dp, 1)
}

/// The members of each part in a batch: eight members leaving one cell alone and landing in one cell together.
const BATCH: [u32; 8] = [1; 8];

#[library_benchmark]
#[bench::eight(args = (design_setup(),), teardown = discard)]
fn ir_parts_batched(mut dp: DesignPoint<HeapBacking>) -> DesignPoint<HeapBacking> {
    let mut streams: Vec<Draws> = (0..8)
        .map(|i| Draws::new(stream_key(Seed::new(1), "POP.bench"), Subject::new(SubjectTag::World, 0), i, 0))
        .collect();
    let parts = dp.parts(&BATCH, &mut streams);
    let lv = levels();
    let origin = dp.origin;
    dp.table.rekey(origin, &dp.kind, &lv);
    let mut index = std::mem::take(&mut dp.index);
    let landed = land(&mut dp.tenb(&NoKinks, &lv), &mut index, parts);
    black_box(landed.rows);
    dp.index = index;
    dp
}

fn spread_setup() -> (Vec<u64>, Vec<(u64, u64)>) {
    let mut d = draws();
    let units = (0..50).map(|_| below_u64(&mut d, 40) + 1).collect();
    (units, vec![(1, 300), (2, 120), (5, 30)])
}

#[library_benchmark]
#[bench::fifty(setup = spread_setup)]
fn ir_seller_spread((units, purchases): (Vec<u64>, Vec<(u64, u64)>)) -> u64 {
    black_box(spread(&mut draws(), &units, &purchases).phases)
}

library_benchmark_group!(
    name = pop,
    benchmarks = [
        ir_step_of,
        ir_landing_key,
        ir_screen_candidate,
        ir_part_end_to_end,
        ir_part_lone,
        ir_parts_batched,
        ir_seller_spread
    ]
);

main!(library_benchmark_groups = pop);
