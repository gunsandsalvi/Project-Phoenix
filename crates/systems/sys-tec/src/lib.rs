//! TEC: the products the world makes and the ways it makes them, in physical units, with the ways each country's
//! industries know from the opening and each firm's known ways. Research, imitation and learning come later (Stage 6).

pub mod prims;
pub mod products;
pub mod sets;
pub mod technology;
pub mod ways;

use phx_core::{Declarations, System};

pub use technology::Technology;

/// Each product's resource and draw on its deposit a unit made, by the product's place; none for a product not
/// extracted.
///
/// # Errors
/// Products the register refuses, or a draw table not by the products' places.
pub fn deposit_draws(register: &phx_core::Register) -> Result<Vec<Option<(u16, if_base::PerUnit)>>, String> {
    let products = register.products("TEC.products")?;
    let draws = register.table1(prims::DEPOSIT_DRAW.id)?;
    (0_i64..)
        .zip(products)
        .map(|(i, p)| match p.extracts {
            phx_num::Missing::Present(r) => draws
                .at(i)
                .map(|raw| Some((r, if_base::PerUnit::from_raw(raw))))
                .map_err(|_| format!("no draw for extracted product {i}")),
            phx_num::Missing::Absent => Ok(None),
        })
        .collect()
}

/// The technology system.
#[derive(Debug)]
pub struct Tec;

impl System for Tec {
    const CODE: &'static str = "TEC";

    fn declare(d: &mut Declarations) {
        let prims = prims::TecPrims::declare(d);
        d.compile(Box::new(move |register, countries| {
            let tech = Technology::compile(&prims, register, countries)?;
            Ok(Box::new(tech))
        }));
    }
}
