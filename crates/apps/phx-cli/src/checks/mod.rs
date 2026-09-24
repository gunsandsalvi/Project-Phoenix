pub mod geo;
pub mod ledger;
pub mod stage0;

use phx_world::Inspector;

/// What a live check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(String),
    /// The check cannot run until what it reads exists; the step that brings it is named.
    NotYet(&'static str),
}

/// A live check: its permanent identity, what it holds, the step it holds from, and its function; a retired check
/// keeps its identity and says why.
#[derive(Clone, Copy, Debug)]
pub struct Check {
    pub id: &'static str,
    pub title: &'static str,
    pub from_step: &'static str,
    pub run: Option<fn(Inspector<'_>) -> Outcome>,
    pub retired: Option<&'static str>,
}

/// One check's metadata and function, or its retirement.
#[macro_export]
macro_rules! live_check {
    (id: $id:literal, title: $title:literal, from_step: $step:literal, check: $f:expr $(,)?) => {
        $crate::checks::Check { id: $id, title: $title, from_step: $step, run: Some($f), retired: None }
    };
    (id: $id:literal, title: $title:literal, from_step: $step:literal, retired: $why:literal $(,)?) => {
        $crate::checks::Check { id: $id, title: $title, from_step: $step, run: None, retired: Some($why) }
    };
}

/// Every live check, by identity; an identity once listed stays, retired with its reason.
pub const CHECKS: &[Check] = &[
    stage0::LC_0_01,
    stage0::LC_0_02,
    stage0::LC_0_03,
    stage0::LC_0_04,
    stage0::LC_0_05,
    stage0::LC_0_06,
    stage0::LC_0_07,
    stage0::LC_0_08,
    stage0::LC_0_09,
    stage0::LC_0_10,
    geo::LC_0_11,
    geo::LC_0_12,
    geo::LC_0_13,
    geo::LC_0_14,
    geo::LC_0_15,
    ledger::LC_0_16,
    ledger::LC_0_17,
    ledger::LC_0_18,
    ledger::LC_0_19,
    ledger::LC_0_20,
    ledger::LC_0_21,
    ledger::LC_0_22,
];
