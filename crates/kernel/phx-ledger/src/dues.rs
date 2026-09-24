use std::collections::BTreeMap;

use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, violation};
use phx_store::Backing;

use crate::algebra::Side;
use crate::books::Books;
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, ReasonId, Reasons, RowOp};

/// The reasons a contract's dues are paid for: its payments, which settle the receivable and payable its due made
/// when it fell, the income having been earned then, and its principal repaid, which moves a claim into money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueReasons {
    pub payment: ReasonId,
    pub principal: ReasonId,
}

impl DueReasons {
    pub fn declare(reasons: &mut Reasons) -> DueReasons {
        let payment =
            ReasonDecl { name: "contract payment", order: 0, paid: Effect::Liability, received: Effect::Asset };
        let principal =
            ReasonDecl { name: "principal repaid", order: 1, paid: Effect::Liability, received: Effect::Asset };
        DueReasons { payment: reasons.declare(payment), principal: reasons.declare(principal) }
    }
}

/// What a day's dues have looked up once: the party owing each line, and each party's account in each currency.
#[derive(Debug, Default)]
pub(crate) struct Found {
    owers: BTreeMap<LineId, PartyId>,
    reckoned: BTreeMap<LineId, (Side, PartyId)>,
    accounts: BTreeMap<(PartyId, u8), Missing<LineId>>,
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
        crate::rows::iter(self.parties.table(table), slot).find(|r| r.row.line == line && r.side() == side)
    }

    /// A line's holders, in its holder list's order.
    fn line_holders(&self, line: LineId) -> impl Iterator<Item = PartyId> + '_ {
        let keys = self.ledger.lines.keys();
        self.ledger.lines.holders(line).map(move |k| {
            let (place, slot) = keys.split(k);
            self.parties.table(place).party(slot)
        })
    }

    /// A line's holders other than a party, in its holder list's order.
    pub(crate) fn line_holders_but(&self, line: LineId, party: PartyId) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| *p != party).collect()
    }

    /// A line's holders on one side, in its holder list's order.
    pub(crate) fn side_holders(&self, line: LineId, side: Side) -> Vec<PartyId> {
        self.line_holders(line).filter(|p| self.row_on_side(*p, line, side).is_some()).collect()
    }

    /// Which side of a line its dues are reckoned on, and the one party on the other side, the counterparty of every
    /// payment: a line of two holders is reckoned on its claimant's row; a line one party holds a side of is reckoned
    /// on the other side's rows, each its own payment with that party. A line of many holders on both sides needs a
    /// pairing drawn before it can pay.
    pub(crate) fn reckoning(&self, line: LineId, reader: PartyId, side: Side, found: &mut Found) -> (Side, PartyId) {
        if let Some(r) = found.reckoned.get(&line) {
            return *r;
        }
        let holders: Vec<PartyId> = self.line_holders(line).collect();
        let r = if let [a, b] = holders.as_slice() {
            let other = if *a == reader { *b } else { *a };
            match side {
                Side::Asset => (Side::Asset, other),
                Side::Liability => (Side::Asset, reader),
            }
        } else {
            let owing: Vec<PartyId> =
                holders.iter().copied().filter(|p| self.row_on_side(*p, line, Side::Liability).is_some()).collect();
            let claiming: Vec<PartyId> =
                holders.iter().copied().filter(|p| self.row_on_side(*p, line, Side::Asset).is_some()).collect();
            match (owing.as_slice(), claiming.as_slice()) {
                ([one], _) => (Side::Asset, *one),
                (_, [one]) => (Side::Liability, *one),
                _ => violation!(
                    clause = "REP.23",
                    "dues on a line of many holders on both sides need a pairing",
                    line = line.get()
                ),
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
            [a, b] if *a == claimant => vec![*b],
            [a, b] if *b == claimant => vec![*a],
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
