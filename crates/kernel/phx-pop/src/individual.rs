use phx_id::{Slot, TableId};
use phx_macros::clause;
use phx_num::violation;
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
        Extension { owner: table.column(space), lots: table.column(space), named_units: table.column(space), table }
    }

    /// An extension row for the individual at `owner`, its lists empty.
    pub fn add(&mut self, owner: Slot) -> Slot {
        let slot = self.table.slots.alloc();
        put(&mut self.owner, slot, owner.get());
        put(&mut self.lots, slot, ListRef::EMPTY);
        put(&mut self.named_units, slot, ListRef::EMPTY);
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

    /// The live extension rows, in slot order.
    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.table.slots.live_slots()
    }
}
