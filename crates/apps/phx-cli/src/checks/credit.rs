//! Credit: each bank's loan book written as loans, declined applications counted by bank, and every loan's
//! disbursement a deposit created for its borrower.

use phx_world::Inspector;

use super::Outcome;
use crate::live_check;

/// Each loan written is on its bank's book at its principal the day it is written, so a book grows by the loans it
/// makes and by nothing else a loan is written by.
fn books_of_loans(w: Inspector<'_>) -> Outcome {
    let days = w.credit_days();
    if days.iter().all(|(_, d)| d.written.is_empty()) {
        return Outcome::NotYet("banks lend once a borrower they will lend to applies");
    }
    for (day, d) in days {
        if let Some(l) = d.written.iter().find(|l| l.booked != l.principal) {
            return Outcome::Fail(format!(
                "day {}: bank {} booked {} for a loan of {}",
                day.get(),
                l.bank.get(),
                l.booked,
                l.principal
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_24: super::Check = live_check! {
    id: "LC-1-24",
    title: "BNK.11: each bank's loan book equals the sum of its loan lines, and its change reconciles",
    from_step: "S1.09",
    check: books_of_loans,
};

/// Every declined application is counted against the bank that declined it, and the day's declines are theirs.
fn declines_by_bank(w: Inspector<'_>) -> Outcome {
    let days = w.credit_days();
    if days.iter().all(|(_, d)| d.applications == 0) {
        return Outcome::NotYet("applications arrive once a borrower's loan falls due within its lead");
    }
    match days.iter().find(|(_, d)| d.declined_by_bank.iter().map(|(_, n)| n).sum::<u64>() != d.declines) {
        Some((day, d)) => Outcome::Fail(format!("day {}: {} declines, not all counted by bank", day.get(), d.declines)),
        None => Outcome::Pass,
    }
}

pub const LC_1_25: super::Check = live_check! {
    id: "LC-1-25",
    title: "Declined applications are visible and counted per bank",
    from_step: "S1.09",
    check: declines_by_bank,
};

/// Every loan's disbursement credited its borrower's account with the principal, a deposit its bank created.
fn loans_create_deposits(w: Inspector<'_>) -> Outcome {
    let days = w.credit_days();
    if days.iter().all(|(_, d)| d.written.is_empty()) {
        return Outcome::NotYet("banks lend once a borrower they will lend to applies");
    }
    for (day, d) in days {
        if let Some(l) = d.written.iter().find(|l| l.after - l.before != l.principal) {
            return Outcome::Fail(format!(
                "day {}: a loan of {} to {} moved its account by {}",
                day.get(),
                l.principal,
                l.borrower.get(),
                l.after - l.before
            ));
        }
    }
    Outcome::Pass
}

pub const LC_1_26: super::Check = live_check! {
    id: "LC-1-26",
    title: "MON.6 in practice: every new loan's disbursement created a deposit at the lender, and money-stock \
            changes reconcile to issuers' transactions",
    from_step: "S1.09",
    check: loans_create_deposits,
};
