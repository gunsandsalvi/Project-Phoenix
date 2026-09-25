use phx_core::calendar::period::Period;
use phx_id::Day;
use phx_macros::clause;
use phx_num::{Ccy, Fixed, Missing, Round, UnitId, violation};

/// A variable some party forecasts: a public series, or one of its own (its income, its sales, its job).
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VarId(u32);

impl VarId {
    pub const fn new(raw: u32) -> VarId {
        VarId(raw)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A party's own forecast of a variable it acts on, with the width of its recent surprises about it.
#[clause("VAL.1", "VAL.4")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outlook {
    pub var: VarId,
    pub unit: UnitId,
    pub ccy: Missing<Ccy>,
    pub horizon: Period,
    pub day: Day,
    pub mean: Fixed<6>,
    pub width: Fixed<6>,
}

impl Outlook {
    /// Whether a decision on `day` may read it: one formed today was formed earlier in the day's stages, which the
    /// caller's stage order guarantees, so only a date after the reading day is refused.
    #[clause("VAL.11")]
    #[must_use]
    pub fn formed_by(&self, day: Day) -> bool {
        self.day <= day
    }
}

/// A value a pure function computed, as a stored fixed-point number; a result that no `i64` holds is an impossible
/// state, never a clamped one.
#[clause("NUM.6")]
pub fn fixed(x: f64) -> Fixed<6> {
    match Fixed::<6>::from_f64(x, Round::HalfEven) {
        Ok(f) => f,
        Err(_) => violation!(clause = "NUM.6", "an outlook outside the fixed-point range"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outlook_is_read_only_from_its_day_on() {
        let o = Outlook {
            var: VarId::new(1),
            unit: UnitId::new(0),
            ccy: Missing::Absent,
            horizon: Period::months(1).unwrap_or_else(|| unreachable!()),
            day: Day::new(10),
            mean: fixed(1.5),
            width: fixed(0.25),
        };
        assert!(o.formed_by(Day::new(10)));
        assert!(o.formed_by(Day::new(11)));
        assert!(!o.formed_by(Day::new(9)));
        assert_eq!(o.mean.raw(), 1_500_000);
    }
}
