//! The core's audit at each day's close, beside the money and goods families settlement and the goods' day read: each
//! contract names parties that are live, and the households hold the persons they opened with, plus those born, less
//! those gone. It reads only and records what it finds; it never repairs.

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_id::Day;
use phx_macros::clause;

use crate::core::Core;

impl Core {
    /// The day's audit of the contracts and the persons, its findings kept for the run.
    #[clause("REP.3", "REP.26", "II.5")]
    pub(crate) fn audit(&mut self, day: Day) {
        let mut found = Vec::new();
        for family in &self.families {
            let edges = &family.store.edges;
            for edge in edges.open_slots() {
                for side in 0..2 {
                    let Some(end) = edges.end(edge, side) else { continue };
                    let live =
                        self.kinds.get(usize::from(end.kind())).is_some_and(|k| k.parties.at(end.slot()).is_some());
                    if !live {
                        found.push(Finding {
                            family: "contracts",
                            clause: "REP.3",
                            owner: FindingOwner::Run,
                            size: 1,
                            unit: Unit::Count,
                            day,
                            detail: format!("a contract of {} names an ended party", family.name),
                        });
                    }
                }
            }
        }
        let (born, gone): (u64, u64) = self.pop_days.iter().fold((0, 0), |(b, g), (_, d)| (b + d.born, g + d.gone));
        let held = self.persons_held();
        let expected = self.persons_opened + born - gone;
        if held != expected {
            found.push(Finding {
                family: "persons",
                clause: "REP.26",
                owner: FindingOwner::Run,
                size: i128::from(held) - i128::from(expected),
                unit: Unit::Count,
                day,
                detail: format!(
                    "the households hold {held} persons where the opening, births and deaths leave {expected}"
                ),
            });
        }
        self.found.extend(found);
    }
}
