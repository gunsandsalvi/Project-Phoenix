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
- **Kind** `share` (`equity.share`): `pricing: 'cleared'`, `carry: 'mark'`, `liabilityOfIssuer: false`
  (A1: a residual claim), unit `shares`, terms `{ issuer, votesPerShare }` (A5), `due: []` and
  `cashFlows: []` (A4: perpetual, promising nothing dated — a dividend is a DECISION, not a term),
  no `defaultOn` (there is no promise to break), `ranking` last (A1.a). Display name: the issuer (A6).
  `splits: true` (D4).
- **The unit `shares`** moves to the kernel's `registry/profiles.ts` and the world's registry data: a
  claim on a book and a claim on a firm are both counted in shares and no one module owns it.
- **The market** (B): one per share line, opened at seed with an opening print. Participants: the
  household cells (C2.a) and the issuer (D2). **A participant's reason lives in the participant's
  own module**: the household's opinion of a share is in `households/portfolio.ts` beside its
  opinion of paper, in ONE traversal with ONE budget, so the same money is never committed twice
  (Law 4). A module that wrote other parties' reservations would be handing the market its answer.
- **The opinion** (B3): what the issuer last declared per share (public, D3), capitalised at what
  that cell requires of a claim that promises nothing — its liquidity premium plus how wrong its own
  income has recently been (§46 B3). Two cells disagree because their histories differ (§46 A3).
- **Issuance** (D1): a primary offer at the firm's own reservation (its book per share); dilution is
  the register's arithmetic; it can fail (D1.c).
- **Buyback** (D2): the firm bids at its reservation and what it buys is REDEEMED — the count falls
  (D2.a) and the cash is gone (D2.b). No treasury share: a residual claim on yourself is not an
  asset of yourself, and D2.b says the cash is gone rather than swapped for one. C2.d is PARTIAL.
- **Dividend** (D3): decided in `equity.decide` from what the firm's own funding read says it has
  spare, over its management's own patience, and paid to holders of record at the moment it is
  applied (E1.a). Declared publicly, which is what makes a cut an event others react to (D3.b).
- **Split** (D4): the kernel door (9.1).
- **Insolvency** (E4) **and the residual** (F2): both through the ranking. A share ranks last, so an
  estate reaches its holders after every creditor, pays them what is left and writes off the rest at
  zero. Nothing wipes anything as a special case. The module stops pricing a line whose issuer has
  been succeeded, and the print goes visibly stale (Clearing E4).
- **Founders are not here.** C2.e's insider — a block that is not for sale, whose vote a takeover
  must obtain (A5.a) — is a party that comes into existence by FUNDING A FIRM'S ENTRY (Firm Birth A),
  which is 13g. Seeding one would state who owns this world before anybody bought anything (Seed E1,
  E2), and a founder with no life to spend its dividends on is a hole money leaves the circuit
  through: the first build of it drained 215 PHX in a year and the treasury missed eight coupons.
  So C1.b's free float is a read of what is ENCUMBERED (Register D5.a: only free units move), which
  is nothing yet and says so; C2.e, A5.a and E1–E3 are PARTIAL, named to 13g.

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

- [x] 9.1 Kernel: `instruments.split`, `register.restate`, `prices.restate` and the `ctx.split` door; `splits` on the kind profile; the accounts family's dust takes in the walk behind every money balance it reads
- [x] `share` kind: residual claim, countable in shares, votes, perpetual, promising nothing, ranking last; the `shares` unit moves to the kernel
- [x] Share markets opened at seed for the listed firms, with an opening print that is a RESOLUTION and not a shape (a split is the invariance that proves it)
- [x] Household participants build schedules from their own opinion, in their own module, out of one budget spread over every place their savings could go
- [x] Law 2 defect found building the ETF and fixed at the cause: `households.demand.steps` is declared a RESOLUTION, and a ladder whose rungs sat at `k/(steps+1)` of the cell's own opinion topped out BELOW it and crept up with the count — a haircut on what a saver would pay that nobody stated, and enough to keep a market from ever crossing. One curve builder for goods and shares alike (`demand.ts`), rungs reaching the opinion, and one line one side so a cell is never on both sides of a book; invariance test added
- [x] Issuance as a primary offer with a size and a reservation; dilution; failure
- [x] Buyback: the firm bids at its own reservation and what it buys is cancelled
- [x] Dividend decided from what the firm has spare over its management's own patience, paid to holders of record, declared publicly
- [x] `desk` party kind inside its bank; desks seeded with the opening float of every line they make a market in
- [x] Desk quotes: two schedules from own state; skew by inventory, width by its own surprise width and the flow it faced, size by remaining limit; no width constant exists (lint)
- [x] Desk rent every period at its bank's cost of funds plus the capital charge; test: a desk carrying inventory for free is unreachable
- [x] Desks in every market they make: bonds and shares; interdealer through the same session
- [x] D4.a: a market whose only liquidity was a desk at its limit fails and prints stale with reason
- [x] D5 as an observer read: inventory, spread, capital usage together
- [x] Split moves no value; the print rebases; test
- [x] Votes as a read (F3, A5) and free float as a read of what is encumbered (C1.b); tests
- [x] Estate: a share ranks last, takes the residual when there is one and is written off at zero when there is not (E4, F2); test
- [x] F4: no income without cash; test: a cell holding shares of a firm that retained earnings shows no income
- [x] From item 8 (Fund Shares E1, E2): an `etf.share` kind whose shares TRADE — a market, a cleared price, and its NAV read beside it from the same book, so it has two values and they are different numbers
- [x] From item 8 (E3, E3.a, G1.a): creation and redemption IN KIND against the basket, by a desk with a reason and a limit — which is why an exchange-traded fund is not a forced seller, and why the gap can persist when nobody will close it
- [x] From item 8 (E4): the premium or discount as a read of the two prices, on the observer; a persistently large one is a finding about liquidity and never a number to clamp
- [x] Year-long run green; determinism; XI-13 test: a desk's schedule is independent of the other orders in the book
- [ ] Coverage re-marked; PARTIAL rows for C1.b, C2.b, C2.c, C2.d, C2.e, C5, C7, A5.a, E1–E3, G1, G2 named; record entry
- [ ] Delete this file; worklist row 9 → done; commit and push

## Exit criteria

Shares clear from opinions in schedules; issuance can fail; desks carry inventory that costs them
rent every period and their spreads come out of their state; a market can fail because the desk
stepped back.

## Guard

Equity B3, B4.a, F4; Dealer Desks B4, C5.a, C5.b, F1, F2, F3; Clearing B4, B5.
