//! BNK, bank lending: here its opening alone — each country's banks, sized by a Zipf law fitted to the published
//! concentration of their assets; each bank's current accounts, where the firms it lends to keep their deposits; and
//! the firms' term loans. Its decisions arrive with its own step.

mod consts;
mod opening;
pub mod zipf;

use phx_core::{Declarations, HandlerTable, StreamDef, System, declare_kind, declare_prim, declare_stream};

pub use opening::{Balances, Contracts, Declared, Parties};

declare_kind! { pub BANK = "bank" { legal_form: "bank", table: Individuals, clause: "BNK.1" } }

declare_stream! { pub OpeningStream = "BNK.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_prim! {
    /// The shortest term, in years, of a firm's term loan at the opening.
    pub LOAN_YEARS_MIN = "BNK.loan_years_min" {
        kind: Shape, value: Count, clause: "GEN.2", scope: Shared, shape: placeholder("BNK")
    }
}

declare_prim! {
    /// The longest term, in years, of a firm's term loan at the opening.
    pub LOAN_YEARS_MAX = "BNK.loan_years_max" {
        kind: Shape, value: Count, clause: "GEN.2", scope: Shared, shape: placeholder("BNK")
    }
}

/// Bank lending.
#[derive(Debug)]
pub struct Bnk;

impl System for Bnk {
    const CODE: &'static str = "BNK";

    fn declare(d: &mut Declarations) {
        d.kind(BANK);
        d.stream(OpeningStream::DECL);
        let years = (d.prim(&LOAN_YEARS_MIN), d.prim(&LOAN_YEARS_MAX));
        d.contribution(Box::new(Declared));
        d.contribution(Box::new(Parties));
        d.contribution(Box::new(Contracts { years }));
        d.contribution(Box::new(Balances));
    }

    fn handlers(_: &mut HandlerTable) {}
}
