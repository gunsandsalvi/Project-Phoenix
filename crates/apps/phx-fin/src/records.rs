//! `-F records`: the horizon rings' part — the design point's events and filed statements, each a ring run past its full
//! horizon; each measured day a month of each ring's days appended and pruned by whole chunks, and the month's events
//! read back by range — and the stored log's: a horizon's days of stored events, one segment a day, then each measured
//! day's segment sorted, encoded, read back by key and the horizon's edge pruned.

use std::collections::BTreeMap;

use phx_id::Day;
use phx_rand::draws::Draws;
use phx_rand::uniform::below_u64;
use phx_record::storage::{SegmentId, Storage, StorageError};
use phx_record::stored::{Item, Key, Reads, StoredDecl, StoredLog};
use phx_store::{AddressSpace, HorizonRing, StoreStats};
use toml::Value;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, slots};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the rings' appends are measured under.
pub const BASE: &str = "records";

/// The base whose own figures the rings' prunes and ranges are.
pub const RING: &str = "ring";

/// An event's row: kind, flags, subjects and its first subject, its cause and size.
type Event = [u64; 3];

/// A filed statement's row as the ring holds it before its block packs it: filer, period, vintage and its lines.
type Statement = [u64; 6];

/// The days a year of filings keeps.
const DAYS_A_YEAR: u64 = 365;

/// The rings' days a measured day runs: a day's appends are a few microseconds, below what one reading of the clock
/// tells from its own cost and the machine's noise, so each operation is read once over a month of them.
const RING_DAYS: u32 = 30;

/// The stored events a day: those past the resident tier's, kept on storage.
const STORED_A_DAY: u64 = 100_000;
/// The kinds and first subjects a stored event's key is drawn over.
const STORED_KINDS: u64 = 64;
const STORED_SUBJECTS: u64 = 6_000_000;
/// A stored event's frame, its bytes.
const STORED_FRAME: u32 = 1 << 14;
/// The keys a measured day reads back from its own segment.
const STORED_READS: u64 = 1_000;
const MIB: f64 = 1_048_576.0;
/// The stored events' row among the base's streams.
const STORED_ROW: u64 = 1;

/// A stored event: its kind and first subject, then its subjects, cause and size.
#[derive(Clone, Copy, Debug)]
struct Stored {
    key: Key,
    words: [i64; 3],
}

impl Item for Stored {
    const WORDS: usize = 3;

    fn key(&self) -> Key {
        self.key
    }

    fn words(&self, out: &mut [i64]) {
        for (o, w) in out.iter_mut().zip(self.words) {
            *o = w;
        }
    }

    fn of(key: Key, words: &[i64]) -> Stored {
        let mut w = [0; 3];
        for (o, x) in w.iter_mut().zip(words) {
            *o = *x;
        }
        Stored { key, words: w }
    }
}

fn stored(d: &mut Draws) -> Stored {
    let small = |d: &mut Draws, n: u64| i64::try_from(below_u64(d, n)).unwrap_or(i64::MAX);
    Stored {
        key: Key(below_u64(d, STORED_KINDS), below_u64(d, STORED_SUBJECTS)),
        words: [small(d, 8) + 1, small(d, 40), small(d, 1_000_000) - 500_000],
    }
}

/// Storage that holds only what the measured days write: the fill's segments are their index's alone, as a long run
/// leaves its storage, without holding a year of bytes in memory.
#[derive(Debug, Default)]
struct Held {
    keep: bool,
    segs: BTreeMap<SegmentId, Vec<u8>>,
    writing: Vec<u8>,
}

fn missing() -> StorageError {
    StorageError("a segment the measured days did not write".to_owned())
}

impl Storage for Held {
    fn put(&mut self, id: SegmentId, bytes: &[u8]) -> Result<(), StorageError> {
        if self.keep {
            self.segs.insert(id, bytes.to_vec());
        }
        Ok(())
    }

    fn read(&self, id: SegmentId, offset: u64, out: &mut [u8]) -> Result<(), StorageError> {
        let seg = self.segs.get(&id).ok_or_else(missing)?;
        let at = usize::try_from(offset).map_err(|_| missing())?;
        out.copy_from_slice(seg.get(at..at + out.len()).ok_or_else(missing)?);
        Ok(())
    }

    fn read_rest(&self, id: SegmentId, offset: u64, out: &mut Vec<u8>) -> Result<(), StorageError> {
        let seg = self.segs.get(&id).ok_or_else(missing)?;
        out.extend_from_slice(seg.get(usize::try_from(offset).map_err(|_| missing())?..).ok_or_else(missing)?);
        Ok(())
    }

    fn remove(&mut self, id: SegmentId) -> Result<(), StorageError> {
        self.segs.remove(&id);
        Ok(())
    }

    fn begin(&mut self) -> Result<(), StorageError> {
        self.writing.clear();
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        self.writing.extend_from_slice(bytes);
        Ok(())
    }

    fn commit(&mut self, id: SegmentId) -> Result<(), StorageError> {
        let bytes = std::mem::take(&mut self.writing);
        if self.keep {
            self.segs.insert(id, bytes);
        }
        Ok(())
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

fn stored_err(e: StorageError) -> FinError {
    FinError(e.0)
}

/// One ring, its rows a day and its horizon.
#[derive(Debug)]
struct Kept<T: phx_store::Pod> {
    ring: HorizonRing<T>,
    a_day: u64,
    horizon: u32,
}

/// The events' and statements' rings, the streams the days draw from, the day it is, and the last range's rows.
#[derive(Debug, Default)]
pub struct Records {
    events: Option<Kept<Event>>,
    statements: Option<Kept<Statement>>,
    /// The stored log of events past the resident tier, and the day it has reached.
    stored: Option<StoredLog<Stored, Held>>,
    stored_day: u32,
    streams: Option<Streams>,
    today: u32,
    read: u64,
}

impl Records {
    /// The rows the last measured day's range read, the same for any workers.
    #[must_use]
    pub fn read(&self) -> u64 {
        self.read
    }
}

/// A horizon as the resolution settles it: a number, or the last of a valve's settings, `"730 → 365"`.
fn settled(value: &Value, key: &str) -> Result<u64, FinError> {
    let refused = || FinError(format!("`resolution.{key}` is no horizon"));
    match value {
        Value::Integer(n) => u64::try_from(*n).map_err(|_| refused()),
        Value::String(s) => s.rsplit('→').next().and_then(|n| n.trim().parse().ok()).ok_or_else(refused),
        _ => Err(refused()),
    }
}

/// A ring of `rows` live rows at a horizon of `horizon` days, in chunks of a day's rows, run through days of drawn rows
/// to `last` and pruned each month, as the measured days run it, so it holds its horizon and the chunks it has pruned,
/// as a long run leaves it.
fn filled<T: phx_store::Pod>(
    (rows, horizon): (u64, u64),
    (streams, last): (Streams, u32),
    draw: fn(&mut Draws) -> T,
) -> Result<Kept<T>, FinError> {
    let a_day = rows.div_ceil(horizon);
    let chunk = slots(a_day.next_power_of_two())?;
    let horizon = slots(horizon)?;
    // The horizon's days, the day at its edge, and the month appended before its prune.
    let chunks = horizon + 2 + RING_DAYS;
    let mut ring = HorizonRing::new(&mut AddressSpace::empty(), (chunk, chunks), horizon).map_err(FinError)?;
    let mut d = streams.draws(BASE, 0, 0);
    let mut day_rows = Vec::new();
    for day in 0..=last {
        day_rows.clear();
        day_rows.extend((0..a_day).map(|_| draw(&mut d)));
        let _ = ring.append(day, &day_rows);
        if day % RING_DAYS == 0 || day == last {
            let _ = ring.prune(day);
        }
    }
    Ok(Kept { ring, a_day, horizon })
}

fn event(d: &mut Draws) -> Event {
    [below_u64(d, u64::MAX), below_u64(d, u64::MAX), below_u64(d, u64::MAX)]
}

fn statement(d: &mut Draws) -> Statement {
    let mut s = [0; 6];
    for v in &mut s {
        *v = below_u64(d, u64::MAX);
    }
    s
}

/// A month of days' rows drawn, then appended day by day under the ring's own name: a row's append is a copy into a
/// chunk the ring pruned long ago, so its cost is its width in cold memory.
fn month_of_ring<T: phx_store::Pod>(
    (k, append): (&mut Kept<T>, &str),
    (first, d): (u32, &mut Draws),
    draw: fn(&mut Draws) -> T,
    m: &mut Measures<'_>,
) {
    let days: Vec<Vec<T>> = (0..RING_DAYS).map(|_| (0..k.a_day).map(|_| draw(d)).collect()).collect();
    let ring = &mut k.ring;
    m.read(BASE, append, k.a_day * u64::from(RING_DAYS), || {
        for (day, rows) in (first..).zip(&days) {
            let _ = ring.append(day, rows);
        }
    });
}

/// The month's prune: it visits no row, so its cost is shared by the rows it ages out, a month's.
fn prune_month<T: phx_store::Pod>(k: &mut Kept<T>, today: u32, m: &mut Measures<'_>) {
    let ring = &mut k.ring;
    let _ = m.read(RING, "prune", k.a_day * u64::from(RING_DAYS), || ring.prune(today));
}

impl FinBase for Records {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] events` at `horizon_events_days` and `[store] filed_statements` at `horizon_filings_years`.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let events = count(&design.store, "events", "store")?;
        let statements = count(&design.store, "filed_statements", "store")?;
        let events_days = settled(design.resolution("horizon_events_days")?, "horizon_events_days")?;
        let filings_days = settled(design.resolution("horizon_filings_years")?, "horizon_filings_years")? * DAYS_A_YEAR;
        // Every ring past its horizon, so each has pruned and takes its chunks again.
        let last = slots(if events_days > filings_days { events_days } else { filings_days })? + 1;
        let e = filled((events, events_days), (*streams, last), event)?;
        let s = filled((statements, filings_days), (*streams, last), statement)?;
        self.today = last;
        (self.events, self.statements, self.streams) = (Some(e), Some(s), Some(*streams));
        // The stored log run through its horizon's days, a segment a day, the day at its edge pruned.
        let decl = StoredDecl { name: "events", frame_bytes: STORED_FRAME, horizon: slots(events_days)?, tiers: &[] };
        let mut log = StoredLog::new(decl, Held::default());
        let mut items = Vec::new();
        for day in 0..=slots(events_days)? {
            // Each day's events from a stream of their own day's, which a year of draws would exhaust.
            let mut d = streams.draws(BASE, STORED_ROW, day);
            items.clear();
            items.extend((0..STORED_A_DAY).map(|_| stored(&mut d)));
            log.flush(Day::new(day), &items).map_err(stored_err)?;
            log.prune(Day::new(day), &[]).map_err(stored_err)?;
        }
        log.storage_mut().keep = true;
        self.stored_day = slots(events_days)?;
        self.stored = Some(log);
        Ok(Filled { rows: events + statements + STORED_A_DAY * events_days })
    }

    /// A month of days' events appended, read back by range at once, as a reader of a ring's history reads it, and
    /// pruned; then a month of statements appended and pruned.
    fn day(&mut self, day: DayType, _: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(events), Some(statements), Some(streams)) =
            (self.events.as_mut(), self.statements.as_mut(), self.streams)
        else {
            return Err(FinError("the records measured before their fill".to_owned()));
        };
        let first = self.today + 1;
        self.today += RING_DAYS;
        let today = self.today;
        let mut d = streams.draws(BASE, u64::from(today), day_of(day)?);
        month_of_ring((events, "append"), (first, &mut d), event, m);
        let ring = &events.ring;
        let (read, folded) = m.read(RING, "range", events.a_day * u64::from(RING_DAYS), || {
            let (mut n, mut folded) = (0, 0_u64);
            // The reader folds each row's first word, so the rows are read and its own arithmetic costs nothing.
            ring.range((first, today), |_, _, rows| {
                n += rows.len();
                folded = rows.iter().fold(folded, |f, r| f ^ r[0]);
            });
            (n, folded)
        });
        // What the reader folded is kept, so the rows it read are read.
        let _ = std::hint::black_box(folded);
        self.read = crate::kept::wide(read);
        prune_month(events, today, m);
        month_of_ring((statements, "append_statements"), (first, &mut d), statement, m);
        prune_month(statements, today, m);
        self.stored_day(&mut d, m)
    }

    fn bytes(&self) -> Bytes {
        let rows =
            self.events.as_ref().map_or(0, |k| k.ring.bytes()) + self.statements.as_ref().map_or(0, |k| k.ring.bytes());
        Bytes { rows, resident: 0 }
    }

    /// The events ring's rows live against its rows a day over its horizon and the day at its edge; the stored log's
    /// resident index and its frames, and the bytes a day's segment holds.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let growth = |k: &Kept<_>| {
            let kept = k.ring.rows_live().to_string().parse::<f64>().ok()?;
            let due = (k.a_day * (u64::from(k.horizon) + 1)).to_string().parse::<f64>().ok()?;
            Some(kept / due)
        };
        let mut out = Vec::new();
        if let Some(Some(g)) = self.events.as_ref().map(growth) {
            out.push(("events_growth", g));
        }
        let real = |n: u64| n.to_string().parse::<f64>().ok();
        if let Some(log) = self.stored.as_ref() {
            let days = log.segments().count();
            out.extend(real(log.index_bytes()).map(|b| ("index_mb", b / MIB)));
            out.extend(real(log.frames()).map(|f| ("stored_frames", f)));
            if let (Some(bytes), Some(n)) = (real(log.stored_bytes()), real(crate::kept::wide(days)))
                && n > 0.0
            {
                out.push(("stored_mb_a_day", bytes / n / MIB));
            }
        }
        out
    }
}

impl Records {
    /// A measured day's stored events: sorted into the log's order, encoded into its segment, read back by key, and
    /// the horizon's edge pruned.
    fn stored_day(&mut self, d: &mut Draws, m: &mut Measures<'_>) -> Result<(), FinError> {
        let Some(log) = self.stored.as_mut() else {
            return Err(FinError("the stored log measured before its fill".to_owned()));
        };
        self.stored_day += 1;
        let today = Day::new(self.stored_day);
        let items: Vec<Stored> = (0..STORED_A_DAY).map(|_| stored(d)).collect();
        m.read(BASE, "sort", STORED_A_DAY, || log.stage(&items));
        let encoded = m.read(BASE, "encode", STORED_A_DAY, || log.encode_staged(today));
        let _ = std::hint::black_box(encoded);
        m.read(BASE, "hand_over", STORED_A_DAY, || log.hand_over(today)).map_err(stored_err)?;
        let keys: Vec<Key> = items.iter().take(crate::kept::index(STORED_READS)?).map(|s| s.key).collect();
        let (mut reads, mut out) = (Reads::default(), Vec::new());
        let found = m.read(BASE, "find", STORED_READS, || {
            let mut frames = 0;
            for k in &keys {
                frames += log.find(*k, (today, today), &mut reads, &mut out)?;
            }
            Ok::<u64, StorageError>(frames)
        });
        let _ = std::hint::black_box((found.map_err(stored_err)?, out.len()));
        log.prune(today, &[]).map_err(stored_err)?;
        Ok(())
    }
}
