//! Items applied by range over hand-built balances: every item reaches its target once, in producing-chunk then
//! emission order, the same for any workers and any waves.
#![cfg(test)]

use super::{Item, Shards, apply_by_range, ranges_of};
use crate::hooks::RangeHook;
use crate::pool::Pool;
use crate::spec::PoolSpec;

/// Targets a range holds in these tests: 2^4.
const SHIFT: u32 = 4;

/// A range's own view: its balances, and the (target, tag) of each item it applied, in order.
struct Range<'a> {
    first: u32,
    balances: &'a mut [i64],
    log: Vec<(u32, u32)>,
}

fn apply_all(
    workers: usize,
    chunks: &[Vec<Item>],
    (targets, lane): (usize, usize),
) -> (Vec<i64>, Vec<(u32, u32)>, u64) {
    let pool = (workers > 0).then(|| Pool::new(&PoolSpec::unpinned(workers)).unwrap());
    let mut balances = vec![0_i64; targets];
    let inputs: Vec<&[Item]> = chunks.iter().map(Vec::as_slice).collect();
    let mut shards = Shards::default();
    let mut ranges: Vec<(Range<'_>, ())> = balances
        .chunks_mut(1 << SHIFT)
        .enumerate()
        .map(|(r, b)| (Range { first: u32::try_from(r).unwrap() << SHIFT, balances: b, log: Vec::new() }, ()))
        .collect();
    apply_by_range(pool.as_ref(), (&inputs, SHIFT, lane), &mut ranges, &mut shards, |range, item| {
        range.balances[usize::try_from(item.target - range.first).unwrap()] += item.amount;
        range.log.push((item.target, item.tag));
    });
    let log = ranges.iter().flat_map(|(r, ())| r.log.iter().copied()).collect();
    drop(ranges);
    (balances, log, shards.waves())
}

/// Items spread over chunks: chunk `c`'s `k`th item on target `(c * 7 + k * 13) % targets`, tagged `(c, k)`.
fn made(chunks: u32, per: u32, targets: u32) -> Vec<Vec<Item>> {
    (0..chunks)
        .map(|c| {
            (0..per)
                .map(|k| Item { target: (c * 7 + k * 13) % targets, tag: c << 16 | k, amount: i64::from(k + 1) })
                .collect()
        })
        .collect()
}

#[test]
fn apply_same_for_any_workers() {
    let chunks = made(40, 3_000, 5_000);
    let one = apply_all(0, &chunks, (5_000, usize::MAX));
    for workers in [1, 2, 3, 8] {
        assert_eq!(apply_all(workers, &chunks, (5_000, usize::MAX)), one, "{workers} workers");
    }
}

#[test]
fn shard_order_is_declared() {
    // Two chunks each write target 5 twice; the later chunk's items come after the earlier's, each in emission order.
    let item = |tag| Item { target: 5, tag, amount: 1 };
    let chunks = vec![vec![item(10), item(11)], vec![item(20), item(21)]];
    let (_, log, _) = apply_all(3, &chunks, (32, usize::MAX));
    assert_eq!(log, [(5, 10), (5, 11), (5, 20), (5, 21)]);
}

#[test]
fn two_sided_items_reach_both() {
    // A due's payer side and payee side, each scattered by its own range.
    let due = [Item { target: 3, tag: 0, amount: -250 }, Item { target: 40, tag: 0, amount: 250 }];
    let (balances, _, _) = apply_all(2, &[due.to_vec()], (64, usize::MAX));
    assert_eq!((balances[3], balances[40]), (-250, 250));
    assert_eq!(balances.iter().sum::<i64>(), 0);
}

#[test]
fn flush_units_preserve_items() {
    let chunks = made(17, 1_001, 777);
    let (balances, log, _) = apply_all(4, &chunks, (777, usize::MAX));
    let mut want = vec![0_i64; 777];
    for item in chunks.iter().flatten() {
        want[usize::try_from(item.target).unwrap()] += item.amount;
    }
    assert_eq!(balances, want);
    assert_eq!(log.len(), 17 * 1_001, "every item applied once");
}

#[test]
fn waves_equal_one_pass() {
    let chunks = made(30, 500, 2_000);
    let (b1, l1, w1) = apply_all(3, &chunks, (2_000, usize::MAX));
    let (b2, l2, w2) = apply_all(3, &chunks, (2_000, 1_600));
    assert_eq!((w1, w2), (1, 10), "three chunks a wave of 1 600 items");
    assert_eq!(b2, b1);
    // Each target's items keep their order across waves, as waves follow chunk order.
    let per_target = |l: &[(u32, u32)]| {
        let mut v = l.to_vec();
        v.sort_by_key(|(t, _)| *t);
        v
    };
    assert_eq!(per_target(&l2), per_target(&l1));
}

#[test]
fn target_outside_columns_stops() {
    let chunks = vec![vec![Item { target: 64, tag: 0, amount: 1 }]];
    let caught = std::panic::catch_unwind(|| apply_all(2, &chunks, (64, usize::MAX)));
    assert!(caught.is_err(), "a target past the store's ranges stops the run");
}

#[test]
fn ended_today_still_applies() {
    // The apply reads no liveness: a party that ended today is settled against as its slot stays until the close.
    let chunks = vec![vec![Item { target: 9, tag: 0, amount: 70 }]];
    let (balances, _, _) = apply_all(0, &chunks, (16, usize::MAX));
    assert_eq!(balances[9], 70);
}

#[test]
fn ranges_balanced_for_dense_index() {
    for count in [1, 15, 16, 17, 4_095, 4_096, 4_097, 5_920_000] {
        let ranges = ranges_of(count, 12);
        assert_eq!(ranges, usize::try_from(count.div_ceil(4_096)).unwrap(), "{count}");
        assert!(usize::try_from((count - 1) >> 12).unwrap() < ranges, "the last target has a range");
    }
}

/// A hook counting what it is told, and refusing an item of another range.
#[derive(Default)]
struct Counting {
    range: Option<usize>,
    opened: u32,
    items: u32,
    closed: u32,
}

impl RangeHook for Counting {
    fn open_range(&mut self, range: usize) {
        self.range = Some(range);
        self.opened += 1;
    }
    fn item(&mut self, item: &Item) {
        assert_eq!(Some(usize::try_from(item.target >> SHIFT).unwrap()), self.range);
        self.items += 1;
    }
    fn close_range(&mut self) {
        self.closed += 1;
    }
}

#[test]
fn hook_sees_each_item_once() {
    let chunks = made(12, 400, 1_000);
    let inputs: Vec<&[Item]> = chunks.iter().map(Vec::as_slice).collect();
    let pool = Pool::new(&PoolSpec::unpinned(3)).unwrap();
    let mut ranges: Vec<((), (Counting, Counting))> =
        (0..ranges_of(1_000, SHIFT)).map(|_| ((), (Counting::default(), Counting::default()))).collect();
    apply_by_range(Some(&pool), (&inputs, SHIFT, usize::MAX), &mut ranges, &mut Shards::default(), |(), _| ());
    let seen: u32 = ranges.iter().map(|((), (a, _))| a.items).sum();
    assert_eq!(seen, 12 * 400);
    assert!(ranges.iter().all(|((), (a, b))| a.items == b.items && a.opened == 1 && a.closed == 1 && b.opened == 1));
}
