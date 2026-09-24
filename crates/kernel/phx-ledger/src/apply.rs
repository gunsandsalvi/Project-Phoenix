use std::collections::BTreeSet;

use phx_core::{AuditStream, SubStep};
use phx_id::{Day, InstrumentId, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Missing, Money, Qty, capacity_exceeded, violation};
use phx_store::{Backing, SystemBacking};

use crate::algebra::Side;
use crate::check::{FailCause, Position, check_legs, unbalanced};
use crate::commitment::Commitments;
use crate::contract_process::Arrears;
use crate::covered::{Covered, Covers};
use crate::effects::EffectRec;
use crate::events::InstrumentEvents;
use crate::fails::Fail;
use crate::holder::HolderArenas;
use crate::holding::{Disposal, Lot, LotOrder, holding};
use crate::instruction::{AccountRef, Denom, DueRow, Instruction, InstructionId, LegKind, LegRec, Reasons, RowOp};
use crate::instrument::{Instruments, IssueChange};
use crate::lien::Liens;
use crate::line::Lines;
use crate::rows::{Optional, RowView, rows};
use crate::terms::TermsInterner;
use crate::units::{NamedUnit, add, named, take};

/// Where a party an instruction names is: live, or followed to the successor it ended into, in a holder table at a
/// slot; or ended with no successor to answer for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Located {
    Live { party: PartyId, table: u16, slot: Slot },
    Ended,
}

/// The world's holders as the apply reaches them.
pub trait Holders {
    /// A party, followed to its successor if it ended during the day, and where its rows are.
    fn locate(&self, party: PartyId) -> Located;
    /// A holder table by its place among the holder tables.
    fn arenas(&mut self, table: u16) -> &mut dyn HolderArenas;
}

/// When an instruction applies: before day one, where only the opening writes, or at a sub-step of a day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApplyAt {
    Opening,
    Day(SubStep),
}

/// The sub-steps at which money moves: stage 2's settlement of the day's fails and retries, banknotes paid at 6d,
/// stages 7 and 8, and a cell's own totals at landing.
pub const MONEY_SUBSTEPS: &[SubStep] = &[
    SubStep::S2c,
    SubStep::S6d,
    SubStep::S7a,
    SubStep::S7b,
    SubStep::S7c,
    SubStep::S7d,
    SubStep::S7e,
    SubStep::S8a,
    SubStep::S8b,
    SubStep::S8c,
    SubStep::S8d,
    SubStep::S8e,
    SubStep::S10b,
];

/// What a day's settlement leaves for the close: its fails, for the contract processes, and its accounting effects.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DayBook {
    pub fails: Vec<Fail>,
    pub effects: Vec<EffectRec>,
}

/// The world's books: everything the ledger keeps, and the one routine by which instructions change it.
#[derive(Debug)]
pub struct Ledger<B: Backing = SystemBacking> {
    pub instruments: Instruments<B>,
    pub lines: Lines<B>,
    pub terms: TermsInterner,
    pub liens: Liens,
    pub covers: Covers,
    pub commitments: Commitments,
    pub events: InstrumentEvents,
    pub reasons: Reasons,
    pub(crate) arrears: Arrears,
    applied: BTreeSet<InstructionId>,
    day: DayBook,
}

/// What every leg of an instruction settles with: its identity, reason and day, and the contract row it pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settling {
    id: InstructionId,
    reason: crate::instruction::ReasonId,
    day: Day,
    pays: Missing<DueRow>,
}

/// A party as located: itself or its successor, its holder table's place and its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct At {
    party: PartyId,
    table: u16,
    slot: Slot,
}

/// A position a leg draws on, by who holds it and what.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Key {
    table: u16,
    slot: Slot,
    account: AccountRef,
}

fn find(arenas: &dyn HolderArenas, slot: Slot, line: LineId, side: Side) -> RowView {
    let Some(view) = rows(arenas, slot).into_iter().find(|r| r.row.line == line && r.side() == side) else {
        violation!(clause = "SET.11", "a leg on a row its party does not have", line = line.get());
    };
    view
}

fn balance(view: &RowView, line: LineId) -> i64 {
    let Missing::Present(b) = view.optional.balance else {
        violation!(clause = "SET.11", "a leg moving a balance on a row that keeps none", line = line.get());
    };
    b
}

impl<B: Backing> Ledger<B> {
    #[must_use]
    pub fn new(instruments: Instruments<B>, lines: Lines<B>) -> Ledger<B> {
        Ledger {
            instruments,
            lines,
            terms: TermsInterner::default(),
            liens: Liens::default(),
            covers: Covers::default(),
            commitments: Commitments::default(),
            events: InstrumentEvents::default(),
            reasons: Reasons::default(),
            arrears: Arrears::default(),
            applied: BTreeSet::new(),
            day: DayBook::default(),
        }
    }

    /// Instructions of one sub-step applied in declared order: by their reasons' payment order, then by identity, so two
    /// drawing on one balance never both succeed by luck.
    #[clause("SET.6")]
    pub fn settle(
        &mut self,
        holders: &mut dyn Holders,
        at: ApplyAt,
        mut list: Vec<Instruction>,
        audit: &mut dyn AuditStream,
    ) -> Vec<Result<InstructionId, Fail>> {
        list.sort_by_key(|i| (self.reasons.get(i.reason).order, i.id));
        list.into_iter().map(|i| self.apply(holders, at, i, audit)).collect()
    }

    /// The one apply routine: an instruction's legs checked against what their parties hold, then all applied or none,
    /// a fail recorded with its cause; its accounting effects kept for the accounts; the audit told what it applied and
    /// touched. An instruction applied twice, with one side, or moving money where money does not move stops the run.
    ///
    /// # Errors
    /// The fail, when the instruction could not settle.
    #[clause("SET.4", "SET.5", "SET.7", "SET.11", "MON.5", "MON.6")]
    pub fn apply(
        &mut self,
        holders: &mut dyn Holders,
        at: ApplyAt,
        instruction: Instruction,
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        let Instruction { id, reason, settle_day, legs, pays, covers, .. } = instruction;
        let settling = Settling { id, reason, day: settle_day, pays };
        if !self.applied.insert(id) {
            violation!(clause = "SET.11", "an instruction applied twice", id = id.get());
        }
        Self::refuse(at, &legs, id);
        let mut located = Vec::with_capacity(legs.len());
        for leg in &legs {
            match holders.locate(leg.party) {
                Located::Live { party, table, slot } => located.push(At { party, table, slot }),
                Located::Ended => return Err(self.fail(settling, FailCause::Ended, leg, covers)),
            }
        }
        let mut keys: Vec<Key> = Vec::new();
        let mut positions: Vec<Position> = Vec::new();
        let mut moves: Vec<(usize, i64)> = Vec::new();
        for (leg, at) in legs.iter().zip(&located) {
            let key = Key { table: at.table, slot: at.slot, account: leg.account };
            let Some((position, delta)) = self.position(holders.arenas(at.table), at.party, at.slot, leg) else {
                continue;
            };
            let at = if let Some(i) = keys.iter().position(|k| *k == key) {
                i
            } else {
                keys.push(key);
                positions.push(position);
                positions.len() - 1
            };
            moves.push((at, delta));
        }
        let leg_of = |m: usize| moves.get(m).and_then(|(p, _)| keys.get(*p)).map(|k| k.account);
        if let Err((cause, m)) = check_legs(&positions, &moves) {
            let failing = legs.iter().find(|l| Some(l.account) == leg_of(m)).map_or(legs.first(), Some);
            let Some(leg) = failing.copied() else {
                violation!(clause = "SET.11", "an instruction with no legs", id = id.get());
            };
            return Err(self.fail(settling, cause, &leg, covers));
        }
        self.settle_legs(holders, settling, &legs, &located, audit);
        for c in covers {
            self.covers.release(c);
        }
        audit.applied(id.get());
        Ok(id)
    }

    fn refuse(at: ApplyAt, legs: &[LegRec], id: InstructionId) {
        for leg in legs {
            let opening = matches!(leg.kind, LegKind::OpeningWrite { .. });
            match at {
                ApplyAt::Day(_) if opening => {
                    violation!(clause = "SET.9", "an opening write after the opening", id = id.get());
                }
                ApplyAt::Day(s) if leg.moves_money() && !MONEY_SUBSTEPS.contains(&s) => {
                    violation!(clause = "SET.11", "money moved at a sub-step where money does not move", id = id.get());
                }
                ApplyAt::Opening if !opening => {
                    violation!(clause = "SET.1", "an instruction before day one that is not an opening write");
                }
                ApplyAt::Day(_) | ApplyAt::Opening => {}
            }
        }
        if let Some(d) = unbalanced(legs) {
            let denom = match d {
                Denom::Ccy(c) => u32::from(c.index()),
                Denom::Unit(u) => u32::from(u.index()),
            };
            violation!(clause = "SET.11", "an instruction with one side", id = id.get(), denom = denom);
        }
    }

    fn fail(&mut self, s: Settling, cause: FailCause, leg: &LegRec, covers: Vec<Covered>) -> Fail {
        for c in covers {
            self.covers.release(c);
        }
        let f = Fail { instruction: s.id, reason: s.reason, cause, party: leg.party, due: s.day, row: s.pays };
        self.day.fails.push(f);
        f
    }

    /// What a leg draws on and how it moves it; a leg that can only add, or opens a row, draws on nothing.
    fn position(&self, arenas: &dyn HolderArenas, party: PartyId, slot: Slot, leg: &LegRec) -> Option<(Position, i64)> {
        match (leg.kind, leg.account) {
            (LegKind::Money | LegKind::Row(RowOp::Adjust), AccountRef::Line { line, side }) => {
                let view = find(arenas, slot, line, side);
                let b = balance(&view, line);
                let terms = self.terms.get(self.lines.terms(line));
                if let (LegKind::Money, Denom::Ccy(c)) = (leg.kind, leg.denom)
                    && c != terms.ccy
                {
                    violation!(
                        clause = "MON.13",
                        "money paid in another currency than its account's",
                        line = line.get()
                    );
                }
                let money_holder = matches!(leg.kind, LegKind::Money) && side == Side::Asset;
                if !money_holder {
                    return Some((Position { now: b, floor: Missing::Absent, short: FailCause::Funds }, leg.qty));
                }
                let pending = match view.optional.pending {
                    Missing::Present(p) => p,
                    Missing::Absent => 0,
                };
                let floor = match terms.facility {
                    Missing::Present(f) => {
                        let Some(limit) = f.limit.amt().checked_mul(i64::from(view.row.count)) else {
                            capacity_exceeded!("a facility's limit over a row's members", i64::MAX, f.limit.amt());
                        };
                        -limit
                    }
                    Missing::Absent => 0,
                };
                Some((Position { now: b - pending, floor: Missing::Present(floor), short: FailCause::Funds }, leg.qty))
            }
            (
                LegKind::Units { .. } | LegKind::Transformation(_) | LegKind::OpeningWrite { .. },
                AccountRef::Instrument(id),
            ) => {
                let inst = self.instruments.get(id);
                if leg.denom != Denom::Unit(inst.unit) {
                    violation!(
                        clause = "NUM.5",
                        "units counted in another unit than their instrument's",
                        id = id.get()
                    );
                }
                let issuing = matches!(leg.kind, LegKind::Units { .. }) && inst.issuer == Missing::Present(party);
                if issuing {
                    let now = inst.issued.n();
                    return Some((Position { now, floor: Missing::Present(0), short: FailCause::FreeUnits }, -leg.qty));
                }
                let held = match holding(arenas, slot, id) {
                    Missing::Present(h) => h.quantity.raw(),
                    Missing::Absent => 0,
                };
                let free = held - self.bound(party, id);
                Some((Position { now: free, floor: Missing::Present(0), short: FailCause::FreeUnits }, leg.qty))
            }
            (LegKind::Units { .. }, AccountRef::Unit(unit)) => {
                let now = i64::from(named(arenas, slot).iter().any(|u| u.id == unit));
                Some((Position { now, floor: Missing::Present(0), short: FailCause::NotHeld }, leg.qty))
            }
            (LegKind::OpeningWrite { .. }, AccountRef::Line { line, side }) => {
                let b = balance(&find(arenas, slot, line, side), line);
                Some((Position { now: b, floor: Missing::Absent, short: FailCause::Funds }, leg.qty))
            }
            (LegKind::Row(RowOp::Close), AccountRef::Line { line, side }) => {
                let _ = find(arenas, slot, line, side);
                None
            }
            (LegKind::Row(RowOp::Open(_)), AccountRef::Line { .. }) => None,
            _ => violation!(clause = "SET.11", "a leg whose kind does not fit its account", party = party.get()),
        }
    }

    /// What of a holding is bound elsewhere: pledged, or covering an open offer.
    fn bound(&self, party: PartyId, id: InstrumentId) -> i64 {
        self.liens.pledged(party, id) + self.covers.committed(party, id)
    }

    fn settle_legs(
        &mut self,
        holders: &mut dyn Holders,
        s: Settling,
        legs: &[LegRec],
        located: &[At],
        audit: &mut dyn AuditStream,
    ) {
        let decl = self.reasons.get(s.reason);
        let mut taken: Vec<NamedUnit> = Vec::new();
        let order: Vec<usize> = (0..legs.len())
            .filter(|i| legs.get(*i).is_some_and(|l| l.qty < 0))
            .chain((0..legs.len()).filter(|i| legs.get(*i).is_some_and(|l| l.qty >= 0)))
            .collect();
        for i in order {
            let (Some(leg), Some(at)) = (legs.get(i), located.get(i)) else { continue };
            let arenas = holders.arenas(at.table);
            self.settle_leg(arenas, *at, leg, s.day, &mut taken);
            audit.touched(arenas.table(), at.slot);
            if let (LegKind::Money, Denom::Ccy(ccy)) = (leg.kind, leg.denom) {
                let effect = if leg.qty < 0 { decl.paid } else { decl.received };
                let amount = Money::new(leg.qty.abs(), ccy);
                self.day.effects.push(EffectRec { instruction: s.id, party: at.party, effect, amount });
            }
        }
        if !taken.is_empty() {
            violation!(clause = "SET.11", "a named unit given that nobody received", id = s.id.get());
        }
    }

    fn settle_leg(
        &mut self,
        arenas: &mut dyn HolderArenas,
        at: At,
        leg: &LegRec,
        day: Day,
        taken: &mut Vec<NamedUnit>,
    ) {
        let At { party, slot, .. } = at;
        match (leg.kind, leg.account) {
            (
                LegKind::Money | LegKind::Row(RowOp::Adjust) | LegKind::OpeningWrite { .. },
                AccountRef::Line { line, side },
            ) => {
                let view = find(arenas, slot, line, side);
                let Some(next) = balance(&view, line).checked_add(leg.qty) else {
                    violation!(clause = "Law 7", "a balance overflows", line = line.get());
                };
                let optional = Optional { balance: Missing::Present(next), ..view.optional };
                self.lines.set_words(arenas, slot, line, side, optional);
            }
            (LegKind::Units { cost }, AccountRef::Instrument(id))
                if self.instruments.get(id).issuer == Missing::Present(party) =>
            {
                let _ = cost;
                let why = if leg.qty < 0 { IssueChange::Issuance } else { IssueChange::Buyback };
                self.instruments.change_issued(id, Qty::new(-leg.qty, self.instruments.get(id).unit), why);
            }
            (LegKind::Units { cost }, AccountRef::Instrument(id)) => {
                self.move_units(arenas, at, id, leg.qty, cost, day);
            }
            (LegKind::Transformation(_) | LegKind::OpeningWrite { .. }, AccountRef::Instrument(id)) => {
                let why = if leg.qty > 0 { IssueChange::Issuance } else { IssueChange::Buyback };
                self.instruments.change_issued(id, Qty::new(leg.qty, self.instruments.get(id).unit), why);
                self.move_units(arenas, at, id, leg.qty, 0, day);
            }
            (LegKind::Units { .. }, AccountRef::Unit(unit)) => {
                if leg.qty < 0 {
                    taken.push(take(arenas, slot, unit));
                } else {
                    let Some(at) = taken.iter().position(|u| u.id == unit) else {
                        violation!(clause = "SET.11", "a named unit received that nobody gave", unit = unit);
                    };
                    add(arenas, slot, taken.swap_remove(at));
                }
            }
            (LegKind::Row(RowOp::Open(new)), AccountRef::Line { line, .. }) => {
                if i64::from(new.count) != leg.qty {
                    violation!(clause = "SET.11", "a row opened with another count than its leg's", line = line.get());
                }
                self.lines.add_row(arenas, at.table, slot, line, new);
            }
            (LegKind::Row(RowOp::Close), AccountRef::Line { line, side }) => {
                self.lines.remove_row(arenas, at.table, slot, line, side);
            }
            _ => violation!(clause = "SET.11", "a leg whose kind does not fit its account", party = party.get()),
        }
    }

    fn move_units(&mut self, arenas: &mut dyn HolderArenas, at: At, id: InstrumentId, qty: i64, cost: i64, day: Day) {
        if qty > 0 {
            self.instruments.acquire(arenas, at.table, at.slot, id, Lot::new(day, qty, cost));
        } else if qty < 0 {
            let disposal = Disposal { units: -qty, bound: self.bound(at.party, id), order: LotOrder::FirstIn };
            let _ = self.instruments.dispose(arenas, at.table, at.slot, id, disposal);
        }
    }

    /// The day's fails and effects, handed to the close; instructions live until then, and no identity recurs, so the
    /// applied set starts again.
    pub fn close(&mut self) -> DayBook {
        self.applied.clear();
        core::mem::take(&mut self.day)
    }

    /// The rows in arrears.
    #[must_use]
    pub fn arrears(&self) -> &Arrears {
        &self.arrears
    }

    /// The fails recorded so far today.
    #[must_use]
    pub fn fails(&self) -> &[Fail] {
        &self.day.fails
    }
}
