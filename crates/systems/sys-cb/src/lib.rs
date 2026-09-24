//! CB, the central bank: here its opening alone — each country's central bank and its treasury, the reserves banks
//! hold at it, the treasury's account and the claim on the treasury that backs them. Its decisions, the corridor and
//! its operations arrive with its own step; the treasury is brought forward for its account, and its system's step
//! takes it over.

mod consts;
mod opening;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_kind, declare_stream};

pub use opening::{Balances, Declared, Lines, Parties};

declare_kind! { pub CENTRAL_BANK = "central_bank" { legal_form: "central bank", table: Individuals, clause: "CB.1" } }
declare_kind! { pub TREASURY = "treasury" { legal_form: "treasury", table: Individuals, clause: "CB.1" } }

declare_stream! { pub OpeningStream = "CB.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }

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
    }

    fn handlers(_: &mut HandlerTable) {}
}
