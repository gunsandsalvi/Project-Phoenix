use phx_id::{Day, PartyId, RowRef};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_store::LogicalHasher;

use crate::map::KernelMap;

/// A live party's row, and how many records name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Live {
    row: RowRef,
    refs: u32,
}

/// An ended party kept while records name it: the day it ended, its successor, and how many records name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Ended {
    day: Day,
    successor: Missing<PartyId>,
    refs: u32,
}

/// What the directory knows of an identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartyState {
    Live(RowRef),
    Ended {
        day: Day,
        successor: Missing<PartyId>,
    },
    /// Never handed out, or ended with nothing naming it any longer.
    Unknown,
}

/// Where following an identity's successors ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resolved {
    Live(PartyId, RowRef),
    /// A party that ended with no successor: an estate that has distributed, whose heirs are many.
    Ended(PartyId),
    Unknown,
}

/// Every party's identity: live ones to their rows, ended ones while a record names them. Identities come from one
/// counter and are never reused.
#[clause("PTY.1", "PTY.9", "PTY.10", "PTY.13")]
#[derive(Debug)]
pub struct Directory {
    next: u64,
    live: KernelMap<PartyId, Live>,
    ended: KernelMap<PartyId, Ended>,
}

impl Default for Directory {
    fn default() -> Self {
        Directory::new()
    }
}

impl Directory {
    #[must_use]
    pub fn new() -> Directory {
        Directory { next: 1, live: KernelMap::new(), ended: KernelMap::new() }
    }

    /// The next identity that will be handed out; every identity below it was handed out once.
    #[must_use]
    pub fn next(&self) -> u64 {
        self.next
    }

    /// Feeds every identity the directory holds to the world's hash, in order of identity.
    pub fn hash_into(&self, h: &mut LogicalHasher) {
        h.u64(self.next);
        for (party, live) in self.live.sorted() {
            h.u64(party.get());
            h.u64(u64::from(live.row.table.get()));
            h.u64(u64::from(live.row.slot.get()));
            h.u64(u64::from(live.refs));
        }
        for (party, ended) in self.ended.sorted() {
            h.u64(party.get());
            h.u64(u64::from(ended.day.get()));
            match ended.successor {
                Missing::Present(p) => {
                    h.u64(1);
                    h.u64(p.get());
                }
                Missing::Absent => h.u64(0),
            }
            h.u64(u64::from(ended.refs));
        }
    }

    #[must_use]
    pub fn live_count(&self) -> usize {
        self.live.len()
    }

    /// A new party at a row, with the next identity.
    #[clause("PTY.9")]
    pub fn begin(&mut self, row: RowRef) -> PartyId {
        let id = PartyId::new(self.next);
        self.next += 1;
        self.live.insert(id, Live { row, refs: 0 });
        id
    }

    /// Moves a live party to another row, as renumbering does.
    pub fn relocate(&mut self, id: PartyId, row: RowRef) {
        let Some(live) = self.live.get_mut(id) else {
            violation!(clause = "PTY.10", "a party moved that is not live", party = id.get());
        };
        live.row = row;
    }

    /// Ends a live party, naming its successor if it has one; the successor must be live. The record is kept while
    /// anything names the ended party.
    #[clause("PTY.9", "PTY.13")]
    pub fn end(&mut self, id: PartyId, day: Day, successor: Missing<PartyId>) {
        let Some(live) = self.live.remove(id) else {
            violation!(clause = "PTY.13", "a party ended that is not live", party = id.get());
        };
        if let Missing::Present(s) = successor
            && self.live.get(s).is_none()
        {
            violation!(clause = "PTY.9", "a successor that is not live", party = id.get(), successor = s.get());
        }
        if live.refs == 0 {
            return;
        }
        if let Missing::Present(s) = successor {
            self.retain(s);
        }
        self.ended.insert(id, Ended { day, successor, refs: live.refs });
    }

    /// A record now names the party.
    #[clause("PTY.10")]
    pub fn retain(&mut self, id: PartyId) {
        let refs = if let Some(live) = self.live.get_mut(id) {
            &mut live.refs
        } else if let Some(ended) = self.ended.get_mut(id) {
            &mut ended.refs
        } else {
            violation!(clause = "PTY.10", "a record naming a party the directory does not hold", party = id.get());
        };
        let Some(more) = refs.checked_add(1) else {
            capacity_exceeded!("records naming one party", u32::MAX, u64::from(u32::MAX) + 1);
        };
        *refs = more;
    }

    /// A record naming the party is gone; an ended party that nothing names is forgotten, and so, in turn, may be its
    /// successor's record.
    #[clause("PTY.10")]
    pub fn release(&mut self, id: PartyId) {
        let mut next = Some(id);
        while let Some(id) = next.take() {
            if let Some(live) = self.live.get_mut(id) {
                live.refs = decrement(live.refs, id);
                continue;
            }
            let Some(ended) = self.ended.get_mut(id) else {
                violation!(clause = "PTY.10", "a release of a party no record names", party = id.get());
            };
            ended.refs = decrement(ended.refs, id);
            if ended.refs == 0 {
                let successor = ended.successor;
                self.ended.remove(id);
                if let Missing::Present(s) = successor {
                    next = Some(s);
                }
            }
        }
    }

    #[must_use]
    pub fn lookup(&self, id: PartyId) -> PartyState {
        if let Some(live) = self.live.get(id) {
            return PartyState::Live(live.row);
        }
        match self.ended.get(id) {
            Some(e) => PartyState::Ended { day: e.day, successor: e.successor },
            None => PartyState::Unknown,
        }
    }

    /// Follows successors to a live party, or to one that ended with none.
    #[clause("PTY.10")]
    #[must_use]
    pub fn resolve(&self, id: PartyId) -> Resolved {
        let mut at = id;
        loop {
            match self.lookup(at) {
                PartyState::Live(row) => return Resolved::Live(at, row),
                PartyState::Ended { successor: Missing::Present(s), .. } => at = s,
                PartyState::Ended { successor: Missing::Absent, .. } => return Resolved::Ended(at),
                PartyState::Unknown => return Resolved::Unknown,
            }
        }
    }
}

fn decrement(refs: u32, id: PartyId) -> u32 {
    let Some(fewer) = refs.checked_sub(1) else {
        violation!(clause = "PTY.10", "a release of a party no record names", party = id.get());
    };
    fewer
}

#[cfg(test)]
mod tests {
    use phx_id::{Day, PartyId, RowRef, Slot, TableId};
    use phx_num::Missing;

    use super::{Directory, PartyState, Resolved};

    fn row(slot: u32) -> RowRef {
        RowRef { table: TableId::new(0), slot: Slot::new(slot) }
    }

    #[test]
    fn directory_never_reuses() {
        let mut d = Directory::new();
        let a = d.begin(row(0));
        d.end(a, Day::new(1), Missing::Absent);
        let b = d.begin(row(0));
        assert!(b > a, "the same slot, a new identity");
        assert_eq!((d.lookup(a), d.next()), (PartyState::Unknown, 3));
    }

    #[test]
    fn directory_resolves_successor_chain() {
        let mut dir = Directory::new();
        let (first, second, third) = (dir.begin(row(0)), dir.begin(row(1)), dir.begin(row(2)));
        dir.retain(first);
        dir.end(first, Day::new(1), Missing::Present(second));
        dir.end(second, Day::new(2), Missing::Present(third));
        assert_eq!(dir.resolve(first), Resolved::Live(third, row(2)), "the first's record keeps the second's");
        let lone = dir.begin(row(3));
        dir.retain(lone);
        dir.end(lone, Day::new(6), Missing::Absent);
        assert_eq!(dir.resolve(lone), Resolved::Ended(lone));
    }

    #[test]
    fn directory_drops_unreferenced() {
        let mut d = Directory::new();
        let (a, heir) = (d.begin(row(0)), d.begin(row(1)));
        d.retain(a);
        d.retain(a);
        d.end(a, Day::new(3), Missing::Present(heir));
        d.release(a);
        assert!(matches!(d.lookup(a), PartyState::Ended { .. }), "one record still names it");
        d.release(a);
        assert_eq!(d.lookup(a), PartyState::Unknown);
        assert!(matches!(d.lookup(heir), PartyState::Live(_)));
        let caught = std::panic::catch_unwind(move || d.release(PartyId::new(1)));
        assert!(caught.is_err(), "a release of a party no record names");
    }

    #[test]
    fn directory_distributed_estate_reads_ended() {
        let mut d = Directory::new();
        let estate = d.begin(row(0));
        d.retain(estate);
        d.end(estate, Day::new(9), Missing::Absent);
        assert_eq!(d.resolve(estate), Resolved::Ended(estate));
        assert_eq!(d.lookup(estate), PartyState::Ended { day: Day::new(9), successor: Missing::Absent });
    }
}
