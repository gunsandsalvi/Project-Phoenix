use phx_macros::clause;

/// How a sub-step visits rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubStepKind {
    /// The rows the day's agenda lists, in slot order.
    Agenda,
    /// The rows a batch reads, reading only the columns it needs.
    Stream,
    /// Rows found through an index.
    Index,
    /// Every row, where a clause needs every row that day.
    Sweep,
    /// The kernel's own apply point, where no system registers a handler.
    KernelApply,
}

/// Every sub-step of a day, in the order the day runs them.
#[clause("TIME.6", "TIME.8")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SubStep {
    S1a,
    S1b,
    S1c,
    S2a,
    S2b,
    S2c,
    S2d,
    S2e,
    S2f,
    S3a,
    S3b,
    S3c,
    S3d,
    S3e,
    S4a,
    S4b,
    S5a,
    S5b,
    S5c,
    S5d,
    S6a,
    S6b,
    S6c,
    S6d,
    S7a,
    S7b,
    S7c,
    S7d,
    S7e,
    S8a,
    S8b,
    S8c,
    S8d,
    S8e,
    S8f,
    S9a,
    S9b,
    S9c,
    S9d,
    S9e,
    S10a,
    S10b,
    S10c,
    S10d,
    S10e,
    S10f,
}

/// A sub-step's place in the table: its label, whether it runs only for countries whose business day it is, and how
/// it visits rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SubStepInfo {
    pub step: SubStep,
    pub label: &'static str,
    pub business_only: bool,
    pub kind: SubStepKind,
}

const fn info(step: SubStep, label: &'static str, business_only: bool, kind: SubStepKind) -> SubStepInfo {
    SubStepInfo { step, label, business_only, kind }
}

use SubStep as S;
use SubStepKind::{Agenda, Index, KernelApply, Stream, Sweep};

/// The day's sub-steps in order; a sub-step's ordinal is its place here.
pub const SUB_STEPS: [SubStepInfo; 46] = [
    info(S::S1a, "1a", false, Index),
    info(S::S1b, "1b", false, Index),
    info(S::S1c, "1c", false, Index),
    info(S::S2a, "2a", false, Index),
    info(S::S2b, "2b", true, Index),
    info(S::S2c, "2c", true, Stream),
    info(S::S2d, "2d", true, Index),
    info(S::S2e, "2e", true, Index),
    info(S::S2f, "2f", false, KernelApply),
    info(S::S3a, "3a", false, Index),
    info(S::S3b, "3b", false, Agenda),
    info(S::S3c, "3c", false, Agenda),
    info(S::S3d, "3d", false, Index),
    info(S::S3e, "3e", false, Index),
    info(S::S4a, "4a", false, Agenda),
    info(S::S4b, "4b", false, KernelApply),
    info(S::S5a, "5a", false, Index),
    info(S::S5b, "5b", false, Agenda),
    info(S::S5c, "5c", false, Agenda),
    info(S::S5d, "5d", false, KernelApply),
    info(S::S6a, "6a", false, Index),
    info(S::S6b, "6b", false, Index),
    info(S::S6c, "6c", true, Index),
    info(S::S6d, "6d", false, KernelApply),
    info(S::S7a, "7a", true, Stream),
    info(S::S7b, "7b", true, Index),
    info(S::S7c, "7c", true, KernelApply),
    info(S::S7d, "7d", true, Index),
    info(S::S7e, "7e", true, Index),
    info(S::S8a, "8a", true, Index),
    info(S::S8b, "8b", true, Index),
    info(S::S8c, "8c", true, Stream),
    info(S::S8d, "8d", true, Index),
    info(S::S8e, "8e", true, Stream),
    info(S::S8f, "8f", true, Index),
    info(S::S9a, "9a", true, Index),
    info(S::S9b, "9b", true, Index),
    info(S::S9c, "9c", true, Index),
    info(S::S9d, "9d", true, Index),
    info(S::S9e, "9e", true, KernelApply),
    info(S::S10a, "10a", false, Index),
    info(S::S10b, "10b", false, KernelApply),
    info(S::S10c, "10c", true, Sweep),
    info(S::S10d, "10d", false, Index),
    info(S::S10e, "10e", false, Index),
    info(S::S10f, "10f", false, Index),
];

impl SubStep {
    /// Its place in the day, which a draw's address carries.
    #[must_use]
    pub fn ordinal(self) -> u8 {
        let at = SUB_STEPS.iter().position(|i| i.step == self);
        let Some(o) = at.and_then(|a| u8::try_from(a).ok()) else {
            phx_num::violation!(clause = "TIME.6", "a sub-step missing from the day's table");
        };
        o
    }

    #[must_use]
    pub fn info(self) -> SubStepInfo {
        let Some(i) = SUB_STEPS.iter().find(|i| i.step == self) else {
            phx_num::violation!(clause = "TIME.6", "a sub-step missing from the day's table");
        };
        *i
    }

    /// The sub-step labelled so, as `5c`.
    #[must_use]
    pub fn from_label(label: &str) -> Option<SubStep> {
        SUB_STEPS.iter().find(|i| i.label == label).map(|i| i.step)
    }
}

#[cfg(test)]
mod tests {
    use super::{SUB_STEPS, SubStep, SubStepKind};

    #[test]
    fn sub_steps_in_day_order() {
        assert!(SUB_STEPS.windows(2).all(|w| w[0].step < w[1].step));
        let applies: Vec<&str> =
            SUB_STEPS.iter().filter(|i| i.kind == SubStepKind::KernelApply).map(|i| i.label).collect();
        assert_eq!(applies, ["2f", "4b", "5d", "6d", "7c", "9e", "10b"]);
        assert_eq!((SubStep::S1a.ordinal(), SubStep::S10f.ordinal()), (0, 45));
        assert_eq!(SubStep::from_label("7a"), Some(SubStep::S7a));
        assert!(SubStep::S7a.info().business_only && !SubStep::S5c.info().business_only);
    }
}
