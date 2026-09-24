use phx_core::kind_tables::ListKind;
use phx_id::{Day, Slot, TileId};
use phx_macros::{Pod, clause};
use phx_num::violation;

use crate::holder::HolderArenas;
use crate::words::{from_words, to_words, words_of};

/// One of an individual's units of plant or dwellings, 24 bytes: its identity, where it stands, when it entered
/// service, its class and its condition, which it keeps through every sale.
#[clause("REG.9")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct NamedUnit {
    pub id: u64,
    pub site: TileId,
    pub service_day: Day,
    pub class: u16,
    pub condition: u8,
    pad: [u8; 5],
}

impl NamedUnit {
    #[must_use]
    pub fn new(id: u64, site: TileId, service_day: Day, class: u16, condition: u8) -> NamedUnit {
        NamedUnit { id, site, service_day, class, condition, pad: [0; 5] }
    }
}

const UNIT: usize = words_of::<NamedUnit>();

/// An individual's named units.
#[must_use]
pub fn named(arenas: &dyn HolderArenas, holder: Slot) -> Vec<NamedUnit> {
    arenas.read(holder, ListKind::NamedUnits).as_chunks::<UNIT>().0.iter().map(|u| from_words(u)).collect()
}

/// A named unit added to a holder.
pub fn add(arenas: &mut dyn HolderArenas, holder: Slot, unit: NamedUnit) {
    arenas.append(holder, ListKind::NamedUnits, &to_words(&unit));
}

/// A named unit taken from a holder.
pub fn take(arenas: &mut dyn HolderArenas, holder: Slot, id: u64) -> NamedUnit {
    let units = named(arenas, holder);
    let Some(i) = units.iter().position(|u| u.id == id) else {
        violation!(clause = "REG.9", "a named unit taken from a holder that does not hold it", id = id);
    };
    arenas.remove(holder, ListKind::NamedUnits, i * UNIT, UNIT);
    let Some(unit) = units.get(i).copied() else {
        violation!(clause = "REG.9", "a named unit read beyond its holder's list", id = id);
    };
    unit
}

/// A named unit sold: its title moves to the buyer, its site and condition with it unchanged.
#[clause("GEO.15")]
pub fn transfer(
    from: &mut dyn HolderArenas,
    seller: Slot,
    to: &mut dyn HolderArenas,
    buyer: Slot,
    id: u64,
) -> NamedUnit {
    let unit = take(from, seller, id);
    add(to, buyer, unit);
    unit
}
