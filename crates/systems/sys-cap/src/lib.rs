//! CAP, plant: capital goods of six kinds, held by condition class, worn from class to class at each owner's review
//! and retired at the end of their service, their depreciation charged to income and to the unit; every firm's plant
//! at the opening; the capacity a way's plant gives; and the rules by which owners invest, maintain, repair, sell and
//! scrap.

mod consts;
pub mod families;
pub mod kinds;
mod opening;
pub mod review;
pub mod rules;

use phx_core::handler::HandlerDecl;
use phx_core::register::values::Table1;
use phx_core::{
    Cadence, Declarations, HandlerTable, RunsOn, StreamDef, System, VisitDecl, WearDecl, declare_prim, declare_stream,
};
use phx_num::Count;

pub use opening::{BOUGHT, COMPLETED, Plant, STRUCTURES_HELD};

declare_stream! { pub VisitStream = "CAP.visits" { purpose: Occasion, keyed: false, clause: "CAP.4" } }
declare_stream! { pub OpeningStream = "CAP.opening" { purpose: Opening, keyed: false, clause: "GEN.3" } }

declare_prim! {
    /// Each kind's geometric rate of depreciation a year.
    pub DEPRECIATION = "CAP.depreciation" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 6 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Each kind's mean service life, in years.
    pub SERVICE_LIFE = "CAP.service_life" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 3 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Each kind's hyperbolic age-efficiency parameter.
    pub EFFICIENCY_SHAPE = "CAP.efficiency_shape" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 3 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// Days from order to service of each kind a source measures.
    pub LEAD_DAYS = "CAP.lead_days" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "CAP.13", scope: Shared
    }
}

declare_prim! {
    /// The product each kind is bought as, by its place among the products.
    pub BOUGHT_AS = "CAP.bought_as" {
        kind: Technology, value: Table1 { axis_exp: 0, exp: 0 }, clause: "CAP.5", scope: Shared
    }
}

declare_prim! {
    /// The classes of age each kind is held in.
    pub CONDITION_CLASSES = "CAP.condition_classes" { kind: Resolution, value: Count, clause: "REP.24", scope: Shared }
}

declare_prim! {
    /// Days between an owner's reviews of its plant.
    pub REVIEW_DAYS = "CAP.review_days" { kind: Preference, value: Count, clause: "CAP.13", scope: Shared }
}

declare_prim! {
    /// Each kind's net stock per unit of GDP at the opening.
    pub STOCK_PER_GDP = "CAP.stock_per_gdp" {
        kind: Endowment, value: Table1 { axis_exp: 0, exp: 6 }, clause: "GEN.2", scope: PerCountry
    }
}

/// The kinds of firm that hold plant.
pub const HOLDERS: [&str; 2] = ["firm", "small_firm"];

/// What the plant's handlers read, compiled once: the kinds, and each way's plant of each kind per unit of its output
/// a year, by the way's identity, each country's ways in the products' order after the country before's, as the
/// technology registers them.
#[derive(Debug)]
pub struct CapOwn {
    pub kinds: kinds::Kinds,
    pub needs: Vec<Vec<(usize, f64)>>,
    /// The product each kind is bought as, by the kind's place.
    pub bought_as: Vec<u16>,
    /// Each kind's days from order to service, where a source measures them.
    pub lead: Vec<phx_num::Missing<u32>>,
    /// Each product's lot, the units its price is posted for.
    pub lots: Vec<f64>,
    /// The days between a firm's production decisions, over which its sales outlook runs.
    pub production_days: f64,
}

impl CapOwn {
    /// # Errors
    /// The kinds or a country's plant per unit unread.
    pub fn compile(prims: &kinds::Prims, register: &phx_core::Register, countries: usize) -> Result<CapOwn, String> {
        let kinds = kinds::Kinds::compile(prims, register)?;
        let products = register.products("TEC.products")?.len();
        let scale = libm::pow(consts::TEN, f64::from(if_base::consts::PER_UNIT_EXP));
        let mut needs = Vec::new();
        for c in 0..countries {
            let id = phx_id::CountryId::new(u8::try_from(c).map_err(|e| e.to_string())?);
            let capital = register.table2_in("TEC.capital", id)?;
            for j in 0..products {
                let col = i64::try_from(j).map_err(|e| e.to_string())?;
                let way = capital
                    .rows()
                    .iter()
                    .filter_map(|k| {
                        let v = capital.at(*k, col).ok()?;
                        Some((usize::try_from(*k).ok()?, phx_rand::float::from_i64(v) / scale))
                    })
                    .collect();
                needs.push(way);
            }
        }
        let bought_as = register
            .table1(BOUGHT_AS.id)?
            .values()
            .iter()
            .map(|v| u16::try_from(*v).map_err(|e| e.to_string()))
            .collect::<Result<Vec<u16>, String>>()?;
        let lots = register
            .products("TEC.products")?
            .iter()
            .map(|e| match register.units().named(&e.unit) {
                phx_num::Missing::Present(u) => {
                    Ok(libm::pow(consts::TEN, f64::from(register.units().decl(u).map_or(0, |d| d.price_exp))))
                }
                phx_num::Missing::Absent => Err(format!("product `{}` in an undeclared unit", e.name)),
            })
            .collect::<Result<Vec<f64>, String>>()?;
        let lead_table = register.table1(LEAD_DAYS.id)?;
        let lead = (0..kinds.kinds.len())
            .map(|k| {
                let at = i64::try_from(k).map_err(|e| e.to_string())?;
                match lead_table.at(at) {
                    Ok(days) => u32::try_from(days).map(phx_num::Missing::Present).map_err(|e| e.to_string()),
                    Err(_) => Ok(phx_num::Missing::Absent),
                }
            })
            .collect::<Result<Vec<_>, String>>()?;
        let production_days = phx_rand::float::from_u64(register.count("FRM.production_days")?);
        Ok(CapOwn { kinds, needs, bought_as, lead, lots, production_days })
    }
}

/// Plant.
#[derive(Debug)]
pub struct Cap;

impl System for Cap {
    const CODE: &'static str = "CAP";

    fn declare(d: &mut Declarations) {
        let prims = kinds::Prims {
            depreciation: d.prim(&DEPRECIATION),
            life: d.prim(&SERVICE_LIFE),
            shape: d.prim(&EFFICIENCY_SHAPE),
            classes: d.prim(&CONDITION_CLASSES),
        };
        let _ = d.prim::<Table1>(&LEAD_DAYS);
        let _ = d.prim::<Table1>(&BOUGHT_AS);
        let _ = d.prim::<Count>(&REVIEW_DAYS);
        let stock = d.prim(&STOCK_PER_GDP);
        d.stream(OpeningStream::DECL);
        d.stream(VisitStream::DECL);
        d.compile(Box::new(move |register, countries| Ok(Box::new(CapOwn::compile(&prims, register, countries)?))));
        let capacity = <if_firm::facts::Capacity as phx_core::FactDef>::ITEM;
        d.claim(capacity.name);
        d.facet(phx_core::FacetDecl { fact: capacity.name, kind: HOLDERS[0] });
        d.pop_kind(HOLDERS[1]).position(phx_core::PositionDecl { name: capacity.name, clause: capacity.clause });
        d.contribution(Box::new(opening::Declared));
        d.contribution(Box::new(Plant { prims, stock }));
        let cadence = Cadence::Schedule { days: REVIEW_DAYS.id, runs_on: RunsOn::Any };
        for (handler, kind) in [(review::ReviewSmall::NAME, HOLDERS[1]), (review::ReviewLarge::NAME, HOLDERS[0])] {
            d.visit(VisitDecl { handler, kind, cadence, stream: VisitStream::DECL.name, wakes: &[], clause: "CAP.4" });
        }
        for visit in [review::ReviewSmall::NAME, review::ReviewLarge::NAME] {
            d.wear(WearDecl { visit, specs: kinds::wear_specs, clause: "CAP.6" });
        }
        d.family(Box::new(families::Stock));
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<review::ReviewSmall>();
        h.add::<review::ReviewLarge>();
    }
}
