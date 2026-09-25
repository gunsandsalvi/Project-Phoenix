use std::collections::BTreeMap;

use phx_core::KernelMap;
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
    /// One twin's contracts moved from its agent to the household that stands alone for it.
    pub seated: ReasonId,
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
        let seated = ReasonDecl { name: "twin seated", order: 2, paid: Effect::Equity, received: Effect::Equity };
        DueReasons {
            payment: reasons.declare(payment),
            principal: reasons.declare(principal),
            left: reasons.declare(left),
            succeeded: reasons.declare(succeeded),
            destroyed: reasons.declare(destroyed),
            written_off: reasons.declare(written_off),
            distributed: reasons.declare(distributed),
            seated: reasons.declare(seated),
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
    pub claimants: BTreeMap<PartyId, (u32, u32)>,
    pub failed: u64,
    pub losers: Option<Losers>,
}

impl ClearedDay {
    /// A claimant's members drawn to lose; before any payer fails, none.
    pub fn lost(&self, party: PartyId) -> u32 {
        self.losers.as_ref().map_or(0, |l| l.lost(party))
    }
}

/// What a day's dues have looked up once: each cleared line's day.
#[derive(Debug, Default)]
pub(crate) struct Found {
    pub cleared: KernelMap<LineId, ClearedDay>,
}

impl Found {
    /// What the day's look-ups hold in memory: the map's room, and each cleared line's claimants and losers' draw.
    pub(crate) fn bytes(&self) -> usize {
        let cleared: usize = self
            .cleared
            .sorted()
            .iter()
            .map(|(_, c)| {
                c.claimants.len() * size_of::<(PartyId, (u32, u32))>() + c.losers.as_ref().map_or(0, Losers::bytes)
            })
            .sum();
        self.cleared.capacity() * size_of::<(LineId, ClearedDay)>() + cleared
    }
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
        crate::rows::find(self.parties.holder(table), slot, line, side)
    }

    /// A line's holders, in its holder list's order.
    pub(crate) fn line_holders(&self, line: LineId) -> impl Iterator<Item = PartyId> + '_ {
        let keys = self.ledger.lines.keys();
        self.ledger.lines.holders(line).map(move |k| {
            let (place, slot) = keys.split(k);
            self.parties.holder(place).party(slot)
        })
    }

    /// A line's holders other than a party, in its holder list's order.
    pub(crate) fn line_holders_but(&self, line: LineId, party: PartyId) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| *p != party).collect()
    }

    /// The listed holders of one side of a line with their rows' counts, in the holder list's order.
    #[must_use]
    pub fn side_counts(&self, line: LineId, side: Side) -> Vec<(PartyId, u32)> {
        self.side_holders(line, side)
            .into_iter()
            .filter_map(|p| self.row_on_side(p, line, side).map(|r| (p, r.row.count)))
            .collect()
    }

    /// A line's holders on one side, in its holder list's order.
    pub(crate) fn side_holders(&self, line: LineId, side: Side) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| self.row_on_side(*p, line, side).is_some()).collect()
    }

    /// How a line's dues are reckoned: on the other side's rows where one listed side holds one party, the owing
    /// side's first, each row its own payment with that party; a line of many holders on both sides, whose pairing is
    /// not recorded, cleared. A side that keeps no list is taken to hold many.
    #[clause("REP.23")]
    pub(crate) fn reckoning(&self, line: LineId) -> Reckoning {
        let lines = &self.ledger.lines;
        let sole = |side: Side| {
            if !lines.side_decl(line, side).holder_list {
                return None;
            }
            lines.sole_holder(line, side).map(|k| self.party_of_key(k))
        };
        if let Some(counter) = sole(Side::Liability) {
            return Reckoning::On { side: Side::Asset, counter };
        }
        match sole(Side::Asset) {
            Some(counter) => Reckoning::On { side: Side::Liability, counter },
            None => Reckoning::Cleared,
        }
    }

    /// The party a holder-list key names.
    fn party_of_key(&self, key: u32) -> PartyId {
        let (place, slot) = self.ledger.lines.keys().split(key);
        self.parties.holder(place).party(slot)
    }

    /// The one party that owes a line: its liability side's one listed holder.
    pub(crate) fn owed_by(&self, line: LineId) -> PartyId {
        let lines = &self.ledger.lines;
        match lines.sole_holder(line, Side::Liability) {
            Some(k) if lines.side_decl(line, Side::Liability).holder_list => self.party_of_key(k),
            _ => violation!(
                clause = "REG.8",
                "a line owed by other than one party",
                line = line.get(),
                owers = lines.listed_holders(line, Side::Liability)
            ),
        }
    }

    /// Each of a party's rows on means-of-payment lines in a currency, on one side, handed to `each`: from the
    /// holder's summary, or its rows where it holds more than the summary keeps.
    fn each_money_line(&self, party: PartyId, ccy: Ccy, side: Side, mut each: impl FnMut(LineId)) {
        let crate::apply::Located::Live { table, slot, .. } = crate::apply::Holders::locate(&self.parties, party)
        else {
            return;
        };
        let lines = &self.ledger.lines;
        let of_ccy = |line: LineId| self.ledger.terms.get(lines.terms(line)).ccy == ccy;
        match lines.money_rows(table, slot) {
            Some(m) => m.rows().filter(|(l, s)| *s == side && of_ccy(*l)).for_each(|(l, _)| each(l)),
            None => self
                .rows_of(party)
                .into_iter()
                .filter(|(line, s)| *s == side && lines.is_money(*line) && of_ccy(*line))
                .for_each(|(line, _)| each(line)),
        }
    }

    /// The money a party pays from and is paid into in a currency: its one row on the holder's side of a money line.
    pub(crate) fn money_row(&self, party: PartyId, ccy: Ccy) -> Missing<LineId> {
        let (mut account, mut accounts) = (Missing::Absent, 0_u32);
        self.each_money_line(party, ccy, Side::Asset, |line| {
            account = Missing::Present(line);
            accounts += 1;
        });
        if accounts > 1 {
            violation!(
                clause = "MON.5",
                "a party with two money accounts in one currency and no declared account to pay from",
                party = party.get()
            );
        }
        account
    }

    /// Whether a party holds money in a currency: an account in it, or money it issues.
    pub(crate) fn holds_money(&self, party: PartyId, ccy: Ccy) -> bool {
        let mut issues = false;
        self.each_money_line(party, ccy, Side::Liability, |_| issues = true);
        issues || matches!(self.money_row(party, ccy), Missing::Present(_))
    }

    /// The legs moving money between a party's account and the top issuer, the one that pays in its own money: up
    /// through each issuer for an amount paid, down for an amount received (negative); and the top issuer reached.
    #[clause("MON.5", "MON.6")]
    pub(crate) fn route(&self, party: PartyId, x: i64, ccy: Ccy) -> (Vec<LegRec>, PartyId) {
        let mut legs = Vec::new();
        let mut at = party;
        while let Missing::Present(line) = self.money_row(at, ccy) {
            let issuer = self.owed_by(line);
            legs.push(money_leg(at, line, Side::Asset, -x, ccy));
            legs.push(money_leg(issuer, line, Side::Liability, x, ccy));
            at = issuer;
        }
        (legs, at)
    }

    /// The top issuer a party's money reaches, through each issuer of the account it holds.
    pub(crate) fn top_of(&self, party: PartyId, ccy: Ccy) -> PartyId {
        let mut at = party;
        while let Missing::Present(line) = self.money_row(at, ccy) {
            at = self.owed_by(line);
        }
        at
    }

    /// The legs of a payment of money: from the payer's account to the payee's, each bank's liability moved with its
    /// depositor's, and between two issuers the same payment again one level up, until one issuer owes both.
    #[clause("MON.5", "MON.6")]
    pub(crate) fn pay(&self, from: PartyId, to: PartyId, x: i64, ccy: Ccy) -> Vec<LegRec> {
        let (paying, paid) = (self.money_row(from, ccy), self.money_row(to, ccy));
        if let Missing::Present(line) = paid
            && self.owed_by(line) == from
        {
            return vec![money_leg(to, line, Side::Asset, x, ccy), money_leg(from, line, Side::Liability, -x, ccy)];
        }
        if let Missing::Present(line) = paying
            && self.owed_by(line) == to
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
        let (issuer_a, issuer_b) = (self.owed_by(a), self.owed_by(b));
        let mut legs = vec![money_leg(from, a, Side::Asset, -x, ccy), money_leg(to, b, Side::Asset, x, ccy)];
        if a != b {
            legs.push(money_leg(issuer_a, a, Side::Liability, x, ccy));
            legs.push(money_leg(issuer_b, b, Side::Liability, -x, ccy));
        }
        if issuer_a != issuer_b {
            legs.extend(self.pay(issuer_a, issuer_b, x, ccy));
        }
        legs
    }
}
