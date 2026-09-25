use phx_core::{BooksAudit, Gap};
use phx_ledger::audit::{FLOWS, UNITS};
use phx_num::Missing;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// Every instrument or line of the books through one of the ledger's checks: the first gap fails it.
fn every(n: usize, check: impl Fn(usize) -> Vec<Gap>) -> Outcome {
    match (0..n).flat_map(check).next() {
        Some(g) => Outcome::Fail(g.detail),
        None => Outcome::Pass,
    }
}

/// No finding of a family over the run, which must have checked something.
fn quiet(w: Inspector<'_>, family: &'static str, checked: u64) -> Outcome {
    if checked == 0 {
        return Outcome::Fail(format!("{family} checked nothing over the run"));
    }
    match w.findings().iter().find(|f| f.family == family) {
        Some(f) => Outcome::Fail(format!("{family} on day {}: {}", f.day.get(), f.detail)),
        None => Outcome::Pass,
    }
}

fn holdings(w: Inspector<'_>) -> Outcome {
    let books: &dyn BooksAudit = w.books();
    if books.instruments() == 0 {
        return Outcome::Fail("the books issue no instrument".to_owned());
    }
    every(books.instruments(), |i| books.ownership(i))
}

fn sides(w: Inspector<'_>) -> Outcome {
    let books: &dyn BooksAudit = w.books();
    every(books.lines(), |i| books.contracts(i))
}

fn money(w: Inspector<'_>) -> Outcome {
    let books: &dyn BooksAudit = w.books();
    let lines = &w.books().ledger.lines;
    if !lines.ids().any(|l| lines.is_money(l)) {
        return Outcome::Fail("the books keep no money".to_owned());
    }
    match books.money(0..books.lines()).into_iter().next() {
        Some(g) => Outcome::Fail(g.detail),
        None => Outcome::Pass,
    }
}

fn flows(w: Inspector<'_>) -> Outcome {
    let instructions = w.settlements().iter().map(|s| s.dues.payments).sum();
    quiet(w, FLOWS.name, instructions)
}

fn units(w: Inspector<'_>) -> Outcome {
    let settled = w.settlements().iter().map(|s| s.dues.settled).sum();
    quiet(w, UNITS.name, settled)
}

/// Every fail on a contract row before the run's last business day has been turned into arrears on that row by the
/// contract process at the next business day's 2d; the last day's wait for a 2d the run did not reach.
fn fails(w: Inspector<'_>) -> Outcome {
    let Some(last) = w.settlements().iter().rev().find(|s| w.any_business(s.day)).map(|s| s.day) else {
        return Outcome::Fail("no business day was run".to_owned());
    };
    let arrears = w.books().ledger.arrears();
    for s in w.settlements().iter().filter(|s| s.day < last) {
        for f in &s.fails {
            let Missing::Present(row) = f.row else { continue };
            // A party that ended since, or whose members took the row elsewhere, holds no row to read.
            let phx_core::Resolved::Live(party, _) = w.books().parties.directory().resolve(f.party) else { continue };
            if !w.books().rows_of(party).contains(&(row.line, row.side)) {
                continue;
            }
            if arrears.of(row.line, row.side, party).is_none() {
                return Outcome::Fail(format!(
                    "a fail on line {} on day {} ({:?}) is not in arrears",
                    row.line.get(),
                    s.day.get(),
                    f.cause
                ));
            }
        }
    }
    Outcome::Pass
}

/// Each day's reserves moved by the net of its settled payments between banks, and every account by the net of the
/// payments applied to it.
fn reserves(w: Inspector<'_>) -> Outcome {
    let days = w.settlements();
    if days.iter().all(|s| s.dues.settled == 0) {
        return Outcome::Fail("no payment settled over the run".to_owned());
    }
    match days.iter().find(|s| s.dues.reserves_missed > 0 || s.dues.nets_missed > 0) {
        Some(s) => Outcome::Fail(format!(
            "on day {}, {} banks' reserves and {} accounts moved other than by their nets",
            s.day.get(),
            s.dues.reserves_missed,
            s.dues.nets_missed
        )),
        None => Outcome::Pass,
    }
}

/// Each day's settled set, recomputed on the books as stage 7 found them, is sound and maximal: every settled party
/// could pay, and every failed payer was short.
fn greatest(w: Inspector<'_>) -> Outcome {
    let days = w.settlements();
    if days.iter().all(|s| s.dues.payments == 0) {
        return Outcome::Fail("no payment fell due over the run".to_owned());
    }
    match days.iter().find(|s| s.dues.unsound > 0 || s.dues.not_maximal > 0) {
        Some(s) => Outcome::Fail(format!(
            "on day {}, {} parties settled beyond their means and {} failed though they could pay",
            s.day.get(),
            s.dues.unsound,
            s.dues.not_maximal
        )),
        None => Outcome::Pass,
    }
}

/// The levies' amounts are per member times the count; nothing levies before the first tax.
fn levies(_: Inspector<'_>) -> Outcome {
    Outcome::NotYet("the first levy arrives with the taxes (S1.11)")
}

/// The sampled holders' runs held against their rows every day.
fn runs(w: Inspector<'_>) -> Outcome {
    let days = w.settlements();
    if days.iter().all(|s| s.dues.runs_read == 0) {
        return Outcome::Fail("no holder's run was read in full over the run".to_owned());
    }
    match days.iter().find(|s| s.dues.runs_broken > 0) {
        Some(s) => Outcome::Fail(format!(
            "on day {}, {} of {} sampled holders' runs did not hold",
            s.day.get(),
            s.dues.runs_broken,
            s.dues.runs_read
        )),
        None => Outcome::Pass,
    }
}

/// A settlement published for every day of the run, in order.
fn published(w: Inspector<'_>) -> Outcome {
    let mut day = w.day_zero();
    for s in w.settlements() {
        day = day.succ();
        if s.day != day {
            return Outcome::Fail(format!("no settlement published for day {}", day.get()));
        }
    }
    if day == w.today() { Outcome::Pass } else { Outcome::Fail(format!("the last settlement is of day {}", day.get())) }
}

pub const LC_0_16: Check = live_check! {
    id: "LC-0-16",
    title: "For every issued instrument, holdings sum to the issued amount",
    from_step: "S0.14",
    check: holdings,
};

pub const LC_0_17: Check = live_check! {
    id: "LC-0-17",
    title: "Every line's two sides hold equal counts, and each side it lists equals its holders' rows",
    from_step: "S0.14",
    check: sides,
};

pub const LC_0_18: Check = live_check! {
    id: "LC-0-18",
    title: "Per issuer and currency, the balances held equal its money liability; the notes held equal the notes issued",
    from_step: "S0.15",
    check: money,
};

pub const LC_0_19: Check = live_check! {
    id: "LC-0-19",
    title: "Every instruction's paired legs sum to nothing per denomination; transformations and opening writes name their sources",
    from_step: "S0.15",
    check: flows,
};

pub const LC_0_20: Check = live_check! {
    id: "LC-0-20",
    title: "Per holder and asset per day, what it held plus what came in less what went out is what it holds",
    from_step: "S0.15",
    check: units,
};

pub const LC_0_21: Check = live_check! {
    id: "LC-0-21",
    title: "Every fail has a cause, and its line kind's contract process recorded it at 2d",
    from_step: "S0.15",
    check: fails,
};

pub const LC_0_22: Check = live_check! {
    id: "LC-0-22",
    title: "Gross and net settlement, fails by cause and the closing ring are published every day",
    from_step: "S0.15",
    check: published,
};

pub const LC_0_27: Check = live_check! {
    id: "LC-0-27",
    title: "Every bank's reserve movement equals the net of its customers' applied payments",
    from_step: "S0.17",
    check: reserves,
};

pub const LC_0_28: Check = live_check! {
    id: "LC-0-28",
    title: "The settled set is sound and maximal, recomputed on the state at the start of stage 7",
    from_step: "S0.17",
    check: greatest,
};

pub const LC_0_29: Check = live_check! {
    id: "LC-0-29",
    title: "Sampled levy amounts equal per member times count under their conventions",
    from_step: "S0.17",
    check: levies,
};

pub const LC_0_61: Check = live_check! {
    id: "LC-0-61",
    title: "On sampled holders, every row due was read in the holder's run, and no head was later than its segment's earliest due day",
    from_step: "S0.17",
    check: runs,
};
