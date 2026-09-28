//! The world's opening drawn on the core. Each country's central bank and treasury, sited together, and its banks, as
//! many as the Zipf law fitted to their concentration gives, each weighed by its share. Each country's households
//! drawn region by region from its people, each adult's labour, each household's banking and each pensioner's state
//! pension drawn by their systems' rules; every person given its identity. Each party's account from its country's
//! balance sheet: the banks' reserves by their shares, the treasury's deposits, the households' deposits over those
//! that bank by their wealth, and the currency the households hold over those that bank nowhere by their persons.
//! The state pensions are contracts from the treasury naming their person; the jobs and the households' loans are
//! kept for the jobs' dealing and the loans' opening, which follow the firms'.

use phx_core::calendar::Calendar;
use phx_core::settle::AT_ISSUER;
use phx_core::store::{KindStore, Opening};
use phx_core::{Household, OpeningCountry, OpeningCtx, Register, StreamDef, Streams};
use phx_id::{Day, PartyId, PartyKey};
use phx_macros::clause;
use phx_num::{MaybeI64, Missing, violation};
use phx_pop::kind::PopKindDecl;
use phx_pop::persons::{Held, Persons};
use phx_rand::float::len_u64;
use phx_store::{AddressSpace, SystemBacking};

use crate::consts::sheet::{BANKS, CURRENCY, DEPOSITS, GOVERNMENT, HOUSEHOLDS, LOANS_TO_HOUSEHOLDS, RESERVES};
use crate::consts::{
    AGENT_ROWS, AGENT_ROWS_PER_CHUNK, CORE_RANGE_BITS, CORE_WHEEL_DAYS, KIND_ROWS, KIND_ROWS_PER_CHUNK,
};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due, PENSION};
use crate::core_firms::apportion_amount;
use crate::opening::sheet::Sheet;

use crate::consts::kinds::{BANK, CENTRAL_BANK, ESTATE, FIRM, HOUSEHOLD, KINDS, TREASURY};

/// The kinds that hold an account.
const HOLDS_MONEY: [usize; 5] = [TREASURY, BANK, FIRM, ESTATE, HOUSEHOLD];

/// A job drawn at the opening, before its employer is dealt: its household, its person, its month's wage, its
/// country, its class — occupation, hours and the band its tenure began in — and its region.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpenJob {
    pub household: PartyKey,
    pub person: u64,
    pub amount: i64,
    pub country: u8,
    pub class: [u32; 3],
    pub region: u32,
}

/// A household's loan drawn at the opening: its household, its bank, its head, its weight by income, its country and
/// the years it has left.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OpenLoan {
    pub household: PartyKey,
    pub bank: PartyKey,
    pub person: u64,
    pub weight: u64,
    pub country: u8,
    pub years: u64,
}

/// What the opening drew for the openings that follow the firms'.
#[derive(Clone, Debug, Default)]
pub(crate) struct Drawn {
    pub jobs: Vec<OpenJob>,
    pub loans: Vec<OpenLoan>,
}

/// What the core's opening reads.
#[derive(Debug)]
pub struct CoreOpening<'a> {
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub sheets: &'a [Sheet],
    pub streams: &'a Streams,
    pub calendar: &'a Calendar,
    pub today: Day,
    /// The household kind and its place among the population's kinds.
    pub household: (&'a PopKindDecl, usize),
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
}

/// The rules the households' draws follow, from the register alone.
struct Rules {
    dem: sys_dem::Prims,
    jobs: sys_lab::Jobs,
    banks: sys_bnk::households::HouseholdLines,
    pensions: sys_soc::StatePension,
}

impl Rules {
    fn of(register: &Register) -> Result<Rules, String> {
        Ok(Rules {
            dem: sys_dem::Prims::of(register)?,
            jobs: sys_lab::Jobs::of(register)?,
            banks: sys_bnk::households::HouseholdLines::of(register)?,
            pensions: sys_soc::StatePension::of(register)?,
        })
    }
}

impl Core {
    /// The core with no party yet: each kind's store, the households' with their persons.
    fn empty(household: &PopKindDecl, household_pop: usize) -> Core {
        let mut space = AddressSpace::empty();
        let mut kinds = Vec::new();
        let mut persons = Vec::new();
        for (place, _) in KINDS.iter().enumerate() {
            let kind = kind_number(place);
            let (rows, chunk, stride) = if place == HOUSEHOLD {
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK, household.attrs.len() + household.positions.len())
            } else {
                (KIND_ROWS, KIND_ROWS_PER_CHUNK, 1)
            };
            let mut store: KindStore<SystemBacking> = KindStore::new(&mut space, kind, rows, chunk, stride);
            if HOLDS_MONEY.contains(&place) {
                store = store.with_accounts(&mut space, rows, chunk);
            }
            kinds.push(store);
            persons.push((place == HOUSEHOLD).then(|| Persons::new(&mut space, AGENT_ROWS, AGENT_ROWS_PER_CHUNK)));
        }
        Core {
            space,
            household_pop,
            names: KINDS.to_vec(),
            kinds,
            persons,
            keys: Vec::new(),
            issuers: Vec::new(),
            bank_kind: Some(kind_number(BANK)),
            range_bits: CORE_RANGE_BITS,
            families: Vec::new(),
            work: crate::core_day::Work::default(),
            days: Vec::new(),
            hazards: Vec::new(),
            household_decl: Some(household.clone()),
            persons_opened: 0,
            treasuries: Vec::new(),
            estates: Vec::new(),
            next_id: 1,
            banks_of: Vec::new(),
            pop_days: Vec::new(),
            labour: crate::core_labour::CoreLabour::default(),
            goods: crate::core_goods::CoreGoods::default(),
            state: crate::core_day::CoreState::default(),
            stats: crate::core_stats::CoreStats::default(),
            pending: Vec::new(),
            found: Vec::new(),
            lending: Vec::new(),
            drawn: Drawn::default(),
        }
    }

    /// A party begun on the core with the next identity, its record and its account where its kind holds money.
    fn begin_party(&mut self, place: usize, record: &[MaybeI64], account: Option<Opening>) -> PartyKey {
        let id = PartyId::new(self.next_id);
        self.next_id += 1;
        let Some(store) = self.kinds.get_mut(place) else {
            violation!(clause = "REP.1", "a party of a kind the core does not keep", kind = place);
        };
        let party = store.begin(id, record, account);
        let key = PartyKey::new(kind_number(place), party.slot());
        self.keys.push((id, key));
        key
    }

    /// The world's opening drawn on the core.
    ///
    /// # Errors
    /// A primitive the opening reads that the register does not hold.
    #[clause("GEN.2", "GEN.3", "GEN.4", "PTY.1", "PTY.3", "PTY.9", "REP.26", "BNK.1", "SOC.3", "MON.1")]
    pub fn open(o: &CoreOpening<'_>) -> Result<Core, String> {
        let (decl, household_pop) = o.household;
        let mut core = Core::empty(decl, household_pop);
        let rules = Rules::of(o.register)?;
        let ctx = OpeningCtx::new(o.streams, phx_core::CONTRACTS);
        let date = o.calendar.date(o.today);
        let mut pensions = core.pension_family(o.today);
        for (c, sheet) in o.countries.iter().zip(o.sheets) {
            let at = |instrument, sector| phx_ledger::opening::whole(sheet.at(instrument, sector) * c.gdp);
            let site = sys_cb::site(&ctx, c);
            let issuer = core.begin_party(CENTRAL_BANK, &[MaybeI64::present(i64::from(site.get()))], None);
            let treasury = core.begin_party(
                TREASURY,
                &[MaybeI64::present(i64::from(site.get()))],
                Some(Opening { bank: AT_ISSUER, balance: at(DEPOSITS, GOVERNMENT) }),
            );
            core.issuers.push(issuer);
            core.treasuries.push(Some(treasury));
            let (_, _, weights) = sys_bnk::bank_weights(c);
            let reserves = apportion_amount(at(RESERVES, BANKS), &weights);
            let mut banks = Vec::with_capacity(weights.len());
            for (k, balance) in (0_u32..).zip(reserves) {
                let site = sys_bnk::bank_site(&ctx, c, k);
                let record = [MaybeI64::present(i64::from(site.get()))];
                banks.push(core.begin_party(BANK, &record, Some(Opening { bank: AT_ISSUER, balance })));
            }
            let mut draw = CountryDraw {
                c,
                labour: rules.jobs.rule(o.register, date, c),
                banking: rules.banks.banking(o.register, c, weights),
                paid: rules.pensions.pensions(o.register, date, c),
                least: rules.banks.years(o.register).0,
                months: months_a_year(o.calendar, o.today, phx_ledger::opening::monthly(date, c.id)),
                schedule: pensions.schedule_for(c, date),
                treasury,
                banks,
                money: Money::default(),
                loans: Vec::new(),
            };
            for (region, formed) in sys_dem::draw_country(&rules.dem, o.register, (&ctx, date), c, decl) {
                for f in formed {
                    core.open_household((&ctx, o.calendar, decl), &mut draw, &mut pensions, (region, f));
                }
            }
            let CountryDraw { money, mut loans, banks, .. } = draw;
            core.open_deposits(c, &money, (at(DEPOSITS, HOUSEHOLDS), at(CURRENCY, HOUSEHOLDS)), &banks)?;
            // A loan's balance is what its payments repay over the years it has left, so each household's share of
            // the debt is its income's weight times those years, its payment then in proportion to its income.
            let debt = at(LOANS_TO_HOUSEHOLDS, BANKS);
            let weights: Vec<u64> = loans
                .iter()
                .map(|l| {
                    l.weight.checked_mul(l.years).unwrap_or_else(|| {
                        phx_num::capacity_exceeded!("a household loan's weight", u64::MAX, l.weight);
                    })
                })
                .collect();
            for (l, amount) in loans.iter_mut().zip(apportion_amount(debt, &weights)) {
                l.weight = u64::try_from(amount).unwrap_or(0);
            }
            core.drawn.loans.extend(loans);
        }
        core.families.push(pensions);
        core.persons_opened = core.persons_held();
        Ok(core)
    }

    /// A drawn household opened: each adult's labour, the household's banking and each pensioner's pension drawn by
    /// their rules, what they set written to its persons and to it, the household begun, and its jobs, pensions, money
    /// and loan kept or opened.
    fn open_household(
        &mut self,
        (ctx, calendar, decl): (&OpeningCtx<'_>, &Calendar, &PopKindDecl),
        draw: &mut CountryDraw<'_>,
        pensions: &mut DatedFamily,
        (region, formed): (u32, sys_dem::Formed),
    ) {
        let sys_dem::Formed { subject, mut h, drawn, .. } = formed;
        let labour = draw.labour.draw(&h, drawn.1, &mut ctx.draws(&sys_lab::JobsStream::DECL, subject));
        let banked =
            draw.banking.draw(&h, drawn, &mut ctx.draws(&sys_bnk::households::HouseholdsStream::DECL, subject));
        let pensioners = draw.paid.draw(&h, &mut ctx.draws(&sys_soc::PensionStream::DECL, subject));
        for l in &labour {
            let Some(p) = h.persons.get_mut(l.place) else {
                violation!(clause = "REP.26", "labour drawn for a person the household does not hold");
            };
            p.put_attr(sys_lab::LAST_POINT.name, l.last);
            p.put_attr(sys_lab::STATE.name, l.state);
            p.put_attr(sys_lab::OCCUPATION_ATTR.name, l.occupation);
        }
        if let sys_bnk::households::Banked::At { bank, .. } = banked {
            let Ok(place) = u32::try_from(bank + 1) else {
                violation!(clause = "REP.41", "a bank beyond the attribute's values");
            };
            h.set_attr(sys_bnk::households::BANK_ATTR.name, place);
        }
        // A year's income owed at the opening: the wages and pensions drawn, on the months of the year.
        let wages: i64 = labour.iter().filter_map(|l| l.job.as_ref()).map(|j| draw.labour.wage_at(j.point)).sum();
        let pension: i64 = pensioners.iter().filter_map(|(_, sex)| draw.paid.amount.get(*sex)).sum();
        let key = self.begin_household(decl, &h, (wages + pension) * draw.months, &banked, &draw.banks);
        let ids: Vec<u64> = self.persons_of(key);
        let country = draw.c.id.get();
        for l in &labour {
            let (Some(job), Some(person)) = (l.job.as_ref(), ids.get(l.place)) else { continue };
            let at = |i: usize| job.class.get(i).copied().unwrap_or(0);
            self.drawn.jobs.push(OpenJob {
                household: key,
                person: *person,
                amount: draw.labour.wage_at(job.point),
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
                if let (Some((years, weight)), Some(lender), Some(head)) = (loan, draw.banks.get(bank), ids.first()) {
                    draw.loans.push(OpenLoan {
                        household: key,
                        bank: *lender,
                        person: *head,
                        weight,
                        country,
                        years: draw.least + len_u64(years),
                    });
                }
            }
            sys_bnk::households::Banked::Unbanked { persons } => draw.money.unbanked.push((key.slot(), persons)),
        }
    }

    /// A household begun with its attributes, its positions as its kind opens them — its income a year as owed —
    /// its account at its bank, or the issuer where it banks nowhere, and its persons each given an identity.
    fn begin_household(
        &mut self,
        decl: &PopKindDecl,
        h: &Household,
        owed: i64,
        banked: &sys_bnk::households::Banked,
        banks: &[PartyKey],
    ) -> PartyKey {
        let mut record: Vec<MaybeI64> =
            decl.attrs.iter().map(|a| MaybeI64::present(i64::from(h.attr(a.item.name)))).collect();
        record.extend(decl.positions.iter().map(|p| match p.item.opening {
            phx_core::PositionOpening::OwedAYear => MaybeI64::present(owed),
            phx_core::PositionOpening::Missing => MaybeI64::ABSENT,
        }));
        let bank = match banked {
            sys_bnk::households::Banked::At { bank, .. } => match banks.get(*bank) {
                Some(k) => k.slot().get(),
                None => violation!(clause = "BNK.1", "a household's bank beyond its country's"),
            },
            sys_bnk::households::Banked::Unbanked { .. } => AT_ISSUER,
        };
        let key = self.begin_party(HOUSEHOLD, &record, Some(Opening { bank, balance: 0 }));
        let held: Vec<Held> = h
            .persons
            .iter()
            .map(|p| {
                let id = self.next_id;
                self.next_id += 1;
                Held { word: phx_pop::person::pack(decl, p), id }
            })
            .collect();
        if let Some(Some(p)) = self.persons.get_mut(HOUSEHOLD) {
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
        let Some(store) = self.kinds.get_mut(HOUSEHOLD) else { return Ok(()) };
        for ((slot, bank, _), balance) in money.banked.iter().zip(apportion_amount(deposits, &weights)) {
            let Some(b) = banks.get(*bank) else { continue };
            store.open_account(*slot, Opening { bank: b.slot().get(), balance });
            if let Some(w) = at_bank.get_mut(*bank) {
                *w += u64::try_from(balance).unwrap_or(0);
            }
        }
        let weights: Vec<u64> = money.unbanked.iter().map(|(_, n)| *n).collect();
        for ((slot, _), balance) in money.unbanked.iter().zip(apportion_amount(currency, &weights)) {
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
        DatedFamily {
            name: "SOC.pension",
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(TREASURY), kind_number(HOUSEHOLD)], [KIND_ROWS, AGENT_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [false, true],
                (today.succ(), CORE_WHEEL_DAYS),
            ),
            reason: PENSION,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
            finishing: Vec::new(),
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
        let found = self
            .schedules
            .iter()
            .zip(&self.classes)
            .zip(&self.terms)
            .position(|(((_, cc, _), k), t)| *cc == ccy && *k == class && t.as_ref() == Some(&terms));
        if let Some(at) = found {
            return u32::try_from(at).unwrap_or(u32::MAX);
        }
        self.schedules.push((terms.schedule.dates, ccy, terms.payment_order.0));
        self.classes.push(class);
        self.terms.push(Some(terms));
        u32::try_from(self.schedules.len() - 1).unwrap_or(u32::MAX)
    }

    /// A schedule's first date after the opening.
    pub(crate) fn first(&self, calendar: &Calendar, schedule: u32) -> Option<Day> {
        self.schedules.get(usize::try_from(schedule).ok()?).map(|s| s.0.nth(calendar, 1))
    }
}

/// The months of a monthly schedule's dates in the year after `today`.
fn months_a_year(calendar: &Calendar, today: Day, dates: phx_core::calendar::period::ScheduleDates) -> i64 {
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
