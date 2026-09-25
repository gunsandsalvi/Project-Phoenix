//! The world's populations as the day reaches them: each population kind's compiled declaration, the place of its
//! cell table among the books' holder tables, its interned keys, its landing index and the level each position's
//! steps stand at.

use phx_core::{Agenda, AgendaTableSpec, KinkRegistry, PopEntry};
use phx_id::{Day, LineId, TableId};
use phx_ledger::algebra::Side;
use phx_ledger::holder::CellHolders;
use phx_macros::clause;
use phx_num::violation;
use phx_store::{AddressSpace, Backing, LoadError, Reader, Saved, Writer};

use crate::audit::CellsView;
use crate::consts::{AGENDA_BLOCKS, CELL_AGENDA_ROWS};
use crate::index::Index;
use crate::key::KeyInterner;
use crate::kind::PopKindDecl;
use crate::landing::Landed;
use crate::promote::Ranks;
use crate::steps::StepTable;
use crate::table::CellTable;
use crate::tolerance::Tolerances;

/// A kind as the world sets it up: its compiled declaration, how many processes act on its members (its agenda's
/// reasons), and its representation's settings read from the primitives its resolution names.
#[derive(Clone, Debug)]
pub struct KindSetup {
    pub decl: PopKindDecl,
    pub processes: usize,
    pub tolerances: Tolerances,
    pub ranks: Option<Ranks>,
}

/// One population kind in the world.
#[derive(Debug)]
pub struct PopKind {
    pub decl: PopKindDecl,
    pub processes: usize,
    pub tolerances: Tolerances,
    pub ranks: Option<Ranks>,
    /// Its cell table's place among the books' holder tables.
    pub place: u16,
    pub keys: KeyInterner,
    pub index: Index,
    /// Each position's step level, which only tolerance control moves.
    pub levels: Vec<u8>,
}

/// Every population kind, in the order their tables follow the kind tables; each kind's members as the events that
/// began and ended them count them, which its cells' weights are held to; the day's landings; and the agenda of the
/// cells the processes act on, one table for each kind that has any.
#[derive(Debug)]
pub struct Population {
    pub kinds: Vec<PopKind>,
    pub members: Vec<(&'static str, u64)>,
    pub landed: Vec<Landed>,
    pub agenda: Agenda,
}

impl Population {
    /// The kinds compiled from every system's items, in the order given, each position's steps read through `steps`.
    ///
    /// # Errors
    /// Every refusal of every kind at once.
    pub fn compile(
        names: &[&'static str],
        entries: &[PopEntry],
        kinks: &KinkRegistry,
        steps: &dyn Fn(&'static str) -> Result<StepTable, String>,
    ) -> Result<Vec<PopKindDecl>, Vec<String>> {
        let mut errors = Vec::new();
        let mut out = Vec::with_capacity(names.len());
        for kind in names {
            match PopKindDecl::compile(kind, entries, kinks, steps) {
                Ok(d) => out.push(d),
                Err(e) => errors.extend(e),
            }
        }
        if errors.is_empty() { Ok(out) } else { Err(errors) }
    }

    /// The population over its kinds, their tables from `first` on among the books' places, each at the finest steps
    /// until tolerance control widens them, with an empty agenda from `today`.
    #[must_use]
    pub fn new(setups: Vec<KindSetup>, first: u16, today: Day, space: &mut AddressSpace) -> Population {
        let kinds = (first..)
            .zip(setups)
            .map(|(place, s)| PopKind {
                levels: vec![0; s.decl.positions.len()],
                decl: s.decl,
                processes: s.processes,
                tolerances: s.tolerances,
                ranks: s.ranks,
                place,
                keys: KeyInterner::new(),
                index: Index::new(),
            })
            .collect::<Vec<PopKind>>();
        let members = kinds.iter().map(|k| (k.decl.kind, 0)).collect();
        let landed = kinds.iter().map(|_| Landed::default()).collect();
        let Ok(agenda) = Agenda::new(space, today, &agenda_specs(&kinds), AGENDA_BLOCKS) else {
            violation!(clause = "TIME.5", "more processes on a kind than an agenda row has reasons");
        };
        Population { kinds, members, landed, agenda }
    }

    /// The agenda's table for a kind, if processes act on its members.
    #[must_use]
    pub fn agenda_table(&self, kind: usize) -> Option<TableId> {
        self.kinds.get(kind).filter(|k| k.processes > 0).map(|k| TableId::new(k.place))
    }

    /// Members a kind gained or lost by the events that begin and end them.
    #[clause("REP.13", "POP.11")]
    pub fn count(&mut self, kind: usize, gained: u64, lost: u64) {
        let Some((_, n)) = self.members.get_mut(kind) else {
            violation!(clause = "REP.13", "members counted for a kind the world does not keep", kind = kind);
        };
        let Some(next) = n.checked_add(gained).and_then(|m| m.checked_sub(lost)) else {
            violation!(clause = "REP.13", "a population counted below none or past its width", kind = kind);
        };
        *n = next;
    }

    /// The cell tables as the audit reads them, with each kind's members and the day's landings.
    #[must_use]
    pub fn view<'a, B: Backing + 'static>(
        &'a self,
        cells: &'a [Box<dyn CellHolders>],
        sides: &'a dyn Fn(LineId, Side) -> phx_ledger::line::SideDecl,
    ) -> CellsView<'a, B> {
        let tables = (0..self.kinds.len()).map(|i| Population::table::<B>(cells, i)).collect();
        let keys = self.kinds.iter().map(|k| &k.keys).collect();
        let kinds = self.kinds.iter().map(|k| &k.decl).collect();
        CellsView::new((tables, keys, kinds), sides, &self.members, &self.landed)
    }

    /// Each kind's empty cell table, made in the books' address space with identities from `first` on.
    #[must_use]
    pub fn tables<B: Backing + core::fmt::Debug + 'static>(
        decls: &[PopKindDecl],
        space: &mut AddressSpace,
        first: u16,
        rows: u32,
        rows_per_chunk: u32,
    ) -> Vec<Box<dyn CellHolders>>
    where
        CellTable<B>: Send + Sync,
    {
        (first..)
            .zip(decls)
            .map(|(id, d)| -> Box<dyn CellHolders> {
                Box::new(CellTable::<B>::new(space, d, TableId::new(id), rows, rows_per_chunk))
            })
            .collect()
    }

    /// A kind's cell table among the books' cell tables.
    #[must_use]
    pub fn table<B: Backing + 'static>(cells: &[Box<dyn CellHolders>], i: usize) -> &CellTable<B> {
        let found = cells.get(i).and_then(|c| c.as_any().downcast_ref::<CellTable<B>>());
        let Some(t) = found else {
            violation!(clause = "REP.1", "a population kind the books keep no cell table for", kind = i);
        };
        t
    }

    /// A kind's cell table, to change.
    pub fn table_mut<B: Backing + 'static>(cells: &mut [Box<dyn CellHolders>], i: usize) -> &mut CellTable<B> {
        let found = cells.get_mut(i).and_then(|c| c.as_any_mut().downcast_mut::<CellTable<B>>());
        let Some(t) = found else {
            violation!(clause = "REP.1", "a population kind the books keep no cell table for", kind = i);
        };
        t
    }

    /// After the books are read back: each table's layout restored from its kind, and each kind's landing index
    /// rebuilt from its cells.
    #[clause("SET.12")]
    pub fn restore<B: Backing + 'static>(&mut self, cells: &mut [Box<dyn CellHolders>]) {
        for (i, k) in self.kinds.iter_mut().enumerate() {
            let t = Population::table_mut::<B>(cells, i);
            t.relayout(&k.decl);
            k.index = Index::rebuild(t);
        }
    }

    /// Each kind's keys and levels, for a save; the tables are the books'.
    #[clause("SET.12")]
    pub fn save_to(&self, w: &mut Writer<'_>) {
        for k in &self.kinds {
            k.keys.save(w);
            k.levels.save(w);
        }
        let members: Vec<u64> = self.members.iter().map(|(_, n)| *n).collect();
        members.save(w);
        self.agenda.save_to(w);
    }

    /// Each kind's keys and levels read back over the build's kinds.
    ///
    /// # Errors
    /// When the store is damaged or its levels do not fit the build's positions.
    #[clause("SET.12")]
    pub fn load_from(&mut self, r: &mut Reader<'_>, space: &mut AddressSpace) -> Result<(), LoadError> {
        for k in &mut self.kinds {
            k.keys = KeyInterner::load(r)?;
            let levels: Vec<u8> = Vec::load(r)?;
            if levels.len() != k.decl.positions.len() {
                return Err(LoadError::Invalid(format!("levels for another `{}`", k.decl.kind)));
            }
            k.levels = levels;
        }
        let members: Vec<u64> = Vec::load(r)?;
        if members.len() != self.members.len() {
            return Err(LoadError::Invalid("members counted for other kinds".to_owned()));
        }
        for ((_, n), m) in self.members.iter_mut().zip(members) {
            *n = m;
        }
        self.agenda = Agenda::load_from(r, space, &agenda_specs(&self.kinds), AGENDA_BLOCKS)?;
        Ok(())
    }

    /// Each kind's keys and levels, for the world's hash.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        for k in &self.kinds {
            phx_store::hash_saved(&k.keys, h);
            phx_store::hash_saved(&k.levels, h);
        }
        for (_, n) in &self.members {
            h.u64(*n);
        }
        self.agenda.hash_into(h);
    }
}

/// Every row of a kind added, removed or grown since it was last booked, booked afresh: its bookings dropped, and a
/// live row booked for every process to be drawn on `first`, so each draws at its present weight and values.
#[clause("REP.7")]
pub fn book_changed<B: Backing>(kind: &PopKind, table: &mut CellTable<B>, agenda: &mut Agenda, first: Day) {
    let changed = table.take_changed();
    if kind.processes == 0 {
        return;
    }
    let tid = TableId::new(kind.place);
    agenda.grow(tid, table.high_water());
    for s in changed {
        agenda.release(tid, s);
        if table.is_live(s) {
            for r in 0..kind.processes {
                agenda.set_next_with(tid, s, r, first, 0);
            }
        }
    }
}

/// The agenda's tables: one for each kind that processes act on, each reason one of its processes.
fn agenda_specs(kinds: &[PopKind]) -> Vec<AgendaTableSpec> {
    kinds
        .iter()
        .filter(|k| k.processes > 0)
        .map(|k| AgendaTableSpec { table: TableId::new(k.place), max_rows: CELL_AGENDA_ROWS, reasons: k.processes })
        .collect()
}
