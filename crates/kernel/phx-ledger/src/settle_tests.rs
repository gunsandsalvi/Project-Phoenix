//! The apply routine over real kind tables, whose arenas hold the rows it moves.
#![cfg(test)]

use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::kind_tables::KindTable;
use phx_core::{AuditStream, LegDigest, SubStep};
use phx_id::{CountryId, Date, Day, InstrumentId, LineId, PartyId, Slot, TableId};
use phx_num::{Ccy, Missing, Money, Rate, RatePeriod, Round, UnitId};
use phx_store::{AddressSpace, HeapBacking};

use crate::algebra::{DefaultDefinition, Facility, PaymentOrder, Schedule, Seniority, Side, Termination, Terms};
use crate::apply::{ApplyAt, Holders, Ledger, Located};
use crate::check::FailCause;
use crate::holder::{HolderArenas, HolderKeys};
use crate::holding::holding;
use crate::instruction::{
    AccountRef, Denom, DueRow, Effect, Instruction, InstructionId, LegKind, LegRec, ReasonDecl, ReasonId, RowOp, Source,
};
use crate::instrument::{InstrumentFamily, Instruments, NewInstrument};
use crate::line::{LineKindDecl, Lines, NewRow, SideDecl};
use crate::money::MoneyHolders;
use crate::rounding::{Residue, RoundingLanding, split};
use crate::rows::{BALANCE, Optional, rows};
use crate::tests::{holder, table};

type Heap = HeapBacking<4096>;
const EUR: Ccy = Ccy::new(0);
const SETTLE: ApplyAt = ApplyAt::Day(SubStep::S7c);
const KINDS: MoneyHolders = MoneyHolders {
    central_banks: &["central bank"],
    banks: &["bank"],
    treasuries: &["treasury"],
    depositors: &["firm"],
    requesters: &["BNK"],
};

/// The holder tables, and where each party is.
struct Tables {
    kinds: Vec<KindTable<Heap>>,
    at: Vec<(PartyId, u16, Slot)>,
    ended: Vec<PartyId>,
}

impl Holders for Tables {
    fn locate(&self, party: PartyId) -> Located {
        if self.ended.contains(&party) {
            return Located::Ended;
        }
        let Some((p, table, slot)) = self.at.iter().find(|(p, _, _)| *p == party) else { panic!("unknown party") };
        Located::Live { party: *p, table: *table, slot: *slot }
    }

    fn arenas(&mut self, table: u16) -> &mut dyn HolderArenas {
        &mut self.kinds[usize::from(table)]
    }
}

#[derive(Default)]
struct Seen {
    applied: Vec<u64>,
    legs: Vec<(u64, LegDigest)>,
}

impl AuditStream for Seen {
    fn applied(&mut self, instruction: u64) {
        self.applied.push(instruction);
    }

    fn touched(&mut self, _: TableId, _: Slot) {}

    fn leg(&mut self, instruction: u64, leg: LegDigest) {
        self.legs.push((instruction, leg));
    }
}

fn terms(facility: Missing<Facility>) -> Terms {
    Terms {
        ccy: EUR,
        legs: vec![],
        schedule: Schedule {
            dates: ScheduleDates {
                anchor: Date::new(2026, 1, 15).unwrap(),
                period: Period::months(1).unwrap(),
                eom: EndOfMonth::Plain,
                convention: BusinessDayConvention::Following,
                country: CountryId::new(0),
            },
            count: Missing::Absent,
        },
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 30 },
        underlying: Missing::Absent,
        facility,
        stay: Missing::Absent,
    }
}

/// A central bank (party 1), two banks (2, 3) and three firms (4, 5 at the first bank, 6 at the second), each firm
/// with 1 000 on deposit and each bank with 5 000 in reserves; the first bank's deposits carry a facility of 500.
struct World {
    ledger: Ledger<Heap>,
    tables: Tables,
    reserves: LineId,
    deposits: [LineId; 2],
    pay: ReasonId,
    next: u32,
}

fn world() -> World {
    let mut space = AddressSpace::empty();
    let keys = HolderKeys::new(3);
    let mut ledger =
        Ledger::new(Instruments::new(&mut space, 16, 16, 16, keys), Lines::new(&mut space, 16, 16, 16, keys));
    let mut tables =
        vec![table(&mut space, "central bank", 0), table(&mut space, "bank", 1), table(&mut space, "firm", 2)];
    let mut at = Vec::new();
    for (party, t) in [(1, 0_u16), (2, 1), (3, 1), (4, 2), (5, 2), (6, 2)] {
        let slot = holder(&mut space, &mut tables[usize::from(t)], party);
        at.push((PartyId::new(party), t, slot));
    }
    let mut tables = Tables { kinds: tables, at, ended: vec![] };
    let plain = ledger.terms.intern(terms(Missing::Absent));
    let facility =
        Facility { limit: Money::new(500, EUR), rate: Rate::new(0, RatePeriod::Year), day_count: DayCount::Act365F };
    let overdraft = ledger.terms.intern(terms(Missing::Present(facility)));
    let reserves_kind = ledger.lines.declare_money(KINDS.reserves());
    let deposit_kind = ledger.lines.declare_money(KINDS.deposits("current account"));
    let reserves = ledger.lines.open(reserves_kind.index(), plain, Missing::Absent);
    let deposits = [
        ledger.lines.open(deposit_kind.index(), overdraft, Missing::Absent),
        ledger.lines.open(deposit_kind.index(), plain, Missing::Absent),
    ];
    let with = |w: u8| {
        let b = if w & crate::rows::PENDING == 0 { Missing::Absent } else { Missing::Present(0) };
        Optional { balance: Missing::Present(0), pending: b, amount: Missing::Absent }
    };
    let mut open = |ledger: &mut Ledger<Heap>, party: u64, line: LineId, side: Side, words: u8| {
        let Located::Live { table, slot, .. } = tables.locate(PartyId::new(party)) else { unreachable!() };
        let new = NewRow { side, within: 0, count: 1, point: 0, optional: with(words) };
        ledger.lines.add_row(tables.arenas(table), table, slot, line, new);
    };
    open(&mut ledger, 1, reserves, Side::Liability, BALANCE);
    for bank in [2, 3] {
        open(&mut ledger, bank, reserves, Side::Asset, BALANCE);
    }
    open(&mut ledger, 2, deposits[0], Side::Liability, BALANCE);
    open(&mut ledger, 3, deposits[1], Side::Liability, BALANCE);
    for (firm, line) in [(4, deposits[0]), (5, deposits[0]), (6, deposits[1])] {
        open(&mut ledger, firm, line, Side::Asset, BALANCE | crate::rows::PENDING);
    }
    let pay = ledger.reasons.declare(ReasonDecl {
        name: "payment",
        order: 0,
        paid: Effect::Expense,
        received: Effect::Revenue,
    });
    let opening = ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let mut w = World { ledger, tables, reserves, deposits, pay, next: 0 };
    let mut legs = Vec::new();
    for bank in [2, 3] {
        legs.push(write(bank, reserves, Side::Asset, 5_000));
        legs.push(write(1, reserves, Side::Liability, -5_000));
    }
    for (firm, bank, line) in [(4, 2, deposits[0]), (5, 2, deposits[0]), (6, 3, deposits[1])] {
        legs.push(write(firm, line, Side::Asset, 1_000));
        legs.push(write(bank, line, Side::Liability, -1_000));
    }
    let open = w.instruction(opening, legs);
    let _ = w.ledger.apply(&mut w.tables, ApplyAt::Opening, open, &mut Seen::default()).unwrap();
    w
}

fn write(party: u64, line: LineId, side: Side, qty: i64) -> LegRec {
    LegRec {
        party: PartyId::new(party),
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(EUR),
        kind: LegKind::OpeningWrite { identity: 1, cost: 0 },
    }
}

fn money(party: u64, line: LineId, side: Side, qty: i64) -> LegRec {
    LegRec {
        party: PartyId::new(party),
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(EUR),
        kind: LegKind::Money,
    }
}

impl World {
    fn instruction(&mut self, reason: ReasonId, legs: Vec<LegRec>) -> Instruction {
        self.next += 1;
        Instruction {
            id: InstructionId::new(Day::new(10), self.next),
            reason,
            trade_day: Day::new(10),
            settle_day: Day::new(10),
            legs,
            pays: Missing::Absent,
            covers: vec![],
        }
    }

    /// A payment between two depositors of one bank.
    fn same_bank(&mut self, from: u64, to: u64, amount: i64) -> Instruction {
        let line = self.deposits[0];
        let legs = vec![money(from, line, Side::Asset, -amount), money(to, line, Side::Asset, amount)];
        self.instruction(self.pay, legs)
    }

    fn balance(&mut self, party: u64, line: LineId, side: Side) -> i64 {
        let Located::Live { table, slot, .. } = self.tables.locate(PartyId::new(party)) else { unreachable!() };
        let r = rows(self.tables.arenas(table), slot);
        let Some(view) = r.iter().find(|v| v.row.line == line && v.side() == side) else { panic!("no row") };
        let Missing::Present(b) = view.optional.balance else { panic!("no balance") };
        b
    }

    fn line_sum(&mut self, line: LineId, parties: &[(u64, Side)]) -> i64 {
        parties.iter().map(|(p, s)| self.balance(*p, line, *s)).sum()
    }
}

fn caught_clause(f: impl FnOnce()) -> Option<&'static str> {
    let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).expect_err("the run stops");
    payload.downcast_ref::<phx_num::Violation>().map(|v| v.clause)
}

#[test]
fn all_or_none() {
    let mut w = world();
    let line = w.deposits[0];
    let mut legs = vec![money(4, line, Side::Asset, -900), money(5, line, Side::Asset, 900)];
    legs.extend([money(4, line, Side::Asset, -700), money(5, line, Side::Asset, 700)]);
    let i = w.instruction(w.pay, legs);
    let fail = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).expect_err("1 600 of 1 000 and 500 more");
    assert_eq!(fail.cause, FailCause::Funds);
    assert_eq!((w.balance(4, line, Side::Asset), w.balance(5, line, Side::Asset)), (1_000, 1_000), "no leg settled");
    assert_eq!(w.ledger.fails().len(), 1);
}

#[test]
fn negative_only_with_facility_and_is_the_loan() {
    let mut w = world();
    let line = w.deposits[0];
    let i = w.same_bank(4, 5, 1_300);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).expect("within the facility of 500");
    assert_eq!(w.balance(4, line, Side::Asset), -300, "the negative balance is the overdraft, recorded once");
    let i = w.same_bank(4, 5, 300);
    assert!(w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).is_err(), "beyond the facility");
    let other = w.deposits[1];
    let legs = vec![money(6, other, Side::Asset, -1_001), money(6, other, Side::Asset, 1_001)];
    let i = w.instruction(w.pay, legs);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).expect("a leg's own return makes it whole");
    let i = w.instruction(w.pay, vec![money(6, other, Side::Asset, -1_001), money(3, other, Side::Liability, 1_001)]);
    let fail = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).expect_err("no facility, no negative");
    assert_eq!(fail.cause, FailCause::Funds);
}

#[test]
fn interbank_moves_reserves_equally() {
    let mut w = world();
    let ([d1, d2], r) = (w.deposits, w.reserves);
    let legs = vec![
        money(4, d1, Side::Asset, -200),
        money(2, d1, Side::Liability, 200),
        money(2, r, Side::Asset, -200),
        money(3, r, Side::Asset, 200),
        money(3, d2, Side::Liability, -200),
        money(6, d2, Side::Asset, 200),
    ];
    let i = w.instruction(w.pay, legs);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).unwrap();
    assert_eq!((w.balance(2, r, Side::Asset), w.balance(3, r, Side::Asset)), (4_800, 5_200));
    assert_eq!(w.line_sum(r, &[(1, Side::Liability), (2, Side::Asset), (3, Side::Asset)]), 0);
    assert_eq!(w.line_sum(d2, &[(3, Side::Liability), (6, Side::Asset)]), 0, "the issuer's side is its liability");
    assert_eq!(w.balance(6, d2, Side::Asset), 1_200);
}

#[test]
fn rounding_residue_lands_on_named_party() {
    let named = PartyId::new(9);
    let parties = [(PartyId::new(1), 1), (PartyId::new(2), 1), (PartyId::new(3), 1)];
    let landing = RoundingLanding { convention: Round::HalfEven, residue_to: Residue::Named(named) };
    let s = split(Money::new(100, EUR), &parties, landing);
    assert_eq!(s.paid, Money::new(100, EUR), "none is lost");
    assert_eq!(s.shares.last(), Some(&(named, Money::new(1, EUR))));
    let kept = split(Money::new(100, EUR), &parties, RoundingLanding { residue_to: Residue::Payer, ..landing });
    assert_eq!(kept.paid, Money::new(99, EUR), "the payer keeps the unit no share rounds to");
    let payee = split(
        Money::new(101, EUR),
        &[(PartyId::new(1), 1), (PartyId::new(2), 2)],
        RoundingLanding { residue_to: Residue::Payee, ..landing },
    );
    assert_eq!(payee.shares, vec![(PartyId::new(1), Money::new(34, EUR)), (PartyId::new(2), Money::new(67, EUR))]);
}

#[test]
fn double_apply_violates() {
    let mut w = world();
    let first = w.same_bank(4, 5, 10);
    let again = Instruction { legs: first.legs.clone(), covers: vec![], ..first };
    let _ = w.ledger.apply(&mut w.tables, SETTLE, first, &mut Seen::default()).unwrap();
    let clause = caught_clause(|| {
        let _ = w.ledger.apply(&mut w.tables, SETTLE, again, &mut Seen::default());
    });
    assert_eq!(clause, Some("SET.11"));
    let at_4b = w.same_bank(4, 5, 10);
    let clause = caught_clause(|| {
        let _ = w.ledger.apply(&mut w.tables, ApplyAt::Day(SubStep::S4b), at_4b, &mut Seen::default());
    });
    assert_eq!(clause, Some("SET.11"), "money does not move at 4b");
    let one_sided = w.instruction(w.pay, vec![money(4, w.deposits[0], Side::Asset, -10)]);
    let clause = caught_clause(|| {
        let _ = w.ledger.apply(&mut w.tables, SETTLE, one_sided, &mut Seen::default());
    });
    assert_eq!(clause, Some("SET.11"), "no instruction with one side");
}

#[test]
fn transformation_needs_source() {
    let mut w = world();
    let unit = UnitId::new(7);
    let wheat = w.ledger.instruments.issue(NewInstrument {
        family: InstrumentFamily::RealAsset,
        issuer: Missing::Absent,
        unit,
        ccy: EUR,
        terms: crate::terms::TermsId::new(0),
    });
    let made = |qty, source| LegRec {
        party: PartyId::new(4),
        account: AccountRef::Instrument(wheat),
        qty,
        denom: Denom::Unit(unit),
        kind: LegKind::Transformation(source),
    };
    let i = w.instruction(w.pay, vec![made(40, Source::Way(3))]);
    let _ =
        w.ledger.apply(&mut w.tables, ApplyAt::Day(SubStep::S4b), i, &mut Seen::default()).expect("production alone");
    let i = w.instruction(w.pay, vec![made(-50, Source::Purchase(8))]);
    assert_eq!(
        w.ledger.apply(&mut w.tables, ApplyAt::Day(SubStep::S4b), i, &mut Seen::default()).map_err(|f| f.cause),
        Err(FailCause::FreeUnits),
        "no more used up than held"
    );
    let Located::Live { table, slot, .. } = w.tables.locate(PartyId::new(4)) else { unreachable!() };
    let Missing::Present(h) = holding(w.tables.arenas(table), slot, wheat) else { panic!("held") };
    assert_eq!((h.quantity.raw(), w.ledger.instruments.get(wheat).issued.n()), (40, 40), "made units are issued");
    let bare = LegRec { kind: LegKind::Units { cost: 0 }, ..made(5, Source::Way(3)) };
    let i = w.instruction(w.pay, vec![bare]);
    let clause = caught_clause(|| {
        let _ = w.ledger.apply(&mut w.tables, ApplyAt::Day(SubStep::S4b), i, &mut Seen::default());
    });
    assert_eq!(clause, Some("SET.11"), "units from nowhere without what accounts for them");
    let _: InstrumentId = wheat;
}

#[test]
fn contract_process_turns_fails_into_arrears() {
    let mut w = world();
    let loan_kind = w.ledger.lines.declare_money(LineKindDecl {
        name: "loan",
        asset: SideDecl { holder_kinds: &["bank"], words: BALANCE, holder_list: true },
        liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
        transfer_requesters: &["BNK"],
        dated: false,
    });
    let terms = w.ledger.terms.intern(terms(Missing::Absent));
    let loan = w.ledger.lines.open(loan_kind.index(), terms, Missing::Absent);
    let Located::Live { table, slot, .. } = w.tables.locate(PartyId::new(6)) else { unreachable!() };
    let owed = Optional { balance: Missing::Present(-5_000), ..Optional::NONE };
    let new = NewRow { side: Side::Liability, within: 0, count: 1, point: 0, optional: owed };
    w.ledger.lines.add_row(w.tables.arenas(table), table, slot, loan, new);
    let d2 = w.deposits[1];
    let mut i = w.instruction(w.pay, vec![money(6, d2, Side::Asset, -1_500), money(3, d2, Side::Liability, 1_500)]);
    i.pays = Missing::Present(DueRow { line: loan, side: Side::Liability });
    assert!(w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).is_err());
    let book = w.ledger.close();
    assert_eq!(book.measure().fails.get(&FailCause::Funds), Some(&1), "fails are published by cause");
    w.ledger.contract_process(&mut w.tables, &book.fails, Day::new(11));
    let record = |w: &mut World| {
        let r = rows(w.tables.arenas(table), slot);
        r.iter().find(|v| v.row.line == loan).map(crate::rows::RowView::record).unwrap()
    };
    assert_eq!((record(&mut w).missed, record(&mut w).arrears_days), (1, 1));
    w.ledger.contract_process(&mut w.tables, &[], Day::new(13));
    assert_eq!((record(&mut w).missed, record(&mut w).arrears_days), (1, 3), "arrears age each day until cured");
    w.ledger.cure(&mut w.tables, loan, Side::Liability, PartyId::new(6));
    assert_eq!((record(&mut w).missed, record(&mut w).arrears_days), (1, 0));
    assert!(w.ledger.arrears().is_empty());
}

#[test]
fn row_leg_adds_to_both_sides() {
    let mut w = world();
    let loan_kind = w.ledger.lines.declare_money(LineKindDecl {
        name: "loan",
        asset: SideDecl { holder_kinds: &["bank"], words: BALANCE, holder_list: true },
        liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
        transfer_requesters: &["BNK"],
        dated: false,
    });
    let terms = w.ledger.terms.intern(terms(Missing::Absent));
    let loan = w.ledger.lines.open(loan_kind.index(), terms, Missing::Absent);
    let zero = Optional { balance: Missing::Present(0), ..Optional::NONE };
    let open = |party: u64, side: Side| LegRec {
        party: PartyId::new(party),
        account: AccountRef::Line { line: loan, side },
        qty: 1,
        denom: Denom::Unit(UnitId::new(3)),
        kind: LegKind::Row(RowOp::Open(NewRow { side, within: 0, count: 1, point: 0, optional: zero })),
    };
    let i = w.instruction(w.pay, vec![open(3, Side::Asset), open(6, Side::Liability)]);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).unwrap();
    let adjust = |party: u64, side: Side, qty: i64| LegRec {
        party: PartyId::new(party),
        account: AccountRef::Line { line: loan, side },
        qty,
        denom: Denom::Ccy(EUR),
        kind: LegKind::Row(RowOp::Adjust),
    };
    let i = w.instruction(w.pay, vec![adjust(3, Side::Asset, 50), adjust(6, Side::Liability, -50)]);
    let _ = w.ledger.apply(&mut w.tables, ApplyAt::Day(SubStep::S2a), i, &mut Seen::default()).unwrap();
    assert_eq!((w.balance(3, loan, Side::Asset), w.balance(6, loan, Side::Liability)), (50, -50));
    assert_eq!(w.ledger.lines.side_count(loan, Side::Asset), 1);
    let count = |party: u64, side: Side| LegRec { kind: LegKind::Row(RowOp::Count), ..open(party, side) };
    let i = w.instruction(
        w.pay,
        vec![LegRec { qty: 2, ..count(3, Side::Asset) }, LegRec { qty: 2, ..count(6, Side::Liability) }],
    );
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).unwrap();
    let counts = (w.ledger.lines.side_count(loan, Side::Asset), w.ledger.lines.side_count(loan, Side::Liability));
    assert_eq!(counts, (3, 3), "members join both sides together");
}

#[test]
fn ended_parties_are_refused_by_name() {
    let mut w = world();
    w.tables.ended.push(PartyId::new(5));
    let i = w.same_bank(4, 5, 10);
    let fail = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).expect_err("an ended payee");
    assert_eq!((fail.cause, fail.party), (FailCause::Ended, PartyId::new(5)));
}

#[test]
fn declared_order_within_a_sub_step() {
    let mut w = world();
    let late = w.ledger.reasons.declare(ReasonDecl {
        name: "late",
        order: 1,
        paid: Effect::Expense,
        received: Effect::Revenue,
    });
    let line = w.deposits[0];
    let first = w.instruction(late, vec![money(4, line, Side::Asset, -1_000), money(5, line, Side::Asset, 1_000)]);
    let second = w.instruction(w.pay, vec![money(4, line, Side::Asset, -1_000), money(5, line, Side::Asset, 1_000)]);
    let (a, b) = (first.id, second.id);
    let out = w.ledger.settle(&mut w.tables, SETTLE, vec![first, second], &mut Seen::default());
    assert_eq!(out[0], Ok(b), "the earlier-ordered reason settles first whatever its place");
    assert_eq!(out[1].as_ref().map_err(|f| f.instruction).err(), Some(a));
}

#[test]
fn digests_reconcile_to_the_books() {
    // What the audit is told, leg by leg, rebuilds every position it touched from what it held before.
    let mut w = world();
    let ([d1, d2], r) = (w.deposits, w.reserves);
    let mut seen = Seen::default();
    let legs = vec![
        money(4, d1, Side::Asset, -200),
        money(2, d1, Side::Liability, 200),
        money(2, r, Side::Asset, -200),
        money(3, r, Side::Asset, 200),
        money(3, d2, Side::Liability, -200),
        money(6, d2, Side::Asset, 200),
    ];
    let i = w.instruction(w.pay, legs);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut seen).unwrap();
    let i = w.same_bank(5, 4, 50);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut seen).unwrap();
    assert_eq!(seen.applied.len(), 2);
    let mut firsts: Vec<(PartyId, u64, i64, i64)> = Vec::new();
    for (_, d) in &seen.legs {
        match firsts.iter_mut().find(|(p, a, _, _)| *p == d.party && *a == d.account) {
            Some((_, _, _, net)) => *net += d.qty,
            None => firsts.push((d.party, d.account, d.before, d.qty)),
        }
    }
    for (party, account, before, net) in firsts {
        let Located::Live { table, slot, .. } = w.tables.locate(party) else { unreachable!() };
        assert_eq!(w.ledger.position(w.tables.arenas(table), party, slot, account), before + net);
    }
    let money_sum: i64 = seen.legs.iter().filter(|(_, d)| d.money).map(|(_, d)| d.qty).sum();
    assert_eq!(money_sum, 0, "no money made without its issuer");
}

#[test]
fn money_lines_balance_at_their_issuers() {
    let mut w = world();
    let i = w.same_bank(4, 5, 300);
    let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).unwrap();
    let t = &w.tables.kinds;
    let tables: [&dyn HolderArenas; 3] = [&t[0], &t[1], &t[2]];
    for line in [w.reserves, w.deposits[0], w.deposits[1]] {
        assert!(crate::audit::money_line(&w.ledger.lines, &tables, line).is_empty());
    }
}

#[test]
fn settlement_published_gross_and_net() {
    let mut w = world();
    for (from, to, amount) in [(4, 5, 300), (5, 4, 100)] {
        let i = w.same_bank(from, to, amount);
        let _ = w.ledger.apply(&mut w.tables, SETTLE, i, &mut Seen::default()).unwrap();
    }
    let m = w.ledger.close().measure();
    assert_eq!((m.gross.get(&0), m.net.get(&0)), (Some(&400), Some(&200)), "400 paid, 200 changed hands net");
}

#[test]
fn account_codes_round_trip() {
    for a in [
        AccountRef::Line { line: LineId::new(7), side: Side::Asset },
        AccountRef::Line { line: LineId::new(7), side: Side::Liability },
        AccountRef::Instrument(InstrumentId::new(3)),
        AccountRef::Unit(1 << 40),
    ] {
        assert_eq!(AccountRef::from_code(a.code()), a);
        assert_eq!(AccountRef::from_code(a.code() | crate::instruction::ROW_COUNT), a, "a count marks its line");
    }
}
