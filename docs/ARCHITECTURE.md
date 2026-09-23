# Project Phoenix — Architecture

How the world in `docs/spec/PROJECT_PHOENIX.md` is built, top down: the technologies, the layers, the channels
between systems, the data model, the day, the budgets and the guards. The spec says **what**; this document says
**how, in outline**; `docs/IMPLEMENTATION.md` says **how, step by step**. Where this document and the spec
disagree, the spec wins, and this document is fixed in the same change. Clause identifiers in brackets, such as
(REP.8), point to the spec; section signs, such as §4.2, point to this document.

---

## 1. Goals and forces

The architecture serves four goals, in this order when they conflict:

1. **The laws hold** (spec Part I): every flow two-sided, every total exact, every decision from its own party's
   state, chance seeded, looking changing nothing.
2. **The budget holds** (N8): one turn — a business day and the non-business days before it — in at most 1 s at
   the median and 2 s at the worst on a Pixel 11 Pro, sustained over a simulated year, within 3 GB resident memory
   and 4 GB of saves, with about 300 million people.
3. **Modularity**: to change how a system works you change that system's crate and nothing else; adding a system
   is a new crate and one registration line.
4. **No drift**: code, documents and spec stay true to one another because machines check it (§15).

The forces that shape everything below:

- **Scale.** About 120 million households, 5 million incorporated small firms, 12 million household businesses,
  tens of thousands of large firms and institutions. Nothing touches every person every day; the population is
  carried in cells (REP), and cost follows events (N8.6).
- **Memory is the binding budget.** The population's representation dominates it. Every store is sized from its
  declared record layout (§13.1), and the first measurement of the build is how many entries a cell really needs.
- **Memory traffic, not arithmetic, sets the time.** The household state is over a gigabyte; one sweep of it costs
  tens of milliseconds on a phone. The day is organised as a few fused traversals per table (§6.3), not one pass
  per system.
- **A phone is heterogeneous and throttles.** One fast core, several medium ones, little cores to avoid, and about
  60% of burst speed sustained. Work is chunked small, pinned to the fast and medium cores, and budgeted against
  sustained speed.
- **Exactness.** Money and physical units are integers; every conservation identity holds exactly (Law 7).
- **Fifty systems, one world.** Systems never call each other. They cooperate through kernel channels — facts,
  messages, levies, lines, markets, records (§4) — which is what makes modularity real rather than declared.

---

## 2. Technology

| Concern | Choice | Why |
| --- | --- | --- |
| Engine | **Rust**, stable, pinned in `rust-toolchain.toml`, edition 2024 | Layout and allocation control, no garbage-collector pauses, data-race freedom, Android (NDK) and Linux targets, crates as compiler-enforced boundaries. |
| World mathematics | **`libm`** (pure Rust) for every transcendental function in world code | Results that do not depend on the platform's C library, so the phone, CI and the reference machine compute the same values. |
| Parallelism | **Own thread pool** in `phx-exec`, over **rayon-core**, pinned to the fast and medium cores (`sched_setaffinity`), with Android performance-hint sessions | Deterministic fixed-size chunks, no work on little cores, fewer barriers. Only `phx-exec` may depend on rayon. |
| Hash maps | **hashbrown** with a fixed-seed **foldhash**, wrapped in a kernel map type without iteration | Fast lookups; no world outcome can depend on hash order. |
| Randomness | **Own counter-based Philox4x32-10** in `phx-rand`, batch-first samplers | Draws addressable by (stream, identity, day, index): parallel, order-free, reproducible (CHN.1, CHN.6). |
| Compression | **zstd** level 1 after per-column transforms (delta, zigzag, bit-packing) | Fast on phones; the transforms double the ratio on integer columns. |
| Declared data | **TOML** read with **serde** at assembly only | Human-readable, diffable, never on a hot path. |
| Android bridge | **UniFFI** in `phx-ffi` | Generated Kotlin bindings; one foreign surface; pages of data, never whole tables. |
| Android build | **cargo-ndk** + **Gradle**; native libraries aligned for 16 KiB pages | Required by current Android targets. |
| Interface | **Kotlin + Jetpack Compose** | Native and far from the hot path. |
| Command line | **clap** in `phx-cli` | Headless runs, benchmarks, reference runs, reports. |
| Law checker | **`phx-check`** (Rust, `syn` + `cargo metadata`) plus **clippy** `disallowed-*` lists in `clippy.toml` | Type-aware rules through clippy; structural rules through `phx-check` (§15). |
| Performance counters | **cachegrind** through `iai-callgrind` for kernel micro-benchmarks; the engine's own counters for rows, bytes, parts and landings | Deterministic numbers for ratchets; wall time only on the phone (§14.5). |
| API snapshots | **cargo-public-api** for kernel and interface crates | A kernel's surface changes only on purpose (§15). |
| CI | **GitHub Actions**: x86-64 Linux, arm64 Linux (`ubuntu-24.04-arm`) and an Android build job | arm64 runners are the proxy for the phone between device gates. |

External crates are an allow-list checked by `phx-check`; adding one is a decision recorded in §17.

---

## 3. Layers and crates

```text
L4 apps            phx-cli · phx-ffi → android/ · phx-check
L3 assembly        phx-world · phx-obs
L2 systems         sys-dem sys-hh sys-est sys-tec sys-frm ... sys-sta          (depend on L0, L1, IF only)
IF interfaces      if-pop if-firm if-labour if-property if-credit if-banking
                   if-securities if-risk if-state if-open if-energy            (data only)
L1 kernel          phx-store phx-exec phx-core phx-geo phx-ledger phx-pop
                   phx-market phx-val phx-audit
L0 foundation      phx-num phx-rand phx-id phx-macros
```

### 3.1 Dependency rules (checked by `phx-check`)

- A crate depends only on crates in lower layers. Inside L1 the order is `phx-store → phx-exec → phx-core →
  phx-geo → phx-ledger → phx-pop → phx-market → phx-val → phx-audit`, each depending only on those before it.
- **Interface crates** depend only on L0 and L1, contain only data — types, handles, schemas — and no behaviour:
  `phx-check` refuses any function with a body other than a constructor or a field accessor.
- **A system crate never depends on another system crate.** It depends on L0, L1 and any interface crates.
- Only `phx-world` knows every system, through the `System` trait (§5). Only `phx-exec` depends on rayon.
- A crate is created when its first step starts, generated by `phx-cli new-system`, never in advance.

### 3.2 The foundation

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>` with a niche, rounding conventions, checked `i64`/`i128` arithmetic; the one audited home of the arithmetic helpers world code may call. |
| `phx-rand` | CHN.1, CHN.6 | Philox, stream keys, batch samplers: binomial (inversion and BTPE), multinomial (conditional binomial, and alias tables when draws are fewer than categories), hypergeometric (H2PE), geometric, normal, log-normal, Pareto, Gumbel. |
| `phx-id` | — | Identifier types (`PartyId`, slots, `LineId`, `InstrumentId`, …), the calendar's `Day` and `Date` types. |
| `phx-macros` | — | `#[clause]`, `declare_kind!`, `declare_fact!`, `declare_store!`, `declare_message!`, `declare_system!`. |

### 3.3 The kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space (never reallocated), chunk-local arenas with compaction, slot allocators, column descriptors, save encoding. The only crate besides `phx-exec` allowed `unsafe`. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; chunked, fused traversals; gather by prefix sum; `KeyedReduce`; fixed-tree reductions; sorting (radix) utilities. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | The calendar and conventions; the party directory with tombstones; kinds and kind profiles; the primitive register and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); hazard and occasion declarations with **survival screening** (§7.3) for every table; **public records** with audiences (§4.9); events and the public-event rule; findings; contract violations. |
| `phx-geo` | GEO | Tiles, generated map, regions, zones, distances, the network and its capacities, deposits, exposure. |
| `phx-ledger` | MON, SET, REG, ACC, L3's ranking | The **contract algebra** (§4.4); lines and attachments; deposits as attachments with balances (§4.5); instruments, holdings, lots, liens, commitments; **levies** (§4.3); instructions and implicit batches; settlement and the net settlement system (§6.5); **fail records** and line payment records (§4.5); transformation records (production, extraction, consumption, destruction); the estate waterfall; statements. |
| `phx-pop` | REP | Cell tables per population kind; keys (interned), positions, profiles and roles; occasion allocation; splits, parts and landing (§7); tolerance control; choice groups; tracers' world-side hooks; promotion and demotion; locality renumbering; the reference-run mode. |
| `phx-market` | MKT | The six forms (§8), prints, marks and fixings, valuations and valuers, market failures. |
| `phx-val` | VAL | Outlook methods as pure functions; public-series outlooks computed once per method per day; surprise and confidence arithmetic. Own outlooks live where their party lives (positions or facts). |
| `phx-audit` | N1 | Audit families, streaming checks over batches as they settle, incremental and rolling checks, injection mode (§14.3). |

### 3.4 Interface crates

Each interface crate is a domain's shared vocabulary: kind names and roles, facets and fact handles, line-kind terms
built from the contract algebra, message payloads, decision-point input and output types, view schemas. **Every
item names the one system that writes it**, and assembly checks the name (§5.4).

| Crate | Domain |
| --- | --- |
| `if-pop` | households, persons and roles, demographic facts, household decision points |
| `if-firm` | firms, products and ways, production facts, pricing decision points |
| `if-labour` | employment terms, vacancies, applications, offers, separations |
| `if-property` | dwellings, land, tenancies, mortgages' collateral, appraisals |
| `if-credit` | loan terms, applications and quotes, the credit bureau's records, trade credit |
| `if-banking` | deposit terms; bank facts (marginal cost of funds, capital, liquidity); resolution |
| `if-securities` | bonds, shares, fund units, reports, filings, ratings, indices, orders |
| `if-risk` | derivative, insurance and pension terms; margin demands; claims |
| `if-state` | policy values, levies, benefits, public agencies, budgets, elections |
| `if-open` | currencies, exchange-rate regimes, trade, migration |
| `if-energy` | power products, the grid, dispatch |

### 3.5 Systems

One crate per spec system not in the kernel, named `sys-<code>`: `sys-dem` (the spec's POP; renamed so it is not
confused with `phx-pop`), `sys-hh`, `sys-est` (estates, L3), `sys-tec`, `sys-frm`, `sys-cap`, `sys-gds`,
`sys-srv`, `sys-frt`, `sys-lab`, `sys-hsg`, `sys-tcr`, `sys-ene`, `sys-bnk`, `sys-bfl`, `sys-bcp`, `sys-sec`,
`sys-mmk`, `sys-sov`, `sys-crd`, `sys-eqy`, `sys-mna`, `sys-fnd`, `sys-dlr`, `sys-idx`, `sys-rat`, `sys-drv`,
`sys-drx`, `sys-ins`, `sys-pen`, `sys-trs`, `sys-tax`, `sys-soc`, `sys-cb`, `sys-sup`, `sys-pol`, `sys-fx`,
`sys-xb`, `sys-sta`.

### 3.6 Assembly and applications

| Crate or project | Owns |
| --- | --- |
| `phx-world` | The registry and schema compilation (§5.3); the day's stages and sub-steps (§6); GEN's phases, canonical drawing, balancing, opening history and settling (§10); saving and loading (§11); the player's decider (§12.2); metrics. |
| `phx-obs` | Views, portraits and tracers (REP.30, OBS.8), the inspector and participant views; it only reads. |
| `phx-cli` | `run`, `bench`, `reference`, `compare`, `ladder`, `seeds`, `inject`, `experiment`, `report`, `new-system`, `dump-registry` (§14). |
| `phx-ffi` | The engine as an Android library. |
| `android/` | The Compose app and its `bench` flavour. |
| `phx-check` | The law, layering and document checks (§15). |

### 3.7 The repository

```text
Cargo.toml · rust-toolchain.toml · clippy.toml · .cargo/config.toml · CODEOWNERS
crates/foundation/  crates/kernel/  crates/interfaces/  crates/systems/  crates/assembly/  crates/apps/
android/            the app and the bench flavour
data/               primitives, policy values' opening values, calendars, conventions, opening distributions, map
perf/               ratchet baselines (CI counters) and device reports
docs/               spec, ARCHITECTURE.md, IMPLEMENTATION.md
.github/workflows/  ci.yml · arm.yml · android.yml · nightly.yml
```

---

## 4. The channels between systems

Systems never call each other and never read each other's stores. Everything they must share goes through one of
these kernel channels, each with one writer per fact (Law 4) and declared audiences (Law 12).

### 4.1 Facts

A **fact** is a named, typed attribute of parties of declared kinds — a bank's marginal cost of funds, a firm's
output price, a household's credit-record stage — declared once in an interface crate with:
its type and unit; the party kinds it applies to; **exactly one writer** (a system, or a placeholder SHAPE primitive
naming the system that retires it, NUM.7); its **audience** — the party itself, a named authority, or public after a
lag; and, for cells, its representation class (key, position or profile, REP.33).

Facts are compiled into storage at assembly: columns of the cell tables for population kinds, **facet tables** for
individual kinds. Writing needs a token only the writer crate can construct; reading needs a handle and passes the
audience check of the reader's context (§4.9). Assembly refuses a fact with no writer or two. Retiring a
placeholder changes only its writer.

### 4.2 Messages

A **message** is an addressed record: kind (payload type from an interface crate), sender, addressee (a party, or a
line side with a count for cells), issued day, due day, the line or instrument it concerns, and state (open,
answered, lapsed, failed). Applications and quotes (MKT.7), notices (REP.21), demands and calls (TIME.7),
redemption requests (FND.6), capital calls (FND.8), claims (INS), votes (POL.4) and resolution notices all travel as
messages.

Every message kind declares, for each addressee kind, the **answering system**; assembly refuses a kind that can
reach a party kind nobody answers. A message addressed to cells becomes **notice occasions** for the counted
members (REP.21). A message that demands money opens a **commitment** (REG.10), so an unmet demand becomes a fail
routed to its owner (§4.5). Messages live across days in a snapshotted store (SET.16).

### 4.3 Levies

A **levy** is a declared deduction or addition on another system's flows: income tax and contributions withheld at
payroll, value-added tax charged at a sale, import duty at the border, pension contributions. It declares: the flow
reasons it applies to; its base (a leg's amount or a term's field); its schedule (a policy value, §4.6), which is
piecewise linear between declared kinks — registered with `phx-pop` so no landing crosses them; who remits it and
to whom; whether it is the collector's liability until remitted (TAX.2); and its order among levies. `phx-ledger`
composes levies into the instruction that carries the flow, so the wage payment, the withholding and the
contribution settle together. A rule data cannot express may register a calculator function, with the reason in
§17.

### 4.4 The contract algebra and lines

Every contract's terms are a composition of **generic legs**: a fixed amount; a rate on a notional (fixed or
floating over a named transacted reference, with day count and resets, TIME.4); an amount per unit of time; an
indexed amount (a published index with its lag); an amount contingent on a named event; a delivery of units. A
**schedule** places legs on dates by the calendar (TIME.3); **seniority** and **collateral** are data. Accrual
(ACC.1), the listing of dues (TIME.6 stage 1), payment, arrears and waterfalls (L3) are therefore generic kernel
code; line kinds and instrument kinds are declarations in interface crates (REG.5–REG.10, BNK.17, DRV.1, INS.1,
PEN.2).

A **line** (REP.3) is one record: its kind, its interned terms, and two sides. A side lists parties or cells with
their counts. The line's kind profile declares which sides keep a **far-side list** (employment, tenancy, and any
kind whose far-side events are frequent); others are found by a scan of the attachment arena when their rare
far-side event happens.

### 4.5 Attachments, deposits, fails and payment records

- A cell's **attachment** is `{line, role, count}` in the cell's attachment list; an individual's is the same with
  count one. For line kinds whose flows are dated, the attachment also carries a **cached schedule class and per
  member amount**, written only by `phx-ledger`'s line operations whenever terms change, so dated flows need no
  random access to line terms.
- **Deposits are attachments with balances.** A deposit line is (bank, deposit terms); a cell's attachment to it
  carries `balance`, the total held by the members attached. A member's share is `balance / count`. Members share
  their cash position (REP.20), so a cell's per-bank balances stay proportional to their counts, and payments debit
  each attachment in proportion. Deposit insurance, bank failure and interest all read the attachment. There is no
  second account record.
- **Fails** (SET.3) become fail records naming the instruction's reason and the line or instrument it concerns;
  stage 2 of the next business day delivers them to the declared owner of that line kind, which applies what the
  contract says (arrears, penalty, close-out).
- Every line side entry keeps a generic **payment record** — payments made, missed, days in arrears — maintained by
  `phx-ledger` and combined at landing by a declared rule; lenders' and suppliers' views read it (REP.8).

### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run: a tax band, a benefit rule, a policy
rate, a capital ratio, a macroprudential cap. Its opening value comes from `data/` (ENDOWMENT); after that it is a
fact whose one writer is the owner's decision handler, or a declared intervention on a copy of the world (N6). Each
change writes a dated **announcement** record with its effective day, which is at least the next business day
(VAL.6, POL.7). Primitives that no party owns — technologies, preferences — are read-only after assembly.

### 4.7 Decision points and deciders

Every decision the spec names is a **declared decision point**: an input view type and an output intent type from
an interface crate, a rule function registered by its system, and a pure evaluation form usable off-world (for the
decision gap, REP.15). Which function decides is a per-party **decider** fact: the kind's rule set, or the player.
The kernel dispatches; no system branches on who decides. The player's queued action replaces the rule's output for
that decision on that day; on every other decision the rule decides (OBS.4, OBS.6).

### 4.8 Keyed reductions

`KeyedReduce<K, V>` gathers per-chunk sorted partial results — a bank's defaults, a seller's sales, a fund's flows —
and merges them in chunk order into the owner's rows at the apply step. Integers merge exactly; floating values
merge along a fixed tree. It is the only way a pass sums over rows it does not own.

### 4.9 Records, audiences and scoped reads

Every read goes through a context that knows **whose decision it is**. A decision kernel reads through
`ctx.party(row)`, which offers: the party's own rows and facts; the lines, attachments and accounts it is a side or
issuer of — so a bank sees its own depositors' balances and no other bank's; records whose audience includes it
(public, or filed with it as the named authority: supervisory returns, credit-bureau records, tax filings, survey
samples); and prints and marks formed earlier (Law 12, PTY.8, VAL.18). Store-wide views are given only to processes
and to apply steps, never to decisions. In debug builds a read tracer checks every read against its scope during the
CI live run (§14.3).

### 4.10 Public events

The declared rule of what becomes a public event (OBS.3) runs inside the day, in stage 10's first sub-step, reads
the day's state, and writes events into `phx-core`'s public records, from which deciders read them the next day.
`phx-obs` displays them and writes nothing.

---

## 5. The kernel–system boundary

### 5.1 The `System` trait

```rust
pub trait System: Send + Sync + 'static {        // implemented by zero-sized types only (asserted)
    const CODE: SystemCode;                       // "HH", "BNK", ...
    fn declare(d: &mut Declarations);             // kinds, facts, stores, lines, messages, levies, markets,
                                                  // primitives, hazards, occasions, kinks, decision points,
                                                  // audits, metrics, views, opening contributions
    fn handlers(h: &mut HandlerTable);            // per sub-step: kernels with their declared reads and writes
}
```

Declarations generate **typed handles** whose constructors are private to the declaring crate; code that uses an
undeclared stream, fact, store or primitive does not compile, and a declared handle nobody uses fails the
`dead_code` lint. Registration is one line per system in `phx-world/src/systems.rs`, and its order carries no
meaning (§6.2).

### 5.2 Handlers

A handler is a kernel function over chunks of one table, declared with its **sub-step** (§6.1), the facts and
columns it **reads** and **writes**, the messages and intents it may emit, and the streams it draws from. A handler
receives a context granting exactly its declarations: its own rows within the chunk, scoped reads (§4.9), keyed
draws, and intent buffers. It can hold no reference to another crate's store.

### 5.3 Schema compilation

At assembly, declarations compile into: each population kind's key layout (interned key records), fact columns,
profile groups and roles, and kink sets; facet tables for individual kinds; line, instrument, message and market
kind profiles; levies and their order; the stream registry; the primitive register with every value from `data/`
checked against its kind, unit and owner (NUM.3); the handler graph (§6.2); audit families and metrics. A key field
whose value outgrows its declared width is a **contract violation** that stops the run and calls for a layout
change: nothing saturates (Law 6).

### 5.4 Assembly refusals

Assembly stops with a named reason on: a fact with no writer or two; a message kind with an unanswered addressee
kind; two handlers in one sub-step where one writes what the other reads or writes; a levy whose schedule has no
owner; a kink on a position nobody declares; a primitive without value, unit, kind or source; a decision point
without an evaluation form; an interface item whose writer system is not registered.

---

## 6. The day

### 6.1 Stages and sub-steps

The ten stages of TIME.6, each divided into kernel-defined sub-steps. Handlers declare their sub-step; within a
sub-step every handler reads the state as it was when the sub-step began, and intents apply when it ends.

| Stage | Sub-steps |
| --- | --- |
| 1 Open | 1a lapse orders, quotes and messages · 1b list today's dues and dated flows as implicit batches |
| 2 Resolve | 2a post accruals · 2b deliver yesterday's fails to owners; arrears · 2c losses land; endings; estates open and distribute · 2d apply |
| 3 Nature and population | 3a weather and catastrophes · 3b screening: hazards and occasions for every table (§7.3) · 3c demographic events (deaths, births, ageing, formation) · 3d allocate overlapping occasions (§7.4) · 3e apply: splits |
| 4 Real work | 4a production, services, shipments, construction, jobs starting and ending · 4b apply |
| 5 Decide | 5a public-series outlooks per method (VAL.23) · 5b continuous decisions (fused per table) · 5c lumpy decisions for occasion holders; institutions' decisions; player actions · 5d apply: orders, quotes, messages, splits |
| 6 Form prices | 6a every market meeting today; choice groups · 6b marks and fixings · 6c apply: matches become instructions |
| 7 Settle | 7a build batches; compose levies · 7b reserve-net check per batch with intraday credit; remove failing banks' legs · 7c payer checks in declared order · 7d apply every instruction's legs, deposit and reserve, together · 7e ring pass · 7f record fails; stream the audit over settled batches; splits |
| 8 Fund | 8a money market and facilities post · 8b form · 8c settle |
| 9 Value and judge | 9a valuations · 9b accounts and ratios · 9c tests (covenants, margins, capital, limits); demands issued, due next business day · 9d publications (reports, ratings, statistics, announcements) |
| 10 Close | 10a public-event rule · 10b landing of the day's parts; re-bucketing; tolerance control; monthly ranks and promotion; periodic locality renumbering · 10c incremental audit families · 10d views and tracers (observer, read-only) · 10e metrics |

Non-business days run 2a, 3 and 4, the retail, service and electricity meetings of stage 6, and 10 (TIME.8).

### 6.2 Order without order dependence

Assembly builds the handler graph from declared reads and writes and refuses conflicts within a sub-step (§5.4), so
registration order carries no meaning. CI proves it: shuffling the registration list gives the same world hash
(§14.3).

### 6.3 Fused traversals

For each table and sub-step, `phx-exec` runs **one traversal**: chunks of a declared size (512–1 024 rows for cells,
so a chunk's working set stays in a medium core's L2) are handed to the pinned pool, and on each chunk every
handler registered for that sub-step runs in turn while the chunk is in cache. The day budgets at most **three full
sweeps** of the large population arenas (§13.2).

### 6.4 Compute, gather, apply

Handlers write their own rows in their chunk and emit intents into chunk-local buffers. Gathers place buffers by
prefix sum in chunk order; applies run in declared order, parallel over disjoint targets. New identities and slots
are issued in gather order. Every reduction runs over a fixed chunk tree. Chunk sizes are declared per table and
never depend on the thread count, so a run is reproducible on one build and device whatever the pool size (N5), and
— because world mathematics uses `libm` and fixed reduction order — in practice across machines too.

### 6.5 Settlement

- Instructions are **explicit** (a trade, a single payment) or **implicit batches** — (line kind or market, rule,
  day) — whose legs are generated from attachments and side lists when they settle and again when audited, and never
  stored whole. Payroll, rents, mortgage payments, benefits, retail meetings and landings settle this way.
- **Semantics** (SET.4, SET.5, SET.6, MON.5):
  1. 7b: each batch's reserve net per bank is computed, the central bank's intraday credit applied (MON.3); a bank
     that still cannot cover its net has its customers' legs removed and the batch is recomputed, until nothing
     changes. Nothing has been applied yet.
  2. 7c: each payer leg is checked against the payer's free balance as it stood when the sub-step began, less the
     earlier debits in declared order; credits count from the next pass. A multi-leg instruction with a failing leg
     fails whole and releases its reservations; the check repeats until nothing changes. Instructions with several
     payers run in a serial lane.
  3. 7d: every surviving instruction applies all its legs — deposits and reserves together — atomically, sorted by
     account for locality. Applied is final.
  4. 7e: the ring pass settles what the day's credits now allow.
- Physical transformations (SET.9) are checked against transformation records (recipes written by `sys-tec`,
  extraction against the deposit, consumption and destruction against the purchase or event).

---

## 7. The population engine (REP)

### 7.1 Tables

One table per population kind — household, household running a business, firm — in `phx-pop`. A row is a cell of
weight one or more; individuals (the promoted, the player) are rows of weight one flagged `individual` in the same
table, and state that only individuals have (named plant, establishments, lots) lives in an **extension facet**
keyed by party. There is one home per kind; promotion and demotion convert pooled holdings to named ones and back
(REP.29). Every institution kind (bank, fund, insurer, scheme, clearing house, dealer, agency) has its own table in
its system crate and is never a cell.

Row layout: identity (`id u64`, `weight u32`, `flags u16`), key (`u32` interned key id; key records live in a key
table), sort key for landing (`u64`), list references (profiles, attachments, holdings: `u32` offset and `u16`
length each), fact columns for positions as **`i64` totals in declared fixed-point units** (money in smallest
units, ratios and outlooks in declared scales) so every position is an exact total (REP.20). Read-positions (cash,
wealth) are computed from the ledger, never stored.

### 7.2 Arenas and locality

Variable-length lists (profiles, attachments, holdings) live in **chunk-local arenas**: each worker writes only its
chunk's arena, and the landing pass compacts them by copying in slot order. Columns and arenas sit in reserved
address space committed on demand; nothing reallocates. No store holds a heap-owning type. Cells are periodically
**renumbered** — sorted by kind, country, region, zone and key — and every slot reference is remapped; slot
references live only in declared places (line sides, far-side lists, the directory) so this is cheap.

### 7.3 Hazards and occasions: survival screening

For each (row, process), a cached **survival threshold** — the probability that no member is hit today, the product
over profile values of (1 − p)^n, as a `u32` — is kept current wherever counts or rates change. Each day, one uniform
per (row, process) is compared with it; only on a hit are the per-value counts drawn, conditioned on at least one
hit. This is an exact binomial on the cell's own stream (REP.7), at a few nanoseconds per (row, process). A turn's
non-business days are screened in the same visit. Processes declared rare may instead schedule their next hit.

### 7.4 Occasions, overlaps and splits

Stage 3 draws every occasion count for a cell; overlaps between processes (members with both a quit review and a
dwelling need) are allocated by hypergeometric draws from each process's own stream, and each system decides for
the counts assigned to it in stage 5. The resulting parts are the product of outcomes, formed in the processes'
declared order. A split divides profiles and attachments: exactly where all members hold the same, and otherwise by
multivariate hypergeometric draws **jointly within each role** (§7.6), independently across roles (REP.23).
Balances divide with their attachments (§4.5); totals leave by REP.9's rounding.

**Parts** are rows created at an apply step with a **day-local identity**. They are real cells for the rest of the
day. At 10b each part either lands in an existing cell — its day-local identity then resolves to that cell, and
records naming it are re-pointed — or becomes a cell with a permanent identity. Transient parts never enter the
directory.

### 7.5 Landing

- **Position buckets**: for each position, a value maps to (the interval between the cell's own **kinks** it lies
  in, and a step on a sign-and-magnitude log scale measured on the member's own scale, REP.20). Zero is its own
  step, negative values mirror positive ones, and the step width is the position's tolerance. Integer log2 on the
  mantissa, not `ln`, computes it.
- **Candidates**: parts and cells are sorted by (key id, a Morton code of the bucket steps of the declared leading
  positions); candidates are the cells adjacent in that order. The **check** against a candidate compares the full
  key, every position's tolerance, and every kink of either side — kinks from key rules and from each side's own
  lines (a payment due, a credit limit) — so no join averages a key or crosses a kink (REP.8, REP.16). A hash is
  only an index, never an identity.
- **Joining**: weights, totals, profiles and attachments (sort-merge), payment records (declared rule) and holdings
  (pooled cost) add, through `phx-ledger`'s merge operation, which is an instruction of reason **Landing** with legs
  from part to cell (SET.1).
- The whole landing pass is a radix sort, a parallel merge-join over deterministic key shards, and a prefix-sum
  assignment of slots. Cells joining other cells happens only when the tolerance controller runs.

### 7.6 Profiles and roles

Profiles are counted per **role** — each adult role, the dwelling role, the children — and within a role **jointly
over declared groups**; which groups are joint is a declared RESOLUTION choice tested on the ladder (REP.33). An
attachment belongs to its role and is counted jointly with that role's joint group where REP.32 requires it: the
dwelling role counts zone, dwelling class and its tenancy, mortgage and insurance attachments jointly, so a flood in a
zone draws that zone's mortgages and policies. The employment line fixes occupation and skill (its terms), so an
adult role counts its employment attachment jointly with birth year and health only where declared.

Lists are **compactly encoded**: dense small histograms for groups with few values, delta-and-varint coding for
sparse ones, one-byte counts with an escape. The first measurement of the build is entries and bytes per cell on
the opening world (§13.1).

### 7.7 Choice groups

Only market kinds whose purchases are used up at once hold choice groups (REP.37). Counts are drawn in levels —
price class and zone class, then seller cell, then an equal-probability spread over a seller cell's members — with
the sampler chosen by the smaller of draws and categories (alias tables when draws are fewer).

### 7.8 Tolerance control and the reference run

- The controller (REP.28) runs at 10b on a declared cadence: when a kind's cells exceed its budget it widens the
  tolerance of the position whose **decision gap** — estimated by evaluating decision points' pure forms on sampled
  landings drawn from the representation's own world stream — is smallest, and joins the cells that become
  neighbours. The observer's stream is never used.
- **The reference run** is the same world with landing disabled: every household and small firm a row of weight one
  (PTY.12). GEN draws the world as a stream of members (§10.2), so the reference and every rung of the ladder hold
  the same world from one seed. It needs about 75 GB (§13.1); the owner provisions a 128 GB machine for each gate
  that needs it.

---

## 8. Markets, valuation and expectations

- `phx-market` implements each form once (MKT.3–MKT.8): call auctions (schedules → clearing price, rationing); the
  continuous book (arrival by lot, price–time priority, the closing auction that sets the day's close); the dealer
  market (quotes, requests for quotes); posted prices (sellers' points and capacities, buyers' choice groups,
  rationing by lot); bilateral quotes and negotiation (a protocol over messages, §4.2, across days); administered
  facilities. Systems declare markets; the mechanism is the kernel's.
- Marks and fixings are computed at 6b by each form's declared rule (MKT.12). Valuations (MKT.20) are `Money`
  rounded by the valuer's convention, computed at 9a by **valuers** — parties whose methods are registered by the
  system that owns them (a clearing house's settlement price by `sys-drv`, an appraiser by `sys-hsg`, an actuary by
  `sys-ins`), reading only prints.
- `phx-val` computes each public-series outlook once per method per day (VAL.23) as a pure function over public
  records, including the opening history (GEN.5). Own outlooks are positions of cells or facts of individuals.

---

## 9. Endings and estates

| Ending | Handled by | How |
| --- | --- | --- |
| A cell member's death (POP.3) | `sys-dem` | The person's role leaves the household; if the household ends, its members' estate opens as an **estate row** in `sys-est` (an individual of kind estate) |
| A household, firm or fund estate | `sys-est` | Sells what it must through markets, pays claims by the country's law through `phx-ledger`'s waterfall (L3), passes the rest in kind to heirs (POP.9) |
| A bank or insurer | `sys-sup` | Resolution (SUP.5), the rest to an estate in `sys-est` |
| A clearing house | `sys-drv` | Its waterfall (DRV.3); beyond it, resolution by `sys-sup` |
| A sovereign | `sys-trs` | Default and exchange offer (TRS.5) |
| A person's personal insolvency | `sys-hh` | The procedure (HH.21) through an estate row in `sys-est` |

---

## 10. The opening world (GEN)

### 10.1 Phases

GEN runs in declared phases — **parties, physical stock, contracts, balances, history** — each system contributing
what it owns, with declared reads and writes like handlers (§5.2). Each joint distribution (housing stock with
tenure and mortgages, for instance) has one owning system.

### 10.2 Canonical drawing

Members are drawn one by one from the seed, as a stream, and each lands immediately into the representation at the
run's resolution; the reference run lands none. The same seed therefore gives the same world at every rung (PTY.12).
Drawing 120 million households takes seconds on the phone's cores.

### 10.3 Balancing, history, settling

- Balancing (GEN.4) changes amounts only through `phx-ledger` operations of reason **Balancing**, each reported.
- The opening history (GEN.2, GEN.5) is written into public records and marks history; outlooks start from it
  through `phx-val`.
- Settling (GEN.6) runs the ordinary day for the owner's settling length, a world setting. Its history is kept.

### 10.4 Settled worlds for testing

The nightly job generates and settles a world at the owner's length for the current build and saves it; stage gates
are judged on that world. Per-push CI settles for a short declared length so that it finishes in minutes.

---

## 11. Persistence

- A **save** is a directory: a manifest (format version, build, primitive-register and policy hashes, seed, day,
  settings) and one file per store, each a sequence of zstd frames of transformed column pages.
- **Saving pauses the engine**: it runs in the player's idle time between turns, and whenever the app goes to the
  background, on all cores. Its budget is 1–2 s on the phone. There is no copy-on-write.
- Population stores are always saved whole; sparse stores (records, messages, institutions) write increments
  between full saves (SET.12). **Retention**: the current and the previous full save and the increments after the
  latest, within 4 GB (§13.3).
- Loading restores the state exactly (SET.15).

---

## 12. Observation and the player

### 12.1 Views

At 10d `phx-obs` builds immutable views — dashboards, markets, news, the player's party, inspector tables, portraits —
from records, fixed-bin histograms kept incrementally by the systems' metrics, and tracers, and swaps them in behind
an `Arc`. Tables cross the FFI in pages. Nothing in `phx-obs` writes the world (Law 17).

### 12.2 The player

The player is a party whose decider fact names the player (§4.7). Its actions arrive through `phx-ffi` into a queue
read at 5c. It appears in every audit family.

### 12.3 On the phone

`phx-ffi` runs the engine on its own thread with the pinned pool, and offers: create a world (seed, settings), load,
step a turn, read a view page, submit an action, save. The Compose interface has participant and inspector modes
(OBS.2). The bench flavour runs a declared number of turns without the interface and writes a JSON report.

---

## 13. Budgets

### 13.1 Memory (3 GB resident, N8.4)

Every row is sized from its declared layout. The counts are the **design point**: 2.5 million household cells and
1.2 million firm and business cells. The cell budgets are RESOLUTION (N8.5): they are set from these measurements,
and the design fits only if profiles average **at most 40 compact entries** and attachments **at most 12** per
household cell. Both are the first numbers measured on the opening world (Stage 0).

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household rows (identity, key id, sort key, list refs, 12 positions × 8) | 2.5 M | 144 | 360 MB |
| Household profiles, compact | 2.5 M × 40 | 3 | 300 MB |
| Household attachments with cached flow terms | 2.5 M × 12 | 8 | 240 MB |
| Deposit balances on attachments | 2.5 M × 3 | 8 | 60 MB |
| Household holdings (instrument, quantity, pooled cost) | 2.5 M × 3 | 20 | 150 MB |
| Survival thresholds (rows × screened processes) | 3.7 M × 6 | 4 | 90 MB |
| Firm and business cells, rows and lists | 1.2 M | 300 | 360 MB |
| Lines | 4.0 M | 32 | 128 MB |
| Far-side lists (employment, tenancy) | 25 M | 6 | 150 MB |
| Interned keys and terms | 2.0 M | 32 | 64 MB |
| Party directory and tombstones within the history horizon | — | — | 60 MB |
| Individuals, institutions and their facets | ~0.1 M | ~1.5 KB | 150 MB |
| Instruments, individuals' lots, liens, commitments, messages | — | — | 150 MB |
| Markets: books, quotes, marks and fixings history | — | — | 80 MB |
| Public records, opening history, events within the horizon | — | — | 60 MB |
| Map, network, deposits | ~1 M tiles | 32 | 40 MB |
| Day buffers: parts, intents, batch descriptors, sort buffers | — | — | 200 MB |
| Save compression buffers | — | — | 64 MB |
| Views and tracers | — | — | 60 MB |
| Android process baseline (runtime, interface, libraries, stacks) | — | — | 250 MB |
| **Total** | | | **3 016 MB** |

The design point is 16 MB over the budget and has no headroom. That is deliberate honesty: the first measurements set
the cell budgets, and the budgets fall until the total fits with 10% headroom. The reference run at weight one needs
about 75 GB.

### 13.2 Time (1 s median, 2 s worst, N8.2)

Budgets per turn at the play resolution, in **sustained** speed on the phone. The target at burst speed is 0.6 s
median. Rows marked × scale with the days in the turn (a Monday carries three).

| Work | Median turn (Tue–Fri) | Worst turn (a month-end Monday) |
| --- | --- | --- |
| × Screening and hits: hazards and occasions | 40 ms | 120 ms |
| Household decisions, fused | 80 ms | 200 ms |
| Firm decisions, production and pricing, fused | 100 ms | 200 ms |
| × Market meetings and choice groups | 80 ms | 150 ms |
| Labour search rounds | 40 ms | 80 ms |
| Splits, landing and re-bucketing | 120 ms | 300 ms |
| Dated flows as implicit batches (payroll, rent, mortgages, benefits, taxes) | 30 ms | 250 ms |
| Settlement and the fund stage | 50 ms | 150 ms |
| Financial markets, banks, funds, insurers, the state | 120 ms | 250 ms |
| Valuation, accounts and tests | 40 ms | 120 ms |
| Streaming and incremental audit, statistics | 50 ms | 150 ms |
| Events and views | 20 ms | 30 ms |
| Reserve | 230 ms | 0 ms |
| **Total** | **1 000 ms** | **2 000 ms** |

At most three full sweeps of the population arenas per turn; election days and filing deadlines are spread by
campaign intentions and filing windows (POL.4, TAX.2).

### 13.3 Storage (4 GB, N8.4)

A full save of the design point is about 1.3 GB after transforms and compression; two full saves and the sparse
increments after the latest stay within 4 GB. Saves are measured at every device gate.

---

## 14. Testing, the live world and measurement

### 14.1 Evidence

1. **Compile level**: distinct identifier types, currency-tagged money, typed handles for everything declared,
   contexts that cannot reach later state or other parties' rows, `Missing<T>` without defaults.
2. **Logic level**: pure functions — samplers, rounding, clearing, waterfalls, contract legs, bucket functions,
   landing arithmetic, survival screening — tested with `cargo test`. No test depends on `phx-world` or the opening
   (`phx-check` refuses the dev-dependency).
3. **The live world**: `phx run` on the real world, settled (§10.4), with the audit (N1), liveness reads (N2), the
   steps' live checks, and the budgets.

### 14.2 Live checks

Every step in `IMPLEMENTATION.md` declares **live checks** with permanent identifiers (`LC-…`). Each is code in
`phx-cli`'s check suite that reads a live run's records and metrics. A live check is never deleted: a check that
stops applying is retired with its reason, and `phx-check` fails if an identifier disappears.

### 14.3 Run-comparison guards

Every CI run of the live world also proves, by world hashes per day:

- **threads**: the same seed with one worker and with all workers gives the same world (N5);
- **save and load**: 30 days, save, load, 30 more equals 60 straight (SET.15);
- **looking**: views, tracers and the audit switched on or off give the same world (Law 17);
- **streams**: adding an unused stream leaves every existing draw unchanged (CHN.1);
- **registration**: shuffling the registration list gives the same world (§6.2).

In debug builds the read tracer asserts every read is within its scope (§4.9).

### 14.4 The measurement programme

| Command | Measures |
| --- | --- |
| `phx reference` / `phx compare` | the play resolution against the weight-one world on declared reads; the difference published beside results (PTY.12, N8.5) |
| `phx ladder` | the other rungs: tolerances halved and doubled, cell budget, zones, age classes, attribute classes, promotion rank, preference types, tile size (PTY.12) |
| `phx seeds` | seed dispersion (N5) |
| `phx inject` | audit independence: a discrepancy injected into a copy of the world lights its family and no other (N1) |
| `phx experiment` | interventions and knock-outs on copies (N4, N6); knock-outs address declared facts and decision points |
| `phx report` | audit, liveness, live checks, REP.15's costs, GEN.8 and GEN.9, the placeholder count (NUM.7) and share of assumed primitives (N7), the N3 statistics as fixed in their record |

### 14.5 Continuous integration and gates

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with the disallowed lists; `cargo test`; `phx-check`; release build; the live world settled briefly and run 60 days with every live check and the run-comparison guards; counter ratchets |
| `arm.yml` | every push | release build and a 30-day live run on arm64 Linux; the arm64 counter ratchet |
| `android.yml` | every push to `main` | the app and bench flavour for `arm64-v8a` |
| `nightly.yml` | nightly | a settled world at the owner's length; two simulated years; the full report as an artifact |

**Ratchets** are on deterministic counters: kernel micro-benchmarks' instruction counts (`iai-callgrind`), bytes per
store, rows touched per sub-step, parts and landings per day, cells per kind. `perf/` changes need the owner's review
(CODEOWNERS). Wall time is judged only at the **device gate**: at the end of every stage the owner runs the bench
flavour for a simulated year on the phone and commits its report to `perf/device/`.

---

## 15. Drift guards

`phx-check` and clippy fail the build on:

1. **Layering** (§3.1): the crate graph; no system depending on another; interface crates without behaviour;
   rayon only in `phx-exec`; the external allow-list.
2. **Type-aware rules** (clippy `disallowed-methods`, `-types`, `-macros` for every world crate): `min`, `max` and
   `clamp` on numbers; `Instant::now`, `SystemTime::now`; `RandomState`, `std::collections::HashMap` and `HashSet`;
   atomics, `Mutex`, `RwLock`, `OnceLock`, `LazyLock`, `thread_local!`; `println!`. A declared real limit goes
   through `DeclaredLimit::bind`, which returns the value and a binding event (Law 6). The count of
   `allow(clippy::disallowed_*)` in world crates is a ratchet that only falls.
3. **Structural rules** (`phx-check` with `syn`): no `static` items in world crates; `#![forbid(unsafe_code)]`
   everywhere except `phx-store` and `phx-exec`; no heap-owning types in stores; numeric literals only 0, 1, −1 and
   2 in mechanisms, with engineering constants in one `consts` item per crate; no clause identifiers in comments.
4. **Types that refuse**: `Money`, `Qty` and `Missing` have no `Default` and no clamping; kind identifiers have no
   equality outside the kernel; handles are private-constructed (§5.1); system types are zero-sized.
5. **The clause map**: every spec clause is assigned to a step in `IMPLEMENTATION.md`. For a step marked done, each
   clause has its **carrier**, checked against `phx dump-registry`: STATE → a registered store, fact or type;
   DECISION → a decision point; PROCESS → a handler; INVARIANT → an audit family; MEASURE → a metric; FORBID → a
   `phx-check` rule, a clippy rule or a type-level refusal; PRIMITIVE → register entries. `#[clause]` attributes on
   code of the wrong shape are refused.
6. **Documents**: the coverage table (§18) is regenerated and must match; every step has all its sections and a
   status; the architecture's crate list matches the workspace.
7. **Process**: live-check identifiers never disappear (§14.2); a primitive's value in `data/` changes only with its
   `source` field in the same diff (anti-tuning, N7); the placeholder count only falls except by placeholders a
   stage introduces, and no system is marked done while a placeholder naming it remains (NUM.7); public-API
   snapshots of kernel and interface crates change only with the change that needs them.
8. **Ratchets** (§14.5); **never-read declarations**: a primitive, stream or hazard declared but never read during
   the CI live run is reported.

A rule changes only with its reason recorded in §17.

---

## 16. Build and target

- Release profile: `lto = "fat"`, `codegen-units = 1`, `panic = "abort"` with a panic hook that writes the
  violation report before aborting; profile-guided optimisation from bench-flavour profiles collected on the phone.
- Target features for the phone in `.cargo/config.toml`: `+lse,+rcpc,+dotprod,+fp16`; NEON through
  auto-vectorisation; columns 64-byte aligned and padded to the vector width.
- Chunk sizes per table; the pool sized to fast and medium cores; performance-hint sessions report each turn's
  target.

---

## 17. Decision record

1. **Rust, own pool, own random generator, pure-Rust mathematics** (§2).
2. **Systems cooperate only through kernel channels**: facts, messages, levies, lines, markets, records (§4).
3. **Interface crates hold shared vocabulary** and nothing else (§3.4).
4. **Sub-steps with declared reads and writes** remove order dependence (§6.2).
5. **Fused traversals** bound memory traffic (§6.3).
6. **Deposits are attachments with balances**: one representation of who banks where (§4.5).
7. **Landing is an instruction** of reason Landing, from part to cell, so SET.1 holds and the audit sees it (§7.5).
8. **Parts carry day-local identities**; only surviving parts enter the directory (§7.4).
9. **Survival screening** makes daily hazard draws exact and cheap (§7.3).
10. **Sort-based landing with full checks**; hashes are indexes only (§7.5).
11. **Implicit batches** for dated flows and meetings; the audit streams over them (§6.5).
12. **Saving pauses the engine** in idle time; no copy-on-write (§11).
13. **Canonical drawing** makes the reference run the same world (§10.2).
14. **Wall time is judged only on the phone**; CI ratchets use counters (§14.5).
15. **`sys-dem` carries the spec's POP**, to avoid confusion with `phx-pop` (§3.5).
16. **Freight within each country arrives in Stage 1**, brought forward because goods and energy need carriage; the
    spec's Part O records it.

---

## 18. Coverage

Generated by `phx-check coverage` from the clause map, and edited only through it. Status: planned, building, done.

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-id`, `phx-core`, `phx-world` | 0 | 0 | planned |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 1 | planned |
| A3 | NUM | `phx-num`, `phx-core` | 0 | 0 | planned |
| A4 | CHN | `phx-rand`, `phx-core` | 0 | 0 | planned |
| A5 | GEO | `phx-geo` | 0 | 0 | planned |
| A6 | REP | `phx-pop` | 0 | 1 | planned |
| A7 | GEN | `phx-world` and every system's contribution | 0 | 7 | planned |
| B1 | MON | `phx-ledger` | 0 | 3 | planned |
| B2 | SET | `phx-ledger`, `phx-store`, `phx-world` | 0 | 0 | planned |
| B3 | REG | `phx-ledger` | 0 | 3 | planned |
| B4 | ACC | `phx-ledger` | 0 | 2 | planned |
| C1 | MKT | `phx-market` | 0 | 3 | planned |
| C2 | VAL | `phx-val` | 1 | 3 | planned |
| D1 | POP | `sys-dem` | 0 | 6 | planned |
| D2 | HH | `sys-hh` | 1 | 6 | planned |
| E1 | TEC | `sys-tec` | 1 | 6 | planned |
| E2 | FRM | `sys-frm` | 0 | 2 | planned |
| E3 | CAP | `sys-cap` | 1 | 2 | planned |
| F1 | GDS | `sys-gds` | 1 | 1 | planned |
| F2 | SRV | `sys-srv` | 1 | 1 | planned |
| F3 | FRT | `sys-frt` | 1 | 5 | planned |
| F4 | LAB | `sys-lab` | 1 | 1 | planned |
| F5 | HSG | `sys-hsg` | 2 | 2 | planned |
| F6 | TCR | `sys-tcr` | 2 | 2 | planned |
| F7 | ENE | `sys-ene` | 2 | 2 | planned |
| G1 | BNK | `sys-bnk` | 0 | 3 | planned |
| G2 | BFL | `sys-bfl` | 2 | 3 | planned |
| G3 | BCP | `sys-bcp` | 2 | 3 | planned |
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
| J4 | CB | `sys-cb` | 0 | 5 | planned |
| J5 | SUP | `sys-sup` | 2 | 4 | planned |
| J6 | POL | `sys-pol` | 5 | 5 | planned |
| K1 | FX | `sys-fx` | 5 | 5 | planned |
| K2 | XB | `sys-xb` | 5 | 5 | planned |
| M1 | OBS | `phx-obs`, `android/` | 0 | 6 | planned |
| M2 | STA | `sys-sta` | 1 | 5 | planned |
| L3 | estates | `sys-est`, `phx-ledger` | 0 | 3 | planned |
| L1, L2, L4–L12 | transmission chains | tested by `phx experiment` (N4) | 2 | 7 | planned |
| N1 | audit | `phx-audit` and every system's families | 0 | 7 | planned |
| N2–N8 | measurement | `phx-cli`, `phx-world` metrics, `android/` bench | 0 | 7 | planned |

FRM, BNK and CB start at Stage 0 because the opening world of Stage 0 already holds firms, banks and central banks as
parties with balance sheets; their behaviour begins at Stage 1.
