//! The sovereign's debt at the opening: the bills the banks hold, the asset that closes each bank's books at its
//! country's capital ratio once its deposits, loans and reserves are written.

use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::{Adjustment, BALANCES, Contribution, Opening, OpeningPhase};
use phx_id::{CountryId, PartyId};
use phx_ledger::algebra::{Leg, Repayment, Schedule, Side};
use phx_ledger::books::{self, Books};
use phx_ledger::instruction::{Effect, ReasonDecl, ReasonId};
use phx_ledger::opening::{currency, derived, key, open_row, plain_terms, whole, write};
use phx_ledger::rows::BALANCE;
use phx_macros::clause;
use phx_num::Money;
use phx_num::{Missing, violation};

use crate::consts::PERCENT;
use crate::{BILL, WEEKS};

const BANKS: &str = "BNK.banks";
const TREASURIES: &str = "CB.treasury";
/// The central bank's claim on the treasury, which stands for the debt it holds.
const CLAIM: &str = "central bank credit to the treasury";

/// The opening's bills written under the sovereign's own reason.
pub(crate) const OPENED: ReasonDecl =
    ReasonDecl { name: "SOV opening", order: 0, paid: Effect::Equity, received: Effect::Equity, held: Missing::Absent };

fn reason(b: &Books) -> ReasonId {
    b.ledger.reasons.named(OPENED.name)
}

/// A bank's assets and liabilities as its written rows and holdings carry them.
fn sides(b: &Books, bank: PartyId) -> (f64, f64) {
    let (place, slot) = b.parties.row(bank);
    let arenas = b.parties.holder(place);
    let (mut assets, mut owed) = (0_i64, 0_i64);
    for r in phx_ledger::rows::iter(arenas, slot) {
        let Missing::Present(balance) = r.optional.balance else { continue };
        match r.side() {
            Side::Asset => assets += balance,
            Side::Liability => owed -= balance,
        }
    }
    for (_, cost) in phx_ledger::holding::bases(arenas, slot) {
        assets += cost;
    }
    (phx_rand::float::from_i64(assets), phx_rand::float::from_i64(owed))
}

/// The bills a bank holds at the opening: what its books need beyond the assets written for its equity to be the
/// country's capital ratio of its assets, none where they already carry more.
#[clause("GEN.4", "GEN.15")]
#[must_use]
pub fn bills_held(assets: f64, owed: f64, ratio: f64) -> f64 {
    let need = owed / (1.0 - ratio) - assets;
    if need > 0.0 { need } else { 0.0 }
}

/// The bills outstanding at the opening, each bank's spread over the weeks a bill runs, one line maturing each week
/// from the first after the opening, so the treasury rolls a week's part over at each auction as the bills sold
/// before the snapshot would have it. The public debt the banks and the central bank do not hold waits for the
/// holders of later stages, and the difference from the derived debt is reported.
#[clause("GEN.4", "GEN.15", "SOV.1")]
#[derive(Debug)]
pub struct Bills;

impl Contribution for Bills {
    fn name(&self) -> &'static str {
        "sovereign bills"
    }
    fn phase(&self) -> OpeningPhase {
        BALANCES
    }
    fn reads(&self) -> &'static [&'static str] {
        &[BANKS, TREASURIES, "BNK.balances", "CB.balances"]
    }
    fn writes(&self) -> &'static [&'static str] {
        &["SOV.bills"]
    }
    fn drawn(&self) -> &'static [&'static str] {
        &[]
    }
    fn derived(&self) -> &'static [&'static str] {
        &["SOV.bills"]
    }

    fn contribute(&self, opening: &mut Opening<'_>) {
        let (countries, register, calendar, date) =
            (opening.countries, opening.register, opening.calendar, opening.date);
        let Some(weeks) = register.count(WEEKS.id).ok().and_then(|w| u16::try_from(w).ok()) else {
            violation!(clause = "SOV.1", "a bill's weeks undeclared or beyond a period");
        };
        let (b, report) = books::split(opening);
        let reason = reason(b);
        let kind = b.ledger.lines.kind_index(BILL.name);
        for c in countries {
            let Ok(face) = crate::face(register, c.id) else {
                violation!(clause = "SOV.1", "a bill's face undeclared", country = c.id.get());
            };
            let ratio = derived(c, "GEN.bank_capital_ratio") / PERCENT;
            let debt = derived(c, "GEN.public_debt") / PERCENT * c.gdp;
            let Some(treasury) = b.drawn.get(&key(TREASURIES, c.id)).and_then(|v| v.first()).map(|(p, _)| *p) else {
                violation!(clause = "GEN.3", "the bills opening before the treasury", country = c.id.get());
            };
            let Some(banks) = b.drawn.get(&key(BANKS, c.id)).cloned() else {
                violation!(clause = "GEN.3", "the bills opening before the banks", country = c.id.get());
            };
            let ccy = currency(c.id);
            let lines: Vec<_> = (1..=weeks)
                .map(|w| {
                    let (terms, maturity) = bill_terms(b, (calendar, date), c.id, w, face);
                    b.ledger.lines.open(kind, terms, Missing::Present((maturity, 1)))
                })
                .collect();
            let Ok(n) = i64::try_from(lines.len()) else {
                phx_num::capacity_exceeded!("bill lines", i64::MAX, lines.len());
            };
            // Each bank's contracts, spread over the weeks, the first weeks taking what does not divide evenly.
            let mut holdings: Vec<(PartyId, i64)> = Vec::new();
            for (bank, _) in &banks {
                let (assets, owed) = sides(b, *bank);
                let contracts = whole(bills_held(assets, owed, ratio) / phx_rand::float::from_i64(face));
                if contracts > 0 {
                    holdings.push((*bank, contracts));
                }
            }
            let mut held = 0_i128;
            for (i, line) in (0_i64..).zip(&lines) {
                let counts: Vec<(PartyId, u32)> = holdings
                    .iter()
                    .filter_map(|(bank, contracts)| {
                        let count = contracts / n + i64::from(i < contracts % n);
                        let Ok(k) = u32::try_from(count) else {
                            phx_num::capacity_exceeded!("a bank's bills of one maturity", u32::MAX, count);
                        };
                        (k > 0).then_some((*bank, k))
                    })
                    .collect();
                let total: u32 = counts.iter().map(|(_, k)| *k).sum();
                if total == 0 {
                    continue;
                }
                let id = u64::from(line.get());
                let mut rows: Vec<_> =
                    counts.iter().map(|(bank, k)| open_row(register, *bank, *line, Side::Asset, *k, BALANCE)).collect();
                rows.push(open_row(register, treasury, *line, Side::Liability, total, BALANCE));
                b.open(reason, rows, id, report);
                let mut writes: Vec<_> = counts
                    .iter()
                    .map(|(bank, k)| write(*bank, *line, Side::Asset, i64::from(*k) * face, ccy, id))
                    .collect();
                writes.push(write(treasury, *line, Side::Liability, -i64::from(total) * face, ccy, id));
                b.open(reason, writes, id, report);
                held += i128::from(total) * i128::from(face);
            }
            report.adjustments.push(Adjustment {
                what: format!(
                    "country {}: the sovereign's debt as the banks' bills and the central bank's claim hold it",
                    c.id.get()
                ),
                drawn: i128::from(whole(debt)),
                set: held + claim_of(b, treasury),
            });
        }
    }
}

/// What the treasury owes the central bank on its claim, which the central bank's opening always writes.
fn claim_of(b: &Books, treasury: PartyId) -> i128 {
    let Some((line, _)) = b.row_on(treasury, CLAIM) else {
        violation!(clause = "GEN.4", "a treasury with no claim on it", treasury = treasury.get());
    };
    let (place, slot) = b.parties.row(treasury);
    let Some(Missing::Present(v)) =
        phx_ledger::rows::find(b.parties.holder(place), slot, line, Side::Liability).map(|r| r.optional.balance)
    else {
        violation!(clause = "GEN.4", "a claim on the treasury with no balance", treasury = treasury.get());
    };
    -i128::from(v)
}

/// The terms of a bill maturing `w` weeks after the opening, and the day it matures.
fn bill_terms(
    b: &mut Books,
    (calendar, date): (&phx_core::calendar::Calendar, phx_id::Date),
    country: CountryId,
    w: u16,
    face: i64,
) -> (phx_ledger::terms::TermsId, phx_id::Day) {
    let Some(period) = Period::weeks(w) else {
        violation!(clause = "SOV.1", "a bill's weeks beyond a period", weeks = w);
    };
    let ccy = currency(country);
    let dates = ScheduleDates {
        anchor: date,
        period,
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country,
    };
    let terms = plain_terms(
        ccy,
        vec![Leg::Principal { amount: Money::new(face, ccy), repayment: Repayment::Bullet }],
        Schedule { dates, count: Missing::Present(1) },
    );
    let maturity = dates.nth(calendar, 1);
    (b.ledger.terms.intern(terms), maturity)
}

#[cfg(test)]
mod tests {
    use super::bills_held;

    #[test]
    fn bills_close_a_bank_at_its_capital_ratio() {
        // Deposits of 900 at a ratio of a tenth need assets of 1 000; 700 are written, so 300 in bills.
        assert!((bills_held(700.0, 900.0, 0.1) - 300.0).abs() < 1e-9);
        assert!(bills_held(1_200.0, 900.0, 0.1).abs() < 1e-9, "a bank already carrying more holds none");
    }
}
