# Item 12 — The currency layer, benchmarks, ratings, the second opinion

**Objective.** More than one currency, each a named central bank's liability; every pair clearing on
its own flow with triangular consistency an outcome bounded arbitrageurs enforce; forwards that
carry the interest differential (13b builds the derivative; here the spot market and revaluation);
one index system read from constituents; a transacted overnight benchmark; ratings as named
opinions from state that rules refer to; the second opinion guaranteed in every credit book. The
single-currency guard in settlement is deleted here, in the same change as the FX revaluation that
replaces it.

**Read first.** §6 Currency (all); §12 Spot FX (all); §22 Indices (all); §44 Ratings (all); XI-7,
XI-12, XI-13; §43 Cross-Border A (13i builds the rest); §31 F; Money A2.b, C3; Bond N3. Code:
`ledger/settlement.ts` (`checkHomeCurrency`), `world/revalue.ts`, `audit/families/accounts.ts`
(the `inst.ccy !== home` skip), item 9's dealers.

**Clauses this item meets.** Currency A1–A5, A3.a, A4, B1, B2, B2.a, B3, B4, B5, C1, C2, C3, C3.a,
C3.b, C4, C4.a, C5, D1, D1.a, D2, D2.a, D2.b, D3, D4, E1, E2, E2.a, E3, E4; Spot FX A1–A3, B1–B6,
B5.a, C1–C6, C2.a, D1–D5, E1–E4, F1, F1.a, F1.b; Indices A1, A1.a, A2, A3, A4, B1, B2, B2.a, B3, B4,
C1, C2, C2.a, C3 (settlement at 13b), C4, D1, D2, D3, D3.a, D3.b, D4, D4.a, D5, D5.a, E1, E2, E3;
Ratings A1–A5, A2.a, A5.a, B1, B2, B2.a, B3, C1, C1.a, C2, C3, C4, C5, D1–D5, E1–E4; Central Bank
F1–F4; Cross-Border A1–A4; Money C3; Bond N3; XI-7, XI-12, XI-13. Remain PARTIAL: Currency E2
(portfolio shifts fully at 13h), Indices D4 (producer/consumer price indices need 13c's freight and
the distribution margin: built here from the goods that exist, extended at 13c).

---

## Design

### Sub-item 12.1 Kernel: the FX revaluation and the guard's deletion

- `Currency D2`: revaluation gains a step before marks: for every holding whose instrument's currency
  differs from the holder's home currency, the change in the rate since last period, applied to the
  position's value in its own currency, is booked to the holder's equity (D2.a: a firm or a bank) or
  to the central bank's revaluation account (a second equity-like account per central bank, moved
  only by FX revaluation; the accounts family compares equity + revaluation account for a central
  bank: Central Bank A2.c). The rate in force for the period is one rate (D1): the spot market's
  print of the period before the revaluation phase... ordering: the spot market clears in the
  `markets` phase; the revaluation phase reads that period's print; settlement during the period's
  cycles valued at the previous period's print (D3: revaluation happens before anything uses the new
  rate: everything in the period used the old rate; the revaluation at the close brings books to the
  new one; the next period's payments value at it). So `Valuation.markPerUnit` for a foreign
  instrument converts at the rate in force = the print of period t−1 during period t's cycles and
  the print of t at revaluation and audit.
- `Money A2.b` guard: `checkHomeCurrency` is deleted; the accounts family's skip of foreign
  instruments is deleted; assets and liabilities in foreign money convert at the rate in force
  (C5: the rate used to value and to settle is the same rate).
- The reporting numéraire (C4): the observer converts for display only; nothing stores a converted
  balance (C4.a).

### Module `spot-fx`

- `requires: ['dealers', 'money-market']`.
- **The pair set** (Spot FX A3): every pair among the registry's currencies is a market (opened at
  seed), instrument `fx.pair:<base>/<quote>`? A currency pair is not a holding; the "instrument"
  traded is money itself: a spot trade is an instruction with **two money legs** in two currencies at
  a rate (A1: both legs settle). So the market runner needs a market whose "price" is a rate and
  whose trade instruction is money-for-money: sub-item 12.2, kernel door: `MarketDecl.kind: 'asset'
| 'fx'` with an `fx: { base, quote }` block; the print is the rate (C1, C2); the trade is two money
  legs; DvP holds by atomicity.
- **Participants** (B): a party that owes a currency it does not have (B1: read from its own failed
  or upcoming foreign payments: `view.upcomingPayments` in its view: item 13i's importers; here a
  bank's foreign bond coupon, a firm with a foreign-currency loan (13f)), a party with a currency it
  does not want (B2), investors (B3: funds' mandates at 13h), hedgers (B4: 13b), dealers (B5: desks
  that make the pair, quoting from their own inventory of each currency and their limit: D1–D5), the
  central bank (B6: intervention bounded by its reserves: Central Bank F2, a participant with a size
  and a limit never the residual).
- **Cross-consistency** (C2.a, C3): the pairs clear on their own flows; triangular consistency is an
  outcome: an arbitrage participant (the desks, E3: with a limit) posts across three pairs when the
  cross and the direct route disagree by more than its cost; a persistent gap is measurable (E3:
  the observer shows it). C3.b: no vehicle: the ledger never triangulates (XI-12: every conversion is
  a trade in the pair the payer chose, from its own view: the cheapest route it can see is its choice,
  not the kernel's).
- **One convention for what a payment settles in** (F): the seller's money (F1); a buyer short of it
  buys it in the pair market (F1.a: never inside the trade); F1.b: the rule is the market runner's,
  owned once (the goods market's trade instruction denominates cash in the instrument's currency;
  a buyer with no account in it posts an FX order first: its module's `decide` phase reads its
  upcoming purchases and posts the FX order in the same session before the goods market: ordering:
  FX pairs clear before goods in the `markets` phase: the module declares its markets' order
  through `MarketDecl.order` (a kernel field: markets run in declared order: sub-item 12.2)).
- **Reserves and revaluation** (Central Bank F): the central bank holds foreign money (its holding of
  another central bank's money instrument) and foreign paper; revalues into its revaluation account;
  claims on other central banks (F4) are those holdings; intervention (F2) draws them down.

### Module `indices`

- `requires: ['equity', 'sovereign-curve']`.
- An index is a stated rule over stated constituents at stated weights (A1), **read** from prints
  (A2, E2): `view.index(id): IndexRead` is a kernel door (12.3) computing level, chained across
  rebalances (B2.a), from constituents' prints and free-float weights (B1); corporate actions handled
  (B3: a split changes shares and price together). Families: an equity index per region (D1), a
  credit index over the corporate bonds that exist (D2: from 13f), the rate benchmark (D3: the
  overnight money-market print of item 11: **transacted**, D3.a; a policy rate is not a benchmark:
  D3.b; floating coupons (13f) fix on it), producer and consumer price indices (D4: two baskets from
  the goods markets' prints: producer at the factory gate weighted by production; consumer at the
  price a cell paid weighted by cell expenditure: both reads from the ledger's trades). D5: one index
  system; D5.a: the opening history is the seed's prints, not a random walk (the seed writes no
  history before period zero: an index has no history until it has printed; a beta measured against
  it is Missing until the window fills: XI-7).
- Mandates (C2): funds that track an index (item 8's mandate gains `track: IndexId`): a rebalance is
  a real forced trade by every tracker in the same session (C2.a is a VERIFY measured at 16).

### Module `ratings`

- `requires: ['credit-events', 'firms', 'treasury']`.
- **Party kind** `assessor` (named; A5: its own incentives: it is paid by issuers it rates: a fee
  instruction per rating per period, a reason to rate generously that its reputation (its own
  history of misses, a read) counters; both are its own view).
- **A rating** is module-private state per (assessor, issuer or instrument): an ordinal grade
  derived from observable state (A2: leverage, coverage, cash, size, sector, age, and their trends,
  read from the issuer's public statements: Firm E7's published expectation and the estate/default
  journal), **never from the price** (A2.a: the module's phase has no access to prints of the
  issuer's paper by construction: it is given a view that omits prints: `ctx.participant` for the
  assessor with `print` returning Missing for rated instruments: a kernel option on the view:
  `withoutPrints`). Coarse and sticky (A3: a grade moves only when the state crosses a grade boundary
  and stays across it for the assessor's own patience). Published as a public journal event
  `rating.action` (A1, C: rules refer to it: funds' mandates (C1), capital charges (C2: risk weight
  per grade, policy), collateral haircuts (C3: the lender's haircut reads the grade as one input to
  its own PD: a per-issuer measure), covenants (C4: 13f), information (C5: a participant's outlook
  may read a public rating action as one more observation: Expectations A2.a).
- Instrument ratings differ from issuer ratings (B2.a): loss given default from ranking and security.
- Not the only assessment (A5.a): every holder's own PD model (item 6/10) stays; the rating is one
  more public observation.
- D: the loop is emergent: a downgrade forces sales by mandated holders (item 8's mandate), raises
  capital charges, lowers haircuts, raises the issuer's cost of funds (its next quote), which worsens
  its state; D4 is a Part XII chain.

### Module `second-opinion` (XI-13, folded into the existing books)

Not a module: a **test and a rule**. Every credit book (bonds, loans, CDS at 13b) must have a
participant whose reason is a view (Corporate Credit A4.b, CDS B5, IRS B4): item 9's desks and 13h's
hedge funds are those participants; here a kernel-level assembly check: a market whose only
participants are mandate-driven is journaled `market.noView` (public) each period it runs, as a
standing observation (Part XII). The assembly test asserts each credit market has at least one
participant declaration of a kind whose profile declares `speculative: true` (desks, hedge funds).

### Parameters

`centralBank.reserves.target` (policy), `fx.arbitrage.cost` (technology: the desk's cost of a
three-legged trade: its own funding and capital: no parameter: it is the desk's rent; delete this
row when confirmed), `index.<id>.rule` (data), `rating.grades` (data: the ordinal scale),
`rating.boundaries.<grade>` (shape: the assessor's thresholds on coverage and leverage: declared,
justified as the assessor's methodology), `regulation.riskWeight.<grade>` (policy).

### Audit contributions

- `crossMarket` (the family is **built** here, contributor `spot-fx` and `indices`): C3 triangular
  gap per triple as a read against the desks' cost; E3 (Indices): an index and its constituents move
  together by construction: a check that the index read equals the weighted read of prints (a
  derived read against its inputs: allowed because the index is a function and the check is that the
  function is applied: it catches a stale cache, which D5 forbids).
- `money`: D4 (Currency): revaluation gains plus losses equal the rate move on the net open position
  per currency, and the net open position across all parties is what the issuer and the rest of the
  world hold.

### Files

```
packages/engine/src/world/revalue.ts, prices/value.ts, ledger/settlement.ts, audit/families/{accounts,money,crossMarket}.ts (12.1)
packages/engine/src/clearing/market.ts, solver.ts, world/world.ts (12.2: fx markets, market order)
packages/engine/src/world/context.ts (12.3: view.index, withoutPrints, upcomingPayments)
packages/engine/src/mechanisms/spot-fx/{index.ts,participants.ts,arbitrage.ts,convention.ts}
packages/engine/src/mechanisms/indices/{index.ts,equity.ts,credit.ts,benchmark.ts,prices.ts}
packages/engine/src/mechanisms/ratings/{index.ts,assess.ts,publish.ts}
packages/engine/src/seeds/foundation.ts (a second region and currency; cross holdings)
packages/engine/test/{fx-revaluation,spot-fx,triangular,convention,indices,benchmark,price-indices,ratings,second-opinion}.test.ts
```

---

## Steps

- [ ] 12.1 FX revaluation into equity and the central bank's revaluation account; one rate in force per period; valuation converts at it; tests: an unrevalued foreign position is unreachable (D2.b)
- [ ] 12.1 Delete `checkHomeCurrency` and the accounts family's foreign skip in the same commit; the accounts family holds with foreign positions; test
- [ ] 12.1 Audit: Currency D4 contribution; test
- [ ] 12.2 Kernel: `fx` markets whose trade is two money legs at a rate; market declaration order within the `markets` phase; tests
- [ ] 12.2 Seed: a second region and currency with its central bank, a bank, firms, cells, a sovereign line; cross holdings; the world still passes the seed audit
- [ ] `spot-fx`: participants with reasons (owes a currency, holds one it does not want, central bank bounded by reserves); dealers' schedules per pair from inventory of each currency; tests: no conversion without a counterparty (E1), no formula rate (E2: the lint's no-magic-numbers plus a test that the print is a posted price)
- [ ] `spot-fx`: cross-consistency as an outcome: desks arbitrage across triples with a limit; a persistent gap is measurable; tests (E3, C3.b: no vehicle)
- [ ] `spot-fx`: the seller's-money convention owned once; a buyer short of it posts an FX order before the goods market (F1, F1.a, F1.b); tests
- [ ] Central bank: reserves in foreign money, intervention bounded by them, revaluation account, claims on other central banks sum to zero; tests (Central Bank F1–F4)
- [ ] 12.3 Kernel: `view.index(id)` read; `withoutPrints` views; `upcomingPayments`; tests
- [ ] `indices`: equity index per region chained across rebalances, free-float weights, corporate actions; test: a split does not move the level (B3); a rebalance does not jump (B2.a)
- [ ] `indices`: the rate benchmark as the overnight money-market print; a policy rate is not a benchmark; test
- [ ] `indices`: producer and consumer price indices from the ledger's trades with different baskets and weights; test: they diverge when a distribution wedge exists (G1.c: partial until 13c)
- [ ] `indices`: no stored level (E2), no history before the first print (D5.a); test: a beta over a window with fewer prints than the window is Missing
- [ ] Fund mandates that track an index: rebalance as a forced trade in the same session; test
- [ ] `ratings`: assessor party kind paid by issuers; grades from state through a view without prints (A2.a structurally); coarse and sticky; published; tests: a rating never moves when only the price moves
- [ ] `ratings`: instrument vs issuer ratings; consumers: mandates, risk weights per grade, haircuts as one input to the lender's PD, information; tests: a downgrade past a mandate boundary forces sales by every bound holder in the same session (C1.a)
- [ ] `ratings`: E3: the assessor can be wrong: a rated-safe issuer defaults in a scenario test; E4: the distribution of grades is a read
- [ ] Second opinion: `speculative` party-kind flag; assembly check per credit market; `market.noView` standing observation; tests
- [ ] Audit: `crossMarket` family built with its two contributions; tests
- [ ] Observer: rates per pair, triangular gap, indices, price indices, ratings, the numéraire view for display only
- [ ] Year-long run green in two currencies with cross-border coupons flowing; determinism
- [ ] Part XII chain tests as scenarios: a downgrade causes selling, capital pressure and funding loss (D4, direction only)
- [ ] Coverage re-marked; record entry with the guard deletion
- [ ] Delete this file; worklist row 12 → done
- [ ] Commit and push per sub-item

## Exit criteria

Two currencies; every conversion a trade; revaluation exact; the single-currency guard gone; one
index system; a transacted benchmark; ratings from state; every credit market has a view.

## Guard

Currency A4, B3, C3.b, C4.a, D2.b, E3; Spot FX E1, E2, E3, F1.a, F1.b; Indices A3, D3.b, E1, E2;
Ratings A2.a, E1, E2, E3; XI-13.
