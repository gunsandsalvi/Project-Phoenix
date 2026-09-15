# Project Phoenix — the implementation

> The one ordered list of what is left (Law 10). Rewritten 2026-09-15 from a full read of every
> engine source file and every document; findings are in `docs/measurements/2026-09-15-review-findings.md`
> and each is positioned in Part 3. **Take the first open item. Tick steps as they close. When an
> item closes, delete its section and write `docs/RECORD.md`; re-mark `docs/COVERAGE.md` in the same
> commit.** A step names a file, the change and the test. *Delete* names the read that replaces it.
> `npm run check:opens` (item 0.17) runs at any time; the suite runs at the end of a module.

## Part 0 — The measured state (2026-09-15)

### 0.1 By spec system

`npm run check:existence` over `docs/COVERAGE.md`. **1,361 clauses: 888 MET (94 NEVER REACHED),
105 PARTIAL, 368 MISSING.** A MET mark means a module cites the clause; the read found MET rows on
code that cannot run (re-marked by the item named): Insurers A4/B1/B2 (14), Housing B1/C1/C3/C4
(12a), Freight A3/D2/D6 (0.11), Trade Credit D1/D4 (12a.3), Short-Term Debt B3.b/B4 (0.5, 12a.7),
Corporate Credit E5 (17.0), Banks Capital C2 (0.9), §42 A1–A6 (0f), Capital Programme A2/C1 (15.1),
Commodities Spot D3 (18.3). The table is the tool's; `--verify` holds this file to it.

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
| **Corporate Credit** | **13** | 8 | **41** | 0 | 62 |
| Sovereign | 40 | 3 | 8 | **3** | 51 |
| Short-Term Debt | 12 | 3 | 4 | 0 | 19 |
| Equity | 26 | 2 | 9 | 0 | 37 |
| Money Market | 25 | 3 | 0 | 0 | 28 |
| Spot FX | 26 | 1 | 0 | 0 | 27 |
| Fund Shares | 23 | 3 | 0 | 0 | 26 |
| Securities Lending | 15 | 0 | 6 | **14** | 21 |
| Prime Brokerage | 17 | 2 | 5 | 0 | 24 |
| Derivative Layer | 32 | 0 | 0 | **17** | 32 |
| CDS | 17 | 0 | 8 | **17** | 25 |
| IRS | 14 | 1 | 5 | **13** | 20 |
| FX Forwards | 13 | 0 | 8 | 0 | 21 |
| Commodity Futures | 5 | 0 | 15 | **5** | 20 |
| **Commodities Spot** | **3** | 1 | **20** | 0 | 24 |
| Indices | 21 | 1 | 0 | **1** | 22 |
| Banks Lending | 21 | 5 | 6 | 0 | 32 |
| Banks Funding | 28 | 4 | 0 | 0 | 32 |
| Banks Capital | 20 | 2 | 1 | 0 | 23 |
| Dealer Desks | 26 | 1 | 0 | **2** | 27 |
| Insurers | 9 | 0 | 14 | **9** | 23 |
| Hedge Funds | 13 | 2 | 9 | 0 | 24 |
| Private Equity | 12 | 1 | 12 | 0 | 25 |
| Treasury | 20 | 1 | 4 | 0 | 25 |
| Central Bank | 22 | 3 | 4 | 0 | 29 |
| **Polity** | **0** | 0 | **32** | 0 | 32 |
| Firm | 20 | 7 | 3 | 0 | 30 |
| Capital Programme | 22 | 3 | 0 | 0 | 25 |
| Firm Birth | 7 | 6 | 12 | 0 | 25 |
| M&A | 13 | 0 | 9 | **7** | 22 |
| Trade Credit | 8 | 3 | 11 | 0 | 22 |
| Goods | 27 | 2 | 10 | 0 | 39 |
| Freight | 17 | 3 | 0 | 0 | 20 |
| Labour | 23 | 1 | 3 | 0 | 27 |
| **Housing** | **6** | 1 | **19** | **3** | 26 |
| Households | 20 | 4 | 9 | 0 | 33 |
| **Small-Business Pools** | **4** | 3 | **21** | 0 | 28 |
| **Cross-Border** | **6** | 2 | **18** | 0 | 26 |
| Ratings | 16 | 3 | 4 | 0 | 23 |
| Reporting | 32 | 5 | 1 | 0 | 38 |
| Observer | 16 | 0 | 10 | 0 | 26 |
| Expectations | 17 | 2 | 8 | 0 | 27 |

### 0.2 The world does not open — seventeen stops

Nine found by assembling the rig and patching each throw; eight found by reading, each the first
time its path runs. All are item 0.

| # | stop | where | item |
|---|---|---|---|
| 1 | `shortTermDebt.line` declared `shape` with a death | `short-term-debt/index.ts:792` | 0.1 |
| 2 | `smallFirm` an operational depositor with no bank choice | `small-business/index.ts:255` | 0.3 |
| 3 | `funds.manager` anchors to `funds.strike`, declared after it | `funds/index.ts:2331` | 0.2 |
| 4 | `paper.backstop` lands behind cycle-2 siblings; also runs AFTER the maturity it should fund | `short-term-debt/index.ts:841` | 0.5 |
| 5 | small-business seed runs before the banks exist | `small-business/index.ts:271` | 0.4 |
| 6 | one world-level cell key refuses a second cell population | `registry/registry.ts:141` | 0b |
| 7 | fund manager party added once per pool | `funds/index.ts:2571` | 0.7 |
| 8 | `banks.raise` reads this period's reservations before they exist | `banks/subordinated.ts:1324` | 0.8 |
| 9 | fractional staff hours at period 23 | `banks/staff.ts:148` | 0.8 |
| 10 | no `good.<x>.transit.<a>.<b>` instrument exists anywhere: no cargo has ever loaded | `freight/index.ts:323` | 0.11 |
| 11 | a raise at `reservation: 0` is dropped by `onTheGrid`: no raise ever enters a book | `banks/subordinated.ts:1277`, `clearing/market.ts` | 0.9 |
| 12 | any period-0 overdraft throws in `bookDraws` (`costOfFunds` is `none`) | `banks/index.ts:920` | 0.10 |
| 13 | a tender with a cell seller throws at `asQty(units / weight)` | `control/index.ts:538` | 0.13 |
| 14 | the insurer delivers policy units it was never issued | `insurers/index.ts:548` | 0.12 |
| 15 | a loan drawn on 29 February | `banks/loan.ts:688` | 0.14 |
| 16 | deposit insurance limit is $100 per member → `couldLeave ≈ base` → funding room < 0 → **no loan after period 0** | `money-market/index.ts` `insuranceLimit` | 0.15 |
| 17 | cost of capital needs a quote that needs a project that needs a cost of capital → **nothing is built** | `firms/invest.ts costOfCapital` | 0.16 |

Outside the rig: 6 banks/60 firms throws `share.etf.us already exists` (21.6); the rig opens one
country so the FX layer is exercised only by `abroadWorld`.

### 0.3 What the patched rig does (3 banks, 18 firms, 12 cells, 146 markets, ~340 sessions/period)

| period | ms | cleared | noDemand | noSupply | noOverlap | events | parties |
|---|---|---|---|---|---|---|---|
| 1 | 232 | 2 | 326 | 2 | 9 | 889 | 44 |
| 10 | 182 | 0 | 318 | 11 | 9 | 1,833 | — |
| 20 | 329 | 0 | 319 | 15 | 14 | 2,910 | 302 |

- Nothing trades, in order of weight: stops 16 and 17; every bank's required yield is one number
  for every name (`banks/index.ts:1738`, BK3) so every desk's view of every bond is the same curve;
  a household offers labour only after a non-wage receipt (H1); half of every cell's spare is bid
  every period for a dwelling nobody builds (H4); every fund is wound up at `patience` (FD10).
- Parties 44 → 302 and never fewer: cells split on every partial event and `sameState` merge
  cannot fire (R2). The audit total climbs with each split (A1).

### 0.4 The architecture, section by section

| ARCHITECTURE § | verdict | evidence → item |
|---|---|---|
| 1–2 numbers | holds; wrong cost | `Measure<D>` allocates per op; three hand-rolled tolerances (EX2, RP8, H5); a dust band on an integer (W4) → 0g, 21 |
| 3 units | **does not hold for currency** | `Cash` erases the currency (K2) → 16.0 |
| 4.1 money | holds | — |
| 4.2 the wire | two holes | create-with-destroy not enforced (K9); `sold` keyed per seller (K8); no atomicity across books (FX3) → 21, 16.5 |
| 4.3 register | holds; wrong cost | lots never coalesce; `holdingsOf` snapshots; 14 modules walk `instruments.all()` → 0g |
| 4.4 parties/cells | **wrong design** | cannot merge, cannot trade cell↔cell, contradicts Part XII (R2, M3, W5, H2, A1) → 0f |
| 4.5 prices | one gap | a print has no dimension; rate-quoted marks ×100 (IR6) → 18.0 |
| 4.6 clearing | three gaps | offer at no level dropped (M1); negative level refused for time (K6, V1); markets never close (CP4, CF2, CD1) → 0.9, 18a.4, 18.4 |
| 4.7 calendar | modules count periods | FD12, CD2, SL5, OM5 → 0g.13 |
| 4.8 period loop | **claim false** | an unproduced read returns last period's, never throws; order by anchor at insertion (WK1) → 0a |
| 4.9 audit | one contradiction; wrong cost | units family fires on every split (A1); prices family vs valuer (A2); five full passes (A3) → 0f, 0g |
| 4.9b kernel/modules | **wrong design (admitted)** | fourteen single-answer hooks; only one required at seal (W6, A7); registry reads a module event (CO4); observer imports four modules (OB5); modules read each other by event name (BK4) → 0e |
| 4.10 registry | not in `seeds/`, `*/data.ts` | S3, SB2, FR5, CO5, EN3 → 0c.7 |
| 4.10a module state | journal used as a store | H7, H9, RP10, CD5, FD3 → 0e.3 |
| 4.11 observer | wrong cost, wrong dependency | OB1–OB8 → 0g.12, 21.14 |
| 5 errors | `core/measure.ts` throws `RangeError` | K1 → 21.10 |
| 9 lint | spellings, not rules | `Math.*` in 11 mechanism files; `atLeast(x, 0)` floors in 7; literals in data files → 0c.7 |

### 0.5 What is missing as an economy (no clause names it; the item builds it)

| gap | why it cannot emerge today | item |
|---|---|---|
| growth: no learning, no selection, no productivity process; the world decays by construction | fixed recipes, drawn productivity, population monotone down | 12c, 22 |
| a price level: no ask from an outlook, no wage-price loop, no rate rule | F15, H8, 18a | 12b.3, 12d, 18a.1 |
| a credit cycle: lending shut; credit-blind view; constant LGD; no collateral channel | 16/17 stops, BK3, BK7, HO1, LD3 | 0.15, 0.16, 17.0, 12a.4, 15.1 |
| unemployment as a state with duration | employment re-matched every period (BK26) | 12b, 0f |
| one propensity to consume; no liquid/illiquid wealth | one rule per cell | 0f |
| information diffusion: public prints are not observations | H8 | 12d |
| the government buys nothing | outlays are transfers and wages | 19.0 |
| input substitution: one recipe per line | Leontief only | 22 |
| demography: two cohorts, no births | stubs | 0f, 12.3, 14.6 |
| fixed costs and batches at the firm (operating leverage) | linear scaling | 22 |
| external shocks: three stub countries | 16 | 16 |
| readers of the environment (insurer, central bank, heating) | EN2 | 14.2, 18.8, 12d |

### 0.6 The documents

Three numbering systems (0c.1); two completion figures (0c.2); 98 COVERAGE rows cite `B-12` which
does not exist (0c.3); twelve docstrings describe mechanisms the code lacks (0c.6); households
"nobody lends to it" against spec Households E and Housing C — the spec wins (12a); XI-15 "split on
partial event" contradicts Part XII "one cell per key" — resolved by 0f.

---

## Part 1 — The order

| # | item | why here |
|---|---|---|
| 0 | The world opens | seventeen stops; repairs only |
| 0a | Phases ordered by what they read and write | two stops are the anchor design; every module after this declares reads |
| 0b | The cell key belongs to the party kind | stop 6; 0f needs it |
| 0c | One truth in the documents; the guards that bite | before any item closes on the new plan |
| 0d | The overdue suite run, triaged | the measurement eleven modules owed |
| 0e | Questions, not hooks; stores, not events | the module contract every sector item after it uses |
| 0f | The population lattice (cells hold totals) | the representation every mass sector stands on; must precede the columnar state |
| 0g | The core made fast | after 0f (the register's cell half is final); a year in minutes before anything is measured at scale |
| 11 | Small-Business Pools | on 0f |
| 12 | Firm birth, household formation, promotion | entry into the pool |
| 12a | Households borrow, owe and fail; arrears; the immortals | the arrears row that housing, treasury and the sovereign wait on |
| 12b | Employment is a standing relation | duration; no absorbing state |
| 12c | Productivity is an outcome | growth |
| 12d | Observation | diffusion; the price level's second half |
| 14 | Insurers and pensions | a buyer, a claim, a schedule, capital |
| 15 | Housing and land, the rest | after 12a |
| 16 | Cross-border, the rest | 16.0 money carries its currency |
| 17 | Corporate credit, the rest | 17.0 the credit view |
| 17b | The leveraged buyout | after 17.9 |
| 18 | Commodities spot and futures | 18.0 a print carries its dimension |
| 18a | Monetary policy | before the polity |
| 19 | The polity | 19.0 the government buys |
| 20 | Periodicity | after 19 |
| 21 | The local repairs | each when its file is open |
| 22 | The recipe | recipes plural; batches; upkeep |
| 22a | The opening is not an equilibrium | needs 12, 17, 18a |
| 23 | Measure | after everything |
| 24 | The app and the APK | last |

---

## Part 2 — The items

## 0. The world opens

Repairs only; each the smallest change that removes the stop at its cause. No design.

- [ ] 0.1 `short-term-debt/index.ts:792` `PAPER_PARAMS.line`: `kind: 'placeholder'`.
- [ ] 0.2 `funds/index.ts phases[]`: move `funds.strike` above `funds.manager` and `funds.capital`. 0a deletes the constraint.
- [ ] 0.3 Move `firmChoosesBank` + `board()` from `firms/bank.ts` to `registry/switching.ts` as `chooseBankBySwitchingCost(view, costParam)`; `firms`, `small-business`, `households/bank.ts`, `funds/bank.ts`, `ratings/bank.ts` declare `bankChoices: [{ partyKind, chooses: v => chooseBankBySwitchingCost(v, THEIR_COST) }]` and delete their copies (one mechanism, five preferences).
- [ ] 0.4 `small-business/index.ts:230` `requires: ['firms', 'banks', 'seed.foundation']`.
- [ ] 0.5 `short-term-debt/index.ts:841` `paper.backstop`: `cycle: 2`, `anchor: { before: 'corporateActions' }`. Test: an issuer with maturing paper, no cash and a line draws and repays; no `credit.default`.
- [ ] 0.6 Delete after 0b: the small-business seed adds no cell until the key is per kind.
- [ ] 0.7 `funds/index.ts:2565–2578`: managers added once per HOUSE (`new Set(decls.map(d => d.manager))`); no `parties.has` guard.
- [ ] 0.8 `ParticipantView.requiredOf(issuer): Option<Ratio>` in `world/context.ts` = `journal.lastOf('bank.reservation', …)` keenest ever published for the name; delete the scans in `banks/subordinated.ts:1324`, `corporate-bond/index.ts:296`, `short-term-debt/index.ts:313`. `runRaise` on `none` records `bank.raise.unpriced` and returns. `banks/staff.ts:148` `qty: downTick(hours)`; remainder journaled `bank.staff.short`.
- [ ] 0.9 `PrimaryOffer.reservation: Option<PerPiece>`; `runMarket` treats `none` as a market sell; `clearing/market.ts onTheGrid` journals `order.dropped` (public: party, level, size, why) for every discarded order. `banks/subordinated.ts:1277` passes `none()`. Test: a raise with no reservation clears against one bid.
- [ ] 0.10 `banks/index.ts overdraft()` refuses with `bank.overdraft.unpriced` when `costOfFunds` is `none`; delete the `Missing` throw in `bookDraws`. Test: a period-0 overdraft is refused; the world steps.
- [ ] 0.11 `goods/index.ts` seed: for every portable good and every ordered region pair with a leg, `instruments.add` `good.<sub>.transit.<from>.<to>` (physical, carried at cost, no market, the good's unit). Test: a cleared freight session loads cargo, pays freight, `freight.arrived` fires with the units that left.
- [ ] 0.12 `insurers/index.ts:548 runCover`: an `issue` leg to the insurer precedes the `asset` leg in the same instruction. Test: one cover fill settles.
- [ ] 0.13 `control/index.ts:538`: fill rounded down to the seller cell's grain, remainder journaled `tender.unfilled`. (0f deletes the grain.)
- [ ] 0.14 `banks/loan.ts:688` `maturity = calendar.addMonths(drawn, 12)`.
- [ ] 0.15 `money-market/index.ts` `insuranceLimit` `value: 250_000` (named units). `test/params.test.ts`: every `denominated` money param within two orders of magnitude of the seed's per-member opening deposit, read from the register after the seed.
- [ ] 0.16 `firms/invest.ts costOfCapital`: read, in order, the firm's outlook on `funding`; else `lastOf('credit.quoted', firm)` at any period; else `requiredOf(firm)` (0.8); else the sovereign curve print at the project's horizon. Delete the recency gate. Test: a firm never quoted invests when a project clears its hurdle at the curve.
- [ ] 0.17 `test/opens.test.ts`: `rigWorld('opens')` and `abroadWorld('opens')` step 30 periods without throwing; print per period `cleared / noDemand / noSupply / noOverlap / events / parties / cargoLoaded / loansWritten / ms`. `"check:opens"` first in `npm run check`. `CLAUDE.md` "Working here": *`check:opens` may run at any time and must be green before any commit.*
- [ ] 0.18 Record: the seventeen stops, the item that introduced each, the census.

**Exit.** `check:opens` green on both worlds; cargo loads; a loan is written after period 1; a firm invests; no bound, default or catch added.

---

## 0a. Phases ordered by what they read and write

`World.addPhase` (`world/world.ts:1237–1284`) resolves anchors at insertion and appends `after` siblings; ARCHITECTURE 4.8's "an unproduced read throws" is false (`lastOf` returns last period's).

- [ ] 0a.1 `PhaseDecl` gains `reads: readonly Dependency[]`, `writes: readonly Dependency[]`, `Dependency = { kind: 'event', name } | { kind: 'print', family } | { kind: 'store', noun }`; `cycle` and `anchor` deleted.
- [ ] 0a.2 `world/assemble.ts`: collect every module's phases; order by dataflow (a phase after every writer of what it reads; ties by module order, then declaration order); a phase's cycle = the latest cycle among its writers' phases (kernel phases keep theirs). A read with no writer and a cycle throw `Forbidden 'Law 10'` naming both sides. `World.addPhase` takes the resolved list.
- [ ] 0a.3 `MechanismContext.journal`/`prices` reads check the running phase's declaration; an undeclared read throws `Forbidden 'Clearing F1.a'`; a declared read whose writer has not run this period throws `NotYetProduced`.
- [ ] 0a.4 Every module's phases declare reads and writes (inventory: the per-module read lists in the findings file). Delete 0.2's reorder and 0.5's stated cycle; delete the three "order matters" comments in `seeds/foundation.ts` and the `written === period` guard in `environment/index.ts:484` (`step` asserts one run per phase per period).
- [ ] 0a.5 `test/phases.test.ts`: later module anchors before an earlier one's; forward dependency within a module; cycle-2 sibling case; undeclared read throws; unproduced read throws; a cycle names its members. ARCHITECTURE 4.8 rewritten; the observer prints the derived order.

**Exit.** Order is a function of declared reads and writes only.

---

## 0b. The cell key belongs to the party kind

- [ ] 0b.1 `PartyKindProfile.cellKey: readonly CellKeyDimension[]` for `representation: 'cell'` kinds; the registry throws if a named kind declares one or a cell kind none. Household dimensions declared in `households/index.ts`; `SMALL_FIRM` declares `['region', 'bank', 'line']`.
- [ ] 0b.2 Delete `RegistryData.cellKey`; `cellKeyFaults` and `Parties.sameKey` read the kind; `sameKey` across kinds is false; every `registry.cellKey` read (`grep -rn "\.cellKey" src`) goes through the kind; `cells.reKey` checks against it.
- [ ] 0b.3 Tests: two cell kinds keyed differently assemble; a household cell carrying `line` is refused; delete 0.6.

**Exit.** Item 11's seed adds its cells; households unchanged.

---

## 0c. One truth in the documents; the guards that bite

- [ ] 0c.1 `docs/WORKLIST.md` rows 13k–17 → one row per Part 1 item (same ids, `open`, pointing here); `docs/plan/manifest.json` and `tools/plan-progress.ts` count THIS file's ticked steps; `docs/PLAN.md` progress = *"N of M items closed; check:existence: X MET / Y PARTIAL / Z MISSING / W absent"*, nothing else.
- [ ] 0c.2 `docs/COVERAGE.md`: delete the `B-1 to B-8 and B-12` sentence; every `NEVER REACHED` → `UNMEASURED`; `tools/coverage-reached.ts`: a module that produced a public event of its own kind in a 52-period run is REACHED (used at 0d.3).
- [ ] 0c.3 `docs/RECORD.md`: prepend to entries 10b–11.4, 13c, 13d, 13e, 13h, 13m: *closed on a world that did not assemble or on a mechanism found unreachable (Part 0); measurements void; re-verified at 0d.*
- [ ] 0c.4 `docs/ARCHITECTURE.md`: `check:opens`; the phase dataflow (0a); the per-kind key (0b); the lattice (0f, written when 0f closes); the observer's read-only contract; `Cash` erases the currency (until 16.0).
- [ ] 0c.5 Twelve stale docstrings rewritten or deleted with the code they describe: freight storm loss (`freight/index.ts` header); `ledger/instruction.ts` create-with-destroy; `kinds.ts:401`, `module.ts:226` "assembly refuses"; `short-term-debt` header "a bank issues"; `cds/participants.ts:643`; `households` "worklist 9"; `estate/index.ts:608` `standsInFor Process`; `environment/index.ts` Insurers B4 reader; `securitisation/index.ts:940`; `registry/environment.ts:184`; `goods/data.ts:1936`.
- [ ] 0c.6 `tools/check-forbids.ts`: `Math\.(floor|round|ceil|abs|exp|pow|sqrt|min|max)` outside `core/`; `atLeast\(` / `atMost\(` with a literal `0` or `NO_QTY` outside `core/num.ts`; numeric literals in `mechanisms/**/data.ts` and `seeds/**` except 0, 1, -1, 2. Each hit becomes a 21 step until gone; the check fails on any new one.
- [ ] 0c.7 `params.ts:namesAnItem` regex → `\bitem\s+[0-9]`; `check:deaths` reads step ids of this file.

**Exit.** `check:existence --verify`, `plan:check`, `check:deaths`, `check:forbids`, `check:spec` green; one figure in PLAN.md.

---

## 0d. The overdue suite run, triaged

- [ ] 0d.1 `npm run test` once; output to `docs/measurements/2026-suite.txt`.
- [ ] 0d.2 Every red file into exactly one of: (a) asserts a world that no longer exists → 23.1; (b) a mechanism its item has not built → a step in that item naming the test; (c) a build-stopper → a 21 step with file:line; (d) an assertion wrong on its own terms → fixed here. The list is written here before anything changes.
- [ ] 0d.3 `coverage-reached.ts` over 52 periods of both worlds; COVERAGE re-marked; 0.1 regenerated.
- [ ] 0d.4 Record: reds by cause; reached by module.

---

## 0e. Questions, not hooks; stores, not events

Fourteen `SystemModule` single-answer hooks, each a kernel change; only `bankChoices` required at seal; modules read each other by event name; the journal is used as a store.

- [ ] 0e.1 `registry/questions.ts`: `Question<Arg, Answer> = { name, scope: 'partyKind' | 'world', required }` declared by a module; `SystemModule.answers: { [name]: (ctx, ...) => Answer }`; assembly collects by (question, kind), refuses two, requires one at seal for every `required` question a registered kind's profile names. Replace: `outlooks`, `marks`, `creditDecisions`, `bankChoices`, `termsOffered`, `borrowNeeds`, `tradingLimits`, `leverageLimits`, `riskBearing`, `resolves`, `clearingCapacity`, `venueParticipants`, `indices`, `curveFamilies`. Delete the fourteen `provide*` methods and maps in `world/world.ts` and `requireCreditDeciders`.
- [ ] 0e.2 Cross-module facts are questions or registry reads, never event names: `credit.request` (one kind, written by `ctx.request({ short, security, ccy })`; `banks/index.ts:1583 runRequests` reads it; `firms.funding`/`housing.funding` readers become it); the rented room (`registry/physical.ts rentedRoom` reads an agreement row, `commodities` writes one); the wage a place pays (`registry/wages.ts`; `research/index.ts:837`, `banks/staff.ts wageFacing` read it); the environment condition (`registry/environment.ts`). Guard: `grep -rn "ofKind('\w\+\.\w\+')" src/mechanisms` names no kind another module writes; add to `check-forbids`.
- [ ] 0e.3 The journal is a log: `expectations` book and surprises (H7), `households.plan` (H9), `ratings published` (RP10), `cds settledNames` (CD5), `funds asked/paid` (FD3), `insurers claims` (IN2), `moneyMarket printed` (MM13), `bank.dealing` (BK31) each become a declared noun with a store; the event is written from the store once, publicly, and never read back by its writer. `registry/nouns.ts` recounted.
- [ ] 0e.4 Observer imports no module: `research consensusOf` and `fx-derivatives hedgedResidual` become registered `measures`; `OCCUPATION_OF`/`OCCUPATIONS` move to the registry (OB5).
- [ ] 0e.5 ARCHITECTURE 4.9b, 4.10a rewritten. Test: a kind whose profile needs a question with no answer refuses to seal; two answers refuse.

---

## 0f. The population lattice

Replaces XI-15 as implemented (per-member holdings, split on every partial event, merge on
identical state) with a discretised heterogeneous-agent distribution carrying integer mass.
Households and small firms are two PROFILES on one kernel store. Sources: Aiyagari 1994,
Krusell–Smith 1998, Kaplan–Moll–Violante 2018 (the distribution is the state); Kaplan–Violante
2014 (liquid vs illiquid wealth → MPC heterogeneity); Deaton 1991, Carroll 1997 (buffer-stock as a
threshold rule); Mortensen–Pissarides 1994, Kroft–Lange–Notowidigdo 2013 (spell duration);
Mian–Sufi 2011 (collateral channel); Evans–Honkapohja 2001, Malmendier–Nagel 2011 (experience-
weighted adaptive expectations); Brock–Hommes 1997 (predictor switching); Hopenhayn 1992, Melitz
2003, Axtell 2001 (firm selection and the Zipf tail); Stiglitz–Weiss 1981, Petersen–Rajan 1994,
Gertler–Gilchrist 1994 (SME rationing and relationship lending); Kiyotaki–Moore 1997 (trade-credit
chains); Evans–Jovanovic 1989 (liquidity-constrained entry); Caiani et al. 2016, Poledna et al.
2023 (stock-flow-consistent agent-based benchmarks).

**Design decisions (binding).**
- A cell holds TOTALS in whole pieces and an integer weight. `perMember(party, instrument): number` is a read. No per-member leg, grain, divisibility invariant or `sameState`.
- A cell's identity is its KEY on a declared lattice: categorical dimensions (region, bank, cohort band, tenure, employment state, credit record; for small firms also line, age band) each owned by the event that moves it; quantity dimensions (liquid wealth in weeks of expected income, illiquid wealth, leverage, spell length; for small firms size, leverage) with band edges declared per kind as RESOLUTION and tested by invariance. At most one live cell per key, kept by the kernel: a move onto an occupied key merges.
- Movement is the five weight events only. A weight event moving `n` members moves `floor(total × n / weight)` pieces of every holding and equity; the remainder stays. At the close of `revaluation` the kernel reads each cell's quantities against its kind's band edges and moves the members that crossed. Categorical moves come from the owning event (hire/separation, cohort date crossing, foreclosure, default, promotion, death, entry).
- Decisions are threshold rules on the kind's profile, evaluated per cell from its own state and outlook. Preferences per cell: `memory` (exists) and `patience` (weeks of buffer), both drawn at entry from declared widths. No coefficient, hazard or stochastic process anywhere.
- Costs, declared: within-band dispersion (measured by the ladder); moves in whole members; no within-cell network.

**Steps.**
- [ ] 0f.1 `register/register.ts`: delete `copyMemberState`, per-member credit/debit, `sameState`, the per-member divisibility check; add `perMember`; `merge(into, from)` adds totals and weights.
- [ ] 0f.2 `ledger/instruction.ts`, `ledger/settlement.ts`: delete `fromCell`/`toCell`, `validateCellSide`, `cellSide`, `shareFor`, `totalFor`, `commonGrain`, `splitOnTick` across members; `pairFills` steps on the unit's piece; `world/actions.ts cellPays` FORBID deleted.
- [ ] 0f.3 `registry/lattice.ts`: `LatticeDecl { kind, categorical: { dim, movedBy }[], banded: { dim, quantity: read, edges: ParamId[] }[] }`; `PartyKindProfile.lattice`; `cellKey` (0b) = the lattice's key. Household and small-firm declarations as data with `why` per edge set.
- [ ] 0f.4 `world/cells.ts`: the five events as above; `crossings(kind)` at the close of `revaluation` (one writer of position); delete every `split` call in `labour/matching.ts`, `households/lifecycle.ts` (`age` re-keys by date into the standing cohort cell; `die` moves to the standing probate cell of the key), `households/index.ts decide`.
- [ ] 0f.5 Every mechanism site that scales a per-member quantity by `weightOf` deletes the scaling (`grep -rn "weightOf\|scaleQty(.*weight" src/mechanisms`): housing `owned/needs/collect/shortOfMoney/charge`, `money-market/deposits.ts`, `equity/index.ts:366`, `estate/index.ts:1345`, `treasury/index.ts` transfers, `households/*`, `labour/*`, `goods/inventory.ts perish` (one destroy leg per (good, cell); the `charge === undefined ? 0` default deleted).
- [ ] 0f.6 `audit/families/units.ts`: the invariant is one live cell per (kind, key); `flows.ts` per-member branch deleted; `households consumptionIsBought` tolerance = derived dust only.
- [ ] 0f.7 Household profile (`households/profile.ts`, replacing `consume.ts`, `portfolio.ts`, `lifecycle.ts` decisions): consumption = the cohort's quantity basket + (liquid − target) when positive, less debt service and rent due; `target = patience × confidence × expected income`; below target the basket only, cut in the basket's priority order when cash is short. Portfolio: what stands above target goes to fund/bill/deposit by comparing three public prints against the cell's own required liquidity. Labour: hours at a reservation read from spell band, cohort and the region's printed wage; participation reads the wage against the basket (delete the `benefit`-outlook gate at `households/index.ts:340`). Borrowing: `credit.request` when short of a roof or the basket. Life cycle: cohorts in five-year bands by date; retirement switches to drawdown; death to probate and the heir cell (cohort below, same key). Delete `homeBid`'s half-of-spare rule (a home request goes through borrowing). `households.patience` declared.
- [ ] 0f.8 Small-firm profile (`small-business/profile.ts`): production at the cell's drawn productivity through the goods venues of its region (shared `ordersFrom` with firms: F4); ask from its outlook on its sell price; inputs on terms (`termsOffered` for `borrowsOnTerms: true` kinds); hours in the labour venue at the wage its last sales cover; retained earnings buy plant at the cell's hurdle; owner draws to the owner household cell (an ownership ROW per small-firm cell); `credit.request` to the bank of its key; trade credit cell↔cell; default via the kernel's missed payment; promotion when the size band's last edge is crossed (`cells.weight(cell, 'promotion', 1)` → a named `FIRM`).
- [ ] 0f.9 Seed: cells placed on the lattice from the draws (holdings at cost via `endowUnits`/`endowMoney`; small-firm cash and inventory from `SMALL_SIZE`); `smallBusiness.perNamed` (placeholder, dies at 12), `smallBusiness.cellsPerKey` deleted (the lattice is the resolution).
- [ ] 0f.10 `test/lattice.test.ts`: refine every band edge by two → 52-period aggregates move by less than derived dust; parties at period 52 ≤ occupied keys + named; a cell buys from a cell; a hire moves hours and no cell is created; the same transfer to the low and high liquid bands gives different demand; a mean-preserving spread of the size draw raises small-firm defaults and of outlooks raises crossings while means do not move; entry and exit both non-zero; defaults cluster by region and line at constant mean.
- [ ] 0f.11 ARCHITECTURE 4.4 rewritten; COVERAGE re-marked for §41, §42, §46, XI-15, XI-16; record.

**Exit.** 0f.10 holds; `commonGrain`, `shareFor`, `sameState` are gone from `src`.

---

## 0g. The core made fast (Law 18)

Layout and traversal only; every step reports the ladder before and after; a step that moves a ratio is reverted.

- [ ] 0g.1 `test/ladder.test.ts`: 52 periods at (3, 12), (6, 60), (12, 200) banks/firms; per scale: ms/period at 1, 13, 26, 52; parties; events; sessions cleared; audit total; money per member; wage per hour; band counts ×1, ×2. Assert scale-invariant ratios within derived dust; time reported, never asserted.
- [ ] 0g.2 **The period index** (`world/period-index.ts`), written by settlement and revaluation as they write: legs by instrument, legs by party, equity delta by party, reserve flow by bank, `heldTotal` per instrument, dirty parties, the due heap (next due period per instrument, maintained at issue/restate). Readers, each deleting its own walk: audit families (`accounts`, `flows`, `money`, `units`, `names`, `currency`, goods `unitsIdentity`, capital `plantMoves`, banks `bookMoves`), `reporting/report.ts incomeOf/cashOf` (with `cause` on the equity entry at write), `external/index.ts externalOf` (once; the family reads the published event), `banks/lines.ts earnedByLine`, `banks/treasury.ts reserveFlow` and `money-market/session.ts netReserveFlow` (one read), `securitisation interestCollected`, `capital-programme purchases`, `runCorporateActions` and `owedIn` (the heap), `world.ts reach()` (first-traded per market), `equityLedgerFamily` (running sums), `equityDust` once per party.
- [ ] 0g.3 `Measure<D>` → `number & { __d: D }`; `Qty` an integer-checked brand; `core/measure.ts` throws `Impossible('Law 8', …)`; arithmetic and dust unchanged. Mechanical pass over every site.
- [ ] 0g.4 Journal indexes: `lastOfKind(kind)` O(1) (`view.lastPublic`); every `ofKind().filter/at(-1)` site uses `lastOf`, `lastOfKind` or `ofKindIn` (sites: `firms wagesThisPeriod`, `banks/treasury.ts bestRival` (last period only: a bug too), `worthOfMoney`, `probabilityOfDefault`, `environment conditionsIn`, `indices benchmark`, `cds recoveryIsKnown/publishedGradeOf`, `research coverageOf`, `insurers meetCalls`, `central-bank-omo lastRemittance`, `derivative-layer resolveContracts`, `observer` housing/indices/statements, `bond-futures bondCarryOf`, `commodity-futures carryFromWorld`); the latest grade per (assessor, obligor) held by the ratings store and read by `classify`.
- [ ] 0g.5 `view.memo(key, deps, compute)`: recomputes when a named dependency's version moved. Sites that today recompute per call: `equity()`, exposure by issuer, `earned(window)`, `coveredLines`, `stateOf`, `regulationOf`, `eligibleLines`, `holdingsWorth`, `dearest(subUnit)`, `overdue(seller)`, `goodsBoughtIn`, `sizeSegmentOf`, `claimsSeen`, `longestPromise`, `blindView`.
- [ ] 0g.6 Register: `issuedBy`/`ofKind` used at every `instruments.all()` filter (`firms/produce.ts`, `firms/invest.ts`, `treasury/index.ts`, `reporting listedLineOf`, `estate claimsOn`, `funds eligibleLines`, `funds/nav.ts`, `banks/dealing.ts coveredLines`, `money-market fallsDueToIt`, `merchants marketsOf`, `indices listed`, `cds openBooks`, `housing foreclose`, `corporate-bond testCovenants`, `sovereign-curve` family, `world.ts curveAt`); lots coalesce on equal basis and period; `holdingsOf` returns a frozen view; `Parties.ofKind` cached per (kind, version).
- [ ] 0g.7 `curveAt` memoised per (family, period, prices.version); `invertDecreasing` seeded by the last yield; `index()` keyed on the basket's own prints.
- [ ] 0g.8 `ParticipantDecl.markets` mandatory (assembly refuses a decl without it; `everyone` kept); `central-bank-omo` names sovereign lines; `market.noView` one event per period.
- [ ] 0g.9 Solver: sort once, sweep with running sums; outcomes byte-identical on a fixed book.
- [ ] 0g.10 Derivatives: `marginCalls` over pairs; `valueTo` memoised per (contract, period); `requirement` per (poster, holder, ccy, period); capacity per (party, ccy, period) decremented within the session.
- [ ] 0g.11 Columnar state behind the read faces: an interning table gives parties, instruments, units and currencies dense integers at registration; holdings as CSR typed arrays with lots in a side array; equity, weights, `latest` prints, the period index as typed arrays; `RegisterReads`, `PriceStore`, `Parties` keep string-keyed signatures.
- [ ] 0g.12 Observer from the period's slice: `snapshot` reads `journal.inPeriod`; static parts cached by `instruments.version`; a party scope walks its own holdings; `numOf` → `null` on a missing field.
- [ ] 0g.13 Calendar reads: `isAnniversary(epoch, months, period)`, `periodsUntil(date)`, one `yearFractionOfPeriod`; sites: `funds/mandate.ts:816`, `cds/index.ts:171`, `securities-lending due`, `central-bank-omo dueThisPeriod`, the six copies of the year fraction (`banks/dealing-quote.ts`, `staff.ts`, `deposits.ts`, `index.ts costOfFunds`, `treasury.ts`, `funds/manager.ts:201`).
- [ ] 0g.14 Tiered journal: last N periods hot with indexes; older periods compacted to a columnar log readable by the observer and `coverage-reached`; no event lost.
- [ ] 0g.15 Parallel order generation: parties of a market partitioned by a fixed hash of their handle; partitions run on workers (`worker_threads`; Web Workers in the app); orders gathered in partition order; clearing single-threaded. Gate: the ladder byte-identical single- and multi-threaded.
- [ ] 0g.16 Record: the ladder per step at three scales.

**Exit.** (12, 200) one country: a 52-period year under 60 s on CI; (3, 12) under 3 s; every ratio invariant.

---

## 11. Small-Business Pools (§42), the rest

On 0f.8. What is left is the sector's connection to the world.

- [ ] 11.1 Re-mark §42 A1/A2/A3/A5/A6 from the 0d run (a clause reached by a lattice cell is MET; else PARTIAL naming the step).
- [ ] 11.2 Loans: the bank of the cell's key quotes through 17.0 (until 17.0: through today's `quote()`), a loan row per (lender, cell) with amortisation (11.5 of banks: `loan.ts` gains an amortisation schedule here, F2).
- [ ] 11.3 Correlation (B4, B4.a): no parameter. Test: two regions with different demand give different default counts at identical draws.
- [ ] 11.4 The pool into the vehicle: `securitisation` accepts small-firm rows; `moneyOf(bank)` reads the bank's own currency, not the registry's first.
- [ ] 11.5 E1–E6 as six tests; COVERAGE for all 28; record.

**Exit.** A credit tightening reaches a small firm before a large one; an invoice names a small firm; small-firm sessions clear in the census.

---

## 12. Firm birth, household formation, promotion

- [ ] 12.1 `small-business/found.ts`, phase `smallBusiness.found` (reads `firms.decide` writes): a household cell whose liquid wealth above its buffer clears the line's published starting cost (a read of the line's cost base) and whose outlook on the line's margin clears its required return founds `n` firms (integer): one instruction — founders' cash out, the new members' ownership row in. A refusal journaled `smallBusiness.notFounded` with the reason.
- [ ] 12.2 Formation (Households A5): members of cohort `k` whose own income clears the printed rent move to cohort 0 by `cells.weight(cohort0Cell, 'entry', n, 'formed')`. No rate.
- [ ] 12.3 Mortality as TECHNOLOGY per five-year band (an imported real-world primitive, declared with its source); `die` moves members by it, dated.
- [ ] 12.4 Promotion fires in a 52-period run (0f.8); the promoted member's pieces become a named `FIRM`'s holdings.
- [ ] 12.5 `equity` and `control`: a cell's dividend claim is one agreement per cell read by `agreements.byDebtorAndKind`; `dividendLegs` reads `receipt.line` on the leg, no reason parsing.
- [ ] 12.6 Delete `seed.membersPerCohort` (`seeds/foundation.ts:772`); cohort sizes are RESOLUTION numbers tested by invariance; the read that replaces it is `integrate` over `parties.ofKind(HOUSEHOLD)`.
- [ ] 12.7 Test: over 52 periods entries and exits both non-zero, neither the identity of the other; promotions neither zero nor every cell.

---

## 12a. Households borrow, owe and fail; arrears; the immortals

- [ ] 12a.1 `register/arrears.ts`: instrument kind `ARREAR` (issuer = payer, holder = payee, `carriedAtCost`, no market; terms: the failed instruction id, the payment CLASS — wage, tax, transfer, rent, invoice, coupon, fee, service). Written by `ledger/settlement.ts` in the same pass as the fail (one writer). Ranks in the estate by class.
- [ ] 12a.2 `world/failure.ts stillOwed` = `register.issuedBy(self)` filtered `ARREAR`; delete `failedPayments(0)` from it.
- [ ] 12a.3 `runCorporateActions` presents every `ARREAR` of a payer before any new due of the same class; a paid arrear is redeemed. `trade-credit invoiceKind.due` presents the maturity every period from the due date until redeemed; `defaultOn` = the buyer missed it; `treasury.shortfall` (transfers), the unpaid wage, the unpaid estate transfer and the unpaid rent all write the row.
- [ ] 12a.4 Delete the guard at `housing/index.ts:1558`; the mortgage is a loan row per (lender, cell) in totals; `households/profile.ts` subtracts service and rent before the basket (0f.7 already); the lender's limit reads the dwelling print × its haircut (17.0's view).
- [ ] 12a.5 `fails: ['cash']` on the household kind; failed members move to probate and to `credit record = defaulted`; consumer credit is the same row unsecured.
- [ ] 12a.6 The treasury: `fails: ['cash']` conditional on currency (`failedWhy` reads the arrear's currency; it fails only in a money it does not issue); a domestic-money miss is `treasury.shortfall`, never `credit.default` (`credit-events` skips payers whose kind `sovereignIn` the currency); `treasury/index.ts` refuses to auction while an `ARREAR` of the treasury stands (Sovereign G5). Probate and the deposit insurer declare what they fail on or name the XI-3 exception in `why`; `securitisation` vehicles fail on the senior coupon's cash shortfall.
- [ ] 12a.7 `short-term-debt drawBackstops` writes a loan row (the banks module's kind) at the line's rate with a maturity; `BackstopTerms.drawn` is a read of that row; the fee is charged on `limit − row.outstanding`; `grantBackstops` grants on the first `paper.offered` only.
- [ ] 12a.8 A buyer of second-hand plant: `firms/invest.ts` bids in a vintage's market when the print per unit of remaining life is below new-build cost at its hurdle; the estate's ask is two states (last print while it has time; `market` in its last period), never a linear path.
- [ ] 12a.9 Tests: a failed levy is an arrear next period; a mortgage is serviced; a household that cannot fails and its dwelling passes through the lien; a treasury short in a foreign money fails; a backstop draw is a row on both books; `housing.shortfall` no longer fires for every cell every period.

---

## 12b. Employment is a standing relation

- [ ] 12b.1 One employment register (noun `labour.employment`, home `register/employment.ts`): employer, employee cell, hours, wage, since, notice. Delete `labour/register.ts`, `banks/staff.ts`'s `staff` map, the `labour.wages` tally; wages are instructions that READ it; `view.employs()`.
- [ ] 12b.2 The venue matches NET changes: an employer posts the change it wants; a separation runs `notice` periods with wages due, then ends (Labour C3).
- [ ] 12b.3 A wage bid is from the employer's outlook on its revenue (§46), not `earned(1)`; the firm's ask is from its outlook on its own sell price formed from fills and the venue print, its cost the reservation (F15).
- [ ] 12b.4 `labour/matching.ts match` in whole hour-pieces; the remainder is unsold hours journaled `noDemand`; the labour print read for the reader's own region (`treasury/index.ts wageItFaces(region)`).
- [ ] 12b.5 Research analysts are `analysis`-trade staff hired in the venue; `research/index.ts pay()` deleted.
- [ ] 12b.6 Test: a bank with one bad period keeps its desk; a separation pays notice; one register answers who works where.

---

## 12c. Productivity is an outcome

- [ ] 12c.1 Each recipe declares, as TECHNOLOGY, the rate at which hours per unit fall with cumulative units the LINE has made (one number per recipe, log scale, `why`); a firm's hours per unit is a read of its own register history of created units. No stored level.
- [ ] 12c.2 The employment row carries the trade's learned rate at the employer it came from; a hire moves it; a firm's rate is the read over its rows.
- [ ] 12c.3 Entrants (12.1) draw productivity from the line's tail above the incumbents' median; exit is the kernel's default. Test: output per hour rises over a five-year run with no parameter saying so; the size distribution's tail exponent recorded with its killer (Law 17).

---

## 12d. Observation

- [ ] 12d.1 `expectations`: every public print and public event about a party the observer is exposed to enters its outlook as an observation (§46 A2.a): the region's wage, the retail prints it buys at, its bank's board, the dwelling print, the reports of companies it holds. The exposure set is a read of the party's rows and holdings.
- [ ] 12d.2 A party keeps its adaptive predictor and one or two public-anchored predictors per variable and follows the one with the smaller surprise over its memory; no intensity coefficient. Killer: §46 E2/E3 in a 52-period run — if the aggregate moves before the surprises, this step is deleted.
- [ ] 12d.3 The central bank, insurers and households read the environment condition where their decisions need it (18a.1, 14.2, heating in the basket).
- [ ] 12d.4 Test: `market.noView` is zero for goods books in the census.

---

## 14. Insurers and pensions (§27)

- [ ] 14.1 Capital: the seed floats each insurer (a share line subscribed by household cells at cost) so `equity() > 0`.
- [ ] 14.2 The buyer: a firm standing in a physical fact bids for cover at its own outlook of the loss (the environment condition against its plant's `standsWind`); a household cell bids for life cover from the mortality its cohort observes. Bids are schedules (`rungsUpTo`), not points.
- [ ] 14.3 The claim: an environment period that destroys a covered party's plant or inventory settles the schedule's amount from the insurer; `insurer.claim` is that instruction's event.
- [ ] 14.4 Experience: `claimsSeen` is the insurer's outlook on claims per unit of cover from its own history (never the last claim); the price of cover = experience + cost of equity (XI-4 read) × capital per unit; no ratio becomes a level.
- [ ] 14.5 A policy is an AGREEMENT with the schedule on it (the `POLICY` instrument kind deleted); fills paired by the solver's allocation.
- [ ] 14.6 Pensions: a second profile behind the dispatch table — a sponsor (an employer), contributions as two wage-linked legs in the labour pay phase, a schedule per retired cohort over its remaining life (12.3's mortality), the funding ratio as a read (assets at mark over promises at the curve), a shortfall as an arrear on the sponsor.
- [ ] 14.7 `insurers/allocate.ts`: buffers from outlooks (`investable`), `meetCalls` via `lastOwnSince`; a fund door is live only while its fund is.
- [ ] 14.8 COVERAGE re-marked; the 14 MISSING built or `OUT OF SCOPE` with a reason.

---

## 15. Housing and land, the rest

- [ ] 15.1 Land: the treasury's ask is its own outlook of what ground fetched; it releases per period what a planning policy (owner `parliament` after 19) says, never everything it holds; a local authority present in each place sells that place's ground. `capital-programme` refuses to start a vintage whose `landPerUnit × units` exceeds free hectares held and pledges the ground to the vintage. Delete `CENT_TICK` asks in `land/index.ts:197,251`.
- [ ] 15.2 A port with an owner and a berth; congestion an outcome of berths.
- [ ] 15.3 Commercial property: a building built to LET; a lease as an agreement with a term; a landlord kind (a mass sector: a third lattice profile); CRE lending as a secured row.
- [ ] 15.4 A retail firm sells from a lease.
- [ ] 15.5 Tenancy: `TenancyTerms.until`; re-let at expiry; a landlord ends a tenancy after failed rents by its own outlook of the tenant; `letIn` uses the solver's fills; orders in the venue's unit; `mortgagesOf`/`foreclose` read the loan's `security.instrument` and `lien.id` (no `startsWith`/`includes`); `foreclose` reads `register.ofKind(LOAN)`.
- [ ] 15.6 A region with people and no dwelling line is a seed finding reported at the seal.
- [ ] 15.7 COVERAGE re-marked; the 19 MISSING built or `OUT OF SCOPE` with a reason.

---

## 16. Cross-border, the rest

- [ ] 16.0 `core/measure.ts`: `Cash` carries its currency (`{ pieces, ccy }` or a brand with a runtime tag); `plus/minus` of two monies throw `Impossible('Money A2.b')`; `sum(cash[])` across currencies fails at the site; `valueAt` yields the instrument's currency; `inMoney` the one door. One pass over every arithmetic site.
- [ ] 16.1 Every currency literal gone: `grep -rn "'USD'\|USD as CurrencyCode\|currencies.keys().next()" src` returns only registry declarations (`seeds/foundation.ts:2146 priced()`, `control/index.ts:808`, `cds/index.ts:188`, `securitisation moneyOf`).
- [ ] 16.2 `abroadWorld` 30 periods: per country banks, firms, households, listed lines, sessions cleared; the table written here.
- [ ] 16.3 Sourcing across regions: a buyer in one place buys from a seller in another; merchants bid the far print less their outlook of the freight rate on that leg with one budget across lines; producers ship only unsold stock on the dearest leg.
- [ ] 16.4 Foreign-currency issuance: a treasury or firm issues in a money it does not print and can fail in it (12a.6).
- [ ] 16.5 The swap line; `ctx.transact([books])` — one instruction whose trade legs come from two or three sessions, atomic or not at all (FX arbitrage sized by cash held in each money).
- [ ] 16.6 FX participation is a question over every party kind (no kinds list); `needOrders` prechecks a foreign balance or liability before `owedIn`.
- [ ] 16.7 Delete `seed.crossHoldingShare` and `seed.centralBank.openingHoldingShare`; the read is the central bank's own holdings after it buys for its reason (Central Bank F4).
- [ ] 16.8 Cross-Border A–D built or `OUT OF SCOPE`; `check:opens` on `abroadWorld` prints FX sessions clearing.

---

## 17. Corporate credit, the rest

- [ ] 17.0 **The credit view** (`banks/credit-view.ts`, replacing `banks/quote.ts`, `holderReservation`/`publishReservations` in `banks/index.ts`, `dealing-quote.ts viewOf`, `money-market/collateral.ts requiredOf`): per bank per period, for each name it has a reason to price (a `credit.request`, a held row, a book it deals in): PD annualised from its own record; LGD from its own recoveries on estates it was a creditor of (1 with no history); the capital charge by the grade published on the name (unrated → the regulation's loan weight); its cost of funds with subordinated debt in the capital layer only; expected loss per name. `bank.reservation.required[obligor]` per name; a desk's view of a dated claim = `priceAtYield(required[obligor])`; `coveredLines`, `stateOf`, `regulationOf`, exposure by issuer once per bank per period. Delete `atLeast(capital, 0)` at `banks/index.ts:373` (journal `bank.insolvent` at the site instead). Test: two names with different histories get different yields from one bank; two banks with different recoveries quote one name differently.
- [ ] 17.1 The leveraged loan as a floating-rate note (second kind in `corporate-bond`; fixing from `index.benchmark`).
- [ ] 17.2 The arranger: underwriter, fee from proceeds, risk between commitment and placement, syndicate per-member limits (`lines.ts roomFor`), backstop.
- [ ] 17.3 The committed facility: the lender sets the LINE (delete `shortTermDebt.line`; the read is the granted limit on the row), undrawn headroom, the fee, the capital an undrawn line consumes; `ctx.restateInstrument(id, terms)` for rolling a maturity while the relationship performs (the margin loan too).
- [ ] 17.4 Issuing reasons: a management target approached at its own pace (A2.b); coverage tests interest plus scheduled principal (A3.a); tenor, size, diversification as separate reasons; a tap goes to the standing line within the tenor band, never a new date per period; covenants carry the holder's headroom as a term of its bid (placeholder naming Corporate Credit B2).
- [ ] 17.5 Trade credit: a seller short of cash ships for cash; the days are the seller's preference; rows named (seller, buyer, period, seq); `overdue` memoised.
- [ ] 17.6 Short-term debt: an issuer's alternative is its OWN last borrowing cost (so a bank can issue); a buyer's alternative is its kind's (a bank's: the deposit facility); the E3 family excludes lines in an open estate.
- [ ] 17.7 Restructuring as an agreement transition; the covered bond; factoring on §42 receivables; senior notes and CP as repo collateral; the index-linked schedule.
- [ ] 17.8 Audit family "no bank loan held outside the banking system" reading `profile.banking: true`.
- [ ] 17.9 `banks/lines.ts:446`: a negative want publishes `mustShed` and the dealing line reads it; floors deleted. `banks/loan.ts`: amortisation, prepayment, write-off.
- [ ] 17.10 A credit index and tracker over the rated universe.
- [ ] 17.11 §7 answered 62 of 62; record.

---

## 17b. The leveraged buyout

After 17.9. Conditional issuance escrowed by the arranger (16.5's `transact`); the tender pays sellers from escrow; a failed tender returns it; `control` combines at a majority through the estate's door.

---

## 18. Commodities spot and futures

- [ ] 18.0 `Print.quotedAs: 'money' | 'rate' | 'ratio'`; `PriceStore.write` refuses a level whose dimension differs from the market kind's; `ContractReads.print` returns the RATE for rate books; `struckAt` typed by the book; `priceOf`/`onQuoteGrid` never see a rate. Test: a 2 % swap marks at 2 %/yr.
- [ ] 18.1 Delivery at the STRUCK price in `bond-futures` and `commodity-futures deliver`. Test: a long that bought at 98 and takes delivery at 100 is richer by 2 × notional.
- [ ] 18.2 Contracts carry `pairedWith` from the strike; netting by (deliverable, expiry) with variation settled.
- [ ] 18.3 Hedgers: want = −(held − target)/contractSize from the party's own target; a party with none hedges nothing (`bond-futures`, `commodity-futures`, `index-futures`). Storage: a taker's bid is a step schedule per good; a failed payment skips the taker, not the letter.
- [ ] 18.4 `ctx.closeMarket(id)`: a series, vintage or CDS book with no open interest and no print in N periods closes; CDS books open on the first reason, not on existence.
- [ ] 18.5 CDS: `levelFor` = the bond's yield over the sovereign curve at the tenor (one derivation with `measures.ts basisFor`); naked size by the capital the weight consumes; `counterpartyTerm` reads the counterparty's book; one `annuityOf` in `registry/derivatives.ts` for irs/cds marks and margins; the series per currency; roll dates by the calendar.
- [ ] 18.6 The basis trade posts a repo need; the commodity tracker.
- [ ] 18.7 A commodity print reaches a margin, a consumer price and the central bank's outlook (18a.1).
- [ ] 18.8 §20 and §21 clause by clause; COVERAGE; record.

---

## 18a. Monetary policy

- [ ] 18a.1 `money-market/policy.ts`: the central bank's own outlook on the price level from the goods prints in its money weighted by its basket read; compared with its TARGET (policy param; owner `parliament` after 19). On its own calendar it moves the rate one STEP towards closing the gap it sees, or not. No coefficient. Journaled `centralBank.rate` with outlook and surprise. Written through `params.setByMandate` (19.1's door, granted to this module for the rate until 19).
- [ ] 18a.2 The quantity response: at the new rate the facilities re-price and the desk supplies or drains reserves until the overnight print sits in the corridor; both legs on both balance sheets.
- [ ] 18a.3 `central-bank-omo`: a SCHEDULE (the level at which its reason stops), never `price: 'market'`; delete `CB_PARAMS.targetShare`.
- [ ] 18a.4 A negative level is legal for time: `MarketDecl.levelMayBeNegative` from the kind profile (`quotedAs: 'rate'`); `settlement.ts`, `price-store.ts`, `solver.ts`, `core/tick.ts` dispatch on it; `treasury/index.ts openLine` coupon floor deleted (a negative yield issues a zero above par). Test: −0.5 % clears and both balance sheets book negative interest.
- [ ] 18a.5 Delete the JPY `why`'s apology (`money-market/data.ts:185`).

**Exit.** The rate moves in a 52-period run for a recorded reason; a negative rate clears; the desk never sets the sovereign price.

---

## 19. The polity (§47)

- [ ] 19.0 The government BUYS: an outlay programme as a participant in construction, vessel and service lines at the treasury's outlook within its programme's constraint; a public employer in the labour venue.
- [ ] 19.1 `params.setByMandate` on one module's context; every other write throws; owner and setter printed beside every policy value; the target moves to owner `parliament`.
- [ ] 19.2 Constitution primitives (`polity.seats`, `polity.termPeriods` by date, `polity.allotmentRule` dispatch, `polity.coalitionMaxDistance`, `polity.mandateLag`); the four tax bases (profits, interest, gains, consumption) as policy values the treasury reads.
- [ ] 19.3 Platforms (`registry/platforms.ts`): one row per party with a value for every parliament-owned policy; assembly refuses a missing, extra or duplicate row.
- [ ] 19.4 The vote: per cell, each platform applied to its own state at its own outlook through `withoutAggregates`; abstain when indifferent; turnout a read.
- [ ] 19.5 Seats by the rule; coalition; a hung parliament continues the standing mandate.
- [ ] 19.6 The mandate journaled and written at the lag; C3.b as an audit contribution.
- [ ] 19.7 What it controls (fiscal rates, transfers, the buffer, the outlay programme, regulatory ratios, the target, the planning release of 15.1) and never controls (the rate, a price, a quantity, an outcome — assembly throws).
- [ ] 19.8 The arrears' `breached` writer (fiscal policy decides when an arrear is a breach).
- [ ] 19.9 A multi-year run shows the deficit changing through named outlays after an election; approval as a lagged read; B4 and F5 tests; observer; determinism; COVERAGE; record.

---

## 20. Periodicity

- [ ] 20.1 The rating fee annual on the first rating's anniversary; `credit.impaired` on transition only.
- [ ] 20.2 Tax withheld weekly, assessed on the fiscal period the polity owns; VAT quarterly.
- [ ] 20.3 The buyback as a `Process`.

---

## 21. The local repairs

Each when its file is open for another item; file:line and the change.

- [ ] 21.1 `households/decide` → `ordersFrom` throws on its own malformed row; one `wholeOrderOf(spend, price)` in `clearing/` for firms and households.
- [ ] 21.2 `money-market/resolution.ts`: the acquirer's consideration is a leg; no forced buyer (`winner = taking[0]`; none → `bank.resolution.noBank`, the insurer bridges); `writeDownRow` reports the cash written down (no `Math.round`); the Banks Capital E3 conservation family built.
- [ ] 21.3 Estates: rent on the space its inventory sits in (the family is wrong); a `winding` estate is not a household to the `ofKind(HOUSEHOLD)` readers; a capacity line produces to order through `Process`.
- [ ] 21.4 `estate/index.ts:608`: delete the stale `standsInFor: { noun: 'Process' }`.
- [ ] 21.5 Dead reads deleted with their docstrings: `money-market/session.ts struckRate, findSession`; `cds/participants.ts cdsBookOrders`; `households/portfolio.ts ownUncertainty`.
- [ ] 21.6 `seeds/foundation.ts drawTrackers`: one tracker per (country, index).
- [ ] 21.7 `test/rig.ts:473` and the 21 files that name a party ask the draw (with 23.1).
- [ ] 21.8 `funds/index.ts:1296` cites `Clearing C1.b` (absent): fix; `check:spec` covers prose citations.
- [ ] 21.9 String parsing gone: `derivative-layer/margin.ts:1499` structured `secures`; `securities-lending receivedOn/lienFor` by lien id; `observer.ts:688` reads the book's tenor from its terms.
- [ ] 21.10 Types: `Register.credit(basisPerUnit)`, `Agreements.owed/paid` (refuse a fraction), `Guarantees.limit/paid`, `Unpaid.amount`, `MarketResult.price` typed.
- [ ] 21.11 `actions.ts:286` `=== 0`, not a dust band; `settlement.ts apply` `sold` keyed per leg; `precheck` journals `reserve.overdraft` after apply; the kernel enforces create-with-destroy or `instruction.ts`'s claim is deleted.
- [ ] 21.12 `enter` checks the entrant's bank issues its money.
- [ ] 21.13 Shapes and floors: `ratings firstBoundary/boundaryStep` get their killer (grade vs default); `firms dealershipShare` a preference or deleted; `derivative-layer/house.ts:1011` √T sizing named a SHAPE with its killer; `capital-programme weather` loss not rounded by `deliverable`; `commission` reads `capital.ordered`; `ratings/assess.ts:1941` no-op floor deleted, the grade clamp as the scale's ends; `derivative-layer/index.ts:548`, `funds/nav.ts:1552` stated as the residual's arithmetic; `spot-fx`, `external`, `research` tolerances → `dustOf`.
- [ ] 21.14 `prime.ts:801 requirementOn` uses the client's units; `prime.wanted` read this period only; the clearing house `depositClass: 'wholesale'`.
- [ ] 21.15 `funds/manager.ts:359 launchToMake` records `expects` with the notice mechanism as its killer; `commitment.ts callCapital` calls per deal (after 17b).
- [ ] 21.16 `environment/state.ts moveOn` documents its variance; `CLIMATE_CELLS` a RESOLUTION param.
- [ ] 21.17 Freight: the storm's loss leg emitted by `sail` or its docstring deleted; `costOf` crew cost = the carrier's wage bill; `legsBetween` computed once at assembly; `load` uses the solver's fills; `Math.ceil/exp/pow` → `core/num`.
- [ ] 21.18 `fx-derivatives`: read a public event through `lastPublicAbout`, never a view of an arbitrary party.

---

## 22. The recipe

- [ ] 22.1 A line may declare more than one recipe (technology, data); the firm picks by its own cost read.
- [ ] 22.2 A recipe declares a batch; a vintage declares an upkeep per period (technology); a line below batch stops.
- [ ] 22.3 Goods A2 as specified.

---

## 22a. The opening is not an equilibrium

- [ ] 22a.1 Delete `prices.write` from `SeedContext`; delete `seed.openingPrice.*`, `seed.openingYield`, `seed.openingRate`. Opening holdings at cost; the first sessions print from posted reasons (sellers from cost plus required return; buyers from their own outlook of worth); `markOf` is `none` until a print; `rateInForce` for an untraded pair is `none`.
- [ ] 22a.2 Delete `PUBLIC_AT_THE_OPENING`; flotation is 10f's decision over the first year.
- [ ] 22a.3 Delete `seed.households.openingHoldingShare`: households open holding the money fund and the tracker only.
- [ ] 22a.4 Every number in `seeds/foundation.ts` outside the register (`SEED_STOCK_BASIS`, `OPENING_WAGE`, `SEED_PLANT_AGES`, `CARRIER_COUNT`, `PLACES_PER_COUNTRY`, `HULLS_PER_UNIT_OF_SIZE`, `SEED_PROFILE`, `MATURITY_MONTHS`, `MATURITY_DAY`, cohort ages, the epoch) declared with kind, owner and `why`; `priced()` reads the instrument's own `ccy`; household deposits a stated opening stock, not the banks' leverage residual.
- [ ] 22a.5 Remaining `kind: 'shape'` params become placeholders naming an item, resolutions tested by invariance, or are deleted.
- [ ] 22a.6 `check:opens` prints prints appearing over the first weeks; period 1 vs 13 sessions cleared recorded.

**Exit.** `grep -rn "openingPrice\|openingYield\|openingRate\|openingHoldingShare\|PUBLIC_AT_THE_OPENING\|prices\.write" src/seeds src/world/context.ts` returns nothing; `params` has no `shape`.

---

## 23. Measure — Part XII

- [ ] 23.1 Resize the scale model as one bounded change (with 21.7): a test never names a party.
- [ ] 23.2 "A bank that is insolvent is never resolved" diagnosed.
- [ ] 23.3 Part XII's measurements with the level carried, at the smallest scale the ladder shows invariant.
- [ ] 23.4 The accredited line: whether any household clears it.
- [ ] 23.5 Which blueprints the world grew and wound down.

---

## 24. The app and the APK

§45 A4 (inspector vs participant) is the owner's decision. A year in under two minutes on the device is the gate.

---

## Part 3 — The index

Finding ids are those of `docs/measurements/2026-09-15-review-findings.md`.

| findings | item |
|---|---|
| 0.2 #1–#17, C1, BK29, M1, M2, BK5, FR9, IN6, EQ10, BK12, MM16, F3 | 0 |
| C11, WK1, WK2, S6, EN4, FD19, BK30 | 0a |
| C13 | 0b |
| D1–D16, E-25, K9, A7, W6, FR7, CD8, S3, SB2, FR5, CO5, EN3, A5 | 0c |
| C-1 | 0d |
| BK4, CO4, OB5, H7, H9, RP10, CD5, FD3, IN2, MM13, BK31, W6, A7 | 0e |
| R2, M3, W5, H2, F7, H3, A1, MM11, HO10, GD1, H5, H1, H4, H6, SB1, SB2, SB3, TC5, DESIGN-1, C3 (promotion) | 0f |
| C12, D8, A3, A4, A6, R1, R3, R4, K5, K11, V2, W2, W3, W7, G1–G4, G7, G8, G11, F1, F2, F11, GD4, CP1, BK2, BK10, BK14, BK15, BK17–BK20, BK32, MM1–MM5, FD1–FD3, FD5, FD8, FD9, FD12, EQ1, EQ2, EQ8, EQ9, EQ13, EQ17, RP1–RP3, RP5, RP9, EX1, DL1–DL3, DL10, CD1, CD2, CD4, IX1–IX5, SL1, SL5, SZ2–SZ5, ST5, TC1, TC2, IN7, IN9, HO3–HO5, MR1, MR2, FR1, FR8, CO1, SC1, SC2, OM3–OM5, OB1–OB4, OB7, OB8, WK3–WK15, K1, M5 | 0g |
| SZ1 (currency), 11.x | 11 |
| A-17, EQ4, EQ2, M-1 (entry), M-9 | 12 |
| D-1, A-39, A-41, A-20, F13, TC3, HO8, ST2, ST3, HO1, A-53, HO6, C4, A-36, C15, EQ18, SI1, EQ14, EQ16, CP5 | 12a |
| BK26, F8, F9, RP6, F6, RP7, F15, M-4 | 12b |
| M-1, M-11, M5 | 12c, 22 |
| H8, M-6, M-13, EN2 | 12d |
| IN1, IN3, IN4, IN5, IN8, IN10, IN11, A-9, B-2 | 14 |
| LD1–LD5, E-5, HO2, HO7, HO9, HO11 | 15 |
| K2, S1, EQ11, CD3, EX3, FX1, FX3, FX4, V3, MR3, MR5, FR2, B-9 | 16 |
| BK1, BK3, BK6, BK7, BK8, BK9, BK11, BK21, RP14, MM14, CB1–CB6, ST1, ST6, ST8, TC4, TC6, E-24, E-17, E-23, M2, M7 | 17 |
| EQ12, FD14 | 17b, 21.15 |
| IR6, E-11, BF2–BF5, CF1–CF5, IF1, IF2, CD6, CD7, CD9, OP3, CO2, CO3, CP4 | 18 |
| C6, D11, E-7, K6, V1, D-2, OM1, OM2, F12, C7 | 18a |
| F14, D12, M-7 | 19 |
| RP13, C-2 | 20 |
| A-24, F4, E-6, MM7, MM8, MM9, E-2, E-3, E-4, 21.8, E-13, A-69, D16, E-18, DL6, SL2, SL3, OB6, R5, G5, G6, K7, M4, W4, K8, K10, WK13, RP12, F5, DL7, CP3, CP2, RP11, DL4, FD7, EX2, RP8, BK27, BK28, DL11, FD11, EN1, FR3, FR4, FR6, FR7, FXD1 | 21 |
| M-8 | 22 |
| C8, D9, S2, S4, S5, W1, S7, S8, S9 | 22a |
| E-22, B-13 | 23 |
| verified, kept as notes: BK13, BK24, MM15, MM18, GD2, GD3, G9, G12, G13, K3, K4, FD6, FD13, FD15, FD17, FD18, EQ3, EQ5–EQ7, EQ15, MM10, MM12, DL5, DL8, IN11, W8, SL4 (with 17.10) | — |

## Appendix — lessons

1. A `done` row and a `MET` mark are claims; `check:opens` is the falsifier.
2. "Do not measure mid-build" is not "never measure".
3. A missing sector is an item; a defect is evidence.
4. A finding leaves this file only by a ticked step that names it.
5. A performance step that changes a ladder ratio is not a performance step.
6. Insert at the dependency position; never append.
7. A test that names a party asserts a world that no longer exists.
8. A stated opening price is an imported equilibrium.
9. A docstring that describes a mechanism the code lacks is a defect.
10. One design decision expressed in many places is remade, not patched.
