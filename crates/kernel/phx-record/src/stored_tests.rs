//! The stored log over an in-memory storage and a few hand-made items: a segment written and read back, written behind
//! and alike for any order of its items, its index rebuilt from its frames' headers, a tier's merge keeping the order
//! and dropping what is past the horizon, one frame read a segment, a frame past its bytes stopping the run, segments
//! deleted past the horizon unless a snapshot keeps them, and a day of no items writing nothing.
#![cfg(test)]

use std::collections::BTreeMap;
use std::panic::catch_unwind;

use phx_id::Day;

use super::{Item, Key, Reads, StoredDecl, StoredLog};
use crate::consts::FRAME_LEAST;
use crate::storage::{SegmentId, Storage, StorageError};

/// Segments held in memory, counting what was put and synced.
#[derive(Debug, Default)]
struct Mem {
    segs: BTreeMap<SegmentId, Vec<u8>>,
    writing: Option<Vec<u8>>,
    puts: u64,
    syncs: u64,
}

fn missing() -> StorageError {
    StorageError("no such segment".to_owned())
}

impl Storage for Mem {
    fn put(&mut self, id: SegmentId, bytes: &[u8]) -> Result<(), StorageError> {
        self.puts += 1;
        self.segs.insert(id, bytes.to_vec());
        Ok(())
    }

    fn read(&self, id: SegmentId, offset: u64, out: &mut [u8]) -> Result<(), StorageError> {
        let seg = self.segs.get(&id).ok_or_else(missing)?;
        let at = usize::try_from(offset).unwrap();
        out.copy_from_slice(seg.get(at..at + out.len()).ok_or_else(missing)?);
        Ok(())
    }

    fn read_rest(&self, id: SegmentId, offset: u64, out: &mut Vec<u8>) -> Result<(), StorageError> {
        let seg = self.segs.get(&id).ok_or_else(missing)?;
        out.extend_from_slice(seg.get(usize::try_from(offset).unwrap()..).ok_or_else(missing)?);
        Ok(())
    }

    fn remove(&mut self, id: SegmentId) -> Result<(), StorageError> {
        self.segs.remove(&id).map(|_| ()).ok_or_else(missing)
    }

    fn begin(&mut self) -> Result<(), StorageError> {
        self.writing = Some(Vec::new());
        Ok(())
    }

    fn append(&mut self, bytes: &[u8]) -> Result<(), StorageError> {
        self.writing.as_mut().ok_or_else(missing)?.extend_from_slice(bytes);
        Ok(())
    }

    fn commit(&mut self, id: SegmentId) -> Result<(), StorageError> {
        let bytes = self.writing.take().ok_or_else(missing)?;
        self.segs.insert(id, bytes);
        Ok(())
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        self.syncs += 1;
        Ok(())
    }
}

/// An event as a stored kind keeps it: its kind and first subject, its size and cause.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ev {
    key: Key,
    size: i64,
    cause: i64,
}

impl Item for Ev {
    const WORDS: usize = 2;

    fn key(&self) -> Key {
        self.key
    }

    fn words(&self, out: &mut [i64]) {
        out[0] = self.size;
        out[1] = self.cause;
    }

    fn of(key: Key, words: &[i64]) -> Ev {
        Ev { key, size: words[0], cause: words[1] }
    }
}

fn ev(kind: u64, subject: u64, size: i64) -> Ev {
    Ev { key: Key(kind, subject), size, cause: -size }
}

const DECL: StoredDecl = StoredDecl { name: "events", frame_bytes: FRAME_LEAST, horizon: 365, tiers: &[7, 28] };

fn log() -> StoredLog<Ev, Mem> {
    StoredLog::new(DECL, Mem::default())
}

fn find(log: &StoredLog<Ev, Mem>, key: Key, span: (u32, u32)) -> (Vec<(Day, Ev)>, u64) {
    let (mut out, mut reads) = (Vec::new(), Reads::default());
    let frames = log.find(key, (Day::new(span.0), Day::new(span.1)), &mut reads, &mut out).unwrap();
    (out, frames)
}

/// A day's items: many subjects of a few kinds, some subjects twice.
fn day_items(day: u64, n: u64) -> Vec<Ev> {
    (0..n).map(|i| ev(i % 3, (i * 7 + day) % (n / 2 + 1), i64::try_from(i + day).unwrap() - 40)).collect()
}

#[test]
fn stored_segment_round_trip() {
    let mut log = log();
    let items = day_items(5, 2_000);
    assert_eq!(log.flush(Day::new(5), &items).unwrap(), 2_000);
    for probe in items.iter().step_by(37) {
        let (found, _) = find(&log, probe.key, (0, 10));
        let mut want: Vec<Ev> = items.iter().filter(|e| e.key == probe.key).copied().collect();
        want.sort_by_key(|e| (e.size, e.cause));
        let mut got: Vec<Ev> = found
            .iter()
            .map(|(d, e)| {
                assert_eq!(*d, Day::new(5));
                *e
            })
            .collect();
        got.sort_by_key(|e| (e.size, e.cause));
        assert_eq!(got, want, "every item of {:?}", probe.key);
    }
    assert!(log.frames() > 1, "a day of many items cut into frames");
}

#[test]
fn flush_is_write_behind() {
    let mut log = log();
    log.flush(Day::new(1), &day_items(1, 50)).unwrap();
    assert_eq!((log.storage().puts, log.storage().syncs), (1, 0), "the day hands its segment over and never syncs");
    log.storage_mut().sync().unwrap();
    assert_eq!(log.storage().syncs, 1, "only a save syncs");
}

#[test]
fn flush_same_for_any_chunking() {
    let items = day_items(3, 400);
    let mut reversed = items.clone();
    reversed.reverse();
    let mut interleaved: Vec<Ev> = items.iter().step_by(2).chain(items.iter().skip(1).step_by(2)).copied().collect();
    interleaved.rotate_left(17);
    let id = |items: &[Ev]| {
        let mut l = log();
        l.flush(Day::new(3), items).unwrap();
        let seg = *l.segments().next().unwrap();
        (seg.id, l.storage().segs.get(&seg.id).cloned())
    };
    assert_eq!(id(&items), id(&reversed));
    assert_eq!(id(&items), id(&interleaved));
}

#[test]
fn index_rebuilt_from_headers() {
    let mut a = log();
    for day in 0..3 {
        a.flush(Day::new(day), &day_items(u64::from(day), 300)).unwrap();
    }
    let ids: Vec<SegmentId> = a.segments().map(|s| s.id).collect();
    let mem = std::mem::take(&mut a.storage);
    let b: StoredLog<Ev, Mem> = StoredLog::open(DECL, mem, &ids).unwrap();
    assert_eq!(b.segments().copied().collect::<Vec<_>>(), a.segments().copied().collect::<Vec<_>>());
    assert_eq!(b.frames, a.frames, "every frame's first key and offset read back");
}

#[test]
fn merge_keeps_order_and_drops_past_horizon() {
    let mut log = log();
    let mut all = Vec::new();
    for day in 0..9_u32 {
        let items = day_items(u64::from(day), 120);
        log.flush(Day::new(day), &items).unwrap();
        all.extend(items.into_iter().map(|e| (Day::new(day), e)));
    }
    // The first week closes on day 7: its seven day segments merge into one, a few entries a slice.
    let today = Day::new(9);
    let mut visited = 0;
    loop {
        let n = log.merge_slice(today, 50, |e| e.size == 7).unwrap();
        visited += n;
        if n == 0 || log.segments().any(|s| s.first == Day::new(0) && s.last == Day::new(6)) {
            break;
        }
    }
    assert_eq!(visited, 7 * 120, "every entry of the week visited");
    let spans: Vec<(u32, u32)> = log.segments().map(|s| (s.first.get(), s.last.get())).collect();
    assert_eq!(spans, vec![(0, 6), (7, 7), (8, 8)], "the week's segment in its days' place");
    for probe in [Key(0, 3), Key(1, 10), Key(2, 33)] {
        let (found, frames) = find(&log, probe, (0, 8));
        assert!(frames <= 3, "one frame a segment");
        let mut want: Vec<(Day, Ev)> =
            all.iter().filter(|(d, e)| e.key == probe && !(d.get() <= 6 && e.size == 7)).copied().collect();
        want.sort_by_key(|(d, e)| (d.get(), e.size));
        let mut got = found.clone();
        got.sort_by_key(|(d, e)| (d.get(), e.size));
        assert_eq!(got, want, "the merged week keeps every live entry with its day");
    }
    // An entry past the horizon is dropped by the merge that reads it.
    let short = StoredDecl { horizon: 2, tiers: &[7], ..DECL };
    let mut log: StoredLog<Ev, Mem> = StoredLog::new(short, Mem::default());
    for day in 0..7_u32 {
        log.flush(Day::new(day), &[ev(1, 1, i64::from(day))]).unwrap();
    }
    while log.merge_slice(Day::new(7), 3, |_| false).unwrap() > 0 {}
    let (found, _) = find(&log, Key(1, 1), (0, 6));
    let days: Vec<u32> = found.iter().map(|(d, _)| d.get()).collect();
    assert_eq!(days, vec![5, 6], "days 0–4 are past a horizon of two days on day 7");
}

#[test]
fn find_touches_one_frame_a_segment() {
    let mut log = log();
    for day in 0..4 {
        log.flush(Day::new(day), &day_items(u64::from(day), 2_000)).unwrap();
    }
    assert!(log.frames() > 8, "the days cut into many frames");
    let (found, frames) = find(&log, Key(2, 77), (0, 3));
    assert!(!found.is_empty());
    assert_eq!(frames, 4, "one frame read in each of the four segments");
}

#[test]
fn read_many_dedups_frames_one_round() {
    let mut log = log();
    log.flush(Day::new(0), &day_items(0, 2_000)).unwrap();
    let keys = [Key(0, 1), Key(0, 2), Key(0, 3)];
    let (mut out, mut reads) = (Vec::new(), Reads::default());
    let frames = log.read_many(&keys, (Day::new(0), Day::new(0)), &mut reads, &mut out).unwrap();
    assert_eq!(frames, 1, "three neighbouring keys lie in one frame, read once");
    for k in keys {
        let (one, _) = find(&log, k, (0, 0));
        assert_eq!(out.iter().filter(|(_, e)| e.key == k).count(), one.len());
    }
}

#[test]
fn frame_overflow_stops() {
    let wide = StoredDecl { frame_bytes: FRAME_LEAST - 1, ..DECL };
    assert!(catch_unwind(|| StoredLog::<Ev, Mem>::new(wide, Mem::default())).is_err(), "a frame below its bounds");
    // A run of one key longer than a frame goes on in the next, and is read whole.
    let run: Vec<Ev> = (0..2_000).map(|i| ev(4, 4, i)).collect();
    let mut log = log();
    log.flush(Day::new(0), &run).unwrap();
    let (found, frames) = find(&log, Key(4, 4), (0, 0));
    assert_eq!((found.len(), frames > 1), (2_000, true));
    // Frames written at a wider declaration read under a narrower one are past its bytes: the merge refuses them.
    let big = StoredDecl { frame_bytes: FRAME_LEAST * 4, tiers: &[7], ..DECL };
    let mut log: StoredLog<Ev, Mem> = StoredLog::new(big, Mem::default());
    for day in 0..7 {
        log.flush(Day::new(day), &day_items(u64::from(day), 2_000)).unwrap();
    }
    let ids: Vec<SegmentId> = log.segments().map(|s| s.id).collect();
    let narrow = StoredDecl { tiers: &[7], ..DECL };
    let mut log: StoredLog<Ev, Mem> = StoredLog::open(narrow, std::mem::take(&mut log.storage), &ids).unwrap();
    assert!(log.merge_slice(Day::new(7), 10, |_| false).is_err(), "a frame past its kind's bytes");
}

#[test]
fn segment_deleted_past_horizon() {
    let decl = StoredDecl { horizon: 3, tiers: &[], ..DECL };
    let mut log: StoredLog<Ev, Mem> = StoredLog::new(decl, Mem::default());
    let mut kept = None;
    for day in 0..12_u32 {
        log.flush(Day::new(day), &day_items(u64::from(day), 20)).unwrap();
        if day == 0 {
            kept = log.segments().next().map(|s| s.id);
        }
        let retained: Vec<SegmentId> = kept.into_iter().collect();
        log.prune(Day::new(day), &retained).unwrap();
        assert!(log.segments().count() <= 4, "the horizon's days and today's");
    }
    assert_eq!(log.storage().segs.len(), 5, "storage flat at the horizon, and the retained snapshot's segment");
    log.prune(Day::new(12), &[]).unwrap();
    assert_eq!(log.storage().segs.len(), 3, "released once no snapshot keeps it");
}

#[test]
fn no_items_no_segment() {
    let mut log = log();
    assert_eq!(log.flush(Day::new(0), &[]).unwrap(), 0);
    assert_eq!((log.segments().count(), log.storage().puts), (0, 0));
}
