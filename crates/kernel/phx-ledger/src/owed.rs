//! What a party's contracts owe it: the dues its rows are paid, read from their terms before they fall.

use phx_core::calendar::Calendar;
use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, Money, violation};
use phx_store::Backing;

use crate::algebra::{Amount, DueBuf, DueState, Leg, Side, due_at};
use crate::books::Books;
use crate::cleared::{per_contract, times};

impl<B: Backing> Books<B> {
    /// The money a party's contracts owe it on the dates they fall from `from` to before `to`: on each row it holds on
    /// the side that is paid, each date's dues on the row's contracts and its balance as they stand. What repays a
    /// balance is the party's own money coming back, not income, and is left out.
    #[clause("HH.2", "REG.5")]
    #[must_use]
    pub fn owed_between(&self, party: PartyId, (from, to): (Day, Day), calendar: &Calendar) -> i128 {
        let (place, slot) = self.parties.row(party);
        let mut owed = 0_i128;
        let mut buf = DueBuf::default();
        for row in crate::rows::iter(self.parties.holder(place), slot) {
            let line = row.row.line;
            if row.side() != Side::Asset || !self.ledger.lines.dated(line) {
                continue;
            }
            let terms = self.ledger.terms.get(self.ledger.lines.terms(line));
            let Missing::Present(balance) = row.optional.balance else {
                violation!(clause = "REG.8", "a contract row with no balance to reckon its dues on", line = line.get());
            };
            let state = DueState {
                calendar,
                outstanding: Money::new(balance, terms.ccy),
                elected: &|_, _| false,
                occurred: &|_, _| false,
                in_state_since: &|_, _| Missing::Absent,
            };
            let mut k = self.ledger.lines.fallen(line) + 1;
            loop {
                if let Missing::Present(n) = terms.schedule.count
                    && k > n
                {
                    break;
                }
                let day = terms.schedule.day(calendar, k);
                if day >= to {
                    break;
                }
                if day >= from {
                    due_at(terms, Some(k), day, &state, &mut buf);
                    owed += income_of(&terms.legs, &buf, row.row.count);
                }
                k += 1;
            }
        }
        owed
    }
}

/// A date's dues on a row of `count` contracts, beyond what repays a balance: each per-contract leg's due on every
/// contract, and every other leg's once.
fn income_of(legs: &[Leg], dues: &DueBuf, count: u32) -> i128 {
    dues.iter()
        .filter_map(|d| {
            let leg = legs.get(usize::from(d.leg))?;
            let Amount::Money(m) = d.amount else { return None };
            match leg {
                Leg::Principal { .. } | Leg::Amortising => None,
                _ if per_contract(leg) => Some(i128::from(times(m.amt(), count))),
                _ => Some(i128::from(m.amt())),
            }
        })
        .sum()
}
