//! CB, the central bank: each country's central bank and its treasury, the reserves banks hold at it, the treasury's
//! account and the claim on the treasury that backs them; its corridor of standing facilities, which the kernel runs
//! at the fund stage, and its net income remitted to the treasury. Its committee and operations arrive with their own
//! step; the treasury is brought forward for its account, and its system's step takes it over.

pub mod central;
mod consts;
mod opening;

use if_credit::central::CentralKind;
use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_kind, declare_prim, declare_stream};
use phx_ledger::instruction::{Effect, ReasonDecl};
use phx_num::{Fixed, Missing};

pub use opening::{Balances, Declared, Lines, Parties, site};

declare_kind! { pub CENTRAL_BANK = "central_bank" { legal_form: "central bank", table: Individuals, clause: "CB.1" } }
declare_kind! { pub TREASURY = "treasury" { legal_form: "treasury", table: Individuals, clause: "CB.1" } }

declare_stream! { pub OpeningStream = "CB.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_prim! {
    /// The central bank's currency in circulation, per cent of GDP, which the households that bank nowhere hold.
    pub CURRENCY = "CB.currency" { kind: Endowment, value: Fixed { exp: 2 }, clause: "MON.4", scope: PerCountry }
}

declare_prim! {
    /// How far below the policy rate the deposit facility pays.
    pub DEPOSIT_SPREAD = "CB.deposit_spread" {
        kind: Shape, value: Fixed { exp: 4 }, clause: "CB.7", scope: Shared, shape: placeholder("CB")
    }
}

declare_prim! {
    /// How far above the policy rate the lending facility charges.
    pub LENDING_SPREAD = "CB.lending_spread" {
        kind: Shape, value: Fixed { exp: 4 }, clause: "CB.7", scope: Shared, shape: placeholder("CB")
    }
}

declare_prim! {
    /// The share of an eligible loan's balance the lending facility does not lend against.
    pub LOAN_HAIRCUT = "CB.loan_haircut" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "CB.6", scope: Shared, shape: placeholder("CB")
    }
}

/// A bank's position at a facility moved: reserves against the facility's balance.
pub const MOVED: ReasonDecl =
    ReasonDecl { name: "CB facility", order: 2, paid: Effect::Equity, received: Effect::Equity, held: Missing::Absent };
/// Interest a facility pays or charges: the payer's expense, the payee's income.
pub const INTEREST: ReasonDecl = ReasonDecl {
    name: "CB interest",
    order: 2,
    paid: Effect::Expense,
    received: Effect::Revenue,
    held: Missing::Absent,
};
/// The central bank's net income paid to the treasury.
pub const REMITTED: ReasonDecl = ReasonDecl {
    name: "CB remitted",
    order: 2,
    paid: Effect::Equity,
    received: Effect::Revenue,
    held: Missing::Absent,
};

/// The central bank's facilities, whose fund stage the kernel runs.
pub const CENTRAL: CentralKind = CentralKind {
    central_bank: CENTRAL_BANK.name,
    treasury: TREASURY.name,
    bank: "bank",
    reserves: "reserves",
    account: "treasury account",
    deposit_facility: opening::DEPOSIT_FACILITY.name,
    lending_facility: opening::LENDING_FACILITY.name,
    moved: MOVED.name,
    interest: INTEREST.name,
    remitted: REMITTED.name,
    corridor: central::corridor,
};

/// The central bank.
#[derive(Debug)]
pub struct Cb;

impl System for Cb {
    const CODE: &'static str = "CB";

    fn declare(d: &mut Declarations) {
        d.kind(CENTRAL_BANK);
        d.kind(TREASURY);
        d.stream(OpeningStream::DECL);
        d.contribution(Box::new(Declared));
        d.contribution(Box::new(Parties));
        d.contribution(Box::new(Lines));
        d.contribution(Box::new(Balances));
        for p in [&DEPOSIT_SPREAD, &LENDING_SPREAD] {
            let _: phx_core::Prim<Fixed<4>> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<2>> = d.prim(&LOAN_HAIRCUT);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&CURRENCY);
        d.market(Box::new(CENTRAL));
    }

    fn handlers(_: &mut HandlerTable) {}
}
