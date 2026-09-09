# Item 5 — A loss is an event

**Objective.** A claim that goes unpaid becomes a dated event with a named borrower, a status that is
written, a provision that is booked, a write-off that is settled, and a recovery that is what
something fetched. Never a rate. This is the first of the seventeen mechanisms in dependency order
because at least four other systems need "a claim goes unpaid" to be a thing that happens (XI-1).

**Read first.** XI-1 in full; §23 Banks Lending E (all), C4; §32 Firm D; §34 Firm Birth B, C; §7
Corporate Credit G1–G3, G6; §8 Sovereign G; Bond N12, N13, N13.a; Money E1; Register E3, B4.
Code: `ledger/instruction.ts` (Failed records), `world/actions.ts` (coupon and maturity
instructions), `registry/kinds.ts`.

**Clauses this item meets.** Money E1, E1.a, E1.b (fully); Register E3, B4 (fully); Bond N12, N13,
N13.a; Corporate Credit G1, G2, G3, G6 (for the instruments that exist: sovereign paper and, from
item 6, loans); Sovereign G2, G3 (the state; the exchange offer is 13f); Firm D4, D5; Firm Birth C1,
C2, C2.a, C3, C4; Banks Lending E1, E2, E5, E5.a (loans exist from item 6, so E is met at 6 using
this item's vocabulary; this item meets it for sovereign paper); XI-1.

---

## Design

### The vocabulary the kernel gains (sub-item 5.1, kernel door)

- **Default definition on the instrument kind profile**: `profile.defaultOn(instrument, failed:
Failed): DefaultEvent | null`. The kernel's corporate-actions phase, after settling a coupon or
  maturity instruction that **failed** (overdraft refused, insufficient units), asks the profile
  whether that failure is a default under the instrument's own definition (Bond N12: observable by a
  holder). A sovereign bond: a missed payment is a default (Sovereign B5). A loan (item 6): a missed
  payment or a breached covenant. The kernel journals `credit.default` (public: Firm Birth C3, an
  event others react to) with `{ issuer, instrument, holder, amountDue, date }`.
- **Instrument status**: `Instrument.status` gains `{ live: true, performing: boolean }` written only
  by the kernel on a default event, and read by everyone (Banks Lending E2: a status no path writes
  is not a status).
- **Claim ranking as a term**: `profile.ranking(instrument): { seniority: number, secured: readonly
{ instrument, qty }[] }` (N13.a: stated even when "all equal"); the estate (item 7) honours it.

### `credit-events` module (sub-item 5.2)

- `requires: []`; owns no kinds.
- Phase `credit.events` (cycle 0, after `corporateActions`): reads this period's `credit.default`
  events and the failed records behind them; for each holder of a defaulted instrument, books a
  **provision**: an equity move on the holder equal to its own expected loss on the claim (Banks
  Lending D2: a charge to income when the assessment moves). The holder's expected loss is its own
  assessment: until ratings (12) and the holder's own model (10), the assessment is the holder's
  outlook of recovery (`outlook('credit.recovery.<issuer>')`, formed from what it has observed of
  recoveries: none at first, so the first provision is the whole claim: honest, and it unwinds as
  the estate pays). Provisions are journaled `credit.provision` (private to the holder).
- The provision is an equity event, not a register move (D2.b: never a reserve that quietly absorbs);
  the accounts family must still hold: the kernel's accounts family compares equity to assets minus
  liabilities at **marks**; a defaulted instrument's mark is its recovery expectation... Resolution:
  a defaulted instrument's kind profile switches its pricing to `carriedAtCost` with the provision
  as a write-down through `profile.revalue` (item 4.1's door): the holder's lot basis is written
  down to (basis − provision) and equity moves by the same amount, so the read and the account agree.
  For sovereign paper that keeps trading after a default (a distressed market exists), the mark is
  the print and the provision is the mark change: no separate booking. The rule: **a provision is a
  write-down of the lot to the holder's expected recovery, booked once per assessment change.**
- **Write-off** (E5): when the estate closes (item 7) or, before the estate exists, when the
  instrument's profile says the claim is extinguished (a sovereign missed payment is not a
  write-off: Sovereign G is negotiated; so before item 7 nothing writes off), the remaining lot is
  destroyed by an asset leg from the holder to the issuer (a redemption at zero price) and the loss
  reaches equity as the difference (E5.a: principal minus recovery minus provisions already taken,
  which is exactly what the carrying value then is).
- **Acceleration** (Corporate Credit G2): a default on one instrument of an issuer makes every
  instrument of that issuer whose profile declares cross-default due: journaled, and the maturity
  action is placed this period by the kernel reading the profile's `accelerates`.

### The Failed record becomes readable (sub-item 5.3)

`Failed` gains `readonly due: { payee, amount, ccy }` for money legs so the payee's receivable that
did not arrive (Money E1.b) is a read of the ledger; `credit-events` exposes nothing else. Firms
(item 4) read their failed payments in `firms.decide` (Firm D4, D5: a firm that cannot pay defaults;
its cash failure is the trigger, read from its own failed instructions in its view: add
`view.failedPayments(last)` as a door, private to the party).

### Parameters

None new. (The provision reads an outlook; the outlook's memory is item 4's preference.)

### Audit contributions

- `accounts`: none new (the write-down design keeps the existing identity exact).
- `names`: every `credit.default` event names an issuer that exists and an instrument it issued.
- `flows`: a write-off is a redemption leg at zero price; the ledger shows it; Banks Lending E5.a:
  loss to equity equals carrying value written off (a per-event check: contributor `credit-events`).

### Files

```
packages/engine/src/registry/kinds.ts            (defaultOn, ranking, accelerates, performing)
packages/engine/src/register/instruments.ts      (performing status, one writer)
packages/engine/src/world/actions.ts             (asks the profile after a failed coupon/maturity)
packages/engine/src/ledger/instruction.ts        (Failed.due)
packages/engine/src/world/context.ts             (view.failedPayments)
packages/engine/src/mechanisms/sovereign-instruments/index.ts (defaultOn, ranking: pari passu)
packages/engine/src/mechanisms/credit-events/index.ts
packages/engine/test/{default-events,provision,writeoff}.test.ts
```

---

## Steps

- [ ] 5.1 `profile.defaultOn`, `profile.ranking`, `profile.accelerates`; `performing` on instrument status with the kernel as its one writer; tests
- [ ] 5.1 Corporate actions ask the profile after a failed coupon or maturity and journal `credit.default` publicly; test: an empty treasury account produces a default event on the line, not a silent skip
- [ ] 5.1 Sovereign profiles: missed payment only (B5), pari passu (B4), no acceleration (B6); tests
- [ ] 5.2 `credit-events` module: provisions as lot write-downs to the holder's own recovery outlook, booked once per assessment change; accounts family holds; tests
- [ ] 5.2 Provision unwinds as recoveries arrive (an upward re-assessment on a defaulted claim is allowed: it is a claim, not inventory); test
- [ ] 5.2 Write-off as a redemption at zero price when the claim is extinguished; E5.a identity; test
- [ ] 5.2 Acceleration for kinds that declare it (none yet: the corporate kind at 13f); the door exists and is tested with a test-only kind
- [ ] 5.3 `Failed.due` and `view.failedPayments`; firms read their own failed payments and default on cash failure (Firm D4, D5); test
- [ ] Households: a cell that cannot pay a tax or a purchase has a failed instruction; the cell's default is a split of the members who crossed (XI-15; Households E4 arrives at 13d with consumer credit, so here only the state and the split)
- [ ] Observer: defaults, provisions (own scope), write-offs; instrument status shown
- [ ] Year-long run with a forced treasury shortfall scenario in a test (a seed variant with an empty buffer) producing a default event and a provisioned holder book, families green
- [ ] Coverage re-marked; PARTIAL rows for Sovereign G4/G5 (13f), Banks Lending E (6) named
- [ ] Record entry
- [ ] Delete this file; worklist row 5 → done; commit and push

## Exit criteria

A missed payment on any instrument produces a public default event, a written status, a holder-side
provision that keeps the accounts family exact, and (where the profile says so) a write-off with the
E5.a identity; no rate anywhere.

## Guard

XI-1 (no loss rate; no default drawn; population-level default as cell crossings); Banks Lending
D2.b; Firm Birth C2.a; Register A3 (a write-off is a redemption, never a leg to nobody).
