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
    chunk — which `phx-exec` records per thread (the workers and the driver) in its one named thread-local (S0.07).
    Code outside any recorded site reports it as `outside`.
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
| `crates/foundation/phx-num/src/violation.rs` | `Violation { clause: &'static str, message: &'static str, keys: [(&'static str, i128); 8], n_keys: u8 }`; `violation!`; `capacity_exceeded!` |
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
  keys as `i128`. `capacity_exceeded!` does the same with its own payload. The application's panic hook, which runs
  on the panicking thread, adds the site by reading `phx_exec::site` directly (S0.07).
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
| `src/continuous.rs` | `normal` (Wichura's AS241 inverse CDF), `log_normal`, `pareto`, `gumbel`, `exponential`, `gamma` (Marsaglia and Tsang), `beta` (from two gammas), `weibull` (inversion) |
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
    `Tile`, `Region`, `Zone`, `Country`, `Market`, `Instrument`, `Opening` (a stratum and ordinal of an opening
    draw, before its party exists). An identity at or above 2⁶⁰ is a `capacity_exceeded!`.
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
- `normal_moments_and_tails`; `pareto_tail_index`; `gamma_beta_weibull_moments`.
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
| `src/descriptor.rs` | `ColumnDescriptor { name, elem_bytes, rows_per_chunk, fields: [FieldDescriptor] }` with `FieldDescriptor { name, offset, width, transform, tag: FieldTag }`, where `FieldTag` is `PartyRef`, `LineRef`, `InstrumentRef`, `TileRef`, `Day`, `Amount`, `Qty` or `Plain`; counters read bytes per row, and the Names family finds every reference by tag (S0.12) |
| `src/column.rs` | `Column<T: Pod>`: `len`, `push`, `extend`, `get`, `slice`, and `chunks_mut() -> impl Iterator<Item = ChunkMut<'_, T>>`, disjoint mutable views per chunk |
| `src/table.rs` | `Table`: columns sharing one slot space; `SlotAlloc`; `TableChunks`, the per-chunk views of all its columns at once |
| `src/arena.rs` | `ChunkArena`: 8-byte words; `ListRef { off: u32, len: u32, cap: u32 }` in words |
| `src/block_list.rs` | `BlockPool` of 16-entry blocks of `u32`; `BlockList { head: u32, tail: u32, len: u32 }`; `insert_sorted`, `remove_sorted`, iteration in order |
| `src/hash.rs` | SipHash-2-4 with 128-bit output, implemented here and checked against its published test vectors; `LogicalHasher`, which hashes live slots' columns and lists read through their `ListRef`s in slot order, never offsets, dead words, freed slots or page tails (S0.11) |
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
- `siphash_test_vectors`.
- `logical_hash_ignores_layout`: the same logical content in two arena layouts (before and after compaction) hashes equal.

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
| `src/site.rs` | the one named `thread_local!` of the engine: the current (day, sub-step, handler, chunk) of each thread — workers and the driver — read by the application's panic hook through `site::current()` |
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
- **Site** (§2.5): `site.rs` holds the engine's only thread-local, written when a worker starts a chunk and when the
  driver starts an apply, and read only by the panic hook.
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
with 8-bit digits. At most 44 sub-steps a day dispatch, and those with no handlers do not, which keeps barriers within
architecture §13.2's line.

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

### S0.08 — `phx-core` I: the calendar, conventions, schedules and the agenda

**Status**: planned

**Clauses**:
- STATE: TIME.1, TIME.2, TIME.3, TIME.4, TIME.5.
- FORBID: TIME.11, TIME.12.
- PRIMITIVE: TIME.13 *(part: epoch, calendars and conventions; decision schedules per kind are declared by each
  system)*.

**Architecture**: §3.3 (`phx-core`), §6.1, §7.3.

**Depends on**: S0.05, S0.07.

**Goal**: the one calendar of the world:
- business days per country from declared holiday rules;
- dates placed by advancing a date, never by counting days;
- the business-day and day-count conventions contracts name;
- decision schedules and wakes;
- the **agenda**, which says which rows act on which day, bounded in memory, with one calendar entry per row.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-core/src/calendar/mod.rs` | `Calendar`, `CountryCalendar`, `Calendar::plus(day, period)` |
| `src/calendar/rules.rs` | `HolidayRule`, `WeekendRule`, `easter_sunday(year)` |
| `src/calendar/period.rs` | `Period`, `EndOfMonth`, `advance`, `ScheduleDates` |
| `src/calendar/bizday.rs` | `BusinessDayConvention`, `adjust` |
| `src/calendar/daycount.rs` | `DayCount`, `day_fraction` (producing S0.03's `DayFraction` with its `per`) |
| `src/schedule.rs` | `DecisionSchedule`, `Phase`, `next_due`, `WakeKind` |
| `src/agenda.rs` | `Agenda`, `AgendaEntry`, `NextDays`, `TodayAgenda` |
| `data/world.toml` | `epoch`, `[[country]]`, `[[currency]]`, `[[unit]]` |
| `data/<country>/TIME.toml` | the weekend rule and holiday rules of each country, with sources |

**Design**

- **Calendar** (TIME.2):
  - `Calendar { epoch: Date, countries: [CountryCalendar] }` is built at assembly from `data/`.
  - `CountryCalendar` caches business days as a bitset over a window of declared years (an engineering constant in
    `consts.rs`). Beyond the window, answers are computed from the rules on demand. The window is extended at 10f on
    the first day of each year. The rules are the calendar's state; the bitset is derived and outside the world
    hash.
  - Queries:
    - `is_business(country, day)`;
    - `any_business(day)`;
    - `next_business(country, day)`, the first business day strictly after;
    - `on_or_after(country, day)`;
    - `next_turn_day(day)`, the first day after `day` that is a business day in any country (N8.2).
  - `Calendar::plus(day, period)` is the only way to add a period to a day.
- **Holiday rules** (ENDOWMENT, per country; Law 10):
  - `WeekendRule { days: [Weekday] }`.
  - `HolidayRule` is one of:
    - `Fixed { month, day }`;
    - `NthWeekday { month, weekday, n }`, where n = −1 means the last;
    - `EasterOffset { days }`, using the Gregorian computus (Meeus/Jones/Butcher);
    - `Substitute { of, when_on: [Weekday] }`, which moves a holiday falling on those weekdays to the next day that is
      neither a weekend day nor another holiday, resolved in declared rule order.
  - A country's business days are its non-weekend days minus its holidays.
  - The three countries' rules are drawn up at this step, shaped like real calendars and differing from one another
    (spec Appendix E 15), each with its `source_ref`.
- **Periods** (TIME.3):
  - `Period { months: u16, days: u16 }`, of which exactly one field is non-zero; `Period::weeks(n)` is 7n days.
  - Day periods are allowed only where the spec states days: settlement lags, notice in days, weekly schedules.
  - `advance(anchor, period, n, eom) -> Date` is the n-th date **from the anchor**. For month periods, both
    `EndOfMonth` modes cut the day to the target month's length; `Keep` also moves an anchor that is a month-end to
    the target's month-end.
    - 31 January + 1 month is 28 or 29 February in both modes.
    - With `Plain`, 31 January + 2 months is 31 March.
    - With `Keep`, 28 February 2023 + 1 month is 31 March.
  - `ScheduleDates { anchor, period, eom, convention, country }` iterates over the adjusted dates.
- **Business-day conventions**: `Following`, `ModifiedFollowing`, `Preceding`, `ModifiedPreceding` and `Unadjusted`,
  as ISDA defines them. `adjust(country, date, conv) -> Day`.
- **Day counts** (TIME.4):
  - `Act360`, `Act365F`, `ActActIsda`;
  - `Thirty360Bond`: if D1 = 31 then D1 = 30; if D2 = 31 and D1 ≥ 30 then D2 = 30;
  - `Thirty360E`: D1 = 31 becomes 30 and D2 = 31 becomes 30.

  `day_fraction(start, end, dc) -> DayFraction { num, den, per: Year }` is exact. `ActActIsda` across years sums
  `days_in_non_leap/365 + days_in_leap/366` over the common denominator 365·366.
- **Decision schedules and wakes** (TIME.5):
  - `DecisionSchedule { period: Period, convention: BusinessDayConvention, runs_on: Business | Any }` is declared per
    (kind, decision) as a PREFERENCE or TECHNOLOGY primitive.
  - A party's or cell's `Phase { offset_days: u16 }` per schedule is drawn at its creation from the stream
    `TIME.schedule_phase` (S0.10) as a uniform offset within the period.
  - `next_due(schedule, phase, after)`: the period's instances are anchored at the epoch through `advance`; the due
    day is the first instance start plus `offset_days` strictly after `after`, adjusted by the schedule's convention
    to a day its decision point runs.
  - `WakeKind` is one of `Message`, `Surprise`, `PlayerIntent`, `KinkDay` and `EventConcerning`.
- **The agenda** (architecture §7.3):
  - `NextDays`: per table, per declared reason (at most 16 per table, else `capacity_exceeded!`), one `u32` day
    column. A reason is a hazard or review process, a schedule or a wake.
  - Each row also has `booked: u32`, the day of its one calendar entry.
  - `AgendaEntry { slot: u32, table: u16, _pad: u16 }` is 8 bytes. There is **one live entry per row**, at `booked =
    min over its reasons`.
  - `set_next(table, slot, reason, day)` writes `NextDays`. If `day < booked`, it writes a new entry at `day` and sets
    `booked`; the old entry becomes stale. If `day ≥ booked`, it writes nothing.
  - At gather (1b), for each entry in today's bucket:
    - if its row's `booked` is today, the row is on today's agenda;
    - if `booked` is later, the entry is moved to that bucket, because the row's earliest reason moved later;
    - if `booked` is earlier, the entry is stale and is dropped.

    The row's reasons due today are those with `NextDays == today`, read from one cache line and returned as a
    `u16` mask.
  - Buckets are a **timing wheel**:
    - level 0 is 1 024 day buckets;
    - level 1 is 1 024 buckets of 1 024 days, each radix-sorted into level 0 when it comes due;
    - beyond that, an entry is re-booked when its level-1 bucket comes due.

    Buckets are `BlockList`s from a `BlockPool` (S0.06). A bucket whose dropped entries exceed its live ones is
    compacted.
  - `release(table, slot)` marks the row's entries stale for recycling (S0.06). `move_row(table, from, to)` re-books a
    row's entry for renumbering (S0.24).
  - Entries are added through the gather of the sub-step that sets them (S0.07), and each bucket is sorted by (table,
    slot) at gather, so `TodayAgenda { per_table: [(TableId, sorted slots, reason masks)] }` is canonical.

**Unit tests**
- `easter_known_years`: 1961-04-02, 2000-04-23, 2008-03-23, 2024-03-31 and 2038-04-25.
- `business_days_follow_rules`, and `substitutes_do_not_collide`: two weekend holidays in a row move to Monday and
  Tuesday.
- `advance_from_anchor_never_drifts` and `advance_eom_modes` (the three cases above).
- `adjust_modified_following_stays_in_month`: 31 May 2025, a Saturday, adjusts to 30 May.
- `day_fraction_known`:
  - Act/360 over 1 January to 1 July 2025 is 181/360;
  - Act/Act ISDA over 15 December 2023 to 15 January 2024 is (17·366 + 14·365)/(365·366);
  - `Thirty360Bond` over 31 January to 28 February is 28/360;
  - `Thirty360E` over 30 January to 31 March is 60/360.
- `beyond_window_matches_rules`: a day 40 years ahead is computed from the rules and agrees with an extended bitset.
- `next_due_respects_phase_and_convention`.
- `agenda_one_live_entry_per_row`, `agenda_later_moves_entry`, `agenda_stale_bounded_by_compaction`,
  `agenda_gather_is_canonical`, `agenda_far_entries_enter_wheel`, `agenda_release_and_move`.

**Live checks**: none. The world first runs at S0.11.

**Budget**:
- `is_business` is one bit read.
- The agenda needs 4 bytes per (row, reason) plus 4 for `booked`, plus 8 per entry. Stale entries stay at most at the
  live count by compaction. That is architecture §13.1's "Agenda" line: 85 MB at the design point.
- `gather_today` is about 20 ns per entry (architecture §13.2's "Agenda gather").
- Counters: `phx_core.agenda_entries`, `phx_core.agenda_moved`, `phx_core.agenda_stale`.

**Guards**: PC-17: no world crate outside `phx-core::calendar` calls `days_from_civil`, or adds an integer to a `Day`
except through `Calendar::plus`, `Day::succ`, or a declared day period (TIME.11, TIME.12).

**Not allowed**:
- a month taken as 30 days;
- a schedule computed from the previous adjusted date;
- a day count by counting periods;
- holidays typed as dates instead of rules;
- a second calendar;
- more than one live agenda entry per row.

**Done when**
- [ ] The calendar, conventions, schedules and agenda exist, with the tests passing.
- [ ] The three countries' calendar rules are declared with sources.
- [ ] PC-17 is registered.
- [ ] Two reviews are done.

---

### S0.09 — `phx-core` II: the register, policy values, kinds, facts, kind tables, the directory and findings

**Status**: planned

**Clauses**:
- STATE: NUM.3, NUM.4, PTY.1, PTY.4, PTY.6, PTY.8.
- MEASURE: NUM.7.
- FORBID: NUM.8, PTY.13, PTY.14.
- PRIMITIVE: NUM.9.
- PROCESS: PTY.9 *(part: beginnings and endings as kernel operations; estates are S0.25)*.
- INVARIANT: PTY.10 *(part: the directory; the family is S0.12)*.
- PRIMITIVE: PTY.15 *(part: legal forms)*.

**Architecture**: §3.3, §4.1, §4.6, §5.3, §7.2.

**Depends on**: S0.08.

**Goal**:
- the primitive register, from which every declared number is read;
- policy values that their owner may change with an announcement;
- kinds with legal forms;
- facts with one writer each, and the interface-item lists that make that checkable;
- kind tables of individuals, with their arena columns and facet columns;
- the party directory, whose identities are never reused;
- the findings store.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-core/src/register/mod.rs` | `Register`, `PrimDecl`, `Prim<T>`, `PrimValue`, the loader of `data/**/*.toml` |
| `src/register/values.rs` | `Scalar`, `Table1`, `Table2`, `Distribution`, `TypeSet`, `PointTable`, `CalendarRules` |
| `src/register/limit.rs` | `DeclaredLimit<T>`, `Bound<T>`, `TermsToken` |
| `src/policy.rs` | `PolicyValue<T>`, `Announcement<T>` |
| `src/kinds.rs` | `KindId`, `KindDecl`, `LegalForm` |
| `src/facts.rs` | `FactDecl`, `Fact<T>`, `ReprClass`, `Audience`, `ItemDecl` |
| `src/schema.rs` | `TableSchema`, the hook through which facts become columns of kind tables here and of cell tables at S0.21 |
| `src/kind_tables.rs` | `KindTable`: base columns, arena reference columns, facet columns |
| `src/directory.rs` | `Directory`, `PartyState` |
| `src/map.rs` | `KernelMap<K, V>`: hashbrown, fixed-seed foldhash, sharded by `mix64` over `KEYED_SHARDS`; `get`, `insert`, `remove`, `len`; no iteration; `drain_sorted` for saves |
| `src/findings.rs` | `Finding`, `FindingOwner`, `Findings` |
| `phx-macros/src/decl.rs` | `declare_fact!`, `declare_prim!`, `declare_kind!`, `declare_facet!` |

**Design**

- **The register** (NUM.3, NUM.8):
  - `PrimDecl { id, kind: PrimKind, unit, period: Missing<Period>, owner: SystemCode, decided_by: Missing<RoleId>,
    value_type, clause, shape: Missing<ShapeInfo> }`, where `ShapeInfo` is `Placeholder { retired_by: SystemCode }` or
    `Standing { reason }`.
  - Each `[[primitive]]` entry of §2.6 is checked against its declaration:
    - identity, kind and unit equal;
    - `decided_by` present exactly for POLICY;
    - `source` is one of the four;
    - `value` is of the declared type;
    - a SHAPE has its `shape` field;
    - `source_ref` is present.
  - An undeclared entry, or a declaration missing an entry for a country it applies to, stops assembly with the list.
  - Values are immutable after assembly and are read by `Prim<T>::get(&Register, country)`.
  - Standing SHAPEs are listed by `Register::standing_shapes()` for the report (NUM.7).
- **Value types**:
  - `Scalar(Fixed<E> | Money | Rate | Qty)`;
  - `Table1` and `Table2`, with declared axes and a declared rule outside the axes (edge value or refusal, never
    implicit);
  - `Distribution` over the families `Normal`, `LogNormal`, `Pareto`, `LogNormalParetoTail`, `Gamma`, `Beta`,
    `Weibull`, `Discrete` and `Empirical`;
  - `TypeSet { types: [(TypeId, share_ppm: u32)] }`, with shares summing to 10⁶ exactly (NUM.4);
  - `PointTable`;
  - `CalendarRules`.
- **`DeclaredLimit<T>`**: built only by `from_prim(Prim<T>)` or `from_terms(TermsToken, T)`. `TermsToken` is defined
  here with a constructor that PC-18 allows only `phx-ledger` to call. `bind(&self, wanted) -> Bound { taken, excess
  }` is `#[must_use]`.
- **Types at creation** (NUM.4): `draw_type(set, d) -> TypeId` is a weighted pick by shares, from each kind's stream
  `<SYS>.type_at_birth`.
- **Policy values** (architecture §4.6):
  - `PolicyValue<T>` holds its opening value and its announcements, as an arena list in a store (§16.3).
  - `Announcement { announced, effective, value }`; `value_on(day)` is the last effective on or before `day`.
  - `announce` refuses `effective < next_business(country, announced)` and any writer but the owner's decision.
- **Kinds and legal forms** (PTY.4, PTY.13, Law 10):
  - `KindDecl { id, name, legal_form, table: KindTable(TableId) | CellTable(TableId) }`.
  - `LegalForm { may_hold, separate_party, limited_liability, takes_deposits, endings, issues_currency, owners }` is
    POLICY data per country. Assembly refuses a legal form with no endings unless it issues its own currency
    (PTY.13, Law 13).
- **Facts and interface items** (architecture §4.1, Law 4):
  - `FactDecl { name, value_type, unit, kinds, writer: SystemCode | Placeholder { retired_by }, audience, repr: Key |
    Position | Profile | Individual, clause }`.
  - Every interface crate exports `pub const ITEMS: &[ItemDecl]`: facts, messages, rule signatures, line kinds and
    decision points. `phx-world` lists each crate's `ITEMS` beside the systems.
  - A system claims its facts in `declare`. Assembly refuses an item with zero or two claims, or whose writer is not
    registered.
  - `Audience` is `Party`, `Authority(KindId)`, `Public` (from the next sub-step) or `PublicAfter(Period)` (PTY.8).
- **`TableSchema`**: a hook that turns claimed facts into columns. Kind tables implement it here; cell tables implement
  it at S0.21 without reopening this step.
- **Kind tables of individuals**:
  - Base columns: `party: PartyId`, `kind: KindId`, `site: TileId`, `created: Day`, `type_ids`.
  - Arena reference columns for relationship rows, holdings, lots and named units (`ListRef`s into the table's chunk
    arenas, used by `phx-ledger` from S0.14).
  - Facet columns registered by `declare_facet!`.
  - Country, region and home currency are **read** through the site's zone (GEO.3, PTY.6). Whether a party has ended
    is read from the directory. Neither is stored twice (Law 4).
- **Directory** (PTY.1, PTY.9, PTY.10, PTY.13):
  - `Directory { next: u64, live: KernelMap<PartyId, RowRef>, ended: KernelMap<PartyId, Ended> }`, with `Ended { day,
    successor: PartyId, refs: u32 }`. The successor is the estate or the named successor, never absent (PTY.9).
  - `begin(kind, cause) -> PartyId` takes `next`, checked below 2⁶⁰, through the gather.
  - `end(id, day, cause, successor)` moves the party from `live` to `ended`.
  - Record kinds that may name parties call `retain(id)` and `release(id)`; `refs` overflow is
    `capacity_exceeded!`.
  - An `ended` record is dropped when its `refs` reach zero.
  - `lookup(id)` gives `Live(RowRef)`, `Ended { day, successor }`, or `Unknown`. `Unknown` for an id below `next` means
    nothing retains it, and any store or record naming it is a Names finding (S0.12).
  - `resolve(id)` follows successors until a live party, for PTY.10.
- **`Weight`** (PTY.14): the cell weight type is `Weight(u32)`, a count with no multiplication by anything but a count,
  defined here for `phx-pop`.
- **Findings** (II.5): `Finding { family, clause, owner: FindingOwner, size: i128, unit: Unit, day, detail }`.
  - `FindingOwner` is `Party`, `Tile`, `Line`, `Instrument`, `Market` or `Country`.
  - `Unit` is §2.5's.
  - Findings, metrics and run records are kept **outside the world**: they are not in the world hash, and no handler
    can read them (Law 17).
- **NUM.7**: `Register::placeholder_count()`, with the retiring systems, is a metric from S0.11, ratcheted.

**Unit tests**
- `register_refuses_undeclared_and_missing`.
- `register_refuses_wrong_unit_or_kind`.
- `policy_needs_decided_by`.
- `typeset_shares_must_sum`; `draw_type_matches_shares`.
- `policy_value_effective_dates`: effective the same day, or on a non-business day before the next business day, is
  refused.
- `declared_limit_binds_and_reports_excess`: binding 120 against 100 gives 100 taken and 20 excess.
- `directory_never_reuses`, `directory_resolves_successor_chain`, `directory_drops_unreferenced`.
- `kernel_map_has_no_iteration` (compile-fail).
- `legal_form_needs_ending`.
- `items_need_one_claim`.
- `weight_has_no_scaling` (compile-fail).

**Live checks**: none yet.

**Budget**:
- `Prim::get` is one indexed read; a directory lookup is one probe.
- Kind-table rows are architecture §13.1's "kind tables of individuals" line.

**Guards**:
- PC-18: numbers are read only through `Prim` or `PolicyValue`; `TermsToken::new` is called only in `phx-ledger`; no
  `toml` or `serde` outside §2.12's crates.
- Assembly refusals: undeclared or unclaimed items; a fact with zero or two writers; a SHAPE without its field; a
  legal form without an ending.

**Not allowed**:
- a default in a declaration;
- a primitive read as a literal;
- a fact written by two systems;
- iteration over a hash map;
- a party identity computed from a slot;
- a derived fact (country, currency, ended) stored beside its source.

**Done when**
- [ ] The register, policy values, kinds, facts, items, kind tables, the directory and findings exist, with the tests
  passing.
- [ ] The placeholder count is ratcheted.
- [ ] PC-18 is registered.
- [ ] Two reviews are done.

---

### S0.10 — `phx-core` III: the declaration vocabulary, streams, hazards, messages, decision points, rule handles, events and records

**Status**: planned

**Clauses**:
- STATE: CHN.1, CHN.2, OBS.1.
- PROCESS: CHN.4; CHN.3 *(part: the declared purposes of chance; each system declares its processes)*.
- FORBID: CHN.5, CHN.6.
- PRIMITIVE: CHN.8.
- MEASURE: CHN.7 *(part: realised rates are counted per process)*.
- OBS.3 *(part: the event store; the rule is S0.26)*; OBS.4 *(part: the decider fact and dispatch)*.

**Architecture**: §4.2, §4.7, §4.9, §4.10, §5, §6.2, §7.3.

**Depends on**: S0.09.

**Goal**: the vocabulary with which every system and kernel crate declares itself, in `phx-core`, so L1 and L2 crates
can use it:
- the `System` trait, `Declarations` and handler declarations, with their contexts;
- the sub-step table;
- audit-family declarations with a read-only context;
- opening contributions;
- the kink registry;
- named streams, hazards, occasions, messages, decision points, rule handles, events and records.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-core/src/system.rs` | `trait System` exactly as architecture §5.1; `Declarations`; `HandlerTable` |
| `src/substep.rs` | `SubStep`: 1a–10f exactly as architecture §6.1, each with its `business_only` flag, its kind (agenda, stream, index or sweep) and its ordinal |
| `src/handler.rs` | `HandlerDecl`, `HandlerId`, `declare_handler!`; `Ctx<'_, H>`; `Intent<T>` |
| `src/family.rs` | `FamilyDecl { name, owner, clause, mode: Streaming | Incremental | Rolling { cycle_days } }`, with the mode required; `trait Family`; `FamilyCtx`, which has only `&self` read methods |
| `src/contribution.rs` | `trait Contribution` (GEN phases, reads, writes, drawn and derived sides), registered through `Declarations` |
| `src/kinks.rs` | `KinkRegistry`: kinks declared per rule, contract term or constraint, on a position or per-member amount |
| `src/streams.rs` | `Purpose`, `StreamDecl`, `Streams::open(stream, subject, day, substep) -> Draws`, the only constructor of `Draws` |
| `src/hazards.rs` | `HazardDecl`, `DrawScheme`, `RateFn`, `ActsOn` |
| `src/occasions.rs` | `OccasionDecl` |
| `src/messages.rs` | `MessageKindDecl`, `Message`, `MessageStore`, `DayMessages` |
| `src/decisions.rs` | `DecisionPointDecl<I, O>`, `Decider`, `PlayerQueue`, `QueuedIntent`, dispatch |
| `src/rules.rs` | `RuleSig<I, O>`, `ctx.rule(sig, input)` |
| `src/events.rs` | `Event`, `EventStore` |
| `src/records.rs` | `RecordKindDecl`, `RecordStore` |
| `phx-macros/src/decl.rs` | adds `declare_stream!`, `declare_hazard!`, `declare_message!`, `declare_decision!`, `declare_rule!`, `declare_record!`, `declare_family!` |

**Design**

- **`Ctx<'_, H>`** is generic over the handler's declaration type `H`, which `declare_handler!` generates with
  associated types for its reads, direct writes, intents and streams. An accessor exists only for what `H` declares,
  so reading an undeclared column does not compile.
  - Direct writes are allowed only to the handler's own rows' columns that no other handler of the sub-step reads or
    writes (architecture §6.2). Everything else is an `Intent<T>`, applied at the sub-step's apply.
  - `ctx.draws(stream, subject)` calls `Streams::open` with the context's day and sub-step.
  - `ctx.decide(point, row, input)` dispatches (below).
  - `ctx.rule(sig, input)` calls a rule handle.
  - `ctx.party(row)` gives a scoped read (architecture §4.9).
- **Streams** (CHN.1, CHN.3, CHN.5, CHN.6):
  - `Purpose` mirrors CHN.3's list: `Mortality`, `Illness`, `Conception`, `Accident`, `Damage`, `Catastrophe`,
    `EquipmentFailure`, `Discovery`, `Meeting`, `Weather`, `TypeAtBirth`, `SchedulePhase`, `Occasion`, `Taste`,
    `Pairing`, `Sample` and `Lot`.
  - GEN's opening draws (GEN.3) have their own purpose, `Opening`.
  - There is no purpose for an outcome (CHN.5).
  - `Streams::open` is the one constructor of `Draws`. It is called by `Ctx`, and outside handlers by opening contexts
    (map generation, GEN), which carry declared opening sub-step ordinals.
  - Assembly refuses two streams of one name, and a stream whose name's FNV-1a collides with another's.
- **Hazards** (CHN.2):
  - `HazardDecl { name, acts_on: ActsOn, rate: RateFn, outcome, scheme: DrawScheme, stream, clause, source }`.
  - `ActsOn` is `Role(KindId, RoleId)`, `Party(KindId)`, `Tile`, `Region` or `Country`.
  - `RateFn` reads a table primitive at declared axes. An annual probability `q` becomes the daily `p = 1 − (1 −
    q)^(1/days_in_year)`, with the calendar year's own days, computed as `−expm1(log1p(−q) / days)`.
  - `DrawScheme` is `Scheduled { envelope: EnvelopeRule }` or `Daily`. `EnvelopeRule` bounds the rate over a row's
    profile values with `RateTable::max_over(values)`, a named operation.
- **Events** (CHN.4, OBS.3):
  - `Event { id, day, substep, kind, subjects: ListRef of phx_rand::Subject, details: ListRef of (Subject, i64 size in
    the kind's declared unit), public, develops_from: Missing<u64> }`, stored in arenas.
  - A catastrophe's struck tiles and severities are its details.
  - An occurrence writes its event in the apply of the sub-step that drew it.
- **Occasions** (REP.21): `OccasionDecl { point, kind: Review | Need | Meeting | Notice }`. A review names the
  `DecisionSchedule` of its cell's review days.
- **Messages** (architecture §4.2):
  - `MessageKindDecl { name, payload, lives_across_days, answering: [(addressee KindId, SystemCode, SubStep)],
    acceptance: Missing<(SystemCode, HandlerId)>, opens_commitment, pins, clause }`.
  - `Message` is 64 bytes: `id: u64`, `kind: u16`, `state: u8`, a pad, `sender: 16 B`, `addressee: 16 B`, `issued`,
    `due`, `concerns: u64` (tagged line or instrument).
  - Day-local kinds lapse at 1a; the others live in `MessageStore` and are saved.
- **Decision points** (architecture §4.7):
  - `DecisionPointDecl<I, O> { name, system, rule: fn(&I) -> O, eval: fn(&I) -> O, schedule: Missing<DecisionSchedule>,
    wakes: [WakeKind], runs_on_non_business: bool, clause }`. At least one of a schedule or wakes is required.
  - `Decider` is a fact of every party (`Rule | Player { delegate_when_unqueued: bool }`), written by the player's own
    settings (OBS.4).
  - `ctx.decide` uses the player's queued intent for that point when one is queued. Otherwise it uses the rule only if
    the player's decider delegates; if not, the decision is not taken that day.
  - `PlayerQueue` is filled by `phx-world`'s `run_turn(intents)` at 1c (S0.11).
- **Rule handles**: `RuleSig<I, O>` is declared in an interface crate with its implementing system. Assembly refuses
  zero or two implementers.
- **Records** (OBS.1, PTY.8, TIME.10):
  - `RecordKindDecl { name, schema, audience, horizon: Period, writer }`.
  - Entries are dated by (day, sub-step).
  - A reader sees only entries whose audience includes it, dated before its own sub-step. `PublicAfter(lag)` entries
    become visible on `day + lag`.
  - Records are world state: in the hash, and saved.
- **Kinks** (architecture §4.3, §7.6): every rule, contract term or constraint with a kink registers it here (a tax
  band on a year-to-date position, a means test, a credit limit, a payment due), so `phx-pop` and `phx-ledger` read
  kinks without depending on each other.
- **Families**: a family reads through `FamilyCtx` and writes only `Finding`s. `phx-audit` (S0.12) runs them, and
  kernel crates below it (`phx-geo`, `phx-ledger`, `phx-pop`) declare theirs here.

**Unit tests**
- `ctx_refuses_undeclared_column` (compile-fail).
- `stream_names_unique_and_fnv_distinct`.
- `purposes_closed` (compile-fail on a new variant used outside the enum).
- `rate_fn_annual_to_daily_exact`: for q = 0.01 and 365 days, 365 daily draws give q.
- `message_kind_refused_without_answerer`.
- `decider_dispatch`: rule; queued intent; delegation; no decision.
- `records_respect_audience_lag_and_substep`.
- `rule_sig_needs_one_implementer`.
- `decision_point_needs_schedule_or_wakes`.

**Live checks**: none yet.

**Budget**:
- `Streams::open` is O(1).
- Messages are 64 bytes each for kinds that live across days.
- Events are appended per chunk and gathered.

**Guards**:
- PC-19: no crate constructs `Draws` except through `Streams::open`, called only from `phx-core`'s `Ctx` and opening
  contexts.
- Assembly refusals: a stream twice; a hazard without a scheme or source; an unanswered addressee kind; a decision
  point without an evaluation form, or without a schedule or wakes; a rule signature with zero or two implementers;
  a family without a mode.

**Not allowed**:
- a stream shared by two processes;
- a draw with no purpose;
- an event recorded after a handler reacted;
- a record readable before its lag or its sub-step;
- a decision point with no pure evaluation form;
- system-facing vocabulary placed in `phx-world`.

**Done when**
- [ ] The vocabulary and every declaration kind exist and are refused when incomplete, with the tests passing.
- [ ] PC-19 is registered.
- [ ] Two reviews are done.

---

### S0.11 — `phx-world` I, `phx-cli` and the first live world

**Status**: planned

**Clauses**:
- PROCESS: TIME.6, TIME.8.
- N5 *(part: the one-worker guard)*.
- N8.8 *(part: sub-step and turn times are measured)*.

**Architecture**: §3.6, §5, §6.1, §6.2, §6.3, §14.2, §14.3.

**Depends on**: S0.10.

**Goal**: the world as one assembled program:
- the registry, schema compilation and the assembly refusals;
- the handler graph and the day runner, with every stage and sub-step of architecture §6.1;
- `phx-cli` with `phx run`, the `Inspector`, the live-check suite, the run-comparison guards and `phx measure
  calendar`.

The first live world has a calendar and no systems. Every later step adds to a world that already runs.

**Files**

| File | Purpose |
| --- | --- |
| `crates/assembly/phx-world/src/registry.rs` | `assemble(SYSTEMS, INTERFACES) -> Result<World, AssemblyErrors>` |
| `src/compile.rs` | compiles declarations into layouts, the stream registry, the register check, the kink registry and the handler graph |
| `src/refusals.rs` | every refusal of architecture §5.4 that assembly can decide |
| `src/day.rs` | `run_day`, `run_turn(&mut self, intents: &[QueuedIntent])` |
| `src/graph.rs` | `HandlerGraph` |
| `src/hash.rs` | `world_hash()` over logical content (S0.06's `LogicalHasher`) |
| `src/inspector.rs` | `Inspector`: read-only views of the world, its records, and the run's metrics and findings; only `&self` methods and no public fields |
| `src/metrics.rs` | run metrics (sub-step records, turn records, counters); outside the world hash and unreadable by handlers |
| `src/systems.rs` | `SYSTEMS` and `INTERFACES`, one line each; empty at this step |
| `crates/apps/phx-cli/src/main.rs` | `phx run --seed --days --settle --workers --checks --report --guards --read-trace`; `phx measure calendar` |
| `crates/apps/phx-cli/src/checks/mod.rs` | the suite: a hand-written `const CHECKS: &[Check]` of function pointers; `live_check!` defines one check's function and metadata |
| `crates/apps/phx-cli/src/guards.rs` | the run-comparison guards |
| `crates/apps/phx-cli/src/panic_hook.rs` | writes `violations/<run>.json` with the site from `phx_exec::site::current()` |
| `crates/apps/phx-cli/src/measure/calendar.rs` | the longest run of days with no business day anywhere, and each heavy coincidence (quarter-ends and paydays after holidays), to `perf/measure/S0.11-calendar.json` |
| `.github/workflows/ci.yml` | adds the job `live` |

**Design**

- **Assembly**: every `declare`, then every `handlers`, then compilation. Every refusal is reported at once, by item
  and system.
- **Sub-steps and the runner**, for each sub-step of `SubStep` in order:
  1. Skip it if it has no handlers — it does not dispatch — or if it is `business_only` and no country has a business
     day. On a partial business day it runs only for the countries that have one.
  2. Run its handlers over their traversals (S0.07).
  3. Gather their intents.
  4. Apply through the one apply routine (a stub that refuses instructions until S0.15).
  5. Record `SubStepRecord { day, sub_step, rows, bytes, barriers }` in the metrics.
- **Turns**: `run_turn(intents)` places the player's intents in the queue at 1c (TIME.5 wakes). It runs days from the
  day after the last turn up to `next_turn_day` (N8.2), and records `TurnRecord` in the metrics, with wall time
  through the application's `Clock`.
- **The handler graph** (architecture §6.2), within a sub-step:
  - refuse two handlers writing one (table, column or line kind) directly;
  - refuse a direct write that another handler reads;
  - intents never conflict.

  Canonical handler ids come from sorting by (system code, handler name).
- **`read-trace`**, a run-time flag in release builds:
  - traced: the first chunk of each (handler, table) per run, plus chunks with `index ≡ day mod 64`;
  - traced rows carry write stamps (sub-step, handler) from every writer, and every read is checked against the
    declaration and against later stamps (TIME.10);
  - every `Streams::open` is recorded unsampled per chunk, then gathered, sorted and checked for a duplicate (stream,
    subject, sub-step) (§2.16).
- **World hash**: logical content only — live slots, lists read through their references, and kernel-map contents
  sorted — with SipHash-2-4-128 (S0.06). Metrics, findings and derived indexes are outside it.
- **Guards** (architecture §14.3), sharing one baseline run:
  - `workers`: 1 worker against all workers, equal hashes every day (N5);
  - `shuffle`: the registration list reversed and rotated, equal hashes (§6.2);
  - `looking`: views, tracers, the audit and `read-trace` on against off, in one build, equal hashes (Law 17);
  - `streams`: one unused stream added, equal hashes (CHN.1);
  - `save`: added by S0.20.
- **The suite**: `CHECKS` lists every check by id. PC-20 requires every `live_check!` id to be in the list, every
  once-registered id to stay (retired with a reason), and `Inspector`'s public items to be `&self` methods.
- **CI `live`**: `phx run --seed 1 --settle <per-push> --days 60 --checks all --guards all --read-trace` at the
  declared per-push cell budget (architecture §14.7).

**Unit tests**
- `refusals_are_complete`: each refusal kind, over hand-built `Declarations` values (data, not a world).
- `graph_refuses_write_write_and_write_read`, `graph_allows_intents`.
- `canonical_ids_ignore_registration_order`.
- `substep_table_matches_architecture`: the table's business flags equal the list in architecture §6.1.
- `trace_sampling_is_deterministic`.

**Live checks**
- `LC-0-01`: every day has a `SubStepRecord` for every sub-step with handlers that should have run, and none for any
  that should not (TIME.6, TIME.8).
- `LC-0-02`: every turn ends on a day that is a business day somewhere, and covers every day since the last turn (N8.2).
- `LC-0-03`: `read-trace` found no undeclared read, no read of a later write and no duplicate stream open (TIME.10,
  CHN.6).
- `LC-0-04`: every record is dated with the day and sub-step that wrote it (TIME.9).
- `LC-0-05` to `LC-0-08`: the `workers`, `shuffle`, `looking` and `streams` guards hold.

**Budget**:
- An empty day dispatches no sub-step (none has handlers yet); `phx_exec.barriers` per empty day is ratcheted at 0.
- The empty world ≤ 50 MB resident.
- Wall-time targets are the phone's, judged at S0.26.

**Guards**:
- PC-20 as above.
- PC-21: every `SubStep` used by a handler exists in the table.
- Assembly refusals of architecture §5.4 that apply to what exists. The refusal "an invariant clause without a
  family" is `phx-check clauses`'s.

**Not allowed**:
- a sub-step order set by registration;
- a handler reading through a `&World`;
- a live check that writes or sets up state;
- a guard comparing less than the full logical hash;
- wall time, metrics or findings in the world hash or readable by handlers.

**Done when**
- [ ] `phx run` runs a year of empty days on the real calendar.
- [ ] LC-0-01 to LC-0-08 pass in CI.
- [ ] `phx measure calendar` reports the longest holiday block, and architecture §13.2's worst-turn row is updated from
  it in this step's commit.
- [ ] PC-20 and PC-21 are registered.
- [ ] Two reviews are done.

---

### S0.12 — `phx-audit`: the runner, independent records and the first families

**Status**: planned

**Clauses**:
- N1 *(part: the runner, independence and injection; each family completes with the step that builds its facts)*.
- INVARIANT: PTY.10; TIME.9, TIME.10 *(as the Time family, reading the run's stamps)*.
- Law 17 *(the audit never repairs)*.

**Architecture**: §3.3 (`phx-audit`), §15.

**Depends on**: S0.11.

**Goal**: the audit runs every declared family every close:
- streaming checks fused into the apply routine;
- incremental checks over rows touched today;
- rolling full cycles;
- independent records to check against;
- injection on copies, which proves that each family sees what only it should.

The first families are Names and Time.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-audit/src/runner.rs` | runs families at 10d: streaming results, incremental checks, one slice of each rolling cycle |
| `src/stream.rs` | the hook the apply routine calls per applied instruction |
| `src/records.rs` | independent records: per-batch digests, issuers' own totals, issued amounts (filled by later steps) |
| `src/touched.rs` | per table, the bitmap of rows touched today, kept by the apply routine |
| `src/names.rs` | the Names family |
| `src/time.rs` | the Time family |
| `crates/apps/phx-cli/src/inject.rs` | `phx inject --family <f> --from <save>`: loads a copy, applies the family's declared injection outside the ledger, runs the audit, and requires exactly that family to report |

**Design**

- **Families** are declared through `phx-core`'s `FamilyDecl` by the crate or system that owns their facts, each with
  a required mode (S0.10). A family reads only through `FamilyCtx` and writes only `Finding`s (Law 17).
- **Incremental** checks read rows touched today. **Rolling** checks cover `1/cycle_days` of the rows per day in slot
  order, the cycle declared per family (N8.6, N8.9).
- **Independence** (N1): each family declares an injection, a one-unit change to a fact only it owns. `phx inject`
  applies it on a **copy** loaded from a save and requires that family alone to report. Where several families can
  see a discrepancy, it is attributed to the family that owns the fact.
- **Names** (PTY.10):
  - Every field tagged `PartyRef` in any store (S0.06's descriptors), and every party named in a record, event or
    message, resolves by `Directory::resolve` to a live party.
  - Checked on rows touched today; on every holder of a party that ended today, through its lines' holder lists; and
    rolling over the rest.
- **Time** (TIME.9, TIME.10): every record, event and instruction carries the (day, sub-step) that wrote it, no later
  than the current one. `read-trace`'s findings, when on, are reported through this family.

**Unit tests**
- `family_ctx_has_no_writes` (compile-fail).
- `rolling_slices_cover_everything`.
- `names_refuses_dangling`, `names_follows_successors`, over hand-built values.

**Live checks**
- `LC-0-09`: every close ran every declared family, and the report lists zero findings (N1).
- `LC-0-10`: `phx inject` lights each family alone, on a copy of the day-30 save. It is not applicable until S0.20
  provides saves.

**Budget**:
- The audit ≤ 10% of the day's core time at every stage (counter `phx_audit.rows_checked`; the wall time is judged on
  the phone).
- Rolling cycles are declared per family.

**Guards**:
- PC-22: `FamilyCtx` exposes only `&self` methods (checked with `syn` on its impl blocks), and `phx-audit` imports no
  store mutator.

**Not allowed**:
- a family that fixes what it finds;
- a percentage tolerance (Law 7);
- a family checking a fact another family owns;
- an injection on the live run.

**Done when**
- [ ] The runner, Names and Time run every close in the live world.
- [ ] LC-0-09 passes; LC-0-10 is registered.
- [ ] PC-22 is registered.
- [ ] Two reviews are done.

---

### S0.13 — `phx-geo`: the map, weather and catastrophes

**Status**: planned

**Clauses**:
- STATE: GEO.1, GEO.2, GEO.3, GEO.6, GEO.7.
- PROCESS: GEO.10; GEO.8 *(part: catastrophe events on tiles; losses at owners are S0.23 and S0.25)*; CHN.3 *(part:
  weather and catastrophes)*.
- INVARIANT: GEO.11; GEO.12 *(part: the family runs; extraction arrives with GDS)*.
- MEASURE: CHN.7 *(part: weather and catastrophe frequencies)*.
- FORBID: GEO.14, GEO.15, GEO.16, GEO.17.
- PRIMITIVE: GEO.18 *(part: infrastructure arrives with FRT)*.

**Architecture**: §3.3 (`phx-geo`), §7.10; the owner's map decision (spec Appendix E 29).

**Depends on**: S0.12.

**Goal**: the physical world generated from the seed:
- about 40,000 land tiles of 10 km, across three countries with 12, 8 and 5 regions and 2,000 zones;
- terrain, deposits, exposure per hazard, and climate;
- distances over real paths;
- daily weather per region, and catastrophes on tiles, as dated events.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-geo/src/tile.rs` | `Tile` (12 bytes): `elevation_m: i16`, `surface: u8`, `terrain: u8`, `climate: u8`, `_pad: [u8; 3]`, `zone: MaybeZone(u32)` (sea has none); coordinates are derived from the tile's id; country and region are read through the zone table |
| `src/zones.rs` | `Zone { region, centroid: TileId }`, `Region { country }` |
| `src/exposure.rs` | one exposure column per declared hazard (Law 10) |
| `src/generate.rs` | `generate_map(opening_ctx, &MapParams) -> Result<Map, Rejection>` |
| `src/noise.rs` | gradient noise over Philox, pure |
| `src/partition.rs` | seeded region growing with target sizes |
| `src/distance.rs` | `ZoneDistances`, per country; `path_length(a, b)` by A* on demand |
| `src/deposits.rs` | `Deposit { tile, resource, grade: Fixed<3>, opening: QtyRaw or unbounded, extracted: QtyRaw }` |
| `src/stock.rs` | `StockByTileClass` and the (zone, class) index of holdings, empty until S0.25 |
| `src/weather.rs` | the 3a handler |
| `src/catastrophe.rs` | the 3a handler |
| `src/audit.rs` | the GEO.11 and GEO.12 families, declared through `phx-core` |
| `data/shared/GEO.toml` | projection, tile size, grid, sea level, roughness, octaves; land shares 48/32/20 (derived from the owner's 12/8/5 regions, so regions are of like size); zones per country in the same shares; construction conditions; climate classes and their seasonal parameters; exposure tables by terrain, elevation, water and climate (TECHNOLOGY); resource kinds with grade distributions and densities per terrain (ENDOWMENT) |

**Design**

- **Grid and projection** (GEO.1, GEO.2): a local planar projection in metres, with 10 km tiles (RESOLUTION). The
  rectangle `W × H` is declared so that land is about 40,000 tiles at the declared sea share. Adjacency is the
  8-neighbourhood. Coordinates derive from the id.
- **Generation** (GEO.10), in an opening context with its declared sub-step ordinal, from the stream `GEO.map`, with
  subject `World(attempt)`:
  1. **Elevation** from gradient noise, with the SHAPE parameters declared as standing SHAPEs.
  2. **Sea** by the sea level; **terrain** from elevation and slope; **climate** from latitude, elevation and distance
     to sea, each by a declared table.
  3. **Countries** by seeded region growing over land, to the declared shares. Seeds are drawn over the largest
     landmass. Islands (land components below the declared minimum) join the country of the nearest mainland tile.
     Ties go by lot.
  4. **Regions** within each country, then **zones** within each region, by region growing that stops a zone at its
     declared maximum and merges any zone below its minimum into its smallest neighbour. The zone counts per country
     are declared.
  5. **Exposure** per hazard (GEO.7) and **deposits** per tile and resource (GEO.16). Assembly refuses a resource
     declared present on every terrain.
- **Construction conditions** are declared:
  - each country's mainland holds at least its declared share of its land;
  - each region is connected;
  - each zone is within its declared size range.

  A failed attempt is **rejected and recorded** with its condition, and the next uses the next attempt number. There
  are at most `MAP_MAX_ATTEMPTS` attempts, then `capacity_exceeded!`. Nothing is nudged (GEO.17).
- **Distances** (GEO.2): edge lengths are three-dimensional distances between tile centres, rounded to whole metres
  before summing. `ZoneDistances` are Dijkstra from each zone's centroid (the medoid of its tiles) within its country,
  run once after the map is accepted, in parallel: about 6 MB. `path_length` for individuals' sites is A* on demand,
  counted.
- **Weather** (CHN.3), at 3a every day:
  - per region, a latent Gaussian AR(1) per variable (persistence declared per climate class);
  - mapped through each variable's declared marginal: temperature normal; rain zero-inflated gamma; wind Weibull;
    sunshine beta; from the stream `GEO.weather`, subject `Region`;
  - a region's climate parameters are its tiles' parameters weighted by area;
  - recorded as a public record of the day.
- **Catastrophes** (GEO.8, CHN.3), at 3a:
  - each land tile has a daily origin probability per hazard, a declared function of its exposure;
  - the count of origins per (country, exposure class) is a binomial over its tiles, and the origins are uniform picks
    among them;
  - a footprint spreads to 8-neighbours breadth-first, each with the hazard's declared spread probability for the
    neighbour's exposure, in tile-id order, with draws from the catastrophe's stream;
  - each struck tile's severity comes from the declared distribution for its exposure;
  - the event records the struck tiles and severities as details.
- **Stock per (tile, class)** is created empty and filled from S0.25.

**Unit tests**
- `noise_is_pure_and_seeded`.
- `partition_reaches_shares_and_bounds` on a hand-made mask.
- `islands_join_nearest_country`.
- `dijkstra_matches_brute_force` on a 10×10 grid.
- `weather_marginals_and_persistence` over 10⁵ draws.
- `footprint_connected`.
- `origin_counts_by_exposure_class_exact`.

**Live checks**
- `LC-0-11`: every land tile's zone is in one region and country; every region has land; every site of any party lies
  in its country and region (GEO.11).
- `LC-0-12`: the rejection record lists every failed attempt with its condition; the accepted map meets every
  condition (GEO.10).
- `LC-0-13`: each region's realised weather is within z = 6.1 of its declared climate for the season, with the
  variance adjusted for the persistence (CHN.7).
- `LC-0-14`: catastrophe frequencies per hazard are within z = 6.1 of their declared rates over the nightly two-year
  run. It is not applicable per push.
- `LC-0-15`: GEO.12 — extracted plus remaining equals the opening quantity for every finite deposit.

**Budget**:
- Map generation ≤ 10 s on the phone, judged at S0.26; the x86 benchmark of this block's review was 0.6–1 s for the
  distances on 4 cores.
- The map ≤ 80 MB: tiles, exposure columns, distances and deposits.
- Weather and catastrophes ≤ 1 ms a day.
- Counters: `phx_geo.map_bytes`, `phx_geo.generation_attempts`, `phx_geo.astar_calls`.

**Guards**: PC-23: no crate but `phx-geo` writes a tile, zone or site; no ownership change writes a site (GEO.15); no
crate keeps map geometry of its own (GEO.14).

**Not allowed**:
- a hand-placed country, deposit or coastline;
- a retry that changes parameters;
- distances as grid steps;
- negative rain or wind clamped to zero — the marginal is declared instead;
- a fixed number of hazards in a type.

**Done when**
- [ ] The live world generates its map from its seed, and runs a year of weather and catastrophes.
- [ ] LC-0-11 to LC-0-15 pass (LC-0-14 nightly).
- [ ] PC-23 is registered.
- [ ] Two reviews are done.

---

### S0.14 — `phx-ledger` I: instruments, holdings, lines, relationship rows and the contract algebra

**Status**: planned

**Clauses**:
- STATE: REG.1, REG.2, REG.3, REG.4, REG.5, REG.6, REG.7, REG.8, REG.9, REG.10; REP.3 *(part: lines and rows; the
  cell side is S0.21)*.
- INVARIANT: REG.13, REG.14, REG.15; REP.31 *(part: line sides; attachments against profiles are S0.23)*.
- FORBID: REG.16, REG.17.
- PRIMITIVE: REG.18.

**Architecture**: §4.4, §4.5, §7.2.

**Depends on**: S0.13.

**Goal**: everything that can be held, and every contract, with one record per thing (Law 4, REG.14):
- instruments with issuers and issued amounts;
- holdings (pooled for cells, lots for individuals) with liens;
- individuals' named units of plant and dwellings;
- lines of identical contracts, with rows in holders' arenas and holder lists per line;
- the contract algebra;
- commitments.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-ledger/src/instrument.rs` | `InstrumentFamily` (Debt, Equity, FundUnit, Contract, RealAsset, Banknote); `Instrument { id, family, issuer: Missing<PartyId>, unit, ccy, issued: QtyRaw, terms: TermsId, state }`; the issuer is absent only for real assets (REG.16) |
| `src/holding.rs` | `CellHolding { instrument: u32, flags: u32, quantity: i64, pooled_cost: i64 }` (24 bytes); `IndividualHolding { instrument: u32, flags: u32, quantity: i64, lots: ListRef }` (28 bytes, padded to 32), with `Lot { acquired: Day, quantity: i64, cost: i64 }` in the lot arena; the instrument's holder list |
| `src/units.rs` | `NamedUnit { id: u64, kind: CapKind, site: TileId, condition: u8, service_day: Day }` for an individual's plant and dwellings (REG.9); cells count units by (zone, class) as holdings |
| `src/lien.rs` | `Lien { holder: PartyId, instrument, seq: u32, units, to: PartyId, chain: Missing<LienId> }`, keyed by (holder, instrument, seq); the holding carries a flag and the pledged total is read from the lien table |
| `src/algebra.rs` | legs: `FixedAmount`, `RateOnNotional { reference: Fixed | Floating { series, spread, reset }, day_count }`, `StepSchedule { steps: [(Day, Rate)] }`, `PayableInKind { instrument }`, `PerTime`, `Indexed { series, base }`, `Contingent { event, amount }`, `Delivery { unit, quantity }`; `Schedule`, `Seniority`, `Collateral { kind, zone, class }`, `PaymentOrder`; early termination; conversion or write-down; default definition |
| `src/terms.rs` | the terms interner: `TermsId(u32)`, reference-counted, sharded |
| `src/line.rs` | `LineKindDecl`; `Line { kind: u16, flags: u16, terms: u32, side_counts: [u32; 2], next_due: Day, holders: BlockList }` (32 bytes) |
| `src/rows.rs` | `RelRow { line: u32, count: u32, point: u16, record: u16, role: u8, flags: u8 }` (16 bytes) and the optional words per kind (`balance`, `pending`, `amount`), encoded as whole 8-byte words in the holder's arena (S0.06) |
| `src/holder.rs` | `trait HolderArenas`: a holder's row run, holdings, lots and named units, by `RowRef`; implemented by the kind tables (wired by `phx-world`) and, at S0.21, by the cell tables |
| `src/commitment.rs` | `Commitment { kind, parties: [PartyId; 2], legs: ListRef, creates: ListRef, retires: ListRef, expires: Day, state }`, with its lists in an arena |
| `src/audit.rs` | the Ownership and Contracts families |

**Design**

- **Instrument families are declarations** (Law 10, REG.18). A debt family names its legs; equity its class, votes and
  preferred dividend; fund units their fund; contracts their line kind; real assets their physical class. Generic code
  reads the legs.
- **Legs and dues** (REG.5):
  - A floating reference must be a series some market prints; the opening history's series count as printed (Law 3).
  - `due_on(terms, day, state, out: &mut DueBuf)` writes the legs due into a caller's buffer. It is pure and
    allocates nothing.
  - Step-up and payable-in-kind coupons are their own legs.
- **Holdings** (REG.1, REG.2, REG.4):
  - A cell's holding is one pooled lot (REP.8). An individual's is its lots, and its basis is read from them — never a
    second pooled figure (Law 4).
  - Liens live in the lien table, keyed by holder, instrument and sequence. Free units are quantity minus the pledged
    total, and pledging beyond them violates `REG.16`.
  - Each instrument keeps its holders as a `BlockList`.
  - Negative physical quantities or free units violate `REG.15`.
- **Named units** (REG.9): an individual's plant and dwellings are named units with site and condition, in its arena.
  A cell's are counts by (zone, class), holdings of the class instrument.
- **Lines and rows** (REP.3, REG.8, REG.14):
  - A line is one record. Its two sides are its holders' rows, each with a role (side and the role within the party)
    and a count.
  - The line keeps its side counts incrementally, and `next_due` from its schedule (S0.17 builds the day's due-line
    bitmap from it).
  - The asset on one side and the liability on the other are read from the same line's two sides.
  - Rows are reached only through `HolderArenas` by `RowRef`; nothing outside the arena names a position in it
    (architecture §4.5).
- **Holder lists**: `phx_store::BlockList`, with one `BlockPool` per shard chosen by `mix64(line)`, so parallel updates
  never share a pool and block ids never depend on workers.
- **Commitments** (REG.10, architecture §4.4): a commitment records, as its writer's declaration, the legs it
  contributes and the rows it creates or retires when drawn. It lives across days and is saved.
- **Families**:
  - **Ownership**: holdings sum to issued per instrument (REG.13), incrementally for instruments touched today and
    rolling for the rest.
  - **Contracts**: each line's side counts equal its rows; the asset on one side equals the liability on the other
    (REG.14).

**Unit tests**
- `due_on_fixed_coupon_bond`; `due_on_floating_uses_fixing_day`; `step_up_and_pik`.
- `amortising_schedule_sums_to_principal`.
- `pledge_refuses_more_than_free`.
- `line_side_counts_track_rows`.
- `holder_list_per_shard_pool_deterministic`: the same inserts across 1 and 8 workers give identical logical lists.
- `terms_interner_refcounts`.

**Live checks**: LC-0-16 (Ownership) and LC-0-17 (Contracts) are registered, and apply from S0.16.

**Budget**:
- `RelRow` is 16 bytes, plus 8 per optional word; holdings are 24 or 32 bytes; `Line` is 32 bytes. Architecture
  §13.1's lines are updated in this step's commit.
- A due computation per leg ≤ 50 ns; accruing rows' dues are counted separately from the settlement stream's reads.
- Counters: `phx_ledger.rows`, `phx_ledger.lines`, `phx_ledger.terms_interned`.

**Guards**:
- PC-24: no crate but `phx-ledger` writes a holding, row, line, lien or issued amount.
- Assembly refusals: a line kind without transfer requesters; a commitment kind without its legs; a floating leg over
  an unprinted series.

**Not allowed**:
- a second record of a contract or of a basis;
- an index into a holder's arena kept outside it;
- a holding without a holder, or a claim without an issuer;
- a grouping held in place of an instrument;
- per-kind code for a contract the algebra can express.

**Done when**
- [ ] The types, the algebra and the families exist, with the tests passing.
- [ ] The public-API snapshot is committed.
- [ ] PC-24 is registered.
- [ ] Two reviews are done.

---

### S0.15 — `phx-ledger` II: money, accounts, instructions and settlement

**Status**: planned

**Clauses**:
- STATE: MON.1, MON.2, MON.3, MON.16; SET.1, SET.2, SET.3; MON.4 *(part: banknotes as holdings; withdrawal
  decisions are S1.09)*.
- PROCESS: MON.6; SET.4, SET.5, SET.7; MON.5 *(part: single instructions; batches are S0.17)*; SET.6 *(part:
  declared order; the ring is S0.17)*.
- INVARIANT: MON.7, MON.8, MON.9; SET.8, SET.9; NUM.5.
- MEASURE: SET.10 *(part: gross, net and fails by cause; the ring is S0.17)*.
- FORBID: MON.11, MON.12, MON.13, MON.14; SET.11, SET.16.
- PRIMITIVE: MON.15 *(part: overdraft terms as contract terms; the central bank's are S1.10)*; SET.17 *(part:
  settlement conventions)*.

**Architecture**: §4.5, §6.4, §6.5.

**Depends on**: S0.14.

**Goal**:
- money as liabilities of named issuers;
- instructions that settle whole or fail visibly, through one apply routine;
- rounding that lands on named parties;
- fails recorded, with arrears written by each line kind's contract process;
- the Money, Flows and Units families.

**Files**

| File | Purpose |
| --- | --- |
| `src/money.rs` | money line kinds as data: reserves (CB ↔ bank), deposit kinds (bank ↔ depositor), the treasury account (CB ↔ treasury); banknotes as each central bank's instrument |
| `src/instruction.rs` | `Instruction { id: u64, reason: ReasonId, trade_day, settle_day, legs: ListRef }`; `LegRec { party: PartyId, account: AccountRef, qty: i64, denom: Denom, kind: LegKind }`, where `AccountRef` is `Line(LineId)`, `Instrument(InstrumentId)` or `Unit(u64)`, `Denom` is `Ccy` or `UnitId`, and `LegKind` is `Money`, `Units`, `Row`, `Transformation { accounts_for }` or `OpeningWrite` |
| `src/check.rs` | pure `check_legs(balances, limits, free_units, legs) -> Result<(), FailCause>` over slices |
| `src/apply.rs` | the one apply routine: check, apply all or none, record the fail, write accounting effects, feed the audit |
| `src/rounding.rs` | `RoundingLanding { convention: Round, residue_to: Payer | Payee | Named(PartyId) }` |
| `src/fails.rs` | `Fail { instruction, cause, due, line: Missing<LineId> }` |
| `src/contract_process.rs` | 2d: each line kind's generic contract process turns yesterday's fails on its lines into arrears and payment-record updates (SET.3, SET.16) |
| `src/effects.rs` | each reason's declared accounting effect on each side (revenue, expense, asset, liability, equity), emitted as events at apply for `phx-acct` (S0.19) |
| `src/audit.rs` | adds the Money, Flows and Units families |

**Design**

- **Money** (MON.1, MON.2, MON.11, MON.14): every balance is a row on a money line whose issuer is the line's other
  side. Banknotes are holdings of a central bank's note instrument, each with a named holder.
- **Overdrafts** (MON.3, MON.12): a negative balance exists only on a deposit row whose terms declare an overdraft or
  intraday facility, within its `DeclaredLimit` from terms. The negative balance **is** the loan: there is no second
  record (Law 4). Any other negative result fails the instruction.
- **Instructions** (SET.1, SET.2): numbered in gather order. Every leg names its party and denomination. A trade's
  commitment is held on both books between trade and settlement dates (REG.10).
- **The apply routine** (SET.4, SET.5, SET.11, architecture §6.4):
  1. `check_legs`;
  2. apply all legs, or none, writing a `Fail` with its cause;
  3. emit the reason's accounting effects;
  4. feed the audit stream.
  
  An instruction id applied twice violates `SET.11`. Money legs are refused outside the sub-steps of architecture
  §6.1, by a violation.
- **Order within a sub-step** (SET.6): instructions drawing on one balance apply in declared payment order, then id.
- **Between banks** (MON.5): the payment carries the reserve legs itself.
- **Creation and destruction** (MON.6, MON.8): these are the legs of the payment itself. The Money family checks that
  every change in money stock is matched by an issuer's own transaction.
- **Rounding** (MON.16): every computed amount is rounded by its declared convention, and the residue lands on the
  named party.
- **Transformations** (SET.9): a transformation leg names the party and what accounts for it (a way, a deposit, a
  purchase, a hazard event). An `OpeningWrite` names the GEN identity it served (S0.16).
- **Fails** (SET.3, SET.16) are state. Each line kind's contract process at 2d turns them into arrears and payment
  records, so every fail has an owner process from the first day, whether or not the line's system decides anything
  yet.
- **A party ending during a day** (SET.7): instructions naming it settle or are refused against its estate, through
  `Directory::resolve`.
- **Families**:
  - **Money**: MON.7, MON.8, MON.9;
  - **Flows**: both legs in one denomination each, instructions reconciling to holdings (SET.8, SET.9);
  - **Units**: opening plus in equals out plus closing per holder and asset per day (NUM.5 on units).

**Unit tests** (pure functions over slices)
- `all_or_none`.
- `negative_only_with_facility_and_is_the_loan`.
- `interbank_moves_reserves_equally`.
- `rounding_residue_lands_on_named_party`.
- `double_apply_violates`.
- `transformation_needs_source`.
- `contract_process_turns_fails_into_arrears`.

**Live checks** (applicable from S0.16)
- `LC-0-18`: Money — per issuer and currency, balances equal its liability; notes held equal notes issued.
- `LC-0-19`: Flows — legs sum to zero per denomination, except transformations and opening writes, which are checked
  against their sources.
- `LC-0-20`: Units — per holder and asset per day.
- `LC-0-21`: every fail has a cause, and its line kind's contract process recorded it at 2d (SET.3).
- `LC-0-22`: gross and net settlement and fails by cause are published per day (SET.10, part).

**Budget**: the apply is ≤ 30 ns per payment in parallel by target chunk (architecture §13.2); counters
`phx_ledger.payments_applied`, `phx_ledger.fails` by cause.

**Guards**: PC-25: money moves only through `apply`; no crate but `phx-ledger` writes a balance.

**Not allowed**:
- a silent negative balance;
- an overdraft recorded twice;
- a currency conversion inside a payment;
- a partial settlement;
- a fact a decision reads kept only in an instruction;
- a rounding residue on nobody.

**Done when**
- [ ] Money, instructions, the apply routine, fails with their contract process, accounting effects and the three
  families exist, with the tests passing.
- [ ] LC-0-18 to LC-0-22 are registered.
- [ ] PC-25 is registered.
- [ ] Two reviews are done.

---

### S0.16 — GEN I and the institutions

**Status**: planned

**Clauses**:
- STATE: GEN.1 *(part)*, GEN.5 *(part: contracts' terms)*.
- PROCESS: GEN.3 *(part)*, GEN.4 *(part: institutions' books)*; PTY.9 *(part: beginnings by GEN)*.
- INVARIANT: GEN.7 *(part)*.
- FORBID: GEN.11.
- PRIMITIVE: GEN.12 *(part)*.
- The opening parties of FRM, BNK and CB brought forward without behaviour (spec Part O).

**Architecture**: §10.

**Depends on**: S0.15.

**Goal**: GEN's framework — phases, contributions, drawn and derived sides, balancing through recorded opening
writes, reports — and the first real parties:
- each country's central bank, with its treasury account;
- its banks;
- its large firms as individuals;
- their opening balance sheets, equity accounts and contracts;
- the payments those contracts make on their dates.

**Files**

| File | Purpose |
| --- | --- |
| `crates/assembly/phx-world/src/gen/mod.rs` | runs the phases (Parties, PhysicalStock, Contracts, Balances, History) over the `Contribution`s registered through `phx-core` (S0.10) |
| `src/gen/sides.rs` | drawn and derived sides per line kind; largest-remainder apportionment with ties by lot; the report of differences |
| `src/gen/balance.rs` | balancing as `OpeningWrite` legs, each naming the identity it served and its counter-entry |
| `src/gen/report.rs` | `GenReport`: distributions and sources, balancing writes, apportionment differences, attempts |
| `crates/systems/sys-cb/`, `sys-bnk/`, `sys-frm/` | the crates with their declarations and `gen.rs`; placeholders each naming the step that retires them (S1.10, S1.09, S1.03) |
| `data/<country>/gen/{CB,BNK,FRM}.toml` | the distributions, with sources |

**Design**

- **Phases**: each contribution declares its phase, reads, writes and sides. Contributions in a phase run in parallel
  where their writes are disjoint.
- **Opening draws** use streams `<SYS>.opening` with the subject `Opening(stratum, ordinal)`, so a party's draws
  exist before its identity.
- **Institutions**:
  - central banks, banks, and firms above the declared size rank of each industry (drawn directly as individuals from
    the size distribution's top; the promotion machinery arrives at S0.24);
  - their plant and stocks;
  - their loans, deposits and interbank lines, with the lender side derived by apportionment over banks' drawn market
    shares (architecture §10.2).
- **Balancing** (GEN.4), as **opening writes** — a leg kind of the ledger (S0.15), used only before day one. Each
  names the party, the amount and the identity it served, and has its counter-entry:
  - every liability gets a holder, and holdings sum to issued;
  - deposits equal banks' liabilities, and reserves the central bank's;
  - every party's books close through an entry to its **opening equity**, which is how each equity account opens
    (ACC.4; the one place equity is computed from assets and liabilities);
  - contract dates fall on the calendar.

  Balancing sets no price or rate (GEN.11).
- **Dated flows**: the opening contracts fall due by their schedules from day one and settle at stage 7 as single
  instructions (batches arrive at S0.17). Fails become arrears through the contract process (S0.15).
- **Placeholders**: `sys-frm`, `sys-bnk` and `sys-cb` declare their opening contributions and facts. Each missing
  decision is a SHAPE placeholder naming the step that retires it, and the placeholder count records it (NUM.7).

**Unit tests**
- `largest_remainder_apportions_exactly`.
- `opening_writes_close_books`.
- `opening_subjects_distinct`.

**Live checks**
- `LC-0-23`: day one passes every family (GEN.7).
- `LC-0-24`: the GEN report lists every opening write with party, amount and identity, and each distribution with its
  source.
- `LC-0-25`: every opening contract's payments fall on business days by its convention.
- `LC-0-26`: liveness (N2): payments per day are above zero, and every fail has a cause.
- `LC-0-16` to `LC-0-22` now apply and pass.

**Budget**: institutions' rows are within §13.1's individuals line; GEN I's time is judged on the phone at S0.26.

**Guards**: PC-26: GEN writes only through contributions and opening writes, and no GEN code sets a price, rate or
quantity outside a declared distribution (GEN.11).

**Not allowed**:
- balancing that sets a rate or price;
- a distribution parameter chosen after seeing a run;
- behaviour in a placeholder beyond its declared SHAPE;
- a party without a site, a legal form or an owner.

**Done when**
- [ ] The live world opens with the institutions, balanced and reported, and pays its dated flows for a year.
- [ ] LC-0-16 to LC-0-26 pass.
- [ ] PC-26 is registered.
- [ ] Two reviews are done.

---

### S0.17 — `phx-ledger` III: the settlement stream, the fixed point, levies, standing and pooled flows, transfers and the waterfall

**Status**: planned

**Clauses**:
- PROCESS: MON.5 *(batches, net settlement, failure per payer)*; SET.6; TIME.7; REP.8 *(part: the pooled-flow rule as
  a pure function)*; L3 *(part: the waterfall's ranking)*.
- MEASURE: SET.10.
- The line transfer and the split at a kink (architecture §4.4).
- TAX.2 and TAX.7 *(part: the levy machinery)*.

**Architecture**: §4.3, §4.4, §6.5, §7.4.

**Depends on**: S0.16.

**Goal**: settlement at the world's scale:
- the due-line bitmap and one stream over every holder's rows;
- the greatest fixed point with banks' nets and intraday credit, and failure as a prefix of each payer's payment
  order;
- parallel application;
- levies per member;
- standing flows;
- the pooled-flow rule as a pure function behind a positions interface;
- line transfers, including the split at a kink by person;
- the estate waterfall.

**Files**

| File | Purpose |
| --- | --- |
| `src/due.rs` | 1b: the due-line bitmap from lines' `next_due`, and the advance of each due line's next date |
| `src/stream.rs` | 7a: one stream over every holder table's rows, holder-major, giving per (party, bank) debits, credits and the first failing row |
| `src/positions.rs` | `trait PayerPositions { weight, per_member_funds(bank), kinks_into(position, buf), standing_rate(row) }`, implemented by the kind tables (weight one) and, from S0.21, the cell tables |
| `src/pooled.rs` | `pooled(funds_pm, weight, rows_in_order, kinks) -> RowOutcomes`: pure |
| `src/fixed_point.rs` | 7b |
| `src/apply_batch.rs` | 7c: gathered by target chunk and applied in parallel; reserves once per bank by net |
| `src/split_request.rs` | `SplitRequest { holder, row, count, cause }` intents, turned into parts by `phx-pop` (S0.23) |
| `src/levy.rs` | `LevyDecl`; per-member computation; withholding as a split of the gross |
| `src/standing.rs` | standing-flow rates; the day's leg per paying row; pending on non-business days |
| `src/transfer.rs` | `LineTransfer`, `SplitAtKink` |
| `src/waterfall.rs` | the estate waterfall |

**Design**

- **1b**: every line whose `next_due` is today sets its bit in a bitmap (375 KB at 3 M lines, cache-resident). The
  line's next date is advanced by its schedule at 7's apply.
- **7a, the stream**: a sequential pass over every holder table's rows, holder-major. For each row whose line's bit is
  set:
  - its per-member amount is `point_table[point]` or `amount`;
  - levies are computed per member (withholding splits the gross into the net and the remittance);
  - the amount times the count is a **debit** if the holder is on the paying side and a **credit** otherwise, to the
    (party, bank) its payment order names.

  Debits are tested in the payer's declared payment order by `pooled()` through `PayerPositions` (REP.8, amended),
  giving the **first failing row**. Only per-(party, bank) records are kept: debit and credit totals, the first
  failing row, a removed flag and a worklist link — 72 bytes, about 70 MB at the design point, within the day buffers.
- **7b, the fixed point**: start from every payment standing. Iterate a fail-only worklist:
  - a payer whose funds (balance, facility and credits still standing) cannot cover its standing debits fails **from
    its first unaffordable row onward** (a prefix of its payment order, REP.8), which is monotone in its funds;
  - its due lines' other sides, found through the lines' holder lists, lose their credits, and those parties and their
    banks go on the worklist;
  - a bank whose net, after intraday credit (MON.3), cannot be covered has its customers' legs removed (MON.5), and
    their payers go on the worklist.

  The result is the **greatest** set of payments that can settle given one another, which includes every ring (SET.6,
  TIME.6).
- **7c**: surviving payments are gathered by target chunk and applied in parallel; reserves move once per bank by net
  (MON.5). Failed payers' payees are drawn by REP.23 where pairings were not recorded, and their legs fail visibly.
  Accounting effects are emitted.
- **Pooled flows** (REP.8):
  - `pooled()` tests each row in order against both the payer's per-member funds left by earlier rows and the reached
    members' own positions (share before the row plus the row's per-member amount), for every registered kink.
  - A funds kink fails the row and the rows after it.
  - Any other kink crossed (a band, a means test), on an outflow or an **inflow** (credits are tested on the payee
    side the same way), emits a `SplitRequest` for the reached members, and the payment itself proceeds.
- **Levies** (architecture §4.3): `LevyDecl { reasons, base, schedule, remitter, payee, collector_liability, order,
  rounding, follow_on }`. The amount is computed per member from its base, rounded per member by the levy's declared
  convention, then times the count (TAX.7). A collector's liability accrues on its accruing row until remitted
  (TAX.2).
- **Standing flows** (architecture §7.4): rates on rows. On a business day each paying row's amount is a leg of the
  day's batch; on a non-business day it is pending on the deposit row, or a banknote leg at 6c.
- **Line transfers**: `LineTransfer { line, from: RowRef, to: PartyId, count, reason }` moves count and balance pro
  rata by REP.9's `split_total`.
- **`SplitAtKink`** (SUP.2, SUP.5): per holder, over all its deposit rows at the failing issuer in the declared
  coverage order, the insured amount per member is `DeclaredLimit::bind(limit × holders)` against the member's
  balances. The insured part (bound × count, and the rest of the row's total to the claim line) moves to the receiving
  bank with its pending payments. This is exact for any total.
- **The waterfall** (L3): a pure function from an estate's realised assets and claims to payments:
  - secured claims are paid first from their own collateral's proceeds;
  - then classes in the country's declared order;
  - equal-ranking claims pro rata, with the residue landing on the party the law names (MON.16);
  - in each currency the estate holds, never converted (MON.13).

  Its instructions settle at stage 7.

**Unit tests**
- `fixed_point_is_greatest_with_prefix_rule`: the 60/50 case (funds 100, rows 60 then 50) and its 59 variant behave
  monotonically; a ring settles; a short payer fails exactly its dependants; results equal a brute-force greatest set
  on small graphs.
- `worklist_revisits_payees_and_banks`.
- `bank_net_removal_resettles`.
- `levy_per_member_times_count`: 7 members at 1 001, 10%, nearest rounding: 7 × 100 = 700 against the aggregate's 701.
- `withholding_splits_gross`.
- `pooled_inflow_band_split`.
- `split_at_kink_across_rows_by_person`: two rows at one bank, limit × holders consumed in coverage order, with totals
  not divisible by the count.
- `waterfall_secured_first_pro_rata_and_currencies`.

**Live checks**
- `LC-0-27`: every bank's reserve movement equals the net of its customers' applied payments (MON.5).
- `LC-0-28`: from the 7b records kept until the audit, recomputed on the state at the start of stage 7:
  - **soundness**: every settled payer could pay given the other settled payments;
  - **maximality**: every failed payer was short given them (SET.6).
- `LC-0-29`: levies: sampled levy amounts equal per member × count under their conventions. It applies from S1.11,
  when the first levy exists.
- `LC-0-22` now also publishes the closing ring's size (SET.10).
- Pooled flows, transfers, the split at a kink and the waterfall rest on unit tests until their first live users
  (S0.25, S1.09, S2.04, S2.08). Each user step's live checks cover them.

**Budget**:
- 7a ≤ 10 ns per row read (the x86 benchmark of this block's review: 8–11 core-ns with the bitmap);
- 7c ≤ 30 ns per payment in parallel;
- the per-payer records ≤ 72 bytes each;
- counters: `phx_ledger.rows_streamed`, `phx_ledger.payments`, `phx_ledger.fixed_point_iterations`,
  `phx_ledger.day_buffer_peak_bytes`, ratcheted.

**Guards**: PC-27: no materialised batch — no buffer sized by a batch's rows — in `stream.rs`, `fixed_point.rs`,
`apply_batch.rs` and `batch.rs`.

**Not allowed**:
- a gross bank matrix;
- a per-leg write in 7a;
- a payee reduction where the stream gives the totals;
- a levy on an aggregate;
- a transfer that moves a count without its balance;
- a waterfall out of order;
- `phx-ledger` naming a `phx-pop` type.

**Done when**
- [ ] The stream, the fixed point, the apply and the rest exist, and the institutions' dated flows settle through them.
- [ ] LC-0-27 and LC-0-28 pass; LC-0-29 is registered.
- [ ] Unit costs are recorded by instruction counts and judged on the phone at S0.26.
- [ ] PC-27 is registered.
- [ ] Two reviews are done.

---

### S0.18 — `phx-market`: the six forms

**Status**: planned

**Clauses**:
- STATE: MKT.1, MKT.2.
- PROCESS: MKT.3, MKT.4, MKT.6, MKT.7, MKT.8, MKT.9, MKT.10, MKT.11, MKT.12; MKT.5 *(part: the form; dealers' quotes
  are S3.06)*.
- INVARIANT: MKT.13, MKT.14.
- MEASURE: MKT.15.
- FORBID: MKT.16, MKT.17, MKT.18, MKT.19.
- PRIMITIVE: MKT.21.

**Architecture**: §8, §7.9.

**Depends on**: S0.17.

**Goal**: every price in the world comes out of one of six mechanisms, each implemented once and parameterised by
declared data. A match is an instruction, a meeting that does not clear records its failure, and prints and marks are
public records.

Each form forms live prices from the stage whose systems post in it (spec Part O, amended). Here each is proven at
logic level.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-market/src/market.rs` | `MarketDecl { key, form, meeting_days, convention, participants: rule handle, ticks or point table }` |
| `src/order.rs` | `Order { party, market, side, schedule or limit, timing: Continuous | AtTheClose, day, reason: DecisionPointId }` |
| `src/call.rs` | the call auction |
| `src/book.rs` | the continuous book and its closing call |
| `src/dealer.rs` | dealer quotes as orders posted by dealers' decision points; client requests to several dealers; inter-dealer trades |
| `src/posted.rs` | posted prices over `trait GroupDemand` (declared here; `phx-pop` implements it at S0.23) |
| `src/bilateral.rs` | the bilateral protocol over messages across days |
| `src/administered.rs` | a declared rate and the quantity it meets |
| `src/print.rs` | `Print { market, instrument, day, unit, ccy, quantity, price, form, matches: MatchSetId }`; marks and fixings |
| `src/failure.rs` | `MarketFailure { market, day, kind }`, published |
| `src/audit.rs` | the Prices family |

**Design**

- **One implementation per form** (Law 10). Orders carry their poster's reason; an order without a limit (MKT.16) or
  of "whatever is left at any price" (MKT.17) is refused, and the market never adds its own demand or supply.
- **The call auction** (MKT.3):
  - the price is where posted supply meets posted demand on the tick grid;
  - ties go to maximum executed volume, then minimum imbalance, then nearest the last print where one exists, then
    the lower price; with no last print, maximum volume, then minimum imbalance, then the lower price;
  - orders with limits strictly better than the price fill in full;
  - orders at the price are rationed pro rata or by declared priority, by largest remainder with ties by lot;
  - one print per meeting names its match set.
- **The continuous book** (MKT.4):
  - `Continuous` orders arrive in an order drawn by lot and trade at the resting price in price-time priority;
  - `AtTheClose` orders and the book's residue go to the **closing call**, whose price is the day's close.
- **The dealer market** (MKT.5): dealers' quotes are orders posted by their own decision points (DLR, Stage 3).
  Clients request from several dealers and take the best, ties by lot.
- **Posted prices** (MKT.6): sellers' points; buyers' group demand through `GroupDemand`; capacity with re-choice
  rounds by lot, counted; sales per seller cell as totals.
- **Bilateral** (MKT.7): the message protocol, each answer the answering system's decision.
- **Administered** (MKT.8): refused at assembly without a quantity response.
- **Prints and marks** (MKT.2, MKT.12, MKT.14): a print exists only with a match set; each form declares its day's
  mark; fixings are records labelled as fixings. Valuations are `phx-acct`'s (S0.19).
- **Failures** (MKT.10) are published, with consequences for the participants' systems to read.
- **Prices family**: every mark came from a print or a declared valuation; per match, quantity bought equals sold and
  money paid equals received.

**Unit tests**
- `call_auction_clears_known_book`, `call_tie_rules_with_and_without_last_print`, `call_better_limits_fill_in_full`,
  `call_rationing_largest_remainder_and_lots`.
- `book_price_time_priority`, `book_at_the_close_orders_form_close`.
- `posted_capacity_rechoice_by_lot`.
- `bilateral_protocol_states`.
- `no_overlap_records_failure`.
- `order_without_limit_refused`.

**Live checks**
- `LC-0-30`: the Prices family is clean every close.
- `LC-0-31`: every print traces to its match set, and every published failure to its meeting.
- `LC-0-32`: MKT.15's measures are published per market per day, for the markets that met.

**Budget**: call O(n log n); book O(n log depth); posted meetings within architecture §13.2's line. Counters
`phx_market.matches`, `phx_market.failures`, `phx_market.rechoice_rounds`.

**Guards**: PC-28: no crate but `phx-market` creates a `Print`; a valuation is not convertible to a print.

**Not allowed**:
- a price from a formula, target or another price's statistic;
- a spread on a mid;
- a market adding demand to clear;
- a print without a match;
- an order placed for a party by anything but its own decision.

**Done when**
- [ ] The six forms, prints, marks and failures exist, with the tests passing.
- [ ] LC-0-30 to LC-0-32 pass.
- [ ] PC-28 is registered.
- [ ] Two reviews are done.

---

### S0.19 — `phx-acct`: books, carrying bases, valuations and statements

**Status**: planned

**Clauses**:
- STATE: ACC.1, ACC.2, ACC.3, ACC.4, ACC.6.
- PROCESS: ACC.8, ACC.9; ACC.7 *(part: write-downs; provisions are S2.01)*; MKT.20 *(part: the valuer framework)*.
- INVARIANT: ACC.10, ACC.11, ACC.12.
- FORBID: ACC.13, ACC.14, ACC.15, ACC.16.
- PRIMITIVE: ACC.17.
- Groups (ACC.5) are S3.05.

**Architecture**: §3.3 (`phx-acct`), §8.

**Depends on**: S0.18.

**Goal**: every party's books are a read of the register and the contracts, under declared rules:
- accruals with receivables and payables;
- carrying bases and unrealised differences;
- equity accounts moved only by the accounting effects of named events;
- valuations by named valuers from prints;
- statements;
- the Accounts family, checking two independent records.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-acct/src/basis.rs` | `CarryingBasis`, chosen at acquisition within the legal form's permitted set (ACC.17), stored in the holding's or row's flags; the effective rate lives in the terms |
| `src/position.rs` | carrying values as reads |
| `src/accrual.rs` | receivables and payables from accrual events |
| `src/equity.rs` | `EquityAccount { balance: i64, ccy }` per party with owners, moved only by `EquityEvent`s from S0.15's accounting effects, accruals and revaluations the basis sends to equity |
| `src/valuer.rs` | `ValuerDecl { method: rule handle, inputs: print series, rounding }`; `Valuation { value, prints, ages, interpolated }`; valuations at 9a |
| `src/statement.rs` | statements as reads |
| `src/writedown.rs` | write-downs and reversals up to original cost |
| `src/audit.rs` | the Accounts family |

**Design**

- **Two records** (ACC.10): the equity account moves only by events: the accounting effects each reason declares
  (S0.15), accruals, and revaluations to equity. It opens at GEN's opening equity (S0.16). Assets minus liabilities
  are read from the register and contracts at carrying values. Their equality is the check, and neither is computed
  from the other after day one.
- **Carrying values** are reads:
  - fair value reads the mark, or a named valuer's valuation (MKT.20);
  - amortised cost reads the effective-interest schedule;
  - lower of cost and net realisable value reads cost and the mark;
  - cost less depreciation reads the declared method.

  An absent mark with no valuation makes the carrying value **absent**, and the party's ACC.10 check is reported as
  unreadable, never passed on cost or zero (NUM.8).
- **Income** only on delivery, accrual or receipt (ACC.13); a capitalised cost is not expensed again (ACC.14).
- **Cells** use weighted average (ACC.6) and pooled cost (REP.8). Households and estates have net worth as a read
  (ACC.4).
- **Foreign-currency positions** do not exist before FX (Stage 5), which adds the translation.
- **The Accounts family**: ACC.10, incrementally and rolling; ACC.11 per statement period; ACC.12 line by line.

**Unit tests**
- `bank_bond_two_bases`.
- `writedown_reversal_capped_at_cost`.
- `equity_moves_only_by_events`.
- `absent_mark_is_unreadable`.
- `statement_is_a_read`.
- `valuation_is_not_a_print` (compile-fail).

**Live checks**
- `LC-0-33`: Accounts is clean for every party with an equity account, from the institutions of S0.16 on.
- `LC-0-34`: receivables equal payables across the world (ACC.12).

**Budget**: carrying values on read; statements at period ends; 16 bytes of equity account per party with owners.

**Guards**: PC-29: no stored field in a statement; no crate but `phx-acct` writes an equity account; no equity event
without a declared accounting effect.

**Not allowed**:
- a stored value beside units that is not a read;
- smoothing;
- an equity account computed from assets minus liabilities after opening;
- a carrying value defaulted to cost when the mark is absent.

**Done when**
- [ ] Books, bases, valuations, statements and the family exist, with the tests passing.
- [ ] LC-0-33 and LC-0-34 pass.
- [ ] PC-29 is registered.
- [ ] Two reviews are done.

---

### S0.20 — Persistence and the save guard

**Status**: planned

**Clauses**:
- STATE: SET.12, SET.13.
- INVARIANT: SET.15.
- PRIMITIVE: SET.17 *(snapshot intervals and history horizons)*.
- N8.10 *(part: the save's time is measured and budgeted)*.

**Architecture**: §11, §13.3, §14.3.

**Depends on**: S0.19.

**Goal**: saves are the world's state itself:
- population stores written whole, sparse stores as increments between full saves, with compaction;
- allocator state saved;
- derived indexes rebuilt on load;
- retention of a full save and its latest increment;
- the world pauses at a day's close while a save is written;
- the `save` guard.

**Files**

| File | Purpose |
| --- | --- |
| `crates/assembly/phx-world/src/save/mod.rs` | `save(world, dir, kind: Full | Increment)`, `load(dir) -> World` |
| `src/save/manifest.rs` | format, build, register and policy hashes, seed, day, settings, per store its bytes, committed extents and logical hash |
| `src/save/sparse.rs` | increments for sparse stores (messages across days, commitments, records within their horizons, events, the directory's ended records): what changed since the last full save, kept as change logs by the stores |
| `src/save/retention.rs` | retention: the latest full save and its latest increment, plus the one being written; the older unit is deleted only after the new one's manifest is written and synced |
| `src/save/rebuild.rs` | rebuilding the landing index, holder lists and the agenda's buckets on load, from saved state |
| `crates/apps/phx-cli/src/guards.rs` | adds the `save` guard |

**Design**

- **What is saved** (SET.12, SET.16):
  - every store: columns, arenas and block pools, with their allocator state (free slots, block pools' free lists,
    interners' reference counts and free ids);
  - the directory, the register's policy values and announcements, messages that live across days, commitments,
    fails, pending amounts, records within their horizons, events, and the agenda's `NextDays` and `booked`.
  
  Random streams need nothing: draws are addressed by day, sub-step and identity, so the saved day is their position.
  The run's findings and metrics are saved beside the world, outside its hash.
- **Encoding**: columns through S0.06's transforms and zstd; arenas as raw words through zstd (the transforms do not
  help on interleaved records, as measured in this block's review); written in parallel.
- **Moments** (SET.12, N8.10):
  - saves are taken only at a day's close;
  - when the player saves, when the app is set aside, and at the declared interval (every simulated quarter, the
    owner's default);
  - increments on a declared day of each month that is not a month-end, compacted into a full save on a declared
    cycle.
- **SET.13**:
  - instructions live until the close;
  - each kind of history is kept for its declared horizon (RESOLUTION), dropped as it ages out, with the directory's
    `retain` and `release` kept in step;
  - a kept record naming an ended party reads as ended, through the directory.
- **Retention** (N8.4, architecture §11): the unit is a full save plus its latest increment. The older unit is deleted
  only after the newer is complete.
- **Load** verifies the manifest's hashes, refuses a save from another build or register, with the reason (N5), and
  rebuilds derived indexes from saved state. The rebuild time is counted.
- **The guard**: 30 days, save, load, 30 days has the same logical world hash as 60 days straight (SET.15).

**Unit tests**
- `manifest_roundtrip`.
- `sparse_change_log_replays_exactly`.
- `retention_keeps_full_with_its_increment`.
- `allocator_state_roundtrip`.
- `rebuild_equals_live_indexes`: over hand-built columns.

**Live checks**
- `LC-0-35`: the `save` guard (SET.15).
- `LC-0-36`: save sizes and write times are recorded. They are judged against 4 GB for two units, and against the save
  budget, at S0.26.
- `LC-0-10` (S0.12) becomes applicable and passes.

**Budget**: judged at S0.26 on the phone.
- Full save size ≤ 1.5 GB.
- The save's duration against the **owner's save budget** (N8.10), recorded as pending in §12 with a proposed value.
- Load rebuild ≤ 3 s.

**Guards**: the `save` guard runs in CI's `live` job from this step on.

**Not allowed**:
- a summary in place of state;
- a derived index saved and trusted without rebuilding;
- deleting a full save its increment needs;
- replaying instructions to rebuild state.

**Done when**
- [ ] Saves and loads are exact; LC-0-35 and LC-0-10 pass; LC-0-36 records.
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
| Save duration (N8.10) | **pending** — proposed: a full save within 5 s and an increment within 1 s on the phone, the world paused | — |
| Land shares of the three countries | derived, not asked: 48/32/20, in proportion to the owner's 12/8/5 regions, so regions are of like size | 2026-09-23 |
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
| TIME | S0.11 | 6, 8 |
| TIME | S0.12 | 9, 10 |
| TIME | S0.17 | 7 |
| PTY | S0.09 | 1, 4, 6, 8, 13, 14 |
| PTY | S0.12 | 10 |
| PTY | S0.25 | 2, 3, 5, 9, 11, 15 |
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
| SET | S0.15 | 1, 2, 3, 4, 5, 7, 8, 9, 11, 16 |
| SET | S0.17 | 6, 10 |
| SET | S0.20 | 12, 13, 15, 17 |
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
| OBS | S0.10 | 1 |
| OBS | S0.26 | 3, 4, 5, 9 |
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
