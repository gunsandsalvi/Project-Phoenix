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
use phx_rand::{Draws, below_u64};
use phx_store::{AddressSpace, Backing};

use crate::check::LineKinks;
use crate::consts::{
    DESIGN_ENTRIES_PER_ROLE, DESIGN_PER_MEMBER, DESIGN_POSITIONS, DESIGN_ROW_BALANCE, DESIGN_ROW_MEMBERS, DESIGN_ROWS,
    DESIGN_TEN, DESIGN_THREE, DESIGN_WEIGHT, POPULATION_CHUNK, POPULATION_DRAWN,
};
use crate::index::Index;
use crate::key::{KeyInterner, KeyRecord};
use crate::kind::PopKindDecl;
use crate::landing::{LandingIndex, TenB, hold_key};
use crate::part::{Part, PartId};
use crate::profile::Profile;
use crate::split::{Cells, Parted, SplitSpec, split, split_batch};
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
    asset: SideDecl {
        holder_kinds: &["bank"],
        words: 0,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &[KIND],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    transfer_requesters: &["BNK"],
    dated: false,
};

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
    pub index: Index,
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
        e(PopItem::Role(RoleDecl { name: "head", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
        e(PopItem::Role(RoleDecl { name: "partner", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
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
fn cell<B: Backing>(
    d: &mut DesignPoint<B>,
    key: KeyRecord,
    lines: &[LineId],
    per_member: &[i64; DESIGN_POSITIONS],
) -> Slot {
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
    let mut positions = [0_i64; DESIGN_POSITIONS];
    for (total, each) in positions.iter_mut().zip(per_member) {
        let Some(t) = i64::from(DESIGN_WEIGHT).checked_mul(*each) else {
            capacity_exceeded!("a design-point total", i64::MAX, DESIGN_WEIGHT);
        };
        *total = t;
    }
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
        index: Index::new(),
        origin: Slot::new(0),
        target: Slot::new(0),
    };
    let key = KeyRecord::default();
    d.target = cell(&mut d, key, &lines, &[DESIGN_PER_MEMBER; DESIGN_POSITIONS]);
    d.origin = cell(&mut d, key, &lines, &[DESIGN_PER_MEMBER; DESIGN_POSITIONS]);
    d
}

/// A population of `cells` design-point cells over a few rows each, for measuring what 10b's representation work
/// costs per cell: each cell's composition and first positions drawn, so some share landing keys and most do not, and
/// the cells added in no order of key.
#[must_use]
pub fn population<B: Backing>(cells: u32, d: &mut Draws) -> DesignPoint<B> {
    let mut space = AddressSpace::empty();
    let keys = HolderKeys::new(1);
    let rows = cells.next_power_of_two();
    let mut ledger = Ledger::new(
        Instruments::new(&mut space, rows, rows, rows, keys),
        Lines::new(&mut space, rows, rows, rows, keys),
    );
    let loan = ledger.lines.declare_money(LOAN).index();
    let lines: Vec<LineId> =
        (0..DESIGN_THREE).map(|_| ledger.lines.open(loan, TermsId::new(0), Missing::Absent)).collect();
    let kind = kind();
    let table = CellTable::new(&mut space, &kind, TableId::new(1), rows, POPULATION_CHUNK);
    let mut p = DesignPoint {
        space,
        ledger,
        table,
        keys: KeyInterner::new(),
        directory: Directory::new(),
        kind,
        index: Index::new(),
        origin: Slot::new(0),
        target: Slot::new(0),
    };
    for _ in 0..cells {
        let mut key = KeyRecord::default();
        let Ok(composition) = u32::try_from(below_u64(d, u64::from(DESIGN_THREE))) else {
            violation!(clause = "REP.19", "a composition past its values");
        };
        p.kind.key.set(&mut key, 0, composition);
        // The first positions drawn over their three steps, the rest alike: some cells share a landing key, most do
        // not, and widening one position unites some.
        let mut per_member = [DESIGN_PER_MEMBER; DESIGN_POSITIONS];
        for each in per_member.iter_mut().take(POPULATION_DRAWN) {
            *each = i64::try_from(below_u64(d, u64::from(DESIGN_THREE))).unwrap_or(0) * DESIGN_PER_MEMBER;
        }
        let _ = cell(&mut p, key, &lines, &per_member);
    }
    p
}

/// A population at the finished world's size, for the full-load bench: `cells` design-point cells, each holding
/// `rows_per_cell` rows on lines drawn from a pool of `lines`, with the design point's profile entries and positions;
/// data for measuring what the finished world's stores cost and what its kernels take over them, not a world.
#[must_use]
pub fn population_at<B: Backing>(cells: u32, rows_per_cell: u32, lines: u32, d: &mut Draws) -> DesignPoint<B> {
    let mut space = AddressSpace::empty();
    let keys = HolderKeys::new(1);
    let Some(rows) = cells.checked_mul(rows_per_cell) else {
        capacity_exceeded!("full-load rows", u32::MAX, u64::from(cells) * u64::from(rows_per_cell));
    };
    let half = u32::try_from(phx_store::consts::BLOCK_HALF).unwrap_or(u32::MAX);
    let blocks = lines + rows.div_ceil(half);
    let mut ledger = Ledger::new(
        Instruments::new(&mut space, 1, 1, 1, keys),
        Lines::new(&mut space, lines.next_power_of_two(), POPULATION_CHUNK, blocks, keys),
    );
    let loan = ledger.lines.declare_money(LOAN).index();
    let pool: Vec<LineId> = (0..lines).map(|_| ledger.lines.open(loan, TermsId::new(0), Missing::Absent)).collect();
    let kind = kind();
    let table = CellTable::new(&mut space, &kind, TableId::new(1), cells.next_power_of_two(), POPULATION_CHUNK);
    let mut p = DesignPoint {
        space,
        ledger,
        table,
        keys: KeyInterner::new(),
        directory: Directory::new(),
        kind,
        index: Index::new(),
        origin: Slot::new(0),
        target: Slot::new(0),
    };
    let take = usize::try_from(rows_per_cell).unwrap_or(usize::MAX);
    for _ in 0..cells {
        let mut key = KeyRecord::default();
        let Ok(composition) = u32::try_from(below_u64(d, u64::from(DESIGN_THREE))) else {
            violation!(clause = "REP.19", "a composition past its values");
        };
        p.kind.key.set(&mut key, 0, composition);
        let mut per_member = [DESIGN_PER_MEMBER; DESIGN_POSITIONS];
        for each in per_member.iter_mut().take(POPULATION_DRAWN) {
            *each = i64::try_from(below_u64(d, u64::from(DESIGN_THREE))).unwrap_or(0) * DESIGN_PER_MEMBER;
        }
        // A cell's rows on consecutive lines of the pool from a drawn start, so no two of its rows share a line.
        let start = usize::try_from(below_u64(d, u64::from(lines))).unwrap_or(0);
        let mine: Vec<LineId> = pool.iter().cycle().skip(start).take(take).copied().collect();
        let _ = cell(&mut p, key, &mine, &per_member);
    }
    p.origin = Slot::new(0);
    p.target = Slot::new(0);
    p
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

    /// Parts of the members given split from the first cell together, each drawn from its own stream.
    pub fn parts(&mut self, members: &[u32], streams: &mut [Draws]) -> Vec<Part> {
        let origin = self.table.party(self.origin);
        let mut batch: Vec<(PartId, SplitSpec<'_>, &mut Draws)> = members
            .iter()
            .zip(streams.iter_mut())
            .zip(0_u32..)
            .map(|((k, d), seq)| {
                let spec = SplitSpec {
                    count: *k,
                    given: &[],
                    rows: &[],
                    own: &[],
                    reviewed: Missing::Absent,
                    rounding: Round::HalfEven,
                };
                (PartId { origin, seq }, spec, d)
            })
            .collect();
        let mut cells = Cells { ledger: &mut self.ledger, table: &mut self.table, place: 0, keys: &self.keys };
        split_batch(&mut cells, self.origin, &mut batch)
            .into_iter()
            .map(|p| match p {
                Parted::Part(p) => *p,
                Parted::Whole => violation!(clause = "REP.8", "a design-point part of every member"),
            })
            .collect()
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

    #[test]
    fn a_batch_joins_as_its_parts_one_by_one() {
        use phx_ledger::rows::rows;

        use crate::join::{Joined, Landing, join, join_batch};

        let parts = |dp: &mut super::DesignPoint<HeapBacking<4096>>| -> Vec<Part> {
            (0..6_u32).map(|i| dp.part(i, 1 + i * 7, &mut draws("REP.bench", i))).collect()
        };
        let (mut a, mut b) = (design_point::<HeapBacking<4096>>(), design_point::<HeapBacking<4096>>());
        let (pa, pb) = (parts(&mut a), parts(&mut b));
        assert_eq!(pa, pb, "the same draws make the same parts");
        let batch = join_batch(&mut a.ledger, &mut a.table, 0, a.target, pa);
        let mut one_by_one = Joined { erased: vec![0.0; b.table.positions()], ..Joined::default() };
        for p in pb {
            let d = join(&mut b.ledger, &mut b.table, 0, &Landing { part: p.id, target: b.target }, p);
            one_by_one.rows += d.rows;
            one_by_one.holder_list_changes += d.holder_list_changes;
            for (t, e) in one_by_one.erased.iter_mut().zip(d.erased) {
                *t += e;
            }
        }
        assert_eq!(batch, one_by_one);
        let (ta, tb) = (a.target, b.target);
        assert_eq!(a.table.weight(ta), b.table.weight(tb));
        assert_eq!(a.table.profile(ta), b.table.profile(tb));
        for i in 0..a.table.positions() {
            assert_eq!(a.table.position(ta, i), b.table.position(tb, i));
        }
        let view = |t: &crate::table::CellTable<HeapBacking<4096>>, s| {
            rows(t, s).iter().map(|v| (v.row, v.optional)).collect::<Vec<_>>()
        };
        assert_eq!(view(&a.table, ta), view(&b.table, tb), "every row's members and words");
    }

    #[test]
    fn a_batch_splits_as_its_splits_one_by_one() {
        // Small parts, and large ones that take some rows whole.
        for counts in [[1_u32, 9, 3, 40, 1], [150, 1, 45, 2, 1]] {
            batch_equals_one_by_one(&counts);
        }
    }

    fn batch_equals_one_by_one(counts: &[u32; 5]) {
        use phx_ledger::rows::rows;
        use phx_num::Missing;
        use phx_num::round::Round;

        use crate::part::PartId;
        use crate::split::{Cells, Parted, SplitSpec, split, split_batch};

        let spec = |count| SplitSpec {
            count,
            given: &[],
            rows: &[],
            own: &[],
            reviewed: Missing::Absent,
            rounding: Round::HalfEven,
        };
        let (mut a, mut b) = (design_point::<HeapBacking<4096>>(), design_point::<HeapBacking<4096>>());
        let id = |dp: &super::DesignPoint<HeapBacking<4096>>, seq| PartId { origin: dp.table.party(dp.origin), seq };
        let mut streams: Vec<phx_rand::Draws> = (0..5).map(|i| draws("DEM.death", i)).collect();
        let mut batch: Vec<(PartId, SplitSpec<'_>, &mut phx_rand::Draws)> = streams
            .iter_mut()
            .zip(counts)
            .enumerate()
            .map(|(i, (d, k))| (id(&a, u32::try_from(i).unwrap()), spec(*k), d))
            .collect();
        let origin = a.origin;
        let got = split_batch(
            &mut Cells { ledger: &mut a.ledger, table: &mut a.table, place: 0, keys: &a.keys },
            origin,
            &mut batch,
        );
        let origin_b = b.origin;
        let want: Vec<Parted> = counts
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let seq = u32::try_from(i).unwrap();
                let pid = id(&b, seq);
                split(
                    &mut Cells { ledger: &mut b.ledger, table: &mut b.table, place: 0, keys: &b.keys },
                    origin_b,
                    pid,
                    &spec(*k),
                    &mut draws("DEM.death", seq),
                )
            })
            .collect();
        assert_eq!(got, want, "the same parts");
        assert_eq!(a.table.weight(origin), b.table.weight(origin_b));
        assert_eq!(a.table.profile(origin), b.table.profile(origin_b));
        let view = |t: &crate::table::CellTable<HeapBacking<4096>>, s| {
            rows(t, s).iter().map(|v| (v.row, v.optional)).collect::<Vec<_>>()
        };
        assert_eq!(view(&a.table, origin), view(&b.table, origin_b));
    }
}
