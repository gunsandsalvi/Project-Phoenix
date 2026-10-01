//! The banks on their kind's store: each bank's books (its income-statement lines and its equity at the opening) and
//! its lending record (its standard, the applications it read, declined and quoted, the loans it made, what its
//! write-offs lost since its last review, and by class the loan-days it has held and the defaults it has seen). Its
//! site stays in the core's record until the remaining kinds move.

use phx_id::{PartyRef, Slot};
use phx_macros::clause;
use phx_num::{Missing, capacity_exceeded, violation};
use phx_pop::directory::Directory;
use phx_pop::kinds::{AttrW, KindStore, Row, Word};
use phx_pop::layout::{BANK, Layout};
use phx_store::{AddressSpace, SystemBacking};

use crate::account_lines::BookWords;
use crate::consts::bank::SHARE_ONE;
use crate::consts::{DAYS_A_YEAR, LOAN_CLASSES as CLASSES};

/// The groups a lending review and a fund stage gather.
const LENDING: u8 = 1;
const RESERVES: u8 = 2;

/// What a bank counts of the applications it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Count {
    Applications,
    Declined,
    Quoted,
    Lent,
}

/// The handles of a bank's lending record.
#[derive(Clone, Copy, Debug)]
struct Lending {
    standard: AttrW<u32>,
    applications: AttrW<u64>,
    declined: AttrW<u64>,
    quoted: AttrW<u64>,
    lent: AttrW<u64>,
    written: AttrW<i64>,
    loan_days: [AttrW<u64>; CLASSES],
    defaults: [AttrW<u32>; CLASSES],
}

/// The banks' store: their rows by slot, and their words' handles.
#[clause("BNK.5", "BNK.20")]
#[derive(Debug, phx_macros::Saved)]
pub struct BankStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    #[saved(skip, rebuild = BankStore::bind)]
    words: Option<(BookWords, Lending, AttrW<i64>)>,
}

/// The bank map's layout and its words' handles.
fn compiled() -> (Layout, BookWords, Lending, AttrW<i64>) {
    let Ok(mut layout) = Layout::compile(&BANK, &[]) else {
        violation!(clause = "REP.1", "the bank's layout refused");
    };
    let books = BookWords::bind(&mut layout);
    let l = &mut layout;
    let standard = handle(l, "standard", 0);
    let (applications, declined) = (handle(l, "applications", 0), handle(l, "declined", 0));
    let (quoted, lent) = (handle(l, "quoted", 0), handle(l, "lent", 0));
    let written = handle(l, "written", 0);
    let loan_days = core::array::from_fn(|i| handle(l, "loan_days", i));
    let defaults = core::array::from_fn(|i| handle(l, "defaults", i));
    let lending = Lending { standard, applications, declined, quoted, lent, written, loan_days, defaults };
    let Ok(target) = layout.writer("reserve_target", 0, "CB") else {
        violation!(clause = "REP.1", "a bank's reserves target refused");
    };
    (layout, books, lending, target)
}

/// A lending word's write handle, handed to the banks once.
fn handle<T: Word>(layout: &mut Layout, name: &str, i: usize) -> AttrW<T> {
    match u16::try_from(i).map_err(|e| e.to_string()).and_then(|i| layout.writer(name, i, "BNK")) {
        Ok(a) => a,
        Err(_) => violation!(clause = "REP.1", "a bank's lending word refused"),
    }
}

impl BankStore {
    /// The banks' store: room for `capacity` banks of kind `kind`.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, capacity: u32) -> BankStore {
        let (layout, books, lending, target) = compiled();
        BankStore { store: KindStore::new(space, kind, &layout, capacity), kind, words: Some((books, lending, target)) }
    }

    /// A loaded store's handles, compiled again from the bank's layout.
    fn bind(&mut self) -> u64 {
        let (_, books, lending, target) = compiled();
        self.words = Some((books, lending, target));
        0
    }

    /// The kind its banks are of.
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.kind
    }

    /// A bank the directory began, its books empty.
    pub fn begin(&mut self, dir: &Directory<SystemBacking>, r: PartyRef) {
        self.store.begin(dir, r, &[]);
    }

    fn words(&self) -> (BookWords, Lending, AttrW<i64>) {
        match self.words {
            Some(w) => w,
            None => violation!(clause = "REP.1", "a bank store read before its handles are bound"),
        }
    }

    /// The banks' rows and the words their books are kept in.
    #[must_use]
    pub fn books(&self) -> (&KindStore<SystemBacking>, BookWords) {
        (&self.store, self.words().0)
    }

    pub fn books_mut(&mut self) -> (&mut KindStore<SystemBacking>, BookWords) {
        let books = self.words().0;
        (&mut self.store, books)
    }

    /// A bank's lending record at a slot, its row gathered once; none for a slot no bank was begun at.
    #[must_use]
    pub fn lender(&self, slot: Slot) -> Option<LenderView<'_>> {
        Some(LenderView { row: self.store.gather_at(slot, LENDING)?, w: self.words().1 })
    }

    fn held<T: Word>(&self, slot: Slot, a: AttrW<T>) -> T {
        match self.lender(slot).map(|v| v.row.get(a.read())) {
            Some(Missing::Present(v)) => v,
            _ => violation!(clause = "BNK.20", "a lending word read for no bank", slot = slot.get()),
        }
    }

    pub fn set_standard(&mut self, slot: Slot, standard: u32) {
        let a = self.words().1.standard;
        self.store.set_at(slot, a, Missing::Present(standard));
    }

    /// One more of what a bank counts.
    pub fn count(&mut self, slot: Slot, what: Count) {
        let a = self.words().1.counted(what);
        let n = self.held(slot, a) + 1;
        self.store.set_at(slot, a, Missing::Present(n));
    }

    /// A write-off's loss added to what the bank's write-offs lost since its last review.
    pub fn add_written(&mut self, slot: Slot, amount: i64) {
        let a = self.words().1.written;
        let Some(v) = self.held(slot, a).checked_add(amount) else {
            capacity_exceeded!("a bank's written-off loss", i64::MAX, amount);
        };
        self.store.set_at(slot, a, Missing::Present(v));
    }

    /// What the bank's write-offs lost since its last review, taken for the review and begun again at nothing.
    pub fn take_written(&mut self, slot: Slot) -> i64 {
        let a = self.words().1.written;
        let v = self.held(slot, a);
        self.store.set_at(slot, a, Missing::Present(0));
        v
    }

    /// Loan-days a bank held in a class added to its record.
    pub fn add_loan_days(&mut self, slot: Slot, class: usize, days: u64) {
        let a = class_word(&self.words().1.loan_days, class);
        let Some(v) = self.held(slot, a).checked_add(days) else {
            capacity_exceeded!("a bank's loan-days in a class", u64::MAX, days);
        };
        self.store.set_at(slot, a, Missing::Present(v));
    }

    /// The share of its deposits a bank's reserves are held to; none before its first fund stage sets it.
    #[must_use]
    pub fn reserve_target(&self, slot: Slot) -> Option<f64> {
        let a = self.words().2;
        match self.store.gather_at(slot, RESERVES)?.get(a.read()) {
            Missing::Present(v) => Some(share_of(v)),
            Missing::Absent => None,
        }
    }

    /// A bank's reserves target set, as its word holds it.
    pub fn set_reserve_target(&mut self, slot: Slot, share: f64) {
        let a = self.words().2;
        self.store.set_at(slot, a, Missing::Present(share_word(share)));
    }

    /// A default the bank has seen in a class.
    pub fn add_default(&mut self, slot: Slot, class: usize) {
        let a = class_word(&self.words().1.defaults, class);
        let n = self.held(slot, a) + 1;
        self.store.set_at(slot, a, Missing::Present(n));
    }
}

impl Lending {
    /// The word a count is kept in.
    fn counted(&self, what: Count) -> AttrW<u64> {
        match what {
            Count::Applications => self.applications,
            Count::Declined => self.declined,
            Count::Quoted => self.quoted,
            Count::Lent => self.lent,
        }
    }
}

/// A share as its word holds it, in 2⁻³² parts; one past the word stops the run.
#[clause("MON.14")]
#[must_use]
pub fn share_word(share: f64) -> i64 {
    match phx_rand::float::floor_to_i64((share * SHARE_ONE).round()) {
        Some(v) => v,
        None => violation!(clause = "MON.14", "a reserves target beyond its word"),
    }
}

/// A share read back from its word.
#[must_use]
pub fn share_of(word: i64) -> f64 {
    phx_rand::float::from_i64(word) / SHARE_ONE
}

/// A class's word; a class past the record's stops the run.
fn class_word<T>(words: &[AttrW<T>; CLASSES], class: usize) -> AttrW<T> {
    match words.get(class) {
        Some(a) => *a,
        None => violation!(clause = "BNK.20", "a loan class past a bank's record", class = class),
    }
}

/// A bank's lending record as gathered.
#[derive(Clone, Copy, Debug)]
pub struct LenderView<'a> {
    row: Row<'a>,
    w: Lending,
}

impl LenderView<'_> {
    /// The worst class it admits; none before the lending law opens it.
    #[must_use]
    pub fn standard(&self) -> Option<u32> {
        match self.row.get(self.w.standard.read()) {
            Missing::Present(s) => Some(s),
            Missing::Absent => None,
        }
    }

    #[must_use]
    pub fn counted(&self, what: Count) -> u64 {
        match self.row.get(self.w.counted(what).read()) {
            Missing::Present(n) => n,
            Missing::Absent => violation!(clause = "BNK.20", "a bank's count never absent read absent"),
        }
    }

    /// The years of loans it has held in a class.
    #[must_use]
    pub fn loan_years(&self, class: usize) -> f64 {
        match self.row.get(class_word(&self.w.loan_days, class).read()) {
            Missing::Present(d) => phx_rand::float::from_u64(d) / DAYS_A_YEAR,
            Missing::Absent => violation!(clause = "BNK.20", "a bank's loan-days never absent read absent"),
        }
    }

    /// The defaults it has seen in a class.
    #[must_use]
    pub fn defaults(&self, class: usize) -> u64 {
        match self.row.get(class_word(&self.w.defaults, class).read()) {
            Missing::Present(n) => u64::from(n),
            Missing::Absent => violation!(clause = "BNK.20", "a bank's defaults never absent read absent"),
        }
    }
}

#[cfg(test)]
#[path = "bank_store_tests.rs"]
mod tests;
