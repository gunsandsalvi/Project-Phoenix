//! The representation's settings, shared by every population kind, and the streams it draws from.

use phx_core::{Declarations, Prim, Register, StreamDef, declare_prim, declare_stream};
use phx_macros::clause;
use phx_num::{Count, violation};

declare_prim! {
    /// Every population agent's multiplicity: the factor under twins, one under a small world.
    pub MULTIPLICITY = "REP.multiplicity" { kind: Resolution, value: Count, clause: "REP.40", scope: Shared }
}

declare_prim! {
    /// What the setup's total population is divided by before the countries are derived: one under twins, the factor
    /// under a small world.
    pub POPULATION_DIVISOR = "REP.population_divisor" { kind: Resolution, value: Count, clause: "REP.40", scope: Shared }
}

declare_stream! { pub ClearedStream = "REP.cleared" { purpose: Pairing, keyed: false, clause: "REP.23" } }
declare_stream! { pub HouseholdsStream = "REP.households" { purpose: Sample, keyed: false, clause: "REP.26" } }

/// The representation's primitives as the world reads them.
#[derive(Debug)]
pub struct RepPrims {
    pub multiplicity: Prim<Count>,
    pub population_divisor: Prim<Count>,
}

impl RepPrims {
    pub fn declare(d: &mut Declarations) -> RepPrims {
        d.stream(HouseholdsStream::DECL);
        d.stream(ClearedStream::DECL);
        RepPrims { multiplicity: d.prim(&MULTIPLICITY), population_divisor: d.prim(&POPULATION_DIVISOR) }
    }
}

/// The representation in force: the multiplicity of every population agent and the divisor of the setup's
/// population.
#[clause("REP.40", "REP.17")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Representation {
    pub multiplicity: u32,
    pub population_divisor: u32,
}

impl Representation {
    /// The representation the register holds; a multiplicity or divisor of nought is refused.
    #[must_use]
    pub fn of(prims: &RepPrims, register: &Register) -> Representation {
        let read = |prim: Prim<Count>| {
            let v = prim.shared(register).get();
            let Ok(v) = u32::try_from(v) else {
                violation!(clause = "REP.40", "a representation's factor beyond counting", factor = v);
            };
            if v == 0 {
                violation!(clause = "REP.17", "a representation of no twins or no population");
            }
            v
        };
        let r = Representation {
            multiplicity: read(prims.multiplicity),
            population_divisor: read(prims.population_divisor),
        };
        if r.multiplicity > 1 && r.population_divisor > 1 {
            violation!(
                clause = "REP.40",
                "twins in a small world, which is neither representation",
                multiplicity = r.multiplicity,
                divisor = r.population_divisor
            );
        }
        r
    }

    /// Which representation this is, for reports: twins, or a small world; a factor of one is both.
    #[must_use]
    pub fn name(self) -> String {
        match (self.multiplicity, self.population_divisor) {
            (1, 1) => "one agent for every party".to_owned(),
            (1, k) => format!("a small world at {k}"),
            (k, _) => format!("twins at {k}"),
        }
    }
}
