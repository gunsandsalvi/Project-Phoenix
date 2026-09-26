//! Retail: no service is ever stored; the margins, the services' share and the buyers' naming of sellers, once
//! households buy.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

fn no_margins(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("the retail margin and the services' share are read once households buy (S1.12)")
}

pub const LC_1_16: super::Check = live_check! {
    id: "LC-1-16",
    title: "SRV.7: the services' share of output and employment, the retail margin and its compression when \
            wholesale costs rise, and the frequency and size of retail price changes are reported",
    from_step: "S1.06",
    check: no_margins,
};

/// No good of a product delivered as it is made has units in existence.
fn none_stored(w: Inspector<'_>) -> Outcome {
    let Ok(products) = w.register().products("TEC.products") else {
        return Outcome::Fail("the products are not declared".to_owned());
    };
    let ledger = &w.books().ledger;
    for (key, id) in ledger.goods.iter() {
        let at_once = products.get(usize::from(key.product)).is_some_and(|p| p.delivered_at_once);
        let issued = ledger.instruments.get(id).issued.n();
        if at_once && issued != 0 {
            return Outcome::Fail(format!(
                "{issued} units of product {} stored in zone {}",
                key.product,
                key.zone.get()
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_17: super::Check = live_check! {
    id: "LC-1-17",
    title: "no service is stored: no good of a product delivered as it is made has units in existence at a close",
    from_step: "S1.06",
    check: none_stored,
};

fn no_household_spending(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("every unit of household spending is traced to its seller once households buy (S1.12)")
}

pub const LC_1_18: super::Check = live_check! {
    id: "LC-1-18",
    title: "HH.15 (part): every unit of household spending names its seller through a match-set record, and the \
            sellers' credits per meeting sum to the buyers' debits",
    from_step: "S1.06",
    check: no_household_spending,
};
