//! The phone model's day and turn lines: a line's core-ms is its measured VM nanoseconds times the phone's factor times
//! its day's count; a line no driver measures yet is the steps' declared figure, reported as declared.

use crate::DayType;
use crate::design::Design;

/// Where a line's figures come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum Source {
    Measured,
    Declared,
}

/// One line of the day: its core-ms on a business, a non-business and a heavy day.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Line {
    pub name: String,
    pub core_ms: [f64; 3],
    pub source: Source,
}

/// A line's phone core-ms from its measured VM nanoseconds an item, the phone's factor and its day's count.
#[must_use]
pub fn line_core_ms(vm_ns: f64, k: f64, count: f64) -> f64 {
    const NS_A_MS: f64 = 1e6;
    vm_ns * k * count / NS_A_MS
}

/// The day types' core-ms and the binding turns' wall ms on the phone's cores.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Days {
    /// Business, non-business, heavy, and the business day after closed days.
    pub core_ms: [f64; 4],
    /// Four closed days then the business day after them, and three closed days then a heavy day, in wall ms.
    pub turns_ms: [f64; 2],
}

/// Every line's sum by day type, and the two binding turns: their days' core-ms summed over the phone's cores.
#[must_use]
pub fn days(lines: &[Line], design: &Design) -> Days {
    let sum = |i: usize| lines.iter().filter_map(|l| l.core_ms.get(i)).sum::<f64>();
    let (b, nb, h) = (sum(DayType::B.index()), sum(DayType::Nb.index()), sum(DayType::H.index()));
    let bc = b + design.bc_extra_core_ms;
    let (closed_before_bc, closed_before_h) = (4.0, 3.0);
    let cores = design.phone.cores;
    Days {
        core_ms: [b, nb, h, bc],
        turns_ms: [(closed_before_bc * nb + bc) / cores, (closed_before_h * nb + h) / cores],
    }
}

/// The steps' declared stage lines, those no driver measures, with the day's total left out.
#[must_use]
pub fn declared(design: &Design) -> Vec<Line> {
    design
        .stage
        .iter()
        .filter(|(name, _)| name.as_str() != "day")
        .map(|(name, core_ms)| Line { name: name.clone(), core_ms: *core_ms, source: Source::Declared })
        .collect()
}
