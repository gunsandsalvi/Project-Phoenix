#![cfg(test)]
//! One cell of a household kind with a deposit, a loan, a fund holding and a review exposure, for tests of what
//! splits and landings do to a cell.

use phx_core::register::values::Partition;
use phx_core::{
    GroupDecl, KeyAttrDecl, KinkRegistry, PopEntry, PopItem, PositionDecl, PositionOf, ProfileComponent, RateDecl,
    RoleDecl, ScaleRef, Weight,
};
use phx_exec::KeyedReduce;
use phx_id::{Day, InstrumentId, LineId, PartyId, Slot, TableId};
use phx_ledger::algebra::Side;
use phx_ledger::apply::Ledger;
use phx_ledger::holder::HolderKeys;
use phx_ledger::holding::CellHolding;
use phx_ledger::instrument::{InstrumentFamily, Instruments, NewInstrument};
use phx_ledger::line::{LineKindDecl, Lines, SideDecl};
use phx_ledger::part::DetachedRow;
use phx_ledger::rows::{BALANCE, Optional, RelRow, role};
use phx_ledger::terms::TermsId;
use phx_num::{Amount, Ccy, Missing, QtyRaw, UnitId};
use phx_rand::{Draws, Seed, Subject, SubjectTag, stream_key};
use phx_store::{AddressSpace, HeapBacking};

use crate::key::{KeyInterner, KeyRecord};
use crate::kind::PopKindDecl;
use crate::profile::Profile;
use crate::split::Cells;
use crate::steps::StepTable;
use crate::table::{CellTable, NewCell};

pub(crate) const PLACE: u16 = 0;
pub(crate) const HH: &str = "household";
/// The groups: the head's age, the head's age and health joint, the partner's age.
pub(crate) const AGE: usize = 0;
pub(crate) const AGE_HEALTH: usize = 1;
pub(crate) const PARTNER_AGE: usize = 2;

const DEPOSIT: LineKindDecl = LineKindDecl {
    name: "deposit",
    asset: SideDecl {
        holder_kinds: &[HH],
        words: BALANCE,
        holder_list: false,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    liability: SideDecl {
        holder_kinds: &["bank"],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    transfer_requesters: &["BFL"],
    dated: false,
};

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
        holder_kinds: &[HH],
        words: BALANCE,
        holder_list: true,
        holder_roles: &[],
        exclusive: false,
        many: false,
    },
    transfer_requesters: &["BNK"],
    dated: false,
};

const THREE: &[ProfileComponent] = &[ProfileComponent { name: "age_band", values: 3 }];
const SIX: &[ProfileComponent] =
    &[ProfileComponent { name: "age_band", values: 3 }, ProfileComponent { name: "health", values: 2 }];

fn steps(_: &'static str) -> Result<StepTable, String> {
    StepTable::new(&Partition { exp: 0, bounds: [0, 50, 100, 200].into() })
}

/// The household kind: a head and a partner, their profile groups, a composition in the key, income per member
/// scaled by spending, and one reviewed decision.
pub(crate) fn kind() -> PopKindDecl {
    let e = |system, item| PopEntry { system, kind: HH, item };
    let group = |name, role, components| PopItem::ProfileGroup(GroupDecl { name, role, components, clause: "REP.32" });
    let entries = [
        e("DEM", PopItem::Role(RoleDecl { name: "head", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
        e("DEM", PopItem::Role(RoleDecl { name: "partner", per_member: phx_core::RoleCount::One, clause: "REP.26" })),
        e("DEM", PopItem::KeyAttr(KeyAttrDecl { name: "DEM.composition", values: 4, clause: "REP.19" })),
        e("DEM", group("DEM.age", "head", THREE)),
        e("DEM", group("DEM.age_health", "head", SIX)),
        e("DEM", group("DEM.partner_age", "partner", THREE)),
        e("HH", PopItem::StandingRate(RateDecl { name: "HH.spending", unit: "money/day", clause: "REP.20" })),
        e(
            "HH",
            PopItem::Position(PositionDecl {
                name: "HH.income",
                unit: "money",
                of: PositionOf::Member,
                scale: ScaleRef::Rate("HH.spending"),
                steps: "REP.steps",
                clause: "REP.20",
            }),
        ),
        e("HH", PopItem::ReviewKind("HH.move")),
    ];
    PopKindDecl::compile(HH, &entries, &KinkRegistry::default(), &steps).unwrap()
}

/// A world's books reduced to what a cell's rows need: a deposit line, a loan line and a fund.
pub(crate) struct Books {
    pub ledger: Ledger<HeapBacking>,
    pub deposit: LineId,
    pub loan: LineId,
    pub fund: InstrumentId,
}

pub(crate) fn books(space: &mut AddressSpace) -> Books {
    let keys = HolderKeys::new(1);
    let mut ledger = Ledger::new(Instruments::new(space, 16, 16, 16, keys), Lines::new(space, 16, 16, 16, keys));
    let deposit = ledger.lines.declare_money(DEPOSIT).index();
    let loan = ledger.lines.declare_money(LOAN).index();
    let deposit = ledger.lines.open(deposit, TermsId::new(0), Missing::Absent);
    let loan = ledger.lines.open(loan, TermsId::new(0), Missing::Absent);
    let fund = ledger.instruments.issue(NewInstrument {
        family: InstrumentFamily::FundUnit,
        issuer: Missing::Present(PartyId::new(99)),
        unit: UnitId::new(0),
        ccy: Ccy::new(0),
        terms: TermsId::new(0),
    });
    Books { ledger, deposit, loan, fund }
}

/// A row placed on a cell as the opening would leave it.
pub(crate) fn row(line: LineId, side: Side, count: u32, balance: Missing<i64>) -> DetachedRow {
    DetachedRow {
        row: RelRow { line, count, record: 0, point: 0, role: role(side, 0), flags: 0 },
        optional: Optional { balance, ..Optional::NONE },
        arrears_since: Missing::Absent,
    }
}

/// The interner holding one key, the composition at `value`, for `cells` cells.
pub(crate) fn keys(kind: &PopKindDecl, value: u32, cells: i64) -> (KeyInterner, KeyRecord) {
    let mut record = KeyRecord::default();
    kind.key.set(&mut record, 0, value);
    let mut keys = KeyInterner::new();
    keys.apply(&KeyedReduce::run(None, &[vec![(record, cells)]], |a, v| *a += v));
    (keys, record)
}

/// A cell of a hundred households: heads by age and by age and health, partners by age, 10 000 of income, a deposit of
/// 50 001 shared by all, a loan of 9 000 owed by thirty, a fund held by ten, and 7 000 of review exposure.
pub(crate) fn cell(
    space: &mut AddressSpace,
    kind: &PopKindDecl,
    b: &mut Books,
    keys: &KeyInterner,
) -> (CellTable<HeapBacking>, Slot) {
    let mut t = CellTable::new(space, kind, TableId::new(1), 64, 16);
    let layout = t.profile_layout().clone();
    let mut p = Profile::empty(&layout);
    for (g, v, n) in [
        (AGE, 0, 50),
        (AGE, 1, 30),
        (AGE, 2, 20),
        (AGE_HEALTH, 0, 40),
        (AGE_HEALTH, 1, 10),
        (AGE_HEALTH, 2, 25),
        (AGE_HEALTH, 3, 5),
        (AGE_HEALTH, 4, 20),
        (PARTNER_AGE, 0, 40),
        (PARTNER_AGE, 2, 60),
    ] {
        p.add(&layout, g, v, n);
    }
    let Missing::Present(key) = keys.id(&keys_record(keys)) else { panic!("the fixture's key is interned") };
    let new = NewCell {
        party: PartyId::new(7),
        created: Day::new(0),
        weight: Weight::new(100),
        key,
        positions: &[10_000],
        profile: &p,
    };
    let s = t.add(space, new, kind, &[0]);
    b.ledger.attach_row(&mut t, PLACE, s, row(b.deposit, Side::Asset, 100, Missing::Present(50_001)));
    b.ledger.attach_row(&mut t, PLACE, s, row(b.loan, Side::Liability, 30, Missing::Present(9_000)));
    let holding = CellHolding {
        instrument: b.fund,
        count: 10,
        quantity: QtyRaw::from_raw(1_000),
        pooled_cost: Amount::from_raw(2_003),
    };
    b.ledger.attach_holding(&mut t, PLACE, s, holding);
    t.set_exposure(s, 0, 7_000);
    (t, s)
}

fn keys_record(keys: &KeyInterner) -> KeyRecord {
    keys.record(crate::key::KeyId::new(0))
}

pub(crate) fn cells<'a>(
    b: &'a mut Books,
    t: &'a mut CellTable<HeapBacking>,
    keys: &'a KeyInterner,
) -> Cells<'a, HeapBacking, HeapBacking> {
    Cells { ledger: &mut b.ledger, table: t, place: PLACE, keys }
}

pub(crate) fn draws(tag: &str, i: u32) -> Draws {
    Draws::new(stream_key(Seed::new(11), tag), Subject::new(SubjectTag::Party, 7), i, 0)
}

/// Line kinks as a list of points per line side.
#[derive(Debug, Default)]
pub(crate) struct Points(pub Vec<(LineId, Side, i64)>);

impl crate::check::LineKinks for Points {
    fn points(&self, line: LineId, side: Side) -> Vec<i64> {
        self.0.iter().filter(|(l, s, _)| (*l, *s) == (line, side)).map(|(_, _, p)| *p).collect()
    }
}

/// A table of cells as 10b meets it: its books, keys, directory and index.
pub(crate) struct Ten {
    pub space: AddressSpace,
    pub books: Books,
    pub table: CellTable<HeapBacking>,
    pub keys: KeyInterner,
    pub directory: phx_core::Directory,
    pub kind: PopKindDecl,
    pub index: crate::index::Index,
    pub kinks: Points,
}

/// A cell of a hundred members spending one a day, earning a hundred each, with a deposit and no loan.
pub(crate) const PLAIN: Spec = Spec { weight: 100, income: 10_000, deposit: 50_000, loan: None };

/// A cell to add: its weight, income, deposit, and loan if it has one.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Spec {
    pub weight: u32,
    pub income: i64,
    pub deposit: i64,
    pub loan: Option<(u32, i64)>,
}

impl Ten {
    pub(crate) fn new() -> Ten {
        let mut space = AddressSpace::empty();
        let books = books(&mut space);
        let kind = kind();
        let table = CellTable::new(&mut space, &kind, TableId::new(1), 64, 16);
        Ten {
            space,
            books,
            table,
            keys: KeyInterner::new(),
            directory: phx_core::Directory::new(),
            kind,
            index: crate::index::Index::new(),
            kinks: Points::default(),
        }
    }

    pub(crate) fn key(&self, composition: u32) -> KeyRecord {
        let mut record = KeyRecord::default();
        self.kind.key.set(&mut record, 0, composition);
        record
    }

    /// A cell of the composition given, its heads all young and healthy and its partners all young, spending one a
    /// member a day.
    pub(crate) fn add(&mut self, composition: u32, spec: Spec) -> (PartyId, Slot) {
        let key = self.key(composition);
        crate::landing::hold_key(&mut self.keys, key, 1);
        let Missing::Present(id) = self.keys.id(&key) else { panic!("held") };
        let layout = self.table.profile_layout().clone();
        let mut p = Profile::empty(&layout);
        for g in [AGE, AGE_HEALTH, PARTNER_AGE] {
            p.add(&layout, g, 0, spec.weight);
        }
        let party = PartyId::new(self.directory.next());
        let new = NewCell {
            party,
            created: Day::new(0),
            weight: Weight::new(spec.weight),
            key: id,
            positions: &[spec.income],
            profile: &p,
        };
        let s = self.table.add(&mut self.space, new, &self.kind, &[0]);
        assert_eq!(self.directory.begin(phx_id::RowRef { table: self.table.id(), slot: s }), party);
        self.table.set_rate(s, 0, Missing::Present(1));
        let b = &mut self.books;
        b.ledger.attach_row(
            &mut self.table,
            PLACE,
            s,
            row(b.deposit, Side::Asset, spec.weight, Missing::Present(spec.deposit)),
        );
        if let Some((count, balance)) = spec.loan {
            b.ledger.attach_row(
                &mut self.table,
                PLACE,
                s,
                row(b.loan, Side::Liability, count, Missing::Present(balance)),
            );
        }
        self.table.rekey(s, &self.kind, &[0]);
        crate::landing::LandingIndex::insert(&mut self.index, self.table.hot(s).landing_key, party, s);
        (party, s)
    }

    pub(crate) fn tenb(&mut self) -> crate::landing::TenB<'_, HeapBacking, HeapBacking> {
        crate::landing::TenB {
            ledger: &mut self.books.ledger,
            table: &mut self.table,
            place: PLACE,
            keys: &mut self.keys,
            directory: &mut self.directory,
            space: &mut self.space,
            kind: &self.kind,
            levels: &[0],
            kinks: &self.kinks,
            today: Day::new(3),
        }
    }

    /// Members split from a cell, drawn from the stream given.
    pub(crate) fn split(&mut self, slot: Slot, seq: u32, count: u32, tag: &str) -> crate::part::Part {
        let origin = self.table.party(slot);
        let mut cells =
            Cells { ledger: &mut self.books.ledger, table: &mut self.table, place: PLACE, keys: &self.keys };
        let spec = crate::split::SplitSpec {
            count,
            given: &[],
            rows: &[],
            own: &[],
            reviewed: Missing::Absent,
            rounding: phx_num::round::Round::HalfEven,
        };
        match crate::split::split(&mut cells, slot, crate::part::PartId { origin, seq }, &spec, &mut draws(tag, seq)) {
            crate::split::Parted::Part(p) => *p,
            crate::split::Parted::Whole => panic!("a part was expected"),
        }
    }
}
