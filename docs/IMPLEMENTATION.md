# Project Phoenix — Implementation plan

This file sets out how to build the world in `docs/spec/PROJECT_PHOENIX.md`, step by step, on the architecture in
`docs/ARCHITECTURE.md`:
- the spec says **what**;
- the architecture says **how**, top down;
- this file says exactly what to build, in what order, how to know it is right, and what is not allowed on the way.

It is written so that a builder who has never seen the project can take the next step and finish it without inventing
anything.

Where this file and the architecture disagree, the architecture is fixed first, in its own change, and this file
follows. Where either disagrees with the spec, the spec wins.

---

## 0. How to use this file

### 0.1 The rules of the work

1. **One step at a time, in order.**
   - Take the first step whose status is `planned` and set it to `building`.
   - Finish it — every item of its **Done when** — before touching the next.
   - Never have two steps in `building`.
2. **Read before writing.**
   - Before the first line of a step's code, read the spec clauses the step lists and the architecture sections it
     lists.
   - Read every earlier step it depends on, as built.
   - Whatever these do not settle is settled by the spec's laws. If the laws do not settle it, it is an owner
     decision (§12): ask, and stop that step until it is answered.
3. **Build exactly what the step says.**
   - Its files, types, layouts, algorithms and declarations are the design.
   - A better idea is recorded as a finding (§0.5). If accepted, it becomes a change to this file in its own commit,
     before any code follows it.
   - Code never runs ahead of this file.
4. **Never weaken an earlier step.**
   - A later step extends a kernel crate only through the extension points an earlier step declared.
   - A change to an earlier step's public surface is itself a step, inserted at its dependency position (§0.4).
5. **The live world is the test.**
   - From S0.11 on, every step adds live checks (§0.3) and runs the live world with every earlier check.
   - A step is not done while any live check fails, whether its own or an earlier one, unless the failure is recorded
     as a finding naming the later step that will fix it.
6. **Every step ends with two independent reviews** of its diff, by reviewers who did not write it (in practice, two
   fresh agents with the prompts of §0.7).
   - One reads the diff against the spec clauses and this step's text.
   - The other reads it against the architecture, the budget and this step's **Not allowed**.
   - Every finding is either fixed in the step, or recorded in §11 with the step that will fix it.
   - Only then is the step `done`.
7. **Commits.** A step has one or more commits, each saying what and why, with the step identifier as its first word
   (`S0.05: …`). The step's last commit:
   - sets its status to `done`;
   - regenerates the coverage table (`phx-check coverage --write`);
   - records the step's measured numbers in `perf/`.

### 0.2 The shape of a step

Every step has these sections, in this order; `phx-check docs` refuses a step missing one (PC-09).

| Section | What it says |
| --- | --- |
| **Status** | `planned`, `building`, `done` or `retired` |
| **Clauses** | the spec clauses the step completes, each with its type; clauses it starts but a later step completes, marked *(part)* |
| **Architecture** | the architecture sections it implements |
| **Depends on** | earlier steps |
| **Goal** | what exists at the end, in two or three sentences |
| **Files** | every file created or changed, with its purpose |
| **Design** | types and layouts, with field sizes; algorithms, precise enough to code from; declarations; handlers with their sub-steps, reads and writes |
| **Unit tests** | logic-level tests, by name and what each asserts |
| **Live checks** | `LC-` identifiers and what each reads and asserts on the running world |
| **Budget** | bytes per row and per store; cost per unit of work; the counters the ratchets hold |
| **Guards** | the `phx-check` rules, clippy lists and assembly refusals the step adds |
| **Not allowed** | the shortcuts this step is most tempted by, named so a reviewer can look for them |
| **Done when** | the checklist; every item something a reviewer can verify |

### 0.3 Live checks

A live check:
- is code in `crates/apps/phx-cli/src/checks/`;
- is registered with a permanent identifier `LC-<stage>-<nn>`;
- reads a live run's records, metrics and state through the inspector's read-only surface;
- returns pass, fail (with the facts that failed) or not applicable.

Every CI live run runs every live check, forever. A check that stops applying is marked `retired` with its reason;
its identifier is never reused, and `phx-check` enforces both.

A live check never repairs, never writes, and never sets up the world it reads (§2.10).

### 0.4 Changing this plan

- Step identifiers (`S<stage>.<nn>`) are permanent.
  - A new step is **inserted at its dependency position**, with the next free number of its stage.
  - The order of steps in the file is the build order.
- A change to the design of a `done` step is a new step that says what it changes and why.
- A step is never deleted. A step that is no longer needed is `retired` with its reason.

### 0.5 Findings

Every finding is recorded in §11:
- a failing live check;
- a number that looks wrong;
- a reviewer's finding not fixed in its step;
- a measurement over budget.

Each record has:
- an identifier (`F-<nnn>`);
- the step and day found;
- what was measured, and where;
- the mechanism suspected;
- the step that will address it.

A finding is closed only by a step. Nothing is tuned to make a finding go away (spec N7, GEN.11).

### 0.6 Owner decisions

The owner's decisions are those the spec reserves (Appendix E, N8, GEN.6). They are recorded in §12 with the answer
and its date. A step that needs one that is not yet recorded stops and asks.

### 0.7 The review prompts

**Reviewer A (spec and step)**

> Read `CLAUDE.md`, the step's section in `docs/IMPLEMENTATION.md`, the spec clauses it lists and the laws
> (spec Part I). Then read the diff (`git diff <base>..HEAD`).
>
> For each clause, say where the diff carries it and whether it carries it exactly. Look for:
> - an outcome written as a rule;
> - a default standing in for a missing value;
> - a bound;
> - a kind branch;
> - a second writer or a second representation;
> - a read of the future or of another party's private state;
> - a draw outside a named stream;
> - a one-sided flow.
>
> Report blockers, serious and minor findings, each with file and line, the clause it breaks and a one-line fix.
> Change nothing.

**Reviewer B (architecture, budget and shortcuts)**

> Read `CLAUDE.md`, the step's section in `docs/IMPLEMENTATION.md`, the architecture sections it lists and its
> **Not allowed**. Then read the diff.
>
> Look for:
> - departures from the step's types, layouts and algorithms;
> - layering and channel breaches;
> - hidden full sweeps;
> - allocation on hot paths;
> - unmeasured costs;
> - counters not ratcheted;
> - tests that build a world;
> - live checks that set up what they read;
> - comments that reference documents or history.
>
> Run the step's budget measurements yourself where they can be run. Report as Reviewer A does. Change nothing.

---

## 1. The build at a glance

Stages follow spec Part O. Each stage ends with its gate step. The stage's exit, the budget (N8) and — from Stage
1 — the comparison with the reference run (N8.5) are all judged on the settled live world.

| Stage | Steps |
| --- | --- |
| **0 Foundations** | S0.01 workspace, toolchain, CI and `phx-check` · S0.02 `phx-macros` · S0.03 `phx-num` · S0.04 `phx-rand` · S0.05 `phx-id` · S0.06 `phx-store` · S0.07 `phx-exec` · S0.08 `phx-core` I: calendar, conventions, schedules, the agenda · S0.09 `phx-core` II: the register, policy values, kinds, facts, the directory, findings · S0.10 `phx-core` III: streams, hazards, messages, decision points, rule handles, events, records · S0.11 `phx-world` I and the first live world · S0.12 `phx-audit` · S0.13 `phx-geo` and the map · S0.14 `phx-ledger` I: instruments, holdings, lines, rows, the contract algebra · S0.15 `phx-ledger` II: money, accounts, instructions, settlement · S0.16 GEN I and the institutions · S0.17 `phx-ledger` III: batches, the payer pass, the fixed point, levies, standing and pooled flows, transfers, the waterfall · S0.18 `phx-market` · S0.19 `phx-acct` · S0.20 persistence and the save guard · S0.21 `phx-pop` I: tables, keys, steps, positions, profiles · S0.22 `phx-pop` II: screening, reviews and occasions · S0.23 `phx-pop` III: splits, parts, landing, re-keying, the seller spread · S0.24 `phx-pop` IV: tolerance control, promotion, renumbering, the reference mode · S0.25 GEN II and the population: households and small firms, the opening lines paying, `sys-dem`, `sys-est` · S0.26 `phx-obs`, `phx-ffi`, the Android bench, the measurement programme and the Stage 0 gate |
| **1 The circular flow** | S1.01 `phx-val` · S1.02 `sys-tec` · S1.03 `sys-frm` · S1.04 `sys-cap` · S1.05 `sys-gds` · S1.06 `sys-srv` · S1.07 `sys-frt` · S1.08 `sys-lab` · S1.09 `sys-bnk` · S1.10 `sys-cb` · S1.11 `sys-trs`, `sys-tax`, `sys-soc`, `sys-sov` · S1.12 `sys-hh` · S1.13 `sys-dem` births · S1.14 `sys-idx` and `sys-sta` · S1.15 GEN III · S1.16 the Stage 1 gate |
| **2 Credit and failure** | S2.01 losses and provisions · S2.02 `sys-tcr` · S2.03 the firm lifecycle · S2.04 estates and inheritance in kind · S2.05 `sys-hsg` · S2.06 `sys-bfl` · S2.07 `sys-bcp` · S2.08 `sys-sup` · S2.09 `sys-ene` · S2.10 the credit bureau and filed accounts · S2.11 personal insolvency · S2.12 the Stage 2 gate |
| **3 Money and capital markets** | S3.01 `sys-mmk` · S3.02 `sys-cb` in full · S3.03 `sys-trs` and `sys-sov` in full · S3.04 `sys-crd` · S3.05 `sys-eqy` · S3.06 `sys-dlr` · S3.07 `sys-fnd` · S3.08 non-bank lenders · S3.09 `sys-idx` in full · S3.10 `sys-rat` · S3.11 the Stage 3 gate |
| **4 Risk transfer** | S4.01 `sys-drv` · S4.02 `sys-drx` · S4.03 `sys-ins` · S4.04 `sys-pen` · S4.05 `sys-sec` · S4.06 `sys-mna` · S4.07 the Stage 4 gate |
| **5 The full state and the open world** | S5.01 `sys-tax` in full · S5.02 `sys-soc` in full · S5.03 `sys-pol` · S5.04 `sys-fx` · S5.05 `sys-xb` and cross-border freight · S5.06 the Stage 5 gate |
| **6 Growth and the full population** | S6.01 `sys-tec` in full · S6.02 `sys-dem` in full · S6.03 `sys-hh` in full · S6.04 `phx-obs` and the app in full · S6.05 the Stage 6 gate |
| **7 Realism** | S7.01 the stylised facts · S7.02 the causal chains · S7.03 resolution and seeds · S7.04 calibration · S7.05 the realism gate |

§13 maps every spec clause to the step that completes it.

---

## 2. Conventions every step follows

These are decisions taken once, so that fifty systems written over years read as one codebase. A step may not depart
from them. A convention changes only by a change to this section, in its own commit, with its reason.

### 2.1 Repository layout

```text
crates/foundation/{phx-num,phx-rand,phx-id,phx-macros}/
crates/kernel/{phx-store,phx-exec,phx-core,phx-geo,phx-ledger,phx-pop,phx-market,phx-acct,phx-val,phx-audit}/
crates/interfaces/if-*/        crates/systems/sys-*/
crates/assembly/{phx-world,phx-obs}/        crates/apps/{phx-cli,phx-ffi,phx-check}/
android/                        the Compose app and its bench flavour
data/world.toml                 world settings: epoch, countries, currencies, settling length, save interval
data/shared/<SYS>.toml          primitives common to all countries
data/<country>/<SYS>.toml       primitives of one country
data/<country>/gen/<SYS>.toml   that system's opening distributions for one country
perf/ratchets.toml              counter ratchets (§2.11)
perf/device/                    device reports, committed by the owner
perf/measure/                   `phx measure` reports, one per step that changes a budget line
docs/                           the spec, the architecture, this file
```

### 2.2 Inside a crate

- `src/lib.rs` holds module declarations and re-exports only.
- `src/consts.rs` holds engineering constants (chunk sizes, arena thresholds, sampler switch points), one `pub const`
  each with a comment saying why its value is what it is. No other file has a numeric literal other than 0, 1, −1 or
  2 (§2.7).
- **System crates** have a fixed layout:

  | File | Contents |
  | --- | --- |
  | `src/lib.rs` | the zero-sized `System` type and its `declare` and `handlers` |
  | `src/decl.rs` | every declaration, grouped by kind (facts, lines, messages, levies, hazards, decision points, primitives, audits, metrics) |
  | `src/handlers/<substep>_<name>.rs` | one handler each, named by sub-step (`5c_hire.rs`) |
  | `src/rules/<decision>.rs` | the pure decision functions, one per decision point, with their evaluation forms |
  | `src/audit.rs` | the system's audit families |
  | `src/gen.rs` | its opening contributions |
  | `src/metrics.rs` | its measures |

- **Interface crates** have one module per concept (`terms.rs`, `facts.rs`, `messages.rs`, `decisions.rs`, `rules.rs`,
  `views.rs`) and no function bodies beyond constructors and field accessors.

### 2.3 Names

| Thing | Form | Example |
| --- | --- | --- |
| System code | the spec's code | `LAB` |
| Primitive | `<SYS>.<snake_case>` | `DEM.mortality_table` |
| Fact | `<SYS>.<snake_case>` | `LAB.employment_status` |
| Stream | `<SYS>.<process>` | `DEM.mortality` |
| Metric | `<SYS>.<snake_case>` | `REP.parts_per_day` |
| Audit family | `<SYS>.<family>` | `MON.issuer_balance` |
| Live check | `LC-<stage>-<nn>` | `LC-0-07` |
| `phx-check` rule | `PC-<nn>` | `PC-06` |
| Finding | `F-<nnn>` | `F-012` |
| Counter | `<crate>.<snake_case>` | `phx_pop.parts_joined` |

Types are nouns in the spec's words (`EmploymentLine`, not `EmpLn`). No abbreviation that the spec's glossary does
not use.

### 2.4 Clauses in code

- The item that **carries** a clause bears `#[clause("REP.8")]`:
  - a type or store for STATE;
  - a decision point's rule function for DECISION;
  - a handler for PROCESS;
  - an audit family for INVARIANT;
  - a metric for MEASURE;
  - a check, clippy rule or refusing type for FORBID;
  - a register entry for PRIMITIVE.
- Declarations carry the clause as data (`.clause("LAB.1")`), so `phx dump-registry` lists carriers.
- `phx-check clauses` verifies that each carrier has the right shape (§16.5 of the architecture).
- Comments never name clauses, documents or history (CLAUDE.md). The attribute is the only link.

### 2.5 Violations and findings

- A **contract violation** is an impossible state (II.5). It is raised by `violation!(clause = "SET.4", "what is
  impossible", key = value, …)`, defined in `phx-num` (S0.03), the lowest crate every other depends on.
  - It never returns and nothing catches it. With `panic = "abort"`, the application's panic hook writes
    `violations/<run-id>.json` and aborts.
  - The report carries the clause, the message and the keys. It also carries the site — day, sub-step, handler,
    chunk — which `phx-exec` records per worker in its one named thread-local (S0.07). Code outside a traversal
    reports the site as `outside`.
  - `clause` may name a clause (`SET.4`) or a law (`Law 7`).
- An **engineering limit** reached — a reservation, a field width, an index size — is not a clause of the world. It
  is raised by `capacity_exceeded!(what, declared, needed)`, which names the declaration to enlarge, and it also
  stops the run.
- An **audit finding** is written to the findings store (S0.09): `Finding { family, clause, owner: PartyId, size:
  i128, unit: Unit, day: Day, detail }`, where `Unit` is `Money(Ccy)` or `Qty(UnitId)`. The audit never repairs (N1,
  Law 17).
- A **run finding** — a live check that fails, or a measure outside its expected range — goes to §11 by hand. The
  run's report lists every audit finding and every failing live check.

### 2.6 Numbers and data

- Every declared number is a register entry in `data/`, in this form. A key that does not apply is **absent**,
  never empty:

  ```toml
  [[primitive]]
  id = "DEM.mortality_table"
  kind = "TECHNOLOGY"          # TECHNOLOGY | PREFERENCE | POLICY | ENDOWMENT | RESOLUTION | SHAPE
  unit = "per person per year"
  period = "year"              # only for rates and flows
  owner = "DEM"                # the system that reads and declares it
  decided_by = "treasury"      # POLICY only: the institution's role that may change it (Law 2, Law 16)
  source = "measured"          # measured | estimated | assumed | placeholder
  source_ref = "…"             # a citation, or the reason for an assumption
  shape = "placeholder:LAB"    # SHAPE only: "placeholder:<SYS>" or "standing:<reason>"
  value = …                    # scalar, table or distribution, typed by the declaration
  ```

- A system reads a primitive only through the typed handle its declaration returns. Assembly refuses an entry with
  no declaration, and a declaration with no entry.
- **State holds no floating-point number.** Money and units are `i64`, and rates and positions are fixed-point `i64`
  in declared units. Floats exist only inside pure functions (§2.19).
- **No `min`, `max` or `clamp` on a number the world decides** (Law 6). Three typed operations replace them:
  - `Qty::matched(offered, wanted)`, the quantity both sides of a trade accept (S0.03);
  - `Day::earlier` and `Day::later` (S0.05);
  - `DeclaredLimit::bind` (S0.09), for a real limit.

  Any other use needs `#[expect(clippy::disallowed_methods, reason = "…")]`, and the count of those only falls.

### 2.7 Literals

Mechanism code contains no numeric literal other than 0, 1, −1 and 2 (PC-06).
- Literals in **type positions** (array lengths, const-generic arguments) are allowed, since they state a layout, not
  a number of the world. Shift counts and bit widths use named constants or `u32::BITS` and its kin.
- Mathematical constants of an algorithm (Philox multipliers, the BTPE switch point) live in `consts.rs` with their
  source.
- `tests/`, `benches/` and `#[cfg(test)]` code are exempt.

### 2.8 Handlers

Every handler has this shape, and its context grants exactly what its declaration lists (S0.11):

```rust
#[clause("LAB.9")]
pub fn hire(ctx: &mut Ctx<'_, Hire>, rows: AgendaChunk<'_, HouseholdCells>) { … }
```

`Hire` is the handler's declaration type, generated by `declare_handler!` in `decl.rs`. It carries:
- the sub-step;
- the columns and facts it reads and writes;
- the messages and intents it emits;
- its streams.

A handler allocates nothing on the heap per row; scratch comes from the context's chunk arena.

### 2.9 Logic-level tests

- Tests live in `#[cfg(test)] mod tests` beside the code, named `<subject>_<what_holds>`, and use only values they
  build themselves: numbers, small slices, pure functions.
- A test never constructs a `World`, a `Registry` or a store of the real world (CLAUDE.md). A test that needs a table
  builds a `phx-store` column over a `VecBacking` (S0.06); that is data, not a world.
- **Statistical tests** of samplers use fixed keys. Their thresholds are normal bounds at z ≥ 6.1 (a false-failure
  probability below 10⁻⁹), derived in the test's comment, so the tests are deterministic. A key is never changed to
  make a test pass: a failure is a defect in the sampler or the test's derivation.
- `proptest` is allowed as a dev-dependency, with a fixed seed and case count from `consts.rs`, reduced under
  `cfg(miri)`.
- `trybuild` compile-fail tests prove what the types refuse.

### 2.10 Live checks

```rust
live_check! {
    id: "LC-0-07",
    title: "Every day's close has a record for every stage that ran",
    from_step: "S0.11",
    check: |w: &Inspector| -> Outcome { … },
}
```

- `Inspector` is the read-only surface of `phx-world`. It has no method that writes, and PC-20 refuses a `&mut`
  reaching the world from `checks/`.
- A check reads only what the run left: records, metrics, findings, state at a close.
- The live run is `phx run --settle <declared> --days <n> --checks all`. CI's per-push run and the nightly run are
  fixed in §14.7 of the architecture.

### 2.11 Counters and ratchets

- Kernel micro-benchmarks count instructions with `gungraun` (formerly `iai-callgrind`), under valgrind in CI. The
  engine counts rows, bytes, parts, legs and barriers.
- Wall time is measured only on the phone, and CI's wall time is never ratcheted.
- `perf/ratchets.toml` holds each counter's value and direction:

  ```toml
  [[ratchet]]
  counter = "phx_pop.bytes_per_household_cell"
  value = 464
  direction = "down"
  ```

- CI fails when a counter moves the wrong way. A counter may worsen only in a commit that edits its entry, with the
  reason, reviewed by the owner (CODEOWNERS).
- A counter with no entry is refused, never read as zero (NUM.8).

### 2.12 Dependencies

`phx-check` holds the external allow-list of **direct** dependencies; transitive ones are the direct ones' business.
Each crate is allowed only in the crates named.

| Crate | Allowed in |
| --- | --- |
| `hashbrown`, `foldhash` | `phx-core` (the kernel map, §2.17), `phx-store` |
| `libm` | foundation, kernel, systems |
| `rayon-core` | `phx-exec` |
| `libc` | `phx-exec`, `phx-store` |
| `zstd` | `phx-store` |
| `serde`, `toml` | `phx-core` (register loading), `phx-world`, `phx-cli`, `phx-check` |
| `serde_json` | `phx-cli`, `phx-check`, `phx-world` (reports) |
| `clap` | `phx-cli`, `phx-check` |
| `regex` | `phx-check` |
| `uniffi`, `ndk-sys` | `phx-ffi` |
| `syn`, `quote`, `proc-macro2` | `phx-macros`, `phx-check` |
| `cargo_metadata` | `phx-check` |
| `proptest`, `trybuild`, `gungraun` | dev-dependencies anywhere |

Random-number crates (`rand`, `rand_core`, `getrandom`) and other hashers (`ahash`, `fxhash`) are refused as direct
dependencies of world crates (PC-13). Adding a crate is a change to this table and to architecture §18, in its own
commit.

### 2.13 Arithmetic

- Every profile builds with `overflow-checks = true`, so an integer overflow anywhere stops the run.
- Money, quantities and counts use the checked operations of `phx-num`. An overflow is `violation!(clause = "Law 7")`,
  since the exact identity can no longer hold.
- Modular (wrapping) arithmetic is allowed only where the algorithm is modular — Philox, hashing, radix digits,
  encoding deltas — through the named helpers of `phx-rand`, `phx-store` and `phx-exec`. `wrapping_*`,
  `saturating_*` and `overflowing_*` are refused everywhere else (clippy lists, S0.01).

### 2.14 Errors

- `Result` is for a refusal the caller must handle as part of the world: a payment fails, an order has no match, a
  register entry is malformed at assembly.
- `violation!` is for a state that cannot exist: a currency mixed, units negative, a fact with two writers.
- A pure function returns `Result` only for input it can meet in a legal world. Otherwise it violates.

### 2.15 Rounding

A `Round` is always read from a declared convention — a contract's terms, a law, a unit's declaration. It is never
written as a literal in a mechanism. `Round::InFavourOf(Side)` expresses "in the payee's favour" (MON.16).

### 2.16 Chance

- Each process has its own stream (`<SYS>.<process>`), declared with its purpose (S0.10).
- A handler draws only through `ctx.draws(stream, subject)`. The subject is the thing acted on: a party, a part, a
  line, a tile, a region, a zone, a market, an instrument, or the world.
- A (stream, subject) is opened **at most once per sub-step**. The sub-step's ordinal is part of the address (S0.04),
  and a second open is a violation of `CHN.6`, checked under `read-trace`.

### 2.17 Order

- No result depends on the order of anything unordered (CHN.6). Maps are the kernel map (S0.09), which cannot be
  iterated.
- Every ordered output — a gather, a list, a tie — is ordered by permanent identity (party, line, instrument id),
  then by declared rule, and ties a rule leaves equal go by lot (CHN.3).

### 2.18 Constants

`consts.rs` holds engineering constants only: chunk sizes, thresholds for switching between exact algorithms,
arena growth, horizons of in-memory buffers. None may change the distribution of any outcome of the world. A
constant that would is a primitive.

### 2.19 Floats

- Floats appear only inside pure functions.
- Uniforms lie on the **open** interval (0, 1), as (k + ½)·2⁻⁵³.
- Transcendental functions come from `libm`; `mul_add` and platform intrinsics are refused.
- Small probabilities use `log1p` and `expm1`.
- An `f64` becomes an integer only by checked rounding, which refuses a non-finite or out-of-range value (NUM.6).

### 2.20 Encoding, diagnostics and names

- Everything saved or hashed is **little-endian**.
- World crates print nothing (`println!` and its kin are refused). They report through records, metrics and
  findings, and only the applications print.
- Field names carry their unit or period when a type does not: `_days`, `_m` (metres), `_per_year`, `_ppm`.

---

## 3. Stage 0 — Foundations

**Exit** (spec Part O):
- A world of parties on a map can pay each other, hold and transfer instruments and physical units, and form a price
  in each market form, with every audit family that applies running clean.
- The full opening population and its small firms, carried in cells with their holdings, profiles and the lines of
  Stage 0's systems — including the opening employment, tenancy, deposit and loan lines paying as their terms say —
  live a simulated year of deaths, illness, ageing and catastrophes within the memory budget and the time budget on
  the target device.
- The measurements of architecture §14.6 are taken.

---

### S0.01 — Workspace, toolchain, CI and `phx-check`

**Status**: planned

**Clauses**:
- TIME.11 FORBID *(part: the wall clock is refused)*.
- CHN.6 FORBID *(part: random-number crates and other hashers are refused)*.
- NUM.8 FORBID *(part: numeric literals in mechanisms are refused)*.

**Architecture**: §2, §3.1, §3.7, §14.7, §16, §17.

**Depends on**: nothing.

**Goal**: a workspace that builds, formats, lints and checks itself in CI on x86-64 and for Android from the first
day. `phx-check` enforces every rule that can exist before there is code for it to read, and its rule table is the
one place later steps add rules.

**Files**

| File | Purpose |
| --- | --- |
| `Cargo.toml` | the workspace, profiles, lints and dependencies below |
| `rust-toolchain.toml` | `channel` = the exact current stable version at the step's start; components `rustfmt`, `clippy`, `rust-src`; targets `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `aarch64-linux-android` |
| `rust-toolchain-miri.toml` | a pinned nightly used only by the `miri` job |
| `rustfmt.toml` | `max_width = 120`, `edition = "2024"`, `use_small_heuristics = "Max"`, `newline_style = "Unix"` |
| `clippy.toml` | the root disallowed lists below; `allow-unwrap-in-tests`, `allow-expect-in-tests`, `allow-panic-in-tests`, `allow-indexing-slicing-in-tests` = true |
| `crates/*/*/clippy.toml` | only in exempted crates: the root file minus that crate's declared exemptions (PC-16) |
| `.cargo/config.toml` | `[target.aarch64-linux-android] rustflags = ["-C", "target-feature=+lse,+rcpc,+dotprod,+fp16", "-C", "link-arg=-Wl,-z,max-page-size=16384"]`; aliases `check-all`, `live` |
| `CODEOWNERS` | `/perf/`, `/docs/spec/` and `/data/**/gen/` need the owner's review |
| `.github/workflows/ci.yml` | jobs `fmt`, `clippy`, `test`, `phx-check`, `android-build` (the workspace for `aarch64-linux-android` with `cargo-ndk`, NDK pinned); later steps add jobs, and none is removed |
| `perf/ratchets.toml` | the seeded entries `allow_count = 0` and `expect_count = 0` |
| `crates/apps/phx-check/Cargo.toml`, `src/main.rs` | clap subcommands: `all`, `layering`, `rules`, `docs`, `clauses`, `coverage [--write]` |
| `crates/apps/phx-check/src/rules/mod.rs` | the rule table: `struct Rule { id, title, since: &'static str, run: fn(&Workspace) -> Vec<Breach> }` |
| `crates/apps/phx-check/src/workspace.rs` | loads `cargo metadata` (direct dependencies) and parses every `.rs` file of world crates with `syn` (full, with `visit`) |
| `crates/apps/phx-check/src/comments.rs` | a small lexer that extracts comments, skipping string and character literals; `syn` does not keep plain comments |
| `crates/apps/phx-check/src/rules/*.rs` | one file per rule below |

**Design**

- **World crates** are every crate under `foundation/`, `kernel/`, `interfaces/`, `systems/` and `assembly/`. The
  crates under `apps/` are exempt from world rules, except layering.
- **Workspace** (`Cargo.toml`): `members = ["crates/*/*"]`; `resolver = "3"`; `[workspace.package]` with `edition =
  "2024"` and `rust-version` equal to the toolchain.
- **Profiles**:
  - `dev` and `test`: `overflow-checks = true`.
  - `release`: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `panic = "abort"`, `overflow-checks = true`,
    `debug = "line-tables-only"`.
  - `device`: inherits `release`, with `lto = "fat"`.
  - `bench`: inherits `release`.
- **Lints** (`[workspace.lints]`, inherited by every crate with `[lints] workspace = true`):
  - `rust`:
    - `unsafe_code = "deny"`, which `phx-store` and `phx-exec` lower with `#![allow(unsafe_code)]` in `lib.rs`
      (PC-03 refuses it anywhere else);
    - `missing_debug_implementations = "warn"`.
  - `clippy`, with the groups at `priority = -1` so single lints override them:
    - `all = "deny"`, `pedantic = "deny"`;
    - `undocumented_unsafe_blocks`, `cast_possible_truncation`, `cast_possible_wrap`, `cast_sign_loss`,
      `float_cmp`, `as_conversions`, `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`,
      `allow_attributes`, `allow_attributes_without_reason` = `"deny"`.
    - An exception is `#[expect(lint, reason = "…")]`, never `#[allow]`.
- **Clippy disallowed lists** (root `clippy.toml`, one path per entry, exact paths only):
  - `disallowed-methods`:
    - `core::cmp::min`, `core::cmp::max`, `core::cmp::Ord::min`, `core::cmp::Ord::max`, `core::cmp::Ord::clamp`;
    - `core::iter::Iterator::min`, `…::max`, `…::min_by`, `…::max_by`, `…::min_by_key`, `…::max_by_key`;
    - `f64::min`, `f64::max`, `f64::clamp`, `f64::minimum`, `f64::maximum`, `f64::mul_add`;
    - for each integer type, `saturating_add`, `saturating_sub`, `saturating_mul`, `wrapping_add`, `wrapping_sub`,
      `wrapping_mul`, `overflowing_add`, `overflowing_sub`, `overflowing_mul`;
    - `std::time::Instant::now`, `std::time::SystemTime::now`, `std::thread::spawn`, `std::process::exit`,
      `std::env::var`, `std::env::var_os`.
  - `disallowed-types`:
    - `std::collections::HashMap`, `std::collections::HashSet`, `std::hash::RandomState`;
    - `std::sync::Mutex`, `std::sync::RwLock`, `std::sync::OnceLock`, `std::sync::LazyLock`;
    - each of `std::sync::atomic::{AtomicBool, AtomicU8, AtomicU16, AtomicU32, AtomicU64, AtomicUsize, AtomicI8,
      AtomicI16, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr}`, listed one per line;
    - `std::cell::RefCell`, `std::rc::Rc`.
  - `disallowed-macros`: `std::println`, `std::eprintln`, `std::print`, `std::eprint`, `std::dbg`,
    `std::thread_local`.
  - **Exemptions** are declared per crate in `phx-check`'s table and realised by a per-crate `clippy.toml`, which
    replaces the root file:
    - `phx-exec`: atomics, `std::thread::spawn` and one `thread_local!` in `site.rs`;
    - `phx-rand`, `phx-store`, `phx-exec`: the modular integer methods, through named helpers;
    - `phx-cli`: `Instant::now`, `env::var` and the printing macros;
    - `phx-check`: the printing macros;
    - `phx-ffi`: `Instant::now`.
- **`phx-check` rules introduced here**:

  | Id | Rule |
  | --- | --- |
  | PC-01 | Layering, from `cargo metadata`: a crate depends only on lower layers, inside L0 and L1 only in the architecture's order (§3.1), no system crate on another, interface crates only on L0 and L1 |
  | PC-02 | Direct external dependencies only as in §2.12 |
  | PC-03 | `allow(unsafe_code)` only in `phx-store` and `phx-exec` |
  | PC-04 | `rayon`, `rayon-core` and `libc` as dependencies only where §2.12 allows |
  | PC-05 | No `static` or `static mut` items in world crates (constants are `const`), except the named `thread_local!` of `phx-exec/src/site.rs` |
  | PC-06 | No integer or float literal other than 0, 1, −1, 2 in world crates outside `consts.rs`, type positions, `tests/`, `benches/` and `#[cfg(test)]`; arguments of `assert!`, `vec!`, `violation!` and `format!` are parsed and checked as expressions; a literal in `consts.rs` needs a doc comment |
  | PC-07 | No comment in any crate matches `\b[A-Z]{2,4}\.\d+\b`, `\bLaw \d+\b`, `\bN\d+(\.\d+)?\b`, `§`, `\bspec(ification)?\b`, `ARCHITECTURE`, `IMPLEMENTATION`, `PROJECT_PHOENIX`, `\bS\d+\.\d+\b`, `TODO` or `FIXME`, read through the comment lexer |
  | PC-08 | Interface crates: no `fn` with a body other than a constructor (`new`, `from_*`) or a field accessor returning a field |
  | PC-09 | Documents: every workspace crate appears in architecture §3's lists; every step in this file has every section of §0.2 in order, and a valid status; at most one step is `building`; a crate exists only if its step is `building` or `done` |
  | PC-10 | The counts of `#[expect(...)]`, `#![expect(...)]` and `cfg_attr(…, expect(…))` in world crates are at most the values in `perf/ratchets.toml` |
  | PC-11 | Every `#[expect]` in a world crate has a `reason` |
  | PC-13 | No direct dependency of a world crate on `rand`, `rand_core`, `getrandom`, `ahash` or `fxhash` |
  | PC-16 | Each per-crate `clippy.toml` equals the root file minus that crate's declared exemptions |

  Each rule records the step that introduced it (`since`). Later steps add rules here, numbered on. PC-12, PC-14 and
  PC-15 are assigned by S0.03, S0.05 and S0.06.
- **CI (`ci.yml`)**:
  - on every push and pull request: `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`;
    `cargo test --workspace`; `cargo run -p phx-check -- all`; `android-build`;
  - cache keyed on `Cargo.lock` and the toolchains;
  - a pinned runner image with a timeout of 30 minutes.

**Unit tests** (in `phx-check`, over manifest graphs and source strings)
- `layering_refuses_system_on_system`.
- `layering_refuses_l0_order`: `phx-num` depending on `phx-rand` is refused.
- `interfaces_may_use_l1`: an interface crate depending on `phx-ledger` passes.
- `literals_refuse_three_in_mechanism`: `fn f() -> i64 { 3 }` is refused. None of these are refused:
  - `[u32; 4]` in a type;
  - `const X: i64 = 3;` in `consts.rs` with a doc comment;
  - `assert!(x == 3)` inside `#[cfg(test)]`.
- `literals_in_macro_args_refused`: `violation!(clause = "Law 7", "m", k = 3)` in a mechanism is refused.
- `comments_refuse_references`: `// see REP.8`, `// per Law 6` and `/// as N8.5 says` are refused. None of these are
  refused: `// the ring settles together`, `// inspect the special case`, `// CPU architecture`.
- `comment_lexer_skips_strings`: `let s = "// REP.8";` gives no comment.
- `interfaces_refuse_behaviour`.
- `docs_subset_rule`: a crate missing from architecture §3 is refused; a crate listed but not yet created passes.
- `docs_refuse_missing_section`; `docs_refuse_two_building`.
- `expect_needs_reason`.
- `per_crate_clippy_matches_root_minus_exemptions`.

**Live checks**: none. There is no world yet.

**Budget**: CI per push ≤ 10 minutes at this step; `phx-check all` ≤ 5 s on the workspace.

**Guards**: PC-01 to PC-11, PC-13 and PC-16; the lints and clippy lists; CI runs all of them, and `android-build`,
on every push.

**Not allowed**:
- creating kernel or system crates ahead of their steps;
- `#[allow]` anywhere in world crates;
- a lint turned off for a whole crate;
- a rule implemented by text search where `syn` or `cargo metadata` can answer it;
- a CI job allowed to fail.

**Done when**
- [ ] `cargo build`, `fmt`, `clippy` and `test` pass for x86-64; `android-build` passes.
- [ ] `phx-check all` passes, and each rule's tests show it refuses what it should and accepts what it should.
- [ ] CI runs every job on push and is green.
- [ ] The rule table lists its rules with `since = "S0.01"`.
- [ ] Two reviews done (§0.1 rule 6).

---

### S0.02 — `phx-macros`: the clause attribute, the store derive and the declaration macros' home

**Status**: planned

**Clauses**: none of the world; the tools every clause's carrier uses (§2.4).

**Architecture**: §3.2 (`phx-macros`), §5.1, §16.5.

**Depends on**: S0.01.

**Goal**: the procedural macros that other crates need from their first line:
- `#[clause]`, so every carrier is marked from the start;
- `#[derive(Pod)]`, the only way a type becomes storable.

The `declare_*` macros are added by the steps whose kernel types they wrap. This crate is their only home.

**Files**

| File | Purpose |
| --- | --- |
| `crates/foundation/phx-macros/src/lib.rs` | the proc-macro entry points |
| `src/clause.rs` | `#[clause("SYS.n", …)]`: validates each identifier against `^[A-Z]{2,4}\.\d+$`, `^Law \d{1,2}$`, `^N\d(\.\d+)?$`; emits the item unchanged |
| `src/pod.rs` | `#[derive(Pod)]`: requires `#[repr(C)]`; every field's type implements `Pod`; the sum of field sizes equals the type's size (a `const` assertion, so padding is refused at compile time); no `f32` or `f64` field (PC-12); emits `unsafe impl ::phx_store::Pod for T {}` and `impl ::phx_store::__seal::Sealed for T {}` |

**Design**

- The derive's `unsafe impl` is the **sanctioned path** to `Pod`. A hand-written `impl Pod` or `impl Sealed`
  anywhere is refused by PC-12. `Pod` is `unsafe trait Pod: Copy + Sealed + 'static` in `phx-store` (S0.06).
- `#[clause]` generates nothing. `phx-check clauses` reads it with `syn`.

**Unit tests**
- `clause_accepts_forms`: `#[clause("REP.8")]`, `#[clause("Law 7")]` and `#[clause("N8.5")]` compile.
- `clause_refuses_forms` (compile-fail): `#[clause("rep8")]`, `#[clause("REP-8")]`.
- `pod_refuses_padding`, `pod_refuses_float`, `pod_requires_repr_c` (compile-fail, once `phx-store` exists; until
  then asserted on the macro's expansion text).

**Live checks**: none.

**Budget**: none at run time.

**Guards**: none new; PC-12 is registered by S0.03 and becomes active at S0.06.

**Not allowed**:
- a macro that generates behaviour beyond declarations;
- a `#[clause]` that accepts free text.

**Done when**
- [ ] Both macros exist with their tests.
- [ ] Two reviews are done.

---

### S0.03 — `phx-num`: quantities, money, rounding and violations

**Status**: planned

**Clauses**:
- STATE: NUM.1; NUM.2 *(part: currency on every amount; the reporting numéraire arrives with FX, Stage 5)*; MON.16
  *(part: smallest units and rounding by convention; landing on named parties is S0.15)*.
- INVARIANT: NUM.5 *(part: the arithmetic refuses mixing; the audit is S0.12)*; NUM.6 *(part: no non-finite number
  can enter a fixed-point value)*.
- FORBID: NUM.8 *(part: `Missing` has no default)*.
- Law 7 *(part: exact integer money, checked arithmetic)*.
- II.5 *(the `violation!` path)*.

**Architecture**: §3.2, §16.2, §16.4.

**Depends on**: S0.02.

**Goal**: the types every number in the world is made of, and the one way to stop the run on an impossible state:
- money is whole smallest units of a named currency;
- quantities carry their unit;
- rates carry their period;
- positions are fixed-point;
- nothing can be read as zero when absent, mixed across currencies or units, or silently overflow.

**Files**

| File | Purpose |
| --- | --- |
| `crates/foundation/phx-num/src/violation.rs` | `Violation { clause: &'static str, message: &'static str, keys: [(&'static str, i128); 8], n_keys: u8 }`; `violation!`; `capacity_exceeded!`; `SiteProvider`, the hook through which `phx-exec` supplies the site |
| `src/money.rs` | `Ccy`, `Amount`, `Money`, `Reported` |
| `src/qty.rs` | `UnitId`, `QtyRaw`, `Qty`, `Count` |
| `src/price.rs` | `Price`, `PriceRaw`, `value_of` |
| `src/rate.rs` | `Rate`, `RatePeriod`, `DayFraction`, `accrue` |
| `src/fixed.rs` | `Fixed<const E: u8>` |
| `src/round.rs` | `Round`, `Side`, `div_round`, `split_total`, `per_member_times_count` |
| `src/missing.rs` | `Missing<T>`, `MaybeI64` |
| `src/points.rs` | `PointTable`, `PointIdx` |
| `src/consts.rs` | `RATE_SCALE = 10^12`, `ABSENT_I64 = i64::MIN`, `MAX_PRICE_EXP = 18`, each with why |

**Design**

- **Violations** (II.5, §2.5): `violation!` builds a `Violation` and calls `std::panic::panic_any`. It evaluates its
  keys as `i128`. `capacity_exceeded!` does the same with its own payload. The site is added by the panic hook
  through `SiteProvider`, which `phx-exec` sets once when the pool starts (S0.07).
- `Ccy(u8)`: a currency's index in `data/world.toml`'s `[[currency]]` list (code, name, smallest unit's name,
  country). Currency identity is data (Law 10).
- `Amount(i64)`: whole smallest units with **no currency**, used only in store columns whose currency the column
  fixes. It has no arithmetic.
- `Money { amt: i64, ccy: Ccy }` is the computing type:
  - `+` and `-` between two `Money` violate `NUM.5` if the currencies differ, and `Law 7` on overflow;
  - `Money::sum_in(ccy, iter)`; `Money::neg`; `Money::times(self, n: Count)`, where `Count(u64)` is a count of
    identical things (members, units), never an arbitrary number;
  - there is no `Mul<Money>`, no conversion from or to `f64`, and no `Sum` that could meet an empty iterator;
  - `Money::at(ccy, amount: Amount) -> Money` and `Money::amount(self) -> Amount` cross between column and value.
- `Reported<M>`: a figure in the reporting numéraire, distinct from `Money` (NUM.2), built only by FX (Stage 5).
- `UnitId(u16)`: an index into the unit table (declared data: its kind — face, shares, fund units, contracts,
  physical good, hours, persons, dwellings, square metres, kilometres — its name and its **price exponent**, at most
  `MAX_PRICE_EXP`).
- `Qty { n: i64, unit: UnitId }`, with the same checked arithmetic, refusing mixed units (violation of `NUM.5`).
  `QtyRaw(i64)` is its column form. `Qty::matched(offered: Qty, wanted: Qty) -> Qty` is the quantity both sides accept
  (§2.6), refusing mixed units.
- `Price { raw: i64, ccy: Ccy, unit: UnitId }` is smallest money units × 10⁻ᵉˣᵖ per unit, with `exp` read from the
  unit table. `PriceRaw(i64)` is its column form.
  - `value_of(q: Qty, p: Price, units: &UnitTable, r: Round) -> Money` computes `q.n × p.raw / 10^exp` with checked
    `i128` multiplication, rounded once by `r`, and checks that the units agree. It is the only route from a quantity
    to a value (NUM.1).
  - A price may be negative (Law 6).
- `Rate { raw: i64, per: RatePeriod }` is a fraction × 10¹² per `RatePeriod` (`Year`, `Month`, `Day`), so every rate
  says its period (Law 8).
  - `DayFraction { num: i64, den: i64, per: RatePeriod }` is produced by the calendar's day counts (S0.08) and says
    what period it is a fraction of.
  - `accrue(principal: Money, rate: Rate, f: DayFraction, r: Round) -> Money` requires `rate.per == f.per` (a
    violation of `TIME.4` otherwise) and computes `principal × rate × num / (10¹² × den)` with checked `i128`
    arithmetic, rounded once.
- `Fixed<const E: u8>(i64)`: a decimal with exponent `E`, for positions and outlooks. It offers:
  - addition and subtraction within the same `E`, checked;
  - `mul_div(a, b, r)` through checked `i128`;
  - `Fixed::from_f64(x, r) -> Result<Self, NumError>`, which refuses non-finite and out-of-range values;
  - `to_f64`.
- **Rounding**:
  - `enum Round { HalfEven, HalfAwayFromZero, TowardZero, Floor, Ceil, InFavourOf(Side) }` with `enum Side { Payer,
    Payee }`. `InFavourOf` resolves by the amount's direction: for an amount owed by payer to payee, `Payee` rounds up
    in magnitude and `Payer` rounds down.
  - `div_round(n: i128, d: i128, r: Round) -> i128` requires `d > 0` (a violation of `Law 7` otherwise) and returns
    the correctly rounded quotient for every sign of `n`.
  - `split_total(total: i64, k: u64, w: u64, r: Round) -> (leaving: i64, staying: i64)`, with `leaving =
    round(k·total/w)` and `staying = total − leaving`, requiring `0 < w` and `k ≤ w` (REP.9). The total is conserved
    by construction.
  - `per_member_times_count(per_member: i64, count: Count) -> i64`, checked (REP.9, TAX.7).
- `Missing<T>`: `enum Missing<T> { Present(T), Absent }`.
  - It derives `Clone`, `Copy`, `PartialEq`, `Eq` and `Debug`.
  - It has **no** `Default`, `unwrap_or`, `unwrap_or_default`, `map_or`, `From<Option<T>>` or `Into<Option<T>>`; a
    reader must `match`.
  - `MaybeI64(i64)` is its column form, with `ABSENT_I64` as the absent marker. Writing `i64::MIN` as a present value
    violates `NUM.8`.
- **Point tables**:
  - `PointTable { raw: Box<[i64]> }` holds strictly increasing raw prices, built from a register entry.
  - `PointIdx(u16)` indexes it; `point(i) -> i64` is one indexed read.
  - `at_or_below(raw) -> Missing<PointIdx>` and `at_or_above(raw) -> Missing<PointIdx>` use binary search.
  - Point tables live in the registry, not in stores.
- `NumError` has only `Overflow`, `NonFinite`, `OutOfRange` and `ZeroDivisor`, used by pure functions whose inputs
  can meet them in a legal world. Mixing currencies or units is always a violation, never an `Err` (§2.14).

**Unit tests**
- `money_add_two_currencies_violates` (a `should_panic` test on the payload's clause).
- `money_overflow_violates`.
- `value_of_rounds_once`:
  - 3 units at raw 3 333 with exp 3 is 9.999: 10 under `HalfEven`, 9 under `TowardZero`;
  - 5 units at raw 1 500 with exp 3 is 7.5: 8 under `HalfEven` and `HalfAwayFromZero`;
  - 1 unit at raw 2 500 with exp 3 is 2.5: 2 under `HalfEven` and 3 under `HalfAwayFromZero`.
- `value_of_negative_price`: 4 units at raw −1 250 with exp 3 is −5.
- `div_round_matches_definition`: for every `n` in −50..=50, every `d` in 1..=7 and every `Round`, the result equals
  exact rational rounding computed with integers.
- `split_total_conserves`: `leaving + staying == total` for totals with sign changes, `w` in 1..=97 and every `k`;
  `leaving` is within one unit of `k·total/w`.
- `accrue_known`: 1 000 000 at 5% a year over 31/365 of a year under `HalfEven` equals 4 247.
- `accrue_refuses_period_mismatch`: a daily rate with a fraction of a year violates.
- `round_in_favour_of_payee_rounds_up_magnitude`.
- `fixed_from_f64_refuses_nan`: NaN and ±∞ give `Err(NonFinite)`.
- `missing_has_no_default` (compile-fail): `Missing::<i64>::default()` and `.unwrap_or(0)` do not compile.
- `point_table_search`: on {100, 199, 250, 999}, `at_or_below(250)` is 2, `at_or_below(99)` is `Absent` and
  `at_or_above(1000)` is `Absent`.
- `maybe_i64_refuses_marker`.

**Live checks**: none.

**Budget**: `gungraun` instruction counts for `value_of`, `accrue`, `div_round` and `split_total` are recorded in
`perf/ratchets.toml` (direction `down`).

**Guards**:
- PC-12: no hand-written `impl Pod` or `impl Sealed` anywhere, and no `f32` or `f64` field in a type that derives
  `Pod` (active from S0.06).

**Not allowed**:
- a `From<f64>` for `Money`;
- `Default` on any money, quantity or missing type;
- an `Err` for a currency mix;
- a percentage tolerance in any comparison;
- `Money::times` with a non-count argument.

**Done when**
- [ ] Every type and function above exists, with the tests passing, including the compile-fail tests.
- [ ] The counters are in `perf/ratchets.toml`.
- [ ] PC-12 is registered.
- [ ] Two reviews are done.

---

### S0.04 — `phx-rand`: Philox and the samplers

**Status**: planned

**Clauses**:
- STATE: CHN.1 *(part: the counter-based source and stream keys; the stream registry is S0.10)*.
- FORBID: CHN.6 *(part: no draw depends on order)*.
- MEASURE: CHN.7 *(supporting: the samplers are exact; the realised frequencies are measured live from S0.13 and
  S0.25)*.

**Architecture**: §2 (randomness), §3.2, §7.3.

**Depends on**: S0.03.

**Goal**: one counter-based generator from which every draw of the world is addressed by (stream, subject, day,
sub-step, index), and the exact samplers the representation needs. The samplers are fast in batches, and each is
proven against its exact distribution.

**Files**

| File | Purpose |
| --- | --- |
| `crates/foundation/phx-rand/src/philox.rs` | Philox4x32-10: `philox(ctr: [u32; 4], key: [u32; 2]) -> [u32; 4]` and `philox_x4`, written for auto-vectorisation; the modular multiply-high and key bump as named helpers |
| `src/key.rs` | `Seed(u64)`, `StreamKey`, `Subject`, `stream_key(seed, name)` |
| `src/draws.rs` | `Draws`, the cursor over one address |
| `src/uniform.rs` | `open_unit() -> f64` in (0, 1) as (k + ½)·2⁻⁵³; `below_u32(n)`, `below_u64(n)` by Lemire's method |
| `src/binomial.rs` | `binomial`, `binomial_at_least_one`, `binomials_joint_at_least_one` |
| `src/multinomial.rs` | conditional binomials; `AliasTable`, `multinomial_alias` |
| `src/hypergeometric.rs` | HIN inversion when small, H2PE otherwise; `multivariate_hypergeometric` |
| `src/picks.rs` | `Fenwick` over `u64` counts; `pick_without_replacement(counts, k, out)` in O(e + k log e) |
| `src/continuous.rs` | `normal` (Wichura's AS241 inverse CDF), `log_normal`, `pareto`, `gumbel`, `exponential` |
| `src/geometric.rs` | `geometric(p) -> Missing<u64>` |
| `src/thin.rs` | `accept(prob) -> bool` |
| `src/consts.rs` | Philox constants, the BTPE and HIN switch points, AS241 coefficients, each with its source |

**Design**

- **Addressing** (CHN.1, CHN.6, §2.16):
  - `stream_key(seed, name) -> StreamKey([u32; 2])` is words 0 and 1 of `philox([fnv_lo, fnv_hi, seed_lo,
    seed_hi], FIXED_KEY)`, where `fnv` is FNV-1a-64 of the stream's name. A stream's key therefore depends only on its
    name and the seed, and adding a stream changes no other. Name collisions are refused by the registry (S0.10).
  - The counter is `[subject_lo, subject_hi, day, (substep << 24) | block]`, where `substep` is the sub-step's ordinal
    (at most 255) and `block` counts blocks of four words (at most 2²⁴). Exceeding it is a
    `capacity_exceeded!`.
  - `Subject` is a `u64`: a 4-bit tag and 60 bits of identity. The tags are `World`, `Party`, `Part`, `Line`,
    `Tile`, `Region`, `Zone`, `Country`, `Market`, `Instrument`, `Stream`. An identity at or above 2⁶⁰ is a
    `capacity_exceeded!`.
  - Draws for different subjects never share a counter, so the order in which subjects are processed cannot change a
    draw.
- **`Draws`** `{ key: [u32; 2], ctr: [u32; 4], buf: [u32; 4], pos: u8 }`:
  - `Draws::new(key, subject, day, substep)` starts at block 0;
  - every sampler takes `&mut Draws`;
  - a sampler that consumes a variable number of words (rejection) draws from the same cursor, so its result depends
    only on the address.
- **Uniforms**:
  - `open_unit` never returns 0 or 1, so every inversion's logarithm is finite (§2.19).
  - `below_u64(n)` uses Lemire's multiply-shift with the `u128` product and rejection below the threshold `(u64::MAX
    % n + 1) % n`, computed without wrapping. `below_u32` is its 32-bit form.
- **Exactness**: every sampler returns an exact draw from its distribution, up to the 53-bit uniform.
  - `binomial(n, 0)` is 0 and `binomial(n, 1)` is `n`.
  - p outside [0, 1] or not finite is a violation of `CHN.2`.
  - `binomial` switches to BTPE when `n·p ≥ BINV_SWITCH` and `n·(1−p) ≥ BINV_SWITCH` (two comparisons, no `min`).
  - `hypergeometric(total, successes, draws)` requires `successes ≤ total` and `draws ≤ total`.
- **`binomial_at_least_one(n, p)`**: draws from Binomial(n, p) conditioned on at least one success. With `log_p0 = n ·
  log1p(−p)`:
  - if `exp(log_p0) > ½`, by inversion from 1 with the conditional probabilities, whose normaliser `−expm1(log_p0)`
    is computed without cancellation;
  - otherwise by rejection of zeros.
  
  `n·p = 0` is a violation (conditioning on an impossible event).
- **`binomials_joint_at_least_one(ns, ps, out)`** draws counts jointly conditioned on a total of at least one
  (architecture §7.3). In log space, `log_z_v = n_v · log1p(−p_v)` and `log_r_v = Σ_{u>v} log_z_u`. For value `v` in
  order, while no success has been drawn:
  - `out[v] = 0` with probability `exp(log_z_v) · (−expm1(log_r_v)) / (−expm1(log_z_v + log_r_v))`;
  - otherwise `out[v]` is drawn by `binomial_at_least_one`.
  
  After the first success, plain binomials. A total probability of zero (every `n_v·p_v = 0`) is a violation of
  `CHN.2`.
- **`multinomial(n, probs, out)`**: conditional binomials in the given order. **`multinomial_alias(n, table, out)`**:
  `n` alias draws, used when `n` is below the number of categories. Both give counts summing to `n`.
- **`multivariate_hypergeometric(counts, n, out)`**: sequential conditional hypergeometrics, with `out ≤ counts`
  elementwise and a sum of `n`.
- **`pick_without_replacement(counts, k, out)`**: build a Fenwick tree over `counts`, then make `k` picks, each a
  `below_u64` of the remaining total and a tree descent, decrementing as it goes. The result is picks per category.
- **`geometric(p) -> Missing<u64>`**: the number of failures before the first success, `floor(ln U / log1p(−p))`
  with `U = open_unit()`.
  - `p = 1` gives 0; `p = 0` is a violation (the caller schedules nothing);
  - a value beyond `u64` or beyond the calendar's range is `Absent`: no success within the world's representable
    time. The caller schedules no candidate and redraws if the rate rises.
- All transcendental functions come from `libm`.

**Unit tests** (fixed keys; each threshold derived in its comment at z ≥ 6.1)
- `philox_known_answer`: the Random123 known-answer vectors for Philox4x32-10.
- `philox_x4_equals_scalar` on 10⁴ counters.
- `stream_keys_independent_of_registration`: keys for names A, B and C are the same computed in order A, B, C or C,
  A, B, or with D added.
- `subjects_never_collide`: a party and a line with equal numeric ids give distinct counters.
- `substeps_never_collide`: the same subject and day in two sub-steps give distinct counters.
- `below_exhaustive_u8`: an 8-bit instance of the same method, for every n in 1..=255, maps the 256 inputs with each
  output's count equal to `floor(256/n)` or accepted exactly.
- `open_unit_never_zero_or_one` over the extreme words.
- `binomial_exact`: for (n, p) in {(0, .3), (1, .5), (7, .01), (40, .2), (1000, .5), (10⁶, 10⁻⁵), (10⁹, .5)}, the mean
  and variance of 10⁵ draws are within bounds, and chi-square against the exact pmf passes for the small cases.
- `binomial_at_least_one_never_zero_and_exact`, across the switch point and for p = 10⁻¹².
- `joint_at_least_one_matches_conditioned_product`: for ns = [3, 5, 2] and ps = [.01, .2, .05], the joint frequency
  of each outcome vector over 10⁶ draws matches the product of binomial pmfs conditioned on a total of at least one.
- `joint_at_least_one_refuses_impossible`.
- `multinomial_sums_and_marginals`; `alias_matches_weights`.
- `hypergeometric_exact`; `mvh_bounds_and_sum`; `picks_match_mvh`.
- `geometric_mean_and_absent_beyond_range`.
- `normal_moments_and_tails`; `pareto_tail_index`.
- `same_address_same_draw`.

**Live checks**: none.

**Budget**: `gungraun` instruction counts are ratcheted for each sampler. The phone targets below are checked at
S0.26 and then stand as the unit costs of architecture §13.2.

| Operation | Phone target |
| --- | --- |
| Philox per four words, batched with NEON | ≤ 3 ns |
| Philox per four words, scalar | ≤ 9 ns |
| `open_unit` | ≤ 2 ns |
| `binomial` with small `n·p`, given a precomputed `log1p(−p)` | ≤ 40 ns |
| BTPE | ≤ 80 ns |
| Alias draw | ≤ 5 ns |
| `pick_without_replacement`, per pick at e = 64 | ≤ 30 ns |

These are reference targets, not ratchets; the x86-64 numbers of the second review of this block were 9 ns for
scalar Philox and 67–107 ns for a small binomial with its `pow`.

**Guards**: PC-13 already refuses other random crates as direct dependencies of world crates.

**Not allowed**:
- a sampler that approximates, such as a normal approximation to the binomial;
- a draw keyed by a loop index or a thread id;
- a global or cached generator state;
- `std`'s `powf` or `ln` in place of `libm`;
- a uniform that can be 0.

**Done when**
- [ ] Every sampler above exists, with the tests passing and the known answers matching.
- [ ] The instruction counts are ratcheted.
- [ ] Two reviews are done.

---

### S0.05 — `phx-id`: identities and the day

**Status**: planned

**Clauses**:
- STATE: TIME.1 *(part: `Day`)*, TIME.2 *(part: `Date` and the mapping)*, PTY.1 *(part: identities never reused)*.
- INVARIANT: TIME.9 *(part: no default day)*.
- FORBID: PTY.13 *(part: no identity reused)*.

**Architecture**: §3.2.

**Depends on**: S0.04.

**Goal**: the identifier types every crate shares, with no `Default` and no conversion across kinds; the day and the
civil date; the subjects of random draws.

**Files**

| File | Purpose |
| --- | --- |
| `crates/foundation/phx-id/src/ids.rs` | `PartyId`, `Slot`, `TableId`, `RowRef`, `LineId`, `InstrumentId`, `MarketId`, `TileId`, `ZoneId`, `RegionId`, `CountryId`, `DayLocalId`, `MsgId`, `StreamId`, `SystemCode` |
| `src/day.rs` | `Day`, `Date`, `days_from_civil`, `civil_from_days`, `Weekday`, `Day::earlier`, `Day::later` |
| `src/subject.rs` | `impl From<PartyId> for Subject` and the others: each id into its `phx_rand::Subject` tag |

**Design**

- **Identities**:
  - `PartyId(NonZeroU64)` is allocated by the directory (S0.09) from one monotone counter and never reused (PTY.1,
    PTY.13). It stays below 2⁶⁰ (checked when allocated).
  - `LineId(u32)` and `InstrumentId(u32)` are never reused either: a retired line or instrument keeps its id.
  - `Slot(u32)` is a row's storage index. It may be recycled (architecture §7.2); an identity never is.
  - `RowRef { table: TableId(u16), slot: Slot }`.
- **Other ids**: `MarketId(u16)`, `TileId(u32)`, `ZoneId(u32)`, `RegionId(u16)`, `CountryId(u8)`, `DayLocalId(u32)`,
  `MsgId(u64)`, `StreamId(u32)`, and `SystemCode { bytes: [u8; 4], len: u8 }` for codes of two to four letters.
- **No mixing**: no id implements `From` or `Into` another id, and none implements `Default`. Each derives `Copy`,
  `Eq`, `Ord` and `Hash` (for the kernel map) and `Debug`.
- **Days and dates**:
  - `Day(u32)` counts days since the epoch declared in `data/world.toml`. `Day::succ` is checked.
  - `Day::earlier(a, b)` and `Day::later(a, b)` are the named comparisons of §2.6.
  - `Date { year: i32, month: u8, day: u8 }` is proleptic Gregorian. `days_from_civil(Date) -> i64` and
    `civil_from_days(i64) -> Date` are Hinnant's algorithms over `i64` serials, converting to `Day` with a check.
  - `Weekday::of(Day)` is derived from the epoch's civil date, not declared.

**Unit tests**
- `civil_roundtrip`: every serial from −10⁶ to 10⁶ round-trips; the leap years 1900, 2000 and 2100 are correct.
- `weekday_known`: 1 January 2000 is a Saturday.
- `ids_do_not_mix` (compile-fail): `PartyId == LineId`, `Day::default()` and `LineId::from(PartyId)`.
- `party_id_refuses_2_pow_60`.

**Live checks**: none.

**Budget**: every id is `Copy`, at its stated width.

**Guards**: PC-14: no id type in `phx-id` implements `Default`, derived or by hand.

**Not allowed**:
- a `usize` or `u64` passed where an id is meant;
- `as` casts between ids;
- a date computed by adding 30 days for a month.

**Done when**
- [ ] The types exist, with the tests passing.
- [ ] PC-14 is registered.
- [ ] Two reviews are done.

---

### S0.06 — `phx-store`: columns, arenas, block lists, slots and encoding

**Status**: planned

**Clauses**:
- STATE: SET.12 *(part: the encoding of state)*.
- INVARIANT: SET.15 *(supporting: encoding round-trips exactly; the invariant is S0.20's guard)*.

**Architecture**: §3.3, §4.5, §7.1, §7.2, §11, §13.1.

**Depends on**: S0.05.

**Goal**: the memory every table of the world lives in:
- columns in reserved address space that never move;
- chunk-local arenas of variable-stride records, compacted in place;
- global block lists for lists that span chunks, such as a line's holders;
- slot allocation with deterministic recycling;
- per-chunk mutable views for parallel handlers;
- the page encoding that saves use.

This is one of the two crates allowed `unsafe`.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-store/src/backing.rs` | `trait Backing`: `reserve(bytes)`, `commit(range)`, `decommit(range)`. `MmapBacking` (Linux and Android: `mmap` with `PROT_NONE` and `MAP_NORESERVE`; `mprotect` to commit; `madvise(MADV_DONTNEED)` to decommit) is tested natively. `VecBacking` is for tests and Miri. |
| `src/pod.rs` | `pub unsafe trait Pod: Copy + __seal::Sealed + 'static`; `pub mod __seal { pub trait Sealed {} }`; `Pod` impls for the integer types and the ids of `phx-id` |
| `src/descriptor.rs` | `ColumnDescriptor { name, elem_bytes, rows_per_chunk, transform }`, from which counters read bytes per row |
| `src/column.rs` | `Column<T: Pod>`: `len`, `push`, `extend`, `get`, `slice`, and `chunks_mut() -> impl Iterator<Item = ChunkMut<'_, T>>`, disjoint mutable views per chunk |
| `src/table.rs` | `Table`: columns sharing one slot space; `SlotAlloc`; `TableChunks`, the per-chunk views of all its columns at once |
| `src/arena.rs` | `ChunkArena`: 8-byte words; `ListRef { off: u32, len: u32, cap: u32 }` in words |
| `src/block_list.rs` | `BlockPool` of 16-entry blocks of `u32`; `BlockList { head: u32, tail: u32, len: u32 }`; `insert_sorted`, `remove_sorted`, iteration in order |
| `src/hash.rs` | a fixed 128-bit streaming hash over 64-byte blocks with a Philox-based compression, for world hashes (S0.11) |
| `src/encode.rs` | `encode_column`, `decode_column` |
| `src/consts.rs` | reservation defaults, `ARENA_DEAD_FRACTION`, `ARENA_GROWTH` (5/4), `DEFAULT_ROWS_PER_CHUNK` (4 096), `ENCODE_BLOCK` (1 024), `FRAME_BYTES` (1 MiB), `VA_BUDGET` |

**Design**

- **Reservation**:
  - Each column reserves address space for its declared maximum rows and commits pages as it grows.
  - The page size is read once from the system when the backing is created; 16 KiB and 4 KiB are both handled.
  - Growth past the reservation is `capacity_exceeded!("column rows", declared, needed)`.
  - The total reserved address space stays within `VA_BUDGET` (64 GiB, well inside a 39-bit address space).
  - Reservations with `PROT_NONE` and `MAP_NORESERVE` do not count as resident memory; only committed pages do, and
    the memory budget is measured on those (N8.4).
- **Chunks**: rows are grouped in chunks of `rows_per_chunk`, a power of two declared per table. Chunk `c` holds
  slots `[c·R, (c+1)·R)`. `chunks_mut` yields disjoint `&mut` views, so parallel handlers each own their chunks.
- **Slot allocation**: `SlotAlloc` keeps a free list of released slots.
  - Slots released during a day join the list at the day's close, in ascending order.
  - Allocation takes the lowest free slot, then extends.
  - Allocation within a sub-step goes through the gather of S0.07, so the slot a row receives never depends on
    thread timing.
- **`ChunkArena`** holds the variable-length lists of one chunk's rows (relationship rows, profiles, holdings):
  - It is a reserved region per chunk of 8-byte words. The variable-stride records of architecture §4.5 (16, 24 or
    32 bytes) are encoded by their owners (`phx-ledger`, `phx-pop`) as whole words.
  - `ListRef { off, len, cap }` counts words.
  - `append(list, words)` writes in place if `len + n ≤ cap`. Otherwise it copies the list to the arena's end with
    `cap = ceil(ARENA_GROWTH × (len + n))`, rounded up to four words, and marks the old span dead.
  - `remove(list, at, n)` shifts the rest down, so a list declared sorted stays sorted.
  - **Compaction**, when dead words exceed `ARENA_DEAD_FRACTION` of the arena: copy every live list, in **slot
    order**, into a chunk-sized scratch; copy the scratch back to the arena's start; rewrite each `ListRef`. This is
    safe whatever the physical order, and it restores locality. Pages above the new end are decommitted.
  - Only the chunk's own rows hold `ListRef`s into it (the invariant that keeps compaction local).
  - `move_list(from_chunk, to_chunk, list)` moves a list across chunks for renumbering (S0.24).
- **`BlockList`** is for lists that are not a chunk's own: a line's holders, an instrument's holders. Blocks of 16
  `u32` come from a global `BlockPool` with a free list. `insert_sorted` and `remove_sorted` keep order within and
  across blocks, splitting and merging blocks at half occupancy.
- **Encoding** (SET.12), a pipeline per column and per block of `ENCODE_BLOCK` elements:
  1. optional **delta**, computed modulo 2⁶⁴ with the named modular helper; its inverse is exact;
  2. optional **zigzag**;
  3. **bit-pack** at the block's widest value.

  Element widths are 1, 2, 4 or 8 bytes; struct columns are encoded field by field, with the transform declared per
  field. Blocks are written as zstd level-1 frames of `FRAME_BYTES`, **little-endian**. The header is: magic `u32`,
  format version `u16`, element width `u8`, transform `u8`, element count `u64`, and per block its bit width `u8`.
  `decode_column` is the exact inverse.

**Unit tests** (under Miri with `VecBacking`, sizes reduced under `cfg(miri)`)
- `column_never_moves`: pointers to element 0 stay equal across growth (10⁶ elements natively, 10⁴ under Miri).
- `chunks_mut_are_disjoint`.
- `slot_recycling_is_ordered`: slots released as 7, 3, 5 in a day are reused next day as 3, 5, 7.
- `arena_append_relocates_and_counts_dead`.
- `arena_compaction_preserves_lists_any_physical_order`: for random operation sequences from a fixed seed, including
  relocations that invert physical order, the lists after compaction equal a `Vec<Vec<u64>>` model, in slot order.
- `arena_sorted_remove_keeps_order`.
- `block_list_sorted` over random inserts and removals, compared with a sorted `Vec`.
- `encode_roundtrip`: every pipeline, over random, sorted, constant, sign-changing and extreme values, round-trips byte
  for byte.
- `pod_derive_refuses_padding_and_floats`, `pod_requires_repr_c` (compile-fail).
- `mmap_backing_commit_decommit` (native only, not under Miri).

**Live checks**: none.

**Budget**:
- Column append is amortised O(1), with no copy.
- Arena compaction is O(live words of the chunk); arena slack is at most 15% (architecture §13.1).
- Block-list insertion is O(log blocks + 16).
- `gungraun` counts for append, compaction per word, block-list insert, and encode and decode per byte are ratcheted.
- Counters: `phx_store.bytes_committed`, `phx_store.arena_dead_words`, `phx_store.compactions`.

**Guards**:
- PC-12 becomes active.
- The CI job `miri` runs `cargo +<nightly> miri test -p phx-store` with the pinned nightly.
- A `cargo-public-api` snapshot of `phx-store` is committed; PC-15 fails the build when a kernel or interface crate's
  public API differs from its snapshot outside a commit that updates the snapshot (architecture §16.7).
- `android-build` now builds `phx-store`, including its `libc` calls.

**Not allowed**:
- `Vec` or `Box` inside a store;
- a `realloc` of a column;
- a list referenced from outside its chunk;
- compaction that reorders rows or skips relocated lists;
- `unsafe` without a safety comment (`undocumented_unsafe_blocks`) and a test that covers it.

**Done when**
- [ ] The types exist, with the tests passing natively and under Miri (except the mmap test, native only).
- [ ] The encoding is versioned and round-trips.
- [ ] The public-API snapshot is committed; PC-15 is registered.
- [ ] Two reviews are done.

---

### S0.07 — `phx-exec`: the pool, traversals, gathers and reductions

**Status**: planned

**Clauses**:
- PROCESS: TIME.6 *(part: the mechanics that run a sub-step's handlers over chunks)*.
- FORBID: CHN.6 *(part: no result depends on thread count, timing or completion order)*.
- N5 *(part: one worker and all workers give the same world)*.

**Architecture**: §2 (parallelism), §3.3, §4.8, §6.2, §6.3, §6.4, §17.

**Depends on**: S0.06.

**Goal**: everything that runs in parallel runs here, with results that depend neither on how many workers there are
nor on the order they finish in. It provides:
- the pool, pinned to fast and medium cores;
- traversals by table chunks or agenda chunks, each snapped to table chunks;
- gathers, keyed reductions, fixed-tree reductions and radix sorts;
- the per-worker site for violation reports;
- the `Clock` and `PerfHint` interfaces.

These are the only parallel primitives the rest of the engine may use.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-exec/src/pool.rs` | `Pool::new(PoolSpec)` over `rayon-core`, pinning each worker in its `start_handler`; `Pool::worker_tids()` |
| `src/spec.rs` | `PoolSpec::detect()`: the cores allowed by `sched_getaffinity`; if `cpu_capacity` is readable, those with capacity at least `LITTLE_CORE_SHARE` of the largest; otherwise all allowed cores; pinning is best effort (`EINVAL` or `EPERM` leaves the worker unpinned and counts it) |
| `src/traverse.rs` | `for_chunks(chunks, f)`; `for_agenda(agenda_rows, cost_per_row: u32, f)` |
| `src/gather.rs` | `IntentBuf<T>` per (chunk, handler), reused across days; `gather` |
| `src/keyed.rs` | `KeyedReduce<K: RadixKey, V>` |
| `src/tree.rs` | `reduce_tree` |
| `src/radix.rs` | LSD radix sort of `(u64, u32)` and `(u128, u32)` with 11-bit digits, skipping constant digits, parallel histograms |
| `src/mix.rs` | `mix64`, the SplitMix64 finaliser: the fixed shard function (never `foldhash`) |
| `src/site.rs` | the one named `thread_local!` of the engine: the current (day, sub-step, handler, chunk) per worker, and the `phx_num::SiteProvider` implementation |
| `src/clock.rs` | `trait Clock { fn now_ns(&self) -> u64; }`, implemented by the applications; never read by the world |
| `src/hint.rs` | `trait PerfHint { fn begin_turn(&self, target_ns: u64); fn end_turn(&self, actual_ns: u64); }` and `NoHint` |
| `src/counters.rs` | `ExecCounters` per sub-step: rows touched, bytes touched (from column descriptors), chunks, barriers, wall time through the injected `Clock` |
| `src/consts.rs` | `KEYED_SHARDS` (256), `RADIX_BITS` (11), `CHUNK_COST`, `LITTLE_CORE_SHARE` (a half), `SPIN_NS` |

**Design**

- **Determinism** (CHN.6, N5):
  - Chunk boundaries depend only on the table's size, or on the agenda's rows and declared cost.
  - Workers take chunks dynamically, but every output is stored at its chunk's index.
  - Gathers concatenate in (chunk, canonical handler id) order.
  - Reductions fold in index order.
  - Nothing reads the worker index except `pool.rs` and `site.rs`.
- **Agenda chunks** (§6.3):
  - `for_agenda` walks the agenda's slots, which are sorted.
  - It closes a unit **only at a table-chunk boundary** once the accumulated cost reaches `CHUNK_COST`, so a unit is
    one or more whole table chunks' agenda rows.
  - Arena writes are therefore always chunk-local, and the boundaries depend on the rows alone.
- **`KeyedReduce<K, V>`**:
  1. Each chunk stable-partitions its `(K, V)` pairs by `mix64(K) mod KEYED_SHARDS`.
  2. Each shard concatenates its partitions in chunk order and runs one stable LSD radix sort by `K`.
  3. It then folds equal keys with the caller's `fold(&mut V, V)` in (chunk, position) order.

  The output is per shard, sorted by key, and can be applied shard by shard (architecture §6.5). Identities assigned
  in shard order depend only on `mix64`, a fixed function of this crate.
- **`reduce_tree(results, f)`**: a left-balanced pairwise tree over results in index order. The shape depends only
  on the number of results.
- **Barriers**: workers spin for `SPIN_NS` after a sub-step before parking, so back-to-back sub-steps do not pay a
  wake-up. Spin time is counted.
- **Site** (§2.5): `site.rs` holds the engine's only thread-local, written when a worker starts a chunk and read only
  by the panic hook through `SiteProvider`.
- **Wall time** reaches only counters and the application, never the world (TIME.11, Law 17).
- **Atomics and threads** appear only inside `phx-exec` (its clippy exemptions), and its public API exposes none.

**Unit tests**
- `traverse_is_worker_independent`: a chunked map over 10⁶ rows gives identical output with 1, 2, 3 and 8 workers.
- `agenda_units_snap_to_table_chunks`.
- `gather_order_is_canonical`: buffers filled in scrambled completion order gather to (chunk, handler) order.
- `keyed_reduce_matches_sequential`: for random pairs from a fixed seed, the output equals a sequential fold in
  (chunk, position) order, for 1 and 8 workers and for a non-commutative fold.
- `tree_reduce_fixed_shape`: a non-associative float sum gives the same bits for 1 and 8 workers.
- `radix_sorts_stably`, including constant-digit skipping.
- `detect_falls_back_without_capacity`: with no capacity files, every allowed core is used.

**Live checks**: none. The one-worker guard becomes live at S0.11.

**Budget**: `gungraun` instruction counts for gather, keyed reduce and radix sort are ratcheted. Reference targets on
the phone, checked at S0.26:

| Operation | Phone target |
| --- | --- |
| Barrier (dispatch and join of an empty sub-step), hot, 6 workers | ≤ 60 µs |
| Gather | ≥ 4 GB/s aggregate |
| Radix sort of 10⁷ `(u64, u32)` | ≤ 250 ms on the pool |
| `KeyedReduce` of 10⁷ pairs | ≤ 300 ms on the pool |

The x86-64 numbers of this block's second review were a 54 µs hot barrier on 4 workers and 216 ms for the radix sort
with 8-bit digits. About 60 sub-steps with work a day keep barriers within architecture §13.2's line.

**Guards**:
- The `phx-exec` clippy exemptions (S0.01) are the only place atomics and threads are allowed; PC-05 names the one
  thread-local.
- A `cargo-public-api` snapshot of `phx-exec` is committed (PC-15).

**Not allowed**:
- `par_iter` or rayon's global pool;
- a chunk size computed from the number of workers;
- an agenda unit that splits a table chunk;
- a reduction with a data-dependent tree;
- `foldhash` or any hasher that could change with a crate version deciding a shard;
- a `Mutex`-protected shared output;
- reading a clock inside a traversal.

**Done when**
- [ ] The primitives exist, with the tests passing for 1, 2, 3 and 8 workers.
- [ ] Pinning and its fallback work on Linux, and the Android build compiles them.
- [ ] Instruction counts are ratcheted; the snapshot is committed.
- [ ] Two reviews are done.

---

## 11. Findings

| Id | Step | Day | What was measured, where | Mechanism suspected | Addressed by | Status |
| --- | --- | --- | --- | --- | --- | --- |

---

## 12. Owner decisions

| Decision | Answer | Date |
| --- | --- | --- |
| Memory budget (N8.4) | 4.5 GB resident | 2026-09-23 |
| Map (spec Appendix E 29) | about 40,000 tiles of 10 km; 12, 8 and 5 regions | 2026-09-23 |
| Accuracy for play (N8.5, Appendix E 30) | on every declared read, within 5% on means and shares and 10% on tail quantiles beyond seed spread; the difference published | 2026-09-23 |
| Coarsening for the phone (Appendix E 31) | pooled flows; employment lines by occupation family and region with a five-year start band; reviews on a cell's review days; sellers spread on review days | 2026-09-23 |
| Budget stance (N8) | keep 1 s / 2 s and 4.5 GB; coarsen the spec rather than relax the budget | 2026-09-23 |
| Settling length (GEN.6) | one simulated year by default; adjustable | 2026-09-23 |
| Save interval (SET.12) | every simulated quarter by default | 2026-09-23 |

---

## 13. The clause map

Every clause of the spec is completed by exactly one step, listed here. Steps before it may carry it in part, as their
**Clauses** sections say. `phx-check clauses` refuses a clause missing from this map or completed by two steps, and a
step marked `done` whose completed clauses have no carrier of the right shape in `phx dump-registry` (architecture
§16.5). `phx-check coverage --write` regenerates architecture §19 from this map and the steps' statuses.

When a stage's block of steps is written in detail, its rows here are refined to the clauses its steps actually
complete, in the same change. Retired clauses (REP.6, REP.11, REP.27, SET.14) keep their numbers and are not mapped.

| System | Step | Clauses |
| --- | --- | --- |
| TIME | S0.08 | 1, 2, 3, 4, 5, 11, 12, 13 |
| TIME | S0.11 | 6, 8, 9, 10 |
| TIME | S0.17 | 7 |
| PTY | S0.09 | 1, 4, 6, 8, 10, 13, 14, 15 |
| PTY | S0.25 | 2, 3, 5, 9, 11 |
| PTY | S3.05 | 7 |
| PTY | S7.03 | 12 |
| NUM | S0.03 | 1, 6 |
| NUM | S0.09 | 3, 4, 7, 8, 9 |
| NUM | S0.15 | 5 |
| NUM | S5.04 | 2 |
| CHN | S0.10 | 1, 2, 4, 5, 6, 8 |
| CHN | S0.25 | 7 |
| CHN | S6.02 | 3 |
| GEO | S0.13 | 1, 2, 3, 6, 7, 10, 11, 14, 15, 16, 17 |
| GEO | S0.25 | 5, 8 |
| GEO | S1.05 | 9, 12 |
| GEO | S1.07 | 4, 13, 18 |
| REP | S0.21 | 1, 3, 4, 19, 20, 32, 33 |
| REP | S0.22 | 7, 12, 21, 35 |
| REP | S0.23 | 8, 9, 16, 17, 22, 23, 24, 36 |
| REP | S0.24 | 2, 10, 13, 14, 15, 18, 28, 29, 31 |
| REP | S0.25 | 25, 26 |
| REP | S0.26 | 30 |
| REP | S1.03 | 34 |
| REP | S1.06 | 37 |
| REP | S1.12 | 5 |
| GEN | S0.16 | 11 |
| GEN | S0.25 | 6 |
| GEN | S0.26 | 8 |
| GEN | S6.05 | 1, 2, 3, 4, 5, 7, 12 |
| GEN | S7.01 | 9, 10 |
| MON | S0.15 | 1, 2, 3, 6, 7, 8, 9, 11, 12, 13, 14, 16 |
| MON | S0.17 | 5 |
| MON | S1.09 | 4 |
| MON | S1.10 | 15 |
| MON | S1.14 | 10 |
| SET | S0.15 | 1, 2, 3, 4, 5, 7, 8, 9, 10, 11, 16, 17 |
| SET | S0.17 | 6 |
| SET | S0.20 | 12, 13, 15 |
| REG | S0.14 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13, 14, 15, 16, 17, 18 |
| REG | S3.05 | 11, 12 |
| ACC | S0.19 | 1, 2, 3, 4, 6, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17 |
| ACC | S2.01 | 7 |
| ACC | S3.05 | 5 |
| MKT | S0.18 | 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 21 |
| MKT | S3.06 | 5 |
| MKT | S4.01 | 20 |
| VAL | S1.01 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23 |
| POP | S0.25 | 3, 4 |
| POP | S1.13 | 5, 10 |
| POP | S2.04 | 9, 15 |
| POP | S6.02 | 1, 2, 6, 7, 8, 11, 12, 13, 14, 16 |
| HH | S1.12 | 1, 2, 3, 4, 5, 6, 7, 13, 15, 18, 19, 20 |
| HH | S2.05 | 8, 9, 10 |
| HH | S2.11 | 14, 21 |
| HH | S4.03 | 11 |
| HH | S5.03 | 12 |
| HH | S6.03 | 16, 17 |
| TEC | S1.02 | 1, 2, 3, 4, 9, 12, 13 |
| TEC | S6.01 | 5, 6, 7, 8, 10, 11 |
| FRM | S1.03 | 1, 2, 4, 5, 6, 7, 8, 11, 13, 14, 16, 17, 18, 20, 22, 23 |
| FRM | S2.03 | 12, 15, 19, 21 |
| FRM | S3.05 | 3, 9, 10 |
| CAP | S1.04 | 1, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13 |
| CAP | S2.05 | 2, 7 |
| GDS | S1.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13 |
| SRV | S1.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9 |
| FRT | S1.07 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| LAB | S1.08 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17 |
| HSG | S2.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 |
| TCR | S2.02 | 1, 2, 3, 4, 5, 6, 7, 8 |
| ENE | S2.09 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 |
| BNK | S1.09 | 1, 2, 4, 5, 6, 8, 11, 14, 15, 16, 17, 18, 19, 20 |
| BNK | S2.01 | 7, 9, 10, 12 |
| BNK | S2.10 | 21 |
| BNK | S2.12 | 13 |
| BNK | S3.04 | 3 |
| BNK | S3.08 | 22 |
| BFL | S2.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 |
| BCP | S2.07 | 1, 2, 3, 4, 5, 6, 7, 8, 9 |
| SEC | S4.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 |
| MMK | S3.01 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| SOV | S1.11 | 1, 2, 3, 4, 5, 6 |
| SOV | S3.03 | 7, 8, 9, 10 |
| CRD | S3.04 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| EQY | S3.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| MNA | S4.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| FND | S3.07 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13 |
| DLR | S3.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| IDX | S1.14 | 1, 3, 4, 5, 6, 7 |
| IDX | S3.09 | 2 |
| RAT | S2.10 | 2, 8 |
| RAT | S3.10 | 1, 3, 4, 5, 6, 7, 9 |
| DRV | S4.01 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| DRX | S4.02 | 1, 2, 3, 4, 5, 6, 7, 8 |
| INS | S4.03 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| PEN | S4.04 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| TRS | S1.11 | 1, 4, 6, 9 |
| TRS | S3.03 | 2, 3, 5, 7, 8 |
| TAX | S1.11 | 1, 2, 5, 7 |
| TAX | S5.01 | 3, 4, 6, 8 |
| SOC | S1.11 | 1, 3, 7 |
| SOC | S5.02 | 2, 4, 5, 6, 8, 9 |
| CB | S1.10 | 1 |
| CB | S3.02 | 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 16 |
| CB | S5.05 | 11, 15 |
| SUP | S2.08 | 1, 2, 3, 5, 6, 7, 8, 9, 12 |
| SUP | S4.07 | 4, 10, 11 |
| POL | S5.03 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| FX | S5.04 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| XB | S5.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| OBS | S0.10 | 1, 3 |
| OBS | S0.26 | 4, 5, 9 |
| OBS | S6.04 | 2, 6, 7, 8 |
| STA | S1.14 | 1, 2, 3, 4, 5 |
| L1 | S2.01 | a loss is an event |
| L2, L4 | S3.11 | the forced seller; the cost of capital |
| L3 | S2.04 | estates, their ranking and destinations (the waterfall's machinery is S0.17) |
| L5–L12 | S7.02 | the causal chains, as N4 tests them |
| N1 | S6.05 | the audit's families complete with the last system; the framework is S0.12 |
| N2, N6, N8 | S0.26 | liveness reads, experiments, the budget's measurement; judged again at every gate |
| N3 | S7.01 | the stylised facts |
| N4 | S7.02 | the causal-chain tests |
| N5 | S7.03 | reproducibility, resolution and seeds |
| N7 | S7.04 | calibration |
