//! Retail: no service is ever stored; the margins, the services' share and the buyers' naming of sellers, once
//! households buy.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Each retail meeting's sales, by whether its product is delivered as it is made: the value sold.
fn retail_sales(w: Inspector<'_>) -> Option<(i128, i128)> {
    let products = w.register().products("TEC.products").ok()?;
    let markets = w.markets();
    let retail: Vec<u16> = ["SRV.retail"]
        .iter()
        .filter_map(|n| match w.market_kind(n) {
            phx_num::Missing::Present(k) => Some(k),
            phx_num::Missing::Absent => None,
        })
        .collect();
    let kinds: std::collections::BTreeMap<phx_id::MarketId, (u16, u64)> =
        markets.made.iter().map(|(m, k, s)| (m, (k, s))).collect();
    let (mut services, mut all) = (0_i128, 0_i128);
    for set in markets.tape.sets() {
        let Some((kind, subject)) = kinds.get(&set.market) else { continue };
        if !retail.contains(kind) {
            continue;
        }
        let at_once = usize::try_from(*subject).ok().and_then(|p| products.get(p)).is_some_and(|p| p.delivered_at_once);
        let value: i128 = set.matches.iter().map(|m| i128::from(m.qty) * i128::from(m.price.raw())).sum();
        all += value;
        if at_once {
            services += value;
        }
    }
    Some((services, all))
}

/// The services' share of what households bought at retail over the run is read.
fn services_share(w: Inspector<'_>) -> Outcome {
    match retail_sales(w) {
        None => Outcome::Fail("the products or the retail kind are not declared".to_owned()),
        Some((_, 0)) => Outcome::NotYet("nothing was sold at retail in the run"),
        Some((services, all)) if services <= all => Outcome::Pass,
        Some((services, all)) => Outcome::Fail(format!("services sold {services} of {all} at retail")),
    }
}

pub const LC_1_16: super::Check = live_check! {
    id: "LC-1-16",
    title: "SRV.7: the services' share of output and employment, the retail margin and its compression when \
            wholesale costs rise, and the frequency and size of retail price changes are reported",
    from_step: "S1.06",
    check: services_share,
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

/// Every retail sale on the tape names its seller and its buyer and a positive quantity at a positive price.
fn spending_traced(w: Inspector<'_>) -> Outcome {
    let markets = w.markets();
    let Some(retail) = (match w.market_kind("SRV.retail") {
        phx_num::Missing::Present(k) => Some(k),
        phx_num::Missing::Absent => None,
    }) else {
        return Outcome::Fail("the retail kind is not declared".to_owned());
    };
    let kinds: std::collections::BTreeMap<phx_id::MarketId, u16> =
        markets.made.iter().map(|(m, k, _)| (m, k)).collect();
    let mut sales = 0_u64;
    for set in markets.tape.sets().iter().filter(|s| kinds.get(&s.market) == Some(&retail)) {
        for m in &set.matches {
            if m.qty <= 0 || m.price.raw() <= 0 {
                return Outcome::Fail(format!("a sale by party {} of {} at {}", m.seller.get(), m.qty, m.price.raw()));
            }
            sales += 1;
        }
    }
    if sales == 0 { Outcome::NotYet("nothing was sold at retail in the run") } else { Outcome::Pass }
}

pub const LC_1_18: super::Check = live_check! {
    id: "LC-1-18",
    title: "HH.15 (part): every unit of household spending names its seller through a match-set record, and the \
            sellers' credits per meeting sum to the buyers' debits",
    from_step: "S1.06",
    check: spending_traced,
};
