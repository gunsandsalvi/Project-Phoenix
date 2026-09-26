//! A commodity's grade: its deposit's, falling as the richest part is taken first, and the class it falls in.

use phx_macros::clause;

/// A deposit's grade once a share of it has been taken: the opening grade times e^(−κ·share), the richest part going
/// first; a deposit without end keeps its grade.
#[clause("GDS.3", "GDS.13")]
#[must_use]
pub fn now(opening: f64, fall: f64, taken_share: f64) -> f64 {
    opening * libm::exp(-fall * taken_share)
}

/// The class a grade falls in: the first whose upper bound it lies below, or the last, above every bound.
#[clause("GDS.13")]
#[must_use]
pub fn class(grade: f64, bounds: &[f64]) -> u8 {
    let at = bounds.iter().position(|b| grade < *b).unwrap_or(bounds.len());
    u8::try_from(at).unwrap_or(u8::MAX)
}

#[cfg(test)]
mod tests {
    use super::{class, now};

    #[test]
    fn grade_falls_with_what_is_taken_and_classes_by_bounds() {
        assert!((now(1.2, 0.5, 0.0) - 1.2).abs() < 1e-12);
        assert!(now(1.2, 0.5, 0.5) < 1.2);
        let bounds = [0.8, 1.25];
        assert_eq!((class(0.5, &bounds), class(1.0, &bounds), class(2.0, &bounds)), (0, 1, 2));
        assert_eq!(class(1.0, &[]), 0, "a product of one class");
    }
}
