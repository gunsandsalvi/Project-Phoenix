//! The world's populations as the day reaches them: each population kind's compiled declaration, the place of its
//! cell table among the books' holder tables, its interned keys, its landing index and the level each position's
//! steps stand at.

use phx_core::{KinkRegistry, PopEntry};
use phx_id::TableId;
use phx_ledger::holder::CellHolders;
use phx_macros::clause;
use phx_num::violation;
use phx_store::{AddressSpace, Backing, LoadError, Reader, Saved, Writer};

use crate::audit::CellsView;
use crate::index::Index;
use crate::key::KeyInterner;
use crate::kind::PopKindDecl;
use crate::landing::Landed;
use crate::steps::StepTable;
use crate::table::CellTable;

/// One population kind in the world.
#[derive(Debug)]
pub struct PopKind {
    pub decl: PopKindDecl,
    /// Its cell table's place among the books' holder tables.
    pub place: u16,
    pub keys: KeyInterner,
    pub index: Index,
    /// Each position's step level, which only tolerance control moves.
    pub levels: Vec<u8>,
}

/// Every population kind, in the order their tables follow the kind tables; each kind's members as the events that
/// began and ended them count them, which its cells' weights are held to; and the day's landings.
#[derive(Debug, Default)]
pub struct Population {
    pub kinds: Vec<PopKind>,
    pub members: Vec<(&'static str, u64)>,
    pub landed: Vec<Landed>,
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
    /// until tolerance control widens them.
    #[must_use]
    pub fn new(decls: Vec<PopKindDecl>, first: u16) -> Population {
        let kinds = (first..)
            .zip(decls)
            .map(|(place, decl)| PopKind {
                levels: vec![0; decl.positions.len()],
                decl,
                place,
                keys: KeyInterner::new(),
                index: Index::new(),
            })
            .collect::<Vec<PopKind>>();
        let members = kinds.iter().map(|k| (k.decl.kind, 0)).collect();
        let landed = kinds.iter().map(|_| Landed::default()).collect();
        Population { kinds, members, landed }
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
    pub fn view<'a, B: Backing + 'static>(&'a self, cells: &'a [Box<dyn CellHolders>]) -> CellsView<'a, B> {
        let tables = (0..self.kinds.len()).map(|i| Population::table::<B>(cells, i)).collect();
        CellsView::new(tables, &self.members, &self.landed)
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
    }

    /// Each kind's keys and levels read back over the build's kinds.
    ///
    /// # Errors
    /// When the store is damaged or its levels do not fit the build's positions.
    #[clause("SET.12")]
    pub fn load_from(&mut self, r: &mut Reader<'_>) -> Result<(), LoadError> {
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
    }
}
