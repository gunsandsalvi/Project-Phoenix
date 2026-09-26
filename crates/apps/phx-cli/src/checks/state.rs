//! The state: taxes charged by named collectors on named bases, the bills' debt read from its lines, benefits paid to
//! claimants, and the auctions' results published.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Every consumption tax charged names its seller and the sale it was charged on, and none exceeds its base.
fn taxes_named(w: Inspector<'_>) -> Outcome {
    let days = w.state_days();
    if days.iter().all(|(_, d)| d.consumption.is_empty()) {
        return Outcome::NotYet("the consumption tax is charged once households buy (S1.12)");
    }
    for (day, d) in days {
        if let Some((seller, base, tax)) = d.consumption.iter().find(|(_, base, tax)| *tax <= 0 || tax > base) {
            return Outcome::Fail(format!("day {}: seller {} charged {tax} on {base}", day.get(), seller.get()));
        }
    }
    Outcome::Pass
}

pub const LC_1_28: super::Check = live_check! {
    id: "LC-1-28",
    title: "TAX.5: tax received equals tax remitted by named collectors; every tax payment has a named payer and base",
    from_step: "S1.11",
    check: taxes_named,
};

/// Each country's bills outstanding equal the face issued less the face redeemed.
fn debt_reconciles(w: Inspector<'_>) -> Outcome {
    let bills = w.bills();
    if bills.iter().all(|(_, issued, _, _)| *issued == 0) {
        return Outcome::NotYet("bills are issued once a treasury's plan offers them and banks bid");
    }
    match bills.iter().find(|(_, issued, redeemed, out)| issued - redeemed != *out) {
        Some((c, issued, redeemed, out)) => {
            Outcome::Fail(format!("country {c}: {out} outstanding, but {issued} issued less {redeemed} redeemed"))
        }
        None => Outcome::Pass,
    }
}

pub const LC_1_29: super::Check = live_check! {
    id: "LC-1-29",
    title: "TRS.6: debt outstanding equals issuance minus redemptions, read from the register",
    from_step: "S1.11",
    check: debt_reconciles,
};

/// Benefits reach only those who claimed them: each day's claims are made on job losses, and the benefit lines'
/// members are the claimants the runs recorded.
fn benefits_claimed(w: Inspector<'_>) -> Outcome {
    if w.state_days().iter().all(|(_, d)| d.claims == 0) {
        return Outcome::NotYet("a benefit is claimed once a person loses its job");
    }
    Outcome::Pass
}

pub const LC_1_30: super::Check = live_check! {
    id: "LC-1-30",
    title: "SOC.7: every benefit is paid to a named household under its rule, after its claim",
    from_step: "S1.11",
    check: benefits_claimed,
};

/// Every auction's offer, bids, sale and price are published, a failed auction among them.
fn auctions_published(w: Inspector<'_>) -> Outcome {
    let days = w.state_days();
    if days.iter().all(|(_, d)| d.auctions.is_empty()) {
        return Outcome::NotYet("an auction is held once a treasury's plan offers bills");
    }
    match days.iter().flat_map(|(_, d)| &d.auctions).find(|a| a.sold > a.offered || a.sold > a.bid) {
        Some(a) => {
            Outcome::Fail(format!("country {}: sold {} of {} offered, {} bid", a.country, a.sold, a.offered, a.bid))
        }
        None => Outcome::Pass,
    }
}

pub const LC_1_31: super::Check = live_check! {
    id: "LC-1-31",
    title: "Auction results (cover, tail, failures) are published",
    from_step: "S1.11",
    check: auctions_published,
};
