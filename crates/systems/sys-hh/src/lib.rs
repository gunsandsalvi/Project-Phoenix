//! HH, households: each spends on its weekly schedule by the buffer-stock rule over its own cash and its outlook of its
//! permanent income, and asks for its budget's shares of what it spends at retail. Its work, savings and founding
//! arrive with their own steps.

mod buffer;
mod consts;

pub use buffer::{Model, Solution, solve, spend};

use if_pop::facts::{After, Income};
use phx_core::{
    Cadence, Ctx, Declarations, FactStore, HandlerDecl, HandlerTable, Reads, RunsOn, StreamDef, System, VisitDecl,
    Writes, declare_handler, declare_prim, declare_stream,
};
use phx_id::Slot;
use phx_macros::clause;
use phx_market::intents::ShopIntent;
use phx_market::retail::Want;
use phx_num::{Count, Fixed, Missing};

use crate::consts::WEEKS_A_YEAR;

declare_stream! { pub VisitStream = "HH.visits" { purpose: SchedulePhase, keyed: false, clause: "HH.4" } }

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

declare_prim! {
    /// A household's shares of its spending over the products, in their order.
    pub BUDGET_SHARES = "HH.budget_shares" {
        kind: Preference, value: Table1 { axis_exp: 0, exp: 6 }, clause: "HH.5", scope: PerCountry
    }
}

/// What the households' decisions read, compiled at assembly: the buffer-stock rule solved for the type, the outlook's
/// gain, and each country's budget shares.
#[derive(Debug)]
pub struct Own {
    pub rule: Solution,
    pub gain: f64,
    pub shares: Vec<Vec<f64>>,
}

/// The retail kind's code a household's wants name.
const RETAIL: &str = "SRV.retail";

declare_handler! {
    /// A household's spending on its weekly schedule.
    pub Spend = "HH.spend" {
        substep: S5c,
        table: "household",
        reads: [Income, After],
        writes: [Income, After],
        intents: [ShopIntent],
        clause: "HH.4",
        body: spend_week,
    }
}

/// A household's week: its income since its last decision taken into its outlook of its permanent income, its
/// spending set by the rule at its cash on hand over that outlook, and its budget's share of that asked of each product
/// at retail. On its first decision it only notes what it holds.
#[clause("HH.1", "HH.2", "HH.4", "HH.5", "HH.18", "HH.19", "REP.5")]
fn spend_week<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl + Reads<Income> + Reads<After> + Writes<Income> + Writes<After> + phx_core::Emits<ShopIntent>,
    S: FactStore + ?Sized,
{
    let own: &Own = ctx.own::<Own>();
    let Missing::Present(money) = ctx.money(row) else { return };
    let Missing::Present(after) = ctx.read::<After>(row) else {
        ctx.write::<After>(row, money);
        return;
    };
    let week = phx_rand::float::from_i64(money - after) * WEEKS_A_YEAR;
    let income = match ctx.read::<Income>(row) {
        Missing::Present(p) => phx_val::heuristics::adaptive(phx_rand::float::from_i64(p), week, own.gain),
        Missing::Absent => week,
    };
    if income <= 0.0 {
        ctx.write::<After>(row, money);
        return;
    }
    let cash = phx_rand::float::from_i64(money) / income;
    let spent = buffer::spend(&own.rule, cash) * income / WEEKS_A_YEAR;
    let Some(shares) = own.shares.first() else { return };
    let mut total = 0;
    for (product, share) in (0_u16..).zip(shares) {
        let Some(amount) = phx_rand::float::floor_to_i64(spent * share) else {
            phx_num::capacity_exceeded!("a household's spending on a product", i64::MAX, 0);
        };
        if amount > 0 {
            ctx.emit(&ShopIntent {
                row,
                kind: phx_ledger::instruction::name_code(RETAIL),
                product,
                want: Want::Money(amount),
            });
            total += amount;
        }
    }
    let Some(outlook) = phx_rand::float::floor_to_i64(income) else {
        phx_num::capacity_exceeded!("a household's income outlook", i64::MAX, 0);
    };
    ctx.write::<Income>(row, outlook);
    ctx.write::<After>(row, money - total);
}

/// Households.
#[derive(Debug)]
pub struct Hh;

impl System for Hh {
    const CODE: &'static str = "HH";

    fn declare(d: &mut Declarations) {
        d.stream(VisitStream::DECL);
        let _: phx_core::Prim<Fixed<3>> = d.prim(&PATIENCE);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&RISK_AVERSION);
        for p in [&PERMANENT_SD, &TRANSITORY_SD, &REAL_RETURN, &INCOME_GROWTH] {
            let _: phx_core::Prim<Fixed<3>> = d.prim(p);
        }
        let _: phx_core::Prim<Fixed<4>> = d.prim(&NO_INCOME);
        let _: phx_core::Prim<Fixed<2>> = d.prim(&INCOME_GAIN);
        let _: phx_core::Prim<Count> = d.prim(&SPENDING_DAYS);
        let _: phx_core::Prim<phx_core::register::values::Table1> = d.prim(&BUDGET_SHARES);
        for fact in [<Income as phx_core::FactDef>::ITEM.name, <After as phx_core::FactDef>::ITEM.name] {
            d.claim(fact);
        }
        let mut household = d.pop_kind(if_pop::HOUSEHOLD);
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
                    let t = register.table1_in(BUDGET_SHARES.id, id)?;
                    Ok(t.values().iter().map(|v| phx_rand::float::from_i64(*v) / crate::consts::SHARE_PARTS).collect())
                })
                .collect::<Result<Vec<Vec<f64>>, String>>()?;
            Ok(Box::new(Own { rule, gain: register.fixed(INCOME_GAIN.id)?, shares }))
        }));
        d.visit(VisitDecl {
            handler: Spend::NAME,
            kind: if_pop::HOUSEHOLD,
            cadence: Cadence::Schedule { days: SPENDING_DAYS.id, runs_on: RunsOn::Business },
            stream: VisitStream::DECL.name,
            wakes: &[],
            clause: "HH.4",
        });
    }

    fn handlers(h: &mut HandlerTable) {
        h.add::<Spend>();
    }
}
