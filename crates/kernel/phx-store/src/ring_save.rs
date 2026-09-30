//! A ring saved as its live chunks alone and laid again from its first chunk at a load; two rings compared by what
//! they keep, wherever their chunks lie.

use crate::Transform;
use crate::backing::Backing;
use crate::column::Column;
use crate::convert::to_usize;
use crate::pod::{Pod, as_bytes};
use crate::save::{LoadError, Reader, Saved, Writer, capacity, within};

use super::{ChunkHeader, HorizonRing};

impl<T: Pod, B: Backing> PartialEq for HorizonRing<T, B> {
    /// Two rings are equal when they keep the same horizon, floors and ordinals, and their live chunks hold the same
    /// days and rows, wherever their chunks lie.
    fn eq(&self, other: &HorizonRing<T, B>) -> bool {
        let same = |(a, ad, ar): (ChunkHeader, &[u32], &[T]), (b, bd, br): (ChunkHeader, &[u32], &[T])| {
            (a.first_day, a.last_day, a.first_ordinal, a.len) == (b.first_day, b.last_day, b.first_ordinal, b.len)
                && ad == bd
                && as_bytes(ar) == as_bytes(br)
        };
        (self.chunk_rows, self.max_chunks, self.horizon, self.base, self.next)
            == (other.chunk_rows, other.max_chunks, other.horizon, other.base, other.next)
            && self.floors.slice() == other.floors.slice()
            && self.live == other.live
            && self.chunks(0).zip(other.chunks(0)).all(|(a, b)| same(a, b))
    }
}

impl<T: Pod, B: Backing> Saved for HorizonRing<T, B> {
    /// The live chunks alone, in order: a load lays them from the ring's first chunk, the pruned ones not kept.
    fn save(&self, w: &mut Writer<'_>) {
        w.count(to_usize(self.chunk_rows) * to_usize(self.max_chunks));
        (self.chunk_rows, self.max_chunks, self.horizon).save(w);
        (self.base, self.next).save(w);
        self.floors.save(w);
        w.count(self.live);
        for (h, d, r) in self.chunks(0) {
            // Where a chunk lies is the ring's own layout, laid again by the load.
            ChunkHeader { at: 0, ..h }.save(w);
            w.rows(d, Transform::Plain);
            w.rows(r, Transform::Plain);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<HorizonRing<T, B>, LoadError> {
        let rows = capacity::<T>(r)?;
        let (chunk_rows, max_chunks, horizon) = <(u32, u32, u32)>::load(r)?;
        if to_usize(chunk_rows).checked_mul(to_usize(max_chunks)) != Some(rows) {
            return Err(LoadError::Invalid("a ring's chunks not its rows".to_owned()));
        }
        let mut ring = HorizonRing::new(r.space(), (chunk_rows, max_chunks), horizon).map_err(LoadError::Invalid)?;
        (ring.base, ring.next) = <(u64, u64)>::load(r)?;
        ring.floors = Column::load(r)?;
        let live = r.count()?;
        within(live, to_usize(max_chunks), "a ring's live chunks")?;
        let per = to_usize(chunk_rows);
        let mut ordinal = ring.base;
        for i in 0..live {
            let mut h = ChunkHeader::load(r)?;
            within(to_usize(h.len), per, "a ring's chunk's rows")?;
            if h.first_ordinal != ordinal || h.len == 0 {
                return Err(LoadError::Invalid("a ring's chunks not in their ordinals' order".to_owned()));
            }
            ordinal += u64::from(h.len);
            h.at = ring.take_chunk();
            let span = to_usize(h.at) * per..to_usize(h.at) * per + to_usize(h.len);
            let end = to_usize(ring.made) * per;
            let (Some(d), Some(rw)) =
                (ring.days.slice_mut(end).get_mut(span.clone()), ring.rows.slice_mut(end).get_mut(span))
            else {
                return Err(LoadError::Invalid("a ring's chunk past its rows".to_owned()));
            };
            r.rows_into(Transform::Plain, d)?;
            r.rows_into(Transform::Plain, rw)?;
            ring.live += 1;
            ring.put_header(i, h);
        }
        if ordinal != ring.next {
            return Err(LoadError::Invalid("a ring's chunks not its ordinals".to_owned()));
        }
        Ok(ring)
    }
}
