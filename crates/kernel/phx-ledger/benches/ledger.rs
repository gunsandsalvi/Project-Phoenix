#![expect(
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::unwrap_used,
    reason = "gungraun's harness prints and exits; a setup that fails is a broken benchmark"
)]

use std::hint::black_box;

use gungraun::{library_benchmark, library_benchmark_group, main};
use phx_core::kind_tables::RunHead;
use phx_core::{
    AuditStream, BusinessDayConvention, Calendar, CountryRules, DayCount, EndOfMonth, GenReport, LegDigest, Period,
    ScheduleDates, WeekendRule,
};
use phx_id::{CountryId, Date, Day, LineId, PartyId, Slot, TableId, TileId, Weekday};
use phx_ledger::algebra::{
    DefaultDefinition, DueBuf, DueState, Leg, PaymentOrder, Reference, Repayment, Schedule, Seniority, Side,
    Termination, Terms, due_on,
};
use phx_ledger::books::{Books, BooksSize};
use phx_ledger::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, RowOp};
use phx_ledger::line::{LineKindDecl, NewRow, SideDecl};
use phx_ledger::money::MoneyHolders;
use phx_ledger::pending::Closed;
use phx_ledger::pooled::{Kink, PooledRow, pooled};
use phx_ledger::rows::{BALANCE, Optional, PENDING};
use phx_num::{Ccy, Missing, Money, Rate, RatePeriod, UnitId};
use phx_store::HeapBacking;

const EUR: Ccy = Ccy::new(0);

fn calendar() -> Calendar {
    let rules =
        CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
    Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2020).unwrap()
}

/// A ten-year bond paying a fixed coupon a year and its principal at the end, asked for its dues on its fifth
/// coupon date: two legs, one due.
fn bond() -> (Calendar, Terms, Day) {
    let cal = calendar();
    let schedule = Schedule {
        dates: ScheduleDates {
            anchor: Date::new(2026, 1, 15).unwrap(),
            period: Period::months(12).unwrap(),
            eom: EndOfMonth::Plain,
            convention: BusinessDayConvention::Following,
            country: CountryId::new(0),
        },
        count: Missing::Present(10),
    };
    let terms = Terms {
        ccy: EUR,
        legs: vec![
            Leg::RateOnNotional {
                reference: Reference::Fixed(Rate::new(50_000_000_000, RatePeriod::Year)),
                day_count: DayCount::Thirty360Bond,
            },
            Leg::Principal { amount: Money::new(1_000_000, EUR), repayment: Repayment::Bullet },
        ],
        schedule,
        seniority: Seniority(0),
        collateral: Missing::Absent,
        payment_order: PaymentOrder(0),
        termination: Termination::None,
        conversion: Missing::Absent,
        default: DefaultDefinition { missed_payments: 1, grace_days: 30 },
        underlying: Missing::Absent,
        facility: Missing::Absent,
        stay: Missing::Absent,
    };
    let day = schedule.day(&cal, 5);
    (cal, terms, day)
}

/// The same bond on the day after its fifth coupon date: nothing due, so only the search for a date is paid.
fn bond_between() -> (Calendar, Terms, Day) {
    let (cal, terms, day) = bond();
    (cal, terms, day.succ())
}

fn dues(setup: (Calendar, Terms, Day)) -> usize {
    let (cal, terms, day) = setup;
    let state = DueState {
        calendar: &cal,
        outstanding: Money::new(1_000_000, EUR),
        elected: &|_, _| false,
        occurred: &|_, _| false,
        in_state_since: &|_, _| Missing::Absent,
    };
    let mut out = DueBuf::default();
    due_on(black_box(&terms), black_box(day), &state, &mut out);
    black_box(out.iter().count())
}

#[library_benchmark]
#[bench::fresh(bond())]
fn ir_due_on_coupon(setup: (Calendar, Terms, Day)) -> usize {
    dues(setup)
}

#[library_benchmark]
#[bench::fresh(bond_between())]
fn ir_due_on_between(setup: (Calendar, Terms, Day)) -> usize {
    dues(setup)
}

/// Sixty-four holders' run heads, half due on the day: the cost of a head read for a holder whose rows are not.
fn heads() -> (Vec<RunHead>, u32) {
    let heads = (0..64_u32).map(|i| RunHead { next_due: 100 + i, offset: 0, len: 1 }).collect();
    (heads, 131)
}

#[library_benchmark]
#[bench::heads(setup = heads)]
fn ir_run_heads_64((heads, day): (Vec<RunHead>, u32)) -> usize {
    black_box(&heads).iter().filter(|h| h.due(Day::new(day))).count()
}

/// Sixteen rows of a payer's order against its funds and two kinks, the last row failing.
fn rows() -> (Vec<PooledRow>, Vec<Kink>) {
    let rows =
        (0..16).map(|i| PooledRow { per_member: 60 + i, reached: 3, position: 1_000, moves: -(60 + i) }).collect();
    (rows, vec![Kink { at: 0, fails: true }, Kink { at: 500, fails: false }])
}

#[library_benchmark]
#[bench::rows(setup = rows)]
fn ir_pooled_16_rows((rows, kinks): (Vec<PooledRow>, Vec<Kink>)) -> usize {
    pooled(black_box(3_000), 3, &rows, &kinks).len()
}

type Heap = HeapBacking<4096>;
const HOLDERS: MoneyHolders = MoneyHolders {
    central_banks: &["central bank"],
    banks: &["bank"],
    treasuries: &["treasury"],
    depositors: &["firm"],
    requesters: &["BNK"],
};
const LOAN: LineKindDecl = LineKindDecl {
    name: "loan",
    asset: SideDecl { holder_kinds: &["bank"], words: BALANCE, holder_list: true, holder_roles: &[], exclusive: false },
    liability: SideDecl {
        holder_kinds: &["firm"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
    },
    transfer_requesters: &["BNK"],
    dated: true,
};
const FIRMS: u32 = 16;

struct Quiet;

impl AuditStream for Quiet {
    fn applied(&mut self, _: u64) {}
    fn touched(&mut self, _: TableId, _: Slot) {}
    fn leg(&mut self, _: u64, _: LegDigest) {}
}

fn open(party: PartyId, line: LineId, side: Side, count: u32, words: u8) -> LegRec {
    let word = |f: u8| if words & f == 0 { Missing::Absent } else { Missing::Present(0) };
    let optional = Optional { balance: word(BALANCE), pending: word(PENDING), amount: Missing::Absent };
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: i64::from(count),
        denom: Denom::Unit(UnitId::new(0)),
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

/// A bank lending on one line to sixteen firms at 12% a year paid monthly, each with a deposit at the bank, on the
/// day their interest falls due: stage 1b and the whole of stage 7 over sixteen payments.
fn loans() -> (Books<Heap>, Calendar, Day) {
    let size = BooksSize { rows: 64, rows_per_chunk: 64, instruments: 16, lines: 16, per_chunk: 16, blocks: 64 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let dates = ScheduleDates {
        anchor: Date::new(2026, 1, 15).unwrap(),
        period: Period::months(1).unwrap(),
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: CountryId::new(0),
    };
    let due = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let cb = books.parties.begin("central bank", TileId::new(0), Day::new(0));
    let bank = books.parties.begin("bank", TileId::new(0), Day::new(0));
    let firms: Vec<PartyId> = (0..FIRMS).map(|_| books.parties.begin("firm", TileId::new(0), Day::new(0))).collect();
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let loan_kind = books.ledger.lines.declare_money(LOAN).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, dates));
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = books.ledger.lines.open(deposit_kind, account, Missing::Absent);
    let terms = Terms {
        legs: vec![Leg::RateOnNotional {
            reference: Reference::Fixed(Rate::new(120_000_000_000, RatePeriod::Year)),
            day_count: DayCount::Act365F,
        }],
        schedule: Schedule { dates, count: Missing::Present(12) },
        ..Terms::account(EUR, dates)
    };
    let id = books.ledger.terms.intern(terms);
    let loan = books.ledger.lines.open(loan_kind, id, Missing::Present((due, 1)));
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let mut legs = vec![open(cb, reserves, Side::Liability, 1, BALANCE), open(bank, reserves, Side::Asset, 1, BALANCE)];
    legs.push(open(bank, deposits, Side::Liability, FIRMS, BALANCE));
    legs.push(open(bank, loan, Side::Asset, FIRMS, BALANCE));
    for firm in &firms {
        legs.push(open(*firm, deposits, Side::Asset, 1, BALANCE | PENDING));
        legs.push(open(*firm, loan, Side::Liability, 1, BALANCE));
    }
    books.open(reason, legs, 0, &mut GenReport::default());
    let mut writes = Vec::new();
    for firm in &firms {
        writes.extend([write(*firm, deposits, Side::Asset, 10_000), write(bank, deposits, Side::Liability, -10_000)]);
        writes.extend([write(*firm, loan, Side::Liability, -1_000), write(bank, loan, Side::Asset, 1_000)]);
    }
    books.open(reason, writes, 1, &mut GenReport::default());
    (books, cal, due)
}

#[library_benchmark]
#[bench::loans(setup = loans)]
fn ir_settle_day_16((mut books, cal, day): (Books<Heap>, Calendar, Day)) -> u64 {
    let due = books.ledger.mark_due(day, &cal);
    let draws = |l: phx_id::LineId| {
        let subject = phx_rand::Subject::new(phx_rand::SubjectTag::Line, u64::from(l.get()));
        phx_rand::Draws::new(phx_rand::stream_key(phx_rand::Seed::new(1), "REP.cleared"), subject, 0, 0)
    };
    books.settle_day(&due, day, &cal, &Closed::default(), &draws, &mut Quiet).settled
}

library_benchmark_group!(
    name = ledger,
    benchmarks = [ir_due_on_coupon, ir_due_on_between, ir_run_heads_64, ir_pooled_16_rows, ir_settle_day_16]
);

main!(library_benchmark_groups = ledger);
