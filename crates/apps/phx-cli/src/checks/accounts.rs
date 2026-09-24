use phx_num::Missing;
use phx_world::Inspector;

use super::{Check, Outcome};
use crate::live_check;

/// The first finding of a family over the run, if any.
fn found(w: Inspector<'_>, family: &str) -> Option<String> {
    w.findings()
        .iter()
        .find(|f| f.family == family)
        .map(|f| format!("{} on day {}: {}", f.family, f.day.get(), f.detail))
}

/// No finding of the equity or period families over the run, and, at its end, every equity account equal to its
/// party's assets less liabilities at their carrying values, read in full.
fn accounts_clean(w: Inspector<'_>) -> Outcome {
    let accounts = w.accounts();
    if accounts.equity.is_empty() {
        return Outcome::Fail("no party keeps an equity account".to_owned());
    }
    for family in [phx_acct::audit::EQUITY.name, phx_acct::audit::PERIODS.name] {
        if let Some(why) = found(w, family) {
            return Outcome::Fail(why);
        }
    }
    for party in accounts.equity.parties() {
        let Missing::Present(account) = accounts.equity.of(party) else {
            return Outcome::Fail(format!("party {} listed without its account", party.get()));
        };
        match accounts.net_assets(w.books(), party) {
            Err(why) => return Outcome::Fail(format!("unreadable at the run's end: {why}")),
            Ok(net) if net != i128::from(account.balance()) => {
                return Outcome::Fail(format!(
                    "party {}: assets less liabilities {net} against an equity account of {} at the run's end",
                    party.get(),
                    account.balance()
                ));
            }
            Ok(_) => {}
        }
    }
    Outcome::Pass
}

/// No finding of the claims family over the run, and what the world counts receivable at its end what it counts
/// payable.
fn claims_agree(w: Inspector<'_>) -> Outcome {
    if let Some(why) = found(w, phx_acct::audit::CLAIMS.name) {
        return Outcome::Fail(why);
    }
    let (receivable, payable) = w.accounts().claims.totals();
    if receivable != payable {
        return Outcome::Fail(format!("{receivable} receivable against {payable} payable at the run's end"));
    }
    Outcome::Pass
}

pub const LC_0_33: Check = live_check! {
    id: "LC-0-33",
    title: "Accounts is clean for every party with an equity account",
    from_step: "S0.19",
    check: accounts_clean,
};

pub const LC_0_34: Check = live_check! {
    id: "LC-0-34",
    title: "Receivables equal payables across the world",
    from_step: "S0.19",
    check: claims_agree,
};
