use std::collections::BTreeMap;

use phx_core::kind_tables::{KindTable, ListKind, NewIndividual};
use phx_core::{AuditStream, Directory, FactColumn, FactDecl, GenReport, Resolved, TableSchema, WriteRecord};
use phx_id::{Day, LineId, PartyId, RowRef, Slot, TableId, TileId};

use crate::algebra::Side;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing, SystemBacking};

use crate::apply::{ApplyAt, DayBook, Holders, Ledger, Located};
use crate::dues::DueReasons;
use crate::fails::Fail;
use crate::holder::{CellHolders, HolderArenas, HolderKeys, HolderTable};
use crate::instruction::{Instruction, InstructionId, LegKind, LegRec, ReasonId};
use crate::instrument::Instruments;
use crate::line::Lines;

/// The world's parties: one kind table for each individual kind, then one agent table for each population kind, each
/// by its place among them, and the directory that finds a party's row and follows an ended party to its successor.
#[derive(Debug)]
pub struct Parties<B: Backing = SystemBacking> {
    tables: Vec<KindTable<B>>,
    cells: Vec<Box<dyn CellHolders>>,
    directory: Directory,
    space: AddressSpace,
}

impl<B: Backing> Parties<B> {
    /// A holder table's place by its kind.
    #[must_use]
    pub fn place(&self, kind: &str) -> u16 {
        let Some(i) = self.holders().position(|t| t.kind() == kind) else {
            violation!(clause = "PTY.9", "a party of a kind the world keeps no table for");
        };
        place16(i)
    }

    fn place_of(&self, table: TableId) -> u16 {
        let Some(i) = self.holders().position(|t| t.table() == table) else {
            violation!(
                clause = "PTY.10",
                "a party whose row is in a table the world does not keep",
                table = table.get()
            );
        };
        place16(i)
    }

    /// Every holder table in place order: the kind tables, then the population's agent tables.
    pub fn holders(&self) -> impl Iterator<Item = &dyn HolderTable> + '_ {
        let kinds = self.tables.iter().map(|t| -> &dyn HolderTable { t });
        kinds.chain(self.cells.iter().map(|c| -> &dyn HolderTable { &**c }))
    }

    /// Every holder table's place.
    pub fn places(&self) -> impl Iterator<Item = u16> + use<B> {
        (0..self.tables.len() + self.cells.len()).map(place16)
    }

    /// A holder table by its place, of either sort.
    #[must_use]
    pub fn holder(&self, place: u16) -> &dyn HolderTable {
        let i = usize::from(place);
        if let Some(t) = self.tables.get(i) {
            return t;
        }
        let Some(c) = i.checked_sub(self.tables.len()).and_then(|j| self.cells.get(j)) else {
            violation!(clause = "PTY.10", "a holder table's place beyond the tables", place = place);
        };
        &**c
    }

    fn holder_mut(&mut self, place: u16) -> &mut dyn HolderTable {
        let i = usize::from(place);
        let kinds = self.tables.len();
        if let Some(t) = self.tables.get_mut(i) {
            return t;
        }
        let Some(c) = i.checked_sub(kinds).and_then(|j| self.cells.get_mut(j)) else {
            violation!(clause = "PTY.10", "a holder table's place beyond the tables", place = place);
        };
        &mut **c
    }

    /// The population's agent tables, in place order after the kind tables.
    #[must_use]
    pub fn cells(&self) -> &[Box<dyn CellHolders>] {
        &self.cells
    }

    /// The first agent table's place.
    #[must_use]
    pub fn first_cell_place(&self) -> u16 {
        place16(self.tables.len())
    }

    /// The agent tables with the directory and the address space, apart, so the population can begin and end its
    /// agents and grow their arenas at once.
    pub fn cells_mut(&mut self) -> (&mut Vec<Box<dyn CellHolders>>, &mut Directory, &mut AddressSpace) {
        (&mut self.cells, &mut self.directory, &mut self.space)
    }

    /// Compacts every holder table's chunks whose arenas' dead words have passed the declared share, at a moment no
    /// list reference is held outside the tables; returns the chunks compacted.
    #[clause("SET.12")]
    pub fn compact_arenas(&mut self) -> u64 {
        let kinds: u64 = self.tables.iter_mut().map(KindTable::compact_due).sum();
        kinds + self.cells.iter_mut().map(|c| c.compact_due()).sum::<u64>()
    }

    /// A party begun in its kind's table, sited on a tile, by a named beginning on a day: its identity is the next the
    /// directory gives, and its row is found from it.
    #[clause("PTY.9")]
    pub fn begin(&mut self, kind: &str, site: TileId, created: Day) -> PartyId {
        self.begin_weighted(kind, site, created, 1)
    }

    /// A party begun standing for `weight` real parties alike: an agent's estate, one for each of its twins.
    #[clause("REP.1", "REP.17")]
    pub fn begin_weighted(&mut self, kind: &str, site: TileId, created: Day, weight: u32) -> PartyId {
        let place = self.place(kind);
        let party = PartyId::new(self.directory.next());
        let Some(table) = self.tables.get_mut(usize::from(place)) else {
            violation!(clause = "PTY.9", "a holder table's place beyond the tables", place = place);
        };
        let slot = table.add(&mut self.space, NewIndividual { party, site, created, weight, types: &[] });
        let begun = self.directory.begin(RowRef { table: table.id(), slot });
        if begun != party {
            violation!(
                clause = "PTY.9",
                "a party begun under another identity than its row holds",
                party = party.get()
            );
        }
        party
    }

    /// A column on a kind's individuals for a fact its systems declare, absent in every row until written.
    ///
    /// # Errors
    /// When the world keeps no table of the kind, the fact is not the kind's, or the kind keeps it already.
    #[clause("PTY.8")]
    pub fn add_facet(&mut self, kind: &str, name: &'static str, fact: &FactDecl) -> Result<(), String> {
        let Some(table) = self.tables.iter_mut().find(|t| t.kind() == kind) else {
            return Err(format!("`{name}` is kept on `{kind}`, a kind of individual the world does not keep"));
        };
        table.add_fact(&mut self.space, name, fact).map(|_| ())
    }

    /// An individual's table, its row and the column of one of its kind's facts.
    fn facet_row(&mut self, party: PartyId, name: &str) -> (&mut KindTable<B>, Slot, FactColumn) {
        let (place, slot) = self.row(party);
        let Some(table) = self.tables.get_mut(usize::from(place)) else {
            violation!(clause = "PTY.8", "a fact of a party that is no individual", party = party.get());
        };
        let Some(column) = table.facet_named(name) else {
            violation!(clause = "PTY.8", "a fact its party's kind does not keep", party = party.get());
        };
        (table, slot, column)
    }

    /// An individual's fact, absent until its writer writes it.
    pub fn fact(&self, party: PartyId, name: &str) -> Missing<i64> {
        let (place, slot) = self.row(party);
        let table = self.table(place);
        let Some(column) = table.facet_named(name) else {
            violation!(clause = "PTY.8", "a fact its party's kind does not keep", party = party.get());
        };
        table.fact(slot, column)
    }

    /// An individual's fact as the opening draws it: one of the opening's writes, before the day's writers run.
    pub fn open_fact(&mut self, party: PartyId, name: &str, value: i64) {
        let (table, slot, column) = self.facet_row(party, name);
        table.write_fact(slot, column, value);
    }

    /// Where a party's row is.
    pub fn row(&self, party: PartyId) -> (u16, Slot) {
        match self.directory.resolve(party) {
            Resolved::Live(_, row) => (self.place_of(row.table), row.slot),
            Resolved::Ended(_) | Resolved::Unknown => {
                violation!(clause = "PTY.10", "a party read that is not live", party = party.get())
            }
        }
    }

    /// The members a party's row stands for on each contract it holds: an agent's multiplicity, one for an individual.
    #[clause("REP.3")]
    #[must_use]
    pub fn unit(&self, party: PartyId) -> u32 {
        let (place, slot) = self.row(party);
        self.holder(place).weight(slot)
    }

    /// Ends an individual whose rows and holdings have all left it: its row freed and its identity ended with no
    /// successor.
    #[clause("PTY.9", "PTY.13")]
    pub fn end(&mut self, party: PartyId, day: Day) {
        let (place, slot) = self.row(party);
        let Some(table) = self.tables.get_mut(usize::from(place)) else {
            violation!(clause = "PTY.9", "a party ended outside the tables of individuals", party = party.get());
        };
        table.remove(slot);
        self.directory.end(party, day, Missing::Absent);
    }

    /// A party's site.
    pub fn site(&self, party: PartyId) -> TileId {
        let (place, slot) = self.row(party);
        self.table(place).site(slot)
    }

    /// A kind table by its place, to change: its rows' facts as their handlers write them.
    pub fn table_mut(&mut self, place: u16) -> &mut KindTable<B> {
        let Some(t) = self.tables.get_mut(usize::from(place)) else {
            violation!(clause = "PTY.10", "a holder table's place beyond the tables", place = place);
        };
        t
    }

    #[must_use]
    pub fn table(&self, place: u16) -> &KindTable<B> {
        let Some(t) = self.tables.get(usize::from(place)) else {
            violation!(clause = "PTY.10", "a holder table's place beyond the tables", place = place);
        };
        t
    }

    /// Every holder table, by its place, as the ledger's families read them.
    #[must_use]
    pub fn arenas(&self) -> Vec<&dyn HolderArenas> {
        self.holders().map(|t| -> &dyn HolderArenas { t }).collect()
    }

    #[must_use]
    pub fn directory(&self) -> &Directory {
        &self.directory
    }

    /// The individual kinds the world keeps tables for, in their places' order.
    pub fn kinds(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.tables.iter().map(KindTable::kind)
    }

    /// The parties of a kind, in their table's order.
    pub fn of_kind(&self, kind: &str) -> impl Iterator<Item = PartyId> + '_ {
        let t = self.table(self.place(kind));
        t.slots().map(move |s| t.party(s))
    }
}

impl<B: Backing> Holders for Parties<B> {
    fn locate(&self, party: PartyId) -> Located {
        match self.directory.resolve(party) {
            Resolved::Live(p, row) => Located::Live { party: p, table: self.place_of(row.table), slot: row.slot },
            Resolved::Ended(_) => Located::Ended,
            Resolved::Unknown => {
                violation!(clause = "PTY.10", "an instruction naming a party that never was", party = party.get())
            }
        }
    }

    fn arenas(&mut self, table: u16) -> &mut dyn HolderArenas {
        self.holder_mut(table)
    }
}

fn place16(i: usize) -> u16 {
    let Ok(place) = u16::try_from(i) else {
        capacity_exceeded!("holder tables", u16::MAX, i);
    };
    place
}

/// The world's books: the ledger and the parties whose rows it moves, and the counterparties' drawn sizes the
/// opening apportions over.
#[clause("REG.4", "SET.1")]
#[derive(Debug)]
pub struct Books<B: Backing = SystemBacking> {
    pub ledger: Ledger<B>,
    pub parties: Parties<B>,
    pub drawn: BTreeMap<String, Vec<(PartyId, u64)>>,
    pub dues: DueReasons,
    opened: u32,
    /// Each line side's members by holder as members last left it, with the side's version then: drawn from again
    /// while the side is unchanged, so members leaving a line of many holders read its side once, not each time.
    pub(crate) leaving: BTreeMap<(LineId, Side), (u64, crate::cleared::Tally)>,
    /// Stage 7's buffers, kept from one day to the next.
    pub(crate) buffers: crate::apply_batch::DayBuffers,
    /// The workers stage 7 reads its shards on, where the world gives it some; without, the shards run in turn.
    pub(crate) pool: Option<std::sync::Arc<phx_exec::Pool>>,
}

/// The opening's instructions feed no audit: day one's audit reads the state they leave.
struct Unaudited;

impl AuditStream for Unaudited {
    fn applied(&mut self, _: u64) {}
    fn touched(&mut self, _: TableId, _: Slot) {}
    fn leg(&mut self, _: u64, _: phx_core::LegDigest) {}
}

/// The sizes of the world's books: rows per kind table and per chunk, and the ledger's instruments, lines and holder
/// list blocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BooksSize {
    pub rows: u32,
    pub rows_per_chunk: u32,
    pub instruments: u32,
    pub lines: u32,
    pub per_chunk: u32,
    pub blocks: u32,
}

impl<B: Backing> Books<B> {
    /// The workers the books were given, if any.
    #[must_use]
    pub fn pool(&self) -> Option<&phx_exec::Pool> {
        self.pool.as_deref()
    }

    /// `f` of every index below `n`, on the books' workers where they have any, in index order either way.
    pub fn on_pool<T: Send>(&self, n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
        phx_exec::pool::map(self.pool(), n, f)
    }

    /// Workers for stage 7's shards; its results are the same with or without them.
    pub fn use_pool(&mut self, pool: std::sync::Arc<phx_exec::Pool>) {
        self.pool = Some(pool);
    }

    /// Empty books with a kind table for each individual kind, in the order given.
    #[must_use]
    pub fn new(kinds: &[&'static str], size: BooksSize) -> Books<B> {
        Books::with_cells(kinds, 0, |_, _| Vec::new(), size)
    }

    /// Empty books with a kind table for each individual kind, then `cells` agent tables, which `make` builds in the
    /// books' address space from the first agent table's identity on.
    #[must_use]
    pub fn with_cells(
        kinds: &[&'static str],
        cells: usize,
        make: impl FnOnce(&mut AddressSpace, u16) -> Vec<Box<dyn CellHolders>>,
        size: BooksSize,
    ) -> Books<B> {
        let mut space = AddressSpace::empty();
        let n = place16(kinds.len());
        let tables = (0..n)
            .zip(kinds)
            .map(|(i, kind)| KindTable::new(&mut space, kind, TableId::new(i), size.rows, size.rows_per_chunk, 0))
            .collect();
        let tables_of_cells = make(&mut space, n);
        if tables_of_cells.len() != cells {
            violation!(
                clause = "REP.1",
                "cell tables other than the books were made for",
                made = tables_of_cells.len()
            );
        }
        let cells = tables_of_cells;
        let keys = HolderKeys::new(place16(kinds.len() + cells.len()));
        let mut ledger = Ledger::new(
            Instruments::new(&mut space, size.instruments, size.per_chunk, size.blocks, keys),
            Lines::new(&mut space, size.lines, size.per_chunk, size.blocks, keys),
        );
        let dues = DueReasons::declare(&mut ledger.reasons);
        Books {
            ledger,
            parties: Parties { tables, cells, directory: Directory::new(), space },
            drawn: BTreeMap::new(),
            dues,
            opened: 0,
            leaving: BTreeMap::new(),
            buffers: crate::apply_batch::DayBuffers::default(),
            pool: None,
        }
    }

    /// An opening instruction: rows opened and balances or holdings written before day one, each write reported with
    /// the opening identity it served and the counterparty whose write answers it. A refusal stops the run, since the
    /// opening's books must close.
    #[clause("GEN.4")]
    pub fn open(&mut self, reason: ReasonId, legs: Vec<LegRec>, identity: u64, report: &mut GenReport) {
        let id = InstructionId::new(Day::new(0), self.opened);
        let Some(next) = self.opened.checked_add(1) else {
            capacity_exceeded!("opening instructions", u32::MAX, self.opened);
        };
        self.opened = next;
        for leg in legs.iter().filter(|l| matches!(l.kind, LegKind::OpeningWrite { .. })) {
            let counter = legs.iter().find(|o| o.party != leg.party).map_or(leg.party, |o| o.party);
            report.write(WriteRecord { party: leg.party, amount: i128::from(leg.qty), identity, counter });
        }
        let instruction = Instruction {
            id,
            reason,
            trade_day: Day::new(0),
            settle_day: Day::new(0),
            legs,
            pays: phx_num::Missing::Absent,
            covers: Vec::new(),
        };
        if let Err(f) = self.ledger.apply(&mut self.parties, ApplyAt::Opening, instruction, &mut Unaudited) {
            violation!(clause = "GEN.4", "an opening instruction that would not settle", party = f.party.get());
        }
    }

    /// An instruction applied to the books by the one apply routine.
    ///
    /// # Errors
    /// The fail, when it could not settle.
    pub fn apply(
        &mut self,
        at: ApplyAt,
        instruction: Instruction,
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        self.ledger.apply(&mut self.parties, at, instruction, audit)
    }

    /// A party's rows, by line and side.
    #[must_use]
    pub fn rows_of(&self, party: PartyId) -> Vec<(LineId, Side)> {
        let (place, slot) = self.parties.row(party);
        crate::rows::rows(self.parties.holder(place), slot).iter().map(|r| (r.row.line, r.side())).collect()
    }

    /// A party's row on a line of the named kind, if it holds one.
    #[must_use]
    pub fn row_on(&self, party: PartyId, kind: &str) -> Option<(LineId, Side)> {
        self.rows_of(party).into_iter().find(|(line, _)| self.ledger.lines.kind_name(*line) == kind)
    }

    /// A party's equity: what its rows and holdings are worth to it, each row's balance signed by its side and each
    /// holding at its cost. At the opening every party counts in its own country's currency alone.
    #[must_use]
    pub fn equity(&self, party: PartyId) -> i128 {
        let (place, slot) = self.parties.row(party);
        let table = self.parties.holder(place);
        let rows = crate::rows::rows(table, slot).into_iter().filter_map(|r| match r.optional.balance {
            phx_num::Missing::Present(b) => Some(i128::from(b)),
            phx_num::Missing::Absent => None,
        });
        let held = crate::holding::bases(table, slot).into_iter().map(|(_, c)| i128::from(c));
        rows.chain(held).sum()
    }

    /// The books' logical content: each party's row, due-day run head and lists in its table's slot order, the
    /// instruments, the lines, the terms with their holders and free identities, the liens, covers, commitments,
    /// arrears and open procedures.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        for t in &self.parties.tables {
            h.bytes(t.kind().as_bytes());
            for slot in t.slots() {
                h.u64(t.party(slot).get());
                h.u64(u64::from(t.site(slot).get()));
                h.u64(u64::from(t.created(slot).get()));
                let head = t.run_head(slot);
                for v in [head.next_due, head.offset, head.len] {
                    h.u64(u64::from(v));
                }
                for list in [ListKind::RelationshipRows, ListKind::Holdings, ListKind::Lots, ListKind::NamedUnits] {
                    let words = t.words(slot, list);
                    h.u64(phx_rand::float::len_u64(words.len()));
                    for w in words {
                        h.u64(*w);
                    }
                }
            }
        }
        for c in &self.parties.cells {
            c.hash_into(h);
        }
        self.ledger.instruments.hash_into(h);
        self.ledger.lines.hash_into(h);
        let l = &self.ledger;
        phx_store::hash_saved(&l.terms, h);
        phx_store::hash_saved(&l.liens, h);
        phx_store::hash_saved(&l.covers, h);
        phx_store::hash_saved(&l.commitments, h);
        phx_store::hash_saved(&l.arrears, h);
        phx_store::hash_saved(&l.procedures, h);
        phx_store::hash_saved(&l.chains, h);
    }

    /// Stage 2d's contract process over the books.
    pub fn contract_process(&mut self, fails: &[Fail], today: Day) {
        self.ledger.contract_process(&mut self.parties, fails, today);
    }

    /// The day's book, handed to the close.
    pub fn close(&mut self) -> DayBook {
        self.ledger.close()
    }
}

/// The world's books, as the assembly hands them to an opening contribution.
pub fn of<'a>(opening: &'a mut phx_core::Opening<'_>) -> &'a mut Books {
    split(opening).0
}

/// The world's books and the opening's report together.
pub fn split<'a>(opening: &'a mut phx_core::Opening<'_>) -> (&'a mut Books, &'a mut GenReport) {
    let Some(books) = opening.books.downcast_mut::<Books>() else {
        violation!(clause = "GEN.3", "an opening handed something other than the world's books");
    };
    (books, &mut *opening.report)
}

impl<B: Backing> Books<B> {
    /// The books for a save: every kind table, the directory, the ledger, the opening's drawn sizes and its count
    /// of instructions.
    #[clause("SET.12")]
    pub fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        self.parties.tables.save(w);
        for c in &self.parties.cells {
            c.save_to(w);
        }
        self.parties.directory.save(w);
        self.ledger.save_to(w);
        self.drawn.save(w);
        self.opened.save(w);
    }

    /// The books read back over `declared`, books the build made and ran the declarations phase on alone, which
    /// carry the line kinds, reasons and due reasons the build declares. The holder lists are rebuilt from the
    /// holdings and rows, in table and slot order.
    ///
    /// # Errors
    /// When the store is damaged or holds other kinds or declarations than the build's.
    #[clause("SET.12", "SET.15")]
    pub fn load_from(r: &mut phx_store::Reader<'_>, declared: Books<B>) -> Result<Books<B>, phx_store::LoadError> {
        use phx_store::Saved as _;
        let Books { ledger, parties, dues, .. } = declared;
        let keys = ledger.instruments.keys();
        let tables: Vec<KindTable<B>> = phx_store::Saved::load(r)?;
        let kinds_differ = tables.len() != parties.tables.len()
            || tables.iter().zip(&parties.tables).any(|(a, b)| a.kind() != b.kind() || a.id() != b.id());
        if kinds_differ {
            return Err(phx_store::LoadError::Invalid("kind tables other than the build's".to_owned()));
        }
        let mut cells = Vec::with_capacity(parties.cells.len());
        for like in &parties.cells {
            let c = like.load_like(r)?;
            if c.kind() != like.kind() || c.table() != like.table() {
                return Err(phx_store::LoadError::Invalid("cell tables other than the build's".to_owned()));
            }
            cells.push(c);
        }
        drop(parties);
        let directory = Directory::load(r)?;
        let ledger = Ledger::load_from(r, ledger, keys)?;
        let drawn = BTreeMap::load(r)?;
        let opened = u32::load(r)?;
        let space = r.take_space();
        let mut books = Books {
            ledger,
            parties: Parties { tables, cells, directory, space },
            drawn,
            dues,
            opened,
            leaving: BTreeMap::new(),
            buffers: crate::apply_batch::DayBuffers::default(),
            pool: None,
        };
        books.relist();
        Ok(books)
    }

    /// Empty books of these books' kinds and sizes, carrying their declarations: line kinds, reasons, instrument
    /// events and due reasons, as a save of these books is read back over; `make` builds their empty agent tables.
    #[must_use]
    pub fn declared(
        &self,
        size: BooksSize,
        make: impl FnOnce(&mut AddressSpace, u16) -> Vec<Box<dyn CellHolders>>,
    ) -> Books<B> {
        let kinds: Vec<&'static str> = self.parties.kinds().collect();
        let mut out = Books::with_cells(&kinds, self.parties.cells.len(), make, size);
        out.ledger.lines.copy_decls(&self.ledger.lines);
        out.ledger.reasons = self.ledger.reasons.clone();
        out.ledger.events = self.ledger.events.clone();
        out.dues = self.dues;
        out
    }

    /// Every holder put back on the holder lists of the instruments it holds and the lines it has rows on.
    fn relist(&mut self) {
        let Books { ledger, parties, .. } = self;
        for (place, table) in (0_u16..).zip(parties.holders()) {
            for slot in phx_store::table::live_in(table.live_words()) {
                ledger.lines.refile_head(table, place, slot);
                for (instrument, _) in crate::holding::bases(table, slot) {
                    ledger.instruments.relist(place, slot, instrument);
                }
                let mut by_line: BTreeMap<LineId, Vec<Side>> = BTreeMap::new();
                for row in crate::rows::rows(table, slot) {
                    by_line.entry(row.row.line).or_default().push(row.side());
                }
                for (line, sides) in by_line {
                    ledger.lines.relist(table, place, slot, line, &sides);
                }
            }
        }
    }
}
