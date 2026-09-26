//! The state's first cut in the world: income tax withheld from wages as the dues pay them, bound to the books when
//! the world opens or loads; the consumption tax charged at the till; the benefit a person who loses its job claims,
//! ended when it is hired or its term runs out; and each country's bills sold at its weekly auction at the fund
//! stage, paid at maturity by the dues.

use std::collections::BTreeMap;

use if_state::kinds::{
    BenefitKind, BenefitLaw, Bid, BidIn, BillKind, BillLaw, ClaimIn, PaymentOrder, SizeIn, TaxKind, TaxLaw,
    TreasuryKind,
};
use phx_core::calendar::period::{Period, ScheduleDates};
use phx_core::{Declarations, OpeningCountry, Register, SubStep};
use phx_id::{CountryId, Day, LineId, PartyId};
use phx_ledger::algebra::{Leg, PaymentOrder as Order, Repayment, Schedule, Side};
use phx_ledger::apply::ApplyAt;
use phx_ledger::instruction::LegRec;
use phx_macros::clause;
use phx_num::{Ccy, Missing, Money, violation};

use crate::world::World;

/// The days of a week in order, the first the week's first.
const WEEK: [phx_id::Weekday; 7] = [
    phx_id::Weekday::Monday,
    phx_id::Weekday::Tuesday,
    phx_id::Weekday::Wednesday,
    phx_id::Weekday::Thursday,
    phx_id::Weekday::Friday,
    phx_id::Weekday::Saturday,
    phx_id::Weekday::Sunday,
];

/// What the state carries across days: each benefit line by its country and monthly amount; each country's
/// treasury's cash at its last auction; the face of bills each country has issued and redeemed; and each bill line's
/// maturity.
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub(crate) struct StateBook {
    pub benefits: BTreeMap<(u8, i64), LineId>,
    pub cash: BTreeMap<u8, i64>,
    pub issued: BTreeMap<u8, i64>,
    pub redeemed: BTreeMap<u8, i64>,
    pub bills: BTreeMap<LineId, Day>,
}

/// A bill auction's result: the country, the face offered and bid, the face sold and its price.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Auction {
    pub country: u8,
    pub offered: i64,
    pub bid: i64,
    pub sold: i64,
    pub price: f64,
}

/// The day's state: the consumption tax charged, each with its seller and its base; the claims made; the auctions.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StateDay {
    pub consumption: Vec<(PartyId, i64, i64)>,
    pub claims: u64,
    pub benefits_ended: u64,
    pub auctions: Vec<Auction>,
}

/// Each country's state: its taxes, its benefit, its bills, its payment order and its treasury.
#[derive(Clone, Debug)]
pub(crate) struct Country {
    pub tax: Missing<TaxLaw>,
    pub benefit: Missing<BenefitLaw>,
    pub bills: Missing<BillLaw>,
    pub order: Missing<PaymentOrder>,
}

/// The state as the world keeps it: its kinds, each country's law, its book and the day's tally.
#[derive(Debug, Default)]
pub(crate) struct State {
    pub tax: Option<TaxKind>,
    pub benefit: Option<BenefitKind>,
    pub bills: Option<BillKind>,
    pub countries: Vec<Country>,
    pub book: StateBook,
    pub day: StateDay,
}

/// The one kind of a type the systems declare, if any.
fn one<T: Copy + 'static>(d: &Declarations) -> Result<Option<T>, String> {
    let kinds: Vec<T> = d.markets.iter().filter_map(|(_, k)| k.downcast_ref::<T>()).copied().collect();
    match kinds.as_slice() {
        [] => Ok(None),
        [k] => Ok(Some(*k)),
        _ => Err(format!("more than one {}", std::any::type_name::<T>())),
    }
}

/// A kind found, its error kept.
fn take<T>(r: Result<Option<T>, String>, errors: &mut Vec<String>) -> Option<T> {
    r.unwrap_or_else(|e| {
        errors.push(e);
        None
    })
}

/// A country's law compiled, its error kept; none where no system declares its kind.
fn compiled<T>(what: &str, c: &OpeningCountry, r: Option<Result<T, String>>, errors: &mut Vec<String>) -> Missing<T> {
    match r {
        Some(Ok(v)) => Missing::Present(v),
        Some(Err(e)) => {
            errors.push(format!("{what} in country {}: {e}", c.id.get()));
            Missing::Absent
        }
        None => Missing::Absent,
    }
}

/// The state's kinds the systems declare, with each country's law.
pub(crate) fn bind(
    d: &Declarations,
    register: &Register,
    countries: &[OpeningCountry],
    book: StateBook,
) -> Result<State, Vec<String>> {
    let mut errors = Vec::new();
    let tax = take(one::<TaxKind>(d), &mut errors);
    let benefit = take(one::<BenefitKind>(d), &mut errors);
    let bills = take(one::<BillKind>(d), &mut errors);
    let treasury = take(one::<TreasuryKind>(d), &mut errors);
    let countries: Vec<Country> = countries
        .iter()
        .map(|c| Country {
            tax: compiled("the taxes", c, tax.map(|k| (k.law)(register, c)), &mut errors),
            benefit: compiled("the benefit", c, benefit.map(|k| (k.law)(register, c)), &mut errors),
            bills: compiled("the bills", c, bills.map(|k| (k.law)(register, c)), &mut errors),
            order: compiled("the payment order", c, treasury.map(|k| (k.order)(register, c)), &mut errors),
        })
        .collect();
    if errors.is_empty() {
        Ok(State { tax, benefit, bills, countries, book, day: StateDay::default() })
    } else {
        Err(errors)
    }
}

impl World {
    /// A country's treasury.
    #[clause("TRS.1")]
    fn treasury_of(&self, country: CountryId) -> Option<PartyId> {
        self.books.parties.of_kind("treasury").find(|t| self.country_of_party(*t) == Missing::Present(country))
    }

    /// Income tax bound to the books as the world opens or loads: withheld from every payment on the taxed line kind
    /// in each country's currency, into its treasury's account, by its bands over a member's yearly wage.
    #[clause("TAX.2", "TAX.7")]
    pub(crate) fn state_rebuild(&mut self) {
        let Some(tax) = self.state.tax else { return };
        let kind = self.books.ledger.lines.kind_index(tax.withheld_from);
        let mut withholding = Vec::new();
        for (i, c) in self.state.countries.iter().enumerate() {
            let Missing::Present(law) = &c.tax else { continue };
            let Ok(id) = u8::try_from(i) else { continue };
            let country = CountryId::new(id);
            let Some(treasury) = self.treasury_of(country) else { continue };
            let Some(mean) = self.labour.laws.get(i).map(|l| l.mean_monthly * crate::consts::MONTHS_A_YEAR) else {
                continue;
            };
            let scale = crate::consts::RATE_ONE;
            let bands = law
                .bands
                .iter()
                .map(|(edge, rate)| phx_ledger::levy::Band {
                    from: phx_ledger::opening::whole(edge * mean),
                    share: phx_ledger::opening::whole(rate * scale),
                })
                .collect();
            withholding.push(phx_ledger::levy::Withholding {
                kind,
                ccy: phx_ledger::opening::currency(country),
                payee: treasury,
                bands,
                periods: crate::consts::MONTHS,
            });
        }
        self.books.withholding = withholding;
    }

    /// The consumption tax on a sale at the till: its rate's share of the price paid, which the seller owes its
    /// country's treasury, a whole share for each of the seller's twins, paid in the sale's instruction.
    #[clause("TAX.1", "TAX.2", "TAX.5", "TAX.7")]
    pub(crate) fn consumption_tax(&mut self, seller: PartyId, (amount, ccy): (i64, Ccy), legs: &mut Vec<LegRec>) {
        let Missing::Present(country) = self.country_of_party(seller) else { return };
        let Some(Country { tax: Missing::Present(law), .. }) = self.state.countries.get(usize::from(country.get()))
        else {
            return;
        };
        let rate = law.consumption_rate;
        let Some(tax_kind) = self.state.tax else { return };
        let Some(treasury) = self.treasury_of(country) else { return };
        let raw = phx_ledger::opening::whole((tax_kind.included)(phx_rand::float::from_i64(amount), rate));
        let unit = i64::from(self.books.parties.unit(seller));
        let tax = raw - raw.rem_euclid(unit);
        if tax > 0 {
            self.books.pay_into(seller, treasury, (tax, ccy), legs);
            self.state.day.consumption.push((seller, amount, tax));
        }
    }

    /// A person who lost its job claims the benefit where it is worth the hours claiming takes: its row on its
    /// country's benefit line of its share of the wage it lost, joined with as many of the treasury's members.
    #[clause("SOC.3", "SOC.7", "LAB.12")]
    pub(crate) fn claim_benefit(
        &mut self,
        day: Day,
        (party, person): (PartyId, u32),
        (country, wage): (CountryId, f64),
    ) {
        let Some(kind) = self.state.benefit else { return };
        let Some(Country { benefit: Missing::Present(law), order, .. }) =
            self.state.countries.get(usize::from(country.get())).cloned()
        else {
            return;
        };
        let Some(labour) = self.labour.laws.get(usize::from(country.get())) else { return };
        let hour = wage / (labour.weeks_a_month * f64::from(labour.full_time_hours));
        let monthly = phx_ledger::opening::whole(law.replacement * wage);
        let claims = (kind.claim)(&ClaimIn {
            monthly: phx_rand::float::from_i64(monthly),
            months: f64::from(law.months),
            claiming_cost: law.claim_hours * hour,
        });
        if !claims || monthly <= 0 {
            return;
        }
        let Some(treasury) = self.treasury_of(country) else { return };
        let line = self.benefit_line(&kind, country, (monthly, law.months), order);
        let twins = self.books.parties.unit(party);
        let Missing::Present(reason) =
            self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.claimed))
        else {
            violation!(clause = "SOC.3", "claims under a reason never declared");
        };
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        if self
            .books
            .members_join((party, line, Side::Asset), treasury, twins, (reason, m), self.audit.stream())
            .is_err()
        {
            return;
        }
        self.attach(party, person, line);
        self.state.day.claims += 1;
    }

    /// A country's benefit line of a monthly amount, opened the first time a claim needs it: paid monthly for the
    /// benefit's months, at the benefits' rank in the treasury's payment order.
    fn benefit_line(
        &mut self,
        kind: &BenefitKind,
        country: CountryId,
        (monthly, months): (i64, u32),
        order: Missing<PaymentOrder>,
    ) -> LineId {
        if let Some(l) = self.state.book.benefits.get(&(country.get(), monthly)) {
            return *l;
        }
        let ccy = phx_ledger::opening::currency(country);
        let dates = phx_ledger::opening::monthly(self.calendar.date(self.today), country);
        let mut terms = phx_ledger::opening::plain_terms(
            ccy,
            vec![Leg::FixedAmount(Money::new(monthly, ccy))],
            Schedule { dates, count: Missing::Present(months) },
        );
        if let Missing::Present(o) = order {
            terms.payment_order = Order(o.benefits);
        }
        let id = self.books.ledger.terms.intern(terms);
        let mut n = 1_u32;
        while dates.nth(&self.calendar, n) <= self.today {
            n += 1;
        }
        let k = self.books.ledger.lines.kind_index(kind.line);
        let line = self.books.ledger.lines.open(k, id, Missing::Present((dates.nth(&self.calendar, n), n)));
        self.state.book.benefits.insert((country.get(), monthly), line);
        line
    }

    /// A hired person's benefit ended: its row's members leave the benefit's line with as many of the treasury's.
    #[clause("SOC.3")]
    pub(crate) fn end_benefit(&mut self, day: Day, (party, person): (PartyId, u32)) {
        let Some(kind) = self.state.benefit else { return };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let (place, slot) = self.books.parties.row(party);
        let first = self.books.parties.first_cell_place();
        let Some(table_kind) = place.checked_sub(first).map(usize::from) else { return };
        let table =
            phx_pop::population::Population::table::<phx_store::SystemBacking>(self.books.parties.cells(), table_kind);
        let on: Vec<LineId> = table
            .attachments(slot)
            .iter()
            .map(|w| phx_pop::person::Attachment::unpack(*w))
            .filter(|a| {
                matches!(a.holder, phx_pop::person::Holder::Person(i) if u32::try_from(i).ok() == Some(person))
                    && self.books.ledger.lines.kind_of(a.line) == k
            })
            .map(|a| a.line)
            .collect();
        if on.is_empty() {
            return;
        }
        let twins = self.books.parties.unit(party);
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S4a));
        let Some(stream) = self.streams.named("LAB.layoff") else { return };
        let mut d = self.streams.open(
            &stream,
            phx_rand::Subject::new(phx_rand::SubjectTag::Party, party.get()),
            day,
            SubStep::S4a.ordinal(),
        );
        for line in on {
            if self.books.members_leave((party, line, Side::Asset), twins, m, &mut d, self.audit.stream()).is_ok() {
                let _ = self.detach(&[(party, line, Side::Asset, twins)], &mut d);
                self.state.day.benefits_ended += 1;
            }
        }
    }

    /// Each country's bill auction on its weekday, at the fund stage before the facilities: the face the treasury's
    /// plan offers, the banks' bids from their reserves above their targets, and the uniform-price clearing, each
    /// winner's contracts written and its price paid to the treasury.
    #[clause("SOV.3", "SOV.4", "SOV.6", "TRS.4", "REG.11")]
    pub(crate) fn bill_auctions(&mut self, day: Day) {
        let Some(kind) = self.state.bills else { return };
        for (i, c) in self.state.countries.clone().into_iter().enumerate() {
            let (Missing::Present(law), Ok(id)) = (c.bills, u8::try_from(i)) else { continue };
            let country = CountryId::new(id);
            let weekday = WEEK.iter().position(|w| *w == self.calendar.date(day).weekday());
            if !self.calendar.is_business(country, day) || weekday != usize::try_from(law.weekday).ok() {
                continue;
            }
            self.auction(day, (country, &kind, law), c.order);
        }
    }

    /// One country's auction.
    fn auction(
        &mut self,
        day: Day,
        (country, kind, law): (CountryId, &BillKind, BillLaw),
        order: Missing<PaymentOrder>,
    ) {
        let Some(treasury) = self.treasury_of(country) else { return };
        let ccy = phx_ledger::opening::currency(country);
        let cash = self.books.money_held(treasury, ccy);
        let Missing::Present(cash) = cash else { return };
        let before = self.state.book.cash.insert(country.get(), cash);
        let outflow = before.map_or(0, |b| b - cash);
        let Some(period) = Period::weeks(law.weeks) else {
            violation!(clause = "SOV.1", "a bill's weeks beyond a period", weeks = law.weeks);
        };
        let Some(week) = Period::weeks(1) else { violation!(clause = "TIME.4", "a week that is no period") };
        let next = self.calendar.plus(day, week);
        let maturing: i64 = self
            .state
            .book
            .bills
            .iter()
            .filter(|(_, d)| **d > day && **d <= next)
            .map(|(l, _)| self.bill_outstanding(*l, treasury))
            .sum();
        let offered = (kind.size)(&SizeIn {
            cash: phx_rand::float::from_i64(cash),
            outflow: phx_rand::float::from_i64(outflow),
            maturing: phx_rand::float::from_i64(maturing),
            buffer_weeks: law.buffer_weeks,
        });
        let contracts = phx_ledger::opening::whole(offered / phx_rand::float::from_i64(law.face));
        if contracts <= 0 {
            return;
        }
        let Some(corridor) = self.central.corridors.get(usize::from(country.get())).copied() else { return };
        let years = f64::from(law.weeks) / crate::consts::WEEKS_A_YEAR;
        let floor_price = 1.0 / (1.0 + corridor.deposit_rate * years);
        let banks: Vec<PartyId> = self
            .books
            .parties
            .of_kind("bank")
            .filter(|b| self.country_of_party(*b) == Missing::Present(country))
            .collect();
        let mut bids = Vec::new();
        for (n, bank) in (0_u32..).zip(&banks) {
            let excess = self.bank_excess(*bank, ccy);
            for (price, face) in (kind.bid)(&BidIn { excess, floor_price }) {
                let whole = phx_ledger::opening::whole(face / price / phx_rand::float::from_i64(law.face));
                bids.push(Bid { bidder: n, price, face: whole });
            }
        }
        let bid: i64 = bids.iter().map(|b| b.face).sum();
        let result = (kind.clear)(contracts, &bids);
        let sold: i64 = result.as_ref().map_or(0, |a| a.won.iter().map(|(_, f)| f).sum());
        self.state.day.auctions.push(Auction {
            country: country.get(),
            offered: contracts * law.face,
            bid: bid * law.face,
            sold: sold * law.face,
            price: result.as_ref().map_or(0.0, |a| a.price),
        });
        let Some(a) = result.filter(|a| !a.won.is_empty()) else { return };
        let line = self.bill_line(kind, (country, ccy), (law, period), order);
        let Missing::Present(reason) = self.books.ledger.reasons.coded(phx_ledger::instruction::name_code(kind.sold))
        else {
            violation!(clause = "SOV.6", "bills sold under a reason never declared");
        };
        let m = crate::agents::move_at(&self.register, day, ApplyAt::Day(SubStep::S8d));
        for (n, won) in a.won {
            let Some(bank) = usize::try_from(n).ok().and_then(|i| banks.get(i)).copied() else { continue };
            let Ok(count) = u32::try_from(won) else { continue };
            let face = won * law.face;
            let price = phx_ledger::opening::whole(phx_rand::float::from_i64(face) * a.price);
            let moves = [(bank, line, Side::Asset, count, face), (treasury, line, Side::Liability, count, -face)];
            let pays = [(bank, treasury, price)];
            if self
                .books
                .move_rows(moves.into_iter(), (ccy, Missing::Present(&pays)), (reason, m), self.audit.stream())
                .is_ok()
            {
                *self.state.book.issued.entry(country.get()).or_insert(0) += face;
            }
        }
    }

    /// What a bank holds in reserves above its target, as the fund stage reads it.
    fn bank_excess(&self, bank: PartyId, ccy: Ccy) -> f64 {
        let Some(ratio) = self.central.book.targets.get(&bank).copied() else { return 0.0 };
        let Some(kind) = self.central.kind else { return 0.0 };
        let lines = &self.books.ledger.lines;
        let k = lines.kind_index(kind.reserves);
        let (place, slot) = self.books.parties.row(bank);
        let reserves: i64 = phx_ledger::rows::rows(self.books.parties.holder(place), slot)
            .into_iter()
            .filter(|r| r.side() == Side::Asset && lines.kind_of(r.row.line) == k)
            .map(|r| if let Missing::Present(b) = r.optional.balance { b } else { 0 })
            .sum();
        let _ = ccy;
        let deposits = self.deposits_of(bank);
        phx_rand::float::from_i64(reserves) - ratio * phx_rand::float::from_i64(deposits)
    }

    /// The face still owed on a bill line: the treasury's balance on it.
    fn bill_outstanding(&self, line: LineId, treasury: PartyId) -> i64 {
        let (place, slot) = self.books.parties.row(treasury);
        match phx_ledger::rows::find(self.books.parties.holder(place), slot, line, Side::Liability)
            .map(|r| r.optional.balance)
        {
            Some(Missing::Present(b)) => -b,
            _ => 0,
        }
    }

    /// An auction's bill line: its face paid at maturity, the weeks after the auction, at the debt service's rank in
    /// the treasury's payment order.
    fn bill_line(
        &mut self,
        kind: &BillKind,
        (country, ccy): (CountryId, Ccy),
        (law, period): (BillLaw, Period),
        order: Missing<PaymentOrder>,
    ) -> LineId {
        let dates = ScheduleDates {
            anchor: self.calendar.date(self.today),
            period,
            eom: phx_core::calendar::period::EndOfMonth::Plain,
            convention: phx_core::calendar::bizday::BusinessDayConvention::Following,
            country,
        };
        let mut terms = phx_ledger::opening::plain_terms(
            ccy,
            vec![Leg::Principal { amount: Money::new(law.face, ccy), repayment: Repayment::Bullet }],
            Schedule { dates, count: Missing::Present(1) },
        );
        if let Missing::Present(o) = order {
            terms.payment_order = Order(o.debt_service);
        }
        let id = self.books.ledger.terms.intern(terms);
        let k = self.books.ledger.lines.kind_index(kind.line);
        let maturity = dates.nth(&self.calendar, 1);
        let line = self.books.ledger.lines.open(k, id, Missing::Present((maturity, 1)));
        self.state.book.bills.insert(line, maturity);
        line
    }

    /// 7c, after the dues: the bills the day redeemed counted, and those fully paid forgotten.
    #[clause("REG.11", "TRS.6")]
    pub(crate) fn bills_redeemed(&mut self) {
        let Some(kind) = self.state.bills else { return };
        let k = self.books.ledger.lines.kind_index(kind.line);
        let paid: Vec<(LineId, i64)> = self
            .books
            .ledger
            .day_book()
            .dues
            .iter()
            .filter(|d| {
                d.outcome == phx_ledger::effects::DueOutcome::Settled && self.books.ledger.lines.kind_of(d.line) == k
            })
            .map(|d| (d.line, d.principal.amt()))
            .collect();
        for (line, principal) in paid {
            let country = self.books.ledger.terms.get(self.books.ledger.lines.terms(line)).ccy;
            let Some(c) = (0..self.state.countries.len())
                .filter_map(|i| u8::try_from(i).ok())
                .find(|i| phx_ledger::opening::currency(CountryId::new(*i)) == country)
            else {
                continue;
            };
            *self.state.book.redeemed.entry(c).or_insert(0) += principal;
        }
    }
}
