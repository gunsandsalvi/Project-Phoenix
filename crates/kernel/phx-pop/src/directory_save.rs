//! The directory saved: every kind's slots, generations and live count, and its tombstones as one run, the day's and
//! the recent ones merged in; nothing is rebuilt.

use phx_id::Day;
use phx_store::{Backing, LoadError, Reader, Saved, Transform, Writer};

use crate::directory::{Directory, KindTable};
use crate::tombs::{Tomb, Tombs, merge_in};

impl<B: Backing> Saved for Directory<B> {
    fn save(&self, w: &mut Writer<'_>) {
        self.first.save(w);
        self.horizon.save(w);
        w.count(self.kinds.len());
        for t in &self.kinds {
            t.slots.save(w);
            t.generations.save(w);
            t.live.save(w);
        }
        let mut today = self.tombs.today.clone();
        today.sort_unstable_by_key(|t| t.key());
        let mut all = self.tombs.main.clone();
        merge_in(&mut all, &self.tombs.recent);
        let mut sorted = Vec::with_capacity(all.len() + today.len());
        sorted.extend_from_slice(&all);
        merge_in(&mut sorted, &today);
        w.rows(&sorted, Transform::Plain);
    }

    fn load(r: &mut Reader<'_>) -> Result<Directory<B>, LoadError> {
        let (first, horizon) = (Day::load(r)?, u32::load(r)?);
        let n = r.count()?;
        let mut kinds = Vec::with_capacity(n);
        for _ in 0..n {
            kinds.push(KindTable { slots: Saved::load(r)?, generations: Saved::load(r)?, live: u64::load(r)? });
        }
        let main: Vec<Tomb> = r.rows(Transform::Plain)?;
        Ok(Directory { kinds, tombs: Tombs::from_main(main), first, horizon })
    }
}
