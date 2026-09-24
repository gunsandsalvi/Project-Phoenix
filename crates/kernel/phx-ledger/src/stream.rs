use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_id::{Day, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Ccy, Missing, Money, violation};
use phx_store::Backing;

use crate::algebra::{Amount, DueBuf, DueState, Leg, Side, due_at};
use crate::apply::{Holders, Located};
use crate::books::Books;
use crate::due::DueLines;
use crate::dues::{Found, row_leg};
use crate::instruction::{AccountRef, LegKind, LegRec};
use crate::pending::Closed;
use crate::rows::RowView;
use crate::runs;

/// One row's dues on a line due today, reckoned on the row of the side its line is reckoned on and paid by the line's
/// liability side to its asset side; `principal` is the part that repays the claim, moving the contract's rows with
/// the money. A payment is named by its line and the holder of the row it was reckoned on, since a holder keeps one
/// row on a side of a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payment {
    pub line: LineId,
    pub payer: PartyId,
    pub payee: PartyId,
    pub amount: i64,
    pub principal: i64,
    pub ccy: Ccy,
    pub order: u8,
    pub reckoned_on: PartyId,
}

impl Payment {
    pub fn key(&self) -> (LineId, PartyId) {
        (self.line, self.reckoned_on)
    }
}

/// A party's account at the start of stage 7 and what the day's standing payments do to it: what it may draw on (its
/// balance less what is pending, with any facility its terms grant), and the debits and credits on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Record {
    pub account: LineId,
    pub funds: i128,
    pub debit: i128,
    pub credit: i128,
}

impl Record {
    /// What the account would hold past its floor if every standing payment settled.
    #[must_use]
    pub fn standing(&self) -> i128 {
        self.funds + self.credit - self.debit
    }
}

/// Stage 7a's result: one record per party with an account the day's payments touch, the holders whose runs were
/// scanned, and the day's counts: heads read, rows of scanned segments read, the rows among them due today, the
/// payments they make and those held pending. Nothing is kept per payment.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DayRecords {
    pub records: BTreeMap<PartyId, Record>,
    pub scanned: Vec<(u16, Slot)>,
    pub heads_read: u64,
    pub rows_scanned: u64,
    pub rows_due: u64,
    pub payments: u64,
    pub pending: u64,
    pub gross: i128,
}

/// The issuers a payment's money passes through: the parties on the owing side of the money lines it moves.
#[must_use]
pub fn issuers(legs: &[LegRec]) -> Vec<PartyId> {
    legs.iter()
        .filter(|l| matches!(l.kind, LegKind::Money))
        .filter(|l| matches!(l.account, AccountRef::Line { side: Side::Liability, .. }))
        .map(|l| l.party)
        .collect()
}

fn count(n: usize) -> u64 {
    phx_rand::float::len_u64(n)
}

impl<B: Backing> Books<B> {
    /// A holder's due rows, whether or not its head was read today: the rows of its segment on lines due today.
    pub(crate) fn due_rows_of(&self, party: PartyId, due: &DueLines) -> Vec<RowView> {
        let Located::Live { table, slot, .. } = self.parties.locate(party) else { return Vec::new() };
        let arenas = self.parties.holder(table);
        runs::segment(arenas, slot, arenas.run_head(slot)).filter(|r| due.is_due(r.row.line)).collect()
    }

    /// The payment a claimant's row makes due today, if any: its dues summed, reckoned on its balance.
    pub(crate) fn payment(
        &self,
        holder: PartyId,
        row: &RowView,
        day: Day,
        calendar: &Calendar,
        found: &mut Found,
    ) -> Option<Payment> {
        let line = row.row.line;
        let (reckoned, counter) = self.reckoning(line, holder, row.side(), found);
        if row.side() != reckoned {
            return None;
        }
        let terms = self.ledger.terms.get(self.ledger.lines.terms(line));
        if let Missing::Present(procedure) = terms.stay
            && self.ledger.procedures.contains(&procedure)
        {
            return None;
        }
        let Missing::Present(balance) = row.optional.balance else {
            violation!(clause = "REG.8", "a contract row with no balance to reckon its dues on", line = line.get());
        };
        let outstanding = match reckoned {
            Side::Asset => balance,
            Side::Liability => -balance,
        };
        let state = DueState {
            calendar,
            outstanding: Money::new(outstanding, terms.ccy),
            elected: &|_, _| false,
            occurred: &|_, _| false,
            in_state_since: &|_, _| Missing::Absent,
        };
        let mut buf = DueBuf::default();
        due_at(terms, Some(self.ledger.lines.fallen(line)), day, &state, &mut buf);
        let (mut amount, mut principal) = (0_i64, 0_i64);
        for d in buf.iter() {
            let Amount::Money(m) = d.amount else {
                violation!(
                    clause = "REG.5",
                    "a due in other than money, which no settlement pays yet",
                    line = line.get()
                );
            };
            amount += m.amt();
            if matches!(terms.legs.get(usize::from(d.leg)), Some(Leg::Principal { .. })) {
                principal += m.amt();
            }
        }
        if amount == 0 {
            return None;
        }
        let (from, to) = match reckoned {
            Side::Asset => (counter, holder),
            Side::Liability => (holder, counter),
        };
        Some(Payment {
            line,
            payer: from,
            payee: to,
            amount,
            principal,
            ccy: terms.ccy,
            order: terms.payment_order.0,
            reckoned_on: holder,
        })
    }

    /// Every payment a party takes part in today, in its payment order: its due rows in its run's order, by the
    /// order of their terms; each claimant's row is its own payment, and each row owing a line is every payment the
    /// line's claimants make due.
    #[clause("REP.8")]
    pub(crate) fn payments_of(
        &self,
        party: PartyId,
        due: &DueLines,
        day: Day,
        calendar: &Calendar,
        found: &mut Found,
    ) -> Vec<Payment> {
        let mut rows = self.due_rows_of(party, due);
        rows.sort_by_key(|r| self.ledger.terms.get(self.ledger.lines.terms(r.row.line)).payment_order.0);
        let mut out = Vec::new();
        for r in rows {
            let (reckoned, _) = self.reckoning(r.row.line, party, r.side(), found);
            if r.side() == reckoned {
                out.extend(self.payment(party, &r, day, calendar, found));
                continue;
            }
            for other in self.line_holders_but(r.row.line, party) {
                if let Some(row) = self.row_on_side(other, r.row.line, reckoned) {
                    out.extend(self.payment(other, &row, day, calendar, found));
                }
            }
        }
        out
    }

    /// What a payment does, leg by leg: the money from the payer's means of payment to the payee's, through deposits
    /// and, between banks, reserves; and the principal repaid off the contract's rows.
    pub(crate) fn effects(&self, p: &Payment, found: &mut Found) -> Vec<LegRec> {
        let mut legs = self.pay(p.payer, p.payee, p.amount, p.ccy, found);
        if p.principal != 0 {
            legs.push(row_leg(p.payee, p.line, Side::Asset, -p.principal, p.ccy));
            legs.push(row_leg(p.payer, p.line, Side::Liability, p.principal, p.ccy));
        }
        legs
    }

    /// A party's account and what it may draw on at the start of stage 7: its balance less what is pending, and the
    /// facility its account's terms grant over the row's members.
    pub(crate) fn record_of(&self, party: PartyId, account: LineId) -> Record {
        let Located::Live { table, slot, .. } = self.parties.locate(party) else {
            violation!(clause = "SET.7", "a payment touching a party that has ended", party = party.get());
        };
        let facility = match self.ledger.terms.get(self.ledger.lines.terms(account)).facility {
            Missing::Present(f) => f.limit.amt(),
            Missing::Absent => 0,
        };
        let positions = self.parties.holder(table);
        let weight = i128::from(positions.weight(slot));
        Record { account, funds: positions.per_member_funds(slot, account, facility) * weight, debit: 0, credit: 0 }
    }

    /// Adds or takes away a payment's effects on the accounts it touches: a leg drawing on an account is a debit, a
    /// leg paying into one a credit; an issuer's side of its own money is no account and has no record.
    pub(crate) fn book(&self, records: &mut BTreeMap<PartyId, Record>, legs: &[LegRec], sign: i128) -> Vec<PartyId> {
        let mut touched = Vec::new();
        for leg in legs.iter().filter(|l| matches!(l.kind, LegKind::Money)) {
            let AccountRef::Line { line, side: Side::Asset } = leg.account else { continue };
            let rec = records.entry(leg.party).or_insert_with(|| self.record_of(leg.party, line));
            if rec.account != line {
                violation!(clause = "MON.5", "a party paying from two accounts in one day", party = leg.party.get());
            }
            let q = i128::from(leg.qty) * sign;
            if q < 0 {
                rec.debit -= q;
            } else {
                rec.credit += q;
            }
            touched.push(leg.party);
        }
        touched
    }

    /// Stage 7a: one stream over every holder table's run heads, holder-major. A holder whose head has not come costs
    /// that one read; on its head's day its segment's rows on lines due today are read, and each claimant's row adds
    /// its payment's debits and credits to the records of the accounts it touches.
    #[clause("MON.5", "REP.8", "SET.6")]
    pub(crate) fn stream(
        &self,
        due: &DueLines,
        day: Day,
        calendar: &Calendar,
        closed: &Closed,
        found: &mut Found,
    ) -> DayRecords {
        let mut out = DayRecords::default();
        for place in self.parties.places() {
            let table = self.parties.holder(place);
            for slot in phx_store::table::live_in(table.live_words()) {
                out.heads_read += 1;
                let Some((rows, read)) = runs::due_rows(table, slot, day, due) else { continue };
                out.scanned.push((place, slot));
                out.rows_scanned += read;
                out.rows_due += count(rows.len());
                let holder = table.party(slot);
                for row in &rows {
                    let Some(p) = self.payment(holder, row, day, calendar, found) else { continue };
                    let legs = self.effects(&p, found);
                    out.payments += 1;
                    if closed.holds(p.payer, &issuers(&legs)) {
                        out.pending += 1;
                        continue;
                    }
                    let _ = self.book(&mut out.records, &legs, 1);
                    out.gross += i128::from(p.amount);
                }
            }
        }
        out
    }
}
