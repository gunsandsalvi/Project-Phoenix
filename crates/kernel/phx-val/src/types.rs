//! The outlook types as the world reads them, compiled once: the memory types with each one's adaptive gain, the
//! switching types with each one's intensity, by which a party's are drawn, the heuristics' shared parameters, and the
//! age classes whose lived years weight a series' history.

use phx_core::Register;
use phx_core::register::values::{Distribution, TypeSet};
use phx_macros::{clause, opening};
use phx_num::Missing;
use phx_rand::float::from_i64;

/// The types and parameters every party's outlooks read.
#[derive(Clone, Debug, PartialEq)]
pub struct Types {
    pub memory: TypeSet,
    pub gains: Vec<f64>,
    pub switching: TypeSet,
    pub intensities: Vec<f64>,
    pub trend: f64,
    pub anchor: f64,
    pub performance_memory: f64,
    /// The exponent of the lived years' weights, and the first ages of the classes whose lived years weight a
    /// series' history, the first of them nought so that every age has its class.
    pub theta: f64,
    pub windows: Vec<i64>,
}

/// A distribution cut into its types, with each type's value.
#[opening]
fn cut(register: &Register, (distribution, count): (&str, &str)) -> Result<(TypeSet, Vec<f64>), String> {
    let types = u16::try_from(register.count(count)?).map_err(|e| e.to_string())?;
    let d: &Distribution = register.distribution(distribution)?;
    let scale = libm::pow(crate::consts::DECADE, f64::from(d.exp));
    let set = TypeSet::build(d, types)?;
    let values = set.types().iter().map(|t| from_i64(t.value) / scale).collect();
    Ok((set, values))
}

impl Types {
    /// # Errors
    /// A primitive the types read that the register does not hold, or a distribution that cuts into no types.
    #[opening]
    pub fn compile(register: &Register) -> Result<Types, String> {
        let (memory, gains) = cut(register, (crate::prims::ADAPTIVE_GAIN.id, crate::prims::MEMORY_TYPES.id))?;
        let (switching, intensities) =
            cut(register, (crate::prims::SWITCHING_INTENSITY.id, crate::prims::SWITCHING_TYPES.id))?;
        Ok(Types {
            memory,
            gains,
            switching,
            intensities,
            trend: register.fixed(crate::prims::TREND_GAMMA.id)?,
            anchor: register.fixed(crate::prims::ANCHOR_KAPPA.id)?,
            performance_memory: register.fixed(crate::prims::PERFORMANCE_MEMORY.id)?,
            theta: register.fixed(crate::prims::EXPERIENCE_THETA.id)?,
            windows: windows(register)?,
        })
    }

    /// The views of a public series each method forms: every memory type at every age class, and at none.
    #[must_use]
    pub fn views(&self) -> usize {
        self.gains.len() * (self.windows.len() + 1)
    }

    /// A view's memory type and its age class, none for the view of no age.
    pub fn of_view(&self, view: usize) -> (usize, Missing<usize>) {
        let per = self.windows.len() + 1;
        let at = view % per;
        (view / per, if at == 0 { Missing::Absent } else { Missing::Present(at - 1) })
    }

    /// The age class an age falls in: the last whose first age it has reached.
    #[clause("VAL.23")]
    pub fn window_of(&self, age: u32) -> Missing<u16> {
        match self.windows.iter().rposition(|first| *first <= i64::from(age)).and_then(|w| u16::try_from(w).ok()) {
            Some(w) => Missing::Present(w),
            None => Missing::Absent,
        }
    }
}

/// The view a decider's method forms among a series' views, with `classes` age classes: its memory type at its age
/// class, or at none where it has no age.
#[clause("VAL.23")]
#[must_use]
pub fn view(classes: usize, memory: u16, window: Missing<u16>) -> usize {
    let at = match window {
        Missing::Present(w) => usize::from(w) + 1,
        Missing::Absent => 0,
    };
    usize::from(memory) * (classes + 1) + at
}

/// The age classes' first ages, refused unless the first is nought.
fn windows(register: &Register) -> Result<Vec<i64>, String> {
    let bounds = register.partition(crate::prims::AGE_WINDOWS.id)?.bounds.to_vec();
    if bounds.first() == Some(&0) {
        Ok(bounds)
    } else {
        Err(format!("{}: the first age class does not begin at birth", crate::prims::AGE_WINDOWS.id))
    }
}

#[cfg(test)]
mod tests {
    use phx_num::Missing;

    use super::view;

    #[test]
    fn views_part_memory_types_by_age_class() {
        // Three classes: each memory type's four views, the one of no age first.
        assert_eq!(view(3, 0, Missing::Absent), 0);
        assert_eq!(view(3, 0, Missing::Present(2)), 3);
        assert_eq!(view(3, 1, Missing::Absent), 4, "the next memory type's first view");
        assert_eq!(view(3, 1, Missing::Present(0)), 5);
    }
}
