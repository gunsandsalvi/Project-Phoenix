use phx_id::{Day, PartyId, Slot, TableId, TileId};
use phx_macros::clause;
use phx_num::consts::ABSENT_I64;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::consts::INDIVIDUAL_ARENA_WORDS;
use phx_store::{AddressSpace, Backing, ChunkArena, Column, ListRef, Region, SystemBacking, Table};

use crate::facts::FactDecl;
use crate::register::values::TypeId;
use crate::schema::{FactColumn, TableSchema};

/// A fact kept as a column of a kind's table of individuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FacetDecl {
    pub fact: &'static str,
    pub kind: &'static str,
}

/// The lists every individual keeps in its chunk's arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListKind {
    RelationshipRows,
    Holdings,
    Lots,
    NamedUnits,
}

impl ListKind {
    /// Every list, in the order a row keeps them.
    pub const ALL: [ListKind; 4] =
        [ListKind::RelationshipRows, ListKind::Holdings, ListKind::Lots, ListKind::NamedUnits];
}

/// Each row's reference to each of its lists.
#[derive(Debug, phx_macros::Saved)]
struct Lists<B: Backing> {
    relationship_rows: Column<ListRef, B>,
    holdings: Column<ListRef, B>,
    lots: Column<ListRef, B>,
    named_units: Column<ListRef, B>,
}

impl<B: Backing> Lists<B> {
    fn of(&self, kind: ListKind) -> &Column<ListRef, B> {
        match kind {
            ListKind::RelationshipRows => &self.relationship_rows,
            ListKind::Holdings => &self.holdings,
            ListKind::Lots => &self.lots,
            ListKind::NamedUnits => &self.named_units,
        }
    }

    fn all_mut(&mut self) -> [&mut Column<ListRef, B>; 4] {
        [&mut self.relationship_rows, &mut self.holdings, &mut self.lots, &mut self.named_units]
    }

    fn all(&self) -> [&Column<ListRef, B>; 4] {
        [&self.relationship_rows, &self.holdings, &self.lots, &self.named_units]
    }
}

/// A fact stored for every row, absent until its writer writes it.
#[derive(Debug, phx_macros::Saved)]
struct Facet<B: Backing> {
    name: &'static str,
    values: Column<i64, B>,
}

/// The table of one kind of individual: its base columns, the lists each row keeps in its chunk's arena, and a column
/// per fact the kind's systems claim. Country, region and currency are read through the site, and whether a party has
/// ended through the directory, so neither is stored here.
#[clause("PTY.1", "PTY.6")]
#[derive(Debug, phx_macros::Saved)]
pub struct KindTable<B: Backing = SystemBacking> {
    kind: &'static str,
    table: Table<B>,
    party: Column<u64, B>,
    site: Column<u32, B>,
    created: Column<u32, B>,
    weight: Column<u32, B>,
    types: Vec<Column<u16, B>>,
    lists: Lists<B>,
    runs: Column<RunHead, B>,
    arenas: Vec<ChunkArena<B>>,
    facets: Vec<Facet<B>>,
}

/// The head of a holder's due-day run: the earliest day any row of its dated segment can fall due, and where the
/// segment lies in its relationship rows, in words. A row leaving the segment leaves the day as it was, an early
/// bound that costs one scan; an empty segment is never due.
#[clause("REP.3")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct RunHead {
    pub next_due: u32,
    pub offset: u32,
    pub len: u32,
}

impl RunHead {
    pub const EMPTY: RunHead = RunHead { next_due: 0, offset: 0, len: 0 };

    /// Whether a holder's run is scanned on a day: its segment holds rows and its head has come.
    #[must_use]
    pub fn due(self, day: Day) -> bool {
        self.len > 0 && self.next_due <= day.get()
    }
}

/// A new row's fixed columns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewIndividual<'a> {
    pub party: PartyId,
    pub site: TileId,
    pub created: Day,
    /// The real parties the row stands for: one, but for an agent's estate, one for each of its twins.
    pub weight: u32,
    pub types: &'a [TypeId],
}

fn words(n: u32) -> usize {
    let Ok(n) = usize::try_from(n) else { capacity_exceeded!("index width", usize::MAX, n) };
    n
}

fn at(slot: Slot) -> usize {
    let Ok(i) = usize::try_from(slot.get()) else {
        capacity_exceeded!("index width", usize::MAX, slot.get());
    };
    i
}

/// Writes a row's value, appending when the slot is the column's next.
fn put<T: phx_store::Pod, B: Backing>(column: &mut Column<T, B>, slot: Slot, value: T) {
    if at(slot) == column.len() {
        column.push(value);
    } else {
        column.set(slot, value);
    }
}

impl<B: Backing> KindTable<B> {
    /// An empty table for a kind, of at most `max_rows`, whose parties carry one type per declared dimension.
    pub fn new(
        space: &mut AddressSpace,
        kind: &'static str,
        id: TableId,
        max_rows: u32,
        rows_per_chunk: u32,
        type_dimensions: usize,
    ) -> KindTable<B> {
        let table: Table<B> = Table::new(space, id, max_rows, rows_per_chunk);
        KindTable {
            kind,
            party: table.column(space),
            site: table.column(space),
            created: table.column(space),
            weight: table.column(space),
            types: (0..type_dimensions).map(|_| table.column(space)).collect(),
            lists: Lists {
                relationship_rows: table.column(space),
                holdings: table.column(space),
                lots: table.column(space),
                named_units: table.column(space),
            },
            runs: table.column(space),
            arenas: Vec::new(),
            facets: Vec::new(),
            table,
        }
    }

    pub fn id(&self) -> TableId {
        self.table.id
    }

    /// A row for a new individual; its facts are absent until written.
    pub fn add(&mut self, space: &mut AddressSpace, row: NewIndividual<'_>) -> Slot {
        if row.types.len() != self.types.len() {
            violation!(
                clause = "NUM.4",
                "an individual with another count of types than its kind",
                given = row.types.len()
            );
        }
        if row.weight == 0 {
            violation!(clause = "REP.17", "an individual standing for no party", party = row.party.get());
        }
        let slot = self.table.slots.alloc();
        put(&mut self.party, slot, row.party.get());
        put(&mut self.site, slot, row.site.get());
        put(&mut self.created, slot, row.created.get());
        put(&mut self.weight, slot, row.weight);
        for (column, t) in self.types.iter_mut().zip(row.types) {
            put(column, slot, t.get());
        }
        for column in self.lists.all_mut() {
            put(column, slot, ListRef::EMPTY);
        }
        put(&mut self.runs, slot, RunHead::EMPTY);
        for facet in &mut self.facets {
            put(&mut facet.values, slot, ABSENT_I64);
        }
        let chunk = at(slot) / at(Slot::new(self.table.rows_per_chunk()));
        while self.arenas.len() <= chunk {
            self.arenas.push(ChunkArena::new(space, INDIVIDUAL_ARENA_WORDS));
        }
        slot
    }

    /// Frees a row whose party has ended; its slot is handed out again after the day closes.
    pub fn remove(&mut self, slot: Slot) {
        self.live(slot);
        for column in self.lists.all() {
            if column.get(slot).is_some_and(|r| r.len != 0) {
                violation!(clause = "PTY.10", "an individual removed with lists still in its arena", slot = slot.get());
            }
        }
        // Its lists' room is dead from now, so the next compaction accounts for every word.
        for kind in ListKind::ALL {
            if self.list(slot, kind).cap != 0 {
                self.edit_list(slot, kind, ChunkArena::clear);
            }
        }
        self.table.slots.release(slot);
    }

    /// Closes the gaps in every chunk's arena whose dead words have passed the declared share: each live row's lists
    /// in slot order, each reference rewritten where it now lies. Returns the chunks compacted.
    pub fn compact_due(&mut self) -> u64 {
        let per = at(Slot::new(self.table.rows_per_chunk()));
        let due: Vec<usize> =
            (0..self.arenas.len()).filter(|c| self.arenas.get(*c).is_some_and(ChunkArena::needs_compaction)).collect();
        let mut done = 0;
        for chunk in due {
            let slots: Vec<Slot> = self.slots().filter(|s| at(*s) / per == chunk).collect();
            let mut refs: Vec<ListRef> = slots.iter().flat_map(|s| ListKind::ALL.map(|k| self.list(*s, k))).collect();
            if let Some(arena) = self.arenas.get_mut(chunk) {
                // A transient scratch outside the world's reservations, as large as the arena, unmapped as the
                // compaction ends.
                let mut scratch: Region<u64, B> =
                    Region::reserve(&mut AddressSpace::empty(), words(arena.used_words()));
                arena.compact(refs.as_mut_slice(), &mut scratch);
            }
            let mut moved = refs.into_iter();
            for s in &slots {
                for k in ListKind::ALL {
                    let Some(r) = moved.next() else {
                        violation!(clause = "SET.12", "a compaction that returned fewer lists than it took");
                    };
                    self.list_column(k).set(*s, r);
                }
            }
            done += 1;
        }
        done
    }

    fn list_column(&mut self, kind: ListKind) -> &mut Column<ListRef, B> {
        match kind {
            ListKind::RelationshipRows => &mut self.lists.relationship_rows,
            ListKind::Holdings => &mut self.lists.holdings,
            ListKind::Lots => &mut self.lists.lots,
            ListKind::NamedUnits => &mut self.lists.named_units,
        }
    }

    fn live(&self, slot: Slot) {
        if !self.table.slots.is_live(slot) {
            violation!(clause = "PTY.10", "a read of a row no party holds", slot = slot.get());
        }
    }

    /// The rows parties hold, in slot order.
    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.table.slots.live_slots()
    }

    /// The live bits of its rows, a row to a bit.
    #[must_use]
    pub fn live_words(&self) -> &[u64] {
        self.table.slots.live_words()
    }

    pub fn party(&self, slot: Slot) -> PartyId {
        self.live(slot);
        let Some(raw) = self.party.get(slot) else {
            violation!(clause = "PTY.10", "a read of a row no party holds", slot = slot.get());
        };
        PartyId::new(raw)
    }

    /// The real parties a row stands for.
    pub fn weight_at(&self, slot: Slot) -> u32 {
        self.weight.get(slot).unwrap_or_else(|| missing_row(slot))
    }

    pub fn site(&self, slot: Slot) -> TileId {
        self.live(slot);
        TileId::new(self.site.get(slot).unwrap_or_else(|| missing_row(slot)))
    }

    pub fn created(&self, slot: Slot) -> Day {
        self.live(slot);
        Day::new(self.created.get(slot).unwrap_or_else(|| missing_row(slot)))
    }

    pub fn list(&self, slot: Slot, kind: ListKind) -> ListRef {
        self.live(slot);
        self.lists.of(kind).get(slot).unwrap_or_else(|| missing_row(slot))
    }

    /// A row's due-day run head.
    pub fn run_head(&self, slot: Slot) -> RunHead {
        self.live(slot);
        self.runs.get(slot).unwrap_or_else(|| missing_row(slot))
    }

    pub fn set_run_head(&mut self, slot: Slot, head: RunHead) {
        self.live(slot);
        self.runs.set(slot, head);
    }

    /// A row's list, as the words its arena holds.
    #[must_use]
    pub fn words(&self, slot: Slot, kind: ListKind) -> &[u64] {
        let list = self.list(slot, kind);
        let chunk = at(slot) / at(Slot::new(self.table.rows_per_chunk()));
        match self.arenas.get(chunk) {
            Some(arena) => arena.read(list),
            None => &[],
        }
    }

    /// Edits a row's list in its arena, writing back its reference, which an edit may move.
    pub fn edit_list<R>(
        &mut self,
        slot: Slot,
        kind: ListKind,
        f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef) -> R,
    ) -> R {
        let mut list = self.list(slot, kind);
        let out = f(self.arena_mut(slot), &mut list);
        self.list_column(kind).set(slot, list);
        out
    }

    /// The kind of individual the table holds.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    /// The arena of the chunk a row lies in, where its lists live.
    pub fn arena_mut(&mut self, slot: Slot) -> &mut ChunkArena<B> {
        let chunk = at(slot) / at(Slot::new(self.table.rows_per_chunk()));
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "a row beyond the table's chunks", slot = slot.get());
        };
        arena
    }

    pub fn fact(&self, slot: Slot, column: FactColumn) -> Missing<i64> {
        self.live(slot);
        let facet = self.facet(column);
        let raw = facet.values.get(slot).unwrap_or_else(|| missing_row(slot));
        if raw == ABSENT_I64 { Missing::Absent } else { Missing::Present(raw) }
    }

    pub fn write_fact(&mut self, slot: Slot, column: FactColumn, value: i64) {
        self.live(slot);
        if value == ABSENT_I64 {
            violation!(clause = "NUM.8", "the absent marker written as a fact");
        }
        self.facet_mut(column).values.set(slot, value);
    }

    fn facet(&self, column: FactColumn) -> &Facet<B> {
        let Some(f) = usize::try_from(column.0).ok().and_then(|i| self.facets.get(i)) else {
            violation!(clause = "Law 4", "a fact column the table does not have", column = column.0);
        };
        f
    }

    fn facet_mut(&mut self, column: FactColumn) -> &mut Facet<B> {
        let Some(f) = usize::try_from(column.0).ok().and_then(|i| self.facets.get_mut(i)) else {
            violation!(clause = "Law 4", "a fact column the table does not have", column = column.0);
        };
        f
    }
}

fn missing_row<T>(slot: Slot) -> T {
    violation!(clause = "PTY.10", "a read of a row no party holds", slot = slot.get());
}

impl<B: Backing> TableSchema for KindTable<B> {
    fn add_fact(
        &mut self,
        space: &mut AddressSpace,
        name: &'static str,
        fact: &FactDecl,
    ) -> Result<FactColumn, String> {
        if !fact.kinds.contains(&self.kind) {
            return Err(format!("`{name}` is not a fact of `{}`", self.kind));
        }
        if self.facets.iter().any(|f| f.name == name) {
            return Err(format!("`{name}` has a column already"));
        }
        let mut values: Column<i64, B> = self.table.column(space);
        let rows = vec![ABSENT_I64; self.party.len()];
        values.extend(&rows);
        let Ok(index) = u32::try_from(self.facets.len()) else {
            capacity_exceeded!("fact columns", u32::MAX, self.facets.len());
        };
        self.facets.push(Facet { name, values });
        Ok(FactColumn(index))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, PartyId, Slot, TableId, TileId};
    use phx_num::Missing;
    use phx_store::{AddressSpace, HeapBacking};

    use super::{KindTable, ListKind, NewIndividual};
    use crate::facts::{Audience, FactDecl, FactType, ReprClass};
    use crate::register::values::TypeId;
    use crate::schema::TableSchema;

    const TYPES: &[TypeId] = &[TypeId::new(0)];

    const CAPITAL: FactDecl = FactDecl {
        value: FactType::Money,
        unit: Missing::Present("home currency"),
        kinds: &["bank"],
        audience: Audience::Authority("supervisor"),
        repr: ReprClass::Individual,
    };

    #[test]
    fn kind_table_rows_and_facts() {
        let mut space = AddressSpace::empty();
        let mut banks: KindTable<HeapBacking<4096>> = KindTable::new(&mut space, "bank", TableId::new(3), 64, 8, 1);
        let row = |p| NewIndividual {
            party: PartyId::new(p),
            site: TileId::new(p.try_into().unwrap()),
            created: Day::new(1),
            weight: 1,
            types: TYPES,
        };
        let a = banks.add(&mut space, row(10));
        let capital = banks.add_fact(&mut space, "BNK.capital", &CAPITAL).unwrap();
        let b = banks.add(&mut space, row(11));
        assert_eq!((banks.fact(a, capital), banks.fact(b, capital)), (Missing::Absent, Missing::Absent));
        banks.write_fact(b, capital, 500);
        assert_eq!(banks.fact(b, capital), Missing::Present(500));
        assert_eq!(
            (banks.party(b), banks.site(a), banks.list(a, ListKind::Holdings).len),
            (PartyId::new(11), TileId::new(10), 0)
        );
        assert!(banks.add_fact(&mut space, "BNK.capital", &CAPITAL).is_err(), "one column per fact");
        let firm_fact = FactDecl { kinds: &["large_firm"], ..CAPITAL };
        assert!(banks.add_fact(&mut space, "FRM.orders", &firm_fact).is_err(), "another kind's fact");
        banks.remove(a);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| banks.party(a))).is_err());
        assert_eq!(banks.id(), TableId::new(3));
        assert_eq!(banks.created(Slot::new(1)), Day::new(1));
        assert_eq!(banks.weight_at(b), 1, "an individual stands for one party");
        let estate = banks.add(&mut space, NewIndividual { weight: 170, ..row(12) });
        assert_eq!(banks.weight_at(estate), 170, "an agent's estate stands for each of its twins");
        let none = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut s = AddressSpace::empty();
            let mut t: KindTable<HeapBacking<4096>> = KindTable::new(&mut s, "estate", TableId::new(4), 8, 8, 1);
            t.add(&mut s, NewIndividual { weight: 0, ..row(13) })
        }));
        assert!(none.is_err(), "a row standing for no party is refused");
    }

    #[test]
    fn compaction_keeps_every_list_and_frees_the_dead() {
        let mut space = AddressSpace::empty();
        let mut t: KindTable<HeapBacking<4096>> = KindTable::new(&mut space, "firm", TableId::new(0), 64, 64, 1);
        let row =
            |p| NewIndividual {
                party: PartyId::new(p),
                site: TileId::new(0),
                created: Day::new(1),
                weight: 1,
                types: TYPES,
            };
        let slots: Vec<Slot> = (1..5).map(|p| t.add(&mut space, row(p))).collect();
        let mut model: Vec<Vec<u64>> = vec![Vec::new(); slots.len()];
        for round in 0..40_u64 {
            for (i, s) in slots.iter().enumerate() {
                let w = round * 10 + u64::try_from(i).unwrap();
                t.edit_list(*s, ListKind::RelationshipRows, |a, r| a.append(r, &[w]));
                model[i].push(w);
            }
        }
        let gone = slots[1];
        t.edit_list(gone, ListKind::RelationshipRows, |a, r| a.remove(r, 0, r.len));
        model[1].clear();
        t.remove(gone);
        let arena = t.arena_mut(slots[0]);
        assert!(arena.needs_compaction(), "the relocations and the removed row left dead words");
        assert_eq!(t.compact_due(), 1);
        assert_eq!(t.arena_mut(slots[0]).dead_words(), 0);
        for (i, s) in slots.iter().enumerate().filter(|(_, s)| **s != gone) {
            assert_eq!(t.words(*s, ListKind::RelationshipRows), model[i].as_slice());
        }
        assert_eq!(t.compact_due(), 0, "nothing is due once compacted");
    }
}
