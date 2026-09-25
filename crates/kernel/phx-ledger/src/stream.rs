use std::collections::{BTreeMap, BTreeSet};

use phx_core::KernelMap;
use phx_core::calendar::Calendar;
use phx_id::{Day, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Ccy, Missing, Money, violation};
use phx_store::Backing;

use crate::algebra::{Amount, DueBuf, DuePlan, DueState, Leg, Side, due_at, due_by_plan, due_plan};
use crate::apply::{Holders, Located};
use crate::books::Books;
use crate::cleared::{per_contract, times};
use crate::consts::STREAM_SHARDS;
use crate::due::DueLines;
use crate::dues::{ClearedDay, Found, Reckoning, row_leg};
use crate::instruction::{AccountRef, LegKind, LegRec};
use crate::pending::Closed;
use crate::rows::RowView;
use crate::runs;

/// One row's dues on a line due today, reckoned on the row of the side its line is reckoned on and paid by the line's
/// liability side to its asset side; `principal` is the part that repays the claim, moving the contract's rows with
/// the money. A payment is named by its line and the holder of the row it was reckoned on, since a holder keeps one
/// row on a side of a line. A cleared line's payment is one row's, between its holder and the top issuer, for the
/// members it pays or is paid for.
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
    pub cleared: bool,
    pub members: u32,
    /// A party the payment needs an account of holds no money in its currency, so it cannot settle.
    pub moneyless: bool,
}

impl Payment {
    pub fn key(&self) -> (LineId, PartyId) {
        (self.line, self.reckoned_on)
    }
}

/// A row's due as far as the row alone decides it.
pub(crate) enum Reckoned {
    Paid(Payment),
    Cleared { per: i64, ccy: Ccy, order: u8 },
}

/// What a shard of 7a's heads found, in the stream's order, for the serial pass that books it.
enum Step {
    Scanned { place: u16, slot: Slot, read: u64, due: u64 },
    Paid(Payment, Vec<LegRec>),
    Cleared { holder: PartyId, row: RowView, per: i64, ccy: Ccy, order: u8 },
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

/// The day's records, one per party whose account the day's payments touch, kept by the party's holder table and slot
/// so a stream in slot order reads them in order, and stamped with the day they were made, so emptying them costs
/// nothing and they keep their room from one day to the next; read whole only in party order.
#[derive(Debug, Default)]
pub struct Records {
    tables: Vec<Vec<(u32, Option<Record>)>>,
    touched: Vec<(PartyId, u16, Slot)>,
    day: u32,
}

fn slot_at(slot: Slot) -> usize {
    let Ok(at) = usize::try_from(slot.get()) else {
        phx_num::capacity_exceeded!("holder slots", usize::MAX, slot.get());
    };
    at
}

impl Records {
    /// Empties the records for a new day, keeping their room.
    pub fn clear(&mut self) {
        let Some(next) = self.day.checked_add(1) else {
            phx_num::capacity_exceeded!("days of stage 7's records", u32::MAX, self.day);
        };
        self.day = next;
        self.touched.clear();
    }

    /// The record of the party at a holder table and slot, if the day made one.
    #[must_use]
    pub fn get(&self, (table, slot): (u16, Slot)) -> Option<&Record> {
        let (day, record) = self.tables.get(usize::from(table))?.get(slot_at(slot))?;
        if *day == self.day { record.as_ref() } else { None }
    }

    /// The record of a party at a holder table and slot, made by `make` where the day has none.
    pub fn get_or_insert_with(
        &mut self,
        party: PartyId,
        (table, slot): (u16, Slot),
        make: impl FnOnce() -> Record,
    ) -> &mut Record {
        let (t, at) = (usize::from(table), slot_at(slot));
        if self.tables.len() <= t {
            self.tables.resize_with(t + 1, Vec::new);
        }
        let Some(of_table) = self.tables.get_mut(t) else {
            phx_num::capacity_exceeded!("holder tables", t, t);
        };
        if of_table.len() <= at {
            of_table.resize(at + 1, (0, None));
        }
        let Some(entry) = of_table.get_mut(at) else {
            phx_num::capacity_exceeded!("holder slots", at, at);
        };
        if entry.0 != self.day || entry.1.is_none() {
            *entry = (self.day, Some(make()));
            self.touched.push((party, table, slot));
        }
        let Some(record) = entry.1.as_mut() else {
            violation!(clause = "MON.5", "a party's record not kept", party = party.get());
        };
        record
    }

    /// Every record the day made, in party order.
    #[must_use]
    pub fn sorted(&self) -> Vec<(PartyId, Record)> {
        let mut out: Vec<(PartyId, Record)> =
            self.touched.iter().filter_map(|&(p, t, s)| self.get((t, s)).map(|r| (p, *r))).collect();
        out.sort_unstable_by_key(|(p, _)| *p);
        out
    }

    /// The records the room holds, made today or not, so what they take in memory can be counted.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.tables.iter().map(Vec::len).sum()
    }
}

/// Stage 7a's result: one record per party with an account the day's payments touch, the holders whose runs were
/// scanned, the day's payments in the stream's order, so 7c gathers them without reckoning them again, and the day's
/// counts: heads read, rows of scanned segments read, the rows among them due today, the payments they make and those
/// held pending.
#[derive(Debug, Default)]
pub struct DayRecords {
    pub records: Records,
    pub scanned: Vec<(u16, Slot)>,
    pub made: Vec<Payment>,
    /// Each payment's legs as 7a routed them, in the payments' order, and where each payment's begin, so 7c reads a
    /// route rather than routing it again.
    pub legs: Vec<LegRec>,
    pub leg_at: Vec<usize>,
    pub heads_read: u64,
    pub rows_scanned: u64,
    pub rows_due: u64,
    pub payments: u64,
    pub pending: u64,
    pub gross: i128,
    /// The holders whose rows make payments that cannot settle, a party they need an account of holding no money.
    pub moneyless: BTreeSet<PartyId>,
}

impl DayRecords {
    /// The legs 7a routed the `i`th payment by.
    #[must_use]
    pub fn route(&self, i: usize) -> &[LegRec] {
        let from = self.leg_at.get(i).copied().unwrap_or(self.legs.len());
        let to = self.leg_at.get(i + 1).copied().unwrap_or(self.legs.len());
        self.legs.get(from..to).unwrap_or(&[])
    }
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

/// The lesser of two indexes.
fn at_most(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
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

    /// The payment a row makes due today, if any: on the side its line is reckoned on, its dues summed, the amounts
    /// per contract times its count and those its terms reckon on its balance as they are; on a cleared line, every
    /// row's.
    #[clause("REP.23", "REP.31")]
    pub(crate) fn payment(
        &self,
        holder: PartyId,
        row: &RowView,
        day: Day,
        calendar: &Calendar,
        found: &mut Found,
    ) -> Option<Payment> {
        let line = row.row.line;
        if found.plans.get(line).is_none() {
            let terms = self.ledger.terms.get(self.ledger.lines.terms(line));
            let plan = due_plan(terms, Some(self.ledger.lines.fallen(line)), day, calendar);
            let _ = found.plans.insert(line, plan);
        }
        match self.reckon(holder, row, (day, calendar), found.plans.get(line))? {
            Reckoned::Paid(p) => Some(p),
            Reckoned::Cleared { per, ccy, order } => self.cleared_payment(holder, row, per, (ccy, order), found),
        }
    }

    /// A row's due as far as the row alone decides it: its payment, or on a cleared line the due it pays or is paid
    /// per member, which the line's day turns into a payment. Reads the day's plan of the line's dues where given.
    #[clause("REP.23", "REP.31")]
    fn reckon(
        &self,
        holder: PartyId,
        row: &RowView,
        (day, calendar): (Day, &Calendar),
        plan: Option<&DuePlan>,
    ) -> Option<Reckoned> {
        let line = row.row.line;
        let reckoning = self.reckoning(line);
        if let Reckoning::On { side, .. } = reckoning
            && row.side() != side
        {
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
        let outstanding = match row.side() {
            Side::Asset => balance,
            Side::Liability => -balance,
        };
        // A due on the balance is one twin's, rounded by its convention, times the agent's twins, so every twin's
        // share stays whole.
        let unit = self.parties.unit(holder);
        if outstanding % i64::from(unit) != 0 {
            violation!(clause = "REP.9", "an agent's balance not a whole share for each twin", line = line.get());
        }
        let outstanding = Money::new(outstanding / i64::from(unit), terms.ccy);
        let mut buf = DueBuf::default();
        let k = self.ledger.lines.fallen(line);
        let planned;
        let plan = if let Some(p) = plan {
            p
        } else {
            planned = due_plan(terms, Some(k), day, calendar);
            &planned
        };
        if plan.general {
            let state = DueState {
                calendar,
                outstanding,
                elected: &|_, _| false,
                occurred: &|_, _| false,
                in_state_since: &|_, _| Missing::Absent,
            };
            due_at(terms, Some(k), day, &state, &mut buf);
        } else {
            due_by_plan(plan, outstanding, &mut buf);
        }
        let (mut per, mut whole, mut principal) = (0_i64, 0_i64, 0_i64);
        for d in buf.iter() {
            let Amount::Money(m) = d.amount else {
                violation!(
                    clause = "REG.5",
                    "a due in other than money, which no settlement pays yet",
                    line = line.get()
                );
            };
            let Some(leg) = terms.legs.get(usize::from(d.leg)) else {
                violation!(clause = "REG.5", "a due of a leg its terms do not hold", line = line.get());
            };
            if per_contract(leg) {
                per += m.amt();
            } else {
                whole += m.amt();
            }
            if matches!(leg, Leg::Principal { .. }) {
                principal += m.amt();
            }
        }
        let members = row.row.count;
        let Reckoning::On { side: reckoned, counter } = reckoning else {
            if !terms.legs.iter().all(per_contract) || principal != 0 {
                violation!(clause = "REP.23", "a cleared line whose dues are not paid per member", line = line.get());
            }
            return Some(Reckoned::Cleared { per, ccy: terms.ccy, order: terms.payment_order.0 });
        };
        let whole = times(whole, unit);
        let amount = times(per, members) + whole;
        if amount == 0 {
            return None;
        }
        let (from, to) = match reckoned {
            Side::Asset => (counter, holder),
            Side::Liability => (holder, counter),
        };
        let moneyless = !self.holds_money(from, terms.ccy) || !self.holds_money(to, terms.ccy);
        Some(Reckoned::Paid(Payment {
            line,
            payer: from,
            payee: to,
            amount,
            principal: times(principal, members),
            ccy: terms.ccy,
            order: terms.payment_order.0,
            reckoned_on: holder,
            cleared: false,
            members,
            moneyless,
        }))
    }

    /// A cleared line's row's payment: a liability row pays its per-member due for each of its members to the top
    /// issuer; an asset row is paid it for each of its members not drawn to lose to the line's failed payers. Every
    /// row of the line pays the same due and reaches the same top issuer; a holder with no money reaches none, and
    /// its payment names the line's.
    #[clause("REP.23", "MON.5")]
    fn cleared_payment(
        &self,
        holder: PartyId,
        row: &RowView,
        per: i64,
        (ccy, order): (Ccy, u8),
        found: &mut Found,
    ) -> Option<Payment> {
        let line = row.row.line;
        let moneyless = !self.holds_money(holder, ccy);
        let top = match (found.cleared.get(line), moneyless) {
            (Some(day), true) => day.top,
            (None, true) => self.line_top(line, ccy),
            (_, false) => self.top_of(holder, ccy),
        };
        if found.cleared.get(line).is_none() {
            let _ = found
                .cleared
                .insert(line, ClearedDay { top, per_member: per, claimants: BTreeMap::new(), failed: 0, losers: None });
        }
        let Some(day) = found.cleared.get_mut(line) else {
            violation!(clause = "REP.23", "a cleared line's day not kept", line = line.get());
        };
        if day.top != top || day.per_member != per {
            violation!(
                clause = "REP.23",
                "a cleared line whose rows pay different dues or reach different top issuers",
                line = line.get(),
                holder = holder.get(),
                top = top.get(),
                first_top = day.top.get(),
                per_member = per,
                first_per_member = day.per_member
            );
        }
        if row.side() == Side::Asset {
            day.claimants.insert(holder, (row.row.count, self.parties.unit(holder)));
        }
        let (from, to, members) = match row.side() {
            Side::Liability => (holder, top, row.row.count),
            Side::Asset => (top, holder, row.row.count - day.lost(holder)),
        };
        let amount = times(per, members);
        (amount != 0).then_some(Payment {
            line,
            payer: from,
            payee: to,
            amount,
            principal: 0,
            ccy,
            order,
            reckoned_on: holder,
            cleared: true,
            members,
            moneyless,
        })
    }

    /// The top issuer a cleared line's payments reach: the one its holders with money reach.
    fn line_top(&self, line: LineId, ccy: Ccy) -> PartyId {
        let holders: Vec<PartyId> = self.line_holders(line).collect();
        let Some(holder) = holders.into_iter().find(|p| self.holds_money(*p, ccy)) else {
            violation!(clause = "REP.23", "a cleared line none of whose holders holds money", line = line.get());
        };
        self.top_of(holder, ccy)
    }

    /// Every payment a party takes part in today, in its payment order: its due rows in its run's order, by the
    /// order of their terms; each claimant's row is its own payment, and each row owing a line is every payment the
    /// line's claimants make due.
    #[clause("REP.9")]
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
            let reckoned = match self.reckoning(r.row.line) {
                Reckoning::On { side, .. } if side != r.side() => side,
                _ => {
                    out.extend(self.payment(party, &r, day, calendar, found));
                    continue;
                }
            };
            for other in self.line_holders_but(r.row.line, party) {
                if let Some(row) = self.row_on_side(other, r.row.line, reckoned) {
                    out.extend(self.payment(other, &row, day, calendar, found));
                }
            }
        }
        out
    }

    /// What a payment does, leg by leg: the money from the payer's means of payment to the payee's, through deposits
    /// and, between banks, reserves; and the principal repaid off the contract's rows. A cleared line's payment moves
    /// its row's holder's money up to the top issuer, or down from it.
    pub(crate) fn effects(&self, p: &Payment) -> Vec<LegRec> {
        if p.moneyless {
            return Vec::new();
        }
        if p.cleared {
            let (legs, _) = if p.payer == p.reckoned_on {
                self.route(p.payer, p.amount, p.ccy)
            } else {
                self.route(p.payee, -p.amount, p.ccy)
            };
            return legs;
        }
        let mut legs = self.pay(p.payer, p.payee, p.amount, p.ccy);
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
        Record { account, funds: positions.funds(slot, account, facility), debit: 0, credit: 0 }
    }

    /// Adds or takes away a payment's effects on the accounts it touches: a leg drawing on an account is a debit, a
    /// leg paying into one a credit; an issuer's side of its own money is no account and has no record.
    pub(crate) fn book(&self, records: &mut Records, legs: &[LegRec], sign: i128) -> Vec<PartyId> {
        let mut touched = Vec::new();
        for leg in legs.iter().filter(|l| matches!(l.kind, LegKind::Money)) {
            let AccountRef::Line { line, side: Side::Asset } = leg.account else { continue };
            let at = self.parties.row(leg.party);
            let rec = records.get_or_insert_with(leg.party, at, || self.record_of(leg.party, line));
            if rec.account != line {
                violation!(clause = "MON.5", "a party paying from two accounts in one day", party = leg.party.get());
            }
            // A payment taken away leaves the column it was booked in: debits stay what the standing payments draw.
            let q = i128::from(leg.qty);
            if q < 0 {
                rec.debit -= q * sign;
            } else {
                rec.credit += q * sign;
            }
            touched.push(leg.party);
        }
        touched
    }

    /// Stage 7a: one stream over the run heads filed as due by today, holder-major in table and slot order. A holder
    /// whose head has since moved costs that one read; on its head's day its segment's rows on lines due today are
    /// read, and each claimant's row adds its payment's debits and credits to the records of the accounts it touches.
    /// The heads are read in a fixed number of shards, on the pool where the books have one, each shard's payments
    /// reckoned apart; the shards are then booked in the stream's order, so the result is the same with any workers.
    #[clause("MON.5", "REP.9", "SET.6")]
    pub(crate) fn stream(
        &self,
        heads: &[u32],
        mut out: DayRecords,
        due: &DueLines,
        (day, calendar): (Day, &Calendar),
        closed: &Closed,
        found: &mut Found,
    ) -> DayRecords
    where
        B: Sync,
    {
        for &line in due.lines() {
            if found.plans.get(line).is_none() {
                let terms = self.ledger.terms.get(self.ledger.lines.terms(line));
                let _ = found.plans.insert(line, due_plan(terms, Some(self.ledger.lines.fallen(line)), day, calendar));
            }
        }
        let plans = &found.plans;
        let each = heads.len().div_ceil(STREAM_SHARDS);
        let shards = phx_exec::pool::map(self.pool.as_deref(), STREAM_SHARDS, |shard| {
            let from = at_most(shard * each, heads.len());
            let to = at_most(from + each, heads.len());
            self.stream_shard(heads.get(from..to).unwrap_or(&[]), due, (day, calendar), plans)
        });
        for (read, steps) in shards {
            out.heads_read += read;
            for step in steps {
                let (p, legs) = match step {
                    Step::Scanned { place, slot, read, due } => {
                        out.scanned.push((place, slot));
                        out.rows_scanned += read;
                        out.rows_due += due;
                        continue;
                    }
                    Step::Paid(p, legs) => (p, legs),
                    Step::Cleared { holder, row, per, ccy, order } => {
                        let Some(p) = self.cleared_payment(holder, &row, per, (ccy, order), found) else { continue };
                        let legs = if p.moneyless { Vec::new() } else { self.effects(&p) };
                        (p, legs)
                    }
                };
                out.payments += 1;
                out.made.push(p);
                out.leg_at.push(out.legs.len());
                out.legs.extend_from_slice(&legs);
                if p.moneyless {
                    out.moneyless.insert(p.reckoned_on);
                    continue;
                }
                if closed.holds(p.payer, &issuers(&legs)) {
                    if p.cleared {
                        violation!(
                            clause = "MON.5",
                            "a cleared line's payment through a closed bank, which waits for resolution (sys-sup)",
                            line = p.line.get()
                        );
                    }
                    out.pending += 1;
                    continue;
                }
                let _ = self.book(&mut out.records, &legs, 1);
                out.gross += i128::from(p.amount);
            }
        }
        out
    }

    /// One shard of 7a's heads: how many were read, and what each due holder's segment makes, in order.
    fn stream_shard(
        &self,
        heads: &[u32],
        due: &DueLines,
        (day, calendar): (Day, &Calendar),
        plans: &KernelMap<LineId, DuePlan>,
    ) -> (u64, Vec<Step>) {
        let keys = self.ledger.lines.keys();
        let (mut read, mut steps) = (0_u64, Vec::new());
        for &key in heads {
            let (place, slot) = keys.split(key);
            let table = self.parties.holder(place);
            if !phx_store::table::live_at(table.live_words(), slot) {
                continue;
            }
            read += 1;
            let Some((rows, scanned)) = runs::due_rows(table, slot, day, due) else { continue };
            steps.push(Step::Scanned { place, slot, read: scanned, due: count(rows.len()) });
            let holder = table.party(slot);
            for row in rows {
                match self.reckon(holder, &row, (day, calendar), plans.get(row.row.line)) {
                    None => {}
                    Some(Reckoned::Paid(p)) => {
                        let legs = if p.moneyless { Vec::new() } else { self.effects(&p) };
                        steps.push(Step::Paid(p, legs));
                    }
                    Some(Reckoned::Cleared { per, ccy, order }) => {
                        steps.push(Step::Cleared { holder, row, per, ccy, order });
                    }
                }
            }
        }
        (read, steps)
    }
}
