/// Folds results in index order over a left-balanced pairwise tree whose shape depends only on their number: the
/// left subtree of `n` results holds the largest power of two below `n`. None for no results.
pub fn reduce_tree<T>(results: Vec<T>, f: impl Fn(T, T) -> T) -> Option<T> {
    let n = results.len();
    let mut items = results.into_iter();
    (n > 0).then(|| fold(&mut items, n, &f)).flatten()
}

fn fold<T>(items: &mut impl Iterator<Item = T>, n: usize, f: &impl Fn(T, T) -> T) -> Option<T> {
    if n == 1 {
        return items.next();
    }
    let left = n.next_power_of_two() / 2;
    let a = fold(items, left, f)?;
    let b = fold(items, n - left, f)?;
    Some(f(a, b))
}

#[cfg(test)]
mod tests {
    use super::reduce_tree;
    use crate::pool::Pool;
    use crate::spec::PoolSpec;

    #[test]
    fn tree_reduce_fixed_shape() {
        // Float addition is not associative: equal bits need the same tree and the same inputs in the same places.
        let sum = |workers: usize| {
            let pool = Pool::new(&PoolSpec::unpinned(workers)).unwrap();
            let parts = pool.map(1000, |i| {
                let x = f64::from(u32::try_from(i).unwrap());
                (0..100).map(|k| (x * 1e-3 + f64::from(k)).sin() * 1e10).sum::<f64>()
            });
            reduce_tree(parts, |a, b| a + b).unwrap().to_bits()
        };
        let one = sum(1);
        assert_eq!(sum(8), one);
        assert_eq!(sum(3), one);
    }

    #[test]
    fn tree_shape_depends_only_on_count() {
        let shape = |n: usize| reduce_tree((0..n).map(|i| i.to_string()).collect(), |a, b| format!("({a}+{b})"));
        assert_eq!(shape(0), None);
        assert_eq!(shape(1).as_deref(), Some("0"));
        assert_eq!(shape(5).as_deref(), Some("(((0+1)+(2+3))+4)"));
        assert_eq!(shape(6).as_deref(), Some("(((0+1)+(2+3))+(4+5))"));
    }
}
