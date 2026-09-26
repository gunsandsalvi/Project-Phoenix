use phx_core::schema::FactColumn;
use phx_core::{RunHead, Weight};
use phx_id::{Day, PartyId, Slot, TableId};
use phx_macros::clause;
use phx_num::consts::ABSENT_I64;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::consts::ARENA_RESERVED_WORDS;
use phx_store::{AddressSpace, Backing, CellListRef, ChunkArena, Column, ListRef, Region, SystemBacking, Table};

use crate::kind::PopKindDecl;

/// The lists an agent keeps in its chunk's arena: the ledger's relationship rows, holdings, lots and named units, its
/// persons and its attachments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentList {
    Rows,
    Holdings,
    Lots,
    NamedUnits,
    Persons,
    Attachments,
}

impl AgentList {
    pub const ALL: [AgentList; 6] = [
        AgentList::Rows,
        AgentList::Holdings,
        AgentList::Lots,
        AgentList::NamedUnits,
        AgentList::Persons,
        AgentList::Attachments,
    ];

    fn place(self) -> usize {
        let Some(at) = AgentList::ALL.iter().position(|l| *l == self) else {
            violation!(clause = "REP.1", "an agent list the table does not keep");
        };
        at
    }
}

/// An agent's due-day run head in eight bytes: the earliest day a dated row can fall due, and where its dated rows lie
/// in its relationship rows. An agent's rows are few, so their offset and length fit sixteen bits; one that did not is
/// a contract violation calling for a wider layout.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Pod)]
pub struct AgentRunHead {
    pub next_due: u32,
    pub offset: u16,
    pub len: u16,
}

impl AgentRunHead {
    pub const EMPTY: AgentRunHead = AgentRunHead { next_due: 0, offset: 0, len: 0 };

    fn wide(self) -> RunHead {
        RunHead { next_due: self.next_due, offset: u32::from(self.offset), len: u32::from(self.len) }
    }

    fn narrow(head: RunHead) -> AgentRunHead {
        let (Ok(offset), Ok(len)) = (u16::try_from(head.offset), u16::try_from(head.len)) else {
            capacity_exceeded!("an agent's dated rows for its run head", u16::MAX, head.offset + head.len);
        };
        AgentRunHead { next_due: head.next_due, offset, len }
    }
}

/// A new agent: its party, the day it began, its multiplicity and each of its kind's attributes in order.
#[derive(Clone, Copy, Debug)]
pub struct NewAgent<'a> {
    pub party: PartyId,
    pub created: Day,
    pub multiplicity: Weight,
    pub attrs: &'a [u32],
}

/// The table of one population kind: every agent's party, the day it began, its multiplicity, its attributes, its
/// facts, the due-day run head, and its lists in its chunk's arena.
#[clause("REP.1", "REP.41", "REP.26")]
#[derive(Debug, phx_macros::Saved)]
pub struct AgentTable<B: Backing = SystemBacking> {
    kind: &'static str,
    table: Table<B>,
    party: Column<u64, B>,
    created: Column<u32, B>,
    multiplicity: Column<u32, B>,
    attrs: Vec<Column<u32, B>>,
    /// The kind's attributes by name, each the column of `attrs` at its place, which a handler reads as it reads a
    /// large firm's key facts.
    attr_names: Vec<&'static str>,
    /// The kind's positions by name, each the column of `facts` at its place.
    positions: Vec<&'static str>,
    facts: Vec<Column<i64, B>>,
    lists: Vec<Column<CellListRef, B>>,
    runs: Column<AgentRunHead, B>,
    arenas: Vec<ChunkArena<B>>,
    /// The agents live, the real parties they stand for and those parties' persons, kept as agents begin, change and
    /// end, so no day need count them by visiting each.
    agents: u64,
    twins: u64,
    persons_held: u64,
    /// The day's agents added, removed or changed, whose hazards the world draws again.
    #[saved(skip)]
    changed: Vec<Slot>,
    /// The handler running on the agents due, whose reads and writes are checked against its declaration, and what
    /// it read or wrote that it does not declare.
    #[saved(skip)]
    reader: Option<phx_core::ColumnTrace>,
    #[saved(skip)]
    undeclared: usize,
    /// The positions the handlers moved to a new value, and how many times, since last taken.
    #[saved(skip)]
    moved: Vec<(&'static str, u64)>,
}

#[inline]
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

#[inline]
fn read<T: phx_store::Pod, B: Backing>(column: &Column<T, B>, slot: Slot) -> T {
    let Some(v) = column.get(slot) else {
        violation!(clause = "PTY.10", "a read of an agent row no party holds", slot = slot.get());
    };
    v
}

fn word32(n: usize) -> u32 {
    let Ok(w) = u32::try_from(n) else {
        capacity_exceeded!("words of an agent's list", u32::MAX, n);
    };
    w
}

impl<B: Backing> AgentTable<B> {
    /// An empty table for a population kind, of at most `max_rows` agents.
    pub fn new(
        space: &mut AddressSpace,
        kind: &PopKindDecl,
        id: TableId,
        max_rows: u32,
        rows_per_chunk: u32,
    ) -> AgentTable<B> {
        let table: Table<B> = Table::new(space, id, max_rows, rows_per_chunk);
        AgentTable {
            kind: kind.kind,
            party: table.column(space),
            created: table.column(space),
            multiplicity: table.column(space),
            attrs: kind.attrs.iter().map(|_| table.column(space)).collect(),
            attr_names: kind.attrs.iter().map(|a| a.item.name).collect(),
            positions: kind.positions.iter().map(|p| p.item.name).collect(),
            facts: kind.positions.iter().map(|_| table.column(space)).collect(),
            lists: AgentList::ALL.iter().map(|_| table.column(space)).collect(),
            runs: table.column(space),
            arenas: Vec::new(),
            agents: 0,
            twins: 0,
            persons_held: 0,
            changed: Vec::new(),
            reader: None,
            undeclared: 0,
            moved: Vec::new(),
            table,
        }
    }

    /// The agents added, removed or changed since the last call, each once, in slot order.
    pub fn take_changed(&mut self) -> Vec<Slot> {
        let mut out = std::mem::take(&mut self.changed);
        out.sort_unstable();
        out.dedup();
        out
    }

    /// An agent whose hazards read something that changed, to be drawn again.
    pub fn mark_changed(&mut self, slot: Slot) {
        self.changed.push(slot);
    }

    pub fn id(&self) -> TableId {
        self.table.id
    }

    #[must_use]
    pub fn kind(&self) -> &'static str {
        self.kind
    }

    /// Bytes an agent's row takes in the table's columns, its arena lists apart.
    #[must_use]
    pub fn bytes_per_row(&self) -> usize {
        size_of::<u64>()
            + 2 * size_of::<u32>()
            + size_of::<AgentRunHead>()
            + self.attrs.len() * size_of::<u32>()
            + self.facts.len() * size_of::<i64>()
            + self.lists.len() * size_of::<CellListRef>()
    }

    /// A row for a new agent, with its attributes; its facts absent and its lists empty.
    pub fn add(&mut self, space: &mut AddressSpace, agent: NewAgent<'_>) -> Slot {
        if agent.multiplicity == Weight::new(0) {
            violation!(clause = "REP.17", "an agent of no twins", party = agent.party.get());
        }
        if agent.attrs.len() != self.attrs.len() {
            violation!(
                clause = "REP.41",
                "an agent with another count of attributes than its kind",
                given = agent.attrs.len()
            );
        }
        let slot = self.table.slots.alloc();
        put(&mut self.party, slot, agent.party.get());
        put(&mut self.created, slot, agent.created.get());
        put(&mut self.multiplicity, slot, agent.multiplicity.get());
        for (column, v) in self.attrs.iter_mut().zip(agent.attrs) {
            put(column, slot, *v);
        }
        for column in &mut self.facts {
            put(column, slot, ABSENT_I64);
        }
        for column in &mut self.lists {
            put(column, slot, CellListRef::EMPTY);
        }
        put(&mut self.runs, slot, AgentRunHead::EMPTY);
        let chunk = self.chunk(slot);
        while self.arenas.len() <= chunk {
            self.arenas.push(ChunkArena::new(space, ARENA_RESERVED_WORDS));
        }
        // A reused slot's arena lists were cleared when its last agent ended.
        self.changed.push(slot);
        self.agents += 1;
        self.twins += u64::from(agent.multiplicity.get());
        slot
    }

    /// Frees an agent that has ended with no contracts or holdings left; its persons and attachments go with it.
    pub fn remove(&mut self, slot: Slot) {
        self.live(slot);
        for list in [AgentList::Rows, AgentList::Holdings, AgentList::Lots, AgentList::NamedUnits] {
            if self.list(slot, list).len != 0 {
                violation!(clause = "PTY.10", "an agent removed with contracts still in its arena", slot = slot.get());
            }
        }
        let (k, persons) = (u64::from(self.multiplicity(slot).get()), self.persons_of(slot));
        for list in AgentList::ALL {
            self.edit_list(slot, list, ChunkArena::clear);
        }
        self.table.slots.release(slot);
        self.changed.push(slot);
        self.agents -= 1;
        self.twins -= k;
        self.persons_held -= k * persons;
    }

    /// The agents live in the table.
    #[must_use]
    pub fn agents(&self) -> u64 {
        self.agents
    }

    /// The real parties the table's agents stand for: their multiplicities summed.
    #[must_use]
    pub fn twins(&self) -> u64 {
        self.twins
    }

    /// Those parties' persons: each agent's persons times its multiplicity, summed.
    #[must_use]
    pub fn persons_held(&self) -> u64 {
        self.persons_held
    }

    fn persons_of(&self, slot: Slot) -> u64 {
        phx_rand::float::len_u64(self.persons(slot).len())
    }

    fn live(&self, slot: Slot) {
        if !self.table.slots.is_live(slot) {
            violation!(clause = "PTY.10", "a read of an agent row no party holds", slot = slot.get());
        }
    }

    /// Slots the table has handed out, live or freed: every agent's slot lies below it.
    #[must_use]
    pub fn high_water(&self) -> u32 {
        self.table.slots.high_water()
    }

    /// Whether an agent holds the slot.
    #[must_use]
    pub fn is_live(&self, slot: Slot) -> bool {
        self.table.slots.is_live(slot)
    }

    /// The rows agents hold, in slot order.
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
        PartyId::new(read(&self.party, slot))
    }

    pub fn created(&self, slot: Slot) -> Day {
        self.live(slot);
        Day::new(read(&self.created, slot))
    }

    /// The count of identical real parties the agent stands for.
    #[clause("REP.1", "REP.17")]
    pub fn multiplicity(&self, slot: Slot) -> Weight {
        self.live(slot);
        Weight::new(read(&self.multiplicity, slot))
    }

    /// One twin taken from an agent to stand alone, as the player's household is: its multiplicity one less, never
    /// none.
    #[clause("REP.1", "REP.17")]
    pub fn take_twin(&mut self, slot: Slot) {
        let k = self.multiplicity(slot).get();
        let Some(left) = k.checked_sub(1).filter(|l| *l > 0) else {
            violation!(clause = "REP.17", "a twin taken from an agent of one", slot = slot.get());
        };
        self.multiplicity.set(slot, left);
        self.twins -= 1;
        self.persons_held -= self.persons_of(slot);
    }

    /// An attribute's value, by its place among the kind's.
    #[must_use]
    pub fn attr(&self, slot: Slot, i: usize) -> u32 {
        self.live(slot);
        let Some(c) = self.attrs.get(i) else {
            violation!(clause = "REP.41", "an attribute the kind does not hold", attr = i);
        };
        read(c, slot)
    }

    pub fn set_attr(&mut self, slot: Slot, i: usize, value: u32) {
        self.live(slot);
        let Some(c) = self.attrs.get_mut(i) else {
            violation!(clause = "REP.41", "an attribute the kind does not hold", attr = i);
        };
        c.set(slot, value);
        self.changed.push(slot);
    }

    /// Every attribute's value, in the kind's order.
    #[must_use]
    pub fn attrs(&self, slot: Slot) -> Vec<u32> {
        (0..self.attrs.len()).map(|i| self.attr(slot, i)).collect()
    }

    /// The agent's persons, a word each.
    #[must_use]
    pub fn persons(&self, slot: Slot) -> &[u64] {
        self.words(slot, AgentList::Persons)
    }

    /// The agent's persons written anew.
    pub fn set_persons(&mut self, slot: Slot, words: &[u64]) {
        let k = u64::from(self.multiplicity(slot).get());
        self.persons_held = self.persons_held - k * self.persons_of(slot) + k * phx_rand::float::len_u64(words.len());
        self.edit_list(slot, AgentList::Persons, |arena, r| {
            arena.clear(r);
            arena.append(r, words);
        });
        self.changed.push(slot);
    }

    /// The agent's attachments, a word each.
    #[must_use]
    pub fn attachments(&self, slot: Slot) -> &[u64] {
        self.words(slot, AgentList::Attachments)
    }

    pub fn set_attachments(&mut self, slot: Slot, words: &[u64]) {
        self.edit_list(slot, AgentList::Attachments, |arena, r| {
            arena.clear(r);
            arena.append(r, words);
        });
    }

    /// The chunk a slot lies in.
    #[must_use]
    pub fn chunk_of(&self, slot: Slot) -> usize {
        self.chunk(slot)
    }

    fn owner(slot: Slot, list: AgentList) -> u32 {
        let Some(o) =
            slot.get().checked_mul(word32(AgentList::ALL.len())).and_then(|o| o.checked_add(word32(list.place())))
        else {
            capacity_exceeded!("agent list owners", u32::MAX, slot.get());
        };
        o
    }

    fn chunk(&self, slot: Slot) -> usize {
        at(slot) / at(Slot::new(self.table.rows_per_chunk()))
    }

    fn arena(&self, slot: Slot) -> &ChunkArena<B> {
        let Some(a) = self.arenas.get(self.chunk(slot)) else {
            violation!(clause = "PTY.10", "an agent beyond the table's chunks", slot = slot.get());
        };
        a
    }

    fn list_column(&self, list: AgentList) -> &Column<CellListRef, B> {
        let Some(c) = self.lists.get(list.place()) else {
            violation!(clause = "REP.3", "an agent list the table does not keep", list = list.place());
        };
        c
    }

    /// An agent's list, its full reference.
    pub fn list(&self, slot: Slot, list: AgentList) -> ListRef {
        self.live(slot);
        self.arena(slot).resolve(Self::owner(slot, list), read(self.list_column(list), slot))
    }

    /// An agent's list, as its arena's words.
    #[must_use]
    pub fn words(&self, slot: Slot, list: AgentList) -> &[u64] {
        let r = self.list(slot, list);
        self.arena(slot).read(r)
    }

    /// Single words written over agents' lists, each chunk's on its own worker; the lists keep their places and
    /// lengths, so no reference moves.
    pub fn overwrite_words(&mut self, pool: Option<&phx_exec::Pool>, list: AgentList, writes: &[(Slot, usize, u64)]) {
        let mut chunks = phx_core::kind_tables::deal_writes(writes, self.table.rows_per_chunk(), self.arenas.len());
        let Some(column) = self.lists.get(list.place()) else {
            violation!(clause = "REP.3", "an agent list the table does not keep", list = list.place());
        };
        let table = &self.table;
        phx_exec::pool::each(pool, self.arenas.iter_mut().zip(chunks.iter_mut()), |(arena, chunk)| {
            phx_core::kind_tables::written_once(chunk);
            for &(slot, word, value) in chunk.iter() {
                if !table.slots.is_live(slot) {
                    violation!(clause = "PTY.10", "a write to an agent no party holds", slot = slot.get());
                }
                let r = arena.resolve(Self::owner(slot, list), read(column, slot));
                let Some(target) = arena.read_mut(r).get_mut(word) else {
                    violation!(clause = "REG.14", "a word written beyond an agent's list", at = word);
                };
                *target = value;
            }
        });
    }

    /// Edits an agent's list in its arena, writing back its reference, which an edit may move or lengthen past its
    /// compact form.
    pub fn edit_list<R>(
        &mut self,
        slot: Slot,
        list: AgentList,
        f: impl FnOnce(&mut ChunkArena<B>, &mut ListRef) -> R,
    ) -> R {
        let mut full = self.list(slot, list);
        let chunk = self.chunk(slot);
        let owner = Self::owner(slot, list);
        let mut compact = read(self.list_column(list), slot);
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "an agent beyond the table's chunks", slot = slot.get());
        };
        let out = f(arena, &mut full);
        arena.store(owner, &mut compact, full);
        if let Some(c) = self.lists.get_mut(list.place()) {
            c.set(slot, compact);
        }
        out
    }

    /// A column for a fact every agent of the kind holds.
    ///
    /// # Errors
    /// Never as built; a fact's name is checked by the schema that calls it.
    pub fn add_fact(&mut self, space: &mut AddressSpace) -> Result<FactColumn, String> {
        let mut column: Column<i64, B> = self.table.column(space);
        for _ in 0..self.high_water() {
            column.push(ABSENT_I64);
        }
        let index = word32(self.facts.len());
        self.facts.push(column);
        Ok(FactColumn::new(index))
    }

    /// An agent's fact.
    pub fn fact(&self, slot: Slot, column: FactColumn) -> Missing<i64> {
        self.live(slot);
        let Some(c) = self.facts.get(phx_rand::float::index(u64::from(column.index()))) else {
            violation!(clause = "REP.20", "a fact the table does not keep", column = column.index());
        };
        let v = read(c, slot);
        if v == ABSENT_I64 { Missing::Absent } else { Missing::Present(v) }
    }

    pub fn write_fact(&mut self, slot: Slot, column: FactColumn, value: i64) {
        self.live(slot);
        let Some(c) = self.facts.get_mut(phx_rand::float::index(u64::from(column.index()))) else {
            violation!(clause = "REP.20", "a fact the table does not keep", column = column.index());
        };
        if value == ABSENT_I64 {
            violation!(clause = "NUM.8", "the absent marker written as an agent's fact");
        }
        c.set(slot, value);
    }

    /// A position's column, if the kind holds it.
    #[must_use]
    pub fn position(&self, name: &str) -> Option<FactColumn> {
        self.positions.iter().position(|p| *p == name).map(|i| FactColumn::new(word32(i)))
    }

    fn named(&self, name: &str) -> FactColumn {
        let Some(c) = self.position(name) else {
            violation!(clause = "REP.20", "a position the kind does not hold");
        };
        c
    }

    /// An agent's due-day run head.
    pub fn run_head(&self, slot: Slot) -> RunHead {
        self.live(slot);
        read(&self.runs, slot).wide()
    }

    pub fn set_run_head(&mut self, slot: Slot, head: RunHead) {
        self.live(slot);
        self.runs.set(slot, AgentRunHead::narrow(head));
    }

    /// Closes the gaps in a chunk's arena: every live agent's lists in slot order, each reference rewritten where it
    /// now lies.
    pub fn compact(&mut self, chunk: usize, scratch: &mut Region<u64, B>) {
        // Only the chunk's own slots are read, never the whole table.
        let per = self.table.rows_per_chunk();
        let Some(first) = word32(chunk).checked_mul(per) else {
            capacity_exceeded!("an agent table's rows", u32::MAX, chunk);
        };
        let Some(end) = first.checked_add(per) else {
            capacity_exceeded!("an agent table's rows", u32::MAX, chunk);
        };
        let handed = self.high_water();
        let agents: Vec<Slot> =
            (first..end).filter(|s| *s < handed).map(Slot::new).filter(|s| self.is_live(*s)).collect();
        let mut refs: Vec<ListRef> = Vec::new();
        for s in &agents {
            refs.extend(AgentList::ALL.iter().map(|l| self.list(*s, *l)));
        }
        let Some(arena) = self.arenas.get_mut(chunk) else {
            violation!(clause = "PTY.10", "a chunk the table does not have", chunk = chunk);
        };
        arena.compact(refs.as_mut_slice(), scratch);
        let mut moved = refs.into_iter();
        for s in &agents {
            for l in AgentList::ALL {
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
    }

    /// Compacts every chunk whose arena's dead words have passed the declared share; returns the chunks compacted.
    pub fn compact_due(&mut self) -> u64 {
        let due: Vec<usize> =
            (0..self.arenas.len()).filter(|c| self.arenas.get(*c).is_some_and(ChunkArena::needs_compaction)).collect();
        for chunk in &due {
            let used = self.arenas.get(*chunk).map_or(0, ChunkArena::used_words);
            // A transient scratch outside the world's reservations, as large as the arena, unmapped as the
            // compaction ends.
            let mut scratch: Region<u64, B> =
                Region::reserve(&mut phx_store::AddressSpace::empty(), phx_rand::float::index(u64::from(used)));
            self.compact(*chunk, &mut scratch);
        }
        phx_rand::float::len_u64(due.len())
    }

    /// Chunks the table's rows span.
    #[must_use]
    pub fn chunks(&self) -> usize {
        self.arenas.len()
    }

    /// Words every chunk's arena holds, live or dead.
    #[must_use]
    pub fn arena_words(&self) -> u64 {
        self.arenas.iter().map(|a| u64::from(a.used_words())).sum()
    }
}

/// A new agent begun: a party of the directory's next identity at a new row of the table.
#[clause("PTY.9", "REP.1")]
pub fn begin<B: Backing>(
    table: &mut AgentTable<B>,
    directory: &mut phx_core::Directory,
    space: &mut AddressSpace,
    (created, multiplicity, attrs): (Day, Weight, &[u32]),
) -> (Slot, PartyId) {
    let party = PartyId::new(directory.next());
    let slot = table.add(space, NewAgent { party, created, multiplicity, attrs });
    if directory.begin(phx_id::RowRef { table: table.id(), slot }) != party {
        violation!(clause = "PTY.9", "an agent begun under another identity than the directory's next");
    }
    (slot, party)
}

impl<B: Backing> AgentTable<B> {
    /// Checks the reads and writes of the handler about to run on the agents due, or stops checking.
    pub fn trace(&mut self, reader: Option<phx_core::ColumnTrace>) {
        self.reader = reader;
    }

    /// The reads and writes handlers made that they do not declare, since last taken.
    pub fn take_undeclared(&mut self) -> usize {
        std::mem::take(&mut self.undeclared)
    }

    /// The positions handlers moved to a new value, each with how many times, since last taken.
    pub fn take_moved(&mut self) -> Vec<(&'static str, u64)> {
        std::mem::take(&mut self.moved)
    }

    fn check(&mut self, fact: &str, written: bool) {
        if let Some(r) = self.reader
            && !r.writes.contains(&fact)
            && (written || !r.reads.contains(&fact))
        {
            self.undeclared += 1;
        }
    }
}

/// A handler's rows of a population kind: its agents' positions, by name.
impl<B: Backing> phx_core::FactStore for AgentTable<B> {
    fn read(&mut self, fact: &'static str, slot: Slot) -> Missing<i64> {
        self.check(fact, false);
        if self.position(fact).is_none()
            && let Some(i) = self.attr_names.iter().position(|a| *a == fact)
        {
            return Missing::Present(i64::from(self.attr(slot, i)));
        }
        let column = self.named(fact);
        AgentTable::fact(self, slot, column)
    }

    fn write(&mut self, fact: &'static str, slot: Slot, value: i64) {
        self.check(fact, true);
        let column = self.named(fact);
        if self.fact(slot, column) != Missing::Present(value) {
            phx_core::columns::count_moved(&mut self.moved, fact);
        }
        self.write_fact(slot, column, value);
    }
}
