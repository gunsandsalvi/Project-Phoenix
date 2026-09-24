#![cfg(test)]
//! Landings of parts in one table's cells, and re-keys of whole cells.

use phx_id::{Day, LineId, PartyId};
use phx_ledger::algebra::Side;
use phx_ledger::part::cell_holdings;
use phx_ledger::rows::rows;
use phx_num::Missing;

use crate::check::{Refused, RowFacts, View, check};
use crate::fixture::{MapIndex, Points, Spec, Ten};
use crate::key::KeyId;
use crate::landing::{Landed, LandingIndex, land};
use crate::part::Part;
use crate::rekey::{KeyClock, clock_day, clocks_ended, rekey_flagged, set_key};
use crate::steps::StepTable;

const PLAIN: Spec = Spec { weight: 100, income: 10_000, deposit: 50_000, loan: None };

type Rows = Vec<(LineId, u32, Missing<i64>)>;

/// A cell's weight, income, rows and holdings, by its identity.
fn state(ten: &Ten, party: PartyId) -> (u32, i64, Rows, usize) {
    let slot = ten.table.slots().find(|s| ten.table.party(*s) == party).expect("a live cell");
    let r = rows(&ten.table, slot).iter().map(|v| (v.row.line, v.row.count, v.optional.balance)).collect();
    (ten.table.weight(slot).get(), ten.table.position(slot, 0), r, cell_holdings(&ten.table, slot).len())
}

fn facts(line: u32, count: u32, balance: i64, record: u32) -> RowFacts {
    RowFacts {
        line: LineId::new(line),
        side: Side::Liability,
        count,
        record,
        point: 0,
        balance: Missing::Present(balance),
        since: Missing::Absent,
    }
}

fn view(rows: Vec<RowFacts>, steps: &[u16]) -> View {
    let t = StepTable::new(&phx_core::register::values::Partition { exp: 0, bounds: (1..=9).collect() }).unwrap();
    View {
        key: Missing::Present(KeyId::new(0)),
        sig: vec![],
        steps: steps.iter().map(|n| t.step_of(i64::from(*n))).collect(),
        rates: vec![Missing::Present(1)],
        attention: vec![Missing::Absent],
        exposed: vec![false],
        rows,
    }
}

#[test]
fn landing_check_refuses_kink() {
    // A credit limit of 1 000 a member: a part owing 1 200 a member and a cell owing 800 lie across it.
    let limit = Points(vec![(LineId::new(4), Side::Liability, 1_000)]);
    let part = view(vec![facts(4, 5, 6_000, 0)], &[3]);
    let over = view(vec![facts(4, 10, 8_000, 0)], &[3]);
    assert_eq!(check(&part, &over, &limit), Err(Refused::Kink(LineId::new(4))));
    let alike = view(vec![facts(4, 10, 11_000, 0)], &[3]);
    assert_eq!(check(&part, &alike, &limit), Ok(()), "both over the limit, one per member at 1 100");
    // A payment fallen due and missed by the part's members and paid by the cell's: their records differ.
    let missed = view(vec![facts(4, 10, 11_000, 1 << 16)], &[3]);
    assert_eq!(check(&part, &missed, &limit), Err(Refused::Record(LineId::new(4))));
    assert_eq!(check(&part, &view(vec![], &[3]), &limit), Ok(()), "a line only one side holds is no kink between them");
}

#[test]
fn check_refuses_hash_collision() {
    let part = view(vec![], &[3, 1]);
    let collided = view(vec![], &[3, 2]);
    assert_eq!(check(&part, &collided, &Points::default()), Err(Refused::Steps), "a landing key only proposes");
    // An index whose one entry under the part's landing key is a cell of other steps: the part never joins it.
    let mut ten = Ten::new();
    let (b, sb) = ten.add(1, Spec { income: 6_000, ..PLAIN });
    let (_, sa) = ten.add(1, PLAIN);
    let p = ten.split(sa, 0, 20, "DEM.death");
    let mut collided = MapIndex::default();
    collided.insert(ten.table.hot(sa).landing_key, b, sb);
    let landed = land(&mut ten.tenb(), &mut collided, vec![p]);
    assert_eq!((landed.landings, landed.new_cells), (0, 1), "a new cell rather than a join across steps");
}

#[test]
fn join_adds_everything_exactly() {
    let mut ten = Ten::new();
    let (b, sb) = ten.add(1, Spec { weight: 50, income: 5_000, deposit: 20_000, loan: Some((10, 3_000)) });
    let (_, sa) = ten.add(1, Spec { loan: Some((40, 12_000)), ..PLAIN });
    let before_a = (ten.table.position(sa, 0), ten.table.weight(sa).get());
    let before_b = state(&ten, b);
    let p = ten.split(sa, 0, 30, "DEM.death");
    let (pw, pi) = (p.weight.get(), p.positions[0]);
    let prow: Rows = p.rows.iter().map(|d| (d.line(), d.row.count, d.optional.balance)).collect();
    let mut index = std::mem::take(&mut ten.index);
    let landed = land(&mut ten.tenb(), &mut index, vec![p]);
    assert_eq!((landed.landings, landed.new_cells), (1, 0));
    let after_b = state(&ten, b);
    assert_eq!(after_b.0, before_b.0 + pw);
    assert_eq!(after_b.1, before_b.1 + pi);
    for (line, count, balance) in &after_b.2 {
        let (_, c0, b0) = before_b.2.iter().find(|r| r.0 == *line).copied().unwrap_or((*line, 0, Missing::Present(0)));
        let (_, cp, bp) = prow.iter().find(|r| r.0 == *line).copied().unwrap_or((*line, 0, Missing::Present(0)));
        let sum = |x: Missing<i64>| match x {
            Missing::Present(v) => v,
            Missing::Absent => 0,
        };
        assert_eq!((*count, sum(*balance)), (c0 + cp, sum(b0) + sum(bp)), "row {line:?}");
    }
    assert_eq!(ten.table.position(sa, 0) + after_b.1, before_a.0 + before_b.1, "no total moved");
    assert_eq!(ten.table.weight(sa).get() + after_b.0, before_a.1 + before_b.0);
    let profile = ten.table.profile(sb);
    assert_eq!(profile.members(0), u64::from(after_b.0), "profile counts added with the weight");
    assert!(landed.erased.first().is_some_and(|e| *e >= 0.0));
}

#[test]
fn join_touches_holder_list_only_on_first_or_last_row() {
    // b holds the loan: a part of a's borrowers adds its count to b's row and touches no list.
    let mut ten = Ten::new();
    let _ = ten.add(1, Spec { weight: 10, income: 1_000, deposit: 5_000, loan: Some((10, 3_000)) });
    let (_, sa) = ten.add(1, Spec { loan: Some((100, 30_000)), ..PLAIN });
    let p = ten.split(sa, 0, 10, "DEM.death");
    let mut index = std::mem::take(&mut ten.index);
    let landed = land(&mut ten.tenb(), &mut index, vec![p]);
    assert_eq!((landed.landings, landed.holder_list_changes), (1, 0), "a count added to a row held touches no list");
    // c holds none: the part's loan row is its first on the line, so c enters the loan's list, and only it.
    let mut ten = Ten::new();
    let (c, _) = ten.add(1, Spec { weight: 10, income: 1_000, deposit: 5_000, loan: None });
    let (_, sa) = ten.add(1, Spec { loan: Some((100, 30_000)), ..PLAIN });
    let p = ten.split(sa, 0, 10, "DEM.death");
    let mut index = std::mem::take(&mut ten.index);
    let landed = land(&mut ten.tenb(), &mut index, vec![p]);
    assert_eq!(landed.resolved.first().map(|(_, cell)| *cell), Some(c));
    assert_eq!(landed.holder_list_changes, 1, "the loan's list, which the deposit side does not keep");
    assert_eq!(ten.books.ledger.lines.holders(ten.books.loan).count(), 2);
}

/// Parts from three cells, landed in one order or another, over identical tables.
fn landed_in(order: &[usize]) -> (Landed, Vec<(u32, i64)>) {
    let mut ten = Ten::new();
    let _ = ten.add(1, PLAIN);
    let (_, s1) = ten.add(1, PLAIN);
    let (_, s2) = ten.add(1, Spec { income: 6_000, ..PLAIN });
    let (_, s3) = ten.add(2, PLAIN);
    let mut parts: Vec<Part> = vec![
        ten.split(s1, 0, 10, "DEM.death"),
        ten.split(s1, 1, 5, "DEM.illness"),
        ten.split(s2, 0, 20, "DEM.death"),
        ten.split(s3, 0, 7, "DEM.death"),
    ];
    let mut key = ten.key(3);
    std::mem::swap(&mut parts[3].key, &mut key);
    let ordered: Vec<Part> = order.iter().map(|i| parts[*i].clone()).collect();
    let mut index = std::mem::take(&mut ten.index);
    let landed = land(&mut ten.tenb(), &mut index, ordered);
    let mut cells: Vec<(u64, u32, i64)> = ten
        .table
        .slots()
        .map(|s| (ten.table.party(s).get(), ten.table.weight(s).get(), ten.table.position(s, 0)))
        .collect();
    cells.sort_unstable();
    (landed, cells.into_iter().map(|(_, w, i)| (w, i)).collect())
}

#[test]
fn landing_order_independent() {
    let (a, cells_a) = landed_in(&[0, 1, 2, 3]);
    let (b, cells_b) = landed_in(&[3, 2, 1, 0]);
    let (c, cells_c) = landed_in(&[2, 0, 3, 1]);
    assert_eq!(cells_a, cells_b);
    assert_eq!(cells_a, cells_c);
    let mut ra = a.resolved.clone();
    let mut rb = b.resolved.clone();
    ra.sort_unstable();
    rb.sort_unstable();
    assert_eq!(ra, rb, "every part in the same cell");
    assert_eq!((a.landings, a.new_cells), (c.landings, c.new_cells));
}

#[test]
fn clusters_canonical() {
    let (landed, _) = landed_in(&[3, 0, 1, 2]);
    assert_eq!(landed.new_cells, 1, "the part of a key no cell holds starts a cell");
    let mut ten = Ten::new();
    let (_, s1) = ten.add(1, PLAIN);
    let mut parts = vec![ten.split(s1, 0, 10, "DEM.death"), ten.split(s1, 1, 10, "DEM.death")];
    for p in &mut parts {
        p.key = ten.key(3);
    }
    parts.reverse();
    let mut index = std::mem::take(&mut ten.index);
    let landed = land(&mut ten.tenb(), &mut index, parts);
    assert_eq!((landed.new_cells, landed.landings), (1, 1), "the second joins the cell the first started");
    let first = landed.resolved.first().map(|(id, _)| id.seq);
    assert_eq!(first, Some(0), "clusters form in order of the parts' identities, whatever their order in the day");
}

#[test]
fn landing_independent_of_slots() {
    let run = |pad: bool| {
        let mut ten = Ten::new();
        if pad {
            // A cell of another key taken first, so every cell after it lies one slot further on.
            let _ = ten.add(0, PLAIN);
        }
        let (b, sb) = ten.add(1, PLAIN);
        let (_, sa) = ten.add(1, PLAIN);
        let p = ten.split(sa, 0, 10, "DEM.death");
        let mut index = std::mem::take(&mut ten.index);
        let landed = land(&mut ten.tenb(), &mut index, vec![p]);
        (landed.resolved.first().map(|(_, cell)| *cell) == Some(b), sb.get())
    };
    let (plain, padded) = (run(false), run(true));
    assert_ne!(plain.1, padded.1, "the cells lie in other slots");
    assert!(plain.0 && padded.0, "and the part lands in the same cell, the first by identity");
}

#[test]
fn whole_weight_part_rekeys_in_place() {
    let mut ten = Ten::new();
    let (a, sa) = ten.add(1, PLAIN);
    let mut cells =
        crate::split::Cells { ledger: &mut ten.books.ledger, table: &mut ten.table, place: 0, keys: &ten.keys };
    let spec = crate::split::SplitSpec {
        count: 100,
        given: &[],
        rows: &[],
        own: &[],
        reviewed: Missing::Absent,
        rounding: phx_num::round::Round::HalfEven,
    };
    let id = crate::part::PartId { origin: a, seq: 0 };
    let parted = crate::split::split(&mut cells, sa, id, &spec, &mut crate::fixture::draws("DEM.move", 0));
    assert_eq!(parted, crate::split::Parted::Whole);
    let moved = ten.key(2);
    let mut index = std::mem::take(&mut ten.index);
    let mut landed = Landed::default();
    set_key(&mut ten.tenb(), sa, moved);
    rekey_flagged(&mut ten.tenb(), &mut index, &[sa], &mut landed);
    assert_eq!(ten.table.party(sa), a, "the cell keeps its identity");
    assert_eq!(ten.keys.record(ten.table.hot(sa).key_id), moved);
    assert_eq!(index.candidates(ten.table.hot(sa).landing_key), [(a, sa)], "and is found under its new landing key");
}

#[test]
fn rekey_on_step_crossing() {
    let mut ten = Ten::new();
    let (a, sa) = ten.add(1, Spec { income: 6_000, ..PLAIN });
    let (b, sb) = ten.add(1, PLAIN);
    assert_ne!(ten.table.hot(sa).landing_key, ten.table.hot(sb).landing_key, "60 and 100 a member lie in other steps");
    // Income paid to a's members carries them from 60 to 100 a member, across a step boundary.
    ten.table.set_position(sa, 0, 10_000);
    let mut index = std::mem::take(&mut ten.index);
    let mut landed = Landed::default();
    let rekeys = rekey_flagged(&mut ten.tenb(), &mut index, &[sa], &mut landed);
    assert_eq!((rekeys, landed.landings), (1, 1));
    assert!(!ten.table.is_live(sa), "a lands in b whole");
    assert_eq!(state(&ten, b).0, 200);
    assert!(
        matches!(ten.directory.resolve(a), phx_core::Resolved::Unknown),
        "a has ended, and with no record naming it leaves no tombstone"
    );
    let _ = sb;
}

#[test]
fn key_clocks_share_one_reason() {
    let d = Day::new;
    assert_eq!(clock_day(&[d(40), d(12), d(30)]), Missing::Present(d(12)), "the earliest end");
    assert_eq!(clock_day(&[]), Missing::Absent);
    let ten = Ten::new();
    let key = ten.key(3);
    let clocks = [(KeyClock { attr: 0, end_value: 0 }, d(12))];
    assert_eq!(clocks_ended(&ten.kind.key, key, &clocks, d(11)), key, "before its end, as it was");
    assert_eq!(clocks_ended(&ten.kind.key, key, &clocks, d(12)), ten.key(0), "on it, the declared end value");
}
