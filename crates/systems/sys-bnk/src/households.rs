//! The households' banking at the opening: a household any of whose adults holds an account banks with one bank,
//! chosen online in proportion to the banks' shares and held in its key; it keeps a deposit there, its share of the
//! households' deposits by its wealth; and a household any of whose adults has borrowed owes its bank a loan, its
//! share of the households' debt by its income.

use phx_core::calendar::daycount::DayCount;
use phx_core::register::values::Table1;
use phx_core::{KeyAttrDecl, OpeningCountry, OpeningCtx, Prim, Register, StreamDef, declare_stream};
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
use phx_num::{Missing, violation};
use phx_rand::{Draws, Subject, open_unit};

use crate::BANK;
use crate::consts::{MOST_BANKS, PERCENT, SHARE_PARTS, WEALTH_PARTS};
use crate::opening::{drawn, rate};
use phx_ledger::opening::{monthly, plain_terms as terms};

declare_stream! { pub HouseholdsStream = "BNK.opening_households" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// The bank a household banks with, counted from one; nought, none.
pub const BANK_ATTR: KeyAttrDecl = KeyAttrDecl { name: "BNK.bank", values: MOST_BANKS + 1, clause: "REP.19" };

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
    },
    liability: phx_ledger::line::SideDecl {
        holder_kinds: &[if_pop::HOUSEHOLD, phx_core::ESTATE_KIND.name],
        words: phx_ledger::rows::BALANCE,
        holder_list: false,
        holder_roles: &[],
        exclusive: true,
    },
    dated: true,
    transfer_requesters: &["BNK"],
};

/// The pools the households' balances are shares of.
const DEPOSITS: u32 = 0;
const DEBT: u32 = 1;

/// The published shares of adults with an account and having borrowed, at their places in the declared table.
const HOLDS_ACCOUNT: i64 = 0;
const BORROWED: i64 = 2;

/// The banks' draw of the households' lines.
#[derive(Debug)]
pub struct HouseholdLines {
    pub accounts: Prim<Table1>,
}

/// One country's banks as the households draw them.
struct Country {
    banks: Vec<PartyId>,
    online: Online,
    account: f64,
    borrowed: f64,
    deposit: (u16, TermsId, Missing<(Day, u32)>),
    loan: (u16, TermsId, Missing<(Day, u32)>),
    deposits: i64,
    debt: i64,
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
        let table = self.accounts.get(register, c.id);
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
        let loan_terms = books.ledger.terms.intern(terms(
            ccy,
            vec![Leg::RateOnNotional {
                reference: Reference::Fixed(rate(derived(c, "GEN.lending_rate"))),
                day_count: DayCount::Act365F,
            }],
            Schedule { dates: monthly(date, c.id), count: Missing::Absent },
        ));
        let lines = &books.ledger.lines;
        let deposits = whole(derived(c, "GEN.bank_deposits") / PERCENT * c.gdp) - firms;
        if deposits < 0 {
            violation!(clause = "GEN.4", "firms holding more deposits than the country's banks", country = c.id.get());
        }
        Box::new(Country {
            online: Online::new(banks.iter().map(|(_, w)| *w).collect()),
            banks: banks.into_iter().map(|(b, _)| b).collect(),
            account: share(table, HOLDS_ACCOUNT),
            borrowed: share(table, BORROWED),
            deposit: (lines.kind_index(ACCOUNT), deposit_terms, first),
            loan: (lines.kind_index(LOAN.name), loan_terms, first),
            deposits,
            debt: whole(derived(c, "GEN.household_debt") / PERCENT * c.gdp),
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
        keys: &mut Vec<(&'static str, u32)>,
    ) {
        let mut d = ctx.draws(&HouseholdsStream::DECL, subject);
        let adult_roles = [if_pop::HEAD.name, if_pop::PARTNER.name, if_pop::ADULT.name];
        let adults = h.household.persons.iter().filter(|p| adult_roles.contains(&p.role)).count();
        let account = any_of(adults, self.account, &mut d);
        let borrowed = any_of(adults, self.borrowed, &mut d);
        if !account {
            return;
        }
        let at = self.online.next(&mut d);
        let Some(bank) = self.banks.get(at).copied() else {
            violation!(clause = "GEN.4", "a bank chosen beyond the country's banks");
        };
        let Ok(place) = u32::try_from(at + 1) else { violation!(clause = "REP.19", "a bank beyond the key's reach") };
        keys.push((BANK_ATTR.name, place));
        let (kind, terms, first) = self.deposit;
        rows.push(DrawnRow {
            line: LineSpec { kind, terms, counterparty: Missing::Present(bank), first },
            side: Side::Asset,
            holder: Holder::Household,
            balance: Balance::Share { pool: DEPOSITS, weight: weight(h.wealth) },
        });
        if borrowed {
            let (kind, terms, first) = self.loan;
            rows.push(DrawnRow {
                line: LineSpec { kind, terms, counterparty: Missing::Present(bank), first },
                side: Side::Liability,
                holder: Holder::Household,
                balance: Balance::Share { pool: DEBT, weight: weight(h.income) },
            });
        }
    }

    fn pools(&self) -> Vec<(u32, i64)> {
        vec![(DEPOSITS, self.deposits), (DEBT, -self.debt)]
    }

    fn counterparties(&self, _: &Books, _: &LineSpec) -> Vec<(PartyId, u64)> {
        Vec::new()
    }
}

/// The stratum the report keys the households' banking under.
#[must_use]
pub fn stratum(c: &OpeningCountry) -> String {
    key("BNK.households", c.id)
}
