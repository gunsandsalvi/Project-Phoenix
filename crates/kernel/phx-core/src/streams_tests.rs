//! Streams over hand-given declarations: names unique across families, each family's code opening only its own, keyed
//! streams the same whenever read, and a draw at a slot its address's alone.
#![cfg(test)]

use phx_id::Day;
use phx_rand::{Draws, Seed, Subject, SubjectTag, family_key, open_unit, stream_key};

use super::{AdviceDraws, NotObserver, ObserverDraws, Purpose, StreamDecl, StreamFamily, WorldStreams};
use crate::slots::DaySlot;

const fn stream(name: &'static str, purpose: Purpose) -> StreamDecl {
    StreamDecl { name, family: StreamFamily::World, purpose, keyed: false, clause: "CHN.3" }
}

const TRACER: StreamDecl = StreamDecl {
    name: "OBS.tracer",
    family: StreamFamily::Observer,
    purpose: Purpose::Observer,
    keyed: false,
    clause: "OBS.1",
};
const PLAYER: StreamDecl = StreamDecl {
    name: "GEN.player",
    family: StreamFamily::Advice,
    purpose: Purpose::Opening,
    keyed: false,
    clause: "OBS.4",
};

#[test]
fn stream_names_unique_and_fnv_distinct() {
    let seed = Seed::new(1);
    assert!(
        WorldStreams::new(
            seed,
            &[stream("DEM.mortality", Purpose::Mortality), stream("DEM.illness", Purpose::Illness)]
        )
        .is_ok()
    );
    let twice = [stream("DEM.mortality", Purpose::Mortality), stream("DEM.mortality", Purpose::Mortality)];
    assert!(WorldStreams::new(seed, &twice).is_err());
}

#[test]
fn same_name_two_families_refused() {
    let world = stream("OBS.tracer", Purpose::Taste);
    let errors = WorldStreams::new(Seed::new(1), &[world, TRACER]).unwrap_err();
    assert!(errors.iter().any(|e| e.contains("two families")), "{errors:?}");
    // The observer's purpose belongs to its family alone.
    let astray = StreamDecl { family: StreamFamily::World, ..TRACER };
    assert!(WorldStreams::new(Seed::new(1), &[astray]).is_err());
}

#[test]
fn adding_a_stream_changes_no_other_key() {
    let seed = Seed::new(7);
    let names = ["DEM.mortality", "DEM.illness", "GEO.weather"];
    let before: Vec<_> = names.iter().map(|n| stream_key(seed, n)).collect();
    let decls: Vec<StreamDecl> =
        names.iter().chain(&["TEC.discovery"]).map(|n| stream(n, Purpose::Discovery)).collect();
    let with_one_more = WorldStreams::new(seed, &decls).unwrap();
    let subject = Subject::new(SubjectTag::Party, 5);
    for (d, key) in decls.iter().zip(before) {
        let mut a = with_one_more.open(d, subject, Day::new(3), 4);
        let mut b = Draws::new(key, subject, 3, 4);
        assert_eq!(open_unit(&mut a).to_bits(), open_unit(&mut b).to_bits(), "{}", d.name);
    }
}

#[test]
fn observer_refuses_world_stream() {
    let mortality = stream("DEM.mortality", Purpose::Mortality);
    let streams = WorldStreams::new(Seed::new(1), &[TRACER, mortality, PLAYER]).unwrap();
    let (observer, advice) = (ObserverDraws::new(&streams), AdviceDraws::new(&streams));
    let subject = Subject::new(SubjectTag::Party, 1);
    assert!(observer.open(&TRACER, subject, Day::new(1)).is_ok());
    assert_eq!(observer.open(&mortality, subject, Day::new(1)).err(), Some(NotObserver));
    assert_eq!(observer.open(&PLAYER, subject, Day::new(1)).err(), Some(NotObserver));
    assert!(advice.open(&PLAYER, subject, Day::new(1), 0).is_ok());
    assert_eq!(advice.open(&mortality, subject, Day::new(1), 0).err(), Some(NotObserver));
    // The world opens neither the observer's nor the advice's.
    assert!(std::panic::catch_unwind(|| streams.open(&TRACER, subject, Day::new(1), 0)).is_err());
    assert!(std::panic::catch_unwind(|| streams.open(&PLAYER, subject, Day::new(1), 0)).is_err());
    // An advice stream's draws are its family's key's.
    let mut a = advice.open(&PLAYER, subject, Day::new(1), 0).unwrap();
    let mut b = Draws::new(family_key(Seed::new(1), StreamFamily::Advice, "GEN.player"), subject, 1, 0);
    assert_eq!(a.next_u64(), b.next_u64());
}

#[test]
fn world_draws_by_slot() {
    let taste = stream("HH.taste", Purpose::Taste);
    let streams = WorldStreams::new(Seed::new(4), &[taste]).unwrap();
    let draw = |party: u64, slot: DaySlot| {
        streams.open_at(&taste, Subject::new(SubjectTag::Party, party), Day::new(9), slot.ordinal()).next_u64()
    };
    let forward: Vec<u64> = (0..20).map(|p| draw(p, DaySlot::S5b)).collect();
    let backward: Vec<u64> = (0..20).rev().map(|p| draw(p, DaySlot::S5b)).collect();
    assert_eq!(forward, backward.into_iter().rev().collect::<Vec<_>>(), "the order read changes no draw");
    assert_ne!(draw(3, DaySlot::S5b), draw(3, DaySlot::S6a), "each slot its own address");
    let key = stream_key(Seed::new(4), "HH.taste");
    let by_hand = Draws::at(key, Subject::new(SubjectTag::Party, 3), 9, DaySlot::S6a.ordinal()).next_u64();
    assert_eq!(draw(3, DaySlot::S6a), by_hand);
}

#[test]
fn keyed_streams_are_the_same_whenever_read() {
    let phase = StreamDecl {
        name: "TIME.schedule_phase",
        family: StreamFamily::World,
        purpose: Purpose::SchedulePhase,
        keyed: true,
        clause: "TIME.5",
    };
    let streams = WorldStreams::new(Seed::new(2), &[phase]).unwrap();
    let subject = Subject::new(SubjectTag::Party, 9);
    let (mut a, mut b) = (streams.open_keyed(&phase, subject), streams.open_keyed(&phase, subject));
    assert_eq!(open_unit(&mut a).to_bits(), open_unit(&mut b).to_bits());
    let third = open_unit(&mut streams.open_keyed_at(&phase, subject, 3)).to_bits();
    assert_eq!(third, open_unit(&mut streams.open_keyed_at(&phase, subject, 3)).to_bits());
    assert_ne!(third, open_unit(&mut streams.open_keyed_at(&phase, subject, 4)).to_bits(), "each place its own");
    assert!(std::panic::catch_unwind(|| streams.open(&phase, subject, Day::new(1), 0)).is_err());
}
