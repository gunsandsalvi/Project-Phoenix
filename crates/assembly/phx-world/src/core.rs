//! The world on the core during the port: every party the opening began, of every kind, begun again on the core's
//! stores with its identity, in the old books' order of kinds and slots. A household keeps its attributes and then its
//! positions as its record's words, and its persons; an individual keeps the tile it is sited on. The old books stay
//! the committed world until the core takes over.

use phx_core::store::KindStore;
use phx_id::consts::NATURE_KIND;
use phx_id::{PartyId, PartyKey, Slot};
use phx_ledger::books::Books;
use phx_num::{MaybeI64, Missing, violation};
use phx_pop::persons::Persons;
use phx_pop::population::Population;
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, KIND_ROWS, KIND_ROWS_PER_CHUNK};

/// Each kind's parties on the core by its place among the old books' tables, the persons of each kind that holds
/// them, and every party's key by its identity, sorted.
#[derive(Debug)]
pub struct Core {
    pub space: AddressSpace,
    pub names: Vec<&'static str>,
    pub kinds: Vec<KindStore<SystemBacking>>,
    pub persons: Vec<Option<Persons<SystemBacking>>>,
    pub keys: Vec<(PartyId, PartyKey)>,
}

fn kind_number(place: usize) -> u8 {
    match u8::try_from(place) {
        Ok(k) if k < NATURE_KIND => k,
        _ => violation!(clause = "REP.1", "more kinds of party than a key can name", kind = place),
    }
}

fn word(v: u32) -> MaybeI64 {
    MaybeI64::present(i64::from(v))
}

impl Core {
    /// The core's parties mirrored from the old books: each kind table's individuals with their sites, each agent
    /// table's agents with their attributes, positions and persons.
    #[must_use]
    pub fn mirror(books: &Books, population: &Population) -> Core {
        let mut space = AddressSpace::empty();
        let parties = &books.parties;
        let first_agents = usize::from(parties.first_cell_place());
        let (mut names, mut kinds, mut persons, mut keys) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (place, holder) in parties.holders().enumerate() {
            let kind = kind_number(place);
            names.push(holder.kind());
            if place < first_agents {
                let table = parties.table(u16::from(kind));
                let mut store: KindStore<SystemBacking> =
                    KindStore::new(&mut space, kind, KIND_ROWS, KIND_ROWS_PER_CHUNK, 1);
                for slot in table.slots() {
                    let party = store.begin(table.party(slot), &[word(table.site(slot).get())], None);
                    keys.push((table.party(slot), PartyKey::new(kind, party.slot())));
                }
                kinds.push(store);
                persons.push(None);
                continue;
            }
            let Some(pop) = population.kinds.get(place - first_agents) else {
                violation!(clause = "REP.1", "an agent table with no population kind", kind = place);
            };
            let decl = &pop.decl;
            let table = Population::table::<SystemBacking>(parties.cells(), place - first_agents);
            let positions: Vec<_> = decl.positions.iter().map(|p| table.position(p.item.name)).collect();
            let stride = decl.attrs.len() + positions.len();
            let mut store: KindStore<SystemBacking> =
                KindStore::new(&mut space, kind, AGENT_ROWS, AGENT_ROWS_PER_CHUNK, stride);
            let mut held = decl.has_persons().then(|| Persons::new(&mut space, AGENT_ROWS, AGENT_ROWS_PER_CHUNK));
            let mut record = Vec::with_capacity(stride);
            for slot in table.slots() {
                record.clear();
                record.extend(table.attrs(slot).into_iter().map(word));
                record.extend(positions.iter().map(|c| match c.map(|c| table.fact(slot, c)) {
                    Some(Missing::Present(v)) => MaybeI64::present(v),
                    _ => MaybeI64::ABSENT,
                }));
                let party = store.begin(table.party(slot), &record, None);
                if let Some(p) = held.as_mut() {
                    p.set(&mut space, party.slot(), table.persons(slot));
                }
                keys.push((table.party(slot), PartyKey::new(kind, party.slot())));
            }
            let copied = held.as_ref().map_or(0, Persons::held);
            if copied != table.persons_held() {
                violation!(clause = "REP.13", "the core's households hold other persons than the books'", kind = place);
            }
            kinds.push(store);
            persons.push(held);
        }
        keys.sort_unstable_by_key(|(id, _)| *id);
        Core { space, names, kinds, persons, keys }
    }

    /// A party's key on the core, none for a party it does not hold.
    #[must_use]
    pub fn key(&self, party: PartyId) -> Option<PartyKey> {
        self.keys.binary_search_by_key(&party, |(id, _)| *id).ok().and_then(|i| self.keys.get(i)).map(|(_, k)| *k)
    }

    /// The parties of a kind the core holds, by the kind's name.
    #[must_use]
    pub fn count(&self, name: &str) -> u64 {
        self.names
            .iter()
            .position(|n| *n == name)
            .and_then(|i| self.kinds.get(i))
            .map_or(0, |k| k.parties.live_slots().map(|_: Slot| 1_u64).sum())
    }

    /// The persons every household on the core holds.
    #[must_use]
    pub fn persons_held(&self) -> u64 {
        self.persons.iter().flatten().map(Persons::held).sum()
    }
}
