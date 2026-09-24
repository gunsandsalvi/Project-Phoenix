use phx_id::{Day, Slot};
use phx_ledger::part::{RowShare, cell_holdings};
use phx_ledger::rows;
use phx_macros::clause;
use phx_num::Missing;
use phx_num::round::Round;
use phx_store::Backing;

use crate::check::{View, check};
use crate::join::{Landing, join};
use crate::key::{KeyLayout, KeyRecord};
use crate::landing::{Landed, LandingIndex, TenB, hold_key};
use crate::part::{Part, PartId};

/// A cell's key changed for all its members alike: the new key held once more and the old once fewer, the cell keeping
/// its identity. It re-keys at 10b.
#[clause("REP.19")]
pub fn set_key<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, slot: Slot, key: KeyRecord) {
    let old = ctx.keys.record(ctx.table.hot(slot).key_id);
    if old == key {
        return;
    }
    hold_key(ctx.keys, key, 1);
    let Missing::Present(id) = ctx.keys.id(&key) else {
        phx_num::violation!(clause = "REP.19", "a key held and not interned");
    };
    ctx.table.set_key(slot, id);
    hold_key(ctx.keys, old, -1);
}

/// A whole cell taken as a part, to land in another: every row, holding and total leaves it, and its row is freed.
fn take_whole<B: Backing, L: Backing>(ctx: &mut TenB<'_, B, L>, slot: Slot) -> Part {
    let t = &mut *ctx.table;
    let rows: Vec<_> = rows::iter(&*t, slot).map(|r| (r.row.line, r.side(), r.row.count)).collect();
    let mut detached = Vec::with_capacity(rows.len());
    for (line, side, count) in rows {
        // Every member leaving takes each word whole, so no rounding is read.
        let share = RowShare { count, own_balance: 0 };
        detached.push(ctx.ledger.detach_row(t, ctx.place, slot, (line, side), share, Round::HalfEven));
    }
    let mut holdings = Vec::new();
    for h in cell_holdings(&*t, slot) {
        holdings.push(ctx.ledger.detach_holding(t, ctx.place, slot, h.instrument, h.count, Round::HalfEven));
    }
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

/// 10b's re-keying: rows whose steps an apply changed, whose key changed, or whose members all left as one part, in
/// order of identity, each keyed afresh and moved in the index. A cell that now shares its landing key with another it
/// passes the check with lands in it whole; its identity ends, the other its successor, and its key is held once fewer.
#[clause("REP.8", "REP.28", "REP.19")]
pub fn rekey_flagged<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut dyn LandingIndex,
    flagged: &[Slot],
    landed: &mut Landed,
) -> u64 {
    let mut order: Vec<(phx_id::PartyId, Slot)> = flagged.iter().map(|s| (ctx.table.party(*s), *s)).collect();
    order.sort_unstable();
    order.dedup();
    let mut rekeys = 0;
    for (party, slot) in order {
        let old = ctx.table.hot(slot).landing_key;
        ctx.table.rekey(slot, ctx.kind, ctx.levels);
        let new = ctx.table.hot(slot).landing_key;
        rekeys += 1;
        if old != new {
            index.remove(old, party);
            index.insert(new, party, slot);
        }
        let me = View::of_cell(ctx.table, slot, ctx.ledger, ctx.kind, ctx.levels);
        let target = index.candidates(new).into_iter().filter(|(p, _)| *p != party).find(|(_, s)| {
            check(&me, &View::of_cell(ctx.table, *s, ctx.ledger, ctx.kind, ctx.levels), ctx.kinks).is_ok()
        });
        if let Some((to, at)) = target {
            index.remove(new, party);
            let key = ctx.keys.record(ctx.table.hot(slot).key_id);
            let part = take_whole(ctx, slot);
            let id = part.id;
            let done = join(ctx.ledger, ctx.table, ctx.place, &Landing { part: id, target: at }, part);
            ctx.directory.end(party, ctx.today, Missing::Present(to));
            hold_key(ctx.keys, key, -1);
            landed.landings += 1;
            landed.joined(id, to, &done);
        }
    }
    rekeys
}

/// A key attribute that is a clock: a count to an end the declaring system names, set on its end day to its declared
/// end value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyClock {
    pub attr: usize,
    pub end_value: u32,
}

/// The day a row's one key-clock reason is booked for: the earliest end over its clocks, none without clocks. Every
/// clock of a table shares the reason, so a later system's clock adds none.
#[clause("REP.19")]
pub fn clock_day(ends: &[Day]) -> Missing<Day> {
    ends.iter().fold(Missing::Absent, |earliest, d| match earliest {
        Missing::Present(e) if e <= *d => Missing::Present(e),
        _ => Missing::Present(*d),
    })
}

/// A row's key on a day its clocks may end: each clock whose end has come set to its end value, the others as they
/// were.
#[clause("REP.19")]
#[must_use]
pub fn clocks_ended(layout: &KeyLayout, key: KeyRecord, clocks: &[(KeyClock, Day)], today: Day) -> KeyRecord {
    let mut out = key;
    for (clock, end) in clocks {
        if *end <= today {
            layout.set(&mut out, clock.attr, clock.end_value);
        }
    }
    out
}
