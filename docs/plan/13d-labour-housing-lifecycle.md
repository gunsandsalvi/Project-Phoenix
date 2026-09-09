# Item 13d — Labour mobility, housing, the household life cycle

**Objective.** Employment's other half: people move between occupations when their own search
fails, entering at the bottom; a death or a merger releases workers through the one separation path;
participation is a decision with the wage in it. Housing: dwellings as a register of indivisible
units with owners, per location; a price that clears between reservations and what buyers can
borrow; mortgages as loan rows secured on the dwelling with a lender's standard that is a read of
its own book; foreclosure that moves a dwelling and returns it to the market; rent between named
parties. The household life cycle: cohorts as a key dimension, ageing as a dated split, formation
and dissolution as weight events with a named heir, retirement as a cohort crossing funded by a
pension claim (the claim itself at 13h).

**Read first.** §39 Labour A3.b, B1, C4, E, F; XI-10 (second half); §40 Housing (all); §41
Households E1–E4, F (all), D1 (housing as an asset); Households A2 (heterogeneity: the cell's key
dimensions); XI-15 (splits by date); Banks Lending (secured lending: item 6), §7 E8 (pledge); Part
XII chain (a rate rise reaches consumption through floating mortgages **and** prices). Code: item
4's `labour` (the employment register, the matching function, cell splits), `households`; item 6's
`bank-lending`; item 7's estate and forced sales.

**Clauses this item meets.** Labour A3.b, B1 (fully), C4, E1–E4 (fully), F1–F3 (fully); XI-10;
Housing A1, A1.a, A2, A3, A4, A5, B1, B1.a, B1.b, B1.c, B2, B2.a, B3, B4, B4.a, B5, C1–C6, C4.a,
C4.b, C5.a, C5.b, D1–D5, E1–E4; Households D1 (housing), E1, E2, E3, E4, E4.a, E5 (fully), F1,
F1.a, F1.b, F2, F2.a, F3, F3.a (the claim at 13h: PARTIAL until then), F4; Firm Birth D4, D4.a
(the labour leg).

---

## Design

### Module `labour` (extended, same module)

- **Mobility** (A3.b, XI-10): after own-occupation matching in `labour.match`, the unmatched seekers
  of each occupation are offered to the unfilled vacancies of every other occupation through the
  **same matching function**, capped on both sides by what was left; a mover's cell is **split** (A4.c)
  and the moved cell enters the new occupation at that occupation's **entry wage** (the lowest bid that
  filled: a read of the session) with a retraining cost booked as a period of lower productivity for
  the employer (technology per occupation pair: `labour.retraining.<from>.<to>`, periods). This is a
  flow of people; no coefficient reads a wage gap.
- **Participation** (B1): a cell's decision each period in `households.decide`: to seek at all, given
  the going rate for its occupation against its outside option (the benefit it receives: Treasury B1's
  transfer as a share of the going rate: `policy.replacementRate`, parliament) and its own outlook;
  entering and leaving the workforce are events (B3) that move members between the cell's states by a
  split.
- **Release on death or merger** (C4, Firm Birth D4.a): the estate's opening (item 7) and 13g's merger
  post separations for the whole headcount through `labour.separate`, the one path; severance is a
  claim in the estate (ranking with wages: XI-8's waterfall already has the class).
- **E1–E4** close: wages are the cell's income (item 4), a firm's cost (item 4), taxed (item 3/4), and
  E4: job loss changes the cell's ability to service debt: the mortgage service below reads the cell's
  wage income; a cell whose income falls below service **misses** the payment (item 5's event).

### Module `housing`

- `requires: ['households', 'bank-lending', 'estate', 'capital-programme', 'labour']`.
- **Kind** `dwelling`: `pricing: 'cleared'`, `liabilityOfIssuer: false`, countable unit `dwellings`
  (A1: indivisible: the register's lot for a dwelling is one unit), instance per **location** (A1.a:
  one market per location); terms `{ location, built: Period }`; `due`: maintenance each period (A5:
  an instruction owner → a maintenance firm at the going price, a cost line); depreciation as an
  ordinary revaluation entry through `profile.revalue` (A5).
- **The stock** (A4): a register of units with owners; built by a `builder` (a firm whose recipe
  outputs dwellings from land, materials and labour with a build lag: a plant-like build through item
  10's commissioning; a dwelling built this period exists next period), sold at completion at cost
  or above (B1.a); a dwelling never appears otherwise (E1).
- **Occupancy** (A2, A3): every dwelling has an occupier: the owner-occupier cell, or a tenant cell
  paying **rent** to the owner by instruction each period (a `lease` row in the module's state: owner,
  tenant cell, rent, term); a `landlord` is any party (a cell or a firm) holding dwellings it does not
  occupy; a rental stock has dwellings behind it. A cell with no dwelling is housed by renting: its
  `decide` phase bids in the rental market (rent clears per location like any price: B5: the yield is
  a read of rent over price).
- **The price** (B1): one market per location in `markets`: **offers** are owners whose tenure ends
  (a cell dissolving, a cell moving location for a job (mobility above), a landlord selling, an
  estate, a foreclosing lender) each at a **reservation**: the larger of what discharges its own
  mortgage and what its dwelling cost to build (B1.a: a seller's reservation is its own decision
  between two reasons, posted as a schedule; it is not a floor on the print: an offer no bid reaches
  does not clear and the print is whatever cleared); **bids** are what buyers can borrow at the keenest
  mortgage quote available to them plus their own cash (B1.b, B2: the cell's participant shops item
  6's quotes with the dwelling as collateral); units clear where a bid meets a reservation and an
  unreached offer does not clear (B1.c, B4: a seller keeps its dwelling; volume collapses before price:
  B4.a is a measurement at 16).
- **The mortgage** (C): a `loan` row (item 6) with `secured: { instrument: dwelling id, lien }`
  (C1: the register's lien); term, rate fixed or floating (on 12's benchmark), amortisation (C2:
  interest and principal by the kernel's corporate actions); LTV as a read (C3); **default** (C4):
  a missed service (item 5) → the lender's decision to foreclose (its own workout choice) → the
  dwelling moves to the lender (C4.a: an asset leg lender ← cell against the loan's write-down, the
  lien exercised) and is offered into the next session by the lender's forced-sale participant (XI-2:
  the extra supply is what makes a falling price fall further); the recovery is what it fetched (Firm
  Birth D2.a); C4.b (correlated losses) is an outcome measured at 16.
- **The standard** (C5, C5.a): the lender's maximum LTV and income multiple per applicant are reads
  of its own book: the LTV cross-section of its mortgage rows at current prints, its hurdle, its
  capital headroom (item 11): a lender whose book's LTV tail has widened quotes less to the next
  borrower; no `lender.maxLtv` constant exists (C5.a); C5.b's loop (the housing cycle) is emergent.
- **Pooling** (C6): the mortgage rows are transferable into 13e's vehicles; declared here, built
  there.
- **What it feeds** (D): D1: the cell's net worth read includes its dwelling at the print and its
  mortgage at face; consumption (item 4's decision) reads net worth from the cell's own view; D2:
  the builder's programme is investment and employment (item 10); D3: rent is a line in the consumer
  price index (12/13c); D4: the service is a fixed claim on income (E5's debt-service burden read).

### Module `households` (extended: the life cycle)

- **Cohorts** (F1, F1.a): `cohort` is a key dimension of the cell (declared in item 4's registry as
  the cell key: `(region, occupation, cohort, …)`); ageing is a **split by date** at the cohort
  boundary: the kernel's calendar places each cell's members' birth dates (per-member state) and the
  cell events split the crossing members into a new cell in the next cohort (XI-15: exact).
- **Formation and dissolution** (F1.b): entry (a new cell formed from a cohort crossing into
  adulthood: weight event `entry` with cause `formation`) and death (weight event `death` with cause
  `dissolution` at the last cohort by a mortality read per cohort: a technology primitive
  `mortality.<cohort>` declared as such; a household death is not a firm death: it has no estate;
  its wealth transfers: F2).
- **Inheritance** (F2, F2.a): the dissolving members' wealth (deposits, securities, dwellings,
  fund shares; net of debt: a mortgage passes with the dwelling) moves to a **named heir cell** (the
  cell in the next-younger cohort with the same region; a data rule of the module) as `weight_dead ×
the member's wealth`, an ordinary instruction per instrument; a member with negative net wealth
  leaves its lender a loss (item 5's write-off) and no residual on a dead party (D6.a).
- **Retirement** (F3, F3.a): the crossing into the retired cohort is a split; from then the cell's
  income is drawdown from its pension claim (13h's insurer/pension liability: until then the cell's
  own savings; F3 PARTIAL → MET at 13h) and its participation decision is absent by cohort (a read
  of the registry: the retired cohort has no occupation dimension).
- **F4** is a read: the composition is the weights per cohort.

### Parameters

`labour.retraining.<from>.<to>` (technology, periods); `policy.replacementRate` (policy, parliament;
exists from item 3/4 as the transfer rate: renamed only if not already so named); `dwelling.
buildLag`, `dwelling.usefulLife`, `dwelling.maintenancePerPeriod` (technology); `cohort.boundaries`
(data: ages), `mortality.<cohort>` (technology); `household.heirRule` (data). No `lender.maxLtv`, no
`house.price.*`, no `unemployment.*`, no `birthRate` exist.

### Audit contributions

- `units`: dwellings: opening + built = closing per location, exactly (A4); population: Σ weights per cohort equals the population; every
  represented member in exactly one cell (Part XII units; Labour B5 already).
- `ownership`/`flows`: Housing E4: mortgage debt owed by cells equals mortgage assets held by lenders
  and pools, exactly (the register holds it as one row: the check is that no second representation
  exists: a read of rows from both directions).
- `names`: every dwelling has an owner and an occupier (E1); every lease has two live parties.

### Files

```
packages/engine/src/mechanisms/labour/{mobility.ts,participation.ts,release.ts}
packages/engine/src/mechanisms/housing/{index.ts,dwelling.ts,market.ts,mortgage.ts,standard.ts,foreclosure.ts,rent.ts,builder.ts}
packages/engine/src/mechanisms/households/{lifecycle.ts,inheritance.ts,retirement.ts}
packages/engine/src/mechanisms/bank-lending/secured.ts (collateral lien on a loan row; standard as a read)
packages/engine/src/seeds/foundation.ts (dwellings, leases, mortgages, cohorts)
packages/engine/test/{mobility,participation,release,dwellings,housing-market,mortgage,lending-standard,foreclosure,rent,cohorts,inheritance,retirement}.test.ts
```

---

## Steps

- [ ] Mobility: unmatched seekers to other occupations' unfilled vacancies through the same matching function, capped both sides; movers split and enter at the entry wage with a retraining cost; tests: a flow of people, no coefficient (A3.b, XI-10)
- [ ] Participation as a decision with the wage and the outside option in it; entry and exit from the workforce as split events; tests (B1, B3)
- [ ] Release on death and merger through `labour.separate` for the whole headcount; severance ranks in the estate; tests (C4, Firm Birth D4.a)
- [ ] Labour E4: a cell that lost its earner misses its mortgage service through item 5's event; test
- [ ] `dwelling` kind: indivisible, per location, owner in the register; maintenance and depreciation; builders produce dwellings with a lag and sell at completion; no dwelling from nowhere; tests (A1–A5, E1)
- [ ] Occupancy: owner-occupiers, leases with rent by instruction, landlords with dwellings behind them; rental market per location; yield as a read; tests (A2, A3, B5)
- [ ] The housing market: offers from tenure endings at the seller's own reservation, bids from what buyers can borrow plus cash, a cross where unreached offers do not clear; tests (B1–B4, E2)
- [ ] Mortgage as a secured loan row with a lien, fixed or floating, amortising; LTV as a read; tests (C1–C3, E3)
- [ ] The lender's standard as a read of its own book's LTV cross-section, hurdle and headroom; no constant exists; tests: a widened tail tightens the next quote (C5, C5.a)
- [ ] Foreclosure: the dwelling moves to the lender and is sold as a forced sale into the next session; the recovery is what it fetched; the loss lands on the lender; tests (C4, C4.a, XI-2)
- [ ] Housing E4 and units contributions; net worth includes the dwelling; consumption reads it; rent in the consumer index; service in the debt-service burden read; tests (D1–D4, Households E5)
- [ ] Cohorts as a key dimension; ageing as an exact split by date at the boundary; tests (F1, F1.a)
- [ ] Formation and dissolution as weight events with causes; mortality as a declared technology primitive per cohort; tests (F1.b)
- [ ] Inheritance to a named heir cell as ordinary instructions per instrument, mortgage with the dwelling; negative net wealth leaves the lender a loss; no residual on a dead cell; tests (F2, F2.a)
- [ ] Retirement as a cohort crossing; income switches to drawdown; the pension claim declared PARTIAL to 13h; tests (F3, F3.a)
- [ ] Households E1–E4: borrowing for a house, consumption and shortfalls as decisions; the lender's affordability decision; default from the distribution; tests
- [ ] Observer: dwellings by location, prices, volumes, LTV distribution per lender, foreclosures, cohorts; year-long run green with a rate-rise scenario: consumption reached through floating mortgages and through prices with different lags (D5, direction only); determinism
- [ ] Coverage re-marked; PARTIAL rows for F3 (the claim) and C6 (pooling) named; record entry; delete this file; worklist row 13d → done; commit and push

## Exit criteria

A worker moves occupation because its own search failed; a dwelling changes hands only in the
register; a falling market shows volume collapsing before price; a foreclosure adds supply; the
lending standard moves because the lender's own book moved; a cell ages by an exact split and its
wealth goes to a named heir.

## Guard

Labour F1–F3, D5; Housing E1–E3, C5.a (no constant standard); Households A2.d (no two populations
in one state); Firm Birth E3 (no constant population) applied to households.
