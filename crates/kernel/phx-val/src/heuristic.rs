//! The heuristic menu as one sealed trait: no crate but this one can add a way of forecasting, so every outlook in
//! the world is formed by a rule on the declared menu.

use phx_id::Day;
use phx_macros::clause;
use phx_num::Missing;

use crate::heuristics::{self, Announced};

mod menu {
    pub trait OnTheMenu {}
}

/// What a method saw of a series when it forms its next outlook: its own last outlook, the last two values published,
/// the level it anchors to, and a dated change announced, if any.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub previous: f64,
    pub last: f64,
    pub before: f64,
    pub level: f64,
    pub announced: Missing<Announced>,
    pub horizon_end: Day,
}

/// A memory type's parameters for every heuristic: its speed of correction, its extrapolation and its pull to the
/// anchor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    pub lambda: f64,
    pub gamma: f64,
    pub kappa: f64,
}

/// A rule on the menu.
#[clause("VAL.3", "VAL.6", "VAL.22")]
pub trait Heuristic: menu::OnTheMenu + Sync + std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn outlook(&self, seen: &Seen, p: &Params) -> f64;
}

#[derive(Debug)]
pub struct Adaptive;
#[derive(Debug)]
pub struct Trend;
#[derive(Debug)]
pub struct Anchor;
/// With nothing announced the last value published stands, since the rule forecasts only the dated changes it reads.
#[derive(Debug)]
pub struct Announcement;

impl menu::OnTheMenu for Adaptive {}
impl menu::OnTheMenu for Trend {}
impl menu::OnTheMenu for Anchor {}
impl menu::OnTheMenu for Announcement {}

impl Heuristic for Adaptive {
    fn name(&self) -> &'static str {
        "adaptive"
    }
    fn outlook(&self, seen: &Seen, p: &Params) -> f64 {
        heuristics::adaptive(seen.previous, seen.last, p.lambda)
    }
}

impl Heuristic for Trend {
    fn name(&self) -> &'static str {
        "trend"
    }
    fn outlook(&self, seen: &Seen, p: &Params) -> f64 {
        heuristics::trend(seen.last, seen.before, p.gamma)
    }
}

impl Heuristic for Anchor {
    fn name(&self) -> &'static str {
        "anchor"
    }
    fn outlook(&self, seen: &Seen, p: &Params) -> f64 {
        heuristics::anchor(seen.last, seen.level, p.kappa)
    }
}

impl Heuristic for Announcement {
    fn name(&self) -> &'static str {
        "announcement"
    }
    fn outlook(&self, seen: &Seen, _: &Params) -> f64 {
        match seen.announced {
            Missing::Present(a) => heuristics::announcement(seen.last, a, seen.horizon_end),
            Missing::Absent => seen.last,
        }
    }
}

/// The menu, in the order a heuristic's index names it.
pub const MENU: [&dyn Heuristic; 4] = [&Adaptive, &Trend, &Anchor, &Announcement];

/// A heuristic on the menu by its index.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeuristicId(u8);

impl HeuristicId {
    /// The menu's entry at `index`; an index past the menu names no rule.
    pub fn new(index: u8) -> HeuristicId {
        if usize::from(index) >= MENU.len() {
            phx_num::violation!(clause = "VAL.6", "a heuristic not on the menu", index = index);
        }
        HeuristicId(index)
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }

    #[must_use]
    pub fn rule(self) -> &'static dyn Heuristic {
        let Some(h) = MENU.get(usize::from(self.0)) else {
            phx_num::violation!(clause = "VAL.6", "a heuristic not on the menu", index = self.0);
        };
        *h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_forms_each_rule() {
        let seen = Seen {
            previous: 1.0,
            last: 2.0,
            before: 1.5,
            level: 1.0,
            announced: Missing::Present(Announced { effective: Day::new(5), level: 3.0 }),
            horizon_end: Day::new(6),
        };
        let p = Params { lambda: 0.5, gamma: 1.0, kappa: 0.5 };
        let got: Vec<f64> = (0..4).map(|i| HeuristicId::new(i).rule().outlook(&seen, &p)).collect();
        assert_eq!(got, vec![1.5, 2.5, 1.5, 3.0]);
        let names: Vec<&str> = MENU.iter().map(|h| h.name()).collect();
        assert_eq!(names, vec!["adaptive", "trend", "anchor", "announcement"]);
    }

    #[test]
    fn nothing_announced_keeps_the_last_value() {
        let seen = Seen {
            previous: 0.0,
            last: 2.0,
            before: 0.0,
            level: 0.0,
            announced: Missing::Absent,
            horizon_end: Day::new(1),
        };
        let p = Params { lambda: 0.5, gamma: 1.0, kappa: 0.5 };
        assert!((Announcement.outlook(&seen, &p) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn an_index_past_the_menu_is_refused() {
        assert_eq!(crate::testing::refused(|| HeuristicId::new(4)), Some("VAL.6"));
    }
}
