//! The world on the core: every party of every kind in the core's stores with its identity — the institutions sited
//! by a tile, the firms and households by their kinds' stores, each household with its persons, each person with its
//! own identity — its account at its bank, and the dated families of contracts between them. The core draws its
//! own opening (`core_open`).

use phx_core::person_word::PersonWord;
use phx_id::consts::NATURE_KIND;
use phx_id::{PartyKey, PartyRef, Slot};
use phx_num::violation;
use phx_pop::directory::Directory;
use phx_store::{AddressSpace, SystemBacking};

use phx_core::store::KindMoney;

/// The day's work counted for the budget: unit costs reckoned, and the meetings' weights reckoned and sales made.
#[derive(Debug, Default)]
pub struct RunCounts {
    pub unit_costs: phx_exec::Tally,
    pub weighed: phx_exec::Tally,
    pub sales: phx_exec::Tally,
}

/// Each kind's parties on the core: their slots and generations in the directory, their records in the kinds' stores,
/// and the persons of the kind that holds them.
#[derive(Debug, phx_macros::Saved)]
pub struct Core {
    #[saved(skip)]
    pub space: AddressSpace,
    /// The household kind's place among the population's kinds, which its processes are bound by.
    pub household_pop: usize,
    pub names: Vec<&'static str>,
    pub kinds: Vec<KindMoney<SystemBacking>>,
    /// Each kind's places, where its kind declares its parties placed by a site, a region or a country.
    pub places: Vec<Option<crate::place_store::PlaceStore>>,
    /// The persons, each a party of the person kind, threaded through their households.
    pub persons: Option<phx_pop::person_kind::PersonKind>,
    pub directory: Directory<SystemBacking>,
    /// The firms' own state on their kind's store, opened with the firms.
    pub firms: Option<crate::firm_store::FirmStore>,
    /// The households' own state on their kind's store, opened with the households.
    pub households: Option<crate::household_store::HouseholdStore>,
    /// The banks' books on their kind's store, opened with the banks.
    pub banks: Option<crate::bank_store::BankStore>,
    /// The agencies' staffing on their kind's store, opened with the agencies.
    pub agency_store: Option<crate::agency_store::AgencyStore>,
    /// Each country's central bank, the issuer of its currency, by the currency's index.
    pub issuers: Vec<PartyKey>,
    pub bank_kind: Option<u8>,
    pub range_bits: u32,
    pub families: Vec<crate::core_day::DatedFamily>,
    #[saved(skip)]
    pub work: crate::core_day::Work,
    /// What the run counts of the day's work for the budget: never saved, never read by the world.
    #[saved(skip, rebuild = Core::uncounted)]
    pub counts: RunCounts,
    pub days: Vec<crate::core_day::CoreDay>,
    pub hazards: Vec<crate::core_pop::Hazard>,
    #[saved(skip)]
    pub declared: crate::core_kinds::Declared,
    /// The kinds and families the day routes by, bound as they are opened and rebuilt after a load.
    #[saved(skip, rebuild = Core::bind_routes)]
    pub(crate) bound: crate::bound::Bound,
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
    /// Each kind's collectors' family, by the kind's place: found among the families, never saved.
    #[saved(skip, rebuild = Core::reindex_collectors)]
    pub(crate) collectors: Vec<Option<usize>>,
    /// The central banks' facilities, their positions' days, their income and the money they record owing.
    pub central: crate::core_central::Central,
    /// The decisions the world takes, who takes each, and how many each decider took.
    pub decisions: crate::core_decide::Decisions,
    /// The player's household, its intents and each day a decision came for it.
    pub player: crate::core_player::PlayerDesk,
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
    /// The kinds and families bound to their places among the core's.
    fn bind_routes(&mut self) -> u64 {
        self.bound = crate::bound::Bound::of(&self.names, &self.families);
        phx_rand::float::len_u64(self.families.len())
    }

    /// A family opened on the core, and bound among the families the day routes by.
    pub(crate) fn add_family(&mut self, family: crate::core_day::DatedFamily) {
        self.families.push(family);
        self.bound = crate::bound::Bound::of(&self.names, &self.families);
    }

    /// A core read back has counted nothing of the run yet.
    fn uncounted(&mut self) -> u64 {
        self.counts = RunCounts::default();
        0
    }

    /// The build's declarations a core read back from a save holds by reference, bound again as its opening bound
    /// them: they are code, which a save never holds.
    pub(crate) fn rebind(
        &mut self,
        (household, (kinds, heirless)): (Option<phx_pop::kind::PopKindDecl>, crate::core_kinds::Bound),
        state: &crate::state::State,
        (rate, index): (Option<crate::core_stats::Rate>, Option<if_state::stats::IndexKind>),
    ) {
        self.declared = crate::core_kinds::Declared { household, kinds, heirless, points: self.declared.points };
        self.state.claim = state.benefit.map(|k| k.claim);
        self.state.included = state.tax.map(|k| k.included);
        if self.bound.kinds.treasury.is_some() && self.bank_kind.is_some() {
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
        for (place, kind) in self.names.iter().enumerate() {
            let live = i64::try_from(self.directory.live(kind_number(place))).unwrap_or(i64::MAX);
            phx_exec::trace::note(kind, &[("parties", live)]);
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

/// A kind the opening begins parties of, bound from the declarations; a world that declares none stops.
pub(crate) fn declared_kind(kind: Option<usize>) -> usize {
    match kind {
        Some(k) => k,
        None => violation!(clause = "PTY.4", "the opening begins a kind the world does not declare"),
    }
}

/// A kind's number from its place among the kinds; a place past what a key names stops the run.
#[must_use]
pub fn kind_number(place: usize) -> u8 {
    match u8::try_from(place) {
        Ok(k) if k < NATURE_KIND => k,
        _ => violation!(clause = "REP.1", "more kinds of party than a key can name", kind = place),
    }
}

impl Core {
    /// The rows a kind's store reserves, its capacity's.
    pub(crate) fn kind_rows(&self, kind: usize) -> u32 {
        crate::core_kinds::of(&self.declared.kinds, kind).rows
    }

    /// The reference of the party at a key, live or ended this day; none for a slot never handed out.
    #[must_use]
    pub fn reference(&self, key: PartyKey) -> Option<PartyRef> {
        self.directory.reference(key.kind(), key.slot())
    }

    /// The live party at a key ended on a day, naming its estate where it leaves one; a party already ended stays so.
    pub(crate) fn end_party(&mut self, key: PartyKey, day: phx_id::Day, estate: Option<PartyKey>) {
        let successor = match estate.and_then(|e| self.reference(e)) {
            Some(e) => phx_num::Missing::Present(e),
            None => phx_num::Missing::Absent,
        };
        if let Some(party) = self.directory.at(key.kind(), key.slot()) {
            self.directory.end(party, day, successor);
        }
    }

    /// The live slots of the kind at a place among the kinds.
    pub fn live_slots(&self, place: usize) -> impl Iterator<Item = phx_id::Slot> + '_ {
        self.directory.live_slots(kind_number(place))
    }

    /// Whether the party at a key lives.
    #[must_use]
    pub fn lives(&self, key: PartyKey) -> bool {
        self.directory.is_live(key.kind(), key.slot())
    }

    /// The parties of a kind the core holds, by the kind's name.
    #[must_use]
    pub fn count(&self, name: &str) -> u64 {
        self.names.iter().position(|n| *n == name).map_or(0, |i| self.directory.live(kind_number(i)))
    }

    /// Each kind's store as the counters sample it, its live and ever-handed rows the directory's, and the directory
    /// itself.
    #[must_use]
    pub fn store_samples(&self) -> Vec<(&'static str, phx_exec::stats::Sample)> {
        // A kind's own words are on its kind's stores, beside its accounts in the core's.
        let own = |kind: u8| -> u64 {
            let stores = [
                self.firms.as_ref().filter(|f| f.kind() == kind).map(|f| &f.store),
                self.households.as_ref().filter(|h| h.kind() == kind).map(|h| &h.store),
                self.banks.as_ref().filter(|b| b.kind() == kind).map(|b| &b.store),
                self.agency_store.as_ref().filter(|a| a.kind() == kind).map(|a| &a.store),
                self.places.iter().flatten().find(|p| p.kind() == kind).map(|p| &p.store),
                self.persons.as_ref().filter(|p| p.store.kind() == kind).map(|p| &p.store),
            ];
            stores.into_iter().flatten().map(phx_store::StoreStats::bytes).sum()
        };
        let kinds = self.names.iter().zip(&self.kinds).enumerate().map(|(place, (name, store))| {
            let kind = kind_number(place);
            let (live, ever) = (self.directory.live(kind), u64::from(self.directory.high_water(kind)));
            let bytes = phx_store::StoreStats::bytes(store) + own(kind);
            (*name, phx_exec::stats::Sample { rows_live: live, rows_ever: ever, bytes })
        });
        kinds.chain([("directory", phx_exec::stats::Sample::of(&self.directory))]).collect()
    }

    /// The persons live on the core, every one of them held in a household.
    #[must_use]
    #[phx_macros::absent_is_zero(reason = "a world that declares no person kind holds no persons")]
    pub fn persons_held(&self) -> u64 {
        self.bound.kinds.person.map_or(0, |k| self.directory.live(kind_number(k)))
    }

    /// A household's persons, the newest first, each with its word; none for a household of no person.
    pub fn members(&self, household: Slot) -> impl Iterator<Item = (PartyRef, PersonWord)> + '_ {
        let (persons, households) = (self.persons.as_ref(), self.households.as_ref());
        let first = households.map_or(phx_num::Missing::Absent, |h| h.head(household));
        let kind = self.bound.kinds.person.map(kind_number);
        persons.into_iter().flat_map(move |p| p.members(first)).filter_map(move |slot| {
            let r = self.directory.reference(kind?, slot)?;
            Some((r, self.persons.as_ref()?.view_at(slot)?.word()))
        })
    }

    /// The persons a party holds: a household its members, any other party none.
    #[must_use]
    pub fn persons_in(&self, party: PartyKey) -> usize {
        match (self.persons.as_ref(), self.households.as_ref()) {
            (Some(ps), Some(hs)) if hs.kind() == party.kind() => ps.members(hs.head(party.slot())).count(),
            _ => 0,
        }
    }

    /// A person's word, where it is live and its household still holds it.
    #[must_use]
    pub fn person_word(&self, (household, id): (PartyKey, u64)) -> Option<PersonWord> {
        let view = self.persons.as_ref()?.view_at(self.person_of(id)?.slot())?;
        (view.household() == household.slot()).then(|| view.word())
    }

    /// A person's word written, where it is live and its household still holds it.
    pub(crate) fn write_person(&mut self, (household, id): (PartyKey, u64), word: PersonWord) {
        if self.person_word((household, id)).is_none() {
            return;
        }
        let Some(r) = self.person_of(id) else { return };
        if let Some(ps) = self.persons.as_mut() {
            ps.set_word(&self.directory, r, word);
        }
    }

    /// A person by its identity, its reference's word; none where it is not live.
    pub(crate) fn person_of(&self, id: u64) -> Option<PartyRef> {
        let r = PartyRef::from_word(id);
        (Some(r.kind()) == self.bound.kinds.person.map(kind_number) && self.directory.at(r.kind(), r.slot()) == Some(r))
            .then_some(r)
    }
}
