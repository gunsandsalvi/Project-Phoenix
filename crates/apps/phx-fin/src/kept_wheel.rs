//! Today's due wheel (`phx_core::wheel::DueWheel`): the design point's contracts filed over its days, and a day's
//! dues taken and each filed again at its next date.

use std::collections::BTreeMap;

use phx_core::wheel::DueWheel;
use phx_exec::Pool;
use phx_id::Day;
use phx_rand::uniform::below_u64;

use crate::FinError;
use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{BASE, count, index, slots, wide};
use crate::measure::Measures;

/// The wheel, the day it is at, and the contract numbers a day's dues are given past those filed.
#[derive(Debug, Default)]
pub struct Wheel {
    due: Option<DueWheel>,
    today: u32,
    horizon: u32,
    rows: u64,
    taken: Vec<u32>,
    filed: u64,
}

impl Wheel {
    /// `[store] wheel_rows` contracts filed over the wheel's later half and `wheel_far` beyond it, each on a day drawn
    /// for it, so the days measured take exactly the dues they are given.
    ///
    /// # Errors
    /// A design point without the wheel's counts.
    pub fn fill(&mut self, design: &Design, streams: &Streams) -> Result<u64, FinError> {
        let rows = count(&design.store, "wheel_rows", "store")?;
        let far = count(&design.store, "wheel_far", "store")?;
        let horizon = slots(count(&design.store, "wheel_days", "store")?)?;
        let mut wheel = DueWheel::new(Day::new(0), horizon);
        let mut d = streams.draws("kept.wheel", 0, 0);
        // Filed in blocks a day at a time, as a day's new contracts are.
        let mut by_day: Vec<Vec<u32>> = vec![Vec::new(); index(u64::from(horizon))?];
        for edge in 0..rows {
            let half = u64::from(horizon / 2);
            let day = half + below_u64(&mut d, u64::from(horizon) - half);
            if let Some(v) = by_day.get_mut(index(day)?) {
                v.push(slots(edge)?);
            }
        }
        for (day, edges) in (0_u32..).zip(&by_day) {
            wheel.schedule_all(edges, Day::new(day));
        }
        for edge in rows..rows + far {
            let day = u64::from(horizon) + below_u64(&mut d, u64::from(horizon));
            wheel.schedule(slots(edge)?, Day::new(slots(day)?));
        }
        // The wheel turns a day at a time from its first; the opening day holds nothing, and the days measured follow.
        wheel.take(Day::new(0), &mut self.taken, None);
        (self.due, self.today, self.horizon, self.rows, self.filed) = (Some(wheel), 0, horizon, rows + far, rows + far);
        Ok(rows + far)
    }

    /// The next day's dues taken, `[day.*] dues` of them filed on it beyond those the fill filed there, and each
    /// taken filed again at a date past the days measured.
    ///
    /// # Errors
    /// The wheel taken before its fill, or a day's dues not all taken.
    pub fn day(
        &mut self,
        counts: &BTreeMap<String, u64>,
        m: &mut Measures<'_>,
        pool: Option<&Pool>,
    ) -> Result<(), FinError> {
        // A day that takes no dues, as a closed day does not, leaves the wheel where it is.
        let Some(&dues) = counts.get("dues") else { return Ok(()) };
        let Some(wheel) = self.due.as_mut() else { return Err(FinError("the wheel taken before its fill".to_owned())) };
        self.today += 1;
        let today = Day::new(self.today);
        let fresh: Vec<u32> = (self.filed..self.filed + dues).map(slots).collect::<Result<_, _>>()?;
        self.filed += dues;
        wheel.schedule_all(&fresh, today);
        let next = Day::new(self.today + self.horizon - 1);
        let taken = &mut self.taken;
        m.read(BASE, "due", dues, || {
            wheel.take(today, taken, pool);
            wheel.schedule_all(taken, next);
        });
        if u64::try_from(taken.len()).ok() != Some(dues) {
            return Err(FinError(format!("the wheel took {} dues of {dues}", taken.len())));
        }
        Ok(())
    }

    /// A digest of a day's dues, the same for any workers.
    #[must_use]
    pub fn digest(&self) -> u64 {
        self.taken.iter().fold(u64::from(self.today), |a, e| phx_exec::mix::mix64(a ^ u64::from(*e)))
    }

    /// The bytes the wheel's entries hold, a word each.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        self.rows * wide(size_of::<u32>())
    }
}
