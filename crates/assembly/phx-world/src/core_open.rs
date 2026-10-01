//! The world's opening drawn on the core. Each country's central bank and treasury, sited together, and its banks, as
//! many as the Zipf law fitted to their concentration gives, each weighed by its share. Each country's households
//! drawn region by region from its people, each adult's labour, each household's banking and each pensioner's state
//! pension drawn by their systems' rules; every person given its identity. Each party's account from its country's
//! balance sheet: the banks' reserves by their shares, the treasury's deposits, the households' deposits over those
//! that bank by their wealth, and the currency the households hold over those that bank nowhere by their persons.
//! The state pensions are contracts from the treasury naming their person; the jobs, the adults with an occupation
//! and no job, and the households' loans are kept for the jobs' dealing and the loans' opening, which follow the
//! firms', since a wage follows from its employer and a loan from its household's income.

use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::settle::AT_ISSUER;
use phx_core::store::{KindMoney, Opening};
use phx_core::{Household, OpeningCountry, OpeningCtx, Register, StreamDef, WorldStreams};
use phx_id::{Day, PartyKey};
use phx_macros::{clause, opening};
use phx_num::{Missing, violation};
use phx_pop::directory::Directory;
use phx_pop::kind::PopKindDecl;
use phx_pop::persons::{Held, Persons};
use phx_rand::float::len_u64;
use phx_store::{AddressSpace, SystemBacking};

use crate::bank_store::BankStore;
use crate::consts::sheet::{BANKS, CURRENCY, DEPOSITS, GOVERNMENT, HOUSEHOLDS, RESERVES};
use crate::consts::{AGENT_ROWS_PER_CHUNK, CORE_RANGE_BITS, KIND_ROWS_PER_CHUNK};
use crate::core::{Core, declared_kind, kind_number};
use crate::core_day::{DatedFamily, Due, PENSION};
use crate::household_store::{HouseholdOpening, HouseholdStore};
use crate::opening::asked::Asked;
use crate::opening::sheet::Sheet;
use crate::place_store::PlaceStore;
use phx_core::capacity::{AGENT_ROWS, WHEEL_DAYS};

/// A job drawn at the opening, before its employer is dealt: its household, its person, its country, its class —
/// occupation, hours and the band its tenure began in — and its region.
#[derive(Clone, Copy, Debug, phx_macros::Saved)]
pub(crate) struct OpenJob {
    pub household: PartyKey,
    pub person: u64,
    pub country: u8,
    pub class: [u32; 3],
    pub region: u32,
}

/// An adult drawn with an occupation and no job — searching, or working in a firm it owns: its household, its
/// person, its occupation and its country.
#[derive(Clone, Copy, Debug, phx_macros::Saved)]
pub(crate) struct OpenIdle {
    pub household: PartyKey,
    pub person: u64,
    pub occupation: u32,
    pub country: u8,
}

/// A household's loan drawn at the opening: its household, its bank, its head, its country and the years it has
/// left.
#[derive(Clone, Copy, Debug, phx_macros::Saved)]
pub(crate) struct OpenLoan {
    pub household: PartyKey,
    pub bank: PartyKey,
    pub person: u64,
    pub country: u8,
    pub years: u64,
}

/// What the opening drew for the openings that follow the firms', and the wages the jobs' dealing found: each
/// country's activities' rates, an hour weighed by its occupation's pay, and its occupations' pay.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub(crate) struct Drawn {
    pub jobs: Vec<OpenJob>,
    pub idle: Vec<OpenIdle>,
    pub owners: Vec<crate::core_owners::OpenOwner>,
    pub loans: Vec<OpenLoan>,
    pub rates: BTreeMap<(u8, usize), f64>,
    pub pay: BTreeMap<u8, Vec<f64>>,
}

impl Drawn {
    /// An occupation's wage an hour in a country's activity.
    pub(crate) fn wage_in(&self, (country, activity): (u8, usize), occupation: u32) -> Option<f64> {
        let rate = self.rates.get(&(country, activity))?;
        Some(rate * self.pay.get(&country)?.get(usize::try_from(occupation).ok()?)?)
    }
}

/// What the core's opening reads.
#[derive(Debug)]
pub struct CoreOpening<'a> {
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub sheets: &'a [Sheet],
    pub streams: &'a WorldStreams,
    pub calendar: &'a Calendar,
    pub today: Day,
    /// The household kind and its place among the population's kinds.
    pub household: (&'a PopKindDecl, usize),
    /// The kinds' declared traits and each country's heirless destination's kind.
    pub declared: &'a crate::core_kinds::Bound,
    /// The map, whose zones the households live in.
    pub geo: &'a phx_geo::GeoState,
}

/// A country's households' banking: each banked household with its bank's place and deposit's weight, and each
/// unbanked with its persons.
#[derive(Default)]
struct Money {
    banked: Vec<(phx_id::Slot, usize, u64)>,
    unbanked: Vec<(phx_id::Slot, u64)>,
}

/// A country's draw under way: its rules for labour, banking and pensions, the fewest years a household's loan has
/// left, the months of a year's income, the pensions' schedule, its treasury and banks, and its households' money and
/// loans drawn so far.
struct CountryDraw<'a> {
    c: &'a OpeningCountry,
    labour: sys_lab::Rule,
    banking: sys_bnk::households::Banking,
    paid: sys_soc::Pensions,
    least: u64,
    months: i64,
    schedule: u32,
    treasury: PartyKey,
    banks: Vec<PartyKey>,
    money: Money,
    loans: Vec<OpenLoan>,
    types: &'a phx_val::types::Types,
    date: phx_id::Date,
}

/// The rules the households' draws follow, from the register alone.
struct Rules {
    dem: sys_dem::Prims,
    jobs: sys_lab::Jobs,
    banks: sys_bnk::households::HouseholdLines,
    pensions: sys_soc::StatePension,
    types: phx_val::types::Types,
}

/// The days the directory keeps an ended party's tombstone.
#[opening]
fn tombstone_horizon(register: &Register) -> Result<u32, String> {
    u32::try_from(register.count(phx_pop::prims::TOMBSTONE_HORIZON_DAYS.id)?).map_err(|e| e.to_string())
}

impl Rules {
    fn of(register: &Register) -> Result<Rules, String> {
        Ok(Rules {
            dem: sys_dem::Prims::of(register)?,
            jobs: sys_lab::Jobs::of(register)?,
            banks: sys_bnk::households::HouseholdLines::of(register)?,
            pensions: sys_soc::StatePension::of(register)?,
            types: phx_val::types::Types::compile(register)?,
        })
    }
}

impl Core {
    /// The core with no party yet: each declared kind's store, in the catalogue's order, reserving its capacity's rows;
    /// the population kind's with its persons.
    #[opening]
    fn empty(
        (household, household_pop): (&PopKindDecl, usize),
        declared: &crate::core_kinds::Bound,
        (first, horizon): (Day, u32),
    ) -> Core {
        let mut space = AddressSpace::empty();
        let mut kinds = Vec::new();
        let mut places = Vec::new();
        let mut persons = Vec::new();
        for (k, traits) in declared.0.iter().enumerate() {
            let populated = traits.name == household.kind;
            let chunk = if populated { AGENT_ROWS_PER_CHUNK } else { KIND_ROWS_PER_CHUNK };
            let mut money: KindMoney<SystemBacking> = KindMoney::default();
            if traits.holds_money {
                money = money.with_accounts(&mut space, traits.rows, chunk);
            }
            kinds.push(money);
            // A kind placed by its zone holds it in its own store; every other its place store.
            places.push(PlaceStore::new(&mut space, kind_number(k), traits.place, traits.rows));
            persons.push(populated.then(|| Persons::new(&mut space, AGENT_ROWS, AGENT_ROWS_PER_CHUNK)));
        }
        let names: Vec<&'static str> = declared.0.iter().map(|k| k.name).collect();
        let capacities: Vec<u32> = declared.0.iter().map(|k| k.rows).collect();
        let directory = Directory::new(&mut space, &capacities, KIND_ROWS_PER_CHUNK, (first, horizon));
        let happened = phx_core::EventStore::new(
            &mut space,
            phx_core::capacity::EVENT_ROWS,
            crate::consts::EVENT_ROWS_PER_CHUNK,
            phx_core::capacity::ARENA_WORDS,
        );
        Core {
            space,
            household_pop,
            bound: crate::bound::Bound::of(&names, &[]),
            bank_kind: declared.0.iter().position(|k| k.takes_deposits).map(kind_number),
            names,

            kinds,
            places,
            persons,
            directory,
            firms: None,
            households: None,
            banks: None,
            agency_store: None,
            issuers: Vec::new(),
            range_bits: CORE_RANGE_BITS,
            families: Vec::new(),
            work: crate::core_day::Work::default(),
            counts: crate::core::RunCounts::default(),
            days: Vec::new(),
            hazards: Vec::new(),
            declared: crate::core_kinds::Declared {
                household: Some(household.clone()),
                kinds: declared.0.clone(),
                heirless: declared.1.clone(),
                points: crate::core_decide::Points::default(),
            },
            persons_opened: 0,
            treasuries: Vec::new(),
            agencies: Vec::new(),
            agencies_kept: crate::core_agencies::Agencies::default(),
            estates: Vec::new(),
            waiting: std::collections::BTreeMap::new(),
            deaths: Vec::new(),
            insolvency: crate::core_default::Insolvency::default(),
            loan_books: std::collections::BTreeMap::new(),
            accounts: crate::core_accounts::Accounts::default(),
            credit: crate::core_lending::Credit::default(),
            decisions: crate::core_decide::Decisions::default(),
            player: crate::core_player::PlayerDesk::default(),
            central: crate::core_central::Central::default(),
            taxes: crate::core_taxes::Taxes::default(),
            collectors: Vec::new(),
            bills: crate::core_bills::Bills::default(),
            next_id: 1,
            banks_of: Vec::new(),
            pop_days: Vec::new(),
            events: Vec::new(),
            events_today: Vec::new(),
            happened,
            weather: crate::core_weather::Weather::default(),
            deposits: crate::core_deposits::Deposits::default(),
            plant: crate::core_plant::Plant::default(),
            freight: crate::core_freight::Freight::default(),
            owners: crate::core_owners::Owners::default(),
            labour: crate::core_labour::CoreLabour::default(),
            goods: crate::core_goods::CoreGoods::default(),
            state: crate::core_day::CoreState::default(),
            stats: crate::core_stats::CoreStats::default(),
            pending: Vec::new(),
            found: Vec::new(),
            lending: Vec::new(),
            drawn: Drawn::default(),
            rates: crate::core_rates::Rates::default(),
            touched: std::collections::BTreeSet::new(),
            apportioned: Vec::new(),
            closures: Vec::new(),
            timings: Vec::new(),
            stage_ns: Vec::new(),
        }
    }

    /// A party begun on the core at the slot the directory hands out, with its place where its kind keeps one and its
    /// account where its kind holds money.
    pub(crate) fn begin_party(&mut self, place: usize, at: Option<u32>, account: Option<Opening>) -> PartyKey {
        let Some(store) = self.kinds.get_mut(place) else {
            violation!(clause = "REP.1", "a party of a kind the core does not keep", kind = place);
        };
        let party = store.begin(self.directory.begin(kind_number(place)), account);
        match (self.places.get_mut(place).and_then(Option::as_mut), at) {
            (Some(p), Some(at)) => p.begin(&self.directory, party, at),
            (None, None) => {}
            _ => violation!(
                clause = "PTY.5",
                "a party begun without its kind's place, or with one its kind keeps none of"
            ),
        }
        PartyKey::new(party.kind(), party.slot())
    }

    /// The world's opening drawn on the core.
    ///
    /// # Errors
    /// A primitive the opening reads that the register does not hold.
    #[clause("GEN.2", "GEN.3", "GEN.4", "PTY.1", "PTY.3", "PTY.9", "REP.26", "BNK.1", "SOC.3", "MON.1")]
    pub fn open(o: &CoreOpening<'_>) -> Result<Core, String> {
        let (decl, household_pop) = o.household;
        let mut core = Core::empty((decl, household_pop), o.declared, (o.today, tombstone_horizon(o.register)?));
        let rules = Rules::of(o.register)?;
        let ctx = OpeningCtx::new(o.streams, phx_core::CONTRACTS);
        let date = o.calendar.date(o.today);
        let mut pensions = core.pension_family(o.today);
        let bound = core.bound.kinds;
        let household = declared_kind(bound.household);
        let held = (crate::household_store::zone_regions(o.geo), crate::core_goods::months(date));
        let capacity = core.kind_rows(household);
        core.households =
            Some(HouseholdStore::new(&mut core.space, crate::core::kind_number(household), capacity, held));
        let (central_bank, treasury_kind, agency_kind, bank_kind) = (
            declared_kind(bound.central_bank),
            declared_kind(bound.treasury),
            declared_kind(bound.agency),
            declared_kind(bound.bank),
        );
        let rows = core.kind_rows(bank_kind);
        core.banks = Some(BankStore::new(&mut core.space, crate::core::kind_number(bank_kind), rows));
        let rows = core.kind_rows(agency_kind);
        core.agency_store =
            Some(crate::agency_store::AgencyStore::new(&mut core.space, crate::core::kind_number(agency_kind), rows));
        for (c, sheet) in o.countries.iter().zip(o.sheets) {
            core.closures.extend(sheet.closures.iter().map(|(name, share)| (c.id.get(), (*name).to_owned(), *share)));
            let at = |instrument, sector| phx_ledger::opening::whole(sheet.at(instrument, sector) * c.gdp);
            let site = sys_cb::site(&ctx, c);
            let issuer = core.begin_party(central_bank, Some(site.get()), None);
            let treasury = core.begin_party(
                treasury_kind,
                Some(site.get()),
                Some(Opening { bank: AT_ISSUER, balance: at(DEPOSITS, GOVERNMENT) }),
            );
            core.issuers.push(issuer);
            core.treasuries.push(Some(treasury));
            // Its public agency, sited with it, holds its account at the issuer and is funded as it pays.
            let agency = core.begin_party(agency_kind, Some(site.get()), Some(Opening { bank: AT_ISSUER, balance: 0 }));
            let Some(r) = core.reference(agency) else {
                violation!(clause = "PTY.1", "an agency begun the directory does not name", slot = agency.slot().get());
            };
            if let Some(a) = core.agency_store.as_mut() {
                a.begin(&core.directory, r);
            }
            core.agencies.push(Some(agency));
            let (_, _, weights) = sys_bnk::bank_weights(c);
            let reserves = core.apportion(("banks' reserves", c.id.get()), at(RESERVES, BANKS), &weights);
            let mut banks = Vec::with_capacity(weights.len());
            for (k, balance) in (0_u32..).zip(reserves) {
                let site = sys_bnk::bank_site(&ctx, c, k);
                let bank = core.begin_party(bank_kind, Some(site.get()), Some(Opening { bank: AT_ISSUER, balance }));
                let Some(r) = core.reference(bank) else {
                    violation!(clause = "PTY.1", "a bank begun the directory does not name", slot = bank.slot().get());
                };
                if let Some(bs) = core.banks.as_mut() {
                    bs.begin(&core.directory, r);
                }
                banks.push(bank);
            }
            let formed = sys_dem::draw_country(&rules.dem, o.register, (&ctx, date), c);
            let mut labour = rules.jobs.rule(o.register, date, c, Asked::of(o.register, c)?.by_occupation());
            labour.couple(formed.iter().flat_map(|(_, f)| f.iter().map(|f| &f.h)));
            let mut draw = CountryDraw {
                c,
                labour,
                banking: rules.banks.banking(o.register, c, weights),
                paid: rules.pensions.pensions(o.register, date, c),
                least: rules.banks.years(o.register).0,
                months: months_a_year(o.calendar, o.today, phx_ledger::opening::monthly(date, c.id)),
                schedule: pensions.schedule_for(c, date),
                treasury,
                banks,
                money: Money::default(),
                loans: Vec::new(),
                types: &rules.types,
                date,
            };
            for (region, formed) in formed {
                for f in formed {
                    core.open_household((&ctx, o.calendar, decl, o.geo), &mut draw, &mut pensions, (region, f));
                }
            }
            let CountryDraw { money, loans, banks, .. } = draw;
            core.open_deposits(c, &money, (at(DEPOSITS, HOUSEHOLDS), at(CURRENCY, HOUSEHOLDS)), &banks)?;
            core.drawn.loans.extend(loans);
        }
        core.add_family(pensions);
        core.persons_opened = core.persons_held();
        Ok(core)
    }

    /// A drawn household opened: each adult's labour, the household's banking and each pensioner's pension drawn by
    /// their rules, what they set written to its persons and to it, the household begun, and its jobs, pensions, money
    /// and loan kept or opened.
    fn open_household(
        &mut self,
        (ctx, calendar, decl, geo): (&OpeningCtx<'_>, &Calendar, &PopKindDecl, &phx_geo::GeoState),
        draw: &mut CountryDraw<'_>,
        pensions: &mut DatedFamily,
        (region, formed): (u32, sys_dem::Formed),
    ) {
        let sys_dem::Formed { subject, mut h, wealth, site, .. } = formed;
        let zone = match geo.zone_of(site) {
            Missing::Present(z) => u16::try_from(z.get()).ok(),
            Missing::Absent => None,
        };
        let Some(zone) = zone else {
            violation!(clause = "PTY.5", "a household sited on no zone's land", tile = site.get());
        };
        let labour = draw.labour.draw(&h, &mut ctx.draws(&sys_lab::JobsStream::DECL, subject));
        let banked =
            draw.banking.draw(&h, wealth, &mut ctx.draws(&sys_bnk::households::HouseholdsStream::DECL, subject));
        let pensioners = draw.paid.draw(&h, &mut ctx.draws(&sys_soc::PensionStream::DECL, subject));
        // Its outlook types by their shares, and its first stance by its taste alone, no heuristic being scored yet.
        let mut t = ctx.draws(&sys_hh::TypesStream::DECL, subject);
        let memory = phx_core::register::values::draw_type(&draw.types.memory, &mut t).get();
        let switching = phx_core::register::values::draw_type(&draw.types.switching, &mut t).get();
        let stance = phx_rand::below_u64(&mut t, phx_rand::float::len_u64(phx_val::heuristic::MENU.len()));
        let (Some(preference), Ok(stance)) = (sys_hh::preference_type(memory, switching), u8::try_from(stance)) else {
            violation!(clause = "VAL.22", "a household's types past its preference type's", memory = memory);
        };
        // Its age class is its head's, whose lived years weight its outlooks of public series.
        let head = h.persons.iter().find(|p| p.role == if_pop::HEAD.name).or_else(|| h.persons.first());
        let window = head.and_then(|p| u32::try_from(p.age_on(draw.date)).ok()).map(|age| draw.types.window_of(age));
        let Some(Missing::Present(window)) = window else {
            violation!(clause = "VAL.23", "a household drawn with no head of an age class");
        };
        let Ok(window) = u8::try_from(window) else {
            violation!(clause = "VAL.23", "an age class past its word", class = window);
        };
        for l in &labour {
            let Some(p) = h.persons.get_mut(l.place) else {
                violation!(clause = "REP.26", "labour drawn for a person the household does not hold");
            };
            p.put_attr(sys_lab::STATE.name, l.state);
            p.put_attr(sys_lab::OCCUPATION_ATTR.name, l.occupation);
        }
        // A year's income owed at the opening: the pensions drawn, on the months of the year; the wages join it as the
        // jobs are dealt to their employers.
        let pension: i64 = pensioners.iter().filter_map(|(_, sex)| draw.paid.amount.get(*sex)).sum();
        let opening = HouseholdOpening {
            zone,
            preference,
            stance,
            window: Missing::Present(window),
            income: pension * draw.months,
        };
        let key = self.begin_household(decl, (&h, &opening), &banked, &draw.banks);
        let ids: Vec<u64> = self.persons_of(key);
        let country = draw.c.id.get();
        for l in &labour {
            // Employed but no one's employee: self-employed, owning and working in a firm of its region.
            if l.job.is_none()
                && l.state != if_labour::class::SEARCHING
                && l.occupation != if_labour::class::NO_OCCUPATION
                && let Some(person) = ids.get(l.place)
            {
                self.drawn.owners.push(crate::core_owners::OpenOwner {
                    household: key,
                    person: *person,
                    occupation: l.occupation,
                    country,
                    region,
                });
            }
            if l.job.is_none()
                && l.occupation != if_labour::class::NO_OCCUPATION
                && let Some(person) = ids.get(l.place)
            {
                self.drawn.idle.push(OpenIdle { household: key, person: *person, occupation: l.occupation, country });
            }
            let (Some(job), Some(person)) = (l.job.as_ref(), ids.get(l.place)) else { continue };
            let at = |i: usize| job.get(i).copied().unwrap_or(0);
            self.drawn.jobs.push(OpenJob {
                household: key,
                person: *person,
                country,
                class: [at(if_labour::consts::OCCUPATION), at(if_labour::consts::HOURS), at(if_labour::consts::BAND)],
                region,
            });
        }
        for (place, sex) in &pensioners {
            let (Some(person), Some(amount)) = (ids.get(*place), draw.paid.amount.get(*sex)) else { continue };
            let schedule = draw.schedule;
            let due =
                Due { ends: [draw.treasury, key], amount: *amount, nth: 1, schedule, person: *person, arrears: 0 };
            let first = pensions.first(calendar, schedule);
            let _ = pensions.store.open(due, first);
        }
        match banked {
            sys_bnk::households::Banked::At { bank, deposit, loan } => {
                draw.money.banked.push((key.slot(), bank, deposit));
                if let (Some(years), Some(lender), Some(head)) = (loan, draw.banks.get(bank), ids.first()) {
                    draw.loans.push(OpenLoan {
                        household: key,
                        bank: *lender,
                        person: *head,
                        country,
                        years: draw.least + len_u64(years),
                    });
                }
            }
            sys_bnk::households::Banked::Unbanked { persons } => draw.money.unbanked.push((key.slot(), persons)),
        }
    }

    /// A household begun with its words from its opening — its income a year as owed — its account at its bank, or
    /// the issuer where it banks nowhere, and its persons each given an identity.
    fn begin_household(
        &mut self,
        decl: &PopKindDecl,
        (h, opening): (&Household, &HouseholdOpening),
        banked: &sys_bnk::households::Banked,
        banks: &[PartyKey],
    ) -> PartyKey {
        let bank = match banked {
            sys_bnk::households::Banked::At { bank, .. } => match banks.get(*bank) {
                Some(k) => k.slot().get(),
                None => violation!(clause = "BNK.1", "a household's bank beyond its country's"),
            },
            sys_bnk::households::Banked::Unbanked { .. } => AT_ISSUER,
        };
        let household = declared_kind(self.bound.kinds.household);
        let key = self.begin_party(household, None, Some(Opening { bank, balance: 0 }));
        let Some(r) = self.reference(key) else {
            violation!(clause = "PTY.1", "a household begun the directory does not name", slot = key.slot().get());
        };
        if let Some(hs) = self.households.as_mut() {
            hs.begin(&self.directory, r, opening);
        }
        let held: Vec<Held> = h
            .persons
            .iter()
            .map(|p| {
                let id = self.next_id;
                self.next_id += 1;
                Held { word: phx_pop::person::pack(decl, p), id }
            })
            .collect();
        if let Some(Some(p)) = self.persons.get_mut(household) {
            p.set(&mut self.space, key.slot(), &held);
        }
        key
    }

    /// The identities of a household's persons, in their places.
    fn persons_of(&self, household: PartyKey) -> Vec<u64> {
        self.persons
            .get(usize::from(household.kind()))
            .and_then(Option::as_ref)
            .map(|p| p.of(household.slot()).map(|x| x.id).collect())
            .unwrap_or_default()
    }

    /// A country's households' money: its deposits over those that bank by their wealth, each at its bank; its
    /// currency over those that bank nowhere by their persons, held at the issuer. Each bank weighed for the firms'
    /// choice by the deposits its households hold with it.
    #[clause("MON.14")]
    fn open_deposits(
        &mut self,
        c: &OpeningCountry,
        money: &Money,
        (deposits, currency): (i64, i64),
        banks: &[PartyKey],
    ) -> Result<(), String> {
        if (money.banked.is_empty() && deposits != 0) || (money.unbanked.is_empty() && currency != 0) {
            return Err(format!("country {}: households' money with no household to hold it", c.id.get()));
        }
        let weights: Vec<u64> = money.banked.iter().map(|(_, _, w)| *w).collect();
        let mut at_bank = vec![0_u64; banks.len()];
        let shares = self.apportion(("households' deposits", c.id.get()), deposits, &weights);
        let cash: Vec<u64> = money.unbanked.iter().map(|(_, n)| *n).collect();
        let cash = self.apportion(("households' currency", c.id.get()), currency, &cash);
        let Some(store) = self.kinds.get_mut(declared_kind(self.bound.kinds.household)) else { return Ok(()) };
        for ((slot, bank, _), balance) in money.banked.iter().zip(shares) {
            let Some(b) = banks.get(*bank) else { continue };
            store.open_account(*slot, Opening { bank: b.slot().get(), balance });
            if let Some(w) = at_bank.get_mut(*bank) {
                *w += u64::try_from(balance).unwrap_or(0);
            }
        }
        for ((slot, _), balance) in money.unbanked.iter().zip(cash) {
            store.open_account(*slot, Opening { bank: AT_ISSUER, balance });
        }
        let country = usize::from(c.id.get());
        if self.banks_of.len() <= country {
            self.banks_of.resize(country + 1, Vec::new());
        }
        if let Some(of) = self.banks_of.get_mut(country) {
            *of = banks.iter().zip(at_bank).map(|(b, w)| (b.slot().get(), w)).collect();
        }
        Ok(())
    }

    /// The state pensions' family: from each country's treasury to households, naming the person paid.
    fn pension_family(&mut self, today: Day) -> DatedFamily {
        let (treasury, household) =
            (declared_kind(self.bound.kinds.treasury), declared_kind(self.bound.kinds.household));
        let rows = [self.kind_rows(treasury), self.kind_rows(household)];
        DatedFamily {
            name: crate::consts::families::PENSION,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(treasury), kind_number(household)], rows),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [false, true],
                (today.succ(), WHEEL_DAYS),
            ),
            reason: PENSION,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
            alike: BTreeMap::new(),
            alike_upto: 0,
            moves: crate::core_day::LoanMoves::default(),
            lost: 0,
        }
    }
}

impl DatedFamily {
    /// A country's monthly schedule from the opening's date in the family, added once: its dates, currency and
    /// payment order.
    pub(crate) fn schedule_for(&mut self, c: &OpeningCountry, date: phx_id::Date) -> u32 {
        self.schedule_of((c, date), [0, 0, 0], None)
    }

    /// A schedule in the family for a country's monthly dates from the opening's date, or its terms' own, with a
    /// class, added where the family holds none alike.
    pub(crate) fn schedule_of(
        &mut self,
        (c, date): (&OpeningCountry, phx_id::Date),
        class: [u32; 3],
        terms: Option<phx_ledger::algebra::Terms>,
    ) -> u32 {
        let ccy = phx_ledger::opening::currency(c.id);
        let found = self
            .schedules
            .iter()
            .zip(&self.classes)
            .zip(&self.terms)
            .position(|(((_, cc, _), k), t)| *cc == ccy.index() && *k == class && *t == terms);
        if let Some(at) = found {
            return u32::try_from(at).unwrap_or(u32::MAX);
        }
        let dates = match &terms {
            Some(t) => t.schedule.dates,
            None => phx_ledger::opening::monthly(date, c.id),
        };
        let order = terms.as_ref().map_or_else(
            || {
                phx_ledger::opening::plain_terms(
                    ccy,
                    Vec::new(),
                    phx_ledger::algebra::Schedule { dates, count: Missing::Absent },
                )
                .payment_order
                .0
            },
            |t| t.payment_order.0,
        );
        self.schedules.push((dates, ccy.index(), order));
        self.classes.push(class);
        self.terms.push(terms);
        u32::try_from(self.schedules.len() - 1).unwrap_or(u32::MAX)
    }

    /// A schedule in the family for terms of their own dates in a currency, with a class, added where the family holds
    /// none alike.
    pub(crate) fn schedule_in(&mut self, ccy: u8, class: [u32; 3], terms: phx_ledger::algebra::Terms) -> u32 {
        let found = self.alike_to((ccy, class, terms.schedule.dates.anchor)).into_iter().find(|i| {
            let i = usize::try_from(*i).unwrap_or(usize::MAX);
            self.schedules.get(i).is_some_and(|(_, cc, _)| *cc == ccy)
                && self.classes.get(i) == Some(&class)
                && self.terms.get(i).is_some_and(|t| t.as_ref() == Some(&terms))
        });
        if let Some(at) = found {
            return at;
        }
        self.schedules.push((terms.schedule.dates, ccy, terms.payment_order.0));
        self.classes.push(class);
        self.terms.push(Some(terms));
        u32::try_from(self.schedules.len() - 1).unwrap_or(u32::MAX)
    }

    /// The schedules of a currency and class whose dates start from a date, in the order they were added; those added
    /// since the index was last read indexed first.
    fn alike_to(&mut self, key: (u8, [u32; 3], phx_id::Date)) -> Vec<u32> {
        while let (Some((dates, ccy, _)), Some(class)) =
            (self.schedules.get(self.alike_upto), self.classes.get(self.alike_upto))
        {
            let at = u32::try_from(self.alike_upto).unwrap_or(u32::MAX);
            self.alike.entry((*ccy, *class, dates.anchor)).or_default().push(at);
            self.alike_upto += 1;
        }
        self.alike.get(&key).cloned().unwrap_or_default()
    }

    /// A schedule's first date after the opening.
    pub(crate) fn first(&self, calendar: &Calendar, schedule: u32) -> Option<Day> {
        self.schedules.get(usize::try_from(schedule).ok()?).map(|s| s.0.nth(calendar, 1))
    }

    /// A country's monthly schedule from `anchor`, ending after `last` dates where it ends, found where the family
    /// holds it and added where not, so contracts begun alike share one.
    pub(crate) fn monthly_from(&mut self, anchor: phx_id::Date, country: phx_id::CountryId, last: Option<u32>) -> u32 {
        let dates = phx_ledger::opening::monthly(anchor, country);
        let found = self.alike_to((country.get(), [0, 0, 0], dates.anchor)).into_iter().find(|i| {
            let i = usize::try_from(*i).unwrap_or(usize::MAX);
            self.schedules.get(i).is_some_and(|s| s.0 == dates && s.1 == country.get())
                && self.classes.get(i) == Some(&[0, 0, 0])
                && self.terms.get(i).is_some_and(Option::is_none)
                && self.ends_after.get(i) == Some(&last)
        });
        if let Some(at) = found {
            return at;
        }
        self.schedules.push((dates, country.get(), 0));
        self.classes.push([0, 0, 0]);
        self.terms.push(None);
        self.ends_after.push(last);
        u32::try_from(self.schedules.len() - 1).unwrap_or(u32::MAX)
    }
}

/// The place of a schedule's first date after `day`.
pub(crate) fn next_after(dates: &phx_core::calendar::period::ScheduleDates, calendar: &Calendar, day: Day) -> u32 {
    let (from, to) = (dates.anchor, calendar.date(day));
    let months = (i64::from(to.year()) - i64::from(from.year())) * crate::consts::MONTHS + i64::from(to.month())
        - i64::from(from.month());
    // The dates run a month apart from the anchor, so the months since it place the search within a date or two.
    let mut n = match u32::try_from(months) {
        Ok(m) if m > 0 => m,
        _ => 1,
    };
    while n > 1 && dates.nth(calendar, n - 1) > day {
        n -= 1;
    }
    while dates.nth(calendar, n) <= day {
        n += 1;
    }
    n
}

/// The months of a monthly schedule's dates in the year after `today`.
pub(crate) fn months_a_year(calendar: &Calendar, today: Day, dates: phx_core::calendar::period::ScheduleDates) -> i64 {
    let date = calendar.date(today);
    let Some(next) = phx_id::Date::new(date.year() + 1, date.month(), date.day())
        .or_else(|| phx_id::Date::new(date.year() + 1, date.month(), date.day() - 1))
    else {
        violation!(clause = "TIME.2", "a year after the opening with no date");
    };
    let Some(year_on) = calendar.day(next) else {
        violation!(clause = "TIME.2", "a year after the opening before the epoch");
    };
    let mut n = 0_i64;
    let mut k = 1_u32;
    while dates.nth(calendar, k) <= year_on {
        if dates.nth(calendar, k) > today {
            n += 1;
        }
        k += 1;
    }
    n
}
