# Item 13i — Cross-border

**Objective.** Two regions that are not closed boxes: firms buy from and sell to firms in the other
region because of price, availability or cost, in an invoice currency somebody chose, with an FX
exposure the other side hedges or carries; goods move by freight across the border and settle into
an account in the invoice currency; investors hold foreign assets, borrowers issue in money they do
not earn, banks square cross-currency books, direct investment buys firms outright; a region's
current and financial accounts are reads of transactions that sum to zero because every
transaction had two sides; a deficit is financed by somebody who chose to, at a price, and the
financing can stop. Item 12 built the currencies, spot FX, and cross-region holdings; 13b the
forwards; 13c the freight; 13g the control market. This item connects them and adds the reads.

**Read first.** §43 Cross-Border (all; A was met at 12); §6 Currency E; §12 Spot FX F; §21
Commodities E3; §19 D1–D3; §31 F; XI-12; Goods D (delivered prices); Part XII (exports against
imports party by party; a region's accounts). Code: item 12's `spot-fx` and FX revaluation, 13b's
`fx-derivatives`, 13c's `freight` and routes, 13g's tender market, 13f's bonds and syndication.

**Clauses this item meets.** Cross-Border B1, B2, B3, B3.a, B4, C1–C5, C2.a, D1–D6, D3.a, D4.a,
E1–E4, F1–F3; Commodities Spot E3; FX Forwards D1–D3 (fully, the hedgers' reasons now real);
Currency E1–E4 (fully); Spot FX F1–F1.b (fully, the goods leg).

---

## Design

### Module `cross-border`

- `requires: ['goods', 'freight', 'spot-fx', 'fx-derivatives', 'corporate-credit', 'corporate-control',
'treasury', 'central-bank']`.
- **Trade in goods** (A1, A2, B1, B3): a firm's sourcing decision (item 4's `firms.decide`, 13c's
  substitution) sees every location's print including the other region's, converted at the spot
  print in force into its own money plus freight on the route (Goods D; 13c's delivered price), and
  buys where its delivered cost is lowest for its recipe (B1: a decision; B3.a: a rate move changes
  the comparison and the decision follows: expenditure switching as a consequence; no elasticity).
  The invoice currency is the seller's money by item 12's convention (Spot FX F1); the buyer short
  of it posts an FX order first (F1.a); whoever is not in the invoice currency has the exposure
  (A2.a) and its `decide` chooses to hedge with 13b's forward (its participant reads its own
  invoice book: FX Forwards D1) or carry it (its own outlook of the rate vs the forward's carry).
- **Goods move** (B2): 13c's cross-region routes with their carriers, transit and capacity; a border
  is a route.
- **Settlement** (A3): the payment on terms (13e) lands in the seller's account in the invoice
  currency at the seller's bank; the buyer's bank's cross-currency position is the residue (C3).
- **B4** contribution: exports of region A = imports of region B, unit for unit and party to party,
  read from the invoice rows of the period (the flows family: one row, two regions).
- **Finance** (C): foreign holdings from item 12 (C1); **foreign-currency issuance** (C2): a firm's
  or a sovereign's issuance decision (13f, item 3) compares its quotes across currencies from the
  curves and the forward carry and may issue in the other money when cheaper or deeper; it then owes
  a money it does not earn: its coverage read (13f A3) converts the service at the spot print each
  period and a rate move can push it below one and into item 5's event (C2.a: a solvency event, not
  a translation); banks square books with FX swaps (C3: 13b D3); **direct investment** (C4): an
  acquisition across regions through 13g's tender by an acquirer in the other region (the
  consideration crosses the border); **income flows** (C5): coupons, dividends and interest paid by
  the kernel's corporate actions to foreign holders in the instrument's currency (item 12 already;
  E3: a default reaches foreign holders in proportion through the same waterfall).
- **The balance** (D): per region, the **current account** is a read: goods and services (invoice
  rows crossing the border, by direction) plus income flows crossing (coupons, dividends, interest,
  wages if any), computed from the period's settled instructions (D1: never stored); the **financial
  account** is the read of net acquisition of foreign claims from the same instructions (D2); D3:
  they sum to zero per region because every instruction had two sides: an audit contribution to the
  `flows` family per period per region (D3.a: a residual is a lost leg and names the instruction);
  D4: the financing of a deficit is whoever bought the deficit region's claims at the price they
  cleared at (a read: the buyers by name; it can stop when nobody bids: `noDemand` in those markets);
  D4.a: a one-way flow financed by the banks shows as their cross-currency positions (a read per
  bank); D5: the stock of foreign claims per party revalues by item 12's revaluation; D6: the world
  sums to zero in every category: the same contribution summed across regions.
- **What it forces** (E): E1: every market is open to foreign participants with foreign money (item
  12's FX order before the goods market makes it so; a test posts a foreign bid in every market
  kind); E2: the register holds foreign issuers' instruments (item 12); E3 above; E4: a central
  bank's rate reaches the other region through the corridor → the money market → the forward carry →
  the spot market (13b's CIP participants) and through the currency into delivered prices (a chain
  scenario at 16).
- **Commodities E3**: a producing region's terms of trade (its export prints over its import prints,
  a read) move with the commodity price; its currency's fundamentals are its cross-border flows (a
  read); the spot market's participants act on those flows (item 12's participants already have the
  reasons: an exporter selling foreign money it earned).

### Parameters

None new. No `elasticity`, no `tradeFlow.*`, no `capitalFlow.*`, no `currentAccount` store exist.

### Audit contributions

- `flows`: B4 (exports = imports party to party); D3 (current + financial = 0 per region); D6 (the
  world sums to zero).
- `names`: every cross-border invoice names two parties in two regions (F2).

### Files

```
packages/engine/src/mechanisms/cross-border/{index.ts,sourcing.ts,exposure.ts,issuance.ts,balance.ts}
packages/engine/src/mechanisms/firms/… (sourcing across regions, issuance currency choice)
packages/engine/src/audit/families/flows.ts (region contributions)
packages/engine/test/{cross-sourcing,invoice-currency,hedge-or-carry,exports-imports,foreign-issuance,cross-books,direct-investment,current-account,financing-stops,foreign-participants}.test.ts
```

---

## Steps

- [ ] Cross-region sourcing at delivered prices in own money at the spot print plus freight as a decision; invoice currency by the seller's-money convention; the exposed side hedges with a forward or carries from its own outlook; expenditure switching as a consequence in a scenario; tests (A1, A2, A2.a, B1, B3, B3.a, FX Forwards D1)
- [ ] Goods move by freight across a border route; the payment lands in the invoice currency at the seller's bank; the buyer's bank's position is the residue; tests (A3, B2, C3)
- [ ] B4 contribution: exports equal imports unit for unit and party to party from invoice rows; tests
- [ ] Foreign-currency issuance as a decision across curves and carry; the coverage read converts service at spot and a rate move can trigger item 5's event; direct investment through a cross-region tender; tests (C1, C2, C2.a, C4)
- [ ] Income flows to foreign holders in the instrument's currency; a default reaches foreign holders in proportion; tests (C5, E2, E3)
- [ ] Current and financial accounts as reads of the period's settled instructions; D3 contribution per region naming any lost leg; tests (D1–D3, D3.a)
- [ ] Financing as the named buyers of a region's claims at cleared prices, which can stop; bank cross-currency positions as the trace of a one-way flow; stocks revaluing; the world summing to zero; tests (D4, D4.a, D5, D6)
- [ ] Every market open to foreign money (a test posts a foreign bid in each market kind); the central bank's reach through the corridor, the carry and the currency as a scenario; terms of trade as a read; tests (E1, E4, F1–F3, Commodities E3)
- [ ] Observer: balances per region from derivations, positions by party, terms of trade; year-long run green with a terms-of-trade shock; determinism; coverage re-marked; record entry
- [ ] Delete this file; worklist row 13i → done; commit and push

## Exit criteria

A firm switches supplier when the rate moves because its delivered cost changed; a region's current
account is a read that sums with its financial account to zero without a plug; a deficit region's
financing can stop; a foreign-currency borrower can fail on a rate move.

## Guard

Cross-Border F1–F3, D3.a; Currency E4; Spot FX E1–E4 (no conversion without a counterparty, no
formula rate).
