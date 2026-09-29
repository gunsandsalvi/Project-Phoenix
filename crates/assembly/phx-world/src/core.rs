//! The world on the core: every party of every kind in the core's stores with its identity — the institutions sited
//! by a tile, the firms by their record, each household by its attributes, positions and persons, each person with
//! its own identity — its account at its bank, and the dated families of contracts between them. The core draws its
//! own opening (`core_open`).

use phx_id::consts::NATURE_KIND;
use phx_id::{PartyId, PartyKey, Slot};
use phx_num::violation;
use phx_pop::persons::Persons;
use phx_store::{AddressSpace, SystemBacking};

use phx_core::store::KindStore;

/// Each kind's parties on the core, the persons of the kind that holds them, and every party's key by its identity,
/// sorted.
#[derive(Debug)]
pub struct Core {
    pub space: AddressSpace,
    /// The household kind's place among the population's kinds, which its processes are bound by.
    pub household_pop: usize,
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
    /// The next identity the core hands a party or person it begins.
    pub next_id: u64,
    /// Each country's banks on the core, by slot, each weighed by what its customers hold with it at the opening.
    pub banks_of: Vec<Vec<(u32, u64)>>,
    pub pop_days: Vec<(phx_id::Day, crate::core_pop::PopDay)>,
    /// Each day's events by kind, as the hazards' hits recorded them, and today's being counted.
    pub events: Vec<(phx_id::Day, Vec<crate::core_pop::EventCount>)>,
    pub(crate) events_today: Vec<crate::core_pop::EventCount>,
    pub labour: crate::core_labour::CoreLabour,
    pub goods: crate::core_goods::CoreGoods,
    pub state: crate::core_day::CoreState,
    /// Each country's statistics agency: its month's records and its releases.
    pub stats: crate::core_stats::CoreStats,
    /// Flows owed today beside the families' dues: severance at a separation, and the day's retail sales.
    pub pending: Vec<phx_core::flows::Flow>,
    /// Each country's lending rate and the fewest years a loan runs, at which a firm borrows its day's shortfall.
    pub(crate) lending: Vec<(phx_num::Rate, u64)>,
    /// The day's findings, which the world moves to the run's findings at the day's end.
    pub(crate) found: Vec<phx_core::findings::Finding>,
    /// The jobs and households' loans the opening drew, until the openings after the firms' take them.
    pub(crate) drawn: crate::core_open::Drawn,
    /// The realised rates over the sampled households, beside their expectations.
    pub rates: crate::core_rates::Rates,
}

pub(crate) fn kind_number(place: usize) -> u8 {
    match u8::try_from(place) {
        Ok(k) if k < NATURE_KIND => k,
        _ => violation!(clause = "REP.1", "more kinds of party than a key can name", kind = place),
    }
}

impl Core {
    /// The identity of a household's person at a place, none where it holds no one there.
    #[must_use]
    pub fn person_at(&self, household: PartyKey, place: usize) -> Option<u64> {
        let persons = self.persons.get(usize::from(household.kind()))?.as_ref()?;
        persons.of(household.slot()).nth(place).map(|p| p.id)
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
