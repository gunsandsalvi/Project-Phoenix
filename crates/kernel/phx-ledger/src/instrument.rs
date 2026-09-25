use phx_id::{InstrumentId, PartyId, Slot};
use phx_macros::{Pod, clause};
use phx_num::{Ccy, Missing, Qty, QtyRaw, UnitId, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, BlockList, Column, SystemBacking};

use crate::holder::{HolderArenas, HolderKeys, HolderLists};
use crate::holding::{self, Disposal, Disposed, Lot};
use crate::terms::TermsId;

/// What kind of thing an instrument is. Which families a world has, and what each family's terms carry, is declared
/// data; generic code reads the legs.
#[clause("REG.18")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentFamily {
    Debt,
    Equity,
    FundUnit,
    Contract,
    RealAsset,
    Banknote,
}

/// The families in the order their codes are stored.
const FAMILIES: &[InstrumentFamily] = &[
    InstrumentFamily::Debt,
    InstrumentFamily::Equity,
    InstrumentFamily::FundUnit,
    InstrumentFamily::Contract,
    InstrumentFamily::RealAsset,
    InstrumentFamily::Banknote,
];

/// An instrument's state, whose one writer is the ledger's instrument events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentState {
    Live,
    Suspended,
    Defaulted,
    Ceased,
}

/// The states in the order their codes are stored.
const STATES: &[InstrumentState] =
    &[InstrumentState::Live, InstrumentState::Suspended, InstrumentState::Defaulted, InstrumentState::Ceased];

fn code<T: PartialEq>(all: &[T], v: &T) -> u8 {
    let Some(at) = all.iter().position(|x| x == v).and_then(|i| u8::try_from(i).ok()) else {
        violation!(clause = "REG.3", "an instrument's family or state beyond its declared list");
    };
    at
}

fn decode<T: Copy>(all: &[T], code: u8) -> T {
    let Some(v) = all.get(usize::from(code)).copied() else {
        violation!(clause = "REG.3", "a stored family or state code beyond its declared list", code = code);
    };
    v
}

/// Why an instrument's issued amount changes; nothing else changes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueChange {
    Issuance,
    Reopening,
    Buyback,
    Conversion,
    Amortisation,
    Maturity,
    Default,
}

/// An instrument as its readers see it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instrument {
    pub id: InstrumentId,
    pub family: InstrumentFamily,
    pub issuer: Missing<PartyId>,
    pub unit: UnitId,
    pub ccy: Ccy,
    pub issued: Qty,
    pub terms: TermsId,
    pub state: InstrumentState,
}

/// A new instrument's fixed parts; its issued amount starts at nothing and changes only by its issuer's acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NewInstrument {
    pub family: InstrumentFamily,
    pub issuer: Missing<PartyId>,
    pub unit: UnitId,
    pub ccy: Ccy,
    pub terms: TermsId,
}

/// An instrument's stored row, 32 bytes; a party identity is never zero, so zero stores a real asset's absent issuer.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
struct InstrumentRow {
    issuer: u64,
    issued: QtyRaw,
    terms: u32,
    unit: UnitId,
    ccy: Ccy,
    family: u8,
    state: u8,
    pad: [u8; 7],
}

const NO_ISSUER: u64 = 0;

/// Every instrument the world has issued, by identity, with the holders of each in its holder list; an identity is
/// never handed out twice.
#[clause("REG.3", "REG.4", "REG.16")]
#[derive(Debug)]
pub struct Instruments<B: Backing = SystemBacking> {
    rows: Column<InstrumentRow, B>,
    holders: Column<BlockList, B>,
    lists: HolderLists<B>,
}

impl<B: Backing> Instruments<B> {
    /// Room for at most `max` instruments, chunked by `per_chunk`, a power of two, and `blocks` holder-list blocks in
    /// each pool.
    pub fn new(space: &mut AddressSpace, max: u32, per_chunk: u32, blocks: u32, keys: HolderKeys) -> Instruments<B> {
        Instruments {
            rows: Column::new(space, max, per_chunk),
            holders: Column::new(space, max, per_chunk),
            lists: HolderLists::new(space, blocks, keys),
        }
    }

    /// Issues an instrument with nothing yet issued. A claim without an issuer stops the run; only a real
    /// asset has none.
    pub fn issue(&mut self, new: NewInstrument) -> InstrumentId {
        let issuer = match (new.issuer, new.family) {
            (Missing::Present(p), InstrumentFamily::RealAsset) => {
                violation!(clause = "REG.9", "a real asset issued by a party", party = p.get());
            }
            (Missing::Present(p), _) => p.get(),
            (Missing::Absent, InstrumentFamily::RealAsset) => NO_ISSUER,
            (Missing::Absent, _) => violation!(clause = "REG.16", "a claim without an issuer"),
        };
        let Ok(id) = u32::try_from(self.rows.len()) else {
            capacity_exceeded!("instruments", u32::MAX, self.rows.len());
        };
        self.rows.push(InstrumentRow {
            issuer,
            issued: QtyRaw::from_raw(0),
            terms: new.terms.get(),
            unit: new.unit,
            ccy: new.ccy,
            family: code(FAMILIES, &new.family),
            state: code(STATES, &InstrumentState::Live),
            pad: [0; 7],
        });
        self.holders.push(BlockList::EMPTY);
        InstrumentId::new(id)
    }

    fn row(&self, id: InstrumentId) -> InstrumentRow {
        let Some(row) = self.rows.get(Slot::new(id.get())) else {
            violation!(clause = "REG.16", "an instrument that was never issued", id = id.get());
        };
        row
    }

    /// An instrument by its identity.
    #[must_use]
    pub fn get(&self, id: InstrumentId) -> Instrument {
        let row = self.row(id);
        Instrument {
            id,
            family: decode(FAMILIES, row.family),
            issuer: if row.issuer == NO_ISSUER { Missing::Absent } else { Missing::Present(PartyId::new(row.issuer)) },
            unit: row.unit,
            ccy: row.ccy,
            issued: Qty::at(row.unit, row.issued),
            terms: crate::terms::TermsId::new(row.terms),
            state: decode(STATES, row.state),
        }
    }

    /// Changes the issued amount, for one of the reasons that may; an issued amount below nothing stops the
    /// run.
    pub(crate) fn change_issued(&mut self, id: InstrumentId, by: Qty, _why: IssueChange) {
        let mut row = self.row(id);
        let issued = Qty::at(row.unit, row.issued) + by;
        if issued.n() < 0 {
            violation!(clause = "REG.15", "an issued amount below nothing", id = id.get(), by = by.n());
        }
        row.issued = issued.raw();
        self.rows.set(Slot::new(id.get()), row);
    }

    /// Sets the state, for the instrument events alone.
    pub(crate) fn set_state(&mut self, id: InstrumentId, state: InstrumentState) {
        let mut row = self.row(id);
        row.state = code(STATES, &state);
        self.rows.set(Slot::new(id.get()), row);
    }

    fn list(&self, id: InstrumentId) -> BlockList {
        let Some(list) = self.holders.get(Slot::new(id.get())) else {
            violation!(clause = "REG.16", "an instrument that was never issued", id = id.get());
        };
        list
    }

    /// An instrument's holders, as their keys, in order.
    pub fn holders(&self, id: InstrumentId) -> impl Iterator<Item = u32> + '_ {
        self.lists.iter(id.get(), self.list(id))
    }

    /// The holder keys' split, for reading a holder list's entries.
    #[must_use]
    pub fn keys(&self) -> HolderKeys {
        self.lists.keys()
    }

    /// Units acquired by a holder of the table at place `table`, which enters the instrument's holder list with its
    /// first lot.
    pub(crate) fn acquire(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        id: InstrumentId,
        lot: Lot,
    ) {
        let _ = self.row(id);
        if holding::acquire(arenas, holder, id, lot) {
            let mut list = self.list(id);
            self.lists.enter(id.get(), &mut list, table, holder);
            self.holders.set(Slot::new(id.get()), list);
        }
    }

    /// Units that leave a holding, its holder leaving the instrument's holder list with its last unit.
    pub(crate) fn dispose(
        &mut self,
        arenas: &mut dyn HolderArenas,
        table: u16,
        holder: Slot,
        id: InstrumentId,
        disposal: Disposal,
    ) -> Disposed {
        let gone = holding::dispose(arenas, holder, id, disposal);
        if gone.emptied {
            let mut list = self.list(id);
            self.lists.leave(id.get(), &mut list, table, holder);
            self.holders.set(Slot::new(id.get()), list);
        }
        gone
    }

    /// Each instrument's row, in identity order; its holder list is an index of its holders' holdings and stays
    /// out.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        h.bytes(phx_store::as_bytes(self.rows.slice()));
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// A column of `n` values of another column's geometry, in the reader's address space.
pub(crate) fn column_like<T: phx_store::Pod, U: phx_store::Pod, B: Backing>(
    r: &mut phx_store::Reader<'_>,
    like: &Column<U, B>,
    value: T,
) -> Result<Column<T, B>, phx_store::LoadError> {
    let cap = phx_store::narrow::<u32>(like.capacity(), "a column's rows")?;
    let per = phx_store::narrow::<u32>(like.rows_per_chunk(), "a column's rows per chunk")?;
    let mut c = Column::new(r.space(), cap, per);
    for _ in 0..like.len() {
        c.push(value);
    }
    Ok(c)
}

impl<B: Backing> Instruments<B> {
    /// The instruments for a save: their rows and the room their holder lists had; the lists are an index of the
    /// holdings and are rebuilt.
    pub(crate) fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        self.rows.save(w);
        self.lists.blocks().save(w);
    }

    /// The instruments read back with empty holder lists, which the books rebuild from the holdings.
    pub(crate) fn load_from(
        r: &mut phx_store::Reader<'_>,
        keys: HolderKeys,
    ) -> Result<Instruments<B>, phx_store::LoadError> {
        use phx_store::Saved as _;
        let rows: Column<InstrumentRow, B> = Column::load(r)?;
        let blocks = u32::load(r)?;
        let holders = column_like(r, &rows, BlockList::EMPTY)?;
        Ok(Instruments { rows, holders, lists: HolderLists::new(r.space(), blocks, keys) })
    }

    /// A holder put back on an instrument's holder list as a load rebuilds it.
    pub(crate) fn relist(&mut self, table: u16, holder: Slot, id: InstrumentId) {
        let mut list = self.list(id);
        self.lists.enter(id.get(), &mut list, table, holder);
        self.holders.set(Slot::new(id.get()), list);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;
    use phx_num::{Ccy, Missing, Qty, UnitId};
    use phx_store::{AddressSpace, HeapBacking};

    use super::{InstrumentFamily, InstrumentState, Instruments, IssueChange, NewInstrument};
    use crate::holder::HolderKeys;
    use crate::terms::TermsId;

    type Heap = HeapBacking<4096>;

    fn bond(issuer: Missing<PartyId>, family: InstrumentFamily) -> NewInstrument {
        NewInstrument { family, issuer, unit: UnitId::new(0), ccy: Ccy::new(0), terms: TermsId::new(0) }
    }

    #[test]
    fn instruments_are_issued_and_changed_only_by_their_reasons() {
        let mut space = AddressSpace::empty();
        let mut all: Instruments<Heap> = Instruments::new(&mut space, 64, 16, 16, HolderKeys::new(2));
        let id = all.issue(bond(Missing::Present(PartyId::new(3)), InstrumentFamily::Debt));
        all.change_issued(id, Qty::new(1_000, UnitId::new(0)), IssueChange::Issuance);
        all.change_issued(id, Qty::new(-250, UnitId::new(0)), IssueChange::Buyback);
        let i = all.get(id);
        assert_eq!((i.issued.n(), i.state, i.issuer), (750, InstrumentState::Live, Missing::Present(PartyId::new(3))));
        let land = all.issue(bond(Missing::Absent, InstrumentFamily::RealAsset));
        assert_eq!(all.get(land).issuer, Missing::Absent, "a real asset has no issuer");
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn a_claim_without_an_issuer_stops_the_run() {
        let mut space = AddressSpace::empty();
        let mut all: Instruments<Heap> = Instruments::new(&mut space, 64, 16, 16, HolderKeys::new(2));
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = all.issue(bond(Missing::Absent, InstrumentFamily::Debt));
        }));
        let payload = caught.expect_err("a debt without an issuer stops the run");
        assert_eq!(payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause), Some("REG.16"));
    }
}
