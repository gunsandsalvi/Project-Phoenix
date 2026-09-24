use phx_macros::clause;
use phx_num::round::{Round, div_round};
use phx_num::{Missing, capacity_exceeded};

use crate::pooled::{Kink, PooledRow, RowOutcome, pooled};

/// A standing flow as its row carries it: a rate per member per day, plain or times a region's daily index, and
/// whether its kind pays in banknotes on a day with no settlement rather than by card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StandingFlow {
    pub per_member: i64,
    pub index: Missing<&'static str>,
    pub banknotes: bool,
}

/// A standing flow's leg for a day: in that day's batch on a business day; otherwise pending on the payer's deposit
/// until the next business day's batch, or paid in banknotes where its kind says so.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayLeg {
    Batch(i64),
    Pending(i64),
    Banknotes(i64),
}

/// A day's amount: the rate per member, times the region's index that day over its scale for an indexed flow, times
/// the members the flow reaches that day, rounded once.
#[clause("REP.8")]
#[must_use]
pub fn day_amount(per_member: i64, index: Missing<(i64, i64)>, reached: u32, rounding: Round) -> i64 {
    let whole = i128::from(per_member) * i128::from(reached);
    let amount = match index {
        Missing::Present((value, scale)) => div_round(whole * i128::from(value), i128::from(scale), rounding),
        Missing::Absent => whole,
    };
    let Ok(a) = i64::try_from(amount) else {
        capacity_exceeded!("a standing flow's day", i64::MAX, 0);
    };
    a
}

/// Where a day's amount goes, by whether the day is a business day for the flow.
#[must_use]
pub fn day_leg(amount: i64, business: bool, flow: StandingFlow) -> DayLeg {
    match (business, flow.banknotes) {
        (true, _) => DayLeg::Batch(amount),
        (false, true) => DayLeg::Banknotes(amount),
        (false, false) => DayLeg::Pending(amount),
    }
}

/// For a plain flow, the first day from now on which its members' position, moving by the rate each day, reaches a
/// kink, so that day is booked on the agenda; none when the rate never takes it there.
#[clause("REP.16")]
pub fn kink_day(position: i64, per_day: i64, kink: i64) -> Missing<u32> {
    let gap = i128::from(kink) - i128::from(position);
    if per_day == 0 || gap == 0 || (gap > 0) != (per_day > 0) {
        return Missing::Absent;
    }
    let days = div_round(gap.abs(), i128::from(per_day).abs(), Round::Ceil);
    let Ok(d) = u32::try_from(days) else {
        capacity_exceeded!("days to a standing flow's kink", u32::MAX, 0);
    };
    Missing::Present(d)
}

/// An indexed flow's day tested as the stream tests every row it reads: its realised amount against the payer's funds
/// and every kink on the reached members' position, on the day it posts, whether its leg settles or waits pending.
#[clause("REP.8", "REP.16")]
#[must_use]
pub fn indexed_day(
    funds: i128,
    weight: u32,
    reached: u32,
    per_member: i64,
    position: i64,
    kinks: &[Kink],
) -> RowOutcome {
    let row = PooledRow { per_member, reached, position, moves: per_member };
    let [out] = pooled(funds, weight, &[row], kinks)[..] else {
        phx_num::violation!(clause = "REP.8", "a pooled test of one row gave other than one outcome");
    };
    out
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;
    use phx_num::round::Round;

    use super::{DayLeg, StandingFlow, day_amount, day_leg, indexed_day, kink_day};
    use crate::pooled::{Kink, RowOutcome};

    #[test]
    fn indexed_flow_amount_and_daily_kink_test() {
        // Energy at 3 a member per degree-day, the index kept in tenths, over 5 members.
        assert_eq!(day_amount(3, Missing::Present((124, 10)), 5, Round::HalfEven), 186, "3 × 12.4 × 5");
        assert_eq!(day_amount(3, Missing::Absent, 5, Round::HalfEven), 15, "a plain flow ignores any index");
        // Over a path of the index, spending accumulates towards a band at 100 a member; the day it is crossed is the
        // day the test splits the members, whether or not that day settles.
        let band = [Kink { at: 100, fails: false }];
        let path = [80, 150, 240, 310, 90];
        let business = [true, true, false, false, true];
        let flow = StandingFlow { per_member: 1, index: Missing::Present("GEO.degree_days"), banknotes: false };
        let mut position = 0_i64;
        let mut crossed = Vec::new();
        for (day, (index, open)) in path.iter().zip(business).enumerate() {
            let per_member = day_amount(1, Missing::Present((*index, 10)), 1, Round::HalfEven);
            if indexed_day(1_000_000, 5, 5, per_member, position, &band) == (RowOutcome::Splits { reached: 5 }) {
                crossed.push(day);
            }
            let leg = day_leg(per_member * 5, open, flow);
            assert_eq!(matches!(leg, DayLeg::Pending(_)), !open, "a closed day's leg waits pending");
            position += per_member;
        }
        assert_eq!(position, 87);
        assert!(crossed.is_empty(), "8 + 15 + 24 + 31 + 9 never reaches 100");
        let mut position = 60_i64;
        let mut first = None;
        for (day, index) in path.iter().enumerate() {
            let per_member = day_amount(1, Missing::Present((*index, 10)), 1, Round::HalfEven);
            if first.is_none() && indexed_day(1_000_000, 5, 5, per_member, position, &band) != RowOutcome::Pooled {
                first = Some(day);
            }
            position += per_member;
        }
        assert_eq!(first, Some(2), "60 + 8 + 15 = 83, then 24 takes it to 107 on the third day, a closed one");
        // A funds limit on the same path fails the row the day it would be crossed.
        let limit = [Kink { at: 100, fails: true }];
        assert_eq!(indexed_day(1_000_000, 5, 5, 24, 83, &limit), RowOutcome::Fails);
    }

    #[test]
    fn plain_kink_day() {
        assert_eq!(kink_day(0, 7, 100), Missing::Present(15), "day 15 takes 0 by 7 a day to 105");
        assert_eq!(kink_day(100, -7, 0), Missing::Present(15));
        assert_eq!(kink_day(0, -7, 100), Missing::Absent, "moving away never reaches it");
    }
}
