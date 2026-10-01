//! A kind's store saved: its kind, its blank rows, and each group's width and rows raw; the indexes on its
//! words are left out and kept again after the load from its live slots in slot order.

use phx_store::{Backing, LoadError, Reader, Saved, Writer};

use crate::kinds::KindStore;

impl<B: Backing> Saved for KindStore<B> {
    fn save(&self, w: &mut Writer<'_>) {
        self.kind.save(w);
        self.rows.save(w);
        w.count(self.widths.len());
        w.rows(&self.blanks, phx_store::Transform::Plain);
        for (width, col) in self.widths.iter().zip(&self.groups) {
            width.save(w);
            col.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<KindStore<B>, LoadError> {
        let (kind, rows) = (u8::load(r)?, u32::load(r)?);
        let n = r.count()?;
        let blanks = r.rows(phx_store::Transform::Plain)?;
        let (mut widths, mut groups) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for _ in 0..n {
            widths.push(u16::load(r)?);
            groups.push(Saved::load(r)?);
        }
        // The indexes are kept again from the live slots once the directory is loaded.
        Ok(KindStore { kind, widths, blanks, groups, rows, keyed: Vec::new() })
    }
}
