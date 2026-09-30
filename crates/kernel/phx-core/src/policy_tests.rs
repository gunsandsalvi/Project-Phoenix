//! Policy schedules over hand-given calendars and changes: the value in force, what a party could know, periods split
//! at a change, the owner and the next business day enforced, succession, the register's refusal, and a save.
#![cfg(test)]

use phx_id::{CountryId, Date, Day, PartyRef, Slot};
use phx_num::{Missing, Rate, RatePeriod};

use super::{AnnounceRefused, PolicyBook, PolicyH};
use crate::calendar::Calendar;
use crate::calendar::testing::calendar;
use crate::register::values::ValueType;
use crate::register::{PrimDecl, PrimKind, PrimPeriod, RegisterBuilder, RoleId, Scope};

const C: CountryId = CountryId::new(0);

fn party(slot: u32) -> PartyRef {
    PartyRef::new(3, 0, Slot::new(slot))
}

fn day(cal: &Calendar, y: i32, m: u8, d: u8) -> Day {
    cal.day(Date::new(y, m, d).unwrap()).unwrap()
}

/// A book of one schedule opening at 100, owned by party 1, changed to 120 from 10 March 2025 and to 150 from
/// 1 April, both announced on Friday 7 March.
fn book(cal: &Calendar) -> (PolicyBook<i64>, PolicyH<i64>) {
    let mut b = PolicyBook::default();
    let h = b.open(100, C, party(1));
    let friday = day(cal, 2025, 3, 7);
    b.announce((h, party(1)), cal, (friday, day(cal, 2025, 4, 1), 150)).unwrap();
    b.announce((h, party(1)), cal, (friday, day(cal, 2025, 3, 10), 120)).unwrap();
    (b, h)
}

#[test]
fn in_force_is_last_effective() {
    let cal = calendar(2020);
    let (b, h) = book(&cal);
    assert_eq!(b.in_force(h, day(&cal, 2025, 3, 9)), 100);
    assert_eq!(b.in_force(h, day(&cal, 2025, 3, 10)), 120);
    assert_eq!(b.in_force(h, day(&cal, 2025, 3, 31)), 120);
    assert_eq!(b.in_force(h, day(&cal, 2025, 4, 1)), 150);
}

#[test]
fn announced_by_hides_future_announcements() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    let later = day(&cal, 2025, 4, 10);
    b.announce((h, party(1)), &cal, (later, day(&cal, 2025, 5, 1), 90)).unwrap();
    let known = |d| b.announced_by(h, d).map(|e| e.value).collect::<Vec<_>>();
    assert_eq!(known(day(&cal, 2025, 3, 6)), Vec::<i64>::new());
    assert_eq!(known(day(&cal, 2025, 3, 7)), vec![120, 150]);
    assert_eq!(known(later), vec![120, 150, 90]);
}

#[test]
fn segments_split_at_effective_day() {
    let cal = calendar(2020);
    let (b, h) = book(&cal);
    let (from, until) = (day(&cal, 2025, 1, 1), day(&cal, 2026, 1, 1));
    let parts: Vec<_> = b.segments(h, (from, until)).collect();
    assert_eq!(
        parts,
        vec![
            (from, day(&cal, 2025, 3, 10), 100),
            (day(&cal, 2025, 3, 10), day(&cal, 2025, 4, 1), 120),
            (day(&cal, 2025, 4, 1), until, 150)
        ]
    );
    // A period within one value is one segment; one starting on a change's day starts with it.
    let march = (day(&cal, 2025, 3, 10), day(&cal, 2025, 3, 20));
    assert_eq!(b.segments(h, march).collect::<Vec<_>>(), vec![(march.0, march.1, 120)]);
}

#[test]
fn effective_too_soon_refused() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    let friday = day(&cal, 2025, 3, 7);
    assert_eq!(
        b.announce((h, party(1)), &cal, (friday, friday, 1)),
        Err(AnnounceRefused::TooSoon { earliest: day(&cal, 2025, 3, 10) }),
        "the same stage it is made"
    );
    assert!(b.announce((h, party(1)), &cal, (friday, day(&cal, 2025, 3, 8), 1)).is_err(), "a Saturday");
}

#[test]
fn non_owner_refused() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    let friday = day(&cal, 2025, 3, 7);
    assert_eq!(b.announce((h, party(2)), &cal, (friday, day(&cal, 2025, 3, 20), 1)), Err(AnnounceRefused::NotOwner));
}

#[test]
fn owner_succession_moves_schedule() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    assert_eq!(b.succeed(party(1), party(9)), 1);
    let friday = day(&cal, 2025, 3, 7);
    let change = (friday, day(&cal, 2025, 6, 2), 7);
    assert_eq!(b.announce((h, party(1)), &cal, change), Err(AnnounceRefused::NotOwner), "the ended owner");
    assert!(b.announce((h, party(9)), &cal, change).is_ok(), "its successor");
}

#[test]
fn day_zero_reads_opening() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    b.open_day(Day::new(0));
    assert_eq!(b.read(h), 100);
    b.open_day(day(&cal, 2025, 3, 10));
    assert_eq!(b.read(h), 120);
    b.open_day(day(&cal, 2025, 4, 2));
    assert_eq!(b.read(h), 150);
}

#[test]
fn schedule_roundtrip() {
    let cal = calendar(2020);
    let (mut b, h) = book(&cal);
    b.open_day(day(&cal, 2025, 3, 12));
    let (mut back, _) = phx_store::roundtrip(&b).unwrap();
    assert_eq!(back.read(h), 120, "the cache rebuilt at the day last opened");
    for d in (day(&cal, 2025, 3, 1).get()..day(&cal, 2025, 5, 1).get()).map(Day::new) {
        assert_eq!(back.in_force(h, d), b.in_force(h, d));
    }
    back.open_day(day(&cal, 2025, 4, 1));
    assert_eq!(back.read(h), 150, "the change still to come was rebuilt pending");
}

const RATE: PrimDecl = PrimDecl {
    id: "CB.policy_rate",
    kind: PrimKind::Policy,
    unit: Missing::Present("per year"),
    period: Missing::Present(PrimPeriod::Year),
    decided_by: Missing::Present(RoleId("central_bank")),
    value: ValueType::Rate,
    clause: "CB.1",
    shape: Missing::Absent,
    scope: Scope::PerCountry,
};

#[test]
fn absent_policy_refused() {
    // A register that declares no policy rate has no handle for it, so no schedule is ever read as nothing.
    let register = RegisterBuilder::new().build(&[], 1).unwrap();
    assert!(register.handle::<Rate>(&RATE).is_err());
    // A primitive declared otherwise than a policy has no schedule.
    let mut b = RegisterBuilder::new();
    let tech = PrimDecl { kind: PrimKind::Technology, decided_by: Missing::Absent, ..RATE };
    let prim = b.declare::<Rate>(&tech);
    let entry = "[[primitive]]\nid = \"CB.policy_rate\"\nkind = \"TECHNOLOGY\"\nunit = \"per year\"\nperiod = \"year\"\n\
                 owner = \"CB\"\nsource = \"assumed\"\nsource_ref = \"A rate.\"\nvalue = 0.035\n";
    let file =
        crate::register::DataFile { path: "c0.toml".to_owned(), country: Missing::Present(C), text: entry.to_owned() };
    let register = b.build(&[file], 1).unwrap();
    let mut book = PolicyBook::<Rate>::default();
    assert!(book.bind((prim, &register), C, party(1)).is_err());
    let _ = RatePeriod::Year;
}
