//! The agencies on their kind's store: each agency's staffing record, the staff it keeps by region and occupation
//! (region-major) and its appropriation for wages a month. Its site is on its kind's place store.

use phx_id::{PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_pop::consts::{AGENCY_OCCUPATIONS, AGENCY_REGIONS};
use phx_pop::directory::Directory;
use phx_pop::kinds::{AttrW, KindStore, Row};
use phx_pop::layout::{AGENCY, Layout};
use phx_store::{AddressSpace, SystemBacking};

/// The group a staffing review gathers.
const STAFFING: u8 = 0;

/// The handles of an agency's staffing record.
#[derive(Clone, Debug)]
struct Words {
    targets: Vec<AttrW<u32>>,
    budget: AttrW<i64>,
}

/// The agencies' store: their rows by slot, and their words' handles.
#[clause("SOC.2", "SOC.8")]
#[derive(Debug, phx_macros::Saved)]
pub struct AgencyStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    #[saved(skip, rebuild = AgencyStore::bind)]
    words: Option<Words>,
}

/// The agency map's layout and its words' handles.
fn compiled() -> (Layout, Words) {
    let Ok(mut layout) = Layout::compile(&AGENCY, &[]) else {
        violation!(clause = "REP.1", "the agency's layout refused");
    };
    let cells = AGENCY_REGIONS * AGENCY_OCCUPATIONS;
    let targets = (0..cells)
        .map(|i| match layout.writer("targets", i, "SOC") {
            Ok(a) => a,
            Err(_) => violation!(clause = "REP.1", "an agency's staffing word refused"),
        })
        .collect();
    let Ok(budget) = layout.writer("budget", 0, "SOC") else {
        violation!(clause = "REP.1", "an agency's budget word refused");
    };
    (layout, Words { targets, budget })
}

/// A (region, occupation)'s cell among an agency's targets; one past the record's stops the run.
#[must_use]
pub fn cell(region: u32, occupation: u32) -> usize {
    let (regions, occupations) = (u32::from(AGENCY_REGIONS), u32::from(AGENCY_OCCUPATIONS));
    if region >= regions {
        capacity_exceeded!("regions an agency's staffing holds", regions, region);
    }
    if occupation >= occupations {
        capacity_exceeded!("occupations an agency's staffing holds", occupations, occupation);
    }
    usize::try_from(region * occupations + occupation).unwrap_or(usize::MAX)
}

impl AgencyStore {
    /// The agencies' store: room for `capacity` agencies of kind `kind`.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, capacity: u32) -> AgencyStore {
        let (layout, words) = compiled();
        AgencyStore { store: KindStore::new(space, kind, &layout, capacity), kind, words: Some(words) }
    }

    /// A loaded store's handles, compiled again from the agency's layout.
    fn bind(&mut self) -> u64 {
        self.words = Some(compiled().1);
        0
    }

    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// An agency the directory began, keeping no staff yet.
    pub fn begin(&mut self, dir: &Directory<SystemBacking>, r: PartyRef) {
        self.store.begin(dir, r, &[]);
    }

    fn w(&self) -> &Words {
        match &self.words {
            Some(w) => w,
            None => violation!(clause = "REP.1", "an agency store read before its handles are bound"),
        }
    }

    fn target_word(&self, region: u32, occupation: u32) -> AttrW<u32> {
        match self.w().targets.get(cell(region, occupation)) {
            Some(a) => *a,
            None => violation!(clause = "SOC.8", "a staffing cell past the agency's words"),
        }
    }

    /// An agency's staffing record, its row gathered once; none for a slot no agency was begun at.
    #[must_use]
    pub fn staffing(&self, slot: Slot) -> Option<Staffing<'_>> {
        Some(Staffing { row: self.store.gather_at(slot, STAFFING)?, w: self.w() })
    }

    /// One more post an agency keeps in a region and occupation, and its wage a month in its appropriation.
    pub fn keep_post(&mut self, slot: Slot, (region, occupation): (u32, u32), wage: i64) {
        let a = self.target_word(region, occupation);
        let held = self.staffing(slot).map(|s| s.row.get(a.read()));
        let n = match held {
            Some(Missing::Present(n)) => n + 1,
            Some(Missing::Absent) => 1,
            None => violation!(clause = "SOC.8", "a post kept for no agency", slot = slot.get()),
        };
        self.store.set_at(slot, a, Missing::Present(n));
        let b = self.w().budget;
        let Some(Missing::Present(budget)) = self.staffing(slot).map(|s| s.row.get(b.read())) else {
            violation!(clause = "SOC.8", "an agency's budget read for no agency", slot = slot.get());
        };
        let Some(budget) = budget.checked_add(wage) else {
            capacity_exceeded!("an agency's appropriation", i64::MAX, wage);
        };
        self.store.set_at(slot, b, Missing::Present(budget));
    }
}

/// An agency's staffing record as gathered.
#[derive(Clone, Copy, Debug)]
pub struct Staffing<'a> {
    row: Row<'a>,
    w: &'a Words,
}

impl Staffing<'_> {
    /// The staff it keeps in a region and occupation; none where it keeps none there.
    pub fn target(&self, region: u32, occupation: u32) -> Missing<u32> {
        match self.w.targets.get(cell(region, occupation)) {
            Some(a) => self.row.get(a.read()),
            None => Missing::Absent,
        }
    }

    /// Its appropriation for wages a month.
    #[must_use]
    pub fn budget(&self) -> i64 {
        match self.row.get(self.w.budget.read()) {
            Missing::Present(b) => b,
            Missing::Absent => violation!(clause = "SOC.8", "an agency's budget never absent read absent"),
        }
    }

    /// Every region and occupation it keeps staff in, with the staff, region-major.
    pub fn targets(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        let occupations = u32::from(AGENCY_OCCUPATIONS);
        (0_u32..).zip(&self.w.targets).filter_map(move |(i, a)| match self.row.get(a.read()) {
            Missing::Present(n) => Some((i / occupations, i % occupations, n)),
            Missing::Absent => None,
        })
    }
}

#[cfg(test)]
#[path = "agency_store_tests.rs"]
mod tests;
