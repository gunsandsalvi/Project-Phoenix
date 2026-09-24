use std::collections::BTreeMap;
use std::ops::Range;

use phx_core::agenda::Agenda;
use phx_id::{PartyId, RowRef, Slot};
use phx_macros::clause;
use phx_store::Backing;

use crate::index::{Index, IndexChange};
use crate::key::KeyRecord;
use crate::landing::TenB;

/// The swaps that put one range of chunks' live rows in the order that restores locality — by key, whose declared
/// attributes lead with country, region and zone, then by identity — each row going to a slot the range's live rows
/// already hold, so no slot is freed or taken. Applied in turn, they leave every row where the order puts it.
#[clause("REP.1")]
#[must_use]
pub fn plan<B: Backing, L: Backing>(ctx: &TenB<'_, B, L>, chunks: Range<usize>) -> Vec<(Slot, Slot)> {
    let in_range = |s: &Slot| chunks.contains(&ctx.table.chunk_of(*s));
    let live: Vec<Slot> = ctx.table.slots().filter(in_range).collect();
    let mut wanted: Vec<(KeyRecord, PartyId)> =
        live.iter().map(|s| (ctx.keys.record(ctx.table.hot(*s).key_id), ctx.table.party(*s))).collect();
    wanted.sort_unstable();
    // Where each identity lies now, and which identity each slot holds, kept as the swaps are made.
    let mut at: BTreeMap<PartyId, Slot> = live.iter().map(|s| (ctx.table.party(*s), *s)).collect();
    let mut holds: BTreeMap<Slot, PartyId> = live.iter().map(|s| (*s, ctx.table.party(*s))).collect();
    let mut swaps = Vec::new();
    for (slot, (_, party)) in live.iter().zip(&wanted) {
        let Some(from) = at.get(party).copied() else { continue };
        if from == *slot {
            continue;
        }
        let Some(displaced) = holds.get(slot).copied() else { continue };
        swaps.push((*slot, from));
        at.insert(*party, *slot);
        at.insert(displaced, from);
        holds.insert(*slot, *party);
        holds.insert(from, displaced);
    }
    swaps
}

/// The swaps applied with every reference remapped for the rows they move alone: the rows and their arena lists, the
/// ledger's holder lists, the landing index, the directory and the agenda, where the kind's rows are booked in one. No
/// identity changes.
#[clause("REP.1", "PTY.10")]
pub fn apply<B: Backing, L: Backing, A: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut Index,
    mut agenda: Option<&mut Agenda<A>>,
    swaps: &[(Slot, Slot)],
) {
    let id = ctx.table.id();
    for (a, b) in swaps.iter().copied() {
        ctx.table.swap_rows(a, b);
        ctx.ledger.swap_holders(&*ctx.table, ctx.place, a, b);
        if let Some(agenda) = agenda.as_deref_mut() {
            agenda.swap_rows(id, a, b);
        }
        let mut moves = Vec::with_capacity(2);
        for s in [a, b] {
            let party = ctx.table.party(s);
            ctx.directory.relocate(party, RowRef { table: id, slot: s });
            let hot = ctx.table.hot(s);
            if !hot.is_individual() {
                moves.push((hot.landing_key, party, IndexChange::Move(s)));
            }
        }
        index.apply(moves);
    }
}

#[cfg(test)]
mod tests {
    use phx_core::agenda::{Agenda, AgendaTableSpec};
    use phx_core::directory::PartyState;
    use phx_id::{Day, PartyId};
    use phx_ledger::algebra::Side;
    use phx_ledger::rows::rows;
    use phx_num::Missing;
    use phx_store::{AddressSpace, HeapBacking};

    use super::{apply, plan};
    use crate::fixture::{PLAIN, Spec, Ten, draws};
    use crate::index::Index;
    use crate::promote::promote;

    /// A row's identity, weight, income, and each of its rows' line, members and balance.
    type RowView = (PartyId, u32, i64, Vec<(u32, u32, Missing<i64>)>);

    /// Every row as renumbering must leave it, in order of identity.
    fn identity_view(ten: &Ten) -> Vec<RowView> {
        let mut v: Vec<_> = ten
            .table
            .slots()
            .map(|s| {
                let r =
                    rows(&ten.table, s).iter().map(|x| (x.row.line.get(), x.row.count, x.optional.balance)).collect();
                (ten.table.party(s), ten.table.weight(s).get(), ten.table.position(s, 0), r)
            })
            .collect();
        v.sort_unstable_by_key(|x| x.0);
        v
    }

    #[test]
    fn renumber_remaps_every_reference() {
        let mut ten = Ten::new();
        let mut specs = Vec::new();
        // Forty rows over three chunks of sixteen, so rows and their lists move between arenas.
        for i in 0..40_u32 {
            let composition = (i * 7) % 4;
            let loan = (i % 3 == 0).then_some((5 + i, 1_000 * i64::from(i) + 7));
            specs.push((composition, Spec { weight: 10 + i, income: 1_000 + i64::from(i) * 13, loan, ..PLAIN }));
        }
        let mut slots = Vec::new();
        for (c, spec) in &specs {
            slots.push(ten.add(*c, *spec).1);
        }
        // Every member of the cell to be promoted from holds the fund, so the one rising takes a lot of it.
        // The cell of the lowest key, so the individual made last, at the end of the table, sorts into the first chunk.
        let (fund, holder) = (ten.books.fund, slots[0]);
        let weight = ten.table.weight(holder).get();
        let holding = phx_ledger::holding::CellHolding {
            instrument: fund,
            count: weight,
            quantity: phx_num::QtyRaw::from_raw(i64::from(weight) * 40),
            pooled_cost: phx_num::Amount::from_raw(i64::from(weight) * 400),
        };
        let _ = ten.books.ledger.attach_holding(&mut ten.table, 0, holder, holding);
        let mut index = std::mem::take(&mut ten.index);
        let risen = promote(&mut ten.tenb(), &mut index, holder, 1, &mut [draws("REP.promotion", 0)]);
        assert!(ten.table.chunk_of(*slots.last().unwrap()) >= 2);
        let spec = [AgendaTableSpec { table: ten.table.id(), max_rows: 64, reasons: 1 }];
        let mut agenda: Agenda<HeapBacking<4096>> =
            Agenda::new(&mut AddressSpace::empty(), Day::new(0), &spec, 1 << 12).unwrap();
        agenda.grow(ten.table.id(), 64);
        for (n, s) in ten.table.slots().collect::<Vec<_>>().into_iter().enumerate() {
            agenda.set_next(ten.table.id(), s, 0, Day::new(10 + u32::try_from(n).unwrap()));
        }
        let due_of = |ten: &Ten, a: &Agenda<HeapBacking<4096>>| {
            let mut v: Vec<(PartyId, Option<Day>)> =
                ten.table.slots().map(|s| (ten.table.party(s), a.next(ten.table.id(), s, 0))).collect();
            v.sort_unstable();
            v
        };
        let (before, due_before) = (identity_view(&ten), due_of(&ten, &agenda));
        let chunk_before = ten.table.chunk_of(risen[0]);
        let swaps = plan(&ten.tenb(), 0..usize::MAX);
        assert!(!swaps.is_empty(), "the rows were added out of key order");
        apply(&mut ten.tenb(), &mut index, Some(&mut agenda), &swaps);
        assert_eq!(identity_view(&ten), before, "no identity, total or row changed");
        assert_eq!(due_of(&ten, &agenda), due_before, "every due moved with its row");
        assert!(plan(&ten.tenb(), 0..usize::MAX).is_empty(), "the order holds once restored");
        let order: Vec<_> =
            ten.table.slots().map(|s| (ten.keys.record(ten.table.hot(s).key_id), ten.table.party(s))).collect();
        assert!(order.windows(2).all(|w| w[0] <= w[1]), "rows by key, then identity");
        for s in ten.table.slots() {
            let party = ten.table.party(s);
            assert!(
                matches!(ten.directory.lookup(party), PartyState::Live(r) if r.slot == s),
                "the directory points at it"
            );
        }
        assert_eq!(index.entries(), Index::rebuild(&ten.table).entries(), "the index points at every cell's slot");
        let loan = ten.books.loan;
        let mut listed: Vec<u32> = ten.books.ledger.lines.holders(loan).collect();
        listed.sort_unstable();
        let mut holding: Vec<u32> = ten
            .table
            .slots()
            .filter(|s| rows(&ten.table, *s).iter().any(|r| r.row.line == loan && r.side() == Side::Liability))
            .map(|s| ten.books.ledger.lines.keys().key(0, s))
            .collect();
        holding.sort_unstable();
        assert_eq!(listed, holding, "the loan's holder list names the slots holding it");
        let individual = ten.table.slots().find(|s| ten.table.hot(*s).is_individual()).unwrap();
        assert_eq!(ten.table.ext_owner(individual), individual, "the individual's extension follows its row");
        assert_ne!(ten.table.chunk_of(individual), chunk_before, "it changed chunk, so its lot changed arena");
        let lots = phx_ledger::holding::lots(&ten.table, individual, fund);
        assert_eq!(lots.len(), 1, "its lot moved with it");
        assert_eq!((lots[0].quantity.raw(), lots[0].cost.raw()), (40, 400), "at the pooled cost it took");
        assert_eq!(risen.len(), 1);
    }
}
