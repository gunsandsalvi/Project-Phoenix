# Project Phoenix — the implementation

> **This file replaces `docs/AUDIT.md`.** It carries every open finding that file held, adds the
> sectors that file could not see, and states each as work with files and steps rather than as
> evidence. `docs/AUDIT.md` is deleted in the change that lands this file.
>
> **One item at a time, in the order in Part 1 (Law 10).** An item is done when its exit holds, its
> steps are ticked, `docs/RECORD.md` has its entry and `docs/COVERAGE.md` is re-marked — one commit
> (Law 14). When an item closes, delete its section; the record says what it closed.
>
> **A finding under an item is evidence for the change, not a task of its own** — but a finding that
> is a MISSING SECTOR is an item, not evidence. That distinction is why this file exists: the file
> it replaces filed the absence of six sectors as a finding (`C-4`) under an item it then marked
> done, and the rule "do not chase a finding" kept anyone from looking at it again.
>
> **No test is run until an item's steps are complete.** Lint and typecheck are cheap and may run at
> any time; the suite is a measurement and measurements come last (Law 11).

---

## Part 0 — The measured state

### 0.1 What exists, by spec system

`docs/COVERAGE.md`, one row per REASON, VERIFY and FORBID, aggregated per spec system by
`npm run check:existence`. **1,361 clauses: 815 MET, 90 PARTIAL, 456 MISSING — and 95 of the MET
carry `NEVER REACHED`**, meaning a module cites the clause and has never produced an outcome. So
**641 of 1,361 clauses (47%) are missing, partial, or dead.**

Which system a clause belongs to is the SPEC's fact and is joined from the spec index, never parsed
out of COVERAGE's own headings (Law 4). A clause is counted where the spec puts it, and a spec
sub-clause carrying no REASON/VERIFY/FORBID word is a NOTE rather than a requirement — **8 `MET`
rows mark one of those** (`Sovereign I1.a`, `Banks Lending C1.d`, `XI-11`, `Labour A3.b`,
`Households A2.b`, `Households F1.a`, `Households F1.b`, `Households F2.a`) and are outside this
count. The tool names them on every run rather than absorbing them: two documents disagreeing about
what a requirement IS is the kind of thing this check exists to show. Every spec clause has a row —
checked, not assumed.

| system | MET | PARTIAL | MISSING | NEVER REACHED | total |
|---|---|---|---|---|---|
| Money | 28 | 0 | 8 | 0 | 36 |
| Register | 20 | 4 | 2 | 0 | 26 |
| Clearing | 21 | 5 | 1 | 0 | 27 |
| Audit | 20 | 3 | 0 | 0 | 23 |
| Seed | 16 | 0 | 6 | 0 | 22 |
| Currency | 24 | 0 | 1 | 0 | 25 |
| Bond | 14 | 1 | 1 | 0 | 16 |
| Derivative | 18 | 0 | 0 | **3** | 18 |
| **Corporate Credit** | **7** | 4 | **51** | **3** | 62 |
| Sovereign | 40 | 3 | 8 | **3** | 51 |
| **Short-Term Debt** | **0** | 0 | **19** | 0 | 19 |
| Equity | 25 | 2 | 10 | 0 | 37 |
| Money Market | 25 | 3 | 0 | 0 | 28 |
| Spot FX | 26 | 1 | 0 | 0 | 27 |
| Fund Shares | 23 | 3 | 0 | 0 | 26 |
| Securities Lending | 9 | 0 | 12 | **9** | 21 |
| **Prime Brokerage** | **0** | 0 | **24** | 0 | 24 |
| Derivative Layer | 32 | 0 | 0 | **17** | 32 |
| CDS | 17 | 0 | 8 | **17** | 25 |
| IRS | 14 | 1 | 5 | **13** | 20 |
| FX Forwards | 13 | 0 | 8 | 0 | 21 |
| Commodity Futures | 5 | 0 | 15 | **5** | 20 |
| **Commodities Spot** | **3** | 1 | **20** | 0 | 24 |
| Indices | 21 | 1 | 0 | **1** | 22 |
| Banks Lending | 21 | 4 | 7 | 0 | 32 |
| Banks Funding | 28 | 4 | 0 | 0 | 32 |
| Banks Capital | 19 | 3 | 1 | 0 | 23 |
| Dealer Desks | 26 | 1 | 0 | **2** | 27 |
| Insurers | 9 | 0 | 14 | **9** | 23 |
| **Hedge Funds** | **0** | 0 | **24** | 0 | 24 |
| **Private Equity** | **0** | 0 | **25** | 0 | 25 |
| Treasury | 20 | 1 | 4 | 0 | 25 |
| Central Bank | 22 | 3 | 4 | 0 | 29 |
| **Polity** | **0** | 0 | **32** | 0 | 32 |
| Firm | 20 | 7 | 3 | 0 | 30 |
| Capital Programme | 22 | 3 | 0 | 0 | 25 |
| Firm Birth | 7 | 6 | 12 | 0 | 25 |
| M&A | 10 | 0 | 12 | **10** | 22 |
| Trade Credit | 8 | 3 | 11 | 0 | 22 |
| Goods | 27 | 2 | 10 | 0 | 39 |
| Freight | 17 | 3 | 0 | 0 | 20 |
| Labour | 23 | 1 | 3 | 0 | 27 |
| **Housing** | **6** | 1 | **19** | **3** | 26 |
| Households | 20 | 4 | 9 | 0 | 33 |
| **Small-Business Pools** | **2** | 0 | **26** | 0 | 28 |
| **Cross-Border** | **6** | 2 | **18** | 0 | 26 |
| Ratings | 16 | 3 | 4 | 0 | 23 |
| Reporting | 32 | 5 | 1 | 0 | 38 |
| Observer | 16 | 0 | 10 | 0 | 26 |
| Expectations | 17 | 2 | 8 | 0 | 27 |

Regenerate with the check in item 1, never by hand.

### 0.2 The ten systems that produce nothing

**Five have no clause MET at all.** A sixth, Small-Business Pools, has two, and both are the generic
cell kernel (`parties/party.ts`, `world/cells.ts`) rather than anything that is a small firm.

| sector | spec | MET/total | who owns it today |
|---|---|---|---|
| **Polity** | §47 | 0 / 32 | worklist **14** → item **19** |
| **Private Equity** | §29 | 0 / 25 | worklist **13o** → item **13** |
| **Prime Brokerage** | §15 | 0 / 24 | worklist **13o** → item **13** |
| **Hedge Funds** | §28 | 0 / 24 | worklist **13o** → item **13** |
| **Short-Term Debt** | §9 | 0 / 19 | **nobody** → item **10b** |
| **Small-Business Pools** | §42 | 2 / 28 | **nobody** → item **11** |

**And five more are built on paper and dead in the world** — every clause they have MET carries
`NEVER REACHED`, so a module cites it and has never produced an outcome. The check names these
separately because a reader scanning for "0 MET" walks straight past `M&A 10 MET`, and all ten of
those are marks on a mechanism nothing has ever invoked.

| sector | spec | MET (all never reached) / total | the finding | item |
|---|---|---|---|---|
| **CDS** | §17 | 17 / 25 | `A-66`, `B-7` — the book cannot produce a first print | **6** |
| **M&A** | §35 | 10 / 22 | `B-4` — the market for control buys shares and never combines | **6**, **13** |
| **Insurers** | §27 | 9 / 23 | `A-9`, `B-2` — no policy can be registered in any world | **9**, **14** |
| **Securities Lending** | §14 | 9 / 21 | `A-67`, `B-3` — nothing ever borrows a security | **9** |
| **Commodity Futures** | §20 | 5 / 20 | `A-66`, `B-7` | **6**, **18** |

**Ten of fifty-one systems produce nothing.** Five were never written; five were written, marked
`MET`, and never reached. The second kind is the more dangerous, because it reads as done.

### 0.3 How six things were lost: the handoff chain

Traced from the worklist's own rows. Each arrow is an item closing and naming where its unbuilt half
goes.

```
13e done → PLACED small-business cells, the covered bond (M7), factoring,
           receivable pledges, senior notes as repo collateral  →  13f
13f done → its PLACED list names NONE of those five.                     ← dropped
13f done → prime brokerage, restructuring → 13h ;  short-term debt → 13i
13h done → "...came in from 13f and 13g and go on with them"             ← no destination
13i done → "hedge funds, private equity ... came in from 13h"            ← 13i closed; both 0
13n open → states it itself: "PLACED forward three times (13f to 13g to 13h) without landing"
```

Six things were placed into items that then closed without them: **small-business cells, factoring,
receivable pledges, senior notes as repo collateral, the covered bond (M7), short-term debt.** None of
the six had an open home when this file was written. Hedge funds, private equity and prime brokerage
survived only because 13o happens to name them.

This is finding **B-14** — *"a finding was positioned into item 13h, 13h closed, and the finding was
not done"* — repeated at least six more times, including once at the top level of `docs/AUDIT.md`
itself: its item **18** was titled *"The sectors that were waiting"*, owned four findings, and was
marked **DONE** having built one of them (`C-3`). `C-4`, the row naming the six absent sectors, was
inside it.

### 0.4 Why it was invisible, and the check that ends it

Three reinforcing reasons, and the third is the one to fix:

1. `docs/AUDIT.md` was produced by **reading `packages/engine/src`**. An absent sector leaves no
   trace in source, so the method could not find one. It found the six only by carrying them in from
   a deleted holding pen.
2. `docs/COVERAGE.md` is the file that *can* answer the question — one row per clause — and nothing
   ever **aggregated** it. The query is one `awk` line and had never been run.
3. `MET` is a claim about the source, not about the world, and the file says so in its own header —
   which means a reader who wants to know what EXISTS has to hold two facts in their head at once and
   nothing checks the pair.

**Item 1 makes this a check.** A rule that can be a check should be one.

---

## Part 1 — The order

Dependencies, not preference, and the open lines before the new ones. The four arrows that matter:

- **1 before everything**: until a claim of completion is falsifiable, no item below can be believed.
- **3 (the gather) before 6, 7 and 17**: a bank that employs nobody quotes nothing, and a market with
  no dealer cannot open a derivative book, place a bill, or price a corporate bond.
- **9 (`Mandate`) before 13**: building hedge funds on today's `fund` party bakes the pool/decider
  conflation in permanently.
- **10 (the corporate bond is issued) before 10b (short-term debt)**: a firm that cannot issue paper
  at five years cannot issue it at three months either; the issuance path is one mechanism.

| # | item | closes | why here |
|---|---|---|---|
| ~~**1**~~ | ~~The existence check~~ | — | **DONE** — `check:existence`. It found five more dead systems and an 8-row disagreement between the spec and COVERAGE on its first run |
| **2** | The dimension sweep, finished | 18 | the last open row of a half-done item; it makes 18 findings compile errors, which must then be fixed |
| **3** | The gather | 4 | one missing call chain kills lettings, bank employment and every dealer quote in the world |
| **4** | The families that cannot fail | 9 | "checks green" is currently satisfied by four families that cannot fail; nothing below can be measured until they can |
| **5** | Missing is Missing, carried | 4 | a defaulted zero defeats 1 and 2 |
| **6** | The derivative books open | 3 | needs **3** (a dealer), and everything in the layer is downstream of a first print |
| **7** | The three closed lines | 2 | needs **3**; `dwelling` takes the whole housing module with it |
| **8** | The securitisation waterfall | 2 | independent; the subtraction has gone the wrong way round since 13e |
| **9** | The seven private books, and `Mandate` | 5 | the larger half of item 8 of the old file; blocks 13, and `A-43` behind it |
| **10** | The corporate bond is issued | 2 | needs **3** (a buyer); 51 Corporate Credit clauses stand behind it |
| **10b** | Short-term debt (§9) | — | **inserted**: the roll that can fail; one of the six lost things; needs **10**'s issuance path |
| **11** | Small-Business Pools (§42) | — | **inserted**: dependencies (trade credit 13e, bank lending 13d) are both closed; takeable as soon as **9** gives it an agreement |
| **12** | Firm birth | 1 | needs **7** (`Lifecycle`, built) and **15** (`Objective`, built); worklist 13n |
| **13** | Asset managers: §28, §29, §15 | 1 | needs **9**; worklist 13o |
| **14** | Insurers and pensions (§27) | 2 | needs **9** (a policy is an agreement, not an instrument) |
| **15** | Housing, the rest | — | needs **3** and **7**; worklist 13m, and `E-5` |
| **16** | Cross-border, the rest | — | needs **10b** (foreign-currency issuance) and **13** |
| **17** | Corporate credit, the rest | — | needs **10**; 51 clauses, the largest single gap |
| **18** | Commodities spot and futures | — | 35 clauses across two systems; needs **6** for the futures half |
| **19** | The polity (§47) | 3 | worklist 14; carries `D-1`, `D-2`, `E-7` |
| **20** | Periodicity: the fee and the assessment | 1 | worklist 13k; needs **19** for who sets a fiscal period |
| **21** | The local repairs | 3 | each belongs in the next change that opens its file |
| **22** | The recipe | — | worklist 15; last before measurement, deliberately |
| **23** | Measure (Part XII) | 1 | worklist 16; carries `C-1`'s 82 red |
| **24** | The app and the APK | — | worklist 17 |

**Stage A (1)** makes the project able to tell the truth about itself. **Stage B (2–9)** finishes
every line the old file left open. **Stage C (10–18)** builds the sectors that are not there.
**Stage D (19–24)** is the existing worklist tail.

---

## Part 2 — The items

---

## 1. The existence check — **DONE**

> **BUILT: `tools/coverage-existence.ts`, `npm run check:existence`, in `npm run check`.** And it
> found two things on its first run, which is what it is for.
>
> **Ten of fifty-one systems produce nothing, not five.** Five have no clause MET at all. Five more
> — CDS, M&A, Insurers, Securities Lending, Commodity Futures — have every clause they MET marked
> `NEVER REACHED`. The second kind is the more dangerous because it reads as done: a reader
> scanning for "0 MET" walks straight past `M&A 10 MET`, and all ten of those are marks on a
> mechanism nothing has ever invoked. `builtAndDead()` names them separately, because the remedy is
> different — an absent sector needs writing, and one of these needs a way IN to what is already
> written. Part 0.2 carries both lists.
>
> **The two documents disagree about what a requirement IS.** `docs/COVERAGE.md` has **1,369** rows;
> the spec has **1,361** REASON/VERIFY/FORBID clauses. The eight extra are `MET` rows against spec
> sub-clauses that carry no form word — `Sovereign I1.a`, `Banks Lending C1.d`, `XI-11`,
> `Labour A3.b`, `Households A2.b`, `Households F1.a`, `Households F1.b`, `Households F2.a` — which
> the index correctly calls NOTEs. Marking work you did is not wrong; a denominator that silently
> disagreed with the spec's own would be. They are reported on every run rather than absorbed. In
> the other direction, **every spec clause has a row** — checked by the test, not assumed, because a
> clause nobody answered and a clause somebody deleted look identical from inside the file.
>
> **It runs no world, and that is a decision.** Step 1.3 as first written had the tool run the rig
> and cross-check `world/reach.ts`. It does not: the `NEVER REACHED` marks are already WRITTEN into
> `docs/COVERAGE.md` by whoever re-marked it, so the tool READS them (Law 19) instead of
> re-deriving them. The reach read is what keeps those marks true; this is what counts them. The
> gate stays two file reads — fast, deterministic, and safe to put in `npm run check`.
>
> **Which system a clause belongs to is joined from the spec index**, never parsed out of COVERAGE's
> own headings (Law 4: one writer). The existing `readCoverage` and `buildSpecIndex` are reused;
> no third parser was written.

**Why.** Six sectors were lost because a `done` row and a `MET` mark are claims nobody can falsify.
`docs/COVERAGE.md` holds the facts — one row per clause — and nothing aggregated it. The query is one
command and had never been run.

### Steps

- [x] 1.1 `tools/coverage-existence.ts`: `existence()` joins `readCoverage()` to `buildSpecIndex()` by clause id and accumulates `{met, partial, missing, outOfScope, neverReached, total}` per system, in the spec's own order. A clause with no row counts MISSING.
- [x] 1.2 Emit **ABSENT SECTORS** (`met === 0`) and **BUILT AND DEAD** (`met > 0 && neverReached === met`), then the full table.
- [x] 1.3 ~~Cross-check against `world/reach.ts` by running the rig~~ — **not done, and deliberately**: the marks are read from where the reach read caused them to be written. `marksOnNotes()` and `unanswered()` report the two ways the files can disagree instead.
- [x] 1.4 `tools/test/coverage-existence.test.ts`: eight tests over a two-system fixture — the join, MET-and-never-reached as two facts about one row, a NOTE not counted, both absence reads — plus three against the real files: every absent sector is named in this plan, Part 0's table is the generated one, and no spec clause lacks a row.
- [x] 1.5 `package.json`: `check:existence` → `tsx tools/coverage-existence.ts --verify`, in `check` after `check:forbids`. Verified in both directions: green as committed, exit 1 with a diff when a figure in Part 0 is perturbed.
- [x] 1.6 Part 0 regenerated from the tool. Record entry written.

### Exit — held

`npm run check` fails if a spec system's counts move without Part 0 moving with them; a system with
no MET, and a system whose every MET has never been reached, are named on every run; and "item X is
done" is checkable against what exists in one command.

---

## 2. The dimension sweep, finished

**Why.** `docs/AUDIT.md`'s item 16 is the one item left half-built, and it is half-built in a
specific way that matters: `Measure<D>` **erases**, so typing a site changes no behaviour — it makes
a wrong program stop compiling. Stages 1–9 and 10a/10b are done (**1,091 → 407 arithmetic sites**);
one row remains. Closing it turns **eighteen findings into build failures**, and each then has to be
FIXED — the typing surfaces them, it does not repair them.

**Read this before starting.** `Qty` is two claims in one name: a dimension (pieces of something) and
an invariant (a whole number of them). `asQty`/`addQty`/`subQty`/`negQty` assert the grid; a DESIRED
POSITION is not on the grid and must use `asAmount<'piece'>` with `plus`/`minus`. Typing targets with
the grid doors made 330 tests throw `[Law 8] … is not a whole number of the unit's pieces` in stage
10a. The grid assertion belongs at `registry.deliverable`, where a quantity becomes one the register
can hold, and nowhere earlier.

### Files

```
packages/engine/src/mechanisms/{banks,funds,households,firms}/**   the residue
packages/engine/src/seeds/**
packages/engine/src/core/measure.ts     if an operation is genuinely missing — never an escape hatch
```

> **THE ITEM IS IN STAGES, and the reason is Law 14.** As first written this item bundled the
> TYPING — mechanical, no behaviour change, gated by Law 18 — with eighteen FINDINGS, each of which
> is a real change to what the world does. That is nineteen bounded changes in one item. The typing
> is stage **2a**; the findings are grouped by what they are, **2b** to **2e**, and each carries its
> own record entry. Inserted here rather than appended (Law 10) because every one of the findings is
> a program the type refuses once 2a lands.
>
> | stage | what | sites |
> | --- | --- | --- |
> | ~~2a.1~~ | ~~`banks`, `funds`, and the household demography~~ — **DONE** | **402 → 390** |
> | ~~2a.2~~ | ~~`firms`~~ — **DONE**: a recipe coefficient is a `Ratio` | **390 → 372** |
> | 2a.3 | `seeds`, and the residue | 372 |
> | 2b | the conservation breaks: `A-39`, `A-68`, `A-19`, `A-1` (`A-18` closed in 2a.1) | |
> | 2c | rates read as levels: `A-44`, `A-58`, `A-65` | |
> | 2d | the currency reads: `A-23`, `A-47`, `A-50`, `A-51`, `A-61` | |
> | 2e | the local ones: `A-5`, `A-6`, `A-32`, `A-33`, `A-38` | |
>
> ### Stage 2a.1 — as built
>
> **`LoanTerms.rate` and `SubTerms.rate` are `Ratio`s**, and that one change is the shape of the
> whole stage: a rate scaled by a fraction of a year is a SHARE of par, and what that comes to per
> unit is par scaled by the share. The bare `1` in `add(1, interest)` is gone — there is a named
> `PAR` in each file, *"one unit is one piece of its money"*, which is the one place the two scales
> coincide (`E-9`) and now says so instead of being assumed. The cascade was two sites, because
> stage 3 had already typed everything around them.
>
> **`E-11`, and the type found it**: `Outcome.price` is a `PerPiece` and **some books clear a RATE**
> — the subordinated raise, the money market, the IRS, the CDS. The bidders post rates and the
> solver strikes one, and nothing in the book can say which of the two its level is. It is `E-10`'s
> shape (one field, a different dimension per use) at the CLEARING layer rather than the contract
> layer. Named at the door in `subordinated.ts` rather than assumed; positioned below.
>
> **`A-18` is closed, both ends.** `households.waiting` carries the part of a person standing at each
> cell's cohort boundary and at its mortality, per cell and per event, so `Math.floor` TIMES a real
> event instead of deleting it. Both ends: the fraction below one now waits, and a cell whose whole
> weight would cross now crosses AS ITSELF rather than being skipped — which is what pinned the tail
> of every band where it was. Declared `physics` in the ontology register: who is partway through the
> year in which they cross is this sector's own demography and wants no kernel home.

### Steps — stage 2a, the typing

- [x] 2a.1 `banks` and `funds`: `LoanTerms.rate` and `SubTerms.rate` → `Ratio`; `interestTo` → `scale`; `PAR` named in both files; `hoursNeeded` → `scale`, `linesCovered` and `probabilityOfDefault` → `ratioOf`; the desk's one-sided flow → `plus`/`minus`/`absolute`/`ratioOf`; the fund's redemption shortfall → `minus`. `mul`, `div`, `add` and `sub` leave five files.
- [x] 2a.2 `firms`: every recipe coefficient — `hoursPerUnit`, `yieldRate`, `qtyPerUnit`, `unitsPerUnitPerPeriod`, `spoilage` — is a `Ratio`, so what a stock reaches is `over(stock, coefficient)` and what a batch draws is `scale(batch, coefficient)`. Four `asRatio(tech.…)` wrappers deleted as redundant (Law 12). `plannedBatch`, `productiveHours`, `hoursUnderContract` and `areaUnderUse` carry `Qty`; the sales outlook enters as an amount once rather than at each of its three readers.
- [ ] 2.1 Take the residue in the order the file names it — `banks`, `funds`, `households`, `firms`, `seeds`, then what is left — not by site count. Run `npx tsc --noEmit` after each module; the compiler generates the list.
- [ ] 2.2 For each site the compiler rejects, decide which of three it is: a DIMENSION that was right and unstated (type it), a DIMENSION that was wrong (that is one of the eighteen — fix it, below), or a TARGET typed with a grid door (switch to `asAmount<'piece'>` + `plus`/`minus`).
- [ ] 2.3 `E-8` — a declared price does not say which of the two scales it is in. `registry/params.ts`: `dimension: 'price'` splits into `'price:piece'` and `'price:named'`; `equity.openingShare` is the second and every goods level is the first. This is the cause of the known "a share worth a hundredth of a cent".
- [ ] 2.4 `E-9` — a dirty price adds two scales. `prices/curve.ts:readCurve` and `clearing/market.ts` add a per-piece print to a per-named-unit accrual; right today only because par and money share a subdivision. Convert at `Registry.priceOf`, which is the door.
- [ ] 2.5 `E-10` — `Contract.struckAt` means a different dimension per kind: a price for a bond future, a **spread** for a CDS, a **rate** for a swap, an **index level** for an index future, a **basis** for a cross-currency swap. One field, five dimensions. Make `struckAt` a discriminated union keyed off the derivative kind's profile, so the kind's own `mark` and `premiumPerUnit` take what they mean. **This finding was never in the old index; it is `E-10` here.**

### Steps — the eighteen the type now refuses

- [ ] 2.6 **A-39** (the largest conservation break in the model). `labour/matching.ts:payFrom` returns a boolean; it computes `share.total` — what actually moved — and throws it away. Return `share.total`. Then `payWages` sets `bill.paid` from it instead of from the unrounded `mul(perMember, row.headcount)`, and `produce.ts` capitalises the money that changed hands. Also: `payFrom` returns `true` when `share.total <= 0`, recording a sub-piece wage as fully paid with no money leg — that branch returns the zero, and the caller books nothing.
- [ ] 2.7 **A-68**. `freight/index.ts`, the loading instruction: `costPerUnit: div(add(share, 0, 'the freight'), take, …)` — the `add(x, 0)` is the tell, and the missing term is what the cargo cost. Use `costOfDraw(lots, take)` from `register/register.ts`, which is exported for this read: `costPerUnit = (costOfDraw + share) / take`. `arrive()` is already correct and is the pattern.
- [ ] 2.8 **A-65**. `options/index.ts:optionOrders`: `mul(outlook.expected, outlook.confidence)` is money² per unit². The premium must be built from `confidence` as the WIDTH it is, and must depend on `t.strike`, `t.right` and `t.expiry` — today none of the three appears in `mine`. The reservation is the party's own; D7's "the premium is what clears" stays.
- [ ] 2.9 **A-58**. `securitisation:priceFor` is `1 − (owed/(owed+equity))²` — a leverage ratio squared called a cost of funds, with no periodicity and no dependence on the pool. Discount the note's own cash flows at the bank's published `FundingCost.perAnnum` (`banks/index.ts:publishCostOfFunds`) with the same `priceAt(flows, required, on, dayCount)` the money funds use. And `noteBids` posts a single point for the whole spare cash — post a schedule (Clearing A2).
- [ ] 2.10 **A-44**. `HOUSEHOLD_PARAMS.liquidityPremium` is declared `per annum OVER WHAT A DEPOSIT RETURNS` and used as the bare 0.005. Deposits pay now (`money-market/deposits.ts:payDepositInterest`). `portfolio.ts:fundOrders` must compare `p.offered` against `depositRate + premium`, reading the board through `households/bank.ts:board()`.
- [ ] 2.11 **A-47** and **A-50** together. `funds/index.ts:eligible` admits a line on live/kind/tenor with **no currency test**, so every money fund's mandate is every sovereign bill in the world; `ordersOf` then divides one currency by another; `nav.ts:navOf` sums two moneys. Add the currency to the mandate, convert through `ctx.valuation.inMoney` in `navOf` and `holdingsWorth`, and delete `moneyOf` — `ctx.accountOf` is the one writer of which account a party holds a money in.
- [ ] 2.12 **A-51**. `inMoney` is exported on `MechanismContext.valuation` and called by **no module**. Close it at the reads that walk `holdingsOf`: `funds/nav.ts`, `funds/index.ts:holdingsWorth`, `banks/capital.ts:capitalOf`, `money-market/resolution.ts:valueBook`, `households/consume.ts:wealthOf` and `atRisk` (**A-23**), `equity/index.ts:531`.
- [ ] 2.13 **A-61**. `central-bank-omo/index.ts:remit` sends to `treasuries[0]` — an insertion-order artefact — in its own money. Use `registry.centralBankOf(ccy)` paired with `treasuryOf(ctx, bank)` (already in `money-market/resolution.ts`) so each treasury owns its own central bank.
- [ ] 2.14 **A-18**. `households/lifecycle.ts`, twice: `Math.floor(mul(weightOf(cell), share))` with a comment claiming the fraction "stays where it is until enough of it has accumulated". Nothing accumulates. Add the per-cell remainder the comment describes, carried forward, so `floor` times an event rather than deleting it. Then `crossing >= weightOf(cell)` moves the cell as itself rather than skipping it.
- [ ] 2.15 **A-19**. `settleEstates` pays out in one currency (`currencyOf(office.region)`) where `handToProbate` takes in every money a dead cell held. Pay out in every money probate holds; a foreign balance there is otherwise permanent.
- [ ] 2.16 **A-1**. `ledger/settlement.ts:reseat` books a TOTAL into the per-member equity account; `issue` and `redeem` wrap in `perMemberOf` and `reseat` does not, nor does the issuer re-mark in the `credit` case. Make `bump` take the total and divide, so there is one door and a new writer cannot forget.
- [ ] 2.17 **A-33**. `world/world.ts:1204 lastOwn` has no period bound. Add one predicate at the four `labour.wages` readers (`firms/decide.ts:wagesDue`, `wageFacing`, `hoursUnderContract`; `firms/index.ts:wagesPromised`), at `firms/invest.ts:quotedRate`, at `treasury/index.ts:lastWageBill`/`wageItFaces`, and at `banks/staff.ts:linesCovered`.
- [ ] 2.18 **A-38**. `labour/matching.ts:reservation` reads `outlook('income')` — which includes coupons, distributions and (per A-37, closed) sale proceeds. A cell's outside option is what it lives on WITHOUT the job. Read the benefit and the non-labour income separately.
- [ ] 2.19 **A-32**. `levelsBelow` declares `households.demand.steps` a RESOLUTION and its levels are `opinion × k/steps` — a grid of 5 and a grid of 7 share only the top level, and the grid's BOTTOM is `opinion/steps`, so the count sets how far down the cell bids at all. Either make it invariant under refinement, or re-declare it a SHAPE whose count must fall. Same for `pricesOver` in `consume.ts`.
- [ ] 2.20 **A-5**. `registry/registry.ts`: `payable(_ccy, amount)`, `cashFor(_ccy, value)`, `deliverable(_unit, qty)` take a unit and ignore it. Drop the parameter, or make it do the conversion it claims. Not both.
- [ ] 2.21 **A-6**. `goods/index.ts:122-129` declares `minutes per piece of <subUnit>` and holds hours (`TIME_PIECES = 1`, and `registry/grid.ts` says the piece of time is the hour). Correct the unit string. Same stale premise at `labour/index.ts`, whose comment describes a thousandth of an hour.
- [ ] 2.22 Run `npm run check`. Read what it says and write every new finding into this file under the item that should fix it — do not chase one.

### Findings this closes (18)

`A-1` `A-5` `A-6` `A-18` `A-19` `A-23` `A-32` `A-33` `A-38` `A-39` `A-44` `A-47` `A-50` `A-51`
`A-58` `A-61` `A-65` `A-68`, and `E-8`, `E-9`, `E-10`. Bodies in Part 3.

### Exit

`what: string` is gone from the arithmetic; `mul(money, money)`, `add(usd, eur)` and
`mul(perMember, headcount)` are compile errors; and the eighteen are fixed rather than merely
unwriteable. The whole suite is run once at the end and what it says goes in this file.

---

## 3. The gather

**Why.** `World.gather` (`world/world.ts:940`) is the **only** path that runs `venueParticipantDecls`,
and across the whole engine `ctx.gather(...)` appears **once**: `money-market/index.ts:269`. Three
modules declare `venueParticipants`; one of them is gathered.

| module | what the schedule is | gathered? |
|---|---|---|
| `banks` — `sessionOrders` | a bank's money-market schedule | **yes**, by money-market |
| `banks` — `staffOrders` | a bank's bid for labour hours | **never** |
| `housing` — household rent orders | every bid and offer in the lettings venue | **never** |

The chain that follows is the largest single unlock in the model:

```
no gather → no bank bid for hours → no employment row → no labour.wages event
          → linesCovered() === 0 → covers() false for every line
          → dealingOrders() returns [] for every bank, every market, every period
```

So **no bank makes a market in anything** (A-60): no bid-offer spread, no inventory, no inter-dealer
market, `market.noView` on every book households and merchants do not cover, no primary dealer at a
sovereign auction, and XI-2's forced-seller door for banks cannot open because `urgentSale` sits
*after* the `covers` gate. And the lettings venue has never had an order in it (B-5), so no tenancy
is signed, no rent is paid, and `housing.rent` — a whole phase — does nothing for ever.

**What it is.** The door is built, documented and correct. Call it.

### Files

```
packages/engine/src/mechanisms/labour/index.ts     gather the labour venue
packages/engine/src/mechanisms/housing/index.ts    gather the lettings venue
packages/engine/src/mechanisms/banks/staff.ts      covers / linesCovered
packages/engine/src/mechanisms/labour/matching.ts  supply (A-43 rides here)
```

### Steps

- [ ] 3.1 `labour/index.ts`, the phase that opens the occupation venue: call `ctx.gather(venue, 'labour')` before `clear`. `banks/staffOrders` then posts, and a bank employs people.
- [ ] 3.2 `housing/index.ts:lettings`: call `ctx.gather(rentVenue(region), 'housing')` before `letIn`. The venue has bids and offers for the first time.
- [ ] 3.3 `banks/staff.ts:covers` picks the covered lines as the **first `linesCovered` entries of `view.instruments.all()`** — global insertion order, identical for every bank, so it is one global cutoff rather than a per-desk specialisation, and every line past position `linesCovered` is quoted by nobody however many banks make its kind. Make the desk choose its own lines from its own `BankDecl.makes`.
- [ ] 3.4 `banks/dealing.ts:dealingOrders`: `urgentSale` must be reachable when the desk covers nothing — a bank that cannot quote can still be a forced seller (XI-2, Banks Funding D1, Money Market A2.b). Move the `urgent` branch **above** the `covers` gate.
- [ ] 3.5 **A-43** rides here and is the reason to do it now. `labour/matching.ts:supply` walks `ctx.parties.ofKind(HOUSEHOLD)` and posts each cell's order itself — the buyer's market deciding for the seller, which `MechanismContext.gather`'s own contract forbids in as many words. With the venue gathered, `households` declares a `venueParticipant` and `supply` deletes. **Blocked on item 9**: a households-side participant cannot see who is already employed or what trade they have, and that book is one of the seven. Do 3.1–3.4 here; do 3.5 in item 9 and say so in the record.
- [ ] 3.6 Do not measure. Lint and typecheck only; the suite runs when the item's steps are done.

### What it deletes

`labour/matching.ts:supply`, once 9 lands — a module building another module's party's schedule.

### Findings this closes

`A-54`, `A-60`, `B-5`, `B-6`.

### Exit

A bank employs people and quotes a two-sided market; the lettings venue clears a tenancy; `world/reach.ts`
stops naming `participant:banks/bank` and `venueParticipant:housing/household`.

---

## 4. The families that cannot fail

**Why.** The loop's definition of done is *"an item is done when its checks are green"*, and **four of
the nine audit families are green for reasons that are not the state of the world** (`B-10`). A green
audit has been part of the evidence for thirteen `done` rows. Until this closes, no measurement below
means anything.

Audit A1.a, in the spec's own words: *"a read of two independent things that must agree — never a
read of one thing against itself, which always passes."*

### Files

```
packages/engine/src/audit/families/currency.ts   A-10
packages/engine/src/audit/families/flows.ts      A-11
packages/engine/src/audit/families/names.ts      A-12
packages/engine/src/audit/families/units.ts      A-13
packages/engine/src/mechanisms/labour/index.ts   A-14  workforceIdentity
packages/engine/src/mechanisms/goods/index.ts    A-42  unitsIdentity
packages/engine/src/mechanisms/capital-programme/index.ts  A-42
packages/engine/src/mechanisms/funds/index.ts    A-48  equityIsZero
packages/engine/src/world/assemble.ts            A-4
```

### Steps

- [ ] 4.1 **A-10**. `currency.ts:revaluationAddsUp` builds `booked` and `implied` from the same event's `delta`, `carried`, `was` and `now` — and `world/revalue.ts:revalueForeign` computed `delta` as exactly `carried × (now − was)`. It is one path twice. Reach the second half from the REGISTER: walk the holdings, take each one's carrying and the period's rate move, and compare the total against what the equity and revaluation accounts actually moved by.
- [ ] 4.2 **A-14**. `labour/index.ts:workforceIdentity`'s B5 line accumulates `acc.states` as `(E ? w : 0) + (¬E∧W ? w : 0) + (¬E∧¬W ? w : 0)` over the two booleans — mutually exclusive and exhaustive, so `acc.states === acc.people` by construction on every iteration. Replace with a second, INDEPENDENT record: the counts the labour mechanism itself acts on when it hires and separates, against the cells. (The two neighbouring checks, `row.headcount !== weight` and `inRows !== acc.employed`, are real — they read the employment book against the parties store. Only the B5 line is empty.)
- [ ] 4.3 **A-42**. `goods/index.ts:unitsIdentity` and `capital-programme/index.ts:461` both count **every weight event in the world** and switch themselves off when there is one — and `households/lifecycle.ts:age()` journals a `weight` event every period from the first ageing, so neither family has reported anything since. **Delete the exemption rather than narrowing it**: of the three ways a weight moves, `splitCell`/`reKeyCell` preserve the total, `dieCell` refuses a cell that still holds anything, and `weightEvent` is called by nobody (A-17). The guard protects against a case that cannot occur and pays for it with two families.
- [ ] 4.4 **A-48**. `funds/index.ts:equityIsZero`'s docstring claims the zero *"falls out of the wire"*. It does not: `fundShareKind.owes: 'value'` with `derive → navOf().perShare` makes `assets − liabilities = 0` an algebraic identity. Rewrite the docstring to say what the family CAN catch — a disagreement between two valuation paths — and keep the check. (Item 2.11 makes that disagreement real by fixing `navOf`'s currency.)
- [ ] 4.5 **A-11**. `flows.ts`'s `copied` set exempts EVERY holding of a cell named in a split, promotion or merge for the whole period. What has no leg behind it is the copy of the parent's book at the instant of the split, not everything that cell then does. Compare against the parent's remembered per-member state: the split is exact, so the new cell's opening position IS the parent's and any difference is a leg.
- [ ] 4.6 **A-12**. `names.ts`: `m.id === i.market.valueOf()` compares a string to an Option object and is always false; the second disjunct does the whole job. Delete the first.
- [ ] 4.7 **A-13**. Part XII's units family is *"including a population: the sum of cell weights equals the population it stands for, and every represented party sits in exactly one cell."* Neither is in the kernel family; the only contributor that counts people is `labour`, through employment. Add the identity for every cell in the world, not only household cells that have a job — `firms`' small-business pools (item 11) and XI-15 cells generally can lose or invent members today and no family looks.
- [ ] 4.8 **A-4**. `registry/environment.ts:conditionsFor` claims *"assembly is where it fires"* and `world/assemble.ts` checks module ids, dependency cycles, money issuance and bank choices and nothing else — `grep -rn exposedTo world/` returns nothing. A recipe naming a fact no environment module declares silently multiplies by 1 for ever. Check it at assembly and **delete the paragraph** (Law 12: the fix removes the comment).

### Findings this closes

`A-4`, `A-10`, `A-11`, `A-12`, `A-13`, `A-14`, `A-42`, `A-48`, `B-10`.

### Exit

Every audit family that reports green is capable of reporting red; no family compares a number
against itself; and "checks green" is evidence again.

---

## 5. Missing is Missing, carried

**Why.** `docs/AUDIT.md`'s item 2 turned the lint rule on in `core/num.ts` and closed `A-49`. It
carried four out, correctly, because they are **missing mechanisms rather than missing values** (Law
11) — and then nothing built the mechanisms.

### Steps

- [ ] 5.1 **A-25**. `households/index.ts:consumptionIsBought`: `mul(leg.qty, leg.pricePerUnit.some ? leg.pricePerUnit.value : 0, 'what it took')` — a `? : 0` on an Option inside a mechanism. The family then reports `took 0 of goods and paid X`, a violation whose size and message are about the missing price rather than the flow. Either report the unpriced leg as its own defect, or refuse it as inadmissible. Not a zero.
- [ ] 5.2 **A-34**. `firms/decide.ts`, the input bid: `wage.some ? wage.value : 0`. The same function handles a missing wage correctly twice (`unitCost` returns `none()`; `worthMaking` switches its test) and then prices labour at zero in the one place that reaches a market — making the bid too high by `hoursPerUnit × wage`, which for most recipes is the largest term. Every firm is in this state until it has employed somebody, so at world open every input market clears against systematically inflated bids and firms that have never hired outbid the ones that have. `wageFacing` already has the right fallback (`labour.goingRate`); a firm that knows neither is a `Missing`.
- [ ] 5.3 **A-45**. `banks/index.ts:costOfFunds` returns exactly **zero** on three paths: a bank funded by nothing, period 0, and a zero-length year. Zero is not "unknown" here, it is "money is free", and it flows straight into `quote()` and into the desk's edge — so in period 0 every bank in the world quotes as if its funding cost nothing. `FundingCost` says `Missing`, and a bank that cannot cost its funding does not quote a rate (which is what the surrounding code does everywhere else).
- [ ] 5.4 **A-30**, first and third bullets. `portfolio.ts:ownUncertainty` returns 0 for a cell with no income outlook — reported as wanting no extra return for a claim that promises nothing, which reads as certainty and is ignorance. `lifecycle.ts:heirOf`'s `?? ''` went with the function at item 14 of the old file; confirm it is gone and that a world with no cohorts SAYS so.

### Findings this closes

`A-25`, `A-30`, `A-34`, `A-45`.

### Exit

No numeric default survives in a mechanism; a firm that cannot price labour does not bid; a bank that
cannot cost its funding does not quote.

---

## 6. The derivative books open

**Why.** **Nine derivative kinds are declared and not one contract of any class has ever been
written.** 7,510 option sessions all `noDemand`; 3,680 commodity-future sessions all `noDemand`; 60
CDS sessions all `noSupply`; the IRS books never run. **58 of COVERAGE's 99 never-reached MET marks
are this layer.**

The cause (`A-66`) is structural and identical in eight of the nine. Every class builds a party's
order as

```
target = <its hedging need, from its own book>
       ± <conviction, IF its own number differs from THIS BOOK'S LAST PRINT>
order  = target − <what it already has>
```

Each class is careful that the book's own print must not be the LEVEL it posts — and none noticed the
print is still load-bearing for the **direction**. With no print the conviction term drops out, every
party is left with its hedging need, and a hedging need has one sign.

| book | with no print | first session | opens? |
|---|---|---|---|
| fx forward | hedgers one way + an arbitrageur quoting **both sides** | two-sided | **yes** |
| option (put) | `held × aversion / multiplier` for holders, 0 for everyone else | buy-only | no |
| option (call) | 0 for everyone | **no orders at all** | no |
| bond future | `−held / contractSize` | sell-only | no |
| commodity future | `−held / lotUnits` | sell-only | no |
| interest-rate swap | `−fixedDebtOf(view, t)` | sell-only | no |
| CDS, single name | `exposureTo(view, t)` | buy-only | no |
| index future | `book / perContract`, and the only `side` in the file is `'sell'` | sell-only **always** | no |
| CDS series | `if (!last.some) return []` on the book's OWN line | **no orders** | no |
| cross-currency swap | `if (!last.some) return []`, then buy-only | **no orders** | no |

The one that works is the one whose author hit the problem and built the answer — `fx-derivatives`,
a party with nothing to hedge quoting a bid a tick below and an ask a tick above its own carry, with
the comment *"a book whose members were all hedgers printed one number for ever"*.

### Steps

- [ ] 6.1 The general fix, per class: a participant **with nothing to hedge** posts two-sided around its own number, exactly as `fx-derivatives` does. Its number is its own — the cash market's charge for this credit, its own curve, its own carry — and **never this book's own last print**, which is the fixed point the single-name CDS already refuses in a comment.
- [ ] 6.2 `index-futures/futureOrders` has **no buy branch at all**; its docstring is *"A DESK LONG A BOOK OF SHARES SELLS THE INDEX"* — one true reason, and the only one implemented. §46 A3 and XI-13 require two. Add the other side.
- [ ] 6.3 `cds/cdsIndexOrders` and `fx-derivatives/xccyOrders` read their own book's last print as a **precondition** and then post at a multiple of it. Delete the precondition and give each a maker with its own number, per 6.1.
- [ ] 6.4 **C-6, and it is a real missing mechanism rather than a bug.** 31,640 margin-admission decisions, 31,640 refusals, all one gate: *no room — the party holds no cash in the book's currency*. `capacityOf(view, ccy, buffer)` is `cash − cash × buffer`, so room is zero exactly when the party holds none of that money, and every one of them is an FX forward whose parties hold neither leg. The missing mechanism is **eligible collateral in another money**, which `money-market/collateral.ts` already implements for repo (`advances`, `valueToLender`, `haircut`). The derivative layer's `admits` asks only about cash. Wire it.
- [ ] 6.5 `derivative-layer/index.ts:refusedThisPeriod` is documented as *"E4: a standing measurement"* and is called by nobody. Either call it from the layer's audit contribution or delete it — a docstring asserting a measurement that does not happen is worse than the absence (`A-69`).
- [ ] 6.6 Once a book prints: `zeroSum`, the derivative layer's whole invariant, stops walking a set of size 0. That is part of `B-10` and is why this item is after 4.

### Findings this closes

`A-66`, `B-7`, `C-6`, and `A-69`'s `refusedThisPeriod` and `cdsBookOrders` rows.

### Exit

Every derivative book produces a first print without a prior print; the margin gate admits somebody;
`zeroSum` has a non-empty set; and the option-implied move §46 A3 needs exists.

---

## 7. The three closed lines

**Why.** Three of this world's 63 goods have a firm, a recipe, a market and a printed opening
price — and **no bidder, ever**. They are zero at the seed and nothing can start them.

| good | recipe | portable? |
|---|---|---|
| `dwelling` | timber, concrete, steel, glass, building trades | no |
| `facilities` | chemicals, power, paper | no |
| `itServices` | power, electronics | no |

Every route was checked and all are closed: not in the household basket (18 rows, no dwelling); no
recipe names them as an input; `merchants/index.ts:marketsOf` filters `!i.terms.portable`; not in a
fund mandate; not in a bank's `makes`.

They are also **zero at the seed by the same fact**: `foundation.ts:985` treats them as final goods,
`wantedInAPeriod` has no basket entry for any of them, so `want = 0`, `started = 0`, `plantOf = 0`.
And `firms/decide.ts:plan` needs an outlook of its own sales, which `expectations` forms only from an
asset leg the firm was a side of. **A firm that has never sold has no outlook; with no outlook it
makes no plan; with no plan it starts no batch; with no batch it never sells.** Three closed loops at
zero from period zero.

`dwelling` is the one with consequences beyond itself: it takes the whole housing module with it
(`A-55`) and its market's seed placeholder is never replaced — **a PLACEHOLDER with no scheduled
death (Law 2)**, and it is the number `wearOf` and every mortgage size are built on.

### Steps

- [ ] 7.1 `housing/index.ts:askForMortgages` opens `if (cell.representation === 'cell') continue;` — every household is a cell, so no household ever publishes a `housing.funding` request, no bank writes a mortgage, `mortgagesOf` is always empty, and `charge` and `foreclose` are dead phases. The blocker its docstring names (a borrower that misses a payment goes on accruing) is **item 19's arrears** and **item 9's agreement**. Remove the guard when 9 lands; until then this item is 7.2–7.4.
- [ ] 7.2 Give `dwelling` a buyer that is not a mortgage: a household cell that owns fewer dwellings than its members live in bids for one out of its own savings. Owner-occupation is then an OUTCOME, which is what the module header already claims (*"a household that owns as many dwellings as its members live in has nothing to rent"*) and today describes a state no household can be in.
- [ ] 7.3 `facilities` and `itServices` are **inputs nobody draws**. Either a recipe draws them — a firm buys IT services and facilities management, which is what those lines are — or they are not lines. Check `goods/data.ts:RETAIL`/`MAKES` against the input graph and connect or delete. A line with a firm, a market and no buyer is Law 2's placeholder with no death.
- [ ] 7.4 The bootstrap under all three: `firms/decide.ts:plan` returns `{planned: false}` without an outlook of its own sales. A firm that has never sold needs a first reason to make anything. Give the plan a fallback that is a REASON and not a number — the going rate for the line in its own region, which `labour` already publishes for hours and `goods` can for a line.

### Findings this closes

`A-55`, `A-56`.

### Exit

Every declared good has a bidder; no market's opening print survives the first session; the housing
module's buying half runs.

---

## 8. The securitisation waterfall

**Why.** *"a waterfall paying by seniority out of what was actually collected … with a loss landing
from the bottom and nothing stopping it reaching the senior"* — worklist 13e, **done**. **No loss is
ever allocated to any tranche, whatever the borrowers do.**

The vehicle collects **principal + interest** (`LOAN.due` emits a coupon every period and a maturity
at the end, paid to the holder of record). What goes out is **principal only**: `trancheKind` is a
pure pass-through (`cashFlows: () => []`, `due: () => []`, `accrued: () => 0`) and `payTranche`
redeems face at par. So `Σ out ≤ Σ face = the pool's opening principal`, and the interest — the whole
economic return of the deal — never leaves.

Three consequences, and the third disables the module's own subject:

1. **A noteholder earns nothing but its discount.** It pays `priceFor(...)` per unit of face and
   receives exactly face.
2. **The residual has no holder.** When the notes are fully redeemed the vehicle still holds the
   un-run-off rows and every coupon it ever collected. Nothing ceases a vehicle whose pool has run
   off, the party kind has no owner, no equity claim and no distribution, and `fails: ['cash','solvency']`
   will not fire on a party with positive equity. Appendix B: *"no residual with no holder"*.
3. **`absorb` goes inert.** `lost = notes − pool`; because interest is paid out as accelerated
   principal, `notes` falls faster than `pool`, so `lost ≤ 0` early and permanently and the
   junior/senior waterfall, the attachment points, the write-down leg and D4's senior losses are all
   downstream of a subtraction that has gone the wrong way round.

### Steps

- [ ] 8.1 `securitisation/index.ts:payTranche` — separate **interest** from **principal** in what the vehicle pays out. The module's own header states the invariant twice (*"Σ tranche face equals the pool's face after every event"*) and both readings are only true if the two are separated. Interest goes out as interest by seniority; principal redeems face.
- [ ] 8.2 `trancheKind` gets real `cashFlows` and `due`, so a noteholder's return is the interest it is owed and its yield derives from the price it paid (C3).
- [ ] 8.3 With 8.1, `absorb`'s `notes − pool` is a real comparison and a loss reaches the junior. Assert C6: **losses allocated sum to losses incurred, exactly. No tranching creates or destroys loss.**
- [ ] 8.4 The residual: the vehicle's equity has a holder. C4.a — *"often the originating bank keeps the bottom, which means the risk did not leave"* — is the honest answer and makes E3 (*no risk transfer without a transferee*) true rather than vacuous.
- [ ] 8.5 `A-57`'s last part: nothing ceases a vehicle whose pool has run off; `distribute` removes a deal only when the vehicle has already ceased. Give it a wind-up through item 14's `Process`, which exists.

### Findings this closes

`A-57`, `B-8`.

### Exit

A loss lands on the junior tranche and can reach the senior; a noteholder receives interest; a
run-off vehicle winds up and its residual has a named holder.

---

## 9. The seven private books, and `Mandate`

**Why.** `docs/AUDIT.md`'s item 8 built the `Agreement` store and wired four cases. **The seven
private books were not migrated, and that is the larger half.** `EmploymentRow`/`EmploymentBook`,
`Lease`/`LeaseBook`, invoices, `StockLoan[]`, loans, covenants, deals are one noun — two named
parties, dated terms, a state — invented seven times, none visible to the kernel.

`Mandate` rides on that migration, and with it the whole of item 13: **a decider, a pool, and a rule
profile** (`mayHold`, `requires`, `charges`, `leverage`). That is what splits the fund into the three
things it actually is — a **pool** that holds and has no opinions, a **mandate** that rules, and a
**manager** that decides. A separate account is a mandate whose pool is the client's own balance
sheet. An ETF and an MMF are pools with different redemption rules. **A hedge fund is a mandate with
leverage**, and `borrows` moves off `fundKind`, where it is hard-coded `false` — which is what 13h
claimed to have built hedge funds on.

### Steps

- [ ] 9.1 Migrate the seven, one bounded change each (Law 14 — seven items in one commit is what the old item refused, correctly): employment, lease, invoice, stock loan, loan, covenant, deal.
- [ ] 9.2 `Mandate` as a kind of agreement. `fundManagerKind` — a party whose only behaviour is answering where it banks — is deleted; the management fee stops being the placeholder that admits *"no manager competes for the mandate"*, because two managers can now bid for one.
- [ ] 9.3 **A-9** — the insurer, four independent blockers, each alone fatal. (1) `insurers/index.ts:124` returns the CURRENCY CODE where a `UnitId` is wanted, via the file's one `as unknown as` cast — so `Instruments.add` throws `Missing [Appendix A] unit USD does not exist` and **no policy can be registered in any world**; the module declares `COVER` and never uses it. (2) `phases: []`, `participants: []` — nothing runs. (3) `runCover` clears a book and **discards `outcome.fills`**: a session that strikes a price and moves no money. (4) `derive` returns `priced.some ? priced.value : 0` directly under a comment forbidding exactly that. (5) `claimsSeen` reads the insurer's OWN HOLDINGS for policies — a policy is its LIABILITY, the beneficiary holds it — so `written` is always empty. Use `view.instruments.issuedBy(self)`.
- [ ] 9.4 **A-67 / B-3** — securities lending. `runBorrows`, `wantsToBorrow` and `returnLoans` are exported and called by nobody; the one phase walks `state(ctx).open`, which nothing ever pushes to. Two further defects inside the unreachable code, which matter because they are what will run: (a) the fee is **not cleared** — one bidder per session and every lender posting `price: 'market'`, so `outcome.price` is the borrower's reservation regardless of how much paper is on offer, while the docstring says *"Scarce paper is dear and abundant paper is cheap, and neither is a table"*; (b) **two `Want`s on one instrument double-post the lenders**, because `world.post` appends and `postings` clears only at the top of a period — the book shows twice the supply that exists. And `willPay` is `owed/(owed+equity)`, the same non-rate as `A-58`.
- [ ] 9.5 **B-2** — `insurers()` has no `seed`, so no `insurance` party is ever created. Give it one.
- [ ] 9.6 **A-43** lands here (see 3.5): with the employment book in the kernel, `households` declares a `venueParticipant` that can see who is already employed, and `labour/matching.ts:supply` deletes.
- [ ] 9.7 **B-14** is unpositioned in the old file and lands here: whether a fund should hold contracts at all is `Fund Shares A3`'s question, and `TRADES_CONTRACTS` is still `[BANK, FIRM]`. A mandate says what a pool may hold; that is the answer.
- [ ] 9.9 **The stores that stand in for a noun that now exists.** Four kernel nouns were built and the module stores that stand in for them were never migrated, so `registry/nouns.ts` still declares each a PLACEHOLDER pointing at a closed item. Same shape as 9.1, same change: `expectations.outlooks`, `research`, `ratings` and `banks/reserves` → `View` (old item 6 named these three in as many words: *"three private stores that are this noun in three shapes"*); `funds`' previous NAV → `PublishedStatement` (*"a published figure belongs where published figures live"*). Each placeholder's `standsInFor.planItem` points at **this item** until it is migrated, and the declaration is deleted when it is.
- [ ] 9.8 **C-1's ETF row** (`12d-8`): a creation delivers a slice of the book and a desk without the basket does not create; nothing moves a share back to a desk, so `E3` runs one way and a premium of 0.28 of NAV has nobody able to close it. The answer exists once 9.4 lands — *whoever must deliver may borrow* — and needs a kernel door of the shape `termsOffered` has, because a module may not import another.

### Findings this closes

`A-9`, `A-67`, `B-2`, `B-3`, `B-14`, and `A-43` from item 21.

### Exit

A policy is written and an insurer exists; a stock loan is an agreement and a borrow is reachable; a
manager decides for a pool it does not own; `Mandate` exists and item 13 is unblocked.

---

## 10. The corporate bond is issued

**Why.** `mechanisms/corporate-bond/index.ts` declares `CORPORATE_BOND`, `corporateBondId(issuer, n)`,
a full `InstrumentKindProfile` with `cashFlows`, `due`, `ranking` and cross-default, a covenant table
and the `covenant.test` phase.

```
$ grep -rn "kind: CORPORATE_BOND" packages/engine/src   →  nothing
$ grep -rn "CORPORATE_BOND"       packages/engine/src   →  only its own declaration and profile
```

**No corporate bond has ever been issued and none can be: there is no issuance path.**
`testCovenants` runs every period over an empty set. Three `MET` marks stand on it, and **51 more
Corporate Credit clauses (item 17) stand behind it.**

The clause the module is for — a firm funding itself in a market rather than at a bank — has no
mechanism that puts a firm in it. `firms/index.ts:publishFunding` publishes what a firm is short of
and `banks` reads it; **nothing reads it as a reason to issue paper.**

### Steps

- [ ] 10.1 A firm compares two prices for the same money: the rate a bank quoted it (`credit.quoted`, already published) and what a market would charge. When the market is cheaper and the size is worth the fixed cost of an issue, it issues. **Both sides must be cleared prices** — the bank's quote is (Banks Lending C3.a), and the issue's is what the auction strikes.
- [ ] 10.2 The issuance path: a phase in `corporate-bond` that reads `firms.funding`, builds the line through `corporateBondId`, opens a market, and places it. **A placement needs a buyer** — which is item 3 (a bank's dealing desk), item 6 (nothing yet), the money funds (mandate currency, item 2.11) and the insurers (item 9). This is why 10 is after 3 and 9.
- [ ] 10.3 `testCovenants` then runs over a non-empty set, on the issuer's PUBLISHED accounts — which `journal/published.ts` already answers typed. A breach is an EVENT, never a bound (Law 6).
- [ ] 10.4 Cross-default: the profile already says a corporate bond cross-defaults where a sovereign does not. With a live issue, assert it.

### Findings this closes

`B-1`, and `B-13`'s "almost nobody borrows" (96 loans against 9,006 firms) in part.

### Exit

A firm issues a bond because a market was cheaper than its bank; the covenant test has something to
test; `world/reach.ts` stops naming `instrumentKind:corporate.bond`.

---

## 10b. Short-term debt (§9) — **inserted**

**Where this came from.** Worklist 13f closed with *"PLACED: … short-term debt and the roll that can
fail → 13i"*. **13i closed and its row does not mention it.** §9 is **0 of 19 clauses MET** and no
open item owns it. Inserted here because A1 makes it the same instrument contract as item 10's bond
and the same issuance path — building it before 10 would build that path twice.

**Why it matters beyond its own 19 clauses.** `docs/AUDIT.md`'s C-4: *"A firm cannot issue commercial
paper. Spec 9 has no module. `money-market` has interbank rows and repo, which is the BANK's
short-term funding; a FIRM funding itself at three months and the roll that can fail is what the
clause is about."* B3.b — a **run** — is the only place in this world where an issuer must repay
maturing paper out of cash it does not have and must find the money somewhere. Without it there is no
funding-side failure mode at all: every failure in this model today is solvency, never liquidity.

### What it is

`src/mechanisms/short-term-debt/`, one `SystemModule`.

- **The instrument** (A1): satisfies the bond contract, answering it its own way — **no coupon**,
  issued at a **discount**, redeemed at par, and the discount is the whole return (A1.a); under a
  year (A1.b); senior unsecured, ranking with the issuer's other senior debt (A1.c); no option
  (A1.d). `unit`, `priceTick`, `cashFlows` (one flow, at maturity), `due`, `ranking`.
- **The day count is material** (A2.a): at this tenor the quoting convention is part of the number.
  Declare it per line; `calendar`'s `yearFraction` already takes one.
- **Types by issuer, and the type is the credit** (A3): the state, a bank, a firm — one kind, three
  issuers, no kind branch (Law 15).
- **Why an issuer issues** (B1, B2): a short, known need — a tax date, a seasonal working-capital
  swing, a bridge — and because it is cheap when the curve is upward-sloping. Both are reads the
  issuer already has: `treasury` knows its tax dates, `firms` knows its wage bill and its input
  orders, and the curve is `prices/curve.ts`.
- **The roll** (B3): a rollover is a **new issue into a market that must clear** (B3.a). The issuer
  is asking the market to lend again **and it may not** — so a `noDemand` on a roll is the event, not
  an error.
- **The backstop** (B4): a committed bank line, and **it costs money in every period it is not
  used**. *"A committed line with no commitment fee on undrawn headroom is a free option the lender
  did not sell."* This is an `Agreement` (item 9) with a fee, not an instrument.
- **Buyers** (C1, C2): a money fund, a corporate treasurer, a bank liquidity book — the reasons are
  yield against the alternatives (a deposit, a repo, a central bank facility), credit, and liquidity.
  C2.a makes this a channel a policy rate travels down, which is what item 19 needs.
- **A limit per issuer** (C3), *"which is why a deteriorating issuer loses funding before it loses
  solvency"* — the mechanism this whole world is missing.
- **Collateral** (D3): with a haircut, which `money-market/collateral.ts` already implements.

### Steps

- [ ] 10b.1 The module skeleton: kinds, units, params, one phase anchored at `markets`, one participant per party kind, an audit contribution, a seed contribution. Cite every clause with `@spec`.
- [ ] 10b.2 The instrument kind: discount-to-par, one cash flow at maturity, senior unsecured, stated day count. **The yield is derived from price and days to maturity, never into it** (A2, Law 3).
- [ ] 10b.3 The issuer's reason: a firm or treasury with a dated need inside the tenor issues, sized to the need. No investment rate, no issuance schedule — the need is a read of its own book.
- [ ] 10b.4 The roll as a new issue into a market that must clear. **E1: no automatic roll.** *"Paper that always rolls at a written rate is not debt; it is a permanent liability with a coupon, and it removes the only risk the instrument has."*
- [ ] 10b.5 B3.b, the run: when the roll does not clear, the issuer must repay out of cash it does not have. It sells (XI-2's forced seller, which item 3 reopened for banks), it draws the backstop, or it fails. **All three paths must exist** or the run is a scripted event.
- [ ] 10b.6 The backstop as an `Agreement` with a commitment fee on undrawn headroom, paid every period it is not used (B4).
- [ ] 10b.7 The buyer's limit per issuer (C3), held as a `View` (item 6 of the old file, built) — the buyer's own opinion, not a table.
- [ ] 10b.8 C4 as a VERIFY, never an enforcement: when the policy rate moves, the bill yield moves with it **because the buyers' alternative moved**. Measure it; do not tie it.
- [ ] 10b.9 D4: a spread over the equivalent-tenor bill is a **derived read of two cleared prices**, never stored. E2: no price without a market. E3: no negative outstanding, and no maturity that passes without cash moving.
- [ ] 10b.10 Seed contribution, COVERAGE re-marked for all 19 clauses, record entry.

### Exit

A firm funds a three-month need in a market; a roll fails at least once in a long run and the issuer
finds the money somewhere or fails; §9 is 19 of 19 or states what is `OUT OF SCOPE` and why.

---

## 11. Small-Business Pools (§42) — **inserted**

**Where this came from.** Worklist 13e closed with *"PLACED: **small-business cells**, the covered
bond (M7), factoring and receivable pledges, and senior notes as repo collateral → 13f"*. **13f
closed and its PLACED list does not mention any of the five.** `docs/AUDIT.md` carried the fact in a
single row of finding `C-4` — *"Small-business pools (spec 42), carried from 13e and never built"* —
under item **18**, which was then marked DONE.

**§42 is 2 of 28 MET, and both are the generic kernel**: `A6` cites `parties/party.ts` and
`registry/profiles.ts` (cells exist), `E5` cites `audit/families/units.ts` and `world/cells.ts` (a
weight is a count). Nothing in the engine is a small firm. The one mention in 59,927 lines is a
comment at `firms/data.ts:124`.

**Inserted here** because both its dependencies are closed — trade credit (13e) for A4, bank lending
(13d) for A5 — and item 9 gives it the loan agreement B1 needs. It is takeable as soon as 9 lands.

**Why it matters beyond its own 28 clauses.** A5.a: small firms are bank-dependent, *"which makes
them the sector where a credit tightening bites first and hardest"*. Without them a credit tightening
has nowhere to bite: the model's only borrowers are named firms large enough to reach a bond market.
And §36 A4 — trade credit *"is the tier that lives on it"* — so 11 of Trade Credit's 22 clauses are
missing partly because the tier below them does not exist.

### What it is

`src/mechanisms/small-business/`, one `SystemModule`. **A pool with a distribution, never an
average** (A2, A2.a: *"Default is a threshold event; with one average firm a mean-preserving spread
causes no defaults, and the entire credit content of the sector is gone"*).

- **The sector** (A1): small firms are **firms** — they sell, employ, borrow and can fail. Not a new
  kind of thing; a firm with a weight.
- **Cells** (A6): each a named party with a **WEIGHT** — an integer count of the firms it is —
  carrying its own state, borrowing from a **named** lender on its own loan row, and selling to and
  buying from named counterparties on trade credit.
- **A6.a is the design rule and it decides the whole data model.** *"Every relationship that must be
  named is either a dimension of the cell's KEY or a REGISTER ROW, never an attribute averaged inside
  it."* Its **region** and its **bank** are dimensions of the key; its **lender** is a loan row per
  (lender, cell). *"A relationship averaged inside a cell is a relationship the model cannot name,
  and law 4 is broken quietly."*
- **A6.b**: a weight of **one** is a named firm — so the boundary between this sector and Corporate
  Credit's is **not a modelling line but a SIZE**.
- **A6.c**: a cell that **outgrows A5** — large enough to reach the bond market — is **promoted to a
  named firm**, and the promotion is an event with a cause. This is XI-15's `promotion`, one of the
  three weight events that **never fire today** (`A-17`). *"Without it the boundary is arbitrary and
  no firm can ever grow across it."*
- **Observable characteristics** (A3): size, sector, region, leverage, coverage — and **losses depend
  on the DISTRIBUTION of those, not the mean**.
- **Connected both ways** (A4): they employ people and buy from and sell to larger firms, **including
  on trade credit**.
- **The loans** (B1–B4): each a loan from a **named lender** with a rate, a term and an amortisation;
  often **secured** on the firm's assets or the owner's house; they **default**, and the default
  depends on the individual firm's cash flow aggregated over the pool; **defaults are correlated**
  (B4) — same rates, same demand, same region — so the pool's loss is **not the sum of independent
  draws**, which is what makes C's tranching meaningful or dangerous.
- **The pool as an instrument** (C1–C6): transferred into a **vehicle** — a named party holding them,
  funded by issuing claims; **tranched by seniority** with **stated** boundaries; each tranche has a
  **price that clears**; held by **named holders**, *"and that is where the loss actually lands"*;
  C4.a — *"often the originating bank keeps the bottom, which means the risk did not leave"*. This is
  the vehicle item 8 fixes, reused, not a second one.
- **Why it matters** (D1–D4): it moves credit risk from banks to investors; it **frees bank capital**,
  so securitisation is a **lending-capacity mechanism**; the senior tranche is **collateral**; and
  when B4's correlation is worse than the tranching assumed the **senior takes losses it was not
  supposed to** — D4.a, **emergent from B4 and C2, never a scripted event**.

### What must not happen (E1–E6 — each is a test)

- **E1** no pool without underlying loans to named borrowers. *"Tranching a loss rate yields senior
  notes that can never be touched."*
- **E2** no tranche without a holder. **E3** no risk transfer without a transferee.
- **E4** **no constant pool population.** *"If entry is the accounting identity of exit, the
  population is constant by construction and nothing the sector experiences can change it."*
- **E5** no weight that is not a count — five events only, each with a cause and a date (XI-15).
- **E6** **no loss allocated to a pool rather than to its cells.** *"A loss struck against the pool
  and spread back over its members has been evaluated at an average, which is A2.a one level up."*

### Steps

- [ ] 11.1 The module skeleton: `requires: ['firms', 'banks', 'trade-credit']`. Kinds, profiles, units, params, phases anchored at `corporateActions | markets | revaluation`, participants per party kind, audit contributions, a seed contribution. Every clause cited with `@spec`.
- [ ] 11.2 The party kind: a firm cell. `representation: 'cell'`, `fails: ['cash','solvency']`, `borrows: true`, an `objective` (item 15 of the old file requires one), and a key whose dimensions are **region** and **bank** (A6.a) — never its lender, which is a register row.
- [ ] 11.3 Per-member state: what it holds, what it owes, its cash flow, its leverage and its coverage. **`integrate(f)` and never a mean** (XI-15). A partial event splits the cell.
- [ ] 11.4 The seed draw: a distribution of sizes, sectors and regions — **A3's distribution is the point**, so the draw must produce dispersion and the test must show a mean-preserving spread changing the count of defaults (A2.a). No representative small firm.
- [ ] 11.5 They sell and buy (A4): wire them into the goods markets as both sides, and into `trade-credit`'s `termsOffered` as buyers — which is where **A-63**'s "a household gets thirty days on its bread" was fixed by asking what kind of party the buyer is; a small firm is the party that SHOULD get terms.
- [ ] 11.6 They employ (A4): a `venueParticipant` into the labour venue — which item 3 made reachable.
- [ ] 11.7 The loans (B1, B2): a row per (lender, cell) through item 9's agreement, with a rate the bank **quoted** (Banks Lending C3.a — one rate per loan, what was quoted), a term, an amortisation, and security on the firm's assets.
- [ ] 11.8 Default (B3): from the individual cell's cash flow, aggregated over the pool — a **threshold event per cell**, never a hazard rate (Appendix B).
- [ ] 11.9 Correlation (B4, B4.a): the same rates, the same demand and the same region reach every cell in a region, so the correlation is **emergent from shared causes** and is not a parameter.
- [ ] 11.10 **A6.c, the promotion.** A cell whose weight is one and whose size clears A5's bond-market threshold becomes a named firm. This fires `cells.weight` — one of the three XI-15 events with no caller — and is half of `A-17`.
- [ ] 11.11 The pool into item 8's vehicle: C1–C5 reuse the securitisation module. Do not write a second waterfall (Law 4).
- [ ] 11.12 The six FORBIDs as six tests, E4 and E6 especially — E4 needs births (item 12) to be non-vacuous, so write it here and expect it red until 12 lands, and **say so in the record** rather than deleting it.
- [ ] 11.13 COVERAGE re-marked for all 28 clauses; record entry; `check:existence` shows §42 is no longer an absent sector.

### Exit

A credit tightening reaches a small firm before it reaches a large one; a mean-preserving spread over
the pool changes the count of defaults; a cell is promoted to a named firm and the boundary between
the two sectors is a size rather than a modelling line.

---

## 12. Firm birth (worklist 13n)

**Why.** `world/cells.ts` implements XI-15's five weight events. The callers are:

| door | callers |
|---|---|
| `cells.split` | `labour/matching.ts:312`, `labour/matching.ts:405`, `households/lifecycle.ts:250` |
| `cells.reKey` | `households/lifecycle.ts` (ageing) |
| `cells.die` | `households/lifecycle.ts` (after probate) |
| `cells.merge` | **none** |
| `cells.weight` | **none** |

So **entry never happens: no person is ever born in this world**, and no firm either. `age()` moves
members from cohort 0 into cohort 1 and nothing moves into cohort 0; `die()` removes them at the top.
The population is monotonically non-increasing from the seed, by construction and not as an outcome.
`estate` kills firms and nothing creates one.

Appendix B forbids a *declared birth rate*, which is right. What is absent is **the mechanism that
would produce births as an outcome** — a household's own decision, with its own cause — and nothing
naming it as absent. Under Part II that makes it neither MISSING nor OUT OF SCOPE but **unstated**,
which is the one thing a clause may not be.

**Entry is what makes a market contestable.** Without it a survivor's margin is never competed away
and every concentration measure is one-way.

**Unblocked by** item 7 of the old file (`Lifecycle` — the states) and item 15 (`Objective` — a party
declares what it is FOR, which is what somebody starting a firm needs a reason from). Spec 34 is half
built and **the birth half was PLACED forward three times (13f → 13g → 13h) without landing** — the
worklist row says so itself.

### Steps

- [ ] 12.1 A firm is started by somebody, out of something. The founder is a named party with a reason (`Objective`) and a balance sheet the equity cheque comes out of. **No firm appears from nowhere** — that would be a residual with no holder in reverse.
- [ ] 12.2 `cells.weight` fires for entry with a cause and a date. This is the first caller that door has had.
- [ ] 12.3 Household formation is the same mechanism at the other end: nothing moves into cohort 0. Give it a cause.
- [ ] 12.4 `cells.merge` has no caller and is the other half of `A-18`: nothing recombines two cells that have become identical, so the cell count only ever rises. With 2.14's remainder fixed the micro-cells stop being created; merge is what removes the ones already there. **A-2 and A-3 guard `merge` and have never run** — closed as repairs, and this is where they first execute.
- [ ] 12.5 Firm death already works (`estate`). Assert the pair: over a long run, entries and exits are both non-zero and neither is the accounting identity of the other (§42 E4).

### Findings this closes

`A-17`, and `A-18`'s merge half.

### Exit

A firm is born in an ordinary run, with a founder and a reason; the population is an outcome in both
directions; `cells.merge` and `cells.weight` have callers.

---

## 13. Asset managers: hedge funds (§28), private equity (§29), prime brokerage (§15)

**Why.** Three systems, **0 / 24, 0 / 25 and 0 / 24 MET**. Worklist 13o, open, and correctly blocked:
*"item 8 built `Agreement` and did NOT migrate the seven private books, so `Mandate` does not exist
yet. Building this on today's `fund` party would bake the pool/decider conflation in permanently."*
Item 9 unblocks it.

**Measured**: 13 funds, of which **4 hold anything at all** after four periods — 8 holdings between
them against 1,274 subscriptions settled in one period. Every fund here is a money fund, one
commodity fund, or an index tracker, and the trackers hold nothing.

**§28 C1 is why this is not an optional sector**: a hedge fund *"is the natural home of the
speculative side of every derivative book"*. Item 6 gives each book a maker with its own number; this
gives it a party whose whole reason is to hold the other side.

### Hedge funds (§28)

- **A1–A5**: a named party with investors, a register and accounts; **investor capital is equity** and
  investors hold a redeemable share count; a **manager** is a separate party earning a management fee
  on assets and a **performance fee on gains**, *"and the asymmetry of that second fee is a reason for
  risk-taking"*; a **wide mandate** — long, short, levered, many markets; everything **marked at
  cleared prices**.
- **B1–B5**: it borrows from a **named lender** — *"leverage is a fact about a loan, never a property
  of the fund"*; it levers through **derivatives** (notional over margin) and through **repo**; the
  amount available is the **lender's decision** and it changes. B5: gross, net and equity are **three
  reads** and a single "leverage" number hides which one moved.
- **C1–C4**: positions for reasons — relative value, direction, a liquidity premium it is paid to
  hold; it is the **buyer when others are forced sellers, if it has capacity**; it **shorts**, which
  requires a borrow (item 9.4); real trades at cleared prices.
- **D1–D7, the failure mode, and it must be emergent**: a loss reduces equity → with fixed borrowing
  leverage rises → the lender calls margin → meeting the call requires selling → **which moves
  prices** → the move hits other levered holders and D1 starts again for them. **D4.a: never a
  contagion parameter.** D5: redemptions arrive at the same time for the same reason and are a second
  forced-seller channel; D5.a a **gate or notice period** is a real contractual term with real
  consequences for who gets out.
- **E1–E3**: no leverage without a lender; no position that does not mark; **no fund that cannot
  fail** — *"a vehicle that absorbs losses indefinitely is the buyer of last resort in a different
  costume"*.

### Private equity (§29)

- **A1–A5**: committed capital from named investors; **capital is committed, not paid** — it is
  **called** when a deal needs it, *"and the call is a real payment from the investor's account on a
  date it cannot refuse"*. A2.a: an investor must hold liquidity against calls it did not choose the
  timing of. **A2.b is a FORBID and it is the one to get right**: *"a call bounded by the investor's
  spare cash is not an obligation"* — the investor funds it from its own liquidity ladder, selling if
  it must, **or it defaults on the call**, which is itself an event with consequences. A manager on
  committed capital plus carry; the fund has a **life** and **winds up** (item 14's `Process`);
  acquired firms are held in **named vehicles**.
- **B1–B5, the buyout**: a price agreed with the sellers; **most of it debt raised against the target
  itself** — B2.a the debt is the **target's** liability, *"which is why a failed buyout kills the
  firm and not the fund"*; B2.b **the credit market decides which buyouts occur**, a real constraint
  and not a rate applied to a plan. B5: **sources and uses must balance exactly** and the money must
  come out of named accounts.
- **C1–C5, the hold**: the firm services its debt out of cash flow with less room; the owner
  influences investment, costs and distributions; it can **recapitalise** — *"a real transfer from the
  firm's future to the owner's present"*; it can **fail**; and the holding has **a value that is not a
  market price** — C5.a, *"an unlisted mark is not a cleared price … the honest answer is 'marked, not
  cleared'"*.
- **D1–D5, the exit**: it sells, and **the exit produces the first real price the holding has had**;
  proceeds distributed in cash; **the exit depends on the market being open** — in a bad market the
  hold extends and the distributions do not arrive, which feeds back to A2.a.
- **E1–E3**: no buyout without a lender who agreed; no capital call not paid from a real balance; no
  exit at a price nobody paid.

### Prime brokerage (§15)

- **A1–A4**: a named bank and a named client with a contract that can be ended; the broker **holds the
  client's assets and knows the whole position** — that knowledge is what lets it lend; **A3: the
  client can have more than one broker, and then no broker sees the whole position**, a real and
  material blind spot; the broker earns financing spread, stock-borrow fees and commissions.
- **B1–B5**: the broker lends the difference against the assets; **the client's leverage is a loan
  from a named lender, not a property of the client**; the loan has a rate above the broker's own cost
  of funds; **the broker's balance sheet grows by the loan** and it consumes capital and liquidity;
  the short side is financed too.
- **C1–C5, margin, and the spec calls it the core**: the broker sets a requirement **on the whole
  portfolio from its own view of the risk**, accounting for **offsetting positions** — *"a decision by
  the broker, not a formula the client can rely on"*; remeasured as prices move; a shortfall is a
  **call: real money, now**; meet it or be liquidated, and to meet it the client may have to **sell
  into a market that must clear**. **C3.b: the available line is never floored at zero** — *"a client
  drawn past its line is over the line, and the shortfall is what forces the sale. Flooring it makes
  the whole path unreachable — and lending the shortfall straight back at a penalty, from the same
  broker, makes it unreachable twice."* C4.a: **raising margin into a falling market amplifies the
  fall**, *"the mechanism behind most of what looks like contagion"* — and a stated constant margin
  rate deletes exactly that. C5: no margin that is only a number.
- **D1–D4**: the client fails a call → the broker closes the positions, **selling collateral at market
  prices** → proceeds may be less than the loan and the shortfall hits the broker's capital → the
  liquidation **moves prices, which can margin-call other clients**. D4: the chain must be traceable
  party by party; *"a loss that stops at the fund is a broker that was never really lending"*.
- **E1–E4**: exposure per client, known; concentration means collateral is worth less in liquidation
  than marked; A3's multi-broker case means **each broker underestimates**; **no unlimited exposure**.

### Steps

- [ ] 13.1 `Mandate` (item 9.2) is the spine: a hedge fund is **a mandate with leverage**, a separate account is a mandate whose pool is the client's own balance sheet, an ETF and an MMF are pools with different redemption rules. Build the three sectors on it and nothing else.
- [ ] 13.2 Hedge funds: the party, the manager, the two fees, the wide mandate. `borrows` comes off the mandate, not off `fundKind` where it is hard-coded `false`.
- [ ] 13.3 Prime brokerage: the relationship as an `Agreement`; portfolio margin as the broker's own decision (C1.b) held as a `View`; **no floor on the line** (C3.b).
- [ ] 13.4 The loop D1→D4 must fall out of the parts. Do not write a contagion step. Test: one fund's loss reaches another fund's margin call through prices and named counterparties, and the path is traceable.
- [ ] 13.5 Private equity: committed capital, the call as an obligation the investor cannot bound by its spare cash (A2.b), the buyout with debt on the target, the mark that is not a price, the exit that produces the first cleared price. Needs item 9 of the old file (`Control`), which is built.
- [ ] 13.6 Wire hedge funds into every derivative book as the speculative side (§28 C1) — the other half of item 6.
- [ ] 13.7 COVERAGE re-marked for all 73 clauses across the three; `check:existence` shows three fewer absent sectors.

### Findings this closes

`B-14` (a fund holding contracts on purpose is `Fund Shares A3`, answered by a mandate).

### Exit

A hedge fund takes the speculative side of a derivative book; a margin call forces a sale that moves a
price that calls margin on somebody else; a buyout puts debt on a target and the credit market decides
whether it happens.

---

## 14. Insurers and pensions (§27)

**Why.** **9 of 23 MET, all nine never reached.** Item 9 fixes the five blockers (`A-9`) and gives the
sector a seed (`B-2`). What is left is the half `docs/AUDIT.md` named and never placed:

> *"**Pensions are insurers wearing the same name.** Spec 27 is 'INSURERS AND PENSIONS' and there is
> one party kind, `insurance`. A pension has a SPONSOR, contributions from an employer and its
> members, and a funding ratio that is the sponsor's problem when it falls."*

**The model's largest holder of duration does not exist**, which is why nothing in this world is a
natural buyer of a long bond.

### Steps

- [ ] 14.1 Split the kind: an insurer writes policies against premiums; a pension has a **sponsor**, contributions from an employer **and** its members, and a **funding ratio that is the sponsor's problem when it falls**. Two profiles behind one dispatch table, never a kind branch.
- [ ] 14.2 A policy is an **agreement** (item 9), not an instrument kind. That is `A-9`'s root: *"an insurance policy is modelled as an instrument kind — a tradeable security — which is why the insurers module has no seed, no phase and no participant: nobody knows how to WRITE one, because writing an agreement is not something the kernel does."*
- [ ] 14.3 The insurer buys duration: it is the buyer item 10 and 10b need on the other side of a long issue.
- [ ] 14.4 COVERAGE re-marked; the 14 MISSING clauses built or stated `OUT OF SCOPE` with a reason.

### Findings this closes

The remainder of `A-9` and `B-2` after item 9.

### Exit

A policy is written, a premium is paid, a claim is met; a pension's funding ratio falls and its
sponsor has a problem; somebody in this world wants a thirty-year bond.

---

## 15. Housing, the rest (worklist 13m)

**Why.** **6 of 26 MET.** Item 3 opens the lettings venue, item 7 gives the dwelling a buyer, item 12
of the old file built `Space` (the ground of a place, finite, held and cleared — 113,281,635 hectares
in `us.1`, 16,502 traded, eight holders). What stands on it is not built.

`E-5`: **the state holds the ground of every place in its country and can only SELL in the one it sits
in**, because a seller in another place reads as a cross-border trade in the balance of payments. What
is missing is a party **PRESENT in each place** to sell its ground — a local authority, which is the
same noun a port and a planning consent need.

### Steps

- [ ] 15.1 The local authority: a party present in each place, objective `itsOffice` (which the land market already dispatches on, so this lands with no change to that file).
- [ ] 15.2 A port with an owner and a berth, and congestion as an outcome of the berth.
- [ ] 15.3 Commercial property: buildings as assets, leases with a term, CRE lending. *"Warehouses exist only as `STORAGE` plant a firm builds for its own stock — nobody builds space to LET, so there is no landlord, no commercial rent, no lease with a term and no CRE lending."*
- [ ] 15.4 A retail firm that sells from somewhere.
- [ ] 15.5 **A-53**: `housing/index.ts:reservation` bids a household's **entire expected income** as rent, as a **single point**, and `letIn` clears on `marginalBid` — so in any region where dwellings are short the rent takes a household's whole livelihood every period. Two things do not follow from B1.a: nothing on the household's side knows rent exists (`spendPerMember` spreads the same expected income over the basket, so it is committed twice), and every other bid in this world is a **schedule** (Clearing A2). Post a curve, and give `households` a rent term.

### Findings this closes

`A-53`, `E-5`.

### Exit

A tenancy is signed and rent is paid; a landlord exists; a household's budget knows what it pays for
its roof.

---

## 16. Cross-border, the rest

**Why.** **6 of 26 MET.** 13i closed with the EXTERNAL ACCOUNTS built and the rest PLACED — *"sourcing
across regions, foreign-currency issuance and the swap line (**M3**, **M6**) stay carried"*, and
nothing has carried them since.

**B-9 is unsettled and must be settled first.** `docs/BUGS.md` claimed *"13j gave this world four
economies"* in its header and *"the foreign countries are stubs — three of the four are a central
bank, a treasury and a bond line"* in its own section 2. Both cannot be current. `A-61` (every central
bank remitting to `treasuries[0]`) was live evidence for the pessimistic reading and is fixed in item
2.13; measure which is true before building on either.

### Steps

- [ ] 16.1 Settle B-9: run the four-country rig and count what each country actually has. Write the answer here.
- [ ] 16.2 Sourcing across regions: a buyer in one place buying from a seller in another, with freight and the balance of payments both seeing it.
- [ ] 16.3 Foreign-currency issuance (**M6**): an issuer raising money it does not print. This is where **A-36** becomes live — `fails: []` on the treasury is an unconditional exception where the kernel grants a conditional one, and Appendix B names *"sovereign in foreign money"* among the things that must not be immortal. `failedWhy` already has `ccy` in scope and discards it on the solvency branch.
- [ ] 16.4 The central-bank swap line (**M3**).
- [ ] 16.5 The 18 MISSING clauses built or stated `OUT OF SCOPE` with a reason.

### Findings this closes

`A-36`, `B-9`.

### Exit

A firm sources from another country; an issuer raises in a money it does not print and can fail in it;
the swap line exists.

---

## 17. Corporate credit, the rest

**Why.** **7 of 62 MET — 51 MISSING, the largest single gap in the model.** Item 10 gives it the one
thing it has never had: an issued corporate bond. What is behind that is the whole of §7 —
syndication, bookbuilding, facilities, restructuring, the covered bond (**M7**, one of the six lost
things), and the index-linked obligation and other schedule shapes (**M2**, placed to 13f and never
built).

**Take this item's 51 clauses in the spec's own order** and mark each `MET`, `PARTIAL` with what is
missing, or `OUT OF SCOPE` with a reason. Do not delete a clause to look better.

### Steps

- [ ] 17.1 Syndication and bookbuilding: a bank arranging an issue it does not hold all of.
- [ ] 17.2 Facilities: a committed line, with a commitment fee on undrawn headroom (the same noun item 10b.6 needs — build it once).
- [ ] 17.3 Restructuring: placed 13f → 13h, never built. A borrower and its lenders agreeing new terms is an `Agreement` transition, not a new instrument.
- [ ] 17.4 The covered bond (**M7**): placed 13e → 13f, never built. One of the six.
- [ ] 17.5 Factoring and receivable pledges: placed 13e → 13f, never built. Two more of the six, and they sit on item 11's small firms, which is why this is after 11.
- [ ] 17.6 Senior notes as repo collateral: the last of the six. `money-market/collateral.ts` already has the haircut machinery.
- [ ] 17.7 The index-linked obligation and the other schedule shapes (**M2**).

### Exit

§7 is 62 of 62 answered — MET, PARTIAL with what is missing named, or OUT OF SCOPE with a reason —
and all six of the lost things from 0.3 have a home and a state.

---

## 18. Commodities spot and futures

**Why.** **Commodities Spot 3 / 24, Commodity Futures 5 / 20.** 35 missing clauses across two systems
that 13c closed. The futures half needs item 6 (a book that can open); the spot half is independent.

### Steps

- [ ] 18.1 Read §20 and §21 clause by clause against `mechanisms/commodities/` and `mechanisms/commodity-futures/` and write down which of the 35 are genuinely absent and which are MET-and-unmarked. **This is a measurement of the claim, not of the world** — do it with the item 1 tool, not by reading.
- [ ] 18.2 Build what is absent, in the spec's order.
- [ ] 18.3 `A-64`'s storage rent is closed (item 11 of the old file) and the leases store is gone; confirm nothing reintroduced it.

### Exit

§20 and §21 are answered clause by clause.

---

## 19. The polity (§47) — worklist 14

**Why.** **0 of 32 MET.** The plan below is item 14's own, carried unchanged from `docs/AUDIT.md`
Part IV, because it was written against the spec and nothing since has changed it. It carries three
findings: **D-1** (arrears), **D-2** (the central bank as a marginal price-setter), and **E-7** (a
negative policy rate is real and this world cannot express one).

### 19.1 Kernel: owners and the mandate door

Every `policy` parameter already declares an `owner` (`parliament | centralBank | standardSetter |
constitution`). This makes the owner **load-bearing**: a policy owned by `parliament` can be written
only through `params.setByMandate(values, effective: Period)` on the `MechanismContext` of the module
that declares itself the polity (assembly grants the door to exactly one module: D5, C3.a); every
other write path throws `Forbidden`; the seed's standing mandate is the initial value with
`setBy: 'seed.standingMandate'` recorded (XI-17); the observer prints the owner and the setter beside
every policy value (D5).

The central bank's **rate** stays the central bank's (D4, Central Bank A4); its **target** and mandate
text are parliament-owned (D4, Central Bank A3): `centralBank.target.*` moves to owner `parliament`.

### Module `polity`

- `requires: ['households', 'treasury', 'expectations', 'labour']`.
- **The constitution** (A1, A4, C1): three policy primitives with owner `constitution` —
  `polity.seats` (a count), `polity.termPeriods` (placed on the calendar by date), and
  `polity.allotmentRule` (a named rule in a dispatch table: largest remainder, or highest averages).
- **Platforms** (A2): `registry/platforms.ts`, one row per party with a value for **every**
  parliament-owned policy (assembly throws if a platform misses one or names one the parliament does
  not own); platforms differ (A2.a: assembly refuses two identical rows); a party is not a ledger
  party (A2.b: no account, a name with a seat count).
- **The vote** (A3, B): phase `polity.election` on its date. For each household cell the module
  evaluates every platform **applied to that cell's own state at that cell's own outlook** (B1, B2):
  its expected income under that platform's tax and transfer rates, its expected prices paid at its
  own basket, what it owns and owes. It never reads a published unemployment or inflation rate
  (B1.a) — structurally, through a `withoutAggregates` view variant of the kind item 12 built for
  `withoutPrints`. It picks the platform with the highest expected position; a cell for which every
  platform gives the same position **abstains** (B2.b, the only abstention; B2.a: no turnout, swing
  or drawn share). The cell casts `weight` votes through `integrate` (A3, B3). Turnout is a read.
- **Seats and government** (C): seats by the allotment rule (C1); the government by the coalition
  rule (C2 — the largest party adds the party whose platform is nearest its own, distance over the
  policy vector in each policy's own unit, until it holds a majority); a **hung parliament** when no
  such coalition exists within a stated distance continues the standing mandate and is reported (C2.a).
- **The mandate** (C3, C4): the seat-weighted position of the coalition's platforms per policy; a
  journalled `polity.mandate` with date, parties, seats and what changed (C4, E4 — an event that
  causes nothing itself); written through the door with `effective = election + polity.mandateLag`.
  C3.b is an audit contribution: every parliament-owned policy's value equals the standing mandate's,
  every period, exactly.
- **What it controls** (D): tax rates on named bases, transfer rates, the treasury's buffer, the
  outlay programme's size and composition, regulatory ratios and floors, the central bank's target.
  D3.a: no price, quantity or outcome is a parliament-owned policy — assembly throws if a platform
  names a parameter whose kind is not `policy` or whose owner is not `parliament`.
- **Consequences** (E): E1 the treasury programme reads the register; E2 the chain runs through the
  mechanisms; E3 measured at item 23; F4 the **approval rating** is the read "how would the cells vote
  today", computed by the observer with a lag and read by nothing.

**Parameters.** `polity.seats`, `polity.termPeriods`, `polity.allotmentRule`,
`polity.coalitionMaxDistance`, `polity.mandateLag` (policy, owner `constitution`); `platforms` (data).
No `turnout`, no `swing`, no `loyalty`, no `bloc`, no `approval` input.

**Audit contributions.** `names`/`flows`: C3.b — register values equal the standing mandate (a check
of one writer). `units`: votes cast + abstentions = Σ weights of the electorate, exactly (F5).
**Write this one against `A-14`**: a votes-plus-abstentions identity built from one loop with two
accumulators would be the third of its kind in this engine. The two records must be independent — the
ballots the vote produced, against the electorate the parties store holds.

### Files

```
packages/engine/src/registry/params.ts (19.1), registry/platforms.ts
packages/engine/src/world/context.ts (the door granted to one module), world/assemble.ts
packages/engine/src/mechanisms/polity/{index.ts,vote.ts,seats.ts,coalition.ts,mandate.ts,approval.ts}
packages/engine/test/{param-owners,platforms,vote,abstention,seats,coalition,hung,mandate,
                      mandate-writer,spread-changes-seats,election-chain}.test.ts
```

### Steps

- [ ] 19.1 Kernel: policy owners load-bearing; parliament-owned policies writable only by the mandate door granted to one module; the seed's standing mandate recorded as the setter; owner and setter printed beside every value; the central bank's target moved to the parliament; tests (D4, D5, C3.a)
- [ ] 19.2 The constitution's primitives: seats, term on the calendar, allotment rule from a dispatch table, coalition distance, mandate lag; tests (A1, A4, C1)
- [ ] 19.3 Platforms as data rows covering every parliament-owned policy and nothing else, differing; a party holds no account; assembly validation; tests (A2, A2.a, A2.b, D3.a)
- [ ] 19.4 The vote: each cell applies each platform to its own state at its own outlook through a view with no aggregate reads; votes weight for the best; abstains when indifferent; turnout as a read; tests (A3, B1, B1.a, B2, B2.a, B2.b, B3)
- [ ] 19.5 Seats by the rule; the government by the coalition rule; a hung parliament continues the standing mandate and is reported; tests (C1, C2, C2.a)
- [ ] 19.6 The mandate as the seat-weighted coalition platform, journalled with subjects, written through the door at the lag; C3.b contribution; tests (C3, C3.b, C4, E4)
- [ ] 19.7 What it controls: fiscal rates, transfers, buffer, outlay programme, regulatory ratios, the target; never the rate, a price, a quantity or an outcome; tests (D1–D4, D3.a)
- [ ] 19.8 **Arrears (`D-1`)**: a levy that fails becomes a claim the treasury holds on the payer — an instrument with two named sides, carried until paid, written off or ranked in an estate at the place the law states; the receipt is short by it in the accounts and not only in the journal. **Build it as one door for all four cases**: the unpaid tax, the unpaid wage and severance (`A-41`), the unpaid estate transfer (`A-20`), and the horizon `stillOwed` reads (`A-39`); tests (Money E1, D3; XI-8; Polity D1, D3). Note: item 8 of the old file built the store and wired all four — what remains here is `breached`, which has **no writer**: a levy in arrears stays `performing` because nothing has decided when an arrear becomes a breach, and that is fiscal policy with an owner.
- [ ] 19.9 **`D-2`**: the open-market desk closes a gap towards a 25%-of-line target by posting a **MARKET order** — a quantity with no level — so in every sovereign session where it has a gap it bids at the top of the book for a quarter of the line. An order with no level is a price-taker of a price the mechanism has not yet produced (Clearing A4), and a big enough one is the marginal order that sets it. The quantity limit is real policy (Central Bank C1) so this is not Appendix B's buyer of last resort — but it is more aggressive than any real open-market operation. **It posts a SCHEDULE**: the level at which its own reason stops. Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an unchanged book.
- [ ] 19.10 **`E-7`**: a policy rate of zero puts the corridor floor at minus a tenth of a point and the solver refuses a negative price (`Impossible [Law 6] a price cannot be negative: -0.001`). **That is right for the price of a THING and wrong for the price of TIME**, which the Bank of Japan and the ECB both ran below zero for years. A negative policy rate is real and this world cannot express one. Fix it here, where the rate has an owner, rather than rounding the yen up.
- [ ] 19.11 Consequences through the mechanisms only: the treasury programme reads the new numbers; a multi-year scenario shows the deficit changing through named outlays and receipts after an election; tests (E1–E3)
- [ ] 19.12 Approval rating as a lagged observer read that nothing reads; tests (F4)
- [ ] 19.13 B4: two seeds with equal weighted-mean income and different dispersion give different seat counts; F5: seats and mandate computed from members, with the two records independent; tests
- [ ] 19.14 Observer: parliament, platforms, votes by cohort and region, the mandate with owner beside every policy; a multi-year run with two elections green; determinism; coverage re-marked; record entry
- [ ] 19.15 Worklist row 14 → done

### Findings this closes

`D-1` (the `breached` writer), `D-2`, `E-7`.

### Exit

No parliament-owned policy can be set by anything but a mandate; a cell votes from what it experienced
and can abstain; a hung parliament is a reported outcome; a change of government reaches the deficit
only through the treasury's programme.

### Guard

Polity B1.a, B2.a, C3.a, D3.a, F1–F4; Central Bank A4; XI-17 (*"no policy set directly"*).

---

## 20. Periodicity: the fee and the assessment (worklist 13k)

**Why.** Item 10 of the old file fixed the dividend — a board declares on its own fiscal quarters with
an ex date, a record date and a payable date, and **dividend payout legs fell from 8,538 of 61,788
instructions to 308 of 53,266**. Three of `C-2`'s four are still weekly.

- **A rating fee is charged every week.** 16,527 `pays X for its rating` instructions in period 5;
  `ratings.assess` is `cycle: 'anchor'` and calls `collectFees` every period. An issuer pays an
  **issue fee once** and a **surveillance fee annually**. Weekly billing makes the assessor's income a
  flow of the issuer's equity rather than a price for a service, which is the conflict Ratings A5
  exists to keep.
- **Tax is levied every week.** 7,778 `tax due from X` instructions in period 5. Payroll withholding
  is weekly and right; **corporation tax is assessed on a fiscal period** and VAT quarterly. Levying
  everything weekly removes the working-capital consequence of a tax bill, which is the thing a
  treasury and a firm both plan around.
- **A buyback is decided weekly** out of this week's spare cash. A buyback is an announced
  **PROGRAMME** executed over time, and announcing it is the event. Its kind is already in
  `register/corporate.ts` and item 14 of the old file built `Process`; this is one `beginProcess`
  away.

`reporting/fiscal.ts` and `core/rate.ts`'s `Periodicity` union are built. **This item is after 19**
because who sets a fiscal period is fiscal policy with an owner.

### Steps

- [ ] 20.1 The rating fee: an issue fee once, a surveillance fee annually.
- [ ] 20.2 The tax assessment: withheld weekly on the wage, **ASSESSED** on a fiscal period.
- [ ] 20.3 The buyback as a `Process`.

### Findings this closes

`C-2`.

### Exit

Nothing happens weekly that does not happen weekly in the world.

---

## 21. The local repairs

**Why.** Three findings with no type-level cause: plain bugs and a stale read. The item's own rule is
**never a session's work; take each when its file is already open**. Fifteen of the original eighteen
are closed.

### Steps

- [ ] 21.1 **A-24**. `households/decide` publishes its orders into a journal event as `Record<string, unknown>`; `marketsIn`/`ordersFrom` re-validate from scratch (`if (price !== 'market' && typeof price !== 'number') continue;`) and **silently drop** what they cannot parse. A cell whose plan wrote a row this pair of predicates rejects simply does not trade that period, with no throw, no violation and no event — a decision disappearing between the party that took it and the book it was for, which is the one thing the round trip exists to prevent. Apply item 3 of the old file's answer — a typed read that **throws** on a record its own writer malformed. `ordersFrom` also drops `qty <= 0` before `asQty` can complain, so the file's own comment about throwing is only true for positive non-integers.
- [ ] 21.2 **A-43** — closed in item 9.6; confirm `labour/matching.ts:supply` is gone.
- [ ] 21.3 **A-53** — closed in item 15.5; confirm the rent bid is a schedule and `households` has a rent term.
- [ ] 21.4 **E-6**, found while building item 17 of the old file: **an acquirer's consideration in a bank resolution is a missing mechanism.** `money-market/resolution.ts` ranks bidders by what the book is worth to each — faithfully, and `A-59` is closed by publishing `worthToIt`, which is what ranks them. What an acquirer should be **PAID** for taking the book on does not exist: the guarantee pays exactly `v.hole`, so the acquirer ends whole in balance-sheet terms and earns nothing at all for taking on the book, **while the auction ranked bidders precisely by how much they wanted for doing so**. Build the consideration.
- [ ] 21.5 **E-3**, found while building item 10 of the old file: an estate pays RENT for the space its inventory sits in while it winds up, and its own `flows` family reports every non-`corporateAction` payment. Decide which is wrong and fix that one.
- [ ] 21.6 **E-2**, found while building item 8 of the old file: an estate left `winding` because it could not hand over **is still a household cell to the eleven `ofKind(HOUSEHOLD)` readers**, which all ask `status.alive` — so a cell of dead people goes on consuming and looking for work. Item 14 of the old file built `Process` and this is not built on it yet.
- [ ] 21.7 **E-4**, found while building item 11 of the old file: a capacity line still produces into WIP and destroys the unsold part. The number is right and the **accounts line** is not — an inventory write-off where it should be operating leverage on fixed cost. The fix is **produce-to-order**: a capacity line posting availability into the session and making only what clears. It could not be done in item 11 because `firms.produce` anchors after `labour.pay` and therefore **before** `markets`, so a line cannot know its demand when it produces — and reordering for one kind of line would be a kind branch. It needs `Process`, which exists.
- [ ] 21.8 **A stale placeholder of the old file's own making.** `estate/index.ts:598` still declares `standsInFor: { noun: 'Process' }` — but old item 14 BUILT `Process`, deleted `Winding`, and pointed the settle loop at `ctx.processes.running('estate')`. The store is migrated and the declaration was not removed with it. Law 16: a stale declaration is a defect, and this one makes `registry/nouns.ts`'s count of homeless nouns wrong — which is *"the honest measure of how much ontology is missing"*. Delete it and recount.
- [ ] 21.9 **A-69**'s dead reads: `struckRate` and `findSession` (`money-market/session.ts`), `cdsBookOrders` (`cds/participants.ts`). Each carries a docstring asserting something is measured or dispatched and is called by nobody. Call it or delete it.

### Findings this closes

`A-24`, `E-2`, `E-3`, `E-4`, `E-6`, and `A-69`'s remainder.

---

## 22. The recipe (worklist 15)

Unchanged, and last before measurement, deliberately. Goods A2.

---

## 23. Measure — Part XII in full (worklist 16)

**Why.** Part XII is the measurement pass, and it comes **only after** everything above (Law 11: do
not measure mid-build). It carries `C-1`'s 82 red tests, which are not bugs — they are **tests that
named the world they were written against.**

| cause | ≈ red | what it is |
|---|---|---|
| **the rig has no firm in most lines** | 20 | `RIG_PER_LINE` is 0, so twelve firms over sixty-two lines leave most empty and any test asking the draw for a mill is told "it drew 0". Measured: setting it to ONE takes the suite from 79 red to **139** — a rig with every line in it is a different small world, and a dozen files' assertions are written against the one it makes today. Those tests are not wrong, they are **SPECIFIC**. |
| **the foreign countries are stubs** | 6 | see `B-9`; settled at item 16.1 |
| **a bank that is insolvent is never resolved** | 9 | `13b-12`: a bank published capital of −31,237,415,456 and went on making a market for twenty-three periods. Both triggers must exist and the resolution must name which fired (Banks Capital C1.a). Whether the solvency trigger is not reading the published position, or is reading it and the resolution is not being run, is **still unmeasured** — and it has its own diagnosis, **before** this item |
| **an ETF cannot create** | 7 | `12d-8`; answered at item 9.8 |
| **two banks, and everything that needs a third** | 4 | `research` wants coverage to VARY, `deposits` wants a class to split rather than cross, `dealing` wants a market to fail when the desks step back. With three banks and one listed line a count that should be an outcome has one value |
| **singletons** | 9 | `omo` remittance and run-off (`A-61`, `A-62`); `raise`; `ratings` ageing; `equity-anchor`; `indices`; `tick`; `environment`'s crop; `treasury`'s receipts (`A-46`); `world`'s year-long chain. Each needs reading on its own |

**And the eight 13j cost.** Three measurements of one suite: 74 red before 13j; **303** with the rig
opening all four countries; **82** with the rig opening one and the currency tests four. The 303 is a
finding and not a bug — a dozen firms and three banks over four countries gives each a country with no
banking system its own depositors could fund, and `Seed D1` refuses exactly that ninety-four times —
so **a world's count of countries is a RESOLUTION like its count of banks.**

### Steps

- [ ] 23.1 Resize the scale model as **one bounded change** and re-derive what every affected test asserts. A test never names a party (`packages/engine/test/rig.ts`): it asks the draw for a mill, a dealer, a listed line.
- [ ] 23.2 Diagnose "a bank that is insolvent is never resolved" **before** this item, on its own. It is the one row of `C-1` that is a live defect rather than a specific test.
- [ ] 23.3 Part XII's measurements, in full, with the level carried from eight measurements of it (`12-15`, `12d-1`, `12d-6`, `12d-7`, `12d-9`, `12b.1-2`, `12a-2`) and the representation (`12d-16`, `12c.1-1`).

### Findings this closes

`C-1`.

---

## 24. The app and the APK (worklist 17)

Unchanged. The reserved decision (§45 A4, inspector vs participant surface) taken with the owner; the
product shipped to the target device. GitHub Pages on `main`; APK via `android.yml` (Capacitor);
target Pixel 11 Pro XL.

---

## Part 3 — The findings, carried

Every open finding from `docs/AUDIT.md`, verbatim, under the item that closes it. A finding
here is **evidence for the change, not a task of its own** — do not chase one. The severity
letter in each heading is the original read's: **A** structural, **B** live defect, **C** local.

> **Item numbers INSIDE these verbatim bodies are `docs/AUDIT.md`'s**, not this file's. The old
> numbering maps: old 1 Reach → items **1**, **3**, **4**, **6**, **7**, **10** here; old 2 → **5**;
> old 8 `Agreement` → **9**; old 10 `CorporateAction` → **20**; old 16 `Measure<D>` → **2**;
> old 17 the local repairs → **21**; old 18 the sectors that were waiting → **10b**, **11**, **13**,
> **14**, **16**, **17**, **19**. Worklist rows keep their own ids (13k → **20**, 13m → **15**,
> 13n → **12**, 13o → **13**, 14 → **19**, 15 → **22**, 16 → **23**, 17 → **24**).

---

### For item 1. The existence check

#### B-12 — 99 `MET` marks stand on mechanisms that have never produced anything (A)

Counted over `docs/COVERAGE.md`'s 823 `MET` rows, by the module each cites:

| module                    | MET rows | why nothing comes out of it                                             |
| ------------------------- | -------- | ----------------------------------------------------------------------- |
| `derivative-layer`        | 22       | B-7                                                                     |
| `cds`                     | 17       | B-7 (single name never crosses; the series book requires its own print) |
| `irs`                     | 14       | B-7 (the books are never run)                                           |
| `control`                 | 10       | B-4                                                                     |
| `securities-lending`      | 9        | B-3                                                                     |
| `insurers`                | 9        | B-2                                                                     |
| `commodity-futures`       | 5        | B-7                                                                     |
| `bond-futures`            | 4        | B-7                                                                     |
| `corporate-bond`          | 3        | B-1                                                                     |
| `index-futures`           | 3        | B-7 (sell-only by construction)                                         |
| `housing`                 | 3        | B-5                                                                     |
| **distinct requirements** | **99**   |                                                                         |

COVERAGE's own header says `MET at <path>` means _"the cited module implements the clause"_ — which
is literally true of all 99 and is not what a reader takes from it. The file has no way to say
"implemented and never reached", and that is the state 99 of its 823 green marks are in.

#### C-5 — 12,519 declared parameters are never read (carried from `VERIFY.md`)

29,559 declared, 17,040 read in a run. Discounting the ones read at seed time before the instrument
was attached, the substantive ones:

| never read | what it is                          |
| ---------- | ----------------------------------- |
| 5,896      | `firm.hurdle.<firm>`                |
| 5,896      | `firm.horizon.<firm>`               |
| 493        | `equity.payoutPatience.<firm>`      |
| 30         | `bank.lending.capitalAtRisk.<bank>` |
| 9          | `fund.requiredYield.<fund>`         |
| 7          | `fund.fee.<fund>`                   |

VERIFY's own correction stands and is the point: the hurdle IS read, at `firms/decide.ts:461`, but
inside `cost.some ? project(...) : none()` — so a firm whose `costOfCapital` is `none` never
evaluates a project at all. **9,006 firms alive, 3,104 ever evaluate one.** Two thirds of the economy
never makes an investment decision, because no bank has quoted them and no market prices their
equity — which is B-13's first and second causes seen from the parameter register.

---

### For item 2. The dimension sweep, finished

#### A-1 — `reseat` books an issuer's whole liability against a PER-MEMBER equity account (B)

`ledger/settlement.ts:1228-1257`, the `reseat` op. `owed` is summed over holders as
`Σ (lots × weight)` — a TOTAL — and then `bump(op.from, owed)` / `bump(op.to, -owed)` write it
straight into `equity`, which every other path in this file keeps **per member of the party**
(`register/register.ts:EquityMove.delta`, "per member for a cell").

The same defect was found and fixed in two of its three places: `issue` (line ~1150) and `redeem`
(line ~1205) both wrap the amount in `perMemberOf(op.issuer, …)`, whose own comment says why —
_"the day a CELL issued something, it booked a million households' worth of liability against one
household's equity"_. `reseat` is the third place and it was not wrapped. So is the issuer re-mark
inside the `credit` case (line ~1120): `bump(issuerOf(inst), mul(op.totalQty, carrying − basis, …))`
uses `totalQty`, again unscaled.

**Not reachable today** — the three drafters of an `assume` leg (`estate`, `control`,
`money-market/resolution`) all move paper from a named party, and the `owes: 'value'` kinds are
issued by funds and insurers, which are named. It fires the first time a cell's issued paper is
assumed or its shares change hands, which is exactly the door 13d.1 opened.

**Fix:** the same one, in the two remaining places — `perMemberOf(op.from, owed)` /
`perMemberOf(op.to, owed)`, and `perMemberOf(issuerOf(inst), …)` on the credit re-mark. Better: make
`bump` take the total and divide, so there is one door and a new writer cannot forget.

#### A-5 — `payable`, `cashFor` and `deliverable` take a unit and ignore it (C)

`registry/registry.ts`. All three are declared `payable(_ccy, amount)`, `cashFor(_ccy, value)`,
`deliverable(_unit, qty)` and the body is `downTick(amount)` / `toTick(value)`. Every quantity in
the engine is already a count of pieces, so no conversion is possible or wanted — but the signature
says one happens, and it invites exactly one mistake: handing it a NAMED amount (188.49 USD) where
it wants pieces (18849). That is the same class of error as the two unit slips 11.5 found in the
seed, and here the parameter list actively suggests it.

**Fix:** drop the parameter, or make it do the conversion it claims. Not both.

#### A-6 — `labourParam` is declared in minutes and holds hours (C)

`mechanisms/goods/index.ts:122-129`. The value is `(d.labourHoursPerUnit * TIME_PIECES) /
PIECES_PER_UNIT` and `TIME_PIECES` is 1 because a piece of time is AN HOUR
(`registry/grid.ts`: _"Time: THE HOUR"_). The declared `unit` string is
`` `minutes per piece of ${d.subUnit}` ``.

`dimension: 'ratio'` is what a machine checks, so nothing catches it; the prose is what a person
reads, and a person who believes it is out by sixty. Law 8 says the unit is part of the number.

**Extension (verified against `registry/grid.ts`).** `TIME_PIECES = 1` and grid.ts is explicit
about why: _"Time: THE HOUR. Labour is contracted, supplied and paid for by the hour in this world
and no wage in it is struck for part of one, so an hour is the smallest piece of somebody's time
there is."_ So the value is hours per piece and the unit string is wrong by a factor of sixty.

The same stale premise is in a second place. `mechanisms/labour/index.ts`:

```ts
// Labour A1, Law 8: time has a smallest piece too. A thousandth of an hour is about four
// seconds, which is finer than any contract in this world states and coarse enough to be real.
units: [{ id: HOURS, name: 'hours', perUnit: TIME_PIECES }],
```

`TIME_PIECES` is 1. The comment describes 1000. Two comments in two modules both describing a grid
this world does not have (Law 16).

#### A-18 — the fraction of a person is discarded every period, and the world fragments into frozen micro-cells (A)

`households/lifecycle.ts`, twice:

```ts
// XI-15: a weight is a COUNT of people, so what crosses is whole people, and the fraction that
// is not somebody stays where it is until enough of it has accumulated to be somebody.
const crossing = Math.floor(mul(weightOf(cell), share, 'the people standing at the boundary'));
if (crossing <= 0 || crossing >= weightOf(cell)) continue;
```

```ts
// XI-15: whole people. The fraction that is not somebody waits until it is.
const dying = Math.floor(mul(weightOf(cell), rate, 'the people who die this period'));
if (dying <= 0 || dying >= weightOf(cell)) continue;
```

**The comment is false in both places.** Nothing accumulates. `crossing` and `dying` are recomputed
from `weightOf(cell)` every period, so the fraction below one person is thrown away every period and
never "waits until it is" anybody. There is no residual store, no carry, no accumulator anywhere in
the file.

The consequence is not a rounding nicety, it is a trap, and the arithmetic closes it:

- A cohort band spanning 10 years is ~521 periods, so `share ≈ 1/521 ≈ 0.00192`.
- The seed opens 15,000,000 members per cohort (`seed.households.membersPerCohort`) over
  `banks × 2` cells (`seed.households.cellsPerKey = 2`). With four banks a cell is ~1.9M members and
  ages ~3,600 of them a period into a **new** cell (`reKey` → `nextSplitId`, never into an existing
  cell of the target cohort — and `cells.merge` is called by nobody, A-17).
- That new cell of ~3,600 ages `floor(3600 × 0.00192) = 6` a period. Its children age
  `floor(6 × 0.00192) = 0` — **for ever**.
- Mortality is worse, because the rates are smaller: any cell below `1/rate` members has
  `floor(weight × rate) = 0` and **nobody in it ever dies**, at any age, for the life of the run.

So after a few hundred periods the world holds a linearly growing population of micro-cells that
can never age out of the cohort they are in, can never die, and can never merge back into anything.
They keep voting, working, consuming and holding money for ever. `crossing >= weightOf(cell)` adds
the other end of the same defect: a cell small enough that its whole weight would cross is skipped
entirely rather than moved as itself, so the last members of a band are pinned there too.

The honest mechanism is the one the comment already describes: a per-cell remainder that carries
forward, so that `floor` is a timing of a real event rather than a deletion of it. It exists
nowhere.

#### A-19 — probate takes in every currency and pays out one (B)

`handToProbate` was deliberately widened to hand over every money a dead cell held:

```ts
// 13j: EVERY MONEY IT HELD, and not only its own. A world with four of them has households paid a
// coupon in one they do not bank in (Currency C4), and this used to move the cash of the cell's own
// region and leave the rest …
const monies = new Set<CurrencyCode>([ctx.registry.currencyOf(region)]);
for (const h of view.holdings()) { if (isMoney(…)) { monies.add(ctx.instruments.get(h.instrument).ccy); …
```

`settleEstates`, forty lines below, still does what the fixed side used to do:

```ts
const ccy = ctx.registry.currencyOf(office.region);
…
const cash = view.cash(ccy);
```

One currency: the office's own region's. Every other money a dead household held arrives at probate
and never leaves. Probate `never trades` (its own docstring), has `fails: []`, `borrows: false` and
no other outlet in the module, so a foreign balance there is permanent. It is a holder, so it is not
formally a residual with no holder — it is worse in one respect, because the accounts family will
happily confirm it every period: probate's equity grows without bound and the money is out of the
circuit for good.

The inbound fix names the exact reason the outbound one is needed (households are paid coupons in
money they do not bank in) and stops one function short.

#### A-23 — `wealthOf` and `atRisk` add currencies; the function beside them refuses to (C)

`households/consume.ts`:

```ts
function wealthOf(view: ParticipantView, cash: number): number {
  const terms = [cash]; // the HOME currency
  for (const h of view.holdings()) {
    // every holding, whatever it is denominated in
    const print = view.print(h.instrument);
    if (!print.some) continue;
    terms.push(mul(units.value, print.value.price, 'what it holds is worth'));
  }
  return sum(terms).value;
}
```

`view.print` returns the print in the INSTRUMENT's own money. `cash` is `view.cash(currencyOf(region))`,
the cell's own. Nothing converts and nothing checks: this is `Appendix B`'s _"two currencies never
added"_, written out. `atRisk`, ten lines up, does the same thing with `confidence`, which the
`Outlook` contract says is _"in the same unit as the variable"_ — the variable being
`price.<instrument>`, i.e. that instrument's money.

Both feed `spendPerMember`: `wealth` into the gap that sets what a household spends, `atRisk` into
the cushion it holds. A foreign line at a rate of 1 hides completely; at any other rate the cell's
whole spending decision is taken on a sum of two moneys.

`savingLines` in the same module, forty lines away, is careful about exactly this:

```ts
if (!i.status.live || !i.market.some || i.ccy !== ccy || !i.issuer.some) continue;
```

and the seed is careful too (_"Its own country's paper: a household saving in a money it is not paid
in would be a currency position nobody took (Currency D2)"_). So today a household's holdings are
all in its own money and the defect does not bite. It is C and not A for that reason only — the
guard is in the two places that CHOOSE what a household acquires, and absent from the place that
VALUES what it has. `handToProbate`'s own header asserts the opposite is already happening (_"A world
with four of them has households paid a coupon in one they do not bank in"_), and if that is true
this is A rather than C. `ctx.valuation.inMoney` exists and is the read that would settle it.

#### A-32 — `levelsBelow` claims a refinement invariance its own arithmetic does not have (C)

```ts
export function levelsBelow(opinion: number, steps: number): number[] {
  const out: number[] = [];
  for (let step = steps; step >= 1; step -= 1) {
    out.push(mul(opinion, div(step, steps, 'this level of the grid'), 'a level it would pay'));
  }
  return out;
}
```

with the docstring _"every level of a coarser grid is a level of a finer one, so refining adds
answers and moves none"_ and the parameter declared `kind: 'resolution'` with
_"change it and the answer must not move"_ (`households.demand.steps`).

Two things are false:

1. The levels are `opinion × k/steps` for `k = steps…1`. A grid of 5 and a grid of 7 share only the
   top level. The subset property holds only when one count divides the other.
2. The grid's **bottom** is `opinion / steps`, so the count of steps sets how far down the cell bids
   at all. A cell with `steps = 5` posts nothing below `0.2 × opinion`; with `steps = 10` it posts to
   `0.1 × opinion`. A session clearing anywhere below the coarse grid's floor sees a different
   quantity from this cell depending on the resolution, and a session clearing _between_ two rungs
   sees the rung above rather than the curve — `budget / rung` instead of `budget / cleared`.

The step function is a legitimate way to post a curve. The claim of invariance is what is wrong, and
Law 2 is specific that a RESOLUTION is _"tested by invariance"_ — so this is either a resolution that
has never been tested, or a SHAPE (a claim about the answer) wearing a resolution's name, whose count
must fall. `pricesOver` in `consume.ts` has the same property.

#### A-33 — `lastOwn` has no period bound, and four firm reads treat a stale wage bill as this period's (B)

`world/world.ts:1204` — `lastOwn: (kind) => this.journal.lastOf(kind, party)`. It is the most recent
event of that kind for that party, from any period, ever. Several callers guard it
(`households/index.ts` and `firms/index.ts` both test `own.value.period !== view.period` before
reading a plan). Four do not, all of them reading `labour.wages`:

| reader                        | file                | what it becomes                                                                                                                                                                                                            |
| ----------------------------- | ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `wagesDue`                    | `firms/decide.ts`   | the payroll `sellSchedule` force-sells stock to cover                                                                                                                                                                      |
| `wagesPromised`               | `firms/index.ts`    | the payroll published to lenders in `firms.funding`                                                                                                                                                                        |
| `hoursUnderContract`          | `firms/decide.ts`   | the hours the production plan is built on                                                                                                                                                                                  |
| `wageFacing`                  | `firms/decide.ts`   | the wage a unit is costed at                                                                                                                                                                                               |
| `quotedRate`                  | `firms/invest.ts`   | the cost of debt in the cost of capital — _"AT THE MARGIN, NOW … not the average coupon on debt already outstanding"_, read from `lastOwn('credit.quoted')` with no period test, so it is the last quote the firm ever got |
| `lastWageBill`, `wageItFaces` | `treasury/index.ts` | the state's own payroll in its funding programme, and what it bids for an hour                                                                                                                                             |
| `linesCovered`                | `banks/staff.ts`    | how many lines a dealing desk can quote (A-60)                                                                                                                                                                             |

`payWages` writes the event **only for employers that have rows this period**
(`for (const [employer, bill] of bills)`, and `bills` is keyed off `allRows(book)`). So a firm that
sheds its last worker stops generating the event and every one of these four reads freezes at its
last employed period, permanently.

The consequences are not symmetric noise:

- It force-sells at `price: 'market'` every period to cover a payroll of nobody
  (`short = wagesDue − cash`), which is XI-2's forced seller firing on a phantom obligation.
- It publishes that phantom payroll in `firms.funding` as _"what it is about to have to pay"_, which
  is what a bank lends against (`Banks Lending C2`).
- It plans production on hours it no longer employs, and `produce.ts:start` then finds
  `productiveHours` is 0 (that one IS period-filtered) and records `firms.idle` — so the plan and
  the line disagree about the same firm in the same period.

Each docstring states the period it means (_"what it had under contract at the close of the period
before"_, _"the payroll it has already promised"_) and none of them asks for it. The fix is one
predicate, and the reason it matters is that `lastOwn`'s own contract — _"the most recent event of a
kind THIS party is a subject of"_ — makes no promise about when.

#### A-38 — a cell's outside option is every kind of money it received (B)

Downstream of A-37 and separable from it. `labour/matching.ts`:

```ts
function reservation(ctx, cell, hours) {
  const outlook = ctx.participant(cell).outlook('income');
  if (!outlook.some) return undefined;
  return div(outlook.value.expected, hours, 'reservation wage');
}
```

with the docstring _"Its outside option is what it lives on without the job — read from its own
outlook of its own income, which for somebody not working is the benefit this world pays it."_

For somebody not working, `income` is the benefit **plus** every coupon on the paper it holds, plus
every distribution it received from probate, plus (per A-37) anything it sold. B1.a's outside option
is the income it has _instead of_ working; this is the income it has _including_ working, for an
employed cell, and _including its capital_ for any cell.

So the wage a cell demands rises with the paper it owns, one-for-one, with no reason behind it: a
saver is not less willing to work because a bill paid a coupon. And an employed cell's reservation
is its own current wage divided by its hours, so an employed cell can never be matched below what it
already earns — a wage can go up and never down, which is a downward rigidity nobody declared
(Appendix B forbids stated wage rules; this is one arriving through a read).

#### A-39 — the wage bill capitalised into inventory is bigger than the wage that was paid, every period, for every row (A)

**This is the clearest conservation break found so far: value enters the world with no counterparty.**

`labour/matching.ts:payFrom` rounds a wage down to real money before it moves it, and says so:

```ts
// Law 8, E1: a wage is paid in whole pieces of the money, to each worker separately … What the
// fraction below one would have been is not paid, because there is no such coin.
const share = shareFor(ctx.registry, to, currencyUnit(ccy), perMember);
if (share.total <= 0) return true;
…
amount: share.total,          // = downTick(perMember) × weight
```

`payWages`, its only caller, then records the **unrounded** number as what was paid:

```ts
const settled = payFrom(ctx, row.employer, row.worker, perMember, `wages from ${row.employer}`);
const total = mul(perMember, row.headcount, 'wage bill');     // raw perMember, NOT share.perMember
bills.set(row.employer, { …
  paid: add(bill.paid, settled ? total : 0, 'wages paid'), … });
```

`produce.ts` capitalises that number into the batch:

```ts
const wages = wagesThisPeriod(ctx, firm);      // reads bill.paid
const costs: number[] = [wages];
…
legs.push({ kind: 'create', party: firm, instrument: wip, qty: batch,
  costPerUnit: div(cost.value, batch, 'what a unit on the line has cost') });
```

Trace the firm's equity over the two instructions:

| event             | firm equity                                                             |
| ----------------- | ----------------------------------------------------------------------- |
| the wage leg      | `− downTick(perMember) × headcount`                                     |
| the batch created | `− Σ inputCarrying + (wages + Σ inputCost)` = `+ perMember × headcount` |
| **net**           | **`+ (perMember − downTick(perMember)) × headcount`**                   |

and the household side rises by `downTick(perMember) × headcount`, so the world's equity rises by
`(perMember − downTick(perMember)) × headcount` and nothing anywhere falls. `wagePerMember` is
`wagePerHour × hoursPerMember` and `wagePerHour` is `marginalBid(outcome)` — an arbitrary real
number off the book, not a grid amount — so the fraction is non-zero on essentially every row.

It is not dust. The seed opens with `seed.households.membersPerCohort = 15,000,000`, so a headcount
is in the millions; half a piece times two million members is ten thousand currency units of equity,
per firm, per period, created out of nothing and compounding through the inventory it is capitalised
into.

Nothing catches it:

- `firms.productionCosts` compares the production instructions' equity effects against
  `firms.started`'s `wages` field. Both are `bill.paid`. The two records it thinks it is comparing
  are one number (this is the A-10 / A-14 shape again).
- the `accounts` family is satisfied — the firm's assets went up by exactly what its equity did,
  because the WIP carries the invented cost.
- `payFrom` returns `true` when `share.total <= 0`, so a per-member wage under one piece is recorded
  as fully paid with **no money leg at all**, and the whole of it is capitalised.

The fix is one word: `payFrom` already computes `share.total`, the amount that actually moved, and
has to return it instead of a boolean. Everything downstream — `bill.paid`, `firms.started.wages`,
the WIP cost — is then the money that changed hands.

#### A-44 — the household's liquidity premium is declared "over what a deposit returns" and is used as an absolute rate (B)

Declared in `households/index.ts`:

```ts
{ id: HOUSEHOLD_PARAMS.liquidityPremium, value: 0.005,
  unit: 'per annum OVER WHAT A DEPOSIT RETURNS',
  dimension: 'perAnnum', kind: 'preference', owner: 'model',
  why: 'Households D5, D5.a: what a saver wants for giving up instant access to its money. It is
        the whole of the substitution between a deposit and paper held directly …' }
```

Used in `households/index.ts:decide`:

```ts
const required = view.params.perAnnum(HOUSEHOLD_PARAMS.liquidityPremium);
```

and in `portfolio.ts:fundOrders`:

```ts
if (toFund <= 0 || p.offered < required) continue;
```

`required` is the bare 0.005. The deposit rate is never added to it, never read, and does not appear
anywhere in the households module's saving decision.

This was harmless when it was written and is not now. `portfolio.ts`'s own header still says so:

> _"A deposit is a holding of a bank's money — **it returns nothing at all here**, because paying
> for deposits is a decision a bank has not been given yet (Banks Funding B1, worklist 11)"_

Deposits now pay. `mechanisms/money-market/deposits.ts:payDepositInterest` settles a real money leg
every period for every holder of a bank's money whose kind has a deposit class, at the rate that
bank published on its board — and `banks/index.ts:1102` calls `setBoard` for every bank, every
currency, every period. So worklist 11 landed and the household's comparison did not move with it.

The consequence is a wrong substitution, not a cosmetic one: a household will subscribe to a money
fund offering 0.006 while its own bank's board pays 0.02, because `p.offered < required` tests
0.006 against 0.005. D5.a's substitution — _"the choice between a deposit and paper bought directly
… is how a rate reaches a saver"_ — currently runs against a constant instead of against the rate
the saver is actually being paid. The board is reachable: `households/bank.ts:board()` in the same
module already reads `bank.depositRate` for this cell's own bank and class.

Same shape, same file: `savingLines`' price and `fundOrders`' `required` are the only two things a
household weighs a claim against, and neither of them knows the deposit pays anything.

#### A-47 — a fund's mandate has no currency in it, so it bids its own money at a foreign price and its NAV adds two moneys (A)

Three places in `mechanisms/funds/`, one omission:

**1. The mandate.** `eligible(view, d, i)` (`index.ts:1120`) admits a line on three tests — live,
kind in `d.eligible`, and maturing inside `maxTenorPeriods`. There is no currency test.
`d.eligible` for every money fund is `['sovereign.bill']` (`data.ts:drawFunds`), which is **every
sovereign bill in the world**: `eligibleLines` counts the Japanese and European ones alongside the
American, and `ordersOf` will bid in any of their markets.

**2. The bid.** `ordersOf`:

```ts
const each = div(spare, lines, 'what it puts into each line it may hold');
const dirty = add(price, view.accrued(i.id), 'what a unit costs it');
const qty = downTick(div(each, dirty, 'units it bids for'));
```

`spare` comes off this fund's own `fund.struck` event and is in the fund's own money; `price` is
`priceAt(flows, required, …)` in the line's money. `each / dirty` divides one currency by another
and calls the answer a quantity. The fund also does not hold the foreign money it would have to pay
with — `strike` reads its cash as `ctx.register.quantity(fund, moneyOf(ctx, fund, ccy))` in its own
currency only.

**3. The NAV.** `nav.ts:navOf`:

```ts
for (const h of reads.holdingsOf(fund)) { … assets.push(worth.value.value); }
for (const other of reads.instruments()) { … owed.push(worth.value.value); }
const net = sub(sum(assets).value, sum(owed).value, 'what the fund is worth');
return { perShare: div(net, shares, 'net asset value per share'), … };
```

`worthOf` answers in the instrument's own money. Nothing converts. So a fund holding one foreign
bill publishes a NAV that is a sum of two currencies, and every subscription and redemption in the
world transacts at it (`strike` → `subscribe`/`redeem` both take `perShare`).

`holdingsWorth` (the base the forced pro-rata sale is struck on) does the same thing.

This also breaks the module's own audit family. `equityIsZero` works because the share liability is
`issued × navOf(...).perShare = assets − other liabilities` **by construction**, so equity is
identically zero — as long as both sides are computed the same way. `balanceSheet`, which the
`accounts` family uses, converts each holding into the party's own money (Currency D2 —
`world/assemble.ts:stateEquityAsRead` documents that conversion and the bug that came from omitting
it). `navOf` does not. A fund with any foreign holding therefore has a NAV and a balance sheet that
disagree by the FX difference, and `equityIsZero` reports a fund that has "mislaid somebody's money"
when what happened is that two readers of one book used two currencies.

The engine has the read: `ctx.valuation.inMoney` / `rateInForce`, which is what `revalue.ts` uses.

#### A-50 — `moneyOf` hand-builds an account id, which is the one thing `accountOf` exists to prevent (C)

```ts
function moneyOf(ctx: MechanismContext, party: PartyId, ccy: string): InstrumentId {
  const bank = ctx.parties.get(party).bank;
  return instrumentId(`money:${bank}:${ccy}`);
}
```

`MechanismContext.accountOf`'s contract says why this is not allowed:

> _"WHICH account a party holds a given money in. Its own money is at its own bank; a money its
> bank does not issue is held at that money's own central bank … One writer of that rule, the
> kernel's, so **a module never assembles an account out of a party's `bank` field**: a module that
> did would be right in one currency and wrong in every other."_

It also rebuilds the id string by hand rather than calling `moneyInstrumentId` (`core/ids.ts:124`),
so it is a second spelling of the same format too. It is the only such site in the engine
(`grep 'instrumentId(\`money:'` finds this one).

Used twice, both in `strike` and `redeem`, to read the fund's cash before deciding what it can pay —
while the leg that pays it goes through `ctx.accountOf(fund.id, ccy)`. Today both resolve to the
same account because a money fund's currency is its own region's. Under A-47 they diverge: the fund
reads zero at its own bank for a foreign balance held at that money's central bank, and refuses a
redemption it can afford.

#### A-51 — the kernel converts currencies and five module reads of the same books do not (A)

`audit/families/accounts.ts:balanceSheet` is the one read that gets this right, and its comment says
why it has to:

```ts
// Currency C4, C5, D2: A POSITION IN ANOTHER MONEY IS AN ASSET LIKE ANY OTHER, converted at the
// rate in force — the same rate the same period settled at, so what a balance sheet says and
// what a payment does cannot disagree.
assetTerms.push(
  view.valuation.inMoney(
    view.valuation.valueOfLots(inst.id, h.lots, view.period),
    inst.ccy,
    home,
    view.period,
  ),
);
```

Every term on both sides goes through `inMoney`. `world/assemble.ts:stateEquityAsRead` carries the
same lesson written out as a defect that was found and fixed: _"it added `valueOfLots` across every
holding in whatever money the instrument was priced in, where the read it is checked against converts
each one into the party's own (Currency D2). At a rate of one the two agreed and nothing showed."_

`inMoney` is exported on `MechanismContext.valuation` and **is called by no module in the engine**
(`grep -rn 'inMoney' mechanisms/` finds one unrelated local variable in `banks/dealing-quote.ts`).
Every module that walks a party's book sums it in whatever money each line happens to be in:

| site                                       | what the sum is used for                                 | reachable today?                                       |
| ------------------------------------------ | -------------------------------------------------------- | ------------------------------------------------------ |
| `funds/nav.ts:navOf`                       | the NAV every subscription and redemption transacts at   | **yes** — A-47: a fund's mandate has no currency in it |
| `funds/index.ts:holdingsWorth`             | the pro-rata base a forced sale is struck on             | **yes**, same reason                                   |
| `banks/capital.ts:capitalOf`               | the bank's capital position and its risk-weighted assets | only if a bank holds foreign paper                     |
| `money-market/resolution.ts:valueBook`     | the hole in a failing bank, which decides who bears it   | only if a failing bank held foreign paper              |
| `households/consume.ts:wealthOf`, `atRisk` | what a household spends (A-23)                           | guarded upstream today                                 |
| `equity/index.ts:531`                      | the opening share count of every listed firm             | seed-time, single-currency by construction             |

The two `funds` rows are live now. The rest are one holding away, and the failure mode is the one
`stateEquityAsRead` names: at a rate of one everything agrees and nothing shows, so the defect
arrives with the first non-unit rate rather than with the first foreign holding.

What makes it a single finding rather than six: the conversion is not hard, the door exists, the
kernel already uses it, and no module does. That is a habit rather than six oversights, and the
place to close it is at the reads that walk `holdingsOf`.

#### A-58 — the price a bank will pay for a note is a leverage ratio squared, called a cost of funds (A)

```ts
/**
 * C3, Law 3, Law 19: WHAT IT WILL PAY, per unit of face. A note is worth a discount on its face to
 * a buyer that could have lent the money itself — the discount is what its own money costs it,
 * which it reads off what it actually pays for money and never off a table.
 */
function priceFor(view: ParticipantView, ccy: CurrencyCode): number {
  const owed = view.owedIn(ccy);
  const equity = view.equity();
  if (equity <= 0 || owed <= 0) return 1;
  const cost = div(owed, add(owed, equity, 'what funds it'), 'what its own money costs it');
  return sub(1, mul(cost, cost, 'the discount it wants'), 'what it will pay per unit of face');
}
```

`owed / (owed + equity)` is the **debt share of this bank's funding** — a dimensionless ratio
between 0 and 1 — not a rate and not a cost. The variable is named `cost` and the comment says it is
_"what it actually pays for money"_. Squaring it and subtracting from one produces a price per unit
of face with:

- **no periodicity** (Law 8: a rate is not a number until its periodicity is) — the note's tenor
  appears nowhere;
- **no relation to any rate this world produces** — a bank funded 90% by deposits bids
  `1 − 0.81 = 0.19` per unit of face, an 81% discount on a senior tranche; one funded 50/50 bids
  0.75;
- **no dependence on the pool** — two vehicles with completely different loan books get the same bid
  from the same bank.

`owedIn(ccy)` is also the wrong quantity even as a leverage measure: its contract is _"what falls
due in `ccy` less what it holds of it"_ — a short-term funding gap this period, not the bank's
liabilities.

The number the docstring describes exists and is published under the bank's own name every period:
`banks/index.ts:publishCostOfFunds` writes a real per-annum blended cost (`costOfFunds` →
`FundingCost.perAnnum`), built from the coupons the bank actually paid. Discounting the note's own
cash flows at that rate — the same `priceAt(flows, required, on, dayCount, …)` the money funds use
in `funds/index.ts:ordersOf` — is what C3 asks for and is one line away.

Related, same function: `noteBids` posts a single order for the bank's entire spare cash at that one
price. Every other buyer in this world posts a schedule (Clearing A2); this one posts a point.

#### A-61 — every central bank remits its income to the same treasury, in its own money (A)

`mechanisms/central-bank-omo/index.ts:remit`:

```ts
const treasuries = ctx.parties.ofKind(TREASURY).filter((t) => t.status.alive);
const to = treasuries[0];
if (to === undefined) return;
…
const ccy = ctx.registry.currencyOf(ctx.parties.get(cb).region);
const leg: Leg = { kind: 'money', from: { holder: cb, issuer: cb }, to: ctx.accountOf(to.id, ccy), ccy, amount: paid, … };
```

`treasuries[0]` is whichever treasury the parties store returns first — an insertion-order artefact
of the seed's draw, not a fact about who owns this central bank. In a world with four countries
(which `13j` built and which the rest of the engine is careful about) **all four central banks remit
to one country's treasury**, each in its own currency, into accounts that treasury holds at three
foreign central banks.

The clause it cites says the opposite: _"REMITTANCE (E3) is its net INCOME … the treasury owns it."_
Each treasury owns its own central bank. As written, three governments never receive the seigniorage
on their own money and one receives all of it, as an unexplained foreign transfer.

The registry already answers the question: `registry.centralBankOf(ccy)` is used everywhere else to
pair a money with its issuer, and `treasuryOf(ctx, bank)` exists in `money-market/resolution.ts` for
exactly this lookup. Every neighbouring module got the multi-country pass — `declareVenues` opens a
money-market book per currency with the note _"a euro bank cannot settle a dollar loan on the Fed's
books"_; `treasury/runReceipts` skips foreign legs with a worked example of what it cost; the
resolution auction filters bidders by `currencyOf(other.region) !== ccy`. This one was missed.

#### A-65 — every option premium is a money-squared number, and it does not depend on the strike (A)

`mechanisms/options/index.ts:optionOrders`:

```ts
const outlook = view.outlook(`${PRICE_OF}${String(t.underlying)}`);
const moves = outlook.some
  ? mul(outlook.value.expected, outlook.value.confidence, 'what it thinks it moves')
  : 0;
const mine = moves > 0 ? add(moves, mul(moves, aversion, 'what its capital wants'), 'its quote') : 0;
…
return [{ party: view.self.id, side: move > 0 ? 'buy' : 'sell', price: mine, qty: asQty(qty) }];
```

**Two things, and the first is dimensional.**

`confidence` is `width(surprises)` — the standard deviation of `observed − expected` in the
variable's own unit (`expectations/index.ts:183`). For `price.<instrument>` that unit is **money per
unit of the underlying**, which is how every other reader uses it:

- `savingLines`: `sub(expected, confidence, 'what it will pay')` — subtracted from a price;
- `consume.ts:pricesOver(expected, width, steps)` — added to and subtracted from a price;
- `consume.ts:atRisk`: `mul(units.value, confidence, …)` — units × confidence gives money.

`expected` is money per unit too. So `expected × confidence` is **money² per unit²**, and it is
posted as `price` into a book whose tick is `CENT_TICK` and whose unit is `optionContracts` — money
per contract. The comment immediately above says what the right term is and then multiplies it by
the wrong one:

> _"What it thinks the thing MOVES is its own outlook's CONFIDENCE **and not its level**"_

The consequence is not a scaling constant: the premium is proportional to the **square** of the
underlying's price level, so an option on a line at 100 with 1% surprises quotes 100 (the whole
value of the underlying) while the same 1% on a line at 1 quotes 0.01. Every option in this world is
mispriced by a factor of the underlying's price.

**Second: the premium does not depend on the strike, the moneyness or the time to expiry.** `mine`
is built from the outlook alone. `t.strike`, `t.right` and `t.expiry` appear nowhere in it — `t.right`
is used only to decide whether a HOLDER wants a put, and `t.multiplier` only to convert a size. So a
party quotes the same premium for a deep out-of-the-money call and an at-the-money put on the same
line in the same session, and the strike ladder `openBooks` builds (_"a ladder of books on lines this
world already clears, at strikes around what they print"_) is a set of books that every participant
prices identically.

D7's _"the premium is what clears"_ is respected — nothing here derives a price from a volatility,
and `measures` correctly takes the implied move back OFF the printed premium. What is wrong is the
reservation each party brings to the book, which is what decides where it clears.

#### A-68 — loading a cargo writes off what the cargo cost (A)

`mechanisms/freight/index.ts`, the instruction that puts a cargo aboard:

```ts
{ kind: 'destroy', party: shipper, instrument: i.id, qty: asQty(take), why: 'consumed', fromCell: none() },
{ kind: 'create',  party: shipper, instrument: transit, qty: asQty(take),
  costPerUnit: div(add(share, 0, 'the freight'), take, 'what the voyage added to a unit'), toCell: none() },
{ kind: 'money', from: ctx.accountOf(shipper, ccy), to: ctx.accountOf(carrier, ccy), ccy, amount: share, … },
```

Settlement's effect on the shipper's equity, leg by leg (`ledger/settlement.ts:1089-1100, 1112-1116`):

| leg                                           | equity                                                |
| --------------------------------------------- | ----------------------------------------------------- |
| destroy the cargo                             | `− costOfDraw(lots, take)` — its whole carrying value |
| create the goods-in-transit at `share / take` | `+ share`                                             |
| pay the freight                               | `− share`                                             |
| **net**                                       | **`− costOfDraw(lots, take)`**                        |

The goods in transit are carried at the **freight alone**. Everything the cargo cost to buy or to
make is expensed at the moment it is loaded.

`add(share, 0, 'the freight')` is the tell: a sum of one term and a zero, where the second term is
what the cargo cost. `produce.ts` does the same operation correctly forty lines of another file
away — `const costs: number[] = [wages]; … costs.push(heldCost(ctx, firm, input.instrument, qty));`
— and `costOfDraw` is exported from `register/register.ts` for exactly this read.

The total over a completed voyage is right (`−C` at loading, `+P − F` at sale, so `P − C − F`), which
is why no conservation family catches it. What is wrong is the whole of what the module says it is
for:

> _"AND IT TAKES TIME (A3). A cargo leaves the origin now and is ON THE SHIPPER'S BOOK the whole
> way, at a place of its own — **A3.a's working capital: a shipper that has paid for a cargo and not
> yet got it is short of both**."_

It is not on the shipper's book at what it cost — it is on the book at the freight. So a shipper
mid-voyage shows an equity hole the size of its cargo (which `failedWhy`'s solvency trigger reads,
and which can kill a merchant on the water), and the arrival books a profit equal to the cargo's cost
that no trade produced. `arrive()` is correct — it carries the transit lot's own basis forward with
`div(cost, total, 'what a unit cost delivered')` — so the error is entirely at loading, and its own
docstring there says the opposite: _"the destination at what it cost INCLUDING the voyage (D2)"_.

---

### For item 3. The gather

#### A-54 — `gather` is called by exactly one module, so two other modules' venue schedules are never asked for (A)

`World.gather` (`world/world.ts:940`) is the **only** path that runs `venueParticipantDecls`:

```ts
gather(venue: VenueId, owner: string): void {
  const decl = this.venue(venue);
  forbid(decl.clearedBy === owner, 'Law 4', …);
  if (this.gathered.has(venue)) return;
  this.gathered.add(venue);
  for (const p of this.venueParticipantDecls) {
    for (const party of this.parties.ofKind(p.partyKind)) { …this.post(venue, o); }
  }
}
```

Across the whole engine, `ctx.gather(...)` appears once: `money-market/index.ts:269`. Three modules
declare `venueParticipants`:

| module                            | what the schedule is                      | gathered?                |
| --------------------------------- | ----------------------------------------- | ------------------------ |
| `banks` — `sessionOrders`         | a bank's money-market schedule            | **yes**, by money-market |
| `banks` — `staffOrders`           | a bank's bid for labour hours             | **never**                |
| `housing` — household rent orders | every bid and offer in the lettings venue | **never**                |

Consequences, both structural:

**1. The lettings venue is empty every period, for the life of the run.** `housing.lettings` runs
`letIn`, which reads `ctx.posted(rentVenue(region))`. Nothing has posted into it, `clear([])` is not
cleared, and the phase records `RENT_PRINT` with `outcome: 'noDemand'` and returns. So no tenancy is
ever signed, `collect` has no leases, no rent is ever paid, and `housing.rent` — a whole phase — does
nothing forever. Everything in the module's header about rent clearing between the owner's wear and
the tenant's income describes a session that never has an order in it.

**2. No bank ever bids for labour.** `staffOrders` is the bank's demand for hours, and the comment
beside it describes the mechanism it is meant to give: _"A bank whose book earns nothing bids nothing
and hires nobody, which is how a shrinking bank sheds staff without anybody writing a rule for it."_
It bids nothing because it is never asked. Banks in this world employ no one, so `operatingCostOf`
has no wage bill behind it and `labour.match` never sees a financial-sector employer.

`labour` reads `ctx.posted(v.id)` directly, so a FIRM's opening reaches the book (firms
`ctx.post(venue.id, …)` in `firms.decide`) — the bank's does not, because it went the other way.
That is the same split A-43 describes from the other side: labour builds one party's schedule itself
and ignores the door, and the door is what banks used.

#### A-60 — no bank makes a market in anything, because no bank employs anybody (A)

This is A-54's consequence, and it is larger than A-54.

`banks/dealing.ts:dealingOrders` gates every order — bid, offer, primary bid **and forced sale** —
behind one test:

```ts
if (!covers(view, i.id)) return [];
const state = stateOf(view, d);
if (state === undefined) return [];
const quoted = quoteFor(view, i.id, state);
…
const urgent = urgentSale(view, d, i.id);
if (urgent > 0) return [{ party: view.self.id, side: 'sell', price: 'market', qty: urgent }];
```

`covers` asks `linesCovered(view)`, and `linesCovered` (`banks/staff.ts:152`) is:

```ts
const own = view.lastOwn('labour.wages');
if (!own.some) return 0;
const hours = own.value.data['hours'];
if (typeof hours !== 'number' || hours <= 0) return 0;
…
return Math.floor(div(hours, per, 'the lines its people can cover'));
```

A bank only gets a `labour.wages` event if it employs somebody. `payWages` writes one per employer
that has rows; rows come from `hire`; `hire` comes from bids in `ctx.posted(labourVenue)`. Firms and
the treasury post there with `ctx.post` directly. **A bank's labour bid is a `venueParticipant`
(`banks/index.ts:1171`, `staffOrders`) and nothing ever gathers the labour venue** (A-54). So:

no gather → no bank bid for hours → no employment row → no `labour.wages` event →
`linesCovered() === 0` → `covers()` false for every line → `dealingOrders()` returns `[]`, for every
bank, in every market, in every period.

The `banking` occupation exists in `labour/data.ts` and its venue is opened per region, so the
market for bank staff is there with nobody bidding into it.

What that switches off:

- **Every dealer quote in the world.** No bid-offer spread, no inventory, no inter-dealer market
  (Dealer Desks E3, which the module notes is "without a second venue for it" — because the desks
  face each other in the ordinary session, and none of them is there).
- **`market.noView` on every book with orders in it.** `world/runOne` journals that event when no
  `speculative: true` participant posted. The bank face is the `speculative` one for bill, bond,
  share and fund-share books; households and merchants cover some of them, so it fires wherever they
  do not.
- **A bank cannot sell to meet a shortfall.** `urgentSale` is _after_ the `covers` gate, so XI-2's
  forced-seller door for banks — the one Banks Funding D1 and Money Market A2.b describe, and the
  one a fire-sale print is supposed to come out of — cannot open.
- **Primary dealership.** `primaryBid` is behind the same gate, so a sovereign auction has no
  primary dealer bidding into it.
- **The staffing mechanism it was built to express.** _"a desk that sheds staff drops lines, whose
  books then journal `market.noView` because nobody is standing in them"_ — every desk is in the
  shed-everything state permanently, and for a reason that has nothing to do with its book.

Two smaller things in the same path, worth noting because they will bite once the gather is fixed:

- `covers` picks the covered lines as the **first `linesCovered` entries of
  `view.instruments.all()`** — the global instrument-store insertion order, which is identical for
  every bank. So it is not a per-desk specialisation; it is one global cutoff applied to all of them,
  and every line past position `linesCovered` is quoted by nobody however many banks make its kind.
- `linesCovered` reads `lastOwn('labour.wages')` with no period bound — A-33's pattern again.

#### B-5 — the tenancy venue has never had an order in it (A)

Worklist 13d, **done**: _"a tenancy as a VENUE, clearing between what letting wears the owner and
what a household can pay rather than have nowhere."_ `Housing A2`, `B5` and `C4` marked MET.

Per A-54: housing declares its orders as `venueParticipants`, and `gather` — the only door that
runs them — is called by one module, which is not this one. The lettings session reads an empty
book every period and records `noDemand`. Per A-55 the buying half is unreachable too.

#### B-6 — item 9's dealers quote nothing (A)

Worklist 9, **done**: _"Equity, and dealers that carry inventory … a desk with a limit, a funding
cost and an inventory."_ `banks/dealing-quote.ts` is among the best-built files in the engine.

Per A-60: the gate in front of it (`covers` → `linesCovered` → `lastOwn('labour.wages')`) is zero
for every bank for ever, because no bank employs anybody, because its labour bid is a
`venueParticipant` and nothing gathers the labour venue. Not one quote is ever posted.

---

### For item 4. The families that cannot fail

#### A-4 — a claimed assembly guard on `exposedTo` does not exist (B)

`registry/environment.ts:conditionsFor`, the comment on the `value !== undefined` branch: _"A line
naming a fact this world does not have stands in an ordinary period for it. The world that has the
fact is where the check belongs, **and assembly is where it fires**."_

It does not fire anywhere. `world/assemble.ts` checks module ids, dependency cycles, money issuance
and bank choices, and nothing else; `grep -rn exposedTo world/` returns nothing. A recipe naming a
fact no environment module declares silently multiplies by 1 — an ordinary period — for ever.

Today every `exposedTo` in `mechanisms/goods/data.ts` names `growing` or `warmth` and both are
declared, so the number is right. The guard is a lie, and it is the kind that is discovered by a
line that quietly never has a bad season.

**Fix:** check it at assembly, and delete the paragraph (Law 12: the fix removes the comment).

#### A-10 — the currency audit family compares a number against itself (A)

`audit/families/currency.ts:revaluationAddsUp`. It walks `revaluation.fx` events and builds two
maps:

```ts
addTo(booked, ccy, delta);
addTo(implied, ccy, carried * sub(now, was, 'what the rate moved by'));
```

reading `delta`, `carried`, `was` and `now` off the same event. `world/revalue.ts:revalueForeign`
wrote that event, and it computed the field it wrote as:

```ts
const delta = mul(carried, sub(now, was, 'what the rate moved by'), 'what it did');
… { deltaPerMember: delta, ccy, home, was, now, carried }
```

So `implied` is `booked` recomputed from the operands `booked` was computed from — the same product,
to the bit. **The family cannot fail.** Audit A1.a in as many words: _"a read of two independent
things that must agree — never a read of one thing against itself, which always passes."_

The check it is trying to make is real and is not being made: what every revaluation BOOKED against
what the period's rate move on the positions comes to. The second half has to be reached from the
REGISTER — walk the holdings, take each one's carrying and the rate move, and compare the total
against the sum of what the equity and revaluation accounts actually moved by. That is two paths;
this is one path twice.

It is contributed to the `money` family, so `money` reports two contributions of which one is a
tautology, and `built` is true for both.

#### A-11 — `flows` switches Money D3 off for a whole cell for a whole period (C)

`audit/families/flows.ts`, the `copied` set: a cell named as the `to` of a split or promotion, or
the `from` of a merge, has EVERY holding of it exempted from the identity for that period
(`if (copied.has(holder)) continue;`). The narrowing from "every subject of every weight event" was
right and this is still wider than the hole: what has no leg behind it is the copy of the parent's
book at the instant of the split, not everything that cell then does. A cell created by a split at
cycle 0 that trades at cycle 1 has that trade unchecked.

The exemption exists because `copyMemberState` moves a whole book with no instruction. The narrower
form is to compare against the parent's remembered per-member state rather than against nothing —
the split is exact, so the new cell's opening position IS the parent's, and any difference is a leg.

#### A-12 — a comparison in `names` that can never be true (C)

`audit/families/names.ts`:

```ts
!view.markets.some(
  (m) => m.id === i.market.valueOf() || (i.market.some && m.id === i.market.value),
);
```

`i.market` is an `Option<MarketId>`; `.valueOf()` on it returns the option OBJECT, so
`m.id === i.market.valueOf()` compares a string to an object and is always false. The second
disjunct does the whole job. Dead code in the family whose subject is that references resolve.

#### A-13 — the population identity is checked for households and for nothing else (B)

`audit/families/units.ts` checks that a weight is a positive count, that physical stock moves with
its create/destroy legs, and that every lot is on the grid. Part XII's units family is also
_"including a population: the sum of cell weights equals the population it stands for, and every
represented party sits in exactly one cell."_

Neither of those two is in the kernel family. One of them is contributed by a module:
`mechanisms/labour/index.ts:workforceIdentity` (contributor `labour`, family `units`) checks, per
region, that the employment rows' headcounts equal the cells' weights and that no cell holds two
jobs — a real F2 check on the register against the cells. The other contributors to `units` are
`capital-programme` (`index.ts:435`) and `goods` (`:266`, `:328`), neither of which counts people.

So the identity is measured for HOUSEHOLD cells through their employment, and is unmeasured for
every other cell in the world (`firms`' small-business pools, XI-15 cells generally): a split, merge
or promotion on one of those can lose or invent members and no family looks. Narrowed from the
original wording, which said the identity was checked nowhere.

#### A-14 — that same household check is an identity that cannot fail (A)

`mechanisms/labour/index.ts:workforceIdentity`, the check A-13 credits, contains the half of Part
XII's identity that reads on population, and it is written as a tautology:

```ts
acc.people += p.weight;
acc.employed += isEmployed ? p.weight : 0;
// B3: exactly one of the three, counted once each way round.
acc.states +=
  (isEmployed ? p.weight : 0) +
  (!isEmployed && working ? p.weight : 0) +
  (!isEmployed && !working ? p.weight : 0);
```

The three terms are `E`, `¬E ∧ W`, `¬E ∧ ¬W`. They are mutually exclusive and exhaustive over the
two booleans, so for every cell exactly one term is `p.weight` and the other two are `0`. Therefore
`acc.states === acc.people` on every iteration, by construction, and

```ts
if (acc.states !== acc.people) { … 'employed plus unemployed plus inactive is …' }
```

can never fire. The comment says the opposite — _"counted once each way round"_ — but there is only
one round: both sides are the same sum of the same `p.weight`s over the same loop. This is A-10's
defect again (Audit A1.a: a read of one thing against itself), and it is severity A rather than B
because the docstring, the spec citation (`Labour B5`) and `built: true` all report that Part XII's
population identity is being measured when nothing is.

What would make it a measurement is a second, independent record of the three states — the counts
the labour mechanism itself acts on when it hires and separates, or the cohort register's own
population — compared against the cells. Two reads of the same `p.weight` is not two records.

The two neighbouring checks in the same function ARE real: `row.headcount !== weight` and
`inRows !== acc.employed` read the employment book against the parties store, which are genuinely
two writers. Only the B5 line is empty.

#### A-42 — two `units` families switch themselves off in any period with a weight event, which is every period (A)

`mechanisms/goods/index.ts:unitsIdentity`:

```ts
// A weight event moves stock between books without an instruction (a cell splits, a member
// dies): the holders' totals are re-struck, and this period's identity is not about them.
const weights = view.journal.ofKindIn('weight', view.period).length;
const consecutive = seen.period !== undefined && view.period === seen.period + 1;
…
  if (!consecutive || before === undefined || weights > 0) continue;
```

`mechanisms/capital-programme/index.ts:461`, in the same words:

```ts
const weights = view.journal.ofKindIn('weight', view.period).length;
…
const comparable = consecutive && weights === 0;
for (const [k, before] of comparable ? seen.held : new Map<string, number>()) { … }
```

`weights` counts **every weight event in the world**, not the ones touching the instrument being
checked. `households/lifecycle.ts:age()` runs `ctx.cells.reKey(...)` — which journals a `'weight'`
event — for every household cell with enough members to move one person, every period. So from the
first period in which anybody ages, `weights > 0` holds permanently and **neither family reports
anything again for the rest of the run**. Both declare `built: true`, so the audit shows them green.

What is lost is the fine half of Part XII's stock identity. The kernel's own `units` family
(`audit/families/units.ts`) survives, but it compares each instrument's `issued` total against the
create/destroy legs. These two compare the **holdings** — `heldTotal` per instrument, and quantity
per (holder, instrument) — which is where stock that moved between books with no leg behind it would
show. That is the case the goods docstring says it exists for: _"so a stock that moved without a leg
has nowhere to hide."_

The exemption is also unnecessary, which is what makes it worth deleting rather than narrowing. Of
the three ways a weight can move today:

- `splitCell` / `reKeyCell` copy per-member state and split the weight, so
  `perMember × (w−m) + perMember × m = perMember × w`. The total does not move.
- `dieCell` refuses a cell that still holds anything, so nothing physical is on it to move.
- `weightEvent` (entry, death, promotion) would move a total — and per A-17 it is called by nobody.

So the guard protects against a case that cannot occur, and pays for it with the two families it
was attached to.

#### A-48 — `equityIsZero` cannot fail for the reason it says it checks (C)

Separable from A-47, and worth stating because the family's docstring claims the opposite:

> _"A3: a fund's equity is ZERO. … **Nothing in the module enforces it: it falls out of the wire**,
> and this is the check that says whether the wire actually did it."_

It does not fall out of the wire. `fundShareKind.owes: 'value'` and
`derive: (i, at, reads) => navOf(i, at, reads).perShare`, and `navOf` returns
`(assets − other liabilities) / shares`. So the fund's liability is _defined_ as its assets net of
its other liabilities, and `assets − liabilities = 0` is an algebraic identity, not an outcome. No
subscription, redemption, fee, mark or trade can move it — which is exactly the property the module
elsewhere states out loud (`fundKind`: _"its equity is zero by construction (A3)"_).

What the family can still catch is a **disagreement between two valuation paths** — the FX one in
A-47, a liability `navOf` skipped because `worthOf` was `none` while `balanceSheet` counted it, or a
`div`/`mul` residue. That is worth having. It is not what the docstring says it is having, and a
reader who trusts the docstring believes the wire is being checked when the identity is being
restated (Audit A1.a; the same shape as A-10 and A-14).

#### B-10 — "checks green" has been satisfied by families that cannot fail (A)

The loop's definition of done is _"an item is done when its checks are green, `docs/RECORD.md` has its
entry, and `docs/COVERAGE.md` is re-marked."_ Four of the nine audit families are green for reasons
that are not the state of the world:

- `currency`'s money contribution recomputes the number it is checking (A-10);
- `labour`'s population identity is `Σ p.weight` against `Σ p.weight` (A-14);
- `goods`' and `capital-programme`'s `units` contributions switch off in any period with a weight
  event, which is every period after the first ageing (A-42);
- `funds`' `equityIsZero` restates the definition of the NAV (A-48);
- and `zeroSum` — the derivative layer's whole invariant — walks a set of size 0 (VERIFY's census),
  because of B-7.

A green audit has been part of the evidence for thirteen `done` rows.

---

### For item 5. Missing is Missing, carried

#### A-25 — an unpriced physical leg is valued at zero inside an audit family (C)

`households/index.ts`, `consumptionIsBought`:

```ts
addTo(
  bought,
  leg.to,
  mul(leg.qty, leg.pricePerUnit.some ? leg.pricePerUnit.value : 0, 'what it took'),
);
```

A `? … : 0` on an `Option` inside a mechanism, which the error discipline forbids without
qualification (_"No `?? 0`, no `|| 0`, no numeric defaults … Missing is `Missing`"_). The family then
compares that zero against the money the cell paid and reports `took 0 of goods and paid X` — a
violation whose size and message are both about the missing price rather than about the flow. The
family's subject is that goods and money move together; an unpriced leg is a different defect and it
should be reported as one, or the leg should not be admissible.

#### A-30 — three numeric defaults where the discipline is `Missing` (C)

- `portfolio.ts:ownUncertainty` — `if (!income.some || income.value.expected <= 0) return 0;`. A cell
  with no income outlook is reported as wanting **no** extra return for holding a claim that
  promises nothing, which reads as certainty and is ignorance.
- `portfolio.ts:fundPositions` — `offered: typeof offered === 'number' ? offered : 0`. A fund whose
  strike event did not publish `offered` reads as offering nothing, so `p.offered < required` is
  true and the cell silently never subscribes to it.
- `lifecycle.ts:heirOf` — `String(ctx.registry.cohorts[0]?.id ?? '')`, producing a cohort id that
  matches nothing, so a world with no cohorts silently has no heirs rather than saying so.

#### A-34 — a firm with no wage history bids for inputs as if labour were free (B)

`firms/decide.ts`, the input bid:

```ts
price: div(
  sub(sub(
      sub(mul(price.value, tech.yieldRate, …), mul(tech.hoursPerUnit, wage.some ? wage.value : 0, 'its wages'), 'less wages'),
      capitalCharge, …),
    sum(…other inputs…).value, …),
  input.qtyPerUnit, 'what a unit of the input is worth'),
```

`wage.some ? wage.value : 0`. The same function handles the missing wage correctly twice — `unitCost`
is `none()` without a wage, and `worthMaking` switches from `perHour > wage.value` to `perHour > 0`
— and then treats it as zero here, in the one place that reaches a market.

What an input is worth to a firm is the output it makes possible less everything else that unit
needs, and labour is part of everything else. Pricing it at zero makes the bid too high by
`hoursPerUnit × wage`, which for most recipes is the largest term. Every firm is in this state until
it has employed somebody, so at world open every input market clears against a book of systematically
inflated bids, and the firms that have never hired outbid the ones that have.

`wageFacing` already has the fallback the bid should use — the published going rate
(`labour.goingRate`) — and returns `none()` only when neither exists. A firm that knows neither its
own wage nor the market's cannot price an input, and that is a `Missing`, not a zero.

#### A-45 — a bank that owes nothing has a cost of funds of zero (C)

`banks/index.ts:costOfFunds`:

```ts
const blend = (interest: number): FundingCost => ({
  perAnnum: funding <= 0 ? 0 : div(add(interest, onCapital, …), funding, 'per annum'), …});
if (funding <= 0 || ctx.period === 0) return blend(0);
…
if (year <= 0) return blend(0);
```

Three paths return a cost of funds of exactly **zero**: a bank funded by nothing, the first period,
and a zero-length year. Zero is not "unknown" here — it is "money is free", and it flows straight
into a price. `quote()` builds the lending rate on it and `dealing.ts` builds the desk's edge on it,
so in period 0 every bank in the world quotes as if its funding cost nothing.

The neighbouring `atLeast(capital, 0, 'a hole funds nothing: there is no less capital than none')`
is a different case and its argument holds (a negative residual is not a source anybody requires a
return on) — though it is worth noting the comment records exactly the Law 6 pattern: a number went
negative, a quote crossed itself, and a floor was added at the read rather than at the cause.

`FundingCost` could say `Missing` for a bank it cannot cost, and a bank that cannot cost its funding
has no business quoting a rate — which is what the surrounding code does everywhere else (`quote()`
returns `undefined`, `costOfCapital` returns an `Option`).

---

### For item 6. The derivative books open

#### A-66 — eight of the nine derivative books can never produce a first print (A)

Every contract class builds a party's order the same way:

```
target  =  <its hedging need, from its own book>
        ±  <conviction, IF its own number differs from THIS BOOK'S LAST PRINT>
order   =  target − <what it already has>
```

Each class is careful, and says so, that the book's own print must not be the LEVEL it posts —
`bond-futures`: _"a party that posted where THIS book last was would be agreeing with it rather than
saying anything, and a book of those prints one number for ever"_; `cds`, `irs`, `commodity-futures`
and `fx-derivatives` all carry the same paragraph. What none of them noticed is that the print is
still load-bearing for the **direction**: with no print, the conviction term drops out and every
party in the book is left with its hedging need alone — and a hedging need has one sign.

| book                | with no print, `want` is                                                          | so the first session is | can it open? |
| ------------------- | --------------------------------------------------------------------------------- | ----------------------- | ------------ |
| fx forward          | hedgers one way, plus an arbitrageur quoting **bid and ask** around its own carry | two-sided               | **yes**      |
| option (put)        | `held × aversion / multiplier` for holders, 0 for everyone else                   | buy-only                | no           |
| option (call)       | 0 for everyone                                                                    | **no orders at all**    | no           |
| bond future         | `−held / contractSize`                                                            | sell-only               | no           |
| commodity future    | `−held / lotUnits`                                                                | sell-only               | no           |
| interest-rate swap  | `−fixedDebtOf(view, t)`                                                           | sell-only               | no           |
| CDS, single name    | `exposureTo(view, t)`                                                             | buy-only                | no           |
| index future        | `book / perContract`, and the only `side` in the file is `'sell'`                 | sell-only **always**    | no           |
| CDS series          | `if (!last.some) return []` on the book's OWN line                                | **no orders at all**    | no           |
| cross-currency swap | `if (!last.some) return []` on the book's OWN line, then buy-only                 | **no orders at all**    | no           |

Nothing seeds a print for a contract book: `openBooks` in each module calls `ctx.openMarket(...)`
with no price, and `foundation.ts` writes opening prints only for goods and sovereign lines. A
contract book's `instrument` is a synthetic line (`irsLineOf`, `seriesLineOf`, `t.book`) that only a
session can print. So `noDemand`/`noSupply` in period one, no print, and the same again for ever.

The one that works is the one whose author hit the problem and built the answer:

```ts
// B1, B2, B2.b: THE ARBITRAGE, and a party with nothing to hedge is the one that takes it. It
// quotes BOTH WAYS around its own carry — a bid a tick below and an ask a tick above …
// a book whose members were all hedgers printed one number for ever and the cash-and-carry
// relationship this class exists to express was live in period one and dead from period two.
return [
  { party: view.self.id, side: 'buy', price: bid, qty: asQty(size) },
  { party: view.self.id, side: 'sell', price: ask, qty: asQty(size) },
];
```

Two of the eight are worse than a bootstrap problem and would still be broken with a print in hand:

- **`index-futures/futureOrders` has no buy branch at all.** Its docstring is _"A DESK LONG A BOOK OF
  SHARES SELLS THE INDEX"_ — one true reason, and the only one implemented. §46 A3 and XI-13 are
  explicit that a market needs two, and this one is one-sided by construction rather than by
  circumstance.
- **`cdsIndexOrders` and `xccyOrders` read their own book's last print as a precondition**
  (`const last = view.print(seriesLineOf(...)); if (!last.some) return [];`) and then post at
  `last.value.price` or a multiple of it. That is exactly the fixed point the single-name CDS in the
  same directory refuses in a comment two hundred lines away: _"its OWN number — the cash market's
  charge for this credit … **Neither is this book's own last price**."_

Everything downstream of these books is downstream of this: `refusedThisPeriod`, the margin system,
the clearing house's waterfall, `marginIsHeld`, the option-implied move that §46 A3 says the world
needs so parties can disagree about dispersion, and the basis measures each class publishes. All of
it is built, wired and exercised by nothing.

#### B-7 — the derivative layer and its nine classes have never produced a contract (A)

Worklist 13a and 13b, both **done**, 37 steps between them. **99 distinct requirements are marked MET
against modules that have never produced an outcome**, and 58 of those 99 are the derivative layer,
CDS, IRS, the two futures classes and options.

`docs/VERIFY.md` measured the runtime side: nine derivative kinds declared, **not one contract of any
class ever written** over five periods; 7,510 option sessions all `noDemand`, 3,680 commodity-future
sessions all `noDemand`, 60 CDS sessions all `noSupply`, the IRS books never run at all. A-66 gives
the cause at the source: every class makes a party's SIDE depend on comparing its own number against
this book's last print, so with no print the conviction term drops out and every party is left with
its hedging need, which has one sign.

**This corrects `docs/VERIFY.md`'s own conclusion.** That file diagnosed the single measured cause as
the margin gate — _"no party can post margin in a currency it does not hold"_ — and concluded
_"the never-crossing books are downstream of it, not a second cause."_ They are not downstream of it:
they never reach admission, because they never cross. The margin gate is the whole story for **FX
forwards only**, which is the one book that crosses, and it crosses because it is the one class with
a two-way maker that needs no prior print. Two causes, and the structural one is the larger.

#### C-6 — the margin gate: 31,640 admission decisions, 31,640 refusals (carried from `VERIFY.md`)

The one measured cause of a dead book, and it is a real missing mechanism rather than a bug. Over two
periods, diagnosed against the three gates in `capacity().admits`:

| gate                                                         | count      |
| ------------------------------------------------------------ | ---------- |
| the clearing house has ceased                                | 0          |
| **no room — the party holds no cash in the book's currency** | **31,640** |
| the kind cannot say what one unit's margin is                | 0          |
| admitted                                                     | **0**      |

`capacityOf(view, ccy, buffer)` is `cash − cash × buffer`, so room is zero exactly when the party
holds none of that money. Every one of those is an FX forward: a party trading EUR/GBP must post
margin in EUR or GBP and holds neither. In the real world it would buy the currency or post eligible
collateral in another one; here there is no such path.

Two things follow that VERIFY did not draw:

1. It is the whole story for **one** book, not for nine — see **B-7**.
2. The missing mechanism is **eligible collateral in another money**, which `money-market/collateral.ts`
   already implements for the repo market (`advances`, `valueToLender`, `haircut`). The derivative
   layer's `admits` asks only about cash.

#### A-69 — nine exported entry points and reads that nothing calls (C)

Checked across the whole repository (`packages/`, tests and app included), these are defined,
exported, documented and referenced by nothing:

| function                                     | file                          | what it was for                                                                                                                                                                                                                             |
| -------------------------------------------- | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `runBorrows`, `wantsToBorrow`, `returnLoans` | `securities-lending/index.ts` | the whole borrow mechanism (A-67)                                                                                                                                                                                                           |
| `quoteCover`, `policyTerms`                  | `insurers/index.ts`           | the whole insurance mechanism (A-9)                                                                                                                                                                                                         |
| `cdsBookOrders`                              | `cds/participants.ts`         | _"Law 15: ONE PARTICIPANT, TWO SHAPES OF BOOK, and the dispatch is on the shape of the terms rather than on an id"_ — the two classes call `cdsOrders` and `cdsIndexOrders` directly, so the dispatcher it argues for is not the one in use |
| `refusedThisPeriod`                          | `derivative-layer/index.ts`   | _"E4: what the markets struck BEYOND what their members could margin — a standing measurement"_                                                                                                                                             |
| `struckRate`, `findSession`                  | `money-market/session.ts`     | reads of what a session struck                                                                                                                                                                                                              |

The first two rows are missing mechanisms and are logged separately. The rest are dead code, and
they matter for one reason: each carries a docstring asserting that something is measured or
dispatched, and a reader checking whether the world has a given property will find the function and
believe it. `refusedThisPeriod` in particular is described as _"a standing measurement"_ of E4 and
measures nothing, standing or otherwise.

---

### For item 7. The three closed lines

#### A-55 — nobody in this world can buy a dwelling, so the whole of Housing B1–C4 never runs (A)

`housing/index.ts:askForMortgages` begins:

```ts
for (const cell of ctx.parties.alive()) {
  /** 13d.1: A HOUSEHOLD CAN HOLD A ROOF AND CAN BORROW FOR ONE — … All four were built and
   *  measured here … WHAT STOPS IT IS ONE MORE THING, and it is not this item's: a borrower that
   *  MISSES A PAYMENT goes on accruing on the lender's book … the item that owns it is 13f … */
  if (cell.representation === 'cell') continue;
```

Every household is a cell, so no household ever publishes a `housing.funding` request, no bank ever
writes a mortgage, `mortgagesOf` is always empty, and `charge` and `foreclose` — two more whole
phases — are dead. The docstring is candid about the blocker; what it does not say is that the
consequence is the whole buying half of the module.

The other routes were checked and all are closed:

- `dwelling` is **not in the household consumption basket** — `households/data.ts` names 18 sub-units
  (bread, meat, clothing, appliances, vehicles, care, teaching, …) and no dwelling — so `demandOf`
  never bids for one.
- **No recipe names `dwelling` as an input**: the only occurrence in `goods/data.ts` is the good's
  own declaration, so no firm buys one to make something.
- **Merchants exclude it**: `marketsOf` filters `!i.terms.portable`, and a dwelling is declared
  non-portable (_"a house cannot be somewhere other than where it was built, at any price"_).
- **Fund mandates exclude it**: `drawFunds` gives `['sovereign.bill']` and `['good.grain']`.
- **Bank dealing desks exclude it**: `BankDecl.makes` is drawn from
  `['equity.share','fund.share','sovereign.bill','sovereign.bond']`.

So the market `mkt.good.dwelling.<region>` has a seller (the builder firms — `firms/data.ts` draws a
`dwelling` line with occupation `building`) and **no bidder, ever**. Three things follow:

1. The dwelling's price is the seed's placeholder for the life of the run. `foundation.ts:1571`
   writes an opening print for every good with `provenance: {kind:'opening'}` and the comment says
   _"that number is a placeholder and the market's own first session replaces it"_. For this one
   good no session ever does — so it is a PLACEHOLDER with no scheduled death (Law 2), and it is the
   number `wearOf` and `shortOfMoney` are both built on.
2. Builders accumulate unsold dwellings and lose them to spoilage (1% a year, `goods/data.ts:1735`),
   which is the only thing that ever removes one from the world.
3. Owner-occupation is unreachable, so the module header's _"OWNER-OCCUPATION IS AN OUTCOME AND NOT A
   TENURE FLAG: a household that owns as many dwellings as its members live in has nothing to rent"_
   describes a state no household can be in.

Combined with A-54, the housing system produces houses nobody can buy and lets none of them.

#### A-56 — three of this world's lines have a firm, a recipe and a market, and no buyer; they open at zero and never move (A)

Cross-referencing every declared good against every source of demand. `GOODS` is
`[...MAKES, ...RETAIL.map(shelfLine)]` — 54 written lines plus 9 generated retail lines, 63 in all.
The demand sources are: the household basket (`households/data.ts`, 18 rows, all of which resolve to
real goods), every recipe's `inputs` (43 distinct goods are drawn by something), and the goods a
capital kind is `madeFrom` (`building`, `machine`, `vehicle`, `vessel`).

Three goods are in none of them:

| good         | recipe                                          | portable? |
| ------------ | ----------------------------------------------- | --------- |
| `dwelling`   | timber, concrete, steel, glass, building trades | no        |
| `facilities` | chemicals, power, paper                         | no        |
| `itServices` | power, electronics                              | no        |

Non-portable, so `merchants/index.ts:marketsOf` (`if (… !i.terms.portable) continue`) excludes all
three; not in a fund mandate (`['sovereign.bill']`, `['good.grain']`); not in a bank's `makes`
(`['equity.share','fund.share','sovereign.bill','sovereign.bond']`). Nothing anywhere bids for them.

They are also **zero at the seed**, by the same fact. `foundation.ts:985`:

```ts
const finalGoods = [...sizeOfLine.keys()]
  .filter((g) => (drawnBy.get(g) ?? []).length === 0 && !madeInto.has(g)).sort();
…
for (const g of finalGoods) {
  let want = 1;
  if (asked > 0) want = zeroIfNone(wantedInAPeriod.get(g));
  started.set(g, div(want, recipeOf(g).yieldRate, 'started for what is wanted of it'));
}
```

All three qualify as "final goods" (nothing draws them, they are not plant), `wantedInAPeriod` is
keyed by basket sub-unit so it has no entry for any of them, and `asked > 0` because the eighteen
basket lines do have entries. So `want = 0`, `started = 0`, `plantOf(...) = 0`, and
`finished`/`onTheLine` are 0 — their firms open holding nothing, with no plant.

From there nothing can start them. `firms/decide.ts:plan` requires an outlook of its own sales:

```ts
const sales = view.outlook(`sold.${output}`);
if (!price.some || !sales.some || inputPrices.some((p) => !p.some)) return … { planned: false, … };
```

and `expectations/index.ts` forms `sold.<good>` only from an asset leg the firm was actually a side
of. A firm that has never sold has no outlook; with no outlook it makes no plan; with no plan it
starts no batch; with no batch it never sells. The three lines are closed loops at zero from period
zero, with named firms, a market, a printed opening price, and no way in or out.

`dwelling` is the one with consequences beyond itself: it takes the whole housing module with it
(A-55), and its market's seed placeholder never gets replaced, which is the number the rent floor
and every mortgage size would have been built on.

**Correction to my own working note.** An earlier pass of this analysis put nine more goods on this
list (`bread`, `meat`, `clothing`, …) on the grounds that the basket names `retailBread` and no such
good is written in `goods/data.ts`. That was wrong: `shelfLine` **generates** the nine retail lines
from the `RETAIL` table, each taking one unit of the wholesale good as its input, so the basket
resolves and those chains are live. Only the three above survive the check.

---

### For item 8. The securitisation waterfall

#### A-57 — a securitisation vehicle keeps the whole interest stream of its pool, for ever, and nobody owns it (A)

The vehicle collects everything the borrowers pay. `LOAN.due` (`banks/loan.ts:83`) emits a `coupon`
every period and a `maturity` at the end, and the kernel's corporate-action phase pays them to the
holder of record — the vehicle. So the cash in over the life of a deal is **principal + interest**.

What goes out is principal only. `trancheKind` is a pure pass-through with nothing to pay:

```ts
cashFlows: (): readonly CashFlow[] => [],
due: () => [],
accrued: () => 0,
```

and `payTranche` redeems face at par, one unit of money for one unit of face:

```ts
const toLayer = downTick(atMost(available, face, 'a layer takes no more than its face'));
…
{ kind: 'asset', from: holder, to: deal.vehicle, instrument: id, qty: share, pricePerUnit: some(1), … },
{ kind: 'money', from: ctx.accountOf(deal.vehicle, deal.ccy), to: ctx.accountOf(holder, deal.ccy), amount: share, … },
```

So `Σ out ≤ Σ face = the pool's opening principal`, and the interest — the entire economic return of
the deal — never leaves. Three things follow, and the third disables the module's own subject:

1. **A noteholder earns nothing but its discount.** It pays `priceFor(...)` per unit of face and
   receives exactly face. Whatever the borrowers paid in interest is not its.
2. **The residual has no holder.** When the notes are fully redeemed the vehicle still holds the
   un-run-off rows and every coupon it ever collected. Nothing ceases a vehicle whose pool has run
   off (`distribute` removes a deal only when the vehicle has already _ceased_), the party kind has
   no owner, no equity claim and no distribution, and `fails: ['cash','solvency']` will not fire on a
   party with positive equity. Appendix B: _"no residual with no holder"_.
3. **The attachment machinery goes inert.** `absorb` is the whole of C2/C6/D4:
   ```ts
   const pool = sum(
     deal.rows.filter(live).map((row) => ctx.register.quantity(deal.vehicle, row)),
   ).value;
   const notes = sum(deal.layers.map((id) => ctx.register.heldTotal(id).value)).value;
   let lost = sub(notes, pool, 'what the pool no longer covers');
   if (lost <= 0) return;
   ```
   Because interest is paid out as accelerated principal, `notes` falls faster than `pool`. Once
   `notes < pool` — which happens early and permanently — `lost ≤ 0` and **no loss is ever allocated
   to any tranche**, whatever the borrowers do. The junior/senior waterfall, the attachment points,
   the write-down leg and D4's senior losses are all downstream of a subtraction that has gone the
   wrong way round.

The module's header states the intended behaviour twice — _"it takes in what the borrowers pay and
passes it out by seniority"_, _"Σ tranche face equals the pool's face after every event"_ — and both
are true only if what is passed out is separated into interest and principal. Nothing separates them.

#### B-8 — the securitisation waterfall never allocates a loss (A)

Worklist 13e, **done**: _"a waterfall paying by seniority out of what was actually collected … with a
loss landing from the bottom and nothing stopping it reaching the senior."_

Per A-57: `absorb` computes `lost = notes − pool`, and because the pool's interest is paid out as
accelerated principal redemption, `notes` falls faster than `pool` and `lost` is negative from early
on. No loss is ever allocated to any tranche, whatever the borrowers do.

---

### For item 9. The seven private books, and Mandate

#### A-9 — the whole insurer sector cannot open a single policy (A)

`mechanisms/insurers/index.ts`. Four separate things, and each alone would be enough.

**1. The policy kind's unit does not exist.** Line 124:

```ts
unit: (ccy) => ccy as unknown as ReturnType<InstrumentKindProfile['unit']>,
```

It hands back the CURRENCY CODE (`"USD"`) where a `UnitId` is wanted. A currency's unit is
`currencyUnit(ccy)` = `"ccy:USD"`; the module declares a unit of its own, `COVER` (`"cover"`,
`perUnit: MONEY_PIECES`), and never uses it. `Instruments.add` calls `this.registry.unit(unit)`,
which throws `Missing [Appendix A] unit USD does not exist`. **No `policy` instrument can be
registered in any world.** The `as unknown as` is what got it past the compiler — the one cast in
the file, and it is casting away exactly the check that would have caught this.

**2. Nothing runs.** The module declares `phases: []` and `participants: []`. `quoteCover`,
`runCover`, `policyId`, `venueForCover` and `presentValueOf` are exported and called from nowhere in
the engine. §27 is a kind registry and an audit family over an empty set.

**3. `runCover` clears a book and settles nothing.** It opens the venue, posts the bids, calls
`clear`, and on `cleared` records an event and returns — `outcome.fills` is discarded. No policy is
issued, no premium is paid, no beneficiary is named. A session that strikes a price and moves no
money is the "cleared price nobody paid" the market runner deletes prints for (Clearing E1), here in
a venue that nobody checks.

**4. `derive` returns 0 where its own comment forbids it.** Lines 168-173:

```
// XI-6: a curve with nothing on it prices nothing. Missing is Missing — never a zero that would
// read as a liability the institution has discharged.
return priced.some ? priced.value : 0;
```

The comment says what must not happen and the next line does it. A policy whose curve has no points
values at nothing, so the insurer's largest liability vanishes and its equity jumps by the whole of
it — which is precisely "a liability the institution has discharged".

**5.** `claimsSeen` reads the insurer's OWN HOLDINGS for policies (`view.holdings()`, filtered by
`isPolicy`). A policy is the insurer's LIABILITY; the beneficiary holds it. So `written` is always
empty, `cover` is always 0, and `claimsSeen` always returns 0 — the experience half of A4.b's price
is structurally dead. It should read `view.instruments.issuedBy(self)`.

**Effect:** the model's largest holder of duration does not exist. B2.b is the clause §27 was built
for and the sector cannot write the promise the clause is about.

#### A-67 — nothing ever borrows a security: the whole securities-lending mechanism is unreachable (A)

`mechanisms/securities-lending/index.ts` builds the venue, the fee clearing, the title transfer, the
collateral pledge, the manufactured payment and the recall. Two exported functions are the only ways
in, and **both are called by nobody**:

```
$ grep -rn "runBorrows"    packages/engine/src   →  the definition, and nothing else
$ grep -rn "wantsToBorrow" packages/engine/src   →  the definition, and nothing else
```

The module's single phase is `borrow.economics`, which runs `manufacture` and `charge` — both of
which iterate `state(ctx).open`, the book of open borrows. Nothing ever pushes to it. So the phase
walks an empty list every period for the life of the world, and XI-11's _"title passes and the
economics do not"_ is built and never exercised.

The prohibition it exists to satisfy — _"no short without a borrow"_ — holds, but vacuously: there
is no short anywhere in this world either, so there is nothing for the borrow to be behind.

**Two further defects inside the unreachable code, which matter because they are what would run:**

1. **The fee is not cleared, it is the single bidder's reservation.** `runBorrows` loops
   `for (const w of wanted)` and clears a separate session per borrower, in which every lender posts
   `price: 'market'`:

   ```ts
   for (const s of supply)
     ctx.post(venue, { party: s.lender, side: 'sell', price: 'market', qty: s.units });
   ctx.post(venue, { party: w.borrower, side: 'buy', price: w.willPay, qty: w.units });
   const outcome = clear(ctx.posted(venue), 'proRata', 'sellersCompete');
   ```

   One bidder, and no seller with a level to compete on, so `outcome.price` is `w.willPay`
   regardless of how much paper is on offer. The docstring says _"Scarce paper is dear and abundant
   paper is cheap, and neither is a table"_ — as arranged, neither scarcity nor abundance can move
   the fee at all.

2. **Two `Want`s on one instrument double-post the lenders.** `world.post` appends and `postings` is
   cleared only at the top of a period, so the second iteration for the same instrument posts every
   lender's offer again into the same venue. The book then shows twice the supply that exists and a
   lender can be allotted twice what it holds.

**And the same non-rate appears here as in A-58.** `wantsToBorrow`:

```ts
willPay: div(owed, add(owed, equity, 'what funds it'), 'what a period of its own money costs'),
```

`owed / (owed + equity)` is the debt share of funding — a dimensionless ratio, not a per-period cost
— and it is the identical expression, with the identical comment, that `securitisation:priceFor`
uses to price a tranche. Two modules, one mistake, and in both of them the number the comment
describes is published every period by `banks/index.ts:publishCostOfFunds`.

#### B-2 — the insurance sector has no seed, no phase and no participant (A)

Worklist 13h, **done**: _"Closed with the INSURER built."_ COVERAGE marks nine `Insurers` clauses MET
against this file.

```ts
export function insurers(): SystemModule {
  return {
    id: 'insurers',
    spec: 'Insurers',
    requires: ['sovereign-curve'],
    instrumentKinds: [policyKind],
    partyKinds: [insuranceKind],
    curveFamilies: [],
    units: [{ id: COVER, name: 'units of cover', perUnit: MONEY_PIECES }],
    params: [],
    phases: [],
    participants: [],
    families: [promises()],
  };
}
```

No `seed`, so **no `insurance` party is ever created**; `phases: []` and `participants: []`, so
nothing it exports is ever called; and per A-9 the kind's `unit` returns `"USD"` where the registry
wants `"ccy:USD"`, so a policy could not be registered even if something tried. `promises()` — the
`names` family contribution — walks an empty set and reports green.

The sector the module is a careful and correct piece of design for (B2.b's dated promise discounted
at a market rate, the institution wearing the rate move) does not exist in this world.

#### B-3 — securities lending is claimed to clear a fee, and has no way in (A)

Worklist 13f, **done**: _"**Securities lending**: title passes and the economics do not … the fee
CLEARS in a book per line, the manufactured payment is read off what actually reached the borrower,
and the lendable pool is a read of who holds it free, which is what caps a short."_ Nine `MET` marks.

Per A-67: `runBorrows`, `wantsToBorrow` and `returnLoans` are exported and referenced nowhere. The
module's one phase walks `state(ctx).open`, which nothing ever pushes to. There is no short anywhere
in this world for a borrow to be behind.

#### B-14 — a finding was positioned into item 13h, 13h closed, and the finding was not done (B)

`seeds/foundation.ts`, on the derivative layer's list of who may hold a contract, said: _"13h is
where a fund holds derivatives on purpose — and where the one pass that re-marks a fund's claim on
itself is next opened (`docs/BUGS.md`, finding `13b-2`)."_ `docs/RECORD.md` (item 13b.1's entry)
confirms the placement: `13b-2` "to **13h**, folded into two steps there".

13h is **done** on the worklist. `TRADES_CONTRACTS` is still `[BANK, FIRM]`; no fund kind is on it,
and the pass that would re-mark a fund's claim on itself does not exist. The receiving item closed
without the step the positioning was for, and nothing anywhere says so: the record's entry for 13h
does not carry it forward, and the comment in the source went on naming a future that had already
passed and a file that had been deleted.

This is the failure mode of positioning as a protocol. A finding leaves the audit file by being
placed into an item, and from that moment nothing checks that the item ever did it — the finding is
out of the one place findings live and into a plan file that gets deleted when the item closes. Six
of Part II's thirteen findings (**B-1** through **B-8**) have the same shape read from the other end:
an item closed and the thing it was for was not there.

The comment is corrected in this change to say what is true. The finding itself is **unpositioned**:
whether a fund should hold contracts at all is `Fund Shares A3`'s question and it belongs with 13o
(asset managers with strategies), which is where a fund that takes a position on purpose first has a
reason to exist.

---

### For item 10. The corporate bond is issued

#### B-1 — `corporate.bond` is a kind, an id function and a covenant test, and nothing ever issues one (A)

Worklist 13f, **done**:

> _"**The corporate bond**: it can fail, it cross-defaults where a sovereign does not, it says where
> it ranks on the instrument the waterfall already reads, and it carries COVENANTS tested on the
> issuer's PUBLISHED accounts."_

`mechanisms/corporate-bond/index.ts` declares `CORPORATE_BOND`, `corporateBondId(issuer, n)`, a full
`InstrumentKindProfile` with `cashFlows`, `due`, `ranking` and cross-default, a covenant table and
the `covenant.test` phase. Searching the whole engine for a construction site:

```
$ grep -rn "kind: CORPORATE_BOND" packages/engine/src   →  nothing
$ grep -rn "CORPORATE_BOND"       packages/engine/src   →  only its own declaration and profile
```

**No corporate bond has ever been issued and none can be: there is no issuance path.** `testCovenants`
runs every period over an empty set. Three `MET` marks in COVERAGE (`Corporate Credit A1`, `B2`,
`B3`) cite this file.

The clause the module is for — a firm funding itself in a market rather than at a bank — has no
mechanism that puts a firm in it. `firms/index.ts:publishFunding` publishes what a firm is short of
and `banks` reads it; nothing reads it as a reason to issue paper.

#### B-13 — the three things a firm sector does, and this one does none of them (A)

Carried from `docs/VERIFY.md`'s third sweep, and it is the finding that outranks the rest of that
file. Measured over the real world:

|                                                |                                                    |
| ---------------------------------------------- | -------------------------------------------------- |
| parties that have ever taken delivery of PLANT | **0**                                              |
| loans in the world, period 6                   | **96**, against 9,006 firms and thousands of banks |
| companies that have ever published accounts    | **0**                                              |
| research estimates                             | **0**                                              |
| parties that have ever ceased                  | **0**                                              |
| instructions settled over three periods        | 269,139                                            |
| FX revaluation events                          | 1,051,429                                          |
| goods perished in store                        | 51,521                                             |

Nothing is ever built, almost nobody borrows, nobody reports, nobody dies — and the two largest
event streams in the world are revaluing foreign balances and rotting food.

Part I found four contributing causes that VERIFY could not see from the outside, and they are not
the whole of it:

- **nobody reports** — `reporting/publish` requires `isPublic`, and the shares a firm issues are
  held by household cells, so this one should fire; it is unexplained and is the best single lead;
- **nothing is built** — `firms/decide.ts:plan` returns early without an `earnings`-based
  `costOfCapital`, and `requiredOnEquity` needs a share PRINT, which needs a share session that
  cleared, which needs a bank's dealing desk on the other side (A-60);
- **almost nobody borrows** — a firm's own `credit.quoted` read is period-unbounded (A-33), and its
  input bids are inflated by A-34, but neither explains two orders of magnitude;
- **nobody dies** — `fails: []` on households and the treasury (A-36), and `failedWhy`'s solvency
  branch reads an equity account that A-39 is inflating every period.

The diagnosis of the first and third is the next piece of work, and it is not this read's.

---

### For item 12. Firm birth

#### A-17 — three of XI-15's five weight events never fire, and nobody in this world is ever born (A)

`world/cells.ts` implements the five: entry, death, promotion, split, merge. `CellEvents`
(`world/context.ts:384`) exposes `split`, `merge`, `weight` (entry/death/promotion), `reKey`, `die`.
Across `mechanisms/` and `seeds/` the callers are exactly:

| door           | callers                                                                           |
| -------------- | --------------------------------------------------------------------------------- |
| `cells.split`  | `labour/matching.ts:312`, `labour/matching.ts:405`, `households/lifecycle.ts:250` |
| `cells.reKey`  | `households/lifecycle.ts` (ageing)                                                |
| `cells.die`    | `households/lifecycle.ts` (after probate)                                         |
| `cells.merge`  | **none**                                                                          |
| `cells.weight` | **none**                                                                          |

So `entry` never happens: **no person is ever born in this world.** `age()` moves members from
cohort 0 into cohort 1 and nothing whatever moves into cohort 0. `die()` removes them at the top.
The population is monotonically non-increasing from the seed to the end of the run, by construction
and not as an outcome.

This is not "no birth rate" being respected (Appendix B forbids a _declared_ birth rate, which is
right). It is the mechanism that would produce births as an OUTCOME — a household's own decision,
with its own cause — being absent, and nothing naming it as absent. `lifecycle.ts`'s header names
what it leaves out and does not name this. Under Part II that makes it neither MISSING nor OUT OF
SCOPE but unstated, which is the one thing a clause may not be.

`merge` never firing is the other half of A-18 below: nothing in this world ever recombines two
cells that have become identical, so the cell count only ever rises.

---

### For item 15. Housing, the rest

#### A-53 — a household bids its entire income as rent, and its spending plan does not know rent exists (B)

`housing/index.ts:reservation`:

```ts
const income = view.outlook('income');
if (!income.some || income.value.expected <= 0) return undefined;
const per = view.registry.pieces(
  goodUnitOf(view, self.region),
  view.params.ratio(HOUSING_PARAMS.perMember(keyOf(self, 'cohort'))),
);
// Law 8: money pieces a member expects, over the PIECES of occupancy a member lives under
return div(income.value.expected, per, 'what a member would pay for the roof it lives under');
```

`bid × per = income.expected`. The whole of it. `ordersOf` posts that as a **single point**, and
`letIn` clears on `marginalBid`, so in any region where dwellings are short the print rises to the
bids and rent takes a household's entire expected income, every period, indefinitely.

The docstring argues the bid: _"THE MOST A TENANT WILL PAY is what it has, because the alternative
is nowhere to live."_ That is a fair reading of B1.a read from the other side. Two things do not
follow from it:

1. **Nothing on the household's side knows.** `households/consume.ts:spendPerMember` computes
   `wanted = income.expected + gap` and `demandOf` spreads the whole of it over the basket. There is
   no rent term anywhere in the households module. So the same expected income is committed twice —
   once as the rent bid, once as the grocery budget — and the only thing that stops it being an
   arithmetic contradiction is that `collect` runs before the goods session and drains the account
   first. The household is then structurally surprised every period, which feeds back through
   `confidence` and widens its cushion, which it cannot fund either.
2. **It is a point where every other bid in this world is a schedule.** Goods, paper and shares all
   post a curve (`rungsOver`, `rungsUpTo`, `levelsBelow`) precisely because Clearing A2 asks for one.
   A tenant posting one level at its whole income cannot express "I would take a smaller place for
   less", which is the substitution the venue exists to find.

The wear floor on the other side (`wearOf` = spoilage × print) is deliberate and stated, and it is
the honest half: the range the rent clears in has a real bottom and a top that is a household's
entire livelihood.

Same shape as A-40 in the same file: `letIn` discards the solver's `proRata` sell-side allocation
and re-matches owners and tenants itself in ask/bid order. Here the two totals do agree (the loops
consume exactly `outcome.volume` on both sides), so it is a duplication rather than a divergence.

---

### For item 16. Cross-border, the rest

#### A-36 — the treasury's immortality is unconditional where the kernel says it is conditional (C)

`registry/profiles.ts:119`:

```ts
{ id: TREASURY, representation: 'named', moneyIssuer: null, fails: [], borrows: true, depositClass: null, sovereign: true },
```

`world/failure.ts`'s header says what this is meant to be: _"XI-3's two exceptions — the central
bank, and **a treasury in its own money** — are named consequences of what they ARE rather than
omissions."_ And Appendix B lists _"sovereign in foreign money"_ among the things that must not be
immortal.

`fails: []` carries no condition. `failedWhy` asks `partyKind(kind).fails` and nothing else — there
is no currency in the question — so a treasury cannot fail on cash or on solvency in **any** money.

Unreachable today: `mechanisms/treasury/index.ts` derives its currency from
`registry.currencyOf(region)` at every issuance site, so no treasury in this world has ever issued
paper in a money it does not print. The declaration is what is wrong, not the behaviour: it states
an unconditional exception where the spec grants a conditional one, and the condition is exactly the
prohibition. `failedWhy` already has `ccy` in scope (`const ccy = ctx.registry.currencyOf(...)`) and
already discards it on the solvency branch.

#### B-9 — `docs/BUGS.md` contradicts its own header about the four countries (C)

Its header: _"Measured on the whole suite: 82 red of 662 **after item 13j gave this world four
economies**."_ Its section 2, in the same file: _"**The foreign countries are stubs.** Three of the
four countries are a central bank, a treasury and a bond line. There is no foreign economy … →
**13i**, which closed with the external accounts built and the foreign economies not."_

Both cannot be current. 13j is marked done and its row claims each of the four gets the same
construction from the same draw; section 2 says three of them are still stubs and points at the item
before it. One of the two is stale and the file does not say which. A-61 is a live piece of evidence
for the pessimistic reading: every central bank in the world remits its seigniorage to
`treasuries[0]`.

---

### For item 19. The polity

#### D-1 — a levy that fails is recorded and then forgotten: no arrears (`12d-5`) (A)

Measured in `estate.test.ts` at 14 periods of the rig world: **855 failed tax legs** from household
cells, and nothing else a cell posted ever failed. `treasury/index.ts` settles the levy and, when it
fails, adds it to `unpaid` on `treasury.receipts` — which is right (`Money E1`, `D3`: a payer that
cannot pay has not paid). But `unpaid` is a number in an event and **nothing carries it**: the cell
does not owe it next period, the treasury does not chase it, and the receipt is not short by it in
any account. A tax that failed is a hole between two balance sheets that only the journal knows
about.

The mechanism is **arrears** — a levy that is not paid becomes a claim the treasury holds on the
payer, ranking where the law says — and the law is this item: what a tax is, what happens when it
is not paid, and where the claim ranks in XI-8's waterfall are all fiscal policy with an owner
(Polity D1, D3). It is a claim like any other: an instrument with a named creditor and a named
debtor, carried until it is paid, written off, or ranked in an estate.

**It is the same shape as three findings from Part I and they should be built together**, because
one instrument and one door answers all four: **A-41** (an unpaid wage and an unpaid severance
leave no obligation anywhere and the record says nothing is owed), **A-20** (a failed estate
transfer becomes an unhandled throw because there is nowhere for the unpaid part to live), and the
observation under **A-39** that `world/failure.ts:stillOwed` only counts failures whose
`instruction.period === view.period`, so last period's unpaid amount is gone from the solvency test
too. This world records what did not settle and carries none of it.

#### D-2 — the central bank is a marginal price-setter in the sovereign book (`13b.1-10`) (B)

Its open-market desk closes a gap towards a 25%-of-line target by posting a MARKET order — a
quantity with no level — so in every sovereign session where it has a gap it bids at the top of the
book for a quarter of the line. An order with no level is a price-taker of a price the mechanism has
not yet produced (Clearing A4), and a big enough one is the marginal order that sets it. The quantity
limit is real policy (Central Bank C1) and the module correctly refuses to stand in any other market,
so this is NOT Appendix B's buyer of last resort — but it is more aggressive than any real
open-market operation, and it belongs here because what a central bank may and may not do to a price
is this item's subject, alongside its administered rate.

What it needs is the level at which its own reason stops: a schedule (Clearing A2), not a quantity.
Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an
unchanged book.

**Two of Part III's findings are in the same module and should be read with it**: **A-61** (every
central bank remits to `treasuries[0]`, in its own money) and **A-62** (a loss advances the
remittance window, so E4's "it stands there until income covers it" is not what happens).


### Item 14 — the polity: the plan, carried unchanged

### Design

---

### For item 20. Periodicity

#### C-2 — four things this world does every week that the world does not (carried from `SWEEP.md`; worklist **13k**)

> **The dividend is fixed (item 10); the other three are not, and they stay in 13k.** A board now
> declares on its own fiscal quarters with a record date and a payable date after it, and dividend
> payout legs fall from **8,538 of 61,788 instructions to 308 of 53,266** over forty periods. The
> **rating fee**, the **tax assessment** and the **buyback** are still weekly. The first two are
> genuinely periodicity and are 13k's remaining subject; the buyback is a PROGRAMME and needs item
> 14's `Process` — its kind is already in `register/corporate.ts` waiting for one.


Law 8 says the periodicity is part of the number. A weekly period is the resolution; it is not a
licence to do everything weekly. Re-verified at the source in this pass.

- **A dividend is declared and paid every week.** Period 5 settles 249,288 instructions and
  **162,615 of them are dividend payouts — 65% of everything the world does**. `equity.decide` is
  `cycle: 0, anchor: { after: 'firms.decide' }` and runs every period; `decideEquity` distributes
  `spare / patience` each time. A board declares with its results on a fiscal calendar; between the
  declaration and the payment the dividend is a LIABILITY and the share trades EX, which is why total
  return and price return are different numbers. There is no declaration date, no ex date, no record
  date, no payable date and no dividend liability. **The machinery is built and unused**:
  `reporting/fiscal.ts` has `quarterClosedBy`, `anchorOf` and `publishableOn`. A quarterly dividend
  is also a world that does what it does in about a third of the settlements.
- **A rating fee is charged every week.** 16,527 `pays X for its rating` instructions in period 5;
  `ratings.assess` is `cycle: 'anchor'`, every period, and calls `collectFees` each time. An issuer
  pays an issue fee once and a surveillance fee annually. Weekly billing makes the assessor's income
  a flow of the issuer's equity rather than a price for a service, which is the conflict Ratings A5
  exists to keep.
- **Tax is levied every week.** 7,778 `tax due from X` instructions in period 5. Payroll withholding
  is weekly and right; corporation tax is assessed on a fiscal period and VAT quarterly. Levying
  everything weekly removes the working-capital consequence of a tax bill, which is the thing a
  treasury and a firm both plan around. (See also **A-46**: the base is wrong as well as the
  cadence, and **A-63**: the consumption half of it collects nothing at all.)
- **A buyback is decided weekly.** `decideEquity` chooses between a dividend and a buyback each
  period out of this week's spare cash. A buyback is an announced PROGRAMME executed over time, and
  announcing it is the event.

---

### For item 21. The local repairs

#### A-24 — the household's own plan round-trips through `unknown` and drops what it cannot parse (C)

`decide` publishes its orders into a journal event as `Record<string, unknown>`; the participant
reads them back with `marketsIn`/`ordersFrom`, which re-validate from scratch:

```ts
if (price !== 'market' && typeof price !== 'number') continue;
if (typeof qty !== 'number' || qty <= 0) continue;
```

The Law 4 intent is right and worth keeping — one decision, one writer, read back rather than
recomputed. The implementation loses the type at the journal boundary and then handles the loss by
**silently dropping the order**. A cell whose plan wrote a row this pair of predicates rejects
simply does not trade that period, and nothing anywhere says so: no throw, no violation, no event.
That is a decision disappearing between the party that took it and the book it was for, which is
the one thing this round trip exists to prevent.

`ordersFrom` also drops any row with `qty <= 0` before `asQty` can complain, so the file's own
comment — _"through the one door that says a size is a count of pieces — and that throws if what it
published was not"_ — is only true for positive non-integers.

---

### For item 23. Measure

#### C-1 — 82 red of 662, by cause (carried from `BUGS.md`)

Measured on the whole suite after 13j. The count is not the finding; the six causes are, and they are
carried verbatim because each names where it goes.

| cause                                            | ≈ red | what it is                                                                                                                                                                                                                                                                                                                                                                                                                                                            | placed                                                                                                     |
| ------------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| **the rig has no firm in most lines**            | 20    | `RIG_PER_LINE` is 0, so twelve firms over sixty-two lines leave most empty and any test asking the draw for a mill is told "it drew 0". Measured: setting it to ONE takes the suite from 79 red to **139** — a rig with every line in it is a different small world, and a dozen files' assertions are written against the one it makes today. Those tests are not wrong, they are SPECIFIC.                                                                          | item **16**, as one bounded change: re-size the scale model and re-derive what every affected test asserts |
| **the foreign countries are stubs**              | 6     | see **B-9**: this contradicts the same file's header, and which of the two is current is not established                                                                                                                                                                                                                                                                                                                                                              | item **16**, after B-9 is settled                                                                          |
| **a bank that is insolvent is never resolved**   | 9     | `13b-12`: a bank published capital of −31,237,415,456 and went on making a market for twenty-three periods. Both triggers must exist and the resolution must name which fired (Banks Capital C1.a). Whether the solvency trigger is not reading the published position, or is reading it and the resolution is not being run, is **still unmeasured**                                                                                                                 | its own diagnosis, before **16**                                                                           |
| **an ETF cannot create**                         | 7     | `12d-8`: a creation delivers a slice of the book and a desk without the basket does not create. Nothing moves a share back to a desk, so `E3` runs one way and the premium — 0.28 of NAV, twenty-six times a period of carry — has nobody able to close it. The answer now EXISTS (securities lending), and wiring it is a BUILD not a fix: a module may not import another, so "whoever must deliver may borrow" needs a kernel door of the shape `termsOffered` has | with **A-67**: the borrow market has to be reachable first                                                 |
| **two banks, and everything that needs a third** | 4     | `research` wants coverage to VARY, `deposits` wants a class to split rather than cross, `dealing` wants a market to fail when the desks step back. With three banks and one listed line a count that should be an outcome has one value                                                                                                                                                                                                                               | item **16**, same scale-model question                                                                     |
| **singletons**                                   | 9     | `omo` remittance and run-off (see **A-61**, **A-62**); `raise`; `ratings` ageing; `equity-anchor`; `indices`; `tick`; `environment`'s crop; `treasury`'s receipts (see **A-46**); `world`'s year-long chain. Each needs reading on its own                                                                                                                                                                                                                            | item **16**                                                                                                |

**And the eight 13j cost.** Three measurements of the same suite: 74 red before 13j; **303** with the
rig opening all four countries; **82** with the rig opening one and the currency tests four. The 303
is a finding and not a bug — a dozen firms and three banks over four countries gives each a country
with no banking system its own depositors could fund, and `Seed D1` refuses exactly that ninety-four
times — so a world's count of countries is a RESOLUTION like its count of banks. The eight that
remain are all one shape: **a test that named the world it was written against**, in `currency`,
`spot-fx`, `omo`, `indices`, `opening-liquidity`, `bank-capital`, `deposits`, `money-market`, and a
long tail of sizes and totals. Positioned to **16**.

---

### For item 21. The local repairs (continued)

#### A-43 — the labour module builds the household's schedule, through the door the architecture says not to (C)

`MechanismContext.gather`'s contract (`world/context.ts:508`):

> _"ask every party whose module declared a schedule for this venue for one, and post what comes
> back. The module that OPENED the venue calls it … each schedule is built by the module that owns
> that party, with that party's own view. … **building somebody else's schedule inside the clearing
> phase instead is that module deciding for a party it does not own.**"_

`labour/matching.ts:supply` does exactly that: it walks `ctx.parties.ofKind(HOUSEHOLD)`, reads each
cell's own `outlook('income')` through `ctx.participant(cell)`, decides what that cell will work
for, and posts the order itself. The households module declares no `venueParticipants` and labour
never calls `gather`.

The door is not theoretical — two modules use it (`housing/index.ts:711` and `banks/index.ts:1165`
declare `venueParticipants`; `money-market/index.ts:269` calls `gather`). Labour is the one venue
where the seller's decision is taken by the buyer's market.

It is C rather than B because the number `supply` computes is a household's own read and no private
state leaks. What it costs is where the decision lives: a household's reservation wage is
`households`' subject (it is the same `income` outlook `spendPerMember` uses), and it currently
cannot be changed without editing the labour module. A-38's defect — the outside option being every
kind of money received — is in `labour/matching.ts` for exactly this reason.

---

### Carried so nobody tests it again

#### C-7 — the confidence question, answered and closed (carried from `VERIFY.md`)

Recorded because it is a hypothesis that was tested and **disproved**, and the next reader should not
test it again. Twelve periods, the same world:

| period | outlook rows | rows with confidence | contracts | loans |
| ------ | ------------ | -------------------- | --------- | ----- |
| 1      | 30,650       | 0                    | 0         | 6     |
| 2      | 64,803       | 0                    | 0         | 6     |
| 3      | 101,998      | 12,067               | 0         | 6     |
| 4      | 126,191      | 21,223               | 0         | 8     |

`width(surprises)` is zero for a party with one surprise or none, so confidence does not exist until
period 3 and then arrives in bulk. A fifth of all outlooks carry it by period four and the share is
rising, and still not one contract exists. **Confidence is not the gate and never was.**

(Part I found the one place where confidence IS load-bearing and dimensionally wrong: **A-65**, the
option premium, where it is multiplied by the price level instead of used as the width it is.)


---

## Part 4 — The index

### Every open finding, and the item that closes it

| finding | item | |
|---|---|---|
| **A-1** (B) | 2 | `reseat` books an issuer's whole liability against a PER-MEMBER equity account |
| **A-4** (B) | 4 | a claimed assembly guard on `exposedTo` does not exist |
| **A-5** (C) | 2 | `payable`, `cashFor` and `deliverable` take a unit and ignore it |
| **A-6** (C) | 2 | `labourParam` is declared in minutes and holds hours |
| **A-9** (A) | 9, 14 | the whole insurer sector cannot open a single policy |
| **A-10** (A) | 4 | the currency audit family compares a number against itself |
| **A-11** (C) | 4 | `flows` switches Money D3 off for a whole cell for a whole period |
| **A-12** (C) | 4 | a comparison in `names` that can never be true |
| **A-13** (B) | 4 | the population identity is checked for households and for nothing else |
| **A-14** (A) | 4 | that same household check is an identity that cannot fail |
| **A-17** (A) | 12 | three of XI-15's five weight events never fire, and nobody is ever born |
| **A-18** (A) | 2, 12 | the fraction of a person is discarded every period |
| **A-19** (B) | 2 | probate takes in every currency and pays out one |
| **A-23** (C) | 2 | `wealthOf` and `atRisk` add currencies |
| **A-24** (C) | 21 | the household's plan round-trips through `unknown` and drops what it cannot parse |
| **A-25** (C) | 5 | an unpriced physical leg is valued at zero inside an audit family |
| **A-30** (C) | 5 | numeric defaults where the discipline is `Missing` |
| **A-32** (C) | 2 | `levelsBelow` claims a refinement invariance its arithmetic does not have |
| **A-33** (B) | 2 | `lastOwn` has no period bound |
| **A-34** (B) | 5 | a firm with no wage history bids for inputs as if labour were free |
| **A-36** (C) | 16 | the treasury's immortality is unconditional where the kernel says conditional |
| **A-38** (B) | 2 | a cell's outside option is every kind of money it received |
| **A-39** (A) | 2 | the wage bill capitalised into inventory is bigger than the wage paid |
| **A-42** (A) | 4 | two `units` families switch off in any period with a weight event |
| **A-43** (C) | 9, 21 | the labour module builds the household's schedule |
| **A-44** (B) | 2 | the liquidity premium is declared over a deposit and used as an absolute rate |
| **A-45** (C) | 5 | a bank that owes nothing has a cost of funds of zero |
| **A-47** (A) | 2 | a fund's mandate has no currency in it |
| **A-48** (C) | 4 | `equityIsZero` cannot fail for the reason it says it checks |
| **A-50** (C) | 2 | `moneyOf` hand-builds an account id |
| **A-51** (A) | 2 | the kernel converts currencies and five module reads do not |
| **A-53** (B) | 15 | a household bids its entire income as rent |
| **A-54** (A) | 3 | `gather` is called by exactly one module |
| **A-55** (A) | 7 | nobody can buy a dwelling, so Housing B1–C4 never runs |
| **A-56** (A) | 7 | three lines have a firm, a recipe, a market and no buyer |
| **A-57** (A) | 8 | a securitisation vehicle keeps the whole interest stream, for ever |
| **A-58** (A) | 2 | the price a bank will pay for a note is a leverage ratio squared |
| **A-60** (A) | 3 | no bank makes a market, because no bank employs anybody |
| **A-61** (A) | 2 | every central bank remits to the same treasury, in its own money |
| **A-65** (A) | 2 | every option premium is a money-squared number |
| **A-66** (A) | 6 | eight of nine derivative books can never produce a first print |
| **A-67** (A) | 9 | nothing ever borrows a security |
| **A-68** (A) | 2 | loading a cargo writes off what the cargo cost |
| **A-69** (C) | 6, 21 | nine exported entry points that nothing calls |
| **B-1** (A) | 10 | `corporate.bond` is declared and nothing ever issues one |
| **B-2** (A) | 9, 14 | the insurance sector has no seed, no phase and no participant |
| **B-3** (A) | 9 | securities lending is claimed to clear a fee and has no way in |
| **B-5** (A) | 3 | the tenancy venue has never had an order in it |
| **B-6** (A) | 3 | item 9's dealers quote nothing |
| **B-7** (A) | 6 | the derivative layer has never produced a contract |
| **B-8** (A) | 8 | the securitisation waterfall never allocates a loss |
| **B-9** (C) | 16 | the four countries: header and section 2 contradict each other |
| **B-10** (A) | 4 | "checks green" has been satisfied by families that cannot fail |
| **B-12** (A) | 1 | 99 `MET` marks stand on mechanisms that have never produced anything |
| **B-13** (A) | 10 | the three things a firm sector does, and this one does none |
| **B-14** (B) | 9, 13 | a finding positioned into 13h; 13h closed; the finding was not done |
| **C-1** (—) | 23 | 82 red, by cause |
| **C-2** (—) | 20 | four things this world does every week that the world does not |
| **C-4** (—) | **0.2, 0.3, and items 10b–19** | the sectors that are not there — **promoted from a finding to the work itself**; its six rows are the absent sectors in Part 0 |
| **C-5** (—) | 1 | 12,519 declared parameters are never read |
| **C-6** (—) | 6 | the margin gate: 31,640 admission decisions, 31,640 refusals |
| **D-1** (A) | 19 | a levy that fails is recorded and then forgotten |
| **D-2** (B) | 19 | the central bank is a marginal price-setter in the sovereign book |
| **E-2** (B) | 21 | a `winding` estate is still a household cell to eleven readers |
| **E-3** (C) | 21 | an estate pays rent while it winds up and its own family reports it |
| **E-4** (B) | 21 | a capacity line produces into WIP and destroys the unsold part |
| **E-5** (B) | 15 | the state can only sell ground in the place it sits in |
| **E-6** (B) | 21 | an acquirer's consideration in a bank resolution is a missing mechanism |
| **E-7** (B) | 19 | a negative policy rate is real and this world cannot express one |
| **E-8** (B) | 2 | a declared price does not say which of the two scales it is in |
| **E-9** (B) | 2 | a dirty price adds two scales |
| **E-10** (B) | 2 | `Contract.struckAt` means a different dimension per kind — **never indexed before** |
| **E-11** (B) | 2a.2, 6 | `Outcome.price` is a `PerPiece` and some books clear a RATE — the subordinated raise, the money market, the IRS, the CDS. `E-10`'s shape at the clearing layer. **Found by the type at stage 2a.1** |

### Findings already closed, and where

Kept so that deleting `docs/AUDIT.md` does not lose the fact that these were done. `docs/RECORD.md`
carries each in full.

| closed in | findings |
|---|---|
| old item 2, Missing is Missing | `A-49`, and `A-30`'s second bullet |
| old item 4, `worthTo` | `A-27` |
| old item 5, `Receipt` | `A-37`, `A-40`, `A-46`, `A-52`, `A-63`; `A-57` diagnosed |
| old item 6, `View` | `A-31` |
| old item 7, `Lifecycle` | — (`A-17`, `A-36` carried out) |
| old item 8, `Agreement` | `A-20`, `A-26`, `A-41`, `A-62`, `D-1`'s store and four cases |
| old item 9, `Control` | `A-70`, `B-4` |
| old item 10, `CorporateAction` | `C-2`'s dividend |
| old item 11, `OutputKind` | `A-64` |
| old item 14, `Process` | `A-21` |
| old item 17, the local repairs | `A-2`, `A-3`, `A-7`, `A-8`, `A-15`, `A-16`, `A-22`, `A-28`, `A-29`, `A-35`, `A-59`, `B-9`, `B-11`, `B-15`, `C-7` |
| old item 18 | `C-3` (four central banks, four rates), and `E-1` |

### What the reads covered, and what they did not

**Part I** read all 59,927 lines of `packages/engine/src`, file by file, in dependency order, against
three questions: is it bottom up, is currency conserved, is the mechanism real. 70 findings.
**Part II** took every `done` row of the worklist and every `MET` in coverage and asked whether the
thing claimed is in the code and produces anything. 15 findings. **Part III** carried in three deleted
holding pens, de-duplicated. 7 findings. **Part IV** was item 14's own plan. 2 findings.

`docs/VERIFY.md`'s six lettered findings were all re-found independently in Part I —
F1→A-4, F2→A-5, F3→A-8, F4→A-7, F5→A-2, F6→A-3 — and are not repeated.

**What was NOT looked for.** Performance, style, test coverage, and anything in `packages/app`. Nor
was anything run: Parts I–III are a read of the source, and the two places that state what a run WOULD
do (`A-18`'s micro-cells, `A-66`'s books) say so and show the arithmetic instead of a measurement.

**And what no read could find, which is why Part 0 exists.** All four parts read `packages/engine/src`
and the claims made about it. **A sector that was never written leaves no trace in either**, so the
method that produced 94 findings could not produce the six in 0.2 — it carried them in from a deleted
file, as one row of `C-4`, filed under an item that was then marked done. The aggregate of
`docs/COVERAGE.md` is the read that finds them, it takes one command, and until item 1 it had never
been run.

---

## Appendix — the lessons this file exists to keep

1. **A `done` row and a `MET` mark are claims, and a claim nobody can falsify goes stale silently.**
   Six things were placed into items that closed without them, and nothing anywhere said so. Item 1
   is the check; Part 0 is the number it checks against.
2. **"Do not chase a finding" is right for a defect and wrong for an absence.** `C-4` was six missing
   sectors filed as evidence. A missing sector is an item.
3. **A finding leaves this file only by its item closing** — and the item that receives a placement
   must carry it in its own steps, not in prose. Every placement in Part 2 is a checkbox for that
   reason.
4. **Measure at the end of a module, never mid-item** (Law 11), and **run the whole suite, never the
   files you think are affected**. Item 17 of the old file cost this world its entire merchant fleet
   for four commits because a module declared beside the wrong neighbour dragged the assembly sort,
   and every per-item measurement missed it.
5. **A refactor that cannot change behaviour cannot reduce a failure count**, and saying "the same 79"
   is not a result. The 79 red are Law-11 incomplete-model checks waiting on mechanisms in items
   3, 6, 7, 9, 10b, 11, 13 and 14 — not on types.
