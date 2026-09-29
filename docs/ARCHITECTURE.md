# Project Phoenix — Architecture

How the world in `docs/PROJECT_PHOENIX.md` is built, top down: technologies, layers, the channels between
systems, the data model, the day, the budgets and the guards. The spec says **what**; this document says **how, in
outline**; `docs/IMPLEMENTATION.md` says **how, step by step**. Where this document and the spec disagree, the spec
wins and this document is fixed in the same change. Clause identifiers in brackets, such as (REP.3), point to the
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
  tens of thousands of large firms and institutions. The world holds a declared number of their persons, every
  household and small firm they form an agent of one party (REP.40).
- **Cost follows events** (REP.12, N8.6). A row is touched on a day only when it is on that day's **agenda**
  (§7.3): a decision it is scheduled to take, a wake, an occasion or hazard hit, a payment or a kink it reaches.
  Continuous decisions are taken on each party's own schedule (TIME.5) and their flows run between decisions as
  **standing flows** (§6.5). The only daily pass over most rows is settlement's stream, which reads one run head per
  holder and enters a holder's rows only on a day something in them is due (§6.5); decisions and hazard hits touch
  only the agents that act.
- **Agents that act set the time.** Occasions, hits and choices scale with the agents something happens to
  (REP.12); the spec's coarsening (Appendix E 31) — employment lines by occupation family and region, reviews on an
  agent's own days — keeps them few. Each has a unit cost, a count, and a counter that ratchets it (§13.2).
- **Relationships set the memory.** A household agent holds rows in many employment lines, tenancies and loans.
  Each relationship is stored once, with its holder (§4.5), and rows per agent are measured across the one run's
  agents before behaviour is built (§14.6).
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
| Parallelism | **Own pool** in `phx-exec` over **rayon-core**, pinned to fast and medium cores, with Android performance-hint sessions | Cost-sized fixed chunks, no little cores, few barriers. Only `phx-exec` depends on rayon. As built at Stage 0 the world runs on one thread: the pool serves only `phx-ffi`'s benches, and performance-hint sessions are not built (`PerfHint` has only `NoHint`, called by nothing); the performance-hint session is built at the plan's S1.16. |
| Hash maps | **hashbrown** + fixed-seed **foldhash**, behind a kernel map type iterated in key order where order matters (`sorted`) and in no set order only for order-free reads such as sums (`each`), sharded where written in parallel | Fast lookups; no outcome depends on hash order. |
| Randomness | **Own Philox4x32-10** in `phx-rand`, batch-first, NEON-vectorised samplers | Draws addressable by (stream, identity, day, index): parallel, order-free, reproducible (CHN.1, CHN.6). |
| Compression | **zstd** level 1 after per-column transforms (delta, zigzag, bit-packing) | Fast; transforms double the ratio on integer columns. |
| Declared data | **TOML** + **serde**, at assembly only | Diffable, never on a hot path. |
| Android bridge | **UniFFI** in `phx-ffi` | One foreign surface; data crosses in pages. |
| Android build | **cargo-ndk** + **Gradle**; 16 KiB page alignment | Required by current Android targets. |
| Interface | **Kotlin + Jetpack Compose** | Native; off the hot path. |
| Command line | **clap** in `phx-cli` | Runs, measurements, reports. |
| Checks | **`phx-check`** (`syn`, `cargo metadata`) + **clippy** `disallowed-*` lists | Structural and type-aware rules (§16). |
| Counters | **`gungraun`** (formerly `iai-callgrind`, on valgrind) for kernel micro-benchmarks; the engine's own counters | Deterministic ratchets; wall time only on the phone. |
| API snapshots | **cargo-public-api** for kernel and interface crates | Kernel surfaces change only on purpose. |
| CI | **GitHub Actions** on x86-64 Linux, with an Android build job. The world runs on the build machine, never in CI | See §14.7. |

External crates are an allow-list in `phx-check`; adding one is recorded in §18.

---

## 3. Layers and crates

```text
L4 apps            phx-cli · phx-ffi → android/ · phx-check
L3 assembly        phx-world · phx-obs
L2 systems         sys-dem sys-hh sys-est ... sys-sta                    (depend on L0, L1, IF only)
IF interfaces      if-base if-pop if-labour if-property if-firm if-banking if-credit
                   if-securities if-risk if-energy if-open if-state      (data and pure rule signatures only)
L1 kernel          phx-store phx-exec phx-core phx-geo phx-ledger phx-pop
                   phx-market phx-acct phx-val phx-audit
L0 foundation      phx-num phx-rand phx-id phx-macros
```

### 3.1 Dependency rules (checked by `phx-check`)

- A crate depends only on lower layers. Inside L0 the order is `phx-macros → phx-num → phx-rand → phx-id`; inside L3
  it is `phx-world → phx-obs`, so the observer reads the world through its inspector and the world never reads the
  observer; inside L1
  it is `phx-store → phx-exec → phx-core → phx-geo → phx-ledger → phx-pop → phx-market → phx-acct → phx-val →
  phx-audit`. Interface crates may depend on L0, L1 and the interface crates before them in the order `if-base →
  if-pop → if-labour → if-property → if-firm → if-banking → if-credit → if-securities → if-risk → if-energy → if-open
  → if-state`. `if-base` holds the vocabulary several domains share — product, occupation-family, skill, capital-kind
  and way identifiers and their declared data, rating scales and notches, money-market tenors, segments and collateral
  baskets — so no two interface crates need each other. A decision point lives in the decider's crate, or, when its
  input or output types need a later interface crate, in the latest crate its types need: a household's founding of a
  firm is `if-pop`'s; its borrowing and its choice of where to live read loan and mortgage terms, so they are
  `if-credit`'s; a founder's founding of a bank is `if-banking`'s; a depositor's bank choice, whose alternatives
  include money-fund units, and a household's choice of holdings are `if-securities`'; a firm's financing, its
  distress and a borrower's answer to a restructuring offer are `if-credit`'s — `distress`'s action `seek_buyer`
  carries no offer type: its apply records the sale opened, and `sys-mna`'s sale process builds the invitations,
  `if-securities`' types, at the next 5c; a creditor's vote on a plan is `if-firm`'s, where the plan is; a bank's
  reserve position (`fund_position`), which reads the corridor, the tenders and the collateral framework, is
  `if-state`'s. Company law's filed accounts are `if-firm`'s, below every crate that reads them; listed companies'
  reports and ratings are `if-securities`', and the rating scale is `if-base`'s, so a lender's assessment and the
  supervisor's risk weights read it from below.
- **Interface crates** contain types, handles, schemas and rule *signatures*; `phx-check` refuses any function with a
  body other than a constructor or a field accessor.
- **A system crate never depends on another system crate.** Only `phx-world` knows every system (§5). Only
  `phx-exec` depends on rayon. Only `phx-store`, `phx-exec` and `phx-ffi` (the foreign boundary: UniFFI's
  scaffolding and Android's performance-hint calls) may use `unsafe`; the one other `unsafe` is the `unsafe impl`
  that `#[derive(Pod)]` expands to, whose layout the derive has checked, and no source outside those three crates
  may write `allow(unsafe_code)`.
- A crate is created when its first step starts, never in advance.

### 3.2 Foundation

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>`, rounding conventions, checked `i64`/`i128` arithmetic, `violation!` and its payload, **point tables** (a trade's price points as `i64`, REP.34). No floating-point type in any store. It has no named comparisons: `min` and `max` are refused (§16) and callers compare inline. |
| `phx-rand` | CHN.1, CHN.6, CHN.7 | Philox; stream keys; batch samplers: binomial (inversion, BTPE) and **zero-truncated** binomial, multinomial (conditional binomials; alias tables when draws are fewer than categories), hypergeometric (inversion when small, Stadlober's ratio of uniforms otherwise) and multivariate hypergeometric, weighted picks over prefix sums, geometric (for next-candidate days), normal, log-normal, Pareto, Gumbel; rejection thinning. |
| `phx-id` | TIME.1, TIME.2 (the day and civil dates), PTY.1 (identities) | Identifier types (`PartyId`, slots, `LineId`, `RowRef`, `InstrumentId`, day-local ids), `Day`, `Date`. |
| `phx-macros` | — | `#[clause]` (which checks each name is a clause identifier), `#[derive(Pod)]`, `#[derive(Saved)]`, `declare_prim!`, `declare_fact!`, `declare_kind!`, `declare_facet!`, `declare_stream!`, `declare_message!`, `declare_hazard!`, `declare_decision!`, `declare_rule!`, `declare_record!`, `declare_family!`, `declare_handler!`. |

**`phx-macros`.** `#[clause("…")]` emits its item unchanged and refuses, at compile time, any name that is not a
system clause (`SYS.n`, two to four capitals), a law (`Law n`), a measurement (`Nn` or `Nn.m`) or a chain (`Ln`);
`phx-check clauses` reads it with `syn`. `#[derive(Pod)]` is the one way a type becomes storable: it takes only a
non-generic `#[repr(C)]` struct whose fields are all `Pod` and none `f32` or `f64`, asserts in a `const` that the
fields' sizes sum to the type's (so padding is refused), and expands to `unsafe impl Pod` under
`#[expect(unsafe_code)]` and the `Sealed` marker. `Pod::layout` lists each integer of the value with its save
transform (`#[save(delta | zigzag | delta_zigzag | plain)]` per field). Hand-written `Pod` impls exist only in
`phx-store/src/pod.rs`: the integers, `phx-id`'s ids and `phx-num`'s column forms, which lie below the derive's crate.

**`phx-num`.** Each computing type has a bare column form that a column's declaration types: `Money { amt, ccy }` and
`Amount`, `Qty { n, unit }` and `QtyRaw`, `Price { raw, ccy, unit }` and `PriceRaw`, `Missing<i64>` and `MaybeI64`
(absent is `i64::MIN`, which no present value may take). A currency `Ccy(u8)` is the index of the country whose
central bank issues it; a unit `UnitId(u16)` indexes the unit table, which gives each unit its kind and its price
exponent (at most 18). `+`/`-` on `Money` or `Qty` of two currencies or units, and overflow, are violations, never
`Err`; there is no `Mul<Money>`, no `f64` conversion and no empty `Sum` (`Money::sum_in(ccy, …)` instead), and a
money figure is multiplied only by a `Count` of identical things. A price is smallest money units × 10⁻ᵉˣᵖ per unit
and may be negative; `value_of(q, p, units, round)` is the one route from quantity to value, in checked `i128` and
rounded once. `Rate { raw, per }` is a fraction × 10¹² per `Year`, `Month` or `Day`; `accrue` takes a `DayFraction`
of the same period, or violates. `Fixed<E>` is a decimal of exponent `E` for positions and outlooks, entered from
`f64` only through `from_f64`, which refuses non-finite and out-of-range values. Rounding is `Round` (`HalfEven`,
`HalfAwayFromZero`, `TowardZero`, `Floor`, `Ceil`, `InFavourOf(Payer | Payee)`), applied by `div_round`;
`split_total(total, k, w)` gives `round(k·total/w)` and the rest, so a split conserves by construction.
`Missing<T>` has no `Default`, `unwrap_or`, `map_or` or `Option` conversions: a reader matches. A point table is a
strictly increasing `i64` array in the registry, searched by `at_or_below` and `at_or_above`. `NumError` (overflow,
non-finite, out of range, zero divisor) is only for pure functions whose inputs meet it in a legal world.

**`phx-id`.** Every id is `repr(transparent)` over its integer, `Copy`, `Eq`, `Ord` and `Hash`, with no `Default`
and no `From` between ids. `PartyId(u64)` comes from the directory's one monotone counter, is never zero, stays below
2⁶⁰ and is never reused; `LineId` and `InstrumentId` are never reused either; a `Slot` is a storage index and may be.
`Day(u32)` counts days from the epoch of `data/world.toml`, with day zero the day before the first; `Day::succ` is
checked and `Day::earlier`/`Day::later` are the named comparisons. `Date` is proleptic Gregorian, converted by
Hinnant's algorithms over `i64` serials; a weekday is derived from the epoch's civil date, which the caller passes,
since `phx-id` reads no data.

**`phx-rand`'s addressing.** A stream's key is words 0 and 1 of `philox([fnv_lo, fnv_hi, seed_lo, seed_hi],
fixed key)`, `fnv` being FNV-1a-64 of the stream's name, so a key depends on its name and the seed alone and adding a
stream changes no other (CHN.1). The counter is `[subject_lo, subject_hi, day, (sub-step ordinal << 24) | block]`,
a `Subject` being a 4-bit tag (world, party, part, line, tile, region, zone, country, market, instrument, an opening's
stratum and ordinal before its party exists, a profile value) over a 60-bit identity; a larger identity or more than
2²⁴ blocks at one address is `capacity_exceeded!`. Draws for different subjects never share a counter, so processing
order cannot change a draw (CHN.6). `Draws` is a cursor over one address; a sampler that rejects draws on from the
same cursor, so its result depends only on the address. `philox_x4` runs four scalar blocks, so the batch cannot
differ from the scalar. `open_unit` is (k + ½)·2⁻⁵², never 0 or 1, so every inversion's logarithm is finite; bounded
integers are Lemire's multiply-shift with the `u128` product. Every sampler is exact up to the 52-bit uniform, with
transcendental functions from `libm`: binomial by inversion, switching to BTPE when both n·p and n·(1−p) reach the
switch point; `binomial_at_least_one` by inversion from one with the normaliser `−expm1(n·log1p(−p))` when zero is
likely and by rejection of zeros otherwise; `binomials_joint_at_least_one` decides each value's zero in turn in log
space, conditioned on the rest reaching one, then draws plain binomials after the first success; multinomial by
conditional binomials, or alias draws when draws are fewer than categories; hypergeometric by HIN inversion when
small and HRUA (Stadlober's ratio of uniforms) otherwise; picks without replacement by a Fenwick tree over counts in
O(e + k log e); geometric as `floor(ln U / log1p(−p))`, `Absent` beyond `u64`, which a schedule reads as no success
within representable time; normal by Wichura's AS241 inverse, gamma by Marsaglia and Tsang, beta from two gammas,
Weibull by inversion.

### 3.3 Kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space; **chunk-local arenas** compacted in place; slot allocators with recycling; column descriptors; save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; cost-sized chunked traversals over the day's **agenda** or a whole table; gathers by prefix sum keyed (chunk, handler); sharded `KeyedReduce`; fixed-tree reductions; radix sorts. At Stage 0 the world calls none of the pool's traversals or `KeyedReduce` (§6.3); `phx-ffi`'s benches and the crates' tests do. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | The **vocabulary every system and kernel crate declares with**: the `System` trait, `Declarations`, handler declarations and contexts (`Ctx`), the sub-step table, audit-family declarations and their read-only context, the **audit sink** (`trait AuditStream` and the touched-row bitmap, which `phx-audit` implements and `phx-world` injects, so `phx-ledger`'s apply feeds the audit without depending on it), opening contributions, the kink registry, the traits kernel crates meet through without depending on each other (`BooksAudit`, `MarketsAudit`, `AccountsAudit`, `LegRecords` and `CellsAudit`, the books, the tape, the accounts, the settled legs and the agent tables as the audit reads them, §10.4c); calendar and conventions; **decision schedules, wakes and the agenda** (§7.3); the party directory with bounded tombstones; **kind tables of individuals** with facet columns (§4.1); kinds; the primitive register, `DeclaredLimit` (a real limit constructible only from the register, a contract's terms, or a physical token that only `phx-ledger`'s holdings and `phx-geo`'s stock can build, for a capacity) and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); **rule handles** (§4.7); hazard and occasion declarations; **public records** with audiences (§4.9); events; findings; party creation and ending. |
| `phx-geo` | GEO | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure, **physical stock per (tile, class)** and the (zone, class) index of holdings (§7.8). |
| `phx-ledger` | MON, SET, REG, L3's ranking | The **contract algebra** (§4.4); lines and their holder lists; **relationship rows** in holders' arenas (§4.5); instruments, holdings with the holder index, lots, liens, **commitments**; instrument events and the instrument's state, of which it is the one writer; `Covered<Qty>`, the quantity an offer of held units takes, which places a commitment on them; **levies** (§4.3); **instructions**, composite instructions and implicit batches; settlement (§6.5); fail records and payment records; transformation records; **line transfers**, including the split at a kink (§4.4); the estate waterfall. |
| `phx-pop` | REP | **Agent tables** (§7.1): attributes, positions and keyed position lists (§4.5), persons and attachments; hazards drawn ahead per agent and the persons a hit reaches (§7.3); outcomes and endings (§7.4), and a household's `combine` and `divide`; the representation (§7.6). |
| `phx-market` | MKT | The six forms (§8), the coupled call and the **linked call** among them; prints, marks, and instruments' and currency pairs' fixings at 6b by the pricing service's declared method; admission hooks; market failures. On the core it holds the posted-price meeting (`meet`), the hiring kernel, the calls, reach and carriage; the administered, book, bilateral and dealer forms, admission, grants, the markets' instances and their audit, which only the old kernel ran, were deleted at S1.24 and are rebuilt on the core with the stages whose markets need them (S2.06, S3.06, S4). |
| `phx-acct` | ACC, MKT.20 | Valuations and valuers; statements; a group's consolidated statement as a pure read; carrying bases and unrealised differences; equity accounts, with the receivables and payables of dues as they fall, posted from the day's settled records by the kernel's own work at 9b on a business day and, on a day with no 9b, at the audit's sub-step (10d) before the close reads them. The ledger's apply raises an equity effect for a money leg, a row leg that adjusts a balance, and a move of a holder's cost of lots, each signed by the leg's move of its party's net assets, so a leg passing through a party moves its equity by nothing; an instruction's money effects are taken on each party's net money in it, so a bank through which two customers pay each other, one deposit down and another up, neither pays nor is paid. On the core it declares the accounting bases alone: the valuations, statements, consolidation and equity accounts read the old kernel's books, were deleted at S1.24 with them, and are rebuilt on the core's contracts in its part h. |
| `phx-val` | VAL | Outlook methods as pure functions; public-series outlooks once per method per day, and for registered series only per registered pair on days with a new print; surprise and confidence arithmetic; the investor schedule. |
| `phx-audit` | N1 | Families; streaming checks, the `AuditStream` the apply routine feeds; independent records (§15); incremental and rolling checks; injection mode. |

### 3.4 Interfaces

Each interface crate is a domain's shared vocabulary: kinds and roles; fact handles; line-kind terms built from the
contract algebra; message payloads; decision-point input and output types; rule signatures; view schemas. **Every item
names the one system that writes or implements it**; assembly checks it (§5.4). A line kind declares the party kinds
each side may hold, and assembly refuses a row of any other (derivatives: individuals only, spec Appendix E 33). A
decision point may have one rule per decider kind; each decision point's rule is registered by its owning system — the
system that owns that kind's decision — except that a lender valuing a claim on its book does so through `sys-bnk`. An
item that only one system may construct carries a **writer token** only that system can build: a lender's
`LoanAssessment` (in `if-credit`) is built only by `sys-bnk`, so every price and provision on a lender's book comes
from it; other systems read it by handle and call `sys-bnk`'s rule handle `loan_claim_value`, never building one.
As built at Stage 0, `if-pop` is the one interface crate: plain constants (roles, attributes, person attributes)
that `sys-dem` declares and the other systems read, not `ItemDecl`s, so assembly checks no writer for them; the
interfaces' list in `phx-world/src/systems.rs` holds only `phx_geo::ITEMS`, `phx-geo` being registered as a system.

| Crate | Domain |
| --- | --- |
| `if-base` | shared identifiers and declared data: products, occupation families, skills, capital kinds, ways (issued at runtime from Stage 6, an improved way stored against its base as factors; the rule handle `labour_per_unit`), units; as built, the products, the way and the way-set identity as data only, what a way takes and finishes and the way-set interner being `sys-tec`'s; rating scales and notches; money-market tenors, segments, collateral baskets, haircuts and limits; agents' participation per asset class; pension kinds, contribution rate points and fund-menu identifiers, so a job's terms need no later crate |
| `if-pop` | households, persons, roles, demographic facts, household decision points (founding a firm among them; `enrol`, `form`, `separate`), personal insolvency law; the education record, schooling and the enrolment attachment; education and family law and the day-local `Meeting` message; the rule handles `skill_now`, `division_shares` and `type_at_formation` |
| `if-firm` | firms, production facts, known ways (one writer, `sys-tec`), pricing and payout decision points; research, imitation and licensing (`innovate`, `licence_quote`, `licence_accept`), licence lines, the patent instrument and the patent register; company and insolvency law, plans and votes; filed accounts |
| `if-labour` | employment terms (the pension's kind and contribution rates among them), the employment attachment's scheme component, vacancies, applications, offers, separations; the rule handle `labour_state`, a read of a role's attachments and participation |
| `if-property` | dwellings, land, tenancies, collateral descriptions, appraisals, sales |
| `if-credit` | loan terms (interbank loans among them), applications and quotes, the lender's assessment, workouts and loan sales, credit-bureau records, trade credit; the decision points that need them (borrowing, where to live, financing, distress, answering a restructuring, arrears and filing, bidding for a failed bank) |
| `if-banking` | deposit terms, banking arrangements and payment order; bank facts; the funding and capital rule handles (`marginal_cost_of_funds` over each lender's declared sources, `capital_charge` under its declared regime); licensing; resolution |
| `if-securities` | bonds, shares, fund units, dealers, securities loans, prime brokerage, listed companies' reports, ratings (`RatingView`), indices, orders; the decision points on them — the depositor's bank choice, households' holdings, `place_cash`, bids, votes |
| `if-risk` | derivative, insurance and pension terms, and the rule handle `accrued_schedule` over a member's DB right; margin demands; close-outs; claims; the protection scheme and the guarantee fund; the decision points on them — positions, cover, collateral, schemes offered, trustees, repair, members' pensions, bids for an insurer's portfolio or a clearing house's service |
| `if-state` | policy values, levies, benefits, agencies, budgets, elections, rule signatures of tax and benefit; the central bank's regimes, tenders and collateral framework, a bank's `fund_position`; the treasury's plan and payment priority |
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
| `phx-obs` | Views, the agent's view (OBS.8), and in the inspector build the realism recorder (§14.8); read-only. |
| `phx-cli` | `run` (with its report), `inject`, `measure` (`calendar` and `budget` at Stage 0; `realism`, `chains` and `register-report` arrive with Stage 7, §14.4); the live-check suite. |
| `phx-ffi` | The engine as an Android library. At Stage 0 it exports `run_bench`, `run_world`, `run_load` and `run_programme` (§12). |
| `android/` | The Compose app and its `bench` flavour. |
| `phx-check` | Law, layering and document checks (§16). |

`phx-ffi`'s bench harness (`bench.rs`) runs inside the app's process: the probe of the phone (`phx_exec::probe`: each
allowed core's rate of fixed work alone and with every core busy, at the run's start, middle and end beside the
thermal status; random 8-byte reads over 1–3 GiB by every worker at once, with and without prefetch; sequential
bandwidth; an empty dispatch hot) and the kernels' micro-benchmarks, each line sent to the app through the
`BenchHost` callback as it completes and judged against its target, and the report written as JSON. An ignored test
runs it whole on the build machine. The `android/` app has `play` and `bench` flavours, its AGP, Gradle and Kotlin
versions pinned in `gradle/libs.versions.toml` and the wrapper; the bench flavour runs off the interface thread with
the screen kept on.

### 3.7 Repository

```text
Cargo.toml · rust-toolchain.toml · rust-toolchain-miri.toml · clippy.toml · .cargo/config.toml · CODEOWNERS
crates/{foundation,kernel,interfaces,systems,assembly,apps}/
android/  data/  perf/  docs/  tools/{build-run.sh,versions.toml,data/}  .github/workflows/ci.yml
```

`data/world.toml` holds the calendar's constants (the epoch, day zero) and the world's units; the generator's constants
— the population, the countries and their regions, the map's size, the settling length — are `data/shared/GEN.toml`'s.
`data/shared/` holds the register's shared files, one or more per system (`<SYS>.toml`); `data/setup/` the default new
game; `data/profiles/<level>/` the country-group profiles, the choices' ranges and each development level's templates of
every system's primitives, the opening's distributions in its `gen/`; `data/names/` the name tables (§10.0);
`data/observer/READS.toml` the observer's declared macro reads, which no world crate reads; `data/inventory.toml` the
opening's inventory of derived values and distributions with their sources; `data/sources/` the fetched raw series and
their notes. `tools/data/*.py` fetch the sources and derive the profiles from them; `tools/versions.toml` pins every
tool beyond the Rust toolchain (the NDK and Android levels, the cargo tools, valgrind, the nightlies `miri` and
`public-api` use). A country's `data/<country>/` is instantiated at a new game into the run's directory, never
committed. `data/measure/` holds the realism reads' registered definitions, which no world crate reads, and
`perf/{realism,chains,register}/` their reports, append-only (§14.8). They are registered before any read of the world
uses them: N3's facts `N3/F01.toml` to `F28.toml`, N4's chains `N4/L01.toml` to `L12.toml`, and the shared
`CREDIT.toml`; later work adds estimators, never edits a definition. `phx-check`'s `preregistration` and `no_tuning`
rules guard them (§16 item 10). `perf/device/` and `perf/measure/` hold the
gates' reports; `perf/build-run/` the build runs' reports (§14.7); `perf/load/volumes.toml` the full-load bench's
volumes (§14.6); `perf/schema/` the device report's schema; `perf/ratchets.toml` the counters' values.

---

Each system's opening technology and distributions are derived by a pair of scripts, `fetch_<sys>.py` into
`data/sources/raw/` and `derive_<sys>.py` into `data/`, each value its country group's median over the economies the
sources report, a group reporting too few taking the developed group's, marked assumed:
- **TEC** (`data/shared/TEC.toml`, `data/profiles/<level>/TEC.toml`): nineteen products aggregating the industries of
  the OECD's inter-country input-output tables (2019, 76 economies), each product's 2019 dollars carried to 2022 cents
  by the US GDP deflator; one opening way per product per country group — inputs per unit made from the tables' uses
  summed over origins (energy carriers turned into quantities at world prices, as the fuels they are made from), energy
  mining's output split into coal (to electricity) and oil and gas, other mining's into
  ore (to basic metals) and stone, hours by ISCO-08 major group from ILOSTAT's employment and hours by ISIC section
  shared among a section's industries by value added, plant by kind per unit of output a year from the OECD's
  net fixed assets (Table 9A) per unit of value added (Table 6), agricultural land (World Bank) for crops and
  livestock, a kilogram of deposit per kilogram extracted; a growing season's lead time for crops and livestock, a day for
  other goods, none for services; every unit started finished; a batch of one.
- **FRM** (`data/profiles/<level>/FRM_industries.toml`, `FRM.industry_by_size`): each size class's firms (rows the
  OECD's classes by their smallest persons employed) over the industries — the group's business owners (ILOSTAT's
  employers and own-account workers) by ISIC section, split among a section's industries by the OECD's enterprise
  counts, tilted toward a class by the industry's share of the class's enterprises over its share of all; agriculture,
  which the OECD does not count, untilted. Finance, real estate and public administration make no product of the ways.
- **CAP** (`data/shared/CAP_kinds.toml`, `data/profiles/<level>/CAP.toml`): each kind's geometric rate (the BEA's
  current-cost depreciation over its net stock), mean service life (the BEA's declining-balance rate over the
  geometric), efficiency shape (the BLS's and ABS's hyperbolic β), lead time (the Census's construction months for
  structures, the M3 survey's months of unfilled orders for equipment) and the product it is bought
  as; each group's net stock of each kind per unit of GDP (OECD Table 9A over Table 6), a group the OECD does not
  report taking the developed median scaled by its Penn World Table stock of each asset per unit of GDP; cultivated
  assets' lead time a dairy cow's months to first calving, intellectual property's none.
- **GDS** (`data/shared/GDS.toml`): three grade classes per extracted product at the terciles of GEO's log-normal
  grade index; the grade's fall as a deposit is worked; each storable product's yearly spoilage in stock and the room
  it is kept in; the standardised products (crops and livestock and the four extracted), which meet in calls. Each
  group's product price levels (`data/profiles/<level>/GDS_prices.toml`, `GDS.price_level`): a product's ICP 2021 price
  level over GDP's, the median over the group's economies in the input-output tables, since a currency's smallest unit
  is a GDP at purchasing power parity's cent while a product's unit is what a cent bought at world-average prices;
  those priced at world prices — the extracted products and energy carriers, the ICP publishing no heading for energy
  alone — at one over GDP's level. Without it every product costs a cent everywhere, the developed group's services
  cost less than their hours and the developing group's traded goods add implausibly much.

`data/shared/FRT.toml` (vehicles' tonne-km a unit a day, speeds and loading days by mode; goods' tonnes a unit),
`SRV.toml` (the weights of price and distance, the reach) and `VAL.toml` (memory and switching-intensity type sets,
γ, κ, θ, the performance record's memory, attention sensitivity, the heuristics tracked) are declared with their
sources by hand.

## 4. Channels between systems

Systems never call each other and never read each other's stores. Everything shared goes through one of these
channels, each with one writer per fact (Law 4) and declared audiences (Law 12).

### 4.1 Facts and facets

A **fact** is a named, typed attribute of parties of declared kinds, declared in an interface crate with its type and
unit, the kinds it applies to, **exactly one writer** (a system, or a placeholder SHAPE naming the system that retires
it), its **audience** (the party, a named authority, public after a lag) and, for agents, whether it is an attribute,
a position or a person's (REP.41, REP.20, REP.26). Facts compile into columns of the agent tables and into **facet
columns** of the kernel's
**kind tables of individuals** — one table per individual kind (central bank, treasury, bank, fund, insurer, scheme,
clearing house, dealer, agency, political party, large firm, estate), owned by `phx-core`, whose columns belong to the
systems that declared them. A bank is one row, written by BNK, BFL, BCP and SUP each in its own columns. A dealer is a
row of the dealer kind, a party of its declared form (a bank's subsidiary or an independent broker-dealer) with its own
equity and books (DLR.1). Publishers — the curve publisher, the pricing service, benchmark administrators, index
publishers, rating agencies, banks' analyst units and pollsters — are large firms with a **publisher facet** (their
method, what they cover, their records), each its own party reading only public records and what it bought; finance
companies are large firms whose form takes no deposits. A system keeps a fact on a kind of individual by declaring it
on the kind (`Declarations::facet`); the assembly refuses a fact no interface exports or not of that kind, and adds its
column to the kind's table before the opening, absent in every row until written; the column is saved with the table.
Writing needs a token only the writer crate can build; reading needs a handle. Party creation and ending are kernel operations requested by declared systems (SUP.9 creates a bank;
SUP.5 ends one).

### 4.2 Messages

A **message** is an addressed record: kind; sender (a party, or an agent with its count); addressee (a party, or a line
side with a count); issued day; due day; the line or instrument it concerns; state (open, answered, lapsed, failed).
Each kind declares, per addressee kind, the **answering system** and **the sub-step it answers in**, and, where an
answer can be accepted, the **acceptance handler**. Assembly refuses a kind that reaches an unanswered addressee kind.

A message to an agent becomes a **notice occasion** for it (REP.21). A money demand opens a commitment
(REG.10). High-volume kinds (job applications, retail requests) carry counts and are **day-local**; kinds that live
across days (quotes, offers, redemptions, calls, claims) are snapshotted (SET.16).

**Open business belongs to its agent**: a message addressed to or from it, a need or notice occasion carried to a
later day (§7.3), or a commitment (an accepted mortgage offer, a pending sale, a trade that settles later than it
fills) is held by the agent until it is answered, drawn or settled, and a reply reaches the agent that asked (REP.16,
REP.23). Amounts that are balances — card purchases awaiting settlement, receivables, undrawn credit on a held line —
are the agent's positions (REP.9). A person's wait for a public service and its
claim awaiting processing are values of its attachments (the service or benefit kind and a week band), changed in
place when it is served. Duty
and import tax at a border are a **demand** customs issues to the importer of record, a message like any other.

### 4.3 Levies

A **levy** is a declared deduction or addition on another system's flows (income tax and contributions at payroll,
value-added tax at a sale, pension contributions). It declares: the flow reasons it applies to; its **base per
contract** of the side entry (a leg's per-contract amount, or the remitter's own year-to-date figure for that line,
never a figure the remitter cannot see, Law 12; the annual assessment reads the holder's positions, REP.20); its
schedule — a **policy value** (piecewise linear between kinks in `phx-core`'s kink registry), the flow's own **line
terms** (a job's pension contribution rates) or a **payee fact** (a scheme's schedule of contributions); the remitter
and payee; whether it is the collector's liability until remitted (TAX.2), carried on an accruing row (§4.5); its order;
and, where the payee system turns the money into something the holder holds, its **follow-on**: an instruction written
by the payee system from the levy's legs (a defined-contribution subscription into fund units at the next net asset
value, PEN.3; an accrual of defined-benefit rights on the holder's DB row, a `Row` leg in a unit that is not money,
§4.5). The amount for a side entry is computed **per contract, rounded per contract, then multiplied by the count**
(REP.9, TAX.7). `phx-ledger` composes levies into the flow's instruction; the levies of one flow that share a base are
evaluated in one pass with one search of their fused kinks, each rounded by its own convention. A **holding levy**
(property tax) is declared on held classes, not on a flow: on its dates `phx-geo`'s (zone, class) index lists the
holders (§7.8), and each holder's amount joins its (party, bank) leg in the payer pass (§6.5). A crossing is not a flow,
so duty and import tax at a border are not levies: customs computes them per shipment through the tax's rule handles and
issues a demand (§4.2).

As built at Stage 1, income tax is a `phx_ledger::levy::Withholding` per (employment line kind, currency), bound on `Books` when the world opens or loads, its payee the country's treasury: each member's levy is the year's share of `TAX.income_band_*`'s marginal bands over the payment's per-member amount times the line's payments a year (a non-cumulative basis, since no year-to-date positions are kept), rounded once and multiplied by the members; the payer pass routes it to the treasury in the payment's own instruction (§6.5, `DayBook::levied`), and on a pooled line the losers drawn give up both parts. The consumption tax is inside the posted price: the seller owes `TAX.consumption_rate`'s share of the price paid, paid to the treasury in the sale's instruction; a country that declares a value-added tax is charged it this way (a placeholder naming TAX).

A levy's rates are read as `Rates`: a policy value's marginal bands, or one share from the line's terms or the payee's
fact. Withholding (`levy::withhold`) splits a member's gross into the net paid and the remittance, which sum to the
gross.

### 4.4 Contract algebra, lines, instructions and transfers

Every contract's terms are a composition of **generic legs** — a fixed amount; a rate on a notional (fixed or floating
over a named transacted reference, with day count and resets); an amount per unit of time; an indexed amount; an
amount contingent on a named event (a fixed sum, a **valued loss** — a named valuer's valuation bound by a limit less
a deductible — or a benefit **while a state lasts**); a delivery of units; an **elective leg**, paying only when its
named side elects on a date of its schedule (a convertible's conversion, an option's exercise), so nothing is
exercised by default — placed on dates by a calendar **schedule**, over a contract's **underlying** where it has one,
with **seniority**, **collateral description** (kind, zone, class of what secures it) and **payment order** as data.
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
title and the new loan row, all or none. A commitment may also be **drawn by a match**: a between-firm purchase whose
buyer holds the seller's terms grant becomes at 6d an instruction whose `Row` leg adds the amount to the buyer's
payable and the seller's receivable rows on the invoice line of (market, terms, statement period), in place of the
money leg, up to the grant's undrawn limit (TCR.1). An offer of held units places a commitment on them (a
`Covered<Qty>`, built only by `phx-ledger` from free units held or borrowed), released when its trade settles or its
order lapses at 1a, so no unit covers two offers (MKT.16, REG.16).

A **line transfer** moves a count of a side to a new party — a sale of loans (BNK.10, SEC.2), a client moved to
another clearing member (DRV.9), an estate succeeding a party (L3), a foreclosure (HSG.11) — as an instruction with a
reason, settled by the settlement routine with its money legs; each line kind declares which systems may request which
transfers. A **split at a kink** divides each row of a line by a per-contract amount — the insured part of a deposit
(SUP.2, SUP.5) to a receiving bank, the rest to a claim line on the estate — computed per contract and multiplied by
the count, so it is exact; it works over any row kind's per-contract amount (a benefit per year under a protection
scheme's limit, an accrued pension under a guarantee fund's cap). A **stay** is a line transfer too: at an insolvency
procedure's opening the debtor's rows move to **procedure lines** of the same kinds whose terms carry the stay, so a
stay suspends only the debtor's dues and never a line other holders share.

A `LineTransfer` moves a row's count with its balance pro rata, rounded by the transfer's declared convention.
`split_at_kink` takes a holder's rows in the declared coverage order, each row's part per member within what the
per-person limit times the holder's persons has left, bound against the member's share, times the row's count, and the
rest of the row's total beside it, so any total splits exactly.

### 4.5 Relationship rows

A holder's row on a line is found by walking its rows; an individual whose rows run long — a bank's, one per loan
and deposit line — keeps a hint of where each row it appended or inserted lies, moved as rows before it come and go
and checked against the row's head on every read, so a stale hint costs only the walk (`KindTable::row_hint`, never
saved).

A record written to a holder's lists — a row, a holding, a lot — is built as words on the stack (`words::to_words`,
at most `RECORD_WORDS`), never on the heap; a sale takes its lots first in, first out where they stand, removing the
lots taken whole and rewriting at most the one taken in part. The day's large buffers — the ledger's day book, the
accounts' events, a visit's goods view — are emptied and kept from one day to the next, so a day maps no new pages
once the heaviest day has sized them.

Every relationship of a holder to a line is a **row in the holder's chunk arena**, contiguous with the holder's other
rows: `{line u32, count u32, record u32, point u16, role u8, flags u8}` (16 bytes, no padding), plus the optional
columns its kind declares: `balance i64` for **accruing kinds** (deposits, loans, a collector's tax payable, any
running payable); `pending i64` on deposit rows for card payments and other commitments awaiting settlement (SET.2);
`amount i64` for kinds without a point table. `point` is the line's price point, copied because terms never change
(REP.3), so a row's per-contract amount is one lookup in a cache-resident point table. `record` packs days in arrears
(16 bits) and missed payments (16 bits). Each **line** keeps a **holder list** — holder keys, the holder table and
slot in one word, sorted in chunked blocks — so line-major events (a firm closes, a pairing is drawn, a bank fails)
find their holders directly. A many-party line kind whose retail side is reached only on its dues and on rare line-major
days — policies, annuities, claimants, benefits and pensions — keeps no holder list on that side: settlement's stream
gathers the day's due holders (§6.5), and a rare line-major event (a resolution, a scheme's valuation) scans the
holders' arenas once, a declared sweep. The line's side totals are kept incrementally and checked against its holders
(REP.31).

- **Holder-major** traversals — a visit, the payer pass of settlement, an outcome — read a holder's rows
  sequentially; line-major events go through the holder list. Rows grow and shrink by re-appending the holder's run
  at the arena's end and compacting in place; no row index outside the arena names a position in it. A chunk's
  arena reserves address space, committed as written: 32 MiB for a chunk of agents, 128 MiB for a chunk of
  individuals, since a firm or a bank is party to a line of each wage, rent or deposit point its households drew.
  The books compact every chunk whose dead words have passed the declared share at moments no list reference is
  held outside the tables — after each opening contribution and at each day's close — each with a transient
  scratch the size of its arena.
- **Deposits.** An agent's **banking arrangement** — which deposit kinds it holds at which bank — is an
  **attribute** (REP.41). Every deposit row of an agent counts one account. Each deposit kind declares which payments it funds and
  in which order (current before savings; term deposits never). Changing bank is a lumpy decision of the agent
  (REP.5), made when the transfer of its balances settles; the
  arrangement's one writer is `sys-bfl`, which also rewrites it, at 7e, for the holders a resolution's transfer moved.
  Coverage (SUP.2) is per legal person: for a household, limit × the arrangement's adult holders; for a firm, the
  limit once; over the depositor's balances at that bank in the declared coverage order.
- **Banknotes** are a holding (instrument: the central bank's notes in that currency, MON.1); a purchase paid in
  banknotes moves them between holding rows at 6d.
- **Loans** carry balances; an agent's row holds its loan, with its terms, vintage and payment record (REP.9).
- **Fails** become fail records with the reason and the line; the owner of the line kind reads them at its declared
  sub-step (§6.1). **Payment records** are each row's own, never combined; lenders' and suppliers' views read them.
- **Holdings** of instruments are rows in the holder's arena; each instrument keeps its holders sorted, so coupons,
  dividends, bail-ins and REG.13's audit read holders directly (REG.4). An agent's holding keeps one lot at average cost
  (REG.1, ACC.6). Which asset classes an agent holds directly —
  shares, bonds, fund units — is its **participation**, an attribute (REP.41), and income on a holding reaches the
  agent as one leg. An individual's
  holding is its lots, its basis read from them. The instrument's state (live, suspended, defaulted, ceased) has one
  writer, `phx-ledger`'s instrument events, applying the event intents the deciding systems declare.
- **Due-day runs**: a holder's rows of **dated** line kinds (loans, rents, employment, invoices, policies, annuities,
  benefits and pensions, derivatives) are one segment of its row list, and its record keeps the run's **head** — the
  earliest day any of them can be due, and the segment's offset and length — with no list reference of its own: 12 bytes
  in an individual's record (`phx_core::RunHead`, three `u32`), 8 in an agent's (`phx_pop::table::AgentRunHead`, a
  `u32` day and `u16` offset and length, since an agent's rows are few). Rows are never reordered by due day: the head is a lower
  bound, rewritten when the segment is scanned on its day (§6.5).
- **Pensions** (Stage 4, and pensions in payment from Stage 0): a job's pension kind and contribution rates are terms
  of its employment line, and the scheme a person belongs to is a component of its employment attachment (§7.1), so
  employment lines are not split by scheme. A DB row's `balance` is an accrued
  pension, a declared unit that is not money, converted to money only by the actuary's valuation and the pensioner
  line's per-time due. A DC pot is a holding row of fund units with its
  count and a `pending` word for contributions awaiting their dealing; there is no membership row.
- **Derivatives** are rows between individuals with no `amount` word: each margin account keeps the day its variation
  margin last settled, and a day's variation margin is read from the marks kept since.
- **Kin, licences and patents** (Stage 6): kin rows are appended at runtime when a child leaves or a parent
  separates, carried by their role when their holder forms a household; licences are dated rows in their holders'
  due-day runs; a patent is a holding of the patent instrument, its holder and end
  read from the holding, never from the way.
- **Keyed position lists** (Stage 6): a kind may declare a list of positions keyed by an id in the agent's arena — a
  firm's cumulative output per way it has run — each entry a total, like a deposit row's balance, so no bound is set
  on how many ways an agent runs.
- **The ledger's own records.** An instrument is a 32-byte row (issuer, issued amount, terms, unit, currency, family,
  state), its issuer absent only for a real asset (a zero identity, since no party's is zero). Units enter and leave
  holdings only through the instruments' `acquire` and `dispose`, which keep its holder list: a holder joins with its
  first lot and leaves with its last unit. A line is a 36-byte row: kind, flags, terms, its two side counts kept as rows
  change, `next_due`, `fallen` — the index of its schedule's date that fell last, advanced with `next_due` at 1b and
  handed to `due_on`, so no due searches the schedule — and its holder list. An individual's holding is 24 bytes
  (instrument, lots, quantity, flags), its lots (acquired, quantity, cost; 24 bytes each) the next `lots` entries of the
  holder's lot list, kept in holding order, since a list reference inside a list would not survive compaction. An
  individual's plant and dwellings are named units in its arena (`NamedUnit`, 24 bytes: identity, site, service day,
  class, condition). Liens are one table keyed by (holder, instrument, sequence), a re-pledge naming the lien it pledges
  on; an individual's pledged holding carries a flag, and free units are its quantity less the pledged total, a chain
  counted once, at its first lien. Commitments are few and kept in an ordered map by identity.
- **Interning and holder lists.** Terms are interned once and reference-counted (`TermsId`), in shards chosen by
  `mix64` of the terms' canonical words. Holder lists are `phx_store::BlockList`s in several `BlockPool`s, each list's
  pool chosen by `mix64` of its owning line or instrument, so parallel updates never share a pool and block ids never
  depend on workers; a holder key (`HolderKeys`) is the holder table's place in the high bits and the slot below, the
  split set by the number of holder tables.
- **Line kinds** (`LineKindDecl`): a name; per side a `SideDecl` — the holder kinds allowed, the optional words its rows
  carry, whether it keeps a holder list, the roles whose persons hold an agent's rows, whether a holder holds at most
  one row (`exclusive`), and whether a row counts a member per counterpart (`many`); the systems that may request a
  transfer; and whether it is `dated`. `due_on(terms, day, state, out)` writes a contract's dues into the caller's
  `DueBuf`, pure and allocation-free; a floating or indexed leg carries its current fixing as a term, which stands until
  its reference next fixes.


### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run. Its opening value comes from `data/`;
after that it is a fact whose one writer is the owner's decision. Each
change writes a dated announcement with its effective day, at least the next business day (VAL.6, POL.7). A change
that moves a hazard's rate books its process afresh on the agents it concerns (§7.3).

A `PolicyValue<T>` holds its opening value and its announcements in effective-day order, and is saved;
`value_on(day)` is the last effective on or before the day. `announce` refuses a writer other than the owner's
decision and an effective day before the next business day of the policy's country after the announcement.

### 4.7 Decision points, deciders and rule handles

- Every decision the spec names is a **decision point**: an input view type and an output intent type from an
  interface crate, a rule function registered by its system, a declared **schedule** (TIME.5) and **wake conditions**,
  and a pure evaluation form usable off-world (REP.15). The **decider** — the player, an office's holder, an
  institution's founding preferences, a household or a person — is named each time the decision is taken by the
  decision core (§7.18), never stored per decision.

  A decision point's rule is one pure function and is its own evaluation form, so REP.15's estimates evaluate the
  function that decides. A point needs a schedule or wakes. `Ctx::decide` takes the player's queued intent for the
  point when one is queued; otherwise the rule decides only if the player's decider delegates, and else the decision
  is not taken that day. `RuleTable::check` refuses a rule signature with no implementer or two, one implemented by
  another system than its declared one, and an implementation of no declared signature.
- A **rule handle** is a pure function declared in an interface crate and implemented by its owning system — the tax
  on a given income, a benefit entitlement, a lender's cap, a clearing house's margin for a trade, a platform's effect
  on a household — callable by any system with inputs it may read. It reads only its inputs and policy values. Market
  **admission hooks** (DRV.6) are rule handles, taking a member's whole order set at a meeting (§8).

### 4.8 Keyed reductions

`KeyedReduce<K, V>`: chunk-local sorted runs, merged in parallel by owner-key shard, each shard in chunk order. The
only way a pass sums over rows it does not own, and the way every **global structure** — the terms interner, the
directory, line creation, holder lists — is updated: sharded by key hash, each shard applied
by one worker, so nothing serialises and no atomic is needed. As built at Stage 0, settlement's sums are keyed
reductions of this kind: each shard of payments puts what it adds with the shard of the record, net or party it adds
to, and each shard of the day's records or of the kernel map is folded by one worker, taking the payment shards in
order (§6.6). The other global structures are still updated in place on one thread.

`KeyedReduce` stable-partitions each chunk's pairs by `mix64(key)` (the SplitMix64 finaliser, never `foldhash`, so no
crate version decides a shard) into 256 shards; each shard concatenates its partitions in chunk order, radix-sorts
by key and folds equal keys in (chunk, position) order, so a non-commutative fold gives one answer. `KernelMap`, the
kernel's one hash map (hashbrown with fixed-seed foldhash), is sharded by the top eight bits of the same mix; it is
read whole only sorted by key (`sorted`, `drain_sorted` for saves) or, for an order-free read such as a sum, unordered
(`each`), and `fold` fills it from many sources on the pool, each shard taking the sources in order.

### 4.9 Records, audiences, scoped reads

Decision kernels read what their party may: its own rows and facts; the relationship rows it is holder or counterparty
of; records whose audience includes it; earlier prints and marks. As built, a handler's `Ctx` reads and writes a
declared fact by slot (`read::<F>(slot)`, `write::<F>(slot, v)`), and has no per-party accessor. Audience is checked at
compile and assembly time from declarations; `read-trace`, a run-time flag of release builds, on in every build run
(§14.7), samples chunks and verifies reads at run time. Store-wide views go only to processes and applies.

The sample is the first chunk of each (handler, table) in the run and every chunk whose index is congruent to the day
modulo `TRACE_PERIOD` (64). A traced chunk's writes are stamped with their (sub-step, handler), and each read is checked
against the reader's declaration and against a later stamp; every `Streams::open` is recorded unsampled, and at the
close the day's opens are sorted and each repeated (stream, subject, sub-step) counted (`TraceLog`). What the trace
finds is reported through the audit's Time family (§15); the stamps live for a day and the log is kept outside the
world hash.

A record kind declares its audience, its horizon and its writer; entries are dated by (day, sub-step) and are world
state, hashed and saved. `RecordStore::read(kind, reader, calendar)` returns only entries whose audience includes the
reader and that are dated before its own sub-step; a `PublicAfter(lag)` entry becomes visible to others on the day
the calendar places at its date plus the lag. An event carries its kind, day and sub-step, its subjects and details
(a subject with a size in the kind's declared unit — a catastrophe's struck tiles and severities) and may develop only
from an event written before it.

### 4.10 Public events

Every event is recorded private, at the apply of the sub-step that drew it; whether it is public is never its
writer's to say. The OBS.3 rule (`phx_core::EventsRule`, from the standing SHAPE `OBS.public_events`) runs at 10a and
marks public each event dated that day or the day before that it judges so: it names every declared event kind once
— always, never, or when one of its details' sizes reaches a declared size in the kind's unit — and reads each event
alone, so an event judged twice is judged the same way, and yesterday's events recorded after its 10a are judged
today. A public event stays public. `phx-obs` only shows them.

---

## 5. The kernel–system boundary

### 5.1 `System`

```rust
pub trait System: Send + Sync + 'static {        // zero-sized implementors only (asserted)
    const CODE: &'static str;                     // checked at assembly as two to four capitals
    fn declare(d: &mut Declarations);             // kinds, facts, stores, lines, messages, levies, markets, rules,
                                                  // primitives, policy values, hazards, occasions, kinks, decision
                                                  // points, schedules, audits, metrics, views, opening contributions
    fn handlers(h: &mut HandlerTable);            // kernels per sub-step, with reads and writes
}
```

The trait and everything it declares with live in `phx-core`, so systems (L2) and kernel crates (L1) use them without
reaching the assembly. Declarations generate typed handles with private constructors: using something undeclared does
not compile, and an unused declaration fails `dead_code`. A reference across systems — a writer, a reader, an answering
system, a transfer requester — is a system's code as a string, not a compile-time handle; assembly checks the
references §5.4 names.
Registration is one line in `phx-world/src/systems.rs` (each interface crate's items are listed beside it); its order
carries no meaning (§6.2).

### 5.2 Handlers

A handler is a kernel over chunks of one table — its agenda rows, or every row where the sub-step declares a sweep —
declared with its sub-step, the facts and columns it reads and writes, the messages and intents it emits, its
streams, and its body, a function of the context and a row. The declaration generates the handler's run over a chunk:
its context built from the day's parts, then the body for each row. Its context grants exactly what it declares; it
also reads the register, through the handles its system keeps, and the **own state** its system compiled at assembly
(its handles and derived tables, such as `phx-geo`'s map), which only that system's handlers are given. A system
declares what it compiles with `Declarations::compile`, a function of the register and the number of countries run at
assembly and again at a load, so nothing it holds is saved (`sys-tec`'s products, ways and public sets); its family
reads the same state. A system that compiles nothing is given the unit. An event a handler draws is an intent (`EventIntent`), naming its kind by the
place the system resolved at assembly, and is recorded at the apply point, dated by the sub-step that drew it.

### 5.3 Schema compilation

Declarations compile into: fact and facet columns; the record of each agent kind and its persons' layout (§7.1); kink
sets; relationship-row layouts per line kind; point tables; message, instrument, market and levy profiles; the stream
registry; the primitive register checked against `data/` (NUM.3); schedules and wake conditions; the handler graph;
audits and metrics. An attribute or count field whose value outgrows its width is a **contract violation** calling for a
layout change; nothing saturates (Law 6). A policy schedule's count of bands is declared with it (the constitution's,
POL) and fixed here: a platform or a budget moves its values and edges, never the count, so the kink signature's
compiled width holds for the run.

**The register.** A primitive is a `const` `PrimDecl` written with `declare_prim!` — id, kind, unit, period,
`decided_by` (present exactly for POLICY), value type, clause, SHAPE information (`Placeholder { retired_by }` or
`Standing { reason }`) and scope (shared or per country); its owner is the system its id begins with. Each
`[[primitive]]` entry of `data/` is checked against its declaration (id, kind, unit, value type, source, `source_ref`,
a SHAPE's field); an undeclared entry, or a country missing an entry, stops assembly with the list. Values are
immutable after assembly and read through the typed handle `Prim<T>` (`get(&Register, country)` or `shared`), one
indexed read. Decimals in data are exact — an integer, a string, or a float read by its shortest decimal form — and
more places than declared are refused, never rounded. Value types are scalars (`Fixed`, `Rate`, `Money`, `Qty`,
`Count`, `Date`), one- and two-axis tables with a declared rule outside their axes, distributions (normal,
log-normal, Pareto, log-normal with a Pareto tail, gamma, beta, Weibull, discrete, empirical), point tables, a
country's calendar rules (`TIME.calendar`) and its legal forms. A **type set** is never declared: assembly cuts the
declared distribution into the kind's RESOLUTION count of types by the entry's declared discretisation
(`equal_shares`: equal shares of 10⁶ ppm, the remainder one part to each of the first, each type at its share's
middle quantile), so a new count re-cuts the same distribution (NUM.4); gamma and beta quantiles invert their
regularised incomplete functions by bisection. `DeclaredLimit::bind` returns a `#[must_use]` `Bound` whose `taken`
is read only through `Ctx::bound`, which writes a binding record, audience the bound party, whenever the excess is
positive (Law 6). `Register::placeholders()` and `standing_shapes()` feed NUM.7's report.

**Streams.** Each stream is declared with one `Purpose`, a closed list mirroring CHN.3's (mortality, illness,
birthday, conception, accident, damage, third-party harm, catastrophe, equipment failure, discovery, meeting, weather,
type at birth, schedule phase, occasion, taste, pairing, sample, lot, the opening, the observer); none is an outcome
(CHN.5). Assembly refuses two streams of one name and two names whose FNV-1a collide. `Streams::open(stream,
subject, day, ordinal)` is the one constructor of `Draws` beyond `phx-rand`: handlers reach it through `Ctx::draws`
with their day and sub-step, the map's generation and the opening through contexts that carry opening ordinals beyond
the day's sub-steps, and the observer through `ObserverDraws`, which opens only `Observer` streams, as no other
context may (Law 17). A **keyed** stream draws from (stream, subject) alone at an ordinal of its own (`open_keyed`), a
pure function of identity recomputed whenever read, such as a schedule's phase; it is exempt from the duplicate-open
check. PC-19 holds these call sites.

### 5.4 Assembly refusals

A fact with no writer or two; a message kind with an unanswered addressee kind or no answering sub-step; two handlers
in one sub-step where one writes what the other reads or writes; a levy without a schedule owner, or a follow-on not
written by its payee system; a kink on an undeclared position; a primitive without value, unit, kind or source; a
decision point without an evaluation form or schedule; a rule signature without an implementer; a line kind without
declared transfer requesters; a commitment kind without the legs it creates; a hazard without a draw scheme (REP.7);
an interface item whose writer is not registered; a system handler at a kernel apply (§6.2); a handler with no body;
a handler on a table the world does not keep.

  For a population kind, every refusal is reported at once: a name declared twice, an attribute or person attribute
  of no values, roles or person attributes beyond a person's word, a kind sited by two attributes or by one it does
  not hold.

---

## 6. The day

### 6.1 Turns, stages and sub-steps

A **turn** advances the world to the next day that is a business day in any country, running each day in between as
its own day (N8.2). On each day, each country's business-day calendar decides which of its markets and institutions
act (TIME.2). A sub-step marked **B** runs only for countries whose business day it is; everything else runs every
day, as TIME.8 lists.

| Stage | Sub-steps |
| --- | --- |
| 1 Open | 1a lapse orders, quotes and day-local messages whose day has passed · 1b the lines due today marked (§6.5) · 1c the player's queued intents become wakes (§12) |
| 2 Resolve | 2a accruals post where a date needs them · 2b (B) **resolutions** opened by the last business day's failures: valuation and write-downs, a clearing house's recovery by its rulebook on its pending legs (§9.2) · 2c (B) **settle** calls and demands due today (the settlement routine of stage 7); failures handed to their owners at once (TIME.7) · 2d (B) earlier fails delivered to owners; arrears · 2e (B) recognised losses land; parties that cannot go on end; estates open and compute their waterfall · 2f apply |
| 3 Nature and population | 3a weather; catastrophes (§7.8) · 3b the **agenda** gathered, and hazards and occasions drawn for its agents (§7.3) · 3c demographic events; foundings and moves decided before; each country's school year's date; households formed (`combine`) and divided (`divide`) on the day their dwelling is their own · 3d overlapping occasions allocated · 3e hits answered, then stage 3's apply (§6.2): transformation records; hit records reach the systems a process declares interested, which open what they answer (claims); discoveries, imitations and meetings applied |
| 4 Real work | 4a production, services, shipments, construction; jobs starting and ending · 4b apply |
| 5 Decide | 5a public-series outlooks per method, and registered instrument outlooks and their values on days with a new print (§8); migration's memo of inclusive values (`sys-hh`) · 5b continuous decisions of rows scheduled or woken today, fused per table · 5c lumpy decisions of occasion holders; institutions (B); answers to messages at their declared sub-step · 5d apply |
| 6 Form prices | 6a meetings: retail, services and electricity every day; all others (B), open-ended funds' dealing at the value computed after its orders were taken (forward pricing, administered, MKT.8) among them; a resolution's selection among bids by the authority's least-cost rule (B, §9.2); admission hooks over each member's order set (§8) · 6b marks, and instruments' and currency pairs' fixings by the pricing service's declared method (§8) · 6c (B) the curve and the valuation inputs derived from 6b's fixings (discount-factor tables) · 6d apply: matches become instructions, drawing the commitments they meet (a terms grant's `Row` leg in place of the money leg, §4.4); banknotes change hands; on non-business days card payments and electricity trades are recorded as **pending** on the payer's deposit row and the payee's, settling at the next business day's stage 7 |
| 7 Settle (B) | 7a **payer pass**: per holder, its run head, and on the head's day its dated rows, each payer's legs in declared order checked against its funds, per currency; a bank's conversion commitment drawn for a leg in a currency its payer does not hold; holding levies on their dates; per-category tallies of rows on lines whose sides sit in two countries; the day's due holders of lines with no retail holder list gathered; per-bank nets by keyed reduction · 7b **fixed point**: the greatest set of payments that can settle given one another, with banks' nets and intraday credit, by a fail-only worklist (§6.5) · 7c the surviving payments' nets applied per account, one instruction per line, deposits and reserves together, with the streaming audit fused in and the survivors' declared tallies added · 7d fails recorded; payees of failed payers drawn (REP.23) · 7e levy follow-ons written (§4.3); banking arrangements that a settled resolution transfer moved rewritten by their one writer (§4.5, §9.2). As built at Stage 0, all of it — the stream, the fixed point, the apply, the losers' draw and the estates' settlement (§9.1) — runs in 7c's kernel block; 7a, 7b, 7d and 7e run nothing of their own |
| 8 Fund (B) | 8a money-market orders and the central bank's tender orders · 8b the **linked call**: the money market and the tenders meet together (§8) · 8c its trades settle · 8d standing-facility, lender-of-last-resort and the treasury's direct-borrowing requests, met by `phx-market`'s administered form within their declared limits, reading the supervisor's solvency fact · 8e they settle · 8f intraday credit closes: a bank that cannot repay has the shortfall recorded as an **overdue claim of the central bank** and its liquidity failure recorded (MON.3, MON.12, BFL.10) |
| 9 Value and judge (B) | 9a valuations, provisions among them, and each party's sensitivities per (party, bucket) · 9b accounts and ratios, funds' net asset values and the group fact, reading 9a's valuations · 9c tests: margins (a house's initial margin one blocked product over its accounts), covenants, capital, solvency — each test and the consequence it triggers in one handler, consolidated statements a pure `phx-acct` read; demands issued, due next business day; resolutions triggered and bids invited (§9.2); reports read from the books · 9d publications: reports, ratings, net asset values, benchmark reference rates fixed from 8b's match sets (§8), analysts' estimates revised on the day's reports · 9e apply: stage 9's intents — demands, messages, wakes, facts on other parties, the closed fact, income events |
| 10 Close | 10a public events; an election's tally where one closes today · 10b declared sweeps a system registers and `phx-pop` runs (a campaign's intentions written and cleared) · 10c (B) nothing at Stage 0 · 10d on a day with no 9b, the day's accounts posted first (§3.3); incremental audit families · 10e views and pages on a turn's last day (read-only) · 10f metrics |

The **kernel applies** — 2f, 4b, 5d, 6d, 7c, 9e and 10b — are a sub-step kind of their own: no system registers a
handler there (§5.4), and the kernel runs each on every day its stage runs, whatever is queued. What a system does
there is declared — an intent the apply executes (a policy value, a currency trade's legs, an attachment moved in
place), a tally it feeds, a sweep `phx-pop` runs.

On a non-business day, 5b and 5c run only the decision points that TIME.8 lists, other occasions waiting for the first
day their decision is taken, and payments that stages 3 and 4 give rise to (a founding's capital, severance) are
recorded as pending, settling at the next business day's stage 7. Money moves only at 2c, 6d (banknotes), 7 and 8;
estate distributions, resolution transfers and line transfers are instructions settled by that routine.

Assembly runs every system's `declare`, then every `handlers`, then compilation, and reports every refusal at once, by
item and system. `World::run_turn` queues the player's intents for 1c and runs each day from the day after the last turn
up to `Calendar::next_turn_day`, keeping a `TurnRecord` whose wall time comes from the application's `Clock`
(`phx-exec`) and reaches the metrics, never the world. `run_day` walks `SUB_STEPS` in order: a sub-step runs when it has
handlers, when it is a kernel apply of a stage that runs, when it holds the kernel's own work, or when it is the
audit's (10d), and a business-only one only when some country has a business day; each that runs leaves a
`SubStepRecord`. A country's business days are its rules, which are the calendar's state, and a bitset built from them
over the years from the epoch's to `CALENDAR_WINDOW_YEARS` past the current one, rebuilt each 1 January; outside the
window the rules answer.

**The calendar** (`phx-core::calendar`) is built at assembly from each country's declared rules: a weekend and
holidays that are `Fixed`, `NthWeekday` (−1 the last), `EasterOffset` (the Gregorian computus) or `Substitute`
(moved to the next day neither weekend nor holiday, in declared rule order). Business days are cached as a bitset per
country over a window of 64 years from the current year, extended at each year's start and derived, outside the
world hash; days beyond it are computed from the rules. `Calendar::plus(day, period)` is the only way to add a
period to a day (PC-17): a `Period` is months or days, never both, and the n-th date of a schedule is advanced from
its anchor, never from the previous date, cutting the day to the target month's length (`EndOfMonth::Keep` also
keeps a month-end at the month-end). Business-day conventions are ISDA's (`Following`, `ModifiedFollowing`,
`Preceding`, `ModifiedPreceding`, `Unadjusted`); day counts (`Act360`, `Act365F`, `ActActIsda`, `Thirty360Bond`,
`Thirty360E`) give an exact `DayFraction` per year, Act/Act ISDA over the common denominator 365·366. A decision
schedule is a period, a convention and business or any days; its instances are anchored at the epoch, and a party's
due day is its phase — a keyed draw within the period — into the next instance strictly after, moved by the
convention.

### 6.2 Order without order dependence

Within a sub-step every handler reads the state as it was at the sub-step's start. A handler writes directly only its
own rows' columns that no other handler of the sub-step reads or writes, and later sub-steps see those writes; every
other effect is an **intent**, gathered at the end of its sub-step and applied by the one apply routine (§6.4) at the
first **apply point** at or after it: its stage's kernel apply (2f, 4b, 5d, 6d, 7c, 9e, 10b), the end of 3e for stage
3, and otherwise — stages 1 and 8, and 7d, 7e and 10c to 10f after their stage's apply — the end of its own sub-step.
The settlement routine applies what it settles where it runs (2c, 7c, 8c, 8e). Up to an apply point, later sub-steps
therefore read earlier ones' direct writes and never their intents. The handler graph, built from declared reads and
writes at the granularity of (table, column or line kind), refuses two direct writers of one resource and a direct
write another handler reads, so registration order carries no meaning; intent buffers are keyed (chunk, canonical
handler id), so gathers and new identities do not depend on it either, which a logic-level test holds: the canonical
ids a registration list yields are the same for every order of it (§14.3).

A handler's canonical id (`HandlerId`) is its place in the order of (system code, handler name), which `HandlerGraph`
keeps its handlers in.

As built at Stage 0 no system's handler writes an instruction, so the world's apply routine (`phx_world::day`'s
`apply`) records the day's event intents, dated by the sub-step that drew them, and stops the run on any other intent.
What moves the books is the kernel's own work at its sub-step: 3b's catastrophe losses (§9.1); 3e's outcomes on the
agents, the ledger's `members_leave` and the estates' line transfers (§7.4); and 7c's block, which runs
`Books::settle_day` and then the estates' settlement (§6.5, §9.1). Instructions go through the ledger's one
instruction apply (`Ledger::apply`), which feeds the audit.

### 6.3 Traversals

`phx-exec` runs one traversal per table per sub-step, in **cost-sized chunks** (declared per table, never dependent on
the thread count), running every handler of the sub-step on a chunk while it is in cache. Each sub-step declares whether
it is an **agenda** pass (the rows the agenda lists, in slot order), a **stream** over the rows a batch reads (the payer
pass of §6.5, reading only the columns it needs), **index-driven**, a **full sweep** — only where a clause needs every
row that day, and always declared: a surprise's wake pass (§7.3), the singles' counts for meetings, a scheme's valuation
and the sweeps systems register (§6.1) — or a **kernel apply** (§6.2). A **sweep ledger** counts rows and bytes touched
per sub-step, and a ratchet holds it (§16). As built at Stage 0 the ledger is partial: each sub-step's `SubStepRecord`
keeps the rows its handlers visited and its wall time, its bytes and barriers are 0, the kernel's own work at a sub-step
(hazard draws, outcomes, settlement) is not counted in it, and no ratchet reads it.

The kernel's own small tables (regions, countries) keep their facts as columns (`FactColumns`) and are traversed chunk
by chunk on the day's thread, since each is one chunk. The pool's traversal arrives with the first handler on a kind
table, whose chunks are handed to workers as disjoint column views; the order of chunks and handlers, and
so of intents, is the same either way. The kernel's own passes over agents — 3b's hazard draws, 3e's outcomes, 7a's
and 7c's gathers and the audit — run on the pool in shards whose results are joined in slot order; each shard holds
its scratch, the household it reads each agent into and the chances it fills, and reuses it for every agent it draws,
so a pass allocates for its largest household rather than for each agent.

`phx-exec`'s primitives give the same result whatever the worker count. Chunk boundaries depend only on the table's
size, or on the agenda's rows and their declared cost: an agenda unit closes only at a table-chunk boundary once its
cost reaches `CHUNK_COST`, so arena writes stay chunk-local. Workers take chunks as they free, but each result is
stored at its chunk's index; gathers concatenate in (chunk, canonical handler) order; `reduce_tree` folds a
left-balanced pairwise tree whose shape depends only on the number of results. Radix sorts are stable LSD over
11-bit digits, skipping a digit every key shares, with a stable comparison sort below 2 048 pairs. `gather`,
`KeyedReduce` and `radix_sort` take the pool as an option and give the same result without one. The pool pins each
worker to a core whose `cpu_capacity` is at least half the largest (all allowed cores when capacities cannot be
read; a pin refused leaves the worker unpinned and counted); idle workers spin a bounded number of rounds before
parking, so back-to-back sub-steps pay no wake-up. Only `pool.rs` and `site.rs` read a worker's index; atomics and
threads exist only inside `phx-exec` and never appear in its API. Wall time comes from an injected `Clock` and
reaches counters and the application only (TIME.11).

### 6.4 Compute, gather, apply

Handlers write their own rows and emit intents into (chunk, handler) buffers; gathers place them by prefix sum; applies
run in declared order, in parallel over disjoint targets. **One apply routine** serves every apply point (§6.2): it
settles the instructions of reasons allowed there (§6.1), writes transformation records, and checks and feeds the
streaming audit through `phx-core`'s `AuditStream`, which `phx-world` injects (§3.3). Every reduction runs over a fixed
tree. At Stage 0 it applies event intents only; outcomes, settlement and estates are the kernel's own work (§6.2).

An `Instruction` carries its identity — its day in the high word and its place in that day's gather order below — its
reason, trade and settlement days, its legs, the contract row whose due it pays, if any, and the covers its trade's
offer placed. A leg (`LegRec`) names its party, its account (a line side's row, an instrument, or a named unit), a
signed quantity (received positive, given negative), its denomination (a currency, which is its issuing country's
index, or a unit) and its kind: `Money`, `Units { cost }`, `Row` (open, close, adjust, count), `Transformation { source,
cost }` or `OpeningWrite { identity, cost }`, the last two the only legs with no paired side. A money line's two sides
carry one balance, the holder's owed to it and the issuer's owed by it, so each line's balances sum to nothing; a
negative balance is allowed only on a deposit row whose terms carry a `Facility` (limit per member, rate, day count),
and that balance is the loan. `Ledger::apply` reads every leg's position before anything moves, checks them with the
pure `check_legs` — each position at or above its floor on the sum of the legs moving it, an issuer's liability having
none — and applies all or none, a refusal recorded as a `Fail` with its cause and party; it keeps the reason's
accounting effects, taken on each party's net in the instruction, releases the instruction's covers whether it settles
or fails, and feeds the audit each settled leg with what its account held before, which `phx-audit`'s `Digests` keep
apart from the books. Instructions of one sub-step apply by their reasons' declared payment order, then identity; an
identity applied twice, an instruction with one side, or money moved where money does not move stops the run. A
rounded split lands its residue on the payer, the payee with the largest share, or a named party (`RoundingLanding`).

### 6.5 Settlement

- **Implicit batches** — (line kind or market, rule, day) — are never materialised as batches: the one day buffer
  sized by the day's payments is the list 7a hands 7c, so 7c does not reckon them again (as built below). Debits are
  generated payer-major from holder-major rows (point lookups, levies per contract), credits payee-major by keyed
  reduction streamed shard by shard; the two sides agree by REP.31. Each party's rows in a batch are summed
  sequentially into **one leg per (party, bank)**, debited from its deposits in the declared payment order and credited
  to the deposit its kind declares receives that reason; a standing flow is one leg per paying row per day; pending amounts
  from non-business days are legs of the next business day's batch.
- **1b** marks the lines whose dues fall today in a **due-line bitmap** (one bit per line, cache-resident); each line
  carries its next due day, which 1b advances past today as it marks the line, so 7a reads advanced dates.
- **Due-day runs**: every line kind with dues is dated, and a holder's dated rows are one segment of its row list
  behind its **run head** (§4.5). 7a reads each holder's head; on a day before it the holder costs that one read; on
  its day 7a scans the segment and rewrites the head as the least next due day of the segment's lines. A row joining
  the segment lowers the head if earlier, to today when its line fell due at today's 1b (whose date has moved on), so
  rows joining at 3e pay today's due; a row leaving changes nothing, since an early head costs only a scan.
- **7a** is **one stream over the holders' runs**, holder-major: in a scanned segment, a row whose line is due today
  is a debit if the holder is on its paying side and a credit otherwise, at the row's per-contract amount (a point
  lookup, levies per contract) times its count; each (party, bank) gets its debit and credit totals, tested for debits
  in its payment order, the first it cannot meet failing with every one after it, with the first failing row recorded. Banks' nets are sums over parties. Nothing is
  written per leg and no payee reduction is needed. For a due line with no retail holder list, the (holder, row) pairs
  met are gathered into the day buffers, which 7b and 7d read in its place. As built at Stage 0: the stream keeps a
  record per party, and the day's lookups (each line's reckoning and ower, each party's account), in the kernel's
  fixed-seed map, read whole only in key order, and the claimants per line in `BTreeMap`s; and 7b reads unlisted sides
  from the day's payments the stream keeps. The stream keeps the day's payments in its order (about 157 MB on a
  payday of 2.8 M payments, the only field PC-27 lets keep a batch's items), and 7c's gather reads them, reckoning again only a cleared line's claimant credit
  after its losers; both compute each payment's route, 7c nets it in the fixed-seed map and sorts the nets once, and
  a due record is kept per payment for the accounts.
- **Reckoning**: a due line's dues are reckoned on one side's rows, each row its own payment with one counterparty.
  A line of two holders is reckoned on its claimant's row; a line one party holds a side of (a bank's loans to many
  firms, a scheme's members) is reckoned on the other side's rows, each paying or paid by that party. A row's due is
  its terms' per-contract amounts times its count, and what its terms reckon on its balance, which is the row's total.
  The liability side always pays. A holder's record is per currency, one account each; the payment's route runs from
  the payer's account to the payee's through their banks' liabilities and, between banks, reserves.
- **Cleared lines**: a line with many holders on both sides — employers and employees, landlords and tenants — records
  no pairing (REP.23), so it is **cleared**: each row on either side is its own payment against the line, the
  liability side's rows paying their dues and the asset side's rows paid theirs, the two totals equal since the
  sides' counts are (REP.31). Each payment's money is routed from the payer's account up through its issuers to the
  top one, the central bank, and down to the payee's, so the banks' nets settle in reserves and the top issuer's
  legs sum to nothing over the line. A cleared line's dues are per contract (a wage, a rent), refused otherwise. When
  its payers fail, the claimant contracts that lose are drawn (REP.23): the contracts the failed rows owed, as one draw
  over the claimant rows' counts from the line's own stream on the day, made afresh from the same start each time the
  failed count grows, so the fixed point stays a function of the failures alone; each claimant row is paid for its
  contracts not drawn. The draw is one sequence of contracts, each picked by the contracts its row has left (a Fenwick
  tree over the claimant rows, `phx_ledger::cleared::Losers`), from the stream `REP.cleared` keyed by the line and the
  day, so the first n drawn are the same contracts whatever n the failures reach and a growing count only takes credits
  away. A line is cleared when its holder list shows at least two holders on each side, a side that keeps no list
  (households, depositors) counting as many; the reading stops there, and two listed holders are the whole line only
  when both its sides are listed. The claimant rows the losers are drawn from are those the day's stream read, since
  an unlisted side's holders are found only by reading their rows. For
  the accounts the top issuer stands as every cleared payment's counterparty: it is owed the failed payers' dues and
  owes the drawn contracts theirs, recorded as failed dues, so its receivables and payables on the line net to what
  the contracts drawn past the failed count were owed, which it holds and owes them (REP.23;
  `DaySettlement::lost` counts the contracts drawn, `lost_past_failed` those past the failed count). A short bank's customers lose their payments through it, but a
  cleared line's credits into it stand, since they only add to its reserves. A cleared payment through a closed bank
  waits for the resolution `sys-sup` brings (S2.08), and stops the run until then.
- **Banknotes** (MON.4): a central bank's notes are its liability on a cash line of its country, `banknotes`, held by
  households and estates, one row each and no dues; a household that banks nowhere holds its persons' share of the
  currency in circulation a head (`CB.currency`, per cent of GDP) there from the opening, and pays and is paid in them
  through the one money route, its notes its money account, the central bank the top issuer it reaches. The central
  bank's treasury account is what its claim leaves after its reserves and its notes. A banked household's cash, its
  liquidity choice, arrives with HH.7 (S2.05).
- **A party with no money**: a payment whose payer or payee holds no money in its currency — no account, no notes and
  none it issues (`Books::holds_money`) — has no
  legs and fails at the start of 7b, with the cause `FailCause::NoMoney` (MON.12). A cleared line's payer with none
  fails as a short one does, its dues lost by drawn claimant contracts; a cleared line's claimant with none
  loses its due against the top issuer, which holds no row to be in arrears on, and the top is the one the line's
  holders with money reach.
- **7b** starts from every payment succeeding and removes, until nothing changes, the payers who cannot pay given the
  payments still standing; then every bank still short once its payers are done, which cannot cover its net after
  intraday credit (MON.3, MON.5), loses its customers' payments at once, each failed with the bank's cause,
  `FailCause::BankShort`, and the payers answer again, round by round until no bank is short. A payer's own payments
  and a bank's customers are read from the day's payments 7a found (`DayRecords::made`), indexed by payer in payment
  order, not from the holders its lines list, so a side that keeps no list — household depositors, a wage line's
  employees — is reached. A removal lowers the credits of the payment's other side and the nets of its banks, and takes
  their parties again. A payer fails as a **prefix** of its payment order (REP.18), which is monotone, so the result is
  the **greatest** set that can settle given the banks' removals, and rings of payments that can settle together do
  (TIME.6).
- **7c** nets the surviving payments' legs per (line, party, side), money and rows apart, and applies them as one
  instruction per line in line order, read straight from the ordered nets: every account is checked once against its
  net, so no order of application can fail what 7b let stand, and banks' reserves move once per bank by net; applied
  is final (SET.5). Each line's rows are its own, so a line whose every leg moves a balance its read found, one leg
  to a row, is checked and recorded in order while its balance writes are held back; at the end of the apply the held
  writes land a holder chunk to a worker, a word written twice stopping the run, and what each party moved folds a
  key shard to a worker. A line that opens, closes or twice moves a row writes what is held first and applies in
  place. Failure is
  per payer (MON.5): a payer that cannot pay fails its own legs; where the pairing to its payees was not recorded, the
  payees who lose are drawn (REP.23). Tax withheld from a due goes to the tax's payee: it crosses from the payer's bank
  to the tax payee's, and it is the due's payee's expense and the tax payee's income, kept in the day book apart from
  what was earned (`DayBook::levied`), which the national accounts count gross of it.
- **Verdicts**: 7c recomputes, over the payments that settle and the books as stage 7 found them, each party's
  standing (soundness), each failed payer's shortfall at its first failed payment (maximality), each account's
  movement against its net, and each bank's reserves against the net of its customers' payments across banks; one
  holder in 64, by slot congruent to the day, has its run read against all its rows. The counts of breaches are the
  day's, read by the live checks. The **closing ring** (SET.10) is the parties whose settled payments their own funds
  could not cover without the day's credits, and the part those credits paid.
- **Fails and the close**: the day's fails, each with its cause, are the day's book, which the close takes with the
  settlement measure (SET.10) and the dues' counts. A fail waits for the next business day's 2d, where the contract
  process — kernel work at a sub-step with no handlers — reads it against the party that holds its row then and turns
  it into arrears; the directory keeps the failed party resolvable until
  then. Once that 2d has run, the world counts each earlier day's fails whose row is still held and not in arrears
  (`Settled::unrecorded`), which LC-0-21 reads.
- **Pending** is a leg's third state, beside settled and failed. At 7a a leg whose payer's or payee's bank is
  **closed** (§9.2) is fixed as pending and kept out of 7b: it neither fails nor funds anyone. Its amount sits as
  `pending` on the payer's deposit row, which the payer's funds exclude, and on the payee's, where it counts for
  nothing, until it settles at the first stage 7 at which neither bank is closed or fails against a claim line on the
  estate (MON.5). A closed bank's reserve account passes to its estate with everything its transfer leaves.
- **A closed payer**: every leg a closed insurer or clearing house owes is pending the same way, and legs owed to it
  settle as before. On a many-party line whose paying side holds a closed party among others, the holders whose
  credits are its own are drawn once for the whole resolution (REP.23), and the same holders stay paired until it
  settles; the other payers pay theirs as before.
- **Spent rows**: when a holder's segment is scanned, the rows of lines whose schedules are spent leave the segment for
  the end of the holder's rows, so an empty segment is never due.
- **Standing flows** — a rate per member per day on a row, plain or times a region's daily index — arrive with the
  stage that first declares one; nothing is built for them yet.

- **Currencies** (Stage 5): the payer pass tests each payer per (party, bank, currency), and banks' nets are per
  (bank, currency). A leg in a currency its payer does not hold draws its bank's **conversion commitment** at 7a: the
  bank is the counterparty of both legs, trading from its own currency book at its posted quote, so the payment
  settles as a trade or fails whole; a capital rule's handle gates the commitment. A leg in a currency whose system
  has no stage 7 that day is pending, as above, until a business day of both.
- **Tallies**: a reason may declare a per-category tally (the balance of payments). 7a keeps, per payer, a vector of
  per-category amounts for its rows on lines whose sides sit in two countries, and 7c adds the surviving payers'
  vectors into a keyed reduction (§4.8); an instruction settled outside the batches feeds the same tallies at its
  apply. No handler tags a leg.
- Physical transformations are checked against transformation records (SET.9).


### 6.6 Cost bounds

**The rule** (the owner's, plan §12): every operation a day performs costs at most **O(log n)** in the size of any
store of the world — parties, holders, rows, lines, agents, persons, instruments, records, events, history — and a
day's cost is the sum of its events' costs: the rows due, payments, hits, outcomes, orders and visits it has, each at
most O(log n) (N8.6). A pass over a world-sized store is allowed only as a declared **rolling slice**, a fixed share a
day (the audit's thirtieth), counted in §13.2. Nothing a day does grows with elapsed time. A kernel that cannot meet
the rule is a finding, not an exception.

Where the day breaks, the representation that meets it, each part with the
published technique it rests on:

- **Dense identity.** A party id indexes a dense array of its location (ids are issued in order, and the ended keep a
  bounded window of tombstones), so `locate` is one read, with no hashing. Hashed maps remain only where keys are
  sparse and few.
- **Standing facts on their owners.** What does not change from day to day is kept where it belongs, updated when it
  changes. A holder keeps its money account per currency and that account's issuer, written when an account opens or
  closes. A line keeps how it is reckoned and who owes it, written when a listed side's membership changes (its side
  version). The day's look-up maps (`Found`) go.
- **Row index.** A holder's rows are ordered by (line, side), and one row is found by search: a branchless scan of a
  cache line of keys for a small holder, and for a large one (a bank, a treasury) an Eytzinger-ordered key array
  (Khuong and Morin, *Array layouts for comparison-based searching*, 2017). The cost is O(log R). A find returns the row's
  word offset, so a write does not search again.
- **Due index.** Holders sit in the agenda's hierarchical timing wheel keyed by their run head's next due day
  (Varghese and Lauck, *Hashed and hierarchical timing wheels*, 1987). 7a visits only the holders due today, O(1)
  each, and a rewritten head re-files its holder.
- **The day's payment table.** 7a reckons each due row once into a structure of arrays: payer, payee, amount,
  principal, line, order and route. A route depends only on the payer's and payee's accounts and their issuers, so it
  is interned once and payments carry its index, which keeps legs off the heap. Payments are indexed by payer and by
  each bank on their route in compressed sparse rows, built by a counting sort (a semisort; Gu, Shun, Sun and
  Blelloch, *A top-down parallel semisort*, 2015). 7b, 7c, the verdicts and the pending pass read the table, and none
  reckons a payment again.
- **The fixed point** is the greatest set of payments that can settle, found by removal. It descends from Eisenberg
  and Noe's fictitious-default algorithm (*Systemic risk in financial systems*, 2001; Rogers and Veraart, 2013, for
  default costs). Each payer keeps a cursor into its payments in payment order. A prefix fails by advancing the
  cursor, so each payment is removed at most once, and the whole fixed point costs O(payments + removals). A
  customer's payments through a failing bank are that bank's index range. Queued and failed marks are epoch-stamped
  dense arrays, not ordered sets.
- **Parallel without order.** A payer's removal of its own payments is monotone, so its greatest fixed point is unique
  whatever the order (Tarski), and a round can take every short payer at once and reach the same set. A bank's
  removal reads the bank's debit, which a customer's own failure lowers, so it is taken only once the payers are done,
  every short bank of a round at once, which leaves the result the same in any order. 7b runs on one thread, its cost a
  small part of stage 7's (0.07 to 0.28 s of the full load's 2.9 s). A cleared line's losers depend only on its failed
  count. The result is deterministic by construction (Blelloch, Fineman, Gibbons and
  Shun, *Internally deterministic parallel algorithms can be fast*, 2012). 7a runs over due-holder shards, and 7c's
  nets are grouped by a counting sort on (line, party, side) and applied by target shard.
- **Day buffers** live on the books across days: cleared by epoch, never freed. After the first heavy day the kernel
  maps no new pages.
- **Verdicts and audits** fold into the passes that already hold the values. An account's movement is checked where
  its net applies. A failed payer's shortfall is read at its cursor. Per-line balance totals are kept at the apply,
  so the audit's money and contract families read sums and a rolling slice. The accruals' mismatches are running sums
  per line. Arrears keep their start and compute their age when read.
- **Agents.** 3e's outcomes run over agent chunks on the pool (an agent's writes are confined to it, PC-34): each
  household is changed and the words it is to be written back as are reckoned there, and only the writing, with the
  moves of attachments where a person is gone, follows in slot order on one thread, since a gone person's contracts
  leaving can take attachments off other agents. 3b reads the agenda's agents in slot order. Hazards
  are drawn in batches with Philox's four-lane counter blocks (Salmon, Moraes, Dror and Shaw, *Parallel random numbers:
  as easy as 1, 2, 3*, 2011) and bounded integers by multiply-shift (Lemire, *Fast random integer generation in an
  interval*, 2019).
- **Gathers.** A visit reading rows it does not own batches its reads and prefetches them, in the manner of group
  prefetching and asynchronous memory-access chaining (Chen, Ailamaki, Gibbons and Mowry, 2004; Kocberber, Falsafi and
  Grot, 2015). Hot columns are split from cold ones, so a chunk's cache lines carry only what the pass reads.
- **Saves** write only the stores the world saves, in independent zstd frames compressed on the pool (RFC 8878).
  Incompressible blocks are stored raw, and the bench's held stores carry their columns' shapes, not uniform words.

As built at Stage 0's close: dense identity; the standing facts on their owners (a listed side's holders as a count and
the exclusive-or of their keys, each holder's money rows); the due index, as a map from a day to the holders' keys
filed under it, not yet the agenda's wheel; the day's buffers kept across days, its records dealt to 64 shards by runs
of slots and stamped by day; each due line's dues planned once, and a settled due's interest folded by party. On the
world's pool, in fixed shards and waves so the result is the same with any workers: 3b and 3e; 7a, its bookings folded
a records shard to a worker; 7c's gather, each payment's route made again and what it adds (records, nets, income)
folded a key shard to a worker, the failed and pending recorded in the stream's order; 7c's balances, heads and leg
reads; the audit's families, each into its own findings, kept in their order; the opening's regions; and saves, each
store to its own file at once, in fixed 1 MiB zstd frames compressed a wave at a time. Rows are found by reading their
heads, not by an index; 7b runs on one thread; 7c's balance writes land by holder chunk on the pool, its
checks and records in line order on one thread; the money and contract families
still read an unlisted side whole. The rest is carried to the Stage 1 gate (the plan's S1.16).

---

## 7. The population engine (REP)

### 7.1 Tables

One **agent table** per population kind — household, small firm — in `phx-pop`. Each row is an agent (REP.1): a
household or small firm, one real party, never split, joined or averaged. The player's household is one of them
(OBS.4). Institutions and the largest firms are rows of the kernel's kind tables of individuals
(§4.1).

The books keep the agent tables beside the kind tables, one holder place each after the kind tables', known to
`phx-ledger` only through its traits (`HolderTable`, `CellHolders`), so the settlement stream, the holder lists, the
directory and the saves reach agents as they reach individuals; `phx-pop` reaches its own tables by their type. What
the tables do not hold — each kind's compiled declaration, the agenda and the members counted by event — is the
world's `Population`, which the opening's contributions are handed beside the books.

  A population kind is declared item by item by the systems that write it, through `Declarations::pop_kind`
  (`PopKindBuilder`: `attr`, `role`, `person_attr`, `position`, `sited_by`), each item recorded with its declaring
  system as its one writer (Law 10). `phx_pop::kind::PopKindDecl::compile` lays it out: attributes; roles and person
  attributes packed into the person's word, each attribute the bits its values need; positions, a column each in the
  order declared; and the attribute that sites the agent in its region. `AgentTable` implements `phx-ledger`'s
  `HolderArenas`, `PayerPositions`, `HolderTable` and `CellHolders`, and `phx-core`'s `TableSchema` and `FactStore`.

An agent row keeps, in columns: its party, the day it began, one `u32` per declared
**attribute** (REP.41: a household's region and bank, a small firm's region, size, bank, industry (`FRM.industry`), product (`FRM.product`), the way-set it knows
(`TEC.known`) and the way it runs (`TEC.way`)), the due-day run's head
(§4.5), and arena references to its lists in its chunk's arena:

- **rows** and **holdings**, the ledger's (§4.5), each row counting the contracts the agent holds on the line
  (REP.3);
- **persons** (REP.26), one word each: the birth date as the calendar's civil day serial (32 bits, so the oldest
  persons, born before the world's epoch, are held exactly), the role (head, partner, adult, child), and the kind's
  declared person attributes packed after it (sex, health, education), their widths compiled from their values;
- **attachments**, one word each: a person's place in the household (or the household itself), the line and the
  side, so a person's death finds the rows it held without reading any other household.

Positions (REP.20) arrive as declared `i64` columns with the steps that need them, missing until their writer writes
them unless the declaration names what the opening writes (`PositionOpening`): the one kind so far is what the
agent's contracts owe it over the year after the opening, read on the pool once every line is open
(`World::positions_opened`). The table counts its agents as
they begin and end, and the population counts their parties and persons by the events that begin and end them, so
the day's measures (REP.15) visit no agent; the agents family checks the counts against the table (REP.13).

A kind is declared item by item (`PopKindBuilder`), and the system that declares an item is its writer. The items are
attributes (`AttrDecl`, with the number of values each takes), roles (`RoleDecl`), person attributes (`PersonAttrDecl`,
with the value a person holds before any system gives it one), positions (`PositionDecl`), and the attribute whose value
is the agent's region (`sited_by`). An estate is sited in that region. `phx-pop` compiles the items into the kind's
layout, and each width comes from the number of values. An agent's persons and attachments are written only by
`phx-pop`, by the openings that form households (`sys-dem`) and by the world. `phx-check`'s PC-34 refuses these
writes from any other crate.

### 7.2 Arenas, locality and identity

- Variable-length lists live in **chunk-local arenas**, compacted in place per chunk with a chunk-sized scratch;
  freed pages return to the system. Columns sit in reserved address space; nothing reallocates; no store holds a
  heap-owning type.

  As built (`phx-store`): a column reserves address space for its declared maximum rows through a `Backing`
  (`mmap` with `PROT_NONE` and `MAP_NORESERVE`, committed by `mprotect`, decommitted by `MADV_DONTNEED`; a zeroed heap
  allocation under Miri), and commits pages as it grows, at the page size read from the system; reservations count no
  resident memory, and all of them together stay within 64 GiB (`VA_BUDGET`). Growth past a reservation is
  `capacity_exceeded!`. Rows sit in chunks of a power-of-two `rows_per_chunk` declared per table (4 096 by default),
  whose disjoint `&mut` views are what parallel handlers own. `SlotAlloc` hands out the lowest free slot, then extends;
  a slot released during a day returns to use only after the close, so the slot a row receives depends only on the
  order of allocations. A chunk's arena is 8-byte words holding its rows' lists by `ListRef { off, len, cap }` (an
  agent table's compact `CellListRef` of 16-bit length and capacity, a longer list's reference kept in the arena's
  overflow map); a list grows in place while it has room and otherwise moves to the arena's end with 5/4 of its need,
  rounded up to four words, leaving its old span dead; removal shifts down, so a sorted list stays sorted. When dead
  words pass an eighth of the used, compaction copies every live list in slot order through a chunk-sized scratch and
  decommits the pages above the new end. Only a chunk's own rows hold references into its arena. Lists that are not a
  chunk's own — a line's or an instrument's holders — are `BlockList`s: sorted sets held as a tree of 16-entry `u32`
  blocks (one cache line) from a shared `BlockPool`, leaves chained for iteration and inner nodes keeping each
  subtree's least entry, split and merged at half occupancy; adding a present entry or removing an absent one is a
  violation.
- Agents are drawn region by region at the opening, so a chunk holds neighbours; an agent keeps its slot for life,
  and an ended agent's slot is reused by the next one to begin, so no renumbering is needed.
- The **directory** maps permanent identities to slots. A party that ends leaves a tombstone only while a record or
  an event names it (a reference count, taken when the record or event is written), and until the day's close in any
  case, so the day's legs that name it still resolve for the audit; its slot is recycled at once. A row's arrears go
  where its contracts go: a row retired takes its arrears entry with it, and a line transfer carries it to the row
  that takes the contracts.
- As built, the directory keeps live parties in a paged map from identity to `RowRef` and ended ones in the kernel
  map as `Ended { day, successor, refs }`. A record kind that may name a party retains and releases it; an ended
  record is dropped when its references reach zero and the day has closed, releasing its successor in turn. `lookup`
  gives live, ended or unknown — unknown below the counter meaning nothing retains it, so a store naming it is a Names
  finding — and `resolve` follows successors to a live party or to an ended party with none (an estate that has
  distributed), which reads as ended (PTY.10).


### 7.3 The agenda and hazards

The **agenda** of a day lists the agents that act that day: agents whose decision schedule (TIME.5) falls today,
agents woken (a message, a surprise, the player's intent), and agents with a hazard booked today (payments are the
settlement stream's, §6.5). `phx-core` keeps each agent's next day per reason — a reason per process on its kind —
and **one calendar entry per agent**, at the earliest of them, in a timing wheel of day buckets; 3b gathers today's
bucket and reads each agent's reasons due today.

Each table the agenda books declares between one and 16 reasons, so a row's next days, one `u32` each, fill at most
one cache line; beside them the row keeps `booked`, the day of its one live entry, never later than its earliest
reason. Setting a reason earlier than `booked` writes a new entry and leaves the old stale; a later one writes
nothing. Each table has its own two-level wheel of slots: 1 024 day buckets for the current 1 024-day block and 1 024
block buckets, whose entries cascade into day buckets on the block's first day and otherwise wait a turn of the
wheel. Buckets are `BlockBag`s, chains of the block pool's 16-entry blocks, pushed to and drained whole. A gather
walks every day since the last, drops stale entries (those not in the bucket of their row's `booked`), sorts the due
slots once, returns each row's reasons due as a `u16` mask and books the row again at its earliest reason after the
day, so a reason moved later is never lost. A table is compacted when its stale entries outnumber its live ones.
Renumbering moves or swaps a row's reasons and entry; a released slot's entry goes stale. A save writes each row's
next days, and the wheel is rebuilt on load.

Every process on a kind is drawn **ahead** (REP.7, the next-reaction method). For an agent and a process, from a day
_d_: every person's daily chance _q_ᵢ is read at _d_ from the agent's own state, and the agent's chance of a hit on a
day is _p_ = 1 − Π(1 − _q_ᵢ), constant until the next day any person's rate may change (the process's
`changes_after` for each person, the earliest of them, `c`). The days to the next hit are one geometric draw of
chance _p_ from the agent's stream for the process; if the hit falls before `c`, it is booked with the agenda's mark
of a hit beside its day, and otherwise a **redraw** is booked at `c`. Both are exact, since a day's chances are
independent of the days before: the redraw starts afresh from its day. Whatever changes an agent — an outcome, a
person leaving or joining, a new attribute — books every process on it afresh from the next day, at 10b; the
opening's agents, the player's seat among them, are booked the same way once the opening is done.

  _p_ is computed without cancellation, as −expm1(Σ log1p(−_q_ᵢ)), one when any _q_ᵢ is; a chance outside [0, 1]
  stops the run. A process at _p_ = 0 with no day its rate may change is not booked (`hazard::Booking::Never`).

A process's rate reads a table primitive at its declared axes; an annual probability _q_ becomes the daily
`−expm1(log1p(−q) / days)`, with the calendar year's own days, so a year of daily chances compounds to _q_ exactly.
A process acts on a role or every person of a kind's parties, a party, a holding of a class, or a tile, region or
country; tile and region processes are `phx-geo`'s at 3a, the rest the population's at 3b.

A booking come due is followed on from its own day until one falls after today: a hit reaches its persons and the
next booking is drawn from the day after it, a redraw draws from its day, and the persons a hit reached are passed
over by the later draws of the day, since their outcomes have yet to change them.

On a booked hit day, 3b draws which persons the hit reaches, each by its own _q_ᵢ **conditioned on at least one**
(`binomials_joint_at_least_one`), records the event, and hands the agent to 3e.

A hazard on a party itself (a firm's plant failing, a vehicle's accident) reads the agent with no persons: its _q_ is
its own. Catastrophes are drawn at 3a (§7.8).

**The processes on persons** are `sys-dem`'s `PopProcess`es. Each is bound to the register once, at assembly
(`PopProcess::bind`), and there builds the tables it reads for each country. Death reads each country's life table by
Brass's relational model: its survivorship logits are the sourced standard's plus one level, the same for both sexes.
The level is solved at binding, by bracketing and then halving, so that life expectancy at birth equals the country's
`DEM.life_expectancy` (§10.0), with the sexes weighted by the sex ratio at birth. Past the table's last age, the open age
keeps the last year's hazard. That hazard is split by health: the disabled's is `DEM.disabled_mortality` times the
able's, and the able's is the table's over 1 + P(r − 1), P being the chance of disability at that age and sex as the
opening draws it, so the two weighted make the table's. A person's chance of dying before its next birthday, read at
its exact age from its birth date and its health, is compounded over the days of that year of age; a change of health
draws its next death afresh. The onset of lasting disability spreads its yearly hazard by age
and sex over the same days, and is zero for a person already disabled. Both rates change on the person's next birthday
(`changes_after`), so ageing is read from the birth date and never drawn. A death marks the person gone, and a dead
head's place goes to the partner, else the eldest adult, else the eldest child (`household::succeed`). An onset makes
the person disabled in place.

**Decisions on the agenda (visits).** A system declares each decision it takes on the rows of a kind as a **visit**
(`Declarations::visit`): the handler that takes it, the kind — a kind of individual or a population kind — its stream,
the wakes it answers, and its **cadence**: a continuous decision's schedule (TIME.5), each row at its own phase within
the period, drawn once and kept beside its booking; or a lumpy decision's reviews (REP.21) at the daily chance the row's
attention position holds (REP.38), none while it holds none. The assembly refuses a visit whose handler is not its
system's, runs on another table or outside 5b and 5c, or whose attention the kind does not hold, and one that names a
wake other than a surprise on a review drawn at attention. A surprise wakes such a review (REP.35): at 5a, after the
public outlooks, `stance_wakes` reads each row's product, method and place, finds the series its stance reads — its
product's market between firms where it stands, or else its country's retail market — and, where that series surprised
its method at the day's print, books its review for the first day it can still run. The visits keep an
agenda of their own beside the processes' (`Population::visits`), one table per visited kind, a reason per visit, saved
with the population. Every row is booked once the opening is done; the rows due are gathered on the day's first decision
sub-step, each visit's handler runs on its rows due in runs of consecutive slots over its kind's table — an agent
table's positions (REP.20, declared on the kind with `PopKindBuilder::position`) or a kind table's facts — and each row is
booked again after it. A visit of at least `VISIT_SHARD_ROWS` rows runs in `VISIT_SHARDS` fixed shards on the pool
(`World::visit_sharded`): each shard reads its rows' declared facts into a `phx_core::FactOverlay` before its handler
runs and keeps the handler's writes aside, with its own intents and bindings; the shard holding the player's row runs
after the others with the player's queue; and the writes, intents and bindings are taken in the shards' order, so the
day is the one a row-by-row run makes on any pool. A row reads and writes only its own declared facts, and a read
outside them on the pool stops the run. The goods, plant and rights a visit's rows hold are read for it in the same
fixed shards (`World::run_goods`), each shard's lists joined in order. A traced run visits row by row. Each row is booked again: its schedule's next instance, or a review drawn afresh at the attention its decision left it.
A review's row whose attention another visit's handler moves is booked afresh at the new attention the same day, so a
review is never drawn at an attention the row no longer holds. On a run that traces reads, a handler's reads and writes
on its rows are checked against its declaration and counted into the day's read trace. What each day's visits did —
the rows each handler visited and the facts they moved to a new value — is kept as the day's `VisitDay`. A row
that ends leaves its bookings to lapse: a due row no party holds is passed over.

**The firms' decisions** (FRM.5, REP.21, REP.34, REP.38). What a firm holds and expects is declared once in `if-firm`
under one name for both kinds: a large firm's as facts of its row, a small firm's as positions of its agent
(`if_firm::facts::POSITIONS`); its industry, product, known way-set and way run are keys, facts of a large firm and
attributes of a small one. `FRM` writes the sales outlook (`FRM.expected_sales`,
`FRM.sales_width`), the units delivered at its last schedule and last review, its method, last review day, unit cost,
markup, posted price, output rate, price attention, the cost of its staff's hour and required return; `TEC` the ways;
`CAP` the capacity and the investment review's counts. `sys-frm` declares two visits on each kind:
- `FRM.attend_*` (5b, on the production schedule `FRM.production_days`, business days): closure is weighed first
  (§7.11); then the units of its product delivered since the last schedule enter the sales outlook by its memory
  type's adaptive gain (`FRM.method` modulo the memory types), the surprise's size the width; then the price
  attention, the daily chance `phx_val::attention` gives from the loss's curvature — at a markup μ a log gap x loses
  R·x² ÷ (2μ) of revenue R, so the curvature is daily revenue over μ, with no primitive of its own — the outlook's
  relative variance a day, and a review's cost, `FRM.review_hours` at the staff's wage: none while it expects no
  revenue, every day at or below cost. A surprise beyond `VAL.attention_sensitivity` widths sets the chance to one, so
  the review is drawn for the next day, as a surprise in the public series its stance reads wakes it (`stance_wakes`);
  then its stance is reconsidered (VAL.7). Production follows; for the next period each storable input it will be
  short of, counting what it holds at other places, is bid for where it lands for least — its own place's market, or
  another's with the freight home by the cheapest mode carriage is posted in there, each where its market last met a
  seller, in the grade class landing cheapest — when the input is worth its landed price financed: that price, and the
  margin a unit made earns over its unit cost spread over what the unit takes of it (FRM.4, GDS.5); bought elsewhere,
  it is shipped home at its next visit. A firm's output rate (`FRM.output_rate`) is what its staff make a day: the hours a
  week its employment lines hold (`Ctx::staff_hours`, counted with its net assets as its rows are read) over its hours
  a unit, rewritten at each production decision, so hiring and layoffs move what it and its extraction and retail
  sales can make; it is kept in millionths of a unit a day, so a firm making less than a unit a day is counted
  (`phx_core::fact_scale` reads a fixed-point fact).
- `FRM.review_*` (5c, at that attention): the markup moves by `rules::markup::update` over the sales since the last
  review against those expected over its days and by what competitors charge — its product's last price between
  firms where it stands, or else its country's retail mark for it (`Ctx::posted`), absent while neither is marked —
  and the point
  nearest `(1 + μ)·unit cost·π^η` is posted, π the stocked pressure — the sales since the last review against those
  expected over its days, and the stock it aims for against what it holds of its product, each ratio counting a
  period's expected sales, so a period that sold nothing, as a seller of whole lots often has, reads as weak demand
  rather than as none at any price — only when the loss it saves over the days to the next expected review exceeds
  `FRM.menu_hours` at the staff's wage. The review then sets its attention back to the chance its loss's curvature and
  its outlook's width give, so a surprise wakes one review, not one a day until its next production schedule.

A trade's point table (`FRM.price_points`) holds one decade's points; a price's candidates are its decade's and the
two neighbours' points scaled by powers of ten, each whole (`Management::points_near`), so every posted price is a
point at any scale. The management's values are one shared type (`data/shared/FRM.toml`), compiled once
(`Management`). A firm missing an input its decision reads decides nothing; nothing is read as zero.

### 7.4 Outcomes and endings

At 3e each agent hit that day is made explicit — its attributes by name, its persons by role and attributes, and its
positions as read, `phx_core::Household` — and each process's outcome changes it in the processes' declared order
(POP.3 before POP.4). Its positions are handlers' and are never written back; a person an outcome adds is a birth, counted with the deaths.
Then:

- a person gone (a death) leaves every row its attachments name: the row's count falls by one, and the line's other
  side loses as many contracts, drawn one at a time from its holders by their counts (REP.23), in one
  `members_leave` per row. What the other side cannot give passes instead, by moving members, to another holder of
  the leaving party's own side, drawn by its members, as a buyer of the contracts would take them, with no arrears
  carried, since no one failed them; if that side holds no one else either, the run stops (REP.31). An agent drawn
  loses, from its persons, the members that left, drawn among its attachments on that side of the line
  (`World::detach`), so its persons still hold what its rows count. A side's members by holder are read once
  (a `Tally`, from the holder list's keys straight to their rows, in fixed shards on the pool) and kept, moved by
  each leaving and passing, while no other change reaches the side. A side that keeps no holder list is read once a
  day, by `Books::read_unlisted` in one pass over the tables of the kinds that may hold it, before the estates that
  leave against it; every side's tally is let go once its pass is done (`forget_tallies`), so none outlives the
  changes the next pass brings; the households' sides of employment and tenancy keep
  lists, since a firm's end reaches them (§4.5);
- the household's persons and attributes are written back in place; the head's place, if emptied, is taken as the
  outcome says;
- a household no one is left in **ends**: an estate is opened at a zone centre of its region, and every row it held
  passes to it whole by line transfers (PTY.9); its slot is freed and its identity ends. Its holdings pass to the
  same estate (`Books::pass_holdings`), a small firm's plant and goods among them; an agent ending with named units
  stops the run.

Nothing is split or landed: an agent is changed where it is, or ends.

**Births and leaving school** (`sys-dem`, POP.5, POP.10): a household decides on its head's birthday (`DEM.fertility_occasion`, taste `DEM.fertility_taste`) whether to try, and tries when the next child's value `ln((1 + n*)/(1 + n)) − ln(e(n + 1)/e(n)) − ln(1 + 1/(1 + a)) + s·ε` is positive — `n*` its ideal (`DEM.ideal_children`, drawn from its country's shares at its first decision and held as an attribute beside `DEM.trying`), `n` its children, `e` its needs on `DEM.equivalence_scale`, `a` its youngest child's age, `ε` a standard logistic draw; with no income outlook it does not try. While it tries, each woman of its couple conceives at `DEM.fecundability` a cycle compounded over `DEM.cycle_days` (`DEM.conception`), and a conception is a birth at once: a child in school, its sex by the sex ratio at birth, the household then trying no more until its next decision. A child leaves school on the birthday its country's `DEM.school_leaving_age` falls on and becomes an adult out of the labour force (`LeavingSchool`, a placeholder naming POP); the opening's children being those under the age of majority, those past the leaving age leave on day one.

A side's `Tally` keeps its holders in a Fenwick tree over the members they have left, so drawing a member costs the
log of the holders. A cleared line's `Losers` draw
from the same structure. A member leaving takes no share of its row's balance, which stays a claim the line still holds,
and its counterparts give up none of theirs. The leaving draws come from the stream `REP.leaving`, at 3e and at 7c for
an estate's rows. An estate's site comes from `REP.estate_site`. Both streams are keyed by the party.

### 7.5 Lines, counts and settlement

A row's count is the contracts the agent holds on the line (REP.3), so every line's two sides hold equal counts of
real contracts. A rule's amount for an agent is rounded once, by its convention (REP.9).

Settlement reads agents as it reads individuals, through `PayerPositions`: an agent's funds are its account's balance
less what is pending, with its facility, and its payments are met in its payment order while they last (§6.5). A
cleared line's claimants drawn to lose a failed payer's dues (REP.23) are drawn one member at a time the same way
(`Losers`), exactly as many as failed.

### 7.6 The representation

One register primitive (`data/shared/REP.toml`, RESOLUTION) sets it (REP.40, spec Appendix E 44): `REP.persons`, the
persons the world holds of the setup's `GEN.population`, which the opening splits among the countries, so
everything derived from a country's people — the employed, firms, lines, the individuals' rank
(`FRM.rank_per_million`) — follows the world's size, and every household and small firm drawn is one agent of one
party; a size above the setup's population stops the run. `phx run --persons N` sets it before assembly in place of
the register's, for a run of another size; the population's store and the save's manifest record it, so a save
loads only under its own, and `phx inject` reads it from the manifest (§11). The agents are an outcome: the
households the persons form and the small firms they employ, about 0.43 an agent a person (221 515 households and
102 579 small firms at 750 000 persons, seed 1).

The size is set by measuring the day (N8.5). Measured on 2026-09-28 (build machine, four cores, seed 1, 40 days at
750 000 persons): a median business day of about 7 s (7c 3.3 s, 6d 1.0 s, 10d 1.0 s, 6a 0.5 s, 10a 0.4 s, 5c 0.4 s),
a payday of about 21 s (7c 10.7 s, 6d 10.2 s), a peak of 5.4 GB, about 1.25 of the four cores busy. The systems'
rules take under 1% of the CPU time; the ledger 30%, the world's plumbing 20%, page faults from buffers growing in
the day 15%, the per-leg audit 8% and allocation 7%. The budget is missed several times over, in the representation
and its traversal, not in the mechanisms; the restructure (the plan's S1.17–S1.25) replaces them, and 750 000 stays
the committed size until it lands.

Identical twins — agents standing for many parties each, the full population held at the same number of agents —
were retired (spec Appendix E 44): their decisions moved their parties in lock-step, their lumps broke whole-unit
trades and hires, and they held no more distinct agents than a world of the same agents' persons.

The **player's household** (OBS.4) is drawn at the opening among the country's household agents, each equally likely.

### 7.7 Relationship counts and their levers

Relationship rows per agent dominate memory (§13.1). Line terms are what the spec says they are (LAB.1, REP.3: an
employment line is occupation family, skill, wage point, hours, notice and severance terms, pension kind and rates,
start band and region, with its employers on the other side; the scheme a member belongs to is its attachment's, so it
splits no line). Instruments are rows with counts (§4.5). The levers, all declarations, each set for play by
measuring the budget (N8.5):

- **The factor** (REP.40): agents, rows and their work fall with it in either representation.
- **Start bands** are RESOLUTION (Law 9): five years at play.
- **Contract grids** are price points and vintages, conventions of each trade (REP.34).

### 7.8 Places and catastrophes

`phx-geo` keeps **physical stock per (tile, class)** as the one writer of where units stand, and an index listing,
per (zone, class), the **holding** rows (owners) of that class there. A catastrophe at 3a draws in two levels:

1. the units lost are allocated across the holdings of the struck (zone, class) by one multivariate hypergeometric
   draw, and each holder's lost units come from its own count;
2. for each holder hit, the rows its lost units carry — mortgage, dwelling insurance, tenancy — are read from its own
   rows, since their terms name the same zone and class.

**Victims** of harm to third parties (Stage 4) are drawn the same way: for damage to property, a holder from the
(zone, class) index of the harm's zone, weighted by its units; for injury, a household agent of the zone, weighted by
its persons, whose illness `sys-dem` applies as the hit's declared reader.

The same index drives the **holding levy** (property tax, §4.3): on the law's dates it lists the holders of each
taxed class in a zone, and each holder's amount joins the (party, bank) leg it already has in that day's payer pass,
so no row is kept per owner.

A landlord's lost units reach its tenants: the tenancies of the struck (zone, class) on that landlord's lines are drawn
from their tenant side (REP.23), and each hit tenant receives a notice occasion to move. An insured loss opens a
**claim** message to the insurer; a mortgage whose collateral is lost stays a loan with its collateral description
marked lost, and its lender reads that on its next review (REG.9, BNK.17).

The map, its zone distances, the regions' climates, the exposure columns and the deposits are compiled once, at the
opening's map phase, into `phx-geo`'s own state, which its handlers and families share read-only; what changes day by
day is kept in its kernel tables (a region's weather latents, a finite deposit's extracted and remaining quantities)
and hashed with the world. Weather is drawn per region and catastrophes per country at 3a, and both are recorded as
events at stage 3's apply, made public at 10a by the declared rule (§4.10): a variable's value per region a day, and a footprint's struck tiles with their
severities. The losses at owners (the two levels above) join them when stock is placed.

The surface is **closed** (GEO.1): `Grid` wraps both ways, so every tile has the same eight neighbours and there is
no edge anywhere. The wrap lives in `Grid` alone: its neighbours, its directions and its lengths, which take the
shorter way round each axis. Everything else reads them and never touches a coordinate itself, so countries,
regions and zones, zone distances, the distance to the sea, rivers and catastrophe footprints all cross the seams
without a special case. The relief's noise tiles with the grid's period, and plates are read at their wrapped
distance. A row's climate is read at its place in the latitude cycle (GEO.18): its declared warm latitude at the
first row, its declared cool latitude half the world away, evenly between. The phone draws the map scrolling
without end in both directions.

A tile is 12 bytes (`Tile`: elevation in metres, surface, terrain and climate classes, zone); its coordinates derive
from its row-major identity, and its region and country are read through its zone. The grid's side is set so the
declared land count lies at the declared sea share. The map is generated in the opening's map phase
(`phx_geo::generate`) from the stream `GEO.map`, one subject per attempt, which:

1. raises the relief on a fine grid of four cells to a tile's side: plates over a warped closed plane, crusts blended
   across their borders and ridged belts raised where they converge, and fractal gradient noise over a second warp
   (`noise`, lattices drawn once per attempt), all repeating with the grid; the declared count of tiles of highest mean
   is land;
2. erodes it by the stream-power law over a priority flood's drainage (`hydrology`), solved implicitly from the outlets
   up, the cells of a flat taking their turns by lot;
3. ranks the land's cells onto ETOPO1's measured land heights and the sea's onto its depths; a tile's elevation is its
   cells' mean, and its relief, their range, is ranked among the land tiles onto ETOPO1's ranges over windows of a
   tile's span;
4. reads terrain from elevation and relief by the declared classes; a tile's river is the largest stream of the fine
   grid through it, draining where that stream flows on; its climate class by its row's latitude, distance to the sea
   (coastal, inland, interior) and, above its band's declared elevation, the band's highland class;
5. splits the land into countries, each country into regions and each region into zones by **exact halving**
   (`partition::split`): the parts are cut into two groups, the tiles ranked by how much nearer one end they lie than
   the other by travel cost (length over the ground, lengthened by relief climbed and a river crossed) and cut at the
   first group's share, then each group halved again; the ends are the tile furthest from one drawn and the tile
   furthest from that. Tiles of equal rank (a peninsula's) are never parted, and a tile off the largest piece ranks
   with its nearest tile on it, so an island goes with the coast it faces;
6. reads each land tile's exposure per hazard from its terrain and climate by the coastal, river or inland table, one
   column per declared hazard, and draws its deposits per resource — present at its terrain's chance, grade and,
   unless unbounded, quantity log-normal; assembly refuses a resource declared present on every terrain.

An attempt that fails a declared construction condition — a country's land or a region's size beyond its tolerance, a
country's mainland below its floor, a region in more than one piece, a zone outside its size range — is recorded with
the condition and the next attempt drawn, nothing nudged; past `MAP_MAX_ATTEMPTS` the run stops at
`capacity_exceeded!`. The curves come from NOAA's ETOPO1 over the analogue region (35–58°N, 10°W–60°E), the climate
tables from NASA POWER's daily reanalysis (1991–2020) at 71 Old-World places, the hazard rates from EM-DAT over the
World Bank's land area (`tools/data/{relief,climate,hazards}.py`, the raw data in `data/sources/raw/`).

**Distances**: a leg is the three-dimensional distance between tile centres over their elevations, rounded to whole
metres before summing. `ZoneDistances` holds the Dijkstra lengths between zone centroids (a zone's tile of least summed
distance to its others) within each country, computed once when the map is accepted, none across a border;
`path_length` between two tiles is A* on demand, bounded by the plane distance.

**Weather and catastrophes** at 3a: a region's climate is its tiles' classes' parameters weighted by their count. Each
region moves a latent Gaussian AR(1) per variable, persistence declared per class, and maps it through the month's
declared marginal (`Marginal`): temperature normal, rain a zero-inflated gamma, wind Weibull, sunshine a beta over the
day's clear-sky index. Per country, hazard and exposure class, the day's count of origins is one binomial over the
class's land tiles at its daily chance, the origins picked uniformly among them (Floyd's method); a footprint spreads
breadth first over the eight neighbours in tile-id order, each untried land tile joining at the hazard's spread chance
for its exposure, and each struck tile draws its severity from its exposure's distribution.

**On the core** (`core_weather`): the world's day begins with `Core::weather_day`, which draws each region's
weather (`phx_geo::weather::region_day`, its latents kept in the core's `Weather`, saved with it) and each country's
catastrophes (`phx_geo::catastrophe::hazard_day`), each recorded in the core's event store with its region or its
struck tiles, the weather public by the declared rule. The struck tiles and their severities wait for the day's goods,
where `Core::destroy`, beside spoilage, destroys each severity's share in permille of every free unit of every good a
firm sited on the tile holds, whole units, as a flow to nature (`DESTROYED`) the goods family reads and a loss of the
units' cost in the owner's accounts. Plant, dwellings and infrastructure join what a catastrophe destroys when they
are held on the core. Each catastrophe is followed for the prices' reads: its regions and every mark on its day, and
the first day after it each (product, region) mark rose above that (`Shock`), which LC-1-46 reads.

**Deposits on the core** (`core_deposits`, GEO.6, GEO.9, GEO.12, GDS.3, GDS.4, GDS.12): each deposit's right is a
holding over its tile, given at the opening to the firm of its region making the product that draws on its resource
(`TEC.products`' `extracts`, `TEC.deposit_draw`) sited nearest it by plane distance, the lower slot on a tie; a deposit
no such firm's region holds is held by no one. The core's `Deposits` keeps, by the deposit's place among the map's,
its holder, what it has given and, if finite, what it holds and opened with. An extractor decides whether to work
what it holds on its extraction schedule (`GDS.extraction_days`, each firm's day by its slot) and before its first
making, through the decision core (`GDS.extract`, Hotelling's rule: its price net of its unit cost against its
stance's outlook of its mark at its next decision, discounted at its required return). The production rule's capacity
is at most what its deposits give at its way's draw a unit, none while it does not work them; each unit made takes
the draw, rounded up, from its deposits in their order, a finite one never below nothing. Its rights pass to its
estate as its goods do, and an estate holding rights waits for its liquidation. The deposits family holds each finite
deposit's given and held to its opening at every close. A deposit's grade and the grade classes of what it gives are
not yet kept: every extracted unit is of its product's one class, until the commodities' grades and their call.

### 7.9 The valve

- **The world runs once** (spec Appendix E 36): there is no weight-one run, and no run at another resolution or seed
  to compare with. The representation is judged by the run's own macro results against real economies' (N3, N4).
- **The valve** (N8.5) is a declared RESOLUTION setting, never an input the world reads from its own timing: it
  changes only between runs, by a recorded change citing a device or measurement report (§14.8). Its settings are
  register primitives (`data/shared/REP.toml`), which the manifest's register hash covers, so a load with a changed
  valve is refused as other data (§11).


### 7.10 Plant

- **Condition classes** (REP.24): each kind of plant is held in `CAP.condition_classes` classes of age, each an
  instrument per (country, kind, class), so plant of one class is alike: an agent holds it at average cost like any
  holding (§4.4) and nothing is kept per unit. A system declares each kind's classes to the ledger as a **chain**
  (`phx_ledger::chains::Chains`, saved with the ledger), tagged with its kind; no instrument is in two chains.

A kind of service life L in C classes gives each class a span L ÷ C; a class's **efficiency** is the hyperbolic
(L − a) ÷ (L − β·a) at its mid-age a, its **value** e^(−δ·a) of the cost new, and units leave each class at C ÷ L a
year, so a unit's life is Erlang about L (`sys-cap`'s `rules::wear`). The plant review is its own visit every
`CAP.review_days` calendar days.

(And at the end of the **Investment** bullet:)
The rule (`rules::invest`): the project's value is the margin its extra output earns a year, an annuity over the
kind's life at the firm's required return, against the plant's cost at the kind's product's mark where it stands;
the multiple of cost it must beat is Dixit and Pindyck's β ÷ (β − 1), from the rate, the payout over value and the
volatility of the firm's sales between its last two reviews; it invests when the value beats that and its money
covers the cost. `rules::invest::cost_of_funds` (debt quote and owners' return weighted by leverage) and
`rules::maintain::choose` are built and not yet read.
- **Wear** is declared, not coded per system: a `WearDecl` names the visit it follows and, from the register, each
  tag's rate of leaving a class and each class's value. After that visit's handler has run on its rows, the kernel
  (`phx-world`'s `wear`) realises every chain for each row visited, over the days since its last visit (the period;
  on its first, the days since day zero): from each class, the share a constant yearly hazard takes over those days,
  1 − e^(−r·t), half to even, move to the next class carrying their lots' first-in cost times the ratio of the two classes' values, and the
  last class's leave. One instruction per row, of `Transformation { source: Wear(chain), cost }` legs under the
  reason `worn`, paid and received both an expense, so the cost the units lose is charged once to income through the
  effect and once to the unit through its lot. The visit runs on calendar days, so its period is the days worn.
- **The check**: the apply routine refuses a wear instruction whose legs leave a chain or make units (a class
  receiving other than what the one before it gave the same holder); each wear leg's digest carries its chain and
  class, and `CAP.stock` checks the same from the audit's own record.
- **Capacity** reads the classes: a way's capacity is the least over its kinds of units × the class's efficiency over
  the way's plant per unit of output, a `DeclaredLimit` from what is held, never a bound. The owner's plant review
  (5c, with the wear) writes it as the fact `CAP.capacity`, its units a day, which production and a service's
  stall read the next day beside what the staff can make. A making's intent carries the days of making it stands for
  (`Transform::days`), and each making is read against its maker's capacity over those days as its sub-step's
  handlers decided it (`makings_decided`), before a later review moves the fact; one beyond is counted (CAP.9).
- **Investment** (CAP.5, CAP.2): each chain has one more instrument, its **plant under construction**, held apart
  from its classes (`Chains::declare_building`). The investment review (5b, on the plant review's schedule) reads
  what the firm sold since its last from its own deliveries, keeping the count and the period's sales as CAP's facts
  (`CAP.delivered_seen`, `CAP.sales_seen`), their change its uncertainty, yesterday's price, since the day's 5b and 5c
  write it, and the capacity its plant held gives, found as the plant review finds it; it emits a `CAP.invest` intent (the kind, the product it is
  bought as, its units, the stages its lead time gives); the world admits it at 5d and at 6a names the builder
  among the day's stalls of that product — the seller in the owner's reach and country at the lowest posted price —
  and begins a `Project` (saved in `Chains`) at that price. At 6d each project's stage is a trade under `CAP bought`:
  the builder's goods, made by its way at delivery for a service or covered from its stock, used up by
  `Purchase(owner)`; the owner's money to the builder; the units into the plant under construction by `Built(project)`
  at what was paid. It settles with the day's trades in stage 7; a stage the builder cannot deliver waits. On the last
  stage's settling, `CAP completed` moves the project's units from construction into the newest class at the cost they
  carry. The apply routine refuses a `Built` leg on anything but a chain's construction or newest class, and plant put
  in service other than what construction gave.

**Plant on the core** (`core_plant`, CAP.1, CAP.2, CAP.5, CAP.6, CAP.8, CAP.9): plant is capital units in its owners'
holdings (`phx_core::units`: `Class` of kind, band, condition and region, each class a declared unit), so the goods
family holds it as it holds goods. At the opening each firm holds, of each kind its way needs (`TEC.capital` per unit
of output a year, `sys_cap::CapOwn::needs`), its output a year times that in efficient units, spread over the kind's
`CAP.condition_classes` by the weights of a stock grown at its country's `GEN.growth` (`rules::wear::steady_weights`),
each class's units at its value's share of the kind's product's opening price new — the steady-path convention. The
production rule's capacity is at most its plant's (`rules::capacity`: the scarcest kind's efficient units over the
way's plant a unit, a day); each making is counted where plant bound it and where it went beyond (never, by
construction). Every `CAP.review_days` each holder's plant wears along its chains (`phx_core::units::wear`, flows with
nature under `WORN`), the value lost charged to income as depreciation. A capital good bought as investment whose
product a kind is bought as (`CAP.bought_as`) is used at delivery and becomes its buyer's `Project` at what it paid,
named with its producer, an asset at cost on its balance sheet (CAP.2); after the kind's `CAP.lead_days` it enters
service as new plant at its buyer's region, carrying that cost (`BUILT`). An ended firm's projects pass to its estate,
which then waits. What a firm invests is still the placeholder of its country's fixed investment shared by its
opening turnover (`CAP.invest`); the investment decision by CAP.3 (`rules::invest`) and the owner's maintenance,
repair, sale and scrapping by CAP.4 (`rules::maintain`) are the next of plant.

### 7.11 Goods, orders and trades

- **Goods** (GDS.1, GDS.2): a good is an instrument per **(product, grade class, zone)**, of the real-asset family, with
  no issuer, counted in its product's unit and priced in its zone's country's currency. The ledger's goods table
  (`phx_ledger::goods::Goods`, saved with the ledger) keys them and issues each the first time something names it, so
  only goods somewhere made or held exist. A stock is its holder's holding of the good, its lots its cost (ACC.6): no
  system keeps a second count of it. Every good stands at a region's **market zone**, the region's largest zone
  (`GeoState::market_zones`), where its market meets and the freight network's segments end: an individual's at its
  site's region's, an agent's at its region's, what a deposit yields at the deposit's region's (`World::market_zone`). A
  good moves between zones only by a shipment (FRT); a holder reads its goods at other places (`Ctx::elsewhere`), each
  with what is free of pledges, as goods on their way are not.
- **Rights to extract** (GDS.3): each deposit's right is an instrument of one unit (`extraction right`) over its tile,
  in the same table; who holds it is drawn at the Stage 1 opening (S1.15).
- **Grades** (GDS.13): each storable product declares its grade classes (TECHNOLOGY), one more than its bounds. A
  commodity's grade class is its system's reading of its deposit's or its land's grade; a made good has one class. A
  way takes an input's units of any class alike, its lowest class held first, and a buyer bids in the class landing
  cheapest — a placeholder until the spec decides how a grade enters use (Appendix E 45).
- **Products and ways** (TEC.1–TEC.4, TEC.9, TEC.12): products and ways are data (`if-base`), compiled at assembly
  into `sys-tec`'s `Technology`: each country's one opening way per product and its public way-set per industry. An
  extracted product is counted in kilograms, as deposits hold it, fine enough that what a way takes of one for a
  start, rounded up, is near what it uses (in tonnes, a trace input's start took a whole tonne, some sixty times its
  use); every other in what one US cent bought at world-average prices in 2022, so a way's quantities are physical and
  no price moves a draw. A way states per unit made its
  inputs, hours by occupation family, plant by kind per unit a year, land and deposit, with lead time, batch and yield
  in parts per million; for a start it takes inputs rounded up and finishes output rounded down, and a making's start
  is the least that finishes its output (`ways::started_for`). Way-sets are interned
  once, sorted (`WaySets`), so a firm's known ways are a four-byte identity and membership a binary search. The family
  `TEC.production` reads each instruction whose legs name a way from the audit's own record: the way registered, its
  output its product, each storable input used what the way states for the least start; a service input, used as it
  is delivered, is never a production's leg.

- **What a handler reads of its goods** (§4.9): the `Ctx` gives a row its party's units of each good at its place,
  the deposits whose rights it holds with their grade, opening and remaining quantities, the
  units of a product it has delivered since the opening, and each good's latest mark and its public outlook by a
  method at its place. The kernel builds this view (`GoodsView`) for each run of rows before their handler runs,
  from the holder's own arenas. An agent's key attributes read by name as an individual's facts do.
- **Public outlooks** (VAL.23): at 5a the kernel takes in each market's prints since the last pass as a public series
  (`Markets::public`, saved) and forms every method's outlook of it, heuristic by memory type, the first print being
  every method's first outlook (VAL.10); a firm reads the outlook of its own method (`FRM.method`). At each print
  after the first, each method's surprise moves its width and its record — the exponentially weighted squared error in
  widths (`VAL.performance_memory`) — kept with the series (`PublicSeries::widths`, `records`, millionths); a surprise
  beyond `VAL.attention_sensitivity` of the width before it is kept for the day's wakes (REP.35). A firm's stance
  (VAL.7) is reconsidered on its production schedule: among the heuristics of its memory type, the logit of their
  records on the series its stance reads at its switching type's intensity (`FRM.switching`, a type of
  `VAL.switching_intensity`, drawn at the opening), one drawn on `FRM.stance`; while any has no record the stance
  stands. Each day's stances are counted (`StanceDay`: surprises, reviews woken, methods held, firms per heuristic). The goods view
  holds the outlooks of the goods markets alone — between firms and commodities — whose subject is a good where it
  stands, as the marks are; a retail or carriage market's series is read by its own subject.
- **Cost flows** (ACC.6): an individual chooses first in, first out or weighted average once
  (`Ledger::choose_cost_flow`, saved); an agent holds at average cost (ACC.6). A holding at average cost keeps one lot,
  whose day is its units' days averaged by quantity, so what ages with the lot, as spoilage does, ages as its units
  do.
- **Transformations from handlers**: a handler asks for units of goods made or used up for its own row by a
  **transformation intent** (`phx_ledger::intents::Transform`): its reason, carried as its name's code
  (`name_code`, FNV-1a; two declared reasons of one code stop the run), and its legs. It is applied at the handler's next apply point through `Ledger::apply`; a leg
  taken from a deposit stands at the deposit's zone, needs its right and a product the deposit gives, and depletes it
  by GEO's own write (`phx_geo::deposits::extract`), the deposit's one writer. A handler moves no money.
- **Orders** (MKT.16, MKT.17): a handler asks to trade by an **order intent** (`phx_market::intents::OrderIntent`):
  its market kind's code, the product and grade class it is over, at its row's zone, its side and its steps. At
  5d the kernel admits it into the day's book (one asked later stops the run): its instance of the kind over the good
  (made the first time an order names it, `phx_market::instances`, saved with the markets), in **lots** of the
  product's least quantity, ten to its price exponent, so every trade's money is whole (REP.9); an offer is covered by the ledger's commitment on the
  poster's free units (`Covers::cover`), and one the units cannot cover, or whose steps break a rule, is refused and
  counted.
- **Meetings** (MKT.3, MKT.6, GDS.7): a system declares a **market kind** (`Declarations::market`, a `MarketDecl` whose
  key names the kind); a kind that settles after its day is refused, since goods are delivered the day they trade.
  At 6a the kernel meets every instance with orders by its kind's form, in the markets' order: a **call** at the
  place, whose fills are cut to each order's whole lots, the side that gives more losing lots until the sides are
  equal (`call::whole_lots`), then paired, the largest lots first, each match whole lots of both its parties, and what
  no counterparty's lots can meet cut from both sides alike; fills that only net a party's own bids against its own offers fail the call as `OwnOrders`; or
  **posted** prices between firms (`posted::between`), each seller's offer standing
  at its price and the buyers, in an order drawn by lot, taking from the cheapest they accept in quantities whole in
  both parties' lots. Each meeting's print or failure is recorded; a call's print is its mark. Every order's cover is
  released after its meeting.
- **Trades** (SET.1, SET.2, SET.4): at 6d every match becomes a numbered **trade instruction** under the kernel's
  reason `traded`: the seller's units to the buyer at what it paid, against that money from the buyer's account to
  the seller's, the units covered again for exactly what it delivers, its money the quantity priced, rounded once.
  Trades settle in stage 7 after the day's dues and estates, each all or none through the
  apply routine in declared order, so a buyer who cannot pay records a fail with its cause (SET.3), the units stay
  with the seller and its cover is released. An instruction's own covers count as free to it — the units they hold
  back are the ones it delivers — and are released as it settles; the day's failed trades are counted by cause.
- **Revenue** (FRM.13) is recognised on delivery: `traded` declares the buyer's money paid an asset and the seller's
  received revenue, and the cost its units carry out an expense, the cost of what it sold, while the buyer's lot comes
  in as an asset (`ReasonDecl::held`). The units each party delivers are kept with the goods, which its visits read as
  its sales.
- **Retail** (SRV.4, SRV.5, REP.22): a system declares a retail kind (`phx_market::retail::RetailKind`: its market,
  one instance a product and country, so each market's prints are in one currency and it meets that country's
  sellers only, the kinds that sell, the facts naming what a seller sells and its posted price, the weights
  of price and distance, the reach and the stream of tastes). A buyer's want (`ShopIntent`, units or money) is
  admitted at 5d. At 6a, every day, the day's stalls are read in `STALL_SHARDS` fixed shards of the sellers on the pool:
  each posted price and the free units of the good where it stands, less what the day's matches between firms take;
  a service's good is issued, and its maker named, afterwards in the sellers' order. Each market then meets on the
  pool, its buyers, sellers and draws none of another's, and the outcomes are covered in the markets' order. The stalls in reach of each
  buyer zone are found once; each buyer goes to the open seller it values most at `−α·ln p − γ·km + ε`, ε a Gumbel
  taste, which is to each with the logit's chance, so it draws its choice from that chance, one uniform draw of its own
  stream, over the sellers of its zone ordered by what a unit there costs — the prefix it can pay for — never drawing a
  taste per seller, and a buyer turned away draws again among the rest, as its tastes' ranking would; a seller serves
  its buyers in an order drawn by lot in whole units at its price for a lot pro rata, rounded once as the sale settles
  (`phx_market::retail::paid`), and the rest choose again. Prices are posted for a lot so they carry places below a
  currency's least unit; buying in whole lots made a lot's price — a hundred dollars of a service — the least a buyer
  could spend, and left most of a household's weekly budget unspent. Each sale's units are
  covered at once and, at 6d, become a purchase under the kind's reason: the buyer's money to the seller and the
  seller's units used up by the purchase naming the buyer, settled in stage 7 with the trades. A service is
  delivered as it is made and never held: each maker makes the day's sales of it in one making at 6d, and what no
  settled sale takes perishes after the day's trades settle (`unsold_perish`), as a project stage made to order does.
- **Households' spending** (`sys-hh`, HH.4, HH.5): a visit at 5c every `HH.spending_days` business days reads the row's money (`Ctx::money`, which the goods view gives every visited row), its country (`Ctx::country`) and four positions: `HH.income`, the outlook of its permanent income a year; `HH.after`, what it held after its last decision; `HH.received`, the money taken in since its last look at its income; and `HH.looked`, the month of that look. Its money less `HH.after` is added to what it received; at its first decision in a new month it looks, the month's receipts grossed to a year taken into the outlook at `HH.income_gain` — a month, since its pay comes monthly and a week's receipts would read a payday as a year's income and the weeks between as none. It spends by the buffer-stock rule at its cash on hand in years of its income, its money and the year's income it expects (Carroll's cash on hand holds the period's income): `c* + κ(m − m*)` years of income a year, over the decision's share of a year, never more than it holds; and asks each product its country's `HH.budget_shares` of that at retail as a want of money. The opening writes `HH.income` as what the household's contracts owe it over the year after the opening (`PositionOpening::OwedAYear`, `Books::owed_between`): the dues its paid rows fall due for on their dates, repayments of a balance left out. The buffer-stock rule is solved once at assembly by the endogenous grid method (Carroll, 1997) from `HH.patience`, `HH.risk_aversion`, the shocks' spreads and the chance of no income, with `HH.real_return` and `HH.income_growth` as placeholders naming BFL and HH; its target `m*`, consumption there `c*` and propensity `κ` are read from the solution, never declared (at Carroll's baseline m* = 1.28, c* = 0.994, κ = 0.39). `tools/data/derive_hh.py` derives the budget shares per group from the inter-country tables' households' final consumption (HFCE, 2019): the median over the group's economies of each product's share — of a mining industry's output the resource other users than the one transforming it take (oil and gas, stone), as the ways split it — finance, real estate, public administration and households as employers left out, renormalised; the system refuses shares that are not one a product.


`sys-srv` declares the kind (`SRV.retail`) over `FRM.product`, `FRM.price`, `FRM.output_rate`, `TEC.way` and
`CAP.capacity`; purchases settle under `SRV sold`. The tastes are standard Gumbel draws from `SRV.taste`, so the choice
is multinomial logit exactly, and the serving order is drawn from `SRV.capacity_lot`. A buyer with no stall left in
reach goes without, kept for the day (`RetailDay::unserved`), and the re-choice rounds are counted. A service's stall
is what its maker can make that day: the lesser of its staff's output rate and its plant's capacity, no more than its held inputs let its way make (`way_most`), in whole units. A firm buys the services its way uses at retail as the whole units its period's output needs.
- **Freight** (FRT.1–FRT.9, GEO.4, GEO.13): the network is generated with the map (`phx_geo::network`): road and rail
  segments between the market zones of every two regions of a country that share a border, over their land path,
  and sea lanes joining a country's parts, the shortest first; each mode's capacity a day in tonnes. A carriage kind
  (`phx_market::carriage::FreightKind`) names the kinds that carry, a carrier's mode and price facts and a compile of
  its technology. A shipper's want (`ShipIntent`) is admitted at 5d, its goods covered and weighed; at 6a each origin
  and mode meets: carriers found in one pass, their room their vehicles' units (the chain of transport equipment) times
  a unit's tonne-km a day, and shippers taken in an order drawn by lot, each at the cheapest carrier with room for its
  out-and-back trip, a route crossing a full segment refused. At 6d each booking owes the carrier whole lots of
  carriage for its tonne-km; paid in stage 7, the goods are pledged to the carrier and become a shipment in transit,
  saved with the goods, due after its run at the mode's speed and loading at each end. At 5a of its day the lien is
  released and the goods move, used up where they left and made where they arrive, at the cost that left with them;
  GDS's family counts them shipped and arrived. A segment with an end in a zone a catastrophe struck carries nothing
  that day.
- **The goods' markets** (GDS.7): `sys-gds` declares two kinds, `GDS.commodities` — a call at each place each business
  day, for the products `GDS.standardised` names — and `GDS.between_firms`, posted prices, for the rest. The opening
  marks every good its places trade at `GDS.opening_price`, its world price, times its price level in the country it
  stands in (`GDS.price_level`) (`World::snapshot_markets`): each good held at the opening, and each grade class of what
  each deposit yields where its goods stand.
- **Extraction** (GDS.4, GEO.9, GEO.12): the holder of a deposit's right visits on its own schedule
  (`GDS.extraction_days`, 5c). A deposit's grade is its opening grade times e^(−κ·share taken) (`GDS.grade_fall`), the
  richest part first, classed by `GDS.grade_bounds`. It works the deposit when today's mark less its unit cost is at
  least its method's outlook less that cost discounted at its required return over the days to its next decision
  (Hotelling; a deposit without end whenever the mark covers the cost), its output rate for those days in whole units,
  no more than the deposit holds, as a transformation from the deposit, what it takes gathering until it holds a lot;
  and offers what it then holds at no less than the better of its cost and its discounted outlook. Where its goods'
  place has no mark or outlook it sells delivered (FRT.5): its price is the best mark it can carry them to less the
  freight out, it expects that to hold, and what it holds is shipped there. A deposit in another region is priced by
  the mark where its goods stand and its goods are offered there, as are all goods it holds at other places.
- **Spoilage** (GDS.8): a system declares each product's yearly loss in stock and the visit it follows
  (`SpoilageDecl`; `sys-gds`'s stock visit at 5b every `GDS.spoilage_days` calendar days, deciding nothing). After it
  the kernel (`phx-world`'s `spoil`) takes from each lot 1 − e^(−rate·t) of its units, t the days it was held within
  the period, summed and rounded half to even: one instruction a row of transformation legs under
  `spoiled`. A loss to nature — spoiled, struck by a hazard or worn — takes the units held whatever an offer or a
  pledge binds of them (`Source::physical_loss`), and a sale its units covered then fails for want of them. Storage,
  the room goods are kept in, is a service bought, never the same number.

A route is the shortest path of one mode over the segments (`Network::route`); a trip over two modes is two bookings.
An arrival does not wait on its carrier: goods aboard a carrier that failed meanwhile still arrive, their owner's.
Carriage pays under `FRT carried` and arrivals move under `FRT arrived`; the meeting's order is drawn from
`FRT.capacity_lot`, equal prices by lot.
- **Carriers and shippers** (FRT.4, FRT.5): at the opening each firm selling the carriage product is given a mode,
  drawn by the modes' shares of carriage employment (`FRT.mode_share`, stream `FRT.opening`). With the marks at 6a the
  world rebuilds an `AwayTable`: each good's mark at each region's market zone, leaving out a place whose market last
  met with no seller (`Tape::unsold`, from the failures on the tape, which shows none to buy from); and at each origin
  and mode the lowest price carriage is posted at by a carrier with room — a posted-price market's buyer compares the
  posted prices it can see (MKT.6), which carriage needs before its first sale can print. A handler reads it through
  `Ctx::away` (`phx_core::Away`): for its row's zone, each other place in its country that marks the good, by each
  mode carriage is posted in at either end that joins the two, with that mode's route length between them
  (`Network::lengths`, every mode's shortest routes computed once at assembly, since a mode that joins no path between
  two places carries nothing there), the freight of a lot out from here
  and in from there (`phx_market::carriage::freight`: its tonnes times the km, in units of carriage at the lowest posted
  price where it leaves). The shipper's visit (`FRT.ship_*`, every `FRT.shipping_days` at 5c) carries the whole lots it
  holds beyond its planning days' expected sales to the place whose price less its own and the freight out is widest,
  when that is positive. A buyer who bought at another place ships it home (FRT.5): an order or a shipment may name its
  place (`OrderIntent::at`, `ShipIntent::from`). What arrives is what is left of the goods, as goods on their way
  spoil: a loss to nature that leaves a holding with fewer units than are pledged of it cuts the pledges by the excess,
  the latest first (`Liens::shrink`), so a lien pledges units that exist, and an arrival moves what its lien still
  holds. An owner that ended on the way is its successor. The goods a handler reads at its own place
  (`Ctx::held`, `Ctx::goods`) are its free units — what it holds less what is pledged to a carrier or committed to an
  order — so a firm never plans to use goods already on their way.
- **Merchants** (GDS.6): the firms making `GDS.merchant_product` visit every `GDS.merchant_days` at 5c and, for each
  standardised good their place marks, weigh the price their method expects over the visit's days, less what spoils
  and discounted at their required return, against today's: they bid, at no more than that, with the money they hold
  beyond what their own planned sales cost, and when carrying no longer pays offer what they hold at no less.
- **Closure** (FRM.15): at its production schedule an owner weighs the margin it expects a year, held for ever at its
  required return, against what winding it down returns beyond the money it keeps either way (`Ctx::net_assets`: its
  goods at cost less its debts at their balances; plant returns nothing until a market buys it); when
  winding down is worth more it emits
  `FRM.close`, and the world ends
  the firm into an estate as a default does, counted as a closure.
- **Catastrophes** (GEO.8, GDS.9): a struck tile's share of every physical unit its sited individuals hold is lost
  (`Source::Hazard`); an agent stands at a zone, so it loses the event's share of its zone — each struck tile's
  thousandths over the zone's tiles — in whole units, of the plant it holds and the goods it holds there.
- **The goods' balance** (GDS.10): a leg's digest names the source of a transformation's units and, on an instrument,
  what was issued of it before the leg, so the audit keeps per instrument the day moved (`StockDay`) its opening and
  what moved it: paired legs, unpaired legs no transformation accounts for, and each source's units made and used
  up. `GDS.goods` reads it for every good (`BooksAudit::good`): a way or a deposit makes, a way, a purchase, spoilage
  or a hazard uses up, trades make nothing, and the opening plus what was made less what was used up, spoiled and
  destroyed, with what arrived less what was shipped, is what is in existence at the close.

### 7.12 Labour

- **Employment lines** (LAB.1, REP.3): a contract's terms carry, beside its monthly wage leg, a **class**
  (`Terms::class`, by the places the employment kind declares): occupation family, skill, hours, notice days,
  severance days a year of service, region and start band. Contracts differing in any are on different lines, so a
  line is what LAB.1 says it is. The pension's kind and rates join the class with PEN (S4.04); until then a job carries
  none.
- **A person's labour state** (PTY.3): three person attributes, declared by LAB on the household kind: its state
  (`LAB.state`: not searching, searching, retired), the occupation family it last worked in (`LAB.occupation`, none
  for one never employed) and the wage point of its last job (`LAB.last_point`, at the opening the point of the wage it
  would have earned; none for one never counted in work). A hire sets the occupation and point and stops the search; a
  layoff or a quit sets it searching. Employment is its
  attachments on employment lines; nothing else counts it.
- **The labour kind** (`if_labour::LabourKind`, declared by `sys-lab` as a market beside the goods kinds): the
  employment line kind's name, the class's places, the attributes above, and its **decision points** — the employer's
  posting and layoffs, its selection among applicants and its renegotiation offer; the searcher's applications, its
  acceptance, its answer to a renegotiation, its quitting and its retiring; the union's offer — each an
  `if_labour` input and output with its rule in `sys-lab`, dispatched through the player's queue for the player's
  household (OBS.4). The kernel builds each input from the world and applies each output; no rule reads the world.
- **The labour book** (`phx-world`'s `labour`, saved): vacancies (employer, region, class, wage point, jobs open,
  posted day), each region and occupation's list of them; applications (vacancy, applicant agent and person,
  skill and experience, sent day), which live until answered; offers (vacancy, applicant, day made), which live a day;
  hires and separations due at the next 4a, with notice; and each employer's last fill by occupation, keyed by the two (the
  point and days its last vacancy took to fill).
- **The day** (TIME.10, a round a day): at **5c**, after the visits, in order — offers made yesterday answered by
  their applicants; applications sent yesterday seen by their employers at the meeting hazard and selected by skill,
  then experience, then lot, up to the jobs open; the searchers' applications to the vacancies standing at the start of the day; and the employers whose
  production schedule came due today posting, withdrawing and laying off. The searchers are read in
  `SEARCH_SHARDS` fixed shards on the pool, each with its own taste stream, and their applications entered in the
  searchers' order; the player's agent chooses from its queue after the others are read. At **4a** of the next day, the hires join
  their lines and the separations whose notice has run leave them, their severance paid at stage 7 as legs of the
  separation's instruction. A match therefore takes at least three days.
- **Wage points** (REP.34): the monthly wage at point _n_ is `LAB.wage_point_ratio` to the _n_-th power, 128 points, the last meaning none; `sys-lab`'s `wages` holds the arithmetic the kernel calls (`wage_at`, `point_near`, `least_point`, the offer `adapt` and the severance `owed`).
- **Posting** (LAB.4, `rules::post`): on its production schedule an employer reads its planned output a day — its expected sales over its production period (`FRM.production_days`), within its plant's capacity (`CAP.capacity`) — and, per occupation family, its hours a unit — its own hours a unit (`FRM.hours_a_unit`, its productivity, fixed by its filed accounts as its staff's hours over its filed output) shared by its way's mix of occupations (`TEC.labour`) — its staff's hours less those under notice, its open vacancies' hours and an hour's wage at the point it would offer. Its staff's hours make its output together, as production counts them, so it weighs them together: when they exceed what its planned output needs by a whole job it lays off the surplus in whole jobs from the occupations furthest over their part of the way's mix and withdraws its vacancies; otherwise it posts the whole jobs its planned output still needs beyond its staff and vacancies, in the occupations furthest short of their part, where an hour's output at its price outlook — its price over the way's hours a unit, since an hour of any occupation adds its share of a unit — is worth more than that occupation's wage plus financing it until the sale and than the law's least, and withdraws the vacancies an occupation no longer needs. Laying off an occupation beyond the way's mix while the firm is short of hours would drop work production uses (LAB.4: it lays off when sure it cannot use the work). It decides nothing while its facts give it no price or planned output. An employer agent's persons employed are kept in its `FRM.size` attribute as each of its decisions finds them. The point it offers is its last fill's in the occupation, one lower when that fill came in the least days a match takes, else its newest staff's there, else the mean wage's for the hours, never below the law's least; a vacancy standing past `LAB.vacancy_patience_days` is raised a point.
- **Vacancies** are posted and withdrawn in the jobs the employer's rule asks for, one person a job.
- **Layoffs** are the jobs the rule asks for, the lines they leave drawn by their members (stream `LAB.layoff`), never more than a line holds; the employees are drawn from the line (REP.23) and the separation takes effect on the first business day of the employer's country by which its notice has run.
- **Search and acceptance** (LAB.5, LAB.8, REP.22): each searching person not waiting on an application or an offer sees the vacancies standing in its region and occupation, at a skill it has, with jobs open; it draws a Gumbel taste for each (`LAB.match_taste`) and applies to the best by `LAB.wage_weight` times the wage's log plus taste, among those paying above its reservation, a round's share of `LAB.applications_a_week` — a draw without replacement from the logit. An application is seen at `LAB.seen_chance` (`LAB.meeting`). An offer is accepted when the weight times the log of the wage over the reservation, plus the match's taste, is positive. The reservation is `LAB.reservation_share` of the searcher's last wage (a placeholder naming HH); an employee's price outlook at a review is its household's stance's outlook of its country's consumer index, compounded to the next review.
- **Retirement** (LAB.6) is a scheduled hazard on persons (`LAB.retirement`, at `SOC.pension_age` by age and sex): on the first birthday an adult not retired has reached its pension's age, the `retire` decision point (a placeholder rule naming HH: retire at that age) sets it retired; at the next 4a its attachments on employment lines are taken off it and its members leave each line with as many of its employers', and it claims the state pension (§10.3).
- **Severance** owed on a separation is the class's days a year for each whole year since the line's band began, of the line's daily wage, per member leaving. At 7c each employer's day's severance is one instruction: all of it where its money covers it, else each member the same share of what it holds, the rest counted unpaid.

- **Joining a line** (`Books::members_join`): a hire's members join the employee's row and as many join the
  employer's, in one instruction, the rows opened where they do not exist; the hired person gains its attachment.
  Leaving is `members_leave` (§7.4), drawing among the employee side's holders by their counts (REP.23) where the
  employer's members are not named.
- **Searchers** are the agents whose persons search, kept as an index by region and occupation, rebuilt on load from
  the attributes, so a day's rounds visit only them.
- **Reviews** (LAB.17, LAB.9): an employer's pay round, once a review period at a phase it draws the first time its
  schedule comes due (the book keeps each employer's next review day). At 5c its rule offers each of its lines a
  point; at the next 4a the line's employees are drawn from its side (REP.23) and each answers the least point it
  stays for; the two conclude in two moves, and a changed wage is a move of the members to that point's line
  (`members_leave`, then `members_join`, the persons' attachments following), one who quits leaving to search.
- **A failed firm's staff** (LAB.12): its estate's employment lines leave through the separation path before the
  estate settles, their severance paid first.

### 7.13 Credit

- **The credit kind** (`if_credit::CreditKind`, declared by `sys-bnk` as a market beside the goods and labour kinds):
  the firms' term loan line kind, the lenders' kind, the reason loans are written under, the streams of the lenders a
  borrower asks and its tastes over them, each country's law, and the rules — a borrower's class by its cover, a
  bank's quote, decline and standard, a class's learned frequency, a borrower's choice. The kernel builds each input
  and applies each output; no rule reads the world.
- **The credit book** (`phx-world`'s `credit`, saved): applications (borrower, bank, the loan refinanced, principal,
  months, day sent), quotes (the same with the rate and day made), the loans taken, to write at 7c, and each bank's
  learning — its standard, loan-years and defaults by class, losses realised, each loan's class at its last review
  and each defaulted loan's balance. Each loan's maturity is an index rebuilt on load from the lines' schedules.
- **The day** (TIME.10, a round a day): at **5c**, after labour's round, in order — the quotes made yesterday chosen
  among; the applications sent yesterday quoted or declined; and the borrowers whose loans fall due within their lead
  applying. On the first round of a month each bank reviews its book. At **7c** each loan taken is written by
  `Books::lend`: the two rows opened on a line of its own, the principal owed on them, and the principal paid into
  the borrower's account — a deposit the lender creates, or reserves paid to the bank that keeps it (MON.6).
- **Losses**: an estate's settlement returns each debt it wrote off, which the bank that held it learns from.
- **The rules** (`sys-bnk`'s `credit`, BNK.4, BNK.5, BNK.20): a borrower's class is the number of `BNK.cover_bounds` its interest cover reaches — its period's income to date grossed up to a year plus the interest its term loans charge, over that interest; none before a day of its period has passed or where it owes no interest, which is the worst class. A class's default frequency is `BNK.default_rates`' counted as `BNK.prior_loan_years` of the bank's own book, with the loan-years and defaults its book has seen; its loss given default likewise `BNK.loss_given_default` counted as `BNK.prior_recoveries`, with the shares of defaulted balances its write-offs lost. A quote is the rate it can earn instead (the country's policy rate at the opening; a placeholder naming BFL) plus the expected loss plus `BNK.risk_weight` × `BNK.capital_requirement` × `BNK.required_return` plus the loan's cost (`BNK.loan_cost_share` of output per person) over principal and years, rounded up to `BNK.rate_step`. A bank declines a class worse than its standard, or a loan its net assets cannot carry at the capital requirement over its risk-weighted firm loans; it begins at every class whose published frequency is below one, and each monthly review moves the standard a class tighter when its book's defaults cost more than the published frequencies priced, a class looser when less.
- **Shopping** (BNK.6, BNK.19): a firm applies once, `BNK.refinance_lead_days` before its loan's maturity, for the balance and months again, to its own bank and further banks of its country, how many drawn from `BNK.lenders_asked`' shares; it takes the quote whose rate less its taste for the lender in rate steps (`BNK.lender_taste`) is lowest, if its rate is below the firm's required return. The loan written is a line of its own at the quoted rate, interest monthly on the balance from the day written and the principal at the end.


### 7.14 The central bank's fund stage

On the core (`core_central`):
- **The facilities** are two families of overnight contracts, `CB.deposit_facility` (the central bank owing a bank)
  and `CB.lending_facility` (a bank owing its central bank), each position a row reckoned from terms at its
  facility's rate, so the loan books, the accounts and a bank's balance sheet read it as any loan.
- **The fund stage** runs after settlement on each country's business days (TIME.6's 8): every position is returned
  with its interest over the days it stood; on a month's first stage the central bank's net interest since the last
  is remitted to its treasury's account, a loss kept against its equity (CB.10); then each bank's chief executive asks
  the facilities (`BNK.request`, through the decision core) from its reserves after the returns. The stage's flows —
  returns first in each bank's payment order, then what it opens — settle together through the settlement kernel with
  the issuer on one side, which never fails; a return that fails leaves its position standing, overdue.
- **Intraday credit** is each bank's reserve account's facility (MON.3): as much as every bank of its country held in
  reserves at the opening. Reserves left below nothing after the stage are the central bank's overdue claim, counted
  each day with the banks that owe it (BFL.10's liquidity failure; the bank's resolution is Stage 2's).
- **The request** (a placeholder naming BFL): a bank's target is its reserves over its deposits at its first fund
  stage, times its deposits now; it places what it holds above the target and borrows what it lacks as far as its
  collateral lends — its firm loans' balances less `CB.loan_haircut`.
- **The corridor** (placeholders naming CB): the policy rate at the opening less `CB.deposit_spread` and plus
  `CB.lending_spread`.
- **The opening's central-bank loans** — the sheet's `CENTRAL_BANK_LOANS` — are shared over each country's banks by
  their weights and opened as lending-facility positions, returned and asked anew at the first fund stage.
- **The issuers' record** (MON.7, MON.9): the core keeps what the issuers owe as money by class — reserves, the
  treasuries' accounts, banknotes (the accounts held at the issuer by anyone else) — moved by every settled flow, the
  payer's class less and the payee's more, the issuer's own side moving none; at each close it is held to what the
  accounts show, a difference a finding of the money family. The parties' money is held to what the banks and the
  issuers paid them.
- The run report's `fund_stages` gives each country's stage a day: placed, borrowed, the banks doing each, what stood
  overdue, the positions unreturned, what was remitted; LC-1-27 reads it with the money family.

### 7.15 Statistics

Each country's agency (`sys-sta`, bound as the state's `StaKind`, with `sys-idx`'s `IndexKind`) is the state's book
`StatsBook`: what it has collected of each period still to be published or chained on, its releases, and the tape's
match sets read. At 10a it samples the day's sales from the tape by keyed draws (`STA.sample`, each return's arrival
from `STA.returns`), so a sale's inclusion is the same whenever it is read; at 3e a panel household's deaths and
onsets are recorded as its outcomes make them; at a month's last close one pass over the household agents counts the
panel's persons and labour force, and one sweep of the holder tables' money rows reads the banks' reports. Releases
are computed only on their calendar's days, and readers read them through `latest`, which returns nothing before a
release's day (STA.4).

**The national accounts** (STA.3, series `STA.accounts`): at 10a the agency reads the day's settled money from the
ledger's day book, each effect carrying its reason and whether it is money or the cost units carried. Each instruction
in which a firm received revenue is a firm's record, drawn by its instruction for the seller's country: the revenue
is output, and each other party's payment in it a purchase by a firm, a firm's purchase of plant (`CAP bought`), a
household's consumption, or anyone else's; a seller's own payment in its sale is a levy on it. Every firm's and
household's income — its revenue and expense effects other than wear, and the interest and wages it earned or paid —
is counted in full, as are firms' stocks of goods at cost at each census. A release publishes production (output less
purchases plus the change in stocks), expenditure (consumption, investment and others' purchases plus the change in
stocks), income, and the discrepancy of expenditure over income.

**Series and vintages**: six monthly series (`if_state::consts::NAMES`: consumer and producer indices, the labour force survey, the money stock, the period life table, the national accounts), each released on its calendar's business day of the month its lag names. A period's first release (vintage 0) reads the returns in by then, each return's arrival drawn at `STA.early_returns`; its revision (vintage 1) `STA.revision_months` later reads every return; both are kept. `sys_sta::latest` returns the latest vintage of the latest period published by the reader's day.
**The indices** (`sys-idx`): each sampled sale kept at `STA.sample_share` by its keyed draw; a period's link weights each product's unit-value relative by its share of the period before's sampled spending among the products priced in both, and chains on the level last published for the period before; the first period priced opens at `IDX.base`.
**The life table and the survey**: a panel of households drawn by `STA.sample`; a period's exposure is the mean of the panel's persons, by age class (`STA.age_classes`) and health, at its first and last census, times the days between; each entry carries its events, person-days and rate a year, each estimate the panel's count over its sample share and, for a first release, over the share of returns in. The labour force counts the employed, the searching unemployed and those out of the labour force. **Money**: the banks' reserves and deposits by holder class (household, company, the rest).

### 7.16 The state's first cut

Four systems declare kinds the world binds (`if-state`'s `kinds`), kept in `phx-world`'s `state` (saved):
- **The benefit** (`sys-soc`, SOC.3): a person laid off or released by a failed firm claims when the benefit's monthly
  amount (its replacement share of the wage lost) times its months is worth more than the claiming hours at its last
  wage; it joins its country's benefit line of that amount — opened the first time a claim needs it, paid monthly for
  the benefit's months — with as many of the treasury's members. A hire ends it. A person who retires claims the state
  pension (§10.3).
- **Bills** (`sys-sov`, SOV; on the core `core_bills`): each country's auction runs in its fund stage on its declared
  weekday, before the facilities' requests. The minister sizes it (`SOV.size`, through the decision core; the
  placeholder plan naming TRS): the face that keeps the treasury's cash at `SOV.buffer_weeks` of its last week's
  outflow — its cash after the last auction's proceeds less now — after the week's maturities, in whole bills. Each
  bank's chief executive bids (`SOV.bid`) its reserves above its target at the price whose yield over a bill's weeks
  is the deposit facility's rate; the uniform-price clearing takes bids from the highest price down, the last price's
  sharing what is left pro rata in whole bills, and what is not sold is not issued. A bill is a contract of the
  `SOV.bills` family from the treasury to its holder, its balance the price paid, reckoned at the yield its price
  gives and paid in full at maturity, so a treasury short of cash falls into arrears on it. At the opening each bank
  holds the government paper its sheet gives it as bills maturing one tranche a week over a bill's weeks at the
  deposit facility's rate; the central bank's paper waits for its purchases (CB.5) and the households' for their
  portfolios. Each auction's offer, bids, sales, price, cover and tail are kept (the run report's `auctions`,
  LC-1-31); each issue is held to what its holders paid while it stands (LC-0-16); and at each close the debt family
  holds the bills outstanding to what was issued less what was redeemed and written off (TRS.6, LC-1-29).
- **Public agencies** (`sys-soc`'s kind `agency`, legal form `public agency`; on the core `core_agencies`, SOC.2,
  SOC.8): each country has one, producing its public administration, which is no product and is paid by taxes. At the
  opening it takes each occupation's public administration share of its jobs in every region
  (`SOC.public_staff_share`, derived from the ILO's employment by activity), and every job no firm's way takes (the
  armed forces'); the rest are dealt to firms. Its head buys the state's final uses at retail (`SOC.consume`, what the
  treasury bought before) and keeps its staff: each business day it posts what it lacks of its opening staff by region
  and occupation, as far as its appropriation for wages pays (`SOC.staff`; the appropriation is its opening staff's
  wages a month, a placeholder naming POL until the budget votes it); its vacancies are met, selected by the head of
  its service and raised when they stand, as any employer's, and a person it hires leaves the job it held, whoever the
  employer. Its account is at the issuer beside the treasury's, the state's money, and its treasury funds it each day
  for what it pays beyond what it holds (a flow of `FUNDED`), so a treasury short of cash leaves its agency's wages and
  purchases unpaid. The national accounts value its output at its staff's wages: by production and expenditure they
  are added, by income they are wages, not taken off the firms' surplus. Its staff's pay rounds wait for public pay
  scales (POL).
- **Taxes' collectors on the core** (`core_taxes`, TAX.2, TAX.5): a tax arises on its base's payment once it settles —
  income tax withheld by the employer from the wage it pays (the wage flow carries the net), consumption tax in a
  final sale's price charged by the seller — and is then its collector's debt to its treasury, a contract of the
  `TAX.collected` family reckoned from terms (its balance paid in full on `TAX.remit_day` of the month after, asked
  again each month while unpaid, a claim in its collector's estate); the collector's tax is entered when it arises.
  The treasury paying its own staff collects to itself. At each close what arose is held to what was remitted, what
  collectors owe and what their debts owed when they closed (each family's `lost`), a difference a finding of the
  taxes family (LC-1-28). The income tax withheld from sampled households' wages is kept with its gross wage, and
  LC-0-29 recomputes it band by band under its rounding.
- **The payment order** (`sys-trs`, TRS.10; on the core `order_payments`): each treasury's payments are ranked by
  `TRS.payment_order` — its debt service (its bills' redemptions), its pensions, its benefits, its public staff's
  wages and its public purchases — carried on each flow's order, so a treasury short of cash fails the last of them
  first (§6.5); its agency's funding is two flows, one for the day's wages and one for the rest, each at its rank,
  and the agency's own payments take the same ranks.

### 7.17 The core

**The committed world is the core's** (S1.24). `phx_world::World` holds the calendar, register and streams, each
system's compiled state, and `core::Core`; assembly compiles the declarations, generates the map and draws the core's
own opening (`core_open`); a turn runs the core's day (`day.rs`): chance on the persons, labour's round, goods, then
dues and settlement. The observer (`phx_obs`) reads the core's days, households and families; the live checks read
the world through `Inspector`. The sections below that describe the books, the agents' tables, visits, the old
markets and the old audit describe code that no longer runs and are restated in S1.25.

The restructure (the plan's S1.17–S1.25) moves the world onto a core laid out for the finished world at five million
persons. What exists of it, beside the kernel above until the world moves (S1.23, S1.24):

- **References.** `PartyRef` (`phx-id`): a party's kind, its slot's generation (24 bits) and its slot, one word, for what
  outlives a day (records, messages). `PartyKey`: kind (5 bits) and slot (27 bits) in 32 bits, for what lives within a
  day or moves with its party — the day's flows and a contract's two sides — since a contract's side moves when its
  party ends and a slot is reused only after the close.
- **Parties** (`phx_store::Parties`): a kind's slots, allocated lowest first and released at the close (`SlotAlloc`),
  and each slot's generation, raised as the slot takes a new party; `resolve` refuses a stale reference.
- **Contracts** (`phx_store::EdgeTable`): a family's contracts, each a row (`Row`) of its two named parties beside the
  words the family reads on the day's paths (amount, balance, next due, shape of terms), so a contract is read in one
  line; a family with no words keeps `Pair`s. A side that keeps its parties' lists threads each party's contracts
  through `next` and `prev` columns from a head its kind keeps, so opening, closing and moving a side (`move_end`) take
  the same time however many a party holds; a broken link or a head handed in for a side that keeps none stops the
  run. A family's colder words are columns of its own, made by `EdgeTable::column` at the table's capacity;
  `Column::put` writes an existing row or the next and refuses a gap. A contract closed today frees its slot only
  after the close, so a family's capacity holds its contracts and a day's openings; its reader marks it closed in its
  own words.
- **The due wheel** (`phx_core::wheel::DueWheel`): a bucket a day over a horizon, reused round the wheel, and the dues
  beyond it, read every half horizon for those come within reach. A day's dues move on together (`schedule_all`), in
  slot order, so a bucket is mostly one sorted run; a take sorts only what follows that run (radix on the pool) and
  merges it in, and the contracts due are read in slot order. A contract whose due moves or that closes is not taken
  out; its reader skips a stale entry by the contract's own next due.
- **Flows** (`phx_core::flows`): every movement names payer and payee, amount, source, reason, denomination (money of a
  currency or units of a declared unit, never netted together) and payment order, 24 bytes, appended to the buffer of
  the chunk that makes it (`FlowBufs`). A handler makes one through its context (`Ctx::flow`) for a reason it
  declares (`pays: [...]`): a `ReasonDef` carries the reason's code, which says the lines it posts to, and the payment
  order its flows keep; paying an undeclared reason does not compile, and a negative amount stops the run. `Ranges`
  cuts each kind's slots into ranges of `2^range_bits`. A buffer groups itself (`FlowBufs::group`): each chunk puts its
  flows of the settled denomination in its payers' ranges' order in place, the rest last, and makes credits (payee,
  amount, reason, 16 bytes) in its payees' ranges' order, chunks on the pool; within a range a chunk's flows keep an
  order of its own making, which is enough, since a payer's ties fall by lot. `Grouped` is the day's view: each range's
  slices across the chunks, and a place for every flow in the day's order, range after range. Nothing is copied: the
  heaviest day's 21 million flows cost 40 bytes each, where a global partition cost 64.
- **Settlement** (`phx_core::settle`), the ledger's 7b on the core, over a currency's `Books`: each money kind's
  accounts (the bank each is held at, balance, pending commitments, what is held through closed banks, the facility,
  and cash lines if the kind keeps them), the banks' kind, whose balances are their reserves, each bank's deposits, the
  banks closed and the issuer.
  - Pass one, range by range on the pool, adds the commitments to the balances, nets every flow into its parties' nets,
    sums what each bank's customers pay out of it and are paid into it, which its reserves move by, and posts each flow
    to its sides' cash lines (ACC.9); the parties short, banks aside, come out of the same pass.
  - Flows through a closed bank are held, and count against the payer's funds from the next day on (SET.2).
  - The worklist runs in rounds. Each short party decides, range by range on the pool and from the round's state, the
    suffix of its flows in payment order it cannot pay — ties among one payer's flows of one order drawn by lot from a
    stream the caller opens for the payer — gathered from a per-range index by payer built once a day for the ranges
    shorts fall in. The round's failures are then taken off range by range on the pool, payer sides where decided and
    payee sides grouped by range, each job's bank moves summed after. A failure only takes from others, so deciding on
    the round's state keeps the greatest set that can settle, rings included; what a round misses the next finds.
  - A bank still short once its customers are done loses every flow through it (MON.5), the bank's to answer for.
  - The nets are added in place, each bank's deposits move with its customers' nets, and the failed and held flows
    are taken back from the cash lines. A closed day's flows are committed (`commit`): netted into pending, the banks'
    moves into theirs, posted, nothing failed.
  - `Books::deposit_breaks` is the money family's identity: each bank owes what its customers hold.
  - Settlement's state keeps no flow (PC-27): the outcome hands back the failed, with why, and the held.
- **Dues by shape** (`phx_ledger::algebra::shape`): terms split into a shape, the terms with each amount a contract
  fixes for itself taken out, shared by every contract of that kind of terms, and the contract's own amounts. A shape's
  dues on a date are planned once (`shape_plan`), and a contract's due is its amounts and balance through the plan
  (`due_by_shape`), with the algebra's roundings; a leg waiting on an event or an election leaves the plan general,
  reckoned whole from `terms_of`.
- **A handler's facts**: `RecordFacts` over its chunk's party records, row-major — each party one record of `stride`
  words, a fact at its declared offset — so a visit to a sparse row reads a line or two, not a line a fact; and
  `ColumnFacts` over column slices for a pass over dense rows. Both read and write by a fact's place in the handler's
  declaration, an indexed load, inlined across crates.
- **Draws** (`phx_rand::Draws`) build an address's counter once, so a block is one Philox call and its words are read
  inline; an address holds 2^24 blocks, so bulk work takes an address a chunk. `Draws::x4` makes four addresses' first
  blocks at once from one block index (`philox_x4`, the four chains' rounds side by side), each as `from_block` would.
- **The posted-price meeting** (`phx_market::meet`), one product's stalls and buyers, round by round:
  - each place weighs its sellers with units left once a round, an alias table for buyers of units and the same
    sellers by price with running sums for buyers with money, whose affordable sellers are a prefix;
  - each buyer still choosing draws its seller from its own stream at the round's block (`ROUND_BLOCKS` a round), four
    buyers at a time;
  - buyers are grouped by the stall they chose, a stable radix of 8 bits a pass (`partition_into`), each record
    carrying what its service reads (48 bytes), since reading it from the buyer list at random cost more than moving
    it;
  - stalls are served a chunk a job, each its buyers in place: all of them when its units reach, otherwise one at a
    time drawn by lot from their order by who they are, so who is served does not depend on the order buyers came in;
    those served short choose again;
  - the caller keeps a `Meeting` from day to day, whose outcome (sales by chunk of stalls, the unserved, the rounds)
    and working space a day's meeting writes into, so it allocates nothing once the heaviest day has sized it.
- **Goods** (`phx_core::goods`):
  - A good is a product's grade at a zone (GDS.1). `GoodIds` issues each good the declared unit its flows carry the
    first time something names it, within `Denom::units`' 2^15.
  - `Stocks` holds each party's holdings, a row a good (40 bytes, and a link), threaded per party from a head its kind
    keeps, so a row names no party: units, their cost at average cost (ACC.6), the day they came in averaged by units,
    and what is committed to sales and pledged to carriers; the rest is free.
    - `receive` takes units in at a cost. `deliver` takes them from a bound (free, committed, pledged) at their share
      of the cost, rounded half to even, the last unit taking what is left; asking beyond the bound is `Short`, a
      fail (SET.3).
    - `bind` moves units between bounds: a sale's cover, a pledge.
    - `lose` is a loss to nature: it takes units whatever binds them, then cuts the pledges by what the bound units now
      exceed, so a cover left beyond the units fails its sale.
    - A party ends holding nothing (PTY.9).
  - A transformation is a flow with `NATURE` on the side a counterparty would stand: making, use, spoilage, a loss,
    a shipment's leaving and arriving. What accounts for it is its source. Nature's kind is the last of the 32
    (`phx_id::consts::NATURE_KIND`), and `Parties::new` refuses a table of it. `Stocks::apply` applies a flow of units: the payer delivers from a bound, and the payee receives at a
    price paid, a making's cost, or what the units carried out.
  - `Shipments` are goods on their way (FRT.3): owner, carrier, the good leaving and the good arriving, units, day of
    arrival. Each is on its owner's list and in a due wheel's bucket.
    - `depart` pledges the units.
    - `arrive` takes each day's in turn: the pledge is used up where it left and made where it arrives at the cost
      and the day in it carried, as goods age on the way, a pair of transformation flows whose source is the shipment.
    - `shrink` cuts an owner's shipments of a good by what its pledges lost, latest first.
  - `spoil` reckons a holder's spoilage over a period (`spoilage::lost` on the holding's mean day), each a flow to
    nature.
  - The goods' identity (GDS.10): each good's units at the close are its units at the open plus what nature gave
    less what it took (`nature_net`, `breaks`); flows between parties move units without changing how many there are.
  - A sale's goods leg (`GoodsLeg`) goes to the buyer, who holds it as a firm holds its inputs, or to nature with the
    buyer as its source, as a household's purchase uses it up.
- **Capital** (`phx_core::units`): plant, dwellings and vehicles are held by zone and class (REP.24) — kind, band of
  size or quality, condition — as counts in their owners' holdings in `Stocks`, each class a declared unit of the same
  registry as goods (`UnitIds` over `Held`), at average cost, the holding's mean day their service day. So a unit of
  plant is bought, pledged, lost and moved as goods are, and the goods' identity covers capital.
  - A kind's `Chain` declares its yearly rate of leaving a condition and each condition's efficiency and value;
    `issue_chain` issues a kind's conditions at a zone together.
  - `wear` takes from each class, on what the holder held before any moved, the units a constant hazard takes over
    the days (`wear::leaving`), moves them to the next condition at their cost times its value over their own
    (`wear::carried`) with their service day kept, and the last condition's leave: a pair of flows with nature a move,
    the leaving alone for a unit leaving the chain.
  - `capacity` is a holder's units of a kind, each at its condition's efficiency, at a zone or wherever they stand;
    a way's capacity is the least over its kinds of that over what it needs a unit (CAP.9).
- **The posted meeting between firms** (`phx_market::between`): the sellers' offers are sorted once into price levels,
  each level's offers with units left leading it; the buyers come in an order drawn by lot, each step of a buyer's,
  its highest limit first, taking from the cheapest level at or below its limit, the level's offers in an order drawn
  by lot for the step, in quantities whole in both parties' lots. A level sold out is passed by every later step. The
  sales are the posted-price meeting's `Sale`s, money pro rata to the price's lot, and make their flows as those do.
- **Calls** (`phx_market::call`, `coupled_call`) meet as before over their orders sorted by price point. Their
  grouping by priority rank, by party and by node, side and price is over sorted buffers, in the order the maps gave.
- **Labour matching** (`phx_market::hiring`), a step a day:
  - `Standing` groups the open vacancies by region and occupation, each group by the skill it asks, least first, so a
    seeker's reach is a prefix of its group.
  - `search`: each seeker sends a round's share of a week's applications (the whole part, one more at the chance of
    the rest). It draws them one by one without replacement from its own stream, among the vacancies in reach paying
    above its reservation, with chances in proportion to wage^w. That is the logit over w·ln(wage) with a Gumbel taste
    per vacancy (Plackett–Luce), the distribution the old rule's taste per vacancy ranked, without a draw per vacancy.
    Seekers are drawn in chunks on the pool, joined in their order.
  - `select`: yesterday's applications are met at a chance, grouped by vacancy in the order sent, and handed with their
    lots to the employer's rule, which offers no more than the jobs open.
  - `answer`: each person answers its offers together; the best-paid it accepts is its hire, ties to the vacancy
    listed first, and every other offer's job returns.
  - The hires go on to become employment contracts where the world applies them.
- **Pay rounds** (`phx_world::core_labour`, LAB.17, LAB.6, LAB.9): an employer's first round falls on a day drawn in
  the review period after its first production schedule (stream `LAB.review`), then once a period (`CoreLabour::
  reviews`). On its schedule's day each contract not under notice is offered `rules::renegotiate::review` over the
  market's point (`offer_point`) and the most the job's month pays — its wage and the unit's margin at the price the
  firm expects (its stance's outlook of its mark, or its own price before the mark prints) over its cost of making a
  unit (`unit_cost_of`), for each unit a month of the job's hours makes. The offers wait (`CoreLabour::offered`) until
  the employers' rounds are done, then are answered together: one `search` per country over the reviewed employees,
  each as a seeker whose reservation is its own wage, gives the vacancies each sees; each answers by
  `rules::renegotiate::answer` from its reservation, the best it saw and its household's price outlook compounded to
  the next round. Where the work cannot pay its counter, an offer below its reservation (`LAB.reservation_share` of
  its wage) sends it to search, its contract closed; any other leaves it at the offer, its seen vacancies sent as
  applications with the day's (`on_the_job`); a hire of a person who holds a job closes that job (a job-to-job move).
- **Outlooks** (`phx_world::core_outlooks`, VAL.3–VAL.7, VAL.13, VAL.23): a public series is keyed by a code and a
  place — each product's mark in a region for the firms (`CoreGoods::outlooks`), each country's consumer index for
  the households (`CoreStats::outlooks`, its monthly change printed on its release day). A print scores and forms, for
  every memory type, every heuristic of the menu (`phx_val::types::Types`, compiled once for firms and households):
  its error in the method's width enters the heuristic's performance, and each forms its outlook of the next print;
  the anchor's level is the mean of the prints since the opening. A party's memory type, switching type and stance are
  words of its record (the firm's `MEMORY`, `SWITCHING`, `STANCE`; the household's `HH.memory`, `HH.switching`,
  `HH.stance`), drawn at the opening, the first stance by taste alone; a firm reconsiders its stance at each price
  review, a household on each spending occasion (`Outlooks::reconsider`, stream `FRM.stance` or `HH.stance`). Each
  method's lag behind a series' turns is counted in prints (`Outlooks::lags`).
- **Attention** (`phx_world::core_goods`, REP.38, REP.35, VAL.14): after the day's meetings every firm looks at its
  sales since its review (`SOLD` less `SEEN_SOLD`) against those it expects, the surprise entering its sales' width at
  its memory type's gain (`SALES_WIDTH`), and draws from `FRM.visits` whether it reviews its price today at
  `rules::attention::review_chance` — its daily revenue, its markup, the variances of its own sales and of its
  stance's outlook of its product's mark, each relative to its level, and a review's cost in its staff's hours. A
  firm with no width yet, or expecting no sales, reviews on its production schedule. A surprise wider than
  `VAL.attention_sensitivity` widths, its own or its stance's, is kept (`Outlooks::awaiting`) until the firm's next
  price change, which records the days it took by the surprise's size over what was expected (`responses`).
- **Firms' endings** (`phx_world::core_default`, FRM.15, FRM.11, L3, PTY.9): `Insolvency` keeps each country's grace,
  the day each contract in arrears began to be (from the day's failed flows, forgotten once paid or closed), each
  estate's claims and the firms ended. A firm in arrears past its grace defaults at the start of the labour round; a
  solvent one's owner winds it down on its production schedule (`winds_down`). `end_firm` opens an estate with its
  money, passes its goods, closes every contract it is party to into claims (`claims_of`: employees' wages owed and
  severance at the first rank, other creditors at the second), sends its staff to search, zeroes its vacancies and ends
  the party. `estates_pay` pays each rank in proportion as far as the money goes (`shares`), the rest to the heirless
  destination; an estate holding goods stays until they are sold. A household's estate holds its debts as claims
  (`debts_of`).
- **Deliveries** (`phx_world::core_goods`, GDS.2, Law 5): a meeting's sale covers its units at its seller (`bind` to
  committed) and waits (`CoreGoods::deliveries`); after the day's settlement `deliver_sales` delivers each paid sale
  from the cover at what its buyer paid — to the buyer, or to nature for a service or a purchase used as delivered —
  releases each whose money failed (a service's released capacity perishing), and reads the goods' identity over them.
- **Accounts** (`phx_world::core_accounts`, ACC.4, ACC.10, FRM.13): `Accounts` keeps each firm's and bank's equity
  account at the opening (`open_accounts`, from `net_assets`) and its `Income` by line since. `recognise` enters an
  event on its line: deliveries (revenue, cost of sales, services used), spoilage and perished capacity (goods lost),
  a service's inputs used (cost of sales), the day's settled flows by reason (`account_flows`: wages, severance and
  taxes paid; a loan payment as interest), and the families' moves (`account_moves`: principal repaid taken back out
  of interest, arrears' changes accrued, balances written off). `audit_accounts` holds each equity account to
  `net_assets` — money, goods at cost and what it is owed, less what it owes and a bank's deposits — and the revenue
  recognised to the money received for sales.
- **Lending** (`phx_world::core_lending`, BNK.4–BNK.6, BNK.20): `Credit` keeps each country's `if_credit::law::Law`,
  each firm's filed earnings a year at the opening, and each bank's `Lender` (standard, applications, declines, quotes,
  loans). `apply_for_loan` draws the banks a firm asks (`BNK.lenders_asked`), reads its `cover`, and has each bank
  decline or quote by `sys_bnk::credit`; the firm chooses by its tastes (`BNK.lender_taste`). `lend_shortfalls` lends
  the shortfall at the chosen quote's rate from the chosen bank, or nothing.
- **Loan books** (`phx_world::core_books`, BNK.11): each dated family records its moves on its creditors' books
  (`LoanMoves`: lent, principal repaid in `dues`, balances written off by `close_contract`); `book_loans` enters them
  at the day's close and holds each creditor's `LoanBook` to the balances its loan contracts owe it.
- **The world's stores** (`phx_core::store`): a kind (`KindStore`) keeps its `Parties`, each party's record of `stride`
  words in one column, and, if it holds money, its accounts (bank, balance, pending, held, facility) and cash lines,
  each a column indexed by slot, so a party begun in a released slot writes its own words over the ended one's; a
  kind of money requires an account at `begin` and any other refuses one. `books` makes settlement's `Books` from the
  kinds, and `deposits_of` sums what each bank owes. A family (`Family`) keeps its `EdgeTable`, each listed side's heads,
  one a party of its kind's capacity, and its `DueWheel`: `open` threads a contract on its sides' lists and on the wheel
  at its first due and refuses a side of another kind; `close` takes it off its lists and leaves its wheel entry for
  its reader to skip. The world and the full-load bench build on them.
- **Persons** (`phx_pop::persons::Persons`): each household's persons on the core, two words each — its word as its
  kind packs it (birth date, role, attributes: the word is full) and its identity (`Held`) — a list per household in
  its chunk's arena and found by its slot; `set`, `push`, `remove` (the rest keep their order) and `clear` keep the
  count held, `place_of` finds a person by its identity, and `compact_due` closes a chunk's gaps in slot order once
  its dead words pass their share. A person's identity is drawn from the same counter as the parties' (a person is a
  party, PTY.1): each is handed the next as its household begins, a newborn the next after.
- **The core's opening** (`phx_world::core_open`): the core draws its own world, no copy of anything. Its kinds are
  declared in their order (`consts::kinds`: central bank, treasury, bank, firm, estate, household), a kind's number its
  place. Each country's central bank and treasury share a site drawn by CB's; its banks are as many as the Zipf law
  fitted to their concentration gives (`sys_bnk::bank_weights`), each sited by BNK's draw. Its households are drawn
  region by region by DEM (`sys_dem::draw_country`), and each household's adults' labour (`sys_lab::Rule::draw`), its
  banking (`sys_bnk::households::Banking::draw`) and its pensioners (`sys_soc::Pensions::draw`) by their systems' pure
  rules, each keyed by the household's subject; a rule is built from the register alone (`Register::handle` finds a
  declared primitive's handle). The draws' attributes are written to the persons and the household, the household is
  begun with its positions as its kind opens them (its income a year as the wages and pensions drawn times the
  year's months), and each person takes the next identity. Accounts come from the country's balance sheet: the banks'
  reserves by their shares and the treasury's deposits at the issuer, the households' deposits over those that bank
  by their wealth at their bank, the households' currency over those that bank nowhere by their persons at the issuer.
  The pensions are opened at once from the treasury, naming the person; the jobs and the households' loans are kept
  (`core_open::Drawn`) until the firms are drawn, then dealt (`core_jobs`) and apportioned the sheet's loans to
  households by income (`core_credit`). `Core::key` finds a party's key by its identity. The balance sheet itself
  (`opening::sheet`) closes each country's sectors (the central bank's currency and reserves and lends banks the remainder, the banks' reserves and equity at their ratios and bonds held
  by households balancing them, deposits beyond lending held as more paper, firms' equity their assets less debts).
  A sheet the closures cannot balance, or one that breaks an identity, is refused at assembly.
- **Firms of one kind** (`core_firms`): the firm kind's store is begun with every country's firms. A country's firms
  are its employed — its people from 15 at its employment rate — times its firms per employed, shared over the products by the persons each
  product's opening output employs by its way over a full-time year, and over the regions by the persons the core's
  households hold there. A firm's record is its product, region, site tile, productivity (billionths of a log point,
  drawn from the group's spread), its posted price of a lot and its output a year in units. Its productivity and site
  each draw from their own subject under the firms' opening stream; its bank is drawn by the banks' deposits.
- **Day zero's prices and output** (`core_firms::Snapshot`): a product's unit at the opening costs its way's inputs at
  the opening prices plus its labour at the accounts' compensation over output; its markup is the opening price over
  that cost. A firm's price is that markup over its own cost, its labour fewer hours by its productivity's factor,
  posted at the nearest of its management's price points. The accounts' units a year of each product are shared over
  the regions its firms are in by their persons, and within a region over its firms by the retail logit's price term
  (`SRV.price_weight`); the distance term joins when the meetings' places are on the core. The sheet's firm deposits
  are shared over the firms by their turnover at those prices; the turnover is 0.93–0.96 of the accounts' output
  value, the cheaper firms selling more of the units (seed 1, 150,000 persons).
- **Jobs on the core** (`core_jobs`): each job the books hold on an employment line is read once a line (its wage,
  schedule and next date, region and occupation) and dealt to a firm in its region: a region's jobs of an occupation
  are shared over its firms by largest remainder (`deal`) on the hours their output takes of the occupation (their
  way's hours a unit, fewer by their productivity, `sys_frm::rules::way::own_hours`, times their output), in an order
  drawn by lot under the firms' opening stream. Each is a contract in the family `LAB.employment` from firm to
  household, naming the person by its identity and paying the wage on the job's dates. Jobs no firm's way in their
  region takes (the armed forces') are the state's, contracts from their country's treasury in `LAB.public_employment`,
  until the public agencies hold their staff.
- **Labour on the core** (`core_labour`): each country's level by occupation is its opening staff's hours in it over
  the hours its ways ask of it at the opening output, so a firm's hours a unit of an occupation are its way's, fewer by
  its productivity, at that level. Each firm decides on its production schedule (a phase drawn per firm) by the
  labour kind's posting rule; its vacancies stand in `CoreLabour`, searchers are an index of (household, person)
  kept at events, and the round is `phx_market::hiring`'s search, select and answer, a day apart each. A hire is a
  contract in `LAB.employment` on its country's monthly dates, its class (occupation, hours, start year) held per
  schedule. Layoffs are drawn by lot among the firm's jobs in the occupation, given the law's notice; at its end the
  contract closes, severance for the years served is a flow from firm to household, and the person searches again.
  A person whose hazard outcome retires it leaves its jobs; every changed person is written back.
- **Goods on the core** (`core_goods`): a firm's record holds its stock; each day it grows by the planned output's
  day in whole units counted from the core's first day, within what its staff's hours make at its hours a unit.
  Households decide on their spending schedule by the buffer-stock rule, their income outlook, what they received
  and the month counted in their record's positions, and ask each product's budget share. Each product's retail
  meeting (`phx_market::meet`) has a place per region with its firms at no distance; each sale is a flow in the
  day's pending flows and its units leave the seller's stock.
- **Hot modules** (PC-92): no map or trait object in the core's parties, edges, stores, partition, flows, wheel, settlement,
  facts, goods, capital, posted-price meetings or labour matching.
- **Measure**: the phone's time is the CPU time of every thread, spinning workers' included (`process_cpu_ns`), over
  its three sustained cores, never below the wall; page faults per day stand for allocation during the day.

**The full-load bench** (`phx-ffi`'s `load`, `perf/load/volumes.toml`) builds the finished world at the design point
— every kind's parties with records and money, twelve contract families with their list links and dues — and runs a
month through these kernels:
- hazards, and handlers over the day's agenda by phase, spending their rules' declared arithmetic;
- purchases at the posted-price meeting, a product's buyers against its sellers' stalls (price, a good's free stock or
  a service's 1.3 times the product's demand spread evenly, a few km from their region's buyers), products side by
  side, one kept `Meeting` a worker, each sale a money flow and, for a good, a goods leg covering the seller's units;
  a firm buys a good between firms instead, region by region, at a limit drawn for it;
- goods (`load_goods.rs`), in stocks sharded by the firms' netting ranges, each range's worker writing its own:
  - each goods firm holds its product and two inputs at the opening;
  - after settlement each covered sale is delivered, used up by a household or held by a firm at what it paid, or
    released where its payment failed;
  - every firm holds four conditions of its product's plant, worn at its monthly review;
  - each goods firm uses up a unit of each input it holds and makes a day's sales, no more than its plant can;
  - a third of the products spoil, weekly, and a hundredth of the goods firms ship a quarter of their stock to the next
    region, arriving three days on;
  - the goods' identity is read every day;
- a labour round (`load_labour.rs`) on business days: a book of a vacancy for every three of the day's searchers, about
  3% of persons searching, each day's offers answered, applications met and selected, and applications drawn, a
  filled vacancy posted again;
- the wheel's dues, monthly and quarterly, the quarterly through the far list, each reckoned through its family's shape
  of terms (a fixed sum, interest, an annuity, a principal once) from its contract's row, a row a few dues ahead asked
  for early;
- contracts closed and opened, family by family on the pool, and a tenth as many parties ended and begun;
- settlement: accounts at banks drawn for them, reserves a tenth of deposits, balances spread over four decades, cash
  lines of what each party paid and received, closed days' flows committed;
- the audit: reserves conserved, each bank owing what its customers hold, and a rolling thirtieth of the contracts;
- full saves.

Its day's work is three halves of the finished world's estimate (owner, 2026-09-28). Unit costs are CPU time over
units, the rules' declared arithmetic taken off handler rows. On the build machine at S1.20's end (2026-09-28), a
flow grouped, netted, posted and applied cost 86 core-ns, a due 179, a handler row 127, a purchase 159; a business day
failed about 3% of its flows, since the bench's random economy drifts short over its month; the peak was 4 427 MiB,
928 bytes a person. At S1.21's end (2026-09-28), with the markets, goods, plant and the labour round in the month:
- a purchase cost 340 core-ns, with 1.75 choices a buyer, since the bench's service stalls hold even units and the
  logit crowds the cheap and near. By itself one product's meeting at 370 000 buyers costs about 250 ns a buyer on one
  core: draws and alias picks about 45 ns a choice, grouping by stall 40, service 25, the tables 12;
- the goods about 250 ms a business day, the labour round about 130 ms;
- a median turn of 2.4 s and a heavy day of 3.0 s here;
- a peak of 5 186 MiB, 1 087 bytes a person.

These numbers compare kernels with their targets; the budget is judged on the phone (N8.8, owner 2026-09-28: "the
target is on phone benchmark"). The old kernel's bench gave 7.7 s, 29 s and 9 GB at a smaller world.

### 7.18 The decision core

Every decision the world takes runs through one call, `Core::decide`, taken each time the decision is (MND.20,
Appendix E 50). It exists before anyone holds an office, so persons and minds attach to it later (S3.05, Stage 8)
without a decision moving.

- **Decision kinds.** A system declares each of its decisions as a decision point (§4.7) in its `points.rs`: a name,
  its input and output types and its rule, one pure function. The register declares the concern list (MND.2) and, for
  each point by name, who takes it and how (`MND.decisions`, a SHAPE primitive of MND.19): the office it is taken in,
  or `household` (its adults as one) or `person` (a person's own); the concerns it touches; and its mode, `best` or
  `satisfice` (MND.7). At assembly `DecisionKinds` is built from the register and held to the points the systems
  declared, both ways: a declared point the register does not list, a listed decision no system declares, a concern
  not on the list or an office no legal form declares is refused.
- **The decisions taken** (25 at S1.26): in a firm's line head's office its first price, what it makes, the inputs it
  orders, its attention, its price review and move, its stance, its posting (with its lay-offs), its selection among
  applicants and its offers at pay rounds; in its chief executive's, winding down, its choice among loan quotes and its
  investment; in a bank's loan officer's, declining and quoting, and in its chief executive's, its standard; in a
  treasury's minister's, its consumption; a household's spending, stance and trying for a child; a person's search,
  answer to an offer, answer at a pay round, retirement and benefit claim. Inside a decision some parts are drawn
  rather than chosen: whom a lay-off falls on by lot, a buyer's seller in the meeting by its taste from its own
  stream, a posted wage's point by the fill history's arithmetic; each is the decision's own execution, counted with
  it.
- **Offices.** Each legal form lists the offices it decides through (`PTY.legal_forms`, PTY.16): a company its chief
  executive and the head of its line, a bank its chief executive and its loan officers, a treasury its minister, a
  central bank its governor. The core's `Offices` holds, per institution kind, each institution's founding preferences
  by its slot, drawn at its opening (MND.16), and the offices' holders, a map from (institution, office) to a person,
  empty until the processes that fill offices exist (S3.05); a person holding several offices, as an owner managing its
  own firm, is several entries naming one person.
- **Preferences** (`phx_core::decisions::Prefs`): what a decider brings to a rule before minds — its memory type, its
  switching type and its stance on the heuristics' menu (VAL.6, VAL.7), the return it requires (FRM.15) and its
  management type (FRM.5's speeds and curvature, one declared type while no source measures their spread); each
  `Missing` where the decider holds none. A firm's are its founding preferences, moved out of its record; a household's
  are its record's outlook attributes (VAL.23); a person's are its household's until S8.01 draws each person's.
- **Resolution.** `decide(point, party, input)` binds the point's kind once per pass (`bind`, an index, no lookup by
  name on the path), then for each party names the decider: the player where it keeps the decision (S1.24 h seats the
  player on the core), else the office's holder, else the institution's founding preferences; a household's or a
  person's decision is its own. It hands the input builder the decider's preferences — a rule reads an institution's
  preferences only there — calls the rule on the input, and counts the decision by its kind and the decider's standing
  (player, holder, founding preferences, household, person), counts any worker adds to (`phx_exec::Tally`, an integer
  sum the same on any pool) that the run report's `decisions` and LC-1-52 read. A decision whose taker is an office
  the party's form does not declare stops the run (MND.20).
- **A decider's outlooks change with its decisions.** A reconsidered stance is written back to where the decider's
  preferences live: the founding preferences for an office no one holds, the household's record for a household.
- **Processes' decisions.** A decision taken inside a population process (retiring, trying for a child) reads its
  decider through the process view's `decider`, which the core supplies and counts; the rule is then called as any
  other's.
- **Search as a decision.** The hiring kernel's search hands each searcher's reach — each vacancy above its reservation
  with its pull — and its draws to a `choose` the core supplies, which takes LAB.search; the rule draws without
  replacement in proportion to the pulls (`phx_market::hiring::pick`). The kernel's selection hands `choose` the
  vacancy, so the employer's LAB.select names its decider.
- **Cost.** A decision's resolution is an index into the kinds, one into the party kind's offices, an empty-map check,
  the founding row and a count: a few nanoseconds beside the rule's own arithmetic. A point is bound by name once a
  pass, never per party.

## 8. Markets, valuation and expectations

- `phx-market` implements each form once (MKT.3–MKT.8): call auctions (with admission hooks); the continuous book
  (arrival by lot, price–time priority, closing auction); the dealer market; posted prices with
  rationing by lot; bilateral quotes as a protocol over messages across days; administered facilities.
- **The call** (`call::call`) finds its price on the tick grid among the prices that clear — something trades and every
  step strictly better than the price fits within what the other side brings at it, so those fill in full — chosen by
  the operator's tie sequence on the `MarketDecl` (`TieRule`: most volume, least imbalance, nearest the last print,
  skipped with none, then the lower price, which leaves no tie). Steps at the price share the rest pro rata by largest
  remainder with ties by lot, or by declared priority, pro rata within a priority. No overlap, no bid or no offer is a
  failure. **The book** keeps each side in priority order, better price then earlier arrival, a resting step placed by
  binary search; an incoming step trades at each resting step's price, passing over its poster's own; what rests and
  the `AtTheClose` orders meet in the closing call. **A dealer request** trades on the best quote among the dealers
  asked, within the client's limit and the quote's size, equal quotes by lot; no dealer quoting that side is the
  dealers' failure, every quote beyond the limit the client's walking away. **The administered form** meets requests
  in their parties' order, each as far as the facility's registered quantity response grants, within its limit. **A
  linked call** drops the market nodes with no order that day, with their edges, before it meets. `posted::posted`
  meets groups of buyers, whose demand at a price it reads through `phx_core::GroupDemand`, so `phx-market` depends on
  no population crate; a group turned away chooses again among the sellers left, wanting at the new price what its
  members want there less what they bought.

- **The linked call** (MKT.3 over a network; the money market and the central bank's tenders at 8b): the coupled
  call with a cost on an edge and a capacity on a node. Each borrower's collateral is assigned to segments before the
  call at those segments' lenders' haircuts, so every capacity is cash on integer lots and the call stays a pure
  min-cost flow, solved by network simplex warm-started from the last basis, one core per country. The basis is
  state, saved with the market's store (§11): where optima tie, the flow depends on the start, so a restored world
  continues exactly only from the saved basis (SET.15).
- **Calls over networks** share one engine: the network simplex over a min-cost circulation, surplus first and volume
  second, whose arcs are keyed by node, kind and step rank so a saved basis survives schedules that move; each market
  node's price is chosen within its optimal potentials' range by the call's tie rules, node by node, each choice
  narrowing the ranges after it. Zones' matches run through the lines' owners, who keep the congestion rent.
- **Recording**: `phx-market`'s `Markets` keeps the tape (match sets, prints, fixings, marks, failures), each linked
  call's basis and each day's measures; it alone makes a print, from a meeting's matches. A posted meeting's buyer is
  a party, as its match set records sales per buyer. `Reach` is the one set of counterparties a search
  reaches, and the one reader of the border closure.

  The audit's `MKT.prices` family (MKT.13, MKT.14) reads the day's tape through `MarketsAudit` every close: each print
  traced to its match set and trading together what it says at its price, each mark from a print or a fixing its
  market published.
- Marks and fixings at 6b by each form's rule (MKT.12): an instrument's or a currency pair's fixing is made by
  `phx-market` by the pricing service's declared method (the volume-weighted mean of the day's trades, for dealer
  markets); with no trade there is none. **Benchmark reference rates** (IDX.2) are a different fact with one writer:
  each administrator's 9d handler in `sys-idx` fixes them from 8b's match sets, the only input a `BenchmarkFixing` can
  be built from (the plan's PC-56); a day without eligible matches has none. The curve and the day's discount-factor
  tables are fitted at 6c from the day's fixings by the curve publisher, labelled valuation inputs (MKT.20), never
  prints. Valuations (MKT.20) in `phx-acct` at 9a, as `Money` rounded by the valuer's convention, by valuers whose
  methods are registered by their systems and read only prints and valuation inputs; each point is labelled traded,
  interpolated or, beyond the longest traded point, extrapolated by the valuer's own declared method. Valuers discount
  by the day's discount-factor table and never evaluate an exponential per flow.
- **The accounts** (`phx_acct::Accounts`, saved in the `accounts` store) are two records kept apart (ACC.10): each
  equity account, in its party's site's country's currency, opened at the opening's equity for every party of a kind
  whose legal form has owners and moved only by the events posted from the day's book; and the party's assets less
  its liabilities, read when asked (`net_assets`) from the register and the contracts at carrying values. The period
  is the calendar month (`period_of`): a posting in a new month closes the last, each account's opening and closing
  kept with the period's income and capital tallied apart for `ACC.periods`. The close stops the run if a due or
  effect was recorded after the day's posting. Accounts, claims and tallies are ordered maps keyed by party.
- **Carrying bases** (ACC.2, ACC.17): `ACC.carrying_bases` lists, per legal form and purpose (collect, trade, sell,
  use), the bases the standard permits in its order. A position whose holder has not chosen is held for its
  instrument family's purpose (a contract or debt to collect, a share or fund unit to trade, a real asset to use) and
  carried on the first basis listed; a debt owed at amortised cost. A carrying value whose input is absent is
  absent, and so is the party's `net_assets`, reported unreadable. On fair value a mark's move is income the day it
  happens; on any other basis it stays an unrealised difference until a sale. A write-down's reversal stops at
  original cost. A consolidated statement combines the members a group fact names, eliminates claims between them
  from both sides and shows the minority's part, storing nothing.
- **The Accounts families**: `ACC.equity` (ACC.10) rolling over a 30-day cycle, `ACC.periods` (ACC.11) on each period
  closed, `ACC.claims` (ACC.12) every close.

- **Admission hooks** (DRV.6, §4.7) take a member's whole order set at a meeting and allot its headroom across the
  orders in canonical order, so no arrival order decides; on a continuous book they run in the book's lot-drawn
  arrival sequence, carrying the headroom used as state.
- `phx-val` computes public-series outlooks per method per day from public records, which start empty on day zero
  (GEN.5); an outlook with no series yet starts from the closest one observed (VAL.10).
  For **registered** series — instrument prices — it computes them only for (method, instrument) pairs some holder
  or candidate list registers (registered at applies by keyed reduction, §4.8), at 5a on days with a new print,
  with each pair's value (closed-form claim or firm values) shared by every party using the method. A party's own
  outlooks are updated at its visits, an individual's of a registered series as its method's plus its own deviation,
  caught up in O(1); a **surprise** (VAL.4) wakes it and raises its attention (REP.35).

`phx-val` is pure functions in `f64` through `libm`, never over the world: an `Outlook` stores its mean and width as
`Fixed<6>` (`outlook::fixed`, a result no `i64` holds stopping the run), and a `Value` becomes only `Money`, never a
print. The heuristic menu is one sealed trait (`Heuristic`, `MENU`: adaptive, trend, anchor, announcement), so no
other crate adds a way of forecasting; assembly refuses `VAL.heuristics_tracked` other than the menu's length. The
world's methods are compiled at assembly, every heuristic on the menu by every memory type (a type of
`VAL.adaptive_gain`'s distribution, its λ), a method's index its heuristic times the memory types plus its memory
type; γ and κ are shared, and the age window is none. The public series (`PublicSeries`) keeps its last and previous
print, their running sum and count and each method's outlook; an anchor's level is the mean of its prints.
Attention is λ_k = g_k·sqrt(σ²_own + σ²_pub), g_k = ½·sqrt(ψ_k ÷ c_k), its daily chance −expm1(−λ_k), and the
exposure over spans of constant public variance one term a span (`attention::exposure`); switching shares are the
logit of each heuristic's exponentially weighted squared error in the method's widths, equal while none is scored;
experience weighting is (L − k)^θ over the annual means of the years lived.

---

## 9. Endings, estates and resolution

### 9.1 Endings

| Ending | Handled by | How |
| --- | --- | --- |
| A person's death | `sys-dem` | The person leaves the household and its rows (§7.4); if the household ends, an estate row in `phx-core`'s estate table, behaviour in `sys-est` (at Stage 0, `phx_ledger::estate` and `phx-world`'s `estates`, below) |
| Leaving home, separation, formation | `sys-dem` | Not an ending: `divide` makes a new household from persons of one by the family law, `combine` one from persons of two; the origins continue, and a household ends only with its last person |
| Household, firm, fund or political-party estate | `sys-est` | Sells what its debts need; pays by the country's law through the ledger's waterfall (L3), as instructions settled at stage 7; passes the rest in kind (POP.9) |
| Personal insolvency | `sys-hh` | The procedure (HH.21): an estate row sells the non-exempt assets, distributes and ends; for the procedure's period the income levy's follow-on pays the creditors' claim line through the country's trustee; discharge ends the claims |
| Fund | `sys-fnd`, then `sys-est` | A redemption unpaid on its date opens the fund's own procedure (dealing suspended, unpaid redemptions a claim); it ends when its assets fall below what it owes its lenders or its units reach zero, into one estate that sells into markets and pays its lenders, then its unit holders (L3) |
| Finance company, dealer desk | `sys-frm`, then `sys-est` | As a firm: it cannot pay, or its liabilities exceed its assets; its one estate sells its loans or inventory, and a desk's parent loses its shares and its line's shortfall |
| Bank, insurer | `sys-sup` | Resolution (§9.2): an insurer's book to an acquirer or run off, the protection scheme paying to its limit; the rest to an estate |
| Pension scheme | `sys-pen` | Underfunded and repaired while its sponsor lives; on the sponsor's end, the scheme's claim on its estate, then transfer to the guarantee fund, each member's right split at the cap per member, the rest a claim on the scheme's estate |
| Clearing member, clearing house | `sys-drv`, then `sys-sup` | Close-out and porting (§9.3); the house's waterfall; beyond it, recovery by the rulebook, then the service transferred or wound down (§9.2) |
| Public agency | `sys-soc` | Its duties and staff pass to a successor agency named by the budget |
| Sovereign | `sys-trs` | Default and exchange offer |

Estate rows are short-lived individuals, **one per ended party**, holding its count on every line and holding it
succeeds to (§7.4). The ended party's identity names its
estate as successor (PTY.10), so a trade or payment in flight that names it settles with the estate; and its units
pass with what binds them — covers of offers not yet settled, as a weekend's sales are, and pledges — which stay under
its name and count on the estate (`phx_ledger::succession`: `Ledger::bound`, and every cover and pledge placed through
`Ledger::cover` and `Ledger::pledge`), a succession forgotten at the close once nothing is bound under the old name. How many are open follows Little's
law, the rate of openings times their life: firms' about 800 a day × about 40 days, households' about 1 000 a day ×
about 25 days, personal insolvencies' about 70 a day × about 45 days — about 60 thousand open, at 512 bytes each
(§13.1). Their mean life and the number open are counted per kind.

The waterfall (`phx_ledger::waterfall`) is pure and runs one currency at a time, never converting: each secured claim is
paid first from its own collateral's proceeds, a surplus joining the general proceeds and a shortfall joining the
claim's class; then the classes in the country's declared order, each paid in full while the proceeds last and the
first they do not cover pro rata, the division's residue landing on the claim the law names for that class and
currency; what is left is the owners'.

As built there is no `sys-est`: estates are the ledger's `phx_ledger::estate` run by the
world's `estates` (`phx-world/src/estates.rs`). An ended household's rows pass to one estate at 3e; from the next
business day, in 7c's block once the day's dues are paid, the ledger settles it (`Books::settle_estate`): its rows that
hold nothing but money leave with their counterparts first; its money pays its debts through the waterfall, its
creditors as one class until the insolvency law's order of classes (S2.03); and while it still holds units it waits to sell them, counted, writing
nothing off and distributing nothing. One that holds none writes off what it owes beyond, pays what is left to the
party of the kind `HEIRLESS_DESTINATION` names (the treasury) in its country, since no heir is drawn before the kinship
lines exist (S2.04), and its rows leave with their counterparts and it ends; one whose payment fails waits,
counted. Dues it
owes in arrears are not claims in its waterfall until S2.04. A catastrophe's struck tiles lose, at 3b, their share of every
physical unit the individuals sited there hold, as a transformation naming the event (GEO.8).

Firms end in **default of payment** (FRM.15): a kind is put under its country's insolvency law by an
`InsolvencyDecl` naming the grace (`FRM.insolvency_grace_days`, each country's own). The contract process's fails that leave a party of
such a kind in arrears queue the day its grace ends (`World::defaults`, rebuilt from the arrears when a save loads);
at 2e a party whose contract is still in arrears at that day ends: a large firm into an estate that takes every row
by line transfers and every holding (`pass_holdings`), a small firm's agent as a household's does (`end_agent`). Its
equity account closes as it ends (`Accounts::close`): what it held is its estate's. The estate liquidates from the
next business day as any other: its rows leave with their counterparts — its tenants' tenancies and its staff's jobs
end (REP.23), drawn from the households' sides through their holder lists as §7.4 says. Restructuring, the law's
other way, waits for the creditors' decisions (S3); balance-sheet insolvency for the valuation of firms' books.

### 9.2 A bank's failure

1. **Day D, 8f**: the bank cannot repay intraday credit. The shortfall becomes an overdue claim of the central bank on
   the bank, recorded on both books (MON.12); its liquidity failure is recorded (BFL.10). An insolvency found at 9c
   (SUP.5) starts the same path.
2. **D, 9c**: `sys-sup`'s test triggers resolution in the same handler and invites bids by message, applied at 9e.
   From then until its transfer settles the bank is **closed**: it makes no payment of its own, and every leg to or
   from its customers is **pending** (§6.5) — their outgoing payments wait on their rows, payments to them wait on the
   payer's row, settling when the rows reach their receiving bank.
3. **D+1 (next business day), 2b**: the authority takes the book from the bank's **statement of D** (its loans valued
   at 9a over per-(line, arrears stage) totals, MKT.20), so no loan row is valued again; it writes down equity,
   converts or writes down contingent capital and subordinated debt, then senior debt as needed. **5c**: eligible
   banks bid, each valuing the assets under its own assessment (TIME.6), and the insurer chooses the paying bank it
   would use with no acquirer (a decision point). **6a**: the authority selects by its own **least-cost rule**, not a
   market form: the highest bid at or above its reserve — the insurer's cost of paying the insured deposits out — wins
   and pays its bid, ties by lot; with none, the payout path.
4. **D+1, 7**: the transfer settles as one instruction. Each deposit row splits at a kink (§4.4): the insured amount
   and its pending payments move to the acquirer — or, on the payout path, to the paying bank the insurer chose — and
   the rest becomes a claim line on the estate; pending payments beyond the insured amount fail, visibly, against
   the estate claim (MON.5). The **consideration** is a leg of the same instruction: the acquirer
   takes the assets it bid for, and the estate or the insurer pays the difference to the insured deposits it assumed
   (SUP.6). The insurer becomes the estate's creditor; a short fund draws its treasury backstop. The bank's reserve
   account passes to its estate. SUP.7's identity is an audit family on the instruction. At 7e `sys-bfl` writes the
   depositors' new banking arrangements in place (§4.5).
5. **D+2**: customers pay through their receiving bank; their pending payments settle there.

If the transfer fails at D+1's 7b (a leg its payer cannot fund), nothing moves (SET.4) and the bank stays closed; at
D+2's 2b the authority takes the next bid at or above its reserve, or the payout path, and the transfer settles at
D+2's 7, customers paying through their receiving bank from D+3.

**Insurers and clearing houses** (SUP.14) follow the same days. A closed insurer's claims and benefits are pending;
on each many-party line it writes with others, the holders whose credits are its own are drawn once, at D+1's 2b, by
one scan of the holders' arenas; at D+1's 7 its side of each line passes to the acquirer, and where the protection
limit is below a benefit those holders' rows split at the limit per contract, or the book is run off by its estate with
the protection scheme paying the protected shortfall. A clearing house whose waterfall is exhausted at D's 9c is
closed from 9e; at D+1's 2b its rulebook's recovery haircuts the pending variation-margin gains it owes and marks its
unmatched positions for tear-up at 7; if that does not cover the loss, other houses bid for its service at 5c, the
least-cost rule selects at 6a, and at 7 its members' positions and margin move to the winner or are wound down at the
last settlement prices.

### 9.3 A margin call unmet

The call is issued at 9c of day D and due at D+1's 2c (TIME.7). Unmet, the member's default is recorded at 2d, the
member suspended and its lots announced; at 5c surviving members bid for the lots and for its clients' positions
(DRV.9), and the house posts its reserve per lot; they form at 6a and settle at 7; at 9a the close-out is valued; at
9c the house runs its waterfall; at D+2's 2e the losses land on named holders. The defaulting member's own failure is
its kind's: a bank member's follows §9.2, and other members' their estates.

---

## 10. The opening world (GEN)

### 10.0 The setup

A world starts from a setup (spec GEN.14, GEN.15, Appendix E 43), so a new game is one screen or none:

- **World constants**, fixed for the simulation: the total population, the map, three countries, 25 regions, the
  settling year. The budget depends on the total population alone, so no setup can break it (N8).
- **Choices**, per new game: the population split, each country between 10% and 70%; per country six three-level
  choices — development, public debt, private debt, risk appetite, inequality, openness — and a name, real or
  generated. Each has a default or is drawn from the stream `GEN.setup`; a setup outside the guardrails is refused.
- **Derivation**: the development level draws one joint profile of about twenty derived values from its country
  group's published profile (the World Bank's income groups; each value on a scale that keeps it in its domain, the
  group's robust location, dispersion and correlations), perturbed by the seed within its dispersion; each other
  choice's level pins its own values to its third of the group's distribution, drawn within it, and the rest of the
  profile is drawn conditional on them, so values that go together stay together and nothing is clamped. The derived
  values are opening state, recorded in the run's directory, not primitives. Each country's primitives are
  instantiated in the run's directory from its level's templates, and from its derived values by each system's
  mappings: a system names a country primitive a derived value sets (`SetupValue`), written with the country's data
  as an estimated value, so a process reads it from the register — the country's life expectancy, which mortality's
  life table is solved to when the process is bound; its opening distributions and present values follow by declared mappings and accounting
  identities, never an equilibrium solve (GEN.4).
- **Land and regions** follow the split: the 25 regions are allotted by largest remainder with at least three per
  country, and each country's land is its share of the map, so regions are of like size. A country's persons and its
  small firms are apportioned over its regions by their land, the one weight the map gives before the dwelling stock
  places them.
- **The world's persons split once**: the persons the representation holds are split among the countries by the
  setup's shares, and each country's over its regions by their land (REP.40); what the split's rounding leaves is
  reported.
- **Names**: a real name labels the country's institutions and currency and pre-fills its choices; its economy is
  always derived. A generated name comes from the stream `GEN.names`.
- The setup is named by every realism report (GEN.11). A save's manifest does not record it: its register hash is
  over the data files, the countries the setup instantiated among them, so a load with another setup is refused (§11).

**The population's opening distributions** are derived offline by `tools/data/fetch_pop.py` and `derive_pop.py` into
each level's `gen/` files. Each entry is a register value, with its source and mapping in `source_ref`, and is listed in
`data/inventory.toml`. Each value is its group's standard: the median over the group's economies of each one's latest
observation. Where fewer than ten of the group's economies report a value, the median of the ten reporting economies
nearest the group's GDP per head stands in. The mapping says how a country's derived values move the standard:
- survival is given as the standard's Brass logits, with the level solved when mortality is bound (§7.3);
- households by type and size are log ratios on the drawn fertility, with one Theil–Sen slope and each group's
  intercept;
- income and wealth are each a log-normal body with a Pareto top above its 90th percentile, the exponent being the
  group's. The body's sigma is solved to the country's drawn Gini (income) or top tenth's share (wealth), and the
  result is scaled to the country's mean.

The scripts give the same tables when rerun on the same fetched files. A new game copies a level's `gen/` file only for
a system the world registers, since the register refuses an entry that no system declares.

The profiles are derived by `tools/data/derive.py` from the series `tools/data/fetch.py` keeps in `data/sources/raw/`,
each value's latest observation per country (2015–2025): the development levels are the World Bank's income groups
(high income developed, upper-middle emerging, lower-middle and low developing); each value is put on its `Transform` —
logs for positive amounts, logits for percentages and shares, the log growth factor for inflation, log ratios to
services for the sector shares, so the three shares follow by identity — and its group's profile is the median, the
interquartile range over 1.349, and Spearman correlations mapped to the normal's, made positive definite by Higham's
nearest correlation matrix. The draw (`phx-core`'s `register::profile`) forms the moments, draws each pinned value
within its part of its own marginal, then the rest from the joint normal conditional on them (Cholesky), and maps each
back to its own scale; a value its group's profile lacks is missing, named as not derived. `new_game` resolves the
setup — refusing one outside the guardrails with the rule it breaks, drawing open choices from `GEN.setup` — allots
regions and land, draws each country's values and makes its name; `instantiate` writes into the run's directory
(`phx run --setup --run-dir`) `setup.toml`, each country's record `countries/<id>.toml`, and its data `data/<id>/` —
its level's templates, the level's opening tables of the systems the world keeps (`gen/<SYS>.toml`) and its derived
values (`derived.toml`) — which the register reads in place of the templates. Each row of `data/inventory.toml` names a
derived value's, distribution's or present value's owner and source, never a value.

### 10.0a The opening dataset

Each country group's economy of 2019, in shares of its GDP, is one derivation (`tools/data/derive_economy.py`), which
first reruns the ways and price levels (`derive_tec.py`), the budget shares (`derive_hh.py`) and the plant
(`derive_cap.py`) and then writes `data/profiles/<level>/economy/`:
- **The flows** (`flows.toml`): twenty-two activities, the nineteen products with finance, real estate and public
  administration; households' own employment is no market activity. The economy is closed: what each economy used,
  from wherever it came, is made at home. The products' inputs of products are their ways' (`TEC.inputs`) at the
  group's opening prices (`GDS.opening_price` times `GDS.price_level`), so the matrix and the ways agree; what they
  and the three services use of finance, rents and public services, and taxes on products, are ICIO medians
  (`GEN.service_inputs`, `GEN.product_taxes`). Value added is each activity's residue, split into compensation,
  surplus and other taxes on production by Table 6 (`GEN.value_added`). Final uses — households (their products
  their budget shares), collective consumption, investment, inventories — are medians (`GEN.final_uses`), and each
  activity's output is what they need, (I − A)⁻¹ f (`GEN.output`).
- **The stocks** (`stocks.toml`): five sectors (households, firms, banks, the central bank, the government) holding
  currency, deposits, loans, bonds, government paper, reserves, central bank loans and equity
  (`GEN.balance_sheet`, assets positive, each row summing to nothing) and real assets (`GEN.real_assets`: firms'
  plant, which is `CAP.stock_per_gdp`, inventories and land; households' dwellings and land; the government's fixed
  assets). Levels are broad sources' group medians (IMF, World Bank), holdings the OECD's sector accounts, and the
  declared closures balance them: the central bank holds government paper for its currency and reserves; banks
  hold reserves at their ratio, equity at their capital ratio, and bonds that balance them; firms' equity is their
  assets less their debts; households hold the rest.
- A group with too few reporters of a sector table takes the developed group's figure scaled by a broad measure of
  its own (the Penn World Table's structures per GDP, the labour share), and the note says so.
- **The shapes** (`shapes.toml`): distributions the opening scales to the matrices' totals — the spread of firms'
  log physical productivity within an industry (`GEN.productivity_spread`, Hsieh and Klenow's TFPQ, the United States,
  China and India standing for the three groups) and each occupation's mean earnings over all employees'
  (`GEN.occupation_pay`, the median of each group's ILOSTAT reporters nearest 2019).
- **The sources.** Every published series the build reads is in `data/sources/raw/`, one fetcher per family
  (`tools/data/fetch*.py`, the families `bank`, `markets`, `people`, `households`, `state`, `open`, `macro` beside
  the earlier ones), each CSV trimmed to the profiles' economies and the years read, and `manifest.json` names each
  series' source, the steps that read it (`for`) and, for a derived, typed or simulated one, how (`note`). Fetchers
  merge into the manifest under a lock. The per-level ranges the realism tests judge against are derived there too
  (`macro/level_ranges.csv`). The data tools' Python packages are pinned in `tools/versions.toml`.

Assembly checks every country's dataset (`opening::economy::check`, at compile) and refuses it with each break
named, primitive and residue: each activity's supply is its uses, its output its inputs, taxes and value added, GDP
by production and by expenditure one; each instrument's assets its liabilities; firms, banks and the central bank
worth nothing beyond their equity; firms' plant `CAP.stock_per_gdp`; households' spending their budget shares. The
tolerance is the rounding of the stored places over the terms summed (Law 7). A new game copies the level's
`economy/` files (flows, stocks, shapes) into each country.

### 10.1 Phases

After the setup's derivation (§10.0), declared phases — **parties, physical stock, contracts, present values,
balances**, then **day zero** — each system
contributing what it owns,
with declared reads and writes. Each joint distribution has one owning system; each opening line kind names its
writer (the loan line's writer is `sys-bnk`, whoever draws the dwelling).

### 10.2 Drawn and derived sides

For every line kind and physical class, one side is **drawn** and the other **derived**, declared with the kind:

- **Households are drawn** — persons, roles, employment status, occupation, tenure, loans, deposits, holdings, kin —
  from census-like distributions (GEN.2), **with their lines' terms**: the wage point, the rent, the loan's rate and
  remaining term, each drawn directly over the trade's price points, so balancing never sets a price (GEN.11).
- **Counterparty sides are derived**: an employer's realised headcount, a landlord's tenancies, a bank's deposit and
  mortgage books, the dwelling stock per (zone, class). Each stratum's total is **apportioned** across the eligible
  counterparties drawn for it in proportion to their drawn size (firm size by industry, a bank's market share, a
  landlord's portfolio) by largest remainder, ties broken by lot from a named opening stream. The derived side only
  apportions counts; it takes rows on the terms the households drew. Drawn sizes are
  apportionment weights; the realised sides are what exist (Law 4), and each counterparty's difference from its drawn
  size is reported (GEN.4).
- **Vacancies and vacant dwellings** are drawn from their own rates on top of the derived stock.
- **Unmatched demand**: a stratum with no eligible counterparty takes the declared substitute (the nearest zone, the
  next class) and otherwise stays unmatched — unemployed or recorded homeless (HSG.13) — each reported with its count
  (GEN.8). As built at Stage 0 no substitute is declared, and a derived line with no eligible party to take its other
  side stops the opening (`sys-dem`'s `lines`), so no stratum is left unmatched.

### 10.3 Canonical drawing

The opening draws each region once: institutions and firms with their drawn sizes first, then each region's
households, each drawn complete from its own opening streams, persons first, its lines' counterparties chosen online
as it is formed (§10.2), so no second pass, stratum ranks or counter blocks are needed. Regions are drawn in waves on
the books' pool and booked in the regions' order, so the scratch is a wave's. Employment and tenancy lines record no
pairing (REP.23): a household joins the line of the terms it drew, and only the line's counterparty side is
apportioned; institutions' balance sheets close through the ledger's opening writes (GEN.4), each reported.

**Households are drawn persons first** (GEN.2, REP.26), so the persons' ages are the country's and the households are
what those persons make:
1. **Persons.** A region's people are apportioned by largest remainder, ties by lot, over single ages and sexes in
   proportion to the country's age standard raked to its drawn shares; that pool is every person the region holds.
2. **Families.** While the pool holds a child under the age of majority, one is drawn from it, each child alike; its
   mother is drawn from the pool's women in proportion to each age's women and their chance of a living child of
   that age; her other children at each age with her chance of one, while the pool holds one; the family's type among
   those with children in proportion to their shares at the country's fertility; a partner, when the type holds one,
   from the pool's men in proportion to each age's men and the partner gap's chance at it; when the type holds none,
   the lone parent is, at the country's share of lone parents who are fathers (`DEM.single_father_share`), a man
   drawn at the partner gap from a woman of the mother's age, who is drawn but stays in the pool, and the children
   are drawn by her age; else the mother; an older relative, when
   the type holds one, from the pool's persons of 65 and over. A child whom no woman left in the pool can have
   mothered is raised by another adult of the pool, counted in the report; a child no adult is left to raise stops
   the opening, the distributions being inconsistent (GEN.2).
3. **The rest.** While the pool holds an adult, a household's type is drawn among all types in proportion to their
   shares, and its persons from the pool as the type says: one person; a woman and a partner at the gap; persons who
   are not relatives; and for a type with children, a family of grown children — the mother drawn from the pool's
   women in proportion to each age's women and their expected living children from majority, or a lone father at
   the gap from her as for minors, each of her grown
   children at each age with her chance of a living child of that age (`DEM.grown_children`, derived as the minors'
   table is), who hold the household's adult place; a mother none of whose grown children is left heads the type
   without them, and when no woman left can have grown children a type without children is drawn instead. The
   households' sizes are not drawn: the report reads them against `DEM.household_sizes` at the country's fertility.
4. **The type follows the persons**: a person the pool no longer holds is not drawn, so the last households of a
   region hold whom the pool has left, and every person of the pool is in exactly one household.

The elder of a couple heads it, else the mother, else the one person or the first drawn. Each household is formed as
an explicit household, its persons held with their roles (§7.1), each person's health and each adult's education
drawn by its age and sex from the household's own streams, and each household is placed as one agent (§7.6), so the
agents are the population (REP.13). Education is drawn from `DEM.education_female` and `_male`, the Wittgenstein
Centre's attainment: the group's median share of each level up to upper secondary and of post-secondary's total,
which most emerging and developing economies report alone, shared among short post-secondary, bachelor and master by
the median of their parts over the economies that report them apart (`tools/data/derive_pop.py`). The
report gives each country's households by type and persons by age band against the drawn shares.

A person's birth date follows from its age at the snapshot. Its birthday falls on a day drawn evenly over the year, and
it was born in the snapshot's year less its age if that day has passed, or a year earlier if not. Its lasting disability
is drawn at the chance the prevalence table gives its age band and sex. Below the table's first band, the chance is what
the onset hazard gives from birth.

**The households' lines** are drawn with the households, since a household's bank is its attribute and its jobs, loans,
tenancy and pensions are its own rows (REP.3, REP.26). Each system that owns a line kind declares an **attachment
draw**, which `sys-dem`'s formation calls for each household once its persons are drawn, from that system's own opening
streams and the household's subject: the banking arrangement (an attribute), each deposit's balance, each loan's terms
and outstanding amount, each adult's job, the tenancy and each pensioner's pension, each with its terms drawn over the
trade's price points (REP.34), its counterparty named where the household's arrangement names it (its bank, the
treasury) or left to the derived side (employers, landlords, schemes), and its holders — the household, or the persons
of its roles. Where a counterparty is chosen in proportion to drawn sizes (the bank among the country's), the choice is
made **online**: the n-th household of a stratum takes the counterparty furthest below its share of n, ties by lot, so
every prefix of the region's households is apportioned within one of exact whatever the region's total, and no second
pass is needed. An agent's row on a line counts its contracts, and its balance is its own (REP.3, REP.9). A line is opened for each distinct (kind, terms, named counterparty); the counterparty's
side of a named line is one row counting the agents' contracts. Lines whose other side is derived — jobs, tenancies,
defined-benefit pensions — take it once every region is drawn, each line's count apportioned over the eligible
counterparties by their drawn sizes (§10.2), and the unmatched strata and each counterparty's difference from its drawn
size go to the opening report. The balances close through the ledger's opening writes (GEN.4): each row's balance
written against its counterparty's, and a counterparty's books the sum of its rows, its equity the residual, reported.
The report lists each balancing write and each apportionment as it is made to `opening-writes.csv.zst` and
`opening-apportioned.csv.zst` in the run's directory, a compressed frame at a time, and the world keeps only their
counts, the parties begun that no write named and the apportionments that gave a party of no drawn size a share: the
lists run to tens of millions of lines, which the world would otherwise hold, save and load for the whole run.
As built (`phx_ledger::attachments`, `sys-dem`'s `lines`): a system registers its draw with `Declarations::attachment`;
the opening hands the draws to the households' contribution, which runs with the contracts, once every institution and
firm is drawn. For each country each draw is made ready (`AttachmentDraw::country`), then called with each household as
it is formed — its attributes, its persons, and the wealth and income `sys-dem` drew for it from its shapes
(`DEM.opening_means`), as multiples of the country's mean — and gives its rows and the attributes it sets. A row names
its line by kind, terms and named counterparty (`LineSpec`), its holder (the household or a person by place) and its
balance: none, or a share of a pool by a weight. An agent is placed without rows; its rows are kept for it, and once the
country is drawn each line opens as one instruction, its agents' rows and its other side together, so its sides' counts
agree: a named line's counterparty takes one row counting the agents' contracts; a derived line's other side is
apportioned over the parties its draw names, by their drawn sizes, each share reported; each pool's total — the draw's
sourced aggregate — is apportioned over its rows by their summed weights and written against the counterparty. The
banks' draw is the first: a household any of whose adults holds an account banks with one bank, chosen online and held
as its attribute (`BNK.bank`), and keeps a household current account there, its share of the households' deposits (the
country's deposits less the firms') by its wealth; a household any of whose adults has borrowed owes its bank a
household loan, its share of the households' debt by its income, repaid monthly with its interest over the years it
has left — drawn uniformly between `BNK.household_loan_years_min` and `_max` until housing's mortgages draw each term —
each date taking the balance over the dates that remain (`Leg::Amortising`), so a row of many households repays its
summed balance as each of them would. Neither side of a household's line keeps a holder list
on the households' side. Labour's draw gives an employed adult a job, a person's row on the employment line of its wage
point, whose employers are derived over the firms by headcount; housing's gives a renting household a tenancy on the
line of its rent point, whose landlords are derived over the firms by their plant until the dwelling stock names them.
Both are lines of many holders on both sides, so they are cleared (§6.5). Social protection's draw gives each adult who
has reached its sex's pension age the state pension at its sex's coverage: a person's row on its sex's state pension
line, a flat monthly amount — the replacement rate of the mean wage — whose payer, the treasury, is named. A person
who retires in the run claims it as it retires (`claim_pension`, beside the benefit's claim): a draw fixed for the
person by a keyed stream (`SOC.pension_covered`) decides once whether its country's coverage takes it in, and if so it
joins its country's pension line of its sex's replacement rate of the mean wage then, opened the first time a claim
needs it, with as many of the treasury's members. The defined-benefit schemes are S4.04's, with their sources. The firms are counted by
their density over the employed (`FRM.firms_per_employed`) and sized by a Pareto law of the sourced exponent
(`FRM.size_exponent`) whose scale is solved, by halving, so that the firms' whole sizes — each rounded up, as a firm
employs whole persons — sum to the employed in expectation (`sys-frm`'s `law`). The largest, down to the individuals'
rank (REP.2), are drawn first as the top order statistics of the law of scale one and scaled. The small firms are
drawn with the parties, before the households: each country's firms below the rank, the law's scale solved again so
they employ what the large firms leave, sized up to the smallest the rank admits, apportioned over the regions by their
land and over the banks by the banks' drawn sizes, and placed as agents of the small-firm kind, one a firm, with
region, size and bank as attributes (§7.1). They
hold the firms' deposits and debt the large firms do not, by their employees, on each bank's lines for them — a
current account, and a loan line for each whole year a firm's loan may run (`BNK.loan_years_min` to `_max`), each
agent's term drawn alike and its balance repaid monthly over the dates that remain (`Leg::Amortising`) — and are
employers on the employment lines by their headcount beside the large firms. The large firms are rows of the kernel's
firm kind table, individuals (REP.2).

Labour's draw (`sys-lab`'s `jobs`) employs each adult at the country's employment rate times its sex's and ten-year age band's ratio to it (`LAB.employment_by_age`, ILOSTAT's employment-to-population ratios, derived by `tools/data/derive_pop.py`) and as an employee at its sex's share; a job's occupation is drawn by sex among those its education's skill reaches, its hours part-time at its sex's share, its notice and severance the law's, its region its household's and its start band from the tenure shares; its wage is its hours times the labour share's mean wage over an employee's mean hours (over the sexes' shares of employees and of part-time work), times the household's income multiple, on the nearest point; a person's recorded point is what full time would pay it. Once the country is drawn, each region and occupation's jobs are apportioned over the region's firms by headcount weighted by the occupation's share of the hours of the way of the product each makes (`TEC.labour`, whose columns are products), and dealt to the lines in an order drawn by lot. Of the adults not employed, the unemployed search at the rate that makes their share of the labour force the country's, and one past its pension's age is retired.

Every firm, large and small, is drawn in an industry given its size, from an alias table per size class over
`FRM.industry_by_size` (`sys-frm`'s `industry`): a large firm's a fact, a small firm's an attribute. `sys-tec` then
gives each its industry's public way-set in its country as `TEC.known` (`sys-tec`'s `known`), and the opening
reports the firms by industry per country.

A firm of an industry of one product makes it; the firms of an industry of several (mining: ore, coal, oil and gas,
stone) are dealt their products (`sys-frm`'s `filed::Products`) largest first, those of a size in an order drawn by
lot, each to the product whose persons dealt fall furthest below its part of the industry's hours: its part of the
industry's output (`FRM.product_share`, the input-output tables' split of each mining industry by its resource's
users, derived by `tools/data/derive_tec.py`) over its opening price times its way's hours a unit, none for a
resource the country's land holds no deposit of. A draw per firm would leave a country's few mining firms, whose
sizes differ by orders, making whatever product its largest drew. Once staff are hired, each firm's filed accounts
(`filed::Filed`) give it hours a unit of its way's hours (`TEC.labour`, summed over the occupations) times the
country's productivity over its own hourly wage: the productivity is labour's share (`GEN.labour_share`) of the
wage-weighted mean over the country's firms of what their ways add a unit over the hours a unit takes, so the
country's firms' wage bill is labour's share of what they add, each product keeps its way's labour intensity, and a
firm paying more makes more an hour. Its output a day is its staff's hours over that; its unit cost the way's inputs
at opening prices and the hours at its wage; its price the point nearest the lot's opening price at the country's
price level, the markup what that is over the cost.

An adult is employed at the country's employment rate, and is an employee at its sex's share of the employed
(`LAB.status_shares`). A household rents at one minus the country's home ownership. A job's monthly wage is the mean
wage per worker that the labour share gives, times the household's income as a multiple of the mean. A tenancy's rent is
the country's median rent burden applied to the household persons' share of the labour income, times the same multiple.
Each amount is placed on the nearest point of its trade's grid. The points are a declared ratio apart
(`LAB.wage_point_ratio`, `HSG.rent_point_ratio`: a quarter), since each point is a line on whose other side every large
firm holds a row. The multiples of the mean come from the distributions' closed-form means (`Distribution::mean`).

Nothing is balanced after drawing: finer attributes are drawn from their own counter keys, so a coarser setting is a
projection of a finer one, and every number of preference types is a discretisation of the same declared distribution
(NUM.4). Drawing the population takes tens of seconds on the phone's cores.

### 10.4 The snapshot, day zero and settling

The opening is **one date's snapshot of the present** (GEN.5): stocks and contracts, and the single latest value of
everything observed on that date — each market's print and fixing, each reference rate and index level, each
published statistic's latest release, each rating, each firm's latest filed accounts (derived from its books and
lines), households' surveyed expectations. These are **present values**, one each, dated the snapshot day: public
series of one, never a drawn past.

- **The steady-path convention**: whatever a contract's balance or current amount depends on from before the
  snapshot — amortised principal, accrued interest, a floating coupon's current fixing, an indexed amount's accrued
  ratio, a revalued pension slice — is computed as if the present values had held since its start. One convention
  for every contract, so contracts on one reference agree; nothing is stored as a series.
- **Carrying values** are the snapshot's prints or a valuer's method on them, so day one passes the audit (GEN.7).
- **Outlooks** start at the present value they read, households' at their surveyed expectations, each width at the
  observed dispersion (VAL.10); performance records and surprises start absent.
- **Day zero** (GEN.13) is the calendar day before the first; it runs stage 5 only (5a–5d) for the decision kinds
  declared as opening decisions — posted prices, wage offers, rates, standards, ratings confirmed, orders — each
  party once, simultaneously, from its own state and the snapshot. Nothing meets or settles, nothing is repeated to
  agree, and nothing a party decides is drawn (GEN.4, GEN.11). Orders stand into the first day.
- **Settling** (GEN.6) then runs the ordinary day for the owner's length, one simulated year by default; that year is
  the world's only history. Nothing in the world reads the length.
- The epoch lies early enough that every opening contract's start date is a day (TIME.2).

### 10.4a The books at the opening

- **The world's books** (`phx_ledger::books::Books`) hold the ledger and one kind table per kind of individual; the
  assembly opens them empty and hands them to each contribution as `&mut dyn Any` inside `phx_core::Opening`, so
  `phx-core` never names the ledger. Contributions run by phase, then by their system's code, then by name.
- **Drawn sizes** pass between contributions in the books' `drawn` map, keyed by stratum and country (a bank's
  weight, a firm's debt, deposits and plant, a firm's lender); the derived side reads them and apportions (§10.2).
- **Balancing** is the ledger's `OpeningWrite` legs, applied by the one apply routine at `ApplyAt::Opening`, where
  rows may also be opened, and moved whole between holders with their balances, and nothing else moves. Each write is reported with its party, amount, the identity it
  served and its counterparty; once every phase has run, each party's opening **equity** is computed once, as its
  rows' balances signed by side plus its holdings at cost, into the report. At the assembly's end, once the player is
  seated, the ledger forgets the opening's day book whole (`Ledger::opened`): the audit and the accounts read days, and
  the opening is none, so the accounts open on what the opening wrote.
- **Plant** is held by kind and condition class (§7.10), a real asset per (country, kind, class) counted in a cent of
  the kind at world-average prices, held at cost: each country's stock of a kind, its GDP times the group's stock per
  unit of GDP over the price level there of the product the kind is bought as (`GDS.price_level`), apportioned over every firm by its employees times its industry's plant of the kind per hour worked,
  and spread over the classes as a steady stock growing at the country's rate would be.

— each class ρ = (C ÷ L) ÷ (g + C ÷ L) times the one before, its units priced so the classes' values sum to the
stock's value, a unit new costing that price level, each lot costing what it is worth. The large firms' structures pass to
housing in the books' `drawn` map (`CAP.structures`), whose owners stand in for the landlords.
- **Accounts** — reserves, the treasury's account, the central bank's claim on the treasury — have no dates and are
  marked spent at the opening; every other opening line falls due on the first date of its schedule after the
  snapshot.
- **The sovereign's bills** close the banks' books, last of the balances (`sys-sov`'s `Bills`): once each bank's
  deposits, loans and reserves are written, it holds the bills that bring its equity to its country's capital ratio of
  its assets (`GEN.bank_capital_ratio`), none where its assets already carry more, spread over a bill's weeks as one
  line maturing each week, so the weekly auctions roll a week's part over; the world's bill book learns them after the
  opening (`bills_opened`). A bill's face is dollars at the country's units to the dollar. The public debt the banks'
  bills and the central bank's claim do not hold waits for the holders of later stages, and the difference from the
  derived debt is reported as an adjustment.
- **Opening draws** use each system's stream `<SYS>.opening` with the subject `Opening(stratum, ordinal)`
  (`phx_core::contribution`), so a party's draws exist before its identity; a country's strata are numbered by purpose.
- **The institutions**: `sys-cb` opens one central bank and one treasury per country, sited on its land. Reserves are
  the banks' liquid share of their assets, apportioned over the banks by their weights; the central bank's claim on the
  treasury is its drawn assets, raised to the reserves where they are larger — the one change, reported as an
  adjustment — and the treasury's account holds the rest. `sys-bnk` fits a Zipf law's count and exponent to the three-
  and five-bank concentrations (`zipf.rs`) and apportions the firms, in a drawn order, over the banks by those shares,
  largest remainder with ties by lot; each large firm keeps a current account at its bank and owes it a term loan whose
  term is drawn between `BNK.loan_years_min` and `_max` and whose start falls uniformly over the days of its term before
  the snapshot. `sys-frm`'s large firms are the top order statistics of the firm-size law down to the promotion rank,
  drawn one after another in logs. Every kind's legal form is one of `PTY.legal_forms`, and assembly refuses a kind whose
  form the law does not declare.


### 10.4b Dated flows before the payer pass *(retired)*

Retired with the payer pass: stage 7 no longer pays each due as its own instruction from a pass
over every holder's rows. Settlement is `Books::settle_day` — the stream over the holders' runs, the fixed point and
one net per (line, party, side) applied one instruction per line — as §6.5 says, with the fails and the close there.

### 10.4c The audit's view of the books

`phx-core` declares the traits the kernel crates meet through: `BooksAudit`, the books as the audit reads them
(instruments and lines by index, each checked by the ledger, and what a party holds on an account), which the ledger
implements; `MarketsAudit`, `AccountsAudit` and `CellsAudit`, the tape, the accounts and the agent tables; and
`LegRecords`, the audit's own record of the day's settled legs, which `phx-audit` implements. The ledger's five families
(ownership, contracts, money, flows, units) read them through the family context. Each leg's digest carries what it
counts toward its instruction's balance, a liability row's count against the asset side's, as the apply check counts it,
so contracts leaving a line with their counterparts balance. A row recounted outside any instruction — a person leaving
its rows at a death (§7.4) — is recorded by the ledger as an unpaired digest, its count and balance from what the row
held to what it holds, and handed to the audit in its order among the instructions' legs, so a position's opening and
its day's moves still make what it holds. Every row placed, removed or recounted marks its line side changed
(`Lines::moved`), so what was read of a side, such as the contracts a draw is made over, is known stale. In the position
read, a party that has ended holds nothing of its own: what it held passed on when it ended. The money family reads its
day's span of money lines together, a side of many small holders that keeps no list summed in one pass over the tables
of the kinds that may hold it; as built this sweeps every live agent's rows whenever the span holds a retail deposit
line, undeclared and uncounted. The representation family bounds a row by the persons the agent holds in the
roles its side declares (REP.31), as a household's jobs count its adults; a side declared
`many`, whose holder counts a contract for each counterpart (an employer, a bank), is bounded by no count per person.

### 10.5 Settled worlds for testing

After each step's build, the build machine — the development VM where the world is built — generates the world at
the play resolution (until the Stage 0 gate first sets it, the declared initial one) and runs it with the audit and
every live check (§14.7): an ordinary step's for 120 days from day zero, a stage gate's settled for the owner's length
and run two years after it;
its numbers test the code and are never read as the world's. A stage gate's device run generates and settles its own
world on the phone. CI never runs the world.

---

## 11. Persistence

- A **save** is a directory (`phx_world::save`): a manifest and one file per store of zstd frames. The manifest
  (`save/manifest.rs`) holds the format (`SAVE_FORMAT`), the build (a hash of the running binary), the register (a hash
  of the data files the world was assembled over), the seed, the day and its date, the settling length, the persons
  the world was opened with, each store's name, file, bytes compressed and raw and logical hash, and the world hash.
- **Stores**: `core`, the day and the whole core (`#[derive(Saved)]` on `Core` and everything it holds: every kind's
  parties, records, accounts and cash lines, the persons, every family's contracts with their lists and wheels, the
  goods' holdings and units, labour's vacancies, postings and searchers, the lenders, the central banks, bills, taxes,
  agencies, the insolvency law's claims, the accounts, the statistics and outlooks, the decisions' preferences and
  counts, and the laws the opening compiled); and `run`, the run's measures (turns, saves, injections), outside the
  world hash. The run's findings are its report's, printed as they are found; a load starts with none.
- Saves are written at the moments SET.12 declares — when the player saves, when the app is set aside, and at the
  declared interval (`SET.save_every_months`) — at a day's close, and the world pauses while one is written (N8.10).
  The core's store and the run's are written by tasks of their own on the pool, beside one hashing the world; a
  store's bytes are cut into fixed frames of 1 MiB, each compressed on its own as a zstd frame, a wave of sixteen at a
  time across the pool, so the file is the same however they were compressed. Columns are written as their live rows;
  a chunk arena as its raw words with its dead-word count. Under miri, which cannot run the foreign zstd, the stream is
  written as it stands.
- **What a save does not hold** is what the build supplies: the register, the calendar's rules, the streams (keyed by
  name, subject and day, so they have no position), the map (generated from the seed), each system's own state, and
  the declarations the core holds by reference — the population's household kind, the benefit's claim and the
  consumption tax's rule, the bills' kind, the statistics' rate and index — which a load binds again as the opening
  bound them (`Core::rebind`), marked `#[saved(skip)]`. The day's working buffers (`Work`, a meeting's and a wheel's
  scratch) are empty at a close and are skipped too. Every name a save holds — the kinds' and the families'
  (`consts::families`) — is read back as the build's own, and a name the build does not declare is refused.
- **Every save is full** (SET.12, spec Appendix E 22): a restore reads one save. Allocator state (free slots, list
  links, released slots) is saved as it stands.
- **Retention**: the latest complete save, plus the one being written. A save is written into a directory named
  partial; once every file and the directory are synced, it takes its complete name and the root is synced, and only
  then is every other save in the root, complete or left partial by a save that never finished, removed.
- **A load** (`phx_world::load`) refuses a save of another format, build, register, seed or number of persons, each
  naming its reason (N5); it assembles the world from the build and the data as the save's was, draws no opening,
  reads the core and its day, holds them to the manifest's world hash, binds the declarations again, moves the
  calendar's window to the save's year and reads the run's measures.
- **The save check** (LC-0-35): each periodic save of the build run is read back from its files alone, hashed and
  dropped, and its hash held to the manifest's; its sizes and its write and check times are the run's (LC-0-36).
- **No copies**: a save is loaded only to continue the one run, or, on the build machine, apart by `phx inject` to be
  audited and discarded, never run on (N1).
- **Injection** (N1, `save/inject.rs`): the build run takes one more save, at the close of the 30th day after
  settling, into its own directory, and after its last day hands it to `phx inject` in a process of its own, so the
  run's memory stays the world's alone. Each family (`core_audit::FAMILIES`: money, goods, contracts, persons, taxes,
  debt, loans, accounts, revenue) has its discrepancy put into a fresh load of it — a unit where only that family's
  invariant reads it: money in a treasury's account no flow paid, a good's unit nothing made, a contract naming a
  household that never was, a person counted and not held, a unit of tax arisen, of bills issued, on a loan book, in
  an equity account, of revenue — and the audit then runs once over the state as it stands at the save's next day, no
  day stepped (`Core::audit_close`, which holds money and goods to what they were before the injection); the families
  that found something are recorded with the run's measures, where LC-0-10 reads them. A world not read from a save
  refuses an injection.
- **The world hash** is `phx-store`'s `LogicalHasher`, SipHash-2-4 with a 128-bit result under a fixed key
  (`HASH_KEY`), over the day and the core as their save encodes them; a save reads back to exactly the layout it was
  written from, so its hash is its close's.
- **Encoding** (`phx-store`): a column is encoded field by field, each field's transform declared with its type
  (delta modulo 2⁶⁴, zigzag, both, or plain), then bit-packed per block of 1 024 values at the block's widest value.
  Each field's stream is little-endian: magic `PXCL`, format version, field width, transform, element count, then per
  block its bit width and packed values. A decoder refuses a damaged, truncated, foreign or unknown-version input with
  its reason, never a guess.

---

## 12. Observation and the player

- **Views** are built at 10e on a turn's last day from records and the state at its close, into fixed-bin histograms,
  and swapped in behind an `Arc`; tables cross the FFI in pages; `phx-obs` writes nothing
  (Law 17). As built, the host (`phx-cli`'s run, `phx-ffi`'s bench) drives the observer after each turn: its
  `Recorder` reads each day the turn closed from the run's own records — the day's work on the agents, the day's
  settlement and the day's events — into the **macro reads** `data/observer/READS.toml` declares, each a named
  measure (`agents.persons`, `settlement.gross`, `events.<kind>` summing the sizes of the day's events of a declared
  kind, …); its `Views` build, at the turn's close, each read's latest value and the declared histograms of a kind's
  agents, of an attribute's values, over fixed lower edges with the values
  below the first counted apart. Each histogram is an opening distribution (GEN.8): the host keeps the opening's view
  and reports each histogram's distance from it (half the summed differences of the bins' shares) at settling's end
  and at the run's end, and how far its members moved within it: each view keeps the bin of a fixed sample of the
  kind's agents — one in `MOBILITY_SAMPLE` by party identity, so no draw picks it — and the share
  of those in both views whose bin changed is read beside the distance (`phx_obs::moved`). The macro reads count every
  event of a kind, private ones among them, since they measure the world's truth; what a player's page shows of
  events is the public ones alone (OBS). A read may declare why it can rise on every day (`grows`); liveness fails a read that rises on
  every day without one. The observer is a `phx_world::Observer` passed to `run_turn_observed`: after each day it
  takes the reads through the inspector, inside the turn's time, and the world is the same
  world with it or without it. PC-20 refuses any `&mut` to the world's stores, and any naming of `World`, in
  `phx-obs`. On the phone the observer follows the measured turns, not settling, and the views are built twice, at
  settling's end and at the run's end, not at each turn's close.
- **Two builds**: the participant's reads only through `ParticipantScope`, its party's scoped read; the inspector's
  compiles only with the `inspector` feature, which the participant build refuses. As built at Stage 0 there is one
  build: no crate declares an `inspector` feature, `Inspector` and the `Recorder` are always compiled, and
  `ParticipantScope` is not built. Every shown number is a
  `Shown<T>`, built from a record entry, a read at the close, a published statistic, or a fixed-bin aggregate of
  reads; pages are generated from the interface crates' view schemas, and a decision
  page from its point's input view and intent.
- **The representation**: the inspector's pages show REP.15's report — the representation, its factor and its
  counts — beside every distributional number (N5).
- **Events on the core** (OBS.3, CHN.4): each hazard's hit is recorded in the core's `EventStore` (`Core::happened`),
  dated, its household the subject and each person it reached a detail of one person, beside the day's counts by kind;
  at each close the world makes public, by the declared rule (`OBS.public_events`, compiled at assembly into the
  world's `EventsRule`), each event of the day that the rule makes public. The store is saved with the core. LC-0-58
  reads it.
- **An agent's page** (OBS.8) shows the agent itself: its attributes, persons, positions and rows, and the recorded
  events that name it. Nothing is drawn to show it.
- **The player** (OBS.4, `core_player`) is a household drawn at the end of the assembly from the households of the
  country its setup names, each equally likely, by the stream `GEN.player`, and seated in the core's `PlayerDesk`, saved
  with the core. Every decision a household or its person takes runs through the decision core's `say`
  (`Core::decide_own`, and a population process's `AgentView::decide`): for the player's household it counts the
  decision under the player's standing and says `Say::Queued` with the intent queued for that decision, else
  `Say::Rule` where the setup delegates, else `Say::Kept`, which leaves the decision untaken that day (no spending, no
  search, no acceptance or claim, the stance and the household's trying unchanged, the employee working on at an
  offer); for every other household it is the rule's. `decide` refuses a decision of the player's household that
  passes the player. A turn's intents are queued before its first day; at each close each decision that came for the
  household is recorded with how it was taken (`PlayerDay`) and the intent it took leaves the queue, so an intent is
  taken on the first day its decision comes. LC-0-57 reads the record.
- **On the phone**, `phx-ffi` runs the engine on its own thread with the pinned pool: create, load, step a turn, read
  a view page, submit an action, save, and in the inspector build export the recorder's series (§14.8). As built at
  Stage 0 that play surface is not built: `phx-ffi` exports `run_bench` (the probe and micro-benchmarks), `run_world`,
  `run_load` (the full-load bench, §14.6) and `run_programme`, each run on the app's calling thread, the world on that
  one thread. `run_world` assembles the world, settles it for the owner's length (`GEN.settling_years`), reporting the
  settling's turns, days and time, runs the measured turns (the app passes 60), each reporting its wall time, days,
  whether it ends on a business day, its payments and rows scanned, its sub-steps' wall times (summed from the world's
  `SubStepRecord`s, kept by the application's clock), `VmHWM`, PSS and thermal status, then saves the world whole,
  drops it and reads it back, timing both and checking the world hash. The bench flavour shows each result live as it
  completes; `Run all` (`run_programme`) runs the probe, the world's turns and the full-load bench in turn and writes
  the device report's second version (`perf/schema/device-report.json`).

---

## 13. Budgets

Every line below is **count × unit cost**, each with the counter that ratchets it (§16.8). Unit costs are the
third review's **measured** kernels, scaled to a tuned phone core; counts are estimates for the coarsened
representation (spec Appendix E 31). The **design point** is 0.7 million household agents (about 2.4 million persons), 0.25
million firm and business agents, and 2,000 zones. The first measurements (§14.6) replace every
number here; the size and zones are RESOLUTION and are set where both budgets hold (N8.5) with at least 10%
headroom, this document's margin for the estimates' error.

### 13.1 Memory (4.5 GB resident, N8.4), at the worst day's peak, through Stage 2

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household agents: attributes, positions (with each deposit row's), rates, review exposures and attention rates per kind, own outlooks, arena references, the due-day run's head (Stage 2's review kinds S2.05, S2.06) | 0.7 M | 576 | 403 MB |
| Household persons and attachments, a packed word each | 0.7 M × 43 words | 8 | 240 MB |
| Relationship rows (40 per household agent, 12 per firm agent, 2 M of individuals; Stage 0's pensions in payment, 0.35 M state pension rows at 16 bytes and 0.5 M DB pensioner rows at 24; Stage 2's invoice rows, 10 per firm agent and 0.5 M of individuals, one per (holder, market, terms, statement period)) | 36.85 M | ≈ 23 average | 849 MB |
| Line holder lists, with block slack (none on the pensions' retail side, §4.5) | 36 M | 6 | 216 MB |
| Holdings and instruments' holder lists | 5.25 M | 28 | 147 MB |
| Firm and business agents: the record (itemised below) | 0.25 M | 500 | 125 MB |
| Lines (kind, terms id, side counts, next due day, holder list; Stage 2's invoice lines are a few thousand) | 3 M | 32 | 96 MB |
| Loan lines' balance totals per arrears stage (Stage 2) | 0.3 M × 4 | 8 | 10 MB |
| Interned terms with their sharded hash (keys retired with cells) | 1.5 M | 72 | 108 MB |
| Agenda: next days per (agent, reason), one calendar entry per agent | 0.95 M × 16 | 5.5 | 85 MB |
| Kind tables of individuals and their facets | 0.15 M | 1.5 KB | 225 MB |
| Estates open, one per ended party: openings × life (§9.1) | 60 k | 512 | 31 MB |
| Instruments, lots, liens, commitments, messages that live across days (Stage 2's listings, 13 MB) | — | — | 163 MB |
| Markets, marks and fixings history; public records (Stage 2's filed accounts over two years, 48 MB); events | — | — | 198 MB |
| Map, network, deposits, stock per (tile, class) and its index | — | — | 80 MB |
| Directory with bounded tombstones | — | — | 50 MB |
| Day buffers at the worst day (payee reduction streamed shard by shard; intents; sort scratch), by their room; measured at 889 MB on Stage 0's payday of 2.8 M payments, over this line | — | — | 600 MB |
| Arena slack and page tails (15% of variable-length stores) | — | — | 214 MB |
| Save buffers (the renumbering slice retired with cells) | — | — | 94 MB |
| Views (tracers retired with cells) | — | — | 60 MB |
| Android process baseline | — | — | 250 MB |
| **Total** | | | **4 244 MB** |

Through Stage 1 the design point peaks at about 3.98 GB against 4.5 GB — the run head in every household record from
Stage 0 adds 6 MB, Stage 0's pensions in payment 20 MB with slack, the persons and attachments as laid out 30 MB more
than first budgeted, and the landing index (as measured) and group aggregates retired with cells 134 MB less — so 12%
headroom, if the day buffers are brought under their line. Stage 2 adds about 251 MB — invoice rows with their
holder-list entries and slack 104 MB, filed accounts 48 MB, household agents 45 MB, estates 31 MB, listings 13 MB, loan
lines' stage totals 10 MB — to about **4.23 GB**: 6% headroom, short of the required 10% (a peak of at most 4 050 MB).
Stage 3 adds about 240 MB — institutions' positions and their lots 104 MB, households'
holding rows 30 MB, individuals' deviations from their methods' outlooks 19 MB, household attributes with participation
18 MB, registered outlooks 12 MB, household agents 11 MB (592 bytes), money-market lines 10 MB, records and instruments
10 MB, slack 26 MB — to about **4.47 GB**: 0.7% headroom. Stage 4 adds about 387 MB (the
plan's S4.07): relationship rows 138 MB — policies 4.9 M and firms' 0.5 M at 16 bytes, DB active and deferred rights 1.3
M at 24, derivatives 1 M at 16 with no `amount`, claim and compensation lines — household attachments 42 MB (policy
attachments with their renewal bands, and the pension scheme joint with employment), DC pots with their `pending` word
38 MB, lines 35 MB, interned terms 25 MB, household agents 22 MB (624 bytes), kind tables 15 MB, records and valuations
30 MB, holder lists 8 MB (derivative lines and institutional sides only, §4.5), slack 34 MB — to about **4.86 GB**: 8%
over the budget itself. Stage 5 adds about 116 MB (the plan's Stage 5 ledger, after its
reviews' remedies): relationship rows 42 MB — benefit claimants 0.65 M at 16 bytes, earnings-related state-pension
rights 0.6 M at 24, tax payables and instalments 0.35 M at 24, payroll payables' balances per base, foreign-currency
deposits and nostros at 38 — lines and terms 15 MB, attachments 13 MB (intentions during a campaign, waits and claims),
household agents 11 MB (640 bytes), foreign holdings 9 MB (at 32 and 24 bytes), records 8 MB, the `vote` review's side
column during a campaign 5 MB, messages 2 MB, kind tables 1 MB, slack 10 MB — to about **4.97 GB**: 10% over the budget
itself. Stage 6 adds (the plan's Stage 6 ledger): firm agents' known ways, about 50 k at
about 1 KB, 50 MB; household persons' records 28 MB (the education record, schooling, the search band, participation and
retirement); household agents 22 MB (672 bytes); relationship rows 6 MB (kin rows, licences); firm agents 1 MB (504
bytes, past this table's 500-byte line by 4) and their cumulative-output lists 7 MB; ways, known-way sets and patents 7
MB; views 10 MB; imitation pools 4 MB; receipts 1 MB; slack 11 MB — about 147 MB, to about **5.12 GB**: 14% over the
budget itself. How rows fall with the size is measured (§14.6).

The firm agent's record, counted against 500 bytes: the hot record with its three leading position totals (output
stock, cash, sales outlook) 64; input stocks, four, and work in progress 40; output and four input use rates 40; unit
cost, markup, demand and revenue since review 32; the sales outlook's width and last surprise 16; review exposures,
six lumpy kinds (price, way, entry and exit, closure, investment, vacancies) 48; attention gains, six at 4 bytes, 24;
standing money rates, six, 48; the kink signature 8; arena references, eight (plant, employment, loan, deposit, supply
rows, profiles) 64; the due-day run's head 8 — 392, the rest Stage 2–5's (trade credit, tax positions, learning).
The firm table's agenda has about eight reasons: two hazards, reviews, wakes, schedules, kink days, dues and wear.

### 13.2 Time (1 s median, 2 s worst, N8.2)

**Measured** (2026-09-28, build machine, 750 000 persons): a median turn of about 8 s and a worst of 26 s (the smoke,
§14.7), eight to thirteen times the budget, with Stage 1's systems only; the full-load bench at 1.5 times the finished
world's counts gave 7.7 s and 29 s, its unit costs 5 to 50 times the estimates below (settlement about 1.5 µs a payment
against 30 ns). The estimates below were never met; the restructure (the plan's S1.17–S1.25) replaces the layout they
cost and sets unit targets for a finished world of 5 million persons, and this section is restated with its bench.

Unit costs are **phone core-nanoseconds**; wall time is core time over the phone's **sustained** parallel speed,
taken as **3 core-seconds per second** (one fast and five medium cores, sustained) until Stage 0 measures
it per core class (§14.6). A turn's time is the sum of its days: an ordinary business day; a **non-business day**
(TIME.8: no institutions, settlement, funding or valuation; reviews only for the decisions the day's meetings need);
or a **heavy business day** (a quarter-end payday after a holiday, with the carried needs).

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Hazard and need draws per agent, drawn ahead (§7.3) | 1.1 M | 180 ns | 66 ms | 66 ms | 66 ms |
| Agenda maintenance: redraws when an agent's rates change (§7.3) | 0.24 M | 150 ns | 12 ms | 8 ms | 18 ms |
| Agent visits: continuous decisions on schedule, kinks | 0.28 M | 500 ns | 47 ms | 17 ms | 55 ms |
| Group-aggregate updates from changed rates (retired with cells) | 11 M | 20 ns | 73 ms | 27 ms | 85 ms |
| Occasion evaluations, per (agent, decision) | 2.4 M | 80 ns | 64 ms | 13 ms | 80 ms |
| Choices of acting agents | 0.25 M | 400 ns | 33 ms | 10 ms | 40 ms |
| Parts: from decisions, kinks and age (0.15 M) and seller spreads (0.15 M) (retired with cells) | 0.3 M | 2.5 µs | 250 ms | 67 ms | 375 ms |
| Meetings and re-choice rounds (about 10 sellers in reach; about 6 µs at 100) | 0.2 M buyer-products | 650 ns | 43 ms | 43 ms | 43 ms |
| Seller spreads on review days (retired with cells) | 0.05 M sellers | 3 µs | 50 ms | — | 50 ms |
| Labour matching (about 15 vacancies visible per searcher; about 50 ns each) | 0.1 M searching agents | 1 µs | 33 ms | — | 33 ms |
| Physical flows realised at visits and at kinks (stock at zero, lead times, wear classes) | 0.3 M | 150 ns | 15 ms | 10 ms | 15 ms |
| Settlement: run heads; the rows of the segments due today, about a fifth of holders' on an ordinary weekday (Stage 0's pensions in payment among them), those not due reading their line's next due; payments applied (§6.5) | 1.1 M heads; 6.2 M rows, 4 M not due; 2 M payments (58 M rows, 3 M not due, 4.4 M payments heavy) | 2 ns; 10 ns, +5 ns; 30 ns | 48 ms | 3 ms | 243 ms |
| Agenda gather at 1b | 1.5 M entries | 20 ns | 10 ms | 10 ms | 10 ms |
| Institutions, financial markets, the state | — | — | 83 ms | 10 ms | 133 ms |
| Valuation, accounts, tests, publications | — | — | 27 ms | — | 67 ms |
| Audit, statistics, events, views | — | — | 40 ms | 27 ms | 53 ms |
| Barriers and tails | up to 46 sub-steps | — | 30 ms | 20 ms | 35 ms |
| **Total** | | | **924 ms** | **331 ms** | **1 401 ms** |
| Tolerance control (retired with cells) | 0.95 M | 300 ns + joins | +170 ms | +170 ms | +170 ms |
| A publication with a large surprise: the wake pass, then the woken agents' visits | 0.95 M hot records; up to 0.7 M visits | 5 ns; 500 ns | +120 ms | — | +120 ms |

**Stage 2 adds** (the plan's Stage 2 ledger, with its representation choices, §18 item 23):

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Parts: housing transactions, one part each (30 k); bank switches, made at settlement (9 k); credit — arrears and restructuring splits, record-stage re-keys, insolvency entries and discharges, heirs (19 k) (retired with cells: the agent changes in place) | 58 k | 2.5 µs | 48 ms | 5 ms | 72 ms |
| The same parts at the prototype's measured 9.9 µs (the risk case, outside the totals; retired with cells) | 58 k | 9.9 µs | 191 ms | 20 ms | 285 ms |
| Housing search (about 20 listings in reach per searcher, about 50 ns each) | 50 k searching agents | 1 µs | 17 ms | — | 17 ms |
| Occasion evaluations: workouts, terms, financing, distress, housing, vehicles, rents, bank choice, arrears | 0.41 M | 80 ns | 11 ms | 3 ms | 14 ms |
| Institutions: funding, capital, supervision; the electricity auction and offers; provisions and ratios over per-(line, stage) totals | — | — | 10 ms | 5 ms | 15 ms |
| Invoices due, in the holders' due-day runs: on a statement's due day, the rows due and their pooled payments | 1.5 M rows and 0.3 M payments | 10 ns and 30 ns | — | — | 8 ms |
| **Stage 2 total** | | | **86 ms** | **13 ms** | **126 ms** |
| A resolution's D+1: deposit rows split, banking arrangements rewritten, agents rewritten in place | 0.5 M; 20 k; 0.25 M | 100 ns; 300 ns; 80 ns | +25 ms | — | +25 ms |
| A bank-run day: the wake pass, the woken agents' visits, the switchers' changes | 0.95 M; up to 0.35 M; up to 50 k | 5 ns; 500 ns; 2.5 µs | +100 ms (+225 ms at 9.9 µs) | — | +100 ms |

**Stage 3 adds** (the plan's Stage 3 ledger, with its representation choices, §18 item 24):

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| The linked call: the money market and the tenders, one core per country, the countries in parallel (wall time, undivided) | about 20 000 edges per country | — | 3–5 ms | — | 5 ms |
| Money-market orders | 6 k | 2 µs | 4 ms | — | 6 ms |
| Orders and meetings of auctions, books, dealer markets and funds' dealing | about 0.13 M orders; 1 500 closing calls; 3 000 dealing meetings | 100–500 ns; 3 µs; 1 µs | 16 ms | — | 27 ms |
| Institutions' closed-form values; managers' reviews of candidates | 80 k; 30 k | 100 ns; 300 ns | 6 ms | — | 6 ms |
| Households' holdings: reviews; instrument choices, a multinomial over the reach | 30 k; 10 k | 80 ns; 1 µs | 4 ms | — | 4 ms |
| Parts: members changing participation or leaving their row's step (retired with cells) | 2 k | 2.5 µs | 2 ms | — | 2 ms |
| Registered instrument outlooks and values, on new prints; registrations | 0.2 M; 0.1 M | 70 ns; 30 ns | 6 ms | — | 6 ms |
| Dividends and votes over holder rows | 1.5 M on heavy days | 10 ns | — | — | 5 ms |
| The central bank, the treasury, non-bank lenders, fixings, indices, ratings and reports | — | — | 3 ms | — | 9 ms |
| Valuation, margin, repo margin, covenants | 0.9 M; 2 k; 30 k | 10 ns; 4 µs; 200 ns | 8 ms | — | 16 ms |
| Money funds' daily accruals | about 3 000 funds | — | — | 1 ms | — |
| **Stage 3 total** | | | **53 ms** | **1 ms** | **86 ms** |
| A fund-run day: the wake pass, the woken agents' visits, the switchers' changes | 0.95 M; 0.2 M; 30 k | 5 ns; 500 ns; 2.5 µs | +60 ms (+134 ms at 9.9 µs) | — | +60 ms |

**Stage 4 adds** (the plan's Stage 4 ledger, S4.07, with its representation choices, §18 item 25):

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Settlement: Stage 4's dated rows in their holders' runs — policies, annuities and claimants in the segments scanned, derivatives most days; premiums paid | 1.1 M and 1 M rows, 0.8 M not due; 0.25 M payments (4.9 M and 1 M rows, 0.4 M payments heavy) | 10 ns, +5 ns; 30 ns | 11 ms | — | 24 ms |
| Occasion evaluations: `insure`, `pension`, acceptances | 0.25 M | 80 ns | 7 ms | — | 9 ms |
| Choices of acting agents: insurer, fund, trust and annuity choices | 20 k | 400 ns | 3 ms | — | 4 ms |
| Claims, third-party harm included: opened at 3e, adjusted at 5c | 6 × 10⁴ | 300 ns | 6 ms | 2 ms | 6 ms |
| Derivative marks and margin: valuations per distinct terms, rows into accounts and buckets, initial margin per account | 5 × 10⁴; 1 M; 10⁴ | 400 ns; 25 ns; 3 µs | 25 ms | — | 30 ms |
| Derivative meetings: book orders, client requests to about four dealers, users' reviews; closing calls, auctions, expiries | 10⁵; 10⁴; 10⁴ | 300 ns; 1 µs a quote; 2 µs | 32 ms | — | 40 ms |
| Valuation: actuaries per (cover, model point) on valuation dates; roll-forwards per (insurer or scheme, bucket) | — | — | 2 ms | — | 4 ms |
| Institutions: insurers' pricing, underwriting, reinsurance; trustees, sponsors, employers' scheme offers; SEC and MNA decisions | — | — | 10 ms | 1 ms | 15 ms |
| Contribution follow-ons at 7e, per (agent, employment row × scheme) on paydays | about 1 M heavy | 30 ns | 2 ms | — | 10 ms |
| Tenders, annuity purchases, drawdowns, fund switches (parts and promotions retired with cells) | ≤ 5 k | 2.5 µs | ≤ 4 ms | — | ≤ 4 ms |
| **Stage 4 total** | | | **102 ms** | **3 ms** | **146 ms** |
| A scheme valuation date: one declared sweep of the household arenas | — | — | +10 ms | — | +10 ms |
| An insurer's resolution day, D+1: one scan of the arenas, rows split at the protection limit, agents rewritten | 35 M; 0.5 M; 20 k | 2 ns; 100 ns; 300 ns | +45 ms | — | +45 ms |
| A takeover of a widely held firm, its first answer day: notices, evaluations, tendering agents' changes | 0.3 M; 0.3 M; 50 k | 20 ns; 80 ns; 2.5 µs | +50 ms (+175 ms at 9.9 µs) | — | +50 ms |

**Stage 5 adds** (the plan's Stage 5 ledger, after its reviews' re-costing and remedies, §18 item 26):

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Levies: payroll contributions fused with withholding; VAT on cash sales in their instructions and on terms sales per invoice row at its statement; realised gains fed at settlement | 2.4 M levies (6 M heavy); 0.1 M cash sales (1.5 M invoice rows heavy); 0.1 M gains | 7.5 ns; 15 ns a side (3 ns a row); 20 ns | 8 ms | — | 19 ms |
| Tax dues, returns and assessments: remittances and instalments in due-day runs; corporate returns, inheritance tax, arrears | 0.35 M rows and 0.2 M payments heavy | 10 ns and 30 ns | 2 ms | — | 6 ms |
| Benefits in due-day runs; the state pension's earnings-related rights, one follow-on leg per agent per payday | 0.15 M rows, 0.1 M payments; 0.1 M legs (0.8 M, 0.5 M; 0.5 M heavy) | 10 ns, 30 ns; 30 ns | 3 ms | — | 13 ms |
| Claims and service needs; agencies' staffing, purchases and serving draws | 85 k evaluations; 85 k draws | 80 ns; 150 ns | 8 ms | 2 ms | 10 ms |
| Parties and pollsters outside campaigns | — | — | 1 ms | — | 1 ms |
| Currencies: individuals' requests to about four desks, interdealer calls, banks' posted quotes, conversions drawn in payments, fixings, translation | 5 k; 3; 0.15 M; 0.1 M | 1 µs a quote; —; 50 ns; 20 ns | 12 ms | — | 15 ms |
| Currency derivatives: meetings and users' reviews; marks per (pair, maturity) from 6c's forward points, and margin | — | — | 10 ms | — | 13 ms |
| Registered values of foreign instruments | — | — | 1 ms | — | 1 ms |
| Across borders: foreign sellers in border groups' reach; crossings and customs; per-category tallies; migration's upper nest over the day's memo; admissions, movers, invoicing | —; 2 × 10⁴; 0.2 M rows; 50 k reviews | —; 500 ns; 10 ns; — | 18 ms | 2 ms | 21 ms |
| **Stage 5 total** | | | **63 ms** | **4 ms** | **99 ms** |
| A campaign business day in the largest country: platform values per agent, shared benefit and service values, counts per persons' combination (budgeted as the retired parts) | 25 k × 6; 10 k; 67 k agents | 350 ns; 300 ns; 150 ns | +25 ms | — | +25 ms |
| An election's eve in the largest country: `vote` for the adults still undecided | up to 0.2 M agents | as above | +60 ms | — | +60 ms |
| An election day and the next: the tally and the clearing, declared sweeps of the country's agents; a campaign's first day, the opening sweep | 0.34 M agents | 40 ns; 25 ns; 30 ns | +5 ms; +3 ms; +3 ms | the same | +5 ms |
| A budget's effective day: kink signatures re-read, agents rewritten, kink days rebooked | 0.95 M; 0.1 M; 0.2 M | 5 ns; 300 ns; 100 ns | +18 ms | — | +18 ms |
| A property-tax instalment day: the holding levy from the (zone, class) index | 1.2 M holdings; 0.8 M legs | 20 ns; 15 ns | +12 ms | — | +12 ms |
| A property-tax assessment day; the school year's first day | 40 k valuations; 0.2 M agents | 1 µs; 200 ns | +13 ms each | +13 ms (school) | +13 ms each |
| A peg's break or a sudden stop: the wake pass and the woken agents' visits | 0.95 M; up to 0.7 M | 5 ns; 500 ns | +120 ms | — | +120 ms |

**Stage 6 adds** (the plan's Stage 6 ledger, after its reviews' re-costing and remedies, §18 item 27):

| Work | Count, business day | Unit | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- | --- |
| Meetings: singles counted per region; meetings drawn, two subjects each; `form`, every day (TIME.8) | 0.7 M agents; 20 k; 40 k | 3 ns; —; 80 ns | 3 ms | 3 ms | 3 ms |
| Occasion evaluations and choices: `enrol`, `separate`, leaving; courses, acceptances, new households' housing | 20 k; 25 k | 80 ns; 400 ns | 4 ms | — | 4 ms |
| Housing search for new households | 15 k agents | 1 µs | 5 ms | — | 5 ms |
| Leaving, formations (two origins each), separations, known ways (`divide`, `combine`; budgeted as the retired parts) | 25 k | 2.5 µs | 21 ms | — | 21 ms |
| The same at the prototype's measured 9.9 µs a part (the risk case, outside the totals) | 25 k | 9.9 µs | 83 ms | — | 83 ms |
| Research, imitation, licensing and learning: reviews, candidates, thresholds and powers, pools | 10 k; 5 k; 0.3 M | 300 ns; 180 ns; 5 ns | 4 ms | 1 ms | 5 ms |
| Firm agents with known ways of their own: their candidates, visits and dated rows | about 50 k agents | 1.5 µs (0.5; 2) | 25 ms | 8 ms | 33 ms |
| `choose_holdings` over the whole balance sheet | 30 k | 220 ns more | 2 ms | — | 2 ms |
| Families and measures: POP.11 and POP.12 on the rolling cycle; TEC.10; HH.16 | — | — | 2 ms | 1 ms | 3 ms |
| Views and pages on a turn's last day (tracers retired with cells) | — | — | 8 ms | 1 ms | 10 ms |
| **Stage 6 total** | | | **74 ms** | **14 ms** | **86 ms** |
| A country's school year's date: roles at a stage's end complete and choose | about 75 k | 100 + 80 + 400 ns | +15 ms | — | +15 ms |

| Turn | Days | Budget | Stage 1 | Headroom | Through Stage 2 | Headroom | Through Stage 3 | Headroom | Through Stage 4 | Headroom | Through Stage 5 | Headroom | Through Stage 6 | Headroom |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Ordinary weekday (the median turn) | 1 business | 1 000 ms | 924 ms | 7.6% | 1 010 ms | **misses by 1%** | 1 063 ms | **misses by 6%** | 1 165 ms | **misses by 16.5%** | 1 228 ms | **misses by 23%** | 1 302 ms | **misses by 30%** |
| Monday after a weekend (with carried needs and pending settlement) | 2 non-business + 1 business | 2 000 ms | 1 586 ms | 21% | 1 698 ms | 15% | 1 753 ms | 12% | 1 861 ms | 7% | 1 932 ms | 3% | 2 034 ms | **misses by 2%** |
| Heavy Monday (month- or quarter-end payday) | 2 non-business + 1 heavy | 2 000 ms | 2 063 ms | **misses by 3%** | 2 215 ms | **misses by 11%** | 2 303 ms | **misses by 15%** | 2 455 ms | **misses by 23%** | 2 562 ms | **misses by 28%** | 2 676 ms | **misses by 34%** |
| Heavy Monday with tolerance control (retired with cells) | 2 non-business + 1 heavy | 2 000 ms | 2 233 ms | **misses by 12%** | 2 385 ms | **misses by 19%** | 2 473 ms | **misses by 24%** | 2 625 ms | **misses by 31%** | 2 732 ms | **misses by 37%** | 2 846 ms | **misses by 42%** |
| The longest closed run: four days ending on an ordinary business day (once in 63 years) | 4 non-business + 1 business | 2 000 ms | 2 248 ms | **misses by 12%** | 2 386 ms | **misses by 19%** | 2 443 ms | **misses by 22%** | 2 557 ms | **misses by 28%** | 2 636 ms | **misses by 32%** | 2 766 ms | **misses by 38%** |
| Three days closed ending on a quarter-end payday (three times in 63 years) | 3 non-business + 1 heavy | 2 000 ms | 2 394 ms | **misses by 20%** | 2 559 ms | **misses by 28%** | 2 648 ms | **misses by 32%** | 2 803 ms | **misses by 40%** | 2 914 ms | **misses by 46%** | 3 042 ms | **misses by 52%** |

The candidate, redraw and seller-spread units were raised after the population engine's review measured untuned
prototypes on one x86 core (174 ns, 140 ns, 4.1 µs). The same prototype measured a part at 12.1 µs, of which 2.2 µs were
redraws that this table budgets in their own line, so **9.9 µs** against the 2.5 µs above, which is recorded as a
finding; parts are retired with cells, and what an agent's change in place costs is measured (§14.6). The Stage
1 review added the physical flows' realisations and the publication-day wake. Due-day runs for every dated row kind
(§6.5) take the ordinary day's settlement from 120 ms (30 M rows read at 10 ns and 2 M payments at 30 ns: 360 core-ms)
to about 48 ms ((1.1 M × 2 + 6.2 M × 10 + 4 M × 5 + 2 M × 30) ns ≈ 144 core-ms), with a unit for the head's maintenance;
on a heavy payday almost every holder has a due, so they save little there, and Stage 0's pensions in payment add about
7 ms. At these estimates the **median fits with 7.6%**, short of the required 10%; **heavy Mondays and the longest
closed runs miss**. Stage 2 adds about 86 ms to a business day, 13 ms to a non-business day and 126 ms to a heavy day,
so through Stage 2 the **median misses the budget itself by 1%**, and at the measured part cost Stage 2's parts alone
would have added about 143 ms more to it. Stage 3 adds about 53 ms to a business day, 1 ms to a
non-business day and 86 ms to a heavy day, with its representation choices already taken (§18 item 24): through Stage 3
the median is about **1 063 ms, 6% over the budget**, and a heavy Monday about 2 303 ms. Stage 4 adds
about 102 ms to a business day, 3 ms to a non-business day and 146 ms to a heavy day, with its own choices taken (§18
item 25): through Stage 4 the median is about **1 165 ms, 16.5% over the budget**, and a heavy Monday about 2 455 ms.
Stage 5 adds about 63 ms to a business day, 4 ms to a non-business day and 99 ms to a heavy day
after its reviews' remedies (77, 4 and 133 before them), with its choices taken (§18 item 26): through Stage 5 the
median is about **1 228 ms, 23% over the budget**, a heavy Monday about 2 562 ms, and an election's eve adds about 60 ms
in the largest country. Stage 6 adds about 74 ms to a business day, 14 ms to a non-business day and
86 ms to a heavy day after its reviews' remedies, with every known way kept (TEC.4) and its choices taken (§18 item 27):
through Stage 6, the whole world, the median is about **1 302 ms, 30% over the budget**, a Monday after a weekend about
2 034 ms, 2% over, and a heavy Monday about 2 676 ms. For the worst turn, three closed days before a
quarter-end payday, to keep 10% headroom the non-business day must cost at most (1 800 − 1 401) ÷ 3 ≈ **133 ms** at
Stage 1, two-fifths of the estimate, and through Stage 6 the business day must fall by about 400 ms for the median's
headroom. The closed runs are read from the declared calendars (TIME.2) by `phx measure calendar`
(`perf/measure/S0.11-calendar.json`), the calendars committed with their sources before the first read, so no holiday
rule was chosen with the budget in view (N8.9): over the 63-year window the longest run is four days, once, ending on an
ordinary business day; none of four days ends on a payday; three runs of three end on a quarter-end payday. At these
estimates the worst turn, not the median, binds first, so the worst-turn rows set the play resolution. Stage 0's
measurements decide: measured unit costs first, then a larger factor (fewer agents) and coarser zones, which are
RESOLUTION. If no play resolution meets the budget, that is a finding, and the budget is the owner's to decide (N8.7).
No causal date is moved (N8.9).

### 13.3 Storage (4 GB, N8.4)

A full save of the design point is about 1.5 GB after transforms (about 1.6 GB through Stage 2, about 1.7 GB through
Stage 3, about 1.85 GB through Stage 4, about 1.9 GB through Stage 5, about 1.95 GB through Stage 6). Every save
is full (spec Appendix E 22): an increment would have carried about two thirds of a full save, since a month's
paydays and dues touch nearly every holder's arena and record, which put a full save, its increment and the next full
save over 4 GB and an increment far past 1 s. The peak (§11), two full saves, is about 3.96 GB at Stage 6, 1%
headroom, a finding every gate checks on the device (N8.4).

---

## 14. Testing and measurement

### 14.1 Evidence

1. **Compile level**: distinct identifiers, currency-tagged money, typed handles, contexts that cannot reach later
   state or other parties' rows, `Missing<T>` without defaults.
2. **Logic level**: pure functions — samplers, rounding, contract legs, clearing, waterfalls, step functions,
   hazard draws, levies per contract, standing-flow posting — under `cargo test`. No test depends on
   `phx-world` or the opening.
3. **The live world**: `phx run` on the real world, settled, with the audit (N1), liveness (N2), the steps' live
   checks and the budgets.

### 14.2 Live checks

Every step declares **live checks** with permanent identifiers (`LC-…`), implemented in `phx-cli`'s check suite over a
live run's records and metrics. A retired check keeps its identifier and says why; `phx-check` fails if one
disappears. A check reads the world through its inspector (`Run::World`), or the world with what the observer recorded
beside it over the run — the reads' series and the opening distributions' drift (`Run::Observed`), as liveness and
GEN.8 do.

The realised rates (CHN.7) are measured live over a fixed sample of agents (`phx_world::rates`). An agent is sampled
when the mix of its identity is a multiple of `RATE_SAMPLE` (64). The sample is found in one sweep on the first day
measured, and an agent stays in it while it lives. Each day, for every process on its kind, each present person's
chance adds to the expected hits and to their variance, and each person reached adds one to the realised hits. The tallies are kept by process, calendar year and the person's whole years of age (`Rates`), and a
check holds each one to its expectation within its sampling error.

The suite is a hand-written `const CHECKS: &[Check]` in `phx-cli/src/checks/mod.rs`, each check's function and
metadata defined by `live_check!`; `phx-check` requires every defined id to be in the list, and `Inspector` to have no
public field and only `&self` methods.

### 14.3 Determinism by construction

The world is never run twice to prove it deterministic. Determinism is carried by construction and checked at compile
and logic level: chunking, gathers and reductions fixed by index, whatever the worker count (§6.2, `phx-exec`'s
tests); canonical ids, whatever the registration order (§6.2); a stream's key derived from its own name alone (CHN.1);
observers, the audit and the realism recorder reaching the world only through `&self` accessors (Law 17, PC-20,
PC-85). Every state that carries across days and can change an outcome is saved or canonical: a solver's warm start
is saved (§8), derived indexes are rebuilt in canonical order (§11), and the resolution valve is a setting of the
register, which the manifest's hash covers, changed only between runs or at a save boundary, never from the run's own
timing (§7.9). The build run checks that each save, decoded store by store without building a second world, hashes
to the world hash of the close it was written at (SET.15); the check is not meant to run on the phone, and as built it
runs only in `phx run`.

### 14.4 The measurement programme

| Command | Measures |
| --- | --- |
| `phx measure calendar` | the declared calendars' longest closed runs and their paydays after holidays (§13.2), into `perf/measure/` |
| `phx measure budget` | from a build run's report and the device report of the same commit's world: the phone's business and closed turns, each sub-step's median, memory (the opening's and the run's `VmHWM`), stage 7's unit costs — each phone turn's own 7c time, the block that runs all of stage 7, over that turn's own payments and rows scanned, the median and worst of those — and the full-load bench against its criteria |
| `phx measure` *(not built)* | read from the device run's report (§14.6), the representation's own numbers: agents and rows per kind, relationship rows per agent by line kind, persons per agent, bytes per store and peak resident bytes, agenda rows, draws and hits per process, occasions, agents changed and new agents per day by cause, legs per batch, rows and bytes per sub-step, unit costs per line of §13.2 |
| `phx inject` | audit independence (N1), on the build machine, on a save loaded apart, audited and discarded |
| `phx realism` *(Stage 7)* | the stylised facts (N3) read from the run: recording, statistics, verdicts, GEN.10 (§14.8) |
| `phx chains` *(Stage 7)* | the chains' relationships (N4) read from the run (§14.8) |
| `phx register-report` *(Stage 7)* | the primitive register's sources and the shares assumed and estimated (N7) |
| `phx run --report` | the run's audit, liveness, live checks, REP.15's report, GEN.8 and NUM.7 |

, and the population's age structure by `DEM.age_classes` and sex, births, the general fertility rate and school leavers

At Stage 0 `phx measure` has two subcommands, `calendar` and `budget`; the representation's numbers of the third row
are not yet read.

### 14.5 Gates

Every stage ends with: CI green, and a clean build run of the gate commit (§14.7), whose audit and live checks are the
gate's; the **device run**: the bench flavour, built by CI with fat LTO (§18 item 33) and carrying the inspector's
recorder (§14.8), runs a settled simulated year on the phone and exports its report and series,
and the owner commits the report to `perf/device/`; `phx measure` within the memory and time budgets; from Stage 1,
the stage's macro reads, which `phx-cli` reads from the device run's series, against real economies' relationships
(spec Appendix E 25), each miss a finding that does not block the gate. The budget is judged on that same run, so the
recorder's cost is inside it. The **go/no-go** reads the device report: the median turn ≤ 1 000 ms and the worst ≤
2 000 ms over the settled year, peak `VmHWM` and PSS ≤ 4.5 GB, a full save ≤ 5 s; §13's 10%
headroom is reported, and a gate passes without it only as a recorded finding. Stage 7's gate adds the realism reads
(§14.8): realism misses are findings and do not block it; the budget does (N8.8). As built at Stage 0 the device run
settles for the owner's length and then runs the turns the app passes, 60, not yet a year of them (§12).

From Stage 0's gate on, every gate also runs the **full-load bench** on the phone (§14.6, item 7) and holds it to the
same criteria: a gate does not pass without it, so a finished world too slow or too large is found at Stage 0, not at
Stage 6.

### 14.6 Measure first

Stage 0 carries the opening world's employment, tenancy, deposit and loan lines and its pensions in payment paying
as their terms say (spec Part O), so its world already has paydays, dues, pensions, deaths, illness, ageing and
catastrophes. Before any behaviour is built, the Stage 0 gate's device run measures these on the phone, its bench
flavour writing the counts and histograms each needs into its report, and `phx measure` reads them from that report
on the build machine:

1. **Rows per agent by line kind** — employment, tenancy, deposit, loan and the rest — and persons per agent, across
   **the one run's own agents**, at the play resolution only (spec Appendix E 40), at the opening and after a
   simulated year; banking arrangements per region. *(Pending: the bench flavour does not yet write
   these counts and `phx measure` does not read them.)*
2. **The phone**: core-seconds per second by core class across the run, with the thermal status; random-gather
   nanoseconds per row over 1–3 GB with 16 KiB pages and prefetch, with all cores gathering together; sweep
   bandwidth; barrier cost.
3. **An agent's change end to end** at the measured rows per agent, per component: its rows' recounts, holder-list
   maintenance, instruction (agenda redraws are item 5's); agents changed and new agents per day by cause.
4. **Settlement on the heaviest payday and on an ordinary weekday**: run heads read, the share of holders' due-day
   segments scanned and the rows in them not due, rows read and legs, nanoseconds per head, per row and per leg
   including levies, the fixed point's iterations, and the peak of the day buffers.
5. **Hazards and agenda**: draws, redraws and agenda rows per day, with their unit costs; `NextDays` reasons
   per table.
6. **The worst turn**: the worst closed run in the declared calendars (§13.2), its closed days at the measured
   non-business day and the day that ends it at its measured type.
7. **The finished world's load**: the full-load bench, in the bench flavour after the world's year. It allocates the
   population its representation holds at the play resolution with every store at the finished world's size (§13.1's
   Stage 1–6 lines), fills it with random data from its own seeded stream, outside the world's, and runs a simulated
   month at most, holding each of the calendars' day types — ordinary, the Monday after a weekend, the heavy Monday, the
   worst closed run — with the real kernels at each day type's finished-world counts (§13.2): settlement, hazard draws
   and the agenda, each visit's gathers with its ledger's arithmetic for the mechanisms not yet built, the audit, the
   views and full saves. It is judged by the gate's criteria (§14.5); each later gate reruns it with the built stages'
   measured counts. Its numbers are costs, never the world's. As built (`phx-ffi`'s `load.rs` over
   `perf/load/volumes.toml`), it runs after the world's turns in `Run all`, on the pool; its books are built by the
   random-data generator `phx_ledger::synthetic`, and its household agents — random persons and attachments in a real
   agent table — by `phx-ffi`'s `agents.rs`. It settles each business day through `Books::settle_day` and closes the
   ledger's day, draws agents' next hits through `phx_pop::hazard` on the pool, and applies hits' outcomes by making
   households explicit and writing them back (`phx_pop::explicit`); the audit is a sequential sweep of a slice of the
   random words and of the agents' parties, not the real families; the agenda, redraws and each visit's gathers
   are reads and writes of random words; there are no views, members leaving, estates or 9b posting. The
   device report measures an agent's next hit drawn and a hit's outcome applied on the fastest core.

Stage 1's gate adds retail and labour: draws, sellers in reach, occasion evaluations and choices by decision,
vacancies visible and labour rounds, surprise wakes, physical realisations, outlook methods in use, and agents per
stance. From these, §13 is
rewritten with measured numbers for every stage — the measured unit costs times each later stage's ledger counts — so
each gate reports the whole world's projection, not only its own stage's, beside the full-load bench that decides it.
If they do not fit, the remedies are, in order (N8.7): how the world is represented and traversed; then the play
resolution — the size and the zones. If none suffices, that is a finding, and the owner decides; no mechanism is
weakened. Before Stage 4's steps are built, `phx measure` also measures policy rows per (agent, cover) on GEN's draws
(the plan's S4.03) and the rows per agent of defined-benefit rights and DC pots (S4.04).

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test` in the debug and the release profile; `phx-check`; `miri` over `phx-store`'s tests on the pinned nightly; `public-api`, the kernel and interface crates' API snapshots; the workspace built for Android; `android-app`, the engine built for the phone with the `device` profile, its Kotlin bindings and the bench flavour's APK, uploaded as an artifact (§18 item 33); kernel micro-benchmark instruction counts under valgrind, whose ratchets fail CI. The world is not run |

Every job runs on push and pull request on a pinned runner image with a 30-minute timeout, reads its tools' versions
from `tools/versions.toml`, caches on `Cargo.lock` and the toolchains, and may not fail. CODEOWNERS gives the owner
`/perf/` (but `perf/build-run/`), `docs/PROJECT_PHOENIX.md` and `data/**/gen/`.

**The build run** (`tools/build-run.sh`, on the build machine — the development VM, 4 cores and 15 GB): a `--release`
build (thin LTO; only the `device` profile is fat, §17); the world at the play resolution (§10.5), with `read-trace` on,
the audit and every live check, 120 days from day zero; the engine's counter ratchets, whose moves fail the build run,
not CI (§16.8); peak memory; wall time, build and run apart. It uses no other resolution or seed, and its numbers are the
code's, never the world's. It is **clean** when the audit reports nothing, every failing live check is recorded as a
finding in the plan's §11, and no ratchet moves the wrong way. Since S1.09 a step is done on its fast checks and the
world runs at the gates; a stage gate's settled two-year run is the device run, on the phone (owner, 2026-09-28).

**The smoke** (`tools/smoke.sh`) reads the budget at every step: twenty days from day zero at the committed resolution,
no live checks, the report's `budget` block — the turns' median and worst wall time, peak bytes a person, cores busy
(the process's CPU time over the days' wall time) and minor page faults a day — against `perf/budget.toml`'s ratchets,
which may only fall (the cores only rise). A step that must raise one says so and why. `--bench` adds the full-load
bench. The first measure, 2026-09-28 at 750 000 persons: a median turn of 8.0 s, the worst 26.2 s, 6 748 bytes a person,
1.13 cores busy and 138 057 faults a day; 7c 3.4 s, 10d 1.2 s and 6d 1.1 s of the median day.

### 14.8 The realism reads

Stage 7 measures the world's one run (N3, N4, N7; spec Appendix E 36): the normal world run at the play resolution,
opened from one year of settling and read as it is played on the phone. Nothing is re-run, and nothing is compared
with another run. A fact the run has not yet had time to produce is *not yet credited*; a slow distribution is
credited while the world holds it (GEN.10).

- **The recorder**, in `phx-obs`'s inspector build only (at Stage 0 always compiled, §12), reads the run through the
  `Inspector` at each close and writes the series the registered definitions name, outside the save, exported with it;
  it opens no stream and writes nothing to the world (Law 17).
- **Statistics** are computed from those series as a statistician computes them from real data, each with its
  estimator's interval from the run's own sample, and compared per country with the benchmark range its source gives.
- **Chains** (N4) are read as relationships between macro variables — responses around the run's own dated
  occurrences of each chain's first link, lead–lag correlations and regressions — against their benchmarks.
- **Pre-registration**: every definition's hash is named by the report that uses it, and must be an ancestor of that
  report's commit; reports are append-only.
- **No tuning**: the register is diffed by id between commits; each changed primitive, opening distribution or rule
  form cites its source, and a RESOLUTION change cites a device or measurement report, never a realism result.

---

## 15. Guards on the running world

- **The audit** (N1) streams over batches as they apply and checks against **independent records**: per-batch digests
  kept at apply time; issuers' own totals per deposit and loan line; issued amounts per instrument; banks' maintained
  totals of standing flows against their depositors' rates on a rolling cycle. Injection mode lights each family
  alone (`phx inject`).
- **The runner** (`phx-audit`'s `Audit`) runs every declared family at each close, at 10d (`AUDIT_SUBSTEP`), after the
  day's last write, and keeps a `CloseRecord` (families run, rows checked, findings, instructions applied). A family is
  declared by the crate or system that owns its facts (`FamilyDecl`: name, owner, clause, mode), reads only through
  `FamilyCtx`, whose methods are all `&self` reads — the register, directory, calendar, records, events, the day's
  messages, kernel tables, the rows the day's applies touched (`TouchedRows`, a bitmap per table beside the audit
  sink), the records and events written since the last close, the day's read trace, the books, markets, accounts and
  agents through their audit traits, and the settled legs — and writes only `Finding`s. Its mode is `Streaming` (fed
  by the apply through `AuditStream`), `Incremental` (what the day touched and wrote) or `Rolling { cycle_days }`, which
  adds `rolling_slice`: the domain in slot order cut into `cycle_days` equal runs, each day taking the run its number
  modulo the cycle names. A finding carries its family, clause, owner (a party, tile, line, instrument, market,
  country, event, table, or the run itself), size in money, units, days or a count, and day. Each family declares its
  injection (`AuditFamily::inject` over `InjectTarget`), a one-unit change to a fact only it owns.
- **Names** (`PTY.names`, rolling over thirty days): every party a record, event or message names resolves through the
  directory, on what the day wrote and a slice of the rest. **Time** (`TIME.stamps`, incremental): every record, event
  and message the day wrote is stamped with the day and a sub-step before the audit's, a stamp on another day sized in
  days and a late one in sub-steps; the read trace's findings, when it is on, are reported here.


  A finding is `{ family, clause, owner (party, tile, line, instrument, market or country), size, unit, day, detail }`.
  Findings, metrics and run records are kept outside the world: not hashed, and no handler can read them (Law 17).
  A family reads through `FamilyCtx`, whose methods take only `&self`, and writes only findings; kernel crates below
  `phx-audit` declare theirs in `phx-core`.
- **Live checks** and the save check as above.

---

## 16. Drift guards

`phx-check` and clippy fail the build on:

1. **Layering** (§3.1), the external allow-list, rayon only in `phx-exec`, `unsafe` only in `phx-store`, `phx-exec`
   and `phx-ffi` and in `#[derive(Pod)]`'s expansion.
2. **Type-aware rules** (clippy disallowed lists over world crates): `min`, `max`, `clamp` on numbers; `Instant::now`,
   `SystemTime::now`; `RandomState`, std `HashMap`/`HashSet`; atomics, `Mutex`, `RwLock`, `OnceLock`, `LazyLock`,
   `thread_local!`; `println!`. Declared real limits go through `DeclaredLimit::bind` (Law 6), built from the
   register, a contract's terms or a physical token (holdings and stock, for capacities). The count of
   `allow(clippy::disallowed_*)` in world crates only falls.
3. **Structural rules** (`phx-check`): no `static` items in world crates; no heap-owning types in stores; numeric
   literals only 0, 1, −1 and 2 in mechanisms, engineering constants in one `consts` item per crate; no clause
   identifiers in comments; interface crates without behaviour; no full sweep in a sub-step not declared as one
   (*not built*: no rule refuses an undeclared full sweep yet).

; no forecast by running the world (`forecasts`): `phx-val` depends on nothing that holds the world, and no function of
it takes the world, a table or a handler's context.
4. **Types that refuse**: `Money`, `Qty`, `Missing` without `Default` or clamping; kind identifiers without equality
   outside the kernel; private-constructed handles; zero-sized systems.
5. **The clause map**: every spec clause is assigned to a step in `IMPLEMENTATION.md`, and `phx-check clauses` refuses a
   live clause the map omits, one two rows complete, and a mapped clause that is retired or unknown. For a step marked
   done, each clause it completes must have a **carrier** in the code or data: a `#[clause(..)]` attribute, a
   declaration's `clause` field, a data file's `clause` key, or a contract's `violation!(clause = ..)`
   (`crates/apps/phx-check/src/clauses.rs`). The carrier's shape — STATE → store, fact or type; DECISION → decision
   point; PROCESS → handler; INVARIANT → audit family; MEASURE → metric; FORBID → a check, clippy rule or type-level
   refusal; PRIMITIVE → register entries — is not checked: `#[clause]` checks only that each name is a clause
   identifier, and there is no `phx dump-registry`.
6. **Documents**: the coverage table (§19) regenerated and matching; every step with all its sections and a status;
   this document's crate lists matching the workspace.
7. **Process**: live-check identifiers never disappear; a primitive's value in `data/` changes only with its `source`
   in the same diff; the placeholder count only falls except by placeholders a stage introduces, and no system is done
   while a placeholder naming it remains; public-API snapshots of kernel and interface crates change only with the
   change that needs them; `perf/` changes need the owner's review (CODEOWNERS), except the append-only build-run
   reports (§14.7).
8. **Ratchets** on deterministic counters (§14.7): kernel instruction counts, in CI; in the build run, bytes per
   store, per row and at peak; rows and bytes touched per sub-step; agenda rows, draws, occasions, agents changed and
   new agents per day; legs per batch; barriers per day; agents per kind; rows per agent by line kind.
   Declared-but-never-read primitives, streams and hazards are reported. As built at Stage 0 (`perf/ratchets.toml`,
   `phx-cli`'s `run.rs`): CI ratchets the kernels' instruction counts; `phx-check` the counts of `allow` and `expect`
   attributes and of placeholders; the build run only the barriers per empty day, the map's bytes and seven stage-7
   counters, each the most on any day — rows streamed, run heads read, run rows scanned and those not due, payments,
   the fixed point's iterations and the day buffers' peak bytes. The day's draws, redraws, outcomes and new agents are
   the world's day records (`AgentDay`), which the live checks read, neither reported nor
   ratcheted; the other counters are not built.
9. **The markets' rules** (the plan's PC-50 to PC-57): no bank's loan or deposit pricing takes a central-bank rate as
   its cost of funds except through `marginal_cost_of_funds`, while public administered rates may be read; no system
   calls a matching, clearing or allocation function of `phx-market`, whose meeting handlers alone meet markets,
   administered ones among them; a primary issue's participants rule refuses its own sovereign's central bank; an
   offer of held units takes only `Covered<Qty>`; a margin requirement is built from one (broker, client) account; a
   fund's dealing price is written only by its 9b handler from marks and labelled valuations; a benchmark fixing is
   built only from the day's match sets; a rating's or estimate's view has no print, mark or market-index field.

10. **The realism reads' rules** (the plan's PC-90 to PC-93, §14.8): pre-registration by ancestry, with append-only
    reports; no tuning, by a register diff by id with `Primitive-Change`, `Primitive-Rename` and `Resolution-Change`
    trailers, read from `main`'s first-parent history so squash merges keep them; measurement code reaching the world
    only through the `Inspector`; every silent break of Part L mapped to what refuses it.

    `coverage` derives §19's rows whose System is a spec code: the first stage is the earliest stage of a step whose
    clauses name the system's, the stage complete is the latest of its map rows, and the status is `planned` while no
    step naming its clauses is building or done, `done` when every step completing them is done, and `building`
    otherwise; the Spec and Crate columns and the rows that are not a system's are kept as written.

11. **`phx-check` itself** (`crates/apps/phx-check`) parses world crates with `syn` (with a small lexer for comments,
    which `syn` drops) and reads `cargo metadata` for direct dependencies; no rule is a text search where either can
    answer. World crates are those under `foundation/`, `kernel/`, `interfaces/`, `systems/` and `assembly/`; `apps/`
    are held only to layering. Its rule table (`src/rules/mod.rs`) records each rule's id, title and the step that
    introduced it; the rules of the foundation and first kernel crates are:

    | Id | Refuses |
    | --- | --- |
    | PC-01 | a dependency against §3.1's layers and orders |
    | PC-02 | a direct external dependency outside the allow-list (`rules/dependencies.rs`, by crate or layer; `proptest`, `trybuild` and `gungraun` as development dependencies anywhere) |
    | PC-03 | `allow(unsafe_code)` outside `phx-store`, `phx-exec` and `phx-ffi` |
    | PC-04 | `rayon` anywhere, `rayon-core` outside `phx-exec`, `libc` outside `phx-exec` and `phx-store` |
    | PC-05 | a `static` item in a world crate, but `phx-exec/src/site.rs`'s thread-local |
    | PC-06 | a numeric literal other than 0, 1, −1 and 2 in a world crate outside `consts.rs`, type positions, tests and benches, with the arguments of the assert, format, write, `vec!` and `violation!` macros parsed as expressions; a constant in `consts.rs` without a doc comment |
    | PC-07 | a comment matching a clause id, `Law n`, `Nn`, `§`, `spec`, a document's name, a plan step, `TODO` or `FIXME` |
    | PC-08 | a function in an interface crate other than a constructor or a field accessor |
    | PC-09 | a crate missing from §3's lists; a plan step without a valid status or its sections in order (a retired one needs only its status); two steps building; a crate whose step, still in the plan, is not yet building |
    | PC-10 | counts of `allow` and `expect` attributes in world crates above their ratchets |
    | PC-11 | an `#[expect]` without a reason |
    | PC-12 | a hand-written `impl Pod` or `Sealed` outside `phx-store/src/pod.rs`, and a float in a derived `Pod` |
    | PC-13 | a direct dependency of a world crate on `rand`, `rand_core`, `getrandom`, `ahash` or `fxhash` |
    | PC-14 | `Default` on an id of `phx-id` |
    | PC-15 | a kernel or interface crate without a committed `public-api.txt`, or whose API differs from it |
    | PC-16 | a per-crate `clippy.toml` other than the root file minus that crate's declared exemptions |
    | PC-17 | outside `phx-id` and `phx-core`'s `calendar/`, a call of `days_from_civil`/`civil_from_days` or a number added to or taken from a day |
    | PC-18 | a primitive's value reached other than through `Prim` or `PolicyValue`; `TermsToken::new` outside `phx-ledger`, `PhysicalToken::new` outside `phx-ledger` and `phx-geo`; `toml` or `serde` in a world crate other than `phx-core`'s `register/` and the data readers (`phx-world`, `phx-obs`, `phx-cli`, `phx-ffi`); committed data outside its places; and the placeholder SHAPEs of `data/` above their ratchet |
    | PC-19 | `Draws::new` outside `phx-rand` and `phx-core`'s `streams.rs`; `Streams`, `open_keyed` and `ObserverDraws` named outside their listed files (§5.3) |

    The clippy exemptions are declared per crate in `phx-check` and realised by that crate's `clippy.toml`: `phx-exec`
    the atomics, `std::thread::spawn` and `thread_local!`; `phx-rand`, `phx-store` and `phx-exec` the wrapping
    integer methods, through named helpers; `phx-cli` `Instant::now`, `env::var` and the printing macros;
    `phx-check` the printing macros; `phx-ffi` `Instant::now`. Its subcommands: `layering` (PC-01 to PC-04), `rules`
    (the table), `docs` (PC-09 and §19 against the table `coverage` would write), `clauses` (the clause map and
    carriers, item 5), `coverage [--write]`, `all`; `bench-ratchets` (the `gungraun` summaries against
    `perf/ratchets.toml`, a count with no entry refused) and `public-api [--write]` (with the pinned
    `cargo-public-api` and nightly) run in their own CI jobs; `check-all` (the cargo alias `check-all`) runs format,
    clippy and tests as CI does, then `all`. `phx-check all` also refuses a committed device report that does not
    validate against `perf/schema/device-report.json`.

A rule changes only with its reason recorded in §18.

---

## 17. Build and target

- Release: `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` with a hook that writes the violation report,
  overflow checks on; the build run builds it. The `device` profile, which CI builds the phone's library with,
  inherits it with `lto = "fat"`.

The hook (`phx-cli`'s `panic_hook`) writes `violations/<run>.json`: the clause or the capacity exceeded, its keys, and
the site the day runner last entered (`phx_exec::site`: day, sub-step, handler, chunk). `violation!` builds a
`phx_num::Violation` — the clause, a message and at most eight keys as `i128` — and raises it with
`std::panic::panic_any`; `capacity_exceeded!` does the same with its own payload. The hook, on the panicking thread,
adds the site from `phx_exec::site::current()`: the (day, sub-step, handler, chunk) each thread records in the engine's
one `thread_local!` as it starts a chunk or an apply. Stores refuse to compile on a big-endian target, since saves and
hashes read their bytes as little-endian.

- The workspace is `crates/*/*` under resolver 3, edition 2024, with `rust-version` equal to the pinned toolchain
  (`rust-toolchain.toml`: components `rustfmt`, `clippy`, `rust-src`; targets x86-64 Linux, aarch64 Linux and
  `aarch64-linux-android`; `rust-toolchain-miri.toml` pins the nightly only the `miri` job uses). `dev` and `test`
  keep overflow checks on; `release` carries line tables only; `bench` inherits `release`. The phone's link passes
  `-z max-page-size=16384`. `rustfmt.toml`: width 120, `use_small_heuristics = "Max"`, Unix newlines.
- Every crate inherits `[workspace.lints]`: `unsafe_code` denied (lowered only in the three crates of §3.1),
  `missing_debug_implementations` warned; clippy's `all` and `pedantic` denied, with `undocumented_unsafe_blocks`,
  the lossy and sign casts, `as_conversions`, `float_cmp`, `unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing`, `allow_attributes` and `allow_attributes_without_reason` denied one by one. An exception is
  `#[expect(lint, reason = "…")]`, never `#[allow]`, and no lint is turned off for a whole crate. Tests may unwrap,
  expect, panic and index (`clippy.toml`).

- Phone target features: `+lse,+rcpc,+dotprod,+fp16`; NEON by auto-vectorisation and in `phx-rand`'s samplers;
  64-byte aligned, padded columns; software prefetch on holder-list and index gathers.
- Pool sized to fast and medium cores; performance-hint sessions per turn. As built at Stage 0 the world runs on one
  thread and no performance-hint session is opened (§2).

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
10. Settlement: implicit batches never materialised, but for the day's payments 7a hands 7c; a streamed payer pass;
    the greatest fixed point; failure per payer (§6.5).
11. **Tolerances are steps** (REP.4, amended): landing is one lookup of the landing key, a check of the lines' kinks,
    batches per target and clusters of the unlanded (§7.6).
12. Open business pins only what belongs to particular members; balances are positions with steps (§4.2).
13. A trade is one composite instruction drawing on other systems' commitments (§4.4).
14. Canonical two-pass drawing, households drawn and counterparties derived, makes the same world at any setting of
    the resolution valve (§10).
15. Saving pauses the world at declared moments (§11), as SET.12 and N8.10 allow.
16. Demands due today settle in stage 2 so consequences fall the same day (TIME.6, TIME.7); a failed bank is resolved
    at the next business day, its transfer settling at stage 7 (§9.2).
17. A turn runs to the next business day in any country; each country's markets follow its own calendar; non-business
    days run what TIME.8 (amended) lists (§6.1).
18. `sys-dem` carries the spec's POP.
19. Freight within each country arrives in Stage 1; firms, banks and central banks exist as parties from Stage 0
    (Part O records both).
20. **Measure first**: Stage 0 carries the opening lines and pensions in payment paying as their terms say, and the
    phone, the rows-per-cell curve, the unit costs and the worst closed run decide the play resolution before
    behaviour is built (§14.6). The design point's estimate leaves the median 7.6% headroom at Stage 1, short of
    10%, and misses it through Stages 2 to 6; it misses heavy days, tolerance control on a heavy day and the worst
    closed runs, and misses memory by 1.4% through Stage 3, by 10% through Stage 4, by 13% through Stage 5 and by
    16% through Stage 6 (§13).
21. Memory budget 4.5 GB, the owner's choice after the design point was sized.
22. **Owner decisions** (spec Appendix E 29–31): the map is about 40,000 tiles of 10 km with 25 regions allotted by
    the setup's population split (§10.0); the
    world runs once, and the accuracy for play is judged by its own macro relationships against real economies'
    (Appendix E 30, 36); the representation is coarsened for the phone (pooled flows, coarser employment lines,
    reviews on review days, sellers spread on review days). World settings: the settling length defaults to **one
    simulated year** (GEN.6, adjustable); saves default to **every simulated quarter** (SET.12), and every save is
    full and takes at most **5 s** on the phone (N8.10, spec Appendix E 22).
23. **Stage 2's decisions**:
    - invoices accrue per statement period, one row per (holder, market, terms, period), dated rows in the holder's
      due-day run behind the head the settlement stream reads (§6.5); a match may draw a commitment at 6d,
      writing a `Row` leg in place of the money leg (§4.4);
    - a housing transaction's pins ride on one part (§4.2), and a bank switch is made when its transfer settles
      (§4.5);
    - a resolution takes the book from the day's statement, is selected by the authority's least-cost rule, re-keys
      once per distinct key, and takes the next bid or the payout path at D+2 when its transfer fails (§9.2); a closed
      bank's legs are **pending**, a leg's third state (§6.5);
    - a stay is a line transfer to procedure lines (§4.4); estates are one per (part, occasion), and a personal
      insolvency's estate ends after its sale, a trustee paying the creditors from the income levy (§9.1);
    - indexed standing flows are tested against kinks on each day they post, with no envelope (§7.4); the coupled call
      is an exact min-cost flow, the lines' losses bought outside it at balancing;
    - stage 9 applies its intents at 9e, and each test is one handler with the consequence it triggers (§6.1);
    - a decision point lives in the decider's crate or the latest crate its types need; a lender's `LoanAssessment`
      carries a writer token only `sys-bnk` can build (§3.1, §3.4);
    - MMK.1's bilateral term loans are brought forward to Stage 2 on one interbank loan line kind, which the money
      market extends at Stage 3.

    Through Stage 2 the design point misses the 10% headroom on memory and on time, and the median turn misses the
    budget itself (§13); Stage 1's and Stage 2's gates measure before anything else is decided.
24. **Stage 3's decisions**:
    - each country's money market and the central bank's tenders meet in one **linked call** a business day: the
      coupled call with a cost on an edge and a capacity on a node, kept a pure min-cost flow on integers because each
      borrower assigns its collateral to segments before the call at those segments' lenders' haircuts; warm-started
      from the last basis over a pruned network, one core per country (§8, §6.1);
    - stage 6 fits the curve and the valuation inputs derived from 6b's fixings at **6c**, before its apply at
      **6d**; instruments' fixings are made at 6b by `phx-market` by the pricing service's declared method, and
      benchmark reference rates at 9d by their administrator's handler in `sys-idx`, only from 8b's match sets;
      funds' values are computed at 9b,
      consolidated statements are a pure `phx-acct` read at 9c, reports are read at 9c and published at 9d, where
      analysts read them (§6.1, §8);
    - fund dealing is forward-priced, administered and met at 6a, MKT.16's one stated exception; the facilities,
      lender of last resort and the treasury's direct borrowing are met at 8d by `phx-market`'s administered form
      within their declared limits (§6.1);
    - a cell's **participation** per asset class is three bits of its key, at most 8× the distinct keys per base key,
      measured in the run; its instruments are **rows with a member count**, each row's per-member quantity a
      position with steps; a trade settling after its fill makes its part at the fill and is re-keyed in place at
      settlement (§4.2, §4.5, §7.7);
    - instrument outlooks and values are computed per **registered** (method, instrument) pair on days with a new
      print and shared by the cells using the method; claim values are closed-form, over a discount-factor table per
      curve and day; individuals catch up in O(1) (§8);
    - the instrument's state has one writer, `phx-ledger`'s instrument events; an offer of held units is a
      `Covered<Qty>` that places a commitment on them (§4.4, §4.5);
    - each decision point's rule is registered by its owning system, except that a lender valuing a claim on its book
      does so through `sys-bnk`; the rating scale is `if-base`'s, `fund_position` is `if-state`'s, and `bank_choice`
      is `if-securities`' (§3.1, §3.4);
    - a dealer is a party of the dealer form with its own equity; publishers are large firms with a publisher facet
      (§4.1); a fund ends by L3's trigger through its own procedure, and finance companies and desks as firms (§9.1).

    Through Stage 3 the design point misses the median by 6% and memory by 1.4% (§13); the remedy planned first is
    how the world is represented and traversed, measured at Stage 1's and Stage 2's gates.

25. **Stage 4's decisions**:
    - derivatives are lines between individuals only, each line kind declaring its holder kinds; rows carry no
      `amount`, each margin account keeping the day its variation margin last settled; a house's initial margin is
      one blocked product over its accounts' sensitivities; admission hooks take a member's whole order set (§3.4,
      §4.5, §8);
    - the contract algebra gains valued losses, benefits while a state lasts, the elective leg (which a convertible's
      conversion also uses), a contract's underlying, and balances in a declared unit that is not money; the split at
      a kink works over any row kind's per-member position (§4.4);
    - a levy takes its schedule from a policy value, the flow's line terms or the payee's fact, and a follow-on may
      write rights in a unit that is not money (§4.3);
    - every dated row kind sits in its holder's due-day run behind one head in the record (8 bytes in a cell's, 12 in
      an individual's, §4.5), never reordered;
      retail many-party lines — policies, annuities, claimants, benefits and pensions — keep no holder list on their
      retail side, the stream gathering the day's due holders and a rare line-major day scanning the arenas once
      (§4.5, §6.5);
    - a job's pension kind and rates are its line's terms and the scheme a member belongs to is its attachment's, so
      employment lines are not split by scheme; DB rights and DC pots are positions with steps, and DC has no
      membership row (§4.5, §7.7, §7.8);
    - policy lines are many-party, the renewal band an attachment; actuaries project once per (cover, model point)
      and aggregate per (insurer, model point), and schemes are valued per (line, holder's age class) in a declared
      sweep on valuation dates (§4.5, §8);
    - a hit carries its members' profile values to the systems a process declares interested, applied at 3e; victims
      of harm to third parties are drawn from the (zone, class) index and the zone's pieces (§7.3, §7.8);
    - resolution of insurers and clearing houses is `sys-sup`'s on §9.2's timetable: a closed payer's legs are
      pending, a closed many-party payer's holders are paired once for the whole resolution, and a house recovers by
      haircutting its pending legs before its service is transferred or wound down (§6.5, §9.1, §9.2);
    - pensions in payment execute from Stage 0 (spec Part O), and the statistics agency publishes a period life table
      that insurers, households and actuaries read (§14.6).

    Through Stage 4 the design point misses the median by 16.5% and memory by 10% (§13); the
    remedies are N8.7's, representation and traversal first, measured at each gate before the play resolution is
    touched.

26. **Stage 5's decisions**:
    - taxes are levies per member where their bases arise; the levies of one flow share one search of their fused
      kinks; value-added tax on a sale on terms is computed per invoice row at its statement and on a cash sale rides
      the sale's instruction; property tax is a holding levy driven from the (zone, class) index, joining the
      holder's existing leg; duty and import tax at a border are a demand customs issues, not a levy; returns are
      day-local (§4.2, §4.3, §7.8);
    - a policy schedule's count of bands is the constitution's and fixed at compilation, so a budget moves values and
      edges only; an effective day re-keys only the cells whose signature words the moved kinks touch (§5.3, §7.6);
    - waits and pending claims are attachments, not pins; benefit rows are dated rows with no retail holder list;
      the state pension's earnings-related rights are one follow-on leg per cell per payday (§4.2, §4.5, §7.5);
    - vote intentions are a windowed profile group, written and cleared by declared sweeps `phx-pop` runs at 10b; the
      `vote` review's exposure and attention live in a side column only while a country's campaign runs; a platform's
      value is a key-and-step part per (landing key, platform) plus shared benefit and service parts, evaluation at
      the step a tolerance of REP; the tally is a declared sweep at 10a, which runs every day, and
      renumbering by country keeps the sweeps contiguous (§6.1, §7.2, §7.3, §7.8);
    - no system registers a handler at a kernel apply: intents, tallies and sweeps are declared and the kernel runs
      them (§6.1, §6.2);
    - money changes currency only by a trade: a bank is the counterparty of its customers' conversions, from its own
      currency book, drawn at 7a as a conversion commitment that a capital rule's handle gates; the payer pass and
      nets are per currency, and a leg whose currency has no stage 7 that day waits, pending, for a business day of
      both; a fixing converts nothing, and `phx-acct`'s translation is the one translation (§6.5, §8);
    - the balance of payments is fed by declared per-reason tallies, a per-category vector per payer at 7a added for
      the survivors at 7c; `Row` legs on cross-border lines are financial account (§6.5);
    - one closure until Stage 5: `XB.closed_borders` in `phx-market`'s `Reach`, retired in two parts; distances cross
      a border only through declared crossings, zone to crossing (§7.9, §7.10);
    - a peg's break is the central bank's fact that its quote's selling side bound, the regime staying with its
      owner; currency derivatives are marked per (pair, maturity) from 6c's forward points (§8);
    - parties are paid per vote by the treasury and stand on a deposit, employ staff and buy polls; pollsters are
      ordinary firms with the publisher facet; forms another system wrote are registered for new kinds by that system
      (§3.4, §4.1);
    - migration is the upper nest of the housing review's occasion, its inclusive values memoised per (outlook
      method, occupation-family set, destination) a day (§7.3).

    Through Stage 5 the design point misses the median by 23%, a heavy Monday by 28% and memory by 13% (§13); the
    remedies are N8.7's, representation and traversal first, then the play resolution, a valve set
    by measurement (spec Appendix E 36).

27. **Stage 6's decisions**:
    - ways are issued at runtime, an improvement stored against its base as factors, so a recipe is one read; a
      `WayId` is issued only from a discovery or imitation event and never reused; a way's owner and patent end are
      its patent holding's (§3.4, §4.5);
    - every firm knows its industry's public ways, held once per industry (TEC.4, spec Appendix E 42); a firm's own
      known ways stay whole in its key (dominance at today's prices does not last when prices can be negative),
      `sys-tec` their one writer; the firm cells distinct sets keep apart are a measured floor, and a
      profile of known ways not run is the proposed lever (§7.7);
    - cumulative output per way run is a keyed position list in the firm's arena, stepped logarithmically with the
      learning curve's kink declared, so learning re-keys in place; the curve's power runs only past its next
      rounding threshold (§4.5, §7.6);
    - skill and the labour-market state are reads of a role's profile and clocks (the employment's start band, the
      search band), skill written in place only when an origin changes: no hazard, no agenda reason, no daily work
      (§7.8);
    - the education record is its own profile group; compulsory school stages are a read of birth year and the law;
      courses and waits are attachments changed in place (§7.8);
    - meetings are one hazard per region over its singles' counts, each occurrence an event with two subjects drawn by
      pairing and a day-local `Meeting` message to each; `form` runs on the meeting's day (§7.3);
    - a formation is one `combine`d part from two origins, a leaving or separation one `divide`d part; a new
      household's key attributes come from their declaring systems' `.at_formation` handles, its preference type
      drawn, and nothing but money and units adds (§7.5, §9.1);
    - an adult leaving to move runs `migrate` before `where_to_live`, as any mover (§6.1);
    - POP.11 and POP.12 run on the audit's rolling cycle by region;
    - two builds, the participant's through `ParticipantScope` and the inspector's behind a feature; every shown
      number a `Shown<T>` from its source; views and pages on a turn's last day, tracers every day; tracers' older
      histories as change entries in one store beside the saves (§11, §12).

    Through Stage 6, the whole world, the design point misses the median by 30%, a heavy Monday by 34% and memory by
    16%, and two full saves keep about 1% of storage headroom (§13); the remedies are
    N8.7's, representation and traversal first, then the play resolution, a valve set by measurement (spec Appendix
    E 36).

28. **Stage 7's decisions** (§14.8):
    - the world runs once (spec Appendix E 36): the realism reads come from the normal world run and nothing is
      re-run, copied or compared with another run;
    - facts and chains are read as a statistician reads real data, each statistic with its estimator's interval from
      the run's own sample; verdicts are per country; the credit classes S
      (held) and B (produced) are read from the run;
    - pre-registration by ancestry, append-only reports, and no tuning by a register diff by id, a RESOLUTION change
      citing only measurements of the budget;
    - realism misses are findings; the budget alone blocks the gate (N8.8).

29. **One run, a short opening** (spec Appendix E 36, 38; the plan's §12):
    - the world is never run twice, not even to test the code: determinism is carried by construction (§14.3);
    - the world runs off the phone only on the build machine, after each step's build, at the play resolution; CI
      builds and tests, never runs it (§14.7);
    - the opening is one date's snapshot of the present, contracts' pasts on the steady-path convention; every party
      decides once on day zero, at stage 5, and the one settling year is the only history (§10.4); slow distributions
      are credited while held and moved, behaviour once produced (GEN.10);
    - what the run has not yet produced never blocks a gate (spec Appendix E 39), and the representation is measured
      only at the play resolution (Appendix E 40).

30. **The apply model, the audit sink, saved state and the build run**:
    - an intent applies at the first apply point at or after its sub-step; the kernel applies (2f, 4b, 5d, 6d, 7c,
      9e, 10b) are a sub-step kind of their own that runs every day its stage runs, and assembly refuses a system
      handler there (§5.4, §6.2);
    - the audit sink is `phx-core`'s `AuditStream`, implemented by `phx-audit` and injected by `phx-world`, so the
      ledger's apply feeds the audit from below it (§3.3, §6.4);
    - every state that carries across days and can change an outcome is saved or canonical: the linked call's basis
      is saved, and the valve changes only at a save boundary (§8, §11, §14.3);
    - every save is full, so the retention peak is two full saves, about 3.96 GB at Stage 6 (§11, §13.3; spec Appendix
      E 22);
    - `phx-ffi` joins `phx-store` and `phx-exec` as a crate that may use `unsafe`, for the foreign boundary, and
      `#[derive(Pod)]`'s expansion is the one other (§3.1, §16);
    - the save check and `phx inject` run on the build machine only; a gate's audit and live checks come from the
      build run of its commit, and its budget and macro reads from the device run, which carries the recorder
      (§14.3, §14.5, §14.7).

31. **The setup**:
    - the world constants (total population, the map, three countries, 25 regions, the settling year) are fixed; the
      player splits the population, each country between 10% and 70%, so no split can break the budget (§10.0);
    - per country, six three-level choices (development, public debt, private debt, appetite for risk, inequality,
      openness) and a name; a real name labels institutions and currency and pre-fills the choices, and never
      supplies data (spec GEN.14, Appendix E 43);
    - the derived values are drawn jointly from the development level's profile, the other levels' distributions
      conditioned on, nothing clamped; per-level templates instantiate `data/<country>/` at a new game, never
      committed, and a save names the setup through its register hash over the countries it instantiated (§3.7,
      §10.0, §11; spec GEN.15).

32. **The finished world's load at every gate**:
    - from Stage 0's gate, the full-load bench runs random data at the finished world's volumes and shapes through
      the real kernels on the phone, and the gate is judged on it as on the stage's own run (§14.5, §14.6);
    - a miss takes N8.7's remedies before the next stage starts, so the representation and the play resolution are
      set against the finished world from the start;
    - no thermal warm-up precedes a measured year: the budget is the year of consecutive turns (§13.2, §14.5).

33. **The phone app is built by CI**:
    - CI's `android-app` job builds the engine for the phone with fat LTO, writes UniFFI's Kotlin bindings from the
      built library and assembles the bench flavour, which it uploads as an artifact; the owner installs that
      artifact, runs it and commits the report it writes to `perf/device/` (§14.5, §14.6);
    - the app's tool versions are pinned in `android/` (Gradle's version catalog and the wrapper, with the
      distribution's checksum), beside `tools/versions.toml`'s NDK, API levels and build tools.

34. **The population as agents** (2026-09-25; spec REP.1, REP.40, Appendix E 14, 16, 44):
    - cells are retired: every household and small firm is an agent with its own attributes, persons and rows, never
      split, joined or averaged; with cells go keys, profiles, steps and tolerances, parts and landing, pooled flows
      across members, choice groups, the decision gap, promotion and demotion, tracers, seller spreads and
      renumbering (§7);
    - an agent stood for a multiplicity of identical twins, the full population held as agents of many twins or a
      small world of a share of it, until 35 retired twins;
    - lines still record many-party relationships as counts (§7.5); individuals are the institutions and the parties
      ranked at the opening, and the player's household is an agent;
    - the size is the representation's one valve (N8.5, §7.9); §13 keeps its estimates, with the work only cells
      needed marked retired, until the first measurements replace them.


35. **One representation: a world of a declared size** (2026-09-27, the owner; spec REP.40, Appendix E 44):
    - the world holds a declared number of the setup's persons and every household and small firm they form is one
      agent of one party (§7.6); `REP.persons` is the one primitive, 750 000 on the build machine's measure until the
      device resets it, and `phx run --persons N` sets another;
    - twins are removed from every layer: the agent table's multiplicity column, the ledger's holder members and payer
      weight, the pooled-flow rule and standing flows' kinks (built for twins and cells, used by nothing else), the
      tally's unit classes and like-with-like draws, the losers drawn past the failed, a trade's and a tax's whole share
      per twin, the labour round's units and the vacancy lot, the opening's twin counts and the player's seated twins;
    - with twins went the reason only large firms held a deposit's right, so small firms draw rights and extract
      (`GDS.extract_small`);
    - the benches keep the design point's agents; the load report's version is 3.
---

## 19. Coverage

Generated by `phx-check coverage` from the clause map. Status: planned, building, done.

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-id`, `phx-core`, `phx-world` | 0 | 0 | done |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 5 | building |
| A3 | NUM | `phx-num`, `phx-core` | 0 | 5 | building |
| A4 | CHN | `phx-rand`, `phx-core`, `phx-pop` | 0 | 6 | building |
| A5 | GEO | `phx-geo` | 0 | 5 | building |
| A6 | REP | `phx-pop`, `phx-ledger` | 0 | 6 | building |
| A7 | GEN | `phx-world` and every system's contribution | 0 | 7 | building |
| B1 | MON | `phx-ledger` | 0 | 1 | building |
| B2 | SET | `phx-ledger`, `phx-store`, `phx-world` | 0 | 0 | done |
| B3 | REG | `phx-ledger` | 0 | 3 | building |
| B4 | ACC | `phx-acct` | 0 | 3 | building |
| C1 | MKT | `phx-market`, `phx-acct` | 0 | 4 | building |
| C2 | VAL | `phx-val` | 1 | 1 | building |
| D1 | POP | `sys-dem` | 0 | 6 | building |
| D2 | HH | `sys-hh` | 1 | 6 | building |
| E1 | TEC | `sys-tec` | 1 | 6 | building |
| E2 | FRM | `sys-frm` | 0 | 6 | building |
| E3 | CAP | `sys-cap` | 1 | 5 | building |
| F1 | GDS | `sys-gds` | 1 | 2 | building |
| F2 | SRV | `sys-srv` | 1 | 2 | building |
| F3 | FRT | `sys-frt` | 1 | 2 | building |
| F4 | LAB | `sys-lab` | 0 | 6 | building |
| F5 | HSG | `sys-hsg` | 0 | 2 | building |
| F6 | TCR | `sys-tcr` | 2 | 2 | planned |
| F7 | ENE | `sys-ene` | 2 | 4 | planned |
| G1 | BNK | `sys-bnk` | 0 | 4 | building |
| G2 | BFL | `sys-bfl` | 2 | 3 | planned |
| G3 | BCP | `sys-bcp` | 2 | 3 | planned |
| G4 | SEC | `sys-sec` | 4 | 4 | planned |
| H1 | MMK | `sys-mmk` | 2 | 4 | planned |
| H2 | SOV | `sys-sov` | 1 | 5 | building |
| H3 | CRD | `sys-crd` | 3 | 3 | planned |
| H4 | EQY | `sys-eqy` | 3 | 4 | planned |
| H5 | MNA | `sys-mna` | 4 | 4 | planned |
| H6 | FND | `sys-fnd` | 3 | 4 | planned |
| H7 | DLR | `sys-dlr` | 3 | 3 | planned |
| H8 | IDX | `sys-idx` | 1 | 3 | building |
| H9 | RAT | `sys-rat` | 2 | 4 | planned |
| I1 | DRV | `sys-drv` | 4 | 4 | planned |
| I2 | DRX | `sys-drx` | 4 | 5 | planned |
| I3 | INS | `sys-ins` | 4 | 4 | planned |
| I4 | PEN | `sys-pen` (no crate yet: Stage 0's state pension is `sys-soc`'s) | 0 | 5 | building |
| J1 | TRS | `sys-trs` | 1 | 5 | building |
| J2 | TAX | `sys-tax` | 0 | 5 | building |
| J3 | SOC | `sys-soc` | 0 | 5 | building |
| J4 | CB | `sys-cb` (from Stage 0: the central banks and treasuries as parties) | 1 | 5 | building |
| J5 | SUP | `sys-sup` | 2 | 4 | planned |
| J6 | POL | `sys-pol` | 5 | 5 | planned |
| K1 | FX | `sys-fx` | 5 | 5 | planned |
| K2 | XB | `sys-xb` | 5 | 5 | planned |
| M1 | OBS | `phx-core`, `phx-obs`, `android/` | 0 | 8 | building |
| M2 | STA | `sys-sta` | 1 | 5 | building |
| L3 | estates | `phx-ledger` and `phx-world` at Stage 0; `sys-est` from Stage 2 | 0 | 2 | building |
| L1, L2, L4–L12 | transmission chains | read from the run by `phx chains` (N4) | 2 | 7 | planned |
| N1 | audit | `phx-audit` and every system's families, run in every build run | 0 | 6 | building |
| N2–N8 | measurement | `phx-cli`, `phx-world` metrics, `android/` bench | 0 | 7 | building |
