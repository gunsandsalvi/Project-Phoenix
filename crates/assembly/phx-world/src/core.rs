//! The world on the core during the port: every party the opening began, of every kind, begun again on the core's
//! stores with its identity, in the old books' order of kinds and slots. A household keeps its attributes and then its
//! positions as its record's words, and its persons; an individual keeps the tile it is sited on. Each party that
//! holds an account holds one on the core at the bank the books give it, each sector's total its country's balance
//! sheet's, apportioned over the sector's parties by what the books gave each. The old books stay the committed world
//! until the core takes over.

use phx_core::OpeningCountry;
use phx_core::settle::AT_ISSUER;
use phx_core::store::{KindStore, Opening};
use phx_id::consts::NATURE_KIND;
use phx_id::{PartyId, PartyKey, Slot};
use phx_ledger::books::Books;
use phx_num::round::{Round, split_total};
use phx_num::{Ccy, MaybeI64, Missing, violation};
use phx_pop::persons::Persons;
use phx_pop::population::Population;
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::sheet::ACCOUNTS;
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, KIND_ROWS, KIND_ROWS_PER_CHUNK};
use crate::opening::sheet::Sheet;

/// A party's account as the books give it: its kind, slot, country, the bank that owes it and its balance there.
struct Held {
    kind: usize,
    slot: Slot,
    country: usize,
    bank: u32,
    weight: u64,
}

/// Each kind's parties on the core by its place among the old books' tables, the persons of each kind that holds
/// them, and every party's key by its identity, sorted.
#[derive(Debug)]
pub struct Core {
    pub space: AddressSpace,
    /// The place of the first agent table: the kinds before it are individuals.
    pub first_agents: u16,
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
        Core { space, first_agents: parties.first_cell_place(), names, kinds, persons, keys }
    }

    /// Every party's account on the core: each kind's sector's total in its country's sheet, a share of its GDP made
    /// whole, apportioned over the country's parties of that sector by the balance the books gave each (equal parts
    /// where the books gave them none), each held at the bank the books hold it at, a bank's own at the issuer.
    pub fn open_money(&mut self, books: &Books, countries: &[OpeningCountry], sheets: &[Sheet]) {
        let bank_kind = self.names.iter().position(|n| *n == "bank");
        let mut held: Vec<Held> = Vec::new();
        for (kind, name) in self.names.iter().enumerate() {
            if !ACCOUNTS.iter().any(|(n, _, _)| n == name) {
                continue;
            }
            let store = self.kinds.get(kind);
            for slot in store.map(|s| s.parties.live_slots().collect::<Vec<_>>()).unwrap_or_default() {
                let Some(id) = store.and_then(|s| s.parties.id(slot)) else { continue };
                let found =
                    countries.iter().enumerate().find_map(|(i, c)| match books.account(id, Ccy::new(c.id.get())) {
                        Missing::Present((owed_by, balance)) => Some((i, owed_by, balance)),
                        Missing::Absent => None,
                    });
                let Some((country, owed_by, balance)) = found else { continue };
                let bank = match self.key(owed_by) {
                    Some(k) if Some(usize::from(k.kind())) == bank_kind && Some(kind) != bank_kind => k.slot().get(),
                    _ => AT_ISSUER,
                };
                let Ok(weight) = u64::try_from(balance) else {
                    violation!(clause = "GEN.4", "an opening account below nothing", party = id.get());
                };
                held.push(Held { kind, slot, country, bank, weight });
            }
        }
        for (kind, name) in self.names.iter().enumerate() {
            if ACCOUNTS.iter().any(|(n, _, _)| n == name)
                && let Some(store) = self.kinds.get_mut(kind)
            {
                store.add_accounts(
                    &mut self.space,
                    if kind < usize::from(self.first_agents) { KIND_ROWS } else { AGENT_ROWS },
                    if kind < usize::from(self.first_agents) { KIND_ROWS_PER_CHUNK } else { AGENT_ROWS_PER_CHUNK },
                );
            }
        }
        for ((country, c), sheet) in countries.iter().enumerate().zip(sheets) {
            for (_, sector, instrument) in ACCOUNTS {
                let parts: Vec<&Held> = held
                    .iter()
                    .filter(|h| h.country == country)
                    .filter(|h| {
                        self.names.get(h.kind).is_some_and(|n| ACCOUNTS.iter().any(|(k, s, _)| k == n && *s == sector))
                    })
                    .collect();
                // Each sector is apportioned once, by the first kind that names it.
                if ACCOUNTS.iter().position(|(_, s, _)| *s == sector)
                    != ACCOUNTS.iter().position(|(_, s, i)| *s == sector && *i == instrument)
                {
                    continue;
                }
                let total = phx_ledger::opening::whole(sheet.at(instrument, sector) * c.gdp);
                if parts.is_empty() && total != 0 {
                    violation!(
                        clause = "Law 2",
                        "a sector's opening money with no party to hold it",
                        country = country,
                        sector = sector
                    );
                }
                let weights: Vec<u64> = parts.iter().map(|h| h.weight).collect();
                let equal = weights.iter().all(|w| *w == 0);
                let mut whole: u64 = if equal { phx_rand::float::len_u64(parts.len()) } else { weights.iter().sum() };
                let mut left = total;
                for (h, w) in parts.iter().zip(weights) {
                    let k = if equal { 1 } else { w };
                    let (share, rest) = split_total(left, k, whole, Round::HalfEven);
                    left = rest;
                    whole -= k;
                    if let Some(store) = self.kinds.get_mut(h.kind) {
                        store.open_account(h.slot, Opening { bank: h.bank, balance: share });
                    }
                }
            }
        }
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
