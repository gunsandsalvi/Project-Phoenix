use std::collections::BTreeMap;

use phx_core::register::values::Partition;
use phx_core::{
    Directory, GroupDecl, KeyAttrDecl, KinkRegistry, PopEntry, PopItem, PositionDecl, PositionOf, ProfileComponent,
    RateDecl, RoleDecl, ScaleRef, Weight,
};
use phx_id::{Day, LineId, PartyId, RowRef, Slot, TableId};
use phx_ledger::algebra::Side;
use phx_ledger::apply::Ledger;
use phx_ledger::holder::HolderKeys;
use phx_ledger::instrument::Instruments;
use phx_ledger::line::{LineKindDecl, Lines, SideDecl};
use phx_ledger::part::DetachedRow;
use phx_ledger::rows::{BALANCE, Optional};
use phx_ledger::terms::TermsId;
use phx_num::round::Round;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_rand::Draws;
use phx_store::{AddressSpace, Backing};

use crate::check::LineKinks;
use crate::consts::{
    DESIGN_ENTRIES_PER_ROLE, DESIGN_PER_MEMBER, DESIGN_POSITIONS, DESIGN_ROW_BALANCE, DESIGN_ROW_MEMBERS, DESIGN_ROWS,
    DESIGN_TEN, DESIGN_THREE, DESIGN_WEIGHT,
};
use crate::key::{KeyInterner, KeyRecord};
use crate::kind::PopKindDecl;
use crate::landing::{LandingIndex, TenB, hold_key};
use crate::part::{Part, PartId};
use crate::profile::Profile;
use crate::split::{Cells, Parted, SplitSpec, split};
use crate::steps::StepTable;
use crate::table::{CellTable, NewCell};

const KIND: &str = "household";
const NAMES: [&str; DESIGN_POSITIONS] = [
    "p00", "p01", "p02", "p03", "p04", "p05", "p06", "p07", "p08", "p09", "p10", "p11", "p12", "p13", "p14", "p15",
    "p16", "p17", "p18", "p19", "p20", "p21", "p22", "p23", "p24", "p25", "p26", "p27", "p28", "p29",
];
const JOINT: &[ProfileComponent] = &[
    ProfileComponent { name: "a", values: DESIGN_TEN },
    ProfileComponent { name: "b", values: DESIGN_TEN },
    ProfileComponent { name: "c", values: DESIGN_THREE },
];
const LOAN: LineKindDecl = LineKindDecl {
    name: "loan",
    asset: SideDecl { holder_kinds: &["bank"], words: 0, holder_list: true },
    liability: SideDecl { holder_kinds: &[KIND], words: BALANCE, holder_list: true },
    transfer_requesters: &["BNK"],
    dated: false,
};

/// A landing index kept in ordered maps, for measuring a part's lookup away from the world's sharded index.
#[derive(Debug, Default)]
pub struct OrderedIndex {
    cells: BTreeMap<u64, BTreeMap<PartyId, Slot>>,
}

impl LandingIndex for OrderedIndex {
    fn candidates(&self, landing: u64) -> Vec<(PartyId, Slot)> {
        match self.cells.get(&landing) {
            Some(c) => c.iter().map(|(p, s)| (*p, *s)).collect(),
            None => Vec::new(),
        }
    }

    fn insert(&mut self, landing: u64, party: PartyId, slot: Slot) {
        self.cells.entry(landing).or_default().insert(party, slot);
    }

    fn remove(&mut self, landing: u64, party: PartyId) {
        if let Some(c) = self.cells.get_mut(&landing) {
            c.remove(&party);
        }
    }
}

/// Lines with no kinks between their members.
#[derive(Debug)]
pub struct NoKinks;

impl LineKinks for NoKinks {
    fn points(&self, _: LineId, _: Side) -> Vec<i64> {
        Vec::new()
    }
}

/// Two cells at the design point's sizes, of one key and step vector and holding the same rows, the first to split
/// parts from and the second to land them in: data for measuring a part end to end, not a world.
pub struct DesignPoint<B: Backing> {
    pub space: AddressSpace,
    pub ledger: Ledger<B>,
    pub table: CellTable<B>,
    pub keys: KeyInterner,
    pub directory: Directory,
    pub kind: PopKindDecl,
    pub index: OrderedIndex,
    pub origin: Slot,
    pub target: Slot,
}

impl<B: Backing> core::fmt::Debug for DesignPoint<B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DesignPoint")
            .field("origin", &self.origin)
            .field("target", &self.target)
            .finish_non_exhaustive()
    }
}

fn steps(_: &'static str) -> Result<StepTable, String> {
    StepTable::new(&Partition { exp: 0, bounds: [0, DESIGN_PER_MEMBER, DESIGN_PER_MEMBER * 2].into() })
}

fn kind() -> PopKindDecl {
    let e = |item| PopEntry { system: "REP", kind: KIND, item };
    let mut entries = vec![
        e(PopItem::Role(RoleDecl { name: "head", clause: "REP.26" })),
        e(PopItem::Role(RoleDecl { name: "partner", clause: "REP.26" })),
        e(PopItem::KeyAttr(KeyAttrDecl { name: "REP.composition", values: DESIGN_THREE, clause: "REP.19" })),
        e(PopItem::ProfileGroup(GroupDecl { name: "head_joint", role: "head", components: JOINT, clause: "REP.32" })),
        e(PopItem::ProfileGroup(GroupDecl {
            name: "partner_joint",
            role: "partner",
            components: JOINT,
            clause: "REP.32",
        })),
        e(PopItem::StandingRate(RateDecl { name: "rate", unit: "money/day", clause: "REP.20" })),
    ];
    for name in NAMES {
        entries.push(e(PopItem::Position(PositionDecl {
            name,
            unit: "money",
            of: PositionOf::Member,
            scale: ScaleRef::Rate("rate"),
            steps: "REP.steps",
            clause: "REP.20",
        })));
    }
    match PopKindDecl::compile(KIND, &entries, &KinkRegistry::default(), &steps) {
        Ok(k) => k,
        Err(e) => violation!(clause = "REP.20", "the design point's kind does not compile", errors = e.len()),
    }
}

/// A design-point cell: its members spread over seventy-five joint values in each role, a hundred a member in every
/// position, and a row on each line.
fn cell<B: Backing>(d: &mut DesignPoint<B>, key: KeyRecord, lines: &[LineId]) -> Slot {
    hold_key(&mut d.keys, key, 1);
    let Missing::Present(id) = d.keys.id(&key) else {
        violation!(clause = "REP.19", "a key held and not interned");
    };
    let layout = d.table.profile_layout().clone();
    let mut p = Profile::empty(&layout);
    for g in 0..layout.groups.len() {
        let mut left = DESIGN_WEIGHT;
        for v in 0..DESIGN_ENTRIES_PER_ROLE {
            let values_left = DESIGN_ENTRIES_PER_ROLE - v;
            let n = left / values_left;
            p.add(&layout, g, v * DESIGN_THREE, n);
            left -= n;
        }
    }
    let Some(total) = i64::from(DESIGN_WEIGHT).checked_mul(DESIGN_PER_MEMBER) else {
        capacity_exceeded!("a design-point total", i64::MAX, DESIGN_WEIGHT);
    };
    let positions = [total; DESIGN_POSITIONS];
    let party = PartyId::new(d.directory.next());
    let new = NewCell {
        party,
        created: Day::new(0),
        weight: Weight::new(DESIGN_WEIGHT),
        key: id,
        positions: &positions,
        profile: &p,
    };
    let levels = [0_u8; DESIGN_POSITIONS];
    let slot = d.table.add(&mut d.space, new, &d.kind, &levels);
    if d.directory.begin(RowRef { table: d.table.id(), slot }) != party {
        violation!(clause = "PTY.9", "a design-point cell given another identity than the directory's next");
    }
    d.table.set_rate(slot, 0, Missing::Present(1));
    for line in lines {
        let optional = Optional { balance: Missing::Present(DESIGN_ROW_BALANCE), ..Optional::NONE };
        let row = DetachedRow::opening(*line, Side::Liability, 0, DESIGN_ROW_MEMBERS, optional);
        let _ = d.ledger.attach_row(&mut d.table, 0, slot, row);
    }
    d.table.rekey(slot, &d.kind, &levels);
    d.index.insert(d.table.hot(slot).landing_key, party, slot);
    slot
}

/// The design point: two cells of two hundred members, each holding forty rows of twenty-four bytes, seventy-five
/// joint values per role and thirty positions.
#[must_use]
pub fn design_point<B: Backing>() -> DesignPoint<B> {
    let mut space = AddressSpace::empty();
    let keys = HolderKeys::new(1);
    // Columns chunk by powers of two, and the design point's stores hold its forty lines and a few cells.
    let rows = DESIGN_ROWS.next_power_of_two();
    let mut ledger = Ledger::new(
        Instruments::new(&mut space, rows, rows, rows, keys),
        Lines::new(&mut space, rows, rows, rows, keys),
    );
    let loan = ledger.lines.declare_money(LOAN).index();
    let lines: Vec<LineId> =
        (0..DESIGN_ROWS).map(|_| ledger.lines.open(loan, TermsId::new(0), Missing::Absent)).collect();
    let kind = kind();
    let table = CellTable::new(&mut space, &kind, TableId::new(1), rows, rows);
    let mut d = DesignPoint {
        space,
        ledger,
        table,
        keys: KeyInterner::new(),
        directory: Directory::new(),
        kind,
        index: OrderedIndex::default(),
        origin: Slot::new(0),
        target: Slot::new(0),
    };
    let key = KeyRecord::default();
    d.target = cell(&mut d, key, &lines);
    d.origin = cell(&mut d, key, &lines);
    d
}

impl<B: Backing> DesignPoint<B> {
    /// `count` members split from the first cell, drawn from `d`.
    pub fn part(&mut self, seq: u32, count: u32, d: &mut Draws) -> Part {
        let id = PartId { origin: self.table.party(self.origin), seq };
        let mut cells = Cells { ledger: &mut self.ledger, table: &mut self.table, place: 0, keys: &self.keys };
        let spec =
            SplitSpec { count, given: &[], rows: &[], own: &[], reviewed: Missing::Absent, rounding: Round::HalfEven };
        match split(&mut cells, self.origin, id, &spec, d) {
            Parted::Part(p) => *p,
            Parted::Whole => violation!(clause = "REP.8", "a design-point part of every member"),
        }
    }

    /// What 10b works on, over the design point's stores.
    pub fn tenb<'a>(&'a mut self, kinks: &'a NoKinks, levels: &'a [u8]) -> TenB<'a, B, B> {
        TenB {
            ledger: &mut self.ledger,
            table: &mut self.table,
            place: 0,
            keys: &mut self.keys,
            directory: &mut self.directory,
            space: &mut self.space,
            kind: &self.kind,
            levels,
            kinks,
            today: Day::new(1),
        }
    }
}

/// The levels of the design point's positions: every position at its base partition.
#[must_use]
pub fn levels() -> [u8; DESIGN_POSITIONS] {
    [0; DESIGN_POSITIONS]
}

#[cfg(test)]
mod tests {
    use phx_store::HeapBacking;

    use super::{NoKinks, design_point, levels};
    use crate::fixture::draws;
    use crate::landing::land;
    use crate::part::Part;

    #[test]
    fn a_design_point_part_lands_in_its_twin() {
        let mut dp = design_point::<HeapBacking<4096>>();
        assert_eq!(dp.table.profile(dp.origin).entries(), 150);
        let p: Part = dp.part(0, 10, &mut draws("REP.bench", 0));
        assert_eq!(p.positions.len(), 30);
        assert!(p.rows.len() > 30, "most of forty rows reach ten of two hundred members: {}", p.rows.len());
        let kinks = NoKinks;
        let lv = levels();
        let mut index = std::mem::take(&mut dp.index);
        let landed = land(&mut dp.tenb(&kinks, &lv), &mut index, vec![p]);
        assert_eq!((landed.landings, landed.new_cells, landed.holder_list_changes), (1, 0, 0));
        assert!(landed.moved.iter().all(|m| *m == 0));
    }
}
