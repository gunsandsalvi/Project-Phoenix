//! Line transfers and procedure lines over real books.
#![cfg(test)]

use phx_core::calendar::Calendar;
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::{AuditStream, GenReport, LegDigest, SubStep};
use phx_id::{CountryId, Date, Day, LineId, PartyId, Slot, TableId, TileId, Weekday};
use phx_num::round::Round;
use phx_num::{Ccy, Missing, Rate, RatePeriod, UnitId};
use phx_store::HeapBacking;

use crate::algebra::{Leg, Reference, Schedule, Side, Terms};
use crate::apply::ApplyAt;
use crate::books::{Books, BooksSize};
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, ReasonId, RowOp};
use crate::line::{LineKindDecl, NewRow, SideDecl};
use crate::money::MoneyHolders;
use crate::pending::Closed;
use crate::rows::{BALANCE, Optional, PENDING};
use crate::transfer::{LineTransfer, MoveAt, ToProcedureLine};

type Heap = HeapBacking<4096>;
const EUR: Ccy = Ccy::new(0);
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &["central bank"],
    banks: &["bank"],
    treasuries: &["treasury"],
    depositors: &["firm"],
    requesters: &["BNK"],
};
const LOAN: LineKindDecl = LineKindDecl {
    name: "loan",
    asset: SideDecl {
        holder_kinds: &["bank"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &["firm"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    transfer_requesters: &["BNK"],
    dated: true,
};
const CONTRACTS: UnitId = UnitId::new(0);

struct Quiet;

impl AuditStream for Quiet {
    fn applied(&mut self, _: u64) {}
    fn touched(&mut self, _: TableId, _: Slot) {}
    fn leg(&mut self, _: u64, _: LegDigest) {}
}

fn calendar() -> Calendar {
    let rules =
        CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
    Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2025).unwrap()
}

fn dates() -> ScheduleDates {
    ScheduleDates {
        anchor: Date::new(2026, 1, 15).unwrap(),
        period: Period::months(1).unwrap(),
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: CountryId::new(0),
    }
}

fn open(party: PartyId, line: LineId, side: Side, count: u32, words: u8) -> LegRec {
    let word = |f: u8| if words & f == 0 { Missing::Absent } else { Missing::Present(0) };
    let optional = Optional { balance: word(BALANCE), pending: word(PENDING), amount: Missing::Absent };
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: i64::from(count),
        denom: Denom::Unit(CONTRACTS),
        kind: LegKind::Row(RowOp::Open(NewRow { side, within: 0, count, point: 0, optional })),
    }
}

fn write(party: PartyId, line: LineId, side: Side, qty: i64) -> LegRec {
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty,
        denom: Denom::Ccy(EUR),
        kind: LegKind::OpeningWrite { identity: 1, cost: 0 },
    }
}

/// A bank lending on one shared line to three firms, 100, 200 and 300 at 12% a year paid monthly, each firm with
/// 10 000 on deposit at the bank; a second bank that buys loans.
struct World {
    books: Books<Heap>,
    cal: Calendar,
    banks: [PartyId; 2],
    firms: [PartyId; 3],
    loan: LineId,
    deposits: LineId,
    reason: ReasonId,
    due: Day,
}

fn world() -> World {
    let size = BooksSize { rows: 16, rows_per_chunk: 16, instruments: 16, lines: 16, per_chunk: 16, blocks: 16 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let due = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let begin = |books: &mut Books<Heap>, kind| books.parties.begin(kind, TileId::new(0), Day::new(0));
    let cb = begin(&mut books, "central bank");
    let banks = [begin(&mut books, "bank"), begin(&mut books, "bank")];
    let firms = [begin(&mut books, "firm"), begin(&mut books, "firm"), begin(&mut books, "firm")];
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let loan_kind = books.ledger.lines.declare_money(LOAN).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, dates()));
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = books.ledger.lines.open(deposit_kind, account, Missing::Absent);
    let twelve = Rate::new(120_000_000_000, RatePeriod::Year);
    let terms = Terms {
        legs: vec![Leg::RateOnNotional { reference: Reference::Fixed(twelve), day_count: DayCount::Act365F }],
        schedule: Schedule { dates: dates(), count: Missing::Present(12) },
        ..Terms::account(EUR, dates())
    };
    let id = books.ledger.terms.intern(terms);
    let loan = books.ledger.lines.open(loan_kind, id, Missing::Present((due, 1)));
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
        held: phx_num::Missing::Absent,
    });
    let mut legs = vec![open(cb, reserves, Side::Liability, 2, BALANCE)];
    for bank in banks {
        legs.push(open(bank, reserves, Side::Asset, 1, BALANCE));
    }
    legs.push(open(banks[0], deposits, Side::Liability, 3, BALANCE));
    legs.push(open(banks[0], loan, Side::Asset, 3, BALANCE));
    for firm in firms {
        legs.push(open(firm, deposits, Side::Asset, 1, BALANCE | PENDING));
        legs.push(open(firm, loan, Side::Liability, 1, BALANCE));
    }
    books.open(reason, legs, 0, &mut GenReport::default());
    let mut writes = Vec::new();
    for (firm, owed) in firms.iter().zip([100, 200, 300]) {
        writes
            .extend([write(*firm, deposits, Side::Asset, 10_000), write(banks[0], deposits, Side::Liability, -10_000)]);
        writes.extend([write(*firm, loan, Side::Liability, -owed), write(banks[0], loan, Side::Asset, owed)]);
    }
    books.open(reason, writes, 1, &mut GenReport::default());
    World { books, cal, banks, firms, loan, deposits, reason, due }
}

fn row(w: &World, party: PartyId, line: LineId, side: Side) -> (u32, i64) {
    let v = w.books.row_on_side(party, line, side).expect("the row");
    let Missing::Present(b) = v.optional.balance else { panic!("no balance") };
    (v.row.count, b)
}

fn at(w: &World) -> MoveAt {
    MoveAt { contracts: CONTRACTS, rounding: Round::HalfEven, day: w.due, at: ApplyAt::Day(SubStep::S2e) }
}

#[test]
fn procedure_line_leaves_shared_line() {
    let mut w = world();
    let m = at(&w);
    let moved = ToProcedureLine {
        line: w.loan,
        side: Side::Liability,
        from: w.firms[1],
        count: 1,
        procedure: 7,
        reason: w.reason,
    };
    let procedure = w.books.to_procedure_line(moved, m, &mut Quiet).expect("the move settles");
    assert_eq!(row(&w, w.firms[1], procedure, Side::Liability), (1, -200), "the debtor's row moved whole");
    assert_eq!(row(&w, w.banks[0], procedure, Side::Asset), (1, 200), "with the lender's mirror");
    assert_eq!(row(&w, w.banks[0], w.loan, Side::Asset), (2, 400), "the shared line less the moved count");
    assert!(w.books.row_on_side(w.firms[1], w.loan, Side::Liability).is_none());
    for side in [Side::Asset, Side::Liability] {
        assert_eq!(w.books.ledger.lines.side_count(w.loan, side), 2);
        assert_eq!(w.books.ledger.lines.side_count(procedure, side), 1);
    }
    // With the procedure open, its line pays nothing; the other borrowers pay their interest as before.
    w.books.ledger.procedures.insert(7);
    let due = w.books.ledger.mark_due(w.due, &w.cal);
    let s = w.books.settle_day(&due, w.due, &w.cal, &Closed::default(), &crate::cleared::test_draws, &mut Quiet);
    assert_eq!((s.payments, s.settled), (2, 2), "the stayed row's due is suspended");
    let paid: Vec<i64> = w.firms.iter().map(|f| 10_000 - row(&w, *f, w.deposits, Side::Asset).1).collect();
    assert_eq!(paid, vec![1, 0, 3], "a month at 12% on 100 and 300, to the nearest unit; nothing on the stayed 200");
}

#[test]
fn a_transfer_moves_count_with_balance() {
    let mut w = world();
    let m = at(&w);
    let sale =
        LineTransfer { line: w.loan, side: Side::Asset, from: w.banks[0], to: w.banks[1], count: 1, reason: w.reason };
    let _ = w.books.transfer(sale, m, &mut Quiet).expect("the sale settles");
    assert_eq!(row(&w, w.banks[0], w.loan, Side::Asset), (2, 400));
    assert_eq!(row(&w, w.banks[1], w.loan, Side::Asset), (1, 200), "a third of the balance with a third of the count");
    assert_eq!(w.books.ledger.lines.side_count(w.loan, Side::Asset), 3, "the side's members stay");
    let all =
        LineTransfer { line: w.loan, side: Side::Asset, from: w.banks[0], to: w.banks[1], count: 2, reason: w.reason };
    let _ = w.books.transfer(all, m, &mut Quiet).expect("the rest settles");
    assert!(
        w.books.row_on_side(w.banks[0], w.loan, Side::Asset).is_none(),
        "a row all of whose members leave is retired"
    );
    assert_eq!(row(&w, w.banks[1], w.loan, Side::Asset), (3, 600));
}

/// A borrower's row moved onto another borrower's row on the same line: the count and the negative balance arrive
/// together, the count judged apart from the balance.
#[test]
fn a_liability_transfer_joins_a_held_row() {
    let mut w = world();
    let m = at(&w);
    let [first, second, _] = w.firms;
    let taken =
        LineTransfer { line: w.loan, side: Side::Liability, from: second, to: first, count: 1, reason: w.reason };
    let _ = w.books.transfer(taken, m, &mut Quiet).expect("the move settles");
    assert_eq!(row(&w, first, w.loan, Side::Liability), (2, -300), "the count and the debt taken on together");
    assert!(w.books.row_on_side(second, w.loan, Side::Liability).is_none(), "the moved row is retired");
    assert_eq!(w.books.ledger.lines.side_count(w.loan, Side::Liability), 3, "the side's members stay");
}

/// Members moved off a row in arrears carry its arrears to the row that takes them, and a row retired whole keeps
/// none; a taker already in arrears keeps the earlier day.
#[test]
fn a_transfer_carries_the_rows_arrears() {
    let mut w = world();
    let m = at(&w);
    let key = |p| crate::contract_process::ArrearsKey::new(w.loan, Side::Asset, p);
    let (early, late) = (Day::new(3), Day::new(5));
    let [lender, buyer] = w.banks;
    w.books.ledger.arrears.begin(key(lender), late);
    let one = LineTransfer { line: w.loan, side: Side::Asset, from: lender, to: buyer, count: 1, reason: w.reason };
    let _ = w.books.transfer(one, m, &mut Quiet).expect("the sale settles");
    let of = |w: &World, p| w.books.ledger.arrears.of(w.loan, Side::Asset, p);
    assert_eq!((of(&w, lender), of(&w, buyer)), (Some(late), Some(late)), "carried to the taker");
    w.books.ledger.arrears.remove(key(buyer));
    w.books.ledger.arrears.begin(key(buyer), early);
    let rest = LineTransfer { line: w.loan, side: Side::Asset, from: lender, to: buyer, count: 2, reason: w.reason };
    let _ = w.books.transfer(rest, m, &mut Quiet).expect("the sale settles");
    assert_eq!(
        (of(&w, lender), of(&w, buyer)),
        (None, Some(early)),
        "the retired row keeps none; the earlier day stands"
    );
}

/// A line of no balance held by a firm for two members, and by each bank for one on the other side: a member leaving
/// the firm's row takes one of the banks' with it, drawn by their members, and the sides stay equal.
#[test]
fn members_leave_with_their_counterparts() {
    let mut w = world();
    let m = at(&w);
    let kind = w.books.ledger.lines.kind_of(w.loan);
    let terms = w.books.ledger.lines.terms(w.loan);
    let job = w.books.ledger.lines.open(kind, terms, Missing::Absent);
    let legs = vec![
        open(w.firms[0], job, Side::Liability, 2, BALANCE),
        open(w.banks[0], job, Side::Asset, 1, BALANCE),
        open(w.banks[1], job, Side::Asset, 1, BALANCE),
    ];
    w.books.open(w.reason, legs, 2, &mut GenReport::default());
    let mut d = crate::cleared::test_draws(job);
    let _ = w.books.members_leave((w.firms[0], job, Side::Liability), 1, m, &mut d, &mut Quiet).expect("settles");
    assert_eq!(row(&w, w.firms[0], job, Side::Liability), (1, 0));
    let left: Vec<bool> = w.banks.iter().map(|b| w.books.row_on_side(*b, job, Side::Asset).is_some()).collect();
    assert_eq!(left.iter().filter(|l| **l).count(), 1, "one bank's member left with the firm's");
    let sides = [Side::Asset, Side::Liability].map(|s| w.books.ledger.lines.side_count(job, s));
    assert_eq!(sides, [1, 1]);
}

/// A firm settled as an estate: its 10 000 pays its loan of 100, the 9 900 left is paid to the destination, its rows
/// leave with a member of the bank's each, and it ends.
#[test]
fn an_estate_pays_its_debts_passes_the_rest_and_ends() {
    let mut w = world();
    let m = MoveAt { at: ApplyAt::Day(SubStep::S7c), ..at(&w) };
    let mut d = crate::cleared::test_draws(w.loan);
    let s = w.books.settle_estate(w.firms[0], w.firms[1], m, &mut d, &mut Quiet);
    assert!(s.fail.is_none() && s.ended && !s.unsold);
    assert_eq!((s.paid, s.written_off, s.passed), (100, 0, 9_900));
    assert_eq!(row(&w, w.firms[1], w.deposits, Side::Asset), (1, 19_900));
    assert_eq!(
        row(&w, w.banks[0], w.deposits, Side::Liability),
        (2, -29_900),
        "the bank owes the loan's repayment less"
    );
    assert_eq!(row(&w, w.banks[0], w.loan, Side::Asset), (2, 500));
    assert_eq!(w.books.parties.of_kind("firm").count(), 2, "the estate ended");
}

/// An estate holding 200 against a loan of 300 pays the 200 and its creditor loses the rest; nothing is passed on.
#[test]
fn an_estate_short_of_its_debts_writes_off_the_rest() {
    let mut w = world();
    let m = MoveAt { at: ApplyAt::Day(SubStep::S7c), ..at(&w) };
    let legs = w.books.pay(w.firms[2], w.firms[1], 9_800, EUR);
    let _ = w.books.submit(w.reason, legs, m, &mut Quiet).expect("the payment settles");
    let mut d = crate::cleared::test_draws(w.loan);
    let s = w.books.settle_estate(w.firms[2], w.firms[1], m, &mut d, &mut Quiet);
    assert!(s.fail.is_none() && s.ended);
    assert_eq!((s.paid, s.written_off, s.passed), (200, 100, 0));
    assert_eq!(row(&w, w.banks[0], w.loan, Side::Asset), (2, 300), "the bank's claims less the paid and the lost");
    assert_eq!(row(&w, w.firms[1], w.deposits, Side::Asset), (1, 19_800));
}

/// A firm holding plant settled as an estate with 200 against a loan of 300: it pays the 200, and while it holds the
/// plant its creditor loses nothing yet, nothing passes on, and it waits to sell.
#[test]
fn an_estate_holding_units_waits_to_sell_them() {
    let mut w = world();
    let unit = UnitId::new(7);
    let plant = w.books.ledger.instruments.issue(crate::instrument::NewInstrument {
        family: crate::instrument::InstrumentFamily::RealAsset,
        issuer: Missing::Absent,
        unit,
        ccy: EUR,
        terms: crate::terms::TermsId::new(0),
    });
    let firm = w.firms[2];
    w.books.open(w.reason, vec![crate::opening::hold(firm, plant, 5, unit, 500, 9)], 2, &mut GenReport::default());
    let m = MoveAt { at: ApplyAt::Day(SubStep::S7c), ..at(&w) };
    let legs = w.books.pay(firm, w.firms[1], 9_800, EUR);
    let _ = w.books.submit(w.reason, legs, m, &mut Quiet).expect("the payment settles");
    let mut d = crate::cleared::test_draws(w.loan);
    let s = w.books.settle_estate(firm, w.firms[1], m, &mut d, &mut Quiet);
    assert!(s.fail.is_none() && s.unsold && !s.ended);
    assert_eq!((s.paid, s.written_off, s.passed), (200, 0, 0));
    assert_eq!(row(&w, firm, w.loan, Side::Liability), (1, -100), "the rest still owed");
    assert!(w.books.holds_units(firm));
}
