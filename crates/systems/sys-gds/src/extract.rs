//! The extractor's decision on each deposit whose right it holds, on its own schedule: the grade it takes now, today's
//! price there against the price it expects, and so how much to work and at what it will sell what it holds. And the
//! stock visit, at whose rows the kernel realises what spoiled since the last.

use phx_core::register::values::{Table1, Table2};
use phx_core::{Prim, Register};
use phx_ledger::instruction::name_code;
use phx_num::{Count, Missing};
use phx_rand::float::from_i64;

/// The handles the extraction reads.
#[derive(Clone, Copy, Debug)]
pub struct Prims {
    pub bounds: Prim<Table2>,
    pub fall: Prim<Table1>,
    pub days: Prim<Count>,
    pub standardised: Prim<Table1>,
    pub spoilage: Prim<Table1>,
    pub merchant: Prim<Count>,
    pub merchant_days: Prim<Count>,
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

/// What the goods' handlers read of the register, compiled once: each product, the days between an extractor's
/// decisions, each product's yearly loss in stock, the product merchants sell, and the days between their decisions.
#[derive(Clone, Debug, PartialEq)]
pub struct Own {
    pub products: Vec<Product>,
    pub days: i64,
    pub spoilage: Vec<f64>,
    pub merchant: u16,
    pub merchant_days: i64,
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
        let merchant_days = i64::try_from(p.merchant_days.shared(register).get()).map_err(|e| e.to_string())?;
        if days <= 0 || merchant_days <= 0 {
            return Err("an extraction or merchants' schedule of no days".to_owned());
        }
        let spoilage = crate::decimals(p.spoilage.shared(register).values(), crate::places(&crate::SPOILAGE_RATE));
        let merchant = u16::try_from(p.merchant.shared(register).get()).map_err(|e| e.to_string())?;
        Ok(Own { products, days, spoilage, merchant, merchant_days })
    }

    /// The kind a product's goods meet in.
    #[must_use]
    pub fn market(&self, product: u16) -> u64 {
        let standardised = self.products.get(usize::from(product)).is_some_and(|p| p.standardised);
        let kind = if standardised { crate::markets::COMMODITIES } else { crate::markets::BETWEEN_FIRMS };
        name_code(kind.key.kind)
    }
}
