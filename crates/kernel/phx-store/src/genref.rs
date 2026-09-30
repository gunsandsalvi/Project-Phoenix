//! References that outlive a day: a slot and the generation it held when the reference was taken, refused once the
//! row has ended or its slot holds another.

use std::marker::PhantomData;

use phx_id::{ContractLink, Slot, TableRef};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::convert::to_usize;
use crate::stats::StoreStats;
use crate::table::{SlotAlloc, live_at};

/// A generation-checked reference to a row of the table `T` marks: its slot and the slot's generation when it took the
/// row.
#[derive(Debug)]
#[repr(C)]
pub struct GenRef<T> {
    slot: u32,
    generation: u32,
    table: PhantomData<fn() -> T>,
}

impl<T> Clone for GenRef<T> {
    fn clone(&self) -> GenRef<T> {
        *self
    }
}

impl<T> Copy for GenRef<T> {}

impl<T> PartialEq for GenRef<T> {
    fn eq(&self, other: &GenRef<T>) -> bool {
        (self.slot, self.generation) == (other.slot, other.generation)
    }
}

impl<T> Eq for GenRef<T> {}

/// A table whose rows other stores name by a typed reference of its own.
pub trait Referenced {
    type Ref: TableRef;
}

impl<T: Referenced> GenRef<T> {
    /// The reference as its table's own type, which no other table's reference converts to.
    #[must_use]
    pub fn typed(self) -> T::Ref {
        T::Ref::from_parts(Slot::new(self.slot), self.generation)
    }

    /// A typed reference to the table's row, as the table checks it.
    #[must_use]
    pub fn of(r: T::Ref) -> GenRef<T> {
        GenRef { slot: r.slot().get(), generation: r.generation(), table: PhantomData }
    }
}

impl<T> GenRef<T> {
    pub fn slot(self) -> Slot {
        Slot::new(self.slot)
    }

    #[must_use]
    pub fn generation(self) -> u32 {
        self.generation
    }
}

/// Each slot's generation in the table `T` marks, raised each time the slot takes a new row; a raise past the widest
/// generation the table's references carry stops the run.
#[clause("PTY.13")]
#[derive(Debug)]
pub struct Generations<T, B: Backing = SystemBacking> {
    of: Column<u32, B>,
    last: u32,
    table: PhantomData<fn() -> T>,
}

/// The generation after `g`, or none past `last`.
#[must_use]
pub fn next(g: u32, last: u32) -> Option<u32> {
    (g < last).then(|| g + 1)
}

impl<T, B: Backing> Generations<T, B> {
    /// The generations of at most `max` slots, chunked by `rows_per_chunk`, each carried in `bits` bits.
    pub fn new(space: &mut AddressSpace, max: u32, rows_per_chunk: u32, bits: u32) -> Generations<T, B> {
        if bits == 0 || bits > u32::BITS {
            violation!(clause = "SET.12", "a generation of no bits or more than a word", bits = bits);
        }
        Generations {
            of: Column::new(space, max, rows_per_chunk),
            last: u32::MAX >> (u32::BITS - bits),
            table: PhantomData,
        }
    }

    /// A slot taken from the table's slots for a new row, at its next generation; a slot's first row is its
    /// generation zero.
    #[clause("PTY.13", "SET.7")]
    pub fn alloc(&mut self, slots: &mut SlotAlloc<B>) -> GenRef<T> {
        let slot = slots.alloc();
        let generation = if to_usize(slot.get()) == self.of.len() {
            self.of.push(0);
            0
        } else {
            let Some(g) = self.of.get(slot) else {
                violation!(clause = "SET.12", "a slot handed out past its generations", slot = slot.get());
            };
            let Some(n) = next(g, self.last) else {
                capacity_exceeded!("slot generations", self.last, u64::from(g) + 1);
            };
            self.of.set(slot, n);
            n
        };
        GenRef { slot: slot.get(), generation, table: PhantomData }
    }

    /// The slot's generation now, or none for a slot never handed out.
    #[must_use]
    pub fn get(&self, slot: Slot) -> Option<u32> {
        self.of.get(slot)
    }

    /// The low byte of the slot's generation, which a short reference carries.
    #[must_use]
    pub fn short(&self, slot: Slot) -> Option<u8> {
        self.of.get(slot).map(|g| g.to_le_bytes()[0])
    }

    /// The live row a reference names, or none once it has ended or its slot holds another.
    #[must_use]
    pub fn resolve(&self, slots: &SlotAlloc<B>, r: GenRef<T>) -> Option<Slot> {
        let slot = Slot::new(r.slot);
        (live_at(slots.live_words(), slot) & (self.of.get(slot) == Some(r.generation))).then_some(slot)
    }

    /// A reference to the row a slot holds now, or none for a slot never handed out.
    #[must_use]
    pub fn at(&self, slot: Slot) -> Option<GenRef<T>> {
        self.of.get(slot).map(|generation| GenRef { slot: slot.get(), generation, table: PhantomData })
    }

    #[must_use]
    pub fn bytes_committed(&self) -> usize {
        self.of.bytes_committed()
    }
}

impl<T, B: Backing> crate::save::Saved for Generations<T, B> {
    fn save(&self, w: &mut crate::save::Writer<'_>) {
        self.last.save(w);
        self.of.save(w);
    }

    fn load(r: &mut crate::save::Reader<'_>) -> Result<Generations<T, B>, crate::save::LoadError> {
        Ok(Generations { last: u32::load(r)?, of: Column::load(r)?, table: PhantomData })
    }
}

impl<B: Backing> StoreStats for SlotAlloc<B> {
    fn rows_live(&self) -> u64 {
        self.live_count()
    }

    fn rows_ever(&self) -> u64 {
        u64::from(self.high_water())
    }

    fn bytes(&self) -> u64 {
        let bytes = self.bytes_committed();
        u64::try_from(bytes).unwrap_or_else(|_| capacity_exceeded!("a store's bytes", u64::MAX, bytes))
    }
}

/// A reference to a contract row of the table `T` marks: its family's link — family and slot in one word — and the
/// low byte of the row's generation. The byte compares modulo 256, which tells a stale reference from a live one only
/// while its holder re-reads it before the slot has turned 256 times; a slot turns at most once a day, so a holder
/// that re-reads within 255 days never mistakes one. It is held only in a `ShortRefs` column, whose holder declares
/// its re-read.
#[derive(Debug)]
pub struct ShortRef<T> {
    link: ContractLink,
    generation: u8,
    table: PhantomData<fn() -> T>,
}

impl<T> Clone for ShortRef<T> {
    fn clone(&self) -> ShortRef<T> {
        *self
    }
}

impl<T> Copy for ShortRef<T> {}

impl<T> PartialEq for ShortRef<T> {
    fn eq(&self, other: &ShortRef<T>) -> bool {
        (self.link, self.generation) == (other.link, other.generation)
    }
}

impl<T> Eq for ShortRef<T> {}

impl<T> ShortRef<T> {
    /// A family's row at a slot and the low byte of its generation; a slot beyond the link's width stops the run.
    #[must_use]
    pub fn new(family: u8, slot: Slot, generation: u8) -> ShortRef<T> {
        ShortRef { link: ContractLink::new(u32::from(family), slot), generation, table: PhantomData }
    }

    #[must_use]
    pub fn family(self) -> u8 {
        self.link.family()
    }

    pub fn slot(self) -> Slot {
        self.link.slot()
    }

    pub fn link(self) -> ContractLink {
        self.link
    }

    #[must_use]
    pub fn generation(self) -> u8 {
        self.generation
    }

    /// The row it names while that row is live at the generation it was taken at, modulo 256.
    #[must_use]
    pub fn resolve(self, live: &[u64], generation: impl Fn(Slot) -> Option<u8>) -> Option<Slot> {
        let slot = self.slot();
        (live_at(live, slot) & (generation(slot) == Some(self.generation))).then_some(slot)
    }
}

/// A store that holds short references, and the most days it lets pass between two reads of each: a byte, so no
/// holder can declare more days than a short generation tells apart.
pub trait Recheck {
    const DAYS: u8;
}

/// Short references a holder keeps, one to a row of its own, in two columns: the links and the generations' bytes.
#[derive(Debug)]
pub struct ShortRefs<T, H: Recheck, B: Backing = SystemBacking> {
    links: Column<ContractLink, B>,
    generations: Column<u8, B>,
    table: PhantomData<fn() -> (T, H)>,
}

impl<T, H: Recheck, B: Backing> ShortRefs<T, H, B> {
    pub fn new(space: &mut AddressSpace, max: u32, rows_per_chunk: u32) -> ShortRefs<T, H, B> {
        ShortRefs {
            links: Column::new(space, max, rows_per_chunk),
            generations: Column::new(space, max, rows_per_chunk),
            table: PhantomData,
        }
    }

    /// Writes the reference at a row of the holder: the next row, or one it holds.
    pub fn put(&mut self, at: Slot, r: ShortRef<T>) {
        self.links.put(at, r.link);
        self.generations.put(at, r.generation);
    }

    #[must_use]
    pub fn get(&self, at: Slot) -> Option<ShortRef<T>> {
        Some(ShortRef { link: self.links.get(at)?, generation: self.generations.get(at)?, table: PhantomData })
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.links.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.links.is_empty()
    }
}

#[cfg(test)]
#[path = "genref_tests.rs"]
mod tests;
