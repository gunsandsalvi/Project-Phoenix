//! The households' banking at the opening: a household any of whose adults holds an account banks with one bank,
//! chosen online in proportion to the banks' shares and held in its key; it keeps a deposit there, its share of the
//! households' deposits by its wealth; and a household any of whose adults has borrowed owes its bank a loan, its
//! share of the households' debt by its income. A household that banks nowhere holds banknotes instead, its persons'
//! share of the currency in circulation a head.

use phx_core::calendar::daycount::DayCount;
use phx_core::register::values::Table1;
use phx_core::{AttrDecl, OpeningCountry, OpeningCtx, Prim, Register, StreamDef, declare_stream};
use phx_id::{Day, PartyId};
use phx_ledger::algebra::{Leg, Reference, Schedule, Side};
use phx_ledger::attachments::{
    AttachmentDraw, Balance, CountryAttachments, Drawing, DrawnRow, Holder, LineSpec, Online,
};
use phx_ledger::books::Books;
use phx_ledger::line::LineKindDecl;
use phx_ledger::money::MoneyHolders;
use phx_ledger::opening::{currency, derived, key, whole};
use phx_ledger::terms::TermsId;
use phx_macros::clause;
use phx_num::{Count, Missing, violation};
use phx_rand::{Draws, Subject, open_unit};

use crate::BANK;
use crate::consts::{MONTHS_PER_YEAR, MOST_BANKS, PERCENT, SHARE_PARTS, WEALTH_PARTS};
use crate::opening::{drawn, rate};
use phx_ledger::opening::{monthly, plain_terms as terms};

declare_stream! { pub HouseholdsStream = "BNK.opening_households" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// The bank a household banks with, counted from one; nought, none.
pub const BANK_ATTR: AttrDecl = AttrDecl { name: "BNK.bank", values: MOST_BANKS + 1, clause: "REP.41" };

/// Households' current accounts: a bank's liability to its many depositors, and to the estates they leave.
pub const RETAIL: MoneyHolders = MoneyHolders {
    central_banks: &["central_bank"],
    banks: &[BANK.name],
    treasuries: &["treasury"],
    depositors: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
    requesters: &["BNK"],
};

/// The households' current accounts.
pub const ACCOUNT: &str = "household current account";

/// A household's loan from its bank, owed by the household, many to a line.
pub const LOAN: LineKindDecl = LineKindDecl {
    name: "household loan",
    asset: phx_ledger::line::SideDecl {
        holder_kinds: &[BANK.name],
        words: phx_ledger::rows::BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: true,
    },
    liability: phx_ledger::line::SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: phx_ledger::rows::BALANCE,
        holder_list: false,
        holder_roles: &[],
        exclusive: true,
        many: false,
    },
    dated: true,
    transfer_requesters: &["BNK"],
};

/// The pools the households' balances are shares of.
const DEPOSITS: u32 = 0;
const DEBT: u32 = 1;
const CASH: u32 = 2;

/// The party the central bank of a country is drawn as, whose notes the households that bank nowhere hold.
const CENTRAL_BANK: &str = "CB.central_bank";

/// The published shares of adults with an account and having borrowed, at their places in the declared table.
const HOLDS_ACCOUNT: i64 = 0;
const BORROWED: i64 = 2;

/// The banks' draw of the households' lines.
#[derive(Debug)]
pub struct HouseholdLines {
    pub accounts: Prim<Table1>,
    /// The fewest and most years a household's loan has left to run.
    pub loan_years: (Prim<Count>, Prim<Count>),
}

/// One country's banking as its households draw it: the banks chosen online by their shares, and the shares of adults
/// with an account and having borrowed.
#[derive(Clone, Debug)]
pub struct Banking {
    online: Online,
    account: f64,
    borrowed: f64,
    /// The choices of years a loan has left.
    years: usize,
}

/// A household's banking as drawn: none, its persons holding banknotes; or its bank among the country's, by its place
/// there, its deposit's weight by its wealth, and its loan's place among the years' choices and weight by its income
/// where it has borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Banked {
    Unbanked { persons: u64 },
    At { bank: usize, deposit: u64, loan: Option<(usize, u64)> },
}

impl HouseholdLines {
    /// The banks' draw with its primitives found in the register.
    ///
    /// # Errors
    /// A primitive the draw reads that the register does not hold as declared.
    pub fn of(register: &Register) -> Result<HouseholdLines, String> {
        Ok(HouseholdLines {
            accounts: register.handle(&crate::ACCOUNTS)?,
            loan_years: (
                register.handle(&crate::HOUSEHOLD_LOAN_YEARS_MIN)?,
                register.handle(&crate::HOUSEHOLD_LOAN_YEARS_MAX)?,
            ),
        })
    }

    /// The fewest and most years a household's loan has left.
    #[must_use]
    pub fn years(&self, register: &Register) -> (u64, u64) {
        (self.loan_years.0.shared(register).get(), self.loan_years.1.shared(register).get())
    }

    /// A country's banking as its households draw it, over its banks' weights.
    #[must_use]
    pub fn banking(&self, register: &Register, c: &OpeningCountry, banks: Vec<u64>) -> Banking {
        let table = self.accounts.get(register, c.id);
        let (least, most) = self.years(register);
        let Some(span) = most.checked_sub(least) else {
            violation!(clause = "GEN.2", "a household loan's fewest years past its most");
        };
        Banking {
            online: Online::new(banks),
            account: share(table, HOLDS_ACCOUNT),
            borrowed: share(table, BORROWED),
            years: usize::try_from(span + 1).unwrap_or(usize::MAX),
        }
    }
}

impl Banking {
    /// A household's banking: whether any of its adults holds an account, and any has borrowed, each adult drawn
    /// once; a household with an account banks at a bank chosen online by the banks' shares.
    #[clause("GEN.2", "REP.23", "BNK.1")]
    pub fn draw(&mut self, household: &phx_core::Household, (wealth, income): (f64, f64), d: &mut Draws) -> Banked {
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        let adults = household.persons.iter().filter(|p| adult_roles.contains(&p.role)).count();
        let account = any_of(adults, self.account, d);
        let borrowed = any_of(adults, self.borrowed, d);
        if !account {
            return Banked::Unbanked { persons: phx_rand::float::len_u64(household.persons.len()) };
        }
        let bank = self.online.next(d);
        let loan = borrowed.then(|| {
            // The years a loan has left, drawn alike over the range.
            let span = phx_rand::float::from_u64(phx_rand::float::len_u64(self.years));
            let Some(at) = phx_rand::float::floor_to_u64(open_unit(d) * span)
                .and_then(|i| usize::try_from(i).ok())
                .filter(|i| *i < self.years)
            else {
                violation!(clause = "GEN.2", "a household loan's term beyond those drawn");
            };
            (at, weight(income))
        });
        Banked::At { bank, deposit: weight(wealth), loan }
    }
}

/// One country's banks as the books' households draw them.
struct Country {
    banks: Vec<PartyId>,
    banking: Banking,
    deposit: (u16, TermsId, Missing<(Day, u32)>),
    /// The loans' kind, their terms by the years they have left, the fewest first, and their first date.
    loan: (u16, Vec<TermsId>, Missing<(Day, u32)>),
    deposits: i64,
    debt: i64,
    /// The banknotes' line, of the central bank's notes; the currency in circulation a head; and the persons of the
    /// households drawn to hold them.
    cash: (u16, TermsId, PartyId),
    per_head: f64,
    unbanked: u64,
}

fn share(table: &Table1, at: i64) -> f64 {
    let Ok(v) = table.at(at) else {
        violation!(clause = "GEN.2", "a share of adults the table does not hold", at = at)
    };
    phx_rand::float::from_i64(v) / SHARE_PARTS
}

impl AttachmentDraw for HouseholdLines {
    #[clause("GEN.2", "GEN.4", "BNK.1")]
    fn country(
        &self,
        books: &mut Books,
        register: &Register,
        (calendar, today): (&phx_core::Calendar, Day),
        c: &OpeningCountry,
    ) -> Box<dyn CountryAttachments> {
        let banks = drawn(books, crate::opening::BANKS, c.id);
        let firms: i64 = drawn(books, crate::opening::DEPOSITS, c.id)
            .iter()
            .chain(&drawn(books, crate::opening::SMALL_DEPOSITS, c.id))
            .map(|(_, d)| {
                let Ok(d) = i64::try_from(*d) else {
                    violation!(clause = "MON.16", "a firm's deposit beyond whole smallest units");
                };
                d
            })
            .sum();
        let ccy = currency(c.id);
        let date = calendar.date(today);
        let first = Missing::Present((monthly(date, c.id).nth(calendar, 1), 1));
        let deposit_terms = books.ledger.terms.intern(terms(
            ccy,
            vec![Leg::RateOnNotional {
                reference: Reference::Fixed(rate(derived(c, "GEN.deposit_rate"))),
                day_count: DayCount::Act365F,
            }],
            Schedule { dates: monthly(date, c.id), count: Missing::Absent },
        ));
        let (least, most) = (self.loan_years.0.shared(register).get(), self.loan_years.1.shared(register).get());
        // A loan pays interest on what it owes and repays it in equal parts over the months it has left.
        let loan_terms: Vec<TermsId> = (least..=most)
            .map(|years| {
                let Some(months) = u32::try_from(years).ok().and_then(|y| y.checked_mul(MONTHS_PER_YEAR)) else {
                    violation!(clause = "GEN.2", "a household loan's term beyond a count of dates", years = years);
                };
                books.ledger.terms.intern(terms(
                    ccy,
                    vec![
                        Leg::RateOnNotional {
                            reference: Reference::Fixed(rate(derived(c, "GEN.lending_rate"))),
                            day_count: DayCount::Act365F,
                        },
                        Leg::Amortising,
                    ],
                    Schedule { dates: monthly(date, c.id), count: Missing::Present(months) },
                ))
            })
            .collect();
        let lines = &books.ledger.lines;
        let deposits = whole(derived(c, "GEN.bank_deposits") / PERCENT * c.gdp) - firms;
        let Some(&(central_bank, _)) = drawn(books, CENTRAL_BANK, c.id).first() else {
            violation!(clause = "GEN.3", "households drawn before their country's central bank", country = c.id.get());
        };
        let Ok(currency_share) = register.fixed_in("CB.currency", c.id) else {
            violation!(clause = "MON.4", "a country's currency in circulation unread", country = c.id.get());
        };
        let cash_terms = books.ledger.terms.intern(terms(
            ccy,
            Vec::new(),
            Schedule { dates: monthly(date, c.id), count: Missing::Absent },
        ));
        if deposits < 0 {
            violation!(clause = "GEN.4", "firms holding more deposits than the country's banks", country = c.id.get());
        }
        Box::new(Country {
            banking: self.banking(register, c, banks.iter().map(|(_, w)| *w).collect()),
            banks: banks.into_iter().map(|(b, _)| b).collect(),
            deposit: (lines.kind_index(ACCOUNT), deposit_terms, first),
            loan: (lines.kind_index(LOAN.name), loan_terms, first),
            deposits,
            debt: whole(derived(c, "GEN.household_debt") / PERCENT * c.gdp),
            cash: (lines.kind_index(RETAIL.cash().name), cash_terms, central_bank),
            per_head: currency_share / PERCENT * c.gdp / phx_rand::float::from_u64(c.people),
            unbanked: 0,
        })
    }
}

/// Whether any of `adults` does what a share of adults does, each drawn once, so the draws a household takes do not
/// depend on the answer.
fn any_of(adults: usize, p: f64, d: &mut Draws) -> bool {
    (0..adults).filter(|_| open_unit(d) < p).count() > 0
}

fn weight(x: f64) -> u64 {
    let Ok(w) = u64::try_from(whole(x * WEALTH_PARTS)) else {
        violation!(clause = "GEN.2", "a household's weight below nothing");
    };
    w
}

impl CountryAttachments for Country {
    #[clause("GEN.2", "REP.23", "BNK.1")]
    fn draw(
        &mut self,
        _: &mut Books,
        h: Drawing<'_>,
        (ctx, subject): (&OpeningCtx<'_>, Subject),
        rows: &mut Vec<DrawnRow>,
        keys: &mut phx_ledger::attachments::Keys,
    ) {
        let mut d = ctx.draws(&HouseholdsStream::DECL, subject);
        match self.banking.draw(h.household, (h.wealth, h.income), &mut d) {
            // A household that banks nowhere holds its persons' share of the currency in circulation, a head's each.
            Banked::Unbanked { persons } => {
                self.unbanked += persons;
                let (kind, terms, central_bank) = self.cash;
                rows.push(DrawnRow {
                    line: LineSpec {
                        kind,
                        terms,
                        counterparty: Missing::Present(central_bank),
                        first: Missing::Absent,
                    },
                    side: Side::Asset,
                    holder: Holder::Household,
                    balance: Balance::Share { pool: CASH, weight: persons },
                });
            }
            Banked::At { bank: at, deposit, loan } => {
                let Some(bank) = self.banks.get(at).copied() else {
                    violation!(clause = "GEN.4", "a bank chosen beyond the country's banks");
                };
                let Ok(place) = u32::try_from(at + 1) else {
                    violation!(clause = "REP.41", "a bank beyond the attribute's values")
                };
                keys.household.push((BANK_ATTR.name, place));
                let (kind, terms, first) = self.deposit;
                rows.push(DrawnRow {
                    line: LineSpec { kind, terms, counterparty: Missing::Present(bank), first },
                    side: Side::Asset,
                    holder: Holder::Household,
                    balance: Balance::Share { pool: DEPOSITS, weight: deposit },
                });
                if let Some((years, weight)) = loan {
                    let (kind, ref by_years, first) = self.loan;
                    let Some(terms) = by_years.get(years).copied() else {
                        violation!(clause = "GEN.2", "a household loan's term beyond those drawn");
                    };
                    rows.push(DrawnRow {
                        line: LineSpec { kind, terms, counterparty: Missing::Present(bank), first },
                        side: Side::Liability,
                        holder: Holder::Household,
                        balance: Balance::Share { pool: DEBT, weight },
                    });
                }
            }
        }
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        let cash = whole(self.per_head * phx_rand::float::from_u64(self.unbanked));
        vec![(DEPOSITS, self.deposits), (DEBT, -self.debt), (CASH, cash)]
    }

    fn counterparties(&mut self, _: &Books, _: &LineSpec, _: &mut phx_rand::Draws) -> Vec<(PartyId, u64)> {
        Vec::new()
    }
}

/// The stratum the report keys the households' banking under.
#[must_use]
pub fn stratum(c: &OpeningCountry) -> String {
    key("BNK.households", c.id)
}
