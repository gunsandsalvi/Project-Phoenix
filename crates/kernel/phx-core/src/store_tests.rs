//! The stores' bookkeeping over a few parties and contracts: records and accounts at a party's slot, a released slot
//! begun again, the refusals, and a family's lists and wheel.
#![cfg(test)]

use phx_id::{Day, PartyKey, PartyRef, Slot};
use phx_num::MaybeI64;
use phx_store::backing::{AddressSpace, SystemBacking};
use phx_store::edges::Pair;

use super::{Family, KindStore, Opening, deposits_of};

const CAPACITY: u32 = 64;
const ROWS_PER_CHUNK: u32 = 16;
const STRIDE: usize = 3;

fn kind(space: &mut AddressSpace, money: bool) -> KindStore<SystemBacking> {
    let k = KindStore::new(space, CAPACITY, ROWS_PER_CHUNK, STRIDE);
    if money { k.with_accounts(space, CAPACITY, ROWS_PER_CHUNK) } else { k }
}

/// The party the directory hands out at a slot of kind 1, at a generation.
fn party(generation: u32, slot: u32) -> PartyRef {
    PartyRef::new(1, generation, Slot::new(slot))
}

#[test]
fn a_party_begins_with_its_record_and_account() {
    let mut space = AddressSpace::empty();
    let mut k = kind(&mut space, true);
    let a = k.begin(party(0, 0), &[MaybeI64::present(7)], Some(Opening { bank: 0, balance: 500 }));
    let b = k.begin(party(0, 1), &[MaybeI64::present(8), MaybeI64::present(9)], Some(Opening { bank: 1, balance: 20 }));
    assert_eq!(k.record(a.slot()), &[MaybeI64::present(7), MaybeI64::ABSENT, MaybeI64::ABSENT]);
    assert_eq!(k.record(b.slot())[1], MaybeI64::present(9));
    assert_eq!(k.money(), 520);
    assert_eq!(deposits_of(&[k], 2), vec![500, 20]);
}

#[test]
fn a_party_begun_again_in_a_released_slot_takes_its_own_record() {
    let mut space = AddressSpace::empty();
    let mut k = kind(&mut space, true);
    let a = k.begin(party(0, 0), &[MaybeI64::present(1), MaybeI64::present(2)], Some(Opening { bank: 0, balance: 5 }));
    let b = k.begin(party(1, 0), &[MaybeI64::present(3)], Some(Opening { bank: 0, balance: 9 }));
    assert_eq!(b.slot(), a.slot());
    assert_eq!(k.record(b.slot()), &[MaybeI64::present(3), MaybeI64::ABSENT, MaybeI64::ABSENT]);
    assert_eq!(k.money(), 9);
}

#[test]
fn kind_store_begins_at_directory_slot() {
    let mut space = AddressSpace::empty();
    let mut k = kind(&mut space, false);
    let a = k.begin(party(0, 0), &[MaybeI64::present(4)], None);
    let b = k.begin(party(2, 1), &[MaybeI64::present(5)], None);
    assert_eq!((a, b), (party(0, 0), party(2, 1)), "the store keeps the references the directory handed out");
    assert_eq!(k.record(Slot::new(1))[0], MaybeI64::present(5), "each record lies at its party's slot");
    assert_eq!(phx_store::StoreStats::rows_ever(&k), 2);
    let skipped = std::panic::catch_unwind(move || {
        let _ = k.begin(party(0, 5), &[], None);
    });
    assert!(skipped.is_err(), "a slot the directory has not reached yet has no row to write");
}

#[test]
fn a_kind_without_money_refuses_an_account() {
    let refused = std::panic::catch_unwind(|| {
        let mut space = AddressSpace::empty();
        let mut k = kind(&mut space, false);
        let _ = k.begin(party(0, 0), &[], Some(Opening { bank: 0, balance: 1 }));
    });
    assert!(refused.is_err());
}

#[test]
fn a_family_lists_each_partys_contracts_and_dues_them() {
    let mut space = AddressSpace::empty();
    let mut f: Family<Pair, SystemBacking> =
        Family::new(&mut space, ([1, 2], [8, 8]), (16, 16), [true, true], (Day::new(0), 8));
    let p = |kind: u8, slot: u32| PartyKey::new(kind, Slot::new(slot));
    let e0 = f.open(Pair { ends: [p(1, 3), p(2, 0)] }, Some(Day::new(2)));
    let e1 = f.open(Pair { ends: [p(1, 3), p(2, 1)] }, None);
    let mut mine: Vec<u32> = f.of(0, Slot::new(3)).map(Slot::get).collect();
    mine.sort_unstable();
    assert_eq!(mine, vec![e0.get(), e1.get()]);
    f.close(e0);
    assert_eq!(f.of(0, Slot::new(3)).map(Slot::get).collect::<Vec<_>>(), vec![e1.get()]);
    assert_eq!(f.of(1, Slot::new(0)).count(), 0);
    let mut due = Vec::new();
    for day in 0..3 {
        f.wheel.take(Day::new(day), &mut due, None);
    }
    assert_eq!(due, vec![e0.get()], "the wheel keeps a closed contract's entry for its reader to skip");
}

#[test]
fn a_family_refuses_a_side_of_another_kind() {
    let refused = std::panic::catch_unwind(|| {
        let mut space = AddressSpace::empty();
        let mut f: Family<Pair, SystemBacking> =
            Family::new(&mut space, ([1, 2], [8, 8]), (16, 16), [true, false], (Day::new(0), 8));
        let _ = f.open(Pair { ends: [PartyKey::new(2, Slot::new(0)), PartyKey::new(2, Slot::new(0))] }, None);
    });
    assert!(refused.is_err());
}
