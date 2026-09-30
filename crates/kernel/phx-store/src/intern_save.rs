//! An interner saved as its rows, values, slots and the ids listed today; its index left out and laid again from the
//! rows at a load.

use crate::Transform;
use crate::backing::Backing;
use crate::column::Column;
use crate::convert::to_usize;
use crate::rebuild::{RebuildError, Rebuilt};
use crate::region::Region;
use crate::save::{LoadError, Reader, Saved, Writer};
use crate::table::SlotAlloc;

use super::{Interner, LISTED, LIVE, Probe, Reuse};

/// A declared reuse as the byte a save keeps.
fn reuse_byte(reuse: Reuse) -> u8 {
    match reuse {
        Reuse::AfterClose => 0,
        Reuse::Never => 1,
    }
}

impl<B: Backing> Saved for Interner<B> {
    fn save(&self, w: &mut Writer<'_>) {
        reuse_byte(self.reuse).save(w);
        self.max_ids.save(w);
        self.rows.save(w);
        self.bytes.save_prefix(self.used, w, Transform::Plain);
        self.dead.save(w);
        self.slots.save(w);
        self.listed.save(w);
        (self.live, self.ever).save(w);
    }

    fn load(r: &mut Reader<'_>) -> Result<Interner<B>, LoadError> {
        let reuse = match u8::load(r)? {
            0 => Reuse::AfterClose,
            1 => Reuse::Never,
            _ => return Err(LoadError::Invalid("an interner's reuse unknown".to_owned())),
        };
        let max_ids = u32::load(r)?;
        let rows: Column<super::Row, B> = Column::load(r)?;
        let (bytes, used) = Region::<u8, B>::load_prefix(r, Transform::Plain)?;
        let dead = usize::load(r)?;
        let slots = SlotAlloc::load(r)?;
        let listed = Column::load(r)?;
        let (live, ever) = <(u64, u64)>::load(r)?;
        let held = |row: &&super::Row| row.state == LIVE || row.state == LISTED;
        let outside = rows.slice().iter().filter(held).any(|row| to_usize(row.offset) + to_usize(row.len) > used);
        if rows.len() > to_usize(max_ids) || dead > used || outside {
            return Err(LoadError::Invalid("an interner whose rows and values disagree".to_owned()));
        }
        let scratch = Region::reserve(r.space(), bytes.capacity());
        let index = Probe::new(r.space(), max_ids);
        Ok(Interner { reuse, max_ids, rows, bytes, used, dead, scratch, slots, listed, index, live, ever })
    }

    /// The index, left out of the save, laid again from the rows in slot order.
    fn rebuild_derived(&mut self, out: &mut Rebuilt) -> Result<(), RebuildError> {
        let n = self.rebuild_index();
        out.record("Interner.index", n);
        Ok(())
    }
}

/// An interner's saved bytes, the one account of everything it keeps but its index.
fn saved<B: Backing>(i: &Interner<B>) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Ok(mut w) = Writer::new(&mut bytes) {
        i.save(&mut w);
        let _ = w.finish();
    }
    bytes
}

impl<B: Backing> PartialEq for Interner<B> {
    /// Two interners are equal when they save the same bytes and every value they hold finds the same id.
    fn eq(&self, other: &Interner<B>) -> bool {
        saved(self) == saved(other)
            && (0..self.rows.len()).all(|slot| {
                let row = self.rows.slice().get(slot);
                match row {
                    Some(r) if r.state == LIVE || r.state == LISTED => {
                        other.find(self.value(*r)) == self.find(self.value(*r))
                    }
                    _ => true,
                }
            })
    }
}
