//! Credit on the core: the loans the country's sheet holds, each a dated contract from its borrower to its bank paying
//! interest on what it owes and an equal part of it on each monthly date that remains, as its terms' shape reckons.
//! A household's loan is the one the books' opening drew for it; a firm's is its share, by its turnover, of the
//! sheet's loans to firms, at the lending rate, over a term drawn between the declared years, lent by its own bank.

use phx_core::calendar::Calendar;
use phx_core::calendar::daycount::DayCount;
use phx_core::{OpeningCountry, Register, StreamDecl, Streams, opening_subject};
use phx_id::{Day, PartyKey, Slot};
use phx_ledger::algebra::{Leg, Reference, Schedule, Terms};
use phx_ledger::books::Books;
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_pop::population::Population;
use phx_rand::float::{from_i64, len_u64};
use phx_store::SystemBacking;

use crate::consts::firm::{LOANS_PURPOSE, OUTPUT, PRICE, PRODUCT, PURPOSES, REGION};
use crate::consts::reason::REPAID;
use crate::consts::{AGENT_ROWS, AGENT_ROWS_PER_CHUNK, CORE_WHEEL_DAYS, MONTHS};
use crate::core::{Core, kind_number};
use crate::core_day::{DatedFamily, Due};

/// What the loans' opening reads besides the core.
#[derive(Debug)]
pub struct CreditOpening<'a> {
    pub books: &'a Books,
    pub register: &'a Register,
    pub countries: &'a [OpeningCountry],
    pub sheets: &'a [crate::opening::sheet::Sheet],
    pub calendar: &'a Calendar,
    pub today: Day,
    pub streams: &'a Streams,
    pub stream: &'a StreamDecl,
}

/// The line kind a household's loan is drawn on at the books' opening.
const HOUSEHOLD_LOAN: &str = "household loan";

impl Core {
    /// The state's laws on the core from the state's, and its benefit's family: each country's income tax as bands of
    /// its mean wage, its consumption tax's rate and its benefit.
    #[clause("TAX.1", "SOC.1")]
    pub(crate) fn open_state(&mut self, state: &crate::state::State, today: Day) {
        let laws = self.labour.laws.clone();
        let mut s = crate::core_day::CoreState {
            claim: state.benefit.map(|k| k.claim),
            included: state.tax.map(|k| k.included),
            ..crate::core_day::CoreState::default()
        };
        for (i, c) in state.countries.iter().enumerate() {
            let tax = match &c.tax {
                Missing::Present(t) => Some(t.clone()),
                Missing::Absent => None,
            };
            let mean = laws.get(i).map(|l| l.mean_monthly * crate::consts::MONTHS_A_YEAR);
            let payee = self
                .treasuries
                .get(i)
                .copied()
                .flatten()
                .and_then(|k| self.kinds.get(usize::from(k.kind()))?.parties.id(k.slot()));
            s.withholding.push(tax.as_ref().zip(mean).zip(payee).map(|((t, mean), payee)| {
                phx_ledger::levy::Withholding {
                    kind: 0,
                    ccy: phx_ledger::opening::currency(phx_id::CountryId::new(u8::try_from(i).unwrap_or(u8::MAX))),
                    payee,
                    bands: t
                        .bands
                        .iter()
                        .map(|(edge, rate)| phx_ledger::levy::Band {
                            from: phx_ledger::opening::whole(edge * mean),
                            share: phx_ledger::opening::whole(rate * crate::consts::RATE_ONE),
                        })
                        .collect(),
                    periods: MONTHS,
                }
            }));
            s.consumption.push(tax.map(|t| t.consumption_rate));
            s.benefit.push(match c.benefit {
                Missing::Present(b) => Some(b),
                Missing::Absent => None,
            });
        }
        self.state = s;
        let (Some(treasury), Some(household)) =
            (self.names.iter().position(|n| *n == "treasury"), self.names.iter().position(|n| *n == "household"))
        else {
            return;
        };
        let family = DatedFamily {
            name: "SOC.benefit",
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(treasury), kind_number(household)], [AGENT_ROWS, AGENT_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [false, true],
                (today.succ(), CORE_WHEEL_DAYS),
            ),
            reason: crate::consts::reason::BENEFIT,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
        };
        self.families.push(family);
    }

    fn loan_family(&mut self, name: &'static str, borrower: usize, bank: usize, today: Day) -> DatedFamily {
        DatedFamily {
            name,
            store: phx_core::store::Family::new(
                &mut self.space,
                ([kind_number(borrower), kind_number(bank)], [AGENT_ROWS, AGENT_ROWS]),
                (AGENT_ROWS, AGENT_ROWS_PER_CHUNK),
                [true, true],
                (today.succ(), CORE_WHEEL_DAYS),
            ),
            reason: REPAID,
            schedules: Vec::new(),
            classes: Vec::new(),
            terms: Vec::new(),
            ends_after: Vec::new(),
        }
    }

    /// A party's bank on the core, by its account.
    fn bank_of(&self, party: PartyKey) -> Option<PartyKey> {
        let bank = self.bank_kind?;
        let b = self.kinds.get(usize::from(party.kind()))?.accounts.as_ref()?.bank.get(party.slot())?;
        (b != phx_core::settle::AT_ISSUER).then(|| PartyKey::new(bank, Slot::new(b)))
    }

    /// The loans opened on the core: the households' from the books, the firms' from the sheet.
    ///
    /// # Errors
    /// A primitive the opening reads that the register does not hold.
    #[clause("BNK.17", "GEN.2", "GEN.15")]
    pub fn open_loans(&mut self, o: &CreditOpening<'_>) -> Result<(), String> {
        let (Some(household), Some(firm), Some(bank)) = (
            self.names.iter().position(|n| *n == "household"),
            self.names.iter().position(|n| *n == "firm"),
            self.bank_kind.map(usize::from),
        ) else {
            return Ok(());
        };
        let households = self.household_loans(o, household, bank);
        let firms = self.firm_loans(o, firm, bank)?;
        self.families.push(households);
        self.families.push(firms);
        Ok(())
    }

    /// Each household's loans as the books drew them: its balance, its terms, and its next date; lent by its bank and
    /// owed by its head.
    fn household_loans(&mut self, o: &CreditOpening<'_>, household: usize, bank: usize) -> DatedFamily {
        let mut family = self.loan_family("BNK.household_loans", household, bank, o.today);
        let Some(pop_at) = household.checked_sub(usize::from(self.first_agents)) else { return family };
        let ledger = &o.books.ledger;
        let Ok(place) = u16::try_from(household) else { return family };
        let holder = o.books.parties.holder(place);
        let table = Population::table::<SystemBacking>(o.books.parties.cells(), pop_at);
        let mut schedules: std::collections::BTreeMap<phx_id::LineId, u32> = std::collections::BTreeMap::new();
        for slot in table.slots() {
            let Some(key) = self.key(table.party(slot)) else { continue };
            let Some(lender) = self.bank_of(key) else { continue };
            let Some(head) = self.person_at(key, 0) else { continue };
            for r in phx_ledger::rows::iter(holder, slot) {
                if r.side() != phx_ledger::algebra::Side::Liability
                    || ledger.lines.kind_name(r.row.line) != HOUSEHOLD_LOAN
                {
                    continue;
                }
                let Missing::Present(balance) = r.optional.balance else { continue };
                let owed = balance.abs();
                if owed == 0 {
                    continue;
                }
                let terms = ledger.terms.get(ledger.lines.terms(r.row.line)).clone();
                let (dates, next) = (terms.schedule.dates, ledger.lines.next_due(r.row.line));
                let mut nth = 0_u32;
                while dates.nth(o.calendar, nth) < next {
                    nth += 1;
                }
                let schedule = *schedules.entry(r.row.line).or_insert_with(|| {
                    family.schedules.push((dates, terms.ccy.index(), terms.payment_order.0));
                    family.classes.push([0, 0, 0]);
                    family.terms.push(Some(terms.clone()));
                    u32::try_from(family.schedules.len() - 1).unwrap_or(u32::MAX)
                });
                let due = Due { ends: [key, lender], amount: owed, nth, schedule, person: head };
                let _ = family.store.open(due, Some(dates.nth(o.calendar, nth)));
            }
        }
        family
    }

    /// Each firm's share of its country's loans to firms, by its turnover, lent by its bank at the lending rate over
    /// a term drawn between the declared years, repaid in equal parts with interest on what it owes.
    fn firm_loans(&mut self, o: &CreditOpening<'_>, firm: usize, bank: usize) -> Result<DatedFamily, String> {
        let mut family = self.loan_family("BNK.firm_loans", firm, bank, o.today);
        let (min, max) = (o.register.count("BNK.loan_years_min")?, o.register.count("BNK.loan_years_max")?);
        let date = o.calendar.date(o.today);
        for (c, sheet) in o.countries.iter().zip(o.sheets) {
            let lent = phx_ledger::opening::whole(
                sheet.at(crate::consts::sheet::LOANS_TO_FIRMS, crate::consts::sheet::BANKS) * c.gdp,
            );
            let rate = sys_bnk::rate(c.derived("GEN.lending_rate").ok_or("no lending rate")?);
            let ccy = phx_ledger::opening::currency(c.id);
            let mut firms: Vec<(PartyKey, u64)> = Vec::new();
            for slot in self.kinds.get(firm).map(|k| k.parties.live_slots().collect::<Vec<_>>()).unwrap_or_default() {
                let read = |at: usize| match self.kinds.get(firm)?.record(slot).get(at).map(|w| w.get()) {
                    Some(Missing::Present(v)) => Some(v),
                    _ => None,
                };
                let (Some(region), Some(price), Some(output), Some(product)) =
                    (read(REGION), read(PRICE), read(OUTPUT), read(PRODUCT))
                else {
                    continue;
                };
                if !c.regions.iter().any(|(r, _)| i64::from(*r) == region) {
                    continue;
                }
                let lot = sys_frm::FilingPrims::lot(o.register, u16::try_from(product).unwrap_or(u16::MAX));
                let turnover = phx_rand::float::floor_to_u64(from_i64(price) / lot * from_i64(output)).unwrap_or(0);
                firms.push((PartyKey::new(kind_number(firm), slot), turnover));
            }
            let weights: Vec<u64> = firms.iter().map(|(_, t)| *t).collect();
            let shares = crate::core_firms::apportion_amount(lent, &weights);
            let first_schedule = len_u64(family.schedules.len());
            for years in min..=max {
                let Some(months) = u32::try_from(years).ok().and_then(|y| y.checked_mul(u32::try_from(MONTHS).ok()?))
                else {
                    violation!(clause = "GEN.2", "a loan's term beyond a schedule's count", years = years);
                };
                let dates = phx_ledger::opening::monthly(date, c.id);
                let terms: Terms = phx_ledger::opening::plain_terms(
                    ccy,
                    vec![
                        Leg::RateOnNotional { reference: Reference::Fixed(rate), day_count: DayCount::Act365F },
                        Leg::Amortising,
                    ],
                    Schedule { dates, count: Missing::Present(months) },
                );
                family.schedules.push((dates, c.id.get(), terms.payment_order.0));
                family.classes.push([0, 0, 0]);
                family.terms.push(Some(terms));
            }
            let mut draws = o.streams.open(
                o.stream,
                opening_subject(u32::from(c.id.get()) * PURPOSES + LOANS_PURPOSE, 0),
                Day::new(0),
                0,
            );
            for ((key, _), principal) in firms.iter().zip(shares) {
                if principal <= 0 {
                    continue;
                }
                let Some(lender) = self.bank_of(*key) else { continue };
                let years = phx_rand::below_u64(&mut draws, max - min + 1);
                let schedule = u32::try_from(first_schedule + years).unwrap_or(u32::MAX);
                let Some((dates, _, _)) =
                    family.schedules.get(usize::try_from(schedule).unwrap_or(usize::MAX)).copied()
                else {
                    continue;
                };
                let due = Due { ends: [*key, lender], amount: principal, nth: 1, schedule, person: 0 };
                let _ = family.store.open(due, Some(dates.nth(o.calendar, 1)));
            }
        }
        Ok(family)
    }
}
