use std::collections::BTreeMap;

use phx_core::Directory;
use phx_exec::mix64;
use phx_id::{Day, PartyId, RowRef, Slot};
use phx_ledger::apply::Ledger;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::{AddressSpace, Backing};

use crate::check::{LineKinks, View, check};
use crate::consts::STEPS_PER_WORD;
use crate::join::{Landing, join, join_batch};
use crate::key::{KeyId, KeyInterner, KeyRecord};
use crate::kind::PopKindDecl;
use crate::part::{Part, PartId};
use crate::steps::Step;
use crate::table::{CellTable, NewCell};

/// A row's landing key: its key identity, its step vector at the current levels and its kink signature, folded into
/// one word, lengths first so no vector's end can pass for another's start. The key only proposes where a part may
/// land; the landing check compares the three exactly.
#[clause("REP.8", "REP.4")]
#[must_use]
pub fn landing_key(key: KeyId, steps: &[Step], sig: &[u64]) -> u64 {
    let lengths = [steps.len(), sig.len()].map(phx_rand::float::len_u64);
    let mut h = mix64(u64::from(key.get()));
    for n in lengths {
        h = mix64(h ^ n);
    }
    for four in steps.chunks(STEPS_PER_WORD) {
        let word = four.iter().fold(0_u64, |w, s| (w << u16::BITS) | u64::from(s.get()));
        h = mix64(h ^ word);
    }
    for w in sig {
        h = mix64(h ^ *w);
    }
    h
}

/// The landing index as 10b reads and keeps it: for a landing key, the cells holding it in order of their permanent
/// identities, never their slots, so renumbering cannot move where a part lands.
pub trait LandingIndex {
    fn candidates(&self, landing: u64) -> Vec<(PartyId, Slot)>;
    fn insert(&mut self, landing: u64, party: PartyId, slot: Slot);
    fn remove(&mut self, landing: u64, party: PartyId);
}

/// What 10b works on for one table: the ledger, which alone moves rows and holdings; the table, its place among the
/// holder tables and its kind's keys, steps at their levels and line kinks; the directory, which gives new cells their
/// identities; and the day.
pub struct TenB<'a, B: Backing, L: Backing> {
    pub ledger: &'a mut Ledger<L>,
    pub table: &'a mut CellTable<B>,
    pub place: u16,
    pub keys: &'a mut KeyInterner,
    pub directory: &'a mut Directory,
    pub space: &'a mut AddressSpace,
    pub kind: &'a PopKindDecl,
    pub levels: &'a [u8],
    pub kinks: &'a dyn LineKinks,
    pub today: Day,
}

impl<B: Backing, L: Backing> core::fmt::Debug for TenB<'_, B, L> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TenB").field("place", &self.place).field("today", &self.today).finish_non_exhaustive()
    }
}

/// A group of the parts no cell took: those of one key, step vector and signature, which may form cells together.
type Cluster = (KeyRecord, Vec<Step>, Vec<u64>);

/// What a day's landings did, for the representation's counters and costs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Landed {
    pub parts: u64,
    pub landings: u64,
    pub new_cells: u64,
    pub rows: u64,
    pub holder_list_changes: u64,
    /// Per position, the dispersion the day's joins erased.
    pub erased: Vec<f64>,
    /// Each part's cell, by the part's identity, for re-pointing the records that name it.
    pub resolved: Vec<(PartId, PartyId)>,
    /// Per position, what the day's landings moved of the totals they joined: the audit's, which only nought passes.
    pub moved: Vec<i64>,
}

impl Landed {
    /// The totals of the positions of what joins, taken before as negatives and after as positives.
    pub(crate) fn count(&mut self, totals: &[i64], sign: i64) {
        for (i, t) in totals.iter().enumerate() {
            let by = sign * t;
            match self.moved.get_mut(i) {
                Some(m) => {
                    let Some(next) = m.checked_add(by) else {
                        violation!(clause = "Law 7", "a landing's moved total overflows", position = i);
                    };
                    *m = next;
                }
                None => self.moved.push(by),
            }
        }
    }

    pub(crate) fn joined(&mut self, part: PartId, cell: PartyId, done: &crate::join::Joined) {
        self.joined_all(&[part], cell, done);
    }

    pub(crate) fn joined_all(&mut self, parts: &[PartId], cell: PartyId, done: &crate::join::Joined) {
        self.rows += u64::from(done.rows);
        self.holder_list_changes += u64::from(done.holder_list_changes);
        for (i, e) in done.erased.iter().enumerate() {
            match self.erased.get_mut(i) {
                Some(total) => *total += e,
                None => self.erased.push(*e),
            }
        }
        self.resolved.extend(parts.iter().map(|p| (*p, cell)));
    }
}

/// A key held by one more cell, or one fewer, reduced into the interner at once.
pub(crate) fn hold_key(keys: &mut KeyInterner, key: KeyRecord, by: i64) {
    keys.hold(key, by);
}

/// The part's landing key, when some cell holds its key; a key no cell holds has no candidate.
fn landing_of(view: &View) -> Missing<u64> {
    match view.key {
        Missing::Present(id) => Missing::Present(landing_key(id, &view.steps, &view.sig)),
        Missing::Absent => Missing::Absent,
    }
}

/// A part placed in a row of its own: its identity from the directory, its key held once more, its totals, profile,
/// signature, rates, exposures, attention, rows and holdings the part's. An individual's row is made one before its
/// holdings arrive, so they arrive as lots at their pooled cost; a cell's keep them pooled.
pub(crate) fn place_part<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    part: Part,
    individual: bool,
) -> (PartyId, Slot, crate::join::Joined) {
    hold_key(ctx.keys, part.key, 1);
    let Missing::Present(key) = ctx.keys.id(&part.key) else {
        violation!(clause = "REP.19", "a key held and not interned");
    };
    let party = PartyId::new(ctx.directory.next());
    let new = NewCell {
        party,
        created: ctx.today,
        weight: part.weight,
        key,
        positions: &part.positions,
        profile: &part.profile,
    };
    let slot = ctx.table.add(ctx.space, new, ctx.kind, ctx.levels);
    if ctx.directory.begin(RowRef { table: ctx.table.id(), slot }) != party {
        violation!(clause = "PTY.9", "a new row given another identity than the directory's next");
    }
    if individual {
        ctx.table.make_individual(slot);
    }
    ctx.table.set_sig(slot, &part.sig);
    for (r, rate) in part.rates.iter().enumerate() {
        ctx.table.set_rate(slot, r, *rate);
    }
    for (j, e) in part.exposures.iter().enumerate() {
        if let Missing::Present(x) = e {
            ctx.table.set_exposure(slot, j, *x);
        }
    }
    for (j, a) in part.attention.iter().enumerate() {
        if let Missing::Present(x) = a {
            ctx.table.set_attention(slot, j, *x);
        }
    }
    let rows = u32::try_from(part.rows.len()).unwrap_or_else(|_| capacity_exceeded!("rows of a part", u32::MAX, 0));
    let entered = ctx.ledger.attach_rows(ctx.table, ctx.place, slot, part.rows);
    let mut done = crate::join::Joined { rows, holder_list_changes: entered, erased: Vec::new() };
    for holding in part.holdings {
        let listed = if individual {
            ctx.ledger.attach_holding_as_lot(ctx.table, ctx.place, slot, holding, ctx.today)
        } else {
            ctx.ledger.attach_holding(ctx.table, ctx.place, slot, holding)
        };
        if listed {
            done.holder_list_changes += 1;
        }
    }
    ctx.table.rekey(slot, ctx.kind, ctx.levels);
    (party, slot, done)
}

/// A new cell made of a part, keyed at once and entered in the index.
fn new_cell<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut dyn LandingIndex,
    part: Part,
    landed: &mut Landed,
) -> Slot {
    let id = part.id;
    let (party, slot, done) = place_part(ctx, part, false);
    index.insert(ctx.table.hot(slot).landing_key, party, slot);
    landed.new_cells += 1;
    landed.joined(id, party, &done);
    slot
}

/// 10b: the day's parts land. They are sorted by landing key, then by identity, so no tie falls to input order; each
/// looks its key up in the index and takes the first candidate, in order of permanent identity, that passes the check
/// against its state at 10b's start; then every part bound for a target joins it, target by target. Parts without a
/// target are taken in order of identity: each joins the first new cell of its key, steps and signature it passes the
/// check with, or starts one. The check draws nothing, so the landings follow from the parts alone.
#[clause("REP.8", "REP.14", "REP.16")]
pub fn land<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut dyn LandingIndex,
    parts: Vec<Part>,
) -> Landed {
    let mut landed = Landed { parts: phx_rand::float::len_u64(parts.len()), ..Landed::default() };
    let views: Vec<View> = parts.iter().map(|p| View::of_part(p, ctx.keys, ctx.kind, ctx.levels)).collect();
    let mut order: Vec<(Missing<u64>, PartId, usize)> =
        parts.iter().zip(&views).enumerate().map(|(i, (p, v))| (landing_of(v), p.id, i)).collect();
    order.sort_unstable_by_key(|(lk, id, _)| (*lk == Missing::Absent, lk_value(*lk), *id));
    let mut cells: BTreeMap<Slot, View> = BTreeMap::new();
    let mut targeted: Vec<(PartyId, PartId, usize, Slot)> = Vec::new();
    let mut unbound: Vec<(PartId, usize)> = Vec::new();
    for (lk, id, i) in &order {
        let Some(view) = views.get(*i) else { continue };
        let found = match lk {
            Missing::Present(k) => index.candidates(*k).into_iter().find(|(_, slot)| {
                let cell = cells
                    .entry(*slot)
                    .or_insert_with(|| View::of_cell(ctx.table, *slot, ctx.ledger, ctx.kind, ctx.levels));
                check(view, cell, ctx.kinks).is_ok()
            }),
            Missing::Absent => None,
        };
        match found {
            Some((cell, slot)) => targeted.push((cell, *id, *i, slot)),
            None => unbound.push((*id, *i)),
        }
    }
    for p in &parts {
        landed.count(&p.positions, -1);
    }
    let mut touched: Vec<Slot> = targeted.iter().map(|(_, _, _, slot)| *slot).collect();
    touched.sort_unstable();
    touched.dedup();
    for slot in &touched {
        landed.count(&totals(ctx.table, *slot), -1);
    }
    let mut parts: Vec<Option<Part>> = parts.into_iter().map(Some).collect();
    targeted.sort_unstable_by_key(|(cell, id, _, _)| (*cell, *id));
    for bound in targeted.chunk_by(|a, b| a.0 == b.0) {
        let Some((cell, _, _, slot)) = bound.first().copied() else { continue };
        let batch: Vec<Part> =
            bound.iter().filter_map(|(_, _, i, _)| parts.get_mut(*i).and_then(Option::take)).collect();
        let ids: Vec<PartId> = batch.iter().map(|p| p.id).collect();
        let done = join_batch(ctx.ledger, ctx.table, ctx.place, slot, batch);
        landed.landings += phx_rand::float::len_u64(ids.len());
        landed.joined_all(&ids, cell, &done);
    }
    unbound.sort_unstable_by_key(|(id, _)| *id);
    let mut clusters: BTreeMap<Cluster, Vec<(PartyId, Slot)>> = BTreeMap::new();
    for (id, i) in unbound {
        let Some(part) = parts.get_mut(i).and_then(Option::take) else { continue };
        let view = View::of_part(&part, ctx.keys, ctx.kind, ctx.levels);
        let group = (part.key, view.steps.clone(), view.sig.clone());
        let found = clusters.get(&group).and_then(|made| {
            made.iter().copied().find(|(_, slot)| {
                check(&view, &View::of_cell(ctx.table, *slot, ctx.ledger, ctx.kind, ctx.levels), ctx.kinks).is_ok()
            })
        });
        if let Some((cell, slot)) = found {
            let done = join(ctx.ledger, ctx.table, ctx.place, &Landing { part: id, target: slot }, part);
            landed.landings += 1;
            landed.joined(id, cell, &done);
        } else {
            let slot = new_cell(ctx, index, part, &mut landed);
            clusters.entry(group).or_default().push((ctx.table.party(slot), slot));
            touched.push(slot);
        }
    }
    for slot in touched {
        landed.count(&totals(ctx.table, slot), 1);
    }
    landed
}

/// Members the opening draws, which leave no cell: their key, their weight and their profile, with no position,
/// rate, exposure, attention, row or holding yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drawn {
    pub key: KeyRecord,
    pub weight: phx_core::Weight,
    pub profile: crate::profile::Profile,
    /// The rows its households and their persons hold, which the opening opens on the cell they land in.
    pub rows: Vec<crate::explicit::GatheredRow>,
}

/// The opening's members landed as a day's parts land. Having left no cell, their parts are ordered by their place
/// in the list, under the first identity this landing hands out.
#[clause("GEN.3", "REP.8", "REP.14")]
pub fn land_drawn<B: Backing, L: Backing>(
    ctx: &mut TenB<'_, B, L>,
    index: &mut dyn LandingIndex,
    drawn: Vec<Drawn>,
) -> Landed {
    let origin = PartyId::new(ctx.directory.next());
    let (kind, sig) = (ctx.kind, ctx.kind.sig.words());
    let parts = drawn
        .into_iter()
        .zip(0_u32..)
        .map(|(d, seq)| Part {
            id: PartId { origin, seq },
            from: Slot::new(0),
            weight: d.weight,
            key: d.key,
            sig: vec![0; sig],
            positions: vec![0; kind.positions.len()],
            rates: vec![Missing::Absent; kind.rates.len()],
            exposures: vec![Missing::Absent; kind.reviews.len()],
            attention: vec![Missing::Absent; kind.reviews.len()],
            profile: d.profile,
            rows: Vec::new(),
            holdings: Vec::new(),
        })
        .collect();
    land(ctx, index, parts)
}

/// A cell's position totals, in the kind's order.
pub(crate) fn totals<B: Backing>(table: &CellTable<B>, slot: Slot) -> Vec<i64> {
    (0..table.positions()).map(|i| table.position(slot, i)).collect()
}

fn lk_value(lk: Missing<u64>) -> u64 {
    match lk {
        Missing::Present(k) => k,
        Missing::Absent => 0,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::landing_key;
    use crate::key::KeyId;
    use crate::steps::{Step, StepTable};

    fn step(n: u16) -> Step {
        let t = StepTable::new(&phx_core::register::values::Partition { exp: 0, bounds: (1..=40).collect() }).unwrap();
        t.step_of(i64::from(n))
    }

    #[test]
    fn landing_key_equal_iff_key_steps_and_signature_equal() {
        let steps = [step(3), step(0), Step::MISSING, step(7), step(9)];
        let sig = [0b1011_u64];
        let k = landing_key(KeyId::new(12), &steps, &sig);
        assert_eq!(k, landing_key(KeyId::new(12), &steps, &sig), "equal parts, equal keys");
        assert_ne!(k, landing_key(KeyId::new(13), &steps, &sig), "another key");
        let mut other = steps;
        other[2] = step(0);
        assert_ne!(k, landing_key(KeyId::new(12), &other, &sig), "a missing step is not step nought");
        assert_ne!(k, landing_key(KeyId::new(12), &steps, &[0b1010]), "another band");
        assert_ne!(k, landing_key(KeyId::new(12), &steps[..4], &sig), "a shorter vector");
        assert_ne!(landing_key(KeyId::new(1), &[step(1)], &[]), landing_key(KeyId::new(1), &[], &[1]));
        let mut d = Draws::new(stream_key(Seed::new(5), "landing"), Subject::new(SubjectTag::World, 0), 0, 0);
        let mut seen = BTreeSet::new();
        let mut parts = BTreeSet::new();
        for _ in 0..20_000 {
            let key = KeyId::new(u32::try_from(below_u64(&mut d, 50)).unwrap());
            let steps: Vec<Step> = (0..6).map(|_| step(u16::try_from(below_u64(&mut d, 6)).unwrap())).collect();
            let sig = [below_u64(&mut d, 4)];
            if parts.insert((key, steps.clone(), sig)) {
                assert!(seen.insert(landing_key(key, &steps, &sig)), "two different parts, one key");
            }
        }
    }
}
