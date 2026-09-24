//! Stage 7 over small books: the greatest set of payments that can settle, the prefix rule, rings, the worklist's
//! revisits, and a bank that cannot cover its net.
#![cfg(test)]

use phx_core::calendar::Calendar;
use phx_core::calendar::bizday::BusinessDayConvention;
use phx_core::calendar::period::{EndOfMonth, Period, ScheduleDates};
use phx_core::calendar::rules::{CountryRules, WeekendRule};
use phx_core::{AuditStream, GenReport, LegDigest};
use phx_id::{CountryId, Date, Day, LineId, PartyId, Slot, TableId, TileId, Weekday};
use phx_num::{Ccy, Missing, Money, UnitId};
use phx_store::HeapBacking;

use crate::algebra::{Leg, Schedule, Side, Terms};
use crate::books::{Books, BooksSize};
use crate::instruction::{AccountRef, Denom, Effect, LegKind, LegRec, ReasonDecl, RowOp};
use crate::line::{LineKindDecl, NewRow, SideDecl};
use crate::money::MoneyHolders;
use crate::pending::Closed;
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
const INVOICE: LineKindDecl = LineKindDecl {
    name: "invoice",
    asset: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
    liability: SideDecl { holder_kinds: &["firm"], words: BALANCE, holder_list: true },
    transfer_requesters: &["FRM"],
    dated: true,
};

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

/// A world of firms at two banks: `funds[i]` on firm i's deposit at bank `i % 2`, each bank holding `reserves` at the
/// central bank, and an invoice per edge `(payer, payee, amount)` falling due on 16 February 2026, opened in the
/// order given, which is each payer's payment order.
struct World {
    books: Books<Heap>,
    cal: Calendar,
    day: Day,
    firms: Vec<PartyId>,
    banks: [PartyId; 2],
    deposits: [LineId; 2],
    reserves: LineId,
}

fn world(funds: &[i64], edges: &[(usize, usize, i64)], reserves: [i64; 2]) -> World {
    let size = BooksSize { rows: 64, rows_per_chunk: 64, instruments: 16, lines: 64, per_chunk: 64, blocks: 64 };
    let mut books: Books<Heap> = Books::new(&["central bank", "bank", "firm"], size);
    let cal = calendar();
    let day = cal.day(Date::new(2026, 2, 16).unwrap()).unwrap();
    let begin = |books: &mut Books<Heap>, kind| books.parties.begin(kind, TileId::new(0), Day::new(0));
    let cb = begin(&mut books, "central bank");
    let banks = [begin(&mut books, "bank"), begin(&mut books, "bank")];
    let firms: Vec<PartyId> = funds.iter().map(|_| begin(&mut books, "firm")).collect();
    let reserves_kind = books.ledger.lines.declare_reserves(HOLDERS.reserves()).index();
    let deposit_kind = books.ledger.lines.declare_deposits(HOLDERS.deposits("current account")).index();
    let invoice_kind = books.ledger.lines.declare_money(INVOICE).index();
    let account = books.ledger.terms.intern(Terms::account(EUR, dates()));
    let reserves_line = books.ledger.lines.open(reserves_kind, account, Missing::Absent);
    let deposits = banks.map(|_| books.ledger.lines.open(deposit_kind, account, Missing::Absent));
    let reason = books.ledger.reasons.declare(ReasonDecl {
        name: "opening",
        order: 0,
        paid: Effect::Equity,
        received: Effect::Equity,
    });
    let at = |i: usize| i % 2;
    let mut legs = vec![open(cb, reserves_line, Side::Liability, 2, BALANCE)];
    for (b, bank) in banks.iter().enumerate() {
        let customers = u32::try_from(firms.iter().enumerate().filter(|(i, _)| at(*i) == b).count()).unwrap();
        legs.push(open(*bank, reserves_line, Side::Asset, 1, BALANCE));
        legs.push(open(*bank, deposits[b], Side::Liability, if customers == 0 { 1 } else { customers }, BALANCE));
    }
    for (i, firm) in firms.iter().enumerate() {
        legs.push(open(*firm, deposits[at(i)], Side::Asset, 1, BALANCE | PENDING));
    }
    let mut writes = Vec::new();
    for (b, bank) in banks.iter().enumerate() {
        writes.extend([
            write(*bank, reserves_line, Side::Asset, reserves[b]),
            write(cb, reserves_line, Side::Liability, -reserves[b]),
        ]);
    }
    for (i, firm) in firms.iter().enumerate() {
        writes.extend([
            write(*firm, deposits[at(i)], Side::Asset, funds[i]),
            write(banks[at(i)], deposits[at(i)], Side::Liability, -funds[i]),
        ]);
    }
    for &(payer, payee, amount) in edges {
        let terms = Terms {
            legs: vec![Leg::FixedAmount(Money::new(amount, EUR))],
            schedule: Schedule { dates: dates(), count: Missing::Present(1) },
            ..Terms::account(EUR, dates())
        };
        let id = books.ledger.terms.intern(terms);
        let line = books.ledger.lines.open(invoice_kind, id, Missing::Present((day, 1)));
        legs.push(open(firms[payee], line, Side::Asset, 1, BALANCE));
        legs.push(open(firms[payer], line, Side::Liability, 1, BALANCE));
    }
    books.open(reason, legs, 0, &mut GenReport::default());
    books.open(reason, writes, 1, &mut GenReport::default());
    World { books, cal, day, firms, banks, deposits, reserves: reserves_line }
}

fn balance(w: &World, party: PartyId, line: LineId, side: Side) -> i64 {
    let (place, slot) = w.books.parties.row(party);
    let row = crate::rows::iter(w.books.parties.table(place), slot).find(|r| r.row.line == line && r.side() == side);
    match row.map(|r| r.optional.balance) {
        Some(Missing::Present(b)) => b,
        _ => panic!("no balance"),
    }
}

/// Settles the day and gives each firm's deposit after it.
fn settle(w: &mut World) -> (crate::apply_batch::DaySettlement, Vec<i64>) {
    settle_closed(w, &Closed::default())
}

fn settle_closed(w: &mut World, closed: &Closed) -> (crate::apply_batch::DaySettlement, Vec<i64>) {
    let due = w.books.ledger.mark_due(w.day, &w.cal);
    let s = w.books.settle_day(&due, w.day, &w.cal, closed, &mut Quiet);
    let deposits = (0..w.firms.len()).map(|i| balance(w, w.firms[i], w.deposits[i % 2], Side::Asset)).collect();
    (s, deposits)
}

/// A small graph: each party's funds, and its payments as (payer, payee, amount).
type Graph<'a> = (&'a [i64], &'a [(usize, usize, i64)]);

/// The greatest set by brute force: every choice of how long a prefix of its payments each payer pays, kept where
/// every firm's deposit stays at or above nothing; the greatest is the longest prefix each payer has in any kept one.
fn brute_force(funds: &[i64], edges: &[(usize, usize, i64)]) -> Vec<bool> {
    let by_payer: Vec<Vec<usize>> =
        (0..funds.len()).map(|p| (0..edges.len()).filter(|e| edges[*e].0 == p).collect()).collect();
    let mut best = vec![0_usize; funds.len()];
    let mut choice = vec![0_usize; funds.len()];
    loop {
        let settles =
            |e: usize| by_payer[edges[e].0].iter().position(|x| *x == e).is_some_and(|k| k < choice[edges[e].0]);
        let ok = (0..funds.len()).all(|f| {
            let out: i64 = (0..edges.len()).filter(|e| settles(*e) && edges[*e].0 == f).map(|e| edges[e].2).sum();
            let inn: i64 = (0..edges.len()).filter(|e| settles(*e) && edges[*e].1 == f).map(|e| edges[e].2).sum();
            funds[f] + inn - out >= 0
        });
        if ok {
            for f in 0..funds.len() {
                if choice[f] > best[f] {
                    best[f] = choice[f];
                }
            }
        }
        let mut i = 0;
        loop {
            if i == funds.len() {
                return (0..edges.len())
                    .map(|e| by_payer[edges[e].0].iter().position(|x| *x == e).is_some_and(|k| k < best[edges[e].0]))
                    .collect();
            }
            choice[i] += 1;
            if choice[i] <= by_payer[i].len() {
                break;
            }
            choice[i] = 0;
            i += 1;
        }
    }
}

/// Which edges settled, read from the deposits' movement against what each edge would move.
fn settled_edges(funds: &[i64], edges: &[(usize, usize, i64)], after: &[i64]) -> Vec<bool> {
    let greatest = brute_force(funds, edges);
    let moved = |set: &[bool]| -> Vec<i64> {
        let mut d = funds.to_vec();
        for (e, &(p, q, a)) in edges.iter().enumerate() {
            if set[e] {
                d[p] -= a;
                d[q] += a;
            }
        }
        d
    };
    assert_eq!(moved(&greatest), after, "the deposits after settlement are those of the greatest set");
    greatest
}

#[test]
fn fixed_point_is_greatest_with_prefix_rule() {
    let rich = 1_000_000;
    // Funds 100, rows 60 then 50 to others: the first pays, the second fails, and nothing after it pays.
    let edges = [(0, 1, 60), (0, 2, 50), (0, 2, 10)];
    let mut w = world(&[100, 0, 0], &edges, [rich, rich]);
    let (s, after) = settle(&mut w);
    assert_eq!(after, vec![40, 60, 0], "a prefix: 60 paid, 50 short, and the 10 after it fails with it");
    assert_eq!((s.unsound, s.not_maximal, s.nets_missed, s.reserves_missed), (0, 0, 0, 0));
    // Its 59 variant: more funds never settle less.
    let mut w = world(&[110, 0, 0], &edges, [rich, rich]);
    assert_eq!(settle(&mut w).1, vec![0, 60, 50], "with 110 the first two pay and the 10 fails");
    // A ring of payers with nothing settles whole, since each is paid what it pays.
    let ring = [(0, 1, 100), (1, 2, 100), (2, 0, 100)];
    let mut w = world(&[0, 0, 0], &ring, [rich, rich]);
    let (s, after) = settle(&mut w);
    assert_eq!((after, s.settled, s.failed), (vec![0, 0, 0], 3, 0));
    // A short payer fails exactly its dependants: 0 cannot pay 1, so 1 cannot pay 2; 3 pays 2 on its own.
    let chain = [(0, 1, 50), (1, 2, 50), (3, 2, 20)];
    let mut w = world(&[10, 0, 0, 20], &chain, [rich, rich]);
    let (s, after) = settle(&mut w);
    assert_eq!((after, s.failed), (vec![10, 0, 20, 0], 2));
    // Small graphs against the brute-force greatest set.
    let graphs: [Graph<'_>; 4] = [
        (&[30, 20, 0, 5], &[(0, 1, 25), (1, 2, 40), (2, 3, 10), (3, 0, 15), (0, 2, 10)]),
        (&[0, 50, 0, 0], &[(0, 1, 30), (1, 0, 20), (1, 2, 40), (2, 3, 35), (3, 0, 5)]),
        (&[5, 5, 5, 5], &[(0, 1, 10), (1, 2, 10), (2, 3, 10), (3, 0, 10), (0, 2, 1)]),
        (&[100, 0, 0, 0], &[(0, 1, 70), (0, 3, 40), (1, 2, 70), (2, 3, 30), (3, 1, 60)]),
    ];
    for (funds, edges) in graphs {
        let mut w = world(funds, edges, [rich, rich]);
        let (s, after) = settle(&mut w);
        let greatest = settled_edges(funds, edges, &after);
        assert_eq!(s.settled, u64::try_from(greatest.iter().filter(|x| **x).count()).unwrap());
        assert_eq!((s.unsound, s.not_maximal, s.nets_missed, s.reserves_missed), (0, 0, 0, 0));
    }
}

#[test]
fn worklist_revisits_payees_and_banks() {
    let rich = 1_000_000;
    // 0 at the first bank cannot pay 1 at the second; 1 then cannot pay 2 at the first, nor 2 pay 3 at the second.
    let chain = [(0, 1, 50), (1, 2, 50), (2, 3, 50)];
    let mut w = world(&[10, 0, 0, 0], &chain, [rich, rich]);
    let (s, after) = settle(&mut w);
    assert_eq!((after, s.failed), (vec![10, 0, 0, 0], 3), "each failure took its payee again");
    let reserves = w.banks.map(|b| balance(&w, b, w.reserves, Side::Asset));
    assert_eq!(reserves, [rich, rich], "nothing crossed between the banks");
    // With 50 the whole chain settles, and the banks' reserves move by the nets: the first bank's customers pay 50
    // out twice and are paid 50 once.
    let mut w = world(&[50, 0, 0, 0], &chain, [rich, rich]);
    let (s, after) = settle(&mut w);
    assert_eq!((after, s.failed), (vec![0, 0, 0, 50], 0));
    let reserves = w.banks.map(|b| balance(&w, b, w.reserves, Side::Asset));
    assert_eq!(reserves, [rich - 50, rich + 50]);
}

#[test]
fn bank_net_removal_resettles() {
    // The first bank holds 30 in reserves; its customer 0 pays 100 to 1 at the second bank, which it holds, but its
    // bank cannot cover the net, so the payment is removed. Customer 2 at the first bank pays 3 at the second 20,
    // which the bank could cover alone, yet it is removed with the rest of its customers' payments through it;
    // 1 at the second bank pays 3 there 5, which never touches the first bank and settles.
    let edges = [(0, 1, 100), (2, 3, 20), (1, 3, 5)];
    let mut w = world(&[100, 10, 30, 0], &edges, [30, 1_000]);
    let (s, after) = settle(&mut w);
    assert_eq!(after, vec![100, 5, 30, 5]);
    assert_eq!((s.settled, s.failed), (1, 2));
    assert_eq!((s.unsound, s.nets_missed, s.reserves_missed), (0, 0, 0));
    let reserves = w.banks.map(|b| balance(&w, b, w.reserves, Side::Asset));
    assert_eq!(reserves, [30, 1_000], "no reserves moved: the settled payment stayed within the second bank");
}

fn pending_on(w: &World, i: usize) -> i64 {
    let (place, slot) = w.books.parties.row(w.firms[i]);
    let line = w.deposits[i % 2];
    let row =
        crate::rows::iter(w.books.parties.table(place), slot).find(|r| r.row.line == line && r.side() == Side::Asset);
    match row.map(|r| r.optional.pending) {
        Some(Missing::Present(p)) => p,
        _ => panic!("no pending word"),
    }
}

#[test]
fn pending_leg_outside_fixed_point() {
    // The second bank is closed. Firm 0 at the first bank owes 50 to firm 1 at the second and 60 to firm 2 at the
    // first, with 100: the leg through the closed bank is held pending, neither failing nor funding anyone, so the 60
    // settles; the 50 sits as pending on both deposits, and firm 0's funds exclude it from every later payment.
    let edges = [(0, 1, 50), (0, 2, 60)];
    let mut w = world(&[100, 0, 0], &edges, [1_000, 1_000]);
    let mut closed = Closed::default();
    closed.issuers.insert(w.banks[1]);
    let (s, after) = settle_closed(&mut w, &closed);
    assert_eq!((s.payments, s.pending, s.settled, s.failed), (2, 1, 1, 0));
    assert_eq!(after, vec![40, 0, 60], "only the payment within the open bank moved money");
    assert_eq!((pending_on(&w, 0), pending_on(&w, 1)), (50, 50));
    let (place, slot) = w.books.parties.row(w.firms[0]);
    let funds =
        crate::positions::PayerPositions::per_member_funds(w.books.parties.table(place), slot, w.deposits[0], 0);
    assert_eq!(funds, -10, "the payer's funds exclude what it holds pending");
}
