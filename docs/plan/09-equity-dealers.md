# Item 9 — Equity, and dealers that carry inventory

**Objective.** Shares as an instrument that is a residual claim and not a liability, with a vote,
priced by schedules that carry participants' opinions, issued and bought back as decisions the
market prices and can refuse; dealers as desks inside banks with inventory, a limit, a funding cost
paid every period and a skew, whose stepping back is what a failed market is. A share must exist
and have a price before item 10 can ask what equity costs; a dealer must have a limit before a market
can fail because it stepped back.

**Read first.** §10 Equity (all), §26 Dealer Desks (all), XI-4 joint three, XI-13 (the second
opinion), Clearing B3, B3.a, E3, C4; §32 Firm E4, E5; §41 Households D1, D6. Code: `clearing/solver.ts`,
`clearing/market.ts`, `registry/kinds.ts`.

**Clauses this item meets.** Equity A1, A1.a, A1.b, A2, A2.a, A3, A4, A5, A5.a, A6, B1, B2, B3, B4,
B4.a, B5, B6, C1, C1.a, C1.b, C2 (a–e), C3, C4, C6 (pledge; lending at 13f), D1, D1.a, D1.b, D1.c,
D2, D2.a, D2.b, D2.c, D3, D3.a, D3.b, D4, E4, F1, F2, F3, F4, G3; Dealer Desks A1–A4, B1, B2, B3, B4,
C1, C2, C2.a, C3, C4, C5, C5.a, C5.b, C5.c, D1, D2, D3, D4, D4.a, D5, E1, E1.a, E2, E3, E4, F1, F2,
F3; Clearing B3, B3.a, E3; Firm E5. Remain PARTIAL: Equity C5, C7 (leverage and shorts: 13f), E1–E3
(13g), G1, G2 (12).

---

## Design

### Module `equity`

- `requires: ['firms', 'estate']`.
- **Kind** `share`: `pricing: 'cleared'`, `liabilityOfIssuer: false` (A1: a residual claim), unit
  `shares` (countable: A2), terms `{ votesPerShare: 1 }` (A5), `due`: dividends declared by the firm
  (the profile reads the issuer's declared dividend from a journal event: `equity.dividendDeclared`,
  and places it on the date), `defaultOn: null` (equity does not default; it is wiped: E4),
  `ranking`: last. Display name: the issuer (A6).
- **Register**: shares outstanding is `issued`; treasury shares are the issuer's own holding (C2.d):
  a holder equal to the issuer is allowed for this kind (the kernel's settlement treats an asset leg
  to the issuer as a redemption; for a kind that declares `issuerMayHold`, a buyback that is not a
  cancellation credits the issuer's own holding instead: sub-item 9.1, kernel door). Free float (C1.b)
  is a read: issued − insiders' and strategic holdings (holdings by parties whose relationship to
  the issuer is declared in the register as a `strategic` flag on the holding: a register row
  attribute set at acquisition by the acquiring module, e.g. founders at 13g; here the seed).
- **The market** (B): one market per share, opened at seed for listed firms; participants: household
  cells (C2.a) from their own view (their opinion is their outlook of the firm's earnings, discounted
  at their own patience: B3 says a multiple or a DCF is an opinion held by a participant, so the
  schedule is built from it, never the print), institutions with mandates (funds: item 8, their
  mandate's eligible share), the issuer (buybacks), insiders (not for sale: C2.e), dealers (below).
  Market capitalisation is a read (B4); B4.a forbids the tautology, so no test compares it to itself.
- **Issuance** (D1): the firm's decision (Firm E4: when its investment programme needs funding it
  prefers to meet with equity, item 10) posts a primary market (item 3.1's primary block) with size
  and reservation; dilution is the register's arithmetic; it can fail (D1.c).
- **Buyback** (D2): the firm posts a buy order; shares bought are cancelled (redemption) or held
  (treasury shares); it competes with the dividend and investment (D2.c: the firm's phase decides
  among them from its cash above its buffer and its cost of capital at 10).
- **Dividend** (D3): declared in `firms.decide`, journaled publicly, paid by the kernel's corporate
  actions to holders of record on the date (D3.a).
- **Split** (D4): a journaled instrument event changing `issued` and every holding by the ratio in
  one kernel operation (`instruments.split`, a door) that moves no value; the prices family's next
  print is per new share (the market runner rebases the carried print by the ratio, journaled).
- **Insolvency** (E4): the estate (item 7) wipes the register to zero before any creditor takes a
  loss: shares are redeemed at zero as the estate opens.
- **Entitlements** (F): dividends, the residual, the vote (F3: a read: `votes(party, share) =
holding × votesPerShare × weight`, used at 13g and 14); F4: retained earnings reach holders only
  through the price (no income credited: the households module reads dividends received, never the
  firm's retained earnings).

### Module `dealers`

- `requires: ['bank-lending']` (a desk lives inside a bank: A1).
- **Party kind** `desk` (named; `moneyIssuer: null`; a desk's account is at its bank; its
  balance sheet is inside the bank's: the desk is a party whose equity account is a sub-account the
  bank consolidates: the kernel's accounts family treats a desk as a party; the bank's capital reads
  its desks' positions through `view.desks()`... simpler: a desk is a **party** and the bank holds
  the desk's equity as an instrument `desk.equity` (liabilityOfIssuer true) so consolidation is
  ownership, not a special case. F2: the desk is not exempt from its bank's capital: the bank's
  capital computation at 11 includes desks it owns).
- **Quotes as schedules** (C1–C5): each period, for every market the desk makes (a term of the
  desk: the instrument kinds it quotes), the desk's participant posts a **bid and an offer
  schedule** from its own state: mid = its view of value (its outlook of the price: XI-13's second
  opinion), skewed by inventory (C2: long → both sides lower, proportional to inventory over its
  limit), widened by risk (C3: its recent surprise width on that instrument: the confidence read) and
  by adverse selection (C4: the size of the flow it faced last period), sized by its remaining
  limit (D1). The spread is the output (C5); no mid-plus-spread parameter exists (C5.a, C5.b: the
  lint's no-magic-numbers refuses a width constant; the only declared numbers are the desk's limit
  (preference), its required return on capital (preference) and the capital charge (policy)).
- **Rent** (D3, XI-4 joint three): every period an instruction desk → bank for the funding cost of
  its inventory at the bank's cost of funds (item 6's placeholder until 11) plus the capital charge
  (D2); the desk's P&L is spread earned minus what the inventory did (F3: the accounts family already
  makes that so).
- **Limits** (D1, D4): a position limit per instrument and in aggregate (preferences set by the
  bank's risk function: the bank's own view at 11; here the desk's own); when binding, the desk
  widens, shrinks, or posts nothing (D4.a: a market with no dealer and no natural counterparty fails,
  which the solver already reports).
- **Interdealer** (E3): desks are participants in every market they make, so they face each other
  through the same session; hedging (E1) is a trade with a counterparty (a desk long a bond posts a
  sell in the bond future at 13b or the swap at 13b: here, a desk hedges an equity position with the
  index future at 13b; until then, hedging is absent and declared PARTIAL).
- **B4 guard**: a desk's schedule reads its own state only (its view); the test asserts a desk
  posts the same schedule whether or not the market has other orders.

### Parameters

`desk.limit.perInstrument`, `desk.limit.aggregate` (preferences per desk, dispersed); `desk.
requiredReturnOnCapital` (preference); `regulation.riskWeight.share` (policy); `equity.votesPerShare`
(a term, data); household `patience` reused for the equity opinion.

### Audit contributions

- `ownership`: C1.a (held = outstanding) is the kernel's existing check; treasury shares included.
- `accounts`: F3 (a desk that cannot lose money is not a dealer): the desk's equity moves with its
  inventory's marks: kernel revaluation already does it; the observer shows desk P&L decomposed into
  spread earned and inventory revaluation (Observer D2: performance can be bad).

### Files

```
packages/engine/src/registry/kinds.ts, register/instruments.ts (9.1: issuerMayHold, split door, strategic flag)
packages/engine/src/mechanisms/equity/{index.ts,issuance.ts,buyback.ts,dividend.ts,split.ts}
packages/engine/src/mechanisms/dealers/{index.ts,quote.ts,rent.ts,limits.ts}
packages/engine/src/mechanisms/firms/… (dividend, buyback, issuance decisions)
packages/engine/src/mechanisms/households/… (equity opinion and orders)
packages/engine/src/seeds/foundation.ts (listed firms, founders' strategic holdings, desks)
packages/engine/test/{equity,issuance,buyback,dividend,split,dealers,desk-limits}.test.ts
```

---

## Steps

- [ ] 9.1 Kernel: `issuerMayHold` for treasury shares; `instruments.split` door; `strategic` holding flag; tests
- [ ] `share` kind: residual claim, countable, votes, dividend `due` from a declared event, wiped at insolvency; tests
- [ ] Share markets opened at seed for listed firms; household participants build schedules from their own outlook and patience (B3 as an opinion in the schedule); test: two cells with different outlooks post different prices
- [ ] Issuance as a primary market with size and reservation; dilution; failure; tests
- [ ] Buyback: cancellation or treasury shares; competes with dividend and investment in `firms.decide`; tests
- [ ] Dividend declared and paid to holders of record; F4: no income without cash; test: a cell holding shares of a firm that retained earnings shows no income
- [ ] Split moves no value; the print rebases; test
- [ ] Estate wipes equity to zero before creditors (E4); test
- [ ] Votes as a read (F3, A5); test: a cell casts weight × votes
- [ ] Free float as a read excluding strategic holders; test
- [ ] `desk` party kind owned by its bank through `desk.equity`; consolidation by ownership; tests
- [ ] Desk quotes: two schedules from own state; skew by inventory, width by surprise width and adverse selection, size by remaining limit; no width constant exists (lint); tests: long skews both sides down; a limit binding shrinks then stops
- [ ] Desk rent every period at the bank's cost of funds plus the capital charge; test: a desk carrying inventory for free is unreachable
- [ ] Desks in every market they make: bonds, shares; interdealer through the same session; test: two desks with opposite inventories trade with each other
- [ ] D4.a: a market whose only liquidity was a desk at its limit fails and prints stale with reason; test
- [ ] D5 as an observer read: inventory, spread, capital usage together
- [ ] Households and funds as equity participants; the foundation seed lists firms with founders' strategic holdings
- [ ] From item 8 (Fund Shares E1, E2): an `etf.share` kind whose shares TRADE — a market, a cleared price, and its NAV read beside it from the same book, so it has two values and they are different numbers
- [ ] From item 8 (E3, E3.a, G1.a): creation and redemption IN KIND against the basket, by a desk with a reason and a limit — which is why an exchange-traded fund is not a forced seller, and why the gap can persist when nobody will close it. It waits for this item because arbitraging a gap for nothing is the free arbitrage Appendix B forbids: it needs a party that carries inventory, pays for the capital it uses, and runs out of limit
- [ ] From item 8 (E4): the premium or discount as a read of the two prices, on the observer; a persistently large one is a finding about liquidity and never a number to clamp
- [ ] Year-long run green; determinism; XI-13 test: a desk's schedule is independent of the other orders in the book
- [ ] Coverage re-marked; PARTIAL rows for C5, C7, E1–E3, G1, G2, hedging named; record entry
- [ ] Delete this file; worklist row 9 → done; commit and push

## Exit criteria

Shares clear from opinions in schedules; issuance can fail; desks carry inventory that costs them
rent every period and their spreads come out of their state; a market can fail because the desk
stepped back.

## Guard

Equity B3, B4.a, F4; Dealer Desks B4, C5.a, C5.b, F1, F2, F3; Clearing B4, B5.
