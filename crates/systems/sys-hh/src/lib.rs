//! HH, households: each spends on its weekly schedule by the buffer-stock rule over its own cash and its outlook of its
//! permanent income, and asks for its budget's shares of what it spends at retail. Its work, savings and founding
//! arrive with their own steps.

pub mod buffer;
mod consts;
pub mod points;

pub use buffer::{Model, Solution, solve, spend};

use if_pop::facts::{After, Income, Looked, Received};
use phx_core::{AttrDecl, Declarations, StreamDef, System, declare_prim, declare_stream};
use phx_num::{Count, Fixed};

use crate::consts::DAYS_A_YEAR;

declare_stream! { pub VisitStream = "HH.visits" { purpose: SchedulePhase, keyed: false, clause: "HH.4" } }
declare_stream! { pub TypesStream = "HH.outlook_types" { purpose: Opening, keyed: false, clause: "VAL.22" } }
declare_stream! { pub StanceStream = "HH.stance" { purpose: Occasion, keyed: false, clause: "VAL.7" } }

/// A household's memory type, at which its outlooks of public series correct.
pub const MEMORY_ATTR: AttrDecl = AttrDecl { name: "HH.memory", values: crate::consts::MOST_TYPES, clause: "VAL.22" };
/// A household's switching type: how strongly its stance moves toward the heuristic that has forecast best.
pub const SWITCHING_ATTR: AttrDecl =
    AttrDecl { name: "HH.switching", values: crate::consts::MOST_TYPES, clause: "VAL.22" };
/// The heuristic of the menu a household's outlooks of public series rely on.
pub const STANCE_ATTR: AttrDecl = AttrDecl { name: "HH.stance", values: crate::consts::MOST_TYPES, clause: "VAL.7" };
/// The age class of a household's head, whose lived years weight its outlooks of public series.
pub const WINDOW_ATTR: AttrDecl = AttrDecl { name: "HH.window", values: crate::consts::MOST_TYPES, clause: "VAL.23" };

declare_prim! {
    /// A household type's yearly patience, the weight of next year's utility.
    pub PATIENCE = "HH.patience" { kind: Preference, value: Fixed { exp: 3 }, clause: "HH.20", scope: Shared }
}

declare_prim! {
    /// A household type's relative risk aversion.
    pub RISK_AVERSION = "HH.risk_aversion" { kind: Preference, value: Fixed { exp: 2 }, clause: "HH.20", scope: Shared }
}

declare_prim! {
    /// The spread of the log of a year's permanent shock to a household's income.
    pub PERMANENT_SD = "HH.permanent_sd" { kind: Endowment, value: Fixed { exp: 3 }, clause: "HH.20", scope: Shared }
}

declare_prim! {
    /// The spread of the log of a year's transitory shock to a household's income.
    pub TRANSITORY_SD = "HH.transitory_sd" {
        kind: Endowment, value: Fixed { exp: 3 }, clause: "HH.20", scope: Shared
    }
}

declare_prim! {
    /// The chance of a year with no income.
    pub NO_INCOME = "HH.no_income_chance" { kind: Endowment, value: Fixed { exp: 4 }, clause: "HH.20", scope: Shared }
}

declare_prim! {
    /// The gross real return a household earns on what it saves, until it reads its banks' posted rates.
    pub REAL_RETURN = "HH.real_return" {
        kind: Shape, value: Fixed { exp: 3 }, clause: "HH.4", scope: Shared, shape: placeholder("BFL")
    }
}

declare_prim! {
    /// The gross growth a household expects of its permanent income, until it forms its own outlook of it.
    pub INCOME_GROWTH = "HH.income_growth" {
        kind: Shape, value: Fixed { exp: 3 }, clause: "HH.4", scope: Shared, shape: placeholder("HH")
    }
}

declare_prim! {
    /// The share of its last surprise a household's outlook of its income takes in at each look.
    pub INCOME_GAIN = "HH.income_gain" { kind: Preference, value: Fixed { exp: 2 }, clause: "VAL.6", scope: Shared }
}

declare_prim! {
    /// The days between a household's spending decisions.
    pub SPENDING_DAYS = "HH.spending_days" { kind: Preference, value: Count, clause: "HH.4", scope: Shared }
}

/// The accounts' composition of the final uses, and households' column of it.
const COMPOSITION: &str = "GEN.final_composition";
const HOUSEHOLDS: i64 = 0;

/// What the households' decisions read, compiled at assembly: the buffer-stock rule solved for the type, the outlook's
/// gain, the share of a year between two decisions, and each country's budget shares.
#[derive(Debug)]
pub struct Own {
    pub rule: Solution,
    pub gain: f64,
    pub period: f64,
    pub shares: Vec<Vec<f64>>,
    /// The outlook types households are drawn by and the heuristics' parameters.
    pub types: phx_val::types::Types,
}

/// Households.
#[derive(Debug)]
pub struct Hh;

impl System for Hh {
    const CODE: &'static str = "HH";

    fn declare(d: &mut Declarations) {
        d.stream(VisitStream::DECL);
        d.stream(TypesStream::DECL);
        d.stream(StanceStream::DECL);
        d.decision(&points::SPEND);
        d.decision(&points::STANCE);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&PATIENCE);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&RISK_AVERSION);
        for p in [&PERMANENT_SD, &TRANSITORY_SD, &REAL_RETURN, &INCOME_GROWTH] {
            let _: phx_core::Prim<Fixed<3>> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<4>> = d.prim(&NO_INCOME);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&INCOME_GAIN);
        let _: phx_core::Prim<Count> = d.prim(&SPENDING_DAYS);
        for fact in [
            <Income as phx_core::FactDef>::ITEM.name,
            <After as phx_core::FactDef>::ITEM.name,
            <Received as phx_core::FactDef>::ITEM.name,
            <Looked as phx_core::FactDef>::ITEM.name,
        ] {
            d.claim(fact);
        }
        let mut household = d.pop_kind(if_pop::HOUSEHOLD);
        household.attr(MEMORY_ATTR).attr(SWITCHING_ATTR).attr(STANCE_ATTR).attr(WINDOW_ATTR);
        for p in if_pop::facts::POSITIONS {
            household.position(p);
        }
        d.compile(Box::new(|register, countries| {
            let model = Model {
                beta: register.fixed(PATIENCE.id)?,
                rho: register.fixed(RISK_AVERSION.id)?,
                r: register.fixed(REAL_RETURN.id)?,
                g: register.fixed(INCOME_GROWTH.id)?,
                sigma_permanent: register.fixed(PERMANENT_SD.id)?,
                sigma_transitory: register.fixed(TRANSITORY_SD.id)?,
                unemployment: register.fixed(NO_INCOME.id)?,
            };
            let rule = solve(&model)?;
            let shares = (0..countries)
                .map(|i| {
                    let id = phx_id::CountryId::new(u8::try_from(i).map_err(|e| e.to_string())?);
                    // Households' budget shares are their part of the final uses' composition spent on the products.
                    let t = register.table2_in(COMPOSITION, id)?;
                    let products = register.products("TEC.products")?.len();
                    let parts = (0_i64..)
                        .take(products)
                        .map(|p| t.at(p, HOUSEHOLDS).map(phx_rand::float::from_i64).map_err(|e| format!("{e:?}")))
                        .collect::<Result<Vec<f64>, String>>()?;
                    let whole: f64 = parts.iter().sum();
                    if whole <= 0.0 {
                        return Err(format!("`{COMPOSITION}` gives households no spending on the products"));
                    }
                    Ok(parts.iter().map(|v| v / whole).collect())
                })
                .collect::<Result<Vec<Vec<f64>>, String>>()?;
            let days = register.count(SPENDING_DAYS.id)?;
            let period = phx_rand::float::from_u64(days) / DAYS_A_YEAR;
            let types = phx_val::types::Types::compile(register)?;
            let most = usize::try_from(crate::consts::MOST_TYPES).map_err(|e| e.to_string())?;
            if types.gains.len() > most || types.intensities.len() > most || phx_val::heuristic::MENU.len() > most {
                return Err(format!("more outlook types or heuristics than a household's attribute holds ({most})"));
            }
            Ok(Box::new(Own { rule, gain: register.fixed(INCOME_GAIN.id)?, period, shares, types }))
        }));
    }
}
