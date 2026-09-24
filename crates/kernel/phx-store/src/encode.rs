use phx_macros::clause;
use phx_num::violation;

use crate::backing::Backing;
use crate::column::Column;
use crate::consts::{ENCODE_BLOCK, ENCODE_MAGIC, ENCODE_VERSION, FRAME_BYTES, ZSTD_LEVEL};
use crate::convert::{to_u32, to_u64, to_usize};
use crate::descriptor::{ColumnDescriptor, FieldDescriptor, Transform};
use crate::pod::{Pod, as_bytes, as_bytes_mut};

/// Why a saved column could not be read back: the save is damaged or was written by another format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    Magic,
    Version(u16),
    Layout,
    Frame,
    Width(u8),
}

const WORD_BITS: u32 = u64::BITS;

/// The values a field of `width` bytes can take, as a mask.
fn mask(width: u8) -> u64 {
    let bits = u32::from(width) * u8::BITS;
    if bits >= WORD_BITS { u64::MAX } else { (1 << bits) - 1 }
}

fn to_signed(v: u64) -> i64 {
    i64::from_le_bytes(v.to_le_bytes())
}

fn to_unsigned(v: i64) -> u64 {
    u64::from_le_bytes(v.to_le_bytes())
}

/// A `width`-byte two's-complement value widened to 64 bits.
fn sign_extend(v: u64, width: u8) -> i64 {
    let shift = WORD_BITS - u32::from(width) * u8::BITS;
    to_signed(v << shift) >> shift
}

/// Small magnitudes of either sign to small unsigned values: 0, −1, 1, −2, … to 0, 1, 2, 3, ….
fn zigzag(v: i64) -> u64 {
    to_unsigned((v << 1) ^ (v >> (i64::BITS - 1)))
}

fn unzigzag(v: u64) -> u64 {
    (v >> 1) ^ 0_u64.wrapping_sub(v & 1)
}

fn transform_code(t: Transform) -> u8 {
    u8::from(t.delta()) | (u8::from(t.zigzag()) << 1)
}

fn field_values(bytes: &[u8], elem: usize, f: &FieldDescriptor) -> impl Iterator<Item = u64> {
    let (at, width) = (usize::from(f.offset), usize::from(f.width));
    bytes.chunks_exact(elem).map(move |row| {
        let mut word = [0; size_of::<u64>()];
        if let (Some(dst), Some(src)) = (word.get_mut(..width), row.get(at..at + width)) {
            dst.copy_from_slice(src);
        }
        u64::from_le_bytes(word)
    })
}

/// Writes one block's values at the width of its widest, least significant bit first, a word at a time.
fn pack(out: &mut Vec<u8>, values: &[u64]) {
    let bits = WORD_BITS - values.iter().fold(0, |m, v| m | v).leading_zeros();
    let [width, ..] = bits.to_le_bytes();
    out.push(width);
    out.reserve((values.len() * to_usize(bits)).div_ceil(to_usize(u8::BITS)));
    let (mut acc, mut held) = (0_u128, 0_u32);
    for v in values {
        acc |= u128::from(*v) << held;
        held += bits;
        if held >= WORD_BITS {
            let [low, _] = split(acc);
            out.extend_from_slice(&low.to_le_bytes());
            acc >>= WORD_BITS;
            held -= WORD_BITS;
        }
    }
    let [low, _] = split(acc);
    out.extend(low.to_le_bytes().into_iter().take(to_usize(held.div_ceil(u8::BITS))));
}

/// A 128-bit accumulator's low and high words.
fn split(acc: u128) -> [u64; 2] {
    let [b0, b1, b2, b3, b4, b5, b6, b7, b8, b9, b10, b11, b12, b13, b14, b15] = acc.to_le_bytes();
    [u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]), u64::from_le_bytes([b8, b9, b10, b11, b12, b13, b14, b15])]
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], DecodeError> {
    let (head, rest) = input.split_at_checked(n).ok_or(DecodeError::Truncated)?;
    *input = rest;
    Ok(head)
}

fn take_array<const N: usize>(input: &mut &[u8]) -> Result<[u8; N], DecodeError> {
    <[u8; N]>::try_from(take(input, N)?).map_err(|_| DecodeError::Truncated)
}

fn unpack(input: &mut &[u8], count: usize, out: &mut Vec<u64>) -> Result<(), DecodeError> {
    let [width] = take_array::<1>(input)?;
    let bits = u32::from(width);
    if bits > WORD_BITS {
        return Err(DecodeError::Width(width));
    }
    let packed = take(input, (count * to_usize(bits)).div_ceil(to_usize(u8::BITS)))?;
    let value_mask = if bits == WORD_BITS { u64::MAX } else { (1_u64 << bits) - 1 };
    let (mut acc, mut held, mut rest) = (0_u128, 0_u32, packed);
    for _ in 0..count {
        if held < bits {
            // Whole words while they last, then the tail's bytes one at a time.
            if let Some((word, more)) = rest.split_first_chunk::<{ size_of::<u64>() }>() {
                acc |= u128::from(u64::from_le_bytes(*word)) << held;
                held += WORD_BITS;
                rest = more;
            } else {
                while held < bits {
                    let (byte, more) = rest.split_first().ok_or(DecodeError::Truncated)?;
                    acc |= u128::from(*byte) << held;
                    held += u8::BITS;
                    rest = more;
                }
            }
        }
        let [low, _] = split(acc);
        out.push(low & value_mask);
        acc >>= bits;
        held -= bits;
    }
    Ok(())
}

/// One field's stream: its header, then per block of values the bit width and the packed values.
fn encode_field(out: &mut Vec<u8>, f: &FieldDescriptor, bytes: &[u8], elem: usize) {
    let count = bytes.len() / elem;
    out.extend_from_slice(&ENCODE_MAGIC.to_le_bytes());
    out.extend_from_slice(&ENCODE_VERSION.to_le_bytes());
    out.push(f.width);
    out.push(transform_code(f.transform));
    out.extend_from_slice(&to_u64(count).to_le_bytes());
    let (m, mut prev) = (mask(f.width), 0_u64);
    let mut block = Vec::with_capacity(ENCODE_BLOCK);
    let mut values = field_values(bytes, elem, f).peekable();
    while values.peek().is_some() {
        block.clear();
        for v in values.by_ref().take(ENCODE_BLOCK) {
            let d = if f.transform.delta() { v.wrapping_sub(prev) & m } else { v };
            prev = v;
            block.push(if f.transform.zigzag() { zigzag(sign_extend(d, f.width)) } else { d });
        }
        pack(out, &block);
    }
}

fn decode_field(input: &mut &[u8], field: &FieldDescriptor, rows: &mut [u8], elem: usize) -> Result<(), DecodeError> {
    if u32::from_le_bytes(take_array(input)?) != ENCODE_MAGIC {
        return Err(DecodeError::Magic);
    }
    let version = u16::from_le_bytes(take_array(input)?);
    if version != ENCODE_VERSION {
        return Err(DecodeError::Version(version));
    }
    let [width, code] = take_array(input)?;
    let count = u64::from_le_bytes(take_array(input)?);
    if width != field.width || code != transform_code(field.transform) || Ok(count) != u64::try_from(rows.len() / elem)
    {
        return Err(DecodeError::Layout);
    }
    let (values_mask, mut prev, at) = (mask(field.width), 0_u64, usize::from(field.offset));
    let mut values = Vec::with_capacity(ENCODE_BLOCK);
    let mut left = rows.len() / elem;
    let mut row_chunks = rows.chunks_exact_mut(elem);
    while left > 0 {
        let block_len = if left < ENCODE_BLOCK { left } else { ENCODE_BLOCK };
        values.clear();
        unpack(input, block_len, &mut values)?;
        for (coded, row) in values.iter().zip(row_chunks.by_ref()) {
            let step = if field.transform.zigzag() { unzigzag(*coded) & values_mask } else { *coded };
            let value = if field.transform.delta() { prev.wrapping_add(step) & values_mask } else { step };
            prev = value;
            let dst = row.get_mut(at..at + usize::from(field.width)).ok_or(DecodeError::Layout)?;
            dst.copy_from_slice(value.to_le_bytes().get(..dst.len()).ok_or(DecodeError::Layout)?);
        }
        left -= block_len;
    }
    Ok(())
}

fn check_layout<T: Pod>(desc: &ColumnDescriptor) -> usize {
    if usize::from(desc.elem_bytes) != size_of::<T>() {
        violation!(clause = "SET.12", "a descriptor of another width than its column", width = desc.elem_bytes);
    }
    size_of::<T>()
}

/// A column's rows as field streams, before compression.
fn encode_stream<T: Pod>(desc: &ColumnDescriptor, rows: &[T]) -> Vec<u8> {
    check_layout::<T>(desc);
    encode_rows(desc.fields, rows)
}

/// Rows as field streams, one per integer of the layout, each coded as its field declares and bit-packed per block:
/// what a save compresses.
#[clause("SET.12")]
#[must_use]
pub fn encode_rows<T: Pod>(fields: &[FieldDescriptor], rows: &[T]) -> Vec<u8> {
    let bytes = as_bytes(rows);
    let mut stream = Vec::new();
    for f in fields {
        encode_field(&mut stream, f, bytes, size_of::<T>());
    }
    stream
}

/// How many rows a field stream holds, as its first field's header names them.
///
/// # Errors
/// When the stream is too short to hold a header.
pub fn rows_in(stream: &[u8]) -> Result<usize, DecodeError> {
    let mut peek = stream;
    take(&mut peek, size_of::<u32>() + size_of::<u16>() + size_of::<u16>())?;
    usize::try_from(u64::from_le_bytes(take_array(&mut peek)?)).map_err(|_| DecodeError::Layout)
}

/// The exact inverse of `encode_rows`, into rows already sized to the count the stream names.
///
/// # Errors
/// When the stream is damaged, of another layout, or holds more or fewer rows.
#[clause("SET.12")]
pub fn decode_rows<T: Pod>(fields: &[FieldDescriptor], mut stream: &[u8], rows: &mut [T]) -> Result<(), DecodeError> {
    let bytes = as_bytes_mut(rows);
    fields.iter().try_for_each(|f| decode_field(&mut stream, f, bytes, size_of::<T>()))?;
    if stream.is_empty() { Ok(()) } else { Err(DecodeError::Layout) }
}

fn decode_stream<T: Pod, B: Backing>(
    desc: &ColumnDescriptor,
    stream: &[u8],
    into: &mut Column<T, B>,
) -> Result<(), DecodeError> {
    check_layout::<T>(desc);
    let count = rows_in(stream)?;
    // A damaged count must not ask for more rows than the column can ever hold.
    if count > into.capacity() - into.len() {
        return Err(DecodeError::Layout);
    }
    let before = into.len();
    let decoded = decode_rows(desc.fields, stream, into.append_zeroed(count));
    if decoded.is_err() {
        into.truncate(before);
    }
    decoded
}

/// A column encoded for a save: each field delta- or zigzag-coded as declared and bit-packed per block, then
/// compressed in zstd frames of at most `FRAME_BYTES`, each preceded by its length.
#[clause("SET.12")]
#[must_use]
pub fn encode_column<T: Pod>(desc: &ColumnDescriptor, rows: &[T]) -> Vec<u8> {
    let stream = encode_stream(desc, rows);
    let Ok(mut compressor) = zstd::bulk::Compressor::new(ZSTD_LEVEL) else {
        violation!(clause = "SET.12", "no compression context for a column");
    };
    let mut out = Vec::new();
    for piece in stream.chunks(FRAME_BYTES) {
        let Ok(frame) = compressor.compress(piece) else {
            violation!(clause = "SET.12", "compression refused a column", bytes = piece.len());
        };
        out.extend_from_slice(&to_u32(frame.len()).to_le_bytes());
        out.extend_from_slice(&frame);
    }
    out
}

/// The exact inverse of `encode_column`, appending the rows to a column.
///
/// # Errors
/// When the input is damaged, truncated, or of another format or layout.
#[clause("SET.12")]
pub fn decode_column<T: Pod, B: Backing>(
    desc: &ColumnDescriptor,
    mut input: &[u8],
    into: &mut Column<T, B>,
) -> Result<(), DecodeError> {
    let mut decompressor = zstd::bulk::Decompressor::new().map_err(|_| DecodeError::Frame)?;
    let mut stream = Vec::new();
    while !input.is_empty() {
        let len = to_usize(u32::from_le_bytes(take_array(&mut input)?));
        let frame = take(&mut input, len)?;
        let piece = decompressor.decompress(frame, FRAME_BYTES).map_err(|_| DecodeError::Frame)?;
        stream.extend_from_slice(&piece);
    }
    decode_stream(desc, &stream, into)
}

#[cfg(test)]
mod tests {
    use phx_rand::{Draws, Seed, Subject, SubjectTag, below_u64, stream_key};

    use super::{DecodeError, decode_column, decode_stream, encode_column, encode_stream};
    use crate::backing::{AddressSpace, HeapBacking};
    use crate::column::Column;
    use crate::descriptor::{ColumnDescriptor, FieldDescriptor, FieldTag, Transform};
    use crate::pod::{Pod, as_bytes};

    type Heap = HeapBacking<4096>;

    const STRUCT_FIELDS: &[FieldDescriptor] = &[
        FieldDescriptor { name: "a", offset: 0, width: 4, transform: Transform::Delta, tag: FieldTag::Plain },
        FieldDescriptor { name: "b", offset: 4, width: 2, transform: Transform::Zigzag, tag: FieldTag::Plain },
        FieldDescriptor { name: "c", offset: 6, width: 2, transform: Transform::Plain, tag: FieldTag::Plain },
    ];

    const TRANSFORMS: [Transform; 4] = [Transform::Plain, Transform::Delta, Transform::Zigzag, Transform::DeltaZigzag];

    const fn field(width: u8, transform: Transform) -> FieldDescriptor {
        FieldDescriptor { name: "v", offset: 0, width, transform, tag: FieldTag::Plain }
    }

    fn one_field<const W: u8>(transform: Transform) -> &'static [FieldDescriptor] {
        match transform {
            Transform::Plain => &const { [field(W, Transform::Plain)] },
            Transform::Delta => &const { [field(W, Transform::Delta)] },
            Transform::Zigzag => &const { [field(W, Transform::Zigzag)] },
            Transform::DeltaZigzag => &const { [field(W, Transform::DeltaZigzag)] },
        }
    }

    /// Random, sorted, constant, sign-changing and extreme values of one width, as raw words.
    fn shapes(n: usize) -> Vec<Vec<u64>> {
        let mut d = Draws::new(stream_key(Seed::new(3), "encode"), Subject::new(SubjectTag::World, 0), 0, 0);
        let random: Vec<u64> = (0..n).map(|_| below_u64(&mut d, u64::MAX)).collect();
        let mut sorted = random.clone();
        sorted.sort_unstable();
        let signs: Vec<u64> = (0..n)
            .map(|i| {
                let i = u64::try_from(i).unwrap();
                if i % 2 == 0 { i } else { u64::MAX - i }
            })
            .collect();
        let extreme: Vec<u64> = (0..n).map(|i| [0, u64::MAX, 1 << 63, (1 << 63) - 1][i % 4]).collect();
        vec![random, sorted, vec![42; n], signs, extreme, Vec::new()]
    }

    fn roundtrip<T: Pod + PartialEq + std::fmt::Debug>(desc: &ColumnDescriptor, rows: &[T], compress: bool) {
        let mut space = AddressSpace::empty();
        let mut back: Column<T, Heap> = Column::new(&mut space, 1 << 16, 4096);
        if compress {
            decode_column(desc, &encode_column(desc, rows), &mut back).unwrap();
        } else {
            decode_stream(desc, &encode_stream(desc, rows), &mut back).unwrap();
        }
        assert_eq!(as_bytes(back.slice()), as_bytes(rows), "{desc:?}");
    }

    fn every_pipeline(compress: bool) {
        let count = if cfg!(miri) { 70 } else { 3_000 };
        for shape in shapes(count) {
            for t in TRANSFORMS {
                let narrow = |w: u64| shape.iter().map(move |v| v & w);
                let d8 = ColumnDescriptor::checked::<u64>("v", 4096, one_field::<8>(t)).unwrap();
                roundtrip(&d8, &shape, compress);
                let v32: Vec<u32> = narrow(u64::from(u32::MAX)).map(|v| u32::try_from(v).unwrap()).collect();
                roundtrip(&ColumnDescriptor::checked::<u32>("v", 4096, one_field::<4>(t)).unwrap(), &v32, compress);
                let v16: Vec<u16> = narrow(u64::from(u16::MAX)).map(|v| u16::try_from(v).unwrap()).collect();
                roundtrip(&ColumnDescriptor::checked::<u16>("v", 4096, one_field::<2>(t)).unwrap(), &v16, compress);
                let v8: Vec<u8> = narrow(u64::from(u8::MAX)).map(|v| u8::try_from(v).unwrap()).collect();
                roundtrip(&ColumnDescriptor::checked::<u8>("v", 4096, one_field::<1>(t)).unwrap(), &v8, compress);
                // A struct column, encoded field by field with a transform each.
                let rows: Vec<[u16; 4]> = shape
                    .iter()
                    .map(|v| {
                        let [b0, b1, b2, b3, ..] = v.to_le_bytes().map(u16::from);
                        [b0, b1 | (b2 << 8), b3, b0 ^ b3]
                    })
                    .collect();
                roundtrip(&ColumnDescriptor::checked::<[u16; 4]>("s", 4096, STRUCT_FIELDS).unwrap(), &rows, compress);
            }
        }
    }

    #[test]
    fn encode_roundtrip_stream() {
        every_pipeline(false);
    }

    #[test]
    #[cfg_attr(miri, ignore = "zstd is C, which Miri cannot run")]
    fn encode_roundtrip() {
        every_pipeline(true);
    }

    #[test]
    #[cfg_attr(miri, ignore = "zstd is C, which Miri cannot run")]
    fn damaged_saves_are_refused() {
        let desc = ColumnDescriptor::checked::<u64>("v", 4096, one_field::<8>(Transform::Delta)).unwrap();
        let rows: Vec<u64> = (0..100).collect();
        let good = encode_column(&desc, &rows);
        let mut space = AddressSpace::empty();
        let mut back: Column<u64, Heap> = Column::new(&mut space, 1 << 10, 4096);
        assert_eq!(decode_column(&desc, &good[..good.len() - 1], &mut back), Err(DecodeError::Truncated));
        let other = ColumnDescriptor::checked::<u64>("v", 4096, one_field::<8>(Transform::Plain)).unwrap();
        assert_eq!(decode_column(&other, &good, &mut back), Err(DecodeError::Layout));
        let mut stream = encode_stream(&desc, &rows);
        stream[4] = 9;
        assert_eq!(decode_stream(&desc, &stream, &mut back), Err(DecodeError::Version(9)));
    }
}
