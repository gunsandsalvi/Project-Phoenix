# Project Phoenix — Implementation Plan

This document is the plan for building the whole world. It is written for someone who has never
seen this repository or the conversations that produced it. Read it top to bottom once; then read
the section "How to use this plan" before every item.

The plan has two parts:

- **This file**: what we are building, the rules, the architecture, the contracts, the method, and
  the progress figure. It changes rarely.
- **`docs/plan/<item>.md`**: one file per worklist item with the detailed implementation of that
  item. **An item's file is deleted when the item closes** (its outcome moves to `docs/RECORD.md`),
  so the plan directory always contains only what is still to be built. The progress figure below
  is recounted from those files by `npm run plan:progress`.

<!-- progress:start -->
**Plan completion: 54.0%** (276 of 511 steps across 34 items).
**Requirement coverage: 38.6%** (525 MET, 74 PARTIAL, 0 OUT OF SCOPE of 1361 REASON/VERIFY/FORBID clauses).

| item | steps | done | state |
|---|---|---|---|
| 0 — Foundation | 8 | 8 | closed |
| 1 — Money and settlement, one calendar | 7 | 7 | closed |
| 2 — Register, clearing, cells, DvP, value, parameters | 12 | 12 | closed |
| 2a — Kernel/module boundary | 9 | 9 | closed |
| 3 — The sovereign's funding constraint | 26 | 26 | closed |
| 4 — Firms, goods, labour, households, outlooks | 34 | 34 | closed |
| 4a — A line is more than one firm, and firms differ in cost | 9 | 9 | closed |
| 5 — A loss is an event | 13 | 13 | closed |
| 6 — Loans are rows | 16 | 16 | closed |
| 7 — Forced seller, nothing immortal, the estate | 15 | 15 | closed |
| 8 — Redeemable claims | 12 | 12 | closed |
| 9 — Equity and dealers | 24 | 24 | closed |
| 10.1 — The kernel asks a kind what a LOT is carried at | — | — | closed (no item file) |
| 10 — The cost of capital | 16 | 16 | closed |
| 10.2 — Every unit has a smallest piece | — | — | closed (no item file) |
| 10.3 — A quantity is a whole number of indivisible pieces | 15 | 15 | closed |
| 10.4 — What a lot is carried at, after the marks are taken | — | — | closed (no item file) |
| 11 — Money market, corridor, bank capital | 32 | 32 | closed |
| pre12 — The guards that keep the documents true | 16 | 16 | closed |
| [12 — An anchored market: the second opinion, the balance sheets under it, and the currency layer](plan/12-currency-benchmarks-ratings.md) | 32 | 12 | in progress |
| [12a — Reporting and estimates](plan/12a-reporting-and-estimates.md) | 25 | 0 | open |
| [13a — The derivative layer](plan/13a-derivative-layer.md) | 16 | 0 | open |
| [13b — The derivative classes](plan/13b-derivative-classes.md) | 20 | 0 | open |
| [13c — Commodities and freight](plan/13c-commodities-freight.md) | 14 | 0 | open |
| [13d — Labour mobility, housing, household life cycle](plan/13d-labour-housing-lifecycle.md) | 18 | 0 | open |
| [13e — Trade credit, small business, securitisation](plan/13e-trade-credit-pools-securitisation.md) | 18 | 0 | open |
| [13f — Corporate credit, short-term debt, lending and financing](plan/13f-corporate-credit-financing.md) | 22 | 0 | open |
| [13g — Corporate control and firm birth](plan/13g-mna-birth.md) | 13 | 0 | open |
| [13h — Insurers, hedge funds, private equity](plan/13h-insurers-hedge-pe.md) | 17 | 0 | open |
| [13i — Cross-border](plan/13i-cross-border.md) | 10 | 0 | open |
| [14 — The polity](plan/14-polity.md) | 12 | 0 | open |
| [15 — The recipe](plan/15-recipe.md) | 6 | 0 | open |
| [16 — Measure](plan/16-measure.md) | 12 | 0 | open |
| [17 — The app and the APK](plan/17-app-apk.md) | 12 | 0 | open |
<!-- progress:end -->

---

## 0. What Phoenix is

Phoenix is a simulated economy in which nothing is invented: every unit of money and every share
has a named counterparty at every instant; every asset that has a price has a **cleared** one, from
real supply meeting real demand; nothing is capped, floored, plugged or assigned; and the instrument
that checks all this (the audit) is itself true. The specification, `docs/spec/PROJECT_PHOENIX.md`,
describes the world in 48 systems (money, the register, clearing, the audit, the seed, currency;
sixteen markets; seven kinds of financial institution; the treasury, the central bank and the
polity; firms; the real economy; and cross-cutting systems), two instrument contracts (what any
bond and any derivative must have), and seventeen mechanisms that run across systems (a loss is an
event; the forced seller; nothing is immortal; the cost of capital; and so on).

The specification contains **no implementation**. Every requirement in it is one of three forms:

- **REASON**: a cause a participant has (a bank has a cost of funds; a firm has an outlook).
- **VERIFY**: something to **measure**, never to enforce (worse credit trades wider).
- **FORBID**: something that must be **absent** (no buyer of last resort; no overdraft for the
  treasury).

A mechanism produces outcomes from reasons. Writing an outcome down (a price, a share, a default
rate) is the defect this whole project is organised against.

The deployment target is a phone (a Pixel-class Android device, as an APK) with continuous testing
on GitHub Pages. The same TypeScript engine runs in both.

### 0.1 The rules, in one paragraph

`CLAUDE.md` at the repository root is the digest of the specification's nineteen laws and its
prohibitions, and it is loaded into every working session. The ones you will hit hourly: every
number is declared with its kind (technology, preference, policy, resolution, shape, placeholder);
no bound of any kind; tolerances are arithmetic dust, never a percentage; every fact has one writer;
every flow has two sides in the same period and currency; prices come from clearing and everything
else is derived from them; one ordered worklist, one item at a time, a record entry per item; do not
measure the model until it is complete. Read `CLAUDE.md` in full before touching code.

### 0.2 The repository

```
CLAUDE.md                    the rules digest (in context every session)
docs/spec/PROJECT_PHOENIX.md the specification (the authority)
docs/ARCHITECTURE.md         every implementation decision, with the clause it derives from
docs/PLAN.md                 this file
docs/plan/<item>.md          the open items, in detail (deleted when closed)
docs/plan/manifest.json      every item with its step count (for the progress figure)
docs/WORKLIST.md             the one ordered list of items and their state
docs/RECORD.md               outcomes, one entry per closed item
docs/COVERAGE.md             one row per spec clause: MET / PARTIAL / MISSING / OUT OF SCOPE
docs/BUGS.md                 findings parked mid-item, positioned when the item closes
packages/engine/src/         the engine (kernel + modules), pure TypeScript, no DOM
packages/engine/test/        its tests (Vitest, fast-check)
packages/app/                the inspector web app (Vite) and the Capacitor Android wrapper
tools/                       spec index, citation check, coverage recount, plan progress, lint rules
.github/workflows/           CI on every push; Pages on main; Android APK on demand
```

Commands: `npm install`; `npm run check` (lint, typecheck, tests, spec citations, plan progress);
`npm run dev` (the inspector at http://localhost:5173/Project-Phoenix/); `npm run build`;
`npm run e2e` (browser smoke test); `npm run coverage:spec` (recount requirement coverage).

### 0.3 Vocabulary

| Word             | Meaning here                                                                                                                  |
| ---------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **Party**        | anything with a balance sheet: a bank, a firm, the treasury, a household cell                                                 |
| **Cell**         | a party standing for a population of identical members, with an integer **weight**; all its state is **per member**           |
| **Instrument**   | anything that can be held: money at a bank, a bond, a share, a loan, a good, a dwelling                                       |
| **Holding**      | (holder, instrument) with **lots** (units and the basis they cost) and **liens** (encumbered units)                           |
| **Instruction**  | a numbered, two-sided movement of assets and money; the only way state changes                                                |
| **Settlement**   | applies an instruction, all legs or none, and generates the interbank reserve leg                                             |
| **Print**        | one cleared price per instrument per period, with its provenance (traded, stale since when, opening)                          |
| **Period**       | a week of 7 days; 5 settlement **cycles** inside it                                                                           |
| **Phase**        | a unit of work that runs at a stated cycle each period                                                                        |
| **Participant**  | a reason for a party of a kind to post orders in a market                                                                     |
| **Audit family** | one of nine identities checked every period (money, ownership, prices, cross-market, accounts, names, flows, zero-sum, units) |
| **Kernel**       | the stores and the period loop; owns every writer                                                                             |
| **Module**       | one spec system, instrument family or seed, plugged into the kernel through a declared contract                               |
| **Placeholder**  | a declared number standing in for a mechanism not yet built, with the item that deletes it                                    |

---

## 1. Objective and definition of done

**The world is done when all of these hold:**

1. Every REASON, VERIFY and FORBID row in `docs/COVERAGE.md` is MET, or OUT OF SCOPE with a stated
   reason. No MISSING and no PARTIAL rows remain.
2. Every item in `docs/plan/manifest.json` is closed (its file deleted, its record written).
3. The parameter register reports **zero placeholders**; every remaining shape has a `why` that
   justifies it and its count has only fallen across the record.
4. All nine audit families are built and hold to dust over the long run with shocks (Part XII's run
   ladder), from the seed, reproducibly.
5. Part XII's measurement programme has run once in full and its findings are recorded as findings.
6. The app builds as an APK, installs on the target device and runs a working-length run.

Until then, audit families that fail are expected to fail, and the record says which and why.

---

## 2. Principles that shape every item

1. **A mechanism produces outcomes; it never assigns them.** Before writing a value into a price, a
   quantity, a share, a default, ask what mechanism should have produced it. Write that mechanism.
2. **Every number is declared** (Law 2, XI-14). Before writing a literal, decide which of the five
   kinds it is and declare it in the module's `params` with unit, owner and reason. A placeholder
   names the mechanism and the item that delete it. The lint refuses undeclared literals.
3. **State moves only by settlement** (Money D4). A module drafts instructions; it never writes a
   store. If a needed write is not in the context, that is a kernel item to insert, never a cast.
4. **A party decides from its own view** (Observer A4, Expectations D1). Participants and deciders
   are written against `ParticipantView`. If a decision needs more, either it is public state the
   kernel should publish or the decision is reading what it must not.
5. **Failure is a state, not an exception.** A market can fail to clear, a payment can fail, a party
   can cease. Each is a recorded outcome the next mechanism reads. Exceptions are for contract
   violations only (a currency mismatch, a one-sided leg, NaN).
6. **A module never talks to a module.** It reads kernel state and writes through kernel doors. Two
   systems that need each other communicate through instruments, prints, register rows and journal
   events. The lint refuses an import between modules.
7. **Behaviour by kind lives in a profile** (Law 15). A switch on a kind id inside a mechanism is a
   defect; the lint refuses it. Register the behaviour with the kind.
8. **Insert, never append** (Law 10). A missing prerequisite becomes an item inserted before the
   item in hand; the record says where it landed.
9. **Do not measure mid-build** (Law 11). Deterministic checks (lint, types, tests, the seed audit,
   a year-long green run) are gates. Numbers are not.
10. **Cite as you go.** Every module, profile, phase and family carries `@spec` tags; every clause
    met is re-marked in `docs/COVERAGE.md` in the same commit; `npm run check` fails on a citation
    that does not resolve.

---

## 3. The architecture

```
                     +---------------------------------------------------------------+
                     |                            KERNEL                             |
                     |  calendar · registry + params · parties · instruments         |
  modules declare -> |  register (one writer: settlement) · ledger · settlement      |
  kinds, units,      |  price store (one writer: markets) · valuation · solver       |
  params, phases,    |  journal · audit runner · corporate actions · revaluation     |
  participants,      |  cells (one writer of weights) · world (the period loop)      |
  families, seed     +------------------+-----------------+--------------------------+
                                        |                 |
              SeedContext (period 0)    |   MechanismContext (phases)   ParticipantView (per party)
                                        v                 v                      v
   +---------------------+   +---------------------+   +---------------------+   ...
   | seeds/foundation    |   | mechanisms/sovereign|   | mechanisms/firms    |
   +---------------------+   +---------------------+   +---------------------+
```

**The kernel** (`packages/engine/src/{core,calendar,registry,parties,register,ledger,prices,
clearing,journal,audit,world}`) owns the stores and the period loop and the doors modules come
through. It owns the money instrument kind and the party kinds money needs. It changes only when a
Part III clause of the specification demands it, by an inserted item with a record entry.

**A module** (`packages/engine/src/mechanisms/<id>/index.ts`, or `src/seeds/<id>.ts`) implements
one system of the specification, one instrument family, or one seed, as a `SystemModule` value
(`world/module.ts`). Assembly (`world/assemble.ts`) merges modules in dependency order, seeds, states
every equity account as the read, and seals the world with the audit at period zero.

**Data flow in a period.** Phases run in cycle order. A phase reads public state and the views it
needs, drafts instructions, and settles them. Markets collect orders from participants (each
evaluated with its own party's view), clear, settle trades, and print. Revaluation books mark
changes. The audit closes the period. Everything a later phase needs from an earlier one is in the
kernel's stores: a print, a holding, an issued amount, a journal event.

**What crosses module boundaries.** Only kernel objects: instruments (by id and public terms),
prints, register rows read through a view, journal events, parameters. Never a module's own objects.
If two modules seem to need a shared type, it is a kernel type or it is an instrument.

**Module-private state.** A module may keep state the kernel does not model (an outlook, a workout
in progress, a rating). Rules: one writer (the module); reproducible from the seed; declared to the
kernel through a state slot (`ctx.state<T>(name)`, a kernel door inserted at item 4) so it is
snapshotted and reset with the world; every change an observer would notice is journaled; never a
second copy of what the register or the price store already holds.

### 3.1 The contexts (the only doors)

| Context            | Who gets it                                                 | Can                                                                                                                                                                                          | Cannot                                                             |
| ------------------ | ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| `ParticipantView`  | a party, when a participant declaration is evaluated for it | read its own holdings, cash, equity; public prints (already produced); public instrument terms; public events; its own random stream                                                         | see another party's private state; write anything                  |
| `MechanismContext` | a module phase                                              | read public state and any party's own view; settle instructions; register instruments; open markets; apply cell events (split, merge, weight); cease a party; journal; its own random stream | write the register, a print or a weight; reach the world container |
| `SeedContext`      | a seed module, at period zero                               | add parties and instruments; endow money and units; write opening prints; open markets                                                                                                       | anything after the seal                                            |

`World.register` is a runtime-frozen read facade. The store with writes reaches settlement, the seed
and the cell events only. Tests assert no write is reachable through a context.

### 3.2 Kinds and profiles

Kinds (instrument kinds, party kinds) are registered at assembly by the module that owns them, each
with its whole behaviour in one profile (`registry/kinds.ts`):

- `InstrumentKindProfile`: `pricing` (money | cleared | carriedAtCost | derived — where a price
  comes from) and `carry` (mark | cost — what a holder carries it at, a different question:
  ARCHITECTURE §4.5); `liabilityOfIssuer`; `unit(ccy)`; `validateTerms`; `displayName`;
  `due(instrument, period, calendar)`; `accrued`; `cashFlows`; `ranking`. Optional, each a door a
  kind opts into: `carriedAt` (what one LOT is carried at now, item 10.1), `derive` (what a unit of
  a `derived` kind is worth), `fairValueThroughIncome`, `physical`, `defaultOn`, `accelerates`,
  `splits`.
- `PartyKindProfile`: `representation` (named | cell), `moneyIssuer` (null, or the overdraft
  decision), `fails`, `terminal`, `borrows`, `depositClass`. Where a party of the kind BANKS is not
  a field: it is whether the module that owns the kind declared a `bankChoices` reason for it, which
  is the same fact and its answer in one place (Law 4).

The profile is the contract, so it is the thing to read before writing a kind — `registry/kinds.ts`
is the authority and this list is its summary.

The kernel never looks inside an instrument's terms. A module that needs a term (a coupon, a
maturity) exports a typed accessor with a type guard from the module that owns the kind. An
instrument whose kind has no profile cannot be registered.

---

## 4. The module contract

A module is `packages/engine/src/mechanisms/<id>/index.ts` exporting one `SystemModule`:

| Field             | Rule                                                                                                                       |
| ----------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `id`              | stable, lower-case, dotted or hyphenated; equals the directory name                                                        |
| `spec`            | the spec system(s) it implements, e.g. `'Sovereign'`, `'Treasury Sovereign XI-9'`                                          |
| `requires`        | module ids it depends on; assembly orders by these; a cycle or a missing id is a construction error                        |
| `instrumentKinds` | profiles for kinds this module owns; a kind is owned by exactly one module                                                 |
| `partyKinds`      | profiles for party kinds it owns (a fund, an insurer, a clearing house)                                                    |
| `curveFamilies`   | curve families it declares: one owner, one compounding, one day count (Sovereign D3.a)                                     |
| `units`           | physical units it introduces (tonnes, dwellings, hours)                                                                    |
| `params`          | every number it reads: id, value, unit, kind, owner, why; for a placeholder, `standsInFor`                                 |
| `phases`          | what it runs: name, spec, cycle, anchor (`{ before }` or `{ after }` a kernel phase or another module's phase), `run(ctx)` |
| `participants`    | reasons to be in a market, per party kind: `orders(view, market)`                                                          |
| `venueParticipants` | the same for a venue, which asks for schedules rather than clearing a book (item 11.1)                                    |
| `families`        | audit contributions: `{ name, contributor, spec, built, check(view) }`                                                     |
| `outlooks`        | how a party of its kinds forms its own outlook, if it does (§46, XI-16)                                                    |
| `marks`           | what a lot of a kind with no market is worth; exactly one module answers per kind (XI-6)                                   |
| `creditDecisions` | what a customer of a party kind overdrawn at its issuer is told; one module per kind (Money B3.a)                          |
| `bankChoices`     | where a depositor of a party kind banks and why it would move; one module per kind (Banks Funding E1)                     |
| `resolves`        | party kinds whose failure this module takes charge of, so the estate opens none for them (XI-3)                            |
| `seed`            | opening state this module contributes; runs in assembly order before the seal                                              |

**Lifecycle in a run.** Assembly registers kinds, units and params; inserts phases; registers
participants and families; calls `seed`; the kernel states equity and seals. Then every period: each
phase runs at its cycle; each participant is evaluated per party at each market; each family checks
at the close.

**Anchors and cycles.** The kernel's phases are `corporateActions` (cycle 0), `markets` (cycle 1)
and `revaluation` (last cycle, 4). A module phase anchors `{ before }` or `{ after }` one of these or
another module's phase, and names a cycle. Cycles must be non-decreasing along the phase list;
assembly refuses otherwise. Section 8 is the canonical map.

### 4.1 How to add a system

1. Read the spec section end to end, and every Part XI mechanism it cites.
2. In `docs/COVERAGE.md`, list which of its clauses the item will meet; anything it will not gets a
   PARTIAL row with what is missing and which item brings it.
3. Write the kinds, units and params. Profiles first; they are the contract.
4. Decide phases and anchors from Section 8; decide participants and their party kinds.
5. Write the seed contribution with every endowment declared.
6. Write the audit contribution: the identities the system makes checkable.
7. Tests in this order: profiles; one phase on a small assembled world; a year-long green run; property
   tests for any solver or allocation; a determinism test if randomness is drawn.
8. Cite, re-mark COVERAGE, write the record, delete the item file, update the manifest, commit.

### 4.2 How to replace a system

Replace the module directory. Keep the module id if the replacement owns the same kinds; otherwise
give it a new id and re-kind existing instruments through a journaled instrument event. Nothing else
changes: no other module imports it, and the kernel learns the new behaviour through the profiles.
The record says what replaced what and why. The kernel's tests do not change.

### 4.3 How to remove a system

Remove the module. Assembly refuses while another module `requires` it. Instruments of its kinds
must have ceased or been re-kinded first. COVERAGE rows for its clauses return to MISSING or, with a
reason, OUT OF SCOPE (never deleted: Appendix C).

### 4.4 When the kernel may change

Only for a Part III clause (Money, Register, Clearing, Audit, Seed, Currency) or for a door a module
needs that no context offers (a module-state slot, a new corporate-action word, a new leg kind, a
primary-market form of clearing). Each such change is its own sub-item with a record entry, and it
never branches on a module's kind. The list of single writers is fixed: settlement writes holdings
and issued amounts; markets write prints; the cell events write weights; the seed writes endowments
before the seal; nothing else.

---

## 5. How to use this plan (the build loop)

Do not skip steps; do not reorder them.

> **BUILD FORWARD. THE FILE TAKES THE FINDINGS.**
>
> The order the owner set, and it overrides every instinct to stop and fix:
>
> 1. **Implement.** The work is the next item in the ordered list. That is what a session does.
> 2. **Every bug goes in `docs/BUGS.md`.** Every one, without exception — a red test, a number that
>    looks wrong, an audit family that fires, a mechanism that never runs, a world that stops. It is
>    written down where it was seen and what was measured, and the item carries on. Nothing is
>    chased, and nothing is quietly left out either: a finding not written down is a finding lost.
> 3. **Tests run at the END of a full module, not during one.** A suite run mid-item measures a world
>    that is half-built, and what it reports is the half that is missing (Law 11). Run them when the
>    module is complete, read what they say, and put what they say in the bug file.
>
> The reason is Law 11 and Law 10 together. A misbehaving number is not a work item — the missing
> mechanism is — and this world is nowhere near complete, so most of what looks wrong is a mechanism
> that has not been built yet. Stopping to fix it costs the ordered list its order, and fixes the
> symptom of an absence.
>
> What this does NOT license: a contract violation the engine throws on is not "a finding to write
> down and walk past" when it stops the build — an impossible quantity, a one-sided flow, a missing
> writer. Those are the mechanism refusing to be built wrong. Fix what is impossible; write down what
> is merely improbable, because improbable is usually a model that is not finished.

1. **Open.** Take the first open item in `docs/WORKLIST.md`. Open `docs/plan/<item>.md`. Read the
   spec sections it lists in full, and the ARCHITECTURE sections it touches.
2. **Confirm the split.** The item file has sub-items and a step checklist. If the world that has
   arrived makes a step wrong, change the file (and the manifest's step count) and say why in the
   record when the item closes. Do not silently skip a step.
3. **Coverage first.** Mark the rows the sub-item will meet; PARTIAL what it will not.
4. **Profiles and params.** Declare before use. Every literal is a decision.
5. **Mechanism.** Phases, participants, seed. Draft instructions; never write stores. Decide from
   views. Failure as outcomes.
6. **Audit.** Add the identities. A family reports built only when it checks something real.
7. **Tests.** As the item file lists them — WRITTEN as the item goes, RUN when the module is
   complete. A failing family expected by the item is asserted by name. What a run reports goes in
   `docs/BUGS.md`, all of it, before anything is changed in response to it.
8. **Gates.** `npm run check` at the end of the module. `npm run coverage:spec` recounted. The
   browser smoke test green if the surface changed.
9. **Tick the steps** in the item file as you go (`- [x]`); `npm run plan:progress` recounts.
10. **Park what you find. All of it.** Every bug goes in `docs/BUGS.md` — what was measured, where
    it was seen, what is ruled out — and the item carries on from the step it was on. Chasing one is
    how an item stops being one bounded change (Law 14) and how the ordered list stops being ordered
    (Law 10). The one exception is a violation that stops the build: an impossible quantity, a
    one-sided flow, a fact with two writers. Those are fixed where they are, because the engine
    will not run past them — and they are written down too.
11. **Close.** Write the record entry (what, why, found, deleted, forecast with its killer); delete
    the item file; leave its manifest row (a missing file counts as done); set the worklist row to
    done; **position every finding in `docs/BUGS.md`** — into the item that should fix it, or as an
    inserted item of its own, with the record saying where each landed; commit with a message that
    says what and why.

**Definition of done for an item:** steps 3–10 complete; no `TODO` in code (a TODO is a worklist
item or it is nothing); no PARTIAL row without a named item; no placeholder without a named death;
the year-long run green or its expected red families named in the record.

---

## 6. Strategies that recur

### 6.1 Cells and weights (XI-15)

Every household and small-firm quantity is per member; the cell's total is a read. A leg on a cell
is per member at the struck weight (`cellSide`, `totalFor` in `ledger/settlement.ts`). A partial
event (some members hired, some default) splits the cell first through `ctx.cells.split`; never a
fraction of a member; never a headcount inside a cell. The key (region, cohort, bank) is registry
data; stratifying on a new relationship is a data change that lifts a row into the key. Resolution
is measured: Part XII runs the same seed at 1x, 2x, 4x `cellsPerKey`.

### 6.2 Decisions and outlooks (XI-16, §46)

Every decision reads the decider's own outlook: module-private state of the expectations module
(item 4), keyed by party and variable, formed adaptively from the party's own observations, which
are recorded as surprises. One preference: memory, drawn once at entry, dispersed. `view.outlook`
is added to `ParticipantView` at item 4 as a kernel door reading that slot. No global expectation
exists anywhere; a published aggregate is a lagged read that causes nothing.

A decision is a function of (own state, own outlook, public prints, params). Nothing else.

Outlooks are meant to disagree (§46 A3, XI-16): two parties with different histories expect
different prices, and that is the reason a book has two sides, the reason a mean-preserving spread
of outlooks produces threshold crossings that one outlook never would, and the channel a shock
transmits through (the parties surprised first act first). Never average outlooks into a sector
view, never seed them equal, never let one party read another's; the dispersion of outlooks is a
standing read (§46 E1) and a test asserts it is non-zero at the seed and widens in a downturn.

### 6.3 Markets and participants (Clearing)

A market is declared by the module that owns the instrument kind and opened when the instrument is
registered. One instrument, one market, one print per period. A participant declaration says why a
party of a kind is in a market; its orders come from the view. A dealer is a party of a kind with a
limit, a funding cost and inventory; its width is a consequence; there is no residual absorber.
Primary markets (auctions, book-builds) use the same solver with a fixed supply side: the issuer's
size at its reservation; buyers post schedules; failure is `noOverlap` and it has consequences the
issuer's module handles next period.

### 6.4 Failure, loss and the estate (XI-1, XI-2, XI-3, XI-8)

A failed instruction is a recorded state the payee's module reads. A loss is an event with a date: a
status written on a row, a provision booked, a write-off settled; never a rate. Death: a party ceases
through `ctx.cease(party, estate)`; the estate is a named party (kind `estate`, item 7) holding
everything the dead party held, selling it into the same markets over a stated programme, ranking
claims by each instrument's stated seniority, distributing by settlement. Forced sales are orders
posted by the party's own module in the next market session, with the reason journaled.

### 6.5 Time and ordering (Money G, Clearing F)

Dates are placed by the calendar; a module never counts periods. A phase reads prints of the current
period only after the market has run; before that `printOrThrow` throws `NotYetProduced`. If a phase
needs a print from later in the period it is anchored in the wrong place: move it. Section 8's order
is the plan; the assembled phase list is the fact.

### 6.6 Currencies (Currency, Spot FX, XI-12)

Until item 12, settlement refuses an instruction touching an instrument in a money other than the
party's home money. That guard is deleted in item 12 together with the FX revaluation that replaces
it, in one change. After that, every amount still carries its currency; conversion is a trade with a
counterparty in the spot market; nothing converts at the ledger boundary.

### 6.7 Parameters and placeholders (Law 2, XI-14)

Declare before use. `kind` is a decision. Policy has an owner and the register prints it. The count
of shapes and placeholders is on the inspector surface and in every audit report, and it must fall;
an item that raises it says why.

### 6.8 Naming and identity (Law 9, Appendix A)

Instrument ids are opaque; display names come from the kind's profile: a bond is issuer + coupon +
maturity; a bill issuer + tenor; a share its issuer; a good its sub-unit; a market the instrument or
(region, sub-unit). There are no buckets; a book is keyed by the instrument bought.

---

## 7. Testing strategy

| Layer            | What                                                                                                                     | Where                                   |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------ | --------------------------------------- |
| Profile tests    | terms validation, display names, due actions, unit                                                                       | `packages/engine/test/<module>.test.ts` |
| Phase tests      | one phase on a small assembled world; instructions drafted; failures recorded                                            | same                                    |
| Assembly tests   | with the module assembled: the seed audit passes; a year of stepping stays green, or its expected red families are named | same                                    |
| Property tests   | solvers, allocations, conservation over random instruction sets, determinism under seed                                  | `test/property/`                        |
| Resolution tests | 1x, 2x, 4x cells give the same aggregates to dust (from item 4)                                                          | `test/resolution/`                      |
| Kernel tests     | settlement routing, DvP, cells, calendar, register facade, module ordering                                               | `test/*.test.ts`                        |
| Surface tests    | the inspector renders a stepped world in a real browser                                                                  | `packages/app/e2e`                      |

**WHEN THEY RUN.** At the end of a full module, never during one. A suite run mid-item is measuring
a world that is half-built and reporting the half that is missing (Law 11), and a session that
answers it spends itself on tests instead of on the thing the tests are for. Write them as the item
goes; run them when the module closes; put everything the run says in `docs/BUGS.md` before changing
anything in response.

A test never widens a tolerance; a test that needs a bound to pass is a finding; a failing audit
family is asserted by name in the test and named in the record; every test that draws randomness
names its seed. **A test never names a party.** This world's banks, firms, listings and funds are
DRAWN (Seed B1.a) — `firm.4` is not "the big farm", it is whatever the draw made it — so a test asks
the draw for a mill, a dealer, a listed line (`test/rig.ts`), and a test that writes an id down is
asserting against a world that no longer exists.

---

## 8. The canonical period

Where each system's phases sit as the world fills in. A module anchors to the nearest phase and
names its cycle; assembly enforces non-decreasing cycles.

| Cycle | Order | Phase                                                                                                                                                 | Owner (item)                                     |
| ----- | ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| 0     | 1     | `expectations.form`: every party's outlook from its own history                                                                                       | expectations (4)                                 |
| 0     | 2     | `corporateActions`: coupons, maturities, dividends, amortisation                                                                                      | kernel                                           |
| 0     | 3     | `credit.events`: missed payments become defaults; statuses written                                                                                    | credit-events (5)                                |
| 0     | 4     | `margin.calls`: requirements re-measured against last close; calls issued                                                                             | derivative-layer (13a), prime-brokerage (13f)    |
| 0     | 5     | `treasury.programme`, `treasury.outlays`, `treasury.receipts`                                                                                         | treasury (3)                                     |
| 0     | 6     | `<system>.decide` for firms, households, banks, funds, insurers, dealers: decisions from own views                                                    | each system                                      |
| 1     | 1     | `labour.match`: the market in hours                                                                                                                   | labour (4)                                       |
| 1     | 2     | `markets`: goods, freight, housing, primary (auctions, books), secondary (bonds, shares, funds), FX, derivatives, commodities; forced sales post here | kernel; participants from every system           |
| 2     | 1     | `firms.produce`, `firms.invoice`, `households.pay`: inputs consumed, work in progress, deliveries, invoices, payments on terms                        | goods, firms, households (4), trade-credit (13e) |
| 2     | 2     | `estates.sell`: estate programmes post into the next session; distributions settle                                                                    | estate (7)                                       |
| 3     | 1     | `moneyMarket.clear`: after every flow: unsecured, secured, the facility                                                                               | money-market (11)                                |
| 3     | 2     | `resolution`: liquidity and solvency triggers, bank resolution, deaths                                                                                | banks-capital (11), firms (4), estate (7)        |
| 4     | 1     | `revaluation`: marks, FX, write-downs, liability re-marks                                                                                             | kernel                                           |
| 4     | 2     | `expectations.score`: surprises recorded                                                                                                              | expectations (4)                                 |
| 4     | 3     | `polity.election`: on its date                                                                                                                        | polity (14)                                      |
| 4     | 4     | audit                                                                                                                                                 | kernel                                           |

A forced sale caused by a margin call at cycle 0 sells in cycle 1's session of the same period; a
call caused by cycle 4's revaluation is issued next period at cycle 0. That one-period lag is the
honest consequence of "a market sees what has already happened" (Clearing F1). It is stated here
so nobody builds a second session to hide it.

---

## 9. Performance (Law 18)

Do nothing until a standing observation (Part XII: cost growth over a run) says cost grows. Then, in
a declared campaign: storage layout, traversal order, decomposition, scaffolding removal. Never a
mechanism, an economic quantity or a named boundary. Gate on behaviour: the audit families still
hold exactly; reported numbers do not move beyond dust; a relabelling is a declared re-baseline.
Candidates already visible: the audit copies every holding every period; `holdersOf` filters; per-lot
arrays for cells. None matters until thousands of cells and hundreds of instruments; measure first.

---

## 10. Dos and don'ts

**Do**: read the spec section before the code, every time · declare every number · cite every
clause · draft instructions and let settlement fail them · decide from `ParticipantView` · post
schedules, not points · anchor every phase explicitly · split a cell for a partial event · report a
family as not built until it checks something real · insert the missing prerequisite · write the
record as outcomes · tick the item's steps as you go · delete the item file when it closes.

**Don't**: write a bound, a default, a clamp, a percentage tolerance · branch on a kind id in a
mechanism · import another module or the world container · hold a reference to a store · cache a
value beside its units · read a print the period has not produced · seed an outcome · import an
observed ratio · give a party another party's private state · roll back a number · tune a number to
pass a VERIFY · measure mid-build · leave a TODO.

---

## 11. Risks and containment

| Risk                                   | Containment                                                                                           |
| -------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| A mechanism quietly assigns an outcome | REASON/VERIFY/FORBID review at coverage time; a VERIFY is never enforced; lint on bounds and literals |
| Two modules grow a hidden dependency   | cross-module import lint; shared types are kernel types or instruments                                |
| The kernel accretes kind branches      | lint on kind branches; kernel changes only by inserted item with a record                             |
| A placeholder outlives its death       | the count is on the surface and in every report; each death named in the record                       |
| Floating-point drift over a long run   | dust tolerances per check; flows and money families compare ledger to register every period           |
| Non-reproducibility                    | one PRNG with derived streams per module, party and period; no clock; determinism tests               |
| The surface changes the model          | worker boundary; snapshots are copies; the observer never writes                                      |
| The phone is too slow                  | Law 18 campaign, measured first; the engine's API has no layout commitments                           |
| The plan drifts from the code          | item files deleted on close; progress recounted by tool in `npm run check`                            |

---

## 12. Glossary

- **Kernel**: the stores and the loop; owns every writer.
- **Module**: one spec system, instrument family or seed, declared as a `SystemModule`.
- **Profile**: the behaviour of a kind, registered by its module, asked by the kernel.
- **Context**: `ParticipantView`, `MechanismContext`, `SeedContext`; the only doors.
- **Phase**: a module's work at a stated cycle, anchored to a kernel phase.
- **Participant**: a reason for a party of a kind to be in a market, evaluated with that party's view.
- **Family**: one of the nine audit families; made of contributions.
- **Cell**: a party standing for a population, with an integer weight and per-member state.
- **Print**: a cleared price with provenance, one per instrument per period.
- **Placeholder**: a declared number with a scheduled death.
- **Dust**: the only tolerance: terms × ε × Σ|terms|.
- **Door**: a method on a context; the kernel adds doors by inserted items only.

---

## 13. The items

The detailed implementation of each open item is in `docs/plan/`. The progress table at the top
links to them. Items 0, 1, 2 and 2a are closed; their outcomes are in `docs/RECORD.md`.
