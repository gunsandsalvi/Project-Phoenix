//! The representation's setting, shared by every population kind, and the streams it draws from.

use phx_core::{Declarations, Prim, Register, StreamDef, declare_prim, declare_stream};
use phx_macros::clause;
use phx_num::{Count, violation};

declare_prim! {
    /// The persons the world holds of the setup's total population, before the countries are derived; the households
    /// and small firms they form are its agents, each one party.
    pub PERSONS = "REP.persons" { kind: Resolution, value: Count, clause: "REP.40", scope: Shared }
}

declare_stream! { pub ClearedStream = "REP.cleared" { purpose: Pairing, keyed: false, clause: "REP.23" } }
declare_stream! { pub LeavingStream = "REP.leaving" { purpose: Pairing, keyed: false, clause: "REP.23" } }
declare_stream! { pub EstateSiteStream = "REP.estate_site" { purpose: Sample, keyed: false, clause: "PTY.9" } }

/// The representation's primitive as the world reads it.
#[derive(Debug)]
pub struct RepPrims {
    pub persons: Prim<Count>,
}

impl RepPrims {
    pub fn declare(d: &mut Declarations) -> RepPrims {
        d.stream(LeavingStream::DECL);
        d.stream(EstateSiteStream::DECL);
        d.stream(ClearedStream::DECL);
        RepPrims { persons: d.prim(&PERSONS) }
    }
}

/// The representation in force: the persons the world holds, the one setting of its scale, which weighs no party.
#[clause("REP.40", "PTY.14")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, phx_macros::Saved)]
pub struct Representation {
    pub persons: u64,
}

impl Representation {
    /// The representation the register holds; a world of no persons is refused.
    #[must_use]
    pub fn of(prims: &RepPrims, register: &Register) -> Representation {
        let persons = prims.persons.shared(register).get();
        if persons == 0 {
            violation!(clause = "REP.40", "a representation of no population");
        }
        Representation { persons }
    }

    /// The representation, for reports.
    #[must_use]
    pub fn name(self) -> String {
        format!("{} persons", self.persons)
    }
}
