use phx_macros::clause;

use crate::pod::Pod;

/// What a field holds, so that checks can find every reference of one kind and counters can name what a byte is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldTag {
    PartyRef,
    LineRef,
    InstrumentRef,
    TileRef,
    Day,
    Amount,
    Qty,
    Plain,
}

/// The reversible steps a field's values go through before bit-packing; delta suits sorted or slowly moving values,
/// zigzag values that change sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Transform {
    Plain,
    Delta,
    Zigzag,
    DeltaZigzag,
}

impl Transform {
    #[must_use]
    pub fn delta(self) -> bool {
        matches!(self, Transform::Delta | Transform::DeltaZigzag)
    }

    #[must_use]
    pub fn zigzag(self) -> bool {
        matches!(self, Transform::Zigzag | Transform::DeltaZigzag)
    }
}

/// One field of a stored element: where it lies, how wide it is, how it is encoded, what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldDescriptor {
    pub name: &'static str,
    pub offset: u16,
    pub width: u8,
    pub transform: Transform,
    pub tag: FieldTag,
}

/// A column's element layout, which saves, hashes and counters read.
#[clause("SET.12")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnDescriptor {
    pub name: &'static str,
    pub elem_bytes: u16,
    pub rows_per_chunk: u32,
    pub fields: &'static [FieldDescriptor],
}

impl ColumnDescriptor {
    /// A descriptor whose fields, in order, tile the element exactly, each one, two, four or eight bytes wide; any
    /// other layout is refused.
    #[must_use]
    pub fn checked<T: Pod>(
        name: &'static str,
        rows_per_chunk: u32,
        fields: &'static [FieldDescriptor],
    ) -> Option<ColumnDescriptor> {
        let elem_bytes = u16::try_from(size_of::<T>()).ok()?;
        let mut end = 0_u16;
        for f in fields {
            let width_ok = f.width.is_power_of_two() && usize::from(f.width) <= size_of::<u64>();
            if !width_ok || f.offset != end {
                return None;
            }
            end = end.checked_add(u16::from(f.width))?;
        }
        (end == elem_bytes && rows_per_chunk.is_power_of_two()).then_some(ColumnDescriptor {
            name,
            elem_bytes,
            rows_per_chunk,
            fields,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ColumnDescriptor, FieldDescriptor, FieldTag, Transform};

    const fn field(name: &'static str, offset: u16, width: u8) -> FieldDescriptor {
        FieldDescriptor { name, offset, width, transform: Transform::Plain, tag: FieldTag::Plain }
    }

    #[test]
    fn descriptors_tile_their_element() {
        const GOOD: &[FieldDescriptor] = &[field("a", 0, 4), field("b", 4, 2), field("c", 6, 1), field("d", 7, 1)];
        const GAP: &[FieldDescriptor] = &[field("a", 0, 4), field("b", 6, 2)];
        const WIDE: &[FieldDescriptor] = &[field("a", 0, 3), field("b", 3, 5)];
        const SHORT: &[FieldDescriptor] = &[field("a", 0, 4)];
        assert!(ColumnDescriptor::checked::<u64>("x", 4096, GOOD).is_some());
        assert!(ColumnDescriptor::checked::<u64>("x", 4096, GAP).is_none());
        assert!(ColumnDescriptor::checked::<u64>("x", 4096, WIDE).is_none());
        assert!(ColumnDescriptor::checked::<u64>("x", 4096, SHORT).is_none());
        assert!(ColumnDescriptor::checked::<u64>("x", 1000, GOOD).is_none());
    }
}
