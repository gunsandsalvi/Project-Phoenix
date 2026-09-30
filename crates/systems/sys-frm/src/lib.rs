//! FRM, firms: at the opening each country's largest firms, the individuals within the promotion rank, sized by
//! Zipf's law over the country's employment, each in an industry drawn by its size; the debt and deposits
//! they draw, which the banks' contracts carry; and the small firms below the rank, held as agents. In the day, their
//! decisions on the agenda, their default under the insolvency law, and the family of their revenue.

mod consts;
pub mod decide;
pub mod industry;
pub mod points;
pub mod produce;
pub mod rules;

use phx_core::register::values::Table2;
use phx_core::{AttrDecl, Declarations, StreamDef, System, declare_kind, declare_prim, declare_stream};
use phx_num::{Count, Fixed};

declare_kind! { pub FIRM = "firm" { legal_form: "company", place: Region { word: 1 }, clause: "FRM.1" } }

declare_stream! { pub OpeningStream = "FRM.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub VisitStream = "FRM.visits" { purpose: Occasion, keyed: false, clause: "REP.21" } }
declare_stream! { pub StanceStream = "FRM.stance" { purpose: Occasion, keyed: false, clause: "VAL.7" } }

declare_prim! {
    /// The return a firm's management requires of what it holds and does, a year, drawn for each firm.
    pub REQUIRED_RETURN = "FRM.required_return" {
        kind: Preference, value: Distribution { exp: 6 }, clause: "FRM.14", scope: Shared
    }
}

/// The products' opening prices, a unit's in its currency's smallest units, which the goods' system declares.
pub const OPENING_PRICE: &str = "GDS.opening_price";
/// Each product's price at the opening over its world price, by country.
pub const PRICE_LEVEL: &str = "GDS.price_level";

/// What the firms' opening state reads: the days of sales their stocks cover, the management's handles, and the
/// hurdles their managements are drawn from.
#[derive(Clone, Copy, Debug)]
pub struct FilingPrims {
    pub decide: decide::DecidePrims,
    pub required_return: phx_core::Prim<phx_core::register::values::Distribution>,
}

impl FilingPrims {
    /// A product's lot, the units its price is posted for: ten to its unit's price places.
    #[must_use]
    pub fn lot(register: &phx_core::Register, product: u16) -> f64 {
        let entry = register.products("TEC.products").ok().and_then(|p| p.get(usize::from(product)).cloned());
        let exp = entry
            .and_then(|e| match register.units().named(&e.unit) {
                phx_num::Missing::Present(u) => register.units().decl(u).map(|d| d.price_exp),
                phx_num::Missing::Absent => None,
            })
            .unwrap_or_else(|| phx_num::violation!(clause = "GDS.1", "a product in an undeclared unit"));
        libm::pow(consts::DECADE, f64::from(exp))
    }
}

/// The region a small firm is sited in.
pub const REGION: AttrDecl = AttrDecl { name: "FRM.region", values: if_pop::consts::REGIONS, clause: "REP.41" };
/// The bank a small firm banks with, which the banks declare on the kind; the opening draws it with the firm.
pub const BANK_ATTR: &str = "BNK.bank";

declare_prim! {
    /// Enterprises per person employed, by product: how many firms the persons employed making it make.
    pub FIRMS_PER_EMPLOYED = "FRM.firms_per_employed" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

/// What the firms' handlers read of the register, compiled at assembly.
#[derive(Debug)]
pub struct Own {
    management: decide::Management,
    plant: produce::Plant,
}

impl Own {
    #[must_use]
    pub fn management(&self) -> &decide::Management {
        &self.management
    }

    #[must_use]
    pub fn plant(&self) -> &produce::Plant {
        &self.plant
    }
}

declare_prim! {
    /// Days a firm may leave a payment due unpaid before it is in default of payment and liquidated: the insolvency
    /// law's grace.
    pub INSOLVENCY_GRACE_DAYS = "FRM.insolvency_grace_days" {
        kind: Policy, decided_by: "parliament", value: Count, clause: "FRM.15", scope: PerCountry
    }
}

/// Firms.
#[derive(Debug)]
pub struct Frm;

impl System for Frm {
    const CODE: &'static str = "FRM";

    fn declare(d: &mut Declarations) {
        d.kind(FIRM);
        d.stream(OpeningStream::DECL);
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&FIRMS_PER_EMPLOYED);
        declare_decisions(d);
        d.decision(&points::DAY_ZERO_PRICE);
        d.decision(&points::PRODUCE);
        d.decision(&points::INPUTS);
        d.decision(&points::ATTEND);
        d.decision(&points::REVIEW_PRICE);
        d.decision(&points::REPRICE);
        d.decision(&points::STANCE);
        d.decision(&points::CLOSE);
        let _: phx_core::Prim<phx_core::register::values::Distribution> = d.prim(&REQUIRED_RETURN);
    }
}

pub type FixedPrim = phx_core::Prim<Fixed<6>>;
pub type CountPrim = phx_core::Prim<Count>;
pub type TablePrim = phx_core::Prim<Table2>;

/// The firms' management and plant, compiled for the core's rules.
fn declare_decisions(d: &mut Declarations) {
    let _ = d.prim::<Count>(&INSOLVENCY_GRACE_DAYS);
    d.stream(VisitStream::DECL);
    d.stream(StanceStream::DECL);
    let prims = decide::DecidePrims::declare(d);
    d.compile(Box::new(move |register, countries| {
        let adjustment = prims.adjustment_days.shared(register).get();
        Ok(Box::new(Own {
            management: decide::Management::compile(&prims, register)?,
            plant: produce::Plant::compile(register, countries, adjustment)?,
        }))
    }));
}
