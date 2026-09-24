use phx_core::{
    AuditFamily, CellsAudit, FamilyCtx, FamilyDecl, Finding, FindingOwner, Findings, Gap, InjectTarget, Unit,
    declare_family,
};
use phx_id::Slot;
use phx_macros::clause;
use phx_store::Backing;

use crate::table::CellTable;

declare_family! { pub REPRESENTATION = "REP.representation" { mode: Rolling { cycle_days: 30 }, clause: "REP.14" } }

/// A cell's weight and its profiles: it stands for at least one member, and each profile group counts every member
/// of its role, which every member of the cell holds, so each group sums to the weight.
#[clause("REP.14", "REP.17")]
#[must_use]
pub fn representation<B: Backing>(table: &CellTable<B>, slot: Slot) -> Vec<Gap> {
    let owner = FindingOwner::Party(table.party(slot));
    let weight = i128::from(table.weight(slot).get());
    let mut gaps = Vec::new();
    if weight == 0 {
        let detail = format!("cell {}: a weight of no members", table.party(slot).get());
        gaps.push(Gap { owner, size: 1, unit: Unit::Count, detail });
    }
    let profile = table.profile(slot);
    for g in 0..table.profile_layout().groups.len() {
        let counted = i128::from(profile.members(g));
        if counted != weight {
            let detail = format!(
                "cell {}: profile group {g} counts {counted} members of a weight of {weight}",
                table.party(slot).get()
            );
            gaps.push(Gap { owner, size: counted - weight, unit: Unit::Count, detail });
        }
    }
    gaps
}

/// The population's cell tables as the audit reads them: every slot each table has handed out, table after table,
/// so a rolling slice is found by arithmetic and a freed slot is checked as nothing.
pub struct CellsView<'a, B: Backing> {
    tables: &'a [CellTable<B>],
}

impl<B: Backing> core::fmt::Debug for CellsView<'_, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CellsView").field("tables", &self.tables.len()).finish_non_exhaustive()
    }
}

impl<'a, B: Backing> CellsView<'a, B> {
    #[must_use]
    pub fn new(tables: &'a [CellTable<B>]) -> CellsView<'a, B> {
        CellsView { tables }
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
        self.tables.iter().map(slots).sum()
    }

    fn representation(&self, cell: usize) -> Vec<Gap> {
        let mut at = cell;
        for t in self.tables {
            let n = slots(t);
            if at < n {
                let Ok(raw) = u32::try_from(at) else {
                    phx_num::capacity_exceeded!("slots of a cell table", u32::MAX, at);
                };
                let slot = Slot::new(raw);
                return if t.is_live(slot) { representation(t, slot) } else { Vec::new() };
            }
            at -= n;
        }
        phx_num::violation!(clause = "N1", "a cell read past the cells kept", index = cell);
    }
}

/// Every cell's weight against its profile counts, a slice of the cells a day.
#[derive(Debug)]
pub struct Representation;

impl AuditFamily for Representation {
    fn decl(&self) -> FamilyDecl {
        REPRESENTATION
    }

    fn check(&self, ctx: &FamilyCtx<'_>, findings: &mut Findings) -> u64 {
        let cells = ctx.cells();
        let span = ctx.rolling(cells.cells());
        for i in span.iter() {
            for g in cells.representation(i) {
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
        }
        phx_rand::float::len_u64(span.end - span.start)
    }

    /// A member taken off one profile value and nowhere added: the group counts one fewer than the weight.
    fn inject(&self, target: &mut dyn InjectTarget) -> Result<(), String> {
        let Some(tables) = target.cells().downcast_mut::<Vec<CellTable>>() else {
            return Err("the save's cells are not the population's tables".to_owned());
        };
        for t in tables {
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
    use crate::key::KeyId;
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
            entry(PopItem::Role(RoleDecl { name: "adult", clause: "REP.26" })),
            entry(PopItem::Role(RoleDecl { name: "child", clause: "REP.26" })),
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
        p.add(&layout, 1, 0, 10);
        let new = NewCell {
            party: PartyId::new(8),
            created: Day::new(1),
            weight: Weight::new(10),
            key: KeyId::new(0),
            positions: &[],
            profile: &p,
        };
        let s = t.add(&mut space, new, &kind, &[]);
        assert!(representation(&t, s).is_empty());
        p.remove(1, 0, 1);
        t.set_profile(s, &p);
        let gaps = representation(&t, s);
        assert_eq!(gaps.iter().map(|g| g.size).collect::<Vec<_>>(), [-1], "one child uncounted");
        let view = super::CellsView::new(std::slice::from_ref(&t));
        assert_eq!((phx_core::CellsAudit::cells(&view), phx_core::CellsAudit::representation(&view, 0).len()), (1, 1));
    }
}
