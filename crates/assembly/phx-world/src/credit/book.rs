//! The credit book: what the rounds carry from one day to the next and each bank's learning, saved with the world.

use std::collections::{BTreeMap, BTreeSet};

use if_credit::kind::CreditKind;
use if_credit::law::Law;
use phx_id::{Day, LineId, PartyId};

/// A borrower's application to a bank to refinance a loan falling due: the principal and months asked, and the day
/// it was sent.
#[derive(Clone, Debug, PartialEq, Eq, phx_macros::Saved)]
pub(crate) struct Application {
    pub borrower: PartyId,
    pub bank: PartyId,
    pub refinances: LineId,
    pub principal: i64,
    pub months: u32,
    pub sent: Day,
}

/// A bank's quote on an application: its yearly rate and the day it was made.
#[derive(Clone, Debug, PartialEq, phx_macros::Saved)]
pub(crate) struct Quote {
    pub borrower: PartyId,
    pub bank: PartyId,
    pub refinances: LineId,
    pub principal: i64,
    pub months: u32,
    pub rate: f64,
    pub made: Day,
}

/// A bank's learning and standards: the worst class it lends to; by class, the loan-years its book has held and the
/// defaults it has seen; the shares of defaulted balances it has lost and how many; each loan's class at its last
/// review, and each defaulted loan's balance when it defaulted; the month it last reviewed; and the applications it
/// has declined.
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub(crate) struct Bank {
    pub standard: u32,
    pub loan_years: Vec<f64>,
    pub defaults: Vec<u64>,
    pub lost: f64,
    pub recoveries: u64,
    pub classes: BTreeMap<LineId, u32>,
    pub defaulted: BTreeMap<LineId, i64>,
    pub declined: u64,
}

/// What the rounds carry across days, and the banks' learning.
#[derive(Clone, Debug, Default, PartialEq, phx_macros::Saved)]
pub(crate) struct CreditBook {
    pub applications: Vec<Application>,
    pub quotes: Vec<Quote>,
    pub lending: Vec<Quote>,
    pub banks: BTreeMap<PartyId, Bank>,
    pub tried: BTreeSet<LineId>,
    pub reviewed: u32,
}

/// A loan written on a day: its bank, its borrower, its principal, and the borrower's money before and after.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Written {
    pub bank: PartyId,
    pub borrower: PartyId,
    pub principal: i64,
    pub before: i64,
    pub after: i64,
    pub booked: i64,
}

/// The day's tally of the rounds, for the counters and the checks.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CreditDay {
    pub applications: u64,
    pub quotes: u64,
    pub declines: u64,
    pub declined_by_bank: Vec<(PartyId, u64)>,
    pub accepted: u64,
    pub written: Vec<Written>,
    pub reviews: u64,
}

/// Credit as the world keeps it: its kind and laws, its book, the day's tally, and each loan's maturity, read from
/// the world when it opens or loads.
#[derive(Debug, Default)]
pub(crate) struct Credit {
    pub kind: Option<CreditKind>,
    pub laws: Vec<Law>,
    pub book: CreditBook,
    pub day: CreditDay,
    pub maturities: BTreeMap<Day, Vec<LineId>>,
}
