//! FRM, firms: at the opening each country's largest firms, the individuals within the promotion rank, sized by
//! Zipf's law over the country's employment, each in an industry drawn by its size; the debt and deposits
//! they draw, which the banks' contracts carry; and the small firms below the rank, held as agents. In the day, their
//! decisions on the agenda, their default under the insolvency law, and the family of their revenue.

mod consts;
pub mod decide;
pub mod families;
pub mod filed;
pub mod industry;
mod law;
mod opening;
pub mod produce;
pub mod rules;
pub mod small;

use phx_core::handler::HandlerDecl;
use phx_core::register::values::Table2;
use phx_core::{
    AttrDecl, Cadence, Declarations, FacetDecl, FactDef, HandlerTable, InsolvencyDecl, RunsOn, StreamDef, System,
    VisitDecl, declare_kind, declare_prim, declare_stream,
};
use phx_num::{Count, Fixed};

pub use opening::Parties;
pub use small::SmallFirms;

declare_kind! { pub FIRM = "firm" { legal_form: "company", table: Individuals, clause: "FRM.1" } }
declare_kind! { pub SMALL_FIRM = "small_firm" { legal_form: "company", table: Agents, clause: "FRM.23" } }

declare_stream! { pub OpeningStream = "FRM.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }
declare_stream! { pub SmallStream = "FRM.opening_small" { purpose: Opening, keyed: false, clause: "GEN.3" } }
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

/// What the firms' opening state reads: the days of sales their stocks cover, the management's handles, and the
/// hurdles their managements are drawn from.
#[derive(Clone, Copy, Debug)]
pub struct FilingPrims {
    pub decide: decide::DecidePrims,
    pub required_return: phx_core::Prim<phx_core::register::values::Distribution>,
}

impl FilingPrims {
    /// The methods a firm may forecast by: each heuristic with each memory type.
    fn methods(register: &phx_core::Register) -> u64 {
        let types = register.count("VAL.memory_types").unwrap_or_else(|_| {
            phx_num::violation!(clause = "VAL.23", "the memory types unread");
        });
        phx_rand::float::len_u64(phx_val::heuristic::MENU.len()) * types
    }

    /// A product's lot, the units its price is posted for: ten to its unit's price places.
    fn lot(register: &phx_core::Register, product: u16) -> f64 {
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
/// A small firm's persons employed.
pub const SIZE: AttrDecl = AttrDecl { name: "FRM.size", values: consts::MOST_SMALL_EMPLOYED, clause: "FRM.23" };
/// The bank a small firm banks with, which the banks declare on the kind; the opening draws it with the firm.
pub const BANK_ATTR: &str = "BNK.bank";

declare_prim! {
    /// Enterprises per person employed in the business economy, which sets the scale of the firm-size law.
    pub FIRMS_PER_EMPLOYED = "FRM.firms_per_employed" { kind: Endowment, value: Fixed { exp: 6 }, clause: "GEN.2", scope: PerCountry }
}

declare_prim! {
    /// The share of each size class's firms (rows, by the class's smallest size in persons) in each industry
    /// (columns, in the products' order).
    pub INDUSTRY_BY_SIZE = "FRM.industry_by_size" {
        kind: Endowment, value: Table2 { row_exp: 0, column_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
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

/// Units a firm makes by its way: the cost of what the way uses up an expense as it is used, so what it makes carries
/// no cost of its own and its sale's revenue is the margin over the inputs expensed.
pub const MADE: phx_ledger::instruction::ReasonDecl = phx_ledger::instruction::ReasonDecl {
    name: "FRM made",
    order: 2,
    paid: phx_ledger::instruction::Effect::Expense,
    received: phx_ledger::instruction::Effect::Asset,
    held: phx_num::Missing::Present((phx_ledger::instruction::Effect::Expense, phx_ledger::instruction::Effect::Asset)),
};

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
        let prims = opening::Prims {
            firms_per_employed: d.prim(&FIRMS_PER_EMPLOYED),
            size_exponent: d.prim(&SIZE_EXPONENT),
            rank: d.prim(&RANK_PER_MILLION),
            deposit_share: d.prim(&DEPOSIT_SHARE),
            industries: d.prim(&INDUSTRY_BY_SIZE),
        };
        d.claim(<if_firm::known::Industry as FactDef>::ITEM.name);
        d.facet(FacetDecl { fact: <if_firm::known::Industry as FactDef>::ITEM.name, kind: FIRM.name });
        d.claim(<if_firm::known::Product as FactDef>::ITEM.name);
        d.facet(FacetDecl { fact: <if_firm::known::Product as FactDef>::ITEM.name, kind: FIRM.name });
        d.kind(SMALL_FIRM);
        d.stream(SmallStream::DECL);
        let small = small::SmallPrims {
            firms_per_employed: prims.firms_per_employed,
            size_exponent: prims.size_exponent,
            deposit_share: prims.deposit_share,
            industries: prims.industries,
        };
        d.pop_kind(SMALL_FIRM.name)
            .attr(REGION)
            .attr(SIZE)
            .attr(if_firm::known::INDUSTRY)
            .attr(if_firm::known::PRODUCT)
            .sited_by(REGION.name);
        d.contribution(Box::new(Parties { prims }));
        d.contribution(Box::new(SmallFirms { prims: small }));
        let decide = declare_decisions(d);
        let filing = FilingPrims { decide, required_return: d.prim(&REQUIRED_RETURN) };
        d.contribution(Box::new(filed::Declared));
        d.contribution(Box::new(filed::Products));
        d.contribution(Box::new(filed::Stocks { prims: filing }));
        d.contribution(Box::new(filed::Filed { prims: filing }));
        d.family(Box::new(families::Revenue));
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<decide::ReviewSmall>();
        h.add::<decide::ReviewLarge>();
        h.add::<decide::AttendSmall>();
        h.add::<decide::AttendLarge>();
    }
}

pub type FixedPrim = phx_core::Prim<Fixed<6>>;
pub type CountPrim = phx_core::Prim<Count>;
pub type TablePrim = phx_core::Prim<Table2>;

/// The firms' state, a large firm's as facts of its row and a small firm's as positions of its agent, and their
/// decisions on the agenda: the price's attention on each firm's production schedule, the price at its reviews.
fn declare_decisions(d: &mut Declarations) -> decide::DecidePrims {
    let _ = d.prim::<Count>(&INSOLVENCY_GRACE_DAYS);
    for kind in [FIRM.name, SMALL_FIRM.name] {
        d.insolvency(InsolvencyDecl { kind, grace_days: INSOLVENCY_GRACE_DAYS.id, clause: "FRM.15" });
    }
    d.stream(VisitStream::DECL);
    d.stream(StanceStream::DECL);
    for fact in if_firm::facts::FACTS {
        d.claim(fact);
        d.facet(FacetDecl { fact, kind: FIRM.name });
    }
    let mut small = d.pop_kind(SMALL_FIRM.name);
    for p in if_firm::facts::POSITIONS {
        small.position(p);
    }
    let prims = decide::DecidePrims::declare(d);
    d.compile(Box::new(move |register, countries| {
        let adjustment = prims.adjustment_days.shared(register).get();
        Ok(Box::new(Own {
            management: decide::Management::compile(&prims, register)?,
            plant: produce::Plant::compile(register, countries, adjustment)?,
        }))
    }));
    let schedule = Cadence::Schedule { days: decide::PRODUCTION_DAYS.id, runs_on: RunsOn::Business };
    let attention = Cadence::Attention { position: <if_firm::facts::PriceAttention as FactDef>::ITEM.name };
    // A review is woken by a surprise in the public series its stance reads.
    let surprise: &[phx_core::WakeKind] = &[phx_core::WakeKind::Surprise];
    for (handler, kind, cadence, wakes) in [
        (decide::AttendSmall::NAME, SMALL_FIRM.name, schedule, &[][..]),
        (decide::AttendLarge::NAME, FIRM.name, schedule, &[][..]),
        (decide::ReviewSmall::NAME, SMALL_FIRM.name, attention, surprise),
        (decide::ReviewLarge::NAME, FIRM.name, attention, surprise),
    ] {
        d.visit(VisitDecl { handler, kind, cadence, stream: VisitStream::DECL.name, wakes, clause: "REP.21" });
    }
    prims
}
