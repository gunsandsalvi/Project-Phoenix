use phx_core::Weight;
use phx_id::{LineId, Slot};
use phx_ledger::algebra::Side;
use phx_ledger::apply::Ledger;
use phx_ledger::part::{RowShare, cell_holdings};
use phx_ledger::rows;
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Missing, violation};
use phx_rand::{Draws, hypergeometric, multivariate_hypergeometric};
use phx_store::Backing;

use crate::key::KeyInterner;
use crate::part::{Part, PartId};
use crate::table::CellTable;

/// What an event takes from a cell: how many of its members leave; the joint values they hold in the groups the event
/// determined, every other group's drawn; the rows whose leaving members it determined, every other row's drawn; the
/// amounts of positions that are the leavers' alone; a review kind whose leavers reviewed and acted, who take none of
/// its exposure; and the rounding of the shares that leave.
#[derive(Clone, Copy, Debug)]
pub struct SplitSpec<'a> {
    pub count: u32,
    pub given: &'a [(usize, &'a [(u32, u64)])],
    pub rows: &'a [((LineId, Side), RowShare)],
    pub own: &'a [(usize, i64)],
    pub reviewed: Missing<usize>,
    pub rounding: Round,
}

/// What a split or a landing works on: the ledger, which alone moves rows and holdings, one cell table with its place
/// among the holder tables, and the kind's keys.
#[derive(Debug)]
pub struct Cells<'a, B: Backing, L: Backing> {
    pub ledger: &'a mut Ledger<L>,
    pub table: &'a mut CellTable<B>,
    pub place: u16,
    pub keys: &'a KeyInterner,
}

/// What a split makes: a part, or nothing when every member leaves, since the cell itself re-keys in place and keeps
/// its identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parted {
    Whole,
    Part(Box<Part>),
}

/// A total's share for members leaving: their own amount whole, and `count ÷ weight` of the rest by the rounding
/// given, the remainder staying.
fn share(total: i64, own: i64, count: u32, weight: u32, r: Round) -> (i64, i64) {
    let Some(shared) = total.checked_sub(own) else {
        violation!(clause = "Law 7", "a shared total overflows", total = total, own = own);
    };
    let (leaving, staying) = split_total(shared, u64::from(count), u64::from(weight), r);
    let Some(leaving) = leaving.checked_add(own) else {
        violation!(clause = "Law 7", "a leaving share overflows", leaving = leaving, own = own);
    };
    (leaving, staying)
}

/// How many of `holding` members of a cell of `weight` are among `k` leaving, drawn without replacement.
fn leavers(d: &mut Draws, weight: u32, holding: u32, k: u32) -> u32 {
    let x = hypergeometric(d, u64::from(weight), u64::from(holding), u64::from(k));
    let Ok(x) = u32::try_from(x) else {
        violation!(clause = "REP.14", "more leavers drawn than a row holds", drawn = x);
    };
    x
}

/// Members split from a cell into a part: profiles drawn jointly within each group by multivariate hypergeometric
/// draws where the event did not determine them, every row's and holding's leaving members drawn from the members
/// holding it, and every total divided by the rounding given, the stayers keeping the remainder. The draws are the
/// event's own stream, groups in the kind's order, then rows and holdings in the cell's order. No total moves.
#[clause("REP.8", "REP.9", "REP.14", "REP.23")]
pub fn split<B: Backing, L: Backing>(
    at: &mut Cells<'_, B, L>,
    slot: Slot,
    id: PartId,
    spec: &SplitSpec<'_>,
    d: &mut Draws,
) -> Parted {
    let (ledger, table, place, keys) = (&mut *at.ledger, &mut *at.table, at.place, at.keys);
    let weight = table.weight(slot).get();
    let k = spec.count;
    if k == 0 || k > weight {
        violation!(clause = "REP.14", "a split of no members or more than the cell holds", count = k, weight = weight);
    }
    if k == weight {
        return Parted::Whole;
    }
    let layout = table.profile_layout().clone();
    let mut origin = table.profile(slot);
    let mut profile = crate::profile::Profile::empty(&layout);
    for g in 0..layout.groups.len() {
        let taken: Vec<(u32, u64)> = if let Some((_, values)) = spec.given.iter().find(|(given, _)| *given == g) {
            values.to_vec()
        } else {
            let held = origin.held(g);
            let counts: Vec<u64> = held.iter().map(|(_, n)| u64::from(*n)).collect();
            let mut out = vec![0_u64; counts.len()];
            multivariate_hypergeometric(d, &counts, u64::from(k), &mut out);
            held.iter().zip(out).filter(|(_, n)| *n > 0).map(|((v, _), n)| (*v, n)).collect()
        };
        if taken.iter().map(|(_, n)| n).sum::<u64>() != u64::from(k) {
            violation!(clause = "REP.14", "a group's leaving members other than the part's weight", group = g);
        }
        for (v, n) in taken {
            let Ok(n) = u32::try_from(n) else {
                violation!(clause = "REP.14", "more members leaving a value than a cell holds", value = v);
            };
            origin.remove(g, v, n);
            profile.add(&layout, g, v, n);
        }
    }
    table.set_profile(slot, &origin);
    let held: Vec<(LineId, Side, u32)> =
        rows::iter(&*table, slot).map(|r| (r.row.line, r.side(), r.row.count)).collect();
    for ((line, side), _) in spec.rows {
        if !held.iter().any(|(l, s, _)| (l, s) == (line, side)) {
            violation!(clause = "REP.23", "a split naming a row its cell does not hold", line = line.get());
        }
    }
    let mut detached = Vec::new();
    for (line, side, count) in held {
        if count > weight {
            violation!(clause = "REP.31", "a row of more members than its cell", line = line.get());
        }
        let share = match spec.rows.iter().find(|(at, _)| *at == (line, side)) {
            Some((_, s)) => *s,
            None => RowShare { count: leavers(d, weight, count, k), own_balance: 0 },
        };
        if share.count > 0 {
            detached.push(ledger.detach_row(table, place, slot, (line, side), share, spec.rounding));
        }
    }
    let mut holdings = Vec::new();
    for h in cell_holdings(&*table, slot) {
        let x = leavers(d, weight, h.count, k);
        if x > 0 {
            holdings.push(ledger.detach_holding(table, place, slot, h.instrument, x, spec.rounding));
        }
    }
    let n = table.positions();
    let mut positions = Vec::with_capacity(n);
    for i in 0..n {
        let own = spec.own.iter().filter(|(p, _)| *p == i).map(|(_, a)| *a).sum();
        let (leaving, staying) = share(table.position(slot, i), own, k, weight, spec.rounding);
        table.set_position(slot, i, staying);
        positions.push(leaving);
    }
    let mut exposures = Vec::new();
    for j in 0..table.review_kinds() {
        exposures.push(match table.exposure(slot, j) {
            Missing::Present(_) if spec.reviewed == Missing::Present(j) => Missing::Present(0),
            Missing::Present(e) => {
                let (leaving, staying) = split_total(e, u64::from(k), u64::from(weight), spec.rounding);
                table.set_exposure(slot, j, staying);
                Missing::Present(leaving)
            }
            Missing::Absent => Missing::Absent,
        });
    }
    let rates = (0..table.rate_kinds()).map(|r| table.rate(slot, r)).collect();
    let attention = (0..table.review_kinds()).map(|j| table.attention(slot, j)).collect();
    table.set_weight(slot, Weight::new(weight - k));
    Parted::Part(Box::new(Part {
        id,
        from: slot,
        weight: Weight::new(k),
        key: keys.record(table.hot(slot).key_id),
        sig: table.sig(slot),
        positions,
        rates,
        exposures,
        attention,
        profile,
        rows: detached,
        holdings,
    }))
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyId, Slot};
    use phx_ledger::algebra::Side;
    use phx_ledger::part::{RowShare, cell_holdings};
    use phx_ledger::rows::rows;
    use phx_num::Missing;
    use phx_num::round::Round;
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Parted, SplitSpec, split};
    use crate::fixture::{AGE, AGE_HEALTH, Books, PARTNER_AGE, books, cell, cells, draws, keys, kind};
    use crate::part::{Part, PartId};
    use crate::table::CellTable;

    fn id() -> PartId {
        PartId { origin: PartyId::new(7), seq: 0 }
    }

    fn plain(count: u32) -> SplitSpec<'static> {
        SplitSpec { count, given: &[], rows: &[], own: &[], reviewed: Missing::Absent, rounding: Round::HalfEven }
    }

    fn part(parted: Parted) -> Part {
        match parted {
            Parted::Part(got) => *got,
            Parted::Whole => panic!("a part was expected"),
        }
    }

    type RowTotals = Vec<(u32, Missing<i64>)>;
    type HoldingTotals = Vec<(u32, i64, i64)>;

    /// Every row's count and balance, and every holding's, of a cell.
    fn totals(tab: &CellTable<HeapBacking>, slot: Slot) -> (RowTotals, HoldingTotals) {
        let row_totals = rows(tab, slot).iter().map(|v| (v.row.count, v.optional.balance)).collect();
        let holding_totals = cell_holdings(tab, slot)
            .iter()
            .map(|holding_totals| {
                (holding_totals.count, holding_totals.quantity.raw(), holding_totals.pooled_cost.raw())
            })
            .collect();
        (row_totals, holding_totals)
    }

    #[test]
    fn split_conserves_totals_and_profiles() {
        let kind = kind();
        for i in 0..200 {
            let mut space = AddressSpace::empty();
            let mut bk: Books = books(&mut space);
            let (keys, _) = keys(&kind, 1, 1);
            let (mut tab, slot) = cell(&mut space, &kind, &mut bk, &keys);
            let before = tab.profile(slot);
            let got =
                part(split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &plain(25), &mut draws("DEM.death", i)));
            assert_eq!((got.weight.get(), tab.weight(slot).get()), (25, 75));
            let after = tab.profile(slot);
            for g in [AGE, AGE_HEALTH, PARTNER_AGE] {
                assert_eq!(got.profile.members(g), 25, "every group of every role counts the part's members");
                for (v, n) in before.held(g) {
                    assert_eq!(after.count(g, *v) + got.profile.count(g, *v), *n, "group {g} value {v}");
                }
            }
            assert_eq!(got.positions[0] + tab.position(slot, 0), 10_000);
            let (row_totals, holding_totals) = totals(&tab, slot);
            let joined: Vec<(u32, i64)> = [(100, 50_001), (30, 9_000)].to_vec();
            for ((line, side), (count, total)) in
                [(bk.deposit, Side::Asset), (bk.loan, Side::Liability)].iter().zip(joined)
            {
                let stay = rows(&tab, slot).iter().find(|v| (v.row.line, v.side()) == (*line, *side)).map(|v| {
                    let Missing::Present(bal) = v.optional.balance else { panic!("a balance") };
                    (v.row.count, bal)
                });
                let left = got.rows.iter().find(|d| (d.line(), d.side()) == (*line, *side)).map(|d| {
                    let Missing::Present(bal) = d.optional.balance else { panic!("a balance") };
                    (d.row.count, bal)
                });
                let (sc, sb) = stay.unwrap_or((0, 0));
                let (lc, lb) = left.unwrap_or((0, 0));
                assert_eq!((sc + lc, sb + lb), (count, total), "a row's members and balance are conserved");
                assert!(lc <= 25);
            }
            let (hc, hq, hcost) = holding_totals.first().copied().unwrap_or((0, 0, 0));
            let (pc, pq, pcost) =
                got.holdings.first().map_or((0, 0, 0), |x| (x.count, x.quantity.raw(), x.pooled_cost.raw()));
            assert_eq!((hc + pc, hq + pq, hcost + pcost), (10, 1_000, 2_003));
            assert_eq!(row_totals.first().map(|x| x.0), Some(75), "the deposit is every member's");
            let (Missing::Present(left), Missing::Present(stay)) = (got.exposures[0], tab.exposure(slot, 0)) else {
                panic!("the exposure is kept on both sides")
            };
            assert_eq!(left + stay, 7_000);
        }
    }

    #[test]
    fn split_joint_within_group() {
        let kind = kind();
        let (trials, k) = (400_u32, 50_u32);
        let mut sick_young = 0_u64;
        for i in 0..trials {
            let mut space = AddressSpace::empty();
            let mut bk = books(&mut space);
            let (keys, _) = keys(&kind, 1, 1);
            let (mut tab, slot) = cell(&mut space, &kind, &mut bk, &keys);
            let given: &[(usize, &[(u32, u64)])] = &[(AGE, &[(0, 35), (2, 15)])];
            let spec = SplitSpec { given, ..plain(k) };
            let got =
                part(split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &spec, &mut draws("DEM.illness", i)));
            assert_eq!(got.profile.held(AGE), [(0, 35), (2, 15)], "the event's own values leave as drawn");
            for (v, n) in got.profile.held(AGE_HEALTH) {
                assert!(
                    u64::from(tab.profile(slot).count(AGE_HEALTH, *v)) + u64::from(*n) <= 40,
                    "a joint value, never a marginal"
                );
            }
            sick_young += u64::from(got.profile.count(AGE_HEALTH, 1));
        }
        // The joint value (young, ill) is held by 10 of 100: fifty drawn without replacement take 5 on average, with a
        // variance of 50 × 0.1 × 0.9 × 50 ÷ 99, so the mean of 400 trials lies within 0.3 of it but once in 10⁴.
        let mean = phx_rand::float::from_u64(sick_young) / phx_rand::float::from_u64(u64::from(trials));
        assert!((mean - 5.0).abs() < 0.3, "{mean}");
    }

    #[test]
    fn split_by_other_cause_takes_exposure_share() {
        let kind = kind();
        let mut space = AddressSpace::empty();
        let mut bk = books(&mut space);
        let (keys, _) = keys(&kind, 1, 1);
        let (mut tab, slot) = cell(&mut space, &kind, &mut bk, &keys);
        let got = part(split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &plain(25), &mut draws("DEM.death", 0)));
        assert_eq!((got.exposures[0], tab.exposure(slot, 0)), (Missing::Present(1_750), Missing::Present(5_250)));
        let acted = SplitSpec { reviewed: Missing::Present(0), ..plain(15) };
        let again = part(split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &acted, &mut draws("HH.move", 0)));
        assert_eq!(
            (again.exposures[0], tab.exposure(slot, 0)),
            (Missing::Present(0), Missing::Present(5_250)),
            "reviewers who acted take none"
        );
    }

    #[test]
    fn own_amounts_leave_whole_and_given_rows_leave_as_given() {
        let kind = kind();
        let mut space = AddressSpace::empty();
        let mut bk = books(&mut space);
        let (keys, _) = keys(&kind, 1, 1);
        let (mut tab, slot) = cell(&mut space, &kind, &mut bk, &keys);
        let rows_given = [
            ((bk.loan, Side::Liability), RowShare { count: 10, own_balance: 0 }),
            ((bk.deposit, Side::Asset), RowShare { count: 10, own_balance: -600 }),
        ];
        let spec = SplitSpec { rows: &rows_given, own: &[(0, 400)], ..plain(10) };
        let got = part(split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &spec, &mut draws("SET.pooled", 0)));
        let loan = got.rows.iter().find(|d| d.line() == bk.loan).map(|d| d.row.count);
        assert_eq!(loan, Some(10), "the members a flow reached are the ones who leave its row");
        let deposit = got.rows.iter().find(|d| d.line() == bk.deposit).map(|d| d.optional.balance);
        assert_eq!(deposit, Some(Missing::Present(-600 + 5_060)), "their own −600, then a tenth of 50 601");
        assert_eq!(got.positions[0], 400 + 960, "their own 400, then a tenth of 9 600");
        assert!(
            matches!(
                split(&mut cells(&mut bk, &mut tab, &keys), slot, id(), &plain(90), &mut draws("DEM.death", 1)),
                Parted::Whole
            ),
            "every member leaving is the cell itself"
        );
    }
}
