//! The period loop: ordered phases held as DATA, run in order, one at a time.

use crate::calendar::{Calendar, Cycle, Period};

/// What a phase needs of THIS period, and what it puts into it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Produces {
    /// A print in a named book.
    Print(u32),
    /// An event of a named kind.
    Event(u32),
}

/// Where a phase runs, against a kernel phase or another module's.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Anchor {
    Before(u32),
    After(u32),
}

pub struct PhaseDecl {
    pub name: u32,
    pub owner: u32,
    pub anchor: Anchor,
    pub reads: Vec<Produces>,
    pub writes: Vec<Produces>,
}

/// The three moments the whole world turns on.
pub const CORPORATE_ACTIONS: u32 = 0;
pub const MARKETS: u32 = 1;
pub const REVALUATION: u32 = 2;

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
        let kernel = u32::MAX;
        Self {
            order: vec![
                PhaseDecl { name: CORPORATE_ACTIONS, owner: kernel, anchor: Anchor::After(CORPORATE_ACTIONS), reads: vec![], writes: vec![] },
                PhaseDecl { name: MARKETS, owner: kernel, anchor: Anchor::After(MARKETS), reads: vec![], writes: vec![] },
                PhaseDecl { name: REVALUATION, owner: kernel, anchor: Anchor::After(REVALUATION), reads: vec![], writes: vec![] },
            ],
            sealed: false,
        }
    }

    /// A module's phase is INSERTED at the position its anchor puts it.
    pub fn add(&mut self, decl: PhaseDecl) {
        assert!(!self.sealed, "Law 10: phases are declared at assembly");
        assert!(
            !self.order.iter().any(|p| p.name == decl.name),
            "Law 4: a phase is declared twice"
        );
        let anchor = match decl.anchor {
            Anchor::Before(a) | Anchor::After(a) => a,
        };
        let at = self
            .order
            .iter()
            .position(|p| p.name == anchor)
            .expect("Law 10: a phase anchors to one that does not exist");
        let insert = match decl.anchor {
            Anchor::Before(_) => at,
            Anchor::After(_) => {
                let mut n = at + 1;
                while n < self.order.len() {
                    let anchored_here = matches!(
                        self.order[n].anchor,
                        Anchor::After(a) if a == anchor
                    );
                    if !anchored_here {
                        break;
                    }
                    n += 1;
                }
                n
            }
        };
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

/// Where a period is.
pub struct Clock {
    pub calendar: Calendar,
    pub period: Period,
    pub cycle: Cycle,
}

impl Clock {
    pub fn new(calendar: Calendar) -> Self {
        Self { calendar, period: Period(0), cycle: Cycle(0) }
    }

    /// One period on.
    pub fn step(&mut self) {
        self.period = Period(self.period.0 + 1);
        self.cycle = Cycle(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calendar::Day;

    fn decl(name: u32, anchor: Anchor, reads: Vec<Produces>, writes: Vec<Produces>) -> PhaseDecl {
        PhaseDecl { name, owner: 7, anchor, reads, writes }
    }

    #[test]
    fn a_phase_runs_where_its_anchor_puts_it_and_siblings_keep_assembly_order() {
        let mut p = Phases::new();
        p.add(decl(10, Anchor::After(MARKETS), vec![], vec![]));
        p.add(decl(11, Anchor::After(MARKETS), vec![], vec![]));
        p.add(decl(12, Anchor::Before(REVALUATION), vec![], vec![]));
        let names: Vec<u32> = p.order().iter().map(|d| d.name).collect();
        // The two anchored AFTER markets keep the order they were assembled in; the one anchored
        // BEFORE revaluation lands just ahead of it.
        assert_eq!(names, vec![CORPORATE_ACTIONS, MARKETS, 10, 11, 12, REVALUATION]);
    }

    #[test]
    #[should_panic(expected = "is produced after it")]
    fn a_phase_may_not_read_what_a_later_phase_writes() {
        let mut p = Phases::new();
        p.add(decl(10, Anchor::After(CORPORATE_ACTIONS), vec![Produces::Event(99)], vec![]));
        p.add(decl(11, Anchor::After(MARKETS), vec![], vec![Produces::Event(99)]));
        p.seal();
    }

    #[test]
    #[should_panic(expected = "anchors to one that does not exist")]
    fn a_phase_cannot_anchor_to_nothing() {
        let mut p = Phases::new();
        p.add(decl(10, Anchor::After(4242), vec![], vec![]));
    }

    #[test]
    #[should_panic(expected = "phases are declared at assembly")]
    fn nothing_is_added_once_the_world_has_begun() {
        let mut p = Phases::new();
        p.seal();
        p.add(decl(10, Anchor::After(MARKETS), vec![], vec![]));
    }

    #[test]
    fn a_cycle_is_within_a_period_and_nothing_finer_exists() {
        let mut c = Clock::new(Calendar::new(Day(0), 7, 3));
        c.cycle = Cycle(2);
        c.step();
        assert_eq!(c.period, Period(1));
        assert_eq!(c.cycle, Cycle(0));
        assert_eq!(c.calendar.start_of(c.period), Day(7));
    }
}
