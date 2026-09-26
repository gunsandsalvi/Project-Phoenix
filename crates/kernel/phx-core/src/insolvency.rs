//! The insolvency law a kind's parties are under, as a system declares it.

use phx_macros::clause;

/// The insolvency law a kind's parties are under: a party in arrears on any contract for longer than the law's grace
/// (`grace_days`, a count each country's register declares) is in default of payment and ends into an estate, which
/// liquidates what it holds.
#[clause("FRM.15", "L3")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsolvencyDecl {
    pub kind: &'static str,
    pub grace_days: &'static str,
    pub clause: &'static str,
}
