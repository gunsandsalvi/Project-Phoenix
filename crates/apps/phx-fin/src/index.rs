//! `-F index`: every keyed index instance of the inventory at its members at the design point; each day's members
//! moving between keys, every key walked and each list past its dead share compacted at the close; and a flood day's
//! owner draws from the struck zones' owners.

use std::collections::BTreeMap;

use phx_num::Missing;
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, DayBuf, Index, IndexDecl, Keys, Mode, StoreStats};

use crate::design::{Design, IndexInstance};
use crate::fill::Streams;
use crate::kept::{day_of, index, slots, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the indexes are measured under.
pub const BASE: &str = "index";

/// The base the owners' draws are the figure of.
pub const UNITS: &str = "units";

/// The instance whose struck zones' owners a flood day draws.
const OWNERS: &str = "owners_by_zone";

/// The ledger line of the small-bases pool, whose instances this base's MB is.
const POOL_LINE: u64 = 16;

/// A weighted member's weight is below this many steps; an owner's units, below this many.
const WEIGHT_STEPS: u64 = 1 << 20;
const OWNER_UNITS: u64 = 1 << 6;

/// A decimal megabyte.
const MB: f64 = 1_000_000.0;

/// An instance filled: its index, each member's key now (a weighted member's with its position), its ledger line, and
/// its dead share after the day's moves.
#[derive(Debug)]
struct Filled1 {
    name: String,
    ix: Index,
    keys: Vec<(u64, usize)>,
    line: u64,
    dead_share: f64,
}

/// Every instance, the walks' buffer, and the streams the days draw from.
#[derive(Debug, Default)]
pub struct Indexes {
    all: Vec<Filled1>,
    out: Option<DayBuf<u32>>,
    streams: Option<Streams>,
    churn: u64,
    struck: u64,
    draws: u64,
    drawn: u64,
}

/// A count as a float, through its text.
fn real(n: u64) -> Result<f64, FinError> {
    n.to_string().parse().map_err(|e| FinError(format!("{n}: {e}")))
}

fn mode_of(i: &IndexInstance) -> Result<Mode, FinError> {
    match i.mode.as_str() {
        "lazy" => Ok(Mode::Lazy),
        "counted" => Ok(Mode::LazyCounted),
        "weighted" => Ok(Mode::Weighted),
        "count" => Ok(Mode::Count),
        other => Err(FinError(format!("`{}` has no mode `{other}`", i.name))),
    }
}

impl Indexes {
    /// The owners' draws a flood day made that found an owner, the same for any workers.
    #[must_use]
    pub fn drawn(&self) -> u64 {
        self.drawn
    }
}

impl FinBase for Indexes {
    fn name(&self) -> &'static str {
        BASE
    }

    /// Each instance's members at drawn keys, inserted in slot order as a load's rebuild inserts them.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let mut space = AddressSpace::empty();
        let mut rows = 0;
        for (n, inst) in (0_u64..).zip(design.index_instances()?) {
            let mode = mode_of(&inst)?;
            // Room for the members, a day's moves and a list's dead before its compaction.
            let entries = slots(inst.members * 2 + 16)?;
            let keys = if inst.sparse { Keys::Sparse(slots(inst.keys)?) } else { Keys::Dense(slots(inst.keys)?) };
            let name: &'static str = Box::leak(inst.name.clone().into_boxed_str());
            let decl = IndexDecl {
                name,
                member_table: name,
                key: "key",
                predicate: Missing::Absent,
                mode,
                keys,
                entries,
                clause: "SET.12",
            };
            let mut ix = Index::new(&mut space, decl).map_err(FinError)?;
            let mut d = streams.draws(BASE, n, 0);
            let mut drawn: Vec<u64> = (0..inst.members).map(|_| below_u64(&mut d, inst.keys)).collect();
            // The opening places a dense instance's members key by key — a zone's parties, a tile's units — so their
            // slots follow their keys; a sparse instance's members come to their keys as events bring them.
            if !inst.sparse {
                drawn.sort_unstable();
            }
            let mut weights = streams.draws(BASE, n, 1);
            let mut keys_of = Vec::with_capacity(index(inst.members)?);
            for (member, key) in (0..inst.members).zip(drawn) {
                let place = match mode {
                    Mode::Weighted => ix.insert_weighted(key, 1 + below_u64(&mut weights, WEIGHT_STEPS)),
                    Mode::Count => {
                        ix.add(key, 1);
                        0
                    }
                    Mode::Lazy | Mode::LazyCounted if inst.sparse => {
                        ix.insert(key, slots(member)?);
                        0
                    }
                    Mode::Lazy | Mode::LazyCounted => 0,
                };
                keys_of.push((key, place));
            }
            // A dense instance's members lie key by key in their slots: each key's run placed as the opening placed it.
            if mode.lists() && !inst.sparse {
                let mut first = 0;
                while first < keys_of.len() {
                    let key = keys_of.get(first).map_or(0, |k| k.0);
                    let len = keys_of.get(first..).map_or(0, |rest| rest.iter().take_while(|k| k.0 == key).count());
                    ix.place_run(key, slots(wide(first))?, slots(wide(len))?);
                    first += len;
                }
            }
            ix.settle();
            rows += inst.members;
            self.all.push(Filled1 { name: inst.name, ix, keys: keys_of, line: inst.line, dead_share: 0.0 });
        }
        let most = self.all.iter().map(|f| f.keys.len()).fold(0, |a, n| if n > a { n } else { a });
        self.out = Some(DayBuf::new(&mut space, "index.walk", most));
        (self.streams, self.churn) = (Some(*streams), design.index_count("churn")?);
        (self.struck, self.draws) = (design.index_count("struck_zones")?, design.index_count("owner_draws")?);
        if self.struck == 0 {
            return Err(FinError("a flood that strikes no zone".to_owned()));
        }
        Ok(Filled { rows })
    }

    /// The day's moves, each member leaving its key for a drawn one; then every list walked, and those past their dead
    /// share compacted at the close; on a heavy day, the flood's owner draws.
    fn day(&mut self, day: DayType, _counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let moves = self.moves(day)?;
        self.apply(&moves, m);
        let walked = self.walk(m)?;
        self.close(&walked)?;
        if day == DayType::H {
            self.flood(m, day)?;
        }
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.all.iter().map(|f| f.ix.bytes()).sum(), resident: 0 }
    }

    /// The lists' bytes an entry, the worst instance's dead share after a day's moves, and the small-bases pool's
    /// instances' MB.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let lists = self.all.iter().filter(|f| f.ix.decl().mode.lists());
        let (bytes, live) = lists.fold((0, 0), |(b, l), f| (b + f.ix.bytes(), l + f.ix.rows_live()));
        let pool: u64 = self.all.iter().filter(|f| f.line == POOL_LINE).map(|f| f.ix.bytes()).sum();
        let worst = self.all.iter().map(|f| f.dead_share).fold(0.0, |a: f64, s| if s > a { s } else { a });
        let per_entry = bytes.checked_div(live).and_then(|b| real(b).ok());
        [("entry_bytes", per_entry), ("dead_share", Some(worst)), ("mb", real(pool).ok().map(|b| b / MB))]
            .into_iter()
            .filter_map(|(k, v)| Some((k, v?)))
            .collect()
    }
}

impl Indexes {
    /// A flood day: owners at the struck zones walked once each, their units' cumulative built once from the walk
    /// and shared by the zone's draws, each draw a search of it.
    fn flood(&mut self, m: &mut Measures<'_>, day: DayType) -> Result<(), FinError> {
        let (Some(streams), Some(out)) = (self.streams, self.out.as_mut()) else {
            return Err(FinError("the flood before the fill".to_owned()));
        };
        let Some(owners) = self.all.iter().find(|f| f.name == OWNERS) else {
            return Err(FinError(format!("the inventory has no `{OWNERS}`")));
        };
        let Keys::Dense(zones) = owners.ix.decl().keys else {
            return Err(FinError(format!("`{OWNERS}` is not keyed by zone")));
        };
        let mut d = streams.draws(UNITS, 0, day_of(day)?);
        let struck: Vec<u64> = (0..self.struck).map(|_| below_u64(&mut d, u64::from(zones))).collect();
        let per_zone = self.draws.div_ceil(self.struck);
        let xs: Vec<u64> = (0..per_zone * wide(struck.len())).map(|_| below_u64(&mut d, u64::MAX >> 1)).collect();
        let mut cumulative: Vec<u64> = Vec::new();
        self.drawn = m.read(UNITS, "draw", wide(xs.len()), || {
            let mut drawn = 0_u64;
            for (z, zone) in struck.iter().enumerate() {
                out.clear();
                let valid = |member: u32| {
                    usize::try_from(member).ok().and_then(|i| owners.keys.get(i)).is_some_and(|at| at.0 == *zone)
                };
                let _ = owners.ix.members(*zone, valid, out);
                cumulative.clear();
                let mut sum = 0;
                for owner in out.as_slice() {
                    sum += 1 + u64::from(*owner) % OWNER_UNITS;
                    cumulative.push(sum);
                }
                if sum == 0 {
                    continue;
                }
                for x in
                    xs.iter().skip(z * index(per_zone).unwrap_or_default()).take(index(per_zone).unwrap_or_default())
                {
                    let at = cumulative.partition_point(|c| *c <= x % sum);
                    drawn += u64::from(at < cumulative.len());
                }
            }
            drawn
        });
        Ok(())
    }
}

/// Whether an instance keeps its entries as sparse pairs.
fn sparse(f: &Filled1) -> bool {
    matches!(f.ix.decl().keys, Keys::Sparse(_))
}

/// Whether a member's own key column puts it at a key now.
fn at_key(keys: &[(u64, usize)], member: u32, key: u64) -> bool {
    usize::try_from(member).ok().and_then(|i| keys.get(i)).is_some_and(|k| k.0 == key)
}

/// A day's moves of one instance: each moving member and its new key, sorted by the new key, and the old keys, sorted.
type Moves = (Vec<(u32, u64)>, Vec<u64>);

impl Indexes {
    /// Each instance's share of the day's moves, drawn member by member before any is made; the applies are
    /// partitioned by key, so the leavers go by their old keys, then the comers by their new.
    fn moves(&self, day: DayType) -> Result<Vec<Moves>, FinError> {
        let Some(streams) = self.streams else {
            return Err(FinError("the indexes measured before their fill".to_owned()));
        };
        let listed = |f: &Filled1| f.ix.decl().mode.lists();
        let members: u64 = self.all.iter().filter(|f| listed(f)).map(|f| wide(f.keys.len())).sum();
        let mut d = streams.draws(BASE, u64::MAX >> 8, day_of(day)?);
        let mut all = Vec::with_capacity(self.all.len());
        for f in &self.all {
            let n = if listed(f) && members > 0 { self.churn * wide(f.keys.len()) / members / 2 } else { 0 };
            let space = match f.ix.decl().keys {
                Keys::Dense(k) => u64::from(k),
                Keys::Sparse(_) => f.keys.iter().fold(1, |a, k| if k.0 + 1 > a { k.0 + 1 } else { a }),
            };
            let len = wide(f.keys.len());
            let mut own = Vec::new();
            for _ in 0..n {
                own.push((slots(below_u64(&mut d, len))?, below_u64(&mut d, space)));
            }
            let old_key = |m: &u32| usize::try_from(*m).ok().and_then(|i| f.keys.get(i)).map(|k| k.0);
            let mut old: Vec<u64> = own.iter().filter_map(|(m, _)| old_key(m)).collect();
            old.sort_unstable();
            own.sort_unstable_by_key(|(m, to)| (*to, *m));
            all.push((own, old));
        }
        Ok(all)
    }

    /// The moves applied, dense keys' lists and sparse pairs timed apart, their costs of different kinds; the members'
    /// own key columns are the moving events' to write, outside the index's time.
    fn apply(&mut self, moves: &[Moves], m: &mut Measures<'_>) {
        let all = &mut self.all;
        for (op, of_pairs) in [("insert", false), ("insert_pairs", true)] {
            let items: u64 =
                all.iter().zip(moves).filter(|(f, _)| sparse(f) == of_pairs).map(|(_, o)| wide(o.0.len()) * 2).sum();
            m.read(BASE, op, items, || {
                for (f, (own, old)) in all.iter_mut().zip(moves).filter(|(f, _)| sparse(f) == of_pairs) {
                    for key in old {
                        f.ix.left(*key);
                    }
                    for (member, to) in own {
                        f.ix.insert(*to, *member);
                    }
                }
            });
        }
        for (f, (own, _)) in all.iter_mut().zip(moves) {
            for (member, to) in own {
                if let Some(at) = usize::try_from(*member).ok().and_then(|i| f.keys.get_mut(i)) {
                    at.0 = *to;
                }
            }
        }
    }

    /// Every key of every listing instance walked: a dense key's walk timed by its entries read, a sparse key's by its
    /// lookup, its entries few. Returns the keys walked.
    fn walk(&mut self, m: &mut Measures<'_>) -> Result<Vec<Vec<u64>>, FinError> {
        let Some(out) = self.out.as_mut() else {
            return Err(FinError("the indexes walked before their fill".to_owned()));
        };
        let mut walked: Vec<Vec<u64>> = Vec::new();
        let (mut read, mut looked) = (0_u64, 0_u64);
        for f in &self.all {
            let mut keys: Vec<u64> =
                if f.ix.decl().mode.lists() { f.keys.iter().map(|k| k.0).collect() } else { Vec::new() };
            keys.sort_unstable();
            keys.dedup();
            if sparse(f) {
                looked += wide(keys.len());
            } else {
                read += keys.iter().map(|k| u64::from(f.ix.entries(*k))).sum::<u64>();
            }
            walked.push(keys);
        }
        let mut found = 0_u64;
        for (op, of_pairs, items) in [("walk", false, read), ("lookup", true, looked)] {
            found += m.read(BASE, op, items, || {
                let mut kept = 0_u64;
                for (f, keys) in self.all.iter().zip(&walked).filter(|(f, _)| sparse(f) == of_pairs) {
                    for key in keys {
                        out.clear();
                        let _ = f.ix.members(*key, |member| at_key(&f.keys, member, *key), out);
                        kept += wide(out.len());
                    }
                }
                kept
            });
        }
        if found == 0 && read + looked > 0 {
            return Err(FinError("the walks found no member".to_owned()));
        }
        Ok(walked)
    }

    /// The close: each instance's dead share read, its lists or pairs past their dead share compacted, and its new
    /// pairs merged.
    fn close(&mut self, walked: &[Vec<u64>]) -> Result<(), FinError> {
        let Some(out) = self.out.as_mut() else {
            return Err(FinError("the indexes closed before their fill".to_owned()));
        };
        for (f, keys) in self.all.iter_mut().zip(walked) {
            let (dead, entries) = f.ix.dead_and_entries();
            f.dead_share = if entries == 0 { 0.0 } else { real(dead)? / real(entries)? };
            let Filled1 { ix, keys: members, .. } = f;
            if matches!(ix.decl().keys, Keys::Sparse(_)) {
                if ix.pairs_need_compaction() {
                    ix.compact_pairs(|key, member| at_key(members, member, key));
                }
            } else if ix.decl().mode.lists() {
                for key in keys {
                    if ix.needs_compaction(*key) {
                        ix.compact(*key, |member| at_key(members, member, *key), out);
                    }
                }
            }
            ix.settle();
        }
        Ok(())
    }
}
