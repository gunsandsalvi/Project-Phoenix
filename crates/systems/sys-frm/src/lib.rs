//! FRM, firms: here its opening alone — each country's largest firms, the individuals within the promotion rank,
//! sized by Zipf's law over the country's employment; their plant; the debt and deposits they draw, which the banks'
//! contracts carry; and the small firms below the rank, carried in cells. Their decisions arrive with their own step.

mod consts;
mod opening;
pub mod small;

use phx_core::{
    Declarations, HandlerTable, KeyAttrDecl, ResolutionDecl, StreamDef, System, declare_kind, declare_prim,
    declare_stream,
};
use phx_num::{Count, Fixed};

pub use opening::{Declared, Parties, Plant};
pub use small::SmallFirms;

declare_kind! { pub FIRM = "firm" { legal_form: "company", table: Individuals, clause: "FRM.1" } }
declare_kind! { pub SMALL_FIRM = "small_firm" { legal_form: "company", table: Cells, clause: "FRM.23" } }

declare_stream! { pub OpeningStream = "FRM.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub SmallStream = "FRM.opening_small" { purpose: Opening, keyed: false, clause: "GEN.3" } }

/// The region a small firm is sited in.
pub const REGION: KeyAttrDecl = KeyAttrDecl { name: "FRM.region", values: if_pop::consts::REGIONS, clause: "REP.19" };
/// A small firm's employment size class.
pub const SIZE: KeyAttrDecl = KeyAttrDecl { name: "FRM.size_class", values: consts::SIZE_CLASSES, clause: "FRM.23" };
/// The bank a small firm banks with, which the banks declare on the kind; the opening draws it with the firm.
pub const BANK_ATTR: &str = "BNK.bank";

declare_prim! {
    /// The first employments of the size classes a small firm's key holds it in.
    pub SIZE_CLASSES = "FRM.size_classes" { kind: Resolution, value: Partition { exp: 0 }, clause: "FRM.23", scope: Shared }
}

declare_prim! {
    /// The most small-firm cells the representation keeps before tolerance control widens their steps.
    pub CELL_BUDGET = "FRM.cell_budget" { kind: Resolution, value: Count, clause: "REP.18", scope: Shared }
}

declare_prim! {
    /// Enterprises per person employed in the business economy, which sets the scale of the firm-size law.
    pub FIRMS_PER_EMPLOYED = "FRM.firms_per_employed" { kind: Endowment, value: Fixed { exp: 6 }, clause: "GEN.2", scope: PerCountry }
}

declare_prim! {
    /// The exponent of the firm-size law: the share of firms larger than a size falls as the size to its power.
    pub SIZE_EXPONENT = "FRM.size_exponent" { kind: Endowment, value: Fixed { exp: 6 }, clause: "GEN.2", scope: Shared }
}

declare_prim! {
    /// The firms, per million people, that the promotion rank makes individuals: the largest by employees.
    pub RANK_PER_MILLION = "FRM.rank_per_million" { kind: Resolution, value: Count, clause: "REP.2", scope: Shared }
}

declare_prim! {
    /// The share of the banks' deposits that non-financial firms hold.
    pub DEPOSIT_SHARE = "FRM.deposit_share" { kind: Endowment, value: Fixed { exp: 6 }, clause: "GEN.2", scope: Shared }
}

declare_prim! {
    /// The yearly rate at which plant wears out, in the steady state that sizes the opening's capital.
    pub DEPRECIATION = "FRM.depreciation" { kind: Technology, value: Fixed { exp: 6 }, clause: "GEN.5", scope: Shared }
}

/// Firms.
#[derive(Debug)]
pub struct Frm;

impl System for Frm {
    const CODE: &'static str = "FRM";

    fn declare(d: &mut Declarations) {
        d.kind(FIRM);
        d.stream(OpeningStream::DECL);
        let prims = opening::Prims {
            firms_per_employed: d.prim(&FIRMS_PER_EMPLOYED),
            size_exponent: d.prim(&SIZE_EXPONENT),
            rank: d.prim(&RANK_PER_MILLION),
            deposit_share: d.prim(&DEPOSIT_SHARE),
            depreciation: d.prim(&DEPRECIATION),
        };
        d.kind(SMALL_FIRM);
        d.stream(SmallStream::DECL);
        let _: CountPrim = d.prim(&CELL_BUDGET);
        let small = small::SmallPrims {
            firms_per_employed: prims.firms_per_employed,
            size_exponent: prims.size_exponent,
            deposit_share: prims.deposit_share,
            classes: d.prim(&SIZE_CLASSES),
        };
        d.pop_kind(SMALL_FIRM.name).key_attr(REGION).key_attr(SIZE).sited_by(REGION.name).resolution(ResolutionDecl {
            cell_budget: "FRM.cell_budget",
            ranks: None,
            widen_order: &[],
            clause: "REP.18",
        });
        d.contribution(Box::new(Declared));
        d.contribution(Box::new(Parties { prims }));
        d.contribution(Box::new(SmallFirms { prims: small }));
        d.contribution(Box::new(Plant));
    }

    fn handlers(_: &mut HandlerTable) {}
}

pub type FixedPrim = phx_core::Prim<Fixed<6>>;
pub type CountPrim = phx_core::Prim<Count>;
