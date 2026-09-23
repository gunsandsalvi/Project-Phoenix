use phx_id::{CountryId, Day, InstrumentId, LineId, MarketId, PartyId, TileId};
use phx_num::{Ccy, UnitId};

/// What a finding is about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FindingOwner {
    Party(PartyId),
    Tile(TileId),
    Line(LineId),
    Instrument(InstrumentId),
    Market(MarketId),
    Country(CountryId),
}

/// The unit a finding's size is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Money(Ccy),
    Qty(UnitId),
}

/// An invariant the audit found broken: its family and clause, what it concerns, by how much and on which day. The
/// audit records it and never repairs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub family: &'static str,
    pub clause: &'static str,
    pub owner: FindingOwner,
    pub size: i128,
    pub unit: Unit,
    pub day: Day,
    pub detail: String,
}

/// The run's findings, kept outside the world: not in its hash, and never read by a handler.
#[derive(Debug, Default)]
pub struct Findings {
    list: Vec<Finding>,
}

impl Findings {
    pub fn record(&mut self, finding: Finding) {
        self.list.push(finding);
    }

    #[must_use]
    pub fn all(&self) -> &[Finding] {
        &self.list
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.list.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn of_family<'a>(&'a self, family: &'a str) -> impl Iterator<Item = &'a Finding> + 'a {
        self.list.iter().filter(move |f| f.family == family)
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, PartyId};
    use phx_num::Ccy;

    use super::{Finding, FindingOwner, Findings, Unit};

    #[test]
    fn findings_keep_their_order() {
        let mut f = Findings::default();
        let finding = |family, size| Finding {
            family,
            clause: "MON.1",
            owner: FindingOwner::Party(PartyId::new(4)),
            size,
            unit: Unit::Money(Ccy::new(0)),
            day: Day::new(2),
            detail: String::new(),
        };
        f.record(finding("MON.issuer_balance", 5));
        f.record(finding("SET.flows", 7));
        f.record(finding("MON.issuer_balance", -3));
        let sizes: Vec<i128> = f.of_family("MON.issuer_balance").map(|x| x.size).collect();
        assert_eq!((sizes, f.len()), (vec![5, -3], 3));
    }
}
