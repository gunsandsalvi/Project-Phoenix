use phx_core::schema::FactColumn;
use phx_core::{RunHead, Weight};
use phx_id::{Day, PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::consts::ABSENT_I64;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::consts::ARENA_RESERVED_WORDS;
use phx_store::{AddressSpace, Backing, CellListRef, ChunkArena, Column, ListRef, Region, SystemBacking, Table};

use crate::consts::{HOT_LEAD, HOT_STEPS};
use crate::hot::HotRecord;
use crate::individual::{ExtList, Extension};
use crate::key::KeyId;
use crate::kind::{PopKindDecl, Scale};
use crate::landing::landing_key;
use crate::profile::{Profile, ProfileLayout, WordBytes, from_words, read_group, to_words};
use crate::steps::{Step, StepTable};

/// The lists a cell keeps in its chunk's arena: relationship rows, holdings, profiles, and the standing rates too
/// large for their column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellList {
    Rows,
    Holdings,
    Profiles,
    Rates,
}

impl CellList {
    pub const ALL: [CellList; 4] = [CellList::Rows, CellList::Holdings, CellList::Profiles, CellList::Rates];

    /// The list's place among a cell's lists.
    fn place(self) -> usize {
        let Some(i) = CellList::ALL.iter().position(|l| *l == self) else {
            violation!(clause = "REP.3", "a cell list outside the lists a cell keeps");
        };
        i
    }
}

/// A standing rate stored in its column: the rate itself, or this mark, which sends a read to the cell's rate list,
/// where a rate too large for the column is kept and an absent one is not.
const RATE_ELSEWHERE: u32 = u32::MAX;

/// A cell's due-day run head in eight bytes: the earliest day a dated row can fall due, and where its dated rows lie
/// in its relationship rows. A cell's rows are few, so their offset and length fit sixteen bits; one that did not is a
/// contract violation calling for a wider layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct CellRunHead {
    pub next_due: u32,
    pub offset: u16,
    pub len: u16,
}

impl CellRunHead {
    pub const EMPTY: CellRunHead = CellRunHead { next_due: 0, offset: 0, len: 0 };

    fn wide(self) -> RunHead {
        RunHead { next_due: self.next_due, offset: u32::from(self.offset), len: u32::from(self.len) }
    }

    fn narrow(head: RunHead) -> CellRunHead {
        let (Ok(offset), Ok(len)) = (u16::try_from(head.offset), u16::try_from(head.len)) else {
            capacity_exceeded!("a cell's dated rows for its run head", u16::MAX, head.offset + head.len);
        };
        CellRunHead { next_due: head.next_due, offset, len }
    }
}

/// A cell's attention to one kind of lumpy decision: its stake's weight and its own outlook's variance, each a fixed
/// point the deciding system writes; absent until attention is decided.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct Attention {
    pub stake: u32,
    pub own: u32,
}

impl Attention {
    const ABSENT: Attention = Attention { stake: u32::MAX, own: u32::MAX };
}

/// A new cell: its party, when it was created, its weight and key, every position's total in the kind's order, and
/// its profiles.
#[derive(Clone, Copy, Debug)]
pub struct NewCell<'a> {
    pub party: PartyId,
    pub created: Day,
    pub weight: Weight,
    pub key: KeyId,
    pub positions: &'a [i64],
    pub profile: &'a Profile,
}

/// The table of one population kind: every cell's hot record, its positions beyond the leading three as totals, its
/// kink signature, standing rates, review exposures and attention, the due-day run head, and its lists in its
/// chunk's arena. Individuals of the kind are rows of weight one with an extension row.
#[clause("REP.1", "REP.20", "REP.32", "REP.3")]
#[derive(Debug, phx_macros::Saved)]
pub struct CellTable<B: Backing = SystemBacking> {
    kind: &'static str,
    table: Table<B>,
    party: Column<u64, B>,
    created: Column<u32, B>,
    hot: Column<HotRecord, B>,
    positions: Vec<Column<i64, B>>,
    sig: Vec<Column<u64, B>>,
    rates: Vec<Column<u32, B>>,
    exposures: Vec<Column<i64, B>>,
    attention: Vec<Column<Attention, B>>,
    lists: Vec<Column<CellListRef, B>>,
    runs: Column<CellRunHead, B>,
    arenas: Vec<ChunkArena<B>>,
    ext: Extension<B>,
    #[saved(skip)]
    layout: Layout,
}

/// What the table needs of its kind's layout, rebuilt from the kind after a load.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Layout {
    positions: usize,
    profiles: ProfileLayout,
}

fn at(slot: Slot) -> usize {
    let Ok(i) = usize::try_from(slot.get()) else {
        capacity_exceeded!("index width", usize::MAX, slot.get());
    };
    i
}

fn put<T: phx_store::Pod, B: Backing>(column: &mut Column<T, B>, slot: Slot, value: T) {
    if at(slot) == column.len() {
        column.push(value);
    } else {
        column.set(slot, value);
    }
}

fn read<T: phx_store::Pod, B: Backing>(column: &Column<T, B>, slot: Slot) -> T {
    let Some(v) = column.get(slot) else {
        violation!(clause = "PTY.10", "a read of a cell row no party holds", slot = slot.get());
    };
    v
}

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of a cell's list", u32::MAX, n);
    };
    w
}

impl<B: Backing> CellTable<B> {
    /// An empty table for a population kind, of at most `max_rows` cells.
    pub fn new(
        space: &mut AddressSpace,
        kind: &PopKindDecl,
        id: TableId,
        max_rows: u32,
        rows_per_chunk: u32,
    ) -> CellTable<B> {
        let table: Table<B> = Table::new(space, id, max_rows, rows_per_chunk);
        CellTable {
            kind: kind.kind,
            party: table.column(space),
            created: table.column(space),
            hot: table.column(space),
            positions: kind.positions.iter().skip(HOT_LEAD).map(|_| table.column(space)).collect(),
            sig: (0..kind.sig.words()).map(|_| table.column(space)).collect(),
            rates: kind.rates.iter().map(|_| table.column(space)).collect(),
            exposures: kind.reviews.iter().map(|_| table.column(space)).collect(),
            attention: kind.reviews.iter().map(|_| table.column(space)).collect(),
            lists: CellList::ALL.iter().map(|_| table.column(space)).collect(),
            runs: table.column(space),
            arenas: Vec::new(),
            ext: Extension::new(space, id, max_rows, rows_per_chunk),
            layout: Layout::of(kind),
            table,
        }
    }

    /// The kind's layout restored after a load, which the save does not carry.
    pub fn relayout(&mut self, kind: &PopKindDecl) {
        self.layout = Layout::of(kind);
    }

    pub fn id(&self) -> TableId {
        self.table.id
    }

    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    /// Bytes a cell's row takes in the table's columns, its arena lists apart.
    #[must_use]
    pub fn bytes_per_row(&self) -> usize {
        let fixed = size_of::<u64>() + size_of::<u32>() + size_of::<HotRecord>() + size_of::<CellRunHead>();
        fixed
            + self.positions.len() * size_of::<i64>()
            + self.sig.len() * size_of::<u64>()
            + self.rates.len() * size_of::<u32>()
            + self.exposures.len() * size_of::<i64>()
            + self.attention.len() * size_of::<Attention>()
            + self.lists.len() * size_of::<CellListRef>()
    }

    /// A row for a new cell, keyed at once at the given levels: its hot record, positions and profiles; its
    /// signature at band nought, its rates, review exposures and attention absent, and its lists otherwise empty.
    pub fn add(&mut self, space: &mut AddressSpace, cell: NewCell<'_>, kind: &PopKindDecl, levels: &[u8]) -> Slot {
        if cell.weight == Weight::new(0) {
            violation!(clause = "REP.17", "a cell of no members", party = cell.party.get());
        }
        if cell.positions.len() != self.layout.positions {
            violation!(
                clause = "REP.20",
                "a cell with another count of positions than its kind",
                given = cell.positions.len()
            );
        }
        let slot = self.table.slots.alloc();
        put(&mut self.party, slot, cell.party.get());
        put(&mut self.created, slot, cell.created.get());
        let mut hot = HotRecord::new(cell.key, cell.weight);
        for (lead, total) in hot.lead.iter_mut().zip(cell.positions) {
            *lead = *total;
        }
        put(&mut self.hot, slot, hot);
        for (column, total) in self.positions.iter_mut().zip(cell.positions.iter().skip(HOT_LEAD)) {
            put(column, slot, *total);
        }
        for column in &mut self.sig {
            put(column, slot, 0);
        }
        for column in &mut self.rates {
            put(column, slot, RATE_ELSEWHERE);
        }
        for column in &mut self.exposures {
            put(column, slot, ABSENT_I64);
        }
        for column in &mut self.attention {
            put(column, slot, Attention::ABSENT);
        }
        for column in &mut self.lists {
            put(column, slot, CellListRef::EMPTY);
        }
        put(&mut self.runs, slot, CellRunHead::EMPTY);
        let chunk = at(slot) / at(Slot::new(self.table.rows_per_chunk()));
        while self.arenas.len() <= chunk {
            self.arenas.push(ChunkArena::new(space, ARENA_RESERVED_WORDS));
        }
        self.set_profile(slot, cell.profile);
        self.rekey(slot, kind, levels);
        slot
    }

    /// Frees a row whose cell has ended with no rows or holdings left; its profiles and rates go with it.
    pub fn remove(&mut self, slot: Slot) {
        self.live(slot);
        for list in [CellList::Rows, CellList::Holdings] {
            if self.list(slot, list).len != 0 {
                violation!(clause = "PTY.10", "a cell removed with lists still in its arena", slot = slot.get());
            }
        }
        if let Missing::Present(ext) = self.hot(slot).individual_ext() {
            self.ext.remove(ext);
        }
        for list in [CellList::Profiles, CellList::Rates] {
            self.edit_list(slot, list, ChunkArena::clear);
        }
        self.table.slots.release(slot);
    }

    fn live(&self, slot: Slot) {
        if !self.table.slots.is_live(slot) {
            violation!(clause = "PTY.10", "a read of a cell row no party holds", slot = slot.get());
        }
    }

    /// Slots the table has handed out, live or freed: every cell's slot lies below it.
    #[must_use]
    pub fn high_water(&self) -> u32 {
        self.table.slots.high_water()
    }

    /// Whether a cell holds the slot.
    #[must_use]
    pub fn is_live(&self, slot: Slot) -> bool {
        self.table.slots.is_live(slot)
    }

    /// The rows cells hold, in slot order.
    pub fn slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.table.slots.live_slots()
    }

    pub fn party(&self, slot: Slot) -> PartyId {
        self.live(slot);
        PartyId::new(read(&self.party, slot))
    }

    pub fn created(&self, slot: Slot) -> Day {
        self.live(slot);
        Day::new(read(&self.created, slot))
    }

    pub fn hot(&self, slot: Slot) -> HotRecord {
        self.live(slot);
        read(&self.hot, slot)
    }

    pub fn weight(&self, slot: Slot) -> Weight {
        self.hot(slot).weight()
    }

    /// A position's total, by its place in the kind's order.
    #[must_use]
    pub fn position(&self, slot: Slot, i: usize) -> i64 {
        if i >= self.layout.positions {
            violation!(clause = "REP.20", "a position the kind does not hold", position = i);
        }
        let hot = self.hot(slot);
        if let Some(t) = hot.lead.get(i) {
            return *t;
        }
        let Some(column) = i.checked_sub(HOT_LEAD).and_then(|j| self.positions.get(j)) else {
            violation!(clause = "REP.20", "a position the kind does not hold", position = i);
        };
        read(column, slot)
    }

    pub fn set_position(&mut self, slot: Slot, i: usize, total: i64) {
        if i >= self.layout.positions {
            violation!(clause = "REP.20", "a position the kind does not hold", position = i);
        }
        let mut hot = self.hot(slot);
        if let Some(t) = hot.lead.get_mut(i) {
            *t = total;
            self.hot.set(slot, hot);
            return;
        }
        let Some(column) = i.checked_sub(HOT_LEAD).and_then(|j| self.positions.get_mut(j)) else {
            violation!(clause = "REP.20", "a position the kind does not hold", position = i);
        };
        column.set(slot, total);
    }

    /// How many positions the kind keeps.
    #[must_use]
    pub fn positions(&self) -> usize {
        self.layout.positions
    }

    /// How many standing rates the kind keeps.
    #[must_use]
    pub fn rate_kinds(&self) -> usize {
        self.rates.len()
    }

    /// How many kinds of lumpy decision the kind reviews.
    #[must_use]
    pub fn review_kinds(&self) -> usize {
        self.exposures.len()
    }

    /// The cell's kink signature, a word per column.
    #[must_use]
    pub fn sig(&self, slot: Slot) -> Vec<u64> {
        self.live(slot);
        self.sig.iter().map(|c| read(c, slot)).collect()
    }

    pub fn set_sig(&mut self, slot: Slot, sig: &[u64]) {
        self.live(slot);
        if sig.len() != self.sig.len() {
            violation!(clause = "REP.16", "a signature of another width than its kind's", words = sig.len());
        }
        for (c, w) in self.sig.iter_mut().zip(sig) {
            c.set(slot, *w);
        }
    }

    /// A standing rate per member, absent until the cell first decides it.
    pub fn rate(&self, slot: Slot, r: usize) -> Missing<i64> {
        self.live(slot);
        let Some(column) = self.rates.get(r) else {
            violation!(clause = "REP.20", "a standing rate the kind does not hold", rate = r);
        };
        match read(column, slot) {
            RATE_ELSEWHERE => {
                let words = self.words(slot, CellList::Rates);
                let r64 = phx_rand::float::len_u64(r);
                match words.chunks(2).find(|p| p.first() == Some(&r64)) {
                    Some([_, v]) => Missing::Present(v.cast_signed()),
                    _ => Missing::Absent,
                }
            }
            v => Missing::Present(i64::from(v)),
        }
    }

    /// A standing rate set, in its column when it fits and in the rate list when it does not, or cleared.
    pub fn set_rate(&mut self, slot: Slot, r: usize, rate: Missing<i64>) {
        self.live(slot);
        let r64 = phx_rand::float::len_u64(r);
        let found = self.words(slot, CellList::Rates).chunks(2).position(|p| p.first() == Some(&r64));
        if let Some(i) = found {
            self.edit_list(slot, CellList::Rates, |arena, list| arena.remove(list, word32(i * 2), 2));
        }
        let inline = match rate {
            Missing::Present(v) => u32::try_from(v).ok().filter(|v| *v != RATE_ELSEWHERE),
            Missing::Absent => None,
        };
        let Some(column) = self.rates.get_mut(r) else {
            violation!(clause = "REP.20", "a standing rate the kind does not hold", rate = r);
        };
        column.set(
            slot,
            match inline {
                Some(v) => v,
                None => RATE_ELSEWHERE,
            },
        );
        if let (None, Missing::Present(v)) = (inline, rate) {
            self.edit_list(slot, CellList::Rates, |arena, list| arena.append(list, &[r64, v.cast_unsigned()]));
        }
    }

    /// The members' review exposure to one kind of lumpy decision, absent until attention exists.
    pub fn exposure(&self, slot: Slot, k: usize) -> Missing<i64> {
        self.live(slot);
        let Some(column) = self.exposures.get(k) else {
            violation!(clause = "REP.21", "a review kind the kind does not hold", review = k);
        };
        match read(column, slot) {
            ABSENT_I64 => Missing::Absent,
            v => Missing::Present(v),
        }
    }

    pub fn set_exposure(&mut self, slot: Slot, k: usize, exposure: i64) {
        self.live(slot);
        if exposure == ABSENT_I64 {
            violation!(clause = "NUM.8", "the absent marker written as an exposure");
        }
        let Some(column) = self.exposures.get_mut(k) else {
            violation!(clause = "REP.21", "a review kind the kind does not hold", review = k);
        };
        column.set(slot, exposure);
    }

    /// The cell's attention to one kind of lumpy decision, absent until it decides it.
    pub fn attention(&self, slot: Slot, k: usize) -> Missing<Attention> {
        self.live(slot);
        let Some(column) = self.attention.get(k) else {
            violation!(clause = "REP.38", "a review kind the kind does not hold", review = k);
        };
        match read(column, slot) {
            a if a == Attention::ABSENT => Missing::Absent,
            a => Missing::Present(a),
        }
    }

    pub fn set_attention(&mut self, slot: Slot, k: usize, a: Attention) {
        self.live(slot);
        if a == Attention::ABSENT {
            violation!(clause = "NUM.8", "the absent marker written as attention");
        }
        let Some(column) = self.attention.get_mut(k) else {
            violation!(clause = "REP.38", "a review kind the kind does not hold", review = k);
        };
        column.set(slot, a);
    }

    /// The cell's profiles, read from its arena.
    #[must_use]
    pub fn profile(&self, slot: Slot) -> Profile {
        let words = self.words(slot, CellList::Profiles);
        let Some((len, packed)) = words.split_first() else {
            violation!(clause = "REP.32", "a cell with no profile", slot = slot.get());
        };
        let Ok(n) = usize::try_from(*len) else {
            violation!(clause = "REP.32", "a profile longer than this machine's words", slot = slot.get());
        };
        match Profile::decode(&self.layout.profiles, &from_words(packed, n)) {
            Ok(p) => p,
            Err(_) => violation!(clause = "REP.32", "a cell's profile does not read back", slot = slot.get()),
        }
    }

    /// One profile group's values held by the cell's members, read in place without decoding its other groups.
    #[must_use]
    pub fn profile_group(&self, slot: Slot, group: usize) -> Vec<(u32, u32)> {
        let words = self.words(slot, CellList::Profiles);
        let Some((len, packed)) = words.split_first() else {
            violation!(clause = "REP.32", "a cell with no profile", slot = slot.get());
        };
        let Ok(n) = usize::try_from(*len) else {
            violation!(clause = "REP.32", "a profile longer than this machine's words", slot = slot.get());
        };
        match read_group(&self.layout.profiles, &WordBytes { words: packed, len: n }, group) {
            Ok(g) => g,
            Err(_) => violation!(clause = "REP.32", "a cell's profile does not read back", slot = slot.get()),
        }
    }

    /// The cell's profiles written into its arena: their byte count, then their bytes packed into words.
    pub fn set_profile(&mut self, slot: Slot, profile: &Profile) {
        let bytes = profile.encode(&self.layout.profiles);
        let mut words = vec![phx_rand::float::len_u64(bytes.len())];
        words.extend(to_words(&bytes));
        self.edit_list(slot, CellList::Profiles, |arena, list| {
            arena.remove(list, 0, list.len);
            arena.append(list, &words);
        });
    }

    /// The kind's profile layout.
    #[must_use]
    pub fn profile_layout(&self) -> &ProfileLayout {
        &self.layout.profiles
    }

    fn owner(slot: Slot, list: CellList) -> u32 {
        let Some(o) =
            slot.get().checked_mul(word32(CellList::ALL.len())).and_then(|o| o.checked_add(word32(list.place())))
        else {
            capacity_exceeded!("cell list owners", u32::MAX, slot.get());
        };
        o
    }

    fn chunk(&self, slot: Slot) -> usize {
        at(slot) / at(Slot::new(self.table.rows_per_chunk()))
    }

    fn arena(&self, slot: Slot) -> &ChunkArena<B> {
        let Some(a) = self.arenas.get(self.chunk(slot)) else {
            violation!(clause = "PTY.10", "a cell beyond the table's chunks", slot = slot.get());
        };
        a
    }

    fn list_column(&self, list: CellList) -> &Column<CellListRef, B> {
        let Some(c) = self.lists.get(list.place()) else {
            violation!(clause = "REP.3", "a cell list the table does not keep", list = list.place());
        };
        c
    }

    /// A cell's list, its full reference.
    pub fn list(&self, slot: Slot, list: CellList) -> ListRef {
        self.live(slot);
        self.arena(slot).resolve(Self::owner(slot, list), read(self.list_column(list), slot))
    }

    /// A cell's list, as its arena's words.
    #[must_use]
    pub fn words(&self, slot: Slot, list: CellList) -> &[u64] {
        let r = self.list(slot, list);
        self.arena(slot).read(r)
    }

    /// Edits a cell's list in its arena, writing back its reference, which an edit may move or lengthen past its
    /// compact form.
    pub fn edit_list<R>(
        &mut self,
        slot: Slot,
        list: CellList,
        f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef) -> R,
    ) -> R {
        let mut full = self.list(slot, list);
        let chunk = self.chunk(slot);
        let owner = Self::owner(slot, list);
        let mut compact = read(self.list_column(list), slot);
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "a cell beyond the table's chunks", slot = slot.get());
        };
        let out = f(arena, &mut full);
        arena.store(owner, &mut compact, full);
        if let Some(c) = self.lists.get_mut(list.place()) {
            c.set(slot, compact);
        }
        out
    }

    /// An individual's list kept in its extension.
    #[must_use]
    pub fn ext_words(&self, slot: Slot, list: ExtList) -> &[u64] {
        match self.hot(slot).individual_ext() {
            Missing::Present(ext) => self.arena(slot).read(self.ext.list(ext, list)),
            Missing::Absent => &[],
        }
    }

    /// Edits an individual's list kept in its extension; a cell has none.
    pub fn edit_ext<R>(
        &mut self,
        slot: Slot,
        list: ExtList,
        f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef) -> R,
    ) -> R {
        let Missing::Present(ext) = self.hot(slot).individual_ext() else {
            violation!(clause = "REP.2", "a cell given what only an individual keeps", slot = slot.get());
        };
        let mut r = self.ext.list(ext, list);
        let chunk = self.chunk(slot);
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "a cell beyond the table's chunks", slot = slot.get());
        };
        let out = f(arena, &mut r);
        self.ext.set_list(ext, list, r);
        out
    }

    /// A column for a fact only the kind's individuals carry.
    ///
    /// # Errors
    /// A fact that has a column already.
    pub fn add_individual_fact(&mut self, space: &mut AddressSpace, name: &'static str) -> Result<FactColumn, String> {
        self.ext.add_facet(space, name)
    }

    /// An individual's fact; a cell carries none.
    pub fn fact(&self, slot: Slot, column: FactColumn) -> Missing<i64> {
        match self.hot(slot).individual_ext() {
            Missing::Present(ext) => self.ext.fact(ext, column),
            Missing::Absent => violation!(clause = "REP.33", "an individual's fact read on a cell", slot = slot.get()),
        }
    }

    pub fn write_fact(&mut self, slot: Slot, column: FactColumn, value: i64) {
        let Missing::Present(ext) = self.hot(slot).individual_ext() else {
            violation!(clause = "REP.33", "an individual's fact written on a cell", slot = slot.get());
        };
        self.ext.write_fact(ext, column, value);
    }

    /// Marks a row of weight one an individual, with an extension row of its own.
    pub fn make_individual(&mut self, slot: Slot) {
        let mut hot = self.hot(slot);
        if hot.weight() != Weight::new(1) || hot.is_individual() {
            violation!(clause = "REP.2", "an individual made of a cell of many, or twice", slot = slot.get());
        }
        let ext = self.ext.add(slot);
        hot.make_individual(ext);
        self.hot.set(slot, hot);
    }

    /// A cell's due-day run head.
    pub fn run_head(&self, slot: Slot) -> RunHead {
        self.live(slot);
        read(&self.runs, slot).wide()
    }

    pub fn set_run_head(&mut self, slot: Slot, head: RunHead) {
        self.live(slot);
        self.runs.set(slot, CellRunHead::narrow(head));
    }

    /// Closes the gaps in a chunk's arena: every live cell's lists in slot order, then every individual's
    /// extension lists, each reference rewritten where it now lies.
    pub fn compact(&mut self, chunk: usize, scratch: &mut Region<u64, B>) {
        let per = at(Slot::new(self.table.rows_per_chunk()));
        let cells: Vec<Slot> = self.slots().filter(|s| at(*s) / per == chunk).collect();
        let exts: Vec<Slot> = self.ext.slots().filter(|e| at(self.ext.owner(*e)) / per == chunk).collect();
        let mut refs: Vec<ListRef> = Vec::new();
        for s in &cells {
            refs.extend(CellList::ALL.iter().map(|l| self.list(*s, *l)));
        }
        for e in &exts {
            refs.extend([ExtList::Lots, ExtList::NamedUnits].map(|l| self.ext.list(*e, l)));
        }
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "a chunk the table does not have", chunk = chunk);
        };
        arena.compact(refs.as_mut_slice(), scratch);
        let mut moved = refs.into_iter();
        for s in &cells {
            for l in CellList::ALL {
                let Some(full) = moved.next() else { return };
                let mut compact = read(self.list_column(l), *s);
                if let Some(arena) = self.arenas.get_mut(chunk) {
                    arena.store(Self::owner(*s, l), &mut compact, full);
                }
                if let Some(c) = self.lists.get_mut(l.place()) {
                    c.set(*s, compact);
                }
            }
        }
        for e in &exts {
            for l in [ExtList::Lots, ExtList::NamedUnits] {
                let Some(full) = moved.next() else { return };
                self.ext.set_list(*e, l, full);
            }
        }
    }

    /// Chunks the table's rows span.
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.arenas.len()
    }

    /// The cell's steps at the given levels, in the kind's position order: each position's total against the total
    /// of its scale, a standing rate's scale being the rate per member times the weight.
    #[clause("REP.4", "REP.20")]
    #[must_use]
    pub fn steps(&self, slot: Slot, kind: &PopKindDecl, levels: &[u8]) -> Vec<Step> {
        let weight = i128::from(self.weight(slot).get());
        kind.positions
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let scale = match p.scale {
                    Scale::Position(j) => Missing::Present(i128::from(self.position(slot, j))),
                    Scale::Rate(r) => match self.rate(slot, r) {
                        Missing::Present(v) => Missing::Present(i128::from(v) * weight),
                        Missing::Absent => Missing::Absent,
                    },
                };
                let base = p.steps.step_scaled(self.position(slot, i), scale);
                let Some(level) = levels.get(i) else {
                    violation!(clause = "REP.4", "a position with no current level", position = i);
                };
                StepTable::at_level(base, *level)
            })
            .collect()
    }

    /// The cell's landing key, recomputed from its key, positions and kink signature.
    #[clause("REP.8")]
    #[must_use]
    pub fn landing(&self, slot: Slot, kind: &PopKindDecl, levels: &[u8]) -> u64 {
        landing_key(self.hot(slot).key_id, &self.steps(slot, kind, levels), &self.sig(slot))
    }

    /// Writes the cell's landing key and its leading steps from its state now.
    pub fn rekey(&mut self, slot: Slot, kind: &PopKindDecl, levels: &[u8]) {
        let steps = self.steps(slot, kind, levels);
        let mut hot = self.hot(slot);
        hot.landing_key = landing_key(hot.key_id, &steps, &self.sig(slot));
        hot.step_vec_lo = [Step::MISSING.get(); HOT_STEPS];
        for (lo, s) in hot.step_vec_lo.iter_mut().zip(&steps) {
            *lo = s.get();
        }
        self.hot.set(slot, hot);
    }

    /// The key identity a cell holds, rewritten when its key changes.
    pub fn set_key(&mut self, slot: Slot, key: KeyId) {
        let mut hot = self.hot(slot);
        hot.key_id = key;
        self.hot.set(slot, hot);
    }

    /// A cell's weight changed; one whose last member leaves ends instead.
    pub fn set_weight(&mut self, slot: Slot, w: Weight) {
        let mut hot = self.hot(slot);
        if w == Weight::new(0) {
            violation!(clause = "REP.17", "a cell left with no members", slot = slot.get());
        }
        if hot.is_individual() && w != Weight::new(1) {
            violation!(clause = "REP.2", "an individual given a weight other than one", slot = slot.get());
        }
        hot.set_weight(w);
        self.hot.set(slot, hot);
    }
}

impl Layout {
    fn of(kind: &PopKindDecl) -> Layout {
        Layout { positions: kind.positions.len(), profiles: ProfileLayout::new(&kind.groups) }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::register::values::Partition;
    use phx_core::{
        GroupDecl, KeyAttrDecl, KinkRegistry, PopEntry, PopItem, PositionDecl, PositionOf, ProfileComponent, RateDecl,
        RoleDecl, RunHead, ScaleRef, Weight,
    };
    use phx_id::{Day, PartyId, Slot, TableId};
    use phx_num::Missing;
    use phx_store::{AddressSpace, HeapBacking, Region};

    use super::{CellList, CellTable, NewCell};
    use crate::key::KeyId;
    use crate::kind::PopKindDecl;
    use crate::profile::Profile;
    use crate::steps::{Step, StepTable};

    type Heap = HeapBacking;

    const HH: &str = "household";
    const SKILL: &[ProfileComponent] = &[ProfileComponent { name: "skill", values: 5 }];

    fn entry(item: PopItem) -> PopEntry {
        PopEntry { system: "DEM", kind: HH, item }
    }

    fn position(name: &'static str, scale: ScaleRef) -> PopItem {
        PopItem::Position(PositionDecl {
            name,
            unit: "money",
            of: PositionOf::Member,
            scale,
            steps: "REP.s",
            clause: "REP.20",
        })
    }

    fn steps(_: &'static str) -> Result<StepTable, String> {
        StepTable::new(&Partition { exp: 2, bounds: [50, 100, 200].into() })
    }

    /// A kind of `positions` positions, each measured against the spending rate, `rates` rates and `reviews` review
    /// kinds, with one profile group.
    fn kind(positions: usize, rates: usize, reviews: usize) -> PopKindDecl {
        let mut e = vec![
            entry(PopItem::Role(RoleDecl { name: "adult", clause: "REP.26" })),
            entry(PopItem::KeyAttr(KeyAttrDecl { name: "region", values: 20, clause: "REP.19" })),
            entry(PopItem::ProfileGroup(GroupDecl {
                name: "LAB.adult",
                role: "adult",
                components: SKILL,
                clause: "REP.32",
            })),
        ];
        for i in 0..rates {
            let name: &'static str = Box::leak(format!("R.{i:02}").into_boxed_str());
            e.push(entry(PopItem::StandingRate(RateDecl { name, unit: "money/day", clause: "REP.20" })));
        }
        for i in 0..positions {
            let name: &'static str = Box::leak(format!("P.{i:02}").into_boxed_str());
            e.push(entry(position(name, ScaleRef::Rate("R.00"))));
        }
        for i in 0..reviews {
            let name: &'static str = Box::leak(format!("D.{i:02}").into_boxed_str());
            e.push(entry(PopItem::ReviewKind(name)));
        }
        PopKindDecl::compile(HH, &e, &KinkRegistry::default(), &steps).unwrap()
    }

    fn table(k: &PopKindDecl) -> (AddressSpace, CellTable<Heap>) {
        let mut space = AddressSpace::empty();
        let t = CellTable::new(&mut space, k, TableId::new(1), 1_000, 16);
        (space, t)
    }

    fn cell(space: &mut AddressSpace, t: &mut CellTable<Heap>, k: &PopKindDecl, party: u64, weight: u32) -> Slot {
        let totals: Vec<i64> = (0..k.positions.len()).map(|i| i64::try_from(i).unwrap() * 100).collect();
        let mut profile = Profile::empty(t.profile_layout());
        profile.add(t.profile_layout(), 0, 2, weight);
        let new = NewCell {
            party: PartyId::new(party),
            created: Day::new(9),
            weight: Weight::new(weight),
            key: KeyId::new(0),
            positions: &totals,
            profile: &profile,
        };
        t.add(space, new, k, &vec![0; k.positions.len()])
    }

    #[test]
    fn cells_keep_totals_rates_and_profiles() {
        let k = kind(5, 2, 1);
        let (mut space, mut t) = table(&k);
        let s = cell(&mut space, &mut t, &k, 7, 40);
        assert_eq!((t.party(s), t.weight(s), t.created(s)), (PartyId::new(7), Weight::new(40), Day::new(9)));
        assert_eq!((0..5).map(|i| t.position(s, i)).collect::<Vec<_>>(), [0, 100, 200, 300, 400]);
        t.set_position(s, 1, -5);
        t.set_position(s, 4, 9);
        assert_eq!(
            (t.position(s, 1), t.position(s, 4), t.hot(s).lead[1]),
            (-5, 9, -5),
            "leading totals in the hot record"
        );
        assert_eq!(t.rate(s, 0), Missing::Absent, "a rate not yet decided");
        t.set_rate(s, 0, Missing::Present(250));
        t.set_rate(s, 1, Missing::Present(-3));
        assert_eq!((t.rate(s, 0), t.rate(s, 1)), (Missing::Present(250), Missing::Present(-3)), "a rate kept apart");
        t.set_rate(s, 1, Missing::Present(i64::from(u32::MAX) + 9));
        assert_eq!(t.rate(s, 1), Missing::Present(i64::from(u32::MAX) + 9));
        t.set_rate(s, 1, Missing::Absent);
        assert_eq!((t.rate(s, 1), t.words(s, CellList::Rates).len()), (Missing::Absent, 0));
        assert_eq!(t.exposure(s, 0), Missing::Absent, "no exposure before attention");
        let mut p = t.profile(s);
        p.remove(0, 2, 10);
        p.add(t.profile_layout(), 0, 4, 10);
        t.set_profile(s, &p);
        assert_eq!((t.profile(s).count(0, 4), t.profile(s).members(0)), (10, 40));
        let levels = vec![0_u8; k.positions.len()];
        t.set_rate(s, 0, Missing::Present(1));
        t.rekey(s, &k, &levels);
        // Scale: a rate of 1 per member over 40 members is 40; totals 0, -5, 200, 300, 9 over it.
        let steps: Vec<u16> = t.steps(s, &k, &levels).iter().map(|x| x.get()).collect();
        assert_eq!(steps, [0, 0, 3, 3, 0]);
        assert_eq!(t.hot(s).landing_key, t.landing(s, &k, &levels), "the key written is the key recomputed");
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.position(s, 5))).is_err());
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.set_weight(s, Weight::new(0)))).is_err());
        t.set_rate(s, 0, Missing::Absent);
        assert!(t.steps(s, &k, &levels).iter().all(|x| *x == Step::MISSING), "no scale, no step");
        assert_ne!(t.hot(s).landing_key, t.landing(s, &k, &levels), "a stale key is seen");
    }

    #[test]
    fn an_individual_keeps_its_own_lists() {
        let k = kind(1, 1, 0);
        let (mut space, mut t) = table(&k);
        let many = cell(&mut space, &mut t, &k, 1, 3);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.make_individual(many))).is_err());
        let one = cell(&mut space, &mut t, &k, 2, 1);
        t.make_individual(one);
        t.edit_ext(one, crate::individual::ExtList::Lots, |a, r| a.append(r, &[5, 6]));
        assert_eq!(t.ext_words(one, crate::individual::ExtList::Lots), [5, 6]);
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.set_weight(one, Weight::new(2)))).is_err());
    }

    #[test]
    fn run_segment_contiguous_after_append_and_compact() {
        let k = kind(1, 1, 0);
        let (mut space, mut t) = table(&k);
        let cells: Vec<Slot> = (0..4).map(|i| cell(&mut space, &mut t, &k, 10 + i, 2)).collect();
        // Each cell's dated rows open its list, their segment marked by its run head; undated rows follow, and lists
        // grow past their room, move, and leave dead words behind.
        let mark = |n: usize| 1_000 * (u64::try_from(n).unwrap() + 1);
        for (n, s) in cells.iter().enumerate() {
            let dated: Vec<u64> = (0..6).map(|w| mark(n) + w).collect();
            t.edit_list(*s, CellList::Rows, |a, r| a.append(r, &dated));
            t.set_run_head(*s, RunHead { next_due: 30, offset: 0, len: 6 });
        }
        for round in 0..20_u64 {
            for s in &cells {
                t.edit_list(*s, CellList::Rows, |a, r| a.append(r, &[round, round]));
                t.edit_list(*s, CellList::Holdings, |a, r| a.append(r, &[round]));
            }
        }
        let before: Vec<Vec<u64>> = cells.iter().map(|s| t.words(*s, CellList::Rows).to_vec()).collect();
        let profiles: Vec<Profile> = cells.iter().map(|s| t.profile(*s)).collect();
        let dead = t.arenas[0].dead_words();
        assert!(dead > 0, "growth left dead words to close");
        let mut scratch: Region<u64, Heap> = Region::reserve(&mut AddressSpace::empty(), 1 << 16);
        t.compact(0, &mut scratch);
        assert_eq!(t.arenas[0].dead_words(), 0);
        for (n, s) in cells.iter().enumerate() {
            let rows = t.words(*s, CellList::Rows);
            assert_eq!(rows, before[n].as_slice(), "every word where it was in its list");
            let head = t.run_head(*s);
            let (from, to) = (usize::try_from(head.offset).unwrap(), usize::try_from(head.offset + head.len).unwrap());
            let segment = rows.get(from..to).unwrap();
            assert!(segment.iter().all(|w| *w >= mark(n)), "the dated rows still one segment");
            assert_eq!(t.profile(*s), profiles[n]);
            assert_eq!(t.words(*s, CellList::Holdings).len(), 20);
        }
    }

    #[test]
    fn household_record_within_budget() {
        // Stage 0's household as the budget itemises it: three leading and nine further positions, fourteen standing
        // rates, ten review kinds, one word of signature.
        let k = kind(12, 14, 10);
        let (_, t) = table(&k);
        assert!(t.bytes_per_row() <= 464, "{} bytes a household row", t.bytes_per_row());
        assert_eq!(t.bytes_per_row(), 8 + 4 + 64 + 8 + 9 * 8 + 14 * 4 + 10 * 8 + 10 * 8 + 4 * 8);
    }
}
