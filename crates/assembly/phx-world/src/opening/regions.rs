use phx_macros::clause;

/// The regions each country holds, so regions are of like size: each country's quota is its share of the regions; a
/// country whose quota falls short of the floor takes the floor, and the regions left are divided again among the
/// others by their shares, until none falls short; then each takes its quota's whole part, and the regions still
/// left go by largest remainder, ties broken by `lot`, a drawn order of the countries (earlier wins). Shares are
/// whole percent.
///
/// # Errors
/// When the floor alone takes more regions than there are, or the shares are all zero.
#[clause("GEN.14")]
pub fn allot(shares: &[u64], regions: u64, floor: u64, lot: &[usize]) -> Result<Vec<u64>, String> {
    let n = u64::try_from(shares.len()).map_err(|e| e.to_string())?;
    if n.checked_mul(floor).is_none_or(|f| f > regions) {
        return Err(format!("{n} countries of at least {floor} regions need more than {regions}"));
    }
    let mut pinned = vec![false; shares.len()];
    loop {
        let free: Vec<usize> = (0..shares.len()).filter(|i| !pinned.get(*i).copied().unwrap_or(true)).collect();
        let pinned_count = n - u64::try_from(free.len()).map_err(|e| e.to_string())?;
        let left = regions - pinned_count * floor;
        let total: u64 = free.iter().filter_map(|i| shares.get(*i)).sum();
        if total == 0 {
            return Err("no population to allot regions by".to_owned());
        }
        let short: Vec<usize> =
            free.iter().copied().filter(|i| shares.get(*i).is_some_and(|s| s * left < floor * total)).collect();
        if !short.is_empty() {
            for i in short {
                if let Some(p) = pinned.get_mut(i) {
                    *p = true;
                }
            }
            continue;
        }
        let mut out: Vec<u64> = vec![floor; shares.len()];
        let mut remainders = Vec::with_capacity(free.len());
        for i in &free {
            let s = shares.get(*i).copied().unwrap_or(0);
            if let Some(x) = out.get_mut(*i) {
                *x = s * left / total;
            }
            remainders.push((*i, s * left % total));
        }
        let given: u64 = free.iter().filter_map(|i| out.get(*i)).sum();
        let place = |i: usize| lot.iter().position(|l| *l == i).unwrap_or(usize::MAX);
        remainders.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| place(a.0).cmp(&place(b.0))));
        let extra = usize::try_from(left - given).map_err(|e| e.to_string())?;
        for (i, _) in remainders.into_iter().take(extra) {
            if let Some(x) = out.get_mut(i) {
                *x += 1;
            }
        }
        return Ok(out);
    }
}

#[cfg(test)]
mod tests {
    use super::allot;

    #[test]
    fn regions_by_largest_remainder_with_minimum() {
        assert_eq!(allot(&[50, 30, 20], 25, 3, &[0, 1, 2]).unwrap(), vec![13, 7, 5], "a tie of halves, by lot");
        assert_eq!(allot(&[50, 30, 20], 25, 3, &[1, 0, 2]).unwrap(), vec![12, 8, 5]);
        assert_eq!(allot(&[70, 20, 10], 25, 3, &[2, 1, 0]).unwrap(), vec![17, 5, 3], "the smallest held at the floor");
        assert_eq!(allot(&[34, 33, 33], 25, 3, &[2, 1, 0]).unwrap(), vec![9, 8, 8]);
        assert_eq!(allot(&[80, 10, 10], 25, 3, &[0, 1, 2]).unwrap(), vec![19, 3, 3]);
        assert!(allot(&[50, 30, 20], 8, 3, &[0, 1, 2]).is_err());
        for split in [[10, 20, 70], [70, 10, 20], [34, 33, 33], [10, 45, 45]] {
            let r = allot(&split, 25, 3, &[1, 0, 2]).unwrap();
            assert!(r.iter().all(|x| *x >= 3) && r.iter().sum::<u64>() == 25, "{split:?} gives {r:?}");
        }
    }
}
