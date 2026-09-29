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
#[derive(Debug, phx_macros::Saved)]
pub struct Core {
    #[saved(skip)]
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
    #[saved(skip)]
    pub work: crate::core_day::Work,
    pub days: Vec<crate::core_day::CoreDay>,
    pub hazards: Vec<crate::core_pop::Hazard>,
    #[saved(skip)]
    pub household_decl: Option<phx_pop::kind::PopKindDecl>,
    /// The persons the households held when the core opened.
    pub persons_opened: u64,
    /// Each country's treasury, and the estates waiting to settle: each with its country and the day it opened.
    pub treasuries: Vec<Option<PartyKey>>,
    /// Each country's public agency, the state's producer of public administration, and the staff and appropriation
    /// each keeps.
    pub agencies: Vec<Option<PartyKey>>,
    pub agencies_kept: crate::core_agencies::Agencies,
    pub estates: Vec<(PartyKey, phx_id::CountryId, phx_id::Day)>,
    /// Why each estate that has paid what it can still stands: the goods it holds waiting for their liquidation.
    pub waiting: std::collections::BTreeMap<PartyKey, Waits>,
    /// Every death: its person, household, cause and where what it held and owed went.
    pub deaths: Vec<crate::core_pop::Death>,
    /// The insolvency law on the core: graces, contracts in arrears, estates' claims and firms ended.
    pub insolvency: crate::core_default::Insolvency,
    /// Each creditor's loan book, kept by what moves it.
    pub loan_books: std::collections::BTreeMap<PartyKey, crate::core_books::LoanBook>,
    /// Each firm's and bank's equity account and income.
    pub accounts: crate::core_accounts::Accounts,
    /// Each country's lending law, the firms' filed earnings and each bank's lending.
    pub credit: crate::core_lending::Credit,
    /// The sovereigns' bills: their auctions, issues and the debt they are.
    pub bills: crate::core_bills::Bills,
    /// The taxes arising, their collectors' debts and what was remitted.
    pub taxes: crate::core_taxes::Taxes,
    /// The central banks' facilities, their positions' days, their income and the money they record owing.
    pub central: crate::core_central::Central,
    /// The decisions the world takes, who takes each, and how many each decider took.
    pub decisions: crate::core_decide::Decisions,
    /// The player's household, its intents and each day a decision came for it.
    pub player: crate::core_player::PlayerDesk,
    /// The next identity the core hands a party or person it begins.
    pub next_id: u64,
    /// Each country's banks on the core, by slot, each weighed by what its customers hold with it at the opening.
    pub banks_of: Vec<Vec<(u32, u64)>>,
    pub pop_days: Vec<(phx_id::Day, crate::core_pop::PopDay)>,
    /// Each day's events by kind, as the hazards' hits recorded them, and today's being counted.
    pub events: Vec<(phx_id::Day, Vec<crate::core_pop::EventCount>)>,
    pub(crate) events_today: Vec<crate::core_pop::EventCount>,
    /// Every event, dated, with the parties it names, and whether the declared rule has made it public.
    pub happened: phx_core::EventStore,
    /// The regions' weather and today's struck tiles.
    pub weather: crate::core_weather::Weather,
    /// The deposits: their holders, what each has given and holds, and whether each extractor works them.
    pub deposits: crate::core_deposits::Deposits,
    /// The plant's kinds, the projects under way and the days' records.
    pub plant: crate::core_plant::Plant,
    /// Freight: the carriers' modes, the shipments on their way and the days' records.
    pub freight: crate::core_freight::Freight,
    /// Who owns each firm, who works in it as an owner, and who manages it.
    pub owners: crate::core_owners::Owners,
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
    /// The households whose persons the day's labour changed, booked again for every process from the next day.
    pub(crate) touched: std::collections::BTreeSet<u32>,
    /// Every amount the opening shared over parties by weight, and every closure that balanced a country's sheet, a
    /// share of its GDP: the opening report.
    pub apportioned: Vec<Apportioned>,
    pub closures: Vec<(u8, String, f64)>,
    /// Each day's stages and their time by the run's clock, for the bench; never world state, so never saved.
    #[saved(skip)]
    pub timings: Vec<(phx_id::Day, Vec<(&'static str, u64)>)>,
    #[saved(skip)]
    pub(crate) stage_ns: Vec<(&'static str, u64)>,
}

/// Why an estate that has paid what it can still stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub enum Waits {
    /// Its goods wait for their liquidation.
    Liquidation,
    /// Its money waits to be paid out.
    PayingOut,
}

/// An amount the opening shared over parties by their weights: what it was, in which country, the amount, what the
/// parties were given, how many there were and how many of no weight were given anything.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Apportioned {
    pub stratum: String,
    pub country: u8,
    pub total: i64,
    pub given: i64,
    pub parties: u64,
    pub unfounded: u64,
}

impl Core {
    /// The build's declarations a core read back from a save holds by reference, bound again as its opening bound
    /// them: they are code, which a save never holds.
    pub(crate) fn rebind(
        &mut self,
        household: Option<phx_pop::kind::PopKindDecl>,
        state: &crate::state::State,
        (rate, index): (Option<crate::core_stats::Rate>, Option<if_state::stats::IndexKind>),
    ) {
        self.household_decl = household;
        self.state.claim = state.benefit.map(|k| k.claim);
        self.state.included = state.tax.map(|k| k.included);
        if self.names.contains(&"treasury") && self.bank_kind.is_some() {
            self.bills.kind = state.bills;
        }
        self.stats.rate = rate;
        self.stats.index = rate.and(index);
    }

    /// A stage of the day run and timed by the run's clock where there is one, its time kept for today's timings, and
    /// told to the trace as it begins and ends; no outcome reads either.
    pub(crate) fn timed<T>(
        &mut self,
        clock: Option<&dyn phx_exec::Clock>,
        name: &'static str,
        stage: impl FnOnce(&mut Core) -> T,
    ) -> T {
        phx_exec::trace::span(name, || {
            let Some(clock) = clock else { return stage(self) };
            let start = clock.now_ns();
            let out = stage(self);
            if let Some(ns) = clock.now_ns().checked_sub(start) {
                self.stage_ns.push((name, ns));
            }
            out
        })
    }

    /// Each kind's parties, for the bench's trace.
    pub(crate) fn note_parties(&self) {
        for (kind, store) in self.names.iter().zip(&self.kinds) {
            phx_exec::trace::note(kind, &[("parties", phx_exec::trace::count(store.parties.live_slots().count()))]);
        }
    }

    /// An amount shared over weights exactly, and recorded in the opening report.
    pub(crate) fn apportion(
        &mut self,
        (stratum, country): (&'static str, u8),
        total: i64,
        weights: &[u64],
    ) -> Vec<i64> {
        let parts = crate::core_firms::apportion_amount(total, weights);
        let unfounded = weights.iter().zip(&parts).filter(|(w, p)| **w == 0 && **p != 0).count();
        self.apportioned.push(Apportioned {
            stratum: stratum.to_owned(),
            country,
            total,
            given: parts.iter().sum(),
            parties: phx_rand::float::len_u64(parts.len()),
            unfounded: phx_rand::float::len_u64(unfounded),
        });
        parts
    }
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
