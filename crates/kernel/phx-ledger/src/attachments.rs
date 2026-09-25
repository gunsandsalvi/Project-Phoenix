//! The households' lines as the opening draws them: each line-owning system's draw, called with each household as
//! its persons are formed, gives the household's rows — on lines of the terms it drew, with a counterparty the
//! household names or one left to the line's derived side — and the key attributes it sets.

use phx_core::{OpeningCountry, OpeningCtx, Register};
use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Draws, Subject};

use crate::algebra::Side;
use crate::books::Books;
use crate::terms::TermsId;

/// A line a household's row lies on: its kind, its terms, the counterparty the household names, if it names one,
/// and the date its dues first fall with that date's index, if it has dates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineSpec {
    pub kind: u16,
    pub terms: TermsId,
    pub counterparty: Missing<PartyId>,
    pub first: Missing<(Day, u32)>,
}

impl LineSpec {
    /// What makes lines distinct: their kind, terms and named counterparty.
    #[must_use]
    pub fn key(&self) -> (u16, u32, bool, u64) {
        match self.counterparty {
            Missing::Present(p) => (self.kind, self.terms.get(), true, p.get()),
            Missing::Absent => (self.kind, self.terms.get(), false, 0),
        }
    }
}

/// Who in a household holds a row: the household, or one of its persons by its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holder {
    Household,
    Person(usize),
}

/// What a row's balance is: nothing, or its share of a pool's total by a weight the household drew, the pool's total
/// apportioned once every household is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Balance {
    None,
    Share { pool: u32, weight: u64 },
}

/// A row a household's draw gives it: its line, the side the household holds, who holds it, and its balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnRow {
    pub line: LineSpec,
    pub side: Side,
    pub holder: Holder,
    pub balance: Balance,
}

/// What a draw reads of a household: its key and its persons' roles and values, by name, and the household's wealth
/// and income as its own system drew them, each a multiple of its country's median.
#[derive(Clone, Copy, Debug)]
pub struct Drawing<'a> {
    pub household: &'a phx_core::Household,
    pub wealth: f64,
    pub income: f64,
}

/// A system's draw of the households' lines in one country.
pub trait CountryAttachments {
    /// A household's rows, and the key attributes its draw sets, from the system's own streams under the
    /// household's subject.
    fn draw(
        &mut self,
        books: &mut Books,
        household: Drawing<'_>,
        at: (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        keys: &mut Vec<(&'static str, u32)>,
    );

    /// Each pool's total, as the households' rows on it sum it, their side's sign — a deposit's positive, a loan's
    /// negative — apportioned over them by their weights once every household is drawn.
    fn pools(&self) -> Vec<(u32, i64)>;

    /// The parties a derived line's other side is apportioned over, with their drawn sizes.
    fn counterparties(&self, books: &Books, line: &LineSpec) -> Vec<(PartyId, u64)>;
}

/// A system's draw of the households' lines, made ready for each country once its institutions are drawn.
pub trait AttachmentDraw: Send + Sync {
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        when: (&phx_core::Calendar, Day),
        country: &OpeningCountry,
    ) -> Box<dyn CountryAttachments>;
}

/// A choice among counterparties in proportion to their drawn sizes, made online: the n-th choice takes the one
/// furthest below its share of n, ties by lot, so every prefix of the choices is apportioned within one of exact.
#[clause("GEN.4", "REP.23")]
#[derive(Clone, Debug)]
pub struct Online {
    weights: Vec<u64>,
    taken: Vec<u64>,
    made: u64,
}

impl Online {
    #[must_use]
    pub fn new(weights: Vec<u64>) -> Online {
        if weights.iter().all(|w| *w == 0) {
            violation!(clause = "GEN.4", "a choice among counterparties of no size");
        }
        let n = weights.len();
        Online { weights, taken: vec![0; n], made: 0 }
    }

    /// The next choice, by its place among the counterparties.
    pub fn next(&mut self, d: &mut Draws) -> usize {
        let total: u128 = self.weights.iter().map(|w| u128::from(*w)).sum();
        let n = u128::from(self.made + 1);
        // Each one's deficit, n·w − taken·W, compared exactly in whole numbers.
        let deficits: Vec<i128> = self
            .weights
            .iter()
            .zip(&self.taken)
            .map(|(w, t)| (n * u128::from(*w)).cast_signed() - (u128::from(*t) * total).cast_signed())
            .collect();
        let Some(first) = deficits.first().copied() else {
            violation!(clause = "GEN.4", "a choice among no counterparties");
        };
        let best = deficits.iter().fold(first, |b, x| if *x > b { *x } else { b });
        let tied: Vec<usize> = deficits.iter().enumerate().filter(|(_, x)| **x == best).map(|(i, _)| i).collect();
        let at = if let [one] = tied.as_slice() {
            *one
        } else {
            let k = phx_rand::below_u64(d, phx_rand::float::len_u64(tied.len()));
            let Some(i) = usize::try_from(k).ok().and_then(|k| tied.get(k)) else {
                violation!(clause = "CHN.2", "a lot beyond its ties");
            };
            *i
        };
        if let Some(t) = self.taken.get_mut(at) {
            *t += 1;
        }
        self.made += 1;
        at
    }
}

/// A total apportioned over weights: each its floor share, the remainder to the largest remainders, ties by lot.
/// Exact for any total of either sign.
#[clause("GEN.4")]
#[must_use]
pub fn shares(total: i64, weights: &[u64], d: &mut Draws) -> Vec<i64> {
    let sum: u128 = weights.iter().map(|w| u128::from(*w)).sum();
    if sum == 0 {
        if total != 0 {
            violation!(clause = "GEN.4", "a total apportioned over no weight", total = total);
        }
        return vec![0; weights.len()];
    }
    let magnitude = u128::from(total.unsigned_abs());
    let mut out: Vec<u128> = weights.iter().map(|w| magnitude * u128::from(*w) / sum).collect();
    let given: u128 = out.iter().sum();
    let rest = magnitude - given;
    let remainders: Vec<u128> = weights.iter().map(|w| magnitude * u128::from(*w) % sum).collect();
    let mut order: Vec<(u128, u64, usize)> =
        remainders.iter().enumerate().map(|(i, r)| (*r, d.next_u64(), i)).collect();
    order.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let Ok(rest) = usize::try_from(rest) else { phx_num::capacity_exceeded!("a total's remainder", usize::MAX, 0) };
    for (_, _, i) in order.into_iter().take(rest) {
        if let Some(o) = out.get_mut(i) {
            *o += 1;
        }
    }
    out.into_iter()
        .map(|x| {
            let Ok(x) = i64::try_from(x) else {
                phx_num::capacity_exceeded!("a share of a total", i64::MAX, 0);
            };
            if total < 0 { -x } else { x }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::{Online, shares};

    fn draws(seed: u64) -> Draws {
        Draws::new(stream_key(Seed::new(seed), "GEN.test"), Subject::new(SubjectTag::World, 0), 0, 0)
    }

    #[test]
    fn every_prefix_is_within_one_of_its_share() {
        let weights: Vec<u64> = vec![5, 3, 2];
        let mut o = Online::new(weights.clone());
        let mut d = draws(1);
        let mut taken = [0_i64; 3];
        for n in 1..=200_i64 {
            taken[o.next(&mut d)] += 1;
            for (t, w) in taken.iter().zip(&weights) {
                let exact = n * i64::try_from(*w).unwrap();
                assert!((t * 10 - exact).abs() <= 10, "{taken:?} after {n}");
            }
        }
        assert_eq!(taken, [100, 60, 40]);
    }

    #[test]
    fn shares_sum_to_the_total() {
        let w = [7, 0, 13, 1];
        for total in [0_i64, 1, 20, 1_000_003, -999] {
            let s = shares(total, &w, &mut draws(2));
            assert_eq!(s.iter().sum::<i64>(), total);
            assert_eq!(s[1], 0, "no weight, no share");
        }
        assert_eq!(shares(21, &[1, 2], &mut draws(3)), [7, 14]);
    }
}
