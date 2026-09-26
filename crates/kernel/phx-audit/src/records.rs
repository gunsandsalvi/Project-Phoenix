use std::collections::BTreeMap;

use phx_core::LegDigest;
use phx_id::PartyId;
use phx_macros::clause;

/// The audit's own record of a day's settled legs, kept as they apply and apart from the books they moved, so the
/// families check the books against something the books did not write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Digests {
    /// Per instruction and denomination, the sum of its paired legs.
    flows: BTreeMap<(u64, u32), i128>,
    /// Per instruction and currency, the sum of its legs on money lines.
    money: BTreeMap<(u64, u32), i128>,
    /// Per party and account, what it held before the day's first leg on it, and the day's net of its legs.
    positions: BTreeMap<(PartyId, u64), (i64, i128)>,
    /// Per instruction that names a way, its legs made or used up.
    made: BTreeMap<u64, phx_core::Made>,
    /// Per instruction that wears plant, its legs along its chains.
    worn: BTreeMap<u64, phx_core::Worn>,
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
        if leg.paired {
            *self.flows.entry((instruction, leg.denom)).or_insert(0) += i128::from(leg.flow);
        }
        if leg.money {
            *self.money.entry((instruction, leg.denom)).or_insert(0) += q;
        }
        self.positions.entry((leg.party, leg.account)).or_insert((leg.before, 0)).1 += q;
        if let phx_num::Missing::Present(way) = leg.made {
            let made =
                self.made.entry(instruction).or_insert_with(|| phx_core::Made { instruction, way, legs: Vec::new() });
            made.legs.push(phx_core::MadeLeg { party: leg.party, denom: leg.denom, qty: leg.qty });
        }
        if let phx_num::Missing::Present((chain, class)) = leg.worn {
            let worn = self.worn.entry(instruction).or_insert_with(|| phx_core::Worn { instruction, legs: Vec::new() });
            worn.legs.push(phx_core::WornLeg { party: leg.party, chain, class, qty: leg.qty });
        }
    }

    /// Every instruction's paired legs sum to nothing in each denomination.
    #[clause("SET.9")]
    #[must_use]
    pub fn flow_gaps(&self) -> Vec<Gap> {
        self.flows
            .iter()
            .filter(|(_, s)| **s != 0)
            .map(|((instruction, denom), sum)| Gap::Flow { instruction: *instruction, denom: *denom, sum: *sum })
            .collect()
    }

    /// Every change in a currency's money stock is met, leg for leg, by its issuer's own: an instruction's legs on money
    /// lines sum to nothing, the issuer's side taking what a holder's gains.
    #[clause("MON.8")]
    #[must_use]
    pub fn money_gaps(&self) -> Vec<Gap> {
        self.money
            .iter()
            .filter(|(_, s)| **s != 0)
            .map(|((instruction, ccy), sum)| Gap::Money { instruction: *instruction, ccy: *ccy, sum: *sum })
            .collect()
    }

    /// For every position the day's legs touched, what it held at the day's first leg plus what came in less what went
    /// out is what it holds now, read from the books by `held`.
    #[clause("NUM.5", "SET.8")]
    pub fn unit_gaps(&self, held: &dyn Fn(PartyId, u64) -> i64) -> Vec<Gap> {
        self.positions
            .iter()
            .filter_map(|((party, account), (opening, net))| {
                let expected = i128::from(*opening) + net;
                let now = held(*party, *account);
                (expected != i128::from(now)).then_some(Gap::Units {
                    party: *party,
                    account: *account,
                    expected,
                    held: now,
                })
            })
            .collect()
    }

    /// The positions the day's legs touched, by party and account.
    pub fn positions(&self) -> impl Iterator<Item = (PartyId, u64)> + '_ {
        self.positions.keys().copied()
    }

    /// Starts the next day's record.
    pub fn clear(&mut self) {
        self.flows.clear();
        self.money.clear();
        self.positions.clear();
        self.made.clear();
        self.worn.clear();
    }
}

impl phx_core::LegRecords for Digests {
    fn instructions(&self) -> u64 {
        phx_rand::float::len_u64(self.flows.len())
    }

    fn positions(&self) -> u64 {
        phx_rand::float::len_u64(self.positions.len())
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
