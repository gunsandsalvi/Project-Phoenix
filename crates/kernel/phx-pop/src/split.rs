use phx_core::Weight;
use phx_id::{LineId, Slot};
use phx_ledger::algebra::Side;
use phx_ledger::apply::Ledger;
use phx_ledger::part::{RowPlan, RowShare, cell_holdings};
use phx_ledger::rows;
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Missing, violation};
use phx_rand::{Draws, below_u64, hypergeometric, multivariate_hypergeometric, pick_without_replacement};
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
    if k == 1 {
        // One member leaving holds the row with chance holding ÷ weight, which one uniform integer decides exactly.
        return u32::from(below_u64(d, u64::from(weight)) < u64::from(holding));
    }
    let x = hypergeometric(d, u64::from(weight), u64::from(holding), u64::from(k));
    let Ok(x) = u32::try_from(x) else {
        violation!(clause = "REP.14", "more leavers drawn than a row holds", drawn = x);
    };
    x
}

/// The leaving members' profile values, taken from the cell's as the splits before left it: the groups the event
/// determined as it gives them, every other group drawn without replacement from the cell's counts, each group's
/// persons the leaving members' count of its role under the cell's key. What leaves is taken from `origin` and noted
/// in `deltas`, so the cell's profile is written once for all its splits.
fn split_profile(
    layout: &crate::profile::ProfileLayout,
    key: &crate::key::KeyRecord,
    origin: &mut crate::profile::Profile,
    deltas: &mut Vec<(usize, u32, i64)>,
    spec: &SplitSpec<'_>,
    d: &mut Draws,
) -> crate::profile::Profile {
    let mut profile = crate::profile::Profile::empty(layout);
    for g in 0..layout.groups.len() {
        let persons = layout.persons(g, key, u64::from(spec.count));
        let mut taken: Vec<(u32, u64)> = if let Some((_, values)) = spec.given.iter().find(|(given, _)| *given == g) {
            values.to_vec()
        } else {
            let held = origin.held(g);
            let counts: Vec<u64> = held.iter().map(|(_, n)| u64::from(*n)).collect();
            let mut out = vec![0_u64; counts.len()];
            // All three draws are the same multivariate hypergeometric: one person is one uniform over the persons,
            // found by a scan; a few persons one by one; many persons value by value.
            if persons == 0 {
            } else if persons == 1 {
                if let Some(slot) = out.get_mut(crate::pick::one_of(d, &counts)) {
                    *slot = 1;
                }
            } else if persons < phx_rand::float::len_u64(counts.len()) {
                pick_without_replacement(d, &counts, persons, &mut out);
            } else {
                multivariate_hypergeometric(d, &counts, persons, &mut out);
            }
            held.iter().zip(out).filter(|(_, n)| *n > 0).map(|((v, _), n)| (*v, n)).collect()
        };
        if taken.iter().map(|(_, n)| n).sum::<u64>() != persons {
            violation!(clause = "REP.14", "a group's leaving persons other than the part's members hold", group = g);
        }
        taken.sort_unstable_by_key(|(v, _)| *v);
        for (v, n) in taken {
            let (Ok(n), Ok(by)) = (u32::try_from(n), i64::try_from(n)) else {
                violation!(clause = "REP.14", "more members leaving a value than a cell holds", value = v);
            };
            origin.remove(g, v, n);
            profile.add(layout, g, v, n);
            deltas.push((g, v, -by));
        }
    }
    profile
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
    let mut one = [(id, *spec, d)];
    split_batch(at, slot, &mut one).pop().unwrap_or(Parted::Whole)
}

/// Several splits from one cell at one apply sub-step, in the order given, leaving the cell and the parts as splitting
/// them one by one would: each draws from the cell as the splits before it left it, and the cell's profile is read
/// once and written once.
#[clause("REP.8", "REP.9", "REP.14", "REP.23")]
pub fn split_batch<B: Backing, L: Backing>(
    at: &mut Cells<'_, B, L>,
    slot: Slot,
    splits: &mut [(PartId, SplitSpec<'_>, &mut Draws)],
) -> Vec<Parted> {
    let (ledger, table, place, keys) = (&mut *at.ledger, &mut *at.table, at.place, at.keys);
    let layout = table.profile_layout().clone();
    let mut origin = table.profile(slot);
    let mut deltas = Vec::new();
    // The cell's rows as the splits so far left them: the members each holds, a row gone once none do.
    let mut held: Vec<(LineId, Side, u32)> =
        rows::iter(&*table, slot).map(|r| (r.row.line, r.side(), r.row.count)).collect();
    let mut plans: Vec<(Vec<RowLeaving>, Round)> = Vec::with_capacity(splits.len());
    let mut out = Vec::with_capacity(splits.len());
    for (id, spec, d) in splits.iter_mut() {
        let d: &mut Draws = d;
        let weight = table.weight(slot).get();
        let k = spec.count;
        if k == 0 || k > weight {
            violation!(
                clause = "REP.14",
                "a split of no members or more than the cell holds",
                count = k,
                weight = weight
            );
        }
        if k == weight {
            out.push(Parted::Whole);
            continue;
        }
        let key = keys.record(table.hot(slot).key_id);
        let profile = split_profile(&layout, &key, &mut origin, &mut deltas, spec, d);
        plans.push((plan_rows(&mut held, spec, weight, d), spec.rounding));
        let mut holdings = Vec::new();
        for h in cell_holdings(&*table, slot) {
            let x = leavers(d, weight, h.count, k);
            if x > 0 {
                holdings.push(ledger.detach_holding(table, place, slot, h.instrument, x, spec.rounding));
            }
        }
        let (positions, exposures) = split_totals(table, slot, spec, weight);
        let rates = (0..table.rate_kinds()).map(|r| table.rate(slot, r)).collect();
        let attention = (0..table.review_kinds()).map(|j| table.attention(slot, j)).collect();
        table.set_weight(slot, Weight::new(weight - k));
        out.push(Parted::Part(Box::new(Part {
            id: *id,
            from: slot,
            weight: Weight::new(k),
            key,
            sig: table.sig(slot),
            positions,
            rates,
            exposures,
            attention,
            profile,
            rows: Vec::new(),
            holdings,
        })));
    }
    let borrowed: Vec<RowPlan<'_>> = plans.iter().map(|(p, r)| (p.as_slice(), *r)).collect();
    let mut detached = ledger.detach_rows_batch(table, place, slot, &borrowed).into_iter();
    for parted in &mut out {
        if let Parted::Part(p) = parted {
            p.rows = detached.next().unwrap_or_default();
        }
    }
    table.shift_profile(slot, &crate::profile::net(deltas));
    out
}

/// The members leaving one line side of a cell's row.
type RowLeaving = ((LineId, Side), RowShare);

/// The members one split takes from each of the cell's rows as the splits before it left them: the event's own rows as
/// it gives them, every other row's drawn from the members holding it; `held` is left with what stays.
fn plan_rows(held: &mut Vec<(LineId, Side, u32)>, spec: &SplitSpec<'_>, weight: u32, d: &mut Draws) -> Vec<RowLeaving> {
    for ((line, side), _) in spec.rows {
        if !held.iter().any(|(l, s, _)| (l, s) == (line, side)) {
            violation!(clause = "REP.23", "a split naming a row its cell does not hold", line = line.get());
        }
    }
    let mut plan = Vec::with_capacity(held.len());
    for (line, side, count) in held.iter_mut() {
        let share = if let Some((_, s)) = spec.rows.iter().find(|(at, _)| *at == (*line, *side)) {
            *s
        } else {
            // A row drawn is held by households, one member each; persons' rows are always given.
            if *count > weight {
                violation!(clause = "REP.31", "a row drawn of more members than its cell", line = line.get());
            }
            RowShare { count: leavers(d, weight, *count, spec.count), own_balance: 0 }
        };
        if share.count > 0 {
            plan.push(((*line, *side), share));
            let Some(rest) = count.checked_sub(share.count) else {
                violation!(clause = "REP.9", "more members leaving a row than hold it", line = line.get());
            };
            *count = rest;
        }
    }
    held.retain(|(_, _, count)| *count > 0);
    plan
}

/// A split's shares of the cell's positions and review exposures, the stayers keeping the rest: their own amounts
/// leave whole, and reviewers who acted leave with no exposure.
fn split_totals<B: Backing>(
    table: &mut CellTable<B>,
    slot: Slot,
    spec: &SplitSpec<'_>,
    weight: u32,
) -> (Vec<i64>, Vec<Missing<i64>>) {
    let k = spec.count;
    let n = table.positions();
    let mut positions = Vec::with_capacity(n);
    for i in 0..n {
        let own = spec.own.iter().filter(|(p, _)| *p == i).map(|(_, a)| *a).sum();
        let (leaving, staying) = share(table.position(slot, i), own, k, weight, spec.rounding);
        table.set_position(slot, i, staying);
        positions.push(leaving);
    }
    let mut exposures = Vec::with_capacity(table.review_kinds());
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
    (positions, exposures)
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
    fn one_member_leaves_by_its_share() {
        let kind = kind();
        let mut space = AddressSpace::empty();
        let mut bk = books(&mut space);
        let (keys, _) = keys(&kind, 1, 1);
        let (tab, slot) = cell(&mut space, &kind, &mut bk, &keys);
        let (layout, origin) = (tab.profile_layout().clone(), tab.profile(slot));
        let trials = 20_000_u32;
        let (mut sick_young, mut loan) = (0_u64, 0_u64);
        let mut d = draws("DEM.death", 0);
        for _ in 0..trials {
            let (mut held, mut deltas) = (origin.clone(), Vec::new());
            let got = super::split_profile(
                &layout,
                &crate::key::KeyRecord::default(),
                &mut held,
                &mut deltas,
                &plain(1),
                &mut d,
            );
            assert_eq!(got.members(AGE_HEALTH), 1);
            sick_young += u64::from(got.count(AGE_HEALTH, 1));
            loan += u64::from(super::leavers(&mut d, 100, 30, 1));
        }
        // The member is (young, ill) with chance 10 ÷ 100 and holds the loan with chance 30 ÷ 100. Over 20 000 trials
        // the counts' standard deviations are √(20 000 × 0.1 × 0.9) ≈ 42.4 and √(20 000 × 0.3 × 0.7) ≈ 64.8, so each
        // lies within 6.1 of them of its mean but for a chance below 10⁻⁹.
        let within = |x: u64, mean: f64, sd: f64| (phx_rand::float::from_u64(x) - mean).abs() < 6.1 * sd;
        assert!(within(sick_young, 2_000.0, 42.5), "{sick_young}");
        assert!(within(loan, 6_000.0, 64.9), "{loan}");
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
