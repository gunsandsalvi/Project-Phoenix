//! The world's day on the core: the stage table walked slot by slot, each slot's calls into the core made in its
//! place; a turn runs every day up to the next business day of some country.

use phx_core::QueuedIntent;
use phx_exec::Clock;
use phx_id::Day;
use phx_macros::clause;

use phx_core::stages::Mode;

use crate::metrics::TurnRecord;
use crate::world::World;
use route::Call;

mod route;

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
    /// time, as the phone's views are. The player's intents are queued first, each taken on the first day its
    /// decision comes for the player's household.
    #[clause("TIME.6", "N8.2", "Law 17")]
    pub fn run_turn_observed(
        &mut self,
        intents: &[QueuedIntent],
        clock: &dyn Clock,
        mut observer: Option<&mut dyn crate::observe::Observer>,
    ) -> TurnRecord {
        let start = clock.now_ns();
        self.core.queue(intents, self.today);
        let first = self.today.succ();
        let last = self.calendar.next_turn_day(self.today);
        let mut days = 0_u32;
        loop {
            let day = self.today.succ();
            self.run_day(day, clock);
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

    /// The core's day: the stage table walked, each slot that runs today routed to the calls that fill it, in the
    /// table's order, so each reads only what the slots before it wrote.
    #[clause("TIME.6", "TIME.8", "TIME.10")]
    fn run_day(&mut self, day: Day, clock: &dyn Clock) {
        let start = clock.now_ns();
        phx_exec::trace::note("day", &[("day", i64::from(day.get()))]);
        let any_business = self.calendar.any_business(day);
        let t = Some(clock);
        let geo = crate::world::geo_arc(&self.own, self.own_at);
        let hh = crate::world::own_at::<sys_hh::Own>(&self.own, self.own_at.hh);
        let table = std::mem::take(&mut self.stages);
        for slot in table.walk(Mode::Ordinary, any_business) {
            for call in route::calls(slot.slot).iter().copied().filter(|c| route::acts(*c, any_business)) {
                let ctx = crate::core_pop::Ctx {
                    register: &self.register,
                    calendar: &self.calendar,
                    streams: &self.streams,
                    processes: &self.processes,
                    regions: &self.regions,
                    pool: self.pool.as_ref(),
                };
                match call {
                    Call::Weather => {
                        self.core.timed(t, "weather", |c| c.weather_day(geo, (&self.streams, &self.calendar), day));
                    }
                    Call::Rates => self.core.timed(t, "rates", |c| c.measure_rates(&ctx, day)),
                    Call::Hazards => {
                        let pop_day = self.core.timed(t, "hazards", |c| c.run_hazards(&ctx, day));
                        pop_day.note();
                        self.core.pop_days.push((day, pop_day));
                        let events = std::mem::take(&mut self.core.events_today);
                        self.core.events.push((day, events));
                    }
                    Call::Windows => {
                        let date = self.calendar.date(day);
                        if let (1, 1, Some(types)) = (date.month(), date.day(), hh.map(|h| &h.types)) {
                            self.core.refresh_windows(types, date);
                        }
                    }
                    Call::Labour => {
                        let Some(kind) = self.labour.as_ref() else { continue };
                        let lctx = crate::core_labour::LabourCtx {
                            register: &self.register,
                            calendar: &self.calendar,
                            streams: &self.streams,
                            kind,
                            regions: &self.regions,
                            clock: t,
                            pool: self.pool.as_ref(),
                        };
                        let _ = self.core.timed(t, "labour", |c| c.labour_day(&lctx, day));
                    }
                    Call::Goods | Call::Freight => {
                        let frm = crate::world::own_at::<sys_frm::Own>(&self.own, self.own_at.frm);
                        let (Some(rule), Some(frm), Some(weights)) = (hh, frm, self.weights) else { continue };
                        let gctx = crate::core_goods::GoodsCtx {
                            register: &self.register,
                            calendar: &self.calendar,
                            streams: &self.streams,
                            rule,
                            management: frm.management(),
                            regions: &self.regions,
                            weights,
                            pool: self.pool.as_ref(),
                            clock: t,
                        };
                        if call == Call::Goods {
                            self.core.timed(t, "goods", |c| c.goods_day(&gctx, day)).note();
                        } else {
                            self.core.timed(t, "freight", |c| c.ship(&gctx, geo, day));
                        }
                    }
                    Call::SettleClosed | Call::Settle => {
                        let (calendar, streams, pool) = (&self.calendar, &self.streams, self.pool.as_ref());
                        let settled = self.core.timed(t, "settle", |c| {
                            c.run_day((day, calendar, streams, &crate::opening::prims::SETTLE_ORDER), (t, pool))
                        });
                        settled.note();
                    }
                    Call::Publish => {
                        let _ = self.core.happened.publish(day, &self.news);
                    }
                    Call::Audit => self.core.timed(t, "audit", |c| c.audit(day)),
                    Call::Statistics => {
                        let (calendar, regions) = (&self.calendar, &self.regions);
                        self.core.timed(t, "statistics", |c| {
                            c.stats_day(day, (calendar, (regions, geo)), hh.map(|h| &h.types));
                        });
                    }
                    Call::Player => self.core.timed(t, "player", |c| c.player_day(day)),
                }
            }
        }
        self.stages = table;
        self.close_day(day, clock.now_ns().checked_sub(start));
    }

    /// The day's stage timings kept, its findings recorded and its time set on its record.
    fn close_day(&mut self, day: Day, ns: Option<u64>) {
        let stages = std::mem::take(&mut self.core.stage_ns);
        self.core.timings.push((day, stages));
        for f in self.core.found.drain(..) {
            self.findings.record(f);
        }
        if let (Some(ns), Some(last)) = (ns, self.core.days.last_mut()) {
            last.ns = ns;
        }
    }
}
