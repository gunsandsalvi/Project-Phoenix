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
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Ccy, MaybeI64, Missing, violation};
use phx_pop::persons::Persons;
use phx_pop::population::Population;
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::sheet::ACCOUNTS;
use crate::consts::{
    AGENT_ROWS, AGENT_ROWS_PER_CHUNK, CORE_RANGE_BITS, CORE_WHEEL_DAYS, KIND_ROWS, KIND_ROWS_PER_CHUNK,
};
use crate::core_day::{DatedFamily, Due};
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
    /// Each country's central bank, the issuer of its currency, by the currency's index.
    pub issuers: Vec<PartyKey>,
    pub bank_kind: Option<u8>,
    pub range_bits: u32,
    pub families: Vec<crate::core_day::DatedFamily>,
    pub work: crate::core_day::Work,
    pub days: Vec<crate::core_day::CoreDay>,
    pub hazards: Vec<crate::core_pop::Hazard>,
    pub household_decl: Option<phx_pop::kind::PopKindDecl>,
    /// The persons the households held when the core opened.
    pub persons_opened: u64,
    /// Each country's treasury, and the estates waiting to settle: each with its country and the day it opened.
    pub treasuries: Vec<Option<PartyKey>>,
    pub estates: Vec<(PartyKey, phx_id::CountryId, phx_id::Day)>,
    /// The next identity the core hands a party it begins.
    pub next_id: u64,
    pub pop_days: Vec<(phx_id::Day, crate::core_pop::PopDay)>,
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
        let bank_kind = names.iter().position(|n| *n == "bank").map(kind_number);
        let persons_opened = persons.iter().flatten().map(Persons::held).sum();
        let next_id = keys.last().map_or(1, |(id, _)| id.get() + 1);
        Core {
            space,
            first_agents: parties.first_cell_place(),
            names,
            kinds,
            persons,
            keys,
            issuers: Vec::new(),
            bank_kind,
            range_bits: CORE_RANGE_BITS,
            families: Vec::new(),
            work: crate::core_day::Work::default(),
            days: Vec::new(),
            hazards: Vec::new(),
            household_decl: None,
            persons_opened,
            treasuries: Vec::new(),
            estates: Vec::new(),
            next_id,
            pop_days: Vec::new(),
        }
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

    /// Each country's central bank found as the issuer of its currency, and each country's treasury.
    fn institutions(&self, books: &Books, countries: usize) -> (Vec<PartyKey>, Vec<Option<PartyKey>>) {
        let of_kind = |name: &str| -> Vec<PartyId> { books.parties.of_kind(name).collect() };
        let ccy = |c: usize| Ccy::new(u8::try_from(c).unwrap_or(u8::MAX));
        let issuers = (0..countries)
            .map(|c| {
                let found = of_kind("central_bank")
                    .into_iter()
                    .find(|id| books.holds_money(*id, ccy(c)) && matches!(books.account(*id, ccy(c)), Missing::Absent));
                match found.and_then(|id| self.key(id)) {
                    Some(k) => k,
                    None => violation!(clause = "MON.1", "a currency with no issuer on the core", country = c),
                }
            })
            .collect();
        let treasuries = (0..countries)
            .map(|c| {
                of_kind("treasury")
                    .into_iter()
                    .find(|id| matches!(books.account(*id, ccy(c)), Missing::Present(_)))
                    .and_then(|id| self.key(id))
            })
            .collect();
        (issuers, treasuries)
    }

    /// The state pensions in payment as contracts from each country's treasury to each household whose person the
    /// books pay one — every line of the pension's kind a household's person holds — a contract a person, at its
    /// line's amount, currency, payment order and next date, on the line's schedule.
    #[clause("SOC.3", "REP.3")]
    pub fn open_pensions(
        &mut self,
        books: &Books,
        line_kind: &str,
        (calendar, today): (&phx_core::calendar::Calendar, phx_id::Day),
        countries: usize,
    ) {
        let (issuers, treasuries) = self.institutions(books, countries);
        self.issuers = issuers;
        self.treasuries.clone_from(&treasuries);
        let Some(household) = self.names.iter().position(|n| *n == "household") else { return };
        let Some(treasury) = self.names.iter().position(|n| *n == "treasury") else { return };
        let Some(pop_at) = household.checked_sub(usize::from(self.first_agents)) else { return };
        let kinds = [kind_number(treasury), kind_number(household)];
        let mut family = DatedFamily {
            name: "SOC.pension",
            store: phx_core::store::Family::new(
                &mut self.space,
                (kinds, [KIND_ROWS, AGENT_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [false, true],
                (today.succ(), CORE_WHEEL_DAYS),
            ),
            reason: crate::core_day::PENSION,
            schedules: Vec::new(),
        };
        // Each pension line read once: its amount, and where its schedule and next date are among the family's.
        let mut lines: std::collections::BTreeMap<phx_id::LineId, Option<(i64, u32, u32)>> =
            std::collections::BTreeMap::new();
        let ledger = &books.ledger;
        let table = Population::table::<SystemBacking>(books.parties.cells(), pop_at);
        for slot in table.slots() {
            let Some(pensioner) = self.key(table.party(slot)) else { continue };
            for word in table.attachments(slot) {
                let a = phx_pop::person::Attachment::unpack(*word);
                if ledger.lines.kind_name(a.line) != line_kind {
                    continue;
                }
                let read = *lines.entry(a.line).or_insert_with(|| {
                    let terms = ledger.terms.get(ledger.lines.terms(a.line));
                    let amount = terms.legs.iter().find_map(|l| match l {
                        phx_ledger::algebra::Leg::FixedAmount(m) => Some(m.amt()),
                        _ => None,
                    })?;
                    let (dates, next) = (terms.schedule.dates, ledger.lines.next_due(a.line));
                    let mut nth = 0_u32;
                    while dates.nth(calendar, nth) < next {
                        nth += 1;
                    }
                    let schedule = u32::try_from(family.schedules.len())
                        .unwrap_or_else(|_| violation!(clause = "TIME.4", "more schedules than a contract can name"));
                    family.schedules.push((dates, terms.ccy.index(), terms.payment_order.0));
                    Some((amount, schedule, nth))
                });
                let Some((amount, schedule, nth)) = read else {
                    violation!(clause = "SOC.3", "a pension line with no fixed amount", line = a.line.get());
                };
                let country = family.schedules.get(usize::try_from(schedule).unwrap_or(usize::MAX)).map(|s| s.1);
                let Some(Some(treasurer)) = country.and_then(|c| treasuries.get(usize::from(c))) else {
                    violation!(clause = "SOC.3", "a pension with no treasury to pay it", line = a.line.get());
                };
                let first = family
                    .schedules
                    .get(usize::try_from(schedule).unwrap_or(usize::MAX))
                    .map(|s| s.0.nth(calendar, nth));
                let phx_pop::person::Holder::Person(place) = a.holder else {
                    violation!(clause = "SOC.3", "a pension held by a household, not a person", line = a.line.get());
                };
                let person =
                    u32::try_from(place).unwrap_or_else(|_| violation!(clause = "REP.26", "a person beyond a place"));
                let due = Due { ends: [*treasurer, pensioner], amount, nth, schedule, person, pad: 0 };
                let _ = family.store.open(due, first);
            }
        }
        self.families.push(family);
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
