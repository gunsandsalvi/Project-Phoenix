use std::collections::BTreeMap;

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
use crate::contract_process::ArrearsKey;
use crate::fails::Fail;
use crate::instruction::{AccountRef, Denom, Instruction, InstructionId, LegKind, LegRec, ReasonId, RowOp};
use crate::line::NewRow;
use crate::rows::{Optional, RowView};

/// A holder on a line side: the party, its members and its unit.
type Held = (PartyId, u32, u32);

/// A tally's count of members as a change to it.
fn signed(members: u64) -> i64 {
    let Ok(x) = i64::try_from(members) else {
        capacity_exceeded!("a holder's members", i64::MAX, members);
    };
    x
}

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

    pub(crate) fn submit(
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
        let since = self.ledger.arrears.since(ArrearsKey::new(t.line, t.side, t.from));
        let id = self.move_members(t, m, audit)?;
        if let Some(since) = since {
            self.carry_arrears(ArrearsKey::new(t.line, t.side, t.to), since);
        }
        Ok(id)
    }

    /// Members and their share of a row's balance moved to another party's row, one instruction, their arrears left
    /// behind.
    fn move_members(&mut self, t: LineTransfer, m: MoveAt, audit: &mut dyn AuditStream) -> Result<InstructionId, Fail> {
        let view = self.row_or_stop(t.from, t.line, t.side);
        let moved = share(&view, t.count, m.rounding);
        let mut legs = self.leave((t.from, t.line, t.side), t.count, moved, m);
        legs.extend(self.arrive((t.to, t.line, t.side), t.count, moved, &view, m));
        self.submit(t.reason, legs, m, audit)
    }

    /// Arrears carried with members onto another row: the row is in arrears since the earlier of its own and theirs.
    #[clause("SET.3")]
    fn carry_arrears(&mut self, to: ArrearsKey, since: Day) {
        let arrears = &mut self.ledger.arrears;
        match arrears.since(to) {
            Some(own) if own <= since => {}
            Some(_) => {
                arrears.remove(to);
                arrears.begin(to, since);
            }
            None => arrears.begin(to, since),
        }
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

    /// The legs putting `count` new members onto a party's row on a side of a line: onto its row there, or a row opened
    /// with the words its side keeps, each at nothing.
    fn enter(&self, (party, line, side): At, count: u32, m: MoveAt) -> LegRec {
        if self.row_on_side(party, line, side).is_some() {
            return count_leg(party, line, side, i64::from(count), m.contracts, RowOp::Count);
        }
        let words = self.ledger.lines.side_decl(line, side).words;
        let word = |flag: u8| if words & flag == 0 { Missing::Absent } else { Missing::Present(0) };
        let optional = Optional {
            balance: word(crate::rows::BALANCE),
            pending: word(crate::rows::PENDING),
            amount: Missing::Absent,
        };
        let new = NewRow { side, within: 0, count, point: 0, optional };
        count_leg(party, line, side, i64::from(count), m.contracts, RowOp::Open(new))
    }

    /// Members joining a line with as many of its other side, as a hire joins an employee and its employer: `count`
    /// members onto a party's row and as many onto its named counterparty's row on the other side, each row opened
    /// where it does not exist, in one instruction. New members bring no balance.
    ///
    /// # Errors
    /// The fail, when the instruction could not settle.
    #[clause("REP.3", "REP.9", "LAB.1")]
    pub fn members_join(
        &mut self,
        (party, line, side): At,
        counterparty: PartyId,
        count: u32,
        (reason, m): (ReasonId, MoveAt),
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        if count == 0 {
            violation!(clause = "REG.14", "members joining a line, none of them", line = line.get());
        }
        let other = match side {
            Side::Asset => Side::Liability,
            Side::Liability => Side::Asset,
        };
        let legs = vec![self.enter((party, line, side), count, m), self.enter((counterparty, line, other), count, m)];
        self.submit(reason, legs, m, audit)
    }

    /// Members leaving a line with as many of its other side: `count` members off a party's row, and as many off the
    /// rows of the other side's holders, each drawn by the members its row has left and giving its whole unit, so the
    /// sides stay equal and an agent's twins alike, in one instruction. A member leaving takes no share of a row's balance, which would be a claim the line still
    /// holds, and its counterparts none of theirs, which mirror the claims that stay. The members the other side
    /// cannot give whole units for pass instead to other holders of the party's side, drawn alike, as a buyer of the
    /// contracts would take them. A side that keeps no holder list is read beforehand, by `read_unlisted`.
    ///
    /// Returns the other side's holders drawn, with their members that left.
    ///
    /// # Errors
    /// The first fail, when an instruction could not settle; what settled before it stands.
    #[clause("REP.23", "REP.31")]
    pub fn members_leave(
        &mut self,
        (party, line, side): At,
        count: u32,
        m: MoveAt,
        d: &mut phx_rand::Draws,
        audit: &mut dyn AuditStream,
    ) -> Result<Vec<(PartyId, u32)>, Fail>
    where
        B: Sync,
    {
        let other = match side {
            Side::Asset => Side::Liability,
            Side::Liability => Side::Asset,
        };
        let mut theirs = self.side_tally(line, other);
        let ours = self.fresh_tally(line, side);
        let unit = self.parties.unit(party);
        let (taken, rest) = theirs.draw_many(count, unit, d);
        let leaving = count - rest;
        if leaving != 0 {
            let mut legs = self.leave((party, line, side), leaving, self.no_share((party, line, side), leaving, m), m);
            // The members leaving hold no balance, so the counterparts that leave with them take none of theirs.
            for (p, k) in &taken {
                legs.extend(self.leave((*p, line, other), *k, 0, m));
            }
            let _ = self.submit(self.dues.left, legs, m, audit)?;
        }
        self.leaving.insert((line, other), (self.ledger.lines.side_version(line, other), theirs));
        let mut ours = match ours {
            Some(mut t) => {
                t.adjust(party, -i64::from(leaving));
                t
            }
            None if rest == 0 => return Ok(taken),
            None => self.side_tally(line, side),
        };
        if rest != 0 {
            // The party is not among those drawn to take its own members.
            let mine = ours.held(party);
            ours.adjust(party, -signed(mine));
            let (passed, short) = ours.pass(rest, d);
            if short != 0 {
                violation!(
                    clause = "REP.31",
                    "members leaving a line whose sides hold none to take them in whole units",
                    remaining = short,
                    line = line.get(),
                    unit = unit
                );
            }
            ours.adjust(party, signed(mine) - i64::from(rest));
            for (to, k) in passed {
                ours.adjust(to, i64::from(k));
                // The taker takes the contracts as they stand, never the leaving party's arrears on them.
                let t = LineTransfer { line, side, from: party, to, count: k, reason: self.dues.succeeded };
                let _ = self.move_members(t, m, audit)?;
            }
        }
        self.leaving.insert((line, side), (self.ledger.lines.side_version(line, side), ours));
        Ok(taken)
    }

    /// A side's members by holder as members last left it, while the side is unchanged since.
    fn fresh_tally(&mut self, line: LineId, side: Side) -> Option<crate::cleared::Tally> {
        let version = self.ledger.lines.side_version(line, side);
        self.leaving.remove(&(line, side)).filter(|(read, _)| *read == version).map(|(_, t)| t)
    }

    /// A side's members by holder: as last left, while the side is unchanged, else read again through its holder list.
    /// A side that keeps none must have been read at its version.
    fn side_tally(&mut self, line: LineId, side: Side) -> crate::cleared::Tally
    where
        B: Sync,
    {
        if let Some(t) = self.fresh_tally(line, side) {
            return t;
        }
        if !self.ledger.lines.listed_side(line, side) {
            violation!(
                clause = "REP.23",
                "members leaving against a side of no holder list it has not read",
                line = line.get()
            );
        }
        let split = self.ledger.lines.keys();
        // A side of one holder is read at once, with no list to walk.
        if let Some(key) = self.ledger.lines.sole_holder(line, side) {
            let (place, slot) = split.split(key);
            let t = self.parties.holder(place);
            let rows: Vec<Held> = crate::rows::find(t, slot, line, side)
                .map(|r| (t.party(slot), r.row.count, t.weight(slot)))
                .into_iter()
                .collect();
            return crate::cleared::Tally::new(&rows);
        }
        // The list's keys name each holder's table and slot, so its row is read there, not through the directory, in
        // fixed shards of the list on the pool, joined in the list's order.
        let keys: Vec<u32> = self.ledger.lines.holders(line).collect();
        let shards = crate::consts::STREAM_SHARDS;
        let each = keys.len().div_ceil(shards);
        let found = phx_exec::pool::map(self.pool.as_deref(), shards, |k| {
            let from = crate::stream::at_most(k * each, keys.len());
            let to = crate::stream::at_most(from + each, keys.len());
            keys.get(from..to)
                .unwrap_or(&[])
                .iter()
                .filter_map(|key| {
                    let (place, slot) = split.split(*key);
                    let t = self.parties.holder(place);
                    crate::rows::find(t, slot, line, side).map(|r| (t.party(slot), r.row.count, t.weight(slot)))
                })
                .collect::<Vec<Held>>()
        });
        let rows: Vec<Held> = found.into_iter().flatten().collect();
        crate::cleared::Tally::new(&rows)
    }

    /// The sides that keep no holder list, read in one pass over the tables of the kinds that may hold them, for
    /// members to leave against: each side's members by holder, at the side's version. Each table is read in fixed
    /// shards of its slots, on the pool where the books have one, and the shards joined in slot order. Whether the
    /// tables were swept: not when every side keeps its list.
    #[clause("REP.23")]
    pub fn read_unlisted(&mut self, sides: &[(LineId, Side)]) -> bool
    where
        B: Sync,
    {
        let mut wanted: BTreeMap<(LineId, Side), Vec<Held>> =
            sides.iter().filter(|(l, s)| !self.ledger.lines.listed_side(*l, *s)).map(|k| (*k, Vec::new())).collect();
        if wanted.is_empty() {
            return false;
        }
        let lines = &self.ledger.lines;
        let holds = |kind: &str| wanted.keys().any(|(l, s)| lines.side_decl(*l, *s).holder_kinds.contains(&kind));
        let places: Vec<u16> = self.parties.places().filter(|p| holds(self.parties.holder(*p).kind())).collect();
        let shards = crate::consts::STREAM_SHARDS;
        let found = phx_exec::pool::map(self.pool.as_deref(), places.len() * shards, |i| {
            let (Some(place), k) = (places.get(i / shards), i % shards) else { return Vec::new() };
            let t = self.parties.holder(*place);
            let words = t.live_words();
            let each = words.len().div_ceil(shards);
            let from = crate::stream::at_most(k * each, words.len());
            let to = crate::stream::at_most(from + each, words.len());
            let Ok(base) = u32::try_from(from) else {
                capacity_exceeded!("a holder table's live words", u32::MAX, from);
            };
            let mut out: Vec<((LineId, Side), (PartyId, u32))> = Vec::new();
            for local in phx_store::table::live_in(words.get(from..to).unwrap_or(&[])) {
                let slot = phx_id::Slot::new(local.get() + base * u64::BITS);
                for r in crate::rows::rows(t, slot).iter().filter(|r| r.row.count != 0) {
                    if wanted.contains_key(&(r.row.line, r.side())) {
                        out.push(((r.row.line, r.side()), (t.party(slot), r.row.count)));
                    }
                }
            }
            out
        });
        for (at, (p, c)) in found.into_iter().flatten() {
            if let Some(v) = wanted.get_mut(&at) {
                v.push((p, c, self.parties.unit(p)));
            }
        }
        for ((line, side), rows) in wanted {
            let version = self.ledger.lines.side_version(line, side);
            self.leaving.insert((line, side), (version, crate::cleared::Tally::new(&rows)));
        }
        true
    }

    /// The sides' members kept for the leavings of a pass let go once the pass is done, so none outlives the changes
    /// the next pass brings, and their room is taken back.
    pub fn forget_tallies(&mut self) {
        self.leaving.clear();
    }

    /// The balance share of members leaving a line, which must be nothing.
    fn no_share(&self, at: At, count: u32, m: MoveAt) -> i64 {
        let moved = share(&self.row_or_stop(at.0, at.1, at.2), count, m.rounding);
        if moved != 0 {
            violation!(clause = "REP.9", "members leaving a line with a share of a row's balance", line = at.1.get());
        }
        moved
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
