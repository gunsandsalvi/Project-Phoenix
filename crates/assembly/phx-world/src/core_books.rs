//! The banks' loan books on the core. Each creditor's book is kept by what moves it — each amount lent, each part of
//! a loan repaid, each balance written off when its contract closes owing one — and at each day's close it is held
//! to the balances its loan contracts still owe it; a difference is a finding, never a repair.

use std::collections::BTreeMap;

use phx_core::findings::{Finding, FindingOwner, Unit};
use phx_id::{Day, PartyKey};
use phx_macros::clause;

use crate::core::Core;

/// A creditor's loan book: what its loans owe it, and what was lent, repaid and written off since the opening.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoanBook {
    pub book: i128,
    pub lent: i128,
    pub repaid: i128,
    pub written_off: i128,
}

impl Core {
    /// What each creditor's loan contracts owe it now.
    fn owed_on_loans(&self) -> BTreeMap<PartyKey, i128> {
        let mut owed = BTreeMap::new();
        for family in self.families.iter().filter(|f| f.terms.iter().any(Option::is_some)) {
            for edge in family.store.edges.open_slots() {
                let Some(row) = family.store.edges.row(edge) else { continue };
                let at = usize::try_from(row.schedule).unwrap_or(usize::MAX);
                if family.terms.get(at).is_some_and(Option::is_some) {
                    *owed.entry(row.ends[1]).or_insert(0) += i128::from(row.amount);
                }
            }
        }
        owed
    }

    /// The books opened at what the opening's loans owe each creditor, what the opening moved being their start.
    pub fn open_loan_books(&mut self) {
        for family in &mut self.families {
            family.moves = crate::core_day::LoanMoves::default();
        }
        self.loan_books =
            self.owed_on_loans().into_iter().map(|(k, book)| (k, LoanBook { book, ..LoanBook::default() })).collect();
    }

    /// The day's moves entered on each creditor's book, and every book held to what its loans owe it.
    #[clause("BNK.11", "N1", "II.5")]
    pub(crate) fn book_loans(&mut self, day: Day) {
        for family in &mut self.families {
            let moves = std::mem::take(&mut family.moves);
            for (k, a) in moves.lent {
                let b = self.loan_books.entry(k).or_default();
                (b.book, b.lent) = (b.book + i128::from(a), b.lent + i128::from(a));
            }
            for (k, a) in moves.repaid {
                let b = self.loan_books.entry(k).or_default();
                (b.book, b.repaid) = (b.book - i128::from(a), b.repaid + i128::from(a));
            }
            for (k, a) in moves.written_off {
                let b = self.loan_books.entry(k).or_default();
                (b.book, b.written_off) = (b.book - i128::from(a), b.written_off + i128::from(a));
            }
        }
        let owed = self.owed_on_loans();
        for (k, b) in &self.loan_books {
            let held = owed.get(k).copied().unwrap_or(0);
            if held != b.book {
                self.found.push(Finding {
                    family: "loans",
                    clause: "BNK.11",
                    owner: FindingOwner::Run,
                    size: held - b.book,
                    unit: Unit::Count,
                    day,
                    detail: format!("creditor {}: its book holds {} where its loans owe {held}", k.word(), b.book),
                });
            }
        }
    }
}
