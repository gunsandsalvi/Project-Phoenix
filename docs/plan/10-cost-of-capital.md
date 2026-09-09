# Item 10 — The cost of capital

**Objective.** A financial price changes a real decision. Three joints, each of which can break the
chain on its own (XI-4): a bank prices loans from its own blended cost of funds; a firm invests when
its expected return exceeds its cost of capital at the margin, now, funded from cash, debt or equity;
a desk pays rent on inventory. Item 9 built the third; item 6 built the first with a placeholder.
This item builds the capital programme (plant, vintages, depreciation, the investment decision) and
closes the second joint, and replaces the reservation-spread placeholder of item 3.

**Read first.** XI-4 in full; §33 Capital Programme (all); §32 Firm E3, E4, E4.a; §7 Corporate
Credit E5 (the reservation); §26 D3; §37 Goods A2.c, B5 (the capital charge in unit cost). Code:
item 4's `goods` and `firms`, item 6's quote, item 9's `dealers`.

**Clauses this item meets.** Capital Programme A1–A6, A4.a, A4.b, A4.c, A6.a, A6.b, B1, B1.a, B1.b,
B1.c, B1.d, B2, B2.a, B3, B4, B5, C1, C1.a, C2, C3, C4, D1, D2, D3, D4, E1, E2, E3, E4, F1, F1.a,
F1.b; Firm E3, E4, E4.a; Goods A2.c, B5 (capital charge), B1.a (capacity); Corporate Credit E5,
E5.a, E5.b, E5.c, E5.d (for holders that exist); XI-4.

---

## Design

### Module `capital-programme`

- `requires: ['goods', 'firms', 'bank-lending', 'equity']`.
- **Kinds.** `plant`: `pricing: 'carriedAtCost'`, `liabilityOfIssuer: false`, unit per **kind of
  capital** (A4: a registry of capital kinds, each a unit: `machine-hours`, `floor-area`, …, declared
  by the module as units), terms `{ capitalKind, usefulLifePeriods (technology), commissioned:
Period | null, serviceDate }`, `due`: none (depreciation is not a cash action), `revalue`: the
  depreciation charge each period as a write-down of the lot to (cost × remaining life / life)
  **and** a retirement (a destroy leg) when fully worn (A3, A6: one schedule charged in both places
  by construction: the write-down is the charge to income and the reduction of the stock). Gross,
  net, accumulated and the period's charge are reads over the vintages (A6).
- **Capital goods producers**: firms whose recipe outputs a good with `isCapital: true` (Goods A2.c,
  Capital A4.c: whether a purchase is plant is the buyer's question: a good bought by a firm whose
  recipe does not consume it and that has a life is commissioned as plant by the buyer's
  `firms.produce` after the build lag (C3): the `create` leg for plant destroys the capital good
  units bought, so plant is never born from nothing (Firm Birth A2.a)).
- **Capacity** (A2, Goods B1.a): a firm's capacity per output is the scarcest of its plant kinds
  (A4) times the recipe's output per unit of plant per period (technology); utilisation is a read
  (D4, Goods B1.d).
- **The decision** (B): in `firms.decide`, for each candidate project (a line the firm makes, or a
  line it may enter: F1): `expectedReturn` from its outlook of demand and price (B1.a) against
  `costOfCapital` = the marginal debt cost (its best current loan quote from item 6's shopping,
  **now**, not the average coupon: B1.b) weighted with its equity cost (its shareholders' required
  return read from the share market's implied yield: the firm's outlook of its own earnings over its
  share price: B1.b) at its management's own hurdle and horizon (B1.d: preferences `firm.hurdle`,
  `firm.horizon`, dispersed). It invests when return exceeds cost **and** it can fund it (B2: cash
  above buffer, a loan it was quoted, or an issuance it decides: Firm E4.a: a firm with no programme
  raises nothing); uncertainty delays (B4: the option to wait: a firm whose confidence read is low
  requires a wider margin over its hurdle: no coefficient: the margin is its confidence read
  itself); utilisation is a reason (B3).
- **The spend** (C): a purchase order in the capital good's market (C1: the producer's revenue),
  paid in cash (C2), commissioned after the lead time (C3), irreversible (C4: the good is destroyed
  into plant at commissioning).
- **The stock** (D): per firm, vintages; the aggregate is a read; a dead firm's plant is sold by the
  estate to bidders who value it for what it can produce for them (D3: a bidder's reservation is its
  own capacity gap times its expected return, from its own view).
- **Feedback** (E): investment is demand now (the producer's order book), employment (the producer
  hires), credit (the loan quote); E4 is a Part XII chain.
- **Entering a line** (F): a project for a good the firm does not make: plant of the kind the recipe
  needs; the opening stock placeholder of F1.b is deleted here (item 4 declared it).

### Corporate Credit E5 (the holder's reservation)

The holder's reservation for a bond (E5): cost of funds (E5.a: the holder's own: a bank's from item
6/11; a fund's is its investors' required return; a cell's its patience), expected loss (E5.b: the
holder's outlook of the issuer's coverage → its PD model, item 6's shape reused per holder), capital
(E5.c: risk weight × required return for a bank; a mandate for a fund; nothing for a cell: E7 states
each). Item 3's placeholder `sovereign.holders.reservationSpread` is deleted; every participant's
schedule in bond markets is built from these three terms.

### Parameters

`firm.hurdle`, `firm.horizon` (preferences, dispersed); `capitalKinds.*`, `plant.usefulLife.<kind>`,
`recipe.plantPerUnit.<output>` (technology); `plant.buildLag.<kind>` (technology). Deleted:
`labour.capacity.hoursPerFirm` (item 4), `sovereign.holders.reservationSpread` (item 3),
`seed.wip.<line>` opening stock (F1.b).

### Audit contributions

- `units`: A6.b: gross plant change = commissioned − retired − scrapped − abandoned + transfers in −
  transfers out, per firm, from the ledger; capital in transit (bought, not commissioned) the same.
- `units`: D4: output ≤ capacity per firm per output (a VERIFY measured at 16; here the read).

### Files

```
packages/engine/src/mechanisms/capital-programme/{index.ts,plant.ts,decide.ts,spend.ts,capacity.ts}
packages/engine/src/mechanisms/firms/decide.ts, produce.ts (capacity, capital charge, projects)
packages/engine/src/mechanisms/goods/index.ts (isCapital goods)
packages/engine/src/mechanisms/sovereign-curve/…, funds/…, households/… (E5 reservations)
packages/engine/src/mechanisms/estate/programme.ts (plant sales to bidders who can use it)
packages/engine/test/{plant,depreciation,investment,capacity,reservation}.test.ts
```

---

## Steps

- [ ] Capital kinds as units; `plant` kind with vintages, life, depreciation as one schedule via `revalue` (charge and stock in one write-down), retirement when worn; tests: gross, net, accumulated and the charge are reads
- [ ] Capital goods as goods a buyer commissions: `create` plant destroys the good after the build lag; tests: plant is never born from nothing
- [ ] Capacity as the scarcest plant kind; utilisation as a read; production bound by capacity (Goods B1.a); tests
- [ ] The investment decision from the firm's own view: return vs cost of capital at the margin now, hurdle and horizon preferences, confidence as the delay, funding as a constraint (B2.a: a good project with no funding does not invest); tests
- [ ] Funding choice: cash, loan quote, equity issuance into a programme (Firm E4.a); test: a firm with no programme raises nothing
- [ ] The spend: order in the capital good's market, cash, lead time, irreversibility; tests
- [ ] Unit cost gains the capital charge (Goods B5); test
- [ ] Entering a line (F1); the opening-stock placeholder deleted; test: a new line posts nothing until its plant is in service
- [ ] Estate sells plant to bidders who value it for what it can produce for them; abandonment for plant nobody can use; test
- [ ] Corporate Credit E5 reservations per holder class from own cost of funds, expected loss and capital; item 3's spread placeholder deleted; tests: a bank and a fund post different reservations for the same bond
- [ ] Households' equity opinion and bond reservation unified as the cell's own required return (patience); test
- [ ] Audit: A6.b contribution; observer: vintages, capacity, utilisation, investment, cost of capital per firm
- [ ] Year-long run green; XI-4 chain test: a higher policy rate raises quotes, lowers investment, lowers output after the lag (a deterministic scenario test asserting direction, not level; Law 11 respected because it is a mechanism test, not a measurement)
- [ ] Coverage re-marked; record entry with the three deletions
- [ ] Delete this file; worklist row 10 → done
- [ ] Commit and push

## Exit criteria

Every listed clause MET; three placeholders deleted, none added; the chain test passes; a year green.

## Guard

Capital Programme B5 (no investment rate); XI-4 (no average coupon as the cost of capital; no free
inventory); Firm Birth A2.a; Commodities Spot F1.
