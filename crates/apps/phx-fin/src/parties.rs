//! `-F parties`: the party directory at the design point — every kind's parties begun, and two years of endings behind
//! them as tombstones — with live references resolved, ended ones looked up, a day's parties begun and ended, and a
//! mass failure's endings closed in one merge; and the households', firms' and institutions' stores at their byte maps,
//! their hot rows gathered and read by handle and scanned as a surprise day's wakes scan them; and every country's
//! campaign window open at once over its persons' intentions and its households' vote occasions.

use std::collections::BTreeMap;
use std::hint::black_box;

use phx_exec::trace::{Reading, Spent};
use phx_id::{Day, PartyRef};
use phx_num::Missing;
use phx_pop::directory::{Directory, Resolved};
use phx_pop::kinds::{Attr, AttrW, KindStore, Opening};
use phx_pop::layout::{FIRM, GroupDecl, HOUSEHOLD, IntTy, KindMap, Layout, WordDecl};
use phx_pop::windowed::{WAttrW, WindowLayout, WindowedGroup};
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
/// The kinds the stores hold, by their places among `KINDS`.
const HOUSEHOLDS: u8 = 1;
const FIRMS: u8 = 2;
const INSTITUTIONS: u8 = 3;
/// An institution's record at the design point: one group of the width most institution kinds declare.
const INSTITUTION: KindMap =
    KindMap { kind: "institution", groups: &[GroupDecl { name: "record", width: 1_024, words: &[] }] };
/// The campaign's windowed groups: a person's voting intention, and a household's next vote occasion.
const INTENTION: &[GroupDecl] = &[GroupDecl {
    name: "campaign",
    width: 1,
    words: &[WordDecl { name: "intention", ty: IntTy::U8, count: 1, absent: true, writer: "S5.137" }],
}];
const VOTE: &[GroupDecl] = &[GroupDecl {
    name: "campaign",
    width: 2,
    words: &[WordDecl { name: "vote", ty: IntTy::U16, count: 1, absent: true, writer: "S1.227" }],
}];
/// The countries, each a third of every kind's slots as the opening begins them country by country, and a campaign's
/// days.
const COUNTRIES: u32 = 3;
const CAMPAIGN_DAYS: u32 = 42;
const PERSONS: u8 = 0;

/// The zones a household's residence and a firm's zone are drawn among, and the households' and firms' visits a day
/// gathers.
const ZONES: u64 = 1_000;
const HOUSEHOLD_VISITS: u64 = 1_600_000;
const FIRM_VISITS: u64 = 900_000;
/// The words a household visit reads from its gathered hot row, and the visits whose rows stay in cache together.
const VISIT_READS: u64 = 4;
const VISIT_BATCH: u64 = 512;

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
    /// The households', firms' and institutions' stores, the handles the fill writes and the day reads, and the most
    /// pages a day's reads and writes faulted.
    stores: Vec<(u8, KindStore)>,
    handles: Option<Handles>,
    faults: Option<u64>,
    /// The persons' and households' campaign groups with their write handles, and each kind's slots a country holds.
    windows: Option<Campaign>,
}

/// The campaign's groups: each kind's, its handle, and the slots each of its countries covers.
#[derive(Debug)]
struct Campaign {
    groups: Vec<(u8, WindowedGroup, Written, u32)>,
}

/// A windowed group's write handle, by its word's type.
#[derive(Clone, Copy, Debug)]
enum Written {
    Intention(WAttrW<u8>),
    Vote(WAttrW<u16>),
}

impl Campaign {
    fn new(dir: &Directory) -> Result<Campaign, FinError> {
        let (mut persons, mut households) = (
            WindowLayout::compile("person", INTENTION).map_err(FinError)?,
            WindowLayout::compile("household", VOTE).map_err(FinError)?,
        );
        let intention = persons.writer::<u8>("intention", 0, "S5.137").map_err(FinError)?;
        let vote = households.writer::<u16>("vote", 0, "S1.227").map_err(FinError)?;
        let subjects = index(u64::from(COUNTRIES))?;
        Ok(Campaign {
            groups: vec![
                (
                    PERSONS,
                    WindowedGroup::new(PERSONS, &persons, subjects),
                    Written::Intention(intention),
                    dir.high_water(PERSONS),
                ),
                (
                    HOUSEHOLDS,
                    WindowedGroup::new(HOUSEHOLDS, &households, subjects),
                    Written::Vote(vote),
                    dir.high_water(HOUSEHOLDS),
                ),
            ],
        })
    }

    /// A country's slots of a kind: its third of the slots the opening began.
    fn slots(high: u32, country: u32) -> (phx_id::Slot, u32) {
        let share = high.div_ceil(COUNTRIES);
        let first = share * country;
        if first >= high {
            return (phx_id::Slot::new(high), 0);
        }
        let end = if first + share < high { first + share } else { high };
        (phx_id::Slot::new(first), end - first)
    }

    /// A country's windows opened over its slots, nothing yet written.
    fn open(&mut self, country: u32, today: u32) {
        let mut space = AddressSpace::empty();
        for (_, group, _, high) in &mut self.groups {
            let span = Campaign::slots(*high, country);
            group.open(&mut space, country, span, (Day::new(today), Day::new(today + CAMPAIGN_DAYS)));
        }
    }

    /// Every live party a country's windows cover written, as its campaign's first day writes them: the parties
    /// written.
    fn write(&mut self, dir: &Directory, country: u32, today: u32) -> u64 {
        let mut written = 0;
        let day = u16::try_from(today).unwrap_or(u16::MAX - 1);
        for (kind, group, w, high) in &mut self.groups {
            let (first, n) = Campaign::slots(*high, country);
            let covered = dir.live_slots(*kind).skip_while(|s| *s < first).take_while(|s| s.get() < first.get() + n);
            for slot in covered {
                let Some(r) = dir.at(*kind, slot) else { continue };
                match w {
                    Written::Intention(a) => group.set(dir, r, *a, Missing::Present(1)),
                    Written::Vote(a) => group.set(dir, r, *a, Missing::Present(day)),
                }
                written += 1;
            }
        }
        written
    }

    fn close(&mut self, country: u32) {
        for (_, group, _, _) in &mut self.groups {
            group.close(country);
        }
    }

    fn begun(&mut self, r: PartyRef) {
        for (kind, group, _, _) in &mut self.groups {
            if *kind == r.kind() {
                group.begun(r);
            }
        }
    }

    fn bytes(&self) -> u64 {
        self.groups.iter().map(|(_, g, _, _)| StoreStats::bytes(g)).sum()
    }
}

/// The words the fill opens and the day reads: a household's residence, preference and formation day; a firm's own
/// unit and zone.
#[derive(Clone, Copy, Debug)]
struct Handles {
    residence: AttrW<u32>,
    preference: AttrW<u16>,
    formed: AttrW<u32>,
    states: Attr<u8>,
    flags: Attr<u8>,
    own_unit: AttrW<u32>,
    zone: AttrW<u16>,
}

impl Handles {
    fn of(household: &mut Layout, firm: &mut Layout) -> Result<Handles, FinError> {
        Ok(Handles {
            residence: household.writer("residence", 0, "K-32").map_err(FinError)?,
            preference: household.writer("preference", 0, "K-32").map_err(FinError)?,
            formed: household.writer("formed", 0, "K-32").map_err(FinError)?,
            states: household.attr("states", 0).map_err(FinError)?,
            flags: household.attr("flags", 0).map_err(FinError)?,
            own_unit: firm.writer("units", 0, "K-60").map_err(FinError)?,
            zone: firm.writer("zone", 0, "K-32").map_err(FinError)?,
        })
    }

    /// A party's opening words, drawn by its kind.
    fn opening(self, kind: u8, d: &mut phx_rand::Draws, day: u32) -> Result<Vec<Opening>, FinError> {
        let zone = u32::try_from(below_u64(d, ZONES)).map_err(err)?;
        Ok(match kind {
            HOUSEHOLDS => vec![
                Opening::of(self.residence, zone),
                Opening::of(self.preference, u16::try_from(below_u64(d, u64::from(u8::MAX))).map_err(err)?),
                Opening::of(self.formed, day),
            ],
            FIRMS => vec![Opening::of(self.own_unit, zone), Opening::of(self.zone, u16::try_from(zone).map_err(err)?)],
            _ => Vec::new(),
        })
    }
}

/// A word read into the measure's checksum, which keeps the reads from being optimised away; an absent one adds one.
fn folded<T: Into<u64>>(w: Missing<T>) -> u64 {
    match w {
        Missing::Present(v) => v.into(),
        Missing::Absent => 1,
    }
}

/// A party begun in the directory written in its kind's store, where the kind has one.
fn begin_in(
    stores: &mut [(u8, KindStore)],
    dir: &Directory,
    (r, handles): (PartyRef, Handles),
    (d, day): (&mut phx_rand::Draws, u32),
) -> Result<(), FinError> {
    if let Some((_, store)) = stores.iter_mut().find(|(k, _)| *k == r.kind()) {
        store.begin(dir, r, &handles.opening(r.kind(), d, day)?);
    }
    Ok(())
}

fn err(e: impl std::fmt::Display) -> FinError {
    FinError(e.to_string())
}

impl Parties {
    /// The day's visits' hot rows gathered — the households' read by four handles, the firms' by one — and a surprise
    /// day's scan of every household's and firm's hot row; the pages they faulted kept.
    fn kinds_day(&mut self, d: &mut phx_rand::Draws, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(dir), Some(handles)) = (self.directory.as_ref(), self.handles) else {
            return Err(FinError("the stores measured before their fill".to_owned()));
        };
        let of_kind = |kind: u8, n: u64, d: &mut phx_rand::Draws| -> Result<Vec<PartyRef>, FinError> {
            let mut out = Vec::with_capacity(index(n)?);
            while wide(out.len()) < n {
                let at = index(below_u64(d, wide(self.live.len())))?;
                out.extend(self.live.get(at).copied().filter(|r| r.kind() == kind));
            }
            out.sort_unstable_by_key(|r| r.slot());
            Ok(out)
        };
        let (households, firms) = (of_kind(HOUSEHOLDS, HOUSEHOLD_VISITS, d)?, of_kind(FIRMS, FIRM_VISITS, d)?);
        let store = |k: u8| self.stores.iter().find(|(x, _)| *x == k).map(|(_, s)| s);
        let (Some(hs), Some(fs)) = (store(HOUSEHOLDS), store(FIRMS)) else {
            return Err(FinError("no household or firm store".to_owned()));
        };
        // The gathers and the scan read rows the fill committed: a day of them faults no page.
        let before = Reading::now();
        self.folded ^= m.read(BASE, "gather", HOUSEHOLD_VISITS + FIRM_VISITS, || {
            let firms: u64 = firms.iter().map(|r| folded(fs.gather(dir, *r, 0).get(handles.own_unit.read()))).sum();
            let households: u64 =
                households.iter().map(|r| folded(hs.gather(dir, *r, 0).get(handles.residence.read()))).sum();
            black_box(firms + households)
        });
        let scanned = hs.rows() + fs.rows();
        self.folded ^= m.read(BASE, "wake_scan", u64::from(scanned), || {
            // A surprise wakes the households whose attributes it touches: each hot row's first words read in turn.
            let (residence, flags) = (handles.residence.read(), handles.flags);
            let woken = hs
                .scan(0)
                .filter(|(_, row)| row.get(residence) == Missing::Present(0) || row.get(flags) != Missing::Present(0))
                .count();
            let own = handles.own_unit.read();
            let sited = fs.scan(0).filter(|(_, row)| row.get(own) == Missing::Present(0)).count();
            black_box(wide(woken + sited))
        });
        let spent = Spent::between(None, &before, &Reading::now());
        self.faults = match (self.faults, spent.faults) {
            (Some(a), Some(b)) => Some(if a > b { a } else { b }),
            (a, b) => a.or(b),
        };
        // A visit's reads follow its gather while its row is in cache: a batch of visits' rows gathered, then read by
        // four handles each, the batch read again to stand for the visits' days.
        let rows: Vec<_> = households.iter().take(index(VISIT_BATCH)?).map(|r| hs.gather(dir, *r, 0)).collect();
        let reps = HOUSEHOLD_VISITS / VISIT_BATCH;
        self.folded ^= m.read(BASE, "attr", reps * wide(rows.len()) * VISIT_READS, || {
            let mut sum = 0_u64;
            for _ in 0..reps {
                for row in &rows {
                    sum += folded(row.get(handles.residence.read()))
                        + folded(row.get(handles.states))
                        + folded(row.get(handles.preference.read()))
                        + folded(row.get(handles.formed.read()));
                }
            }
            black_box(sum)
        });
        Ok(())
    }
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
        let (mut household, mut firm) =
            (Layout::compile(&HOUSEHOLD, &[]).map_err(FinError)?, Layout::compile(&FIRM, &[]).map_err(FinError)?);
        let institution = Layout::compile(&INSTITUTION, &[]).map_err(FinError)?;
        let handles = Handles::of(&mut household, &mut firm)?;
        let cap = |k: u8| capacities.get(usize::from(k)).copied().ok_or_else(|| FinError(format!("no kind {k}")));
        self.stores = vec![
            (HOUSEHOLDS, KindStore::new(&mut space, HOUSEHOLDS, &household, cap(HOUSEHOLDS)?)),
            (FIRMS, KindStore::new(&mut space, FIRMS, &firm, cap(FIRMS)?)),
            (INSTITUTIONS, KindStore::new(&mut space, INSTITUTIONS, &institution, cap(INSTITUTIONS)?)),
        ];
        let mut d = streams.draws(BASE, 0, 0);
        for (kind, n) in (0_u8..).zip(&counts) {
            for _ in 0..*n {
                let r = dir.begin(kind);
                begin_in(&mut self.stores, &dir, (r, handles), (&mut d, 0))?;
                self.live.push(r);
            }
        }
        let _ = dir.close_day(Day::new(0));
        let a_day = tombstones / u64::from(HISTORY_DAYS);
        for day in 1..=HISTORY_DAYS {
            for _ in 0..a_day {
                let at = index(below_u64(&mut d, wide(self.live.len())))?;
                let Some(&party) = self.live.get(at) else { continue };
                dir.end(party, Day::new(day), Missing::Absent);
                self.ended.push(party);
                let r = dir.begin(party.kind());
                begin_in(&mut self.stores, &dir, (r, handles), (&mut d, day))?;
                if let Some(slot) = self.live.get_mut(at) {
                    *slot = r;
                }
            }
            let _ = dir.close_day(Day::new(day));
        }
        self.held = StoreStats::bytes(&dir);
        self.handles = Some(handles);
        // Every country's campaign open at once, each over its own third of the persons and households.
        let mut campaign = Campaign::new(&dir)?;
        for country in 0..COUNTRIES {
            campaign.open(country, HISTORY_DAYS);
            let _ = campaign.write(&dir, country, HISTORY_DAYS);
        }
        self.windows = Some(campaign);
        (self.directory, self.streams, self.today) = (Some(dir), Some(*streams), HISTORY_DAYS);
        Ok(Filled { rows: counts.iter().sum() })
    }

    /// Live references resolved and ended ones looked up, a day's parties ended and begun, and a mass failure's
    /// endings closed in one merge; then the day's visits' hot rows gathered and read, and a surprise day's scan.
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
        let mut begun = Vec::with_capacity(index(TURNOVER + MASS)?);
        m.read(BASE, "begin_end", 2 * TURNOVER, || {
            for at in &turnover {
                let Some(slot) = lives.get_mut(*at) else { continue };
                if matches!(dir.resolve(*slot), Resolved::Live(_)) {
                    dir.end(*slot, today, Missing::Absent);
                    *slot = dir.begin(slot.kind());
                    begun.push(*slot);
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
                    begun.push(*slot);
                }
            }
            black_box(dir.close_day(today))
        });
        let Some(handles) = self.handles else {
            return Err(FinError("the stores measured before their fill".to_owned()));
        };
        // The parties begun write their rows outside the directory's measures, which are the directory's alone.
        let Some(campaign) = self.windows.as_mut() else {
            return Err(FinError("the campaign measured before its fill".to_owned()));
        };
        for r in begun {
            begin_in(&mut self.stores, dir, (r, handles), (&mut d, self.today))?;
            campaign.begun(r);
        }
        // One country's campaign closed and opened again over its parties as they are now.
        let (reader, today) = (&*dir, self.today);
        m.read(BASE, "window_close", 1, || campaign.close(0));
        m.read(BASE, "window_open", 1, || campaign.open(0, today));
        let (first, n) = Campaign::slots(dir.high_water(PERSONS), 0);
        let covered =
            wide(dir.live_slots(PERSONS).skip_while(|s| *s < first).take_while(|s| s.get() < first.get() + n).count());
        let written = m.read(BASE, "window_write", covered, || campaign.write(reader, 0, today));
        black_box(written);
        self.kinds_day(&mut d, m)
    }

    fn bytes(&self) -> Bytes {
        let stores: u64 = self.stores.iter().map(|(_, s)| StoreStats::bytes(s)).sum();
        Bytes { rows: self.held + stores, resident: 0 }
    }

    /// The directory's megabytes and tombstones; each store's bytes a party and its hot row's; the kinds' megabytes
    /// committed; and the most pages a day's reads faulted.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let real = |n: u64| n.to_string().parse::<f64>().ok();
        let mb = real(self.held).map(|b| b / MIB);
        let tombs = self.directory.as_ref().map(|d| wide(d.tombstones())).and_then(real);
        let store = |k: u8| self.stores.iter().find(|(x, _)| *x == k).map(|(_, s)| s);
        // A party's bytes are its rows' declared widths, which only a widened group raises.
        let per = |k: u8, hot: bool| {
            let widths = store(k)?.widths();
            let bytes: u16 = if hot { widths.first().copied()? } else { widths.iter().sum() };
            Some(f64::from(bytes))
        };
        let kinds_mb = real(self.stores.iter().map(|(_, s)| StoreStats::bytes(s)).sum()).map(|b| b / MIB);
        [
            mb.map(|m| ("directory_mb", m)),
            tombs.map(|n| ("tombstones", n)),
            per(HOUSEHOLDS, false).map(|b| ("household_bytes", b)),
            per(HOUSEHOLDS, true).map(|b| ("household_hot_bytes", b)),
            per(FIRMS, false).map(|b| ("firm_bytes", b)),
            per(FIRMS, true).map(|b| ("firm_hot_bytes", b)),
            kinds_mb.map(|m| ("kinds_mb", m)),
            self.windows.as_ref().and_then(|c| real(c.bytes())).map(|b| ("windowed_mb", b / MIB)),
            self.faults.and_then(real).map(|f| ("faults_per_day", f)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}
