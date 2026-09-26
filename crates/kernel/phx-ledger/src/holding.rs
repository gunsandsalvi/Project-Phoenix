use phx_core::kind_tables::ListKind;
use phx_id::{Day, InstrumentId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Amount, Count, Missing, QtyRaw, capacity_exceeded, violation};

use crate::holder::HolderArenas;
use crate::words::{from_words, to_words, words_of};

/// An individual's holding of an instrument, 24 bytes: its lots are the next `lots` entries of the holder's lot
/// list, which keeps each holding's lots in the order of its holdings, so no reference into the arena is kept in it.
/// Its basis is read from its lots, never kept a second time.
#[clause("REG.1")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct IndividualHolding {
    pub instrument: InstrumentId,
    pub lots: u32,
    pub quantity: QtyRaw,
    pub flags: u32,
    pad: u32,
}

/// An agent's holding, 24 bytes: one lot at average cost held by `count` of its twins alike, so a twin's
/// quantity is the quantity over the count.
#[clause("REG.1", "REP.9", "ACC.6")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct CellHolding {
    pub instrument: InstrumentId,
    pub count: u32,
    pub quantity: QtyRaw,
    pub pooled_cost: Amount,
}

impl CellHolding {
    /// The members holding it.
    pub fn members(&self) -> Count {
        Count::new(u64::from(self.count))
    }
}

/// Units acquired together: the day, how many, and what they cost in the instrument's currency, 24 bytes.
#[clause("REG.1")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Lot {
    pub acquired: Day,
    pad: u32,
    pub quantity: QtyRaw,
    pub cost: Amount,
}

impl Lot {
    #[must_use]
    pub fn new(acquired: Day, quantity: i64, cost: i64) -> Lot {
        Lot { acquired, pad: 0, quantity: QtyRaw::from_raw(quantity), cost: Amount::from_raw(cost) }
    }
}

/// Marks an individual's holding with liens on it, so a reader knows to read the lien table.
pub const PLEDGED: u32 = 1;

/// Sets or clears the mark of liens on a holding.
pub fn mark_pledged(arenas: &mut dyn HolderArenas, holder: Slot, instrument: InstrumentId, pledged: bool) {
    let all = index(arenas, holder);
    let Some((i, (mut h, _))) = all.iter().copied().enumerate().find(|(_, (h, _))| h.instrument == instrument) else {
        violation!(clause = "REG.2", "a lien on a holding the holder does not have", instrument = instrument.get());
    };
    h.flags = if pledged { h.flags | PLEDGED } else { h.flags & !PLEDGED };
    arenas.overwrite(holder, ListKind::Holdings, i * HOLDING, &to_words(&h));
}

/// Which of a holding's lots go first when units leave it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotOrder {
    /// The earliest acquired first.
    FirstIn,
}

/// Units leaving a holding: how many, how many of the holding are bound elsewhere (pledged or covering an offer),
/// and which lots go first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disposal {
    pub units: i64,
    pub bound: i64,
    pub order: LotOrder,
}

/// What leaving units took with them: their cost, read from the lots they left, and whether the holding is gone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Disposed {
    pub cost: i64,
    pub emptied: bool,
}

const HOLDING: usize = words_of::<IndividualHolding>();
const LOT: usize = words_of::<Lot>();

fn usize_of(n: u32) -> usize {
    let Ok(u) = usize::try_from(n) else {
        capacity_exceeded!("lots of a holding", usize::MAX, n);
    };
    u
}

/// A holder's holdings, each with where its lots start in the lot list.
fn index(arenas: &dyn HolderArenas, holder: Slot) -> Vec<(IndividualHolding, usize)> {
    let mut lot_at = 0;
    arenas
        .read(holder, ListKind::Holdings)
        .as_chunks::<HOLDING>()
        .0
        .iter()
        .map(|w| {
            let h: IndividualHolding = from_words(w);
            let at = lot_at;
            lot_at += usize_of(h.lots) * LOT;
            (h, at)
        })
        .collect()
}

/// A holder's holding of an instrument, if it holds any.
pub fn holding(arenas: &dyn HolderArenas, holder: Slot, instrument: InstrumentId) -> Missing<IndividualHolding> {
    match index(arenas, holder).into_iter().find(|(h, _)| h.instrument == instrument) {
        Some((h, _)) => Missing::Present(h),
        None => Missing::Absent,
    }
}

/// A holding's lots, earliest acquired first.
#[must_use]
pub fn lots(arenas: &dyn HolderArenas, holder: Slot, instrument: InstrumentId) -> Vec<Lot> {
    let Some((h, at)) = index(arenas, holder).into_iter().find(|(h, _)| h.instrument == instrument) else {
        return Vec::new();
    };
    let Some(words) = arenas.read(holder, ListKind::Lots).get(at..at + usize_of(h.lots) * LOT) else {
        violation!(
            clause = "REG.4",
            "a holding's lots missing from its holder's lot list",
            instrument = instrument.get()
        );
    };
    words.as_chunks::<LOT>().0.iter().map(|l| from_words(l)).collect()
}

/// Every holding of a holder with its basis, in the order of its holdings: the cost of the lots of the issue it
/// holds, each cost once.
#[clause("REG.17", "ACC.14", "ACC.15")]
#[must_use]
pub fn bases(arenas: &dyn HolderArenas, holder: Slot) -> Vec<(InstrumentId, i64)> {
    let lots = arenas.read(holder, ListKind::Lots);
    index(arenas, holder)
        .into_iter()
        .map(|(h, at)| {
            let Some(words) = lots.get(at..at + usize_of(h.lots) * LOT) else {
                violation!(
                    clause = "REG.4",
                    "a holding's lots missing from its holder's lot list",
                    instrument = h.instrument.get()
                );
            };
            let cost = words.as_chunks::<LOT>().0.iter().map(|l| from_words::<Lot>(l).cost.raw()).sum();
            (h.instrument, cost)
        })
        .collect()
}

/// A holding's basis: the cost of its lots; a holder without the holding has none.
pub fn basis(arenas: &dyn HolderArenas, holder: Slot, instrument: InstrumentId) -> Missing<i64> {
    match holding(arenas, holder, instrument) {
        Missing::Present(_) => Missing::Present(lots(arenas, holder, instrument).iter().map(|l| l.cost.raw()).sum()),
        Missing::Absent => Missing::Absent,
    }
}

/// Units acquired: a lot added at the end of the holding's lots, the holding begun if the holder had none. Returns
/// whether it was begun, for the instrument's holder list.
#[clause("REG.1", "REG.4")]
pub(crate) fn acquire(arenas: &mut dyn HolderArenas, holder: Slot, instrument: InstrumentId, lot: Lot) -> bool {
    if lot.quantity.raw() <= 0 {
        violation!(clause = "REG.15", "an acquisition of no units or fewer", units = lot.quantity.raw());
    }
    let all = index(arenas, holder);
    let found = all.iter().enumerate().find(|(_, (h, _))| h.instrument == instrument);
    if let Some((i, (h, at))) = found {
        let mut h = *h;
        let end = at + usize_of(h.lots) * LOT;
        arenas.insert(holder, ListKind::Lots, end, &to_words(&lot));
        h.lots += 1;
        let Some(q) = h.quantity.raw().checked_add(lot.quantity.raw()) else {
            violation!(clause = "Law 7", "a holding's quantity overflows", units = lot.quantity.raw());
        };
        h.quantity = QtyRaw::from_raw(q);
        arenas.overwrite(holder, ListKind::Holdings, i * HOLDING, &to_words(&h));
        false
    } else {
        let h = IndividualHolding { instrument, lots: 1, quantity: lot.quantity, flags: 0, pad: 0 };
        arenas.append(holder, ListKind::Lots, &to_words(&lot));
        arenas.append(holder, ListKind::Holdings, &to_words(&h));
        true
    }
}

/// What `taken` of a lot's units cost: the whole lot's cost when all go, else their share of it.
fn lot_share(lot: &Lot, taken: i64) -> i64 {
    let q = lot.quantity.raw();
    if taken == q {
        return lot.cost.raw();
    }
    let share = i128::from(lot.cost.raw()) * i128::from(taken) / i128::from(q);
    let Ok(share) = i64::try_from(share) else {
        violation!(clause = "Law 7", "a lot's share of its cost overflows", cost = lot.cost.raw());
    };
    share
}

/// What the first `units` of a holding would take of its lots' cost if they left it, earliest acquired first, as a
/// disposal takes them; `Absent` when it holds fewer.
#[clause("REG.1", "ACC.6")]
pub fn first_in_cost(arenas: &dyn HolderArenas, holder: Slot, instrument: InstrumentId, units: i64) -> Missing<i64> {
    let (mut left, mut cost) = (units, 0_i64);
    for lot in lots(arenas, holder, instrument) {
        if left == 0 {
            break;
        }
        let q = lot.quantity.raw();
        let taken = if q <= left { q } else { left };
        cost += lot_share(&lot, taken);
        left -= taken;
    }
    if left == 0 { Missing::Present(cost) } else { Missing::Absent }
}

/// Units that leave a holding, taken from its lots in the given order, each lot's cost going with its units in
/// proportion; only free units can leave, so the caller gives what of the holding is pledged or committed.
#[clause("REG.1", "REG.2", "REG.15")]
pub(crate) fn dispose(
    arenas: &mut dyn HolderArenas,
    holder: Slot,
    instrument: InstrumentId,
    disposal: Disposal,
) -> Disposed {
    let Disposal { units, bound, order } = disposal;
    let all = index(arenas, holder);
    let Some((i, (mut h, at))) = all.iter().copied().enumerate().find(|(_, (h, _))| h.instrument == instrument) else {
        violation!(clause = "REG.16", "units disposed of that the holder does not hold", instrument = instrument.get());
    };
    let free = h.quantity.raw() - bound;
    if units <= 0 || units > free {
        violation!(clause = "REG.2", "units that are not free leave a holding", units = units, free = free);
    }
    let LotOrder::FirstIn = order;
    let mut lots: Vec<Lot> = lots(arenas, holder, instrument);
    let (mut left, mut cost) = (units, 0_i64);
    for lot in &mut lots {
        if left == 0 {
            break;
        }
        let q = lot.quantity.raw();
        let taken = if q <= left { q } else { left };
        let taken_cost = lot_share(lot, taken);
        cost += taken_cost;
        *lot = Lot::new(lot.acquired, q - taken, lot.cost.raw() - taken_cost);
        left -= taken;
    }
    lots.retain(|l| l.quantity.raw() > 0);
    arenas.remove(holder, ListKind::Lots, at, usize_of(h.lots) * LOT);
    let kept: Vec<u64> = lots.iter().flat_map(to_words).collect();
    arenas.insert(holder, ListKind::Lots, at, &kept);
    h.quantity = QtyRaw::from_raw(h.quantity.raw() - units);
    let Ok(n) = u32::try_from(lots.len()) else {
        capacity_exceeded!("lots of a holding", u32::MAX, lots.len());
    };
    h.lots = n;
    let emptied = h.quantity.raw() == 0;
    if emptied {
        arenas.remove(holder, ListKind::Holdings, i * HOLDING, HOLDING);
    } else {
        arenas.overwrite(holder, ListKind::Holdings, i * HOLDING, &to_words(&h));
    }
    Disposed { cost, emptied }
}
