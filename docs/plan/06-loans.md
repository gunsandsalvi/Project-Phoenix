# Item 6 — Loans are rows

**Objective.** A loan is an instrument, one row per (lender, borrower), written by creating a deposit,
priced from the bank's own cost of funds, its own view of the borrower's expected loss, the capital
the loan consumes and an operating cost; the borrower shops and can refuse; the bank can decline and
the declined volume is visible. A loan that is a row can be transferred, pooled, pledged and matured
(XI-11's prerequisite). The bank's refused customer overdraft becomes a credit decision.

**Read first.** §23 Banks Lending (all), §24 Banks Funding B (the cost of funds), §7 Corporate
Credit C9 (facilities), §32 Firm E4, §36 Trade Credit B4, XI-4 joint one, XI-11 prerequisites,
Money B3.a, B3.c. Code: `registry/profiles.ts` (the bank's overdraft decision), `ledger/settlement.ts`
(precheck), `world/context.ts`.

**Clauses this item meets.** Banks Lending A1, A1.a, A1.b, A2, A3, A3.a, A3.b, A4, A5, B1, B1.a,
B1.b, B1.c, B2, B2.a, B2.c, B2.d, C1, C1.a (with a placeholder: the blended cost of funds is item
11's; here the deposit rate is zero and wholesale absent so the cost of funds is the central bank's
corridor rate placeholder), C1.b, C1.c, C1.d, C2, C2.a, C3, C3.a, C4, D1, D2, D2.a, D2.b, D3, D4,
D4.a, D5, E1–E6, F1, F1.a, F2, F3; Money B3.a, B3.c; Corporate Credit C9; Firm E4 (the funding
choice between retained cash and debt; equity at 9); Trade Credit B4 (bank credit second). Remain
PARTIAL: B2.b liquidity constraint (11), C1.a blended cost (11), syndication D4.a (13f).

---

## Design

### Module `bank-lending`

- `requires: ['credit-events', 'firms', 'households']`.
- **Kinds.** `loan` (pricing `carriedAtCost`, `liabilityOfIssuer: true`, unit `par` in the loan's
  currency, terms `{ principal, rate: Rate | { margin, reference }, maturity, amortisation:
'bullet' | { periodicity, schedule }, security: readonly { instrument, qty }[] | 'unsecured',
covenants: Covenant[], drawdown: 'term' | { facility: { limit, commitmentFeeRate } } }`,
  `due`: interest by day count on the outstanding, principal per the schedule; `defaultOn`: missed
  payment or breached covenant (A5, E1); `ranking`: by security then seniority). The instrument id
  is `loan:<lender>:<borrower>:<n>`; display name `issuer + margin + maturity` (Law 9).
- **Party kind profile change (kernel sub-item 6.1).** `PartyKindProfile.moneyIssuer.overdraft` is
  replaced by a **module-supplied decision**: the profile declares `overdraft: 'module'` and the
  module registers a decider through a new door `ctx.registerOverdraftDecider(partyKind, fn)` at
  assembly (a `SystemModule.overdraftDeciders` field). The bank's decider is the credit decision
  below: an overdraft is a facility draw against a line, or a refusal (B3.a).
- **State.** `lines: Map<(lender, borrower), Facility>`; `declined: journal only`.

### The credit decision (C1–C4)

For a borrower request `{ borrower, amount, term, purpose }` (from firms at item 4's `firms.decide`
when their own cash and buffer do not cover the period's outlays; from households at 13d; from the
treasury never: Treasury D3):

1. Each bank of kind `bank` quotes from its own view (C1): `rate = costOfFunds + expectedLoss +
capitalCharge + operatingCost` where
   - `costOfFunds` (C1.a): the bank's own blended cost across its funding mix, read from item 11
     when it exists; until then a placeholder `bank.costOfFunds.placeholder` equal to the corridor
     floor policy rate declared by the central bank at item 3? Item 3 has no corridor. Placeholder:
     the bank's own last money-market print if any, else the declared policy rate placeholder
     (`centralBank.policyRate` policy, owner centralBank, introduced here as a POLICY primitive that
     item 11 makes effective through the corridor; it is not a placeholder, it is the rate the
     central bank sets (Central Bank B1); what is a placeholder is using it _as_ the cost of funds,
     which dies at 11).
   - `expectedLoss` (C1.b): the bank's own probability of default × loss given default, one model
     per borrower (C4): the model is the bank's outlook of that borrower's coverage (from the
     borrower's published expectation and its own history with it: Firm E7 publishes earnings) mapped
     to a default probability by a stated function (a SHAPE declared, `bank.pd.curve`, justified,
     replaced when ratings and history exist at 12), and LGD from the security's last print
     haircut.
   - `capitalCharge` (C1.c): risk weight (policy, owner standardSetter, per instrument kind and
     security) × required ratio (policy) × the bank's required return on capital (preference).
   - `operatingCost` (C1.d): technology, per loan per period.
2. The borrower takes the keenest quote that its own hurdle accepts (C2: its outlook of the return
   on the purpose against the rate); a wide quote loses the volume (C2.a).
3. The winning bank checks its constraints (B2): capital room (B2.a: risk-weighted assets against
   its requirement; the bank's own buffer preference above it), liquidity (B2.b: placeholder until
   11: no liquidity constraint, declared), appetite (B2.c: a per-borrower and per-sector limit, F3,
   preferences). It writes the loan or **declines**, and a decline is journaled `credit.declined`
   (public? C3.a: declined volume is visible to the observer as an aggregate; the event is private to
   the two parties, the count is a published read).
4. Writing the loan (B1): one instruction: asset leg borrower (issuer of the loan) → bank at price 1
   per unit of par (issuance), money leg bank (own account) → borrower's account at the bank
   (creation). No reserve leaves (B1.a); reserves move only when the borrower spends (B1.b), which
   settlement already does.

### Carrying, service, workout (D, E)

- Interest accrues and is paid by the kernel's corporate actions from the loan profile's `due`
  (D3); non-payment is a default event (item 5) and a status (E2).
- Provision (D2): item 5's write-down to the bank's own expected recovery, re-assessed each period
  the bank's outlook moves (D2.a).
- Workout (E3): the bank's phase `lending.workout` (cycle 0, after `credit.events`) decides per
  impaired row: restructure (a new row replaces the old at new terms, the old redeemed), extend, or
  enforce (E4: security realised by an order in the security's market posted by the bank; the
  proceeds redeem the loan at what they fetched). Each is a decision from the bank's view with a
  cost (the difference between the row's carrying value and what each path is expected to bring).
- Write-off (E5) through item 5's mechanism when the workout ends.
- Sale or syndication (D4): a loan row is transferable like any instrument (a trade instruction);
  the syndicate structure of D4.a is 13f.

### Facilities (C9, A3)

A facility is a `loan` row with `drawdown.facility`: the undrawn commitment is a real obligation
consuming capital at a stated weight (A3.a, policy) and earning a commitment fee (Short-Term Debt
B4) paid each period through `due`. A draw is a money leg bank → borrower against the line
(recorded on the same row's outstanding), never a new row per period (C9).

### Overdrafts (Money B3.a)

The bank's overdraft decider: if the holder has a live facility with room, the overdraft is a draw
(allowed, journaled `credit.draw`); otherwise the credit decision runs for the shortfall as a
request; if declined, the payment fails (refused, recorded). The kernel's placeholder refusal is
deleted here.

### Parameters

| id                                               | kind                                         | unit             | owner          |
| ------------------------------------------------ | -------------------------------------------- | ---------------- | -------------- |
| `centralBank.policyRate`                         | policy                                       | per annum        | centralBank    |
| `bank.costOfFunds.placeholder`                   | placeholder (→11)                            | per annum        | model          |
| `bank.pd.curve.*`                                | shape (→12 partly, → the bank's own history) | ratio            | model          |
| `bank.requiredReturnOnCapital`                   | preference (dispersed per bank)              | per annum        | model          |
| `bank.capitalBuffer`                             | preference                                   | ratio            | model          |
| `bank.limit.perBorrower`, `bank.limit.perSector` | preference                                   | ratio of capital | model          |
| `regulation.capitalRatio`                        | policy                                       | ratio            | parliament     |
| `regulation.riskWeight.<kind>`                   | policy                                       | ratio            | standardSetter |
| `loan.operatingCost`                             | technology                                   | PHX per period   | model          |
| `loan.undrawnWeight`                             | policy                                       | ratio            | standardSetter |

### Audit contributions

- `flows` (contributor `bank-lending`): F2: per bank, new lending − amortisation − prepayment −
  write-off = the change in the book, from the ledger; F1.a holds by construction (the book is the
  sum of rows) and is asserted as a read in the observer, not as a check against itself.
- `accounts`: capital consumed is a read (no new check).

### Files

```
packages/engine/src/registry/kinds.ts, world/module.ts, world/assemble.ts (6.1: overdraft deciders)
packages/engine/src/registry/profiles.ts             (bank: overdraft 'module')
packages/engine/src/mechanisms/bank-lending/{index.ts,quote.ts,decide.ts,workout.ts,facility.ts}
packages/engine/src/mechanisms/firms/…               (borrowing requests from the firm's own view)
packages/engine/test/{loans,credit-decision,facility,workout,overdraft}.test.ts
```

---

## Steps

- [ ] 6.1 Kernel: module-supplied overdraft deciders (`SystemModule.overdraftDeciders`); the kernel's placeholder refusal for banks deleted; test: without a decider assembly fails for a bank kind
- [ ] `loan` kind: terms, `due` (interest by day count, principal by schedule), `defaultOn`, `ranking`, display name issuer + margin + maturity; tests
- [ ] The quote (C1): four named terms, each from the bank's own state; `centralBank.policyRate` declared as policy; the cost-of-funds placeholder declared with death 11; tests: two banks with different capital quote differently
- [ ] The borrower shops (C2): keenest quote under its own hurdle; a wide quote loses; test
- [ ] The bank's constraints (B2): capital room, appetite limits per borrower and sector; declined journaled and counted publicly (C3.a); tests
- [ ] Writing the loan creates the deposit (B1) with no reserve movement (B1.a); test asserts no reserve leg
- [ ] Interest and principal through the kernel's corporate actions; a missed payment is a default (item 5); provision as write-down; tests
- [ ] Workout: restructure, extend, enforce as decisions with costs; enforcement posts the security's sale; tests
- [ ] Facilities: undrawn commitment weight and fee; draws against the line; one row per (lender, borrower); test: no facility per period (C9)
- [ ] Overdraft decider: draw against a line or run the credit decision; refusal recorded; test: a customer with no line and a declined request has a failed payment
- [ ] Firms borrow from their own view when cash and buffer fall short (Firm E4; Trade Credit B4 ordering); test
- [ ] One PD model per borrower per bank (C4): the lint gains no rule, the test asserts the quote and the provision read the same outlook
- [ ] Audit: F2 contribution; observer: loan books as sums of rows, declined volume, facilities
- [ ] Year-long run green; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 6 → done; commit and push

## Exit criteria

Every listed clause MET; the kernel's bank overdraft placeholder gone; one new placeholder
(`bank.costOfFunds.placeholder` → 11) and one shape (`bank.pd.curve` → 12) declared; a year green.

## Guard

Banks Lending B1.c (no lending out of deposits), C4, D2.b, F1.a; Money B3.c; XI-4 joint one (no
cost-of-funds term is a defect: the placeholder is declared, not omitted).
