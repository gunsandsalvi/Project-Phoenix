//! The day's table: its slots in the day's order, a non-business day's list, day zero stage 5 alone, stage 4's
//! trips read at every load, and a read of a later slot's write refused.
#![cfg(test)]

use super::{AsOf, DAY_TABLE, Mode, Read, SlotDecl, compile};
use crate::consts::{BARRIERS_BUSINESS, BARRIERS_NON_BUSINESS};
use crate::slots::{DAY_SLOT_ORDER, DaySlot as S};

fn walked(mode: Mode, any_business: bool) -> Vec<S> {
    compile(&DAY_TABLE).unwrap().walk(mode, any_business).map(|s| s.slot).collect()
}

#[test]
fn slots_in_time6_order() {
    let table = compile(&DAY_TABLE).unwrap();
    assert_eq!(table.slots().iter().map(|s| s.slot).collect::<Vec<_>>(), DAY_SLOT_ORDER);
    let stages: Vec<u8> = table.slots().iter().filter(|s| s.slot != S::Save).map(|s| s.slot.stage()).collect();
    assert!(stages.windows(2).all(|w| w[0] <= w[1]), "stages in order");
    assert_eq!((stages.first(), stages.last()), (Some(&1), Some(&10)));
}

#[test]
fn non_business_runs_time8_list() {
    use S::*;
    let expected = [
        S1a, S1b, S2a, S3a, S3b, S4a, S4b, S4c, S5a, S5b, S5c, S5d, S6a, S6b, S6c, S6d, S10a, S10b, S10c, S10d, S10e,
        Save,
    ];
    assert_eq!(
        walked(Mode::Ordinary, false),
        expected,
        "lapses, accruals, nature, real work, retail decisions and meetings, the close"
    );
}

#[test]
fn closed_countries_skip_business_slots() {
    let open = walked(Mode::Ordinary, false);
    assert!(DAY_TABLE.iter().filter(|s| s.business_only).all(|s| !open.contains(&s.slot)));
}

#[test]
fn h_day_runs_every_business_slot() {
    assert_eq!(walked(Mode::Ordinary, true), DAY_SLOT_ORDER, "a heavy day's table is the business day's");
    assert_eq!(walked(Mode::Settling, true), walked(Mode::Ordinary, true), "settling runs ordinary days");
}

#[test]
fn day_zero_runs_stage_five_only() {
    assert_eq!(walked(Mode::DayZero, true), [S::S5a, S::S5b, S::S5c, S::S5d]);
    assert_eq!(walked(Mode::DayZero, false), walked(Mode::DayZero, true), "on the snapshot, whatever the calendar");
}

#[test]
fn stage_eight_slots_business_only() {
    let eight: Vec<&SlotDecl> = DAY_TABLE.iter().filter(|s| s.slot.stage() == 8).collect();
    assert_eq!(eight.iter().map(|s| s.slot).collect::<Vec<_>>(), [S::S8a, S::S8b, S::S8c, S::S8d, S::S8e, S::S8f]);
    assert!(eight.iter().all(|s| s.business_only && !s.on_non_business));
}

#[test]
fn trips_read_all_loads() {
    let writers: Vec<S> = DAY_TABLE.iter().filter(|s| s.writes.contains(&"loads")).map(|s| s.slot).collect();
    let readers: Vec<S> =
        DAY_TABLE.iter().filter(|s| s.reads.iter().any(|r| r.store == "loads")).map(|s| s.slot).collect();
    assert_eq!((writers.as_slice(), readers.as_slice()), ([S::S4b].as_slice(), [S::S4c].as_slice()));
    assert!(writers.iter().all(|w| readers.iter().all(|r| w < r)), "every trip placed before any is read");
}

const TODAYS_PRINTS: [Read; 1] = [Read { store: "prints", as_of: AsOf::Today }];
const AHEAD: [Read; 1] = [Read { store: "settled", as_of: AsOf::Through(S::S7b) }];
const NOWHERE: [Read; 1] = [Read { store: "rumours", as_of: AsOf::Yesterday }];

#[test]
fn later_read_refused() {
    // Stage 5 reading today's prints, which stage 6 writes, is refused; yesterday's are what it reads.
    let mut table = DAY_TABLE;
    let at = DAY_SLOT_ORDER.iter().position(|s| *s == S::S5a).unwrap();
    table[at] = SlotDecl { reads: &TODAYS_PRINTS, ..table[at] };
    let refused = compile(&table).unwrap_err();
    assert!(
        refused.iter().any(|e| e.contains("S5a reads `prints` as today's, which slot S6c writes later")),
        "{refused:?}"
    );
    table[at] = SlotDecl { reads: &AHEAD, ..DAY_TABLE[at] };
    assert!(compile(&table).unwrap_err().iter().any(|e| e.contains("through slot S7b, after it")));
    table[at] = SlotDecl { reads: &NOWHERE, ..DAY_TABLE[at] };
    assert!(compile(&table).unwrap_err().iter().any(|e| e.contains("which no slot writes")));
}

#[test]
fn barrier_counts_within_budget() {
    let table = compile(&DAY_TABLE).unwrap();
    let (b, nb) = (table.barriers(Mode::Ordinary, true), table.barriers(Mode::Ordinary, false));
    assert!(b <= BARRIERS_BUSINESS && nb <= BARRIERS_NON_BUSINESS, "{b} and {nb}");
    let mut busy = DAY_TABLE;
    for s in &mut busy {
        s.business_only = false;
        s.on_non_business = true;
    }
    assert!(compile(&busy).unwrap_err().iter().any(|e| e.contains("on a non-business day, past its")));
}
