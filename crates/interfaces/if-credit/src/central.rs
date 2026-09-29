//! The central bank's standing facilities as `sys-cb` declares them: each country's corridor, the lines its positions
//! are held on and the reasons they move under; and a bank's request at them, whose rule is `sys-bnk`'s.

use phx_core::{OpeningCountry, Register};

/// A country's corridor: the yearly rates the deposit facility pays and the lending facility charges, and the share
/// of an eligible loan's balance the lending facility lends against.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct Corridor {
    pub deposit_rate: f64,
    pub lending_rate: f64,
    pub haircut: f64,
}

/// What a bank reads when it requests the facilities at the fund stage: its reserves, the reserves it aims to hold,
/// what it has placed and borrowed overnight, and what its eligible collateral lends against. The output is what it
/// places at the deposit facility and borrows from the lending facility overnight, each none or more.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RequestIn {
    pub reserves: i64,
    pub target: i64,
    pub collateral: i64,
}

/// A bank's overnight positions at the facilities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Request {
    pub place: i64,
    pub borrow: i64,
}

/// The central bank's facilities: the kinds of the central bank, the treasury and the banks, the reserves' and the
/// treasury account's line kinds, the two facilities' line kinds, the reasons positions, interest and remittance move
/// under, and each country's corridor.
#[derive(Clone, Copy, Debug)]
pub struct CentralKind {
    pub central_bank: &'static str,
    pub treasury: &'static str,
    pub bank: &'static str,
    pub reserves: &'static str,
    pub account: &'static str,
    pub deposit_facility: &'static str,
    pub lending_facility: &'static str,
    pub moved: &'static str,
    pub interest: &'static str,
    pub remitted: &'static str,
    pub corridor: fn(&Register, &OpeningCountry) -> Result<Corridor, String>,
}
