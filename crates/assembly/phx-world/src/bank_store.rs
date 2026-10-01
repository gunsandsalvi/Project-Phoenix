//! The banks' books on their kind's store: each bank's income-statement lines and its equity at the opening, the bank
//! map's words, until its lending record and its site join them there.

use phx_id::PartyRef;
use phx_num::violation;
use phx_pop::directory::Directory;
use phx_pop::kinds::KindStore;
use phx_pop::layout::{BANK, Layout};
use phx_store::{AddressSpace, SystemBacking};

use crate::account_lines::BookWords;

/// The banks' store: their rows by slot, and their book words' handles.
#[derive(Debug, phx_macros::Saved)]
pub struct BankStore {
    pub store: KindStore<SystemBacking>,
    kind: u8,
    #[saved(skip, rebuild = BankStore::bind)]
    books: Option<BookWords>,
}

/// The bank map's layout and its book words' handles.
fn compiled() -> (Layout, BookWords) {
    let Ok(mut layout) = Layout::compile(&BANK, &[]) else {
        violation!(clause = "REP.1", "the bank's layout refused");
    };
    let books = BookWords::bind(&mut layout);
    (layout, books)
}

impl BankStore {
    /// The banks' store: room for `capacity` banks of kind `kind`.
    #[must_use]
    #[phx_macros::opening]
    pub fn new(space: &mut AddressSpace, kind: u8, capacity: u32) -> BankStore {
        let (layout, books) = compiled();
        BankStore { store: KindStore::new(space, kind, &layout, capacity), kind, books: Some(books) }
    }

    /// A loaded store's handles, compiled again from the bank's layout.
    fn bind(&mut self) -> u64 {
        self.books = Some(compiled().1);
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

    fn words(&self) -> BookWords {
        match self.books {
            Some(b) => b,
            None => violation!(clause = "REP.1", "a bank store read before its handles are bound"),
        }
    }

    /// The banks' rows and the words their books are kept in.
    #[must_use]
    pub fn books(&self) -> (&KindStore<SystemBacking>, BookWords) {
        (&self.store, self.words())
    }

    pub fn books_mut(&mut self) -> (&mut KindStore<SystemBacking>, BookWords) {
        let books = self.words();
        (&mut self.store, books)
    }
}
