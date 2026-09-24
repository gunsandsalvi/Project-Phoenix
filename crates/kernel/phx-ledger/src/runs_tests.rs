//! Due-day runs over real kind tables: which rows a holder's run gives on each day, and its head after rows move.
#![cfg(test)]

use phx_core::calendar::Calendar;
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::{GenReport, kind_tables::RunHead};
use phx_id::{CountryId, Date, Day, LineId, PartyId, TileId, Weekday};
use phx_num::{Ccy, Missing, UnitId};
use phx_store::HeapBacking;

use crate::algebra::{Leg, Schedule, Side, Terms};
use crate::apply::Holders;
use crate::books::{Books, BooksSize};
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, RowOp};
use crate::line::{LineKindDecl, NewRow, SideDecl};
use crate::rows::{BALANCE, Optional};
use crate::runs::{due_rows, rehead, segment, truth};

type Heap = HeapBacking<4096>;
const EUR: Ccy = Ccy::new(0);
const fn kind(dated: bool) -> LineKindDecl {
    LineKindDecl {
        name: if dated { "dated" } else { "undated" },
        asset: SideDecl { holder_kinds: &["bank"], words: BALANCE, holder_list: true },
        liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
        transfer_requesters: &["BNK"],
        dated,
    }
}

fn calendar() -> Calendar {
    let rules =
        CountryRules { weekend: WeekendRule { days: vec![Weekday::Saturday, Weekday::Sunday] }, holidays: vec![] };
    Calendar::new(Date::new(1950, 1, 1).unwrap(), vec![(CountryId::new(0), rules)], 2025).unwrap()
}

fn schedule(day: u8, count: u32) -> Schedule {
    let dates = ScheduleDates {
        anchor: Date::new(2026, 1, day).unwrap(),
        period: Period::months(1).unwrap(),
        eom: EndOfMonth::Plain,
        convention: BusinessDayConvention::Following,
        country: CountryId::new(0),
    };
    Schedule { dates, count: Missing::Present(count) }
}

fn open(party: PartyId, line: LineId, side: Side) -> LegRec {
    let optional = Optional { balance: Missing::Present(0), pending: Missing::Absent, amount: Missing::Absent };
    LegRec {
        party,
        account: AccountRef::Line { line, side },
        qty: 1,
        denom: Denom::Unit(UnitId::new(0)),
        kind: LegKind::Row(RowOp::Open(NewRow { side, within: 0, count: 1, point: 0, optional })),
    }
}

/// A bank and a firm; the firm owes three dated lines, monthly from the 5th, 12th and 20th of January 2026 for two,
/// three and four dates, and one undated line, opened in the order undated, dated, dated, dated so the segment is
/// built by insertion before the undated row.
struct Fixture {
    books: Books<Heap>,
    cal: Calendar,
    firm: PartyId,
    dated: Vec<LineId>,
}

fn fixture() -> Fixture {
    let size = BooksSize { rows: 16, rows_per_chunk: 16, instruments: 16, lines: 16, per_chunk: 16, blocks: 16 };
    let mut books: Books<Heap> = Books::new(&["bank", "firm"], size);
    let cal = calendar();
    let bank = books.parties.begin("bank", TileId::new(0), Day::new(0));
    let firm = books.parties.begin("firm", TileId::new(0), Day::new(0));
    let (dated_kind, undated_kind) =
        (books.ledger.lines.declare_money(kind(true)).index(), books.ledger.lines.declare_money(kind(false)).index());
    let plain = books.ledger.terms.intern(Terms::account(EUR, schedule(1, 1).dates));
    let undated = books.ledger.lines.open(undated_kind, plain, Missing::Absent);
    let mut lines = vec![undated];
    let mut dated = Vec::new();
    for (day, count) in [(5, 2), (12, 3), (20, 4)] {
        let s = schedule(day, count);
        let terms = Terms {
            legs: vec![Leg::Delivery(phx_num::Qty::new(0, UnitId::new(0)))],
            schedule: s,
            ..Terms::account(EUR, s.dates)
        };
        let id = books.ledger.terms.intern(terms);
        let line = books.ledger.lines.open(dated_kind, id, Missing::Present((s.dates.nth(&cal, 1), 1)));
        dated.push(line);
        lines.push(line);
    }
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    for line in lines {
        let legs = vec![open(bank, line, Side::Asset), open(firm, line, Side::Liability)];
        books.open(reason, legs, 0, &mut GenReport::default());
    }
    Fixture { books, cal, firm, dated }
}

fn head(books: &Books<Heap>, party: PartyId) -> RunHead {
    let (place, slot) = books.parties.row(party);
    books.parties.table(place).run_head(slot)
}

#[test]
fn run_head_skips_until_due() {
    let mut f = fixture();
    let (place, slot) = f.books.parties.row(f.firm);
    let first = f.cal.day(Date::new(2026, 1, 1).unwrap()).unwrap();
    let mut scanned_days = 0;
    for d in 0..200 {
        let day = Day::new(first.get() + d);
        let head_before = head(&f.books, f.firm);
        let due = f.books.ledger.mark_due(day, &f.cal);
        let expected: Vec<LineId> = f.dated.iter().copied().filter(|l| due.is_due(*l)).collect();
        let truth = truth(f.books.parties.table(place), slot, day, &due, &f.books.ledger.lines);
        assert!(truth.holds, "day {d}: the run holds against the holder's rows");
        assert_eq!(truth.due, u64::try_from(expected.len()).unwrap());
        let taken = due_rows(f.books.parties.table(place), slot, day, &due);
        match taken {
            None => {
                assert!(expected.is_empty(), "day {d}: rows fell due but the run was not scanned");
                assert!(head_before.len == 0 || head_before.next_due > day.get(), "day {d}: a due head was passed");
            }
            Some((rows, _)) => {
                scanned_days += 1;
                assert_eq!(head_before.next_due, day.get(), "day {d}: scanned on a day that is not its head's");
                let got: Vec<LineId> = rows.iter().map(|r| r.row.line).collect();
                assert_eq!(got, expected, "day {d}: the rows taken are those due");
                rehead(Holders::arenas(&mut f.books.parties, place), slot, &f.books.ledger.lines);
            }
        }
    }
    assert_eq!(scanned_days, 2 + 3 + 4, "each date of each line, none sharing a day");
    assert_eq!(head(&f.books, f.firm).len, 0, "the spent lines' rows left the segment");
    let rows = crate::rows::rows(f.books.parties.table(place), slot);
    assert_eq!(rows.len(), 4, "no row was lost moving out of the segment");
}

#[test]
fn run_head_lower_bound_after_moves() {
    let mut f = fixture();
    let (place, slot) = f.books.parties.row(f.firm);
    let segment_lines = |f: &Fixture| -> Vec<LineId> {
        let h = head(&f.books, f.firm);
        segment(f.books.parties.table(place), slot, h).map(|r| r.row.line).collect()
    };
    assert_eq!(segment_lines(&f), f.dated, "the dated rows, and only they, make the segment, in opening order");
    let earliest = |f: &Fixture| {
        segment_lines(f)
            .iter()
            .map(|l| f.books.ledger.lines.next_due(*l).get())
            .fold(None, |m: Option<u32>, d| Some(m.map_or(d, |m| if d < m { d } else { m })))
    };
    assert_eq!(Some(head(&f.books, f.firm).next_due), earliest(&f));
    f.books.ledger.lines.remove_row(
        Holders::arenas(&mut f.books.parties, place),
        place,
        slot,
        f.dated[0],
        Side::Liability,
    );
    assert_eq!(segment_lines(&f), f.dated[1..].to_vec(), "a row leaving the segment shortens it");
    let h = head(&f.books, f.firm);
    assert!(earliest(&f).is_some_and(|e| h.next_due <= e), "an early head after a row leaves stays a lower bound");
    rehead(Holders::arenas(&mut f.books.parties, place), slot, &f.books.ledger.lines);
    assert_eq!(Some(head(&f.books, f.firm).next_due), earliest(&f), "a scan makes the head exact again");
}
