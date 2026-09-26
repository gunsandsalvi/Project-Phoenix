//! The credit kind as `sys-bnk` declares it: the loan line kind lent on and the reason lending moves under, the
//! streams of the lenders a borrower asks and of its tastes over them, the compile of each country's law, and the
//! banks' and borrowers' rules.

use phx_core::{OpeningCountry, Register};
use phx_num::Missing;

use crate::decisions::{ChooseIn, DeclineIn, QuoteIn, StandardIn};
use crate::law::Law;

/// The credit kind: the firms' term loan line kind and the kinds of the parties that lend on it, the reason a loan is
/// disbursed under, the streams of the lenders asked and of the borrowers' tastes, each country's law, and the rules:
/// a borrower's class by its cover, a bank's quote and its decline, a borrower's choice, a bank's standard, and a
/// class's default frequency learned from the published one and a book.
#[derive(Clone, Copy, Debug)]
pub struct CreditKind {
    pub loan: &'static str,
    pub lender: &'static str,
    pub lent: &'static str,
    pub asked_stream: &'static str,
    pub taste_stream: &'static str,
    pub law: fn(&Register, &OpeningCountry) -> Result<Law, String>,
    pub class_of: fn(&Law, Missing<f64>) -> u32,
    pub quote: fn(&QuoteIn) -> f64,
    pub decline: fn(&DeclineIn) -> bool,
    pub choose: fn(&ChooseIn) -> Missing<u32>,
    pub standard: fn(&StandardIn) -> u32,
    pub learned: fn(f64, f64, f64, f64) -> f64,
}
