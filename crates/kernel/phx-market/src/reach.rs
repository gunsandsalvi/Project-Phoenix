use phx_core::{Declarations, Prim, Register, declare_prim};
use phx_geo::distance::ZoneDistances;
use phx_id::{CountryId, ZoneId};
use phx_macros::clause;
use phx_num::Count;

declare_prim! {
    /// Whether every search stops at its country's border, standing in for what crossing costs until the currency
    /// markets and the trade across borders are built: one where it does.
    pub CLOSED_BORDERS = "XB.closed_borders" {
        kind: Shape, value: Count, clause: "MKT.1", scope: Shared, shape: placeholder("XB")
    }
}

/// The market kernel's primitives, declared with the kernel's.
#[derive(Debug)]
pub struct MarketPrims {
    pub closed_borders: Prim<Count>,
}

impl MarketPrims {
    pub fn declare(d: &mut Declarations) -> MarketPrims {
        MarketPrims { closed_borders: d.prim(&CLOSED_BORDERS) }
    }
}

/// Where a searcher or a counterparty stands: its zone and its country.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub zone: ZoneId,
    pub country: CountryId,
}

/// The one set of counterparties any search reaches: those a market kind's declared search cost reaches from the
/// searcher's site over zone distances. While the borders are closed, only those in the searcher's country; a
/// counterparty with no path to the searcher is out of reach either way, and no missing distance stands for a border.
#[clause("MKT.1", "MKT.6")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reach {
    closed: bool,
}

impl Reach {
    /// The reach as the register declares it: the only read of the closed borders.
    #[must_use]
    pub fn new(prims: &MarketPrims, register: &Register) -> Reach {
        Reach { closed: prims.closed_borders.shared(register).get() == 1 }
    }

    /// The counterparties within the search cost, by their index.
    #[must_use]
    pub fn of(&self, searcher: Site, cost: u64, candidates: &[Site], distances: &ZoneDistances) -> Vec<usize> {
        candidates
            .iter()
            .enumerate()
            .filter(|(_, c)| !self.closed || c.country == searcher.country)
            .filter(|(_, c)| distances.between(searcher.zone, c.zone).is_some_and(|d| d <= cost))
            .map(|(i, _)| i)
            .collect()
    }
}
