//! Outlooks of public series on the core. What a product last sold for in a region, its mark, is a public series
//! printed each day it trades, read by the firms; each country's consumer index, its change printed on its release
//! day, is read by the households. Every method — a heuristic of the menu at a memory type and an age class — forms its
//! outlook of the next print once for every party using it, and scores each of its heuristics by its error in the
//! method's widths. The anchor's level is the mean of the prints since the opening until two years have closed, then
//! for each age class the mean of the closed years' means weighted by the years its members have lived; an institution
//! no person holds has no age and keeps the prints' mean.
//! A party relies on one heuristic, its stance, reconsidered on its occasions — a firm's price review, a household's
//! spending — by the heuristics' recent performance at its switching type's intensity and its own taste. A firm's
//! markup reads its stance's outlook of its product's mark; a household's employed persons answer their pay rounds
//! from its stance's outlook of prices. Each method's lag behind a series' turning points is counted in prints, by
//! memory type and heuristic.
//! After a surprise wider than the attention sensitivity's widths, the days to the first price change of each firm
//! whose stance it bears on are kept by the surprise's size.

use std::collections::BTreeMap;

use phx_core::StreamDef;
use phx_core::slots::DaySlot;
use phx_id::{Day, PartyKey, PartyRef};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::Subject;
use phx_val::heuristic::{HeuristicId, MENU, Params, Seen};
use phx_val::types::Types;

/// The heuristics on the menu.
pub const HEURISTICS: usize = MENU.len();

/// A memory type's view of a series at an age class, or at none: each heuristic's outlook of the next print and its
/// performance, and the width of the method's recent surprises; absent before the print that forms or scores them.
#[derive(Clone, Copy, Debug, PartialEq, phx_macros::Saved)]
pub struct MethodView {
    pub outlook: [Missing<f64>; HEURISTICS],
    pub performance: [Missing<f64>; HEURISTICS],
    pub width: Missing<f64>,
    /// Each heuristic still to follow the series' last turn.
    pub behind: [bool; HEURISTICS],
}

impl Default for MethodView {
    fn default() -> MethodView {
        MethodView {
            outlook: [Missing::Absent; HEURISTICS],
            performance: [Missing::Absent; HEURISTICS],
            width: Missing::Absent,
            behind: [false; HEURISTICS],
        }
    }
}

/// A public series as its methods saw it: the day of its last print, its last two prints, the mean of every print
/// since the opening as the level the anchor returns to, the prints counted, each view, and its changes' squares; the
/// year it last printed in, that year's prints summed and counted, each closed year's mean, newest first, and each age
/// class's level from them once two years have closed.
#[derive(Clone, Debug, PartialEq, phx_macros::Saved)]
pub struct Series {
    pub day: Day,
    pub last: f64,
    pub before: f64,
    pub level: f64,
    pub prints: u64,
    pub methods: Vec<MethodView>,
    /// The way the series last moved, and the print it last turned at.
    pub direction: i8,
    pub turned: u64,
    /// The squares of each print's change over the one before, summed.
    pub squares: f64,
    pub year: i32,
    pub year_sum: f64,
    pub year_prints: u64,
    pub annual: Vec<f64>,
    pub experienced: Vec<Missing<f64>>,
}

/// A day's stances: the firms relying on each heuristic after the day's reviews, the stances reconsidered and those
/// that changed, and the firms a large surprise bore on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct StanceDay {
    pub day: u32,
    pub by_heuristic: [u64; HEURISTICS],
    pub reconsidered: u64,
    pub changed: u64,
    /// The firms a large surprise bore on today.
    pub surprised: u64,
}

/// The public series by product and region, the age classes and the years each one's members have lived, and the days'
/// stances.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub struct Outlooks {
    pub series: BTreeMap<(u16, u32), Series>,
    pub classes: usize,
    pub lived: Vec<Missing<u16>>,
    pub days: Vec<StanceDay>,
    /// By memory type and heuristic: the prints its outlooks took to turn after the series turned, summed, and the
    /// turns they followed.
    pub lags: BTreeMap<(usize, usize), (u64, u64)>,
    /// Today's surprises wider than the attention sensitivity's widths: the series, the memory type and heuristic, and
    /// the surprise over the print.
    pub surprised: Vec<((u16, u32), usize, usize, f64)>,
    /// Each surprised firm's surprise, its day and size, until its first price change after it; and each such
    /// change's surprise and the days it came after.
    pub awaiting: BTreeMap<u32, (Day, f64)>,
    pub responses: Vec<(f64, u32)>,
}

impl Series {
    /// A print's value counted in its year; a new year closes the last into its mean, and once two years have closed
    /// each age class's level is their means weighted by its lived years.
    #[clause("VAL.23")]
    fn enter_year(&mut self, (year, value): (i32, f64), lived: &[Missing<u16>], theta: f64) {
        if self.year != year && self.year_prints > 0 {
            self.annual.insert(0, self.year_sum / phx_rand::float::from_u64(self.year_prints));
            (self.year_sum, self.year_prints) = (0.0, 0);
            if self.annual.len() >= 2 {
                self.experienced = lived
                    .iter()
                    .map(|l| match l {
                        Missing::Present(l) => phx_val::experience::long_mean(&self.annual, u32::from(*l), theta),
                        Missing::Absent => Missing::Absent,
                    })
                    .collect();
            }
        }
        (self.year, self.year_sum, self.year_prints) = (year, self.year_sum + value, self.year_prints + 1);
    }

    /// The variance of its prints' changes over the ones before, none before it has two changes.
    #[must_use]
    pub fn volatility(&self) -> Option<f64> {
        (self.prints > 2).then(|| self.squares / phx_rand::float::from_u64(self.prints - 1))
    }
}

/// The way a change goes: up, down, or nowhere.
fn way(change: f64) -> i8 {
    if change > 0.0 {
        1
    } else if change < 0.0 {
        -1
    } else {
        0
    }
}

/// A value that is there, or none.
fn present(m: Missing<f64>) -> Option<f64> {
    match m {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

/// A memory type's parameters for every heuristic.
fn params(types: &Types, memory: usize) -> Params {
    let Some(lambda) = types.gains.get(memory).copied() else {
        violation!(clause = "VAL.22", "a memory type beyond the types", memory = memory);
    };
    Params { lambda, gamma: types.trend, kappa: types.anchor }
}

impl Outlooks {
    /// A series' print: every view's heuristics scored on it, their errors in the method's width, where they had an
    /// outlook of it, and each forming its outlook of the next; the first print, with no outlook before it, stands for
    /// the outlook it did not have. The first print of a year closes the last.
    #[clause("VAL.3", "VAL.4", "VAL.5", "VAL.6", "VAL.23")]
    pub fn print(&mut self, key: (u16, u32), value: f64, (day, year, sensitivity): (Day, i32, Option<f64>), p: &Types) {
        let s = self.series.entry(key).or_insert_with(|| Series {
            day,
            last: value,
            before: value,
            level: 0.0,
            prints: 0,
            methods: vec![MethodView::default(); p.views()],
            direction: 0,
            turned: 0,
            squares: 0.0,
            year,
            year_sum: 0.0,
            year_prints: 0,
            annual: Vec::new(),
            experienced: Vec::new(),
        });
        s.enter_year((year, value), &self.lived, p.theta);
        if s.prints > 0 && s.last > 0.0 {
            let change = (value - s.last) / s.last;
            s.squares += change * change;
        }
        s.prints += 1;
        s.level += (value - s.level) / phx_rand::float::from_u64(s.prints);
        let moved = way(value - s.last);
        (s.before, s.last, s.day) = (s.last, value, day);
        // A turn is a move against the last one; every method is then behind it until its outlook moves the new way.
        let turn = moved != 0 && s.direction != 0 && moved != s.direction;
        if moved != 0 {
            s.direction = moved;
        }
        if turn {
            s.turned = s.prints;
        }
        for (at, view) in s.methods.iter_mut().enumerate() {
            if turn {
                view.behind = [true; HEURISTICS];
            }
            let (memory, window) = p.of_view(at);
            let params = params(p, memory);
            // Before two years have closed every age class has lived the whole series, whose level is its prints'; so
            // has a class that lived none of the closed years.
            let level = match window {
                Missing::Present(w) => match s.experienced.get(w) {
                    Some(Missing::Present(m)) => *m,
                    _ => s.level,
                },
                Missing::Absent => s.level,
            };
            let errors: Vec<Option<f64>> = view.outlook.iter().map(|o| present(*o).map(|o| value - o)).collect();
            let scored: Vec<f64> = errors.iter().flatten().map(|e| e.abs()).collect();
            if let (Missing::Present(width), Some(s)) = (view.width, sensitivity)
                && width > 0.0
            {
                for (h, e) in errors.iter().enumerate() {
                    if let Some(e) = e
                        && phx_val::surprise::wakes(*e, width, s)
                    {
                        self.surprised.push((key, at, h, e.abs() / value.abs()));
                    }
                }
            }
            if !scored.is_empty() {
                let mean =
                    scored.iter().sum::<f64>() / phx_rand::float::from_u64(phx_rand::float::len_u64(scored.len()));
                let width = phx_val::surprise::width(view.width, mean, params.lambda);
                view.width = Missing::Present(width);
                for (perf, error) in view.performance.iter_mut().zip(&errors) {
                    let Some(e) = error else { continue };
                    // A width of nothing is a method none of whose heuristics has missed.
                    let in_widths = if width > 0.0 { e / width } else { 0.0 };
                    *perf = Missing::Present(phx_val::switching::performance(*perf, in_widths, p.performance_memory));
                }
            }
            for (h, outlook) in view.outlook.iter_mut().enumerate() {
                let previous = present(*outlook).unwrap_or(value);
                let seen = Seen {
                    previous,
                    last: s.last,
                    before: s.before,
                    level,
                    announced: Missing::Absent,
                    horizon_end: day,
                };
                let heuristic = HeuristicId::new(u8::try_from(h).unwrap_or(u8::MAX));
                let next = heuristic.rule().outlook(&seen, &params);
                // The lag behind a turn is counted by memory type on the view of no age.
                if let Some(behind) = view.behind.get_mut(h)
                    && window == Missing::Absent
                    && *behind
                    && present(*outlook).is_some_and(|o| way(next - o) == s.direction)
                {
                    *behind = false;
                    let lag = self.lags.entry((memory, h)).or_insert((0, 0));
                    *lag = (lag.0 + (s.prints - s.turned), lag.1 + 1);
                }
                *outlook = Missing::Present(next);
            }
        }
    }

    /// The view of a decider's preferences: its memory type at its age class; none where it holds no memory type.
    #[must_use]
    pub fn view(&self, prefs: &phx_core::Prefs) -> Option<usize> {
        match prefs.memory {
            Missing::Present(m) => Some(phx_val::types::view(self.classes, m, prefs.window)),
            Missing::Absent => None,
        }
    }

    /// A stance's outlook of a series' next print at a view, where the series has printed.
    pub fn outlook(&self, key: (u16, u32), view: usize, stance: usize) -> Missing<f64> {
        let Some(view) = self.series.get(&key).and_then(|s| s.methods.get(view)) else { return Missing::Absent };
        view.outlook.get(stance).copied().unwrap_or(Missing::Absent)
    }

    /// What a party's stance is reconsidered over: the heuristics' performance at its view of its series, none where
    /// none is scored yet, its switching intensity, and its taste drawn from its own stream.
    #[clause("VAL.7", "REP.22")]
    #[must_use]
    pub fn stance_in(
        &self,
        (key, view, beta): ((u16, u32), usize, f64),
        (streams, stream): (&phx_core::WorldStreams, &phx_core::StreamDecl),
        (party, day): (PartyRef, Day),
    ) -> phx_val::switching::StanceIn {
        let performance = self
            .series
            .get(&key)
            .and_then(|s| s.methods.get(view))
            .map_or([Missing::Absent; HEURISTICS], |v| v.performance);
        let mut d = streams.open_at(stream, Subject::from(party), day, DaySlot::S5b.ordinal());
        phx_val::switching::StanceIn { performance, intensity: beta, taste: phx_rand::open_unit(&mut d) }
    }
}

impl crate::core::Core {
    /// Each decider's age class at a year's close, and at the opening: a household's its head's, a working owner's its
    /// own, the offices it holds carrying it; and each class's lived years, the mean age of the household heads in it,
    /// by which the public series weight their closed years.
    #[clause("VAL.23")]
    pub(crate) fn refresh_windows(&mut self, types: &Types, date: phx_id::Date) {
        let mut ages = vec![(0_u64, 0_u64); types.windows.len()];
        let head_role = self.declared.household.as_ref().and_then(|d| d.role(if_pop::HEAD.name));
        if let (Some(place), Some(head_role)) = (self.bound.kinds.household, head_role) {
            let mut windows = std::mem::take(&mut self.work.windows);
            windows.clear();
            if let Some(Some(ps)) = self.persons.get(place) {
                for slot in self.directory.live_slots(crate::core::kind_number(place)) {
                    // Its head's word read for its role and birth alone; the first person where none heads it.
                    let mut words = ps.of(slot).map(|x| phx_pop::person::role_and_birth(x.word));
                    let first = words.next();
                    let head = first.filter(|(r, _)| *r == head_role).or_else(|| words.find(|(r, _)| *r == head_role));
                    let Some((_, born)) = head.or(first) else { continue };
                    let Ok(age) = u32::try_from(phx_core::pop_process::age_on(born, date)) else { continue };
                    let window = types.window_of(age);
                    if let Missing::Present(c) = window
                        && let Some(a) = ages.get_mut(usize::from(c))
                    {
                        *a = (a.0 + u64::from(age), a.1 + 1);
                    }
                    let window = match window {
                        Missing::Present(c) => match u8::try_from(c) {
                            Ok(c) => Missing::Present(c),
                            Err(_) => violation!(clause = "VAL.23", "an age class past its word", class = c),
                        },
                        Missing::Absent => Missing::Absent,
                    };
                    windows.push((slot, window));
                }
            }
            self.household_write(|hs| {
                for (slot, window) in &windows {
                    hs.set_window(*slot, *window);
                }
            });
            self.work.windows = windows;
        }
        let lived: Vec<Missing<u16>> = ages
            .iter()
            .map(|(sum, n)| match sum.checked_div(*n).and_then(|m| u16::try_from(m).ok()) {
                Some(m) => Missing::Present(m),
                None => Missing::Absent,
            })
            .collect();
        for o in [&mut self.goods.outlooks, &mut self.stats.outlooks] {
            (o.classes, o.lived) = (types.windows.len(), lived.clone());
        }
        let working: Vec<(PartyKey, usize, PartyKey, u64)> = self
            .owners
            .working
            .iter()
            .flat_map(|(firm, ws)| ws.iter().enumerate().map(|(i, w)| (*firm, i, w.household, w.person)))
            .collect();
        for (firm, i, household, person) in working {
            let window = match self.age_of((household, person), date) {
                Some(age) => types.window_of(age),
                None => Missing::Absent,
            };
            if let Some(w) = self.owners.working.get_mut(&firm).and_then(|ws| ws.get_mut(i)) {
                w.window = window;
            }
            if i == 0 {
                self.decisions.set_window(firm, person, window);
            }
        }
    }

    /// A household's stance reconsidered on its occasion, over its country's consumer index as published.
    #[clause("VAL.7", "MND.20")]
    pub(crate) fn reconsider_household(
        &mut self,
        (streams, types): (&phx_core::WorldStreams, &Types),
        (household, id): (phx_id::PartyKey, PartyRef),
        (country, day): (u8, Day),
    ) {
        let Some(stream) = streams.named(sys_hh::StanceStream::DECL.name) else {
            violation!(clause = "VAL.7", "the households' stance stream is not declared");
        };
        let reconsidering = self.point(|p| p.household_stance, &sys_hh::points::STANCE);
        let (_, prefs) = self.decider(reconsidering, household);
        let (Some(view), Missing::Present(switching), Missing::Present(stance)) =
            (self.stats.outlooks.view(&prefs), prefs.switching, prefs.stance)
        else {
            return;
        };
        let Some(beta) = types.intensities.get(usize::from(switching)).copied() else {
            violation!(
                clause = "VAL.7",
                "a household's switching type beyond the types",
                slot = household.slot().get()
            );
        };
        let key = crate::core_stats::cpi_series(country);
        let chosen = self.decide_own(reconsidering, household, |_| {
            self.stats.outlooks.stance_in((key, view, beta), (streams, &stream), (id, day))
        });
        let Some(chosen) = chosen else { return };
        if let Ok(chosen) = u16::try_from(chosen)
            && chosen != stance
        {
            self.set_stance(reconsidering, household, chosen);
        }
    }

    /// The price level a household expects `months` months on over today's: its stance's outlook of its country's
    /// consumer index's monthly change, compounded; prices held before the index's first change is published, the
    /// opening's present prices being all it has seen.
    #[clause("VAL.10", "VAL.23")]
    pub(crate) fn price_outlook(&self, prefs: &phx_core::Prefs, country: u8, months: u32) -> f64 {
        let (Some(view), Missing::Present(stance)) = (self.stats.outlooks.view(prefs), prefs.stance) else {
            return 1.0;
        };
        match self.stats.outlooks.outlook(crate::core_stats::cpi_series(country), view, usize::from(stance)) {
            Missing::Present(change) => (0..months).fold(1.0, |level, _| level * (1.0 + change)),
            Missing::Absent => 1.0,
        }
    }
}
