//! The products the register declares, each unit resolved among the world's units and each industry numbered in
//! the order the products first name it.

use if_base::{IndustryId, ProductDecl, ProductId, Products};
use phx_core::ProductEntry;
use phx_core::register::units::{UnitKind, Units};
use phx_macros::clause;
use phx_num::Missing;

/// The products the register declares.
///
/// # Errors
/// A unit the world does not declare, or one that is not physical, and more products or industries than an
/// identity holds.
#[clause("TEC.1")]
pub fn build(entries: &[ProductEntry], units: &Units) -> Result<Products, String> {
    let mut out = Products::default();
    for (i, e) in entries.iter().enumerate() {
        let Missing::Present(unit) = units.named(&e.unit) else {
            return Err(format!("product `{}` is counted in `{}`, which the world does not declare", e.name, e.unit));
        };
        if units.decl(unit).map(|d| d.kind) != Some(UnitKind::Physical) {
            return Err(format!("product `{}` is counted in `{}`, which is not a physical unit", e.name, e.unit));
        }
        let industry = if let Some(k) = out.industries.iter().position(|n| *n == e.industry) {
            k
        } else {
            out.industries.push(e.industry.clone());
            out.industries.len() - 1
        };
        let (Ok(id), Ok(industry)) = (u16::try_from(i), u16::try_from(industry)) else {
            return Err(format!("{} products, beyond a product's identity", entries.len()));
        };
        out.decls.push(ProductDecl {
            id: ProductId::new(id),
            unit,
            industry: IndustryId::new(industry),
            storable: e.storable,
            delivered_at_once: e.delivered_at_once,
            extracts: e.extracts,
        });
        out.names.push(e.name.clone());
    }
    Ok(out)
}

/// A product's declaration.
#[must_use]
pub fn get(products: &Products, id: ProductId) -> Option<&ProductDecl> {
    products.decls.get(usize::from(id.index()))
}
