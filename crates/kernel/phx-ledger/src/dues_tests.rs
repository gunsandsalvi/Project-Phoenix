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
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = banks.map(|_| books.ledger.lines.open(deposit_kind, account, Missing::Absent));
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
        books.ledger.lines.open(loan_kind, id, Missing::Present((due, 1)))
    });
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
    let due = o.books.ledger.mark_due(o.due, &cal);
    let paid = o.books.settle_day(
        &due,
        o.due,
        &cal,
        &crate::pending::Closed::default(),
        &crate::cleared::test_draws,
        &mut Quiet,
    );
    assert_eq!((paid.lines, paid.payments, paid.settled, paid.failed), (3, 3, 2, 1), "C cannot pay its loan's dues");
    assert_eq!((paid.unsound, paid.not_maximal, paid.nets_missed, paid.reserves_missed), (0, 0, 0, 0));
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
    assert_eq!(fails.len(), 1, "a row's dues settle or fail together");
    assert!(fails.iter().all(|f| f.party == c && f.cause == FailCause::Funds));
    assert!(
        fails.iter().all(|f| f.row == Missing::Present(crate::instruction::DueRow { line: lc, side: Side::Liability }))
    );
}

const WAGE: LineKindDecl = LineKindDecl {
    name: "wage",
    asset: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true, holder_roles: &[], exclusive: false },
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

/// Two payers owing a line of 1 000 a member a month — the first for 2 members with 10 000 on deposit at the first
/// bank, the second for 3 with 100 at the second — and three claimants holding 1, 3 and 1 members, one at each bank
/// and one at the first: no pairing between them is recorded.
#[test]
fn a_cleared_line_pays_row_by_row_and_draws_who_loses() {
    let size = BooksSize { rows: 32, rows_per_chunk: 32, instruments: 16, lines: 16, per_chunk: 16, blocks: 32 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let at = |books: &mut Books<Heap>, kind| books.parties.begin(kind, TileId::new(0), Day::new(0));
    let cb = at(&mut books, "central bank");
    let banks = [at(&mut books, "bank"), at(&mut books, "bank")];
    let payers = [at(&mut books, "firm"), at(&mut books, "firm")];
    let claimants = [at(&mut books, "firm"), at(&mut books, "firm"), at(&mut books, "firm")];
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let wage_kind = books.ledger.lines.declare_money(WAGE).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, monthly()));
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = banks.map(|_| books.ledger.lines.open(deposit_kind, account, Missing::Absent));
    let wage_terms = Terms {
        legs: vec![Leg::FixedAmount(Money::new(1_000, EUR))],
        schedule: Schedule { dates: monthly(), count: Missing::Absent },
        ..Terms::account(EUR, monthly())
    };
    let wage_terms = books.ledger.terms.intern(wage_terms);
    let due = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let wage = books.ledger.lines.open(wage_kind, wage_terms, Missing::Present((due, 1)));
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let banked =
        [(payers[0], 0, 10_000), (payers[1], 1, 100), (claimants[0], 0, 0), (claimants[1], 1, 0), (claimants[2], 0, 0)];
    let mut legs = vec![open(cb, reserves, Side::Liability, 2, BALANCE)];
    for ((bank, line), depositors) in banks.iter().zip(deposits).zip([3, 2]) {
        legs.push(open(*bank, reserves, Side::Asset, 1, BALANCE));
        legs.push(open(*bank, line, Side::Liability, depositors, BALANCE));
    }
    for (party, bank, _) in banked {
        legs.push(open(party, deposits[bank], Side::Asset, 1, BALANCE | PENDING));
    }
    for (party, count) in payers.iter().zip([2, 3]) {
        legs.push(open(*party, wage, Side::Liability, count, BALANCE));
    }
    for (party, count) in claimants.iter().zip([1, 3, 1]) {
        legs.push(open(*party, wage, Side::Asset, count, BALANCE));
    }
    let mut report = GenReport::default();
    books.open(reason, legs, 0, &mut report);
    let mut writes = Vec::new();
    for bank in banks {
        writes.extend([write(bank, reserves, Side::Asset, 50_000), write(cb, reserves, Side::Liability, -50_000)]);
    }
    for (party, bank, amount) in banked.into_iter().filter(|b| b.2 > 0) {
        writes.push(write(party, deposits[bank], Side::Asset, amount));
        writes.push(write(banks[bank], deposits[bank], Side::Liability, -amount));
    }
    books.open(reason, writes, 1, &mut report);

    let marked = books.ledger.mark_due(due, &cal);
    let paid = books.settle_day(
        &marked,
        due,
        &cal,
        &crate::pending::Closed::default(),
        &crate::cleared::test_draws,
        &mut Quiet,
    );
    assert_eq!(
        (paid.payments, paid.failed, paid.lost),
        (5, 1, 3),
        "every row its own payment; the second payer's 3 lost"
    );
    assert_eq!((paid.unsound, paid.not_maximal, paid.nets_missed, paid.reserves_missed), (0, 0, 0, 0));
    assert_eq!(balance(&books, payers[0], deposits[0], Side::Asset), 8_000, "the first payer paid for its 2 members");
    assert_eq!(balance(&books, payers[1], deposits[1], Side::Asset), 100, "the second failed whole");
    let received: Vec<i64> =
        claimants.iter().zip([0, 1, 0]).map(|(c, b)| balance(&books, *c, deposits[b], Side::Asset)).collect();
    assert_eq!(received.iter().sum::<i64>(), 2_000, "the claimants are paid what the payers paid");
    for (r, count) in received.iter().zip([1, 3, 1]) {
        assert!(*r % 1_000 == 0 && *r <= 1_000 * count, "whole members, within each row's count");
    }
    let held: i64 = banks.iter().map(|b| balance(&books, *b, reserves, Side::Asset)).sum();
    assert_eq!(held, 100_000, "reserves moved between the banks, none made");
    assert_eq!(balance(&books, cb, reserves, Side::Liability), -100_000);
}

/// The line above, but the second payer and the third claimant hold no account: the payer's dues fail for want of
/// money, its members' dues lost by claimant members drawn for them, and the claimant's own due is lost against the
/// top issuer, which owes no row of the line.
#[test]
fn a_cleared_line_fails_the_rows_of_holders_with_no_money() {
    let size = BooksSize { rows: 32, rows_per_chunk: 32, instruments: 16, lines: 16, per_chunk: 16, blocks: 32 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let at = |books: &mut Books<Heap>, kind| books.parties.begin(kind, TileId::new(0), Day::new(0));
    let cb = at(&mut books, "central bank");
    let banks = [at(&mut books, "bank"), at(&mut books, "bank")];
    let payers = [at(&mut books, "firm"), at(&mut books, "firm")];
    let claimants = [at(&mut books, "firm"), at(&mut books, "firm"), at(&mut books, "firm")];
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let wage_kind = books.ledger.lines.declare_money(WAGE).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, monthly()));
    let reserves = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = banks.map(|_| books.ledger.lines.open(deposit_kind, account, Missing::Absent));
    let wage_terms = Terms {
        legs: vec![Leg::FixedAmount(Money::new(1_000, EUR))],
        schedule: Schedule { dates: monthly(), count: Missing::Absent },
        ..Terms::account(EUR, monthly())
    };
    let wage_terms = books.ledger.terms.intern(wage_terms);
    let due = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let wage = books.ledger.lines.open(wage_kind, wage_terms, Missing::Present((due, 1)));
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let banked = [(payers[0], 0, 10_000), (claimants[0], 0, 0), (claimants[1], 1, 0)];
    let mut legs = vec![open(cb, reserves, Side::Liability, 2, BALANCE)];
    for ((bank, line), depositors) in banks.iter().zip(deposits).zip([2, 1]) {
        legs.push(open(*bank, reserves, Side::Asset, 1, BALANCE));
        legs.push(open(*bank, line, Side::Liability, depositors, BALANCE));
    }
    for (party, bank, _) in banked {
        legs.push(open(party, deposits[bank], Side::Asset, 1, BALANCE | PENDING));
    }
    for (party, count) in payers.iter().zip([2, 3]) {
        legs.push(open(*party, wage, Side::Liability, count, BALANCE));
    }
    for (party, count) in claimants.iter().zip([1, 3, 1]) {
        legs.push(open(*party, wage, Side::Asset, count, BALANCE));
    }
    let mut report = GenReport::default();
    books.open(reason, legs, 0, &mut report);
    let mut writes = Vec::new();
    for bank in banks {
        writes.extend([write(bank, reserves, Side::Asset, 50_000), write(cb, reserves, Side::Liability, -50_000)]);
    }
    for (party, bank, amount) in banked.into_iter().filter(|b| b.2 > 0) {
        writes.push(write(party, deposits[bank], Side::Asset, amount));
        writes.push(write(banks[bank], deposits[bank], Side::Liability, -amount));
    }
    books.open(reason, writes, 1, &mut report);

    let marked = books.ledger.mark_due(due, &cal);
    let paid = books.settle_day(
        &marked,
        due,
        &cal,
        &crate::pending::Closed::default(),
        &crate::cleared::test_draws,
        &mut Quiet,
    );
    assert_eq!((paid.payments, paid.lost), (5, 3), "every row its own payment; the second payer's 3 lost");
    assert_eq!((paid.unsound, paid.not_maximal, paid.nets_missed, paid.reserves_missed), (0, 0, 0, 0));
    let fails: Vec<(PartyId, FailCause)> = books.ledger.fails().iter().map(|f| (f.party, f.cause)).collect();
    assert_eq!(fails, vec![(payers[1], FailCause::NoMoney)], "only the payer with no money is in arrears");
    assert_eq!(balance(&books, payers[0], deposits[0], Side::Asset), 8_000, "the first payer paid for its 2 members");
    let received: Vec<i64> =
        claimants[..2].iter().zip([0, 1]).map(|(c, b)| balance(&books, *c, deposits[b], Side::Asset)).collect();
    assert!(received.iter().sum::<i64>() <= 2_000, "the claimants with money are paid no more than was paid");
    for (r, count) in received.iter().zip([1, 3]) {
        assert!(*r % 1_000 == 0 && *r <= 1_000 * count, "whole members, within each row's count");
    }
    let held: i64 = banks.iter().map(|b| balance(&books, *b, reserves, Side::Asset)).sum();
    assert_eq!(i128::from(held), -i128::from(balance(&books, cb, reserves, Side::Liability)), "reserves balance");
}
