use phx_id::Slot;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Draws, below_u64};
use phx_store::Backing;

use crate::index::Index;
use crate::kind::PopKindDecl;
use crate::landing::{Landed, LandingIndex, TenB, landing_key};
use crate::rekey::rekey_flagged;
use crate::steps::Step;
use crate::table::CellTable;

/// A cell as a decision's pure form reads it: its members, and its positions' totals and standing rates over them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellFacts {
    pub weight: u32,
    pub totals: Vec<i64>,
    pub rates: Vec<Missing<i64>>,
}

/// A continuous decision the gap estimate evaluates off-world: its pure form, giving the decision's total over a cell's
/// members in its own unit, and the position whose per-member value is its declared scale, so its gaps compare with
/// other decisions' as shares of the member's own scale.
#[derive(Clone, Copy, Debug)]
pub struct GapForm {
    pub name: &'static str,
    pub form: fn(&CellFacts) -> Missing<i64>,
    pub scale: usize,
}

/// A kind's tolerance settings, RESOLUTION primitives read and never set in the run: the cell budget, the cells the gap
/// estimate samples, the share of the budget narrowing stops at, and the order positions widen in when their gaps tie.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tolerances {
    pub budget: u64,
    pub sample: u32,
    pub narrow_num: u64,
    pub narrow_den: u64,
    pub widen_order: Vec<usize>,
}

/// What the sample showed of widening each position one level: the mean gap it would cause, over the pairs found, as
/// a share of the members' own scale; and how many sampled cells found a partner the widening would unite them with.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Estimate {
    pub gap: Vec<f64>,
    pub pairs: Vec<u32>,
    pub sampled: u32,
}

/// The level at which a position's base partition merges into one step: widening stops there.
#[must_use]
pub fn top_levels(kind: &PopKindDecl) -> Vec<u8> {
    kind.positions
        .iter()
        .map(|p| {
            // A partition has at least one step, and its highest step's bits are the shifts that merge it into one.
            let Some(highest) = p.steps.steps().checked_sub(1) else {
                violation!(clause = "REP.4", "a partition of no steps");
            };
            let Ok(bits) = u8::try_from(usize::BITS - highest.leading_zeros()) else {
                violation!(clause = "REP.4", "a partition of more levels than a level counts");
            };
            bits
        })
        .collect()
}

/// The cell's facts, as the pure forms read them.
#[must_use]
pub fn facts<B: Backing>(table: &CellTable<B>, slot: Slot) -> CellFacts {
    CellFacts {
        weight: table.weight(slot).get(),
        totals: (0..table.positions()).map(|i| table.position(slot, i)).collect(),
        rates: (0..table.rate_kinds()).map(|r| table.rate(slot, r)).collect(),
    }
}

/// Two cells as one: members and totals add; rates are the cells' own, which a pair must share to be joined.
fn joined(a: &CellFacts, b: &CellFacts) -> Option<CellFacts> {
    if a.rates != b.rates || a.totals.len() != b.totals.len() {
        return None;
    }
    let totals = a.totals.iter().zip(&b.totals).map(|(x, y)| x.checked_add(*y)).collect::<Option<Vec<i64>>>()?;
    Some(CellFacts { weight: a.weight.checked_add(b.weight)?, totals, rates: a.rates.clone() })
}

/// The gap joining two cells causes in one decision, per member and as a share of the member's scale: the decision's
/// total over their members together less its totals apart, so a decision straight in its inputs has none.
#[clause("REP.39", "REP.15")]
pub fn pair_gap(form: &GapForm, a: &CellFacts, b: &CellFacts) -> Missing<f64> {
    let Some(ab) = joined(a, b) else { return Missing::Absent };
    let (Missing::Present(fa), Missing::Present(fb), Missing::Present(fab)) =
        ((form.form)(a), (form.form)(b), (form.form)(&ab))
    else {
        return Missing::Absent;
    };
    let Some(&scale) = ab.totals.get(form.scale) else {
        violation!(clause = "REP.39", "a gap's scale on a position the kind does not hold", position = form.scale);
    };
    if scale == 0 || ab.weight == 0 {
        return Missing::Absent;
    }
    let members = f64::from(ab.weight);
    let apart = phx_rand::float::from_i64(fa) + phx_rand::float::from_i64(fb);
    let per_member = (phx_rand::float::from_i64(fab) - apart).abs() / members;
    Missing::Present(per_member / (phx_rand::float::from_i64(scale) / members).abs())
}

/// The gap each position's widening would cause, estimated on cells sampled from the representation's own
/// stream. Each sampled cell's partner is found by one index lookup — its landing key with that position's step
/// replaced by its sibling in the merge pair — and each decision's form is evaluated on the two apart and together.
/// Evaluating changes nothing.
#[clause("REP.39", "REP.28")]
#[must_use]
pub fn estimate<B: Backing>(
    table: &CellTable<B>,
    index: &Index,
    kind: &PopKindDecl,
    levels: &[u8],
    forms: &[GapForm],
    sample: u32,
    d: &mut Draws,
) -> Estimate {
    let n = kind.positions.len();
    let mut cells: Vec<(phx_id::PartyId, Slot)> =
        table.slots().filter(|s| !table.hot(*s).is_individual()).map(|s| (table.party(s), s)).collect();
    cells.sort_unstable();
    let mut est = Estimate { gap: vec![0.0; n], pairs: vec![0; n], sampled: 0 };
    let mut gaps: Vec<(f64, u32)> = vec![(0.0, 0); n];
    if cells.is_empty() {
        return est;
    }
    for _ in 0..sample {
        let Some((party, slot)) = usize::try_from(below_u64(d, phx_rand::float::len_u64(cells.len())))
            .ok()
            .and_then(|i| cells.get(i).copied())
        else {
            continue;
        };
        est.sampled += 1;
        let steps = table.steps(slot, kind, levels);
        let (key, sig) = (table.hot(slot).key_id, table.sig(slot));
        let me = facts(table, slot);
        for p in 0..n {
            let Some(sib) = steps.get(p).copied().and_then(Step::sibling) else { continue };
            let mut other = steps.clone();
            if let Some(s) = other.get_mut(p) {
                *s = sib;
            }
            let Some((_, partner)) =
                index.candidates(landing_key(key, &other, &sig)).into_iter().find(|(q, _)| *q != party)
            else {
                continue;
            };
            if let Some(c) = est.pairs.get_mut(p) {
                *c += 1;
            }
            let them = facts(table, partner);
            for form in forms {
                if let (Missing::Present(g), Some((sum, count))) = (pair_gap(form, &me, &them), gaps.get_mut(p)) {
                    *sum += g;
                    *count += 1;
                }
            }
        }
    }
    for (g, (sum, count)) in est.gap.iter_mut().zip(gaps) {
        if count > 0 {
            *g = sum / f64::from(count);
        }
    }
    est
}

/// Widening: the positions of smallest gap, one level each, until the cells their projected joins unite bring
/// the count within the budget. A sampled cell that found a partner projects one join for every two such cells. Ties
/// go by the kind's declared order, then by lot; a position merged into one step widens no further.
#[clause("REP.28")]
pub fn choose_widening(
    est: &Estimate,
    cells: u64,
    settings: &Tolerances,
    levels: &[u8],
    top: &[u8],
    d: &mut Draws,
) -> Vec<usize> {
    let mut order: Vec<(f64, usize, u64, usize)> = est
        .gap
        .iter()
        .enumerate()
        .filter(|(p, _)| levels.get(*p) < top.get(*p))
        .map(|(p, g)| {
            let declared = settings.widen_order.iter().position(|q| *q == p).unwrap_or(usize::MAX);
            (*g, declared, below_u64(d, u64::MAX), p)
        })
        .collect();
    order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    let mut chosen = Vec::new();
    // The joins projected so far, compared in two words so no sum of them can pass for fewer.
    let mut joins: u128 = 0;
    for (_, _, _, p) in order {
        if u128::from(cells) <= u128::from(settings.budget) + joins {
            break;
        }
        chosen.push(p);
        let found = u128::from(est.pairs.get(p).copied().unwrap_or(0));
        if let Some(j) = (u128::from(cells) * found).checked_div(u128::from(est.sampled) * 2) {
            joins += j;
        }
    }
    chosen
}

/// Narrowing on a light day: one position divided a level, the one of largest gap that is not at its base
/// partition, while the cells carried are below the declared share of the budget. Every member of a cell holds the same
/// per-member value, so no cell straddles the new boundary: the cells re-key and none splits.
#[clause("REP.28")]
pub fn choose_narrowing(est: &Estimate, cells: u64, settings: &Tolerances, levels: &[u8]) -> Missing<usize> {
    if settings.narrow_den == 0 {
        violation!(clause = "REP.28", "a narrowing share of no parts");
    }
    let Some(room) = settings.budget.checked_mul(settings.narrow_num).map(|x| x / settings.narrow_den) else {
        violation!(clause = "REP.28", "a narrowing share past a count's width", budget = settings.budget);
    };
    if cells >= room {
        return Missing::Absent;
    }
    let mut best: Missing<(f64, usize)> = Missing::Absent;
    for (p, g) in est.gap.iter().enumerate() {
        if levels.get(p).is_none_or(|l| *l == 0) {
            continue;
        }
        best = match best {
            Missing::Present((b, _)) if b.total_cmp(g).is_ge() => best,
            _ => Missing::Present((*g, p)),
        };
    }
    match best {
        Missing::Present((_, p)) => Missing::Present(p),
        Missing::Absent => Missing::Absent,
    }
}

/// One full sweep at the levels the context holds: every cell re-keyed by a shift, in order of identity, and the cells
/// that now share a landing key and pass the check landing in one another. Returns the cells re-keyed.
#[clause("REP.28", "REP.8")]
pub fn sweep<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, index: &mut Index, landed: &mut Landed) -> u64 {
    let all: Vec<Slot> = ctx.table.slots().filter(|s| !ctx.table.hot(*s).is_individual()).collect();
    rekey_flagged(ctx, index, &all, landed)
}

#[cfg(test)]
mod tests {
    use phx_id::Day;
    use phx_num::Missing;

    use super::{
        CellFacts, Estimate, GapForm, Tolerances, choose_narrowing, choose_widening, estimate, pair_gap, sweep,
    };
    use crate::fixture::{PLAIN, Spec, Ten, draws};
    use crate::landing::{Landed, TenB};

    fn tenb<'a>(ten: &'a mut Ten, levels: &'a [u8]) -> TenB<'a, phx_store::HeapBacking, phx_store::HeapBacking> {
        TenB {
            ledger: &mut ten.books.ledger,
            table: &mut ten.table,
            place: 0,
            keys: &mut ten.keys,
            directory: &mut ten.directory,
            space: &mut ten.space,
            kind: &ten.kind,
            levels,
            kinks: &ten.kinks,
            today: Day::new(3),
        }
    }

    #[test]
    fn widen_merges_pairs_and_shifts_keys() {
        let mut ten = Ten::new();
        // Per member 60, 100 and 250 of their scale: steps 2, 3 and 4 of the partition 0, 50, 100, 200.
        let (a, _) = ten.add(1, Spec { income: 6_000, ..PLAIN });
        let (b, _) = ten.add(1, PLAIN);
        let (c, _) = ten.add(1, Spec { income: 25_000, ..PLAIN });
        let mut index = std::mem::take(&mut ten.index);
        let mut landed = Landed::default();
        let rekeyed = sweep(&mut tenb(&mut ten, &[1]), &mut index, &mut landed);
        assert_eq!((rekeyed, landed.landings), (3, 1), "steps 2 and 3 are one step at level one; step 4 is apart");
        let live: Vec<_> = ten.table.slots().map(|s| ten.table.party(s)).collect();
        assert_eq!(live, [a, c], "the later identity lands in the earlier");
        assert!(!matches!(ten.directory.lookup(b), phx_core::directory::PartyState::Live(_)), "its identity ended");
        let sa = ten.table.slots().next().unwrap();
        assert_eq!((ten.table.weight(sa).get(), ten.table.position(sa, 0)), (200, 16_000), "no total moved");
        assert_eq!(index.entries(), crate::index::Index::rebuild(&ten.table).entries());
    }

    fn linear(c: &CellFacts) -> Missing<i64> {
        Missing::Present(3 * c.totals[0] + 7 * i64::from(c.weight))
    }

    fn capped(c: &CellFacts) -> Missing<i64> {
        // A per-member cap of 80: kinked, so joining members on either side of it changes the total.
        let per = c.totals[0] / i64::from(c.weight);
        Missing::Present(if per > 80 { 80 } else { per } * i64::from(c.weight))
    }

    #[test]
    fn gap_estimate_on_linear_rule_is_zero() {
        let a = CellFacts { weight: 100, totals: vec![6_000], rates: vec![Missing::Present(1)] };
        let b = CellFacts { weight: 50, totals: vec![5_000], rates: vec![Missing::Present(1)] };
        let lin = GapForm { name: "linear", form: linear, scale: 0 };
        assert_eq!(pair_gap(&lin, &a, &b), Missing::Present(0.0));
        let cap = GapForm { name: "capped", form: capped, scale: 0 };
        // Apart: 100 × 60 + 50 × 80 = 10 000; together, at 73 a member: 150 × 73 = 10 950; 950 ÷ 150 over 73.
        let Missing::Present(g) = pair_gap(&cap, &a, &b) else { panic!("a gap") };
        assert!((g - 950.0 / 150.0 / (11_000.0 / 150.0)).abs() < 1e-12, "{g}");
        let other = CellFacts { rates: vec![Missing::Present(2)], ..b };
        assert_eq!(pair_gap(&lin, &a, &other), Missing::Absent, "cells of other rates are never joined");
    }

    #[test]
    fn gap_estimate_samples_merge_pairs() {
        let mut ten = Ten::new();
        let _ = ten.add(1, Spec { income: 6_000, ..PLAIN });
        let _ = ten.add(1, PLAIN);
        let _ = ten.add(1, Spec { income: 25_000, ..PLAIN });
        let forms = [GapForm { name: "capped", form: capped, scale: 0 }];
        let est = estimate(&ten.table, &ten.index, &ten.kind, &[0], &forms, 3_000, &mut draws("REP.tolerance", 0));
        // Two of the three cells have a partner one level up; 3 000 draws of a third each find about 2 000 pairs,
        // with a standard deviation of √(3 000 × 2/3 × 1/3) ≈ 25.8, so within 6.1 of them but once in 10⁹.
        let found = f64::from(est.pairs[0]);
        assert!((found - 2_000.0).abs() < 6.1 * 25.9, "{found}");
        assert!(est.gap[0] > 0.0, "the cap lies between the pair's members");
        let mut alone = Ten::new();
        let _ = alone.add(1, Spec { income: 25_000, ..PLAIN });
        let _ = alone.add(2, PLAIN);
        let est = estimate(&alone.table, &alone.index, &alone.kind, &[0], &forms, 500, &mut draws("REP.tolerance", 1));
        assert_eq!(est.pairs[0], 0, "no partner across keys or outside the merge pair");
    }

    fn settings(order: Vec<usize>) -> Tolerances {
        Tolerances { budget: 100, sample: 0, narrow_num: 9, narrow_den: 10, widen_order: order }
    }

    #[test]
    fn widen_ties_by_declared_order() {
        let est = Estimate { gap: vec![0.0, 0.0, 0.0], pairs: vec![40, 40, 40], sampled: 100 };
        let mut d = draws("REP.tolerance", 2);
        let chosen = choose_widening(&est, 150, &settings(vec![2, 0, 1]), &[0, 0, 0], &[3, 3, 3], &mut d);
        assert_eq!(chosen, [2, 0], "each widening projects 30 of 150 joined: two bring it to 90");
        let chosen = choose_widening(&est, 150, &settings(vec![2, 0, 1]), &[0, 0, 3], &[3, 3, 3], &mut d);
        assert_eq!(chosen, [0, 1], "a position merged into one step widens no further");
        let smaller = Estimate { gap: vec![0.5, 0.25, 0.75], ..est };
        assert_eq!(choose_widening(&smaller, 120, &settings(vec![2, 0, 1]), &[0; 3], &[3; 3], &mut d), [1]);
        assert!(
            choose_widening(&smaller, 100, &settings(vec![]), &[0; 3], &[3; 3], &mut d).is_empty(),
            "within budget"
        );
    }

    #[test]
    fn narrowing_takes_the_largest_gap_below_the_share() {
        let est = Estimate { gap: vec![0.5, 0.25, 0.75], pairs: vec![0; 3], sampled: 10 };
        assert_eq!(choose_narrowing(&est, 80, &settings(vec![]), &[1, 1, 0]), Missing::Present(0));
        assert_eq!(choose_narrowing(&est, 95, &settings(vec![]), &[1, 1, 1]), Missing::Absent, "above 90 of 100");
        assert_eq!(choose_narrowing(&est, 10, &settings(vec![]), &[0, 0, 0]), Missing::Absent, "all at the base");
    }
}
