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

- **Scale.** The design point is 6 million persons and the counts that follow from them (§13, spec Appendix E 51):
  their households, the firms and institutions of three countries, and the contracts, holdings and records between
  them. The world holds a declared number of the setup's persons, every household and firm they form one party
  (REP.40); the play resolution is the valve, set and reset by measuring the budget (N8.5, §13).
- **Cost follows events** (REP.12, N8.6). A day's work is what falls due on it: the due wheel files each contract at
  its next due, the agenda each party at its next decision, and hazards are drawn ahead to the day they hit (§7.7).
  Work over a whole store is done only as a declared sweep, counted and ratcheted (§6.6, §7.2).
- **Memory is set by the bases' rows.** Every store is a base of the core with its bytes a row declared (§7); the
  world's memory is the contract rows and the persons carrying it, their counts times their bytes at the design point
  (§13).
- **A phone is latency-bound and throttles.** Work is partitioned by target range and swept while its range is in
  cache, with a budget of cache misses per item; random gathers are counted; the pool runs on the fast and medium
  cores, and the budget uses the sustained speed measured on the device.
- **Exactness.** Money and units are integers; conservation identities hold exactly (Law 7).
- **Fifty systems, one world.** Systems never call each other; they cooperate through kernel channels (§4) and hold
  rules and declarations, never stores (§3.5).

## 2. Technology

| Concern | Choice | Why |
| --- | --- | --- |
| Engine | **Rust**, stable, pinned in `rust-toolchain.toml`, edition 2024 | Layout and allocation control, no GC pauses, data-race freedom, Android and Linux targets, crates as compiler-enforced boundaries. |
| World mathematics | **`libm`** (pure Rust) for transcendental functions in world code | Platform-independent results. |
| Parallelism | **Own pool** in `phx-exec` over **rayon-core**, pinned to fast and medium cores, with Android performance-hint sessions | Cost-sized fixed chunks, no little cores, few barriers. Only `phx-exec` depends on rayon. The world's meetings, making and input orders run on the pool; performance-hint sessions are not built (`PerfHint` has only `NoHint`, called by nothing), and come with the phone's run of the world. |
| Hash maps | **hashbrown** + fixed-seed **foldhash**, behind a kernel map type iterated in key order where order matters (`sorted`) and in no set order only for order-free reads such as sums (`each`), sharded where written in parallel; off the day's paths only: the core's stores, indexes and passes hold none (PC-92) | Fast lookups at assembly and in reports; no outcome depends on hash order. |
| Randomness | **Own Philox4x32-10** in `phx-rand`, batch-first, NEON-vectorised samplers | Draws addressable by (stream, identity, day, index): parallel, order-free, reproducible (CHN.1, CHN.6). |
| Compression | **zstd** level 1 after per-column transforms (delta, zigzag, bit-packing) | Fast; transforms double the ratio on integer columns. |
| Declared data | **TOML** + **serde**, at assembly only | Diffable, never on a hot path. |
| Android bridge | *(none: the phone's run of the world brings one, when the owner calls the device run)* | One foreign surface; data crosses in pages. |
| Android build | **cargo-ndk** + **Gradle**; 16 KiB page alignment | Required by current Android targets. |
| Interface | **Kotlin + Jetpack Compose** | Native; off the hot path. |
| Command line | **clap** in `phx-cli` | Runs, measurements, reports. |
| Checks | **`phx-check`** (`syn`, `cargo metadata`) + **clippy** `disallowed-*` lists | Structural and type-aware rules (§16). |
| Counters | **The bench** (`tools/bench.sh`): the world run, each stage of its day timed by the run's clock, and its `-F` mode, the finished-volume measure (`phx-fin`, §14.7), each base filled at the design point; the engine's own counters | One tool for every measure (owner). |
| API snapshots | **cargo-public-api** for kernel and interface crates | Kernel surfaces change only on purpose. |
| CI | **GitHub Actions** on x86-64 Linux, with an Android build job. The world runs on the build machine, never in CI | See §14.7. |

External crates are an allow-list in `phx-check`; adding one is recorded in §18.

---

## 3. Layers and crates

```text
L4 apps            phx-fin · phx-cli · phx-play · phx-check  (android/: the game's interface)
L3 assembly        phx-world · phx-obs
L2 systems         sys-dem sys-hh sys-est ... sys-sta                    (depend on L0, L1, IF only)
IF interfaces      if-base if-pop if-labour if-property if-firm if-banking if-credit
                   if-securities if-risk if-energy if-open if-state      (data and pure rule signatures only)
L1 kernel (core)   phx-store phx-exec phx-core phx-geo phx-pop phx-record phx-agenda phx-ledger
                   phx-contract phx-hold phx-market phx-acct phx-val phx-risk phx-end phx-mind phx-audit
L0 foundation      phx-macros phx-num phx-rand phx-id
```

### 3.1 Dependency rules (checked by `phx-check`)

- A crate depends only on lower layers. Inside L0 the order is `phx-macros → phx-num → phx-rand → phx-id`; inside L3
  it is `phx-world → phx-obs`, so the observer reads the world through its inspector and the world never reads the
  observer; inside L1
  it is `phx-store → phx-exec → phx-core → phx-geo → phx-pop → phx-record → phx-agenda → phx-ledger → phx-contract →
  phx-hold → phx-market → phx-acct → phx-val → phx-risk → phx-end → phx-mind → phx-audit`, each crate owning the bases
  §7 gives it; inside L4 it is `phx-fin → phx-cli → phx-play`, `phx-check` apart, and `phx-fin` depends on L0 and L1
  only. Interface crates may depend on L0, L1 and the interface crates before them in the order `if-base →
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
  `phx-exec` depends on rayon. Only `phx-store` and `phx-exec` may use `unsafe`; the one other `unsafe` is the `unsafe impl`
  that `#[derive(Pod)]` expands to, whose layout the derive has checked, and no source outside those three crates
  may write `allow(unsafe_code)`.
- A crate is created when its first step starts, never in advance; this section names the planned crates before
  they exist, each with the step that creates it.
- **Sweep hooks.** A base maintained inside another base's sweep — income lines, levies, book folds posted during a
  lower crate's apply — is reached through the sweep's hook parameter: a generic `H: SweepHooks` declared by the
  lowest crate that runs the sweep, monomorphised with no `dyn`, the concrete hooks composed by `phx-world`'s routing
  at the slot. A lower crate never names a higher one.

### 3.2 Foundation

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-num` | NUM.1, NUM.2, NUM.5, NUM.6, MON.16, Law 7 | `Money`, `Qty`, tick-unit `Price` and `Rate`, fixed-point position values, `Missing<T>`, rounding conventions, checked `i64`/`i128` arithmetic, `violation!` and its payload, **point tables** (a trade's price points as `i64`, REP.34). No floating-point type in any store. It has no named comparisons: `min` and `max` are refused (§16) and callers compare inline. |
| `phx-rand` | CHN.1, CHN.6, CHN.7 | Philox; stream keys; batch samplers: binomial (inversion, BTPE) and **zero-truncated** binomial, multinomial (conditional binomials; alias tables when draws are fewer than categories), hypergeometric (inversion when small, Stadlober's ratio of uniforms otherwise) and multivariate hypergeometric, weighted picks over prefix sums, geometric (for next-candidate days), normal, log-normal, Pareto, Gumbel; rejection thinning. |
| `phx-id` | TIME.1, TIME.2 (the day and civil dates), PTY.1 (identities) | Identifier types (`PartyId`, slots, `LineId`, `RowRef`, `InstrumentId`, day-local ids), `Day`, `Date`. |
| `phx-macros` | — | `#[clause]` (which checks each name is a clause identifier), `#[derive(Pod)]`, `#[derive(Saved)]`, `declare_prim!`, `declare_fact!`, `declare_kind!`, `declare_stream!`, `declare_hazard!`. |

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
Weibull by inversion. `Draws` builds an address's counter once, so a block is one Philox call and its words are read
inline; an address holds 2^24 blocks, so bulk work takes an address a chunk. `Draws::x4` makes four addresses' first
blocks at once from one block index (`philox_x4`, the four chains' rounds side by side), each as `from_block` would.

### 3.3 Kernel

The kernel is the core (§7): every store, index, traversal and kernel the finished world needs, one crate for each
group of bases, designed at the design point (§13) before anything is built on it. A crate's bases are its §7
subsection's; a crate not yet in the workspace is created by the first step named.

| Crate | Carries | Bases (§7) | Today, and what moves |
| --- | --- | --- | --- |
| `phx-store` | SET.12, SET.15 | §7.1: columns in reserved address space, slots and generations, day buffers and the day plan, chunk arenas, sum-trees, keyed indexes, epoch flags, horizon rings, the interner, the save contract and the world hash, `StoreStats` (K-01–K-10; S1.159–S1.168) | Paged columns, arenas, slot allocators, column descriptors, save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | §7.2: chunk plans, partitioned apply, keyed reductions, declared sweeps and rolling cursors, the measurement counters (K-11–K-15; S1.117, S1.169–S1.171) | The pinned pool, chunked traversals, gathers, sharded `KeyedReduce`, fixed-tree reductions, radix sorts. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3, SET, MON | §7.3: streams, the calendar's day facts, the register with handles and policy schedules, the kind catalogue, the units registry, the stage table, the capacity table (K-19–K-24; S1.114, S1.177–S1.185) | The vocabulary every system and kernel crate declares with (§5); and, until their migrations, the core's stores: `store.rs` (kind stores → `phx-pop` K-32, families → `phx-contract` K-53, accounts → `phx-ledger` K-47), `wheel.rs` (→ `phx-agenda` K-42), `flows.rs` and `settle.rs` (→ `phx-ledger` K-48, K-49), `goods.rs` and `units.rs` (→ `phx-hold` K-60, K-66–K-68; `UnitIds` → K-22), `events.rs` (→ `phx-record` K-36), the runtime half of `decisions.rs` (→ `phx-mind` K-100). |
| `phx-geo` | GEO | §7.4: tiles, zones and distances, networks and routes, cells, deposits, weather and catastrophes (K-25, K-26, K-28–K-30; S1.187–S1.193) | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure. |
| `phx-pop` | REP | §7.5: the directory, kind stores and windowed groups, persons, offices, the per-day party cache (K-31–K-35; S1.194–S1.211) | The population kinds compiled from the systems' items; each person packed into its word (`persons::Persons`); `hazard.rs` (→ `phx-agenda` K-44). |
| `phx-record` | — | §7.6: the stored log and the event log, the records store, the day ledger, statistics accumulators and sample frames, life records, tallies and votes, the recorder (K-36–K-41, K-106; created at S1.212) | Planned. |
| `phx-agenda` | — | §7.7: the due wheel, the decision agenda, hazards drawn ahead, messages and notices, the trigger index (K-42–K-46; created at S1.225) | Planned. |
| `phx-ledger` | REG.5, TAX.2, TAX.7 | §7.8: money accounts, flow batches, settlement, dated commitments, the levy engine, settlement tallies (K-47–K-52; S1.240–S1.256) | The contract algebra (`algebra.rs`, `shape.rs` → `phx-contract` K-55), the reasons and line kinds systems declare, the withholding levy's bands, `opening.rs` (→ `phx-world`'s opening), the online choice of a household's bank (`online.rs` → `sys-bnk`, a rule). |
| `phx-contract` | — | §7.9: the contract store, side aggregates, terms, shapes and due plans, status and arrears, books, participations, accruing statement contracts (K-53–K-59; created at S1.257) | Planned. |
| `phx-hold` | — | §7.10: holdings, unit totals and nature's net, the place index, lots and cost flows, bounds and liens, instruments and their holdings and events, named units, capital classes, standing rates, processes in progress (K-27, K-60–K-69; created at S1.271) | Planned. |
| `phx-market` | MKT | §7.11: standing offers, the stall and vacancy books, the posted-price meeting, the sales batch, between firms, the call auction, the network call, the continuous book, the dealer market, the bilateral protocol, the administered form, search and match, carriage, rationed queues, perishable capacity, prints and marks (K-70–K-86; S1.296–S1.327) | The posted-price meeting, the between-firms meeting, the hiring kernel, carriage and reach, on the core. |
| `phx-acct` | ACC, MKT.20 | §7.12: account lines, equity and net assets, carrying values, statements and consolidation, valuations and curves (K-87–K-91; S1.328–S1.335) | The accounting bases' declarations. |
| `phx-val` | VAL | §7.13: public series, the investor schedule (K-92, K-105; S1.336, S1.337, S1.357) | Outlook methods as pure functions, surprise and confidence arithmetic, the investor schedule. |
| `phx-risk` | — | §7.14: books and limits, margin and collateral, exact linear aggregates (K-93–K-95; created at S1.338) | Planned. |
| `phx-end` | — | §7.15: estates, the ending kernel, the waterfall, resolution (K-96–K-99; created at S1.342) | Planned; estates run in `phx-world` until S1.344. |
| `phx-mind` | — | §7.16: the decision core, attention, minds (K-100–K-102; created at S1.348) | Planned; the decision core is `phx-core`'s until S1.349. |
| `phx-audit` | — | §7.17: the audit engine and injection (K-103; created at S1.353) | Planned; the audit's families run in `phx_world::core_audit` until S1.354. |

### 3.4 Interfaces

Each interface crate is a domain's shared vocabulary: kinds and roles; fact handles; line-kind terms built from the
contract algebra; message payloads; decision-point input and output types; rule signatures; view schemas. **Every item
names the one system that writes or implements it**; assembly checks it (§5.4). A line kind declares the party kinds
each side may hold, and assembly refuses a row of any other (derivatives: individuals only, spec Appendix E 33). A
decision point may have one rule per decider kind; each decision point's rule is registered by its owning system — the
system that owns that kind's decision — except that a lender valuing a claim on its book does so through `sys-bnk`. An
item that only one system may construct carries a **writer token** only that system can build: a lender's
`LoanAssessment` (planned, S2.102; in `if-credit`) is built only by `sys-bnk`, so every price and provision on a lender's book comes
from it; other systems read it by handle and call `sys-bnk`'s rule handle `loan_claim_value`, never building one.
Five interface crates stand: `if-base`, `if-pop`, `if-labour`, `if-credit` and `if-state`; the rest are created by
the first step that needs them. Only `if-pop`'s items are `ItemDecl`s with a named writer that assembly checks
(`phx-world/src/systems.rs`, `INTERFACES`); the others hold constants, kinds and decision types the systems read.

| Crate | Domain |
| --- | --- |
| `if-base` | shared identifiers and declared data: products, occupation families, skills, capital kinds, ways (issued at runtime from Stage 6, an improved way stored against its base as factors; the rule handle `labour_per_unit`), units; as built, the products, the way and the way-set identity as data only, what a way takes and finishes and the way-set interner being `sys-tec`'s; rating scales and notches; money-market tenors, segments, collateral baskets, haircuts and limits; agents' participation per asset class; pension kinds, contribution rate points and fund-menu identifiers, so a job's terms need no later crate |
| `if-pop` | households, persons, roles, demographic facts, household decision points (founding a firm among them; `enrol`, `form`, `separate`), personal insolvency law; the education record, schooling and the enrolment attachment; education and family law and the day-local `Meeting` message; the rule handles `skill_now`, `division_shares` and `type_at_formation` |
| `if-firm` | *(not built on the core: the old kernel's firm facts were deleted at S1.24, the core keeping a firm's state in its record)* pricing and payout decision points; research, imitation and licensing (`innovate`, `licence_quote`, `licence_accept`), licence lines, the patent instrument and the patent register; company and insolvency law, plans and votes; filed accounts |
| `if-labour` | employment terms (the pension's kind and contribution rates among them), the employment attachment's scheme component, vacancies, applications, offers, separations; the rule handle `labour_state`, a read of a role's attachments and participation |
| `if-property` | dwellings, land, tenancies, collateral descriptions, appraisals, sales |
| `if-credit` | loan terms (interbank loans among them), applications and quotes, the lender's assessment, workouts and loan sales, credit-bureau records, trade credit; the decision points that need them (borrowing, where to live, financing, distress, answering a restructuring, arrears and filing, bidding for a failed bank) |
| `if-banking` | deposit terms, banking arrangements and payment order; bank facts; the funding and capital rule handles (`marginal_cost_of_funds` over each lender's declared sources, `capital_charge` under its declared regime); licensing; resolution |
| `if-securities` | bonds, shares, fund units, dealers, securities loans, prime brokerage, listed companies' reports, ratings (`RatingView` (planned, S3.207)), indices, orders; the decision points on them — the depositor's bank choice, households' holdings, `place_cash`, bids, votes |
| `if-risk` | derivative, insurance and pension terms, and the rule handle `accrued_schedule` over a member's DB right; margin demands; close-outs; claims; the protection scheme and the guarantee fund; the decision points on them — positions, cover, collateral, schemes offered, trustees, repair, members' pensions, bids for an insurer's portfolio or a clearing house's service |
| `if-state` | policy values, levies, benefits, agencies, budgets, elections, rule signatures of tax and benefit; the central bank's regimes, tenders and collateral framework, a bank's `fund_position`; the treasury's plan and payment priority |
| `if-open` | currencies, regimes, trade, migration |
| `if-energy` | power products, the grid, dispatch |

### 3.5 Systems

`sys-dem` (the spec's POP), `sys-hh`, `sys-est`, `sys-tec`, `sys-frm`, `sys-cap`, `sys-gds`, `sys-srv`, `sys-frt`,
`sys-lab`, `sys-hsg`, `sys-tcr`, `sys-ene`, `sys-bnk`, `sys-bfl`, `sys-bcp`, `sys-sec`, `sys-mmk`, `sys-sov`,
`sys-crd`, `sys-eqy`, `sys-mna`, `sys-fnd`, `sys-dlr`, `sys-idx`, `sys-rat`, `sys-drv`, `sys-drx`, `sys-ins`,
`sys-pen`, `sys-trs`, `sys-tax`, `sys-soc`, `sys-cb`, `sys-sup`, `sys-pol`, `sys-fx`, `sys-xb`, `sys-sta`.

A system crate holds rules and declarations only: rules are pure functions over the views handed in, and
declarations are its kinds, attributes, families, reasons, levies, markets, hazards, decisions, index instances and
sweeps. It holds no store, no per-party pass and no `&mut` world state (PC-97); each system's mechanism steps write
its paragraph below. The rules each system holds on the core, one paragraph a system that owns one:

**`sys-hh`** (HH.4, HH.5). A household decides its spending every `HH.spending_days` business days from its money, its
country and four positions: `HH.income`, the outlook of its permanent income a year; `HH.after`, what it held after
its last decision; `HH.received`, the money taken in since its last look at its income; and `HH.looked`, the month of
that look. Its money less `HH.after` is added to what it received; at its first decision in a new month it looks, the
month's receipts grossed to a year taken into the outlook at `HH.income_gain` — a month, since its pay comes monthly
and a week's receipts would read a payday as a year's income and the weeks between as none. It spends by the
buffer-stock rule at its cash on hand in years of its income (Carroll's cash on hand holds the period's income):
`c* + κ(m − m*)` years of income a year, over the decision's share of a year, never more than it holds; and asks each
product its country's budget shares (households' part of the accounts' final composition) of that at retail as a want
of money. The opening writes `HH.income` as what the household's contracts owe it over the year after the opening: the
dues its contracts fall due for on their dates, repayments of a balance left out. The buffer-stock rule is solved once
at assembly by the endogenous grid method (Carroll, 1997) from `HH.patience`, `HH.risk_aversion`, the shocks' spreads
and the chance of no income, with `HH.real_return` and `HH.income_growth` as placeholders naming BFL and HH; its target
`m*`, consumption there `c*` and propensity `κ` are read from the solution, never declared (at Carroll's baseline
m* = 1.28, c* = 0.994, κ = 0.39). `tools/data/derive_hh.py` derives the budget shares per group from the inter-country
tables' households' final consumption (HFCE, 2019): the median over the group's economies of each product's share — of
a mining industry's output the resource other users than the one transforming it take (oil and gas, stone), as the
ways split it — finance, real estate, public administration and households as employers left out, renormalised; the
system refuses shares that are not one a product.

**`sys-tec`** (TEC.1–TEC.4, TEC.9, TEC.12). Products and ways are data (`if-base`), compiled at assembly into
`sys-tec`'s `Technology`: each country's one opening way per product and its public way-set per industry. An extracted
product is counted in kilograms, as deposits hold it, fine enough that what a way takes of one for a start, rounded
up, is near what it uses; every other in what one US cent bought at world-average prices in 2022, so a way's
quantities are physical and no price moves a draw. A way states per unit made its inputs, hours by occupation family,
plant by kind per unit a year, land and deposit, with lead time, batch and yield in parts per million; for a start it
takes inputs rounded up and finishes output rounded down, and a making's start is the least that finishes its output
(`ways::started_for`). Way-sets are interned once, sorted (`WaySets`), so a firm's known ways are a four-byte identity
and membership a binary search.

**`sys-frm`** (FRM.5, REP.34). A trade's point table (`FRM.price_points`) holds one decade's points; a price's
candidates are its decade's and the two neighbours' points scaled by powers of ten, each whole
(`Management::points_near`), so every posted price is a point at any scale. The management's values are one shared
type (`data/shared/FRM.toml`), compiled once (`Management`). A price review moves the markup by
`rules::markup::update` over the sales since the last review against those expected and by what competitors charge,
and posts the point nearest `(1 + μ)·unit cost·π^η`, π the stocked pressure, only when the loss it saves over the days
to the next expected review exceeds `FRM.menu_hours` at the staff's wage. A firm missing an input its decision reads
decides nothing; nothing is read as zero.

**`sys-cap`** (CAP.3, CAP.5, CAP.9). A kind of service life L in C classes (`CAP.condition_classes`) gives each class a
span L ÷ C; a class's **efficiency** is the hyperbolic (L − a) ÷ (L − β·a) at its mid-age a, its **value**
e^(−δ·a) of the cost new, and units leave each class at C ÷ L a year, so a unit's life is Erlang about L
(`rules::wear`). On each `CAP.review_days` day each firm weighs adding plant (`points::INVEST` through the decision
core, `rules::invest`): the output a day it expects to sell, within what its staff make, beyond what its plant allows,
of its scarcest kind; the margin that output earns a year as an annuity over the kind's life at its required return
(its hurdle rate), against the plant's cost at its product's mark in its region, beaten by Dixit and Pindyck's waiting
multiple β ÷ (β − 1) from the rate, the payout over value and the volatility of its sales between its last two
periods, and funded from its money; investing, it asks that cost of the kind's product at its region's between-firms
meeting. The owner's maintenance, repair, sale and scrapping by CAP.4 (`rules::maintain`) come with the plant's resale
market (S2.03).

**`sys-gds`** (GDS.3, GDS.4, GDS.8, GDS.13). Each storable product declares its grade classes (`GDS.grade_bounds`),
its yearly spoilage in stock and the room it is kept in; which class a way takes of an input waits for the spec's
decision on how a grade enters use (Appendix E 45). An extractor works what it holds by Hotelling's rule (`GDS.extract`):
its price net of its unit cost against its stance's outlook of its mark at its next decision, discounted at its
required return; a deposit without end whenever the mark covers the cost.

**`sys-srv`** (SRV.4, SRV.5, REP.22). A buyer goes to the open seller it values most at `−α·ln p − γ·km + ε`, ε a
standard Gumbel taste from `SRV.taste`, so the choice is multinomial logit exactly: it draws its choice from the
logit's chances, one uniform draw of its own stream over the sellers in its reach ordered by what a unit there costs —
the prefix it can pay for — never a taste per seller, and a buyer turned away draws again among the rest. Each
product's retail meeting has a place per region with its firms at no distance. A seller serves its buyers in an order
drawn from `SRV.capacity_lot`, in whole units at its price for a lot pro rata, rounded once as the sale settles;
prices are posted for a lot so they carry places below a currency's least unit. A buyer with no stall left in reach
goes without. A service is delivered as it is made and never held, and what no settled sale takes perishes. The
services a firm's way uses (`TEC.inputs` of the products not stored) are owed for what it makes, or, a provider, for
what it sells (`CoreGoods::services_owed`), and bought on its production schedule from its region's providers at no
more than a unit's worth to it, used as delivered (`buy_services`), as a provider bills by the period; their least
price there counts in its unit cost, and the opening's expected sales count every product's use by the ways.

**`sys-frt`** (FRT.1, FRT.4–FRT.10, GEO.13). At the opening each firm selling `FRT.carriage_product` is given a mode,
drawn by `FRT.mode_share` on stream `FRT.opening`, and every mode's route and length between two regions' market zones
is taken once from the network (K-26). On its `FRT.shipping_days` schedule, a business day, a firm of a stored product
holding free units beyond its expected sales times the goods' days of cover weighs carrying the whole lots beyond to
each other region of its country that marks its product: the mark there less its own price less the freight of a lot
out (`phx_market::carriage::freight`, at the lowest price carriage is posted at by a carrier of that mode with room at
its region), the widest gap by any posted mode, the carriers' prices and room read once a day into one table by region
and mode (`carriage_offers`) that the shippers, the meetings and the basis read; it decides through the decision core
(`FRT.ship`). The consignments of each origin and mode meet in one carriage meeting (K-83, order by
`FRT.capacity_lot`): each booked with the cheapest carrier whose vehicles' room today (its transport equipment's
efficient units times the mode's tonne-km a day) holds the trip out and back, one whose route crosses a segment already
carrying its day's tonnes refused. A booking's goods are committed and its freight — the whole units of carriage its
tonne-km take, priced at the carrier's posted price a lot and rounded once as a sale is — is a day's flow under
`CARRIED`; after settlement a paid trip departs (K-69, the goods pledged to the carrier) and is the carrier's revenue,
the shipper's services used and a sale of carriage to the national accounts, and an unpaid one frees its goods. On its
day a shipment arrives: used up where it left under `SHIPPED`, made where it arrives under `ARRIVED`, at the cost it
left with. An arrival does not wait on its carrier: goods aboard a carrier that failed meanwhile still arrive, their
owner's. `Core::basis` gives each pair of places' gap between their marks a lot with the least freight between them,
and `Core::cover_by_place` each place's days of sales its stocks cover; the run report's `freight` holds the days.

**`sys-lab`** (LAB.1, LAB.4–LAB.9, LAB.12, LAB.17, PTY.3, REP.34). A person's labour state is three person attributes
of the household kind: its state (`LAB.state`: not searching, searching, retired), the occupation family it last worked
in (`LAB.occupation`) and the wage point of its last job (`LAB.last_point`); a hire sets the occupation and point and
stops the search, a layoff or a quit sets it searching. Its employment is its jobs: contracts in `LAB.employment` from
firm to household naming the person, on its country's monthly dates, each class (occupation, hours, start year) held
per schedule. The labour kind (`if_labour::LabourKind`) names the decision points — the employer's posting and
layoffs, its selection among applicants and its renegotiation offer; the searcher's applications, its acceptance, its
answer to a renegotiation and its retiring — each an `if_labour` input and output with its rule in `sys-lab`; the world
builds each input and applies each output, and no rule reads the world. The monthly wage at point _n_ is
`LAB.wage_point_ratio` to the _n_-th power, 128 points, the last meaning none (`wages`: `wage_at`, `point_near`,
`least_point`, the offer's `adapt` and the severance `owed`). Each country's level by occupation is its opening staff's
and working owners' hours in it over the hours its ways ask of it at the opening output, so a firm's hours a unit of an
occupation are its way's, fewer by its productivity, at that level. **Posting** (LAB.4, `rules::post`): on its
production schedule an employer reads its planned output a day — its expected sales over its production period, within
its plant's capacity — and, per occupation, its hours a unit shared by its way's mix of occupations, its staff's hours
less those under notice, its open vacancies' hours and an hour's wage at the point it would offer. Its staff's hours
make its output together, so when they exceed what its planned output needs by a whole job it lays off the surplus in
whole jobs from the occupations furthest over their part of the way's mix and withdraws its vacancies; otherwise it
posts the whole jobs its planned output still needs, in the occupations furthest short of their part, where an hour's
output at its price outlook is worth more than that occupation's wage plus financing it until the sale and than the
law's least, and withdraws the vacancies an occupation no longer needs; it decides nothing while it has no price or
planned output. The point it offers is its last fill's in the occupation, one lower when that fill came in the least
days a match takes, else its newest staff's there, else the mean wage's for the hours, never below the law's least; a
vacancy standing past `LAB.vacancy_patience_days` is raised a point. Layoffs are drawn by lot among the firm's jobs in
the occupation, given the law's notice; at its end the contract closes, severance for the whole years served — the
class's days a year of the daily wage — is a flow from firm to household, and the person searches again. **Search and
acceptance** (LAB.5, LAB.8, REP.22): each searching person not waiting on an application or an offer applies, a round's
share of `LAB.applications_a_week`, among the vacancies of its region and occupation at a skill it has paying above its
reservation, by the logit of `LAB.wage_weight` times the wage's log (K-82); an application is seen at
`LAB.seen_chance`; an offer is accepted when the weight times the log of the wage over the reservation, plus the
match's taste, is positive. The reservation is `LAB.reservation_share` of the searcher's last wage (a placeholder
naming HH). Search, selection and answer are a day apart each, so a match takes at least three days. **Pay rounds**
(LAB.17, LAB.9): an employer's first round falls on a day drawn in the review period after its first production
schedule (stream `LAB.review`), then once a period (`CoreLabour::reviews`). Each contract not under notice is offered
`rules::renegotiate::review` over the market's point (`offer_point`) and the most the job's month pays — its wage and
the unit's margin at the price the firm expects (its stance's outlook of its mark, or its own price before the mark
prints) over its cost of making a unit (`unit_cost_of`), for each unit a month of the job's hours makes. The offers
wait (`CoreLabour::offered`) until the employers' rounds are done, then are answered together: one search per country
over the reviewed employees, each as a seeker whose reservation is its own wage, gives the vacancies each sees; each
answers by `rules::renegotiate::answer` from its reservation, the best it saw and its household's price outlook
compounded to the next round. Where the work cannot pay its counter, an offer below its reservation sends it to
search, its contract closed; any other leaves it at the offer, its seen vacancies sent as applications with the day's
(`on_the_job`); a hire of a person who holds a job closes that job. **Retirement** (LAB.6) is a scheduled hazard on
persons (`LAB.retirement`, at `SOC.pension_age` by age and sex): on the first birthday an adult not retired has reached
its pension's age, the `retire` decision (a placeholder rule naming HH: retire at that age) sets it retired; it leaves
its jobs and claims the state pension. A failed firm's staff (LAB.12) search again, their wages owed and severance a
first-rank claim on its estate.

**`sys-bnk`** (BNK.4–BNK.6, BNK.19, BNK.20; `credit`). A borrower's class is the number of `BNK.cover_bounds` its
interest cover reaches — its period's income to date grossed up to a year plus the interest its term loans charge, over
that interest; none before a day of its period has passed or where it owes no interest, which is the worst class. A
class's default frequency is `BNK.default_rates`' counted as `BNK.prior_loan_years` of the bank's own book, with the
loan-years and defaults its book has seen; its loss given default likewise `BNK.loss_given_default` counted as
`BNK.prior_recoveries`, with the shares of defaulted balances its write-offs lost. A quote is the rate it can earn
instead (the country's policy rate at the opening; a placeholder naming BFL) plus the expected loss plus
`BNK.risk_weight` × `BNK.capital_requirement` × `BNK.required_return` plus the loan's cost (`BNK.loan_cost_share` of
output per person) over principal and years, rounded up to `BNK.rate_step`. A bank declines a class worse than its
standard, or a loan its net assets cannot carry at the capital requirement over its risk-weighted firm loans; it
begins at every class whose published frequency is below one, and each monthly review moves the standard a class
tighter when its book's defaults cost more than the published frequencies priced, a class looser when less. An
estate's settlement returns each debt it wrote off, which the bank that held it learns from. A firm applies once,
`BNK.refinance_lead_days` before its loan's maturity, for the balance and months again, to its own bank and further
banks of its country, how many drawn from `BNK.lenders_asked`' shares; it takes the quote whose rate less its taste for
the lender in rate steps (`BNK.lender_taste`) is lowest, if its rate is below the firm's required return. The loan
written is a contract of its own at the quoted rate, interest monthly on the balance from the day written and the
principal at the end; the principal is paid into the borrower's account — a deposit the lender creates, or reserves
paid to the bank that keeps it (MON.6).

**`sys-cb`** (CB.10, MON.3, MON.7, MON.9; `phx-world`'s `core_central.rs`). The facilities are two families of
overnight contracts, `CB.deposit_facility` (the central bank owing a bank) and `CB.lending_facility` (a bank owing its
central bank), each position reckoned from terms at its facility's rate, so the loan books, the accounts and a bank's
balance sheet read it as any loan. The fund stage runs after settlement on each country's business days (TIME.6's 8):
every position is returned with its interest over the days it stood; on a month's first stage the central bank's net
interest since the last is remitted to its treasury's account, a loss kept against its equity; then each bank's chief
executive asks the facilities (`BNK.request`, through the decision core) from its reserves after the returns. The
stage's flows — returns first in each bank's payment order, then what it opens — settle together through the
settlement kernel with the issuer on one side, which never fails; a return that fails leaves its position standing,
overdue. Intraday credit is each bank's reserve account's facility: as much as every bank of its country held in
reserves at the opening. Reserves left below nothing after the stage are the central bank's overdue claim, counted
each day with the banks that owe it (BFL.10's liquidity failure; the bank's resolution is Stage 2's). The request (a
placeholder naming BFL): a bank's target is its reserves over its deposits at its first fund stage, times its deposits
now; it places what it holds above the target and borrows what it lacks as far as its collateral lends — its firm
loans' balances less `CB.loan_haircut`. The corridor (placeholders naming CB) is the policy rate at the opening less
`CB.deposit_spread` and plus `CB.lending_spread`. The opening's central-bank loans — the sheet's
`CENTRAL_BANK_LOANS` — are shared over each country's banks by their weights and opened as lending-facility positions,
returned and asked anew at the first fund stage. The core keeps what the issuers owe as money by class — reserves, the
treasuries' accounts, banknotes (the accounts held at the issuer by anyone else) — moved by every settled flow, the
payer's class less and the payee's more, the issuer's own side moving none; at each close it is held to what the
accounts show, a difference a finding of the money family, and the parties' money is held to what the banks and the
issuers paid them. The run report's `fund_stages` gives each country's stage a day: placed, borrowed, the banks doing
each, what stood overdue, the positions unreturned, what was remitted; LC-1-27 reads it with the money family.

**`sys-sta`** (STA.3, STA.4). Each country's agency samples the day's sales by keyed draws (`STA.sample`, each return's
arrival from `STA.returns`), so a sale's inclusion is the same whenever it is read; a panel household's deaths and
onsets are recorded as its outcomes make them; releases are computed only on their calendar's days, and readers read
them through `latest`, which returns nothing before a release's day. **The national accounts** (`STA.accounts`): each
settled payment in which a firm received revenue is a firm's record for the seller's country: the revenue is output,
and each other party's payment in it a purchase by a firm, a firm's purchase of plant (`CAP bought`), a household's
consumption, or anyone else's; a seller's own payment in its sale is a levy on it. Every firm's and household's
income — its revenue and expense other than wear, and the interest and wages it earned or paid — is counted in full, as
are firms' stocks of goods at cost at each census. A release publishes production (output less purchases plus the
change in stocks), expenditure (consumption, investment and others' purchases plus the change in stocks), income, and
the discrepancy of expenditure over income. **Series and vintages**: six monthly series (`if_state::consts::NAMES`:
consumer and producer indices, the labour force survey, the money stock, the period life table, the national
accounts), each released on its calendar's business day of the month its lag names. A period's first release
(vintage 0) reads the returns in by then, each return's arrival drawn at `STA.early_returns`; its revision (vintage 1)
`STA.revision_months` later reads every return; both are kept. `sys_sta::latest` returns the latest vintage of the
latest period published by the reader's day. **The life table and the survey**: a panel of households drawn by
`STA.sample`; a period's exposure is the mean of the panel's persons, by age class (`STA.age_classes`) and health, at
its first and last census, times the days between; each entry carries its events, person-days and rate a year, each
estimate the panel's count over its sample share and, for a first release, over the share of returns in. The labour
force counts the employed, the searching unemployed and those out of the labour force. **Money**: the banks' reserves
and deposits by holder class (household, company, the rest).

**`sys-idx`**. Each sampled sale is kept at `STA.sample_share` by its keyed draw; a period's link weights each
product's unit-value relative by its share of the period before's sampled spending among the products priced in both,
and chains on the level last published for the period before; the first period priced opens at `IDX.base`.

**`sys-soc`** (SOC.2, SOC.3, SOC.8, LAB.6). **The benefit**: a person laid off or released by a failed firm claims when
the benefit's monthly amount (its replacement share of the wage lost) times its months is worth more than the claiming
hours at its last wage; the claim (`Core::claim_benefit`) is a contract from the treasury on a monthly schedule from
the claim's date, ending after the benefit's months, claims begun on one date sharing one schedule
(`DatedFamily::monthly_from`); a hire ends it. **The state pension** (`Core::claim_pension`, a placeholder naming SOC
until S5.02): a person who retires at or past its sex's pension age claims its country's pension where a draw fixed for
the person by the keyed stream `SOC.pension_covered` falls within its sex's coverage: a contract from the treasury
paying its sex's flat amount — the replacement rate of the country's mean wage at the opening, as the pensions in
payment at the opening are — monthly from the pension family's next date, naming the person, for life. Each day's
retirees and claims are counted (`PopDay::retired`, `claimed`; LC-1-53). **Public agencies** (kind `agency`, legal form
`public agency`; `core_agencies`): each country has one, producing its public administration, which is no product and
is paid by taxes. At the opening it takes each occupation's public administration share of its jobs in every region —
public administration's hours of the occupation over all the employees' hours the output asks of it
(`opening::asked`) — and every job no firm's way takes (the armed forces'); the rest are dealt to firms. Its head buys
the state's final uses at retail (`SOC.consume`) and keeps its staff: each business day it posts what it lacks of its
opening staff by region and occupation, as far as its appropriation for wages pays (`SOC.staff`; the appropriation is
its opening staff's wages a month, a placeholder naming POL until the budget votes it); its vacancies are met, selected
by the head of its service and raised when they stand, as any employer's, and a person it hires leaves the job it
held, whoever the employer. Its account is at the issuer beside the treasury's, the state's money, and its treasury
funds it each day for what it pays beyond what it holds (a flow of `FUNDED`), so a treasury short of cash leaves its
agency's wages and purchases unpaid. The national accounts value its output at its staff's wages: by production and
expenditure they are added, by income they are wages, not taken off the firms' surplus. Its staff's pay rounds wait for
public pay scales (POL).

**`sys-sov`** (SOV; `core_bills`). Each country's auction runs in its fund stage on its declared weekday, before the
facilities' requests. The minister sizes it (`SOV.size`, through the decision core; the placeholder plan naming TRS):
the face that keeps the treasury's cash at `SOV.buffer_weeks` of its last week's outflow — its cash after the last
auction's proceeds less now — after the week's maturities, in whole bills. Each bank's chief executive bids
(`SOV.bid`) its reserves above its target at the price whose yield over a bill's weeks is the deposit facility's rate;
the uniform-price clearing takes bids from the highest price down, the last price's sharing what is left pro rata in
whole bills, and what is not sold is not issued. A bill is a contract of the `SOV.bills` family from the treasury to
its holder, its balance the price paid, reckoned at the yield its price gives and paid in full at maturity, so a
treasury short of cash falls into arrears on it. At the opening each bank holds the government paper its sheet gives
it as bills maturing one tranche a week over a bill's weeks at the deposit facility's rate; the central bank's paper
waits for its purchases (CB.5) and the households' for their portfolios. Each auction's offer, bids, sales, price,
cover and tail are kept (the run report's `auctions`, LC-1-31); each issue is held to what its holders paid while it
stands (LC-0-16); and at each close the debt family holds the bills outstanding to what was issued less what was
redeemed and written off (TRS.6, LC-1-29).

**`sys-tax`** (TAX.2, TAX.5; `core_taxes`). A tax arises on its base's payment once it settles — income tax withheld
by the employer from the wage it pays (the wage flow carries the net), the tax on products a final sale pays on top of
its price, charged by the seller — and is then its collector's debt to its treasury, a contract reckoned from terms (its balance paid in full on
`TAX.remit_day` of the month after, asked again each month while unpaid, a claim in its collector's estate); the
collector's tax is entered when it arises. As wages are paid in one family per payer kind, each wage family declares
the collectors' family its payers owe in (`families::COLLECTORS`: `LAB.employment` → `TAX.collected`,
`LAB.public_employment` → `TAX.collected_public`), opened with the wage family's payer kind as its first side and the
treasuries' as its second; the core's `collectors` index, each kind's collectors' family, is built at the opening and
rebuilt after a load (never saved), and a collector of a kind with none stops the run (TAX.5). The one-family-per-
collector-kind form stands until the contract store takes sides of any declared kind (S1.257). The treasury paying
its own staff collects to itself. At each close what arose is held to what was remitted, what collectors owe and what
their debts owed when they closed (every collectors' family's `lost`), a difference a finding of the taxes family
(LC-1-28). The
income tax withheld from sampled households' wages is kept with its gross wage, and LC-0-29 recomputes it band by band
under its rounding.

**`sys-trs`** (TRS.10; `order_payments`). Each treasury's payments are ranked by `TRS.payment_order` — its debt
service (its bills' redemptions), its pensions, its benefits, its public staff's wages and its public purchases —
carried on each flow's order, so a treasury short of cash fails the last of them first; its agency's funding is two
flows, one for the day's wages and one for the rest, each at its rank, and the agency's own payments take the same
ranks.

### 3.6 Assembly and applications

| Crate or project | Owns |
| --- | --- |
| `phx-world` | The registry and schema compilation (§5.3); the opening's orchestration (§10); the day runner walking the stage table (K-23, §7.18); save orchestration (K-104, §11); the player's decider (§12); the inspector. It assembles and routes: once the core closes (S1.360) it holds no store and no per-party pass, `party_map.rs` and every `core_*.rs` store holder having been deleted by the migrations. |
| `phx-obs` | Read-only views: a party (OBS.8), the map (OBS.12), and in the inspector build the realism recorder (§14.8). |
| `phx-fin` | The finished-volume measure: one module per base, each filling its base at the design point (`perf/design.toml`) and timing it (§14.7; created at S1.116). |
| `phx-cli` | `run` (with its report, each day's stages timed, which `tools/bench.sh` reads), `inject` and `fin`; the live-check suite. `realism`, `chains` and `register-report` arrive with Stage 7 (§14.4). |
| `phx-play` | The bridge to the Android app: create, load, step a turn, read a page, submit, save; no world state, reading the world only through `phx-world`'s entry points and `phx-obs` (created at S6.137). |
| `android/` | The Compose app, its `play` flavour. |
| `phx-check` | Law, layering and document checks (§16), the core's among them. |

The `android/` app's AGP, Gradle and Kotlin versions are pinned in `gradle/libs.versions.toml` and the wrapper. The
engine has no Android library: the one bench (§14.7) measures on this machine, and the phone's run of the world is
built on the same report when the owner calls the device run.

### 3.7 Repository

```text
Cargo.toml · rust-toolchain.toml · rust-toolchain-miri.toml · clippy.toml · .cargo/config.toml · CODEOWNERS
crates/{foundation,kernel,interfaces,systems,assembly,apps}/
android/  data/  perf/  docs/  tools/{bench.sh,versions.toml,data/}  .github/workflows/ci.yml
```

`data/world.toml` holds the calendar's constants (the epoch, day zero) and the world's units; the generator's constants
— the population, the countries and their regions, the map's size, the settling length — are `data/shared/GEN.toml`'s.
`data/shared/` holds the register's shared files, one or more per system (`<SYS>.toml`); `data/setup/` the default new
game; `data/profiles/<level>/` each development level's four files — `economy.toml` (the accounts: ways, prices, the
flows' primitives, stocks), `people.toml` (demography, households, education, labour force, wealth, banking),
`law.toml` (every policy its law sets), `profile.toml` (the joint draw of a country's derived values) — each primitive
in the one that describes it, and `later/<SYS>.toml`, tables of a system a later stage builds, which a new game copies
only with it; the derivations write every profile primitive by id through `tools/data/profile_files.py`;
`data/names/` the name tables (§10.0);
`data/observer/READS.toml` the observer's declared macro reads, which no world crate reads; `data/inventory.toml` the
opening's inventory of derived values and distributions with their sources; `data/sources/` the fetched raw series and
their notes. `tools/data/*.py` fetch the sources and derive the profiles from them; `tools/versions.toml` pins every
tool beyond the Rust toolchain (the NDK and Android levels, the cargo tools, the nightlies `miri` and `public-api`
use). A country's `data/<country>/` is instantiated at a new game into the run's directory, never
committed. `data/measure/` holds the realism reads' registered definitions, which no world crate reads, and
`perf/{realism,chains,register}/` their reports, append-only (§14.8). They are registered before any read of the world
uses them: N3's facts `N3/F01.toml` to `F28.toml`, N4's chains `N4/L01.toml` to `L12.toml`, and the shared
`CREDIT.toml`; later work adds estimators, never edits a definition. `phx-check`'s `preregistration` and `no_tuning`
rules guard them (§16 item 10). `perf/budget.toml` holds the budget's ratchets, `perf/ratchets.toml` the counters',
`perf/design.toml` (written at S1.113) the design point — its counts, daily volumes, the phone model and the RESOLUTION settings it is
measured at, which the world never reads (§13) — and `perf/bench/` the bench's kept reports (§14.7), which a
resolution change cites.

---

Each system's opening technology and distributions are derived by a pair of scripts, `fetch_<sys>.py` into
`data/sources/raw/` and `derive_<sys>.py` into `data/`, each value its country group's median over the economies the
sources report, a group reporting too few taking the developed group's, marked assumed:
- **TEC** (`data/shared/TEC.toml`, `economy.toml`): nineteen products aggregating the industries of
  the OECD's inter-country input-output tables (2019, 76 economies), each product's 2019 dollars carried to 2022 cents
  by the US GDP deflator; one opening way per product per country group — inputs per unit made from the tables' uses
  summed over origins (energy carriers turned into quantities at world prices, as the fuels they are made from), energy
  mining's output split into coal (to electricity) and oil and gas, other mining's into
  ore (to basic metals) and stone, hours by ISCO-08 major group from ILOSTAT's employment and hours by ISIC section
  shared among a section's industries by value added, plant by kind per unit of output a year from the OECD's
  net fixed assets (Table 9A) per unit of value added (Table 6), agricultural land (World Bank) for crops and
  livestock, a kilogram of deposit per kilogram extracted; a growing season's lead time for crops and livestock, a day for
  other goods, none for services; every unit started finished; a batch of one.
- **FRM** (`economy.toml`, `FRM.firms_per_employed`): enterprises per person employed by product (OECD SDBS for the
  developed group, crops and livestock from ILOSTAT's employers and own-account workers; ILOSTAT's for the emerging and
  developing groups), the firms' density over each product's employed. Finance, real estate and public administration
  make no product of the ways; `TEC.labour` carries their hours a unit (a currency unit of their output) after the
  products' columns, public administration's staffing the agency.
- **LAB** (`people.toml`): `LAB.self_employed_shares`, each product's employers and own-account workers over its
  employed (ILOSTAT, status by activity; the source counts no other status apart, so contributing family workers are
  employees here); `LAB.women_by_occupation`, women's share of each occupation's employed. The occupations' mix and the
  share of employees are not primitives: the opening draws them from what the output asks (§10.3).
- **CAP** (`data/shared/CAP_kinds.toml`, `economy.toml`): each kind's geometric rate (the BEA's
  current-cost depreciation over its net stock), mean service life (the BEA's declining-balance rate over the
  geometric), efficiency shape (the BLS's and ABS's hyperbolic β), lead time (the Census's construction months for
  structures, the M3 survey's months of unfilled orders for equipment) and the product it is bought
  as; each group's net stock of each kind per unit of GDP (OECD Table 9A over Table 6), a group the OECD does not
  report taking the developed median scaled by its Penn World Table stock of each asset per unit of GDP; cultivated
  assets' lead time a dairy cow's months to first calving, intellectual property's none.
- **GDS** (`data/shared/GDS.toml`): three grade classes per extracted product at the terciles of GEO's log-normal
  grade index; the grade's fall as a deposit is worked; each storable product's yearly spoilage in stock and the room
  it is kept in; the standardised products (crops and livestock and the four extracted), which meet in calls. Each
  group's product price levels (`economy.toml`, `GDS.price_level`): a product's ICP 2021 price
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

### 4.2 Messages

A **message** is an addressed record: kind; sender (a party, or an agent with its count); addressee (a party, or a line
side with a count); issued day; due day; the line or instrument it concerns; state (open, answered, lapsed, failed).
Each kind declares, per addressee kind, the **answering system** and **the sub-step it answers in**, and, where an
answer can be accepted, the **acceptance handler**. Assembly refuses a kind that reaches an unanswered addressee kind.

A message to an agent becomes a **notice occasion** for it (REP.21). A money demand opens a commitment
(REG.10). High-volume kinds (job applications, retail requests) carry counts and are **day-local**; kinds that live
across days (quotes, offers, redemptions, calls, claims) are snapshotted (SET.16).

**Open business belongs to its agent**: a message addressed to or from it, a need or notice occasion carried to a later day (§7.7), or a commitment (an accepted mortgage offer, a pending sale, a trade that settles later than it
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
and payee; whether it is the collector's liability until remitted (TAX.2), carried on an accruing contract; its order;
and, where the payee system turns the money into something the holder holds, its **follow-on**: an instruction written
by the payee system from the levy's legs (a defined-contribution subscription into fund units at the next net asset
value, PEN.3; an accrual of defined-benefit rights on the holder's DB row, a leg in a unit that is not money). The amount for a side entry is computed **per contract, rounded per contract, then multiplied by the count**
(REP.9, TAX.7). The core composes levies into the flow's instruction; the levies of one flow that share a base are
evaluated in one pass with one search of their fused kinks, each rounded by its own convention. A **holding levy**
(property tax) is declared on held classes, not on a flow: on its dates `phx-geo`'s (zone, class) index lists the holders (§7.4, K-27), and each holder's amount joins its payments that day (§7.8). A crossing is not a flow,
so duty and import tax at a border are not levies: customs computes them per shipment through the tax's rule handles and
issues a demand (§4.2).

As built at Stage 1, income tax is a `phx_ledger::levy::Withholding` per currency, set in the core's state when the world opens or loads, its payee the country's treasury: a wage's levy is the year's share of `TAX.income_band_*`'s marginal bands over the wage times the payments a year (a non-cumulative basis, since no year-to-date positions are kept), rounded once; the core takes it from each wage the day's dues pay (`Core::withhold`), and the employer, its collector, remits it to the treasury on the tax's remittance day. The taxes on products are the accounts' (`GEN.product_taxes`): a final purchase — households', the state's, fixed investment — pays its final use's tax a unit spent on top of the posted price, which is the seller's basic price; the seller owes the tax, paid to the treasury in the sale's instruction, and the product's mark counts the price before it (`meet_all`, `arise_tax`). A firm's inputs pay none. The statutory rate (`TAX.consumption_rate`) is retired: the standard rate of value-added tax, 20% in the developed group, is not what households pay a unit spent after exemptions and reduced rates, 10.9% in the accounts, and at the standard rate taken out of basic prices each household sale cost its seller a sixth of its price. A country's value-added tax charged along the chain remains a placeholder naming TAX.

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
carried as a **point index** into that trade's point table; a line kind with no point table carries its amount itself.

A **line** is one record of identical contracts (REP.3): kind, interned terms, and its holders. An
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
money leg, up to the grant's undrawn limit (TCR.1). An offer of held units binds them: the core's goods keep a
holding's units free, committed to sales or pledged to carriers (`phx_core::goods::Stocks::bind`), and only free units
are bound, so no unit covers two offers (MKT.16, REG.16).

A **line transfer** moves a count of a side to a new party — a sale of loans (BNK.10, SEC.2), a client moved to
another clearing member (DRV.9), an estate succeeding a party (L3), a foreclosure (HSG.11) — as an instruction with a
reason, settled by the settlement routine with its money legs; each line kind declares which systems may request which
transfers. A **split at a kink** divides each row of a line by a per-contract amount — the insured part of a deposit
(SUP.2, SUP.5) to a receiving bank, the rest to a claim line on the estate — computed per contract and multiplied by
the count, so it is exact; it works over any row kind's per-contract amount (a benefit per year under a protection
scheme's limit, an accrued pension under a guarantee fund's cap). A **stay** is a line transfer too: at an insolvency
procedure's opening the debtor's rows move to **procedure lines** of the same kinds whose terms carry the stay, so a
stay suspends only the debtor's dues and never a line other holders share.

A line's transfer moves a row's count with its balance pro rata, rounded by the transfer's declared convention.
`split_at_kink` takes a holder's rows in the declared coverage order, each row's part per member within what the
per-person limit times the holder's persons has left, bound against the member's share, times the row's count, and the
rest of the row's total beside it, so any total splits exactly.

### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run. Its opening value comes from `data/`;
after that it is a fact whose one writer is the owner's decision. Each
change writes a dated announcement with its effective day, at least the next business day (VAL.6, POL.7). A change
that moves a hazard's rate books its process afresh on the parties it concerns (§7.7).

A `PolicyValue<T>` holds its opening value and its announcements in effective-day order, and is saved;
`value_on(day)` is the last effective on or before the day. `announce` refuses a writer other than the owner's
decision and an effective day before the next business day of the policy's country after the announcement.

### 4.7 Decision points, deciders and rule handles

- Every decision the spec names is a **decision point**: an input view type and an output intent type from an
  interface crate, a rule function registered by its system, a declared **schedule** (TIME.5) and **wake conditions**,
  and a pure evaluation form usable off-world (REP.15). The **decider** — the player, an office's holder, an
  institution's founding preferences, a household or a person — is named each time the decision is taken by the
  decision core (§7.16), never stored per decision.

  A decision point's rule is one pure function and is its own evaluation form, so REP.15's estimates evaluate the
  function that decides. A point needs a schedule or wakes. The decision runtime exists once: the decision core in
  `phx-world` (`core_decide.rs`: `decide`, `decide_own`, the decider's standing; `core_player.rs`: the player's queue)
  says how each decision is taken as a `phx-core` `Say`, and `Say::take` takes it — the player's queued intent when one
  is queued (one that does not decode stops the run), else the rule on the decider's preferences, or nothing that day
  when the player keeps the decision and queued nothing. The rule table's check refuses a rule signature with no implementer or two, one implemented by
  another system than its declared one, and an implementation of no declared signature. Each point the day takes is bound once to
  its place among the declared decisions (`core_decide::Points`: the systems' points and those the labour market's,
  the bills' and the benefit's kinds name), after the decisions and those kinds open and again at load; a day reads
  the bound place (`Core::point`), and a point bound to none stops the run. The primitives a day path reads are
  compiled at the opening (a product's lot and lead time, the retail meeting's weights), and the systems' own states
  the day reads are found by their codes once (`world::OwnAt`); the register is read by name only in `#[opening]`
  functions.
- A **rule handle** is a pure function declared in an interface crate and implemented by its owning system — the tax
  on a given income, a benefit entitlement, a lender's cap, a clearing house's margin for a trade, a platform's effect
  on a household — callable by any system with inputs it may read. It reads only its inputs and policy values. Market
  **admission hooks** (DRV.6) are rule handles, taking a member's whole order set at a meeting (§8).

### 4.8 Keyed reductions

`KeyedReduce<K, V>`: chunk-local sorted runs, merged in parallel by owner-key shard, each shard in chunk order. The
only way a pass sums over rows it does not own, and the way every **global structure** — the terms interner, the
directory, line creation, holder lists — is updated: sharded by key hash, each shard applied
by one worker, so nothing serialises and no atomic is needed.

`KeyedReduce` stable-partitions each chunk's pairs by `mix64(key)` (the SplitMix64 finaliser, never `foldhash`, so no
crate version decides a shard) into 256 shards; each shard concatenates its partitions in chunk order, radix-sorts
by key and folds equal keys in (chunk, position) order, so a non-commutative fold gives one answer.

### 4.9 Records, audiences, scoped reads

Decision kernels read what their party may: its own rows and facts; the relationship rows it is holder or counterparty
of; records whose audience includes it; earlier prints and marks. Audience is checked at
compile and assembly time from declarations; `read-trace`, a run-time flag of release builds, on in every gate's run
(§14.7), samples chunks and verifies reads at run time. Store-wide views go only to processes and applies.

The sample is the first chunk of each (handler, table) in the run and every chunk whose index is congruent to the day
modulo 64. A traced chunk's writes are stamped with their (sub-step, handler), and each read is checked
against the reader's declaration and against a later stamp; every `Streams::open` is recorded unsampled, and at the
close the day's opens are sorted and each repeated (stream, subject, sub-step) counted. What the trace
finds is reported through the audit's Time family (§15); the stamps live for a day and the log is kept outside the
world hash.

A record kind declares its audience, its horizon and its writer; entries are dated by (day, sub-step) and are world
state, hashed and saved. A record store's read by kind, reader and calendar returns only entries whose audience includes the
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
}
```

The trait and everything it declares with live in `phx-core`, so systems (L2) and kernel crates (L1) use them without
reaching the assembly. Declarations generate typed handles with private constructors: using something undeclared does
not compile, and an unused declaration fails `dead_code`. A reference across systems — a writer, a reader, an answering
system, a transfer requester — is a system's code as a string, not a compile-time handle; assembly checks the
references §5.4 names.
Registration is one line in `phx-world/src/systems.rs` (each interface crate's items are listed beside it); its order
carries no meaning (§6.2).

### 5.3 Schema compilation

Declarations compile into: each kind's layout and its persons' (K-21, K-32); point tables; message, instrument, market and levy profiles; the stream
registry; the primitive register checked against `data/` (NUM.3); schedules and wake conditions; audits and metrics. An attribute or count field whose value outgrows its width is a **contract violation** calling for a
layout change; nothing saturates (Law 6). A policy schedule's count of bands is declared with it (the constitution's,
POL) and fixed here: a platform or a budget moves its values and edges, never the count.

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

A fact with no writer or two; a levy without a schedule owner, or a follow-on not
written by its payee system; a kink on an undeclared position; a primitive without value, unit, kind or source; a
decision point without an evaluation form or schedule; a rule signature without an implementer; a commitment kind without the legs it creates; a hazard without a draw scheme (REP.7);
an interface item whose writer is not registered.

  For a population kind, every refusal is reported at once: a name declared twice, an attribute or person attribute
  of no values, roles or person attributes beyond a person's word, a kind sited by two attributes or by one it does
  not hold.

  A legal form is refused whose owners fall outside the forms' vocabulary (`Owners`: the state, shareholders, its
  members, its heirs and creditors), or whose owners hold its equity (`Feature::HasOwners`) while being its own
  members.

  A kind is refused whose place (`KindDecl::place`: its site's tile, its region or its country in a word of its
  record, or `Sited` by its population declaration) is read from a word beyond the record its parties are begun with,
  or which is sited by a population declaration that sites it by none.

  The opening refuses a primitive absent for what the world holds, naming it, rather than reading it as zero: a product
  with no lead time (`TEC.lead_time`, compiled once into `CoreGoods::lead`), a country with no lending rate
  (`GEN.lending_rate`) or labour law, a product with no opening price in a country (`GDS.opening_price`, read for the
  opening stocks and plant), a country with no drawn growth, and accounts short of the shape the opening reads
  (`opening::economy::shape_breaks`: an input of each activity to each, a final use's take and value added's parts
  of each activity, the taxes one an activity and one a final use, public administration among the activities and the
  sale's final uses among the final uses). Past the opening, a lead, rate or law read for a product or country the
  world does not hold stops the run (`CoreGoods::lead_of`, `at_country`); a zero the dataset writes is read as zero.

---

## 6. The day

### 6.1 Turns, stages and slots

A **turn** advances the world to the next day that is a business day in any country, running each day in between as
its own day (N8.2). On each day, each country's business-day calendar decides which of its markets and institutions
act (TIME.2). The day is the **stage table** (K-23, §7.18): TIME.6's ten stages, each a run of **slots**; a slot
marked **B** runs only for the countries whose business day it is, and on a non-business day only the slots TIME.8
lists run. A slot runs on the pool over the bases it names, or is an **apply**, where the intents of its stage are
applied by target range (§6.2); a barrier follows each slot that ran.

| Stage | Slots (bases) |
| --- | --- |
| 1 Open | 1a today's buckets taken: dated rows, completions, arrivals, lapses, commitments and messages (K-42), the parties due (K-43) and the hazards among them (K-44), the day's facts (K-19), the policy values in force (K-20), terms due resolved (K-55) · 1b offers and messages lapse (K-70, K-45); the player's intents become wakes (K-100) |
| 2 Resolve | 2a accruals where a date needs them (K-55 → K-87) · 2b (B) dated items streamed into both parties' `pending`, levies fused (K-51, K-47, K-12) · 2c (B) resolutions opened; calls and demands due (K-99, K-45, K-48, K-49) · 2d (B) fails become arrears; the day's status transitions (K-56) · 2e (B) instrument events to holders of record (K-65) · 2f (B) losses land, parties end, estates distribute (K-97, K-96, K-57, K-98) |
| 3 Nature and population | 3a weather and catastrophes (K-30, K-27, K-05, K-60, K-26) · 3b hazard hits followed (K-44, K-33, K-31, K-69) |
| 4 Real work | 4a production realised at its kinks and visits, wear, processes arriving, jobs starting and ending (K-68, K-67, K-69, K-53) · 4b every trip of the day placed, adding its legs' loads to the segments it crosses (K-26) · 4c each trip's time and cost read at the loads all the day's trips put on its legs: the day's fixed point (GEO.20), never the load of trips placed before it |
| 5 Decide | 5a public outlooks (K-92) · 5b continuous decisions of the parties due (K-43, K-100, K-35), wants bucketed (K-03) · 5c (B; on a non-business day the occasions TIME.8 admits) lumpy decisions and answers (K-101, K-45, K-80) · 5d apply: offers posted through the shared admission hook, the agenda refiled, triggers registered (K-70–K-72, K-43, K-46) |
| 6 Form prices | 6a meetings (K-73, K-75, K-82, K-76, K-78, K-79, K-81, K-84, K-83, K-77), capacity used (K-85) · 6b the seller-range sweep, (seller, good) batches closed (K-74, K-87, K-51, K-39, K-68, K-60) · 6c prints, marks, fixings, curves (K-86, K-91) · 6d apply: decided matches, banknotes, non-business-day commitments, trades for later settlement (K-48, K-47, K-50, K-71) |
| 7 Settle (B) | 7a the `pending` sweep by account range (K-47, K-48, K-50) · 7b short payers re-derived, the fixed point and the ring, link groups (K-49, K-74) · 7c rows opened and closed, fails recorded (K-53, K-54, K-36) |
| 8 Fund (B) | 8a–8f money-market and tender orders, the linked call, its settlement, the administered facilities and their settlement, a shortfall's end (K-77, K-76, K-49, K-81, K-93, K-47, K-53) |
| 9 Value and judge (B) | 9a valuations, provisions, margin (K-91, K-95, K-89, K-94, K-14, K-35, K-92) · 9b thresholds, books, net asset values, quarter-end books, holder rows (K-88, K-46, K-93, K-87, K-90, K-65, K-41) · 9c tests; demands due the next business day; resolutions triggered (K-45, K-99) · 9d publications; triggers crossed (K-37, K-39, K-46, K-43) |
| 10 Close | 10a public events; an election's tally (K-36, K-41) · 10b declared sweeps (K-14) · 10c the audit (K-103; on a non-business day it recounts `pending` and units only) · 10d the day ledger, accumulators, the recorder, life-record buffers, horizons, store statistics (K-38, K-39, K-106, K-40, K-08, K-15) · 10e slots released, day buffers reset, caches expired (K-02, K-31, K-03, K-35) |

A save is taken at its declared moments, apart from the turn (K-104, N8.10). Money moves only at 2c, 6d
(banknotes), 7 and 8; on a non-business day a purchase is paid in banknotes or recorded as a commitment settling at the
next business day's stage 7 (TIME.8). **Day zero** runs stage 5 alone, on the snapshot (GEN.13); **settling** runs
ordinary days for GEN.6's length.

**Today** (until S1.185 and S1.186): the day is `phx-world/src/day.rs`'s hand sequence over the sub-steps
`phx-core/src/substep.rs` names; the stage table's step (S1.185) compiles the table above into data, and the day
runner (S1.186) walks it.

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

A slot reads today's writes of earlier slots only: stage 5 reads yesterday's prints and publications, stage 9
today's (TIME.10), each slot's read tag declared in the stage table (K-23). **Decide, then apply**: a decider reads
`&self` over its agenda chunk and writes only its own party's columns; every other effect is an **intent** into a
(chunk, handler) buffer, applied by target range at its stage's apply slot. Ties are broken by identity, then rule,
then lot, so no outcome depends on registration or processing order, which a logic-level test holds: the canonical
ids a registration list yields are the same for every order of it (§14.3). Nothing is demanded and paid in one stage
(TIME.7).

Today's settlement runs on the world's pool: its grouping, its passes, a closed day's commitments, the fund stage and
the families' wheel takes (`Core::run_day`, `pay_currencies`, `fund_stage`). The day's flows are in one chunk for each
family's dues, in the families' order, then one for what the day's stages made. Grouping places each chunk's flows by
their payers' ranges stably (`FlowBufs::group`, a counting sort), so a payer's flows, read chunk by chunk, are in the
order they were made however the day is cut into chunks; its lots are drawn in that order and its payments ranked by
(order, lot, place). The outcome is the same for any chunking and any number of workers. The employers', spenders' and hazards' wheels are taken on the
pool and hiring's searches run on it in fixed chunks (`SEARCH_CHUNK`), each returning in its items' order. The families
make their dues, and the hazards' bookings are followed, in turn until the kernel's chunk plans (S1.169) give a
traversal to run them on the pool.

### 6.3 Traversals

A slot traverses one of four ways: **agenda chunks** (K-11's `for_agenda`, the parties due in slot order), **market
keys**, **party ranges** (K-12's `apply_by_range`) or a **declared sweep** (K-14), counted in the sweep ledger and held
by its ratchet. Chunk bounds come from rows and declared costs, never from the worker count; barriers are at most 40 on
a business day, 20 on a non-business day and 48 on a heavy day, each spinning at most 20 µs before parking; the
runner's own overhead is at most 2 ms a day (§13).

`phx-exec`'s primitives give the same result whatever the worker count. Workers take chunks as they free, but each
result is stored at its chunk's index; gathers concatenate in (chunk, canonical handler) order; `reduce_tree` folds a
left-balanced pairwise tree whose shape depends only on the number of results. Radix sorts are stable LSD over
11-bit digits, skipping a digit every key shares, with a stable comparison sort below 2 048 pairs. `gather`,
`KeyedReduce` and `radix_sort` take the pool as an option and give the same result without one. The pool pins each
worker to a core whose `cpu_capacity` is at least half the largest (all allowed cores when capacities cannot be
read; a pin refused leaves the worker unpinned and counted); idle workers spin a bounded number of rounds before
parking. Only `pool.rs` and `site.rs` read a worker's index; atomics and threads exist only inside `phx-exec` and
never appear in its API. Wall time comes from an injected `Clock` and reaches counters and the application only
(TIME.11).

### 6.4 Compute, gather, apply

Work writes its own rows and emits intents into (chunk, work) buffers; gathers place them by prefix sum; applies run
in declared order, in parallel over disjoint targets (K-12). Every reduction runs over a fixed tree.

### 6.5 Settlement

Stage 7 settles the day's flows through settlement (§7.8, K-49): the greatest set of payments that can settle given one
another, each payer failing as a suffix of its payment order and a bank still short losing its customers' payments;
flows through a closed bank pending; money changing currency only by a trade (K-49). The day's flows are grouped by
party range (K-48) and their tallies fused into the sweep (K-52).

### 6.6 Cost bounds

**The rule** (the owner's, plan §12): every operation a day performs costs at most **O(log n)** in the size of any
store of the world — parties, holders, contracts, persons, instruments, records, events, history — and a day's cost
is the sum of its events' costs, each at most O(log n) (N8.6). A pass over a world-sized store is allowed only as a
declared **rolling slice**, a fixed share a day on a declared cycle (the audit's is `[resolution] audit_cycle_days`),
counted in §13.2. Nothing a day does grows with elapsed time. A kernel that cannot meet the rule is a finding, not an
exception.

The core's design rules, each held by a guard or a ratchet; every base designs to them and every review checks
against them. How each base meets them is its own section in §7.

| Rule | Guard |
| --- | --- |
| **R1** Columns per kind, dense by slot; no map, list or wide optional on a day's path. | PC-92 (S1.120) |
| **R2** Due-driven: nothing visits every party every day; whole-store work only as a declared sweep. | PC-96 (S1.121); the rows-visited ratchet (S1.133) |
| **R3** Decide, then apply by target range; the same result for any number of workers. | PC-97 (S1.122); busy cores per span (S1.133) |
| **R4** Maintained aggregates are integers kept by their one writer; the audit recounts them from source rows. | PC-101 (S1.128) |
| **R5** No allocation in the day; buffers sized by the heaviest day and kept. | PC-99 (S1.124); `faults_per_day`, `allocs_per_day` (S1.117, S1.133) |
| **R6** Per-item effects batched; kernels expose batch entry points only. | compile level, in each base |
| **R7** Per-day values computed once and day-stamped. | PC-101 (S1.129); `unit_cost_per_firm_day` ≤ 1 (S1.133) |
| **R8** One dispatch a pass over cost-sized chunks; inline when small; bounded spin. | PC-97; `barriers_*`, `spin_us` (S1.117, S1.133) |
| **R9** Saves hold live rows; every index is rebuilt at load. | PC-101 (S1.130); `fin.save.*` (S1.132) |
| **R10** Every base counted; every span a unit cost. | S1.117; the unit-cost ratchets (S1.132) |
| **R11** O(log n) per event; no per-round rebuild. | `meeting_work_per_sale` (S1.133), `fin.retail.work_per_sale` (S1.132) |
| **R12** A miss budget per item: partition by target, then sweep. | the probe's gather rate in every report (S1.117) |

---

## 7. The core

The core is the kernel's bases: every store, index, traversal and kernel the finished world needs, one crate each in
L1 (§3.1), designed at the design point (§13) before anything is built on it. Systems hold rules and declarations and
read the core through views (§3.5); `phx-world` assembles and routes and holds no store (§3.6).

### 7.0 The core's discipline

Status: planned (S1.103–S1.104)

Every base keeps the cost rule and the design rules R1–R12 (§6.6) and is held to its ratchets at the design point
(§13, §14.7). Each subsection below is one crate: it opens with its status and the steps that build it, and has one
heading per base, which that base's step fills with its design before its code: its layout, API, algorithms and their
bounds, traversal, save and load, capacity, volumes and ratchets, and extension points. A base whose code stands
today keeps a **Today** paragraph, naming what the code holds by file, until its migration deletes it. The
foundation's bases — K-16 in `phx-num`, K-17 in `phx-id`, K-18 in `phx-rand` — are written in §3.2.

PC-92 (§16) holds every module a day's work runs through to R1: the core's crates, `phx-world`'s day modules and the
systems' daily rules, less the cold ones; today's sites are admitted by its exceptions file, which each migration
shrinks.

### 7.1 phx-store

Status: K-02, K-03 and K-05–K-10 built (S1.159–S1.168)

Every store base implements `StoreStats` (`stats.rs`): its rows live now, its rows ever and its bytes, which the
counters sample (K-15) and the world never reads. A kind's `Parties` and a table's `SlotAlloc` report their live and
ever-handed slots and the bytes of their slots, generations and identities; `phx-core`'s `KindStore` adds its records, accounts and cash lines. The rows a walk visits are counted by the traversal that walks it, in
`phx-exec`, since no `phx-store` code may share a counter across workers.

#### K-01 Columns in reserved address space

No step rebuilds it: the columns stand as below, and the core's close (S1.360) writes their capacity and ratchets at
the design point.

**Today** (`backing.rs`, `column.rs`, `table.rs`): a column reserves address space for its declared maximum rows
through a `Backing` (`mmap` with `PROT_NONE` and `MAP_NORESERVE`, committed by `mprotect`, decommitted by
`MADV_DONTNEED`; a zeroed heap allocation under Miri), and commits pages as it grows, at the page size read from the
system; reservations count no resident memory, and all of them together stay within 64 GiB (`VA_BUDGET`). Growth past
a reservation is `capacity_exceeded!`. Rows sit in chunks of a power-of-two `rows_per_chunk` declared per table (4 096
by default), whose disjoint `&mut` views are what parallel work owns. Nothing reallocates, and no store holds a
heap-owning type.

#### K-02 Slots and generations

**Layout** (`table.rs`, `genref.rs`, `parties.rs`): a table's `SlotAlloc` holds its live bits (a bit a slot), its
free slots as a ring over a reserved `Region<u32>` (a power of two at or above the table's slots, of which only the
ring's places are committed: the ring doubles when a close needs more, moving its part before the old end to the new
end, so its pages follow the most slots ever free at once, never the capacity), and the day's released slots.
`Generations<T>` is a `Column<u32>` by slot for the table the marker `T` names, with the widest generation its
references carry: 24 bits for a party (`PartyRef`, `phx-id`), the full word for every other table. `GenRef<T>`
(`slot`, `generation`: 8 B, `Pod`) is the reference that outlives a day; `PartyRef` stays the party's one-word form.
`ShortRef<T>` is a contract family's link (family in the top byte, slot in the 24 bits below, the width
`phx-core`'s `SLOT_BITS` reads) and the low byte of the row's generation.

**API**: `SlotAlloc::alloc` takes the ring's head, else a new slot at the high water; `release` marks a live slot
free and appends it to the day's released list; `close_day` sorts the day's released slots ascending and appends them
to the ring's tail. `Generations::alloc(&mut SlotAlloc)` takes a slot and raises its generation (a slot's first row is
generation zero) and returns the `GenRef`; `resolve(&SlotAlloc, GenRef<T>)` is the live bit and one generation
compare, branch-free, and refuses a reference of another table at compile time. `ShortRef::resolve(live, generation)`
compares the byte, modulo 256. `Parties::begin` and `resolve` stand on these.

**Algorithms and bounds**: which slot a row gets depends only on the order of allocations and releases (CHN.6); a slot
released today is not handed out before the close (SET.7), and the free slot that has waited longest is handed out
first, so a busy table's low slots do not race their generations. A generation raised past its width stops the run
(`capacity_exceeded!`), never wraps. The **reference-life rule**: a slot turns at most once a day, so a short
reference's byte tells a stale row from a live one while its holder re-reads it within 255 days; a `ShortRef` is not
`Pod` and is held only in a `ShortRefs<T, H>` column, whose holder `H: Recheck` declares its re-read in a byte, so no
holder can declare longer (the due wheel's 128 days meets it; records and messages that outlive it hold the parties
and the facts copied at writing, SET.16). Both are compile-level: `genref_wrong_table`, `short_ref_outside_wheel`,
`short_ref_holder_too_seldom`.

**Traversal**: event-driven; the close walks the day's releases only, `O(k log k)` for `k` releases. Resolves are
fused into their callers' gathers.

**Save and load**: the live bits, the ring as it lies (its head, its count and its places), the day's released list
and the generations are primary state and saved; a restored table turns the ring from the same head, hands out the
same slots at the same generations and saves the same bytes (SET.15). Nothing is rebuilt.

**Capacity**: every table's slots are the capacity table's (K-24); a table past them stops with `capacity_exceeded!`.
A contract family holds fewer than 2²⁴ rows.

**Volumes and ratchets**: `tools/bench.sh -F refs` fills the design point's 8.8 M party slots in one table and, on
each day type, ends and begins up to half its `row_life` rows at drawn slots and resolves its `applies` references in
slot order. Over the turn: 9–10 ns a row begun or ended and 1–2 a resolve, `[fin.refs]` `alloc_ns` 15.4,
`resolve_ns` 2.3, `gen_bytes` 4. A row's cost follows the day's density: 6 ns on a heavy day, 16 on a business day
and 45 on a non-business day, whose few ended rows each meet a cold line of the live bits and the generations; in the
world the row's own columns are written in the same pass, and its lines are warm. The ring wraps by a mask (its places
are a power of two), and a release reads and clears its live bit in one touch of the word.

**Extension points**: the directory's generation column and FIFO reuse (S1.194), the keyed indexes' and the
interner's members (S1.165, S1.168), typed references (S1.174), messages' and records' references (S1.237, S1.240),
contract rows' 8-bit generation in their terms word (S1.257), and every later table named by `GenRef`.

#### K-03 Day buffers and the day plan

The buffers (S1.160) and the day plan that places them (S1.161).

**Layout** (`daybuf.rs`): a `DayBuf<T: Pod>` is its name, a `Region<T>` reserved at its declared capacity (K-24), a
length, and its longest length on each kind of day (business, non-business, heavy, the business day after closed
days). Its pages are committed in whole pages as it first grows past them, never released during the run and never
zeroed by the kernel: a page is zeroed once, by the system, when first committed. `DayBufs<T>` is one `DayBuf` to each
(chunk, handler) of a traversal.

**API**: `clear` sets the length to nought; `push`, `extend`, `as_slice`, and `as_mut_slice(len)`, which hands the
caller the buffer at a length to write in place — what lies beyond the old length is an earlier day's, and the caller
writes every element it reads. A push past the capacity stops the run naming the buffer. `mark(day_kind)` records the
day's length when it is the kind's longest. `DayBufs::chunks_mut` hands each chunk's handlers' buffers to the worker
that holds the chunk; `get(chunk, handler)` reads one in place; `join(into)` appends them all in (chunk, handler)
order, so the result does not depend on which worker filled which chunk.

**Algorithms and bounds**: append is a length compare and a store (a page commit only past the committed pages); clear
is `O(1)`; a join copies each buffer once. A buffer is sized by its heaviest day, never an average one, so after the
first heaviest day of each kind a day commits no page and allocates nothing. The day's nets are the accounts' pending
(K-47): no buffer holds a net per party.

**Save and load**: not saved — every buffer is empty at the close, and `DayBuf` implements no `Saved`
(`daybuf_not_saved`); after a load the first day of each kind commits its pages again, once.

**Capacity**: each buffer's capacity is its declared heaviest day's rows (K-24); past it the run stops.

**Volumes and ratchets**: `tools/bench.sh -F daybuf` reserves a buffer for each of the design point's daily counts at
its heaviest day and refills each on every day type, a word an item: 1 ns a word appended (the step's 0.8 VM ns), and
on each type's second day no page faulted and nothing allocated — `[fin.daybuf]` `faults_per_day` 0 and
`allocs_per_day` 0. The buffers' bytes are line 21's, stated by the day plan. **Slack** — memory resident but holding
no live row — is line 22: each column's one page tail, the keyed indexes' dead entries between compactions (K-06), the
free slots awaiting reuse and their generations (K-02), and the wheel's run tails (K-42); `-F all` reads it as what
the bases hold beside their live rows, `[fin.mem] slack_mb` 45.6 at the design point (57 at the steps' counts), today
1 MB with the bases built.

**The day plan** (`dayplan.rs`): every buffer declares its **life** — the slot it is first filled in and the slot
after which it is released, on the stage table's slots — and its bytes at the heaviest day, or `Rest`: the one
buffer sized to what the day's largest live set leaves beside the buffers sharing its slots (the dues' wave lane at
2b), so it reaches the maximum and never passes it. `DayPlan::plan` places each at the lowest offset of one region
where it meets no placed buffer whose life meets its own, the longest lives first, then the largest, then in
declaration order: two buffers whose lives meet never share a byte, and the plan is the declarations' alone, the same
on every run of a build. On the heaviest day's buffers (the design point's `[dayplan]`, each its owner step's figure)
this reaches the largest live set exactly: the region's extent is 6a's live set — wants, between-firm steps, meeting
scratch, spend, prints and the tallies — 160.1 MB at the steps' counts, 128.1 at the design point, §13's line 21.
`DayRegion` reserves the region at the extent and hands a buffer its lane at a slot of its life; a read outside the
life is a violation in debug builds (`check_read`, TIME.10's form: after its release its pages are another buffer's).
Its `StoreStats` report its committed bytes, and `lane_bytes` each lane's. The plan is build data, never saved.
`-F daybuf` fills the heaviest day's lanes in slot order and reads the region resident at `[fin.daybuf] peak_mb` 128.1.
A buffer outside the plan, or a live set figure other than the maximum over the day's slots, is refused.

**Extension points**: the owners of the planned buffers declare their lives in code as they are built (S1.170, S1.211,
S1.245, S1.248, S1.306, S1.326, S1.348); every later traversal's
intents, scratch and per-chunk staging are day buffers (S1.162–S1.171, S1.192, S1.194, S1.212–S1.225, S1.245,
S1.269–S1.312, S1.326, S1.348, S2.211, S4.130, S4.131).

#### K-04 Chunk arenas and block pools

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.360).

**Today** (`arena.rs`, `block_list.rs`): a chunk's arena is 8-byte words holding its rows' lists by
`ListRef { off, len, cap }`; a list grows in place while it has room and otherwise moves to the arena's end with 5/4 of
its need, rounded up to four words, leaving its old span dead; removal shifts down, so a sorted list stays sorted. When
dead words pass an eighth of the used, compaction copies every live list in slot order through a chunk-sized scratch
and decommits the pages above the new end. Only a chunk's own rows hold references into its arena. Lists that are not
a chunk's own are `BlockList`s: sorted sets held as a tree of 16-entry `u32` blocks (one cache line) from a shared
`BlockPool`, leaves chained for iteration and inner nodes keeping each subtree's least entry, split and merged at half
occupancy; adding a present entry or removing an absent one is a violation.

#### K-05 Sum-trees

**Layout** (`sumtree.rs`): `SumTrees` is a pool of Fenwick trees over integer weights: one `Column<u64>` holds every
tree's Fenwick array in an extent of its capacity class (4 · 2ᵏ entries), and a `Column<TreeHeader>` holds each
tree's extent offset, its length (the low 24 bits) and class in one word, and its total — 16 B a tree. A tree that
fills moves to an extent of the next class, the copy amortised O(1) a push, and its old extent is chained to its
class's free extents — each freed extent holding in its first two cells whether another follows and where, so the chain
costs no memory — and taken first by the next tree that needs one.

**API**: `make` a tree; `push(tree, w)` a member at the end, its node summing its own weight and those its span
covers; `update(tree, i, Δ)` the Fenwick walk, checked, a weight below nought a violation and a total past `u64::MAX`
a capacity stop; `set(tree, i, w)` an update by the difference from the weight, read in one short walk; `total` one
read; `prefix_before`; `find(tree, x)` by binary lifting from the highest power of two at or below the length: the
least member whose weights through its own exceed `x`. With `x` drawn below the total from the caller's stream, `find`
draws a member in proportion to its weight; a tree of total nought draws none (`Missing`), which the caller records
as a failed meeting, and `x` at or past the total is a violation. A removal is `set(i, 0)`: every member keeps its
place, so a stored position stays valid; re-packing a tree whose zeros pass half its length is its owner's, at an
apply point, rewriting its members' positions (K-06's weighted mode).

**Algorithms and bounds**: update, set and find O(log n); weights are integers the owner declares (a stall's weight its
price term quantised by the market's scale), so sums are exact and order-free and any worker count draws alike. A tree
is written by one worker at a time — its market key's job — and a meeting reads its own updates only.

**Save and load**: derived: its owner marks it `#[saved(skip, rebuild = …)]` and rebuilds it from its rows at load,
tree after tree in key order, which also leaves no extent stranded; equality is by members and weights, wherever the
extents lie (`rebuild_equals_incremental`, through the round-trip harness).

**Capacity**: the owner's declared weights and trees; the length is below 2²⁴.

**Volumes and ratchets** (`tools/bench.sh -F stalls`): the design point's 1.36 M stalls in 250 k trees, one per
(good, zone) at 1 000 zones (`[store] stall_keys`), built key by key; each day's reprices and sell-outs set in key
order as each key's job does, 16–17 ns each (`[fin.stalls] update_ns` 46); the day's retail wants, bucketed by key,
drawing a stall each, 7 ns a find (`[fin.sumtree] find_ns` 15); and 10 bytes of extents a stall at the classes' slack
(`bytes_per_weight` 12), the headers 16 B a tree besides (4 MB).

**Extension points**: keyed indexes' weighted mode (S1.165); the lot's draws over crossing pairs (S1.188); the
stalls' trees and the posted meeting's draws, retiring the per-round alias tables of `phx-market/src/meet.rs` (S1.301,
S1.306, S1.307); a catastrophe's draw (S2.211); a uniform member drawn (S6.122).

#### K-06 Keyed indexes

One kernel answers every "who is at this key" a rule enumerates — owners at a zone, parties by zone and kind, holders
by instrument — as an instance a system declares in data (`IndexDecl`, `index_decl.rs`: its name, member table, the
declared columns its key and membership read, its mode, its key space and the most entries it holds). It is kept by
the events that change a key, read in O(k) for a key's k members, compacted lazily and rebuilt at load; no index is
rebuilt per call or per day, and none is declared where a party's own chain, a declared sweep (K-14) or another
instance answers.

**Layout** (`index.rs`): an instance's key space is declared **dense** where its keys hold many members each — zones,
(zone, kind), instruments, tiles — or **sparse** where they hold few among millions — a party, a firm, a series.
- A dense key has a 24-byte head: its **run**, the slots the opening placed its members in key by key (the opening
  places a zone's parties, a tile's units, together), two words; its **lazy list** of members come since, a K-04
  block bag in the order they came, duplicates and leavers among them; and its count of known-dead entries.
- A sparse instance holds its entries as **(key, member) pairs**, sorted, 8 bytes an entry with no list of its own a
  key; the pairs come since the last merge wait in a sorted run a sixteenth of its entries long (4 096 at least),
  appended in O(1) as the applies bring them in key order, and merged into the pairs in one pass when it fills or at
  the owner's close.
- **Weighted** mode keeps a key's members in a sum-tree (K-05), a member's position being a field of its own row, with
  each key's free positions in a K-04 sorted block list; **count** mode keeps a key's count, maintained on the same
  events and recounted by the audit (R4); **counted** lazy mode keeps both.

**API**: `insert(key, member)` O(1); `left(key)` counts one dead and never searches; `members(key, valid, out)` appends
the key's members to a day buffer in slot order, each once, and returns the entries dropped: an entry is kept exactly
when `valid` finds its member at the key by its own columns now, so a leaver, a reused slot's other occupant and a
repeat are dropped — no entry carries a generation and no member row a back-pointer. `place_run` (the opening's and a
load's), `compact(key)`, `compact_pairs`, `settle`; `insert_weighted`, `update`, `remove`, `draw` (a key of total
nought draws `Missing`); `count`, `add`.

**Algorithms and bounds**: a walk writes its entries in place into its output, sized once; what it finds rises through
the run and a compacted list, so only the tail after that rising prefix is sorted and merged into it from the back
through the room past the output's end — linear, never a sort of the whole. A dense key whose dead entries pass its
live ones is compacted by its owner's apply at the close: its members rewritten in slot order, its run kept while at
least half its slots are still its members; a sparse instance's pairs are compacted in one pass when its leavers pass
its live pairs. The audit's rolling sweep (K-14) compacts the lists it passes. Every walk reports its dropped entries,
counted against `dead_share`.

**Traversal**: event-driven. Inserts and leaves come from applies partitioned by key range (K-12), so each list has one
writer in a stage and the pairs arrive in key order; walks are `&self` reads from any worker.

**Save and load**: derived — its owner marks it `#[saved(skip, rebuild = …)]` and rebuilds it from the member columns in
slot order, placing each key's run where its members lie together (`rebuild_equals_incremental`); counts recounted.

**Capacity**: the declared key space and entries; a key past the space, or an entry past the room, stops the run.

**Instances** (`perf/design.toml [index.instance]`, each its declaring step's members at the design point; line is
ARCHITECTURE §13's ledger line):

| Instance (declaring step) | Key (space) | Form, mode | Members | Line |
| --- | --- | --- | --- | --- |
| owners of physical units (S1.271) | zone (1 000) | dense, lazy | 2.6 M | 12 |
| rows by holder (S1.271) | holder (8.8 M) | sparse, lazy | 0.6 M | 11 |
| holders by instrument (S1.280) | instrument (61.6 k) | dense, lazy | 3.4 M | 11 |
| named units by tile (S1.284) | tile (40 k) | dense, lazy | 1.4 M | 12 |
| offers by poster (S1.296) | poster (0.92 M) | sparse, lazy | 2.2 M | 17 |
| stalls (S1.301) | (good, zone) (250 k) | dense, weighted | 1.7 M | 17 |
| vacancy runs (S1.304) | (instance, skill) (46.4 k) | sparse, lazy | 0.2 M | 17 |
| parties by zone and kind (S1.404) | (zone, kind) (32 k) | dense, counted | 3.35 M | 16 |
| searchers by region (S1.304) | region (25) | dense, lazy | 0.2 M | 16 |
| messages by addressee (S1.237) | addressee (8.8 M) | sparse, lazy | 0.5 M | 16 |
| queue memberships by claimant (S1.323) | claimant (8.8 M) | sparse, lazy | 0.3 M | 16 |
| registered pairs by series (S1.336) | series (0.1 M) | sparse, lazy | 0.1 M | 16 |
| loans awaiting workout (S2.101) | lender (35) | dense, lazy | 10 k | 16 |
| the bureau's subject index (S2.109) | subject (8.8 M) | sparse, lazy | 0.3 M | 16 |
| firms in a procedure (S2.123) | firm (0.92 M) | sparse, lazy | 3 k | 16 |
| shares by syndicate (S3.140) | syndicate (50 k) | sparse, lazy | 50 k | 16 |
| registrations by instrument (S3.148) | instrument (61.6 k) | sparse, lazy | 50 k | 16 |
| agencies by issuer (S3.207) | issuer (20 k) | sparse, lazy | 20 k | 16 |
| pots pending by (trust, fund) (S4.142) | (trust, fund) (16 k) | dense, lazy | 1.0 M | 16 |
| foreign-currency holders (S5.165) | currency (3) | dense, lazy | 0.2 M | 16 |
| imitation pools (S6.109) | (product, region) (6 250) | dense, weighted | 0.15 M | 16 |
| singles (S6.122) | (region, age class) (500) | dense, weighted | 1.7 M | 16 |
| employment by (zone, industry) (S2.174) | (zone, industry) (19 k) | dense, count | — | 16 |

Not instances: consumers by (region, priority class) on a shed day (S2.211) and persons by region (S6.125), each a
declared sweep; persons by zone for injury victims (S4.134 draws a household from parties by zone and kind).

**Volumes and ratchets** (`tools/bench.sh -F index`): every instance at its members, dense ones placed in runs as the
opening places them, the day's 0.3 M inserts and leaves applied by key, every key walked, the close's compactions, and
on a heavy day a flood's 0.2 M owner draws from 20 struck zones. Measured: 3 bytes an entry over the listing instances
(`[fin.index] entry_bytes` 5.4), the small-bases pool's instances 33.9 MB of §13's line 16 (`mb` 50), a dead share at
most 7 % after a day's moves (`dead_share` 0.5); a dense insert or leave 12–17 ns (`insert_ns` 21), a pair's 4–7 ns
(`insert_pairs_ns` 9); a dense walk 7–8 ns an entry read with its validity check (`walk_ns` 10), where a bare loop over
the same validity column runs 2.3 ns on this machine; a sparse key's lookup 180–200 ns (`lookup_ns` 250); an owner draw
20–23 ns, its zone's cumulative built once from the walk and shared by its draws (`[fin.units] draw_ns` 38).

**Extension points**: each declaring step above declares its instance and adopts it in place of the indexes rebuilt
per call today (`stalls` ×5 a day, `Standing::new` ×2, `post`'s map of every vacancy, `keep_vacancies`, `searching` in
`core_labour.rs`, the scan of every firm for a struck tile in `core_weather.rs`); a ring's optional subject index is a
lazy instance (S1.167); the declared instances maintained on writes (S1.196); the audit's sweep compacts (S1.171).

#### K-07 Epoch flags and change sets

Every store can say what the day changed — which rows, which parties — with no clearing pass.

**Layout** (`epoch.rs`): `EpochBits` holds a bit a row under a day stamp a word of 64 rows, and a summary bit a word
under a day stamp a summary word of 64 words (4 096 rows): 1.53 bits a row with its stamps. `DayStamps` holds a row's
last day, 4 bytes a row, for readers that ask whether a row was touched today — a per-day cache's stamp (K-35), a
perishable capacity's day of use (K-85). A word or row no day has marked holds a stamp no run reaches, only ever
compared with today.

**API**: `mark(slot, today)`: a word whose stamp is not today is cleared and stamped, and its summary bit set, once a
word a day; then the row's bit set — one compare and one bit set once the word is today's. `chunks_mut` hands each
writer its 4 096-row chunks (their words and summary word together), so the applies partitioned by range (K-12) mark
with no atomics. `for_each_marked_word(today, …)` yields each word marked today, its first row and its bits, in slot
order, visiting only the words their summaries mark: a reader handles a word's rows in its own loop.
`for_each_marked` yields rows; `is_marked`; `DayStamps::stamp`, `is_today`.

**Algorithms and bounds**: marking is O(1); a day's iteration is O(summary words + words marked), a payday's dense
day O(rows / 64), always in slot order and the same for any workers. Two marks of a row a day are one.

**Change sets**: each base declares which of its columns the daily audit recounts and which parties' identities it
rechecks, and marks them where it writes; the audit (K-103) iterates the day's set and reads the rows, never a
writer's maintained aggregate (N1, R4).

**Save and load**: not saved; a load starts a clean day, the save being taken at a close after the audit read the day's
set. Stamps that are state (a deposit's last change) are their owning base's saved columns.

**Volumes and ratchets** (`tools/bench.sh -F epoch`): the design point's 8.8 M party slots, each day's touched parties
(`[day.*] touched`) marked three times by three applies through their chunks in slot order, then the day's set read a
word at a time: 2.8–2.9 ns a mark (`[fin.epoch] mark_ns` 3.7; the step's 0.8 counts a mark as one bit set into a word
already in the writer's cache, where the kernel alone also checks its bounds and its word's day), 0.18–0.22 ns a set
row over whole words (`iter_ns` 0.4).

**Extension points**: segment loads stamped by day (S1.188); epoch stamps and the day's touched parties (S1.197,
S1.219); units touched today (S1.273); a capacity's use today (S1.324); the parties, owned parties and estates the day
touched (S1.330, S1.331, S1.342); what the day changed, for the audit (S1.353); holdings' change set (S4.172).

#### K-08 Horizon rings

Every kept history — marks and volumes, events, filed statements, weather, credit records, volatility windows, life
records — is an append-only dated ring with its own declared horizon, pruned by whole chunks, never by a pass over rows:
its rows a year are its rows a day times its horizon, and nothing it holds outlives the horizon (SET.13).

**Layout** (`ring.rs`): `HorizonRing<T: Pod>` keeps each row's day (u32) and payload in chunks of a declared row count,
reserved once for at most a declared number of chunks and committed as chunks are first made. A chunk holds consecutive
days' rows; a day's rows begin a new chunk unless they fit the last one's room, and only a day of more rows than a chunk
spans chunks. The live chunks' headers (24 B: first and last day, first row's ordinal, where the chunk lies, its rows)
form a ring of headers in day order; pruned chunks go on a free list and are taken again before any new one is made, so
a ring in its steady state never commits more. Rows are addressed by a monotone ordinal (u64), the live ones
`[base, next)`.

**API**: `append(day, rows)` appends a day's rows in the order the caller's stage fixes and returns the first one's
ordinal; a day before the last appended stops the run (TIME.9, TIME.10). `row(ordinal)` reads a live row and its day,
none once pruned. `range((from, to), each)` hands the reader each chunk's run of rows within the days — its first
ordinal, its days and its rows — so the reader's own loop is the only one over rows. `prune(today)` drops every leading
chunk whose last day lies before today less the horizon and returns the rows dropped. `set_floor(day, days)` records
the POLICY floor its owner reads from the schedule in force (BNK.21, HH.21, DRV.10, STA.5): the horizon on a day,
`horizon_on`, is the larger of the ring's RESOLUTION horizon and the floor in force; a floor that rises stops pruning
from its day, and rows already pruned stay pruned. Assembly refuses a RESOLUTION horizon below a floor in force at the
opening.

**Subject index**: where a kind declares one, its owner keeps a lazy keyed index (K-06) from a subject — a party, an
instrument, a series — to row ordinals: `index_since(from, index, subject_of)` enters the rows appended from an
ordinal on; `of_subject` walks a subject's live rows in ordinal order, an entry below the base dropped by the walk as
dead by construction; `compact_subject` rewrites a key whose walk drops as many entries as it keeps. A row naming a
party keeps its `PartyRef`; a reader resolves it through the directory's tombstones (S1.194), kept at least the longest
horizon of any ring that can name a party (SET.13, PTY.10).

**Algorithms and bounds**: append is O(rows) copies into a chunk pruned long ago, so its cost is its row's width in
cold memory; `range` is O(log chunks + rows read) — binary search on the headers' last days, then on the first and last
chunks' days; `prune` is O(chunks dropped), no row visited; `row` is O(log chunks).

**Traversal**: appends at their stage, in the stage's chunk order (E7), from the day buffers the stage's writers staged
per chunk (K-05); prune at 10d over the headers; ranges when read. Traversals over a ring's rows run in `phx-exec`,
which counts their rows visited into `ExecCounters::visited`.

**Save and load** (`ring_save.rs`, off the day's paths): the live chunks and their headers are saved in order
(SET.12: the histories are state), with the ordinals, the horizon and the floors; a load lays them again from the
ring's first chunk, the pruned chunks not kept, and refuses chunks whose ordinals do not run from the base to the next.
The subject index is left out and rebuilt from the live rows in ordinal order (`#[saved(skip, rebuild = …)]`,
`rebuild_equals_incremental`). Two rings are equal when their horizons, floors, ordinals and live rows are, wherever
their chunks lie.

**Growth** (E9): each ring reports its rows live and ever through `StoreStats` each simulated year; a ring whose rows
exceed its rows a day times its horizon by more than a quarter is a finding for the run's report.

**Volumes and ratchets** (`tools/bench.sh -F records`): the design point's 584 000 events at the 365-day horizon
(24 B rows) and 736 000 filed statements at two years (48 B rows before S1.216 packs them), each ring run past its
horizon so its appends reuse chunks pruned long ago; each operation is read once over a month of the rings' days, a
day's appends being a few microseconds, below what one reading of the clock tells. Growth reads 1.00 of rows a day
times horizon. An append is a copy into cold memory and costs what a raw copy of the same bytes does on this machine:
4.2–5.0 ns a 28-byte event row (`[fin.records] append_ns` 6.2; the step's 3.8 lies below the copy's floor here),
7.0–8.9 a 52-byte statement row (`append_statements_ns` 11). A range over the month just appended reads rows the
appends have left beyond the cache, 1.7–2.3 ns a row (`[fin.ring] range_ns` 2.8); the same range again reads 0.26, and
a day's rows read right after their append, as the close reads the day's events, about 0.5. A prune 0.05–0.08 ns a row
aged out (`prune_ns` 0.1).

**Extension points**: the weather's history (S1.192); the event log's public ring, a chunk a day (S1.213); the series'
dated chunks (S1.215); filed accounts' blocks (S1.216); the day ledger (S1.219); marks by instrument and by good
(S1.326); yearly means (S1.336); volatility windows (S1.340); the audit's findings kept for their horizon (S1.353); the
recorder (S1.358); a risk factor's daily changes within its lookback horizon (S4.108). Each replaces, in its base's
migration, the history it keeps today: `Core::deaths`, `Taxes::sample`, `Outlooks::responses`, `Weather::shocks`,
`CoreStats::published`, `Series::annual.insert(0, …)`, and the fixed 2²⁴-row `EventStore`.

#### K-09 The interner

Every value many rows share — a contract's terms shape, a way set, a market or route key, an instrument family's terms
— is stored once and named by a 32-bit id found from the value's bytes; each id counts the rows that hold it and
retires at zero.

**Layout** (`intern.rs`): values lie end to end in a byte arena; each id has a 16-byte row (its value's offset and
length, its holders, its slot's generation and its state). The index is open addressing with linear probing over
8-byte cells (a 32-bit tag of the value's hash and the slot), at most two-thirds full: 12 bytes an id. A tag match is
confirmed by comparing the value's bytes, so the row keeps no hash, and a retirement or the load's rebuild hashes the
value again. The hash is `Sip128` under the interner's fixed key (PC-13), so the index depends on the values alone; it is
probed, never iterated, and its order is never read (CHN.6). A cell's home is the hash's high bits scaled to the cells,
so the cells need not be a power of two. Its capacity is declared at construction; nothing resizes.

**Ids**: `InternId` is the slot in 24 bits and its generation in 8. Each interner declares its reuse as data:
`Reuse::AfterClose` (terms shapes, market and route keys) hands a retired slot out again after the close, first in
first out behind its generation (K-02); `Reuse::Never` (way sets, instrument families' term sets, which records name)
keeps a retired id's row, marked retired, and releases only its value's bytes. A slot is never reused within the day
its id retired; ids past 2²⁴, or a generation past its byte, stop the run.

**API**: `intern(bytes)` finds a value and holds it, or stores it and gives it the slot free longest. `find(bytes)`
answers without holding. `get(id)` reads a value in one row read while the id is held or listed today. `hold` and
`release` move the count; a release to zero lists the id to retire at the close unless it is held again first.
`close_day` retires the listed ids still held by none: their cells tombstoned, their bytes dead, their slots freed or
their rows kept retired. It compacts the values in slot order once their dead bytes pass an eighth. A day's
tombstones are cleared by laying the index again from the rows before they would carry it past two-thirds full.

**Grouped lookups**: a random hit costs three dependent reads of memory (its cell, its row, its value) and a hash.
`find_many` and `intern_many` take a stage's values end to end, a group of 16 at a time. Each phase (hashes, first
cells, the rows they name, the values compared) is read across the group, so the group's reads overlap. A first cell
that is not the value's is walked in full. `intern_many` applies each group's holds while its rows are near, and stores
each value not found in order, reusing its hash, so a value staged twice has one id.

**Parallel**: an apply partitioned by owner range finds its range's values read-only (`find_many`) and stages the new
ones in a day buffer. The stage's end interns every range's values in (range, emission) order, so ids depend only on
the order of the day's work (E7; `ids_same_for_any_workers`).

**Save and load** (`intern_save.rs`, off the day's paths): rows, values, slots, the ids listed today and the counts
are saved. The index is left out and laid again at the load by hashing every held value in slot order; the rebuild
pass records it as `Interner.index` (`rebuild_index_finds_every_value`).

**Volumes and ratchets** (`tools/bench.sh -F terms`): the design point's 640 000 terms shapes of 8–16 bytes, each held
by one to four contracts; each day's interns (`[day.*] interns`), one in sixteen new and the rest drawn uniformly over
every shape held, staged and interned a group at a time; as many holds released; then the close. Measured:
- An intern reads 183–226 ns (`[fin.terms] intern_ns` 250). Sip128 is 36 ns a value alone, and a uniformly drawn shape
  is read from memory: grouping cut it from 540 ns, but the step's 46 holds only for hits on cached shapes.
- A get reads 13–31 ns, a release 21–41, and the close 26–34 ns a release.
- An id holds 50.8 bytes (`bytes_per_id` 51): the row 16, the index 12 at the capacity of the shapes and every
  measured day's new ones, a value 12, its slot's and list's words the rest.
- Ledger line 10 holds S1.263's terms at 16-byte rows and 12 bytes of index an id.

**Extension points**: instrument families' and classes' keys (S1.183); route keys (S1.187); a facility's terms id
(S1.241); the contract store's terms words (S1.257, S1.259); terms shapes (S1.263); ids retired without reuse (S1.279);
ids issued at runtime with counts and a flags byte (S6.102); interned sets with counts (S6.103). It replaces, in those
bases' migrations, `sys-tec`'s `WaySets.index: BTreeMap` and the families' per-day new schedules.

#### K-10 The save contract and the world hash

The save contract (S1.162) and the world hash as a tree of frame hashes (S1.163).

**What is primary, what is derived**: a save holds only primary state — columns, slot allocators (live bits, free
rings, released lists, generations), arenas' raw words with their dead counts, rings' live chunks, policy schedules,
the cursors of rolling sweeps. Everything derived — keyed indexes, sum-trees, the links an owner declares derived, the
wheel's buckets and the agenda, the interner's indexes, per-day caches — is left out as
`#[saved(skip, rebuild = path)]`, and day buffers are empty at every close. A derived index saved, a skipped field
restored by `Default`, and a rebuild left lazy to its first use are refused.

**The rebuild pass** (`rebuild.rs`): after every store is read, `phx_store::rebuild` runs `Saved::rebuild_derived`
over the saved tree: store by store in the tree's declaration order, each field's rebuild in field order, so no
rebuild's order depends on registration. The derive writes it: a saved field passes the pass on to its own fields
(and `Vec`, `Option`, `Missing`, tuples and maps to their elements), and a skipped field calls its named path, which
reads only primary columns and the indexes rebuilt before it; one that reads another's result declares it,
`after = field`, and the derive refuses an `after` naming no earlier field (`rebuild_order_is_declared`). A rebuild
returns its rows, or, where it can fail, its rows or why (`RebuildRows`); a failure stops the load naming the store and
field (`RebuildError`, `failed_rebuild_names_store`). The pass reports each index's rows (`Rebuilt`, a `StoreStats`).
A large rebuild cuts its rows into slot ranges and joins them in range order (`by_ranges`), the same for any workers
(`rebuild_same_for_any_workers`), until the chunk plans take it (S1.169).

**The round-trip harness** (`roundtrip.rs`): `roundtrip(&store)` saves a store, reads it back, runs the pass, and
holds the copy's logical hash to the store's and the copy to the store: each index the day kept is equal to the one
rebuilt from scratch (`roundtrip_heavy_fixture`). Every base that leaves an index out calls it in its tests.

**Volumes and ratchets**: `tools/bench.sh -F save` round-trips the design point's 8.8 M party slots and times the pass:
nothing derived stands in them yet, and the pass takes under a microsecond; `[fin.save] rebuild_ms` 190 holds the
whole rebuild at a load — the lists, the wheel, the agenda and the terms index — as their bases add their stores to
the driver (S1.165, S1.225, S1.227, S1.263).

**The world hash** (`hash.rs`): the save's stream of encoded bytes — its per-field transforms and bit-packing
unchanged — is cut into the 1 MiB frames its zstd frames already are, and each frame's SipHash-2-4 under `HASH_KEY`
(`frame_hash`) is taken by the worker that compresses it (`save::seal_frame`, a wave of sixteen frames at a time on
the pool). The frame hashes are combined in a left-balanced binary tree (`frame_root`): the left subtree holds the
largest power of two of leaves below their count, and an inner node hashes its two children's roots, so the root
depends on the frames' bytes and order alone, never on the workers that hashed them (`frame_tree_same_for_any_workers`,
`odd_frame_count_tree`). Only what the save holds enters it, so a derived index never does
(`hash_covers_primary_only`). A load hashes the decompressed stream in the same frames as it reads it (`FrameHasher`,
`Reader::hash_frames`) and holds the root to the manifest's (`root_equal_after_load`); a damaged frame is refused by its
decoder, or reads back to another root (`damaged_frame_refused`). `LogicalHasher` stays for logic-level tests.

**Volumes and ratchets** (`tools/bench.sh -F save`, the hash part): the design point's party rows of primary words,
encoded a core at 760–810 MB/s (`[fin.save] encode_mb_s_core` 400, only rising), and as many 1 MiB frames of their
encoding as the ledger's saved lines hold (3 459 MB: every line but the process, the day buffers, slack and the save
buffers, an upper bound on the encoded bytes) hashed on the pool and combined in 480–570 ms on this machine's four
workers (`hash_ms` 650, the step's bound on three phone cores).

### 7.2 phx-exec

Status: planned (S1.169–S1.171)

#### K-11 Chunk plans

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.169).

**Today** (`pool.rs`, `traverse.rs`, `gather.rs`, `tree.rs`, `radix.rs`): the pool and the traversals of §6.3.

#### K-12 Partitioned apply

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.170).

#### K-13 Keyed reductions

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.360).

**Today** (`keyed.rs`): `KeyedReduce` (§4.8).

#### K-14 Declared sweeps and rolling cursors

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.171).

#### K-15 Measurement counters

The counters measure the run and never reach the world: nothing the world decides reads them. A span of the trace
(§14.7) reads the process at its two ends (`trace::Reading`) and marks its end with what it spent (`trace::Spent`):
its items where it has them (`span_items`), every thread's CPU (`CLOCK_PROCESS_CPUTIME_ID`) and page faults
(`getrusage(RUSAGE_SELF)`), the pools' chunks, dispatches and spin, and its allocations. CPU and faults are read for
the whole process at the span's ends, never at a chunk's: that sum is every worker's work and its spin between
dispatches, the same for any number of workers, and a chunk pays only one relaxed add to the process's chunk count
(`pool::counted_chunk`, around every item `for_each` runs). Reading each worker's own clock at each chunk's ends
(two `CLOCK_THREAD_CPUTIME_ID` and two `RUSAGE_THREAD` reads) cost about 900 ns a chunk on the build machine, since
none of them is served without a system call. A count the system does not give is missing, never zero. When nothing is
traced, a span reads nothing.

Allocations are counted by `alloc::CountingAlloc`, the system allocator with two relaxed counters (calls and bytes),
which only the `bench` feature compiles and only the bench's binary installs as its global allocator (`tools/bench.sh`
builds `phx-cli` with `--features bench`); the world's build has neither the counter nor its cost. A sub-step's
`ExecCounters` add rows and bytes touched, chunks and barriers, rows visited per kind (`visited`, called by the
traversal that walks them, since only `phx-exec` may share a counter across workers), day-buffer bytes and spin; every
add is checked, and a counter past `u64` is `capacity_exceeded!`. A store reports `StoreStats` (§7.1): its rows live
and ever and its bytes, sampled as `stats::Sample`; `stats::growth_per_year` reads a store's growth from its samples at
each simulated month's end, over whole years only, none before a year is sampled.

`probe::baseline`, run on the run's pool before anything of the world is reserved, reads the process's resident bytes
(`/proc/self/statm`: code, libraries, allocator arenas and the workers' stacks) and the rate of random reads of 8-byte
rows over a 64 MiB region (`PROBE_GATHER_BYTES`, past every cache of a phone), plain and prefetched, a
`PROBE_GATHER_READS` a worker: the miss a gather's budget is counted in (R12). `os::peak_resident_bytes` reads the
process's peak.

**Volumes and ratchets**: `tools/bench.sh -F counters` (`phx-fin::counters`) runs a day's dispatches (`[day.*]
barriers`) with every worker's chunk in each: `fin.counters.chunk_ns` (a chunk's counting, 13 ns measured, ratchet
100) and `fin.counters.span_ns` (a span's two readings and its counts: 1.2–1.8 µs measured, the process's CPU and faults
being system calls; ratchet 2 274, under half a millisecond at the run's 184 spans a day).

The run's report and the bench's summary carry the spans' totals, the store samples, the baseline and the gather rate
(§14.7).

**Extension points**: K-03's buffers report their bytes (S1.160); K-11's traversals count rows visited, chunks, barriers
and spin (S1.169); every base implements `StoreStats` at its step.

**Today** (`tally.rs`): `Tally`, an integer sum the same on any pool.

### 7.3 phx-core

Status: planned (S1.114, S1.177–S1.185)

#### Streams

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.177).

**Today** (`streams.rs`): the streams of §5.3.

#### K-19 Calendar and day facts

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.178).

**Today** (`calendar/`): the calendar of §6.1.

#### K-20 The register: handles and policy schedules

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.179).

**Today** (`register/`, `policy.rs`): the register of §5.3 and the policy values of §4.6.

#### K-21 The kind catalogue

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.180, S1.181).

**Today** (`kinds.rs`, `system.rs`, `pop.rs`): the kinds, legal forms and population items systems declare (§5.1).

#### K-22 The units registry

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.183).

**Today** (`goods.rs`, `units.rs`): `GoodIds` issues each good the declared unit its flows carry the first time
something names it, within `Denom::units`' 2^15; capital classes are declared units of the same registry (`UnitIds`
over `Held`).

#### K-23 The stage table

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.185).

**Today** (`substep.rs`): the sub-steps the day runs until the stage table replaces them (§6.1).

#### K-24 The capacity table

`phx-core::capacity` declares every store's capacity (§13.5). `consts::STORES` names each store as the design
point's `[store]` does, with its rows at the design point, a row's bytes (its base's layout, or a cache line,
`ROW_BYTES_UNLAID`, until its base lays the row out) and how it grows: with the persons, at most a twenty-fifth a year
(`GROWTH_DIVISOR`, above any country's population growth in the sources), or not at all (the map's geometry and the
products). A store's capacity is its design rows plus two years' growth (`GROWTH_YEARS`), rounded up to whole chunks
(`CHUNK_ROWS`); `table()` yields every store's, computed from the declarations and never stored apart. A constructor
reads a named constant — `AGENT_ROWS` (the persons), `KIND_ROWS` (the institutions), `EVENT_ROWS` and `ARENA_WORDS`
(the event log and its arena: today every event of a gate's run resident, the `events_unpruned` store, until K-36's
log keeps occurrences on storage within a horizon), `PERSON_ARENA_WORDS` (a chunk of households' arena, each
household's list of persons at its most, `HOUSEHOLD_WORDS`), `INSTRUMENT_ROWS` — computed at compile time from its store's own entry, so no store
stops the run at a literal ceiling. Capacities are address-space reservations committed only as rows are written:
they change no outcome, and at twice the design point with growth they stay inside `phx-store`'s `VA_BUDGET`.

The code widths: a chain link and a wheel entry carry a family code of `FAMILY_BITS` (8) and a slot of `SLOT_BITS`
(24), one 32-bit word; `family_code` gives the codes below `HOLDINGS_CODE` (255, reserved for holdings) and refuses a
family past them, and `slot_fits` a slot below 2²⁴. `WHEEL_DAYS` (128) files a quarter's dues ahead. The link's width
and the wheel's horizon are refused at compile time if either falls short. `phx-world` reserves every store and
wheel from these constants and holds no capacity of its own; `phx-fin`'s `capacities_cover_the_design_point` (S1.116)
holds the table to `[store]`.

### 7.4 phx-geo

Status: planned (S1.187–S1.193)

#### K-25 Tiles, zones and distances

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.360).

**Today** (`grid.rs`, `tile.rs`, `generate.rs`, `relief.rs`, `hydrology.rs`, `partition.rs`, `distance.rs`,
`state.rs`): the map, its zone distances, the regions' climates, the exposure columns and the deposits are compiled
once, at the opening's map phase, into `phx-geo`'s own state, shared read-only; what changes day by day (a region's
weather latents, a finite deposit's extracted and remaining quantities) is the core's and hashed with the world.

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
`path_length` between two tiles is A* on demand, bounded by the plane distance. Every region's **market zone** is its
largest zone (`GeoState::market_zones`), where its market meets and the freight network's segments end.

#### K-26 Networks and routes

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.187, S1.188).

**Today** (`network.rs`): the network is generated with the map: road and rail segments between the market zones of
every two regions of a country that share a border, over their land path, and sea lanes joining a country's parts, the
shortest first; each mode's capacity a day in tonnes. A route is the shortest path of one mode over the segments
(`Network::route`).

#### K-28 Cells

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.190).

#### K-29 Deposits

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.191).

**Today** (`phx-geo`'s `deposits.rs`, `phx-world`'s `core_deposits.rs`; GEO.6, GEO.9, GEO.12, GDS.3, GDS.4, GDS.12):
each deposit's right is a holding over its tile, given at the opening to the firm of its region making the product
that draws on its resource (`TEC.products`' `extracts`, `TEC.deposit_draw`) sited nearest it by plane distance, the
lower slot on a tie; a deposit no such firm's region holds is held by no one. The core's `Deposits` keeps, by the
deposit's place among the map's, its holder, what it has given and, if finite, what it holds and opened with. An
extractor decides whether to work what it holds on its extraction schedule (`GDS.extraction_days`, each firm's day by
its slot) and before its first making, through the decision core (`GDS.extract`, §3.5). The production rule's capacity
is at most what its deposits give at its way's draw a unit, none while it does not work them; each unit made takes the
draw, rounded up, from its deposits in their order (`Core::take_from_deposits`), a finite one never below nothing. Its
rights pass to its estate as its goods do, and an estate holding rights waits for its liquidation. The deposits family
holds each finite deposit's given and held to its opening at every close. A deposit's grade and the grade classes of
what it gives are not kept: every extracted unit is of its product's one class, until the commodities' grades and
their call (S2.09).

#### K-30 Weather and catastrophes

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.192, S1.193).

**Today** (`weather.rs`, `catastrophe.rs`, `climate.rs`, `exposure.rs`; `phx-world`'s `core_weather.rs`): a region's
climate is its tiles' classes' parameters weighted by their count. Each region moves a latent Gaussian AR(1) per
variable, persistence declared per class, and maps it through the month's declared marginal (`Marginal`): temperature
normal, rain a zero-inflated gamma, wind Weibull, sunshine a beta over the day's clear-sky index. Per country, hazard
and exposure class, the day's count of origins is one binomial over the class's land tiles at its daily chance, the
origins picked uniformly among them (Floyd's method); a footprint spreads breadth first over the eight neighbours in
tile-id order, each untried land tile joining at the hazard's spread chance for its exposure, and each struck tile
draws its severity from its exposure's distribution.

The world's day begins with `Core::weather_day`, which draws each region's weather (`phx_geo::weather::region_day`, its
latents kept in the core's `Weather`, saved with it) and each country's catastrophes
(`phx_geo::catastrophe::hazard_day`), each recorded in the core's event store with its region or its struck tiles, the
weather public by the declared rule (§4.10). The struck tiles and their severities wait for the day's goods, where
`Core::destroy`, beside spoilage, destroys each severity's share in permille of every free unit of every good a firm
sited on the tile holds, whole units, as a flow to nature (`DESTROYED`) the goods family reads and a loss of the units'
cost in the owner's accounts. Plant, dwellings and infrastructure join what a catastrophe destroys when they are held
where a catastrophe reaches them. Each catastrophe is followed for the prices' reads: its regions and every mark on its
day, and the first day after it each (product, region) mark rose above that (`Shock`), which LC-1-46 reads.

### 7.5 phx-pop

Status: planned (S1.194–S1.211)

#### K-31 The directory

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.194, S1.195).

**Today** (`phx-world`'s `core.rs`): `Core::key` finds a party's key by its identity.

#### K-32 Kind stores and windowed groups

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.196–S1.206).

**Today** (`phx-core`'s `store.rs`, `phx-pop`'s `kind.rs`): a kind (`KindStore`) keeps its `Parties`,
each party's record of `stride` words in one column, and, if it holds money, its accounts (bank, balance, pending,
held, facility) and cash lines, each a column indexed by slot, so a party begun in a released slot writes its own words
over the ended one's; a kind of money requires an account at `begin` and any other refuses one. `books` makes
settlement's `Books` from the kinds, and `deposits_of` sums what each bank owes. The population kinds are compiled from the systems' items (`kind::compile_kinds`): a kind is declared item by
item (`PopKindBuilder`: attributes, roles, person attributes, positions, and the attribute whose value is the party's
region), the declaring system the item's one writer, and each width comes from the number of values.

#### K-33 Persons

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.207, S1.208).

**Today** (`persons.rs`, `person.rs`): each household's persons, two words each — its word as its kind packs it (birth
date, role, attributes: the word is full) and its identity (`Held`) — a list per household in its chunk's arena and
found by its slot; `set`, `push`, `remove` (the rest keep their order) and `clear` keep the count held, `place_of`
finds a person by its identity, and `compact_due` closes a chunk's gaps in slot order once its dead words pass their
share. A person's identity is drawn from the same counter as the parties' (a person is a party, PTY.1): each is handed
the next as its household begins, a newborn the next after.

**Births and leaving school** (`sys-dem`, POP.5, POP.10): a household decides on its head's birthday
(`DEM.fertility_occasion`, taste `DEM.fertility_taste`) whether to try, and tries when the next child's value
`ln((1 + n*)/(1 + n)) − ln(e(n + 1)/e(n)) − ln(1 + 1/(1 + a)) + s·ε` is positive — `n*` its ideal
(`DEM.ideal_children`, drawn from its country's shares at its first decision and held as an attribute beside
`DEM.trying`), `n` its children, `e` its needs on `DEM.equivalence_scale`, `a` its youngest child's age, `ε` a standard
logistic draw; with no income outlook it does not try. While it tries, each woman of its couple conceives at
`DEM.fecundability` a cycle compounded over `DEM.cycle_days` (`DEM.conception`), and a conception is a birth at once: a
child in school, its sex by the sex ratio at birth, the household then trying no more until its next decision. A child
leaves school on the birthday its country's `DEM.school_leaving_age` falls on and becomes an adult out of the labour
force (`LeavingSchool`, a placeholder naming POP); the opening's children being those under the age of majority, those
past the leaving age leave on day one. A death marks the person gone, and a dead head's place goes to the partner,
else the eldest adult, else the eldest child (`household::succeed`); an onset of disability makes the person disabled
in place.

#### K-34 Offices

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.209, S1.210).

**Today**: the core's `Offices` (§7.16, K-100).

#### K-35 The per-day party cache

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.211).

### 7.6 phx-record

Status: planned (S1.212–S1.224, S1.358)

#### K-36 The stored log and the event log

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.212–S1.214).

**Today** (`phx-core`'s `events.rs` and `events_rule.rs`, `phx-world`'s `EventStore`): the events of §4.10 and §12.

#### K-37 The records store

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.215–S1.218).

#### K-38 The day ledger

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.219, S1.220).

#### K-39 Statistics accumulators and sample frames

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.221, S1.222).

**Today** (`phx-world`'s `core_stats.rs`): the agency's statistics (§3.5, `sys-sta`).

#### K-40 Life records

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.223).

#### K-41 Tallies and votes

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.224).

#### K-106 The recorder

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.358).

**Today** (`phx-obs`): the `Recorder` and its views (§12, §14.8).

### 7.7 phx-agenda

Status: planned (S1.225–S1.239)

#### K-42 The due wheel

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.225, S1.226).

**Today** (`phx-core`'s `wheel.rs`): `DueWheel` keeps a bucket a day over a horizon, reused round the wheel, and the
dues beyond it, read every half horizon for those come within reach. A day's dues move on together (`schedule_all`),
in slot order, so a bucket is mostly one sorted run; a take sorts only what follows that run (radix on the pool) and
merges it in, and the contracts due are read in slot order. A contract whose due moves or that closes is not taken
out; its reader skips a stale entry by the contract's own next due.

#### K-43 The decision agenda

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.227–S1.234).

#### K-44 Hazards drawn ahead

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.235, S1.236).

**Today** (`phx-pop`'s `hazard.rs`, `sys-dem`'s processes): every process on a kind is drawn **ahead** (REP.7, the
next-reaction method). For a household and a process, from a day _d_: every person's daily chance _q_ᵢ is read at
_d_ from the household's own state, and its chance of a hit on a day is _p_ = 1 − Π(1 − _q_ᵢ), constant until the
next day any person's rate may change (the process's `changes_after` for each person, the earliest of them, `c`). The
days to the next hit are one geometric draw of chance _p_ from the household's stream for the process; if the hit
falls before `c`, it is booked, and otherwise a **redraw** is booked at `c`. Both are exact, since a day's chances are
independent of the days before: the redraw starts afresh from its day. Whatever changes a household — an outcome, a
person leaving or joining, a new attribute — books every process on it afresh from the next day. _p_ is computed
without cancellation, as −expm1(Σ log1p(−_q_ᵢ)), one when any _q_ᵢ is; a chance outside [0, 1] stops the run. A
process at _p_ = 0 with no day its rate may change is not booked (`Booking::Never`).

A process's rate reads a table primitive at its declared axes; an annual probability _q_ becomes the daily
`−expm1(log1p(−q) / days)`, with the calendar year's own days, so a year of daily chances compounds to _q_ exactly.
A booking come due is followed on from its own day until one falls after today: a hit reaches its persons and the
next booking is drawn from the day after it, a redraw draws from its day, and the persons a hit reached are passed
over by the later draws of the day, since their outcomes have yet to change them. On a hit day the persons the hit
reaches are drawn, each by its own _q_ᵢ **conditioned on at least one** (`binomials_joint_at_least_one`), and the
event recorded. A hazard on a party itself (a firm's plant failing, a vehicle's accident) has no persons: its _q_ is
its own. Catastrophes are drawn at 3a (K-30).

**The processes on persons** are `sys-dem`'s `PopProcess`es. Each is bound to the register once, at assembly
(`PopProcess::bind`), and there builds the tables it reads for each country. Death reads each country's life table by
Brass's relational model: its survivorship logits are the sourced standard's plus one level, the same for both sexes.
The level is solved at binding, by bracketing and then halving, so that life expectancy at birth equals the country's
`DEM.life_expectancy` (§10.0), with the sexes weighted by the sex ratio at birth. Past the table's last age, the open
age keeps the last year's hazard. That hazard is split by health: the disabled's is `DEM.disabled_mortality` times the
able's, and the able's is the table's over 1 + P(r − 1), P being the chance of disability at that age and sex as the
opening draws it, so the two weighted make the table's. A person's chance of dying before its next birthday, read at
its exact age from its birth date and its health, is compounded over the days of that year of age; a change of health
draws its next death afresh. The onset of lasting disability spreads its yearly hazard by age and sex over the same
days, and is zero for a person already disabled. Both rates change on the person's next birthday (`changes_after`), so
ageing is read from the birth date and never drawn.

#### K-45 Messages and notices

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.237, S1.238).

#### K-46 The trigger index

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.239).

### 7.8 phx-ledger

Status: planned (S1.240–S1.256)

#### K-47 Money accounts

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.240–S1.244).

**Decided** (MON.4): a central bank's notes are its liability, held by households and estates; a household that banks
nowhere holds its persons' share of the currency in circulation a head (`CB.currency`, per cent of GDP) from the
opening, and pays and is paid in notes through the one money route, its notes its money account, the central bank
the top issuer it reaches. The central bank's treasury account is what its claim leaves after its reserves and its
notes. A banked household's cash, its liquidity choice, arrives with HH.7 (S2.05).

**Today**: the kinds' accounts and cash lines (K-32) and settlement's `Books` (K-49).

#### K-48 Flow batches

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.245–S1.247).

**Today** (`phx-core`'s `flows.rs`): every movement names payer and payee, amount, source, reason, denomination
(money of a currency or units of a declared unit, never netted together) and payment order, 24 bytes, appended to the
buffer of the chunk that makes it (`FlowBufs`). `Ranges`
cuts each kind's slots into ranges of `2^range_bits`. A buffer groups itself (`FlowBufs::group`): each chunk puts its
flows of the settled denomination in its payers' ranges' order in place, the rest last, and makes credits (payee,
amount, reason, 16 bytes) in its payees' ranges' order, chunks on the pool; within a range a chunk's flows keep an
order of its own making, which is enough, since a payer's ties fall by lot. `Grouped` is the day's view: each range's
slices across the chunks, and a place for every flow in the day's order, range after range. Nothing is copied.

#### K-49 Settlement

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.248–S1.250).

**Decided**:
- **A party with no money**: a payment whose payer or payee holds no money in its currency — no account, no notes and
  none it issues — fails at once, its cause no money (MON.12).
- **A closed bank** (§9.2): a flow whose payer's or payee's bank is closed is **pending**, a flow's third state beside
  settled and failed: it neither fails nor funds anyone, its amount held against the payer's funds and counting for
  nothing at the payee's, until it settles at the first settlement at which neither bank is closed, or fails against
  a claim on the estate (MON.5). A closed bank's reserve account passes to its estate with everything its transfer
  leaves.
- **A closed payer**: every flow a closed insurer or clearing house owes is pending the same way, and flows owed to it
  settle as before. On a many-party contract whose paying side holds a closed party among others, the holders whose
  credits are its own are drawn once for the whole resolution (REP.23), and the same holders stay paired until it
  settles; the other payers pay theirs as before.
- **Currencies** (Stage 5): each payer is tested per (party, bank, currency), and banks' nets are per (bank,
  currency). A flow in a currency its payer does not hold draws its bank's **conversion commitment**: the bank is the
  counterparty of both legs, trading from its own currency book at its posted quote, so the payment settles as a
  trade or fails whole; a capital rule's handle gates the commitment. A flow in a currency whose system does not
  settle that day is pending, as above, until a business day of both.

**Today** (`phx-core`'s `settle.rs`): settlement over a currency's `Books`: each money kind's accounts (the bank each
is held at, balance, pending commitments, what is held through closed banks, the facility, and cash lines if the kind
keeps them), the banks' kind, whose balances are their reserves, each bank's deposits, the banks closed and the
issuer.
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
- Before the nets are added, the day is measured (`Values`, SET.10), range by range on the pool: the value of the
  flows settled; what the payers' nets drew; and the closing ring, the parties whose settled payments exceed what
  they could pay alone (balance less what is held, plus the facility), with the part of their payments their
  receipts paid. The core's day publishes them with the failures by cause (`CoreDay`: `gross`, `net`, `fails`,
  `ring`, `ring_value`; `made` is the money its flows move, whatever came of them), and the observer reads them.
- The nets are added in place, each bank's deposits move with its customers' nets, and the failed and held flows
  are taken back from the cash lines. A closed day's flows are committed (`commit`): netted into pending, the banks'
  moves into theirs, posted, nothing failed.
- `Books::deposit_breaks` is the money family's identity: each bank owes what its customers hold.
- Settlement's state keeps no flow (PC-27): the outcome hands back the failed, with why, and the held.

#### K-50 Dated commitments

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.251, S1.252).

#### K-51 The levy engine

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.253).

**Today** (`levy.rs`): `levy::Withholding`, the income tax withheld from wages (§4.3).

#### K-52 Settlement tallies

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.254–S1.256).

**Decided**: a reason may declare a per-category tally (the balance of payments): each payer keeps a vector of
per-category amounts for its flows between parties in two countries, and the surviving payers' vectors are added by
keyed reduction (§4.8); a flow settled outside the day's batch feeds the same tallies. No handler tags a flow.

### 7.9 phx-contract

Status: planned (S1.257–S1.270)

#### K-53 The contract store

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.257–S1.260).

**Today** (`phx-store`'s `edges.rs`, `phx-core`'s `store.rs`): `EdgeTable` holds a family's contracts, each a row
(`Row`) of its two named parties beside the words the family reads on the day's paths (amount, balance, next due,
shape of terms), so a contract is read in one line; a family with no words keeps `Pair`s. A side that keeps its
parties' lists threads each party's contracts through `next` and `prev` columns from a head its kind keeps, so
opening, closing and moving a side (`move_end`) take the same time however many a party holds; a broken link or a head
handed in for a side that keeps none stops the run. A family's colder words are columns of its own, made by
`EdgeTable::column` at the table's capacity; `Column::put` writes an existing row or the next and refuses a gap. A
contract closed today frees its slot only after the close, so a family's capacity holds its contracts and a day's
openings; its reader marks it closed in its own words. A family (`Family`) keeps its `EdgeTable`, each listed side's
heads, one a party of its kind's capacity, and its `DueWheel` (K-42): `open` threads a contract on its sides' lists and
on the wheel at its first due and refuses a side of another kind; `close` takes it off its lists and leaves its wheel
entry for its reader to skip.

#### K-54 Side aggregates

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.261, S1.262).

#### K-55 Terms, shapes and due plans

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.263, S1.264).

**Today** (`phx-ledger`'s `algebra.rs` and `shape.rs`): the contract algebra of §4.4. Terms split into a shape, the
terms with each amount a contract fixes for itself taken out, shared by every contract of that kind of terms, and the
contract's own amounts. A shape's dues on a date are planned once (`shape_plan`), and a contract's due is its amounts
and balance through the plan (`due_by_shape`), with the algebra's roundings; a leg waiting on an event or an election
leaves the plan general, reckoned whole from `terms_of`.

#### K-56 Status and arrears

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.265, S1.266).

#### K-57 Books

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.267).

#### K-58 Participations

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.268).

#### K-59 Accruing statement contracts

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.269, S1.270).

### 7.10 phx-hold

Status: planned (S1.271–S1.295)

#### K-60 Holdings

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.271, S1.272).

**Today** (`phx-core`'s `goods.rs`): a good is a product's grade at a zone (GDS.1). `Stocks` holds each party's
holdings, a row a good (40 bytes, and a link), threaded per party from a head its kind keeps, so a row names no party:
units, their cost at average cost (ACC.6), the day they came in averaged by units, and what is committed to sales and
pledged to carriers; the rest is free.
- `receive` takes units in at a cost. `deliver` takes them from a bound (free, committed, pledged) at their share of
  the cost, rounded half to even, the last unit taking what is left; asking beyond the bound is `Short`, a fail
  (SET.3).
- `bind` moves units between bounds: a sale's cover, a pledge.
- `lose` is a loss to nature: it takes units whatever binds them, then cuts the pledges by what the bound units now
  exceed, so a cover left beyond the units fails its sale.
- A party ends holding nothing (PTY.9).
- A transformation is a flow with `NATURE` on the side a counterparty would stand: making, use, spoilage, a loss, a
  shipment's leaving and arriving. What accounts for it is its source. Nature's kind is the last of the 32
  (`phx_id::consts::NATURE_KIND`), and `Parties::new` refuses a table of it. `Stocks::apply` applies a flow of units:
  the payer delivers from a bound, and the payee receives at a price paid, a making's cost, or what the units carried
  out.
- A sale's goods leg (`GoodsLeg`) goes to the buyer, who holds it as a firm holds its inputs, or to nature with the
  buyer as its source, as a household's purchase uses it up.

#### K-61 Unit totals and nature's net

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.273, S1.275).

**Decided**: physical transformations are checked against their records (SET.9).

**Today** (`phx-core`'s `goods.rs`): the goods' identity (GDS.10): each good's units at the close are its units at the
open plus what nature gave less what it took (`nature_net`, `breaks`); flows between parties move units without
changing how many there are.

#### K-27 The place index

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.274).

**Decided**: `phx-geo` keeps **physical stock per (tile, class)** as the one writer of where units stand, and an index
listing, per (zone, class), the **holding** rows (owners) of that class there. A catastrophe at 3a draws in two
levels: the units lost are allocated across the holdings of the struck (zone, class) by one multivariate
hypergeometric draw, each holder's lost units coming from its own count; then, for each holder hit, the contracts its
lost units carry — mortgage, dwelling insurance, tenancy — are read from its own, since their terms name the same zone
and class. **Victims** of harm to third parties (Stage 4) are drawn the same way: for damage to property, a holder from
the (zone, class) index of the harm's zone, weighted by its units; for injury, a household of the zone, weighted by its
persons, whose illness `sys-dem` applies as the hit's declared reader. The same index drives the **holding levy**
(property tax, §4.3): on the law's dates it lists the holders of each taxed class in a zone, and each holder's amount
joins its payments that day, so no contract is kept per owner. A landlord's lost units reach its tenants: the
tenancies of the struck (zone, class) are drawn from their tenant side (REP.23), and each hit tenant receives a
notice to move. An insured loss opens a **claim** message to the insurer; a mortgage whose collateral is lost stays a
loan with its collateral description marked lost, and its lender reads that on its next review (REG.9, BNK.17).

#### K-62 Lots and cost flows

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.276).

#### K-63 Bounds and liens

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.277, S1.278).

**Today**: the bounds of `Stocks` (K-60).

#### K-64 Instruments

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.279, S1.282).

#### K-65 Instrument holdings and their events

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.280, S1.281, S1.283).

#### K-66 Named units

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.284).

#### K-67 Capital classes, wear and capacity

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.285, S1.286).

**Today** (`phx-core`'s `units.rs` and `wear.rs`, `phx-world`'s `core_plant.rs`): plant, dwellings and vehicles are
held by zone and class (REP.24) — kind, band of size or quality, condition — as counts in their owners' holdings in
`Stocks`, each class a declared unit (K-22), at average cost, the holding's mean day their service day. So a unit of
plant is bought, pledged, lost and moved as goods are, and the goods' identity covers capital.
- A kind's `Chain` declares its yearly rate of leaving a condition and each condition's efficiency and value;
  `issue_chain` issues a kind's conditions at a zone together.
- `wear` takes from each class, on what the holder held before any moved, the units a constant hazard takes over the
  days (`wear::leaving`), moves them to the next condition at their cost times its value over their own
  (`wear::carried`) with their service day kept, and the last condition's leave: a pair of flows with nature a move,
  the leaving alone for a unit leaving the chain.
- `capacity` is a holder's units of a kind, each at its condition's efficiency, at a zone or wherever they stand; a
  way's capacity is the least over its kinds of that over what it needs a unit (CAP.9).

At the opening each firm holds, of each kind its way needs (`TEC.capital` per unit of output a year,
`sys_cap::CapOwn::needs`), its output a year times that in efficient units, spread over the kind's
`CAP.condition_classes` by the weights of a stock grown at its country's `GEN.growth` (`rules::wear::steady_weights`),
each class's units at its value's share of the kind's product's opening price new — the steady-path convention. The
production rule's capacity is at most its plant's (`rules::capacity`: the scarcest kind's efficient units over the
way's plant a unit, a day); each making is counted where plant bound it and where it went beyond (never, by
construction). Every `CAP.review_days` each holder's plant wears along its chains (flows with nature under `WORN`),
the value lost charged to income as depreciation.

#### K-68 Standing rates and cumulative output

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.289–S1.291).

**Today** (`phx-core`'s `goods.rs` and `spoilage.rs`): `spoil` reckons a holder's spoilage over a period
(`spoilage::lost` on the holding's mean day), each a flow to nature.

#### K-69 Processes in progress

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.287, S1.288, S1.292).

**Today** (`phx-core`'s `goods.rs`, `phx-world`'s `core_plant.rs`): `Shipments` are goods on their way (FRT.3): owner,
carrier, the good leaving and the good arriving, units, day of arrival. Each is on its owner's list and in a due
wheel's bucket. `depart` pledges the units; `arrive` takes each day's in turn: the pledge is used up where it left and
made where it arrives at the cost and the day in it carried, as goods age on the way, a pair of transformation flows
whose source is the shipment; `shrink` cuts an owner's shipments of a good by what its pledges lost, latest first. A
capital good bought as investment whose product a kind is bought as (`CAP.bought_as`) is used at delivery and becomes
its buyer's `Project` at what it paid, named with its producer, an asset at cost on its balance sheet (CAP.2); after
the kind's `CAP.lead_days` it enters service as new plant at its buyer's region, carrying that cost (`BUILT`). An ended
firm's projects pass to its estate, which then waits.

### 7.11 phx-market

Status: planned (S1.296–S1.327)

#### K-70 Standing offers, price points and admission

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.296–S1.300).

#### K-71 The stall book

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.301–S1.303).

**Today** (`reach.rs`): `Reach` is the one set of counterparties a search reaches, and the one reader of the border
closure.

#### K-72 The vacancy book

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.304, S1.305).

**Today** (`phx-world`'s `core_labour.rs`): the vacancies stand in `CoreLabour`, and the searchers are an index of
(household, person) kept at events.

#### K-73 The posted-price meeting

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.306, S1.308).

**Today** (`meet.rs`, `retail.rs`): one product's stalls and buyers, round by round:
- each place weighs its sellers with units left once a round, an alias table for buyers of units and the same sellers
  by price with running sums for buyers with money, whose affordable sellers are a prefix;
- each buyer still choosing draws its seller from its own stream at the round's block;
- buyers are grouped by the stall they chose by a stable partition, each record carrying what its service reads, since
  reading it from the buyer list at random costs more than moving it;
- stalls are served a chunk a job, each its buyers in place: all of them when its units reach, otherwise one at a time
  drawn by lot from their order by who they are, so who is served does not depend on the order buyers came in; those
  served short choose again;
- the caller keeps a `Meeting` from day to day, whose outcome (sales by chunk of stalls, the unserved, the rounds) and
  working space a day's meeting writes into, so it allocates nothing once the heaviest day has sized it.

#### K-74 Sales batch and delivery

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.309–S1.311).

**Today** (`phx-world`'s `core_goods.rs`; GDS.2, Law 5): a meeting's sale covers its units at its seller (`bind` to
committed) and waits (`CoreGoods::deliveries`); after the day's settlement `deliver_sales` delivers each paid sale from
the cover at what its buyer paid — to the buyer, or to nature for a service or a purchase used as delivered — releases
each whose money failed (a service's released capacity perishing), and reads the goods' identity over them.

#### K-75 Between firms

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.307).

**Today** (`between.rs`): the sellers' offers are sorted once into price levels, each level's offers with units left
leading it; the buyers come in an order drawn by lot, each step of a buyer's, its highest limit first, taking from the
cheapest level at or below its limit, the level's offers in an order drawn by lot for the step, in quantities whole in
both parties' lots. A level sold out is passed by every later step. The sales are the posted-price meeting's `Sale`s,
money pro rata to the price's lot, and make their flows as those do.

#### K-76 The call auction

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.312, S1.313).

**Today**: the bills' uniform-price clearing (`phx-world`'s `core_bills.rs`, §3.5 `sys-sov`).

#### K-77 The network call

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.314).

#### K-78 The continuous book

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.315).

#### K-79 The dealer market

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.316).

#### K-80 The bilateral protocol

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.317, S1.318).

**Today** (`phx-world`'s `core_lending.rs`; BNK.4–BNK.6, BNK.20): `Credit` keeps each country's
`if_credit::law::Law`, each firm's filed earnings a year at the opening, and each bank's `Lender` (standard,
applications, declines, quotes, loans). `apply_for_loan` draws the banks a firm asks (`BNK.lenders_asked`), reads its
`cover`, and has each bank decline or quote by `sys_bnk::credit`; the firm chooses by its tastes (`BNK.lender_taste`).
`lend_shortfalls` lends the shortfall at the chosen quote's rate from the chosen bank, or nothing.

#### K-81 The administered form

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.319, S1.320).

**Today**: the central bank's facilities (`phx-world`'s `core_central.rs`, §3.5 `sys-cb`).

#### K-82 Search and match

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.321, S1.322).

**Today** (`hiring.rs`), a step a day:
- `Standing` groups the open vacancies by region and occupation, each group by the skill it asks, least first, so a
  seeker's reach is a prefix of its group.
- `search`: each seeker sends a round's share of a week's applications (the whole part, one more at the chance of the
  rest). It draws them one by one without replacement from its own stream, among the vacancies in reach paying above
  its reservation, with chances in proportion to wage^w: the logit over w·ln(wage) with a Gumbel taste per vacancy
  (Plackett–Luce), without a draw per vacancy. Seekers are drawn in chunks on the pool, joined in their order.
- `select`: yesterday's applications are met at a chance, grouped by vacancy in the order sent, and handed with their
  lots to the employer's rule, which offers no more than the jobs open.
- `answer`: each person answers its offers together; the best-paid it accepts is its hire, ties to the vacancy listed
  first, and every other offer's job returns.
- The hires go on to become employment contracts where the world applies them.

#### K-83 The carriage meeting

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.360).

**Today** (`carriage.rs`): `carriage::carriage` and `carriage::freight` (§3.5 `sys-frt`).

#### K-84 Rationed queues

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.323).

#### K-85 Perishable daily capacity

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.324, S1.325).

#### K-86 Prints, marks and fixings

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.326, S1.327).

### 7.12 phx-acct

Status: planned (S1.328–S1.335)

#### K-87 Account lines

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.328, S1.329).

**Today** (`phx-world`'s `core_accounts.rs`; ACC.4, ACC.10, FRM.13): `Accounts` keeps each firm's and bank's equity
account at the opening (`open_accounts`, from `net_assets`) and its `Income` by line since. `recognise` enters an event
on its line: deliveries (revenue, cost of sales, services used), spoilage and perished capacity (goods lost), a
service's inputs used (cost of sales), the day's settled flows by reason (`account_flows`: wages, severance and taxes
paid; a loan payment as interest), and the families' moves (`account_moves`: principal repaid taken back out of
interest, arrears' changes accrued, balances written off).

#### K-88 Equity and net assets

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.330, S1.331).

**Today** (`core_accounts.rs`): `audit_accounts` holds each equity account to `net_assets` — money, goods at cost and
what it is owed, less what it owes and a bank's deposits — and the revenue recognised to the money received for sales.

#### K-89 Carrying values

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.332).

**Today** (`basis.rs`): the carrying bases of §8.

#### K-90 Statements and consolidation

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.333, S1.334).

#### K-91 Valuations and curves

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.335).

### 7.13 phx-val

Status: planned (S1.336, S1.337, S1.357)

#### K-92 Public series

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.336, S1.337).

**Today** (`phx-val`'s `types.rs`, `experience.rs`; `phx-world`'s `core_outlooks.rs`; VAL.3–VAL.7, VAL.13, VAL.23): a
public series is keyed by a code and a place — each product's mark in a region for the firms (`CoreGoods::outlooks`),
each country's consumer index for the households (`CoreStats::outlooks`, its monthly change printed on its release
day). A print scores and forms, for every view — each memory type at each age class of `VAL.age_windows` and at none —
every heuristic of the menu (`phx_val::types::Types`, compiled once for firms and households; a view's place
`phx_val::types::view`): its error in the method's width enters the heuristic's performance, and each forms its outlook
of the next print. A series keeps each calendar year's prints summed and counted, and the first print of a year closes
the last into its mean (`Series::annual`, newest first). The anchor's level is the mean of the prints since the
opening, and, once two years have closed, at an age class the closed years' means weighted by its lived years
(`phx_val::experience::long_mean`, `VAL.experience_theta`, `Series::experienced`). A class's lived years are the mean age
of the household heads in it (`Outlooks::lived`); a household's class is its head's (`HH.window`), an office holder's
its own, both set at the opening and on each year's first day (`Core::refresh_windows`); an institution no person
holds has none and reads the view of no age. A party's memory type, switching type and stance are words of its record
(the firm's `MEMORY`, `SWITCHING`, `STANCE`; the household's `HH.memory`, `HH.switching`, `HH.stance`), drawn at the
opening, the first stance by taste alone; a firm reconsiders its stance at each price review, a household on each
spending occasion (`Outlooks::reconsider`, stream `FRM.stance` or `HH.stance`), each at its view. Each method's lag
behind a series' turns is counted in prints on the view of no age (`Outlooks::lags`).

#### K-105 The investor schedule

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.357).

**Today** (`schedule.rs`): the orders a party's value, the variance it sees and its risk aversion give at each tick.

### 7.14 phx-risk

Status: planned (S1.338–S1.341)

#### K-93 Books and limits

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.338, S1.339).

**Today** (`phx-world`'s `core_books.rs`, BNK.11): each dated family records its moves on its creditors' books
(`LoanMoves`: lent, principal repaid in `dues`, balances written off by `close_contract`); `book_loans` enters them at
the day's close and holds each creditor's `LoanBook` to the balances its loan contracts owe it.

#### K-94 Margin and collateral

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.340).

#### K-95 Exact linear aggregates

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.341).

### 7.15 phx-end

Status: planned (S1.342–S1.347)

#### K-96 Estates

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.342, S1.344).

**Decided**: how many estates are open follows Little's law, the rate of openings times their life: firms' about 800
a day × about 40 days, households' about 1 000 a day × about 25 days, personal insolvencies' about 70 a day × about 45
days — about 60 thousand open.

**Today**: the core's estates (§9.1).

#### K-97 The ending kernel

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.343).

**Today** (`phx-world`'s `core_default.rs`; FRM.15, FRM.11, L3, PTY.9): `Insolvency` keeps each country's grace, the
day each contract in arrears began to be (from the day's failed flows, forgotten once paid or closed), each estate's
claims and the firms ended. A firm in arrears past its grace defaults at the start of the labour round; no solvent
firm is wound down by choice until its office holder's mind weighs the line (S8.03), and `CLOSE` stays declared.
`end_firm` opens an estate with its money, passes its goods, closes every contract it is party to into claims
(`claims_of`: employees' wages owed and severance at the first rank, other creditors at the second), sends its staff to
search, zeroes its vacancies and ends the party. A household's estate holds its debts as claims (`debts_of`).

#### K-98 The waterfall

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.345, S1.346).

**Today** (`core_default.rs`): `estates_pay` pays each rank in proportion as far as the money goes (`shares`), the rest
to the heirless destination; an estate holding goods stays until they are sold.

#### K-99 Resolution

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.347).

### 7.16 phx-mind

Status: planned (S1.348–S1.352)

#### K-100 The decision core

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.348, S1.349).

**Today** (`phx-core`'s `decisions.rs`, `phx-world`'s `core_decide.rs`): every decision the world takes runs through
one call, `Core::decide`, taken each time the decision is (MND.20, spec Appendix E 50). It exists before anyone holds
an office, so persons and minds attach to it later (S3.05, Stage 8) without a decision moving.

- **Decision kinds.** A system declares each of its decisions as a decision point (§4.7) in its `points.rs`: a name,
  its input and output types and its rule, one pure function. The register declares the concern list (MND.2) and, for
  each point by name, who takes it and how (`MND.decisions`, a SHAPE primitive of MND.19): the office it is taken in,
  or `household` (its adults as one) or `person` (a person's own); the concerns it touches; and its mode, `best` or
  `satisfice` (MND.7). At assembly `DecisionKinds` is built from the register and held to the points the systems
  declared, both ways: a declared point the register does not list, a listed decision no system declares, a concern
  not on the list or an office no legal form declares is refused.
- **The decisions taken**: in a firm's line head's office its first price, what it makes, the inputs it orders, its
  attention, its price review and move, its stance, its posting (with its lay-offs), its selection among applicants
  and its offers at pay rounds; in its chief executive's, winding down (not taken until S8.03), its choice among loan
  quotes and its investment; in a bank's loan officer's, declining and quoting, and in its chief executive's, its
  standard; in a treasury's minister's, its consumption; a household's spending, stance and trying for a child; a
  person's search, answer to an offer, answer at a pay round, retirement and benefit claim. Inside a decision some
  parts are drawn rather than chosen: whom a lay-off falls on by lot, a buyer's seller in the meeting by its taste from
  its own stream, a posted wage's point by the fill history's arithmetic; each is the decision's own execution,
  counted with it.
- **Offices.** Each legal form lists the offices it decides through (`PTY.legal_forms`, PTY.16): a company its chief
  executive and the head of its line, a bank its chief executive and its loan officers, a treasury its minister, a
  central bank its governor. The core's `Offices` holds, per institution kind, each institution's founding preferences
  by its slot, drawn at its opening (MND.16), and the offices' holders, a map from (institution, office) to a person,
  empty until the processes that fill offices exist (S3.05); a person holding several offices, as an owner managing
  its own firm, is several entries naming one person.
- **Preferences** (`phx_core::decisions::Prefs`): what a decider brings to a rule before minds — its memory type, its
  switching type and its stance on the heuristics' menu (VAL.6, VAL.7), the return it requires (FRM.15) and its
  management type (FRM.5's speeds and curvature, one declared type while no source measures their spread); each
  `Missing` where the decider holds none. A firm's are its founding preferences; a household's are its record's
  outlook attributes (VAL.23); a person's are its household's until S8.01 draws each person's.
- **Resolution.** `decide(point, party, input)` binds the point's kind once per pass (`bind`, an index, no lookup by
  name on the path), then for each party names the decider: the player where it keeps the decision, else the office's
  holder, else the institution's founding preferences; a household's or a person's decision is its own. It hands the
  input builder the decider's preferences — a rule reads an institution's preferences only there — calls the rule on
  the input, and counts the decision by its kind and the decider's standing (player, holder, founding preferences,
  household, person), counts any worker adds to (`phx_exec::Tally`) that the run report's `decisions` and LC-1-52 read.
  A decision whose taker is an office the party's form does not declare stops the run (MND.20).
- **A decider's outlooks change with its decisions.** A reconsidered stance is written back to where the decider's
  preferences live: the founding preferences for an office no one holds, the household's record for a household.
- **Processes' decisions.** A decision taken inside a population process (retiring, trying for a child) reads its
  decider through the process view's `decider`, which the core supplies and counts; the rule is then called as any
  other's.
- **Search as a decision.** The hiring kernel's search hands each searcher's reach — each vacancy above its
  reservation with its pull — and its draws to a `choose` the core supplies, which takes LAB.search; the rule draws
  without replacement in proportion to the pulls (`phx_market::hiring::pick`). The kernel's selection hands `choose`
  the vacancy, so the employer's LAB.select names its decider.
- **Cost.** A decision's resolution is an index into the kinds, one into the party kind's offices, an empty-map
  check, the founding row and a count: a few nanoseconds beside the rule's own arithmetic. A point is bound by name
  once a pass, never per party.

#### K-101 Attention

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.350, S1.351).

**Today** (`phx-world`'s `core_goods.rs`; REP.38, REP.35, VAL.14): after the day's meetings every firm looks at its
sales since its review (`SOLD` less `SEEN_SOLD`) against those it expects, the surprise entering its sales' width at
its memory type's gain (`SALES_WIDTH`), and draws from `FRM.visits` whether it reviews its price today at
`rules::attention::review_chance` — its daily revenue, its markup, the variances of its own sales and of its stance's
outlook of its product's mark, each relative to its level, and a review's cost in its staff's hours. A firm with no
width yet, or expecting no sales, reviews on its production schedule. A surprise wider than
`VAL.attention_sensitivity` widths, its own or its stance's, is kept (`Outlooks::awaiting`) until the firm's next price
change, which records the days it took by the surprise's size over what was expected (`responses`).

#### K-102 Minds

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.352).

### 7.17 phx-audit

Status: planned (S1.353–S1.354)

#### K-103 The audit engine

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.353, S1.354).

**Today** (`phx-world`'s `core_audit.rs`): the audit's families on the core (`FAMILIES`: money, goods, contracts,
persons, taxes, debt, loans, accounts, revenue, deposits) run at each close (`Core::audit_close`), each holding its
invariant against a record kept apart from the state it checks (§15).

### 7.18 The day runner and saves

Status: planned (S1.186, S1.356)

#### K-23's runner

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.186).

**Today** (`phx-world`'s `world.rs`, `day.rs`, `core_day.rs`): `World` holds the calendar, register and streams, each
system's compiled state, and `core::Core`; assembly compiles the declarations, generates the map and draws the core's
opening (`core_open`, §10.3); a turn runs the core's day: chance on the persons, labour's round, goods, then dues and
settlement. The observer (`phx_obs`) reads the core's days, households and families; the live checks read the world
through `Inspector`.

#### K-104 Saves

Layout · API · algorithms and bounds · traversal · save and load · capacity · volumes and ratchets · extension points:
planned (S1.356).

**Today** (`phx-world`'s `save/`): the saves of §11.

## 8. Markets, valuation and expectations

- `phx-market` implements each form once (MKT.3–MKT.8): call auctions (with admission hooks); the continuous book
  (arrival by lot, price–time priority, closing auction); the dealer market; posted prices with
  rationing by lot; bilateral quotes as a protocol over messages across days; administered facilities.
- **The call** (`call::call`) finds its price on the tick grid among the prices that clear — something trades and every
  step strictly better than the price fits within what the other side brings at it, so those fill in full — chosen by
  the operator's tie sequence on the `MarketDecl` (planned, S1.347) (the tie rule: most volume, least imbalance, nearest the last print,
  skipped with none, then the lower price, which leaves no tie). Steps at the price share the rest pro rata by largest
  remainder with ties by lot, or by declared priority, pro rata within a priority. No overlap, no bid or no offer is a
  failure. **The book** keeps each side in priority order, better price then earlier arrival, a resting step placed by
  binary search; an incoming step trades at each resting step's price, passing over its poster's own; what rests and
  the `AtTheClose` (planned, S3.147) orders meet in the closing call. **A dealer request** trades on the best quote among the dealers
  asked, within the client's limit and the quote's size, equal quotes by lot; no dealer quoting that side is the
  dealers' failure, every quote beyond the limit the client's walking away. **The administered form** meets requests
  in their parties' order, each as far as the facility's registered quantity response grants, within its limit. **A
  linked call** drops the market nodes with no order that day, with their edges, before it meets. `posted::posted`
  meets groups of buyers, whose demand at a price it reads through the core's group demand, so `phx-market` depends on
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
- **Recording**: `phx-market` alone makes a print, from a meeting's matches (K-86). A posted meeting's buyer is a
  party, as its match set records sales per buyer. `Reach` is the one set of counterparties a search reaches, and the
  one reader of the border closure (K-71). Each print is traced to its match set and trades together what it says at
  its price; each mark comes from a print or a fixing its market published (MKT.13, MKT.14).
- Marks and fixings at 6b by each form's rule (MKT.12): an instrument's or a currency pair's fixing is made by
  `phx-market` by the pricing service's declared method (the volume-weighted mean of the day's trades, for dealer
  markets); with no trade there is none. **Benchmark reference rates** (IDX.2) are a different fact with one writer:
  each administrator's 9d handler in `sys-idx` fixes them from 8b's match sets, the only input a `BenchmarkFixing` (planned, S3.203) can
  be built from (the plan's PC-56); a day without eligible matches has none. The curve and the day's discount-factor
  tables are fitted at 6c from the day's fixings by the curve publisher, labelled valuation inputs (MKT.20), never
  prints. Valuations (MKT.20) in `phx-acct` at 9a, as `Money` rounded by the valuer's convention, by valuers whose
  methods are registered by their systems and read only prints and valuation inputs; each point is labelled traded,
  interpolated or, beyond the longest traded point, extrapolated by the valuer's own declared method. Valuers discount
  by the day's discount-factor table and never evaluate an exponential per flow.
- **The accounts** are two records kept apart (ACC.10): each
  equity account, in its party's site's country's currency, opened at the opening's equity for every party of a kind
  whose legal form has owners and moved only by the events posted from the day's book; and the party's assets less
  its liabilities, read when asked (`net_assets`) from the register and the contracts at carrying values. The period
  is the calendar month (`period_of`): a posting in a new month closes the last, each account's opening and closing
  kept with the period's income and capital tallied apart for `ACC.periods`. The close stops the run if a due or effect was recorded after the day's posting.
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
`VAL.adaptive_gain`'s distribution, its λ), each at every age class and at none; γ and κ are shared. The public
series keeps its last and previous
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
| A person's death | `sys-dem` | The person leaves the household and its contracts (§7.5); if the household ends, an estate on the core, behaviour in `sys-est` (until then the core's estates, below) |
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

An estate is a short-lived party, **one per ended party**, of the core's estate kind (§7.15). Their mean life and the number open are counted per kind.

As built there is no `sys-est`: the core runs estates (`Core::open_estate`, `Core::estates_pay`). A household no one
is left in ends into one (`Core::end_household`), taking its money and the shares it owns; a firm ends into one
(`Core::end_firm`) taking its money, goods at their cost where they lie, rights to deposits and projects, each contract
it was party to closing into a claim on the estate (`close_contracts`: an employee's wages owed and severance for its
years served at the employees' rank, what else it owes at the creditors'), its owners owning the estate in its place.
From its country's next business day the estate pays its claims by rank, each rank in proportion to what it is owed as
far as the money goes, and the rest to its owners, a share each, or where it has none to the institution of the kind
its country's inheritance law names (`DEM.heirless_to`, the treasury in every profile), since no heir is drawn before
the kinship lines exist (S2.04), to which what it owns passes too; it ends after the day's settlement once it holds nothing. The
insolvency law's order of classes and the secured claims' collateral are S2.03's.

Firms end in **default of payment** (FRM.15): each country's insolvency law names the grace
(`FRM.insolvency_grace_days`). A contract a failed flow put in arrears is remembered from the day its arrears began
(`Core::note_arrears`) and forgotten once paid or closed; a firm whose arrears outlast its country's grace ends into an
estate (`Core::end_defaulted`), its staff searching again. Restructuring, the law's other way, waits for the
creditors' decisions (S3); balance-sheet insolvency for the valuation of firms' books.

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
   depositors' new banking arrangements in place.
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
- survival is given as the standard's Brass logits, with the level solved when mortality is bound (§7.7);
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
its level's four files, the `later/` tables of the systems the world keeps and its derived values (`derived.toml`)
— which the register reads in place of the templates. Each row of `data/inventory.toml` names a
derived value's, distribution's or present value's owner and source, never a value.

### 10.0a The opening dataset

Each country group's economy of 2019, in shares of its GDP, is one derivation (`tools/data/derive_economy.py`), which
first reruns the ways and price levels (`derive_tec.py`), reads the budget shares (`derive_hh.py`) and reruns the plant
(`derive_cap.py`) and then writes the flows, stocks and shapes into `data/profiles/<level>/economy.toml`:
- **The flows**: twenty-two activities, the nineteen products with finance, real estate and public
  administration; households' own employment is no market activity. The economy is closed: what each economy used,
  from wherever it came, is made at home. The products' inputs of products are their ways' (`TEC.inputs`) at the
  group's opening prices (`GDS.opening_price` times `GDS.price_level`), so the matrix and the ways agree; what they
  and the three services use of finance, rents and public services, and taxes on products, are ICIO medians
  (`GEN.service_inputs`, `GEN.product_taxes`). Each final use's weight in GDP (`GEN.final_weights`) and what it
  spends on each activity (`GEN.final_composition`, households' products their budget shares, which `sys-hh` reads
  from it) are medians, and each activity's split of value added into compensation, other taxes on production and
  the surplus left is Table 6's (`GEN.value_added_parts`). Only these are stored: each activity's output, what its
  uses need, (I − A)⁻¹ f, its value added, its residue, split by its parts, and the final uses' levels are computed at
  assembly (`opening::economy::accounts`), read by the snapshot, the jobs' wages, the owners' income and the goods'
  opening, so no stored number repeats what the identities fix.
- **The stocks**: five sectors (households, firms, banks, the central bank, the government) holding
  currency, deposits, loans, bonds, government paper, reserves, central bank loans and equity, and real assets.
  Stored are only the currency in circulation and who holds what (`GEN.holdings`: households' part of currency,
  households' and firms' parts of deposits, firms' debt in loans, banks' part of government paper; the OECD's sector
  accounts), and the real assets measured apart from plant (`GEN.real_assets`: firms' inventories and land,
  households' dwellings and land, the government's fixed assets; OECD Table 9B). Each country's debts, deposits and
  banks' ratios are its drawn levels (the profile), its firms' plant what their ways need for the accounts' output
  at the steady path of its drawn growth, valued as the opening's plant is (`core_plant::opening_plant`), and
  `opening::sheet` closes the rest once: the central bank holds government paper for its currency and reserves;
  banks hold reserves at their ratio, equity at their capital ratio, and bonds that balance them; firms' equity is
  their assets less their debts; households hold the rest.
- A group with too few reporters of a sector table takes the developed group's figure scaled by a broad measure of
  its own (the Penn World Table's structures per GDP, the labour share), and the note says so.
- **The shapes**: distributions the opening scales to the matrices' totals — the spread of firms'
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
by production and by expenditure one, no output or value added below nothing, each holding a share of its whole and
no real asset below nothing; the ways' hours one column an activity, none below nothing, every activity the world
staffs asking some; each product's self-employed share and firms per person employed, and each occupation's women, a
share each, and a product making some firms; the taxes on products one an activity and one a final use; and each
country's closed sheet (`opening::sheet::country_sheet`): each instrument's
assets its liabilities, firms, banks and the central bank worth nothing beyond their equity. The tolerance is the
rounding of the stored places over the terms summed (Law 7).

### 10.3 Canonical drawing

**The core's opening** (`phx_world::core_open`) draws its own world, no copy of anything. Its kinds are the systems'
declarations in the core's order (`consts::kinds::KINDS`: central bank, treasury, bank, firm, estate, household,
agency), a kind's number its place. What the core reads of a kind is read from its declarations, bound at assembly and
again at load and never saved (`Core::declared`, `core_kinds`): whether it keeps an equity account (its form's
`HasOwners`), whether it holds money (its form may and issues no currency), whether it takes deposits, who owns it
(an account of a state-owned form at the issuer is the state's account), the money stock's class its deposits count in
(reserves of the deposit-taking forms, then the holder forms `if-state` publishes, all others in one), its place (a
party's country read from its site's tile through the map, its region word, its population declaration's region or
its country word, which an estate's record holds), and each country's heirless destination's kind. The kinds and
families the day routes by are bound to handles (`bound.rs`, `Core::bound`): each kind its system declares, found by
that declaration's name, and each family the core opens, bound as it is opened (`Core::add_family`) and rebuilt after
a load; whether a family's contracts are jobs is declared (`families::JOBS`), and a party's debts are its contracts in
every other family. No day path finds a kind or a family by its name. Each
country's central bank and treasury share a site drawn by CB's; its banks are as many as the Zipf law fitted to their
concentration gives (`sys_bnk::bank_weights`), each sited by BNK's draw. Its households are drawn region by region by
DEM (`sys_dem::draw_country`), and each household's adults' labour (`sys_lab::Rule::draw`), its banking
(`sys_bnk::households::Banking::draw`) and its pensioners (`sys_soc::Pensions::draw`) by their systems' pure rules,
each keyed by the household's subject; a rule is built from the register alone (`Register::handle` finds a declared
primitive's handle). The draws' attributes are written to the persons and the household, the household is begun with
its positions as its kind opens them (its income a year as the wages and pensions drawn times the year's months), and
each person takes the next identity. Accounts come from the country's balance sheet: the banks' reserves by their
shares and the treasury's deposits at the issuer, the households' deposits over those that bank by their wealth at
their bank, the households' currency over those that bank nowhere by their persons at the issuer. The pensions are
opened at once from the treasury, naming the person; the jobs and the households' loans are kept (`core_open::Drawn`)
until the firms are drawn, then dealt (`core_jobs`) and apportioned the sheet's loans to households by income
(`core_credit`). The balance sheet itself (`opening::sheet`) closes each country's sectors (the central bank's currency
and reserves and lends banks the remainder, the banks' reserves and equity at their ratios and bonds held by households
balancing them, deposits beyond lending held as more paper, firms' equity their assets less debts). A sheet the
closures cannot balance, or one that breaks an identity, is refused at assembly.

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

The elder of a couple heads it, else the mother, else the one person or the first drawn. Each household is formed with
its persons held with their roles (K-33), each person's health and each adult's education drawn by its age and sex
from the household's own streams, and each household is one party (REP.13). Education is drawn from
`DEM.education_female` and `_male`, the Wittgenstein Centre's attainment: the group's median share of each level up to
upper secondary and of post-secondary's total, which most emerging and developing economies report alone, shared among
short post-secondary, bachelor and master by the median of their parts over the economies that report them apart
(`tools/data/derive_pop.py`). The report gives each country's households by type and persons by age band against the
drawn shares.

A person's birth date follows from its age at the snapshot. Its birthday falls on a day drawn evenly over the year, and
it was born in the snapshot's year less its age if that day has passed, or a year earlier if not. Its lasting disability
is drawn at the chance the prevalence table gives its age band and sex. Below the table's first band, the chance is what
the onset hazard gives from birth.

**The households' banking and pensions.** A household any of whose adults holds an account banks with one bank, chosen
online among its country's by their shares (`phx_ledger::online`: the n-th household takes the bank furthest below its
share of n, ties by lot, so every prefix of the households is apportioned within one of exact) and held as its
attribute (`BNK.bank`); its deposit is its share of the households' deposits (the country's deposits less the firms')
by its wealth. A household any of whose adults has borrowed owes its bank a household loan, its share of the
households' debt by its income times the years it has left — drawn uniformly between `BNK.household_loan_years_min`
and `_max` until housing's mortgages draw each term — repaid monthly with its interest, each date taking the balance
over the dates that remain (`Leg::Amortising`). Social protection's draw gives each adult who has reached its sex's
pension age the state pension at its sex's coverage: a flat monthly amount — the replacement rate of the mean wage —
from the treasury, naming the person. A person who retires in the run claims it as it retires (§3.5, `sys-soc`). The
defined-benefit schemes are S4.04's, with their sources.

**Firms of one kind** (`core_firms`): the firm kind's store is begun with every country's firms, counted product by
product: the country's employed — its people from 15 at its employment rate — shared over the staffed activities by
the hours each one's output asks, times the product's density over its employed (`FRM.firms_per_employed`), and over
the regions by the persons the core's households hold there. A firm's record is its product, region, site tile,
productivity (billionths of a log point, drawn from the group's spread), its posted price of a lot and its output a
year in units. Its productivity and site each draw from their own subject under the firms' opening stream; its bank
is drawn by the banks' deposits. Every firm makes one product, drawn per product and region with the firm count;
`sys-tec` gives it its product's way in its country as `TEC.known` (`sys-tec`'s `known`).

What the output asks (`opening::asked`): each occupation's hours a year, by the activities the world staffs — each
product's units in the accounts and public administration's output (a currency unit a unit) times the way's hours a
unit (`TEC.labour`) — split into employees' and the self-employed's by each product's share
(`LAB.self_employed_shares`), public administration's all the agency's employees. The persons' occupations, the firms
by product, the owners' dealing and the agency's share of each occupation's jobs all read it, so the people drawn are
the people the output needs, up to one level a country: the hours the employed work over the hours the group's ways ask
at the country's output, which the running world's capacity reads per occupation (`labour.level`).

Labour's draw (`sys-lab`'s `jobs`) employs each adult at the country's employment rate times its sex's and ten-year age band's ratio to it (`LAB.employment_by_age`, ILOSTAT's employment-to-population ratios, derived by `tools/data/derive_pop.py`). What it does is matched to what the output asks, before any household is opened (`Rule::couple`, `by_rank`): the hours asked of each occupation, as employees' and as the self-employed's, are shared over the sexes by each sex's share of the occupation (`LAB.women_by_occupation`); each sex's adults, each counted at its chance of being employed, are ordered by the skill their education gives (`LAB.education_skill`) and fill the occupations ordered by the skill each asks (`LAB.occupation_skill`), quantile to quantile, a skill's persons spread over the work they fill by what each part asks. The most schooled fill the most skilled work; where schooling is short of what the work asks, as in the developing group, the work is learnt by doing it, and a searcher's skill is its education's or its occupation's, the greater (`core_labour`). A person's occupation, and whether it is an employee or runs its own business, is drawn by its sex's and skill's chances; a job's hours are part-time at its sex's share, its notice and severance the law's, its region its household's and its start band from the tenure shares. No wage is drawn (`core_jobs`): the self-employed are dealt to the region's firms first, in an order drawn by lot, each occupation's over the firms by the hours their output takes of it times their product's self-employed share, so a firm may have no working owner and be run by its founding preferences; then each region and occupation's jobs, public administration's share to the agency and the rest over the firms by the hours their output takes of the occupation net of their owners' (`TEC.labour`), in an order drawn by lot. A job's wage an hour is its activity's compensation in the accounts over the hours the activity's jobs work, each hour weighed by its occupation's pay (`GEN.occupation_pay`), a month's on the nearest wage point; a working owner's hours earn the self-employed's labour income — the labour share (ILO SDG 10.4.1) less the compensation — shared as they would be paid employed in their activity, held as its last point; searchers take their occupation's wage over every activity. Households' incomes, and their loans apportioned by income times years left, follow. Of the adults not employed, the unemployed search at the rate that makes their share of the labour force the country's, and one past its pension's age is retired.

**Jobs** (`core_jobs`): each job the opening drew for a household's person is read once (its wage, schedule and next
date, region and occupation) and dealt to a firm in its region: a region's jobs of an occupation are shared over its
firms by largest remainder (`deal`) on the hours their output takes of the occupation (their way's hours a unit, fewer
by their productivity, `sys_frm::rules::way::own_hours`, times their output), in an order drawn by lot under the firms'
opening stream. Each is a contract in the family `LAB.employment` from firm to household, naming the person by its
identity and paying the wage on the job's dates. Jobs no firm's way in their region takes (the armed forces') are the
state's, held by the public agencies (§3.5, `sys-soc`).

**Owners** (`core_owners`, FRM.1, FRM.23, PTY.16): an adult the labour draw finds employed but no one's employee is
self-employed. After the jobs, each region's self-employed, in an order drawn by lot under the firms' opening stream
(purpose `OWNERS_PURPOSE`), are dealt one to each of its firms, the rest of each occupation over its firms by the hours
their output takes of it (`deal`), evenly where none takes it; a firm left with none is counted. Each dealt person's
household holds a share of the firm (`Owners::of`, `holds`) and the person works in it (`Owners::working`) its
country's full-time week (`LAB.full_time_hours`) in its occupation, counted with the staff in the level, the posting
rule's hours held and the staff's capacity, and as employed in the statistics. The first working owner manages the
firm: it holds every office a decision is taken in by the firm's form (`Decisions::appoint`), with the preferences the
firm was founded with, so the firm's decisions are counted as its holder's. A working owner who dies, emigrates or
retires stops working there, its offices passing to the next owner working there or standing empty; the share stays
its household's. A household that ends passes its shares to its estate, opened even with no money; a firm that ends
makes its owners its estate's, its working owners search again (with no benefit, being paid no wage), and the estate
pays what its claims leave to its owners, a share each; an estate with no owners pays it to its country's treasury, to
which what the estate owns passes too. The owners' income while the firm runs is its payout (FRM.10, S2.03).

A firm's productivity is drawn from its group's spread (`GEN.productivity_spread`), and each product's firms' log
productivities are shifted together (`core_firms::centring`) so that their hours a unit, weighed by the output the
logit deals them at their prices, average the way's: the ways' hours are the average firm's, as the accounts measure
them, so the accounts' compensation is what the firms' staff cost at their own productivities. The shift is solved by
widening a bracket by doubling and halving it until its halves meet, no tolerance declared. **Day zero's prices and
output** (`core_firms::Snapshot`): a product's unit at the opening costs its way's inputs at the opening prices plus
its labour at the accounts' compensation over output; a firm's day-zero price is its product's in the accounts at its
productivity, posted at the nearest of its management's price points, and its markup is that price over its own unit
cost — its staff's wages and its owners' labour income over what they make, and its inputs at the opening's prices
(`core_goods`). The accounts' units a year of each product are shared over the regions its firms are in by their
persons, and within a region over its firms by the retail logit's price term (`SRV.price_weight`); the distance term
joins when the meetings' places are on the core. The sheet's firm deposits are shared over the firms by their turnover
at those prices. Each wage is placed on the nearest point of labour's grid, the points a declared ratio apart
(`LAB.wage_point_ratio`: a quarter), until firms post their own.

Nothing is balanced after drawing: finer attributes are drawn from their own counter keys, so a coarser setting is a
projection of a finer one, and every number of preference types is a discretisation of the same declared distribution
(NUM.4).

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

### 10.5 The one run

There are no settled worlds for testing: the world runs once (spec Appendix E 36). On the build machine the bench runs
it (§14.7): a step reads its fast checks and the budget, and a stage gate's run (`tools/bench.sh -g`) is settled for
the owner's length and run two years after it with the audit and every live check. Its numbers test the code and are
never read as the world's. CI never runs the world.

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
  scratch) are empty at a close and are skipped too. A derived index is left out as `#[saved(skip, rebuild = path)]`,
  and the load runs the rebuild pass (K-10) on the core once the world is rebound, a failed rebuild stopping the load
  with its store's name; `skip` alone is PC-101's to refuse, today's sites admitted until each base names its rebuild. Every name a save holds — the kinds' and the families'
  (`consts::families`) — is read back as the build's own, and a name the build does not declare is refused.
- **Every save is full** (SET.12, spec Appendix E 22): a restore reads one save. Allocator state (free slots, list
  links, released slots) is saved as it stands.
- **Retention**: the latest complete save, plus the one being written. A save is written into a directory named
  partial; once every file and the directory are synced, it takes its complete name and the root is synced, and only
  then is every other save in the root, complete or left partial by a save that never finished, removed.
- **A load** (`phx_world::load`) refuses a save of another format, build, register, seed or number of persons, each
  naming its reason (N5); it assembles the world from the build and the data as the save's was, draws no opening,
  reads the core and its day, holds the root of the frames it read to the manifest's world hash, runs the rebuild
  pass (K-10), binds the declarations again, moves the
  calendar's window to the save's year and reads the run's measures.
- **The save check** (LC-0-35): each periodic save of a gate's run is read back from its files alone, its frames
  hashed as they are read, and dropped, and their root held to the manifest's; its sizes and its write and check times are the run's (LC-0-36).
- **No copies**: a save is loaded only to continue the one run, or, on the build machine, apart by `phx inject` to be
  audited and discarded, never run on (N1).
- **Injection** (N1, `save/inject.rs`): a gate's run takes one more save, at the close of the 30th day after
  settling, into its own directory, and after its last day hands it to `phx inject` in a process of its own, so the
  run's memory stays the world's alone. Each family (`core_audit::FAMILIES`: money, goods, contracts, persons, taxes,
  debt, loans, accounts, revenue) has its discrepancy put into a fresh load of it — a unit where only that family's
  invariant reads it: money in a treasury's account no flow paid, a good's unit nothing made, a contract naming a
  household that never was, a person counted and not held, a unit of tax arisen, of bills issued, on a loan book, in
  an equity account, of revenue — and the audit then runs once over the state as it stands at the save's next day, no
  day stepped (`Core::audit_close`, which holds money and goods to what they were before the injection); the families
  that found something are recorded with the run's measures, where LC-0-10 reads them. A world not read from a save
  refuses an injection.
- **The world hash** is the root of the `core` store's frame hashes (K-10): SipHash-2-4 with a 128-bit result under a
  fixed key (`HASH_KEY`) over each 1 MiB frame of the day and the core as their save encodes them, combined in a
  left-balanced tree, each frame hashed by the worker that compresses it; a save reads back to exactly the stream it
  was written as, so its root is its close's. The save's two tasks write the `core` store and the `run` store.
- **Encoding** (`phx-store`): a column is encoded field by field, each field's transform declared with its type
  (delta modulo 2⁶⁴, zigzag, both, or plain), then bit-packed per block of 1 024 values at the block's widest value.
  Each field's stream is little-endian: magic `PXCL`, format version, field width, transform, element count, then per
  block its bit width and packed values. A decoder refuses a damaged, truncated, foreign or unknown-version input with
  its reason, never a guess.

---

## 12. Observation and the player

- **Views** are built at 10e on a turn's last day from records and the state at its close, into fixed-bin histograms,
  and swapped in behind an `Arc`; tables cross the FFI in pages; `phx-obs` writes nothing
  (Law 17). As built, the host (`phx-cli`'s run) drives the observer after each turn: its
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
- **Two builds**: the participant's reads only through `ParticipantScope` (planned, S6.131), its party's scoped read; the inspector's
  compiles only with the `inspector` feature, which the participant build refuses. As built at Stage 0 there is one
  build: no crate declares an `inspector` feature, `Inspector` and the `Recorder` are always compiled, and
  `ParticipantScope` (planned, S6.131) is not built. Every shown number is a
  `Shown<T>` (planned, S6.131), built from a record entry, a read at the close, a published statistic, or a fixed-bin aggregate of
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
- **On the phone**, the engine is to run on its own thread with the pinned pool: create, load, step a turn, read a
  view page, submit an action, save, and in the inspector build export the recorder's series (§14.8). Nothing runs on
  the phone yet: the phone's run of the world is built when the owner calls the device run, reporting as the bench's
  run does (§14.7).

---

## 13. Budgets

Every figure the budget is judged by is a key of `perf/design.toml` (S1.113), written there once; this section says
what each table means and how the budget is judged, and names the key. A figure in the plan or here that differs from
the file is a defect. The world never reads the file; `tools/bench.sh -F` (`phx-fin`, §14.7) fills each base at the
design point and measures it against its `[fin.<base>]` ratchets in `perf/budget.toml`.

### 13.1 The design point

The design point (spec Appendix E 51, plan §12) is `[point] persons` with every count per person the finished world
is expected to hold raised by `[point] per_person_margin`, so the finished world is measured with room above it; it
measures the code, not the world, and is not the play resolution (N8.5, E 36). `[store]` holds each count with the
step whose figure it is, `[store.contracts]` the contract families' rows, and `[day.b]`, `[day.nb]`, `[day.h]` and
`[day.bc]` the daily volumes of an ordinary business day, a non-business day (TIME.8), a quarter-end payday after three
closed days, and the business day after four closed days. The steps state their figures at the counts of
`[point] steps_persons`; each total is their statements × `[point] scale`, the parts that do not grow with the persons
(the process, the map and network, the save buffers, the barriers) excepted.

### 13.2 Time

Wall time is core time ÷ `[phone] cores`; a VM nanosecond is a phone nanosecond ÷ `k_compute` or `k_gather`. A turn
is the sum of its days; the lines are `fin.turn.median_ms` for the ordinary business day and `fin.turn.worst_ms` for
the binding turns, 4 NB + B′ and 3 NB + H, each with 10 % headroom below `[phone] turn_ms`; a non-business day
weighs four times on the worst turn. Four fused chains keep the day's work single-pass: a sale is its payment; a due
is streamed into both parties' `pending`; nets live in `pending`; production is realised in the (seller, good) batch.

The tables: `[unit]`, each kind of work's cost with its miss budget; `[stage]`, the per-stage lines by day type;
`[fin.fixed.<line>]`, each fixed line a table of its steps' shares; `[fin.decide]` and `[fin.decide.additions]`, the
decision units and the work added to them; `[fin.calendar.<event>]`, the dated days, each joining H
(`joins_h`), B′ (`joins_bprime`) or neither, a campaign day (`campaign`), and the events (`events`: a mass default,
a bank's run or resolution, a catastrophe). A step states its share as "its share of `fin.fixed.<line>`: x / y / z
core-ms (B / NB / H)", "adds u phone-ns to `fin.decide.<unit>_ns` at n items" or "on `fin.calendar.<event>`: x core-ms".

Nothing is counted twice: users carry shares and kernels carry units — a kernel states its unit ratchet and the count
it is measured at, each step that calls it states its share, and a kernel only the core's own day calls states the
share itself; `recount` is the whole audit (the touched parties' identities, the recounts, the rolling part);
`kinks` is stage 4a's realisation of standing rates and wear, and `real` is freight, construction, processes,
energy dispatch and weather, both every day; work that runs on non-business days states its NB share; a step's items
added to a unit are costed at that unit. The worst turn is each binding turn with every dated day that can fall on it
and a campaign day; each event is composed on it one at a time, on its worst day — its own shares plus the tails of
its cause that fall there — against the same `fin.turn.worst_ms`. The core's close (S1.360) and every gate sum the
tables so.

### 13.3 Memory

The ledger (`[ledger]`, lines 1–25) is the resident memory at the heaviest day, each line with its owner bases and
their bytes a row (`[bytes]`), against `fin.mem.peak_mb` (`[phone] memory_mb`: 4.5 GiB less 10 %, N8.4). A step states
its bytes as "ledger line n: +x MB resident", or "+0 B (inside S1.nnn)" where a base's figure holds them; each line is
the sum of its steps' statements at `[resolution]`'s settings × `[point] scale`. The lines most sensitive to the
design point are a contract row, a person and a firm.

`[resolution]` holds every RESOLUTION setting the design point is measured at: each with its `forced_by` and what it
frees or costs, and the settings the owner set (plan §12) marked as the owner's. The map's grid and its cells are
fixed (spec E 29, GEO.19).

### 13.4 Saves

Raw and on-disk bytes, save and load time, and retention (N8.10, N8.4) are the `fin.save.*` keys: two saves retained
— the latest and the one being written — with history once, within `fin.save.storage_mb`; save and load each within
`[phone] save_s` and `load_s`. The world hash is a tree of per-frame hashes (K-10), so a save hashes what it writes.

### 13.5 Capacity

Every width is sized for the design point with two years' growth: a party's slot 27 bits and its kind 5; unit ids 24
bits; a chain link and a wheel entry's family 8 bits and slot 24, code 255 for holdings; terms 24 bits with an 8-bit
generation; money `i64` with world totals in `i128`; the wheel 128 days; radix digits 11 bits. `phx-core::capacity`
(K-24, S1.114) holds the table and refuses a count past it where it arises.

### 13.6 The valve

The play resolution is set and reset by the budget's measure in the one run (N8.5, E 36). The world holds
`REP.persons` of the setup's population (`data/shared/REP.toml`, RESOLUTION), which the opening splits among the
countries, so everything derived from a country's people follows the world's size; `phx run --persons N` sets it for
that run, the population's store and the save's manifest record it, and a save loads only under its own (§11). The
valve is a declared setting, never an input the world reads from its own timing: it changes only between runs, by a
recorded change citing a bench report (`perf/bench/`) or the owner's decision (plan §12), which `phx-check`'s
no-tuning rule enforces.

It is refined in E 41's order — the persons, the preference types, each kind's attribute classes and the zones, the
horizons and snapshot intervals, the age classes, the audit's cycle, the draw scheme — and coarsened in its reverse. A
miss is met by representation and traversal first (N8.7), then by the valve; what neither closes is the owner's, put
with its numbers.

### 13.7 Risks

Each risk with the measure that tells it first: the non-business day, which weighs four times on the worst turn
(`[day.nb]` against `[stage]`); the retail chain (the sales batch's unit); memory (`[ledger]` against `-F all`); rule
costs (`[unit]`'s miss budgets); life records (their store on storage within its horizon); heavy dues (`[day.h]`);
the device's sustained speed (`[phone] cores`, until the device run measures it); 16 KiB pages (the columns' page
tails); and surprise and wake days (`[fin.calendar.events]`).

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
register, which the manifest's hash covers, changed only between runs or at a save boundary, never from the run's own timing (§13). A gate's run checks that each save, decoded store by store without building a second world, hashes
to the world hash of the close it was written at (SET.15); the check is not meant to run on the phone, and as built it
runs only in `phx run`.

### 14.4 The measurement programme

| Command | Measures |
| --- | --- |
| `tools/bench.sh` | the world run on this machine at any persons, days, seed and workers, or a gate's run (§14.7): its whole report, each day's stages timed, the budget's block against `perf/budget.toml`, and a summary |
| `phx inject` | audit independence (N1), on the build machine, on a save loaded apart, audited and discarded |
| `phx realism` *(Stage 7)* | the stylised facts (N3) read from the run: recording, statistics, verdicts, GEN.10 (§14.8) |
| `phx chains` *(Stage 7)* | the chains' relationships (N4) read from the run (§14.8) |
| `phx register-report` *(Stage 7)* | the primitive register's sources and the shares assumed and estimated (N7) |
| `phx run --report` | the run's audit, liveness, live checks, REP.15's report, GEN.8 and NUM.7 |

The representation's own numbers the budget needs — rows per kind, contracts per party, bytes per store, draws and
hits per process, flows per day, the stages' times — are read from the run's report by the bench; those the report
does not yet carry are added to it when a step needs them.

### 14.5 Gates

Every stage ends with: CI green, and a clean gate run of the gate commit (`tools/bench.sh -g`, §14.7), whose audit
and live checks are the gate's; the **device run**, when the owner calls it: the world run on the phone for a
settled simulated year, carrying the inspector's recorder (§14.8), its report read as the bench reads its own, within
the memory and time budgets; from Stage 1,
the stage's macro reads, which `phx-cli` reads from the device run's series, against real economies' relationships
(spec Appendix E 25), each miss a finding that does not block the gate. The budget is judged on that same run, so the
recorder's cost is inside it. The **go/no-go** reads the device run's report: the median turn ≤ 1 000 ms and the worst ≤
2 000 ms over the settled year, peak `VmHWM` and PSS ≤ 4.5 GB, a full save ≤ 5 s; §13's 10%
headroom is reported, and a gate passes without it only as a recorded finding. Stage 7's gate adds the realism reads
(§14.8): realism misses are findings and do not block it; the budget does (N8.8). The phone's run is rebuilt on the bench's report when the owner calls the device run; until then each gate's world
run at the committed resolution and, beside it, at the design point's persons (`tools/bench.sh -p`, §13) stands in
for it.

### 14.6 Measure first

The world's run measures itself: each day the core times its stages by the run's clock — the hazards, the rates, the
labour round and its parts, the goods day and its parts, freight, settlement, the audit, the statistics — and the
report carries them day by day beside the day's flows, settled and failed payments, the budget's block (the turns'
median and worst, bytes a person, cores busy, page faults a day) and every mechanism's own section. The bench reads
them at any size of world, so the representation is measured before it is built on: at each gate, at the committed
resolution and at the design point, and at any step whose change could move a stage's cost. From these, §13 is
rewritten with measured numbers — the measured unit costs times each later stage's counts — so each gate reports the
whole world's projection, not only its own stage's. If they do not fit, the remedies are, in order (N8.7): how the
world is represented and traversed; then the play resolution — the size and the zones. If none suffices, that is a
finding, and the owner decides; no mechanism is weakened. The phone's time is the CPU time of every thread, spinning
workers' included (`process_cpu_ns`), over its three sustained cores, never below the wall; page faults per day stand
for allocation during the day.

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test` in the debug and the release profile; `phx-check`; `miri` over `phx-store`'s tests on the pinned nightly; `public-api`, the kernel and interface crates' API snapshots; the workspace built for Android; `android-app`, the game app's APK, uploaded as an artifact. The world is not run |

Every job runs on push and pull request on a pinned runner image with a 30-minute timeout, reads its tools' versions
from `tools/versions.toml`, caches on `Cargo.lock` and the toolchains, and may not fail. CODEOWNERS gives the owner
`/perf/`, `docs/PROJECT_PHOENIX.md` and `data/profiles/`.

**The bench** (`tools/bench.sh`, on the build machine — the development VM, 4 cores and 15 GB — the one benchmarking tool) builds `--release` (thin LTO, §17) with the `bench` feature, which counts every allocation (K-15), and runs the world: `-p` the persons (the committed resolution by
default, where the budget is judged), `-d` the days from day zero (twenty, the span the budget's ratchets are measured
over), `-s` the seed, `-w` the workers, `-c` the live checks; `-g` a stage gate's run instead — settled, two years,
the audit and every live check; `-k` keeps the report in `perf/bench/`; `-t` stops the world after so many seconds; `-P` samples the run (the kernel tools' `perf`) and ranks every function by
the time spent in it.
**The trace** (`phx_exec::trace`) tells the run as it happens: every span of the opening and of each day — its stages
and sub-stages, settlement's passes, each product's meeting — as it begins and ends with its own time and what else it
spent (K-15: every thread's CPU and faults, chunks, barriers, spin and allocations), and notes of what each did (parties, dues, flows, buyers, stalls, rounds, sales, failures); a loop notes how far it is at each power
of two of its rounds, so one that runs away shows. The host sets where its marks go (`phx run` prints each with the
time since the start and the memory held); with none set it tells nothing, and no outcome reads it. Ctrl-C or `-t`
stop the world, never the bench, whose summary is then read from the log. A span's line ends with `items=` where the
span has items, `cpu_ms=` (every thread's), `faults=` and, in the bench's build, `allocs=`; the printer gathers every
span's totals by name. It writes the run's whole report (each day's stages in microseconds under
`core_days[].stages_us`; `spans`, each name's `count`, `wall_ms`, `cpu_ms`, `items`, `ns_per_item` — CPU over items,
none for a span without items — `faults`, `allocs`, `alloc_bytes`, `chunks`, `barriers` and `spun`, a sum missing
where any of its parts was; `stores`, every kind's store sampled at the opening and at each simulated month's end —
`rows_live`, `rows_ever`, `bytes` — with each store's `rows_live_growth_per_year` over whole years only;
`baseline_mb`, the process's resident memory before the opening; `gather_rate`, a random row read's ns and core-ns,
plain and prefetched, over its region), the run's log and a summary: the run, the budget's block against
`perf/budget.toml`'s ratchets — which may only fall (the cores only rise), a step that must raise one saying so and
why — the spans still open where the run did not finish, the baseline and the gather rate, every span's count, wall,
worst, CPU, items, ns an item, faults and allocations, heaviest CPU first, the heaviest meetings, settlement's and the day's notes, the parties opened, memory, cores, page faults, the flows and the findings.
Its numbers are the code's, never the world's. A gate's run is **clean** when the audit reports nothing, every failing
live check is recorded where it is settled, and no ratchet moves the wrong way. A step is done on its fast checks, the budget read by the bench, and the world's long
run is at the gates; a stage gate's settled two-year run is also the device run's, on the phone.

**The finished-volume measure** (`tools/bench.sh -F <bases|all>`, `phx fin`, the crate `phx-fin`) measures the code at
the design point, never a world (N8.8). Each base's driver (`FinBase`, `src/<base>.rs`, one registry line) fills its
base from `perf/design.toml` with draws of its own stream `fin.<base>` at the design point's seed, then runs it through
its own kernels a day of each type — `-D` picks among B, NB, H and BC, `turn` every one; BC's counts are B's with
`[day.bc]` written over — the first day of each type warming the day buffers and never counted, each operation read
on the application's clock through `Measures::read`. Per base and operation it reports items, CPU summed over threads, wall, ns
an item (CPU over items, the same for any number of workers), faults and allocations. It composes the phone's lines —
a line's core-ms its VM ns × the phone's factor × its day's count; a line no driver measures yet the steps' declared
`[stage]` figure, reported as declared — each day type's sum, the business day after closed days with its declared
extra, and the binding turns (4 NB + B′ and 3 NB + H over `[phone] cores`). It fails on a store of
`phx_core::capacity` short of its `[store]` count and on any `fin.<base>.<measure>` ratchet of `perf/budget.toml` a
measure misses; `-k` keeps the report as `perf/bench/<commit>-fin-<bases>.json`. `phx-fin` reads the foundation and the
kernel only (PC-01).

**The kept baseline** (`-F kept`, `phx-fin::kept`) measures today's kernels that the core keeps or replaces, at the
design point, on the machine's pool: flows grouped and settled over `[store] accounts` at `[store] banks`
(`flow`, `flow_h` on the heaviest day), the due wheel over `wheel_rows` and `wheel_far` taking a day's `dues` and
filing each again (`due`), the posted-price meeting over `stalls` by product and zone meeting a day's `retail` wants
(`purchase`), hiring's search over `vacancies` by a day's `searches` (`search`), the calendar's civil and business-day
reads (`civil`) and the streams' draws (`draw`); a kernel runs only on a day whose counts name its work, so a closed
day settles and takes nothing. Each fill draws from its own `fin.kept.*` stream, the same for any workers; what a
day's work is given is made before it is timed. The report gives each base's rows and MiB filled, read against
`fin.<base>.mb`. Until their bases land, the composite day puts each measure in its declared stage line in place of the
unit cost it stands for (`kept::LEAVES`: settlement's flows in 7 Settle, the dues in 1 Open, the meeting in 6 Prices,
the search in 5 Decide), a line so changed reported as `Partly`; so `-F all` reads today's real distance to the
budget. The `[fin.kept]` ratchets stand at the measures of 2026-09-30 with the budget file's margins, each named for
the step whose base retires it (`kept::RETIRERS`).

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

- **The audit** (N1) holds the state at each close against **independent records** kept apart from it: each family
  one invariant with its owner and clause, reading what the day did and a rolling slice of the rest. It never
  repairs. A finding carries its family, clause, owner (a party, tile, contract, instrument, market, country, event or
  the run), size in money, units, days or a count, and day. Findings, metrics and run records are kept outside the
  world: not hashed, and nothing the world decides reads them (Law 17). Injection lights each family alone
  (`phx inject`, §11). The families on the core are §7.17's.
- **Live checks** and the save check as above.

---

## 16. Drift guards

`phx-check` and clippy fail the build on:

1. **Layering** (§3.1), the external allow-list, rayon only in `phx-exec`, `unsafe` only in `phx-store` and `phx-exec`
   and in `#[derive(Pod)]`'s expansion.
2. **Type-aware rules** (clippy disallowed lists over world crates): `min`, `max`, `clamp` on numbers; `Instant::now`,
   `SystemTime::now`; `RandomState`, std `HashMap`/`HashSet`; atomics, `Mutex`, `RwLock`, `OnceLock`, `LazyLock`,
   `thread_local!`; `println!`. Declared real limits go through `DeclaredLimit::bind` (Law 6), built from the
   register, a contract's terms or a physical token (holdings and stock, for capacities). The count of
   `allow(clippy::disallowed_*)` in world crates only falls.
3. **Structural rules** (`phx-check`): no `static` items in world crates; no heap-owning types in stores; numeric
   literals only 0, 1, −1 and 2 in mechanisms, engineering constants in one `consts` item per crate; no clause
   identifiers in comments; interface crates without behaviour; no forecast by running the world (PC-33): `phx-val`
   depends on nothing that holds the world, and no function of it takes the world, a table or a handler's context; no
   full sweep in a sub-step not declared as one (*not built*: no rule refuses an undeclared full sweep yet).
4. **Types that refuse**: `Money`, `Qty`, `Missing` without `Default` or clamping; kind identifiers without equality
   outside the kernel; private-constructed handles; zero-sized systems.
5. **The clause map**: every spec clause is assigned to a step in `IMPLEMENTATION.md`, and `phx-check clauses` refuses a
   live clause the map omits, one two rows complete, and a mapped clause that is retired or unknown. The transmission
   chains (Part L's headings, `L1`–`L12`) and the measurement items (Part N's, `N1`–`N8`, a heading retired when its
   first line opens `_Retired_`) are clauses like a system's: a map row's system is `L` or `N`, its numbers may be a
   range (`1–12`, refused when it runs backwards), a step completes one by listing it bare (`N8`; `N8.8` is its text),
   and a `#[clause(..)]` naming an item or a sub-item (`N8.2`) carries it. For a step marked
   done, each clause it completes must have a **carrier** in the code or data: a `#[clause(..)]` attribute, a
   declaration's `clause` field, a data file's `clause` key, or a contract's `violation!(clause = ..)`
   (`crates/apps/phx-check/src/clauses.rs`). The carrier's shape — STATE → store, fact or type; DECISION → decision
   point; PROCESS → handler; INVARIANT → audit family; MEASURE → metric; FORBID → a check, clippy rule or type-level
   refusal; PRIMITIVE → register entries — is not checked: `#[clause]` checks only that each name is a clause
   identifier, and there is no `phx dump-registry`.
6. **Documents**: the coverage table (§19) regenerated and matching; every step with all its sections and a status;
   this document's crate lists matching the workspace; no document naming a working paper of the plan's writing or,
   in the plan, a writer's name for a range of steps (PC-09's `SCRATCH` and `WRITERS`, whole words); every clause the
   plan cites as completed at a step completed there by the clause map; every step cited as retiring a placeholder
   register id naming it in its own text; and a base, kernel or index step's Extension points naming exactly the later
   steps whose Depends on names it (the reverse index), each after it; and every standing step's Clauses line in one
   form — `none`, or items split at `;`, each `SYS.n TYPE` with the type the spec's bullet declares, `Law n *(part)*`
   or a bare chain or measurement item, each with at most one `*(part …)*`; and every edge case of a standing step
   naming its evidence — a test its unit tests list (all it names, where the unit tests say they hold the edge cases'
   tests), a test of another step it names that lists it, a test the workspace holds, a live check a step lists or the
   code runs, a `-F` case its budget names — or `n/a:` with the reason; and every standing step that changes code
   done on the fast checks and the bench's read by its Kind — a base, kernel or index its `tools/bench.sh -F`
   measure, a gate the `-g` run (its own or §2.24's), any other the bench's run — a `docs` step exempt. The design
   point's figure set (`perf/design.toml`) is held by PC-09 too: its tables standing, each fixed line its shares ×
   scale (the barriers unscaled), each calendar list its items and its core-ms its sum × scale, the stage day its
   stages, the ledger's total its lines, and every `[table] key` and `fin.fixed.<line>` the plan cites present.
7. **Process**: live-check identifiers never disappear; a primitive's value in `data/` changes only with its `source`
   in the same diff; the placeholder count only falls except by placeholders a stage introduces, and no system is done
   while a placeholder naming it remains; public-API snapshots of kernel and interface crates change only with the
   change that needs them; `perf/` changes need the owner's review (CODEOWNERS), except the bench's kept reports
   (§14.7).
8. **Ratchets** on deterministic counters (§14.7): in the world's run, bytes per
   store, per row and at peak; rows and bytes touched per sub-step; agenda rows, draws, occasions, agents changed and
   new agents per day; legs per batch; barriers per day; agents per kind; rows per agent by line kind.
   Declared-but-never-read primitives, streams and hazards are reported. `perf/ratchets.toml` holds `phx-check`'s
   counts of `allow` and `expect` attributes and of placeholders and each rule's admitted exceptions
   (`phx_check.exceptions_pcNN`); the world's run ratchets the budget's block (`perf/budget.toml`, §14.7): the turn's
   median and worst, bytes a person, faults a day and cores busy, and the core's rules — `unit_cost_per_firm_day`
   (unit costs reckoned, counted on the core in a field never saved, over firms × days; the rule's value 1, R7),
   `meeting_work_per_sale` (the sellers' weights the meeting reckons, over its sales; R11), `allocs_per_day` (the
   bench's allocator from the tenth day on; the rule's 0, R5), `spans_below_busy` (spans of 64 k items or more on
   fewer than 2.5 busy cores; R3), `barriers_per_day` and `spin_rounds_per_day` (the pools' dispatches and spin, R8),
   `baseline_mb`. A `phx_budget.` ratchet the run did not produce is refused, never read as met; rows visited a day
   join with K-14's sweep ledger (S1.171). The other counters are not built.
9. **The design point's ratchets**, `perf/budget.toml`'s `[fin]` section, seeded from `perf/design.toml` by
   `phx fin --seed-budget` (`phx-fin::seed`) so the two files cannot disagree, between its marker lines and edited
   only by it: `fin.day.<b|nb|h|bc>_core_ms` from the day's stage line and `[day.bc]`'s extra; `fin.turn.median_ms`
   and `worst_ms`, `[phone] turn_ms` less its `turn_headroom`; `fin.mem.peak_mb`, `baseline_mb` (ledger line 1),
   `bytes_per_person` and one `fin.mem.<line>_mb` for each ledger line; each fixed line's
   `fin.fixed.<line>.<b|nb|h>_core_ms`; each `[unit]` in VM ns, `fin.unit.<unit>_ns`, at `k_compute` until its base
   measures its own, and `fin.decide.spend_mind_ns` and `handler_mind_ns`; `fin.contracts.<family>_rows` from
   `[store.contracts]`; `fin.bytes.<row>` from `[bytes]`. Every key is directed down; a figure the seeder needs and
   cannot find is refused. A key a measure has put below its seed keeps the measure when the section is written
   anew, and `phx fin` fails, once the section stands, on a key looser than its seed or missing. A base step adds its figure to the design
   point, then seeds its keys; the frame's measured keys (`fin.counters`, `fin.kept`) stand beside the section.

10. **The realism reads' rules** (the plan's PC-90 to PC-93, §14.8): pre-registration by ancestry, with append-only
    reports; no tuning, by a register diff by id with `Primitive-Change`, `Primitive-Rename` and `Resolution-Change`
    trailers, read from `main`'s first-parent history so squash merges keep them, a citation that named a result or a
    finding standing corrected once a later commit cites the same entry by its sources alone; measurement code reaching the world
    only through the `Inspector`; every silent break of Part L mapped to what refuses it.

    `coverage` derives §19's rows whose System is a spec code, and those whose Spec cell lists chain or measurement
    items (keyed by those items): the first stage is the earliest stage of a step whose clauses name them, the stage
    complete is the latest of their map rows — earliest and latest in the build order the plan's §1 table gives (0,
    1, 2, 3, 8, 4, 5, 6, 7), a stage the table does not name refused — and the status is `planned` while no step
    naming them is building or done, `done` when every step completing them is done, and `building` otherwise; the
    Spec and Crate columns and the other rows are kept as written.

11. **`phx-check` itself** (`crates/apps/phx-check`) parses world crates with `syn` (with a small lexer for comments,
    which `syn` drops) and reads `cargo metadata` for direct dependencies; no rule is a text search where either can
    answer. World crates are those under `foundation/`, `kernel/`, `interfaces/`, `systems/` and `assembly/`; `apps/`
    are held only to layering. Its rule table (`src/rules/mod.rs`) records each rule's id, title and the step that
    introduced it; the rules of the foundation and first kernel crates are:

    | Id | Refuses |
    | --- | --- |
    | PC-01 | a dependency against §3.1's layers and orders |
    | PC-02 | a direct external dependency outside the allow-list (`rules/dependencies.rs`, by crate or layer; `proptest` and `trybuild` as development dependencies anywhere) |
    | PC-03 | `allow(unsafe_code)` outside `phx-store` and `phx-exec` |
    | PC-04 | `rayon` anywhere, `rayon-core` outside `phx-exec`, `libc` outside `phx-exec` and `phx-store` |
    | PC-05 | a `static` item in a world crate, but `phx-exec/src/site.rs`'s thread-local and `phx-exec/src/trace.rs`'s one static, where the bench's marks go and what the counters count across threads |
    | PC-06 | a numeric literal other than 0, 1, −1 and 2 in a world crate outside `consts.rs`, type positions, tests and benches, with the arguments of the assert, format, write, `vec!` and `violation!` macros parsed as expressions; a constant in `consts.rs` without a doc comment |
    | PC-07 | a comment matching a clause id, `Law n`, `Nn`, `§`, `spec`, a document's name, a plan step, `TODO` or `FIXME` |
    | PC-08 | a function in an interface crate other than a constructor or a field accessor |
    | PC-09 | a crate missing from §3's lists; a plan step without a valid status or its sections in order (a retired one needs only its status); two steps building; a crate whose step, still in the plan, is not yet building; and a code name in this document the workspace lacks (`rules/arch_names.rs`, over `src/names.rs`, every item any crate declares outside its tests and every source file): a backticked CamelCase identifier, `snake_case(` call, `Type::item` or `crate::path`, or path ending `.rs`, outside a §7 subsection whose status reads planned or building and unless followed on its line by `(planned, S…)`; crate names, clause ids, config keys, `perf/` and `data/` paths and shell flags are no code names (S1.131) |
    | PC-10 | counts of `allow` and `expect` attributes in world crates above their ratchets |
    | PC-11 | an `#[expect]` without a reason |
    | PC-12 | a hand-written `impl Pod` or `Sealed` outside `phx-store/src/pod.rs`, and a float in a derived `Pod` |
    | PC-13 | a direct dependency of a world crate on `rand`, `rand_core`, `getrandom`, `ahash` or `fxhash` |
    | PC-14 | `Default` on an id of `phx-id` |
    | PC-15 | a kernel or interface crate without a committed `public-api.txt`, or whose API differs from it; `public-api --write` records beside each item the crates outside it that name it outside their tests (`// used by: …`, no part of the API compared), and `public-api --unused` lists the items none names — a report, since a base is built before its users, which S1.360 reads (S1.131) |
    | PC-16 | a per-crate `clippy.toml` other than the root file minus that crate's declared exemptions |
    | PC-17 | outside `phx-id` and `phx-core`'s `calendar/`, a call of `days_from_civil`/`civil_from_days` or a number added to or taken from a day |
    | PC-18 | a primitive's value reached other than through `Prim` or `PolicyValue`; `toml` or `serde` in a world crate other than `phx-core`'s `register/` and the data readers (`phx-world`, `phx-obs`, `phx-cli`); committed data outside its places; and the placeholder SHAPEs of `data/` above their ratchet |
    | PC-19 | `Draws::new` outside `phx-rand` and `phx-core`'s `streams.rs`; `Streams`, `open_keyed` and `ObserverDraws` named outside their listed files (§5.3) |
    | PC-92 | on the day's paths — every non-test module of the core's crates (§3.1's kernel list), `phx-world`'s `day.rs` and `core_*.rs`, every `sys-*/src/rules/**` module, less the cold ones named with their reasons in `rules/hot_paths.rs` (the register and contributions of `phx-core`, `phx-store`'s saving modules, `phx-world`'s `registry.rs`, `compile.rs` and `save/`, and any `opening/` module), a new file of a hot crate hot by default — a map (`BTreeMap`, `BTreeSet`, `HashMap`, `HashSet`, the kernel's map, `PartyMap`), a trait object, a struct field typed `Vec<Vec<_>>`, `Vec<i128>`, `Column<i128>`, `Vec<Option<_>>` or `Column<Option<_>>` (a scalar total is not a field of those), or a field named `next`, `prev`, `heads` or `next_*` outside `phx-store`; its exceptions file admits today's sites (S1.120) |
    | PC-96 | in a hot module of `phx-world` or a system (PC-92's hot set; the kernel's crates implement the traversals and are not read), a whole-table walk: a call of `live_slots`, `live_every`, `open_slots`, `firm_slots`, `deposits_of`, `money_totals` or `issuer_held`, or of `all`, `totals` or `money` with no argument, or a range `0..x.len()`, `0..x.count()` or `0..x.rows()`; admitted inside what is handed to `for_chunks`, `for_agenda` or `apply_by_range`, in a function carrying `#[sweep(store, cycle = …)]` or `#[opening]`; a `#[sweep]` without its cycle is refused; its exceptions file admits today's sites (S1.121) |
    | PC-97 | in `phx-world` and the systems, a literal `None` at the pool's place in a call of a public kernel function or method taking `Option<&Pool>` (collected from the kernel's crates); in any world crate but `phx-exec`, a call `Pool::map`, `Pool::for_each` or `pool::each`/`map`, or `.map`/`.for_each`/`.each` on a receiver named `…pool`, so only the kernel's traversals dispatch; in a system's `src/rules/**`, a parameter `&mut T` but the kernel's output buffers (`DayBuf`, `DayBufs` (planned, S1.160), `IntentBuf`, `OptionSet` (planned, S1.154)); tests are not read; its exceptions file admits today's sites (S1.122) |
    | PC-98 | in PC-92's hot set, outside functions marked `#[opening]` (`phx-macros`: a marker on a function, emitted unchanged, refused on anything else), a read by name: a call of the register's readers (`count`, `fixed`, `table1`, `table2`, `products`, `stored_by_id`, `value`) or any method whose first argument is a string literal on a receiver held as `register` or `reg`; `==` or `!=` with a string literal; `.starts_with`, `.ends_with` or `.contains` of a string literal. A primitive, kind, family, reason or market is read by its handle, bound in the opening (Law 10); a literal in a message is no comparison; its exceptions file admits today's sites (S1.123) |
    | PC-99 | in PC-92's hot set, outside functions marked `#[cold]` (an error path) or `#[opening]`, the syntax of allocation: `Vec::new`, `Vec::with_capacity`, `Box::new`, `String::new`, `String::from`, `vec!`, `format!`, and `.collect()`, `.to_vec()`, `.to_owned()`, `.to_string()`, `.clone()`; a push into a kernel buffer is admitted, and the arguments of `violation!` and `capacity_exceeded!` are tokens, not calls; what the syntax cannot see the bench's allocation counter measures (K-15); its exceptions file admits today's sites (S1.124) |
    | PC-100 | in a world crate, a field of a struct deriving `Saved` (not marked `#[saved(skip)]`) that names a party or row by where it is now: a map or set (`BTreeMap`, `BTreeSet`, `HashMap`, `HashSet`, the kernel's map) keyed by `u32`, `PartyKey`, `PartyId`, `Slot` or an edge slot, or by a tuple they lead; a `PartyMap`; a `Vec<(PartyKey, _)>`. It keys by a generation-checked reference (`PartyRef`, `GenRef<T>` (planned, S1.159), `ContractRef` (planned, S1.174)) or becomes its base's column (S1.125). And in a world crate outside tests, an absent value read as zero — `unwrap_or(0)`, `unwrap_or(0.0)`, `unwrap_or(…::ZERO)`, `map_or(0, …)`, `unwrap_or_default()` — outside a function marked `#[absent_is_zero(reason = "…")]` (`phx-macros`: a marker on a function whose reason is not empty, for an absence that truly is zero, a count of an entry not there) (S1.126); its exceptions file admits today's sites. And, with no exception, a literal capacity: a `const` named `*_ROWS`, `*_WORDS`, `*_CAPACITY` or `INSTRUMENTS` whose value is a shift of literals outside `phx-core`'s `capacity.rs`, or a shift of literals handed to a store's constructor (`Column`, `Parties`, `KindStore`, `Table`, `SlotAlloc`, `Region`, `EdgeTable`, `BlockPool`, `Persons`, `ChunkArena`); a chunk size stays an engineering constant (S1.127) |
    | PC-22 | in `phx-audit`, outside tests, a world store (`Column`, `Table`, `SlotAlloc`, `Parties`, `KindStore`, `EdgeTable`, `ChunkArena`, `BlockList`, `BlockPool`, `Region`, `Persons`, `DueWheel`, `EventStore`, `Register`, `Calendar`, `Core`, `World`) held by `&mut`: the audit holds the world by shared reference; its reads of maintained aggregates are PC-101's (re-aimed at S1.128, the family context it named gone) |
    | PC-101 | a field marked `#[maintained(writer = path)]` (the `Maintained` derive of `phx-macros`, which refuses a missing writer or a type that is no integer) whose type is no integer (`i8`–`i64`, `u8`–`u64`, or `phx-num`'s `Amount`, `Count`, `Money`, `Qty`, `QtyRaw`, `PriceRaw`, `Fixed`); and in `phx-audit`, outside tests, a field access naming any maintained field, which the audit recounts from source rows instead (N1's independence), a clash of names resolved by renaming (S1.128); and in a world crate, outside tests, a call of a function marked `#[per_day]` (`phx-macros`: a marker on a function, emitted unchanged), which is only ever named as a path handed to the day's cache (`DayCached::get_or` (planned, S1.211), K-35), so a per-day value is computed once a day (S1.129); and a field marked `#[saved(skip)]` without `rebuild = path`, its exceptions file admitting today's sites (S1.130) |
    | PC-94 | a name — an identifier, or a declared name in a string — in a world crate ending `_small` or `_large`: a mechanism split by size, where a firm is one kind whatever its size (S1.24) |

    **Exceptions** (`src/exceptions.rs`): a rule widened over code that does not yet keep it admits today's sites in
    `crates/apps/phx-check/exceptions/<rule>.toml`, each `[[site]]` by path, enclosing item (a type's function, a struct, a
    module) and what was found, with a count. A site found beyond its count is a breach; a listed site found fewer
    times is a stale exception, so the file only shrinks; the total is held to `phx_check.exceptions_pcNN` in
    `perf/ratchets.toml`, which only falls. `phx-check exceptions <rule> [--write]` lists what the rule finds, and
    `--write` records it once, when the rule is widened; each migration removes its own sites in its own commit.

    The clippy exemptions are declared per crate in `phx-check` and realised by that crate's `clippy.toml`: `phx-exec`
    the atomics, `std::thread::spawn`, `thread_local!` and the `OnceLock` holding where the trace goes; `phx-rand`, `phx-store` and `phx-exec` the wrapping
    integer methods, through named helpers; `phx-cli` `Instant::now`, `env::var` and the printing macros;
    `phx-check` the printing macros. Its subcommands: `layering` (PC-01 to PC-04), `rules`
    (the table), `docs` (PC-09 and §19 against the table `coverage` would write), `clauses` (the clause map and
    carriers, item 5), `coverage [--write]`, `all`; `public-api [--write]` (with the pinned
    `cargo-public-api` and nightly) run in their own CI jobs; `check-all` (the cargo alias `check-all`) runs format,
    clippy and tests as CI does, then `all`.

A rule changes only with its reason recorded in §18.

---

## 17. Build and target

- Release: `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` with a hook that writes the violation report,
  overflow checks on; the bench builds it. The phone's own profile comes back with the phone's run of the world.

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
- Pool sized to fast and medium cores, the world's work on it; performance-hint sessions per turn are not built (§2).

---

## 18. Decision record

1. Rust, own pool, own random generator, pure-Rust mathematics.
2. Systems cooperate only through kernel channels (§4); interface crates hold shared vocabulary and rule signatures.
3. Individuals of institutional kinds are kernel rows with system-owned facets (§7.5).
4. **Relationships are rows in the holder's arena**, found from the line by its holder list. *Superseded by 36 and
   the core (S1.101).*
5. A cell's **banking arrangement is key**; balances live on rows, one per deposit kind and bank. *Superseded by 35.*
6. Levies are computed per member and may carry a follow-on written by their payee system (§4.3).
7. Sub-steps with declared reads and writes; order-free gathers (§6.2).
8. **Cost follows events**: the agenda (§7.7); continuous decisions and reviews on each cell's own days; standing flows
   settled as legs in a streamed pass; pooled flows, one leg per party per batch; only accruals posted lazily.
   *Superseded by 36 and the core (S1.101).*
9. Screening is scheduled at an envelope rate with thinning by default; daily only for dense processes (§7.7).
   *Superseded by 36.*
10. Settlement: implicit batches never materialised, but for the day's payments 7a hands 7c; a streamed payer pass; the
    greatest fixed point; failure per payer (§6.5). *Superseded by 36.*
11. **Tolerances are steps** (REP.4, amended): landing is one lookup of the landing key, a check of the lines' kinks,
    batches per target and clusters of the unlanded. *Superseded by 35.*
12. Open business pins only what belongs to particular members; balances are positions with steps (§4.2). *Superseded by
    35.*
13. A trade is one composite instruction drawing on other systems' commitments (§4.4). *Superseded by 36.*
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
    phone, the rows-per-cell curve, the unit costs and the worst closed run decide the play resolution before behaviour
    is built (§14.6). The design point's estimate leaves the median 7.6% headroom at Stage 1, short of 10%, and misses
    it through Stages 2 to 6; it misses heavy days, tolerance control on a heavy day and the worst closed runs, and
    misses memory by 1.4% through Stage 3, by 10% through Stage 4, by 13% through Stage 5 and by 16% through Stage 6
    (§13). *Superseded by 37 and the design point and budget frame (S1.103).*
21. Memory budget 4.5 GB, the owner's choice after the design point was sized.
22. **Owner decisions** (spec Appendix E 29–31): the map is about 40,000 tiles of 10 km with 25 regions allotted by
    the setup's population split (§10.0); the
    world runs once, and the accuracy for play is judged by its own macro relationships against real economies'
    (Appendix E 30, 36); the representation is coarsened for the phone (pooled flows, coarser employment lines,
    reviews on review days, sellers spread on review days) — this coarsening *superseded by 35*. World settings: the
    settling length defaults to **one simulated year** (GEN.6, adjustable); saves default to **every simulated
    quarter** (SET.12), and every save is full and takes at most **5 s** on the phone (N8.10, spec Appendix E 22).
23. **Stage 2's decisions** (*superseded by 35, 36 and the design point and budget frame (S1.103)*):
    - invoices accrue per statement period, one row per (holder, market, terms, period), dated rows in the holder's
      due-day run behind the head the settlement stream reads (§6.5); a match may draw a commitment at 6d,
      writing a `Row` leg in place of the money leg (§4.4);
    - a housing transaction's pins ride on one part (§4.2), and a bank switch is made when its transfer settles;
    - a resolution takes the book from the day's statement, is selected by the authority's least-cost rule, re-keys
      once per distinct key, and takes the next bid or the payout path at D+2 when its transfer fails (§9.2); a closed
      bank's legs are **pending**, a leg's third state (§6.5);
    - a stay is a line transfer to procedure lines (§4.4); estates are one per (part, occasion), and a personal
      insolvency's estate ends after its sale, a trustee paying the creditors from the income levy (§9.1);
    - indexed standing flows are tested against kinks on each day they post, with no envelope; the coupled call
      is an exact min-cost flow, the lines' losses bought outside it at balancing;
    - stage 9 applies its intents at 9e, and each test is one handler with the consequence it triggers (§6.1);
    - a decision point lives in the decider's crate or the latest crate its types need; a lender's `LoanAssessment` (planned, S2.102)
      carries a writer token only `sys-bnk` can build (§3.1, §3.4);
    - MMK.1's bilateral term loans are brought forward to Stage 2 on one interbank loan line kind, which the money
      market extends at Stage 3.

    Through Stage 2 the design point misses the 10% headroom on memory and on time, and the median turn misses the
    budget itself (§13); Stage 1's and Stage 2's gates measure before anything else is decided.
24. **Stage 3's decisions** (*superseded by 35, 36 and the design point and budget frame (S1.103)*):
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
      settlement (§4.2);
    - instrument outlooks and values are computed per **registered** (method, instrument) pair on days with a new
      print and shared by the cells using the method; claim values are closed-form, over a discount-factor table per
      curve and day; individuals catch up in O(1) (§8);
    - the instrument's state has one writer, the core register's instrument events; an offer of held units binds
      them, and only free units are bound (§4.4);
    - each decision point's rule is registered by its owning system, except that a lender valuing a claim on its book
      does so through `sys-bnk`; the rating scale is `if-base`'s, `fund_position` is `if-state`'s, and `bank_choice`
      is `if-securities`' (§3.1, §3.4);
    - a dealer is a party of the dealer form with its own equity; publishers are large firms with a publisher facet;
      a fund ends by L3's trigger through its own procedure, and finance companies and desks as firms (§9.1).

    Through Stage 3 the design point misses the median by 6% and memory by 1.4% (§13); the remedy planned first is
    how the world is represented and traversed, measured at Stage 1's and Stage 2's gates.

25. **Stage 4's decisions** (*superseded by 35, 36 and the design point and budget frame (S1.103)*):
    - derivatives are lines between individuals only, each line kind declaring its holder kinds; rows carry no
      `amount`, each margin account keeping the day its variation margin last settled; a house's initial margin is
      one blocked product over its accounts' sensitivities; admission hooks take a member's whole order set (§3.4, §8);
    - the contract algebra gains valued losses, benefits while a state lasts, the elective leg (which a convertible's
      conversion also uses), a contract's underlying, and balances in a declared unit that is not money; the split at
      a kink works over any row kind's per-member position (§4.4);
    - a levy takes its schedule from a policy value, the flow's line terms or the payee's fact, and a follow-on may
      write rights in a unit that is not money (§4.3);
    - every dated row kind sits in its holder's due-day run behind one head in the record (8 bytes in a cell's, 12 in
      an individual's), never reordered;
      retail many-party lines — policies, annuities, claimants, benefits and pensions — keep no holder list on their
      retail side, the stream gathering the day's due holders and a rare line-major day scanning the arenas once
      (§6.5);
    - a job's pension kind and rates are its line's terms and the scheme a member belongs to is its attachment's, so
      employment lines are not split by scheme; DB rights and DC pots are positions with steps, and DC has no
      membership row;
    - policy lines are many-party, the renewal band an attachment; actuaries project once per (cover, model point)
      and aggregate per (insurer, model point), and schemes are valued per (line, holder's age class) in a declared
      sweep on valuation dates (§8);
    - a hit carries its members' profile values to the systems a process declares interested, applied at 3e; victims
      of harm to third parties are drawn from the (zone, class) index and the zone's pieces;
    - resolution of insurers and clearing houses is `sys-sup`'s on §9.2's timetable: a closed payer's legs are
      pending, a closed many-party payer's holders are paired once for the whole resolution, and a house recovers by
      haircutting its pending legs before its service is transferred or wound down (§6.5, §9.1, §9.2);
    - pensions in payment execute from Stage 0 (spec Part O), and the statistics agency publishes a period life table
      that insurers, households and actuaries read (§14.6).

    Through Stage 4 the design point misses the median by 16.5% and memory by 10% (§13); the
    remedies are N8.7's, representation and traversal first, measured at each gate before the play resolution is
    touched.

26. **Stage 5's decisions** (*superseded by 35, 36 and the design point and budget frame (S1.103)*):
    - taxes are levies per member where their bases arise; the levies of one flow share one search of their fused
      kinks; value-added tax on a sale on terms is computed per invoice row at its statement and on a cash sale rides
      the sale's instruction; property tax is a holding levy driven from the (zone, class) index, joining the
      holder's existing leg; duty and import tax at a border are a demand customs issues, not a levy; returns are
      day-local (§4.2, §4.3);
    - a policy schedule's count of bands is the constitution's and fixed at compilation, so a budget moves values and
      edges only; an effective day re-keys only the cells whose signature words the moved kinks touch (§5.3);
    - waits and pending claims are attachments, not pins; benefit rows are dated rows with no retail holder list;
      the state pension's earnings-related rights are one follow-on leg per cell per payday (§4.2);
    - vote intentions are a windowed profile group, written and cleared by declared sweeps `phx-pop` runs at 10b; the
      `vote` review's exposure and attention live in a side column only while a country's campaign runs; a platform's
      value is a key-and-step part per (landing key, platform) plus shared benefit and service parts, evaluation at
      the step a tolerance of REP; the tally is a declared sweep at 10a, which runs every day, and
      renumbering by country keeps the sweeps contiguous (§6.1);
    - no system registers a handler at a kernel apply: intents, tallies and sweeps are declared and the kernel runs
      them (§6.1, §6.2);
    - money changes currency only by a trade: a bank is the counterparty of its customers' conversions, from its own
      currency book, drawn at 7a as a conversion commitment that a capital rule's handle gates; the payer pass and
      nets are per currency, and a leg whose currency has no stage 7 that day waits, pending, for a business day of
      both; a fixing converts nothing, and `phx-acct`'s translation is the one translation (§6.5, §8);
    - the balance of payments is fed by declared per-reason tallies, a per-category vector per payer at 7a added for
      the survivors at 7c; `Row` legs on cross-border lines are financial account (§6.5);
    - one closure until Stage 5: `XB.closed_borders` in `phx-market`'s `Reach`, retired in two parts; distances cross
      a border only through declared crossings, zone to crossing;
    - a peg's break is the central bank's fact that its quote's selling side bound, the regime staying with its
      owner; currency derivatives are marked per (pair, maturity) from 6c's forward points (§8);
    - parties are paid per vote by the treasury and stand on a deposit, employ staff and buy polls; pollsters are
      ordinary firms with the publisher facet; forms another system wrote are registered for new kinds by that system
      (§3.4);
    - migration is the upper nest of the housing review's occasion, its inclusive values memoised per (outlook
      method, occupation-family set, destination) a day.

    Through Stage 5 the design point misses the median by 23%, a heavy Monday by 28% and memory by 13% (§13); the
    remedies are N8.7's, representation and traversal first, then the play resolution, a valve set
    by measurement (spec Appendix E 36).

27. **Stage 6's decisions** (*superseded by 35, 36 and the design point and budget frame (S1.103)*):
    - ways are issued at runtime, an improvement stored against its base as factors, so a recipe is one read; a
      `WayId` is issued only from a discovery or imitation event and never reused; a way's owner and patent end are
      its patent holding's (§3.4);
    - every firm knows its industry's public ways, held once per industry (TEC.4, spec Appendix E 42); a firm's own
      known ways stay whole in its key (dominance at today's prices does not last when prices can be negative),
      `sys-tec` their one writer; the firm cells distinct sets keep apart are a measured floor, and a
      profile of known ways not run is the proposed lever;
    - cumulative output per way run is a keyed position list in the firm's arena, stepped logarithmically with the
      learning curve's kink declared, so learning re-keys in place; the curve's power runs only past its next
      rounding threshold;
    - skill and the labour-market state are reads of a role's profile and clocks (the employment's start band, the
      search band), skill written in place only when an origin changes: no hazard, no agenda reason, no daily work;
    - the education record is its own profile group; compulsory school stages are a read of birth year and the law;
      courses and waits are attachments changed in place;
    - meetings are one hazard per region over its singles' counts, each occurrence an event with two subjects drawn by
      pairing and a day-local `Meeting` message to each; `form` runs on the meeting's day;
    - a formation is one `combine`d part from two origins, a leaving or separation one `divide`d part; a new
      household's key attributes come from their declaring systems' `.at_formation` handles, its preference type
      drawn, and nothing but money and units adds (§9.1);
    - an adult leaving to move runs `migrate` before `where_to_live`, as any mover (§6.1);
    - POP.11 and POP.12 run on the audit's rolling cycle by region;
    - two builds, the participant's through `ParticipantScope` (planned, S6.131) and the inspector's behind a feature; every shown
      number a `Shown<T>` (planned, S6.131) from its source; views and pages on a turn's last day, tracers every day; tracers' older
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

30. **The apply model, the audit sink, saved state and the build run** (*superseded by 36 and 37*):
    - an intent applies at the first apply point at or after its sub-step; the kernel applies (2f, 4b, 5d, 6d, 7c,
      9e, 10b) are a sub-step kind of their own that runs every day its stage runs, and assembly refuses a system
      handler there (§5.4, §6.2);
    - the audit's sink is declared by `phx-core`, implemented by `phx-audit` and injected by `phx-world`, so the
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

32. **The finished world's load at every gate** (*superseded by 37 and `phx-fin` (S1.116)*):
    - from Stage 0's gate, the full-load bench runs random data at the finished world's volumes and shapes through
      the real kernels on the phone, and the gate is judged on it as on the stage's own run (§14.5, §14.6);
    - a miss takes N8.7's remedies before the next stage starts, so the representation and the play resolution are
      set against the finished world from the start;
    - no thermal warm-up precedes a measured year: the budget is the year of consecutive turns (§13.2, §14.5).

33. **The phone app is built by CI** (*superseded by 37*):
    - CI's `android-app` job builds the engine for the phone with fat LTO, writes UniFFI's Kotlin bindings from the
      built library and assembles the bench flavour, which it uploads as an artifact; the owner installs that
      artifact, runs it and commits the report it writes to `perf/device/` (§14.5, §14.6);
    - the app's tool versions are pinned in `android/` (Gradle's version catalog and the wrapper, with the
      distribution's checksum), beside `tools/versions.toml`'s NDK, API levels and build tools.

34. **The population as agents** (*superseded by 35*) (2026-09-25; spec REP.1, REP.40, Appendix E 14, 16, 44):
    - cells are retired: every household and small firm is an agent with its own attributes, persons and rows, never
      split, joined or averaged; with cells go keys, profiles, steps and tolerances, parts and landing, pooled flows
      across members, choice groups, the decision gap, promotion and demotion, tracers, seller spreads and
      renumbering;
    - an agent stood for a multiplicity of identical twins, the full population held as agents of many twins or a
      small world of a share of it, until 35 retired twins;
    - lines still record many-party relationships as counts; individuals are the institutions and the parties
      ranked at the opening, and the player's household is an agent;
    - the size is the representation's one valve (N8.5, §13); §13 keeps its estimates, with the work only cells
      needed marked retired, until the first measurements replace them.

35. **One representation: a world of a declared size** (2026-09-27, the owner; spec REP.40, Appendix E 44):
    - the world holds a declared number of the setup's persons and every household and small firm they form is one
      agent of one party (§13); `REP.persons` is the one primitive, 750 000 on the build machine's measure until the
      device resets it, and `phx run --persons N` sets another;
    - twins are removed from every layer: the agent table's multiplicity column, the ledger's holder members and payer
      weight, the pooled-flow rule and standing flows' kinks (built for twins and cells, used by nothing else), the
      tally's unit classes and like-with-like draws, the losers drawn past the failed, a trade's and a tax's whole share
      per twin, the labour round's units and the vacancy lot, the opening's twin counts and the player's seated twins;
    - with twins went the reason only large firms held a deposit's right, so small firms draw rights and extract
      (`GDS.extract_small`);
    - the benches keep the design point's agents; the load report's version is 3.
36. **The old kernel deleted** (2026-09-29, S1.24): the core is the world's one kernel; the old kernel's books,
    handlers, agent tables, audit, markets' instances and declarations nothing on the core read are deleted crate by
    crate, their clauses carried by the core or moved to the steps that build them. PC-19 names the files that open
    keyed streams on the core (the state's claims); PC-21 guards the sub-steps the code names, the macros' copy of the
    table gone with the macros; PC-94 is added, refusing a mechanism split by size. S1.25 restates or retires the
    rules over the retired machinery.
37. **One bench** (2026-09-29, the owner: one benchmarking file for the new world, every other deleted): the world
    itself, run by `tools/bench.sh` on this machine, is the one measure. Each day the core times its stages by the
    run's clock (`Core::timed`), never a world state, and the report carries them; the trace tells every span and note
    as it happens, so one run shows where any time goes and where a stuck run stands; the bench's flags set the persons,
    days, seed, workers and checks, `-g` a gate's run and `-k` a kept report in `perf/bench/`. Deleted with it: the
    smoke and the build run (`tools/smoke.sh`, `tools/build-run.sh`), the build runs', device and measurement reports
    and the device report's schema, the full-load bench and its volumes, the phone's bench app and its library
    (`phx-ffi`, the `bench` flavour, CI's phone library and bench APK), the kernels' micro-benchmarks with their
    instruction ratchets, their CI job and `phx-check`'s `bench-ratchets`, and `phx measure`. Superseded: items 30 and
    33's device-report and phone-library provisions. The phone's run of the world is rebuilt on the bench's report
    when the owner calls the device run.
38. **The core first** (2026-09-29, the owner; spec N8.7, N8.8, Appendix E 51): every base the finished world needs —
    every store, index, traversal and kernel — is designed at the design point and built in the kernel crates of §3.3,
    in the order of §3.1, before any behaviour is put on it; each base is followed by the migration of its current
    users, and every later step activates bases. `phx-world` assembles and routes and, once the core closes, holds no
    store and no per-party pass; systems hold rules and declarations. PC-01 holds the kernel's order of §3.1 and the
    applications' (`phx-fin`, `phx-cli`, `phx-play`, `phx-check` apart), `phx-fin` reading the foundation and the
    kernel only; PC-02 allows `serde`, `toml` and `serde_json` in `phx-fin` (the design point's file, the budget
    and its report). Supersedes the kernel's crate list of decision 36.
40. **The finished-volume measure** (2026-09-30; spec N8.8, Appendix E 51): the one bench measures the code at the design
    point as well as the world at the committed resolution: `-F` fills each base through its driver and composes the
    day and the turns from measured lines where a driver exists and the steps' declared figures where none does yet,
    each base's step adding its driver; its numbers are the code's, never the world's (§14.7).
39. **The design point and the budget frame** (2026-09-30, the owner; spec Appendix E 51, E 41, N8.7): every base is
    designed and measured at the design point, `perf/design.toml`'s one figure set, which the plan and this document
    cite by key and never restate; the time lines are the binding turns with every dated day and a campaign day on
    them and each event composed in turn; the memory ledger is the steps' own statements summed by line; a miss is
    met by representation first, then the valve in E 41's reverse order. Supersedes the budget tables of decisions 23
    to 27.

---

## 19. Coverage

Generated by `phx-check coverage` from the clause map. Status: planned, building, done.

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-id`, `phx-core`, `phx-world` | 0 | 0 | done |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 5 | building |
| A3 | NUM | `phx-num`, `phx-core` | 0 | 1 | building |
| A4 | CHN | `phx-rand`, `phx-core`, `phx-pop` | 0 | 6 | building |
| A5 | GEO | `phx-geo` | 0 | 2 | building |
| A6 | REP | `phx-pop`, `phx-store`, `phx-world` | 0 | 6 | building |
| A7 | GEN | `phx-world` and every system's contribution | 0 | 7 | building |
| B1 | MON | `phx-core`, `phx-world` | 0 | 1 | building |
| B2 | SET | `phx-core`, `phx-store`, `phx-world` | 0 | 1 | building |
| B3 | REG | `phx-core`, `phx-ledger` | 0 | 3 | building |
| B4 | ACC | `phx-acct` | 0 | 2 | building |
| C1 | MKT | `phx-market`, `phx-acct` | 0 | 1 | building |
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
| F7 | ENE | `sys-ene` | 1 | 4 | planned |
| G1 | BNK | `sys-bnk` | 0 | 4 | building |
| G2 | BFL | `sys-bfl` | 1 | 3 | planned |
| G3 | BCP | `sys-bcp` | 2 | 3 | planned |
| G4 | SEC | `sys-sec` | 4 | 4 | planned |
| H1 | MMK | `sys-mmk` | 1 | 4 | planned |
| H2 | SOV | `sys-sov` | 1 | 5 | building |
| H3 | CRD | `sys-crd` | 3 | 3 | planned |
| H4 | EQY | `sys-eqy` | 3 | 4 | planned |
| H5 | MNA | `sys-mna` | 4 | 4 | planned |
| H6 | FND | `sys-fnd` | 1 | 4 | planned |
| H7 | DLR | `sys-dlr` | 3 | 4 | planned |
| H8 | IDX | `sys-idx` | 1 | 3 | building |
| H9 | RAT | `sys-rat` | 2 | 4 | planned |
| I1 | DRV | `sys-drv` | 1 | 4 | planned |
| I2 | DRX | `sys-drx` | 4 | 5 | planned |
| I3 | INS | `sys-ins` | 4 | 4 | planned |
| I4 | PEN | `sys-pen` (no crate yet: Stage 0's state pension is `sys-soc`'s) | 0 | 5 | building |
| J1 | TRS | `sys-trs` | 1 | 5 | building |
| J2 | TAX | `sys-tax` | 0 | 5 | building |
| J3 | SOC | `sys-soc` | 0 | 5 | building |
| J4 | CB | `sys-cb` (from Stage 0: the central banks and treasuries as parties) | 1 | 5 | building |
| J5 | SUP | `sys-sup` | 1 | 4 | planned |
| J6 | POL | `sys-pol` | 5 | 5 | planned |
| K1 | FX | `sys-fx` | 1 | 5 | planned |
| K2 | XB | `sys-xb` | 5 | 5 | planned |
| M1 | OBS | `phx-core`, `phx-obs`, `android/` | 0 | 6 | building |
| M2 | STA | `sys-sta` | 1 | 6 | building |
| L3 | estates | `phx-world` until `sys-est` at Stage 2 | 0 | 7 | building |
| L1, L2, L4–L12 | transmission chains | read from the run by `phx chains` (N4) | 1 | 7 | planned |
| N1 | audit | `phx-world`'s `core_audit` until `phx-audit` (§7.17), and every system's families, run in every gate's run | 0 | 6 | building |
| N2–N8 | measurement | `phx-cli`, `phx-fin`, `tools/bench.sh` | 0 | 7 | building |
