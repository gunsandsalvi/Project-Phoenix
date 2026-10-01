//! `-F parties`: the party directory at the design point — every kind's parties begun, and two years of endings behind
//! them as tombstones — with live references resolved, ended ones looked up, a day's parties begun and ended, and a
//! mass failure's endings closed in one merge.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_id::{Day, PartyRef};
use phx_num::Missing;
use phx_pop::directory::{Directory, Resolved};
use phx_rand::uniform::below_u64;
use phx_store::{AddressSpace, StoreStats};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the directory is measured under.
pub const BASE: &str = "parties";

/// The kinds the design point holds, by their `[store]` counts.
const KINDS: [&str; 4] = ["persons", "households", "firms", "institutions"];
/// The days of endings behind the opening, the tombstones' horizon, and room past each kind's count.
const HISTORY_DAYS: u32 = 730;
const ROOM_QUARTERS: u64 = 5;
const QUARTERS: u64 = 4;
/// Ended references looked up a day (and live ones on a day that applies nothing), the parties begun and ended in a
/// measured stretch, and a mass failure's endings.
const READS: u64 = 1_000_000;
const TURNOVER: u64 = 11_000;
const MASS: u64 = 115_000;
/// The rows a chunk of generations holds.
const CHUNK_ROWS: u32 = 4_096;

const MIB: f64 = 1_048_576.0;

/// The directory, its live and ended references, and the day reached.
#[derive(Debug, Default)]
pub struct Parties {
    directory: Option<Directory>,
    live: Vec<PartyRef>,
    ended: Vec<PartyRef>,
    streams: Option<Streams>,
    today: u32,
    /// The directory's bytes at the design point, as the fill leaves it: its parties and two years of tombstones.
    held: u64,
    folded: u64,
}

fn err(e: impl std::fmt::Display) -> FinError {
    FinError(e.to_string())
}

impl FinBase for Parties {
    fn name(&self) -> &'static str {
        BASE
    }

    /// Every kind's parties begun at the opening, then two years of the design point's endings, each a party of a
    /// drawn kind ended and one begun in its place, the days closed in turn.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let store = |key: &str| design.store.get(key).copied().ok_or_else(|| FinError(format!("no [store] {key}")));
        let counts: Vec<u64> = KINDS.iter().map(|k| store(k)).collect::<Result<_, _>>()?;
        let tombstones = store("tombstones")?;
        let capacities: Vec<u32> = counts
            .iter()
            .map(|n| u32::try_from(n * ROOM_QUARTERS / QUARTERS).map_err(err))
            .collect::<Result<_, _>>()?;
        let mut space = AddressSpace::empty();
        let mut dir: Directory = Directory::new(&mut space, &capacities, CHUNK_ROWS, (Day::new(0), HISTORY_DAYS));
        for (kind, n) in (0_u8..).zip(&counts) {
            for _ in 0..*n {
                self.live.push(dir.begin(kind));
            }
        }
        let _ = dir.close_day(Day::new(0));
        let mut d = streams.draws(BASE, 0, 0);
        let a_day = tombstones / u64::from(HISTORY_DAYS);
        for day in 1..=HISTORY_DAYS {
            for _ in 0..a_day {
                let at = index(below_u64(&mut d, wide(self.live.len())))?;
                let Some(&party) = self.live.get(at) else { continue };
                dir.end(party, Day::new(day), Missing::Absent);
                self.ended.push(party);
                if let Some(slot) = self.live.get_mut(at) {
                    *slot = dir.begin(party.kind());
                }
            }
            let _ = dir.close_day(Day::new(day));
        }
        self.held = StoreStats::bytes(&dir);
        (self.directory, self.streams, self.today) = (Some(dir), Some(*streams), HISTORY_DAYS);
        Ok(Filled { rows: counts.iter().sum() })
    }

    /// Live references resolved and ended ones looked up, a day's parties ended and begun, and a mass failure's
    /// endings closed in one merge.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(dir), Some(streams)) = (self.directory.as_mut(), self.streams) else {
            return Err(FinError("the directory measured before its fill".to_owned()));
        };
        let mut d = streams.draws(BASE, 1, crate::kept::day_of(day)?);
        let mut pick = |from: &[PartyRef]| -> Result<PartyRef, FinError> {
            from.get(index(below_u64(&mut d, wide(from.len())))?)
                .copied()
                .ok_or_else(|| FinError("no party".to_owned()))
        };
        // A gather meets the day's references in slot order, as its rows lie: as many as the day's applies, as the
        // references' own base reads them.
        let reads = counts.get("applies").copied().unwrap_or(READS);
        let mut live: Vec<PartyRef> = (0..reads).map(|_| pick(&self.live)).collect::<Result<_, _>>()?;
        let mut ended: Vec<PartyRef> = (0..READS).map(|_| pick(&self.ended)).collect::<Result<_, _>>()?;
        live.sort_unstable_by_key(|r| (r.kind(), r.slot()));
        ended.sort_unstable_by_key(|r| (r.kind(), r.slot()));
        let reader = &*dir;
        self.folded ^= m.read(BASE, "resolve", reads, || {
            black_box(wide(live.iter().filter(|r| matches!(reader.resolve(**r), Resolved::Live(_))).count()))
        });
        self.folded ^= m.read(BASE, "tomb_lookup", READS, || {
            black_box(wide(ended.iter().filter(|r| !matches!(reader.resolve(**r), Resolved::Live(_))).count()))
        });
        self.today += 1;
        let today = Day::new(self.today);
        let turnover: Vec<usize> =
            (0..TURNOVER).map(|_| index(below_u64(&mut d, wide(self.live.len())))).collect::<Result<_, _>>()?;
        let lives = &mut self.live;
        m.read(BASE, "begin_end", 2 * TURNOVER, || {
            for at in &turnover {
                let Some(slot) = lives.get_mut(*at) else { continue };
                if matches!(dir.resolve(*slot), Resolved::Live(_)) {
                    dir.end(*slot, today, Missing::Absent);
                    *slot = dir.begin(slot.kind());
                }
            }
        });
        let mass: Vec<usize> =
            (0..MASS).map(|_| index(below_u64(&mut d, wide(lives.len())))).collect::<Result<_, _>>()?;
        m.read(BASE, "mass_end", 1, || {
            for at in &mass {
                let Some(slot) = lives.get_mut(*at) else { continue };
                if matches!(dir.resolve(*slot), Resolved::Live(_)) {
                    dir.end(*slot, today, Missing::Absent);
                    *slot = dir.begin(slot.kind());
                }
            }
            black_box(dir.close_day(today))
        });
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: self.held, resident: 0 }
    }

    fn figures(&self) -> Vec<(&'static str, f64)> {
        let mb = (self.bytes().rows.to_string().parse::<f64>().ok()).map(|b| b / MIB);
        let tombs =
            self.directory.as_ref().map(|d| wide(d.tombstones())).and_then(|n| n.to_string().parse::<f64>().ok());
        [mb.map(|m| ("directory_mb", m)), tombs.map(|n| ("tombstones", n))].into_iter().flatten().collect()
    }
}
