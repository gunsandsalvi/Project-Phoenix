//! TEC: the products the world makes and the ways it makes them, in physical units, with the ways each country's
//! industries know from the opening and each firm's known ways. Research, imitation and learning come later (Stage 6).

pub mod prims;
pub mod products;
pub mod sets;
pub mod technology;
pub mod ways;

use phx_core::{Declarations, FactDef, HandlerTable, System};

pub use technology::Technology;

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
        let known = <if_firm::known::Known as FactDef>::ITEM.name;
        d.claim(known);
        d.claim(<if_firm::known::WayUsed as FactDef>::ITEM.name);
    }

    fn handlers(_: &mut HandlerTable) {}
}
