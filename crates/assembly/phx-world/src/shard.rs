//! A list cut into a fixed count of shards for the pool: the cut depends on the list and the count alone, never on
//! the workers, so what each shard holds, and the order its results merge in, is the same on every machine.

/// The `k`th of `shards` near-equal consecutive parts of `items`; empty past the end.
pub(crate) fn part<T>(items: &[T], shards: usize, k: usize) -> &[T] {
    let each = items.len().div_ceil(shards);
    let from = lesser(k * each, items.len());
    items.get(from..lesser(from + each, items.len())).unwrap_or(&[])
}

fn lesser(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[cfg(test)]
mod tests {
    use super::part;

    #[test]
    fn parts_cover_the_list_once_in_order() {
        let items: Vec<u32> = (0..103).collect();
        for shards in [1, 2, 7, 64, 200] {
            let joined: Vec<u32> = (0..shards).flat_map(|k| part(&items, shards, k).iter().copied()).collect();
            assert_eq!(joined, items);
        }
        assert!(part::<u32>(&[], 8, 3).is_empty());
    }
}
