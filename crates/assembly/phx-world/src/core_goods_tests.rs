//! A firm's size is derived: its staff's hours are its job contracts', whichever are open.
#![cfg(test)]

use phx_core::store::Family;
use phx_id::{Day, PartyKey, Slot};
use phx_store::{AddressSpace, HeapBacking};

use super::staff_hours;
use crate::core_day::Due;

const FIRM: u8 = 0;
const HOUSEHOLD: u8 = 1;
/// A full-time job's weekly hours, the first of the classes.
const FULL: [u32; 3] = [0, 35, 0];
const PART: [u32; 3] = [0, 14, 0];

fn job(firm: u32, household: u32, schedule: u32) -> Due {
    Due {
        ends: [PartyKey::new(FIRM, Slot::new(firm)), PartyKey::new(HOUSEHOLD, Slot::new(household))],
        amount: 1,
        nth: 1,
        schedule,
        person: u64::from(household),
        arrears: 0,
    }
}

#[test]
fn size_is_derived() {
    let mut space = AddressSpace::empty();
    let mut jobs: Family<Due, HeapBacking<4096>> =
        Family::new(&mut space, ([FIRM, HOUSEHOLD], [4, 8]), (16, 8), [true, true], (Day::new(1), 8));
    let classes = [FULL, PART];
    let hired: Vec<Slot> = [(0, 0, 0), (0, 1, 0), (0, 2, 1), (1, 3, 0)]
        .into_iter()
        .map(|(f, h, s)| jobs.open(job(f, h, s), None))
        .collect();
    let day = |hours: f64| hours / 7.0;
    assert!((staff_hours(&jobs, &classes, Slot::new(0)) - day(35.0 + 35.0 + 14.0)).abs() < 1e-12);
    assert!((staff_hours(&jobs, &classes, Slot::new(1)) - day(35.0)).abs() < 1e-12);
    jobs.close(hired[1]);
    assert!((staff_hours(&jobs, &classes, Slot::new(0)) - day(35.0 + 14.0)).abs() < 1e-12, "a job ended is gone");
    assert!(staff_hours(&jobs, &classes, Slot::new(2)).abs() < 1e-12, "a firm with no job has no staff");
}
