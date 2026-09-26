use std::collections::{BTreeMap, BTreeSet};

use phx_core::{AuditStream, LegDigest, SubStep};
use phx_id::{Day, InstrumentId, LineId, PartyId, Slot};
use phx_macros::clause;
use phx_num::{Missing, Money, Qty, capacity_exceeded, violation};
use phx_store::{Backing, SystemBacking};

use crate::algebra::Side;
use crate::check::{FailCause, Position, check_legs, unbalanced};
use crate::commitment::Commitments;
use crate::contract_process::{Arrears, ArrearsKey};
use crate::covered::{Covered, Covers};
use crate::effects::EffectRec;
use crate::events::InstrumentEvents;
use crate::fails::Fail;
use crate::holder::HolderArenas;
use crate::holding::{Disposal, Lot, LotOrder, holding};
use crate::instruction::{
    AccountRef, Denom, DueRow, Instruction, InstructionId, LegKind, LegRec, ROW_COUNT, Reasons, RowOp,
};
use crate::instrument::{InstrumentFamily, Instruments, IssueChange};
use crate::lien::Liens;
use crate::line::Lines;
use crate::rows::{Optional, RowView};
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
/// and stages 7 and 8.
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

/// A party's money moved in one currency on a day, the key of what it paid and received.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Moved {
    ccy: u8,
    party: PartyId,
}

impl phx_core::MapKey for Moved {
    fn key64(self) -> u64 {
        (self.party.get() << u8::BITS) | u64::from(self.ccy)
    }
}

/// What a day's settlement leaves for the close: its fails, for the contract processes, its accounting effects, and
/// what moved in each currency, gross and by party.
#[derive(Debug, Default)]
pub struct DayBook {
    pub fails: Vec<Fail>,
    pub effects: Vec<EffectRec>,
    /// The dues of the day kept one by one, those that did not settle, for the claims each leaves.
    pub dues: Vec<crate::effects::DueRec>,
    /// The interest of the day's dues that settled, folded by party, the payee's earned and the payer's spent: a
    /// settled due leaves no claim, so the accounts read only what each party earned.
    pub earned: phx_core::KernelMap<PartyId, i128>,
    pub disposed: Vec<DisposedRec>,
    moved: phx_core::KernelMap<Moved, (i128, i128)>,
}

impl DayBook {
    /// Whether the day has recorded nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fails.is_empty()
            && self.effects.is_empty()
            && self.dues.is_empty()
            && self.earned.is_empty()
            && self.disposed.is_empty()
            && self.moved.is_empty()
    }

    /// What the day's records hold in memory, by their room.
    #[must_use]
    pub fn bytes(&self) -> usize {
        self.fails.capacity() * size_of::<Fail>()
            + self.effects.capacity() * size_of::<EffectRec>()
            + self.dues.capacity() * size_of::<crate::effects::DueRec>()
            + self.earned.capacity() * size_of::<(PartyId, i128)>()
            + self.disposed.capacity() * size_of::<DisposedRec>()
            + self.moved.capacity() * size_of::<(Moved, (i128, i128))>()
    }
}

/// Units that left a holding, with the cost their lots carried out, for the accounts to realise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DisposedRec {
    pub party: PartyId,
    pub instrument: InstrumentId,
    pub units: i64,
    pub cost: i64,
    pub day: Day,
}

/// A day's settlement as published: per currency, the value paid gross and the value that changed hands net of what
/// each party both paid and received, and the fails by cause.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Settlement {
    pub gross: BTreeMap<u8, i128>,
    pub net: BTreeMap<u8, i128>,
    pub fails: BTreeMap<FailCause, u64>,
}

impl DayBook {
    /// The day's settlement values and fails, measured from its book.
    #[clause("SET.10")]
    #[must_use]
    pub fn measure(&self) -> Settlement {
        let mut m = Settlement::default();
        for (Moved { ccy, .. }, (paid, received)) in self.moved.sorted() {
            *m.gross.entry(ccy).or_insert(0) += paid;
            let net = received - paid;
            if net > 0 {
                *m.net.entry(ccy).or_insert(0) += net;
            }
        }
        for f in &self.fails {
            *m.fails.entry(f.cause).or_insert(0) += 1;
        }
        m
    }
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
    /// The day of the instructions numbered last and how many; every instruction of a day draws its number here.
    pub(crate) numbered: (Day, u32),
    /// The insolvency procedures open, whose stays suspend the dues of their procedure lines.
    pub procedures: BTreeSet<u16>,
    /// The chains of classes units wear along.
    pub chains: crate::chains::Chains,
    pub(crate) day: DayBook,
}

/// What every leg of an instruction settles with: its identity, reason and day, and the contract row it pays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settling {
    id: InstructionId,
    reason: crate::instruction::ReasonId,
    day: Day,
    pays: Missing<DueRow>,
}

/// What a leg draws on before anything moves: the position its move is checked against, with the move.
pub(crate) type Drawn = Option<(Position, i64)>;
/// An instruction's legs as read before anything moves: where each leg's party is and what it draws, or the first leg
/// whose party has ended.
pub(crate) type LegReads = Result<(Vec<At>, Vec<Drawn>), usize>;

/// A party as located: itself or its successor, its holder table's place and its slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct At {
    pub(crate) party: PartyId,
    pub(crate) table: u16,
    pub(crate) slot: Slot,
}

/// A position a leg draws on, by who holds it and what: a row's member count apart from its balance, since the two
/// are counted in different denominations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    table: u16,
    slot: Slot,
    code: u64,
}

/// The rows an instruction opens, by party and account: a leg adjusting one moves a row that holds nothing yet.
pub(crate) fn opened(legs: &[LegRec]) -> Vec<(PartyId, AccountRef)> {
    legs.iter().filter(|l| matches!(l.kind, LegKind::Row(RowOp::Open(_)))).map(|l| (l.party, l.account)).collect()
}

/// A row's balance moved by a leg's quantity, written over the row as it was found.
fn settle_balance(arenas: &mut dyn HolderArenas, slot: Slot, (line, view): (LineId, &RowView), qty: i64) {
    let Some(next) = balance(view, line).checked_add(qty) else {
        violation!(clause = "Law 7", "a balance overflows", line = line.get());
    };
    let optional = Optional { balance: Missing::Present(next), ..view.optional };
    crate::rows::rewrite(arenas, slot, view, optional);
}

fn find(arenas: &dyn HolderArenas, slot: Slot, line: LineId, side: Side) -> RowView {
    let Some(view) = crate::rows::find(arenas, slot, line, side) else {
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
            numbered: (Day::new(0), 0),
            procedures: BTreeSet::new(),
            chains: crate::chains::Chains::default(),
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
        self.refuse(at, &legs, id);
        let mut located = Vec::with_capacity(legs.len());
        for leg in &legs {
            match holders.locate(leg.party) {
                Located::Live { party, table, slot } => located.push(At { party, table, slot }),
                Located::Ended => return Err(self.fail(settling, FailCause::Ended, leg, covers)),
            }
        }
        let opened = opened(&legs);
        let drawn: Vec<Drawn> = legs
            .iter()
            .zip(&located)
            .map(|(leg, at)| self.leg_draw(holders.arenas(at.table), *at, leg, &opened))
            .collect();
        self.check_and_settle(holders, (settling, covers), (&legs, &located, &drawn), audit)
    }

    /// The apply routine over legs already read: where each leg's party is and what it draws, or the first leg whose
    /// party has ended; the reads were made before anything moved, as `apply` makes them.
    ///
    /// # Errors
    /// The fail, when the instruction could not settle.
    pub(crate) fn apply_read(
        &mut self,
        holders: &mut dyn Holders,
        at: ApplyAt,
        instruction: Instruction,
        reads: LegReads,
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        let Instruction { id, reason, settle_day, legs, pays, covers, .. } = instruction;
        let settling = Settling { id, reason, day: settle_day, pays };
        if !self.applied.insert(id) {
            violation!(clause = "SET.11", "an instruction applied twice", id = id.get());
        }
        self.refuse(at, &legs, id);
        match reads {
            Ok((located, drawn)) => {
                self.check_and_settle(holders, (settling, covers), (&legs, &located, &drawn), audit)
            }
            Err(n) => {
                let Some(leg) = legs.get(n) else {
                    violation!(clause = "SET.7", "an ended party on no leg", id = id.get());
                };
                Err(self.fail(settling, FailCause::Ended, leg, covers))
            }
        }
    }

    /// What a leg draws on before anything moves: the position its move is checked against, with the move; none for a
    /// leg that moves no position. A row the instruction opens holds nothing until it does, and nothing bounds it.
    pub(crate) fn leg_draw(
        &self,
        arenas: &dyn HolderArenas,
        at: At,
        leg: &LegRec,
        opened: &[(PartyId, AccountRef)],
    ) -> Option<(Position, i64)> {
        if matches!(leg.kind, LegKind::Row(RowOp::Adjust)) && opened.contains(&(leg.party, leg.account)) {
            return Some((Position { now: 0, floor: Missing::Absent, short: FailCause::Funds }, leg.qty));
        }
        self.draws(arenas, at.party, at.slot, leg)
    }

    /// An instruction's moves checked against the positions its legs read, then all settled or none.
    fn check_and_settle(
        &mut self,
        holders: &mut dyn Holders,
        (settling, covers): (Settling, Vec<crate::covered::Covered>),
        (legs, located, drawn): (&[LegRec], &[At], &[Drawn]),
        audit: &mut dyn AuditStream,
    ) -> Result<InstructionId, Fail> {
        let id = settling.id;
        let mut keys: BTreeMap<Key, usize> = BTreeMap::new();
        let mut positions: Vec<Position> = Vec::new();
        let mut moves: Vec<(usize, i64)> = Vec::new();
        let mut moved_by: Vec<usize> = Vec::new();
        for (n, ((leg, at), d)) in legs.iter().zip(located).zip(drawn).enumerate() {
            let Some((position, delta)) = *d else {
                continue;
            };
            let key = Key { table: at.table, slot: at.slot, code: leg.position_code() };
            let at = *keys.entry(key).or_insert_with(|| {
                positions.push(position);
                positions.len() - 1
            });
            moves.push((at, delta));
            moved_by.push(n);
        }
        if let Err((cause, m)) = check_legs(&positions, &moves) {
            let Some(leg) = moved_by.get(m).and_then(|n| legs.get(*n)).copied() else {
                violation!(clause = "SET.4", "a failing move with no leg", id = id.get());
            };
            return Err(self.fail(settling, cause, &leg, covers));
        }
        self.settle_legs(holders, settling, legs, located, audit);
        for c in covers {
            self.covers.release(c);
        }
        audit.applied(id.get());
        Ok(id)
    }

    fn refuse(&self, at: ApplyAt, legs: &[LegRec], id: InstructionId) {
        for leg in legs {
            let opening = matches!(leg.kind, LegKind::OpeningWrite { .. });
            match at {
                ApplyAt::Day(_) if opening => {
                    violation!(clause = "SET.9", "an opening write after the opening", id = id.get());
                }
                ApplyAt::Day(s) if self.money(leg) && !MONEY_SUBSTEPS.contains(&s) => {
                    violation!(clause = "SET.11", "money moved at a sub-step where money does not move", id = id.get());
                }
                ApplyAt::Opening if !opening && !matches!(leg.kind, LegKind::Row(_)) => {
                    violation!(
                        clause = "SET.1",
                        "an instruction before day one that neither opens, moves nor writes a row"
                    );
                }
                ApplyAt::Day(_) | ApplyAt::Opening => {}
            }
        }
        self.chains.check(legs);
        if let Some(d) = unbalanced(legs) {
            let denom = match d {
                Denom::Ccy(c) => u32::from(c.index()),
                Denom::Unit(u) => u32::from(u.index()),
            };
            violation!(clause = "SET.11", "an instruction with one side", id = id.get(), denom = denom);
        }
    }

    /// Whether a leg moves money: on a money line, or banknotes between holders.
    fn money(&self, leg: &LegRec) -> bool {
        match leg.account {
            AccountRef::Instrument(id) => self.instruments.get(id).family == InstrumentFamily::Banknote,
            AccountRef::Line { .. } | AccountRef::Unit(_) => leg.moves_money(),
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
    fn draws(&self, arenas: &dyn HolderArenas, party: PartyId, slot: Slot, leg: &LegRec) -> Option<(Position, i64)> {
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
                LegKind::Units { .. } | LegKind::Transformation { .. } | LegKind::OpeningWrite { .. },
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
            (LegKind::Row(RowOp::Count), AccountRef::Line { line, side }) => {
                let now = i64::from(find(arenas, slot, line, side).row.count);
                Some((Position { now, floor: Missing::Present(0), short: FailCause::FreeUnits }, leg.qty))
            }
            _ => violation!(clause = "SET.11", "a leg whose kind does not fit its account", party = party.get()),
        }
    }

    /// What a party holds on the account a position code names: a row's balance, or its member count when the code
    /// is marked so; a holding's units, or for its issuer the units issued, as owed; whether a named unit is held.
    #[must_use]
    pub fn position(&self, arenas: &dyn HolderArenas, party: PartyId, slot: Slot, code: u64) -> i64 {
        let row = |line: LineId, side: Side| crate::rows::find(arenas, slot, line, side);
        match AccountRef::from_code(code) {
            AccountRef::Line { line, side } if code & ROW_COUNT != 0 => {
                row(line, side).map_or(0, |r| i64::from(r.row.count))
            }
            AccountRef::Line { line, side } => row(line, side).map_or(0, |r| balance(&r, line)),
            AccountRef::Instrument(id) if self.instruments.get(id).issuer == Missing::Present(party) => {
                -self.instruments.get(id).issued.n()
            }
            AccountRef::Instrument(id) => match holding(arenas, slot, id) {
                Missing::Present(h) => h.quantity.raw(),
                Missing::Absent => 0,
            },
            AccountRef::Unit(unit) => i64::from(named(arenas, slot).iter().any(|u| u.id == unit)),
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
        // Rows are opened before anything moves on them and retired after; between, what leaves goes before what
        // arrives.
        let rank = |l: &LegRec| {
            let (opens, closes) =
                (matches!(l.kind, LegKind::Row(RowOp::Open(_))), matches!(l.kind, LegKind::Row(RowOp::Close)));
            (closes, !opens, l.qty >= 0)
        };
        let mut order: Vec<usize> = (0..legs.len()).collect();
        order.sort_by_key(|i| legs.get(*i).map(rank));
        for i in order {
            let (Some(leg), Some(at)) = (legs.get(i), located.get(i)) else { continue };
            let arenas = holders.arenas(at.table);
            // A leg moving a line's balance reads its row once, for what it held before and what it holds after.
            let line_row = match (leg.kind, leg.account) {
                (
                    LegKind::Money | LegKind::Row(RowOp::Adjust) | LegKind::OpeningWrite { .. },
                    AccountRef::Line { line, side },
                ) => Some((line, find(arenas, at.slot, line, side))),
                _ => None,
            };
            let before = match &line_row {
                Some((line, view)) => balance(view, *line),
                None => self.position(arenas, at.party, at.slot, leg.position_code()),
            };
            let basis = self.held_basis(arenas, *at, leg);
            match &line_row {
                Some((line, view)) => settle_balance(arenas, at.slot, (*line, view), leg.qty),
                None => self.settle_leg(arenas, *at, leg, s.day, &mut taken),
            }
            audit.touched(arenas.table(), at.slot);
            let money = matches!(leg.kind, LegKind::Money);
            let digest = LegDigest {
                party: at.party,
                account: leg.position_code(),
                denom: leg.denom.code(),
                qty: leg.qty,
                flow: crate::check::flow(leg),
                before,
                paired: leg.paired(),
                money,
                made: match leg.kind {
                    LegKind::Transformation { source: crate::instruction::Source::Way(way), .. } => {
                        Missing::Present(way)
                    }
                    _ => Missing::Absent,
                },
                worn: match (leg.kind, leg.account) {
                    (
                        LegKind::Transformation { source: crate::instruction::Source::Wear(_), .. },
                        AccountRef::Instrument(id),
                    ) => match self.chains.of(id) {
                        Missing::Present((chain, class)) => Missing::Present((chain, narrow_class(class))),
                        Missing::Absent => Missing::Absent,
                    },
                    _ => Missing::Absent,
                },
            };
            audit.leg(s.id.get(), digest);
            if let (LegKind::Money, Denom::Ccy(ccy)) = (leg.kind, leg.denom) {
                let effect = if leg.qty < 0 { decl.paid } else { decl.received };
                let amount = Money::new(leg.qty, ccy);
                self.day.effects.push(EffectRec { instruction: s.id, party: at.party, effect, amount });
                if matches!(leg.account, AccountRef::Line { side: Side::Asset, .. }) {
                    let key = Moved { ccy: ccy.index(), party: at.party };
                    let (paid, received) = self.day.moved.get_or_insert_with(key, || (0, 0));
                    if leg.qty < 0 {
                        *paid += i128::from(-leg.qty);
                    } else {
                        *received += i128::from(leg.qty);
                    }
                }
            }
            // What else the leg moved of its party's net assets: a row's balance, or the cost of a holding's lots.
            let worth = match (leg.kind, leg.denom, basis) {
                (LegKind::Row(RowOp::Adjust), Denom::Ccy(ccy), _) => Missing::Present((leg.qty, ccy)),
                (_, _, Missing::Present((was, ccy))) => match self.held_basis(arenas, *at, leg) {
                    Missing::Present((now, _)) => Missing::Present((now - was, ccy)),
                    Missing::Absent => Missing::Absent,
                },
                _ => Missing::Absent,
            };
            if let Missing::Present((moved, ccy)) = worth
                && moved != 0
            {
                let effect = if moved < 0 { decl.paid } else { decl.received };
                let amount = Money::new(moved, ccy);
                self.day.effects.push(EffectRec { instruction: s.id, party: at.party, effect, amount });
            }
        }
        if !taken.is_empty() {
            violation!(clause = "SET.11", "a named unit given that nobody received", id = s.id.get());
        }
    }

    /// What a holder's lots of the instrument a leg moves cost, in the instrument's currency, where the leg moves
    /// units a holder holds rather than an issuer's.
    fn held_basis(&self, arenas: &dyn HolderArenas, at: At, leg: &LegRec) -> Missing<(i64, phx_num::Ccy)> {
        let AccountRef::Instrument(id) = leg.account else { return Missing::Absent };
        let instrument = self.instruments.get(id);
        if instrument.issuer == Missing::Present(at.party) {
            return Missing::Absent;
        }
        // A holding not held has no lots, so costs nothing.
        let cost = match crate::holding::basis(arenas, at.slot, id) {
            Missing::Present(b) => b,
            Missing::Absent => 0,
        };
        Missing::Present((cost, instrument.ccy))
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
                settle_balance(arenas, slot, (line, &view), leg.qty);
            }
            (LegKind::Units { .. }, AccountRef::Instrument(id))
                if self.instruments.get(id).issuer == Missing::Present(party) =>
            {
                let why = if leg.qty < 0 { IssueChange::Issuance } else { IssueChange::Buyback };
                self.instruments.change_issued(id, Qty::new(-leg.qty, self.instruments.get(id).unit), why);
            }
            (LegKind::Units { cost }, AccountRef::Instrument(id)) => {
                self.move_units(arenas, at, id, leg.qty, cost, day);
            }
            (LegKind::Transformation { .. } | LegKind::OpeningWrite { .. }, AccountRef::Instrument(id)) => {
                let why = if leg.qty > 0 { IssueChange::Issuance } else { IssueChange::Buyback };
                self.instruments.change_issued(id, Qty::new(leg.qty, self.instruments.get(id).unit), why);
                let cost = match leg.kind {
                    LegKind::OpeningWrite { cost, .. } | LegKind::Transformation { cost, .. } => cost,
                    _ => 0,
                };
                if leg.qty < 0 && cost != 0 {
                    violation!(clause = "ACC.6", "units used up that carry a cost of their own", id = id.get());
                }
                self.move_units(arenas, at, id, leg.qty, cost, day);
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
                if -i64::from(find(arenas, slot, line, side).row.count) != leg.qty {
                    violation!(
                        clause = "SET.11",
                        "a row retired with another count than its members",
                        line = line.get()
                    );
                }
                self.lines.remove_row(arenas, at.table, slot, line, side);
                // A retired row's arrears retire with it; a transfer carries them to the row that takes its members.
                self.arrears.remove(ArrearsKey::new(line, side, party));
            }
            (LegKind::Row(RowOp::Count), AccountRef::Line { line, side }) => {
                let count = i64::from(find(arenas, slot, line, side).row.count) + leg.qty;
                let Ok(count) = u32::try_from(count) else {
                    violation!(
                        clause = "REG.14",
                        "a row's member count below nothing or beyond its width",
                        line = line.get()
                    );
                };
                self.lines.set_count(arenas, slot, line, side, count);
            }
            _ => violation!(clause = "SET.11", "a leg whose kind does not fit its account", party = party.get()),
        }
    }

    fn move_units(&mut self, arenas: &mut dyn HolderArenas, at: At, id: InstrumentId, qty: i64, cost: i64, day: Day) {
        if qty > 0 {
            // A class of a chain holds alike units, so its holding keeps one lot at average cost.
            let pooled = matches!(self.chains.of(id), Missing::Present(_));
            self.instruments.acquire_as(arenas, (at.table, at.slot), id, Lot::new(day, qty, cost), pooled);
        } else if qty < 0 {
            let disposal = Disposal { units: -qty, bound: self.bound(at.party, id), order: LotOrder::FirstIn };
            let gone = self.instruments.dispose(arenas, at.table, at.slot, id, disposal);
            self.day.disposed.push(DisposedRec { party: at.party, instrument: id, units: -qty, cost: gone.cost, day });
        }
    }

    /// The day's book so far, for the accounts to read.
    #[must_use]
    pub fn day_book(&self) -> &DayBook {
        &self.day
    }

    /// The day's fails and effects, handed to the close; instructions live until then, and no identity recurs, so the
    /// applied set starts again.
    pub fn close(&mut self) -> DayBook {
        self.applied.clear();
        core::mem::take(&mut self.day)
    }

    /// The opening's records forgotten: the audit and the accounts read days, and the opening is none; the accounts
    /// open on what the opening wrote.
    pub fn opened(&mut self) {
        self.day = DayBook::default();
    }

    /// The next instruction's identity on a day: its day, and its place in that day's numbering.
    pub fn next_id(&mut self, day: Day) -> InstructionId {
        if self.numbered.0 != day {
            self.numbered = (day, 0);
        }
        let id = InstructionId::new(day, self.numbered.1);
        let Some(next) = self.numbered.1.checked_add(1) else {
            capacity_exceeded!("instructions of one day", u32::MAX, self.numbered.1);
        };
        self.numbered.1 = next;
        id
    }

    /// A fail the day's settlement found without applying an instruction: a payment the fixed point removed.
    pub(crate) fn record_fail(&mut self, f: Fail) {
        self.day.fails.push(f);
    }

    /// A due that fell today, and what became of it, for the accounts.
    pub(crate) fn record_due(&mut self, d: crate::effects::DueRec) {
        self.day.dues.push(d);
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

impl<B: Backing> Ledger<B> {
    /// The ledger for a save, taken at a day's close: its instruments, lines, terms, liens, covers, commitments,
    /// arrears and procedures, and its reasons' names, which a load checks against the build's. The day's book and
    /// the instructions applied live for the day, so a save finding them is a contract violation.
    #[clause("SET.12", "SET.13")]
    pub(crate) fn save_to(&self, w: &mut phx_store::Writer<'_>) {
        use phx_store::Saved as _;
        if !self.applied.is_empty() || !self.day.is_empty() {
            violation!(clause = "SET.13", "a save taken while the day's instructions still live");
        }
        self.reasons.names().collect::<Vec<_>>().save(w);
        self.instruments.save_to(w);
        self.lines.save_to(w);
        self.terms.save(w);
        self.liens.save(w);
        self.covers.save(w);
        self.commitments.save(w);
        self.arrears.save(w);
        self.procedures.save(w);
        self.chains.save(w);
    }

    /// The ledger read back over the build's own declarations, which `decls` carries from the declarations phase.
    ///
    /// # Errors
    /// When the store is damaged or its declarations are not the build's.
    pub(crate) fn load_from(
        r: &mut phx_store::Reader<'_>,
        mut decls: Ledger<B>,
        keys: crate::holder::HolderKeys,
    ) -> Result<Ledger<B>, phx_store::LoadError> {
        use phx_store::Saved as _;
        let reasons: Vec<&'static str> = phx_store::Saved::load(r)?;
        if !reasons.iter().copied().eq(decls.reasons.names()) {
            return Err(phx_store::LoadError::Invalid("reasons other than the build's".to_owned()));
        }
        let instruments = Instruments::load_from(r, keys)?;
        let lines = Lines::load_from(r, decls.lines.take_decls(), keys)?;
        Ok(Ledger {
            instruments,
            lines,
            terms: TermsInterner::load(r)?,
            liens: Liens::load(r)?,
            covers: Covers::load(r)?,
            commitments: Commitments::load(r)?,
            events: core::mem::take(&mut decls.events),
            reasons: core::mem::take(&mut decls.reasons),
            arrears: Arrears::load(r)?,
            applied: BTreeSet::new(),
            numbered: (Day::new(0), 0),
            procedures: BTreeSet::load(r)?,
            chains: crate::chains::Chains::load(r)?,
            day: DayBook::default(),
        })
    }
}

/// A class's place in its chain, which a chain's few classes keep far below the digest's width.
fn narrow_class(class: usize) -> u32 {
    let Ok(c) = u32::try_from(class) else {
        phx_num::capacity_exceeded!("classes of a chain", u32::MAX, class);
    };
    c
}
