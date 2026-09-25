//! A method — a heuristic, a memory type and the window of lived years an age class weights — and the outlook of a
//! public series it forms, once for everyone using it.

use phx_macros::clause;

use crate::heuristic::{HeuristicId, Params, Seen};

/// A memory type, one of the finite set the register declares.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MemoryType(pub u8);

/// The lived years an age class weights: no member reads a year it did not live through.
#[must_use]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AgeWindow(pub u16);

/// How a party forms an outlook of a public series. Everyone using the same method saw the same prints the same way
/// and holds the same outlook.
#[clause("VAL.23")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Method {
    pub heuristic: HeuristicId,
    pub memory: MemoryType,
    pub window: AgeWindow,
}

/// The method's next outlook of a series, from what it saw and its memory type's parameters.
#[clause("VAL.5", "VAL.23")]
#[must_use]
pub fn outlook(method: Method, seen: &Seen, params: &Params) -> f64 {
    method.heuristic.rule().outlook(seen, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use phx_id::Day;
    use phx_num::Missing;

    #[test]
    fn methods_differ_only_by_what_they_are() {
        let seen = Seen {
            previous: 1.0,
            last: 2.0,
            before: 1.0,
            level: 0.0,
            announced: Missing::Absent,
            horizon_end: Day::new(1),
        };
        let p = Params { lambda: 0.5, gamma: 0.5, kappa: 0.5 };
        let m = |h| Method { heuristic: HeuristicId::new(h), memory: MemoryType(0), window: AgeWindow(30) };
        assert!((outlook(m(0), &seen, &p) - 1.5).abs() < f64::EPSILON);
        assert!((outlook(m(1), &seen, &p) - 2.5).abs() < f64::EPSILON);
        assert!((outlook(m(2), &seen, &p) - 1.0).abs() < f64::EPSILON);
    }
}
