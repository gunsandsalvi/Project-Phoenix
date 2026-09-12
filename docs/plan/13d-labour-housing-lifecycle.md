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

## Findings this item carries (folded in from the review)

Three subjects, and the first two are the same defect read twice: a party doing work it could not do.

### A household runs a securities analyst's models (SHAPE)

In the same period, for the same cell, `mechanisms/households/` uses two different epistemologies:

- **Bread** (`consume.ts:197`): a demand curve centred on `view.outlook('price.<bread>')`, its own
  adaptive expectation formed from what it actually paid, spread over its own confidence. It knows
  nothing about the bakery's cost base. This is exactly right and it is what a household is.
- **A government bond** (`portfolio.ts:127`): `priceAt(flows, required, on, family.dayCount)` — the
  instrument's own cash flows discounted at the cell's own required yield, on the curve family's
  declared day count.
- **A share** (`portfolio.ts:152`): `bookPerShare + earnedPerShare ÷ year ÷ (required + uncertainty)`
  — the issuer's published accounts, per share, plus capitalised earnings at a risk-adjusted rate.

`portfolio.ts` never calls `view.outlook('price.<line>')` at all; the only outlook it reads is
`income`. The engine differentiates parties by what they may SEE, what they HOLD, what they OWE and
what they PREFER, and in no way at all by what they are able to WORK OUT — one analytical technology
distributed to everybody, which is the representative agent one level up.

Three consequences, and the third is load-bearing:

1. **XI-16's disagreement is doing half its work.** §46 A3 says the disagreement is what gives a
   market two sides. Today parties disagree for one reason only — they observed different things, at
   speeds drawn apart. They never disagree because they THINK differently, and adaptive learning from
   a common set of prints converges. A household that extrapolates and a desk that discounts cash
   flows disagree structurally and permanently, and hardest right after a large move.
2. **There is no unsophisticated money.** `dealing-quote.ts`'s `adverseOf` charges for one-sided flow
   on the reasoning that the desk "was facing somebody who knew more"; in a world where every
   counterparty knows as much as the desk, that term measures noise and market-making has no
   customers.
3. **XI-2 has no retail door.** A household never sells because a price fell: `shareOrders` sells
   only when `short > 0` — it needs cash — and its opinion comes from published accounts, which do
   not move when the print does. It redeems from a money fund only "when its own cushion is short",
   which is an INCOME shock. So a pure market shock produces no redemption wave, the fund's
   forced-seller door (Fund Shares C2.b) never opens on it, and the contagion channel the model was
   built to measure is missing its most common real-world trigger.

**And the machinery already exists.** An outlook is adaptive by construction (Expectations B1) —
extrapolation is what an adaptive outlook IS — and `ownUncertainty` is already the cell's read of how
wrong it has been. Pricing a share and a bond the way it already prices bread deletes both valuation
branches (Law 12: the fix removes code) and takes `households`'s dependency on `curveFamilyOf` and
`priceAt` with it. **Selling into a fall then falls out with no new number**: an outlook surprised
downward expects less and trusts itself less, so the bid falls AND the cushion
(`bufferPeriods × (expected + confidence)`, already built) rises, so the cell wants cash. That
matters, because Appendix B forbids a sentiment coefficient and a "fear parameter" would be one.

`funds/tracker.ts` is the model for what a naive participant looks like: "A TRACKER DOES NOT PRICE...
posts a size and no level... which is exactly what makes it a transmission channel rather than an
investor."

### Consumption is a fixed share of MONEY spending (SHAPE)

`ConsumptionDecl.share` is "the share of what it spends that goes on this good", and `data.ts` states
the consequence: "a household facing a dearer loaf buys fewer of them and spends the same on bread".
That is unit-elastic demand — spending on a good never responds to its price and a price change never
moves spending BETWEEN goods — and it is precisely what `phoenix/no-value-recipe` refuses on the
production side, with the argument written out in the rule: "A price that doubles would halve the
physical draw, which is the strongest substitution assumption there is, sitting where the model chose
none. State the quantity in the input's own units."

**It is inert today** (one final good, both cohorts at `share: 1`) and becomes the strongest
substitution assumption in the model the day a second final good exists. A cohort's preference over
goods is a real PREFERENCE and may be declared; what it may not be is a share of money. Declared as a
quantity a household wants per period, what it actually buys is the outcome of that want meeting a
price — which is what the demand schedule is already shaped to express. **13c must not add a second
final good before this lands**; that file carries the note.

### A bank makes everything out of nothing (DEFECT)

`ctx.post(venue, ...)` into a labour venue is called from exactly two places in the engine —
`firms/index.ts:302` and `treasury/index.ts:756` — and `OCCUPATIONS` is five rows with no financial
one. No bank, fund, fund manager, assessor, clearing house or central bank employs anybody. Firms
need workers and hours to produce; banks get their output free.

**The template already exists, one desk over.** `research/index.ts:105` `pay()` is a real instruction
in the bank's own money to the household cells whose members do the work, split per member as a wage
is (XI-15); the hours are TECHNOLOGY (`research.hoursPerName` — "covering a name takes a person a
stated amount of time, which is a fact about the work") and what the time costs is read off the wage
this world's own labour market printed, "never a research budget somebody wrote down, which would be
the cost stated instead of paid"; and an audit contribution checks it settled, because "a research
cost with no payee is a one-sided flow even when nothing failed". What it is missing is the
RELATIONSHIP: no headcount, no venue, no matching, no severance, and no capacity —
`hoursPerName × names` with no check that enough people exist. **Research pays the wage bill without
the employment.**

Everything else a bank does has neither. `loan.operatingCost` is the sharp end and its declaration is
corrected at 13b.1; the MECHANISM it stands in for is built here.

**A dealing desk's capacity is entirely financial.** `dealing-quote.ts` sizes a quote by its own
position limit, the room in its whole book, and `cash ÷ linesQuoted` — three balance-sheet
constraints — and `linesQuoted` is simply how many lines the desk chose. So a desk can make a market
in every line in the world at once, for free. With headcount, coverage is `hoursEmployed ÷
hoursPerLine`, and what follows is a mechanism the model cannot presently express at all: dealing
earns → the desk hires → it covers more lines → spreads narrow; dealing loses → it fires → coverage
falls → the lines it dropped lose their last participant with a view. That is the procyclicality of
market liquidity, and the observable already exists — `market.noView` is journalled for a book with
orders and nobody in it who has a view (XI-13). Today it can only fire from what desks HOLD; it
should fire from what they CUT.

Two more consequences. **Financial-sector wages never reach households**: a bank's profit goes to
capital and stops, so an entire sector's compensation — the most cyclical one — is missing from the
income side. **A bank cannot be squeezed on earnings**: XI-3 gives it cash and solvency, both
financial, and today every cost it has is variable and proportional to what it does, so a bank with
no business has no costs and survives by shrinking to nothing. A payroll due every period, with
severance owed to cut it (`LABOUR_NUMBERS.severancePeriods` is already 4), is the operating leverage
that turns a revenue fall into a capital fall — which is how banks get into trouble before anything
defaults.


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

### Module `households` (the reasons a saver actually has)

- **It prices a financial claim the way it already prices bread** (C1, §46 B1, B3): the cell's own
  `outlook('price.<line>')`, spread over that outlook's own confidence, through the same
  `rungsOver`/`levelsBelow` schedule its consumption demand uses. `savingLines`'s two valuation
  branches — the curve discount and the published-accounts read — are **deleted**, and with them
  `households`'s import of `curveFamilyOf`/`priceAt`. A line the cell has never traded and never seen
  print has no outlook and gets no bid, which is the honest answer (B6) and the same one `cds`
  already gives for a name nobody has priced.
- **Selling into a fall is derived, not declared** (XI-2's third door): an outlook that has just been
  surprised downward expects less and trusts itself less, so the bid falls and the cushion rises in
  the same read. No coefficient is added anywhere. The fund redemption (`fundOrders`) then fires on a
  MARKET shock as well as an income shock, which is what makes Fund Shares C2.b's door reachable from
  a price.
- **Holding the market rather than picking names** (D5, Fund Shares A4): preferring a tracker to
  single lines is a real PREFERENCE — what choosing costs the chooser — and it makes retail flow
  undifferentiated across names, so a retail wave moves everything together. The tracker exists and
  already refuses to price.
- **Consumption as a quantity, not a share of money** (C3, Goods A2.b's argument read for demand):
  `ConsumptionDecl` states what a cohort wants per period in the good's own unit; what it buys is
  that want meeting a price.
- **Wealth as a key dimension** (A2, A2.b, XI-15): households differ by wealth or they cannot differ
  in who delegates, who holds paper and who holds only a deposit. `cellKey` is
  `['region', 'cohort', 'bank']` and is read by nothing — **blocked on 13b.1**, which either makes it
  data or declares the key fixed. The cohort machinery here ("cohorts by exact split") is what carries
  it once the kernel allows a fourth dimension.

### Module `banks` and module `labour` (a bank employs people)

- **Financial occupations are data** (Law 15), one row each in `labour/data.ts` beside `field` and
  `works`: a trading desk, a credit desk, an advisory desk — each with its own skill, so a bank
  competing with a machine-builder for engineering hours, or with nobody for finance hours, falls out
  of the matching that already exists.
- **A bank posts openings like a firm**, through `venueParticipants` — the door exists and the banks
  module already uses it for the money-market session, so this is one more declaration and not a new
  mechanism. Hiring lag, severance and the separation path are the ones `labour` already has.
- **Each desk's recipe is hours per unit of what it does, never a share of money**: hours per loan per
  period (Banks Lending C1.d), hours per line quoted (Dealer Desks D1), and — declared here, consumed
  by the items that build those desks — hours per mandate (13g) and hours per deal (13f). Each is
  TECHNOLOGY in the sense `goods/data.ts` means it.
- **`loan.operatingCost` dies here** and the rate a bank quotes carries its real wage bill per loan
  instead (Law 12: the fix removes the parameter). 13b.1 has already re-declared it as a placeholder
  naming this item, so the count of placeholders falls by one when this lands.
- **A desk's coverage becomes `hoursEmployed ÷ hoursPerLine`**, which is what makes `linesQuoted` a
  constraint rather than a choice, and what makes `market.noView` reachable from a firing.
- **`research`'s payment becomes employment** (Reporting D2): the same wage bill, through the venue,
  with a headcount behind it — so the desk has a capacity and the analysts are employed rather than
  paid.

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

- [x] Mobility: unmatched seekers to other occupations' unfilled vacancies through the same matching function, capped both sides; movers split and enter at the entry wage with a retraining cost; tests: a flow of people, no coefficient (A3.b, XI-10)
- [x] Participation as a decision with the wage and the outside option in it; entry and exit from the workforce as split events; tests (B1, B3)
- [x] Release on death and merger through `labour.separate` for the whole headcount; severance ranks in the estate; tests (C4, Firm Birth D4.a)
- [ ] Labour E4: a cell that lost its earner misses its mortgage service through item 5's event; test
- [x] `dwelling` kind: indivisible, per location, owner in the register; maintenance and depreciation; builders produce dwellings with a lag and sell at completion; no dwelling from nowhere; tests (A1–A5, E1)
- [x] Occupancy: owner-occupiers, leases with rent by instruction, landlords with dwellings behind them; rental market per location; yield as a read; tests (A2, A3, B5)
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
- [ ] A household prices a financial claim off its own `outlook('price.<line>')` spread by its own confidence, through the same schedule its consumption uses; `savingLines`'s curve and accounts branches deleted; tests: a cell that has never seen a line print posts nothing for it; two cells with different memories bid different levels (C1, §46 B1, B3, B6)
- [ ] The retail door into XI-2: a cell surprised downward bids lower AND wants more cash in one read, so a price shock produces redemptions and the fund's forced sale fires on a market shock and not only an income shock; no coefficient added; tests (XI-2, Fund Shares C2.b)
- [ ] Preferring the market to the names: a cell's saving goes to a tracker rather than single lines at its own declared preference; retail flow is undifferentiated across names; tests (D5, Fund Shares A4, Indices C2)
- [x] `ConsumptionDecl` states a quantity per period in the good's own unit, not a share of money; what is bought is the want meeting a price; tests: a dearer loaf is fewer loaves and less spent on bread (C3)
- [ ] Wealth as a cell key dimension, once 13b.1 has made the key data; cohorts and wealth split by the same exact-split machinery; tests (A2, A2.b, XI-15)
- [ ] Financial occupations in `labour/data.ts`; a bank posts openings through `venueParticipants` with the hiring lag and the severance the venue already has; tests: a bank competes for hours and can fail to fill a vacancy (Labour A1–A4)
- [ ] Each bank desk's recipe is hours per unit of what it does — per loan per period, per line quoted — declared TECHNOLOGY; `loan.operatingCost` deleted and the quote carries the real wage bill; tests: the rate a bank quotes moves when the wage it pays moves, and the borrower's payment has a named payee behind it (Banks Lending C1.d, Law 5)
- [ ] A dealing desk's coverage is `hoursEmployed ÷ hoursPerLine`; `research`'s payment becomes employment with a headcount; tests: a desk that fires drops lines, and a dropped line's book journals `market.noView` (Dealer Desks D1, D4, XI-13, Reporting D2)
- [ ] Coverage re-marked; PARTIAL rows for F3 (the claim) and C6 (pooling) named; record entry; delete this file; worklist row 13d → done; commit and push

## Exit criteria

A worker moves occupation because its own search failed; a dwelling changes hands only in the
register; a falling market shows volume collapsing before price; a foreclosure adds supply; the
lending standard moves because the lender's own book moved; a cell ages by an exact split and its
wealth goes to a named heir.

## Guard

Labour F1–F3, D5; Housing E1–E3, C5.a (no constant standard); Households A2.d (no two populations
in one state); Firm Birth E3 (no constant population) applied to households.
