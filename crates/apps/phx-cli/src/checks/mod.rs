pub mod accounts;
pub mod credit;
pub mod firms;
pub mod freight;
pub mod geo;
pub mod goods;
pub mod labour;
pub mod ledger;
pub mod markets;
pub mod observer;
pub mod opening;
pub mod outlooks;
pub mod plant;
pub mod population;
pub mod retail;
pub mod saves;
pub mod stage0;
pub mod technology;

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
    opening::LC_0_23,
    opening::LC_0_24,
    opening::LC_0_25,
    opening::LC_0_26,
    ledger::LC_0_27,
    ledger::LC_0_28,
    ledger::LC_0_29,
    markets::LC_0_30,
    markets::LC_0_31,
    markets::LC_0_32,
    accounts::LC_0_33,
    accounts::LC_0_34,
    saves::LC_0_35,
    saves::LC_0_36,
    population::LC_0_37,
    population::LC_0_38,
    population::LC_0_39,
    population::LC_0_40,
    population::LC_0_41,
    population::LC_0_42,
    population::LC_0_43,
    population::LC_0_44,
    population::LC_0_45,
    population::LC_0_46,
    population::LC_0_47,
    population::LC_0_48,
    population::LC_0_49,
    population::LC_0_50,
    population::LC_0_51,
    population::LC_0_52,
    population::LC_0_53,
    population::LC_0_54,
    population::LC_0_55,
    population::LC_0_56,
    observer::LC_0_57,
    observer::LC_0_58,
    observer::LC_0_59,
    observer::LC_0_60,
    ledger::LC_0_61,
    outlooks::LC_1_01,
    outlooks::LC_1_02,
    outlooks::LC_1_03,
    outlooks::LC_1_04,
    outlooks::LC_1_43,
    outlooks::LC_1_44,
    technology::LC_1_05,
    firms::LC_1_06,
    firms::LC_1_07,
    firms::LC_1_08,
    plant::LC_1_10,
    plant::LC_1_11,
    plant::LC_1_12,
    goods::LC_1_13,
    goods::LC_1_14,
    goods::LC_1_15,
    retail::LC_1_16,
    retail::LC_1_17,
    retail::LC_1_18,
    freight::LC_1_19,
    freight::LC_1_20,
    labour::LC_1_21,
    labour::LC_1_22,
    labour::LC_1_23,
    credit::LC_1_24,
    credit::LC_1_25,
    credit::LC_1_26,
    firms::LC_1_45,
    goods::LC_1_46,
    freight::LC_1_47,
];
