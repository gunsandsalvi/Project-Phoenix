use phx_core::facts::{FactDecl, ReprClass};
use phx_core::schema::{FactColumn, TableSchema};
use phx_core::{ListKind, RunHead};
use phx_id::{LineId, PartyId, Slot, TableId};
use phx_ledger::algebra::Side;
use phx_ledger::holder::{CellHolders, HolderArenas, HolderTable};
use phx_ledger::pooled::Kink;
use phx_ledger::positions::PayerPositions;
use phx_ledger::rows::{self, RowView};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, ChunkArena, ListRef};

use crate::individual::ExtList;
use crate::table::{CellList, CellTable};

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a holder's list", u32::MAX, n);
    };
    w
}

/// Where the ledger's list of a holder lies in a cell table: a cell's own lists, or an individual's extension.
enum Place {
    Cell(CellList),
    Ext(ExtList),
}

fn place(list: ListKind) -> Place {
    match list {
        ListKind::RelationshipRows => Place::Cell(CellList::Rows),
        ListKind::Holdings => Place::Cell(CellList::Holdings),
        ListKind::Lots => Place::Ext(ExtList::Lots),
        ListKind::NamedUnits => Place::Ext(ExtList::NamedUnits),
    }
}

impl<B: Backing> CellTable<B> {
    fn edit_holder<R>(
        &mut self,
        holder: Slot,
        list: ListKind,
        f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef) -> R,
    ) -> R {
        match place(list) {
            Place::Cell(l) => self.edit_list(holder, l, f),
            Place::Ext(l) => self.edit_ext(holder, l, f),
        }
    }
}

/// The ledger reaches a cell's rows and holdings, and an individual's lots and named units, in its row's chunk arena,
/// as it reaches an individual's in a kind table.
#[clause("REP.3", "REG.4")]
impl<B: Backing> HolderArenas for CellTable<B> {
    fn table(&self) -> TableId {
        self.id()
    }

    fn kind(&self) -> &'static str {
        CellTable::kind(self)
    }

    fn party(&self, holder: Slot) -> PartyId {
        CellTable::party(self, holder)
    }

    fn read(&self, holder: Slot, list: ListKind) -> &[u64] {
        match place(list) {
            Place::Cell(l) => self.words(holder, l),
            Place::Ext(l) => self.ext_words(holder, l),
        }
    }

    fn append(&mut self, holder: Slot, list: ListKind, words: &[u64]) {
        self.edit_holder(holder, list, |arena, r| arena.append(r, words));
    }

    fn overwrite(&mut self, holder: Slot, list: ListKind, at: usize, words: &[u64]) {
        self.edit_holder(holder, list, |arena, r| {
            let Some(target) = arena.read_mut(*r).get_mut(at..at + words.len()) else {
                violation!(clause = "REG.14", "words written beyond a holder's list", at = at, len = words.len());
            };
            target.copy_from_slice(words);
        });
    }

    fn remove(&mut self, holder: Slot, list: ListKind, at: usize, count: usize) {
        if count > 0 {
            self.edit_holder(holder, list, |arena, r| arena.remove(r, word32(at), word32(count)));
        }
    }

    fn run_head(&self, holder: Slot) -> RunHead {
        CellTable::run_head(self, holder)
    }

    fn set_run_head(&mut self, holder: Slot, head: RunHead) {
        CellTable::set_run_head(self, holder, head);
    }
}

/// What the settlement stream and the pooled-flow rule read of a cell: its weight, a member's share of an account,
/// and the kinks its members may cross.
#[clause("REP.8", "REP.9")]
impl<B: Backing> PayerPositions for CellTable<B> {
    fn weight(&self, holder: Slot) -> u32 {
        CellTable::weight(self, holder).get()
    }

    fn per_member_funds(&self, holder: Slot, account: LineId, facility_per_member: i64) -> i128 {
        let Some(view) = rows::iter(self, holder).find(|r| r.row.line == account && r.side() == Side::Asset) else {
            violation!(clause = "MON.5", "funds read on an account its holder does not hold", line = account.get());
        };
        let Missing::Present(balance) = view.optional.balance else {
            violation!(clause = "MON.5", "an account with no balance", line = account.get());
        };
        let pending = match view.optional.pending {
            Missing::Present(p) => i128::from(p),
            Missing::Absent => 0,
        };
        let members = i128::from(view.row.count);
        if members == 0 {
            violation!(clause = "REP.9", "an account held by no member", line = account.get());
        }
        (i128::from(balance) - pending) / members + i128::from(facility_per_member)
    }

    /// Every kink of the cell's key rules lies in its signature, whose points the rules owning them give; a cell
    /// whose kind has none has no kink to cross.
    fn kinks_into(&self, holder: Slot, _: &mut Vec<Kink>) {
        if !self.sig(holder).is_empty() {
            violation!(clause = "REP.16", "a kink on a cell whose points no rule has given", slot = holder.get());
        }
    }

    /// A cell's rows carry no rate of their own: its standing rates are the cell's, per member.
    fn standing_rate(&self, _: Slot, _: &RowView) -> Missing<i64> {
        Missing::Absent
    }
}

impl<B: Backing> HolderTable for CellTable<B> {
    fn live_words(&self) -> &[u64] {
        self.live_words()
    }
}

/// The books keep a population's table beside the kind tables, saving, reading back and hashing it with theirs.
impl<B: Backing + core::fmt::Debug + 'static> CellHolders for CellTable<B>
where
    CellTable<B>: Send + Sync,
{
    fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        phx_store::Saved::save(self, w);
    }

    fn load_like(&self, r: &mut phx_store::Reader<'_>) -> Result<Box<dyn CellHolders>, phx_store::LoadError> {
        let t: CellTable<B> = phx_store::Saved::load(r)?;
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
        CellTable::compact_due(self)
    }
}

/// Facts about a population kind are carried as the kind declares its attributes: in the key, as positions, in
/// profiles, each through the kind's own items. Only a fact carried on individuals alone is a column, kept in their
/// extension.
#[clause("REP.33")]
impl<B: Backing> TableSchema for CellTable<B> {
    fn add_fact(
        &mut self,
        space: &mut AddressSpace,
        name: &'static str,
        fact: &FactDecl,
    ) -> Result<FactColumn, String> {
        if !fact.kinds.contains(&CellTable::kind(self)) {
            return Err(format!("`{name}` is not a fact of `{}`", CellTable::kind(self)));
        }
        match fact.repr {
            ReprClass::Individual => self.add_individual_fact(space, name),
            ReprClass::Key | ReprClass::Position | ReprClass::Profile => Err(format!(
                "`{name}` is carried in the key, a position or a profile of `{}`, which the kind declares as its own item",
                CellTable::kind(self)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::facts::{Audience, FactDecl, FactType, ReprClass};
    use phx_core::register::values::Partition;
    use phx_core::schema::TableSchema;
    use phx_core::{KinkRegistry, ListKind, PopEntry, PopItem, RoleDecl, RunHead, Weight};
    use phx_id::{Day, PartyId, TableId};
    use phx_ledger::holder::HolderArenas;
    use phx_num::Missing;
    use phx_store::{AddressSpace, HeapBacking};

    use crate::key::KeyId;
    use crate::kind::PopKindDecl;
    use crate::profile::Profile;
    use crate::steps::StepTable;
    use crate::table::{CellTable, NewCell};

    fn steps(_: &'static str) -> Result<StepTable, String> {
        StepTable::new(&Partition { exp: 0, bounds: [1].into() })
    }

    #[test]
    fn the_ledger_reaches_cells_as_it_reaches_individuals() {
        let role = PopEntry {
            system: "DEM",
            kind: "household",
            item: PopItem::Role(RoleDecl { name: "a", per_member: phx_core::RoleCount::One, clause: "x" }),
        };
        let k = PopKindDecl::compile("household", &[role], &KinkRegistry::default(), &steps).unwrap();
        let mut space = AddressSpace::empty();
        let mut t: CellTable<HeapBacking> = CellTable::new(&mut space, &k, TableId::new(4), 64, 8);
        let profile = Profile::empty(t.profile_layout());
        let new = |party, weight| NewCell {
            party: PartyId::new(party),
            created: Day::new(1),
            weight: Weight::new(weight),
            key: KeyId::new(0),
            positions: &[],
            profile: &profile,
        };
        let cell = t.add(&mut space, new(5, 30), &k, &[]);
        let person = t.add(&mut space, new(6, 1), &k, &[]);
        t.make_individual(person);
        let h: &mut dyn HolderArenas = &mut t;
        h.append(cell, ListKind::RelationshipRows, &[1, 2, 3, 4]);
        h.overwrite(cell, ListKind::RelationshipRows, 1, &[9]);
        h.insert(cell, ListKind::RelationshipRows, 0, &[7]);
        h.remove(cell, ListKind::RelationshipRows, 3, 1);
        assert_eq!(h.read(cell, ListKind::RelationshipRows), [7, 1, 9, 4]);
        h.append(person, ListKind::Lots, &[11]);
        assert_eq!((h.read(person, ListKind::Lots), h.read(cell, ListKind::Lots)), (&[11][..], &[][..]));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.append(cell, ListKind::Lots, &[1]))).is_err()
        );
        h.set_run_head(cell, RunHead { next_due: 40, offset: 1, len: 2 });
        assert_eq!(h.run_head(cell), RunHead { next_due: 40, offset: 1, len: 2 });
        let wide = RunHead { next_due: 40, offset: 0, len: u32::from(u16::MAX) + 1 };
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| h.set_run_head(cell, wide))).is_err());
        assert_eq!((h.party(cell), h.kind()), (PartyId::new(5), "household"));

        let fact = |repr| FactDecl {
            value: FactType::Money,
            unit: Missing::Present("home currency"),
            kinds: &["household"],
            audience: Audience::Public,
            repr,
        };
        assert!(
            t.add_fact(&mut space, "HH.cash", &fact(ReprClass::Position)).is_err(),
            "a position is the kind's item"
        );
        let col = t.add_fact(&mut space, "HH.rating", &fact(ReprClass::Individual)).unwrap();
        assert_eq!(t.fact(person, col), Missing::Absent);
        t.write_fact(person, col, 3);
        assert_eq!(t.fact(person, col), Missing::Present(3));
    }
}
