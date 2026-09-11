# Item 13a — The derivative layer

**Objective.** The infrastructure every derivative class runs on: a contract that is a bilateral
obligation between two named parties (never a holding), whose mark is one number read from two
sides and sums to zero across the world; collateral and margin that are held, not consumed; a
central counterparty that is a real party with a default fund, its own capital and a stated
waterfall it can run past the end of; capacity that refuses a contract a member could not
re-margin; default that closes out at a stated value and lands the loss on named survivors. No
class is built here; a test-only contract kind on a bond price exercises every door so 13b can add
classes as modules without touching the kernel again.

**Read first.** Part IV, The Derivative (D1–D12, X1–X3); §16 Derivative Layer (all); XI-2 door 1
(the margin call); Register D (liens); Money E1 (the payment that fails); Firm Birth D2.c; §45 B1.
Code: `register/register.ts` (liens), `ledger/settlement.ts`, `audit/families/zeroSum.ts` (reports
"not built"), item 7's estate and close-out ranking, item 5's `Failed`.

**Clauses this item meets.** Derivative contract D1, D1.a, D1.b, D2, D2.a, D3, D3.a, D4, D5, D6,
D6.a, D7, D7.a, D7.b, D8, D8.a, D9, D9.a, D10, D10.a, D11, D11.a, D12, X1, X2, X3; Derivative Layer
A1–A4, B1, B2, B3, B3.a, B4, C1, C1.a, C2, C2.a, C3, C3.a, C3.b, C4, C4.a, C4.b, C4.c, C4.d, C5,
D1, D2, D2.a, D2.b, D2.c, D3, D4, D4.a, D5, E1, E2, E3, E4, F1–F4, G1–G4; XI-2 door 1; Firm Birth
D2.c. The zero-sum audit family is built here.

---

## Findings this item carries

Both are one thing: **a balance reached by arithmetic is charged the dust of its answer instead of
the dust of its terms** (Law 7). This item is where it has to be fixed, because `D1.b` asks that the
marks across a contract sum to zero **exactly** — a family whose subject is an exact identity cannot
be built on a door that mis-states what an identity costs.

### A stated balance is charged the dust of its answer, not of its terms (`12d-21`)

`opened(value)` (`core/num.ts:81`) returns `dust: moveDust(v, 0)` = `ε × |v|`: the rounding of
stating the number itself. That is right for a balance somebody stated. It is wrong for a balance
somebody **computed**, because Law 7's dust is `terms × ε × Σ|magnitudes|` — of the terms that
produced it, never of the answer.

Where it bites: a fund's equity is zero by construction (Fund Shares A3), and that zero is a
contribution of ~4e11 minus a book of ~4e11. `opened(0)` charges it `ε × 0` and the seed audit's
accounts family then reports the real residue — measured at **0.00008869**, which is `ε × 4e11` to
the digit — as a violation. Sub-cent, but not dust by the derivation being used, and widening
anything is forbidden.

The fix is a door that takes the terms rather than the total, so a balance reached by subtraction
opens with the dust its subtraction earned. An attempt at it during 12d touched `opened`,
`stateEquityAsRead` (`world/assemble.ts`) and the equity read together, did not clear the failure
and carried a lint error; it was reverted whole (Law 13 — a change wrong on its own terms). The
finding stands and the cause is stated above. Seen at `packages/engine/src/core/num.ts:81`, seed
audit at period zero via the ETF launch.

### The trading-book check's dust counts its own terms and not the other side's (`12b-5`)

`mechanisms/banks/index.ts`, `tradingBookIsCapitalised`, contributing to `accounts`. From period 41
of a 52-period run, every period:

```
bank.a: its dealing book weighs 50408462604.28294 and it published 50408462604.2827
```

A gap of 0.000244140625 — 2^-12, pure binary dust — on numbers of 5.0e10. The derivation is short,
not absent: it allows `dustOf(terms.length + 2, |rwa| + |asked|)`, where `terms` are the dealing
lines above target. But `rwa` is the bank's published weighting of its WHOLE book, a sum over far
more terms than this check can see, and Law 7 says the tolerance is what the arithmetic did — so the
terms that went into the other side belong in it. The honest fix is the same door: a published
balance carries the count its own sum had, so a reader can derive the dust of the comparison rather
than guessing at it.

---

## Design

### Sub-item 13a.1 Kernel: the contract store

- A derivative is **not a holding** (X1): it has no issuer and enters no issued-amount check. The
  kernel gains a second register, `register/contracts.ts`: a `Contract` row `{ id, kind, a: PartyId,
b: PartyId, terms, opened: Period, state: 'open' | 'terminated', house: PartyId | null }` where `a`
  and `b` are the two sides and, for a cleared contract, both rows face the house (C2: one trade
  becomes two contracts member↔house, and the house is flat by construction because it holds both).
  Identity is D12: counterparties + underlying + term + strike; two contracts on one underlying with
  different strikes are two rows; an offsetting trade with a different counterparty is a third row
  (B3.a), never a collapse.
- Doors on `MechanismContext`: `contracts.open(kind, a, b, terms, price)` (a trade that cleared),
  `contracts.close(id, value)` (offset, expiry, early termination at a stated close-out D11.a),
  `contracts.novate(id, from, to)` (B4: journaled with the old counterparty's consent as a decision
  the profile asks its participant). Reads on `ParticipantView`: `contracts.mine()` (own side only:
  Observer A4), `contracts.exposureTo(counterparty)` (netted per pair: C1.a), never a read across
  counterparties netted (G3: no such door exists).
- A **derivative kind profile** (`DerivativeKindProfile`, registered like instrument kinds): `underlying:
(terms) => PrintRef | EventRef` (D3: a print `(market, instrument)` this world clears, or a named
  party's credit event this world records; assembly throws `InvalidRegistry` for anything else: G4,
  D3.a), `mark(contract, period, view) => Money` (D8: from the side of `a`; `b`'s is the negation:
  one number, two reads: A3), `legs(contract, period) => Leg[]` (D4, D6.a: the periodic payments
  due this period, both directions, currency per leg: D5), `initialMargin(contract, view)` (below),
  `closeOut(contract, period) => Money` (D11.a).
- **Valuation**: `value(party, contract)` = `±mark`; the accounts family adds each party's contract
  marks to its assets or liabilities (D1: asset to one, liability to the other, every instant).
- **The zero-sum family** (`audit/families/zeroSum.ts`, built here): per contract `mark(a) + mark(b)
= 0` exactly (D1.b: not dust: the same number negated); in aggregate; variation margin paid = received
  per period from the ledger (D2.b). Independence: a contract defect lights this family and no other.

### Module `derivative-layer`

- `requires: ['credit-events', 'estate', 'money-market']`.
- **Party kind** `clearingHouse` (C2, C3): a named party; `moneyIssuer: null`; its balance sheet is
  the margin it holds (its liability: below), the default fund (its liability to members), and its
  own capital (equity: the residual, a read: C3).
- **Instrument kinds** (owned here): `margin.claim` (issued by the receiver of cash margin, a house or
  a bilateral counterparty; `liabilityOfIssuer: true`; `pricing: 'money'`-like at par in its
  currency: it is a claim to the return of cash; a member's asset: C3.a: posting margin is an asset
  swap: money out, claim in, one instruction, two legs) and `defaultFund.contribution` (issued by the
  house to each member; `liabilityOfIssuer: true`; written down by the waterfall: C4). Securities
  posted as collateral stay the poster's holding with a **lien** in favour of the receiver (Register
  D; D9.a: they leave the free balance and cannot move); the kernel's existing encumbrance check
  covers Sec Lending C4.
- **Initial margin** (D1): `initialMargin = measuredMove(underlying, view) × notional × √(remainingLife
/ horizon)` where `measuredMove` is the underlying print's own realised move over the close-out
  horizon read from the price store's history (a read, not a rate per class: D1's FORBID by
  construction: no parameter `margin.rate.<kind>` exists); the close-out horizon is a policy of the
  house (`clearingHouse.closeOutHorizon`, periods, policy, standardSetter). Bilateral agreements use
  the same read with the receiver's own horizon preference.
- **Variation margin** (D2): phase `margin.calls` (cycle 0, order 4, already in the canonical
  period): for every open contract, the change in the mark since last close is an instruction from
  the losing side to the winning side (bilateral) or member→house and house→member (cleared: two
  instructions, the house flat: C2) in cash in the contract's currency (D2.a). It goes through
  settlement like any payment: a party short of cash **fails** the instruction (D2.c: the recorded
  `Failed` state of item 5, never a borrowing), and the failure is a missed payment → `credit.events`
  next period → close-out (below).
- **Calls and close-out** (D4): an unmet variation call or a margin shortfall after the initial
  margin is re-measured against the last close is a **call**: journaled `margin.call` (public: §45
  B1); the party's own `decide` phase (its module) responds by posting cash, pledging securities, or
  posting sell orders with reason `forced.sale` (XI-2 door 1; D4.a: the same channel as a
  redemption); a call unmet after the session closes the position at the stated close-out value in
  `resolution` (D4, D11.a), journaled.
- **Default fund** (C3.b): sized cover-one: the largest member's initial margin × √(closeOutHorizon)
  re-measured each period; contributions pro rata to each member's margin, trued up by an
  instruction each period; a member that leaves (no open contracts) is refunded.
- **The waterfall** (C4, in `resolution` after item 7's estate opens for the defaulter): the
  defaulter's net obligation to the house across all its contracts at that house (C4.b) is met in
  order from: its margin (its `margin.claim` redeemed against the loss), its fund contribution
  (written down), the house's own capital (its equity falls), the survivors' contributions pro rata
  (C4.a: each survivor books the write-down against equity: a real loss on a real sheet); what is
  left is the house's **unsecured claim on the estate** (C4.c: a row in item 7's waterfall ranking
  with other unsecured claims: Firm Birth D2.c); every round is journaled `waterfall.round` with
  who, the loss, what each line paid, what stayed unfunded (C4.d). **Past the end** (C5): the house
  has an unfunded loss; its equity is negative; XI-3: it fails into an estate; survivors' claims on
  it are claims on that estate. Nothing tops it up.
- **Survivors** of a defaulted contract are paid in full by the house and get their margin back
  (C4.b); bilaterally, the survivor's claim is mark minus collateral held, on the estate (F2, F3).
- **Capacity** (E): a clearing member's admitted share is read once per period from its own liquid
  cash net of what it has already committed (E1: `view.cash` minus margin posted this period minus
  its buffer preference; a read, not a limit parameter), drawn down as it is consumed (E3). In the
  `markets` phase a derivative market's cleared trade is **cut to the smaller of the two members'
  remaining shares** at the strike, in the same pass as the contract and its initial-margin
  instruction (E2: the clearing runner for derivative markets is a `MarketDecl` variant `{ kind:
'contract' }` that admits size against capacity before writing the row); the remainder is a
  **refusal** journaled `derivatives.refused` with the sizes (E4: the standing observation "refusals"
  of Part XII). The limit is never raised by a mechanism.
- **Bilateral exposure**: `exposureTo(counterparty)` nets marks across the contracts the two have
  with each other (C1, C1.a) less collateral held; every reservation in 13b's classes carries a term
  for the counterparty from this read and the party's own PD of it (D10.a).
- **Test-only kind** `test.forward` (in `packages/engine/test/support/`): a forward on a bond print
  with a cash-settled payoff, used by every test here; deleted when 13b's first class replaces it in
  the year-long run.

### Parameters

`clearingHouse.closeOutHorizon` (policy, standardSetter); `party.marginBuffer` (preference per
member, dispersed); `clearingHouse.defaultFund.coverage` is **not** a parameter (cover-one is the
rule: C3.b); no margin rate per class exists (D1).

### Audit contributions

- `zeroSum` (built here): per-contract and aggregate marks; VM paid = received.
- `accounts`: contract marks in assets/liabilities; `margin.claim` and `defaultFund.contribution`
  as ordinary instruments; the house's equity is its residual.
- `ownership`: collateral liens: encumbered units never move (kernel).
- `names`: every contract's two parties exist or have a successor (F4 traceability: the loss chain
  is the journal's `waterfall.round` and `closeOut` events).

### Files

```
packages/engine/src/register/contracts.ts, registry/derivatives.ts (13a.1)
packages/engine/src/world/context.ts (contract doors and reads), world/world.ts (contract store)
packages/engine/src/clearing/contractMarket.ts (admission against capacity at the strike)
packages/engine/src/audit/families/zeroSum.ts
packages/engine/src/mechanisms/derivative-layer/{index.ts,margin.ts,house.ts,waterfall.ts,capacity.ts,collateral.ts}
packages/engine/test/{contracts,zero-sum,margin,clearing-house,waterfall,capacity,novation}.test.ts
packages/engine/test/support/testForward.ts
```

---

## Steps

- [ ] A balance carries the dust of its TERMS: a door that opens a computed balance with what its arithmetic did, and a published balance that carries its own term count so the reader of it can too; the seed audit's residue at the ETF launch and the trading-book check both derive rather than widen; tests (Law 7; `12d-21`, `12b-5`)
- [ ] 13a.1 Kernel: the contract store with two sides, identity, `open/close/novate` doors, own-side reads, per-pair exposure read and no cross-counterparty read; `DerivativeKindProfile`; tests
- [ ] 13a.1 Kernel: valuation of a contract as a signed mark; the accounts family carries it as asset and liability; the `zeroSum` family built (per contract exact, aggregate, VM paid = received); tests: one contract defect lights one family
- [ ] Underlying must be a print this world clears or an event it records: assembly throws otherwise (G4, D3.a); test
- [ ] Collateral: `margin.claim` as an asset swap; securities collateral as a lien; posted collateral leaves the free balance and cannot move; tests (D9.a, D3)
- [ ] `clearingHouse` party kind; default fund contributions sized cover-one from the measured move and the horizon, trued up each period, refunded on leaving; the house's capital as a residual read; tests
- [ ] Cleared trades: one trade becomes two contracts against the house; every leg written twice; the house flat by construction; test: no member ever pays another
- [ ] Bilateral: exposure netted per pair less collateral; no door nets across counterparties (G3); tests
- [ ] Initial margin from the underlying's own measured move × notional × remaining life; no rate per class exists (lint + test)
- [ ] Variation margin in cash each period through settlement; paid = received; a party short of cash fails the instruction and is in item 5's state, never a borrowing (D2.c); tests
- [ ] `margin.calls` phase: calls journaled; the called party's own module responds (cash, pledge, forced sale with reason); an unmet call closes out at the stated value in `resolution`; tests (D4, D4.a, XI-2 door 1)
- [ ] Capacity: admitted share read from own liquid cash net of committed; contracts cut at the strike in the same pass as the margin leg; refusals journaled and measurable; tests (E1–E4)
- [ ] Default and the waterfall: defaulter's net obligation, margin, fund, house capital, survivors pro rata with equity write-downs, unsecured claim on the estate ranking with others, every round journaled; past the end the house fails; survivors paid in full; tests with item 7's estate (C4, C5, F1–F4)
- [ ] Novation with consent; offsetting trades never collapse (B3.a); tests
- [ ] Observer: contracts by party (own side), marks, margin, capacity, refusals, waterfall rounds; year-long run green with the test-only forward; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 13a → done; commit and push

## Exit criteria

A contract exists on two books as one row; marks sum to zero exactly and the family says so; a
margin call can force a sale and an unmet one closes out; a member can lose money because another
member failed; the house can run past its waterfall and fail; a market refuses what its members
cannot margin.

## Guard

Derivative contract D1.a, X1, X2, X3; Derivative Layer C5, D2.c, G1–G4, E4 ("measure; do not
raise the limit"); Firm Birth D2.c.
