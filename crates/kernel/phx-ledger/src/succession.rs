//! Parties that ended into a successor while some of their units were still bound — covering an offer not yet
//! settled, or pledged — and which units of a holding its holder cannot give. A successor takes its predecessor's
//! holdings with what binds them: the covers and liens stay under the name they were placed in, and count on the
//! successor, since an instruction naming the predecessor settles with it.

use std::collections::BTreeMap;

use phx_id::{InstrumentId, PartyId};
use phx_macros::clause;
use phx_num::{Qty, violation};

use crate::apply::Ledger;
use crate::covered::{Covered, Uncovered};
use crate::lien::LienKey;
use phx_store::Backing;

/// Each ended party with units still bound under its name, and its successor; and each successor's direct
/// predecessors, so a party none succeeded is read in one look-up.
#[derive(Clone, Debug, Default, phx_macros::Saved)]
pub struct Successions {
    successor: BTreeMap<PartyId, PartyId>,
    predecessors: BTreeMap<PartyId, Vec<PartyId>>,
}

impl Successions {
    /// A party that has passed what it held on to its successor.
    #[must_use]
    pub fn has_successor(&self, party: PartyId) -> bool {
        self.successor.contains_key(&party)
    }

    /// Whether a covers' or liens' holder is the party or one it succeeded, following successors to their end.
    #[must_use]
    pub fn stands_for(&self, holder: PartyId, party: PartyId) -> bool {
        let mut at = holder;
        loop {
            if at == party {
                return true;
            }
            match self.successor.get(&at) {
                Some(next) => at = *next,
                None => return false,
            }
        }
    }

    /// The parties a party succeeded, directly or through others, that still have units bound under their names.
    fn predecessors(&self, party: PartyId) -> Vec<PartyId> {
        let mut out: Vec<PartyId> = Vec::new();
        let mut next = vec![party];
        while let Some(p) = next.pop() {
            for q in self.predecessors.get(&p).into_iter().flatten() {
                out.push(*q);
                next.push(*q);
            }
        }
        out.sort_unstable();
        out
    }

    fn record(&mut self, from: PartyId, to: PartyId) {
        self.successor.insert(from, to);
        self.predecessors.entry(to).or_default().push(from);
    }

    /// The successions whose predecessor nothing binds any more, forgotten, a chain's from its first link on: a party
    /// that succeeded one still bound stays, so that one is still reached from the end of the chain.
    fn prune(&mut self, binds: impl Fn(PartyId) -> bool) {
        loop {
            let gone: Vec<PartyId> = self
                .successor
                .keys()
                .copied()
                .filter(|p| !binds(*p) && self.predecessors.get(p).is_none_or(Vec::is_empty))
                .collect();
            if gone.is_empty() {
                return;
            }
            self.forget(&gone);
        }
    }

    fn forget(&mut self, gone: &[PartyId]) {
        for p in gone.iter().copied() {
            if let Some(to) = self.successor.remove(&p)
                && let Some(list) = self.predecessors.get_mut(&to)
            {
                list.retain(|q| *q != p);
                if list.is_empty() {
                    self.predecessors.remove(&to);
                }
            }
        }
    }
}

impl<B: Backing> Ledger<B> {
    /// A party's holdings passing to its successor with what binds them.
    #[clause("PTY.9", "PTY.10", "L3")]
    pub fn succeed(&mut self, from: PartyId, to: PartyId) {
        if from == to || self.successions.stands_for(to, from) {
            violation!(clause = "PTY.9", "a party succeeding itself", party = from.get());
        }
        self.successions.record(from, to);
    }

    /// The units of a holding its holder cannot give: pledged, or covering an offer not yet settled, under its own
    /// name or those of the parties it succeeded; none for a party whose holdings have passed to its successor.
    #[clause("REG.10", "REG.16")]
    #[must_use]
    pub fn bound(&self, party: PartyId, id: InstrumentId) -> i64 {
        if self.successions.has_successor(party) {
            return 0;
        }
        let under = |p: PartyId| self.liens.pledged(p, id) + self.covers.committed(p, id);
        under(party) + self.successions.predecessors(party).into_iter().map(under).sum::<i64>()
    }

    /// Units covering an offer, from those the holder holds that nothing else binds.
    ///
    /// # Errors
    /// `Uncovered` when fewer units are free than the offer names.
    pub fn cover(&mut self, holder: PartyId, id: InstrumentId, qty: Qty, held: i64) -> Result<Covered, Uncovered> {
        let elsewhere = self.bound(holder, id) - self.covers.committed(holder, id);
        self.covers.cover(holder, id, qty, held, elsewhere)
    }

    /// Units of a holding pledged to a party, from those the holder holds that nothing else binds.
    pub fn pledge(&mut self, holder: PartyId, id: InstrumentId, units: i64, to: PartyId, held: i64) -> LienKey {
        let elsewhere = self.bound(holder, id) - self.liens.pledged(holder, id);
        self.liens.pledge(holder, id, units, to, held, elsewhere)
    }

    /// Whether a cover an instruction carries holds back the party's own units: placed under its name or one it
    /// succeeded.
    pub(crate) fn covers_for(&self, covered: &Covered, party: PartyId, id: InstrumentId) -> bool {
        covered.instrument() == id && self.successions.stands_for(covered.holder(), party)
    }

    /// The successions whose predecessors have nothing bound under their names any more, forgotten.
    pub fn prune_successions(&mut self) {
        let (liens, covers) = (&self.liens, &self.covers);
        self.successions.prune(|p| liens.binds(p) || covers.binds(p));
    }
}

#[cfg(test)]
mod tests {
    use phx_id::PartyId;

    use super::Successions;

    #[test]
    fn a_successor_stands_for_its_predecessors() {
        let mut s = Successions::default();
        let (firm, estate, heir) = (PartyId::new(1), PartyId::new(2), PartyId::new(3));
        s.record(firm, estate);
        s.record(estate, heir);
        assert!(s.stands_for(firm, heir) && s.stands_for(estate, heir) && s.stands_for(heir, heir));
        assert!(!s.stands_for(heir, firm));
        assert!(s.has_successor(firm) && !s.has_successor(heir));
        assert_eq!(s.predecessors(heir), vec![firm, estate]);
        assert!(s.predecessors(firm).is_empty());
        s.prune(|p| p == firm);
        assert!(s.has_successor(estate), "a successor with a predecessor still bound stays");
        s.prune(|_| false);
        assert!(!s.has_successor(firm) && !s.has_successor(estate), "then both are forgotten, in turn");
    }
}
