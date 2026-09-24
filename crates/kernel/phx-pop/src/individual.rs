use phx_core::schema::FactColumn;
use phx_id::{Slot, TableId};
use phx_macros::clause;
use phx_num::consts::ABSENT_I64;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, Column, ListRef, SystemBacking, Table};

/// State only an individual of a population kind keeps, beside its weight-one row: its lots and named units, whose
/// words lie in its row's chunk arena with its other lists. A promoted household or the player's firm is such a row.
#[clause("REP.2", "REP.29")]
#[derive(Debug, phx_macros::Saved)]
pub struct Extension<B: Backing = SystemBacking> {
    table: Table<B>,
    owner: Column<u32, B>,
    lots: Column<ListRef, B>,
    named_units: Column<ListRef, B>,
    facets: Vec<Facet<B>>,
}

/// A fact only individuals of the kind carry, a value per extension row, absent until its writer writes it.
#[derive(Debug, phx_macros::Saved)]
struct Facet<B: Backing> {
    name: &'static str,
    values: Column<i64, B>,
}

/// The lists an individual keeps apart from a cell's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtList {
    Lots,
    NamedUnits,
}

fn put<T: phx_store::Pod, B: Backing>(column: &mut Column<T, B>, slot: Slot, value: T) {
    if usize::try_from(slot.get()).is_ok_and(|i| i == column.len()) {
        column.push(value);
    } else {
        column.set(slot, value);
    }
}

impl<B: Backing> Extension<B> {
    pub fn new(space: &mut AddressSpace, id: TableId, max_rows: u32, rows_per_chunk: u32) -> Extension<B> {
        let table: Table<B> = Table::new(space, id, max_rows, rows_per_chunk);
        Extension {
            owner: table.column(space),
            lots: table.column(space),
            named_units: table.column(space),
            facets: Vec::new(),
            table,
        }
    }

    /// An extension row for the individual at `owner`, its lists empty.
    pub fn add(&mut self, owner: Slot) -> Slot {
        let slot = self.table.slots.alloc();
        put(&mut self.owner, slot, owner.get());
        put(&mut self.lots, slot, ListRef::EMPTY);
        put(&mut self.named_units, slot, ListRef::EMPTY);
        for f in &mut self.facets {
            put(&mut f.values, slot, ABSENT_I64);
        }
        slot
    }

    /// Frees an extension row whose lists are empty.
    pub fn remove(&mut self, ext: Slot) {
        if [ExtList::Lots, ExtList::NamedUnits].into_iter().any(|l| self.list(ext, l).len != 0) {
            violation!(clause = "PTY.10", "an individual's extension removed with lists in its arena", ext = ext.get());
        }
        self.table.slots.release(ext);
    }

    fn live(&self, ext: Slot) {
        if !self.table.slots.is_live(ext) {
            violation!(clause = "PTY.10", "an extension row no individual holds", ext = ext.get());
        }
    }

    /// The individual's row moved, as renumbering moves it.
    pub fn set_owner(&mut self, ext: Slot, owner: Slot) {
        self.live(ext);
        self.owner.set(ext, owner.get());
    }

    pub fn owner(&self, ext: Slot) -> Slot {
        self.live(ext);
        let Some(o) = self.owner.get(ext) else {
            violation!(clause = "PTY.10", "an extension row no individual holds", ext = ext.get());
        };
        Slot::new(o)
    }

    fn column(&self, list: ExtList) -> &Column<ListRef, B> {
        match list {
            ExtList::Lots => &self.lots,
            ExtList::NamedUnits => &self.named_units,
        }
    }

    pub fn list(&self, ext: Slot, list: ExtList) -> ListRef {
        self.live(ext);
        let Some(r) = self.column(list).get(ext) else {
            violation!(clause = "PTY.10", "an extension row no individual holds", ext = ext.get());
        };
        r
    }

    pub fn set_list(&mut self, ext: Slot, list: ExtList, r: ListRef) {
        self.live(ext);
        match list {
            ExtList::Lots => self.lots.set(ext, r),
            ExtList::NamedUnits => self.named_units.set(ext, r),
        }
    }

    /// A column for a fact only individuals carry.
    ///
    /// # Errors
    /// A fact that has a column already.
    pub fn add_facet(&mut self, space: &mut AddressSpace, name: &'static str) -> Result<FactColumn, String> {
        if self.facets.iter().any(|f| f.name == name) {
            return Err(format!("`{name}` has a column already"));
        }
        let mut values: Column<i64, B> = self.table.column(space);
        values.extend(&vec![ABSENT_I64; self.owner.len()]);
        let Ok(index) = u32::try_from(self.facets.len()) else {
            capacity_exceeded!("fact columns", u32::MAX, self.facets.len());
        };
        self.facets.push(Facet { name, values });
        Ok(FactColumn::new(index))
    }

    fn facet(&self, column: FactColumn) -> &Facet<B> {
        let Some(f) = usize::try_from(column.index()).ok().and_then(|i| self.facets.get(i)) else {
            violation!(clause = "Law 4", "a fact column the table does not have", column = column.index());
        };
        f
    }

    pub fn fact(&self, ext: Slot, column: FactColumn) -> Missing<i64> {
        self.live(ext);
        match self.facet(column).values.get(ext) {
            Some(ABSENT_I64) | None => Missing::Absent,
            Some(v) => Missing::Present(v),
        }
    }

    pub fn write_fact(&mut self, ext: Slot, column: FactColumn, value: i64) {
        self.live(ext);
        if value == ABSENT_I64 {
            violation!(clause = "NUM.8", "the absent marker written as a fact");
        }
        let Some(f) = usize::try_from(column.index()).ok().and_then(|i| self.facets.get_mut(i)) else {
            violation!(clause = "Law 4", "a fact column the table does not have", column = column.index());
        };
        f.values.set(ext, value);
    }

    /// The live extension rows, in slot order.
    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.table.slots.live_slots()
    }
}
