//! The outlook types as the world reads them, compiled once: the memory types with each one's adaptive gain, the
//! switching types with each one's intensity, by which a party's are drawn, and the heuristics' shared parameters.

use phx_core::Register;
use phx_core::register::values::{Distribution, TypeSet};
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
}

/// A distribution cut into its types, with each type's value.
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
        })
    }
}
