# Project Phoenix — Architecture

How the world in `docs/spec/PROJECT_PHOENIX.md` is built: the technologies, the layers, the boundaries, the data
model, the execution model and the budgets, top down. The spec says **what**; this document says **how, in
outline**; `docs/IMPLEMENTATION.md` says **how, step by step**. Where this document and the spec disagree, the spec
wins and this document is fixed in the same change.

Clause identifiers in brackets, such as (REP.8), point to the spec.

---

## 1. Goals and the forces that shape the design

The architecture serves four goals, in this order when they conflict:

1. **The laws hold.** Every flow has two sides, every total is exact, every decision reads its own party's state,
   chance is seeded, looking changes nothing (Part I of the spec).
2. **The budget holds** (N8): one business day per turn in at most 1 s median and 2 s worst on a Pixel 11 Pro,
   sustained over a simulated year, within 3 GB resident memory and 4 GB of saves, with hundreds of millions of
   people and millions of small firms.
3. **Modularity.** To change how a system works, you change that system's crate, and nothing else. Adding a system
   is one new crate and one registration line.
4. **No drift.** The code, this document, the implementation plan and the spec stay true to one another because
   machines check it, not because people remember.

The forces:

- **Scale.** About 300 million people in 120 million households, 5 million incorporated small firms, 12 million
  household businesses, tens of thousands of large firms and institutions. Nothing may touch every person every
  day; the population is carried in cells (REP), and cost follows events (N8.6).
- **Memory.** The population's representation dominates memory. Every store has a byte budget (§11), and the
  layout is chosen for bytes first, then for cache behaviour.
- **Parallelism.** A phone has one or two fast cores and several medium ones, throttled when hot. The day is
  written as data-parallel passes over flat arrays, with writes gathered and applied in a declared order, so that
  a result never depends on how threads interleave.
- **Exactness.** Money and physical units are integers; every conservation identity holds exactly (Law 7).
- **Many systems, one world.** 52 systems interact through shared state — money, holdings, contracts, prices,
  public records — never by calling each other.

---

## 2. Technology choices

| Concern | Choice | Why |
| --- | --- | --- |
| Engine language | **Rust** (stable, pinned in `rust-toolchain.toml`, edition 2024) | Control of layout and allocation, no garbage-collector pauses, data-race freedom for parallel passes, first-class Android (NDK) and Linux targets, crates as compiler-enforced module boundaries. |
| Parallelism | **rayon** thread pool, used only through the kernel's pass helpers | Work-stealing over chunks; deterministic results come from the gather-and-apply discipline (§6), not from the scheduler. |
| Hashing | **hashbrown** with a fixed, keyed hasher (foldhash with constant seeds) | Fast, and iteration order is a function of the build only; world code never iterates a hash map where order matters (§13). |
| Randomness | **Own counter-based generator: Philox4x32-10** (`phx-rand`) | Every draw is addressable by (stream, entity, day, index): parallel, order-free and reproducible (CHN.1, CHN.6). Samplers are own code (binomial BTPE and inversion, hypergeometric H2PE, multinomial by conditional binomials, geometric, normal, log-normal, Pareto, Gumbel). |
| Compression | **zstd** (level 1 for snapshots, streamed) | Fast on phones, good ratio on columnar integer data. |
| Small collections | **smallvec** | Inline storage for the short per-cell lists that dominate the population. |
| Data files | **TOML** for declared data (primitives, calendars, conventions, opening distributions), read once at assembly with **serde** | Human-readable, diffable, versioned; never on a hot path. |
| Android bridge | **UniFFI** | Generates the Kotlin bindings; one Rust crate (`phx-ffi`) is the only foreign surface. |
| Android build | **cargo-ndk** + **Gradle** (Android Gradle Plugin) | Builds `libphx_ffi.so` for `arm64-v8a` and packages it. |
| User interface | **Kotlin + Jetpack Compose** | Native, fast to build, and nowhere near the hot path. |
| Command line | **clap** in `phx-cli` | Headless runs, benchmarks, reference runs, reports. |
| Law checker | **Own Rust tool** `phx-check` (uses `syn` to parse Rust, `cargo metadata` for the crate graph) | The drift guards of §13 as a program that fails the build. |
| Continuous integration | **GitHub Actions** | Linux runners for checks, tests and live-world runs; an Android job for the app build. |

External crates are an allow-list in `phx-check` (§13): adding one is a decision recorded here.

---

## 3. Layers and crates

```text
                       ┌──────────────────────────────────────────┐
  L4 apps              │ phx-cli   phx-ffi → android/   phx-check  │
                       └──────────────────────────────────────────┘
                       ┌──────────────────────────────────────────┐
  L3 assembly          │ phx-world (registry, day pipeline, GEN,   │
                       │            saves, player, metrics)  phx-obs│
                       └──────────────────────────────────────────┘
                       ┌──────────────────────────────────────────┐
  L2 systems           │ sys-pop sys-hh sys-frm ... sys-sta (39)    │  ← depend on L1 and L0 only
                       └──────────────────────────────────────────┘
                       ┌──────────────────────────────────────────┐
  L1 kernel            │ phx-core phx-store phx-geo phx-ledger     │
                       │ phx-pop phx-market phx-val phx-audit      │
                       └──────────────────────────────────────────┘
                       ┌──────────────────────────────────────────┐
  L0 foundation        │ phx-num phx-rand phx-macros               │
                       └──────────────────────────────────────────┘
```

**The dependency rules** (checked by `phx-check`, §13):

- A crate depends only on crates in lower layers, and on the kernel crates below it inside L1 in the order
  `phx-core → phx-store → phx-geo → phx-ledger → phx-pop → phx-market → phx-val → phx-audit`.
- **A system crate never depends on another system crate.** Systems interact only through kernel state: money and
  holdings (`phx-ledger`), lines (`phx-ledger`, `phx-pop`), prints, marks and valuations (`phx-market`), public
  records and events (`phx-core`). The spec's "Depends on" lines describe which state a system reads, not code
  dependencies, which is why the spec's mutual dependencies (MMK and BFL, SOV and TRS) cost nothing here.
- Only `phx-world` knows every system; it knows them only through the `System` trait (§5).
- Only `phx-ffi` and `phx-cli` are entry points.

### 3.1 The kernel crates

| Crate | Spec it carries | What it owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, `Rate`, `Price` in tick units, `Missing<T>`, rounding conventions, checked arithmetic with currency and unit tags, `i128` intermediates. |
| `phx-rand` | CHN.1, CHN.6 | Philox generator, stream keys, samplers. |
| `phx-macros` | — | The `#[clause(..)]` attribute and the store-declaration macros. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | Identifiers, the calendar and day, the party directory and kind registry with kind profiles, the primitive register, hazard-process declarations, events, public records, contract violations, findings. |
| `phx-store` | SET.12, SET.15 | Structure-of-arrays tables, arenas with free lists, slot maps, column descriptors, dirty-chunk tracking, snapshot encoding. |
| `phx-geo` | GEO | Tiles, generated map, regions, zones, distances, the network graph and its capacities, deposits, hazard exposure. |
| `phx-ledger` | MON, SET, REG, ACC, L3's ranking and distribution | Accounts, instruments, holdings with pooled lots, liens, commitments, lines, instructions and batches, settlement, the net settlement system, estates' claim waterfall, statements. |
| `phx-pop` | REP | Cell tables per population kind, keys, positions, profiles, attachments, landing and buckets, kinks, the hazard and occasion scheduler, choice groups, pairing draws, tracers, promotion, tolerance control. |
| `phx-market` | MKT | The six market forms, prints, daily marks and fixings, valuations and valuers, market failure records. |
| `phx-val` | VAL | Outlook methods, the heuristic menu, performance tracking, switching, surprises and confidence, the shared computation of public-series outlooks per method (VAL.23). |
| `phx-audit` | N1 | The audit framework, incremental and rolling checks, the kernel's own families, findings reports. |

### 3.2 The assembly and observation crates

| Crate | Spec it carries | What it owns |
| --- | --- | --- |
| `phx-world` | TIME.6–TIME.8, GEN, SET.12–SET.16, N8.10, OBS.4 | The system registry, schema compilation (§5.3), the day pipeline, GEN's orchestration and balancing, saving and loading, the player's party and its action queue, run metrics. |
| `phx-obs` | OBS, REP.30 | Observer views built at the end of each turn, the inspector and participant views, portraits from tracers, the public event feed. |

### 3.3 The system crates

One crate per spec system that is not in the kernel, named `sys-<code>` (lower case). Each implements the
`System` trait (§5), declares its kinds, attributes, stores, markets, primitives, hazards, kinks and audits, and
registers its phase handlers and its opening contribution.

`sys-pop` `sys-hh` `sys-tec` `sys-frm` `sys-cap` `sys-gds` `sys-srv` `sys-frt` `sys-lab` `sys-hsg` `sys-tcr`
`sys-ene` `sys-bnk` `sys-bfl` `sys-bcp` `sys-sec` `sys-mmk` `sys-sov` `sys-crd` `sys-eqy` `sys-mna` `sys-fnd`
`sys-dlr` `sys-idx` `sys-rat` `sys-drv` `sys-drx` `sys-ins` `sys-pen` `sys-trs` `sys-tax` `sys-soc` `sys-cb`
`sys-sup` `sys-pol` `sys-fx` `sys-xb` `sys-sta`, and `sys-est`, which carries the behaviour of the estate kind
(L3: what an estate sells, where and when).

### 3.4 The applications

| Crate or project | Purpose |
| --- | --- |
| `phx-cli` | `phx run` (the live world, headless), `phx bench` (timed runs with per-phase profiles), `phx reference` (the weight-one reference run, §10.3), `phx compare` (play resolution against reference), `phx report` (HTML reports of a run: audit, liveness, measures, findings), `phx experiment` (interventions and knock-outs, N4, N6). |
| `phx-ffi` | The engine as a library for Android: create or load a world, step it on its own thread, read views, submit player actions, save. |
| `android/` | The Gradle project: the app (Compose UI) and a `bench` flavour that runs a declared number of turns headless and writes a JSON report. |
| `phx-check` | The law checker and document checker (§13). |

### 3.5 The repository

```text
Cargo.toml                 workspace
rust-toolchain.toml        pinned toolchain
.cargo/config.toml         target features for the phone and release profiles
crates/foundation/         phx-num phx-rand phx-macros
crates/kernel/             phx-core phx-store phx-geo phx-ledger phx-pop phx-market phx-val phx-audit
crates/assembly/           phx-world phx-obs
crates/systems/            sys-*
crates/apps/               phx-cli phx-ffi phx-check
android/                   the app and the bench flavour
data/                      declared data: primitives, calendars, conventions, opening distributions, map settings
perf/                      baselines for the performance ratchets (§12.4)
docs/                      spec, ARCHITECTURE.md, IMPLEMENTATION.md
.github/workflows/         ci.yml, android.yml, nightly.yml
```

---

## 4. The data model

### 4.1 Identity

- A **party** has a `PartyId(u64)`, issued monotonically and never reused (PTY.1, PTY.13). The party directory
  maps it to its kind and its **slot** — its row in its kind's table.
- Slots are `u32` rows in structure-of-arrays tables. Slots are reused through free lists; identities never are.
  References inside the hot stores use slots; references that outlive a party (records, events, history) use
  identities.
- Instruments (`InstrumentId`), lines (`LineId`), markets (`MarketId`), accounts (`AccountId`), tiles, zones,
  regions and countries have dense `u32` identifiers of their own.
- Every identifier type is distinct at compile time. A slot is never confused with an identity.

### 4.2 Kinds and kind profiles (Law 10)

Every party, instrument, line and market has a **kind** registered at assembly with a **profile**: declared data
that mechanisms read instead of branching on the kind. A party kind's profile states its legal form (PTY.4), what
it may hold, its decider (a rule set, or the player), how it can end, which population table carries it, and its
representation profile (§4.4). An instrument kind's profile states its family and terms schema (REG.5–REG.10). A
line kind's profile states its roles, its terms schema, which side's events are frequent (§4.6), and its payment
schedule. A market kind's profile states its form, meeting days, settlement convention and participants (MKT.1).

### 4.3 Stores

All world state lives in **stores** registered with `phx-store`: structure-of-arrays tables, each column a typed
`Vec`, chunked in 64 KiB pages with a dirty bit per page. A store is owned by exactly one crate, which is its one
writer (Law 4); other crates read it only through the owner's published views. Every store is snapshotted (§9); a
store that is not registered cannot exist, because world code has no other place to keep state (`phx-check`
forbids statics and thread-locals in world crates).

### 4.4 The population tables (REP)

Each population kind (household; household running a business; small firm) has a **cell table** in `phx-pop`,
whose schema is compiled at assembly from what the systems declare (§5.3):

| Column group | Layout | Notes |
| --- | --- | --- |
| identity | `id: u64`, `weight: u32`, `flags: u16` | Flags: individual, player, tracer present, dirty, ended. |
| key | packed `u64` (or `u128` where declared) | Bit fields assembled from every system's key attributes (REP.19). |
| bucket | `u64` | The landing bucket: hash of key and per-position bucket indices (§7.2). |
| positions | one column per declared position | Money and quantity positions as `i64` totals; ratio positions (outlooks, shares) as `f32` member means. Read-positions (cash, wealth) are not stored: they are read from the ledger (REP.20). |
| profiles | `(offset u32, len u16)` into a profile arena | Entries `{role u8, group u8, value u16, count u32}`; counted jointly within a group, independently across groups (REP.32). |
| attachments | `(offset u32, len u16)` into an attachment arena | Entries `{line u32, role u8, count u32}` (REP.3). |
| accounts | `(first u32, len u8)` into the account store | A cell holds one deposit account per bank its members use (§4.5). |
| holdings | `(offset u32, len u16)` into the holdings arena | Instruments held, with pooled average cost (REP.8). |

Individuals of these kinds — the promoted and the player — are rows of weight one flagged `individual`: the same
table, the same code (REP.2). Individual large firms and every institution live in their system's own tables with
richer state; the firm decision code is written once against a `FirmView` trait implemented by both the firm cell
table and the individual firm table (Law 10 without duplication).

### 4.5 Money, holdings and contracts (MON, REG, ACC)

- **Accounts** (MON.2): `{holder slot, holder kind, issuer, currency, balance i64}`. A cell's balance is a total
  for all its members at that issuer; its members' cash position is the sum of its balances over its weight.
- **Holdings** (REG.1): `{holder, instrument, quantity i64, pooled cost i64}` for cells; individuals keep lots
  (REG.1) in a lot arena.
- **Instruments** (REG.3–REG.7): issuer, family, terms (interned), issued amount.
- **Liens and commitments** (REG.2, REG.10): side tables keyed by holding and by party.
- **Lines** (REP.3, REG.8, REG.14): `{kind, terms (interned), side A, side B}`. Each side is a short list of
  `{party slot or cell slot, count}`. A line is one record with its sides; both the cell's attachment and the
  line's side entry are written by `phx-ledger`'s line operations, which are the one writer.
- **Reverse index policy**: every line kind declares whether events arrive often from the far side (employment:
  a firm closes; tenancy: a landlord sells). Those kinds keep the far-side lists sorted for direct lookup; the rest
  are found by a scan of the attachment arena when their rare event happens (a bank failing), which is cheap once
  and saves hundreds of megabytes (§11).

### 4.6 Prices and public records (MKT, OBS.1, STA)

- **Prints** (MKT.2): `{market, instrument, day, price, qty, form}` in a day buffer; each market's **daily mark**
  (MKT.12) and volume are kept as history; individual prints are released after the day (SET.13).
- **Valuations** (MKT.20): `{position, valuer, method, value, inputs' ages}`, never mixed with prints.
- **Public records**: published statistics with revisions, reports and filings, ratings, policy decisions, and
  events (OBS.1, OBS.3), each with its publication day. Deciders read public state only through these (Law 12).

### 4.7 Numbers (NUM, Law 7, Law 8)

- Money: `i64` smallest units, tagged with a currency; products and shares through `i128`, rounded by the
  contract's or law's convention (MON.16) with the residue landing on the declared party.
- Physical quantities: `i64` in each good's smallest unit.
- **Posted terms are integers in their tick units**: prices, wages, rates and fees sit on price points (REP.34), so
  contract terms are exact.
- Continuous quantities (outlooks, probabilities, tastes, valuations): `f64` in computation, `f32` where stored per
  cell.
- `Missing<T>` for anything that can be absent; no default stands in for it (NUM.8).
- Contract violations (II.5) are raised by `violation!(clause, ..)`, which stops the run with its clause, its
  party and its day.

---

## 5. The kernel–system boundary

### 5.1 The `System` trait

```rust
pub trait System: Send + Sync + 'static {
    fn code(&self) -> SystemCode;                    // "HH", "BNK", ...
    fn declare(&self, d: &mut Declarations);         // kinds, attributes, stores, markets, line kinds,
                                                     // primitives, hazards, kinks, audits, views
    fn open(&self, o: &mut OpeningCtx);              // its part of the opening world (GEN)
    fn phases(&self, p: &mut PhaseTable);            // handlers per day stage, in declared order
}
```

Registration is one line per system in `phx-world/src/systems.rs`. The order of that list is the order of
handlers within a stage when two systems share a stage, and it is part of the world's definition.

### 5.2 What a handler may touch

A handler receives a stage context that grants exactly what the stage allows (TIME.6, Law 12):

- **reads**: its own stores; kernel stores through read views; public records up to what is published; prints
  and marks formed in earlier stages; the calendar; its declared primitives;
- **draws**: random draws keyed by stream, entity, day and index (CHN.1);
- **writes**: its own stores' rows within the chunk it was handed; and **intents** — instructions, orders and
  quotes, applications, parts (splits of cells), events — into the context's buffers.

It cannot hold a mutable reference to another crate's store, cannot see another party's private state, and cannot
read anything produced after its stage: the context has no method for it, so the compiler refuses the attempt.

### 5.3 Schema compilation

At assembly, every system's declarations are compiled into:

- each population kind's **key layout** (bit fields), **position columns**, **profile groups** and **kink sets**;
- each line kind's, instrument kind's and market kind's profile;
- the **stream registry** (names hashed to stream identifiers);
- the **primitive register**, with every value read from `data/` and checked against its declared kind and unit
  (NUM.3), missing values refused;
- the **phase table** and the **audit families**.

A system reaches its declared attributes through typed accessors generated by `declare_kind!`, so an attribute
another system declared is invisible to it unless that system published it as readable.

---

## 6. The day

### 6.1 Stages

The day runs the ten stages of TIME.6 — open, resolve, nature and population, real work, decide, form prices,
settle, fund, value and judge, close — and non-business days run the subset TIME.8 allows. A **turn** is one
business day with the non-business days before it (N8.2).

### 6.2 Passes: compute in parallel, apply in order

Every handler is written as one or more **passes**:

1. **Compute**: the kernel splits a table into chunks (4 096 rows for cells), hands chunks to the thread pool, and
   the handler reads state and writes its own rows and its intents into a chunk-local buffer.
2. **Gather**: chunk buffers are concatenated in chunk order.
3. **Apply**: the kernel applies intents — settles instructions, performs splits and landings, posts orders,
   records events — in a declared order, in parallel where the targets are disjoint.

Because every draw is keyed and every gather is in chunk order, a run is reproducible on one build and device
whatever the thread count on that device (N5). No lock guards world state; no atomic counter decides an outcome.

### 6.3 Settlement (SET, MON.5)

Instructions are columnar batches: `{reason, currency or unit, legs: [(account or holding, amount)]}`. Settlement
in stage 7:

1. checks every payer leg against free balance in the declared order (SET.6), in parallel across disjoint account
   partitions, marking fails (SET.3);
2. applies the rest, crediting and debiting in parallel by partition;
3. nets each bank's reserve movements over each batch through the net settlement system (MON.5), then settles
   large-value payments gross;
4. runs the ring pass (TIME.6 stage 7).

Physical transformations (SET.9) are checked against their recipe, deposit or event record by the same pass.

### 6.4 The population's daily cycle (REP)

For each population kind, a day's cycle is:

1. **draw** hazard hits and occasions per profile value (REP.7, REP.21) — a vectorised pass over profile entries
   with a daily count, or a scheduled next hit for processes declared rare;
2. **decide**: continuous decisions for every cell with work that day; lumpy decisions for the members with
   occasions (REP.5);
3. **meet**: choice groups at market meetings (REP.37);
4. **split**: members whose state now differs become parts (REP.8);
5. **land**: parts are bucketed and joined to cells (§7.2);
6. **re-bucket**: cells whose positions moved are re-bucketed and, where two now share a bucket, joined.

---

## 7. The population engine (REP)

### 7.1 Cells and weights

A cell is a row; its weight changes only through entry, death, split, landing, promotion and demotion (REP.17).
Totals are exact integers; a member's share of a total is computed on demand (REP.9).

### 7.2 Buckets and landing

- For each position, the bucket index is `(k, j)`: `k` is which interval between the position's **kinks** the
  value lies in (kinks from every rule that applies to the cell's key, REP.8), and `j` is the index of the value
  inside that interval on a log scale whose step is the position's **tolerance**, measured on the member's own
  scale (REP.20).
- A cell's bucket is a hash of its key and its bucket indices. A part lands in the cell with its bucket, or
  becomes a new cell (REP.8). Two members in one bucket are within tolerance and on the same side of every kink by
  construction.
- Landing adds weights, totals, profile entries (sort-merge), attachments (sort-merge) and holdings (pooled cost).
- **Tolerance control** (REP.28): each position of each kind has a tolerance level; the controller widens the
  level where the representation's own estimate of the decision gap is smallest when the cell count exceeds its
  budget, and narrows it where the gap is largest when there is room.

### 7.3 Profiles, attachments and pairing draws

Profiles and attachments are sorted short lists. A split divides them: exactly where members all hold the same,
and otherwise by multivariate hypergeometric draws within each role, independently across roles (REP.23).
Far-side events on lines with a reverse index look up the sides directly; the rest scan.

### 7.4 Hazards and occasions

Every hazard and occasion is declared (CHN.2, REP.21) with its rate function over key and profile, and its scheme:
daily counts (the default, vectorised) or next-hit scheduling in a day-bucketed calendar queue (for rare
processes). Both give the same distribution (REP.7).

### 7.5 Choice groups

At a market meeting (REP.37), buying cells are grouped by (choice probabilities, bucket); a group draws its counts
per seller alternative once; each cell pays its own budget and receives its share. A cell of identical sellers is
as many alternatives as its weight, and the counts reaching each of its members are drawn, so its members part
company by their sales (REP.22).

### 7.6 Tracers, promotion, the reference run

- Tracers (REP.30) are observer-side records: a member's cell, its profile values and its history; updated at
  splits by draws from the observer's stream; they never write to the world.
- Promotion (REP.29) reads ranks on a declared day each month.
- **The reference run** is the same code with tolerances set to zero and no cell budget: every household and small
  firm a cell of weight one (PTY.12). It needs a large machine (§10.3), and nothing else changes.

---

## 8. Markets, valuation and expectations

- `phx-market` implements the six forms once each (MKT.3–MKT.8): call auctions (schedules to a clearing price and
  rationing), the continuous book (arrival by lot, price-time priority, closing auction), the dealer market
  (quotes and requests for quotes), posted prices (sellers' prices and capacities, buyers' choice groups, rationing
  by lot), bilateral quotes and negotiation, and administered facilities. A system declares a market by kind and
  instance; the mechanism is the kernel's.
- Marks and fixings are computed after the stage that forms prices (MKT.12); valuations (MKT.20) are computed by
  valuers — parties whose methods are registered by the system that owns them (a clearing house's settlement price
  by `sys-drv`, an appraiser by `sys-hsg`).
- `phx-val` computes public-series outlooks once per method each day and caches them (VAL.23); each party's own
  outlooks are its own state: positions for cells, fields of their tables for individuals.

---

## 9. Persistence

- A **save** is a directory: a manifest (format version, build identifier, primitive-register hash, seed, day) and
  one file per store, each a sequence of zstd frames of 1 MiB column pages.
- **Full snapshots and increments** (SET.12): an increment writes only pages whose dirty bit is set since the last
  full snapshot; compaction merges them into a new full snapshot on a declared cycle.
- **Saving is not a turn** (N8.10): it runs on a background thread between turns over pages frozen by
  copy-on-write at page granularity; a turn that must modify a page not yet written copies it first. The copies'
  memory is part of the 3 GB budget (§11).
- Loading restores the state exactly (SET.15). Nothing is replayed.

---

## 10. Observation, the player, and the three ways the world is run

### 10.1 Observer and player

- At the end of each turn `phx-obs` builds immutable **views** — dashboards per country, markets, news, the
  player's own party, the inspector's tables — and publishes them behind an `Arc` swap; the interface reads views,
  never the engine (Law 17). Portraits come from tracers (OBS.8).
- The **player** is a party whose kind profile names the player as its decider. Actions are queued through
  `phx-ffi` and applied in the next decide stage (OBS.4).

### 10.2 On the phone

`phx-ffi` runs the engine on its own thread with a worker pool sized to the fast and medium cores, and exposes:
create a world (seed, settings), load, step one turn, read views, submit actions, save. The Compose interface has
a participant mode and an inspector mode (OBS.2).

### 10.3 Headless and reference

- `phx run` runs the real world at the play resolution on Linux: the live world, used at every step (§12).
- `phx reference` runs the weight-one world. Its budget: about 40–60 GB of memory and many cores (a large cloud
  machine, provisioned by the owner when a stage's gate needs it).
- `phx compare` reads both runs' declared reads and reports the differences against the declared accuracy (N8.5).

---

## 11. The budgets

### 11.1 Memory (3 GB resident, N8.4)

Design targets, verified at the Stage 0 and Stage 1 gates; a store over budget is a finding before the stage can
end.

| Store | Count (design) | Bytes each | Budget |
| --- | --- | --- | --- |
| Household cells, core columns | 3.0 M | 128 | 384 MB |
| Household profiles | 3.0 M × ~30 entries | 6 | 540 MB |
| Household attachments | 3.0 M × ~15 | 8 | 360 MB |
| Reverse indexes (employment, tenancy) | ~20 M entries | 6 | 120 MB |
| Firm cells and household businesses, all columns | 1.5 M | 220 | 330 MB |
| Lines | 4.0 M | 40 | 160 MB |
| Individuals and institutions (firms, banks, funds, insurers, schemes, the state) | ~0.1 M | ~1 KB | 100 MB |
| Instruments, holdings of individuals, liens, commitments | — | — | 150 MB |
| Markets, prints of the day, marks and fixings history | — | — | 80 MB |
| Map, network, deposits | ~1 M tiles | ~32 | 40 MB |
| Day buffers (intents, parts, instructions) | — | — | 250 MB |
| Snapshot copy-on-write pages and compression buffers | — | — | 150 MB |
| Views and tracers | — | — | 60 MB |
| Headroom | | | 276 MB |
| **Total** | | | **3 000 MB** |

### 11.2 Time (1 s median, 2 s worst, N8.2)

Budgets per business day at the play resolution on the target phone, for the complete world at Stage 7. Each
stage's steps set their own share and the ratchets (§12.4) hold them.

| Work | Median day | Worst day |
| --- | --- | --- |
| Hazards and occasions | 80 ms | 120 ms |
| Household decisions and market meetings | 120 ms | 200 ms |
| Firm decisions, production and pricing | 120 ms | 200 ms |
| Labour search and matching | 50 ms | 100 ms |
| Splits, landings and re-bucketing | 100 ms | 180 ms |
| Payroll, rents, mortgages, benefits, taxes (dated flows) | 40 ms | 250 ms |
| Settlement and the funding stage | 60 ms | 150 ms |
| Financial markets, banks, funds, insurers, the state | 120 ms | 250 ms |
| Valuation, marks and accounts | 40 ms | 120 ms |
| Audit (incremental) and statistics | 60 ms | 150 ms |
| Views | 20 ms | 30 ms |
| Reserve | 170 ms | 250 ms |
| **Total** | **1 000 ms** | **2 000 ms** |

---

## 12. Testing, the live world and measurement

### 12.1 Three kinds of evidence

1. **Compile level**: the types refuse the defect — distinct identifier types, currency-tagged money, stage
   contexts that cannot reach later state, `Missing<T>` that must be handled.
2. **Logic level**: pure functions over values — samplers, rounding, auction clearing, waterfalls, landing
   arithmetic, bucket indices — tested with `cargo test`.
3. **The live world**: `phx run` on the real world as it exists at that step, with the audit (N1), the liveness
   reads (N2), the step's own live checks and the budgets. No test builds a world of its own.

### 12.2 The live world at every step

Every step in `IMPLEMENTATION.md` names its **live checks**: what the running world must show once the step is in.
CI runs the live world on every change (§12.3), so a later step cannot silently break an earlier step's checks:
every live check stays in the suite forever.

### 12.3 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format, clippy with warnings as errors, `cargo test`, `phx-check`, a release build, **the live world** for 60 days at play resolution with audit, liveness and every live check, the CI performance ratchet |
| `android.yml` | every push to `main` | the app and the bench flavour for `arm64-v8a` |
| `nightly.yml` | nightly | the live world for two simulated years; the report as an artifact |

### 12.4 Performance and memory ratchets

- `phx bench` records per-phase times and per-store bytes. `perf/baseline-ci.json` holds the last accepted values
  on the CI machine; a change that is slower or larger than the baseline beyond the declared noise band fails CI
  until the baseline is raised in its own commit, with its reason.
- The **device gate** at the end of every stage (N8.8): the bench flavour runs a simulated year on the phone and
  reports median and worst turn, per-phase times, peak memory and thermal state; the owner runs it and commits the
  report to `perf/device/`.

---

## 13. Drift guards

`phx-check` runs in CI and fails the build on any of these:

1. **Layering**: the crate graph obeys §3; no system crate depends on another; the external crate allow-list.
2. **Clause map**: every spec clause is assigned to a step in `IMPLEMENTATION.md`; every clause of a step marked
   done has a `#[clause(..)]` attribute on the code that carries it; every clause named in code exists in the spec.
3. **The code rules** of `CLAUDE.md` as patterns in world crates (every crate except `phx-cli`, `phx-ffi`,
   `phx-check`): no `min`, `max` or `clamp` on world numbers outside `phx-num`'s audited helpers; no numeric
   literals beyond 0, 1, −1 and 2 in system crates; no floating type holding money; no `std::time`, no unseeded
   randomness, no `static mut`, no `thread_local`, no `println!`; no iteration over a hash map in world code; no
   `unwrap_or(0)`-style numeric defaults; no comparison against a kind identifier in a system crate.
4. **Registration**: every store is registered; every stream, hazard, kink, market and primitive a crate uses is
   declared by it.
5. **Documents**: the coverage table (§15) is regenerated from the clause map and must match the file; the step
   list in `IMPLEMENTATION.md` is well formed (every step has its sections and a status).
6. **Ratchets**: performance and memory (§12.4).

A rule is changed only in the same change as the reason for it, recorded in this document.

---

## 14. Risks and how the design meets them

| Risk | Where it would show | Response built in |
| --- | --- | --- |
| The population does not compress enough | cell counts at Stage 0 and Stage 1 gates | tolerance control; the attribute classes are declarations and can move (profile ↔ key) without code change; memory table per store |
| The compressed world is not accurate enough | `phx compare` against the reference run from Stage 1 | the declared accuracy gate (N8.5); decision gap and dispersion erased reported per position (REP.15) |
| A phone throttles | the device gate's thermal reading | the budget is set against sustained, not burst, speed; the worker pool size is a setting |
| Month-ends and election days exceed 2 s | the worst-day column of the device gate | dated flows are columnar batches; campaign intentions spread the vote (POL.4); filing windows spread returns (TAX.2) |
| Modules leak into each other | `phx-check` layering | systems cannot name each other's crates |
| The documents drift from the code | `phx-check` clause map and coverage | CI fails |

---

## 15. Coverage

Generated by `phx-check coverage` from the clause map; edited only through it. **Status**: planned, building,
done (every clause of the system annotated in code and its "Done when" met at the stage's gate).

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-core`, `phx-world` | 0 | 0 | planned |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 1 | planned |
| A3 | NUM | `phx-num`, `phx-core` | 0 | 0 | planned |
| A4 | CHN | `phx-rand`, `phx-core` | 0 | 0 | planned |
| A5 | GEO | `phx-geo` | 0 | 0 | planned |
| A6 | REP | `phx-pop` | 0 | 1 | planned |
| A7 | GEN | `phx-world` and every system's opening | 0 | 7 | planned |
| B1 | MON | `phx-ledger` | 0 | 3 | planned |
| B2 | SET | `phx-ledger`, `phx-store`, `phx-world` | 0 | 0 | planned |
| B3 | REG | `phx-ledger` | 0 | 3 | planned |
| B4 | ACC | `phx-ledger` | 0 | 2 | planned |
| C1 | MKT | `phx-market` | 0 | 3 | planned |
| C2 | VAL | `phx-val` | 1 | 3 | planned |
| D1 | POP | `sys-pop` | 0 | 6 | planned |
| D2 | HH | `sys-hh` | 1 | 6 | planned |
| E1 | TEC | `sys-tec` | 1 | 6 | planned |
| E2 | FRM | `sys-frm` | 1 | 2 | planned |
| E3 | CAP | `sys-cap` | 1 | 2 | planned |
| F1 | GDS | `sys-gds` | 1 | 1 | planned |
| F2 | SRV | `sys-srv` | 1 | 1 | planned |
| F3 | FRT | `sys-frt` | 1 | 5 | planned |
| F4 | LAB | `sys-lab` | 1 | 1 | planned |
| F5 | HSG | `sys-hsg` | 2 | 2 | planned |
| F6 | TCR | `sys-tcr` | 2 | 2 | planned |
| F7 | ENE | `sys-ene` | 2 | 2 | planned |
| G1 | BNK | `sys-bnk` | 1 | 3 | planned |
| G2 | BFL | `sys-bfl` | 2 | 2 | planned |
| G3 | BCP | `sys-bcp` | 2 | 2 | planned |
| G4 | SEC | `sys-sec` | 4 | 4 | planned |
| H1 | MMK | `sys-mmk` | 3 | 3 | planned |
| H2 | SOV | `sys-sov` | 1 | 3 | planned |
| H3 | CRD | `sys-crd` | 3 | 3 | planned |
| H4 | EQY | `sys-eqy` | 3 | 3 | planned |
| H5 | MNA | `sys-mna` | 4 | 4 | planned |
| H6 | FND | `sys-fnd` | 3 | 3 | planned |
| H7 | DLR | `sys-dlr` | 3 | 3 | planned |
| H8 | IDX | `sys-idx` | 1 | 3 | planned |
| H9 | RAT | `sys-rat` | 2 | 3 | planned |
| I1 | DRV | `sys-drv` | 4 | 4 | planned |
| I2 | DRX | `sys-drx` | 4 | 4 | planned |
| I3 | INS | `sys-ins` | 4 | 4 | planned |
| I4 | PEN | `sys-pen` | 4 | 4 | planned |
| J1 | TRS | `sys-trs` | 1 | 3 | planned |
| J2 | TAX | `sys-tax` | 1 | 5 | planned |
| J3 | SOC | `sys-soc` | 1 | 5 | planned |
| J4 | CB | `sys-cb` | 1 | 3 | planned |
| J5 | SUP | `sys-sup` | 2 | 2 | planned |
| J6 | POL | `sys-pol` | 5 | 5 | planned |
| K1 | FX | `sys-fx` | 5 | 5 | planned |
| K2 | XB | `sys-xb` | 5 | 5 | planned |
| M1 | OBS | `phx-obs`, `android/` | 0 | 5 | planned |
| M2 | STA | `sys-sta` | 1 | 5 | planned |
| L1–L12 | transmission chains | tested by `phx experiment` (N4) | 2 | 7 | planned |
| N1 | audit | `phx-audit` and every system's families | 0 | 7 | planned |
| N2–N8 | measurement | `phx-cli`, `phx-world` metrics, `android/` bench | 0 | 7 | planned |
