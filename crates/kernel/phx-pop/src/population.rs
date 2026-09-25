//! The world's populations as the day reaches them: each population kind's compiled declaration, the place of its
//! agent table among the books' holder tables, the representation in force, each kind's parties counted by event, and
//! the agenda of the agents the processes act on.

use phx_core::{Agenda, AgendaTableSpec, PopEntry};
use phx_id::{Day, TableId};
use phx_ledger::holder::CellHolders;
use phx_macros::clause;
use phx_num::violation;
use phx_store::{AddressSpace, Backing, LoadError, Reader, Saved, Writer};

use crate::consts::{AGENDA_BLOCKS, AGENT_AGENDA_ROWS};
use crate::kind::PopKindDecl;
use crate::prims::Representation;
use crate::table::AgentTable;

/// One population kind in the world: its compiled declaration, how many processes act on its persons (its agenda's
/// reasons), and its table's place among the books' holder tables.
#[derive(Clone, Debug)]
pub struct PopKind {
    pub decl: PopKindDecl,
    pub processes: usize,
    pub place: u16,
}

/// Every population kind, in the order their tables follow the kind tables; the representation in force; each kind's
/// real parties and persons as the events that began and ended them count them, which its agents' multiplicities are
/// held to; and the agenda of the agents the processes act on, one table for each kind that has any.
#[derive(Debug)]
pub struct Population {
    pub kinds: Vec<PopKind>,
    pub representation: Representation,
    pub members: Vec<(&'static str, u64)>,
    pub persons: Vec<u64>,
    pub agenda: Agenda,
}

impl Population {
    /// The kinds compiled from every system's items, in the order given.
    ///
    /// # Errors
    /// Every refusal of every kind at once.
    pub fn compile(names: &[&'static str], entries: &[PopEntry]) -> Result<Vec<PopKindDecl>, Vec<String>> {
        let mut errors = Vec::new();
        let mut out = Vec::with_capacity(names.len());
        for kind in names {
            match PopKindDecl::compile(kind, entries) {
                Ok(d) => out.push(d),
                Err(e) => errors.extend(e),
            }
        }
        if errors.is_empty() { Ok(out) } else { Err(errors) }
    }

    /// The population over its kinds, each with the number of processes on it, their tables from `first` on among
    /// the books' places, with an empty agenda from `today`.
    #[must_use]
    pub fn new(
        kinds: Vec<(PopKindDecl, usize)>,
        representation: Representation,
        first: u16,
        today: Day,
        space: &mut AddressSpace,
    ) -> Population {
        let kinds: Vec<PopKind> =
            (first..).zip(kinds).map(|(place, (decl, processes))| PopKind { decl, processes, place }).collect();
        let members = kinds.iter().map(|k| (k.decl.kind, 0)).collect();
        let persons = kinds.iter().map(|_| 0).collect();
        let Ok(agenda) = Agenda::new(space, today, &agenda_specs(&kinds), AGENDA_BLOCKS) else {
            violation!(clause = "TIME.5", "more processes on a kind than an agenda row has reasons");
        };
        Population { kinds, representation, members, persons, agenda }
    }

    /// The agenda's table for a kind, if processes act on its persons.
    #[must_use]
    pub fn agenda_table(&self, kind: usize) -> Option<TableId> {
        self.kinds.get(kind).filter(|k| k.processes > 0).map(|k| TableId::new(k.place))
    }

    /// Parties and persons a kind gained or lost by the events that begin and end them.
    #[clause("REP.13", "POP.11")]
    pub fn count(&mut self, kind: usize, (gained, lost): (u64, u64), (born, died): (u64, u64)) {
        let (Some((_, n)), Some(p)) = (self.members.get_mut(kind), self.persons.get_mut(kind)) else {
            violation!(clause = "REP.13", "parties counted for a kind the world does not keep", kind = kind);
        };
        let (Some(next), Some(people)) = (
            n.checked_add(gained).and_then(|m| m.checked_sub(lost)),
            p.checked_add(born).and_then(|m| m.checked_sub(died)),
        ) else {
            violation!(clause = "REP.13", "a population counted below none or past its width", kind = kind);
        };
        (*n, *p) = (next, people);
    }

    /// Each kind's empty agent table, made in the books' address space with identities from `first` on.
    #[must_use]
    pub fn tables<B: Backing + core::fmt::Debug + 'static>(
        decls: &[PopKindDecl],
        space: &mut AddressSpace,
        first: u16,
        rows: u32,
        rows_per_chunk: u32,
    ) -> Vec<Box<dyn CellHolders>>
    where
        AgentTable<B>: Send + Sync,
    {
        (first..)
            .zip(decls)
            .map(|(id, d)| -> Box<dyn CellHolders> {
                Box::new(AgentTable::<B>::new(space, d, TableId::new(id), rows, rows_per_chunk))
            })
            .collect()
    }

    /// A kind's agent table among the books' population tables.
    #[must_use]
    pub fn table<B: Backing + 'static>(tables: &[Box<dyn CellHolders>], i: usize) -> &AgentTable<B> {
        let found = tables.get(i).and_then(|c| c.as_any().downcast_ref::<AgentTable<B>>());
        let Some(t) = found else {
            violation!(clause = "REP.1", "a population kind the books keep no agent table for", kind = i);
        };
        t
    }

    /// A kind's agent table, to change.
    pub fn table_mut<B: Backing + 'static>(tables: &mut [Box<dyn CellHolders>], i: usize) -> &mut AgentTable<B> {
        let found = tables.get_mut(i).and_then(|c| c.as_any_mut().downcast_mut::<AgentTable<B>>());
        let Some(t) = found else {
            violation!(clause = "REP.1", "a population kind the books keep no agent table for", kind = i);
        };
        t
    }

    /// Each kind's parties and persons counted, the representation and the agenda, for a save; the tables are the
    /// books'.
    #[clause("SET.12")]
    pub fn save_to(&self, w: &mut Writer<'_>) {
        self.representation.save(w);
        let members: Vec<u64> = self.members.iter().map(|(_, n)| *n).collect();
        members.save(w);
        self.persons.save(w);
        self.agenda.save_to(w);
    }

    /// The counts, the representation and the agenda read back over the build's kinds.
    ///
    /// # Errors
    /// When the store is damaged, or was saved under another representation.
    #[clause("SET.12", "REP.40")]
    pub fn load_from(&mut self, r: &mut Reader<'_>, space: &mut AddressSpace) -> Result<(), LoadError> {
        let representation = Representation::load(r)?;
        if representation != self.representation {
            return Err(LoadError::Invalid(format!(
                "a save of {} read by a build holding {}",
                representation.name(),
                self.representation.name()
            )));
        }
        let members: Vec<u64> = Vec::load(r)?;
        let persons: Vec<u64> = Vec::load(r)?;
        if members.len() != self.members.len() || persons.len() != self.persons.len() {
            return Err(LoadError::Invalid("parties counted for other kinds".to_owned()));
        }
        for ((_, n), m) in self.members.iter_mut().zip(members) {
            *n = m;
        }
        self.persons = persons;
        self.agenda = Agenda::load_from(r, space, &agenda_specs(&self.kinds), AGENDA_BLOCKS)?;
        Ok(())
    }

    /// The counts, the representation and the agenda, for the world's hash.
    pub fn hash_into(&self, h: &mut phx_store::LogicalHasher) {
        phx_store::hash_saved(&self.representation, h);
        for (_, n) in &self.members {
            h.u64(*n);
        }
        for n in &self.persons {
            h.u64(*n);
        }
        self.agenda.hash_into(h);
    }
}

/// The agenda's tables: one for each kind that processes act on, each reason one of its processes.
fn agenda_specs(kinds: &[PopKind]) -> Vec<AgendaTableSpec> {
    kinds
        .iter()
        .filter(|k| k.processes > 0)
        .map(|k| AgendaTableSpec { table: TableId::new(k.place), max_rows: AGENT_AGENDA_ROWS, reasons: k.processes })
        .collect()
}
