//! CB, the central bank: each country's central bank and its treasury, sited together; the currency the households
//! that bank nowhere hold; and its corridor of standing facilities, which the core runs at the fund stage. Its
//! committee and operations arrive with their own step; the treasury is brought forward for its account, and its
//! system's step takes it over.

pub mod central;
mod consts;
mod opening;

use phx_core::{Declarations, StreamDef, System, declare_kind, declare_prim, declare_stream};
use phx_num::Fixed;

pub use opening::site;

declare_kind! { pub CENTRAL_BANK = "central_bank" { legal_form: "central bank", place: Site { word: 0 }, store: "institutions", clause: "CB.1" } }
declare_kind! { pub TREASURY = "treasury" { legal_form: "treasury", place: Site { word: 0 }, store: "institutions", clause: "CB.1" } }

declare_stream! { pub OpeningStream = "CB.opening" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }

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

/// The central bank.
#[derive(Debug)]
pub struct Cb;

impl System for Cb {
    const CODE: &'static str = "CB";

    fn declare(d: &mut Declarations) {
        d.kind(CENTRAL_BANK);
        d.kind(TREASURY);
        d.stream(OpeningStream::DECL);
        for p in [&DEPOSIT_SPREAD, &LENDING_SPREAD] {
            let _: phx_core::Prim<Fixed<4>> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<2>> = d.prim(&LOAN_HAIRCUT);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&CURRENCY);
    }
}
