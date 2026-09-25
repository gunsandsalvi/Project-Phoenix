//! Landings re-read after they joined, one in a declared number by the part's identity: what the cell and its parts
//! held of each line side before, against what the cell holds after, and where their members lay against the lines'
//! kinks. A straight rule is linear in a row's members and balance, so its total is unchanged exactly when every line
//! side's members and balance are.

use std::collections::BTreeMap;

use phx_exec::mix64;
use phx_id::LineId;
use phx_ledger::algebra::Side;
use phx_macros::clause;
use phx_num::Missing;

use crate::check::{LineKinks, View, at_or_above};
use crate::consts::LANDING_SAMPLE;
use crate::part::PartId;

/// One landing re-read: the line sides it held, those whose members or balance the join did not keep, and the rows
/// whose members the join moved across a point of their line's kinks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LandingSample {
    pub sides: u32,
    pub unequal: u32,
    pub crossed: u32,
}

/// Whether a part's landing is re-read: one in the declared number, by its identity's mix.
#[must_use]
pub fn sampled(id: PartId) -> bool {
    mix64(id.origin.get() ^ (u64::from(id.seq) << u32::BITS)).is_multiple_of(LANDING_SAMPLE)
}

/// Members and summed balance of one line side, the balance absent where no row of it keeps one.
type SideTotal = (u64, Missing<i128>);

/// A line side held by no row yet.
const NONE: (SideTotal, SideTotal) = ((0, Missing::Absent), (0, Missing::Absent));

fn add(total: &mut SideTotal, count: u32, balance: Missing<i64>) {
    total.0 += u64::from(count);
    total.1 = match (total.1, balance) {
        (Missing::Present(a), Missing::Present(b)) => Missing::Present(a + i128::from(b)),
        (Missing::Absent, Missing::Present(b)) => Missing::Present(i128::from(b)),
        (held, Missing::Absent) => held,
    };
}

/// A landing re-read: `before` is the cell as it stood and each part that joined it, `after` the cell as it stands.
#[clause("REP.8", "REP.16", "REP.36")]
#[must_use]
pub fn reread(before: &[&View], after: &View, kinks: &dyn LineKinks) -> LandingSample {
    let mut held: BTreeMap<(LineId, Side), (SideTotal, SideTotal)> = BTreeMap::new();
    for view in before {
        for r in &view.rows {
            add(&mut held.entry((r.line, r.side)).or_insert(NONE).0, r.count, r.balance);
        }
    }
    for r in &after.rows {
        add(&mut held.entry((r.line, r.side)).or_insert(NONE).1, r.count, r.balance);
    }
    let mut out = LandingSample::default();
    for (was, is) in held.values() {
        out.sides += 1;
        if was != is {
            out.unequal += 1;
        }
    }
    for view in before {
        for r in &view.rows {
            let Some(now) = after.rows.iter().find(|a| (a.line, a.side) == (r.line, r.side)) else { continue };
            let (Missing::Present(a), Missing::Present(b)) = (r.balance, now.balance) else { continue };
            let crossed = kinks
                .points(r.line, r.side)
                .into_iter()
                .any(|p| at_or_above(a, r.count, p) != at_or_above(b, now.count, p));
            if crossed {
                out.crossed += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use phx_id::LineId;
    use phx_ledger::algebra::Side;
    use phx_num::Missing;

    use super::{LandingSample, reread};
    use crate::check::{LineKinks, RowFacts, View};

    struct At(i64);

    impl LineKinks for At {
        fn points(&self, _: LineId, _: Side) -> Vec<i64> {
            vec![self.0]
        }
    }

    fn view(rows: &[(u32, u32, i64)]) -> View {
        View {
            key: Missing::Absent,
            sig: Vec::new(),
            steps: Vec::new(),
            rates: Vec::new(),
            attention: Vec::new(),
            exposed: Vec::new(),
            rows: rows
                .iter()
                .map(|(line, count, balance)| RowFacts {
                    line: LineId::new(*line),
                    side: Side::Asset,
                    count: *count,
                    record: 0,
                    point: 0,
                    balance: Missing::Present(*balance),
                    since: Missing::Absent,
                })
                .collect(),
        }
    }

    #[test]
    fn a_join_that_adds_every_side_keeps_every_straight_total() {
        let (cell, part) = (view(&[(1, 10, 500), (2, 3, 30)]), view(&[(1, 2, 100)]));
        let after = view(&[(1, 12, 600), (2, 3, 30)]);
        assert_eq!(reread(&[&cell, &part], &after, &At(1_000)), LandingSample { sides: 2, unequal: 0, crossed: 0 });
    }

    #[test]
    fn a_lost_member_or_unit_and_a_crossed_kink_are_counted() {
        let (cell, part) = (view(&[(1, 10, 500)]), view(&[(1, 2, 100)]));
        let short = view(&[(1, 12, 599)]);
        assert_eq!(reread(&[&cell, &part], &short, &At(1_000)).unequal, 1);
        // Fifty a member before, sixty after the part at a hundred a member joined: the point at 55 is crossed.
        let (cell, part) = (view(&[(1, 10, 500)]), view(&[(1, 10, 700)]));
        let after = view(&[(1, 20, 1_200)]);
        assert_eq!(reread(&[&cell, &part], &after, &At(55)).crossed, 1);
    }
}
