//! Today's flows grouped and settled (`phx_core::flows`, `phx_core::settle`): the design point's accounts at its banks,
//! a day's flows between them, grouped by payer range and settled against the books.

use std::collections::BTreeMap;

use phx_core::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};
use phx_core::settle::{Book, Books, Settle};
use phx_exec::Pool;
use phx_id::{PartyKey, Slot};
use phx_rand::uniform::below_u64;
use phx_rand::{Draws, Subject, SubjectTag};

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{BASE, count, day_of, index, slots, whole, wide};
use crate::measure::Measures;
use crate::{DayType, FinError};

/// The kinds of the fill: the accounts, the banks, the issuer.
const ACCOUNTS: u8 = 0;
const BANKS: u8 = 1;
const ISSUER: u8 = 2;
/// The world's netting ranges: 4 096 slots a range.
const RANGE_BITS: u32 = 12;
/// A day's flows are made in this many chunks, as a day's handlers make them on the pool.
const CHUNKS: usize = 64;
/// An account opens with up to this many cents, and a flow moves up to a tenth of it, so most settle and some fail.
const BALANCE_CENTS: u64 = 1_000_000;
const FLOW_CENTS: u64 = 100_000;

/// The books and the day's flows of each day type, made once so a measured day times only grouping and settling.
#[derive(Debug, Default)]
pub struct Flows {
    bank: Vec<u32>,
    balance: Vec<i64>,
    pending: Vec<i64>,
    held: Vec<i64>,
    facility: Vec<i64>,
    reserves: Vec<i64>,
    bank_zero: Vec<i64>,
    bank_held: Vec<i64>,
    bank_facility: Vec<i64>,
    deposits: Vec<i64>,
    closed: Vec<bool>,
    made: BTreeMap<DayType, Vec<Flow>>,
    bufs: FlowBufs,
    settle: Settle,
    streams: Option<Streams>,
}

fn account(slot: u64) -> Result<PartyKey, FinError> {
    Ok(PartyKey::new(ACCOUNTS, Slot::new(slots(slot)?)))
}

impl Flows {
    /// The accounts, each at a bank drawn for it with a balance drawn for it, and the banks' reserves and deposits.
    ///
    /// # Errors
    /// A design point without `[store] accounts` or `banks`.
    pub fn fill(&mut self, design: &Design, streams: &Streams) -> Result<u64, FinError> {
        let (n, banks) = (count(&design.store, "accounts", "store")?, count(&design.store, "banks", "store")?);
        let mut d = streams.draws("kept.flows", 0, 0);
        self.bank = (0..n).map(|_| slots(below_u64(&mut d, banks))).collect::<Result<_, _>>()?;
        self.balance = (0..n).map(|_| whole(below_u64(&mut d, BALANCE_CENTS))).collect::<Result<_, _>>()?;
        let len = index(n)?;
        (self.pending, self.held, self.facility) = (vec![0; len], vec![0; len], vec![0; len]);
        let nb = index(banks)?;
        self.deposits = vec![0; nb];
        for (b, v) in self.bank.iter().zip(&self.balance) {
            if let Some(dep) = self.deposits.get_mut(index(u64::from(*b))?) {
                *dep += v;
            }
        }
        // Reserves at the whole of each bank's deposits, so a bank is never short and only payers fail.
        self.reserves.clone_from(&self.deposits);
        (self.bank_zero, self.bank_held, self.bank_facility) = (vec![0; nb], vec![0; nb], vec![0; nb]);
        self.closed = vec![false; nb];
        self.streams = Some(*streams);
        Ok(n + banks)
    }

    fn make(&self, day: DayType, flows: u64) -> Result<Vec<Flow>, FinError> {
        let n = u64::try_from(self.balance.len()).map_err(|e| FinError(e.to_string()))?;
        let streams = self.streams.ok_or_else(|| FinError("flows made before the fill".to_owned()))?;
        let mut d = streams.draws("kept.flows", 1, day_of(day)?);
        (0..flows)
            .map(|i| {
                Ok(Flow {
                    payer: account(below_u64(&mut d, n))?,
                    payee: account(below_u64(&mut d, n))?,
                    amount: whole(below_u64(&mut d, FLOW_CENTS) + 1)?,
                    source: slots(i)?,
                    denomination: Denom::money(0),
                    reason: 0,
                    order: 0,
                })
            })
            .collect()
    }

    /// A day's flows grouped and settled: `flow` on an ordinary day, `flow_h` on the heaviest.
    ///
    /// # Errors
    /// Flows made before the fill.
    pub fn day(
        &mut self,
        day: DayType,
        counts: &BTreeMap<String, u64>,
        m: &mut Measures<'_>,
        pool: Option<&Pool>,
    ) -> Result<(), FinError> {
        // A day that settles no flows, as a closed day does not, runs none.
        let Some(&flows) = counts.get("flows") else { return Ok(()) };
        if !self.made.contains_key(&day) {
            let made = self.make(day, flows)?;
            self.made.insert(day, made);
        }
        let made = self.made.get(&day).map_or(&[][..], Vec::as_slice);
        self.bufs.reset(CHUNKS);
        let per = made.len().div_ceil(CHUNKS);
        if per > 0 {
            for (buf, part) in self.bufs.chunks_mut().iter_mut().zip(made.chunks(per)) {
                buf.extend_from_slice(part);
            }
        }
        let width = |n: usize| slots(wide(n));
        let high = [width(self.balance.len())?, width(self.reserves.len())?, 1];
        let ranges = Ranges::new(RANGE_BITS, &high);
        let op = if day == DayType::H { "flow_h" } else { "flow" };
        let streams = self.streams.ok_or_else(|| FinError("flows settled before the fill".to_owned()))?;
        let key = streams.key("kept.order");
        let lot = move |p: PartyKey| Draws::new(key, Subject::new(SubjectTag::Party, u64::from(p.word())), 0, 0);
        let (bufs, settle) = (&mut self.bufs, &mut self.settle);
        let mut books = Books {
            kinds: vec![
                Some(Book {
                    bank: &self.bank,
                    balance: &mut self.balance,
                    pending: &mut self.pending,
                    held: &mut self.held,
                    facility: &self.facility,
                    lines: None,
                }),
                Some(Book {
                    bank: &[],
                    balance: &mut self.reserves,
                    pending: &mut self.bank_zero,
                    held: &mut self.bank_held,
                    facility: &self.bank_facility,
                    lines: None,
                }),
                None,
            ],
            banks: BANKS,
            deposits: &mut self.deposits,
            closed: &self.closed,
            issuer: PartyKey::new(ISSUER, Slot::new(0)),
        };
        m.read(BASE, op, flows, || {
            bufs.group(pool, &ranges, Denom::money(0));
            let grouped = Grouped::new(&[&*bufs], &ranges);
            settle.settle(pool, &grouped, &ranges, &mut books, &lot)
        });
        Ok(())
    }

    /// A digest of the books, the same for any workers.
    #[must_use]
    pub fn digest(&self) -> u64 {
        let words = self.bank.iter().map(|b| u64::from(*b)).chain(self.balance.iter().map(|v| v.cast_unsigned()));
        words.fold(0, |a, w| phx_exec::mix::mix64(a ^ w))
    }

    /// The bytes the books and the made flows hold.
    #[must_use]
    pub fn bytes(&self) -> u64 {
        let words = self.balance.len() * 4 + self.reserves.len() * 5;
        let flows: usize = self.made.values().map(Vec::len).sum();
        wide(words * size_of::<i64>() + self.bank.len() * size_of::<u32>() + flows * size_of::<Flow>())
    }
}
