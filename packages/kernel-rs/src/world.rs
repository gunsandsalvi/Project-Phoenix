//! The week loop: NINE STAGES in the order Money G2 fixes, held as DATA and run one at a time.

use crate::calendar::{Calendar, Week};

/// What one phase hands another inside a week: an event of a named journal kind.
///
/// A print is not one. Every print is written at BOOKS by the one solver, and every phase that
/// reads one runs in a later stage by Money G2's own order, so a print needs no declaration to be
/// ordered — and a variant nothing constructs is a shape with no producer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Produces(pub u32);

pub struct PhaseDecl {
    pub name: u32,
    pub owner: u32,
    /// Which of the nine stages it runs in.
    pub at: u32,
    /// What it needs of THIS week. A phase that wants an earlier week's rows is ordered by the
    /// calendar and declares nothing: running before the writer costs it a week, not an answer.
    pub reads: Vec<Produces>,
    /// What its own module says into the journal. A kernel door the phase opens — settlement, a
    /// cessation — writes the kernel's record and not this phase's.
    pub writes: Vec<Produces>,
}

/// THE NINE STAGES OF A PERIOD (Money G2), in the order they run and no other. A week settles
/// ONCE, so this is the whole of the structure time has here.
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

pub const STAGES: [u32; 9] = [
    OPENS, OWED, POPULATION, WORK, VIEWS, BOOKS, JUDGED, SCHEDULED, CLOSES,
];

/// G2.e is two things: a party forms its view, and THEN it posts what it wants. This is the second,
/// and it is the kernel's, because when a party may post is not a thing any system chooses. It runs
/// in VIEWS after every module phase in it, and the books it posted into clear a stage later.
pub const POSTS: u32 = 9;

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
                .map(|s| PhaseDecl {
                    name: *s,
                    owner: KERNEL,
                    at: *s,
                    reads: vec![],
                    writes: vec![],
                })
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
            vec![
                OPENS, OWED, POPULATION, WORK, 10, 11, VIEWS, BOOKS, JUDGED, 12, SCHEDULED, CLOSES
            ]
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
    #[should_panic(expected = "produces after it")]
    fn a_phase_may_not_read_what_a_later_phase_writes() {
        let mut p = Phases::new();
        p.add(decl(10, WORK, vec![Produces(99)], vec![]));
        p.add(decl(11, JUDGED, vec![], vec![Produces(99)]));
        p.seal();
    }

    #[test]
    #[should_panic(expected = "both write")]
    fn one_kind_has_one_writer() {
        // And this is why the read check may look up THE writer: a reader between two of them —
        // holding half a week and saying so nowhere — is refused before any read is considered.
        let mut p = Phases::new();
        p.add(decl(10, OWED, vec![], vec![Produces(99)]));
        p.add(decl(11, WORK, vec![Produces(99)], vec![]));
        p.add(decl(12, JUDGED, vec![], vec![Produces(99)]));
        p.seal();
    }

    #[test]
    fn a_read_placed_after_its_writer_is_the_order_holding() {
        let mut p = Phases::new();
        p.add(decl(10, WORK, vec![], vec![Produces(99)]));
        p.add(decl(11, JUDGED, vec![Produces(99)], vec![]));
        p.seal();
        assert_eq!(p.len(), 11);
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
    fn a_week_is_the_whole_clock() {
        let mut c = Clock::new(Calendar::new());
        c.step();
        assert_eq!(c.week, Week(1));
        assert_eq!(c.calendar.at(c.week), Week(1));
    }
}
