use std::cmp::Ordering;

use phx_id::{PartyId, Slot};
use phx_macros::clause;
use phx_num::round::Round;
use phx_num::{Missing, violation};
use phx_rand::{Draws, multivariate_hypergeometric};
use phx_store::Backing;

use crate::consts::RANK_BINS;
use crate::index::Index;
use crate::landing::{Landed, LandingIndex, TenB, land, place_part};
use crate::part::{Part, PartId};
use crate::split::{Cells, Parted, SplitSpec, split_batch};
use crate::table::CellTable;

/// A kind's ranks, RESOLUTION primitives: the position whose per-member value ranks its parties, the number of parties
/// carried as individuals from the top, and the lower rank an individual must fall below to rejoin the cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ranks {
    pub measure: usize,
    pub promote: u64,
    pub demote: u64,
}

/// What a rank read found: for each cell, how many of its members rise into the promotion rank; and the individuals
/// that fell below the demotion rank and may rejoin the cells.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RankRead {
    pub promote: Vec<(Slot, u32)>,
    pub demote: Vec<Slot>,
}

/// A row's per-member value of the measure, as a total over its members, compared exactly, with its histogram bin.
#[derive(Clone, Copy, Debug)]
struct Value {
    total: i64,
    weight: u32,
    bin: usize,
}

impl Value {
    fn new(total: i64, weight: u32) -> Value {
        // The bin is the per-member value's sign and bit length, so bins are in the values' order.
        let per = total.div_euclid(i64::from(weight));
        let bits = usize::try_from(u64::BITS - per.unsigned_abs().leading_zeros()).unwrap_or(0);
        let middle = RANK_BINS / 2;
        let bin = if per < 0 { middle - bits } else { middle + bits };
        Value { total, weight, bin }
    }

    fn cmp(self, other: Value) -> Ordering {
        (i128::from(self.total) * i128::from(other.weight)).cmp(&(i128::from(other.total) * i128::from(self.weight)))
    }
}

/// A row as a rank read sees it: its identity and slot, its per-member value, and whether it is an individual.
type RankRow = (PartyId, Slot, Value, bool);

/// Every live row with its identity, members and per-member value, and whether it is an individual.
fn rows<B: Backing>(table: &CellTable<B>, measure: usize) -> Vec<RankRow> {
    table
        .slots()
        .map(|s| {
            let v = Value::new(table.position(s, measure), table.weight(s).get());
            (table.party(s), s, v, table.hot(s).is_individual())
        })
        .collect()
}

/// The rows at the edge of the top `rank` members, highest first, and the members strictly above it. One pass builds a
/// histogram of members by bin, never sorting the population; only the rows of the bin the edge falls in are ordered.
fn edge(all: &[RankRow], members: &[u64; RANK_BINS], rank: u64) -> (Vec<RankRow>, Missing<Value>, u64) {
    let mut above = 0_u64;
    let mut at_bin = Missing::Absent;
    for b in (0..RANK_BINS).rev() {
        let m = members.get(b).copied().unwrap_or(0);
        if above + m >= rank && m > 0 {
            at_bin = Missing::Present(b);
            break;
        }
        above += m;
    }
    let Missing::Present(b) = at_bin else { return (Vec::new(), Missing::Absent, above) };
    let mut in_bin: Vec<RankRow> = all.iter().copied().filter(|r| r.2.bin == b).collect();
    in_bin.sort_by(|x, y| y.2.cmp(x.2).then(x.0.cmp(&y.0)));
    // Walk down the bin to the value at which the rank is reached; the rows holding it are the edge.
    let mut edge_value = Missing::Absent;
    for group in in_bin.chunk_by(|x, y| x.2.cmp(y.2) == Ordering::Equal) {
        let Some(first) = group.first() else { continue };
        let m: u64 = group.iter().map(|r| u64::from(r.2.weight)).sum();
        if above + m >= rank {
            edge_value = Missing::Present(first.2);
            break;
        }
        above += m;
    }
    let Missing::Present(v) = edge_value else { return (Vec::new(), Missing::Absent, above) };
    let at: Vec<RankRow> = in_bin.iter().copied().filter(|r| r.2.cmp(v) == Ordering::Equal).collect();
    (at, Missing::Present(v), above)
}

/// A kind's monthly rank read: every cell above the promotion rank's edge rises whole; at the edge, the individuals
/// holding its value keep their place and the rest of the rank is drawn by lot among the edge cells' members, the
/// cells in order of identity; every individual below the demotion rank's edge that `stays` does not keep may rejoin.
#[clause("REP.29", "REP.2", "REP.10")]
pub fn read_ranks<B: Backing>(
    table: &CellTable<B>,
    ranks: Ranks,
    stays: &dyn Fn(PartyId) -> bool,
    d: &mut Draws,
) -> RankRead {
    if ranks.demote <= ranks.promote {
        violation!(clause = "REP.29", "a demotion rank not below the promotion rank", promote = ranks.promote);
    }
    let all = rows(table, ranks.measure);
    // One pass over the rows: members by bin, which both ranks' edges read.
    let mut members = [0_u64; RANK_BINS];
    for (_, _, v, _) in &all {
        if let Some(m) = members.get_mut(v.bin) {
            *m += u64::from(v.weight);
        }
    }
    let mut read = RankRead::default();
    let (at, edge_value, above) = edge(&all, &members, ranks.promote);
    if let Missing::Present(v) = edge_value {
        for (_, slot, value, individual) in &all {
            if !individual && value.cmp(v) == Ordering::Greater {
                read.promote.push((*slot, value.weight));
            }
        }
        let mut edge_cells: Vec<(PartyId, Slot, u32)> = Vec::new();
        let mut edge_individuals = 0_u64;
        for (party, slot, value, individual) in at {
            if individual {
                edge_individuals += 1;
            } else {
                edge_cells.push((party, slot, value.weight));
            }
        }
        edge_cells.sort_unstable();
        let counts: Vec<u64> = edge_cells.iter().map(|(_, _, w)| u64::from(*w)).collect();
        let mut drawn = vec![0_u64; counts.len()];
        let held: u64 = counts.iter().sum();
        // The individuals at the edge keep their places; the room left is drawn among the edge cells' members, and
        // more individuals tying there than the rank has room for leave none.
        let take = match (ranks.promote - above).checked_sub(edge_individuals) {
            Some(room) if room < held => room,
            Some(_) => held,
            None => 0,
        };
        multivariate_hypergeometric(d, &counts, take, &mut drawn);
        for ((_, slot, _), n) in edge_cells.iter().zip(drawn) {
            if n > 0 {
                let Ok(n) = u32::try_from(n) else {
                    violation!(clause = "REP.29", "more members promoted than a cell holds");
                };
                read.promote.push((*slot, n));
            }
        }
    }
    let (_, demote_edge, _) = edge(&all, &members, ranks.demote);
    if let Missing::Present(v) = demote_edge {
        for (party, slot, value, individual) in &all {
            if *individual && value.cmp(v) == Ordering::Less && !stays(*party) {
                read.demote.push(*slot);
            }
        }
    }
    read.promote.sort_unstable_by_key(|(s, _)| table.party(*s));
    read.demote.sort_unstable_by_key(|s| table.party(*s));
    read
}

/// Members of a cell made individuals, one row each: split out one by one with their profile values drawn, their key and
/// positions the cell's, their shares of its holdings becoming lots. Every member rising takes the cell's own row as
/// the last one, so its identity carries on as an individual. Returns the individuals' rows; the cell re-keys at 10b.
#[clause("REP.29", "REP.2", "REP.14")]
pub fn promote<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut Index,
    slot: Slot,
    members: u32,
    d: &mut [Draws],
) -> Vec<Slot> {
    let weight = ctx.table.weight(slot).get();
    if members == 0 || members > weight || d.len() < usize::try_from(members).unwrap_or(usize::MAX) {
        violation!(clause = "REP.29", "a promotion of no members, more than the cell holds, or unstreamed");
    }
    // Every member rising: the cell's own row becomes the last individual.
    let parts_out = if members == weight { members - 1 } else { members };
    let origin = ctx.table.party(slot);
    let spec =
        SplitSpec { count: 1, given: &[], rows: &[], own: &[], reviewed: Missing::Absent, rounding: Round::HalfEven };
    let mut batch: Vec<(PartId, SplitSpec<'_>, &mut Draws)> =
        d.iter_mut().zip(0..parts_out).map(|(draws, seq)| (PartId { origin, seq }, spec, draws)).collect();
    let mut cells = Cells { ledger: &mut *ctx.ledger, table: &mut *ctx.table, place: ctx.place, keys: &*ctx.keys };
    let parts = split_batch(&mut cells, slot, &mut batch);
    let mut out = Vec::with_capacity(parts.len() + 1);
    for parted in parts {
        let Parted::Part(p) = parted else {
            violation!(clause = "REP.29", "a promoted member took its whole cell");
        };
        let (_, s, _) = place_part(ctx, *p, true);
        out.push(s);
    }
    if members == weight {
        index.remove(ctx.table.hot(slot).landing_key, origin);
        individualize(ctx, slot);
        out.push(slot);
    }
    out
}

/// A cell of one member made an individual in its own row: its holdings become lots at their pooled cost.
fn individualize<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, slot: Slot) {
    let held: Vec<phx_ledger::holding::CellHolding> = phx_ledger::part::cell_holdings(&*ctx.table, slot);
    let pooled: Vec<_> = held
        .iter()
        .map(|h| ctx.ledger.detach_holding(ctx.table, ctx.place, slot, h.instrument, h.count, Round::HalfEven))
        .collect();
    ctx.table.make_individual(slot);
    for h in pooled {
        let _ = ctx.ledger.attach_holding_as_lot(ctx.table, ctx.place, slot, h, ctx.today);
    }
}

/// An individual taken whole as a part to rejoin the cells: its rows and holdings leave it, its lots pooled at their
/// cost, and its row is freed.
fn take_individual<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, slot: Slot) -> Part {
    let t = &mut *ctx.table;
    let rows: Vec<_> = phx_ledger::rows::iter(&*t, slot).map(|r| (r.row.line, r.side(), r.row.count)).collect();
    let mut detached = Vec::with_capacity(rows.len());
    for (line, side, count) in rows {
        let share = phx_ledger::part::RowShare { count, own_balance: 0 };
        detached.push(ctx.ledger.detach_row(t, ctx.place, slot, (line, side), share, Round::HalfEven));
    }
    let holdings = ctx.ledger.detach_lots_pooled(t, ctx.place, slot);
    let part = Part {
        id: PartId { origin: t.party(slot), seq: 0 },
        from: slot,
        weight: t.weight(slot),
        key: ctx.keys.record(t.hot(slot).key_id),
        sig: t.sig(slot),
        positions: (0..t.positions()).map(|i| t.position(slot, i)).collect(),
        rates: (0..t.rate_kinds()).map(|r| t.rate(slot, r)).collect(),
        exposures: (0..t.review_kinds()).map(|j| t.exposure(slot, j)).collect(),
        attention: (0..t.review_kinds()).map(|j| t.attention(slot, j)).collect(),
        profile: t.profile(slot),
        rows: detached,
        holdings,
    };
    t.remove(slot);
    part
}

/// Individuals below the demotion rank rejoin the cells: each is taken whole as a part and lands as any part does; its
/// identity ends, the cell it landed in its successor.
#[clause("REP.29", "REP.8")]
pub fn demote<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, index: &mut Index, slots: &[Slot]) -> Landed {
    let parts: Vec<Part> = slots
        .iter()
        .map(|s| {
            if !ctx.table.hot(*s).is_individual() {
                violation!(clause = "REP.29", "a cell demoted that is not an individual", slot = s.get());
            }
            take_individual(ctx, *s)
        })
        .collect();
    let landed = land(ctx, index, parts);
    for (part, cell) in &landed.resolved {
        ctx.directory.end(part.origin, ctx.today, Missing::Present(*cell));
    }
    landed
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyId, Slot};
    use phx_rand::Draws;

    use super::{Ranks, demote, promote, read_ranks};
    use crate::fixture::{PLAIN, Spec, Ten, draws};
    use crate::landing::LandingIndex;
    use crate::measure::Census;

    fn streams(tag: &str, n: u32) -> Vec<Draws> {
        (0..n).map(|i| draws(tag, i)).collect()
    }

    const NOBODY: fn(PartyId) -> bool = |_| false;

    #[test]
    fn promotion_ties_by_lot() {
        let mut ten = Ten::new();
        let (_, a) = ten.add(1, PLAIN);
        let (_, b) = ten.add(2, Spec { weight: 50, income: 5_000, ..PLAIN });
        let (_, c) = ten.add(3, Spec { weight: 20, income: 30_000, ..PLAIN });
        let ranks = Ranks { measure: 0, promote: 40, demote: 400 };
        let read = read_ranks(&ten.table, ranks, &NOBODY, &mut draws("REP.promotion", 0));
        // C's twenty members at 1 500 each are above the edge; A's hundred and B's fifty tie at 100 each, and the twenty
        // left of the rank are drawn from those hundred and fifty.
        assert!(read.promote.contains(&(c, 20)), "{:?}", read.promote);
        let at_edge: u32 = read.promote.iter().filter(|(s, _)| [a, b].contains(s)).map(|(_, n)| n).sum();
        assert_eq!(at_edge, 20, "{:?}", read.promote);
        let again = read_ranks(&ten.table, ranks, &NOBODY, &mut draws("REP.promotion", 0));
        assert_eq!(read, again, "the lot is the stream's");
        assert!(read.demote.is_empty(), "no individual yet");
    }

    #[test]
    fn promotion_on_declared_action() {
        let mut ten = Ten::new();
        let (_, a) = ten.add(1, PLAIN);
        let (party_b, b) = ten.add(1, Spec { weight: 2, income: 400, ..PLAIN });
        let before = Census::of(&ten.table);
        let mut index = std::mem::take(&mut ten.index);
        let risen = promote(&mut ten.tenb(), &mut index, a, 3, &mut streams("FRM.seek_buyer", 3));
        assert_eq!(risen.len(), 3);
        for s in &risen {
            assert!(ten.table.hot(*s).is_individual() && ten.table.weight(*s).get() == 1);
        }
        assert_eq!(ten.table.weight(a).get(), 97, "the members taking the action, and only they, rise");
        let whole = promote(&mut ten.tenb(), &mut index, b, 2, &mut streams("FRM.seek_buyer", 2));
        assert_eq!(whole.len(), 2);
        assert_eq!(whole.last(), Some(&b), "the cell's own row is the last individual");
        assert_eq!(ten.table.party(b), party_b, "and keeps its identity");
        assert!(!index.candidates(ten.table.hot(b).landing_key).iter().any(|(p, _)| *p == party_b), "never landed");
        let after = Census::of(&ten.table);
        assert_eq!((after.members, after.individuals), (before.members, 5), "no member made or lost");
        let income: i64 = ten.table.slots().map(|s| ten.table.position(s, 0)).sum();
        assert_eq!(income, 10_400, "no total moved");
    }

    #[test]
    fn demotion_hysteresis() {
        let mut ten = Ten::new();
        let _ = ten.add(1, Spec { weight: 100, income: 1_000, ..PLAIN });
        let (_, rich) = ten.add(1, Spec { weight: 3, income: 3_000_000, ..PLAIN });
        let (_, mid) = ten.add(1, Spec { weight: 1, income: 5_000, ..PLAIN });
        let (_, low) = ten.add(1, Spec { weight: 1, income: 10, ..PLAIN });
        let mut index = std::mem::take(&mut ten.index);
        let _ = promote(&mut ten.tenb(), &mut index, rich, 3, &mut streams("REP.promotion", 3));
        let _ = promote(&mut ten.tenb(), &mut index, mid, 1, &mut streams("REP.promotion", 1));
        let _ = promote(&mut ten.tenb(), &mut index, low, 1, &mut streams("REP.promotion", 1));
        // Three rich individuals hold the promotion rank of three. The one at 5 000 is outside it but within the
        // demotion rank of 30, above the cell's hundred at 10 each; the one at 10 ties with them, below that rank's
        // edge only if strictly below it, so it stays until it falls below.
        let ranks = Ranks { measure: 0, promote: 3, demote: 30 };
        let read = read_ranks(&ten.table, ranks, &NOBODY, &mut draws("REP.promotion", 1));
        assert!(read.promote.is_empty(), "the rank is full of individuals already");
        assert!(read.demote.is_empty(), "{:?}", read.demote);
        ten.table.set_position(low, 0, 1);
        let read = read_ranks(&ten.table, ranks, &NOBODY, &mut draws("REP.promotion", 1));
        assert_eq!(read.demote, [low], "below the demotion rank's edge, and only it");
        let low_party = ten.table.party(low);
        let kept = move |p: PartyId| p == low_party;
        let read = read_ranks(&ten.table, ranks, &kept, &mut draws("REP.promotion", 1));
        assert!(read.demote.is_empty(), "an individual with a public instrument stays one");
        let landed = demote(&mut ten.tenb(), &mut index, &[low]);
        assert_eq!(landed.parts, 1);
        assert!(!ten.table.is_live(low) || ten.table.party(low) != low_party, "its row is freed");
        let rows: Vec<Slot> = ten.table.slots().filter(|s| !ten.table.hot(*s).is_individual()).collect();
        assert_eq!(index.entries(), crate::index::Index::rebuild(&ten.table).entries(), "{rows:?}");
    }
}
