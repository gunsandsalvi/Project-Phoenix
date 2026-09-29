//! A line kind as a system declares it: its two sides and who may hold them.

use crate::algebra::Side;

/// One side of a line kind: the kinds of party that may hold it, the optional words its rows carry, and whether the
/// line keeps its holders in a list — a many-party retail side reached only on its dues keeps none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideDecl {
    pub holder_kinds: &'static [&'static str],
    pub words: u8,
    pub holder_list: bool,
    /// On a population's agents, the roles whose persons hold the side's rows, each person a contract; none, the
    /// households.
    pub holder_roles: &'static [&'static str],
    /// Whether a holder holds at most one row of the kind on this side, as a person holds one job.
    pub exclusive: bool,
    /// Whether a holder's row counts a member for each of its counterparts, as an employer a job for each employee,
    /// so it holds any number; otherwise it holds one, or one for each of its persons in `holder_roles`.
    pub many: bool,
}

/// A line kind as a system declares it: its two sides, and the systems that may request a transfer of its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineKindDecl {
    pub name: &'static str,
    pub asset: SideDecl,
    pub liability: SideDecl,
    pub transfer_requesters: &'static [&'static str],
    /// Whether its lines have dues on dates, so its rows sit in their holders' due-day runs.
    pub dated: bool,
}

impl LineKindDecl {
    #[must_use]
    pub fn side(&self, side: Side) -> &SideDecl {
        match side {
            Side::Asset => &self.asset,
            Side::Liability => &self.liability,
        }
    }
}
