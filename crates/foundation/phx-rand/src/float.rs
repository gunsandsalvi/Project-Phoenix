use phx_num::capacity_exceeded;

/// A length as a count; every supported target's pointer is at most 64 bits wide.
#[must_use]
pub fn len_u64(len: usize) -> u64 {
    match u64::try_from(len) {
        Ok(v) => v,
        Err(_) => capacity_exceeded!("a length as u64", u64::MAX, len),
    }
}

/// A count as an index; a count too large to index is a table larger than memory.
#[must_use]
pub fn index(i: u64) -> usize {
    match usize::try_from(i) {
        Ok(v) => v,
        Err(_) => capacity_exceeded!("a count as an index", len_u64(usize::MAX), i),
    }
}

/// A `u64` as the nearest `f64`, from two exactly representable halves joined by one rounding.
#[must_use]
pub fn from_u64(n: u64) -> f64 {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = n.to_le_bytes();
    let (high, low) = (u32::from_le_bytes([b4, b5, b6, b7]), u32::from_le_bytes([b0, b1, b2, b3]));
    f64::from(high) * libm::ldexp(1.0, u32::BITS.cast_signed()) + f64::from(low)
}

/// An `i64` as the nearest `f64`, the same way; the high half keeps the sign.
#[must_use]
pub fn from_i64(n: i64) -> f64 {
    let [b0, b1, b2, b3, b4, b5, b6, b7] = n.to_le_bytes();
    let (high, low) = (i32::from_le_bytes([b4, b5, b6, b7]), u32::from_le_bytes([b0, b1, b2, b3]));
    f64::from(high) * libm::ldexp(1.0, u32::BITS.cast_signed()) + f64::from(low)
}

/// `floor(x)` as an integer, read from the float's bits so no cast can wrap or saturate; `None` when `x` is not
/// finite or its floor lies beyond ±2^126.
fn floor_to_i128(x: f64) -> Option<i128> {
    let floor = libm::floor(x);
    let limit = libm::ldexp(1.0, (i128::BITS - 2).cast_signed());
    if !(floor > -limit && floor < limit) {
        return None;
    }
    let bits = floor.to_bits();
    let fraction_bits = f64::MANTISSA_DIGITS - 1;
    let exponent_bits = u64::BITS - 1 - fraction_bits;
    let biased = i32::try_from((bits >> fraction_bits) & ((1 << exponent_bits) - 1)).ok()?;
    if biased == 0 {
        return Some(0);
    }
    let shift = biased - (f64::MAX_EXP - 1) - fraction_bits.cast_signed();
    let significand = i128::from((bits & ((1 << fraction_bits) - 1)) | (1 << fraction_bits));
    let magnitude = if shift >= 0 { significand << shift.unsigned_abs() } else { significand >> shift.unsigned_abs() };
    Some(if floor < 0.0 { -magnitude } else { magnitude })
}

#[must_use]
pub fn floor_to_u64(x: f64) -> Option<u64> {
    floor_to_i128(x).and_then(|v| u64::try_from(v).ok())
}

#[must_use]
pub fn floor_to_i64(x: f64) -> Option<i64> {
    floor_to_i128(x).and_then(|v| i64::try_from(v).ok())
}

#[cfg(test)]
mod tests {
    use super::{floor_to_i64, floor_to_u64, from_i64, from_u64};

    #[test]
    fn conversions_are_exact_where_representable() {
        for n in [0_u64, 1, 7, 1 << 40, (1 << 53) - 1, (1 << 63) + 12_345] {
            assert_eq!(floor_to_u64(from_u64(n)).map(|m| m.abs_diff(n) <= n >> 52), Some(true), "{n}");
        }
        assert_eq!(floor_to_u64(from_u64(u64::MAX)), None, "u64::MAX rounds up to 2^64");
        assert_eq!(floor_to_u64(from_u64((1 << 53) - 1)), Some((1 << 53) - 1));
        assert_eq!(floor_to_i64(-2.5), Some(-3));
        assert_eq!(floor_to_i64(from_i64(i64::MIN)), Some(i64::MIN));
        assert_eq!(floor_to_u64(-0.5), None);
        assert_eq!(floor_to_u64(1.9e19), None);
        assert_eq!(floor_to_u64(f64::NAN), None);
        assert_eq!(floor_to_u64(0.999), Some(0));
    }
}
