//! A windowed group saved: its kind, width and blank, and each open window with its subject, slots, days and the rows
//! it has written; a closed window saves nothing.

use phx_id::Day;
use phx_store::{Backing, LoadError, Reader, Saved, Transform, Writer};

use crate::windowed::{Window, WindowedGroup};

impl<B: Backing> Saved for WindowedGroup<B> {
    fn save(&self, w: &mut Writer<'_>) {
        self.kind.save(w);
        self.width.save(w);
        w.rows(&self.blank, Transform::Plain);
        w.count(self.windows.capacity());
        w.count(self.windows.len());
        for x in &self.windows {
            (x.subject, x.first, x.slots, x.filled).save(w);
            (x.opened, x.closes).save(w);
            x.rows.save(w);
        }
    }

    fn load(r: &mut Reader<'_>) -> Result<WindowedGroup<B>, LoadError> {
        let (kind, width) = (u8::load(r)?, u16::load(r)?);
        let blank = r.rows(Transform::Plain)?;
        let (room, n) = (r.count()?, r.count()?);
        let mut windows = Vec::with_capacity(room);
        for _ in 0..n {
            let (subject, first, slots, filled) = <(u32, u32, u32, u32)>::load(r)?;
            let (opened, closes) = <(Day, Day)>::load(r)?;
            windows.push(Window { subject, first, slots, opened, closes, rows: Saved::load(r)?, filled });
        }
        Ok(WindowedGroup { kind, width, blank, windows })
    }
}
