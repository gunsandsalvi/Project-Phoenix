//! Values interned, held and released by hand: a value has one id while any row holds it, retires at the close after
//! its last holder, and its slot returns only as its reuse declares.
#![cfg(test)]

use phx_num::Missing;

use super::{InternId, Interner, Reuse};
use crate::backing::{AddressSpace, HeapBacking};
use crate::roundtrip::roundtrip;
use crate::stats::StoreStats;

type Heap = HeapBacking<4096>;

fn interner(max_ids: u32, reuse: Reuse) -> Interner<Heap> {
    Interner::new(&mut AddressSpace::empty(), (max_ids, 1 << 16), reuse).unwrap()
}

fn value(i: u32) -> Vec<u8> {
    // Values of differing lengths, some sharing a prefix.
    let len = usize::try_from(i % 7).unwrap() + 1;
    i.to_le_bytes().iter().copied().cycle().take(len).collect()
}

#[test]
fn intern_twice_same_id() {
    let mut t = interner(16, Reuse::AfterClose);
    let a = t.intern(b"loan 5y fixed");
    let b = t.intern(b"loan 5y floating");
    assert_eq!(t.intern(b"loan 5y fixed"), a);
    assert_ne!(a, b);
    assert_eq!(t.count(a), Missing::Present(2));
    assert_eq!(t.find(b"loan 5y floating"), Missing::Present(b));
    assert_eq!(t.find(b"loan 7y fixed"), Missing::Absent);
}

#[test]
fn release_to_zero_retires_at_close() {
    let mut t = interner(16, Reuse::AfterClose);
    let a = t.intern(b"tenancy");
    t.release(a);
    assert_eq!(t.get(a), Missing::Present(&b"tenancy"[..]), "held by none, it is listed until the close");
    assert_eq!(t.intern(b"tenancy"), a, "held again the same day, it keeps its id");
    t.release(a);
    assert_eq!(t.close_day(), 1);
    assert_eq!(t.get(a), Missing::Absent);
    assert_eq!(t.find(b"tenancy"), Missing::Absent);
    assert_eq!(t.live(), 0);
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.hold(a)));
    assert!(caught.is_err(), "a retired id is held by no one");
}

#[test]
fn retired_not_reused_within_day() {
    let mut t = interner(16, Reuse::AfterClose);
    let a = t.intern(b"a");
    t.release(a);
    let b = t.intern(b"b");
    assert_ne!(b.slot(), a.slot(), "a slot listed today is not handed out today");
    let _ = t.close_day();
    let c = t.intern(b"c");
    assert_eq!((c.slot(), c.generation()), (a.slot(), 1), "after the close the slot returns behind its generation");
    assert_eq!(t.get(a), Missing::Absent, "the old id reads nothing");
    assert_eq!(t.get(c), Missing::Present(&b"c"[..]));
}

#[test]
fn get_is_one_read() {
    let mut t = interner(64, Reuse::AfterClose);
    let ids: Vec<InternId> = (0..40).map(|i| t.intern(&value(i))).collect();
    for (i, id) in (0..40).zip(&ids) {
        assert_eq!(t.get(*id), Missing::Present(value(i).as_slice()));
    }
    let empty = t.intern(b"");
    assert_eq!(t.get(empty), Missing::Present(&b""[..]), "an empty value is a value");
}

#[test]
fn same_value_one_id() {
    let mut t = interner(64, Reuse::AfterClose);
    let held = t.intern(b"lease 3y");
    // Two ranges each found the value new and staged it; the stage's end interns every staged value in range order,
    // the twice-staged one in the same group and more values than a group holds.
    let values: Vec<&[u8]> = vec![b"swap 10y", b"lease 3y", b"swap 10y"];
    let mut staged = values.clone();
    staged.extend((0..20).map(|_| &b"bond 2y"[..]));
    let (mut bytes, mut ends) = (Vec::new(), Vec::new());
    for v in &staged {
        bytes.extend_from_slice(v);
        ends.push(u32::try_from(bytes.len()).unwrap());
    }
    let mut found = vec![Missing::Absent; ends.len()];
    t.find_many((&bytes, &ends), &mut found);
    assert_eq!(
        found.iter().map(|f| *f == Missing::Absent).collect::<Vec<_>>(),
        staged.iter().map(|v| *v != b"lease 3y").collect::<Vec<_>>()
    );
    t.intern_many((&bytes, &ends), &mut found);
    let [Missing::Present(a), Missing::Present(b), Missing::Present(c), ..] = found[..] else { panic!("{found:?}") };
    assert_eq!((a, b), (c, held));
    assert_eq!(t.count(a), Missing::Present(2));
    assert_eq!(t.count(held), Missing::Present(2));
    assert!(found[3..].iter().all(|f| *f == found[3]), "a value staged across groups has one id");
    let Missing::Present(bond) = found[3] else { panic!() };
    assert_eq!(t.count(bond), Missing::Present(20));
}

/// The day's interns by range: each worker finds its ranges' values, staging those not yet interned, and the stage's
/// end interns every range's values in range order.
fn interned_by(workers: usize, ranges: &[Vec<Vec<u8>>]) -> Vec<Vec<InternId>> {
    let mut t = interner(256, Reuse::AfterClose);
    for i in 0..20 {
        let _ = t.intern(&value(i));
    }
    let mut staged: Vec<Vec<(usize, Missing<InternId>)>> = vec![Vec::new(); ranges.len()];
    std::thread::scope(|s| {
        let t = &t;
        let found: Vec<_> = (0..workers)
            .map(|w| {
                s.spawn(move || {
                    let mine = (w..ranges.len()).step_by(workers);
                    mine.map(|r| (r, ranges[r].iter().enumerate().map(|(k, v)| (k, t.find(v))).collect::<Vec<_>>()))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for h in found {
            for (r, f) in h.join().unwrap() {
                staged[r] = f;
            }
        }
    });
    let mut ids = Vec::new();
    for (r, found) in staged.iter().enumerate() {
        let mut range = Vec::new();
        for (k, f) in found {
            range.push(match f {
                Missing::Present(id) => {
                    t.hold(*id);
                    *id
                }
                Missing::Absent => t.intern(&ranges[r][*k]),
            });
        }
        ids.push(range);
    }
    ids
}

#[test]
fn ids_same_for_any_workers() {
    let ranges: Vec<Vec<Vec<u8>>> = (0..8_u32).map(|r| (0..12).map(|k| value(r * 5 + k * 3)).collect()).collect();
    let one = interned_by(1, &ranges);
    for workers in [2, 3, 8] {
        assert_eq!(interned_by(workers, &ranges), one, "{workers} workers");
    }
}

#[test]
fn churn_keeps_size() {
    let mut t = interner(64, Reuse::AfterClose);
    let mut yesterday: Vec<InternId> = Vec::new();
    let mut sizes = Vec::new();
    // Two hundred days over sixteen slots turn each slot about a hundred times, within its generations.
    for day in 0..200_u32 {
        let today: Vec<InternId> = (0..8).map(|k| t.intern(&(day * 8 + k).to_le_bytes())).collect();
        for id in yesterday {
            t.release(id);
        }
        yesterday = today;
        let _ = t.close_day();
        sizes.push((t.bytes(), t.live()));
    }
    assert!(sizes[50..].iter().all(|s| *s == sizes[50]), "the store stays its size: {:?}", &sizes[48..53]);
    assert_eq!(t.live(), 8);
    assert_eq!(t.rows_ever(), 200 * 8);
}

#[test]
fn rebuild_index_finds_every_value() {
    let mut t = interner(256, Reuse::AfterClose);
    let ids: Vec<InternId> = (0..200).map(|i| t.intern(&value(i * 13))).collect();
    for id in ids.iter().step_by(3) {
        t.release(*id);
    }
    let _ = t.close_day();
    let (back, rebuilt) = roundtrip(&t).unwrap();
    assert_eq!(rebuilt.stores(), [("Interner.index", t.live())]);
    for (i, id) in (0..200).zip(&ids) {
        let want = if i % 3 == 0 { Missing::Absent } else { Missing::Present(*id) };
        assert_eq!(back.find(&value(i * 13)), want, "value {i}");
    }
}

#[test]
fn slot_capacity_stops() {
    let mut t = interner(4, Reuse::AfterClose);
    for i in 0..4 {
        let _ = t.intern(&value(i));
    }
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.intern(&value(4))));
    assert!(caught.is_err(), "a fifth id in an interner of four stops the run");
    assert!(Interner::<Heap>::new(&mut AddressSpace::empty(), ((1 << 24) + 1, 16), Reuse::Never).is_err());
}

#[test]
fn generation_wrap_stops() {
    let mut t = interner(1, Reuse::AfterClose);
    for g in 0..=u8::MAX {
        let id = t.intern(&[g]);
        assert_eq!(id.generation(), g);
        t.release(id);
        let _ = t.close_day();
    }
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| t.intern(b"next")));
    assert!(caught.is_err(), "a slot's generation past its byte stops the run");
}

#[test]
fn never_reuse_keeps_retired_row() {
    let mut t = interner(16, Reuse::Never);
    let way = t.intern(b"way: smelting");
    t.release(way);
    let _ = t.close_day();
    assert!(t.is_retired(way));
    assert_eq!(t.get(way), Missing::Absent, "its bytes are released");
    let again = t.intern(b"way: smelting");
    assert_ne!(again.slot(), way.slot(), "a retired slot is never taken again");
    assert!(t.is_retired(way), "and the old id still reads as retired");
    assert_eq!(t.rows_ever(), 2);
}
