//! Goods: their balance by place and the deposits' depletion clean; the markets' reads and a drought's price, once
//! goods are held and traded.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// A family registered and without findings.
fn clean(w: Inspector<'_>, name: &str) -> Result<(), String> {
    if !w.families().iter().any(|f| f.name == name) {
        return Err(format!("the family `{name}` is not registered"));
    }
    match w.findings().iter().find(|f| f.family == name) {
        Some(f) => Err(format!("{} on day {}: {}", f.family, f.day.get(), f.detail)),
        None => Ok(()),
    }
}

/// The goods' family is registered and found nothing, over goods that exist.
fn goods_clean(w: Inspector<'_>) -> Outcome {
    if let Err(e) = clean(w, sys_gds::families::GOODS.name) {
        return Outcome::Fail(e);
    }
    if w.books().ledger.goods.iter().next().is_none() {
        return Outcome::NotYet("firms hold stocks from S1.15");
    }
    Outcome::Pass
}

pub const LC_1_13: super::Check = live_check! {
    id: "LC-1-13",
    title: "per good and place, opening stock plus produced plus arrived equals consumed plus shipped plus spoiled \
            plus destroyed plus closing stock: the family of goods (GDS.10) is clean every close",
    from_step: "S1.05",
    check: goods_clean,
};

fn deposits_clean(w: Inspector<'_>) -> Outcome {
    match clean(w, phx_geo::audit::DEPOSITS.name) {
        Ok(()) => Outcome::Pass,
        Err(e) => Outcome::Fail(e),
    }
}

pub const LC_1_14: super::Check = live_check! {
    id: "LC-1-14",
    title: "for every finite deposit, extracted plus remaining equals its opening quantity: the family of deposits \
            (GEO.12) is clean",
    from_step: "S1.05",
    check: deposits_clean,
};

fn no_prints(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("the goods markets print once firms hold stocks (S1.15); the basis reads freight (S1.07)")
}

pub const LC_1_15: super::Check = live_check! {
    id: "LC-1-15",
    title: "the reads of GDS.11 are reported: volatility against stocks, the basis between places against freight, \
            and producer prices moving before consumer prices",
    from_step: "S1.05",
    check: no_prints,
};

fn no_drought_prices(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("a drought's place prices its crops once firms hold and trade them (S1.15)")
}

pub const LC_1_46: super::Check = live_check! {
    id: "LC-1-46",
    title: "when a drought strikes one place, the price there rises before prices elsewhere (GDS.9)",
    from_step: "S1.05",
    check: no_drought_prices,
};
