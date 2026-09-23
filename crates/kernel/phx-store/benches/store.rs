#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::needless_pass_by_value,
    clippy::unwrap_used,
    reason = "gungraun's harness prints, exits and passes setups by value; a setup that fails is a broken benchmark"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_store::{
    AddressSpace, BlockList, BlockPool, ChunkArena, Column, ColumnDescriptor, FieldDescriptor, FieldTag, ListRef,
    Region, Transform, decode_column, encode_column,
};

const ROWS: u32 = 4096;
const ENCODED: u64 = 4096;

const SORTED: &[FieldDescriptor] =
    &[FieldDescriptor { name: "slot", offset: 0, width: 8, transform: Transform::Delta, tag: FieldTag::Plain }];

fn column() -> Column<u64> {
    let mut space = AddressSpace::empty();
    let mut c = Column::new(&mut space, ROWS * 2, ROWS);
    for i in 0..100 {
        c.push(i);
    }
    c
}

fn arena_with_room() -> (ChunkArena, ListRef) {
    let mut space = AddressSpace::empty();
    let mut a = ChunkArena::new(&mut space, 1 << 16);
    let mut list = ListRef::EMPTY;
    a.append(&mut list, &[1, 2, 3, 4, 5]);
    (a, list)
}

/// 64 lists of 16 words, each relocated once so half the arena is dead.
fn arena_to_compact() -> (ChunkArena, Vec<ListRef>, Region<u64>) {
    let mut space = AddressSpace::empty();
    let mut a = ChunkArena::new(&mut space, 1 << 16);
    let mut lists = vec![ListRef::EMPTY; 64];
    for (i, list) in (0_u64..).zip(lists.iter_mut()) {
        a.append(list, &[i; 12]);
    }
    for list in &mut lists {
        a.append(list, &[7; 8]);
    }
    let scratch = Region::reserve(&mut space, 1 << 16);
    (a, lists, scratch)
}

fn pool_with_list() -> (BlockPool, BlockList) {
    let mut space = AddressSpace::empty();
    let mut pool = BlockPool::new(&mut space, 1 << 12);
    let mut list = BlockList::EMPTY;
    for k in 0..10_000 {
        pool.insert_sorted(&mut list, k * 2);
    }
    (pool, list)
}

fn descriptor() -> ColumnDescriptor {
    ColumnDescriptor::checked::<u64>("slots", ROWS, SORTED).unwrap()
}

fn sorted_values() -> Vec<u64> {
    (0..ENCODED).map(|i| i * 3 + i % 5).collect()
}

fn encoded() -> (Vec<u8>, Column<u64>) {
    let mut space = AddressSpace::empty();
    (encode_column(&descriptor(), &sorted_values()), Column::new(&mut space, ROWS * 2, ROWS))
}

#[library_benchmark]
#[bench::fresh(column())]
fn ir_column_push(mut c: Column<u64>) -> usize {
    c.push(black_box(7));
    black_box(c.len())
}

#[library_benchmark]
#[bench::fresh(arena_with_room())]
fn ir_arena_append(setup: (ChunkArena, ListRef)) -> ListRef {
    let (mut a, mut list) = setup;
    a.append(&mut list, black_box(&[6, 7]));
    black_box(list)
}

#[library_benchmark]
#[bench::fresh(arena_to_compact())]
fn ir_arena_compact(setup: (ChunkArena, Vec<ListRef>, Region<u64>)) -> u32 {
    let (mut a, mut lists, mut scratch) = setup;
    a.compact(lists.as_mut_slice(), &mut scratch);
    black_box(a.used_words())
}

#[library_benchmark]
#[bench::fresh(pool_with_list())]
fn ir_block_insert(setup: (BlockPool, BlockList)) -> BlockList {
    let (mut pool, mut list) = setup;
    pool.insert_sorted(&mut list, black_box(10_001));
    black_box(list)
}

#[library_benchmark]
#[bench::fresh(sorted_values())]
fn ir_encode(values: Vec<u64>) -> usize {
    black_box(encode_column(&descriptor(), black_box(&values)).len())
}

#[library_benchmark]
#[bench::fresh(encoded())]
fn ir_decode(setup: (Vec<u8>, Column<u64>)) -> usize {
    let (bytes, mut into) = setup;
    decode_column(&descriptor(), black_box(&bytes), &mut into).unwrap();
    black_box(into.len())
}

library_benchmark_group!(
    name = store,
    benchmarks = [ir_column_push, ir_arena_append, ir_arena_compact, ir_block_insert, ir_encode, ir_decode]
);

main!(library_benchmark_groups = store);
