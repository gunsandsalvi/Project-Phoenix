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

## The finding this item carries

### The dollar drifts one way because only one side of the world trades (item 12's finding **12-14**; `docs/RECORD.md`)

**Measured** at item 12's close: USD/EUR, USD/GBP and USD/JPY all go 1.000000 → 1.018545 over nine
periods, monotonically and almost identically; the three crosses never trade at all and stay at 1.

**Why.** The three countries abroad are stubs — a central bank, a treasury and one benchmark line,
with no firms, no households and no imports. So every pair has US parties on one side with a foreign
balance they have no use for and nobody on the other with a reason to want it, and a one-sided book
moves the only way it can. The crosses have no participant at all because no party's own money is in
them, which is correct for that world and is why the triangular gap stays at zero.

**It is not a defect of the currency layer.** It is this item: trade invoiced in another country's
money is what puts parties on BOTH sides of a pair, and a reason to hold a foreign balance is 13h's
portfolio decision. When both exist, a rate that drifts one way is a finding about the mechanism;
until they do, it is a book with one side in it.

**Test this item owes it.** Over a run with cross-border invoicing live, a pair has parties of both
countries posting in it, and the drift is no longer monotone — or if it still is, the reason is a
flow that is measured, not an absence.

### The countries are the one population in this world that was typed, not drawn (`12d-18`, `12d-15`, `12d-19`, `12b-3`, `12b-4`)

Five measurements, one cause, and it is the structural half of the finding above.

**Typed, not drawn.** Every other population in this world is drawn from stated spreads and is
deterministic in the seed value (`drawBanks`, `drawFirms`, `drawListed`, `drawFunds`, `drawEtfs`,
`drawAssessors` — Seed B1.a, B4). There is no `drawCountries`. `ABROAD` (`foundation.ts:154`) is a
hand-written table of three rows carrying `'Europe' / 'European Central Bank' / 'European Treasury' /
'bund'`, `'United Kingdom' / …/ 'gilt'`, `'Japan' / …/ 'jgb'`, and the home region is written the
same way. The consequence is not the names: a typed population gets exactly the attributes somebody
typed, and what was typed is a central bank, a treasury and a ten-year line — so the three rows are
**structurally identical** (one tenor, one opening yield, one `crossHoldingShare` split evenly, one
opening rate) and Seed B4's *"a sector of equals never produces a market"* is broken by construction
in the one sector where nobody noticed, because nobody thought of a country as a member of a
population. It is also why every attribute a country needs and did not get — banks, firms,
households, a labour market, a goods market — is absent rather than declared missing: the row has no
field for it, so nothing counts it.

**What it measures as.** In the rig (`fx`, 12 periods) every one of the six currency markets prints
`noDemand` in every session and the rate stands at its opening 1 for ever:

```
banks              [ 'bank.a:us', 'bank.b:us', 'bank.c:us' ]
firm regions       [ 'us' ]
household regions  [ 'us' ]
```

and the arithmetic says exactly why: a dealer's bid in `USD/EUR` is `least(room − held, cash(EUR) /
rate)` — to BID for the pair it must hold the QUOTE money — and no party in this world holds a euro.
So every desk offers and none bids, in every pair, for ever (`12d-15`). It also explains the banks
being refused at the foreign windows with no market to go to instead, and the foreign treasuries'
auctions being covered many times over and placing nothing. Over all six pairs from period 1 to 30
(`12b-3`), not one bid and not one trade: `seed.openingRate`'s own justification — "each pair's own
first session replaces it" — is false, because there is no first session.

**And one parameter cannot state a consistent triangle** (`12b-4`). `seed.openingRate` is ONE number
for all six pairs, so the only value that leaves the triangle consistent is 1: at 0.8 the
`crossMarket` family reports `USD through EUR into GBP costs 0.64 against 0.8 direct`. A world cannot
currently be opened with realistic rates at all — 150 JPY to the dollar is not expressible. The seed
also ADDS two currencies while it is at it: `centralBankAssets` sums `units × price` across foreign
reserve lines into a USD total, and the foreign treasuries' buffer is a USD value endowed as `c.ccy`
money — Money A2.b holding only because the two are the same size.

**And `region` is a declared cell key dimension with one value** (`12d-19`). Only the United States
has households, so `region` stratifies nothing and every cell in the world carries the same value for
a third of its key. A key dimension that cannot cut the ensemble costs a cross product and buys
nothing, and it hides the fact that the represented sector is one country's. The fix is not to delete
the dimension; it is populations abroad.

### Two names written twice (`12d-17`, `12d-20`)

`BANK_COUNT` is 30 and the seed's display name is `Bank ${String.fromCharCode(65 + n)}`, so banks 27
to 30 are called **`Bank [`**, **`Bank \`**, **`Bank ]`** and **`Bank ^`**. The *id* generator beside
it (`bankName`, `banks/data.ts:260`) carries to `bank.aa` correctly; the display name is a second,
worse copy of the same rule written at the other end of the codebase (Law 4), and it was right only
while the count was under 27. Law 9: a display name is what a market calls the thing, and `Bank ^` is
not a name — and the count that breaks it is a RESOLUTION nobody may be afraid to raise. Beside it,
`funds/data.ts:241` names every bank's money-fund manager `North Asset Management ${at}` in a world
called the United States (left over from a one-region world called North), and `'American Index
Managers'` and `'US Listed Equity Fund'` are typed the same way while the FIRMS in the same world are
drawn: fund managers are a population like any other (Seed B1, B1.a) and there is no `drawManagers`.
Both land here because this is the item that draws the populations the names are for.

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

### The central-bank swap line as a facility

`§31 A2.b` names *"claims on other central banks, swap-line draws as rows"* — the **row** exists, as
an asset class on the central bank's sheet. Nothing names the **facility**: who may draw, at what
price, against what, to what limit, and what the drawing central bank does with the money.
`FX Forwards B3.a` requires the cross-currency basis to widen when funding in one currency is scarce,
and `Law 6` forbids bounding it; with no facility the only thing that stops a basis widening is
private balance sheet — right as far as it goes, and missing the mechanism that actually stops it in
the world. It is also the joint `XI-3` needs most: a bank short of a money its own central bank cannot
print is the one liquidity failure the lender of last resort cannot reach, and the swap line is why it
sometimes can. It cannot precede 13b's `xccy`: a facility that prices off a basis needs the basis to
clear first.

- A standing arrangement between two **named** central banks: one lends its own money against the
  other's at the current rate for a term (a spot leg and a forward leg at the rate the arrangement
  states, both instructions on both balance sheets, in two monies, with the revaluation account
  `§31 A2.c` already requires).
- The drawing central bank **on-lends it to its own banks** at its own overnight rate plus a stated
  spread, against collateral, limited by the line — an ordinary secured row in item 11's book, with
  the same eligibility and haircut reads, in a money that is not its own.
- The consequence is that the basis meets a **ceiling that is itself a price** (`Law 6`, `XI-14`): a
  facility priced above the market does not bind, which is the point, and what the spread is set to
  is a POLICY with an owner and a register row. A draw is an event; the line's size is a term of the
  arrangement; nothing is capped anywhere.

### Greenfield direct investment, and the parent–subsidiary group

Two thirds of this exists. `Cross-Border C4` names direct investment as **buying a firm outright** —
an acquisition, which 13g covers. `Firm Birth A2` lets **any named investor** fund a birth, and
`A2.a` requires a greenfield build to **buy** its plant from a producer. What is nowhere is the
**relation**: a firm whose owner is another firm, in another region, and what follows from it.
Building abroad is the main route by which a real firm acquires a cost base in another currency, and
the only one that creates a **new competitor** in the foreign market (`Firm Birth A5`) rather than
changing the owner of an existing one; without it every cross-border corporate position is either
portfolio or acquisition, and a multinational's exposure to a currency is a translation question
rather than a real one.

- **The build**: the parent funds the subsidiary out of a named account, across the border, in a money
  somebody sold it (`Spot FX F1`); the subsidiary buys its plant from a producer (`Firm Birth A2.a`:
  no minted endowment, and the build lag is item 10's); the parent's holding is a register row in the
  subsidiary's own equity, like any other holder's.
- **Solo versus consolidated accounts** (`§48`): a company's own statements are what this world
  reports, and a group is not a reporting entity here; consolidation is therefore a **READ** over the
  group — the parent's own statements plus what it holds of its subsidiaries, walked — and never a
  stored aggregate (Appendix B). ("Consolidated" appears in this specification only for the central
  bank's holding of sovereign debt, and that is a different word.)
- **Profit repatriation** (`Cross-Border C5`): an intra-group dividend across a border, declared by
  the subsidiary and paid to the parent's account in whatever money `XI-12` says it settles in —
  priced, settled and revalued like every other cross-border income flow, never netted.
- **Support or abandon** (`XI-3` asked of a group rather than of a firm): the parent may put more
  equity in, or it may not, and a subsidiary that fails goes to `Firm Birth E2`'s destination for
  everything it held with the parent ranking last as its owner. A parent that supports has decided to;
  a parent that abandons has decided to; neither is a rule.

### Parameters

`swapLine.spread` (policy, owner: the lending central bank) and `swapLine.limit` (a term of each
arrangement, per pair, public); `seed.countries` becomes a DRAW rather than a table (Seed B1.a: the
spreads it is drawn from are declared, the rows are not typed), and `seed.openingRate` becomes a rate
**per pair** that opens a consistent triangle. No `elasticity`, no `tradeFlow.*`, no `capitalFlow.*`,
no `currentAccount` store, and no ceiling on a basis exist.

### Audit contributions

- `flows`: B4 (exports = imports party to party); D3 (current + financial = 0 per region); D6 (the
  world sums to zero).
- `names`: every cross-border invoice names two parties in two regions (F2); every subsidiary names a
  parent that exists and every display name comes from the one rule that makes the id (Law 4, Law 9).
- `money`: a swap-line draw is two legs in two monies on two central banks' sheets, and the
  revaluation account carries the difference (§31 A2.c); nothing is created on one side only.

### Files

```
packages/engine/src/mechanisms/cross-border/{index.ts,sourcing.ts,exposure.ts,issuance.ts,balance.ts,greenfield.ts,group.ts}
packages/engine/src/mechanisms/central-bank-omo/swapLine.ts (the facility, on both sheets)
packages/engine/src/seeds/foundation.ts (countries drawn; a rate per pair; names from one rule)
packages/engine/src/mechanisms/firms/… (sourcing across regions, issuance currency choice)
packages/engine/src/audit/families/flows.ts (region contributions)
packages/engine/test/{cross-sourcing,invoice-currency,hedge-or-carry,exports-imports,foreign-issuance,cross-books,direct-investment,greenfield,group-accounts,swap-line,drawn-countries,current-account,financing-stops,foreign-participants}.test.ts
```

---

## Steps

- [ ] **Countries are drawn like every other population** (`12d-18`, `12d-15`, `12d-19`): `drawCountries` from stated spreads, each with banks, firms and households of its own, so a pair has parties on both sides, `region` cuts the ensemble, and Seed B4's sector of equals is broken here too; tests: no two countries structurally identical, every pair with a participant of each side (Seed B1, B1.a, B4)
- [ ] **An opening that can state a consistent triangle** (`12b-4`, `12b-3`): a rate per pair rather than one number for six, with the crossMarket family holding at the seal and no two currencies added anywhere in the seed's own sums; tests (Money A2.b, Currency A4)
- [ ] **One writer per name** (`12d-17`, `12d-20`): the seed's display names come from the same rule its ids do and carry past twenty-six, and the fund managers are drawn like every other population; tests (Law 4, Law 9, Seed B1.a)
- [ ] Cross-region sourcing at delivered prices in own money at the spot print plus freight as a decision; invoice currency by the seller's-money convention; the exposed side hedges with a forward or carries from its own outlook; expenditure switching as a consequence in a scenario; tests (A1, A2, A2.a, B1, B3, B3.a, FX Forwards D1)
- [ ] Goods move by freight across a border route; the payment lands in the invoice currency at the seller's bank; the buyer's bank's position is the residue; tests (A3, B2, C3)
- [ ] B4 contribution: exports equal imports unit for unit and party to party from invoice rows; tests
- [ ] Foreign-currency issuance as a decision across curves and carry; the coverage read converts service at spot and a rate move can trigger item 5's event; direct investment through a cross-region tender; tests (C1, C2, C2.a, C4)
- [ ] Income flows to foreign holders in the instrument's currency; a default reaches foreign holders in proportion; tests (C5, E2, E3)
- [ ] Current and financial accounts as reads of the period's settled instructions; D3 contribution per region naming any lost leg; tests (D1–D3, D3.a)
- [ ] Financing as the named buyers of a region's claims at cleared prices, which can stop; bank cross-currency positions as the trace of a one-way flow; stocks revaluing; the world summing to zero; tests (D4, D4.a, D5, D6)
- [ ] Every market open to foreign money (a test posts a foreign bid in each market kind); the central bank's reach through the corridor, the carry and the currency as a scenario; terms of trade as a read; tests (E1, E4, F1–F3, Commodities E3)
- [ ] **Both sides of a pair** (the finding above): over a run with invoicing live, each pair has parties of both countries posting in it, and the one-way drift item 12 measured is either gone or is a flow that can be named
- [ ] The central-bank swap line as a facility: two named central banks, a spot and a forward leg in two monies on both sheets with the revaluation account, on-lent to the drawing bank's own banks at its overnight rate plus a stated spread against collateral, limited by the line; the basis meets a ceiling that is a PRICE and a facility priced above the market does not bind; tests (§31 A2.b, A2.c; FX Forwards B3.a; XI-3; Law 6)
- [ ] Greenfield direct investment: a parent funds a subsidiary across the border in a money somebody sold it, the subsidiary buys its plant from a producer with the build lag, and the parent's holding is a register row in the subsidiary's own equity — a new competitor in the foreign market rather than a change of owner; tests (Cross-Border C4, Firm Birth A2, A2.a, A5, Spot FX F1)
- [ ] The group: consolidation as a READ over the parent's own statements and what it holds, never a stored aggregate; repatriation as an intra-group dividend across a border, priced and settled in the money XI-12 names; a parent that supports or abandons, with the subsidiary's failure going to Firm Birth E2's destination and the parent ranking last; tests (§48, Cross-Border C5, XI-3, XI-12)
- [ ] Observer: balances per region from derivations, positions by party, terms of trade, swap-line draws, group reads with the solo statements beside them; year-long run green with a terms-of-trade shock; determinism; coverage re-marked; record entry
- [ ] Delete this file; worklist row 13i → done; commit and push

## Exit criteria

A firm switches supplier when the rate moves because its delivered cost changed; a region's current
account is a read that sums with its financial account to zero without a plug; a deficit region's
financing can stop; a foreign-currency borrower can fail on a rate move.

## Guard

Cross-Border F1–F3, D3.a; Currency E4; Spot FX E1–E4 (no conversion without a counterparty, no
formula rate); Law 6 (the swap line is a price, never a ceiling on the basis); Appendix B (no stored
consolidated aggregate); Seed B1.a (nothing in the opening world is typed where it could be drawn).
