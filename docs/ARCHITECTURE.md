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
  tens of thousands of large firms and institutions. The population is carried as agents, in twins or a small world
  (REP.40).
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
| Parallelism | **Own pool** in `phx-exec` over **rayon-core**, pinned to fast and medium cores, with Android performance-hint sessions | Cost-sized fixed chunks, no little cores, few barriers. Only `phx-exec` depends on rayon. As built at Stage 0 the world runs on one thread: the pool serves only `phx-ffi`'s benches, and performance-hint sessions are not built (`PerfHint` has only `NoHint`, called by nothing); making the world's traversals parallel is the plan's F-056. |
| Hash maps | **hashbrown** + fixed-seed **foldhash**, behind a kernel map type with no iteration, sharded where written in parallel | Fast lookups; no outcome depends on hash order. |
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

### 3.3 Kernel

| Crate | Carries | Owns |
| --- | --- | --- |
| `phx-store` | SET.12, SET.15 | Paged columns in reserved address space; **chunk-local arenas** compacted in place; slot allocators with recycling; column descriptors; save encoding. |
| `phx-exec` | TIME.6 mechanics, N5 | The pinned pool; cost-sized chunked traversals over the day's **agenda** or a whole table; gathers by prefix sum keyed (chunk, handler); sharded `KeyedReduce`; fixed-tree reductions; radix sorts. At Stage 0 the world calls none of the pool's traversals or `KeyedReduce` (§6.3); `phx-ffi`'s benches and the crates' tests do. |
| `phx-core` | TIME, PTY, NUM.3, NUM.7, CHN.2–CHN.4, OBS.1, OBS.3 | The **vocabulary every system and kernel crate declares with**: the `System` trait, `Declarations`, handler declarations and contexts (`Ctx`), the sub-step table, audit-family declarations and their read-only context, the **audit sink** (`trait AuditStream` and the touched-row bitmap, which `phx-audit` implements and `phx-world` injects, so `phx-ledger`'s apply feeds the audit without depending on it), opening contributions, the kink registry, the traits kernel crates meet through without depending on each other (`BooksAudit`, `MarketsAudit`, `AccountsAudit`, `LegRecords` and `CellsAudit`, the books, the tape, the accounts, the settled legs and the agent tables as the audit reads them, §10.4c); calendar and conventions; **decision schedules, wakes and the agenda** (§7.3); the party directory with bounded tombstones; **kind tables of individuals** with facet columns (§4.1); kinds; the primitive register, `DeclaredLimit` (a real limit constructible only from the register, a contract's terms, or a physical token that only `phx-ledger`'s holdings and `phx-geo`'s stock can build, for a capacity) and **policy values** (§4.6); **facts** (§4.1); **messages** (§4.2); **rule handles** (§4.7); hazard and occasion declarations; **public records** with audiences (§4.9); events; findings; party creation and ending. |
| `phx-geo` | GEO | Tiles, map generation, regions, zones, distances, network capacities, deposits, exposure, **physical stock per (tile, class)** and the (zone, class) index of holdings (§7.8). |
| `phx-ledger` | MON, SET, REG, L3's ranking | The **contract algebra** (§4.4); lines and their holder lists; **relationship rows** in holders' arenas (§4.5); instruments, holdings (agents' counting their multiplicity) with the holder index, lots, liens, **commitments**; instrument events and the instrument's state, of which it is the one writer; `Covered<Qty>`, the quantity an offer of held units takes, which places a commitment on them; **levies** (§4.3); **instructions**, composite instructions and implicit batches; settlement (§6.5); **standing flows** and a party's pooled legs (§6.5); fail records and payment records; transformation records; **line transfers**, including the split at a kink (§4.4); the estate waterfall. |
| `phx-pop` | REP | **Agent tables** (§7.1): attributes, positions and keyed position lists (§4.5), persons and attachments; hazards drawn ahead per agent and the persons a hit reaches (§7.3); outcomes and endings (§7.4), and a household's `combine` and `divide`; the representation (§7.6). |
| `phx-market` | MKT | The six forms (§8), the coupled call and the **linked call** among them; prints, marks, and instruments' and currency pairs' fixings at 6b by the pricing service's declared method; admission hooks; market failures. |
| `phx-acct` | ACC, MKT.20 | Valuations and valuers; statements; a group's consolidated statement as a pure read; carrying bases and unrealised differences; equity accounts, with the receivables and payables of dues as they fall, posted from the day's settled records by the kernel's own work at 9b on a business day and, on a day with no 9b, at the audit's sub-step (10d) before the close reads them. The ledger's apply raises an equity effect for a money leg, a row leg that adjusts a balance, and a move of a holder's cost of lots, each signed by the leg's move of its party's net assets, so a leg passing through a party moves its equity by nothing. |
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
| `if-base` | shared identifiers and declared data: products, occupation families, skills, capital kinds, ways (issued at runtime from Stage 6, an improved way stored against its base as factors; the rule handle `labour_per_unit`), units; as built at S1.02, the products, the way and the way-set identity as data only, what a way takes and finishes and the way-set interner being `sys-tec`'s; rating scales and notches; money-market tenors, segments, collateral baskets, haircuts and limits; agents' participation per asset class; pension kinds, contribution rate points and fund-menu identifiers, so a job's terms need no later crate |
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
committed. From Stage 7, `data/measure/` holds the realism reads' registered definitions, which no world crate reads,
and `perf/{realism,chains,register}/` their reports, append-only (§14.8). `perf/device/` and `perf/measure/` hold the
gates' reports; `perf/build-run/` the build runs' reports (§14.7); `perf/load/volumes.toml` the full-load bench's
volumes (§14.6); `perf/schema/` the device report's schema; `perf/ratchets.toml` the counters' values.

---

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
are the agent's positions, whole multiples of its multiplicity (REP.9). A person's wait for a public service and its
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

### 4.5 Relationship rows

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
  **attribute** (REP.41). Every deposit row of an agent therefore counts its multiplicity, and a twin's share of a
  balance or a pending amount is it ÷ the multiplicity (REP.9). Each deposit kind declares which payments it funds and
  in which order (current before savings; term deposits never). Changing bank is a lumpy decision of the agent
  (REP.5), made when the transfer of its balances settles; the
  arrangement's one writer is `sys-bfl`, which also rewrites it, at 7e, for the holders a resolution's transfer moved.
  Coverage (SUP.2) is per legal person: for a household, limit × the arrangement's adult holders; for a firm, the
  limit once; over the depositor's balances at that bank in the declared coverage order.
- **Banknotes** are a holding (instrument: the central bank's notes in that currency, MON.1); a purchase paid in
  banknotes moves them between holding rows at 6d.
- **Loans** carry balances; an agent's row holds its twins' identical loans, with one terms, vintage and payment
  record, so each twin's balance is exact (REP.9).
- **Fails** become fail records with the reason and the line; the owner of the line kind reads them at its declared
  sub-step (§6.1). **Payment records** are each row's own, never combined; lenders' and suppliers' views read them.
- **Holdings** of instruments are rows in the holder's arena; each instrument keeps its holders sorted, so coupons,
  dividends, bail-ins and REG.13's audit read holders directly (REG.4). An agent's are `{instrument u32, count u32,
  quantity i64, pooled cost i64}` (24 bytes), one lot at average cost (REG.1), `count` its multiplicity, like a
  relationship row: a twin's quantity is quantity ÷ count (REP.9). Which asset classes an agent holds directly —
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
  line's per-time due; each twin's right is its share (REP.9). A DC pot is a holding row of fund units with its
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

### 4.6 Policy values

A **policy value** is a POLICY primitive its owner may change during a run. Its opening value comes from `data/`;
after that it is a fact whose one writer is the owner's decision. Each
change writes a dated announcement with its effective day, at least the next business day (VAL.6, POL.7). A change
that moves a hazard's rate books its process afresh on the agents it concerns (§7.3).

### 4.7 Decision points, deciders and rule handles

- Every decision the spec names is a **decision point**: an input view type and an output intent type from an
  interface crate, a rule function registered by its system, a declared **schedule** (TIME.5) and **wake conditions**,
  and a pure evaluation form usable off-world (REP.15). The **decider** is the kind's rule or the player, derived for
  a party from `PlayerQueue` (§12), not stored per party. The kernel dispatches inside the decision point.
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

### 4.9 Records, audiences, scoped reads

Decision kernels read what their party may: its own rows and facts; the relationship rows it is holder or counterparty
of; records whose audience includes it; earlier prints and marks. As built, a handler's `Ctx` reads and writes a
declared fact by slot (`read::<F>(slot)`, `write::<F>(slot, v)`), and has no per-party accessor. Audience is checked at
compile and assembly time from declarations; `read-trace`, a run-time flag of release builds, on in every build run
(§14.7), samples chunks and verifies reads at run time. Store-wide views go only to processes and applies.

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

### 5.4 Assembly refusals

A fact with no writer or two; a message kind with an unanswered addressee kind or no answering sub-step; two handlers
in one sub-step where one writes what the other reads or writes; a levy without a schedule owner, or a follow-on not
written by its payee system; a kink on an undeclared position; a primitive without value, unit, kind or source; a
decision point without an evaluation form or schedule; a rule signature without an implementer; a line kind without
declared transfer requesters; a commitment kind without the legs it creates; a hazard without a draw scheme (REP.7);
an interface item whose writer is not registered; a system handler at a kernel apply (§6.2); a handler with no body;
a handler on a table the world does not keep.

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
(hazard draws, outcomes, settlement) is not counted in it, and no ratchet reads it (F-060).

The kernel's own small tables (regions, countries) keep their facts as columns (`FactColumns`) and are traversed chunk
by chunk on the day's thread, since each is one chunk. The pool's traversal arrives with the first handler on a kind
table, whose chunks are handed to workers as disjoint column views; the order of chunks and handlers, and
so of intents, is the same either way. As built at Stage 0 the world runs on one thread: the only handlers are
`phx-geo`'s on its kernel tables, and the kernel's work — 3b's hazard draws, 3e's outcomes, stage 7 and the audit —
runs serially; `--workers` is only reported. Chunk-parallel traversals are the plan's F-056.

### 6.4 Compute, gather, apply

Handlers write their own rows and emit intents into (chunk, handler) buffers; gathers place them by prefix sum; applies
run in declared order, in parallel over disjoint targets. **One apply routine** serves every apply point (§6.2): it
settles the instructions of reasons allowed there (§6.1), writes transformation records, and checks and feeds the
streaming audit through `phx-core`'s `AuditStream`, which `phx-world` injects (§3.3). Every reduction runs over a fixed
tree. At Stage 0 it applies event intents only; outcomes, settlement and estates are the kernel's own work (§6.2).

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
  by the pooled-flow rule (§7.5), with the first failing row recorded. Banks' nets are sums over parties. Nothing is
  written per leg and no payee reduction is needed. For a due line with no retail holder list, the (holder, row) pairs
  met are gathered into the day buffers, which 7b and 7d read in its place. As built at Stage 0: the stream keeps a
  record per party, and the day's lookups (each line's reckoning and ower, each party's account), in the kernel's
  fixed-seed map, read whole only in key order, and the claimants per line in `BTreeMap`s; the pooled-flow rule is
  given each agent as one payer, every row reaching all its twins, so no row splits an agent's funds (F-065); and no
  gather of unlisted sides is built (F-058). The stream keeps the day's payments in its order (about 157 MB on the
  twins payday of 2.8 M payments, the only field PC-27 lets keep a batch's items), and 7c's gather reads them, reckoning again only a cleared line's claimant credit
  after its losers; both compute each payment's route, 7c nets it in the fixed-seed map and sorts the nets once, and
  a due record is kept per payment for the accounts (F-057).
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
- **A party with no money**: a payment whose payer or payee holds no money in its currency — no account, and none it
  issues (`Books::holds_money`), as a household that banks nowhere until banknotes are held (S1.09, S1.12) — has no
  legs and fails at the start of 7b, with the cause `FailCause::NoMoney` (MON.12). A cleared line's payer with none
  fails as a short one does, its dues lost by drawn claimant contracts; a cleared line's claimant with none
  loses its due against the top issuer, which holds no row to be in arrears on, and the top is the one the line's
  holders with money reach.
- **7b** starts from every payment succeeding and removes, until nothing changes, the payers who cannot pay given the
  payments still standing, and the customer legs of banks that cannot cover their nets after intraday credit (MON.3,
  MON.5), each such payment failed with the bank's cause, `FailCause::BankShort`. A removal revisits the removed payer's due lines through their holder lists or the day's gather, lowering the
  credits of their other side and the nets of their banks. As built, only through holder lists, so a side that keeps
  none — a failing bank's household depositors, a lender's household loans — is not revisited (F-058). A payer fails as
  a **prefix** of its payment order (REP.18), which is monotone, so the result is the **greatest** set that can settle,
  and rings of payments that can settle together do (TIME.6).
- **7c** nets the surviving payments' legs per (line, party, side), money and rows apart, and applies them as one
  instruction per line in line order, read straight from the ordered nets: every account is checked once against its
  net, so no order of application can fail what 7b let stand, and banks' reserves move once per bank by net; applied
  is final (SET.5). The apply is sequential while the day's payments are few, its nets an ordered `BTreeMap`; the
  split by target chunk across the pool waits for the phone's measure to call for it (the plan's F-021). Failure is
  per payer (MON.5): a payer that cannot pay fails its own legs; where the pairing to its payees was not recorded, the
  payees who lose are drawn (REP.23).
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

Where the day breaks it is the plan's F-087 and S0.26f. The representation that meets it, each part with the
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
  whatever the order (Tarski), and a round can take every short payer at once, on every worker, and reach the same set.
  A bank's removal of its customers' payments is not yet: it reads the bank's debit, which a customer's own failure
  lowers, so it must read the customers' payments that stand once their payers are done (F-088). A cleared line's
  losers depend only on its failed count. The result is deterministic by construction (Blelloch, Fineman, Gibbons and
  Shun, *Internally deterministic parallel algorithms can be fast*, 2012). 7a runs over due-holder shards, and 7c's
  nets are grouped by a counting sort on (line, party, side) and applied by target shard.
- **Day buffers** live on the books across days: cleared by epoch, never freed. After the first heavy day the kernel
  maps no new pages.
- **Verdicts and audits** fold into the passes that already hold the values. An account's movement is checked where
  its net applies. A failed payer's shortfall is read at its cursor. Per-line balance totals are kept at the apply,
  so the audit's money and contract families read sums and a rolling slice. The accruals' mismatches are running sums
  per line. Arrears keep their start and compute their age when read.
- **Agents.** 3e's outcomes run over agent chunks on the pool (an agent's writes are confined to it, PC-34). Hazards
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
heads, not by an index; 7b runs on one thread (F-088); 7c's writes apply line by line; the money and contract families
still read an unlisted side whole. The rest is carried to Stage 1 (the plan's S0.26f, F-087).

---

## 7. The population engine (REP)

### 7.1 Tables

One **agent table** per population kind — household, small firm — in `phx-pop`. Each row is an agent (REP.1): a
household or small firm of its kind's multiplicity, never split, joined or averaged. The player's household is a row
of multiplicity one (OBS.4). Institutions and the largest firms are rows of the kernel's kind tables of individuals
(§4.1).

The books keep the agent tables beside the kind tables, one holder place each after the kind tables', known to
`phx-ledger` only through its traits (`HolderTable`, `CellHolders`), so the settlement stream, the holder lists, the
directory and the saves reach agents as they reach individuals; `phx-pop` reaches its own tables by their type. What
the tables do not hold — each kind's compiled declaration, the agenda and the members counted by event — is the
world's `Population`, which the opening's contributions are handed beside the books.

An agent row keeps, in columns: its party, the day it began, its **multiplicity** (`u32`), one `u32` per declared
**attribute** (REP.41: a household's region and bank, a small firm's region, size and bank), the due-day run's head
(§4.5), and arena references to its lists in its chunk's arena:

- **rows** and **holdings**, the ledger's (§4.5), each row counting the agent's multiplicity times the contracts its
  twin holds on the line (REP.3);
- **persons** (REP.26), one word each: the birth date as the calendar's civil day serial (32 bits, so the oldest
  persons, born before the world's epoch, are held exactly), the role (head, partner, adult, child), and the kind's
  declared person attributes packed after it (sex, health, education), their widths compiled from their values;
- **attachments**, one word each: a person's place in the household (or the household itself), the line and the
  side, so a person's death finds the rows it held without reading any other household.

Positions (REP.20) arrive as declared `i64` columns with the steps that need them. The table counts its agents as
they begin and end, and the population counts their parties and persons by the events that begin and end them, so
the day's measures (REP.15) visit no agent; the agents family checks the three against the table (REP.14).

### 7.2 Arenas, locality and identity

- Variable-length lists live in **chunk-local arenas**, compacted in place per chunk with a chunk-sized scratch;
  freed pages return to the system. Columns sit in reserved address space; nothing reallocates; no store holds a
  heap-owning type.
- Agents are drawn region by region at the opening, so a chunk holds neighbours; an agent keeps its slot for life,
  and an ended agent's slot is reused by the next one to begin, so no renumbering is needed.
- The **directory** maps permanent identities to slots. A party that ends leaves a tombstone only while a record or
  an event names it (a reference count, taken when the record or event is written), and until the day's close in any
  case, so the day's legs that name it still resolve for the audit; its slot is recycled at once. A row's arrears go
  where its contracts go: a row retired takes its arrears entry with it, and a line transfer carries it to the row
  that takes the contracts.

### 7.3 The agenda and hazards

The **agenda** of a day lists the agents that act that day: agents whose decision schedule (TIME.5) falls today,
agents woken (a message, a surprise, the player's intent), and agents with a hazard booked today (payments are the
settlement stream's, §6.5). `phx-core` keeps each agent's next day per reason — a reason per process on its kind —
and **one calendar entry per agent**, at the earliest of them, in a timing wheel of day buckets; 3b gathers today's
bucket and reads each agent's reasons due today.

Every process on a kind is drawn **ahead** (REP.7, the next-reaction method). For an agent and a process, from a day
_d_: every person's daily chance _q_ᵢ is read at _d_ from the agent's own state, and the agent's chance of a hit on a
day is _p_ = 1 − Π(1 − _q_ᵢ), constant until the next day any person's rate may change (the process's
`changes_after` for each person, the earliest of them, `c`). The days to the next hit are one geometric draw of
chance _p_ from the agent's stream for the process; if the hit falls before `c`, it is booked with the agenda's mark
of a hit beside its day, and otherwise a **redraw** is booked at `c`. Both are exact, since a day's chances are
independent of the days before: the redraw starts afresh from its day. Whatever changes an agent — an outcome, a
person leaving or joining, a new attribute — books every process on it afresh from the next day, at 10b; the
opening's agents, the player's seat among them, are booked the same way once the opening is done.

A booking come due is followed on from its own day until one falls after today: a hit reaches its persons and the
next booking is drawn from the day after it, a redraw draws from its day, and the persons a hit reached are passed
over by the later draws of the day, since their outcomes have yet to change them.

On a booked hit day, 3b draws which persons the hit reaches, each by its own _q_ᵢ **conditioned on at least one**
(`binomials_joint_at_least_one`), records the event, and hands the agent to 3e. A hit on a person of an agent of
multiplicity _k_ is that person's hit in every twin (REP.1): the outcome is the agent's, once.

A hazard on a party itself (a firm's plant failing, a vehicle's accident) reads the agent with no persons: its _q_ is
its own. Catastrophes are drawn at 3a (§7.8).

**Decisions on the agenda (visits).** A system declares each decision it takes on the rows of a kind as a **visit**
(`Declarations::visit`): the handler that takes it, the kind — a kind of individual or a population kind — its stream,
the wakes it answers, and its **cadence**: a continuous decision's schedule (TIME.5), each row at its own phase within
the period, drawn once and kept beside its booking; or a lumpy decision's reviews (REP.21) at the daily chance the row's
attention position holds (REP.38), none while it holds none. The assembly refuses a visit whose handler is not its
system's, runs on another table or outside 5b and 5c, or whose attention the kind does not hold, and, until wakes
reach visits with the markets that print surprises (plan S1.05), one that names a wake. The visits keep an
agenda of their own beside the processes' (`Population::visits`), one table per visited kind, a reason per visit, saved
with the population. Every row is booked once the opening is done; the rows due are gathered on the day's first decision
sub-step, each visit's handler runs on its rows due in runs of consecutive slots over its kind's table — an agent
table's positions (REP.20, declared on the kind with `PopKindBuilder::position`) or a kind table's facts — and each row is
booked again after it: its schedule's next instance, or a review drawn afresh at the attention its decision left it.
A review's row whose attention another visit's handler moves is booked afresh at the new attention the same day, so a
review is never drawn at an attention the row no longer holds. On a run that traces reads, a handler's reads and writes
on its rows are checked against its declaration and counted into the day's read trace. What each day's visits did —
the rows each handler visited and the facts they moved to a new value — is kept as the day's `VisitDay`. A row
that ends leaves its bookings to lapse: a due row no party holds is passed over.

### 7.4 Outcomes and endings

At 3e each agent hit that day is made explicit — its attributes by name and its persons by role and attributes,
`phx_core::Household` — and each process's outcome changes it in the processes' declared order (POP.3 before POP.4).
Then:

- a person gone (a death) leaves every row its attachments name: the row's count falls by the multiplicity, and the
  line's other side loses as many contracts, drawn from its holders by their counts (REP.23), in one
  `members_leave` per row. A holder drawn gives its whole **unit** — an agent's multiplicity, one for an
  individual — and a draw is made only among the holders whose unit fits in what is left, so an agent's twins stay
  alike. What no unit fits (an individual's count need not be a multiple of the agents' units) passes instead, by
  moving members, to other holders of the leaving party's own side whose unit fits in it, each drawn by its members
  and taking every whole unit of what is left whatever it holds, as a buyer of the contracts would take them, with no
  arrears carried, since no one failed them (the plan's F-074); if no holder's unit fits there either, the run stops
  (REP.31). An agent drawn loses, from its persons, a
  twin's share of the members that left, drawn among its attachments on that side of the line
  (`World::detach`), so its persons still hold what its rows count. A side's members by holder are read once
  (a `Tally`, from the holder list's keys straight to their rows, in fixed shards on the pool) and kept, moved by
  each leaving and passing, while no other change reaches the side. A side that keeps no holder list is read once a
  day, by `Books::read_unlisted` in one pass over the tables of the kinds that may hold it, before the estates that
  leave against it; every side's tally is let go once its pass is done (`forget_tallies`), so none outlives the
  changes the next pass brings; the households' sides of employment and tenancy keep
  lists, since a firm's end reaches them (§4.5);
- the household's persons and attributes are written back in place; the head's place, if emptied, is taken as the
  outcome says;
- a household no one is left in **ends**: an estate is opened at a zone centre of its region, with the agent's
  multiplicity as its row's weight in the estates' kind table (one estate for each twin, REP.1), and every row it
  held passes to it whole by line transfers (PTY.9); its slot is freed and its identity ends. The estate's
  waterfall runs on one twin's balances and pays each amount for every twin. Households hold no
  holdings before Stage 3; one ending with any stops the run until its holdings pass the same way.

Nothing is split or landed: an agent is changed where it is, or ends.

### 7.5 Lines, counts and settlement

A row's count is the agent's multiplicity times its twin's contracts on the line (REP.3), so every line's two sides
hold equal counts of real contracts in either representation. A rule's amount for an agent is its amount for one
twin, rounded by its convention, times the multiplicity (REP.9), so every balance an agent holds is a whole
multiple of its multiplicity and each twin's share is whole.

Settlement reads agents as it reads individuals, through `PayerPositions`: an agent's weight is its multiplicity,
its funds per member are its account's balance over it, and the pooled-flow rule tests each row against one twin's
funds (§6.5). Since every row an agent holds reaches all its twins, no row ever reaches some of them: the rule pays
or fails an agent's rows whole, and never asks for a split. A cleared line's claimants drawn to lose a failed
payer's dues (REP.23) are drawn in whole units the same way (`Losers`), until at least the failed members are
reached. The opening apportions a derived line's other side a twin's contracts at a time, so every agent's rows
there are whole multiples too.

### 7.6 The representation

Two register primitives (`data/shared/REP.toml`, RESOLUTION) set it (REP.40, spec Appendix E 44):

- `REP.multiplicity` — the multiplicity of every agent of a population kind: the factor under twins, one under a
  small world;
- `REP.population_divisor` — what the setup's total population is divided by before the countries are derived: one
  under twins, the factor under a small world.

The opening divides `GEN.population` by the divisor before the split among the countries, so everything derived
from a country's people — the employed, firms, lines, the individuals' rank (`FRM.rank_per_million`) — follows the
smaller world; it then draws one agent for every `REP.multiplicity` households and small firms, each counting that
many on its lines. `phx run --representation twins:K|small:K` sets both before assembly in place of the register's,
for build runs comparing the two; the population's store and the save's manifest record the representation, so a
save loads only under its own, and `phx inject` reads it from the manifest (§11). Twins is the default.

The **player's household** (OBS.4) is drawn at the opening among the country's household agents, each weighted by
its multiplicity, so every real household is as likely. Under twins one twin of the agent drawn is seated as an agent
of its own, of multiplicity one, with the agent's attributes, persons and attachments, and one twin's share of every
row moved to it by line transfers (the opening allows rows to move, §10.4a); the agent keeps one twin fewer. An agent
of one twin, as in a small world, is the player's as it stands.

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

### 7.9 The valve

- **The world runs once** (spec Appendix E 36): there is no weight-one run, and no run at another resolution or seed
  to compare with. The representation is judged by the run's own macro results against real economies' (N3, N4).
- **The valve** (N8.5) is a declared RESOLUTION setting, never an input the world reads from its own timing: it
  changes only between runs, by a recorded change citing a device or measurement report (§14.8). Its settings are
  register primitives (`data/shared/REP.toml`), which the manifest's register hash covers, so a load with a changed
  valve is refused as other data (§11).

---

## 8. Markets, valuation and expectations

- `phx-market` implements each form once (MKT.3–MKT.8): call auctions (with admission hooks); the continuous book
  (arrival by lot, price–time priority, closing auction); the dealer market; posted prices with
  rationing by lot; bilateral quotes as a protocol over messages across days; administered facilities.
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

Estate rows are short-lived individuals, **one per ended party**: an agent's twins, which end together, share one
estate, holding their count on every line and holding it succeeds to (§7.4). How many are open follows Little's
law, the rate of openings times their life: firms' about 800 a day × about 40 days, households' about 1 000 a day ×
about 25 days, personal insolvencies' about 70 a day × about 45 days — about 60 thousand open, at 512 bytes each
(§13.1). Their mean life and the number open are counted per kind.

As built at Stage 0 (plan S0.25e) there is no `sys-est`: estates are the ledger's `phx_ledger::estate` run by the
world's `estates` (`phx-world/src/estates.rs`). An ended household's rows pass to one estate at 3e; from the next
business day, in 7c's block once the day's dues are paid, the ledger settles it (`Books::settle_estate`): its rows that
hold nothing but money leave with their counterparts first; its money pays its debts through the waterfall, its
creditors as one class (the plan's F-096); and while it still holds units it waits to sell them, counted, writing
nothing off and distributing nothing. One that holds none writes off what it owes beyond, pays what is left to the
party of the kind `HEIRLESS_DESTINATION` names (the treasury) in its country, since no heir is drawn before the kinship
lines exist (the plan's F-050), and its rows leave with their counterparts and it ends; one whose payment fails waits,
counted. Dues it
owes in arrears are not claims in its waterfall (F-066). A catastrophe's struck tiles lose, at 3b, their share of every
physical unit the individuals sited there hold, as a transformation naming the event (GEO.8).

Firms end in **default of payment** from S1.03 (FRM.15): a kind is put under its country's insolvency law by an
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
  country, and each country's land is its share of the map, so regions are of like size.
- **Names**: a real name labels the country's institutions and currency and pre-fills its choices; its economy is
  always derived. A generated name comes from the stream `GEN.names`.
- The setup is named by every realism report (GEN.11). A save's manifest does not record it: its register hash is
  over the data files, the countries the setup instantiated among them, so a load with another setup is refused (§11).

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

- **Pass A**: institutions, firms and their drawn sizes are drawn; every household is drawn **complete** from
  counter keys (a fixed counter block per person and attribute), region by region, and stratum counts are kept per
  GEN chunk of about a million households, sparsely.
- **Allocation and balancing** (GEN.4) run on the totals: counterparty sides are derived (§10.2); institutions'
  balance sheets close through ledger operations of reason `Balancing`, each reported.
- **Pass B** redraws every household identically from the same keys and applies the allocations: a household's
  canonical rank within its stratum (its chunk's prefix count plus its place in the chunk) falls in one
  counterparty's apportioned range, which assigns its bank and lenders without a second pass and independently of
  chunking. Employment and tenancy lines record no pairing (REP.23): a household joins the line of the terms it drew,
  and only the line's counterparty side is apportioned. Pass B is a replay of pass A's draws, and places each region's
  agents in bulk before drawing the next, so the scratch is one region's.

**Households are drawn persons first** (GEN.2, REP.26), so the persons' ages are the country's and the households are
what those persons make:
1. **Persons.** A region's people are apportioned by largest remainder, ties by lot, over single ages and sexes in
   proportion to the country's age standard raked to its drawn shares; that pool is every person the region holds.
2. **Families.** While the pool holds a child under the age of majority, one is drawn from it, each child alike; its
   mother is drawn from the pool's women in proportion to each age's women and their chance of a living child of
   that age; her other children at each age with her chance of one, while the pool holds one; the family's type among
   those with children in proportion to their shares at the country's fertility; a partner, when the type holds one,
   from the pool's men in proportion to each age's men and the partner gap's chance at it; an older relative, when
   the type holds one, from the pool's persons of 65 and over. A child whom no woman left in the pool can have
   mothered is raised by another adult of the pool, counted in the report; a child no adult is left to raise stops
   the opening, the distributions being inconsistent (GEN.2).
3. **The rest.** While the pool holds an adult, a household's type is drawn among those without children in
   proportion to their shares, and its persons from the pool as the type says: one person; a woman and a partner at
   the gap; persons who are not relatives.
4. **The type follows the persons**: a person the pool no longer holds is not drawn, so the last households of a
   region hold whom the pool has left, and every person of the pool is in exactly one household.

The elder of a couple heads it, else the mother, else the one person or the first drawn. Each household is formed as
an explicit household, its persons held with their roles (§7.1), each person's health and each adult's education
drawn by its age and sex from the household's own streams, and the region's households are placed one agent for every
`REP.multiplicity` of them (§7.6), so the agents' multiplicities sum to the population (REP.13). The
report gives each country's households by type and persons by age band against the drawn shares.

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
pass is needed. An agent's row on a line counts its multiplicity times its twin's contracts, and its balance is its
twins' total (REP.3, REP.9). A line is opened for each distinct (kind, terms, named counterparty); the counterparty's
side of a named line is one row counting the agents' contracts. Lines whose other side is derived — jobs, tenancies,
defined-benefit pensions — take it once every region is drawn, each line's count apportioned over the eligible
counterparties by their drawn sizes (§10.2), and the unmatched strata and each counterparty's difference from its drawn
size go to the opening report. The balances close through the ledger's opening writes (GEN.4): each row's balance
written against its counterparty's, and a counterparty's books the sum of its rows, its equity the residual, reported.
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
household loan, its share of the households' debt by its income. Neither side of a household's line keeps a holder list
on the households' side. Labour's draw gives an employed adult a job, a person's row on the employment line of its wage
point, whose employers are derived over the firms by headcount; housing's gives a renting household a tenancy on the
line of its rent point, whose landlords are derived over the firms by their plant until the dwelling stock names them.
Both are lines of many holders on both sides, so they are cleared (§6.5). Social protection's draw gives each adult who
has reached its sex's pension age the state pension at its sex's coverage: a person's row on its sex's state pension
line, a flat monthly amount — the replacement rate of the mean wage — whose payer, the treasury, is named. Persons
reaching the pension age during the run join with the claim (S1.11); the defined-benefit schemes lack sources (plan §11,
F-045, F-046). The small firms are drawn with the parties, before the households: each country's firms below the
individuals' rank (REP.2), by the firm-size law cut at the smallest the rank admits, counted by employment size class,
apportioned over the regions by their land and over the banks by the banks' drawn sizes, and placed as agents of the
small-firm kind, one for every `REP.multiplicity` firms, with region, size class and bank as attributes (§7.1). They
hold the firms' deposits and debt the large firms do not, by their employees, on each bank's lines for them, and are
employers on the employment lines by their headcount beside the large firms. The large firms are rows of the kernel's
firm kind table, individuals (REP.2).

Nothing is balanced after drawing: finer attributes are drawn from their own counter keys, so a coarser setting is a
projection of a finer one, and every number of preference types is a discretisation of the same declared distribution
(NUM.4). Drawing the population takes tens of seconds on the phone's cores. As built the opening runs on one thread, and
took about ten minutes on the build machine (the plan's F-038, F-062).

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
  rows may also be opened, and moved whole between holders with their balances — as one twin's contracts move to the
  player's seat (§7.6) — and nothing else moves. Each write is reported with its party, amount, the identity it
  served and its counterparty; once every phase has run, each party's opening **equity** is computed once, as its
  rows' balances signed by side plus its holdings at cost, into the report. At the assembly's end, once the player is
  seated, the ledger forgets the opening's day book whole (`Ledger::opened`): the audit and the accounts read days, and
  the opening is none, so the accounts open on what the opening wrote.
- **Plant** is a real asset per country, counted in its own unit priced in the currency's smallest unit, held at
  cost; its size is the steady state's capital, investment over growth plus depreciation.
- **Accounts** — reserves, the treasury's account, the central bank's claim on the treasury — have no dates and are
  marked spent at the opening; every other opening line falls due on the first date of its schedule after the
  snapshot.

### 10.4b Dated flows before the payer pass *(retired)*

Retired at S0.17, which brought the payer pass: stage 7 no longer pays each due as its own instruction from a pass
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
line, undeclared and uncounted (F-059). The representation family bounds a row by the agent's multiplicity times the
persons its twin holds in the roles its side declares (REP.31), as a household's jobs count its adults; a side declared
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

- A **save** is a directory: a manifest and one file per store of zstd frames of transformed pages. The manifest
  (`save/manifest.rs`) holds the format, the build, the register (a hash of the data files the world was assembled
  over), the seed, the day and its date, the settling length (`settling_years`), whether the read trace was on, the
  representation's multiplicity and divisor (§7.6), each
  store's name, file, bytes compressed and raw and logical hash, and the world hash. It holds no policy hash, setup or
  valve: the register hash covers them where they are data (§7.9, §10.0).
- Saves are written at the moments SET.12 declares — when the player saves, when the app is set aside, and at the
  declared interval (an owner setting, by default every simulated quarter) — and the world pauses while one is written
  (N8.10). As built, the stores are written one after another on one thread.
- **Every save is full** (SET.12, spec Appendix E 22): it writes every store whole, and a restore reads one save.
  Allocator state (free slots, block pools, interners) and the linked call's basis (§8) are
  saved with the stores, and derived indexes are rebuilt on load in canonical order; the world hash covers logical
  content only, so layout never makes two equal worlds differ (§14.3).
- **Retention**: the latest complete save, plus the one being written; the older is deleted only after the new one is
  complete (SET.15), so the peak is two full saves (§13.3).
- **No copies**: a save is loaded only to continue the one run, or, on the build machine, apart by `phx inject` to be
  audited and discarded, never run on (N1).
- **Injection** (N1): the build run takes one more save, at the close of the 30th day after settling, into its own
  directory, and after its last day hands it to `phx inject` in a process of its own, so the run's memory stays the
  world's alone. Each family's injection goes into a fresh load of it; the audit then closes over the longest rolling
  cycle from the save's next day, no day stepped, and the families that found something are recorded with the run's
  measures, where LC-0-10 reads them. A world not read from a save refuses an injection. Legs a family injects go to
  the audit's own sink, as if they had settled.
- **Stores** of a save, each one file, in the order the world hash reads them:
  - `world`: the day, the fails waiting for the contract process, the player's queue, bindings, the closed fact;
  - `books`: the kind tables, the population's agent tables, the directory and the ledger;
  - `population`: the agenda, and each kind's members as the events that began and ended them count them;
  - `markets`;
  - `accounts`;
  - `records`;
  - `events`;
  - `geo`: the map as generated and the kernel tables' columns.

  Beside them, outside the world hash, `run` holds the run's metrics, findings, settlements, the opening's report, the
  read trace's log and the handlers whose first chunk it traced (`traced_first`).
  The map and the books are saved rather than regenerated, since the map's generation and the opening each take
  longer than a load's 3 s.
- **What the build supplies on load**: everything declared in code or data — the register, the calendar's rules,
  streams, handlers, rules, the audit's families, and the books' declarations (line kinds and reasons). The books'
  declarations are the opening's first phase, `DECLARATIONS`, which a load runs alone on empty books before reading
  the `books` store; the save records their names, and a load refuses a save whose names differ.
- **A load verifies** the manifest's format, build, register and seed before it reads a store, and after it has read
  them all, that the world it rebuilt hashes to the manifest's world hash; either refusal names its reason (N5).
- **What a load rebuilds**, in canonical order, never trusting a saved copy:
  - instruments' and lines' holder lists, from holdings and rows;
  - the due wheel, from the lines' next due days;
  - the terms interner's index, from its entries;
  - the tape's last print per market;
  - the accounts' claims per party;
  - the calendar's window;
  - the audit's cursors, from the stores' lengths.

  Allocator state is saved as it stands: the slot allocators, the arenas' words with their dead words, and the
  interner's free identities and reference counts.
- **The world hash** covers every carried state a later day can read: with the directory, kind tables, instruments,
  lines, markets, accounts, records, events and kernel tables, also the run heads, the terms and their reference
  counts, liens, covers, commitments, arrears, the fails waiting and the player's queue. Layout, holder lists and
  other indexes stay out.

---

## 12. Observation and the player

- **Views** are built at 10e on a turn's last day from records and the state at its close, into fixed-bin histograms,
  and swapped in behind an `Arc`; tables cross the FFI in pages; `phx-obs` writes nothing
  (Law 17). As built, the host (`phx-cli`'s run, `phx-ffi`'s bench) drives the observer after each turn: its
  `Recorder` reads each day the turn closed from the run's own records — the day's work on the agents, the day's
  settlement and the day's events — into the **macro reads** `data/observer/READS.toml` declares, each a named
  measure (`agents.persons`, `settlement.gross`, `events.<kind>` summing the sizes of the day's events of a declared
  kind, …); its `Views` build, at the turn's close, each read's latest value and the declared histograms of a kind's
  agents, of an attribute's values counted by their multiplicities, over fixed lower edges with the values
  below the first counted apart. Each histogram is an opening distribution (GEN.8): the host keeps the opening's view
  and reports each histogram's distance from it (half the summed differences of the bins' shares) at settling's end
  and at the run's end. The observer is a `phx_world::Observer` passed to `run_turn_observed`: after each day it
  takes the reads through the inspector, inside the turn's time, and the world is the same
  world with it or without it. PC-20 refuses any `&mut` to the world's stores, and any naming of `World`, in
  `phx-obs`. On the phone the observer follows the measured turns, not settling, and the views are built twice, at
  settling's end and at the run's end, not at each turn's close (F-064).
- **Two builds**: the participant's reads only through `ParticipantScope`, its party's scoped read; the inspector's
  compiles only with the `inspector` feature, which the participant build refuses. As built at Stage 0 there is one
  build: no crate declares an `inspector` feature, `Inspector` and the `Recorder` are always compiled, and
  `ParticipantScope` is not built. Every shown number is a
  `Shown<T>`, built from a record entry, a read at the close, a published statistic, or a fixed-bin aggregate of
  reads; pages are generated from the interface crates' view schemas, and a decision
  page from its point's input view and intent.
- **The representation**: the inspector's pages show REP.15's report — the representation, its factor and its
  counts — beside every distributional number (N5).
- **An agent's page** (OBS.8) shows the agent itself: its attributes, persons, positions and rows, and the recorded
  events that name it; under twins it says the agent stands for _k_ identical households or firms. Nothing is drawn
  to show it.
- **The player** is a party of multiplicity one (REP.1, OBS.4) whose decider fact names the player. A queued intent is
  a **wake**: at 1c of the first day its decision point runs, it gives the player an occasion for that decision (REP.21)
  and is decided there; until then it stays queued. On days the player has queued nothing for a scheduled decision, the
  rule decides only if the player's settings delegate (OBS.4). It appears in every audit family. As built, the setup
  states the player's country and whether it delegates; at the end of the assembly the world draws a household of that
  country from the stream `GEN.player`, each household equally likely, as an agent of multiplicity one (§7.1), and seats
  it in `PlayerQueue`, saved and hashed with the world, which names the player's decider (`Decider::Player`) and refuses
  an intent queued for any other party.
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
representation (spec Appendix E 31). The **design point** is 0.7 million household agents (a multiplicity of about
170 under twins), 0.25 million firm and business agents, and 2,000 zones. The first measurements (§14.6) replace every
number here; the factor and zones are RESOLUTION and are set where both budgets hold (N8.5) with at least 10%
headroom, this document's margin for the estimates' error.

### 13.1 Memory (4.5 GB resident, N8.4), at the worst day's peak, through Stage 2

| Store | Count | Bytes each | Budget |
| --- | --- | --- | --- |
| Household agents: attributes, positions (with each deposit row's), rates, review exposures and attention rates per kind, own outlooks, arena references, the due-day run's head (itemised in the plan, S0.21, S1.12; Stage 2's review kinds S2.05, S2.06) | 0.7 M | 576 | 403 MB |
| Household persons and attachments, a packed word each | 0.7 M × 43 words | 8 | 240 MB |
| Relationship rows (40 per household agent, 12 per firm agent, 2 M of individuals; Stage 0's pensions in payment, 0.35 M state pension rows at 16 bytes and 0.5 M DB pensioner rows at 24; Stage 2's invoice rows, 10 per firm agent and 0.5 M of individuals, one per (holder, market, terms, statement period)) | 36.85 M | ≈ 23 average | 849 MB |
| Line holder lists, with block slack (none on the pensions' retail side, §4.5) | 36 M | 6 | 216 MB |
| Holdings and instruments' holder lists | 5.25 M | 28 | 147 MB |
| Firm and business agents: the record (itemised in the plan, S1.03) | 0.25 M | 500 | 125 MB |
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
| Day buffers at the worst day (payee reduction streamed shard by shard; intents; sort scratch), by their room; measured at 889 MB on Stage 0's twins payday, over this line (the plan's F-082) | — | — | 600 MB |
| Arena slack and page tails (15% of variable-length stores) | — | — | 214 MB |
| Save buffers (the renumbering slice retired with cells) | — | — | 94 MB |
| Views (tracers retired with cells) | — | — | 60 MB |
| Android process baseline | — | — | 250 MB |
| **Total** | | | **4 244 MB** |

Through Stage 1 the design point peaks at about 3.98 GB against 4.5 GB — the run head in every household record from
Stage 0 adds 6 MB, Stage 0's pensions in payment 20 MB with slack, the persons and attachments as laid out 30 MB more
than first budgeted, and the landing index (as measured) and group aggregates retired with cells 134 MB less — so 12%
headroom, if the day buffers are brought under their line (F-082). Stage 2 adds about 251 MB — invoice rows with their
holder-list entries and slack 104 MB, filed accounts 48 MB, household agents 45 MB, estates 31 MB, listings 13 MB, loan
lines' stage totals 10 MB — to about **4.23 GB**: 6% headroom, short of the required 10% (a peak of at most 4 050 MB),
as the plan's F-005 records. Stage 3 adds about 240 MB — institutions' positions and their lots 104 MB, households'
holding rows 30 MB, individuals' deviations from their methods' outlooks 19 MB, household attributes with participation
18 MB, registered outlooks 12 MB, household agents 11 MB (592 bytes), money-market lines 10 MB, records and instruments
10 MB, slack 26 MB — to about **4.47 GB**: 0.7% headroom, as the plan's F-003 records. Stage 4 adds about 387 MB (the
plan's S4.07): relationship rows 138 MB — policies 4.9 M and firms' 0.5 M at 16 bytes, DB active and deferred rights 1.3
M at 24, derivatives 1 M at 16 with no `amount`, claim and compensation lines — household attachments 42 MB (policy
attachments with their renewal bands, and the pension scheme joint with employment), DC pots with their `pending` word
38 MB, lines 35 MB, interned terms 25 MB, household agents 22 MB (624 bytes), kind tables 15 MB, records and valuations
30 MB, holder lists 8 MB (derivative lines and institutional sides only, §4.5), slack 34 MB — to about **4.86 GB**: 8%
over the budget itself, as the plan's F-004 records. Stage 5 adds about 116 MB (the plan's Stage 5 ledger, after its
reviews' remedies): relationship rows 42 MB — benefit claimants 0.65 M at 16 bytes, earnings-related state-pension
rights 0.6 M at 24, tax payables and instalments 0.35 M at 24, payroll payables' balances per base, foreign-currency
deposits and nostros at 38 — lines and terms 15 MB, attachments 13 MB (intentions during a campaign, waits and claims),
household agents 11 MB (640 bytes), foreign holdings 9 MB (at 32 and 24 bytes), records 8 MB, the `vote` review's side
column during a campaign 5 MB, messages 2 MB, kind tables 1 MB, slack 10 MB — to about **4.97 GB**: 10% over the budget
itself, as the plan's F-006 records. Stage 6 adds (the plan's Stage 6 ledger): firm agents' known ways, about 50 k at
about 1 KB, 50 MB; household persons' records 28 MB (the education record, schooling, the search band, participation and
retirement); household agents 22 MB (672 bytes); relationship rows 6 MB (kin rows, licences); firm agents 1 MB (504
bytes, past this table's 500-byte line by 4) and their cumulative-output lists 7 MB; ways, known-way sets and patents 7
MB; views 10 MB; imitation pools 4 MB; receipts 1 MB; slack 11 MB — about 147 MB, to about **5.12 GB**: 14% over the
budget itself, as the plan's F-007 records. How rows fall as the factor rises is measured (§14.6).

### 13.2 Time (1 s median, 2 s worst, N8.2)

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
| The same parts at F-001's measured 9.9 µs (the risk case, outside the totals; retired with cells) | 58 k | 9.9 µs | 191 ms | 20 ms | 285 ms |
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
| The same at F-001's measured 9.9 µs a part (the risk case, outside the totals) | 25 k | 9.9 µs | 83 ms | — | 83 ms |
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
finding (F-001); parts are retired with cells, and what an agent's change in place costs is measured (§14.6). The Stage
1 review added the physical flows' realisations and the publication-day wake. Due-day runs for every dated row kind
(§6.5) take the ordinary day's settlement from 120 ms (30 M rows read at 10 ns and 2 M payments at 30 ns: 360 core-ms)
to about 48 ms ((1.1 M × 2 + 6.2 M × 10 + 4 M × 5 + 2 M × 30) ns ≈ 144 core-ms), with a unit for the head's maintenance;
on a heavy payday almost every holder has a due, so they save little there, and Stage 0's pensions in payment add about
7 ms. At these estimates the **median fits with 7.6%**, short of the required 10%; **heavy Mondays and the longest
closed runs miss**. Stage 2 adds about 86 ms to a business day, 13 ms to a non-business day and 126 ms to a heavy day,
so through Stage 2 the **median misses the budget itself by 1%**, and at the measured part cost Stage 2's parts alone
would have added about 143 ms more to it (the plan's F-005). Stage 3 adds about 53 ms to a business day, 1 ms to a
non-business day and 86 ms to a heavy day, with its representation choices already taken (§18 item 24): through Stage 3
the median is about **1 063 ms, 6% over the budget**, and a heavy Monday about 2 303 ms (the plan's F-003). Stage 4 adds
about 102 ms to a business day, 3 ms to a non-business day and 146 ms to a heavy day, with its own choices taken (§18
item 25): through Stage 4 the median is about **1 165 ms, 16.5% over the budget**, and a heavy Monday about 2 455 ms
(the plan's F-004). Stage 5 adds about 63 ms to a business day, 4 ms to a non-business day and 99 ms to a heavy day
after its reviews' remedies (77, 4 and 133 before them), with its choices taken (§18 item 26): through Stage 5 the
median is about **1 228 ms, 23% over the budget**, a heavy Monday about 2 562 ms, and an election's eve adds about 60 ms
in the largest country (the plan's F-006). Stage 6 adds about 74 ms to a business day, 14 ms to a non-business day and
86 ms to a heavy day after its reviews' remedies, with every known way kept (TEC.4) and its choices taken (§18 item 27):
through Stage 6, the whole world, the median is about **1 302 ms, 30% over the budget**, a Monday after a weekend about
2 034 ms, 2% over, and a heavy Monday about 2 676 ms (the plan's F-007). For the worst turn, three closed days before a
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

### 14.3 Determinism by construction

The world is never run twice to prove it deterministic. Determinism is carried by construction and checked at compile
and logic level: chunking, gathers and reductions fixed by index, whatever the worker count (§6.2, the plan's S0.07
tests); canonical ids, whatever the registration order (§6.2); a stream's key derived from its own name alone (CHN.1);
observers, the audit and the realism recorder reaching the world only through `&self` accessors (Law 17, PC-20,
PC-85). Every state that carries across days and can change an outcome is saved or canonical: a solver's warm start
is saved (§8), derived indexes are rebuilt in canonical order (§11), and the resolution valve is a setting of the
register, which the manifest's hash covers, changed only between runs or at a save boundary, never from the run's own
timing (§7.9). The build run checks that each save, decoded store by store without building a second world, hashes
to the world hash of the close it was written at (SET.15); the check is not meant to run on the phone, and as built it
runs only in `phx run` (F-060).

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
   random words and of the agents' multiplicities, not the real families; the agenda, redraws and each visit's gathers
   are reads and writes of random words; there are no views, members leaving, estates or 9b posting (F-064). The
   device report measures an agent's next hit drawn and a hit's outcome applied on the fastest core.

Stage 1's gate adds retail and labour: draws, sellers in reach, occasion evaluations and choices by decision,
vacancies visible and labour rounds, surprise wakes, physical realisations, outlook methods in use, and agents per
stance. From these, §13 is
rewritten with measured numbers for every stage — the measured unit costs times each later stage's ledger counts — so
each gate reports the whole world's projection, not only its own stage's, beside the full-load bench that decides it.
If they do not fit, the remedies are, in order (N8.7): how the world is represented and traversed; then the play
resolution — the factor and the zones. If none suffices, that is a finding, and the owner decides; no mechanism is
weakened. Before Stage 4's steps are built, `phx measure` also measures policy rows per (agent, cover) on GEN's draws
(the plan's S4.03) and the rows per agent of defined-benefit rights and DC pots (S4.04).

### 14.7 Continuous integration

| Workflow | When | What |
| --- | --- | --- |
| `ci.yml` | every push | format; clippy with disallowed lists; `cargo test` in the debug and the release profile; `phx-check`; `miri` over `phx-store`'s tests on the pinned nightly; `public-api`, the kernel and interface crates' API snapshots; the workspace built for Android; `android-app`, the engine built for the phone with the `device` profile, its Kotlin bindings and the bench flavour's APK, uploaded as an artifact (§18 item 33); kernel micro-benchmark instruction counts under valgrind, whose ratchets fail CI. The world is not run |

**The build run** (`tools/build-run.sh`, on the build machine — the development VM, 4 cores and 15 GB — after each
step's build, from S0.11, the first step with a world; S0.01 to S0.10 have none): a `--release` build (thin LTO; only
the `device` profile is fat, §17); the world at the play resolution (§10.5), with `read-trace` on, the audit and every
live check — an ordinary step's for 120 days from day zero, a stage gate's (`--gate`) settled at the owner's length and
run two years after it; the engine's counter ratchets, whose moves fail the build run, not CI — as built, the barriers
per empty day, the map's bytes and seven stage-7 counters (§16.8); peak memory; wall time, build and run apart. It is
the only run of the world off the phone, and it uses no other resolution or seed. It is **clean** when the audit reports
nothing, every failing live check is recorded as a finding in the plan's §11, and no ratchet moves the wrong way. A
step's last commit adds the report of the build run on the commit before it, as `perf/build-run/<that commit>.json`, and
marks the step `done`; these reports are append-only and, being the code's numbers and never the world's, need no owner
review. The play resolution needs about 5 GB. At 300 million persons the opening takes about ten minutes and each
business day about 13 s on the build machine's one thread (F-056, F-062).

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
   the world's day records (`CellDay`), which the live checks read, neither reported nor
   ratcheted; the other counters are not built (F-060).
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

A rule changes only with its reason recorded in §18.

---

## 17. Build and target

- Release: `lto = "thin"`, `codegen-units = 1`, `panic = "abort"` with a hook that writes the violation report,
  overflow checks on; the build run builds it. The `device` profile, which CI builds the phone's library with,
  inherits it with `lto = "fat"`.
- Phone target features: `+lse,+rcpc,+dotprod,+fp16`; NEON by auto-vectorisation and in `phx-rand`'s samplers;
  64-byte aligned, padded columns; software prefetch on holder-list and index gathers.
- Pool sized to fast and medium cores; performance-hint sessions per turn. As built at Stage 0 the world runs on one
  thread and no performance-hint session is opened (§2, F-056).

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
    16% through Stage 6 (§13; findings F-001 to F-007 of the plan).
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
      of harm to third parties are drawn from the (zone, class) index and the zone's pieces (§7.3, §7.10);
    - resolution of insurers and clearing houses is `sys-sup`'s on §9.2's timetable: a closed payer's legs are
      pending, a closed many-party payer's holders are paired once for the whole resolution, and a house recovers by
      haircutting its pending legs before its service is transferred or wound down (§6.5, §9.1, §9.2);
    - pensions in payment execute from Stage 0 (spec Part O), and the statistics agency publishes a period life table
      that insurers, households and actuaries read (§14.6).

    Through Stage 4 the design point misses the median by 16.5% and memory by 10% (§13; the plan's F-004); the
    remedies are N8.7's, representation and traversal first, measured at each gate before the play resolution is
    touched.

26. **Stage 5's decisions**:
    - taxes are levies per member where their bases arise; the levies of one flow share one search of their fused
      kinks; value-added tax on a sale on terms is computed per invoice row at its statement and on a cash sale rides
      the sale's instruction; property tax is a holding levy driven from the (zone, class) index, joining the
      holder's existing leg; duty and import tax at a border are a demand customs issues, not a levy; returns are
      day-local (§4.2, §4.3, §7.10);
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

    Through Stage 5 the design point misses the median by 23%, a heavy Monday by 28% and memory by 13% (§13; the
    plan's F-006); the remedies are N8.7's, representation and traversal first, then the play resolution, a valve set
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
    16%, and two full saves keep about 1% of storage headroom (§13; the plan's F-007); the remedies are
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
    - an agent's multiplicity is the count of identical twins it stands for; a build holds its world as **twins**, the
      full population as agents of multiplicity _k_, or a **small world**, one _k_-th of it as agents of multiplicity
      one, set by `REP.multiplicity` and `REP.population_divisor`, twins by default (§7.6);
    - lines still record many-party relationships as counts, an agent's count its multiplicity times its twin's
      contracts (§7.5); individuals are the institutions and the parties ranked at the opening, and the player's household is an
      agent of multiplicity one;
    - the factor is the representation's one valve (N8.5, §7.9); §13 keeps its estimates, with the work only cells
      needed marked retired, until the first measurements replace them.

---

## 19. Coverage

Generated by `phx-check coverage` from the clause map. Status: planned, building, done.

| Spec | System | Crate | First stage | Complete at stage | Status |
| --- | --- | --- | --- | --- | --- |
| A1 | TIME | `phx-id`, `phx-core`, `phx-world` | 0 | 0 | done |
| A2 | PTY | `phx-core`, `phx-pop` | 0 | 3 | building |
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
| D2 | HH | `sys-hh` | 1 | 6 | planned |
| E1 | TEC | `sys-tec` | 1 | 6 | building |
| E2 | FRM | `sys-frm` | 0 | 6 | building |
| E3 | CAP | `sys-cap` | 1 | 5 | planned |
| F1 | GDS | `sys-gds` | 1 | 2 | planned |
| F2 | SRV | `sys-srv` | 1 | 1 | planned |
| F3 | FRT | `sys-frt` | 1 | 1 | planned |
| F4 | LAB | `sys-lab` | 0 | 6 | building |
| F5 | HSG | `sys-hsg` | 0 | 2 | building |
| F6 | TCR | `sys-tcr` | 2 | 2 | planned |
| F7 | ENE | `sys-ene` | 2 | 4 | planned |
| G1 | BNK | `sys-bnk` | 0 | 4 | building |
| G2 | BFL | `sys-bfl` | 2 | 3 | planned |
| G3 | BCP | `sys-bcp` | 2 | 3 | planned |
| G4 | SEC | `sys-sec` | 4 | 4 | planned |
| H1 | MMK | `sys-mmk` | 2 | 4 | planned |
| H2 | SOV | `sys-sov` | 1 | 5 | planned |
| H3 | CRD | `sys-crd` | 3 | 3 | planned |
| H4 | EQY | `sys-eqy` | 3 | 4 | planned |
| H5 | MNA | `sys-mna` | 4 | 4 | planned |
| H6 | FND | `sys-fnd` | 3 | 4 | planned |
| H7 | DLR | `sys-dlr` | 3 | 3 | planned |
| H8 | IDX | `sys-idx` | 1 | 3 | planned |
| H9 | RAT | `sys-rat` | 2 | 4 | planned |
| I1 | DRV | `sys-drv` | 4 | 4 | planned |
| I2 | DRX | `sys-drx` | 4 | 5 | planned |
| I3 | INS | `sys-ins` | 4 | 4 | planned |
| I4 | PEN | `sys-pen` (no crate yet: Stage 0's state pension is `sys-soc`'s; F-045) | 0 | 5 | building |
| J1 | TRS | `sys-trs` | 1 | 5 | planned |
| J2 | TAX | `sys-tax` (no crate yet) | 0 | 5 | building |
| J3 | SOC | `sys-soc` | 0 | 5 | building |
| J4 | CB | `sys-cb` (from Stage 0: the central banks and treasuries as parties) | 1 | 5 | planned |
| J5 | SUP | `sys-sup` | 2 | 4 | planned |
| J6 | POL | `sys-pol` | 5 | 5 | planned |
| K1 | FX | `sys-fx` | 5 | 5 | planned |
| K2 | XB | `sys-xb` | 5 | 5 | planned |
| M1 | OBS | `phx-core`, `phx-obs`, `android/` | 0 | 6 | building |
| M2 | STA | `sys-sta` | 1 | 5 | planned |
| L3 | estates | `phx-ledger` and `phx-world` at Stage 0; `sys-est` from Stage 1 (S1.03) | 0 | 2 | building |
| L1, L2, L4–L12 | transmission chains | read from the run by `phx chains` (N4) | 2 | 7 | planned |
| N1 | audit | `phx-audit` and every system's families, run in every build run | 0 | 6 | building |
| N2–N8 | measurement | `phx-cli`, `phx-world` metrics, `android/` bench | 0 | 7 | building |
