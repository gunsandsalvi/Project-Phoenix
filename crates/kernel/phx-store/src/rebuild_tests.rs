//! A store read back is rebuilt to what it held, the same however its rebuild is cut, and a failing rebuild names
//! its store.
#![cfg(test)]

use super::{by_ranges, rebuild};
use crate::roundtrip::roundtrip;

/// A heavy day's close in small: rows by slot, and two indexes kept as the day went — each party's rows, and the
/// parties with any, which reads the first.
#[derive(Debug, PartialEq, phx_macros::Saved)]
struct Book {
    parties: u32,
    rows: Vec<(u32, i64)>,
    #[saved(skip, rebuild = Book::index_by_party)]
    by_party: Vec<Vec<u32>>,
    #[saved(skip, rebuild = Book::index_holders, after = by_party)]
    holders: Vec<u32>,
}

impl Book {
    fn by_party_of(&self, parties: std::ops::Range<usize>, out: &mut Vec<Vec<u32>>) {
        for p in parties {
            let party = u32::try_from(p).unwrap();
            out.push((0_u32..).zip(&self.rows).filter(|(_, r)| r.0 == party).map(|(slot, _)| slot).collect());
        }
    }

    fn index_by_party(&mut self) -> u64 {
        let parties = usize::try_from(self.parties).unwrap();
        self.by_party = by_ranges(parties, 4, |range, out| self.by_party_of(range, out));
        u64::try_from(self.rows.len()).unwrap()
    }

    fn index_holders(&mut self) -> u64 {
        self.holders = (0_u32..).zip(&self.by_party).filter(|(_, rows)| !rows.is_empty()).map(|(p, _)| p).collect();
        u64::try_from(self.holders.len()).unwrap()
    }

    /// The book as the day kept it: each row added to its party's list as it came.
    fn kept(parties: u32, rows: Vec<(u32, i64)>) -> Book {
        let mut by_party = vec![Vec::new(); usize::try_from(parties).unwrap()];
        for (slot, (party, _)) in (0_u32..).zip(&rows) {
            by_party[usize::try_from(*party).unwrap()].push(slot);
        }
        let holders = (0_u32..).zip(&by_party).filter(|(_, r)| !r.is_empty()).map(|(p, _)| p).collect();
        Book { parties, rows, by_party, holders }
    }
}

#[test]
fn roundtrip_heavy_fixture() {
    let rows: Vec<(u32, i64)> = (0..5_000_u32).map(|i| (i * 7 % 97, i64::from(i) - 2_500)).collect();
    let (back, rebuilt) = roundtrip(&Book::kept(120, rows)).unwrap();
    assert_eq!(rebuilt.stores(), [("Book.by_party", 5_000), ("Book.holders", 97)], "in declared order");
    assert_eq!(back.holders.len(), 97, "parties 97 to 119 hold no row");
}

#[test]
fn rebuild_same_for_any_workers() {
    let book = Book::kept(50, (0..800_u32).map(|i| (i * 13 % 50, 1)).collect());
    for ranges in 1..=8 {
        let mut out = Vec::new();
        let joined = by_ranges(50, ranges, |range, part| {
            book.by_party_of(range, part);
            out.push(part.len());
        });
        assert_eq!(joined, book.by_party, "{ranges} ranges joined in range order");
    }
}

/// A store whose index cannot come back from what it saved.
#[derive(Debug, PartialEq, phx_macros::Saved)]
struct Broken {
    rows: Vec<u8>,
    #[saved(skip, rebuild = Broken::refuse)]
    index: Vec<u8>,
}

impl Broken {
    fn refuse(&mut self) -> Result<u64, String> {
        Err(format!("{} rows name no row", self.rows.len()))
    }
}

#[test]
fn failed_rebuild_names_store() {
    let mut broken = Broken { rows: vec![1, 2], index: Vec::new() };
    let e = rebuild(&mut broken).unwrap_err();
    assert_eq!((e.store, e.why.as_str()), ("Broken.index", "2 rows name no row"));
    assert!(roundtrip(&broken).unwrap_err().contains("Broken.index"), "the load stops naming the store");
}
