//! `-F settle`: the sweeps declared over the design point's accounts — each business day's `pending` sweep, moving
//! every account's pending amount into its balance, and every day's rolling slice of the audit's recount over its
//! cycle — run on saved cursors as chunk plans and counted in the sweep ledger.

use std::collections::BTreeMap;

use phx_exec::{Cursor, Pool, PoolSpec, SweepDecl, SweepLedger, SweepRun, Walk, When};
use phx_rand::uniform::below_u64;

use crate::design::Design;
use crate::fill::Streams;
use crate::kept::{count, day_of, index, wide};
use crate::measure::Measures;
use crate::{Bytes, DayType, Filled, FinBase, FinError};

/// The base the sweeps are measured under.
pub const BASE: &str = "settle";

/// The ledger's one store, the accounts, and its two slots: settlement's and the audit's.
const ACCOUNTS: usize = 0;
const SETTLE_SLOT: usize = 0;
const AUDIT_SLOT: usize = 1;
const SLOTS: usize = 2;

/// The `pending` sweep reads every account each business day: a cycle of one day.
const PENDING: SweepDecl =
    SweepDecl { name: "pending", store: ACCOUNTS, slot: SETTLE_SLOT, when: When::Rolling { cycle: 1 } };

/// Accounts a table chunk holds, as the ledger's accounts are laid out (4 096 of them, an L2's share).
const ROWS_PER_CHUNK: u32 = 1 << 12;

/// A row's declared cost in the plan's units: a sequential read and write of its 16 bytes.
const ROW_COST: u64 = 1;

/// The largest amount a drawn pending entry carries.
const AMOUNT: u64 = 1 << 20;

/// An account's row: its balance and the amount pending on it until the day's sweep.
type Row = [i64; 2];

/// The accounts, the pool, each sweep's cursor and buffers, the ledger, the streams, the day it is and what the days
/// read: the rows the sweeps settled and the most rows a day no declaration explained.
#[derive(Debug, Default)]
pub struct Settle {
    rows: Vec<Row>,
    pool: Option<Pool>,
    audit: Option<SweepDecl>,
    cursors: Option<[Cursor; 2]>,
    pending_run: SweepRun<(u64, i64)>,
    audit_run: SweepRun<i64>,
    ledger: SweepLedger,
    streams: Option<Streams>,
    today: u32,
    settled: u64,
    undeclared: u64,
    declared: u64,
}

impl Settle {
    /// The pending entries settled over the measured days, the same for any workers.
    #[must_use]
    pub fn settled(&self) -> u64 {
        self.settled
    }
}

impl FinBase for Settle {
    fn name(&self) -> &'static str {
        BASE
    }

    /// `[store] accounts` rows with no balance and nothing pending, and the audit's cycle of `[resolution]
    /// audit_cycle_days`.
    fn fill(&mut self, design: &Design, streams: &Streams) -> Result<Filled, FinError> {
        let accounts = count(&design.store, "accounts", "store")?;
        let cycle = design.resolution("audit_cycle_days")?;
        let cycle = cycle.as_integer().and_then(|c| u32::try_from(c).ok());
        let cycle =
            cycle.ok_or_else(|| FinError("`[resolution] audit_cycle_days` is not a count of days".to_owned()))?;
        self.audit =
            Some(SweepDecl { name: "audit", store: ACCOUNTS, slot: AUDIT_SLOT, when: When::Rolling { cycle } });
        self.rows = vec![[0; 2]; index(accounts)?];
        self.cursors = Some([Cursor::starting(0), Cursor::starting(0)]);
        self.ledger = SweepLedger::new(1, SLOTS);
        self.pool = Some(Pool::new(&PoolSpec::detect()).map_err(|e| FinError(e.0))?);
        self.streams = Some(*streams);
        Ok(Filled { rows: accounts })
    }

    /// The day's entries made pending — its applies on a business day, its pending payments on a closed one — then,
    /// on a business day, the `pending` sweep over every account, and every day the audit's slice.
    fn day(&mut self, day: DayType, counts: &BTreeMap<String, u64>, m: &mut Measures<'_>) -> Result<(), FinError> {
        let (Some(streams), Some(audit), Some([pending_at, audit_at])) =
            (self.streams, self.audit, self.cursors.as_mut())
        else {
            return Err(FinError("the sweeps measured before their fill".to_owned()));
        };
        let accounts = wide(self.rows.len());
        let high_water = u32::try_from(self.rows.len()).map_err(|e| FinError(e.to_string()))?;
        let walk = Walk { high_water, rows_per_chunk: ROWS_PER_CHUNK, cost_per_row: ROW_COST };
        let business = counts.contains_key("dues");
        let entries = counts.get(if business { "applies" } else { "pending" }).copied();
        let mut d = streams.draws(BASE, u64::from(self.today), day_of(day)?);
        for i in 0..entries.unwrap_or(0) {
            let at = index(below_u64(&mut d, accounts))?;
            let amount = i64::try_from(1 + i % AMOUNT).map_err(|e| FinError(e.to_string()))?;
            if let Some(row) = self.rows.get_mut(at) {
                row[1] += amount;
            }
        }
        let (pool, today, ledger) = (self.pool.as_ref(), self.today, &mut self.ledger);
        if business {
            let run = &mut self.pending_run;
            let rows = &mut self.rows;
            let chunks = m.read(BASE, "sweep", accounts, || {
                run.rolling_mut(pool, (&PENDING, pending_at, walk), today, (ledger, rows), |_, rows, out| {
                    // Summed in locals and stored once: a sum kept through the result's pointer is stored at
                    // every row.
                    let (mut entries, mut amount) = (0, 0);
                    for [balance, pending] in rows {
                        entries += u64::from(*pending != 0);
                        amount += *pending;
                        (*balance, *pending) = (*balance + *pending, 0);
                    }
                    *out = (entries, amount);
                })
                .to_vec()
            });
            self.settled += chunks.iter().map(|c| c.0).sum::<u64>();
        }
        let rows = &self.rows;
        let slice = high_water.div_ceil(match audit.when {
            When::Rolling { cycle } => cycle,
            When::OnReason => 1,
        });
        let run = &mut self.audit_run;
        let _ = m.read(BASE, "recount", u64::from(slice), || {
            let sums = run.rolling(pool, (&audit, audit_at, walk), today, ledger, |slots, out| {
                let places = usize::try_from(slots.start).ok().zip(usize::try_from(slots.end).ok());
                *out = places.and_then(|(from, to)| rows.get(from..to)).into_iter().flatten().map(|r| r[0]).sum();
            });
            sums.iter().sum::<i64>()
        });
        let visits = self.ledger.close_day();
        if visits.undeclared > self.undeclared {
            self.undeclared = visits.undeclared;
        }
        if visits.declared > self.declared {
            self.declared = visits.declared;
        }
        self.today += 1;
        Ok(())
    }

    fn bytes(&self) -> Bytes {
        Bytes { rows: wide(self.rows.len() * size_of::<Row>()), resident: phx_store::StoreStats::bytes(&self.ledger) }
    }

    /// The most rows a day's passes visited that no declaration explains, the most its declared sweeps read, and a
    /// cursor's bytes.
    fn figures(&self) -> Vec<(&'static str, f64)> {
        let real = |n: u64| n.to_string().parse::<f64>().ok();
        let mut out = Vec::new();
        if let Some(u) = real(self.undeclared) {
            out.push(("ledger_undeclared_rows", u));
        }
        if let Some(d) = real(self.declared) {
            out.push(("ledger_declared_rows", d));
        }
        if let Some(b) = real(wide(size_of::<Cursor>())) {
            out.push(("cursor_bytes", b));
        }
        out
    }
}
