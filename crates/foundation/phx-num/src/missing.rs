use phx_macros::clause;

use crate::consts::ABSENT_I64;
use crate::violation;

/// A value that may be absent; with no default and no fallback accessor, a reader must say what absence means.
#[clause("NUM.8")]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing<T> {
    Present(T),
    Absent,
}

/// `Missing<i64>`'s column form, where one reserved value marks absence.
#[repr(transparent)]
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaybeI64(i64);

impl MaybeI64 {
    pub const ABSENT: MaybeI64 = MaybeI64(ABSENT_I64);

    /// The marker is never a present value, so absence can never be read back as a number.
    #[clause("NUM.8")]
    pub fn present(value: i64) -> MaybeI64 {
        if value == ABSENT_I64 {
            violation!(clause = "NUM.8", "the absent marker written as a present value");
        }
        MaybeI64(value)
    }

    pub fn from_missing(value: Missing<i64>) -> MaybeI64 {
        match value {
            Missing::Present(v) => MaybeI64::present(v),
            Missing::Absent => MaybeI64::ABSENT,
        }
    }

    pub const fn get(self) -> Missing<i64> {
        if self.0 == ABSENT_I64 { Missing::Absent } else { Missing::Present(self.0) }
    }
}

#[cfg(test)]
mod tests {
    use super::{MaybeI64, Missing};
    use crate::violation::testing::violated_clause;

    #[test]
    fn maybe_i64_refuses_marker() {
        assert_eq!(violated_clause(|| MaybeI64::present(i64::MIN)), "NUM.8");
        assert_eq!(MaybeI64::present(-5).get(), Missing::Present(-5));
        assert_eq!(MaybeI64::ABSENT.get(), Missing::Absent);
        assert_eq!(MaybeI64::from_missing(Missing::Absent), MaybeI64::ABSENT);
    }
}
