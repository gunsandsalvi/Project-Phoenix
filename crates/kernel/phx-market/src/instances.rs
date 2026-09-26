//! Markets by kind: each kind declared by its system once, as the template of its instances; an instance per subject
//! the kind is over, made the first time an order names it, each with its own identity, so only the markets someone
//! trades in exist.

use std::collections::BTreeMap;

use phx_id::MarketId;
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};

use crate::market::{MarketDecl, MarketKey};

/// Every market kind declared, in the order declared. A kind's declaration is its instances' template: each instance
/// takes its own identity and subject in place of the kind's.
#[clause("MKT.1", "GDS.7")]
#[derive(Clone, Debug, Default)]
pub struct Kinds {
    kinds: Vec<MarketDecl>,
}

/// The instances made, saved with the markets: each kind's place and subject by identity, and back.
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Made {
    list: Vec<(u16, u64)>,
    by_key: BTreeMap<(u16, u64), MarketId>,
}

impl Kinds {
    /// A kind declared; two of one name stop the run.
    pub fn declare(&mut self, kind: MarketDecl) {
        let code = phx_ledger::instruction::name_code(kind.key.kind);
        if self.kinds.iter().any(|k| phx_ledger::instruction::name_code(k.key.kind) == code) {
            violation!(clause = "MKT.1", "two market kinds of one name or code");
        }
        self.kinds.push(kind);
    }

    /// Every kind's name, in the order declared, which a load checks against the build's.
    pub fn names(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.kinds.iter().map(|k| k.key.kind)
    }

    /// A kind's place by its name's code, as an order intent carries it.
    pub fn kind(&self, code: u64) -> Missing<u16> {
        match self.kinds.iter().position(|k| phx_ledger::instruction::name_code(k.key.kind) == code).map(u16::try_from)
        {
            Some(Ok(i)) => Missing::Present(i),
            Some(Err(_)) => capacity_exceeded!("market kinds", u16::MAX, self.kinds.len()),
            None => Missing::Absent,
        }
    }

    /// A kind's declaration by its place.
    #[must_use]
    pub fn kind_decl(&self, kind: u16) -> &MarketDecl {
        let Some(k) = self.kinds.get(usize::from(kind)) else {
            violation!(clause = "MKT.1", "a market of an undeclared kind", kind = kind);
        };
        k
    }

    /// The instance of a kind over a subject, made the first time it is named.
    pub fn instance(&self, made: &mut Made, kind: u16, subject: u64) -> MarketId {
        if let Missing::Present(id) = made.of(kind, subject) {
            return id;
        }
        let _ = self.kind_decl(kind);
        let Ok(n) = u32::try_from(made.list.len()) else {
            capacity_exceeded!("markets", u32::MAX, made.list.len());
        };
        let id = MarketId::new(n);
        made.list.push((kind, subject));
        made.by_key.insert((kind, subject), id);
        id
    }

    /// An instance's declaration: its kind's, with its own identity and subject.
    #[must_use]
    pub fn decl(&self, made: &Made, market: MarketId) -> MarketDecl {
        let Some(&(kind, subject)) = usize::try_from(market.get()).ok().and_then(|i| made.list.get(i)) else {
            violation!(clause = "MKT.1", "a market never made", market = market.get());
        };
        let k = self.kind_decl(kind);
        MarketDecl { id: market, key: MarketKey { kind: k.key.kind, subject }, ..*k }
    }

    /// Whether every instance a save made is of a kind the build declares.
    ///
    /// # Errors
    /// The first instance of a kind the build does not declare.
    pub fn check(&self, made: &Made) -> Result<(), String> {
        match made.list.iter().find(|(k, _)| usize::from(*k) >= self.kinds.len()) {
            Some((k, _)) => Err(format!("a market of kind {k}, which the build does not declare")),
            None => Ok(()),
        }
    }
}

impl Made {
    /// The instance of a kind over a subject, if one has been made.
    pub fn of(&self, kind: u16, subject: u64) -> Missing<MarketId> {
        self.by_key.get(&(kind, subject)).copied().map_or(Missing::Absent, Missing::Present)
    }

    /// An instance's subject, if it has been made.
    pub fn subject_of(&self, market: MarketId) -> Missing<u64> {
        match usize::try_from(market.get()).ok().and_then(|i| self.list.get(i)) {
            Some((_, s)) => Missing::Present(*s),
            None => Missing::Absent,
        }
    }

    /// Every instance made, by identity: its kind's place and its subject.
    pub fn iter(&self) -> impl Iterator<Item = (MarketId, u16, u64)> + '_ {
        (0_u32..).zip(&self.list).map(|(i, (k, s))| (MarketId::new(i), *k, *s))
    }
}

#[cfg(test)]
mod tests {
    use phx_id::MarketId;
    use phx_num::Missing;

    use super::{Kinds, Made};
    use crate::market::{Form, MarketDecl, MarketKey, Ration, TieRule};

    const KIND: MarketDecl = MarketDecl {
        id: MarketId::new(0),
        name: "commodities",
        key: MarketKey { kind: "GDS.commodities", subject: 0 },
        form: Form::Call,
        operator: "exchange",
        meeting_days: "business days",
        settle_days: 0,
        participants: "firms",
        tick: 1,
        ties: &[TieRule::MaxVolume, TieRule::LowerPrice],
        ration: Ration::ProRata,
        stream: "GDS.lots",
        quantity_response: Missing::Absent,
        admission: Missing::Absent,
    };

    #[test]
    fn an_instance_is_made_once_per_subject() {
        let (mut m, mut made) = (Kinds::default(), Made::default());
        m.declare(KIND);
        let Missing::Present(k) = m.kind(phx_ledger::instruction::name_code("GDS.commodities")) else {
            panic!("the kind is declared")
        };
        let a = m.instance(&mut made, k, 77);
        let b = m.instance(&mut made, k, 78);
        assert_eq!((a, m.instance(&mut made, k, 77)), (MarketId::new(0), MarketId::new(0)));
        assert_eq!(b, MarketId::new(1));
        assert_eq!(m.decl(&made, b).key.subject, 78);
        assert_eq!(m.decl(&made, b).id, b);
        assert_eq!(m.kind(1), Missing::Absent);
        assert!(m.check(&made).is_ok() && Kinds::default().check(&made).is_err());
    }
}
