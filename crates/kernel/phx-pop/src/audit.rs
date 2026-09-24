use phx_core::{
    AuditFamily, CellsAudit, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, Gap, InjectTarget, Unit,
    declare_family,
};
use phx_id::Slot;
use phx_ledger::part::cell_holdings;
use phx_ledger::rows;
use phx_macros::clause;
use phx_store::Backing;

use crate::key::{KeyInterner, KeyRecord};
use crate::landing::Landed;
use crate::table::CellTable;

declare_family! { pub REPRESENTATION = "REP.representation" { mode: Rolling { cycle_days: 30 }, clause: "REP.14" } }

/// A cell's weight, its profiles and its attachments: it stands for at least one member; each profile group counts
/// every person of its role, which each member of the cell holds as many of as its key says, so each group sums to the
/// weight times that count; and no row or holding is held by more members than the cell has.
#[clause("REP.14", "REP.17", "REP.31")]
#[must_use]
pub fn representation<B: Backing>(table: &CellTable<B>, slot: Slot, key: &KeyRecord) -> Vec<Gap> {
    let owner = FindingOwner::Party(table.party(slot));
    let weight = i128::from(table.weight(slot).get());
    let mut gaps = Vec::new();
    if weight == 0 {
        let detail = format!("cell {}: a weight of no members", table.party(slot).get());
        gaps.push(Gap { owner, size: 1, unit: Unit::Count, detail });
    }
    let profile = table.profile(slot);
    let layout = table.profile_layout();
    for g in 0..layout.groups.len() {
        let counted = i128::from(profile.members(g));
        let persons = i128::from(layout.persons(g, key, u64::from(table.weight(slot).get())));
        if counted != persons {
            let detail = format!(
                "cell {}: profile group {g} counts {counted} persons where a weight of {weight} holds {persons}",
                table.party(slot).get()
            );
            gaps.push(Gap { owner, size: counted - persons, unit: Unit::Count, detail });
        }
    }
    let holdings = cell_holdings(table, slot);
    let attached = rows::iter(table, slot)
        .map(|r| (format!("line {}", r.row.line.get()), r.row.count))
        .chain(holdings.iter().map(|h| (format!("instrument {}", h.instrument.get()), h.count)));
    for (what, count) in attached {
        let over = i128::from(count) - weight;
        if over > 0 {
            let detail =
                format!("cell {}: {what} held by {count} members of a weight of {weight}", table.party(slot).get());
            gaps.push(Gap { owner, size: over, unit: Unit::Count, detail });
        }
    }
    gaps
}

/// The population's cell tables as the audit reads them: every slot each table has handed out, table after table,
/// so a rolling slice is found by arithmetic and a freed slot is checked as nothing; each kind's population; and the
/// day's landings in each table.
pub struct CellsView<'a, B: Backing> {
    tables: Vec<&'a CellTable<B>>,
    keys: Vec<&'a KeyInterner>,
    populations: &'a [(&'static str, u64)],
    landed: &'a [Landed],
}

impl<B: Backing> core::fmt::Debug for CellsView<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CellsView").field("tables", &self.tables.len()).finish_non_exhaustive()
    }
}

impl<'a, B: Backing> CellsView<'a, B> {
    #[must_use]
    pub fn new(
        tables: Vec<&'a CellTable<B>>,
        keys: Vec<&'a KeyInterner>,
        populations: &'a [(&'static str, u64)],
        landed: &'a [Landed],
    ) -> CellsView<'a, B> {
        CellsView { tables, keys, populations, landed }
    }
}

fn slots(t: &CellTable<impl Backing>) -> usize {
    let Ok(n) = usize::try_from(t.high_water()) else {
        phx_num::capacity_exceeded!("index width", usize::MAX, t.high_water());
    };
    n
}

impl<B: Backing> CellsAudit for CellsView<'_, B> {
    fn cells(&self) -> usize {
        self.tables.iter().map(|t| slots(t)).sum()
    }

    fn representation(&self, cell: usize) -> Vec<Gap> {
        let mut at = cell;
        for (t, keys) in self.tables.iter().zip(&self.keys) {
            let n = slots(t);
            if at < n {
                let Ok(raw) = u32::try_from(at) else {
                    phx_num::capacity_exceeded!("slots of a cell table", u32::MAX, at);
                };
                let slot = Slot::new(raw);
                return if t.is_live(slot) {
                    representation(t, slot, &keys.record(t.hot(slot).key_id))
                } else {
                    Vec::new()
                };
            }
            at -= n;
        }
        phx_num::violation!(clause = "N1", "a cell read past the cells kept", index = cell);
    }

    /// Each table's weights summed against its kind's population; a kind whose population the world does not give
    /// cannot be shown whole, which is a gap of its own.
    fn populations(&self) -> Vec<Gap> {
        let mut gaps = Vec::new();
        for t in &self.tables {
            let weights: i128 = t.slots().map(|s| i128::from(t.weight(s).get())).sum();
            let Some((_, population)) = self.populations.iter().find(|(k, _)| *k == t.kind()) else {
                let detail = format!("`{}`: {weights} members and no population to hold them to", t.kind());
                gaps.push(Gap { owner: FindingOwner::Table(t.id()), size: weights, unit: Unit::Count, detail });
                continue;
            };
            if weights != i128::from(*population) {
                let detail = format!("`{}`: weights sum to {weights} of a population of {population}", t.kind());
                gaps.push(Gap {
                    owner: FindingOwner::Table(t.id()),
                    size: weights - i128::from(*population),
                    unit: Unit::Count,
                    detail,
                });
            }
        }
        gaps
    }

    /// Each position total the day's landings moved in a table.
    fn landings(&self) -> Vec<Gap> {
        let mut gaps = Vec::new();
        for (t, landed) in self.tables.iter().zip(self.landed) {
            for (i, by) in landed.moved.iter().enumerate().filter(|(_, by)| **by != 0) {
                let detail = format!("`{}`: the day's landings moved position {i} by {by}", t.kind());
                gaps.push(Gap { owner: FindingOwner::Table(t.id()), size: i128::from(*by), unit: Unit::Count, detail });
            }
        }
        gaps
    }
}

/// Every cell's weight against its profile counts and attachments, a slice of the cells a day; every day, each
/// population's weights against it and the day's landings against the totals they joined.
#[derive(Debug)]
pub struct Representation;

impl AuditFamily for Representation {
    fn decl(&self) -> FamilyDecl {
        REPRESENTATION
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let cells = ctx.cells();
        let span = ctx.rolling(cells.cells());
        let whole = cells.populations().into_iter().chain(cells.landings());
        for g in span.iter().flat_map(|i| cells.representation(i)).chain(whole) {
            findings.record(Finding {
                family: REPRESENTATION.name,
                clause: REPRESENTATION.clause,
                owner: g.owner,
                size: g.size,
                unit: g.unit,
                day: ctx.day(),
                detail: g.detail,
            });
        }
        phx_rand::float::len_u64(span.end - span.start)
    }

    /// A member taken off one profile value and nowhere added: the group counts one fewer than the weight.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let Some(tables) = target.cells().downcast_mut::<Vec<Box<dyn phx_ledger::holder::CellHolders>>>() else {
            return Err("the save's cells are not the population's tables".to_owned());
        };
        for t in tables.iter_mut().filter_map(|c| c.as_any_mut().downcast_mut::<CellTable>()) {
            let Some(slot) = t.slots().next() else { continue };
            let mut profile = t.profile(slot);
            let groups = t.profile_layout().groups.len();
            let Some((g, value)) = (0..groups).find_map(|g| profile.held(g).first().map(|(v, _)| (g, *v))) else {
                continue;
            };
            profile.remove(g, value, 1);
            t.set_profile(slot, &profile);
            return Ok(());
        }
        Err("the save keeps no cell with a profile".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{GroupDecl, KinkRegistry, PopEntry, PopItem, ProfileComponent, RoleDecl, Weight};
    use phx_id::{Day, PartyId, TableId};
    use phx_store::{AddressSpace, HeapBacking};

    use super::representation;
    use crate::key::{KeyInterner, KeyRecord};
    use crate::kind::PopKindDecl;
    use crate::profile::Profile;
    use crate::steps::StepTable;
    use crate::table::{CellTable, NewCell};

    const HEALTH: &[ProfileComponent] = &[ProfileComponent { name: "health", values: 3 }];

    fn steps(_: &'static str) -> Result<StepTable, String> {
        StepTable::new(&Partition { exp: 0, bounds: [1].into() })
    }

    #[test]
    fn profiles_count_every_member_of_their_role() {
        let entry = |item| PopEntry { system: "DEM", kind: "household", item };
        let entries = [
            entry(PopItem::Role(RoleDecl { name: "adult", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
            entry(PopItem::Role(RoleDecl {
                name: "child",
                per_member: phx_core::RoleCount::Key("DEM.children"),
                clause: "REP.26",
            })),
            entry(PopItem::KeyAttr(phx_core::KeyAttrDecl { name: "DEM.children", values: 4, clause: "REP.19" })),
            entry(PopItem::ProfileGroup(GroupDecl {
                name: "adult_h",
                role: "adult",
                components: HEALTH,
                clause: "REP.32",
            })),
            entry(PopItem::ProfileGroup(GroupDecl {
                name: "child_h",
                role: "child",
                components: HEALTH,
                clause: "REP.32",
            })),
        ];
        let kind = PopKindDecl::compile("household", &entries, &KinkRegistry::default(), &steps).unwrap();
        let mut space = AddressSpace::empty();
        let mut t: CellTable<HeapBacking> = CellTable::new(&mut space, &kind, TableId::new(2), 64, 8);
        let layout = t.profile_layout().clone();
        let mut p = Profile::empty(&layout);
        p.add(&layout, 0, 1, 7);
        p.add(&layout, 0, 2, 3);
        p.add(&layout, 1, 0, 20);
        let mut record = KeyRecord::default();
        kind.key.set(&mut record, 0, 2);
        let mut keys = KeyInterner::new();
        keys.hold(record, 1);
        let phx_num::Missing::Present(key) = keys.id(&record) else { panic!("the key is held") };
        let new = NewCell {
            party: PartyId::new(8),
            created: Day::new(1),
            weight: Weight::new(10),
            key,
            positions: &[],
            profile: &p,
        };
        let s = t.add(&mut space, new, &kind, &[]);
        assert!(representation(&t, s, &record).is_empty(), "ten households of two children each count twenty");
        p.remove(1, 0, 1);
        t.set_profile(s, &p);
        let gaps = representation(&t, s, &record);
        assert_eq!(gaps.iter().map(|g| g.size).collect::<Vec<_>>(), [-1], "one child uncounted");
        let view = super::CellsView::new(vec![&t], vec![&keys], &[("household", 10)], &[]);
        assert_eq!((phx_core::CellsAudit::cells(&view), phx_core::CellsAudit::representation(&view, 0).len()), (1, 1));
        assert!(phx_core::CellsAudit::populations(&view).is_empty(), "ten members of a population of ten");
        let short = super::CellsView::new(vec![&t], vec![&keys], &[("household", 12)], &[]);
        let gaps = phx_core::CellsAudit::populations(&short);
        assert_eq!(gaps.iter().map(|g| g.size).collect::<Vec<_>>(), [-2], "two households nowhere");
    }
}
