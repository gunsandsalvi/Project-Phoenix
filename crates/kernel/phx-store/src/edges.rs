//! A family of contracts: each one a row between its two named parties, holding the words its family reads on the
//! day's paths beside them, so a contract is read in one line. A side that keeps its parties' lists threads each
//! party's contracts of the family through the rows, from a head the party's kind keeps, so opening and closing a
//! contract takes the same time however many a party holds.

use phx_id::{PartyKey, Slot};
use phx_macros::{Pod, clause};
use phx_num::violation;

use crate::backing::{AddressSpace, Backing, SystemBacking};
use crate::column::Column;
use crate::pod::Pod;
use crate::table::SlotAlloc;

/// A family's contract row: its two parties, side by side with the words the family keeps.
pub trait Row: Pod {
    fn ends(&self) -> [PartyKey; 2];
    fn set_end(&mut self, side: usize, party: PartyKey);
}

/// A contract with no words of its own beyond its two parties.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod)]
pub struct Pair {
    pub ends: [PartyKey; 2],
}

impl Row for Pair {
    fn ends(&self) -> [PartyKey; 2] {
        self.ends
    }

    fn set_end(&mut self, side: usize, party: PartyKey) {
        if let Some(e) = self.ends.get_mut(side) {
            *e = party;
        }
    }
}

/// No contract: the end of a party's list, or an empty head.
pub const NONE: u32 = u32::MAX;

/// A side's list links: each contract's next and previous contract of the same party on that side.
#[derive(Debug)]
struct Links<B: Backing> {
    next: Column<u32, B>,
    prev: Column<u32, B>,
}

/// A family's contracts: slots, each contract's row, and the list links of each side that keeps lists.
#[clause("REP.3", "REG.8")]
#[derive(Debug)]
pub struct EdgeTable<R: Row = Pair, B: Backing = SystemBacking> {
    slots: SlotAlloc<B>,
    rows: Column<R, B>,
    links: [Option<Links<B>>; 2],
    max: u32,
    rows_per_chunk: u32,
}

impl<R: Row, B: Backing> EdgeTable<R, B> {
    /// A family of at most `max` contracts, whose sides keep their parties' lists where `listed` says.
    pub fn new(space: &mut AddressSpace, max: u32, rows_per_chunk: u32, listed: [bool; 2]) -> EdgeTable<R, B> {
        let links = listed.map(|l| {
            l.then(|| Links {
                next: Column::new(space, max, rows_per_chunk),
                prev: Column::new(space, max, rows_per_chunk),
            })
        });
        EdgeTable {
            slots: SlotAlloc::new(space, max),
            rows: Column::new(space, max, rows_per_chunk),
            links,
            max,
            rows_per_chunk,
        }
    }

    /// A column of the family, one value a contract slot, for its declarer to keep.
    pub fn column<T: crate::pod::Pod>(&self, space: &mut AddressSpace) -> Column<T, B> {
        Column::new(space, self.max, self.rows_per_chunk)
    }

    /// Opens a contract between its row's two parties, putting it first on the lists of the sides that keep them,
    /// whose heads the caller hands in.
    pub fn open(&mut self, row: R, heads: [Option<&mut u32>; 2]) -> Slot {
        let edge = self.slots.alloc();
        self.rows.put(edge, row);
        for (links, head) in self.links.iter_mut().zip(heads) {
            match (links, head) {
                (Some(l), Some(h)) => {
                    l.next.put(edge, *h);
                    l.prev.put(edge, NONE);
                    if *h != NONE {
                        l.prev.set(Slot::new(*h), edge.get());
                    }
                    *h = edge.get();
                }
                (None, None) => {}
                _ => violation!(clause = "REP.3", "a side's list head handed in where the side keeps none, or not"),
            }
        }
        edge
    }

    /// Closes a contract, taking it off its sides' lists; its slot is handed out again after the day closes.
    pub fn close(&mut self, edge: Slot, heads: [Option<&mut u32>; 2]) {
        if !self.slots.is_live(edge) {
            violation!(clause = "REP.3", "a contract closed that is not open", edge = edge.get());
        }
        for (links, head) in self.links.iter_mut().zip(heads) {
            unlink(links.as_mut(), head, edge);
        }
        self.slots.release(edge);
    }

    /// Moves a contract's side to another party, as a succession or a transfer does, off the old party's list and onto
    /// the new one's.
    pub fn move_end(
        &mut self,
        edge: Slot,
        side: usize,
        to: PartyKey,
        from_head: Option<&mut u32>,
        to_head: Option<&mut u32>,
    ) {
        let (Some(mut row), Some(links)) = (self.rows.get(edge), self.links.get_mut(side)) else {
            violation!(clause = "REP.3", "a contract has two sides", side = side);
        };
        row.set_end(side, to);
        self.rows.set(edge, row);
        unlink(links.as_mut(), from_head, edge);
        match (links.as_mut(), to_head) {
            (Some(l), Some(h)) => {
                l.next.set(edge, *h);
                l.prev.set(edge, NONE);
                if *h != NONE {
                    l.prev.set(Slot::new(*h), edge.get());
                }
                *h = edge.get();
            }
            (None, None) => {}
            _ => violation!(clause = "REP.3", "a side's list head handed in where the side keeps none, or not"),
        }
    }

    /// The party on a contract's side.
    #[must_use]
    pub fn end(&self, edge: Slot, side: usize) -> Option<PartyKey> {
        self.rows.get(edge).and_then(|r| r.ends().get(side).copied())
    }

    /// A contract's row.
    #[must_use]
    pub fn row(&self, edge: Slot) -> Option<R> {
        self.rows.get(edge)
    }

    /// Every slot's row below the high water, open or not, to read in slot order.
    pub fn rows(&self) -> &[R] {
        self.rows.slice()
    }

    /// Every slot's row, for the family to write its own words; a party is moved by `move_end`, which keeps the
    /// lists.
    pub fn rows_mut(&mut self) -> &mut [R] {
        self.rows.slice_mut()
    }

    #[must_use]
    pub fn is_open(&self, edge: Slot) -> bool {
        self.slots.is_live(edge)
    }

    /// A party's contracts on a side that keeps lists, from its head, most recently opened first; a side that keeps
    /// none is refused.
    pub fn list(&self, side: usize, head: u32) -> impl Iterator<Item = Slot> + '_ {
        let Some(Some(links)) = self.links.get(side) else {
            violation!(clause = "REP.3", "a party's contracts listed on a side that keeps no lists", side = side);
        };
        let mut at = head;
        std::iter::from_fn(move || {
            if at == NONE {
                return None;
            }
            let edge = Slot::new(at);
            let Some(next) = links.next.get(edge) else {
                violation!(clause = "REP.3", "a listed contract with no link", edge = edge.get());
            };
            at = next;
            Some(edge)
        })
    }

    /// Slots ever handed out: every open contract's slot lies below.
    #[must_use]
    pub fn high_water(&self) -> u32 {
        self.slots.high_water()
    }

    pub fn open_slots(&self) -> impl Iterator<Item = Slot> + '_ {
        self.slots.live_slots()
    }

    /// Returns the day's closed slots to use.
    pub fn close_day(&mut self) {
        self.slots.close_day();
    }
}

/// Takes a contract off one side's list, whose head is handed in where the side keeps lists.
fn unlink<B: Backing>(links: Option<&mut Links<B>>, head: Option<&mut u32>, edge: Slot) {
    match (links, head) {
        (Some(l), Some(h)) => {
            let (Some(next), Some(prev)) = (l.next.get(edge), l.prev.get(edge)) else {
                violation!(clause = "REP.3", "a listed contract with no links", edge = edge.get());
            };
            if prev == NONE {
                if *h != edge.get() {
                    violation!(clause = "REP.3", "a contract first on no list but its party's head names another");
                }
                *h = next;
            } else {
                l.next.set(Slot::new(prev), next);
            }
            if next != NONE {
                l.prev.set(Slot::new(next), prev);
            }
        }
        (None, None) => {}
        _ => violation!(clause = "REP.3", "a side's list head handed in where the side keeps none, or not"),
    }
}

#[cfg(test)]
mod tests {
    use phx_id::{PartyKey, Slot};

    use super::{EdgeTable, NONE, Pair};
    use crate::backing::{AddressSpace, HeapBacking};

    fn party(slot: u32) -> PartyKey {
        PartyKey::new(1, Slot::new(slot))
    }

    #[test]
    fn adjacency_insert_remove_iterate() {
        let mut space = AddressSpace::empty();
        let mut t: EdgeTable<Pair, HeapBacking> = EdgeTable::new(&mut space, 64, 16, [true, false]);
        let mut employer = NONE;
        let jobs: Vec<Slot> =
            (0..4).map(|w| t.open(Pair { ends: [party(0), party(10 + w)] }, [Some(&mut employer), None])).collect();
        let listed: Vec<Slot> = t.list(0, employer).collect();
        assert_eq!(listed, jobs.iter().rev().copied().collect::<Vec<_>>(), "most recent first");
        t.close(jobs[1], [Some(&mut employer), None]);
        t.close(jobs[3], [Some(&mut employer), None]);
        assert_eq!(t.list(0, employer).collect::<Vec<_>>(), vec![jobs[2], jobs[0]], "a middle and a head removed");
        let mut other = NONE;
        t.move_end(jobs[0], 0, party(5), Some(&mut employer), Some(&mut other));
        assert_eq!(t.list(0, employer).collect::<Vec<_>>(), vec![jobs[2]]);
        assert_eq!(t.list(0, other).collect::<Vec<_>>(), vec![jobs[0]]);
        assert_eq!(t.end(jobs[0], 0), Some(party(5)));
        t.close(jobs[2], [Some(&mut employer), None]);
        assert_eq!(employer, NONE, "the last contract off leaves the head empty");
    }
}
