//! The households' banking at the opening: a household any of whose adults holds an account banks with one bank,
//! chosen online in proportion to the banks' shares and held in its key; it keeps a deposit there, its share of the
//! households' deposits by its wealth; and a household any of whose adults has borrowed owes its bank a loan, its
//! share of the households' debt by its income. A household that banks nowhere holds banknotes instead, its persons'
//! share of the currency in circulation a head.

use phx_core::register::values::Table1;
use phx_core::{OpeningCountry, Prim, Register, declare_stream};
use phx_ledger::online::Online;
use phx_ledger::opening::whole;
use phx_macros::clause;
use phx_num::{Count, violation};
use phx_rand::{Draws, open_unit};

use crate::consts::{SHARE_PARTS, WEALTH_PARTS};

declare_stream! { pub HouseholdsStream = "BNK.opening_households" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }

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
/// there, its deposit's weight by its wealth, and its loan's place among the years' choices where it has borrowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Banked {
    Unbanked { persons: u64 },
    At { bank: usize, deposit: u64, loan: Option<usize> },
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
    pub fn draw(&mut self, household: &phx_core::Household, wealth: f64, d: &mut Draws) -> Banked {
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
            at
        });
        Banked::At { bank, deposit: weight(wealth), loan }
    }
}

fn share(table: &Table1, at: i64) -> f64 {
    let Ok(v) = table.at(at) else {
        violation!(clause = "GEN.2", "a share of adults the table does not hold", at = at)
    };
    phx_rand::float::from_i64(v) / SHARE_PARTS
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
