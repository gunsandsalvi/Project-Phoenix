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

A check may read an **experiment copy** (spec N6): a copy of the settled live world to which `phx experiment` applies
a declared intervention — a changed primitive or endowment, recorded with the copy — and runs beside the untouched
world at the same seed. The intervention is the experiment's, not the check's: a declared intervention on a copy is
not arranging the live world. It never places a decision for a party, falsifies a record or changes a rule.

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
> - a one-sided flow;
> - anything the change needs that the spec does not say, or says otherwise, without the spec amended in its own
>   commit.
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
> - comments that reference documents or history;
> - a decision the change takes that `docs/ARCHITECTURE.md` or this plan does not yet say, or now contradicts.
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
- Slot order is storage, never an order of outcomes. A gather collects in (chunk, handler) order for speed, but any id
  it assigns — an event, a message, a line created at a split, a new party — is assigned after its items are ordered
  by (the subject's permanent identity, the subject's own sequence), and every sort whose ties would fall back on
  input order carries those two as its last keys. Renumbering (S0.24) therefore changes no later state, which the
  identity hash checks (S0.06, S0.24).

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

### 2.21 Decisions and behaviour

The spec states each decision's inputs, never its answer (Part II). A rule's **form** is a design decision taken here,
grounded in the literature it names. **Every** decision rule's form is a standing SHAPE (spec Law 2 and Appendix C):
it is listed in `data/shared/SHAPES.toml` with its reason — no mechanism in scope derives how people decide — and its
source, and `phx-check` refuses a decision point whose form is not listed. Its **parameters** are primitives
(PREFERENCE, TECHNOLOGY or POLICY) in the register, with sources; no parameter is an outcome (a markup, a propensity
to consume, a default probability), which is either a position the party carries and updates or a read. Every rule is
a pure function with its evaluation form (REP.15).

- **Inputs are the spec's.** A rule reads every input its DECISION clause lists, and nothing a party could not know:
  its own state, public records after their lag, and what it bought or was sent (Law 12).
- **Reviewing costs something real** (REP.21, Law 14). Each lumpy decision's review cost is TECHNOLOGY in hours: a
  firm's come out of its staff's hours that day (so review is paid in capacity), a household's out of its adults'
  leisure (so it enters the value of leisure its next decisions read). A menu cost of changing a posted point is paid
  the same way. The cost is counted per decision kind.
- **Streams** are named `<SYS>.<process>` in each step, one per purpose (§2.16).
- **Counters** named in each step are ratcheted in `perf/ratchets.toml` (architecture §16.8).
- **Interface crates** follow architecture §3.1's order, with shared identifiers in `if-base`.
- **Unit tests** are pure functions over values they are handed, or compile-level refusals (§2.9); anything that needs
  a registry, a ledger or a world is a live check.
- **Placeholders** name the system and step that retire them; each retiring step says so.
- **Reviews and menus**, **streams**, **counters** and **interface crates** above hold for every step from Stage 1 on;
  Stage 0's steps meet them where they apply.

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
- INVARIANT: NUM.5 *(part: the arithmetic refuses mixing; the audit is S0.12)*; NUM.6 *(no non-finite number can
  enter a fixed-point value, and no store holds a float)*.
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
- MEASURE: CHN.7 *(part: the samplers are exact; the realised frequencies are measured live from S0.13 and
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
| `src/hash.rs` | SipHash-2-4 with 128-bit output, implemented here and checked against its published test vectors; `LogicalHasher`, which hashes live slots' columns and lists read through their `ListRef`s in slot order, never offsets, dead words, freed slots or page tails (S0.11); `IdentityHasher`, which hashes the same content with rows in permanent-identity order and every slot reference translated to the identity it points at, so two worlds equal but for storage order hash equal |
| `src/encode.rs` | `encode_column`, `decode_column` |
| `src/consts.rs` | reservation defaults, `ARENA_DEAD_FRACTION`, `ARENA_GROWTH` (5/4), `DEFAULT_ROWS_PER_CHUNK` (4 096), `ENCODE_BLOCK` (1 024), `FRAME_BYTES` (1 MiB), `VA_BUDGET` |

**Design**

- **Reservation**:
  - Each column reserves address space for its declared maximum rows and commits pages as it grows.
  - The page size is read once from the system when the backing is created; 16 KiB and 4 KiB are both handled.
  - Growth past the reservation is `capacity_exceeded!("column rows", declared, needed)`.
  - The total reserved address space stays within `VA_BUDGET`, a setting of the run mode: 64 GiB for play (well
    inside a 39-bit address space) and 512 GiB for the reference run on the large machine, whose 47-bit address space holds it (S0.24).
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
  - `ListRef { off, len, cap }` counts words, 12 bytes. Cell tables use the compact `CellListRef { off: u32, len: u16,
    cap: u16 }`, 8 bytes; a list that outgrows 16 bits sets `len` to its sentinel and moves its reference to the
    arena's overflow map, so no list size the world decides is bounded by the encoding.
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
- PRIMITIVE: TIME.13 *(epoch, calendars, conventions, and the schedule vocabulary each system declares its
  decision schedules with)*.

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
  - A party's or cell's `Phase { offset_days: u16 }` per schedule is a keyed draw (S0.10) from the stream
    `TIME.schedule_phase` with the subject (its identity, the schedule): a uniform offset within the period,
    recomputed when read, so it is never stored.
  - `next_due(schedule, phase, after)`: the period's instances are anchored at the epoch through `advance`; the due
    day is the first instance start plus `offset_days` strictly after `after`, adjusted by the schedule's convention
    to a day its decision point runs.
  - `WakeKind` is one of `Message`, `Surprise`, `PlayerIntent`, `KinkDay` and `EventConcerning`.
- **The agenda** (architecture §7.3):
  - `NextDays`: per table, per declared reason (at most 16 per table, refused at assembly beyond that), one `u32`
    day column. A reason is a hazard process, a schedule, a wake or **the review reason**: all of a row's review kinds
    share one reason, whose day is the earliest of their review days, computed from their schedules and the row's
    phases; all its wakes share another; all its continuous-decision schedules a third. A household table at Stage 1
    then needs about twelve reasons: five hazards, birthdays, reviews, wakes, schedules, carried occasions, kink days
    and standing-flow dues; a firm table about eight.
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
- **`DeclaredLimit<T>`**: built only by `from_prim(Prim<T>)`, `from_terms(TermsToken, T)` or
  `from_physical(PhysicalToken, T)`. `TermsToken` is defined here with a constructor that PC-18 allows only
  `phx-ledger` to call; `PhysicalToken`, for a capacity read from held units or physical stock, only `phx-ledger`
  (holdings) and `phx-geo` (stock). `bind(&self, wanted) -> Bound { taken, excess
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
  - **Keyed draws**: a stream declared `keyed` draws from (stream, subject) alone, as a pure function of identity,
    recomputed whenever read and exempt from the duplicate-open check (a schedule's phase, S0.08).
  - `Streams::open` is the one constructor of `Draws`. It is called by `Ctx`, and outside handlers by opening contexts
    (map generation, GEN), which carry declared opening sub-step ordinals.
  - Assembly refuses two streams of one name, and a stream whose name's FNV-1a collides with another's.
- **Hazards** (CHN.2):
  - `HazardDecl { name, acts_on: ActsOn, rate: RateFn, outcome, scheme: DrawScheme, stream, clause, source }`.
  - `ActsOn` is `Role(KindId, RoleId)`, `Party(KindId)`, `Tile`, `Region`, `Country` or `Holding(ClassId)` (units of
    a held class: plant failing, a vehicle's accident). All are screened at 3b by `phx-pop` and the kind tables; the
    owning system applies the outcome at 3e.
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
- **Stage 8's order** (TIME.6, architecture §6.1): at 8a banks post their money-market orders and the central bank its
  tender orders; at 8b the money market and the tenders meet together; at 8c their trades settle; at 8d the
  standing facilities, lender of last resort and the treasury's direct borrowing meet the requests made to them; at
  8e those settle; at 8f intraday credit closes. Each sub-step reads the reserve positions the one before left.
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
  sorted — with SipHash-2-4-128 (S0.06). Metrics, findings and derived indexes are outside it. The **identity hash**
  (S0.06's `IdentityHasher`) is computed on demand, for comparisons across storage orders (renumbering, S0.24).
- **Replay contexts**: an opening pass that redraws what an earlier pass drew from the same keys (GEN's pass B,
  S0.25) is declared a replay of it. Its opens are exempt from the duplicate-open check, and equality with the first
  pass is tested instead.
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
| `src/network.rs` | the declared extension point for the transport and power network, empty until S1.07 fills it (GEO.4) |
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
- The extension points later steps use: instrument events and the instrument's state (S1.11, S3.03 to S3.05) and
  covered quantities (S0.18, S3.05) (architecture §4.4, §4.5).

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
| `src/holding.rs` | `CellHolding { instrument: u32, count: u32, quantity: i64, pooled_cost: i64 }` (24 bytes), `count` the members holding it; `IndividualHolding { instrument: u32, flags: u32, quantity: i64, lots: ListRef }` (28 bytes, padded to 32), with `Lot { acquired: Day, quantity: i64, cost: i64 }` in the lot arena; the instrument's holder list |
| `src/units.rs` | `NamedUnit { id: u64, kind: CapKind, site: TileId, condition: u8, service_day: Day }` for an individual's plant and dwellings (REG.9); cells count units by (zone, class) as holdings |
| `src/lien.rs` | `Lien { holder: PartyId, instrument, seq: u32, units, to: PartyId, chain: Missing<LienId> }`, keyed by (holder, instrument, seq); an individual's holding carries a flag, and the pledged total is read from the lien table |
| `src/algebra.rs` | legs: `FixedAmount`, `RateOnNotional { reference: Fixed | Floating { series, spread, reset }, day_count }`, `StepSchedule { steps: [(Day, Rate)] }`, `PayableInKind { instrument }`, `PerTime`, `Indexed { series, base }`, `Contingent { event, amount }`, `Delivery { unit, quantity }`; `Schedule`, `Seniority`, `Collateral { kind, zone, class }`, `PaymentOrder`; early termination; conversion or write-down; default definition |
| `src/terms.rs` | the terms interner: `TermsId(u32)`, reference-counted, sharded |
| `src/line.rs` | `LineKindDecl`; `Line { kind: u16, flags: u16, terms: u32, side_counts: [u32; 2], next_due: Day, holders: BlockList }` (32 bytes) |
| `src/rows.rs` | `RelRow { line: u32, count: u32, point: u16, record: u16, role: u8, flags: u8 }` (16 bytes) and the optional words per kind (`balance`, `pending`, `amount`), encoded as whole 8-byte words in the holder's arena (S0.06) |
| `src/holder.rs` | `trait HolderArenas`: a holder's row run, holdings, lots and named units, by `RowRef`; implemented by the kind tables (wired by `phx-world`) and, at S0.21, by the cell tables |
| `src/commitment.rs` | `Commitment { kind, parties: [PartyId; 2], legs: ListRef, creates: ListRef, retires: ListRef, expires: Day, state }`, with its lists in an arena |
| `src/events.rs` | the instrument's events and its `state` (live, suspended, defaulted, ceased): this module is the state's one writer, applying the event intents systems declare; empty of event kinds until S1.11 adds maturities |
| `src/covered.rs` | `Covered<Qty>`: a quantity of free units held or borrowed, built only here, placing a commitment on those units |
| `src/audit.rs` | the Ownership and Contracts families |

**Design**

- **Instrument families are declarations** (Law 10, REG.18). A debt family names its legs; equity its class, votes and
  preferred dividend; fund units their fund; contracts their line kind; real assets their physical class. Generic code
  reads the legs.
- **Legs and dues** (REG.5):
  - A floating reference must be a series some market prints; the opening history's series count as printed (Law 3).
    The reference is a term fixed at origination, by the series' one identity. Until that series' fixings begin
    (the benchmarks' at S3.09), a reset reads its last published value with its age, a placeholder naming IDX; the
    fixings then continue the same identity, so no contract's terms change.
  - `due_on(terms, day, state, out: &mut DueBuf)` writes the legs due into a caller's buffer. It is pure and
    allocates nothing.
  - Step-up and payable-in-kind coupons are their own legs.
- **Holdings** (REG.1, REG.2, REG.4):
  - A cell's holding is one pooled lot (REP.8) held by `count` of its members, like a relationship row, so a member's
    quantity is quantity ÷ count (REP.9). An individual's is its lots, and its basis is read from them — never a
    second pooled figure (Law 4).
  - Liens live in the lien table, keyed by holder, instrument and sequence; an individual's holding carries a flag, a
    cell's pledges are found in the table. Free units are quantity minus the pledged total, and pledging beyond them
    violates `REG.16`.
  - **Covered quantities** (`covered.rs`): an offer of held units takes a `Covered<Qty>`, which only `phx-ledger`
    builds, from free units held or borrowed. Building one places a commitment on the units (REG.10) until its order
    lapses at 1a or its trade settles, so the same units cannot cover two offers.
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
- **Instrument events** (`events.rs`): each event kind (a maturity, a coupon, a default, a procedure's suspension) is
  declared by the system that decides it, as an intent this module applies. The instrument's `state` has this one
  writer (Law 4); every system reads it.
- **Families**:
  - **Ownership**: holdings sum to issued per instrument (REG.13), incrementally for instruments touched today and
    rolling for the rest.
  - **Contracts**: each line's side counts equal its rows; the asset on one side equals the liability on the other
    (REG.14).

**Unit tests**
- `due_on_fixed_coupon_bond`; `due_on_floating_uses_fixing_day`; `step_up_and_pik`.
- `amortising_schedule_sums_to_principal`.
- `pledge_refuses_more_than_free`.
- `covered_refuses_committed_units`: units covering one open offer cannot cover a second.
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
- The extension point later steps use: a trade's settlement releasing the commitment its covered offer placed
  (S3.05) (architecture §4.4).

**Architecture**: §4.4 (row legs), §4.5, §6.4, §6.5.

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
- **Row legs** (REG.10, architecture §4.4): a `Row` leg creates a row, retires one, or adds to an accruing row's
  balance on both sides of its line. It is how a drawn commitment writes: a composite instruction draws one at 5c,
  and a **match drawing a commitment at 6d** (S0.18) writes the commitment's declared `Row` legs in place of the
  money leg they replace, up to its undrawn limit, through this same apply routine.
- **Covered units** (S0.14): the instruction that settles a trade releases, in the same all-or-nothing apply, the
  commitment its seller's `Covered<Qty>` placed; a failed trade releases it too, so no unit stays committed to a
  trade that no longer exists.
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
- `row_leg_adds_to_both_sides`: a `Row` leg on an accruing line adds the same amount to both sides' rows.

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
- The extension points later steps use: procedure lines for a stay (S2.03, S2.11), pending legs for a closed bank
  (S2.08), indexed standing flows (S2.09) (architecture §4.4, §6.5, §7.4).
- TAX.2 *(part: the levy machinery)*; TAX.7 *(part: the levy machinery)*.

**Architecture**: §4.3, §4.4, §6.5, §7.4.

**Depends on**: S0.16.

**Goal**: settlement at the world's scale:
- the due-line bitmap and one stream over every holder's rows;
- the greatest fixed point with banks' nets and intraday credit, and failure as a prefix of each payer's payment
  order;
- parallel application;
- levies per member;
- standing flows, including flows indexed to a region's daily index;
- the pooled-flow rule as a pure function behind a positions interface;
- pending legs, the hook a closed bank uses;
- line transfers, including the split at a kink by person and the move to procedure lines that carry a stay;
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
| `src/standing.rs` | standing-flow rates, plain and indexed; the day's leg per paying row; pending on non-business days; kink days, at the index's envelope for indexed flows |
| `src/pending.rs` | the closed-issuer hook: legs fixed as pending at 7a, kept out of 7b, settled or failed later |
| `src/transfer.rs` | `LineTransfer`, `SplitAtKink`, `ToProcedureLine` |
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
  day's batch; on a non-business day it is pending on the deposit row, or a banknote leg at 6d.
- **Indexed standing flows** (architecture §7.4): a flow's declaration may name a **daily index per region** (a
  weather index such as degree-days, written by `phx-geo` at 3a, S0.13). The day's amount is the per-member rate × the
  holder's region's index that day × the members the flow reaches that day, computed in the 7a stream (or as pending,
  on a non-business day); nothing is written on the row. Its **kink day** — when the flow would carry a position
  across a kink — is computed at the index's **envelope**: the highest the index can reach over the rate's validity
  window, a declared bound of the region's climate (ENDOWMENT), never a realised future value. The booked day is
  therefore never later than the true one; on it the row is re-checked with the index realised so far and, if the
  kink is not yet reached, re-booked the same way.
- **Pending legs** (the closed-issuer hook, architecture §6.5): a leg is standing, failed or **pending**. At 7a a leg
  whose payer's or payee's deposit issuer carries the `closed` fact is fixed as pending. The fact is declared here and
  written by the system that registers as its writer (S2.08); with no writer registered, no leg is ever pending.
  - A pending leg is kept out of 7b: it is neither a debit nor a credit there, so it fails nobody and funds nobody.
  - Its amount is recorded as `pending` on the payer's deposit row, which the payer's funds exclude from every later
    payment until it settles or fails, and on the payee's row, where it counts for nothing until it settles.
  - It settles at the first stage 7 at which neither issuer is closed — its rows having moved to a receiving bank —
    or fails visibly where the transfer leaves its row on a claim line (MON.5).
  - A closed issuer's reserve account and every account a transfer leaves behind pass to its estate, which holds them
    from then on; the estate's sales and payments settle through them.
- **Line transfers**: `LineTransfer { line, from: RowRef, to: PartyId, count, reason }` moves count and balance pro
  rata by REP.9's `split_total`.
- **Procedure lines** (a stay): `ToProcedureLine { from: RowRef, count, procedure }` moves a debtor's rows to a new
  line of the same kind whose terms add the procedure's stay — dues suspended while the named procedure is open — with
  counts and balances moved as a line transfer. A line kind names the procedures that may request it. A stay
  therefore suspends only the moved rows, never a line other holders share.
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
- `indexed_flow_amount_and_envelope_kink_day`: the day's amount from a rate, an index and a count; the kink day
  booked at the envelope is never after the day the realised index reaches the kink, and a re-check re-books it.
- `pending_leg_outside_fixed_point`: over a given graph with one closed issuer, the pending legs neither fail nor
  fund anyone, and the payer's funds exclude them.
- `procedure_line_leaves_shared_line`: moving one holder's rows to a procedure line leaves the other holders' dues
  and the line's side totals as they were, less the moved count.

**Live checks**
- `LC-0-27`: every bank's reserve movement equals the net of its customers' applied payments (MON.5).
- `LC-0-28`: from the 7b records kept until the audit, recomputed on the state at the start of stage 7:
  - **soundness**: every settled payer could pay given the other settled payments;
  - **maximality**: every failed payer was short given them (SET.6).
- `LC-0-29`: levies: sampled levy amounts equal per member × count under their conventions. It applies from S1.11,
  when the first levy exists.
- `LC-0-22` now also publishes the closing ring's size (SET.10).
- Pooled flows, transfers, the split at a kink, procedure lines, pending legs, indexed flows and the waterfall rest on
  unit tests until their first live users (S0.25, S1.09, S2.03, S2.04, S2.08, S2.09). Each user step's live checks
  cover them.

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
- The extension points later steps use: the coupled call (S2.09), the linked call (S3.01, S3.02), matches drawing
  commitments (S2.02) and offers of held units as `Covered<Qty>` (S3.05) (architecture §4.4, §8).

**Architecture**: §4.4 (commitments drawn by matches), §8, §7.9.

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
| `src/order.rs` | `Order { party, market, side, schedule or limit, timing: Continuous | AtTheClose, day, reason: DecisionPointId }`; an offer of held units takes its quantity as `Covered<Qty>` (S0.14) |
| `src/call.rs` | the call auction |
| `src/book.rs` | the continuous book and its closing call |
| `src/dealer.rs` | dealer quotes as orders posted by dealers' decision points; client requests to several dealers; inter-dealer trades |
| `src/coupled_call.rs` | the call auction over a network of zones with line capacities (MKT.3's form with a network), solved exactly as a min-cost flow |
| `src/linked_call.rs` | the coupled call with a cost on an edge and a capacity on a node: calls keyed by borrower, linked by lenders' budgets (S3.01) |
| `benches/linked_call.rs` | the linked call's prototype on a declared network of the money market's size, measuring its unit cost |
| `src/posted.rs` | posted prices over `phx_core::GroupDemand` |
| `crates/kernel/phx-core/src/group_demand.rs` | `trait GroupDemand`, added to `phx-core` by this step: declared below both `phx-pop`, which implements it at S0.23, and `phx-market`, which reads it, so neither depends on the other out of layer order |
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
- **The coupled call** (MKT.3 over a network): one call for a set of zones joined by lines with capacities, each zone
  with its own offer and bid step curves on integer quantities and tick prices.
  - It maximises the traded surplus subject to every line's flow within its capacity in each direction. That is a
    min-cost flow — source to offer steps (cost: the offer price, which may be negative), steps to their zone, zone to
    zone along lines (capacity, cost 0), zone to bid steps (cost: minus the bid), to the sink — solved exactly by
    network simplex on integers, so the result is the optimum, not an approximation.
  - Each zone's price is its node's potential; zones with no binding line between them share one, and a congested
    line separates them. Where a potential is not unique, the call's tie rules above choose within its range; ties
    among equal steps go by lot from the market's declared stream.
  - Line losses are not in the auction: the network's owner buys them outside it (S2.09 does so at balancing), so the
    auction stays a min-cost flow.
  - One print per zone names the meeting's match set.
- **The linked call** (MKT.3 over a network of lenders and borrowers; the money market's at S3.01): the coupled call
  with two declared extensions — a cost on an edge and a capacity on a node — solved by the same network simplex.
  - Every capacity is a quantity of the one traded thing on integer lots: a lender's budget, a limit's headroom, and
    the cash a borrower's collateral raises in one segment, which the borrower assigns to that segment before the
    call at that segment's lender's haircuts. No edge multiplies a flow, so the call stays a pure min-cost flow.
  - It starts from the previous meeting's optimal basis, drops borrower nodes that bid nothing, and compresses each
    schedule to its breakpoints.
  - Each market's price is its node's potential, chosen within its range by the call's tie rules.
- **The continuous book** (MKT.4):
  - `Continuous` orders arrive in an order drawn by lot and trade at the resting price in price-time priority;
  - `AtTheClose` orders and the book's residue go to the **closing call**, whose price is the day's close.
- **The dealer market** (MKT.5): dealers' quotes are orders posted by their own decision points (DLR, Stage 3).
  Clients request from several dealers and take the best, ties by lot.
- **Posted prices** (MKT.6): sellers' points; buyers' group demand through `GroupDemand`; capacity with re-choice
  rounds by lot, counted; sales per seller cell as totals.
- **Bilateral** (MKT.7): the message protocol, each answer the answering system's decision.
- **Matches drawing commitments** (REG.10, architecture §4.4): a match in a posted or bilateral market whose buyer
  holds an undrawn commitment of the seller or of the market's sellers (a terms grant, from S2.02) becomes at 6d an
  instruction that draws it: the commitment's declared `Row` legs (S0.15) replace the money leg, up to its undrawn
  limit, and the rest pays as money. The match set records which matches drew which commitment.
- **Covered offers** (MKT.16, REG.16): an offer of held units in a call, book or dealer market takes a `Covered<Qty>`
  (S0.14), so an order cannot offer units its poster does not hold free or has not borrowed; the commitment lapses
  with the order at 1a or is released by its trade's settlement (S0.15).
- **Administered** (MKT.8): refused at assembly without a quantity response.
- **Prints and marks** (MKT.2, MKT.12, MKT.14): a print exists only with a match set; each form declares its day's
  mark; fixings are records labelled as fixings. Valuations are `phx-acct`'s (S0.19).
- **Failures** (MKT.10) are published, with consequences for the participants' systems to read.
- **Prices family**: every mark came from a print or a declared valuation; per match, quantity bought equals sold and
  money paid equals received.

**Unit tests**
- `call_auction_clears_known_book`, `call_tie_rules_with_and_without_last_print`, `call_better_limits_fill_in_full`,
  `call_rationing_largest_remainder_and_lots`.
- `coupled_call_matches_lp_small`: against a brute-force optimum on small networks, including negative offers.
- `congested_line_separates_prices`: two zones share a price until their line binds.
- `linked_call_matches_lp_small`: against a brute-force optimum, with edge costs and node capacities.
- `linked_call_warm_start_same_optimum`: starting from a given basis reaches the optimum a cold start reaches.
- `match_draws_commitment_up_to_limit`: a purchase within the undrawn limit writes `Row` legs, the excess money.
- `book_price_time_priority`, `book_at_the_close_orders_form_close`.
- `posted_capacity_rechoice_by_lot`.
- `bilateral_protocol_states`.
- `no_overlap_records_failure`.
- `order_without_limit_refused`.

**Live checks**
- `LC-0-30`: the Prices family is clean every close.
- `LC-0-31`: every print traces to its match set, and every published failure to its meeting.
- `LC-0-32`: MKT.15's measures are published per market per day, for the markets that met.

**Budget**: call O(n log n); book O(n log depth); the coupled call a network simplex over a few dozen zones and
their steps, ≤ 5 ms for all three countries' blocks of a day; posted meetings within architecture §13.2's line.
The linked call's prototype, on a declared network of about 20 000 edges and 6 000 posters, measures its unit cost
warm and cold, and its instruction count is ratcheted (`phx_market.linked_call_instructions`); each country's call
runs on one core, the countries in parallel, so its time counts in wall time undivided. Counters
`phx_market.matches`, `phx_market.failures`, `phx_market.rechoice_rounds`, `phx_market.commitments_drawn`.

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
- Groups (ACC.5) complete at S3.05, which writes the group fact.
- The extension point later steps use: a group's consolidated statement as a read (S2.07, S3.05) (architecture
  §3.3).

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
| `src/consolidate.rs` | a group's consolidated statement, a pure read over the members a group fact names |
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
- **Consolidation** (`consolidate.rs`): given the members a group fact names, it combines their statements,
  eliminates claims between members line by line from both sides of each line, and shows minority holders. It
  stores nothing; each member keeps its own books. The group fact is written from S3.05.
- **Foreign-currency positions** do not exist before FX (Stage 5), which adds the translation.
- **The Accounts family**: ACC.10, incrementally and rolling; ACC.11 per statement period; ACC.12 line by line.

**Unit tests**
- `bank_bond_two_bases`.
- `writedown_reversal_capped_at_cost`.
- `equity_moves_only_by_events`.
- `absent_mark_is_unreadable`.
- `statement_is_a_read`.
- `consolidation_eliminates_intragroup_loan`: over given members' statements with a loan between two of them.
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
- The save's duration against the **owner's save budget** (N8.10, §12): a full save within 5 s and an increment
  within 1 s on the phone, the world paused.
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

### S0.21 — `phx-pop` I: cell tables, keys, positions, steps and profiles

**Status**: planned

**Clauses**:
- STATE: REP.1, REP.3 (with S0.14's line records), REP.4, REP.19, REP.20, REP.32, REP.33.
- FORBID: REP.17; PTY.14 *(part: cells, where a weight is a count)*.
- INVARIANT: REP.14 *(part: profile counts sum to weights, every role)*.

**Architecture**: §3.3 (`phx-pop`), §4.5, §7.1, §7.2, §7.6 (steps), §7.8, §13.1.

**Depends on**: S0.20.

**Goal**: the tables the population lives in. For each population kind — household, household running a business,
small firm — a cell table holds:
- the landing-hot record;
- the positions as exact totals, with their steps;
- the interned key;
- profiles per role, counted jointly within declared groups;
- the holder-major arenas of relationship rows and holdings.

`phx-ledger` reaches cells only through the interfaces it declares (`HolderArenas`, `PayerPositions`), which the cell
tables implement here.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-pop/src/kind.rs` | `PopKindDecl { kind: KindId, roles: [RoleDecl], key: KeyLayout, positions: [PositionDecl], standing_rates: [RateDecl], profile_groups: [GroupDecl], review_kinds: [DecisionPointId], pins: [PinDecl] }`, assembled from every system's contributions through `d.pop_kind(kind)`, a builder whose `.key_attr`, `.position`, `.standing_rate`, `.profile_group`, `.review_kind` and `.pin` record the declaring system as each item's writer (Law 10) |
| `src/table.rs` | `CellTable`: the columns below, over `phx-store` |
| `src/hot.rs` | `HotRecord` (64 bytes, `#[repr(C)]`, `Pod`): `landing_key: u64` (hash), `key_id: u32`, `weight: u32`, `flags: u16`, `pad: u16`, `step_vec_lo: [u16; 8]` (the leading positions' steps), `individual_ext: u32`, `lead: [i64; 3]` (the three leading position totals, which landing and screening read most) |
| `src/sig.rs` | the kink signature: its width per kind compiled from the kink registry as Σ ⌈log₂(bands + 1)⌉ over (position, rule), in whole `u64` words kept beside the positions, outside the hot record |
| `src/key.rs` | `KeyLayout`; `KeyRecord` (bit-packed attributes, up to 32 bytes); the sharded interner `KeyInterner` (hash to `key_id` with reference counts, updated by keyed reduction) |
| `src/position.rs` | `PositionDecl { name, unit, scale: ScaleRef, steps: StepTable, kinks: [KinkRef] }`; position columns as `i64` totals |
| `src/steps.rs` | `StepTable { boundaries: Box<[i64]> }`, a non-uniform base partition on the member's own scale, and coarser levels made by merging adjacent pairs; `step_of(per_member_scaled) -> u16` |
| `src/scale.rs` | `ScaleRef`: which position or flow a position is measured against (own outgoings, income, sales), REP.20 |
| `src/profile.rs` | per role, per declared group: joint value counts as a compact list (dense small histogram, or `(value_code, count)` pairs with delta-varint coding) in the chunk arena |
| `src/holder.rs` | `impl phx_ledger::HolderArenas` and `impl phx_ledger::PayerPositions` for `CellTable`; `impl phx_core::TableSchema` for `CellTable` |
| `src/individual.rs` | the extension facet for individuals of population kinds (weight one, flagged) |
| `src/audit.rs` | the Representation family, part one: profile counts sum to the weight in every role; the weight is positive |

**Design**

- **Declared per kind** (REP.33, Law 10): which attributes are key, position or profile; the roles; the profile
  groups within each role; the positions with their scale, steps and kinks. `phx-pop` never names an attribute;
  systems declare them in their interface crates and `phx-world` compiles the layouts (architecture §5.3).
- **The key** (REP.19): a bit-packed record of the declared key attributes, interned to a `key_id` with a reference
  count. Two cells share a `key_id` if and only if their keys are identical. The interner is sharded and updated only
  by keyed reduction (architecture §4.8). The banking arrangement is a key attribute of every population kind
  (architecture §4.5).
- **Positions** (REP.20): each is an `i64` total in its declared fixed-point unit. The member's value is total ÷
  weight, and its **scaled** value is that divided by the member's own scale, from which `step_of` reads the step.
  Read-positions (cash, wealth) are not columns: they are read from the cell's rows and holdings. Each deposit row's
  per-member balance and pending amount count as positions with steps (§4.5 of the architecture).
- **Steps** (REP.4, amended): a declared base partition per position, non-uniform (finer near kinks and where a
  response is steep, REP.10). Level k merges pairs of level k−1. The current level per position is a RESOLUTION
  setting, changed only by tolerance control (S0.24).
- **Landing key**: `hash(key_id, step vector at the current levels, key-rule kink signature)`. The kink signature
  records, for every rule with kinks on a position (a tax schedule's bands, a means test, a borrowing constraint),
  which interval between its kinks the per-member value is in: ⌈log₂(bands + 1)⌉ bits per (position, rule), read
  from `phx-core`'s kink registry (S0.10). Its width is compiled per kind and grows in whole words; there is no fixed
  width to run out.
- **Scaled values** compare exactly: the step of a position is found by comparing the total T with boundary × S, where
  S is the member's scale total, in `i128`, since T ÷ W over S ÷ W is T ÷ S. A scale that is missing or not positive
  leaves the step missing, and a missing step is its own step value, never a default.
- **Profiles** (REP.32): per role and group, counts of members per joint value; the sum over values equals the
  number of members in that role (REP.14). The encoding is chosen per group by declared cardinality: dense
  histograms for small domains, and sorted `(value, count)` pairs, delta-varint coded, for large ones.
- **Rows and holdings** live in the cell's chunk arena (S0.06) in the layouts of S0.14.
- **The ledger's interfaces**: `phx-ledger` declares `HolderArenas` (S0.14) and `PayerPositions` (S0.17). `phx-pop`
  implements both for cell tables, as the kind tables do with weight one. This keeps the L1 order (`phx-ledger` before
  `phx-pop`) while the ledger's stream and pooled-flow test read cells' arenas, positions and kinks. The `Weight` type
  is `phx-core`'s (S0.09), a count with no scaling (PTY.14).

**Unit tests**
- `step_of_non_uniform_boundaries`.
- `merge_level_is_pairwise`.
- `landing_key_equal_iff_key_steps_and_signature_equal`.
- `signature_width_from_registry`: three bands on two roles' positions and a means test give ⌈log₂ 4⌉ × 2 + 1 bits.
- `step_of_scaled_exact_in_i128`, with a missing scale giving a missing step.
- `pop_kind_builder_records_writers`: two systems adding positions to one kind are both recorded, and a duplicate
  name is refused.
- `profile_encoding_roundtrip`, dense and sparse.
- `profile_sum_equals_role_count` after random adds and removes.
- `interner_refcounts_under_keyed_reduce`.

**Live checks**: registered here, applicable from S0.25 when the population exists.
- `LC-0-37`: in every cell, the profile counts sum to the weight in each role, every close (REP.14).
- `LC-0-38`: every cell's landing key equals the one recomputed from its key, positions and kinks (re-keying is never
  missed).

**Budget**:
- The household cell record is itemised here for Stage 0's declarations with the room Stage 1 fills; S1.12 gives the
  Stage 1 record (504 bytes, architecture §13.1):

  | Item | Bytes |
  | --- | --- |
  | Hot record, with the three leading position totals | 64 |
  | Other positions: year-to-date per adult role (2 × 2 × 8), five more totals | 72 |
  | Review exposures, ten lumpy kinds × 8 | 80 |
  | Standing rates per member, fourteen × 4 (`u32` fixed point, overflowing to the arena's side map) | 56 |
  | Attention per kind: the own rate and g_k, two `u32` fixed-point values (10 × 8) | 80 |
  | Kink signature, one word | 8 |
  | Arena references, twelve `CellListRef`s × 8 | 96 |
  | **Total** | **456** |

  Review phases are keyed draws, recomputed when read (S0.08), so they take no bytes.

  Any item a later step adds is counted against this table in that step, and the counter below is ratcheted.
- Profiles take 2 bytes per entry at 150 entries.
- `step_of` ≤ 10 ns; computing a landing key ≤ 100 ns.
- Counters: `phx_pop.bytes_per_household_cell`, `phx_pop.profile_entries_per_cell` (ratcheted).

**Guards**:
- PC-30: no crate writes a cell column except through `phx-pop`'s typed writes, which a system reaches only through
  its handler's `Ctx` for the items it declared through `d.pop_kind` (the same path facts take into kind tables).
- No system names another system's attribute.
- Every key, position and profile attribute is declared with its REP.33 class.

**Not allowed**:
- an averaged key attribute (REP.16);
- a weight that is not a count;
- a position stored per member instead of as a total;
- a kink computed ad hoc instead of registered.

**Done when**
- [ ] The tables, keys, positions, steps and profiles exist.
- [ ] `HolderArenas`, `PayerPositions` and `TableSchema` are implemented for cell tables.
- [ ] The tests pass.
- [ ] LC-0-37 and LC-0-38 are registered.
- [ ] PC-30 is registered.
- [ ] Two reviews are done.

---

### S0.22 — `phx-pop` II: screening, reviews and occasions

**Status**: planned

**Clauses**:
- PROCESS: REP.7, REP.12; REP.21 *(part: review days, review exposure and the count reviewing; attention as a
  decision is S1.01)*; REP.35 *(part: the wake; the surprise is S1.01)*.
- CHN.3 *(part: occasions, pairing draws and schedule phases for cells)*.

**Architecture**: §7.3, §7.5 (occasions), §6.1 (3b, 3d), §13.2.

**Depends on**: S0.21.

**Goal**: chance and occasions reach members, and only the rows that act are touched:
- scheduled screening at an envelope rate with thinning;
- daily screening only for dense processes;
- reviews on each cell's own review days, drawn from its review exposure, and woken by surprises;
- needs and notices carried as open business;
- overlapping occasions allocated by hypergeometric draws.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-pop/src/screen.rs` | the 3b handler over the agenda's candidate rows and the declared dense processes |
| `src/envelope.rs` | per (row, process): the envelope p̄, the next candidate day by `geometric(π̄)`, and redrawing when the weight rises or the envelope's validity window ends |
| `src/review.rs` | review days per (cell, decision kind) from `DecisionSchedule` and the cell's phase; review-exposure positions; the count reviewing per profile value |
| `src/occasion.rs` | `Occasion { row, decision, profile_combo, count, kind }` intents; carried needs and notices as pins (§4.2) |
| `src/overlap.rs` | 3d: allocating a member hit by several processes the same day, by hypergeometric draws in declared process order |
| `src/pick.rs` | which members a hit reaches: weighted picks over the profile counts (S0.04's Fenwick picks), jointly within the role's groups |

**Design**

- **Scheduled screening** (REP.7, architecture §7.3):
  - For each (row, process), `NextDays` (S0.08) holds the next candidate day, drawn as `1 + geometric(π̄)` with π̄ = 1
    − (1 − p̄)^W̄. W̄ is the weight rounded up to the next rung of a geometric ladder of ratio 5/4 (an engineering
    constant): a landing that raises the weight within its rung needs no redraw, since thinning absorbs the gap, at
    the cost of at most a quarter more rejected candidates.
  - **The envelope** p̄ is the largest daily rate over the row's profile values (`RateTable::max_over`, S0.10) and over
    the rate function's declared **validity window**: the days until the next date on which any input of the rate can
    change without a visit (the year boundary when age tables roll, a policy value's effective day, the next
    scheduled review of the row). Each `RateFn` declares those change dates, and the envelope is recomputed on the
    first of them. A change of profile values happens only at an event or a landing, which visits the row and
    recomputes the envelope then.
  - On a candidate day, accept with probability π/π̄, where `π = −expm1(Σ n_v · log1p(−p_v))`.
  - If accepted, draw the counts per value with `binomials_joint_at_least_one` (S0.04).
  - Then draw the next candidate.
  - A weight rising past its rung at landing, or the end of the validity window, redraws the next candidate. A rate
    or weight falling needs nothing: thinning absorbs it.
  - A hazard with several outcomes (an illness for a spell or for good) draws the joint counts per (value, outcome)
    from the multinomial conditioned on at least one hit, with the same thinning.
  - Rates are read from the hazard's `RateFn` at each profile value (S0.10), never cached across days, so no cache
    can go stale.
- **Daily screening** for processes declared `Daily` (dense: most rows hit most days): over the agenda's rows and the
  process's declared set, `Σ n_v · log1p(−p_v)` with one `expm1`, vectorised, then the same draws.
- **Reviews** (REP.21, amended):
  - Each (cell, decision kind) has its review days from the kind's `DecisionSchedule` and the cell's phase (drawn from
    `TIME.schedule_phase` when the cell is created). They are always days its decision point runs (TIME.8).
  - The **review exposure** position is the members' total of −ln(1 − a_t), summed daily from the cell's attention
    a_t — a position computed at the cell's visits and accrued lazily as rate × days (architecture §7.4). It is
    additive, has no steps and is not in the landing key: it adds at landing (REP.21).
  - Attention a_t is S1.01's decision. Until it exists the attention position is missing, exposure does not accrue
    and no review occasion is drawn: Stage 0's decision points run on schedules, needs and notices only.
  - On a review day, the count reviewing per profile value is `binomial(n_v, −expm1(−exposure ÷ W))`. The reviewers'
    share of the exposure leaves: the total falls by k/W of itself. Reviewers who act split out with zero exposure
    (S0.23 hands a split part a share of each additive position except this one); reviewers who do not act stay, and
    the cell's mean then stands for members whose true exposures differ — the approximation REP.21 accepts.
  - A **surprise** (VAL.4) that bears on a decision kind wakes the cell for that kind on the next day its point runs
    (REP.35), through the agenda.
- **Needs and notices** reach particular members on their own day. On a day their decision point does not run
  (TIME.8), those members carry the occasion as open business: a pin in their part's key (architecture §4.2).
- **Occasions** are emitted per (row, decision, profile combination) with counts; the systems evaluate them at 5c
  with counts (S0.10's decision points).
- **Overlaps** (3d): members hit by several processes on the same day are allocated by hypergeometric draws from each
  process's stream, in declared process order.
- **Events**: every hazard occurrence records its `Event` at 3b's apply, before any handler of a later sub-step reads
  it (CHN.4).
- **Measures**: per process, the members exposed and hit per day, feeding `CHN.realised_rate` (CHN.7).

**Unit tests**
- `envelope_scheme_matches_daily_binomial`: over 10⁶ simulated days for a cell of weight 170 with three profile
  values, the counts per value under the envelope-and-thinning scheme match daily binomial counts (chi-square), for
  constant and for falling rates.
- `redraw_at_validity_window_end_is_exact`.
- `weight_ladder_thinning_exact`: raising the weight within its rung without a redraw leaves the counts' distribution
  unchanged (chi-square against daily binomials).
- `multi_outcome_conditioned_on_a_hit`.
- `review_count_first_day_exact`: on a cell's first review day, with constant attention, the review count matches a
  daily Bernoulli per member exactly in distribution.
- `review_count_bias_measured`: over later review days, the upward bias from carrying the mean exposure (the
  concavity of 1 − e^(−x)) is measured against a per-member simulation and reported, not asserted away.
- `exposure_leaves_with_reviewers`.
- `overlap_allocation_sums`.
- `picks_joint_within_group`.

**Live checks** (applicable from S0.25)
- `LC-0-39`: CHN.7 — for every hazard, the realised hit rate over the run is within its sampling error of the declared
  rate, per profile value class (a miss is a finding).
- `LC-0-40`: REP.12 — no cell was visited on a day it had no agenda entry; the rows touched per sub-step match the
  agenda (the sweep ledger).
- `LC-0-41`: every hazard occurrence has its event recorded at the sub-step that drew it (CHN.4).
- `LC-0-42`: carried needs and notices were decided on the first day their decision point ran.

**Budget** (architecture §13.2):
- a scheduled candidate ≤ 180 ns in slot order, including the joint draw when it is accepted (most are: the envelope
  sits close to π); the review's prototype measured 174 ns on one x86 core, untuned;
- an agenda redraw ≤ 150 ns (the prototype: about 140 ns, of which the transcendentals are 60), with the weight ladder
  cutting redraws to the landings that cross a rung;
- a daily dense (row, process) ≤ 5 ns;
- counters: `phx_pop.candidates`, `phx_pop.redraws`, `phx_pop.dense_evals`, `phx_pop.occasion_groups`, ratcheted per
  day.

**Guards**: PC-31: no screening outside 3b and the agenda; a process without a declared draw scheme is refused at
assembly.

**Not allowed**:
- visiting every cell daily;
- a cached survival probability;
- an approximation of the binomial;
- a whole cell deciding a lumpy decision at once (REP.16);
- a review drawn on a day its decision point does not run.

**Done when**
- [ ] Screening, reviews, occasions and overlaps work, with the tests passing.
- [ ] LC-0-39 to LC-0-42 are registered.
- [ ] PC-31 is registered.
- [ ] Two reviews are done.

---

### S0.23 — `phx-pop` III: splits, parts, pooled flows, landing, re-keying and the seller spread

**Status**: planned

**Clauses**:
- PROCESS: REP.8, REP.9, REP.23, REP.36; REP.22 *(part: counts, capacity by lot and the seller spread; tastes are
  drawn by the choosing systems from S1.01)*; REP.24 *(part: tiles drawn when needed; wear and repair between
  condition classes are CAP's and HSG's)*.
- INVARIANT: REP.14 *(with S0.21's part: no split, landing or pairing draw moves a total)*.
- FORBID: REP.16.
- The extension points later steps use: key clocks on one shared agenda reason (S2.10, S2.11) and indexed flows in
  group aggregates (S2.09) (architecture §7.3, §7.4).

**Architecture**: §7.3 (the key-clock reason), §7.4, §7.5, §7.6, §7.9, §7.10, §13.2.

**Depends on**: S0.22.

**Goal**: members whose shared state differs split into parts, and parts land in one lookup, exactly:
- splits divide profiles, rows and balances jointly;
- pooled flows split only across kinks;
- parts land in batches per target, or cluster into new cells;
- rows whose steps change are re-keyed;
- pairings are drawn when they matter;
- seller cells spread their sales on their review days;
- a landing never moves a total and never crosses a kink.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-pop/src/part.rs` | `Part { origin: PartyId, seq: u32, from: Slot, weight: u32, key_rec, positions, profile entries, rows, holdings, pins }` in the day arena; `(origin, seq)` is its canonical identity, `from` only a locator |
| `src/split.rs` | `split(cell, counts per role and group) -> Part`: multivariate hypergeometric draws of profiles and rows jointly within each role's declared groups; balances divided by `split_total` (REP.9) |
| `src/pooled.rs` | turns the ledger's `SplitRequest` intents (S0.17) into parts: the reached members drawn from the row's count (REP.23), with their outcome |
| `src/group_demand.rs` | `impl phx_core::GroupDemand` (S0.18): pieces, groups and their budgets for the posted-price meetings, with indexed flows' rates kept per index |
| `src/pairing.rs` | REP.23: draw which members of a line side an event concerns, with their profile values; the drawn members split out |
| `src/landing.rs` | 10b: sort parts by landing key; per target, check and join in one pass; clusters for the unlanded |
| `src/check.rs` | the landing check: same `key_id`, same signature and the same full step vector, recomputed from each side's totals and scales (the hash only finds candidates); then the kinks of either side's lines, read from the candidate's contiguous rows |
| `src/join.rs` | the `Landing` instruction per part: weights, totals, profiles, rows (by line and role), payment records by the declared rule, holdings at pooled cost (REP.8, SET.1) |
| `src/rekey.rs` | re-keying rows flagged by applies whose steps changed, and by key clocks reaching their end |
| `src/seller_spread.rs` | REP.22, amended: the capacity-respecting spread over a seller cell's members on its review day, in phases (below) |
| `src/tiles.rs` | REP.24: the tile a unit stands on, drawn from the zone's stock per tile when something depends on it |
| `src/audit.rs` | the Representation family, part two: weights sum to populations (REP.13); lines' sides equal and attachments reconcile with profiles (REP.31); no total moved by a split or landing (REP.14) |

**Design**

- **Splits** divide:
  - profiles and rows jointly within each role's declared groups, by multivariate hypergeometric draws from the
    process's stream;
  - balances and totals by `split_total`, with the stayer keeping the remainder;
  - holdings at pooled cost by the same rule;
  - every additive position except review exposure, which the acting reviewers leave with at zero (S0.22).

  A split never moves a total (REP.14). A part's `seq` numbers the parts of one origin cell in the order its handlers
  emitted them, which is fixed by the sub-step table and the handler's own order, never by chunking.
- **Pooled flows** (REP.8, amended): the ledger's stream tests each row through `PayerPositions` and the pure
  `pooled()` rule (S0.17). A funds kink fails the row and the rows after it, in payment order. Any other kink crossed
  emits a `SplitRequest`, which this step turns into a part for the reached members, drawn from the row's count. The
  spread erased is recorded per flow (REP.15).
- **Landing** (10b):
  1. **Sort** parts by (landing key, origin identity, `seq`) (radix, S0.07), so no tie falls back on input order.
     A part whose count is its origin cell's whole weight (in the reference mode, every part) is not a new part: the
     cell re-keys in place, keeping its identity, and then lands as a whole cell if another shares its key.
  2. **Look up** each key in the landing index (sharded; S0.24 maintains it), which gives candidate cells in order of
     permanent identity (§2.17), never slot order: renumbering moves slots and must not move outcomes.
  3. **Check** the first candidate that passes: the same `key_id`, signature and full step vector, compared exactly
     (a hash match only proposes a candidate; the hot record holds only eight steps), then the line kinks of both
     sides — a payment due within the declared horizon, a credit limit, the insured limit — read from the
     candidate's contiguous rows and the part's own.
  4. **Join**: all parts bound for one target are checked against the target's state at 10b's start, then joined in
     one pass over its rows.
  5. **Clusters**: parts without a target are grouped by landing key; in canonical order (by `(origin, seq)`), each
     part joins the first new cell it passes the check with, or starts one. A new cell gets a permanent identity from
     the directory (S0.09) through the gather, in the same canonical order.
  6. **Streams**: draws in splits use the stream of the process that caused them; the check draws nothing; ties the
     rules leave go by `REP.landing_lot`, a declared stream (§2.16).
- **Re-keying**: an apply that moves a position across a step boundary flags the row. At 10b its landing key is
  recomputed and the index updated; then, if it now shares a key with another cell and passes the check, it lands
  there, as a whole cell (REP.28's "land together").
- **Key clocks** (REP.19): a key attribute may be declared a **clock** — a count of days or months to an end that
  the declaring system names (a credit record's horizon, a procedure's period). On its end day the members re-key in
  place to the attribute's declared end value, and land as above. All of a table's key clocks share **one** agenda
  reason (S0.08), whose day is the earliest end over the row's clocks, so a later system's clock adds no reason.
- **Indexed flows in group aggregates** (architecture §7.4, S0.17): a group's budget for a category bought by an
  indexed standing flow is kept as the sum of its pieces' rates per index; a meeting reads that sum × the group's
  region's index today. A group lies in one zone, so in one region, and no piece is touched when the weather moves.
- **Pairings** (REP.23): an event concerning some members of a line side draws them from the counts at that moment,
  with their profile values; they split out. Attachments in different roles are drawn independently.
- **The seller spread** (REP.22, amended), on a seller cell's review day, from the stream `REP.seller_spread`:
  - The days' sales since the last review are recorded by the meetings (S0.18) as counts of **purchases** per
    quantity: a buyer's purchase is the units it bought in one meeting, and it reaches one member.
  - Each purchase reaches a member drawn uniformly from those with units left, as REP.22 says; members are equally
    likely whatever their stock.
  - It is done in **phases**, exact in distribution against purchases arriving one by one in a uniformly random
    order. Let `u` be the fewest units any active member holds and `q` the largest purchase left. If `u ≥ q`, a phase
    takes `L = ⌊u ÷ q⌋` purchases (drawn from the remaining counts per quantity by multivariate hypergeometric draws)
    and deals each quantity class over the active members by an equal-probability multinomial. No member can run
    out inside the phase, so the phase is exact. If `u < q`, one purchase is drawn by lot and reaches a uniformly
    drawn member; a member that cannot fill it sells what it holds, closes, and the rest of the purchase chooses
    again among the others. Members at zero leave the active set before the next phase.
  - The phases per spread are counted (`phx_pop.spread_phases`, ratcheted), since many members near zero lengthen
    it.
  - Members end with their own revenue and units, and those whose positions leave their steps split into parts.
- **Tiles** (REP.24): when an event needs a unit's tile, it is drawn from the zone's stock of that class per tile
  (S0.13's store).

**Unit tests**
- `split_conserves_totals_and_profiles`.
- `split_joint_within_group`.
- `pooled_rule_cases`: the fourth-round reviewer's case fails the row; a small due is pooled; an inflow crossing a tax
  band splits the reached members.
- `landing_check_refuses_kink`: a part and a target on opposite sides of a payment due are refused.
- `join_adds_everything_exactly`.
- `landing_order_independent`: permuting the parts gives identical results.
- `clusters_canonical`.
- `seller_spread_never_exceeds_units`.
- `seller_spread_matches_uniform_sequential_purchases`: against a one-by-one simulation with purchases in random
  order and a uniform member among those with units left, the members' sales match (chi-square), including cells
  where members run out.
- `landing_independent_of_slots`: renumbering the table before 10b gives identical landings.
- `whole_weight_part_rekeys_in_place`: the cell keeps its identity.
- `check_refuses_hash_collision`: two parts with equal hashes and different step vectors never join.
- `rekey_on_step_crossing`.
- `key_clocks_share_one_reason`: over given clocks on a row, the reason's day is the earliest end, and the re-key at
  it sets the declared end value.
- `indexed_group_budget`: a group's day budget is the sum of its pieces' rates per index × the index.

**Live checks** (applicable from S0.25)
- `LC-0-43`: Representation — weights sum to each population, lines' sides equal, attachments reconcile with
  profiles, every close (REP.13, REP.31).
- `LC-0-44`: no landing crossed a kink: sampled landings are re-checked against both sides' kinks from the records
  (REP.8, REP.16).
- `LC-0-45`: REP.36 — for sampled landings, the totals of every straight rule (tax, interest, benefits, repayments)
  are unchanged at the moment of landing to within one smallest unit per member.
- `LC-0-46`: REP.15's costs are reported per day: the dispersion erased per landing and per pooled flow; splits,
  landings and new cells per day.

**Budget** (architecture §13.2), agenda redraws excluded (they are S0.22's line):
- a part end to end ≤ 2.5 µs at 40 rows per cell of 24 bytes, 150 profile entries and 30 positions, split as: rows
  divided 0.5; profiles and positions divided 0.5; the origin re-keyed 0.2; key, lookup and check 0.4; join and holder
  lists 0.9. The review's untuned prototype on one x86 core measured 12.1 µs (1.8, 2.0, 0.5, 1.0 and 4.0 for these,
  and 2.2 of redraws now moved out); this is a recorded risk (§11), judged on the phone at S0.26;
- a seller spread ≤ 3 µs per seller cell of 50 members, about 49 binomials a phase (the prototype: 4.1 µs, untuned),
  with the phases per spread counted over the settled year on the phone;
- the parts' day buffer is within §13.1's 600 MB (256 bytes per part plus its rows);
- counters: `phx_pop.parts`, `phx_pop.new_cells`, `phx_pop.landings`, `phx_pop.rekeys`, ratcheted per day by cause.

**Guards**: PC-32: no landing outside 10b; no split outside an apply sub-step; the `Landing` instruction is the only
way a cell's totals change at 10b.

**Not allowed**:
- probing neighbouring steps;
- joining across a kink;
- a part that becomes a cell without trying the index;
- a pairing recorded;
- a seller member selling units it did not hold;
- a landing order that depends on the thread count.

**Done when**
- [ ] Splits, pooled flows, landing, clusters, re-keying, pairings, the seller spread and tiles work, with the tests
  passing.
- [ ] LC-0-43 to LC-0-46 are registered.
- [ ] PC-32 is registered.
- [ ] Two reviews are done.

---

### S0.24 — `phx-pop` IV: tolerance control, promotion, renumbering, the landing index and the reference mode

**Status**: planned

**Clauses**:
- PROCESS: REP.10, REP.28; REP.29 *(part: promotion and demotion by rank; promotion on a sale decision is S4.06)*.
- STATE: REP.2 *(part: promotion by rank)*.
- INVARIANT: REP.13, REP.31 *(the family, complete)*.
- MEASURE: REP.15.
- PRIMITIVE: REP.18 *(part: the RESOLUTION settings; taste distributions and review costs come with the choosing
  systems of Stage 1)*.
- PTY.12 *(part: the reference-run mode)*.

**Architecture**: §7.2, §7.6, §7.11, §14.4.

**Depends on**: S0.23.

**Goal**: the representation keeps itself within budget and honest:
- the landing index;
- tolerance control that merges steps on the day the budget is exceeded, and narrows on light days, stopping at a
  declared share;
- promotion and demotion by rank;
- incremental renumbering;
- REP.15's measures;
- the reference mode, in which nothing lands.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-pop/src/index.rs` | the landing index: sharded `KernelMap<u64, CandidateList>` with small inline lists; updated by keyed reduction at 10b |
| `src/tolerance.rs` | REP.28: estimating the decision gap per position from the pure evaluation forms over landings sampled from the representation's world stream; merging or dividing steps; re-keying; the joins that follow |
| `src/promote.rs` | REP.29: monthly ranks per kind, promotion to individual, demotion below the lower rank, ties by lot |
| `src/renumber.rs` | one chunk range per declared light day, reordered by (kind, country, region, zone, key); remapping holder lists, the directory and the index for the rows it moves |
| `src/measure.rs` | REP.15: dispersion erased, decision gaps, counts per day, the share of each population at weight one |
| `src/reference.rs` | the reference mode: landing disabled; every household and small firm a row of weight one (PTY.12) |

**Design**

- **The landing index**:
  - landing key → up to 4 candidates inline, each its `PartyId` (8 bytes) and slot (4 bytes), with an overflow list,
    per shard, kept in `PartyId` order so a lookup's order never depends on storage (§2.17);
  - updated only at 10b by keyed reduction of (key, insert/remove);
  - rebuilt on load (S0.20) and verified against the world hash.
- **Tolerance control** (REP.28):
  - **Trigger**: at 10b, when the cells carried exceed the cell budget.
  - **Gap estimate**: for each position, the gap that widening it one level would cause. Landings already made cannot
    show it: they are joins inside the current steps. So the estimate samples cells from the representation's own
    stream (`REP.tolerance`, a declared sample per kind) and finds each one's partner by one index lookup: its landing
    key with that position's step replaced by its sibling in the merge pair. For each pair found it evaluates each
    affected decision point's pure form (REP.15) on the two apart and joined. The same samples give, per position,
    the share of cells that would find a partner, which projects how far each widening brings the count down.
  - **Normalisation**: each decision's gap is in its own unit (REP.15). It is divided by the decision's declared scale
    per member (the same `ScaleRef` the positions use, S0.21), so gaps compare across decisions as shares of the
    member's own scale. The normalisation is declared with each decision point.
  - **Ties**: equal gaps — at Stage 0 every gap is zero, since every decision is a placeholder — go by the kind's
    declared position order for widening (a RESOLUTION primitive), then by lot from `REP.tolerance`.
  - **Widen**: choose, from the projection, the widenings of smallest gap whose projected joins bring the count within
    budget, then apply them in **one** full sweep (architecture §6.3): every cell's key is recomputed by a shift and
    the cells now sharing a key land. If the count is still over after it, a second sweep runs that day with the
    projection redone; sweeps per trigger are counted (`phx_pop.tolerance_sweeps`), and the heavy-day budget holds
    two.
  - **Narrow**: on declared light days, divide steps where the gap is largest, and stop when the cells carried reach
    the declared share of the budget. Every member of a cell holds the same per-member value, so no cell straddles the
    new boundary: narrowing is a re-key only, for landings from then on (REP.28). It is a declared full sweep, one
    position per light day.
  - Kinks are never crossed at any level, since the signature is part of the key.
- **Promotion** (REP.29):
  - Ranks per kind are read on a declared day of each month and at every issuance of a public instrument. The rank
    measure is declared per kind (employees for firms, net worth for households; RESOLUTION). The rank's edge is
    found by one pass building a histogram of the measure over the kind's cells and individuals, never a sort: about
    1 M rows at 2 ns, 2 ms a month.
  - A member rising into the rank splits out and becomes an individual: its key and positions come from the cell, and
    its profile values are drawn.
  - An individual rejoins the cells below the demotion rank, if it has no public instrument.
  - Ties at the edge go by lot from `REP.promotion`, with the kind as subject and the tied cells in `PartyId` order.
- **Renumbering** (architecture §7.2): per light day, one declared range of chunks is re-sorted by (kind, country,
  region, zone, key). Rows and their arena lists move with `move_list` (S0.06), and every slot reference — holder
  lists, the directory, the index, the agenda — is remapped for the moved rows only.
- **The reference mode**: `phx reference` runs the same world with every household and small firm at weight one,
  drawn by GEN's canonical two passes (S0.25) so it is the same world (PTY.12), on the owner's large machine. What
  differs from play is declared in one place, the mode's settings:
  - `VA_BUDGET` 512 GiB and table reservations sized for about 137 M rows;
  - the landing index, clusters and tolerance control off: every split is of a whole weight of one, which re-keys in
    place (S0.23);
  - promotion and renumbering on, as in play, so the individuals and the storage order follow the same rules.

  Its size: about 120 M household rows at about 1 KB each (the 464-byte record and about 25 rows of 24 bytes) and 17 M
  small firms at about 1.3 KB, plus the play world's other stores: about 145 GB, within the owner's 256 GB machine
  (architecture §7.11).

**Unit tests**
- `index_insert_remove_lookup`.
- `widen_merges_pairs_and_shifts_keys`.
- `gap_estimate_on_linear_rule_is_zero`: a decision that is straight between kinks has a zero gap (REP.36).
- `gap_estimate_samples_merge_pairs`: the sampled pairs differ only in the merged position's step.
- `widen_ties_by_declared_order`.
- `promotion_ties_by_lot`.
- `demotion_hysteresis`.
- `renumber_remaps_every_reference`: over a small hand-built set of columns and lists.

**Live checks** (applicable from S0.25)
- `LC-0-47`: the cells carried never exceed the cell budget at any close (REP.28).
- `LC-0-48`: every party within the promotion rank is an individual at each monthly read; no individual with a public
  instrument is in a cell (REP.2, REP.29).
- `LC-0-49`: REP.15's measures are reported every day, with the share of each population at weight one.
- `LC-0-50`: the identity hash (S0.06) taken immediately before and after 10c's renumbering slice is equal; and the
  `renumber` guard runs a year with renumbering on and off from one seed and compares identity hashes every day, so a
  slot order leaking into later state is caught.

**Budget**:
- Tolerance control ≤ 170 ms on the day it runs (architecture §13.2).
- Narrowing one position ≤ 170 ms, on a light day.
- The monthly rank pass ≤ 5 ms.
- A renumbering slice ≤ 60 ms wall on a light day.
- The index at about 50 bytes per cell (12 per inline candidate), 12 MB more than architecture §13.1's 38; the
  memory table is updated with it.
- Gap estimation: a declared sample of about 10⁴ cells per kind, one lookup per (sample, position), ≤ 20 ms.
- Counters: `phx_pop.tolerance_runs`, `phx_pop.cells`, `phx_pop.individuals_per_kind`.

**Guards**: none new.

**Not allowed**:
- widening that crosses a kink;
- a gap estimated from the observer's stream (REP.16);
- promotion by a money threshold;
- renumbering that changes an identity;
- a reference run with a different opening.

**Done when**
- [ ] The index, tolerance control, promotion, renumbering, measures and the reference mode exist, with the tests
  passing.
- [ ] LC-0-47 to LC-0-50 are registered.
- [ ] Two reviews are done.

---

### S0.25 — GEN II and the population: households and small firms, the opening lines paying, `sys-dem` and `sys-est`

**Status**: planned

**Clauses**:
- STATE: PTY.2, PTY.3, PTY.5, REP.25, REP.26; POP.1 *(part: the roles of Stage 0)*, POP.2 *(part)*; FRM.23 *(part:
  small firms as cells, without behaviour)*; GEO.5 *(land held)*.
- PROCESS: POP.3, POP.4; GEN.6; PTY.9 *(endings with estates)*; GEO.8 *(losses at owners)*; L3 *(part: household
  estates)*; POP.9 *(part: heirs from kinship lines, distribution in kind)*; GEN.4 *(part: the population's
  balancing)*.
- INVARIANT: PTY.11; GEN.7 *(part: the population's world passes every family on day one)*.
- MEASURE: CHN.7 *(the realised rates, live)*.
- FORBID: POP.15 *(part)*.
- PRIMITIVE: PTY.15; POP.16 *(part: life tables and health hazards)*; GEN.12 *(part)*.
- The opening world's employment, tenancy, deposit and loan lines paying as their terms say (spec Part O Stage 0):
  LAB.1 *(part)*, HSG.2 *(part)*, BNK.1 *(part)* and BNK.19 *(part)*, with placeholders naming LAB, HSG and BNK for every decision they
  lack.

**Architecture**: §7.1, §7.4, §9.1, §10, §13, §14.6.

**Depends on**: S0.24.

**Goal**: the full opening population, carried in cells, living a simulated year:
- about 120 million households and 17 million small firms and household businesses, drawn complete in two canonical
  passes and landed at the play resolution;
- their holdings, profiles and lines — employment, tenancy, deposits, loans — balanced against the institutions of
  S0.16;
- deaths, illness, ageing and catastrophes acting on members;
- paydays and dues paying by the lines' terms;
- household estates opening and distributing;
- the world settling for its declared length.

This is the world the Stage 0 gate measures.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-pop/src/*.rs` | household and person roles, key and position attributes of Stage 0 (region, composition, preference type, tenure, age class, credit-record stage, banking arrangement, clocks), profile attributes (birth year, occupation family, skill, health, zone, dwelling class), facts and decision points declared (with placeholders) |
| `crates/interfaces/if-labour/src/terms.rs` | the employment line's terms (LAB.1, amended); the wage point table |
| `crates/interfaces/if-property/src/terms.rs` | the tenancy and mortgage terms with zone and class |
| `crates/interfaces/if-banking/src/terms.rs` | deposit kinds, the banking arrangement, payment order, coverage order |
| `crates/systems/sys-dem/` | POP's mortality, illness and ageing as hazards and processes; the population's opening contribution |
| `crates/systems/sys-est/` | household estates: opening, the waterfall through the ledger (S0.17), distribution in kind to heirs (POP.9, part) |
| `crates/systems/sys-lab/src/gen.rs`, `sys-hsg/src/gen.rs`, `sys-bnk/src/gen.rs`, `sys-frm/src/gen.rs` | their lines' opening contributions and their placeholders |
| `crates/assembly/phx-world/src/gen/population.rs` | the two canonical passes (architecture §10.3) |
| `data/<country>/gen/{DEM,FRM,LAB,HSG,BNK}.toml` | the opening distributions with sources: life tables, censuses, household composition, income and wealth, kin, firm sizes, tenure, mortgages, deposits. Household composition, income and wealth are declared by `sys-dem` until `sys-hh` exists (S1.12), when the register records the change of declarer |
| `data/<country>/DEM.toml` | life tables and health hazards by age (TECHNOLOGY) |
| `data/world.toml` | `settling_years = 1` (the owner's default, adjustable) |

**Design**

- **Pass A**: every household is drawn **complete** from counter keys — its members as roles, their birth years,
  occupation families, skills and health; its region and zone; its tenure, dwelling class, deposits, loans and
  holdings; its kin (below) — in parallel.
  - **The keys**: each attribute's draws come from the stream `<SYS>.opening_<attribute>` of the system that declares
    it, with the subject (country, region, household ordinal) and a fixed **counter block** per (member, attribute).
    A draw that needs a variable number of variates takes them inside its own block, so it never shifts the draws
    after it.
  - **Household ordinals are laid out by region** (region being in the key), so a region's households are one range.
    Pass A and pass B run region by region.
  - **The household draws its lines' terms**: the wage point, hours and start band of each employment, the rent
    and term of a tenancy, the rate and remaining term of a loan, the kind of each deposit. Each is drawn from its
    sourced distribution directly over the trade's price points (REP.34), so no term is set by balancing (GEN.11).
  - **Kin** (POP.1): each household draws the count of its adults' children living in other households, by their
    age classes and regions, from the census sources. The children's side is drawn and the parents' side derived, so
    no relation is counted twice. These are kin lines between households (REP.3); heirs are drawn from them (POP.9).
  - Pass A keeps **stratum counts per GEN chunk**, sparse, not only totals, so pass B can rank each household within
    its stratum by a prefix sum over chunks in canonical order. A GEN chunk is about 1 M households (an engineering
    constant, apart from the tables' chunks), so the counts are about 120 chunks × the strata present in each.
- **Allocation** (architecture §10.2): counterparty sides are **derived** by largest-remainder apportionment over the
  drawn sizes of the counterparties eligible in each stratum:
  - employers for employment lines, by occupation family, skill and region;
  - landlords for tenancies;
  - banks for deposits and mortgages;
  - the dwelling stock per (zone, class).

  The derived side only apportions counts: it receives rows on terms the households drew. Unmatched strata take the
  declared substitute or stay unmatched (unemployed, recorded homeless), each reported. Balancing through the ledger
  closes every book (GEN.4).
  - The banking arrangement, being a key attribute, is assigned per household: within each stratum, the household's
    canonical rank (its chunk's prefix plus its place in the chunk) falls in one bank's apportioned range. The same
    rule assigns the lenders of loans, which name one counterparty. Employment and tenancy lines record no pairing
    (REP.23): a household joins the line of the terms it drew, and only the line's counterparty side is apportioned in
    counts, so no employer or landlord is stored per household.
- **Pass B** is a replay context of pass A (S0.11): it redraws every household identically from the same keys and
  applies the allocations, region by region. Each region's parts are sorted by landing key and landed in bulk at the
  run's resolution before the next region is drawn, so the sort scratch is one region's (at most about 15 M parts,
  about 250 MB of keys), within the day buffers of architecture §13.1. The reference mode lands none.
- **Small firms and household businesses** (FRM.23) are drawn the same way: incorporated firms below the promotion
  rank as small-firm cells; unincorporated businesses as households of the business-running kind.
- **Opening lines paying**:
  - Employment lines pay wages on each employer's pay dates; tenancies pay rent; loans pay instalments; deposits
    accrue interest by their terms.
  - These are settled through S0.17's batches with pooled flows, with levies absent: taxes arrive with TAX at S1.11,
    and their absence is a placeholder naming TAX.
  - Nobody decides anything yet. A payer that cannot pay fails, the fail waits for the placeholder decision of its
    line's system, and it is counted.
- **`sys-dem`**:
  - Mortality (POP.3) and illness and disability (POP.4) are hazards by birth year and health, scheduled at an
    envelope (S0.22), acting on counts of roles (REP.26).
  - Ageing: members crossing an age class on their birthdays are parts, drawn across the year (REP.25). For each
    (cell, birth year) whose members cross a class boundary this year, the count crossing on a day is binomial over
    those still to cross with probability one over the days left in the year, scheduled at an envelope like a
    hazard (S0.22) from the stream `DEM.birthday`.
  - A death is an event. The person's share of the household's claims passes by the household's rules, and a
    household with no surviving member becomes an estate (PTY.11, POP.15).
- **`sys-est`** (L3, part): an estate row per ended household in `phx-core`'s estate kind table. The estate sells what
  its debts need — here only through the market forms of S0.18 that exist, and otherwise waits, counted, on a
  placeholder naming its buyers' systems. It pays through the waterfall, and distributes the rest in kind to heirs
  drawn from its kinship lines' counts (REP.23), by the declared inheritance law (POLICY). With no heir, the law's
  declared destination takes the rest (the treasury, in every opening country), so nothing is ownerless (POP.15, Law
  13). An estate ends when its distribution settles.
- **Catastrophes** (GEO.8): S0.13's events reach owners through the two-level draw of architecture §7.10:
  - units lost across holdings;
  - then each hit holder's dwelling-role attachments;
  - tenants reached through the landlord's tenancies;
  - claims to insurers deferred to INS (Stage 4, a placeholder).
- **Settling** (GEN.6): the world runs by its own mechanisms for `settling_years` before day one of play; its history
  is kept.

**Unit tests**
- `canonical_passes_identical`: pass B redraws what pass A drew for the same keys (the per-member draw function over
  keys is pure).
- `counter_blocks_isolate_variable_draws`: a draw taking more variates leaves every later attribute's draws unchanged.
- `kin_counted_once`.
- `apportionment_by_stratum_exact`.
- `rank_assignment_independent_of_chunking`: the same households get the same banks for any chunk size.
- `no_heir_goes_to_declared_destination`.
- `death_moves_claims_by_rule`.

**Live checks**
- `LC-0-51`: day one passes every audit family with the full population (GEN.7).
- `LC-0-52`: the populations reconcile — births (none yet), deaths and entries; every person is in exactly one
  household; every household has a member or is an estate (PTY.11, REP.13).
- `LC-0-53`: every death has a cause and a destination for everything held and owed (POP.15); every estate
  distributes and ends, or waits on a named placeholder, with the waiting estates counted by placeholder.
- `LC-0-54`: realised mortality and illness per age class match their declared tables within sampling error over the
  year (CHN.7).
- `LC-0-55`: paydays and dues settle through pooled flows, and every fail has a cause and a waiting owner; the count
  of fails by line kind is published (liveness, N2).
- `LC-0-56`: the GEN report lists every apportionment difference and every unmatched stratum (GEN.4).
- `LC-0-37` to `LC-0-50` become applicable and pass.

**Budget**: the whole of architecture §13 as it applies to Stage 0's world — memory at the worst day's peak,
the Stage 0 lines of §13.2, the reference run's size — measured at S0.26.

**Guards**: every placeholder names its retiring system, and the placeholder count is ratcheted (NUM.7).

**Not allowed**:
- a household drawn twice differently;
- counterparty sizes overwritten outside the apportionment;
- behaviour in a placeholder;
- a mortality rate that is not the table's;
- an estate that values rather than sells, or that distributes before its debts are paid.

**Done when**
- [ ] The full population opens, balances, settles for a year and lives a simulated year with deaths, illness, ageing,
  catastrophes, paydays and dues.
- [ ] LC-0-37 to LC-0-56 pass.
- [ ] Two reviews are done.

---

### S0.26 — `phx-obs`, `phx-ffi`, the Android bench, the measurement programme and the Stage 0 gate

**Status**: planned

**Clauses**:
- PROCESS: OBS.4 *(part: the player as an individual, its queue as wakes)*; REP.30 *(tracers)*.
- STATE: REP.2 *(the player an individual from the start; with S0.24's promotion)*; OBS.3 *(part: the public-event
  rule, a standing SHAPE)*.
- FORBID: OBS.5 *(part)*.
- PRIMITIVE: OBS.9.
- MEASURE: GEN.8; N2; N6 *(experiments on copies)*; N8.1–N8.10 *(the budget measured on the device)*; PTY.12 *(part:
  `phx reference`, `phx compare`, `phx ladder`)*.

**Architecture**: §11, §12, §13, §14.4–§14.7.

**Depends on**: S0.25.

**Goal**: the world measured where it matters:
- the phone runs it through `phx-ffi` in the bench flavour;
- the measurement programme reports the representation's numbers and unit costs;
- the reference run and the comparison exist;
- the phone's fundamentals are measured;
- the Stage 0 gate is judged, and architecture §13 is rewritten with measured numbers.

**Files**

| File | Purpose |
| --- | --- |
| `crates/assembly/phx-obs/src/*.rs` | minimal views, fixed-bin histograms and tracers (REP.30); read-only; swapped behind an `Arc` at 10e |
| `crates/kernel/phx-core/src/events_rule.rs` | the OBS.3 rule at 10a: which recorded events become public, declared as a standing SHAPE. It reads only the day's events (which every system records at the sub-step that caused them, CHN.4) and public records, so it needs no state `phx-core` cannot see |
| `crates/apps/phx-ffi/src/lib.rs` | UniFFI surface: create, load, step a turn, read a view page, submit an action (the player's queue), save; the `PerfHint` implementation over Android's performance-hint API; worker thread ids passed to it; the `Clock` implementation over the monotonic clock |
| `crates/apps/phx-ffi/src/bench.rs` | the bench flavour's engine side: the phone probe, the kernel micro-benchmarks of S0.04, S0.07 and S0.17, and the report writer, all inside the app's process |
| `perf/schema/device-report.json` | the report's JSON schema; `phx-check` refuses a committed report that does not validate |
| `android/` | the Compose app shell and its `bench` flavour: runs N turns headless and writes a JSON report |
| `crates/apps/phx-cli/src/measure/*.rs` | `phx measure`: the counts and unit costs of architecture §14.4 and §14.6 |
| `crates/apps/phx-cli/src/reference.rs`, `compare.rs`, `ladder.rs`, `experiment.rs`, `seeds.rs` | the measurement commands |
| `crates/kernel/phx-exec/src/probe.rs` | the phone fundamentals, called by `phx-ffi`'s bench: sustained core-seconds per second by core class after a 30-minute soak; random gathers over 1–3 GB with all cores, 16 KiB pages and prefetch; sweep bandwidth; barrier cost |
| `perf/device/S0.26-*.json`, `perf/measure/S0.26-*.json` | the reports, committed by the owner |

**Design**

- **The player** (OBS.4, REP.2): an **individual** from the start, flagged so it is never landed and never demoted
  (REP.29), whose decider names the player. A queued intent is a wake: at 1c of the first day its decision point
  runs, it gives the player an occasion for that decision. On days with nothing queued, the rule decides only if the
  player's settings delegate (S0.10); otherwise the decision is not taken. The player appears in every audit family.
- **Tracers** (REP.30): a declared number drawn at the opening from the observer's stream; they follow parts by
  profile share and change nothing (Law 17). They are in the `looking` guard (S0.11).
  - Parts are day-local and gone by 10e, so `phx-pop` keeps a read-only **split log** for the day, written only for
    splits of cells that hold a tracer (a flag the observer sets): the origin cell, the part's `seq`, its counts per
    profile value and the cell it landed in or started. `phx-obs` reads it at 10e to move each tracer, and it is
    dropped at the next 1a. It is not world state, is not hashed, and its bytes are counted in the views line of
    architecture §13.1.
- **`phx measure`** reports architecture §14.6's six items:
  - the rows-per-cell curve over three or four cell budgets, at the opening and after a year; profile entries per
    role; distinct keys and banking arrangements per region;
  - the phone's fundamentals from the probe, with 16 KiB pages and prefetch;
  - parts and new cells per day by cause, and a part's unit cost per component (S0.23's split);
  - on the heaviest payday: legs and rows, nanoseconds per row and per leg with levies, the fixed point's iterations,
    and the day buffers' peak;
  - candidates, redraws and agenda rows per day, with their unit costs;
  - the longest holiday block times the measured non-business day;

  and bytes per store with peak resident memory, read as `VmHWM` and PSS.
- **The device report** (`perf/device/`, validated against its schema): per turn and per sub-step, wall times with
  the day's type; the day's counters; thermal status and headroom (`AThermal`); `VmHWM` and PSS; core frequencies;
  the performance-hint session's status; the build and the world hash. The measured year starts after a 30-minute
  soak of the same world, so it is sustained in N8.3's sense.
- **Unit costs on the phone** come from the kernel micro-benchmarks in the bench (S0.04, S0.07, S0.17) and from
  per-sub-step timers divided by the day's counters. A part, which spans 10b's sub-steps, is timed per component by
  counters on its stages.
- **Deferred targets**: every phone target of the earlier steps is judged here, each passing when its median over the
  settled year (or over its micro-benchmark) is within it:

  | Step | Targets |
  | --- | --- |
  | S0.04 | Philox, `open_unit`, binomials, alias draw and picks, as its table |
  | S0.07 | barrier ≤ 60 µs; gather ≥ 4 GB/s; radix sort and keyed reduce of 10⁷ |
  | S0.11 | the empty world ≤ 50 MB |
  | S0.13 | map generation ≤ 10 s |
  | S0.16, S0.25 | GEN I and II and settling: measured and reported (no budget set; beyond ten minutes, raised with the owner) |
  | S0.17 | 7a ≤ 10 ns per row read; 7c ≤ 30 ns per payment; a due ≤ 50 ns per leg |
  | S0.20 | a full save ≤ 5 s and ≤ 1.5 GB; an increment ≤ 1 s; load rebuild ≤ 3 s |
  | S0.21 | `step_of` ≤ 10 ns; a landing key ≤ 100 ns |
  | S0.22 | a candidate ≤ 180 ns; a redraw ≤ 150 ns; a dense evaluation ≤ 5 ns |
  | S0.23 | a part ≤ 2.5 µs by component; a seller spread ≤ 3 µs |
  | S0.24 | tolerance control ≤ 170 ms; the rank pass ≤ 5 ms; a renumbering slice ≤ 60 ms; gap estimation ≤ 20 ms |
- **`phx reference` and `phx compare`** run the weight-one world of the same seed on the large machine and compare
  the play resolution on the declared reads (N8.5, Appendix E 30). Stage 0's reads are demographic and monetary;
  Stage 1's gate adds the circular flow.
- **`phx experiment`** runs declared interventions on copies (N6). At Stage 0 its interventions are N6's kinds only (a
  changed primitive or endowment at a date, or a knock-out); none places an order for a party.
- **The Stage 0 gate**:
  - CI green;
  - the device report of a settled simulated year on the phone, committed by the owner;
  - memory within budget at the measured design point, with the 10% headroom of architecture §13;
  - time within budget for Stage 0's world, and, since Stage 0 has little behaviour, architecture §13.2 **projected**
    from the measured unit costs to Stage 1's counts: a projected miss is recorded as a finding (§11) before Stage 1
    starts, since the spec's Stage 0 exit bounds memory, not the daily flows;
  - saves within the owner's save budget (§12): a full save ≤ 5 s and an increment ≤ 1 s on the phone;
  - the measurements of architecture §14.6 recorded.

  Architecture §13 is rewritten with the measured numbers in this step's commit. If the budget is missed, the
  remedies of N8.7 apply in order, and the owner decides if none suffices.

**Unit tests**
- `event_rule_is_pure`.
- `tracer_follow_probability`.
- `ffi_types_roundtrip`.

**Live checks**
- `LC-0-57`: the player's queued intents are decided on the first day their decision point runs; with nothing queued,
  the rule decides; the player appears in every family.
- `LC-0-58`: every public event was produced by the rule from the state, with a date and subjects (OBS.3, OBS.5).
- `LC-0-59`: liveness (N2): money circulates, fails are counted by cause, no quantity grows without bound for an
  unnamed reason, no dead fixed point.
- `LC-0-60`: GEN.8 — each opening distribution's distance from the world's own is reported at the end of settling and
  of the year.

**Budget**: this step measures the budget. The gate requires the Stage 0 world within 4.5 GB peak and within the time
budget on the phone, sustained over a year.

**Guards**: `perf/device/` changes need the owner's review; the ratchets take their first measured values.

**Not allowed**:
- a device report not produced by the bench flavour on the phone;
- a budget judged on a desktop;
- a view that writes;
- a comparison with a reference run of a different opening.

**Done when**
- [ ] The phone runs a settled simulated year in the bench flavour, and the report is committed, with save and load
  times.
- [ ] `phx measure`, `reference`, `compare` and `experiment` produce their reports.
- [ ] Architecture §13 is rewritten with measured numbers.
- [ ] The Stage 0 exit of spec Part O holds.
- [ ] LC-0-57 to LC-0-60 pass.
- [ ] Two reviews are done.

---

## 4. Stage 1 — The circular flow

**Exit** (spec Part O):
- All three countries, each closed to the others, run the circular flow:
  - households earn wages and spend them at firms that pay wages;
  - firms are born and die;
  - banks lend and are repaid;
  - the treasury taxes and spends;
  - the world keeps doing so for decades without anything imposed.
- A simulated year of it, with the full population at the play resolution, meets the performance budget on the target
  device, with the resolution ladder within the declared accuracy (Appendix E 30). This is the **first go/no-go**.

**Decision rules** follow §2.21.

---

### S1.01 — `phx-val`: outlooks, heuristics, surprises and values

**Status**: planned

**Clauses**:
- STATE, DECISION, PROCESS, INVARIANT, MEASURE, FORBID, PRIMITIVE: VAL.1–VAL.23, all of them.
- PROCESS: REP.21 *(completes it: attention as a continuous decision, its review cost paid)*; REP.35 *(completes it:
  the surprise that raises attention and wakes)*; REP.22 *(part: tastes over heuristics)*.
- The extension points later steps use: the investor schedule (S3.03) and registered series, whose outlooks are
  computed only for the pairs registered (S3.03, S3.05) (architecture §8).

**Architecture**: §3.3 (`phx-val`), §7.3 (surprises wake), §8.

**Depends on**: S0.26.

**Goal**: every decision reads its own party's view:
- outlooks of public series, computed once per method per day;
- outlooks of a party's own variables, carried as its positions;
- heuristics chosen by their recent performance, with a cell's members split across stances;
- surprises and confidence as reads;
- values by the party's own simple models, never prices.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-val/src/outlook.rs` | `Outlook { var: VarId, unit, ccy: Missing<Ccy>, horizon: Period, day: Day, mean: Fixed<6>, width: Fixed<6> }` |
| `src/heuristics.rs` | the menu (VAL.6) as pure functions: `adaptive`, `trend`, `anchor`, `announcement` |
| `src/method.rs` | `Method { heuristic, memory: MemoryType, window: AgeWindow }`; public-series outlooks per (series, method) at 5a |
| `src/experience.rs` | experience weighting by age class (Malmendier and Nagel, 2016): lived-years weights `w(k) ∝ (L − k)^θ` over a member's lived years `L` |
| `src/switching.rs` | performance records and discrete-choice switching (Brock and Hommes, 1997) |
| `src/surprise.rs` | surprise = observed − expected; confidence width as an exponentially weighted mean of absolute surprises |
| `src/value.rs` | the value methods of VAL.8 as pure functions |
| `src/registered.rs` | registered series: the (method, series) pairs some party registers, kept by keyed reduction, and their outlooks at 5a |
| `src/schedule.rs` | the investor schedule, a pure function; empty until S3.03 fills it |
| `src/attention.rs` | attention from stakes and review cost (Reis, 2006): intensity and daily probability |
| `src/heuristic.rs` | `trait Heuristic`, sealed, so no crate but `phx-val` can implement one (PC-33 is a compile-level refusal) |
| `data/shared/VAL.toml` | memory and switching-intensity type sets (NUM.4); heuristic parameters (λ, γ, κ) per memory type; attention sensitivity per type; the menu listed in `SHAPES.toml` with its sources. Patience and risk aversion are HH's and management's primitives (one register entry each), passed to the value methods as arguments |

`phx-val` declares no positions and no decision points: it is pure functions and the public-series outlooks. The
positions that carry own outlooks, stance and attention, and the stance review decision point, are declared by the
systems whose parties hold them — `sys-hh` for households (S1.12) and `sys-frm` for firms (S1.03) — through
`d.pop_kind` and their interface crates.

**Design**

- **Public series** (VAL.23): for each published series (prices, rates, indices, statistics — the record kinds of
  S0.10 marked public) and each method in use, the outlook is computed once at 5a and read by every party using that
  method.
  - A method is (heuristic, memory type, age window).
  - Experience weighting: a member of age class `a` weights the observations of its lived years by `(L − k)^θ`, with
    θ a PREFERENCE of its memory type. The weights act on annual means of the series, so a method's long mean is at
    most one term per lived year, and it is recomputed when a year's mean closes, not daily.
  - The opening history (GEN.5) is part of the series.
- **Registered series** (VAL.23), for record kinds declared registered (instrument prices from S3.03): the pairs
  (method, series) are registered and released at applies by keyed reduction (architecture §4.8), as holders and
  candidate lists change, and the outlook of a pair is computed at 5a only on a day its series has a new print. An
  individual's own outlook of such a series is its method's outlook plus its own deviation; under `adaptive` the
  deviation decays by (1 − λ)^n over the n prints since it last read, and `trend` and `anchor` carry none beyond
  the anchor's own term, so catching up costs O(1) and no series' history is kept for it.
- **Heuristics** (VAL.6), each over a series with its publication lags:
  - `adaptive`: `E_t = E_{t−1} + λ·(x_t − E_{t−1})`, with λ the party's memory speed;
  - `trend`: `E_t = x_t + γ·(x_t − x_{t−1})`;
  - `anchor`: `E_t = x_t + κ·(A − x_t)`, with `A` the experience-weighted long mean or a published target;
  - `announcement`: a published, dated change enters the outlook from its effective day.

  λ, γ and κ are PREFERENCE parameters per type. The menu is a standing SHAPE.
- **Own variables** (VAL.23): an outlook of a party's own income, sales or job is its position (REP.20), updated at its
  visits from its own receipts by its current heuristic, and joined at landing only within its step.
- **Switching** (VAL.7):
  - Performance per (method, series) is the exponentially weighted squared error at the party's memory.
  - An individual weights heuristics by `exp(−β·perf_h) / Σ exp(−β·perf_j)`, with β its switching intensity.
  - A member of a cell holds one **stance**, a key attribute. On a stance review occasion (a decision kind with its
    own schedule, S0.22), the count choosing each heuristic is a multinomial with those probabilities and each
    member's taste draw (REP.22). Members who change stance split into parts.
- **Surprises and confidence** (VAL.4, VAL.9):
  - `surprise = observed − expected`, per party, variable and date. For public variables it is a public record per
    (method, series) at each publication. For a cell's own variables it is a position: the dated last surprise of each
    own variable, beside its outlook's mean and width (counted in S1.12's layout).
  - `width` is the exponentially weighted mean of |surprise|.
  - A surprise larger than a type's declared attention sensitivity times its width **wakes** the parties it bears on
    for the decisions that read the variable (REP.35): own variables at the visit that observed them; public series
    by architecture §7.3's wake pass over the keys of the stances and types it reaches.
- **Attention** (REP.21), a continuous decision of the cell, computed at its visits, per lumpy decision kind k:
  - the loss from an unreviewed decision grows as ½·ψ_k·σ²·τ² over τ days, where σ² is the variance per day of what
    the decision targets (read from the party's outlook widths, in the target's unit squared per day) and ψ_k is the
    loss's curvature at the party's position (money per unit squared, from the decision's pure evaluation form);
  - reviewing costs c_k in money: its hours (TECHNOLOGY) at the party's own wage or value of leisure;
  - minimising c_k ÷ τ + ¼·ψ_k·σ²·τ gives the review intensity λ_k = ½·sqrt(ψ_k·σ² ÷ c_k), per day;
  - σ² has the party's own part and its method's public part, so λ_k = g_k·(sqrt(σ²_own) + sqrt(σ²_pub,m)) with
    g_k = ½·sqrt(ψ_k ÷ c_k). The cell stores g_k·sqrt(σ²_own) and g_k, and its exposure over a span is that rate ×
    days plus g_k × the rise in its method's cumulative public series (architecture §7.3), exact without a visit;
  - the daily review probability is `a_k = −expm1(−λ_k)`, so −ln(1 − a_k) = λ_k feeds the review exposure (S0.22).

  The form is optimal inattention (Reis, 2006), listed in `SHAPES.toml`; its separate sources of variance are this
  plan's choice, so a public surprise raises attention without visiting any cell.
- **Values** (VAL.8, VAL.10): pure methods reading the party's own outlooks, patience and risk aversion:
  - `claim_value(cash_flows, required_return)`;
  - `firm_value(distributions or earnings, required_return)`, or `comparable(prints of similar things)`;
  - `project_value(expected output × expected price − running costs over life, cost of funds)`;
  - `dwelling_value(rent saved or earned, expected price, alternative)`;
  - `offer_value(wage, outside option, moving cost)`;
  - `platform_value(policy applied to own position and outlooks)`.

  The required return is patience plus risk aversion times the variance the party sees (its outlook widths). With no
  history, a method starts from the closest observed things (VAL.10). A value is a `Value` type that cannot become a
  `Print` (S0.18).

**Unit tests**
- `adaptive_converges_to_constant`.
- `trend_extrapolates`.
- `anchor_reverts`.
- `announcement_effective_day`.
- `experience_weights_by_age`: an older class weights earlier years more.
- `switching_shares_logit`.
- `surprise_width_ewma`.
- `attention_rises_with_stake_and_falls_with_cost`.
- `attention_units`: λ is per day for ψ in money per unit², σ² in unit² per day and c in money.
- `exposure_span_exact`: the stored rate and the method's cumulative series give the same exposure as summing λ daily.
- `values_are_not_prices` (compile-fail).
- `claim_value_discounting`.
- `registered_deviation_catch_up_exact`: decaying a deviation over n prints equals updating it print by print.

**Live checks**
- `LC-1-01`: VAL.12 — outlook dispersion across parties is reported per variable, is positive wherever parties
  differ in method or history, and widens in the weeks after a large surprise.
- `LC-1-02`: VAL.11 — no outlook was formed after the stage that uses it (the read-trace and outlook dates).
- `LC-1-03`: VAL.15 — heuristic shares per series are reported and move over the run (a constant share for a year is
  a finding); their lead over price swings is published.
- `LC-1-43`: VAL.13 — for each turning point of a published series, the lag of each method's outlook behind it is
  published, by memory type and heuristic mix.
- `LC-1-44`: VAL.14 — after each large surprise, the days until each stance's first changed decision are published,
  ranked by the size of its surprise.
- `LC-1-04`: VAL.16 and VAL.21 — no variable is read by every party as one expectation. There is more than one
  distinct value per valued thing wherever two parties with different histories value it.

**Budget**:
- Public-series outlooks: about 10³ series × about 10² methods ≤ 5 ms a day; experience-weighted long means ≤ 20 ms
  when a year's mean closes.
- Own outlooks and attention are updated at visits (inside the visit's unit cost; the review's prototype measured the
  visit's arithmetic, ten attention intensities and the spending rule, at 96 ns on one x86 core).
- Counters, ratcheted: `phx_val.methods_in_use`, `phx_val.surprise_wakes`, `phx_val.public_surprise_records`.

**Guards**: PC-33: `Heuristic` is sealed in `phx-val`, so no other crate implements one (compile-level); and no
function of `phx-val` takes the world or a table, so none can run it to forecast (a signature check).

**Not allowed**:
- a global expected inflation;
- a sentiment parameter;
- an outlook reading another party's private state;
- a value used as a price;
- a method computed per member when it is the same for every member using it.

**Done when**
- [ ] Outlooks, switching, surprises, attention and values exist, with the tests passing.
- [ ] LC-1-01 to LC-1-04, LC-1-43 and LC-1-44 pass on the Stage 1 world as it grows (they apply from S1.12).
- [ ] PC-33 is registered.
- [ ] Two reviews are done.

---

### S1.02 — `sys-tec`: products and the opening ways

**Status**: planned

**Clauses**:
- STATE: TEC.1, TEC.2, TEC.3, TEC.4.
- INVARIANT: TEC.9.
- FORBID: TEC.12.
- PRIMITIVE: TEC.13 *(part: the opening ways)*.
- Research, imitation, learning and obsolescence (TEC.5–TEC.8, TEC.10, TEC.11) are S6.01.

**Architecture**: §3.5, §4.1.

**Depends on**: S1.01.

**Goal**: what can be made, and how:
- products (goods and services) with their physical units and industries;
- ways with inputs, labour, capital, land or deposit, lead time, batch, yield and by-products, all in physical units;
- the ways each firm knows;
- TEC.9's family: every unit of output made by a known way from inputs actually consumed.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-base/src/products.rs` | `ProductDecl { id, unit, industry, storable: bool, spoil_rate: Missing<Rate>, delivered_at_once: bool }`: what differs between goods and services is declared data a mechanism reads (a service is not storable and is delivered as it is made), never a variant to branch on (Law 10) |
| `crates/interfaces/if-base/src/ways.rs` | `WayId`; `Way { product, inputs: [(ProductId, QtyRaw per unit)], labour: [(OccFamily, Skill, hours per unit)], capital: [(CapKind, service units per unit)], land_or_deposit, lead_time: Period, batch: QtyRaw, yield_ppm: u32, by_products }`; `WaySetId`, an interned set of known ways |
| `crates/interfaces/if-firm/src/known.rs` | the firm fact of its known ways, a `WaySetId` |
| `crates/systems/sys-tec/src/*` | declarations; the way register; the TEC.9 audit family; the opening ways contribution |
| `data/<country>/TEC.toml` | products and opening ways per country, from input–output and engineering data, with sources (TECHNOLOGY) |

**Design**

- **Products and ways are data** (Law 10). The spec's minimum set is enough for the circular flow and for the stylised
  facts later:
  - food, energy carriers (placeholders until ENE, S2.09), manufactured consumer goods, capital goods and
    construction;
  - services: retail distribution, personal services, health and education;
  - commodities extracted from deposits.

  Each product has the ways its sources describe, differing in input, labour and capital mix (TEC.3); the count per
  product is TECHNOLOGY from data, not a representation choice, and nothing caps it. How finely products are
  distinguished (the product space) is RESOLUTION and is tested on the ladder.
- **Knowing a way** (TEC.4): a firm's known ways are a `WaySetId`, an interned set of 4 bytes in the key for cells
  (REP.19) and a fact for individuals, so the key's width does not grow with the number of ways. The key record's
  width is compiled from the declared key attributes in whole 8-byte words (S0.21), so no attribute is squeezed to
  fit. Known ways are lost when a firm ends without a successor (S1.03's endings).
- **Output** (TEC.9): production is a physical standing flow realised lazily (architecture §7.4): each realisation
  writes one transformation record for its span, naming the way, the inputs consumed and the output finished after
  the yield. The family checks every unit of output against a known way and inputs consumed.

**Unit tests**
- `way_units_are_physical`: no value-share field exists (TEC.12, a compile-level refusal).
- `yield_applied_exactly`: 1 000 started at 97% yield gives 970, with the declared rounding of units.
- `product_kind_is_data`: no `match` on a product's kind exists (a compile-level check through PC-rule Law 10).

**Live checks**
- `LC-1-05`: TEC.9 — every production record in the run names a way its producer knew, with inputs consumed as
  recorded.

**Budget**: a way register of a few hundred ways; known-way sets interned (counter `phx_tec.way_sets`, ratcheted).

**Guards**: none new.

**Not allowed**:
- a recipe as a value share;
- a product-specific branch in any mechanism;
- a way used by a firm that does not know it.

**Done when**
- [ ] Products and opening ways are declared with sources.
- [ ] TEC.9 runs.
- [ ] Two reviews are done.

---

### S1.03 — `sys-frm`: firms decide, produce, price, pay, are born and end

**Status**: planned

**Clauses**:
- STATE: FRM.1, FRM.2, FRM.23.
- DECISION: FRM.4, FRM.5, FRM.6, FRM.11; FRM.7 *(part: buying inputs; employing is S1.08)*; FRM.8 *(part: the
  production side; investing is S1.04)*; REP.34 *(part: posted prices of goods and services; wage points are S1.08,
  lenders' rate points S1.09)*.
- PROCESS: FRM.13, FRM.14; FRM.16 *(part: households found firms; firms and funds found them from S3.07)*; FRM.15
  *(part: default of payment and liquidation; the balance-sheet test and restructuring are S2.03)*; L3 *(part: firm
  estates liquidate through the waterfall)*.
- INVARIANT: FRM.17, FRM.18.
- FORBID: FRM.20, FRM.21.
- PRIMITIVE: FRM.22.
- Financing, payouts, groups, distress and the rest of the lifecycle (FRM.3, FRM.9, FRM.10, FRM.12, FRM.19) are S2.03
  and S3.05.

**Architecture**: §3.1 (interface order), §4.7, §7.4 (physical standing flows), §7.9 (posted prices, pressure), §9.1
(estates).

**Depends on**: S1.02.

**Goal**: firms — individuals and small-firm cells — that:
- decide what to produce, which way to run, what to charge at price points, and what inputs to buy, each from their
  own state and outlooks;
- produce by their ways, lazily realised;
- recognise revenue and cost with named counterparties;
- are founded by named founders with named money;
- end, by their owners' choice to close or by failing to pay, into estates that sell what they hold and pay their
  creditors in the law's order.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-firm/src/{facts,decisions,views}.rs` | firm facts (stock, capacity, known ways, output and input rates, unit cost, markup, pressure, sales outlook); decision points `produce`, `price`, `choose_way`, `buy_inputs`, `enter_exit`, `close`, `stance` |
| `crates/interfaces/if-pop/src/found.rs` | the household's decision point `found` (the decider's crate, architecture §3.1) |
| `crates/systems/sys-frm/src/rules/produce.rs` | FRM.4: target output |
| `src/rules/price.rs` | FRM.5 and REP.34: the price review, over the seller kind's declared pressure |
| `src/rules/markup.rs` | the markup's update on each review |
| `src/rules/way.rs` | FRM.6: the cheapest way at the prices faced |
| `src/rules/inputs.rs` | GDS.5 for firms: input orders for planned production |
| `src/rules/found.rs` | FRM.16: whether a household founds |
| `src/rules/close.rs` | whether an owner closes a solvent firm |
| `src/handlers/*.rs` | 5b production and input rates; 5c price, way, entry-and-exit and closure reviews on review and wake days, and founding decisions; 3c foundings executed; 2e failures after grace; the realisation of physical flows at visits and kinks |
| `crates/systems/sys-est/src/firm.rs` | firm estates: opening, selling stock through its market and plant bilaterally, the waterfall (S0.17), releasing staff |
| `src/gen.rs` | small firms and household businesses' key attributes and positions beyond S0.25's; the opening markups |
| `data/<country>/FRM.toml` | management type sets (target stock cover, adjustment times, markup adjustment speeds, pricing curvature, horizons: PREFERENCE); review schedules; price points per trade (POLICY of each trade); founding costs; the grace before a default of payment and the liquidation horizon (insolvency law, POLICY); review and menu costs in hours (TECHNOLOGY) |
| `data/<country>/gen/FRM.toml` | the opening markups' distribution by industry, from margins data (ENDOWMENT, GEN) |

**Design**

- **Production** (FRM.4), a continuous decision on the firm's schedule (weekly by default):
  - target output `y* = E[demand over the lead time] + (s* − s) ÷ τ`, where `s` is the stock, `s*` the target stock
    (weeks of expected sales, a PREFERENCE of its management type) and `τ` its adjustment time;
  - `y*` is limited by capacity (CAP.9: a `DeclaredLimit` from a physical token over its plant, S0.09) and by the
    inputs and labour it has;
  - the margin at expected prices must be positive, counting the financing cost of the work in progress at its
    marginal rate: until S1.09 the rate of its opening loan terms, a placeholder naming BNK (S1.09);
  - a `y*` at or below zero means the firm produces nothing this period: the rule's own kink (a negative output is
    not a state), recorded, and it wakes the entry-and-exit review.

  The form is the production-smoothing and inventory model of Holt, Modigliani, Muth and Simon (1960), listed in
  `SHAPES.toml`. The result is a physical standing flow (architecture §7.4): output and input use rates, realised
  lazily as rate × days at the firm's visits and at kinks (a stock reaching zero, a lead time ending), each
  realisation one transformation record for its span.
- **Way** (FRM.6): among the ways its plant supports, the one with the lowest unit cost at the prices faced (FRM.14),
  reviewed on the production schedule.
- **Pricing** (FRM.5, REP.34), a lumpy decision on the firm's review days and on days a surprise in its sales wakes it
  (REP.21, REP.35). `sys-frm` is the one writer of every firm's posted point (architecture §7.9):
  - the desired price is `p* = (1 + μ)·E[unit cost]·π^η`, where π is the seller kind's **pressure**, a fact the kind's
    system supplies as declared data: for a stocked good `π = (D ÷ E[D])·((s* + E[D]) ÷ (s + E[D]))` over the review
    interval, with D including demand turned away; for a service or carriage `π = (D ÷ E[D])·(f* ÷ f)`, f its fill
    and f* its target fill. π is positive whenever E[D] is; with E[D] missing the review is skipped and recorded;
  - **the markup μ is a position of the firm**, never a primitive: drawn at the opening from the opening margins by
    industry (GEN), and on each review moved by `Δμ = α_s·(sales ÷ E[sales] − 1) + α_c·(p̄_seen ÷ p − 1)`, where
    p̄_seen is the mean of competitors' posted points it can see (public prints) and α_s, α_c are its management's
    adjustment speeds (PREFERENCE);
  - the posted price is the price point nearest `p*` in the trade's point table, moving only if the gain in expected
    profit over the review interval exceeds the menu cost, paid in its staff's hours (TECHNOLOGY).

  The form is state-dependent pricing with a menu cost (Golosov and Lucas, 2007; Alvarez, Guiso and Lippi, 2012) over
  price points (Levy et al., 2011), listed in `SHAPES.toml`. Firms in one cell that decide differently split (REP.5).
- **Input buying** (GDS.5): orders for the inputs of planned production plus target input stock, up to the input's
  value in use (VAL.8), counting the financing cost. Orders go to the posted, call or bilateral markets of the inputs
  (S1.05).
- **Revenue and cost** (FRM.13, FRM.14):
  - revenue is recognised on delivery from the meeting's match set, which names each buyer group (architecture §7.9);
    for cells, the group's sales are spread over members by S0.23;
  - costs are named lines with named payees;
  - unit cost follows FRM.14, including the capital charge of an idle line;
  - FRM.17 and FRM.18 are families.
- **Founding** (FRM.16): a household of the founding kind, on its founding occasion (a lumpy decision at 5c), founds
  when its value of the venture exceeds its alternative. The value reads only what it can know (Law 12, VAL.10):
  posted prices of the product (public), the founder's own known ways at the input prices it faces, and published
  STA series of the industry's output and prices; never other firms' margins. The firm is created at the next day's
  3c: the founder pays in named money, buys its plant from producers (S1.04) and starts small. A birth with a named
  founder, never a birth rate.
- **Enter or exit a line** (FRM.11): enter by investing in plant and knowing a way (CAP); exit a product whose
  expected margin stays negative over the management's horizon.
- **Endings** (FRM.15, FRM.21, L3):
  - **Closure**: on its closure review (a lumpy decision, REP.5), an owner closes a solvent firm when the value of
    continuing (VAL.8 over its expected margins and horizon) is below what winding down would return: its stock and
    plant at the prices it expects to fetch, less what it owes.
  - **Failure**: a payment the firm cannot make fails at settlement (S0.17). A fail still unpaid after the
    insolvency law's grace (POLICY) is a **default of payment**, and at 2e the firm ends into liquidation. Every
    insolvency liquidates until S2.03 builds restructuring: a placeholder naming FRM (S2.03).
  - **The estate** (`sys-est`): an estate row (an individual of the estate kind) succeeds the firm; its lines and
    holdings pass to it by line transfer. It sells stock through the product's market and plant bilaterally, asking
    by the law's liquidation horizon; what it fetches is an outcome. It pays through S0.17's waterfall, releases its
    staff on their contracts' notice and severance (a placeholder naming LAB until S1.08), and ends when its
    distribution settles. A solvent closure's residual goes to the owners.
  - Members of a firm cell fail by their own payments: those whose payment fails split out (S0.17's prefix failure),
    and the members that end on one occasion end into **one** estate holding their count, since their lines and
    holdings are identical (one estate per (part, occasion)).
- **Streams**: `FRM.found_taste` (the founder's taste over ventures), `FRM.close_lot` (ties in which members close).

**The firm cell's record** (architecture §13.1), counted against 500 bytes:

| Item | Bytes |
| --- | --- |
| Hot record, with the three leading position totals (output stock, cash position, sales outlook mean) | 64 |
| Stocks: input stocks (four × 8), work in progress | 40 |
| Physical rates: output and four input use rates | 40 |
| Unit cost, markup, pressure's demand since review, revenue since review | 32 |
| Sales outlook width and last surprise | 16 |
| Review exposures, six lumpy kinds × 8 (price, way, entry and exit, closure, investment, vacancies) | 48 |
| Attention rates, six × 4 | 24 |
| Standing money rates (wages, dues, rents, input payments), six × 8 | 48 |
| Kink signature, one word | 8 |
| Arena references, eight `CellListRef`s × 8 (plant per zone, kind and condition; employment, loan, deposit, supply rows; profiles) | 64 |
| **Total** | **384** |

The rest of the 500 is Stage 2–5's (trade credit, tax positions, learning), each counted in its step. The firm table's
agenda reasons are about eight: two hazards (equipment failure, accident on held units), reviews, wakes, schedules,
kink days, dues and wear.

**Unit tests**
- `produce_target_formula`, including the kink at zero.
- `price_moves_only_past_menu_cost`.
- `price_is_a_point`.
- `pressure_defined_at_zero_stock`.
- `markup_update_speeds`.
- `way_choice_cheapest`.
- `input_order_counts_financing`.
- `found_compares_value_and_alternative`.
- `close_compares_continuing_and_winding_down`.

**Live checks**
- `LC-1-06`: FRM.17 and FRM.18 families clean.
- `LC-1-07`: every posted price is a point of its trade's table (REP.34), and prices change only on review or wake days.
- `LC-1-08`: SRV.7 and FRM.19 reads — the frequency and size of price changes, and the markups, are reported per trade.
- `LC-1-09`: every founding names a founder, the money it paid and the plant it bought (FRM.16, FRM.21).
- `LC-1-45`: firms end every year in every industry that has firms, by closure and by default of payment, each with an
  estate or a successor, its sales and its waterfall (FRM.15, FRM.21, L3); no firm keeps failing payments past its
  grace without ending.

**Budget**:
- Firm visits are architecture §13.2's "Row visits"; price, way, closure and founding reviews are "Occasion
  evaluations"; realisations are "Physical flows realised".
- Estates: about 5 000 firm endings a day at the opening's size, grouped by (part, occasion) into about 800 estates,
  each an individual for weeks; counter `phx_frm.estates_open`, ratcheted, against architecture §9.1's estimate.
- Counters, ratcheted: `phx_frm.price_reviews`, `phx_frm.price_changes`, `phx_frm.foundings`, `phx_frm.closures`,
  `phx_frm.defaults`, `phx_pop.bytes_per_firm_cell`.

**Guards**: none new.

**Not allowed**:
- a markup as a primitive, or applied to a factory price as a retail price (SRV.8);
- a cost line as a share of revenue (FRM.20);
- a birth rate or an exit rate;
- a price off the point table;
- a firm that fails payments for ever without ending;
- a founding that reads another firm's private state.

**Done when**
- [ ] Firms produce, price, buy inputs, are founded and end from their own states; the families are clean.
- [ ] LC-1-06 to LC-1-09 and LC-1-45 pass once the circular flow is closed (from S1.12).
- [ ] Two reviews are done.

---

### S1.04 — `sys-cap`: plant

**Status**: planned

**Clauses**:
- STATE: CAP.1.
- DECISION: CAP.3, CAP.4; FRM.8 *(completes it, with S1.03's part)*.
- MEASURE: CAP.10 *(its reads published through STA from S1.14)*.
- PROCESS: CAP.5, CAP.6; REP.24 *(part: plant's wear and repair between condition classes)*.
- INVARIANT: CAP.8, CAP.9.
- FORBID: CAP.11, CAP.12.
- PRIMITIVE: CAP.13.
- Construction projects and infrastructure (CAP.2, CAP.7) are S2.05.

**Architecture**: §7.10, §4.4 (purchases).

**Depends on**: S1.03.

**Goal**: plant as real units:
- bought from named producers of capital goods, paid in stages, in service after the lead time;
- worn by use and age;
- maintained, repaired, sold or scrapped by its owner's own comparison;
- invested in only when the firm's own value of the project beats its marginal cost of money and hurdle, and it can
  fund it.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-base/src/capital.rs` | `CapKind` (declared: capacity per unit, life, lead time, wear curve) |
| `crates/interfaces/if-firm/src/plant.rs` | `CapUnit` for individuals, `(zone, kind, condition)` counts for cells |
| `crates/systems/sys-cap/src/rules/invest.rs` | CAP.3 |
| `src/rules/maintain.rs` | CAP.4 |
| `src/hazards.rs` | equipment failure and accidents declared with `ActsOn::Holding(class)` (S0.10), screened by `phx-pop` at 3b |
| `src/handlers/*.rs` | 3e failure outcomes; 4a completions; wear realised at visits and at its booked days; 5c investment and maintenance decisions on review and wake days |
| `data/<country>/CAP.toml` | capital kinds, lives, lead times, wear curves (TECHNOLOGY); hurdle and horizon type sets (PREFERENCE) |

**Design**

- **Investment** (CAP.3), a lumpy decision on the firm's investment review days (quarterly by default):
  - Value the project with VAL.8's `project_value`: the expected extra output sold over the plant's life at expected
    prices, less running costs, discounted at the firm's marginal cost of money now: its quoted borrowing rate for new
    debt (its bank's quote, S1.09) and its owners' required return, weighted by the mix of debt and own money its
    management's leverage tolerance would fund the project with (CAP.3 names both).
  - Invest when value − cost ≥ hurdle × cost and the firm can fund it: cash beyond its buffer, plus an accepted loan
    offer.
  - Utilisation above its target raises expected extra sales; the width of its demand outlook raises the option value
    of waiting, which is a hurdle term (Dixit and Pindyck, 1994).
- **Purchase** (CAP.5): an order to a named producer of the capital good (a bilateral contract, MKT.7), paid in stages
  on the contract's schedule and delivered after the lead time. It is a commitment until delivery (REG.10) and in
  service when complete.
- **Wear** (CAP.6): condition classes move by use and age on the declared curve. The next day a unit class reaches
  its next condition is computed from the curve and booked on the agenda (the wear reason), so wear is realised only
  then and at visits. Depreciation is one schedule, charged both to income (ACC) and to the unit. Failures and
  accidents are hazards on held units (`ActsOn::Holding`, stream `CAP.failure`), screened at 3b by `phx-pop`, whose
  outcome `sys-cap` applies at 3e by moving units to a failed class; a catastrophe damages units (S0.25's two-level
  draw).
- **Maintain, repair, sell or scrap** (CAP.4): compare the value of each option with keeping as is. Selling goes
  through a posted or bilateral market; scrapping retires the unit (CAP.8).
- **Capacity** (CAP.9): a firm's capacity per way is the least over its plant kinds, labour and inputs of what each
  allows. This is the scarcest-input rule of CAP.1, a real limit: `DeclaredLimit::from_physical` over the units the
  firm holds (S0.09's physical token, built by `phx-ledger`), never a bound.

**Unit tests**
- `invest_only_above_hurdle_and_funded`.
- `waiting_value_rises_with_uncertainty`.
- `wear_moves_condition_classes`.
- `capacity_scarcest_input`.

**Live checks**
- `LC-1-10`: CAP.8 — per owner and kind, capital next day equals today plus completions minus retirements plus
  transfers.
- `LC-1-11`: CAP.9 — no output exceeds the capacity that made it.
- `LC-1-12`: every investment is a purchase from a named producer, with a commitment until delivery; CAP.10's reads
  (investment's share and volatility, its response to borrowing costs and utilisation, the capital stock's age) are
  published.

**Budget**: investment decisions are lumpy occasions (architecture §13.2); plant is counted per (zone, kind,
condition) for cells; wear realisations are in "Physical flows realised". Counters, ratcheted: `phx_cap.investments`,
`phx_cap.failures`, `phx_cap.wear_realisations`.

**Guards**: none new.

**Not allowed**:
- an investment rate;
- capacity from nowhere;
- plant that moves with its owner;
- depreciation charged twice.

**Done when**
- [ ] Plant is bought, worn, maintained and scrapped by owners' comparisons.
- [ ] LC-1-10 to LC-1-12 pass.
- [ ] Two reviews are done.

---

### S1.05 — `sys-gds`: goods, commodities, stocks and extraction

**Status**: planned

**Clauses**:
- STATE: GDS.1, GDS.2; GDS.3 *(part: goods and commodities; land is HSG's, S2.05)*.
- DECISION: GDS.4, GDS.5, GDS.6.
- PROCESS: GDS.7, GDS.8, GDS.9; GEO.9.
- INVARIANT: GDS.10; GEO.12.
- MEASURE: GDS.11.
- FORBID: GDS.12.
- PRIMITIVE: GDS.13.

**Architecture**: §7.9, §8.

**Depends on**: S1.04.

**Goal**: physical goods keyed by grade and place; stocks as lots with cost; commodities extracted from deposits that
deplete; markets between firms; spoilage and storage as different things; supply shocks from weather and
catastrophes at named places.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-firm/src/goods.rs` | `GoodKey { product, grade, place: ZoneId or site }`; the markets declared per good and place |
| `crates/systems/sys-gds/src/rules/extract.rs` | GDS.4: how much of a deposit to work (Hotelling's rule against the extractor's own price outlook) |
| `src/rules/stockist.rs` | GDS.6: a merchant's buy, hold and sell decision |
| `src/pressure.rs` | the stocked-good pressure fact read by S1.03's price review |
| `src/handlers/*.rs` | 5c extraction decisions and stockists' orders into the markets, which `phx-market` meets at 6a on business days; extraction applied as intents that `phx-geo` applies to its deposits; spoilage as a physical standing flow realised at visits and kinks; 3e crop and stock destruction from weather and catastrophes |
| `data/<country>/GDS.toml` | grades, spoilage rates, storage technology (TECHNOLOGY) |

**Design**

- **Markets** (GDS.7): standardised commodities meet each business day in a call auction at each place (MKT.3;
  architecture §6.1's 6a); other goods between
  firms are posted list prices (MKT.6) or bilateral supply contracts (MKT.7) with terms, lead times and volumes (as
  lines). Retail is SRV's.
- **Extraction** (GDS.4, GEO.9):
  - an extractor works its deposit when today's price exceeds its expected discounted future price net of the
    extraction cost (the Hotelling comparison through its own outlook and patience), up to its plant's capacity;
  - the deposit depletes exactly (GEO.12): the quantity is an intent that `phx-geo`, the deposit's one writer, applies;
  - where the richest part goes first, the grade falls with the quantity taken, by the deposit's declared curve.
- **Buyers** (GDS.5) are S1.03's input orders.
- **Stockists** (GDS.6): buy when the expected price at the horizon, less storage, spoilage and the financing cost at
  their marginal rate, exceeds today's price; sell when it does not.
- **Spoilage and storage** (GDS.8): spoilage removes units at its own cost by the declared rate, lazily like production
  (architecture §7.4), each realisation a transformation record naming the stock. Storage is a service bought from whoever owns the room. They are never one number.
- **Supply shocks** (GDS.9): weather (S0.13) sets yields of crops at named places; catastrophes destroy stocks and crops
  through the two-level draw.
- **The GDS.10 family**: per good and place, opening stock plus produced plus arrived equals consumed plus shipped plus
  spoiled plus destroyed plus closing stock.

**Unit tests**
- `hotelling_extract_decision`.
- `stockist_carry_condition`.
- `spoilage_exact_units`.

**Live checks**
- `LC-1-13`: GDS.10 clean every close.
- `LC-1-14`: GEO.12 clean.
- `LC-1-15`: GDS.11 reads are reported: volatility against stocks, the basis between places against freight, and
  producer prices moving before consumer prices.
- `LC-1-46`: on an experiment copy (N6) with a drought declared at one place, the price there rises before prices
  elsewhere (GDS's Done when).

**Budget**: commodity auctions are a few thousand a business day; posted list prices between firms are inside the
meetings line; spoilage is in "Physical flows realised". Counters, ratcheted: `phx_gds.auctions`,
`phx_gds.extraction_orders`.

**Guards**: none new.

**Not allowed**:
- a commodity price from a path;
- negative inventory;
- extraction without a deposit;
- spoilage and storage as one number.

**Done when**
- [ ] Goods reconcile by place.
- [ ] LC-1-13 to LC-1-15 and LC-1-46 pass.
- [ ] Two reviews are done.

---

### S1.06 — `sys-srv`: services and distribution

**Status**: planned

**Clauses**:
- STATE: SRV.1, SRV.2.
- DECISION: SRV.3, SRV.4; HH.5 *(part: the choice of seller, with S1.12)*.
- PROCESS: SRV.5, SRV.6; REP.37; REP.22 *(part: tastes over sellers)*.
- MEASURE: SRV.7.
- FORBID: SRV.8.
- PRIMITIVE: SRV.9.

**Architecture**: §7.9 (choice groups, pieces, the seller spread), §8 (posted prices).

**Depends on**: S1.05.

**Goal**: most of the economy's output:
- services produced and consumed the same day, with capacity that perishes;
- distributors holding goods and selling at posted retail prices;
- households choosing among the providers they can reach, in choice groups;
- retail prices that include the margin, the freight to the outlet and consumption tax.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-firm/src/retail.rs` | provider and outlet facts: capacity per day, region served, posted price; distributors' stock |
| `crates/systems/sys-srv/src/rules/*.rs` | SRV.3's staffing and distributors' stock levels; the services' **pressure** fact (fill against target fill) that S1.03's price review reads; `sys-frm` remains the one writer of every posted point |
| `src/groups.rs` | the retail and service market kinds' choice-group declarations (the group key, the reach) for `phx-market`'s posted form, which meets them at 6a |
| `data/<country>/SRV.toml` | service technologies; the reach of shopping by distance (TECHNOLOGY of travel); taste distributions per preference type (PREFERENCE) |

**Design**

- **Meetings** (SRV.5, REP.37), daily, including non-business days where sellers open (TIME.8):
  1. Per (group, category), the group's budget (from standing flows) and its needs by quantity (HH.20) meet the posted
     prices of the seller cells and individuals within reach.
  2. Choice probabilities are multinomial logit over each seller's price, distance from the group's zone and the
     type's taste distribution, among the sellers in the group's **reach**: those within its declared search cost
     (REP.18), about ten at the design point. Gumbel tastes give logit exactly (McFadden, 1974). The taste draws are
     the stream `SRV.taste`; capacity lots `SRV.capacity_lot`.
  3. Counts are drawn over seller cells by conditional binomials.
  4. Capacity binds by lot, and the rest re-choose in rounds that are counted.
  5. Each buyer cell pays its own budget by its pooled leg; each seller is credited from the match set by keyed
     reduction (architecture §7.9), so every unit of spending names its seller through the match-set record.
  6. Sales per seller cell are totals, spread on its review days (S0.23).
- **Distributors** (SRV.2) hold stock bought at wholesale (S1.05); their posted points are set by `sys-frm`'s review
  (S1.03) over their unit cost, which includes the wholesale cost. Their
  margin is the difference between what they paid and what they charge; it is never a stated markup (SRV.8).
- **Retail price** (SRV.6): the posted price includes consumption tax (S1.11) and the distributor's freight cost of
  bringing goods to the outlet (S1.07). Until those steps exist, each component is absent and marked by a placeholder
  naming TAX (S1.11) and FRT (S1.07), which those steps retire.
- **Services** (SRV.1): capacity per day from staff hours and plant. Unused capacity is lost at the day's close; there
  is no stock of services.

**Unit tests**
- `logit_shares_from_gumbel_tastes`.
- `capacity_rechoice_by_lot`.
- `retail_price_components`: over given wholesale cost, freight and tax, the components add up.

**Live checks**
- `LC-1-16`: SRV.7 — the services share of output and employment, the retail margin and its compression when wholesale
  costs rise, and the frequency and size of retail price changes are reported.
- `LC-1-17`: no service is stored: unused capacity at the close equals capacity minus sales, and nothing carries over.
- `LC-1-18`: HH.15 (part) — every unit of household spending names its seller through a match-set record, and the
  sellers' credits per meeting sum to the buyers' debits.

**Budget**: architecture §13.2's "Meetings and choice groups" line: 0.2 M group-products at 650 ns with about ten
sellers in reach (the review's prototype: 0.6 µs at 10 sellers, 1.9 µs at 30, 6 µs at 100, on one x86 core). Reach is
RESOLUTION-tested on the ladder. Counters, ratcheted: `phx_market.sellers_in_reach`, `phx_market.rechoice_rounds`,
`phx_srv.unused_capacity`.

**Guards**: none new.

**Not allowed**:
- a household buying at the factory gate without going there;
- a retail price as factory price × markup;
- a stored service;
- a meeting per household instead of per group.

**Done when**
- [ ] Services and retail run daily through choice groups, with capacity and re-choice.
- [ ] LC-1-16 to LC-1-18 pass.
- [ ] Two reviews are done.

---

### S1.07 — `sys-frt`: freight within each country, vehicles and infrastructure

**Status**: planned

**Clauses**:
- STATE: FRT.1, FRT.2, FRT.3; GEO.4 *(part: the network and its capacities; the infrastructure's life, maintenance
  and condition are S2.05's, with CAP.7)*.
- DECISION: FRT.4, FRT.5.
- PROCESS: FRT.6, FRT.7, FRT.8.
- INVARIANT: FRT.9; GEO.13.
- MEASURE: FRT.10.
- FORBID: FRT.11.
- PRIMITIVE: FRT.12; GEO.18.
- Freight across borders is S5.05. This step retires S1.06's placeholder naming FRT.

**Architecture**: §7.10, §8.

**Depends on**: S1.06.

**Goal**: moving goods costs time and money and needs a vehicle, a route and capacity:
- the opening infrastructure, with network segments and their capacities, owned by named parties;
- vehicles as capital units;
- carriers offering room from where their vehicles stand;
- shippers booking when the price gap exceeds the freight;
- goods pledged to the carrier in transit.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-geo/src/network.rs` | network segments (road, rail, sea lanes, pipelines, power lines) with capacity per day, generated at map time from the terrain and region centres, owned by declared parties (ENDOWMENT); the module is S0.13's declared extension point, filled here |
| `crates/interfaces/if-firm/src/freight.rs` | `Vehicle`, `Route`, `Shipment` |
| `crates/systems/sys-frt/src/rules/*.rs` | FRT.4: carriers' repositioning and the carriage **pressure** fact (load factor against target) that `sys-frm`'s review reads to set carriers' posted points; FRT.5: shippers' bookings |
| `src/handlers/*.rs` | 5c bookings and repositioning; `phx-market` meets the route markets (posted) at 6a; arrivals realised on the day they are booked on the agenda; 3e closures from catastrophes |
| `data/<country>/FRT.toml` | vehicle technologies, speeds, running and keeping costs, loading times (TECHNOLOGY) |

**Design**

- **Infrastructure** (GEO.4): segments between tiles with a mode and a capacity per day, generated at map time by a
  recorded procedure (shortest-path trees between region centres over land and, for sea lanes, between ports),
  declared as ENDOWMENT. They are owned by the treasury or by named firms from GEN.
- **Routes** (FRT.2): paths over segments between two sites, with every transfer between modes.
- **Carriers** (FRT.4): offer room on routes from where their vehicles stand. The price of room is the carrier's
  posted point, set by `sys-frm`'s review (S1.03) over running cost and the load-factor pressure `sys-frt` supplies.
  They reposition empty vehicles when the expected margin elsewhere beats the empty run's cost.
- **Shippers** (FRT.5): book room when the price gap between places exceeds the freight and loading costs. Otherwise
  they hold, sell locally or do not trade.
- **Transit** (FRT.6): goods are pledged to the carrier (a lien, S0.14) and released on arrival; a shipment's arrival
  day is booked on the agenda, so shipments in transit are touched only at departure and arrival. Freight enters the
  delivered price.
- **Capacity** (GEO.13, FRT.9): a segment carries no more a day than its capacity. Bookings beyond it are refused by
  lot (stream `FRT.capacity_lot`), never repriced by a multiplier (FRT.7).
- **Failure** (FRT.8): the goods remain the owner's and are recovered after a declared delay and cost; lost cargo is a
  claim in the carrier's estate.

**Unit tests**
- `route_capacity_binds_by_lot`.
- `shipper_books_only_above_freight`.
- `arrival_day_from_route_and_speed`: over a given route's segments, modes and speeds.

**Live checks**
- `LC-1-19`: FRT.9 and GEO.13 — every shipment has one owner, carrier and vehicle; no vehicle is in two places; no
  segment is over capacity.
- `LC-1-20`: FRT.10 — freight rates and price gaps between places are reported, and gaps track freight.
- `LC-1-47`: every lien of goods in transit is released on arrival or passes to the owner's claim in a failed carrier's
  estate (FRT.6, FRT.8).

**Budget**: shipments about 10⁵ a day, each touched at booking and arrival; within "Institutions, markets".
Counters, ratcheted: `phx_frt.shipments`, `phx_frt.refused_bookings`.

**Guards**: none new.

**Not allowed**:
- instantaneous transport;
- room without a vehicle;
- goods in transit owned by nobody;
- a location gap closed by formula.

**Done when**
- [ ] Goods move between places only on booked vehicles over real routes.
- [ ] LC-1-19, LC-1-20 and LC-1-47 pass.
- [ ] Two reviews are done.

---

### S1.08 — `sys-lab`: labour

**Status**: planned

**Clauses**:
- STATE, DECISION, PROCESS, INVARIANT, MEASURE, FORBID, PRIMITIVE: LAB.1–LAB.4, LAB.6–LAB.17, including collective
  bargaining (LAB.10) where declared union coverage exists, and the minimum wage (LAB.11).
- DECISION: LAB.5 *(part: search within the region; moving for work is S2.05, retraining S6.02)*; FRM.7 *(completes
  it: employing, with S1.03's buying)*; HH.6 *(part: search, acceptance, quits and retirement, with S1.12)*; REP.34
  *(part: wage points)*.
- PROCESS: REP.22 *(part: match quality as the taste over vacancies)*.
- This step retires S0.25's placeholders naming LAB for the opening employment lines' decisions.

**Architecture**: §4.2 (applications as counted messages), §7.5, §7.7 (employment lines), §13.2 ("Labour matching").

**Depends on**: S1.07.

**Goal**: people's time sold to employers by searchers who apply, choose and quit, for wages set by employers who
post vacancies at wage points and compete for workers:
- employment, unemployment, vacancies and wages are reads of contracts, applications and offers;
- wages move only by renegotiation at review dates, collective agreements, or turnover.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-labour/src/*.rs` | vacancy (LAB.2), application (a day-local message with counts), offer, separation, the employment line's terms (S0.25), union coverage |
| `crates/systems/sys-lab/src/rules/post.rs` | LAB.4: posting vacancies and choosing the wage offer |
| `src/rules/layoff.rs` | LAB.4: layoffs |
| `src/rules/search.rs` | LAB.5: the searcher's applications and reservation |
| `src/rules/select.rs` | LAB.7: selection among applicants |
| `src/rules/renegotiate.rs` | LAB.17 |
| `src/rules/bargain.rs` | LAB.10 |
| `src/handlers/*.rs` | 5c posting, applications, offers and acceptances, one step of each a day (below); 4a jobs starting and ending; notice and severance as legs of the separation instruction, settled at stage 7 |
| `crates/interfaces/if-pop/src/labour.rs` | the household's labour decision points `search`, `accept`, `quit`, `retire` (the decider's crate), registered and implemented by `sys-lab` since they are LAB's clauses; they read the household's reservation through the rule handle `reservation`, which `sys-hh` implements at S1.12 (until then a placeholder naming HH: benefits and other household income plus the type's value of leisure) |
| `data/<country>/LAB.toml` | search effort and reach; the meeting hazard per application (TECHNOLOGY of search); notice, severance and minimum-wage law; union coverage; wage points per occupation family (POLICY of the trade); vacancy patience and the renegotiation protocol (PREFERENCE of management) |

**Design**

- **Vacancies** (LAB.4), on the employer's review days:
  - post when the marginal worker's expected revenue (output price outlook × marginal product of the way) exceeds the
    wage plus the financing cost of paying wages before sales;
  - the wage offer is the wage point that the employer's own fill history says fills within its target time, moved up
    one point after a vacancy stays open past its patience and down one point after quick fills: posted wages that
    adapt to vacancy duration (Faberman and Menzio, 2018; wage posting's prevalence, Hall and Krueger, 2012), listed in
    `SHAPES.toml`; dispersion across employers follows (Burdett and Mortensen, 1998);
  - the minimum wage is a declared limit on the points offered (LAB.11).
- **Search** (LAB.5, LAB.8), in rounds on searchers' occasions. **A round is a day** (architecture §7.9):
  applications sent at 5c reach employers, who make offers at the next day's 5c, which applicants accept or refuse at
  the 5c after; a match takes at least three days, as real hiring does.
  - a cell's searchers in a role apply to vacancies within their region and reach, up to their effort;
  - the count applying to each vacancy is a multinomial over the visible vacancies by expected value (wage against
    reservation) with taste draws (REP.22, stream `LAB.match_taste`);
  - applications are counted messages that live across days; each is seen by the employer with the meeting hazard per
    application (stream `LAB.meeting`);
  - the employer selects by skill and experience, and among equals by lot (LAB.7, stream `LAB.select_lot`);
  - the member offered accepts if the offer plus its match-quality taste beats its reservation (McCall, 1970, listed
    in `SHAPES.toml`), where the reservation is the rule handle above;
  - those not chosen apply again in the next round.
- **Hiring** creates or extends a row on the employment line of (occupation family, skill, wage point, hours, notice,
  severance, start band, region). The member's cell changes a profile attachment in place, or splits if its key or a
  kink changes (architecture §7.5).
- **Layoffs** (LAB.4): when the employer is sure it cannot use the work — its expected output over the notice period
  falls below what the staff produce. The laid-off members are drawn from the line (REP.23, stream `LAB.layoff`) and
  paid notice and severance as the contract owes, as legs of the separation instruction settled at stage 7.
- **Quits and retirement** (LAB.6): on the employee's review occasions, when an offer or retirement is better for its
  household.
- **Renegotiation** (LAB.17): at each contract's review date the employer offers the wage point nearest the lesser of
  the work's expected revenue per hour (its output price outlook × marginal product) and the wage its own recent
  fills show the market pays for that occupation family and skill. The employee accepts if the offer is at least its
  reservation; otherwise it counters at its reservation's point, which the employer accepts if that is still below
  the work's expected revenue, and otherwise the employee quits. The protocol ends in two moves. A new wage is a new
  line; members move rows.
- **Collective bargaining** (LAB.10): where coverage exists (ENDOWMENT), a union party — opened by GEN III (S1.15) —
  negotiates one agreement for the covered lines by a declared alternating-offers protocol (Rubinstein, 1982, listed
  in `SHAPES.toml`). A strike is a real stoppage: no output,
  no wages, for its days.
- **Sticky wages** (LAB.9) follow from the above: nothing else moves a contract's wage (LAB.15).

**Unit tests**
- `vacancy_value_test`.
- `wage_point_adapts_to_fill_history`.
- `reservation_components`.
- `selection_ties_by_lot`.
- `severance_owed_by_terms`: over given terms and tenure.
- `renegotiation_protocol_terminates`.

**Live checks**
- `LC-1-21`: LAB.13 — no person has more hours than a day; headcount equals contracts; no wage paid to nobody.
- `LC-1-22`: LAB.14 — the Beveridge relation, Okun's co-movement, unemployment durations, wage dispersion within
  occupation families and job-to-job flows are reported.
- `LC-1-23`: the count of matches equals the sum of acceptances (no aggregate matching function, LAB.15).

**Budget**: architecture §13.2's "Labour matching": 0.1 M searching groups at 1 µs, which holds for about fifteen
vacancies visible per group (the review's prototype: about 50 ns per vacancy visible). Counters, ratcheted:
`phx_lab.searching_groups`, `phx_lab.vacancies_visible`, `phx_lab.applications`, `phx_lab.rounds_to_match`.

**Guards**: none new.

**Not allowed**:
- an aggregate matching function;
- a wage changed outside renegotiation, bargaining or turnover;
- an exogenous unemployment rate;
- employment without an employer.

**Done when**
- [ ] Labour flows are reads of individual applications, offers and contracts.
- [ ] LC-1-21 to LC-1-23 pass.
- [ ] Two reviews are done.

---

### S1.09 — `sys-bnk`: deposits and lending, one tier

**Status**: planned

**Clauses**:
- STATE: BNK.1, BNK.2, BNK.17, BNK.18; MON.4.
- DECISION: BNK.4, BNK.5, BNK.6, BNK.20; REP.34 *(completes it: lenders' rate points)*.
- PROCESS: BNK.8, BNK.19; REP.22 *(part: tastes over lenders)*.
- INVARIANT: BNK.11.
- FORBID: BNK.14; BNK.15 *(part: the one assessment prices; provisions read it from S2.01)*.
- PRIMITIVE: BNK.16.
- This step retires S0.25's placeholders naming BNK for the opening deposit and loan lines' decisions.
- Provisions and workouts (BNK.7, BNK.9, BNK.10, BNK.12) are S2.01; the bureau (BNK.21) S2.10; non-banks (BNK.22)
  S3.08; syndication (BNK.3) S3.04.
- The bank's marginal cost of funds is a placeholder naming BFL (S2.06), and so are its deposit-rate rule and its use
  of the central bank's facilities; the capital a loan consumes is a placeholder naming BCP (S2.07).
- Borrowers in Stage 1 are firms and the opening loans' households paying down; households apply for new loans from
  S2.05, with the household borrowing decision.

**Architecture**: §4.4, §4.5 (deposits, banking arrangement), §9.2.

**Depends on**: S1.08.

**Goal**: loans as named contracts, written by a bank that creates the deposit it lends:
- each bank quotes from its own marginal cost of funds, its own assessment of the borrower, the capital the loan
  consumes and its cost of making it;
- banks decline, and tighten when their state worsens;
- borrowers shop;
- deposits pay interest, and can be withdrawn as banknotes.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/*.rs` | loan line kinds as declared data (term amortising, bullet, balloon; credit lines; mortgage terms from `if-property`, which sits below it), applications and quotes as messages across days, the assessment view |
| `crates/interfaces/if-banking/src/*.rs` | deposit kinds and their rates as each bank's decision; the banking arrangement |
| `crates/systems/sys-bnk/src/rules/quote.rs` | BNK.4 |
| `src/rules/decline.rs` | BNK.5 |
| `src/rules/assess.rs` | BNK.20: the bank's own probability of default and loss given default, learned from its own book |
| `src/rules/deposit_rate.rs` | the bank's deposit rates, a placeholder rule naming BFL (S2.06) |
| `src/rules/facilities.rs` | the placeholder use of the central bank's facilities (naming BFL, S2.06) |
| `src/handlers/*.rs` | 5c quotes and declines at the answering sub-step of applications; 5c acceptances write the loan's composite instruction, whose disbursement leg creates the deposit (MON.6) when it settles at 7c; 8d facility requests |
| `data/<country>/BNK.toml` | operating cost per loan (TECHNOLOGY); required return on capital, risk appetite, how fast standards move (PREFERENCE of management); the placeholders `cost_of_funds` (naming BFL: the central bank's deposit-facility rate), `capital_charge` (naming BCP: a declared risk weight per loan kind) and the reserve target for the facilities (naming BFL) |

**Design**

- **Assessment** (BNK.20, BNK.15): each bank sorts borrowers into **classes** by what it observes — for a cell, the
  steps of its debt service over income and loan-to-value and its credit-record stage (all in its key or positions);
  for an individual, its reported accounts' ratios in steps — and learns each class's default probability from **its
  own book**: an adaptive outlook (VAL.6) of the defaults it has seen in that class, starting from its opening
  history (GEN.5; VAL.10 for a class it has never seen). It reads only its own lines' payment records and what the
  application carries; other lenders' records wait for the bureau (S2.10). Its loss given default per collateral
  class, unsecured included, is its own adaptive outlook of the recoveries it has realised, so no recovery rate is
  fixed (BNK.14). The same assessment prices and, from S2.01, provisions (BNK.15). The form — class frequencies
  learned adaptively — is listed in `SHAPES.toml`.
- **Quote** (BNK.4): `rate = cost_of_funds + PD × LGD + capital_charge × required_return + operating_cost / principal`.
  - `cost_of_funds` is the placeholder until BFL: the central bank's deposit-facility rate, which the bank can always
    earn instead.
  - `capital_charge` is the capital the loan consumes: the placeholder's declared risk weight per loan kind until BCP.

  The rate is put on the lender's price points (REP.34).
- **Decline** (BNK.5): when the expected return is below the required return, when the borrower fails the bank's
  standards, or when its capital or liquidity cannot carry the loan. Its standards (the highest loan-to-value and
  income multiple, the lowest coverage) move on its review days by its management's declared speed against the gap
  between its outlook of losses on its own book and the losses its pricing allowed: tighter when losses run above,
  looser when below. The form is listed in `SHAPES.toml`.
- **Borrowers shop** (BNK.6): application messages to the lenders the borrower can reach; quotes come back; it takes
  the quote with the best value to it (VAL.8) plus its taste for each lender (REP.22, stream `BNK.lender_taste`), if
  that value is positive, or goes without.
- **Cells** (BNK.20): the same terms go to every member who applies on one occasion. Those who accept split into their
  part with the new line (REP.8).
- **Lending creates a deposit** (BNK.8, MON.6): the acceptance writes one composite instruction at 5c, whose
  disbursement leg credits the borrower's deposit at the lending bank when it settles at 7c. Reserves move only when
  the borrower pays elsewhere.
- **Instalments** (BNK.19) fall due on their dates through S0.17's batches, with pooled flows; prepayment is a
  borrower's decision on its `refinance` review (S1.12).
- **Deposit rates** (a placeholder naming BFL): each bank sets its rate point per deposit kind on its review days: it
  moves by its management's speed toward the rate that would bring its deposits to its funding need (the loans it
  expects to make less its own funds), pulled toward competitors' posted rates it can see. The form is listed in
  `SHAPES.toml` as a placeholder naming BFL; it is not the firms' pricing form. S2.06 retires it, and `sys-bfl` is
  then the one writer of deposit rate points.
- **The central bank's facilities** (placeholder naming BFL): at 8d a bank whose reserves are below its declared
  target borrows the difference at the lending facility against collateral the central bank accepts (S1.10), and one
  above it places the excess at the deposit facility. It is the bank's own request, never a sweep placed for it
  (MKT.9).
- **Banknotes** (MON.4): depositors withdraw and deposit banknotes by their own decision (HH.7's liquidity choice,
  S1.12). Banks get notes from the central bank against reserves.

**Unit tests**
- `quote_components`.
- `decline_on_standards`.
- `same_terms_for_members_of_one_occasion`.
- `class_default_probability_adapts`: over given observed defaults.
- `disbursement_legs_from_terms`: the composite instruction's legs from given terms.

**Live checks**
- `LC-1-24`: BNK.11 — each bank's loan book equals the sum of its loan lines, and its change reconciles.
- `LC-1-25`: declined applications are visible and counted per bank (BNK.13 read, completed S2.12).
- `LC-1-26`: MON.6 in practice — every new loan's disbursement created a deposit at the lender, and money-stock changes
  reconcile to issuers' transactions (LC-0-18 still clean).

**Budget**: applications and quotes are occasions; loan rows are in §13.1's rows line. Counters, ratcheted:
`phx_bnk.applications`, `phx_bnk.declines`, `phx_bnk.loans_written`, `phx_bnk.facility_requests`.

**Guards**: none new.

**Not allowed**:
- lending out of deposits or reserves;
- a loan book that is a number;
- one assessment shared by all lenders;
- a fixed recovery rate;
- a default probability from weights no book taught it.

**Done when**
- [ ] Banks quote, decline and lend by creating deposits; borrowers shop.
- [ ] LC-1-24 to LC-1-26 pass.
- [ ] Two reviews are done.

---

### S1.10 — `sys-cb`: settlement and a fixed policy rate

**Status**: planned

**Clauses**:
- STATE: CB.1 *(part: the domestic balance sheet)*; MON.15.
- PROCESS: CB.7 *(part: the corridor's two facilities at a fixed rate)*; CB.10 *(part: net income remitted)*.
- DECISION: CB.6 *(part: eligible collateral and haircuts as a declared placeholder naming CB)*.
- The policy committee (CB.4), operations, lender of last resort and financing regimes are S3.02.
- The fixed rate is a placeholder naming CB (S3.02).

**Architecture**: §6.1 (stage 8), §6.5.

**Depends on**: S1.09.

**Goal**: each country's central bank as a real balance sheet:
- reserves, banknotes and the treasury's account as liabilities;
- a deposit facility and a lending facility at a fixed declared rate, meeting whatever quantity comes to them in the
  fund stage;
- intraday credit at settlement;
- banknotes issued and retired against reserves.

**Files**

| File | Purpose |
| --- | --- |
| `crates/systems/sys-cb/src/*.rs` | the facilities as administered markets (MKT.8) at 8d; intraday credit terms (MON.3, MON.15); note issuance |
| `data/<country>/CB.toml` | the placeholder policy rate and corridor width, and the placeholder list of eligible collateral with haircuts (government bills and performing loans; both SHAPE, placeholder naming CB, S3.02); intraday credit terms (POLICY) |

**Design**:
- The facilities meet at 8d–8e the banks' own requests (S1.09), against the eligible collateral, at the placeholder
  rates: an administered price with a real quantity response (MKT.8).
- Intraday credit closes at 8f (architecture §9.2 handles a shortfall).
- Banknotes are issued to banks against reserves and retired when returned.
- Net income is remitted to the treasury on the declared dates (CB.10), completed at S3.02.

**Unit tests**: `facility_legs_from_request`: the legs of a lending-facility loan and its collateral pledge from a given
request, rate and haircut; `note_issue_legs`: reserves against notes, one for one.

**Live checks**: `LC-1-27`: MON.7 and MON.9 clean with the central bank's facilities in use; facility quantities are
reported daily.

**Budget**: negligible. Counters, ratcheted: `phx_cb.facility_uses`.

**Guards**: none.

**Not allowed**:
- a market rate equal to the policy rate by construction (CB.13);
- an overdraft for the treasury;
- a facility used without the bank's request.

**Done when**
- [ ] The facilities work at the fixed rate.
- [ ] LC-1-27 passes.
- [ ] Two reviews are done.

---

### S1.11 — The state, first cut: `sys-trs`, `sys-tax`, `sys-soc`, `sys-sov`

**Status**: planned

**Clauses**:
- STATE: TRS.1, TRS.9; SOV.1 *(part: bills)*; SOV.2 *(part: pari passu, no covenants, not callable; buybacks and
  switches are S3.03)*.
- DECISION: SOV.3 *(part: bill auctions sized by the placeholder plan)*; SOV.4 *(part: banks bid; other bidders
  arrive with their systems)*; SOV.5 *(part: a placeholder naming DLR, S3.06)*; SOC.3 *(part: the one benefit's
  claim)*.
- PROCESS: TRS.4, SOV.6; TAX.2 *(part: withholding at payroll, the consumption tax at the till, the annual return,
  remittance)*; REG.11 *(part: bills mature)*.
- INVARIANT: TRS.6, TAX.5.
- FORBID: TAX.7, SOC.7.
- STATE: TAX.1 *(part: income tax and one consumption tax)*; SOC.1 *(part: one benefit)*.
- The funding plan (TRS.2, TRS.3) is S3.03; the full tax system S5.01; the full social system and agencies' own
  decisions S5.02; the parliament's budget S5.03.
- This step retires S0.25's placeholder naming TAX (levies absent from the opening lines) and S1.06's placeholder
  naming TAX (the consumption tax in retail prices).

**Architecture**: §4.3 (levies), §4.6 (policy values), §7.8 (year-to-date positions).

**Depends on**: S1.10.

**Goal**: a state that:
- taxes named payers when bases arise — income tax withheld at payroll, and a consumption tax charged by sellers;
- pays one benefit to named households who claim it on eligibility;
- pays the wages of the public staff the opening world gives it and buys the inputs their work needs, by placeholder
  rules that name the systems that will decide them;
- funds itself by selling bills at uniform-price call auctions before it spends.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-state/src/*.rs` | levies (income tax withholding, consumption tax); the benefit rule and the income tax schedule as rule handles; the treasury's facts; the bill instrument family |
| `crates/systems/sys-tax/src/*.rs` | levy declarations with their kinks; the annual return from the per-role year-to-date positions; remittance on the calendar; the TAX.5 family |
| `crates/systems/sys-soc/src/*.rs` | eligibility events (job loss), the claim decision, payments on dates; the means test as a key-rule kink; the placeholder staffing and purchases of the opening public agencies |
| `crates/systems/sys-trs/src/*.rs` | outlays to named recipients; the cash buffer; the placeholder funding rule naming TRS.2 (S3.03) |
| `crates/systems/sys-sov/src/*.rs` | bills (SOV.1, SOV.2); auctions (SOV.6) as call auctions; banks' bid schedules (SOV.4); the primary-dealer placeholder naming DLR (S3.06) |
| `data/<country>/{TAX,SOC,TRS,SOV}.toml` | tax schedules (bands, allowances, rates) and the consumption tax's form (value-added or retail sales, POLICY of the parliament); the filing window; the benefit rule; the claiming cost in hours (TECHNOLOGY); auction formats and the buffer policy (POLICY) |

**Design**

- **Levies** (TAX.2, TAX.7):
  - Income tax is withheld per member from the employer's own year-to-date figure for that line (architecture §4.3),
    rounded per member and multiplied by the count; the employer holds it as its liability to the treasury and
    remits it on the calendar.
  - The consumption tax is ad valorem, in the form each country declares: a retail sales tax at Stage 1 in every
    opening country (a value-added tax needs input-tax positions on every business sale, which S5.01 builds). It is
    computed per unit sold at the till from the posted gross price, is the seller's liability until remitted, and is
    paid through the retail meeting's legs.
- **The annual return** (TAX.2): each adult files on a day it chooses within the filing window, as a lumpy decision:
  a member due a refund files at its first review in the window, one owing pays at its last review before the
  deadline (the form listed in `SHAPES.toml`; the filing days spread over the window, N8.9). Its assessment reads the
  per-role year-to-date positions (architecture §7.8), with capital income attributed by the country's rule, and pays
  or is refunded the difference.
- **The benefit** (SOC.3): eligibility is an event (a job lost). The claim is the member's lumpy decision on that
  event's occasion: it claims when the benefit's value over its expected spell exceeds the claiming cost (its hours at
  its value of leisure), listed in `SHAPES.toml`. Payment is a standing flow to the named household on the benefit's
  dates until eligibility ends. The means test is a key-rule kink.
- **Public staff and purchases**, until the agencies decide (SOC.8 at S5.02, appropriations at S5.03): the opening
  world's public agencies (GEN, S0.16) keep their opening employment lines and pay them; a departure is replaced by
  posting a vacancy at the line's wage point (a placeholder naming SOC, S5.02), and the inputs their work needs are
  bought at posted prices by the declared technology of public services (SOC.9's TECHNOLOGY), a placeholder naming
  SOC. No spending path is written: outlays are what these lines and purchases cost.
- **Bills** (SOV): the treasury's placeholder plan (naming TRS.2, S3.03) sizes each auction as its outlook of
  outlays less receipts over the declared horizon, plus the gap between its buffer target (POLICY) and its cash, and
  announces the calendar in advance. Banks bid schedules from their own liquidity: a bank's demand for bills at each
  price is what it would hold in bills rather than reserves, given the bill's yield against its deposit-facility
  rate and its own liquidity target (a form listed in `SHAPES.toml`). The placeholder primary dealers (naming DLR,
  S3.06) are the banks the treasury has named, each undertaking to bid, with size and price its own. The
  uniform-price call auction clears (MKT.3); unsold paper is not issued, and a failed auction is an event (MKT.10).
  Bills mature and are paid (REG.11). Bills are bought only at auction until the dealer market exists (S3.06).
- **When cash runs short**, the placeholder draws the buffer and defers discretionary purchases; it never cuts a
  statutory payment (a benefit, a wage owed, a debt service), and never borrows from the central bank.

**Unit tests**
- `withholding_per_member_rounded`.
- `consumption_tax_ad_valorem_per_unit`.
- `return_filing_day_rule`: refund early, payment late.
- `claim_decision_value_against_cost`.
- `bank_bill_schedule_monotone`.
- `uniform_price_auction_bills`.

**Live checks**
- `LC-1-28`: TAX.5 — tax received equals tax remitted by named collectors; every tax payment has a named payer and
  base.
- `LC-1-29`: TRS.6 — debt outstanding equals issuance minus redemptions, read from the register.
- `LC-1-30`: SOC.7 — every benefit is paid to a named household under its rule, after its claim.
- `LC-1-31`: auction results (cover, tail, failures) are published.
- `LC-1-48`: the means test's kink is registered and no cell holds members on both sides of it (read from the kink
  signatures).

**Budget**: levies are inside settlement's unit cost; returns are spread over the filing window (N8.9); claims are
occasions. Counters, ratcheted: `phx_tax.returns_filed`, `phx_soc.claims`, `phx_sov.auctions`.

**Guards**: none new.

**Not allowed**:
- a tax on an aggregate;
- a transfer to a sector;
- a forced buyer at an auction;
- an automatic overdraft at the central bank;
- a spending path;
- a statutory payment cut to balance the cash.

**Done when**
- [ ] The state taxes, pays a benefit on claims, pays its opening staff and funds itself with bills.
- [ ] LC-1-28 to LC-1-31 and LC-1-48 pass.
- [ ] Two reviews are done.

---

### S1.12 — `sys-hh`: households spend, work and save

**Status**: planned

**Clauses**:
- STATE: HH.1, HH.2, HH.3.
- DECISION: HH.4, HH.5; HH.6 *(completes it: participation and hours, with S1.08's search, quits and retirement)*;
  HH.7
  *(part: deposits, banknotes and bills at auction)*; REP.5.
- PROCESS: HH.13 *(part: debt service on its dates and arrears; default's consequences come with S2.01, S2.05 and
  S2.10, and HH.13 completes at S2.11)*.
- INVARIANT: HH.15.
- FORBID: HH.18, HH.19.
- PRIMITIVE: HH.20.
- Housing and borrowing (HH.8, HH.10) are S2.05; moving (HH.9) S2.05 and S5.05; insurance S4.03; voting S5.03;
  arrears actions and insolvency S2.11; HH.16–HH.17 S6.03.
- This step retires S1.08's placeholder naming HH (the reservation).

**Architecture**: §7.3 (attention), §7.4 (standing flows), §7.5, §7.9.

**Depends on**: S1.11.

**Goal**: households decide from their own state and outlooks:
- how much to spend this period;
- what to buy (needs by quantity, a budget on the rest, sellers by choice);
- whether and how much to work;
- how to hold their savings among deposits, banknotes and bills.

These decisions close the circular flow.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-pop/src/decisions.rs` | decision points `spend`, `allocate`, `hours`, `hold_savings`, `stance`, with their views; the rule handle `reservation` |
| `crates/systems/sys-hh/src/rules/spend.rs` | HH.4: the buffer-stock consumption rule |
| `src/rules/buffer.rs` | the target buffer and the propensity to consume out of it, solved from the type's preferences (below) |
| `src/rules/allocate.rs` | HH.5: needs, then CES budget shares |
| `src/rules/hours.rs` | HH.6: participation and hours |
| `src/rules/reservation.rs` | the reservation wage that S1.08's decisions read |
| `src/rules/hold.rs` | HH.7: deposits, banknotes and bills |
| `src/gen.rs` | takes over from `sys-dem` the household composition, income and wealth distributions of S0.25 (the register records the change of declarer), and adds the preference types; the household kind's positions, standing rates, review kinds and pins, through `d.pop_kind` (S0.21) |
| `data/<country>/HH.toml` | preference type sets (patience, risk aversion, taste shares and substitution elasticity, value of leisure, memory); minimum needs by composition (TECHNOLOGY of living); what drawing cash costs (TECHNOLOGY); decision schedules (weekly spending, monthly savings review) |

**Design**

- **Spending** (HH.4), a continuous decision on the household's weekly schedule and on wakes:
  - **Inputs**, as HH.4 lists them: current income; wealth and how liquid it is (its deposit and banknote positions,
    and bills); its outlooks of income and prices (S1.01); its confidence (the income outlook's width); its patience;
    the rate it earns on saving (its banks' posted deposit rates, less its price outlook); and what it can borrow — the
    loan offers it holds, none new before S2.05 (a placeholder naming HH, S2.05).
  - **Form** (listed in `SHAPES.toml`): the buffer-stock rule (Deaton, 1991; Carroll, 1997),
    `c = needs + p̂ + κ·(m − m*)`, where `m` is cash on hand over permanent income, `p̂` its permanent-income outlook
    net of needs, `m*` the target buffer and `κ` the propensity to consume out of the buffer.
  - **m* and κ are derived, never declared**: both come from solving the buffer-stock model (Carroll, 1997) for the
    type's patience β and risk aversion ρ, the real return R and the income outlook's width. The solution is a pure
    function of those inputs; since the inputs a cell holds are in steps, it is solved once per (type, width step,
    rate step) when first needed and memoised, which is exact. No propensity to consume is an input (HH.16 measures
    it).
  - A household that cannot borrow spends at most what it has. Below its needs it claims what benefits it is eligible
    for (S1.11) or goes without, recorded as an event.
  - The result is a standing flow per category (architecture §7.4).
- **Allocation** (HH.5): needs (HH.20) by quantity first; the rest by CES budget shares with the type's taste shares and
  substitution elasticity (Stone–Geary needs under a CES upper tier; listed in `SHAPES.toml`), over the prices it faces
  where it can shop. Substitution follows relative prices. Sellers are chosen at meetings (S1.06).
- **Work** (HH.6): hours and participation are lumpy decisions of each adult role on its occasions, comparing the wage
  it can get with its reservation and its value of leisure. The **reservation** (the rule handle S1.08 reads) is the
  wage at which working the offered hours leaves the household as well off as not: benefits it would receive, other
  household income, the value of leisure of those hours and its income outlook (McCall, 1970).
- **Savings** (HH.7, first cut), on the monthly review:
  - **banknotes** by the Baumol–Tobin form (Baumol, 1952; Tobin, 1956): the average holding is `sqrt(b·C ÷ (2·i))`,
    with C its spending paid in banknotes, i its deposit rate and b what drawing cash costs (TECHNOLOGY: the hours at
    its value of leisure, and any fee its bank posts);
  - **bills** only at auction: the household's own limit order, placed through its bank as agent, at its
    reservation yield — its deposit rate over the bill's horizon plus what its risk aversion asks for the bill's
    price risk (a mean–variance comparison over its own outlooks; listed in `SHAPES.toml`) — for the amount that
    comparison gives, so it buys only where the auction clears at or above that yield; bills are held to maturity
    until the dealer market exists (S3.06);
  - **deposits** hold the rest, across the kinds its banks offer by their posted rates.
- **Prepayment**, on the `refinance` review: a household with a loan prepays the part of its cash beyond its buffer
  target (m*) when the loan's rate exceeds what that cash earns on deposit, net of any prepayment fee in the loan's
  terms; the comparison is listed in `SHAPES.toml`. S2.05 adds refinancing and drawing on equity to the same review.
- **Debt service** (HH.13) is paid on its dates through pooled flows; a failed payment is arrears, recorded; the
  default's consequences arrive with S2.01.
- **Streams**: `HH.stance_taste`, `HH.hours_taste`.

**The household cell's record at Stage 1** (architecture §13.1; S0.21's table filled):

| Item | Bytes |
| --- | --- |
| Hot record, with the three leading position totals | 64 |
| Other positions: year-to-date per adult role (2 × 2 × 8), five more totals | 72 |
| Own outlooks: income and job security, each mean, width and dated last surprise (2 × 24) | 48 |
| Review exposures, ten lumpy kinds × 8 (stance, founding, search and acceptance, quit, retirement, hours, filing, claim, fertility, `refinance`: prepaying, and from S2.05 refinancing and drawing on equity) | 80 |
| Attention per kind: the own rate and g_k, two `u32` fixed-point values (10 × 8) | 80 |
| Standing rates per member, fourteen × 4 (`u32` fixed point, overflowing to the arena's side map) | 56 |
| Kink signature, one word | 8 |
| Arena references, twelve `CellListRef`s × 8 | 96 |
| **Total** | **504** |

Stages 2–6 add review kinds (dwelling, moving, borrowing, insuring, voting, asset classes), each counted in its step
against architecture §13.1's line. The household table's agenda reasons at Stage 1 are twelve: five hazards
(mortality, illness, conception, accident, damage), birthdays, reviews, wakes, schedules, carried occasions, kink days
and standing-flow dues.

**Unit tests**
- `buffer_stock_rule_properties`: consumption rises with cash on hand, with a falling propensity; the buffer rises with
  the outlook's width and with risk aversion.
- `buffer_solution_matches_published_values`: m* and κ against Carroll's (1997) published cases.
- `needs_first_then_ces`.
- `reservation_components`.
- `baumol_tobin_holding`.
- `bill_limit_at_reservation_yield`: the order's limit is the deposit rate plus the price-risk premium.

**Live checks**
- `LC-1-32`: HH.15 — every household's spending reaches named sellers (through match-set records), and every unit of
  income came from a named payer.
- `LC-1-33`: households going without their needs are recorded as events and counted.
- `LC-1-34`: liveness (N2) for the circular flow — wages paid, spending received, production, employment and lending
  are non-zero and respond to a primitive moved on a copy (N6).

**Budget**: architecture §13.2's visits, occasion evaluations and standing flows; the memoised buffer solutions (about
10⁴ entries). Counters, ratcheted: `phx_pop.bytes_per_household_cell` (at 504), `phx_hh.going_without`,
`phx_hh.bill_orders`, `phx_pop.keys_per_stance`, `phx_pop.parts_by_cause`.

**Guards**: none new.

**Not allowed**:
- a representative household;
- a consumption function applied to an aggregate;
- a propensity to consume declared as a number;
- a household holding what nobody else took;
- a decision evaluated at a group average across a kink.

**Done when**
- [ ] Households spend, work and save from their own states; the circular flow closes.
- [ ] LC-1-32 to LC-1-34 pass, and every live check of S1.01–S1.11 now applies and passes (spec Part O's rule for
  systems a later system of the stage completes).
- [ ] Two reviews are done.

---

### S1.13 — `sys-dem`: births, and leaving school

**Status**: planned

**Clauses**:
- DECISION: POP.10.
- PROCESS: POP.5; CHN.3 *(part: conception)*.
- Education and skill (POP.6) are S6.02; until then leaving school is a placeholder naming POP (below).

**Architecture**: §7.3, §7.5.

**Depends on**: S1.12.

**Goal**: households decide whether to have a child; conception is a hazard given that decision; a birth adds a child
role to the household; children leave school and enter the labour market; the population grows or shrinks as an
outcome.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-pop/src/fertility.rs` | the decision point `try_for_child` |
| `crates/systems/sys-dem/src/rules/fertility.rs` | POP.10 |
| `src/hazards.rs` | the conception hazard, acting only on the roles whose decision is to try |
| `src/school.rs` | the placeholder school-leaving rule |
| `data/<country>/DEM.toml` | conception hazard by age (TECHNOLOGY); preference for children (PREFERENCE); the statutory school-leaving age (POLICY, education law) |

**Design**
- **Fertility** (POP.10), a lumpy decision of a couple's household on its occasions: it tries for a child when its
  value of a child exceeds the child's cost. The value reads, as POP.10 lists: its income and its outlook (mean and
  width), its dwelling (rooms per member, tenure), its members' ages (the partner's age, the youngest child's) and its
  preference for children, with a taste per occasion (stream `DEM.fertility_taste`). The form — a discrete choice over
  the child's net value (Becker, 1960; Hotz, Klerman and Willis, 1997) — is listed in `SHAPES.toml`.
- **Conception** is a hazard by age on the members who decided to try (CHN, stream `DEM.conception`), screened at an
  envelope like any hazard.
- **A birth** is an event: the household's composition changes (a key change, so a part).
- **Leaving school**, until S6.02 builds education: a child role reaching the statutory school-leaving age (POLICY)
  becomes another adult role of its household, in the labour-market state "out of the labour force" until its first
  search decision (S1.08). The member reaching the age is drawn from the birth years (REP.25). A placeholder naming
  POP (S6.02), so cohorts enter the labour force and the decades run of the exit is possible.

**Unit tests**
- `fertility_value_inputs`: over given inputs, the value rises with income and space and falls with the youngest
  child's closeness.
- `conception_acts_only_on_deciders`: over given counts, the members exposed are those who decided.

**Live checks**
- `LC-1-35`: POP.11 — the population equals births and arrivals minus deaths and departures.
- `LC-1-36`: POP.13 — age structure and fertility are reported.
- `LC-1-49`: every cohort reaching the school-leaving age enters the adult roles on its days, and the labour force's
  inflow is published.

**Budget**: births about 10⁴ a day as parts; school leavers about 5 × 10³ a day as parts. Counters, ratcheted:
`phx_dem.births`, `phx_dem.school_leavers`.

**Guards**: none.

**Not allowed**: a birth rate; a conception without a decision; a participation rate for school leavers.

**Done when**
- [ ] Births follow decisions and hazards; school leavers enter the adult roles.
- [ ] LC-1-35, LC-1-36 and LC-1-49 pass.
- [ ] Two reviews are done.

---

### S1.14 — `sys-idx` and `sys-sta`: price indices and published statistics

**Status**: planned

**Clauses**:
- STATE: IDX.3; MON.10; STA.2, STA.3, STA.4, STA.5; STA.1 *(part: national accounts, prices, labour and money; house
  prices come at S2.05, the balance of payments at S5.05)*; IDX.1 *(part)*, IDX.4 *(part)*, IDX.5 *(part)*, IDX.6
  *(part)* and IDX.7 *(part)*: the price indices; market indices are S3.09.

**Architecture**: §4.9 (records), §8.

**Depends on**: S1.13.

**Goal**: each country's statistics agency publishes the national accounts, the consumer and producer price indices,
labour and money statistics on a calendar, from samples and records, late and revised. Decisions that read aggregates
read these.

**Files**

| File | Purpose |
| --- | --- |
| `crates/systems/sys-sta/src/*.rs` | the agency, its samples through the stream `STA.sample`, publication calendars, revisions |
| `crates/systems/sys-idx/src/*.rs` | the CPI and PPI rules, chained |
| `data/<country>/STA.toml` | survey designs, sample sizes, calendars, revision policies (POLICY) |

**Design**:
- Statistics are computed from **samples** of the period's records: households' purchases (match-set records) for the
  CPI, factory-gate sales for the PPI, payrolls and surveys for labour, banks' reports for money.
- They are published on their days as public records with revisions (STA.2, STA.4), spread over the calendar (N8.9).
- The CPI weights households' spending and includes rents and consumption tax (IDX.3). Indices are chained (IDX.4).
- A published index is kept with each revision (IDX.1).
- Each published series is a public series for the outlooks of S1.01, and its surprises are recorded per method.

**Unit tests**: `chained_index_no_jump`; `revision_from_later_sample`: over given samples, the revision is the later
estimate and both are kept with their dates.

**Live checks**: `LC-1-37`: STA.3 — output by expenditure, income and production agree up to the published
discrepancy; `LC-1-38`: STA.4 — no party read a statistic before its publication day. (IDX.5's check of market
indices moves to S3.09, where they exist.)

**Budget**: statistics are computed on their days from samples; within the "statistics" line. Counters, ratcheted:
`phx_sta.records_sampled`, `phx_sta.publications`.

**Guards**: none.

**Not allowed**: a statistic available before publication; an index input to its own constituents.

**Done when**
- [ ] Every country publishes on its calendar.
- [ ] LC-1-37 and LC-1-38 pass.
- [ ] Two reviews are done.

---

### S1.15 — GEN III: the Stage 1 opening

**Status**: planned

**Clauses**: GEN.2 *(part)*, GEN.3 *(part)*, GEN.4 *(part)* and GEN.5 *(part)*: every Stage 1 system's opening
contribution.

**Architecture**: §10.

**Depends on**: S1.14.

**Goal**: the opening world of Stage 1:
- firms' ways, plant, stocks, markups, posted prices and wage offers;
- banks' loan books, their opening histories of defaults and recoveries by class, and deposit rates;
- the treasury's bills and the public agencies' staff (the agencies themselves are S0.16's);
- union parties where coverage exists (ENDOWMENT), with their agreements;
- households' preferences, stances and outlooks from the opening history;
- vacancies;
- dwellings held without a housing market, a placeholder naming HSG (S2.05).

It is drawn, apportioned and balanced by GEN's procedure, and settled.

**Files**

| File | Purpose |
| --- | --- |
| each Stage 1 system's `gen.rs` | its opening contribution: phase, drawn and derived sides, sources |
| `data/<country>/gen/*.toml` | the distributions, with sources |
| `data/world.toml` | `opening_history_years` (ENDOWMENT: how much history the opening carries, from the sources available) |

**Design**:
- Each contribution declares its phase, its drawn and derived sides, and its distribution sources.
- The opening history (`opening_history_years` of prices, statistics and banks' default records) is written into public
  and private records as their audiences say (GEN.2, GEN.5).
- Posted prices and wage offers are drawn at points (REP.34); markups from the opening margins (S1.03).
- Stances are not drawn: each member's stance is its switching choice (S1.01) over the heuristics' performance on the
  opening history, drawn with the stream `HH.stance_taste`.

**Unit tests**: none beyond the contributions' own.

**Live checks**: `LC-1-40`: day one passes every family (GEN.7); `LC-1-41`: the GEN report lists every balancing change
and apportionment difference.

**Budget**: world creation — drawing, balancing and settling — is measured on the phone and reported with its parts;
no budget is set for it yet, and a total beyond ten minutes is raised with the owner (as S0.26). Counters:
`phx_gen.balancing_changes`.

**Guards**: none.

**Not allowed**: an opening parameter changed after a run; a posted price that is not a point; a stance drawn from a
share.

**Done when**
- [ ] The Stage 1 world opens, balances and settles.
- [ ] LC-1-40 and LC-1-41 pass.
- [ ] Two reviews are done.

---

### S1.16 — The Stage 1 gate: the circular flow, the reference run and the go/no-go

**Status**: planned

**Clauses**: PTY.12 *(part: the ladder and the reference comparison at Stage 1)*; REP.18 *(part: every RESOLUTION
setting, taste distribution and review cost of Stage 1's decisions declared and measured; later stages add theirs)*;
N8 *(the budget at Stage 1)*; N2; the Stage 1 exit.

**Architecture**: §7.11, §13, §14.5, §14.6, §14.7.

**Depends on**: S1.15.

**Goal**: judge the first go/no-go on measured numbers, against criteria fixed before the run.

**Files**

| File | Purpose |
| --- | --- |
| `perf/device/S1.16-*.json`, `perf/measure/S1.16-*.json` | the device report and the measurements |
| `data/shared/READS.toml` | the declared reads compared with the reference, **frozen** before the first Stage 1 comparison: declared at S0.26 for Stage 0's reads and extended at the start of S1.01 |
| `perf/compare/S1.16-*.json`, `perf/ladder/S1.16-*.json` | the comparison and the ladder |

**Design**:
- **The reads** (`READS.toml`, N8.5): distributional reads — employment and unemployment by region and age class, the
  consumption and income distributions, firm sizes, prices by category, money and credit, default counts — and
  **per-person** reads from tracers in play and from every person in the reference: employment-spell lengths, income
  transitions between deciles over a year, and consumption responses to income changes.
- **The reference** is re-sized with Stage 1's state before the run (architecture §7.11: about 185 GB) and run for
  **five seeds**, so Appendix E 30's band is judged beyond the reference's own spread across seeds. The play resolution
  runs twenty seeds.
- **The ladder** (`phx ladder`, PTY.12): at least three rungs of the cell budget and of the tolerances between the play
  resolution and the reference, each compared on the reads.
- **The device run**: the settled Stage 1 world on the phone, a 30-minute soak, then a simulated year.
- **The decades run**: thirty simulated years on the weekly job (architecture §14.7), with the liveness reads.
- **Pass criteria**, fixed here before the run (architecture §14.5):
  - the median turn ≤ 1 000 ms and the worst ≤ 2 000 ms over the settled year;
  - peak `VmHWM` and PSS ≤ 4.5 GB; a full save ≤ 5 s and an increment ≤ 1 s; the latest complete save and the one
    being written together ≤ 4 GB on the device's storage (N8.4);
  - every read within Appendix E 30's band beyond the reference's seed spread, at the play resolution and at each
    rung of the ladder above it;
  - the exit's reads: households earn wages and spend them; firms are founded and end in every industry that has
    firms; banks lend and loans are repaid; the treasury taxes and pays; all through thirty years;
  - §13's 10% headroom is reported; a pass without it is recorded as a finding.
- **A miss** is a finding. N8.7's remedies apply in order: how the world is represented and traversed, then the play
  resolution. If none suffices, the owner decides the budget or the accuracy (N8.7); the stage ends only when the
  budget in force is met (N8.8), and the plan does not continue to Stage 2 until then.
- Architecture §13 is rewritten with the measured numbers.

**Unit tests**: none.

**Live checks**: `LC-1-42`: the decades run keeps the circular flow alive (N2): money stuck on ended parties is zero
at every close; no stock or rate grows for thirty years with no named cause (each growing quantity is traced to the
flows that feed it, read from the ledger); and no state repeats unchanged for a year. `LC-1-50`: the exit's reads above
hold on the gate run.

**Budget**: this is the budget's gate.

**Guards**: none.

**Not allowed**: a gate judged off the device; a comparison with a different opening; reads chosen after seeing the
comparison; a tuned primitive.

**Done when**
- [ ] The device report, the comparison over the declared seeds and the ladder are committed.
- [ ] Every pass criterion above holds, or the owner's decision under N8.7 is recorded in §12 and the budget then in
  force is met.
- [ ] LC-1-42 and LC-1-50 pass.
- [ ] Architecture §13 is updated with measured numbers.
- [ ] Two reviews are done.

---

## 5. Stage 2 — Credit and failure

**Exit** (spec Part O):
- A borrower's own cash failure produces a default, an estate, a loss on named holders and a housing foreclosure.
- A bank can fail for liquidity or solvency, and is resolved.
- The budget and the reference comparison hold at the Stage 2 gate.

**Decision rules** follow §2.21.

**Placeholders retired in this stage**, each by the step named:

| Placeholder | Introduced | Retired by |
| --- | --- | --- |
| Every insolvency liquidates (naming FRM) | S1.03 | S2.03 |
| Household estates wait for buyers of dwellings and securities (naming their buyers' systems) | S0.25 | S2.04 (dwellings from S2.05) |
| No new household loans (naming HH) | S1.12 | S2.05 |
| The opening dwellings held without a housing market (naming HSG) | S1.15 | S2.05 |
| The bank's marginal cost of funds, its deposit-rate rule, its use of the central bank's facilities, its reserve target (naming BFL) | S1.09 | S2.06 |
| The capital a loan consumes, a declared risk weight per loan kind (naming BCP) | S1.09 | S2.07 |
| The energy carriers as plain goods (naming ENE) | S1.02 | S2.09 |
| A lender's assessment reads only its own lines' records (naming BNK's bureau) | S1.09 | S2.10 |

**Placeholders this stage introduces**, each naming its retirer:

| Placeholder | Introduced | Retired by |
| --- | --- | --- |
| A borrower's answer to a restructuring offer (naming FRM for firms, HH for households) | S2.01 | S2.03, S2.11 |
| Dwellings in enforcement wait for a housing market (naming HSG) | S2.01 | S2.05 |
| A loan seller's shadow cost of liquidity and capital, missing, so its reservation is its claim value (naming BFL and BCP) | S2.01 | S2.06, S2.07 |
| Estates' securities with no market wait (naming EQY) | S2.04 | S3.05 |
| Public owners' maintenance of infrastructure (naming SOC) | S2.05 | S5.02 |
| A bank's decision to raise capital recorded and unmet (naming EQY and CRD) | S2.07 | S3.04, S3.05 |
| A bank's group is the bank alone, with no group fact (naming EQY) | S2.07 | S3.05 |

**Brought forward**: MMK.1's unsecured interbank **term** loans between named banks, bilateral, come forward to S2.06
for BFL.6's interbank borrowing (spec Part O: the move is recorded here). Overnight loans and the market stay at
S3.01, which extends S2.06's interbank loan line kind and declares no second one.

**Representation.** Five choices keep the stage's counts down, each in its step and in architecture §18:
- invoices accrue on the trade's statement period, one row per (holder, market, terms, period) (S2.02);
- invoice runs are ordered by due day behind a head index, so the stream reads only rows due today (S2.02);
- a housing transaction's pins ride on one part, and a bank switch is made when its transfer settles (S2.05, S2.06);
- a resolution takes the book from the day's statement and re-keys once per distinct key (S2.08);
- one estate per (part, occasion), and a personal insolvency's estate ends once its assets are distributed (S2.04,
  S2.11).

**The stage's budget ledger.** Each step names the lines of architecture §13.2 and §13.1 it adds to; §13 carries them
as its Stage 2 lines. Wall time is core time ÷ 3 (architecture §13.2). An ordinary business day gains about **86 ms**:
- parts, about 58 k a day — housing sales, lettings and moves 30 k, bank switches 9 k, credit (arrears and
  restructuring splits, record-stage re-keys, insolvency entries and discharges, heirs) 19 k: 58 k × 2.5 µs ÷ 3 ≈
  48 ms at the target unit cost, and 58 k × 12.1 µs ÷ 3 ≈ 234 ms at F-001's measured one, the stage's largest risk;
- housing search: 50 k groups × 20 listings × 50 ns ÷ 3 ≈ 17 ms;
- occasion evaluations: about 0.4 M × 80 ns ÷ 3 ≈ 11 ms;
- institutions — funding, capital, supervision, the electricity auction and offers, provisions over per-(line,
  arrears stage) totals — about 10 ms;
- invoices: one head read per holder, (0.25 M + 30 k) × 2 ns ÷ 3 ≈ 0.2 ms; on a statement's due day about 1.5 M
  rows read and 0.3 M payments, (1.5 M × 10 + 0.3 M × 30) ns ÷ 3 ≈ 8 ms, which falls on the heavy day.

A heavy day gains about 126 ms and a non-business day about 13 ms. Two event lines join §13.2: a resolution's D+1,
about 25 ms, and a bank-run day, about 100 ms (about 260 ms at the measured part cost).

Memory at the worst day's peak gains about **251 MB**:
- invoice rows, 0.25 M firm cells × 10 + 0.5 M of individuals = 3 M × 24 B = 72 MB; their holder-list entries,
  3 M × 6 B = 18 MB; arena slack, 15% of both = 14 MB: 104 MB in all;
- filed accounts, 48 MB;
- household cells, 504 → 568 bytes: 64 B × 0.7 M = 45 MB;
- estates open, by Little's law about 60 k at 512 B: 31 MB;
- listings, 13 MB;
- loan lines' balances per arrears stage, 0.3 M lines × 4 stages × 8 B = 10 MB.

Through Stage 2 the design point projects a median turn of 996 + 86 = **1 082 ms**, 8% over the 1 000 ms budget, and a
peak of 4 045 + 251 = **4 296 MB**, 4.5% under 4.5 GB. The required 10% headroom — a median of at most 900 ms and a
peak of at most 4 050 MB — is **missed on both** (F-005). S1.16 rewrites §13 with measured numbers first; S2.12 judges
the stage on the device, and a miss takes N8.7's remedies in order. Every increment below is a counter, ratcheted from
the step that adds it; `phx_pop.distinct_keys`, per key attribute, counts the keys the stage's attributes make — the
grant per buyer class (S2.02), the banking arrangement (S2.06), the credit-record stage (S2.10), the procedure clock
(S2.11) — since each is a floor under the cells carried.

---

### S2.01 — `sys-bnk`: losses, provisions, workouts, enforcement, write-offs and loan sales

**Status**: planned

**Clauses**:
- DECISION: BNK.7.
- PROCESS: BNK.9; BNK.10 *(part: sales between banks; non-bank lenders buy from S3.08, and securitisation vehicles
  from S4.05, where it completes)*; ACC.7 *(completes it: provisions)*; MKT.20 *(part: the loan-book valuer)*; HH.13
  *(part: collection and enforcement; the rest completes at S2.11)*.
- INVARIANT: BNK.12.
- FORBID: BNK.15 *(completes it: the one assessment prices and provisions)*.
- L1 *(completes it: the chain's machinery — a missed payment, arrears, the contract's default, the lender's
  workout or enforcement, the sale, the write-off and the loss on named holders as dated events; later lenders and
  holders — funds, securitisation vehicles, insurers — join it through the same events when their systems add their
  triggers)*.

**Architecture**: §3.1 (decision-point homes), §3.4 (writer tokens), §4.4 (line transfers), §4.5 (payment
records), §6.1 (2d, 2e, 5c, 7, 9a, 9c, 9e), §9.1.

**Depends on**: S1.16 and the owner's go.

**Goal**: a loan is carried at amortised cost with an expected-loss provision from the same assessment that priced
it. A missed instalment becomes arrears on its date, arrears reach the contract's default definition, and the lender
decides — wait, restructure, extend, waive for a fee or enforce — from what each is worth to it. Collateral is sold
for what it fetches, a write-off books what was not provided, and the loss lands on named holders as dated events.
Loans can be sold to other banks at a negotiated price.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/workout.rs` | message kinds `ArrearsNotice` (lender to borrower, a notice occasion), `RestructuringOffer` (answered by the borrower), `EnforcementDemand` (a demand due the next business day at 2c, TIME.7); the workout option set as declared data per loan kind |
| `crates/interfaces/if-credit/src/sale.rs` | `LoanSaleAsk` and `LoanSaleQuote` (bilateral, MKT.7, across days); the loan-sale transfer reason and its requesters |
| `crates/interfaces/if-credit/src/assessment.rs` | `LoanAssessment` and its writer token, which only `sys-bnk` can build (PC-40); the rule handle `loan_claim_value` (implemented by `sys-bnk`, called by its rules and by the loan-book valuer's method): the claim value (VAL.8) of a line's or rows' expected payments under a given assessment |
| `crates/interfaces/if-credit/src/decisions.rs` | adds the bank's decision points `workout`, `sell_loans`, `quote_loans`; they live here because their types are loan terms and assessments (architecture §3.1) |
| `crates/systems/sys-bnk/src/rules/provision.rs` | BNK.9, BNK.15: expected loss from the pricing assessment |
| `src/rules/workout.rs` | BNK.7: the lender's choice among the options |
| `src/rules/loan_sale.rs` | BNK.10: the seller's reservation and the buyer's quote |
| `src/handlers/2d_arrears.rs` | reads fail records and arrears on BNK's line kinds; writes arrears notices and books workout wakes |
| `src/handlers/5c_workout.rs` | the workout decision for each woken (line, holder row) |
| `src/handlers/5c_loan_sales.rs` | asks, quotes and acceptances of loan sales |
| `src/handlers/2e_writeoff.rs` | write-offs and recognised enforcement losses |
| `src/handlers/9a_provision.rs` | provision moves on the bank's review days and after arrears changes, over per-(line, arrears stage) totals |
| `src/handlers/9c_covenants.rs` | covenant tests on the statements received, and the workout wake a breach books, in one handler |
| `src/audit.rs` | the BNK.12 family |
| `data/<country>/BNK.toml` | adds: workout and enforcement costs in staff hours and fees to named parties (TECHNOLOGY, legal POLICY for fees); the provisioning standard's horizons by arrears stage (POLICY: accounting standards, ACC.17); the provision review schedule (PREFERENCE of management); review costs per workout, loan-sale and quote decision in hours (TECHNOLOGY) |
| `data/shared/SHAPES.toml` | the workout form and the loan-sale form, with sources |

**Design**

- **The assessment is one type** (BNK.15): `LoanAssessment { class: ClassId, pd: Rate, lgd: Rate }`, declared in
  `if-credit` with a writer token only `sys-bnk` can build (architecture §3.4), so only S1.09's `assess` rule makes
  one, from the bank's own outlooks of class default frequencies and realised recoveries. Everything a lender prices
  or provides for on its book takes `&LoanAssessment`: the quote, the provision, the workout's values, the loan-sale
  and factoring quote (`quote_loans`), the interbank quote (S2.06), the bid for a failed bank (S2.08) and a bank's
  vote on a plan (S2.03) — all rules of `sys-bnk` — and the loan-book valuer's method below, which calls
  `loan_claim_value`. Other systems read an assessment by handle and never build one (PC-40). A seller's loss outlook
  per buyer class (S2.02) and a depositor's safety outlook (S2.06) are those parties' own outlooks, not a lender's
  book, and PC-40 does not reach them.
- **The class is a term of the loan** (REP.23): the bank's grade at writing is stored in the loan line's terms, so a
  row's class is read from its line, never recomputed from the borrower's private state. The row's `record`
  (architecture §4.5) carries days in arrears and missed payments.
- **Arrears and default are contract events** (L1, HH.13):
  - an instalment that fails at 7 is a fail record; at 2d the ledger's contract process turns it into arrears on
    the row (S0.15);
  - when a row's days in arrears reach its terms' **default definition** (the contract's declared number of days),
    the contract process records a `DefaultEvent` naming the line, the holder and the count. No stream can produce
    one (PC-41); a default is never drawn (Law 15);
  - for a cell, the row's members share its record; members of a line whose records diverge are drawn out (REP.23)
    and split, as S0.23 does.
- **2d `arrears`** (index-driven over the day's fail records of BNK's line kinds): for each (line, holder row) whose
  arrears stage changed it writes an `ArrearsNotice` to the borrower (a notice occasion for the row's members, read
  by S2.03's distress and S2.11's arrears actions) and books the bank's `workout` wake for that row.
- **Covenants** (BNK.7): a loan's covenants are declared terms of its line kind (the most leverage, the least interest
  cover, a reporting duty). A borrower that is a firm sends its lender its statement (ACC.9) on the covenant's
  reporting dates, a message the lender reads privately; at 9c one handler, `9c_covenants`, tests it against the
  covenants and, on a breach or a missed report, books the bank's `workout` wake for the row, applied at 9e.
  Households' loans carry no financial covenants.
- **Workout** (BNK.7), a lumpy decision of the bank at 5c on each woken row, and on its review of rows still in
  arrears. Inputs, as BNK.7 lists: the row's arrears or covenant breach, and what each option is worth to the bank:
  - **wait**: the claim value (VAL.8 `claim_value`) of the contractual payments at the row's class default
    probability conditioned on its arrears stage (the bank's own class-by-stage outlook, S1.09's adaptive form);
  - **restructure** or **extend**: the claim value under new terms (a longer term or a lower rate on the lender's
    points), at the default probability of the class those terms put the borrower in;
  - **waive for a fee**: the claim value with the fee added and the breach cured;
  - **enforce**: the collateral's valuation (MKT.20, the bank's named valuer) less enforcement costs and the time to
    sell, discounted at the bank's required return; for unsecured debt, its outlook of collection recoveries;
  - each option's value is net of its cost in staff hours at the bank's wage bill and fees paid to named parties.

  The bank takes the option of highest value; ties go by lot (stream `BNK.workout_lot`). The form — a lender's
  comparison of expected recoveries under renegotiation and foreclosure (Adelino, Gerardi and Willen, 2013; Ghent,
  2011) — is listed in `SHAPES.toml`. Its parameters are the costs (TECHNOLOGY and legal POLICY) and the bank's
  required return (PREFERENCE, BNK.16); no recovery or loss rate is a parameter.
- **Cells** (BNK.20): one decision per woken row applies to every member on the row, since they share its record.
  A restructuring is a `RestructuringOffer` to the row's members; those who accept move to a new line with the new
  terms (a line transfer of their count, S0.17) and split out (REP.8). Until the borrower's own rules exist, the
  offer is answered by accepting when the new instalment is below the old: a placeholder naming FRM (S2.03) for firms
  and HH (S2.11) for households.
- **The loan-book valuer** (MKT.20, part): the bank's named valuer values each line's per-stage totals by
  `loan_claim_value` at the bank's class default probabilities and loss outlooks, discounted at the rate of the bank's
  recent quotes for that class (the prints of its own lending), a declared method, at 9a with the provisions; the
  bank's statement carries it, and S2.08's resolution reads that statement.
- **Enforcement**: an `EnforcementDemand` accelerates the balance, due at the next business day's 2c (TIME.7).
  Unpaid, it is the borrower's default of payment:
  - a firm borrower goes to its insolvency procedure (S1.03's liquidation until S2.03), where the bank is a secured
    creditor of its collateral's proceeds through the waterfall (S0.17);
  - a dwelling is foreclosed by S2.05 (HSG.11). Until S2.05 the loan stays in enforcement, counted, on a placeholder
    naming HSG;
  - an unsecured household debt stays in collection, the bank's outlook of recoveries updated by what it collects,
    until S2.11 adds the procedure.
- **Recognised losses** (2e `writeoff`): when the collateral's sale settles, or collection ends by the bank's own
  choice (the workout rule finds that continuing collection is worth less than its cost), the bank writes the loan
  off. The write-off is an instruction with a `Row` leg retiring the row and the accounting effect of the loss not
  provided; it applies at 2f. The loss that reaches capital is principal − recovery − provisions (BNK.12). Each
  write-off is an event naming the lender, the borrower and the sizes.
- **Provisions** (BNK.9, ACC.7): at 9a on the bank's review days and for lines with a row whose arrears stage
  changed today:
  - each loan line keeps its balance total **per arrears stage**, maintained incrementally where balances move — by
    the apply at 7c and 2c, and by 2d when a row changes stage — and checked against its rows by the Contracts
    family, so no pass streams the holder lists;
  - expected loss per line = Σ over its stages of the stage's balance total × `pd(class, stage, horizon)` × `lgd`,
    with the horizon by stage from the accounting standard (POLICY): twelve months while performing, lifetime once in
    arrears; a bank's lines are spread over its review days by its phase (TIME.schedule_phase);
  - the provision is a valuation of the bank's own book (9a, architecture §6.1), so 9b's statements and ratios read
    it the same day; each move is an income event (ACC.7) recorded at 9e's apply.
- **Learning** (S1.09's assessment, now fed): each default updates the bank's class-by-stage outlook, and each
  realised recovery its loss-given-default outlook per collateral class, so what the bank books teaches it.
- **Loan sales** (BNK.10), bilateral (MKT.7):
  - on its review, a bank that wants to shed (S2.07's capital, S2.06's liquidity) sends `LoanSaleAsk` for whole lines
    or its side of a line to the banks it can reach, with the line's terms and payment records;
  - each asked bank answers at the next 5c with a price from its own claim value of the line under **its own**
    assessment of the line's grade and records (BNK.15), or declines;
  - the seller's reservation is its own claim value less its **shadow cost** of the liquidity or capital the line
    ties up (S2.06's and S2.07's facts: what one more unit of cash or capital is worth to it today), so a bank short
    of either sells at a discount, and one that is not does not; until S2.06 and S2.07 the shadow cost is missing and
    the reservation is the claim value, a placeholder naming BFL and BCP;
  - it takes the best quote at or above its reservation at the 5c after; the acceptance handler writes one
    instruction — the line transfer of the lender side and the price — settled at 7.
  Ties go by lot (stream `BNK.loan_sale_lot`). The form is a first-price request for quotes (MKT.7), listed in
  `SHAPES.toml`. Buyers are banks until the non-bank lenders of S3.08 (finance companies and private-credit funds)
  and the vehicles of S4.05 exist, each answering through `quote_loans`.
- **The quote on a line side** (`quote_loans`): a bank answers a `LoanSaleAsk` or a seller's `FactoringAsk` (S2.02)
  by the same rule: its `loan_claim_value` of the rows offered under its own assessment of their class and records,
  less its required return on the capital they would consume (S2.07), or it declines.
- **Review costs**: each workout, loan-sale and quote decision costs the bank's staff hours (TECHNOLOGY), counted per
  decision kind, paid in its staff capacity (§2.21).
- **Streams**: `BNK.workout_lot`, `BNK.loan_sale_lot`.

**Unit tests**
- `provision_uses_pricing_assessment`: over a given assessment and stage totals, the expected loss equals Σ total ×
  pd × lgd at each stage's horizon.
- `stage_totals_follow_moves`: over given payments and stage changes, each stage's total equals its rows' balances.
- `workout_takes_best_option`: over given option values and costs, including a tie resolved by a given lot.
- `enforce_value_net_of_costs_and_time`.
- `writeoff_loss_identity`: principal − recovery − provisions, with provisions above and below the loss.
- `loan_sale_reservation_net_of_shadow_cost`: over a given claim value and shadow cost, the reservation falls as the
  shadow cost rises, and a quote below it is refused.
- `covenant_test_on_statement`: over a given statement and covenant terms, breaches are found.
- `default_event_not_constructible_outside_ledger` (compile-fail, PC-41).

**Live checks**
- `LC-2-01`: BNK.12 — the family is clean: the loss reaching each bank's equity on every write-off equals principal
  − recovery − provisions.
- `LC-2-02`: L1 — every default event traces to a missed instalment on a dated fail record and the row's arrears
  reaching its default definition; none has another source.
- `LC-2-03`: BNK.14 in practice — per collateral class with at least two enforcements in the year, the recovery
  rates' interquartile range across enforcements is above zero, and the class's quarterly mean recovery is not the
  same in every quarter; a class failing either fails the check, with the class and its recoveries as the facts.
- `LC-2-24`: BNK.15 — every provision move names the row class and assessment the bank's quotes used on that day
  (read from the bank's records), and moves only on review days or arrears changes.
- `LC-2-25`: ACC.7 — every provision move and write-down is an income event of its day; reversals never exceed
  the original cost.

**Budget**:
- Workout decisions are "Occasion evaluations"; arrears notices to cells are notice occasions in the same line.
- Provisions are in "Valuation, accounts, tests, publications": about 0.3 M loan lines × 4 stage totals a month,
  spread over review days, at ≤ 20 core-ns each — under 0.1 ms a day. The stage totals are 0.3 M × 4 × 8 B = 10 MB in
  §13.1's lines.
- Restructured members are parts ("Parts: from decisions").
- Counters, ratcheted: `phx_bnk.arrears_notices`, `phx_bnk.defaults`, `phx_bnk.workouts` by option,
  `phx_bnk.writeoffs`, `phx_bnk.provision_totals_read`, `phx_bnk.loan_sales`, `phx_bnk.quotes` by ask kind,
  `phx_bnk.enforcements_waiting`.

**Guards**:
- PC-40: `LoanAssessment` is declared in `if-credit` with a writer token only `sys-bnk` can build, so only its
  assessment rule constructs one; on a lender's book — quotes to households, firms and banks, provisions, workouts,
  quotes on line sides, bids for failed banks, a lender's plan vote — every default probability and loss given
  default comes from a `LoanAssessment`, through `sys-bnk`'s rules or its `loan_claim_value` rule handle, and no other
  type on those paths carries either (a compile-level refusal and a signature check of those rules). Other parties'
  own outlooks — a seller's loss outlook per buyer class, a depositor's safety outlook — are outside it.
- PC-41: `DefaultEvent` is constructible only by `phx-ledger`'s contract process, from a row's payment record and its
  terms' default definition (compile level).

**Not allowed**:
- a loss rate on a book;
- a default drawn;
- a fixed recovery;
- a provision from a model other than the pricing assessment;
- a write-off that books a loss already provided;
- a loan sold without a buyer who valued it.

**Done when**
- [ ] Defaults are contract events, worked out or enforced by the lender's own comparison, and losses land as dated
  events on named holders.
- [ ] LC-2-01 to LC-2-03, LC-2-24 and LC-2-25 pass.
- [ ] PC-40 and PC-41 are registered.
- [ ] Two reviews are done.

---

### S2.02 — `sys-tcr`: trade credit

**Status**: planned

**Clauses**:
- STATE: TCR.1.
- DECISION: TCR.2, TCR.3.
- PROCESS: TCR.4.
- INVARIANT: TCR.5.
- MEASURE: TCR.6.
- FORBID: TCR.7.
- PRIMITIVE: TCR.8.
- FRM.18's family, completed at S1.03, now reads real invoices.

**Architecture**: §3.1 (decision-point homes), §4.4 (commitments drawn by matches, line transfers), §4.5 (accruing
rows), §6.1 (2d, 5b, 5c, 6d, 7), §6.5, §13.1.

**Depends on**: S2.01.

**Goal**: sellers between firms grant terms to buyers they have seen pay; sales on terms accrue on the invoice line of
(market, terms, statement period), the seller's receivable and the buyer's payable in one record; buyers pay on the
due date or take the discount; late payment stresses the seller's cash; a buyer's default lands on sellers drawn from
its lines; receivables can be sold to a bank.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/invoices.rs` | the invoice line kind: terms `{ market, period_end, term_days, discount_ppm, discount_days }` on the trade's customary grid and statement period (TCR.8), an accruing kind (`balance` on each row); the `TermsGrant` commitment kind, drawn by matches at 6d (S0.18); `CollectionNotice`; `FactoringAsk`, answered by the bank's `quote_loans` (S2.01) |
| `crates/interfaces/if-credit/src/decisions.rs` | adds `grant_terms` (the seller's) and `pay_or_discount` (the buyer's), which live here because their types are invoice terms (architecture §3.1) |
| `crates/systems/sys-tcr/src/rules/terms.rs` | TCR.2 |
| `src/rules/pay_or_discount.rs` | TCR.3: the buyer's comparison |
| `src/rules/factor.rs` | TCR.3: the seller's factoring ask and its acceptance; the bank's quote is `sys-bnk`'s `quote_loans` (PC-40) |
| `src/handlers/5c_terms.rs` | the seller's terms review; writes `TermsGrant` commitments |
| `src/handlers/5b_pay.rs` | the buyer's continuous payables decision on its schedule: early payments as instructions |
| `src/handlers/2d_overdue.rs` | invoice fails turned into collection notices and wakes |
| `src/handlers/5c_factoring.rs` | asks, quotes and acceptances |
| `src/audit.rs` | the TCR.5 family |
| `data/<country>/TCR.toml` | customary terms per trade: term lengths and discount on the trade's grid (POLICY of the trade); the terms review schedule (PREFERENCE of management); the review cost in hours (TECHNOLOGY) |
| `data/shared/SHAPES.toml` | the terms form, the pay-or-discount form and the seller's factoring form, with sources |

**Design**

- **Invoices** (TCR.1): sales on terms accrue on the trade's **statement period** (a term of TCR.8: end of month in
  most trades). The invoice line is one per (market, terms, period), and each party holds **one accruing row per
  (market, terms, period)** on it. A match in a posted between-firm market (S1.05) whose buyer holds an undrawn
  `TermsGrant` from the seller is settled at 6d as goods against the invoice rows: the match draws the commitment
  (S0.18, architecture §4.4), whose `Row` leg adds the amount to the buyer's payable row and the seller's receivable
  row in place of the money leg — appending a row only at a party's first purchase or sale of the period. The seller's
  row is the receivable, the buyer's the payable, one line (Law 4). A purchase beyond the grant's undrawn limit pays
  at settlement as before. Retail is never on terms.
- **Bilateral supply** (S1.05): a supply line between firms carries its credit terms as its own payment leg — due
  its term after each delivery — so it needs no invoice rows.
- **Invoice runs** (the settlement stream, architecture §6.5): a holder's invoice rows are kept as their own run in
  its arena, ordered by due day, and its record keeps the run's **head**, the earliest due day. The 7a stream reads a
  holder's head and enters its run only when the head is today, reading rows while their due day is today; so on a
  day with nothing due an invoice holder costs one read. Paid rows retire at 7c and the head advances; a row that
  fails stays in the run, re-dated by its line kind's contract process at 2d to the day its terms next demand it.
- **Terms** (TCR.2), a lumpy decision of the seller at 5c on its terms review days and when an overdue wakes it.
  Inputs: what it has seen of each buyer class — the payment records on its own invoice lines with that class
  (days late, fails), from S2.10 the buyer's filed accounts — its own cash position and outlook, and its cost of
  funds (its bank's quote, S1.09). The grant per buyer class is a credit limit and a term on the trade's grid:
  - the term offered is the longest customary term whose expected cost — the receivable's financing at the seller's
    marginal rate plus the buyer class's expected loss from its own record (an adaptive outlook of losses per class,
    VAL.6) — is below the margin it expects on the sales terms bring;
  - the limit is the expected purchases over the term the seller would carry, within its own cash buffer;
  - it tightens (shorter term, lower limit, or none) as its own cash outlook worsens or the class's losses rise.

  The form — trade credit as a seller's financing and screening choice (Petersen and Rajan, 1997; Wilner, 2000) — is
  listed in `SHAPES.toml`; its parameters are the trade's grid (POLICY) and the seller's management preferences.
  Buyer classes are what the seller observes itself (Law 12): its own invoice records with the buyer — a buyer cell's
  record on the seller's lines — and, from S2.10, the buyer's public filed accounts; a bureau record only if the
  seller buys it as lenders do (S2.10's `BoughtRecord`). The class's loss outlook is the prior, and the buyer's own
  record on the seller's lines moves it (TCR.2 is per buyer); members of one cell share that record, so they are
  granted alike. A seller cell's grant per class is a key attribute, like a posted price
  (REP.34), and a change splits the members who make it.
- **Pay or discount** (TCR.3), a continuous decision of the buyer on its weekly schedule: for each payable line with a
  discount window open, pay early when the discount's implied annual rate `(d ÷ (1 − d)) × (days in year ÷ (term −
  discount days))` exceeds its marginal cost of funds (its deposit rate if it has spare cash beyond its buffer, else
  its credit line's rate or quote). Early payments are instructions settled at 7. The form is the textbook cost of
  forgone discount (Ng, Smith and Smith, 1999), listed in `SHAPES.toml`.
- **Due dates** (TCR.4): an invoice line's due day is its period's end plus its term; the settlement stream pays it at
  7 like any due (architecture §6.5), through the invoice runs above, with pooled flows for cells. A failed payment is
  a fail; at 2d the contract process makes it arrears and `2d_overdue` sends the buyer a `CollectionNotice` (a notice
  occasion, read by S2.03's distress) and wakes the seller's terms review. Late payment is a real state: the seller's
  cash is short by what did not arrive.
- **Default** (TCR.4, TCR.7): when a buyer ends, its payable rows pass to its estate with everything else (L3), and
  the receivables become unsecured claims ranked by the law (S0.17's waterfall). A cell buyer's payable side is a
  count on the line: the sellers who lose are drawn from the line's seller side (REP.23, stream
  `TCR.default_pairing`) when the estate distributes. No receivable survives its debtor: the waterfall's shortfall
  is each drawn seller's recorded loss.
- **Factoring** (TCR.3, MKT.7): a seller asks the banks it can reach at 5c when its cash outlook falls short of its
  buffer; each bank answers at the next 5c by its `quote_loans` decision (S2.01), a price from its `loan_claim_value`
  of the receivable rows under its one `LoanAssessment` of the buyers' class (BNK.15, PC-40); the seller takes the
  best quote above its value of waiting (the receivables' expected collections discounted at its marginal cost of
  funds, less its shadow cost of cash); the acceptance writes a line transfer of the seller side with the price,
  settled at 7. Ties by lot (`TCR.factor_lot`). The seller's form — a request for quotes against its own value of
  waiting (Klapper, 2006) — is listed in `SHAPES.toml`; the bank's is S2.01's.
- **TCR.5 family**: per invoice line, the seller side's balances equal the buyer side's, and the line's rows reconcile
  with the purchases drawn onto it, paid and written off.
- **Review costs**: the terms review costs management hours (TECHNOLOGY), counted.
- **Streams**: `TCR.default_pairing`, `TCR.factor_lot`.

**The firm cell's record** gains, against S1.03's 384 bytes of 500:

| Item | Bytes |
| --- | --- |
| Review exposure and attention rate for the terms review (8 + 4) | 12 |
| Arena reference to the invoice run, one `CellListRef` | 8 |
| The invoice run's head, its earliest due day (`u32`) | 4 |
| **Total after S2.02** | **408** |

**Unit tests**
- `discount_vs_cost_of_funds`: the implied rate of 2/10 net 30 against rates above and below it.
- `terms_tighten_with_record_and_cash`: over given class losses and cash outlooks, the term and limit fall.
- `terms_on_trade_grid`: the grant is always a point of the trade's grid.
- `invoice_row_replaces_money_leg`: the legs of a matched sale drawn against a grant, and past its limit.
- `purchases_accrue_on_period_row`: purchases in one period add to one row per side; the next period appends one.
- `invoice_run_head_skips_until_due`: over a given run, the rows read are exactly those due on the day.
- `default_losses_drawn_from_seller_side`: over given counts, the losses sum to the shortfall.

**Live checks**
- `LC-2-04`: TCR.5 — the family is clean every close.
- `LC-2-05`: TCR.6 — days of payment by trade, their change over the cycle, and chains of distress from a buyer's
  default to its sellers' own arrears are published.
- `LC-2-26`: TCR.7 — every invoice line's due day is after its sale day; no receivable names an ended debtor except
  as a claim on its estate.

**Budget**:
- Memory (§13.1 "Relationship rows", "Line holder lists" and "Arena slack"): about 10 invoice rows per firm cell
  (buyer side: four input markets × two open periods; seller side: about two) and about 0.5 M rows of individuals,
  3 M rows × 24 B = 72 MB; their holder-list entries, 3 M × 6 B = 18 MB; arena slack, 15% of both, 14 MB: 104 MB.
  Invoice lines are a few thousand (markets × terms × open periods), under 1 MB. All enter §13.1 in this step's
  commit.
- Time (§13.2 "Invoices due"): on a day with nothing due, one head read per holder, (0.25 M + 30 k) × 2 ns ÷ 3 ≈
  0.2 ms; on a statement's due day, about 1.5 M rows read and 0.3 M pooled payments, (1.5 M × 10 + 0.3 M × 30) ns ÷
  3 ≈ 8 ms, counted on the heavy day. A purchase on terms adds to two rows at 6d in place of a money leg, at the
  apply's unit cost; rows are appended once per party and period, about 20 times fewer than one per purchase.
- Terms reviews are "Occasion evaluations"; pay-or-discount runs inside the firm's visit.
- Counters, ratcheted: `phx_tcr.invoice_rows`, `phx_tcr.invoice_lines`, `phx_tcr.invoice_row_appends`,
  `phx_tcr.invoice_row_retires`, `phx_tcr.overdue`, `phx_tcr.discounts_taken`, `phx_tcr.factored`,
  `phx_pop.distinct_keys` (the grant per buyer class), `phx_pop.bytes_per_firm_cell` (at 408).

**Guards**: none new.

**Not allowed**:
- a sale that settles instantly by construction;
- an invoice row per purchase, or a stream that reads invoice rows not due;
- a receivable surviving its debtor;
- terms as a share of sales or a fixed days-payable;
- a seller reading a buyer's private accounts.

**Done when**
- [ ] Trade credit binds buyers and sellers; late payment and defaults reach named sellers.
- [ ] LC-2-04, LC-2-05 and LC-2-26 pass.
- [ ] Two reviews are done.

---

### S2.03 — `sys-frm`: funding, payouts, distress, insolvency and restructuring

**Status**: planned

**Clauses**:
- DECISION: FRM.9 *(part: retained cash, trade credit, bank loans and credit lines; bonds, paper and shares are S3.04
  and S3.05)*; FRM.10 *(part: dividends; buybacks are S3.05)*; FRM.12 *(part: every act of distress but seeking a
  buyer, which is S4.06)*.
- PROCESS: FRM.15 *(completes it: the balance-sheet test, the law's procedures, restructuring)*.
- MEASURE: FRM.19.
- This step retires S1.03's placeholder naming FRM (every insolvency liquidates) and S2.01's placeholder answer to a
  restructuring offer for firms.

**Architecture**: §3.1 (decision-point homes), §4.4 (procedure lines), §6.1 (2e, 5b, 5c, 7, 9b, 9c, 9e), §7.4
(kinks), §9.1.

**Depends on**: S2.02.

**Goal**: firms fund themselves from the cheapest source they can reach within their management's leverage
tolerance, pay dividends from what they do not need, act in distress in order of cost, and fail in two ways under
their country's insolvency law — into a restructuring in which creditors vote by class, or a liquidation into an
estate.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-firm/src/insolvency.rs` | the insolvency law as declared data per country: triggers, who may open which procedure, the stay (the procedure lines' terms, S0.17), classes by seniority, voting thresholds, the priority rule, the filing duty on balance-sheet insolvency; message kinds `Plan` (new terms as kernel terms), `Vote`; the creditor's decision point `vote_on_plan`, with a rule per creditor kind — a bank's from `sys-bnk` over its `LoanAssessment` (PC-40), every other creditor's from `sys-frm` |
| `crates/interfaces/if-firm/src/decisions.rs` | adds `payout`, `propose_plan` |
| `crates/interfaces/if-credit/src/decisions.rs` | adds `finance`, `distress` and `answer_restructuring` (the borrower's; the firm's rule here, the household's at S2.11), which live here because they read loan quotes, invoice terms and S2.01's offers (architecture §3.1) |
| `crates/systems/sys-frm/src/rules/finance.rs` | FRM.9 |
| `src/rules/payout.rs` | FRM.10 |
| `src/rules/distress.rs` | FRM.12 |
| `src/rules/plan.rs` | the debtor's plan and a non-bank creditor's vote |
| `src/rules/answer.rs` | the firm's answer to a restructuring offer |
| `crates/systems/sys-bnk/src/rules/vote.rs` | a bank creditor's vote, over its own assessment |
| `src/handlers/5c_finance.rs`, `5b_payout.rs`, `5c_distress.rs` | the decisions |
| `src/handlers/9c_solvency.rs` | the balance-sheet test |
| `src/handlers/2e_insolvency.rs` | opening a procedure, confirming or failing a plan |
| `src/handlers/5c_plan.rs`, `5c_vote.rs` | plans proposed and votes answered |
| `data/<country>/FRM.toml` | adds: the insolvency law (POLICY of the parliament); leverage tolerance, cash buffer target in weeks of outgoings, dividend adjustment speed (PREFERENCE of management); financing, payout and distress review schedules (PREFERENCE); review costs in hours and the procedure's fees (TECHNOLOGY, legal POLICY) |
| `data/shared/SHAPES.toml` | the four forms below, with sources |

**Design**

- **Funding** (FRM.9), a lumpy decision on the firm's financing review days (monthly) and when its cash outlook shows
  a shortfall. Inputs, as FRM.9 lists: what each source costs it now and how close it is to its management's leverage
  tolerance (PREFERENCE). The need is its outlook of outgoings, dues and planned investment over its horizon less its
  cash beyond its buffer target. The sources it can reach, each at its marginal cost:
  - retained cash: its deposit rate forgone;
  - trade credit: the implied rate of the discounts it would forgo (S2.02);
  - an undrawn credit line: the line's rate;
  - a new loan: the best quote from the banks it asks (BNK.6, S1.09), an application sent now and answered later.

  It takes sources cheapest first until the need is met, stopping at the source that would carry its debt over its
  tolerance. The form — a pecking order by marginal cost bounded by a leverage tolerance (Myers, 1984; Myers and
  Majluf, 1984) — is listed in `SHAPES.toml`; bonds, paper and shares join the source list when S3.04 and S3.05
  declare them, as data (Law 10).
- **Payout** (FRM.10), a continuous decision on the firm's payout schedule after each statement (ACC.9):
  - the free cash `D* = cash − expected outgoings and dues over the horizon − buffer target − planned investment's
    own funding`;
  - the dividend moves from the last one by its management's adjustment speed s: `D = D_last + s·(D* − D_last)`,
    what owners expect being the last dividend; it is paid only within the law's distributable reserves (a
    `DeclaredLimit` from the company law), and not at all while negative;
  - paid to the holders of its ownership lines (FRM.23) as an instruction at 7.

  The form is Lintner's (1956) partial adjustment, its target derived from the firm's own cash and plans rather than
  declared, listed in `SHAPES.toml`. The adjustment speed is PREFERENCE; no payout ratio is a parameter.
- **Distress** (FRM.12), a lumpy decision on a need: an arrears or collection notice (S2.01, S2.02), a failed payment,
  or a cash outlook short within the horizon. The actions it can take, each with the cash it raises and the value it
  loses:
  - cut production and input orders (the value of the margin forgone);
  - lay off staff (LAB's layoff path, S1.08, with severance as owed);
  - sell plant or stock (CAP.4's sale, S1.04; stock at a lower posted point);
  - draw its lines;
  - borrow at any quote it gets;
  - ask suppliers for time (a request to the sellers on its payable lines, answered by their terms rule);
  - ask its lenders to restructure (answered by their workout, S2.01).

  It takes them in order of value lost per unit of cash raised until the shortfall is covered. The form — the
  responses of distressed firms (Asquith, Gertner and Scharfstein, 1994), ordered by cost — is listed in
  `SHAPES.toml`. Seeking a buyer is added by S4.06.
- **Failure** (FRM.15), in two ways, either without the other:
  - **default of payment**: a fail still unpaid after the law's grace, found at 2e (S1.03);
  - **balance-sheet insolvency**: liabilities over assets at the firm's own carrying values (ACC.10's read). For
    cells, net worth per member at zero is a registered kink (architecture §7.4), so no cell joins across it, and a
    member whose net worth crosses it is flagged; the 9c `solvency` handler is index-driven over flagged cells and
    over individuals whose carrying values moved today.
- **The law** (POLICY): each country declares which trigger opens which procedure and who may open it — the debtor
  (its own choice, or its duty to file on balance-sheet insolvency) or a creditor's petition (a bank's enforcement,
  S2.01) — and the procedures:
  - **liquidation**: the firm ends into an estate at 2e (S1.03, S2.04);
  - **restructuring**: a stay, then a plan and a vote. At the opening (2e) the debtor's rows on its claim lines are
    moved to **procedure lines** whose terms carry the stay (S0.17's `ToProcedureLine`), so the stay suspends the
    debtor's dues alone and its co-holders on the shared lines pay on; confirmation or liquidation retires the
    procedure lines.
- **The plan**: at 5c after opening, the debtor proposes one plan (`propose_plan`): its own going-concern value
  (VAL.8 `firm_value` over its expected margins after the plan) is distributed over its claims by the law's priority
  rule, class by class of seniority; each class is offered new terms or a haircut (new lines on the lenders' points).
  If the going-concern value is below its own estimate of what liquidation would pay, it proposes none and
  liquidates.
- **The vote**: each creditor answers at the next 5c by `vote_on_plan`: yes when the plan's claim value to it under
  its own assessment is at least its own valuation of its liquidation recovery (the waterfall over its valuation of
  the assets, MKT.20). A bank's claim values come from its `LoanAssessment` by `sys-bnk`'s rule; a supplier's from its
  loss outlook per buyer class (S2.02); others' from their own outlooks. The form is listed in `SHAPES.toml`. A
  creditor cell's members vote alike, with the count of the line side. A class accepts when the yes votes reach the
  law's majority by value and by count. The plan is confirmed at the next 2e when every class accepts, or when the
  law's rule for dissenting classes allows it; otherwise the firm liquidates.
- **Confirmation**: one instruction settled at 7 retires the old claims' rows and writes the new ones (line transfers
  and new lines), and the creditors' losses are recognised at the next 2e (S2.01's write-off). The form — a plan by
  absolute priority with class voting (Franks and Torous, 1989; Bebchuk, 1988) — is listed in `SHAPES.toml`; the
  thresholds are the law's (POLICY).
- **Cells**: the members who fail are the ones S0.17's prefix failure splits out, so a procedure opens for a part of
  count k; its plan and votes apply to all k alike, and if it liquidates the k end into **one** estate holding their
  count on every line it succeeds to (S2.04's estate per (part, occasion)).
- **A lender's offer** (`answer_restructuring`, the firm's): a firm answering a `RestructuringOffer` (S2.01) accepts
  when the offer's value to it — its going-concern value under the new terms, by its own cash-flow outlook discounted
  at its marginal cost of money — beats the best of its other distress routes (the distress rule's values above,
  enforcement included). The form is the same comparison as the distress rule, listed in `SHAPES.toml`; it retires
  S2.01's placeholder for firms.
- **No immortal firm** (FRM.21): funding never exceeds what a source grants; nothing tops up a firm's cash.
- **Review costs**: financing and distress decisions cost management hours (TECHNOLOGY); plans and votes cost the
  procedure's fees, paid to named parties (legal POLICY).
- **Streams**: `FRM.plan_tie_lot` (ties the rules leave, among classes' equal claims).

**The firm cell's record** gains:

| Item | Bytes |
| --- | --- |
| Review exposure and attention rate for the financing review (8 + 4) | 12 |
| The last dividend per member, a position | 8 |
| **Total after S2.03** | **428** |

Payout is continuous and distress is taken on needs, so neither has a review exposure.

**Unit tests**
- `funding_cheapest_within_tolerance`: over given costs and debt, the sources taken and the stop at the tolerance.
- `payout_partial_adjustment_within_reserves`.
- `distress_orders_by_value_lost_per_cash`.
- `insolvency_tests_either_without_other`.
- `plan_by_absolute_priority`: a given going-concern value over given classes.
- `restructuring_vote_by_class`: majorities by value and count, with a dissenting class under each law's rule.
- `answer_restructuring_against_distress_routes`: over given values, the offer is taken only when it beats the best.

**Live checks**
- `LC-2-06`: FRM.19 — size, growth, age, productivity, margins, markups, the share of firms in distress, and entry and
  exit by industry and age are published; exits happen both for cash and for solvency.
- `LC-2-07`: PTY.9 — every firm that ended has an estate or a successor.
- `LC-2-27`: every procedure names its trigger, its opener and the law's procedure; every restructuring has a plan,
  each class's votes and an outcome on the dates the law gives.
- `LC-2-28`: no firm whose liabilities exceed its assets on its carrying values stays outside a procedure longer than
  its law allows.

**Budget**:
- Financing and distress decisions are "Occasion evaluations"; payouts run inside the firm's visit; restructured and
  failed members are parts.
- The solvency test is index-driven at 9c, in "Valuation, accounts, tests".
- Counters, ratcheted: `phx_frm.financing_decisions`, `phx_frm.dividends`, `phx_frm.distress_actions` by kind,
  `phx_frm.insolvencies` by trigger, `phx_frm.restructurings`, `phx_frm.plans_confirmed`,
  `phx_frm.procedure_rows_moved`, `phx_pop.bytes_per_firm_cell` (at 428).

**Guards**: none new.

**Not allowed**:
- a firm that cannot fail;
- an exit rate;
- a restructuring outcome written as a rule;
- a stay written on a line other holders share;
- a payout ratio as a primitive;
- a leverage target imposed from outside management's tolerance.

**Done when**
- [ ] Firms fund themselves, pay out, act in distress, fail both ways and are restructured or liquidated.
- [ ] LC-2-06, LC-2-07, LC-2-27 and LC-2-28 pass.
- [ ] Two reviews are done.

---

### S2.04 — `sys-est` in full: estates, the waterfall, inheritance in kind

**Status**: planned

**Clauses**:
- PROCESS: L3 *(completes it: the machinery by which every ending distributes — S0.17's waterfall and this step's
  administration; later parties' endings — funds, insurers, schemes, clearing houses — use it when their systems add
  their triggers)*; POP.9 *(completes it:
  a person's estate sells only what its debts need and passes the rest in kind)*; REG.12 *(part: holdings resolved by
  estates; maturity, redemption and conversion are S3.05's)*.
- FORBID: POP.15 *(completes it)*.
- This step retires S0.25's placeholder naming the buyers' systems for what household estates hold, for everything
  but dwellings (S2.05) and securities without a market (S3.05, S3.06).

**Architecture**: §4.4 (line transfers), §6.1 (2e, 5c, 7, 10b), §9.1.

**Depends on**: S2.03.

**Goal**: every ending has a destination for everything. Firms' estates sell into real markets to real bidders;
persons' estates sell only what their debts, taxes and costs need and pass the rest in kind to heirs drawn from kin
lines; claims rank by the country's law; employees are released through the labour market; suppliers lose
receivables by name; references resolve to the estate.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-pop/src/estate.rs` | the inheritance law as declared data: heirs' classes and shares from kin lines, the law's destination with no heir; the heir's need occasion `Inherited` |
| `crates/systems/sys-est/src/rules/administer.rs` | the administrator's choice of what to sell, and its asks |
| `src/rules/heirs.rs` | shares per heir class, in kind |
| `src/handlers/5c_administer.rs` | sale decisions and ask revisions, orders into the markets |
| `src/handlers/2e_distribute.rs` | the waterfall once sales are done, and the distribution in kind |
| `src/audit.rs` | the Estates family: every ended party's positions on its estate or successor; every estate distributes |
| `data/<country>/EST.toml` | the insolvency law's ranking of classes and the liquidation horizon; the inheritance law; administration fees (POLICY and TECHNOLOGY), with sources |
| `data/shared/SHAPES.toml` | the administrator's form, with its source |

**Design**

- **Opening** (2e, S0.25 and S1.03): an estate row, an individual of the estate kind, succeeds the party; its holdings
  and lines pass to it by line transfer, and every reference resolves to it (PTY.10). There is **one estate per (part,
  occasion)**: the members of a cell who end on one occasion — a firm cell's members failing together, a household
  cell's members dying on one day — share one estate, which holds their count on every line and holding it succeeds
  to, since their claims and assets are identical. A firm's staff are released on
  their contracts' notice and severance through LAB's separation path (S1.08), their claims ranked as the law
  prefers them.
- **What to sell** (L3, POP.9), the administrator's decision at 5c on the estate's review days (weekly):
  - a **firm's or an institution's estate** sells everything but money: stock where it always sold (posted or call,
    S1.05), plant bilaterally to buyers who can use it (CAP.4's market, S1.04), receivables to banks (S2.02's
    factoring) or by collection, loans (S2.01's sales);
  - a **person's estate** sells only what its debts, taxes and costs need: it covers them first from money, then sells
    the holdings that raise the cash with least loss of value to the heirs (the most liquid first, a divisible
    holding only in the quantity needed, an indivisible unit — a dwelling, a vehicle, a business — only when the rest
    does not suffice). Everything else passes in kind.
- **Asking** (L3): the administrator lists at its valuation (MKT.20, the estate's named valuer), and at each review
  moves its ask down one point of the trade's table when the time left to the law's liquidation horizon (POLICY) is
  shorter than its outlook of the time to sell at that ask (its outlook of time on market for the thing, from the
  market's published records). What it fetches is an outcome; there is no floor. The form — a descending ask under a
  deadline (Shleifer and Vishny, 1992, on liquidation values) — is listed in `SHAPES.toml`; its only parameter is the
  law's horizon.
- **The waterfall** (2e `distribute`, S0.17): when everything to be sold has sold or the horizon has passed (what did
  not sell passes in kind or to the law's destination):
  - secured claims from their own collateral's proceeds, then classes in the law's order — preferential (employees,
    taxes, and deposits where the law prefers them), ordinary unsecured including trade creditors, subordinated, then
    equity — equal ranks pro rata, per currency, with the residue landing on the party the law names;
  - its instructions settle at 7. A claim held by a cell's line side is paid to the side, and where only some of its
    members are reached the payees who lose are drawn from the line (REP.23, the claim line's stream);
  - each holder's shortfall is its recognised loss, an event at its next 2e (L1).
- **Inheritance in kind** (POP.9): what remains passes to heirs drawn from the estate's kin lines (REP.23, stream
  `EST.heirs`) by the law's classes and shares:
  - divisible holdings (deposits, securities, fund units) by share, exactly (REP.9);
  - indivisible units (a dwelling, a household business, a vehicle) whole to one heir drawn by the law's order, the
    others receiving the difference in money where the estate has it, and otherwise holding the unit jointly as the
    law declares;
  - the transfers are one instruction settled at 7; the heirs' members split with what they received (REP.23) and land
    at 10b, and each receives an `Inherited` need occasion for the unit, on which it keeps, sells or runs it by its
    own decisions (S2.05's `where_to_live` for a dwelling; S1.03's closure for a business).
  - With no heir the law's destination takes the rest (S0.25).
- **Holdings resolve** (REG.12): every holding the estate held ends as cash, a holding of an heir, a recovery claim
  paid by the waterfall, or a recorded loss; a security with no market yet (bonds and shares before S3.05) passes in
  kind, and waits, counted, on a placeholder naming EQY (S3.05) where the debts need it sold.
- **The end** (L3): when its distribution settles, the estate ends with no successor (nothing remains) and its
  references close (SET.13).
- **Review costs**: administration fees and hours, paid to named parties by the estate (a preferential cost).
- **Streams**: `EST.heirs` (pairing), `EST.sale_lot` (which of identical units sell first).

**Unit tests**
- `estate_sells_only_what_debts_need`: over given holdings and debts, the sale set, including an indivisible unit sold
  only when the rest falls short.
- `ask_moves_toward_deadline`.
- `inheritance_in_kind_to_heirs`: shares, whole units and money differences.
- `waterfall_order_with_preferred_deposits`.
- `holdings_resolve_to_named_ends`.

**Live checks**
- `LC-2-08`: L3, PTY.10 — every ended party's positions landed on a successor or its estate; none vanished.
- `LC-2-09`: L3 — every estate sale is a match in a market at its price; no estate distributed a valuation.
- `LC-2-29`: POP.9 — every person's estate names its heirs and what each took in kind; what it sold is no more than
  its debts, taxes and costs need plus at most one indivisible unit.
- `LC-2-30`: REG.12 — every holding in an ended estate resolved to cash, an heir's holding, a paid claim or a recorded
  loss.

**Budget**:
- Estates open follow Little's law, open = rate × life (architecture §9.1): about 5 000 firm members end a day,
  grouped by (part, occasion) into about 800 estates, each about 40 days in administration (the weekly reviews to the
  law's horizon), about 32 k open; household estates about 1 000 a day × about 25 days, about 25 k. With S2.11's
  insolvency estates (about 3 k), about 60 k are open at the worst day. Each estate is a kind-table row of 512 bytes
  (the estate facet: succession, count, law, horizon, sale and waterfall state, arena references), so §13.1's estates
  line is 60 k × 512 B = 31 MB; their rows and holdings are in the relationship-rows and holdings lines.
- Heirs' parts are in "Parts".
- Counters, ratcheted: `phx_est.estates_open` by kind, `phx_est.mean_life_days` by kind, `phx_est.sales`,
  `phx_est.distributions`, `phx_est.in_kind_transfers`, `phx_est.waiting_on_placeholder` by system.

**Guards**: none new.

**Not allowed**:
- an estate that values rather than sells;
- an estate per member where the members ended together;
- a claim ranked by instrument type rather than its seniority;
- an estate collecting receivables while its trade creditors rank nowhere;
- a person's estate selling what its debts do not need;
- an inheritance paid as a share of a valuation.

**Done when**
- [ ] Every ending distributes, and persons' estates pass what remains in kind.
- [ ] LC-2-08, LC-2-09, LC-2-29 and LC-2-30 pass.
- [ ] Two reviews are done.

---

### S2.05 — `sys-hsg`: housing, land, mortgages, foreclosure, construction and moving

**Status**: planned

**Clauses**:
- STATE: HSG.1, HSG.2, HSG.3, HSG.19; CAP.2; GDS.3 *(completes it: land grown on)*; GEO.4 *(part: private owners'
  life, maintenance and condition; public owners decide from S5.02, where it completes)*.
- DECISION: HSG.4, HSG.5, HSG.6, HSG.7, HSG.8, HSG.9, HSG.18; HH.8, HH.10; HH.9 *(part: moving within a country;
  abroad is S5.05)*; LAB.5 *(part: search in other regions, with a move)*.
- PROCESS: HSG.10, HSG.11, HSG.12, HSG.20; CAP.7 *(part: private owners; public owners at S5.02)*; POP.8 *(part:
  within a country)*; MKT.20 *(part: the dwelling appraiser)*; REP.22 *(completes it: tastes
  over dwellings)*; REP.24 *(completes it: dwellings' and households' vehicles' wear and repair between condition
  classes)*; HH.13 *(part:
  repossession)*.
- INVARIANT: HSG.13, HSG.14.
- MEASURE: HSG.15; STA.1 *(part: house prices and rents)*.
- FORBID: HSG.16.
- PRIMITIVE: HSG.17.
- This step retires S1.15's placeholder naming HSG (dwellings held without a housing market), S0.25's naming HSG (the
  opening tenancy lines' decisions), S1.12's naming HH (no new household loans), S2.01's naming HSG (dwellings in
  enforcement wait) and S2.04's naming HSG (estates' dwellings wait).

**Architecture**: §3.1 (decision-point homes), §4.2 (pins), §4.4 (the composite sale), §7.3, §7.4, §7.5, §7.9
(rent points), §7.10, §9.1, §13.

**Depends on**: S2.04.

**Goal**: dwellings and land as assets and services. Households decide where to live — rent or buy, which zone and
class, which region — from their own state, what lenders offer and the rents and prices in reach. Sellers ask,
buyers bid within their deposit and their lender's offer, and sales match by search and negotiation; landlords set
rents; builders buy land and build; lenders set mortgage standards; foreclosures add supply; land trades by
negotiation or auction. Households borrow for a dwelling, a vehicle or a shortfall, and against their home's equity.
A rate rise reaches house prices through what buyers can borrow.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-property/src/*.rs` | dwelling classes (kind, size, quality band, condition band) and land parcels as declared data; the tenancy, land-lease and mortgage terms (S0.25); message kinds `Listing` (across days, with a count), `Viewing`, `Offer`, `Counter`; the `SaleCommitment` pin; zoning as policy values |
| `crates/interfaces/if-credit/src/decisions.rs` | adds the household's `where_to_live` (HH.8, HH.9, HSG.4) and `borrow` (HH.10), which live here because they read dwellings and mortgage and loan terms (architecture §3.1); `borrow` is also taken on S1.12's `refinance` review (prepaying, refinancing, drawing on equity) |
| `crates/interfaces/if-credit/src/mortgage.rs` | `LoanCommitment` and `RedemptionCommitment` kinds (architecture §4.4); the mortgage offer message |
| `crates/systems/sys-hsg/src/rules/{where_to_live,ask,bid,answer,rent,landlord,build,land,repair}.rs` | the decisions below |
| `src/search.rs` | listings and searchers meeting at 6a by the meeting hazard; viewings become offer occasions |
| `src/handlers/*.rs` | 5c housing reviews, listing, bids, answers and acceptances (the composite sale), rent reviews, builders' and landowners' decisions; 6a search meetings and land auctions; 4a construction progress, completions and dwelling wear at booked kinks; 3c foreclosure notices take effect; 9d house-price and rent records |
| `crates/systems/sys-bnk/src/rules/mortgage.rs` | HSG.9: standards and offers as commitments; HH.10's other loans (consumer, vehicle, credit lines, home equity) |
| `crates/systems/sys-hh/src/rules/borrow.rs` | HH.10: the household's borrowing decision |
| `crates/systems/sys-dem/src/handlers/3c_move.rs` | POP.8: the move executed, re-keying region and zone |
| `crates/systems/sys-cap/src/construction.rs` | CAP.2 projects; CAP.7 infrastructure's life, maintenance and condition |
| `crates/systems/sys-sta/src/housing.rs` | house-price and rent series from sales and lettings records (STA.1, part) |
| `data/<country>/HSG.toml` | construction technology and lead times, dwelling wear curves and repair costs, search meeting rates, completion lead time of a sale (TECHNOLOGY); zoning, property and tenancy law, agents' and legal fees (POLICY); dwelling and location taste distributions (PREFERENCE); review schedules and costs in hours (PREFERENCE, TECHNOLOGY); moving costs in hours and services (TECHNOLOGY) |
| `data/<country>/gen/HSG.toml` | the opening stock by (zone, class) with owners, land ownership and leases, construction under way (ENDOWMENT) |
| `data/shared/SHAPES.toml` | the forms below, with sources |

**Design**

- **Dwellings** (HSG.1, REP.24): an individual's are named units (S0.14); a cell's are holdings per (zone, class),
  each unit's tile drawn when something depends on it. Occupancy is the dwelling role's attachment: owned, a tenancy
  row (HSG.2), or a lodger's room tenancy. A household with none is recorded homeless (HSG.13).
- **Where to live** (HH.8, HH.9, HSG.4), one lumpy decision of the household on its housing review days (yearly) and
  on needs: a notice to leave, a tenancy ending, an inheritance, a catastrophe, a job offer in another region, and
  from S6.02 a household forming. It reads, as HH.8 and POP.8 list: its income and outlook; its savings for a deposit;
  what lenders will offer it (their posted mortgage rates and standards, and its own last quotes; it applies only
  after choosing to buy); the rents and asking prices in reach (published listing
  and lettings records per (zone, class)); its outlook for both (public-series outlooks of the house-price and rent
  series, S1.01); and, for another region, its outlook of that region's published wages and unemployment for its
  adults' occupation families, and the real cost of moving. The alternatives are staying, renting and buying, per
  (region in reach, zone, class):
  - the value of owning is the user cost — the mortgage rate it is quoted on the loan it needs, the return forgone on
    its deposit, maintenance, expected price change — against the rent saved (Poterba, 1984);
  - each alternative's utility adds the type's tastes for size and location (HH.3) and a taste draw per member
    (REP.22, stream `HSG.dwelling_taste`), less moving costs where it moves;
  - the choice is a nested logit — region, then tenure, zone and class (McFadden, 1978; Kennan and Walker, 2011) —
    listed in `SHAPES.toml`. Members' counts per alternative are multinomial (REP.22).

  Members who choose to rent search the lettings in reach; those who choose to buy apply for a mortgage and search
  for sale listings; those who move region are moved by `sys-dem` at 3c on the day their new dwelling is theirs (or
  at once, renting, from the first letting accepted). Review costs are hours at the value of leisure (TECHNOLOGY).
- **Moving** (POP.8, HH.9): the movers split out, re-keyed to the destination's region and zone; their lines end or
  continue by their terms — employment by notice (a quit, S1.08), a tenancy by its notice, loans and deposits
  unchanged. The move's cost is paid to named parties: a removal service bought at posted prices (S1.06) and agents'
  fees in the sale or letting. Searchers may apply to vacancies in other regions they can reach (LAB.5, part); an
  accepted offer there is a need occasion for `where_to_live`.
- **Mortgage offers** (HSG.9): a would-be buyer applies to lenders it can reach (BNK.6); each lender quotes by S1.09's
  form and its **standards**: the highest loan-to-value and income multiple and the lowest debt-service coverage it
  will write, moved on its review days by S1.09's form against its losses, funding (S2.06) and capital (S2.07), and
  bound by the supervisor's limits from S2.08. An accepted offer is a `LoanCommitment` (REG.10) for a maximum
  principal at a rate, valid for a declared period, which pins the members who hold it (architecture §4.2).
- **One part per transaction** (architecture §4.2): members acting on a housing transaction split out once, at its
  first pin — a buyer's accepted `LoanCommitment`, a seller's accepted offer — into a part that carries every later
  pin of the transaction (the `SaleCommitment` on both sides) as key attributes. The next pin is written on that same
  cell, which re-keys in place (S0.23), and completion clears the pins by another re-key in place, after which the
  cell lands wherever its key does. A sale therefore makes one part per side, not one per step.
- **Sellers** (HSG.5): an owner who lists — having chosen to move, trade or sell as an investment, or an estate, or a
  lender after foreclosure — asks from its own outlook of the price (its method's outlook of the (zone, class)
  series), what it owes on the dwelling, and its urgency (the time it can wait, from its own horizon, or the law's for
  an estate or a lender). It moves its ask down a point when its time on market exceeds its outlook of time to sell at
  that ask. There is no floor: what binds is the composite sale's feasibility (S0.17), since the sale's redemption leg
  must be funded from the price or the seller's cash, so an owner who cannot fund it cannot complete a sale below its
  debt, and one who can may. The form — asks from outlook, debt and urgency (Genesove and Mayer, 1997; Merlo and
  Ortalo-Magné, 2004) — is listed in `SHAPES.toml`. A cell's listing is one `Listing` with a count of identical units
  at one ask point.
- **Search** (HSG.10), at 6a each day (the housing market meets on business days): each searching group — the pieces
  of a cell's buyers or renters in one zone with one budget step (architecture §7.9) — meets each listing in its reach
  of its chosen (zone, class) by the declared meeting hazard per (listing, searcher) (CHN, stream `HSG.meeting`);
  meetings are `Viewing`s, which become offer occasions at the next 5c.
- **Buyers** (HSG.6): a buyer bids up to its own value (VAL.8 `dwelling_value`: the rent it saves and the price it
  expects), limited by its deposit plus its loan commitment. It bids the price point halfway between the ask and its
  value when its value is above the ask, expecting the protocol below to split the rest, and at its value otherwise.
- **Negotiation** (HSG.10), a declared alternating protocol over days (MKT.7): the seller answers an offer at the next
  5c — taking the best of the day's offers, ties by lot (`HSG.offer_lot`): accept when at or above its ask, counter at
  the midpoint of offer and ask when above its reservation (its own value net of its urgency), otherwise refuse; the
  buyer answers a counter at the 5c after by the same rule against its value; the protocol ends in two rounds
  (Rubinstein, 1982, truncated; listed in `SHAPES.toml`). A listing that meets no buyer stays listed or is withdrawn
  by its owner's review, so volumes fall before prices.
- **The sale** (architecture §4.4): `sys-hsg`'s acceptance handler writes one composite instruction drawing on the
  buyer's lender's `LoanCommitment` and the seller's lender's `RedemptionCommitment`: the buyer's deposit, the loan
  paid out, the price to the seller, the payoff, agents' and legal fees, the title and the new loan row, all or none,
  settling at 7 on the completion day (the law's lead time). Until then a `SaleCommitment` pins both sides' members.
- **Lettings** (HSG.2, HSG.7): lettings are posted at rent points. The rent asked per (landlord, zone, class) is
  written by `sys-hsg`, the one writer of every landlord's rent point (households and firms): on its rent review days,
  by S1.03's state-dependent form over the landlord's user cost of holding (financing, maintenance, depreciation,
  expected appreciation) with the pressure of its vacancy against its target vacancy (PREFERENCE), moved only past
  its menu cost. Renters meet lettings by the same search; the first acceptable letting met is taken at its posted
  rent (MKT.6). A tenancy is a row on the line of (zone, class, rent point, term). Landlords buy and sell dwellings as
  investments on their review: buy when their value of the rent earned exceeds the price and costs (VAL.8), sell by
  the seller's form.
- **Builders** (HSG.8, CAP.2): a construction firm, on its investment review (CAP.3's form, S1.04), values a project
  by the expected sale prices of the dwellings it permits less land, construction and financing cost at its marginal
  rate; it buys land (below), obtains permission — zoning (a policy value) and the permission's lead time (POLICY) —
  and starts a **construction project** (CAP.2): owner, site, builder, budget, schedule and work done, carried at
  cost. Work progresses at 4a as a physical flow realised at booked kinks; completion after the lead time adds the
  units to the owner's holding, which lists them.
- **Land** (HSG.3, HSG.18–HSG.20, GDS.3): parcels are holdings of tile area, zoned. A landowner sells, leases or holds
  by its own value of the land — what it earns (crops by its known ways on that land's grade, rent from a lease, a
  site for plant or dwellings) against what it expects the land to fetch. Sales go by negotiation (MKT.7) or by the
  region's land auction on its declared days (MKT.3, at 6a; ties by `HSG.land_lot`); leases are lines (owner, tenant,
  parcel, rent, term). The land price is a read of sales and is absent where none happened; a tax or lender reads a
  valuation (MKT.20). Crops grown on owned or leased land are commodities whose grade is the land's (GDS.3).
- **Foreclosure** (HSG.11, HH.13): after S2.01's enforcement on a mortgage, `sys-bnk` requests the title transfer of
  the dwelling (a line transfer its kind allows the lender), settled at 7; the occupants receive a notice to leave,
  effective at 3c after the law's notice period, a need occasion for `where_to_live`; the lender lists with its own
  short horizon, and the sale's proceeds pay the loan through S2.01. What the sale leaves — a shortfall, which is a
  claim on the borrower where the country's recourse law allows it and a write-off where it does not, or a surplus,
  which goes to the borrower — follows the declared law (POLICY).
- **The dwelling appraiser** (MKT.20, part): each lender names an appraiser, a party whose declared method values a
  dwelling from the recent sales of its (zone, class) nearby (the prints of this market), adjusted by condition class;
  a mortgage offer, a home-equity draw and an estate's inventory read its valuation.
- **Households' vehicles** (HH.10, REP.24): a household holds vehicles as units by (zone, class), bought from
  producers at posted prices (S1.06) as a lumpy decision on its occasions (a vehicle's value to it — the trips it
  makes at its value of time against the alternative — against the price and running cost; listed in `SHAPES.toml`),
  worn by age and use on the declared curve like dwellings, repaired, sold bilaterally or scrapped, and financed by
  term loans (BNK.17) through the borrowing decision below. Holdings rows in the household's arena; the review kind is
  `vehicle` (counted below).
- **Borrowing** (HH.10), a lumpy decision on needs — a dwelling purchase, a vehicle, a shortfall that the spending
  rule meets at its funds kink (S1.12) — and on the `refinance` review. The household takes a loan when the value to
  it (the buffer-stock rule's value with the loan's cash and its repayments, against without, at the quoted rate)
  is positive, choosing among the offers by S1.09's shopping (stream `BNK.lender_taste`); it draws its credit lines
  (BNK.18) when its spending would otherwise cross its funds kink; it refinances or draws on its home's equity at the
  lender's valuation (MKT.20) when the rate saved or the cash's value exceeds the fees. The form — buffer-stock saving
  with borrowing (Carroll, 1997; Kaplan and Violante, 2014) — is listed in `SHAPES.toml`.
- **Wear and repair** (HSG.12, REP.24): dwellings move between condition classes by age and use on the declared curve,
  lazily, the next class day booked on the agenda and realised at 4a; damage from catastrophes and accidents moves
  them down (S0.25's two-level draw). On its housing review an owner repairs when the value of the better class
  exceeds the repair's cost (CAP.4's form), buying the repair service at posted prices.
- **Infrastructure** (CAP.7, GEO.4): segments of the network (S1.07) have a life, a maintenance need and a condition
  class, with capacity falling by condition (TECHNOLOGY). A private owner maintains, extends and charges users (a
  toll at its posted point) by CAP.3 and CAP.4's forms. A public owner's decisions wait for its agencies (S5.02):
  until then it repairs a segment when its condition falls below its opening class, bought through S1.11's public
  purchases, a placeholder naming SOC.
- **Published series** (STA.1, part): the house-price and rent series per (region, class) from the sales and lettings
  records, with their lags, are public series for outlooks.
- **Streams**: `HSG.dwelling_taste`, `HSG.meeting`, `HSG.offer_lot` (among equal offers on one listing the same day),
  `HSG.land_lot`.

**The household cell's record** gains, against S1.12's 504 bytes:

| Item | Bytes |
| --- | --- |
| Review exposure and attention for `where_to_live` (8 + 8) | 16 |
| Review exposure and attention for `vehicle` (8 + 8) | 16 |
| Review exposure and attention for the rent review, for households that let dwellings (8 + 8) | 16 |
| **Total after S2.05** | **552** |

Borrowing is taken on needs and on S1.12's `refinance` review, so it adds no kind. **The firm cell's record** gains
the rent review for firms that let dwellings (exposure 8 + attention rate 4), 12 bytes: **440** of 500 after S2.05.

**Agenda reasons** stay within S0.08's sixteen per table: a dwelling's or vehicle's next condition class is a kink
day (architecture §7.4) on the existing kink-day reason, and the key clocks of S2.10 and S2.11 share S0.23's one
key-clock reason. Households use thirteen (S1.12's twelve and the key clock), firms nine.

**Unit tests**
- `user_cost_comparison`.
- `nested_logit_shares`: over given utilities, tenure and zone shares sum to the members.
- `bid_limited_by_deposit_and_commitment`.
- `ask_from_outlook_debt_and_urgency`: a forced seller asks less; nothing floors the ask.
- `negotiation_ends_in_two_rounds`.
- `composite_sale_all_or_none`: over given legs, one short leg fails every leg.
- `one_part_per_transaction`: over a given transaction's pins, one split, then re-keys in place.
- `move_compares_regions_net_of_cost`.
- `borrow_value_positive_only_when_it_smooths`.
- `dwelling_wear_moves_classes`.

**Live checks**
- `LC-2-10`: HSG.13 — every dwelling has one owner and at most one occupying household besides lodgers; every
  household is housed or recorded homeless.
- `LC-2-11`: HSG.14 — mortgages owed equal mortgages held.
- `LC-2-12`: HSG.15 — prices, rents, their ratio, transactions and time on market are published per region, and so
  is the **lead of volumes over prices**: the lag in months at which the cross-correlation of monthly changes in
  transactions and in the price series peaks, over the last three years. It passes when every region with sales has
  all of these published; whether volumes lead is S7.01's to judge.
- `LC-2-31`: every dwelling sale is one composite instruction drawing on its lenders' commitments; no mortgage exists
  without a lender's balance sheet on its other side (HSG.16).
- `LC-2-32`: HSG.20 — the land price is absent wherever no sale happened, and every valuation read is labelled.
- `LC-2-33`: POP.8 — every move names its cost paid to named parties and its region; the population per region
  reconciles (POP.11).
- `LC-2-34`: HSG's Done when — on an experiment copy (N6) with the facility rates raised, what buyers can borrow
  falls, then volumes, then prices, and floating mortgage payments reach spending.
- `LC-2-35`: every foreclosure traces default, enforcement, title transfer, notice, listing and sale.

**Budget**:
- Memory (§13.1): the household cell at 552 bytes, 48 B × 0.7 M = 34 MB more; the firm cell within its 500;
  listings as messages across days, about 0.2 M with counts at 64 bytes, 13 MB; commitments within their line.
- Time (§13.2): housing search is a new share of "Meetings" — about 50 k searching groups a business day meeting
  about 20 listings each at ≤ 50 core-ns, 50 k × 20 × 50 ns ÷ 3 ≈ 17 ms wall. Sales, lettings and moves make about
  30 k parts a day with one part per transaction: 30 k × 2.5 µs ÷ 3 ≈ 25 ms wall in "Parts" (≈ 121 ms at F-001's
  measured 12.1 µs). Reviews and offers are "Occasion evaluations"; wear and construction are "Physical flows
  realised".
- Counters, ratcheted: `phx_hsg.listings`, `phx_hsg.viewings`, `phx_hsg.sales`, `phx_hsg.lettings`,
  `phx_hsg.foreclosures`, `phx_hsg.completions`, `phx_hsg.land_trades`, `phx_hsg.parts_per_transaction`,
  `phx_hh.moves`, `phx_hh.loans_taken`, `phx_hh.vehicle_reviews`, `phx_hsg.rent_reviews`,
  `phx_pop.bytes_per_household_cell` (at 552), `phx_pop.bytes_per_firm_cell` (at 440).

**Guards**: none new.

**Not allowed**:
- a house price path or a rent index driving rents;
- a reservation floor;
- a mortgage without a lender's balance sheet;
- a dwelling without an owner;
- a move without a cost;
- a land price where no sale happened.

**Done when**
- [ ] Housing, land and mortgages run from individual decisions; foreclosures add supply; households move and borrow.
- [ ] LC-2-10 to LC-2-12 and LC-2-31 to LC-2-35 pass.
- [ ] Two reviews are done.

---

### S2.06 — `sys-bfl`: bank funding and liquidity

**Status**: planned

**Clauses**:
- STATE: BFL.1; BFL.2 *(part: bilateral interbank term loans and the central bank's facility; repo at S3.01,
  certificates, paper and bonds at S3.04, where it completes)*; BFL.3 *(part: reserves, banknotes, bills held to
  maturity, assets the central bank accepts and loans saleable bilaterally; securities' market depth at S3.06, where
  it completes)*; MMK.1 *(part: unsecured term loans between named banks, bilateral, brought forward; overnight loans
  and the market at S3.01, where it completes)*.
- DECISION: BFL.4, BFL.5; BFL.6 *(part: bilateral interbank borrowing, pledging, bidding for deposits, shrinking
  lending and the facility; the money market's route is S3.01)*; BFL.7 *(part: another bank, banknotes and bills; a
  money fund at S3.07, where it completes)*.
- PROCESS: BFL.8, BFL.9; BFL.10 *(part: short after the central bank's facility; after the money market too at
  S3.01, where it completes)*.
- INVARIANT: BFL.11.
- MEASURE: BFL.12.
- FORBID: BFL.13.
- PRIMITIVE: BFL.14.
- This step retires S1.09's four placeholders naming BFL: the marginal cost of funds, the deposit-rate rule, the
  facility use and the reserve target. `sys-bfl` is the one writer of deposit rate points from here (the register
  records the change of declarer), and the one writer of the banking arrangement.

**Architecture**: §3.1 (decision-point homes), §4.5 (deposits, the banking arrangement), §6.1 (5b, 5c, 7, 7e,
8d–8f), §7.3 (wakes), §9.2, §13.2 (the bank-run day).

**Depends on**: S2.05.

**Goal**: a bank's funding is a mix of liabilities that leave at different speeds. Each bank prices deposits from its
own need, holds a buffer from its own view of how fast its funding could leave, acts in order of cost when short,
and fails for liquidity when its reserves are short after the facility has run, whatever its capital. Depositors move
their money on what they can observe; runs emerge.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-banking/src/funding.rs` | deposit kinds by holder class, sight and term (BFL.1); the rule handle `marginal_cost_of_funds` over the funding sources each lender's form declares (implemented by `sys-bfl`, read by `sys-bnk`'s quotes); the bank's liquidity facts (buffer target, surplus beyond it, liquid assets by haircut and depth, run-off outlooks, the shadow cost of cash) |
| `crates/interfaces/if-credit/src/interbank.rs` | the interbank term loan, a loan line kind (BNK.2; MMK.1, part) between named banks, which S3.01 extends with overnight terms and the market rather than declaring a second kind |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `bank_choice` (the depositor's, households' and firms'), implemented by `sys-bfl`; it lives here because its alternatives include money-fund units (S3.07) beside deposit terms and banks' published figures (architecture §3.1) |
| `crates/systems/sys-bfl/src/rules/{deposit_rates,cost_of_funds,buffer,shortfall,interbank,bank_choice}.rs` | the decisions below; `interbank` is the borrowing bank's application and acceptance |
| `src/safety.rs` | the public-series outlooks of each bank's safety, per method |
| `src/handlers/*.rs` | 5c deposit-rate and buffer reviews, shortfall actions, interbank applications and acceptances; 5c depositors' bank choices, whose transfers make their parts at 7c when they settle; 7e the banking arrangements of the holders a settled resolution transfer moved; 8d facility requests; 9d the bank's published funding figures |
| `crates/systems/sys-bnk/src/rules/quote.rs` | extended: banks among the borrower classes, classed from their published funding and capital figures and public events; the interbank quote and decline are S1.09's |
| `data/<country>/BFL.toml` | buffer appetite and deposit-rate adjustment speed (PREFERENCE of management); review schedules; switching costs in hours (TECHNOLOGY); taste distributions over banks (PREFERENCE); review costs |
| `data/shared/SHAPES.toml` | the forms below, with sources |

**Design**

- **Deposit classes** (BFL.1): deposit kinds are declared per holder class (households, firms, institutions) and as
  sight or term (a term deposit a row with its maturity, rolled by its holder's savings decision, S1.12). Each is a
  contract with its rate point and terms.
- **Wholesale funding** (BFL.2, part): in Stage 2, bilateral interbank term loans between named banks (MMK.1's term
  loans, brought forward) and the central bank's lending facility (S1.10). Repo and the market arrive with S3.01,
  certificates, paper and bonds with S3.04.
- **Liquid assets** (BFL.3, part): reserves, banknotes, bills (held to maturity until S3.06) and assets the central
  bank accepts (S1.10's list), each with its haircut, and loans saleable bilaterally (S2.01) with the depth the bank's
  own record of such sales shows.
- **Marginal cost of funds** (the rule handle): over the funding sources the lender's form declares (data: here a
  bank's deposits, interbank quotes and the lending facility; the money market's routes from S3.01, paper and bonds
  from S3.04, other lenders' forms from S3.08), the rate of the cheapest source that would fund one more unit today —
  its posted deposit rate plus the rise it expects to need to attract the unit (its own adaptive outlook of how its
  deposits have answered its rate moves against competitors'), an interbank quote it holds, or the lending facility
  against its free eligible collateral — plus the carry of the buffer the unit's funding class requires (the class's
  buffer share × the gap between that rate and the liquid assets' yield). A source is data, so a later step adds one
  without changing the rule. The form is funds-transfer pricing (Dermine, 2013), listed in `SHAPES.toml`. S1.09's
  quote now reads it. The **shadow cost of cash** — what one more unit of liquidity is worth to the bank today, the
  marginal cost less the liquid assets' yield — is a liquidity fact S2.01's loan sales read.
- **Deposit rates** (BFL.4), on the bank's review days: the rate point per deposit kind moves by its management's
  speed toward the rate its funding need asks for — the loans it expects to write and the buffer it targets less its
  own funds, against the inflow its deposit-response outlook gives at each rate — and is pulled toward competitors'
  posted rates it can see. A bank short of funding therefore bids up. The form (Hannan and Berger, 1991, on deposit
  rates' adjustment) is listed in `SHAPES.toml`, retiring S1.09's placeholder rule.
- **The buffer** (BFL.5), on the bank's liquidity review days (weekly): for each funding class, the bank's outlook of
  net outflows over the horizon — its depositor mix by class, and the maturities of its term deposits and interbank
  borrowing that fall within the horizon, with the mean and width of their roll-over from its own history (VAL.6) —
  gives the probability that outflows exceed a buffer B; B is set where that probability times the cost of meeting a
  shortfall at the facility (the lending rate less the liquid assets' yield, plus the collateral it would tie up)
  equals the carry of holding B, scaled by its buffer appetite (PREFERENCE) — Poole's (1968) reserve demand, listed in
  `SHAPES.toml` — and, from S2.07, never below the supervisor's liquidity requirement (a `DeclaredLimit` from its
  register entry).
- **When short** (BFL.6), a lumpy decision on the liquidity review and when its day's position wakes it: the actions
  available, each with its marginal cost — borrow interbank (a quote), pledge free eligible assets at the facility,
  sell loans (S2.01), bid for deposits (a deposit-rate review now), shrink new lending (raising S1.09's required
  return and standards through the bank's liquidity fact), and at 8d draw the facility — taken cheapest first. The
  form, least cost first among a bank's liquidity actions (Cornett, McNutt, Strahan and Tehranian, 2011), is listed in
  `SHAPES.toml`.
- **Interbank term loans** (BFL.6, MMK.1, MKT.7) are loans (BNK.2) on the `if-credit` interbank line kind: a bank
  short of funding applies at 5c to the banks it can reach (BNK.6's application); each answers at the next 5c by
  S1.09's quote and decline in `sys-bnk` — its marginal cost of funds, its one `LoanAssessment` of the borrower bank's
  class (read from the borrower's published funding and capital figures and public events), the capital charge
  (S2.07) — declining when its liquidity fact shows no surplus beyond its buffer; the borrower takes the best quote at
  the 5c after, ties by lot (`BFL.interbank_lot`), and the loan settles at 7. The lender provisions and works it out
  by S2.01 like any loan. The quote's form is S1.09's, whose `SHAPES.toml` entry names banks among its borrower
  classes; the borrower's is the shortfall form above. Same-day shortfalls go to the facility.
- **The facility** (8d): a bank whose reserves after stage 7 are below its buffer's reserve part requests the
  difference at the lending facility against eligible collateral; one above places the excess at the deposit
  facility. It is the bank's own request (MKT.9).
- **Reserves** (BFL.8, BFL.11): a bank's reserve balance is its account at the central bank, moved only by settlement:
  the residue of everyone's payments.
- **Failure for liquidity** (BFL.10): at 8f a bank that cannot repay intraday credit after the facility has run has
  the shortfall recorded as an overdue claim of the central bank and its liquidity failure recorded, and goes to
  resolution by architecture §9.2's path (S2.08).
- **Depositors** (BFL.7), a lumpy decision of households and firms on their `bank_choice` review days and when woken:
  - **what it observes**: the banks' posted rates; each bank's published funding and capital figures (9d); and public
    events (OBS.3) of withdrawals, queues and failures, at the bank and at banks that look like it (declared
    similarity: region, size class and business kind);
  - these form, per (bank, method), a **public-series outlook** of the bank's safety — the chance the depositor puts
    on losing uninsured money there — computed once per method per day (S1.01), so no depositor's state is read; a
    surprise in it wakes the keys whose banking arrangement holds that bank, through architecture §7.3's wake pass;
  - the value of each alternative (keep, another bank, banknotes, bills at auction) is its rate less the expected loss
    on the balance above the insured limit (S2.08; the whole balance before it), less the switching cost in hours at
    its value of leisure, plus a taste per bank (REP.22, stream `BFL.bank_taste`);
  - members choose by multinomial logit. A switch is a **transfer instruction** of the movers' balances to the new
    bank, settled at 7; the movers' part is made at 7c's apply when it settles, carrying the new banking arrangement
    (a key attribute), and lands at 10b; a switch that fails makes no part. So a switch is one part, made once, and
    only if it happened.

  The form — deposit choice with insured and uninsured balances and switching costs (Egan, Hortaçsu and Matvos, 2017;
  Klemperer, 1987) — is listed in `SHAPES.toml`.
- **The banking arrangement** has one writer, `sys-bfl`: its bank-choice switches above, and, when a resolution's
  transfer has settled (S2.08), its 7e handler writing the new arrangement of the holders whose deposit rows the
  transfer moved — once per distinct key, with a remap per cell — which 10b re-keys in place. The resolution moves
  lines; the arrangement follows them.
- **Runs** (BFL.9) emerge: withdrawals take reserves, the bank's shortfall actions and dearer funding are published,
  surprises wake more depositors. Deposit insurance removes the insured's reason to move.
- **Streams**: `BFL.bank_taste`, `BFL.interbank_lot`.

**Records gained**:

| Record | Item | Bytes |
| --- | --- | --- |
| Household cell | review exposure and attention for `bank_choice` (8 + 8) | 16 (total **568**) |
| Firm cell | review exposure and attention rate for `bank_choice` (8 + 4) | 12 (total **452**) |
| Bank (kind table) | the liquidity facet: buffer target, surplus, shadow cost of cash, run-off outlooks by class, deposit-response outlook | about 200 per bank |

**Unit tests**
- `marginal_cost_of_funds_cheapest_source_plus_carry`: over a given list of declared sources, including one added.
- `buffer_poole_condition`: over given outflow outlooks and rates, B rises with the width and the facility's penalty,
  and never falls below a given requirement.
- `shortfall_action_order`.
- `insured_depositor_weighs_cover`: a balance within the limit carries no expected loss.
- `bank_choice_logit_with_switching_cost`.
- `buffer_reads_maturities`: interbank and term-deposit maturities within the horizon raise B.
- `switch_part_only_on_settlement`: over a given settled and a given failed transfer, one part and none.

**Live checks**
- `LC-2-13`: BFL.11 — every change of a bank's reserve balance is a leg of a settled instruction.
- `LC-2-14`: BFL.12 — stickiness by deposit class, the maturity gap and funding costs by bank are published; outflows
  after a failure are published by insured and uninsured balances and by similarity to the failed bank.
- `LC-2-36`: BFL.10 — every liquidity failure followed the facility's use at 8d and the 8f shortfall, and went to
  resolution.
- `LC-2-37`: depositors' bank choices read only posted rates, published figures and public events (Law 12, the
  read-trace).
- `LC-2-38`: BFL's Done when — on an experiment copy (N6, §0.3) whose declared intervention is a catastrophe
  destroying collateral in one bank's region (an endowment changed, never a withdrawal placed or a record falsified),
  the bank's losses are published through its own statements, and withdrawals follow at it and at banks like it, and
  stop where insurance covers them: against the untouched run at the same seed, uninsured balances leave the struck
  bank and its similar banks faster than others, and insured balances do not.

**Budget**:
- Memory: the household cell at 568 bytes, 11 MB more; the firm cell at 452 of 500.
- Time, an ordinary business day: depositors' reviews are "Occasion evaluations"; switches are about 9 k parts a day,
  made once at settlement, 9 k × 2.5 µs ÷ 3 ≈ 8 ms in "Parts" (≈ 36 ms at the measured 12.1 µs); the safety outlooks
  are part of 5a's public-series line; banks' decisions are "Institutions".
- **The bank-run day**, an event line in §13.2 beside the publication-day wake: the wake pass over 0.95 M hot records
  × 5 ns; up to 0.35 M woken cells (the large bank's depositors and those of banks like it) visited at 500 ns; up to
  50 k switchers' parts at 2.5 µs: (4.8 + 175 + 125) core-ms ÷ 3 ≈ **100 ms** (≈ 260 ms at the measured part cost).
- Counters, ratcheted: `phx_bfl.deposit_rate_changes`, `phx_bfl.interbank_loans`, `phx_bfl.facility_draws`,
  `phx_bfl.liquidity_failures`, `phx_bfl.bank_switches`, `phx_bfl.safety_wakes`, `phx_bfl.run_day_woken_visits`,
  `phx_bfl.run_day_parts`, `phx_pop.distinct_keys` (banking arrangements), `phx_pop.bytes_per_household_cell` (at
  568), `phx_pop.bytes_per_firm_cell` (at 452).

**Guards**: none new.

**Not allowed**:
- a single deposit type;
- unlimited, unpriced or uncollateralised central-bank credit;
- a bank whose funding cannot leave;
- a depositor reading another's account or a bank's private state;
- a switch made before its transfer settles;
- an interbank loan priced or provided for outside the lender's `LoanAssessment`;
- a withdrawal rate.

**Done when**
- [ ] Banks fund, price deposits, hold buffers, act when short and can fail for liquidity while solvent; depositors
  move.
- [ ] LC-2-13, LC-2-14 and LC-2-36 to LC-2-38 pass (LC-2-36 and LC-2-38 from S2.08, when resolution exists).
- [ ] Two reviews are done.

---

### S2.07 — `sys-bcp`: bank capital

**Status**: planned

**Clauses**:
- STATE: BCP.1, BCP.2.
- DECISION: BCP.3; BCP.4 *(part: retained earnings, and the decision to raise recorded against a placeholder naming
  EQY and CRD; issuance is S3.04 and S3.05)*.
- PROCESS: BCP.5, BCP.6.
- MEASURE: BCP.7.
- FORBID: BCP.8.
- PRIMITIVE: BCP.9.
- This step retires S1.09's placeholder naming BCP (a declared risk weight per loan kind as the capital charge).
- It introduces one placeholder naming EQY (S3.05): until the group fact exists, a bank's group is the bank alone.

**Architecture**: §4.1 (facets), §6.1 (5c, 9a, 9b, 9c, 9e), §9.2.

**Depends on**: S2.06.

**Goal**: capital is the equity account and the layers beneath it that absorb losses in order. The supervisor's
requirements are computed from each bank's own books. Each bank chooses its buffer above them from its own caution
and the cost of equity, and near the line restricts dividends, sheds risk-weighted assets and prices loans dearer,
which slows lending.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-banking/src/capital.rs` | layers as declared seniority on the bank's instruments and equity (BCP.1); requirement kinds; the rule handle `capital_charge` under each lender's declared regime (implemented by `sys-bcp`, read by `sys-bnk`'s quote); the bank's capital facet (ratios by requirement, target buffer, gap) |
| `crates/systems/sys-bcp/src/requirements.rs` | risk-weighted, leverage, large-exposure and liquidity ratios from the bank's books, and on its group's consolidated statement (S0.19) |
| `src/rules/{buffer,payout,shed,raise}.rs` | BCP.3, BCP.4 |
| `src/handlers/9b_ratios.rs` | ratios on reporting dates and on the bank's review days |
| `src/handlers/5c_capital.rs` | the buffer review and its consequences |
| `src/audit.rs` | the Layers family (BCP.8) |
| `data/<country>/BCP.toml` | requirements, risk weights by asset class, large-exposure limit, the liquidity requirement's run-off rates by funding class (POLICY of the supervisor); capital appetite and the dividend adjustment speed (PREFERENCE of management); review schedules and costs |
| `data/shared/SHAPES.toml` | the buffer, shedding and raising forms, with sources |

**Design**

- **Layers** (BCP.1): ordinary equity (the equity account, ACC.4), then contingent capital with its trigger ratio,
  then subordinated debt, then senior creditors and uninsured depositors, declared as seniority on the bank's
  instruments and lines. Stage 2's banks carry equity and the subordinated debt their opening sources give them, held
  by the opening holders (GEN.2); contingent capital arrives with CRD (S3.04). The layers absorb losses in order only
  in resolution (S2.08); before it, losses fall on equity (BCP.5).
- **Requirements** (BCP.2), computed at 9b from the bank's own books (ACC) on its reporting dates and its review days,
  after 9a's valuations and provisions (S2.01), so each ratio carries the day's provisions:
  - the risk-weighted ratio: equity over Σ exposures × the supervisor's risk weight for each asset class (a rule
    handle over the asset's declared class and, where the rule says, its loan-to-value from the lender's valuation
    and, from S3.10, its rating on a scale declared in `if-base`);
  - the leverage ratio: equity over total exposure;
  - the large-exposure test: exposure to each counterparty over capital, a `DeclaredLimit` that binds the bank's
    quotes to that counterparty;
  - the liquidity requirement: liquid assets after haircuts over the outflows its funding classes would run off at the
    supervisor's declared rates, a `DeclaredLimit` S2.06's buffer reads.
  - **On the group**: the supervisor's test at 9c (S2.08) applies the same pure ratio functions to the consolidated
    statement of the bank's group — `phx-acct`'s consolidation read (S0.19) over the members the group fact names —
    as well as to the bank's own, so capital a subsidiary uses (a dealer desk from S3.06) counts at its parent's
    group. Until S3.05 writes the group fact, the group is the bank alone (the placeholder naming EQY).
- **The capital charge** (retiring S1.09's placeholder), the rule handle, under the lender's declared regime (data:
  for a bank, the supervisor's requirements; for other lenders' forms, S3.08, the leverage limit in their funding
  covenants or mandate): the capital a loan consumes is its exposure × its risk weight × the lender's target ratio
  (requirement plus its chosen buffer); S1.09's quote multiplies it by the required return on capital (BNK.16).
- **The buffer** (BCP.3), a lumpy decision on the bank's capital review (monthly) and when a loss or a requirement
  change wakes it. Inputs: its own caution (capital appetite, PREFERENCE) and the cost of equity (its required return
  less the cost of the debt it would otherwise use), and its outlook of losses (mean and width of its loss outlook,
  S2.01). The target buffer is where the chance that losses over its horizon carry its ratio below the requirement,
  times the cost of a breach (the supervisor's declared consequences: restricted dividends, a demanded plan), equals
  the cost of holding the buffer. The form is the capital-buffer choice under costly breaches (Milne and Whalley,
  2001; Peura and Keppo, 2006), listed in `SHAPES.toml`.
- **Near the line** (BCP.3), read as the gap between its ratio and requirement plus target:
  - dividends: the bank's payout follows S2.03's partial-adjustment form within the gap: nothing is paid that would
    carry the ratio below requirement plus target, and nothing while a supervisor's restriction stands (S2.08);
  - shedding: it offers loans for sale (S2.01) until the gap closes, cheapest capital relief first — the loans whose
    sale frees the most risk-weighted assets per unit of value lost below their claim value — the form of banks
    meeting a capital gap by shrinking risk-weighted assets (Gropp, Mosk, Ongena and Wix, 2019), listed in
    `SHAPES.toml`;
  - pricing: the shadow cost of capital — the rise in expected breach cost per unit of risk-weighted assets — is added
    to its required return in S1.09's quote, so it lends dearer and less (Van den Heuvel, 2002, the bank capital
    channel).
- **Raising capital** (BCP.4): when the value of restoring the buffer exceeds the issue's cost at its own valuation —
  the dilution of existing owners when the market prices the bank below its own value (Myers and Majluf, 1984), the
  form listed in `SHAPES.toml` — the bank decides to issue; until EQY (S3.05) and CRD (S3.04) exist the decision is
  recorded, counted and unmet (a placeholder naming EQY and CRD). Capital rises only by retained income meanwhile
  (BCP.5).
- **Breach** (BCP.6): a ratio below its requirement is tested by the supervisor at 9c (S2.08), which issues the
  consequences.
- **Layers family** (BCP.8): in every resolution, write-downs follow the layers' order (checked from S2.08's records);
  every bank's capital is its equity account (ACC.10); every bank has its requirement computed on each reporting date.
- **Streams**: none.

**Unit tests**
- `ratios_from_books`: over given exposures, weights and equity, each ratio.
- `capital_charge_from_weight_and_target`: under a given bank regime and a given leverage-limit regime.
- `buffer_rises_with_loss_width_and_breach_cost`.
- `near_line_restricts_payout_and_prices_dearer`.
- `large_exposure_binds_quote`.

**Live checks**
- `LC-2-15`: BCP.7 — which requirement binds per bank, and payouts and lending near the line, are published; capital
  ratios over the cycle are published.
- `LC-2-39`: BCP.8 — no loss skipped a layer; every bank's capital equals its equity account.
- `LC-2-40`: every bank had every requirement computed from its own books on every reporting date; none is exempt.

**Budget**: ratios read a bank's loan lines' per-stage totals (S2.01) and stream only its exposures to individuals
for the large-exposure test, on its reporting dates, spread by phase, inside "Valuation, accounts, tests"; the rest is
"Institutions" and negligible. Counters, ratcheted: `phx_bcp.ratio_computations`, `phx_bcp.exposure_rows_streamed`,
`phx_bcp.near_line_days`, `phx_bcp.raise_unmet`.

**Guards**: none new.

**Not allowed**:
- a capital pot;
- a loss skipping a layer;
- an exempt bank;
- a bank's own risk model in place of the supervisor's weights;
- capital raised from nowhere.

**Done when**
- [ ] Losses change lending through each bank's own buffer decision.
- [ ] LC-2-15, LC-2-39 and LC-2-40 pass (LC-2-39's layer order from S2.08).
- [ ] Two reviews are done.

---

### S2.08 — `sys-sup`: supervision, deposit insurance, resolution, licensing and macroprudential limits

**Status**: planned

**Clauses**:
- STATE: SUP.1, SUP.2, SUP.3.
- DECISION: SUP.10.
- PROCESS: SUP.5, SUP.6; SUP.4 *(part: banks; insurers and clearing houses are S4.07)*; SUP.9 *(part: licensing
  banks; insurers, pension schemes, clearing houses and dealers are licensed as their systems arrive, and it
  completes at S4.07)*.
- INVARIANT: SUP.7.
- MEASURE: SUP.11 *(part: banks)*.
- FORBID: SUP.8.
- PRIMITIVE: SUP.12.

**Architecture**: §3.1 (decision-point homes), §4.4 (the split at a kink), §4.5 (the banking arrangement), §6.1
(2b, 5c, 6a, 7, 8d, 8f, 9c, 9e), §6.5 (pending legs), §9.1, §9.2, §13.2 (the resolution's D+1).

**Depends on**: S2.07.

**Goal**: named authorities in each country test banks on their reporting dates and act on breaches the next business
day, insure depositors up to a limit per person from a fund built by premiums, resolve a failed bank by architecture
§9.2's path so that no position vanishes and every bail-out has a named payer, license new banks their founders
capitalise, and set macroprudential limits from their own outlook.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-banking/src/resolution.rs` | the resolution's facts (the `closed` fact S0.17's pending legs read, with `sys-sup` its writer; valuation, write-downs), message kinds `BidInvitation`, `Bid`; the insurer's decision point `choose_paying_bank` (implemented by `sys-sup`); the treasury backstop line kind; the insurer's coverage order |
| `crates/interfaces/if-banking/src/decisions.rs` | adds `found_bank` (a founder's, any party's, implemented by `sys-sup`), which lives here because its types are the licence and the minimum capital (architecture §3.1) |
| `crates/interfaces/if-credit/src/decisions.rs` | adds `bid_for_failed_bank` (the acquirer's, implemented by `sys-bnk`), which lives here because it values loans under the bidder's `LoanAssessment` (PC-40) |
| `crates/systems/sys-sup/src/{supervisor,insurer,resolution,licensing,macroprudential}.rs` | the parties' declarations and rules |
| `src/handlers/9c_test.rs` | tests on reporting dates and the trigger, in one handler: consequences issued, due the next business day; below the point of non-viability, or after a liquidity failure at 8f, resolution triggered, the bank closed and bids invited — all applied at 9e |
| `src/handlers/2b_value.rs` | the book taken from D's statement; write-downs by layer; on a failed transfer, the next bid or the payout path |
| `src/handlers/5c_paying_bank.rs` | the insurer's choice of paying bank |
| `src/handlers/6a_select.rs` | the authority's least-cost selection among bids and the transfer's composite instruction |
| `src/handlers/5c_macro.rs`, `5c_found.rs`, `5c_license.rs` | limits, foundings and licences |
| `src/audit.rs` | the SUP.7 family; the insurer's fund reconciliation |
| `crates/systems/sys-bnk/src/rules/bid.rs`, `src/handlers/5c_bid.rs` | the acquirer's bid |
| `data/<country>/SUP.toml` | supervisory rules and reporting dates, consequences of each breach, the point of non-viability, licensing criteria and minimum capital, insurance limit per person and premium schedule by risk class, the backstop line's limit, the resolution fund's levy, resolution tools and the least-cost rule, the macroprudential reaction schedules (POLICY); review schedules; review costs in hours for bids, foundings, the choice of paying bank and the macroprudential review (TECHNOLOGY) |
| `data/<country>/gen/SUP.toml` | the authorities as parties, the insurer's opening fund, the resolution fund (ENDOWMENT) |
| `data/shared/SHAPES.toml` | the acquirer's bid form, the founder's form and the insurer's paying-bank form, with sources; the supervisor's reaction schedules and the least-cost rule are its declared POLICY |

**Design**

- **The parties** (SUP.1–SUP.3): a supervisor, a deposit insurer and a resolution authority per country, opened by
  `sys-sup`'s opening contribution as individuals of the agency kind, each with its accounts.
- **Tests** (SUP.4): at 9c on each bank's reporting dates, the supervisor reads S2.07's ratios. A breach issues its
  declared consequence as a demand due the next business day (TIME.7): a restriction on distributions (a fact S2.07's
  payout reads), a demanded capital plan (answered by the bank's buffer review at 5c), and below the point of
  non-viability, resolution. The test and the trigger are one handler, `9c_test`, whose results apply at 9e. The
  supervisor's **solvency fact** per bank is what the central bank's facility reads at 8d (S1.10).
- **Deposit insurance** (SUP.2): coverage is per legal person — for a household, the limit × the banking arrangement's
  adult holders; for a firm or any other legal person, the limit once (per member, for a firm cell) — over the
  depositor's balances at the bank in the declared coverage order (architecture §4.5). Premiums are charged quarterly
  per bank by the risk class its capital and funding put it in (POLICY schedule), as a demand issued at 9c and settled
  at the next business day's 2c. A treasury backstop line (a credit line from the treasury, its limit POLICY) is drawn
  when the fund is short.
- **Resolution** (SUP.5, SUP.6), exactly architecture §9.2:
  1. **D, 8f**: the bank cannot repay intraday credit; the shortfall is the central bank's overdue claim and its
     liquidity failure is recorded (S1.10, S2.06). Or **D, 9c**: the supervisor finds it insolvent or below the point
     of non-viability.
  2. **D, 9c** (`9c_test`): the authority triggers resolution and sends `BidInvitation`s to the eligible acquirers
     (banks licensed in the country whose capital after the purchase would meet their requirements), applied at 9e.
     The bank is **closed** from then until its transfer settles: its `closed` fact makes the ledger's 7a fix every
     leg to or from its customers as **pending** (S0.17): its customers' outgoing payments wait as pending on their
     rows, and payments to them as pending on the payer's row, excluded from the payer's funds.
  3. **D+1, 2b** (`2b_value`): the authority takes the book from the bank's **statement of D** (9b), whose loans the
     loan-book valuer valued at 9a over their per-stage totals (S2.01; a bank that failed at 8f has its lines valued
     and its statement drawn that day), so no loan row is valued again. The hole is written down through the layers in
     order (BCP.1) — equity, then contingent capital converted or written down, then subordinated debt, then senior
     debt as needed — as `Row` and equity legs applied at 2f. The authority also values what each creditor class would
     have received in a liquidation (S0.17's waterfall over the liquidation valuation) for the no-creditor-worse-off
     test.
  4. **D+1, 5c**: each invited bank decides `bid_for_failed_bank` (`sys-bnk`): its own value of the assets offered
     (their `loan_claim_value` under its own `LoanAssessment`, BNK.15) less the insured deposits it would assume and
     the cost of the capital they consume (S2.07), bidding that less its required return, or declining. The form — a
     first-price sealed bid from own valuation (Granja, Matvos and Seru, 2017, on failed-bank auctions) — is listed in
     `SHAPES.toml`. The same day the insurer decides `choose_paying_bank`: among the eligible banks, the one its own
     public-series outlook of banks' safety (S2.06's, read as depositors read it) puts safest, ties by lot
     (`SUP.paying_bank_lot`) — the form, insured deposits paid through the safest eligible bank acting as agent (FDIC,
     Resolutions Handbook, 2019), listed in `SHAPES.toml`.
  5. **D+1, 6a** (`6a_select`): the authority selects by its own **least-cost rule** (POLICY), not a market form: its
     reserve is the insurer's cost of paying the insured deposits out; the highest bid at or above the reserve wins
     and pays its bid, ties by lot (`SUP.bid_lot`); with none, the payout path. The handler writes the transfer's
     composite instruction.
  6. **D+1, 7**: the transfer settles as one instruction (SET.4). Each deposit row splits at a kink (S0.17's
     `SplitAtKink`): the insured amount and its pending payments move to the acquirer — or, on the payout path, to the
     paying bank the insurer chose — and the rest becomes a claim row on the estate; pending payments beyond the
     insured amount fail, visibly, against the estate claim (MON.5). The closed bank's reserve account and everything
     the transfer leaves pass to its estate (S0.17). The consideration is a leg: the acquirer takes the assets
     it bid for, and the estate or the insurer pays the difference to the insured deposits it assumed; the insurer
     becomes the estate's creditor (SUP.6), drawing its backstop if its fund is short. Compensation owed by the
     no-creditor-worse-off test is paid in the same instruction from the resolution fund, or the treasury where the
     fund is short: named payers (SUP.5, SUP.8).
  7. **D+2**: customers pay through their receiving bank, and their pending payments settle there. Their banking
     arrangements were rewritten by `sys-bfl` at D+1's 7e and re-keyed at 10b (S2.06), once per distinct key with a
     remap per cell; no cell splits, since the insured amount is computed per member and a cell's members share their
     per-member balance (REP.9).
  8. **A failed transfer**: if the transfer fails at D+1's 7b (a leg its payer cannot fund), nothing moves (SET.4) and
     the bank stays closed; at D+2's 2b the authority takes the next bid at or above its reserve, or the payout path,
     and writes the transfer again, settling at D+2's 7; customers pay through their receiving bank from D+3.

  The rest of the bank goes to an estate (S2.04): it sells the remaining loans (S2.01's sales) and pays by the law's
  order, with deposits and the insurer where the law prefers them.
- **SUP.7 family**: on each resolution, what the acquirer took, what the insurer paid, what the estate realised and
  what holders lost sum to the hole the valuation found; checked on the transfer instruction and completed when the
  estate ends.
- **Licensing** (SUP.9, part): a founder — any party whose named accounts can subscribe the minimum capital (a firm, a
  household, a group of them; funds from S3.07) — decides `found_bank` on its review when its value of the venture
  (VAL.8 `firm_value` over the spreads it can see: posted loan and deposit rates and the published returns of banks)
  beats its required return on the capital; it subscribes the minimum capital (POLICY) from named accounts; the
  supervisor licenses at 5c by its declared criteria, and the bank is created at the next day's 3c (FRM.16's founding
  path). The form is S1.03's founding comparison, listed in `SHAPES.toml`.
- **Macroprudential limits** (SUP.10), on the supervisor's review (quarterly), from its own outlook of the published
  credit and house-price series (S1.01's public-series outlooks), by its declared reaction schedule (POLICY: caps on
  loan-to-value and debt-to-income for new mortgages, a countercyclical buffer, sectoral risk weights, each piecewise
  linear in its outlook, after the Basel committee's guide, 2010). Each change is a policy value with an announcement
  and an effective day (architecture §4.6); each binds lenders as a `DeclaredLimit` read by S1.09's quote and decline
  and S2.07's requirements, and a binding is an event the lender sees.
- **Review costs**: a bid, a founding decision, the choice of paying bank and the macroprudential review each cost
  the decider's staff hours (TECHNOLOGY), counted per decision kind, paid in its staff capacity (§2.21).
- **Streams**: `SUP.bid_lot`, `SUP.paying_bank_lot`.

**Unit tests**
- `resolution_identity`: over a given book, bids and insured amounts, the four parts sum to the hole.
- `writedown_by_layer_order`.
- `insured_split_by_person`: limit × holders across two rows in coverage order.
- `no_creditor_worse_off`: a class whose resolution outcome is below its liquidation outcome is paid the difference.
- `least_cost_reserve_selects_or_declines`: the highest bid at or above the reserve pays its bid; none below it.
- `failed_transfer_takes_next_bid_or_payout`.
- `rekey_once_per_distinct_key`: over given cells and keys, one new key per distinct key and one remap per cell.
- `premium_by_risk_class`.

**Live checks**
- `LC-2-16`: SUP.7 holds in every resolution.
- `LC-2-17`: SUP.8 — no failed bank's positions vanish; every bail-out and every compensation names its payer.
- `LC-2-18`: SUP.11 (part) — failures, their clustering, the cost of each resolution and who bore it, the insurance
  fund through the run and how often macroprudential limits bind are published.
- `LC-2-41`: every resolution ran on D, D+1 and D+2 at the sub-steps above — D+2 and D+3 where its transfer failed
  once — and a closed bank made no payment of its own.
- `LC-2-42`: the insurance fund equals opening fund plus premiums plus recoveries plus backstop draws minus payouts;
  no insured payout to a person exceeded the limit.
- `LC-2-43`: every macroprudential change has an announcement and an effective day, and every binding is a recorded
  event of the lender it bound.

**Budget**:
- A resolution is a heavy day's event: for the largest bank at the design point, no loan row is valued (the book is
  D's statement); 0.5 M deposit rows split at 100 core-ns, 50 core-ms; about 20 k distinct keys re-keyed at 300
  core-ns, 6 core-ms; 0.25 M cells remapped at 80 core-ns, 20 core-ms: 76 core-ms ÷ 3 ≈ **25 ms** wall on D+1, an
  event line in §13.2 beside the publication-day wake.
- Tests, premiums and limits are "Institutions".
- Counters, ratcheted: `phx_sup.tests`, `phx_sup.breaches`, `phx_sup.resolutions`, `phx_sup.rows_split`,
  `phx_bfl.resolution_keys_remapped`, `phx_sup.transfers_failed`, `phx_sup.backstop_draws`, `phx_sup.licences`,
  `phx_sup.limit_bindings`.

**Guards**: none new.

**Not allowed**:
- insurance without a fund and a limit;
- a bail-out without a payer;
- positions vanishing;
- a failed bank that keeps paying;
- an acquirer assigned rather than bidding.

**Done when**
- [ ] A failing bank is resolved through named parties, and every one of its positions lands on a successor.
- [ ] LC-2-16 to LC-2-18 and LC-2-41 to LC-2-43 pass; LC-2-36, LC-2-38 and LC-2-39 now apply and pass.
- [ ] Two reviews are done.

---

### S2.09 — `sys-ene`: energy

**Status**: planned

**Clauses**:
- STATE: ENE.1, ENE.2, ENE.3, ENE.4.
- DECISION: ENE.7; ENE.5 *(part: offers from running costs and the fuel outlook; a generator's contracts come with
  S4.02, where it completes)*; ENE.6 *(part: retail suppliers and large consumers buying wholesale; buying under a
  contract comes with S4.02, where it completes)*.
- PROCESS: ENE.8, ENE.9, ENE.10.
- INVARIANT: ENE.11.
- MEASURE: ENE.12.
- FORBID: ENE.13.
- PRIMITIVE: ENE.14.
- This step retires S1.02's placeholders naming ENE (energy carriers as plain goods).

**Architecture**: §6.1 (3a, 4a, 5b, 5c, 6a, 6d, 7; electricity every day), §7.4 (indexed standing flows), §8, §9.1.

**Depends on**: S2.08.

**Goal**: electricity produced by named plants and priced every day per region by one call auction across each
country's grid, subject to its lines' capacities; fuels as commodities; weather-driven renewable output and heating
and cooling demand; retail tariffs; imbalances settled with the system operator; shortages as named, dated losses.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-energy/src/*.rs` | plant technologies (fuel-fired, hydro, nuclear, wind, solar, storage) as declared data; the power products per (region, day, block); the grid's lines (the power-line segments of S1.07's network); the system operator's facts; decision points `offer_power`, `bid_power`, `store` |
| `crates/systems/sys-ene/src/rules/{offer,bid,storage,shed}.rs` | ENE.5, ENE.6, storage, ENE.9's rule application |
| `src/pressure.rs` | the retail supplier's pressure fact read by `sys-frm`'s tariff review |
| `src/handlers/5b_offers.rs` | generators', storage's, suppliers' and large consumers' daily orders, a continuous decision at 5b, including non-business days (TIME.8) |
| `src/handlers/4a_dispatch.rs` | delivery: output realised, balancing, imbalances measured, load shed |
| `data/<country>/ENE.toml` | plant technologies, efficiencies, ramp limits, running costs, line losses (TECHNOLOGY); each region's climate (ENDOWMENT, S0.13); blocks, load-shedding priorities, imbalance pricing, energy policy and regulated returns (POLICY); energy needs per degree-day by household composition (TECHNOLOGY of living) |
| `data/<country>/gen/ENE.toml` | the opening fleet and grid owners, the system operator, suppliers (ENDOWMENT) |
| `data/shared/SHAPES.toml` | the offer, bid and storage forms, with sources |

**Design**

- **Plants** (ENE.1): individual capital units (CAP) of generator firms, each with its technology, capacity,
  efficiency, fuel, ramp limit and running cost. Wind, solar and hydro availability is the day's weather per region
  (S0.13) × capacity; plant failures are CAP's hazards on held units (S1.04).
- **Fuels** (ENE.3) are GDS commodities (S1.05): extracted, stored, shipped and traded at their places; plants buy
  them as inputs by GDS.5.
- **The grid** (ENE.2, GEO.4): the power-line segments of each country's network with capacities and losses; a named
  system operator per country, the grid's owner of record where GEN gives no other.
- **Energy as an input and a good** (ENE.4): ways use it in physical units (TEC.2); households buy it as a consumption
  category whose need is per degree-day (TECHNOLOGY of living). Its standing flow is an **indexed** flow (S0.17,
  S0.23): a rate per degree-day, whose day's amount is the rate × the region's degree-days that day in the payer pass
  and in the group aggregate, so the weather moves demand without touching any cell; its kink days are booked at the
  climate's envelope of degree-days and re-checked on the day.
- **Generators' offers** (ENE.5), a continuous decision every day for the next day, per plant and block: a step curve
  at the plant's avoidable cost — fuel at its own fuel-price outlook × heat rate, plus variable costs — in
  ramp-feasible quantities; an inflexible plant offers the output it cannot stop at minus the cost of stopping and
  restarting, and a subsidised plant at minus its subsidy, so offers may be negative. The form — offers at own
  avoidable cost (Wolak, 2000; Green and Newbery, 1992, for the supply-function view) — is listed in `SHAPES.toml`.
- **Storage**: buys in blocks where the price it expects is below its own outlook of the dearer block's price net of
  round-trip losses, and offers what it holds where above, within its capacity (Sioshansi, Denholm, Jenkin and Weiss,
  2009), listed in `SHAPES.toml`. Electricity is held only by storage units (PC-42).
- **Demand bids** (ENE.6): a retail supplier bids its outlook of its customers' consumption for the next day (its own
  outlook of the degree-days and of its customers' rates), price-taking up to its value of serving (the tariff it
  charges, less its costs); a large consumer bids its production's need up to the energy's value in use (GDS.5's
  form).
- **The wholesale market** (ENE.8), at 6a every day, for delivery the next day, per country's grid and block: one
  coupled call (declared by S0.18), maximising the traded surplus over the offer and bid steps subject to the lines'
  capacities, solved exactly as a min-cost flow; each region's price is its node's potential, so regions with no
  binding line between them share a price and a congested line separates them. Ties go by `ENE.clear_lot`. Line
  losses are not in the auction: the system operator buys them at delivery (below). Trades are recorded at 6d and, on
  non-business days, settle as pending on the next business day (TIME.8).
- **Retail tariffs** (ENE.6): fixed or variable, the supplier's posted points written by `sys-frm`'s price review
  (S1.03) over its unit cost — its wholesale price outlook — with the pressure `sys-ene` supplies (its customers'
  consumption against its expectation); the supplier bears the difference between tariff and wholesale.
- **Delivery and imbalance** (ENE.10), at 4a of the delivery day, after the day's weather (3a): plants produce their
  accepted quantities as far as their availability allows; the system operator balances the differences by calling the
  remaining offer steps up or the accepted ones down, in merit order, and buys the lines' losses (TECHNOLOGY per line
  and flow) the same way, paying for them from its network charges; each party whose delivery or take differs from its
  position pays or is paid the day's imbalance price (the marginal balancing step, POLICY of market design), settled
  at 7.
- **Shortage** (ENE.9): when a region's supply with imports cannot meet demand, the operator sheds load by the
  country's declared priorities (POLICY), within a class by lot (`ENE.shed_lot`): the consumers cut off are named —
  for cells, members drawn from the region's consumers by count (REP.23) — and a firm's production that needed the
  power is lost that day, an event with its units, and a household's energy consumption goes without, recorded (HH.4).
  The shed draw records, per row hit, the members cut off that day, and the day's amount of their indexed energy flow
  counts only the members supplied: no payment runs without its energy leg.
- **Investment** (ENE.7): generator firms invest in plants and storage by CAP.3's form (S1.04) over their own price
  outlooks and the energy policy's terms; grid owners build lines by the same form at the regulated return (POLICY).
- **ENE.11 family**: per region and day, produced plus imported equals consumed plus exported plus lost plus stored;
  no line carries more than its capacity.
- **Streams**: `ENE.clear_lot`, `ENE.shed_lot`.

**Unit tests** (the coupled call's own are S0.18's)
- `negative_price_when_inflexible_exceeds_demand`.
- `offer_at_avoidable_cost`.
- `storage_buys_low_sells_high_net_of_losses`.
- `degree_day_flow_amount`: the day's amount from a rate and degree-days.
- `shed_by_rule_names_losers`.
- `shed_members_not_charged`: the day's flow amount counts only the members supplied.
- `losses_bought_at_balancing`: over a given flow, the operator's purchase equals the lines' losses.

**Live checks**
- `LC-2-19`: ENE.11 — per region and day, the balance holds and no line is over capacity.
- `LC-2-20`: ENE.12 — spikes against spare capacity, the effect of renewable output on prices and their volatility,
  and fuel and power shocks reaching producer prices before consumer prices are published.
- `LC-2-44`: every shed load names its consumers and dated loss; every imbalance is settled with the system operator.
- `LC-2-45`: ENE's Done when — on an experiment copy (N6, §0.3) whose declared intervention cuts a fuel deposit's
  output (an endowment changed), each series — the fuel's price, power prices, producer prices, consumer prices —
  has a **first day** on which it exceeds the untouched run at the same seed by more than twice the series' own
  standard deviation of daily changes on the untouched run over the year before. It passes when those days fall in
  that order, each on or after the one before.

**Budget**: one coupled auction per country's grid per block per day (12, 8 and 5 regions), ≤ 5 ms in all (S0.18),
in "Institutions, financial markets"; offers about 2 000 plants a day; degree-day amounts inside the payer pass's unit
cost; kink-day re-checks at the envelope counted. Counters, ratcheted: `phx_ene.auctions`, `phx_ene.offers`,
`phx_ene.congested_lines`, `phx_ene.shed_mwh`, `phx_ene.imbalance_legs`, `phx_ene.envelope_rechecks`.

**Guards**: PC-42: a product declared not storable has no holding except on a unit kind declared as storage
(an assembly refusal and a `phx-check` rule over declarations).

**Not allowed**:
- electricity stored outside storage plants;
- a price from a formula;
- an outage without a named cause;
- a grid without an owner;
- a cell visited because the weather changed.

**Done when**
- [ ] Power is produced, priced, carried and rationed; a fuel shock reaches consumer prices through producers' costs.
- [ ] LC-2-19, LC-2-20, LC-2-44 and LC-2-45 pass.
- [ ] PC-42 is registered.
- [ ] Two reviews are done.

---

### S2.10 — The credit bureau and filed accounts

**Status**: planned

**Clauses**:
- STATE: BNK.21; RAT.8 *(part: filed accounts and the registry; the rules that refer to them grow with S3.10)*; RAT.2
  *(part: the filing calendar's spread; listed companies' reports arrive with S3.10)*.
- This step retires S1.09's limit of a lender's assessment to its own lines' records (a placeholder naming BNK's
  bureau), and extends S2.02's terms to read filed accounts.

**Architecture**: §3.1 (the filed accounts' home), §4.9 (records and audiences), §6.1 (5c, 7, 9d), §7.3 (the
key-clock reason).

**Depends on**: S2.09.

**Goal**: lenders report every borrower's contracts, payments and defaults to a named bureau as its law requires, and
buy a borrower's record when it applies; for members of cells the record is the credit-record stage in their key.
Every incorporated firm files annual accounts with its country's registry within the legal window, on a day it
chooses; filings are public after the registry's lag, and lenders and suppliers read them.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/bureau.rs` | the bureau's record kinds (audience: the bureau, then lenders who buy); the reporting calendar; `BoughtRecord`, a token only the purchase creates (PC-43); the credit-record stage's steps and clocks |
| `crates/interfaces/if-firm/src/filings.rs` | the filed-accounts record kind (public after the registry's lag), abbreviated as the law declares; it is company law's, so it lives with the firm (architecture §3.1), below every crate that reads it |
| `crates/systems/sys-bnk/src/bureau.rs` | the bureau party's contribution; reports on the calendar; the stage written for cells |
| `crates/systems/sys-bnk/src/rules/assess.rs` | extended: classes read bought records and filed accounts |
| `crates/systems/sys-rat/src/filing.rs` | the filing decision and the registry's records |
| `crates/systems/sys-tcr/src/rules/terms.rs` | extended: buyer classes read filed accounts |
| `data/<country>/BNK.toml`, `RAT.toml` | credit-reporting law: what is reported, how often, the record's horizon by event, the fee (POLICY); company law: the filing window, abbreviation, late-filing fines, the registry's lag, the accounting reference date (POLICY); preparation lead time and filing cost in hours (TECHNOLOGY) |
| `data/<country>/gen/{BNK,RAT}.toml` | the bureau and the registry as parties; the opening records from the opening history (GEN.5) |
| `data/shared/SHAPES.toml` | the filing-day form, with its source |

**Design**

- **The bureau** (BNK.21): a named party per country (a firm individual), with its accounts and fees.
- **Reporting**: on the law's reporting dates (monthly), each lender's report is a record per borrower line: balances,
  days in arrears, defaults and write-offs, read from its own rows' payment records (architecture §4.5) and written
  at 9d with audience the bureau.
- **The credit-record stage** (cells): a key attribute of the household and firm kinds (S0.25), written by `sys-bnk`'s
  bureau from the reports: members on a reported row move to the stage its record gives (arrears, default, insolvency
  from S2.11). The stage carries its clock — months until the record's horizon ends (POLICY by event) — so members
  reaching the horizon return to a clean stage on the day it ends, a re-key in place on S0.23's shared key-clock
  reason (REP.19), so the clock adds no agenda reason. Members whose stage changes differ in key and split.
- **Buying a record**: when a lender answers an application (S1.09's 5c), it buys the applicant's record: for a cell,
  its stage (in the key the application discloses); for an individual, the bureau's record of all its reported
  contracts. The purchase writes a fee leg to the bureau, settled at 7, and returns a `BoughtRecord`; the assessment
  reads another lender's data only through that token (PC-43). Its class then counts the stage and other lenders'
  arrears.
- **Filed accounts** (RAT.8): each incorporated firm's fiscal year ends on its accounting reference date (company
  law: the founding month's anniversary where the firm chose none; a key attribute of small-firm cells). Its annual
  statement (ACC.9) is ready after the preparation lead time; it files on a day it chooses within the window, decided
  on its review days: at its first review after the statement is ready when its result beats last year's, and at its
  last review before the deadline otherwise.
  The form — bad news filed late (Givoly and Palmon, 1982) — is listed in `SHAPES.toml`; it spreads filings across
  the calendar (N8.9). Missing the deadline is a recorded breach with the law's fine.
- **The registry** publishes each filing after its lag as a public record, abbreviated as the law declares (assets,
  liabilities, equity, turnover, profit, staff, cash). A small-firm cell's members are identical, so a cell files one
  record with its count.
- **Readers**: S1.09's assessment classes individual firms and small-firm cells by their filed ratios in steps;
  S2.02's terms class buyers the same way. A filing is never read before its publication day (Law 12).
- **Streams**: none.

**Unit tests**
- `record_stage_moves_with_reports`.
- `stage_clock_expires_at_horizon`.
- `filing_day_rule`: good results early, bad results late, both within the window.
- `bought_record_only_by_purchase` (compile-fail, PC-43).

**Live checks**
- `LC-2-21`: Law 12 — lenders' assessments read only their own lines, records they bought and public filings (the
  read-trace).
- `LC-2-46`: every incorporated firm filed within its window or its breach is recorded; filings are spread across the
  calendar, their count per day published.
- `LC-2-47`: every member's credit-record stage matches its lines' reported records, and every record ends at its
  horizon.

**Budget**:
- Memory: filed records kept two years, about 0.27 M a year of about 88 bytes, about 48 MB in §13.1's public records
  line; bureau records of individuals are small.
- Time: reports stream lenders' rows on the reporting date, spread by lender phase, inside "Valuation, accounts,
  tests"; stage changes are parts; filings are occasion evaluations.
- Counters, ratcheted: `phx_bnk.bureau_reports`, `phx_bnk.records_bought`, `phx_rat.filings`,
  `phx_rat.late_filings`, `phx_pop.stage_rekeys`, `phx_pop.distinct_keys` (the credit-record stage).

**Guards**: PC-43: another lender's reported data reaches an assessment only through `BoughtRecord` (compile level).

**Not allowed**:
- one assessment shared by lenders;
- a filing read before its date;
- a record kept past its horizon;
- a credit score that is not a record.

**Done when**
- [ ] The bureau and filings work, and lenders and suppliers read them.
- [ ] LC-2-21, LC-2-46 and LC-2-47 pass.
- [ ] PC-43 is registered.
- [ ] Two reviews are done.

---

### S2.11 — `sys-hh`: arrears actions and personal insolvency

**Status**: planned

**Clauses**:
- PROCESS: HH.13 *(completes it: default's consequences — collection, repossession and the credit record — with
  S2.01, S2.05 and S2.10)*, HH.14, HH.21.
- This step retires S2.01's placeholder answer to a restructuring offer for households (naming HH).

**Architecture**: §3.1 (decision-point homes), §4.3 (levies and follow-ons), §6.1 (2e, 3c, 5c, 7), §7.3 (needs,
the key-clock reason), §9.1.

**Depends on**: S2.10.

**Goal**: a household in arrears acts, choosing among its actions by what each is worth to it; a household that
cannot pay may enter, or its creditors may put it into, its country's personal insolvency procedure: its assets
beyond the law's exemptions are sold through an estate and paid in the law's order, part of its income above an
allowance is paid through a trustee to its creditors for the declared period, the rest of its debts are discharged at
the period's end, and the bureau keeps the record for the declared time.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/decisions.rs` | adds the household's `arrears_action` and `file_insolvency` (need occasions), and its rule for S2.03's `answer_restructuring`; they live here because they read S2.01's notices and offers and S2.02's collection notices (architecture §3.1) |
| `crates/interfaces/if-pop/src/insolvency.rs` | the personal insolvency law as declared data: entry, exemptions by asset class, the income allowance, the period, the order of claims, the trustee (a named party per country) |
| `crates/systems/sys-hh/src/rules/{arrears,file,answer}.rs` | HH.14, HH.21's entry, the answer to a lender's offer |
| `crates/systems/sys-hh/src/insolvency.rs` | the procedure: the estate of the assets, the claim line, the income levy and its follow-on through the trustee, discharge |
| `crates/systems/sys-bnk/src/rules/workout.rs` | the option set gains the creditor's petition for unsecured household debt (data) |
| `data/<country>/HH.toml` | the procedure (POLICY: personal insolvency law, with its fees); filing cost in hours (TECHNOLOGY); taste distributions over arrears actions (PREFERENCE) |
| `data/shared/SHAPES.toml` | the arrears-action, filing and answering forms, with sources |

**Design**

- **Arrears actions** (HH.14), a lumpy decision on the need occasion an `ArrearsNotice` or `CollectionNotice` gives
  the members on the row. The actions, each with the cash it raises and what it costs the household:
  - cut spending (a wake of the spending rule, S1.12, with the arrears among its dues);
  - draw savings (term deposits and bills to its current account, S1.12's holding);
  - sell assets (holdings; a dwelling by listing it, S2.05);
  - borrow elsewhere (S2.05's `borrow`, as its record allows);
  - move to a cheaper dwelling (a need occasion for `where_to_live`);
  - send another adult to work (a wake of that adult's participation and hours decision, S1.12).

  Each action's value is the utility it keeps against the cash it raises, with a taste per member and action
  (REP.22, stream `HH.arrears_taste`); members choose by multinomial logit and those who differ split. The form — the
  responses of households in financial distress (Sullivan, 2008), as a discrete choice — is listed in `SHAPES.toml`.
- **A lender's offer** (`answer_restructuring`, the household's rule): members accept a restructuring or extension
  when the value of the new terms, by the buffer-stock rule, beats keeping the old in arrears — the form, S1.12's
  buffer-stock comparison with and without the new terms (Carroll, 1997), listed in `SHAPES.toml`; this retires
  S2.01's placeholder for households.
- **Entry** (HH.21), by the household's own decision or a creditor's petition:
  - `file_insolvency`, a lumpy decision on the need occasion of a default or a demand: members file when the debts
    discharged less the non-exempt assets they would lose, the income levy over the period, the fees and the value of
    the credit offers the record would cost them exceeds zero. The form — the financial benefit of filing (Fay, Hurst
    and White, 2002) — is listed in `SHAPES.toml`; the costs are the law's (POLICY) and hours (TECHNOLOGY);
  - a creditor's petition: an option of the lender's workout (S2.01) for unsecured household debt, valued like
    enforcement.
- **The procedure**: at 2e of the entry day an **insolvency estate** opens — an individual of the estate kind, one
  per (part, occasion) as S2.04's — for the members of one row who entered on one occasion, holding their count on
  every line it succeeds to, since their claims and assets are identical. Their non-exempt assets pass to it by line
  transfer; exempt assets (by class, up to the law's amounts, per member) stay with the household. It sells what it
  holds by S2.04's administration, pays by the law's order through the waterfall, settled at 7, and **ends** once that
  distribution settles. What the creditors are still owed stays on a **claim line** between them and the members in
  the procedure, its stay carried by S0.17's procedure-line terms.
- **The income levy**: for the law's period, wages reaching the members in the procedure carry a levy (architecture
  §4.3): the amount above the law's allowance per member, computed per member and times the count, remitted by the
  employer to the country's **trustee** (a named party, the law's official receiver), its allowance a registered kink.
  Its **follow-on**, written by `sys-hh` as the payee's system, pays what the trustee received to the claim line's
  creditor side by the law's order, settled at 7. The members carry a key attribute "in procedure" with its clock to
  discharge (REP.19), on S0.23's shared key-clock reason.
- **Discharge**: on the clock's last day the claim line's unpaid claims are discharged — the creditors write them off
  at 2e (S2.01) — and the members re-key. The bureau's stage records the procedure for its horizon (S2.10).
- **Streams**: `HH.arrears_taste`.

**Unit tests**
- `arrears_action_values`: over given cash needs and costs, the shares by action.
- `file_benefit_components`.
- `exemptions_respected`: per member, by class, up to the law's amounts.
- `income_levy_above_allowance_per_member`.
- `trustee_follow_on_pays_claim_line_in_order`.
- `discharge_after_period`.

**Live checks**
- `LC-2-22`: every personal insolvency has its entry (filing or petition), its estate, its sales, the estate's end,
  its levy, the trustee's payments to the claim line and its discharge, on the law's dates.
- `LC-2-48`: HH.13 — every household default traces to its arrears occasion, the actions its members took, and the
  consequences its contract gave: collection, repossession, the record.

**Budget**:
- Memory: about 1 000 entries a day, grouped by occasion into about 70 estates a day; each ends once its assets are
  sold and distributed, about 45 days, so about 70 × 45 ≈ 3 k are open (Little's law), at 512 bytes, 1.6 MB in
  §13.1's estates line (S2.04's 60 k include them). The claim lines through the period are lines with rows in the
  members' and creditors' arenas.
- Time: arrears and filing occasions are "Occasion evaluations"; the levy is inside the settlement stream's per-member
  levy cost; entries and discharges are parts.
- Counters, ratcheted: `phx_hh.arrears_actions` by kind, `phx_hh.insolvency_entries` by opener,
  `phx_hh.discharges`, `phx_hh.trustee_payments`, `phx_est.estates_open` and `phx_est.mean_life_days` (insolvency
  estates), `phx_pop.distinct_keys` (the procedure clock).

**Guards**: none new.

**Not allowed**:
- debts that vanish without the procedure;
- a default drawn;
- exemptions applied to a cell's total instead of per member;
- a record without a horizon.

**Done when**
- [ ] Households in arrears act, and personal insolvency runs from entry to discharge.
- [ ] LC-2-22 and LC-2-48 pass.
- [ ] Two reviews are done.

---

### S2.12 — The Stage 2 gate

**Status**: planned

**Clauses**: BNK.13; PTY.12 *(part: the ladder and the reference comparison at Stage 2)*; REP.18 *(part: Stage 2's
RESOLUTION settings, taste distributions and review costs declared and measured)*; N8 *(the budget at Stage 2)*; N2;
the Stage 2 exit.

**Architecture**: §7.11, §13, §14.5, §14.6, §14.7.

**Depends on**: S2.11.

**Goal**: judge the Stage 2 exit and the budget on measured numbers, against criteria fixed before the run.

**Files**

| File | Purpose |
| --- | --- |
| `perf/device/S2.12-*.json`, `perf/measure/S2.12-*.json` | the device report and the measurements |
| `data/shared/READS.toml` | extended at the start of S2.01, and frozen before the first Stage 2 comparison, with Stage 2's reads |
| `perf/compare/S2.12-*.json`, `perf/ladder/S2.12-*.json` | the comparison and the ladder |

**Design**:
- **The reads** added (N8.5): default counts and rates by borrower kind and class; recoveries by collateral class;
  firm exits by cause (closure, default of payment, balance-sheet insolvency) and restructurings; days of payment;
  house prices, rents, transactions and time on market by region; moves between regions; bank funding costs and
  deposit flows by class; insolvency entries; wholesale power prices by region and block; and per-person reads —
  arrears spells, time from default to discharge, and tenure transitions over a year.
- **The reference** is re-estimated with Stage 2's state before the run (architecture §7.11) and run for five seeds; a
  size beyond the owner's machine is raised with the owner before the run. The play resolution runs twenty seeds.
- **The ladder**: at least three rungs, as S1.16.
- **The device run**: the settled Stage 2 world, a 30-minute soak, then a simulated year.
- **The decades run**: thirty simulated years on the weekly job, with the liveness reads.
- **Pass criteria**, fixed here before the run, as S1.16's: the median turn ≤ 1 000 ms and the worst ≤ 2 000 ms over
  the settled year; peak `VmHWM` and PSS ≤ 4.5 GB; a full save ≤ 5 s and an increment ≤ 1 s; the latest complete
  save and the one being written together ≤ 4 GB on the device's storage (N8.4); every read within
  Appendix E 30's band beyond the reference's seed spread at the play resolution and at each rung above it; the exit's
  reads below; §13's 10% headroom reported, a pass without it recorded as a finding.
- **A miss** is a finding; N8.7's remedies apply in order, then the owner decides (N8.7, N8.8). Stage 3 does not start
  until the budget in force is met.
- Architecture §13 is rewritten with the measured numbers.

**Unit tests**: none.

**Live checks**
- `LC-2-23`: L1 — for a sample of defaults in the run, the chain is traced end to end in the events: the missed
  payment, the arrears, the default, the workout or enforcement, the sale and its price, and the loss on each named
  holder in the order of its claim (N4's liveness form; the test is S7.02).
- `LC-2-49`: the exit — on the gate run, borrowers' own cash failures produced defaults, estates, losses on named
  holders and housing foreclosures; a bank failed for liquidity and one for solvency and each was resolved, either in
  the run or, where none did, on an experiment copy (N6, §0.3) whose declared intervention changes only a primitive or
  an endowment — a catastrophe destroying collateral in the bank's region, a shock to a primitive — never a
  withdrawal placed for a party, a falsified record or a changed rule.
- `LC-2-50`: N2 over the decades run — each of defaults, foreclosures, restructurings, estates opened, bank switches,
  firm foundings and bank foundings counts at least one in every one of the thirty years; money stuck on ended parties
  is zero at every close; and no stock grows without a named cause: a stock whose ratio to nominal output rises in
  each of ten consecutive years fails unless the flows that feed it, read from the ledger, account for the rise.
- `LC-2-51`: BNK.13 — declined applications per bank, standards against each bank's capital and funding, the
  pass-through of the facility rates to loan rates with its lag, and which constraint binds per bank over time are
  published.

**Budget**: this is the budget's gate.

**Guards**: none.

**Not allowed**: tuning; a gate off the device; reads chosen after seeing the comparison; an exit shown by an
intervention that changes a rule.

**Done when**
- [ ] The device report, the comparison over the declared seeds and the ladder are committed.
- [ ] Every pass criterion holds, or the owner's decision under N8.7 is recorded in §12 and the budget then in force
  is met.
- [ ] LC-2-23 and LC-2-49 to LC-2-51 pass.
- [ ] Architecture §13 is updated with measured numbers.
- [ ] Two reviews are done.

---

## 6. Stage 3 — Money and capital markets

**Exit** (spec Part O):
- The policy rate reaches loan rates, asset prices and investment through markets.
- A margin spiral and a fund run can happen.
- The budget and the reference comparison hold at the gate (S3.11).

**Decision rules** follow §2.21, and these conventions hold for every step of the stage:
- **Orders are schedules from values** (MKT.9, MKT.16). An order in a financial market is its poster's schedule: the
  investor schedule of S3.03 (a pure function in `phx-val`) unless the step names another form. Every limit is the
  poster's own; nobody posts "at market", and no order is placed for a party by anything but its own decision. The
  one stated exception is dealing in an open-ended fund at its next value (MKT.8, MKT.16; S3.07).
- **A bank's claims are valued by `sys-bnk`** (PC-40, architecture §3.4). Every price or value a bank puts on a claim
  on its book — an interbank or repo edge, a bill or bond it bids for or holds, paper it may underwrite, a
  syndicate share, a vote on an exchange offer — comes from its one `LoanAssessment` of the obligor, through
  `sys-bnk`'s rules or the rule handle `loan_claim_value`. Other parties value claims by their own outlooks
  through `phx-val`.
- **Decision points live where their types are** (architecture §3.1): a decision whose types are bonds, shares, fund
  units or orders on them is `if-securities`'; one whose types are loan or interbank terms or a `LoanAssessment` is
  `if-credit`'s; one that reads the corridor, the tenders or the collateral framework — a bank's `fund_position` —
  is `if-state`'s, as the central bank's and the treasury's are. Each rule is registered by the system that owns
  its decider kind's decision, except that a lender valuing a claim on its book does so through `sys-bnk` (§3.4).
- **Holdings move with their trades.** A cell's participation per asset class is a key attribute and its holdings
  are rows with a member count (S3.05). Members whose fill changes their participation, or leaves them at a step
  other than their row's holders', make their part at the fill (6d), pinned by the trade's commitment until it
  settles (architecture §4.2), and are re-keyed in place when it settles at 7c; members who enter at the holders'
  step join the row without a part; an unfilled order makes none.
- **Instrument outlooks are registered** (S1.01). Public-series outlooks of instrument prices are computed per
  (method, instrument) only for pairs some holder or candidate list registers, registered at applies by keyed
  reduction, and at 5a only on a day the instrument has a new print. Each such pair's value is computed there once
  and read by every cell using the method. Claim values are closed-form sums over the terms' cash flows; an
  untraded bond is discounted by the day's discount-factor table of its curve, built at 6c (S3.03). An individual's
  own outlook of an instrument is its method's plus its own deviation, caught up in O(1) when it reads.
- **Publishers are firms.** The curve publisher (S3.03), the pricing service (S3.06), benchmark administrators and
  index publishers (S3.09), rating agencies and banks' analyst units (S3.10) are large firms with a publisher facet
  (architecture §4.1), opened by GEN, each a party reading only public records and what it bought. A dealer is a
  party of the dealer form with its own equity (S3.06).
- **Where things happen** (architecture §6.1). Institutions decide at 5c; markets meet at 6a on business days, funds'
  dealing among them; marks and fixings at 6b; the curve and the valuation inputs derived from the day's fixings at
  6c; matches become instructions at 6d; trades settle on their market's convention (MKT.21) at stage 7, as
  commitments until then (REG.10), their new rows written by `Row` legs (S0.15); the money market and the central
  bank's tenders post at 8a, meet in one linked call at 8b and settle at 8c; facilities and the treasury's direct
  borrowing at 8d; valuations at 9a; funds' values, statements and the group fact at 9b; each test at 9c is one
  handler with the call, demand or trigger it issues, reading consolidated statements as a pure read, applied at 9e
  and due the next business day (TIME.7); publications at 9d. No financial market meets on a non-business day
  (TIME.8).
- **Institutions pay for reviews** in their staff's hours that day, which their other work then cannot use (§2.21).
  Their decisions run on declared schedules and on wakes; they carry no attention positions.

**Placeholders retired in this stage**, each by the step named:

| Placeholder | Introduced | Retired by |
| --- | --- | --- |
| The fixed policy rate and corridor (naming CB) | S1.10 | S3.02 |
| The declared list of eligible collateral with haircuts (naming CB) | S1.10 | S3.02 |
| The treasury's funding rule and its rule for when cash runs short (naming TRS) | S1.11 | S3.03 |
| A bank's decision to raise capital recorded and unmet (naming EQY and CRD) | S2.07 | S3.04 (debt), S3.05 (shares) |
| A bank's group is the bank alone, with no group fact (naming EQY) | S2.07 | S3.05 |
| Estates' securities with no market wait (naming EQY) | S2.04 | S3.05 (shares), S3.06 (bonds) |
| The named banks as primary dealers; bills only at auction and held to maturity (naming DLR) | S1.11, S1.12 | S3.06 |
| A floating reference's reset reads its series' last published value (naming IDX) | S0.14 | S3.09 |

**Placeholders this stage introduces**, each naming its retirer:

| Placeholder | Introduced | Retired by |
| --- | --- | --- |
| The central bank's collateral eligibility reads no rating (naming RAT) | S3.02 | S3.10 |
| Listing reads a firm's filed accounts as its reports (naming RAT) | S3.05 | S3.10 |

S1.11's deferral of discretionary purchases (naming SOC) stays until S5.02.

**Representation.** These choices keep the stage's counts down, each in its step and for architecture §18:
- a cell's participation per asset class (shares, bonds, fund units) is three bits of its key, so its distinct keys
  grow at most 8× per base key, measured at S3.05 on the ladder; its holdings are rows with a member count, each
  row's per-member quantity a position with steps, and income reaches holding members as pooled flows (S3.05);
- each country's money market and the central bank's tenders are one linked call a business day on S0.18's network
  solver, warm-started from the last basis, with borrowers who bid nothing dropped and schedules compressed to their
  breakpoints (S3.01, S3.02);
- instrument outlooks and values are computed per registered (method, instrument) pair on days with a new print and
  shared by every cell using the method; claim values are closed-form, over a discount-factor table per curve and
  day for untraded bonds (above);
- a shareholder vote is one pass over the instrument's holder list with a pure rule, so an abstaining cell costs one
  read (S3.05).

**The stage's budget ledger**, against architecture §13 as it stands through Stage 2 (a median weekday of 1 082 ms and
a peak of 4 296 MB, F-005). Wall time is core time ÷ 3 (§13.2), except the linked call, which runs on one core per
country with the countries in parallel and so counts undivided. Each step's **Budget** names its counts and the
counters that ratchet them.

| Step | Work, business day (core time) | Business | Non-business | Heavy |
| --- | --- | --- | --- | --- |
| S3.01 | 6 k orders at 2 µs (12 ms); the linked call, 3–5 ms of wall time per country | 8 ms | — | 11 ms |
| S3.02 | committee; tender allotment, inside the linked call; about 150 facility requests; purchase requests | 1 ms | — | 2 ms |
| S3.03 | about two auctions of up to 10⁴ orders at 300 ns; the curve and its discount-factor table | 2 ms | — | 4 ms |
| S3.04 | 20 books of 10³ orders at 300 ns (6 ms); 500 paper placements at 2 µs (1 ms); 20 k closed-form claim values at 100 ns (2 ms) | 3 ms | — | 6 ms |
| S3.05 | 24 k book orders at 300 ns (7.2 ms); 1 500 closing calls at 3 µs (4.5 ms); 30 k household reviews at 80 ns (2.4 ms) and 10 k instrument choices at 1 µs (10 ms); 2 k parts at 2.5 µs (5 ms); 30 k institutional values at 100 ns (3 ms) | 11 ms | — | 16 ms |
| Registered outlooks and values (from S3.03) | 0.2 M pairs with a new print at 70 ns (14 ms); 0.1 M registrations at 30 ns (3 ms) | 6 ms | — | 6 ms |
| S3.06 | 30 k quotes at 150 ns with their values at 100 ns (7.5 ms); 20 k client requests at 500 ns (10 ms); 10 k interdealer trades and fixings at 300 ns (3 ms) | 7 ms | — | 10 ms |
| S3.07 | managers' reviews, 600 × 50 candidates at 300 ns (9 ms); 30 k dealing orders at 100 ns (3 ms); 3 k dealing meetings at 1 µs (3 ms) | 5 ms | — | 8 ms |
| S3.08, S3.09, S3.10 | non-bank lenders; fixings and index reads; ratings, estimates and report days | 2 ms | — | 7 ms |
| Valuation, tests | 0.9 M positions at 10 ns (9 ms); margin, 2 k accounts at 4 µs (8 ms); repo margin, 30 k at 200 ns (6 ms); covenants | 8 ms | — | 16 ms |
| Money funds' daily accruals | about 3 000 funds | — | 1 ms | — |
| **Stage 3 total** | | **53 ms** | **1 ms** | **86 ms** |
| S3.05's parts at F-001's measured 12.1 µs (the risk case, outside the totals) | 2 k parts | +6 ms | — | +6 ms |
| A fund-run day (an event line): wake pass, woken visits, parts | 0.95 M; 0.2 M; 30 k | +60 ms (+150 ms at 12.1 µs) | — | +60 ms |

| Turn | Budget | Through Stage 2 | Through Stage 3 | Headroom |
| --- | --- | --- | --- | --- |
| Ordinary weekday (the median turn) | 1 000 ms | 1 082 ms | 1 135 ms | **misses by 13.5%** |
| Monday after a weekend | 2 000 ms | 1 770 ms | 1 825 ms | 9%, short of 10% |
| Heavy Monday | 2 000 ms | 2 212 ms | 2 300 ms | **misses by 15%** |
| Heavy Monday with tolerance control | 2 000 ms | 2 382 ms | 2 470 ms | **misses by 24%** |
| A four-day holiday block ending on a heavy day | 2 000 ms | 2 900 ms | 2 990 ms | **misses by 50%** |

Memory at the worst day's peak gains about **240 MB**:
- household cells, 568 → 584 bytes (`choose_holdings`: review exposure 8, attention 8 — the own rate and g_k, as for
  every household kind, S1.12): 16 B × 0.7 M = 11 MB; firm cells, 452 → 464 bytes (`place_cash`, 8 + 4, S3.01),
  inside §13.1's 500-byte line;
- households' holding rows of securities and fund units, about 1 M × 24 B, with holder-list entries 1 M × 6 B:
  30 MB;
- institutions' positions, about 1.2 M (funds 0.9 M; banks, firms and desks 0.3 M) at 32 B with a 6 B holder-list
  entry: 46 MB; their lots, about 1.5 per position at 32 B: 58 MB;
- instruments and their terms, about 60 k: 4 MB;
- fixings, benchmark, index, rating and report records, and each instrument's last close: 6 MB;
- repo, interbank, securities-loan and prime-brokerage lines, agreements and limit tables: 10 MB;
- registered instrument outlooks and values, about 0.5 M pairs × 24 B: 12 MB;
- individuals' own deviations from their methods' outlooks, one per position, about 1.2 M × 16 B: 19 MB;
- participation in the key: three bits of the key record, no word more; household keys estimated a third more,
  about 0.25 M × 72 B: 18 MB (at most 8× per base key; measured at S3.05);
- arena slack, 15% of the 175 MB of variable stores added: 26 MB;
- funds', desks', publishers', agencies' and finance companies' kind-table rows, about 5 k, inside §13.1's
  individuals line.

Through Stage 3 the design point projects a median turn of **1 135 ms**, 13.5% over the budget, a heavy Monday of
**2 300 ms**, 15% over, and a peak of **4 536 MB**, 0.8% over 4.5 GB. The budget itself is missed on time and on
memory, the required 10% headroom (at most 900 ms and 4 050 MB) by far, and nothing remains for Stage 4 (F-003,
F-004). S1.16 and S2.12 measure first, and S3.11 judges on the device.

**The remedy planned first** for the projected miss is N8.7's first, how the world is represented and traversed.
The stage's lines above already carry it: the linked call's warm start and pruned network, registered and shared
instrument outlooks and values, closed-form claim values, and participation in the key with holdings as counted
rows. What remains is taken from the largest lines as S1.16 and S2.12 measure them — parts (F-001) and settlement's
stream (§13.2) — before the play resolution (the cell budget, the tolerances and the zones) is touched; the
population never is.

---

### S3.01 — `sys-mmk`: the money market and repo

**Status**: planned

**Clauses**:
- STATE: MMK.1 *(completes it: overnight loans and the market, on S2.06's interbank line kind, with S2.06's
  bilateral term loans)*; MMK.2; MMK.3 *(part: firms here and money funds from S3.07; insurers complete it at
  S4.03)*; BFL.2 *(part: repo; certificates, paper and bonds complete it at S3.04)*.
- DECISION: MMK.4, MMK.5; BFL.6 *(completes it: the money market's route, beside S2.06's bilateral borrowing,
  pledging, bidding for deposits, shrinking lending and the facility)*.
- PROCESS: MMK.6, MMK.7, MMK.8; BFL.10 *(completes it: a bank short after the money market and the facility have
  both run fails for liquidity at 8f)*.
- MEASURE: MMK.9.
- FORBID: MMK.10.
- PRIMITIVE: MMK.11.
- A same-day shortfall meets the market at 8b before the facility at 8d.
- Money funds lend from S3.07 and insurers from S4.03, through the cash lenders' decision point declared here.
- S2.06's `marginal_cost_of_funds` gains the money market's routes as declared sources.

**Architecture**: §3.1 and §3.4 (decision-point homes, rule registration, the writer token), §6.1 (stage 8,
9c–9e), §8, §4.4 (the repo line, `Row` legs), §4.5, §9.2.

**Depends on**: S2.12 and the owner's go.

**Goal**: after the day's payments settle, every bank, and every cash-rich party with a repo agreement, posts from
its own position what it would borrow or lend at each rate, unsecured and against collateral, within its own limits
per counterparty and haircuts per security and borrower. The markets meet at 8b as call auctions keyed by borrower
and linked by the lenders' budgets. Who lends, who borrows, at what rate and who is refused are outcomes.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-base/src/money_market.rs` | tenors, segments, collateral baskets, haircuts, limits |
| `crates/interfaces/if-credit/src/interbank.rs` | S2.06's interbank line kind, extended: overnight, matched at 8b |
| `crates/interfaces/if-credit/src/repo.rs` | the repo line kind and the master agreement line kind (below) |
| `crates/interfaces/if-credit/src/decisions.rs` | adds `set_limits` (a lender's) |
| `crates/interfaces/if-state/src/decisions.rs` | adds `fund_position` (a bank's); it lives here because it reads the corridor, the tenders and the collateral framework (architecture §3.1) |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `place_cash` (a firm's: deposits, bills, repo, funds) |
| `crates/systems/sys-mmk/src/rules/reserves.rs` | MMK.4 for banks overnight: the reserve schedule and the collateral's assignment to segments |
| `src/rules/term.rs` | MMK.4 for longer tenors, and banks' bids at the central bank's tenders (S3.02) |
| `src/rules/cash.rs` | MMK.3: non-banks' offers and firms' cash placement |
| `src/rules/limits.rs`, `src/rules/haircuts.rs` | MMK.5: non-banks' limits; every lender's haircuts |
| `crates/systems/sys-bnk/src/rules/interbank_terms.rs` | MMK.5: a bank's limits and spreads (PC-40) |
| `src/handlers/8a_post.rs` | orders from `fund_position` and `place_cash` |
| `src/handlers/5c_limits.rs` | limit and haircut reviews; firm cells' `place_cash` reviews |
| `src/handlers/9c_repo_margin.rs` | repo margin: the test and its call in one handler, applied at 9e |
| `src/handlers/5c_repo_default.rs` | a lender's sale of collateral after an unmet call or a failed repurchase |
| `src/markets.rs` | market kinds per (country, segment, tenor), keyed by borrower, meeting at 8b in S0.18's linked call |
| `src/audit.rs` | the Collateral family (MMK.10) |
| `src/gen.rs` | opening limit lines, master agreements, open repos and interbank loans |
| `data/<country>/MMK.toml` | tenors, collateral baskets and eligibility, repo conventions, lots, ticks (POLICY) |
| `data/<country>/BFL.toml` | adds the money market's routes to `marginal_cost_of_funds`' declared sources |
| `data/<country>/gen/MMK.toml` | the opening agreements and positions, with sources |

`MMK.toml` also holds the review schedules and review hours (TECHNOLOGY) and the lenders' management types (risk
appetite, confidence, the share of a market's turnover it expects to sell per day: PREFERENCE).

**Design**

- **Markets** (MMK.1, MMK.2, MMK.6): per country, one call auction (MKT.3) per (segment, tenor, borrower). A segment
  is unsecured, or repo against one of the operator's declared collateral baskets with one lender, so the haircuts
  in a segment are one lender's. An interbank loan is lender + borrower + terms (Law 9), so a doubted name's market
  prices apart from a sound one's. Tenors (overnight, one week, one and three months), baskets, lots and ticks are
  the operator's POLICY (MMK.11); a basket only defines segments. A market meets on the days its borrower posts.
- **The linked call** (S0.18's `linked_call.rs`): the markets of one country, and the central bank's tenders
  (S3.02), meet together at 8b, because a lender's cash can go to any of them.
  - The network: source → lender `l` (capacity: its lendable cash, a `DeclaredLimit` from its balance; the central
    bank's, the tender's size) → offer node `(l, t)`, costed by its offer schedule for tenor `t` → edge
    `(l, b, s, t)`, existing only under a limit line (unsecured), a master agreement (repo) or the collateral
    framework (a tender), with capacity its limit headroom and cost its expected-loss spread → market node
    `(b, s, t)` → edge to borrower node `(b, t)`, with capacity the cash that the collateral the borrower assigned to
    segment `s` raises at that segment's haircuts (none for unsecured) and cost its encumbrance cost → sink, costed
    by the negative of its bid schedule.
  - Every capacity is cash on integer lots and no edge multiplies a flow, so the call maximises the traded surplus
    as one pure min-cost flow (Ahuja, Magnanti and Orlin, 1993). Ties among equal steps and equal-cost flows go by
    lot (stream `MMK.path_lot`, subject the country's market day), so identity is only the last key (§2.17).
  - Each market's price is its node's potential on the tick grid. Where a range is optimal, MKT.3's tie rules pick
    it: nearest the market's last print, then the lower rate. Every match into a market settles at its price, so
    each market has one price, and lenders to a doubted name are paid more.
  - A market with bids and no fill is a published `MarketFailure { kind: NoLender }` (MMK.7, MKT.10).
  - `phx-market` is the only crate that runs it (PC-51).
- **Banks' orders** (MMK.4, BFL.6): the decision point `fund_position`, each business day at 8a, for every bank.
  - **Inputs**: its reserves after stage 7 (a read, BFL.8); its reserve target for tomorrow's close — its buffer's
    reserve part (BFL.5, S2.06) and, where reserves are required, the average still owed over the maintenance period
    (S3.02); its own outlook of tomorrow's net payment flow, mean μ and width σ, adaptive over its own daily flows
    (VAL.6); the corridor's two rates (public); its free collateral by security at its own marks; the haircuts each
    lender published to it and its limits (below).
  - **Form** (Poole, 1968; Bindseil, 2004; Whitesell, 2006), listed in `SHAPES.toml`: a unit more held overnight
    costs `r − i_DF` against the deposit facility and saves `(i_LF − i_DF) × P(short tomorrow)`. With the flow outlook
    normal, the reserves wanted at rate `r` are `R*(r) = T + μ + σ·Φ⁻¹(1 − (r − i_DF) ÷ (i_LF − i_DF))`, on the ticks
    strictly inside the corridor. Its order at `r` is `R*(r) − R`: a bid where positive, an offer where negative. At
    the corridor's edges it posts nothing, since the facilities are its alternatives (MMK.8).
  - A bid is one schedule at the borrower node. Before posting, the bank assigns its free collateral across the
    repo segments it reaches, each unit to the segment where it raises the most cash net of its encumbrance cost,
    units its buffer counts last (listed with the form). The linked call places the bid in unsecured or repo markets
    by the lenders it can reach and that encumbrance cost: zero on collateral beyond its buffer's need,
    `(i_LF − i_DF)` times its outlook's shortfall probability on collateral its buffer counts.
  - What the market leaves is its own request at 8d to the facility (S2.06's form, now read after 8b), then lender
    of last resort (S3.02); a bank still short fails for liquidity at 8f and goes to §9.2's path (BFL.10, completed).
  - Bilateral term loans at 5c (S2.06) continue beside the market; both are rows of the one interbank line kind.
- **Term** (`term.rs`), on the bank's weekly funding review (a scheduled institutional decision), and at each of the
  central bank's tenders at the tender's tenor (S3.02):
  - it bids for tenor `T` up to its term funding gap (the part of its buffer it holds against outflows beyond `T`,
    S2.06) at rates up to its outlook of the compounded overnight rate over `T` — announced policy changes enter from
    their effective day (VAL.6) — plus `P_roll × (i_LF − E[r_on])`, where `P_roll` is its outlook of the share of its
    overnight bids left unfilled, from its own record;
  - it offers at rates from that compounded outlook plus the liquidity it gives up, `P_need × (i_LF − E[r_on])`,
    with `P_need` from its flow outlook. The borrower's expected loss is on the edge, from `loan_claim_value`
    (below), never in the offer.

  The term premium is an outcome (MMK.9). The form extends Poole's over the tenor and is listed with it.
- **The bank's cost of funds** (S2.06's rule handle): the money market's routes — overnight at its outlook of its
  own clearing rate, term and repo at their rates plus its rollover risk (`P_roll` above) — are declared sources of
  `marginal_cost_of_funds`, so a doubted bank's dearer funding reaches its loan and deposit quotes (L4).
- **Non-banks' offers** (MMK.3): `place_cash` at 8a for parties holding a master agreement — individual firms here,
  money funds from S3.07, insurers from S4.03, each kind registered by data. Cash beyond its liquidity target (its
  own buffer rule: S2.03's for firms) is offered in repo from its alternative (its deposit rate or the bill yield it
  can get) upward, to its agreement counterparties only. A firm cell holds no agreement (opening one costs the
  hours of `MMK.agreement_hours`); its `place_cash` is a lumpy review at 5c among deposits and bills at auction, with
  money funds from S3.07. The form — cash beyond a precautionary target placed at the best alternative by its own
  yields (Miller and Orr, 1966) — is listed in `SHAPES.toml` for `place_cash` and for repo offers.
- **Limits** (MMK.5), the decision point `set_limits`, on the lender's weekly review and on wakes: a public event
  about the counterparty (a liquidity failure, a resolution at a bank like it), a fail of the counterparty's payment
  to it, and from S3.10 a rating change. One rule per lender kind (architecture §3.4):
  - **a bank's** is `sys-bnk`'s (PC-40): its one `LoanAssessment` of the counterparty — S2.06's classing of banks from
    their published funding and capital figures and public events, from S2.10 bought records, from S3.10 ratings —
    gives PD and LGD; the edge's spread for tenor `T` is the rate at which `loan_claim_value` of a loan of that tenor
    equals its principal, less its cost of funds; the limit is `L = A ÷ (PD × LGD)`, where `A` is the expected loss
    its management will carry per counterparty (PREFERENCE, BNK.16), within its large-exposure limit (S2.07);
  - **a non-bank's** (individual firms here, money funds from S3.07) is `sys-mmk`'s, the same form over its own
    outlooks of the counterparty's failure (its adaptive outlook of failures it has seen, resolutions being public,
    VAL.5, from VAL.10 at the opening) and of recoveries.
  - The form — limits set by counterparty risk (Afonso, Kovner and Schoar, 2011) within an exposure budget (Basel
    Committee, 2014) — is listed in `SHAPES.toml`.
- **Haircuts** (MMK.5), per (security, borrower), on the same review: `h = z·σ·sqrt(T_liq)`, where σ is the
  lender's outlook width of the security's daily price change, `T_liq` its outlook of the days needed to sell the
  borrower's expected delivery at the security's recent turnover (MKT.15's read) and its own share of turnover sold
  per day, and z its confidence. The haircuts are published to the borrower (a record whose audience is the pair)
  before 8a. A borrower whose PD exceeds the lender's appetite gets a limit of zero. The form — value-at-risk
  haircuts (Brunnermeier and Pedersen, 2009; Gorton and Metrick, 2012) — is listed.
- **Repo** (MMK.2, REG.2), one line kind:
  - start leg at 8c: cash from lender to borrower, collateral units from borrower to lender, and `Row` legs creating
    the line's rows (S0.15); title passes;
  - the line holds the repurchase leg (cash with interest; the units back), due on its date in the stage-7 stream;
  - a coupon or dividend paid to the lender as holder of record (REG.11) is owed back as a manufactured payment on
    the line, due on the income's day;
  - where the agreement allows re-use, the received units are the lender's free units; each onward repo or sale
    writes a chain link on their tag, so the chain is traceable. The borrower's free units exclude them, and its
    right to their return is the line's delivery leg, never a free unit (MMK.10);
  - variation margin: at 9c one handler compares the collateral at 9a's marks (or a named valuer's valuation,
    labelled) with the cash lent grossed up by its haircut and, on a gap beyond the agreement's threshold, issues the
    call, applied at 9e and due at the next business day's 2c (TIME.7);
  - an unmet call or a failed repurchase defaults the agreement: the lender keeps the units and sells them at 5c by
    its own orders, limited by its declared liquidation horizon (POLICY of the lender), claiming any shortfall and
    returning any excess (L2's door of a funding line withdrawn).
- **Settlement** (MKT.11): matches become instructions and settle at 8c through the stage-7 routine, reserves moving
  once per bank by net. An unsecured match's `Row` legs write rows on S2.06's interbank line kind (its overnight
  terms added here), repaid at maturity in the stage-7 stream; the borrower's intraday credit carries it until it
  borrows again (MON.3). The lender provisions and works the loan out by S2.01 like any loan.
- **Streams**: `MMK.path_lot`.
- **Review costs** (TECHNOLOGY, hours): `MMK.funding_review_hours` per daily posting, `MMK.limit_review_hours` per
  counterparty reviewed, `MMK.agreement_hours` per agreement opened, `MMK.place_cash_hours` per firm cell's
  `place_cash` review; each counted per decision kind.
- **Opening** (GEN): limit lines and master agreements between banks and with large firms, drawn from supervisory
  exposure data by bank size (ENDOWMENT); the open loans and repos of the opening balance sheets; the history of
  overnight prints for outlooks (GEN.5).

**Unit tests**
- `poole_schedule_falls_with_rate`: `R*(r)` falls as r rises, and nothing is posted at the corridor's edges.
- `poole_schedule_centred_by_target_and_outlook`.
- `term_schedule_prices_rollover_risk`.
- `term_offer_excludes_expected_loss`.
- `collateral_assigned_to_segments_as_cash`: an assignment's capacities are cash at each segment's haircuts.
- `linked_call_matches_brute_force`: on small money-market networks, the surplus equals an exhaustive maximisation.
- `linked_call_one_price_per_market`.
- `linked_call_respects_limits_budgets_and_collateral`.
- `linked_call_equal_paths_by_lot`: two draws give different orders; identity decides only between equal draws.
- `limit_falls_as_pd_rises`.
- `haircut_rises_with_width_and_size`.
- `repo_legs_pass_title_and_income`.
- `reuse_chain_links_every_onward_use`.
- `repo_call_from_marks_and_threshold`.

**Live checks**
- `LC-3-01`: MMK.9 — the spread between the strongest and weakest names, repo haircuts by basket, the term premium
  and the pass-through of the corridor's changes to overnight prints are reported.
- `LC-3-02`: MMK.10, the Collateral family — no unit is free for two holders, and every received unit's chain ends
  at a holder.
- `LC-3-15`: MMK.4 — every money-market order names its poster's decision point, and every order of a bank's
  `fund_position` lies strictly inside the corridor; the share of banks that only lent, or only borrowed, over each
  year is reported (a measure, MMK.9).
- `LC-3-16`: MMK.7 and BFL.10 — every borrower market with bids and no fill is a published failure, followed the same
  day by that borrower's facility request or its liquidity failure; every liquidity failure followed the market, the
  facility and the 8f shortfall, in that order, and went to resolution (§9.2).
- `LC-3-17`: every repo call names the marks it read, and every unmet call ends in the lender's sale and a settled
  shortfall or excess.

**Budget**
- About 150 banks and 3 000 non-bank lenders with agreements post each business day (6 000 from S3.07); networks of
  about 20 000 edges per country; the linked call 3–5 ms of wall time per country, warm-started, on one core per
  country with the countries in parallel (S0.18's prototype measures it, and its instruction count is ratcheted on
  a fixed network); an order ≤ 2 µs; margin at 9c ≤ 200 ns per open repo, about 30 000 (the valuation line).
- The ledger's line: 8 ms on a business day (12 core-ms of orders ÷ 3, plus the call's 3–5 ms), 11 ms on a heavy
  day, nothing on a non-business day.
- Firm cells: `place_cash`'s review exposure and attention rate (8 + 4) add 12 bytes: 452 → **464** of 500.
- Lines, agreements and limit tables, under 10 MB, within §13.1's lines and instruments lines.
- Counters, ratcheted: `phx_mmk.orders`, `phx_mmk.network_edges`, `phx_mmk.augmenting_paths`,
  `phx_mmk.failures`, `phx_mmk.repo_calls`, `phx_mmk.collateral_sales`, `phx_market.linked_call_instructions`,
  `phx_pop.bytes_per_firm_cell`.

**Guards**: PC-51: no system crate calls a matching, clearing or allocation function of `phx-market`; markets are met
only by `phx-market`'s meeting handlers, so no system assigns lenders to borrowers (MMK.10).

**Not allowed**:
- surplus banks assigned to lend;
- one limit or one haircut shared by every lender, or a haircut per basket in place of per security and borrower;
- a market rate equal to the policy rate by construction, or a bank's `fund_position` order outside the corridor;
- an edge whose flow is multiplied, so the call is no longer a pure min-cost flow;
- a unit of collateral free on both sides;
- a facility drawn without the bank's request;
- an order at 5c that reads stage 8's positions.

**Done when**
- [ ] Reserve positions are funded in the market, at the facilities or not at all, by each bank's own orders.
- [ ] LC-3-01, LC-3-02 and LC-3-15 to LC-3-17 pass.
- [ ] PC-51 is registered.
- [ ] Two reviews are done.

---

### S3.02 — `sys-cb` in full: rates, operations, collateral, lender of last resort and the financing regime

**Status**: planned

**Clauses**:
- STATE: CB.2, CB.3; CB.1 *(part: the domestic balance sheet in full; foreign reserves, claims on other central
  banks and the revaluation account for foreign positions are S5.04)*.
- DECISION: CB.4, CB.6, CB.14; CB.5 *(part: programmes decided and sized; purchases trade through dealers, which
  complete it at S3.06)*.
- PROCESS: CB.7, CB.8, CB.9; CB.10 *(part: remittance, and a loss kept against equity; the treasury's making it good
  completes it at S3.03)*.
- MEASURE: CB.12.
- FORBID: CB.13.
- PRIMITIVE: CB.16.
- Intervention with foreign reserves (CB.11) is S5.04; swap lines (CB.15) are S5.05.
- This step retires S1.10's placeholders naming CB: the fixed policy rate and corridor, and the declared list of
  eligible collateral with haircuts.
- It introduces one placeholder naming RAT (S3.10): eligibility reads no rating until ratings exist.

**Architecture**: §6.1 (stages 5 and 8), §4.6 (policy values), §8, §9.2.

**Depends on**: S3.01.

**Goal**: each central bank sets its rates on its committee's schedule from its own outlooks against its mandate;
implements them through a corridor or a floor with tenders sized from its own forecasts; lends to banks against
collateral it chooses at haircuts it sets, and as lender of last resort by its declared conditions; buys and sells
assets for policy through markets; funds its treasury only as the regime allows; and remits its income.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-state/src/central_bank.rs` | mandate, target, financing and operating regimes, rate grid, the tender |
| `crates/interfaces/if-state/src/collateral.rs` | the framework: eligibility and haircut per security |
| `crates/systems/sys-cb/src/rules/rate.rs` | CB.4: the rate decision |
| `src/rules/operations.rs` | CB.14: tender sizes from the autonomous-factor forecast |
| `src/rules/collateral.rs` | CB.6 |
| `src/rules/purchases.rs` | CB.5 |
| `src/handlers/5c_committee.rs` | the rate decision on meeting days; the announcement |
| `src/handlers/5c_operations.rs` | tender and fine-tuning announcements |
| `src/handlers/8a_tender.rs` | the central bank's tender order, a lender in S3.01's linked call |
| `src/handlers/5c_purchases.rs` | purchase and sale orders into secondary markets |
| `src/handlers/9c_reserves.rs` | the maintenance test and its penalty demand in one handler, applied at 9e |
| `src/handlers/9c_remit.rs` | net income on remittance dates and the remittance due, applied at 9e |
| `src/markets.rs` | the deposit and lending facilities, lender of last resort and the treasury's direct borrowing as administered markets (MKT.8), met at 8d by `phx-market`'s administered form, each limit a `DeclaredLimit` (PC-51) |
| `data/<country>/CB.toml` | mandate, target, financing regime (parliament); rule, neutral rate, regime, framework, penalty (CB) |

`CB.toml` holds the committee's meeting calendar and review hours, the rate grid, the maintenance period and reserve
ratio where required, the remittance dates, and the four lender-of-last-resort conditions as declared data.

**Design**

- **The rate decision** (CB.4), at 5c on the committee's meeting days:
  - **Inputs**: its outlook of inflation on the published consumer index with its lag (a public-series outlook by
    the central bank's own method, S1.01); its outlook of activity — the published unemployment rate and output —
    against its own experience-weighted long mean of each (the anchor heuristic), which is its view of the gap; the
    neutral real rate `r̂`, POLICY of the committee (CB.16), reviewed on its declared schedule, never an outlook of
    its own output; its mandate and target.
  - **Form**: an inertial Taylor rule over its own real-time outlooks (Taylor, 1993; Clarida, Galí and Gertler,
    2000; Orphanides, 2003), listed in `SHAPES.toml`:
    `i* = r̂ + π̂ + φ_π·(π̂ − π*) + φ_u·(û* − û)`, then `i = ρ·i_prev + (1 − ρ)·i*`, on the central bank's rate grid.
    φ_π, φ_u and ρ are POLICY of the central bank (CB.16); φ_u is zero where the mandate names no employment goal —
    data, not a branch (Law 10).
  - The decision writes a dated announcement effective the next business day (VAL.6, §4.6). The facility rates move
    with it by the corridor's declared widths (POLICY of the central bank). No bank's pricing takes it as a cost of
    funds but through `marginal_cost_of_funds` (PC-50).
  - Review cost: `CB.committee_hours` per meeting.
- **Operations** (CB.14), at 5c on tender days:
  - the regime — a corridor with scarce reserves or a floor with ample ones — is POLICY of the central bank;
  - under a corridor, the tender's size is `R_target − R_now − ΔA`: `ΔA` is its outlook of the autonomous factors
    until the next tender (the treasury account's and banknotes' changes, adaptive over its own records, with the
    treasury's announced auctions and redemptions entering on their dates), and `R_target` its outlook of banks'
    aggregate demand at the policy rate (required reserves plus its adaptive outlook of the excess banks held);
  - under a floor, `R_target` is the regime's declared ample level (POLICY) and tenders run at full allotment;
  - the form — liquidity-neutral allotment from forecast autonomous factors (Bindseil, 2004) — is listed.
  - The tender is a repo against the framework's collateral, and one lender in S3.01's linked call at 8b: a lender
    node whose capacity is the tender's size, whose offer schedule runs from the minimum bid rate (the policy rate)
    up, and whose edges run to banks' market nodes under the framework's haircuts. So each bank's schedule meets once,
    beside the money market. At full allotment the node's rate is fixed and its capacity is all the cash the
    collateral banks assigned to it raises, an administered price met inside the call (MKT.8). Banks bid from S3.01's
    term rule at the tender's tenor, not from `fund_position`. Rationing ties go by the call's lot (stream
    `MMK.path_lot`). Fine-tuning operations are the same, on days its forecast's surprise exceeds its declared
    sensitivity.
  - Review cost: `CB.operations_hours` per tender or fine-tuning decision.
- **Reserve requirements** (CB.14), where a country has them: a bank's requirement is the declared ratio of its
  reservable liabilities at the period's start; each close's reserves count toward the average; the average owed is
  an input of `fund_position` and of S2.06's buffer. At the period's end one 9c handler tests the average and issues
  the penalty (POLICY), applied at 9e and due the next business day.
- **The corridor** (CB.7): the deposit facility pays its rate on reserves placed at 8d; the lending facility lends
  overnight at 8d against eligible collateral at its haircut. Both are administered prices with quantity responses
  (MKT.8), met by `phx-market`'s administered form only on banks' own requests (S1.09), their limits
  `DeclaredLimit`s. Collateral pledged is a lien (REG.2), released at repayment.
- **Collateral** (CB.6), on its framework review (quarterly) and on wakes:
  - eligibility is declared data (asset classes and issuer kinds, POLICY) plus, from S3.10, a minimum rating; until
    then, the placeholder naming RAT: no rating is read;
  - a security's haircut is `z_cb·σ·sqrt(T_cb)` with σ the central bank's outlook width of its daily price change and
    `T_cb` its declared liquidation horizon; `z_cb` and `T_cb` are POLICY. The form is value-at-risk haircuts, listed
    with S3.01's;
  - review cost: `CB.framework_hours`.
- **Lender of last resort** (CB.8), at 8d, on a bank's request after the market and the lending facility, through the
  administered form with the four conditions as its admission hook: granted when the supervisor's solvency fact
  (S2.08) says solvent that day, against the framework's collateral and loans it declares good, at the lending rate
  plus its declared penalty, for as much as the collateral carries after haircuts ("freely"). Refused, the bank goes
  to §9.2's path. The four conditions are declared data, visible, and a country may declare others.
- **Purchases** (CB.5), on the committee's meeting days:
  - a programme is decided when `i*` lies below the lowest rate on its grid it will set (its declared lower bound,
    POLICY); its size is `(i_lb − i*) × s_cb`, with `s_cb` its declared purchases per unit of rate gap (POLICY),
    spread over its declared horizon; maturing holdings are reinvested while a programme runs and run off otherwise;
    the form — purchases sized by the rate gap (Gagnon et al., 2011; Wu and Xia, 2016) — is listed;
  - it buys only in secondary markets, as a client of dealers (MKT.5), with limits at its own value of each bond:
    VAL.8's claim value at its own outlook of the policy path. The dealer market meets from S3.06, so purchases
    trade from then (spec Part O's rule for a later system of the stage);
  - purchases create reserves and sales destroy them (CB.9, MON.6);
  - the eligible set is declared data and excludes its own sovereign under a regime that forbids funding it;
  - review cost: `CB.purchases_hours` per programme review.
- **The financing regime** (CB.3, CB.9), POLICY of the parliament: whether the central bank may buy its sovereign's
  debt in the secondary market, and whether it may lend to its treasury directly, within what limit and at what
  rate — declared values, read as a `DeclaredLimit`, never a branch. A direct loan is the treasury's own request at
  8d (S3.03), met by the administered form and settled at 8e; the treasury's account rises, and reserves and deposits
  are created as it spends. The central bank never bids at a primary auction (CB.13): the auction's participants
  rule refuses its kind (PC-52).
- **Income and remittance** (CB.10): at 9c on each remittance date, from its statement of 9b (ACC.9), realised net
  income is a remittance due to the treasury the next business day (applied at 9e, paid at stage 7); unrealised
  revaluation gains go to equity and are not remitted; a loss is kept against equity, which may go below zero
  (Law 13), until the treasury decides to make it good (S3.03).
- **Streams**: none new; the tender's ties are the linked call's.
- **Opening** (GEN): the balance sheet — government securities, loans to banks, notes, reserves, the treasury
  account, equity — from the country's central-bank data; the policy rate's history and the committee's
  outlooks' starting points (GEN.5).

**Unit tests**
- `taylor_rule_inertia_and_grid`.
- `taylor_rule_reads_the_lagged_index`: the input is the last published value, never a later one.
- `employment_weight_zero_by_mandate_data`.
- `tender_size_from_autonomous_factors`.
- `tender_node_capacity_and_haircuts`: the tender's node carries its size, and its edges the framework's haircuts.
- `maintenance_average_owed`.
- `lolr_conditions_as_data`.
- `haircut_from_width_and_horizon`.
- `purchase_size_from_rate_gap`.
- `remittance_excludes_unrealised_gains`.
- `direct_lending_bound_by_regime`.

**Live checks**
- `LC-3-03`: CB.12 — the transmission of rate decisions to overnight prints, bank funding costs, loan quotes, asset
  prices, investment and inflation, with lags, is reported (L4's liveness).
- `LC-3-18`: CB.13 — no central-bank order at a primary auction; every central-bank loan is collateralised within
  its haircuts, priced at an announced rate and within the regime's limit; no purchase follows a failed auction by
  construction (each purchase names its programme's decision).
- `LC-3-19`: CB.8 — every lender-of-last-resort loan went to a bank the supervisor held solvent that day, and every
  refused request's bank entered resolution (§9.2).
- `LC-3-20`: CB.10 — each remittance equals realised net income for its period; unrealised gains stayed in equity.
- `LC-3-21`: the central bank's forecast errors of autonomous factors and banks' facility use are published per
  tender.

**Budget**
- The committee meets about eight times a year; tenders weekly, inside the linked call; facilities about 150
  requests a day; purchases tens of requests a day from S3.06.
- The ledger's line: 1 ms on a business day, 2 ms on a heavy day.
- Counters, ratcheted: `phx_cb.rate_decisions`, `phx_cb.tender_bids`, `phx_cb.facility_uses`, `phx_cb.lolr`,
  `phx_cb.purchase_requests`.

**Guards**
- PC-50: no rule of a bank's loan or deposit pricing takes a central-bank rate as its cost of funds except through
  the rule handle `marginal_cost_of_funds` (a signature check on those rules' input views); public administered rates
  — the announced policy and facility rates — may be read by any rule through their public records (L4's "a bank
  that prices from the policy rate").
- PC-52: a market declared as a primary issue refuses, at assembly, a participants rule that admits the central bank's
  kind (CB.13).

**Not allowed**:
- a market rate equal to the policy rate by construction;
- a purchase sized by an auction's weakness, or at a primary auction;
- lending without collateral, price or limit;
- a facility or a treasury loan met outside `phx-market`'s administered form;
- financing of the treasury beyond its regime;
- a rate decision reading an unpublished index, or a neutral rate read from its own output.

**Done when**
- [ ] Rates, operations, facilities, lender of last resort, purchases and remittance run from the central bank's own
  decisions and declared policy.
- [ ] LC-3-03 and LC-3-18 to LC-3-21 pass (LC-3-03's asset-price and investment links from S3.05).
- [ ] PC-50 and PC-52 are registered.
- [ ] Two reviews are done.

---

### S3.03 — `sys-trs` and `sys-sov` in full: the funding plan, bonds, the curve and default

**Status**: planned

**Clauses**:
- STATE: SOV.1 *(completes it: fixed-coupon and inflation-linked bonds, reopened lines and their tranches)*; SOV.2
  *(completes it: buybacks and switches)*.
- DECISION: TRS.2, TRS.3; SOV.3 *(completes it)*; SOV.4 *(part: one decision point, `bid_at_auction`, with a rule per
  bidder kind, each kind of Stages 3 and 4 registering its own; foreign reserve managers complete it at S5.04)*.
- PROCESS: SOV.7 *(part: the curve over auction prints; the dealer market and its fixings complete it at S3.06)*;
  TRS.5 *(part: default in the sovereign's own currency, the exchange offer and holdouts; debt in a foreign
  currency completes it at S5.04)*; CB.10 *(completes it: the treasury making good a central-bank loss)*.
- MEASURE: TRS.7, SOV.8.
- FORBID: TRS.8, SOV.9.
- PRIMITIVE: SOV.10, TRS.10.
- This step retires S1.11's placeholders naming TRS: the funding rule and the rule for when cash runs short (whose
  deferral of discretionary purchases stays a placeholder naming SOC, S5.02).
- S1.11's placeholder naming DLR (primary dealers) stays until S3.06; the secondary market meets from S3.06.

**Architecture**: §6.1 (stages 5, 6 and 8), §8, §9.1 (the sovereign's ending).

**Depends on**: S3.02.

**Goal**: the treasury funds ahead of spending by a forward plan from its own outlooks, keeping a buffer it chooses;
auctions of bills and bonds can fail, and the treasury handles a shortfall by its declared options; every kind of
bidder bids from its own value; one publisher fits the curve through the day's prints; a sovereign defaults exactly
where its declared payment priority, its regime and the market leave a debt service unpaid, and offers an exchange
that holders accept or refuse.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-val/src/schedule.rs` | the investor schedule, a pure function (below), filling S1.01's extension point |
| `crates/interfaces/if-securities/src/sovereign.rs` | bill, fixed and indexed families; lines and tranches |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `bid_at_auction` and `answer_exchange`, a rule per kind |
| `crates/interfaces/if-state/src/funding.rs` | the plan, the auction calendar, the mandate, the payment priority |
| `crates/interfaces/if-state/src/decisions.rs` | adds `make_good_cb_loss` (the treasury's) |
| `crates/systems/sys-trs/src/rules/plan.rs` | TRS.2 |
| `src/rules/shortfall.rs` | TRS.3 |
| `src/rules/service.rs` | TRS.5: paying by the declared priority, TRS.10 |
| `src/rules/exchange.rs` | TRS.5: the exchange offer's terms |
| `src/rules/operations.rs` | SOV.2: buybacks and switches |
| `src/rules/cb_loss.rs` | CB.10: making good a central-bank loss |
| `src/handlers/5c_*.rs` | plan reviews, shortfall, debt service, operations, exchange offers |
| `src/handlers/8d_cb_request.rs` | the treasury's request for direct central-bank lending, where the regime allows |
| `src/handlers/2e_default.rs` | a debt-service fail past the bond's grace becomes the sovereign's default |
| `crates/systems/sys-sov/src/rules/bid.rs` | SOV.4: the bidding rule of every bidder kind but banks and dealers |
| `src/rules/accept.rs` | non-bank holders' answers to an exchange offer or a switch |
| `crates/systems/sys-bnk/src/rules/{bid,exchange}.rs` | a bank's bid and answer, by `loan_claim_value` (PC-40) |
| `src/handlers/5c_bid.rs` | bids on auction days |
| `src/handlers/6c_curve.rs` | the curve publisher's fit (SOV.7) and the day's discount-factor tables |
| `src/markets.rs` | auction and buyback markets per line; switches are bilateral offers; the secondary market is S3.06's |
| `data/<country>/{TRS,SOV}.toml` | mandate, buffer cover, benchmarks, formats, indexation lag (POLICY) |

`TRS.toml` also holds the declared payment priority (POLICY of the parliament, TRS.10), the central bank's equity
floor the treasury restores (POLICY of the parliament), the plan's horizon and review schedule and the review hours
(TECHNOLOGY); `SOV.toml` holds the curve publisher's method (POLICY of the publisher) and the operations' notice
periods.

**Design**

- **The investor schedule** (`phx-val/src/schedule.rs`), used by every bidder here and by later steps:
  - `schedule(value, variance, risk_aversion, held, funds, free_units, ticks) -> Schedule`: at each tick `p` the
    target holding is `h*(p) = (value − p) ÷ (γ·variance)` and the order is `h*(p) − held`, a bid where positive and
    bound by its funds, an offer where negative and bound by its free units (a short needs a borrow, S3.06);
  - `value` is VAL.8's claim or firm value at the party's own outlooks, `variance` its outlook width of the price
    over its horizon, γ its risk aversion (the household's or the management's PREFERENCE);
  - a tick where the expected gain of trading is below the poster's cost of trading — fees and its review hours —
    is left out, which gives a band of no trade (Constantinides, 1986; Davis and Norman, 1990);
  - the form is CARA–normal demand (Grossman, 1976), listed in `SHAPES.toml`.
- **The funding plan** (TRS.2), on the plan's quarterly review and on wakes (a failed auction; a surprise in receipts
  or outlays beyond its sensitivity):
  - **Inputs**: its own outlooks of outlays and receipts per category and calendar period (adaptive over its own
    records of the same period, VAL.6); redemptions and coupons read from the register; announced policy changes from
    their effective day; its cash; its mandate.
  - **Need** over the horizon `H`: redemptions plus its outlook of the deficit plus the buffer target less its cash.
  - **Buffer**: `B = z_T × w`, where `w` is the width of its outlook of cumulative net flows over an auction interval
    (its own surprises) and `z_T` the cover its mandate states (POLICY, TRS.9).
  - **Split**: across bills, fixed bonds and indexed bonds, and across the mandate's benchmark maturities, by the
    mandate's declared issuance shares (POLICY); a benchmark line is reopened until it reaches the declared benchmark
    size, then a new line opens; each tranche is its own record (SOV.1).
  - **Calendar** (SOV.3): auctions on the declared days per family, each sized as its share of the need, announced as
    a public record at least the declared notice before (POLICY).
  - The form — a forward funding plan with a precautionary buffer (IMF and World Bank, 2014; Williams, 2004) — is
    listed in `SHAPES.toml`. Review cost: `TRS.plan_review_hours`.
- **Bidding** (SOV.4), at 5c on auction days, the decision point `bid_at_auction` (`if-securities`, where bond terms
  are), with a rule per bidder kind (architecture §3.4):
  - **a bank's** is `sys-bnk`'s: the bill or bond's value is `loan_claim_value` under its one `LoanAssessment` of the
    sovereign (sovereign defaults being public events), at its outlook of short rates; its bills keep S1.11's
    liquidity form, now with S3.01's corridor and S2.06's buffer;
  - **a firm's and a fund's** are `sys-sov`'s (a dealer's is `sys-dlr`'s, S3.06): the claim value (VAL.8) at its
    own outlook of short rates (announcements from their effective day) and its own outlook of the sovereign's
    default, over its horizon;
  - the order is the bidder's investor schedule at that value;
  - households bid through their bank by S1.12's limit order at their own reservation yield; firms through
    `place_cash`; funds from S3.07, dealers from S3.06, insurers and pensions from Stage 4, reserve managers from
    S5.04, each by registering its kind;
  - the central bank's kind is refused by the auction's participants rule (PC-52).
- **Auctions** (SOV.6, S1.11): uniform-price call auctions at 6a; rationing at the price by largest remainder, ties by
  lot (stream `SOV.auction_lot`); unsold paper is not issued and is a published event (MKT.10); cover and tail are
  published. Settlement on the declared convention.
- **A shortfall** (TRS.3), at 5c on the business day after an auction that sold less than offered, by the declared
  protocol (POLICY of the treasury), listed with the plan's form:
  1. draw the buffer down to its declared minimum (the next days' statutory outlays);
  2. for the rest, the cheapest by its own outlooks of: an extra auction at the next business day at another size or
     maturity (a longer bill, or a reopening of a bond line), and, where the regime allows, direct central-bank
     lending at the regime's rate within its limit (a request at 8d);
  3. defer discretionary purchases (the placeholder naming SOC, S5.02).
- **Debt service and default** (TRS.5, TRS.10):
  - at 5c on the business day before a service date, if its cash, the funding it can still reach (settling auctions,
    the regime's limit) and its outlook of receipts fall short of the day's payments, it pays in the payment priority
    declared by its parliament (POLICY, TRS.10) — for example statutory payments, then debt service, then
    discretionary purchases — or the order the country declares. The treasury's "will not pay" is carried by this
    declared priority only; there is no other decision to default;
  - a coupon or redemption the treasury cannot pay fails at stage 7 and is recorded at 2d against the instrument's
    event (S0.14's events); past the bond's grace (its terms) it is the sovereign's default at 2e: a public event on
    every line of its debt (cross-default by terms), the instruments' state written by `phx-ledger`'s events;
  - in its own currency, this can happen only where the regime forbids central-bank funding or its limit binds and
    the market refuses; where funding is allowed, the treasury borrows at 8d and the consequence runs through money
    and prices instead. Nothing forces either outcome.
- **The exchange offer** (TRS.5), at 5c after a default:
  - the treasury offers new bonds whose debt service over its declared horizon fits its outlook of receipts less
    statutory outlays: face reduced by `1 − PV(affordable service) ÷ PV(contractual service)`, discounted at its own
    outlook of its pre-default borrowing cost. The form — sizing by a debt-sustainability analysis (IMF, 2013) — is
    listed;
  - each holder answers by message (`answer_exchange`) by the offer's deadline: it accepts when its own value of the
    new bond is at least its value of holding out, which is the old claim at its own recovery outlook (from
    recoveries it has seen, VAL.10 from the offer itself); a bank's values are `sys-bnk`'s (`loan_claim_value`). The
    form (Pitchford and Wright, 2012) is listed. Cells answer for all members;
  - where acceptances by face reach the bond's collective-action threshold (its terms), every holder of that line is
    bound; otherwise holdouts keep their holdings of the defaulted bond, its events failing as they fall due;
  - the exchange settles at stage 7 as one instruction per line: old units retired, new issued, holdouts untouched;
  - market exclusion is not declared: bidders' own assessments now carry the default, and auctions fail or clear high
    as they will.
- **Buybacks and switches** (SOV.2), on the monthly operations review: with cash above its buffer target, the
  treasury buys back lines maturing within its declared smoothing horizon, by a reverse call auction in which its
  order is limited at its own value (discounted at its funding-cost outlook), ties by lot (stream
  `SOV.operations_lot`). A switch is bilateral (MKT.7): the treasury offers the old line's holders units of the new
  line at a ratio of its own values; each holder answers at the next 5c from its own values of both lines, by the
  exchange answer's form. Both are listed with the plan's form.
- **Making good a central-bank loss** (CB.10), the decision point `make_good_cb_loss` (`if-state`, the treasury's), on
  the plan's review: when the central bank's published equity is below the floor its mandate names (POLICY of the
  parliament), the treasury decides a transfer restoring it, funded in its plan as an outlay to a named party
  (TRS.4); a floor declared absent leaves the loss carried. The form — recapitalisation to a declared floor (Stella,
  2005) — is listed in `SHAPES.toml`.
- **Indexed bonds** (SOV.1, SOV.10): principal indexed by the contract algebra's `Indexed` leg on the published
  consumer index with the declared lag; break-even inflation is a read (SOV.8).
- **The curve** (SOV.7), at 6c, by one publisher (a large firm with the publisher facet, opened by GEN): zero rates
  bootstrapped from the day's prints of bills and bonds (auction prints here, dealer-market fixings of 6b from
  S3.06), linear in the zero rate between traded maturities, nothing beyond the longest, every point labelled traded
  or interpolated. It reads only the day's prints, never its own earlier output (SOV.9); fewer than two traded
  points leave the day's curve absent and the last one shows its age. It is a record labelled as a valuation input
  (MKT.20), never a print, and with it the day's **discount-factor table** per curve, which valuers of untraded
  bonds and closed-form claim values read. The method is POLICY of the publisher.
- **The treasury account** never goes below zero (TRS.8): it has no facility (MON.3), and a payment it cannot make
  fails.
- **Streams**: `SOV.auction_lot`, `SOV.operations_lot`.
- **Review costs** (TECHNOLOGY, hours): `TRS.plan_review_hours`, `TRS.operations_hours`, `SOV.bid_hours` per bid,
  `SOV.exchange_answer_hours` per holder answer.
- **Opening** (GEN): the outstanding lines and tranches of each sovereign from its debt office's data; the primary
  dealers' agreements (S1.11's banks); the curve publisher; the history of auction prints (GEN.5).

**Unit tests**
- `investor_schedule_monotone_and_bounded`: bids bound by funds, offers by free units; the no-trade band widens with
  the cost of trading.
- `plan_need_and_buffer`.
- `plan_reopens_until_benchmark_size`.
- `shortfall_protocol_order`, including a return at another maturity.
- `payment_priority_as_data`.
- `exchange_haircut_from_affordable_service`.
- `holdout_versus_accept`.
- `collective_action_threshold_binds_line`.
- `reverse_auction_limit_at_own_value`.
- `switch_answer_from_own_values`.
- `cb_loss_made_good_to_floor`.
- `curve_labels_traded_and_interpolated_and_reads_no_prior_curve`.
- `discount_factors_from_zero_rates`.
- `indexed_principal_with_lag`.

**Live checks**
- `LC-3-04`: TRS.7 and SOV.8 — auction tails and cover, the curve's level and slope, yields' response to issuance and
  policy, break-even inflation and deficits in downturns are reported.
- `LC-3-22`: SOV.6 and SOV.9 — every auction's issued amount equals what it sold; every unsold offer is a published
  event; every order at an auction names its bidder's decision point.
- `LC-3-23`: TRS.2 and TRS.8 — every auction was announced at least its notice before; the treasury account never
  closed below zero; every outlay names its recipient.
- `LC-3-24`: SOV.7 — every curve point is labelled, and no fit read an earlier curve (the read-trace).
- `LC-3-25`: TRS.5 and TRS.10 — every sovereign default names its failed leg, its country's regime, the funding it
  could not reach and the declared priority that left the leg unpaid; a run with no default passes it vacuously, and
  the report says so.

**Budget**
- About two auctions per business day per country, each with up to 10⁴ orders (households' through banks): the call
  ≤ 1 ms; the curve and its discount-factor tables ≤ 0.5 ms; plan and operation reviews negligible.
- Registered instrument outlooks and values begin here: about 0.2 M pairs with a new print a business day at 70 ns
  and 0.1 M registrations at 30 ns, 17 core-ms, about 6 ms, the ledger's own line.
- The ledger's line: 2 ms on a business day, 4 ms on a heavy day.
- Counters, ratcheted: `phx_sov.auctions`, `phx_sov.auction_orders`, `phx_sov.failed_auctions`,
  `phx_sov.curve_points_traded`, `phx_trs.shortfalls`, `phx_val.instrument_outlooks`, `phx_val.registered_pairs`.

**Guards**: none new (PC-52 is S3.02's).

**Not allowed**:
- an automatic central-bank overdraft, or central-bank funding beyond the regime;
- a forced buyer or a residual absorber at an auction, or an order without a limit;
- a yield that sets a price, or a curve fed its own output;
- a deficit target that sets anything;
- a default decided other than by the declared payment priority, or a statutory payment cut outside it.

**Done when**
- [ ] The treasury funds ahead of spending, handles failed auctions and defaults only where its declared priority,
  its regime and the market leave a debt service unpaid.
- [ ] LC-3-04 and LC-3-22 to LC-3-25 pass (secondary prints from S3.06).
- [ ] Two reviews are done.

---

### S3.04 — `sys-crd`: corporate and bank debt, underwriting and syndicated loans

**Status**: planned

**Clauses**:
- STATE: CRD.1; BNK.3; BFL.2 *(completes it: certificates of deposit, paper and bonds banks issue, beside S2.06's
  interbank loans and S3.01's repo)*.
- DECISION: CRD.2, CRD.3, CRD.4; FRM.9 *(part: bonds and commercial paper)*; BCP.4 *(part: contingent capital and
  subordinated debt)*.
- PROCESS: CRD.6, CRD.8; CRD.5 *(part: book-built issues, taps and paper placed by bilateral quote; paper sold
  through dealers completes it at S3.06)*; CRD.7 *(part: contingent capital; convertibles' conversion into shares
  completes it at S3.05)*.
- MEASURE: CRD.9.
- FORBID: CRD.10.
- PRIMITIVE: CRD.11.
- This step retires S2.07's placeholder naming CRD (a bank's decision to raise capital recorded and unmet) for
  contingent capital and subordinated debt; S2.03's `finance` gains bonds and paper as sources, as data; and banks
  can now share a loan (BNK.3), which S1.09 left out.

**Architecture**: §4.4 (commitments), §8, §9.1.

**Depends on**: S3.03.

**Goal**: firms and banks issue bonds, notes, paper, convertibles and contingent capital when they are the cheapest
money they can reach, through underwriters who choose their risk; investors bid their own values in book-built calls;
paper is rolled or fails to roll; missed payments and breaches are events holders act on; large loans are shared by
syndicates of named banks.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/debt.rs` | terms of bonds, notes, paper, convertibles, contingent capital |
| `crates/interfaces/if-securities/src/issue.rs` | mandate requests and answers, the book, the commitment |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `issue_debt`, `underwrite`, `invest_debt`, `holder_vote` |
| `crates/interfaces/if-credit/src/syndicate.rs` | the syndicated loan line's terms and its invitation message |
| `crates/systems/sys-crd/src/rules/issue.rs` | CRD.2: size, tenor, form, walk-away |
| `src/rules/invest.rs` | CRD.4: non-bank investors' values |
| `src/rules/holders.rs` | CRD.6: acceleration and waivers |
| `src/rules/convert.rs` | CRD.7: the convertible holder's choice |
| `src/handlers/5c_*.rs` | issue decisions, mandates, book orders, paper rolls, votes, conversions |
| `src/handlers/9c_covenants.rs` | bond covenant tests and contingent triggers, each with its event; applied at 9e |
| `src/handlers/2e_default.rs` | missed payments past grace become defaults |
| `crates/systems/sys-frm/src/rules/finance.rs` | S2.03's `finance`, its source list gaining bonds and paper (data) |
| `crates/systems/sys-bcp/src/rules/raise.rs` | S2.07's capital raising, now through issues |
| `crates/systems/sys-bnk/src/rules/{underwrite,invest_debt,syndicate}.rs` | CRD.3, a bank's CRD.4, BNK.3 (PC-40) |
| `data/<country>/CRD.toml` | fee points (POLICY of the trade); review hours (TECHNOLOGY); conventions (POLICY) |

**Design**

- **Instruments** (CRD.1): families of REG.5, declared as data — senior and subordinated bonds, floating-rate notes
  (issued from S3.09, whose benchmarks are the transacted references S0.14 requires), commercial paper, convertibles,
  preferred shares (S3.05) and contingent capital — each with covenants where its terms have them and a seniority the
  waterfall honours (L3).
- **The funding choice** (FRM.9, BCP.4), S2.03's `finance` for firms (in `if-credit`, its sources plain costs, so it
  stays there) and S2.07's review for banks:
  - the sources it can reach now include a bond, whose cost is its own outlook of the yield its paper would clear at
    (its outstanding bonds' latest prints or fixings; with none, comparable issuers of its own choosing, VAL.10) plus
    the fee it expects, and paper, at its outlook of paper yields; banks add certificates of deposit (paper of their
    own, BFL.2), contingent capital and subordinated debt, compared with shedding assets (S2.07);
  - the cheapest source within its management's leverage tolerance (or the bank's buffer target) wins; the choice is
    a firm's marginal cost of money, which CAP.3 and FRM.4 read, so a rate rise reaches a firm that borrowed years ago
    when it refinances (CRD.8, L4).
- **Issuing** (CRD.2), when a bond or paper wins:
  - size: the funding need (the programme, the maturing debt, the shortfall);
  - tenor: its management's preference for matching the life of what it funds (PREFERENCE), over the tenors the
    market's conventions allow;
  - form: the family whose cost is lowest after its terms (a convertible's lower coupon against the dilution it
    expects; a subordinated note's cost against the capital it counts as);
  - walk-away: its order in the book is the full size at prices at or above its reservation — the price at which the
    next-cheapest source would cost as much; a book clearing below it issues nothing (MKT.10);
  - the form — the financing choice by marginal cost within a leverage tolerance (Myers and Majluf, 1984; Graham and
    Harvey, 2001) — is listed in `SHAPES.toml`. A firm cell that issues a public instrument is promoted at issuance
    (REP.29).
- **Underwriting** (CRD.3), bilateral (MKT.7) across days: the issuer's mandate request at 5c to the banks it reaches
  (its lenders and its declared reach); each answers at the next 5c; the issuer accepts at the 5c after.
  - A bank's answer (`underwrite`, `sys-bnk`'s rule): **underwritten** when its fee covers the value of the put it
    writes — its expected loss on unsold paper, the paper valued by `loan_claim_value` under its `LoanAssessment` of
    the issuer against its outlook of the book's demand (its own record of recent books' cover and prices) — plus its
    operating cost and its required return on the capital the unsold paper would consume (S2.07's `capital_charge`),
    within its position limit; otherwise **best effort** at its operating cost; or no answer. Fees are on the trade's
    fee points. The form (Smith, 1977) is listed.
  - A size beyond the lead's limit is shared by a **syndicate**: the lead invites banks by message, each answering by
    the same rule within its own limit.
  - The issuer takes the answer with the lowest all-in cost to it, ties by lot (stream `CRD.mandate_lot`).
- **The book** (CRD.5): a call auction at 6a on the announced day, on the underwriter's book.
  - Investors' orders (CRD.4, `invest_debt`) are their investor schedules. A bank's value is `sys-bnk`'s:
    `loan_claim_value` under its `LoanAssessment` of the issuer, discounted at its marginal cost of funds (S2.06) plus
    S2.07's capital charge times its required return. Every other investor's is `sys-crd`'s: the claim value at its
    own outlook of the issuer's default — classes by steps of its filed or reported ratios (and, from S3.10, its
    rating), each class's default frequency an adaptive outlook of defaults seen or published, VAL.10 at the opening
    — and its own recovery outlook by seniority, at its cost of funds, within its mandate (declared data).
  - A bank's bonds held to collect are carried at amortised cost with an expected-loss provision from the same
    assessment, moved at 9a by S2.01's provision rule over its holdings (BNK.15, ACC.7).
  - Under an underwritten basis, the underwriting commitment (a commitment, REG.10) is the underwriters' order for the
    whole size at the committed price: a limit they chose, carried as a real risk. Under best effort they post none,
    and what does not sell is not issued (CRD.10).
  - Rationing at the price by largest remainder, ties by lot (stream `CRD.book_lot`). A tap adds to an existing
    line by the same process.
- **Paper** (CRD.5): rolled at maturity by the issuer's own decision; placed by bilateral quote to the cash
  investors it reaches (firms' `place_cash`; money funds from S3.07), answered at the next 5c from their schedules,
  the best taken up to its need, ties by lot (stream `CRD.paper_lot`); through dealers from S3.06. Each answer is
  the investor's schedule at its own value, the form listed with S3.03's. Paper that cannot be rolled leaves a
  maturing payment the issuer must meet from cash, lines or distress (FRM.12).
- **Events** (CRD.6):
  - a missed payment is recorded at 2d against the instrument's event and, past the terms' grace, is a default at
    2e; a breached covenant is found at 9c by one handler that tests the issuer's published statements (filed
    accounts, S2.10, in `if-firm`; reports, S3.10) and issues the event, applied at 9e; each is a public event
    holders see;
  - each holder then answers a vote by message: accelerate when its value of claiming now (its recovery outlook ×
    face) exceeds its value of waiting (the claim value at its revised assessment), or accept a waiver when the
    issuer offers a consent fee worth more to it than the difference; the issuer offers a fee when avoiding
    acceleration is worth more to it than the fee. The terms' threshold by face decides (the form is listed);
  - default makes the claim a claim on the estate (S2.04) or a class in the restructuring vote (S2.03). While the
    issuer's procedure is open its bonds' events are suspended — the instrument-side counterpart of S0.17's procedure
    lines. The default and the suspension are event intents `sys-crd` declares; the instrument's state has one
    writer, `phx-ledger`'s `events.rs` (S0.14), and every system reads it.
- **Conversion** (CRD.7): a convertible's holder converts in the terms' windows when the shares it would get are worth
  more to it than the bond (its own values; Ingersoll, 1977, listed), from S3.05 when shares trade. Contingent capital
  converts or writes down when its trigger ratio, read at 9c from the issuer's published statements (S2.07's ratios
  as the bank publishes them), crosses the stated level: the test and the event are one handler, applied at 9e, and
  the event applies to holders of record at the start of the next business day (TIME.7, REG.11). In resolution the
  authority converts or writes it down first (S2.08).
- **Syndicated loans** (BNK.3): a loan request beyond the lead bank's own limit (its large-exposure limit, BCP) is
  arranged: the lead invites banks by message, each joins when the terms' rate is at least its own quote for that
  borrower (BNK.4, under its own `LoanAssessment`) and takes a share within its limit — the form, a participant
  joining at terms that beat its own quote (Sufi, 2007), listed in `SHAPES.toml`; oversubscription is cut pro
  rata, ties by lot (stream `BNK.syndicate_lot`). The loan is one loan line whose lender side has a row per lender of
  record, each row's balance its share; payments go pro rata to those rows; each lender provisions and works out its
  own row by S2.01, bound by the terms' majority clause for waivers and enforcement; the lead takes the terms' fee; a
  row can be sold (S2.01's line transfer, BNK.10).
- **Streams**: `CRD.mandate_lot`, `CRD.book_lot`, `CRD.paper_lot`, `BNK.syndicate_lot`.
- **Review costs** (TECHNOLOGY, hours): `CRD.issue_review_hours`, `CRD.underwrite_hours`, `CRD.invest_review_hours`,
  `CRD.vote_hours`, `BNK.syndicate_hours`.
- **Opening** (GEN): outstanding corporate and bank bonds, paper and syndicated loans with their holders, from
  issuance data; the history of their prints (GEN.5).

**Unit tests**
- `funding_choice_cheapest_within_tolerance`.
- `walk_away_below_reservation`.
- `underwritten_fee_covers_put_value`.
- `underwritten_book_leaves_unsold_on_underwriter`.
- `best_effort_leaves_nothing_on_agent`.
- `investor_claim_value_by_seniority`.
- `acceleration_by_threshold`.
- `contingent_trigger_from_published_ratio`.
- `syndicate_shares_within_limits_sum_to_loan`.

**Live checks**
- `LC-3-05`: CRD.9 — spreads by issuer class and seniority, their widening in downturns and after similar names'
  defaults, the clustering of defaults and the dispersion of recoveries are reported.
- `LC-3-26`: CRD.10 — every underwritten issue's unsold amount sits on its underwriters' books; no best-effort agent
  holds paper from its own book; every issue names its underwriters' answers.
- `LC-3-27`: CRD.6 — every acceleration follows a public event and names its holders' vote; every contingent
  conversion names the published ratio that triggered it.
- `LC-3-28`: BNK.3 — every syndicated loan's shares sum to its principal, each within its lender's limit.
- `LC-3-48`: CRD.6 — after each default, the reassessments of other issuers' holders and investors (their classes'
  default outlooks revised, VAL.4) and the change in those issuers' spreads are traceable in events; a run with no
  default passes it vacuously, and the report says so.

**Budget**
- About twenty books a business day across the countries, each up to 10³ orders at 300 ns; about 500 paper
  placements a day at 2 µs; covenant tests on statement days (about 2 000 on a heavy day at 1 µs); investors' claim
  values only for the instruments in their holdings and candidate lists, about 20 k a day, closed-form at 100 ns.
- The ledger's line: 3 ms on a business day ((6 + 1 + 2) core-ms ÷ 3), 6 ms on a heavy day; bond covenant tests in
  the valuation line.
- Counters, ratcheted: `phx_crd.issues`, `phx_crd.walkaways`, `phx_crd.book_orders`, `phx_crd.paper_rolls`,
  `phx_crd.paper_failures`, `phx_crd.defaults`, `phx_bnk.syndicated_loans`, `phx_val.claim_values`.

**Guards**: none new.

**Not allowed**:
- a price from a spread;
- a fixed recovery;
- a seniority that changes the price but not the payout;
- an underwriter that carries no risk, or a best-effort agent left holding paper;
- a firm's cost of debt read from its old coupons.

**Done when**
- [ ] Firms and banks issue, roll and default on debt, and a default moves others' spreads through investors'
  reassessments.
- [ ] LC-3-05, LC-3-26 to LC-3-28 and LC-3-48 pass (convertibles' conversions from S3.05).
- [ ] Two reviews are done.

---

### S3.05 — `sys-eqy`: equity, households' holdings, groups and instrument events

**Status**: planned

**Clauses**:
- STATE: EQY.1, EQY.2; PTY.7; FRM.3; ACC.5.
- DECISION: EQY.4; EQY.3 *(part: investors' own values and orders; index trackers complete it at S3.09)*; EQY.5
  *(part: votes at meetings; takeover votes complete it at S4.06)*; FRM.9 *(completes it: shares)*; FRM.10
  *(completes it: buybacks)*; BCP.4 *(completes it: shares)*; HH.7 *(part: shares and bonds held directly; funds are
  S3.07's part, and HH.7 completes at S6.03)*.
- PROCESS: EQY.7, EQY.8; EQY.6 *(part: the continuous book; market makers' quotes complete it at S3.06)*; EQY.9
  *(part: offers only of covered units; borrows complete it at S3.06)*; CRD.7 *(completes it: convertibles convert
  into shares)*; REG.11, REG.12.
- MEASURE: EQY.10.
- FORBID: EQY.11.
- PRIMITIVE: EQY.12.
- This step retires S2.07's placeholder naming EQY (a bank's decision to raise capital recorded and unmet) for
  shares; S2.07's placeholder naming EQY (a bank's group is the bank alone), since the group fact exists; and S2.04's
  placeholder naming EQY (estates' securities with no market) for shares, which estates now sell on their books;
  bonds follow at S3.06. S2.03's `finance` gains shares as a source, as data.
- It introduces one placeholder naming RAT (S3.10): listing reads a firm's filed accounts (S2.10, in `if-firm`) as
  its reports.

**Architecture**: §4.2 (parts at the fill), §4.5 (holdings), §7.5, §7.6 and §7.7 (participation in the key), §8,
§6.1 (stages 2, 5, 6 and 9).

**Depends on**: S3.04.

**Goal**: shares as residual claims with votes, held by named holders — households among them, choosing what to hold
by their own values — issued and bought back by firms when that is their cheapest course, priced on continuous books
by investors with different views; dividends, splits, buybacks and every other instrument event land on holders of
record; groups are read from holdings and report consolidated statements; shareholders are paid last.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/equity.rs` | share classes, free float, listing, events, resolutions |
| `crates/interfaces/if-base/src/participation.rs` | the participation key attribute of household and firm cells: one bit per asset class (shares, bonds, fund units) |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `choose_holdings`, `invest_equity`, `issue_equity`, `vote` |
| `crates/interfaces/if-firm/src/equity.rs` | the group fact; S2.03's `payout` gains buybacks (kernel orders) |
| `crates/kernel/phx-ledger/src/events.rs` | S0.14's instrument events, gaining every event kind of REG.11 and REG.12 |
| `crates/systems/sys-eqy/src/rules/value.rs` | EQY.3: firm values for orders |
| `src/rules/issue.rs` | EQY.4: offerings |
| `src/rules/vote.rs` | EQY.5 |
| `src/rules/control.rs` | PTY.7, FRM.3: control through holdings |
| `src/handlers/5c_*.rs` | orders, offerings, payouts, votes on meeting days, conversions |
| `src/handlers/9b_groups.rs` | the group fact at statement dates |
| `crates/systems/sys-frm/src/rules/payout.rs` | FRM.10: dividends (S2.03) and buybacks |
| `crates/systems/sys-hh/src/rules/portfolio.rs` | HH.7: participation and instruments held |
| `data/<country>/EQY.toml` | listing and short-sale rules, ticks, meeting windows, thresholds (POLICY) |
| `data/<country>/HH.toml` | adds the portfolio review's schedule and hours, entry hours, reach (TECHNOLOGY) |

**Design**

- **Holdings of cells** (HH.7, architecture §4.5, §7.6, §7.7):
  - a cell's **participation** — whether its members hold shares, bonds or fund units directly, one bit per asset
    class — is a key attribute in the key record, as its banking arrangement is, so a cell's distinct keys grow at
    most 8× per base key; the distinct-key curve is measured here on the ladder (`phx_pop.distinct_keys`);
  - its instruments are held as rows with a member count (S0.14's `CellHolding`), like relationship rows: members of
    a participating cell may hold different instruments of the class, and each row's per-member quantity is a
    position with steps in the landing key, so no landing joins holdings of different sizes;
  - dividends, coupons and other income reach a row's members as pooled flows (REP.8);
  - members whose fill changes their participation, or leaves them at a step other than their row's holders', make
    their part at the fill (6d), pinned by the trade's commitment until it settles (architecture §4.2), and are
    re-keyed in place at 7c when it settles; members who enter a row at its holders' step join it without a part; an
    unfilled order makes none, and a partial fill adds to the part's row;
  - estates are one per (part, occasion) (S2.04), so a cell's members ending together bring their share of each row
    to one estate.
- **Households' choice** (HH.7), a new lumpy kind, the decision point `choose_holdings` (`if-securities`, where its
  types are; the rule `sys-hh`'s), with attention (REP.21):
  - **Inputs**: its wealth beyond the buffer target of S1.12's spending rule (so liquidity comes first); its
    outlooks of each class's return and width — for shares, its method's registered outlooks and values of the
    instruments in its reach, computed once at 5a and shared by every cell using the method; its deposit rates and
    bill yields; its trust, which is its own record: losses it took on a class widen its width (VAL.9); its bank's
    brokerage fee points; its risk aversion.
  - **Classes**: mean–variance weights over its own outlooks, `w = (E[r] − r_d) ÷ (γ·σ²)` of the wealth beyond its
    buffer (Merton, 1969), entering a class only when the certainty-equivalent gain exceeds the hours of opening it
    (`HH.brokerage_entry_hours` at its value of leisure; Vissing-Jørgensen, 2002). The entry threshold is a
    key-rule kink in the landing key's signature (REP.16), and the decision gap at it is measured (REP.15).
  - **Instruments**: among the listed shares and bonds in its reach — a declared search cost (TECHNOLOGY), counted
    and tested on the ladder — by logit over its own value's excess over price; the members' tastes are one
    multinomial draw over their counts per (cell, profile) (REP.22, stream `HH.holding_taste`), about 1 µs a choice.
    Members choosing differently hold different rows.
  - Its monthly savings decision (S1.12) keeps the amounts within the instruments held, with the no-trade band its
    fees give (S3.03's schedule), so a household trades rarely.
  - Orders go through its bank as broker: limit orders at its own value, count × per-member quantity.
  - The form is listed in `SHAPES.toml`; the review cost is `HH.portfolio_review_hours`.
- **Other investors** (EQY.3), at 5c on their review days: banks within their regulated portfolios, firms (stakes and
  treasury), funds from S3.07, market makers from S3.06. A share's value is VAL.8's firm value: its own outlook of
  the distributions or earnings (reports and guidance as published), discounted at its own required return — its
  outlook of short rates (a bank's or a firm's, its cost of funds) plus a risk premium from its risk aversion and
  its outlook's width — or comparable prints where it has no history (VAL.10). The order is its investor schedule
  (S3.03). Index trackers trade from S3.09.
- **The book** (EQY.6, MKT.4): each listed share has a continuous book meeting at 6a. Orders posted at 5c are
  `Continuous` or `AtTheClose` by the poster's choice (index trackers trade at the close); `Continuous` orders arrive
  in an order drawn by lot (stream `EQY.arrival_lot`) and trade at the resting price in price–time priority; the
  residue and the `AtTheClose` orders meet in the closing call, rationed by lot (stream `EQY.close_lot`), whose price
  is the day's close and its mark (6b). Settlement on the exchange's convention.
- **Shorts** (EQY.9): an offer takes a `Covered<Qty>` (S0.14), which `phx-ledger` builds only from free units held or
  borrowed (PC-53). Borrows exist from S3.06; until then no short can be posted.
- **Listing** (EQY.2): a firm is listed when its shares trade on a book; listing requires the exchange's declared
  conditions (a free float, published reports: POLICY, EQY.12). Until S3.10, reports are its filed accounts — the
  placeholder naming RAT.
- **Issuing** (EQY.4, FRM.9, BCP.4), on the funding review: shares compete with S3.04's sources. Their cost to the
  firm is its outlook of the price it would clear at against its management's own value per share (issuing below its
  own value transfers value to buyers), plus fees. It issues when it has a programme to fund and equity is its
  cheapest acceptable source (market timing, Baker and Wurgler, 2002; listed with S3.04's form). An offering is
  book-built as in S3.04 (underwriters, the call at 6a, ties by lot: stream `EQY.offering_lot`); an initial offering
  also lists the share; a cell firm is promoted at issuance (REP.29). An issue can fail (BCP.4).
- **Payouts** (FRM.10): S2.03's `payout` — the dividend moving from the last toward the free cash `D*` by the
  management's adjustment speed, within distributable reserves — gains buybacks: the free cash beyond the dividend
  buys back shares when the price is below the management's own value per share (Ikenberry, Lakonishok and
  Vermaelen, 1995), by limit orders on the book at that value, or is retained. A listed firm's dividends carry record
  and payment dates at least the exchange's notice apart and are paid as REG.11 events. The buyback form is listed
  beside S2.03's.
- **Votes** (EQY.5), on meeting days (each firm's, within the declared window after its year end): resolutions on
  distributions and on the authority to issue or buy back. Each holder of record at the record date votes when its
  stake times its value difference exceeds its cost of voting (its hours, `EQY.vote_hours`, at its value of time;
  Downs, 1957, the form listed in `SHAPES.toml`), for when its own value is higher with the resolution. The voting
  threshold is a key-rule kink (REP.16), so members either side of it never join. The pass runs instrument-major
  over the holder list with the pure rule, so abstaining cells cost one read. Resolutions pass by the declared
  majorities of votes cast (POLICY of company law). Takeover votes are S4.06's.
- **Conversions** (CRD.7): S3.04's convertible holders convert from here when the shares they would get are worth more
  to them than the bond; the conversion is a unit-only event at 2c, the bond's units retired and new shares issued.
- **Events** (REG.11, EQY.7): dividends, coupons, amortisation, calls, conversions, splits, buybacks, write-downs,
  maturities and defaults apply to the holders of record at the start of their day (the holder list as it stood at
  1a), through `phx-ledger`'s `events.rs`. Money legs settle at stage 7, unit-only events (a split, a conversion, a
  write-down) at 2c, rounded by the terms with cash in lieu to named parties. Issuance dilutes and buybacks cancel
  (issued amounts move, REG.3).
- **Ceasing** (REG.12): a matured, redeemed, converted or resolved instrument resolves every holding to something
  named — cash, another instrument, a recovery claim or a recorded loss; a holding left unresolved is a violation of
  REG.12. In insolvency shares are paid after every creditor (EQY.8, S0.17's waterfall) and go to zero, never below.
- **Groups** (PTY.7, FRM.3, ACC.5), at 9b on statement dates: a party controls another when the votes it holds, with
  those held by parties it controls, exceed half — the least fixed point over the holdings graph of individuals, in
  identity order. The group fact is a read. Intra-group loans, sales and guarantees are ordinary contracts; each
  member keeps its own books and limited liability. A group's consolidated statement is S0.19's pure read over the
  members the fact names, taken at 9c by the tests that need it — BCP's requirements on a bank's group (S2.07),
  whose placeholder naming EQY this retires — and by reports (S3.10).
- **Streams**: `EQY.arrival_lot`, `EQY.close_lot`, `EQY.offering_lot`, `HH.holding_taste`.
- **Review costs** (TECHNOLOGY, hours): `HH.portfolio_review_hours`, `HH.brokerage_entry_hours`,
  `EQY.invest_review_hours`, `EQY.payout_review_hours`, `EQY.vote_hours`.
- **Opening** (GEN): listed shares with their holders, free floats and prints' history; households' direct holdings
  and participation by wealth and age from survey data; opening dividend policies (the last dividends paid).

**The household cell's record**: `choose_holdings` adds one lumpy kind — its review exposure (8 bytes) and its
attention (8 bytes: the own rate and g_k, as for every household kind, S1.12) — so the record grows from 568 bytes
after Stage 2 to **584**. Participation is three bits of the key record, not a word of the cell. The record's size
is a ratchet move the owner reviews (§2.11).

**Unit tests**
- `participation_in_key_blocks_mixed_joins`.
- `holding_row_step_in_landing_key`: members at different per-member quantities of one instrument never join.
- `entering_at_holders_step_makes_no_part`.
- `portfolio_weights_mean_variance_beyond_buffer`.
- `participation_only_above_entry_cost`.
- `no_trade_band_from_fees`.
- `required_return_short_rate_plus_premium`.
- `book_price_time_and_closing_call`.
- `offer_refuses_uncovered_quantity` (compile-fail).
- `lintner_adjustment_and_buyback_below_value`.
- `vote_only_above_cost`.
- `holders_of_record_at_day_start`.
- `split_rounding_cash_in_lieu`.
- `control_least_fixed_point_with_cross_holdings`.
- `insolvency_pays_shareholders_last`.

**Live checks**
- `LC-3-06`: EQY.10 — fat tails, little autocorrelation, volatility clustering and its rise after falls in returns,
  and price moves on earnings surprises, are reported (measured, never enforced).
- `LC-3-29`: EQY.11 and REG.16 — no offer without covered units; no holding below zero; no income to a holder but by
  a distribution.
- `LC-3-30`: REG.11 and REG.12 — every event paid its holders of record at the start of its day; every ceased
  instrument's holdings resolved to something named.
- `LC-3-31`: ACC.5 — every group's consolidated statement eliminates its intra-group positions and passes the Accounts
  family.
- `LC-3-32`: EQY.8 — in every insolvency, shareholders received nothing until every creditor class was paid.
- `LC-3-49`: HH.7 — every cell's holding row traces to its members' `choose_holdings` decision or to the opening
  (GEN), through the fills and parts that carried it; the distinct keys per base key are reported by participation.

**Budget**
- About 1 500 listed shares; about 24 000 orders a business day (4 000 from households, whose fees keep them rare) at
  300 ns; closing calls at 3 µs each; votes on meeting days over up to 0.5 M holder rows at 10 ns; groups at
  statement dates and consolidation when read (the valuation line).
- Households' `choose_holdings` reviews are about 30 k a day (monthly reviews over 0.7 M cells) at 80 ns, of which
  about 10 k choose instruments, a multinomial over the reach at about 1 µs; parts about 2 000 a day, members
  changing participation or leaving their row's step.
- The ledger's line: 11 ms on a business day (24 k orders at 300 ns, 1 500 closing calls at 3 µs, 30 k reviews at
  80 ns, 10 k choices at 1 µs, 2 k parts at 2.5 µs and 30 k institutional values at 100 ns: 32 core-ms ÷ 3), 16 ms
  on a heavy day (dividends over about 1 M holding rows and votes over 0.5 M holders, at 10 ns); the parts at
  F-001's 12.1 µs add 6 ms.
- Memory: about 1 M household holding rows (30 MB with holder lists); the household record at 584 bytes (+11 MB);
  household keys estimated a third more with participation (18 MB), at most 8× per base key.
- Counters, ratcheted: `phx_pop.bytes_per_household_cell` (at 584), `phx_pop.distinct_keys` (participation),
  `phx_pop.holding_parts`, `phx_pop.holding_joins_without_part`, `phx_hh.portfolio_reviews`,
  `phx_hh.holding_choices`, `phx_eqy.book_orders`, `phx_eqy.closing_calls`, `phx_eqy.offerings`, `phx_eqy.buybacks`,
  `phx_eqy.votes_cast`, `phx_ledger.instrument_events`.

**Guards**: PC-53: `phx_market::Order`'s offer side takes only `Covered<Qty>`, whose constructor is private to
`phx-ledger` (a compile-level refusal of a short without a borrow).

**Not allowed**:
- a price from a multiple, a book value, a discounted cash flow or a target;
- income to a shareholder from earnings not distributed;
- a short without a borrow;
- a household holding what it did not choose, two cells with different participation joined, or members at
  different steps of one row joined;
- a vote counted from holders who were not of record.

**Done when**
- [ ] Shares trade from investors' own values; households hold what they chose; firms issue, pay out and buy back by
  their own decisions; groups consolidate; events land on holders of record.
- [ ] LC-3-06, LC-3-29 to LC-3-32 and LC-3-49 pass.
- [ ] PC-53 is registered.
- [ ] Two reviews are done.

---

### S3.06 — `sys-dlr`: dealers, primary dealers, securities lending and prime brokerage

**Status**: planned

**Clauses**:
- STATE: DLR.1, DLR.2, DLR.3; BFL.3 *(completes it: securities' market depth — the dealer markets' published depth
  and turnover (MKT.15) — in a bank's liquid assets, their haircuts and its buffer, S2.06)*.
- DECISION: DLR.4, DLR.5, DLR.6; SOV.5 *(completes it)*.
- PROCESS: DLR.7, DLR.8, DLR.9; MKT.5 *(completes it: dealers' quotes)*; SOV.7 *(completes it: bonds and bills trade
  in the dealer market, and the curve reads its fixings)*; EQY.6 *(completes it: market makers' quotes on the share
  books)*; EQY.9 *(completes it: shorts on borrowed shares, paying for the borrow)*; CRD.5 *(completes it: paper sold
  through dealers)*; CB.5 *(completes it: the central bank's purchases and sales trade through dealers)*.
- MEASURE: DLR.10.
- FORBID: DLR.11.
- PRIMITIVE: DLR.12.
- This step retires S1.11's placeholder naming DLR (the named banks as primary dealers), the Stage 1 rule that bills
  are bought only at auction and held to maturity (S1.11, S1.12), and what remained of S2.04's placeholder naming
  EQY: estates now sell bonds through dealers.

**Architecture**: §4.1 (the dealer kind), §8, §9.1, §9.3 (a call unmet), §6.1.

**Depends on**: S3.05.

**Goal**: dealers are parties of the dealer form with their own equity, inventory, a daily funding cost, a capital
charge and limits, quoting both sides from their own state and stepping back when a limit binds; bonds, bills and
paper trade on their quotes and between dealers; primary dealers bid in every auction from their own capacity;
securities are lent against collateral for a fee that rises as supply runs short; prime brokers set margin on whole
portfolios and close out clients who cannot meet a call.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/dealer.rs` | the dealer form, desk facts, quotes, the primary-dealer agreement |
| `crates/interfaces/if-securities/src/lending.rs` | the securities loan line kind; availability; recall |
| `crates/interfaces/if-securities/src/prime.rs` | the prime-brokerage account, requirement, calls, close-out |
| `crates/systems/sys-dlr/src/rules/quote.rs` | DLR.4, on bond, bill and paper markets and on share books |
| `src/rules/interdealer.rs` | DLR.7 |
| `src/rules/lend.rs` | DLR.5: availability, fee and recall |
| `src/rules/borrow.rs` | a borrower's choice of lender |
| `src/rules/margin.rs` | DLR.6 |
| `src/rules/closeout.rs` | DLR.8: close-out orders |
| `src/rules/relationship.rs` | a prime broker ending a relationship |
| `src/rules/pd_bid.rs` | SOV.5 |
| `src/handlers/5c_*.rs` | quotes, interdealer orders, availability, recalls, buy-ins, close-outs, PD bids |
| `src/handlers/9c_margin.rs` | the margin test and its call in one handler, applied at 9e |
| `src/fixing.rs` | the pricing service's declared fixing method, which `phx-market` applies at 6b |
| `data/<country>/DLR.toml` | limits and capital allocations (POLICY, DLR.12); scenarios; conventions |

`DLR.toml` also holds the brokers' margin scenarios, close-out horizons and urgency concessions (POLICY of each
broker), the pricing service's fixing method (POLICY of the service), fee points, recall notice and buy-in rules
(POLICY of the market), clients' reach (TECHNOLOGY) and review hours (TECHNOLOGY).

**Design**

- **Desks** (DLR.1): a desk is a party of the dealer form — a subsidiary of a bank or an independent broker-dealer,
  its form declared — with its own equity (for a subsidiary, its parent's shares in it) and its own books. Its
  inventory is funded by its equity, by an intra-group line from its parent (a loan line at the parent's marginal
  cost of funds, S2.06's rule handle, accrued daily and paid on the line's dates) or its lenders, and by repo at stage
  8 as a borrower (S3.01). Its capital charge is its own: its parent allocates it a limit on risk-weighted inventory
  (POLICY of the bank), within its equity; the parent's requirement is tested on the consolidated group (S2.07,
  S3.05). Position limits per instrument and in total are POLICY of the bank and the supervisor (DLR.12), read as
  `DeclaredLimit`s. A desk can fail: its losses fall on its equity, and one that cannot pay its line or its repo ends
  as a firm does (FRM.15, L3), its parent losing its shares and its line's shortfall.
- **Quotes** (DLR.4), at 5c each business day, for each instrument the desk covers — those of its declared asset
  classes it holds or clients asked it about within its record's window:
  - its view `v` is its own value (VAL.8's claim value at its own outlooks, or its heuristic outlook of the
    instrument's prints), never a market mid. A desk is its own party, carrying a trading book at fair value, not a
    lender's book, so its values are its own (PC-40 reaches its parent's lending, not its inventory);
  - the reservation price is `v − q·γ·σ²·τ` for inventory `q`, so long inventory lowers both sides;
  - the half-width is `½·γ·σ²·τ + (1/γ)·ln(1 + γ/κ) + a`, where κ is its adaptive outlook of how fills respond to its
    width and `a` its outlook of the loss per unit after a client trade (its markouts), so risk and adverse selection
    widen it;
  - each side's size is its headroom under the instrument's limit, the total limit, the capital allocation and its
    funding, bound by `DeclaredLimit::bind`; a side whose headroom is zero is not quoted;
  - γ and τ are PREFERENCE of the desk's management; σ² is its outlook width;
  - the form (Ho and Stoll, 1981; Glosten and Milgrom, 1985; Avellaneda and Stoikov, 2008) is listed in
    `SHAPES.toml`; a quote is two limit orders, and the bid–offer is an output (MKT.19).
- **Market makers** (EQY.6): a desk covering listed shares posts the same quote as two limit orders on each share's
  continuous book (S3.05), its offer side a `Covered<Qty>` of units held or borrowed.
- **The dealer market** (MKT.5), at 6a: clients' requests — each an order to the dealers of its reach (a declared
  count per client kind, TECHNOLOGY) with its own limit — are taken in an order drawn by lot (stream `DLR.rfq_lot`);
  each takes the best quote among its dealers with size left, ties by lot (stream `DLR.client_tie_lot`); a request no
  quote meets within its limit is unfilled, and a market with requests and no trade records its failure (MKT.10).
  Clients now include the central bank's purchases (S3.02), households selling bills through their bank, and paper
  sold through dealers (S3.04).
- **The interdealer market** (DLR.7): each desk posts at 5c an order moving its inventory toward its target (zero, or
  its declared carry position) at its own reservation price; they meet in a call per instrument at 6a (MKT.3),
  rationed by lot (stream `DLR.interdealer_lot`). The form (Ho and Stoll, 1983) is listed with the quotes'.
- **Fixings** (MKT.12, MKT.20), at 6b: `phx-market` fixes each instrument that traded by the pricing service's
  declared method (the pricing service a large firm with the publisher facet, opened by GEN): the volume-weighted
  mean of its client and interdealer prices; with no trade, no fixing, and the last shows its age. The curve (S3.03)
  reads the day's fixings at 6c, a valuation input; a valuer marks an untraded bond from it, labelled interpolated.
- **Primary dealers** (SOV.5): a primary-dealer agreement is a line between the treasury and a desk. Its privileges
  are declared data (access to the treasury's operations); its undertaking is a bid in every
  auction. Every other bidder bids for itself through `bid_at_auction` with its own limit orders. The bid is the
  desk's investor schedule at its own value, bound by its limits and inventory, so a desk at its limit bids small or
  low; nobody is obliged to take any quantity at any price. The treasury reviews membership yearly by its declared
  criteria (POLICY: a share of auctions won).
- **Securities lending** (DLR.2, DLR.5), a posted market (MKT.6) per security:
  - lenders (banks' and, from S3.07, funds' holdings within their mandates; insurers from S4.03) post at 5c the free
    units they will lend and a fee point, revised on their review: `fee = f_0·(1 + u ÷ (1 − u))^η`, where `f_0` is its
    reservation (its outlooks of the recall it may need and of the spread on reinvested collateral) and `u` its
    utilisation, lent over lendable; η is PREFERENCE of its management. The form (D'Avolio, 2002; Duffie, Gârleanu and
    Pedersen, 2002) is listed;
  - borrowers — desks delivering a short or covering a fail, hedge funds through their broker from S3.07 — take the
    cheapest in their reach, the form (search among a declared reach, Stigler, 1961) listed; capacity binds by lot
    (stream `DLR.borrow_lot`);
  - a loan passes title against cash or securities collateral at the lender's haircut (S3.01's form), with the fee
    accruing and every coupon or dividend manufactured back to the lender (REG.11);
  - recall: the lender recalls when its value of selling or voting the units exceeds the fee still to earn on the
    loan (Aggarwal, Saffi and Sturgess, 2015, listed); a recall is due the next business day (TIME.7); the borrower
    returns the units at 2c or is bought in: at 5c the lender posts buy orders limited by its declared buy-in horizon,
    charged to the borrower, and keeps the collateral until settled (DLR.9);
  - cash collateral the lender reinvests is its own position (S3.01's repo, S3.07's money funds), with its own risk.
- **Prime brokerage** (DLR.3, DLR.6, DLR.8), for clients from S3.07 (hedge funds):
  - the account holds the client's assets under a lien to the broker (REG.2), its cash loan (a loan line) and its
    borrowed securities;
  - at 9c the requirement is the largest loss of the account's positions over the broker's declared scenarios, each
    scenario's moves scaled by the broker's own outlook widths per class (POLICY scenarios, standard-portfolio
    analysis; Brunnermeier and Pedersen, 2009), plus an add-on for concentration (position against the market's
    turnover); it is computed per (broker, client) account and never nets across counterparties (DLR.11, PC-54);
  - the test and the call are one 9c handler: equity at 9a's marks below the requirement issues a call, applied at
    9e and due at the next business day's 2c (TIME.7);
  - unmet, the failure reaches the broker at once and it closes out at 5c: sells the client's positions and buys back
    its shorts by limit orders, each day's slice the remainder over the days left of its declared close-out horizon,
    limited at its outlook of the bid less its declared urgency concession; proceeds repay the broker, a shortfall is
    its loss and its claim on the client (L2's door of a margin call);
  - what is still unfilled when the horizon is exhausted passes to the broker by appropriation under its lien, at a
    named valuer's valuation, labelled (MKT.20), which settles the client's shortfall or excess; the broker then
    holds it as its own inventory, to sell by its own decisions;
  - the broker ends a relationship on its review when the client's fees no longer cover its expected cost of risk
    (Mitchell, Pedersen and Pulvino, 2007, listed);
  - raising requirements as widths grow is how margin can spiral; nothing scripts it.
- **Streams**: `DLR.rfq_lot`, `DLR.client_tie_lot`, `DLR.interdealer_lot`, `DLR.borrow_lot`.
- **Review costs** (TECHNOLOGY, hours): `DLR.quote_hours` per instrument quoted, `DLR.margin_hours` per account,
  `DLR.lend_review_hours`, `DLR.closeout_hours`.
- **Opening** (GEN): desks with their equity, inventories and parent lines; the primary-dealer agreements; lending
  programmes; the pricing service; the history of fixings (GEN.5).

**Unit tests**
- `quote_skews_with_inventory`.
- `quote_widens_with_risk_and_markouts`.
- `side_stops_at_binding_limit`.
- `client_takes_best_among_its_dealers`.
- `interdealer_moves_inventory_to_target`.
- `fixing_vwap_and_absent_without_trade`.
- `pd_bid_bounded_by_limits`.
- `lending_fee_rises_with_utilisation`.
- `recall_when_sale_or_vote_worth_more_than_fee`.
- `buy_in_charged_to_borrower`.
- `margin_scenarios_scale_with_widths`.
- `margin_per_account_no_netting` (compile-fail).
- `closeout_slices_over_horizon`.
- `closeout_remainder_appropriated_at_valuation`.

**Live checks**
- `LC-3-07`: DLR.10 — dealers' inventory, widths and capital use move together in stress; borrow fees against
  utilisation; margin raised in falling markets.
- `LC-3-08`: a margin call's chain — call, failure, close-out orders, sales, prints, others' requirements — is
  traceable in events from the first call to the last sale or appropriation; a run with no unmet call passes it
  vacuously, and the report says so.
- `LC-3-33`: DLR.11 — every quote names its desk's decision and lies within its limits; a desk with no headroom on a
  side posted nothing on it; every desk's funding cost was charged each day it held inventory; every desk's equity
  is its own equity account.
- `LC-3-34`: SOV.5 — every primary dealer bid in every auction, at size and price of its own.
- `LC-3-35`: DLR.9 — every recall was returned or bought in, and every buy-in's cost landed on its borrower.

**Budget**
- About 300 desks quoting about 30 000 (instrument, desk) pairs a business day at 150 ns, each with its value at
  100 ns; about 20 000 client requests at 500 ns; interdealer calls and fixings for about 10 000 instruments at
  300 ns; lending availability revised on review days; margin for about 2 000 accounts at about 4 µs (in the
  valuation line).
- The ledger's line: 7 ms on a business day ((4.5 + 3 + 10 + 3) core-ms ÷ 3), 10 ms on a heavy day; margin about
  3 ms of the valuation line.
- Counters, ratcheted: `phx_dlr.quotes`, `phx_dlr.sides_stopped`, `phx_dlr.client_requests`,
  `phx_dlr.interdealer_trades`, `phx_dlr.fixings`, `phx_dlr.borrows`, `phx_dlr.recalls`, `phx_dlr.buy_ins`,
  `phx_dlr.margin_calls`, `phx_dlr.closeouts`, `phx_dlr.appropriations`.

**Guards**: PC-54: `MarginRequirement` is constructed only by `sys-dlr`'s rule from one (broker, client) account's
positions and the broker's declared scenarios; no function takes two clients' positions (DLR.11: no margin that is
only a number, no netting across counterparties).

**Not allowed**:
- a dealer quoting because a market needs one, or a spread on a mid;
- a desk without its own equity or a funding cost, or one that cannot fail;
- a dealer profit without the inventory's gains and losses;
- a short without a borrow;
- a margin that is only a number, or netted across clients;
- a close-out order "at any price";
- a bidder at an auction whose order is not its own.

**Done when**
- [ ] Markets have depth only where dealers have capacity; a dealer at its limit steps back; a margin call can be
  followed from the call to the last sale.
- [ ] LC-3-07, LC-3-08 and LC-3-33 to LC-3-35 pass (prime brokerage's from S3.07).
- [ ] PC-54 is registered.
- [ ] Two reviews are done.

---

### S3.07 — `sys-fnd`: funds

**Status**: planned

**Clauses**:
- STATE: FND.1, FND.2.
- DECISION: FND.3, FND.12; FND.4 *(part: managers' investing from their own views, hedge funds' long and short
  positions; their derivatives complete it at S4.02)*; HH.7 *(part: money, bond and equity funds among savings
  choices)*; BFL.7 *(completes it: a money fund among a depositor's alternatives in S2.06's `bank_choice`)*.
- PROCESS: FND.5, FND.6, FND.7, FND.8; FRM.16 *(completes it: firms and funds found firms)*.
- INVARIANT: FND.9.
- MEASURE: FND.10.
- FORBID: FND.11.
- PRIMITIVE: FND.13.
- Hedge funds' derivative positions arrive with DRV and DRX (S4.01, S4.02); private-equity buyouts with MNA (S4.06).
- Dealing at the next value is the one stated exception of MKT.16, an administered price (MKT.8).

**Architecture**: §4.1 (the fund kind), §8 (administered dealing), §9.1 (fund estates), §6.1 (stages 5, 6, 9).

**Depends on**: S3.06.

**Goal**: pooled vehicles of every declared kind, whose investors own the result: flows chase what investors saw,
subscriptions become purchases and redemptions become sales into markets that must clear, with the cost of late
sales on those who stay; exchange-traded funds are kept near their value by authorised dealers with balance sheet
to spare; leveraged funds can be closed out by their brokers; managers launch funds that pay and close those that
do not.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/funds.rs` | mandates, units, dealing and stable-price terms, AP agreements, the fund's procedure |
| `crates/interfaces/if-securities/src/decisions.rs` | adds `invest_fund`, `manage_liquidity`, `launch_or_close` |
| `crates/interfaces/if-securities/src/fund_safety.rs` | the public-series outlook of each money fund's safety |
| `crates/systems/sys-fnd/src/rules/invest.rs` | FND.4 |
| `src/rules/liquidity.rs` | FND.6: the buffer, sales for redemptions, gates |
| `src/rules/choose.rs` | FND.3: institutions' and firms' subscriptions and redemptions |
| `src/rules/etf.rs` | FND.7: authorised dealers' creations and redemptions, and their sales of created units |
| `src/rules/launch.rs` | FND.12 |
| `src/handlers/9b_nav.rs` | net asset values from 9a's marks and labelled valuations |
| `src/handlers/5c_*.rs` | investing, liquidity, authorised dealers, launches and closures |
| `src/handlers/9c_calls.rs` | private-equity capital calls, issued at 9c, applied at 9e |
| `src/markets.rs` | each fund's dealing as an administered market (MKT.8) at 6a |
| `crates/systems/sys-hh/src/rules/portfolio.rs` | S3.05's choice, now with funds |
| `crates/systems/sys-frm/src/rules/found.rs` | S1.03's founding, now with firm and fund founders |
| `crates/systems/sys-bfl/src/rules/bank_choice.rs` | S2.06's `bank_choice`, with money funds among its alternatives |
| `crates/systems/sys-est/src/fund.rs` | fund estates |
| `data/<country>/FND.toml` | mandates and redemption terms (FND.13); fund regulation (POLICY); managers' types |

`FND.toml`'s managers' types (PREFERENCE) are risk aversion, buffer cover, liquidation order, tracking tolerance and
stop-loss; it also holds review hours (TECHNOLOGY).

**Design**

- **Kinds** (FND.1, FND.2, Law 10): a fund is a party of the fund kind whose mandate is declared data — what it may
  hold, its leverage limit, how it deals and redeems (frequency, notice, settlement, gates), a stable unit price where
  its terms promise one, committed capital and life for private-equity and private-credit funds, authorised dealers
  for exchange-traded funds. Its manager is a firm paid the fee its terms state. No mechanism asks which kind a fund
  is.
- **Dealing** (FND.5, FND.6, MKT.8, MKT.16): an open-ended fund stands behind dealing at its **next** net asset value
  (forward pricing), an administered price with a quantity response on both books: it issues and cancels units, and
  its authorised dealers create and redeem them. Orders taken at 5c of day D deal at 6a of the next business day at
  D's value, computed at 9b of D, and settle on the terms' convention; units are issued or cancelled at settlement.
  This is MKT.16's one stated exception: the value is a read of the prices the fund's holdings formed, which no
  order in the fund's own dealing forms.
- **Net asset value** (FND.5, FND.9), at 9b each business day, reading 9a's valuations: assets at their marks, or at a
  named valuer's valuation labelled with its prints' age, less liabilities (loans, accrued fees, redemptions
  payable), over units, rounded by the unit price's convention with the residue on the fund. A stale print gives a
  stale value, and dealing at it moves value between investors. Published at 9d.
- **Stable prices** (FND.11): a money fund whose terms promise a stable unit price deals at it while its value lies
  within the terms' band; outside, the break is a public event and dealing moves to the value, as its terms state.
- **Investors** (FND.3):
  - households choose funds within S3.05's `choose_holdings`, as instruments of their reach: a fund's expected return
    is their method's outlook over its published value series (adaptive or trend, so flows chase performance) less its
    fee; withdrawals for cash needs come through their monthly savings decision;
  - firms through `place_cash` (money funds against deposits and bills); banks and, from Stage 4, insurers and
    pensions through their own portfolio decisions;
  - **depositors** (BFL.7): S2.06's `bank_choice` gains each money fund in the depositor's reach as an alternative,
    valued at its outlook of the fund's yield less the expected loss from a break of its stable price — a
    public-series outlook of the fund's safety per method, like S2.06's of banks — less the switching hours. A move
    is a subscription dealt at the fund's value; the movers' part is made at the fill (6d), pinned until it settles,
    and re-keyed in place at 7c with its new participation (S3.05);
  - institutions' form: mean–variance class weights over their own outlooks, funds within a class by expected return
    net of fee (Sirri and Tufano, 1998; Berk and Green, 2004), listed in `SHAPES.toml`.
- **Managers' investing** (FND.4), on their review days and on wakes (flows, margin calls, losses):
  - active funds post their investor schedules at their own values, within the mandate's limits;
  - trackers hold their index's constituents at its weights and rebalance when the tracking difference exceeds their
    tolerance, by `AtTheClose` limit orders within that tolerance (EQY.3); market indices are S3.09's, so trackers
    trade from then (spec Part O's rule for a later system of the stage);
  - hedge funds take long and short positions on their views (shorts borrowed through their prime broker, S3.06),
    leverage from named lenders only (FND.11), and cut positions when their broker calls margin or losses reach their
    stop (PREFERENCE);
  - subscriptions are invested per the mandate at the next 5c.
- **Liquidity and redemptions** (FND.6), on dealing days:
  - the cash buffer target is `z_f` times its outlook width of daily net flows (Chernenko and Sunderam, 2016;
    `z_f` PREFERENCE);
  - redemptions are paid from the buffer; beyond it the fund sells by its declared liquidation order (pro rata or
    most liquid first, PREFERENCE) with limit orders, draws a credit line where it has one (a real limit), and where
    its terms allow applies a gate: the part beyond the gate is deferred to the next dealing day as a claim, pro rata
    with ties by lot (stream `FND.gate_lot`), never dropped;
  - a redemption it cannot pay on its date fails at stage 7 and opens the fund's own procedure, as its terms and law
    declare: dealing is suspended, the unpaid redemptions become claims ranking as the terms say, and the fund sells
    by its liquidation order;
  - the fund ends by L3's trigger — its assets below what it owes its lenders, or its units reaching zero — into one
    estate (below);
  - since redeemers are paid at the value of their dealing day, the cost of later sales falls on those who stay —
    the first-mover advantage from which runs come (Chen, Goldstein and Jiang, 2010). Nothing scripts a run.
- **Exchange-traded funds** (FND.7): units list on a book (S3.05). An authorised dealer (a desk with an agreement)
  decides at 5c to create when the last close exceeds the last value by more than its all-in cost (buying the basket
  at its outlook, funding it) within its limits, or to redeem when below; creations and redemptions are in kind at the
  fund's next value (administered) at the next 6a, ties by lot (stream `FND.creation_lot`). The units it creates are
  its inventory, sold on the fund's book by its own quotes (S3.06), and the units it redeems are bought there the
  same way, which is how the gap narrows. The form (Petajisto, 2017) is listed. Price and value are two numbers, and
  the gap is an outcome.
- **Private equity** (FND.8): committed capital is a commitment (REG.10). A capital call is issued at 9c (applied at
  9e) when the fund's investment decision needs money (a founding, or a stake agreed by bilateral negotiation, MKT.7)
  and is due the next business day; an investor that cannot pay defaults on the call, with the terms' forfeiture:
  its interest passes to the fund, for the remaining investors.
- **Founding** (FRM.16): S1.03's founding decision now has firm and fund founders, the founder's kind being data;
  the value reads what the founder can know, the money comes from its named accounts, and the founding is executed at
  3c. Funds join S2.08's `found_bank` founders the same way (SUP.9).
- **Launch and closure** (FND.12), on the manager's quarterly review: it launches a fund of a kind when its value of
  the expected fees — its outlook of inflows from comparable funds' published flows (VAL.10), net of running costs —
  exceeds the launch cost, seeding it from named accounts; it closes one whose expected fees no longer cover its
  costs. The form — launches where expected fees pay and closures where they no longer do (Khorana and Servaes,
  1999; Zhao, 2005) — is listed in `SHAPES.toml`. A closing or failed fund ends into one estate (`sys-est`, S2.04's
  administration), which sells into markets, pays its lenders and then its unit holders (L3's table); a household
  estate holding fund units redeems them at the fund's value like any holder.
- **Fees**: the fee is a term chosen at launch; later changes are the manager's posted point, set by `sys-frm`'s
  price review (S1.03) with the fund's flows against expectation as the pressure `sys-fnd` supplies.
- **Streams**: `FND.gate_lot`, `FND.creation_lot`.
- **Review costs** (TECHNOLOGY, hours): `FND.invest_review_hours`, `FND.liquidity_hours`, `FND.ap_hours`,
  `FND.launch_review_hours`, `FND.choose_hours` for institutional investors.
- **Opening** (GEN): funds by kind with their managers, mandates, holdings, investors' units (households' from wealth
  surveys) and value histories; authorised dealers' agreements; hedge funds' prime-brokerage accounts (S3.06).

**Unit tests**
- `nav_identity_with_rounding_on_fund`.
- `stale_mark_gives_labelled_stale_value`.
- `forward_pricing_deals_at_next_value`.
- `stable_price_breaks_outside_band`.
- `redemption_beyond_buffer_sells_by_order`.
- `gate_defers_never_drops`.
- `late_sale_cost_on_stayers`.
- `failed_redemption_opens_procedure`.
- `ap_creates_only_beyond_cost_within_limits`.
- `capital_call_default_forfeits_to_fund`.
- `launch_value_against_cost`.

**Live checks**
- `LC-3-09`: FND.9 — every fund's units times its value equals assets minus liabilities to its price's rounding,
  with the residue on the fund.
- `LC-3-10`: FND.10 — flows against past performance, money-fund flows against deposit rates, exchange-traded funds'
  discounts in stress and hedge funds' deleveraging are reported.
- `LC-3-36`: FND.11 — every redemption was paid in full on its date, deferred by a gate its terms allow, or failed
  with its fund's procedure opened; every fund that ended met L3's trigger; no fund held leverage from no lender.
- `LC-3-37`: FND.8 — every capital call was paid on its date or its investor's default and forfeiture to the fund are
  recorded.
- `LC-3-38`: FND.6 — each fund's forced sales name the redemptions they met, and the cost borne by staying investors
  is published per episode.

**Budget**
- About 3 000 funds; values at 9a over about 0.9 M positions at 10 ns (9 ms of the valuation line's 16), each fund's
  value read from them at 9b; managers' reviews over their candidate lists (about 600 a day, 50 candidates each at
  300 ns); dealing orders about 30 000 a day at 100 ns; about 3 000 administered meetings at 1 µs.
- The ledger's line: 5 ms on a business day ((9 + 3 + 3) core-ms ÷ 3), 8 ms on a heavy day; values at 9a, about
  3 ms, in the valuation line; money funds' daily accruals about 1 ms on a non-business day.
- Memory: funds' positions about 0.9 M at 32 B with 6 B holder-list entries (34 MB) and their lots, about 1.5 per
  position at 32 B (43 MB): about 77 MB of the stage's institutions' line; their kind-table rows within the
  individuals line.
- Counters, ratcheted: `phx_fnd.funds`, `phx_fnd.nav_positions`, `phx_fnd.dealing_orders`,
  `phx_fnd.forced_sales`, `phx_fnd.gates`, `phx_fnd.creations`, `phx_fnd.capital_calls`, `phx_fnd.launches`,
  `phx_fnd.closures`, `phx_fnd.procedures`, `phx_ledger.lots`.

**Guards**: PC-55: a fund unit's dealing price is written only by `sys-fnd`'s 9b handler from 9a's marks and
labelled valuations; no declaration holds a constant unit price, and a stable price exists only as a term compared
with the value.

**Not allowed**:
- a constant value by construction;
- leverage without a lender;
- a redemption rationed by cash with the rest dropped;
- a fund that cannot fail, or one that ends other than by L3's trigger;
- a tracker ordering "at any price";
- dealing at a value computed before the order was taken.

**Done when**
- [ ] Fund flows become purchases and sales in named markets; redemptions force sales; a leveraged fund can be closed
  out by its broker (LC-3-08 and LC-3-45); funds are launched and closed.
- [ ] LC-3-09, LC-3-10 and LC-3-36 to LC-3-38 pass, and LC-3-08 now applies.
- [ ] PC-55 is registered.
- [ ] Two reviews are done.

---

### S3.08 — Non-bank lenders

**Status**: planned

**Clauses**:
- STATE: BNK.22.
- PROCESS: BNK.10 *(part: non-bank lenders, private-credit funds among them, buy loans through `quote_loans`;
  securitisation vehicles complete it at S4.05)*.

**Architecture**: §3.4 (writer tokens), §4.1 (kinds and facets), §4.7 (rule handles), §9.1 (endings).

**Depends on**: S3.07.

**Goal**: finance companies funded by bonds, paper and bank lines, and private-credit funds, write loans under the
same contracts and decisions as banks, without deposits, and fail when their funding does.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-credit/src/lenders.rs` | lender kinds as data: who lends, from what funding |
| `crates/systems/sys-bnk/src/decl.rs` | BNK.4–BNK.7's decision points declared for every lender kind by data |
| `data/<country>/BNK.toml` | the finance-company and private-credit-fund lender kinds |
| `data/<country>/BFL.toml` | their forms' funding sources, which S2.06's `marginal_cost_of_funds` reads |
| `data/<country>/BCP.toml` | their forms' capital regimes, which S2.07's `capital_charge` reads |
| `data/<country>/gen/BNK.toml` | opening finance companies and their books, with sources |

**Design**

- **Same decisions** (BNK.22, Law 10): quote, decline, assessment, workout and provision (BNK.4–BNK.7, S1.09 and
  S2.01) run for every party whose kind the data lists as a lender; the handlers traverse each lender kind's table.
  `sys-bnk` builds each non-bank lender's `LoanAssessment` by the same rule (the writer token, PC-40), which reads
  what it can observe: its own lines, records it bought from the bureau (S2.10), public filings. Non-bank lenders
  report to the bureau as its law requires. Borrowers shop among every lender they reach (BNK.6), banks and
  non-banks alike; a loan sale's asks (S2.01) reach them, answered by `quote_loans`.
- **Legal form** (PTY.4): a finance company is a firm whose form may not take deposits; a private-credit fund is a
  fund (S3.07). A deposit line whose issuer's form forbids deposits is refused at assembly and by the ledger (MON.1).
- **Cost of funds**: S2.06's rule handle `marginal_cost_of_funds` reads the sources each lender's form declares —
  deposits where permitted; paper and bonds at its own outlook of its paper's yield (S3.04); drawing its bank lines
  at their terms; for a private-credit fund, uncalled commitments at its investors' preferred return (its terms).
  The next unit of funding costs what the cheapest reachable source costs at the margin.
- **Capital consumed**: S2.07's rule handle `capital_charge` reads the lender's declared regime — for these forms,
  the leverage limit in its funding covenants or its mandate (declared data).
- **Funding and failure**: a finance company rolls its paper (S3.04) and draws lines its banks may cut (BNK.18); a
  private-credit fund calls capital (S3.07). A roll that fails and a line that is cut leave it short: it stops
  lending (its quotes decline on liquidity, BNK.5), and a payment it cannot make is its default of payment (FRM.15) or
  its fund's ending (L3); its one estate (S2.04) sells its loans (BNK.10).
- **Streams**: none new; S1.09's `BNK.lender_taste` covers borrowers' tastes over all lenders.
- **Opening** (GEN): finance companies with their loan books, paper and lines, from supervisory and industry data.

**Unit tests**
- `nonbank_quote_uses_its_own_marginal_cost`.
- `deposit_line_refused_for_nondeposit_form`.
- `capital_consumed_by_declared_regime`.

**Live checks**
- `LC-3-11`: a non-bank lender whose funding fails stops lending and, if it cannot pay, fails, with its estate selling
  its loans.
- `LC-3-39`: BNK.22 — no non-bank lender holds a deposit liability, and every non-bank loan came from the same
  decision points as banks'.

**Budget**: a few hundred finance companies and private-credit funds, inside BNK's lines; with S3.09 and S3.10, 2 ms
of the ledger on a business day and 7 ms on a heavy day, this step's share under 0.5 ms. Counters, ratcheted:
`phx_bnk.nonbank_loans_written`, `phx_bnk.nonbank_failures`, `phx_bnk.nonbank_loan_purchases`.

**Guards**: none new.

**Not allowed**:
- lending code specific to a lender's kind;
- a non-bank taking deposits;
- a non-bank funded from nowhere, or one that cannot fail.

**Done when**
- [ ] Non-bank lenders lend through banks' decisions from their own funding, and can fail.
- [ ] LC-3-11 and LC-3-39 pass.
- [ ] Two reviews are done.

---

### S3.09 — `sys-idx` in full: reference rates and market indices

**Status**: planned

**Clauses**:
- STATE: IDX.2; IDX.1 *(completes it: market indices, with S1.14's price indices)*.
- DECISION: EQY.3 *(completes it: index trackers track their index within their own tolerance, by limit orders)*.
- PROCESS: IDX.4 *(completes it)*.
- INVARIANT: IDX.5 *(completes it)*.
- FORBID: IDX.6 *(completes it)*.
- PRIMITIVE: IDX.7 *(completes it)*.
- This step retires S0.14's placeholder naming IDX: a floating reference's reset reads its series' last published
  value until the benchmark's fixings begin, here, on the same series identity.

**Architecture**: §8, §4.9 (records), §6.1 (8b and 9d).

**Depends on**: S3.08.

**Goal**: floating-rate benchmarks are reads of the money market's transactions, absent on a day without them;
market indices are rules over their constituents' prints, never stored as numbers of their own, chained across
changes; a tracked index change is a real, simultaneous trade by every fund that tracks it.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/indices.rs` | index rules and chain links; benchmarks and fallbacks |
| `crates/systems/sys-idx/src/benchmark.rs` | IDX.2: fixings from match sets |
| `src/market_index.rs` | IDX.1: the level as a read |
| `src/rebalance.rs` | IDX.4: constituent changes, links, announcements |
| `src/handlers/9d_benchmarks.rs`, `9d_rebalance.rs` | fixings and announcements, public the next day |
| `src/audit.rs` | the IDX.5 family |
| `data/<country>/IDX.toml` | each publisher's methods, rules, weights, bases, rebalance dates (POLICY, IDX.7) |

**Design**

- **Reference rates** (IDX.2): each country's administrator (a large firm with the publisher facet, opened by GEN)
  fixes the overnight benchmark at 9d of each business day as the volume-weighted trimmed mean of that day's eligible
  unsecured overnight matches at 8b, eligibility and trimming being its declared method (POLICY; ECB, 2019; IOSCO,
  2013). Term benchmarks fix on term matches, and compounded averages are reads over past fixings. A day without
  eligible matches has no fixing; a contract fixing on that day uses its own declared fallback (its terms; ISDA,
  2020), never a rate the administrator posts. The fixing is a record labelled as a fixing (MKT.12), public from the
  next day, kept because contracts pay on it. Floating loans (BNK.1), interbank term loans (S2.06), mortgages (S2.05)
  and notes (S3.04) fix on it from here, each on its terms' fixing day: their reference was the series' one identity
  from origination (S0.14), so the placeholder ends with no contract's terms changed.
- **Market indices** (IDX.1): a publisher's rule over constituents and weights (capitalisation from shares
  outstanding and closes, amount outstanding for bonds, or equal) with a base. The level is a read —
  `base × (product of stored links) × Σ wᵢ·pᵢ(d) ÷ divisor` over the constituents' last prints (closes, fixings),
  each carrying its age — and is never stored. Outlooks read it as a public series through that read.
- **Changes** (IDX.4): on the publisher's rebalance dates its rule selects constituents from published data; the
  change is announced at 9d with an effective day at least the declared notice later; the link at the effective day
  is stored as the publisher's record, so the level does not jump. Trackers (S3.07) see the announcement and trade by
  their own decisions, at the close of the effective day, by `AtTheClose` limit orders within their tolerance
  (EQY.3, completed here).
- **The IDX.5 family**: each day, an index's return equals the weighted return of its constituents, to the arithmetic
  allowance of its price convention (Law 7), incrementally.
- **Forbids** (IDX.6): the constituent rule refuses an instrument whose issuer tracks the index or whose price is set
  from it; an index without constituents is refused at assembly and at rebalance; a benchmark fixing is built only
  from match sets (PC-56); one publisher's rule has one identity.
- **Streams**: none.
- **Opening** (GEN): administrators and index publishers, large firms with the publisher facet; index definitions
  and histories (GEN.5); the stored links.

**Unit tests**
- `benchmark_trimmed_volume_weighted_mean`.
- `benchmark_absent_without_transactions`.
- `fallback_by_contract_terms`.
- `index_level_is_a_read_of_prints_and_links`.
- `chain_link_no_jump`.
- `index_return_identity`.
- `constituent_rule_refuses_own_tracker`.

**Live checks**
- `LC-3-12`: IDX.5 and IDX.6 — the family is clean for every market index; no index is an input to its own
  constituents.
- `LC-3-40`: IDX.2 — every benchmark fixing traces to the day's eligible matches; no fixing exists for a day without
  them; every fallback used names its contract's terms.
- `LC-3-41`: IDX.4 — every constituent change stored its link; every tracker's trades on the effective day are
  recorded.

**Budget**: three overnight fixings a business day and a few term fixings; index levels read on demand (about 50
indices of up to 500 constituents); rebalances on heavy days; under 0.5 ms of the ledger's 2 ms line on a business
day. Counters, ratcheted: `phx_idx.fixings`, `phx_idx.absent_fixings`, `phx_idx.fallbacks_used`,
`phx_idx.constituent_changes`.

**Guards**: PC-56: `BenchmarkFixing`'s only constructor takes the match-set identities of the day's 8b meetings (a
compile-level refusal of a posted benchmark).

**Not allowed**:
- a posted benchmark, or a policy rate as a benchmark;
- a stored market-index level;
- an index jump at a constituent change;
- one index under two names.

**Done when**
- [ ] Floating rates fix on transacted rates; market indices are reads of prints; tracked changes are real trades.
- [ ] LC-3-12, LC-3-40 and LC-3-41 pass.
- [ ] PC-56 is registered.
- [ ] Two reviews are done.

---

### S3.10 — `sys-rat`: ratings, reports, guidance and estimates

**Status**: planned

**Clauses**:
- STATE: RAT.1, RAT.3; RAT.2 *(completes it: listed companies' reports, with S2.10's filing calendar)*; RAT.8
  *(completes it: the rules that refer to S2.10's filed accounts)*.
- PROCESS: RAT.5; RAT.4 *(part: the rules of Stages 1 to 3 that refer to ratings; insurers' mandates complete it at
  S4.03)*.
- MEASURE: RAT.6.
- FORBID: RAT.7.
- PRIMITIVE: RAT.9.
- This step retires the placeholders naming RAT of S3.02 (collateral eligibility without ratings) and S3.05 (filed
  accounts standing in for reports).

**Architecture**: §3.1 (the rating scale in `if-base`), §4.1 (publishers), §4.9 (records and audiences), §7.3
(wakes), §6.1 (stages 5, 9).

**Depends on**: S3.09.

**Goal**: agencies publish coarse, sticky ordinal ratings from issuers' published state, never from prices; listed
companies publish reports on dates they choose within the legal window, with guidance; banks' analyst units publish
estimates from what they observed; the rules that refer to ratings bind their holders at once when a rating crosses
a boundary; reports settle expectations, and prices move because schedules moved.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-base/src/rating_scale.rs` | rating scales and notches, read by `if-credit`'s `LoanAssessment` and by S2.07's risk weights |
| `crates/interfaces/if-securities/src/ratings.rs` | ratings, `RatingView`, rule references |
| `crates/interfaces/if-securities/src/reports.rs` | listed companies' reports, restatements, guidance, estimates |
| `crates/systems/sys-rat/src/rules/rate.rs` | RAT.1: the agency's method |
| `src/rules/report_date.rs` | RAT.2: the publication date, S2.10's filing-day form (`sys-rat/src/filing.rs`) |
| `src/rules/estimate.rs` | RAT.3: analysts' estimates |
| `src/handlers/5c_*.rs` | rating reviews, report dates, estimates |
| `src/handlers/9c_report.rs` | a report read from the books (ACC.9) and, for a group, its consolidated statement, on its date |
| `src/handlers/9d_publish.rs` | ratings, reports, guidance and estimates published; analysts' estimates revised on the day's reports |
| `data/<country>/RAT.toml` | scales and methods (POLICY of agencies); windows and deadlines (POLICY) |

`RAT.toml` also holds the analysts' memory types (PREFERENCE), the preparation and review hours (TECHNOLOGY) and the
opening fiscal year-ends (ENDOWMENT).

**Design**

- **Ratings** (RAT.1), by each agency (a large firm with the publisher facet; several per country), on its annual
  review of each rated issuer and on wakes (a report or filing, a missed payment, a default):
  - inputs are a `RatingView` of the issuer's published state — reported ratios over the agency's window (leverage,
    coverage, size, the volatility of earnings), for a sovereign its published debt, deficit and output (STA) — and
    no price (PC-57);
  - the method is a scorecard: declared weights and thresholds (POLICY of the agency) map the score to a notch on its
    scale (declared in `if-base`, so `LoanAssessment` and risk weights read it below `if-securities`), which moves
    only when the score crosses a threshold by more than the declared band, so ratings are coarse and sticky and lag
    what the data lag; agencies with different weights disagree;
  - the form (Altman, 1968; Cantor and Mann, 2003) is listed in `SHAPES.toml`; the issuer pays the agency's fee, a
    point set by `sys-frm`'s review (S1.03).
- **Reports** (RAT.2): a listed company's fiscal year-end is its accounting reference date under company law (S2.10);
  its publication date after each period follows S2.10's filing-day form — once its preparation hours (TECHNOLOGY,
  from its staff's capacity) are spent, at its first review when its result beats the last period's and at its last
  review before the deadline otherwise (Givoly and Palmon, 1982) — within the exchange's window, so dates spread
  across the calendar (N8.9). Reports live in `if-securities`; filed accounts stay company law's, in `if-firm`. The
  report is a read of its books (ACC.9) at the period's end — for a group, its consolidated statement (S0.19) — taken
  at 9c on its date and published at 9d; an event recognised later but dated inside a published period restates it,
  and both versions are kept with their dates. Listing now requires reports (S3.05's placeholder retired).
- **Guidance** (RAT.3) is management's own outlook of its next period's earnings (VAL), published with the report.
- **Estimates** (RAT.3): analyst units — research subsidiaries of banks, each a large firm with the publisher facet —
  publish outlooks of the earnings of the firms they cover, each by its own method (the heuristic menu over published
  reports, guidance and statistics), never from the share price (PC-57). They read only public records, never their
  parent's `LoanAssessment` or any other private state of it (Law 12). A report published at 9d is read by the
  analysts covering its firm at 9d, and their revised estimates are published with it, public from the next day. A
  consensus is a read.
- **Rules refer to ratings** (RAT.4), as declared references, each naming the scale it reads: the central bank's
  eligibility (S3.02's placeholder retired), the supervisor's risk weights by rating (S2.07's table gains the
  rating), lenders' classes — `sys-bnk`'s `LoanAssessment` of borrowers and counterparties reads ratings as a class
  input from here (S1.09, S2.06, S3.01) — funds' mandates (S3.07: a downgrade below a mandate's minimum is a
  breach, cured by the fund's own sales within the mandate's cure period), insurers' from S4.03, and contract
  triggers (step-up coupons, collateral to post). An agency's scale that no rule refers to is refused at assembly
  (RAT.7). A rating published at 9d reaches every bound holder at its next business day's decisions, at once.
- **Reports settle expectations** (RAT.5): a report is an observation of every party's outlook of that firm's
  earnings; its surprise (VAL.4) wakes the holders and candidates it bears on (REP.35) and revises their values, so
  their orders change.
- **Filings** (RAT.8): S2.10's filed accounts — the record kind in `if-firm`, written by `sys-rat`'s filing rule —
  complete here as the non-listed firm's report, now read by the rules above (agencies rate unlisted issuers from
  them).
- **Streams**: none.
- **Review costs** (TECHNOLOGY, hours): `RAT.rating_review_hours`, `RAT.report_hours`, `RAT.estimate_hours`.
- **Opening** (GEN): agencies, analyst units, fiscal year-ends; opening ratings are not drawn but computed on day one
  by each method from the opening published history (GEN.5).

**Unit tests**
- `rating_view_has_no_price` (compile-fail).
- `scorecard_notch_with_hysteresis`.
- `agencies_disagree_by_weights`.
- `report_date_good_early_bad_late_within_window`.
- `restatement_keeps_both_versions`.
- `unreferenced_scale_refused`.

**Live checks**
- `LC-3-13`: RAT.6 — the downgrade loop (selling, capital pressure, dearer funding, weaker state, further downgrade)
  is traceable step by step in events wherever it occurs; price moves against the size of report surprises and the
  dispersion of estimates after volatile results are reported.
- `LC-3-42`: RAT.7 — no rating or estimate read a print or a price series (the read-trace); every scale is referred
  to by a rule; every reported number is a read of its company's books.
- `LC-3-43`: RAT.2 and N8.9 — report dates are spread across each window, and no report was read before its date.
- `LC-3-50`: RAT.5 — after each report whose surprise woke holders and candidates, the orders those parties posted
  changed with their revised values, and the price move is traceable to those orders in events; no analyst read
  its parent's private state (the read-trace).

**Budget**: about 100 rating reviews a business day at 5 µs; up to 400 reports a day in the season (their statement
reads in the valuation line, 2 ms of its 16); about 2 000 estimates a day; report surprises' wakes within §13.2's
publication-day line; with S3.08 and S3.09, the ledger's 2 ms on a business day and 7 ms on a heavy day, this
step's share about 1 ms and 5 ms. Counters, ratcheted:
`phx_rat.ratings_changed`, `phx_rat.reports`, `phx_rat.restatements`, `phx_rat.estimates`,
`phx_rat.boundary_crossings`.

**Guards**: PC-57: `RatingView` and the estimate view have no field of type `Print` or `Mark` and no market-index
identifier (IDX.1) (a signature check), so no rating or estimate can read a price; benchmark fixings and published
statistics, the consumer index among them, may be read.

**Not allowed**:
- a rating derived from a price, or one nothing refers to;
- an estimate from the share price or the model's forecast, or from a parent's private state;
- a price-reaction rule;
- a reported number the books do not produce.

**Done when**
- [ ] Ratings, reports, guidance and estimates are published from published state; rules bind holders when ratings
  cross their boundaries; reports move prices through revised views.
- [ ] LC-3-13, LC-3-42, LC-3-43 and LC-3-50 pass.
- [ ] PC-57 is registered.
- [ ] Two reviews are done.

---

### S3.11 — The Stage 3 gate: the forced seller, the cost of capital and the budget

**Status**: planned

**Clauses**: L2, L4 *(their liveness; their tests are N4's, S7.02)*; PTY.12 *(part: the ladder and the reference
comparison at Stage 3)*; REP.18 *(part: every RESOLUTION setting, taste distribution and review cost of Stage 3's
decisions declared and measured)*; N8 *(the budget at Stage 3)*; the Stage 3 exit.

**Architecture**: §7.11, §13, §14.4, §14.5, §14.7.

**Depends on**: S3.10.

**Goal**: judge the exit, the budget and the comparison with the reference on measured numbers, against criteria and
experiments fixed before the run.

**Files**

| File | Purpose |
| --- | --- |
| `data/shared/READS.toml` | Stage 3's reads, declared at the start of S3.01 and frozen before the first comparison |
| `data/shared/EXPERIMENTS.toml` | the declared interventions (N6, §0.3), frozen before the run |
| `perf/device/S3.11-*.json`, `perf/measure/S3.11-*.json` | the device report and the measurements |
| `perf/compare/S3.11-*.json`, `perf/ladder/S3.11-*.json` | the comparison and the ladder |

**Design**

- **The reads** (N8.5) added: overnight and term rates and spreads by name class; repo haircuts; bill and bond yields
  and the curve's slope; corporate spreads by class and seniority; the distribution of share returns (tails,
  volatility clustering); fund flows against performance; exchange-traded funds' discounts; dealers' widths; and per
  person, from tracers in play and from every person in the reference, portfolio participation and composition by
  wealth and age and the returns households earned.
- **The reference** is re-sized with Stage 3's state (households' holding rows and participation; architecture §7.11
  re-estimated) and run for five seeds; the play resolution for twenty; the ladder over at least three rungs.
- **The device run**: the settled Stage 3 world, a 30-minute soak, then a simulated year. **The decades run**: thirty
  simulated years on the weekly job, with the liveness reads.
- **Experiments** (N6, §0.3), declared before the run on copies of the settled world at the same seed; each changes
  only a primitive or an endowment, never places a decision for a party, falsifies a record or changes a rule:
  1. the inflation target (the parliament's POLICY, CB.16) lowered by two points from a stated date, so the
     committee's own rule raises its rate;
  2. dealers' position limits (their banks' POLICY, DLR.12) halved from a stated date;
  3. a catastrophe of declared severity striking a region where listed firms hold their plant and banks' collateral
     lies (an endowment changed, as S2.12's LC-2-49 allows).

  Whatever rates, calls, redemptions and defaults follow are the parties' own. Knock-outs — a link held fixed — are
  N4's chain tests, on copies at S7.02, not this gate's.
- **L4**: on experiment 1, against the untouched run, the chain from the rate to overnight prints, banks' marginal
  cost of funds, loan quotes, bond and share prices, firms' marginal cost of money, investment, output and employment
  is traced with its lags.
- **L2**: the run's and the experiments' events are searched for forced sales by door — a margin call unmet, a
  redemption beyond a buffer, a funding line withdrawn or a repo defaulted, a mandate boundary crossed, a capital
  requirement met by shrinking, an estate's liquidation — each with its sale's prints and the holders it reached.
- **The exit**: a margin spiral (calls, close-out sales, a price fall, wider requirements, further calls at other
  accounts) and a fund run (redemptions, forced sales, a fall in value, further redemptions) each appear as a
  traceable chain in the settled run or an experiment, and the report says which. If either appears nowhere, that is a
  finding against the mechanisms suspected (brokers' margin, funds' dealing terms, investors' flow responses); the
  stage does not end, the owner is told, and nothing is tuned (N7).
- **Pass criteria**, fixed here before the run (architecture §14.5), as S2.12's: the median turn ≤ 1 000 ms and the
  worst ≤ 2 000 ms over the settled year; peak `VmHWM` and PSS ≤ 4.5 GB; a full save ≤ 5 s and an increment ≤ 1 s;
  the latest complete save and the one being written together ≤ 4 GB on the device's storage (N8.4); every
  read within Appendix E 30's band beyond the reference's seed spread, at the play resolution and each rung above it;
  the exit's reads; each step's line of the stage's ledger reported with its counters; §13's 10% headroom reported,
  and a pass
  without it recorded as a finding.
- **Placeholders**: every placeholder Stage 3 retires is gone, and every one remaining names a later step (S5.02's
  discretionary purchases among them).
- **A miss** is a finding; N8.7's remedies apply in order; the stage ends only when the budget in force is met (N8.8).
- Architecture §13 is rewritten with the measured numbers.

**Unit tests**: none.

**Live checks**
- `LC-3-14`: on experiment 1, the chain from the rate decision to investment is traceable, with lags.
- `LC-3-44`: L2 — every forced sale names its door, its prints and the holders its price reached.
- `LC-3-47`: N2 over the decades run — auctions, money-market matches, issues, trades on every book, fund dealings,
  rating changes and reports count at least one in every one of the thirty years.
- `LC-3-45`: the exit — a margin spiral and a fund run each exist as traceable chains in the settled run or on an
  experiment copy whose declared intervention changes only a primitive or an endowment; for each, the report says
  whether it appears in the untouched settled run as well as on a copy.
- `LC-3-46`: L4's links are live — on experiment 1, against the untouched run at the same seed, overnight prints,
  banks' marginal cost of funds, loan quotes, bond yields, share prices and firms' investment each differ beyond seed
  spread, each first responding no earlier than the link before it; on experiment 2, dealers' widths and the bond
  yields they quote differ beyond seed spread.

**Budget**: this is the budget's gate.

**Guards**: none.

**Not allowed**:
- a gate judged off the device;
- reads, experiments or criteria chosen after seeing the run;
- an experiment that places an order, a call or a redemption;
- a tuned primitive.

**Done when**
- [ ] The device report, the comparison over the declared seeds and the ladder are committed.
- [ ] Every pass criterion holds, or the owner's decision under N8.7 is recorded in §12 and the budget then in force
  is met.
- [ ] LC-3-14 and LC-3-44 to LC-3-47 pass.
- [ ] Architecture §13 is updated with measured numbers.
- [ ] Two reviews are done.

---

## 7. Stage 4 — Risk transfer

**Exit** (spec Part O):
- DRV with client clearing, DRX (swaps, credit, currencies, futures, options), INS, PEN with its trustees', sponsors'
  and members' decisions, SEC and MNA run in all three countries, each still closed to the others.
- Every derivative class forms its price with views on both sides.
- Hazard events become insurance claims.
- Pension liabilities move with rates.
- A settled simulated year meets the budget (N8) on the device, with the ladder within the declared accuracy.

**Decision rules** follow §2.21.

**Where Stage 4's state lives**:
- **Derivatives** are lines between individuals (banks, dealers, funds, insurers, schemes, large firms, clearing
  houses), the owner's decision (spec Appendix E 33): no household or small-firm cell holds one, and their risk runs
  through their contracts' terms. A derivative line kind declares individuals as its only holder kinds, and assembly
  refuses a cell kind on either side.
- **Policies, annuities and pension rights** are relationship rows on lines in the holders' arenas (architecture
  §4.5), attached to the adult and dwelling roles' profiles (REP.32). DB rights are the rows' balances; DC pots are
  holding rows of fund units with a member count (S3.05), flagged `pension`.
- **Policy and annuity lines are many-party lines**, as employment lines are (REP.3): one line per identical terms,
  with the insurers writing those terms on one side and the holders on the other. Which insurer covers which member
  is drawn when it matters — a claim, a lapse, an insurer's failure (REP.23) — so a taste-driven choice between
  insurers posting the same terms moves side counts, not rows or cells.
- **Due-day runs**: policy, annuity and scheme rows are kept, like S2.02's invoices, as runs ordered by due day
  behind one head per holder (architecture §6.5), so settlement's stream reads a holder's head and enters a run only
  on a day something in it is due.
- **The household record** grows from 584 bytes (568 through Stage 2, S3.05's `choose_holdings` +16) to 624: two
  lumpy kinds, `insure` (S4.03) and `pension` (S4.04), 16 bytes each, and the due-day runs' head, 8 bytes. The firm
  record grows from 464 bytes (through Stage 3) to 480, for the firm's `insure`, inside §13.1's 500-byte line.
- **Decision points** live in the latest interface crate their types need (architecture §3.1) — `if-risk` for
  hedging, cover, collateral, pensions and resolution bids; `if-securities` for securitisation and control — with one
  rule per decider kind, each registered by the system that owns that kind's decision (architecture §3.4): the
  system whose clause the decision is, except that a bank valuing a claim on its book does so through `sys-bnk`.
- **Experiments** (spec N6, §0.3): S4.07's declared interventions on copies of the settled world show what a single
  run may not — a catastrophe breaking an under-reinsured insurer, a rate move calling hedged schemes' margin.
- **Opening**: Stage 4's systems are opened by their steps' contributions. The opening world holds no derivative
  position and no derivative print; both are formed from day zero of settling (GEN.6), and views start from the
  closest observed series (VAL.10).
- **FX forwards and swaps** (DRX.3) are declared here and meet from S5.04, when currencies meet.
- **The stage's budget ledger** is S4.07's: about +465 MB, and +59 ms on a business day (+3 non-business, +89 heavy),
  so that from Stage 3's baseline (1 135 ms, 4 536 MB) the design point projects a median of about 1 194 ms and a peak
  of about 5 000 MB, both over budget (F-003, F-004). Each step's **Budget** names its §13 lines and counters.

---

### S4.01 — `sys-drv`: derivative contracts, margin and clearing houses

**Status**: planned

**Clauses**:
- STATE: DRV.1, DRV.2, DRV.3, DRV.9.
- PROCESS: DRV.4, DRV.5, DRV.6; MKT.20 *(completes it: the last methods it names — a clearing house's settlement
  price from trades and submissions, a calculation agent's valuation, and the swap curve beside S3.03's, with
  valuers' extrapolation labelled — on which the actuaries of S4.03 and S4.04 register)*.
- INVARIANT: DRV.7.
- FORBID: DRV.8.
- PRIMITIVE: DRV.10.
- The owner's decision that only individuals hold derivatives (spec Appendix E 33) is carried by the line kinds'
  declared holder kinds.
- This step introduces one placeholder naming SUP (S4.07): a clearing house that runs past the end of its waterfall
  ends into an estate.

**Architecture**: §3.4 (`if-risk`), §4.4 (contract algebra, line transfers), §4.7 (admission hooks), §6.1 (2c, 2d,
2e, 5c, 6a, 6b, 6c, 6d, 7, 9a, 9c, 9d, 9e), §8, §9.1, §9.3.

**Depends on**: S3.11.

**Goal**: every derivative is a two-sided contract between named parties, marked every business day by a named valuer
from prints:
- variation margin moves each day's change in mark in cash, called at 9c and paid at the next business day's 2c;
- initial margin follows the underlying's measured volatility and the position's remaining life;
- clearing houses stand between members, with margin, a default fund and a declared waterfall;
- clients clear through members, and a member's default ports its clients or closes them out;
- a default runs down the waterfall and can reach survivors' contributions, and past its end the house fails.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-risk/src/derivatives.rs` | `DerivativeTerms`; line kinds; `Clearing` |
| `if-risk/src/margin.rs` | `MarginCall` message; margin and default-fund line kinds; `LiquidityReport` |
| `if-risk/src/clearing.rs` | the clearing-house kind and facets; `Rulebook`; rule handles `admit`, `house_im` |
| `if-risk/src/closeout.rs` | `DefaultLots`, `CloseOut` records; decision points `default_bid`, `accept_port` |
| `if-risk/src/collateral.rs` | decision points `post_collateral`, `choose_clearer` |
| `crates/systems/sys-drv/src/valuers/settlement.rs` | settlement prices from trades and submissions |
| `crates/systems/sys-sov/src/curve.rs` | S3.03's publisher fits the swap curve beside the sovereign one |
| `src/valuers/premium.rs` | untraded options from traded premiums |
| `src/valuers/agent.rs` | a bilateral agreement's calculation agent |
| `src/margin/{variation,initial,account}.rs` | VM per row; IM per account by filtered historical simulation |
| `src/default_fund.rs` | sizing, calls, pro-rata write-downs |
| `src/waterfall.rs` | the house's waterfall as a pure function over declared layers |
| `src/rules/{default_bid,reserve,accept_port,post_collateral,choose_clearer}.rs` | each decider kind's rule |
| `src/handlers/*.rs` | 9a marks, close-out values; 9c calls, waterfall, sizing (applied at 9e); 9d; 2d; 5c; 2e |
| `src/audit.rs` | `DRV.two_sided`, `DRV.vm_balance`, `DRV.margin_cover` |
| `src/gen.rs` | the opening clearing houses, their members, capital and default funds |
| `data/<country>/DRV.toml` | rulebooks; bilateral margin rules; clearing obligations; valuers' methods; hours |
| `data/<country>/gen/DRV.toml` | houses per exchange, membership criteria, opening capital and fund, with sources |
| `data/shared/SHAPES.toml` | the forms of `default_bid`, the house's reserve, `accept_port`, `post_collateral` |

**Design**

- **Contracts** (DRV.1, DRV.2, DRV.8). `DerivativeTerms` is a composition of S0.14's legs (fixed and floating rates
  on a notional, deliveries, amounts contingent on a named event, amounts indexed to a print) plus:
  - `underlying: Series(SeriesId) | Event(EventKindId, PartyId)`: a series some market of this world prints, or a
    published event kind on a named entity. Assembly refuses any other (DRV.8: no underlying only inside the
    contract);
  - `exercise: Schedule` (empty, one date, or every business day to expiry), so style is data (Law 10);
  - `clearing: House(PartyId) | Agreement(LineId)`.
  Terms are interned (S0.14). The price at inception is the trade's print, never solved for (DRV.1).
- **Lines**:
  - **bilateral**: one party per side, count = contracts;
  - **cleared** (DRV.2): the trade's instruction at 7 novates it onto two lines per (house, terms) — members long
    against the house, and members short against it. A member's long and short in one series net at novation,
    since they face one counterparty; nothing nets across counterparties (DRV.8);
  - **client** (DRV.9): lines per (member, terms, direction) with clients on the other side, mirrored by the
    member's rows on the house's lines in a client account, segregated per client as the rulebook declares.
  - Each row is `RelRow` (16 bytes) plus `amount i64`, the mark per contract that its last VM settled against: 24
    bytes. Rows sit in their holder's due-day run (coupons, premiums, expiries), so settlement's stream reads them
    only on their due days; VM and IM read them at 9a and 9c.
  - **Members** are banks and their dealer desks (parties of the dealer kind, S3.06) meeting the house's criteria.
- **Margin lines**: cash IM is a row with `balance` on a margin line (member's asset, house's liability, 24 bytes);
  securities IM is a lien to the house (REG.2), still the poster's asset but not free (DRV.4). Default-fund
  contributions are rows with `balance` on the fund line.
- **Marks** (DRV.4, MKT.20), at 9a by named valuers (S0.19's `ValuerDecl`), one `Valuation` per (valuer, terms) with
  open rows, showing its prints, their ages and what is interpolated or extrapolated:
  - **settlement price**: the day's mark of the series' market (MKT.12); with no print, the median of members'
    submissions, each member's own valuer's figure from prints (audience: the house);
  - **curve**: S3.03's publisher fits, at 6c, a swap curve beside the sovereign one, through the day's swap fixings
    (S4.02's market, fixed at 6b) and reference-rate fixings (S3.09), by S3.03's method — linear in zero rates between
    traded tenors, each point labelled traded or interpolated, nothing beyond the longest — with its discount-factor
    table. A valuer needing a point beyond it (an actuary, S4.03, S4.04) extrapolates in its own declared method,
    holding the last traded forward rate flat, and labels the point extrapolated (MKT.20);
  - **premium**: an untraded option is valued from the day's option prints on the same underlying, interpolated in
    each print's implied volatility (a statistic of its premium, Law 3) across strike and in total variance across
    expiry, as market practice does (Gatheral, 2006), and marked interpolated;
  - **calculation agent** (bilateral): the agreement's named agent applies the same methods;
  - a series with no print within the method's declared horizon has no mark that day: its VM is not called, its IM
    stands, and the absence is recorded (NUM.8).
- **Variation margin** (DRV.4, DRV.7), at 9c, where each call is issued by the handler that computes it and applied
  with stage 9's intents at 9e (architecture §6.1):
  - per row, `(mark − amount) × count`, signed by side; summed per account (a member's house or client account, or
    one bilateral agreement), which is the only netting (DRV.8);
  - one `MarginCall` per (account, direction), due at the next business day's 2c (TIME.7). Each member's payment and
    each of the house's payments is its own instruction, so one payer's failure fails only its own legs (SET.4);
  - on settlement at 2c, the rows' `amount` is set to the called mark; a failed call leaves it and is a default.
  - Bilateral agreements apply their terms' threshold and minimum transfer amount, declared limits of the contract.
- **Initial margin** (DRV.4), at 9c, per account:
  - **the house's method** (DRV.10, POLICY of the house): expected shortfall at the house's confidence of the
    account's value change over its margin period of risk, by filtered historical simulation over its lookback of the
    underlyings' own daily changes, each scenario scaled by the ratio of today's EWMA volatility to that day's
    (Barone-Adesi, Giannopoulos and Vosper, 1999). It rises with measured volatility and falls with remaining life,
    since sensitivities are read from the valuation;
  - the account's value change per scenario is `Σ_f s_f·Δ_f + ½·Σ_f g_f·Δ_f²` over its risk factors, with the
    sensitivities `s_f`, `g_f` summed from its rows at 9a (delta and gamma, the house's declared method);
  - the call is the requirement less posted collateral at the house's haircuts, due at 2c; an excess is returned at
    2c;
  - bilateral agreements use the bilateral margin rules (POLICY of the supervisor): the same method or the declared
    schedule by class and maturity, exchanged both ways and held segregated.
- **Collateral** (`post_collateral`, a lumpy decision of the poster at 5c on its review days and after a call):
  calls are paid in cash at 2c; the poster may then substitute eligible securities for cash margin, choosing the
  eligible holdings with the lowest cost per unit of collateral value after haircut, its own expected return on each
  against the rate the house pays on cash (collateral optimisation, Duffie, Scheicher and Vuillemey, 2015). The
  substitution settles at 7.
- **Admission** (DRV.6): the rule handle `admit(member, order) -> Admit | Cut(qty) | Refuse`, called by `phx-market`'s
  admission hook at 6a for every order of a member or its client:
  - the member's stressed margin after the order — the house's stress scenarios (POLICY) applied to its accounts plus
    the order's sensitivities — against the liquid resources in its latest `LiquidityReport`;
  - the report is written at 9d in the member's own context from its own rows (reserves, deposits, free eligible
    holdings at the house's haircuts), audience its houses (Law 12);
  - a cut or refusal is an event the member sees (Law 6).
- **Clients** (DRV.9): a party that is not a member chooses its clearer (`choose_clearer`, lumpy, on its review
  days): among members that accept it, the lowest posted fee point, ties by lot (`DRV.clearer_lot`). The clearing
  agreement's terms carry the member's add-on over the house's requirement (the member's POLICY) and the client's
  named backup member.
- **Default** (DRV.5, architecture §9.3), for a call issued at 9c of D:
  - **D+1, 2c**: a call unpaid is the payer's default under its rulebook or agreement; **2d**: it is recorded, the
    member is suspended (`admit` refuses it), and the house announces `DefaultLots` — the defaulter's house account
    in lots per class, and each client account as a port lot;
  - **D+1, 5c**: surviving members bid for lots (`default_bid`) and for port lots (`accept_port`); the house posts its
    reserve per lot; a client's backup member answers first;
  - **D+1, 6a**: each lot forms as a call auction (MKT.3) among the members' bids and the house's reserve, ties by lot
    (`DRV.default_auction_lot`); **7**: auction trades settle, ported clients' rows and margin move by line transfer
    (architecture §4.4), and the defaulter's liened collateral passes to the house by appropriation under its lien
    at a named valuer's valuation (S3.06's path);
  - **D+1, 9a**: the close-out is valued: the auction prints against the rows' `amount`, plus collateral at its marks;
  - **D+1, 9c**: the waterfall runs (below); unported clients' positions were in the lots, and their margin returns
    net of their own losses;
  - **D+2, 2e**: losses land on named holders: the defaulter's margin and contribution applied, survivors'
    contribution rows written down pro rata by largest remainder (ties `DRV.prorata_lot`), the house's capital
    charged, and the uncovered rest a claim on the defaulter, ranking in its estate (DRV.5). The defaulter's own
    failure path is its kind's (a bank's, S2.08).
  - A lot with no bid above the house's reserve stays in the house's own account, re-offered at the next business
    day's 5c; its losses are the house's while it holds it.
  - **Bilateral**: the survivor terminates the agreement at D+1's 9a at its calculation agent's valuation, nets only
    that agreement's contracts, applies the collateral it holds, and the net is a claim on the defaulter or a
    payment to it at 7 of D+2.
- **Decision forms** (listed in `SHAPES.toml`). Each decision point lives in `if-risk`, and `sys-drv` registers each
  decider kind's rule, since the decisions are DRV's (architecture §3.4); a bank member valuing a lot reads its
  counterparty claims through `sys-bnk`'s `loan_claim_value`:
  - `default_bid`: the member's own value of the lot (`claim_value` over its dealers' own outlooks, a `Value`)
    less the cost of carrying its risk until it can hedge, `½·λ·σ²·Q·τ`, the dealer's reservation price under
    inventory risk (Ho and Stoll, 1981); λ is its management's risk aversion (PREFERENCE), σ² its own outlook's
    width, τ the days to hedge (TECHNOLOGY of the market);
  - **the house's reserve**: its own value of the lot (a `Value` from its own outlooks) less the same cost at the
    house's risk aversion (PREFERENCE of its management) until the next auction: it sells when a bid beats holding,
    never at any price (MKT.17);
  - `accept_port`: accept when the client's expected fees over the member's horizon exceed the capital and liquidity
    the ported positions consume at its required return, and `admit` passes (the value comparison of S1.09's
    decline);
  - `post_collateral` and `choose_clearer`, above, each poster's and client's;
  - review costs: each of these costs staff hours (TECHNOLOGY), paid from the deciding institution's staff that day.
- **The waterfall** (DRV.3, DRV.10), a pure function `waterfall(loss, layers) -> [(layer, used)]` over the rulebook's
  declared layers, in order: the defaulter's margin, its fund contribution, the house's dedicated capital, survivors'
  contributions pro rata, then any assessment the rulebook declares (cash calls on survivors up to its declared
  multiple, issued at 9c and due at the next business day's 2c). A loss beyond the last layer is the event
  `WaterfallExhausted` (DRV.5, DRV.8): until S4.07 a placeholder naming SUP ends the house into an estate at the next
  2e, its open contracts terminated at their last settlement prices, and every claim on it ranked there.
- **The default fund** (DRV.3, DRV.10): on the rulebook's sizing dates at 9c (applied at 9e), sized to cover the
  stressed losses beyond margin of the `cover_n` members with the largest (POLICY of the house), apportioned by each
  member's stressed loss; top-ups are calls due at 2c.
- **Houses are parties** (Law 13): a kind-table individual with equity, investments of cash margin at its declared
  accounts, fees per contract (terms), and its balance sheet read by `phx-acct`. Houses are opened by GEN and founded
  later only through licensing (SUP.9).
- **Streams**: `DRV.default_auction_lot`, `DRV.prorata_lot`, `DRV.clearer_lot`.
- **Opening** (`gen.rs`): one house per exchange per country (ENDOWMENT); members are the banks meeting its declared
  criteria, derived from GEN's banks; capital and a first fund at the rulebook's minimum. No position is open.

**Unit tests**
- `vm_is_mark_change_times_count`: per row, signed by side.
- `vm_paid_equals_received_per_line`: over given marks, calls and failures.
- `netting_within_account_only`: two accounts facing different counterparties never net.
- `fhs_im_rises_with_volatility`: given scenario changes, a higher EWMA ratio gives a higher requirement.
- `fhs_im_falls_with_remaining_life`: over sensitivities of a shorter swap.
- `admission_cuts_beyond_reported_liquidity`.
- `waterfall_layers_in_declared_order`, including exhaustion.
- `prorata_writedown_largest_remainder`: totals not divisible by the members' shares.
- `default_fund_cover_n`.
- `closeout_value_from_prints`.
- `bilateral_closeout_nets_one_agreement`.
- `curve_labels_interpolated_and_extrapolated`: over given traded tenors.
- `ho_stoll_bid_below_value`: the bid falls with λ, σ² and Q.
- `underlying_check_refuses_unprinted`: the pure assembly check refuses terms over a series no declared market
  prints, or an event kind not published.

**Live checks**
- `LC-4-01`: DRV.7 — marks across the two sides of every contract sum to zero, and variation margin paid equals
  received on every line, every business day.
- `LC-4-02`: DRV.8 — every derivative line has two named sides; every account with open rows has IM posted, or a
  recorded reason (a declared bilateral threshold, a mark absent that day).
- `LC-4-10`: DRV.5 — every default is traceable from the unpaid call to the lots, the auction prints, the waterfall's
  layers and the losses on named holders, on the dates of architecture §9.3.
- `LC-4-11`: DRV.6 — every admission cut or refusal is recorded against a member's report, and no member's stressed
  margin exceeded its reported liquid resources after an admitted order.
- `LC-4-12`: IM per contract is published per house and series beside the underlying's measured volatility; it moves
  with it over the run (a flat IM for a year is a finding).
- `LC-4-13`: MKT.20 — every valuation names its valuer, its prints and their ages, and labels what is interpolated or
  extrapolated; no valuation entered a print, an index or a market (MKT.14).

**Budget**
- Derivative lines are held by individuals: about 0.5 M lines (16 MB) and 1 M rows at 24 bytes (24 MB), with about
  5 × 10⁴ distinct terms interned (4 MB): §13.1's lines, relationship-rows and interned-terms lines (S4.07's ledger).
- 9a: a valuation with its sensitivities ≤ 400 core-ns per distinct terms, and ≤ 25 ns per row summed into its
  account; 9c: IM ≤ 1.5 µs per account at a 250-day lookback. At 5 × 10⁴ terms, 1 M rows and 10⁴ accounts that is
  about 60 core-ms, **20 ms** wall a business day: a new §13.2 line, "Derivative marks and margin" (S4.07's ledger).
- The scenario store: lookback × risk factors × 8 bytes (about 1.2 MB), within the markets and marks line.
- Settlement's stream reads the rows' run heads only; a day's coupons and expiries are within its rows-due count.
- Counters, ratcheted: `phx_drv.lines`, `phx_drv.rows`, `phx_drv.terms_valued`, `phx_drv.accounts_margined`,
  `phx_drv.calls`, `phx_drv.call_fails`, `phx_drv.admission_cuts`, `phx_drv.closeouts`, `phx_drv.ports`,
  `phx_drv.marks_absent`.

**Guards**
- PC-60: `MarginCall` is issued only at 9c and due only at the next business day's 2c; no crate but `sys-drv` writes a
  margin line, a default-fund line or a settlement price (assembly and `phx-check`).
- PC-61: an order in a derivative market is S3.03's schedule over the poster's own `Value`; nothing builds an order
  or a limit from a `Valuation` or a valuer's output (compile-level), so no price is solved from a formula or a
  curve (DRX.7).

**Not allowed**:
- a derivative with one side, or an underlying no market prints;
- netting across counterparties;
- a mark from a model with no prints, or a valuation used as a print;
- a call due in the stage it was issued;
- a close-out at any price;
- a house that cannot run out, or a loss beyond the waterfall that lands on nobody.

**Done when**
- [ ] Marks move cash through margin every business day; a member's default runs through the waterfall and can reach
  survivors' contributions.
- [ ] LC-4-01, LC-4-02 and LC-4-10 to LC-4-13 pass once S4.02's markets trade (they apply from S4.02).
- [ ] PC-60 and PC-61 are registered.
- [ ] Two reviews are done.

---

### S4.02 — `sys-drx`: the derivative classes

**Status**: planned

**Clauses**:
- STATE: DRX.1, DRX.2, DRX.4, DRX.5; DRX.3 *(declared here; its markets meet from S5.04, when currencies meet)*.
- MEASURE: DRX.6 *(its FX parts apply from S5.04)*.
- FORBID: DRX.7.
- PRIMITIVE: DRX.8.
- DECISION: ENE.5 *(completes it: a generator's contracts, sold forward on its region's price)*; ENE.6 *(completes
  it: suppliers and large consumers buying under a contract)*; FND.4 *(completes it: hedge funds' derivative
  positions, the fund kind's rule of `take_position`)*.
- The users' position decision has no DECISION clause in the spec; its inputs are read from DRX.1–DRX.5's users and
  reasons, FND.4, INS.5 and PEN.8 (see the notes).

**Architecture**: §4.4, §4.7, §6.1 (5c, 6a, 6b, 6c, 6d, 7, 9a), §8.

**Depends on**: S4.01.

**Goal**: interest-rate swaps, credit default swaps, futures and options, power contracts on each region's
day-ahead price among them (and, from S5.04, FX forwards and swaps), each a class of declared data over DRV (Law 10),
each meeting in its own market, each with participants holding their own reasons on both sides:
- hedgers whose exposures are read from their own books;
- speculators and dealers with their own views and inventories;
- fixed rates, premiums and futures prices formed by those orders, never solved for;
- implied statistics — the swap curve, implied default probabilities, implied volatility, the basis — as reads.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-risk/src/classes.rs` | `ClassDecl`: underlying, legs, market, risk unit, clearing |
| `if-risk/src/position.rs` | the decision point `take_position`; the rule handle `position` (below) |
| `crates/systems/sys-drx/src/decl.rs` | classes, markets, series listing, streams |
| `src/rules/position.rs` | `position`, implementing the handle |
| `src/rules/take_position/{firm,bank}.rs` | the firm and bank kinds' rules (DRX's users) |
| `sys-fnd`, `sys-dlr`, `sys-ene` `src/rules/position.rs` | the fund (FND.4), dealer (DLR.4) and energy (ENE) rules |
| `src/rules/{carry,deliver,exercise,option_value}.rs` | storage arbitrage; delivery; exercise; own option value |
| `src/cds_auction.rs` | the credit-event auction of the defaulted debt |
| `src/expiry.rs` | final settlement, delivery assignment, exercise assignment |
| `src/listing.rs` | series listed by each exchange's rule |
| `src/handlers/*.rs` | 5c decisions and notices; 7 deliveries and exercises; 9d reads |
| `src/metrics.rs` | swap curve, spreads, implied statistics, basis, convergence, hedged against unhedged |
| `crates/systems/sys-dlr/src/rules/quote.rs` | S3.06's quote form over each class's risk unit (inventory as data) |
| `crates/systems/sys-fnd/src/rules/invest.rs` | mandates' permitted classes; positions through `position` |
| `crates/systems/sys-ene/src/rules/offer.rs` | S2.09's offer form, reading the generator's contracts |
| `data/<country>/DRX.toml` | class specifications; listing rules; contract months; auction lag; reach; hours |
| `data/shared/SHAPES.toml` | the forms of `position`, `carry`, `exercise`, `option_value` |

**Design**

- **Classes are data** (DRX.8, Law 10). `ClassDecl { underlying: SeriesKind | EventKind, legs: TermsTemplate, market:
  MarketForm, risk_unit: RiskUnit, clearing: ClearingRule, exercise: Schedule, settlement: legs }`, where the risk
  unit is the sensitivity the class's inventory and exposures are counted in (PV01 per tenor bucket, CS01 per name,
  delta per underlying). Generic code reads the declaration; no mechanism names a class.
  - **Swaps** (DRX.1): fixed against floating on a reference rate S3.09 fixes, on the market's standard dates
    (quarterly effective and maturity dates, as standard swaps trade), so terms are shared across trades.
  - **CDS** (DRX.2): a running premium on the trade's point grid, contingent on a credit event of a named entity
    with public debt (CRD.6's events), single names and index baskets. An index basket is declared by an index
    publisher's composition rule (S3.09's machinery): the names most traded over its window, rolled on its dates.
  - **Futures** (DRX.4): on commodities (a grade at a delivery place), on sovereign bonds (a deliverable basket with
    the exchange's conversion factors), and on equity indices (cash-settled on the index).
  - **Options** (DRX.5): on rates (caps, floors, swaptions), equities, indices and commodity futures; the exercise
    schedule is one date (European) or every business day to expiry (American).
  - **FX** (DRX.3): forwards, swaps and cross-currency swaps, declared now; their markets list no series until S5.04
    gives a country a second currency.
- **Markets** (MKT forms, S0.18), all business-day (B):
  - swaps, CDS and OTC options: the dealer market (MKT.5) — clients ask the dealers in their reach (a declared search
    cost, TECHNOLOGY), each dealer answering from S3.06's quote form over its inventory in the class's risk unit;
    interdealer trading in standard series as a call at 6a (DLR.7);
  - futures and listed options: continuous books with a closing call (MKT.4), the close being the settlement price;
  - series are listed by each exchange's rule (POLICY of the exchange): contract months, and strikes at declared
    intervals around the last settlement price;
  - clearing follows the declared obligation (POLICY of the supervisor): exchange contracts at the exchange's house,
    standard swaps and index CDS between financial parties at a house, the rest bilateral under an agreement.
- **The position rule** (the rule handle `position`, listed in `SHAPES.toml`), read by every user's rule:
  `q* = −h·E + (m − p) ÷ (λ·s²)`, bounded only by declared limits (mandates, position limits). It is S3.03's investor
  schedule (`phx-val`), called with the exposure's hedge `h·E` counted as already held, so the order is a schedule
  from the party's own value with S3.03's no-trade band, never "at market".
  - `E` is the party's exposure in the class's risk unit, read exactly from its own books: a firm's floating-rate
    debt, a bank's repricing gap, an insurer's or scheme's liability PV01 by bucket, a lender's exposure to a
    reference entity, a producer's planned output or input. `h` is the contract's sensitivity per unit of exposure
    from its terms, so no covariance is estimated.
  - `m` is the party's own outlook of the contract's payoff (its heuristic outlook of the underlying, S1.01), `s` its
    width, `p` the quote it faces, and λ its risk aversion (PREFERENCE of its management or mandate).
  - The first term is the minimum-variance hedge (Ederington, 1979); the second is the speculative demand of a
    hedger with a view (Anderson and Danthine, 1981). A party with no exposure holds only the second, so hedgers and
    speculators are one form with different books, and every class has views on both sides (DRX.7).
- **Users**, each deciding on its own review days (lumpy) and when a surprise in the underlying wakes it (REP.35),
  each review costing staff hours (TECHNOLOGY) paid from that day's capacity:
  - **firms** (the firm kind's rule, `sys-drx`): individuals only (large firms); their floating debt, commodity inputs
    and outputs;
  - **banks** (the bank kind's rule, `sys-drx`): their repricing gap from their book's schedule; protection on loans
    they cannot sell, with `E` = exposure at default × the loss given default of their own `LoanAssessment`, read
    through `sys-bnk`'s `loan_claim_value` (architecture §3.4);
  - **funds** (the fund kind's rule, `sys-fnd`, completing FND.4): hedge-fund mandates permitting a class hold its
    second term only; bond and equity funds hedge by mandate; S3.07's `invest` reads the positions as part of the
    fund's book;
  - **dealers** (the dealer kind's rule, `sys-dlr`, S3.06's form): quote both sides, skewed by inventory in the risk
    unit, and hedge their books in the underlying from their own values' deltas;
  - **generators, suppliers and large consumers** (the energy kinds' rule, `sys-ene`, ENE.5, ENE.6): below;
  - **insurers and schemes** join at S4.03 and S4.04 through the same handle.
- **Power contracts** (ENE.5, ENE.6): a class of cash-settled contracts for difference on a region's day-ahead price
  per block and delivery month (the coupled call's prints, S2.09), traded on the exchange's book. A generator sells
  forward its expected output (`E` its planned output by block), a supplier or large consumer buys its expected
  consumption, each through `position`. S2.09's offer form now reads the generator's contracts: its offers stay at
  avoidable cost — a contract for difference changes its revenue, not its marginal cost — and its decisions to run,
  keep or retire a plant read its hedged margin over the contracts' horizon. A large consumer buying under a contract
  still bids its physical need at the auction, its price fixed by the contract's settlement.
- **Futures and storage** (DRX.4, DRX.6): a holder of stock sells a future against it, or buys spot to deliver, when
  the future's price exceeds spot plus its own storage and financing cost to expiry, up to its free storage capacity
  (a `DeclaredLimit` from its held storage units, a physical token); it buys back when the future sits below. The
  form is the theory of storage (Working, 1949; Brennan, 1958), listed in `SHAPES.toml`. Contango within carry and
  backwardation when stocks are short follow from it; nothing forces convergence.
- **Delivery** (DRX.4): within the delivery window a short gives notice (5c) when delivering beats closing out, and
  chooses what to deliver among its holdings of the deliverable grades or bonds by the lowest cost to it, read
  exactly; a bond delivered is a `Covered<Qty>` (S3.05). Longs receiving are assigned by lot
  (`DRX.delivery_assignment`), and the delivery settles at 7 as units against payment at the final settlement price
  (and conversion factor). Index futures settle in cash on the index.
- **Options**:
  - a party's own value of an option is its expected payoff under its own outlook of the underlying — log-normal with
    the outlook's mean and width — discounted at its required return (`claim_value`, S1.01), the premium as expected
    payoff under the holder's own beliefs (Sprenkle, 1961). It is a `Value`, never a price; the premium is what the
    book or the dealers form;
  - writers sell and buyers buy through `position` with that value as `m`;
  - **exercise** (DRX.5) is the holder's decision at 5c of the business day after each exercise date, reading that
    date's settlement price: exercise when the exercise value net of its costs exceeds its own value of holding the
    option on (zero after the last date), listed in `SHAPES.toml`. Writers are assigned by lot
    (`DRX.exercise_assignment`); delivery or cash settles at 7. Nothing exercises by default.
- **Credit events** (DRX.2): a reference entity's default (a public event, CRD.6) stops the premium leg and opens a
  settlement auction on the class's declared lag: a call auction (MKT.3) at 6a of the defaulted debt, where protection
  buyers who hold it may sell (offers as `Covered<Qty>`, S3.05), and dealers and distressed funds bid their own values
  of the claim on the estate (their outlook of its recovery), ties by lot (`DRX.cds_auction_lot`). The final price is
  the auction's print; every contract on the entity pays `notional × (1 − final price ÷ par)` at 7 of the next
  business day. Recovery is what the debt fetched, never fixed (DRX.7).
- **Reads** (DRX.6), at 9d as public statistics of prints: the swap curve (fixed rates by tenor) and its spread over
  the sovereign curve; CDS premiums and their basis over bond spreads; implied default probabilities; implied
  volatility by strike and expiry, and its skew; futures curves against spot and storage cost; convergence at expiry;
  the change in interest cost after a rate move for hedged and unhedged firms.
- **Review costs**: exercise, delivery and carry decisions cost the deciding institution's staff hours (TECHNOLOGY),
  as every review above does.
- **Streams**: `DRX.rfq_lot` (ties among equal dealer quotes), `DRX.delivery_assignment`, `DRX.exercise_assignment`,
  `DRX.cds_auction_lot`.

**Unit tests**
- `position_hedge_term_exact`: with no view, `q* = −h·E`.
- `position_view_term_sign`: the speculative term changes sign with `m − p` and shrinks with λ and s².
- `carry_threshold`: sell the future only above spot plus storage and financing; capacity binds.
- `cheapest_to_deliver_by_own_cost`.
- `delivery_legs_at_final_price`.
- `option_value_rises_with_width`, and a deep in-the-money call's value approaches its forward intrinsic value.
- `exercise_only_when_exercise_beats_holding`.
- `cds_payout_from_auction_price`.
- `premium_leg_stops_at_event`.
- `legs_from_class_decl`: one generic template function builds a swap's, a future's and a CDS's legs from their
  declarations.

**Live checks**
- `LC-4-03`: DRX.6 and DRX.7 — the reads above are published per class and country; every class that traded in a
  month traded on both sides with parties holding no exposure in it (views), and every formed price has its match.
- `LC-4-14`: DRX.4, ENE.5, ENE.6 — every expiry's deliveries and cash settlements are against its final settlement
  price, every power contract settled against its region's day-ahead print, and the futures–spot gap at expiry is
  reported without any order placed to close it.
- `LC-4-15`: DRX.2 — every CDS payout equals notional × (1 − the auction's final price ÷ par), and the auction's
  prints are its match set; recoveries vary across events.
- `LC-4-16`: DRX.5 — every exercise has its holder's decision record; unexercised in-the-money options are counted
  and published.

**Budget**
- Derivative meetings are within architecture §13.2's "Institutions, financial markets, the state" line: about 10⁴
  client requests and 10⁵ book orders a business day, with MKT's unit costs.
- Users' reviews are institutions' decisions: about 10⁴ a day at ≤ 2 µs each.
- Lines and rows are S4.01's.
- Counters, ratcheted: `phx_drx.trades_by_class`, `phx_drx.requests`, `phx_drx.hedge_reviews`,
  `phx_drx.deliveries`, `phx_drx.exercises`, `phx_drx.cds_auctions`, `phx_drx.series_listed`.

**Guards**: PC-61 applies to every class's market.

**Not allowed**:
- a forward from parity, a swap rate solved from a curve, or a premium from a pricing formula;
- a fixed recovery;
- a future forced to converge;
- a class whose only participants are hedgers;
- an exposure estimated where the books state it exactly;
- automatic exercise.

**Done when**
- [ ] Each class forms its prices from both sides' own reasons; implied statistics are reads.
- [ ] LC-4-03 and LC-4-14 to LC-4-16 pass, with DRX.3's FX parts not applicable until S5.04.
- [ ] LC-4-01, LC-4-02 and LC-4-10 to LC-4-13 now apply and pass.
- [ ] Two reviews are done.

---

### S4.03 — `sys-ins`: insurance and reinsurance

**Status**: planned

**Clauses**:
- STATE: INS.1, INS.2.
- DECISION: INS.3, INS.4, INS.5; HH.11.
- PROCESS: INS.6; INS.7 *(the insolvency is stated here; its resolution is SUP's at S4.07, and until then a
  placeholder naming SUP ends the insurer into an estate)*.
- INVARIANT: INS.8.
- MEASURE: INS.9.
- FORBID: INS.10.
- PRIMITIVE: INS.11.
- MMK.3 *(completes it: insurers lend cash in repo and buy short paper through S3.01's `place_cash`, the insurer kind
  registered by data)*; RAT.4 *(completes it: insurers' mandates refer to ratings, a downgrade below a mandate's
  minimum a breach cured by the insurer's own sales)*; CHN.3 *(part: harm to third parties, the hazard liability
  cover answers, spec Appendix E 34, declared here with that cover)*.
- This step retires S0.25's placeholder deferring catastrophe claims to INS.

**Architecture**: §4.2 (claims as messages), §4.4 (the algebra's contingent legs), §4.5, §6.1 (3e, 5c, 7, 9a, 9b,
9c, 2e), §7.3, §7.5, §7.10, §9.1.

**Depends on**: S4.02.

**Goal**: insurers pool this world's hazards and can fail:
- insurers post premiums from their own claims experience, their own view of the hazard, the capital each policy
  consumes and the return they require, decline what they cannot stand behind, and buy reinsurance for their peaks;
- households and firms buy cover from their own view of their exposure and their risk aversion, or go without;
- every claim is opened by a recorded hazard event on a covered subject and paid on the policy's terms;
- insurers invest to match their liabilities, which are valued at the day's traded rates;
- an insurer whose assets fall below its liabilities is insolvent.

**Files**

| File | Purpose |
| --- | --- |
| `crates/kernel/phx-ledger/src/algebra.rs` | `ValuedLoss`, `WhileState`: S0.14's declared amount forms, filled |
| `crates/interfaces/if-risk/src/insurance.rs` | `CoverKindDecl`; policy, annuity and treaty lines; the insurer |
| `if-risk/src/claims.rs` | the `Claim` message kind; `Application` (day-local, counted) |
| `if-risk/src/cover.rs` | the decision point `insure` (households' and firms' rules); the handle `cover_demand` |
| `crates/systems/sys-hh/src/rules/insure.rs` | HH.11: the household kind's rule |
| `crates/systems/sys-ins/src/rules/insure_firm.rs` | INS.4: the firm kind's rule |
| `crates/systems/sys-ins/src/liability.rs` | the hazard of harm to third parties; victims; the law's damages |
| `crates/systems/sys-ins/src/rules/price.rs` | INS.3: premiums per (cover, class) |
| `src/rules/underwrite.rs` | INS.3: accept or decline an application |
| `src/rules/{reinsure,quote_layer}.rs` | the cedent's purchase; the reinsurer's quote |
| `src/rules/invest.rs` | INS.5 |
| `src/rules/cover_demand.rs` | the buyers' form, as a rule handle |
| `src/experience.rs` | claims experience per (insurer, cover, class, year); credibility |
| `src/claims.rs` | opening claims at 3e; adjusting and paying at 5c and 7; reinsurance recoveries |
| `src/valuers/{adjuster,actuary}.rs` | the loss adjuster's and the actuary's methods (MKT.20) |
| `src/solvency.rs` | the solvency requirement; the insolvency fact; the placeholder ending |
| `src/handlers/*.rs` | by sub-step, below |
| `src/audit.rs` | `INS.claim_has_event`, `INS.premium_has_payer` |
| `src/gen.rs` | insurers, policies, annuities, treaties, opening experience |
| `data/<country>/INS.toml` | cover kinds; points; rating factors; solvency rules; hours; lags; renewal band |
| `data/<country>/gen/INS.toml` | insurers by class of business; policy holdings by household and firm kind |
| `data/shared/SHAPES.toml` | the forms of `price`, `underwrite`, `reinsure`, `quote_layer`, `invest`, `insure` |

**Design**

- **Cover kinds are data** (INS.1, INS.11, Law 10). `CoverKindDecl { covered, perils: [HazardId], legs, term,
  rating_factors, renewable }`:
  - `covered`: an adult role's life or health state, the dwelling role's unit, a held class's units (plant,
    vehicles, cargo), a named unit, or a party's liability for harm its vehicles, premises or work do to others;
  - `perils`: the declared hazards (CHN.3) whose events on the covered subject pay — mortality (POP.3), illness and
    disability (POP.4), damage and accidents, equipment failure, catastrophes (S0.13), harm to third parties;
  - `legs`: S0.14's `Contingent { event, amount }`, the amount being a fixed sum (life), a `ValuedLoss { valuer,
    limit, deductible }` (property: the adjuster's valuation, bound by the sum insured, less the deductible), or a
    `WhileState { benefit, waiting, state }` (income protection: a benefit per period while the state lasts, after a
    waiting period). Annuities are a per-time amount while the annuitant lives;
  - `rating_factors`: what an application may carry and a premium may depend on (POLICY of the law, INS.11);
  - reinsurance is a cover kind between insurers: a layer (retention, limit per event, reinstatements) over the
    cedent's paid claims on declared perils in a region.
- **Lines** (INS.1, REP.3): a policy line is (cover, rating class, sum point, deductible point, premium point, renewal
  band), with the insurers writing those terms on one side and the holders on the other, each with a count. Rows are
  16 bytes, in the holder's due-day run. Premium, sum and deductible points are each trade's grids (REP.34, POLICY of
  the trade); the renewal band is RESOLUTION (`INS.renewal_band`), tested on the ladder. Income-protection claimants
  are rows of role `claimant` on their line; annuities are lines of (payment point, indexation, age class at purchase,
  start band). A dwelling policy's class names the zone and dwelling class, so a flood's draws meet the right policies
  (architecture §7.8).
- **Premiums** (INS.8): dues on the line's schedule, pooled per holder (REP.8), paid at 7 and credited to the insurer
  side by its counts. A premium failed past the terms' grace lapses the cover of the members it failed for; the
  insurer losing the count is drawn (`INS.lapse_pairing`).
- **Pricing** (`price`, INS.3), a lumpy decision of each insurer on its review days, and when a surprise in its
  claims wakes it (a catastrophe), per (cover, rating class):
  - **expected claims**: frequency × severity by credibility, `Z·own + (1 − Z)·collective` with `Z = n ÷ (n + k)`,
    `n` its exposure-years in the class and `k` estimated from its own book as the ratio of the within-class to the
    between-class variance of its claims (Bühlmann and Straub, 1970). The collective is the public record of the
    peril: STA's published mortality and illness rates by age, and the public catastrophe events of the class's
    zone over the opening history and the run. No hazard table is read directly (Law 12);
  - **premium**: the present value at the day's curves (S3.03's sovereign and S4.01's swap curve) of expected claims
    and expenses over the term, plus the capital the policy consumes (its marginal solvency charge, POLICY of the
    supervisor, times its target buffer, PREFERENCE of its management) times its required return less the yield it
    earns on that capital: discounted cash-flow pricing (Myers and Cohn, 1987);
  - posted at the nearest premium point, moving only if the expected gain over the review interval exceeds its
    repricing cost in staff hours (TECHNOLOGY): S1.03's menu-cost form (Golosov and Lucas, 2007).
  - A class it will not write is posted as not offered, a visible absence (Law 8).
- **Underwriting** (`underwrite`, INS.3): applications are day-local counted messages sent at 5c and answered at the
  next business day's 5c. The insurer accepts unless writing them would take its solvency ratio below its target
  buffer, or its sum insured in a (zone, peril) above its risk appetite (PREFERENCE) times its capital: capital is
  costly and concentration is limited by it (Froot and Stein, 1998). When the appetite binds partway, the day's
  applicants are served by lot (`INS.application_lot`). Accepted policies are written at 7 with the first premium.
- **Buyers** (`insure`, HH.11, INS.4), a decision point in `if-risk`, the household kind's rule registered by
  `sys-hh` (HH.11 is HH's) and the firm kind's by `sys-ins` (INS.4), both calling `sys-ins`'s rule handle
  `cover_demand`; for a household, a lumpy decision on
  its `insure` review days, and on needs: a
  policy's renewal (a notice from the insurer with its current point), a dwelling bought, a child born, a surprise in
  the covered peril (a catastrophe in its zone). Evaluated per (row, decision, profile combination) with counts,
  per cover kind whose subject its members have:
  - **willingness to pay**, `p·L + ½·ρ·p·(1 − p)·L² ÷ W` (Mossin, 1968; the Arrow–Pratt approximation): `p` its own
    outlook of the peril's frequency for its profile (its heuristic over the published rate for its age class and
    health, or its zone's catastrophe history, S1.01), `L` the loss it would bear (for life, the income outlook of
    that adult less the needs that end with it; for income protection, that income over the expected spell less the
    benefits it is eligible for; for a dwelling, the rebuilding cost at the construction prices it sees), ρ its risk
    aversion and `W` its wealth;
  - it takes the (sum, deductible) whose willingness to pay most exceeds the premium; the insurer is a multinomial
    count over the insurers in its reach offering that class, by premium and taste (REP.22, `INS.insurer_taste`),
    among those leaving a positive surplus, or none;
  - adding, dropping or moving cover changes attachments in place (architecture §7.5); no key changes, so no part;
  - the review costs hours at its value of leisure (TECHNOLOGY), as every household review does.
  - **Firms** (the firm kind's rule, INS.4) use the same form with their management's risk aversion over their
    plant, vehicles, cargo and liability, the review in staff hours; households' covers include their vehicles' and
    dwellings' liability.
- **Harm to third parties** (CHN.3, spec Appendix E 34), a hazard `sys-ins` declares with the cover that answers it:
  - it acts on holdings of the declared classes (vehicles, premises) and on firms' work (their staff hours), at a
    rate per unit that is TECHNOLOGY (accident statistics, with sources), screened like any hazard (S0.22);
  - a hit names its victim: for damage to property, a holder drawn from the (zone, class) index of the harm's zone
    (architecture §7.10), its units damaged by the declared severity; for injury, a member drawn from the zone's
    dwelling-role members, whose illness outcome `sys-dem` applies (POP.4); streams `INS.third_party_harm` and
    `INS.victim_pairing`;
  - the liable party owes the victim damages by the law's rule (POLICY), valued by the law's named assessor from
    prints (repair costs, the victim's lost income) and written as a claim line from the liable party to the victim,
    due on the law's schedule; an insured liable party's liability cover opens a `Claim` like any other, and an
    uninsured one pays or fails, the claim ranking in its estate (S2.04).
- **Claims** (INS.6, INS.8):
  - **attachments are joint with what hazards read**: `sys-ins` declares its policy attachments in the adult role's
    profile group with health and age, and in the dwelling role's group with zone and class (REP.32), so a hazard's
    pick of hit members carries their cover;
  - **opened at 3e**: every hit on a covered subject — a death, an illness onset, damage or an accident to a
    dwelling, plant, vehicle or cargo, and a catastrophe's losses at owners (architecture §7.10) — carries the hit
    members' attachments, drawn jointly within their role group by the hazard's own pick (REP.32). `sys-ins` opens one
    `Claim` per (policy line, holder, event) with the count and the loss measure (units or condition steps lost, the
    state entered). The paying insurers are drawn from the line's insurer side (`INS.claim_pairing`). A loss below
    its deductible opens nothing. This retires S0.25's placeholder;
  - **adjusted at 5c** after the cover kind's adjusting lag (TECHNOLOGY): the amount per member by the terms — the
    fixed sum; the adjuster's valuation (a named valuer: repair or rebuilding cost of what was lost, from the day's
    construction and goods prints) bound by the sum insured (`DeclaredLimit::bind` from the terms) less the
    deductible; or, for income protection, the member joins the claimant rows and is paid the benefit on the line's
    dates until its recovery, its death or the term. Rounded per member, times the count;
  - **paid at 7**, to the household or, if it has ended, its estate. A death also ends the member's annuity rows.
  - **Reinsurance recoveries**: per treaty and event, `layer(paid, retention, limit)` over the cedent's paid claims
    on the covered perils in the region, claimed on the treaty's settlement dates and paid at 7.
- **Reinsurance** (`reinsure`, `quote_layer`, INS.3), on the market's declared renewal dates (POLICY of the trade):
  the cedent asks the reinsurers in its reach (MKT.7); each quotes the layer by the same discounted cash-flow form
  over its own view of the region's catastrophe losses (credibility over public events and its own book) and the
  capital the layer consumes; the cedent buys a layer when its price is below the capital it relieves times its
  required return (Froot, 2001), ties by lot (`INS.reinsurer_lot`). A reinsurer is an insurer (Law 10).
- **Investment** (`invest`, INS.5), on its review days and when a surprise in rates wakes it:
  - swap positions through S4.02's `position` with `E` its liability PV01 by bucket, and bond orders, so its assets'
    PV01 matches its hedge (Redington, 1952), a structural buyer of long bonds and receiver of fixed;
  - the rest by S3.03's investor schedules over its own outlooks within its mandate and the regulator's eligible
    assets and limits (POLICY, `DeclaredLimit`s), class weights by mean–variance (Markowitz, 1952);
  - its mandate's rating references (RAT.4) read S3.10's ratings: a downgrade below a mandate's minimum is a breach,
    cured by its own sales within the mandate's cure period;
  - cash beyond its liquidity target is lent in repo and in short paper by S3.01's `place_cash` (MMK.3), the insurer
    kind registered by data.
- **Liabilities and solvency** (INS.2, INS.7, INS.10):
  - at 9a on each insurer's valuation dates (declared, spread over the month, N8.9), its actuary — a named valuer
    registered by `sys-ins` — values open claims at their adjusters' valuations, unearned premium, and the present
    value of future claims less future premiums on in-force long-term lines (life, annuities, claimants), at the day's
    traded curves and on the insurer's experience basis, each line at its rating class's model point (actuarial
    practice), beyond the longest traded point extrapolating in its own declared method and labelling it (MKT.20).
    Between valuation dates, a roll-forward by bucket PV01 × the day's curve moves, labelled so;
  - 9b: its accounts (ACC) and its solvency ratio against the requirement (POLICY); assets below liabilities is the
    stated insolvency, the fact `insurer_insolvent`, written by `sys-ins` at 9b for the tests of 9c to read;
  - until S4.07, a placeholder naming SUP: `sys-ins`'s 9c handler finds the fact and triggers the ending in the same
    handler (applied at 9e), and at the next business day's 2e the insurer ends into one estate (S2.04),
    its policies' premium legs stop, and open claims and the value of in-force benefits become claims ranked by the
    law (POLICY), with annuitants and claimants on claim lines of the estate. Nothing vanishes (Law 13).
- **Review costs**: an insurer's `price`, `reinsure`, `quote_layer` and `invest` reviews cost its staff hours
  (TECHNOLOGY), paid from that day's capacity; underwriting and adjusting are operations, their hours per application
  and per claim a running cost (TECHNOLOGY).
- **Streams**: `INS.third_party_harm`, `INS.victim_pairing`, `INS.insurer_taste`, `INS.claim_pairing`,
  `INS.lapse_pairing`, `INS.application_lot`,
  `INS.reinsurer_lot`.
- **Opening** (`gen.rs`): insurers by class of business with drawn sizes; households and firms draw their policies,
  annuities and claims in payment with their terms over the points (S0.25's procedure), and the insurer sides are
  apportioned by drawn market shares; treaties; asset portfolios balanced (GEN.4). Each insurer's opening experience
  is the expected claims of its opening book under the declared hazards over `opening_history_years` (ENDOWMENT), and
  the opening history's catastrophe events are drawn from S0.13's declared process over the same years (stream
  `GEO.opening_catastrophe`), public.

**The household record's addition** (architecture §13.1, counted against S1.12's table):

| Item | Bytes |
| --- | --- |
| The record before Stage 4 (S1.12 504; S2.05, S2.06 +64; S3.05 +16) | 584 |
| Review exposure, `insure` | 8 |
| Attention for `insure`: own rate and g_k | 8 |
| Head of the due-day runs (earliest due day, `u32`, padded) | 8 |
| **Through S4.03** | **608** |

Policy and annuity rows live in the arena. The firm record adds the same 16 bytes for the firm's `insure`, against
S1.03's 500; its due-day head is S2.02's.

**Unit tests**
- `credibility_weight_and_k`: Z rises with exposure; k from given within- and between-class variances.
- `dcf_premium_components`.
- `premium_point_moves_past_repricing_cost`.
- `claim_fixed_sum`; `claim_valued_loss_bound_and_deductible`; `claim_below_deductible_opens_nothing`.
- `claim_requires_event` (compile-fail): a `Claim` cannot be built without an `EventRef` of a declared peril.
- `while_state_pays_after_waiting`.
- `mossin_wtp`: rises with ρ and with the loss's variance; falls with wealth.
- `layer_recovery_retention_and_limit`.
- `liability_moves_with_rates`: over a given curve shifted down, the value rises.
- `roll_forward_first_order`.
- `appetite_binds_by_lot`.
- `redington_bucket_match`.

**Live checks**
- `LC-4-04`: INS.8 — every claim paid follows a recorded hazard event on a covered subject, and every premium came
  from a named holder on a policy line.
- `LC-4-05`: INS.9 — premiums by class and zone before and after each catastrophe, solvency ratios against the curve,
  and the share of catastrophe losses reinsurers bore are published.
- `LC-4-17`: HH.11, INS.4 — every policy written has its holder's `insure` decision record and its insurer's
  acceptance; declines and classes not offered are counted per insurer and published.
- `LC-4-18`: INS.2, INS.10 — every liability valuation shows its curve's prints and their ages; none used a rate that
  is not the day's curve.
- `LC-4-19`: INS.6 — every catastrophe's losses on insured units reach claims on the lines covering the struck (zone,
  class), and the treaties' recoveries follow their terms; no estate or owner waits on S0.25's placeholder.

**Budget**
- Household record +24 bytes (17 MB); firm record +16 bytes (4 MB).
- Policy rows: about 7 per household cell (life and income protection per distinct class among its adults,
  dwelling cover per distinct (zone, class)), 4.9 M rows, 78 MB, and 29 MB of holder lists at 16 and 6 bytes;
  measured on the rows-per-cell curve (architecture §14.6) and counted in S4.07's re-estimate of §13.1.
- Settlement: policy rows are in due-day runs, so 7a reads one head per household (2 ns) and only the rows due:
  premiums spread over the month's business days, about 0.25 M rows a day (10 ns) and their pooled payments.
- Insure evaluations: about 0.2 M a day within "Occasion evaluations"; claims about 5 × 10⁴ a day at ≤ 300 ns;
  valuations on valuation dates only.
- Counters, ratcheted: `phx_ins.policy_lines`, `phx_ins.policy_rows_per_household_cell`, `phx_ins.claims_opened`,
  `phx_ins.claims_paid`, `phx_ins.insure_evaluations`, `phx_ins.declines`, `phx_ins.lapses`,
  `phx_ins.lines_valued`, `phx_pop.bytes_per_household_cell`, `phx_pop.bytes_per_firm_cell`.

**Guards**
- PC-62: a `Claim` is built only from an `EventRef` of a declared peril and a policy row, and no function that reads a
  premium returns a claim amount (compile-level and a signature check).
- PC-63: liability discounting takes a `TradedCurve`, which only S3.03's curve publisher (and a valuer's labelled
  extrapolation of it) builds from prints; no rate primitive or literal can make one (compile-level).

**Not allowed**:
- a claim as a share of premium, or a claim without an event;
- a fixed discount rate, or an insurer reading the hazard table instead of its experience and published records;
- a policy without a named holder, or a liability without beneficiaries;
- a household deciding at a cell's average across a deductible or a sum insured;
- an insurer that cannot fail.

**Done when**
- [ ] Hazards become claims at the right insurers; premiums rise after catastrophes; a catastrophe can break an
  under-reinsured insurer, which ends into an estate until S4.07 resolves it.
- [ ] LC-4-04, LC-4-05 and LC-4-17 to LC-4-19 pass.
- [ ] PC-62 and PC-63 are registered.
- [ ] Two reviews are done.

---

### S4.04 — `sys-pen`: pensions

**Status**: planned

**Clauses**:
- STATE: PEN.1, PEN.2, PEN.3.
- DECISION: PEN.8, PEN.9, PEN.10; HH.7 *(part: pensions among a household's savings)*.
- PROCESS: PEN.4, PEN.5; SOC.3 *(part: the state pension's claim at pension age, brought into `sys-soc` here; SOC
  completes at S5.02)*.
- MEASURE: PEN.6.
- FORBID: PEN.7.
- PRIMITIVE: PEN.11.
- This step retires every placeholder naming PEN; the plan declares none before it (see the notes on retirees'
  income before S4.04). It introduces one naming SOC (S5.02): an earnings-related state pension pays its flat part.

**Architecture**: §4.3 (levies and follow-ons), §4.4 (composite instructions, the split at a kink), §4.5, §6.1 (4a,
5c, 7, 7e, 9a, 9c, 2c, 2e), §7.4, §7.8, §9.1.

**Depends on**: S4.03.

**Goal**: people provide for old age in the three ways the spec names:
- a state pension paid by SOC to named pensioners from current receipts;
- defined-benefit schemes that owe each member a benefit schedule, sponsored by employers that must repair
  deficits, with liabilities valued at the day's traded rates and trustees who hedge them;
- defined-contribution accounts as members' fund units, fed at payroll, drawn down or annuitised at retirement.

Retirement changes a household's income from wages to the pensions of the kinds it holds.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-labour/src/terms.rs` | the employment line's pension terms (see the notes) |
| `crates/interfaces/if-risk/src/pensions.rs` | the scheme kind; DB and DC line kinds; `AccruedPension`; levies |
| `if-risk/src/guarantee.rs` | the guarantee-fund kind; compensation lines; the recovery-plan line |
| `if-risk/src/pension_decisions.rs` | decision points `trustees`, `repair`, `pension`, `at_retirement` |
| `crates/systems/sys-soc/src/benefits/state_pension.rs` | PEN.1: the state pension as a declared benefit |
| `crates/systems/sys-pen/src/rules/trustees.rs` | PEN.8 |
| `src/rules/repair.rs` | PEN.9: the firm and agency kinds' rules |
| `src/rules/{member,at_retirement}.rs` | PEN.10: the household kind's rules |
| `src/levy.rs` | contribution levies and their follow-ons (units, accrued rights) |
| `src/funding.rs` | valuations, technical provisions, recovery plans, contribution schedules |
| `src/valuers/actuary.rs` | the scheme actuary's method (MKT.20) |
| `src/guarantee.rs` | the guarantee fund: levy, assessment, transfer, compensation |
| `src/failure.rs` | a sponsor's end, the scheme's claim, the scheme's end |
| `src/handlers/*.rs` | by sub-step, below |
| `src/audit.rs` | `PEN.liability_has_members`, `PEN.contribution_lands` |
| `src/gen.rs` | schemes, rights, pensions in payment, pots, the guarantee fund |
| `crates/systems/sys-hh/src/rules/reservation.rs` | the reservation reads the pensions retirement would bring |
| `data/<country>/PEN.toml` | pension law; auto-enrolment minima; guarantee caps and levy; trustee types; hours |
| `data/<country>/SOC.toml` | the state pension rule and age (POLICY) |
| `data/<country>/gen/PEN.toml` | schemes, memberships, rights, pots and assets, with sources |
| `data/shared/SHAPES.toml` | the forms of `trustees`, `repair`, `pension`, `at_retirement` |

**Design**

- **The state pension** (PEN.1, SOC.3): a benefit of `sys-soc` declared by its rule (POLICY of the parliament). The
  state pension age is an age-class boundary of the key, so members crossing it on their birthdays are parts (REP.25);
  each claims by S1.11's claim form (value against claiming hours) and is paid a standing flow to its household until
  death, out of the treasury's receipts. A rule with an earnings-related part pays its flat part until S5.02
  (placeholder naming SOC).
- **Pension terms are job terms**: the employment line (LAB.1) carries the scheme a job belongs to and its
  contribution rates, so members' schemes are exact for every pairing REP.23 leaves unrecorded, as their wage is.
- **Defined benefit** (PEN.2, PEN.7):
  - a scheme is a kind-table individual sponsored by an individual employer (a large firm or a public agency);
  - its benefit design is terms: accrual rate, pension age, revaluation index, indexation in payment, survivor share,
    early-retirement reduction. Rights accrue as revalued career-average amounts, the owner's decision (spec Appendix
    E 32); the opening world's final-salary schemes are carried as their career-average equivalents;
  - lines: active, deferred and pensioner, per (scheme, section); rows carry `balance: AccruedPension`, the holder's
    total annual pension, in money per year, 24 bytes, in the holder's due-day run. `AccruedPension` is not money and
    cannot become money but by the actuary's valuation (PEN.7);
  - hiring onto a DB job attaches the member to its scheme's active line (in place, architecture §7.5);
  - **accrual** (PEN.4): the payroll levy's follow-on at 7e writes, per member of each employment row in a DB job,
    `accrual rate × pensionable pay`, rounded per member, times the count, as a `Row` leg (S0.15) adding to the
    holder's active row and the scheme's side alike, pooled per (cell, row). Revaluation is an accrual posted lazily
    on the index's dates (architecture §7.4);
  - leaving a job moves the leavers' share `k·balance ÷ c` (REP.9, `c` the row's count) to the deferred line;
    retiring moves it to the pensioner line, reduced by the early-retirement term where it applies;
  - **benefits** are dues on the pensioner line, `balance ÷ count` per member a year, paid monthly at 7; a death
    removes the member's share, and the survivor share moves to the surviving adult role's pensioner row.
- **Defined contribution** (PEN.3): a scheme (an employer trust or a multi-employer trust) has a fund menu; membership
  is a row on the line (scheme, fund), 16 bytes; the pot is a holding row of that fund's units with a member count
  (S3.05), flagged `pension`, outside its liquid wealth until pension age. It sets no participation bit of the key,
  since no liquid-portfolio decision reads it; its per-member quantity is a position with steps.
  - **Contributions** (PEN.4): levies at payroll (architecture §4.3) from the job's terms — the employee's deducted,
    the employer's added, at least the law's minimum (POLICY) — plus the member's chosen extra as a standing flow.
    The follow-on at 7e is a subscription order, one per (scheme, fund) and day, dealt by S3.07's forward pricing
    (taken with the next business day's orders, dealt at the following 6a at that day's 9a value), with units allotted
    per holder row and the rounding residue landing on the fund (FND.9).
- **Retirement** (PEN.4): LAB's `retire` (S1.08) reads the reservation, which now counts the pensions retiring would
  bring (DB rows, the pot, the state pension). When the job ends at 4a:
  - DB rights move to the pensioner line;
  - the member's `at_retirement` occasion (a need, 5c) chooses an annuity or drawdown (below). An annuity is one
    composite instruction written by `sys-pen`: it draws on the fund's redemption commitment for the member's units
    and the chosen insurer's annuity commitment at its quoted point (S4.03), and settles at 7 when the redemption
    settles on the fund's convention — units redeemed, premium paid, annuity row written, all or none. Drawdown lifts
    the pot's flag, and the household's savings decisions (HH.7) draw it;
  - the state pension is claimed at its own age.
- **Where the decisions live**: `trustees`, `repair`, `pension` and `at_retirement` are decision points in
  `if-risk` (their types are pension terms and fund units); every kind's rule — the scheme's, the sponsors' (firms
  and agencies) and the members' — is registered by `sys-pen`, since the decisions are PEN's (architecture §3.4).
- **Trustees** (`trustees`, PEN.8), a lumpy decision of each scheme on its review days, and when a surprise in its
  funding ratio or a margin call wakes it, from its liabilities and their PV01 by bucket, its funding ratio, its
  mandate (data) and the sponsor's strength:
  - **allocation**: the liability-hedge ratio and the growth share that maximise `E[surplus return] − (λ ÷ 2)·
    Var(surplus)` over its own outlooks (Sharpe and Tint, 1990), λ the trustee type's risk aversion (PREFERENCE). The
    surplus counts the sponsor's promised repair contributions at the trustees' own assessment of the sponsor — an
    adaptive class frequency learned from published accounts and ratings, as S1.09's assessment is (S3.10);
  - the hedge is S4.02's `position` with `E` = liability PV01 × the hedge ratio, plus bond orders and repo (MMK) where
    the mandate allows leverage;
  - **the collateral buffer** (PEN.5): liquid assets covering the variation margin its swaps and repo would need under
    a rise in the long rate of its prudence multiple (PREFERENCE) times its own outlook's width, over the days to its
    next review; a regulator's minimum buffer (POLICY), where declared, binds as a `DeclaredLimit`;
  - its reviews cost adviser hours bought from a named firm at its posted price.
- **Funding** (PEN.11), on the law's valuation dates: the scheme actuary values the liabilities (below); a deficit
  needs a recovery plan within the law's longest period (POLICY), and the actuary's schedule of contributions sets the
  employer's rate for future accrual, a fact of the scheme the payroll levy reads.
- **The sponsor** (`repair`, PEN.9), a lumpy decision on each valuation date: the repair period between the shortest
  it can pay and the law's longest, paying faster when its marginal cost of funds (its funding options, FRM.9) is
  below the deficit's own cost — the curve's rate over the period plus the guarantee levy's charge on underfunding —
  and slower when its investment and dividends earn more: the deficit treated as corporate debt (Treynor, 1977; Bulow,
  1982). The plan is a line (sponsor → scheme) paid on its schedule at 7. Staff hours.
- **Members** (`pension`, PEN.10, HH.7), a lumpy kind of the household, on its review days and on needs (a job with a
  DC scheme started):
  - **extra contributions**: of its saving beyond its target buffer m* (S1.12's solution), the part whose value per
    unit is higher in the pension — the employer's match (terms), the lock-up to pension age at its patience, tax
    relief from S5.01 — than in liquid savings (Carroll, 1997; VAL.8). The result is a standing flow into the scheme;
  - **fund**: the menu fund whose risky share is nearest `(μ − r) ÷ (ρ·σ²) × (1 + H ÷ F)`, with μ and σ² its own
    outlooks of the fund's return, H its human capital (its income outlook to pension age at its patience) and F its
    pot (Merton, 1969; Bodie, Merton and Samuelson, 1992), with taste draws (REP.22, `PEN.fund_taste`). A switch moves
    the attachment in place and the units by redemption and subscription at NAV;
  - **`at_retirement`**: an annuity when its money's worth under the member's own survival outlook (the published
    mortality for its birth year and health, S1.14), plus the longevity insurance its risk aversion values, exceeds
    one; else drawdown (Yaari, 1965; Mitchell, Poterba, Warshawsky and Brown, 1999). The insurer is chosen by quote
    and taste (`PEN.annuity_taste`);
  - reviews cost hours at its value of leisure (TECHNOLOGY).
- **Valuation** (PEN.2, MKT.20): the scheme actuary — a named valuer registered by `sys-pen` — values each DB line at
  its model point from its benefit schedule, the published mortality and the day's traded curves (S3.03, S4.01;
  PC-63), in full on valuation dates and daily by a roll-forward of bucket PV01 × the day's curve moves, labelled so.
  A falling rate raises the liability and can open a deficit (PEN.5); a hedged scheme's swaps call margin in cash when
  rates rise, met from its buffer; a buffer short of a call sells at the next 5c and 6a (L2) or defaults on the call
  (S4.01).
- **Failure** (architecture §9.1):
  - a sponsor that ends leaves the scheme a claim on its estate: the deficit on the law's buyout basis, valued by the
    actuary, ranked by the law;
  - after that claim's distribution, a scheme that cannot pay its benefits at the guarantee fund's protected level
    transfers to it: assets by line transfer, and each member row split at a kink per member (architecture §4.4) —
    benefits up to the law's cap (`DeclaredLimit::bind` per member) to the fund's compensation lines, the rest to
    claim lines on the scheme's estate. Then the scheme ends;
  - a scheme whose sponsor lives is never transferred: it is underfunded and repaired (PEN.7);
  - **the guarantee fund** (a party, POLICY) levies schemes annually by the law's risk formula over their underfunding
    and their sponsors' published ratings, demanded at 9c and due at the next business day's 2c; it pays compensation
    on its lines at 7; a short fund draws its treasury backstop where the law declares one.
- **Streams**: `PEN.fund_taste`, `PEN.annuity_taste`.
- **Opening** (`gen.rs`): schemes and sponsors; employment lines' pension terms drawn with the lines; accrued rights,
  deferred rights and pensions in payment drawn with households (by age, occupation family and sector, from pension
  surveys), the scheme side derived; DC pots drawn as fund-unit holdings; scheme assets from allocation sources,
  balanced (GEN.4); state pensioners as the benefit's opening claimants; the guarantee fund with its opening fund.

**The household record's addition** (counted against S1.12's table):

| Item | Bytes |
| --- | --- |
| Through S4.03 | 608 |
| Review exposure, `pension` | 8 |
| Attention for `pension`: own rate and g_k | 8 |
| **Through Stage 4** | **624** |

The extra contribution is a standing rate within S1.12's fourteen, or its overflow map (counted). `at_retirement` is a
need, with no exposure. Scheme rows live in the arena.

**Unit tests**
- `db_accrual_per_member_times_count`.
- `revaluation_lazy_equals_daily`.
- `leaver_share_moves_exactly`: `k·balance ÷ c` with a balance not divisible by c.
- `accrued_pension_is_not_money` (compile-fail).
- `dc_units_at_next_nav_residue_on_fund`.
- `recovery_plan_within_law`.
- `repair_pace_by_cost_of_funds`.
- `surplus_hedge_ratio_rises_with_risk_aversion`.
- `buffer_covers_stressed_margin`.
- `merton_share_with_human_capital`.
- `annuity_moneys_worth_decision`.
- `compensation_split_at_cap_per_member`.
- `roll_forward_first_order`.

**Live checks**
- `LC-4-06`: PEN.6 — funding ratios against the curve, sponsors' contributions against their investment, and
  state-pension spending against the population over pension age are published.
- `LC-4-20`: PEN.7 — every DB line with a liability has members; every contribution levied at payroll became units or
  accrued rights by its follow-on the same day; no DB benefit is paid from a cash balance.
- `LC-4-21`: PEN.4 — members retiring move from wages to the pensions of the kinds they hold (published by kind);
  every benefit payment names a living member's household or a survivor; deaths end benefits.
- `LC-4-22`: PEN.2 — every sponsor's end leaves a claim in its estate; every transfer to the guarantee fund shows its
  split at the cap per member; no right vanished.

**Budget**
- Household record +16 bytes (11 MB); Stage 4's total +40 bytes, 584 → 624 (28 MB).
- Scheme rows: about 4 per household cell (the distinct schemes of its adults' jobs, past jobs and pensions), 2.8 M
  rows at 24 or 16 bytes, 56 MB, and 17 MB of holder lists; employment lines split by pension terms. Both measured on
  the rows-per-cell curve and counted in S4.07's re-estimate.
- Settlement: scheme rows are in the due-day runs; benefits fall on the schemes' pay days, most at month's end, so
  their rows (about 0.5 M pensioner rows) are read on the heavy day, not the median one.
- Follow-ons at 7e: one write per (cell, scheme row) on paydays at ≤ 30 ns; about 1 M on a heavy payday (10 ms wall).
- Valuations on valuation dates; the daily roll-forward ≤ 50 ns per line.
- Counters, ratcheted: `phx_pen.scheme_rows_per_household_cell`, `phx_pen.follow_on_writes`, `phx_pen.retirements`,
  `phx_pen.annuities_bought`, `phx_pen.drawdowns`, `phx_pen.recovery_plans`, `phx_pen.guarantee_transfers`,
  `phx_soc.state_pension_claims`, `phx_pop.standing_rate_overflows`.

**Guards**: PC-64: `AccruedPension` has no conversion to `Money` or to a deposit balance except through the actuary's
valuation (compile-level), so no DB promise is a cash balance (PEN.7).

**Not allowed**:
- a pension liability without members, or a DB promise carried as a cash balance;
- a fixed discount rate;
- a scheme that cannot be underfunded, or a deficit that nobody repairs;
- a contribution computed on an aggregate payroll;
- benefits paid to the dead.

**Done when**
- [ ] Pensions run in their three kinds; retirement changes a household's income; an ageing population raises
  state-pension spending through named payments.
- [ ] LC-4-06 and LC-4-20 to LC-4-22 pass.
- [ ] PC-64 is registered.
- [ ] Two reviews are done.

---

### S4.05 — `sys-sec`: securitisation

**Status**: planned

**Clauses**:
- STATE: SEC.1, SEC.7, SEC.8, SEC.9.
- DECISION: SEC.10, SEC.11.
- PROCESS: SEC.2, SEC.3, SEC.4, SEC.12; BNK.10 *(completes it: vehicles buy whole lines through S2.01's loan-sale
  transfer; funds buy from S3.07)*.
- INVARIANT: SEC.5.
- MEASURE: SEC.13.
- FORBID: SEC.6.
- PRIMITIVE: SEC.14.

**Architecture**: §4.4 (line transfers of cell lines, composite instructions), §6.1 (5c, 6a, 7, 9a, 9c, 9d, 2e), §8,
§9.1.

**Depends on**: S4.04.

**Goal**: banks sell pools of named loans into vehicles funded by tranches, and lend again:
- a bank securitises when the capital and funding freed are worth more to it than the loans' income, at the price the
  tranches' book forms;
- the arranger structures the tranches; investors buy them from their own views of the pool;
- servicers collect and pass cash through the deal's waterfall and tests;
- losses are borrowers' own defaults, counted member by member and allocated bottom-up; prepayments reach holders;
- tranches trade and are pledged, and correlated defaults reach senior holders.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/securitisation.rs` | the vehicle kind; `DealTerms`; `WaterfallDecl`; pools |
| `if-securities/src/sec_decisions.rs` | decision points `securitise`, `structure`, `cleanup` |
| `crates/systems/sys-sec/src/rules/securitise.rs` | SEC.10: the bank kind's rule |
| `crates/systems/sys-bnk/src/rules/workout.rs` | S2.01's workout, registered for the vehicle kind too |
| `src/rules/structure.rs` | the arranger's tranche sizes and tests |
| `src/rules/cleanup.rs` | the servicer's clean-up call |
| `src/waterfall.rs` | `run_waterfall`: pure, over collections, balances, tests and declared steps |
| `src/pool.rs` | selection, transfer, collections, write-offs, reports |
| `src/handlers/*.rs` | 5c decisions; 7 transfers and payments; 9c determinations; 9d reports; 2e losses |
| `src/audit.rs` | `SEC.losses_sum`, `SEC.cash_in_out`, `SEC.pool_equals_holdings` |
| `src/gen.rs` | opening vehicles, pools and tranche holders |
| `crates/systems/sys-crd/src/rules/invest.rs` | CRD.4's investor value reads tranche projections (SEC.11) |
| `crates/systems/sys-rat/src/method.rs` | agencies' structured methods: the waterfall under declared stresses |
| `crates/systems/sys-bcp/src/weights.rs` | tranches' risk weights (SEC.14) |
| `data/<country>/SEC.toml` | pool kinds; retention; tranche capital treatment; conventions; hours |
| `data/<country>/gen/SEC.toml` | outstanding deals by pool kind, with sources |
| `data/shared/SHAPES.toml` | the forms of `securitise`, `structure`, `cleanup` |

**Design**

- **Pool kinds are data** (SEC.7, Law 10): residential mortgages (S2.05's mortgage lines), consumer loans, small-
  business loans (lines to firm cells) and corporate term loans (collateralised loan obligations, managed). A
  `PoolKindDecl` names the eligible loan line kinds, whether the pool is static or managed, its reinvestment period,
  its eligibility terms (concentrations, ratings) as `DeclaredLimit`s from the deal's terms, and the default
  waterfall template.
- **The vehicle** (SEC.1) is a kind-table individual with its deal terms: originator, servicer (the originator unless
  the terms name another), arranger, trustee account, collection and reserve accounts, fees. It holds whole loan
  lines' lender sides, so each pool line is a line to cells standing for its count of identical loans (SEC.7).
- **Tranches** (SEC.1, SEC.8) are debt instruments of the vehicle (CRD's families), each with its seniority rank,
  attachment and detachment points and a floating or fixed coupon; their distributions are instrument events (REG.11)
  whose amounts per unit are the waterfall's outputs.
- **The waterfall** (SEC.8): `WaterfallDecl` is an ordered list of steps `{ source, pay_to, condition }` — interest,
  principal or reserve collections to fees, tranche interest or principal by rank, the reserve account or the
  residual, conditional on the overcollateralisation and interest-coverage tests. `run_waterfall(collections,
  balances, tests, steps) -> Allocations` is pure: a failing test diverts junior interest to senior principal;
  pass-through pools pay principal as it arrives, prepayments included.
- **The decision to securitise** (`securitise`, SEC.10; in `if-securities`, since tranches are its types; `sys-sec`
  registers the bank kind's rule, which values the pool's loans through `sys-bnk`'s `loan_claim_value`), a lumpy
  decision of the bank on its review
  days, when a surprise in its capital or funding wakes it: sell a pool when the value to it of the capital freed (the
  pool's risk weight less the retained tranches', times its required return on capital, S2.07) and the funding freed
  (at its marginal cost of funds, S2.06), plus the expected proceeds, exceeds the loans' income forgone and the deal's
  costs (arranger's fees, agencies' fees, its hours): loan sales priced against the cost of bank capital (Pennacchi,
  1988). The expected proceeds are its own outlook of the tranches' prices: the last prints of similar tranches, else
  the closest observed rated debt (VAL.10). The pool is its eligible lines, taken in the order of their capital charge
  per unit of income, ties by lot (`SEC.pool_selection_lot`).
- **Structuring** (`structure`, SEC.14's terms chosen by the arranger): each tranche's subordination is the thinnest
  for which it earns its target rating under the agencies' published structured method (the waterfall run under the
  agency's declared stress scenarios, POLICY of the agency, S3.10), the targets being the rating boundaries that
  investors' mandates read (RAT.4); tests' triggers and the reserve target are the arranger's terms for the pool kind
  (practice as described by Coval, Jurek and Stafford, 2009).
- **Issue and sale** (SEC.2, SEC.9): the arranger runs a book-built call per tranche (CRD.5, S3.04). The bank's sell
  orders carry, per tranche, its reservation from the value comparison above; a tranche that does not clear leaves
  the deal unfunded. Settlement is one composite instruction at 7 drawing on each tranche's allotment commitments:
  the pool lines' lender sides pass to the vehicle by line transfer, investors pay, the bank receives the proceeds,
  and the bank keeps what the retention rule and its own choice leave it — a vertical slice or the first-loss piece,
  bound from below by the rule (POLICY, `DeclaredLimit`). All or none.
- **Investors** (SEC.11): CRD.4's investor decision values a tranche by `claim_value` (S1.01) over its cash flows
  projected by `run_waterfall` under its own outlooks of the pool's default, prepayment and recovery rates — adaptive
  outlooks of the deal's reported performance and of the pool kind's published statistics — at its cost of funds plus
  its risk aversion times the width of those outlooks, within its mandate and the ratings that bind it. No correlation
  is a parameter: a wider outlook of the default rate lowers the senior tranches' value.
- **Servicing and collections** (SEC.1, SEC.12): borrowers keep paying on their lines (S0.17's stream, pooled flows),
  now credited to the vehicle's collection account. The vehicle kind's workout rule is S2.01's, registered by
  `sys-bnk` (BNK.7 is BNK's), run by the servicer with its own `LoanAssessment` through `sys-bnk`'s `loan_claim_value`
  (architecture §3.4), on the servicing agreement's terms and bears none of their loss. Prepayments are the borrowers'
  own refinancing decisions (S1.09, S2.05) and arrive as principal.
- **Losses** (SEC.3): a write-off on a pool line — a borrower's own default after its own cash failure, drawn member
  by member in the cell's line (REP.23) and enforced by the servicer (S2.01, S2.05) — is a pool loss record at 2e. At
  the next determination they are allocated to tranches bottom-up, writing down principal in reverse seniority, each
  write-down a loss event on the tranche's named holders.
- **Determination and payment**: at 9c of each determination date (applied at 9e) the waterfall runs over the period's
  collections and losses; the payment instruction settles at 7 of the payment date (the deal's declared lag); the
  investor report (pool balance, arrears, losses, prepayments, test results) is a public record at 9d.
- **Trading and pledging** (SEC.4): tranches trade in the dealer market (S3.06) and are repo collateral (S3.01) at
  each lender's haircut; agencies re-rate them from the reports (S3.10), and a downgrade across a mandate boundary is
  read by bound holders (L7).
- **Managed pools**: a collateralised loan obligation's manager (a fund manager, S3.07) buys eligible loans in the
  secondary loan market (BNK.10) at its own values, within the deal's eligibility limits, in its reinvestment period.
- **The end**: the servicer's clean-up call (`cleanup`), when the pool falls below the terms' threshold and the pool's
  value to it exceeds the tranches' outstanding balances plus costs; or the pool is exhausted and the remaining
  tranches written off. The vehicle then ends with nothing left (Law 13).
- **Review costs**: `securitise`, `structure` and `cleanup` cost the deciding bank's staff hours (TECHNOLOGY); the
  deal pays the arranger's and agencies' fees at their posted points.
- **Streams**: `SEC.pool_selection_lot`.
- **Opening** (`gen.rs`): outstanding deals by pool kind (sources): each vehicle takes the lender side of opening loan
  lines, apportioned among lenders and vehicles by drawn shares (architecture §10.2); tranche holders are apportioned
  over investors by their drawn holdings.

**Unit tests**
- `waterfall_sequential_pay`.
- `oc_ic_failure_diverts_junior_interest`.
- `losses_allocated_bottom_up`.
- `prepayment_passes_through`.
- `cash_out_equals_cash_in`: allocations sum to collections plus reserve drawn.
- `retention_bound_from_rule`.
- `thinnest_subordination_for_target_rating`: over a given stress method.
- `securitise_value_comparison`.
- `pool_loss_requires_writeoff` (compile-fail): a pool loss is built only from a `WriteOff` record.

**Live checks**
- `LC-4-07`: SEC.5 — tranche losses sum to the pool's realised losses; cash out equals cash in by the waterfall; the
  pool's loans equal the loans the vehicle holds.
- `LC-4-23`: SEC.3, SEC.6 — every pool loss traces to a write-off on a named line after its borrowers' own failed
  payments; no pool lost by a rate.
- `LC-4-24`: SEC.6, SEC.9 — every tranche outstanding has named holders, and every originator holds at least its
  retention.
- `LC-4-25`: SEC.13 — issuance by pool kind against lending conditions, tranche spreads by rating, senior losses and
  the securitised share of lending are published.

**Budget**
- Vehicles are individuals, a few thousand; pool lines move by line transfer and add no rows.
- Determinations are monthly per deal, each a waterfall over tens of steps: negligible.
- Counters, ratcheted: `phx_sec.vehicles`, `phx_sec.pool_lines`, `phx_sec.deals_launched`, `phx_sec.deals_unfunded`,
  `phx_sec.determinations`, `phx_sec.tranche_writedowns`.

**Guards**: PC-65: the allocation of pool losses takes only `WriteOff` records built by `sys-bnk`'s write-off
(compile-level), so no pool loses by a rate (SEC.6).

**Not allowed**:
- a pool losing by a rate, or a default drawn;
- a tranche without a holder, or a transfer without a transferee;
- a correlation declared as a parameter;
- a servicer bearing a loss it does not hold;
- a tranche price from a model rather than its book or its dealers.

**Done when**
- [ ] Banks sell mortgage, consumer, small-business and corporate pools and lend again; prepayments and defaults flow
  through waterfalls; senior tranches can lose.
- [ ] LC-4-07 and LC-4-23 to LC-4-25 pass.
- [ ] PC-65 is registered.
- [ ] Two reviews are done.

---

### S4.06 — `sys-mna`: takeovers and private equity

**Status**: planned

**Clauses**:
- STATE: MNA.9.
- DECISION: MNA.1, MNA.2, MNA.3; FRM.12 *(completes it: seeking a buyer)*; EQY.5 *(completes it: holders' votes on
  takeover offers, as acceptances counted on the register)*.
- PROCESS: REP.29 *(its sale clause, spec Appendix E 35: a small firm whose owners seek a buyer is promoted to an
  individual on that decision, by S0.24's promotion)*.
- PROCESS: MNA.4, MNA.5.
- INVARIANT: MNA.6.
- MEASURE: MNA.7.
- FORBID: MNA.8.
- PRIMITIVE: MNA.10.
- This step retires S2.03's deferral of FRM.12's last act of distress to S4.06.

**Architecture**: §4.2 (offers, notices, pins), §4.4 (composite instructions, line transfers), §6.1 (5c, 5d, 6d, 7,
9d, 2c), §9.1.

**Depends on**: S4.05.

**Goal**: firms bought and sold whole, each deal priced, funded and refusable:
- an acquirer bids when its own value of the target exceeds the price and it holds committed funds;
- each holder accepts or refuses from its own value, counted on the register; management may resist; rivals may bid;
- completion is one composite instruction — shares, payments, debts assumed, repaid or triggered — and the target's
  staff, suppliers and customers carry over;
- private-equity funds buy with committed capital and debt that becomes the target's own, pay themselves from more
  debt, and sell when an exit beats holding.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-securities/src/takeover.rs` | `TakeoverOffer` (record and message); `Tender` commitment |
| `if-securities/src/control.rs` | decision points `bid`, `accept`, `respond`, `buyout`, `recap`, `exit` |
| `crates/interfaces/if-credit/src/decisions.rs` | S2.03's `distress` gains the action `seek_buyer` |
| `crates/systems/sys-mna/src/rules/{bid,respond}.rs` | the firm and fund kinds' rules (MNA.1, MNA.2) |
| `src/rules/accept.rs` | every holder kind's rule (MNA.2, EQY.5) |
| `src/rules/{buyout,recap,exit}.rs` | the fund kind's rules (MNA.3) |
| `src/sale.rs` | the sale process `seek_buyer` opens: invitations and the owners' acceptance |
| `src/completion.rs` | the composite completion; squeeze-out; merger or subsidiary |
| `src/handlers/*.rs` | 5c offers, answers and expiries; 7 completions; 9d records |
| `src/audit.rs` | `MNA.sources_uses` |
| `src/gen.rs` | private-equity funds' opening portfolio firms and their buyout debt |
| `data/<country>/MNA.toml` | takeover law (POLICY); advisory fee points; hours |
| `data/shared/SHAPES.toml` | the forms of `bid`, `accept`, `respond`, `buyout`, `recap`, `exit`, `seek_buyer` |

**Design**

- **Targets are individuals**: a firm with public shares, a large unlisted firm above the promotion rank (REP.2), or
  a small firm promoted by its owners' decision to seek a buyer (REP.29, spec Appendix E 35): the members of a firm
  cell whose owners take `seek_buyer` split out as individuals at that decision's apply, each with its own lines and
  holdings, and are named targets from the next day. A promoted firm that no bid buys stays an individual until it
  falls below the demotion rank (REP.29).
- **The offer** (MNA.9): `TakeoverOffer { acquirer, target, cash_per_share, shares_per_share, threshold, conditions,
  expiry, financing: [CommitmentId] }`, a public record at 9d and a message to every holder on the register (EQY.1):
  individuals receive it; cells' holders receive notice occasions for their counts (REP.21). The law (MNA.10, POLICY):
  disclosure of stakes above its thresholds as public records; a mandatory offer once a holding crosses its control
  threshold; the offer period; committed funds before announcement; squeeze-out above its threshold.
- **The bid** (`bid`, MNA.1), a lumpy decision of a firm on its review days, woken by a sale invitation, a rival's
  offer or a surprise in the target's price:
  - its own value of the target: `firm_value` (S1.01) over its outlook of the target's earnings under the changes it
    believes it can make — its own known ways on the target's products at the target's input prices, and the
    duplicated overhead it would separate — discounted at its required return (PREFERENCE of management). A change
    counts only if it is a way, a separation or a price a real decision will later carry out (MNA.8);
  - the price: the lowest at which its outlook of holders' values (the market price and the premiums of past completed
    offers, public records, through its anchor heuristic) says the threshold is met; it bids only if its value exceeds
    that price and its financing commitments are in hand (MNA.10's committed funds);
  - financing: cash, a loan commitment (S1.09), a bond issue settled before (S3.04), or new shares (EQY.4) as
    consideration;
  - the bid costs advisers' fees paid to a named bank at its posted points and staff hours (Law 14).
- **Acceptances** (`accept`, MNA.2), on each holder's notice occasion at 5c until expiry: accept when the offer
  exceeds its own value of keeping the share (its outlook of the share's price, S1.01), with a taste on each side
  (REP.22, `MNA.accept_taste`), so a cell's reviewing members split into accepting and refusing counts. An accepting
  member tenders its shares — a `Covered<Qty>` of its free units (S3.05) — into a `Tender` commitment (REG.10), which
  pins it until completion or lapse (architecture §4.2). The acceptance is the holder's takeover vote (EQY.5). Hedge
  funds' merger-arbitrage orders come from FND.4's form reading the offer.
- **Management** (`respond`, MNA.2): the target's management publishes a recommendation at 5c — accept when the offer
  exceeds its own value of the firm, otherwise reject with that value, which holders read as a published anchor
  (VAL.6), and invite rivals by sale invitations. Rivals decide by `bid`.
- **Expiry** at 5c of the expiry date: `sys-mna`'s acceptance handler counts the tendered shares on the register; with
  the threshold met, it writes the completion; otherwise the offer lapses and the tenders are released.
- **Completion** (MNA.4, MNA.6), one composite instruction written at 5c and settled at 7, drawing on the tenders, the
  lenders' loan commitments, the share issue's commitment and the target's debts' change-of-control terms:
  - the tendered shares move to the acquirer; each holder is paid its cash and new shares;
  - each debt of the target is assumed, or repaid by its lender's redemption commitment, or accelerated by its terms;
  - fees are paid to the named advisers;
  - sources equal uses per currency (an audit family on the instruction);
  - the target becomes a subsidiary (FRM.3), keeping its lines, staff, suppliers and customers; or, as the offer's
    terms say, merges: its lines and holdings pass to the acquirer by line transfer and its shares are cancelled.
  - Above the squeeze-out threshold, the remaining holders' shares are acquired at the offer price on the law's date.
  - Headcount falls only by LAB's layoffs on their own occasions, with notice and severance (MNA.8).
- **Private equity** (MNA.3, MNA.5), decisions of a private-equity fund (FND.2):
  - `buyout`: by `bid`'s form over a holding period to the fund's remaining life, bidding through a new company it
    founds (a named party with its capital called from investors, FND.8, demands due at the next business day's 2c)
    and debt raised against the target's assets (lenders' own quotes and standards);
  - on completion the bidding company merges into the target, so the debt is the target's own; a failed buyout ends
    the firm into its estate and costs the fund only its equity (MNA.5);
  - `recap`, on its review days: the controlled firm borrows as much as its lenders offer when the quoted rate is
    below the fund's required return, and pays the proceeds as a distribution — buyout leverage following what lenders
    will lend (Axelson, Strömberg and Weisbach, 2009). It is a directive the firm's funding and payout decisions
    (FRM.9, FRM.10) read as its owner's;
  - `exit`, on its review days and on any offer: sell when a bid, a listing's book-built price (EQY.4) or another
    fund's price beats its own value of holding to the fund's end.
- **Where the decisions live**: in `if-securities`, the crate of the offer's types (architecture §3.1); `sys-mna`
  registers every decider kind's rule, since the decisions are MNA's (architecture §3.4). `seek_buyer` is FRM.12's,
  registered by `sys-frm`.
- **Seeking a buyer** (`seek_buyer`, FRM.12), the last action added to S2.03's `distress` (`if-credit`, `sys-frm`'s
  rule), taken in its order of costs when cheaper acts do not close the firm's cash gap. A small firm's members who
  take it are promoted at 5d (above); every seller then sends sale invitations at the next 5c to firms in its
  industry it can see and to private-equity and distressed funds; answers by `bid` come at the next business day's
  5c; the owners accept the best bid above their own value of carrying on (the firm's closure comparison, S1.03), by
  the completion above.
- **Review costs**: a holder's acceptance review costs hours at its value of leisure (households) or its staff's
  (institutions); `bid`, `respond`, `buyout`, `recap`, `exit` and `seek_buyer` cost staff hours, and a bid the
  advisers' fees (TECHNOLOGY and posted points).
- **Streams**: `MNA.accept_taste`, `MNA.proration_lot` (tenders scaled back in a partial offer, ties by lot).

**Unit tests**
- `offer_needs_threshold`.
- `sources_equal_uses_legs`.
- `acceptance_counts_multinomial`.
- `squeeze_out_threshold`.
- `bid_only_below_own_value_and_with_funds`.
- `recap_reads_lender_quotes`.
- `exit_beats_holding`.
- `synergy_only_from_ways_and_separations`: the value's changes are built only from way and separation plans.

**Live checks**
- `LC-4-08`: MNA.6 — sources equal uses in every deal, in named accounts.
- `LC-4-26`: MNA.4 — every completion moved exactly the shares tendered (and squeezed out), paid every holder, and
  assumed, repaid or accelerated every debt by its terms; every lapse released its tenders.
- `LC-4-27`: MNA.7, MNA.8 — premiums, deal volume against credit conditions and acquirers' bond and share prices
  around deals are published; every headcount fall after a deal is a set of separations with severance.

**Budget**
- Deals are rare: offers, tenders and completions are individual work; tenders pin cell members until expiry,
  counted.
- Counters, ratcheted: `phx_mna.offers`, `phx_mna.completions`, `phx_mna.lapses`, `phx_mna.acceptance_evaluations`,
  `phx_mna.pinned_members`, `phx_mna.buyouts`, `phx_mna.recaps`.

**Guards**: PC-66: no transfer of a firm's shares, lines or staff for control outside a settled completion or an
estate's succession (`phx-check` over the reasons allowed to request those transfers), so no deal is by assignment.

**Not allowed**:
- a deal by assignment, or an acquisition without payment;
- a synergy without a real way, separation or price behind it;
- a headcount cut without separations;
- a buyout's debt left on the fund;
- a bid without committed funds.

**Done when**
- [ ] Takeovers happen when bidders value targets above their price and can fund them, and fail when holders or
  lenders refuse; buyouts, recaps and exits run.
- [ ] LC-4-08, LC-4-26 and LC-4-27 pass.
- [ ] PC-66 is registered.
- [ ] Two reviews are done.

---

### S4.07 — Supervision of insurers and clearing houses, and the Stage 4 gate

**Status**: planned

**Clauses**:
- PROCESS: SUP.4 *(completes it: insurers and clearing houses)*; SUP.9 *(completes it: licensing insurers,
  multi-employer pension trusts and clearing houses by S2.08's founding path)*.
- MEASURE: SUP.11 *(completes it)*.
- PTY.12 *(part: the ladder and the comparison with Stage 4's reads)*; N8 *(the budget at Stage 4)*; N2; N6 *(the
  stage's declared experiments)*; the Stage 4 exit.
- This step retires the placeholders naming SUP introduced by S4.01 (a house past its waterfall) and S4.03 (an
  insolvent insurer).

**Architecture**: §3.1, §3.4, §6.1 (2b, 2c, 5c, 6a, 7, 7e, 9c, 9e), §6.5 (pending legs), §9.1, §9.2, §9.3, §13,
§14.5.

**Depends on**: S4.06.

**Goal**:
- the supervisor tests insurers and clearing houses on their reporting dates, and a breach has its declared
  consequence on the next business day;
- a failing insurer or clearing house is resolved on architecture §9.2's timetable, by the authority's least-cost
  rule, through named parties, and every position lands on a successor;
- new insurers, pension trusts and clearing houses begin when founders capitalise them and the supervisor licenses
  them;
- the exit, the budget and the comparison with the reference run are judged on the settled Stage 4 world, with the
  stage's declared experiments.

**Files**

| File | Purpose |
| --- | --- |
| `crates/interfaces/if-risk/src/resolution.rs` | the protection scheme; `bid_for_portfolio`, `bid_for_service` |
| `if-risk/src/licensing.rs` | `found_institution` per institution kind (a founder's, implemented by `sys-sup`) |
| `crates/systems/sys-sup/src/handlers/9c_test_{insurer,house}.rs` | each test with its consequence, applied at 9e |
| `src/handlers/2b_value.rs`, `6a_select.rs` | S2.08's valuation and least-cost selection, for insurers and houses |
| `src/protection.rs` | the policyholder protection scheme: levies, limits, payments, backstop |
| `src/licensing.rs` | licences for insurers, pension trusts and clearing houses |
| `src/audit.rs` | `SUP.resolution_identity` extended to insurers and houses |
| `src/rules/{bid_for_portfolio,bid_for_service}.rs` | the insurer and house kinds' bids (SUP's decisions) |
| `sys-ins` `src/value.rs`, `sys-drv` `src/value.rs` | rule handles `portfolio_value`, `service_value` |
| `data/<country>/SUP.toml` | tests, consequences, least-cost rules, protection limits and levies, licences (POLICY) |
| `data/shared/SHAPES.toml` | the bids' and the founders' forms |
| `data/shared/READS.toml`, `data/shared/EXPERIMENTS.toml` | Stage 4's reads and experiments, frozen before the run |
| `perf/{device,measure,compare,ladder}/S4.07-*.json` | the device report, measurements, comparison and ladder |

**Design**

- **Tests** (SUP.4), at 9c of each institution's reporting dates, each test and the consequence it triggers in one
  handler, applied at 9e (architecture §6.1):
  - insurers: the solvency ratio from S4.03's 9b against the requirement and the minimum (POLICY);
  - clearing houses: prefunded resources against the `cover_n` stress, liquid resources against the largest members'
    payment obligations, the margin method's back-test (POLICY);
  - a breach's declared consequence is a demand due the next business day (TIME.7): a plan, a restriction on
    distributions, a stop on new business (the insurer's `underwrite` then declines everything), more capital or fund;
    an insurer's stated insolvency or a breach of its minimum triggers resolution in the same handler.
- **Closed**: from the trigger until its transfer settles, the institution carries the `closed` fact (writer
  `sys-sup`, S2.08). Every leg it pays — an insurer's claims, annuities and claimants' benefits; a house's variation
  margin owed — is **pending** (S0.17's hook, extended to a closed payer: see the notes); legs paid to it settle as
  before. On a many-party policy line, the holders whose pending credits they are were drawn by the stream's pairing
  (REP.23). Pending legs settle with the successor or fail against the estate's claim.
- **An insurer's resolution** (INS.7, SUP.3, SUP.8), on §9.2's timetable, replacing S4.03's placeholder:
  - **D, 9c**: the trigger; bid invitations to the licensed insurers whose solvency would survive the purchase;
  - **D+1, 2b**: the book is the insurer's **statement of D** — its liabilities the actuary's valuation of D at 9a,
    full or rolled forward and labelled (MKT.20) — so nothing is valued again; equity and subordinated debt are
    written down by layer (`Row` and equity legs at 2f); the reserve is the protection scheme's cost of paying the
    protected benefits out in run-off;
  - **D+1, 5c**: `bid_for_portfolio` (the insurer kind's rule, registered by `sys-sup`, reading `sys-ins`'s handle
    `portfolio_value`): its own value of the portfolio's lines on
    its own experience basis and curve, less the assets it would take and the capital the lines consume at its
    required return; S2.08's first-price sealed bid from own valuation (Granja, Matvos and Seru, 2017), listed in
    `SHAPES.toml`;
  - **D+1, 6a**: the authority's least-cost rule (POLICY), not a market form: the highest bid at or above the reserve
    wins and pays it, ties by lot (S2.08's `SUP.bid_lot`); with none, the run-off path;
  - **D+1, 7**: one instruction. On a many-party line the failed insurer's side row passes to the acquirer by line
    transfer, and holders' rows are untouched. Where the law's protection limit is below a benefit, the holders it
    reaches are drawn from the line (REP.23) and their rows split at a kink per member (architecture §4.4): the
    protected part, `DeclaredLimit::bind` per member, continues with the acquirer, the scheme paying the difference;
    the rest becomes a claim line on the estate. On the run-off path the policies stay with the estate (S2.04), which
    pays claims as they fall due through its waterfall and the scheme pays the protected shortfall. Everything else
    goes to the estate. SUP.7's identity is a family on the instruction;
  - **a failed transfer** (a leg its payer cannot fund at 7b): nothing moves; at D+2's 2b the next bid at or above
    the reserve, or run-off, settling at D+2's 7.
  - **The protection scheme** (a party, POLICY): funded by levies on insurers by the law's rule, demanded at 9c and
    due at the next business day's 2c; a short fund draws its treasury backstop, a named payer.
- **A clearing house's resolution** (DRV.5, DRV.8, SUP.8), replacing S4.01's placeholder:
  - the `WaterfallExhausted` event of D's 9c opens resolution at the next business day's 2b (architecture §6.1's
    resolutions opened by the last business day's failures), and the house is closed from then;
  - the rulebook's recovery tools first, where declared: its variation-margin gains paid at that day's 2c are
    haircut pro rata (`DRV.prorata_lot`), and its unmatched positions torn up at their last settlement prices;
  - then **5c**: other houses decide `bid_for_service` (the clearing-house kind's rule, registered by `sys-sup`,
    reading `sys-drv`'s `service_value`: the fees it would earn against the capital, fund and liquidity the positions
    need); **6a**: the least-cost rule, its reserve the cost to members of a wind-down; **7**: members' positions and
    margin move by line transfer;
  - with no bid, wind-down at 7: every contract terminated at its last settlement price, and the net amounts claims on
    the house's estate or payments to it; the estate distributes by the law (S2.04).
- **Licensing** (SUP.9): S2.08's founding path for the kinds of this stage. A founder decides `found_institution` on
  its review when its value of the venture — `firm_value` over the premiums, fees or charges it can see and the
  published returns of that kind — beats its required return on the minimum capital (POLICY); it subscribes from
  named accounts; the supervisor licenses at 5c by the kind's declared criteria; the party is created at the next
  day's 3c. The form is S1.03's founding comparison, listed in `SHAPES.toml`.
- **Measures** (SUP.11), published at 9d and in the run's report: failures of banks, insurers and houses and their
  clustering; the cost of each resolution and who bore it; the deposit-insurance, protection and guarantee funds
  through the cycle; how often macroprudential limits bind (S2.08).
- **Review costs**: bids and founding decisions cost the decider's staff hours (TECHNOLOGY).
- **Streams**: none new (S2.08's `SUP.bid_lot`, S4.01's `DRV.prorata_lot`).
- **The gate**, as S1.16:
  - **the reads** (`READS.toml`, frozen before the first Stage 4 comparison): premiums by cover and class, claims by
    peril, funding ratios, swap rates and spreads, CDS premiums, implied volatilities, futures bases, tranche
    spreads, takeover premiums; per person, from tracers and the reference: the change of income at retirement, and
    insured and uninsured losses after catastrophes by wealth decile;
  - **experiments** (N6, §0.3), declared before the run on copies of the settled world, each changing a primitive or
    making the owner's declared intervention, none placing a decision: (1) a catastrophe of declared severity realised
    on a copy over the zone where insured dwellings concentrate; (2) the policy rate raised 200 basis points for a
    year; (3) lowered 200 basis points for a year;
  - **the reference** re-sized with Stage 4's state (about 0.3 KB more per household at weight one), five seeds; the
    play resolution twenty; the ladder over at least three rungs; the device run (a 30-minute soak, then a settled
    year); the decades run with the liveness reads;
  - **pass criteria** are S1.16's, fixed before the run; a miss is a finding, and N8.7's remedies apply in order.
- **The stage's budget ledger**, against architecture §13 as it stands through Stage 3 (a median weekday of about
  1 135 ms, a heavy Monday of about 2 300 ms, a peak of about 4 536 MB: F-003); wall time is core time ÷ 3. The gate's
  measurements replace it:

  | Memory (§13.1 line) | Addition |
  | --- | --- |
  | Household cells: 584 → 624 bytes (`insure`, `pension`, the due-day head) | 28 MB |
  | Firm cells: 464 → 480 bytes (`insure`), inside the 500-byte line | 4 MB |
  | Household profiles: policy and pension attachments, about 20 entries | 28 MB |
  | Relationship rows: policies 4.9 M, schemes 2.8 M, firms' policies 0.5 M, derivatives 1 M, others | 170 MB |
  | Line holder lists: 9.4 M × 6 B | 56 MB |
  | Holdings: DC pots as counted holding rows, about 1 M × (24 + 6) B; no participation bit, no key growth | 30 MB |
  | Lines: policy, scheme and derivative lines, about 1.1 M × 32 B | 35 MB |
  | Interned terms, about 0.35 M × 72 B | 25 MB |
  | Kind tables: insurers, schemes, houses, vehicles, bidding and promoted firms, about 10 k × 1.5 KB | 15 MB |
  | Claims, tenders and tranches across days; marks, valuations, experience | 30 MB |
  | Arena slack, 15% of the variable-length additions | 42 MB |
  | **Stage 4** | **about 465 MB** |

  | Time (§13.2 line) | Business | Non-business | Heavy |
  | --- | --- | --- | --- |
  | Settlement: 0.7 M heads at 2 ns; rows due 0.25 M (0.75 M heavy) at 10 ns; payments at 30 ns | 2 ms | — | 7 ms |
  | Occasion evaluations: `insure`, `pension`, acceptances, about 0.25 M at 80 ns | 7 ms | — | 9 ms |
  | Choices of acting members: insurer, fund and annuity choices, about 20 k at 400 ns | 3 ms | — | 4 ms |
  | Claims, third-party harm included: opened at 3e, adjusted at 5c, about 6 × 10⁴ at 300 ns | 6 ms | 2 ms | 6 ms |
  | Derivative marks and margin (a new line, S4.01) | 20 ms | — | 25 ms |
  | Valuation: insurers' and schemes' valuations and roll-forwards | 3 ms | — | 6 ms |
  | Institutions and markets: hedging, pricing, underwriting, trustees; meetings | 12 ms | 1 ms | 18 ms |
  | Contribution follow-ons at 7e, on paydays | 2 ms | — | 10 ms |
  | Parts: tenders, promotions, annuity purchases, fund switches, ≤ 5 k (≤ 20 ms at 12.1 µs) | ≤ 4 ms | — | ≤ 4 ms |
  | **Stage 4** | **about 59 ms** | **about 3 ms** | **about 89 ms** |

  | Turn | Budget | Through Stage 3 | Through Stage 4 |
  | --- | --- | --- | --- |
  | Ordinary weekday (the median turn) | 1 000 ms | 1 135 ms | **about 1 194 ms, misses by 19%** |
  | Monday after a weekend | 2 000 ms | 1 825 ms | about 1 890 ms, 5.5% headroom, short of 10% |
  | Heavy Monday | 2 000 ms | 2 300 ms | **about 2 395 ms, misses by 20%** |
  | Heavy Monday with tolerance control | 2 000 ms | 2 470 ms | **about 2 565 ms, misses by 28%** |
  | A four-day holiday block ending on a heavy day | 2 000 ms | 2 990 ms | **about 3 091 ms, misses by 55%** |
  | Peak memory | 4 500 MB | 4 536 MB | **about 5 000 MB, misses by 11%** |

  Through Stage 4 the design point misses the median by about a fifth and the memory budget by about a tenth. The
  remedies are N8.7's, in order: how the world is represented and traversed (the largest Stage 4 items are the policy
  and scheme rows with their lists and slack, about 300 MB, and derivative margin, 20 ms), then the play resolution;
  if none suffices, the owner decides. Nothing is removed from the world. F-004 is restated with these numbers.

**Unit tests**
- `portfolio_transfer_moves_insurer_side_only`: holders' rows on a many-party line are unchanged.
- `protected_split_per_member_at_limit`.
- `least_cost_selects_or_runs_off`.
- `failed_transfer_next_bid_or_run_off`.
- `teardown_at_last_settlement_price`.
- `vm_gains_haircut_pro_rata`.
- `service_transfer_moves_margin_with_positions`.
- `resolution_identity_insurer`: acquirer, scheme, estate and holders' losses sum to the hole.

**Live checks**
- `LC-4-09`: every insurer or clearing-house failure is resolved: every policy, annuity, claim, contract and margin
  balance lands on an acquirer, the protection scheme, the estate or a named loss (SUP.7, SUP.8); every resolution
  ran on §9.2's days and sub-steps, and a closed institution made no payment of its own.
- `LC-4-28`: SUP.4, SUP.9, SUP.11 — every test ran on its reporting date; every breach's consequence acted on the
  next business day; every new insurer, trust or house names its founder, capital and licence; the measures above are
  published.
- `LC-4-29`: the exit's reads hold on the gate run in every country: every derivative class formed prices with views
  on both sides each month; hazards became claims each month; funding ratios and insurers' liabilities moved with the
  curve; LC-1-42's liveness holds over thirty years with Stage 4's systems.
- `LC-4-30`: on experiment 1, the chain from the catastrophe to claims, reinsurance recoveries, premiums and any
  insurer's failure and resolution is traceable; on experiments 2 and 3, the chain from the rate to liabilities,
  deficits, sponsors' contributions, hedged schemes' margin calls and any forced sales.

**Budget**: this is Stage 4's budget gate. Counters, ratcheted: `phx_sup.tests_by_kind`, `phx_sup.breaches`,
`phx_sup.resolutions_by_kind`, `phx_sup.licences_by_kind`, and every Stage 4 counter at its measured value.

**Guards**: none new. The placeholder count falls by the two naming SUP.

**Not allowed**:
- tuning a primitive, a rule or the opening to pass;
- a gate judged off the device, or reads, experiments or criteria chosen after seeing the run;
- an experiment that places a decision for a party;
- a failed insurer or house whose positions vanish, a bail-out without a named payer, or an acquirer assigned;
- removing members, lines or a system to fit the budget.

**Done when**
- [ ] Insurers and clearing houses are tested, licensed, and resolved when they fail, through named parties.
- [ ] The device report, the comparison over the declared seeds, the ladder and the experiments are committed; every
  pass criterion holds, or the owner's decision under N8.7 is recorded in §12 and the budget then in force is met.
- [ ] Architecture §13 carries Stage 4's measured lines.
- [ ] No placeholder naming DRV, DRX, INS, PEN, SEC, MNA or SUP (for insurers and houses) remains.
- [ ] LC-4-09 and LC-4-28 to LC-4-30 pass, with every earlier live check.
- [ ] Two reviews are done.

---

## 11. Findings

| Id | Step | Day | What was measured, where | Mechanism suspected | Addressed by | Status |
| --- | --- | --- | --- | --- | --- | --- |
| F-001 | S0.23 | review, 2026-09-23 | A part end to end at 12.1 µs against 2.5 µs: the review's untuned prototype on one x86 core at 2.1 GHz (rows 1.8, profiles and positions 2.0, re-key 0.5, redraws 2.2, key and check 1.0, join and holder lists 4.0) | none: the representation's cost. At 4 µs the median day is about 1.13 s | S0.23's implementation, then S0.26 on the phone; the remedies of N8.7 in order | open |
| F-002 | S0.22, S0.23 | review, 2026-09-23 | A candidate at 174 ns (target 100), a redraw at 140 ns (target 30), a seller spread at 4.1 µs (target 0.8), same prototype. Targets raised to 180 ns, 150 ns and 3 µs, and redraws cut by the weight ladder; architecture §13.2's projected median day becomes 981 ms (2% headroom), a heavy Monday 2 025 ms (misses by 1%) | none: the representation's cost | S0.26 on the phone; N8.7 | open |
| F-003 | S3 | planning, 2026-09-23 | Stage 3, with its representation choices (participation per asset class in the key and holdings as counted rows; the linked call warm-started over a pruned network; registered instrument outlooks and values computed on new prints and shared; closed-form claim values), adds about 53 ms to an ordinary business day — the linked call 3–5 ms of wall time per country, undivided; households' `choose_holdings` about 30 k reviews and 10 k choices at about 1 µs; registered outlooks and values 6 ms — about 86 ms to a heavy one, and about 240 MB at peak (institutions' positions and their lots at 32 B, 104 MB; households' holding rows 30 MB; individuals' deviations 19 MB; keys a third more with participation 18 MB; household cells 11 MB). Through Stage 3 the median weekday projects at about 1 135 ms (13.5% over the budget), a heavy Monday at about 2 300 ms (15% over) and the peak at about 4 536 MB (0.8% over 4.5 GB); a fund-run day adds about 60 ms (150 ms at F-001's measured part cost) | none: the representation's cost | S1.16's measurements, then each gate; N8.7's remedies in order, representation and traversal first; the owner if none suffices | open |
| F-004 | S4 | planning, 2026-09-23 | Stage 4 adds about 465 MB (policy and pension rows with their lists and slack about 300 MB) and 59 ms to a business day (89 ms heavy; derivative margin about 20 ms): through Stage 4 the median weekday projects at about 1 194 ms (19% over), the heavy Monday about 2 395 ms (20% over) and the peak about 5 000 MB (11% over the 4.5 GB budget itself) | none: the representation's cost | S1.16's measurements, then each gate; N8.7's remedies in order; the owner if none suffices | open |
| F-005 | S2 | planning, 2026-09-23 | Stage 2, with its representation choices (invoices per statement period in due-day runs; one part per housing transaction; bank switches made at settlement; resolution from the day's statement; one estate per (part, occasion)), adds about 86 ms to an ordinary business day — parts 58 k × 2.5 µs ÷ 3 ≈ 48 ms at the target, ≈ 234 ms at F-001's measured 12.1 µs; housing search 17 ms; occasion evaluations 11 ms; institutions 10 ms — about 126 ms to a heavy day, and about 251 MB at peak (invoice rows with their holder lists and slack 104 MB, filed accounts 48 MB, household cells 45 MB, estates 31 MB). Through Stage 2 the design point projects a median turn of 996 + 86 = 1 082 ms and a peak of 4 045 + 251 = 4 296 MB: **the required 10% headroom (at most 900 ms and 4 050 MB) is missed on both**, and the median misses the budget itself by 8%. With Stages 3 and 4 (F-003, F-004) the full world projects near 1.2 s and 4.9 GB | none: the representation's cost | S1.16's measurements, then each gate; N8.7's remedies in order (representation and traversal, then the play resolution); the owner if none suffices | open |

---

## 12. Owner decisions

| Decision | Answer | Date |
| --- | --- | --- |
| Memory budget (N8.4) | 4.5 GB resident | 2026-09-23 |
| Map (spec Appendix E 29) | about 40,000 tiles of 10 km; 12, 8 and 5 regions | 2026-09-23 |
| Accuracy for play (N8.5, Appendix E 30) | on every declared read, within 5% on means and shares and 10% on tail quantiles beyond seed spread; the difference published | 2026-09-23 |
| Coarsening for the phone (Appendix E 31) | pooled flows; employment lines by occupation family and region with a five-year start band; reviews on a cell's review days; sellers spread on review days | 2026-09-23 |
| Budget stance (N8) | keep 1 s / 2 s and 4.5 GB; coarsen the spec rather than relax the budget | 2026-09-23 |
| Save duration (N8.10) | a full save within 5 s and an increment within 1 s on the phone, the world paused | 2026-09-23 |
| Land shares of the three countries | derived, not asked: 48/32/20, in proportion to the owner's 12/8/5 regions, so regions are of like size | 2026-09-23 |
| Settling length (GEN.6) | one simulated year by default; adjustable | 2026-09-23 |
| Save interval (SET.12) | every simulated quarter by default | 2026-09-23 |
| Pension accrual (PEN.2, spec Appendix E 32) | career-average revalued amounts; final-salary schemes carried as their equivalents | 2026-09-23 |
| Derivative holders (spec Appendix E 33) | individuals only: households and small firms carry risk through their contracts' terms | 2026-09-23 |
| Liability cover (CHN.3, spec Appendix E 34) | harm to third parties is a hazard, so liability claims come from events | 2026-09-23 |
| Selling a small firm (REP.29, spec Appendix E 35) | promoted to an individual when its owners seek a buyer | 2026-09-23 |

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
| GEO | S1.07 | 13, 18 |
| GEO | S5.02 | 4 |
| REP | S0.21 | 1, 3, 4, 17, 19, 20, 32, 33 |
| REP | S0.22 | 7, 12 |
| REP | S0.23 | 8, 9, 14, 16, 23, 36 |
| REP | S0.24 | 10, 13, 15, 28, 31 |
| REP | S0.25 | 25, 26 |
| REP | S0.26 | 2, 30 |
| REP | S1.01 | 21, 35 |
| REP | S1.06 | 37 |
| REP | S1.09 | 34 |
| REP | S1.12 | 5 |
| REP | S2.05 | 22, 24 |
| REP | S4.06 | 29 |
| REP | S6.05 | 18 |
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
| POP | S5.05 | 8 |
| POP | S6.02 | 1, 2, 6, 7, 11, 12, 13, 14, 16 |
| HH | S1.12 | 1, 2, 3, 4, 5, 6, 15, 18, 19, 20 |
| HH | S2.05 | 8, 10 |
| HH | S2.11 | 13, 14, 21 |
| HH | S4.03 | 11 |
| HH | S5.03 | 12 |
| HH | S5.05 | 9 |
| HH | S6.03 | 7, 16, 17 |
| TEC | S1.02 | 1, 2, 3, 4, 9, 12 |
| TEC | S6.01 | 5, 6, 7, 8, 10, 11, 13 |
| FRM | S1.03 | 1, 2, 4, 5, 6, 11, 13, 14, 17, 18, 20, 21, 22, 23 |
| FRM | S1.04 | 8 |
| FRM | S1.08 | 7 |
| FRM | S2.03 | 15, 19 |
| FRM | S3.05 | 3, 9, 10 |
| FRM | S3.07 | 16 |
| FRM | S4.06 | 12 |
| CAP | S1.04 | 1, 3, 4, 5, 6, 8, 9, 10, 11, 12, 13 |
| CAP | S2.05 | 2 |
| CAP | S5.02 | 7 |
| GDS | S1.05 | 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13 |
| GDS | S2.05 | 3 |
| SRV | S1.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9 |
| FRT | S1.07 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| LAB | S1.08 | 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17 |
| LAB | S6.02 | 5 |
| HSG | S2.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20 |
| TCR | S2.02 | 1, 2, 3, 4, 5, 6, 7, 8 |
| ENE | S2.09 | 1, 2, 3, 4, 7, 8, 9, 10, 11, 12, 13, 14 |
| ENE | S4.02 | 5, 6 |
| BNK | S1.09 | 1, 2, 4, 5, 6, 8, 11, 14, 16, 17, 18, 19, 20 |
| BNK | S2.01 | 7, 9, 12, 15 |
| BNK | S2.10 | 21 |
| BNK | S2.12 | 13 |
| BNK | S3.04 | 3 |
| BNK | S3.08 | 22 |
| BNK | S4.05 | 10 |
| BFL | S2.06 | 1, 4, 5, 8, 9, 11, 12, 13, 14 |
| BFL | S3.01 | 6, 10 |
| BFL | S3.04 | 2 |
| BFL | S3.06 | 3 |
| BFL | S3.07 | 7 |
| BCP | S2.07 | 1, 2, 3, 5, 6, 7, 8, 9 |
| BCP | S3.05 | 4 |
| SEC | S4.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14 |
| MMK | S3.01 | 1, 2, 4, 5, 6, 7, 8, 9, 10, 11 |
| MMK | S4.03 | 3 |
| SOV | S1.11 | 6 |
| SOV | S3.03 | 1, 2, 3, 8, 9, 10 |
| SOV | S3.06 | 5, 7 |
| SOV | S5.04 | 4 |
| CRD | S3.04 | 1, 2, 3, 4, 6, 8, 9, 10, 11 |
| CRD | S3.05 | 7 |
| CRD | S3.06 | 5 |
| EQY | S3.05 | 1, 2, 4, 7, 8, 10, 11, 12 |
| EQY | S3.06 | 6, 9 |
| EQY | S3.09 | 3 |
| EQY | S4.06 | 5 |
| MNA | S4.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| FND | S3.07 | 1, 2, 3, 5, 6, 7, 8, 9, 10, 11, 12, 13 |
| FND | S4.02 | 4 |
| DLR | S3.06 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| IDX | S1.14 | 3 |
| IDX | S3.09 | 1, 2, 4, 5, 6, 7 |
| RAT | S3.10 | 1, 2, 3, 5, 6, 7, 8, 9 |
| RAT | S4.03 | 4 |
| DRV | S4.01 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| DRX | S4.02 | 1, 2, 3, 4, 5, 6, 7, 8 |
| INS | S4.03 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| PEN | S4.04 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11 |
| TRS | S1.11 | 1, 4, 6, 9 |
| TRS | S3.03 | 2, 3, 7, 8, 10 |
| TRS | S5.04 | 5 |
| TAX | S1.11 | 5, 7 |
| TAX | S5.01 | 1, 2, 3, 4, 6, 8 |
| SOC | S1.11 | 7 |
| SOC | S5.02 | 1, 2, 3, 4, 5, 6, 8, 9 |
| CB | S3.02 | 2, 3, 4, 6, 7, 8, 9, 12, 13, 14, 16 |
| CB | S3.03 | 10 |
| CB | S3.06 | 5 |
| CB | S5.04 | 1, 11 |
| CB | S5.05 | 15 |
| SUP | S2.08 | 1, 2, 3, 5, 6, 7, 8, 10, 12 |
| SUP | S4.07 | 4, 9, 11 |
| POL | S5.03 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| FX | S5.04 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 |
| XB | S5.05 | 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 |
| OBS | S0.10 | 1 |
| OBS | S0.26 | 9 |
| OBS | S6.04 | 2, 3, 4, 5, 6, 7, 8 |
| STA | S1.14 | 2, 3, 4, 5 |
| STA | S5.05 | 1 |
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
