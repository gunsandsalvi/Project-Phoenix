use std::collections::BTreeMap;

use phx_core::LegDigest;
use phx_id::PartyId;
use phx_macros::clause;

/// A position, by party and account, with what it held before the day's first leg on it and the day's net of its
/// legs.
type Position = ((PartyId, u64), (i64, i128));

/// The audit's own record of a day's settled legs, kept as they apply and apart from the books they moved, so the
/// families check the books against something the books did not write.
#[derive(Debug, Default)]
pub struct Digests {
    /// The instruction whose legs are arriving, and its paired legs' and money legs' sums by denomination so far.
    open: Option<u64>,
    open_flows: Vec<(u32, i128)>,
    open_money: Vec<(u32, i128)>,
    /// Per instruction and denomination, the sums its legs left unbalanced when another's legs came between, which
    /// the day's reading adds up.
    flows: Vec<((u64, u32), i128)>,
    money: Vec<((u64, u32), i128)>,
    /// The instructions and denominations whose legs were read.
    flow_keys: u64,
    /// Each leg's party and account, what it held before the leg and what the leg moved, in the order they settle;
    /// kept as a list, not looked up by position, since a lookup per leg costs a random probe of a large table.
    legs: Vec<((PartyId, u64), i64, i128)>,
    /// Per party and account, what it held before the day's first leg on it, and the day's net of its legs: the legs
    /// folded, by position.
    positions: Vec<Position>,
    /// Per instruction that names a way, its legs made or used up.
    made: BTreeMap<u64, phx_core::Made>,
    /// Per instruction that wears plant, its legs along its chains.
    worn: BTreeMap<u64, phx_core::Worn>,
    /// Per instrument account the day's legs moved, what was issued before them and what moved it.
    stocks: BTreeMap<u64, phx_core::StockDay>,
}

/// A difference the records show: which instruction or position, which denomination, and by how much.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gap {
    /// An instruction's paired legs that do not sum to nothing in a denomination.
    Flow { instruction: u64, denom: u32, sum: i128 },
    /// An instruction that changed the money stock of a currency without its issuer's matching leg.
    Money { instruction: u64, ccy: u32, sum: i128 },
    /// A position whose opening and the day's legs do not make what it holds at the close.
    Units { party: PartyId, account: u64, expected: i128, held: i64 },
}

impl Digests {
    /// A leg as it settles.
    pub fn record(&mut self, instruction: u64, leg: LegDigest) {
        let q = i128::from(leg.qty);
        // An instruction's legs arrive together, so their sums are kept only while they do.
        if self.open != Some(instruction) {
            self.flush();
            self.open = Some(instruction);
        }
        if leg.paired {
            if let Some((_, sum)) = self.open_flows.iter_mut().find(|(d, _)| *d == leg.denom) {
                *sum += i128::from(leg.flow);
            } else {
                self.flow_keys += 1;
                self.open_flows.push((leg.denom, i128::from(leg.flow)));
            }
        }
        if leg.money {
            match self.open_money.iter_mut().find(|(d, _)| *d == leg.denom) {
                Some((_, sum)) => *sum += q,
                None => self.open_money.push((leg.denom, q)),
            }
        }
        self.legs.push(((leg.party, leg.account), leg.before, q));
        if let phx_num::Missing::Present(way) = leg.made {
            let made =
                self.made.entry(instruction).or_insert_with(|| phx_core::Made { instruction, way, legs: Vec::new() });
            made.legs.push(phx_core::MadeLeg { party: leg.party, denom: leg.denom, qty: leg.qty, unit: leg.unit });
        }
        if let phx_num::Missing::Present((chain, class)) = leg.worn {
            let worn = self.worn.entry(instruction).or_insert_with(|| phx_core::Worn { instruction, legs: Vec::new() });
            worn.legs.push(phx_core::WornLeg { party: leg.party, chain, class, qty: leg.qty });
        }
        if let phx_num::Missing::Present(issued) = leg.issued {
            let day = self.stocks.entry(leg.account).or_insert_with(|| phx_core::StockDay {
                account: leg.account,
                opening: issued,
                ..phx_core::StockDay::default()
            });
            match leg.source {
                phx_num::Missing::Present(source) => {
                    if !day.transformed.iter().any(|t| t.0 == source) {
                        day.transformed.push((source, 0, 0));
                    }
                    if let Some(t) = day.transformed.iter_mut().find(|t| t.0 == source) {
                        if leg.qty < 0 { t.2 -= q } else { t.1 += q }
                    }
                }
                phx_num::Missing::Absent if leg.paired => day.traded += q,
                phx_num::Missing::Absent => day.unaccounted += q,
            }
        }
    }

    /// The legs recorded since the last fold folded into the positions: sorted by position, keeping their order within
    /// one, so the first leg's holding before it is the position's opening.
    pub fn fold(&mut self) {
        if self.legs.is_empty() {
            return;
        }
        let mut all: Vec<((PartyId, u64), i64, i128)> =
            self.positions.drain(..).map(|(key, (opening, net))| (key, opening, net)).collect();
        all.append(&mut self.legs);
        all.sort_by_key(|(key, _, _)| *key);
        for (key, before, q) in all {
            match self.positions.last_mut() {
                Some((k, (_, net))) if *k == key => *net += q,
                _ => self.positions.push((key, (before, q))),
            }
        }
    }

    /// The positions folded, stopping the run where a leg is read before its fold.
    fn folded(&self) -> &[Position] {
        if !self.legs.is_empty() {
            phx_num::violation!(
                clause = "SET.8",
                "the day's positions read with legs not yet folded",
                legs = self.legs.len()
            );
        }
        &self.positions
    }

    /// The open instruction's sums moved to the day's, where they are unbalanced.
    fn flush(&mut self) {
        let Some(instruction) = self.open.take() else { return };
        self.flows.extend(self.open_flows.drain(..).filter(|(_, s)| *s != 0).map(|(d, s)| ((instruction, d), s)));
        self.money.extend(self.open_money.drain(..).filter(|(_, s)| *s != 0).map(|(d, s)| ((instruction, d), s)));
    }

    /// Each instruction and denomination's sum over the day's unbalanced parts and the open instruction's, where it
    /// is not nothing.
    fn unbalanced(rest: &[((u64, u32), i128)], open: Option<u64>, now: &[(u32, i128)]) -> Vec<((u64, u32), i128)> {
        let mut sums: BTreeMap<(u64, u32), i128> = BTreeMap::new();
        for (key, s) in rest {
            *sums.entry(*key).or_insert(0) += *s;
        }
        if let Some(instruction) = open {
            for (d, s) in now {
                *sums.entry((instruction, *d)).or_insert(0) += *s;
            }
        }
        sums.into_iter().filter(|(_, s)| *s != 0).collect()
    }

    /// Every instruction's paired legs sum to nothing in each denomination.
    #[clause("SET.9")]
    #[must_use]
    pub fn flow_gaps(&self) -> Vec<Gap> {
        Self::unbalanced(&self.flows, self.open, &self.open_flows)
            .into_iter()
            .map(|((instruction, denom), sum)| Gap::Flow { instruction, denom, sum })
            .collect()
    }

    /// Every change in a currency's money stock is met, leg for leg, by its issuer's own: an instruction's legs on money
    /// lines sum to nothing, the issuer's side taking what a holder's gains.
    #[clause("MON.8")]
    #[must_use]
    pub fn money_gaps(&self) -> Vec<Gap> {
        Self::unbalanced(&self.money, self.open, &self.open_money)
            .into_iter()
            .map(|((instruction, ccy), sum)| Gap::Money { instruction, ccy, sum })
            .collect()
    }

    /// For every position the day's legs touched, what it held at the day's first leg plus what came in less what went
    /// out is what it holds now, read from the books by `held`.
    #[clause("NUM.5", "SET.8")]
    pub fn unit_gaps(&self, held: &dyn Fn(PartyId, u64) -> i64) -> Vec<Gap> {
        self.folded()
            .iter()
            .filter_map(|&((party, account), (opening, net))| {
                let expected = i128::from(opening) + net;
                let now = held(party, account);
                (expected != i128::from(now)).then_some(Gap::Units { party, account, expected, held: now })
            })
            .collect()
    }

    /// The positions the day's legs touched, by party and account.
    pub fn positions(&self) -> impl Iterator<Item = (PartyId, u64)> + '_ {
        self.folded().iter().map(|(key, _)| *key)
    }

    /// Starts the next day's record.
    pub fn clear(&mut self) {
        self.open = None;
        self.open_flows.clear();
        self.open_money.clear();
        self.flow_keys = 0;
        self.flows.clear();
        self.money.clear();
        self.legs.clear();
        self.positions.clear();
        self.made.clear();
        self.worn.clear();
        self.stocks.clear();
    }
}

impl phx_core::LegRecords for Digests {
    fn instructions(&self) -> u64 {
        self.flow_keys
    }

    fn positions(&self) -> u64 {
        phx_rand::float::len_u64(self.folded().len())
    }

    fn flow_gaps(&self) -> Vec<phx_core::Gap> {
        Digests::flow_gaps(self).into_iter().map(Gap::found).collect()
    }

    fn money_gaps(&self) -> Vec<phx_core::Gap> {
        Digests::money_gaps(self).into_iter().map(Gap::found).collect()
    }

    fn unit_gaps(&self, books: &dyn phx_core::BooksAudit) -> Vec<phx_core::Gap> {
        Digests::unit_gaps(self, &|p, a| books.position(p, a)).into_iter().map(Gap::found).collect()
    }

    fn made(&self) -> Vec<phx_core::Made> {
        self.made.values().cloned().collect()
    }

    fn worn(&self) -> Vec<phx_core::Worn> {
        self.worn.values().cloned().collect()
    }

    fn stocks(&self) -> Vec<phx_core::StockDay> {
        self.stocks.values().cloned().collect()
    }
}

impl Gap {
    /// The gap as a family reports it: an instruction's gaps are the run's to own, since no one party broke them.
    fn found(self) -> phx_core::Gap {
        use phx_core::{FindingOwner, Unit};
        match self {
            Gap::Flow { instruction, denom, sum } => phx_core::Gap {
                owner: FindingOwner::Run,
                size: sum,
                unit: Unit::Count,
                detail: format!("instruction {instruction}: its paired legs in denomination {denom} sum to {sum}"),
            },
            Gap::Money { instruction, ccy, sum } => phx_core::Gap {
                owner: FindingOwner::Run,
                size: sum,
                unit: Unit::Count,
                detail: format!("instruction {instruction}: its legs on money lines in currency {ccy} sum to {sum}"),
            },
            Gap::Units { party, account, expected, held } => phx_core::Gap {
                owner: FindingOwner::Party(party),
                size: i128::from(held) - expected,
                unit: Unit::Count,
                detail: format!("party {}: account {account} holds {held} where its legs make {expected}", party.get()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use phx_core::LegDigest;
    use phx_id::PartyId;
    use phx_num::Missing;

    use super::{Digests, Gap};

    fn leg(flow: i64) -> LegDigest {
        LegDigest {
            party: PartyId::new(1),
            account: 7,
            denom: 3,
            qty: flow,
            flow,
            before: 0,
            paired: true,
            money: true,
            made: Missing::Absent,
            worn: Missing::Absent,
            source: Missing::Absent,
            issued: Missing::Absent,
            unit: 1,
        }
    }

    #[test]
    fn interleaved_instructions_balance() {
        let mut d = Digests::default();
        d.record(1, leg(5));
        d.record(2, leg(4));
        d.record(1, leg(-5));
        d.record(2, leg(-3));
        assert_eq!(d.flow_gaps(), vec![Gap::Flow { instruction: 2, denom: 3, sum: 1 }]);
        assert_eq!(d.money_gaps(), vec![Gap::Money { instruction: 2, ccy: 3, sum: 1 }]);
        d.record(3, leg(2));
        assert_eq!(d.flow_gaps().len(), 2, "the open instruction's sum counts before it closes");
    }
}
