use std::collections::{BTreeMap, BTreeSet};

use phx_id::{LineId, PartyId};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_rand::{Draws, below_u64};

/// The parties the `closed` fact names, which the system registered as its writer sets (a bank or a clearing house
/// under resolution, a closed insurer): legs through a closed issuer, and every leg a closed payer owes, wait
/// pending. On a many-party line a closed payer shares with others, the holders whose credits are its own are drawn
/// once, when it closes, and stay paired until its resolution settles. With no writer registered, nothing is closed.
#[clause("MON.5", "REP.23")]
#[derive(Clone, Debug, Default, PartialEq, Eq, phx_macros::Saved)]
pub struct Closed {
    pub issuers: BTreeSet<PartyId>,
    pub payers: BTreeSet<PartyId>,
    pairings: BTreeMap<(PartyId, LineId), Vec<PartyId>>,
}

impl Closed {
    /// Whether nothing is closed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.issuers.is_empty() && self.payers.is_empty()
    }

    /// Whether a payment waits pending: its payer is closed, or a party its money passes through is a closed issuer.
    #[must_use]
    pub fn holds(&self, payer: PartyId, through: &[PartyId]) -> bool {
        self.payers.contains(&payer) || through.iter().any(|p| self.issuers.contains(p))
    }

    /// The holders of a many-party line whose credits are a closed payer's: `count` of them drawn from the line's
    /// holders the first time it is asked, the same holders every time after, until the resolution settles.
    pub fn pairing(
        &mut self,
        payer: PartyId,
        line: LineId,
        holders: &[PartyId],
        count: usize,
        draws: &mut Draws,
    ) -> &[PartyId] {
        if count > holders.len() {
            violation!(clause = "REP.23", "a pairing of more holders than the line has", count = count);
        }
        self.pairings.entry((payer, line)).or_insert_with(|| {
            let mut pool = holders.to_vec();
            for i in 0..count {
                let left = phx_rand::float::len_u64(pool.len() - i);
                let Ok(j) = usize::try_from(below_u64(draws, left)) else {
                    capacity_exceeded!("holders of a line", usize::MAX, 0);
                };
                pool.swap(i, i + j);
            }
            pool.truncate(count);
            pool
        })
    }

    /// A resolution settled: its payer's pairings end with it.
    pub fn settled(&mut self, payer: PartyId) {
        self.payers.remove(&payer);
        self.pairings.retain(|(p, _), _| *p != payer);
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{LineId, PartyId};
    use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};

    use super::Closed;

    fn draws(day: u32) -> Draws {
        Draws::new(stream_key(Seed::new(1), "REP.pairing"), Subject::new(SubjectTag::World, 0), day, 0)
    }

    #[test]
    fn closed_payer_pairing_drawn_once() {
        let mut closed = Closed::default();
        let insurer = PartyId::new(9);
        closed.payers.insert(insurer);
        let holders: Vec<PartyId> = (1..=10).map(PartyId::new).collect();
        let line = LineId::new(3);
        let first = closed.pairing(insurer, line, &holders, 4, &mut draws(1)).to_vec();
        let second = closed.pairing(insurer, line, &holders, 4, &mut draws(2)).to_vec();
        assert_eq!(first.len(), 4);
        assert_eq!(first, second, "the second day's draws change nothing: the pairing was drawn once");
        assert!(closed.holds(insurer, &[]));
        assert!(!closed.holds(PartyId::new(1), &[]), "another payer on the line pays as before");
        closed.settled(insurer);
        let after = closed.pairing(insurer, line, &holders, 4, &mut draws(2)).to_vec();
        assert!(!closed.holds(insurer, &[]));
        assert_eq!(after.len(), 4, "a later resolution draws afresh");
    }
}
