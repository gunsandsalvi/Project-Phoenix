//! Tracers: members drawn at the opening from the observer's own stream, each with its own values of its kind's groups
//! counted once per member, and followed through the day's splits with the observer's draws. A tracer is a read: it
//! changes nothing of the world, and a world run with tracers is the world run without them.

use std::collections::BTreeMap;

use phx_core::{TRACER_STREAM, TracedCells};
use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::Missing;
use phx_rand::{Draws, Subject, SubjectTag, below_u64, open_unit};
use phx_world::observe::{Counts, Split};
use phx_world::{Inspector, Observer};

use crate::reads::Recorder;

/// One member followed: its kind, the cell it is in, its values of the groups counted once per member, the day it
/// began and each day it moved and where to, and the day its cell ended with no successor, if it has.
#[clause("REP.30", "OBS.8")]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tracer {
    pub kind: usize,
    pub cell: PartyId,
    pub values: Vec<(usize, u32)>,
    pub since: Day,
    pub moves: Vec<(Day, PartyId)>,
    pub ended: Missing<Day>,
}

/// Every tracer, and the cells that hold one.
#[derive(Clone, Debug, Default)]
pub struct Tracers {
    tracers: Vec<Tracer>,
    traced: BTreeMap<PartyId, u32>,
}

/// How much weight the members of one side of a split carry for a tracer of these values: its members times, for each
/// group, the share of them holding the tracer's value, as though a cell's groups were drawn apart from each other.
fn score(weight: u32, counts: &Counts, values: &[(usize, u32)]) -> f64 {
    if weight == 0 {
        return 0.0;
    }
    let w = f64::from(weight);
    values.iter().fold(w, |s, (g, v)| {
        let held = counts.iter().filter(|(cg, cv, _)| cg == g && cv == v).map(|(_, _, n)| f64::from(*n)).sum::<f64>();
        s * held / w
    })
}

/// Which side of a split a tracer is on: nought for the cell it was in, `i + 1` for the `i`th that left, each with
/// probability its score's share.
#[clause("REP.30")]
#[must_use]
pub fn follow(split: &Split, values: &[(usize, u32)], d: &mut Draws) -> usize {
    let scores: Vec<f64> = std::iter::once(score(split.stayed, &split.stayed_counts, values))
        .chain(split.left.iter().map(|l| score(l.weight, &l.counts, values)))
        .collect();
    let total: f64 = scores.iter().sum();
    if total <= 0.0 {
        return 0;
    }
    let mut at = open_unit(d) * total;
    for (i, s) in scores.iter().enumerate() {
        if at < *s {
            return i;
        }
        at -= s;
    }
    scores.iter().rposition(|s| *s > 0.0).unwrap_or(0)
}

impl Tracers {
    /// The declared number of tracers, each a member drawn from every kind's cells alike, its values drawn from its
    /// cell's counts of each group counted once per member.
    #[clause("REP.30")]
    #[must_use]
    pub fn open(w: Inspector<'_>, count: u64) -> Tracers {
        let draws = w.observer_draws();
        let Ok(mut d) = draws.open(&TRACER_STREAM, Subject::new(SubjectTag::World, 0), w.today()) else {
            return Tracers::default();
        };
        let mut members: Vec<(usize, phx_id::Slot, u64)> = Vec::new();
        for (k, _) in w.population().kinds.iter().enumerate() {
            let table = w.cell_table(k);
            members.extend(table.slots().map(|s| (k, s, u64::from(table.weight(s).get()))));
        }
        let all: u64 = members.iter().map(|(_, _, m)| m).sum();
        let mut out = Tracers::default();
        if all == 0 {
            return out;
        }
        for _ in 0..count {
            let mut at = below_u64(&mut d, all);
            let Some((k, slot)) = members.iter().find_map(|(k, s, m)| {
                if at < *m {
                    Some((*k, *s))
                } else {
                    at -= m;
                    None
                }
            }) else {
                continue;
            };
            let table = w.cell_table(k);
            let Some(kind) = w.population().kinds.get(k) else { continue };
            let profile = table.profile(slot);
            let mut values = Vec::new();
            for (g, group) in kind.decl.groups.iter().enumerate() {
                if !matches!(group.per_member, Missing::Absent) {
                    continue;
                }
                let held = profile.held(g);
                let n: u64 = held.iter().map(|(_, c)| u64::from(*c)).sum();
                if n == 0 {
                    continue;
                }
                let mut at = below_u64(&mut d, n);
                if let Some(v) = held.iter().find_map(|(v, c)| {
                    if at < u64::from(*c) {
                        Some(*v)
                    } else {
                        at -= u64::from(*c);
                        None
                    }
                }) {
                    values.push((g, v));
                }
            }
            let cell = table.party(slot);
            out.tracers.push(Tracer {
                kind: k,
                cell,
                values,
                since: w.today(),
                moves: Vec::new(),
                ended: Missing::Absent,
            });
        }
        out.reindex();
        out
    }

    /// The tracers the register declares.
    ///
    /// # Errors
    /// A register that declares no number of tracers.
    pub fn declared(w: Inspector<'_>) -> Result<Tracers, String> {
        Ok(Tracers::open(w, w.register().count(phx_core::TRACERS.id)?))
    }

    fn reindex(&mut self) {
        self.traced.clear();
        for t in self.tracers.iter().filter(|t| t.ended == Missing::Absent) {
            *self.traced.entry(t.cell).or_insert(0) += 1;
        }
    }

    /// Follows every tracer through the day's splits in order, then to the successor of a cell that ended, and ends
    /// one whose cell ended with none.
    #[clause("REP.30", "Law 17")]
    pub fn follow_day(&mut self, w: Inspector<'_>) {
        let day = w.today();
        let draws = w.observer_draws();
        for (i, split) in w.split_log().splits.iter().enumerate() {
            for (t, tracer) in self.tracers.iter_mut().enumerate() {
                if tracer.ended != Missing::Absent || tracer.cell != split.origin || tracer.kind != split.kind {
                    continue;
                }
                let subject = Subject::new(
                    SubjectTag::Part,
                    (u64::try_from(i).unwrap_or(u64::MAX) << u32::BITS) | u64::try_from(t).unwrap_or(0),
                );
                let Ok(mut d) = draws.open(&TRACER_STREAM, subject, day) else { continue };
                let side = follow(split, &tracer.values, &mut d);
                if let Some(left) = side.checked_sub(1).and_then(|l| split.left.get(l)) {
                    tracer.cell = left.landed;
                    tracer.moves.push((day, left.landed));
                }
            }
        }
        let directory = w.books().parties.directory();
        for tracer in self.tracers.iter_mut().filter(|t| t.ended == Missing::Absent) {
            match directory.resolve(tracer.cell) {
                phx_core::Resolved::Live(p, _) if p == tracer.cell => {}
                phx_core::Resolved::Live(p, _) => {
                    tracer.cell = p;
                    tracer.moves.push((day, p));
                }
                phx_core::Resolved::Ended(_) | phx_core::Resolved::Unknown => tracer.ended = Missing::Present(day),
            }
        }
        self.reindex();
    }

    #[must_use]
    pub fn tracers(&self) -> &[Tracer] {
        &self.tracers
    }
}

impl TracedCells for Tracers {
    fn is_traced(&self, cell: PartyId) -> bool {
        self.traced.contains_key(&cell)
    }
}

/// The observer the host runs beside the world: the tracers followed each day, and the reads taken each day.
#[derive(Debug)]
pub struct Watch {
    pub tracers: Tracers,
    pub recorder: Recorder,
}

impl TracedCells for Watch {
    fn is_traced(&self, cell: PartyId) -> bool {
        self.tracers.is_traced(cell)
    }
}

impl Observer for Watch {
    fn day_closed(&mut self, w: Inspector<'_>) {
        self.tracers.follow_day(w);
        self.recorder.read(w);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_rand::{Seed, Subject, SubjectTag, stream_key};
    use phx_world::observe::{Left, Split};

    use super::follow;

    fn split() -> Split {
        Split {
            kind: 0,
            origin: PartyId::new(1),
            stayed: 90,
            stayed_counts: vec![(0, 1, 90)],
            left: vec![Left { weight: 10, counts: vec![(0, 1, 5), (0, 2, 5)], landed: PartyId::new(2) }],
        }
    }

    #[test]
    fn tracer_follow_probability() {
        let key = stream_key(Seed::new(9), "OBS.tracer");
        let draws = 20_000_u32;
        let mut moved = [0_u32; 2];
        for i in 0..draws {
            let mut d = phx_rand::Draws::new(key, Subject::new(SubjectTag::Part, u64::from(i)), 0, 0);
            moved[0] += u32::from(follow(&split(), &[(0, 1)], &mut d) == 1);
            let mut d = phx_rand::Draws::new(key, Subject::new(SubjectTag::Part, u64::from(i)), 1, 0);
            moved[1] += u32::from(follow(&split(), &[(0, 2)], &mut d) == 1);
        }
        // Of the 95 members holding value 1, five left: about one in nineteen follows them; all of value 2 left.
        let share = f64::from(moved[0]) / f64::from(draws);
        assert!((share - 5.0 / 95.0).abs() < 0.01, "{share}");
        assert_eq!(moved[1], draws, "every member holding value 2 left, so every such tracer follows");
    }
}
