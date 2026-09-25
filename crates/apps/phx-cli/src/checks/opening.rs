use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// No family found anything on the first day after the opening.
fn first_day(w: Inspector<'_>) -> Outcome {
    let one = w.day_zero().succ();
    if !w.closes().iter().any(|c| c.day == one) {
        return Outcome::Fail("the audit did not close day one".to_owned());
    }
    match w.findings().iter().find(|f| f.day == one) {
        Some(f) => Outcome::Fail(format!("{} on day one: {}", f.family, f.detail)),
        None => Outcome::Pass,
    }
}

/// Every party the opening began is named by an opening write, and every distribution it read names its source.
fn reported(w: Inspector<'_>) -> Outcome {
    let report = w.opening();
    if report.writes.is_empty() {
        return Outcome::Fail("the opening reports no write".to_owned());
    }
    if let Some((name, _)) = report.distributions.iter().find(|(_, source)| source.is_empty()) {
        return Outcome::Fail(format!("the distribution {name} names no source"));
    }
    let parties = &w.books().parties;
    let opened = w.day_zero();
    for kind in parties.kinds() {
        let t = parties.table(parties.place(kind));
        // Parties begun during the run, as estates are, were begun by no opening.
        let named = |p: phx_id::PartyId| report.writes.iter().any(|x| x.party == p || x.counter == p);
        if let Some(p) = t.slots().filter(|s| t.created(*s) <= opened).map(|s| t.party(s)).find(|p| !named(*p)) {
            return Outcome::Fail(format!("the {kind} {} is named by no opening write", p.get()));
        }
    }
    Outcome::Pass
}

/// Every line still due falls due on a business day of its schedule's country.
fn business_days(w: Inspector<'_>) -> Outcome {
    let ledger = &w.books().ledger;
    for line in ledger.lines.ids().filter(|l| !ledger.lines.done(*l)) {
        let country = ledger.terms.get(ledger.lines.terms(line)).schedule.dates.country;
        let due = ledger.lines.next_due(line);
        if !w.is_business(country, due) {
            return Outcome::Fail(format!("line {} falls due on day {}, no business day", line.get(), due.get()));
        }
    }
    Outcome::Pass
}

/// Payments fall due and are processed on every business day, and every fail is of a contract's due. That some
/// settle every day waits for firms that earn: until then their dues drain their deposits.
fn liveness(w: Inspector<'_>) -> Outcome {
    for s in w.settlements().iter().filter(|s| w.any_business(s.day)) {
        if s.dues.payments == 0 {
            return Outcome::Fail(format!("no payment fell due on business day {}", s.day.get()));
        }
        if s.rowless > 0 {
            return Outcome::Fail(format!("{} fails on day {} of no contract's due", s.rowless, s.day.get()));
        }
    }
    Outcome::Pass
}

pub const LC_0_23: Check = live_check! {
    id: "LC-0-23",
    title: "Day one passes every family",
    from_step: "S0.16",
    check: first_day,
};

pub const LC_0_24: Check = live_check! {
    id: "LC-0-24",
    title: "The GEN report lists every opening write with party, amount and identity, and each distribution with its source",
    from_step: "S0.16",
    check: reported,
};

pub const LC_0_25: Check = live_check! {
    id: "LC-0-25",
    title: "Every opening contract's payments fall on business days by its convention",
    from_step: "S0.16",
    check: business_days,
};

pub const LC_0_26: Check = live_check! {
    id: "LC-0-26",
    title: "Payments fall due and are processed every business day, and every fail has a cause",
    from_step: "S0.16",
    check: liveness,
};
