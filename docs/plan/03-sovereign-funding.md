# Item 3 — The sovereign's funding constraint

**Objective.** A treasury that must raise money before it spends it, from an auction that can fail,
into a secondary market whose curve is read from prints with provenance; a central bank that buys in
the market at a size it chooses and remits its income; no overdraft for the treasury anywhere.
This is the benchmark everything else prices over, and it must be issued by a borrower that can fail
to fund itself (XI-9).

**Read first.** Spec §30 Treasury (all), §8 Sovereign A, C, D, E, F, H, §31 Central Bank A, C, E,
XI-9, Bond N7, N9, N9.b, Money D3, Clearing A–F. Code: `world/module.ts`, `world/context.ts`,
`clearing/market.ts`, `clearing/solver.ts`, `ledger/settlement.ts`, `mechanisms/sovereign-instruments`.

**Clauses this item meets.** Treasury A1, A1.a, A2, A3, A3.a, B1, B2, B4, C1, C1.a, C3, D1, D2,
D2.a, D3, D3.a, D4, D4.a, D4.b, D5, D5.a, D6, E1, E2, E3, E4; Sovereign A1, A1.a, A1.b, A1.c, A2,
A2.a, A2.b, A2.c, A3, A3.a, A3.b, C1, C1.a, C1.b, C2, C3, C3.a, C3.b, C4, C5, C6, C7, D1, D2, D3,
D3.a, D3.b, D3.c, D4, D5, D6, E1, E1.a, E2 (a–e; E2.f at item 4), E3, E4, E5, F1, F2, F3, F4, F5,
H1, H2, H3, H3.a, H4, H5; Central Bank A1, A1.a, A2, A2.a, A2.b, A2.c, A3, A4, C1, C1.a, C1.b, C2,
C2.a, C3, C4, E1, E2, E3, E3.a, E4, E5; Bond N7, N7.a, N9, N9.a, N9.b; XI-9.
Remain PARTIAL after this item: Treasury B3/B3.a and C2 (the cycle and unemployment arrive at 4;
policy at 14), Sovereign G (default: items 5 and 7), Sovereign I (the future: 13b).

---

## Design

### Modules

| id                  | spec                           | requires                            | owns                                                                                              |
| ------------------- | ------------------------------ | ----------------------------------- | ------------------------------------------------------------------------------------------------- |
| `treasury`          | Treasury, Sovereign A, XI-9    | `sovereign-instruments`             | the treasury's programme, outlays, receipts, buffer, debt management                              |
| `sovereign-auction` | Sovereign C                    | `sovereign-instruments`, `treasury` | the primary market: one auction per line per auction date                                         |
| `sovereign-curve`   | Sovereign D, E                 | `sovereign-instruments`             | secondary-market participants; the curve as a read with provenance                                |
| `central-bank-omo`  | Central Bank C, E, Sovereign H | `sovereign-instruments`             | open-market purchases as a price-taking quantity; reinvestment as a separate decision; remittance |

`treasury.north` and `cb.north` remain seeded by `seeds/foundation`; the modules read the parties of
kind `treasury` and `centralBank` from the registry, never by id.

### Kernel doors (sub-items 3.1–3.3, each its own commit)

**3.1 Primary market form of clearing** (Sovereign C2, C5, C7; Corporate Credit C3 later).
In `clearing/market.ts`, extend `MarketDecl` with an optional `primary` block evaluated by the same
solver:

```ts
readonly primary?: {
  readonly issuer: PartyId;           // who supplies
  readonly size: number;              // units offered this session
  readonly reservation: number;       // the issuer's walk-away price per unit (Sovereign C5)
  readonly allotment: 'uniformPrice'; // every winner pays the stop-out (Sovereign C2)
};
```

`runMarket` with `primary` builds one sell order `{ party: issuer, price: reservation, qty: size }`
and clears against the buyers' schedules. Outcomes: `cleared` with `volume ≤ size` (weak demand
resolves as a higher yield, i.e. a lower price, or a smaller size: C5); `noDemand`/`noOverlap` is a
**failed auction** and writes no print for the new line (a line with no print is `Unpriced` until it
trades, which is correct: it does not exist as a price). The market journals `auction.result` (public)
with `{ line, size, allotted, stopOut, cover: bids/size, tail: stopOut vs average bid }` (C4). Trades
settle as an **issuance** leg (from the issuer) plus cash, in one instruction each (Clearing D3).

**3.2 Accrued interest travels with the trade** (Bond N9.b, Register E1.a). In `ledger/instruction.ts`
add to `AssetLeg` an optional `accruedPerUnit: Option<number>`; settlement moves `qty × accrued` as a
second money amount from buyer to seller in the same instruction (dirty settles, clean is quoted and
printed). Where the accrued comes from: the instrument kind's profile gains
`accrued(instrument, date, calendar): number` (per unit); the kernel's market runner reads it for the
session date (the period's start date) and stamps it on every trade leg of an instrument whose profile
provides it. Equity effects: the seller's income (+accrued), the buyer's asset at basis (clean price)
plus a receivable-in-price that the next coupon repays: book it as buyer equity −accrued now and
+coupon on the date; the accounts family holds because the buyer's lot basis excludes accrued while its
cash fell by dirty. (Reason: a coupon is not a windfall to whoever holds it on the date.)

**3.3 A curve read with provenance** (Sovereign D3, D3.a, D3.b, D3.c). A kernel door on both
`ParticipantView` and `MechanismContext`: `curve(family: CurveFamilyId): CurveRead` where a curve
family is registered by a module (`sovereign-curve` registers `gov.north`) with the set of lines
that belong to it and one compounding convention. `CurveRead` is `{ points: { tenorYears, yield,
provenance: 'traded' | 'stale' | 'interpolated' | 'extrapolated' | 'none' }[] }` built at read from
prints: a point is `traded` if its line printed a trade this period, `stale` if the print is carried,
`interpolated` between two traded or stale points, `extrapolated` beyond them, `none` if no line
near. Yield is **derived** from the clean price and the line's cash flows by one convention
(annual compounding, declared once in the module: D3.c). The fit's own previous output is never an
observation. Nothing in the kernel stores the curve.

### Kinds and profiles

`sovereign-instruments` already owns `sovereign.bond` and `sovereign.bill`. This item adds to the bond
profile: `accrued(i, date, cal)` (N9.b: coupon × year fraction since the last coupon date under the
bond's day count) and to the bill profile `accrued = 0` (a bill accretes against its own print, F2).
The treasury module owns no kinds. `central-bank-omo` owns none.

### Parameters

| id                                       | kind                      | unit                                     | owner                                | why / death                                                                                                                                                         |
| ---------------------------------------- | ------------------------- | ---------------------------------------- | ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `treasury.buffer.periods`                | preference                | periods                                  | model                                | Treasury D4.b: the buffer the treasury wants, as periods of known outlays; the treasury's own patience with auction risk                                            |
| `treasury.programme.horizon`             | preference                | periods                                  | model                                | D4: how far ahead the programme looks at redemptions and outlays                                                                                                    |
| `treasury.tenorMix.short`                | policy                    | ratio                                    | model (a debt-management choice, E1) | the share of the need issued in bills vs bonds; a real choice with a real cost                                                                                      |
| `treasury.auction.calendar`              | policy                    | periods between auctions per line family | model                                | Sovereign C1: announced ahead                                                                                                                                       |
| `treasury.outlays.transfers.perMember`   | policy (standing mandate) | PHX per period                           | parliament                           | Treasury B1, B3: the standing mandate's transfer to household cells until the polity exists (item 14 replaces the standing mandate with the read of the parliament) |
| `treasury.outlays.publicWages.perMember` | policy (standing mandate) | PHX per period                           | parliament                           | public wages to the working cohort until labour exists (item 4 routes it through the employment register)                                                           |
| `treasury.tax.interestIncome`            | policy                    | ratio                                    | parliament                           | Treasury C1: the one base that exists before item 4; income and consumption taxes arrive with 4                                                                     |
| `sovereign.primaryDealers.minBidShare`   | policy                    | ratio of the offered size                | model                                | Sovereign C3: the obligation, in exchange for privileges                                                                                                            |
| `sovereign.primaryDealers`               | policy                    | list of bank ids                         | model                                | which banks carry the obligation (data, not code)                                                                                                                   |
| `centralBank.omo.targetHoldingShare`     | policy                    | ratio of outstanding                     | centralBank                          | Central Bank C1, C1.a: the size the central bank chooses, as a policy target of its holding; the purchase each period is the gap to it, as a quantity               |
| `centralBank.reinvest`                   | policy                    | 0 or 1                                   | centralBank                          | C4: reinvestment of maturities is a separate decision                                                                                                               |
| `centralBank.remittance.periodicity`     | policy                    | months                                   | model                                | E3: net income remitted on a calendar date                                                                                                                          |
| `bank.liquidityBuffer.perDeposit`        | placeholder               | ratio                                    | model                                | §11 A2.a: a bank's buffer is a preference derived from its liabilities' stickiness; until deposit classes exist (item 11) a stated ratio stands in. Death: item 11  |
| `sovereign.holders.reservationSpread`    | placeholder               | per annum                                | model                                | each holder class's reservation yield over the curve (Corporate Credit E5 analogue) until cost of funds (10) and outlooks (4) produce it. Death: item 10            |

Deleted by this item: `seed.openingPrice.gov.north.2036` (the first auction prints the line; seeded
lines open with a placeholder **per line** that dies at that line's first traded print, which is
item 3's own first period, so the parameter is deleted in this item and the seed writes opening
prints from a single `seed.openingYield` placeholder that dies here too).

### Module-private state

`treasury`: the programme (`{ period, need, plannedAuctions: { line, size, date }[] }`), journaled
as `treasury.programme` (public: Sovereign C1.a, the calendar is public). `central-bank-omo`: none.
`sovereign-curve`: none (the curve is a read). `sovereign-auction`: none (the result is a journal
event and the prints).

### Phases

| phase                     | cycle | anchor                     | does                                                                                                                                                                                                                                                                                                                                                                          |
| ------------------------- | ----- | -------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `treasury.programme`      | 0     | after `corporateActions`   | reads redemptions due within the horizon from every live sovereign instrument's `due()`, the standing mandate's outlays per period, expected receipts (from its own outlook at item 4; until then the last period's receipts), the buffer preference; computes the need (A2.a: deficit plus redemptions); sizes the next auctions (A2.c: tenor mix) and journals the calendar |
| `treasury.outlays`        | 0     | after `treasury.programme` | settles transfers and public wages to cells (per member), interest is already paid by the kernel's corporate actions; each outlay is one instruction per payee; a failed outlay (refused overdraft) is journaled `treasury.shortfall` (public, A3.a) and the programme reads it next period (D5: pay from the buffer, defer, come back at a different size)                   |
| `treasury.receipts`       | 0     | after `treasury.outlays`   | settles taxes on interest income received last period by each party (C1.a: the base is the payer's own statement: read from the ledger's coupon instructions to that party); one instruction per payer                                                                                                                                                                        |
| `centralBank.omo`         | 0     | before `markets` (cycle 1) | computes the gap between its target holding share and its holding of the benchmark family; its participant posts that quantity                                                                                                                                                                                                                                                |
| `treasury.debtManagement` | 2     | after `markets`            | buybacks and switches (F5): posts next period's orders to buy an illiquid old line when its print is stale for more than its own patience, sized to the buffer's room                                                                                                                                                                                                         |
| `centralBank.remittance`  | 4     | before `revaluation`       | on the calendar date: net income since the last remittance (coupons received minus interest paid on facilities minus operating expense, read from its own ledger effects, **not revaluation**: E3.a) settled to the treasury's account; a loss is not remitted and reduces equity (E4)                                                                                        |

### Participants

| party kind    | market                                          | reason                                                                    | orders from the view                                                                                                                                                                                                                                                                                                                                                                        |
| ------------- | ----------------------------------------------- | ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `treasury`    | its own auctions                                | the issuer                                                                | the primary block on the market declaration (size, reservation = par price at the curve's yield plus its own patience): not an order; the market builds the supply                                                                                                                                                                                                                          |
| `bank`        | auctions                                        | the obligation (C3) and the liquidity buffer (E2.a: zero risk weight, E5) | if an obligated dealer: a bid for `minBidShare × size` at a price it derives from its reservation yield (curve yield + its reservation spread placeholder); otherwise the gap between its buffer preference and its holding                                                                                                                                                                 |
| `bank`        | secondary                                       | rebalancing to the buffer                                                 | sells the excess above the buffer, buys the shortfall, at its reservation                                                                                                                                                                                                                                                                                                                   |
| `centralBank` | auctions and secondary                          | policy (H1, C3: a price-taker)                                            | a buy order for the gap quantity at a price far above the market? No: a price-taker posts a quantity at the top of the posted range: the kernel gains a `marketOrder: true` flag on an order meaning "at the clearing price whatever it is", implemented as a bid at the highest posted ask (a posted price, not a bracket: C4.c). Sub-item 3.1 adds it. Never in the primary market (C1.b) |
| `household`   | secondary (bills, item 4 adds the substitution) | none yet                                                                  | none until item 4 (E2.f)                                                                                                                                                                                                                                                                                                                                                                    |

### Instructions drafted

- Transfers: money leg from the treasury's account (at the central bank) to each cell's account
  (per member), cause `transfer`, reason `standing mandate transfers`.
- Taxes: money leg from each payer to the treasury, cause `transfer`, reason `tax on interest income`.
- Auction allotments: built by the kernel's market runner: an asset leg from the treasury (issuance)
  to the buyer at the stop-out price plus a money leg buyer → treasury.
- Buybacks: the kernel's market runner with the treasury as a buyer whose credit is a redemption leg
  (to the issuer), which settlement already handles.
- Remittance: money leg central bank (own account) → treasury account at the central bank (a
  redemption of reserves into the treasury's balance: settlement handles `to === issuer`? No: the
  treasury's account is a holding of the central bank's money; the central bank pays from its own
  account (`holder === issuer`, creation) to the treasury: reserves created, equity −; correct).

### Journal events

`treasury.programme` (public), `treasury.shortfall` (public), `auction.result` (public, kernel),
`treasury.buyback` (public), `centralBank.remittance` (public), `centralBank.loss` (public).

### Audit contributions

- `flows`, contributor `treasury`: Treasury D6: Σ issued of sovereign instruments this period equals
  Σ allotted at auctions minus Σ redeemed minus Σ bought back, from the ledger, to dust.
- `prices`, contributor `sovereign-curve`: every curve point marked `traded` corresponds to a print
  with `traded` provenance this period (a read against itself is not allowed, so the check is that
  the provenance labels are consistent with the price store: D3.b).

### Seed changes (`seeds/foundation`)

Replace the single line with a maturity profile (Seed C3.a): bills at 3, 6, 12 months and bonds at
2, 5, 10 years outstanding, each opened with a print derived from one `seed.openingYield` placeholder
(par-priced at that yield) that dies at the first traded print in item 3's first periods; sizes
spread so no two lines redeem in the same period. The treasury's buffer opens at its preference.
Primary dealers: `bank.a` and `bank.b`.

### Files

```
packages/engine/src/clearing/market.ts            (3.1: primary block, market orders)
packages/engine/src/clearing/solver.ts            (3.1: market-order bids at the top posted ask)
packages/engine/src/ledger/instruction.ts         (3.2: accruedPerUnit on AssetLeg)
packages/engine/src/ledger/settlement.ts          (3.2: the accrued money leg and its equity effect)
packages/engine/src/registry/kinds.ts             (3.2: profile.accrued; 3.3: CurveFamily registration)
packages/engine/src/world/context.ts              (3.3: curve() on both contexts; marketOrder)
packages/engine/src/world/world.ts                (3.3: curve read)
packages/engine/src/mechanisms/sovereign-instruments/index.ts  (accrued; curve family)
packages/engine/src/mechanisms/treasury/index.ts
packages/engine/src/mechanisms/sovereign-auction/index.ts
packages/engine/src/mechanisms/sovereign-curve/index.ts
packages/engine/src/mechanisms/central-bank-omo/index.ts
packages/engine/src/seeds/foundation.ts           (maturity profile; dealers; buffer)
packages/engine/test/{market-primary,accrued,curve,treasury,auction,omo}.test.ts
```

---

## Steps

- [x] 3.1 Kernel: primary block on `MarketDecl`; `runMarket` builds the issuer's supply from it; `auction.result` journaled; market-order bids; tests: a primary market clears at the stop-out, fails on `noOverlap` and writes no print
- [ ] 3.1 Kernel: `MarketResult` carries `cover` and `tail`; the inspector shows them
- [x] 3.2 Kernel: `accruedPerUnit` on `AssetLeg`; settlement moves it and books the equity effects; the accounts family holds across a coupon date with a mid-period trade; test
- [x] 3.2 Profiles: `accrued()` on the bond profile by day count; zero on the bill; tests
- [ ] 3.3 Kernel: curve families registered by modules; `curve()` on both contexts; provenance per point; one compounding convention; tests: traded, stale, interpolated, extrapolated, none
- [ ] `treasury` module skeleton: params declared (table above); phases anchored; module-private programme state journaled
- [ ] `treasury.programme`: the need from due actions and outlays; the tenor mix; the auction calendar; test: redemptions ahead raise the need in the periods before them
- [ ] `treasury.outlays`: transfers and public wages to cells per member; a refused overdraft journals a shortfall; test: an empty account produces a shortfall event, never a negative balance
- [ ] `treasury.receipts`: tax on interest income read from the payer's own coupon receipts; test: receipts equal rate × what was actually paid, party by party
- [ ] `sovereign-auction`: opens a primary market per planned auction; the obligated dealers' bids; test: cover ratio and tail on the journal; a dealer at its limit bids nothing and the auction can fail (C3.a)
- [ ] `sovereign-auction`: a failed auction leaves the treasury lower than planned and the programme adjusts next period (size, tenor, buffer); test
- [ ] `sovereign-curve`: registers the curve family and the bank participants' rebalancing orders; test: a bank above its buffer sells, below buys
- [ ] `sovereign-curve`: yield derived from clean price under one convention; never the other way; test: round trip through the curve does not reproduce a price (N7.b is a FORBID, so the test asserts no code path reads the curve to set a print)
- [ ] `central-bank-omo`: target holding share; the gap posted as a market order in the secondary market only; test: the central bank never appears in a primary market (C1.b)
- [ ] `central-bank-omo`: reinvestment as a separate decision; test: with reinvest off, a maturity shrinks the holding and the base
- [ ] `centralBank.remittance`: income not revaluation; a loss reduces equity and is not remitted; test: a period with an unrealised gain remits nothing of it
- [ ] `treasury.debtManagement`: buybacks and switches of illiquid old lines within the buffer's room; test: a stale old line is bid for; the old line's issued falls
- [ ] Seed: maturity profile of bills and bonds; opening prints from one `seed.openingYield` placeholder; dealers named; test: no two lines redeem in the same period
- [ ] Delete `seed.openingPrice.gov.north.2036`; delete `seed.openingYield` once every seeded line has printed a trade within the first auction cycle (or record why a line has not: an untraded line's opening print stays an opening condition and the placeholder stays with a named death)
- [ ] Audit: `flows` contribution for Treasury D6; `prices` contribution for D3.b; tests
- [ ] Observer: the inspector shows the curve with provenance colours, auction results, the treasury's programme and buffer (Observer F2: fixed income shows price and derived yield)
- [ ] Year-long run green with the auctions, coupons, transfers and taxes flowing; determinism test with two seeds
- [ ] Coverage: re-mark every clause in the list above; PARTIAL rows for B3, C2, G, I named to their items
- [ ] Record entry: what, why, found, deleted (two placeholders), forecast with killer
- [ ] Delete this file; worklist row 3 → done
- [ ] Commit and push

## Tests (names)

`market-primary.test.ts`: clears at stop-out; weak demand lowers the stop-out; no bids → failed;
market order fills at the top posted ask, never above. `accrued.test.ts`: dirty = clean + accrued;
the coupon after a mid-period trade is not a windfall; accounts family holds. `curve.test.ts`:
provenance per point; convention declared once. `treasury.test.ts`: need, programme, shortfall,
receipts party by party. `auction.test.ts`: obligation, cover, tail, failure, programme reaction.
`omo.test.ts`: price-taker, never primary, reinvestment, remittance, loss.

## Exit criteria

Every listed clause MET; two placeholders deleted (`seed.openingPrice…`, `seed.openingYield`) and
two added with named deaths (`bank.liquidityBuffer.perDeposit` → 11,
`sovereign.holders.reservationSpread` → 10), net zero, stated in the record; the placeholder count
on the surface reads 2; a year green.

## Guard (FORBIDs in play)

Treasury D3, D5.a; Sovereign A3.b; Central Bank C1.b, E2; Bond N7.b; Clearing B4, B5, C4.c;
Sovereign D3.b (the fit's own output is never an observation).
