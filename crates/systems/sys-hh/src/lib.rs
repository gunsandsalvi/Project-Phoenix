//! HH, households: each spends on its weekly schedule by the buffer-stock rule over its own cash and its outlook of its
//! permanent income, and asks for its budget's shares of what it spends at retail. Its work, savings and founding
//! arrive with their own steps.

pub mod buffer;
mod consts;
pub mod points;

pub use buffer::{Model, Solution, solve, spend};

use if_pop::facts::{After, Income, Looked, Received};
use phx_core::{Declarations, StreamDef, System, declare_prim, declare_stream};
use phx_num::{Count, Fixed};

use crate::consts::DAYS_A_YEAR;

declare_stream! { pub VisitStream = "HH.visits" { family: World, purpose: SchedulePhase, keyed: false, clause: "HH.4" } }
declare_stream! { pub TypesStream = "HH.outlook_types" { family: World, purpose: Opening, keyed: false, clause: "VAL.22" } }
declare_stream! { pub StanceStream = "HH.stance" { family: World, purpose: Occasion, keyed: false, clause: "VAL.7" } }

/// A household's preference type, its memory type (at which its outlooks of public series correct) and its switching
/// type (how strongly its stance moves toward the heuristic that has forecast best) as one value; none past the types a
/// household's preference holds.
#[phx_macros::clause("VAL.22", "NUM.4")]
#[must_use]
pub fn preference_type(memory: u16, switching: u16) -> Option<u16> {
    let most = u16::try_from(crate::consts::MOST_TYPES).ok()?;
    if memory >= most || switching >= most {
        return None;
    }
    memory.checked_mul(most)?.checked_add(switching)
}

/// A preference type's memory and switching types.
#[must_use]
pub fn preference_parts(preference: u16) -> Option<(u16, u16)> {
    let most = u16::try_from(crate::consts::MOST_TYPES).ok()?;
    Some((preference.checked_div(most)?, preference.checked_rem(most)?))
}

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
            if types.gains.len() > most || types.intensities.len() > most {
                return Err(format!("more outlook types than a household's preference type holds ({most} of each)"));
            }
            Ok(Box::new(Own { rule, gain: register.fixed(INCOME_GAIN.id)?, period, shares, types }))
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::{preference_parts, preference_type};

    #[test]
    fn a_preference_type_reads_back_its_parts() {
        for (m, s) in [(0, 0), (3, 7), (15, 15)] {
            assert_eq!(preference_type(m, s).and_then(preference_parts), Some((m, s)));
        }
        assert_eq!(preference_type(16, 0), None, "a memory type past the types held");
    }
}
