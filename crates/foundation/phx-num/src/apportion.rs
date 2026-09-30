//! Apportionment: a whole amount split among claimants in proportion to their weights, exactly — every share a whole
//! number of units and the shares summing to the whole — with the rounding's residue landing by a named rule, so no
//! unit is lost, made or left with no holder.

use phx_macros::clause;

use crate::consts::{ABSENT_I64, SELECT_BUCKETS, SMALL_SPLIT, SMALL_SPLIT_FEW};
use crate::round::{Round, div_round};
use crate::violation;

/// How the units the floors leave fall among equal remainders: by the claimants' declared order, the lower index
/// first, or by keys the caller drew from its named stream, the lower key first and equal keys by the order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ties<'a> {
    Order,
    Keys(&'a [u64]),
}

/// Where the rounding's residue lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Residue<'a> {
    /// Every share floored and the units left given one each to the largest remainders, compared exactly.
    LargestRemainder { ties: Ties<'a> },
    /// Every share rounded by the governing convention and the whole residue on the claimant at `index`.
    To { index: usize, round: Round },
}

/// `total` split over `weights` into `out`, a share a claimant in their order, summing to `total` by the rule's
/// residue. A negative total — a loss shared — is split as its size and negated, so its residue lands by the same
/// rule. No claimant, or no weight, is no split: the caller's rule names where the whole goes, so it stops the run, as
/// does a sum of weights or a share past its word, an absent total, or `out` or the keys not one a claimant.
#[clause("MON.16")]
pub fn apportion(total: i64, weights: &[u64], rule: Residue<'_>, out: &mut [i64]) {
    if out.len() != weights.len() {
        violation!(clause = "MON.16", "shares not one a claimant", claimants = weights.len(), shares = out.len());
    }
    if let Residue::LargestRemainder { ties: Ties::Keys(keys) } = rule
        && keys.len() != weights.len()
    {
        violation!(clause = "MON.16", "tie keys not one a claimant", claimants = weights.len(), keys = keys.len());
    }
    if total == ABSENT_I64 {
        violation!(clause = "MON.16", "an absent amount apportioned");
    }
    let Some(whole) = weights.iter().try_fold(0_u64, |sum, w| sum.checked_add(*w)) else {
        violation!(clause = "MON.16", "a split's weights past a word", claimants = weights.len());
    };
    if whole == 0 {
        violation!(clause = "MON.16", "a split with no claimant to hold it", claimants = weights.len());
    }
    match rule {
        Residue::LargestRemainder { ties } => largest_remainder(total, (weights, whole), ties, out),
        Residue::To { index, round } => residue_to(total, (weights, whole), (index, round), out),
    }
}

/// The full product of two words, which a pair of words always holds.
fn wide(a: u64, b: u64) -> u128 {
    u128::from(a) * u128::from(b)
}

/// A total's share of a whole found by multiplying, not dividing: `t = whole·q + part`, and `part / whole` held as a
/// 64-bit binary fraction `f = floor(part·2⁶⁴ / whole)`, one division for the split. A weight's floor is `q·w` plus
/// `floor(w·f / 2⁶⁴)`, which falls short of the exact `floor(w·part / whole)` by at most one, since `f` is short of the
/// fraction by less than 2⁻⁶⁴ and `w` below 2⁶⁴; the remainder, found by multiplying back, says whether it did.
#[derive(Clone, Copy)]
struct Divider {
    q: u64,
    part: u64,
    fraction: u64,
    whole: u64,
}

impl Divider {
    fn new(t: u64, whole: u64) -> Divider {
        let (q, part) = (t / whole, t % whole);
        let fraction = (u128::from(part) << u64::BITS) / u128::from(whole);
        match u64::try_from(fraction) {
            Ok(fraction) => Divider { q, part, fraction, whole },
            Err(_) => violation!(clause = "MON.16", "a part of the whole past the whole", part = part),
        }
    }

    /// `t·w / whole` as its floor and remainder: the floor at most `t`, since `w` is at most `whole`, and the
    /// remainder below `whole`.
    fn split(self, w: u64) -> (u64, u64) {
        let high = wide(self.fraction, w) >> u64::BITS;
        let exact = wide(self.part, w);
        let (mut floor, mut rem) = match u64::try_from(high) {
            Ok(estimate) => (estimate, exact - wide(estimate, self.whole)),
            Err(_) => violation!(clause = "MON.16", "a share's estimate past its word", weight = w),
        };
        if rem >= u128::from(self.whole) {
            (floor, rem) = (floor + 1, rem - u128::from(self.whole));
        }
        match u64::try_from(rem) {
            Ok(rem) => (self.q * w + floor, rem),
            Err(_) => violation!(clause = "MON.16", "a remainder past its divisor", weight = w),
        }
    }
}

fn largest_remainder(total: i64, (weights, whole): (&[u64], u64), ties: Ties<'_>, out: &mut [i64]) {
    let t = total.unsigned_abs();
    if weights.len() <= SMALL_SPLIT_FEW {
        split_on_stack::<SMALL_SPLIT_FEW>(t, (weights, whole), ties, out);
    } else if weights.len() <= SMALL_SPLIT {
        split_on_stack::<SMALL_SPLIT>(t, (weights, whole), ties, out);
    } else {
        split_in_place(t, (weights, whole), ties, out);
    }
    if total < 0 {
        for o in out.iter_mut() {
            *o = -*o;
        }
    }
}

/// A floor as a share, which fits one since it is at most the total.
fn share_of(floor: u64) -> i64 {
    match i64::try_from(floor) {
        Ok(f) => f,
        Err(_) => violation!(clause = "MON.16", "a share past its word", share = floor),
    }
}

/// A split among at most `N` claimants, `N` a power of two: their floors and remainders in one pass, the remainders
/// held on the stack; the units left go to the remainders in the top buckets of their leading bits, one bucket a
/// claimant, and within the bucket the cut falls in, by the exact order (remainder descending, tie key ascending,
/// index ascending) among its few.
fn split_on_stack<const N: usize>(t: u64, (weights, whole): (&[u64], u64), ties: Ties<'_>, out: &mut [i64]) {
    let mut rems = [0_u64; N];
    let mut floors = 0_u64;
    let divider = Divider::new(t, whole);
    for ((w, o), r) in weights.iter().zip(out.iter_mut()).zip(rems.iter_mut()) {
        let (floor, rem) = divider.split(*w);
        floors += floor;
        (*o, *r) = (share_of(floor), rem);
    }
    // The floors fall short of the total by fewer units than there are claimants, each one remainder's worth.
    let left = t - floors;
    if left == 0 {
        return;
    }
    let Some(rems) = rems.get(..weights.len()) else {
        violation!(clause = "MON.16", "a split past its stack", claimants = weights.len());
    };
    // A remainder is below the whole, so shifted up by the whole's leading zeros its top bits name one of `N`
    // buckets.
    let (up, shift) = (whole.leading_zeros(), u64::BITS - N.trailing_zeros());
    let mut counts = [0_u64; N];
    for r in rems {
        if let Some(c) = usize::try_from(*r << up >> shift).ok().and_then(|b| counts.get_mut(b)) {
            *c += 1;
        }
    }
    let (mut cut, mut need) = (N, left);
    let whole_bucket = loop {
        let Some(b) = cut.checked_sub(1) else {
            violation!(clause = "MON.16", "fewer claimants than the units to give", units = left);
        };
        cut = b;
        let Some(&count) = counts.get(b) else { violation!(clause = "MON.16", "a bucket past its counts") };
        if count >= need {
            break count == need;
        }
        need -= count;
    };
    let key = |i: usize| match ties {
        Ties::Order => 0,
        Ties::Keys(keys) => keys.get(i).copied().into_iter().sum(),
    };
    let before = |(j, rj): (usize, u64), (i, ri): (usize, u64)| rj > ri || rj == ri && (key(j), j) < (key(i), i);
    let Ok(cut) = u64::try_from(cut) else { violation!(clause = "MON.16", "a bucket past a word") };
    // A unit to every claimant above the cut's bucket without a branch, since which claimants are is as random as
    // their remainders; only those in the cut's bucket, a few, are ranked.
    for (i, (o, r)) in out.iter_mut().zip(rems).enumerate() {
        let b = *r << up >> shift;
        let mut takes = b > cut;
        if b == cut {
            takes = whole_bucket || {
                let ahead =
                    rems.iter().enumerate().filter(|(j, rj)| **rj << up >> shift == cut && before((*j, **rj), (i, *r)));
                u64::try_from(ahead.count()).is_ok_and(|a| a < need)
            };
        }
        *o += i64::from(takes);
    }
}

/// A split among many claimants: each remainder held in its share's place while the units left are given, selecting
/// a byte at a time over them, and each floor found again, by multiplying, as the share is written; so nothing is
/// held beside the shares.
fn split_in_place(t: u64, (weights, whole): (&[u64], u64), ties: Ties<'_>, out: &mut [i64]) {
    let divider = Divider::new(t, whole);
    let mut floors = 0_u64;
    for (w, o) in weights.iter().zip(out.iter_mut()) {
        let (floor, rem) = divider.split(*w);
        floors += floor;
        *o = i64::from_ne_bytes(rem.to_ne_bytes());
    }
    let rem_at = |i: usize| out.get(i).map(|o| u64::from_ne_bytes(o.to_ne_bytes()));
    let left = t - floors;
    let mut chosen = (left > 0).then(|| choose::<SELECT_BUCKETS>(weights.len(), (left, whole), &rem_at, ties));
    for (w, o) in weights.iter().zip(out.iter_mut()) {
        let rem = u64::from_ne_bytes(o.to_ne_bytes());
        let takes = chosen.as_mut().is_some_and(|c| c.takes(rem));
        *o = share_of(divider.split(*w).0) + i64::from(takes);
    }
}

/// Which of the claimants whose remainder equals the cut take a unit.
#[derive(Clone, Copy)]
enum Pick<'a> {
    All,
    First,
    Keyed { keys: &'a [u64], key_cut: u64, first: Option<u64> },
}

/// The claimants chosen for the units left, told one at a time in their order: those above the cut, and those at it
/// the pick names.
struct Chosen<'a> {
    cut: u64,
    pick: Pick<'a>,
    at: usize,
    first_left: u64,
}

impl Chosen<'_> {
    /// Whether the next claimant in order, of remainder `rem`, takes a unit.
    fn takes(&mut self, rem: u64) -> bool {
        let i = self.at;
        self.at += 1;
        if rem != self.cut {
            return rem > self.cut;
        }
        match self.pick {
            Pick::All => true,
            Pick::First => take_one(&mut self.first_left),
            Pick::Keyed { keys, key_cut, first } => match keys.get(i).map(|k| !*k) {
                Some(flipped) if flipped > key_cut => true,
                Some(flipped) if flipped == key_cut => first.is_none() || take_one(&mut self.first_left),
                _ => false,
            },
        }
    }
}

/// The cut among `n` claimants' remainders for `left` units, counting `B` digits a pass, and the pick at it by the
/// ties' rule: the lower key first being the largest of the keys' complements among the remainders at the cut.
fn choose<'a, const B: usize>(
    n: usize,
    (left, whole): (u64, u64),
    rem: &impl Fn(usize) -> Option<u64>,
    ties: Ties<'a>,
) -> Chosen<'a> {
    let bits = u64::BITS - whole.leading_zeros();
    let (cut, equal_taken) = kth_largest::<B>(n, (left, bits), rem);
    let (pick, first_left) = match (equal_taken, ties) {
        (None, _) => (Pick::All, 0),
        (Some(k), Ties::Order) => (Pick::First, k),
        (Some(k), Ties::Keys(keys)) => {
            let flipped = |i: usize| match (rem(i), keys.get(i)) {
                (Some(r), Some(key)) if r == cut => Some(!*key),
                _ => None,
            };
            let (key_cut, first) = kth_largest::<B>(n, (k, u64::BITS), &flipped);
            (Pick::Keyed { keys, key_cut, first }, first.into_iter().sum())
        }
    };
    Chosen { cut, pick, at: 0, first_left }
}

/// One of `left` taken, while any is.
fn take_one(left: &mut u64) -> bool {
    if *left == 0 {
        return false;
    }
    *left -= 1;
    true
}

/// The value at which the `k` largest keys among the candidates end, found a digit of `B` values at a time from the
/// top of their `bits` over counts kept on the stack, with no allocation: every key above it is among them, and of
/// the keys equal to it all are (`None`) or the returned count. `k` is at least one and at most the candidates.
fn kth_largest<const B: usize>(
    n: usize,
    (mut k, bits): (u64, u32),
    key: &impl Fn(usize) -> Option<u64>,
) -> (u64, Option<u64>) {
    let digit_bits = B.trailing_zeros();
    let Ok(digit_mask) = u64::try_from(B - 1) else { violation!(clause = "MON.16", "a digit past a word") };
    let (mut prefix, mut mask) = (0_u64, 0_u64);
    let mut shift = bits.div_ceil(digit_bits) * digit_bits;
    while shift > 0 {
        shift -= digit_bits;
        let mut counts = [0_u64; B];
        for v in (0..n).filter_map(key) {
            if v & mask == prefix
                && let Some(c) = usize::try_from((v >> shift) & digit_mask).ok().and_then(|d| counts.get_mut(d))
            {
                *c += 1;
            }
        }
        let mut digit = B;
        let at_cut = loop {
            let Some(d) = digit.checked_sub(1) else {
                violation!(clause = "MON.16", "fewer candidates than the units to give", units = k);
            };
            digit = d;
            let Some(&count) = counts.get(d) else { violation!(clause = "MON.16", "a digit past its counts") };
            if count >= k {
                break count;
            }
            k -= count;
        };
        let Ok(d) = u64::try_from(digit) else { violation!(clause = "MON.16", "a digit past a word") };
        prefix |= d << shift;
        mask |= digit_mask << shift;
        if at_cut == k {
            return (prefix, None);
        }
    }
    (prefix, Some(k))
}

fn residue_to(total: i64, (weights, whole): (&[u64], u64), (index, round): (usize, Round), out: &mut [i64]) {
    if index >= out.len() {
        violation!(clause = "MON.16", "a residue landing on no claimant", index = index, claimants = out.len());
    }
    let mut rest = i128::from(total);
    for (i, (w, o)) in weights.iter().zip(out.iter_mut()).enumerate() {
        if i == index {
            continue;
        }
        let share = div_round(i128::from(total) * i128::from(*w), i128::from(whole), round);
        rest -= share;
        *o = match i64::try_from(share) {
            Ok(s) => s,
            Err(_) => violation!(clause = "MON.16", "a share past its word", claimant = i),
        };
    }
    let Some(named) = out.get_mut(index) else {
        violation!(clause = "MON.16", "a residue landing on no claimant", index = index);
    };
    *named = match i64::try_from(rest) {
        Ok(r) => r,
        Err(_) => violation!(clause = "MON.16", "a residue past its word", index = index),
    };
}

#[cfg(test)]
#[path = "apportion_tests.rs"]
mod tests;
