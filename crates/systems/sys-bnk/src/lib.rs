//! BNK, bank lending: here its opening alone — each country's banks, sized by a Zipf law fitted to the published
//! concentration of their assets; each bank's current accounts, where the firms it lends to keep their deposits; and
//! the firms' term loans. Its decisions arrive with its own step.

mod consts;
pub mod households;
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

declare_prim! {
    /// Adults with an account, saving at a bank and borrowing from one, which households' banking arrangements read.
    pub ACCOUNTS = "BNK.accounts_and_borrowing" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
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
        let accounts: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&ACCOUNTS);
        d.stream(households::HouseholdsStream::DECL);
        d.pop_kind(if_pop::HOUSEHOLD).key_attr(households::BANK_ATTR);
        let draw: Box<dyn phx_ledger::attachments::AttachmentDraw> = Box::new(households::HouseholdLines { accounts });
        d.attachment(Box::new(draw));
        d.contribution(Box::new(Declared));
        d.contribution(Box::new(Parties));
        d.contribution(Box::new(Contracts { years }));
        d.contribution(Box::new(Balances));
    }

    fn handlers(_: &mut HandlerTable) {}
}
