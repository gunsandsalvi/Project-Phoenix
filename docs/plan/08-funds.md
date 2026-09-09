# Item 8 — Redeemable claims

**Objective.** Funds as named parties with a share count, a mandate that constrains what they buy, a
net asset value that is a read, subscriptions that buy and redemptions that must find cash by
selling into markets that clear; the money fund that can break the buck; the exchange-traded fund
with two values. This is the second door into the forced seller (XI-2 door 2).

**Read first.** §13 Fund Shares (all), XI-2, §41 Households D5, D5.a, §11 B5 (non-bank cash), §22
C2 (index mandates: 12), §14 B2 (lending: 13f). Code: item 7's forced-sale event, `clearing/market.ts`.

**Clauses this item meets.** Fund Shares A1–A4, B1, B2, B2.a, B3, B4, C1, C1.a, C2, C2.a, C2.b, C3,
C4, C4.a, C5, D1, D2, D2.a, D3, D4, D5, E1, E2, E3, E3.a, E4, F1, F2, F3, G1, G1.a, G1.b; Households
D5.a (the fund leg); XI-2 door 2; Money Market B5, B5.a (as a buyer of bills; the market itself at 11).

---

## Design

### Module `funds`

- `requires: ['households', 'estate']`.
- **Party kinds.** `fund` (named; `moneyIssuer: null`), `fundManager` (named).
- **Instrument kinds.**
  - `fund.share`: `pricing: 'nav'` (a fourth pricing: the price is the NAV read, not a print; the
    kernel gains `pricing: 'derived'` with `profile.value(instrument, period, view)` returning the
    per-unit value from a read: sub-item 8.1, a kernel door; the prices family accepts `derived` as
    priced when the profile's read succeeds), `liabilityOfIssuer: true` (A2, A3: the fund's liability
    is its shares, its equity is zero by construction), countable unit `shares`.
  - `etf.share`: `pricing: 'cleared'` (E1) with a market; its NAV is a second read on the fund's
    view (E2); the gap is a read (E4).
- **The mandate** (A4): terms of the fund party: `{ eligible: readonly InstrumentKindId[], limits:
{ kind, maxShare }[], buffer: preference }`; a fund buys only eligible kinds (a constraint enforced
  at order time by the module: an ineligible order is a contract violation in the module, not a
  clamp: the module never generates one).
- **NAV** (B1): `(Σ holdings at marks − liabilities) / shares outstanding`, read at the fund's view
  each time; never stored. Marks are the last prints (B2); a stale print makes a stale NAV and the
  subscriber or redeemer transacts at it (B2.a: a real transfer between holders, journaled `fund.
staleNav` public).
- **Fees** (B3, F3): the manager is a separate party; a fee instruction fund → manager each period
  (management fee rate: a term of the fund, priced by the manager: a shape until competition among
  managers exists; declared) reduces NAV.
- **Subscription** (C1): a household cell's portfolio decision (Households D5) posts a subscription
  request: an instruction: money leg cell → fund at NAV × shares, asset leg fund (issuer) → cell
  (issuance of shares) in one instruction; the fund's phase `funds.invest` (cycle 0, after
  `households.decide`) then posts buy orders per its mandate for the cash above its buffer (C1.a).
- **Redemption** (C2): the request is a redemption at today's NAV: an instruction: asset leg cell →
  fund (redemption of shares), money leg fund → cell. If the fund's cash is short, the instruction
  **fails** (recorded); the module then posts **sell orders** for the shortfall in the next session
  (C2.a, C2.b: the forced seller) with reason `forced.sale` (XI-2), and re-runs the redemption after
  the markets (phase `funds.redeem` cycle 2, after `markets`): the redeemer is paid at the NAV
  **struck when it asked** (C4: the timing mismatch falls on the remaining holders because the sales
  happened at today's prices: the difference is a revaluation the remaining holders carry through the
  NAV read). A redemption that still cannot be met after the sale is **not dropped** (C2.b FORBID):
  it stays queued and journaled `fund.gate` (public: a gate is information), and the fund's status
  is `gated` until met; a gated fund that cannot meet redemptions over the policy horizon fails
  (F: XI-3: its equity is gone when NAV ≤ 0; a gated fund with positive NAV keeps selling).
- **The money fund** (D): a fund whose mandate is bills and short paper (D1); its shares are counted
  in shares and its NAV floats (D4); a saver's deposit-substitute (D2): the household portfolio
  decision (item 4.6) compares the deposit rate, the money fund's yield (a read of its holdings'
  yields from the curve) and bills directly (D2.a, D5: a consequence, not an allocation).
- **The ETF** (E): a fund whose shares trade; creation and redemption in kind (G1.a: an authorised
  participant, a bank, exchanges a basket for shares; it is not a forced seller); the premium or
  discount is a read; an arbitrage participant (the bank) has a reason and a limit (E3, E3.a).
- **Failure** (XI-3): NAV ≤ 0 (a levered fund at 13h) or a gate past the horizon: `estate.open`.

### Parameters

`fund.managementFee` (shape, per annum, → competition among managers, 13h), `fund.buffer`
(preference per fund), `fund.gate.horizon` (policy, standardSetter), `fund.mandates.*` (data).

### Audit contributions

- `accounts`: A3: a fund's equity account is zero every period to dust (contributor `funds`).
- `ownership`/`flows`: C5: shares created − redeemed = outstanding; cash in and out matches, from
  the ledger.
- `prices`: B4: Σ holders' shares × NAV = assets − liabilities, exactly (the NAV read against the
  register: two reads of the same thing? No: NAV is derived from the register; B4 is a VERIFY of the
  read's arithmetic and is asserted as a unit test, not an audit check against itself).

### Files

```
packages/engine/src/registry/kinds.ts, prices/value.ts, audit/families/prices.ts (8.1: derived pricing)
packages/engine/src/mechanisms/funds/{index.ts,nav.ts,subscribe.ts,redeem.ts,moneyFund.ts,etf.ts}
packages/engine/src/mechanisms/households/portfolio.ts (the substitution gains the fund leg)
packages/engine/test/{funds,nav,redemption,money-fund,etf}.test.ts
```

---

## Steps

- [ ] 8.1 Kernel: `pricing: 'derived'` with `profile.value(...)`; valuation, revaluation and the prices family accept it; a derived value that throws is `Unpriced`; tests
- [ ] `fund`, `fundManager` party kinds; `fund.share` and `etf.share` kinds; mandates as party terms; tests
- [ ] NAV as a read; stale prints make a stale NAV, journaled; fee instruction to the manager each period; tests
- [ ] Subscription: one instruction (cash in, shares issued); the fund invests the cash per mandate above its buffer; test: an ineligible kind is never ordered
- [ ] Redemption: at the NAV struck when asked; a cash shortfall fails the instruction, posts forced sales, re-runs after the markets; the difference lands on the remaining holders; tests
- [ ] Gate: unmet redemptions queue, never drop; `fund.gate` public; a gate past the horizon fails the fund into the estate; tests
- [ ] Money fund: bills mandate, floating NAV, breaking the buck when a bill defaults; test
- [ ] Households' substitution among deposit, money fund and bills from the cell's own view (D2, D5.a); test: flows follow the yield gap
- [ ] ETF: traded share with a market, NAV read, in-kind creation and redemption by a bank participant with a limit, premium/discount as a read; tests
- [ ] Failure: NAV ≤ 0 opens an estate; investors' shares resolve to what the estate returns; test
- [ ] Audit: A3 and C5 contributions; observer: NAV, flows, gates, premium/discount
- [ ] Year-long run green with a redemption wave scenario seed; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 8 → done; commit and push

## Exit criteria

A redemption wave sells into the markets and moves prints; a gate is reachable and visible; the
money fund's NAV can fall below one; the fund's equity is zero to dust every period.

## Guard

Fund Shares C2.b (never rationed-and-dropped), D4 (no constant NAV), F1, F2; XI-2 (a floor at zero
on the fund's cash would delete the door).
