use phx_core::facts::FactDecl;
use phx_core::schema::{FactColumn, TableSchema};
use phx_core::{ListKind, RunHead};
use phx_id::{LineId, PartyId, Slot, TableId};
use phx_ledger::holder::{CellHolders, HolderArenas, HolderTable};
use phx_ledger::pooled::Kink;
use phx_ledger::positions::PayerPositions;
use phx_ledger::rows::RowView;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing};

use crate::table::{AgentList, AgentTable};

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a holder's list", u32::MAX, n);
    };
    w
}

fn list_of(list: ListKind) -> AgentList {
    match list {
        ListKind::RelationshipRows => AgentList::Rows,
        ListKind::Holdings => AgentList::Holdings,
        ListKind::Lots => AgentList::Lots,
        ListKind::NamedUnits => AgentList::NamedUnits,
    }
}

/// The ledger reaches an agent's rows, holdings, lots and named units in its row's chunk arena, as it reaches an
/// individual's in a kind table.
#[clause("REP.3", "REG.4")]
impl<B: Backing> HolderArenas for AgentTable<B> {
    fn at_average_cost(&self) -> bool {
        true
    }

    fn table(&self) -> TableId {
        self.id()
    }

    fn kind(&self) -> &'static str {
        AgentTable::kind(self)
    }

    fn party(&self, holder: Slot) -> PartyId {
        AgentTable::party(self, holder)
    }

    fn read(&self, holder: Slot, list: ListKind) -> &[u64] {
        self.words(holder, list_of(list))
    }

    fn append(&mut self, holder: Slot, list: ListKind, words: &[u64]) {
        self.edit_list(holder, list_of(list), |arena, r| arena.append(r, words));
    }

    fn overwrite(&mut self, holder: Slot, list: ListKind, at: usize, words: &[u64]) {
        self.edit_list(holder, list_of(list), |arena, r| {
            let Some(target) = arena.read_mut(*r).get_mut(at..at + words.len()) else {
                violation!(clause = "REG.14", "words written beyond a holder's list", at = at, len = words.len());
            };
            target.copy_from_slice(words);
        });
    }

    fn overwrite_words(&mut self, pool: Option<&phx_exec::Pool>, list: ListKind, writes: &mut [(Slot, usize, u64)]) {
        AgentTable::overwrite_words(self, pool, list_of(list), writes);
    }

    fn remove(&mut self, holder: Slot, list: ListKind, at: usize, count: usize) {
        if count > 0 {
            self.edit_list(holder, list_of(list), |arena, r| arena.remove(r, word32(at), word32(count)));
        }
    }

    fn run_head(&self, holder: Slot) -> RunHead {
        AgentTable::run_head(self, holder)
    }

    fn set_run_head(&mut self, holder: Slot, head: RunHead) {
        AgentTable::set_run_head(self, holder, head);
    }
}

/// What the settlement stream and the pooled-flow rule read of an agent: its multiplicity, one twin's share of an
/// account, and the kinks its twins may cross, of which it has none yet.
#[clause("REP.9", "REP.1")]
impl<B: Backing> PayerPositions for AgentTable<B> {
    fn weight(&self, holder: Slot) -> u32 {
        self.multiplicity(holder).get()
    }

    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let (available, members) = phx_ledger::positions::account_funds(self, holder, account);
        available / members + i128::from(facility_per_member)
    }

    fn funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let (available, members) = phx_ledger::positions::account_funds(self, holder, account);
        available + i128::from(facility_per_member) * members
    }

    fn kinks_into(&self, _: Slot, _: &mut Vec<Kink>) {}

    fn standing_rate(&self, _: Slot, _: &RowView) -> Missing<i64> {
        Missing::Absent
    }
}

impl<B: Backing> HolderTable for AgentTable<B> {
    fn live_words(&self) -> &[u64] {
        self.live_words()
    }
}

/// The books keep a population's table beside the kind tables, saving, reading back and hashing it with theirs.
impl<B: Backing + core::fmt::Debug + 'static> CellHolders for AgentTable<B>
where
    AgentTable<B>: Send + Sync,
{
    fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        phx_store::Saved::save(self, w);
    }

    fn load_like(&self, r: &mut phx_store::Reader<'_>) -> Result<Box<dyn CellHolders>, phx_store::LoadError> {
        let t: AgentTable<B> = phx_store::Saved::load(r)?;
        Ok(Box::new(t))
    }

    fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        phx_store::hash_saved(self, h);
    }

    fn as_any(&self) -> &dyn core::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn core::any::Any {
        self
    }

    fn compact_due(&mut self) -> u64 {
        AgentTable::compact_due(self)
    }
}

/// A fact of a population kind is a column every agent of the kind holds.
#[clause("REP.20")]
impl<B: Backing> TableSchema for AgentTable<B> {
    fn add_fact(
        &mut self,
        space: &mut AddressSpace,
        name: &'static str,
        fact: &FactDecl,
    ) -> Result<FactColumn, String> {
        if !fact.kinds.contains(&AgentTable::kind(self)) {
            return Err(format!("`{name}` is not a fact of `{}`", AgentTable::kind(self)));
        }
        AgentTable::add_fact(self, space)
    }
}

#[cfg(test)]
mod tests {
    use phx_core::{AttrDecl, ListKind, PopEntry, PopItem, RunHead, Weight};
    use phx_id::{Day, PartyId, TableId};
    use phx_ledger::holder::HolderArenas;
    use phx_ledger::positions::PayerPositions;
    use phx_store::{AddressSpace, HeapBacking};

    use crate::kind::PopKindDecl;
    use crate::table::{AgentTable, NewAgent};

    #[test]
    fn the_ledger_reaches_agents_as_it_reaches_individuals() {
        let region = PopEntry {
            system: "DEM",
            kind: "household",
            item: PopItem::Attr(AttrDecl { name: "region", values: 25, clause: "x" }),
        };
        let k = PopKindDecl::compile("household", &[region]).unwrap();
        let mut space = AddressSpace::empty();
        let mut t: AgentTable<HeapBacking> = AgentTable::new(&mut space, &k, TableId::new(4), 64, 8);
        let new = |party, twins| NewAgent {
            party: PartyId::new(party),
            created: Day::new(1),
            multiplicity: Weight::new(twins),
            attrs: &[3],
        };
        let agent = t.add(&mut space, new(5, 20));
        let other = t.add(&mut space, new(6, 1));
        let h: &mut dyn HolderArenas = &mut t;
        h.append(agent, ListKind::RelationshipRows, &[1, 2, 3, 4]);
        h.overwrite(agent, ListKind::RelationshipRows, 1, &[9]);
        h.insert(agent, ListKind::RelationshipRows, 0, &[7]);
        h.remove(agent, ListKind::RelationshipRows, 3, 1);
        assert_eq!(h.read(agent, ListKind::RelationshipRows), [7, 1, 9, 4]);
        h.append(other, ListKind::Lots, &[11]);
        assert_eq!((h.read(other, ListKind::Lots), h.read(agent, ListKind::Lots)), (&[11][..], &[][..]));
        h.set_run_head(agent, RunHead { next_due: 40, offset: 1, len: 2 });
        assert_eq!(h.run_head(agent), RunHead { next_due: 40, offset: 1, len: 2 });
        let wide = RunHead { next_due: 40, offset: 0, len: u32::from(u16::MAX) + 1 };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.set_run_head(agent, wide))).is_err());
        assert_eq!((h.party(agent), h.kind()), (PartyId::new(5), "household"));
        assert_eq!((t.weight(agent), t.weight(other), t.attr(agent, 0)), (20, 1, 3));
    }

    #[test]
    fn an_agent_of_no_twins_is_refused() {
        let k = PopKindDecl::compile("household", &[]).unwrap();
        let mut space = AddressSpace::empty();
        let mut t: AgentTable<HeapBacking> = AgentTable::new(&mut space, &k, TableId::new(4), 64, 8);
        let none = NewAgent { party: PartyId::new(1), created: Day::new(1), multiplicity: Weight::new(0), attrs: &[] };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.add(&mut space, none))).is_err());
    }
}
