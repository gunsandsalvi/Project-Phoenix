use phx_macros::clause;

/// A point a row's reached members may cross: a value of their own position per member, and whether crossing it
/// fails the row (funds at zero, a limit) or splits the reached members off with their own outcome (a band, a means
/// test).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kink {
    pub at: i64,
    pub fails: bool,
}

/// A row as the pooled-flow rule tests it: what it takes from the funds per member (paid out positive, received
/// negative), how many of the party's members it reaches, and those members' own position per member before it, which
/// kinks lie on, with how the row moves it (income to date rises with a wage received).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PooledRow {
    pub per_member: i64,
    pub reached: u32,
    pub position: i64,
    pub moves: i64,
}

/// What the rule makes of a row: paid from the pool, failed with every row after it, or paid with its reached
/// members split off because they crossed a kink.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowOutcome {
    Pooled,
    Fails,
    Splits { reached: u32 },
}

/// Whether a move from a position crosses a kink: the kink lies after the position and at or before where the move
/// takes it, in the move's direction.
fn crosses(position: i64, by: i64, at: i64) -> bool {
    let to = i128::from(position) + i128::from(by);
    let (from, at) = (i128::from(position), i128::from(at));
    if by >= 0 { from < at && at <= to } else { to <= at && at < from }
}

/// The pooled-flow rule: a party's rows tested one at a time in its payment order, each against the funds its
/// members hold per member after the rows before it, and its reached members' own position against every kink. A
/// row whose reached members' share of the funds cannot pay it fails, and every row after it with it, so failure is a
/// prefix of the order; a row that carries its reached members across another kink splits them off and is paid.
/// Pure: the caller hands the funds, the members and the kinks.
#[clause("REP.8", "REP.16")]
#[must_use]
pub fn pooled(funds: i128, weight: u32, rows: &[PooledRow], kinks: &[Kink]) -> Vec<RowOutcome> {
    if weight == 0 {
        phx_num::violation!(clause = "REP.8", "a pooled flow over a party with no members");
    }
    let mut left = funds;
    let mut failed = false;
    let mut out = Vec::with_capacity(rows.len());
    let members = i128::from(weight);
    for row in rows {
        if failed {
            out.push(RowOutcome::Fails);
            continue;
        }
        let total = i128::from(row.per_member) * i128::from(row.reached);
        let share = left.div_euclid(members);
        let short = row.per_member > 0 && share < i128::from(row.per_member);
        let limit = kinks.iter().any(|k| k.fails && crosses(row.position, row.moves, k.at));
        if short || limit {
            failed = true;
            out.push(RowOutcome::Fails);
            continue;
        }
        left -= total;
        let band = kinks.iter().any(|k| !k.fails && crosses(row.position, row.moves, k.at));
        let split = band || (row.reached < weight && row.per_member != 0);
        out.push(if split { RowOutcome::Splits { reached: row.reached } } else { RowOutcome::Pooled });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{Kink, PooledRow, RowOutcome, pooled};

    /// A payment out that moves no position a kink lies on.
    fn out(per_member: i64, reached: u32) -> PooledRow {
        PooledRow { per_member, reached, position: 0, moves: 0 }
    }

    #[test]
    fn failure_is_a_prefix_of_the_order() {
        let rows = [out(60, 1), out(50, 1), out(10, 1)];
        assert_eq!(pooled(100, 1, &rows, &[]), [RowOutcome::Pooled, RowOutcome::Fails, RowOutcome::Fails]);
        assert_eq!(pooled(110, 1, &rows, &[]), [RowOutcome::Pooled, RowOutcome::Pooled, RowOutcome::Fails]);
        let first_short = [out(110, 1), out(10, 1)];
        assert_eq!(
            pooled(100, 1, &first_short, &[]),
            [RowOutcome::Fails, RowOutcome::Fails],
            "a later row never jumps"
        );
    }

    #[test]
    fn pooled_inflow_band_split() {
        let band = Kink { at: 1_000, fails: false };
        let below = PooledRow { per_member: -100, reached: 4, position: 800, moves: 100 };
        assert_eq!(pooled(0, 4, &[below], &[band]), [RowOutcome::Pooled], "received, and still below the band");
        let across = PooledRow { per_member: -300, reached: 4, position: 800, moves: 300 };
        assert_eq!(pooled(0, 4, &[across], &[band]), [RowOutcome::Splits { reached: 4 }], "received across the band");
        let exactly = PooledRow { per_member: -200, reached: 4, position: 800, moves: 200 };
        assert_eq!(
            pooled(0, 4, &[exactly], &[band]),
            [RowOutcome::Splits { reached: 4 }],
            "reaching the band is crossing it"
        );
    }

    #[test]
    fn a_row_reaching_some_members_splits_them() {
        assert_eq!(pooled(100, 5, &[out(10, 2)], &[]), [RowOutcome::Splits { reached: 2 }]);
        assert_eq!(pooled(100, 5, &[out(30, 2)], &[]), [RowOutcome::Fails], "20 a member cannot pay 30");
    }

    #[test]
    fn a_limit_fails_the_row() {
        let limit = Kink { at: 500, fails: true };
        let over = PooledRow { per_member: 200, reached: 1, position: 400, moves: 200 };
        let within = PooledRow { per_member: 50, reached: 1, position: 400, moves: 50 };
        assert_eq!(pooled(10_000, 1, &[over], &[limit]), [RowOutcome::Fails]);
        assert_eq!(pooled(10_000, 1, &[within], &[limit]), [RowOutcome::Pooled]);
    }
}
