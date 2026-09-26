//! Firms: their revenue family clean, their posted prices points of their trades' tables, moved only on review days, and
//! every firm in default of payment past its law's grace ended.

use phx_core::{FactDef, Resolved};
use phx_num::Missing;
use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Why the firms' prices are not read yet.
const NO_PRICES: &str = "no firm posts a price before its latest filed accounts are drawn (S1.15)";

/// The family of the firms' revenue is registered and found nothing.
fn families_clean(w: Inspector<'_>) -> Outcome {
    let names = [sys_frm::families::REVENUE.name];
    let registered = w.families();
    if let Some(missing) = names.iter().find(|n| !registered.iter().any(|f| f.name == **n)) {
        return Outcome::Fail(format!("the family `{missing}` is not registered"));
    }
    match w.findings().iter().find(|f| names.contains(&f.family)) {
        Some(f) => Outcome::Fail(format!("{} on day {}: {}", f.family, f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

pub const LC_1_06: super::Check = live_check! {
    id: "LC-1-06",
    title: "the family of the firms' revenue (FRM.17) is clean; the claims' (FRM.18) joins with the invoices (S2.02)",
    from_step: "S1.03",
    check: families_clean,
};

/// Every price a firm posts is a point of its trade's table, and prices move only at the firms' reviews.
fn prices_are_points(w: Inspector<'_>) -> Outcome {
    let Some(own) = w.own::<sys_frm::Own>("FRM") else { return Outcome::Fail("the world keeps no firms".to_owned()) };
    let name = <if_firm::facts::Price as FactDef>::ITEM.name;
    let (mut posted, mut off) = (0_u64, None);
    let mut see = |party: phx_id::PartyId, price: phx_num::Missing<i64>| {
        if let phx_num::Missing::Present(p) = price {
            posted += 1;
            if off.is_none() && !own.management().is_point(p) {
                off = Some((party, p));
            }
        }
    };
    let books = w.books();
    for party in books.parties.of_kind(sys_frm::FIRM.name) {
        let (place, slot) = books.parties.row(party);
        let t = books.parties.table(place);
        see(party, t.facet_named(name).map_or(phx_num::Missing::Absent, |c| t.fact(slot, c)));
    }
    if let Some(k) = w.population().kinds.iter().position(|k| k.decl.kind == sys_frm::SMALL_FIRM.name) {
        let t = w.agent_table(k);
        for slot in t.slots() {
            see(t.party(slot), t.position(name).map_or(phx_num::Missing::Absent, |c| t.fact(slot, c)));
        }
    }
    if let Some((party, p)) = off {
        return Outcome::Fail(format!("party {} posts {p}, no point of its trade's table", party.get()));
    }
    let reviews = [
        <sys_frm::decide::ReviewSmall as phx_core::HandlerDecl>::NAME,
        <sys_frm::decide::ReviewLarge as phx_core::HandlerDecl>::NAME,
    ];
    for v in w.visit_days() {
        let moved = v.moved_of(name);
        if moved != 0 && reviews.iter().all(|r| v.visits_of(r) == 0) {
            return Outcome::Fail(format!("{moved} prices moved on day {} with no firm reviewed", v.day.get()));
        }
    }
    if posted == 0 {
        return Outcome::NotYet(NO_PRICES);
    }
    Outcome::Pass
}

pub const LC_1_07: super::Check = live_check! {
    id: "LC-1-07",
    title: "every posted price is a point of its trade's table, and prices change only on review or wake days",
    from_step: "S1.03",
    check: prices_are_points,
};

fn price_reads(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet(NO_PRICES)
}

pub const LC_1_08: super::Check = live_check! {
    id: "LC-1-08",
    title: "the frequency and size of price changes, and the markups, are reported per trade (SRV.7, FRM.19)",
    from_step: "S1.03",
    check: price_reads,
};

/// No firm stays in arrears past its law's grace without ending; and firms end, by closure and by default, in every
/// industry every year.
fn firms_end(w: Inspector<'_>) -> Outcome {
    let register = w.register();
    let Ok(graces) = register.counts_per_country(sys_frm::INSOLVENCY_GRACE_DAYS.id) else {
        return Outcome::Fail("the insolvency law's grace is not declared per country".to_owned());
    };
    let books = w.books();
    let today = w.today();
    for (key, since) in books.ledger.arrears().iter() {
        let Resolved::Live(party, _) = books.parties.directory().resolve(key.party) else { continue };
        let (place, _) = books.parties.row(party);
        let kind = books.parties.holder(place).kind();
        if kind != sys_frm::FIRM.name && kind != sys_frm::SMALL_FIRM.name {
            continue;
        }
        let Missing::Present(country) = w.country_of_party(party) else {
            return Outcome::Fail(format!("firm {} sited in no country", party.get()));
        };
        let Some(Ok(grace)) = graces.get(usize::from(country.get())).map(|g| u32::try_from(*g)) else {
            return Outcome::Fail(format!("no insolvency grace within a day's count for country {}", country.get()));
        };
        let Some(ends) = since.get().checked_add(grace) else { continue };
        if (ends..=today.get()).any(|d| w.any_business(phx_id::Day::new(d))) {
            return Outcome::Fail(format!(
                "party {} in arrears on line {} since day {}, past its grace, and not ended",
                party.get(),
                key.line.get(),
                since.get()
            ));
        }
    }
    Outcome::NotYet("firms close from S1.05, and their endings per industry are read over a year of the run")
}

pub const LC_1_45: super::Check = live_check! {
    id: "LC-1-45",
    title: "firms end every year in every industry, by closure and by default, each with an estate or a successor; \
            no firm keeps failing payments past its grace without ending",
    from_step: "S1.03",
    check: firms_end,
};
