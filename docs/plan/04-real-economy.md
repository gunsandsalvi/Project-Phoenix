# Item 4 — Firms, goods, labour, households, outlooks

**Objective.** Firms that produce from a recipe, employ people, invoice buyers and sell to
households that earn, consume and save; every party deciding from its own outlook formed from its
own history. This item gives the model its first real cash flows, so that a loss (item 5) has
someone losing something. It is the largest item and is split into seven sub-items, each its own
commit.

**Read first.** Spec §32 Firm (all), §37 Goods A–F, §39 Labour A–D, §41 Households A–E, §46
Expectations (all), XI-16, XI-10 (first half: the employment register), XI-15, Part XIII step 4.
Code: `world/context.ts`, `world/module.ts`, `registry/kinds.ts`, `clearing/solver.ts`,
`register/register.ts` (lots), `world/revalue.ts`.

**Clauses this item meets.** Firm A1–A4, B1, B1.a, B1.b, B2, B3, B4, B4.a, B4.b, B5, B6, C1–C4,
C4.a, C4.b, D1–D5, E1, E2, E5, E6, E7, F1, F3, F4; Goods A1, A2, A2.a, A2.b, A2.c, A3, A4, B1,
B1.a–B1.d, B2, B3, B4, B5, B5.a, B5.b, C1–C6, E1, E2, E2.a, E2.c, E3, E4, E4.a, E5, F1, F2, F3,
F5, F5.a, F5.b; Labour A1, A2, A3, A3.a, A4, A4.a, A4.b, A4.c, B1, B1.a, B2, B3, B4, B5, C1, C1.a,
C2, C3, C5, D1, D1.a, D1.b, D1.c, D2, D2.a, D2.b, D3, D4, D5, E1, E2, E2.a, E3, E4, F1, F2, F3;
Households A1, A2, A2.a–A2.g, A3, B1–B5, B3.a, C1, C1.a–C1.d, C2, C3, C4, C5, D1, D1.a, D2, D3,
D4, D5, D5.a, D6, E5; Expectations A1–A5, B1, B1.a, B1.b, B2, B2.a, B3, B4, B5, C1, C2, C3, C4, C5,
D1–D4, E1–E4; XI-16; Sovereign E2.f; Treasury B3 (the cycle), C1 (income and consumption taxes),
C2, C3. Remain PARTIAL: Goods D (freight: 13c), Goods E2.b (broker-dealer: 9/13c), Goods G
(indices: 12), Labour A3.b, C4, E (13d), Households E1–E4, F (13d), Firm E3, E4 (10).

---

## Design

### Sub-items and modules

| sub-item | module         | spec                           | requires                          | owns                                                                                                                                   |
| -------- | -------------- | ------------------------------ | --------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| 4.1      | kernel door    | —                              | —                                 | module state slots; `view.outlook`; `profile.revalue` for carried-at-cost write-downs                                                  |
| 4.2      | `expectations` | Expectations, XI-16            | —                                 | every party's outlooks, surprises, confidence; the memory preference                                                                   |
| 4.3      | `goods`        | Goods A–C, E, F                | `expectations`                    | goods as instrument kinds with physical units; inventory lots at cost; recipes; a market per (region, sub-unit); lower-of-cost-and-NRV |
| 4.4      | `labour`       | Labour A–D, XI-10              | `expectations`                    | the employment register as rows; vacancies as bids; the matching market in hours; participation and acceptance                         |
| 4.5      | `firms`        | Firm, Goods B, F               | `goods`, `labour`, `expectations` | production, hiring, pricing, invoicing, payment, dividends; the firm's outlook use                                                     |
| 4.6      | `households`   | Households A–E, Sovereign E2.f | `goods`, `labour`, `expectations` | consumption from the cell's outlook, allocation across goods, saving, the deposit/bill substitution                                    |
| 4.7      | seed           | Seed                           | all above                         | firms with plant-less recipes, employment rows, household outlooks; the three seed shapes deleted                                      |

### 4.1 Kernel doors

- **State slots.** `MechanismContext.state<T>(name: string, initial: () => T): T` returns a
  module-owned object stored under `${moduleId}/${name}` in a kernel map; snapshotted by the observer
  (as JSON) and cloned with the world. A slot's writer is the module that created it; a second
  module asking for the same name is a construction error. Slots are keyed data, never a second copy
  of register or price state (the assembly test reads every slot and asserts no `qty`, `balance`,
  `price` fields in them; a naming rule, checked).
- **`view.outlook(variable: OutlookVariable): Outlook`** on `ParticipantView`: reads the
  expectations module's slot for `(self.id, variable)`. `Outlook = { expected: number, unit, per:
Periodicity, confidence: number (a read: the width of recent surprises), formed: Period }`. A
  party with no outlook for a variable gets `Missing` (never a default). `OutlookVariable` is a
  string registered by the module that publishes the observation (`goods.price.<subUnit>`,
  `labour.wage.<occupation>`, `household.income`, `firm.demand.<subUnit>`, `sovereign.yield.<line>`).
- **`profile.revalue?(lot, mark, period): { delta: number }`** on `InstrumentKindProfile`: for
  carried-at-cost kinds that must be written down (Goods E2), the revaluation phase asks the profile
  for the delta per lot; a positive delta is refused by the kernel for a non-dealer kind (E2.c is a
  FORBID: the kernel asserts `delta ≤ 0` unless the profile declares `fairValueThroughIncome`).

### 4.2 `expectations`

- State: `outlooks: Map<PartyId, Map<OutlookVariable, { expected, memory, surprises: ring }>>`.
- Parameters: `expectations.memory.mean` (preference, periods) and `expectations.memory.dispersion`
  (shape until a mechanism disperses it: Death: none; it is a preference's dispersion, the one shape
  XI-16 admits: "dispersed across parties, drawn once at entry"; declare it `preference` with its
  dispersion as `resolution`? No: the width of a preference's distribution across parties is a
  SHAPE. Declared shape, counted, justified in `why`).
- Phase `expectations.form` (cycle 0, before `corporateActions`): for every alive party and every
  variable it has observed, `expected_t = expected_{t-1} + (observed_{t-1} − expected_{t-1}) / memory`
  (B1: adaptively, at its own speed); confidence = the standard deviation of the last `memory`
  surprises (B3, a read). Journals nothing per party (private), publishes an aggregate
  `expectations.dispersion` (public, lagged one period: E2) as a read.
- Phase `expectations.score` (cycle 4, after `revaluation`): records `surprise = observed −
expected` for every variable the party observed this period (B2), as private events
  `expectations.surprise`. Observation sources: prints the party traded at, wages it received or paid,
  prices it paid, income it received, read from the ledger's instructions where the party is a side
  (A2: what that party observed, nothing else).
- FORBID guards: no variable named `market.*` or `expected.*` at the sector level (A2.b); the
  module's `form` phase reads only period t−1 and earlier (B4, D1).

### 4.3 `goods`

- Kinds: one instrument kind `good` with terms `{ subUnit, unit, perishable: boolean, storable }`;
  each sub-unit is registry data (`goods.registry`: id, unit, perishable, storage cost per unit per
  period as technology). `pricing: 'carriedAtCost'`, `liabilityOfIssuer: false` (a good is a real
  thing, not a claim), `unit` from the sub-unit, `revalue` implements lower-of-cost-and-NRV against
  the market's last print (E2, E2.a: never up), `due` none.
- Recipes: registry data `{ output: subUnit, inputs: { subUnit, qtyPerUnit }[], labourHoursPerUnit,
leadTimePeriods }` (technology, A2.a Leontief; A2.b: physical quantities, never value shares; the
  lint refuses a recipe expressed in currency).
- Markets: one per (region, sub-unit), opened at seed; participants: firms sell (their offers are
  finished lots at a price from their own outlook and unit cost), firms buy inputs, households buy
  consumption, the treasury buys public purchases (item 3's outlay programme gains a purchases line
  here), estates sell (item 7). Rationing: pro rata (C4, stated once). Price in the seller's currency
  (C6).
- Inventory: lots per (holder, good) with basis = unit cost (E1); spoilage removes units from lots at
  the lot's cost as a journaled `goods.spoilage` instruction? A unit that perishes is not a transfer:
  it is a **transformation on the holder's own book**, so the kernel needs an instruction kind whose
  leg has one side: **no**. Spoilage is a `writeOff` leg: asset leg from holder to **nobody** is
  forbidden (Register A3). Resolution: spoilage is settled as an asset leg to a registry-declared
  `sink` party of kind `nature`? The spec says units perish "recorded so the units identity can see
  them" (E4). Design: a kernel leg kind `destroy` (asset leaves the world: for physical goods only,
  whose kind profile declares `destructible`), journaled, counted by the units family as `perished`.
  Insert as a 4.1 door (`Leg.kind = 'destroy'`) with the equity effect −carrying value.
- Audit: `units` contribution: produced + opening = consumed + closing + perished, per (good,
  region), every period (D5), from the ledger's legs (produce = a `create` leg from the recipe's
  transformation: the mirror of destroy, allowed only for a kind that declares `producible` and only
  through the goods module's production phase, journaled with the inputs it consumed: F1 forbids
  consumption without production or inventory, so `create` is refused unless the same instruction
  destroys the recipe's inputs (Leontief) and work-in-progress accounting is in lots at cost).

### 4.4 `labour`

- Units: `hours`. Occupations: registry data (A3: skill, sector, region as key dimensions of a
  cohort's occupation; declared `labour.occupations`).
- The employment register: module state `rows: Map<rowId, { firm, cell, occupation, wage, start,
headcount }>` where `headcount` is always the whole cell's weight (A4.b, A4.c): a hire of part of a
  cell splits the cell first (`ctx.cells.split`) so the hired members become their own cell. Rows are
  journaled `labour.hire`, `labour.separation` (public: an observer would notice).
- Market: one market per (region, occupation) in hours per period, opened at seed; **every posting is
  a bid** (D1): a firm's vacancies are buy orders at the wage it offers for the hours it wants;
  household cells post sell orders for their members' hours at the cell's reservation (B1.a: never
  below the benefit it already receives as a share of the going rate: the benefit is an outlay of
  the treasury from item 3, so the reservation reads the cell's own last benefit receipt). The
  solver fills highest bids first, pro rata within a tie (D1, D1.a); the bid that took the last
  match is the print (D1). A match is a **hire**: the instruction is an asset-less agreement: the
  market runner creates the employment row instead of an asset leg, so the labour market uses the
  kernel solver but its own settlement: the `labour` module's phase `labour.match` (cycle 1, before
  `markets`) runs `clear()` directly on the orders it collected from participants and creates rows,
  splitting cells. Wages are then paid by the firm each period through `firms.pay` as money
  instructions per member (F1: from the employer's account).
- Stickiness (D2, D2.b): a row's wage is a contract; it changes only on renegotiation, which the firm
  does at its own horizon (D1.b) when the going rate (D1.c, a read: employment-weighted average of
  rows) has moved past its renegotiation cost (technology: `labour.renegotiationCost`, hours of the
  firm's own labour). Firing cost (C3): a severance of `n` periods' wage (policy) settled at
  separation; the asymmetry between hiring and firing is those two costs, never two speeds.
- Participation (B1): a cell's members are `employed | unemployed | inactive` per cell (B3: one state
  per cell; a change of state for part of a cell splits it). Search (B4): an unemployed cell posts
  hours every period; matching is imperfect because bids are finite and rationed (D3).
- Audit: `units` contribution: employed + unemployed + inactive weights = the population (B5), and
  every employment row's headcount equals its cell's weight (A4.c).

### 4.5 `firms`

- Phase `firms.decide` (cycle 0, after `expectations.form`, after `treasury.outlays`): for each
  firm, from its view: expected demand per sub-unit it makes (`outlook('firm.demand.<subUnit>')`),
  its inventory, its capacity (until item 10, capacity is the recipe's labour hours it can hire:
  B1.a's plant arrives at 10), inputs on hand (B1.b: a shortage is read and binds), labour (B1.c);
  decides the production quantity (B1) and the vacancies to post (Labour C1: hire when the expected
  price × marginal output exceeds the wage it must offer, from its outlook of the going rate), the
  price it will offer finished goods at (E1: its outlook of demand against its unit cost), and the
  dividend (E5: cash above its own buffer preference).
- Phase `firms.produce` (cycle 2, after `markets`): consumes inputs per the recipe (Goods B2),
  charges wages and the capital charge (none until 10) into work in progress at cost (B3, B5), yields
  finished lots after the lead time (B4: scrap as `destroy` legs at the lot's cost); a period that
  starts nothing capitalises nothing (B5.a); a throttled batch carries the whole period cost (B5.b).
  Idle labour is a period expense (F5.a).
- Phase `firms.invoice` (cycle 2): for every delivery this period, a receivable row (module state
  `invoices`, mirrored as a payable on the buyer, one object read from both sides: C4.b) with terms
  (Goods F2: immediate until trade credit at 13e makes terms a decision; declared placeholder
  `firms.terms.periods = 0` with death 13e).
- Phase `firms.pay` (cycle 2): wages to cells per member (Labour F1), invoices due, dividends;
  failures are recorded and read by item 5.
- Decisions are dispatch tables keyed by nothing: every firm decides the same way from its own state;
  what varies by industry is data (recipes, lead times), never a branch (Firm F4, Law 15).
- Published expectation (E7): the firm journals its expected earnings (public) and the surprise is
  scored by the expectations module.

### 4.6 `households`

- Phase `households.decide` (cycle 0, after `expectations.form`): per cell, from its view: expected
  income (`outlook('household.income')`), wealth (own holdings at last prints), liquidity (cash),
  confidence (C1.c, C1.d: a cell surprised widely holds more liquidity); consumption per member is
  a decision from those with the cell's preferences (patience and risk aversion drawn at entry,
  dispersed: preferences; the functional form is a SHAPE declared as such with a `why`); allocation
  across goods by preference weights and relative prices (C3: the weights are preferences per cohort,
  registry data); the residual is saving (C2); the portfolio decision between a deposit and a bill
  (D5.a): the cell posts a bid in the bill market when the bill's yield from the curve exceeds its
  deposit rate (zero until item 11) by its own liquidity preference.
- Participants: `household` cells in goods markets (buy orders per member at the most they will pay:
  C1 of Goods: the cell's outlook of the price plus its urgency), in the bill market (D5.a), in the
  labour markets (sell hours).
- Taxes: income tax on wages received and consumption tax on purchases, remitted by the payer to the
  treasury (Households B4, Treasury C1.a) as instructions in `households.pay`.
- Audit: `flows`: sector income equals what cells were paid, read from the ledger (B5).

### 4.7 Seed

Firms with recipes (a small graph: a raw sub-unit produced from labour only, an intermediate, a
consumer good), employment rows for the working cohort's cells at seeded wages (a shape until the
first matching prints: placeholder `seed.wage.<occupation>` dies at the first labour print), work in
progress consistent with lead times (Seed D1), households' outlooks initialised to their first
observation (the seed's own endowment flows: no history before period zero, so the first outlook is
the seed's stated expected income placeholder dying at the first observation), and the three
existing shapes (`seed.households.depositPerMember`, `bondPerMember`, `dispersion`) deleted: cell
endowments become the first period's wages and the seed's opening deposits are one period of wages
(a stock consistent with the flow, D1).

### Parameters (summary; each declared in its module)

Technology: recipes, lead times, storage costs, spoilage rates, renegotiation cost, hiring lag.
Preference: memory (with dispersion as shape), patience, risk aversion, liquidity preference,
consumption weights per cohort, firm buffer preference. Policy: severance periods, benefit
replacement rate, income and consumption tax rates (owner parliament). Placeholders: `firms.terms.periods`
(→13e), `seed.wage.<occupation>` (→ first print), `seed.expectedIncome` (→ first observation),
`labour.capacity.hoursPerFirm` (plant, →10).

### Files

```
packages/engine/src/world/context.ts, world.ts       (4.1: state slots, outlook, destroy/create legs)
packages/engine/src/ledger/instruction.ts, settlement.ts (4.1: destroy and create legs)
packages/engine/src/registry/kinds.ts                (4.1: profile.revalue, destructible, producible)
packages/engine/src/mechanisms/expectations/index.ts
packages/engine/src/mechanisms/goods/{index.ts,recipes.ts,inventory.ts}
packages/engine/src/mechanisms/labour/{index.ts,register.ts,matching.ts}
packages/engine/src/mechanisms/firms/{index.ts,decide.ts,produce.ts,invoice.ts}
packages/engine/src/mechanisms/households/{index.ts,consume.ts,portfolio.ts}
packages/engine/src/seeds/foundation.ts
packages/engine/test/{state-slots,expectations,goods,labour,firms,households,resolution}.test.ts
```

---

## Steps

- [ ] 4.1 State slots on `MechanismContext`; snapshotted; single-owner check; test
- [ ] 4.1 `view.outlook()` door; `Missing` when unformed; test
- [ ] 4.1 `destroy` and `create` legs for kinds that declare `destructible`/`producible`; equity effects; the units family counts them; tests
- [ ] 4.1 `profile.revalue` for carried-at-cost kinds; the kernel refuses an upward revaluation for a non-dealer kind (Goods E2.c); test
- [ ] 4.2 `expectations` module: state, params (memory preference with a declared dispersion shape), `form` and `score` phases, surprise events, confidence read; test: an outlook lags a step change by its memory; no outlook reads period t
- [ ] 4.2 The dispersion aggregate published lagged; test: it moves after the surprises (E2)
- [ ] 4.3 `goods` kind and sub-unit registry; recipes as physical quantities; the lint refuses a currency-denominated recipe; tests
- [ ] 4.3 Inventory lots at cost; spoilage and storage fee as two different things (E4.a); lower-of-cost-and-NRV write-down through `profile.revalue`; tests
- [ ] 4.3 One market per (region, sub-unit); rationing pro rata; price in the seller's money; test: unsold output stays with the seller (C5)
- [ ] 4.3 Audit: units identity per good and region (D5); test: a spoilage leg shows in `perished`
- [ ] 4.4 Occupations registry; the employment register as rows with headcount = cell weight; hire and separation split cells; tests
- [ ] 4.4 Vacancies as bids; cells' hours as offers at their reservation; `labour.match` clears highest bids first; the print is the last matched bid; tests: an offer above the going rate fills more (D1.a); the going rate is a read (D1.c)
- [ ] 4.4 Contract stickiness: renegotiation at the firm's horizon past its cost; severance at separation; tests: a wage does not move with the going rate inside the cost; separation costs the firm
- [ ] 4.4 Participation from the cell's own view; states employed/unemployed/inactive per cell; search every period; audit: states sum to the population; tests
- [ ] 4.5 `firms.decide`: production, vacancies, price, dividend from the firm's own view; test: an input shortage binds (B1.b); a firm never branches on industry (lint)
- [ ] 4.5 `firms.produce`: recipe consumption, work in progress at cost, yield and scrap, lead time, idle cost as period expense; tests: B5.a and B5.b
- [ ] 4.5 `firms.invoice` and `firms.pay`: receivable/payable as one object read from both sides; wages per member; dividends; failed payments recorded; tests
- [ ] 4.5 Published expectation (E7) journaled and scored; test
- [ ] 4.6 `households.decide`: consumption per member from own outlook, wealth, liquidity, confidence; allocation by preference and relative price; saving as residual; tests
- [ ] 4.6 Households as participants in goods markets and the bill market (D5.a); test: a higher bill yield draws cells into bills
- [ ] 4.6 Income and consumption taxes remitted by the payer; treasury receipts now read real bases; test: receipts are the sum of what payers paid
- [ ] 4.6 Audit: sector income as a read (B5); test
- [ ] 4.6 Households A2.g: a mean-preserving spread of income across cells changes consumption crossings while the weighted mean does not; test
- [ ] 4.7 Seed: recipes, firms, employment rows, work in progress consistent with lead times, outlooks; the three seed shapes deleted; placeholders declared with deaths
- [ ] 4.7 Resolution test harness: 1x, 2x, 4x `cellsPerKey` give the same sector aggregates to dust after a year (XI-15)
- [ ] Treasury: outlay programme gains public purchases in goods markets; public wages route through employment rows; receipts read wages and consumption bases
- [ ] Observer: inventories, employment, wages, outlooks (own scope only), the placeholder and shape counts
- [ ] Year-long run: green families named; expected red ones named in the record (none expected)
- [ ] Determinism test across the whole item with two seeds
- [ ] Coverage re-marked for every clause listed; PARTIAL rows named to items
- [ ] Record entry
- [ ] Delete this file; worklist row 4 → done
- [ ] Commit and push per sub-item (seven commits)
- [ ] Update `docs/ARCHITECTURE.md` 4.9b with the state-slot and destroy/create doors

## Exit criteria

Every listed clause MET; three seed shapes deleted; the placeholder count is exactly the four named
above with their deaths; the resolution test passes; a year green.

## Guard

Goods A2.b (recipe as value share), E2.c (inventory above cost), F1 (consumption without
production); Labour F1, F2, F3, D5 (one channel from prices to wages); Households A2.d, A2.f, B3.a;
Firm F1, F2, F3; Expectations A2.b, A4, B1.b, D1, D2, D3, D4.
