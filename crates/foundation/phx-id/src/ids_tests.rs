//! Identities and references over hand-given values: their widths packed and refused past, and staleness told.
#![cfg(test)]

use phx_num::{CapacityExceeded, Missing, Violation};

use super::{ContractLink, ContractRef, HoldingRef, PartyId, PartyKey, PartyRef, SystemCode, TableRef};
use crate::Slot;

#[test]
fn party_id_refuses_2_pow_60() {
    assert_eq!(PartyId::new((1 << 60) - 1).get(), (1 << 60) - 1);
    let Err(payload) = std::panic::catch_unwind(|| PartyId::new(1 << 60)) else {
        panic!("2^60 accepted");
    };
    let c = payload.downcast_ref::<CapacityExceeded>().expect("a capacity payload");
    assert_eq!((c.declared, c.needed), (1 << 60, 1 << 60));
    let Err(payload) = std::panic::catch_unwind(|| PartyId::new(0)) else {
        panic!("zero accepted");
    };
    assert_eq!(payload.downcast_ref::<Violation>().expect("a violation").clause, "PTY.1");
}

#[test]
fn system_codes_are_two_to_four_capitals() {
    assert_eq!(SystemCode::new("DEM").map(|c| c.to_string()).as_deref(), Some("DEM"));
    assert_eq!(SystemCode::new("LABR").map(|c| c.to_string()).as_deref(), Some("LABR"));
    for bad in ["", "D", "DEMOG", "dem", "DE1", "DÉ"] {
        assert!(SystemCode::new(bad).is_none(), "{bad}");
    }
}

#[test]
fn party_ref_packs_kind_generation_and_slot() {
    let r = PartyRef::new(7, 0x00ab_cdef, Slot::new(0xdead_beef));
    assert_eq!((r.kind(), r.generation(), r.slot().get()), (7, 0x00ab_cdef, 0xdead_beef));
    assert_eq!(PartyRef::from_word(r.word()), r);
    let over = std::panic::catch_unwind(|| PartyRef::new(0, 1 << 24, Slot::new(0)));
    assert!(over.is_err(), "a generation past 24 bits is refused");
}

#[test]
fn party_key_packs_kind_and_slot() {
    let k = PartyKey::new(21, Slot::new(100_000_000));
    assert_eq!((k.kind(), k.slot().get()), (21, 100_000_000));
    assert!(std::panic::catch_unwind(|| PartyKey::new(32, Slot::new(0))).is_err());
    assert!(std::panic::catch_unwind(|| PartyKey::new(0, Slot::new(1 << 27))).is_err());
    let r = PartyRef::new(3, 9, Slot::new(44));
    assert_eq!(r.key(), PartyKey::new(3, Slot::new(44)));
}

#[test]
fn contract_link_packs_family_and_slot() {
    let link = ContractLink::new(254, Slot::new((1 << 24) - 1));
    assert_eq!((link.family(), link.slot().get()), (254, (1 << 24) - 1));
    assert_eq!(ContractLink::from_word(link.word()), link);
    let holdings = ContractLink::new(u32::from(crate::consts::HOLDINGS_FAMILY), Slot::new(7));
    assert_eq!((holdings.family(), holdings.slot().get()), (255, 7));
}

fn capacity(f: impl FnOnce() -> ContractLink + std::panic::UnwindSafe) -> (i128, i128) {
    let Err(payload) = std::panic::catch_unwind(f) else { panic!("a width past the link accepted") };
    let c = payload.downcast_ref::<CapacityExceeded>().expect("a capacity payload");
    (c.declared, c.needed)
}

#[test]
fn family_slot_width_stops() {
    assert_eq!(capacity(|| ContractLink::new(0, Slot::new(1 << 24))), (1 << 24, (1 << 24) + 1));
}

#[test]
fn family_code_width_stops() {
    assert_eq!(capacity(|| ContractLink::new(256, Slot::new(0))), (256, 257));
}

#[test]
fn contract_ref_generation_checked() {
    let link = ContractLink::new(3, Slot::new(40));
    let r = ContractRef::new(link, 9);
    assert_eq!(r.resolve(9), Missing::Present(link));
    assert_eq!(r.resolve(10), Missing::Absent, "the slot holds another row");
}

#[test]
fn table_ref_packs_slot_and_generation() {
    let h = HoldingRef::from_parts(Slot::new(0xdead_beef), 0x0102_0304);
    assert_eq!((h.slot().get(), h.generation()), (0xdead_beef, 0x0102_0304));
    assert_eq!(HoldingRef::from_word(h.word()), h);
}
