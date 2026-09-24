use libm::{expm1, log1p};
use phx_id::{Day, Slot};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::{Draws, binomials_joint_at_least_one, open_unit};
use phx_store::Backing;

use crate::envelope::{Booking, candidate, envelope, next_booking, rung};
use crate::table::CellTable;

/// A process as the screen reads it over one cell: the profile group its rate is read at, each joint value's daily
/// rate on a day, and, for a scheduled process, the largest rate the cell's values reach from a day until the first
/// day any input of the rate may change, with that day.
pub struct Process<'a> {
    pub group: usize,
    pub rate: &'a dyn Fn(u32, Day) -> f64,
    pub envelope: &'a EnvelopeFn,
}

/// The largest rate a cell's values reach from a day, and the first day any input of the rate may change.
pub type EnvelopeFn = dyn Fn(&[u32], Day) -> (f64, Missing<Day>);

impl core::fmt::Debug for Process<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process").field("group", &self.group).finish_non_exhaustive()
    }
}

/// Members of a cell a process hit on a day, per joint value of its group, and how many it was screened over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub row: Slot,
    pub exposed: u64,
    pub by_value: Vec<(u32, u64)>,
}

/// What the screen did, for the budget's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenCounters {
    pub candidates: u64,
    pub redraws: u64,
    pub dense_evals: u64,
}

/// The values a cell's members hold in a process's group, and how many hold each.
fn held<B: Backing>(table: &CellTable<B>, slot: Slot, group: usize) -> (Vec<u32>, Vec<u64>) {
    let held = table.profile_group(slot, group);
    (held.iter().map(|(v, _)| *v).collect(), held.iter().map(|(_, n)| u64::from(*n)).collect())
}

/// A cell's first booking for a process, or its booking redrawn: the envelope over its values from `first` until the
/// rate may next change, and the candidate drawn at it.
#[clause("REP.7")]
pub fn book<B: Backing>(
    table: &CellTable<B>,
    slot: Slot,
    process: &Process<'_>,
    first: Day,
    d: &mut Draws,
    counters: &mut ScreenCounters,
) -> Missing<Booking> {
    counters.redraws += 1;
    let (values, _) = held(table, slot, process.group);
    let (p_bar, changes) = (process.envelope)(&values, first);
    let chance = envelope(p_bar, rung(table.weight(slot).get()));
    if chance <= 0.0 {
        // A cell no member of which can be hit books only the day its rates may change.
        return match changes {
            Missing::Present(v) => Missing::Present(Booking::Redraw(v)),
            Missing::Absent => Missing::Absent,
        };
    }
    next_booking(d, first, chance, changes)
}

/// A cell's candidate day for a process at 3b: thinned by the chance its members give today over the envelope it
/// was drawn at, the hits drawn per value if accepted, and its next booking drawn at the same envelope. A weight that
/// has risen past its rung since the booking was drawn books afresh from today instead.
#[clause("REP.7", "REP.12")]
pub fn screen_candidate<B: Backing>(
    table: &CellTable<B>,
    slot: Slot,
    process: &Process<'_>,
    today: Day,
    booked_rung: u64,
    d: &mut Draws,
    counters: &mut ScreenCounters,
) -> (Option<Hit>, Missing<Booking>) {
    counters.candidates += 1;
    let weight = table.weight(slot).get();
    if u64::from(weight) > booked_rung {
        return (None, book(table, slot, process, today, d, counters));
    }
    let (values, counts) = held(table, slot, process.group);
    let (p_bar, changes) = (process.envelope)(&values, today);
    let rates: Vec<f64> = values.iter().map(|v| (process.rate)(*v, today)).collect();
    let mut out = vec![0_u64; counts.len()];
    let hit = candidate(d, &counts, &rates, p_bar, booked_rung, &mut out).then(|| Hit {
        row: slot,
        exposed: counts.iter().sum(),
        by_value: values.iter().copied().zip(out).filter(|(_, k)| *k > 0).collect(),
    });
    (hit, next_booking(d, today.succ(), envelope(p_bar, booked_rung), changes))
}

/// A dense process screened daily: the cell's chance of a hit today from every value's rate, one `expm1` for the
/// cell, and the hits per value drawn when it is hit.
#[clause("REP.7")]
pub fn screen_daily<B: Backing>(
    table: &CellTable<B>,
    slot: Slot,
    process: &Process<'_>,
    today: Day,
    d: &mut Draws,
    counters: &mut ScreenCounters,
) -> Option<Hit> {
    counters.dense_evals += 1;
    let (values, counts) = held(table, slot, process.group);
    let rates: Vec<f64> = values.iter().map(|v| (process.rate)(*v, today)).collect();
    let log_escape: f64 = counts.iter().zip(&rates).map(|(n, p)| phx_rand::float::from_u64(*n) * log1p(-p)).sum();
    if open_unit(d) >= -expm1(log_escape) {
        return None;
    }
    let mut out = vec![0_u64; counts.len()];
    binomials_joint_at_least_one(d, &counts, &rates, &mut out);
    Some(Hit {
        row: slot,
        exposed: counts.iter().sum(),
        by_value: values.into_iter().zip(out).filter(|(_, k)| *k > 0).collect(),
    })
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{GroupDecl, KinkRegistry, PopEntry, PopItem, ProfileComponent, RoleDecl, Weight};
    use phx_id::{Day, PartyId, Slot, TableId};
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Process, ScreenCounters, book, screen_candidate, screen_daily};
    use crate::envelope::{Booking, rung};
    use crate::key::KeyId;
    use crate::kind::PopKindDecl;
    use crate::profile::Profile;
    use crate::steps::StepTable;
    use crate::table::{CellTable, NewCell};

    const AGE: &[ProfileComponent] = &[ProfileComponent { name: "age_band", values: 3 }];

    fn steps(_: &'static str) -> Result<StepTable, String> {
        StepTable::new(&Partition { exp: 0, bounds: [1].into() })
    }

    fn cell() -> (CellTable<HeapBacking>, Slot) {
        let entry = |item| PopEntry { system: "DEM", kind: "household", item };
        let entries = [
            entry(PopItem::Role(RoleDecl { name: "person", clause: "REP.26" })),
            entry(PopItem::ProfileGroup(GroupDecl { name: "age", role: "person", components: AGE, clause: "REP.32" })),
        ];
        let kind = PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap();
        let mut space = AddressSpace::empty();
        let mut t = CellTable::new(&mut space, &kind, TableId::new(1), 64, 8);
        let layout = t.profile_layout().clone();
        let mut p = Profile::empty(&layout);
        p.add(&layout, 0, 0, 60);
        p.add(&layout, 0, 2, 40);
        let new = NewCell {
            party: PartyId::new(1),
            created: Day::new(0),
            weight: Weight::new(100),
            key: KeyId::new(0),
            positions: &[],
            profile: &p,
        };
        let s = t.add(&mut space, new, &kind, &[]);
        (t, s)
    }

    fn draws(i: u32) -> Draws {
        Draws::new(stream_key(Seed::new(9), "DEM.illness"), Subject::new(SubjectTag::Party, 1), i, 0)
    }

    #[test]
    fn a_cell_is_screened_on_its_candidate_days() {
        let (t, s) = cell();
        let rate = |v: u32, _: Day| if v == 2 { 0.01 } else { 0.002 };
        let window =
            |values: &[u32], _: Day| (if values.contains(&2) { 0.01 } else { 0.002 }, Missing::Present(Day::new(365)));
        let process = Process { group: 0, rate: &rate, envelope: &window };
        let mut counters = ScreenCounters::default();
        let mut next = book(&t, s, &process, Day::new(0), &mut draws(0), &mut counters);
        let (mut hits, mut members_hit) = (0_u64, 0_u64);
        while let Missing::Present(Booking::Candidate(day)) = next {
            let (hit, after) = screen_candidate(&t, s, &process, day, rung(100), &mut draws(day.get()), &mut counters);
            if let Some(h) = hit {
                hits += 1;
                assert_eq!(h.exposed, 100);
                assert!(h.by_value.iter().all(|(v, k)| (*v == 0 && *k <= 60) || (*v == 2 && *k <= 40)));
                members_hit += h.by_value.iter().map(|(_, k)| k).sum::<u64>();
            }
            next = after;
        }
        assert_eq!(next, Missing::Present(Booking::Redraw(Day::new(365))), "a year's window ends in a redraw");
        // Over a year a hundred members at these rates are hit about 0.52 times a day.
        let expected = 365.0 * (60.0 * 0.002 + 40.0 * 0.01);
        assert!(
            (phx_rand::float::from_u64(members_hit) - expected).abs() < 5.0 * expected.sqrt(),
            "{members_hit} against {expected}"
        );
        assert!(hits > 0 && counters.candidates >= hits && counters.redraws == 1);
    }

    #[test]
    fn a_weight_past_its_rung_books_afresh_and_dense_processes_run_daily() {
        let (t, s) = cell();
        let rate = |_: u32, _: Day| 1.0;
        let window = |_: &[u32], _: Day| (1.0, Missing::Absent);
        let process = Process { group: 0, rate: &rate, envelope: &window };
        let mut counters = ScreenCounters::default();
        let (hit, next) = screen_candidate(&t, s, &process, Day::new(3), rung(50), &mut draws(1), &mut counters);
        assert_eq!((hit, next), (None, Missing::Present(Booking::Candidate(Day::new(3)))), "a certain hit, today");
        assert_eq!(counters.redraws, 1);
        let all = screen_daily(&t, s, &process, Day::new(3), &mut draws(2), &mut counters).unwrap();
        assert_eq!(all.by_value, [(0, 60), (2, 40)], "every member hit at a rate of one");
    }
}
