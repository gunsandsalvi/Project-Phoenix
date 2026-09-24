#![expect(unsafe_code, reason = "the storable marker is a promise about layout, made here for the base types")]

use phx_id::{
    CountryId, Day, DayLocalId, InstrumentId, LineId, MarketId, MsgId, PartyId, RegionId, SeriesId, Slot, StreamId,
    TableId, TileId, ZoneId,
};
use phx_num::{Amount, Ccy, Count, Fixed, MaybeI64, PointIdx, PriceRaw, QtyRaw, UnitId, capacity_exceeded};

use crate::descriptor::{FieldDescriptor, FieldTag, Transform};

#[cfg(not(target_endian = "little"))]
compile_error!("stores read their bytes as little-endian, the order saves and hashes are written in");

/// A type a store can hold as plain bytes.
///
/// # Safety
/// The type has no padding, holds no pointer or reference, and every bit pattern of its size is a valid value.
pub unsafe trait Pod: Copy + __seal::Sealed + 'static {
    /// The value's integers, each at its offset from `at` and encoded by `transform`, in the order a save writes
    /// them.
    fn layout(at: u16, transform: Transform, out: &mut Vec<FieldDescriptor>);
}

/// One integer of a stored value, at its offset.
fn leaf<T>(at: u16, transform: Transform, out: &mut Vec<FieldDescriptor>) {
    let Ok(width) = u8::try_from(size_of::<T>()) else {
        capacity_exceeded!("the bytes of a stored integer", u8::MAX, size_of::<T>());
    };
    out.push(FieldDescriptor { name: "", offset: at, width, transform, tag: FieldTag::Plain });
}

/// A field's offset within a stored value laid out at `at`, for the derive.
#[doc(hidden)]
#[must_use]
pub fn field_at(at: u16, offset: usize) -> u16 {
    match u16::try_from(offset).ok().and_then(|o| at.checked_add(o)) {
        Some(o) => o,
        None => capacity_exceeded!("the bytes of a stored value", u16::MAX, offset),
    }
}

/// Keeps `Pod` out of reach except through this module and the derive.
pub mod __seal {
    pub trait Sealed {}
}

macro_rules! base_pod {
    ($($t:ty),* $(,)?) => {
        $(
            // SAFETY: an integer, or a `repr(transparent)` wrapper of one, has no padding and accepts every bit
            // pattern.
            unsafe impl Pod for $t {
                fn layout(at: u16, transform: Transform, out: &mut Vec<FieldDescriptor>) {
                    leaf::<$t>(at, transform, out);
                }
            }
            impl __seal::Sealed for $t {}
        )*
    };
}

base_pod!(u8, u16, u32, u64, i8, i16, i32, i64);
base_pod!(Slot, TableId, LineId, InstrumentId, MarketId, TileId, ZoneId, RegionId, CountryId, DayLocalId, MsgId);
base_pod!(StreamId, PartyId, Day, SeriesId);
base_pod!(Amount, QtyRaw, PriceRaw, MaybeI64, PointIdx, Ccy, UnitId, Count);

// SAFETY: `Fixed` is `repr(transparent)` over an `i64`.
unsafe impl<const E: u8> Pod for Fixed<E> {
    fn layout(at: u16, transform: Transform, out: &mut Vec<FieldDescriptor>) {
        leaf::<Self>(at, transform, out);
    }
}
impl<const E: u8> __seal::Sealed for Fixed<E> {}

// SAFETY: an array lays its elements out with no gaps, so it has no padding its element lacks.
unsafe impl<T: Pod, const N: usize> Pod for [T; N] {
    fn layout(at: u16, transform: Transform, out: &mut Vec<FieldDescriptor>) {
        for i in 0..N {
            T::layout(field_at(at, i * size_of::<T>()), transform, out);
        }
    }
}
impl<T: Pod, const N: usize> __seal::Sealed for [T; N] {}

/// The bytes of stored values, in the order saves and hashes read them.
#[must_use]
pub fn as_bytes<T: Pod>(values: &[T]) -> &[u8] {
    // SAFETY: `T` has no padding, so every byte of the slice is initialised, and `u8` has no alignment.
    unsafe { std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), size_of_val(values)) }
}

/// The bytes of stored values, to be overwritten; any bytes written are valid values.
pub fn as_bytes_mut<T: Pod>(values: &mut [T]) -> &mut [u8] {
    // SAFETY: as above, and `T` accepts every bit pattern, so no write through the bytes can make an invalid value.
    unsafe { std::slice::from_raw_parts_mut(values.as_mut_ptr().cast::<u8>(), size_of_val(values)) }
}

/// A stored value read back from its bytes, or none when they are not its size.
#[must_use]
pub fn from_bytes<T: Pod>(bytes: &[u8]) -> Option<T> {
    // SAFETY: the length is the type's size, `T` accepts every bit pattern, and the read tolerates any alignment.
    (bytes.len() == size_of::<T>()).then(|| unsafe { bytes.as_ptr().cast::<T>().read_unaligned() })
}

#[cfg(test)]
mod tests {
    use super::{as_bytes, as_bytes_mut, from_bytes};

    #[test]
    fn bytes_are_little_endian_and_writable() {
        let mut v = [0x0102_0304_u32, 5];
        assert_eq!(as_bytes(&v), &[4, 3, 2, 1, 5, 0, 0, 0]);
        as_bytes_mut(&mut v)[4] = 9;
        assert_eq!(v, [0x0102_0304, 9]);
        assert_eq!(from_bytes::<u32>(&[1, 0, 0, 0]), Some(1));
        assert_eq!(from_bytes::<u32>(&[1, 0, 0]), None);
    }
}
