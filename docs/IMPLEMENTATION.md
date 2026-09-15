# Project Phoenix — the implementation

> The one ordered list of what is left (Law 10). Rewritten 2026-09-15 from a full read of every
> engine source file and every document; findings are in `docs/FINDINGS.md`
> and each is positioned in Part 3. **Take the first open item. Tick steps as they close. When an
> item closes, delete its section and write `docs/RECORD.md`; re-mark `docs/COVERAGE.md` in the same
> commit.** A step names a file, the change and the test. *Delete* names the read that replaces it.
> `npm run check:opens` (item 0.17) runs at any time; the suite runs at the end of a module.

## Part 0 — The measured state (2026-09-15)

### 0.1 By spec system

`npm run check:existence` over `docs/COVERAGE.md`. **1,361 clauses: 888 MET (94 UNMEASURED),
105 PARTIAL, 368 MISSING.** A MET mark means a module cites the clause; the read found MET rows on
code that cannot run (re-marked by the item named): Insurers A4/B1/B2 (14), Housing B1/C1/C3/C4
(12a), Freight A3/D2/D6 (0.11), Trade Credit D1/D4 (12a.3), Short-Term Debt B3.b/B4 (0.5, 12a.7),
Corporate Credit E5 (17.0), Banks Capital C2 (0.9), §42 A1–A6 (0f), Capital Programme A2/C1 (15.1),
Commodities Spot D3 (18.3). The table is the tool's; `--verify` holds this file to it.

**The 94 UNMEASURED marks are now measured** (0d.3): 52 periods of both worlds, and the capabilities
that have never produced anything are listed under 0d. Nine derivative classes of nine, the
corporate bond, the policy and two of the four currency-pair participants. **The marks stand**, and
what changed is that they are evidence rather than a reading of the source. Item 0 moved two of
these into the reached column — `freight`'s transit lines exist and `smallBusiness`'s cells do —
and neither is yet a MET a run has confirmed.

| system | MET | PARTIAL | MISSING | UNMEASURED | total |
|---|---|---|---|---|---|
| Money | 28 | 0 | 8 | 0 | 36 |
| Register | 20 | 4 | 2 | 0 | 26 |
| Clearing | 21 | 5 | 1 | 0 | 27 |
| Audit | 20 | 3 | 0 | 0 | 23 |
| Seed | 16 | 0 | 6 | 0 | 22 |
| Currency | 24 | 0 | 1 | 0 | 25 |
| Bond | 14 | 1 | 1 | 0 | 16 |
| Derivative | 18 | 0 | 0 | **3** | 18 |
| **Corporate Credit** | **14** | 8 | **40** | 0 | 62 |
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
| Firm Birth | 13 | 7 | 5 | **1** | 25 |
| M&A | 13 | 0 | 9 | **7** | 22 |
| Trade Credit | 8 | 3 | 11 | 0 | 22 |
| Goods | 27 | 2 | 10 | 0 | 39 |
| Freight | 17 | 3 | 0 | 0 | 20 |
| Labour | 23 | 1 | 3 | 0 | 27 |
| Housing | 7 | 1 | 18 | **3** | 26 |
| Households | 20 | 8 | 5 | 0 | 33 |
| Small-Business Pools | 26 | 1 | 1 | **4** | 28 |
| **Cross-Border** | **6** | 2 | **18** | 0 | 26 |
| Ratings | 16 | 3 | 4 | 0 | 23 |
| Reporting | 32 | 5 | 1 | 0 | 38 |
| Observer | 16 | 0 | 10 | 0 | 26 |
| Expectations | 17 | 2 | 8 | 0 | 27 |

### 0.2 The world does not open — twenty-three stops

Nine found by assembling the rig and patching each throw; eight found by reading, each the first
time its path runs; six more found by STEPPING the patched world, each one hidden behind the last.
All are item 0.

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
| 17 | cost of capital needs a quote that needs a project that needs a cost of capital → **nothing is built** | `registry/capital.ts costOfCapital` | 0.16 |
| 18 | `reporting.publish` reads a price the period has not printed yet | `reporting/index.ts`, `Clearing F1.a` | 0.5b |
| 19 | an agreement row still names the party that ceased: the fee is addressed to somebody who is not there | `world/succession.ts` (absent), `Money E4` | 0.19a |
| 20 | a mandate that passes to an estate makes the estate a fund, and its share line does not exist | `funds/mandate.ts livingPools`, `Register A4` | 0.19a |
| 21 | a backstop line the kernel has torn up is charged a commitment fee every period after | `short-term-debt/index.ts:584` | 0.19b |
| 22 | an issuer bids in its own paper book, and the solver refuses the crossing fill | `short-term-debt/index.ts:389`, `Clearing A2` | 0.19c |
| 23 | a bank quotes and writes a loan in a money it issues none of | `banks/index.ts publishQuotes`, `Money A1` | 0.19d |

Stops 19–23 are each behind the one before it: 19 is reached only once 4, 8, 9, 10 and 16 let the
world past period 2; 20 is reached only by fixing 19; 23 only in `abroadWorld`, at period 13.

Outside the rig: 6 banks/60 firms throws `share.etf.us already exists` (21.6); the rig opens one
country so the FX layer is exercised only by `abroadWorld`.

**A silent one, and it had happened before.** 0.4 gave `small-business` `requires: seed.foundation`
to fix stop 5. Assembly sorts by `requires`, so a module declared mid-list that needs the seed drags
every module after it behind the seed — and `freight` is after `small-business`. The foundation's
hull block then ran before the carrier parties existed and **this world opened with no merchant
fleet**, exactly as the comment on `land()` says it did once before. It threw nothing. The rule is
in that comment and is now in `small-business` too: a module that needs the seed is DECLARED after
it, never made to require it in place.

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
| 0 | The world opens — **done** (section removed; see `docs/RECORD.md`) | seventeen stops; repairs only |
| 0a | Phases ordered by what they read and write — **done** (section removed; see `docs/RECORD.md`) | two stops are the anchor design; every module after this declares reads |
| 0b | The cell key belongs to the party kind — **done** (section removed; see `docs/RECORD.md`) | stop 6; 0f needs it |
| 0c | One truth in the documents; the guards that bite — **done** (section removed; see `docs/RECORD.md`) | before any item closes on the new plan |
| 0d | The overdue suite run, triaged — **done** (section removed; see `docs/RECORD.md`) | the measurement eleven modules owed |
| 0e | Questions, not hooks — **done** (section removed; see `docs/RECORD.md`) | the module contract every sector item after it uses |
| 0e′ | Stores, not events; and the observer imports nothing — **done** | the same subject; before 0f, because the lattice declares stores |
| 0f | The population lattice (cells hold totals) — **done** (section removed; see `docs/RECORD.md`) | the representation every mass sector stands on; must precede the columnar state |
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

## 0g. The core made fast (Law 18)

Layout and traversal only; every step reports the ladder before and after; a step that moves a ratio is reverted.

- [x] 0g.1 `test/ladder.ts` (a script, one rung per process: `npm run ladder -- <banks> <firms> <grain>`) and `test/ladder.test.ts` (the invariants, first rung). **Findings at the first run.** `equity/share.ts votesOf` multiplied a cell's TOTAL shares by its weight (a 0f.1 site no opens run reached — votes are counted only where a listed line has a record date) and the first rung cast more votes than there are safe integers; fixed (votes = units held × votes per share). The run then died of HEAP at 8 GB before the third rung: the journal keeps every event of every period in one array and the ladder holds two worlds at once — the measurement 0g.14 (tiered journal) and 0g.2 (the period index) exist for; the per-rung figures are in the record. **Third build stop (second rung, period 32):** `accelerate` guarded only the calling line, so an issuer's lines were redeemed once per ordering — factorial — and 1.2 million failed maturities of one firm's paper ate the heap in one period; every line is called once per pass whoever calls it. **Fourth (third rung, period 3): 4.4 million lots** — a fill per lot, a copy per re-key, a concatenation per merge; a lot is a basis and a date and equal ones join (0g.6's coalescing, done here; a merge orders by date first). The ladder is in the record: (12, 200) a year in 37 s, (3, 12) in 5.1 s. Steps: 52 periods at (3, 12), (6, 60), (12, 200) banks/firms; per scale: ms/period at 1, 13, 26, 52; parties; events; sessions cleared; audit total; money per member; wage per hour; band counts ×1, ×2. Assert scale-invariant ratios within derived dust; time reported, never asserted.
- [ ] 0g.2 **The period index** — **first sub-step done (0g.2a):** `Ledger.deltasIn(period)` (holding deltas by holder|instrument, issued deltas by line, units made/destroyed by line), written at `append`, read by the `flows`, `money` and `units` families in place of three walks of the period's records; first rung 94 → 88 ms/period, behaviour byte-identical. The rest, as listed, moves when a profile names the walk (the third-rung profile is flat: no family above 3%): (`world/period-index.ts`), written by settlement and revaluation as they write: legs by instrument, legs by party, equity delta by party, reserve flow by bank, `heldTotal` per instrument, dirty parties, the due heap (next due period per instrument, maintained at issue/restate). Readers, each deleting its own walk: audit families (`accounts`, `flows`, `money`, `units`, `names`, `currency`, goods `unitsIdentity`, capital `plantMoves`, banks `bookMoves`), `reporting/report.ts incomeOf/cashOf` (with `cause` on the equity entry at write), `external/index.ts externalOf` (once; the family reads the published event), `banks/lines.ts earnedByLine`, `banks/treasury.ts reserveFlow` and `money-market/session.ts netReserveFlow` (one read), `securitisation interestCollected`, `capital-programme purchases`, `runCorporateActions` and `owedIn` (the heap), `world.ts reach()` (first-traded per market), `equityLedgerFamily` (running sums), `equityDust` once per party.
- [ ] 0g.3 `Measure<D>` → `number & { __d: D }`; `Qty` an integer-checked brand; `core/measure.ts` throws `Impossible('Law 8', …)`; arithmetic and dust unchanged. Mechanical pass over every site.
- [ ] 0g.4 Journal indexes: `lastOfKind(kind)` O(1) (`view.lastPublic`); every `ofKind().filter/at(-1)` site uses `lastOf`, `lastOfKind` or `ofKindIn` (sites: `firms wagesThisPeriod`, `banks/treasury.ts bestRival` (last period only: a bug too), `worthOfMoney`, `probabilityOfDefault`, `environment conditionsIn`, `indices benchmark`, `cds recoveryIsKnown/publishedGradeOf`, `research coverageOf`, `insurers meetCalls`, `central-bank-omo lastRemittance`, `derivative-layer resolveContracts`, `observer` housing/indices/statements, `bond-futures bondCarryOf`, `commodity-futures carryFromWorld`); the latest grade per (assessor, obligor) held by the ratings store and read by `classify`.
- [ ] 0g.5 (taken first, at 0g.1's measurement: the household basket was costed once per VENUE per cell — a twelfth of a period at the first rung — and is costed once per cell at its decision, kept in its `DECIDED` working store and read by `willWork`; behaviour byte-identical on the ladder) `view.memo(key, deps, compute)`: recomputes when a named dependency's version moved. Sites that today recompute per call: `equity()`, exposure by issuer, `earned(window)`, `coveredLines`, `stateOf`, `regulationOf`, `eligibleLines`, `holdingsWorth`, `dearest(subUnit)`, `overdue(seller)`, `goodsBoughtIn`, `sizeSegmentOf`, `claimsSeen`, `longestPromise`, `blindView`.
- [ ] 0g.6 Register: (lots coalesce on equal basis and period — done at 0g.1) `issuedBy`/`ofKind` used at every `instruments.all()` filter (`firms/produce.ts`, `registry/capital.ts`, `treasury/index.ts`, `reporting listedLineOf`, `estate claimsOn`, `funds eligibleLines`, `funds/nav.ts`, `banks/dealing.ts coveredLines`, `money-market fallsDueToIt`, `merchants marketsOf`, `indices listed`, `cds openBooks`, `housing foreclose`, `corporate-bond testCovenants`, `sovereign-curve` family, `world.ts curveAt`); lots coalesce on equal basis and period; `holdingsOf` returns a frozen view; `Parties.ofKind` cached per (kind, version).
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

## 11. Small-Business Pools (§42): the profile, then the rest

The representation is 0f's (cells on the lattice, one per key, holdings as totals). The PROFILE — what a cell of small firms decides — is 11.0, repositioned from 0f.8 (see the 0f record): the module today has `phases: []` and `participants: []`, and every verb below is a sub-step sized on the day it is taken, the way 0f.7 was. `PRODUCING_KINDS` (`registry/profiles.ts`) has NO READER: the comment says every module that asks "who are the firms" walks it, and six sites still walk `ofKind(FIRM)` — 11.0a makes the claim true or deletes it.

- [x] 11.0 Small-firm profile (`small-business/profile.ts`). Decisions are threshold rules per cell from its own state and outlook; orders are totals; nothing branches on the kind.
  - [x] 11.0a Sell and produce (`small-business/profile.ts`): a cell makes its line out of its MEMBERS' hours (one person and a van: no wage leaves it for them; hires are 11.0c) at the recipe the good declares, from the inputs it holds; it bids for the next batch's inputs at what each is worth to it — the output at the price it expects less the other inputs — with the money it has; and it offers what it made at whatever the book gives, because a service cannot be held (Firm E4). Decided before the labour venue, started after wages are paid, sold when the market asks. The seed opens each cell with one period of its inputs at the opening print (Seed D1, the foundation's own statement for a named firm). `registry/expectation.ts expectedPriceOf` is the one public read of "own outlook, else the tape" (firms and households still carry their copies — a Law 4 finding, positioned at 11.0b). `rigFor` gained `makes: [subUnit]` so a test can ask for a world with a seller of its input. **Not the drawn productivity the step named:** a per-cell draw would be a second dispersion beside size (Law 2), and a cell is many firms; the recipe is the technology. **Findings:** `PRODUCING_KINDS` claimed every module walked it and none does — the comment now says so; in the rig the named mines never start (`bound: labour`, `unitCost: null`, no stock at the seed), so no input line trades and small firms live off their opening stock — the named firms' condition, positioned at 12b.3. **Fixed here (build stops, freight, reached for the first time when small firms bid for flour abroad):** cargo loaded and landed under causes settlement refuses for a `create`; a carrier's free hulls counted once per session and pledged twice; the sail phase's undeclared read of the weather and write of its own event.
  - [x] 11.0b Inputs on terms: `PartyKindProfile.buysOnTerms` declared on every kind (firms, small firms and — as measured, the one buyer that ever took terms in the scale model — the treasury say yes; households (C1.d), estates, pools, houses and the rest say no); `shipsOnTerms` reads it before it judges the buyer; the trade-credit module answers for small-firm SELLERS too, so the row a cell writes to a cell is the trade credit the tier lives on (§36 A4). A trade-credit test that named the seller as the holder of its own receivable now resolves the seller through its successor (Register F2): the seller had ceased and its estate held the row. **Measured, not chased:** in the rig no small firm ever gets an invoice, because no input line trades there (12b.3).
  - [x] 11.0c Hours: the cell posts into the trade its line employs, in its region, at what an hour is worth to it — the output an hour makes possible at the price it expects less the rest of the recipe, the named firm's own arithmetic (Labour C1) — for the hours its stock of inputs can use beyond its members' own, and posts an empty opening when that is nothing (C3: the venue sheds at its cost). Hours under contract count in what it plans and the payroll that settled counts in what it starts and what a unit cost (C2, Goods B5), through the labour module's own event. `PRODUCING_KINDS` deleted: the venue reads postings from whoever posts, and no other reader ever came. **Measured, not chased:** in the rig no small firm's stock out-runs its members' hours, so every posting is empty and no small firm hires — 12b.3's world.
  - [x] 11.0d The owner: an ownership ROW per small-firm cell (`smallBusiness.ownership`, a going-concern commitment opened at the seed) to the household cell of its region and bank in the first working cohort; each period, after the session, what stands above what the cell keeps goes to its owner as a dividend in a two-sided instruction. **What it keeps was the whole of the step:** a draw of everything above what it happened to bid for emptied every cell to the same nothing and the lattice merged the sector into one cell a line (98 → 27 in a period); what it keeps is a period of trading AT ITS OWN SCALE — the inputs for the batch its hours can make, in full, at the prices it expects — plus the wages it owes and what falls due on what it has issued. No payout ratio, no target return. **Found with it:** a cell that ran its members' hours flat out made far more than the book took and the unsold perished at cost, killing five cells in six by period one; the batch is what it EXPECTS TO SELL (its own outlook of its fills; one piece to find out), the named firm's own rule (Firm B1).
  - [x] 11.0e Borrow and default: what a period of trading at its own scale needs beyond what it holds, the cell asks its bank for through `ctx.request`, unsecured (a service line has no plant to pledge); the bank reads it next period and the loan row is per (lender, cell) in totals — the banks module already lent to a cell, and the first rung ends the year with 71 small firms of 144 against 65 without credit. Default is the kernel's missed payment: a coupon it cannot pay fails at settlement, the cell fails on cash and goes to its estate. **Not built, positioned:** the failed MEMBERS moving by `cells.weight(cell, 'death', n)` while the rest trade on needs a loan and a miss that are per member, which is 12a's arrears design (12a.1, 12a.5) — with totals a cell fails as one. **Found, fixed in the kernel:** `owedIn` summed a coupon due off the money's grid (a rate times a face, never rounded), and the first cell to owe one read a position that was not a count (Law 8); it is `payable` now.
  - [x] 11.0f **POSITIONED (Law 10), not built, and this says where.** Two things were in this step and neither is the profile's to build today. **Plant.** Every line a small firm runs declares the plant a unit of it takes (`goods/data.ts`, Capital Programme A2: the room the table is in, the ward, the classroom), and the named firm's start is limited by its vintages (`firms/produce.ts capacityFrom`); the profile's `canStart` reads hours and inputs and NO plant, so a cell makes a meal with no room to serve it in — a finding, written here. The limit cannot go in alone: a cell holds no plant, so the limit is a sector that makes nothing from period one, and the seed's plant is a named firm's (`foundation.ts` 1189: a vintage per firm at its headroom). The purchase needs what a cell's money costs — its bank's quote, which is 11.2 — and the named firm's own arithmetic for it (`registry/capital.ts costOfCapital`, `project`) lives in the firms module, which a module never imports: the read has to become the registry's, the way `expectedPriceOf` did at 11.0a (Law 4: firms and small firms carry one arithmetic for what a project must earn). So it is **11.2a**, after the quote. **Promotion.** The event exists (`cells.ts reKeyCell`, kind `promotion`, cause `crossed an edge`) and fires today when a cell's band moves; what it makes is a fresh CELL on the lattice, never a named `FIRM`, because a named firm is a `FirmDecl` the firms module draws at the seed and nothing births one after it — that is 12.1's instruction and 12.4's promotion, which sit where they sit. And with holdings as totals *a member* whose size crosses the last edge is not a thing the model can see (the cell's size is one number for all of them): the per-member position is 12a.1/12a.5's, said at 11.0e too. **Closed with it:** 11.0's exit as sized — a cell decides, trades, is owed and owes, is owned, borrows and fails — is measured in the scale model and the rig; what it cannot yet do is written under the item that gives it the mechanism.

- [x] 11.1 Re-mark §42 A1/A2/A3/A5/A6 from the 0d run (a clause reached by a lattice cell is MET; else PARTIAL naming the step). **Done from a 52-period reach run of both scale models (`coverage:reached`), which is what the item asked for in place of the 0d run:** every capability the small-business, banks, labour, goods and trade-credit modules declare was produced; A1 and A3 re-marked MET (A1 UNMEASURED on employment, 12b.3), A2/A5/A6 stand. The run stopped the four-country world first (a drawing on an inherited line addressed the ceased cell) and that was fixed at cause before it was read.
- [x] 11.2 Loans: the bank of the cell's key quotes through 17.0 (until 17.0: through today's `quote()`), a loan row per (lender, cell) with amortisation (11.5 of banks: `loan.ts` gains an amortisation schedule here, F2). **Finding (11.1, the reach run):** a loan's `terms.borrower` names who SIGNED it, and after a merge or an estate the row is reseated onto the successor while the terms are not — `draw` addressed the ceased cell and settlement refused it (Money E4; fixed at 11.1 with `parties.resolve`, the money-market module's own read); `provisionFor` (`banks/index.ts` ~1618) still asks for the PD of `i.terms.borrower`, a name that has ceased, so an inherited row is provisioned against a party's record that is no longer anybody's (one PD model per borrower, Law 4): read the successor here too. **Done:** the bank of a cell's key is the one bank that quotes it (`publishQuotes` reads the key's `bank` dimension — a structural read of the lattice, never the kind); `LoanTerms.amortising` is struck at origination (a secured row is a term loan and amortises, an unsecured ask is a line, C9); `DueAction` gains `amortisation` (a slice of what is outstanding, read off the calendar each period, no stored schedule) and the kernel's `redeem` takes it in whole pieces; `owedIn` and the row's `cashFlows` carry the same schedule. **Measured, not chased:** a cell's working-capital row is unsecured, so a cell's amortiser is first reached by its first secured row (11.2a, 12a.4). **Findings, positioned:** (i) a cell quoted by its bank and then moved (`deposit.moved`, the same period, later) has next period's row written by the bank it left — the quote is a period stale across a move (17.0, the credit view prices per bank per period what it has a reason to price); (ii) `test/loans.test.ts` has seven reds that predate this item and are identical without it (17.0): *creates a deposit* (four `credit.written` for one ask), *a row with a lender of record*, *gets dearer for a borrower watched into difficulty* (the request was never written), *the provision*, *an overdrawn customer is borrowing* (`test.overspend` reads `bank.capital` undeclared, Clearing F1.a), *accrues interest paid to the lender* (the lender was not paid at all), and *runs a year with lending in it* (80 audit violations — flows on a retired-household cell after a merge, and `units: smallFirm cells stand for −51 people more than the weight events account for`, which is E5's own family firing: 11.5).
- [x] 11.2a.1 Plant, the stock (from 11.0f): the small-firm line carries the plant its recipe names and `canStart` is limited by `capacityFrom(line.plant, vintagesHeld(view), rentedRoom(view))` like the named firm's; the seed opens each cell with the plant a member's opening batch takes — the whole machine the fraction reaches (a firm whose batch takes three hundredths of a room still needs the room), in the three vintages every seeded party's plant is in, carried at the straight line since service. `seedVintage` and `SEED_PLANT_AGES` moved from the capital-programme module and the foundation seed into `registry/physical.ts` (a module's seed may not import a module; one construction of a vintage, Law 4). Test: every cell holds a vintage of each kind its line takes that the world makes, and still makes its line.
- [x] 11.2a.2 Plant, the purchase: `registry/physical.ts` (or a new `registry/capital.ts`) gains the one read of what a project must earn — cost of capital from the party's own rows (its paper's cleared yield, else its bank's loan quote) plus its hurdle — and `registry/capital.ts costOfCapital`/`project` read it (Law 4: one arithmetic, the firms module's copy deleted); what a cell retains above what it keeps bids in the plant market for the kind it is shortest of, at the price where the contribution of the capacity it adds clears what it must earn — the named firm's `project`, no second copy; the row the bank writes against the plant is the secured term loan 11.2 amortises. Test: a cell short of room bids for premises. **Done:** `registry/capital.ts` holds `costOfCapital` (the window and the horizon are the caller's), `project`, `plantOffers`, `capacityNow`, `plantByKind`; `firms/invest.ts` deleted and the firm's `expectedPrice` copy deleted for `expectedPriceOf` (the Law 4 finding of 11.0b closed for firms; households' copy stands). A cell's hurdle and horizon are two PREFERENCES drawn once per population (`smallBusiness.terms`, a working store); what it retains above a period of trading after its bids is what it can put to a project; the orders go out with its input bids. Its ask names its plant as security. **Found, fixed:** a secured ask was written as a new term loan every time — a cell pledging its plant on every week's working-capital ask was written 414 rows in twenty periods — so the ask now SAYS how it repays (`CreditAsk.repays`: at option = a line, on a schedule = a term loan), which is the borrower's term and not an inference from the pledge. **Measured, not chased:** no cell bids for plant in the scale model, because every cell opened holding the whole machine its batch reaches and its plant lets it run at more than it expects to sell; the test asserts the rule from both sides. **Finding, positioned at 21:** `PlannedOrder` is declared three times (`registry/capital.ts`, `firms/decide.ts`, `households/index.ts`) — one type, the registry's.
- [x] 11.3 Correlation (B4, B4.a): no parameter. Test: two regions with different demand give different default counts at identical draws. **Done:** the test is one four-country world at one seed, twelve periods; failures per region differ and cluster (measured before writing it: 34, 5, 29, 5 cells over twenty periods, 26 of the 34 in one period). No parameter: asserted over `params.all()`. B3 PARTIAL — the unit of default is the cell (12a).
- [x] 11.4 The pool into the vehicle: `securitisation` accepts small-firm rows; `moneyOf(bank)` reads the bank's own currency, not the registry's first. **Done:** `saleable` already read the two profile facts and takes a cell's row; a probe in the scale model sees one in a bank's book. `moneyOf` deleted — it read the registry's FIRST currency for every bank in the world; the read that replaces it is `registry.currencyOf(bank.region)`, per bank, in `arrange`. **Findings (pre-existing, identical without this item):** `test/securitisation.test.ts` has two reds — *declares NO number at all* asserts an empty parameter list and the module declares one RESOLUTION (`securitisation.demand.steps`, which Law 2 allows: a stale assertion), and *cuts no deal out of need* asserts every short bank binds on `leverage` and the world's bind on `weighted` (a stale reading of what the world does, Law 11) — both under 17.0 with the capital view.
- [x] 11.5 E1–E6 as six tests; COVERAGE for all 28; record. **Done:** six tests over one twenty-period scale model; all 28 rows re-marked (D3 MISSING — a tranche as collateral is 14/17b's; B2 and B3 PARTIAL; A6.c PARTIAL). **Fixed at cause, found by E5's test:** the kernel's `cease` recorded no weight event for a cell — a cell that failed on its cash went to its estate and its members vanished from the population with nothing behind it, which is what the units family said every period a cell failed (`smallFirm cells stand for −49 people more…`); `ceaseCell` is the one writer of that death now, and `dieCell` is the empty-handed case of it.

**Exit.** A credit tightening reaches a small firm before a large one; an invoice names a small firm; small-firm sessions clear in the census.

---

## 12. Firm birth, household formation, promotion

- [x] 12.1 `small-business/found.ts`, phase `smallBusiness.found` (reads `firms.decide` writes): a household cell whose liquid wealth above its buffer clears the line's published starting cost (a read of the line's cost base) and whose outlook on the line's margin clears its required return founds `n` firms (integer): one instruction — founders' cash out, the new members' ownership row in. A refusal journaled `smallBusiness.notFounded` with the reason. **Done:** `found.ts`, phase `smallBusiness.found` after `households.decide`. The kernel's `enter` takes a cell now and places it on its lattice, its arrival an `entry` weight event (E5's family reads it). The household publishes `requiredPerAnnum` on its plan. **Found by the measurement, built:** the first cut founded 38 million security firms in one period — the return test had no labour in it and every member of a cell decides alike, so a cell founded with all of itself, every period. Three mechanisms, none a bound: the founder's own hours at the region's printed wage come off the contribution (Firm A3); a firm has a person in it, so a household founds one per member not already running one, counted on the ownership row's terms (`OwnershipTerms.members`); and no more firms than the demand the book left unmet at the last print — which a traded print now carries (`demandAtPrice`, `supplyAtPrice`; `unmetAt` in the price store). **Measured, not chased:** NO SERVICE LINE HAS EVER TRADED in the scale model — every small-firm line's print is stale from period 0, `noDemand` for security, wholesale, professional, repair, personal care, media, design, teaching, care, waste (nobody buys them) and `noSupply` for power, transport, logistics, facilities, telecoms, IT (nobody offers) — so every household is refused with that reason and none founds; the sector sells nothing there and lives off its opening stock. Demand side positioned at 22 (the recipe: the basket and the overheads name no service), supply side at 12b.3. **Findings, positioned:** a founder's hours are still offered in the labour venue by its household (12b, employment as a standing relation); a cell founds with all its members or none (12a.1/12a.5).
- [x] 12.2 Formation (Households A5): members of cohort `k` whose own income clears the printed rent move to cohort 0 by `cells.weight(cohort0Cell, 'entry', n, 'formed')`. No rate. **Done, sized honestly:** this world's cohorts are `working` (from 18) and `retired` (from 65) — there is no cohort k of dependants, and adding one is a population that consumes and does not work (a household composition noun this model does not have). So formation is entry at the FIRST cohort's lower boundary by the same geometry that ages the band out at its upper one (`crossingShare`: the share of a band standing at a boundary in a period, carried as whole people in `households.waiting.toForm`), and a household forms when its people can pay for a roof: the cell's own expected income per member clears the rent its region's lettings venue last struck, read through `registry/funding.ts rentPrintedIn` (never by event name, 0e′.3). The ones who cannot wait, on the record, with the reason. No rate. **Measured, not chased:** the scale model's lettings venue prints `noSupply` in eleven periods of twelve — nobody offers a dwelling to let — so no rent has ever cleared and nobody forms; positioned at 15.
- [x] 12.3 Mortality as TECHNOLOGY per five-year band (an imported real-world primitive, declared with its source); `die` moves members by it, dated. **Done:** `MORTALITY` is sixteen five-year bands from 18 to 100, each the chance of dying within a year with its source on the row (SSA 2020 period life table, sexes averaged, two significant figures); one TECHNOLOGY param per band (`households.mortality.<from>-<to>`); `cohortMortalityPerAnnum` derives a cohort's rate as the mean of the bands it spans weighted by the years of each inside it — the last cohort to the end of the table — and `die` puts it on the period by the calendar's year fraction. Nothing is per cohort any more, and a world that cuts its cohorts differently reads the same table.
- [x] 12.4a.1 **The kernel's doors (from 12.4a):** `ParamRegister.declare(decl)` — the constructor's guards, asked after the seal, twice is still twice — and `ctx.declare(decl)`, journaled `param.declared`; `ctx.cells.promote(cell, members, to, cause)` — `promoteCell` moves the members' share of every lot into a named party that entered this period and holds nothing (`moveShare` may take the last member now), records a `promotion` weight event WITH before and after (the population of cells falls by what left, which the units family reads) and what moved per instrument, and when the whole cell goes it ceases with the party it became as its successor. Test: a named party enters, declares its own number, takes one member of a small-firm cell with its share of every lot; the family is silent.
- [x] 12.4a.2 **A born firm is a firm to its modules.** `firms(rows)` indexes its rows once (`indexOf`) and `equity(rows)` walks its rows in `decide`, `floatations`, `publishReads` and its seed; both declare per-firm params from the rows at assembly. So a party that entered as a `FIRM` holds what it holds and decides nothing (the module's own sentence). Three bounded changes: (i) `firms.register` — a store that grows (a NOUN, homed at 22 with the recipe: a firm's technology is a registry row), `lineOf` reads rows ∪ store, and the participants' `lineOf` guard goes (a firm that decided nothing posts nothing already); `bearFirm(ctx, {line, bank, region, owner, name})` draws the three numbers from `FIRM_SPREAD` under the born name, declares them through `ctx.declare`, enters the party and records `firm.born`; (ii) `equity` reads `firm.born` through a registry read (`registry/births.ts`, never the event name) in a phase after the birth, draws the firm's `payoutPatience`, declares it, issues the share line to the owner by an issuance instruction at the firm's book per share (the seed's own rule), and its row loops read rows ∪ born; (iii) `smallBusiness.promote`: a cell whose equity per member reaches the smallest named firm's equity (a READ of the world, never a declared size — the lattice's top edge is a RESOLUTION and Law 2 forbids the answer moving with it) records the intent per member; the firms module bears each and promotes the member into it; the ownership row's members fall by what left and the owner holds the shares instead. Test: a firm born mid-run decides, produces, is quoted and pays a dividend like a seeded one. **Done as sized,** with the trigger read off the bond market: the smallest named firm with paper outstanding in a market — a failing firm's equity is not the boundary, which is what "the smallest named firm" read as the first time it ran and promoted a power cell in period 5. `registry/births.ts` carries both facts between the modules. **Found, fixed:** a promoted cell's last member left a weight of zero (a cell of nobody) — the cell ceases into the party it became; what the cell issued is reseated onto it (a coupon addressed to the ceased cell stopped the world); and the labour module re-keyed an employment row's WORKER by the name that signed it after that cell had merged away — the world stopped in period 5 of a scale model on a seed no test had drawn — resolved through its successor now (Register F2). **Findings, positioned:** a member promoted out of a cell that stays takes its share of every asset and none of the cell's liabilities (the rows stay the cell's, in totals) — 12a.1/12a.5's per-member position; the firms `productionCosts` audit family reads the seed's rows only and skips born firms (21); an employment row's terms name the worker that signed, a stale copy after a merge (12b, rows as agreements).
- [x] 12.4 Promotion fires in a 52-period run (0f.8); the promoted member's pieces become a named `FIRM`'s holdings. **Measured (12.1–12.3, a 52-period year of the scale model):** no small-firm cell crosses a size edge in the year; 23 of 72 surviving firms sit in the top size band, where the seed put them. So what would be promoted exists and nothing can receive it: 12.4a first. **Fires:** in twelve periods of the four-country world 16 cells (63 members, in power, telecoms, waste and logistics) outgrew the tier, 63 named firms were born with their share lines issued and settled, and they made 161 decisions; in thirty periods of the one-country scale model none, because no firm there has brought paper to a market, so the bond market has no size to read (the honest answer, and 0d's: no corporate bond has ever been issued there).
- [x] 12.5 `equity` and `control`: a cell's dividend claim is one agreement per cell read by `agreements.byDebtorAndKind`; `dividendLegs` reads `receipt.line` on the leg, no reason parsing. **Done:** `Agreements.byDebtorAndKind` (off the debtor index the store already kept, Law 18) on the read facade; `payDividend` walks the resolved issuer's own rows of `DIVIDEND_DECLARED`; the dividend receipt carries `on` — the share line for a declaration, the ownership row for a small firm's draw — and `dividendLegs` reads the leg. The `control` module never parsed a reason (item 3 already). `equity.test.ts` 16 reds and `equity-ledger.test.ts` 1 red predate this and are unchanged.
- [ ] 12.6 Delete `seed.membersPerCohort` (`seeds/foundation.ts:772`); cohort sizes are RESOLUTION numbers tested by invariance; the read that replaces it is `integrate` over `parties.ofKind(HOUSEHOLD)`.
- [ ] 12.7 Test: over 52 periods entries and exits both non-zero, neither the identity of the other; promotions neither zero nor every cell.

---

## 12a. Households borrow, owe and fail; arrears; the immortals

- [ ] 12a.1 `register/arrears.ts`: instrument kind `ARREAR` (issuer = payer, holder = payee, `carriedAtCost`, no market; terms: the failed instruction id, the payment CLASS — wage, tax, transfer, rent, invoice, coupon, fee, service). Written by `ledger/settlement.ts` in the same pass as the fail (one writer). Ranks in the estate by class.
- [ ] 12a.2 `world/failure.ts stillOwed` = `register.issuedBy(self)` filtered `ARREAR`; delete `failedPayments(0)` from it.
- [ ] 12a.3 `runCorporateActions` presents every `ARREAR` of a payer before any new due of the same class; a paid arrear is redeemed. `trade-credit invoiceKind.due` presents the maturity every period from the due date until redeemed; `defaultOn` = the buyer missed it; `treasury.shortfall` (transfers), the unpaid wage, the unpaid estate transfer and the unpaid rent all write the row.
- [ ] 12a.4 Delete the guard at `housing/index.ts:1558`; the mortgage is a loan row per (lender, cell) in totals; `households/profile.ts` subtracts service and rent before the basket (0f.7 already); the lender's limit reads the dwelling print × its haircut (17.0's view). **Finding (11.1):** `housing/index.ts foreclose` reads `t.borrower` off the mortgage and treats a borrower that is not alive as one that FAILED; a household cell that merged (its bank moved) is not alive and is succeeded, and its dwelling would be foreclosed on for having moved bank (Register F2: resolve the successor; an estate is the one successor that means failure).
- [ ] 12a.5 `fails: ['cash']` on the household kind; failed members move to probate and to `credit record = defaulted`; consumer credit is the same row unsecured.
- [ ] 12a.6 The treasury: `fails: ['cash']` conditional on currency (`failedWhy` reads the arrear's currency; it fails only in a money it does not issue); a domestic-money miss is `treasury.shortfall`, never `credit.default` (`credit-events` skips payers whose kind `sovereignIn` the currency); `treasury/index.ts` refuses to auction while an `ARREAR` of the treasury stands (Sovereign G5). Probate and the deposit insurer declare what they fail on or name the XI-3 exception in `why`; `securitisation` vehicles fail on the senior coupon's cash shortfall.
- [ ] 12a.7 `short-term-debt drawBackstops` writes a loan row (the banks module's kind) at the line's rate with a maturity; `BackstopTerms.drawn` is a read of that row; the fee is charged on `limit − row.outstanding`; `grantBackstops` grants on the first `paper.offered` only.
- [ ] 12a.8 A buyer of second-hand plant: `registry/capital.ts` bids in a vintage's market when the print per unit of remaining life is below new-build cost at its hurdle; the estate's ask is two states (last print while it has time; `market` in its last period), never a linear path.
- [ ] 12a.9 Tests: a failed levy is an arrear next period; a mortgage is serviced; a household that cannot fails and its dwelling passes through the lien; a treasury short in a foreign money fails; a backstop draw is a row on both books; `housing.shortfall` no longer fires for every cell every period.

---

## 12b. Employment is a standing relation

- [ ] 12b.1 One employment register (noun `labour.employment`, home `register/employment.ts`): employer, employee cell, hours, wage, since, notice. Delete `labour/register.ts`, `banks/staff.ts`'s `staff` map, the `labour.wages` tally; wages are instructions that READ it; `view.employs()`.
- [ ] 12b.2 The venue matches NET changes: an employer posts the change it wants; a separation runs `notice` periods with wages due, then ends (Labour C3).
- [ ] 12b.3 A wage bid is from the employer's outlook on its revenue (§46), not `earned(1)`; the firm's ask is from its outlook on its own sell price formed from fills and the venue print, its cost the reservation (F15). **And (12.1):** power, transport, logistics, facilities, telecoms and IT print `noSupply` from period 0 in the scale model — the small firms in them never offer, because their inputs never trade there. **And (12.4a.2):** an employment row's terms name the worker cell that signed; after that cell merges the name is stale and the row is not succeeded (the store succeeds debtor and creditor, not terms) — `separate` and the wage leg resolve it through the successor for now; the row's worker should be its creditor, never a term.
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

- [ ] 15.1 Land: the treasury's ask is its own outlook of what ground fetched; it releases per period what a planning policy (owner `parliament` after 19) says, never everything it holds; a local authority present in each place sells that place's ground. `capital-programme` refuses to start a vintage whose `landPerUnit × units` exceeds free hectares held and pledges the ground to the vintage. Delete `CENT_TICK` asks in `land/index.ts:197,251`. **Finding (12.2):** the scale model's lettings venue prints `noSupply` in eleven periods of twelve and `noDemand` in the other — no owner offers a dwelling to let, so no rent has ever cleared there, no household can form (12.2) and no tenancy is signed.
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

- [ ] 17.0 **The credit view** (`banks/credit-view.ts`, replacing `banks/quote.ts`, `holderReservation`/`publishReservations` in `banks/index.ts`, `dealing-quote.ts viewOf`, `money-market/collateral.ts requiredOf`): per bank per period, for each name it has a reason to price (a `credit.request`, a held row, a book it deals in): PD annualised from its own record; LGD from its own recoveries on estates it was a creditor of (1 with no history); the capital charge by the grade published on the name (unrated → the regulation's loan weight); its cost of funds with subordinated debt in the capital layer only; expected loss per name. `bank.reservation.required[obligor]` per name; a desk's view of a dated claim = `priceAtYield(required[obligor])`; `coveredLines`, `stateOf`, `regulationOf`, exposure by issuer once per bank per period. Delete `atLeast(capital, 0)` at `banks/index.ts:373` (journal `bank.insolvent` at the site instead). Test: two names with different histories get different yields from one bank; two banks with different recoveries quote one name differently. **Findings parked here (11.4):** `test/securitisation.test.ts` *declares NO number at all* (asserts no params; `securitisation.demand.steps` is a declared RESOLUTION) and *cuts no deal out of need* (asserts `binds === 'leverage'`; the world binds on `weighted`) — both stale assertions to be re-read against the credit view.
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

- [ ] 21.1 `households/decide` → `ordersFrom` throws on its own malformed row; one `wholeOrderOf(spend, price)` in `clearing/` for firms and households. **And (11.2a.2):** `PlannedOrder` declared three times (`registry/capital.ts`, `firms/decide.ts`, `households/index.ts`); the registry's is the one. **And (12.4a.2):** `firms/index.ts productionCosts` reads the seed's rows only, so a born firm's production is outside the family's check; the family should read the produce event's own line (it carries it) and not a row.
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
- [ ] 22.3 Goods A2 as specified. **Finding (12.1):** no service line has a buyer in the scale model — every capacity line's print is stale from period 0 with `noDemand` (the households' basket and the firms' overheads name no service) — so the small-firm tier sells nothing there and nothing can be founded; the demand for services is the recipe's and the basket's to state.

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

Finding ids are those of `docs/FINDINGS.md`.

| findings | item |
|---|---|
| 0.2 #1–#17, C1, BK29, M1, M2, BK5, FR9, IN6, EQ10, BK12, MM16, F3 | 0 |
| C11, WK1, WK2, S6, EN4, FD19, BK30 | 0a |
| C13 | 0b |
| D1–D16, E-25, K9, A7, W6, FR7, CD8, S3, SB2, FR5, CO5, EN3, A5 | 0c |
| C-1 | 0d |
| BK4, CO4, W6, A7 | 0e |
| OB5, H7, H9, FD3, BK31 | 0e′ (closed: the observer imports no module; the plans, the strike and the bank's allotment are `working` stores) |
| RP10, CD5, IN2, MM13 | 0e′ — **withdrawn on measurement**: none is a same-period read-back. CD5 is cross-period and argues its own case; RP10, IN2 and MM13 read their own events of earlier periods or as an aggregate of what already happened (Observer A5), which is a log read as a log |
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
