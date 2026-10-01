//! The person kind over a few hand-begun households: the word's fields round-trip and refuse overflow, ages read from
//! birth dates, moves and endings keep every household's list the set of its persons, and the lists round-trip.
#![cfg(test)]

use std::collections::BTreeSet;
use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::{Date, Day, PartyRef, Slot};
use phx_num::Missing;
use phx_store::{AddressSpace, HeapBacking};

use super::{PersonKind, StoreHeads};
use crate::directory::{Directory, Resolved};
use crate::kinds::{AttrW, KindStore};
use crate::layout::{HOUSEHOLD, Layout, PERSON, width};
use phx_core::person_word::{
    BIRTH_YEAR, EDUCATION, EDUCATION_FIELD, HEALTH, LABOUR, LIFE_RECORD, OCCUPATION, POINT, PersonWord, ROLE, SEX,
    Skills,
};

type Heap = HeapBacking<4096>;

const PERSONS: u8 = 0;
const HOUSEHOLDS: u8 = 1;

struct World {
    dir: Directory<Heap>,
    persons: PersonKind<Heap>,
    households: KindStore<Heap>,
    head: AttrW<u32>,
}

impl World {
    fn new() -> World {
        let mut space = AddressSpace::empty();
        let dir = Directory::new(&mut space, &[64, 16], 16, (Day::new(0), 100));
        let mut layout = Layout::compile(&HOUSEHOLD, &[]).unwrap();
        let head = layout.writer("persons_head", 0, "K-33").unwrap();
        let households = KindStore::new(&mut space, HOUSEHOLDS, &layout, 16);
        World { dir, persons: PersonKind::new(&mut space, PERSONS, 64), households, head }
    }

    fn household(&mut self) -> PartyRef {
        let r = self.dir.begin(HOUSEHOLDS);
        self.households.begin(&self.dir, r, &[]);
        r
    }

    fn person(&mut self, household: PartyRef, year: i32) -> PartyRef {
        let mut heads = StoreHeads { store: &mut self.households, head: self.head };
        let word = PersonWord::new(Date::new(year, 3, 1).unwrap(), 0);
        self.persons.begin_person(&mut self.dir, &mut heads, household, (word, &[]))
    }

    fn members(&mut self, household: PartyRef) -> BTreeSet<u32> {
        let heads = StoreHeads { store: &mut self.households, head: self.head };
        self.persons.members(&heads, household.slot()).map(Slot::get).collect()
    }

    fn move_to(&mut self, p: PartyRef, to: PartyRef) {
        let mut heads = StoreHeads { store: &mut self.households, head: self.head };
        self.persons.move_person(&self.dir, &mut heads, p, to);
    }

    fn end(&mut self, p: PartyRef, day: u32) {
        let mut heads = StoreHeads { store: &mut self.households, head: self.head };
        self.persons.end_person(&mut self.dir, &mut heads, p, (Day::new(day), Missing::Absent));
    }
}

fn slots(ps: &[PartyRef]) -> BTreeSet<u32> {
    ps.iter().map(|p| p.slot().get()).collect()
}

fn refused(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn word_fields_round_trip() {
    let born = Date::new(1911, 2, 28).unwrap();
    let w = PersonWord::new(born, 3);
    let fields = [(SEX, 1), (HEALTH, 3), (EDUCATION, 15), (EDUCATION_FIELD, 9), (LABOUR, 2), (OCCUPATION, 10)];
    let w = fields.iter().fold(w, |w, (f, v)| w.with(*f, *v)).with(POINT, 127).with(LIFE_RECORD, 1);
    assert_eq!((w.born(), w.get(ROLE)), (born, 3), "the birth date and role kept beside every other field");
    for (f, v) in fields {
        assert_eq!(w.get(f), v);
    }
    assert_eq!((w.get(POINT), w.get(LIFE_RECORD)), (127, 1));
    assert_eq!(w.with(HEALTH, 0).get(EDUCATION), 15, "a field written leaves its neighbours");
    let late = PersonWord::new(Date::new(2090, 12, 31).unwrap(), 0);
    assert_eq!(late.born(), Date::new(2090, 12, 31).unwrap());
    assert_eq!(w.with(BIRTH_YEAR, late.get(BIRTH_YEAR)).birth_year(), 2090, "a year written leaves the rest");
    let ancient = PersonWord::new(Date::new(-500, 1, 1).unwrap(), 0);
    assert_eq!(ancient.born(), Date::new(-500, 1, 1).unwrap(), "a year before zero held");
    let s = Skills::default().with(0, 15).with(7, 4).with(3, 9);
    assert_eq!((s.get(0), s.get(3), s.get(7), s.get(1)), (15, 9, 4, 0));
}

#[test]
fn field_overflow_stops() {
    let w = PersonWord::new(Date::new(1980, 1, 1).unwrap(), 0);
    assert!(
        refused(|| {
            let _ = w.with(ROLE, 8);
        }),
        "a role past seven"
    );
    assert!(refused(|| {
        let _ = w.with(POINT, 128);
    }));
    assert!(
        refused(|| {
            let _ = Skills::default().with(2, 16);
        }),
        "a skill level past fifteen"
    );
    assert!(
        refused(|| {
            let _ = Skills::default().with(8, 1);
        }),
        "a ninth occupation family"
    );
}

#[test]
fn age_read_from_birth_day() {
    let w = PersonWord::new(Date::new(1980, 6, 15).unwrap(), 0);
    assert_eq!(w.age_on(Date::new(2025, 6, 14).unwrap()), 44);
    assert_eq!(w.age_on(Date::new(2025, 6, 15).unwrap()), 45, "a year older on its birthday");
    let leap = PersonWord::new(Date::new(2000, 2, 29).unwrap(), 0);
    assert_eq!(leap.age_on(Date::new(2021, 2, 28).unwrap()), 20, "a leap birthday falls on the 1st of March");
    assert_eq!(leap.age_on(Date::new(2021, 3, 1).unwrap()), 21);
    for (y, m, d) in [(1911, 2, 28), (1999, 12, 31), (2026, 1, 1)] {
        let born = Date::new(y, m, d).unwrap();
        let today = Date::new(2030, 7, 4).unwrap();
        assert_eq!(PersonWord::new(born, 0).age_on(today), phx_core::pop_process::age_on(born, today));
    }
}

#[test]
fn person_layout_is_66_bytes() {
    let w = World::new();
    assert_eq!((width(&PERSON), w.persons.store.widths()), (66, [32_u16, 20, 14].as_slice()));
}

#[test]
fn unbanked_reads_missing() {
    let mut w = World::new();
    let h = w.household();
    let p = w.person(h, 1990);
    let view = w.persons.view(&w.dir, p);
    assert_eq!(view.account(), Missing::Absent, "a person banking nowhere has no account");
    assert_eq!(
        (view.chain_head(), view.search_start(), view.goal()),
        (Missing::Absent, Missing::Absent, Missing::Absent)
    );
    assert_eq!(view.household(), h.slot());
}

#[test]
fn move_is_unlink_link_write() {
    let mut w = World::new();
    let (a, b) = (w.household(), w.household());
    let ps: Vec<PartyRef> = (0..4).map(|i| w.person(a, 1950 + i)).collect();
    let word = w.persons.view(&w.dir, ps[1]).word();
    w.move_to(ps[1], b);
    assert_eq!(w.members(a), slots(&[ps[0], ps[2], ps[3]]));
    assert_eq!(w.members(b), slots(&[ps[1]]));
    let view = w.persons.view(&w.dir, ps[1]);
    assert_eq!((view.household(), view.word()), (b.slot(), word), "its household written, its word untouched");
    let stale = w.dir.begin(HOUSEHOLDS);
    w.dir.end(stale, Day::new(0), Missing::Absent);
    assert!(refused(|| w.move_to(ps[0], stale)), "a household not live takes no one");
    assert!(refused(|| w.move_to(ps[0], ps[2])), "a person is no household");
}

#[test]
fn members_after_moves_is_the_set() {
    let mut w = World::new();
    let hs: Vec<PartyRef> = (0..4).map(|_| w.household()).collect();
    let ps: Vec<PartyRef> = (0..12).map(|i| w.person(hs[i % 4], 1960 + i32::try_from(i).unwrap())).collect();
    // Each person moved to the household after its own, the first of each list, a middle one and the last in turn.
    for (i, p) in ps.iter().enumerate() {
        w.move_to(*p, hs[(i + 1) % 4]);
    }
    for (k, h) in hs.iter().enumerate() {
        let expected: Vec<PartyRef> =
            ps.iter().enumerate().filter(|(i, _)| (i + 1) % 4 == k).map(|(_, p)| *p).collect();
        assert_eq!(w.members(*h), slots(&expected), "household {k}");
    }
}

#[test]
fn writes_disjoint_by_household() {
    let mut w = World::new();
    let hs: Vec<PartyRef> = (0..5).map(|_| w.household()).collect();
    let ps: Vec<PartyRef> = (0..20).map(|i| w.person(hs[i % 5], 1970)).collect();
    for (i, p) in ps.iter().enumerate().filter(|(i, _)| i % 3 == 0) {
        w.move_to(*p, hs[(i * 7) % 5]);
    }
    // Each person lies in one household's list, the one its row names: a household's writer reaches no other's.
    let mut seen = BTreeSet::new();
    for h in &hs {
        for s in w.members(*h) {
            assert!(seen.insert(s), "person {s} in two households");
            assert_eq!(w.persons.view_at(Slot::new(s)).unwrap().household(), h.slot());
        }
    }
    assert_eq!(seen, slots(&ps));
}

#[test]
fn ended_person_unlinked() {
    let mut w = World::new();
    let h = w.household();
    let ps: Vec<PartyRef> = (0..3).map(|i| w.person(h, 1940 + i)).collect();
    w.end(ps[1], 5);
    assert_eq!(w.members(h), slots(&[ps[0], ps[2]]), "unlinked the day it ends");
    assert_eq!(w.dir.resolve(ps[1]), Resolved::Ended { day: Day::new(5), successor: Missing::Absent });
    w.end(ps[2], 5);
    w.end(ps[0], 5);
    assert!(w.members(h).is_empty());
    assert!(refused(|| w.end(ps[0], 5)), "a person ended twice");
}

#[test]
fn many_deaths_one_household_range() {
    let mut w = World::new();
    let hs: Vec<PartyRef> = (0..8).map(|_| w.household()).collect();
    let ps: Vec<PartyRef> = (0..48).map(|i| w.person(hs[i % 8], 1930)).collect();
    // A catastrophe's day: two thirds of the persons of the first four households die, each an unlink and an ending.
    let dead: Vec<PartyRef> = ps.iter().enumerate().filter(|(i, _)| i % 8 < 4 && i % 3 != 0).map(|(_, p)| *p).collect();
    for p in &dead {
        w.end(*p, 9);
    }
    let _ = w.dir.close_day(Day::new(9));
    for (k, h) in hs.iter().enumerate() {
        let alive: Vec<PartyRef> =
            ps.iter().enumerate().filter(|(i, p)| i % 8 == k && !dead.contains(p)).map(|(_, p)| *p).collect();
        assert_eq!(w.members(*h), slots(&alive), "household {k}");
    }
}

#[test]
fn birth_and_death_same_day_in_order() {
    let mut w = World::new();
    let h = w.household();
    let ps: Vec<PartyRef> = (0..3).map(|i| w.person(h, 1960 + i)).collect();
    // The death before the birth, as the processes are declared: the newborn takes no slot the day released.
    w.end(ps[0], 3);
    let child = w.person(h, 2025);
    assert_ne!(child.slot(), ps[0].slot());
    assert_eq!(w.members(h), slots(&[ps[1], ps[2], child]));
    assert_eq!(w.persons.view(&w.dir, child).household(), h.slot());
}

#[test]
fn save_round_trip_lists() {
    let mut w = World::new();
    let hs: Vec<PartyRef> = (0..3).map(|_| w.household()).collect();
    let ps: Vec<PartyRef> = (0..9).map(|i| w.person(hs[i % 3], 1975)).collect();
    w.move_to(ps[4], hs[0]);
    w.end(ps[2], 1);
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&w.persons, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let mut back: PersonKind<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    phx_store::Saved::rebuild_derived(&mut back, &mut phx_store::rebuild::Rebuilt::default()).unwrap();
    let before: Vec<BTreeSet<u32>> = hs.iter().map(|h| w.members(*h)).collect();
    let heads = StoreHeads { store: &mut w.households, head: w.head };
    for (h, set) in hs.iter().zip(&before) {
        assert_eq!(&back.members(&heads, h.slot()).map(Slot::get).collect::<BTreeSet<u32>>(), set);
    }
    for p in ps.iter().filter(|p| **p != ps[2]) {
        assert_eq!(back.view(&w.dir, *p).word(), w.persons.view(&w.dir, *p).word());
    }
}
