//! The world's money and contracts on the core. A kind that holds money keeps its parties' accounts and the cash lines
//! their statements report; a family keeps its contracts, the list heads its listed sides' kinds keep, and the wheel
//! its contracts come due on. Every column is indexed by slot, so a party's or a contract's words are found by its slot
//! alone; a party's own words are its kind's store's.

use phx_id::{Day, PartyRef, Slot};
use phx_macros::clause;
use phx_num::{capacity_exceeded, violation};
use phx_store::backing::{AddressSpace, Backing};
use phx_store::edges::{NONE, Row};
use phx_store::{Column, EdgeTable, StoreStats};

use crate::settle::{Book, Books, Lines};
use crate::wheel::DueWheel;

/// A kind's accounts, each party's in its home currency, its country's: the bank each is held at, its balance, the
/// card payments pending, what it paid through a closed bank and is held, and the facility its terms grant.
#[clause("MON.2", "PTY.6")]
#[derive(Debug, phx_macros::Saved)]
pub struct Accounts<B: Backing> {
    pub bank: Column<u32, B>,
    pub balance: Column<i64, B>,
    pub pending: Column<i64, B>,
    pub held: Column<i64, B>,
    pub facility: Column<i64, B>,
}

/// A kind's parties' cash lines: `width` lines a party, and the line each reason's payments and receipts post to.
#[derive(Debug, phx_macros::Saved)]
pub struct CashLines<B: Backing> {
    pub amounts: Column<i64, B>,
    pub width: usize,
    pub paid: Vec<Option<usize>>,
    pub received: Vec<Option<usize>>,
}

/// A kind's parties' accounts and cash lines if the kind holds money, each at the slot the directory gave its party.
#[derive(Debug, phx_macros::Saved)]
pub struct KindMoney<B: Backing> {
    pub accounts: Option<Accounts<B>>,
    pub cash: Option<CashLines<B>>,
}

/// A kind's rows are its accounts, one for each slot ever handed out; which are live is the directory's.
impl<B: Backing> StoreStats for KindMoney<B> {
    fn rows_live(&self) -> u64 {
        self.rows_ever()
    }

    #[phx_macros::absent_is_zero(reason = "a kind that holds no money keeps no rows here")]
    fn rows_ever(&self) -> u64 {
        u64::try_from(self.accounts.as_ref().map_or(0, |a| a.balance.len())).unwrap_or(u64::MAX)
    }

    #[phx_macros::absent_is_zero(reason = "a kind that holds no money commits no bytes here")]
    fn bytes(&self) -> u64 {
        let accounts = self.accounts.as_ref().map_or(0, |a| {
            [a.balance.bytes_committed(), a.pending.bytes_committed(), a.held.bytes_committed()]
                .into_iter()
                .chain([a.facility.bytes_committed(), a.bank.bytes_committed()])
                .sum()
        });
        let own = accounts + self.cash.as_ref().map_or(0, |c| c.amounts.bytes_committed());
        u64::try_from(own).unwrap_or_else(|_| capacity_exceeded!("a store's bytes", u64::MAX, own))
    }
}

/// A new party's account: the bank it is held at and its opening balance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opening {
    pub bank: u32,
    pub balance: i64,
}

fn index(n: u32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

/// Where a slot's `i`th word of `stride` lies in a column of such words.
fn at(slot: Slot, stride: usize, i: usize) -> usize {
    index(slot.get())
        .checked_mul(stride)
        .and_then(|a| a.checked_add(i))
        .unwrap_or_else(|| violation!(clause = "REP.1", "a record beyond any column"))
}

fn word(slot: Slot, stride: usize, i: usize) -> Slot {
    Slot::new(
        u32::try_from(at(slot, stride, i))
            .unwrap_or_else(|_| violation!(clause = "REP.1", "a record beyond its column")),
    )
}

/// A value at a slot: written over a party begun again in a released slot, or put at the column's next row.
fn place<T: phx_store::pod::Pod, B: Backing>(column: &mut Column<T, B>, slot: Slot, value: T) {
    if index(slot.get()) < column.len() { column.set(slot, value) } else { column.put(slot, value) }
}

/// A kind that holds no money until accounts are added.
impl<B: Backing> Default for KindMoney<B> {
    fn default() -> Self {
        KindMoney { accounts: None, cash: None }
    }
}

impl<B: Backing> KindMoney<B> {
    /// The kind with accounts, one a party.
    #[must_use]
    pub fn with_accounts(mut self, space: &mut AddressSpace, capacity: u32, rows_per_chunk: u32) -> KindMoney<B> {
        self.add_accounts(space, capacity, rows_per_chunk);
        self
    }

    /// The kind with `width` cash lines a party, each reason posting its payments and receipts to the lines given.
    #[must_use]
    pub fn with_cash(
        mut self,
        space: &mut AddressSpace,
        (capacity, rows_per_chunk): (u32, u32),
        width: usize,
        (paid, received): (Vec<Option<usize>>, Vec<Option<usize>>),
    ) -> KindMoney<B> {
        let words = u32::try_from(at(Slot::new(capacity), width, 0))
            .unwrap_or_else(|_| violation!(clause = "ACC.9", "a kind's cash lines beyond a column"));
        self.cash = Some(CashLines { amounts: Column::new(space, words, rows_per_chunk), width, paid, received });
        self
    }

    /// A party the directory began given its account, which a kind holding money requires and any other refuses, at
    /// the slot the directory handed out.
    #[clause("PTY.9", "REP.1")]
    pub fn begin(&mut self, party: PartyRef, account: Option<Opening>) -> PartyRef {
        if self.accounts.is_some() != account.is_some() {
            violation!(clause = "Law 5", "an account for a kind that holds no money, or none for one that does");
        }
        let slot = party.slot();
        if let Some(o) = account {
            self.open_account(slot, o);
        }
        if let Some(c) = self.cash.as_mut() {
            for i in 0..c.width {
                place(&mut c.amounts, word(slot, c.width, i), 0);
            }
        }
        party
    }

    /// A party's account opened at its bank with its opening balance, nothing pending, held or granted.
    pub fn open_account(&mut self, slot: Slot, o: Opening) {
        let Some(a) = self.accounts.as_mut() else {
            violation!(clause = "Law 5", "an account for a kind that holds no money", slot = slot.get());
        };
        place(&mut a.bank, slot, o.bank);
        place(&mut a.balance, slot, o.balance);
        place(&mut a.pending, slot, 0);
        place(&mut a.held, slot, 0);
        place(&mut a.facility, slot, 0);
    }

    /// Accounts added to a kind whose parties have begun, each to be opened before settlement reads it.
    pub fn add_accounts(&mut self, space: &mut AddressSpace, capacity: u32, rows_per_chunk: u32) {
        if self.accounts.is_some() {
            violation!(clause = "Law 5", "a kind given accounts twice");
        }
        self.accounts = Some(Accounts {
            bank: Column::new(space, capacity, rows_per_chunk),
            balance: Column::new(space, capacity, rows_per_chunk),
            pending: Column::new(space, capacity, rows_per_chunk),
            held: Column::new(space, capacity, rows_per_chunk),
            facility: Column::new(space, capacity, rows_per_chunk),
        });
    }

    /// The kind's accounts as settlement reads them, if it holds money.
    #[must_use]
    pub fn book(&mut self) -> Option<Book<'_>> {
        let Accounts { bank, balance, pending, held, facility } = self.accounts.as_mut()?;
        let lines = self.cash.as_mut().map(|c| Lines {
            amounts: c.amounts.slice_mut(),
            width: c.width,
            paid: &c.paid,
            received: &c.received,
        });
        Some(Book {
            bank: bank.slice(),
            balance: balance.slice_mut(),
            pending: pending.slice_mut(),
            held: held.slice_mut(),
            facility: facility.slice(),
            lines,
        })
    }

    /// The kind's balances and pending together, summed.
    #[must_use]
    #[phx_macros::absent_is_zero(reason = "a kind that holds no money holds none")]
    pub fn money(&self) -> i128 {
        self.accounts
            .as_ref()
            .map_or(0, |a| a.balance.slice().iter().chain(a.pending.slice()).map(|x| i128::from(*x)).sum())
    }
}

/// Every kind's accounts in one currency as settlement reads them.
#[must_use]
pub fn books<'a, B: Backing>(
    kinds: &'a mut [KindMoney<B>],
    banks: u8,
    (deposits, closed): (&'a mut [i64], &'a [bool]),
    issuer: phx_id::PartyKey,
) -> Books<'a> {
    Books { kinds: kinds.iter_mut().map(KindMoney::book).collect(), banks, deposits, closed, issuer }
}

/// What each bank's customers hold with it, their pending included: what each bank owes them.
#[must_use]
pub fn deposits_of<'a, B: Backing + 'a>(kinds: impl IntoIterator<Item = &'a KindMoney<B>>, banks: usize) -> Vec<i64> {
    let mut owed = vec![0_i64; banks];
    for a in kinds.into_iter().filter_map(|k| k.accounts.as_ref()) {
        for ((b, m), p) in a.bank.slice().iter().zip(a.balance.slice()).zip(a.pending.slice()) {
            if let Some(o) = owed.get_mut(index(*b)) {
                *o += m + p;
            }
        }
    }
    owed
}

/// A family's contracts: the table, the list heads its listed sides' kinds keep, a head a party, and the wheel its
/// contracts come due on.
#[derive(Debug)]
pub struct Family<R: Row, B: Backing> {
    pub edges: EdgeTable<R, B>,
    pub heads: [Option<Column<u32, B>>; 2],
    pub wheel: DueWheel,
    pub kinds: [u8; 2],
}

impl<R: Row, B: Backing> phx_store::Saved for Family<R, B> {
    fn save(&self, w: &mut phx_store::Writer<'_>) {
        let [a, b] = &self.heads;
        self.edges.save(w);
        a.save(w);
        b.save(w);
        self.wheel.save(w);
        self.kinds.save(w);
    }

    fn load(r: &mut phx_store::Reader<'_>) -> Result<Family<R, B>, phx_store::LoadError> {
        let edges = EdgeTable::load(r)?;
        let heads = [Option::load(r)?, Option::load(r)?];
        Ok(Family { edges, heads, wheel: DueWheel::load(r)?, kinds: <[u8; 2]>::load(r)? })
    }
}

impl<R: Row, B: Backing> Family<R, B> {
    /// An empty family of up to `capacity` contracts between parties of `kinds`, each listed side's heads one a party
    /// of its kind's capacity, its wheel starting at `first` over `horizon` days.
    #[must_use]
    pub fn new(
        space: &mut AddressSpace,
        (kinds, parties): ([u8; 2], [u32; 2]),
        (capacity, rows_per_chunk): (u32, u32),
        listed: [bool; 2],
        (first, horizon): (Day, u32),
    ) -> Family<R, B> {
        let heads = [0, 1].map(|side| {
            listed.get(side).copied().unwrap_or(false).then(|| {
                let size = parties.get(side).copied().unwrap_or(0);
                let mut h = Column::new(space, size, rows_per_chunk);
                h.extend(&vec![NONE; index(size)]);
                h
            })
        });
        Family {
            edges: EdgeTable::new(space, capacity, rows_per_chunk, listed),
            heads,
            wheel: DueWheel::new(first, horizon),
            kinds,
        }
    }

    /// Opens a contract, threading it on each listed side's list and on the wheel at its first due.
    #[clause("REP.3")]
    pub fn open(&mut self, row: R, due: Option<Day>) -> Slot {
        let ends = row.ends();
        for (side, end) in ends.iter().enumerate() {
            if self.kinds.get(side) != Some(&end.kind()) {
                violation!(clause = "REP.3", "a contract's side of another kind", side = side);
            }
        }
        let [h0, h1] = &mut self.heads;
        let hs = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(index(ends[0].slot().get()))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(index(ends[1].slot().get()))),
        ];
        let edge = self.edges.open(row, hs);
        if let Some(day) = due {
            self.wheel.schedule(edge.get(), day);
        }
        edge
    }

    /// Closes a contract, taking it off its sides' lists; its wheel entry is left, for its reader to skip.
    pub fn close(&mut self, edge: Slot) {
        let Some(row) = self.edges.row(edge) else {
            violation!(clause = "REP.3", "a contract closed that is not open", edge = edge.get());
        };
        let ends = row.ends();
        let [h0, h1] = &mut self.heads;
        let hs = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(index(ends[0].slot().get()))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(index(ends[1].slot().get()))),
        ];
        self.edges.close(edge, hs);
    }

    /// A party's contracts on a listed side.
    pub fn of(&self, side: usize, slot: Slot) -> impl Iterator<Item = Slot> + '_ {
        let head = self
            .heads
            .get(side)
            .and_then(Option::as_ref)
            .and_then(|h| h.slice().get(index(slot.get())).copied())
            .unwrap_or(NONE);
        self.edges.list(side, head)
    }
}

#[path = "store_tests.rs"]
mod tests;
