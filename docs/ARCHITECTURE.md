# Project Phoenix — Architecture

How the world in `docs/spec/PROJECT_PHOENIX.md` is built, top down: technologies, layers, the channels between
systems, the data model, the day, the budgets and the guards. The spec says **what**; this document says **how, in
outline**; `docs/IMPLEMENTATION.md` says **how, step by step**. Where this document and the spec disagree, the spec
wins and this document is fixed in the same change. Clause identifiers in brackets, such as (REP.8), point to the
spec; section signs, such as §4.2, point to this document.

---

## 1. Goals and forces

The architecture serves four goals, in this order when they conflict:

1. **The laws hold** (spec Part I).
2. **The budget holds** (N8): a turn in at most 1 s at the median and 2 s at the worst on a Pixel 11 Pro, sustained
   over a simulated year, within 3 GB resident memory and 4 GB of saves, with about 300 million people.
3. **Modularity**: to change how a system works you change that system's crate and nothing else; adding a system
   is a new crate and one registration line.
4. **No drift**: code, documents and spec stay true to one another because machines check it (§16).

The forces:

- **Scale.** About 120 million households, 5 million incorporated small firms, 12 million household businesses,
  tens of thousands of large firms and institutions. The population is carried in cells (REP); cost follows events
  (N8.6).
- **Relationships, not people, are the hard part of memory.** A household cell of weight 50 has workers in dozens of
  distinct employment lines, dwellings under many tenancies and mortgages, deposits at several banks. Stored
  naively, relationships grow with people, and no cell budget shrinks them. The architecture stores each
  relationship once (§4.5), keeps line terms to what must be exact (§7.6), gives declared levers to cut relationship
  counts, and makes their measurement the first act of the build (§14.6).
- **Member activity, not cells, sets the time.** Occasions, splits, landings and choices scale with members who act.
  Every mechanism is designed for cost per acting member, and the budget has a line for each (§13.2).
- **Memory traffic.** One sweep of the population state costs tens of milliseconds on a phone. Work is fused into few
  traversals and indexed rather than swept wherever possible (§6.3).
- **A phone throttles.** Work is chunked by cost, pinned to fast and medium cores, and budgeted against sustained
  speed.
- **Exactness.** Money and units are integers; conservation identities hold exactly (Law 7).
- **Fifty systems, one world.** Systems never call each other; they cooperate through kernel channels (§4).

---

## 2. Technology

| Concern | Choice | Why |
| --- | --- | --- |
| Engine | **Rust**, stable, pinned in `rust-toolchain.toml`, edition 2024 | Layout and allocation control, no GC pauses, data-race freedom, Android and Linux targets, crates as compiler-enforced boundaries. |
| World mathematics | **`libm`** (pure Rust) for transcendental functions in world code | Platform-independent results. |
| Parallelism | **Own pool** in `phx-exec` over **rayon-core**, pinned to fast and medium cores, with Android performance-hint sessions | Cost-sized fixed chunks, no little cores, few barriers. Only `phx-exec` depends on rayon. |
| Hash maps | **hashbrown** + fixed-seed **foldhash**, behind a kernel map type with no iteration | Fast lookups; no outcome depends on hash order. |
| Randomness | **Own Philox4x32-10** in `phx-rand`, batch-first samplers | Draws addressable by (stream, identity, day, index): parallel, order-free, reproducible (CHN.1, CHN.6). |
| Compression | **zstd** level 1 after per-column transforms (delta, zigzag, bit-packing) | Fast; transforms double the ratio on integer columns. |
| Declared data | **TOML** + **serde**, at assembly only | Diffable, never on a hot path. |
| Android bridge | **UniFFI** in `phx-ffi` | One foreign surface; data crosses in pages. |
| Android build | **cargo-ndk** + **Gradle**; 16 KiB page alignment | Required by current Android targets. |
| Interface | **Kotlin + Jetpack Compose** | Native; off the hot path. |
| Command line | **clap** in `phx-cli` | Runs, benchmarks, reference runs, reports. |
| Checks | **`phx-check`** (`syn`, `cargo metadata`) + **clippy** `disallowed-*` lists | Structural and type-aware rules (§16). |
| Counters | **`iai-callgrind`** for kernel micro-benchmarks; the engine's own counters | Deterministic ratchets; wall time only on the phone. |
| API snapshots | **cargo-public-api** for kernel and interface crates | Kernel surfaces change only on purpose. |
| CI | **GitHub Actions**: x86-64 Linux; arm64 Linux where the plan allows; an Android build job | See §14.7 for runner limits. |

External crates are an allow-list in `phx-check`; adding one is recorded in §18.

---

## 3. Layers and crates

```text
L4 apps            phx-cli · phx-ffi → android/ · phx-check
L3 assembly        phx-world · phx-obs
L2 systems         sys-dem sys-hh sys-est ... sys-sta                    (depend on L0, L1, IF only)
IF interfaces      if-pop if-firm if-labour if-property if-credit if-banking
                   if-securities if-risk if-state if-open if-energy      (data and pure rule signatures only)
L1 kernel          phx-store phx-exec phx-core phx-geo phx-ledger phx-pop
                   phx-market phx-acct phx-val phx-audit
L0 foundation      phx-num phx-rand phx-id phx-macros
```

### 3.1 Dependency rules (checked by `phx-check`)

- A crate depends only on lower layers. Inside L1 the order is `phx-store → phx-exec → phx-core → phx-geo →
  phx-ledger → phx-pop → phx-market → phx-acct → phx-val → phx-audit`.
- **Interface crates** contain types, handles, schemas and rule *signatures*; `phx-check` refuses any function with a
  body other than a constructor or a field accessor.
- **A system crate never depends on another system crate.** Only `phx-world` knows every system (§5). Only
  `phx-exec` depends on rayon. Only `phx-store` and `phx-exec` may use `unsafe`.
- A crate is created when its first step starts, by `phx-cli new-system` or `new-kernel`, never in advance.

### 3.2 Foundation

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>` with a niche, rounding conventions, checked `i64`/`i128` arithmetic, `DeclaredLimit`. |
| `phx-rand` | CHN.1, CHN.6 | Philox; stream keys; batch samplers: binomial (inversion, BTPE), multinomial (conditional binomials; alias tables when draws are fewer than categories), hypergeometric (H2PE) and multivariate hypergeometric, weighted picks over prefix sums, geometric, normal, log-normal, Pareto, Gumbel; binary exponentiation for survival. |
| `phx-id` | — | Identifier types (`PartyId`, slots, `LineId`, `RelId`, `InstrumentId`, day-local ids), `Day`, `Date`. |
| `phx-macros` | — | `#[clause]`, `declare_kind!`, `declare_fact!`, `declare_store!`, `declare_message!`, `declare_rule!`, `declare_system!`. |

### 3.3 Kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space; chunk-local arenas compacted in place; slot allocators; column descriptors; save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; cost-sized chunked traversals with fused kernels; gathers by prefix sum keyed (chunk, handler); sharded `KeyedReduce`; fixed-tree reductions; radix sorts. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | Calendar and conventions; the party directory with tombstones; **kind tables of individuals** with facet columns (§4.1); kinds and profiles; the primitive register and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); **rule handles** (§4.7); hazard and occasion declarations; **public records** with audiences (§4.9); events; findings; contract violations; party creation and ending as kernel operations. |
| `phx-geo` | GEO | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure, **physical stock per (tile, class)** (§7.9). |
| `phx-ledger` | MON, SET, REG, L3's ranking | The **contract algebra** (§4.4); lines; **relationship tables** (§4.5); instruments, holdings with the holder index, lots, liens, commitments; **levies** (§4.3); instructions and implicit batches; settlement and net settlement (§6.5); **fail records** and payment records; transformation records; **line transfers** (§4.4); the estate waterfall. |
| `phx-pop` | REP | Cell tables; keys (interned with reference counts); positions; profiles by role; **screening** of hazards and occasions for cells and individuals (§7.3); occasion allocation; splits and parts; landing and its index (§7.5); tolerance control; choice groups; promotion; renumbering; the reference-run mode. |
| `phx-market` | MKT | The six forms (§8); prints, marks and fixings; admission hooks; market failures. |
| `phx-acct` | ACC, MKT.20 | Valuations and valuers; statements; carrying bases and unrealised differences; equity accounts. |
| `phx-val` | VAL | Outlook methods as pure functions; public-series outlooks once per method per day; surprise and confidence arithmetic. |
| `phx-audit` | N1 | Families; streaming checks fused into apply steps; independent records (§15); incremental and rolling checks; injection mode. |

### 3.4 Interfaces

Each interface crate is a domain's shared vocabulary: kinds and roles; fact handles; line-kind terms built from the
contract algebra; message payloads; decision-point input and output types; rule signatures; view schemas. **Every
item names the one system that writes or implements it**; assembly checks it (§5.4).

| Crate | Domain |
| --- | --- |
| `if-pop` | households, persons, roles, demographic facts, household decision points |
| `if-firm` | firms, products, ways, production facts, pricing decision points |
| `if-labour` | employment terms, vacancies, applications, offers, separations |
| `if-property` | dwellings, land, tenancies, collateral descriptions, appraisals |
| `if-credit` | loan terms, applications and quotes, credit-bureau records, trade credit |
| `if-banking` | deposit terms and payment order; bank facts; resolution |
| `if-securities` | bonds, shares, fund units, reports, filings, ratings, indices, orders |
| `if-risk` | derivative, insurance and pension terms; margin demands; claims |
| `if-state` | policy values, levies, benefits, agencies, budgets, elections, rule signatures of tax and benefit |
| `if-open` | currencies, regimes, trade, migration |
| `if-energy` | power products, the grid, dispatch |

### 3.5 Systems

`sys-dem` (the spec's POP), `sys-hh`, `sys-est`, `sys-tec`, `sys-frm`, `sys-cap`, `sys-gds`, `sys-srv`, `sys-frt`,
`sys-lab`, `sys-hsg`, `sys-tcr`, `sys-ene`, `sys-bnk`, `sys-bfl`, `sys-bcp`, `sys-sec`, `sys-mmk`, `sys-sov`,
`sys-crd`, `sys-eqy`, `sys-mna`, `sys-fnd`, `sys-dlr`, `sys-idx`, `sys-rat`, `sys-drv`, `sys-drx`, `sys-ins`,
`sys-pen`, `sys-trs`, `sys-tax`, `sys-soc`, `sys-cb`, `sys-sup`, `sys-pol`, `sys-fx`, `sys-xb`, `sys-sta`.

### 3.6 Assembly and applications

| Crate or project | Owns |
| --- | --- |
| `phx-world` | Registry and schema compilation (§5.3); stages and sub-steps (§6); GEN (§10); saving (§11); the player's decider (§12.2); metrics. |
| `phx-obs` | Views, tracers, portraits; read-only. |
| `phx-cli` | `run`, `bench`, `reference`, `compare`, `ladder`, `seeds`, `inject`, `experiment`, `measure`, `report`, `new-system`, `new-kernel`, `dump-registry`. |
| `phx-ffi` | The engine as an Android library. |
| `android/` | The Compose app and its `bench` flavour. |
| `phx-check` | Law, layering and document checks (§16). |

### 3.7 Repository

```text
Cargo.toml · rust-toolchain.toml · clippy.toml · .cargo/config.toml · CODEOWNERS
crates/{foundation,kernel,interfaces,systems,assembly,apps}/
android/  data/  perf/  docs/  .github/workflows/{ci,arm,android,nightly}.yml
```

---

## 4. Channels between systems

Systems never call each other and never read each other's stores. Everything shared goes through one of these
channels, each with one writer per fact (Law 4) and declared audiences (Law 12).

### 4.1 Facts and facets

A **fact** is a named, typed attribute of parties of declared kinds, declared in an interface crate with its type and
unit, the kinds it applies to, **exactly one writer** (a system, or a placeholder SHAPE naming the system that retires
it), its **audience** (the party, a named authority, public after a lag) and, for cells, its representation class
(key, position or profile, REP.33). Facts compile into columns of the cell tables and into **facet columns** of the
kernel's **kind tables of individuals** — one table per individual kind (bank, fund, insurer, scheme, clearing
house, dealer, agency, large firm, estate), owned by `phx-core`, whose columns belong to the systems that declared
them. A bank is one row, written by BNK, BFL, BCP and SUP each in its own columns. Writing needs a token only the
writer crate can build; reading needs a handle. Party creation and ending are kernel operations requested by declared
systems (SUP.9 creates a bank; SUP.5 ends one).

### 4.2 Messages

A **message** is an addressed record: kind; sender (a party, or a cell with a count); addressee (a party, or a line
side with a count); issued day; due day; the line or instrument it concerns; state (open, answered, lapsed, failed).
Each kind declares, per addressee kind, the **answering system** and **the sub-step it answers in**, and, where an
answer can be accepted, the **acceptance handler** — the system that writes the resulting line or instruction (a
mortgage offer accepted: `sys-bnk` writes the loan line, `sys-hsg` the dwelling's transfer). Assembly refuses a kind
that reaches an unanswered addressee kind.

A message to cells becomes **notice occasions** for the counted members (REP.21). A money demand opens a commitment
(REG.10). High-volume kinds (job applications, retail requests) carry counts and are **day-local**; kinds that live
across days (quotes, redemptions, calls, claims) are snapshotted (SET.16).

**Open business pins its members**: members of a cell with an open message or commitment carry it as a key attribute
of their part, so they land only with members holding the identical item, and a reply reaches exactly the members
who asked (REP.16, REP.23).

### 4.3 Levies

A **levy** is a declared deduction or addition on another system's flows (income tax and contributions at payroll,
value-added tax at a sale, duty at a border, pension contributions). It declares: the flow reasons it applies to; its
**base per member** of the side entry (a leg's per-member amount, or a per-person accumulator such as taxable income
to date); its schedule (a policy value, piecewise linear between kinks registered with `phx-pop`); the remitter and
payee; whether it is the collector's liability until remitted (TAX.2); and its order. The amount for a side entry is
computed **per member, rounded per member, then multiplied by the count** (REP.9, TAX.7). `phx-ledger` composes
levies into the flow's instruction.

### 4.4 Contract algebra, lines and transfers

Every contract's terms are a composition of **generic legs** — a fixed amount; a rate on a notional (fixed or floating
over a named transacted reference, with day count and resets); an amount per unit of time; an indexed amount; an
amount contingent on a named event; a delivery of units — placed on dates by a calendar **schedule**, with
**seniority**, **collateral description** (kind, zone, class of what secures it) and **payment order** as data.
Accrual, dues, payment, arrears, provisioning inputs and waterfalls are generic kernel code; line and instrument kinds
are declarations (REG.5–REG.10, BNK.17, DRV.1, INS.1, PEN.2).

A **line** is one record of identical contracts (REP.3): kind, interned terms (only what must be exact), and its
**relationship rows** (§4.5). A **line transfer** moves a count of a side to a new party — a sale of loans (BNK.10,
SEC.2), a deposit transfer in resolution (SUP.5), a client moved to another clearing member (DRV.9), an estate
succeeding a party (L3), a foreclosure (HSG.11) — as an instruction with a reason; each line kind declares which
systems may request which transfers.

### 4.5 Relationship tables

For every line kind, a **relationship table** holds one row per (line, holder, role): `{line, holder slot, role,
count, per-member cached amount (i32 with overflow violation), balance (i64, for deposit and loan kinds), payment
record (u16 days in arrears, u16 missed)}`, sorted by line. A per-holder index lists each holder's relationship rows.
**One row serves both directions**: the holder's view (its attachments) and the counterparty's (its side). There is
no separate attachment arena and no separate far-side list.

- **Deposits and loans** carry balances on their rows. A cell's members share their cash position (REP.20); a member's
  share of a row is `balance / count`. Each deposit kind declares which payments it funds and the **payment order**
  across a member's deposit rows (current before savings; term deposits never), so payments debit rows in a declared
  deterministic order; the deposit-insurance limit (SUP.2) is a kink on per-member row balance.
- **Fails** become fail records with the reason and the line; the owner of the line kind reads them at its declared
  sub-step (§6.1).
- **Payment records** combine at landing by a declared rule; lenders' and suppliers' views read them (REP.8).
- **Holder index for instruments**: each instrument keeps its holders sorted, so coupons, dividends, bail-ins and
  REG.13's audit read holders directly (REG.4).

### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run. Its opening value comes from `data/`;
after that it is a fact whose one writer is the owner's decision, or a declared intervention on a copy (N6). Each
change writes a dated announcement with its effective day, at least the next business day (VAL.6, POL.7).

### 4.7 Decision points, deciders and rule handles

- Every decision the spec names is a **decision point**: an input view type and an output intent type from an
  interface crate, a rule function registered by its system, and a pure evaluation form usable off-world (REP.15).
  The **decider** is a per-party fact: the kind's rule, or the player. The kernel dispatches inside the decision point;
  on days the player has queued nothing for a decision, the rule decides (OBS.4, OBS.6).
- A **rule handle** is a pure function declared in an interface crate and implemented by its owning system — the tax
  on a given income, a benefit entitlement, a lender's cap, a clearing house's margin for a trade, a platform's effect
  on a household — callable by any system with inputs it may read. It reads only its inputs and policy values. Market
  **admission hooks** (DRV.6) are rule handles.

### 4.8 Keyed reductions

`KeyedReduce<K, V>`: chunk-local sorted runs, merged in parallel by owner-key shard, each shard in chunk order. The
only way a pass sums over rows it does not own.

### 4.9 Records, audiences, scoped reads

Decision kernels read through `ctx.party(row)`: the party's own rows and facts; the relationship rows it is holder or
counterparty of; records whose audience includes it; earlier prints and marks. Audience is checked at compile and
assembly time from declarations; a release feature `read-trace` samples chunks and verifies reads at run time in CI
(§14.3). Store-wide views go only to processes and applies.

### 4.10 Public events

The OBS.3 rule runs at stage 10's first sub-step and writes to `phx-core`'s public records; `phx-obs` only shows them.

---

## 5. The kernel–system boundary

### 5.1 `System`

```rust
pub trait System: Send + Sync + 'static {        // zero-sized implementors only (asserted)
    const CODE: SystemCode;
    fn declare(d: &mut Declarations);             // kinds, facts, stores, lines, messages, levies, markets, rules,
                                                  // primitives, policy values, hazards, occasions, kinks, decision
                                                  // points, audits, metrics, views, opening contributions
    fn handlers(h: &mut HandlerTable);            // kernels per sub-step, with reads and writes
}
```

Declarations generate typed handles with private constructors: using something undeclared does not compile, and an
unused declaration fails `dead_code`. Registration is one line in `phx-world/src/systems.rs`; its order carries no
meaning (§6.2).

### 5.2 Handlers

A handler is a kernel over chunks of one table, declared with its sub-step, the facts and columns it reads and writes,
the messages and intents it emits, and its streams. Its context grants exactly that.

### 5.3 Schema compilation

Declarations compile into: key layouts (interned key records with reference counts); fact columns and facet columns;
profile groups per role; kink sets; relationship-table layouts per line kind; message, instrument, market and levy
profiles; the stream registry; the primitive register checked against `data/` (NUM.3); the handler graph; audits and
metrics. A key or count field whose value outgrows its width is a **contract violation** calling for a layout change;
nothing saturates (Law 6).

### 5.4 Assembly refusals

A fact with no writer or two; a message kind with an unanswered addressee kind or no answering sub-step; two handlers
in one sub-step where one writes what the other reads or writes; a levy without a schedule owner; a kink on an
undeclared position; a primitive without value, unit, kind or source; a decision point without an evaluation form; a
rule signature without an implementer; a line kind without declared transfer requesters; an interface item whose
writer is not registered.

---

## 6. The day

### 6.1 Turns, stages and sub-steps

A **turn** advances the world to the next day that is a business day in any country, running each day in between as
its own day (N8.2). On each day, each country's business-day calendar decides which of its markets and institutions
act (TIME.2, TIME.8). Every day runs the stages below; a sub-step marked **B** runs only for countries whose business
day it is.

| Stage | Sub-steps |
| --- | --- |
| 1 Open | 1a lapse orders, quotes and day-local messages · 1b list dues and dated flows as implicit batches (B) · 1c read the player's queue |
| 2 Resolve | 2a accruals · 2b **settle calls and demands due today** (the same settlement routine as 7, B); failures handed to their owners at once (TIME.7) · 2c deliver yesterday's fails to owners; arrears · 2d losses land; endings; estates open and distribute · 2e apply |
| 3 Nature and population | 3a weather; catastrophes allocated by tile (§7.9) · 3b screening of hazards and occasions for cells and individuals (§7.3) · 3c demographic events; foundings and moves decided the day before · 3d allocate overlapping occasions (§7.4) · 3e apply: splits and transformation records |
| 4 Real work | 4a production, services, shipments, construction, jobs starting and ending · 4b apply |
| 5 Decide | 5a public-series outlooks per method · 5b continuous decisions, fused per table · 5c lumpy decisions for occasion holders; institutions (B); answers to messages at their declared sub-step · 5d apply |
| 6 Form prices | 6a meetings: retail, services and electricity every day; all others (B) · 6b marks and fixings · 6c apply: matches to instructions; on non-business days goods and consumption legs with card commitments settling at the next business day's stage 7 |
| 7 Settle (B) | 7a generate per-payer debit lists and the bank-by-bank gross matrix from batches; compose levies · 7b **joint fixed point**: payer checks and banks' reserve nets with intraday credit over the whole reserve position, until nothing changes · 7c apply every surviving instruction's legs, deposits and reserves together, with the streaming audit fused in · 7d ring pass · 7e fails recorded; payees of failed payers drawn (REP.23); splits |
| 8 Fund (B) | 8a market round: post, form, settle · 8b facilities round: standing facilities and lender-of-last-resort loans (reading the supervisor's solvency fact), settle · 8c close intraday credit; record liquidity failures (BFL.10) |
| 9 Value and judge | 9a valuations · 9b accounts and ratios · 9c tests; demands issued, due next business day · 9d publications |
| 10 Close | 10a public-event rule · 10b landing of the day's parts; index maintenance; on declared light days, tolerance control and renumbering; monthly ranks · 10c incremental audit families · 10d views and tracers (read-only) · 10e metrics |

Non-business days also run the declared decision points of 5b and 5c that their meetings need (buyers' spending,
generators' offers).

### 6.2 Order without order dependence

Within a sub-step every handler reads the state as it was at the sub-step's start; intents apply at its end. The
handler graph is built from declared reads and writes and refuses conflicts, so registration order carries no
meaning; intent buffers are keyed (chunk, canonical handler id), so gathers and new identities do not depend on it
either. CI proves it by shuffling the registration list (§14.3).

### 6.3 Traversals

`phx-exec` runs one traversal per table per sub-step, in **cost-sized chunks** (declared per table, never dependent
on the thread count), running every handler of the sub-step on a chunk while it is in cache. Sub-steps with no work
are skipped. Each sub-step declares whether it is a **full sweep**, an **active-rows** pass (rows with hits,
occasions, flows or parts) or **index-driven**; a **sweep ledger** counts bytes touched per sub-step, and a ratchet
holds it (§16).

### 6.4 Compute, gather, apply

Handlers write their own rows and emit intents into (chunk, handler) buffers; gathers place them by prefix sum;
applies run in declared order, in parallel over disjoint targets. **One apply routine** serves every apply sub-step:
it settles the instructions of reasons allowed in that sub-step (money payments only in 2b, 7 and 8; transformation,
landing, estate distribution and line transfer instructions in their declared sub-steps), checks them and feeds the
streaming audit. Every reduction runs over a fixed tree.

### 6.5 Settlement

- **Explicit instructions** and **implicit batches**. Implicit batches — (line kind or market, rule, day) — are never
  materialised: debits are generated payer-major from relationship rows' cached per-member amounts, credits
  payee-major; the two sides agree by REP.31.
- 7a produces **per-payer debit lists** in declared order and the **bank-by-bank gross matrix**. 7b iterates payer
  checks and banks' reserve nets **jointly** over these compact aggregates until nothing changes, regenerating legs only
  for payers whose state changed; nothing is applied before 7c.
- **Failure is per payer** (MON.5): a payer that cannot pay fails its own legs; where the pairing to its payees was not
  recorded, the payees who lose are drawn (REP.23). A multi-payer batch is split into per-payer instructions at 7a.
- 7c applies all legs of every surviving instruction together; applied is final (SET.5).
- Physical transformations are checked against transformation records (SET.9).

---

## 7. The population engine (REP)

### 7.1 Tables

One cell table per population kind — household, household running a business, small firm — in `phx-pop`. Individuals
of these kinds (the promoted, the player) are rows of weight one flagged `individual` in the same table, with an
**extension facet** for state only individuals have. There is one home per kind. Institutions are rows of the
kernel's kind tables of individuals (§4.1), never cells.

A cell row: `id u64`, `weight u32`, `flags u16`, key id `u32` (interned), landing-grid key `u64`, list references
(profiles, holdings: `u32` offset, `u16` length) and **position columns** as `i64` totals in declared fixed-point
units (REP.20). Read-positions (cash, wealth) are read from relationship and holding rows; the landing check tests
them too.

### 7.2 Arenas and locality

Variable-length lists live in chunk-local arenas, compacted in place per chunk with a chunk-sized scratch; freed pages
return to the system. Columns sit in reserved address space; nothing reallocates; no store holds a heap-owning type.
Cells are **renumbered** on declared light days (by kind, country, region, zone, key), and every slot reference —
relationship rows, the per-holder index, the directory, the landing index — is remapped in the same pass.

### 7.3 Screening

For each (row, process): the probability that no member is hit is computed **at screen time** from the process's rate
per profile value by binary exponentiation (`q^n`), with no cache and no invalidation; one uniform per (row, process,
day) decides whether any member is hit. On a hit:

- where the rate is **uniform within a role**, one binomial gives the total and the hit members are picked by weighted
  picks over a prefix of counts, O(k log e);
- where rates differ by value, candidates are drawn at the highest rate and thinned.

Each day of a turn is screened on its own day (§6.1). Rare processes may schedule their next hit instead (REP.7).
Individuals are screened the same way with counts of one.

### 7.4 Occasions, overlaps, splits and parts

Overlapping occasions in one cell are allocated by hypergeometric draws from each process's stream in stage 3; each
system decides for the counts assigned to it; parts are the product of outcomes in declared process order. A split
divides profiles and relationship rows jointly within each role's declared groups by multivariate hypergeometric
draws, and balances divide with their rows; totals leave by REP.9's rounding.

**Parts** are rows with **day-local identities**, created lightweight: a weight-one part carries only its own
profile entries and its own relationship rows. At 10b a part either lands in a cell — its identity resolves to that
cell and the records naming it are re-pointed through back-pointers — or becomes a cell with a permanent identity.
Transient parts never enter the directory. Members pinned by open business (§4.2) land only with identical items.

### 7.5 Landing

- **The landing grid**: each cell's grid key is (key id, global bucket vector), where each position's bucket is a step
  on a sign-and-magnitude log scale on the member's own scale, zero its own step, with the step equal to the
  position's tolerance; buckets are global, not relative to one cell's kinks.
- **The landing index** is a hash over grid keys, maintained incrementally. A part probes its bucket and the
  neighbouring buckets of its leading positions. The **check** compares full key, every position's tolerance
  (read-positions included) and every kink of either side — key rules and each side's own lines (a payment due, a
  credit limit, the insured limit) — so no join averages a key or crosses a kink (REP.8, REP.16).
- **Order independence**: all parts bound for one target are checked against the target's state at 10b's start, then
  joined together.
- **Joining** is one `Landing` instruction per part, from part to cell, whose legs are derived from the part's rows
  (SET.1); weights, totals, profiles, relationship rows (by line and role), payment records and holdings (pooled cost)
  add.

### 7.6 Relationship counts and their levers

Relationship rows per cell dominate memory (§13.1). The levers, all declarations, all tested on the ladder:

- **Line terms carry only what must be exact** for the flows and decisions that read them. Employment lines are
  (occupation, skill, wage point); start dates, work zone and employer identity live on the employer's side as counts
  and are drawn when they matter (REP.23). Tenancies are (zone, class, rent point, term); mortgages (bank, rate
  point, term, vintage on the declared vintage grid, collateral zone and class).
- **Attributes may move from profile to key** (REP.33): an adult role's occupation family in the key concentrates a
  cell's employment rows.
- **Contract grids** are price points and vintages, conventions of each trade (REP.34).

### 7.7 Profiles

Profiles are counted per role, jointly within declared groups (REP.32, REP.33). Joints the spec requires are carried
by contract terms where they belong: a mortgage's and a dwelling policy's collateral description names zone and class,
so a flood's draws meet the right mortgages and policies. Lists are compactly encoded: dense small histograms, delta
and varint coding, one-byte counts with an escape.

### 7.8 Choice groups

Only market kinds whose purchases are used up at once hold choice groups (REP.37). **Shares** are computed once per
(zone, preference type, product) per day; groups are cells within tolerance with the same zone value; counts are drawn
per group over seller classes then seller cells, apportioned to the group's cells at the group's mix, and aggregated
per seller cell, whose members' demand is spread once per day (REP.22). Rounds of re-choice after capacity binds are
budgeted separately.

### 7.9 Places and catastrophes

`phx-geo` keeps **physical stock per (tile, class)** as the one writer of where units stand; relationship and holding
rows carry (zone, class); an index lists, per (zone, class), the rows holding it. A catastrophe at 3a gathers the rows
holding the struck (zone, class), allocates the units lost in one multivariate hypergeometric draw across them, and
scatters the losses back (REP.24, GEO.8).

### 7.10 Tolerance control, promotion, the reference run

- The controller (REP.28) runs at 10b on declared light days, estimating decision gaps from pure decision-point forms
  over landings sampled from the **representation's own world stream**; it joins cells that become neighbours.
- **Promotion** reads ranks monthly and at every issuance of a public instrument (REP.2, REP.29).
- **The reference run** disables landing: every household and small firm is a row of weight one (PTY.12). GEN's
  canonical drawing (§10.2) makes it the same world. It needs **about 120 GB**; the owner provisions a 256 GB machine,
  and a gate's reference work (settling, a year, several seeds) takes one to three days of compute.

---

## 8. Markets, valuation and expectations

- `phx-market` implements each form once (MKT.3–MKT.8): call auctions (with admission hooks); the continuous book
  (arrival by lot, price–time priority, closing auction); the dealer market; posted prices with choice groups and
  rationing by lot; bilateral quotes as a protocol over messages across days; administered facilities.
- Marks and fixings at 6b by each form's rule (MKT.12). Valuations (MKT.20) in `phx-acct` at 9a, as `Money` rounded by
  the valuer's convention, by valuers whose methods are registered by their systems and read only prints.
- `phx-val` computes public-series outlooks per method per day from public records, including the opening history.

---

## 9. Endings and estates

| Ending | Handled by | How |
| --- | --- | --- |
| A cell member's death | `sys-dem` | The role leaves the household; if the household ends, an estate row in `phx-core`'s estate table, behaviour in `sys-est` |
| Household, firm, fund or political-party estate | `sys-est` | Sells what its debts need; pays by the country's law through the ledger's waterfall (L3); passes the rest in kind (POP.9) |
| Personal insolvency | `sys-hh` | The procedure (HH.21) through an estate row |
| Bank, insurer | `sys-sup` | Resolution (SUP.5) with line transfers; the rest to an estate |
| Pension scheme | `sys-pen` | Sponsor contributions, benefit cuts, the guarantee fund, then an estate |
| Clearing house | `sys-drv` | Its waterfall; beyond it, `sys-sup` |
| Public agency | `sys-soc` | Its duties and staff pass to a successor agency named by the budget |
| Sovereign | `sys-trs` | Default and exchange offer |

Estate rows are short-lived individuals; about 1 million a year pass through, a few tens of thousands at once
(§13.1).

---

## 10. The opening world (GEN)

### 10.1 Phases

Declared phases — **parties, physical stock, contracts, balances, history** — each system contributing what it owns,
with declared reads and writes. Each joint distribution has one owning system; each opening line kind names its
writer (the loan line's writer is `sys-bnk`, whoever draws the dwelling).

### 10.2 Canonical drawing

- **Pass A**: institutions, firms and physical stock are drawn; then every household is drawn **complete** — members,
  roles, holdings and its relationship choices at the stratum level of line terms — from per-member counter keys, in
  parallel, and only stratum totals are kept.
- **Balancing** (GEN.4) runs on the totals: counterparty sides are set from them (banks' deposit liabilities,
  employers' headcounts); unmatched demand or supply becomes vacancy or unemployment, allocated by canonical member
  order; institutions' balance sheets close through ledger operations of reason `Balancing`, each reported.
- **Pass B** redraws every household identically from the same keys, applies the balancing allocations, and lands
  them in bulk in sort order at the run's resolution; the reference run lands none.

The same seed therefore gives the same world at every rung (PTY.12), and nothing is balanced after merging. Drawing
the full population takes tens of seconds on the phone's cores.

### 10.3 History and settling

The opening history is written into public records and marks; outlooks start from it. Settling (GEN.6) runs the
ordinary day for the owner's length, a world setting; its history is kept.

### 10.4 Settled worlds for testing

Nightly: a world is generated and settled at the owner's length for the current build and saved; stage gates are
judged on it. Per push: a short declared settling.

---

## 11. Persistence

- A **save** is a directory: a manifest (format, build, register and policy hashes, seed, day, settings) and one file
  per store of zstd frames of transformed pages.
- Saves are written at the moments SET.12 declares — when the player saves, when the app is set aside, and at the
  declared interval (an owner setting, by default every simulated quarter) — and the world pauses while one is written
  (N8.10); on the phone, all cores write, about 1–2 s.
- Population stores are written whole; sparse stores write increments between full saves.
- **Retention**: the latest complete save, plus the one being written; the older is deleted only after the new one is
  complete (SET.15), so the peak is two saves (§13.3).

---

## 12. Observation and the player

- **Views** are built at 10d from records, incrementally maintained fixed-bin histograms and tracers, and swapped in
  behind an `Arc`; tables cross the FFI in pages; `phx-obs` writes nothing (Law 17).
- **Tracers** follow members through splits by the observer's stream, conditioned on their profile values (REP.30).
- **The player** is a party whose decider fact names the player; its queue is read at 1c and dispatched inside each
  decision point; it appears in every audit family.
- **On the phone**, `phx-ffi` runs the engine on its own thread with the pinned pool: create, load, step a turn, read
  a view page, submit an action, save. The bench flavour runs a declared number of turns headless and writes a JSON
  report.

---

## 13. Budgets

### 13.1 Memory (3 GB resident, N8.4)

Every row is sized from its declared layout. The counts are a **provisional design point**, to be replaced by the
first measurements (§14.6); the cell budgets are RESOLUTION and fall until the total fits with 10% headroom (N8.5).

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household rows (identity, key id, grid key, list refs, 20 positions × 8) | 2.5 M | 208 | 520 MB |
| Household profiles, compact | 2.5 M × 40 | 3 | 300 MB |
| **Relationship rows**, all line kinds, households and firms (one row serves both directions) | 36 M | 24 | 864 MB |
| Per-holder relationship index | 36 M | 4 | 144 MB |
| Household holdings (instrument, quantity, pooled cost) and holder index | 7.5 M | 24 | 180 MB |
| Firm and business cells: rows, keys, profiles | 1.2 M | 400 | 480 MB |
| Lines (kind, terms id, row range) | 4 M | 16 | 64 MB |
| Interned keys and terms, reference-counted | 2.5 M | 48 | 120 MB |
| Landing index | 3.7 M | 16 | 60 MB |
| Kind tables of individuals and their facets; estates | 0.15 M | 1.5 KB | 225 MB |
| Instruments, lots, liens, commitments, messages that live across days | — | — | 120 MB |
| Markets, marks and fixings history; public records; opening history; events | — | — | 120 MB |
| Map, network, deposits, stock per (tile, class) and its index | — | — | 80 MB |
| Day buffers: parts, intents, per-payer lists, bank matrix, sort scratch | — | — | 250 MB |
| Save buffers | — | — | 64 MB |
| Views and tracers | — | — | 60 MB |
| Android process baseline | — | — | 250 MB |
| **Total** | | | **3 901 MB** |

**The design point does not fit.** It totals about 3.9 GB; 3 GB with 10% headroom is 2.7 GB. About 1.3 GB of the
total does not depend on the population (the process baseline, day buffers, individuals, markets and records, map,
views), so the population's stores must fit in about 1.4 GB rather than 2.6 GB. That means roughly 1.3 million
household cells with at most 10 relationship rows each, and firm cells cut likewise — which the accuracy gate may not
accept — or a memory budget of about 4.5 GB, or a smaller population. The first measurements (§14.6) decide which;
the choice is the owner's (§18). The weight-one reference run needs about 120 GB.

### 13.2 Time (1 s median, 2 s worst, N8.2)

Per turn at the play resolution in **sustained** speed on the phone (about 4 core-seconds per wall second); the target
at burst speed is 0.6 s. Rows marked × scale with the days in the turn. The **worst turn** is the longest turn of the
year that contains a quarter-end, typically three or four days.

| Work | Median turn (1 day) | Worst turn (4 days, quarter-end) |
| --- | --- | --- |
| × Screening | 20 ms | 80 ms |
| × Member occasions and lumpy decisions (per acting member) | 100 ms | 250 ms |
| Household continuous decisions, fused | 60 ms | 120 ms |
| Firm decisions, production and pricing, fused | 80 ms | 150 ms |
| × Market meetings and choice groups | 60 ms | 180 ms |
| Labour search rounds | 30 ms | 60 ms |
| × Splits, parts and landing (per part) | 90 ms | 300 ms |
| Dated flows as implicit batches | 20 ms | 200 ms |
| Settlement joint fixed point and the fund stage | 40 ms | 150 ms |
| Financial markets, banks, funds, insurers, the state | 90 ms | 180 ms |
| Valuation, accounts, tests | 30 ms | 90 ms |
| × Audit, statistics, events and views | 40 ms | 120 ms |
| Barriers and tails (about 100 per day) | 40 ms | 120 ms |
| Reserve | 300 ms | 0 ms |
| **Total** | **1 000 ms** | **2 000 ms** |

Tolerance control and renumbering run only on declared light days, within those days' reserve. Election days and
filing deadlines are spread by campaign intentions and filing windows (POL.4, TAX.2).

### 13.3 Storage (4 GB, N8.4)

A full save of the design point is about 1.5 GB after transforms; the latest save plus one being written stay within
4 GB.

---

## 14. Testing and measurement

### 14.1 Evidence

1. **Compile level**: distinct identifiers, currency-tagged money, typed handles, contexts that cannot reach later
   state or other parties' rows, `Missing<T>` without defaults.
2. **Logic level**: pure functions — samplers, rounding, contract legs, clearing, waterfalls, bucket functions,
   landing arithmetic, screening, levies per member — under `cargo test`. No test depends on `phx-world` or the
   opening.
3. **The live world**: `phx run` on the real world, settled, with the audit (N1), liveness (N2), the steps' live
   checks, the run-comparison guards and the budgets.

### 14.2 Live checks

Every step declares **live checks** with permanent identifiers (`LC-…`), implemented in `phx-cli`'s check suite over a
live run's records and metrics. A retired check keeps its identifier and says why; `phx-check` fails if one
disappears.

### 14.3 Run-comparison guards

World hashes per day prove: one worker equals all workers (N5); 30 days + save + load + 30 days equals 60 straight
(SET.15); views, tracers, audit and `read-trace` on or off give the same world (Law 17); adding an unused stream
changes no draw (CHN.1); shuffling the registration list changes nothing (§6.2). The guards share one baseline run.

### 14.4 The measurement programme

| Command | Measures |
| --- | --- |
| `phx measure` | the representation's own numbers: rows per kind, relationship rows per cell by line kind, profile entries per role, bytes per store, occasions and parts per day, choice groups and draws, legs per batch, sweeps and bytes per sub-step |
| `phx reference` / `phx compare` | the play resolution against the weight-one world on declared reads; the difference published beside results (PTY.12, N8.5) |
| `phx ladder` | the other rungs of PTY.12 |
| `phx seeds` | seed dispersion (N5) |
| `phx inject` | audit independence (N1) |
| `phx experiment` | interventions and knock-outs on copies (N4, N6) |
| `phx report` | audit, liveness, live checks, REP.15's costs, GEN.8–GEN.9, NUM.7, N7, the N3 statistics as fixed in their record |

### 14.5 Gates

Every stage ends with: CI green; the device gate (the bench flavour runs a settled simulated year on the phone; the
owner commits the report to `perf/device/`); `phx measure` within the memory and time budgets; from Stage 1,
`phx compare` within the declared accuracy (N8.5).

### 14.6 Measure first

Before any behaviour is built on the representation, Stage 0 generates the full opening world and `phx measure`
reports: relationship rows per cell by line kind, profile entries per role and bytes per entry, bytes per store, and
the landing rate of the opening. Stage 1's gate adds occasions per day by process, parts per day, the cost of one
split and landing, choice groups and draws, legs on the worst turn, and — on the phone — sustained bandwidth for one
sweep and barrier cost. If the numbers do not fit, the levers of §7.6 are applied in order (line terms, attributes to
key, contract grids), then the cell budgets fall, and only then does the owner decide between a larger memory budget
and a smaller population.

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test`; `phx-check`; release build with thin LTO and `read-trace`; the live world briefly settled and run 60 days with every live check and the run-comparison guards; counter ratchets |
| `arm.yml` | every push, where the plan provides arm64 runners | release build and a 30-day live run on arm64 Linux |
| `android.yml` | every push to `main` | the app and bench flavour, fat LTO |
| `nightly.yml` | nightly | fat LTO; a world settled at the owner's length; two simulated years; the full report |

Standard runners for private repositories have limited cores and memory; the live world at play resolution needs
about 3.5 GB and runs slowly there, which is acceptable for correctness, and the per-push run length is sized to keep
CI under 30 minutes.

---

## 15. Guards on the running world

- **The audit** (N1) streams over batches as they apply and checks against **independent records**: per-batch digests
  kept at apply time; issuers' own totals per deposit and loan line; issued amounts per instrument. Injection mode
  lights each family alone (`phx inject`).
- **Live checks and run-comparison guards** as above.

---

## 16. Drift guards

`phx-check` and clippy fail the build on:

1. **Layering** (§3.1), the external allow-list, rayon only in `phx-exec`, `unsafe` only in `phx-store` and
   `phx-exec`.
2. **Type-aware rules** (clippy disallowed lists over world crates): `min`, `max`, `clamp` on numbers; `Instant::now`,
   `SystemTime::now`; `RandomState`, std `HashMap`/`HashSet`; atomics, `Mutex`, `RwLock`, `OnceLock`, `LazyLock`,
   `thread_local!`; `println!`. Declared real limits go through `DeclaredLimit::bind` (Law 6). The count of
   `allow(clippy::disallowed_*)` in world crates only falls.
3. **Structural rules** (`phx-check`): no `static` items in world crates; no heap-owning types in stores; numeric
   literals only 0, 1, −1 and 2 in mechanisms, engineering constants in one `consts` item per crate; no clause
   identifiers in comments; interface crates without behaviour.
4. **Types that refuse**: `Money`, `Qty`, `Missing` without `Default` or clamping; kind identifiers without equality
   outside the kernel; private-constructed handles; zero-sized systems.
5. **The clause map**: every spec clause is assigned to a step in `IMPLEMENTATION.md`; for a step marked done, each
   clause has its **carrier** in `phx dump-registry` — STATE → store, fact or type; DECISION → decision point;
   PROCESS → handler; INVARIANT → audit family; MEASURE → metric; FORBID → a check, clippy rule or type-level
   refusal; PRIMITIVE → register entries — and `#[clause]` attributes on items of the wrong shape are refused.
6. **Documents**: the coverage table (§19) regenerated and matching; every step with all its sections and a status;
   this document's crate lists matching the workspace.
7. **Process**: live-check identifiers never disappear; a primitive's value in `data/` changes only with its `source`
   in the same diff; the placeholder count only falls except by placeholders a stage introduces, and no system is done
   while a placeholder naming it remains; public-API snapshots of kernel and interface crates change only with the
   change that needs them; `perf/` changes need the owner's review (CODEOWNERS).
8. **Ratchets** on deterministic counters (§14.7): kernel instruction counts, bytes per store and per row, rows and
   bytes touched per sub-step, parts and landings per day, barriers per day, cells per kind; declared-but-never-read
   primitives, streams and hazards are reported.

A rule changes only with its reason recorded in §18.

---

## 17. Build and target

- Release: `lto = "fat"` (nightly and device builds; `thin` per push), `codegen-units = 1`, `panic = "abort"` with a
  hook that writes the violation report; profile-guided optimisation from bench-flavour profiles.
- Phone target features: `+lse,+rcpc,+dotprod,+fp16`; NEON by auto-vectorisation; 64-byte aligned, padded columns.
- Pool sized to fast and medium cores; performance-hint sessions per turn.

---

## 18. Decision record

1. Rust, own pool, own random generator, pure-Rust mathematics.
2. Systems cooperate only through kernel channels (§4); interface crates hold shared vocabulary and rule signatures.
3. Individuals of institutional kinds are kernel rows with system-owned facets (§4.1).
4. **Relationships are stored once**, in per-kind relationship tables serving both directions (§4.5).
5. Deposits and loans carry balances on relationship rows, with declared payment order (§4.5).
6. Levies are computed per member (§4.3).
7. Sub-steps with declared reads and writes; order-free gathers (§6.2).
8. Settlement: implicit batches never materialised; a joint fixed point of payers and banks; failure per payer (§6.5).
9. Screening computes survival at screen time; hits are allocated by weighted picks (§7.3).
10. Landing on a global grid with neighbour probing and full checks; parts joined order-independently (§7.5).
11. Open business pins members (§4.2).
12. Canonical two-pass drawing makes every rung the same world (§10.2).
13. Saving pauses the world at declared moments (§11), as the spec's SET.12 and N8.10 now allow.
14. Demands due today settle in stage 2 so consequences fall the same day (TIME.6, TIME.7).
15. A turn runs to the next business day in any country; each country's markets follow its own calendar (§6.1).
16. `sys-dem` carries the spec's POP.
17. Freight within each country arrives in Stage 1; firms, banks and central banks exist as parties from Stage 0
    (Part O records both).
18. **Measure first**: the representation's relationship counts decide the cell budgets before behaviour is built
    (§14.6).

**Pending owner decisions**: the map size and regions per country (spec Appendix E); the accuracy for play (N8.5);
the settling length (GEN.6); the default save interval; and, if the first measurements require it, a larger memory
budget or a smaller population.

---

## 19. Coverage

Generated by `phx-check coverage` from the clause map. Status: planned, building, done.

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-id`, `phx-core`, `phx-world` | 0 | 0 | planned |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 1 | planned |
| A3 | NUM | `phx-num`, `phx-core` | 0 | 0 | planned |
| A4 | CHN | `phx-rand`, `phx-core`, `phx-pop` | 0 | 0 | planned |
| A5 | GEO | `phx-geo` | 0 | 0 | planned |
| A6 | REP | `phx-pop`, `phx-ledger` | 0 | 1 | planned |
| A7 | GEN | `phx-world` and every system's contribution | 0 | 7 | planned |
| B1 | MON | `phx-ledger` | 0 | 3 | planned |
| B2 | SET | `phx-ledger`, `phx-store`, `phx-world` | 0 | 0 | planned |
| B3 | REG | `phx-ledger` | 0 | 3 | planned |
| B4 | ACC | `phx-acct` | 0 | 2 | planned |
| C1 | MKT | `phx-market`, `phx-acct` | 0 | 3 | planned |
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
