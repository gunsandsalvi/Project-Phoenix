//! The bank's lending record over a hand-begun bank: each of its old fields a word of its own, the classes as many as
//! the map's, and its counts, loan-days, defaults and write-offs moved by their writers alone.
#![cfg(test)]

use phx_id::Day;
use phx_pop::directory::Directory;
use phx_store::{AddressSpace, SystemBacking};

use super::{BankStore, Count, compiled};
use crate::consts::{DAYS_A_YEAR, LOAN_CLASSES};

#[test]
fn lender_record_columns_declared() {
    assert_eq!(usize::from(phx_pop::consts::LOAN_CLASSES), LOAN_CLASSES, "the record's classes are the map's");
    let (_, _, w, _) = compiled();
    let mut places: Vec<(u8, u16)> = [w.standard.read().place(), w.written.read().place()]
        .into_iter()
        .chain([w.applications, w.declined, w.quoted, w.lent].iter().map(|a| a.read().place()))
        .chain(w.loan_days.iter().map(|a| a.read().place()))
        .chain(w.defaults.iter().map(|a| a.read().place()))
        .collect();
    let n = places.len();
    places.sort_unstable();
    places.dedup();
    assert_eq!(places.len(), n, "every field and class its own word");
    let mut space = AddressSpace::empty();
    let mut dir: Directory<SystemBacking> = Directory::new(&mut space, &[4], 4, (Day::new(0), 30));
    let mut banks = BankStore::new(&mut space, 0, 4);
    let bank = dir.begin(0);
    banks.begin(&dir, bank);
    let slot = bank.slot();
    assert_eq!(banks.lender(slot).unwrap().standard(), None, "no standard before the law opens it");
    banks.set_standard(slot, 9);
    banks.count(slot, Count::Applications);
    banks.count(slot, Count::Applications);
    banks.count(slot, Count::Quoted);
    banks.add_loan_days(slot, 3, 730);
    banks.add_default(slot, 3);
    banks.add_written(slot, 250);
    let l = banks.lender(slot).unwrap();
    assert_eq!((l.standard(), l.counted(Count::Applications), l.counted(Count::Quoted)), (Some(9), 2, 1));
    assert_eq!((l.counted(Count::Declined), l.defaults(3), l.defaults(4)), (0, 1, 0));
    assert!((l.loan_years(3) - 730.0 / DAYS_A_YEAR).abs() < f64::EPSILON);
    assert_eq!((banks.take_written(slot), banks.take_written(slot)), (250, 0), "taken once, begun again at nothing");
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| banks.add_default(slot, LOAN_CLASSES))).is_err());
}

#[test]
fn reserve_target_fixed_round_trip() {
    use super::{share_of, share_word};
    for share in [0.0, 0.035, 0.123_456_7, 1.5] {
        assert!((share_of(share_word(share)) - share).abs() <= 1.0 / crate::consts::bank::SHARE_ONE);
    }
    let mut space = AddressSpace::empty();
    let mut dir: Directory<SystemBacking> = Directory::new(&mut space, &[4], 4, (Day::new(0), 30));
    let mut banks = BankStore::new(&mut space, 0, 4);
    let bank = dir.begin(0);
    banks.begin(&dir, bank);
    assert_eq!(banks.reserve_target(bank.slot()), None, "no target before the first fund stage sets one");
    banks.set_reserve_target(bank.slot(), 0.08);
    assert_eq!(banks.reserve_target(bank.slot()), Some(share_of(share_word(0.08))));
}
