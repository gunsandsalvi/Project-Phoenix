use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Read, Write};

use phx_macros::clause;
use phx_num::{Missing, violation};

use crate::backing::AddressSpace;
use crate::consts::{ENCODE_BLOCK, SAVE_FRAME_BYTES, SAVE_FRAME_WAVE, SAVE_READ_STEP, VA_BUDGET};
use crate::descriptor::{FieldDescriptor, Transform};
use crate::encode::{DecodeError, decode_rows, encode_rows, rows_in};
use crate::pod::{Pod, as_bytes, as_bytes_mut, from_bytes};

/// Why a save could not be read back: the file, its compression, a store's encoding, or what it says.
#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Decode(DecodeError),
    Invalid(String),
}

impl core::fmt::Display for LoadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "reading the save: {e}"),
            LoadError::Decode(e) => write!(f, "decoding a store: {e:?}"),
            LoadError::Invalid(why) => write!(f, "the save is not a world: {why}"),
        }
    }
}

impl From<io::Error> for LoadError {
    fn from(e: io::Error) -> LoadError {
        LoadError::Io(e)
    }
}

impl From<DecodeError> for LoadError {
    fn from(e: DecodeError) -> LoadError {
        LoadError::Decode(e)
    }
}

/// A sink that counts the bytes it passes on.
struct Counted<'a> {
    inner: &'a mut dyn Write,
    bytes: u64,
}

impl Write for Counted<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.bytes += crate::convert::to_u64(n);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A store's compression: zstd, except under the interpreter that checks this crate's unsafe code, which cannot run a
/// foreign library and so reads and writes the stream as it stands.
#[cfg(not(miri))]
mod frame {
    use std::io::{self, BufReader, Read, Write};

    use crate::consts::ZSTD_LEVEL;

    pub type Compress<W> = zstd::stream::write::Encoder<'static, W>;
    pub type Decompress<'a> = zstd::stream::read::Decoder<'static, BufReader<&'a mut dyn Read>>;

    pub fn compress<W: Write>(sink: W) -> io::Result<Compress<W>> {
        zstd::stream::write::Encoder::new(sink, ZSTD_LEVEL)
    }

    pub fn finish<W: Write>(c: Compress<W>) -> io::Result<W> {
        c.finish()
    }

    pub fn decompress(source: &mut dyn Read) -> io::Result<Decompress<'_>> {
        zstd::stream::read::Decoder::new(source)
    }

    pub fn whole(raw: &[u8]) -> io::Result<Vec<u8>> {
        zstd::bulk::compress(raw, ZSTD_LEVEL)
    }
}

#[cfg(miri)]
mod frame {
    use std::io::{self, Read, Write};

    pub type Compress<W> = W;
    pub type Decompress<'a> = &'a mut dyn Read;

    pub fn compress<W: Write>(sink: W) -> io::Result<W> {
        Ok(sink)
    }

    pub fn finish<W: Write>(mut c: W) -> io::Result<W> {
        c.flush()?;
        Ok(c)
    }

    pub fn whole(raw: &[u8]) -> io::Result<Vec<u8>> {
        Ok(raw.to_vec())
    }

    pub fn decompress(source: &mut dyn Read) -> io::Result<Decompress<'_>> {
        Ok(source)
    }
}

/// One frame of a store compressed on its own, as the frames of a framed writer are: a zstd frame, which read in turn
/// with the store's other frames reads as one stream.
///
/// # Errors
/// When no compression context can be made.
pub fn compress_frame(raw: &[u8]) -> io::Result<Vec<u8>> {
    frame::whole(raw)
}

/// What compresses a framed writer's frames: the frames given, each compressed as `compress_frame` does, in order.
pub type Frames<'a> = &'a dyn Fn(&[Vec<u8>]) -> Vec<io::Result<Vec<u8>>>;

/// One store written as a zstd stream: what saves write never fails the world; the first error the sink returns is
/// kept and reported when the store is finished.
pub struct Writer<'a> {
    out: Out<'a>,
    failed: Option<io::Error>,
    raw: u64,
}

/// Where a writer's bytes go: a store's compressed stream; its fixed frames, a wave at a time compressed by the
/// caller, who may compress them at once; or a hash of a value's encoding as it stands.
enum Out<'a> {
    Store(frame::Compress<Counted<'a>>),
    Framed { sink: Counted<'a>, frames: Vec<Vec<u8>>, compress: Frames<'a> },
    Hash(&'a mut crate::hash::LogicalHasher),
}

/// A wave of a framed writer's frames compressed and written in order.
fn flush_frames(sink: &mut Counted<'_>, frames: &mut Vec<Vec<u8>>, compress: Frames<'_>) -> io::Result<()> {
    for out in compress(frames) {
        sink.write_all(&out?)?;
    }
    frames.clear();
    Ok(())
}

impl core::fmt::Debug for Writer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Writer").field("raw", &self.raw).field("failed", &self.failed).finish_non_exhaustive()
    }
}

impl<'a> Writer<'a> {
    /// A store's writer into a sink.
    ///
    /// # Errors
    /// When no compression context can be made.
    pub fn new(sink: &'a mut dyn Write) -> io::Result<Writer<'a>> {
        let out = frame::compress(Counted { inner: sink, bytes: 0 })?;
        Ok(Writer { out: Out::Store(out), failed: None, raw: 0 })
    }

    /// A store's writer into a sink in fixed frames, a wave of them compressed at a time by `compress`: the frames
    /// are cut by bytes alone, so the file is the same however they are compressed.
    #[must_use]
    pub fn framed(sink: &'a mut dyn Write, compress: Frames<'a>) -> Writer<'a> {
        let frames = vec![Vec::with_capacity(SAVE_FRAME_BYTES)];
        Writer { out: Out::Framed { sink: Counted { inner: sink, bytes: 0 }, frames, compress }, failed: None, raw: 0 }
    }

    /// Bytes as they stand.
    pub fn bytes(&mut self, b: &[u8]) {
        match &mut self.out {
            Out::Store(out) => {
                if self.failed.is_none()
                    && let Err(e) = out.write_all(b)
                {
                    self.failed = Some(e);
                }
            }
            Out::Framed { sink, frames, compress } => {
                let mut rest = b;
                while !rest.is_empty() {
                    let Some(last) = frames.last_mut() else {
                        violation!(clause = "SET.12", "a framed writer with no frame open");
                    };
                    let room = SAVE_FRAME_BYTES - last.len();
                    let (now, later) = rest.split_at(if rest.len() < room { rest.len() } else { room });
                    last.extend_from_slice(now);
                    rest = later;
                    if last.len() == SAVE_FRAME_BYTES {
                        if frames.len() == SAVE_FRAME_WAVE
                            && self.failed.is_none()
                            && let Err(e) = flush_frames(sink, frames, *compress)
                        {
                            self.failed = Some(e);
                        }
                        frames.retain(|f| !f.is_empty());
                        frames.push(Vec::with_capacity(SAVE_FRAME_BYTES));
                    }
                }
            }
            Out::Hash(h) => h.bytes(b),
        }
        self.raw += crate::convert::to_u64(b.len());
    }

    /// A count, which a reader reads back with `count`.
    pub fn count(&mut self, n: usize) {
        self.bytes(&crate::convert::to_u64(n).to_le_bytes());
    }

    /// Rows through their layout's transforms and bit-packing, before the store's compression.
    #[clause("SET.12")]
    pub fn rows<T: Pod>(&mut self, rows: &[T], transform: Transform) {
        let mut fields = Vec::new();
        T::layout(0, transform, &mut fields);
        let stream = encode_rows(&fields, rows);
        self.count(stream.len());
        self.bytes(&stream);
    }

    /// The store closed: its compressed bytes, and the bytes before compression.
    ///
    /// # Errors
    /// The first error the sink returned.
    pub fn finish(self) -> io::Result<(u64, u64)> {
        if let Some(e) = self.failed {
            return Err(e);
        }
        match self.out {
            Out::Store(out) => Ok((frame::finish(out)?.bytes, self.raw)),
            Out::Framed { mut sink, mut frames, compress } => {
                frames.retain(|f| !f.is_empty());
                flush_frames(&mut sink, &mut frames, compress)?;
                sink.flush()?;
                Ok((sink.bytes, self.raw))
            }
            Out::Hash(_) => Ok((0, self.raw)),
        }
    }
}

/// A value's content into a hash, as its save encoding states it: for keyed state whose encoding is its logical
/// content, with no layout in it.
pub fn hash_saved<T: Saved>(value: &T, h: &mut crate::hash::LogicalHasher) {
    let mut w = Writer { out: Out::Hash(h), failed: None, raw: 0 };
    value.save(&mut w);
}

/// One store read back from its zstd stream, reserving what it rebuilds in its own address space.
pub struct Reader<'a> {
    input: frame::Decompress<'a>,
    space: AddressSpace,
    names: Vec<&'static str>,
}

impl core::fmt::Debug for Reader<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Reader").field("space", &self.space).finish_non_exhaustive()
    }
}

impl<'a> Reader<'a> {
    /// A store's reader over its bytes.
    ///
    /// # Errors
    /// When no decompression context can be made.
    pub fn new(source: &'a mut dyn Read) -> io::Result<Reader<'a>> {
        Ok(Reader { input: frame::decompress(source)?, space: AddressSpace::empty(), names: Vec::new() })
    }

    /// Fills a buffer from the store.
    ///
    /// # Errors
    /// When the store ends first.
    pub fn fill(&mut self, buf: &mut [u8]) -> Result<(), LoadError> {
        self.input.read_exact(buf).map_err(LoadError::Io)
    }

    /// `n` bytes, read a step at a time, so that a damaged count runs out of store before it runs out of memory.
    ///
    /// # Errors
    /// When the store ends first.
    pub fn bytes(&mut self, n: usize) -> Result<Vec<u8>, LoadError> {
        let mut out = Vec::new();
        while out.len() < n {
            let step = if n - out.len() < SAVE_READ_STEP { n - out.len() } else { SAVE_READ_STEP };
            let at = out.len();
            out.resize(at + step, 0);
            if let Some(buf) = out.get_mut(at..) {
                self.fill(buf)?;
            }
        }
        Ok(out)
    }

    /// A count written by `Writer::count`.
    ///
    /// # Errors
    /// When the store ends first or the count is beyond this machine's words.
    pub fn count(&mut self) -> Result<usize, LoadError> {
        let mut b = [0; size_of::<u64>()];
        self.fill(&mut b)?;
        usize::try_from(u64::from_le_bytes(b)).map_err(|_| LoadError::Invalid("a count beyond this machine".to_owned()))
    }

    /// Rows written by `Writer::rows`, into rows already sized to their count.
    ///
    /// # Errors
    /// When the store is damaged or holds another count or layout.
    #[clause("SET.12")]
    pub fn rows_into<T: Pod>(&mut self, transform: Transform, into: &mut [T]) -> Result<(), LoadError> {
        let n = self.count()?;
        let stream = self.bytes(n)?;
        if rows_in(&stream)? != into.len() {
            return Err(LoadError::Decode(DecodeError::Layout));
        }
        let mut fields: Vec<FieldDescriptor> = Vec::new();
        T::layout(0, transform, &mut fields);
        decode_rows(&fields, &stream, into)?;
        Ok(())
    }

    /// Rows written by `Writer::rows`, as many as they were.
    ///
    /// # Errors
    /// When the store is damaged or holds another layout.
    pub fn rows<T: Pod>(&mut self, transform: Transform) -> Result<Vec<T>, LoadError> {
        let n = self.count()?;
        let stream = self.bytes(n)?;
        let count = rows_in(&stream)?;
        let mut out = vec_zeroed::<T>(count, stream.len())?;
        let mut fields: Vec<FieldDescriptor> = Vec::new();
        T::layout(0, transform, &mut fields);
        decode_rows(&fields, &stream, &mut out)?;
        Ok(out)
    }

    /// The names the build declares, which a saved name must be one of.
    pub fn with_names(&mut self, names: &[&'static str]) {
        self.names = names.to_vec();
        self.names.sort_unstable();
        self.names.dedup();
    }

    /// A saved name as the build's own.
    ///
    /// # Errors
    /// When the build declares no such name.
    pub fn name(&self, text: &str) -> Result<&'static str, LoadError> {
        match self.names.binary_search(&text) {
            Ok(i) => self.names.get(i).copied().ok_or_else(|| LoadError::Invalid(format!("the name `{text}`"))),
            Err(_) => Err(LoadError::Invalid(format!("`{text}`, a name this build does not declare"))),
        }
    }

    /// The address space what this store rebuilt was reserved in.
    pub fn space(&mut self) -> &mut AddressSpace {
        &mut self.space
    }

    /// The address space, handed to the owner of what was rebuilt; the reader keeps an empty one.
    pub fn take_space(&mut self) -> AddressSpace {
        core::mem::replace(&mut self.space, AddressSpace::empty())
    }

    /// Whether the store has been read to its end, as a whole store must be.
    ///
    /// # Errors
    /// When the store cannot be read.
    pub fn at_end(&mut self) -> Result<bool, LoadError> {
        let mut one = [0_u8; 1];
        Ok(self.input.read(&mut one)? == 0)
    }
}

/// Zeroed rows, as many as a stream names, refused when the stream is far too short to hold them.
fn vec_zeroed<T: Pod>(count: usize, stream_bytes: usize) -> Result<Vec<T>, LoadError> {
    // Every block of rows costs at least its width byte once packed; a count past what the stream allows is damage.
    if count.div_ceil(ENCODE_BLOCK) > stream_bytes {
        return Err(LoadError::Decode(DecodeError::Layout));
    }
    let Some(zero) = from_bytes::<T>(&vec![0; size_of::<T>()]) else {
        violation!(clause = "SET.12", "a stored type whose zero bytes are not its size");
    };
    Ok(vec![zero; count])
}

/// A value a save writes and reads back exactly.
#[clause("SET.12", "SET.15")]
pub trait Saved: Sized {
    fn save(&self, w: &mut Writer<'_>);
    /// # Errors
    /// When the store is damaged or does not hold this value.
    fn load(r: &mut Reader<'_>) -> Result<Self, LoadError>;
}

impl<T: Pod> Saved for T {
    fn save(&self, w: &mut Writer<'_>) {
        w.bytes(as_bytes(core::slice::from_ref(self)));
    }

    fn load(r: &mut Reader<'_>) -> Result<T, LoadError> {
        let mut out = vec_zeroed::<T>(1, 1)?;
        r.fill(as_bytes_mut(&mut out))?;
        out.pop().ok_or_else(|| LoadError::Invalid("a value read as nothing".to_owned()))
    }
}

impl Saved for bool {
    fn save(&self, w: &mut Writer<'_>) {
        w.bytes(&[u8::from(*self)]);
    }

    fn load(r: &mut Reader<'_>) -> Result<bool, LoadError> {
        match u8::load(r)? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(LoadError::Invalid(format!("a truth value of {b}"))),
        }
    }
}

impl Saved for usize {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(*self);
    }

    fn load(r: &mut Reader<'_>) -> Result<usize, LoadError> {
        r.count()
    }
}

impl Saved for u128 {
    fn save(&self, w: &mut Writer<'_>) {
        w.bytes(&self.to_le_bytes());
    }

    fn load(r: &mut Reader<'_>) -> Result<u128, LoadError> {
        let mut b = [0; size_of::<u128>()];
        r.fill(&mut b)?;
        Ok(u128::from_le_bytes(b))
    }
}

impl Saved for i128 {
    fn save(&self, w: &mut Writer<'_>) {
        w.bytes(&self.to_le_bytes());
    }

    fn load(r: &mut Reader<'_>) -> Result<i128, LoadError> {
        let mut b = [0; size_of::<i128>()];
        r.fill(&mut b)?;
        Ok(i128::from_le_bytes(b))
    }
}

impl Saved for f64 {
    fn save(&self, w: &mut Writer<'_>) {
        w.bytes(&self.to_bits().to_le_bytes());
    }

    fn load(r: &mut Reader<'_>) -> Result<f64, LoadError> {
        Ok(f64::from_bits(u64::load(r)?))
    }
}

impl Saved for String {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(self.len());
        w.bytes(self.as_bytes());
    }

    fn load(r: &mut Reader<'_>) -> Result<String, LoadError> {
        let n = r.count()?;
        String::from_utf8(r.bytes(n)?).map_err(|_| LoadError::Invalid("a name that is not text".to_owned()))
    }
}

impl<T: Saved> Saved for Vec<T> {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(self.len());
        for v in self {
            v.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<Vec<T>, LoadError> {
        let n = r.count()?;
        let mut out = Vec::new();
        for _ in 0..n {
            out.push(T::load(r)?);
        }
        Ok(out)
    }
}

impl<T: Saved> Saved for Option<T> {
    fn save(&self, w: &mut Writer<'_>) {
        self.is_some().save(w);
        if let Some(v) = self {
            v.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<Option<T>, LoadError> {
        Ok(if bool::load(r)? { Some(T::load(r)?) } else { None })
    }
}

impl<T: Saved> Saved for Missing<T> {
    fn save(&self, w: &mut Writer<'_>) {
        matches!(self, Missing::Present(_)).save(w);
        if let Missing::Present(v) = self {
            v.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<Missing<T>, LoadError> {
        Ok(if bool::load(r)? { Missing::Present(T::load(r)?) } else { Missing::Absent })
    }
}

impl<A: Saved, B: Saved> Saved for (A, B) {
    fn save(&self, w: &mut Writer<'_>) {
        self.0.save(w);
        self.1.save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<(A, B), LoadError> {
        Ok((A::load(r)?, B::load(r)?))
    }
}

impl<A: Saved, B: Saved, C: Saved> Saved for (A, B, C) {
    fn save(&self, w: &mut Writer<'_>) {
        self.0.save(w);
        self.1.save(w);
        self.2.save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<(A, B, C), LoadError> {
        Ok((A::load(r)?, B::load(r)?, C::load(r)?))
    }
}

impl<K: Saved + Ord, V: Saved> Saved for BTreeMap<K, V> {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(self.len());
        for (k, v) in self {
            k.save(w);
            v.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<BTreeMap<K, V>, LoadError> {
        let n = r.count()?;
        let mut out = BTreeMap::new();
        for _ in 0..n {
            let k = K::load(r)?;
            if out.insert(k, V::load(r)?).is_some() {
                return Err(LoadError::Invalid("a key saved twice".to_owned()));
            }
        }
        Ok(out)
    }
}

impl<T: Saved + Ord> Saved for BTreeSet<T> {
    fn save(&self, w: &mut Writer<'_>) {
        w.count(self.len());
        for v in self {
            v.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<BTreeSet<T>, LoadError> {
        let n = r.count()?;
        let mut out = BTreeSet::new();
        for _ in 0..n {
            if !out.insert(T::load(r)?) {
                return Err(LoadError::Invalid("a member saved twice".to_owned()));
            }
        }
        Ok(out)
    }
}

/// A count saved as a word, read back into the width the store keeps it in.
///
/// # Errors
/// When the saved count does not fit.
pub fn narrow<T: TryFrom<usize>>(n: usize, what: &str) -> Result<T, LoadError> {
    T::try_from(n).map_err(|_| LoadError::Invalid(format!("{what} beyond its width")))
}

/// A reservation sized from a save, refused when it exceeds what the store may ever hold.
pub(crate) fn within(n: usize, capacity: usize, what: &str) -> Result<(), LoadError> {
    if n > capacity { Err(LoadError::Invalid(format!("{what}: {n} beyond a capacity of {capacity}"))) } else { Ok(()) }
}

/// A region's capacity read back, refused when a reservation of it would pass the address-space budget.
pub(crate) fn capacity<T>(r: &mut Reader<'_>) -> Result<usize, LoadError> {
    let cap = r.count()?;
    match cap.checked_mul(size_of::<T>()) {
        Some(bytes) if bytes <= VA_BUDGET => Ok(cap),
        _ => Err(LoadError::Invalid(format!("a reservation of {cap} elements beyond the address-space budget"))),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use phx_id::{PartyId, Slot, TableId};
    use phx_num::Missing;

    use super::{LoadError, Reader, Saved, Writer};
    use crate::Transform;
    use crate::arena::{ChunkArena, ListRef};
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::column::Column;
    use crate::hash::LogicalHasher;
    use crate::table::Table;

    type Heap = HeapBacking<4096>;
    type Value = (BTreeMap<PartyId, (i128, Missing<String>)>, Vec<Option<f64>>, bool);

    fn write(f: impl FnOnce(&mut Writer<'_>)) -> Vec<u8> {
        let mut out = Vec::new();
        let mut w = Writer::new(&mut out).unwrap();
        f(&mut w);
        w.finish().unwrap();
        out
    }

    fn read<T>(bytes: &[u8], f: impl FnOnce(&mut Reader<'_>) -> Result<T, LoadError>) -> T {
        let mut src: &[u8] = bytes;
        let mut r = Reader::new(&mut src).unwrap();
        let v = f(&mut r).unwrap();
        assert!(r.at_end().unwrap(), "a store is read to its end");
        v
    }

    #[test]
    fn a_framed_store_reads_back_as_one_stream() {
        let words: Vec<u64> = (0..600_000_u64).map(|i| i * i).collect();
        let compress = |frames: &[Vec<u8>]| frames.iter().map(|f| super::compress_frame(f)).collect::<Vec<_>>();
        let mut file = Vec::new();
        let mut w = Writer::framed(&mut file, &compress);
        for chunk in words.chunks(1000) {
            let bytes: Vec<u8> = chunk.iter().flat_map(|x| x.to_le_bytes()).collect();
            w.bytes(&bytes);
        }
        let (_, raw) = w.finish().unwrap();
        assert_eq!(raw, 600_000 * 8);
        let mut input: &[u8] = &file;
        let mut r = Reader::new(&mut input).unwrap();
        let back = r.bytes(600_000 * 8).unwrap();
        assert!(back.chunks(8).map(|b| u64::from_le_bytes(b.try_into().unwrap())).eq(words.iter().copied()));
        assert!(r.at_end().unwrap());
    }
    #[test]
    fn values_roundtrip() {
        let map: BTreeMap<PartyId, (i128, Missing<String>)> = [
            (PartyId::new(3), (-7, Missing::Present("bank".to_owned()))),
            (PartyId::new(9), (i128::MAX, Missing::Absent)),
        ]
        .into();
        let v: Value = (map, vec![Some(1.5_f64), None], true);
        let bytes = write(|w| v.save(w));
        let back: Value = read(&bytes, Saved::load);
        assert_eq!(back, v);
    }

    #[derive(Debug, PartialEq, phx_macros::Saved)]
    enum Shape {
        Empty,
        Pair(u8, i64),
        Named { name: String, at: Missing<PartyId> },
    }

    #[derive(Debug, PartialEq, phx_macros::Saved)]
    struct Kept {
        shapes: Vec<Shape>,
        #[saved(skip)]
        index: Vec<u32>,
    }

    #[test]
    fn derived_values_roundtrip_and_skip_their_indexes() {
        let kept = Kept {
            shapes: vec![
                Shape::Empty,
                Shape::Pair(4, -9),
                Shape::Named { name: "reserves".to_owned(), at: Missing::Present(PartyId::new(2)) },
            ],
            index: vec![1, 2],
        };
        let back: Kept = read(&write(|w| kept.save(w)), Saved::load);
        assert_eq!(back.shapes, kept.shapes);
        assert!(back.index.is_empty(), "an index is rebuilt by its owner, never read back");
    }

    #[test]
    fn rows_roundtrip_through_their_transforms() {
        let rows: Vec<[u32; 2]> = (0..5_000).map(|i| [i * 3, 5_000 - i]).collect();
        for t in [Transform::Plain, Transform::Delta, Transform::Zigzag, Transform::DeltaZigzag] {
            let bytes = write(|w| w.rows(&rows, t));
            assert_eq!(read(&bytes, |r| r.rows::<[u32; 2]>(t)), rows);
        }
    }

    #[test]
    fn damage_is_refused() {
        let bytes = write(|w| vec![1_u64, 2, 3].save(w));
        let cut = bytes.get(..bytes.len() - 4).unwrap();
        let mut src: &[u8] = cut;
        let mut r = Reader::new(&mut src).unwrap();
        assert!(<Vec<u64>>::load(&mut r).is_err(), "a truncated store is refused");
        let bytes = write(|w| 2_u8.save(w));
        let mut src: &[u8] = &bytes;
        assert!(bool::load(&mut Reader::new(&mut src).unwrap()).is_err(), "a truth value is 0 or 1");
    }

    #[test]
    fn allocator_state_roundtrip() {
        let mut space = AddressSpace::empty();
        let mut table: Table<Heap> = Table::new(&mut space, TableId::new(2), 1_000, 64);
        let slots: Vec<Slot> = (0..10).map(|_| table.slots.alloc()).collect();
        table.slots.release(slots[3]);
        table.slots.close_day();
        table.slots.release(slots[7]);
        let mut arena: ChunkArena<Heap> = ChunkArena::new(&mut space, 1 << 12);
        let (mut a, mut b) = (ListRef::EMPTY, ListRef::EMPTY);
        arena.append(&mut a, &[1, 2, 3]);
        arena.append(&mut b, &[9]);
        arena.append(&mut a, &[4, 5, 6, 7, 8, 9]);
        let bytes = write(|w| {
            table.save(w);
            arena.save(w);
        });
        let (mut back, arena_back): (Table<Heap>, ChunkArena<Heap>) =
            read(&bytes, |r| Ok((Table::load(r)?, ChunkArena::load(r)?)));
        assert_eq!(back.slots.live_slots().collect::<Vec<_>>(), table.slots.live_slots().collect::<Vec<_>>());
        assert_eq!(back.slots.alloc(), table.slots.alloc(), "the free slot comes back first");
        back.slots.close_day();
        table.slots.close_day();
        assert_eq!(back.slots.alloc(), table.slots.alloc(), "a slot released today waits for the close");
        assert_eq!((arena_back.used_words(), arena_back.dead_words()), (arena.used_words(), arena.dead_words()));
        assert_eq!(arena_back.read(a), arena.read(a));
        assert_eq!(arena_back.read(b), [9]);
    }

    #[test]
    fn streamed_hash_equals_whole_hash() {
        let mut space = AddressSpace::empty();
        let mut first: Column<u64, Heap> = Column::new(&mut space, 1 << 12, 64);
        let mut second: Column<i32, Heap> = Column::new(&mut space, 1 << 12, 64);
        (0..3_000_u64).for_each(|i| first.push(i * i));
        (0..700_i32).for_each(|i| second.push(-i));
        let whole = {
            let mut h = LogicalHasher::new([1, 2]);
            first.slice().iter().for_each(|v| h.u64(*v));
            second.slice().iter().for_each(|v| h.u64(u64::from(v.unsigned_abs())));
            h.finish()
        };
        let stores = [write(|w| first.save(w)), write(|w| second.save(w))];
        let mut h = LogicalHasher::new([1, 2]);
        let back: Column<u64, Heap> = read(&stores[0], Column::load);
        back.slice().iter().for_each(|v| h.u64(*v));
        drop(back);
        let back: Column<i32, Heap> = read(&stores[1], Column::load);
        back.slice().iter().for_each(|v| h.u64(u64::from(v.unsigned_abs())));
        assert_eq!(h.finish(), whole, "hashing each store as it is read equals hashing the columns together");
    }
}
