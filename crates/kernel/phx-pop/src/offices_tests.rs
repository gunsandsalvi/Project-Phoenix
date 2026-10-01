//! The offices over a few hand-founded institutions: the office of a kind at its block's offset, holders by ownership
//! and by appointment, empty offices reading none, a death vacating every office, blocks reused and grown, and the
//! founding preferences an institution must hold.
#![cfg(test)]

use std::panic::{AssertUnwindSafe, catch_unwind};

use phx_id::{ContractLink, Day, PartyKey, PartyRef, Slot};
use phx_num::Missing;
use phx_store::{AddressSpace, HeapBacking};

use super::{Holder, OfficeRef, Offices, founding};
use crate::directory::Directory;

type Heap = HeapBacking<4096>;

const PERSONS: u8 = 0;
const FIRMS: u8 = 1;
/// A company's offices, as its form declares them: a chief executive, then a line head.
const COMPANY: [u16; 2] = [3, 5];

fn fixture() -> (Directory<Heap>, Offices<Heap>) {
    let mut space = AddressSpace::empty();
    let dir = Directory::new(&mut space, &[16, 16], 16, (Day::new(0), 100));
    (dir, Offices::new(&mut space, PERSONS, 64))
}

fn firm(slot: u32) -> PartyKey {
    PartyKey::new(FIRMS, Slot::new(slot))
}

fn opened(o: &mut Offices<Heap>, f: PartyKey) -> OfficeRef {
    match o.open(f, &COMPANY) {
        Missing::Present(first) => first,
        Missing::Absent => panic!("a company declares offices"),
    }
}

fn refused(f: impl FnOnce()) -> bool {
    catch_unwind(AssertUnwindSafe(f)).is_err()
}

#[test]
fn office_of_kind_is_first_plus_offset() {
    let (_, mut o) = fixture();
    let (a, b) = (opened(&mut o, firm(0)), opened(&mut o, firm(1)));
    let line = o.office(b, (1, COMPANY[1]), firm(1));
    assert_eq!(line.row().get(), b.row().get() + 1);
    assert_eq!(o.office(a, (0, COMPANY[0]), firm(0)), a);
    assert!(
        refused(|| {
            let _ = o.office(a, (1, COMPANY[0]), firm(0));
        }),
        "another kind at the offset"
    );
    assert!(
        refused(|| {
            let _ = o.office(a, (2, COMPANY[0]), firm(0));
        }),
        "past the block, another institution's"
    );
}

#[test]
fn owner_holder_direct() {
    let (mut dir, mut o) = fixture();
    let owner = dir.begin(PERSONS);
    let first = opened(&mut o, firm(0));
    o.fill_owned(first, owner, Day::new(4));
    assert_eq!((o.holder(first, &dir), o.since(first)), (Holder::Owner(owner), Missing::Present(Day::new(4))));
    let not_a_person = PartyRef::new(FIRMS, 0, Slot::new(0));
    assert!(refused(|| o.fill_owned(first, not_a_person, Day::new(4))));
}

#[test]
fn appointed_holder_decodes_contract_ref() {
    let (dir, mut o) = fixture();
    let first = opened(&mut o, firm(0));
    let line = o.office(first, (1, COMPANY[1]), firm(0));
    let contract = ContractLink::new(7, Slot::new(1234));
    o.begin_filling(line, true);
    assert_eq!(o.flags(line), (true, true));
    o.fill_appointed(line, contract);
    assert_eq!(o.holder(line, &dir), Holder::Appointment(contract));
    assert_eq!((o.since(line), o.flags(line)), (Missing::Absent, (false, true)), "its start is the contract's");
}

#[test]
fn no_offices_no_block() {
    let (_, mut o) = fixture();
    assert_eq!(o.open(firm(0), &[]), Missing::Absent, "a household declares no office");
    assert_eq!(phx_store::StoreStats::rows_ever(&o), 0);
}

#[test]
fn empty_office_reads_missing() {
    let (dir, mut o) = fixture();
    let first = opened(&mut o, firm(0));
    assert_eq!((o.holder(first, &dir), o.since(first)), (Holder::Vacant, Missing::Absent));
}

#[test]
fn fill_and_vacate_same_day_in_order() {
    let (mut dir, mut o) = fixture();
    let (p, q) = (dir.begin(PERSONS), dir.begin(PERSONS));
    let first = opened(&mut o, firm(0));
    o.fill_owned(first, p, Day::new(2));
    o.vacate(first);
    o.fill_owned(first, q, Day::new(2));
    assert_eq!(o.holder(first, &dir), Holder::Owner(q), "the day's last event stands");
    o.vacate(first);
    assert_eq!(o.holder(first, &dir), Holder::Vacant);
}

#[test]
fn death_vacates_all_offices() {
    let (mut dir, mut o) = fixture();
    let p = dir.begin(PERSONS);
    let (a, b) = (opened(&mut o, firm(0)), opened(&mut o, firm(1)));
    let b_line = o.office(b, (1, COMPANY[1]), firm(1));
    o.fill_owned(a, p, Day::new(1));
    o.fill_appointed(b_line, ContractLink::new(2, Slot::new(9)));
    o.vacate_all([a, b_line]);
    dir.end(p, Day::new(3), Missing::Absent);
    assert_eq!((o.holder(a, &dir), o.holder(b_line, &dir)), (Holder::Vacant, Holder::Vacant));
}

#[test]
fn missing_founding_preferences_stops() {
    assert_eq!(founding(Missing::Present(3), firm(0)), 3);
    assert!(refused(|| {
        let _ = founding(Missing::Absent, firm(0));
    }));
}

#[test]
fn line_head_grows_block() {
    let (mut dir, mut o) = fixture();
    let p = dir.begin(PERSONS);
    let (a, _) = (opened(&mut o, firm(0)), opened(&mut o, firm(1)));
    o.fill_owned(a, p, Day::new(1));
    let grown = o.grow(a, 6);
    assert_eq!(o.holder(grown, &dir), Holder::Owner(p), "the holders move with the block");
    assert_eq!(o.office(grown, (2, 6), firm(0)).row().get(), grown.row().get() + 2);
    // The block it left is taken by the next institution founded with as many offices.
    let c = opened(&mut o, firm(2));
    assert_eq!(c, a);
    o.close(c);
    assert_eq!(phx_store::StoreStats::rows_live(&o), 5);
}

#[test]
fn offices_save_round_trip() {
    let (mut dir, mut o) = fixture();
    let p = dir.begin(PERSONS);
    let a = opened(&mut o, firm(0));
    o.fill_owned(a, p, Day::new(1));
    let b = opened(&mut o, firm(1));
    o.close(b);
    let mut bytes = Vec::new();
    let mut writer = phx_store::Writer::new(&mut bytes).unwrap();
    phx_store::Saved::save(&o, &mut writer);
    writer.finish().unwrap();
    let mut source = bytes.as_slice();
    let mut reader = phx_store::Reader::new(&mut source).unwrap();
    let mut back: Offices<Heap> = phx_store::Saved::load(&mut reader).unwrap();
    assert_eq!(back.holder(a, &dir), Holder::Owner(p));
    assert_eq!(opened(&mut back, firm(2)), b, "the freed block waits in the save");
}
