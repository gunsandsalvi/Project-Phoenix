//! BNK, bank lending: each country's banks, sized by a Zipf law fitted to the published concentration of their
//! assets; each bank's current accounts, where the firms it lends to keep their deposits; the firms' term loans; and
//! the banks' quotes, declines and standards and the borrowers' shopping, whose rules are here and whose rounds the
//! core runs.

mod consts;
pub mod credit;
pub mod households;
mod opening;
pub mod points;
pub use opening::{bank_site, bank_weights, rate};
pub mod zipf;

use phx_core::{Declarations, StreamDef, System, declare_kind, declare_prim, declare_stream};
use phx_num::{Count, Fixed};

declare_kind! { pub BANK = "bank" { legal_form: "bank", place: Site, store: "banks", clause: "BNK.1" } }

declare_stream! { pub OpeningStream = "BNK.opening" { family: World, purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub AskedStream = "BNK.lenders_asked" { family: World, purpose: Meeting, keyed: false, clause: "BNK.6" } }
declare_stream! { pub TasteStream = "BNK.lender_taste" { family: World, purpose: Taste, keyed: false, clause: "REP.22" } }

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
    /// The fewest years a household's loan at the opening has left to run.
    pub HOUSEHOLD_LOAN_YEARS_MIN = "BNK.household_loan_years_min" {
        kind: Shape, value: Count, clause: "GEN.2", scope: Shared, shape: placeholder("HSG")
    }
}

declare_prim! {
    /// The most years a household's loan at the opening has left to run.
    pub HOUSEHOLD_LOAN_YEARS_MAX = "BNK.household_loan_years_max" {
        kind: Shape, value: Count, clause: "GEN.2", scope: Shared, shape: placeholder("HSG")
    }
}

declare_prim! {
    /// Adults with an account, saving at a bank and borrowing from one, which households' banking arrangements read.
    pub ACCOUNTS = "BNK.accounts_and_borrowing" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

declare_prim! {
    /// The least interest cover of each class of borrower above the worst.
    pub COVER_BOUNDS = "BNK.cover_bounds" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 2 }, clause: "BNK.20", scope: Shared
    }
}

declare_prim! {
    /// Each class's published yearly default frequency, the worst first.
    pub DEFAULT_RATES = "BNK.default_rates" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 4 }, clause: "BNK.20", scope: Shared
    }
}

declare_prim! {
    /// The published share of a defaulted unsecured loan's balance lost.
    pub LOSS_GIVEN_DEFAULT = "BNK.loss_given_default" {
        kind: Endowment, value: Fixed { exp: 4 }, clause: "BNK.20", scope: Shared
    }
}

declare_prim! {
    /// The capital a loan of full risk weight consumes, as a share of it.
    pub CAPITAL_REQUIREMENT = "BNK.capital_requirement" {
        kind: Policy, decided_by: "banking supervisor", value: Fixed { exp: 4 }, clause: "BNK.4", scope: Shared
    }
}

declare_prim! {
    /// A firm's term loan's risk weight, until banks hold their own capital models.
    pub RISK_WEIGHT = "BNK.risk_weight" {
        kind: Shape, value: Fixed { exp: 2 }, clause: "BNK.4", scope: Shared, shape: placeholder("BCP")
    }
}

declare_prim! {
    /// The return a bank's management requires on the capital a loan consumes.
    pub REQUIRED_RETURN = "BNK.required_return" {
        kind: Preference, value: Fixed { exp: 3 }, clause: "BNK.16", scope: Shared
    }
}

declare_prim! {
    /// The cost of making a loan, as a share of output per person.
    pub LOAN_COST = "BNK.loan_cost_share" { kind: Technology, value: Fixed { exp: 4 }, clause: "BNK.16", scope: Shared }
}

declare_prim! {
    /// The step between the yearly rates lenders quote.
    pub RATE_STEP = "BNK.rate_step" {
        kind: Policy, decided_by: "the trade's convention", value: Fixed { exp: 5 }, clause: "REP.34", scope: Shared
    }
}

declare_prim! {
    /// The shares of borrowers asking one, two and three lenders.
    pub LENDERS_ASKED = "BNK.lenders_asked" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 2 }, clause: "BNK.6", scope: Shared
    }
}

declare_prim! {
    /// The loan-years of its own book a bank counts a published default frequency as.
    pub PRIOR_LOAN_YEARS = "BNK.prior_loan_years" { kind: Preference, value: Count, clause: "BNK.20", scope: Shared }
}

declare_prim! {
    /// The recoveries of its own a bank counts the published loss given default as.
    pub PRIOR_RECOVERIES = "BNK.prior_recoveries" { kind: Preference, value: Count, clause: "BNK.20", scope: Shared }
}

declare_prim! {
    /// The days before its maturity a borrower seeks to refinance a loan.
    pub LEAD_DAYS = "BNK.refinance_lead_days" { kind: Preference, value: Count, clause: "BNK.19", scope: Shared }
}

/// Bank lending.
#[derive(Debug)]
pub struct Bnk;

impl System for Bnk {
    const CODE: &'static str = "BNK";

    fn declare(d: &mut Declarations) {
        d.kind(BANK);
        d.stream(OpeningStream::DECL);
        for p in [&LOAN_YEARS_MIN, &LOAN_YEARS_MAX, &HOUSEHOLD_LOAN_YEARS_MIN, &HOUSEHOLD_LOAN_YEARS_MAX] {
            let _: phx_core::Prim<Count> = d.prim(p);
        }
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&ACCOUNTS);
        d.stream(households::HouseholdsStream::DECL);
        d.stream(AskedStream::DECL);
        d.stream(TasteStream::DECL);
        for p in [&COVER_BOUNDS, &DEFAULT_RATES, &LENDERS_ASKED] {
            let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(p);
        }
        for p in [&PRIOR_LOAN_YEARS, &PRIOR_RECOVERIES, &LEAD_DAYS] {
            let _: phx_core::Prim<Count> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<4>> = d.prim(&LOSS_GIVEN_DEFAULT);
        let _: phx_core::Prim<Fixed<4>> = d.prim(&CAPITAL_REQUIREMENT);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&RISK_WEIGHT);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&REQUIRED_RETURN);
        let _: phx_core::Prim<Fixed<4>> = d.prim(&LOAN_COST);
        let _: phx_core::Prim<Fixed<5>> = d.prim(&RATE_STEP);
        d.decision(&points::DECLINE);
        d.decision(&points::QUOTE);
        d.decision(&points::STANDARD);
        d.decision(&points::CHOOSE);
        d.decision(&points::REQUEST);
    }
}
