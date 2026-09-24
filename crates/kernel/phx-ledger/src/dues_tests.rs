//! The opening's writes and a day's dues over real books, whose kind tables hold the rows they move.
#![cfg(test)]

use phx_core::calendar::Calendar;
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::daycount::DayCount;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::{AuditStream, GenReport, LegDigest};
use phx_id::{CountryId, Date, Day, LineId, PartyId, Slot, TableId, TileId, Weekday};
use phx_num::{Ccy, Missing, Money, Rate, RatePeriod, UnitId};
use phx_store::HeapBacking;

use crate::algebra::{Leg, Reference, Repayment, Schedule, Side, Terms};
use crate::books::{Books, BooksSize};
use crate::check::FailCause;
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, RowOp};
use crate::line::{LineKindDecl, NewRow, SideDecl};
use crate::money::MoneyHolders;
use crate::rows::{BALANCE, Optional, PENDING};

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
    asset: SideDecl { holder_kinds: &["bank"], words: BALANCE, holder_list: true },
    liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
    transfer_requesters: &["BNK"],
};
/// Twelve percent a year in the rate's scale, where one whole is 10^12.
const TWELVE_PERCENT: i64 = 120_000_000_000;

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

fn monthly() -> ScheduleDates {
    ScheduleDates {
        anchor: Date::new(2026, 1, 15).unwrap(),
        period: Period::months(1).unwrap(),
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: CountryId::new(0),
    }
}

fn open(party: PartyId, line: LineId, side: Side, count: u32, words: u8) -> LegRec {
    let word = |flag: u8| if words & flag == 0 { Missing::Absent } else { Missing::Present(0) };
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
        kind: LegKind::OpeningWrite { identity: party.get(), cost: 0 },
    }
}

fn balance(books: &Books<Heap>, party: PartyId, line: LineId, side: Side) -> i64 {
    let (place, slot) = books.parties.row(party);
    let row = crate::rows::iter(books.parties.table(place), slot).find(|r| r.row.line == line && r.side() == side);
    match row.map(|r| r.optional.balance) {
        Some(Missing::Present(b)) => b,
        _ => panic!("no balance on line {}", line.get()),
    }
}

/// A central bank; two banks with 5 000 000 in reserves each; firm A at the first bank with 1 000 000 on deposit and
/// a loan of 500 000 from it; firm B at the second bank with 1 000 000 on deposit and a loan of 100 000 from the
/// first; firm C at the first bank with 1 000 on deposit and a loan of 1 000 000 from it. Each loan pays 12% a year
/// monthly from 15 January 2026 and its principal at its one date, 16 February (the 15th is a Sunday).
struct Opened {
    books: Books<Heap>,
    report: GenReport,
    parties: [PartyId; 6],
    reserves: LineId,
    deposits: [LineId; 2],
    loans: [LineId; 3],
    due: Day,
}

fn opened() -> Opened {
    let size = BooksSize { rows: 16, rows_per_chunk: 16, instruments: 16, lines: 16, per_chunk: 16, blocks: 16 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let at = |books: &mut Books<Heap>, kind| books.parties.begin(kind, TileId::new(0), Day::new(0));
    let cb = at(&mut books, "central bank");
    let banks = [at(&mut books, "bank"), at(&mut books, "bank")];
    let firms = [at(&mut books, "firm"), at(&mut books, "firm"), at(&mut books, "firm")];
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let loan_kind = books.ledger.lines.declare_money(LOAN).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, monthly()));
    let reserves = books.ledger.lines.open(reserves_kind, account, Day::new(0));
    let deposits = banks.map(|_| books.ledger.lines.open(deposit_kind, account, Day::new(0)));
    let due = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let lent = [(firms[0], banks[0], 500_000), (firms[1], banks[0], 100_000), (firms[2], banks[0], 1_000_000)];
    let loans = lent.map(|(_, _, principal)| {
        let terms = Terms {
            legs: vec![
                Leg::RateOnNotional {
                    reference: Reference::Fixed(Rate::new(TWELVE_PERCENT, RatePeriod::Year)),
                    day_count: DayCount::Act365F,
                },
                Leg::Principal { amount: Money::new(principal, EUR), repayment: Repayment::Bullet },
            ],
            schedule: Schedule { dates: monthly(), count: Missing::Present(1) },
            ..Terms::account(EUR, monthly())
        };
        let id = books.ledger.terms.intern(terms);
        books.ledger.lines.open(loan_kind, id, due)
    });
    for line in [reserves, deposits[0], deposits[1]] {
        books.ledger.lines.advance(line, Missing::Absent);
    }
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let mut report = GenReport::default();
    let banked = [(firms[0], 0, 1_000_000), (firms[1], 1, 1_000_000), (firms[2], 0, 1_000)];
    let mut legs = vec![open(cb, reserves, Side::Liability, 2, BALANCE)];
    for ((bank, line), depositors) in banks.iter().zip(deposits).zip([2, 1]) {
        legs.push(open(*bank, reserves, Side::Asset, 1, BALANCE));
        legs.push(open(*bank, line, Side::Liability, depositors, BALANCE));
    }
    for (firm, bank, _) in banked {
        legs.push(open(firm, deposits[bank], Side::Asset, 1, BALANCE | PENDING));
    }
    for ((firm, bank, _), line) in lent.iter().zip(loans) {
        legs.push(open(*bank, line, Side::Asset, 1, BALANCE));
        legs.push(open(*firm, line, Side::Liability, 1, BALANCE));
    }
    books.open(reason, legs, 0, &mut report);
    let mut writes = Vec::new();
    for bank in banks {
        writes
            .extend([write(bank, reserves, Side::Asset, 5_000_000), write(cb, reserves, Side::Liability, -5_000_000)]);
    }
    for (firm, bank, amount) in banked {
        writes.push(write(firm, deposits[bank], Side::Asset, amount));
        writes.push(write(banks[bank], deposits[bank], Side::Liability, -amount));
    }
    for ((firm, bank, principal), line) in lent.iter().zip(loans) {
        writes.push(write(*bank, line, Side::Asset, *principal));
        writes.push(write(*firm, line, Side::Liability, -principal));
    }
    books.open(reason, writes, 1, &mut report);
    let [f0, f1, f2] = firms;
    let [b0, b1] = banks;
    Opened { books, report, parties: [cb, b0, b1, f0, f1, f2], reserves, deposits, loans, due }
}

#[test]
fn opening_writes_close_books() {
    let o = opened();
    let equity: Vec<i128> = o.parties.iter().map(|p| o.books.equity(*p)).collect();
    assert_eq!(equity.iter().sum::<i128>(), 0, "claims net to nothing where no real asset is held");
    assert_eq!(
        equity,
        vec![-10_000_000, 5_000_000 - 1_001_000 + 1_600_000, 5_000_000 - 1_000_000, 500_000, 900_000, -999_000]
    );
    for p in o.parties {
        let written: i128 = o.report.writes.iter().filter(|w| w.party == p).map(|w| w.amount).sum();
        assert_eq!(written, o.books.equity(p), "each party's equity is what its writes put on its books");
    }
    assert!(o.report.writes.iter().all(|w| w.counter != w.party), "every write names the party that answers it");
}

#[test]
fn dues_are_paid_from_the_payers_money() {
    let mut o = opened();
    let cal = calendar();
    let paid = o.books.pay_dues(o.due, &cal, &mut Quiet);
    assert_eq!((paid.lines, paid.instructions, paid.settled), (3, 6, 4), "C can pay neither its interest nor its loan");
    let [cb, b0, b1, a, b, c] = o.parties;
    let [la, lb, lc] = o.loans;
    // A month's interest over 31 days at 12%: 5 095.89 on A's 500 000, 1 019.18 on B's 100 000, to the nearest.
    let (ia, ib) = (5_096, 1_019);
    assert_eq!(balance(&o.books, a, o.deposits[0], Side::Asset), 1_000_000 - ia - 500_000);
    assert_eq!(balance(&o.books, b, o.deposits[1], Side::Asset), 1_000_000 - ib - 100_000);
    assert_eq!(balance(&o.books, c, o.deposits[0], Side::Asset), 1_000, "C's payments failed whole");
    assert_eq!(balance(&o.books, b1, o.reserves, Side::Asset), 5_000_000 - ib - 100_000, "B's bank paid for it");
    assert_eq!(balance(&o.books, b0, o.reserves, Side::Asset), 5_000_000 + ib + 100_000);
    assert_eq!(balance(&o.books, cb, o.reserves, Side::Liability), -10_000_000, "reserves moved, none made");
    for (party, line) in [(a, la), (b, lb)] {
        assert_eq!(balance(&o.books, party, line, Side::Liability), 0, "the principal repaid clears the loan");
    }
    assert_eq!(balance(&o.books, c, lc, Side::Liability), -1_000_000);
    assert!(o.loans.iter().all(|l| o.books.ledger.lines.done(*l)), "the loans' one date is spent");
    let fails = o.books.ledger.fails();
    assert_eq!(fails.len(), 2);
    assert!(fails.iter().all(|f| f.party == c && f.cause == FailCause::Funds));
    assert!(
        fails.iter().all(|f| f.row == Missing::Present(crate::instruction::DueRow { line: lc, side: Side::Liability }))
    );
}
