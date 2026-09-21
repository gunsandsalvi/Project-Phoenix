//! The period loop: NINE STAGES in the order Money G2 fixes, held as DATA and run one at a time.

use crate::calendar::{Calendar, Week};

/// What a phase needs of THIS period, and what it puts into it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Produces {
    /// A print in a named book.
    Print(u32),
    /// An event of a named kind.
    Event(u32),
}

pub struct PhaseDecl {
    pub name: u32,
    pub owner: u32,
    /// Which of the nine stages it runs in.
    pub at: u32,
    pub reads: Vec<Produces>,
    pub writes: Vec<Produces>,
}

/// THE NINE STAGES OF A PERIOD (Money G2), in the order they run and no other. A period settles
/// ONCE, so this is the whole of the structure time has here.
/// The period opens: what an earlier period scheduled for this one arrives.
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
/// What the judgement implies is scheduled, for the period AFTER.
pub const SCHEDULED: u32 = 7;
/// The period closes: the gridlock pass, then the audit.
pub const CLOSES: u32 = 8;

pub const STAGES: [u32; 9] =
    [OPENS, OWED, POPULATION, WORK, VIEWS, BOOKS, JUDGED, SCHEDULED, CLOSES];

/// Whose a stage marker is, so the one pass can tell a stage from a module's phase in it.
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
            order: STAGES
                .iter()
                .map(|s| PhaseDecl { name: *s, owner: KERNEL, at: *s, reads: vec![], writes: vec![] })
                .collect(),
            sealed: false,
        }
    }

    /// A module's phase is INSERTED into its stage, after whatever is already in it.
    pub fn add(&mut self, decl: PhaseDecl) {
        assert!(!self.sealed, "Law 10: phases are declared at assembly");
        assert!(
            !self.order.iter().any(|p| p.name == decl.name),
            "Law 4: a phase is declared twice"
        );
        let stage = self
            .order
            .iter()
            .position(|p| p.name == decl.at && p.owner == KERNEL)
            .expect("Money G2: a phase runs in one of the nine stages");
        // After the stage marker and after its siblings, so a stage runs in assembly order and a
        // phase cannot land in the stage before it.
        let mut insert = stage + 1;
        while insert < self.order.len() && self.order[insert].at == decl.at {
            insert += 1;
        }
        self.order.insert(insert, decl);
    }

    /// Once sealed, the order is what a period runs and no phase may be added.
    pub fn seal(&mut self) {
        let mut written_by: Vec<(Produces, usize)> = Vec::new();
        for (at, phase) in self.order.iter().enumerate() {
            for w in &phase.writes {
                written_by.push((*w, at));
            }
        }
        for (at, phase) in self.order.iter().enumerate() {
            for r in &phase.reads {
                if let Some(&(_, wrote_at)) = written_by.iter().find(|(w, _)| w == r) {
                    assert!(
                        wrote_at < at,
                        "Law 10: phase {} reads {:?}, which is produced after it",
                        phase.name,
                        r
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

/// Where a period is. There is nothing finer, so this is the whole clock.
pub struct Clock {
    pub calendar: Calendar,
    pub period: Week,
}

impl Clock {
    pub fn new(calendar: Calendar) -> Self {
        Self { calendar, period: Week(0) }
    }

    /// One period on.
    pub fn step(&mut self) {
        self.period = self.calendar.next(self.period);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Week;

    fn decl(name: u32, at: u32, reads: Vec<Produces>, writes: Vec<Produces>) -> PhaseDecl {
        PhaseDecl { name, owner: 7, at, reads, writes }
    }

    #[test]
    fn a_phase_runs_in_its_stage_and_siblings_keep_assembly_order() {
        let mut p = Phases::new();
        p.add(decl(10, WORK, vec![], vec![]));
        p.add(decl(11, WORK, vec![], vec![]));
        p.add(decl(12, JUDGED, vec![], vec![]));
        let names: Vec<u32> = p.order().iter().map(|d| d.name).collect();
        // The two in WORK keep the order they were assembled in, both after the stage they name and
        // before the stage after it — so no phase can run in the stage ahead of its own.
        assert_eq!(
            names,
            vec![OPENS, OWED, POPULATION, WORK, 10, 11, VIEWS, BOOKS, JUDGED, 12, SCHEDULED, CLOSES]
        );
    }

    #[test]
    fn the_nine_stages_are_the_order_and_nothing_else_is() {
        let p = Phases::new();
        let names: Vec<u32> = p.order().iter().map(|d| d.name).collect();
        assert_eq!(names, STAGES.to_vec());
        // Every one of them is the kernel's own, so a module phase in the list is distinguishable
        // from the stage it sits in without asking anything else.
        assert!(p.order().iter().all(|d| d.owner == KERNEL));
    }

    #[test]
    #[should_panic(expected = "is produced after it")]
    fn a_phase_may_not_read_what_a_later_phase_writes() {
        let mut p = Phases::new();
        p.add(decl(10, WORK, vec![Produces::Event(99)], vec![]));
        p.add(decl(11, JUDGED, vec![], vec![Produces::Event(99)]));
        p.seal();
    }

    #[test]
    #[should_panic(expected = "one of the nine stages")]
    fn a_phase_cannot_run_outside_the_nine() {
        let mut p = Phases::new();
        p.add(decl(10, 4242, vec![], vec![]));
    }

    #[test]
    #[should_panic(expected = "phases are declared at assembly")]
    fn nothing_is_added_once_the_world_has_begun() {
        let mut p = Phases::new();
        p.seal();
        p.add(decl(10, WORK, vec![], vec![]));
    }

    #[test]
    fn a_period_is_the_whole_clock() {
        let mut c = Clock::new(Calendar::new());
        c.step();
        assert_eq!(c.period, Week(1));
        assert_eq!(c.period, Week(1));
    }
}
