# Item 13b — The derivative classes

**Objective.** Four classes as modules on 13a's layer, each answering the derivative contract its
own way: credit default swaps with a curve, an event settled at the estate's realised recovery and
an index series; interest-rate swaps whose fixed rate clears and whose curve is a read; FX forwards,
FX swaps and cross-currency swaps that carry the interest differential through an arbitrage banks
take with balance sheet, with one basis; an equity index future that settles against the index
read, which is what a desk hedges inventory with. Every side of every book has a reason, and every
book has a participant with a view (XI-13).

**Read first.** §17 CDS, §18 IRS, §19 FX Forwards (all); Derivative contract D7.b (at par); Indices
C3; Dealer Desks E1, E2; Corporate Credit H4, H4.a; Sovereign C (the curve); XI-13; XI-12. Code:
13a's profile and market variant, 12's `spot-fx`, benchmark and `view.index`, item 7's estate
recovery, item 5's credit events.

**Clauses this item meets.** CDS A1, A1.a–A1.d, A2, A3, A4, A4.a, A5, A5.a, A5.b, B1, B1.a, B2,
B2.a, B3, B4, B5, C1, C2, C3, C3.a, C3.b, C4, D1, D2, D2.a, D2.b, D3, D4, D5, E1–E4; IRS A1,
A1.a–A1.d, A2, A3, A4, B1, B2, B2.a (the pension at 13h; declared PARTIAL until then), B3, B4, B5,
C1, C1.a, C2, C3, C3.a, C4, D1–D4, D3.a, E1–E3; FX Forwards A1, A1.a–A1.d, A2, A3, A4, B1, B2,
B2.a, B2.b, B3, B3.a, B3.b, B4, C1, C1.a, C2, C3, C4, D1, D2, D2.a, D3, D4, E1–E4; Indices C3;
Dealer Desks E1, E2; Corporate Credit H4, H4.a; Equity (hedging) from item 9's PARTIAL list.

---

## Design

### Rate-priced markets

A swap or a CDS clears on a **rate**, not a price: the print is the fixed rate (bp per annum) or the
running spread, in a declared unit (`bp.pa`), and the contract is struck at par at that print (D7.b:
the mechanism is the solver over schedules of rate against size; the valuation formula is never run
backwards). The `MarketDecl` variant `{ kind: 'contract' }` from 13a gains a `quotedAs: 'price' |
'rate'` term; the solver is unchanged (a schedule is size against a level; the level's unit is data).
A schedule for a rate-priced market is sorted the natural way for the side (a protection buyer pays
less spread for more size; a seller wants more): the module's participants post correctly and the
solver's monotonicity check throws on a malformed one.

### Module `cds`

- `requires: ['derivative-layer', 'ratings']`.
- **Kind** `cds`: terms `{ reference: PartyId, tenor, currency, notional }`; underlying = the named
  reference's credit event (A1.a, A4: the reference is a party with debt outstanding; A4.a: assembly
  refuses a reference nobody can observe failing, i.e. a party with no defaultable instrument);
  payoff par − recovery on the event (A1.b); premium leg periodic in cash, stopping on the event
  (A2); mark = the protection leg's expected value read from the cleared spread curve (A3, C2: the
  implied default probability and expected recovery are **derived** from the spread and the term
  structure; the profile's `mark` reads the curve, never a PD model).
- **Curve** (A1.d): one market per (reference, tenor) for several tenors (data: `cds.tenors`);
  the term structure of credit is the set of prints.
- **Participants**: protection buyers: banks holding the reference's loans they cannot sell (B1.a:
  the bank's participant reads its own loan rows and its large-exposure headroom), holders of the
  reference's bonds, and a **view** (B1: a party whose outlook of the reference's coverage is worse
  than the spread implies); sellers: funds and insurers wanting credit exposure without funding a
  bond (B2), and a view (B2: spread too wide for the risk); desks (B4). B5: the assembly check from
  item 12 requires a `speculative` participant on both sides of every CDS book: desks' own outlook
  qualifies here; 13h's hedge funds join. Naked sellers are capitalised as an unfunded exposure
  (B3: risk weight per 11's capital: `regulation.riskWeight.cds.sold`, policy).
- **Reservations** carry the counterparty term (13a D10.a; E2: a seller whose own PD, as read by
  the buyer, correlates with the reference: the buyer's view of the seller's exposure to the
  reference through `exposureTo` and public holdings: wrong-way risk is a cost in the reservation).
- **The event** (D): item 5's credit event for the reference (D1: the stated definition); the
  contract then pays no premium, marks at its expected payoff, and **holds past its maturity** until
  item 7's estate closes (D2.b); at close the settlement is par − the realised recovery per unit of
  the defaulted obligations (D2: read from the estate's distribution: no fixed recovery: D2.a); the
  payment is cash seller → buyer (D3: it can fail the seller: the same Failed state); the contract
  terminates (D4).
- **The index** (A5): a `cds.index` kind on a series of names fixed at the roll (data per series);
  a name's event settles its weight once for every contract on the line; the line runs on with
  survivors; its own book; the index-vs-single-name basis is a read (A5.b).
- **Reads**: basis vs the cash bond's spread over the sovereign curve at every tenor both books print
  (C3, C3.a: a read on the observer; C3.b: a standing observation); net notional per reference (E3).

### Module `irs`

- `requires: ['derivative-layer', 'indices']`.
- **Kind** `irs`: terms `{ currency, tenor, notional, fixedRate (the strike), fixedPeriodicity,
floatPeriodicity, accrual conventions }` (A2: legs need not match); underlying = the overnight
  benchmark print (12's transacted rate: A1.a, E3); the floating leg fixes on its date as the
  **compounded** overnight prints over the accrual period (A3: a read of history); payoff: the
  **net** of fixed and floating per payment date (A1.b); no notional exchange (E1: the profile's
  legs never include one; a test asserts it).
- **Curve** (C1): one market per tenor (data `irs.tenors`); the swap curve is the set of cleared
  fixed rates (C1.a: a read); forward rates derived (C2); the swap spread vs the sovereign curve is a
  read (C3, C3.a). E2: the fixed rate never comes from the discount curve: the profile has no
  `parRate()`; the mark discounts the fixed leg against the floating leg's forwards read from the
  curve of prints.
- **Participants**: issuers with fixed debt wanting floating and vice versa (B1: the firm's and the
  bank's participants from their own debt rows), a bank managing its gap (B3: from its own repricing
  read of item 11), a **view** on rates (B4: a party whose outlook of the policy path differs from
  the curve: desks here, hedge funds at 13h), desks (B5); pensions (B2, B2.a) at 13h: declared
  PARTIAL here and closed there.
- **Cash**: variation margin from 13a turns the mark into cash (D3, D3.a: the hedger's funding
  problem shows in its cash while its hedged item revalues the other way).

### Module `fx-derivatives`

- `requires: ['derivative-layer', 'spot-fx', 'money-market']`.
- **Kind** `fx.forward`: terms `{ buy: Currency, sell: Currency, buyAmount, sellAmount, maturity }`
  (A1.d: a leg states its own money); payoff at maturity: both notionals move, two money legs in two
  currencies in one instruction (A1.b, A2, E3: DvP across currencies); the price is the forward rate
  (A1.c) cleared in a market per (pair, tenor); the mark is against the forward print for the tenor
  **left** (A3: a parity-struck forward is worth nothing at strike and earns carry over its life).
- **FX swap** (A4): spot one way, forward back: two instructions written together by the
  participant that wants a secured loan of one currency against another; the bank's funding
  participant for a foreign book (D3) uses it; it is booked as what it is: a spot trade and a forward
  row.
- **The forward rate** (B1): cleared. **CIP** (B2, B2.a): an arbitrage participant per bank: borrow
  one currency (a money-market quote), buy the other spot, lend it (a money-market offer), sell it
  forward: the four legs are real orders sized by the bank's balance-sheet headroom and capital
  charge (B2.b: not free), so the forward sits near spot × the funding differential because banks
  take the trade, and the residual **basis** is one read (B3, B3.b: no second basis; the only basis
  is `forward − spot × (1 + r_buy)/(1 + r_sell)` read from prints).
- **Kind** `xccy`: notionals exchanged at start and end at the original rate (C1, C3), periodic
  interest both legs each on its own currency's benchmark or a fixed rate (C1.a); its price is the
  basis on one leg (C4); used by issuers that raised in one currency and need another (C2).
- **Participants**: importers/exporters with known foreign payments (D1: from their invoice books),
  investors holding foreign assets who roll (D2, D2.a: a recurring cost in their P&L), banks funding
  foreign books (D3), desks whose width is their carrying cost (D4).
- **Reads**: E4: for a party with a hedged foreign asset, the asset's FX revaluation and the
  forward's mark move against each other and the residual is the basis and the imperfection: an
  observer read per party, never zeroed.

### Module `index-futures`

- `requires: ['derivative-layer', 'indices', 'equity', 'dealers']`.
- **Kind** `index.future`: terms `{ index, expiry, multiplier }`; underlying = `view.index(id)`
  (Indices C3: the index is a settlement price); cash-settled at expiry against the index read;
  margined daily by 13a.
- **Participants**: desks hedging equity inventory (Dealer Desks E1: a desk long shares posts a sell
  schedule sized to its net inventory; E2: the hedge is a contract with a counterparty and its own
  margin), funds with mandates that track the index (a cheaper way to be exposed), a view.

### Parameters

`cds.tenors`, `irs.tenors`, `fx.forward.tenors` (data); `cds.index.series.*` (data: names, weights,
roll date); `regulation.riskWeight.cds.sold`, `regulation.riskWeight.derivative` (policy, parliament);
no `recovery.rate` (D2.a), no `basis.*`, no `parity.*` exist.

### Audit contributions

- `zeroSum`: each class's marks and legs enter 13a's family unchanged.
- `crossMarket`: CDS C3 basis and IRS C3 swap spread are reads, not checks (they are consequences);
  Indices C3: the future's settlement equals the index read on expiry (a check).
- `flows`: FX Forwards E3: no maturity passes without both legs settling (an instruction that fails
  is a recorded fail in the estate path, never a silent skip).

### Files

```
packages/engine/src/clearing/contractMarket.ts (quotedAs: 'rate')
packages/engine/src/mechanisms/cds/{index.ts,contract.ts,event.ts,indexSeries.ts,participants.ts}
packages/engine/src/mechanisms/irs/{index.ts,contract.ts,curve.ts,participants.ts}
packages/engine/src/mechanisms/fx-derivatives/{index.ts,forward.ts,swap.ts,xccy.ts,cip.ts,participants.ts}
packages/engine/src/mechanisms/index-futures/{index.ts,contract.ts}
packages/engine/src/mechanisms/dealers/hedge.ts (E1)
packages/engine/test/{rate-markets,cds,cds-event,cds-index,irs,swap-curve,fx-forward,fx-swap,cip,xccy,index-future,desk-hedge}.test.ts
```

---

## Steps

- [ ] Rate-priced contract markets: the print is a rate in a declared unit; struck at par by the solver, never by a formula run backwards (D7.b); malformed schedules throw; tests
- [ ] `cds` kind: reference must be a party with defaultable debt; premium leg in cash stopping on the event; mark read from the spread curve; tests (A1–A4)
- [ ] CDS participants with reasons on both sides and a view on both; naked sellers capitalised; the second-opinion assembly check holds for every CDS book; tests (B1–B5)
- [ ] CDS curve across tenors; implied PD and recovery derived from spreads; basis vs the cash bond at every tenor both books print, as a read; tests (A1.d, C1–C4)
- [ ] CDS event: from item 5's credit event; no premium, expected mark, held past maturity until the estate closes; settlement at the realised recovery; payment can fail the seller; termination; tests (D1–D5)
- [ ] CDS index series: names fixed at the roll, a name's event settles its weight once per contract, the line runs on; own book; index-vs-single-name basis read; tests (A5)
- [ ] Wrong-way risk in the buyer's reservation through the counterparty term; net notional per reference as a read; tests (E1–E4)
- [ ] `irs` kind: two legs with own periodicity and accrual; floating fixes on the compounded overnight print; only the net moves; no notional exchange; tests (A1–A4, E1, E3)
- [ ] IRS participants: fixed↔floating issuers from their own debt, banks' gap from their own repricing read, a view, desks; B2 pension declared PARTIAL to 13h; tests
- [ ] Swap curve as a read of cleared fixed rates; forward rates derived; swap spread vs the sovereign curve read; no par rate from a discount curve exists (E2); tests (C1–C4)
- [ ] `fx.forward` kind: two money legs in two currencies settle in one instruction at maturity; mark against the forward for the tenor left; margined; tests (A1–A3, E3)
- [ ] FX swap as spot plus forward booked as a secured loan of one currency; banks fund foreign books with it; tests (A4, D3)
- [ ] CIP as an arbitrage banks take with four real legs bounded by balance sheet and capital; one basis read from prints; no parity formula anywhere (E1, B3.b); tests (B1–B4)
- [ ] `xccy` kind: notionals exchanged at start and end at the original rate, periodic interest both legs, price includes the basis; tests (C1–C4)
- [ ] FX hedgers: invoice books, foreign-asset holders rolling, desks with width from carrying cost; E4 residual as an observer read per party; tests (D1–D4, E2, E4)
- [ ] `index.future` kind: cash-settled against the index read at expiry; margined; tests (Indices C3)
- [ ] Desks hedge inventory with a contract that has a counterparty and margin (Dealer Desks E1, E2); item 9's hedging PARTIAL closed; the test-only forward from 13a deleted; tests
- [ ] Observer: curves per class, bases, net notional per reference, hedged residuals; year-long run green in two currencies; determinism; scenario test: swap spread and CDS basis behave differently calm and stressed (direction only)
- [ ] Coverage re-marked; PARTIAL rows for IRS B2/B2.a named to 13h; record entry
- [ ] Delete this file; worklist row 13b → done; commit and push

## Exit criteria

Every class clears on its own book at a rate or price the solver struck; a credit event pays what
the estate actually recovered; the swap curve and the forward curve are reads of prints; the basis
is one number; a desk's hedge is a position with a counterparty.

## Guard

CDS D2.a, E4, B5; IRS E1–E3; FX Forwards B3.b, E1–E3; Corporate Credit H4.a; Derivative contract
X3.
