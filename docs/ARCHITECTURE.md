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
   over a simulated year, within 4.5 GB resident memory and 4 GB of saves, with about 300 million people.
3. **Modularity**: to change how a system works you change that system's crate and nothing else; adding a system
   is a new crate and one registration line.
4. **No drift**: code, documents and spec stay true to one another because machines check it (§16).

The forces:

- **Scale.** About 120 million households, 5 million incorporated small firms, 12 million household businesses,
  tens of thousands of large firms and institutions. The population is carried in cells (REP).
- **Cost follows events** (REP.12, N8.6). A row is touched on a day only when it is on that day's **agenda**
  (§7.3): a decision it is scheduled to take, a wake, an occasion or hazard hit, a payment or a kink it reaches.
  Continuous decisions are taken on each party's own schedule (TIME.5) and their flows run between decisions as
  **standing flows** (§7.4). The only daily pass over most rows is settlement's stream over the columns it needs
  (§6.5); decisions, parts and screening touch only the rows that act.
- **Members who act set the time.** Occasions, parts, landings and choices scale with members, not cells; the
  spec's coarsening (Appendix E 31) — pooled flows, reviews on a cell's own days, sellers spread weekly — keeps them
  few. Each has a unit cost, a count, and a counter that ratchets it (§13.2).
- **Relationships set the memory.** A household cell has members in many employment lines, tenancies and loans.
  Each relationship is stored once, with its holder (§4.5), and rows per cell are measured as a curve over cell
  budgets before behaviour is built (§14.6).
- **A phone is latency-bound and throttles.** Hot paths are laid out for sequential access in the holder's chunk;
  random gathers are counted; work is chunked by cost and pinned to fast and medium cores; the budget uses the
  sustained speed measured on the device.
- **Exactness.** Money and units are integers; conservation identities hold exactly (Law 7).
- **Fifty systems, one world.** Systems never call each other; they cooperate through kernel channels (§4).

---

## 2. Technology

| Concern | Choice | Why |
| --- | --- | --- |
| Engine | **Rust**, stable, pinned in `rust-toolchain.toml`, edition 2024 | Layout and allocation control, no GC pauses, data-race freedom, Android and Linux targets, crates as compiler-enforced boundaries. |
| World mathematics | **`libm`** (pure Rust) for transcendental functions in world code | Platform-independent results. |
| Parallelism | **Own pool** in `phx-exec` over **rayon-core**, pinned to fast and medium cores, with Android performance-hint sessions | Cost-sized fixed chunks, no little cores, few barriers. Only `phx-exec` depends on rayon. |
| Hash maps | **hashbrown** + fixed-seed **foldhash**, behind a kernel map type with no iteration, sharded where written in parallel | Fast lookups; no outcome depends on hash order. |
| Randomness | **Own Philox4x32-10** in `phx-rand`, batch-first, NEON-vectorised samplers | Draws addressable by (stream, identity, day, index): parallel, order-free, reproducible (CHN.1, CHN.6). |
| Compression | **zstd** level 1 after per-column transforms (delta, zigzag, bit-packing) | Fast; transforms double the ratio on integer columns. |
| Declared data | **TOML** + **serde**, at assembly only | Diffable, never on a hot path. |
| Android bridge | **UniFFI** in `phx-ffi` | One foreign surface; data crosses in pages. |
| Android build | **cargo-ndk** + **Gradle**; 16 KiB page alignment | Required by current Android targets. |
| Interface | **Kotlin + Jetpack Compose** | Native; off the hot path. |
| Command line | **clap** in `phx-cli` | Runs, benchmarks, reference runs, reports. |
| Checks | **`phx-check`** (`syn`, `cargo metadata`) + **clippy** `disallowed-*` lists | Structural and type-aware rules (§16). |
| Counters | **`gungraun`** (formerly `iai-callgrind`, on valgrind) for kernel micro-benchmarks; the engine's own counters | Deterministic ratchets; wall time only on the phone. |
| API snapshots | **cargo-public-api** for kernel and interface crates | Kernel surfaces change only on purpose. |
| CI | **GitHub Actions**: x86-64 Linux; arm64 Linux where the plan allows; an Android build job; a larger runner nightly | See §14.7. |

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

- A crate depends only on lower layers. Inside L0 the order is `phx-macros → phx-num → phx-rand → phx-id`; inside
  L1 it is `phx-store → phx-exec → phx-core → phx-geo → phx-ledger → phx-pop → phx-market → phx-acct → phx-val →
  phx-audit`. Interface crates may depend on L0 and L1.
- **Interface crates** contain types, handles, schemas and rule *signatures*; `phx-check` refuses any function with a
  body other than a constructor or a field accessor.
- **A system crate never depends on another system crate.** Only `phx-world` knows every system (§5). Only
  `phx-exec` depends on rayon. Only `phx-store` and `phx-exec` may use `unsafe`.
- A crate is created when its first step starts, by `phx-cli new-system` or `new-kernel`, never in advance.

### 3.2 Foundation

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>`, rounding conventions, checked `i64`/`i128` arithmetic, the named comparisons that replace `min` and `max` (§16), `violation!` and its payload, **point tables** (a trade's price points as `i64`, REP.34). No floating-point type in any store. |
| `phx-rand` | CHN.1, CHN.6, CHN.7 | Philox; stream keys; batch samplers: binomial (inversion, BTPE) and **zero-truncated** binomial, multinomial (conditional binomials; alias tables when draws are fewer than categories), hypergeometric (H2PE) and multivariate hypergeometric, weighted picks over prefix sums, geometric (for next-candidate days), normal, log-normal, Pareto, Gumbel; rejection thinning. |
| `phx-id` | TIME.1, TIME.2 (the day and civil dates), PTY.1 (identities) | Identifier types (`PartyId`, slots, `LineId`, `RowRef`, `InstrumentId`, day-local ids), `Day`, `Date`. |
| `phx-macros` | — | `#[clause]`, `#[derive(Pod)]`, `declare_kind!`, `declare_fact!`, `declare_store!`, `declare_message!`, `declare_rule!`, `declare_system!`. |

### 3.3 Kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space; **chunk-local arenas** compacted in place; slot allocators with recycling; column descriptors; save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; cost-sized chunked traversals over the day's **agenda** or a whole table; gathers by prefix sum keyed (chunk, handler); sharded `KeyedReduce`; fixed-tree reductions; radix sorts. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | Calendar and conventions; **decision schedules, wakes and the agenda** (§7.3); the party directory with bounded tombstones; **kind tables of individuals** with facet columns (§4.1); kinds and profiles; the primitive register, `DeclaredLimit` (a real limit constructible only from the register or a contract's terms) and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); **rule handles** (§4.7); hazard and occasion declarations; **public records** with audiences (§4.9); events; findings; party creation and ending. |
| `phx-geo` | GEO | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure, **physical stock per (tile, class)** and the (zone, class) index of holdings (§7.10). |
| `phx-ledger` | MON, SET, REG, L3's ranking | The **contract algebra** (§4.4); lines and their holder lists; **relationship rows** in holders' arenas (§4.5); instruments, holdings with the holder index, lots, liens, **commitments**; **levies** (§4.3); **instructions**, composite instructions and implicit batches; settlement (§6.5); **standing flows** and pooled flows (§7.4); fail records and payment records; transformation records; **line transfers**, including the split at a kink (§4.4); the estate waterfall. |
| `phx-pop` | REP | Cell tables; keys (interned, reference-counted, sharded); positions and their **steps** (REP.4); profiles by role; **screening** (§7.3); occasion allocation; splits and parts; **landing** and its index (§7.6); choice-group pieces (§7.9); tolerance control; promotion; renumbering; the reference-run mode. |
| `phx-market` | MKT | The six forms (§8); prints, marks and fixings; admission hooks; market failures. |
| `phx-acct` | ACC, MKT.20 | Valuations and valuers; statements; carrying bases and unrealised differences; equity accounts. |
| `phx-val` | VAL | Outlook methods as pure functions; public-series outlooks once per method per day; surprise and confidence arithmetic. |
| `phx-audit` | N1 | Families; streaming checks fused into applies; independent records (§15); incremental and rolling checks; injection mode. |

### 3.4 Interfaces

Each interface crate is a domain's shared vocabulary: kinds and roles; fact handles; line-kind terms built from the
contract algebra; message payloads; decision-point input and output types; rule signatures; view schemas. **Every
item names the one system that writes or implements it**; assembly checks it (§5.4).

| Crate | Domain |
| --- | --- |
| `if-pop` | households, persons, roles, demographic facts, household decision points |
| `if-firm` | firms, products, ways, production facts, pricing decision points |
| `if-labour` | employment terms, vacancies, applications, offers, separations |
| `if-property` | dwellings, land, tenancies, collateral descriptions, appraisals, sales |
| `if-credit` | loan terms, applications and quotes, credit-bureau records, trade credit |
| `if-banking` | deposit terms, banking arrangements and payment order; bank facts; resolution |
| `if-securities` | bonds, shares, fund units, reports, filings, ratings, indices, orders |
| `if-risk` | derivative, insurance and pension terms; margin demands; close-outs; claims |
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
| `phx-world` | Registry and schema compilation (§5.3); stages and sub-steps (§6); GEN (§10); saving (§11); the player's decider (§12); metrics. |
| `phx-obs` | Views, tracers, portraits; read-only. |
| `phx-cli` | `run`, `bench`, `reference`, `compare`, `ladder`, `seeds`, `inject`, `experiment`, `measure`, `report`, `new-system`, `new-kernel`, `dump-registry`; the live-check suite. |
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
answer can be accepted, the **acceptance handler**. Assembly refuses a kind that reaches an unanswered addressee kind.

A message to cells becomes **notice occasions** for the counted members (REP.21). A money demand opens a commitment
(REG.10). High-volume kinds (job applications, retail requests) carry counts and are **day-local**; kinds that live
across days (quotes, offers, redemptions, calls, claims) are snapshotted (SET.16).

**Open business that belongs to particular members pins them**: members of a cell with an open message addressed to
or from them, a need or notice occasion carried to a later day (§7.3), or a commitment that is theirs alone (an
accepted mortgage offer, a pending sale) carry it as a key attribute of their part, so they land only with members
holding the identical item, and a reply reaches exactly the members who asked (REP.16, REP.23). Amounts that are
balances — card purchases awaiting settlement, receivables, undrawn credit on a held line — are positions with steps
(§7.6) that divide by REP.9; they pin nothing, and only a kink keeps members apart.

### 4.3 Levies

A **levy** is a declared deduction or addition on another system's flows (income tax and contributions at payroll,
value-added tax at a sale, duty at a border, pension contributions). It declares: the flow reasons it applies to; its
**base per member** of the side entry (a leg's per-member amount, or the remitter's own year-to-date figure for that
line, never a figure the remitter cannot see, Law 12; the annual assessment reads the per-role positions, §7.8); its
schedule (a
policy value, piecewise linear between kinks registered with `phx-pop`); the remitter and payee; whether it is the
collector's liability until remitted (TAX.2), carried on an accruing row (§4.5); its order; and, where the payee
system turns the money into something the member holds, its **follow-on**: an instruction written by the payee system
from the levy's legs (a defined-contribution subscription into fund units at the next net asset value, PEN.3; an
accrual of defined-benefit rights on the member's position, REP.20). The amount for a side entry is computed **per
member, rounded per member, then multiplied by the count** (REP.9, TAX.7). `phx-ledger` composes levies into the
flow's instruction.

### 4.4 Contract algebra, lines, instructions and transfers

Every contract's terms are a composition of **generic legs** — a fixed amount; a rate on a notional (fixed or floating
over a named transacted reference, with day count and resets); an amount per unit of time; an indexed amount; an
amount contingent on a named event; a delivery of units — placed on dates by a calendar **schedule**, with
**seniority**, **collateral description** (kind, zone, class of what secures it) and **payment order** as data.
Accrual, dues, payment, arrears, provisioning inputs and waterfalls are generic kernel code; line and instrument kinds
are declarations (REG.5–REG.10, BNK.17, DRV.1, INS.1, PEN.2). Amounts that sit on a trade's price points (REP.34) are
carried as a **point index** into that trade's point table; a line kind with no point table carries an `amount i64`
column (§4.5).

A **line** is one record of identical contracts (REP.3): kind, interned terms, and a **holder list** (§4.5). An
**instruction** carries legs of any reasons and systems and settles all-or-nothing (SET.4). A **composite
instruction** is written by the one handler that accepts a trade and draws on **commitments** other systems wrote:
a commitment (REG.10) records, as its writer's declaration, the legs it contributes and the rows it creates or retires
when drawn, so the ledger creates a loan row for `sys-bnk` without `sys-hsg` writing it (Law 4). A dwelling bought
with a mortgage is one composite instruction written by `sys-hsg`'s acceptance handler, drawing on the buyer's
lender's **loan commitment** and the seller's lender's **redemption commitment** (the payoff amount and the loan row
it retires, written by `sys-bnk`): the buyer's deposit, the loan paid out, the payment to the seller, the payoff, the
title and the new loan row, all or none.

A **line transfer** moves a count of a side to a new party — a sale of loans (BNK.10, SEC.2), a client moved to
another clearing member (DRV.9), an estate succeeding a party (L3), a foreclosure (HSG.11) — as an instruction with a
reason, settled by the settlement routine with its money legs; each line kind declares which systems may request
which transfers. A **split at a kink** divides each row of a line by a per-member amount — the insured part of a
deposit (SUP.2, SUP.5) to a receiving bank, the rest to a claim line on the estate — computed per member and
multiplied by the count, so it is exact.

### 4.5 Relationship rows

Every relationship of a holder to a line is a **row in the holder's chunk arena**, contiguous with the holder's
other rows: `{line u32, count u32, point u16, record u16, role u8, flags u8}` (16 bytes), plus the optional columns
its kind declares: `balance i64` for **accruing kinds** (deposits, loans, a collector's tax payable, any running
payable); `pending i64` on deposit rows for card payments and other commitments awaiting settlement (SET.2); `amount
i64` for kinds without a point table. `point` is the line's price point, copied because terms never change (REP.3),
so a row's per-member amount is one lookup in a cache-resident point table. `record` packs days in arrears and missed
payments. Each **line** keeps a **holder list** — holder slots sorted, in chunked blocks — so line-major events (a
firm closes, a pairing is drawn, a bank fails) find their holders directly. The line's side totals are kept
incrementally and checked against its holders (REP.31).

- **Holder-major** traversals — a visit, the payer pass of settlement, splits, landing — read a holder's rows
  sequentially; line-major events go through the holder list. Rows grow and shrink by re-appending the holder's run
  at the arena's end and compacting in place; no row index outside the arena names a position in it.
- **Deposits.** A cell's **banking arrangement** — which deposit kinds it holds at which bank — is a **key
  attribute** (REP.33). Every deposit row of a cell therefore has count equal to the weight, a member's share of a
  balance is balance ÷ weight (REP.9), and each deposit row's per-member balance and pending amount are positions with
  steps in the landing key (§7.6), so no join averages money held at different banks or in different kinds. Each
  deposit kind declares which payments it funds and in which order (current before savings; term deposits never).
  Changing bank is a lumpy decision whose members split into a part (REP.5). Coverage (SUP.2) is per person: limit ×
  the arrangement's adult holders, over the member's balances at that bank in the declared coverage order.
- **Banknotes** are a holding (instrument: the central bank's notes in that currency, MON.1); a purchase paid in
  banknotes moves them between holding rows at 6c.
- **Loans** carry balances; a row's members share its terms, vintage and payment record, so its per-member balance
  is exact. Members of one line whose payment records diverge are drawn out (REP.23) and split.
- **Fails** become fail records with the reason and the line; the owner of the line kind reads them at its declared
  sub-step (§6.1). **Payment records** combine at landing by a declared rule; lenders' and suppliers' views read them
  (REP.8).
- **Holdings** of instruments are rows `{instrument u32, quantity i64, pooled cost i64, flags u32}` in the holder's
  arena; each instrument keeps its holders sorted, so coupons, dividends, bail-ins and REG.13's audit read holders
  directly (REG.4).

### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run. Its opening value comes from `data/`;
after that it is a fact whose one writer is the owner's decision, or a declared intervention on a copy (N6). Each
change writes a dated announcement with its effective day, at least the next business day (VAL.6, POL.7). A change
that moves a hazard's rate above its screening envelope reschedules the rows it concerns (§7.3).

### 4.7 Decision points, deciders and rule handles

- Every decision the spec names is a **decision point**: an input view type and an output intent type from an
  interface crate, a rule function registered by its system, a declared **schedule** (TIME.5) and **wake conditions**,
  and a pure evaluation form usable off-world (REP.15). The **decider** is a per-party fact: the kind's rule, or the
  player. The kernel dispatches inside the decision point.
- A **rule handle** is a pure function declared in an interface crate and implemented by its owning system — the tax
  on a given income, a benefit entitlement, a lender's cap, a clearing house's margin for a trade, a platform's effect
  on a household — callable by any system with inputs it may read. It reads only its inputs and policy values. Market
  **admission hooks** (DRV.6) are rule handles.

### 4.8 Keyed reductions

`KeyedReduce<K, V>`: chunk-local sorted runs, merged in parallel by owner-key shard, each shard in chunk order. The
only way a pass sums over rows it does not own, and the way every **global structure** — the landing index, the key
and terms interner, the directory, line creation, holder lists — is updated: sharded by key hash, each shard applied
by one worker, so nothing serialises and no atomic is needed.

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
                                                  // points, schedules, audits, metrics, views, opening contributions
    fn handlers(h: &mut HandlerTable);            // kernels per sub-step, with reads and writes
}
```

Declarations generate typed handles with private constructors: using something undeclared does not compile, and an
unused declaration fails `dead_code`. Registration is one line in `phx-world/src/systems.rs`; its order carries no
meaning (§6.2).

### 5.2 Handlers

A handler is a kernel over chunks of one table — its agenda rows, or every row where the sub-step declares a sweep —
declared with its sub-step, the facts and columns it reads and writes, the messages and intents it emits, and its
streams. Its context grants exactly that.

### 5.3 Schema compilation

Declarations compile into: key layouts (interned key records with reference counts); fact and facet columns; the
landing-hot record of each cell kind (§7.1); position steps; profile groups per role; kink sets; relationship-row
layouts per line kind; point tables; message, instrument, market and levy profiles; the stream registry; the
primitive register checked against `data/` (NUM.3); schedules and wake conditions; the handler graph; audits and
metrics. A key or count field whose value outgrows its width is a **contract violation** calling for a layout change;
nothing saturates (Law 6).

### 5.4 Assembly refusals

A fact with no writer or two; a message kind with an unanswered addressee kind or no answering sub-step; two handlers
in one sub-step where one writes what the other reads or writes; a levy without a schedule owner, or a follow-on not
written by its payee system; a kink on an undeclared position; a primitive without value, unit, kind or source; a
decision point without an evaluation form or schedule; a rule signature without an implementer; a line kind without
declared transfer requesters; a commitment kind without the legs it creates; a hazard without a draw scheme (REP.7);
an interface item whose writer is not registered.

---

## 6. The day

### 6.1 Turns, stages and sub-steps

A **turn** advances the world to the next day that is a business day in any country, running each day in between as
its own day (N8.2). On each day, each country's business-day calendar decides which of its markets and institutions
act (TIME.2). A sub-step marked **B** runs only for countries whose business day it is; everything else runs every
day, as TIME.8 lists. Every apply sub-step may emit **parts** (§7.5); all of them land at 10b.

| Stage | Sub-steps |
| --- | --- |
| 1 Open | 1a lapse orders, quotes and day-local messages · 1b build the **agenda** (§7.3) · 1c the player's queued intents become wakes (§12) |
| 2 Resolve | 2a accruals post where a date needs them · 2b (B) **resolutions** opened by the last business day's failures: valuation and write-downs (§9.2) · 2c (B) **settle** calls and demands due today (the settlement routine of stage 7); failures handed to their owners at once (TIME.7) · 2d (B) earlier fails delivered to owners; arrears · 2e (B) recognised losses land; parties that cannot go on end; estates open and compute their waterfall · 2f apply |
| 3 Nature and population | 3a weather; catastrophes (§7.10) · 3b hazards and occasions drawn for agenda rows (§7.3) · 3c demographic events; foundings and moves decided before · 3d overlapping occasions allocated (§7.5) · 3e apply: transformation records |
| 4 Real work | 4a production, services, shipments, construction; jobs starting and ending · 4b apply |
| 5 Decide | 5a public-series outlooks per method · 5b continuous decisions of rows scheduled or woken today, fused per table · 5c lumpy decisions of occasion holders; institutions (B); answers to messages at their declared sub-step · 5d apply |
| 6 Form prices | 6a meetings: retail, services and electricity every day; all others (B) · 6b marks and fixings · 6c apply: matches become instructions; banknotes change hands; on non-business days card payments and electricity trades are recorded as **pending** on the payer's deposit row and the payee's, settling at the next business day's stage 7 |
| 7 Settle (B) | 7a **payer pass**: per payer, its legs in declared order checked against its funds, streaming over holder-major rows; per-bank nets by keyed reduction · 7b **fixed point**: the greatest set of payments that can settle given one another, with banks' nets and intraday credit, by a fail-only worklist (§6.5) · 7c apply every surviving instruction's legs, deposits and reserves together, with the streaming audit fused in · 7d fails recorded; payees of failed payers drawn (REP.23) · 7e levy follow-ons written (§4.3) |
| 8 Fund (B) | 8a money-market orders · 8b the market forms · 8c its trades settle · 8d standing-facility and lender-of-last-resort requests, reading the supervisor's solvency fact · 8e they settle · 8f intraday credit closes: a bank that cannot repay has the shortfall recorded as an **overdue claim of the central bank** and its liquidity failure recorded (MON.3, MON.12, BFL.10) |
| 9 Value and judge (B) | 9a valuations · 9b accounts and ratios · 9c tests: margins, covenants, capital, solvency; demands issued, due next business day; resolutions triggered and bids invited (§9.2) · 9d publications |
| 10 Close | 10a public events · 10b **landing** of the day's parts and re-keying of rows whose steps changed (§7.6); tolerance control when the cells carried exceed the budget (§7.11) · 10c (B) monthly ranks; a renumbering slice on declared light days · 10d incremental audit families · 10e views and tracers (read-only) · 10f metrics |

On a non-business day, 5b and 5c run only the decision points that TIME.8 lists (§7.3 says what happens to other
occasions), and payments that stages 3 and 4 give rise to (a founding's capital, severance) are recorded as pending,
settling at the next business day's stage 7. Money moves only at 2c, 6c (banknotes), 7 and 8, and within a cell's
own totals at 10b's landings; estate distributions, resolution transfers and line transfers are instructions settled
by that routine.

### 6.2 Order without order dependence

Within a sub-step every handler reads the state as it was at the sub-step's start; intents apply at its end. The
handler graph is built from declared reads and writes and refuses conflicts, so registration order carries no
meaning; intent buffers are keyed (chunk, canonical handler id), so gathers and new identities do not depend on it
either. CI proves it by shuffling the registration list (§14.3).

### 6.3 Traversals

`phx-exec` runs one traversal per table per sub-step, in **cost-sized chunks** (declared per table, never dependent
on the thread count), running every handler of the sub-step on a chunk while it is in cache. Each sub-step declares
whether it is an **agenda** pass (the rows the agenda lists, in slot order), a **stream** over the rows a batch reads
(the payer pass of §6.5, reading only the columns it needs), **index-driven**, or a **full sweep** (only where a clause
needs every row that day: tolerance control on the day it runs, §7.11). A **sweep ledger** counts rows and bytes
touched per sub-step, and a ratchet holds it (§16).

### 6.4 Compute, gather, apply

Handlers write their own rows and emit intents into (chunk, handler) buffers; gathers place them by prefix sum;
applies run in declared order, in parallel over disjoint targets. **One apply routine** serves every apply sub-step:
it settles the instructions of reasons allowed there (§6.1), writes transformation records, emits parts, checks and
feeds the streaming audit. Every reduction runs over a fixed tree.

### 6.5 Settlement

- **Implicit batches** — (line kind or market, rule, day) — are never materialised: debits are generated payer-major
  from holder-major rows (point lookups, levies per member), credits payee-major by keyed reduction streamed shard by
  shard; the two sides agree by REP.31. Each party's rows in a batch are summed sequentially into **one leg per
  (party, bank)** (pooled flows, §7.4), debited from its deposits in the declared payment order and credited to the
  deposit its kind declares receives that reason; a standing flow is one leg per paying row per day; pending amounts
  from non-business days are legs of the next business day's batch.
- **7a** streams each payer's legs in its declared payment order against its funds and records only per-payer totals,
  a failure flag and the first failing leg; per-bank nets are keyed reductions. Nothing is written per leg.
- **7b** starts from every payment succeeding and removes, until nothing changes, the payers who cannot pay given the
  payments still standing, and the customer legs of banks that cannot cover their nets after intraday credit (MON.3,
  MON.5); only removed payers' payees are revisited. The result is the **greatest** set that can settle, so rings of
  payments that can settle together do (TIME.6). A cell's check follows the pooled-flow rule (REP.8): its rows'
  payments are tested one row at a time in the declared payment order, each against the cell's per-member funds left
  by the rows before it; a row is pooled if neither the cell's per-member funds nor the reached members' own (their
  share less the row's per-member amount) cross zero or another kink, and otherwise the row's payment fails for its
  count.
- **7c** applies all legs of every surviving instruction together; applied is final (SET.5). Failure is per payer
  (MON.5): a payer that cannot pay fails its own legs; where the pairing to its payees was not recorded, the payees who
  lose are drawn (REP.23).
- Physical transformations are checked against transformation records (SET.9).

---

## 7. The population engine (REP)

### 7.1 Tables

One cell table per population kind — household, household running a business, small firm — in `phx-pop`. Individuals
of these kinds (the promoted, the player) are rows of weight one flagged `individual` in the same table, with an
**extension facet** for state only individuals have. Institutions are rows of the kernel's kind tables of individuals
(§4.1), never cells.

A cell row keeps a **landing-hot record** of one 64-byte line — landing key (key id and step vector hashed), weight,
flags, the key-rule kink signature (§7.6), the leading positions — and, in columns, the other positions as `i64`
totals in declared fixed-point units (REP.20), standing-flow rates (§7.4), schedule slots (§7.3) and arena references
(`u32` offset, `u16` length) to its profiles, relationship rows and holdings. Read-positions (cash, wealth) are read
from the cell's own rows.

### 7.2 Arenas, locality and identity

- Variable-length lists live in **chunk-local arenas**, compacted in place per chunk with a chunk-sized scratch;
  freed pages return to the system. Columns sit in reserved address space; nothing reallocates; no store holds a
  heap-owning type.
- **Renumbering** restores locality (by kind, country, region, zone, key) incrementally: one chunk range per declared
  light day, remapping holder lists, the directory and the landing index for the rows it moves.
- The **directory** maps permanent identities to slots. A cell that ends leaves a tombstone only while a record names
  it (a reference count kept by record kinds that may name cells); then its slot is recycled. Day-local part
  identities never enter it.

### 7.3 The agenda and screening

The **agenda** of a day lists the rows that act that day: rows whose decision schedule (TIME.5) falls today, rows
woken (a message, a surprise, the player's intent, a standing flow reaching a kink, §7.4), rows with a scheduled
payment, and rows with a hazard or occasion **candidate** today. `phx-core` keeps it as a calendar of day buckets
filled when each row's next day is set; 1b gathers today's bucket.

Each hazard and occasion declares its **draw scheme** (REP.7):

- **Scheduled** (the default): per (row, process), the next candidate day is drawn ahead at an **envelope** rate π̄ =
  1 − (1 − p̄)^W, where p̄ bounds the per-member rate over the row's profile values. On a candidate day the candidate is
  accepted with probability π/π̄, where π = 1 − Π_v (1 − p_v)^{n_v}; if accepted, the counts per value are drawn
  **jointly conditioned on a total of at least one** — values in declared order, each binomial conditioned on the
  hits still owed — so the result is exactly the daily binomial counts (CHN.7). The next candidate is redrawn when the
  weight changes or a rate rises above its envelope; a falling rate needs nothing.
- **Daily** only for processes dense enough that most rows have a hit most days: `Σ n_v ln(1 − p_v)` with one `exp`
  per (row, process), vectorised, over agenda rows and rows of the process's own declared set.

Hit members are picked by weighted picks over a prefix of the profile counts, O(k log e). Individuals are screened the
same way with counts of one.

**Reviews** (REP.21) are not screened daily. Each cell has, per kind of lumpy decision, its own **review days** — a
schedule declared per decision kind (weekly, monthly), with the cell's phase within it drawn at its creation from the
stream `CORE.schedule_phase` — always days on which that decision point runs; a surprise (VAL.4) wakes the cell for
the decisions it bears on (REP.35). Each (cell, decision kind) carries a **review exposure** position: the members'
total of −ln(1 − a_t) summed over the days since each last reviewed, added daily from the cell's attention a_t, adding
at landing, and losing the reviewers' share when they review. On a review day the count who review is drawn per
profile value with probability 1 − exp(−exposure ÷ weight), and those members are evaluated with counts (§7.5). A cell therefore enters the agenda for reviews on a fraction of days
set by its schedules, not every day. **Needs and notices** reach particular members on their own day; on a day their
decision point does not run (TIME.8) those members carry the occasion as open business (§4.2) until it does.

### 7.4 Standing flows and pooled flows

A continuous decision taken on a row's schedule (REP.5: "this week's spending") sets **standing flows**: per-member
rates — spending by category, saving into a named account, production and use of inputs — that hold every day until
the row's next decision. They are carried as rates on the row and summed, per group they feed, into **group
aggregates**, kept incrementally by keyed reduction when a rate changes (§7.9). Each day:

- markets read the aggregates, not the rows (§7.9);
- each paying row's amount for the day — its rate, scaled by what its group actually bought when capacity bound — is
  **one leg of that day's batch**, written in the streamed payer pass (§6.5): settled at stage 7 on a business day; on
  a non-business day paid in banknotes at 6c or recorded as pending on the deposit row (TIME.8);
- a physical flow (output, use of inputs) is a transformation record of the day (SET.9).

**Pooled flows** (REP.8): a flow that reaches some of a cell's members — wages on some of its employment rows, a due
on some of its loan rows — is applied to the cell's totals. The payer pass sums a party's rows of the batch
sequentially and writes **one leg per (party, bank)**, so a payroll or a day's dues is a leg per employer and per cell,
not per row. Rows are tested one at a time in the declared payment order: a row is pooled when neither the cell's
per-member positions nor the reached members' own (their share before the row plus the row's per-member amount) cross
a kink — funds at zero, a limit, a tax band on the per-role year-to-date positions, a means test. Otherwise the
row's members split with their own outcome (an outflow fails for them; an inflow lands them past the kink). The
spread erased is recorded per flow (REP.15). A non-business day's standing flows are paid by card, recorded as
pending, unless the kind's declaration says banknotes.

Only **accruals** — interest, accrued rights — are posted lazily, on the dates that need them (REP.12), and at any
split, landing or re-key that touches the row, and they enter the kink-day computation. A row whose
position crosses a step boundary at any apply is flagged and **re-keyed** at 10b, so its landing key is always true
(§7.6). At each decision the ledger computes the day a standing flow would carry a position across a kink (funds
reaching zero, a limit, a band) and puts the row on that day's agenda, so no kink is crossed unseen (REP.16).

### 7.5 Occasions, overlaps, splits and parts

Overlapping occasions in one cell are allocated by hypergeometric draws from each process's stream at 3d; each system
decides for the counts assigned to it, evaluated **per (row, decision, profile combination)** with counts; parts are
the product of outcomes in declared process order. A split divides profiles and relationship rows jointly within each
role's declared groups by multivariate hypergeometric draws, and balances divide with their rows; totals leave by
REP.9's rounding. A split happens only when members' **key** changes, when a flow would carry them across a kink
(§7.4), when they act on a lumpy decision that changes their key or their indivisible holdings, or when open business
pins them (§4.2). A change only to a profile or an attachment — a job taken at a similar wage, a policy switched — is
applied to the row's counts in place.

**Parts** are rows with **day-local identities**: a part carries only its own profile entries, rows and positions.
At 10b a part either lands in a cell — its identity resolves to that cell and the records naming it are re-pointed
through back-pointers — or becomes a cell with a permanent identity. Members pinned by open business (§4.2) land only
with identical items. A change that applies to every member of a cell alike (a key rule changing) **re-keys** the
cell in place instead of making a part; members crossing an age class on their own birthdays (REP.25) are parts.

### 7.6 Landing

- **Steps**: each position declares a base partition of its range on the member's own scale (REP.4, REP.20),
  non-uniform where responses are steep — finer near default, a covenant, a limit (REP.10) — and coarser levels made
  by merging adjacent steps in pairs, so widening a tolerance (REP.28) re-keys a cell by a shift, never by
  recomputation.
- **The landing key** of a row is (key id, step vector, key-rule kink signature). The step vector covers every
  position, including each deposit row's per-member balance and pending amount (§4.5) and the per-role year-to-date
  positions (§7.8). The signature records on which side of every kink of the key's rules (tax bands, means tests,
  borrowing constraints) the per-member positions lie. Two members are within tolerance exactly when their step
  vectors are equal.
- **The landing index** is a sharded hash from landing key to the cells holding it, in slot order: cells may share a
  key while their lines' kinks or pins differ. A part looks up its key and takes the first candidate that passes the
  **check**, which reads the candidate's own rows (contiguous) for the kinks of either side's lines — a payment due, a
  credit limit, the insured limit — so no join averages a key or crosses a kink (REP.8, REP.16).
- **Batches**: parts are sorted by landing key at 10b; all parts bound for one target are checked against its state at
  10b's start and joined together in one pass over its rows.
- **Clusters**: parts that find no target are grouped by landing key; within a group, in canonical order, each part
  joins the first new cell it passes the check with, or starts one. New cells per day are counted (§13.2).
- **Joining** is one `Landing` instruction per part, whose legs are derived from the part's rows (SET.1); weights,
  totals, profiles, relationship rows (by line and role), payment records and holdings (pooled cost) add.

### 7.7 Relationship counts and their levers

Relationship rows per cell dominate memory (§13.1). Line terms are what the spec says they are (LAB.1, REP.3: an
employment line is occupation family, skill, wage point, hours, notice and severance terms, start band and region,
with its employers on the other side). The banking arrangement is key (§4.5), so the number of distinct arrangements in a
region is a floor under its cell count; it is measured with the rows-per-cell curve (§14.6). The levers, all
declarations, all tested on the ladder:

- **Attributes may move from profile to key** (REP.33): an adult role's occupation family or skill in the key
  concentrates a cell's employment rows.
- **Start bands** are RESOLUTION (Law 9): five years at play.
- **Contract grids** are price points and vintages, conventions of each trade (REP.34).
- **Tolerances** (REP.4, REP.28): wider steps mean fewer parts and fewer cells.

### 7.8 Profiles and year-to-date positions

Profiles are counted per role, jointly within declared groups (REP.32, REP.33). Joints the spec requires are carried
by contract terms where they belong: a mortgage's and a dwelling policy's collateral description names zone and class,
so a flood's draws meet the right mortgages and policies (§7.10). Lists are compactly encoded: dense small histograms,
delta and varint coding, one-byte counts with an escape.

**Year-to-date figures** — taxable income, contributions — are **per-role positions** (REP.20, REP.26): each adult
role's total over the cell's members, with steps, fed by the flows that reach that role. The tax schedule's bands are
key-rule kinks in the landing key's signature, so no join averages across a band (REP.16). A member who leaves takes
its share k·A/W with it (REP.9); a member laid off keeps its figure, since the position belongs to the role, not the
line. The annual assessment reads the positions (TAX.2).

### 7.9 Choice groups and pieces

Only market kinds whose purchases are used up at once hold choice groups (REP.37). A cell's members in one zone form a
**piece** (a household's zone is a profile, REP.24); the choice probabilities of a meeting depend on the zone, the
preference type and the positions the choice reads, so a **group** is the pieces that share those (a declared key per
market kind, a step vector only where probabilities read positions). Group budgets are aggregates of the pieces'
standing flows (§7.4), kept incrementally. Each day, per (group, product), counts are drawn over seller cells by
conditional binomials; each seller cell takes the day's demand as its total, and on its review days its sales since
the last are spread over its members by the capacity-respecting draw of REP.22, each member taking its own revenue and
units, which is when identical sellers part company (counted as parts, §13.2). Each buyer cell pays its
own budget and receives its share at the group's mix. Rounds of re-choice after capacity binds are counted and budgeted (§13.2).

### 7.10 Places and catastrophes

`phx-geo` keeps **physical stock per (tile, class)** as the one writer of where units stand, and an index listing,
per (zone, class), the **holding** rows (owners) of that class there. A catastrophe at 3a draws in two levels:

1. the units lost are allocated across the holdings of the struck (zone, class) by one multivariate hypergeometric
   draw, and each holder's lost units come from its own count;
2. for each holder hit, its dwelling-role attachments — mortgage, dwelling insurance, tenancy — are drawn jointly for
   the hit members (REP.32), since their terms name the same zone and class.

A landlord's lost units reach its tenants: the tenancies of the struck (zone, class) on that landlord's lines are
drawn from their tenant side (REP.23), and each hit tenant receives a notice occasion to move. Losses are scattered
back as parts; an insured loss opens a **claim** message to the insurer; a mortgage whose collateral is lost stays a
loan with its collateral description marked lost, and its lender reads that on its next review (REG.9, BNK.17).

### 7.11 Tolerance control, promotion, the reference run

- **Tolerance control** (REP.28) runs at 10b **on the day the cells carried exceed the budget**: it merges steps
  where the decision gap is smallest — estimated from pure decision-point forms over landings sampled from the
  **representation's own world stream** — and lands the cells that now share a landing key, that day. It is a
  declared full sweep, budgeted on the heavy day (§13.2). Narrowing runs on declared light days and stops when the
  cells carried reach a declared share of the budget, so a heavy day's new cells rarely trigger widening.
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
  A row's own outlooks are updated at its visits; a **surprise** (VAL.4) wakes it and raises its attention (REP.35).

---

## 9. Endings, estates and resolution

### 9.1 Endings

| Ending | Handled by | How |
| --- | --- | --- |
| A cell member's death | `sys-dem` | The role leaves the household; if the household ends, an estate row in `phx-core`'s estate table, behaviour in `sys-est` |
| Household, firm, fund or political-party estate | `sys-est` | Sells what its debts need; pays by the country's law through the ledger's waterfall (L3), as instructions settled at stage 7; passes the rest in kind (POP.9) |
| Personal insolvency | `sys-hh` | The procedure (HH.21) through an estate row |
| Bank, insurer | `sys-sup` | Resolution (§9.2); the rest to an estate |
| Pension scheme | `sys-pen` | Sponsor contributions, benefit cuts, the guarantee fund, then an estate |
| Clearing member, clearing house | `sys-drv` | Close-out and porting (§9.3); the house's waterfall; beyond it, `sys-sup` |
| Public agency | `sys-soc` | Its duties and staff pass to a successor agency named by the budget |
| Sovereign | `sys-trs` | Default and exchange offer |

Estate rows are short-lived individuals; about 1 million a year pass through, a few tens of thousands at once
(§13.1).

### 9.2 A bank's failure

1. **Day D, 8f**: the bank cannot repay intraday credit. The shortfall becomes an overdue claim of the central bank on
   the bank, recorded on both books (MON.12); its liquidity failure is recorded (BFL.10). An insolvency found at 9c
   (SUP.5) starts the same path.
2. **D, 9c**: `sys-sup` triggers resolution and invites bids by message. From now until its transfer settles the
   bank is **closed**: it makes no payment of its own; its customers' outgoing payments wait as pending on their rows;
   payments to them wait as pending receivables of the payer's leg, settling when the rows reach their receiving
   bank.
3. **D+1 (next business day), 2b**: the authority values the book from D's valuations (MKT.20), writes down equity,
   converts or writes down contingent capital and subordinated debt, then senior debt as needed. **5c**: eligible
   acquirers bid (TIME.6); the authority takes the best bid or none.
4. **D+1, 7**: the transfer settles as one instruction. Each deposit row splits at a kink (§4.4): the insured amount
   and its pending payments move to the acquirer — or, with no acquirer, to a paying bank the insurer chooses — and
   the rest becomes a claim line on the estate; pending payments beyond the insured amount fail, visibly, against
   the estate claim (MON.5). The **consideration** is a leg of the same instruction: the acquirer
   takes the assets it bid for, and the estate or the insurer pays the difference to the insured deposits it assumed
   (SUP.6). The insurer becomes the estate's creditor; a short fund draws its treasury backstop. SUP.7's identity is an
   audit family on the instruction.
5. **D+2**: customers pay through their receiving bank; their pending payments settle there.

### 9.3 A margin call unmet

The call is issued at 9c of day D and due at D+1's 2c (TIME.7). Unmet, the member's default is recorded; at 5c the
house posts close-out orders and invites other members to take the defaulter's clients (DRV.9); they form at 6a and
settle at 7; at 9a the close-out is valued; at 9c the house runs its waterfall; at D+2's 2e the losses land on named
holders and the defaulting member's estate opens.

---

## 10. The opening world (GEN)

### 10.1 Phases

Declared phases — **parties, physical stock, contracts, balances, history** — each system contributing what it owns,
with declared reads and writes. Each joint distribution has one owning system; each opening line kind names its
writer (the loan line's writer is `sys-bnk`, whoever draws the dwelling).

### 10.2 Drawn and derived sides

For every line kind and physical class, one side is **drawn** and the other **derived**, declared with the kind:

- **Households are drawn** — members, roles, employment status, occupation, tenure, loans, deposits, holdings — from
  census-like distributions (GEN.2).
- **Counterparty sides are derived**: an employer's realised headcount, a landlord's tenancies, a bank's deposit and
  mortgage books, the dwelling stock per (zone, class). Each stratum's total is **apportioned** across the eligible
  counterparties drawn for it in proportion to their drawn size (firm size by industry, a bank's market share, a
  landlord's portfolio) by largest remainder, ties broken by lot from a named opening stream. Drawn sizes are
  apportionment weights; the realised sides are what exist (Law 4), and each counterparty's difference from its drawn
  size is reported (GEN.4).
- **Vacancies and vacant dwellings** are drawn from their own rates on top of the derived stock.
- **Unmatched demand**: a stratum with no eligible counterparty takes the declared substitute (the nearest zone, the
  next class) and otherwise stays unmatched — unemployed or recorded homeless (HSG.13) — each reported with its count
  (GEN.8).

### 10.3 Canonical drawing

- **Pass A**: institutions, firms and their drawn sizes are drawn; every household is drawn **complete** from
  per-member counter keys, in parallel, and only stratum totals are kept.
- **Allocation and balancing** (GEN.4) run on the totals: counterparty sides are derived (§10.2); institutions'
  balance sheets close through ledger operations of reason `Balancing`, each reported.
- **Pass B** redraws every household identically from the same keys, applies the allocations, and lands them in bulk
  in sort order at the run's resolution; the reference run lands none.

The same seed therefore gives the same world at every rung (PTY.12), and nothing is balanced after merging. Drawing
the full population takes tens of seconds on the phone's cores.

### 10.4 History and settling

The opening history is written into public records and marks; outlooks start from it. Settling (GEN.6) runs the
ordinary day for the owner's length, a world setting; its history is kept.

### 10.5 Settled worlds for testing

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

- **Views** are built at 10e from records, incrementally maintained fixed-bin histograms and tracers, and swapped in
  behind an `Arc`; tables cross the FFI in pages; `phx-obs` writes nothing (Law 17).
- **Tracers** follow members through splits by the observer's stream, conditioned on their profile values (REP.30).
- **The player** is an individual (OBS.4) whose decider fact names the player. A queued intent is a **wake**: at 1c
  of the first day its decision point runs, it gives the player an occasion for that decision (REP.21) and is decided
  there; until then it stays queued. On days the player has queued nothing for a scheduled decision, the rule decides
  (OBS.6). The player appears in every audit family.
- **On the phone**, `phx-ffi` runs the engine on its own thread with the pinned pool: create, load, step a turn, read
  a view page, submit an action, save. The bench flavour runs a declared number of turns headless and writes a JSON
  report.

---

## 13. Budgets

Every line below is **count × unit cost**, each with the counter that ratchets it (§16.8). Unit costs are the
third review's **measured** kernels, scaled to a tuned phone core; counts are estimates for the coarsened
representation (spec Appendix E 31). The **design point** is 0.7 million household cells (average weight about 170),
0.25 million firm and business cells, and 2,000 zones. The first measurements (§14.6) replace every number here; the
cell budgets, tolerances and zones are RESOLUTION and are set where both budgets hold with at least 10% headroom
(N8.5).

### 13.1 Memory (4.5 GB resident, N8.4), at the worst day's peak

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household cells: landing-hot line, positions (with each deposit row's), rates, review phases and exposures, attention, arena references | 0.7 M | 464 | 325 MB |
| Household profiles, compact | 0.7 M × 150 entries | 2 | 210 MB |
| Relationship rows (40 per household cell, 12 per firm cell, 2 M of individuals) | 33 M | 23 average | 759 MB |
| Line holder lists, with block slack | 33 M | 6 | 198 MB |
| Holdings and instruments' holder lists | 5.25 M | 28 | 147 MB |
| Firm and business cells: rows, keys, profiles | 0.25 M | 500 | 125 MB |
| Lines (kind, terms id, side totals, holder-list reference) | 3 M | 16 | 48 MB |
| Interned keys and terms with their sharded hash | 1.5 M | 72 | 108 MB |
| Landing index (sharded) | 0.95 M | 38 | 36 MB |
| Agenda: next days per (row, process) and the day calendar | 0.95 M × 16 | 7 | 106 MB |
| Group aggregates and pieces | — | — | 60 MB |
| Kind tables of individuals and their facets; estates | 0.15 M | 1.5 KB | 225 MB |
| Instruments, lots, liens, commitments, messages that live across days | — | — | 150 MB |
| Markets, marks and fixings history; public records; opening history; events | — | — | 150 MB |
| Map, network, deposits, stock per (tile, class) and its index | — | — | 80 MB |
| Directory with bounded tombstones | — | — | 50 MB |
| Day buffers at the worst day (payee reduction streamed shard by shard; parts; intents; sort scratch) | — | — | 600 MB |
| Arena slack and page tails (15% of variable-length stores) | — | — | 197 MB |
| Renumbering slice, save buffers | — | — | 94 MB |
| Views and tracers | — | — | 60 MB |
| Android process baseline | — | — | 250 MB |
| **Total** | | | **3 978 MB** |

The design point peaks at about 3.98 GB against 4.5 GB: 12% headroom. Rows per cell rise as cells get heavier, so a
smaller cell budget saves less than proportionally; the curve is measured (§14.6). The weight-one reference run
needs about 120 GB.

### 13.2 Time (1 s median, 2 s worst, N8.2)

Unit costs are **phone core-nanoseconds**; wall time is core time over the phone's **sustained** parallel speed,
taken as **3 core-seconds per second** (one fast and five medium cores after a thermal soak) until Stage 0 measures
it per core class (§14.6). A turn's time is the sum of its days: an ordinary business day; a **non-business day**
(TIME.8: no institutions, settlement, funding or valuation; reviews only for the decisions the day's meetings need);
or a **heavy business day** (a quarter-end payday after a holiday, with the carried needs).

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Hazard and need candidates (scheduled, thinned) | 1.0 M | 100 ns | 33 ms | 33 ms | 33 ms |
| Agenda maintenance: redraws after weight changes | 2.4 M | 30 ns | 24 ms | 17 ms | 36 ms |
| Row visits: continuous decisions on schedule, kinks | 0.28 M | 500 ns | 47 ms | 17 ms | 55 ms |
| Group-aggregate updates from changed rates | 11 M | 20 ns | 73 ms | 27 ms | 85 ms |
| Occasion evaluations, per (row, decision, profile combination) | 2.4 M | 80 ns | 64 ms | 13 ms | 80 ms |
| Choices of acting members | 0.25 M | 400 ns | 33 ms | 10 ms | 40 ms |
| Parts: from decisions, kinks and age (0.15 M) and seller spreads (0.15 M) | 0.3 M | 2.5 µs | 250 ms | 67 ms | 375 ms |
| Meetings and choice groups, re-choice rounds | 0.2 M group-products | 650 ns | 43 ms | 43 ms | 43 ms |
| Seller spreads on review days | 0.05 M seller cells | 0.8 µs | 13 ms | — | 13 ms |
| Labour matching | 0.1 M searching groups | 1 µs | 33 ms | — | 33 ms |
| Settlement: rows read in the payer pass; legs | 30 M; 2 M (60 M; 4 M heavy) | 5 ns; 60 ns | 90 ms | 3 ms | 180 ms |
| Institutions, financial markets, the state | — | — | 83 ms | 10 ms | 133 ms |
| Valuation, accounts, tests, publications | — | — | 27 ms | — | 67 ms |
| Audit, statistics, events, views | — | — | 40 ms | 27 ms | 53 ms |
| Barriers and tails | ~60 sub-steps with work | — | 30 ms | 20 ms | 35 ms |
| **Total** | | | **883 ms** | **287 ms** | **1 261 ms** |
| Tolerance control, on a day the cells carried exceed the budget | 0.95 M cells | 300 ns + joins | +170 ms | +170 ms | +170 ms |

| Turn | Days | Time | Budget | Headroom |
| --- | --- | --- | --- | --- |
| Ordinary weekday (the median turn) | 1 business | 883 ms | 1 000 ms | 12% |
| Monday after a weekend (with carried needs and pending settlement) | 2 non-business + 1 business | 1 497 ms | 2 000 ms | 25% |
| Heavy Monday (month- or quarter-end payday) | 2 non-business + 1 heavy | 1 835 ms | 2 000 ms | 8% |
| Heavy Monday with tolerance control | 2 non-business + 1 heavy | 2 005 ms | 2 000 ms | **misses** |
| A four-day holiday block ending on a heavy day (Easter) | 4 non-business + 1 heavy | 2 409 ms | 2 000 ms | **misses by 20%** |

At these estimates the **median fits**; **heavy Mondays fit with less than the required 10%**; **tolerance control on
a heavy day and the longest holiday blocks miss**. For the Easter block to keep 10% headroom the non-business day must
cost at most (1 800 − 1 261) ÷ 4 ≈ **135 ms**, less than half the estimate. Tolerance control rarely falls on a
heavy day if narrowing on light days stops at a declared share of the cell budget, leaving room for a heavy day's new
cells (§7.11); how often it still does is measured. The longest block is read from the declared calendars
(TIME.2) at S0.07. Stage 0's measurements decide: measured unit costs first, then wider tolerances (fewer parts) and
coarser zones (fewer groups), which are RESOLUTION. If no play resolution meets the budget within the accuracy for
play, that is a finding, and the budget is the owner's to decide (N8.7). No causal date is moved (N8.9).

### 13.3 Storage (4 GB, N8.4)

A full save of the design point is about 1.5 GB after transforms; the latest save plus one being written stay within
4 GB.

---

## 14. Testing and measurement

### 14.1 Evidence

1. **Compile level**: distinct identifiers, currency-tagged money, typed handles, contexts that cannot reach later
   state or other parties' rows, `Missing<T>` without defaults.
2. **Logic level**: pure functions — samplers, rounding, contract legs, clearing, waterfalls, step functions,
   landing arithmetic, screening, levies per member, standing-flow posting — under `cargo test`. No test depends on
   `phx-world` or the opening.
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
| `phx measure` | the representation's own numbers: rows per kind, relationship rows per cell by line kind, profile entries per role, bytes per store and peak resident bytes, agenda rows, candidates and hits per process, occasions and evaluation groups, parts and new cells per day by cause, choice groups and draws, legs per batch, rows and bytes per sub-step, unit costs per line of §13.2 |
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

Stage 0 carries the opening world's employment, tenancy, deposit and loan lines paying as their terms say (spec
Part O), so its world already has paydays, dues, deaths, illness, ageing and catastrophes. Before any behaviour is
built it measures, on the phone, and the Stage 0 gate judges them:

1. **Rows per cell by line kind** — employment, tenancy, deposit, loan and the rest — and profile entries per role, as
   a **curve over three or four cell budgets**, at the opening and after a simulated year; distinct keys and banking
   arrangements per region, the floor they put under the cell count.
2. **The phone**: sustained core-seconds per second by core class after a 30-minute thermal soak; random-gather
   nanoseconds per row over 1–3 GB with 16 KiB pages and prefetch, with all cores gathering together; sweep
   bandwidth; barrier cost.
3. **A part end to end** at the measured rows per cell: split, landing check, join, holder-list maintenance,
   agenda redraws, profile merge, instruction; parts and new cells per day by cause.
4. **Settlement on the heaviest payday**: rows read and legs, nanoseconds per row and per leg including levies, the
   fixed point's iterations, and the peak of the day buffers.
5. **Screening and agenda**: candidates, redraws and agenda rows per day, with their unit costs.
6. **The worst turn**: the longest holiday block in the declared calendars times the measured non-business day.

Stage 1's gate adds retail and labour: choice groups and draws, group-aggregate updates per visit, seller spreads,
occasion evaluations and choices by decision. From these, §13 is rewritten with measured numbers. If they do not
fit, the remedies are, in order (N8.7): how the world is represented and traversed; then the play resolution — the
cell budget, the tolerances and the zones. If no play resolution meets both the budget and the accuracy (N8.5), that
is a finding, and the owner decides; the population is never reduced.

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test`; `phx-check`; release build with thin LTO and `read-trace`; the live world at a **declared reduced cell budget**, labelled so, briefly settled and run 60 days with every live check and the run-comparison guards; counter ratchets |
| `arm.yml` | every push, where the plan provides arm64 runners | release build and a 30-day live run on arm64 Linux |
| `android.yml` | every push to `main` | the app and bench flavour, fat LTO |
| `nightly.yml` | nightly, on a larger runner | fat LTO; the world at the play resolution, settled at the owner's length; two simulated years; peak memory; the full report |

The per-push run length is sized to keep CI under 30 minutes on standard runners; the play resolution needs about
4 GB and runs nightly on the larger runner.

---

## 15. Guards on the running world

- **The audit** (N1) streams over batches as they apply and checks against **independent records**: per-batch digests
  kept at apply time; issuers' own totals per deposit and loan line; issued amounts per instrument; banks' maintained
  totals of standing flows against their depositors' rates on a rolling cycle. Injection mode lights each family
  alone (`phx inject`).
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
   identifiers in comments; interface crates without behaviour; no full sweep in a sub-step not declared as one.
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
8. **Ratchets** on deterministic counters (§14.7): kernel instruction counts; bytes per store, per row and at peak;
   rows and bytes touched per sub-step; agenda rows, candidates, evaluation groups, parts, new cells and landings per
   day; legs per batch; barriers per day; cells per kind; rows per cell by line kind. Declared-but-never-read
   primitives, streams and hazards are reported.

A rule changes only with its reason recorded in §18.

---

## 17. Build and target

- Release: `lto = "fat"` (nightly and device builds; `thin` per push), `codegen-units = 1`, `panic = "abort"` with a
  hook that writes the violation report; profile-guided optimisation from bench-flavour profiles.
- Phone target features: `+lse,+rcpc,+dotprod,+fp16`; NEON by auto-vectorisation and in `phx-rand`'s samplers;
  64-byte aligned, padded columns; software prefetch on holder-list and index gathers.
- Pool sized to fast and medium cores; performance-hint sessions per turn.

---

## 18. Decision record

1. Rust, own pool, own random generator, pure-Rust mathematics.
2. Systems cooperate only through kernel channels (§4); interface crates hold shared vocabulary and rule signatures.
3. Individuals of institutional kinds are kernel rows with system-owned facets (§4.1).
4. **Relationships are rows in the holder's arena**, found from the line by its holder list (§4.5).
5. A cell's **banking arrangement is key**; balances live on rows, one per deposit kind and bank (§4.5).
6. Levies are computed per member and may carry a follow-on written by their payee system (§4.3).
7. Sub-steps with declared reads and writes; order-free gathers (§6.2).
8. **Cost follows events**: the agenda (§7.3); continuous decisions and reviews on each cell's own days; standing
   flows settled as legs in a streamed pass; pooled flows, one leg per party per batch; only accruals posted lazily
   (§7.4).
9. Screening is scheduled at an envelope rate with thinning by default; daily only for dense processes (§7.3).
10. Settlement: implicit batches never materialised; a streamed payer pass; the greatest fixed point; failure per
    payer (§6.5).
11. **Tolerances are steps** (REP.4, amended): landing is one lookup of the landing key, a check of the lines' kinks,
    batches per target and clusters of the unlanded (§7.6).
12. Open business pins only what belongs to particular members; balances are positions with steps (§4.2).
13. A trade is one composite instruction drawing on other systems' commitments (§4.4).
14. Canonical two-pass drawing, households drawn and counterparties derived, makes every rung the same world (§10).
15. Saving pauses the world at declared moments (§11), as SET.12 and N8.10 allow.
16. Demands due today settle in stage 2 so consequences fall the same day (TIME.6, TIME.7); a failed bank is resolved
    at the next business day, its transfer settling at stage 7 (§9.2).
17. A turn runs to the next business day in any country; each country's markets follow its own calendar; non-business
    days run what TIME.8 (amended) lists (§6.1).
18. `sys-dem` carries the spec's POP.
19. Freight within each country arrives in Stage 1; firms, banks and central banks exist as parties from Stage 0
    (Part O records both).
20. **Measure first**: Stage 0 carries the opening lines paying as their terms say, and the phone, the rows-per-cell
    curve, the unit costs and the worst holiday block decide the play resolution before behaviour is built (§14.6).
    The design point's estimate misses only the longest holiday blocks, by about 15% (§13.2).
21. Memory budget 4.5 GB, the owner's choice after the design point was sized.
22. **Owner decisions** (spec Appendix E 29–31): the map is about 40,000 tiles of 10 km with 12, 8 and 5 regions;
    the accuracy for play is 5% on means and shares and 10% on tail quantiles beyond seed spread; the representation
    is coarsened for the phone (pooled flows, coarser employment lines, reviews on review days, sellers spread on
    review days). World settings: the
    settling length defaults to **one simulated year** (GEN.6, adjustable); saves default to **every simulated
    quarter** (SET.12).

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
