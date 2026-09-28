//! The full-load bench: the finished world's cost on the phone before the finished world exists. It builds the core's
//! stores at the design point's sizes — every kind's parties with their columns and money, every family's contracts
//! between them with their list links and due days — and runs a simulated month through the core's kernels: hazards,
//! handlers over column slices spending their rules' declared arithmetic, purchases drawn from sellers' logit weights,
//! the dues the wheel hands each day, every flow netted and applied, the audit's identities, and full saves. Its
//! numbers are costs, never the world's.

use std::hint::black_box;
use std::time::Instant;

use phx_core::column_facts::RecordFacts;
use phx_core::flows::{Flow, FlowBufs, Grouped, Ranges};
use phx_core::wheel::DueWheel;
use phx_exec::{Clock, Pool, PoolSpec};
use phx_id::{Day, PartyKey, Slot};
use phx_num::MaybeI64;
use phx_rand::{AliasTable, Draws, Seed, Subject, SubjectTag, below_u64, stream_key};
use phx_store::edges::NONE;
use phx_store::{AddressSpace, Column, EdgeTable, Parties, SystemBacking};
use serde::Deserialize;

use crate::bench::{BenchHost, BenchLine};
use crate::json::Json;

/// The report section's layout.
const LOAD_VERSION: u64 = 4;
/// The budget: a turn's median and worst wall time, peak resident memory, a full save's time and two saves' bytes.
const MEDIAN_MS: u64 = 1_000;
const WORST_MS: u64 = 2_000;
const MEMORY_BYTES: u64 = 4_500_000_000;
const SAVE_MS: u64 = 5_000;
const TWO_SAVES_BYTES: u64 = 4 * GIB;
const MIB: u64 = 1 << 20;
const GIB: u64 = 1 << 30;
const KIB: u64 = 1 << 10;
const NS_PER_MS: u64 = 1_000_000;
const MILLION: u64 = 1_000_000;
const THOUSAND: u64 = 1_000;
/// The core's unit targets, in phone core-nanoseconds (plan, the restructure): a flow netted and applied, a due taken
/// and emitted, a handler row beyond its rule, a purchase drawn, a hazard hit.
const FLOW_NS: u64 = 25;
const DUE_NS: u64 = 20;
const ROW_NS: u64 = 40;
const CHOICE_NS: u64 = 50;
const HAZARD_NS: u64 = 100;
/// The share of the contracts the audit reads in full each day: its rolling slice.
const AUDIT_SLICES: u64 = 30;
/// Pieces a parallel kind of work is cut into, per worker.
const PIECES_PER_WORKER: usize = 4;
/// Days of the due wheel's buckets: more than a month, so every due of the month sits in a bucket.
const WHEEL_DAYS: u32 = 64;
/// A contract's next due after the one paid: a month on.
const MONTH_DAYS: u32 = 30;
/// The quarterly families have a third of their contracts due in any month.
const QUARTER_MONTHS: u64 = 3;
/// Words a save writes at a time.
const SAVE_WORDS: usize = 1 << 16;
/// Iterations timed to find what one iteration of a rule's stand-in arithmetic costs.
const CALIBRATION_ITERATIONS: u64 = 1 << 24;
/// A price's spread around its level, as a share, for the logit weights.
const PRICE_SPREAD: f64 = 0.2;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum DayType {
    Business,
    Closed,
    Heavy,
}

impl DayType {
    fn index(self) -> usize {
        match self {
            DayType::Business => 0,
            DayType::Closed => 1,
            DayType::Heavy => 2,
        }
    }

    fn name(self) -> &'static str {
        match self {
            DayType::Business => "business",
            DayType::Closed => "closed",
            DayType::Heavy => "heavy",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Kernel {
    Hazard,
    Handler,
    Choice,
    Dues,
    Settle,
    Audit,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Dues {
    None,
    Payday,
    Spread,
    Quarterly,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldSpec {
    persons: u64,
    work_num: u64,
    work_den: u64,
    range_bits: u32,
    rows_per_chunk: u32,
    regions: u32,
    products: u32,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct KindSpec {
    name: String,
    per_million: u64,
    bytes: u64,
    money: bool,
    line: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FamilySpec {
    name: String,
    per_million: u64,
    bytes: u64,
    listed: [bool; 2],
    payer: String,
    payee: String,
    dues: Dues,
    mean: i64,
    line: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoreSpec {
    name: String,
    per_million_bytes: u64,
    line: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Month {
    days: Vec<DayType>,
    saves: Vec<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkSpec {
    name: String,
    kernel: Kernel,
    table: Option<String>,
    payee: Option<String>,
    reads: Option<usize>,
    writes: Option<usize>,
    rule_ns: Option<u64>,
    flows: Option<u64>,
    per_million: [u64; 3],
    line: String,
}

/// The finished world's declared volumes.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Volumes {
    world: WorldSpec,
    #[serde(rename = "kind")]
    kinds: Vec<KindSpec>,
    #[serde(rename = "family")]
    families: Vec<FamilySpec>,
    #[serde(rename = "store")]
    stores: Vec<StoreSpec>,
    month: Month,
    #[serde(rename = "work")]
    works: Vec<WorkSpec>,
}

impl Volumes {
    /// A per-million count at the design point.
    fn at(&self, per_million: u64) -> u64 {
        (self.world.persons * per_million).div_ceil(MILLION)
    }

    /// A day's count of a kind of work, with the margin.
    fn work(&self, per_million: u64) -> u64 {
        self.at(per_million) * self.world.work_num / self.world.work_den
    }

    /// The flows of the heaviest day: every contract's due with its margin, and every kind of work's flows at its
    /// heavy count; a bound to reserve by, never a count of work.
    fn heavy_flows(&self, contracts: u64) -> u64 {
        let work: u64 = self
            .works
            .iter()
            .map(|w| self.work(w.per_million.get(2).copied().unwrap_or(0)) * w.flows.unwrap_or(0) / THOUSAND)
            .sum();
        contracts * self.world.work_num / self.world.work_den + work
    }

    fn kind(&self, name: &str) -> Result<u8, String> {
        self.kinds
            .iter()
            .position(|k| k.name == name)
            .and_then(|p| u8::try_from(p).ok())
            .ok_or_else(|| format!("no kind `{name}` among the volumes' kinds"))
    }
}

/// The monotonic clock, which the bench alone reads.
struct Mono(Instant);

impl Clock for Mono {
    fn now_ns(&self) -> u64 {
        u64::try_from(self.0.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }
}

fn proc_kib(file: &str, field: &str) -> Option<u64> {
    let text = std::fs::read_to_string(format!("/proc/self/{file}")).ok()?;
    let line = text.lines().find(|l| l.starts_with(field))?;
    let kib: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    kib.checked_mul(KIB)
}

fn show(host: &dyn BenchHost, name: &str, value: String, target: String, verdict: &str) {
    host.on_line(BenchLine {
        section: "full load".to_owned(),
        name: name.to_owned(),
        value,
        target,
        verdict: verdict.to_owned(),
        thermal_status: host.thermal_status(),
    });
}

fn draws(stream: u64, subject: u64, day: u32) -> Draws {
    Draws::new(stream_key(Seed::new(stream), "LOAD.bench"), Subject::new(SubjectTag::World, subject), day, 0)
}

fn to_usize(n: u64) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

fn to_u32(n: u64) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The `p`th of `n` equal pieces of `total`: its first unit and its number of units, the last taking what is left.
fn piece(total: u64, n: usize, p: usize) -> (u64, u64) {
    let (n, p) = (u64::try_from(n).unwrap_or(1), u64::try_from(p).unwrap_or(0));
    let each = total.div_ceil(n);
    let first = p * each;
    if first >= total {
        return (total, 0);
    }
    let rest = total - first;
    (first, if rest < each { rest } else { each })
}

/// A rule's arithmetic stood in for: `iterations` dependent multiply-adds on the row's values, so none is skipped.
fn spin(seed: f64, iterations: u64) -> f64 {
    let mut x = seed;
    for _ in 0..iterations {
        x = (x * 1.000_000_1 + 0.5) * 0.999_999_9;
    }
    x
}

/// Nanoseconds one iteration of the stand-in takes on this machine.
fn calibrate(clock: &Mono) -> f64 {
    let t0 = clock.now_ns();
    black_box(spin(black_box(1.0), CALIBRATION_ITERATIONS));
    phx_rand::float::from_u64(clock.now_ns() - t0) / phx_rand::float::from_u64(CALIBRATION_ITERATIONS)
}

/// A chunk's job: its index and rows of a column, and the flow buffer it fills.
type ChunkJob<'a, T> = ((usize, &'a mut [T]), &'a mut Vec<Flow>);

/// One kind's parties: their slots, their records of facts, row-major at `stride` words a party, and their money and
/// what is pending on it, in columns, where the kind holds money.
struct Kind {
    parties: Parties<SystemBacking>,
    records: Column<MaybeI64, SystemBacking>,
    stride: usize,
    money: Option<Column<i64, SystemBacking>>,
    pending: Option<Column<i64, SystemBacking>>,
    n: u32,
}

/// One family's contracts: the table, each contract's amount and next due day, its other columns, the list heads on
/// its parties' kinds, and its wheel.
struct Family {
    edges: EdgeTable<SystemBacking>,
    amount: Column<i64, SystemBacking>,
    due: Column<u32, SystemBacking>,
    others: Vec<Column<u64, SystemBacking>>,
    heads: [Option<Column<u32, SystemBacking>>; 2],
    wheel: DueWheel,
}

/// The stores the month runs over.
struct Load {
    pool: std::sync::Arc<Pool>,
    kinds: Vec<Kind>,
    families: Vec<Family>,
    held: Vec<Vec<u64>>,
    ranges: Ranges,
    /// Flows made since the last settlement: the closed days' carried into the next business day.
    pending: Vec<FlowBufs>,
    grouped: Grouped,
    due_today: Vec<u32>,
    /// The next day the wheels hand out.
    wheel_day: u32,
    ns_per_iteration: f64,
    tables: Vec<Option<(Vec<u32>, AliasTable)>>,
    tables_day: Option<u32>,
}

/// A day's count of each unit of work, for the costs a unit.
#[derive(Default, Clone, Copy)]
struct Units {
    flows: u64,
    dues: u64,
    rows: u64,
    choices: u64,
    hits: u64,
    shorts: u64,
}

fn pieces(pool: &Pool) -> usize {
    pool.workers() * PIECES_PER_WORKER
}

impl Load {
    /// Hazard hits: each draws its party's next event from its own stream and writes the day to one of its columns,
    /// the hits taken chunk by chunk on the pool.
    fn hazards(&mut self, kind: u8, count: u64, day: u32, rows_per_chunk: u32) -> u64 {
        let Some(k) = self.kinds.get_mut(usize::from(kind)) else { return 0 };
        let n = u64::from(k.n);
        let mut d = draws(4, 0, day);
        let mut hit: Vec<u32> = (0..count).map(|_| to_u32(below_u64(&mut d, n))).collect();
        hit.sort_unstable();
        let stride = k.stride;
        let rpc = to_usize(u64::from(rows_per_chunk));
        let jobs: Vec<(usize, &mut [MaybeI64])> = k.records.slice_mut().chunks_mut(rpc * stride).enumerate().collect();
        let hit = &hit;
        self.pool.for_each(jobs, |(c, rows)| {
            let lo = to_u32(u64::try_from(c * rpc).unwrap_or(0));
            let from = hit.partition_point(|s| *s < lo);
            let to = hit.partition_point(|s| *s < lo + rows_per_chunk);
            for slot in hit.get(from..to).unwrap_or(&[]) {
                let mut hd = draws(5, u64::from(*slot), day);
                let next = phx_rand::exponential(&mut hd, 1.0);
                if let Some(cell) = rows.get_mut(to_usize(u64::from(*slot - lo)) * stride) {
                    *cell = MaybeI64::present(phx_rand::float::floor_to_i64(next * 1e6).unwrap_or(0));
                }
            }
        });
        count
    }

    /// A handler over `count` rows of a kind: each row's declared facts read from its chunk's records, its
    /// rule's arithmetic spent, its writes made, and its flows emitted to parties of the payee kind.
    fn handler(
        &mut self,
        w: &WorkSpec,
        kind: u8,
        payee: u8,
        count: u64,
        day: u32,
        rows_per_chunk: u32,
    ) -> (u64, FlowBufs) {
        let iterations =
            phx_rand::float::floor_to_i64(phx_rand::float::from_u64(w.rule_ns.unwrap_or(0)) / self.ns_per_iteration)
                .and_then(|i| u64::try_from(i).ok())
                .unwrap_or(0);
        let flows_per_thousand = w.flows.unwrap_or(0);
        let payee_n = self.kinds.get(usize::from(payee)).map_or(1, |k| k.n);
        let Some(k) = self.kinds.get_mut(usize::from(kind)) else { return (0, FlowBufs::default()) };
        let n = u64::from(k.n);
        // The day's agenda stands in for the schedule: a row is due when its hash under the day falls below the share
        // of rows due, so each chunk finds its own rows, in slot order, as the agenda hands them.
        let threshold = if count >= n { u64::MAX } else { count * (u64::MAX / n) };
        let salt = phx_exec::mix64(u64::from(day) << 8 | u64::from(kind));
        let chunks = to_usize(u64::from(k.n.div_ceil(rows_per_chunk)));
        let k_n = k.n;
        let stride = k.stride;
        let declared = w.writes.unwrap_or(0);
        let writes = if declared < stride - 1 { declared } else { stride - 1 };
        let reads = w.reads.unwrap_or(0);
        // The declared facts' offsets in a record: the reads from its start, the writes at its end.
        let offsets: Vec<usize> =
            (0..reads).map(|r| r % (stride - writes)).chain((0..writes).map(|x| stride - writes + x)).collect();
        let writable: Vec<bool> = (0..reads).map(|_| false).chain((0..writes).map(|_| true)).collect();
        let rpc = to_usize(u64::from(rows_per_chunk));
        let mut bufs = FlowBufs::default();
        bufs.reset(chunks);
        let jobs: Vec<ChunkJob<'_, MaybeI64>> =
            k.records.slice_mut().chunks_mut(rpc * stride).enumerate().zip(bufs.chunks_mut().iter_mut()).collect();
        let (offsets, writable) = (&offsets, &writable);
        let visited: u64 = self
            .pool
            .map_items(jobs, |((c, records), buf)| {
                let first = to_u32(u64::try_from(c).unwrap_or(0)) * rows_per_chunk;
                let last = if first + rows_per_chunk < k_n { first + rows_per_chunk } else { k_n };
                let due: Vec<u32> =
                    (first..last).filter(|s| phx_exec::mix64(u64::from(*s) ^ salt) < threshold).collect();
                if due.is_empty() {
                    return 0;
                }
                let mut store = RecordFacts::new(Slot::new(first), stride, records, offsets, writable);
                for (i, s) in due.iter().enumerate() {
                    let slot = Slot::new(*s);
                    let mut acc = 0.0;
                    for r in 0..reads {
                        if let phx_num::Missing::Present(v) = store.read_at(r, slot) {
                            acc += phx_rand::float::from_i64(v);
                        }
                    }
                    let out = spin(acc, iterations);
                    for wi in 0..writes {
                        let v = phx_rand::float::floor_to_i64(out).unwrap_or(0);
                        store.write_at(reads + wi, slot, v);
                    }
                    // Thousandths of a flow a row: a row emits one each time the running count passes a thousand.
                    let made = (u64::try_from(i).unwrap_or(0) + 1) * flows_per_thousand / THOUSAND
                        - u64::try_from(i).unwrap_or(0) * flows_per_thousand / THOUSAND;
                    for _ in 0..made {
                        // Its counterparty is its own, a supplier or a payee it knows; any stands in for it here.
                        let to = PartyKey::new(
                            payee,
                            Slot::new(to_u32(phx_exec::mix64(u64::from(*s) ^ salt) % u64::from(payee_n))),
                        );
                        buf.push(Flow {
                            payer: PartyKey::new(kind, slot),
                            payee: to,
                            amount: phx_rand::float::floor_to_i64(out.abs() % 1e5).unwrap_or(0) + 1,
                            source: *s,
                            reason: 1,
                            denomination: 0,
                            order: 0,
                        });
                    }
                }
                u64::try_from(due.len()).unwrap_or(0)
            })
            .into_iter()
            .sum();
        (visited, bufs)
    }

    /// The day's sellers of each (region, product) and the alias table of their logit weights over posted prices: built
    /// once a day, as sellers post, and read by every buyer of the group.
    fn seller_tables(&mut self, seller: u8, v: &WorldSpec, day: u32) {
        if self.tables_day == Some(day) {
            return;
        }
        let Some(sellers) = self.kinds.get(usize::from(seller)) else { return };
        let groups = to_usize(u64::from(v.regions * v.products));
        let (prices, stride) = (sellers.records.slice(), sellers.stride);
        let mut members: Vec<Vec<u32>> = vec![Vec::new(); groups];
        for s in 0..sellers.n {
            if let Some(g) = members.get_mut(to_usize(u64::from(s % (v.regions * v.products)))) {
                g.push(s);
            }
        }
        let members = &members;
        self.tables = self.pool.map(groups, |g| {
            let list = members.get(g)?;
            if list.is_empty() {
                return None;
            }
            let weights: Vec<f64> = list
                .iter()
                .map(|s| {
                    let p = match prices.get(to_usize(u64::from(*s)) * stride).map(|x| x.get()) {
                        Some(phx_num::Missing::Present(p)) => phx_rand::float::from_i64(p % 1000),
                        _ => 0.0,
                    };
                    (-(p / 1000.0) / PRICE_SPREAD).exp()
                })
                .collect();
            Some((list.clone(), AliasTable::new(&weights)))
        });
        self.tables_day = Some(day);
    }

    /// Purchases: each buyer-product draws a seller among its region's sellers of the product by their logit weights
    /// over posted prices, and pays it; most also take goods. Buyers are taken group by group, so each group's table
    /// stays in cache while its buyers draw.
    fn choices(
        &mut self,
        (buyer, seller): (u8, u8),
        count: u64,
        flows_per_thousand: u64,
        v: &WorldSpec,
        day: u32,
    ) -> FlowBufs {
        self.seller_tables(seller, v, day);
        let groups = self.tables.len();
        let n_buyers = self.kinds.get(usize::from(buyer)).map_or(1, |k| k.n);
        let per_region = u64::from(n_buyers.div_ceil(v.regions));
        let mut bufs = FlowBufs::default();
        bufs.reset(groups);
        let jobs: Vec<(usize, &mut Vec<Flow>)> = bufs.chunks_mut().iter_mut().enumerate().collect();
        let tables = &self.tables;
        let (regions, products) = (u64::from(v.regions), u64::from(v.products));
        self.pool.for_each(jobs, |(g, buf)| {
            let Some(Some((list, table))) = tables.get(g) else { return };
            let (first, units) = piece(count, groups, g);
            let region = u64::try_from(g).unwrap_or(0) / products;
            let mut d = draws(8, u64::try_from(g).unwrap_or(0), day);
            buf.reserve(to_usize(units * flows_per_thousand / THOUSAND + 1));
            for u in 0..units {
                // The group's buyers are its region's: every `regions`th slot from the region's first.
                // The buyers come from the day's spending decisions in slot order; the group's are taken in turn.
                let b = region + regions * ((first + u) % per_region);
                if b >= u64::from(n_buyers) {
                    continue;
                }
                let Some(s) = list.get(table.draw(&mut d)) else { continue };
                let buyer_key = PartyKey::new(buyer, Slot::new(to_u32(b)));
                let seller_key = PartyKey::new(seller, Slot::new(*s));
                let amount = i64::try_from(phx_exec::mix64(b ^ u) % 5000).unwrap_or(0) + 1;
                let money = Flow {
                    payer: buyer_key,
                    payee: seller_key,
                    amount,
                    source: *s,
                    reason: 2,
                    denomination: 0,
                    order: 0,
                };
                buf.push(money);
                let made =
                    (first + u + 1) * flows_per_thousand / THOUSAND - (first + u) * flows_per_thousand / THOUSAND;
                for _ in 1..made {
                    buf.push(Flow { payer: seller_key, payee: buyer_key, denomination: 1, reason: 3, ..money });
                }
            }
        });
        bufs
    }

    /// Takes every wheel's dues from the last day handed out through `day`, emitting each due's flow and moving the
    /// contract to its next due a month on, chunk by chunk of the family's contracts on the pool; a margin's share of
    /// dues pays a second leg.
    fn dues(&mut self, day: u32, margin: (u64, u64), rows_per_chunk: u32) -> (u64, Vec<FlowBufs>) {
        let mut out = Vec::new();
        let mut total = 0_u64;
        let mut due = std::mem::take(&mut self.due_today);
        let from = self.wheel_day;
        let rpc = to_usize(u64::from(rows_per_chunk));
        for f in &mut self.families {
            for d in from..=day {
                f.wheel.take(Day::new(d), &mut due, Some(&self.pool));
                if due.is_empty() {
                    continue;
                }
                total += u64::try_from(due.len()).unwrap_or(0);
                let next = d + MONTH_DAYS;
                let chunks = f.due.len().div_ceil(rpc);
                let mut bufs = FlowBufs::default();
                bufs.reset(chunks);
                let (edges, amount) = (&f.edges, &f.amount);
                let list = &due;
                let jobs: Vec<ChunkJob<'_, u32>> =
                    f.due.slice_mut().chunks_mut(rpc).enumerate().zip(bufs.chunks_mut().iter_mut()).collect();
                self.pool.for_each(jobs, |((c, dues), buf)| {
                    let lo = to_u32(u64::try_from(c * rpc).unwrap_or(0));
                    let a = list.partition_point(|e| *e < lo);
                    let b = list.partition_point(|e| *e < lo + rows_per_chunk);
                    for (i, e) in list.get(a..b).unwrap_or(&[]).iter().enumerate() {
                        let edge = Slot::new(*e);
                        let (Some(from), Some(to), Some(amt)) =
                            (edges.end(edge, 0), edges.end(edge, 1), amount.get(edge))
                        else {
                            continue;
                        };
                        let flow = Flow {
                            payer: from,
                            payee: to,
                            amount: amt,
                            source: *e,
                            reason: 4,
                            denomination: 0,
                            order: 1,
                        };
                        buf.push(flow);
                        let i = u64::try_from(a + i).unwrap_or(0);
                        let extra = (i + 1) * margin.0 / margin.1 - i * margin.0 / margin.1;
                        for _ in 1..extra {
                            buf.push(Flow { amount: amt / 2 + 1, reason: 5, ..flow });
                        }
                        if let Some(cell) = dues.get_mut(to_usize(u64::from(*e - lo))) {
                            *cell = next;
                        }
                    }
                });
                for e in &due {
                    f.wheel.schedule(*e, Day::new(next));
                }
                out.push(bufs);
            }
        }
        self.due_today = due;
        self.wheel_day = day + 1;
        (total, out)
    }

    /// Nets every flow since the last netting by range and applies each range's nets to its parties' money, or, on a
    /// closed day, to what they have pending, which the next business day settles; returns the flows and the payers
    /// the debits leave below nothing.
    fn settle(&mut self, business: bool) -> (u64, u64) {
        let refs: Vec<&FlowBufs> = self.pending.iter().collect();
        self.grouped.group(Some(&self.pool), &refs, &self.ranges);
        let flows = u64::try_from(self.grouped.by_payer.items.len()).unwrap_or(0);
        let range = 1_usize << self.ranges.range_bits();
        let mut jobs: Vec<(usize, &mut [i64], &mut [i64])> = Vec::new();
        let mut at = 0_usize;
        for k in &mut self.kinds {
            let count = to_usize(u64::from(k.n)).div_ceil(range);
            if let (Some(m), Some(p)) = (k.money.as_mut(), k.pending.as_mut()) {
                for (i, (mc, pc)) in m.slice_mut().chunks_mut(range).zip(p.slice_mut().chunks_mut(range)).enumerate() {
                    jobs.push((at + i, mc, pc));
                }
            }
            at += count;
        }
        let (grouped, ranges) = (&self.grouped, &self.ranges);
        let shorts: u64 = self
            .pool
            .map_items(jobs, |(r, money, pending)| {
                if business {
                    for (m, p) in money.iter_mut().zip(pending.iter_mut()) {
                        *m += *p;
                        *p = 0;
                    }
                    grouped.apply(r, ranges, money)
                } else {
                    let _ = grouped.apply(r, ranges, pending);
                    0
                }
            })
            .into_iter()
            .sum();
        self.pending.clear();
        (flows, shorts)
    }

    /// The day's identities: each money kind's total, every settled flow's two sides, and a thirtieth of the contracts
    /// read in full.
    fn audit(&self, day: u32) -> i64 {
        let money: i64 =
            self.kinds.iter().filter_map(|k| k.money.as_ref()).map(|m| m.slice().iter().sum::<i64>()).sum();
        let sides: i64 = self.grouped.by_payer.items.iter().map(|f| f.amount).sum::<i64>()
            - self.grouped.by_payee.items.iter().map(|f| f.amount).sum::<i64>();
        let slice = u64::from(day) % AUDIT_SLICES;
        let contracts: i64 = self
            .families
            .iter()
            .map(|f| {
                let a = f.amount.slice();
                let each = a.len().div_ceil(to_usize(AUDIT_SLICES));
                a.iter().skip(to_usize(slice) * each).take(each).sum::<i64>()
            })
            .sum();
        money ^ sides ^ contracts
    }

    /// A full save: every kind's and family's columns and the held stores, each to its own file, compressed in frames
    /// on the pool; returns the bytes written.
    fn save(&self, dir: &std::path::Path) -> Result<u64, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut sources: Vec<Vec<&[u8]>> = Vec::new();
        for k in &self.kinds {
            let mut cols: Vec<&[u8]> = vec![phx_store::as_bytes(k.records.slice())];
            if let Some(m) = &k.money {
                cols.push(phx_store::as_bytes(m.slice()));
            }
            sources.push(cols);
        }
        for f in &self.families {
            let mut cols: Vec<&[u8]> = vec![
                phx_store::as_bytes(f.edges.ends(0)),
                phx_store::as_bytes(f.edges.ends(1)),
                phx_store::as_bytes(f.amount.slice()),
                phx_store::as_bytes(f.due.slice()),
            ];
            cols.extend(f.others.iter().map(|c| phx_store::as_bytes(c.slice())));
            cols.extend(f.heads.iter().flatten().map(|c| phx_store::as_bytes(c.slice())));
            sources.push(cols);
        }
        for h in &self.held {
            sources.push(vec![phx_store::as_bytes(h.as_slice())]);
        }
        let pool: &Pool = &self.pool;
        let written = pool.map(sources.len(), |i| {
            let path = dir.join(format!("store-{i}.zst"));
            let file = std::fs::File::create(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let mut sink = std::io::BufWriter::new(file);
            let compress = |frames: &[Vec<u8>]| {
                pool.map(frames.len(), |f| {
                    frames.get(f).map_or_else(|| Ok(Vec::new()), |frame| phx_store::save::compress_frame(frame))
                })
            };
            let mut w = phx_store::save::Writer::framed(&mut sink, &compress);
            for col in sources.get(i).map_or(&[][..], Vec::as_slice) {
                for chunk in col.chunks(SAVE_WORDS * size_of::<u64>()) {
                    w.bytes(chunk);
                }
            }
            let (bytes, _) = w.finish().map_err(|e| e.to_string())?;
            std::io::Write::flush(&mut sink).map_err(|e| e.to_string())?;
            Ok::<u64, String>(bytes)
        });
        written.into_iter().sum()
    }
}

/// A kind's parties at the design point, with random facts and money.
fn build_kind(space: &mut AddressSpace, pool: &Pool, k: &KindSpec, index: u8, n: u32, rows_per_chunk: u32) -> Kind {
    let mut parties = Parties::new(space, index, n, rows_per_chunk);
    for _ in 0..n {
        let _ = parties.begin();
    }
    let words_a_party = to_usize(k.bytes.div_ceil(u64::try_from(size_of::<i64>()).unwrap_or(1)));
    // A record holds at least a fact read and one written.
    let stride = if words_a_party < 2 { 2 } else { words_a_party };
    let words = u64::from(n) * u64::try_from(stride).unwrap_or(1);
    let mut records: Column<MaybeI64, SystemBacking> = Column::new(space, to_u32(words), rows_per_chunk);
    let np = pieces(pool);
    let values: Vec<MaybeI64> = pool
        .map(np, |p| {
            let (first, len) = piece(words, np, p);
            let mut d = draws(9, (u64::from(index) << 40) | first, 0);
            (0..len)
                .map(|_| MaybeI64::present(i64::try_from(below_u64(&mut d, 1 << 20)).unwrap_or(0)))
                .collect::<Vec<_>>()
        })
        .concat();
    records.extend(&values);
    let money = k.money.then(|| {
        let mut m: Column<i64, SystemBacking> = Column::new(space, n, rows_per_chunk);
        m.extend(&vec![1_000_000_i64; to_usize(u64::from(n))]);
        m
    });
    let pending = k.money.then(|| {
        let mut p: Column<i64, SystemBacking> = Column::new(space, n, rows_per_chunk);
        p.extend(&vec![0_i64; to_usize(u64::from(n))]);
        p
    });
    Kind { parties, records, stride, money, pending, n }
}

/// A family's contracts between random parties of its two kinds, each first due by its family's dues.
fn build_family(
    space: &mut AddressSpace,
    f: &FamilySpec,
    (kinds, sizes): ([u8; 2], [u32; 2]),
    n: u32,
    (rows_per_chunk, heavy, month): (u32, u32, u32),
    seed: u64,
) -> Family {
    let mut edges = EdgeTable::new(space, n, rows_per_chunk, f.listed);
    let mut heads: [Option<Column<u32, SystemBacking>>; 2] = [0, 1].map(|side| {
        f.listed.get(side).copied().unwrap_or(false).then(|| {
            let size = sizes.get(side).copied().unwrap_or(1);
            let mut h: Column<u32, SystemBacking> = Column::new(space, size, rows_per_chunk);
            h.extend(&vec![NONE; to_usize(u64::from(size))]);
            h
        })
    });
    let mut amount: Column<i64, SystemBacking> = Column::new(space, n, rows_per_chunk);
    let mut due: Column<u32, SystemBacking> = Column::new(space, n, rows_per_chunk);
    let mut wheel = DueWheel::new(Day::new(0), WHEEL_DAYS);
    let mut d = draws(10, seed, 0);
    for _ in 0..n {
        let ends: [PartyKey; 2] = [0, 1].map(|side| {
            let (kind, size) = (kinds.get(side).copied().unwrap_or(0), sizes.get(side).copied().unwrap_or(1));
            PartyKey::new(kind, Slot::new(to_u32(below_u64(&mut d, u64::from(size)))))
        });
        let [h0, h1] = &mut heads;
        let hs: [Option<&mut u32>; 2] = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[0].slot().get())))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[1].slot().get())))),
        ];
        let edge = edges.open(ends, hs);
        let a = f.mean / 2 + i64::try_from(below_u64(&mut d, u64::try_from(f.mean).unwrap_or(0) + 1)).unwrap_or(0);
        amount.put(edge, a);
        let first = match f.dues {
            Dues::None => None,
            Dues::Payday => Some(heavy),
            Dues::Spread => Some(to_u32(below_u64(&mut d, u64::from(month)))),
            Dues::Quarterly => {
                (below_u64(&mut d, QUARTER_MONTHS) == 0).then(|| to_u32(below_u64(&mut d, u64::from(month))))
            }
        };
        due.put(edge, first.unwrap_or(u32::MAX));
        if let Some(day) = first {
            wheel.schedule(edge.get(), Day::new(day));
        }
    }
    let words = to_usize(if f.bytes > 12 { (f.bytes - 12).div_ceil(8) } else { 0 });
    let others = (0..words)
        .map(|_| {
            let mut c: Column<u64, SystemBacking> = Column::new(space, n, rows_per_chunk);
            c.extend(&vec![0_u64; to_usize(u64::from(n))]);
            c
        })
        .collect();
    Family { edges, amount, due, others, heads, wheel }
}

/// Builds the stores: every kind, every family, and the held stores.
fn build(v: &Volumes, host: &dyn BenchHost, clock: &Mono) -> Result<Load, String> {
    let pool = std::sync::Arc::new(Pool::new(&PoolSpec::detect()).map_err(|e| e.0)?);
    let mut space = AddressSpace::empty();
    let rpc = v.world.rows_per_chunk;
    let mut kinds = Vec::with_capacity(v.kinds.len());
    for (i, k) in v.kinds.iter().enumerate() {
        let n = to_u32(v.at(k.per_million));
        show(host, "building", format!("{} {}", n, k.name), String::new(), "");
        kinds.push(build_kind(&mut space, &pool, k, u8::try_from(i).unwrap_or(u8::MAX), n, rpc));
    }
    let days = u32::try_from(v.month.days.len()).unwrap_or(0);
    let heavy =
        v.month.days.iter().position(|d| *d == DayType::Heavy).map_or(0, |p| to_u32(u64::try_from(p).unwrap_or(0)));
    let mut families = Vec::with_capacity(v.families.len());
    for (i, f) in v.families.iter().enumerate() {
        let n = to_u32(v.at(f.per_million));
        show(host, "building", format!("{} {} contracts", n, f.name), String::new(), "");
        let ks = [v.kind(&f.payer)?, v.kind(&f.payee)?];
        let sizes = ks.map(|k| kinds.get(usize::from(k)).map_or(1, |x: &Kind| x.n));
        families.push(build_family(&mut space, f, (ks, sizes), n, (rpc, heavy, days), u64::try_from(i).unwrap_or(0)));
    }
    let held = v
        .stores
        .iter()
        .map(|s| {
            let words = to_usize(v.at(s.per_million_bytes) / u64::try_from(size_of::<u64>()).unwrap_or(1));
            let np = pieces(&pool);
            pool.map(np, |p| {
                let (first, len) = piece(u64::try_from(words).unwrap_or(0), np, p);
                (first..first + len).map(|i| phx_exec::mix64(i) & ((1 << 25) - 1)).collect::<Vec<u64>>()
            })
            .concat()
        })
        .collect();
    let high: Vec<u32> = kinds.iter().map(|k| k.n).collect();
    let ranges = Ranges::new(v.world.range_bits, &high);
    // The day's grouped flows are reserved at the heaviest day's declared count: address space, touched only as used.
    let mut grouped = Grouped::default();
    let heavy = v.heavy_flows(families.iter().map(|f| u64::try_from(f.due.len()).unwrap_or(0)).sum());
    grouped.by_payer.items.reserve(to_usize(heavy));
    grouped.by_payee.items.reserve(to_usize(heavy));
    let ns_per_iteration = calibrate(clock);
    Ok(Load {
        pool,
        kinds,
        families,
        held,
        ranges,
        pending: Vec::new(),
        grouped,
        due_today: Vec::new(),
        wheel_day: 0,
        ns_per_iteration,
        tables: Vec::new(),
        tables_day: None,
    })
}

/// What one day's work took, kind by kind, and its units.
struct DayRecord {
    index: usize,
    kind: DayType,
    walls: Vec<(String, u64, u64)>,
    total_ns: u64,
    units: Units,
}

/// The month's turns: each business or heavy day closes a turn holding the closed days before it.
fn turns(days: &[DayRecord]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut running = 0_u64;
    for d in days {
        running += d.total_ns;
        if d.kind != DayType::Closed {
            out.push(running);
            running = 0;
        }
    }
    out
}

fn median(values: &[u64]) -> Option<u64> {
    let mut v = values.to_vec();
    v.sort_unstable();
    v.get(v.len() / 2).copied()
}

fn greatest(values: &[u64]) -> Option<u64> {
    values.iter().copied().reduce(|a, b| if b > a { b } else { a })
}

fn verdict(ok: bool) -> &'static str {
    if ok { "met" } else { "missed" }
}

/// One day's work, each kind at its count for the day's type, with the margin; returns each kind's wall time and
/// units, and the day's units.
fn run_day(load: &mut Load, v: &Volumes, i: usize, kind: DayType, clock: &Mono) -> Result<DayRecord, String> {
    let day = to_u32(u64::try_from(i).unwrap_or(0));
    let mut units = Units::default();
    let mut walls = Vec::with_capacity(v.works.len());
    let day_start = clock.now_ns();
    for w in &v.works {
        let per_million = w.per_million.get(kind.index()).copied().unwrap_or(0);
        let t0 = clock.now_ns();
        let mut done = 0_u64;
        if per_million > 0 {
            let count = v.work(per_million);
            let table = w.table.as_deref().map(|t| v.kind(t)).transpose()?;
            match w.kernel {
                Kernel::Hazard => {
                    done = load.hazards(table.unwrap_or(0), count, day, v.world.rows_per_chunk);
                    units.hits += done;
                }
                Kernel::Handler => {
                    let payee = match w.payee.as_deref() {
                        Some(p) => v.kind(p)?,
                        None => table.unwrap_or(0),
                    };
                    let (rows, bufs) = load.handler(w, table.unwrap_or(0), payee, count, day, v.world.rows_per_chunk);
                    done = rows;
                    units.rows += rows;
                    load.pending.push(bufs);
                }
                Kernel::Choice => {
                    let sellers = v.kind("firm")?;
                    let bufs =
                        load.choices((table.unwrap_or(0), sellers), count, w.flows.unwrap_or(THOUSAND), &v.world, day);
                    done = count;
                    units.choices += count;
                    load.pending.push(bufs);
                }
                Kernel::Dues => {
                    let (dues, bufs) = load.dues(day, (v.world.work_num, v.world.work_den), v.world.rows_per_chunk);
                    done = dues;
                    units.dues += dues;
                    load.pending.extend(bufs);
                }
                Kernel::Settle => {
                    let (flows, shorts) = load.settle(kind != DayType::Closed);
                    done = flows;
                    units.flows += flows;
                    units.shorts += shorts;
                }
                Kernel::Audit => {
                    black_box(load.audit(day));
                    done = 1;
                }
            }
        }
        walls.push((w.name.clone(), clock.now_ns() - t0, done));
    }
    for k in &mut load.kinds {
        k.parties.close_day();
    }
    Ok(DayRecord { index: i, kind, walls, total_ns: clock.now_ns() - day_start, units })
}

/// The full-load bench over the volumes at `volumes_path`, saves written to and removed from `save_dir`, its report
/// written to `report_path` and returned.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, a save that cannot be written, or a report that cannot.
pub fn run(host: &dyn BenchHost, volumes_path: &str, save_dir: &str, report_path: &str) -> Result<String, String> {
    let text = measure(host, volumes_path, save_dir)?.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show(host, "report", report_path.to_owned(), String::new(), "");
    Ok(text)
}

/// The full-load bench's month, each day and save shown as it completes: the load section of the device report.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, or a save that cannot be written.
pub fn measure(host: &dyn BenchHost, volumes_path: &str, save_dir: &str) -> Result<Json, String> {
    let clock = Mono(Instant::now());
    let text = std::fs::read_to_string(volumes_path).map_err(|e| format!("{volumes_path}: {e}"))?;
    let v: Volumes = toml::from_str(&text).map_err(|e| format!("{volumes_path}: {e}"))?;
    let started = clock.now_ns();
    let mut load = build(&v, host, &clock)?;
    let built_ms = (clock.now_ns() - started) / NS_PER_MS;
    let built_peak = proc_kib("status", "VmHWM:");
    show(host, "built", format!("{built_ms} ms, peak {} MiB", built_peak.unwrap_or(0) / MIB), String::new(), "");
    let workers = u64::try_from(load.pool.workers()).unwrap_or(1);
    let mut records: Vec<DayRecord> = Vec::new();
    let mut saves: Vec<(usize, u64, u64)> = Vec::new();
    for (i, kind) in v.month.days.iter().enumerate() {
        let r = run_day(&mut load, &v, i, *kind, &clock)?;
        let peak = proc_kib("status", "VmHWM:").unwrap_or(0) / MIB;
        let heaviest: Vec<String> = {
            let mut w: Vec<&(String, u64, u64)> = r.walls.iter().collect();
            w.sort_by_key(|x| std::cmp::Reverse(x.1));
            w.iter().take(4).map(|(n, ns, _)| format!("{n} {} ms", ns / NS_PER_MS)).collect()
        };
        let name = format!("day {} ({})", i + 1, kind.name());
        show(
            host,
            &name,
            format!("{} ms, peak {peak} MiB; {}", r.total_ns / NS_PER_MS, heaviest.join(", ")),
            String::new(),
            "",
        );
        records.push(r);
        if v.month.saves.contains(&i) {
            let t0 = clock.now_ns();
            let path = std::path::Path::new(save_dir).join(format!("load-save-{i}"));
            let bytes = load.save(&path)?;
            let ms = (clock.now_ns() - t0) / NS_PER_MS;
            std::fs::remove_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let name = format!("save after day {}", i + 1);
            show(
                host,
                &name,
                format!("{ms} ms, {} MiB", bytes / MIB),
                format!("≤ {SAVE_MS} ms"),
                verdict(ms <= SAVE_MS),
            );
            saves.push((i + 1, ms, bytes));
        }
    }
    let unit_costs = unit_costs(&records, workers);
    for (name, ns, target) in &unit_costs {
        show(host, name, format!("{ns} core-ns"), format!("≤ {target} core-ns"), verdict(ns <= target));
    }
    let turn_ms: Vec<u64> = turns(&records).iter().map(|ns| ns / NS_PER_MS).collect();
    let (middle, worst) = (median(&turn_ms), greatest(&turn_ms));
    let (peak, pss) = (proc_kib("status", "VmHWM:"), proc_kib("smaps_rollup", "Pss:"));
    let two_saves: u64 = saves.iter().rev().take(2).map(|(_, _, b)| b).sum();
    let show_ms = |m: Option<u64>| m.map_or("none".to_owned(), |m| format!("{m} ms"));
    show(
        host,
        "median turn",
        show_ms(middle),
        format!("≤ {MEDIAN_MS} ms"),
        verdict(middle.is_some_and(|m| m <= MEDIAN_MS)),
    );
    show(host, "worst turn", show_ms(worst), format!("≤ {WORST_MS} ms"), verdict(worst.is_some_and(|m| m <= WORST_MS)));
    let peak_text = peak.map_or("unread".to_owned(), |p| format!("{} MiB", p / MIB));
    show(
        host,
        "peak resident",
        peak_text,
        format!("≤ {MEMORY_BYTES} bytes"),
        verdict(peak.is_some_and(|p| p <= MEMORY_BYTES)),
    );
    show(
        host,
        "two saves",
        format!("{} MiB", two_saves / MIB),
        format!("≤ {} MiB", TWO_SAVES_BYTES / MIB),
        verdict(two_saves <= TWO_SAVES_BYTES),
    );
    let report =
        report_json(&v, &records, (built_ms, built_peak), (&turn_ms, middle, worst), (peak, pss), &saves, &unit_costs);
    drop(load);
    Ok(report)
}

/// Each unit's cost over the month, in core-nanoseconds: the wall time its kinds of work took times the workers, over
/// the units they did, with its target.
fn unit_costs(records: &[DayRecord], workers: u64) -> Vec<(String, u64, u64)> {
    let mut sums: Vec<(&str, u64, u64, u64)> = vec![
        ("a flow netted and applied", 0, 0, FLOW_NS),
        ("a due taken and emitted", 0, 0, DUE_NS),
        ("a handler row, its rule included", 0, 0, ROW_NS),
        ("a purchase drawn", 0, 0, CHOICE_NS),
        ("a hazard hit", 0, 0, HAZARD_NS),
    ];
    for r in records {
        for (name, ns, done) in &r.walls {
            let which = if name == "settlement" {
                0
            } else if name == "dues" {
                1
            } else if name.contains("purchases") {
                3
            } else if name.starts_with("hazards") {
                4
            } else if name == "audit" {
                continue;
            } else {
                2
            };
            if let Some(s) = sums.get_mut(which) {
                s.1 += ns;
                s.2 += done;
            }
        }
    }
    sums.into_iter()
        .map(|(n, ns, done, target)| (n.to_owned(), (ns * workers).checked_div(done).unwrap_or(0), target))
        .collect()
}

/// The load section of the device report.
fn report_json(
    v: &Volumes,
    records: &[DayRecord],
    (built_ms, built_peak): (u64, Option<u64>),
    (turn_ms, middle, worst): (&[u64], Option<u64>, Option<u64>),
    (peak, pss): (Option<u64>, Option<u64>),
    saves: &[(usize, u64, u64)],
    unit_costs: &[(String, u64, u64)],
) -> Json {
    let count = |n: usize| Json::UInt(u64::try_from(n).unwrap_or(u64::MAX));
    let days = records.iter().map(|r| {
        let works = r.walls.iter().map(|(n, ns, done)| {
            Json::obj([
                ("work", Json::str(n.clone())),
                ("ms", Json::UInt(ns / NS_PER_MS)),
                ("units", Json::UInt(*done)),
            ])
        });
        Json::obj([
            ("day", count(r.index + 1)),
            ("type", Json::str(r.kind.name())),
            ("ms", Json::UInt(r.total_ns / NS_PER_MS)),
            ("flows", Json::UInt(r.units.flows)),
            ("dues", Json::UInt(r.units.dues)),
            ("rows", Json::UInt(r.units.rows)),
            ("choices", Json::UInt(r.units.choices)),
            ("hits", Json::UInt(r.units.hits)),
            ("short_payers", Json::UInt(r.units.shorts)),
            ("works", Json::Array(works.collect())),
        ])
    });
    let saves = saves
        .iter()
        .map(|(d, ms, b)| Json::obj([("after_day", count(*d)), ("ms", Json::UInt(*ms)), ("bytes", Json::UInt(*b))]));
    let units = unit_costs.iter().map(|(n, ns, t)| {
        Json::obj([("unit", Json::str(n.clone())), ("core_ns", Json::UInt(*ns)), ("target_core_ns", Json::UInt(*t))])
    });
    let lines = v
        .kinds
        .iter()
        .map(|k| (k.name.clone(), k.line.clone()))
        .chain(v.families.iter().map(|f| (f.name.clone(), f.line.clone())))
        .chain(v.stores.iter().map(|s| (s.name.clone(), s.line.clone())))
        .chain(v.works.iter().map(|w| (w.name.clone(), w.line.clone())))
        .map(|(n, l)| Json::obj([("name", Json::str(n)), ("line", Json::str(l))]));
    Json::obj([
        ("load_version", Json::UInt(LOAD_VERSION)),
        ("commit", Json::str(option_env!("PHX_COMMIT").unwrap_or("unknown"))),
        ("persons", Json::UInt(v.world.persons)),
        ("work_margin", Json::str(format!("{}/{}", v.world.work_num, v.world.work_den))),
        ("built_ms", Json::UInt(built_ms)),
        ("built_vm_hwm_bytes", Json::opt(built_peak, Json::UInt)),
        ("days", Json::Array(days.collect())),
        ("turn_ms", Json::Array(turn_ms.iter().map(|m| Json::UInt(*m)).collect())),
        ("median_turn_ms", Json::opt(middle, Json::UInt)),
        ("worst_turn_ms", Json::opt(worst, Json::UInt)),
        ("vm_hwm_bytes", Json::opt(peak, Json::UInt)),
        ("pss_bytes", Json::opt(pss, Json::UInt)),
        ("saves", Json::Array(saves.collect())),
        ("unit_costs", Json::Array(units.collect())),
        (
            "criteria",
            Json::obj([
                ("median_turn_ms", Json::UInt(MEDIAN_MS)),
                ("worst_turn_ms", Json::UInt(WORST_MS)),
                ("memory_bytes", Json::UInt(MEMORY_BYTES)),
                ("save_ms", Json::UInt(SAVE_MS)),
                ("two_saves_bytes", Json::UInt(TWO_SAVES_BYTES)),
            ]),
        ),
        ("lines", Json::Array(lines.collect())),
    ])
}

/// The app's entry: runs the full-load bench on the calling thread, which must not be the interface's.
#[uniffi::export]
#[expect(clippy::needless_pass_by_value, reason = "the foreign interface hands over owned values")]
pub fn run_load(
    host: std::sync::Arc<dyn BenchHost>,
    volumes_path: String,
    save_dir: String,
    report_path: String,
) -> String {
    crate::stopped::watch(&report_path);
    match run(host.as_ref(), &volumes_path, &save_dir, &report_path) {
        Ok(report) => report,
        Err(error) => {
            show(host.as_ref(), "the full-load bench stopped", error.clone(), String::new(), "");
            error
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Sender, channel};

    use super::{BenchHost, BenchLine, run};

    struct Host(Sender<BenchLine>);

    impl BenchHost for Host {
        fn thermal_status(&self) -> i32 {
            0
        }

        fn on_line(&self, line: BenchLine) {
            self.0.send(line).unwrap();
        }
    }

    /// The finished world's volumes at a design point of twenty thousand persons, everything else kept.
    fn small() -> String {
        let full = include_str!("../../../../perf/load/volumes.toml");
        let mut table: toml::Table = toml::from_str(full).unwrap();
        let world = table.get_mut("world").unwrap().as_table_mut().unwrap();
        world.insert("persons".to_owned(), 20_000.into());
        toml::to_string(&table).unwrap()
    }

    /// The full-load bench at the design point, each line appended to `phx-load-full-lines.txt` in the system's
    /// temporary directory as it comes, the report and saves beside it.
    #[test]
    #[ignore = "the full-load bench at the design point on the build machine: many minutes, run by hand, release profile"]
    fn load_at_full_volumes() {
        use std::io::Write;
        struct Lines(std::fs::File);
        impl BenchHost for Lines {
            fn thermal_status(&self) -> i32 {
                0
            }

            fn on_line(&self, l: BenchLine) {
                let mut f = &self.0;
                writeln!(f, "{} · {} · {} {}", l.name, l.value, l.target, l.verdict).unwrap();
                f.flush().unwrap();
            }
        }
        let dir = std::env::temp_dir();
        let lines = std::fs::File::create(dir.join("phx-load-full-lines.txt")).unwrap();
        let volumes = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../perf/load/volumes.toml");
        let report = dir.join("phx-load-full-report.json");
        run(&Lines(lines), volumes, dir.to_str().unwrap(), report.to_str().unwrap()).unwrap();
    }

    #[test]
    fn load_runs_end_to_end() {
        let dir = std::env::temp_dir().join("phx-load-small");
        std::fs::create_dir_all(&dir).unwrap();
        let volumes = dir.join("phx-load-volumes.toml");
        std::fs::write(&volumes, small()).unwrap();
        let (tx, rx) = channel();
        let report = dir.join("phx-load-report.json");
        let text = run(&Host(tx), volumes.to_str().unwrap(), dir.to_str().unwrap(), report.to_str().unwrap()).unwrap();
        let lines: Vec<BenchLine> = rx.try_iter().collect();
        assert!(text.contains("\"median_turn_ms\"") && lines.iter().any(|l| l.name == "worst turn"));
        assert!(text.contains("\"unit_costs\""));
    }
}
