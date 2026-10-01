//! The stored log: a history too large to keep resident, written each day as one immutable segment of encoded frames
//! named by its content, found through a small resident index of each frame's first key, merged in tiers when its kind
//! asks, and deleted past its horizon once no retained snapshot lists it.
//!
//! A segment is its header (magic, version, first and last day) and its frames. A frame is its header (its entries'
//! bytes, its entries, its first key) and its entries in the log's order — key, day, words — each entry its key
//! delta-coded from the one before in the frame, its day from the segment's first and its words zigzagged, every
//! number a varint. A key's entries lie in the frame the index finds for it, and in the frames after it that begin with
//! it when a run of the key fills one.

use std::cmp::Ordering;
use std::marker::PhantomData;

use phx_id::Day;
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::Sip128;

use crate::consts::{
    ADDRESS_KEY, ENTRY_MOST, FRAME_HEADER, FRAME_LEAST, FRAME_MOST, ITEM_WORDS, SEGMENT_HEADER, SEGMENT_MAGIC,
    SEGMENT_VERSION, VARINT_BITS, VARINT_MORE, VARINT_MOST,
};
use crate::storage::{SegmentId, Storage, StorageError};

/// The order a stored kind keeps its items in: two words, the first major.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Key(pub u64, pub u64);

/// An item a stored kind keeps: its key and its words besides, each a signed number.
pub trait Item: Copy {
    /// How many words it holds besides its key, at most `ITEM_WORDS`.
    const WORDS: usize;
    fn key(&self) -> Key;
    /// Its words, written into the first `WORDS` of `out`.
    fn words(&self, out: &mut [i64]);
    /// The item its key and words make.
    fn of(key: Key, words: &[i64]) -> Self;
}

/// A stored kind as it declares itself: its name, the bytes its frames hold, its horizon in days, and the spans in
/// days its tiers merge into, each a multiple of the one before; none for a kind that keeps its day segments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredDecl {
    pub name: &'static str,
    pub frame_bytes: u32,
    pub horizon: u32,
    pub tiers: &'static [u32],
}

/// A frame as the resident index finds it: its first key and where it begins in its segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrameEntry {
    first: Key,
    offset: u64,
}

/// A segment standing in the log: its address, the days it covers, its frames among the index's, and its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Segment {
    pub id: SegmentId,
    pub first: Day,
    pub last: Day,
    frames: (u32, u32),
    bytes: u64,
}

/// An entry as the log orders it: its key, its day and its item's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entry {
    key: Key,
    day: u32,
    words: [i64; ITEM_WORDS],
}

impl Entry {
    fn of<T: Item>(item: &T, day: u32) -> Entry {
        let mut words = [0; ITEM_WORDS];
        item.words(&mut words);
        Entry { key: item.key(), day, words }
    }

    fn cmp(&self, o: &Entry) -> Ordering {
        (self.key, self.day, self.words).cmp(&(o.key, o.day, o.words))
    }
}

fn zigzag(v: i64) -> u64 {
    u64::from_le_bytes(((v << 1) ^ (v >> (i64::BITS - 1))).to_le_bytes())
}

fn unzigzag(v: u64) -> i64 {
    i64::from_le_bytes((v >> 1).to_le_bytes()) ^ -i64::from_le_bytes((v & 1).to_le_bytes())
}

fn put_varint(out: &mut [u8; ENTRY_MOST], at: &mut usize, mut v: u64) {
    // An entry's bytes are sized for its most varints, so a place past them is past every entry's.
    while v >= u64::from(VARINT_MORE) {
        let [low, ..] = v.to_le_bytes();
        if let Some(b) = out.get_mut(*at) {
            *b = low | VARINT_MORE;
        }
        *at += 1;
        v >>= VARINT_BITS;
    }
    let [low, ..] = v.to_le_bytes();
    if let Some(b) = out.get_mut(*at) {
        *b = low;
    }
    *at += 1;
}

fn get_varint(input: &[u8], at: &mut usize) -> Option<u64> {
    let mut v = 0_u64;
    for i in 0..VARINT_MOST {
        let byte = *input.get(*at)?;
        *at += 1;
        let shift = VARINT_BITS * u32::try_from(i).ok()?;
        v |= u64::from(byte & !VARINT_MORE).checked_shl(shift)?;
        if byte & VARINT_MORE == 0 {
            return Some(v);
        }
    }
    None
}

/// An entry's bytes after the one before it in its frame: its key, its day and its words.
fn encode(before: Key, (key, day, words): (Key, u32, &[i64]), out: &mut [u8; ENTRY_MOST]) -> usize {
    let mut at = 0;
    let Some(major) = key.0.checked_sub(before.0) else {
        violation!(clause = "SET.13", "a stored entry out of its key's order", key = key.0);
    };
    put_varint(out, &mut at, major);
    let minor = if major == 0 { key.1.checked_sub(before.1) } else { Some(key.1) };
    let Some(minor) = minor else { violation!(clause = "SET.13", "a stored entry out of its key's order") };
    put_varint(out, &mut at, minor);
    put_varint(out, &mut at, u64::from(day));
    for w in words {
        put_varint(out, &mut at, zigzag(*w));
    }
    at
}

/// The words an item of a kind holds, of an entry's.
fn first_words<T: Item>(words: &[i64; ITEM_WORDS]) -> &[i64] {
    match words.get(..T::WORDS) {
        Some(w) => w,
        None => violation!(clause = "SET.13", "a stored item of more words than an entry holds"),
    }
}

/// The bytes an entry's encoding wrote.
fn written(bytes: &[u8; ENTRY_MOST], len: usize) -> &[u8] {
    match bytes.get(..len) {
        Some(b) => b,
        None => violation!(clause = "SET.13", "an entry past its most bytes"),
    }
}

fn decode(input: &[u8], at: &mut usize, prev: Key, n: usize) -> Option<Entry> {
    let major = get_varint(input, at)?;
    let minor = get_varint(input, at)?;
    let key = if major == 0 { Key(prev.0, prev.1.checked_add(minor)?) } else { Key(prev.0.checked_add(major)?, minor) };
    let day = u32::try_from(get_varint(input, at)?).ok()?;
    let mut words = [0; ITEM_WORDS];
    for w in words.iter_mut().take(n) {
        *w = unzigzag(get_varint(input, at)?);
    }
    Some(Entry { key, day, words })
}

fn read_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + size_of::<u32>())?.try_into().ok()?))
}

fn read_u64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + size_of::<u64>())?.try_into().ok()?))
}

/// A frame's header read: its entries' bytes, its entries and its first key.
fn frame_header(b: &[u8]) -> Option<(u32, u32, Key)> {
    let len = read_u32(b, 0)?;
    let count = read_u32(b, size_of::<u32>())?;
    let k0 = read_u64(b, 2 * size_of::<u32>())?;
    let k1 = read_u64(b, 2 * size_of::<u32>() + size_of::<u64>())?;
    Some((len, count, Key(k0, k1)))
}

fn segment_header(first: Day, last: Day, out: &mut Vec<u8>) {
    out.extend_from_slice(&SEGMENT_MAGIC.to_le_bytes());
    out.extend_from_slice(&SEGMENT_VERSION.to_le_bytes());
    out.extend_from_slice(&first.get().to_le_bytes());
    out.extend_from_slice(&last.get().to_le_bytes());
}

fn widen(n: usize) -> u64 {
    u64::try_from(n).unwrap_or_else(|_| capacity_exceeded!("a stored segment's bytes", u64::MAX, n))
}

fn narrow(n: usize) -> u32 {
    u32::try_from(n).unwrap_or_else(|_| capacity_exceeded!("a stored log's frames", u32::MAX, n))
}

fn at_usize(n: u64) -> usize {
    usize::try_from(n).unwrap_or_else(|_| capacity_exceeded!("a stored segment's bytes", usize::MAX, n))
}

/// The frame being written: where its header lies in the buffer, its first key, the key before the next entry, its
/// entries and the key of its last, so a run of one key is never split.
#[derive(Clone, Copy, Debug)]
struct Open {
    at: usize,
    first: Key,
    before: Key,
    count: u32,
}

/// Entries cut into frames of at most a kind's bytes, written to a buffer whose first byte lies at `base` in its
/// segment, each frame's first key and offset added to an index.
#[derive(Debug)]
struct Framer {
    limit: usize,
    open: Option<Open>,
}

impl Framer {
    fn new(limit: u32) -> Framer {
        Framer { limit: at_usize(u64::from(limit)), open: None }
    }

    fn push(&mut self, (buf, base): (&mut Vec<u8>, u64), index: &mut Vec<FrameEntry>, e: (Key, u32, &[i64])) {
        let mut bytes = [0; ENTRY_MOST];
        if let Some(o) = self.open {
            let len = encode(o.before, e, &mut bytes);
            if buf.len() - o.at + len <= self.limit {
                buf.extend_from_slice(written(&bytes, len));
                self.open = Some(Open { before: e.0, count: o.count + 1, ..o });
                return;
            }
            self.close(buf);
        }
        let len = encode(Key::default(), e, &mut bytes);
        if FRAME_HEADER + len > self.limit {
            violation!(clause = "SET.13", "an entry past its frame", key = e.0.0);
        }
        let at = buf.len();
        buf.extend_from_slice(&[0; FRAME_HEADER]);
        buf.extend_from_slice(written(&bytes, len));
        index.push(FrameEntry { first: e.0, offset: base + widen(at) });
        self.open = Some(Open { at, first: e.0, before: e.0, count: 1 });
    }

    /// The open frame's header written.
    fn close(&mut self, buf: &mut [u8]) {
        let Some(o) = self.open.take() else { return };
        let len = narrow(buf.len() - o.at - FRAME_HEADER);
        let mut header = [0; FRAME_HEADER];
        for (dst, src) in header.iter_mut().zip(
            len.to_le_bytes()
                .into_iter()
                .chain(o.count.to_le_bytes())
                .chain(o.first.0.to_le_bytes())
                .chain(o.first.1.to_le_bytes()),
        ) {
            *dst = src;
        }
        match buf.get_mut(o.at..o.at + FRAME_HEADER) {
            Some(h) => h.copy_from_slice(&header),
            None => violation!(clause = "SET.13", "a frame's header past its buffer"),
        }
    }

    /// The bytes before the open frame, which no later entry changes.
    fn settled(&self, buf: &[u8]) -> usize {
        self.open.map_or(buf.len(), |o| o.at)
    }

    /// The settled bytes taken from the buffer's front, the open frame moved to its start.
    fn drop_settled(&mut self, buf: &mut Vec<u8>) -> usize {
        let n = self.settled(buf);
        buf.drain(..n);
        if let Some(o) = self.open.as_mut() {
            o.at -= n;
        }
        n
    }
}

/// One input of a merge: its segment, its first day and bytes, its frames among the merge's, the next of them, its open
/// frame's place and bytes in the merge's buffer, the cursor in them, the key before it, the entries left, and the
/// entry it heads with.
#[derive(Clone, Copy, Debug)]
struct Input {
    id: SegmentId,
    first: Day,
    bytes: u64,
    frames: (u32, u32),
    frame: u32,
    region: (usize, usize),
    cursor: usize,
    before: Key,
    left: u32,
    head: Option<Entry>,
}

/// A merge, under way or idle with its buffers kept: the period it fills, its inputs and their frames, their open
/// frames' bytes, the merged segment's frames made, its bytes not yet written, the bytes written and hashed, and its
/// frame cutter.
#[derive(Debug)]
struct Merge {
    active: bool,
    first: Day,
    last: Day,
    inputs: Vec<Input>,
    frames: Vec<FrameEntry>,
    read: Vec<u8>,
    made: Vec<FrameEntry>,
    out: Vec<u8>,
    written: u64,
    hash: Sip128,
    framer: Framer,
}

impl Merge {
    /// No merge under way, its buffers empty until the first.
    #[phx_macros::opening]
    fn idle(frame_bytes: u32) -> Merge {
        Merge {
            active: false,
            first: Day::new(0),
            last: Day::new(0),
            inputs: Vec::new(),
            frames: Vec::new(),
            read: Vec::new(),
            made: Vec::new(),
            out: Vec::new(),
            written: 0,
            hash: Sip128::new(ADDRESS_KEY),
            framer: Framer::new(frame_bytes),
        }
    }
}

/// What a read of the log keeps between its frames: the bytes read and the frames wanted.
#[derive(Debug, Default)]
pub struct Reads {
    bytes: Vec<u8>,
    wanted: Vec<(u32, u32)>,
}

/// A stored kind's log: its declaration, its storage, its standing segments in order of their first day with their
/// frames' index, the segments out of it that a snapshot may still list, the day's segment's buffer, and a merge
/// under way.
#[clause("SET.13")]
#[derive(Debug)]
pub struct StoredLog<T: Item, S: Storage> {
    decl: StoredDecl,
    storage: S,
    segments: Vec<Segment>,
    frames: Vec<FrameEntry>,
    released: Vec<SegmentId>,
    buf: Vec<u8>,
    keys: Vec<(u128, u32)>,
    scratch: Vec<(u128, u32)>,
    narrow: Vec<(u64, u32)>,
    narrow_scratch: Vec<(u64, u32)>,
    sorted: Vec<T>,
    staged_frames: u32,
    merge: Merge,
    item: PhantomData<T>,
}

/// A segment's address: its bytes' hash.
fn address(bytes: &[u8]) -> SegmentId {
    let mut h = Sip128::new(ADDRESS_KEY);
    h.write(bytes);
    h.finish()
}

#[cold]
fn damaged(what: &str) -> StorageError {
    StorageError(what.to_owned())
}

impl<T: Item, S: Storage> StoredLog<T, S> {
    /// A kind's log on its storage, holding no segment. A frame outside the declared bounds, or items of more words
    /// than an entry holds, stop the run.
    #[phx_macros::opening]
    pub fn new(decl: StoredDecl, storage: S) -> StoredLog<T, S> {
        if !(FRAME_LEAST..=FRAME_MOST).contains(&decl.frame_bytes) {
            violation!(clause = "SET.13", "a stored kind's frames outside their bounds", bytes = decl.frame_bytes);
        }
        if T::WORDS > ITEM_WORDS {
            violation!(clause = "SET.13", "a stored item of more words than an entry holds");
        }
        StoredLog {
            decl,
            storage,
            segments: Vec::new(),
            frames: Vec::new(),
            released: Vec::new(),
            buf: Vec::new(),
            keys: Vec::new(),
            scratch: Vec::new(),
            narrow: Vec::new(),
            narrow_scratch: Vec::new(),
            sorted: Vec::new(),
            staged_frames: 0,
            merge: Merge::idle(decl.frame_bytes),
            item: PhantomData,
        }
    }

    /// A kind's log on its storage holding the segments a snapshot lists, its index rebuilt from their frames'
    /// headers.
    ///
    /// # Errors
    /// A listed segment the storage does not hold, or one damaged.
    #[phx_macros::opening]
    pub fn open(decl: StoredDecl, storage: S, ids: &[SegmentId]) -> Result<StoredLog<T, S>, StorageError> {
        let mut log = StoredLog::new(decl, storage);
        let mut bytes = Vec::new();
        for id in ids {
            bytes.clear();
            log.storage.read_rest(*id, 0, &mut bytes)?;
            let (version_at, first_at) = (size_of::<u32>(), size_of::<u32>() + size_of::<u16>());
            let version = bytes.get(version_at..first_at).and_then(|b| b.try_into().ok()).map(u16::from_le_bytes);
            if read_u32(&bytes, 0) != Some(SEGMENT_MAGIC) || version != Some(SEGMENT_VERSION) {
                return Err(damaged("a segment of another format"));
            }
            let (Some(first), Some(last)) = (read_u32(&bytes, first_at), read_u32(&bytes, first_at + size_of::<u32>()))
            else {
                return Err(damaged("a segment's header cut short"));
            };
            let start = narrow(log.frames.len());
            let mut at = SEGMENT_HEADER;
            while at < bytes.len() {
                let Some((len, _, key)) = bytes.get(at..).and_then(frame_header) else {
                    return Err(damaged("a frame's header cut short"));
                };
                log.frames.push(FrameEntry { first: key, offset: widen(at) });
                at += FRAME_HEADER + at_usize(u64::from(len));
            }
            let seg = Segment {
                id: *id,
                first: Day::new(first),
                last: Day::new(last),
                frames: (start, narrow(log.frames.len()) - start),
                bytes: widen(bytes.len()),
            };
            let place = log.segments.partition_point(|s| s.first <= seg.first);
            log.segments.insert(place, seg);
        }
        Ok(log)
    }

    /// The segments standing, in order of their first day, which a snapshot lists.
    pub fn segments(&self) -> impl Iterator<Item = &Segment> + '_ {
        self.segments.iter()
    }

    /// The storage the segments are on.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    /// The day's items written as one segment, sorted by key and then by their words, so its bytes are the same for
    /// any order they were handed in; none written for a day of none. Its bytes are handed to storage and not waited
    /// for. The items written.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn flush(&mut self, day: Day, items: &[T]) -> Result<u64, StorageError> {
        self.stage(items);
        self.flush_staged(day)
    }

    /// The day's items put in the log's order: by key, then by their words, an order no chunking changes. The keys
    /// are radix-sorted with each item's place; a run of one key is ordered by its words.
    pub fn stage(&mut self, items: &[T]) {
        if u32::try_from(items.len()).is_err() {
            capacity_exceeded!("a stored day's items", u32::MAX, items.len());
        }
        // Keys narrow enough are packed into one word, minor bits below major, so the sort passes over half the bits.
        let (mut major_bits, mut minor_bits) = (0_u64, 0_u64);
        for i in items {
            let Key(major, minor) = i.key();
            (major_bits, minor_bits) = (major_bits | major, minor_bits | minor);
        }
        let shift = u64::BITS - minor_bits.leading_zeros();
        let narrow_keys = u64::BITS - major_bits.leading_zeros() + shift <= u64::BITS;
        self.keys.clear();
        self.narrow.clear();
        for (item, at) in items.iter().zip(0_u32..) {
            let Key(major, minor) = item.key();
            if narrow_keys {
                let Ok(packed) = u64::try_from((u128::from(major) << shift) | u128::from(minor)) else {
                    violation!(clause = "SET.13", "a key packed past its word", key = major);
                };
                self.narrow.push((packed, at));
            } else {
                self.keys.push(((u128::from(major) << u64::BITS) | u128::from(minor), at));
            }
        }
        self.sorted.clear();
        if narrow_keys {
            phx_exec::radix_sort(None, &mut self.narrow, &mut self.narrow_scratch);
            gather(&self.narrow, items, &mut self.sorted);
        } else {
            phx_exec::radix_sort(None, &mut self.keys, &mut self.scratch);
            gather(&self.keys, items, &mut self.sorted);
        }
    }

    /// The staged items encoded into the day's segment and handed to storage. The items written.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn flush_staged(&mut self, day: Day) -> Result<u64, StorageError> {
        let written = self.encode_staged(day);
        self.hand_over(day)?;
        Ok(written)
    }

    /// The staged items cut into the day's segment's frames, their first keys in the index. The items encoded.
    pub fn encode_staged(&mut self, day: Day) -> u64 {
        self.buf.clear();
        self.staged_frames = narrow(self.frames.len());
        if self.sorted.is_empty() {
            return 0;
        }
        segment_header(day, day, &mut self.buf);
        let mut cutter = Framer::new(self.decl.frame_bytes);
        let mut words = [0; ITEM_WORDS];
        for item in &self.sorted {
            item.words(&mut words);
            cutter.push((&mut self.buf, 0), &mut self.frames, (item.key(), 0, first_words::<T>(&words)));
        }
        cutter.close(&mut self.buf);
        let encoded = widen(self.sorted.len());
        self.sorted.clear();
        encoded
    }

    /// The encoded segment named by its content and handed to storage, not waited for, and placed in the index; none
    /// for a day of none.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn hand_over(&mut self, day: Day) -> Result<(), StorageError> {
        if self.buf.is_empty() {
            return Ok(());
        }
        let id = address(&self.buf);
        self.storage.put(id, &self.buf)?;
        let start = self.staged_frames;
        let frames = (start, narrow(self.frames.len()) - start);
        let seg = Segment { id, first: day, last: day, frames, bytes: widen(self.buf.len()) };
        let place = self.segments.partition_point(|s| s.first <= day);
        self.segments.insert(place, seg);
        self.buf.clear();
        Ok(())
    }

    /// The first frame of a segment that may hold a key: the last that begins before it, or the first when none does.
    fn frame_of(&self, seg: &Segment, key: Key) -> u32 {
        let (start, n) = seg.frames;
        let frames = self.frames.get(at_usize(u64::from(start))..at_usize(u64::from(start + n))).unwrap_or(&[]);
        let i = frames.partition_point(|f| f.first < key);
        start + narrow(i.checked_sub(1).unwrap_or(i))
    }

    /// Whether a frame of a segment begins at or before a key, so may hold it.
    fn may_hold(&self, seg: &Segment, frame: u32, key: Key) -> bool {
        frame < seg.frames.0 + seg.frames.1
            && self.frames.get(at_usize(u64::from(frame))).is_some_and(|f| f.first <= key)
    }

    /// A frame's bytes, header and entries, read into `out` from its segment: its entries and its segment's first day.
    fn read_frame(&self, seg: &Segment, frame: u32, out: &mut Vec<u8>) -> Result<(u32, Day), StorageError> {
        let Some((offset, len)) = span(&self.frames, (seg.frames, seg.bytes), frame) else {
            return Err(damaged("a frame past the index"));
        };
        out.resize(len, 0);
        self.storage.read(seg.id, offset, out)?;
        match frame_header(out) {
            Some((_, count, _)) => Ok((count, seg.first)),
            None => Err(damaged("a frame's header cut short")),
        }
    }

    /// Each of a frame's entries, `count` of them as its header says, with its day.
    fn entries(bytes: &[u8], (count, first): (u32, Day)) -> impl Iterator<Item = Entry> + '_ {
        let mut at = FRAME_HEADER;
        let mut before = Key::default();
        (0..count).map_while(move |_| {
            let e = decode(bytes, &mut at, before, T::WORDS)?;
            before = e.key;
            Some(Entry { day: Day::unpacked(first, e.day).get(), ..e })
        })
    }

    /// Every item of a key over a span of days, with its day, appended to `out`: in each segment over the span the
    /// one frame the index says may hold it is read. The frames read.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn find(
        &self,
        key: Key,
        (from, to): (Day, Day),
        reads: &mut Reads,
        out: &mut Vec<(Day, T)>,
    ) -> Result<u64, StorageError> {
        let mut read = 0;
        for seg in self.segments.iter().filter(|s| s.last >= from && s.first <= to) {
            let mut frame = self.frame_of(seg, key);
            while self.may_hold(seg, frame, key) {
                let held = self.read_frame(seg, frame, &mut reads.bytes)?;
                read += 1;
                let mut past = false;
                for e in Self::entries(&reads.bytes, held).skip_while(|e| e.key < key) {
                    if e.key != key {
                        past = true;
                        break;
                    }
                    if (from.get()..=to.get()).contains(&e.day) {
                        out.push((Day::new(e.day), T::of(e.key, &e.words)));
                    }
                }
                if past {
                    break;
                }
                frame += 1;
            }
        }
        Ok(read)
    }

    /// Every item of any of `keys`, sorted, over a span of days, appended to `out`: the frames that may hold them
    /// gathered, each read once, in order of their segment and offset. The frames read.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn read_many(
        &self,
        keys: &[Key],
        (from, to): (Day, Day),
        reads: &mut Reads,
        out: &mut Vec<(Day, T)>,
    ) -> Result<u64, StorageError> {
        reads.wanted.clear();
        for (s, seg) in (0_u32..).zip(&self.segments).filter(|(_, s)| s.last >= from && s.first <= to) {
            for k in keys {
                let mut frame = self.frame_of(seg, *k);
                reads.wanted.push((s, frame));
                // A run of the key that fills its frame goes on in the frames that begin with it.
                while self.frames.get(at_usize(u64::from(frame + 1))).is_some_and(|f| f.first == *k)
                    && frame + 1 < seg.frames.0 + seg.frames.1
                {
                    frame += 1;
                    reads.wanted.push((s, frame));
                }
            }
        }
        reads.wanted.sort_unstable();
        reads.wanted.dedup();
        for i in 0..reads.wanted.len() {
            let Some(&(s, frame)) = reads.wanted.get(i) else { break };
            let Some(seg) = self.segments.get(at_usize(u64::from(s))) else { continue };
            let held = self.read_frame(seg, frame, &mut reads.bytes)?;
            for e in Self::entries(&reads.bytes, held) {
                if keys.binary_search(&e.key).is_ok() && (from.get()..=to.get()).contains(&e.day) {
                    out.push((Day::new(e.day), T::of(e.key, &e.words)));
                }
            }
        }
        Ok(widen(reads.wanted.len()))
    }

    /// The segments wholly past the horizon taken out of the index, and every segment out of it deleted once no
    /// retained snapshot lists it and no merge reads it. The segments deleted.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn prune(&mut self, today: Day, retained: &[SegmentId]) -> Result<u64, StorageError> {
        let horizon = self.decl.horizon;
        let merge = &self.merge;
        let reading = |id: SegmentId| merge.active && merge.inputs.iter().any(|x| x.id == id);
        let released = &mut self.released;
        compact((&mut self.segments, &mut self.frames), |s| {
            let past = s.last.get().checked_add(horizon).is_some_and(|edge| edge < today.get()) && !reading(s.id);
            if past {
                released.push(s.id);
            }
            !past
        });
        let (mut deleted, mut kept) = (0, 0);
        for i in 0..released.len() {
            let Some(id) = released.get(i).copied() else { break };
            if retained.contains(&id) || reading(id) {
                if let Some(slot) = released.get_mut(kept) {
                    *slot = id;
                }
                kept += 1;
            } else {
                self.storage.remove(id)?;
                deleted += 1;
            }
        }
        released.truncate(kept);
        Ok(deleted)
    }

    /// The next merge's period: for the lowest tier, the oldest of its periods closed by today holding two or more
    /// segments of a shorter span within it.
    fn next_group(&self, today: Day) -> Option<(Day, Day)> {
        for span in self.decl.tiers {
            let mut best: Option<u32> = None;
            let mut run: Option<(u32, u32)> = None;
            for s in &self.segments {
                let (first, last) = (s.first.get(), s.last.get());
                let period = first / span;
                let within = last / span == period && last - first + 1 < *span;
                let closed = (period + 1).checked_mul(*span).is_some_and(|end| end <= today.get());
                if !(within && closed) {
                    continue;
                }
                run = match run {
                    Some((p, n)) if p == period => Some((p, n + 1)),
                    _ => Some((period, 1)),
                };
                if let Some((p, n)) = run
                    && n >= 2
                    && best.is_none_or(|b| p < b)
                {
                    best = Some(p);
                }
            }
            if let Some(p) = best {
                // A period's days counted by the tier's span: its first and its last.
                let (first, last) = (p * span, (p + 1) * span - 1);
                return Some((Day::new(first), Day::new(last)));
            }
        }
        None
    }

    /// A merge begun over the segments within a period, each input's first entry read.
    fn begin_merge(&mut self, (first, last): (Day, Day)) -> Result<(), StorageError> {
        let limit = at_usize(u64::from(self.decl.frame_bytes));
        let m = &mut self.merge;
        m.inputs.clear();
        m.frames.clear();
        m.made.clear();
        m.out.clear();
        (m.active, m.first, m.last, m.written) = (true, first, last, 0);
        m.hash = Sip128::new(ADDRESS_KEY);
        m.framer = Framer::new(self.decl.frame_bytes);
        for seg in self.segments.iter().filter(|s| s.first >= first && s.last <= last) {
            let (start, n) = seg.frames;
            let mine = narrow(m.frames.len());
            if let Some(f) = self.frames.get(at_usize(u64::from(start))..at_usize(u64::from(start + n))) {
                m.frames.extend_from_slice(f);
            }
            m.inputs.push(Input {
                id: seg.id,
                first: seg.first,
                bytes: seg.bytes,
                frames: (mine, n),
                frame: 0,
                region: (m.inputs.len() * limit, 0),
                cursor: 0,
                before: Key::default(),
                left: 0,
                head: None,
            });
        }
        m.read.resize(m.inputs.len() * limit, 0);
        segment_header(first, last, &mut m.out);
        self.storage.begin()?;
        for i in 0..m.inputs.len() {
            advance(&self.storage, m, i, T::WORDS)?;
        }
        Ok(())
    }

    /// A bounded slice of the next merge: up to `budget` entries taken from its inputs in the log's order, those past
    /// the horizon or that `dead` refuses dropped, the rest written to the merged segment, which takes its inputs'
    /// place in the index once they are spent. A merge is begun when none is under way and a tier's period is ready.
    /// The entries visited.
    ///
    /// # Errors
    /// The storage's refusal.
    pub fn merge_slice(&mut self, today: Day, budget: u64, dead: impl Fn(&T) -> bool) -> Result<u64, StorageError> {
        if !self.merge.active {
            let Some(period) = self.next_group(today) else { return Ok(0) };
            self.begin_merge(period)?;
        }
        let limit = at_usize(u64::from(self.decl.frame_bytes));
        let horizon = self.decl.horizon;
        let m = &mut self.merge;
        let mut visited = 0;
        while visited < budget {
            let mut least: Option<(usize, Entry)> = None;
            for (i, input) in m.inputs.iter().enumerate() {
                if let Some(h) = input.head
                    && least.is_none_or(|(_, l)| h.cmp(&l) == Ordering::Less)
                {
                    least = Some((i, h));
                }
            }
            let Some((i, e)) = least else {
                self.finish_merge()?;
                return Ok(visited);
            };
            visited += 1;
            advance(&self.storage, m, i, T::WORDS)?;
            let past = e.day.checked_add(horizon).is_some_and(|edge| edge < today.get());
            if past || dead(&T::of(e.key, &e.words)) {
                continue;
            }
            let Some(day) = Day::new(e.day).since(m.first) else {
                violation!(clause = "SET.13", "a merged entry before its period", day = e.day);
            };
            m.framer.push((&mut m.out, m.written), &mut m.made, (e.key, day, first_words::<T>(&e.words)));
            let settled = m.framer.settled(&m.out);
            if settled >= limit {
                let Some(bytes) = m.out.get(..settled) else {
                    violation!(clause = "SET.13", "a merge's settled bytes past its buffer");
                };
                self.storage.append(bytes)?;
                m.hash.write(bytes);
                m.written += widen(m.framer.drop_settled(&mut m.out));
            }
        }
        Ok(visited)
    }

    /// A spent merge's segment written out and named, its inputs out of the index and released, and it in their place.
    fn finish_merge(&mut self) -> Result<(), StorageError> {
        let m = &mut self.merge;
        m.framer.close(&mut m.out);
        self.storage.append(&m.out)?;
        m.hash.write(&m.out);
        let bytes = m.written + widen(m.out.len());
        let id = std::mem::replace(&mut m.hash, Sip128::new(ADDRESS_KEY)).finish();
        self.storage.commit(id)?;
        let (inputs, released) = (&m.inputs, &mut self.released);
        compact((&mut self.segments, &mut self.frames), |s| {
            let input = inputs.iter().any(|x| x.id == s.id);
            if input {
                released.push(s.id);
            }
            !input
        });
        let start = narrow(self.frames.len());
        self.frames.extend_from_slice(&m.made);
        let seg = Segment { id, first: m.first, last: m.last, frames: (start, narrow(m.made.len())), bytes };
        let place = self.segments.partition_point(|s| s.first <= m.first);
        self.segments.insert(place, seg);
        m.active = false;
        Ok(())
    }

    /// The bytes of the resident index: its segments' and its frames' entries.
    #[must_use]
    pub fn index_bytes(&self) -> u64 {
        widen(self.segments.len() * size_of::<Segment>() + self.frames.len() * size_of::<FrameEntry>())
    }

    /// The frames the index holds.
    #[must_use]
    pub fn frames(&self) -> u64 {
        widen(self.frames.len())
    }

    /// The bytes the standing segments hold on storage.
    #[must_use]
    pub fn stored_bytes(&self) -> u64 {
        self.segments.iter().map(|s| s.bytes).sum()
    }
}

/// Items gathered in the order their sorted keys give, each run of one key then ordered by its words.
fn gather<K: Copy + PartialEq, T: Item>(sorted: &[(K, u32)], items: &[T], out: &mut Vec<T>) {
    out.extend(sorted.iter().filter_map(|(_, at)| items.get(at_usize(u64::from(*at))).copied()));
    let mut start = 0;
    while let Some((key, _)) = sorted.get(start) {
        let len = sorted.iter().skip(start).take_while(|(k, _)| k == key).count();
        if len > 1
            && let Some(run) = out.get_mut(start..start + len)
        {
            run.sort_unstable_by(|a, b| Entry::of(a, 0).cmp(&Entry::of(b, 0)));
        }
        start += len;
    }
}

/// The segments `keep` refuses taken out of an index, the frames of those it keeps closed up in order.
fn compact((segments, frames): (&mut Vec<Segment>, &mut Vec<FrameEntry>), mut keep: impl FnMut(&Segment) -> bool) {
    let (mut seg_to, mut frame_to) = (0, 0_u32);
    for i in 0..segments.len() {
        let Some(seg) = segments.get(i).copied() else { break };
        if !keep(&seg) {
            continue;
        }
        let (start, n) = seg.frames;
        let (from, to) = (at_usize(u64::from(start)), at_usize(u64::from(frame_to)));
        frames.copy_within(from..from + at_usize(u64::from(n)), to);
        if let Some(s) = segments.get_mut(seg_to) {
            *s = Segment { frames: (frame_to, n), ..seg };
        }
        seg_to += 1;
        frame_to += n;
    }
    segments.truncate(seg_to);
    frames.truncate(at_usize(u64::from(frame_to)));
}

/// A merge input's next entry made its head, its next frame read when its open one is spent; none when it has none.
fn advance<S: Storage>(storage: &S, m: &mut Merge, i: usize, words: usize) -> Result<(), StorageError> {
    let Some(mut input) = m.inputs.get(i).copied() else { return Ok(()) };
    if input.left == 0 {
        if input.frame >= input.frames.1 {
            input.head = None;
            if let Some(x) = m.inputs.get_mut(i) {
                *x = input;
            }
            return Ok(());
        }
        let Some((offset, len)) = span(&m.frames, (input.frames, input.bytes), input.frames.0 + input.frame) else {
            return Err(damaged("a merge's frame past its input"));
        };
        let Some(dst) = m.read.get_mut(input.region.0..input.region.0 + len).filter(|_| len <= m.framer.limit) else {
            return Err(damaged("a frame past its kind's bytes"));
        };
        storage.read(input.id, offset, dst)?;
        let Some((_, count, _)) = frame_header(dst) else { return Err(damaged("a merge's frame cut short")) };
        input.frame += 1;
        (input.region.1, input.cursor, input.before, input.left) = (len, FRAME_HEADER, Key::default(), count);
    }
    let Some(bytes) = m.read.get(input.region.0..input.region.0 + input.region.1) else {
        return Err(damaged("a merge's frame past its buffer"));
    };
    let Some(e) = decode(bytes, &mut input.cursor, input.before, words) else {
        return Err(damaged("a merge's entry cut short"));
    };
    input.before = e.key;
    input.left -= 1;
    input.head = Some(Entry { day: Day::unpacked(input.first, e.day).get(), ..e });
    if let Some(x) = m.inputs.get_mut(i) {
        *x = input;
    }
    Ok(())
}

/// Where a frame of a segment lies: its offset and its bytes, header and entries, up to the next frame or the
/// segment's end.
fn span(frames: &[FrameEntry], (seg_frames, bytes): ((u32, u32), u64), frame: u32) -> Option<(u64, usize)> {
    let at = |i: u32| frames.get(at_usize(u64::from(i))).map(|f| f.offset);
    let offset = at(frame)?;
    let end = if frame + 1 < seg_frames.0 + seg_frames.1 { at(frame + 1)? } else { bytes };
    Some((offset, at_usize(end.checked_sub(offset)?)))
}

#[cfg(test)]
#[path = "stored_tests.rs"]
mod tests;
