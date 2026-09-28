//! The full-load bench: the finished world's cost on the phone before the finished world exists. It builds the core's
//! stores at the design point's sizes — every kind's parties with their records and money, every family's contracts
//! between them with their list links and due days — and runs a simulated month through the core's kernels: hazards,
//! handlers over the day's agenda spending their rules' declared arithmetic, purchases each drawn from the buyer's own
//! taste stream over its sellers' logit weights, the dues the wheel hands each day read from their contracts, contracts
//! opened and closed and parties begun and ended, every flow netted and applied, the audit's identities, and full
//! saves. Its numbers are costs, never the world's.

use std::hint::black_box;
use std::time::Instant;

use phx_core::column_facts::{Layout, RecordFacts};
use phx_core::flows::{Denom, Flow, FlowBufs, Grouped, Ranges};
use phx_core::settle::{AT_ISSUER, Book, Books, Settle};
use phx_core::wheel::DueWheel;
use phx_exec::{Clock, Pool, PoolSpec, mix64};
use phx_id::{Day, PartyId, PartyKey, Slot};
use phx_num::MaybeI64;
use phx_rand::{AliasTable, Draws, Seed, StreamKey, Subject, SubjectTag, below_u64, stream_key};
use phx_store::edges::NONE;
use phx_store::{AddressSpace, Column, EdgeTable, Parties, SystemBacking};
use serde::Deserialize;

use crate::bench::{BenchHost, BenchLine};
use crate::json::Json;

/// The report section's layout.
const LOAD_VERSION: u64 = 5;
/// The budget: a turn's median and worst wall time on the phone, peak resident memory, a full save's time and two
/// saves' bytes.
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
/// The phone's sustained parallel speed: three core-seconds a second. A day's time on the phone is its CPU time here
/// over three, at one build-machine core-second a phone core-second until the device's probe measures the ratio, and
/// never less than its wall time here.
const PHONE_CORES: u64 = 3;
/// The core's unit targets, in phone core-nanoseconds: a flow netted and applied, a due taken, read and emitted, a
/// handler row beyond its rule's arithmetic, a purchase drawn, a hazard hit, a contract opened or closed; the audit's
/// identities a million persons; resident bytes a person at the heaviest day.
const FLOW_NS: u64 = 25;
const DUE_NS: u64 = 20;
const ROW_NS: u64 = 40;
const CHOICE_NS: u64 = 50;
const HAZARD_NS: u64 = 100;
const TURNOVER_NS: u64 = 100;
const AUDIT_MS_PER_MILLION: u64 = 3;
const BYTES_PER_PERSON: u64 = 800;
/// The share of the contracts the audit reads in full each day: its rolling slice.
const AUDIT_SLICES: u64 = 30;
/// Days of the due wheel's buckets: two months, so a quarterly due drawn beyond it waits in the far list.
const WHEEL_DAYS: u32 = 64;
/// A contract's next due after the one paid: a month on; a quarterly one's first due falls within a quarter.
const MONTH_DAYS: u32 = 30;
const QUARTER_DAYS: u64 = 91;
/// Parts per million of a contract's balance its due adds, as a rate on it would.
const RATE_PPM: i64 = 4_000;
/// A contract's balance stand-in, at most this many smallest units.
const BALANCE_SPAN: u64 = 1 << 30;
/// Words a save writes at a time.
const SAVE_WORDS: usize = 1 << 16;
/// Iterations timed to find what one iteration of a rule's stand-in arithmetic costs.
const CALIBRATION_ITERATIONS: u64 = 1 << 24;
/// A price's spread around its level, as a share, for the logit weights.
const PRICE_SPREAD: f64 = 0.2;
/// A tenth of the day's contract turnover ends and begins parties.
const PARTY_SHARE: u64 = 10;

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
    Turnover,
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

/// The process's minor page faults so far.
fn process_times() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let fields: Vec<&str> = stat.rsplit_once(')')?.1.split_whitespace().collect();
    // After the name the fields run from the third: minor faults are the tenth.
    fields.get(10 - 3)?.parse().ok()
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

fn index_u64(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// The `p`th of `n` equal pieces of `total`: its first unit and its number of units, the last taking what is left.
fn piece(total: u64, n: usize, p: usize) -> (u64, u64) {
    let (n, p) = (index_u64(n), index_u64(p));
    let each = total.div_ceil(n);
    let first = p * each;
    if first >= total {
        return (total, 0);
    }
    let rest = total - first;
    (first, if rest < each { rest } else { each })
}

/// A rule's arithmetic stood in for: `iterations` dependent multiplications and additions on the row's values.
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

/// One kind's parties: their slots, their records of facts, row-major at `stride` words a party, and their money and
/// what is pending on it, in columns, where the kind holds money.
struct Kind {
    parties: Parties<SystemBacking>,
    records: Column<MaybeI64, SystemBacking>,
    stride: usize,
    money: Option<Column<i64, SystemBacking>>,
    pending: Option<Column<i64, SystemBacking>>,
    accounts: Option<Accounts>,
    n: u32,
}

/// Where a kind's accounts are held, what each holds through closed banks, and the facility each is granted.
struct Accounts {
    bank: Column<u32, SystemBacking>,
    held: Column<i64, SystemBacking>,
    facility: Column<i64, SystemBacking>,
}

impl Kind {
    /// The kind's accounts as the settlement reads them, if it holds money.
    fn book(&mut self) -> Option<Book<'_>> {
        let (money, pending, a) = (self.money.as_mut()?, self.pending.as_mut()?, self.accounts.as_mut()?);
        let Accounts { bank, held, facility } = a;
        Some(Book {
            bank: bank.slice(),
            balance: money.slice_mut(),
            pending: pending.slice_mut(),
            held: held.slice_mut(),
            facility: facility.slice(),
        })
    }
}

/// One family's contracts: the table, each contract's amount, balance and next due day, its other columns, the list
/// heads on its parties' kinds, its wheel and its parties' kinds.
struct Family {
    edges: EdgeTable<SystemBacking>,
    amount: Column<i64, SystemBacking>,
    balance: Column<i64, SystemBacking>,
    due: Column<u32, SystemBacking>,
    others: Vec<Column<u64, SystemBacking>>,
    heads: [Option<Column<u32, SystemBacking>>; 2],
    wheel: DueWheel,
    kinds: [u8; 2],
    mean: i64,
}

/// A handler's agenda: its kind's rows by the phase of their schedule, each bucket in slot order; a day's rows are its
/// bucket's, as the schedule hands them.
struct Agenda {
    buckets: Vec<Vec<u32>>,
    /// Passes a row makes a day when the day asks more of the kind than it has rows.
    passes: u64,
}

/// The stores the month runs over.
struct Load {
    pool: std::sync::Arc<Pool>,
    kinds: Vec<Kind>,
    families: Vec<Family>,
    held: Vec<Vec<u64>>,
    ranges: Ranges,
    /// A flow buffer for each kind of work and each family's dues, kept across days so a day fills what the heaviest
    /// sized; whether each holds flows not yet settled.
    bufs: Vec<FlowBufs>,
    filled: Vec<bool>,
    grouped: Grouped,
    settlement: Settle,
    /// The banks' kind, the banks closed, the currency's issuer and the stream payment orders' lots are drawn from.
    bank_kind: u8,
    closed: Vec<bool>,
    issuer: PartyKey,
    lot: StreamKey,
    due_today: Vec<u32>,
    /// The next day the wheels hand out.
    wheel_day: u32,
    ns_per_iteration: f64,
    tables: Vec<Option<(Vec<u32>, AliasTable)>>,
    tables_day: Option<u32>,
    agendas: Vec<Option<Agenda>>,
    taste: StreamKey,
    /// The banks' reserves and what they have pending together, which only the issuer changes; and what each bank
    /// owes its customers.
    reserves_total: i128,
    deposits: Vec<i64>,
    /// Parties ended today, begun again after the close, and the next identity handed out.
    ended: Vec<u8>,
    next_id: u64,
}

/// A day's count of each unit of work, for the costs a unit.
#[derive(Default, Clone, Copy)]
struct Units {
    flows: u64,
    dues: u64,
    rows: u64,
    rule_ns: u64,
    choices: u64,
    hits: u64,
    turnover: u64,
    shorts: u64,
    rounds: u64,
}

/// A chunk's job: its index and rows of a column, and the flow buffer it fills.
type ChunkJob<'a, T> = ((usize, &'a mut [T]), &'a mut Vec<Flow>);

/// Hazard hits: each draws its party's next event from its own stream and writes the day to its record, the hits
/// taken chunk by chunk on the pool.
fn hazards(pool: &Pool, k: &mut Kind, count: u64, day: u32, rows_per_chunk: u32) -> u64 {
    let n = u64::from(k.n);
    let mut d = draws(4, 0, day);
    let mut hit: Vec<u32> = (0..count).map(|_| to_u32(below_u64(&mut d, n))).collect();
    hit.sort_unstable();
    let stride = k.stride;
    let rpc = to_usize(u64::from(rows_per_chunk));
    let jobs: Vec<(usize, &mut [MaybeI64])> = k
        .records
        .slice_mut()
        .chunks_mut(rpc * stride)
        .enumerate()
        .filter(|(c, _)| {
            let lo = to_u32(index_u64(c * rpc));
            hit.get(hit.partition_point(|s| *s < lo)).is_some_and(|s| *s < lo + rows_per_chunk)
        })
        .collect();
    let hit = &hit;
    pool.for_each(jobs, |(c, rows)| {
        let lo = to_u32(index_u64(c * rpc));
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

/// What a handler is: its declared reads and writes, its rule's iterations, its flows a thousand rows and their
/// payees' kind and count.
struct HandlerRun {
    reads: usize,
    writes: usize,
    iterations: u64,
    flows_per_thousand: u64,
    kind: u8,
    payee: (u8, u32),
    salt: u64,
}

/// A handler over the day's agenda of a kind: each row's declared facts read from its chunk's records, its rule's
/// arithmetic spent, its writes made, and its flows emitted, chunk by chunk on the pool, only the chunks the agenda
/// names dispatched.
fn handler(
    pool: &Pool,
    k: &mut Kind,
    rows: &[u32],
    run: &HandlerRun,
    passes: u64,
    bufs: &mut FlowBufs,
    rpc: u32,
) -> u64 {
    let stride = k.stride;
    let writes = if run.writes < stride - 1 { run.writes } else { stride - 1 };
    // The declared facts' offsets in a record: the reads from its start, the writes at its end.
    let offsets: Vec<usize> =
        (0..run.reads).map(|r| r % (stride - writes)).chain((0..writes).map(|x| stride - writes + x)).collect();
    let writable: Vec<bool> = (0..run.reads).map(|_| false).chain((0..writes).map(|_| true)).collect();
    // The chunks the agenda's rows fall in, each with its span of the rows.
    let mut spans: Vec<(usize, usize, usize)> = Vec::new();
    let mut from = 0;
    while let Some(first) = rows.get(from) {
        let chunk = first / rpc;
        let to = from + rows.get(from..).map_or(0, |r| r.partition_point(|s| *s / rpc == chunk));
        spans.push((to_usize(u64::from(chunk)), from, to));
        from = to;
    }
    bufs.reset(spans.len());
    let rows_a_chunk = to_usize(u64::from(rpc)) * stride;
    let mut chunks = k.records.slice_mut().chunks_mut(rows_a_chunk).enumerate();
    let mut jobs: Vec<ChunkJob<'_, MaybeI64>> = Vec::with_capacity(spans.len());
    for ((chunk, _, _), buf) in spans.iter().zip(bufs.chunks_mut().iter_mut()) {
        if let Some(job) = chunks.find(|(c, _)| c == chunk) {
            jobs.push((job, buf));
        }
    }
    let (offsets, writable, spans) = (&offsets, &writable, &spans);
    let items: Vec<(usize, ChunkJob<'_, MaybeI64>)> = jobs.into_iter().enumerate().collect();
    let visited: u64 = pool
        .map_items(items, |(j, ((c, records), buf))| {
            let Some((_, lo, hi)) = spans.get(j) else { return 0 };
            let first = to_u32(index_u64(c)) * rpc;
            let mut store =
                RecordFacts::new(Slot::new(first), stride, records, Layout { names: &[], offsets, writable });
            let due = rows.get(*lo..*hi).unwrap_or(&[]);
            for (i, s) in due.iter().enumerate() {
                let slot = Slot::new(*s);
                let mut acc = 0.0;
                for r in 0..run.reads {
                    if let phx_num::Missing::Present(v) = store.read_at(r, slot) {
                        acc += phx_rand::float::from_i64(v);
                    }
                }
                let out = spin(acc, run.iterations * passes);
                for wi in 0..writes {
                    store.write_at(run.reads + wi, slot, phx_rand::float::floor_to_i64(out).unwrap_or(0));
                }
                // Thousandths of a flow a row: a row emits one each time the running count passes a thousand.
                let seen = index_u64(i) * passes;
                let made =
                    (seen + passes) * run.flows_per_thousand / THOUSAND - seen * run.flows_per_thousand / THOUSAND;
                for m in 0..made {
                    // Its counterparty is its own, a supplier or a payee it knows; any stands in for it here.
                    let to = to_u32(mix64(u64::from(*s) ^ run.salt ^ m) % u64::from(run.payee.1));
                    buf.push(Flow {
                        payer: PartyKey::new(run.kind, slot),
                        payee: PartyKey::new(run.payee.0, Slot::new(to)),
                        amount: phx_rand::float::floor_to_i64(out.abs() % 1e5).unwrap_or(0) + 1,
                        source: *s,
                        denomination: Denom::money(0),
                        reason: 1,
                        order: 0,
                    });
                }
            }
            index_u64(due.len()) * passes
        })
        .into_iter()
        .sum();
    visited
}

/// The day's sellers of each (region, product) and the alias table of their logit weights over posted prices: built
/// once a day, as sellers post, and read by every buyer of the group.
fn seller_tables(pool: &Pool, sellers: &Kind, v: &WorldSpec) -> Vec<Option<(Vec<u32>, AliasTable)>> {
    let groups = to_usize(u64::from(v.regions * v.products));
    let (prices, stride) = (sellers.records.slice(), sellers.stride);
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); groups];
    for s in 0..sellers.n {
        if let Some(g) = members.get_mut(to_usize(u64::from(s % (v.regions * v.products)))) {
            g.push(s);
        }
    }
    let members = &members;
    pool.map(groups, |g| {
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
    })
}

/// Purchases: each buyer-product draws a seller among its region's sellers of the product by their logit weights,
/// with the buyer's own taste from its own stream, and pays it; most also take goods. Buyers are taken group by group,
/// so each group's table stays in cache while its buyers draw.
fn choices(
    pool: &Pool,
    tables: &[Option<(Vec<u32>, AliasTable)>],
    (buyer, n_buyers, seller): (u8, u32, u8),
    (count, flows_per_thousand): (u64, u64),
    (v, taste, day): (&WorldSpec, StreamKey, u32),
    bufs: &mut FlowBufs,
) {
    let groups = tables.len();
    let per_region = u64::from(n_buyers.div_ceil(v.regions));
    bufs.reset(groups);
    let jobs: Vec<(usize, &mut Vec<Flow>)> = bufs.chunks_mut().iter_mut().enumerate().collect();
    let (regions, products) = (u64::from(v.regions), u64::from(v.products));
    pool.for_each(jobs, |(g, buf)| {
        let Some(Some((list, table))) = tables.get(g) else { return };
        let (first, units) = piece(count, groups, g);
        let (region, product) = (index_u64(g) / products, index_u64(g) % products);
        buf.reserve(to_usize(units * flows_per_thousand / THOUSAND + 1));
        for u in 0..units {
            // The buyers come from the day's spending decisions in slot order; the group's are taken in turn.
            let b = region + regions * ((first + u) % per_region);
            if b >= u64::from(n_buyers) {
                continue;
            }
            let mut d = Draws::new(taste, Subject::new(SubjectTag::Party, b), day, to_u32(product).to_le_bytes()[0]);
            let Some(s) = list.get(table.draw(&mut d)) else { continue };
            let buyer_key = PartyKey::new(buyer, Slot::new(to_u32(b)));
            let seller_key = PartyKey::new(seller, Slot::new(*s));
            let amount = i64::try_from(mix64(b ^ u) % 5000).unwrap_or(0) + 1;
            let money = Flow {
                payer: buyer_key,
                payee: seller_key,
                amount,
                source: *s,
                denomination: Denom::money(0),
                reason: 2,
                order: 0,
            };
            buf.push(money);
            let made = (first + u + 1) * flows_per_thousand / THOUSAND - (first + u) * flows_per_thousand / THOUSAND;
            for _ in 1..made {
                buf.push(Flow {
                    payer: seller_key,
                    payee: buyer_key,
                    denomination: Denom::units(1),
                    reason: 3,
                    ..money
                });
            }
        }
    });
}

/// Takes a family's dues from the last day handed out through `day`: each due read from its contract — its amount
/// and what its rate adds on its balance — emitted, and the contract moved to its next due a month on, only the chunks
/// holding dues dispatched; a margin's share of dues pays a second leg.
fn dues(
    pool: &Pool,
    f: &mut Family,
    days: (u32, u32),
    margin: (u64, u64),
    (due, bufs): (&mut Vec<u32>, &mut FlowBufs),
    rpc: u32,
) -> u64 {
    let mut total = 0_u64;
    let rows_a_chunk = to_usize(u64::from(rpc));
    bufs.reset(f.due.len().div_ceil(rows_a_chunk));
    for d in days.0..=days.1 {
        f.wheel.take(Day::new(d), due, Some(pool));
        if due.is_empty() {
            continue;
        }
        let next = d + MONTH_DAYS;
        let (edges, amount, balance) = (&f.edges, &f.amount, &f.balance);
        let list: &[u32] = due;
        let mut spans: Vec<(usize, usize, usize)> = Vec::new();
        let mut from = 0;
        while let Some(first) = list.get(from) {
            let chunk = first / rpc;
            let to = from + list.get(from..).map_or(0, |r| r.partition_point(|s| *s / rpc == chunk));
            spans.push((to_usize(u64::from(chunk)), from, to));
            from = to;
        }
        let mut chunks = f.due.slice_mut().chunks_mut(rows_a_chunk).enumerate();
        let mut buffers = bufs.chunks_mut().iter_mut().enumerate();
        let mut jobs: Vec<(ChunkJob<'_, u32>, (usize, usize))> = Vec::with_capacity(spans.len());
        for (chunk, a, b) in &spans {
            if let (Some(job), Some((_, buf))) = (chunks.find(|(c, _)| c == chunk), buffers.find(|(c, _)| c == chunk)) {
                jobs.push(((job, buf), (*a, *b)));
            }
        }
        let moved: Vec<Vec<u32>> = pool.map_items(jobs, |(((c, dues_col), buf), (a, b))| {
            let lo = to_u32(index_u64(c * rows_a_chunk));
            let mut moved = Vec::with_capacity(b - a);
            for (i, e) in list.get(a..b).unwrap_or(&[]).iter().enumerate() {
                let edge = Slot::new(*e);
                let Some(cell) = dues_col.get_mut(to_usize(u64::from(*e - lo))) else { continue };
                // A stale entry — a contract whose due moved, or a slot since reused — is not today's due.
                if *cell != d || !edges.is_open(edge) {
                    continue;
                }
                let (Some(from), Some(to), Some(amt), Some(bal)) =
                    (edges.end(edge, 0), edges.end(edge, 1), amount.get(edge), balance.get(edge))
                else {
                    continue;
                };
                let flow = Flow {
                    payer: from,
                    payee: to,
                    amount: amt + bal * RATE_PPM / i64::try_from(MILLION).unwrap_or(1),
                    source: *e,
                    denomination: Denom::money(0),
                    reason: 4,
                    order: 1,
                };
                buf.push(flow);
                let i = index_u64(a + i);
                let extra = (i + 1) * margin.0 / margin.1 - i * margin.0 / margin.1;
                for _ in 1..extra {
                    buf.push(Flow { amount: amt / 2 + 1, reason: 5, ..flow });
                }
                *cell = next;
                moved.push(*e);
            }
            moved
        });
        // The chunks' moved contracts in chunk order are in slot order, so the bucket they join keeps them sorted.
        for m in &moved {
            total += index_u64(m.len());
            f.wheel.schedule_all(m, Day::new(next));
        }
    }
    total
}

/// A day's turnover of contracts: in each family its share of the day's count closed, each off its parties' lists,
/// and as many opened between parties drawn afresh, each put on the wheel for its first due.
fn turnover(f: &mut Family, sizes: [u32; 2], count: u64, day: u32, seed: u64) -> u64 {
    let mut d = draws(11, seed, day);
    let high = u64::from(f.edges.high_water());
    let mut done = 0_u64;
    for _ in 0..count {
        let edge = Slot::new(to_u32(below_u64(&mut d, high)));
        if !f.edges.is_open(edge) {
            continue;
        }
        let (Some(a), Some(b)) = (f.edges.end(edge, 0), f.edges.end(edge, 1)) else { continue };
        let [h0, h1] = &mut f.heads;
        let hs = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(a.slot().get())))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(b.slot().get())))),
        ];
        f.edges.close(edge, hs);
        done += 1;
    }
    for _ in 0..done {
        let ends: [PartyKey; 2] = [0, 1].map(|side| {
            let (kind, size) = (f.kinds.get(side).copied().unwrap_or(0), sizes.get(side).copied().unwrap_or(1));
            PartyKey::new(kind, Slot::new(to_u32(below_u64(&mut d, u64::from(size)))))
        });
        let [h0, h1] = &mut f.heads;
        let hs = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[0].slot().get())))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[1].slot().get())))),
        ];
        let edge = f.edges.open(ends, hs);
        let a = f.mean / 2 + i64::try_from(below_u64(&mut d, u64::try_from(f.mean).unwrap_or(0) + 1)).unwrap_or(0);
        f.amount.put(edge, a);
        f.balance.put(edge, i64::try_from(below_u64(&mut d, BALANCE_SPAN)).unwrap_or(0));
        let first = day + 1 + to_u32(below_u64(&mut d, u64::from(MONTH_DAYS)));
        f.due.put(edge, first);
        for c in &mut f.others {
            c.put(edge, 0);
        }
        f.wheel.schedule(edge.get(), Day::new(first));
    }
    done * 2
}

impl Load {
    /// Nets every buffer filled since the last netting by range and applies each range's nets to its parties' money,
    /// or, on a closed day, to what they have pending, which the next business day settles; returns the flows and the
    /// payers the debits leave below nothing.
    fn settle(&mut self, business: bool, day: u32) -> (u64, u64, u64) {
        let refs: Vec<&FlowBufs> = self.bufs.iter().zip(&self.filled).filter(|(_, f)| **f).map(|(b, _)| b).collect();
        self.grouped.group(Some(&self.pool), &refs, &self.ranges, Denom::money(0));
        let flows = index_u64(self.grouped.by_payer.items.len());
        let mut books = Books {
            kinds: self.kinds.iter_mut().map(Kind::book).collect(),
            banks: self.bank_kind,
            deposits: &mut self.deposits,
            closed: &self.closed,
            issuer: self.issuer,
        };
        let pool: &Pool = &self.pool;
        let failed = if business {
            let key = self.lot;
            let lot = |p: PartyKey| Draws::new(key, Subject::new(SubjectTag::Party, u64::from(p.word())), day, 0);
            let out = self.settlement.settle(Some(pool), &self.grouped, &self.ranges, &mut books, &lot);
            (index_u64(out.failed.len()), out.rounds)
        } else {
            self.settlement.commit(Some(pool), &self.grouped, &self.ranges, &mut books);
            (0, 0)
        };
        self.filled.fill(false);
        (flows, failed.0, failed.1)
    }

    /// The day's identities: the money every account and its pending hold together is what it was, since every flow
    /// moves it between two of them; and a thirtieth of the contracts read in full.
    fn audit(&mut self, day: u32) -> Result<i64, String> {
        let reserves = reserves_of(&self.kinds, self.bank_kind);
        if reserves != self.reserves_total {
            return Err(format!("reserves changed from {} to {reserves} on day {day}", self.reserves_total));
        }
        let owed = deposits_of(&self.kinds, self.deposits.len());
        if let Some(b) = owed.iter().zip(&self.deposits).position(|(o, d)| o != d) {
            return Err(format!("bank {b}'s customers hold other than it owes them on day {day}"));
        }
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
        Ok(contracts)
    }

    /// A tenth of the day's turnover ends parties of the households and firms, and yesterday's ended are begun again,
    /// taking the slots the close released.
    fn churn(&mut self, count: u64, day: u32, kinds: [u8; 2]) -> u64 {
        let mut done = 0;
        for k in std::mem::take(&mut self.ended) {
            if let Some(kind) = self.kinds.get_mut(usize::from(k)) {
                self.next_id += 1;
                let _ = kind.parties.begin(PartyId::new(self.next_id));
                done += 1;
            }
        }
        let mut d = draws(12, 0, day);
        for _ in 0..count / PARTY_SHARE {
            let k = kinds.get(to_usize(below_u64(&mut d, 2))).copied().unwrap_or(0);
            let Some(kind) = self.kinds.get_mut(usize::from(k)) else { continue };
            let slot = Slot::new(to_u32(below_u64(&mut d, u64::from(kind.n))));
            if let Some(r) = kind.parties.at(slot) {
                kind.parties.end(r);
                self.ended.push(k);
                done += 1;
            }
        }
        done
    }

    /// A full save: every kind's and family's columns and the held stores, each to its own file, compressed in frames
    /// on the pool; returns the bytes written.
    fn save(&self, dir: &std::path::Path) -> Result<u64, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let mut sources: Vec<Vec<&[u8]>> = Vec::new();
        for k in &self.kinds {
            let mut cols: Vec<&[u8]> = vec![phx_store::as_bytes(k.records.slice())];
            cols.extend(k.money.iter().chain(&k.pending).map(|m| phx_store::as_bytes(m.slice())));
            sources.push(cols);
        }
        for f in &self.families {
            let mut cols: Vec<&[u8]> = vec![
                phx_store::as_bytes(f.edges.ends(0)),
                phx_store::as_bytes(f.edges.ends(1)),
                phx_store::as_bytes(f.amount.slice()),
                phx_store::as_bytes(f.balance.slice()),
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

/// A kind's parties at the design point, with random records and money.
fn build_kind(
    space: &mut AddressSpace,
    pool: &Pool,
    k: &KindSpec,
    (index, n, banks): (u8, u32, Option<u32>),
    rows_per_chunk: u32,
) -> Kind {
    let mut parties = Parties::new(space, index, n, rows_per_chunk);
    for i in 0..n {
        let _ = parties.begin(PartyId::new((u64::from(index) << 32) | (u64::from(i) + 1)));
    }
    let words_a_party = to_usize(k.bytes.div_ceil(index_u64(size_of::<i64>())));
    // A record holds at least a fact read and one written.
    let stride = if words_a_party < 2 { 2 } else { words_a_party };
    let words = u64::from(n) * index_u64(stride);
    let mut records: Column<MaybeI64, SystemBacking> = Column::new(space, to_u32(words), rows_per_chunk);
    let np = pool.workers();
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
    let column = |space: &mut AddressSpace, v: i64| {
        let mut m: Column<i64, SystemBacking> = Column::new(space, n, rows_per_chunk);
        m.extend(&vec![v; to_usize(u64::from(n))]);
        m
    };
    // Balances spread over four orders of magnitude, as accounts' are, so few are short on a day.
    let money = k.money.then(|| {
        let mut d = draws(15, u64::from(index), 0);
        let spread: Vec<i64> = (0..n)
            .map(|_| {
                let scale = (0..4 + below_u64(&mut d, 5)).fold(1_i64, |a, _| a * 10);
                scale * (1 + i64::try_from(below_u64(&mut d, 9)).unwrap_or(0))
            })
            .collect();
        let mut m: Column<i64, SystemBacking> = Column::new(space, n, rows_per_chunk);
        m.extend(&spread);
        m
    });
    let pending = k.money.then(|| column(space, 0));
    // Each account at a bank drawn for it; the banks' own money is their reserves, held at the issuer.
    let accounts = k.money.then(|| {
        let mut bank: Column<u32, SystemBacking> = Column::new(space, n, rows_per_chunk);
        let at: Vec<u32> = (0..n)
            .map(|s| banks.map_or(AT_ISSUER, |b| to_u32(mix64(u64::from(s) ^ u64::from(index)) % u64::from(b))))
            .collect();
        bank.extend(&at);
        Accounts { bank, held: column(space, 0), facility: column(space, 0) }
    });
    Kind { parties, records, stride, money, pending, accounts, n }
}

/// The banks' reserves and what they have pending, summed.
fn reserves_of(kinds: &[Kind], banks: u8) -> i128 {
    kinds
        .get(usize::from(banks))
        .and_then(|k| k.money.as_ref().zip(k.pending.as_ref()))
        .map_or(0, |(m, p)| m.slice().iter().chain(p.slice()).map(|x| i128::from(*x)).sum())
}

/// What the customers of each bank hold, their pending included, summed by bank.
fn deposits_of(kinds: &[Kind], banks: usize) -> Vec<i64> {
    let mut owed = vec![0_i64; banks];
    for k in kinds {
        let (Some(m), Some(p), Some(a)) = (k.money.as_ref(), k.pending.as_ref(), k.accounts.as_ref()) else { continue };
        for ((b, m), p) in a.bank.slice().iter().zip(m.slice()).zip(p.slice()) {
            if let Some(o) = owed.get_mut(to_usize(u64::from(*b))) {
                *o += m + p;
            }
        }
    }
    owed
}

/// A family's contracts between random parties of its two kinds, each first due by its family's dues.
fn build_family(
    space: &mut AddressSpace,
    f: &FamilySpec,
    (kinds, sizes): ([u8; 2], [u32; 2]),
    (n, headroom): (u32, u32),
    (rows_per_chunk, heavy, month): (u32, u32, u32),
    seed: u64,
) -> Family {
    // A contract closed today frees its slot only after the day closes, so the day's openings need room of their own.
    let capacity = n + headroom;
    let mut edges = EdgeTable::new(space, capacity, rows_per_chunk, f.listed);
    let mut heads: [Option<Column<u32, SystemBacking>>; 2] = [0, 1].map(|side| {
        f.listed.get(side).copied().unwrap_or(false).then(|| {
            let size = sizes.get(side).copied().unwrap_or(1);
            let mut h: Column<u32, SystemBacking> = Column::new(space, size, rows_per_chunk);
            h.extend(&vec![NONE; to_usize(u64::from(size))]);
            h
        })
    });
    let mut amount: Column<i64, SystemBacking> = edges.column(space);
    let mut balance: Column<i64, SystemBacking> = edges.column(space);
    let mut due: Column<u32, SystemBacking> = edges.column(space);
    let mut wheel = DueWheel::new(Day::new(0), WHEEL_DAYS);
    let mut d = draws(10, seed, 0);
    for i in 0..n {
        // A chunk of contracts an address, since one address holds fewer draws than the largest family needs.
        if i % rows_per_chunk == 0 {
            d = draws(10, (seed << 32) | u64::from(i / rows_per_chunk), 0);
        }
        let ends: [PartyKey; 2] = [0, 1].map(|side| {
            let (kind, size) = (kinds.get(side).copied().unwrap_or(0), sizes.get(side).copied().unwrap_or(1));
            PartyKey::new(kind, Slot::new(to_u32(below_u64(&mut d, u64::from(size)))))
        });
        let [h0, h1] = &mut heads;
        let hs = [
            h0.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[0].slot().get())))),
            h1.as_mut().and_then(|h| h.slice_mut().get_mut(to_usize(u64::from(ends[1].slot().get())))),
        ];
        let edge = edges.open(ends, hs);
        let a = f.mean / 2 + i64::try_from(below_u64(&mut d, u64::try_from(f.mean).unwrap_or(0) + 1)).unwrap_or(0);
        amount.put(edge, a);
        balance.put(edge, i64::try_from(below_u64(&mut d, BALANCE_SPAN)).unwrap_or(0));
        let first = match f.dues {
            Dues::None => None,
            Dues::Payday => Some(heavy),
            Dues::Spread => Some(to_u32(below_u64(&mut d, u64::from(month)))),
            Dues::Quarterly => Some(to_u32(below_u64(&mut d, QUARTER_DAYS))),
        };
        due.put(edge, first.unwrap_or(u32::MAX));
        if let Some(day) = first {
            wheel.schedule(edge.get(), Day::new(day));
        }
    }
    let words = to_usize(if f.bytes > 20 { (f.bytes - 20).div_ceil(8) } else { 0 });
    let others = (0..words)
        .map(|_| {
            let mut c: Column<u64, SystemBacking> = edges.column(space);
            c.extend(&vec![0_u64; to_usize(u64::from(n))]);
            c
        })
        .collect();
    Family { edges, amount, balance, due, others, heads, wheel, kinds, mean: f.mean }
}

/// A handler's agenda: its kind's rows by the phase of their schedule, the period the kind's rows over the day's
/// business count, so a day hands out about that count; more a day than rows, and each row makes several passes.
fn agenda(n: u32, count: u64, salt: u64) -> Agenda {
    let rows = u64::from(n);
    if count == 0 {
        return Agenda { buckets: Vec::new(), passes: 0 };
    }
    let (period, passes) = if count >= rows { (1, count.div_ceil(rows)) } else { (rows / count, 1) };
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); to_usize(period)];
    for s in 0..n {
        if let Some(b) = buckets.get_mut(to_usize(mix64(u64::from(s) ^ salt) % period)) {
            b.push(s);
        }
    }
    Agenda { buckets, passes }
}

/// Builds the stores: every kind, every family, the held stores, and each handler's agenda.
fn build(v: &Volumes, host: &dyn BenchHost, clock: &Mono) -> Result<Load, String> {
    let pool = std::sync::Arc::new(Pool::new(&PoolSpec::detect()).map_err(|e| e.0)?);
    let mut space = AddressSpace::empty();
    let rpc = v.world.rows_per_chunk;
    let mut kinds = Vec::with_capacity(v.kinds.len());
    let bank_kind = v.kind("bank")?;
    let n_banks = to_u32(v.at(v.kinds.get(usize::from(bank_kind)).map_or(0, |k| k.per_million)));
    for (i, k) in v.kinds.iter().enumerate() {
        let n = to_u32(v.at(k.per_million));
        show(host, "building", format!("{} {}", n, k.name), String::new(), "");
        let index = u8::try_from(i).unwrap_or(u8::MAX);
        kinds.push(build_kind(&mut space, &pool, k, (index, n, (index != bank_kind).then_some(n_banks)), rpc));
    }
    let days = u32::try_from(v.month.days.len()).unwrap_or(0);
    let heavy = v.month.days.iter().position(|d| *d == DayType::Heavy).map_or(0, |p| to_u32(index_u64(p)));
    let turnover = v
        .works
        .iter()
        .filter(|w| w.kernel == Kernel::Turnover)
        .flat_map(|w| w.per_million.iter().map(|p| v.work(*p)))
        .reduce(|a, b| if b > a { b } else { a });
    let headroom = to_u32(turnover.unwrap_or(0));
    let mut families = Vec::with_capacity(v.families.len());
    for (i, f) in v.families.iter().enumerate() {
        let n = to_u32(v.at(f.per_million));
        show(host, "building", format!("{} {} contracts", n, f.name), String::new(), "");
        let ks = [v.kind(&f.payer)?, v.kind(&f.payee)?];
        let sizes = ks.map(|k| kinds.get(usize::from(k)).map_or(1, |x: &Kind| x.n));
        families.push(build_family(&mut space, f, (ks, sizes), (n, headroom), (rpc, heavy, days), index_u64(i)));
    }
    let held = v
        .stores
        .iter()
        .map(|s| {
            let words = v.at(s.per_million_bytes) / index_u64(size_of::<u64>());
            let np = pool.workers();
            pool.map(np, |p| {
                let (first, len) = piece(words, np, p);
                (first..first + len).map(|i| mix64(i) & ((1 << 25) - 1)).collect::<Vec<u64>>()
            })
            .concat()
        })
        .collect();
    let mut agendas = Vec::with_capacity(v.works.len());
    for (i, w) in v.works.iter().enumerate() {
        let made = match (w.kernel, w.table.as_deref()) {
            (Kernel::Handler, Some(t)) => {
                let n = kinds.get(usize::from(v.kind(t)?)).map_or(0, |k| k.n);
                Some(agenda(n, v.work(w.per_million.first().copied().unwrap_or(0)), mix64(index_u64(i))))
            }
            _ => None,
        };
        agendas.push(made);
    }
    let high: Vec<u32> = kinds.iter().map(|k| k.n).collect();
    let ranges = Ranges::new(v.world.range_bits, &high);
    // The day's grouped flows are reserved at the heaviest day's declared count: address space, touched only as used.
    let mut grouped = Grouped::default();
    let heavy_flows = v.heavy_flows(families.iter().map(|f| index_u64(f.due.len())).sum());
    grouped.by_payer.items.reserve(to_usize(heavy_flows));
    grouped.by_payee.items.reserve(to_usize(heavy_flows));
    // Each bank holds a tenth of what its customers hold in reserves.
    let deposits = deposits_of(&kinds, to_usize(u64::from(n_banks)));
    if let Some(m) = kinds.get_mut(usize::from(bank_kind)).and_then(|k| k.money.as_mut()) {
        for (r, d) in m.slice_mut().iter_mut().zip(&deposits) {
            *r = d / 10;
        }
    }
    let reserves_total = reserves_of(&kinds, bank_kind);
    let sources = v.works.len() + families.len();
    let persons = v.world.persons;
    // The central bank: a party past every institution's slot, so no flow of the bench names it and money is made by
    // no one.
    let institution = v.kind("institution")?;
    let issuer = PartyKey::new(institution, Slot::new(kinds.get(usize::from(institution)).map_or(0, |k| k.n)));
    Ok(Load {
        pool,
        kinds,
        families,
        held,
        ranges,
        bufs: (0..sources).map(|_| FlowBufs::default()).collect(),
        filled: vec![false; sources],
        grouped,
        settlement: Settle::default(),
        bank_kind,
        closed: vec![false; to_usize(u64::from(n_banks))],
        issuer,
        lot: stream_key(Seed::new(14), "SET.order"),
        due_today: Vec::new(),
        wheel_day: 0,
        ns_per_iteration: calibrate(clock),
        tables: Vec::new(),
        tables_day: None,
        agendas,
        taste: stream_key(Seed::new(13), "LOAD.taste"),
        reserves_total,
        deposits,
        ended: Vec::new(),
        next_id: persons << 8,
    })
}

/// What one day's work took, kind by kind, and its units.
struct DayRecord {
    index: usize,
    kind: DayType,
    walls: Vec<(String, u64, u64)>,
    /// Each work's CPU time across every thread, in the works' order: what it keeps the cores busy for.
    cpus: Vec<u64>,
    total_ns: u64,
    cpu_ns: u64,
    faults: u64,
    units: Units,
}

impl DayRecord {
    /// The day's time on the phone: its CPU time here over the phone's sustained cores, and never less than its wall
    /// time here.
    fn phone_ns(&self) -> u64 {
        let spread = self.cpu_ns / PHONE_CORES;
        if spread > self.total_ns { spread } else { self.total_ns }
    }
}

/// The month's turns on the phone: each business or heavy day closes a turn holding the closed days before it.
fn turns(days: &[DayRecord]) -> Vec<u64> {
    let mut out = Vec::new();
    let mut running = 0_u64;
    for d in days {
        running += d.phone_ns();
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

/// A handler's work on one day: the day's agenda of the kind, cut to the day's count, run.
fn run_handler(
    load: &mut Load,
    v: &Volumes,
    (w, wi): (&WorkSpec, usize),
    (day, count): (u32, u64),
    table: Option<u8>,
    units: &mut Units,
) -> Result<u64, String> {
    let rpc = v.world.rows_per_chunk;
    let kind = table.unwrap_or(0);
    let payee = match w.payee.as_deref() {
        Some(p) => v.kind(p)?,
        None => kind,
    };
    let payee_n = load.kinds.get(usize::from(payee)).map_or(1, |k| k.n);
    let iterations =
        phx_rand::float::floor_to_i64(phx_rand::float::from_u64(w.rule_ns.unwrap_or(0)) / load.ns_per_iteration)
            .and_then(|i| u64::try_from(i).ok())
            .unwrap_or(0);
    let run = HandlerRun {
        reads: w.reads.unwrap_or(0),
        writes: w.writes.unwrap_or(0),
        iterations,
        flows_per_thousand: w.flows.unwrap_or(0),
        kind,
        payee: (payee, payee_n),
        salt: mix64((u64::from(day) << 8) | index_u64(wi)),
    };
    let (Some(Some(agenda)), Some(k), Some(bufs)) =
        (load.agendas.get(wi), load.kinds.get_mut(usize::from(kind)), load.bufs.get_mut(wi))
    else {
        return Ok(0);
    };
    let bucket = to_usize(u64::from(day))
        .checked_rem(agenda.buckets.len())
        .and_then(|b| agenda.buckets.get(b))
        .map_or(&[][..], Vec::as_slice);
    // On a day that asks fewer than its business count, the rows whose second phase falls below the share.
    let business = v.work(w.per_million.first().copied().unwrap_or(0));
    let rows: Vec<u32> = if count >= business {
        bucket.to_vec()
    } else {
        let cut = u64::MAX.checked_div(business).map_or(0, |c| c * count);
        bucket.iter().copied().filter(|s| mix64(u64::from(*s) ^ run.salt) < cut).collect()
    };
    let visited = handler(&load.pool, k, &rows, &run, agenda.passes, bufs, rpc);
    if let Some(f) = load.filled.get_mut(wi) {
        *f = true;
    }
    units.rows += visited;
    units.rule_ns += visited * w.rule_ns.unwrap_or(0);
    Ok(visited)
}

/// One kind of work on one day, at its count with the margin; returns its units.
fn run_work(
    load: &mut Load,
    v: &Volumes,
    (w, wi): (&WorkSpec, usize),
    per_million: u64,
    (day, kind): (u32, DayType),
    units: &mut Units,
) -> Result<u64, String> {
    let count = v.work(per_million);
    let table = w.table.as_deref().map(|t| v.kind(t)).transpose()?;
    let rpc = v.world.rows_per_chunk;
    let done = match w.kernel {
        Kernel::Hazard => {
            let Some(k) = load.kinds.get_mut(usize::from(table.unwrap_or(0))) else { return Ok(0) };
            let hits = hazards(&load.pool, k, count, day, rpc);
            units.hits += hits;
            hits
        }
        Kernel::Handler => run_handler(load, v, (w, wi), (day, count), table, units)?,
        Kernel::Choice => {
            let sellers = v.kind("firm")?;
            if load.tables_day != Some(day) {
                let Some(s) = load.kinds.get(usize::from(sellers)) else { return Ok(0) };
                load.tables = seller_tables(&load.pool, s, &v.world);
                load.tables_day = Some(day);
            }
            let buyer = table.unwrap_or(0);
            let n_buyers = load.kinds.get(usize::from(buyer)).map_or(1, |k| k.n);
            let Some(bufs) = load.bufs.get_mut(wi) else { return Ok(0) };
            let spec = (&v.world, load.taste, day);
            choices(
                &load.pool,
                &load.tables,
                (buyer, n_buyers, sellers),
                (count, w.flows.unwrap_or(THOUSAND)),
                spec,
                bufs,
            );
            if let Some(f) = load.filled.get_mut(wi) {
                *f = true;
            }
            units.choices += count;
            count
        }
        Kernel::Dues => {
            if kind == DayType::Closed {
                return Ok(0);
            }
            let from = load.wheel_day;
            let base = v.works.len();
            let mut total = 0;
            let mut due = std::mem::take(&mut load.due_today);
            for (fi, f) in load.families.iter_mut().enumerate() {
                let Some(bufs) = load.bufs.get_mut(base + fi) else { continue };
                total += dues(&load.pool, f, (from, day), (v.world.work_num, v.world.work_den), (&mut due, bufs), rpc);
                if let Some(x) = load.filled.get_mut(base + fi) {
                    *x = true;
                }
            }
            load.due_today = due;
            load.wheel_day = day + 1;
            units.dues += total;
            total
        }
        Kernel::Turnover => {
            let contracts: u64 = load.families.iter().map(|f| index_u64(f.due.len())).sum();
            let kinds = &load.kinds;
            // Families hold their own contracts and lists, so each turns over on its own worker.
            let jobs: Vec<(usize, &mut Family)> = load.families.iter_mut().enumerate().collect();
            let mut done: u64 = load
                .pool
                .map_items(jobs, |(fi, f)| {
                    let share = (count * index_u64(f.due.len())).checked_div(contracts).unwrap_or(0);
                    let sizes = f.kinds.map(|k| kinds.get(usize::from(k)).map_or(1, |x| x.n));
                    turnover(f, sizes, share, day, index_u64(fi))
                })
                .into_iter()
                .sum();
            done += load.churn(count, day, [v.kind("household")?, v.kind("firm")?]);
            units.turnover += done;
            done
        }
        Kernel::Settle => {
            let (flows, shorts, rounds) = load.settle(kind != DayType::Closed, day);
            units.rounds += rounds;
            units.flows += flows;
            units.shorts += shorts;
            flows
        }
        Kernel::Audit => {
            black_box(load.audit(day)?);
            1
        }
    };
    Ok(done)
}

/// One day's work, each kind at its count for the day's type, with the margin; returns each kind's wall time and
/// units, and the day's units.
fn run_day(load: &mut Load, v: &Volumes, i: usize, kind: DayType, clock: &Mono) -> Result<DayRecord, String> {
    let day = to_u32(index_u64(i));
    let mut units = Units::default();
    let mut walls = Vec::with_capacity(v.works.len());
    let mut cpus = Vec::with_capacity(v.works.len());
    let day_start = clock.now_ns();
    let before = process_times();
    let day_cpu = phx_exec::process_cpu_ns();
    for (wi, w) in v.works.iter().enumerate() {
        let per_million = w.per_million.get(kind.index()).copied().unwrap_or(0);
        let (t0, c0) = (clock.now_ns(), phx_exec::process_cpu_ns());
        let done = if per_million > 0 { run_work(load, v, (w, wi), per_million, (day, kind), &mut units)? } else { 0 };
        walls.push((w.name.clone(), clock.now_ns() - t0, done));
        cpus.push(c0.zip(phx_exec::process_cpu_ns()).map_or(0, |(a, b)| b - a));
    }
    for k in &mut load.kinds {
        k.parties.close_day();
    }
    for f in &mut load.families {
        f.edges.close_day();
    }
    let faults = before.zip(process_times()).map_or(0, |(f0, f1)| f1 - f0);
    let cpu_ns = day_cpu.zip(phx_exec::process_cpu_ns()).map_or(0, |(a, b)| b - a);
    Ok(DayRecord { index: i, kind, walls, cpus, total_ns: clock.now_ns() - day_start, cpu_ns, faults, units })
}

/// The full-load bench over the volumes at `volumes_path`, saves written to and removed from `save_dir`, its report
/// written to `report_path` and returned.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, a save that cannot be written, a report that cannot, or an
/// identity the audit finds broken.
pub fn run(host: &dyn BenchHost, volumes_path: &str, save_dir: &str, report_path: &str) -> Result<String, String> {
    let text = measure(host, volumes_path, save_dir)?.pretty();
    std::fs::write(report_path, &text).map_err(|e| format!("{report_path}: {e}"))?;
    show(host, "report", report_path.to_owned(), String::new(), "");
    Ok(text)
}

/// The full-load bench's month, each day and save shown as it completes: the load section of the device report.
///
/// # Errors
/// Volumes that cannot be read, a pool that cannot start, a save that cannot be written, or an identity the audit
/// finds broken.
pub fn measure(host: &dyn BenchHost, volumes_path: &str, save_dir: &str) -> Result<Json, String> {
    let clock = Mono(Instant::now());
    let text = std::fs::read_to_string(volumes_path).map_err(|e| format!("{volumes_path}: {e}"))?;
    let v: Volumes = toml::from_str(&text).map_err(|e| format!("{volumes_path}: {e}"))?;
    let started = clock.now_ns();
    let mut load = build(&v, host, &clock)?;
    let built_ms = (clock.now_ns() - started) / NS_PER_MS;
    let built_peak = proc_kib("status", "VmHWM:");
    show(host, "built", format!("{built_ms} ms, peak {} MiB", built_peak.unwrap_or(0) / MIB), String::new(), "");
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
        let value = format!(
            "{} ms on the phone, {} ms here, {} faults, peak {peak} MiB; {}",
            r.phone_ns() / NS_PER_MS,
            r.total_ns / NS_PER_MS,
            r.faults,
            heaviest.join(", ")
        );
        show(host, &name, value, String::new(), "");
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
    let (peak, pss) = (proc_kib("status", "VmHWM:"), proc_kib("smaps_rollup", "Pss:"));
    let mut unit_costs = unit_costs(&records);
    let audit_cpu: u64 = records
        .iter()
        .flat_map(|r| r.walls.iter().zip(&r.cpus))
        .filter(|((n, _, _), _)| n == "audit")
        .map(|(_, cpu)| cpu)
        .sum();
    let audit_ms = (audit_cpu / NS_PER_MS).checked_div(index_u64(records.len())).unwrap_or(0);
    unit_costs.push((
        "the audit a million persons, core-ms".to_owned(),
        audit_ms * MILLION / v.world.persons,
        AUDIT_MS_PER_MILLION,
    ));
    unit_costs.push(("resident bytes a person".to_owned(), peak.unwrap_or(0) / v.world.persons, BYTES_PER_PERSON));
    for (name, value, target) in &unit_costs {
        show(host, name, value.to_string(), format!("≤ {target}"), verdict(value <= target));
    }
    let turn_ms: Vec<u64> = turns(&records).iter().map(|ns| ns / NS_PER_MS).collect();
    let (middle, worst) = (median(&turn_ms), greatest(&turn_ms));
    let two_saves: u64 = saves.iter().rev().take(2).map(|(_, _, b)| b).sum();
    let show_ms = |m: Option<u64>| m.map_or("none".to_owned(), |m| format!("{m} ms on the phone"));
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

/// Each unit's cost over the month, in core-nanoseconds: the CPU time its kinds of work kept every thread busy for,
/// spinning workers' included, over the units they did, a handler's rule's declared arithmetic taken off, with its target.
fn unit_costs(records: &[DayRecord]) -> Vec<(String, u64, u64)> {
    let mut sums: Vec<(&str, u64, u64, u64)> = vec![
        ("a flow netted and applied, core-ns", 0, 0, FLOW_NS),
        ("a due taken, read and emitted, core-ns", 0, 0, DUE_NS),
        ("a handler row beyond its rule, core-ns", 0, 0, ROW_NS),
        ("a purchase drawn, core-ns", 0, 0, CHOICE_NS),
        ("a hazard hit, core-ns", 0, 0, HAZARD_NS),
        ("a contract or party opened or closed, core-ns", 0, 0, TURNOVER_NS),
    ];
    let mut rule_ns = 0_u64;
    for r in records {
        rule_ns += r.units.rule_ns;
        for ((name, _, done), ns) in r.walls.iter().zip(&r.cpus) {
            let which = match name.as_str() {
                "settlement" => 0,
                "dues" => 1,
                "audit" => continue,
                "turnover" => 5,
                n if n.contains("purchases") => 3,
                n if n.starts_with("hazards") => 4,
                _ => 2,
            };
            if let Some(s) = sums.get_mut(which) {
                s.1 += ns;
                s.2 += done;
            }
        }
    }
    if let Some(rows) = sums.get_mut(2) {
        // The rule's own arithmetic is the systems', not the core's; measured noise can leave less than it.
        rows.1 = u64::try_from(i128::from(rows.1) - i128::from(rule_ns)).unwrap_or(0);
    }
    sums.into_iter().map(|(n, ns, done, target)| (n.to_owned(), ns.checked_div(done).unwrap_or(0), target)).collect()
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
    let count = |n: usize| Json::UInt(index_u64(n));
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
            ("phone_ms", Json::UInt(r.phone_ns() / NS_PER_MS)),
            ("cpu_ms", Json::UInt(r.cpu_ns / NS_PER_MS)),
            ("faults", Json::UInt(r.faults)),
            ("flows", Json::UInt(r.units.flows)),
            ("dues", Json::UInt(r.units.dues)),
            ("rows", Json::UInt(r.units.rows)),
            ("choices", Json::UInt(r.units.choices)),
            ("hits", Json::UInt(r.units.hits)),
            ("turnover", Json::UInt(r.units.turnover)),
            ("failed_flows", Json::UInt(r.units.shorts)),
            ("settlement_rounds", Json::UInt(r.units.rounds)),
            ("works", Json::Array(works.collect())),
        ])
    });
    let saves = saves
        .iter()
        .map(|(d, ms, b)| Json::obj([("after_day", count(*d)), ("ms", Json::UInt(*ms)), ("bytes", Json::UInt(*b))]));
    let units = unit_costs.iter().map(|(n, value, t)| {
        Json::obj([("unit", Json::str(n.clone())), ("value", Json::UInt(*value)), ("target", Json::UInt(*t))])
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
        assert!(text.contains("\"unit_costs\""), "the units' costs are reported");
    }
}
