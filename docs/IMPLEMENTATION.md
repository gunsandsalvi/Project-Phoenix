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
| Corporate Credit | 49 | 7 | 6 | 0 | 62 |
| Sovereign | 44 | 2 | 5 | **3** | 51 |
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
| Banks Lending | 25 | 4 | 3 | 0 | 32 |
| Banks Funding | 28 | 4 | 0 | 0 | 32 |
| Banks Capital | 20 | 2 | 1 | 0 | 23 |
| Dealer Desks | 26 | 1 | 0 | **2** | 27 |
| Insurers | 16 | 5 | 0 | 0 | 23 |
| Hedge Funds | 13 | 2 | 9 | 0 | 24 |
| Private Equity | 24 | 1 | 0 | 0 | 25 |
| Treasury | 20 | 1 | 4 | 0 | 25 |
| Central Bank | 22 | 3 | 4 | 0 | 29 |
| **Polity** | **0** | 0 | **32** | 0 | 32 |
| Firm | 20 | 7 | 3 | 0 | 30 |
| Capital Programme | 22 | 3 | 0 | 0 | 25 |
| Firm Birth | 13 | 7 | 5 | **1** | 25 |
| M&A | 13 | 0 | 9 | **7** | 22 |
| Trade Credit | 9 | 3 | 10 | 0 | 22 |
| Goods | 27 | 2 | 10 | 0 | 39 |
| Freight | 17 | 3 | 0 | 0 | 20 |
| Labour | 23 | 1 | 3 | 0 | 27 |
| Housing | 14 | 9 | 3 | 2 | 26 |
| Households | 20 | 9 | 4 | 0 | 33 |
| Small-Business Pools | 26 | 2 | 0 | **5** | 28 |
| Cross-Border | 20 | 6 | 0 | 0 | 26 |
| Ratings | 17 | 2 | 4 | 0 | 23 |
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
| 3 units | holds since 16.0 | `Cash` carries its currency; two currencies are refused where they meet (K2 closed at 16.0) |
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
| 11 | Small-Business Pools — **done** (section removed; see `docs/RECORD.md`; 11.2a's plant and 17.0's findings positioned) | on 0f |
| 12 | Firm birth, household formation, promotion — **done** (section removed; see `docs/RECORD.md`) | entry into the pool |
| 12a | Households borrow, owe and fail; arrears; the immortals — **done** (section removed; see `docs/RECORD.md`; findings positioned at 12b.1, 12b.3, 17.0, 21.2) | the arrears row that housing, treasury and the sovereign wait on |
| 12b | Employment is a standing relation — **done** (section removed; see `docs/RECORD.md`; findings positioned at 12d.1, 14, 21.1, 22.3) | duration; no absorbing state |
| 12c | Productivity is an outcome — **done** (section removed; see `docs/RECORD.md`; findings positioned at 21.1, 21.19, 22.2) | growth |
| 12d | Observation — **done** (section removed; see `docs/RECORD.md`; findings positioned at 21.1, 21.20, 21.22, 22.3) | diffusion; the price level's second half |
| 14 | Insurers and pensions — **done** (section removed; see `docs/RECORD.md`; findings positioned at 21.23, 21.24, 21.25, 21.26, 22.3; C5 and D4 out of scope at 17.0/17.10 and 18.3) | a buyer, a claim, a schedule, capital |
| 15 | Housing and land, the rest — **done** (section removed; see `docs/RECORD.md`; findings positioned at 21.27, 21.28, 21.29, 21.31, 21.32, 21.33, 21.34, 21.35, 21.36, 21.37, 21.38, 21.39, 21.40, 21.41, 21.42, 21.43; 12.2 closed at 15.5; Housing B4.a and D5 measured at Part XII) | after 12a |
| 16 | Cross-border, the rest — **done** (section removed; see `docs/RECORD.md`; findings positioned at 21.44–21.55; 16.7's other half at 18a.3; the firm half of 16.4 at 17.4; Cross-Border C4 at 17b) | after 15 |
| 17 | Corporate credit, the rest — **done** (section removed; see `docs/RECORD.md`; §7 answered 62 of 62. Findings positioned at 23.1 (21.65, 21.67), 23.2 (21.64, 21.66), 23.3 (21.67, 21.69), 19 (21.62); 21.59 closed at 17.7, 21.61 at 17.7a, 21.63 answered at 17.9a, 21.68 answered at 17.10a; the covered bond out of scope by the owner's decision) | 17.0 the credit view |
| 17b | The leveraged buyout — **done** (section removed; see `docs/RECORD.md`; §29 answered 25 of 25. Findings positioned at 23.1 (21.73, 21.74, 21.75, 21.76), 23.3 (21.72), 18 (21.71), 18a.1 (21.60(c) and 21.60's diversification half); 21.15 closed at 17b.4, 21.60(a) at 17b.8 and 21.60(b) at 17b.8a; §29 C2's other half became item 17c) | after 17.9 |
| 17c | The board | after 17b; §29 C2's other half, and it has no other home |
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


---


---

---

## 17c. The board

Inserted at 17b.10, at its dependency position (Law 10), because §29 C2 has a half with nowhere else
to go: *"the owner INFLUENCES the firm — investment, costs, distributions."* Distributions are built
(17b.6) and the other two are not, and what they need is a mechanism this world has nowhere: **a
party taking a decision on another party's behalf.** `ctx.control` says who controls whom and every
decision in this world is still taken by the party it is about, so a controlled firm invests what it
would have invested and pays what it would have paid. A buyout that changes nothing about how the
company is run is a buyout with the operating half missing (§35 D4, §29 C1).

- [ ] 17c.1 **A controller's say over what its company builds** (§29 C2, Capital Programme B1). The
  investment decision reads the DECIDER's cost of money rather than the firm's own where the firm is
  controlled — which is the whole of what a sponsor changes about a company and is one read, not a
  second decision (Law 4).
- [ ] 17c.2 **A controller's say over what its company spends** (§29 C2, Firm B2). The cost base is
  the firm's and stays the firm's; what a controller supplies is a target for it, which the firm
  meets by the ordinary decisions (hiring, inputs, plant) and not by a number written onto its books.
- [ ] 17c.3 §35 D4 and §29 C2 re-marked; record.

**Exit.** A company under an owner invests and spends differently from the same company standing on
its own, and the difference is a read of the owner's own numbers rather than an override.

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

- [ ] 18a.1 `money-market/policy.ts`: the central bank's own outlook on the price level from the goods prints in its money weighted by its basket read; compared with its TARGET (policy param; owner `parliament` after 19). On its own calendar it moves the rate one STEP towards closing the gap it sees, or not. No coefficient. Journaled `centralBank.rate` with outlook and surprise. Written through `params.setByMandate` (19.1's door, granted to this module for the rate until 19). **And (12d.3):** the outlook reads the environment condition of its regions beside the prints (`registry/environment.ts conditionsIn`), so a cold winter's fuel prints reach it as weather and not as the level. **And (16.7, 16.8):** the reserve reason — a target for what a central bank holds in other moneys and the rule by which it buys or sells them in the pairs (Central Bank F1, F2) — carried here as 21.54 and 21.55: no central bank holds anything abroad since 16.7, and the pairs stop clearing by period 8 of the four-country world.
- [ ] 18a.2 The quantity response: at the new rate the facilities re-price and the desk supplies or drains reserves until the overnight print sits in the corridor; both legs on both balance sheets.
- [ ] 18a.3 `central-bank-omo`: a SCHEDULE (the level at which its reason stops), never `price: 'market'`; delete `CB_PARAMS.targetShare` — and with it `seed.centralBank.openingHoldingShare` (16.7's other half): the central bank opens holding what its schedule buys in the first session, and the seed sizes the banks' reserves from their own liquidity rule rather than from a stated share of every line.
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

- [ ] 21.76 `reporting/index.ts`: IN TWENTY-FOUR PERIODS OF THE RIG ONLY CELLS PUBLISH ACCOUNTS. The seventeen reports at period 20 are household and small-business cells; the first NAMED FIRM publishes at period 25, and firms' year-ends are staggered after that. Everything that reads a company's published accounts therefore has nothing to read for most of a run: `control/index.ts worthAt` (a bid is what the target PUBLISHED, so no named company can be valued at all), `corporate-bond openLine` (a covenant is its published accounts), and 17b.8a's facility covenant (a lender with nothing to test asks for no promise). It is the DEEPER CAUSE of 21.73 — `control` never runs — and it is not obviously a defect: a quarter is thirteen weeks and a staggered year-end is real. What is not established is whether a named firm publishes on the same cadence a cell does once it has started. Positioned at 23.1 with 21.73, where the scale model is resized and a world that can be measured before period 25 is what is wanted (17b.8a).
- [ ] 21.75 17b.7's change is not covered by a test and cannot be in any world this repository builds: a deal struck in the money the shares are in only differs from one struck in the buyer's where the two moneys differ, and the rig is ONE COUNTRY while `control` has never run in the four-country world (21.73). What the change fixes was visible by reading — every price carried `currencyOf(buyer.region)` whatever the line was, so a foreign seller would have been paid in the wrong money (Law 8) — and it is fixed; what is missing is the world that would show it. Positioned at 23.1 with the scale model's resize, which is where a buyer and a company in two moneys become possible (17b.7).
- [ ] 21.74 `committed-capital.test.ts:45` asserts `expect(undrawnOn(promised(100_000_000, 0))).toBe(100_000_000)` — a `Cash` against a number, so it can never pass. The two lines under it read `.pieces` and do. Pre-existing (1 red, 5 green at `3f7b4e1`; 1 red, 9 green after 17b.4's four). A wrong assertion is not a finding about the world and is not chased here (the three rules); positioned at 23.1 with the rest of the suite's triage (17b.4).
- [ ] 21.73 `control/index.ts` NEVER RUNS. Twenty-four periods of the `opens` rig: `control.tender` 0, `control.failed` 0, `control.acquired` 0, `control.financing` 0 — so §35 and §29 B are a thousand lines nothing has ever exercised in an assembled world, and every test of them is a scale model built by hand. `couldBuy` finds six candidates in period 24 and all six are index ETFs (`etf.us`, `etf.equity.large.us.1`, …): the rig draws no buyout fund at all (`drawPrivateEquity` needs a bank at or above `SPONSOR_SIZE` and the rig's banks are smaller), and a tracker has no view of a company to value one with. Positioned at 23.1, which is where the scale model is resized and is exactly this: a test never names a party, so the draw has to make a buyer (17b.2).
- [ ] 21.72 `banks/index.ts publishQuotes` STOPS QUOTING EVERY NAME. `opens` rig, quotes per period: 121, 102, 81, 55, 62, 62 … and 0 from period 9 or so to the end of twenty-four, with declines rising the other way (12 at p12, 106 at p24). The reasons the banks give for the declines after period 10: 178× `appetite`, 58× `it cannot cost its own funding`, 61× `nobody lends to a party of this kind`. A world where no bank will quote anybody has no credit market at all, and §29 B2.b (*“the credit market decides which buyouts occur”*) cannot be exercised in it. NOT CHASED (Law 11): appetite is the bank's own room and the room is made of capital, funding and what it already has out, none of which this reading separates. Positioned at 23.3, with Part XII's measurements, where what a bank's room is made of is measured with the level carried (17b.2).
- [ ] 21.71 `short-term-debt/index.ts BACKSTOP` and `registry/credit.ts FACILITY` are ONE OBJECT under two names (Law 4): a named bank's committed line to a named borrower, at a limit and a rate, whose undrawn headroom consumes the lender's capital and whose drawing is a loan row. The differences are a commitment FEE (the backstop has one, and §18 B4 is right that a line without one is a free option) and an END (the facility lapses, the backstop stands). Both are TERMS, not kinds. Merging them means moving §18's fee onto the kernel's shape and deciding whether an acquisition commitment is charged for, which is §18's economics and not §29's; positioned at 18 with commercial paper's own item (17b.1).
- [ ] 21.70 `registry/credit.ts LoanTerms.borrower` IS A MIRROR OF `Instrument.issuer` (Law 4), and the guard is what shows it: `banks/loan.ts reagree` has to REFUSE a change of borrower to keep the two in step, which is a check standing in for a fact with two writers. `creditorOf` already reads the holder off the register rather than the terms, for exactly this reason, and who OWES a row is the same shape seen from the other side. Delete the term; `borrowerOf(i)` beside `creditorOf` reads `i.issuer`; nine sites read it instead (`banks/index.ts` ×3, `money-market/index.ts` ×2, `housing/index.ts`, `banks/loan.ts` ×3), and the `reagree` clause goes with it. Found at 17b's first design, which wanted to move a liability by the `assume` leg and could not without the terms disagreeing with the register. Nothing is wrong today — no loan has ever changed hands as a liability — which is what makes it a mirror rather than a break (17b.0).
- [ ] 21.1 `households/decide` → `ordersFrom` throws on its own malformed row; one `wholeOrderOf(spend, price)` in `clearing/` for firms and households. **And (11.2a.2):** `PlannedOrder` declared three times (`registry/capital.ts`, `firms/decide.ts`, `households/index.ts`); the registry's is the one. **And (12.4a.2):** `firms/index.ts productionCosts` reads the seed's rows only, so a born firm's production is outside the family's check; the family should read the produce event's own line (it carries it) and not a row. **And (12b.3):** `capacityFrom` counts a fuel line's capacity in PIECES of output a period — 9.7e15 for thirty-nine million pieces of machinery over a plant need of 0.002 a unit at a million pieces a unit — and `downTick` refuses a count past 2^53 (`sb-line`, period 4). A capacity that large is not a count anybody makes: the grain of the good's piece against the grain of its plant is a RESOLUTION, and the overflow is the grid, not the capacity. **And (12c.3):** the same throw takes both `promotion.test.ts` tests and the entrant test of `growth.test.ts` at the period after a promotion in the `coalRaw` rig — a born firm's plant over its line's need — so a firm born in the scale model cannot be measured until the grid is.
- [ ] 21.2 `money-market/resolution.ts`: the acquirer's consideration is a leg; no forced buyer (`winner = taking[0]`; none → `bank.resolution.noBank`, the insurer bridges); `writeDownRow` reports the cash written down (no `Math.round`); the Banks Capital E3 conservation family built. **And (12a.9):** a customer's payment that fails because ITS BANK is refused at the central bank — twelve levies at period 3 of the confiscatory scale model — leaves no row on anybody: settlement writes the payer's arrear only when the payer was refused, and the bank that could not settle owes nothing on the record. A bank refused at the window has failed to deliver its customer's money (Money E1, Banks Capital C1.a); the row is the bank's.
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
- [ ] 21.20 `expectations`: the book of outlooks is keyed by party id and follows no cell event — a cell split off its parent by a hire, a release or a promotion starts with no outlook although its people have the parent's history, and a merge drops the arriving cell's; XI-15 says a cell is one group with one history, so the offspring's book is the parent's at the split and a merge weighs the two by their people (`observation.test.ts` skips the cells born after a statement for this; 12d.1).
- [ ] 21.22 `world/world.ts runOne`, `world/module.ts speculative`: WHETHER AN ORDER IS A VIEW is a fact about the ORDER, and the flag sits on the participant. A firm's bid for an input names a level from what it thinks the output will fetch less the wages a unit still needs — its own money behind its own opinion — and its ask for the output is its cost (F15), which is not; one participant posts both, so the firms' participant is not flagged and every wholesale goods book journals `market.noView` with firms bidding in it (`no-view.test.ts`, 12d.4: `bread`, `flour`, `power`, `itServices`, `facilities`, `chemicals`, `meat`, `cloth`, `steel`… in period 30 of the rig). The order carries the mark (`Order.view`), the participant sets it per order, the kernel counts orders and the participant flag goes; the households' saver and the merchants keep saying what they already say, per order. Until then the census counts a firm's view as none and 12d.4's test is red.
- [ ] 21.23 `registry/capital.ts requiredOnEquity` reads what a company's equity costs as its earnings yield — its own earnings outlook over what the market says it is worth — so a listed company that has not yet earned is told its equity costs NOTHING, and the listed insurer of `quote-37` quoted a price of nothing for it in two periods of six (14.4). What shareholders require of a claim is never below what the curve pays them for holding nothing of it; the read wants the floor the market itself sets, not a yield of nothing.
- [ ] 21.24 `insurers/pensions.ts sponsorshipRowKind` carries no value: a pension fund's shortfall sits on the fund's equity and on the members' as the mark moves, and on the SPONSOR only as each call arrives — an employer standing behind a scheme short by a year of its payroll shows nothing of it until it pays (Insurers D3, Law 5, IAS 19 in the world this reflects). The covenant is a row the kernel can value — the sponsor's share of the fund's shortfall, read off the fund's marks and its equity in the same pass — and the circularity (the covenant is an asset of the fund whose shortfall it values) wants the valuation to read the fund's equity BEFORE its covenants, which `RowValuationReads` does not yet offer (14.6).
- [ ] 21.25 `insurers/pensions.ts pensionPerMember`: a pension indexed to the trade most of a place worked in when the promise opened is worth nothing and pays nothing in a period nobody works in that trade — the going rate is a read of the rows and there are none — and the row says so only in its mark. A promise indexed to a rate is a promise indexed to the LAST rate struck until a new one is (Law 8: a level with no history is a lie, and this is the reverse — a history with no level); `goingRatePublishedAt` has the last print and the read should fall back to it, dated (14.6).
- [ ] 21.26 `funds/index.ts`: the money fund of a bank winds up at a NAV of NOTHING (`fund.money.bank.a`, `exp-f`, period 16 — every holder queued at a price of nothing by `queueEverybody`), and `redeem` divided what it could pay by that price, which stopped the world. 14.7 made a share struck at nothing owed nothing, said on the record; what is NOT answered is why the pool's whole book came to nothing — the paper it held and who failed on it — and what a holder of a share worth nothing in a pool that has ceased is left holding (XI-3: the pool's death has to have a destination for its issued line).
- [ ] 21.27 `land/index.ts` seed: the ground under the opening's plant is endowed to its holder at nothing and NOT pledged to the vintages standing on it — a seed states stocks and cannot write a lien — so a firm can sell the hectares from under its opening plant while it cannot from under a vintage it commissioned (15.1). The opening is where 22a says every opening belongs; the lien wants a seed door that can state an encumbrance the world opened with (XI-8's `openCommitment` for a lien).
- [ ] 21.28 `registry/capital.ts project`, `capital-programme/index.ts groundToCarry`: machines the programme refuses to stand — no ground under them — stay as goods the firm holds, and the project does not count them: next period it bids for the ground and then for machines AGAIN, so a firm short of ground buys its plant twice. The project's plant need wants to read what it already holds of the capital good (`view.quantity(goodId(kind.madeFrom, region))`) before it bids (15.1).
- [ ] 21.29 `small-business`: a pool of small firms holds no ground and its rooms stand on nobody's — the seed gives it plant and no hectares, `commissionOne` refuses nothing of it because a pool builds through no purchase the programme reads, and a room a pool rents is a LEASE, which is 15.3's (commercial property). Until then the ground under every pool's plant is counted as the authority's unbuilt ground (15.1).
- [ ] 21.30 `registry/ports.ts`, `freight/index.ts berthFree`: a quay has an owner and a berth and the owner EARNS NOTHING — port dues are a price and a price is cleared (Law 3), which wants a berth session per quay per period that carriers bid in for the calls they want to make, the authority offering its berths, congestion then a price as well as a queue; until then a call is free and the queue is the whole of what a berth does (15.2).
- [ ] 21.31 `test/freight.test.ts`: three older tests are red at 15.1's HEAD and were not counted in the eighteen-suite baseline — `runs every leg every period and reports the outcome of each` (no leg in `byLeg`), `prints the same grade separately in every place that makes it` and `sources locally where the thing is made` (one place where two were expected). The rig's legs and places have moved under them since 13c; whether the world or the assertion is stale is a read to make against `legsBetween` and the draw's regions (15.2 found them and did not chase them).
- [ ] 21.32 `property/index.ts tenantOrders`, `firms/decide.ts`: no firm's plan binds on premises in thirty periods of the rig — every `firms.plan` says `bound: demand` or `bound: labour` — so the lettings books print `noDemand` every period and the only leases signed are a test's. The tenant's reason is built; what is missing is a firm short of ROOM: the service lines (13c.2) whose recipes take premises open holding their own, and nothing makes a firm outgrow them. When a line's demand outruns its room (22.3's shops with buyers), the bid is there (15.3).
- [ ] 21.33 `property/index.ts collect`: a bank resolution merges the landlords of the failed bank into another bank's cell (`weight: merge`), and a lease signed with the smaller cell then comes to less than a piece of money a landlord — said as `property.rentPaid` unpaid, never fixed. A lease is with EVERY member of the cell alike (XI-15), and a merge changes who that is; the honest row is one per landlord member, which is a cell event the register does not yet have (a lease that splits with the cell, as 21.20 asks of outlooks) (15.3).
- [ ] 21.34 `property/index.ts buildOf`: a landlord builds by buying buildings from whoever sells them in its place, and the rig has no builder selling buildings — the `building` good's market prints nothing — so its pace is a bid nobody fills and its ask for a loan is against a purchase that never happens. The builder is 13c.2's construction line; until it sells, no building is built to let after the opening (15.3).
- [ ] 21.35 `ledger/settlement.ts` (the equity routing, ARCHITECTURE 4.9a): a CELL's equity account is per member and a money leg to a cell is a TOTAL — a wage of `perMember × members` lands as one amount — and the routing books the total per member, so a household cell's equity walk leaves its own balance sheet by the weight from the first payment it receives (`hh.working.bank.a`: 87.8m a member against 24.2m at mark in period 1 of the rig, after 15.4 made the marks right). The `accounts` family's cell rows are this. What a leg does to a cell's account is the amount over the weight, as the marks now are (15.4).
- [ ] 21.36 `estate/index.ts`, `ledger/settlement.ts`, `runCorporateActions`: while an estate's programme runs, every claim on the dead firm is PRESENTED AGAIN each period — a paper line's maturity that failed last period matures again this one — and each failure writes the estate an arrear (`estate.firm.# owes … it could not pay on instruction #`), and each arrear matures, fails and writes another, so the dead firm's debt stands twice on the register (the paper AND its arrears, Law 4) and the count grows by every claim every period: the 40-firm control rig runs 1,200 ledger records a period to period 22 and 27,000 from period 26 (3,834 paper maturities failed, 3,834 estate arrears written, 3,753 arrears failed in period 24 alone), and `control.test.ts` *bids for nothing* times out at 180 s where it took eleven seconds at 15.3. `distribute` says what is not paid *stays outstanding until the estate closes* (D2.a, Banks Lending E5): a claim in an open estate is owed to the estate's rank order and not to the calendar, so its maturity is presented once and its failure is the estate's record, not a new row. Seen at 15.4 after the cells' marks changed the path; the estate is 21.3's.
- [ ] 21.37 `goods/index.ts unitsOf`, `goods/data.ts`: the dwelling good is subdivided a million ways (`PIECES_PER_UNIT`, the tonne's grid) while the comment beside it says a good counted in whole things has no piece below one of itself — a cell of nineteen thousand holds 7,866 dwellings as 7.87 billion pieces and a member holds four tenths of a roof, which is the occupancy it lives under and not a thing anybody holds (Housing A1: indivisible). The register should count whole dwellings and the book the occupancy (15.5 moved the book to `DWELLING_WEEKS`; the register is still on the tonne's grid) (15.5).
- [ ] 21.38 `housing/index.ts reservation`, `letIn`: the most a tenant will pay is what it expects to earn over the roof it lives under, and the session prices at the marginal bid — so a session whose only bidder left is a rich singleton clears at its whole income (period 3 of the rig: 3.117 a piece, 3.1 million a dwelling a period, 1.2 dwellings) and the same tenant then fails the rent it bid. The reservation is what the module's header says it is (B1.a from the other side); what is missing is the rest of the tenant's budget — it bids as if a roof were the only thing it buys, where `consume.ts` decides the rest of its spending per cell (Households C1). The roof's bid belongs in the same budget the basket and the home come out of (15.5).
- [ ] 21.39 `housing/index.ts publishShortfall`, `households/index.ts homeBid`: a household that RENTS never buys — `short = needs − owned − rented` is zero once it is housed, `homeBid` never fires and no mortgage is asked for, so owner-occupation cannot arise from a renting household and the sector is renters for ever. Housing B1, B2: a buyer's reason is that owning is cheaper than renting at what it can borrow — rent against the mortgage payment, both reads — and nothing here compares them. `mortgage.test.ts` buys the roof its row stands on by hand until it does (15.5).
- [ ] 21.40 `banks/index.ts overdraft`, `test/mortgage.test.ts`: a household cell that holds a roof is lent to on its request (`bank.c lends 4,178,362 to hh.working.bank.c.11`, period 4) and draws on its line when its account is emptied by hand, so the coupon on the row the test built is paid out of the bank's own advance and no default is ever recorded — *when it cannot pay, the default is recorded on the row* is red since 15.5 and cannot be staged from outside: whether a bank funds a borrower is the bank's decision, and the test's scenario needs a bank whose standard refuses (17.0's credit view). Seen at 15.5; the row was green while no bank quoted a household name (12a.4).
- [ ] 21.41 `treasury/index.ts`, `test/treasury.test.ts` *funds itself over a year when the market is there*: eleven of the treasury's instructions fail in a year of the rig where none did at 15.4 — the test asserts no failed instruction and lists eleven. Seen at 15.5 after the landlords' dwellings and their rents changed where the money sits (the banks' funding and what they lend differ from period 2); which payments failed and why is a read of the eleven records to make against the funding programme (Treasury D3, XI-9) — measured, not diagnosed (Law 11).
- [ ] 21.42 `indices/baskets.ts`: the consumer basket is physical goods a member buys and the rent a member pays is not in it, so the price level a household or the central bank reads leaves out what Housing D3 calls a large component of it; the rent is a row's terms (`rentOwedBy`) and the book's print (`housing.rent`), both readable, and the basket that includes them is one basket and not a second index (Law 4) (15.7).
- [ ] 21.44 `small-business/index.ts` promotion, `control/index.ts`: a size is compared in ONE money, so the boundary a small firm crosses to be promoted is kept per money and a small firm in a money where no named firm has brought paper is never promoted, however large (Currency C4: comparing sizes across moneys is a REPORT at a rate, and the promotion is a decision — 16.0 chose to compare within a money rather than translate at the rate in force; whether a size boundary is a money's or the world's is a design question for 16.2's table) (16.0).
- [ ] 21.45 `observer/observer.ts sectors`, `indices/index.ts`: the observer reports a sector's output and stock in the FIRST region's money, and the global equity line in `statedIn`; both are reporting choices translated at the rate in force (Currency C4) and both are declared in code rather than as a RESOLUTION on the parameter register with an invariance test (Law 2: a report's numéraire must move nothing) (16.0).
- [ ] 21.46 `banks/index.ts costOfFunds`, `banks/dealing.ts bookValue`, `short-term-debt headroomFor`: a bank's capital behind a book in a foreign money, a desk's room and a paper buyer's concentration limit are its home-money residual TRANSLATED at the rate in force; the rate moving therefore moves the room without a trade (Currency D2 makes the move a revaluation, but nothing yet hedges it — the owner's correction says a corporate CAN hedge; no party has a reason to in this world) (16.0).
- [ ] 21.47 the four-country world at thirty periods (16.2's table): about one session in a hundred clears (78 of 6,825 in EUR, 48 of 11,965 in JPY), and a hundred to two hundred parties per money are dead — one bank per money in the scale model, and a treasury (`treasury.us`) refused an overdraft of 1.6 × 10¹¹ pieces at its central bank every period from 29. Not chased (Law 11): a measurement for Part XII, and the sovereign's failure is XI-9 doing what it is for; positioned here so 16.8's `check:opens` on `abroadWorld` reads it against this table (16.2).
- [ ] 21.54 Central Bank F1, F2 (16.7): no central bank in this world has a reason to hold another money's paper — the stated share is gone and nothing buys reserves — so F1's reserves are zero, F2's intervention has nothing to intervene with, and the FX pairs open with less two-way flow than the seed used to plant. The reason is a policy decision (a reserve target, an intervention rule) and belongs with 18a's mandate; positioned at 18a.1 (16.7).
- [x] 21.68 **Answered at 17.10a, not a defect.** The assessors do speak: 240 `rating.action` events in sixty periods of `rated-a`, the first at period 21, behind 236 statements and 1,192 disclosures. What the censuses had found was the CALENDAR — a quarter is thirteen weeks, nothing is reported on one that began before the epoch (Seed A2), and both censuses stepped fewer periods than the first close takes. A test pins the chain and its order (17.10a).
- [ ] 21.69 `mechanisms/ratings/assess.ts` (17.10a's census, sixty periods of `rated-a`): EVERY GRADE THIS WORLD HAS EVER PUBLISHED IS THE WORST ONE. All 240 rating actions carry `c`, the bottom of the scale — no name is ever graded anything else by any of the three assessors. So the investment-grade index is permanently empty while the high-yield one carries every rated line, every claim takes the regulation's worst weight where a grade is read, and the assessors cannot disagree with each other about anything (Ratings A4: two houses looking at one issuer should not always agree). It may be a true reading of a world whose banks open in breach and whose firms fail (21.66), or the measure may be saturated; which it is, is a read to make against a run and not a number to chase (Law 11). Positioned at 23.3, with Part XII's measurements (17.10a).
- [ ] 21.67 `test/money-market.test.ts`, the assembled world (17.7d's census of five periods of the rig): FIFTEEN OF THE SIXTEEN TESTS IN THAT FILE ARE RED, AND NONE OF THEM IS ABOUT THE MONEY MARKET. Each ends in `expect(unexpected(w.step().audit)).toEqual([])` — which is every violation the audit reported, of every family — so each is asserting that the WHOLE WORLD is clean, and the assembled world reports 79 to 325 violations a period across four families: `prices` 9 (a listed line is held and the session printed no price for it), `accounts` 23–36, `names` 18, `flows` 20–127, and `units` 148 in the one period it fires. They were red at `af70a23` and are unchanged by 17.7d (verified in a worktree at that commit — same fifteen, same names). 17.9 found the same shape one file over and worse: `test/bank-capital.test.ts` DOES NOT COLLECT AT ALL — its `run` helper asserts an empty audit at module scope, so the file reports “no tests” and every case in it has been silently unrun. Two different things and both are written down here: the SHAPE of the test, which makes a money-market case fail for a reason in the goods market (positioned at 23.1, where the scale model and what a test asserts are resized together); and each family's own cause, which is a read to make against a run and never a number to chase (Law 11), positioned at 23.3 with the rest of Part XII's measurements (17.7d).
- [ ] 21.66 `mechanisms/banks/capital.ts`, the seed (17.7b's census, re-read at 17.9): EVERY BANK IN THE SCALE MODEL OPENS IN CAPITAL BREACH, IN PERIOD ONE, with negative headroom — bank.a at a weighted ratio of 0.089 against a minimum of 0.08 plus its own buffer of 0.039, bank.c below the requirement outright. **17.9 built the answer to it and it is not enough**: a line is now told to come down, the desk sells at market, and the credit side came back to life (38 loans written in period 2 where the world had written none after period 5). What it has not done is close the hole — two of the three banks are insolvent by period 3 and stop publishing anything, and the third's headroom runs from −34bn to −102bn over twelve periods while it sheds. A bank that cannot shed its way back is its RESOLVER's, and nothing resolves it. Positioned at 23.2, which is that sentence's item (17.9).
- [ ] 21.65 `mechanisms/trade-credit/index.ts`, the rig's draw (17.7a's census, twelve periods of four seeds): THE TIER TRADE CREDIT LIVES ON DOES NOT USE IT. Every invoice the scale model writes is one firm on the TREASURY — seventeen of them, one seller, one buyer — and no firm ships another firm on terms at all, which is the connection §42 A4 calls *"the tier that lives on it"*. One seed of the four (`credit-terms`) writes no invoice in twelve periods. And the one buyer this world has keeps no deposit at a commercial bank (`PartyKindProfile.depositClass` is null for a treasury, rightly — it banks at the central bank), so it never knows what its money earns and never takes a discount: the early-payment mechanism is exercised by a test and by nothing in the assembled world. Why firms do not sell each other on terms is a read to make against a run and not a number to chase (Law 11); positioned at 23.1, where the scale model is resized as one bounded change (17.7a).
- [ ] 21.64 `mechanisms/banks/index.ts`, `world/actions.ts` (17.7's census of the 30-period rig): EVERY BANK IN THE SCALE MODEL IS INSOLVENT FROM PERIOD SIX AND NONE IS RESOLVED. Twenty-six `bank.insolvent` events; `bank.c` closes with a residual of −5.0 × 10⁹ against 61 × 10⁹ owed. An insolvent bank publishes no cost of funds (17.0, deliberately: there is nothing for its owners to require a return on), so it forms NO CREDIT VIEW — and with no view there is no quote, no reservation and no workout. `credit.written` is 3, 2, 0 in periods 3–5 and zero in every period after, so the whole credit side of the world stops at period six and what comes after it is a world of twenty-five loans nobody can add to. Not chased (Law 11): the missing mechanism is the resolution that should have taken these banks, and it is 23.2's (17.7).
- [x] 21.63 **Answered at 17.9a, not a defect.** A live loan row with nothing outstanding is an UNDRAWN LINE, which is what C9 says a line is: one row per (lender, borrower) that its outstanding moves on, drawn and repaid at the borrower's option. The five such rows in the thirty-period rig are prime-brokerage and overdraft lines their borrowers had paid back (`prime repay`), and every one of them is `performing: true`; a TERM loan that amortises to nothing does cease, at the redemption that empties it (`world/actions.ts redeem`). What the original note also said — that two of them were still not performing — was a mis-pairing of two reads, and no such row exists (17.9a).
- [ ] 21.62 `register/agreements.ts` (17.7): AN AGREEMENT HAS NO CURE. `breached` reaches only `discharged` (paid in full) or `terminated` (written off), so a commitment renegotiated after a missed payment has nowhere to land — the transition 17.7 built for a ROW in the register has no twin for the row beside it. Nothing in this world re-agrees one yet, which is why it was not built with the instrument's (Law 14); the first that will is an arrear a payer is given time on. Positioned at 19 (17.9b): the write-off's own sibling turned out not to need it — an instrument's end is the kernel's and books nothing — and the first thing that will ask for an agreement to be CURED is a payer given time on an arrear, which is the polity's levy (17.7, 17.9b).
- [ ] 21.60 `corporate-bond/index.ts` (17.4, listed there and not built): THREE OF THE ITEM'S REASONS ARE STILL MISSING. (a) TENOR, SIZE AND DIVERSIFICATION as separate reasons: the tenor is a market convention read from one parameter and the size is what the firm is short of, so a management that wants a shorter line or a second maturity to spread what falls due has no way to say so. (b) COVENANTS AS A TERM OF THE HOLDER'S BID: what an issuer promises is still the arithmetic of its own accounts as this borrowing leaves them (the tightest covenant a lender could ask for), because no holder bids a covenant — B2's negotiation has one side. (c) 21.49's CROSS-BORDER C2: a firm issues in the money of its published need, which is its home money, so the lender base and the cost it would read in another money never enter the decision. Each is a mechanism and not a number. **(a) closed at 17b.8**: `CreditAsk.months` — a term is a decision about a NEED and the need is the borrower's, so every borrower says how long it wants the money for and the row is written for that. What it replaced was worse than this finding said: `lending.loanMonths` was the term of EVERY loan in the world, so a mortgage ran twelve months. Diversification (a second maturity so that less falls due at once) is NOT closed and is re-positioned at 18a.1 with the other funding-shape reads. **(b) is 17b.8a**, inserted after 17b.8: a covenant is a term of the commitment a lender bargains for, and a buyout is where it bites. (c) is NOT closed by 17b.7: what 17b.7 settled is that a DEAL is struck in the money the shares are in (Cross-Border C4), which is a unit and not a choice — a firm CHOOSING the money it issues in needs the covered comparison (a forward on the pair for the tenor, Currency B4) that 21.50 also waits on, so it is re-positioned at 18a.1 with the other cross-border reads (21.54, 21.55) (17.4, 17b.7).
- [ ] 21.58 `corporate-bond/index.ts issueBonds`, `test/corporate-bond.test.ts` *brings paper at all* and *has something to test* (red at the close of item 16 and red now; not caused at 17.1 — the same two were red in a worktree at `319b7c2`): NO FIRM IN `ranWorld('bond-issue', 10)` BRINGS PAPER. The world publishes 116 funding needs, 539 credit quotes, 14 reservations and 3 benchmark fixings in ten periods, and zero `bond.offered` and zero `bond.refused` — so `place` is either not reached or returns before it offers. The gates between a published short and an offer are `shortOf` (the LONG-term half of the gap), `wouldHold` (somebody published what they require of the name), and `cheaper || !enough` (the market beats the bank, or the bank will not lend enough). Which of them holds every firm back is a read to make against a run, not a number to chase (Law 11). It is the same absence 10.3's covenant table is waiting on, and it is where 17.2's arranger will land (17.1).
- [ ] 21.57 `tools/check-forbids.ts PEEK_BASELINE` (17.0a): 96 sites in 41 mechanism files take another party's own view (`ctx.participant(x)`, `ctx.blind(x)`) — its holdings, its equity, what it took in, what it owes — and a module that asks for a COUNTERPARTY's is reading state that party never showed it (Observer A4). Some are the module's own party deciding, which is what the door is for; the rest should be a disclosure or a published read. The count per file is ratcheted and may only fall; which sites are which is a read to make file by file, not a number to chase (Law 11). The heaviest are `banks/index.ts` (13), `money-market/index.ts` (11), `housing/index.ts` (5) and `freight/index.ts` (4) (17.0a).
- [ ] 21.55 `spot-fx`, `money-market swapDraw` (16.8's census of the four-country `opens` world, twelve periods): the six pairs clear 3–6 sessions a period through period 4 and NONE from period 8, while five banks a period draw on the swap line to settle what they owe abroad and no cargo sails in twelve periods (`cargo=0` every period). A pair with no seller of the foreign money is `noSupply`, and the swap line is the buyer of last resort a pair should not need (Spot FX C3); why the sellers leave — the central banks hold nothing abroad after 16.7, and a bank that has drawn on the line holds no foreign money to sell — is not chased (Law 11). Positioned at 18a.1 with 21.54, whose reserve reason is the missing side (16.8).
- [ ] 21.52 `corporate-bond/index.ts place()`: a firm's published short came to 1.385 × 10²² pieces in `abroadWorld('fx-A')` — more units at its walk-away than a whole number can count — and the tick threw (Law 8). The issue is refused at the site and said (`bond.refused`); the number upstream is a firm's `firms.funding` short that no mechanism bounds and nothing yet explains; a measurement for Part XII, positioned with 17.0's credit view, which is what a lender would read before quoting it (16.6).
- [ ] 21.53 `derivative-layer`, `spot-fx` (16.6): once every kind is asked whether it owes a money it has not got, the USD clearing house squares its foreign margin balances in the pairs and CEASES by period 2–4 of the `fx-A` and `opens` worlds; a member defaulting into the dead house then tore up against the dead name (fixed where it was: margin is returned to a side's living successor). Why a house that holds members' margin in four moneys fails once it trades them is not chased (Law 11); positioned at 18.4 with the house's own book (16.6).
- [ ] 21.51 `clearing/market.ts runMarket`, `world.ts settleTransacts` (16.5): a book in which a `transact` group's legs settled BESIDE ordinary trades prints the ordinary trades' volume only — the group settles after the book has printed and a period has one print (Clearing F2) — so the print's `qty` understates what moved there by the trip's legs; the trip's volume per book is on `transact.settled`. A book whose only trades were a trip's prints when the trip settles, which is right. Positioned at 18.0 with the print's dimension (a print could carry what settled after it as a second provenance row) (16.5).
- [ ] 21.50 `treasury/index.ts fundingCostIn`: the cost of borrowing abroad is the benchmark yield plus the treasury's ONE-PERIOD outlook of the pair extrapolated over a year; a covered comparison would read the forward on the pair (`fx-derivatives`) for the tenor and pay the basis (Currency B4), and none does yet. Both are honest reads a party has, but a treasury with a naive outlook borrows in the lowest nominal yield every auction (the first four-country run picked EUR at once); positioned at 16.5, where the swap line gives it the covered rate to read (16.4).
- [ ] 21.48 `freight.test.ts`, three red before 16.3 and after it (found at 16.3, not caused there — the 15.7 tree runs the same three red): *runs every leg every period* finds a `freight.session` with no leg in it (a period in which no place had anything to ship writes an empty record), and *prints the same grade separately in every place that makes it* and *sources locally where the thing is made* find grain made in ONE place in the `basis-a`/`subs-a` draws — the rig's draw puts a line in one region, so a location basis cannot form in the scale model (Commodities Spot D1, Freight D3). The first is the freight module's (record the period as `noDemand` on every leg, or nothing); the other two are the draw's (`rig.ts drawFirms` per line per region) (16.3).
- [ ] 21.43 `goods/index.ts goods.spoilage`, `housing/index.ts`: a dwelling wears out and nobody can maintain one — spoilage leaves the holder's stock every period and there is no outlay that stops it, so an owner's only answer to wear is to hold less (Housing A5: it depreciates AND needs maintenance, a real cost to the owner). Maintenance is a purchase of inputs and hours against the wear, and the owner's reason to make it is the rent it keeps; positioned with 22.2's upkeep per period, which is the same mechanism for plant (15.7).
- [x] 21.19 **Closed at 15.4** (`lastMarkOf`: every mark in the file is the last print, and a line never printed is worth nothing a lender lends against). `securities-lending/index.ts openLoan` (and the four other `markPerUnit` reads in the file) mark the paper AT THIS PERIOD from a phase anchored `after: corporateActions`, before the paper's market has run: period 55 of the `growth` rig throws `NotYetProduced` for `treasury.us.2028-06-15` and the world stops there (12c.3). A lender lends against the last print and its age (the `worthOf` read), or the phase sits after the markets; the throw is the phase in the wrong place, as `prices/value.ts` says it is.

---

## 22. The recipe

- [ ] 22.1 A line may declare more than one recipe (technology, data); the firm picks by its own cost read.
- [ ] 22.2 A recipe declares a batch; a vintage declares an upkeep per period (technology); a line below batch stops. **Finding (12c.3):** in a year of the scale model the twelve named firms produce ONCE — period 1, off the seed's work in progress — and never again: every plan after it is `batch 0, bound demand`, because the stock the seed gave them (314 million loaves at one baker) exceeds what they expect to sell (27 million a period) for longer than they live; by period 52 six of twelve are dead and thirty-seven estates are open, and output per hour has nothing to be measured on. A line that never starts is the absence this item's batch names from the other side; the seed's opening stock is 22a's.
- [ ] 22.3 Goods A2 as specified. **Finding (12.1):** no service line has a buyer in the scale model — every capacity line's print is stale from period 0 with `noDemand` (the households' basket and the firms' overheads name no service) — so the small-firm tier sells nothing there and nothing can be founded; the demand for services is the recipe's and the basket's to state. **And (12b.3, from 12d.1):** with a firm's ask at its cost, a machinery maker asks 0.616 a piece where the print had been 630,879 — the print was a belief asking the market to agree — and a buyer's gap takes thirty-nine million pieces of it in the `sb-line` scale model at period 3; the price level of this world is what these asks were holding up (the capacity throw it sets off is 21.1's). **And (12b.3, from 12a.8 and 12d.1):** the banking venue's print — sixty-seven billion an hour on one bank's bid at any price — is forty-three million an hour at period 4 of `sb-found` and thirteen million by period 8 once the bid is the bank's outlook on its earnings; still a price nobody else in the world pays for an hour, because the `banking` occupation has one kind of bidder: what a bank's hour is worth beside a baker's is the recipe's to state. **And (14.1):** with a pool's payout back to what it was actually paid, the one listed firm of the (4, 40) rig fails in period 21 unable to pay 44.8m that fell due, where it lived to report in period 29 before — a saver's own subscription coming back to it as a dividend was demand from nowhere, and what it held up is what falls; `equity-anchor.test.ts` has nothing to measure until a listed firm lives to its first report. **And (12d.4):** the retail books of a place with no household in it (`retailBread.us.1`, `retailMeat.us.1`, `retailClothing.us.1`, thirty periods of thirty) run with sellers and nobody to buy — Goods A1 opens a good where it can be made or sold, and a shop where nobody lives is a place with a good and no buyer, which is honest and is a fact about where the seed puts people.

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
