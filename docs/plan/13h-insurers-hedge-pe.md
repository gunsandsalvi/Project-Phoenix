# Item 13h — Insurers and pensions, hedge funds, private equity

**Objective.** Three institutions with three different relationships to loss. An insurer or pension
owes a scheduled liability to named beneficiaries whose present value is read from a market curve,
so a rate move is a solvency event for it and it is the structural holder of duration; it writes
cover at a price that answers its own experience and capital, draws its own claims, matches, hedges
with swaps that call margin, and can fail. A hedge fund's investors bear the result through
redeemable equity; its leverage is a loan from a prime broker; it is the speculative side every
derivative book needs, the buyer when others are forced, and the first link of the loss loop. A
private-equity fund calls committed capital on dates investors cannot refuse, buys firms with debt
raised against the target by lenders that decide, holds them at a mark that is honestly not a
price, and exits only when a market is open. This item also closes what earlier items declared
PARTIAL for these holders: the pension's swap demand, the household's pension claim, fund fee
competition, portfolio shifts across currencies.

**Read first.** §27 Insurers and Pensions (all); §28 Hedge Funds (all); §29 Private Equity (all);
IRS B2, B2.a; Households F3; Fund Shares B3 (the fee shape from item 8); Currency E2; §35 (the
control market from 13g is what a buyout bids through, which is why 13g precedes this item); XI-2, XI-3; §15 (the broker). Code: item 8's funds and gates,
13f's prime brokerage and securities lending, 13b's swaps, 13g's tender market, item 7's estate.

**Clauses this item meets.** Insurers A1, A2, A2.a, A2.b, A3, A4, A4.a, A4.b, A4.c, B1, B2, B2.a,
B2.b, B3, B4, C1, C2, C2.a, C3, C4, C5, D1–D5, D4.a, E1–E4; Hedge Funds A1–A5, B1, B1.a, B2, B3,
B4, B5, C1–C4, D1–D7, D4.a, D5.a, E1–E3; Private Equity A1, A2, A2.a, A2.b, A3, A4, A5, B1, B2,
B2.a, B2.b, B3, B4, B5, C1–C5, C5.a, D1–D5, D4.a, E1–E3; IRS B2, B2.a; Households F3, F3.a (fully);
Currency E2 (fully); Fund Shares B3 (the shape's death); CDS B5 and IRS B4 (the view on both sides,
fully).

---

## Design

### Module `insurers`

- `requires: ['households', 'funds', 'irs', 'securities-lending', 'ratings', 'estate']`.
- **Party kinds** `insurer`, `pension` (A1: named, with accounts and registers); the sponsor of a
  pension is a firm (a term).
- **Liabilities** (A2, B1): instrument kinds `policy` (an insurer's liability to a beneficiary cell:
  terms `{ line, cover, term, premiumPerPeriod }`) and `pensionClaim` (a pension's liability to a
  cohort cell: terms `{ schedule: [{ period, amount }] }` accrued from contributions); both
  `liabilityOfIssuer: true`, `pricing: 'derived'` (item 8.1) with `value` = the schedule discounted at
  the **swap curve read** for each cash flow's tenor (B2; B2.b: no fixed rate, no cash balance: the
  liability never stores its value; a stored liability is impossible by construction because the
  profile has no store), so falling rates raise it (B2.a). The beneficiary does not absorb the
  investment result (A2.a); a unit-linked contract is item 8's `fund.share` and is declared so
  (A2.b). E1: every liability row names its beneficiary (the register's holder).
- **Cover** (A4): a **cover market** per line in `markets`: household cells (and firms for
  commercial lines) post demand for cover from their own decision (a line in the cell's basket:
  item 4's consumption decision gains insurance as a good whose price is the premium); insurers post
  supply at their **quote** = expected claims per unit of cover from their **own** experience (A4.c:
  a read of their own claims history per line) plus the return required on the capital held against
  the premium (A4.b: `insurer.requiredReturn` preference × the capital charge read from item 11's
  risk weights: `regulation.riskWeight.insurance.<line>`), sized by what its **surplus** can stand
  behind (A4.a: cover written ≤ surplus / the line's measured loss width: a read of its own history,
  not a ratio parameter); the solver gives the policy to the lower quote; an insurer with no surplus
  posts nothing and loses renewals (it loses book before licence); unplaced cover pays no premium.
- **Claims** (A4.c, B3, B4): per member, an event from the line's frequency and severity
  (technology primitives `claims.<line>.frequency`, `claims.<line>.severity`, declared as such: they
  are the world's physical hazard, not a credit rate: XI-1 concerns defaults, which stay events of
  state); a **catastrophe** is one journaled event hitting every policy of a line in a region at
  once (a scenario seed's event; B4); claims are instructions insurer → beneficiary.
- **Assets** (C): the portfolio decision (C1) matches the schedule's tenors (C2: the participant's
  bond and swap demand is built from the liability schedule's cash flows by tenor: C2.a: a one-way
  demand for long bonds and receiver swaps: IRS B2 → MET); illiquid assets: a limited partner in PE
  (C3: below); lends securities (C4: 13f's market); mandate limits credit by grade (C5: item 12's
  forced sale on downgrade applies).
- **The gap** (D): duration and cash-flow mismatch reads (D1); equity (A3: a read) moves opposite to
  a bank's (D2: measured); a **funding shortfall** (assets < liabilities at the read) triggers a named
  action in the party's `decide`: the sponsor contributes (an instruction sponsor → pension from the
  sponsor's cash plan: a decision the sponsor can refuse), the fund de-risks (sells), or benefits
  are cut (a journaled amendment of the schedules: a contractual term says whether allowed) (D3);
  hedging with receiver swaps (D4) creates margin calls through 13a (D4.a: the sector's failure mode:
  a rate rise improves solvency and demands cash); D5 measured at 16.
- **Failure** (A3, XI-3): negative equity → `resolution`: policies and claims are transferred to a
  successor insurer that bids for the book (a tender in the cover market) or into the estate, where
  beneficiaries rank as unsecured creditors (E1: nobody's claim vanishes).
- **Pensions and households** (Households F3): contributions from wages (an instruction cell →
  pension per period from item 4's income) accrue the claim's schedule; at retirement (13d's cohort
  crossing) the claim pays its schedule: the retired cell's income (F3 PARTIAL → MET).

### Module `hedge-funds`

- `requires: ['funds', 'prime-brokerage', 'securities-lending', 'derivative-layer', 'cds', 'irs',
'fx-derivatives', 'index-futures', 'commodity-futures']` (C1: it participates in every book).
- **Party kind** `hedgeFund`: item 8's fund with a **wide mandate** (A4: eligible: every kind; long,
  short, levered), investors' equity as redeemable shares (A2) with a **notice period and a gate**
  (D5.a: terms; item 8's gate mechanism), a manager party earning a management fee and a
  **performance fee** on gains above a high-water mark (A3: an instruction fund → manager when NAV
  exceeds the mark: a term), everything marked at cleared prices (A5, E2: an unmarked position
  is `Unpriced` and throws).
- **Leverage** (B): a margin loan from its prime broker (B1, B1.a, E1), derivatives (B2: the notional
  exceeds the margin), repo (B3: item 11) — the amount is the lender's decision (B4); gross, net and
  equity are three reads on the observer (B5).
- **What it does** (C): positions for reasons (C1): the `speculative` participant in every derivative
  and credit book (its outlook vs the print: this is the party that lets the market disagree with
  the model; XI-13's assembly check is satisfied by hedge funds on both sides of every book, so the
  desks' own view is no longer the only one); a buyer of forced sales when it has capacity (C2: its
  participant reads the session's `forced.sale` orders as an opportunity and bids at its own
  valuation within its remaining line); shorts with borrows (C3: 13f); every trade is a cleared
  trade (C4).
- **The failure loop** (D): loss → equity down, leverage up (D1: reads) → the broker's call (D2:
  13f's margin) → sales that move prints (D3) → other holders' marks and their calls (D4: emergent,
  D4.a) → redemptions (D5: item 8) → failure (D6: NAV ≤ 0 or an unmet call: the broker eats the
  shortfall against its capital, the investors lose their equity; E3: it can fail); D7 traceable.

### Module `private-equity`

- `requires: ['funds', 'corporate-control', 'corporate-credit', 'bank-lending', 'equity', 'insurers']`.
- **Party kinds** `peFund` (A1: committed capital from named investors: insurers, pensions, wealthy
  cells; a `commitment` row per investor with an amount and the fund's life: A4), `holdco` (A5: a
  vehicle per deal, a party with its own balance sheet).
- **Calls** (A2): the fund's `decide` posts a **capital call** when a deal needs equity: an
  instruction investor → fund on the date, which the investor's cash plan must meet from its
  liquidity ladder (selling if it must: A2.a); an unmet call **fails** the instruction (item 5's
  Failed state: A2.b: a default on the call, journaled, with the contractual consequence: the
  defaulting investor's commitment is forfeited pro rata (a term)); nothing bounds a call to spare
  cash.
- **The buyout** (B): the fund bids for a firm through 13g's tender market (B1); the price is
  funded by **debt raised against the target** (B2: a loan or bond issued by the holdco/target
  through 13f's markets, secured on the target's assets; B2.a: the target's liability; B2.b: lenders
  decide, so the deal happens only if the debt clears: the tender bid is conditional on the financing
  clearing in the same session: the commitment market for the acquisition loan runs before the
  tender) and the **equity cheque** from calls (B3); at completion the target's register shows the
  holdco as owner and the target's balance sheet carries the new debt (B4); B5: sources = uses
  exactly: an audit contribution per deal from the ledger.
- **The hold** (C): the target operates and services its debt (C1); the owner influences investment,
  costs and distributions (C2: the holdco's participant sets the firm's management preferences: the
  firm module's decision reads its owner's preferences through its view when the owner holds
  control: 13g's control read); recapitalisation (C3: new debt at the target paying a distribution
  to the holdco: a decision the lenders can refuse); failure (C4: item 7: the lenders lose, the
  equity is wiped, the fund's other holdings are unaffected: B2.a); the **mark** (C5): the holding's
  value in the fund's NAV is a mark from the fund's own model (its outlook of the target's earnings
  at its own hurdle), carried with provenance `marked` (C5.a: the prices family shows "marked, not
  cleared"; it never enters an index or a collateral haircut as a cleared price: a kind with
  `pricing: 'derived'` whose read reports `marked`).
- **The exit** (D): a sale to a corporate or another fund through 13g's tender market, or an IPO
  through item 9's issuance (D1); the exit price is the first cleared price (D2); proceeds
  distributed to investors in cash by instructions (D3); a closed market (a failed primary or no bid
  above the fund's reservation) extends the hold (D4) and investors owe calls while receiving no
  distributions (D4.a: emergent); D5: returns are a read of distributions against calls.
- **Wind-up** (A4): at the end of the life the fund sells what it holds into whatever market exists
  and distributes; an unsold holding at wind-up is distributed in kind (the shares themselves) to
  investors pro rata: claims resolve, never freeze.

### Fee competition and portfolio shifts

- Item 8's `fund.managementFee` shape dies: each manager quotes a fee (its decision from its own
  costs and the flows it observed); each investor's subscription decision (Households D5) compares
  funds on net expected return from its own outlook; the fee is a term struck at subscription.
- Currency E2 (portfolio shifts across currencies) is complete once insurers, hedge funds and PE
  hold foreign assets from their own decisions (12's revaluation applies to them unchanged).

### Parameters

`insurer.requiredReturn` (preference per insurer); `regulation.riskWeight.insurance.<line>`
(policy, parliament); `claims.<line>.frequency`, `claims.<line>.severity` (technology); `hedgeFund.
noticePeriods`, `hedgeFund.gate` (terms per fund: data); `peFund.life` (a term); `pe.callForfeit`
(a term). Deleted: `fund.managementFee` (item 8's shape). No `catastrophe.probability` as a
mechanism input (a scenario event), no `contagion.*`, no `hurdle` other than the party's own.

### Audit contributions

- `accounts`: an insurer's equity is assets at marks minus liabilities at the curve read, both from
  the register (E3: no stored liability exists); Insurers E4: sector holdings enter the ownership
  family unchanged.
- `flows`: PE B5 sources = uses per deal; calls and distributions two-sided.
- `prices`: a `marked` provenance is reported distinctly and never counted as cleared.
- `names`: every liability has a beneficiary (E1); every holdco has an owner and a target.

### Files

```
packages/engine/src/mechanisms/insurers/{index.ts,liability.ts,cover.ts,claims.ts,matching.ts,gap.ts,pension.ts,resolution.ts}
packages/engine/src/mechanisms/hedge-funds/{index.ts,mandate.ts,leverage.ts,participants.ts,fees.ts}
packages/engine/src/mechanisms/private-equity/{index.ts,commitments.ts,buyout.ts,hold.ts,exit.ts,mark.ts}
packages/engine/src/mechanisms/funds/fees.ts (competition), households/portfolio.ts (fund choice, pension contributions)
packages/engine/test/{insurer-liability,cover-market,claims,catastrophe,matching,gap-hedge,insurer-failure,pension-claim,hedge-fund,hf-leverage,hf-view,hf-loop,pe-calls,buyout,pe-hold,pe-exit,fund-fees}.test.ts
```

---

## Steps

- [ ] **From item 11 (XI-2 door three, Prime Brokerage C3.b)**: a broker cuts a leveraged client's line below what it has drawn, and the client's own module posts the sales that repay it, at whatever the book gives. Item 11 built this door for a BANK (a bank refused by the session sells its own paper) and published the line a bank will fund for one name (Banks Lending F3), but it has no client to cut: this world's only leveraged holder of marketable assets is a desk, and a desk is its own bank's arm. A hedge fund with a prime broker is the party. Test: `limit − exposure` is negative with no floor in the path, the sale moves the print, and the print reaches other holders
- [ ] `insurer` and `pension` kinds; `policy` and `pensionClaim` liabilities to named beneficiaries valued at the swap curve read each time; no stored or fixed-rate value can exist; tests (A1–A3, B1, B2, B2.a, B2.b, E1, E3)
- [ ] The cover market: quotes from own experience and capital, sized by surplus; policy to the lower quote; unplaced cover; premiums and claims as instructions; an insurer with no surplus writes nothing; tests (A4, A4.a–A4.c)
- [ ] Claims as per-member events from declared technology primitives; a catastrophe as one event on many policies; tests (B3, B4)
- [ ] Matching: bond and swap demand built from the liability schedule by tenor (IRS B2 → MET); LP in PE; securities lending; downgrade-forced sales; tests (C1–C5)
- [ ] The gap: duration and cash-flow reads; equity moving opposite to a bank's; shortfall actions by the sponsor, the fund or the schedule; swap hedges that call margin; tests (D1–D4, D4.a)
- [ ] Failure: negative equity resolves to a successor's tender or the estate with beneficiaries ranking; tests (A3, XI-3)
- [ ] Pension contributions from wages accrue the schedule; drawdown at retirement is the retired cell's income (Households F3 → MET); tests
- [ ] `hedgeFund` kind: wide mandate, redeemable equity with notice and gate, manager with performance fee above a high-water mark, every position marked or `Unpriced`; tests (A1–A5, D5.a, E2)
- [ ] Leverage from the prime broker, derivatives and repo at the lender's decision; gross, net and equity as three reads; tests (B1–B5, E1)
- [ ] Positions for reasons: the speculative participant on both sides of every derivative and credit book (XI-13 check now satisfied by hedge funds); buyer of forced sales within its line; shorts with borrows; tests (C1–C4, CDS B5, IRS B4)
- [ ] The loop: loss, call, sale, price, other funds' calls, redemptions, failure landing on the broker and the investors; scenario test traceable party by party; tests (D1–D7, E3)
- [ ] `peFund` and `holdco` kinds; commitments; calls as instructions on dates that fail when unmet with the contractual forfeit; manager fees; life and wind-up resolving to cash or in-kind; tests (A1–A5, E2)
- [ ] The buyout: a conditional tender funded by target debt that lenders clear and an equity cheque from calls; the target's balance sheet transformed; sources = uses contribution; tests (B1–B5, E1)
- [ ] The hold and the exit: owner influence through preferences, recapitalisation lenders can refuse, failure wiping the equity only, the `marked` provenance shown as such, exit by tender or IPO as the first cleared price, a closed market extending the hold, distributions in cash, returns as a read; tests (C1–C5, D1–D5, E3)
- [ ] Fund fee competition: managers quote, investors compare on their own outlook; item 8's fee shape deleted; Currency E2 complete; observer: liabilities and gaps, cover books, hedge-fund reads, PE deals with marks flagged; year-long run green with a rate-move scenario; determinism
- [ ] Coverage re-marked; record entry with the shape's death; delete this file; worklist row 13h → done; commit and push

## Exit criteria

A rate move revalues a pension's liability, its hedge and its cash in three different places; an
insurer can lose its book before its licence and can fail; a hedge fund's loss reaches another fund
through a broker and a price; a buyout happens only when lenders lend and its sources equal its
uses; a PE mark is never mistaken for a price.

## Guard

Insurers B2.b, E1–E3; Hedge Funds E1–E3; Private Equity A2.b, C5.a, E1–E3; XI-13 (a view on both
sides of every book).
