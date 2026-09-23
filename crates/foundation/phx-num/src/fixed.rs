use std::ops::{Add, Sub};

use phx_macros::clause;

use crate::consts::DECIMAL_BASE_F64;
use crate::error::NumError;
use crate::round::{Round, Side, div_round};
use crate::violation;

/// A decimal `raw × 10^-E`, for positions and outlooks; no float is ever stored.
#[clause("NUM.6")]
#[must_use]
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fixed<const E: u8>(i64);

impl<const E: u8> Fixed<E> {
    pub const fn from_raw(raw: i64) -> Self {
        Fixed(raw)
    }

    #[must_use]
    pub const fn raw(self) -> i64 {
        self.0
    }

    /// `self × num ÷ den`, rounded once; the scales cancel, so the raw values combine directly.
    pub fn mul_div(self, num: Self, den: Self, r: Round) -> Self {
        let product = i128::from(self.0) * i128::from(num.0);
        let (product, divisor) = match den.0.signum() {
            0 => violation!(clause = "Law 7", "a fixed-point division by zero", raw = self.0),
            1 => (product, i128::from(den.0)),
            _ => (-product, -i128::from(den.0)),
        };
        let Ok(raw) = i64::try_from(div_round(product, divisor, r)) else {
            violation!(clause = "Law 7", "a fixed-point product overflows", raw = self.0, num = num.0, den = den.0);
        };
        Fixed(raw)
    }

    /// Refuses a non-finite or out-of-range value, so no invalid number enters the world's state.
    ///
    /// # Errors
    /// `NonFinite` for NaN or an infinity; `OutOfRange` when the rounded value does not fit.
    pub fn from_f64(x: f64, r: Round) -> Result<Self, NumError> {
        if !x.is_finite() {
            return Err(NumError::NonFinite);
        }
        let scaled = x * scale_f64(E);
        let rounded = match r {
            Round::HalfEven => scaled.round_ties_even(),
            Round::HalfAwayFromZero => scaled.round(),
            Round::TowardZero | Round::InFavourOf(Side::Payer) => scaled.trunc(),
            Round::Floor => scaled.floor(),
            Round::Ceil => scaled.ceil(),
            Round::InFavourOf(Side::Payee) => {
                if scaled < 0.0 {
                    scaled.floor()
                } else {
                    scaled.ceil()
                }
            }
        };
        integral_to_i64(rounded).map(Fixed)
    }

    #[must_use]
    pub fn to_f64(self) -> f64 {
        i64_to_f64(self.0) / scale_f64(E)
    }
}

/// 10^E as a float: each power up to 10^22 is exact, so repeated multiplication adds no error there.
fn scale_f64(e: u8) -> f64 {
    DECIMAL_BASE_F64.powi(i32::from(e))
}

/// An `i64` as the nearest `f64`, from two exactly representable halves joined by one rounding.
fn i64_to_f64(n: i64) -> f64 {
    let (Ok(high), Ok(low)) = (i32::try_from(n >> u32::BITS), u32::try_from(n & i64::from(u32::MAX))) else {
        violation!(clause = "Law 7", "an i64 does not split into its halves", n = n);
    };
    let half = 2.0_f64.powi(u32::BITS.cast_signed());
    f64::from(high) * half + f64::from(low)
}

/// An integral, finite `f64` as an `i64`, read from its bits so no cast can wrap or saturate.
fn integral_to_i64(x: f64) -> Result<i64, NumError> {
    let limit = 2.0_f64.powi((i64::BITS - 1).cast_signed());
    if !(x >= -limit && x < limit) {
        return Err(NumError::OutOfRange);
    }
    let bits = x.to_bits();
    let fraction_bits = f64::MANTISSA_DIGITS - 1;
    let exponent_bits = u64::BITS - 1 - fraction_bits;
    let biased = (bits >> fraction_bits) & ((1 << exponent_bits) - 1);
    if biased == 0 {
        return Ok(0);
    }
    let Ok(biased) = i32::try_from(biased) else {
        return Err(NumError::OutOfRange);
    };
    let shift = biased - (f64::MAX_EXP - 1) - fraction_bits.cast_signed();
    let significand = i128::from((bits & ((1 << fraction_bits) - 1)) | (1 << fraction_bits));
    let magnitude = if shift >= 0 { significand << shift.unsigned_abs() } else { significand >> shift.unsigned_abs() };
    let signed = if x < 0.0 { -magnitude } else { magnitude };
    i64::try_from(signed).map_err(|_| NumError::OutOfRange)
}

impl<const E: u8> Add for Fixed<E> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let Some(raw) = self.0.checked_add(other.0) else {
            violation!(clause = "Law 7", "a fixed-point sum overflows", a = self.0, b = other.0);
        };
        Fixed(raw)
    }
}

impl<const E: u8> Sub for Fixed<E> {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let Some(raw) = self.0.checked_sub(other.0) else {
            violation!(clause = "Law 7", "a fixed-point difference overflows", a = self.0, b = other.0);
        };
        Fixed(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::Fixed;
    use crate::error::NumError;
    use crate::round::{Round, Side};
    use crate::violation::testing::violated_clause;

    type F6 = Fixed<6>;

    #[test]
    fn fixed_from_f64_refuses_nan() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(F6::from_f64(x, Round::HalfEven), Err(NumError::NonFinite));
        }
        assert_eq!(F6::from_f64(1e13, Round::HalfEven), Err(NumError::OutOfRange));
        assert_eq!(Fixed::<0>::from_f64(-9_223_372_036_854_775_808.0, Round::Floor), Ok(Fixed::from_raw(i64::MIN)));
        assert_eq!(Fixed::<0>::from_f64(9_223_372_036_854_775_808.0, Round::Floor), Err(NumError::OutOfRange));
    }

    #[test]
    fn fixed_from_f64_rounds_by_convention() {
        assert_eq!(F6::from_f64(1.234_567_5, Round::HalfEven).map(F6::raw), Ok(1_234_568));
        assert_eq!(F6::from_f64(-0.000_000_5, Round::HalfEven).map(F6::raw), Ok(0));
        assert_eq!(F6::from_f64(-0.000_000_4, Round::InFavourOf(Side::Payee)).map(F6::raw), Ok(-1));
        assert_eq!(F6::from_f64(0.0, Round::Ceil).map(F6::raw), Ok(0));
        assert_eq!(F6::from_f64(-2.5, Round::TowardZero).map(F6::raw), Ok(-2_500_000));
        assert!((F6::from_raw(-1_500_000).to_f64() + 1.5).abs() < f64::EPSILON);
        assert!((F6::from_raw(i64::MAX).to_f64() - 9.223_372_036_854_776e12).abs() < 1.0);
    }

    #[test]
    fn fixed_arithmetic_is_checked() {
        let (a, b) = (F6::from_raw(3_000_000), F6::from_raw(2_000_000));
        assert_eq!(a.mul_div(b, F6::from_raw(-4_000_000), Round::HalfEven), F6::from_raw(-1_500_000));
        assert_eq!(a + b, F6::from_raw(5_000_000));
        assert_eq!(violated_clause(|| a.mul_div(b, F6::from_raw(0), Round::Floor)), "Law 7");
        assert_eq!(violated_clause(|| F6::from_raw(i64::MAX) + F6::from_raw(1)), "Law 7");
    }
}
