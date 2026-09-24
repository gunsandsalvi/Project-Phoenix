use phx_core::AuditStream;
use phx_core::register::limit::DeclaredLimit;
use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::round::{Round, split_total};
use phx_num::{Amount, Missing, UnitId, capacity_exceeded, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::apply::ApplyAt;
use crate::books::Books;
use crate::fails::Fail;
use crate::instruction::{AccountRef, Denom, Instruction, InstructionId, LegKind, LegRec, ReasonId, RowOp};
use crate::line::NewRow;
use crate::rows::{Optional, RowView};

/// A move of a count of one side of a line to another party — a sale of loans, a client moved to another clearing
/// member, an estate succeeding a party, a foreclosure — with its balance moved pro rata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineTransfer {
    pub line: LineId,
    pub side: Side,
    pub from: PartyId,
    pub to: PartyId,
    pub count: u32,
    pub reason: ReasonId,
}

/// A debtor's rows moved to a procedure line of the same kind whose terms carry the procedure's stay, so the stay
/// suspends only the moved rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToProcedureLine {
    pub line: LineId,
    pub side: Side,
    pub from: PartyId,
    pub count: u32,
    pub procedure: u16,
    pub reason: ReasonId,
}

/// What each instruction needs besides its legs: the unit rows are counted in, the balances' rounding, the day and
/// where it applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoveAt {
    pub contracts: UnitId,
    pub rounding: Round,
    pub day: Day,
    pub at: ApplyAt,
}

fn count_leg(party: PartyId, line: LineId, side: Side, members: i64, contracts: UnitId, op: RowOp) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: members,
        denom: Denom::Unit(contracts),
        kind: LegKind::Row(op),
    }
}

fn balance_leg(party: PartyId, line: LineId, side: Side, qty: i64, ccy: phx_num::Ccy) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(ccy),
        kind: LegKind::Row(RowOp::Adjust),
    }
}

/// A party's row on a side of a line, as a transfer names it.
type At = (PartyId, LineId, Side);

/// A row's balance share for `count` of its members, by the rounding the transfer names.
fn share(view: &RowView, count: u32, rounding: Round) -> i64 {
    match view.optional.balance {
        Missing::Present(b) => split_total(b, u64::from(count), u64::from(view.row.count), rounding).0,
        Missing::Absent => 0,
    }
}

impl<B: Backing> Books<B> {
    fn row_or_stop(&self, party: PartyId, line: LineId, side: Side) -> RowView {
        let Some(view) = self.row_on_side(party, line, side) else {
            violation!(clause = "REG.14", "a transfer of a row its party does not hold", line = line.get());
        };
        view
    }

    /// The legs moving `count` of a row's members, and their share of its balance, off it: a count leg, or the row
    /// retired when all its members leave; the balance first, so a retired row leaves nothing behind.
    fn leave(&self, (party, line, side): At, count: u32, moved: i64, m: MoveAt) -> Vec<LegRec> {
        let view = self.row_or_stop(party, line, side);
        if count > view.row.count || count == 0 {
            violation!(clause = "REG.14", "a transfer of more members than a row has, or none", line = line.get());
        }
        let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
        let mut legs = Vec::new();
        if moved != 0 {
            legs.push(balance_leg(party, line, side, -moved, ccy));
        }
        let op = if count == view.row.count { RowOp::Close } else { RowOp::Count };
        legs.push(count_leg(party, line, side, -i64::from(count), m.contracts, op));
        legs
    }

    /// The legs putting `count` members and a balance onto a party's row on a line: onto its row there, or a row
    /// opened with the words its side keeps and the point the members came from.
    fn arrive(&self, (party, line, side): At, count: u32, moved: i64, source: &RowView, m: MoveAt) -> Vec<LegRec> {
        let ccy = self.ledger.terms.get(self.ledger.lines.terms(line)).ccy;
        let mut legs = Vec::new();
        if self.row_on_side(party, line, side).is_some() {
            legs.push(count_leg(party, line, side, i64::from(count), m.contracts, RowOp::Count));
        } else {
            let zero = |w: Missing<i64>| match w {
                Missing::Present(_) => Missing::Present(0),
                Missing::Absent => Missing::Absent,
            };
            let o = source.optional;
            let optional = Optional { balance: zero(o.balance), pending: zero(o.pending), amount: o.amount };
            let new = NewRow { side, within: 0, count, point: source.row.point, optional };
            legs.push(count_leg(party, line, side, i64::from(count), m.contracts, RowOp::Open(new)));
        }
        if moved != 0 {
            legs.push(balance_leg(party, line, side, moved, ccy));
        }
        legs
    }

    fn submit(
        &mut self,
        reason: ReasonId,
        legs: Vec<LegRec>,
        m: MoveAt,
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        let id = self.ledger.next_id(m.day);
        let instruction = Instruction {
            id,
            reason,
            trade_day: m.day,
            settle_day: m.day,
            legs,
            pays: Missing::Absent,
            covers: Vec::new(),
        };
        self.ledger.apply(&mut self.parties, m.at, instruction, audit)
    }

    /// A line transfer: `count` members of a party's row, with their share of its balance split exactly by count, moved to
    /// another party's row on the same side, as one instruction.
    ///
    /// # Errors
    /// The fail, when the instruction could not settle.
    #[clause("REP.9", "BNK.10", "L3")]
    pub fn transfer(&mut self, t: LineTransfer, m: MoveAt, audit: &mut dyn AuditStream) -> Result<InstructionId, Fail> {
        let view = self.row_or_stop(t.from, t.line, t.side);
        let moved = share(&view, t.count, m.rounding);
        let mut legs = self.leave((t.from, t.line, t.side), t.count, moved, m);
        legs.extend(self.arrive((t.to, t.line, t.side), t.count, moved, &view, m));
        self.submit(t.reason, legs, m, audit)
    }

    /// A debtor's rows moved to a procedure line: a new line of the same kind whose terms add the procedure's stay,
    /// falling due on the old line's dates; the debtor's members and balance move to it, and so do as many members of
    /// the line's one counterparty, with the balance that mirrors the debtor's, so both lines keep their sides equal and
    /// no other holder's dues change.
    ///
    /// # Errors
    /// The fail, when the instruction could not settle.
    #[clause("REG.8", "REP.9", "L3")]
    pub fn to_procedure_line(
        &mut self,
        p: ToProcedureLine,
        m: MoveAt,
        audit: &mut dyn AuditStream,
    ) -> Result<LineId, Fail> {
        let other_side = match p.side {
            Side::Asset => Side::Liability,
            Side::Liability => Side::Asset,
        };
        let counterparties: Vec<PartyId> = self
            .line_holders_but(p.line, p.from)
            .into_iter()
            .filter(|c| self.row_on_side(*c, p.line, other_side).is_some())
            .collect();
        let [counterparty] = counterparties.as_slice() else {
            violation!(
                clause = "REP.23",
                "a procedure line over a side of many holders needs its pairing drawn",
                line = p.line.get()
            );
        };
        let mut terms = self.ledger.terms.get(self.ledger.lines.terms(p.line)).clone();
        terms.stay = Missing::Present(p.procedure);
        let id = self.ledger.terms.intern(terms);
        let first = if self.ledger.lines.done(p.line) {
            Missing::Absent
        } else {
            Missing::Present((self.ledger.lines.next_due(p.line), self.ledger.lines.fallen(p.line) + 1))
        };
        let kind = self.ledger.lines.kind_of(p.line);
        let new = self.ledger.lines.open(kind, id, first);
        let debtor = self.row_or_stop(p.from, p.line, p.side);
        let theirs = self.row_or_stop(*counterparty, p.line, other_side);
        let moved = share(&debtor, p.count, m.rounding);
        let mut legs = self.leave((p.from, p.line, p.side), p.count, moved, m);
        legs.extend(self.leave((*counterparty, p.line, other_side), p.count, -moved, m));
        legs.extend(self.arrive((p.from, new, p.side), p.count, moved, &debtor, m));
        legs.extend(self.arrive((*counterparty, new, other_side), p.count, -moved, &theirs, m));
        self.submit(p.reason, legs, m, audit).map(|_| new)
    }
}

/// A holder's rows split at a per-person limit: in the declared coverage order, each row's part per member within
/// what the limit, times the holder's persons, has left — bound against the member's share of the row — times the
/// row's count, and the rest of the row's total beside it. Exact for any total: the insured part is a whole amount
/// per member times the count, and the remainder of a total not divisible by the count stays in the rest. General
/// over row kinds: a deposit's balance, a benefit per year, an accrued pension.
#[clause("SUP.2", "SUP.5", "Law 6")]
#[must_use]
pub fn split_at_kink(rows: &[(i64, u32)], per_person: DeclaredLimit<Amount>, persons: u32) -> Vec<(i64, i64)> {
    let whole = i128::from(i64::MAX);
    let limit = whole - i128::from(per_person.bind(Amount::from_raw(i64::MAX)).excess().raw());
    let mut left = limit * i128::from(persons);
    rows.iter()
        .map(|&(total, count)| {
            if count == 0 || total < 0 {
                violation!(clause = "SUP.2", "a split over a row of no members or a negative total", total = total);
            }
            let per_member = i128::from(total) / i128::from(count);
            let insured_each = if per_member < left { per_member } else { left };
            left -= insured_each;
            let insured = insured_each * i128::from(count);
            let Ok(insured) = i64::try_from(insured) else {
                capacity_exceeded!("an insured part", i64::MAX, 0);
            };
            (insured, total - insured)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use phx_core::register::limit::{DeclaredLimit, TermsToken};
    use phx_num::Amount;

    use super::split_at_kink;

    fn limit(per_person: i64) -> DeclaredLimit<Amount> {
        DeclaredLimit::from_terms(TermsToken::new(), Amount::from_raw(per_person))
    }

    #[test]
    fn split_at_kink_across_rows_by_person() {
        // A cell of 3 members, 2 adult holders each, at one bank: a current account of 100 001 and a savings account
        // of 50 000. The limit of 20 000 a person gives 40 000 a member, taken first from the current account (33 333
        // a member) and then 6 667 from savings (16 666 a member).
        let out = split_at_kink(&[(100_001, 3), (50_000, 3)], limit(20_000), 2);
        assert_eq!(out, vec![(99_999, 2), (20_001, 29_999)]);
        let covered: i64 = out.iter().map(|(i, _)| i).sum();
        assert_eq!(covered, 3 * 40_000, "limit × persons per member, times the members");
        assert!(out.iter().zip([100_001, 50_000]).all(|((i, r), t)| i + r == t), "exact for any total");
    }

    #[test]
    fn split_at_kink_over_benefit_rows() {
        // A guarantee fund's cap of 1 000 a year a person over pensions in payment of 5 members totalling 7 777 a
        // year: 1 000 a member is covered, 5 000 in all, and the remainder of the indivisible total stays uncovered.
        assert_eq!(split_at_kink(&[(7_777, 5)], limit(1_000), 1), vec![(5_000, 2_777)]);
        assert_eq!(
            split_at_kink(&[(4_999, 5)], limit(1_000), 1),
            vec![(4_995, 4)],
            "999 a member, the indivisible 4 beside"
        );
    }
}
