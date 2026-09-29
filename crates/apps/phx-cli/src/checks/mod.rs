pub mod awaiting;
pub mod core;
pub mod geo;
pub mod lives;

use phx_world::Inspector;

/// What a live check found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(String),
    /// The check cannot run until what it reads exists; the step that brings it is named.
    NotYet(&'static str),
}

/// What the observer recorded beside the world over the run: the declared reads and their series, and each opening
/// distribution's distance from the world's own at settling's end and at the run's end.
#[derive(Clone, Copy, Debug)]
pub struct Observed<'a> {
    pub reads: &'a [phx_obs::ReadDecl],
    pub series: &'a [phx_obs::Series],
    pub settled: &'a [phx_obs::Drift],
    pub ended: &'a [phx_obs::Drift],
}

/// What a check reads: the world alone, or the world with what the observer recorded of it.
#[derive(Clone, Copy, Debug)]
pub enum Run {
    World(fn(Inspector<'_>) -> Outcome),
    Observed(fn(Inspector<'_>, &Observed<'_>) -> Outcome),
}

/// A live check: its permanent identity, what it holds, the step it holds from, and its function; a retired check
/// keeps its identity and says why.
#[derive(Clone, Copy, Debug)]
pub struct Check {
    pub id: &'static str,
    pub title: &'static str,
    pub from_step: &'static str,
    pub run: Option<Run>,
    pub retired: Option<&'static str>,
}

/// One check's metadata and function, or its retirement.
#[macro_export]
macro_rules! live_check {
    (id: $id:literal, title: $title:literal, from_step: $step:literal, check: $f:expr $(,)?) => {
        $crate::checks::Check {
            id: $id,
            title: $title,
            from_step: $step,
            run: Some($crate::checks::Run::World($f)),
            retired: None,
        }
    };
    (id: $id:literal, title: $title:literal, from_step: $step:literal, observed: $f:expr $(,)?) => {
        $crate::checks::Check {
            id: $id,
            title: $title,
            from_step: $step,
            run: Some($crate::checks::Run::Observed($f)),
            retired: None,
        }
    };
    (id: $id:literal, title: $title:literal, from_step: $step:literal, retired: $why:literal $(,)?) => {
        $crate::checks::Check { id: $id, title: $title, from_step: $step, run: None, retired: Some($why) }
    };
}

/// Every live check, by identity; an identity once listed stays, retired with its reason.
pub const CHECKS: &[Check] = &[
    awaiting::LC_0_01,
    core::LC_0_02,
    awaiting::LC_0_03,
    awaiting::LC_0_04,
    awaiting::LC_0_05,
    awaiting::LC_0_06,
    awaiting::LC_0_07,
    awaiting::LC_0_08,
    core::LC_0_09,
    awaiting::LC_0_10,
    geo::LC_0_11,
    geo::LC_0_12,
    awaiting::LC_0_13,
    awaiting::LC_0_14,
    awaiting::LC_0_15,
    awaiting::LC_0_16,
    awaiting::LC_0_17,
    core::LC_0_18,
    awaiting::LC_0_19,
    core::LC_0_20,
    awaiting::LC_0_21,
    core::LC_0_22,
    core::LC_0_23,
    awaiting::LC_0_24,
    awaiting::LC_0_25,
    core::LC_0_26,
    core::LC_0_27,
    awaiting::LC_0_28,
    awaiting::LC_0_29,
    awaiting::LC_0_30,
    awaiting::LC_0_31,
    awaiting::LC_0_32,
    awaiting::LC_0_33,
    awaiting::LC_0_34,
    awaiting::LC_0_35,
    awaiting::LC_0_36,
    awaiting::LC_0_37,
    awaiting::LC_0_38,
    lives::LC_0_39,
    awaiting::LC_0_40,
    core::LC_0_41,
    awaiting::LC_0_42,
    awaiting::LC_0_43,
    awaiting::LC_0_44,
    awaiting::LC_0_45,
    awaiting::LC_0_46,
    awaiting::LC_0_47,
    awaiting::LC_0_48,
    awaiting::LC_0_49,
    awaiting::LC_0_50,
    core::LC_0_51,
    core::LC_0_52,
    awaiting::LC_0_53,
    lives::LC_0_54,
    core::LC_0_55,
    awaiting::LC_0_56,
    awaiting::LC_0_57,
    awaiting::LC_0_58,
    core::LC_0_59,
    core::LC_0_60,
    awaiting::LC_0_61,
    core::LC_0_62,
    core::LC_0_63,
    core::LC_0_64,
    core::LC_0_65,
    awaiting::LC_1_01,
    awaiting::LC_1_02,
    awaiting::LC_1_03,
    core::LC_1_04,
    awaiting::LC_1_43,
    awaiting::LC_1_44,
    awaiting::LC_1_05,
    awaiting::LC_1_06,
    core::LC_1_07,
    awaiting::LC_1_08,
    awaiting::LC_1_10,
    awaiting::LC_1_11,
    awaiting::LC_1_12,
    core::LC_1_13,
    awaiting::LC_1_14,
    awaiting::LC_1_15,
    awaiting::LC_1_16,
    core::LC_1_17,
    awaiting::LC_1_18,
    awaiting::LC_1_19,
    awaiting::LC_1_20,
    core::LC_1_21,
    awaiting::LC_1_22,
    core::LC_1_23,
    awaiting::LC_1_24,
    awaiting::LC_1_25,
    core::LC_1_26,
    awaiting::LC_1_27,
    awaiting::LC_1_28,
    awaiting::LC_1_29,
    core::LC_1_30,
    awaiting::LC_1_31,
    awaiting::LC_1_32,
    awaiting::LC_1_33,
    core::LC_1_34,
    core::LC_1_35,
    lives::LC_1_36,
    core::LC_1_37,
    awaiting::LC_1_38,
    awaiting::LC_1_39,
    core::LC_1_40,
    awaiting::LC_1_41,
    lives::LC_1_49,
    lives::LC_1_51,
    awaiting::LC_1_45,
    awaiting::LC_1_46,
    awaiting::LC_1_47,
];
