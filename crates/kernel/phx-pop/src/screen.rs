use libm::{expm1, log1p};
use phx_id::{Day, Slot};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded};
use phx_rand::{Draws, binomials_joint_at_least_one, open_unit};
use phx_store::Backing;

use crate::envelope::{Booking, candidate, envelope, next_booking, rung};
use crate::table::CellTable;
use phx_id::TableId;

/// A process as the screen reads it over one cell: the profile groups its rate is read at, the persons each household
/// of the cell holds in them, each joint value's daily rate on a day, and, for a scheduled process, the largest rate
/// the cell's values reach from a day until the first day any input of the rate may change, with that day.
pub struct Process<'a> {
    pub groups: &'a [usize],
    pub persons: u64,
    pub rate: &'a dyn Fn(usize, u32, Day) -> f64,
    pub envelope: &'a EnvelopeFn<'a>,
}

/// The largest rate a cell's values, each in its group, reach from a day, and the first day any input of the rate may
/// change.
pub type EnvelopeFn<'a> = dyn Fn(&[(usize, u32)], Day) -> (f64, Missing<Day>) + 'a;

impl core::fmt::Debug for Process<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process").field("groups", &self.groups).finish_non_exhaustive()
    }
}

/// Persons of a cell a process hit on a day, per group and joint value, and how many it was screened over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub row: Slot,
    pub exposed: u64,
    pub by_value: Vec<(usize, u32, u64)>,
}

/// What the screen did, for the budget's counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenCounters {
    pub candidates: u64,
    pub redraws: u64,
    pub dense_evals: u64,
}

/// The values a cell's persons hold in a process's groups, each with its group, and how many hold each.
fn held<B: Backing>(table: &CellTable<B>, slot: Slot, groups: &[usize]) -> (Vec<(usize, u32)>, Vec<u64>) {
    let (mut values, mut counts) = (Vec::new(), Vec::new());
    for g in groups {
        for (v, n) in table.profile_group(slot, *g) {
            values.push((*g, v));
            counts.push(u64::from(n));
        }
    }
    (values, counts)
}

/// The persons a weight rung stands for: its households times the persons each holds in the process's groups.
fn persons_under(rung: u64, persons: u64) -> u64 {
    let Some(n) = rung.checked_mul(persons) else {
        capacity_exceeded!("persons under a weight rung", u64::MAX, rung);
    };
    n
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
    let (values, _) = held(table, slot, process.groups);
    let (p_bar, changes) = (process.envelope)(&values, first);
    let chance = envelope(p_bar, persons_under(rung(table.weight(slot).get()), process.persons));
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
    let (values, counts) = held(table, slot, process.groups);
    let (p_bar, changes) = (process.envelope)(&values, today);
    let rates: Vec<f64> = values.iter().map(|(g, v)| (process.rate)(*g, *v, today)).collect();
    let under = persons_under(booked_rung, process.persons);
    let mut out = vec![0_u64; counts.len()];
    let hit = candidate(d, &counts, &rates, p_bar, under, &mut out).then(|| Hit {
        row: slot,
        exposed: counts.iter().sum(),
        by_value: values.iter().zip(out).filter(|(_, k)| *k > 0).map(|((g, v), k)| (*g, *v, k)).collect(),
    });
    (hit, next_booking(d, today.succ(), envelope(p_bar, under), changes))
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
    let (values, counts) = held(table, slot, process.groups);
    let rates: Vec<f64> = values.iter().map(|(g, v)| (process.rate)(*g, *v, today)).collect();
    let log_escape: f64 = counts.iter().zip(&rates).map(|(n, p)| phx_rand::float::from_u64(*n) * log1p(-p)).sum();
    if open_unit(d) >= -expm1(log_escape) {
        return None;
    }
    let mut out = vec![0_u64; counts.len()];
    binomials_joint_at_least_one(d, &counts, &rates, &mut out);
    Some(Hit {
        row: slot,
        exposed: counts.iter().sum(),
        by_value: values.into_iter().zip(out).filter(|(_, k)| *k > 0).map(|((g, v), k)| (g, v, k)).collect(),
    })
}

fn rung_word(weight: u32) -> u32 {
    let r = rung(weight);
    let Ok(r) = u32::try_from(r) else {
        phx_num::capacity_exceeded!("a weight rung", u32::MAX, r);
    };
    r
}

/// One cell's screening for one process on a day the agenda gathered it: booked afresh when its booking was a redraw or
/// none has been drawn, then every candidate that falls today thinned, and the next booking kept in the agenda with the
/// weight rung it was drawn at, which thinning reads. A redraw is kept as a booking drawn with no rung.
#[clause("REP.7", "REP.12")]
pub fn screen_due<B: Backing, A: Backing>(
    table: &CellTable<B>,
    slot: Slot,
    process: &Process<'_>,
    at: (TableId, usize, Day),
    agenda: &mut phx_core::Agenda<A>,
    d: &mut Draws,
    counters: &mut ScreenCounters,
) -> Option<Hit> {
    let (tid, reason, day) = at;
    let weight = table.weight(slot).get();
    let mut with = agenda.with(tid, slot, reason);
    let mut booking = if with == 0 {
        with = rung_word(weight);
        book(table, slot, process, day, d, counters)
    } else {
        Missing::Present(Booking::Candidate(day))
    };
    let mut hit: Option<Hit> = None;
    loop {
        match booking {
            Missing::Present(Booking::Candidate(c)) if c == day => {
                let (h, next) = screen_candidate(table, slot, process, day, u64::from(with), d, counters);
                if weight > with {
                    with = rung_word(weight);
                }
                if let Some(h) = h {
                    hit = Some(match hit {
                        Some(mut prior) => {
                            prior.by_value.extend(h.by_value);
                            prior
                        }
                        None => h,
                    });
                }
                booking = next;
            }
            Missing::Present(Booking::Candidate(c)) => {
                agenda.set_next_with(tid, slot, reason, c, with);
                break;
            }
            Missing::Present(Booking::Redraw(r)) if r == day => {
                with = rung_word(weight);
                booking = book(table, slot, process, day, d, counters);
            }
            Missing::Present(Booking::Redraw(r)) => {
                agenda.set_next_with(tid, slot, reason, r, 0);
                break;
            }
            Missing::Absent => break,
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{GroupDecl, KinkRegistry, PopEntry, PopItem, ProfileComponent, RoleDecl, Weight};
    use phx_id::{Day, PartyId, Slot, TableId};
    use phx_num::Missing;
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{Process, ScreenCounters, book, screen_candidate, screen_daily, screen_due};
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
            entry(PopItem::Role(RoleDecl { name: "person", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
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
        let rate = |_: usize, v: u32, _: Day| if v == 2 { 0.01 } else { 0.002 };
        let window = |values: &[(usize, u32)], _: Day| {
            (if values.contains(&(0, 2)) { 0.01 } else { 0.002 }, Missing::Present(Day::new(365)))
        };
        let process = Process { groups: &[0], persons: 1, rate: &rate, envelope: &window };
        let mut counters = ScreenCounters::default();
        let mut next = book(&t, s, &process, Day::new(0), &mut draws(0), &mut counters);
        let (mut hits, mut members_hit) = (0_u64, 0_u64);
        while let Missing::Present(Booking::Candidate(day)) = next {
            let (hit, after) = screen_candidate(&t, s, &process, day, rung(100), &mut draws(day.get()), &mut counters);
            if let Some(h) = hit {
                hits += 1;
                assert_eq!(h.exposed, 100);
                assert!(h.by_value.iter().all(|(_, v, k)| (*v == 0 && *k <= 60) || (*v == 2 && *k <= 40)));
                members_hit += h.by_value.iter().map(|(_, _, k)| k).sum::<u64>();
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
    fn a_cell_screened_through_the_agenda_is_hit_at_its_rates() {
        let (t, s) = cell();
        let rate = |_: usize, v: u32, _: Day| if v == 2 { 0.01 } else { 0.002 };
        // The rates change at each year's start, so the booking is drawn afresh on day 365.
        let window = |values: &[(usize, u32)], d: Day| {
            let bar = if values.contains(&(0, 2)) { 0.01 } else { 0.002 };
            (bar, Missing::Present(Day::new((d.get() / 365 + 1) * 365)))
        };
        let process = Process { groups: &[0], persons: 1, rate: &rate, envelope: &window };
        let tid = TableId::new(1);
        let spec = [phx_core::AgendaTableSpec { table: tid, max_rows: 8, reasons: 1 }];
        let mut agenda: phx_core::Agenda<HeapBacking> =
            phx_core::Agenda::new(&mut AddressSpace::empty(), Day::new(0), &spec, 1 << 10).unwrap();
        agenda.grow(tid, 8);
        agenda.set_next_with(tid, s, 0, Day::new(1), 0);
        let (mut counters, mut members_hit) = (ScreenCounters::default(), 0_u64);
        let days = 730;
        for day in 1..=days {
            for today in agenda.gather(Day::new(day)).per_table {
                for (slot, mask) in today.slots.iter().zip(&today.reasons) {
                    assert_eq!((*slot, *mask), (s, 1), "only the cell, only for its process");
                    let at = (tid, 0, Day::new(day));
                    if let Some(h) = screen_due(&t, s, &process, at, &mut agenda, &mut draws(day), &mut counters) {
                        members_hit += h.by_value.iter().map(|(_, _, k)| k).sum::<u64>();
                    }
                }
            }
            assert!(agenda.booked(tid, s).is_some(), "the cell always holds its next booking");
        }
        assert_eq!(agenda.with(tid, s, 0) == 0, agenda.booked(tid, s) == Some(Day::new(730)), "a redraw has no rung");
        let expected = f64::from(days) * (60.0 * 0.002 + 40.0 * 0.01);
        assert!(
            (phx_rand::float::from_u64(members_hit) - expected).abs() < 5.0 * expected.sqrt(),
            "{members_hit} against {expected}"
        );
        assert!(counters.redraws >= 2, "a fresh booking and one at each year's start");
    }

    #[test]
    fn a_weight_past_its_rung_books_afresh_and_dense_processes_run_daily() {
        let (t, s) = cell();
        let rate = |_: usize, _: u32, _: Day| 1.0;
        let window = |_: &[(usize, u32)], _: Day| (1.0, Missing::Absent);
        let process = Process { groups: &[0], persons: 1, rate: &rate, envelope: &window };
        let mut counters = ScreenCounters::default();
        let (hit, next) = screen_candidate(&t, s, &process, Day::new(3), rung(50), &mut draws(1), &mut counters);
        assert_eq!((hit, next), (None, Missing::Present(Booking::Candidate(Day::new(3)))), "a certain hit, today");
        assert_eq!(counters.redraws, 1);
        let all = screen_daily(&t, s, &process, Day::new(3), &mut draws(2), &mut counters).unwrap();
        assert_eq!(all.by_value, [(0, 0, 60), (0, 2, 40)], "every member hit at a rate of one");
    }
}
