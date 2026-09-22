//! The week loop: THIRTY-ONE SLOTS in the order Money G2 fixes, held as DATA and run one at a time.

use crate::calendar::{Calendar, Week};

/// What one phase hands another inside a week: an event of a named journal kind.
///
/// A print is not one. Every print is written at f by the one solver, and every phase that reads one
/// runs in a later slot by Money G2's own order, so a print needs no declaration to be ordered — and
/// a variant nothing constructs is a shape with no producer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Produces(pub u32);

pub struct PhaseDecl {
    pub name: u32,
    pub owner: u32,
    /// Which of the thirty-one slots it runs in.
    pub at: u32,
    /// What it needs of THIS week. A phase that wants an earlier week's rows is ordered by the
    /// calendar and declares nothing: running before the writer costs it a week, not an answer.
    pub reads: Vec<Produces>,
    /// What its own module says into the journal. A kernel door the phase opens — settlement, a
    /// cessation — writes the kernel's record and not this phase's.
    pub writes: Vec<Produces>,
}

/// THE NINE STAGES OF A PERIOD (Money G2). A week settles ONCE, so this is the whole of the
/// structure time has here — and each stage is a group of the slots below, which is what a phase
/// actually names.
/// The week opens: what an earlier week scheduled for this one arrives.
pub const OPENS: u32 = 0;
/// What the past owes resolves: accrue, due, loss, cease, estate.
pub const OWED: u32 = 1;
/// The population changes, before anybody acts.
pub const POPULATION: u32 = 2;
/// The real work is done, so there is something to sell and something to bid for.
pub const WORK: u32 = 3;
/// Every deciding party forms its own view, once.
pub const VIEWS: u32 = 4;
/// The books clear, once.
pub const BOOKS: u32 = 5;
/// What printed is valued and judged.
pub const JUDGED: u32 = 6;
/// What the judgement implies is scheduled, for the week AFTER.
pub const SCHEDULED: u32 = 7;
/// The week closes: the gridlock pass, then the audit.
pub const CLOSES: u32 = 8;

/// THE THIRTY-ONE SLOTS OF A PERIOD, which is what a stage is made of. G2 spells the sub-order of
/// seven of the nine out — "in that order, each reading the one before" — so a phase runs in a SLOT
/// and a stage is the group of them a reader still thinks in.
/// Offers that stood to a past week expire, and what was in flight whose time has come closes.
pub const A1: u32 = 0;
/// Payments whose deadline has passed are given up as fails.
pub const A2: u32 = 1;
/// What accrues accrues.
pub const B1: u32 = 2;
/// What falls due is paid, or becomes an arrear.
pub const B2: u32 = 3;
/// An unpaid claim becomes a named holder's loss.
pub const B3: u32 = 4;
/// A party that cannot go on ceases — read after the week's losses are booked.
pub const B4: u32 = 5;
/// And its estate distributes, because no death is without a destination.
pub const B5: u32 = 6;
/// Entry.
pub const C1: u32 = 7;
/// Death.
pub const C2: u32 = 8;
/// Promotion.
pub const C3: u32 = 9;
/// Split.
pub const C4: u32 = 10;
/// Merge.
pub const C5: u32 = 11;
/// Lines run and draw their inputs.
pub const D1: u32 = 12;
/// Batches finish.
pub const D2: u32 = 13;
/// Plant wears.
pub const D3: u32 = 14;
/// Goods move.
pub const D4: u32 = 15;
/// Engagements are made and ended.
pub const D5: u32 = 16;
/// And whoever is short brings paper — all of it before the market.
pub const D6: u32 = 17;
/// Every deciding party forms its own view, once, from its own history.
pub const E1: u32 = 18;
/// And then posts what it wants at a price it will pay. The kernel's, because when a party may post
/// is not a thing any system chooses.
pub const E2: u32 = 19;
/// The books clear, once, per market and instrument.
pub const F: u32 = 20;
/// Positions marked.
pub const G1: u32 = 21;
/// Gains and losses landed on named balance sheets.
pub const G2: u32 = 22;
/// Derived levels read.
pub const G3: u32 = 23;
/// Constraints tested.
pub const G4: u32 = 24;
/// Accounts published.
pub const G5: u32 = 25;
/// Opinions formed on what was published.
pub const G6: u32 = 26;
/// And what is public made visible, last.
pub const G7: u32 = 27;
/// What the judgement implies is scheduled, for the week AFTER (G1.c).
pub const H: u32 = 28;
/// One pass over every payment the week holds, so a ring that can settle together does.
pub const I1: u32 = 29;
/// Then the audit, over what the week actually left behind.
pub const I2: u32 = 30;

/// Which stage each slot belongs to, in slot order — the whole of the week, twice over.
pub const SLOTS: [(u32, u32); 31] = [
    (A1, OPENS),
    (A2, OPENS),
    (B1, OWED),
    (B2, OWED),
    (B3, OWED),
    (B4, OWED),
    (B5, OWED),
    (C1, POPULATION),
    (C2, POPULATION),
    (C3, POPULATION),
    (C4, POPULATION),
    (C5, POPULATION),
    (D1, WORK),
    (D2, WORK),
    (D3, WORK),
    (D4, WORK),
    (D5, WORK),
    (D6, WORK),
    (E1, VIEWS),
    (E2, VIEWS),
    (F, BOOKS),
    (G1, JUDGED),
    (G2, JUDGED),
    (G3, JUDGED),
    (G4, JUDGED),
    (G5, JUDGED),
    (G6, JUDGED),
    (G7, JUDGED),
    (H, SCHEDULED),
    (I1, CLOSES),
    (I2, CLOSES),
];

/// Whose a slot marker is, so the one pass can tell a slot from a module's phase in it.
pub const KERNEL: u32 = u32::MAX;

pub struct Phases {
    order: Vec<PhaseDecl>,
    sealed: bool,
}

impl Default for Phases {
    fn default() -> Self {
        Self::new()
    }
}

impl Phases {
    pub fn new() -> Self {
        Self {
            order: SLOTS
                .iter()
                .map(|(slot, _)| PhaseDecl {
                    name: *slot,
                    owner: KERNEL,
                    at: *slot,
                    reads: vec![],
                    writes: vec![],
                })
                .collect(),
            sealed: false,
        }
    }

    /// A module's phase is INSERTED into its slot, after whatever is already in it.
    pub fn add(&mut self, decl: PhaseDecl) {
        assert!(!self.sealed, "Law 10: phases are declared at assembly");
        assert!(
            !self.order.iter().any(|p| p.name == decl.name),
            "Law 4: a phase is declared twice"
        );
        let slot = self
            .order
            .iter()
            .position(|p| p.name == decl.at && p.owner == KERNEL)
            .expect("Money G2: a phase runs in one of the thirty-one slots");
        // After the slot marker and after its siblings, so a slot runs in assembly order and a
        // phase cannot land in the slot before it.
        let mut insert = slot + 1;
        while insert < self.order.len() && self.order[insert].at == decl.at {
            insert += 1;
        }
        self.order.insert(insert, decl);
    }

    /// Once sealed, the order is what a week runs and no phase may be added.
    pub fn seal(&mut self) {
        let mut written_by: Vec<(Produces, usize, u32)> = Vec::new();
        for (at, phase) in self.order.iter().enumerate() {
            for w in &phase.writes {
                // Law 4: one fact, one writer. Two phases saying the same kind leave a reader
                // between them holding half of the week, and neither half says it is a half.
                if let Some(&(_, _, first)) = written_by.iter().find(|(x, _, _)| x == w) {
                    panic!(
                        "Law 4: phases {} and {} both write {:?}",
                        first, phase.name, w
                    );
                }
                written_by.push((*w, at, phase.name));
            }
        }
        for (at, phase) in self.order.iter().enumerate() {
            for r in &phase.reads {
                // THE writer, because the loop above left at most one. Chasing every writer of a
                // kind instead would be a second answer to a question Law 4 has already closed.
                if let Some(&(_, wrote_at, wrote)) = written_by.iter().find(|(x, _, _)| x == r) {
                    assert!(
                        wrote_at < at,
                        "Money G2: phase {} needs {:?} of this week, which phase {} produces after it",
                        phase.name,
                        r,
                        wrote
                    );
                }
            }
        }
        self.sealed = true;
    }

    pub fn order(&self) -> &[PhaseDecl] {
        &self.order
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

/// Where a week is. There is nothing finer, so this is the whole clock.
pub struct Clock {
    pub calendar: Calendar,
    pub week: Week,
}

impl Clock {
    pub fn new(calendar: Calendar) -> Self {
        Self {
            calendar,
            week: Week(0),
        }
    }

    /// One week on.
    pub fn step(&mut self) {
        self.week = Week(self.week.0 + 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Week;

    fn decl(name: u32, at: u32, reads: Vec<Produces>, writes: Vec<Produces>) -> PhaseDecl {
        PhaseDecl {
            name,
            owner: 7,
            at,
            reads,
            writes,
        }
    }

    #[test]
    fn a_phase_runs_in_its_slot_and_siblings_keep_assembly_order() {
        let mut p = Phases::new();
        p.add(decl(100, D1, vec![], vec![]));
        p.add(decl(101, D1, vec![], vec![]));
        p.add(decl(102, G3, vec![], vec![]));
        let names: Vec<u32> = p.order().iter().map(|d| d.name).collect();
        // The two in d1 keep the order they were assembled in, both after the slot they name and
        // before the slot after it — so no phase can run in the slot ahead of its own.
        let at = |slot: u32| names.iter().position(|n| *n == slot).expect("a slot");
        assert_eq!(at(D1) + 1, at(100));
        assert_eq!(at(100) + 1, at(101));
        assert_eq!(at(101) + 1, at(D2));
        assert_eq!(at(G3) + 1, at(102));
        assert_eq!(at(102) + 1, at(G4));
    }

    #[test]
    fn the_thirty_one_slots_are_the_order_and_nothing_else_is() {
        let p = Phases::new();
        let names: Vec<u32> = p.order().iter().map(|d| d.name).collect();
        assert_eq!(
            names,
            SLOTS.iter().map(|(slot, _)| *slot).collect::<Vec<_>>()
        );
        // Every one of them is the kernel's own, so a module phase in the list is distinguishable
        // from the slot it sits in without asking anything else.
        assert!(p.order().iter().all(|d| d.owner == KERNEL));
    }

    #[test]
    #[should_panic(expected = "produces after it")]
    fn a_phase_may_not_read_what_a_later_phase_writes() {
        let mut p = Phases::new();
        p.add(decl(100, D1, vec![Produces(99)], vec![]));
        p.add(decl(101, G3, vec![], vec![Produces(99)]));
        p.seal();
    }

    #[test]
    #[should_panic(expected = "both write")]
    fn one_kind_has_one_writer() {
        // And this is why the read check may look up THE writer: a reader between two of them —
        // holding half a week and saying so nowhere — is refused before any read is considered.
        let mut p = Phases::new();
        p.add(decl(100, B2, vec![], vec![Produces(99)]));
        p.add(decl(101, D1, vec![Produces(99)], vec![]));
        p.add(decl(102, G3, vec![], vec![Produces(99)]));
        p.seal();
    }

    #[test]
    fn a_read_placed_after_its_writer_is_the_order_holding() {
        let mut p = Phases::new();
        p.add(decl(100, D1, vec![], vec![Produces(99)]));
        p.add(decl(101, G3, vec![Produces(99)], vec![]));
        p.seal();
        assert_eq!(p.len(), SLOTS.len() + 2);
    }

    #[test]
    #[should_panic(expected = "one of the thirty-one slots")]
    fn a_phase_cannot_run_outside_the_thirty_one() {
        let mut p = Phases::new();
        p.add(decl(100, 4242, vec![], vec![]));
    }

    #[test]
    #[should_panic(expected = "phases are declared at assembly")]
    fn nothing_is_added_once_the_world_has_begun() {
        let mut p = Phases::new();
        p.seal();
        p.add(decl(100, D1, vec![], vec![]));
    }

    #[test]
    fn a_week_is_the_whole_clock() {
        let mut c = Clock::new(Calendar::new());
        c.step();
        assert_eq!(c.week, Week(1));
        assert_eq!(c.calendar.at(c.week), Week(1));
    }
}
