use std::collections::BTreeMap;

use phx_core::calendar::Calendar;
use phx_core::{AuditStream, SubStep};
use phx_id::{Day, LineId, PartyId};
use phx_macros::clause;
use phx_num::{Ccy, Missing, Money, violation};
use phx_store::Backing;

use crate::algebra::{Amount, DueBuf, DueState, Leg, Side, due_on};
use crate::apply::ApplyAt;
use crate::books::Books;
use crate::instruction::{
    AccountRef, Denom, DueRow, Effect, Instruction, InstructionId, LegKind, LegRec, ReasonDecl, ReasonId, Reasons,
    RowOp,
};

/// The reasons a contract's dues are paid for: its payments, which are income and expense, and its principal repaid,
/// which moves a claim into money.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DueReasons {
    pub payment: ReasonId,
    pub principal: ReasonId,
}

impl DueReasons {
    pub fn declare(reasons: &mut Reasons) -> DueReasons {
        let payment =
            ReasonDecl { name: "contract payment", order: 0, paid: Effect::Expense, received: Effect::Revenue };
        let principal =
            ReasonDecl { name: "principal repaid", order: 1, paid: Effect::Liability, received: Effect::Asset };
        DueReasons { payment: reasons.declare(payment), principal: reasons.declare(principal) }
    }
}

/// What a day's dues came to: the lines that fell due, the instructions made, and those that settled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DuesPaid {
    pub lines: u64,
    pub instructions: u64,
    pub settled: u64,
}

/// What a day's dues have looked up once: the party owing each line, and each party's account in each currency.
#[derive(Debug, Default)]
struct Found {
    owers: BTreeMap<LineId, PartyId>,
    accounts: BTreeMap<(PartyId, u8), Missing<LineId>>,
}

fn money_leg(party: PartyId, line: LineId, side: Side, qty: i64, ccy: Ccy) -> LegRec {
    LegRec { party, account: AccountRef::Line { line, side }, qty, denom: Denom::Ccy(ccy), kind: LegKind::Money }
}

fn row_leg(party: PartyId, line: LineId, side: Side, qty: i64, ccy: Ccy) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(ccy),
        kind: LegKind::Row(RowOp::Adjust),
    }
}

impl<B: Backing> Books<B> {
    /// A line's holders on one side, in its holder list's order.
    fn side_holders(&self, line: LineId, side: Side) -> Vec<PartyId> {
        let keys = self.ledger.lines.keys();
        self.ledger
            .lines
            .holders(line)
            .map(|k| keys.split(k))
            .map(|(place, slot)| self.parties.table(place).party(slot))
            .filter(|p| self.rows_of(*p).contains(&(line, side)))
            .collect()
    }

    /// The one party that owes a line: its liability side's holder.
    fn owed_by(&self, line: LineId, found: &mut Found) -> PartyId {
        if let Some(p) = found.owers.get(&line) {
            return *p;
        }
        let owers = self.side_holders(line, Side::Liability);
        let [p] = owers.as_slice() else {
            violation!(clause = "REG.8", "a line owed by other than one party", line = line.get(), owers = owers.len());
        };
        found.owers.insert(line, *p);
        *p
    }

    /// The money a party pays from and is paid into in a currency: its one row on the holder's side of a money line.
    fn money_row(&self, party: PartyId, ccy: Ccy, found: &mut Found) -> Missing<LineId> {
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
    fn pay(&self, from: PartyId, to: PartyId, x: i64, ccy: Ccy, found: &mut Found) -> Vec<LegRec> {
        let (paying, paid) = (self.money_row(from, ccy, found), self.money_row(to, ccy, found));
        if let Missing::Present(line) = paid
            && self.owed_by(line, found) == from
        {
            return vec![money_leg(to, line, Side::Asset, x, ccy), money_leg(from, line, Side::Liability, -x, ccy)];
        }
        if let Missing::Present(line) = paying
            && self.owed_by(line, found) == to
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
        let (issuer_a, issuer_b) = (self.owed_by(a, found), self.owed_by(b, found));
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

    /// Stage 7's dated flows: every line due today pays each holder of its asset side what its terms make due, from
    /// the party that owes it, each due its own instruction, applied in the declared payment order; each paying line
    /// then moves to its next date. A due that fails stays for the contract process.
    #[clause("REG.5", "REG.8", "SET.1", "SET.4")]
    pub fn pay_dues(&mut self, day: Day, calendar: &Calendar, audit: &mut dyn AuditStream) -> DuesPaid {
        let due: Vec<LineId> = self
            .ledger
            .lines
            .ids()
            .filter(|l| !self.ledger.lines.done(*l) && self.ledger.lines.next_due(*l) == day)
            .collect();
        let mut found = Found::default();
        let owed = self.gather(&due, &mut found);
        let mut list = Vec::new();
        let mut next = Vec::with_capacity(due.len());
        let mut buf = DueBuf::default();
        for line in &due {
            let terms = self.ledger.terms.get(self.ledger.lines.terms(*line)).clone();
            let Some(k) = terms.schedule.date_index(calendar, day) else {
                violation!(clause = "REG.5", "a line due on a day that is none of its dates", line = line.get());
            };
            let spent = matches!(terms.schedule.count, Missing::Present(n) if k >= n);
            next.push((
                *line,
                if spent { Missing::Absent } else { Missing::Present(terms.schedule.dates.nth(calendar, k + 1)) },
            ));
            let payer = self.owed_by(*line, &mut found);
            for &(holder, outstanding) in owed.get(line).map_or(&[][..], Vec::as_slice) {
                let state = DueState {
                    calendar,
                    outstanding: Money::new(outstanding, terms.ccy),
                    elected: &|_, _| false,
                    occurred: &|_, _| false,
                    in_state_since: &|_, _| Missing::Absent,
                };
                due_on(&terms, day, &state, &mut buf);
                for d in buf.iter() {
                    let Amount::Money(m) = d.amount else {
                        violation!(
                            clause = "REG.5",
                            "a due in other than money, which no settlement pays yet",
                            line = line.get()
                        );
                    };
                    if m.amt() == 0 {
                        continue;
                    }
                    let principal = matches!(terms.legs.get(usize::from(d.leg)), Some(Leg::Principal { .. }));
                    let mut legs = self.pay(payer, holder, m.amt(), terms.ccy, &mut found);
                    if principal {
                        legs.push(row_leg(holder, *line, Side::Asset, -m.amt(), terms.ccy));
                        legs.push(row_leg(payer, *line, Side::Liability, m.amt(), terms.ccy));
                    }
                    let reason = if principal { self.dues.principal } else { self.dues.payment };
                    list.push((reason, legs, *line));
                }
            }
        }
        let instructions: Vec<Instruction> = list
            .into_iter()
            .enumerate()
            .map(|(n, (reason, legs, line))| {
                let Ok(seq) = u32::try_from(n) else {
                    phx_num::capacity_exceeded!("instructions of one day", u32::MAX, n);
                };
                Instruction {
                    id: InstructionId::new(day, seq),
                    reason,
                    trade_day: day,
                    settle_day: day,
                    legs,
                    pays: Missing::Present(DueRow { line, side: Side::Liability }),
                    covers: Vec::new(),
                }
            })
            .collect();
        let made = phx_rand::float::len_u64(instructions.len());
        let results = self.ledger.settle(&mut self.parties, ApplyAt::Day(SubStep::S7c), instructions, audit);
        for (line, n) in next {
            self.ledger.lines.advance(line, n);
        }
        DuesPaid {
            lines: phx_rand::float::len_u64(due.len()),
            instructions: made,
            settled: phx_rand::float::len_u64(results.iter().filter(|r| r.is_ok()).count()),
        }
    }

    /// The rows on today's due lines, in one pass over every holder's rows: each line's owing party, kept for the
    /// payments, and each holder of its asset side with the balance its dues are reckoned on.
    fn gather(&self, due: &[LineId], found: &mut Found) -> BTreeMap<LineId, Vec<(PartyId, i64)>> {
        let mut owed: BTreeMap<LineId, Vec<(PartyId, i64)>> = BTreeMap::new();
        let mut owers: BTreeMap<LineId, Vec<PartyId>> = BTreeMap::new();
        if due.is_empty() {
            return owed;
        }
        for kind in self.parties.kinds() {
            let table = self.parties.table(self.parties.place(kind));
            for slot in table.slots() {
                let party = table.party(slot);
                for r in crate::rows::rows(table, slot).into_iter().filter(|r| due.binary_search(&r.row.line).is_ok()) {
                    let line = r.row.line;
                    match (r.side(), r.optional.balance) {
                        (Side::Liability, _) => owers.entry(line).or_default().push(party),
                        (Side::Asset, Missing::Present(b)) => owed.entry(line).or_default().push((party, b)),
                        (Side::Asset, Missing::Absent) => {
                            violation!(clause = "REG.8", "a contract row with no balance to pay on", line = line.get());
                        }
                    }
                }
            }
        }
        for (line, parties) in owers {
            let [p] = parties.as_slice() else {
                violation!(
                    clause = "REG.8",
                    "a line owed by other than one party",
                    line = line.get(),
                    owers = parties.len()
                );
            };
            found.owers.insert(line, *p);
        }
        owed
    }
}
