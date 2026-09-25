use std::collections::BTreeMap;

use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::books::Books;
use crate::cleared::Losers;
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, ReasonId, Reasons, RowOp};

/// The reasons a contract's dues are paid for: its payments, which settle the receivable and payable its due made
/// when it fell, the income having been earned then, and its principal repaid, which moves a claim into money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueReasons {
    pub payment: ReasonId,
    pub principal: ReasonId,
    /// Members leaving a line with their counterparts, as a job ends with its worker.
    pub left: ReasonId,
    /// An estate succeeding to what the households it ended with held.
    pub succeeded: ReasonId,
    /// Units a hazard destroyed at their holder, which no one receives.
    pub destroyed: ReasonId,
    /// What an estate owed beyond what it held, lost by its creditor.
    pub written_off: ReasonId,
    /// What an estate held beyond what it owed, passed to its heirs or the law's destination.
    pub distributed: ReasonId,
}

impl DueReasons {
    pub fn declare(reasons: &mut Reasons) -> DueReasons {
        let payment =
            ReasonDecl { name: "contract payment", order: 0, paid: Effect::Liability, received: Effect::Asset };
        let principal =
            ReasonDecl { name: "principal repaid", order: 1, paid: Effect::Liability, received: Effect::Asset };
        // Neither moves money, so they share a place after the payments in the order.
        let left = ReasonDecl { name: "members left", order: 2, paid: Effect::Equity, received: Effect::Equity };
        let succeeded =
            ReasonDecl { name: "estate succeeded", order: 2, paid: Effect::Equity, received: Effect::Equity };
        let destroyed =
            ReasonDecl { name: "destroyed by a hazard", order: 2, paid: Effect::Expense, received: Effect::Equity };
        let written_off = ReasonDecl { name: "written off", order: 2, paid: Effect::Expense, received: Effect::Equity };
        let distributed =
            ReasonDecl { name: "estate distributed", order: 2, paid: Effect::Equity, received: Effect::Equity };
        DueReasons {
            payment: reasons.declare(payment),
            principal: reasons.declare(principal),
            left: reasons.declare(left),
            succeeded: reasons.declare(succeeded),
            destroyed: reasons.declare(destroyed),
            written_off: reasons.declare(written_off),
            distributed: reasons.declare(distributed),
        }
    }
}

/// How a line's dues are reckoned: on one side's rows, each paying or paid by the one party on the other side, or,
/// on a line of many holders on both sides, cleared, each row its own payment through the top issuer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Reckoning {
    On { side: Side, counter: PartyId },
    Cleared,
}

/// A cleared line's day: the top issuer its payments reach, the one due its rows pay per member, its claimant rows as
/// the day's stream read them, since a side of many small holders keeps no holder list, the members of its failed
/// payers, and the claimant members drawn to lose them.
#[derive(Debug)]
pub(crate) struct ClearedDay {
    pub top: PartyId,
    pub per_member: i64,
    pub claimants: BTreeMap<PartyId, u32>,
    pub failed: u64,
    pub losers: Option<Losers>,
}

impl ClearedDay {
    /// A claimant's members drawn to lose; before any payer fails, none.
    pub fn lost(&self, party: PartyId) -> u32 {
        self.losers.as_ref().map_or(0, |l| l.lost(party))
    }
}

/// What a day's dues have looked up once: the party owing each line, how each line is reckoned, each party's account
/// in each currency, and each cleared line's day.
#[derive(Debug, Default)]
pub(crate) struct Found {
    owers: BTreeMap<LineId, PartyId>,
    reckoned: BTreeMap<LineId, Reckoning>,
    accounts: BTreeMap<(PartyId, u8), Missing<LineId>>,
    issues: BTreeMap<(PartyId, u8), bool>,
    pub cleared: BTreeMap<LineId, ClearedDay>,
}

pub(crate) fn money_leg(party: PartyId, line: LineId, side: Side, qty: i64, ccy: Ccy) -> LegRec {
    LegRec { party, account: AccountRef::Line { line, side }, qty, denom: Denom::Ccy(ccy), kind: LegKind::Money }
}

pub(crate) fn row_leg(party: PartyId, line: LineId, side: Side, qty: i64, ccy: Ccy) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(ccy),
        kind: LegKind::Row(RowOp::Adjust),
    }
}

impl<B: Backing> Books<B> {
    /// A party's row on a side of a line, found by reading its run only as far as the row.
    pub(crate) fn row_on_side(&self, party: PartyId, line: LineId, side: Side) -> Option<crate::rows::RowView> {
        let crate::apply::Located::Live { table, slot, .. } = crate::apply::Holders::locate(&self.parties, party)
        else {
            return None;
        };
        crate::rows::iter(self.parties.holder(table), slot).find(|r| r.row.line == line && r.side() == side)
    }

    /// A line's holders, in its holder list's order.
    pub(crate) fn line_holders(&self, line: LineId) -> impl Iterator<Item = PartyId> + '_ {
        let keys = self.ledger.lines.keys();
        self.ledger.lines.holders(line).map(move |k| {
            let (place, slot) = keys.split(k);
            self.parties.holder(place).party(slot)
        })
    }

    /// Whether a line's holder list holds both its sides, so that two holders are the whole line; a side of many small
    /// holders keeps no list, and its holders are found only by reading their rows.
    fn both_listed(&self, line: LineId) -> bool {
        let lines = &self.ledger.lines;
        lines.listed_side(line, Side::Asset) && lines.listed_side(line, Side::Liability)
    }

    /// A line's holders other than a party, in its holder list's order.
    pub(crate) fn line_holders_but(&self, line: LineId, party: PartyId) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| *p != party).collect()
    }

    /// A line's holders on one side, in its holder list's order.
    pub(crate) fn side_holders(&self, line: LineId, side: Side) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| self.row_on_side(*p, line, side).is_some()).collect()
    }

    /// How a line's dues are reckoned: a line of two holders, both sides listed, on its claimant's row; a line one
    /// party holds a side of on the other side's rows, each its own payment with that party; a line of many holders on
    /// both sides, whose pairing is not recorded, cleared. A side that keeps no list is taken to hold many.
    #[clause("REP.23")]
    pub(crate) fn reckoning(&self, line: LineId, reader: PartyId, side: Side, found: &mut Found) -> Reckoning {
        if let Some(r) = found.reckoned.get(&line) {
            return *r;
        }
        let holders: Vec<PartyId> = self.line_holders(line).collect();
        let r = if self.both_listed(line)
            && let [a, b] = holders.as_slice()
        {
            let other = if *a == reader { *b } else { *a };
            match side {
                Side::Asset => Reckoning::On { side: Side::Asset, counter: other },
                Side::Liability => Reckoning::On { side: Side::Asset, counter: reader },
            }
        } else {
            let (mut owing, mut claiming) = (Vec::new(), Vec::new());
            for p in holders {
                if owing.len() > 1 && claiming.len() > 1 {
                    break;
                }
                if self.row_on_side(p, line, Side::Liability).is_some() {
                    owing.push(p);
                }
                if self.row_on_side(p, line, Side::Asset).is_some() {
                    claiming.push(p);
                }
            }
            match (owing.as_slice(), claiming.as_slice()) {
                ([one], _) => Reckoning::On { side: Side::Asset, counter: *one },
                (_, [one]) => Reckoning::On { side: Side::Liability, counter: *one },
                _ => Reckoning::Cleared,
            }
        };
        found.reckoned.insert(line, r);
        r
    }

    /// The one party that owes a line: its liability side's holder. On a line of two holders it is the one that is
    /// not its claimant, found without reading either's rows.
    pub(crate) fn owed_by(&self, line: LineId, claimant: PartyId, found: &mut Found) -> PartyId {
        if let Some(p) = found.owers.get(&line) {
            return *p;
        }
        let holders: Vec<PartyId> = self.line_holders(line).collect();
        let owers: Vec<PartyId> = match holders.as_slice() {
            [a, b] if *a == claimant && self.both_listed(line) => vec![*b],
            [a, b] if *b == claimant && self.both_listed(line) => vec![*a],
            _ => holders.into_iter().filter(|p| self.row_on_side(*p, line, Side::Liability).is_some()).collect(),
        };
        let [p] = owers.as_slice() else {
            violation!(clause = "REG.8", "a line owed by other than one party", line = line.get(), owers = owers.len());
        };
        found.owers.insert(line, *p);
        *p
    }

    /// The money a party pays from and is paid into in a currency: its one row on the holder's side of a money line.
    pub(crate) fn money_row(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> Missing<LineId> {
        if let Some(m) = found.accounts.get(&(party, ccy.index())) {
            return *m;
        }
        let lines = &self.ledger.lines;
        let held: Vec<LineId> = self
            .rows_of(party)
            .into_iter()
            .filter(|(line, side)| {
                *side == Side::Asset && lines.is_money(*line) && self.ledger.terms.get(lines.terms(*line)).ccy == ccy
            })
            .map(|(line, _)| line)
            .collect();
        let account = match held.as_slice() {
            [] => Missing::Absent,
            [line] => Missing::Present(*line),
            _ => violation!(
                clause = "MON.5",
                "a party with two money accounts in one currency and no declared account to pay from",
                party = party.get()
            ),
        };
        found.accounts.insert((party, ccy.index()), account);
        account
    }

    /// Whether a party has money in a currency to pay from and be paid into: an account, or, for an issuer of that
    /// money, its own liability. A party with neither holds no money in it.
    #[clause("MON.5", "MON.12")]
    pub(crate) fn holds_money(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> bool {
        if let Missing::Present(_) = self.money_row(party, ccy, found) {
            return true;
        }
        if let Some(i) = found.issues.get(&(party, ccy.index())) {
            return *i;
        }
        let lines = &self.ledger.lines;
        let issues = self.rows_of(party).into_iter().any(|(line, side)| {
            side == Side::Liability && lines.is_money(line) && self.ledger.terms.get(lines.terms(line)).ccy == ccy
        });
        found.issues.insert((party, ccy.index()), issues);
        issues
    }

    /// The legs moving money between a party's account and the top issuer, the one that pays in its own money: up
    /// through each issuer for an amount paid, down for an amount received (negative); and the top issuer reached.
    #[clause("MON.5", "MON.6")]
    pub(crate) fn route(&self, party: PartyId, x: i64, ccy: Ccy, found: &mut Found) -> (Vec<LegRec>, PartyId) {
        let mut legs = Vec::new();
        let mut at = party;
        while let Missing::Present(line) = self.money_row(at, ccy, found) {
            let issuer = self.owed_by(line, at, found);
            legs.push(money_leg(at, line, Side::Asset, -x, ccy));
            legs.push(money_leg(issuer, line, Side::Liability, x, ccy));
            at = issuer;
        }
        (legs, at)
    }

    /// The top issuer a party's money reaches, through each issuer of the account it holds.
    pub(crate) fn top_of(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> PartyId {
        let mut at = party;
        while let Missing::Present(line) = self.money_row(at, ccy, found) {
            at = self.owed_by(line, at, found);
        }
        at
    }

    /// The legs of a payment of money: from the payer's account to the payee's, each bank's liability moved with its
    /// depositor's, and between two issuers the same payment again one level up, until one issuer owes both.
    #[clause("MON.5", "MON.6")]
    pub(crate) fn pay(&self, from: PartyId, to: PartyId, x: i64, ccy: Ccy, found: &mut Found) -> Vec<LegRec> {
        let (paying, paid) = (self.money_row(from, ccy, found), self.money_row(to, ccy, found));
        if let Missing::Present(line) = paid
            && self.owed_by(line, to, found) == from
        {
            return vec![money_leg(to, line, Side::Asset, x, ccy), money_leg(from, line, Side::Liability, -x, ccy)];
        }
        if let Missing::Present(line) = paying
            && self.owed_by(line, from, found) == to
        {
            return vec![money_leg(from, line, Side::Asset, -x, ccy), money_leg(to, line, Side::Liability, x, ccy)];
        }
        let (Missing::Present(a), Missing::Present(b)) = (paying, paid) else {
            violation!(
                clause = "MON.5",
                "a payment with no money to pay from or into",
                from = from.get(),
                to = to.get()
            );
        };
        let (issuer_a, issuer_b) = (self.owed_by(a, from, found), self.owed_by(b, to, found));
        let mut legs = vec![money_leg(from, a, Side::Asset, -x, ccy), money_leg(to, b, Side::Asset, x, ccy)];
        if a != b {
            legs.push(money_leg(issuer_a, a, Side::Liability, x, ccy));
            legs.push(money_leg(issuer_b, b, Side::Liability, -x, ccy));
        }
        if issuer_a != issuer_b {
            legs.extend(self.pay(issuer_a, issuer_b, x, ccy, found));
        }
        legs
    }
}
