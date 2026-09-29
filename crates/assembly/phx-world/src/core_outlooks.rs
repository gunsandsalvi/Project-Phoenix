//! The firms' outlooks of public series on the core. What a product last sold for in a region, its mark, is a public
//! series printed each day it trades. Every method — a heuristic of the menu at a memory type — forms its outlook of
//! the next print once for every firm using it, and scores each of its heuristics by its error in the method's widths.
//! A firm relies on one heuristic, its stance, reconsidered at each price review by the heuristics' recent performance
//! at its switching type's intensity and its own taste; its markup reads its stance's outlook of its product's mark.

use std::collections::BTreeMap;

use phx_id::{Day, PartyId};
use phx_macros::clause;
use phx_num::{Missing, violation};
use phx_rand::{Subject, SubjectTag};
use phx_val::heuristic::{HeuristicId, MENU, Params, Seen};

/// The heuristics on the menu.
pub const HEURISTICS: usize = MENU.len();

/// A memory type's view of a series: each heuristic's outlook of the next print and its performance, and the width
/// of the method's recent surprises; absent before the print that forms or scores them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MethodView {
    pub outlook: [Missing<f64>; HEURISTICS],
    pub performance: [Missing<f64>; HEURISTICS],
    pub width: Missing<f64>,
}

impl Default for MethodView {
    fn default() -> MethodView {
        MethodView {
            outlook: [Missing::Absent; HEURISTICS],
            performance: [Missing::Absent; HEURISTICS],
            width: Missing::Absent,
        }
    }
}

/// A public series as its methods saw it: the day of its last print, its last two prints, the mean of every print
/// since the opening as the level the anchor returns to, the prints counted, and each memory type's view.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    pub day: Day,
    pub last: f64,
    pub before: f64,
    pub level: f64,
    pub prints: u64,
    pub methods: Vec<MethodView>,
}

/// A day's stances: the firms relying on each heuristic after the day's reviews, the stances reconsidered and those
/// that changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StanceDay {
    pub day: u32,
    pub by_heuristic: [u64; HEURISTICS],
    pub reconsidered: u64,
    pub changed: u64,
}

/// The public series by product and region, and the days' stances.
#[derive(Clone, Debug, Default)]
pub struct Outlooks {
    pub series: BTreeMap<(u16, u32), Series>,
    pub days: Vec<StanceDay>,
}

/// What the methods read: each memory type's speed, the trend's and the anchor's parameters, the weight of the last
/// error in a performance.
#[derive(Clone, Copy, Debug)]
pub struct MethodParams<'a> {
    pub gains: &'a [f64],
    pub trend: f64,
    pub anchor: f64,
    pub performance_memory: f64,
}

/// A value that is there, or none.
fn present(m: Missing<f64>) -> Option<f64> {
    match m {
        Missing::Present(v) => Some(v),
        Missing::Absent => None,
    }
}

impl MethodParams<'_> {
    fn params(&self, memory: usize) -> Params {
        let Some(lambda) = self.gains.get(memory).copied() else {
            violation!(clause = "VAL.22", "a memory type beyond the types", memory = memory);
        };
        Params { lambda, gamma: self.trend, kappa: self.anchor }
    }
}

impl Outlooks {
    /// A series' print: every memory type's heuristics scored on it, their errors in the method's width, where they
    /// had an outlook of it, and each forming its outlook of the next; the first print, with no outlook before it,
    /// stands for the outlook it did not have.
    #[clause("VAL.3", "VAL.4", "VAL.5", "VAL.6", "VAL.23")]
    pub fn print(&mut self, key: (u16, u32), value: f64, day: Day, p: &MethodParams<'_>) {
        let types = p.gains.len();
        let s = self.series.entry(key).or_insert_with(|| Series {
            day,
            last: value,
            before: value,
            level: 0.0,
            prints: 0,
            methods: vec![MethodView::default(); types],
        });
        s.prints += 1;
        s.level += (value - s.level) / phx_rand::float::from_u64(s.prints);
        (s.before, s.last, s.day) = (s.last, value, day);
        for (memory, view) in s.methods.iter_mut().enumerate() {
            let params = p.params(memory);
            let errors: Vec<Option<f64>> = view.outlook.iter().map(|o| present(*o).map(|o| value - o)).collect();
            let scored: Vec<f64> = errors.iter().flatten().map(|e| e.abs()).collect();
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
                    level: s.level,
                    announced: Missing::Absent,
                    horizon_end: day,
                };
                let heuristic = HeuristicId::new(u8::try_from(h).unwrap_or(u8::MAX));
                *outlook = Missing::Present(heuristic.rule().outlook(&seen, &params));
            }
        }
    }

    /// A stance's outlook of a series' next print, where the series has printed.
    pub fn outlook(&self, key: (u16, u32), memory: usize, stance: usize) -> Missing<f64> {
        let Some(view) = self.series.get(&key).and_then(|s| s.methods.get(memory)) else { return Missing::Absent };
        view.outlook.get(stance).copied().unwrap_or(Missing::Absent)
    }

    /// A firm's stance reconsidered: the heuristics' shares at its switching intensity over their performance for its
    /// memory type on its series, equal where none is scored yet, and one chosen by its own taste.
    #[clause("VAL.7", "REP.22")]
    #[must_use]
    pub fn reconsider(
        &self,
        (key, memory, beta): ((u16, u32), usize, f64),
        (streams, stream): (&phx_core::Streams, &phx_core::StreamDecl),
        (party, day): (PartyId, Day),
    ) -> usize {
        let performance = self
            .series
            .get(&key)
            .and_then(|s| s.methods.get(memory))
            .map_or([Missing::Absent; HEURISTICS], |v| v.performance);
        let mut shares = [0.0; HEURISTICS];
        phx_val::switching::shares(&performance, beta, &mut shares);
        let mut d = streams.open(stream, Subject::new(SubjectTag::Party, party.get()), day, 0);
        let mut u = phx_rand::open_unit(&mut d);
        for (h, share) in shares.iter().enumerate() {
            if u < *share {
                return h;
            }
            u -= share;
        }
        HEURISTICS - 1
    }
}
