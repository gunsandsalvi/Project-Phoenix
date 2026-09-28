//! The world's day on the core: chance on the households' persons, labour's round, goods and money, then the day's
//! dues and settlement; a turn runs every day up to the next business day of some country.

use phx_core::QueuedIntent;
use phx_exec::Clock;
use phx_id::Day;
use phx_macros::clause;

use crate::metrics::TurnRecord;
use crate::world::World;

impl World {
    /// The last day run.
    pub fn today(&self) -> Day {
        self.today
    }

    /// Runs a turn with no observer.
    pub fn run_turn(&mut self, intents: &[QueuedIntent], clock: &dyn Clock) -> TurnRecord {
        self.run_turn_observed(intents, clock, None)
    }

    /// Runs a turn: every day from the day after the last turn up to the next day that is a business day in some
    /// country, each as its own day, the observer reading each once it has ended; its reading is inside the turn's
    /// time, as the phone's views are. The player's intents wait for the player's party on the core.
    #[clause("TIME.6", "N8.2", "Law 17")]
    pub fn run_turn_observed(
        &mut self,
        _intents: &[QueuedIntent],
        clock: &dyn Clock,
        mut observer: Option<&mut dyn crate::observe::Observer>,
    ) -> TurnRecord {
        let start = clock.now_ns();
        let first = self.today.succ();
        let last = self.calendar.next_turn_day(self.today);
        let mut days = 0_u32;
        loop {
            let day = self.today.succ();
            self.core_day(day, clock);
            self.today = day;
            let date = self.calendar.date(day);
            if date.month() == 1 && date.day() == 1 {
                self.calendar.move_window(date.year());
            }
            if let Some(o) = observer.as_deref_mut() {
                o.day_closed(crate::Inspector::new(self));
            }
            days += 1;
            if day == last {
                break;
            }
        }
        let record = TurnRecord { first, last, days, wall_ns: clock.now_ns().checked_sub(start) };
        self.metrics.turns.push(record);
        record
    }

    /// The core's day: chance on its persons, labour's round, goods, then its dues and settlement.
    fn core_day(&mut self, day: Day, clock: &dyn Clock) {
        let start = clock.now_ns();
        let regions = self.regions.clone();
        let ctx = crate::core_pop::Ctx {
            register: &self.register,
            calendar: &self.calendar,
            streams: &self.streams,
            processes: &self.processes,
            regions: &regions,
        };
        let pop_day = self.core.run_hazards(&ctx, day);
        self.core.pop_days.push((day, pop_day));
        if let Some(kind) = self.labour.as_ref() {
            let lctx = crate::core_labour::LabourCtx {
                register: &self.register,
                calendar: &self.calendar,
                streams: &self.streams,
                kind,
                regions: &regions,
            };
            let _ = self.core.labour_day(&lctx, day);
        }
        let own_of = |code: &str| self.own.iter().find(|(c, _)| *c == code).map(|(_, s)| s);
        let hh = own_of(<sys_hh::Hh as phx_core::System>::CODE).and_then(|s| s.downcast_ref::<sys_hh::Own>());
        let frm = own_of(<sys_frm::Frm as phx_core::System>::CODE).and_then(|s| s.downcast_ref::<sys_frm::Own>());
        if let (Some(rule), Some(frm), Ok(weights)) = (hh, frm, crate::registry::retail_weights(&self.register)) {
            let gctx = crate::core_goods::GoodsCtx {
                register: &self.register,
                calendar: &self.calendar,
                streams: &self.streams,
                rule,
                management: frm.management(),
                regions: &regions,
                weights,
            };
            let _ = self.core.goods_day(&gctx, day);
        }
        let _ = self.core.run_day(day, &self.calendar, &self.streams, &crate::opening::prims::SETTLE_ORDER);
        self.core.audit(day);
        self.core.stats_day(day, &self.calendar, &regions);
        for f in self.core.found.drain(..) {
            self.findings.record(f);
        }
        if let (Some(ns), Some(last)) = (clock.now_ns().checked_sub(start), self.core.days.last_mut()) {
            last.ns = ns;
        }
    }
}
