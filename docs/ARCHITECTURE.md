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
  Continuous decisions are taken on each party's own schedule (TIME.5) and their flows run between visits as
  **standing flows** posted when something reads them (§7.4). Nothing sweeps the population daily.
- **Members who act set the time.** Occasions, parts, landings and choices scale with members, not cells. Each has
  a unit cost with a target, a count, and a counter that ratchets it (§13.2).
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
| Counters | **`iai-callgrind`** for kernel micro-benchmarks; the engine's own counters | Deterministic ratchets; wall time only on the phone. |
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
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>` with a niche, rounding conventions, checked `i64`/`i128` arithmetic, `DeclaredLimit`, **point tables** (a trade's price points as `i64`, REP.34). |
| `phx-rand` | CHN.1, CHN.6, CHN.7 | Philox; stream keys; batch samplers: binomial (inversion, BTPE) and **zero-truncated** binomial, multinomial (conditional binomials; alias tables when draws are fewer than categories), hypergeometric (H2PE) and multivariate hypergeometric, weighted picks over prefix sums, geometric (for next-candidate days), normal, log-normal, Pareto, Gumbel; rejection thinning. |
| `phx-id` | — | Identifier types (`PartyId`, slots, `LineId`, `RowRef`, `InstrumentId`, day-local ids), `Day`, `Date`. |
| `phx-macros` | — | `#[clause]`, `declare_kind!`, `declare_fact!`, `declare_store!`, `declare_message!`, `declare_rule!`, `declare_system!`. |

### 3.3 Kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space; **chunk-local arenas** compacted in place; slot allocators with recycling; column descriptors; save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; cost-sized chunked traversals over the day's **agenda** or a whole table; gathers by prefix sum keyed (chunk, handler); sharded `KeyedReduce`; fixed-tree reductions; radix sorts. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | Calendar and conventions; **decision schedules, wakes and the agenda** (§7.3); the party directory with bounded tombstones; **kind tables of individuals** with facet columns (§4.1); kinds and profiles; the primitive register and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); **rule handles** (§4.7); hazard and occasion declarations; **public records** with audiences (§4.9); events; findings; contract violations; party creation and ending. |
| `phx-geo` | GEO | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure, **physical stock per (tile, class)** and the (zone, class) index of holdings (§7.10). |
| `phx-ledger` | MON, SET, REG, L3's ranking | The **contract algebra** (§4.4); lines and their holder lists; **relationship rows** in holders' arenas (§4.5); instruments, holdings with the holder index, lots, liens, **commitments**; **levies** (§4.3); **instructions**, composite instructions and implicit batches; settlement (§6.5); **standing flows** and lazy posting (§7.4); fail records and payment records; transformation records; **line transfers**, including the split at a kink (§4.4); the estate waterfall. |
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

**Open business that differs per member pins it**: members of a cell with an open message addressed to or from
them, or a commitment that is theirs alone (an accepted mortgage offer, a pending sale), carry it as a key attribute
of their part, so they land only with members holding the identical item, and a reply reaches exactly the members who
asked (REP.16, REP.23). Commitments every member holds alike — card purchases awaiting settlement, undrawn credit on
a held line — are totals that divide by REP.9 and pin nothing.

### 4.3 Levies

A **levy** is a declared deduction or addition on another system's flows (income tax and contributions at payroll,
value-added tax at a sale, duty at a border, pension contributions). It declares: the flow reasons it applies to; its
**base per member** of the side entry (a leg's per-member amount, or a per-person accumulator, §7.8); its schedule (a
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
carried as a **point index** into that trade's point table; a line kind with no point table carries `i64`.

A **line** is one record of identical contracts (REP.3): kind, interned terms, and a **holder list** (§4.5). An
**instruction** carries legs of any reasons and systems and settles all-or-nothing (SET.4). A **composite
instruction** is written by the one handler that accepts a trade and draws on **commitments** other systems wrote:
a commitment (REG.10) records, as its writer's declaration, the legs and the line it creates when drawn, so the
ledger creates a loan row for `sys-bnk` without `sys-hsg` writing it (Law 4). A dwelling bought with a mortgage is
one composite instruction written by `sys-hsg`'s acceptance handler: the buyer's deposit, the drawn mortgage
commitment, the payment to the seller, the seller's mortgage payoff, the title and the new loan row, all or none.

A **line transfer** moves a count of a side to a new party — a sale of loans (BNK.10, SEC.2), a client moved to
another clearing member (DRV.9), an estate succeeding a party (L3), a foreclosure (HSG.11) — as an instruction with a
reason, settled by the settlement routine with its money legs; each line kind declares which systems may request
which transfers. A **split at a kink** divides each row of a line by a per-member amount — the insured part of a
deposit (SUP.2, SUP.5) to a receiving bank, the rest to a claim line on the estate — computed per member and
multiplied by the count, so it is exact.

### 4.5 Relationship rows

Every relationship of a holder to a line is a **row in the holder's chunk arena**, contiguous with the holder's
other rows: `{line u32, count u32, point u16, record u16, role u8, flags u8}` (16 bytes), plus `balance i64` for
**accruing kinds** (deposits, loans, a collector's tax payable, any running payable a kind declares) and `accumulator
i64` for **income kinds** (§7.8). `point` is the line's price point, copied because terms never change (REP.3), so a
row's per-member amount is one lookup in a cache-resident point table. `record` packs days in arrears and missed
payments. Each **line** keeps a **holder list** — holder slots sorted, in chunked blocks — so line-major events (a
firm closes, a pairing is drawn, a bank fails) find their holders directly. The line's side totals are kept
incrementally and checked against its holders (REP.31).

- **Holder-major** traversals — a visit, the payer pass of settlement, splits, landing — read a holder's rows
  sequentially; line-major events go through the holder list. Rows grow and shrink by re-appending the holder's run
  at the arena's end and compacting in place; no row index outside the arena names a position in it.
- **Deposits.** A cell's **banking arrangement** — which deposit kinds it holds at which bank — is a **key
  attribute** (REP.33). Every deposit row of a cell therefore has count equal to the weight, and a member's share of
  a balance is balance ÷ weight (REP.9). Each deposit kind declares which payments it funds and in which order
  (current before savings; term deposits never). Changing bank is a lumpy decision whose members split into a part
  (REP.5). Coverage (SUP.2) is per person: limit × the arrangement's adult holders, over the member's balances at
  that bank in the declared coverage order.
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
| 2 Resolve | 2a accruals post where a date needs them · 2b (B) **resolutions** opened by the last business day's failures (§9.2) · 2c (B) **settle** calls and demands due today, and resolution transfers (the settlement routine of stage 7); failures handed to their owners at once (TIME.7) · 2d (B) earlier fails delivered to owners; arrears · 2e (B) recognised losses land; parties that cannot go on end; estates open and compute their waterfall · 2f apply |
| 3 Nature and population | 3a weather; catastrophes (§7.10) · 3b hazards and occasions drawn for agenda rows (§7.3) · 3c demographic events; foundings and moves decided before · 3d overlapping occasions allocated (§7.5) · 3e apply: transformation records |
| 4 Real work | 4a production, services, shipments, construction; jobs starting and ending · 4b apply |
| 5 Decide | 5a public-series outlooks per method · 5b continuous decisions of rows scheduled or woken today, fused per table · 5c lumpy decisions of occasion holders; institutions (B); answers to messages at their declared sub-step · 5d apply |
| 6 Form prices | 6a meetings: retail, services and electricity every day; all others (B) · 6b marks and fixings · 6c apply: matches become instructions; banknotes change hands; on non-business days card payments and electricity trades become commitments settling at the next business day's stage 7 |
| 7 Settle (B) | 7a **payer pass**: per payer, its legs in declared order checked against its funds, streaming over holder-major rows; per-bank nets by keyed reduction · 7b **fixed point**: the greatest set of payments that can settle given one another, with banks' nets and intraday credit, by a fail-only worklist (§6.5) · 7c apply every surviving instruction's legs, deposits and reserves together, with the streaming audit fused in · 7d fails recorded; payees of failed payers drawn (REP.23) · 7e levy follow-ons written (§4.3) |
| 8 Fund (B) | 8a money-market orders · 8b the market forms · 8c its trades settle · 8d standing-facility and lender-of-last-resort requests, reading the supervisor's solvency fact · 8e they settle · 8f intraday credit closes: a bank that cannot repay has the shortfall recorded as an **overdue claim of the central bank** and its liquidity failure recorded (MON.3, MON.12, BFL.10) |
| 9 Value and judge (B) | 9a valuations · 9b accounts and ratios · 9c tests: margins, covenants, capital, solvency; demands issued, due next business day; resolutions triggered and bids invited · 9d acquirers' bids · 9e publications |
| 10 Close | 10a public events · 10b **landing** of the day's parts (§7.6) · 10c tolerance control when the cells carried exceed the budget; monthly ranks; a renumbering slice on declared light days · 10d incremental audit families · 10e views and tracers (read-only) · 10f metrics |

On a non-business day, 5b and 5c run only the decision points that TIME.8 lists; an occasion of any other decision
point stays on its row and is carried to the first day that point runs. Money moves only at 2c, 6c (banknotes), 7 and
8; estate distributions, resolution transfers and line transfers are instructions settled by that routine.

### 6.2 Order without order dependence

Within a sub-step every handler reads the state as it was at the sub-step's start; intents apply at its end. The
handler graph is built from declared reads and writes and refuses conflicts, so registration order carries no
meaning; intent buffers are keyed (chunk, canonical handler id), so gathers and new identities do not depend on it
either. CI proves it by shuffling the registration list (§14.3).

### 6.3 Traversals

`phx-exec` runs one traversal per table per sub-step, in **cost-sized chunks** (declared per table, never dependent
on the thread count), running every handler of the sub-step on a chunk while it is in cache. Each sub-step declares
whether it is an **agenda** pass (the rows the agenda lists, in slot order), a **full sweep** (only where a spec clause
needs every row that day — none in the day as designed) or **index-driven**. A **sweep ledger** counts rows and bytes
touched per sub-step, and a ratchet holds it (§16).

### 6.4 Compute, gather, apply

Handlers write their own rows and emit intents into (chunk, handler) buffers; gathers place them by prefix sum;
applies run in declared order, in parallel over disjoint targets. **One apply routine** serves every apply sub-step:
it settles the instructions of reasons allowed there (§6.1), writes transformation records, emits parts, checks and
feeds the streaming audit. Every reduction runs over a fixed tree.

### 6.5 Settlement

- **Implicit batches** — (line kind or market, rule, day) — are never materialised: debits are generated payer-major
  from holder-major rows (point lookups, levies per member), credits payee-major by keyed reduction; the two sides
  agree by REP.31. **Standing flows** (§7.4) enter as one leg per payer and bank.
- **7a** streams each payer's legs in its declared payment order against its funds and records only per-payer totals,
  a failure flag and the first failing leg; per-bank nets are keyed reductions. Nothing is written per leg.
- **7b** starts from every payment succeeding and removes, until nothing changes, the payers who cannot pay given the
  payments still standing, and the customer legs of banks that cannot cover their nets after intraday credit (MON.3,
  MON.5); only removed payers' payees are revisited. The result is the **greatest** set that can settle, so rings of
  payments that can settle together do (TIME.6). A cell's check is per member: a member's funds are balance ÷ weight
  (§4.5) and its legs are its rows' per-member amounts; members whose legs exceed them fail, drawn from the counts
  (REP.23).
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
  accepted with probability π/π̄, where π = 1 − Π_v (1 − p_v)^{n_v}; if accepted, the counts per value are drawn from
  the binomials conditioned on at least one hit (zero-truncated, CHN.7). This gives exactly the daily binomial counts.
  The next candidate is redrawn when the weight changes or a rate rises above its envelope; a falling rate needs
  nothing.
- **Daily** only for processes dense enough that most rows have a hit most days: `Σ n_v ln(1 − p_v)` with one `exp`
  per (row, process), vectorised, over agenda rows and rows of the process's own declared set.

Hit members are picked by weighted picks over a prefix of the profile counts, O(k log e). Individuals are screened the
same way with counts of one. Occasions of a decision point that does not run today (TIME.8) stay on the row until it
does.

### 7.4 Standing flows

A continuous decision taken on a row's schedule (REP.5: "this week's spending") sets **standing flows**: per-member
rates — spending by category, saving into a named account, production and use of inputs — that run every day until
the row's next decision. They are carried as rates on the row and summed, per group they feed, into **group
aggregates** kept incrementally by keyed reduction when a rate changes (§7.9). Each day:

- markets read the aggregates, not the rows (§7.9);
- money leaves each paying row as one leg per bank in the day's batch (§6.5), from the bank's maintained total of its
  depositors' rates;
- a row's own balances and stocks are **posted lazily**: on each visit, and on the day a posting is due by a date
  that needs it (REP.12, N8.6), the ledger posts rate × days since the last posting, exactly, in whole units.

When a row is scheduled or woken, its standing flows are posted first, so every decision reads current state. At
each decision the ledger computes the day a standing flow would carry a position across a kink (funds reaching zero,
a limit, a band) and puts the row on that day's agenda, so no kink is crossed unseen (REP.16) and the payer check of
§6.5 stays per member.

### 7.5 Occasions, overlaps, splits and parts

Overlapping occasions in one cell are allocated by hypergeometric draws from each process's stream at 3d; each system
decides for the counts assigned to it, evaluated **per (row, decision, profile combination)** with counts; parts are
the product of outcomes in declared process order. A split divides profiles and relationship rows jointly within each
role's declared groups by multivariate hypergeometric draws, and balances divide with their rows; totals leave by
REP.9's rounding. Members whose change leaves every shared attribute in its step and every key attribute unchanged do
not split: a profile or attachment change is applied to the row's counts in place (REP.8's last paragraph).

**Parts** are rows with **day-local identities**: a part carries only its own profile entries, rows and positions.
At 10b a part either lands in a cell — its identity resolves to that cell and the records naming it are re-pointed
through back-pointers — or becomes a cell with a permanent identity. Members pinned by open business (§4.2) land only
with identical items. A change that applies to every member of a cell alike (a clock reaching a new age class, a
key rule changing) **re-keys** the cell in place instead of making a part.

### 7.6 Landing

- **The landing key** of a row is (key id, step vector, key-rule kink signature): each position's step (REP.4) on the
  member's own scale; the signature records on which side of every kink of the key's rules (tax bands, means tests,
  borrowing constraints) the per-member positions lie. Two members are within tolerance exactly when their step
  vectors are equal.
- **The landing index** is a sharded hash from landing key to cell. A part looks up its own landing key: one probe.
  The **check** then reads the target's own rows (contiguous) for the kinks of either side's lines — a payment due, a
  credit limit, the insured limit, each income row's band (§7.8) — so no join averages a key or crosses a kink (REP.8,
  REP.16).
- **Batches**: parts are sorted by landing key at 10b; all parts bound for one target are checked against its state at
  10b's start and joined together in one pass over its rows.
- **Clusters**: parts that find no target are grouped by landing key in canonical order; each group that passes the
  mutual check becomes one new cell. New cells per day are counted (§13.2).
- **Joining** is one `Landing` instruction per part, whose legs are derived from the part's rows (SET.1); weights,
  totals, profiles, relationship rows (by line and role), payment records and holdings (pooled cost) add.
- **Steps are hierarchical**: each position's steps are declared at a base width times 2^k, so widening a tolerance
  (REP.28) merges steps and re-keys cells by a shift, never by recomputation.

### 7.7 Relationship counts and their levers

Relationship rows per cell dominate memory (§13.1). Line terms are what the spec says they are (LAB.1, REP.3: an
employment line is occupation, skill, wage point, start band and zone, with its employers on the other side). The
levers, all declarations, all tested on the ladder:

- **Attributes may move from profile to key** (REP.33): an adult role's occupation family in the key concentrates a
  cell's employment rows; the banking arrangement is key by design (§4.5).
- **Contract grids** are price points and vintages, conventions of each trade (REP.34).
- **Tolerances** (REP.4, REP.28): wider steps mean fewer parts and fewer cells.

### 7.8 Profiles and accumulators

Profiles are counted per role, jointly within declared groups (REP.32, REP.33). Joints the spec requires are carried
by contract terms where they belong: a mortgage's and a dwelling policy's collateral description names zone and class,
so a flood's draws meet the right mortgages and policies (§7.10). Lists are compactly encoded: dense small histograms,
delta and varint coding, one-byte counts with an escape.

**Per-person accumulators** — taxable income to date, contributions to date — live on the income rows that earn them
(the `accumulator` column, §4.5) as the members' total. A person's figure is its income rows' accumulators in its role
plus its share of the cell's capital income by the country's attribution rule (TAX.2). Rows of one line in one cell
are kept apart by the **band** their members' figure lies in, so no row averages across a band kink (REP.16);
accruals that move a row's figure across a band move the row whole, since its members accrue alike. The annual
assessment reads the rows.

### 7.9 Choice groups and pieces

Only market kinds whose purchases are used up at once hold choice groups (REP.37). A cell's members in one zone form a
**piece** (a household's zone is a profile, REP.24); the choice probabilities of a meeting depend on the zone, the
preference type and the positions the choice reads, so a **group** is the pieces that share those (a declared key per
market kind, a step vector only where probabilities read positions). Group budgets are aggregates of the pieces'
standing flows (§7.4), kept incrementally. Each day, per (group, product): counts are drawn over seller cells by
conditional binomials, then over each seller cell's members once per day (REP.22); each cell pays its own budget and
receives its share at the group's mix. Rounds of re-choice after capacity binds are counted and budgeted (§13.2).

### 7.10 Places and catastrophes

`phx-geo` keeps **physical stock per (tile, class)** as the one writer of where units stand, and an index listing,
per (zone, class), the **holding** rows (owners) of that class there. A catastrophe at 3a draws in two levels:

1. the units lost are allocated across the holdings of the struck (zone, class) by one multivariate hypergeometric
   draw, and each holder's lost units come from its own count;
2. for each holder hit, its dwelling-role attachments — mortgage, dwelling insurance, tenancy — are drawn jointly for
   the hit members (REP.32), since their terms name the same zone and class.

Losses are scattered back as parts; an insured loss opens a **claim** message to the insurer; a mortgage whose
collateral is lost stays a loan with its collateral description marked lost, and its lender reads that on its next
review (REG.9, BNK.17).

### 7.11 Tolerance control, promotion, the reference run

- **Tolerance control** (REP.28) runs at 10c **on the day the cells carried exceed the budget**: it merges steps
  where the decision gap is smallest — estimated from pure decision-point forms over landings sampled from the
  **representation's own world stream** — and lands the cells that now share a landing key, that day. Narrowing runs
  on declared light days.
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
2. **D, 9c–9d**: `sys-sup` triggers resolution and invites bids by message; eligible acquirers answer at 9d.
3. **Until D+1's 2b** the bank makes no payment of its own; there is no settlement between, and weekend card payments
   wait as commitments.
4. **D+1 (next business day), 2b**: the authority values the book from D's valuations (MKT.20), writes down equity,
   converts or writes down contingent capital and subordinated debt, then senior debt as needed; chooses the best
   bid or none. Insured deposits leave by the **split at a kink** (§4.4) to the acquirer, or, with no acquirer, to a
   paying bank chosen by the insurer (SUP.6), which receives the insurer's payment; the rest of each row becomes a
   claim line on the estate. The transfers' money legs settle at 2c; customers' payments of D+1 run through their
   receiving bank at stage 7. The insurer becomes the estate's creditor; a short fund draws its treasury backstop.

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
  mortgage books, the dwelling stock per (zone, class). Each stratum's total is allocated across the eligible
  counterparties drawn for it by a **multinomial in proportion to their drawn size** (firm size by industry, a bank's
  market share, a landlord's portfolio), from a named opening stream. Drawn sizes are allocation weights; the
  realised sides are what exist (Law 4).
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

Every line below is **count × unit cost**, each with the counter that ratchets it (§16.8). The counts are a
**provisional design point**: 1.4 million household cells (average weight about 86) and 0.7 million firm and
business cells. The first measurements (§14.6) replace them; the cell budgets are RESOLUTION and are set where both
budgets hold with at least 10% headroom (N8.5).

### 13.1 Memory (4.5 GB resident, N8.4), at the worst day's peak

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household cells: landing-hot line, positions, rates, schedule slots, arena references | 1.4 M | 256 | 358 MB |
| Household profiles, compact | 1.4 M × 90 entries | 2 | 252 MB |
| Relationship rows, all holders (21 per household cell, 8 per firm cell) | 35 M | 20 average | 700 MB |
| Line holder lists | 35 M | 4 | 140 MB |
| Holdings and instruments' holder lists | 8 M | 28 | 224 MB |
| Firm and business cells: rows, keys, profiles | 0.7 M | 400 | 280 MB |
| Lines (kind, terms id, side totals, holder-list reference) | 5 M | 16 | 80 MB |
| Interned keys and terms with their sharded hash | 2.5 M | 72 | 180 MB |
| Landing index (sharded, 2²² buckets) | 2.1 M | 34 | 72 MB |
| Agenda calendar | 2.1 M | 8 | 17 MB |
| Group aggregates and pieces | — | — | 40 MB |
| Kind tables of individuals and their facets; estates | 0.15 M | 1.5 KB | 225 MB |
| Instruments, lots, liens, commitments, messages that live across days | — | — | 150 MB |
| Markets, marks and fixings history; public records; opening history; events | — | — | 150 MB |
| Map, network, deposits, stock per (tile, class) and its index | — | — | 80 MB |
| Directory with bounded tombstones | — | — | 50 MB |
| Day buffers at the worst day: parts (0.6 M × 256 B), intents, per-payer totals and flags, sort scratch | — | — | 400 MB |
| Arena slack and page tails (12% of variable-length stores) | — | — | 160 MB |
| Renumbering slice, save buffers | — | — | 94 MB |
| Views and tracers | — | — | 60 MB |
| Android process baseline | — | — | 250 MB |
| **Total** | | | **3 962 MB** |

The design point peaks at about 3.96 GB against 4.5 GB: 12% headroom. Rows per cell rise as cells get heavier, so a
smaller cell budget saves less than proportionally; the curve is measured (§14.6). The weight-one reference run
needs about 120 GB.

### 13.2 Time (1 s median, 2 s worst, N8.2)

Unit costs are **phone core-nanoseconds**; wall time is core time over the phone's **sustained** parallel speed,
taken as 4 core-seconds per second until Stage 0 measures it (§14.6). A turn's time is the sum of its days: an
ordinary business day, a **non-business day** (TIME.8: no institutions, settlement, funding or valuation), or a
**heavy business day** (a quarter-end payday after a weekend, with the carried occasions).

| Work | Count per business day | Unit target | Business day | Non-business day | Heavy day |
| --- | --- | --- | --- | --- | --- |
| Agenda and screening: daily processes; scheduled candidates | 2.1 M rows × 8; 1.2 M | 4 ns; 50 ns | 32 ms | 32 ms | 32 ms |
| Row visits: posting, continuous decisions on schedule | 1.2 M household, 0.55 M firm | 200 ns, 300 ns | 101 ms | 80 ms | 120 ms |
| Occasion evaluations, per (row, decision, profile combination) | 2.6 M | 60 ns | 39 ms | 20 ms | 79 ms |
| Choices of acting members (random utility, alias draws) | 0.4 M | 300 ns | 30 ms | 20 ms | 50 ms |
| Parts: split, landing check, join, instruction | 0.4 M | 1.2 µs | 120 ms | 90 ms | 180 ms |
| Meetings and choice groups, re-choice rounds | 0.5 M group-products | 280 ns | 35 ms | 35 ms | 35 ms |
| Labour matching | 0.2 M searching groups | 300 ns | 15 ms | — | 15 ms |
| Settlement: payer pass, fixed point, apply; funding | 6 M legs (45 M heavy) | 15 ns | 30 ms | 5 ms | 184 ms |
| Institutions, financial markets, the state | — | — | 70 ms | 10 ms | 110 ms |
| Valuation, accounts, tests, publications | — | — | 25 ms | — | 50 ms |
| Audit, statistics, events, views | — | — | 35 ms | 25 ms | 50 ms |
| Barriers and tails | ~60 sub-steps with work | 0.3 ms | 40 ms | 30 ms | 45 ms |
| **Total** | | | **572 ms** | **347 ms** | **950 ms** |

| Turn | Days | Time | Budget |
| --- | --- | --- | --- |
| Ordinary weekday (the median turn) | 1 business | 572 ms | 1 000 ms |
| Monday after a weekend | 2 non-business + 1 business | 1 266 ms | 2 000 ms |
| Heavy Monday (month- or quarter-end payday) | 2 non-business + 1 heavy | 1 644 ms | 2 000 ms |
| Holiday Monday that is also a quarter-end payday | 3 non-business + 1 heavy | 1 991 ms | 2 000 ms |

The last row has no reserve: whether it occurs is a property of the declared calendars (TIME.2), which the builder
reads before Stage 1's gate; if it occurs, the unit targets above must be beaten or the play resolution falls. The
targets are aggressive — per part and per visit most of all — and are the first things Stage 0 measures. Election
days and filing deadlines are spread by campaign intentions and filing windows (POL.4, TAX.2).

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

Stage 0 measures, before any behaviour is built:

- **The phone**: sustained core-seconds per second after a 30-minute thermal soak; random-gather nanoseconds per row
  over 1–3 GB with 16 KiB pages and prefetch; sweep bandwidth; barrier cost with parked and spinning workers.
- **The representation**: relationship rows per cell by line kind and profile entries per role, as a **curve over
  three or four cell budgets**; bytes per store and peak resident bytes.
- **Unit costs** on the Stage 0 world, which already has deaths, illness, ageing and catastrophes: per part (split,
  landing, join, instruction), per screened (row, process) and per candidate, per visit, per settlement leg.

Stage 1's gate adds the counts: agenda rows, occasions and evaluation groups, parts and new cells per day by cause,
choice groups and draws, legs on the worst turn, and the rows-per-cell curve **after settling and after a simulated
year**, ratcheted. From these, §13 is rewritten with measured numbers. If they do not fit, the remedies are, in order
(N8.7): how the world is represented and traversed; then the play resolution — the cell budget and the tolerances,
which also cut parts. If no play resolution meets both the budget and the accuracy (N8.5), that is a finding, and the
owner decides between the budget and the accuracy; the population is never reduced.

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test`; `phx-check`; release build with thin LTO and `read-trace`; the live world at a **declared reduced cell budget**, labelled so, briefly settled and run 60 days with every live check and the run-comparison guards; counter ratchets |
| `arm.yml` | every push, where the plan provides arm64 runners | release build and a 30-day live run on arm64 Linux |
| `android.yml` | every push to `main` | the app and bench flavour, fat LTO |
| `nightly.yml` | nightly, on a larger runner | fat LTO; the world at the play resolution, settled at the owner's length; two simulated years; peak memory; the full report |

The per-push run length is sized to keep CI under 30 minutes on standard runners.

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
8. **Cost follows events**: the agenda (§7.3), continuous decisions on schedule, standing flows posted lazily (§7.4).
9. Screening is scheduled at an envelope rate with thinning by default; daily only for dense processes (§7.3).
10. Settlement: implicit batches never materialised; a streamed payer pass; the greatest fixed point; failure per
    payer (§6.5).
11. **Tolerances are steps** (REP.4, amended): landing is one lookup of the landing key, a check of the lines' kinks,
    batches per target and clusters of the unlanded (§7.6).
12. Open business pins only what differs per member (§4.2).
13. A trade is one composite instruction drawing on other systems' commitments (§4.4).
14. Canonical two-pass drawing, households drawn and counterparties derived, makes every rung the same world (§10).
15. Saving pauses the world at declared moments (§11), as SET.12 and N8.10 allow.
16. Demands due today settle in stage 2 so consequences fall the same day (TIME.6, TIME.7); a failed bank is resolved
    at the next business day's 2b (§9.2).
17. A turn runs to the next business day in any country; each country's markets follow its own calendar; non-business
    days run what TIME.8 (amended) lists (§6.1).
18. `sys-dem` carries the spec's POP.
19. Freight within each country arrives in Stage 1; firms, banks and central banks exist as parties from Stage 0
    (Part O records both).
20. **Measure first**: the phone, the rows-per-cell curve and the unit costs decide the cell budgets before behaviour
    is built (§14.6).
21. Memory budget 4.5 GB, the owner's choice after the design point was sized.

22. **Owner decisions** (spec Appendix E 29–30): the map is about 40,000 tiles of 10 km with 12, 8 and 5 regions;
    the accuracy for play is twice the reference run's seed spread on every declared read. World settings: the
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
