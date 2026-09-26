//! The extractor's decision on each deposit whose right it holds, on its own schedule: the grade it takes now, today's
//! price there against the price it expects, and so how much to work and at what it will sell what it holds. And the
//! stock visit, at whose rows the kernel realises what spoiled since the last.

use if_firm::facts::{Method, OutputRate, RequiredReturn, UnitCost};
use phx_core::handler::{Ctx, FactStore, HandlerDecl, Reads};
use phx_core::register::values::{Table1, Table2};
use phx_core::{Emits, Prim, Register, declare_handler};
use phx_id::Slot;
use phx_ledger::instruction::{Source, name_code};
use phx_ledger::intents::{Made, Transform};
use phx_macros::clause;
use phx_market::intents::OrderIntent;
use phx_market::order::{Side, Step};
use phx_num::{Count, Missing, PriceRaw};
use phx_rand::float::from_i64;

use crate::consts::DAYS_A_YEAR;
use crate::rules;

/// The handles the extraction reads.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub bounds: Prim<Table2>,
    pub fall: Prim<Table1>,
    pub days: Prim<Count>,
    pub standardised: Prim<Table1>,
}

/// A product as its extraction and its trade read it: its classes' bounds, how its grade falls as its deposit is
/// worked, its least traded quantity, and whether it meets in a call.
#[derive(Clone, Debug, PartialEq)]
pub struct Product {
    pub bounds: Vec<f64>,
    pub fall: f64,
    pub lot: i64,
    pub standardised: bool,
}

/// What the goods' handlers read of the register, compiled once: each product, and the days between decisions.
#[derive(Clone, Debug, PartialEq)]
pub struct Own {
    pub products: Vec<Product>,
    pub days: i64,
}

impl Own {
    /// # Errors
    /// A product whose unit the world does not declare, a schedule of no days, or tables out of the products' order.
    pub fn compile(p: &Prims, register: &Register) -> Result<Own, String> {
        let entries = register.products("TEC.products")?;
        let (bounds, fall, standardised) =
            (p.bounds.shared(register), p.fall.shared(register), p.standardised.shared(register));
        let bounds_exp = crate::places(&crate::GRADE_BOUNDS);
        let fall_exp = crate::places(&crate::GRADE_FALL);
        let scale = |v: i64, exp: u8| from_i64(v) / libm::pow(crate::consts::TEN, f64::from(exp));
        let mut products = Vec::with_capacity(entries.len());
        for (i, e) in (0_i64..).zip(entries) {
            let Missing::Present(unit) = register.units().named(&e.unit) else {
                return Err(format!("product `{}` is counted in an undeclared unit", e.name));
            };
            let exp = register.units().decl(unit).map_or(0, |d| d.price_exp);
            let lot = (0..exp).try_fold(1_i64, |l, _| l.checked_mul(crate::consts::DECADE));
            let Some(lot) = lot else { return Err(format!("`{}`'s price exponent beyond a quantity", e.unit)) };
            let classes: Vec<f64> = if bounds.rows().contains(&i) {
                bounds
                    .columns()
                    .iter()
                    .map(|c| bounds.at(i, *c).map(|v| scale(v, bounds_exp)).map_err(|_| format!("no bound {c}")))
                    .collect::<Result<_, String>>()?
            } else {
                Vec::new()
            };
            products.push(Product {
                bounds: classes,
                fall: fall.at(i).map_or(0.0, |v| scale(v, fall_exp)),
                lot,
                standardised: standardised.at(i).is_ok_and(|v| v == 1),
            });
        }
        let days = i64::try_from(p.days.shared(register).get()).map_err(|e| e.to_string())?;
        if days <= 0 {
            return Err("an extraction schedule of no days".to_owned());
        }
        Ok(Own { products, days })
    }

    /// The kind a product's goods meet in.
    #[must_use]
    pub fn market(&self, product: u16) -> u64 {
        let standardised = self.products.get(usize::from(product)).is_some_and(|p| p.standardised);
        let kind = if standardised { crate::markets::COMMODITIES } else { crate::markets::BETWEEN_FIRMS };
        name_code(kind.key.kind)
    }
}

declare_handler! {
    /// A small firm's decisions on the deposits whose rights it holds.
    pub ExtractSmall = "GDS.extract_small" {
        substep: S5c,
        table: "small_firm",
        reads: [UnitCost, OutputRate, RequiredReturn, Method],
        writes: [],
        intents: [Transform, OrderIntent],
        clause: "GDS.4",
        body: extract,
    }
}

declare_handler! {
    /// A large firm's decisions on the deposits whose rights it holds.
    pub ExtractLarge = "GDS.extract_large" {
        substep: S5c,
        table: "firm",
        reads: [UnitCost, OutputRate, RequiredReturn, Method],
        writes: [],
        intents: [Transform, OrderIntent],
        clause: "GDS.4",
        body: extract,
    }
}

declare_handler! {
    /// A small firm's stock visit.
    pub StockSmall = "GDS.stock_small" {
        substep: S5b,
        table: "small_firm",
        reads: [],
        writes: [],
        clause: "GDS.8",
        body: stock,
    }
}

declare_handler! {
    /// A large firm's stock visit.
    pub StockLarge = "GDS.stock_large" {
        substep: S5b,
        table: "firm",
        reads: [],
        writes: [],
        clause: "GDS.8",
        body: stock,
    }
}

/// Nothing is decided at the stock visit: it is the day what spoiled since the last is realised.
fn stock<H, S>(_: &mut Ctx<'_, H, S>, _: Slot)
where
    H: HandlerDecl,
    S: FactStore + ?Sized,
{
}

fn read<F, H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot) -> Option<i64>
where
    F: phx_core::FactDef,
    H: HandlerDecl + Reads<F>,
    S: FactStore + ?Sized,
{
    match ctx.read::<F>(row) {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

/// The extraction: for each deposit whose right the firm holds, the grade class its next units take; when today's
/// price there, net of the firm's unit cost, beats the net price it expects discounted at the return it requires,
/// the units its plant runs over the days to its next decision, no more than the deposit holds; and an offer of what
/// it then holds of the good, at the least it would take rather than hold, the better of its cost and the price it
/// expects by its method, discounted. A firm without its cost, its run, its required return or its method, or a good
/// with no price or outlook there yet, decides nothing.
#[clause("GDS.4", "GDS.13", "MKT.16")]
fn extract<H, S>(ctx: &mut Ctx<'_, H, S>, row: Slot)
where
    H: HandlerDecl
        + Reads<UnitCost>
        + Reads<OutputRate>
        + Reads<RequiredReturn>
        + Reads<Method>
        + Emits<Transform>
        + Emits<OrderIntent>,
    S: FactStore + ?Sized,
{
    let own: &Own = ctx.own::<Own>();
    let (Some(cost), Some(per_day), Some(required), Some(method)) = (
        read::<UnitCost, H, S>(ctx, row),
        read::<OutputRate, H, S>(ctx, row),
        read::<RequiredReturn, H, S>(ctx, row),
        read::<Method, H, S>(ctx, row).and_then(|m| u16::try_from(m).ok()),
    ) else {
        return;
    };
    let rate = from_i64(required) / crate::consts::RATE_SCALE;
    let horizon = from_i64(own.days) / DAYS_A_YEAR;
    let rights = ctx.rights(row).to_vec();
    for right in rights {
        let Some(product) = own.products.get(usize::from(right.product)) else { continue };
        let taken = match (right.opening, right.remaining) {
            (Missing::Present(o), Missing::Present(r)) if o > 0 => from_i64(o - r) / from_i64(o),
            _ => 0.0,
        };
        let grade = rules::grade::now(from_i64(right.grade) / crate::consts::GRADE_SCALE, product.fall, taken);
        let class = rules::grade::class(grade, &product.bounds);
        let (Missing::Present(price), Missing::Present(outlook)) =
            (ctx.mark(row, right.product, class), ctx.outlook(row, right.product, class, method))
        else {
            continue;
        };
        let Some(unit_cost) = cost.checked_mul(product.lot) else { continue };
        let (price, outlook, unit_cost) = (from_i64(price), from_i64(outlook), from_i64(unit_cost));
        let finite = matches!(right.remaining, Missing::Present(_));
        let mut made = 0;
        if rules::extract::works(price, unit_cost, outlook, (rate, horizon), finite) {
            let remaining = match right.remaining {
                Missing::Present(r) => Some(r),
                Missing::Absent => None,
            };
            made = rules::extract::quantity(per_day, own.days, remaining, product.lot);
        }
        if made > 0 {
            let leg = Made {
                product: right.product,
                grade: class,
                qty: made,
                source: Source::Deposit(right.deposit),
                cost: 0,
            };
            ctx.emit(&Transform { row, reason: name_code(crate::EXTRACTED.name), legs: vec![leg] });
        }
        let held = ctx.held(row, right.product, class) + made;
        let offered = held - held % product.lot;
        let reservation = rules::stockist::carry_value(outlook, (0.0, 0.0), (rate, horizon));
        let floor = if reservation > unit_cost { reservation } else { unit_cost };
        let Some(limit) = phx_rand::float::floor_to_i64(-floor).map(|f| -f) else { continue };
        if offered > 0 {
            let steps = vec![Step { limit: PriceRaw::from_raw(limit), qty: offered }];
            let kind = own.market(right.product);
            ctx.emit(&OrderIntent { row, kind, product: right.product, grade: class, side: Side::Sell, steps });
        }
    }
}
