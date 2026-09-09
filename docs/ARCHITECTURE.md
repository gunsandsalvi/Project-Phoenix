# Project Phoenix — Architecture

The specification (`docs/spec/PROJECT_PHOENIX.md`) contains no implementation. This document is the
set of implementation decisions, each derived from a clause of the specification. A decision that
cannot cite a clause is a preference and is marked as one. Update this file **in the same change** as
the thing it describes (Appendix C).

Citations use the spec's grammar: `Money C2.a` is system 1 (Money and Settlement), node C2.a.
`Law 7` is First Principle 7. `XI-15` is a mechanism. `App B 22` is a consolidated prohibition.

---

## 1. Platform and language

**Decision.** One TypeScript codebase. The **engine** is a pure library with no DOM, no I/O, no clock
and no randomness of its own. The **app** is a web front-end that runs the engine inside a Web
Worker. The Android build wraps the same web app with Capacitor.

**Derived from.** The deployment path is stated by the owner: continuous testing on GitHub Pages, final
delivery as an APK for a Pixel 11 Pro XL. A browser runtime is the only one that serves both without a
second implementation, and one implementation is law 4 applied to the codebase itself. The worker
boundary is §45 E3 — *no surface that changes the model* — made physical: the UI never holds a
reference to engine memory; it receives structured-clone snapshots.

**Toolchain.** TypeScript 5.9 in strict mode with `exactOptionalPropertyTypes`,
`noUncheckedIndexedAccess`, `noImplicitOverride`, `noFallthroughCasesInSwitch`. Vite for the app,
Vitest for tests, fast-check for property tests, Playwright (preinstalled Chromium) for an app smoke
test, ESLint with typed rules and project-specific rules (§9 below). npm workspaces:
`packages/engine`, `packages/app`.

---

## 2. Numbers

**Decision.** IEEE-754 doubles everywhere. Every identity check derives its own tolerance from the
arithmetic that produced the number: `sum(terms)` returns the value **and** its dust bound,
`count × ε × Σ|terms|`, using Neumaier compensated summation. There is no global epsilon and no
percentage anywhere.

`NaN`, `±Infinity` and `-0` cannot enter the state: every numeric constructor (`money()`, `qty()`,
`price()`, `rate()`) and every arithmetic helper validates and **throws** on a non-finite result.

**Derived from.** Law 7 defines the only admissible tolerance as *(number of terms) × (machine epsilon)
× (sum of absolute magnitudes)*, i.e. floating-point error — so the representation is floating point,
and the check carries its own dust. Audit A4/A4.a. Register B2.b.

**Rejected.** Integer minor units. Every trade at a real price would then produce a rounding residual,
and law 2 says a residual with no holder is a defect; the spec's own tolerance definition presumes
floats.

---

## 3. Value objects and units

Every quantity carries its unit and cannot be combined with a different one (App A, Units):

| Type | Fields | Rule |
|---|---|---|
| `Money` | `amount`, `ccy` | `add` throws across currencies (Money A2.b). No implicit currency (Currency A4). |
| `Qty` | `amount`, `unit` (`par`, `shares`, `tonnes`, `contracts`, `dwellings`, `hours`, …) | `add` throws across units. |
| `Price` | `amount`, `ccy`, `perUnit` | A price of money in itself is `1` — the only hard-coded one (Money D2). |
| `Rate` | `amount`, `per: Periodicity` | A rate without its periodicity does not construct (Law 8). Conversion is an explicit call through the calendar's day count (Money G3.c). |
| `Period` | integer index on the one calendar | No default period; a record without one does not construct (Money G4.a). |

**Missing is missing** (App A, Missing values). A read of something absent throws a
`Missing` error naming what was asked. There is no `?? 0`, no `|| 0`, no formatted default; the
lint forbids them in the engine. Callers that can legitimately proceed without a value use the
explicit `tryX` variant returning `Option<T>` and handle `none` in code that a reader can see.

**Identifiers** are branded string types (`PartyId`, `InstrumentId`, `AccountId`, `MarketId`,
`InstructionId`) so they cannot be mixed. An id is never a display name (Law 9). Display names come
from one naming grammar (`naming.ts`) that reads the instrument's own terms: issuer + coupon +
maturity, issuer + tenor, the issuer for a share.

---

## 4. The data model

### 4.1 Money is an instrument (Money D2, A1, B1)

A deposit at bank X in USD is the instrument `money:X:USD`, issued by X, unit `USD`. A reserve at the
central bank is `money:CB:USD`, issued by the central bank. An **account** (holder, issuer, currency)
is therefore a **holding** of a money instrument, and the register holds it exactly like a bond. Money
has no separate store, so money conservation and ownership conservation are one check family each
with no special cases. The issuer's money liability is the read of holdings in its money instrument
(Money A4, B1.b), never a stored aggregate.

### 4.2 The wire: instructions (Money D1–D4)

The **only** way state changes is by applying a numbered `Instruction`:

```
Instruction {
  id: sequential integer          // D1.a: replayable
  period, cycle                   // G2, G4: belongs to the cycle it was issued in
  legs: Leg[]                     // every leg names both sides
  reason: Reason                  // C1.b: why the units moved
  cause: CauseRef                 // C2: trade | coupon | maturity | corporate action | default | …
  price?: Price                   // C2.a: if a trade, the print
}
Leg = MoneyLeg { fromAccount, toAccount, amount: Money }
    | AssetLeg { from, to, instrument, qty: Qty, lots? }
```

**Settlement** applies an instruction with one rule (C2): payer minus, payee plus. For a money leg
between accounts at different issuers it generates the **interbank reserve leg** itself (C2.a); a
same-bank payment moves no reserves (C2.b). All legs of an instruction apply **atomically**: every leg
is pre-checked (free units, currency match, account existence, the issuer's overdraft decision), and if
any leg cannot apply, **none** does and a `Fail` is recorded with the instruction that failed and why
(Register C3, XI-5). Delivery versus payment is therefore a property of the ledger, not a convention.

Settlement is **final** (E2): there is no reversal API; a correction is a new instruction. Order within
a cycle is the instruction order (E3).

### 4.3 The register (Register A–F)

`Holding (holder, instrument)` → `{ lots: Lot[], liens: Lien[] }`. A lot carries quantity and basis
(D4); a lien encumbers units (D5); free units = held − encumbered, and **only free units can move**
(D5.a). Both directions are indexed — by holder and by instrument (D2.a) — and both indexes are
written by the single settlement path. Issued amount per instrument (B1) is changed only by issuance,
re-opening, buyback, amortisation, maturity; the audit compares holdings to it (B2).

Derivative contracts are **not holdings** (Derivative X1). They live in a separate `Contracts` store
whose invariant is zero-sum (D1.b). The foundation defines the store interface; the layer (§16) fills
it in its worklist position.

### 4.4 Parties: named or cell (XI-15)

A `Party` is `Named` or `Cell`. A cell has an integer `weight` (a count) and holds **per-member**
state: its holdings and accounts are stored per member, and the cell's total is `weight × member` at
read. An instruction leg on a cell is denominated **per member** and records the weight it was applied
at, so the counterparty's total is `perMember × weight` and both legs still sum to zero.

Consequences the representation enforces rather than checks:

- a cell is homogeneous by construction — there is no field a member could differ in;
- an event that applies to part of a cell must **split** it first (`splitCell(cell, members)`), which
  is exact; identical cells **merge**;
- the weight changes by exactly five events — entry, death, promotion, split, merge — through one
  `weightEvent` API that journals cause and date; nothing else can write it;
- aggregation is `cell.integrate(f) = f(memberState) × weight`. There is no `mean()`; the average is
  unreachable (XI-15: *a question that cannot be phrased will not be asked wrong*).

The cell **key** (region, cohort, bank) is registry data; lifting a relationship into the key is a data
change and a re-stratification event, never a mechanism change (Small-Business Pools A6.a).

### 4.5 Prices and value (XI-6, Clearing D/E)

`PriceStore (market, instrument, period)` → `Print { price, provenance, tradedQty }`, written only by
the clearing engine. Provenance is one of `traded`, `stale(fromPeriod)`, `interpolated`,
`extrapolated`, `none`. A market with no trades writes a **stale** print carried from the last traded
one, visibly (Clearing E4, C4.b); it never silently refreshes.

`value(holding, period) = qty × price(instrument, period)` at read. There is no stored value beside
units (App B 39). An instrument declared `carriedAtCost` in the registry values at its lots' basis; any
other unpriced instrument **throws** `Unpriced` when valued.

### 4.6 The clearing engine (Clearing A–F)

One solver for every market. A participant posts a `Schedule`: a monotone step function from price to
signed quantity, built from the participant's own state (A3) and never from the clearing price (A4).
The solver finds the price where posted supply meets posted demand (C1), rations by the market's stated
rule (C3), and returns one of the representable outcomes (C4.b):

`cleared | noDemand | noSupply | noOverlap | excessCommitted`

A search bracket is never returned as a price (C4.c). Trades are emitted as instructions (D2, D3), and
the print becomes the mark (D4). The solver is a pure function of the schedules (C5) and is tested for
determinism with property tests.

### 4.7 Time (Money G)

One `Calendar`: an epoch date, a period length of **7 days**, `cyclesPerPeriod` (a RESOLUTION
parameter), and one mapping period ↔ date. A periodicity is placed by **advancing a date** and
landing in the first period on or after it (G3.a); nothing counts periods. Day counts read the
calendar's dates (G3.c). No periodicity finer than a period exists (G3.b); finer structure is cycles.

### 4.8 The period loop

A period is an **ordered list of phases** held as data (`schedule.ts`), each a `Mechanism` with a
name, its spec citations, and a `run(world)`. Markets clear at their stated point (Clearing F1); a
phase that reads a print not yet produced this period gets a `NotYetProduced` error, not a stale
value — the fix is the order (F1.a). Every period ends with settlement of the last cycle, then
revaluation ordering checks, then the **audit** (Audit C1–C3). The loop is the same every period;
phases are never skipped conditionally (Audit C3).

### 4.9 The audit (Audit A–E)

Each **family** (money, ownership, prices, cross-market, accounts, names, flows, zero-sum, units) is a
module exporting `check(view): Violation[]` where `view` is a **readonly** projection of the world.
A violation names the owner (party, instrument or instruction), the size in a unit, the period and
the citation (A2, A3). Tolerance is the dust returned by the arithmetic (A4). The audit **never
repairs** (C4) — the type of `view` has no mutating methods. Output is counts by family, worst
instances, and it is reproducible from the seed (D1–D3). A family that is not yet built reports itself
as `not built`, never as green.

Equity is a **stated account** per party, moved only by named events; the accounts family compares it
to the read of assets minus liabilities (Audit B5, B5.a). The seed sets it once to the read; after
that only events move it.

### 4.9a Accounting: how equity moves, and the routing rule

These fell out of building items 1 and 2 and are recorded here because they are load-bearing.

- **Everything about a party is per member of that party.** Holdings, cash, the equity account and
  every effect on them are per member (a named party is a cell of weight one). The only place a
  total appears is a leg's other side and the register's `heldTotal`. Mixing the two was the first
  defect the audit caught.
- **Routing across issuers** (Money C2.a). A money leg from account (h1, i1) to (h2, i2): payer minus
  (or *creation* if h1 = i1), payee plus (or *destruction* if h2 = i2). If i1 ≠ i2, then for each
  issuer that is a bank (not the central bank): the bank's own money is redeemed on the paying side or
  issued on the receiving side, and its reserve account at the central bank moves by the amount. Money
  whose issuer is the central bank changes holder and is never redeemed by a transfer. Creation and
  destruction happen only through an issuer's own account (Money C4), which is what the money family
  counts.
- **Equity effects of an instruction** are computed from the legs, never from a "nature" flag: what
  came in at its price (or at the giver's carrying value for a transfer without a price) minus what
  went out at carrying value; for the issuer of a liability, minus what it issued at price plus what
  it redeemed at the holder's carrying value; and, on a holder-to-holder transfer, the issuer's
  liability is re-marked from the giver's carrying value to the receiver's basis (Register B3: the
  liability is the same number read from the other side). An exchange at a price nets to zero, a
  transfer is income and expense, a sale away from the mark is a realised gain or loss. Instruments
  whose profile says they are not a liability of their issuer (shares) produce no issuer effect, so
  capital paid in is simply cash received.
- **Carrying value** of a lot during period t, before revaluation, is the period t−1 print if the lot
  was acquired before t, else its basis. It is derived, never stored. Revaluation books
  `qty × (mark_t − carrying)` to the holder and the reverse to the issuer of a liability, so the
  accounts family holds exactly after it and world equity is zero-sum across every event.
- **The audit remembers** the previous period's issued amounts and per-member holdings so that the
  flows and money families compare the ledger's deltas with the register's change: two independent
  records (Audit A1.a). Both skip when asked twice in one period.

### 4.10 Registry and parameters (Law 2, Law 15, XI-14)

All data lives in the **registry**: currencies (each naming its issuing central bank), regions (each
naming its currency), units, party kinds, instrument kinds, goods, recipes, cell-key dimensions,
platforms. Behaviour that varies by kind lives in a **profile** behind a dispatch table keyed by kind,
with exhaustiveness enforced by the type system. Mechanics never branch on a kind (lint, §9).

Every number that shapes behaviour is declared in the **parameter register** with value, unit, owner,
and provenance kind: `technology | preference | policy | resolution | shape | placeholder`. A
placeholder names the mechanism whose absence it stands in for and the worklist item that deletes it.
The count of shapes and placeholders is a reported metric and must fall (XI-14). Engine code reads
numbers only through `params.get(id)`; numeric literals other than `0, 1, -1, 2` are linted out of the
engine except in `core/num.ts`.

### 4.11 Events and the observer surface (§45)

The `Journal` is an append-only list of events **generated from state transitions** by the engine
(defaults, fails, weight events, prints, failed auctions, …); it is a read of what happened, never an
input. The observer API (`observer/`) takes a `Scope` — `inspector` or `party(id)` — and answers
queries from a snapshot: prints with their provenance and age (A1.a), positions, public state,
published aggregates with a stated lag (A5), and the journal. A party scope filters out other
parties' private state (A4). The UI can only reach the engine through this API over the worker bridge.

### 4.12 Reproducibility (Seed A5, Audit D3)

The engine takes a seed and a registry; all randomness comes from one injected PRNG (`sfc32`) advanced
in a defined order. The engine never reads the wall clock or `Math.random` (lint). A run is identified
by `(seed, registry hash, periods)`; the same run produces the same instructions, prints and
violations.

---

## 5. Error discipline

Two kinds of wrongness, kept apart on purpose:

| Kind | Example | What happens |
|---|---|---|
| **Contract violation** — something impossible by construction | adding USD to EUR; a leg with one side; a move of encumbered units; an unpriced read; a rate with no periodicity; NaN; a missing period; a phase reading a print not yet produced; a weight changed outside the five events | **Throws** a `PhoenixError` subclass carrying the spec citation. Never caught inside the engine. The run stops at the site. |
| **Invariant violation** — a statement about the state that is false | holdings ≠ issued; money stock moved with no issuer act; equity account ≠ assets − liabilities | **Audit finding** with owner and size; reported, never repaired, never thrown (Audit C4; Law 11: deliberately failing checks are the normal state of an incomplete model). Tests assert on the report. |

Rules that follow:

- no `catch` that swallows; no `try` in mechanism code (lint);
- no `console` in the engine (lint);
- no default values for numbers; no optional numeric fields that mean "unset";
- exhaustive `switch` over kinds, checked by the compiler (`assertNever`);
- every public engine function validates its inputs with the value-object constructors, so a bad
  number cannot get past the first call.

---

## 6. Package layout

```
packages/engine/src/
  core/        num.ts errors.ts ids.ts money.ts qty.ts rate.ts option.ts assert.ts
  calendar/    calendar.ts periodicity.ts daycount.ts
  registry/    registry.ts params.ts profiles.ts naming.ts
  parties/     party.ts cells.ts
  ledger/      instruction.ts settlement.ts ledger.ts
  register/    instruments.ts holdings.ts lots.ts liens.ts register.ts
  prices/      price-store.ts value.ts
  clearing/    schedule.ts solver.ts outcome.ts
  contracts/   (arrives with the derivative layer: the zero-sum store, Derivative X1)
  audit/       audit.ts families/{money,ownership,prices,accounts,names,flows,units,...}.ts
  journal/     journal.ts events.ts
  world/       world.ts schedule.ts step.ts seed.ts snapshot.ts
  observer/    observer.ts scope.ts
  rng/         prng.ts
  index.ts
packages/engine/test/      mirrors src; property tests under test/property
packages/app/src/          worker.ts (engine host), main.ts, ui/
tools/                     spec-index.ts (parses the spec), check-citations.ts
docs/                      spec/ ARCHITECTURE.md WORKLIST.md RECORD.md COVERAGE.md
```

Mechanisms (Parts V–X) will live in `packages/engine/src/mechanisms/<system>/`, one directory per
spec system, each exporting the phases it contributes to the period schedule. They are added in
worklist order only.

---

## 7. Citations in code

Every module, mechanism and audit family carries `@spec` tags naming the clauses it implements:

```ts
/** @spec Money C2 C2.a C2.b */
```

`tools/check-citations.ts` parses the spec into an index of requirement ids and fails the build if a
citation does not resolve. `docs/COVERAGE.md` is the requirement → status map (`MET at <path>`,
`MISSING`, `OUT OF SCOPE (reason)`), re-marked in the same change that meets a requirement (App C).

---

## 8. Work discipline (Laws 10–17, Part XIII)

- `docs/WORKLIST.md` is the **one ordered list**. Work the first open item; a new item is inserted at
  the position its dependencies put it, and the record says where and why.
- One bounded change per item; a commit per item; the commit message says what and why.
- `docs/RECORD.md` is a ledger of outcomes, not a diary.
- No measurement, tuning or diagnosis of numbers until Part XII is reached. Deterministic checks
  (lint, types, tests, audit at period zero) are gates, not experiments.
- Never roll back a number. Only a change wrong on its own terms is undone.
- A bound is deleted in the same change that builds the mechanism it covered; until then it is a
  registered placeholder.

---

## 9. Project lint rules (self-correction)

Enforced by ESLint over `packages/engine/src` (rules in `eslint.config.js`, custom rules in
`tools/eslint-rules/`):

| Rule | Law |
|---|---|
| `phoenix/no-bounds`: no `Math.min`, `Math.max`, `clamp`, `Math.abs` used as a floor, outside `core/num.ts` | Law 6, App B 22 |
| `phoenix/no-numeric-default`: no `?? <number>`, `\|\| <number>`, `= 0` default params for amounts | App A Missing values |
| `phoenix/no-magic-numbers`: literals other than `0, 1, -1, 2` outside `core/`, `registry/`, tests | Law 2, XI-14 |
| `phoenix/no-kind-branch`: no `=== '<kind>'` comparisons on `.kind/.sector/.industry` inside `mechanisms/` | Law 15, App B 48 |
| `phoenix/no-clock-no-random`: no `Date`, `Math.random`, `performance.now` in engine | Seed A5, Audit D3 |
| `phoenix/no-console`, `no-empty` catch, `no-restricted-syntax` on `try` inside mechanisms | §5 |
| `@typescript-eslint/switch-exhaustiveness-check`, `no-explicit-any`, `no-non-null-assertion`, `strict-boolean-expressions` | §5 |

The parameter register is checked at engine start: a placeholder without a named mechanism and
worklist item fails construction.

---

## 10. Deployment

- **GitHub Pages** (`.github/workflows/pages.yml`): on push to `main`, build the app with base path
  `/Project-Phoenix/` and deploy. Every push to any branch runs `ci.yml` (lint, typecheck, unit and
  property tests, citation check, app build, Playwright smoke).
- **Android** (`.github/workflows/android.yml`, manual and on tags): build the web app, run
  `cap add android` and `cap sync` from `capacitor.config.ts` (the generated project is not committed),
  `gradle assembleDebug`, upload the APK as an artifact. Release signing is added when a keystore
  exists; it is not part of the foundation.
- The app ships a web manifest so it installs as a PWA on the Pixel during development; the APK is
  the final form of the same build.

---

## 11. Decisions deferred, with the clause that makes them a decision

- **Inspector or participant surface** (§45 A4): the observer API takes a scope, so both are
  possible; which one the shipped app is remains an owner decision. Development uses the inspector
  scope.
- **Engine performance** (Law 18): storage layout is the campaign's to change later; nothing in
  the API exposes it. Cell arithmetic is per member so a resolution change is a data change.
- **Persistence**: a run is reproducible from `(seed, registry, periods)`, so saving a run is saving
  those; a materialised snapshot format is not needed until the app needs resume.
- **Single currency in settlement**: until the currency layer (worklist 12) revalues foreign positions
  into equity (Currency D2), an instruction may not touch an instrument in a money other than the
  party's home money; settlement throws `Mismatch` (Money A2.b). The guard is deleted in that item.
