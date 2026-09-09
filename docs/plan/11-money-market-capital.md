# Item 11 — The money market, the corridor, bank funding and capital

**Objective.** A bank's reserve position is the residue of everyone else's payments; the money
market clears after the flows, unsecured pricing the name and secured pricing the collateral; the
corridor's floor and ceiling make the policy rate effective through the market and never by
assertion; a bank has deposit classes with different stickiness, a blended cost of funds, a capital
requirement against risk weights with a buffer it chooses, and it can raise equity and subordinated
debt in markets that can refuse. The reserve overdraft that item 1 records unpriced is priced here
and the placeholder path deleted.

**Read first.** §11 Money Market (all); §24 Banks Funding (all); §25 Banks Capital A, B; §31 Central
Bank B, D; §22 D3 (the benchmark: 12); XI-4 joint one; Money B3.b, B3.c, C4.b. Code: `ledger/
settlement.ts` (reserve overdraft path), `registry/profiles.ts` (central bank profile), items 6, 9.

**Clauses this item meets.** Money Market A1, A1.a, A1.b, A2, A2.a, A2.b, A3, A3.a, B1, B2, B2.a,
B2.b, B3, B3.a, B3.b, B3.c, B4, B5, B5.a, B6, B6.a, B7, C1, C1.a, C2, C3, C4, C4.a, C4.b, C5, D1,
D2, D3, D4, D5, D5.a, D5.b, D6, E1, E2, E3; Banks Funding A1, A1.a–d, A2, A2.a, A3, A4, A5, B1,
B1.a, B1.b, B2, B2.a, B2.b, B3, C1, C1.a, C2, C2.a, C3, C3.a, C4, D1–D6, D6.a, E1–E5, F1–F4; Banks
Capital A3, B1, B1.a, B1.b, B1.c, B2, B3, B3.a, C2, C2.a, C2.b; Central Bank B1, B1.a, B2, B3, B3.a,
B4, D1, D2, D3, D3.a, D3.b, D4; Money B3.b, B3.c (fully); Banks Lending B2.b, C1.a (fully).

---

## Design

### Module `money-market`

- `requires: ['bank-lending', 'treasury']`.
- **Kinds.** `interbank.loan` (unsecured: a `loan`-like row between two banks, overnight or term:
  B6; `defaultOn` missed repayment), `repo` (secured: a row with collateral pledged through the
  register's liens: B3, B3.c; eligibility per instrument kind is data: B3.a; haircut per (kind,
  issuer's credit: the lender's own PD of the issuer) B3.b: no haircut table per kind alone).
- **Deposit classes** (Banks Funding A1): the money instrument stays one kind, but the holder's
  relationship gains a class read from the holder's party kind and size: retail (household cells,
  insured per member up to the limit: A1.a), corporate (firms), wholesale (funds, insurers, other
  banks); stickiness is not a parameter: it is the holders' own decisions (E1, E2: the wholesale
  depositor's participant leaves when it observes weakness: a published ratio, a facility draw
  (public event from item 1), a rate paid up, a run of short closes: all public journal events).
- **The deposit rate** (B1.a): each bank sets a rate per class as a decision from its own funding
  need and the alternatives its depositors have (the money fund's yield and the bill yield from the
  curve: the rate is bounded above by the cheaper of its wholesale cost and the fund's yield on the
  contested share: a consequence of the decision, not a clamp: the decision function reads those and
  chooses; the lint sees no `min`); interest paid by an instruction each period (B1: real money to
  the holder).
- **Blended cost of funds** (B2): a read across the mix; feeds item 6's quote (item 6's placeholder
  deleted).
- **The session** (A3, B): phase `moneyMarket.clear` (cycle 3, after every other market and after
  `firms.pay` and `households.pay`): every bank's participant posts a schedule from its position
  (its reserve balance after the flows, read from its view), its buffer preference (A2.a: derived
  from its deposit classes' observed outflow variance: a read of its own history, no ratio) and its
  cost of funds; lenders price the borrower's name (B2: their own PD of that bank) and the collateral
  (B3); non-banks with cash (money funds, firms, insurers) are in the same market (B5) with the
  floor as their alternative (B5.a). Two books: overnight and term (B6). The solver clears each; a
  name that finds no bid at any level is a refusal (B2.a, B7) recorded as a market outcome per name
  (the module journals `moneyMarket.refused` public).
- **The corridor** (C, Central Bank B, D): the central bank's participant posts the **floor** (a bid
  for any amount at the floor rate: an administered rate with a real quantity response: reserves
  parked leave the system: C1.a: a transfer to the central bank's own account, i.e. destruction of
  reserves, journaled) and the **ceiling** (the standing facility: an offer to lend at the ceiling
  rate, collateralised (C4) and bounded by each borrower's unencumbered eligible paper (C4.b): the
  facility is a repo row with the central bank as lender). The policy rate is the central bank's
  decision (B1: on its mandate; the rule is a decision from its own outlook of inflation and
  activity at 12/16; until then a stated policy owned by the central bank); it never appears as a
  cleared rate (B3.a).
- **The reserve overdraft** (Money B3.b, Central Bank D3.b): a bank whose reserve account is below
  zero after the session and the window is **in default of payment**: the kernel's recorded-but-
  unpriced overdraft path is deleted; the central bank's overdraft decider (item 6's door) now
  refuses when the bank has no eligible collateral, and prices any allowed overdraft at the window
  rate plus a penalty as a repo row created by the decider. D6: the lender of last resort lends
  freely, against good collateral, at a penalty, to the solvent; it does not lend to an insolvent
  bank (D3.a): the decider reads the bank's capital (below) and refuses.
- **When a name cannot fund** (D): the bank's phase `banks.funding` (cycle 0): it sells liquid
  assets (D1: orders in the next session), bids up for deposits (D2), stops originating (D1: its
  credit decision reads its own funding state at item 6's constraint B2.b, now real), draws the
  facility (D3), or fails for liquidity (D4: distinct from solvency: `banks-capital` resolution reads
  which trigger fired).
- **The run** (D5, Banks Funding E): wholesale depositors' participants observe public events and
  move deposits (an instruction to another bank or to a money fund); the loop (E3.a) is the register.

### Module `banks-capital` (second half)

- **Capital** (A1–A4 from item 7; here A3): the bank raises equity through item 9's issuance and
  subordinated debt through a bond issuance (a `bond` kind with a subordinated rank: 13f's corporate
  kinds; here a `bank.subordinated` instrument kind owned by this module, `ranking` below senior);
  both priced by markets that can refuse (C2.b).
- **Requirements** (B1): risk-weighted assets from the register (weights per kind and security:
  policy from item 6) against the ratio; the leverage backstop (B1.b); which binds is a read (B1.c).
- **The buffer** (B2): the bank's choice (preference), and breaching it restricts distributions (B3:
  the dividend decision reads it), demands a plan (journaled), intensifies supervision (a public
  event the depositors' participants read: E2.a).
- **Reports** (Banks Funding F): the observer reads deposits by class, reserves, the liquidity
  metric (F4: a published ratio with a lag: Observer A5).

### Parameters

`centralBank.corridor.floorSpread`, `centralBank.corridor.ceilingSpread`, `centralBank.
overdraftPenalty` (policy, centralBank); `regulation.leverageRatio` (policy, parliament);
`regulation.depositInsurance.limit` exists; `repo.eligible.<kind>` (policy, centralBank: data);
`bank.buffer.preference` **deleted** (item 3's `bank.liquidityBuffer.perDeposit` placeholder dies:
the buffer is now derived from observed outflow variance); `bank.costOfFunds.placeholder` deleted;
`centralBank.policyRate` stays (a POLICY primitive, now effective through the corridor).

### Audit contributions

- `money`: B3.c's overdraft check now expects a repo row behind every negative reserve balance: the
  kernel family is updated (the placeholder message deleted).
- `flows`: Banks Funding F3 is the kernel's accounts family; B2.b (one rate per liability): the
  interest instruction and the cost-of-funds read use the same rate: a unit test, not a check
  against itself.
- `prices`: Money Market C3: the market rate inside the corridor is a VERIFY read at 16.

### Files

```
packages/engine/src/ledger/settlement.ts, registry/profiles.ts (delete the recorded-unpriced path)
packages/engine/src/audit/families/money.ts (overdraft with a lender row)
packages/engine/src/mechanisms/money-market/{index.ts,session.ts,corridor.ts,repo.ts,deposits.ts,funding.ts}
packages/engine/src/mechanisms/banks-capital/{capital.ts,requirements.ts,raise.ts}
packages/engine/src/mechanisms/bank-lending/quote.ts (blended cost of funds)
packages/engine/test/{money-market,corridor,repo,deposit-classes,run,capital,raise}.test.ts
```

---

## Steps

- [ ] `interbank.loan` and `repo` kinds; repo collateral through liens; eligibility as data; haircut from the lender's own PD of the issuer (B3.b); tests
- [ ] Deposit classes as a read of the holder; per-member insurance read; tests
- [ ] The deposit rate as each bank's decision reading its need and its depositors' alternatives; interest paid by instruction; test: no `min` exists and the rate still never exceeds the cheaper alternative on the contested share in a year of runs (a VERIFY-style assertion, recorded as a finding if it fails, not a clamp)
- [ ] Blended cost of funds as a read; item 6's placeholder deleted; test: two banks with different mixes quote loans differently
- [ ] The session after the flows: schedules from position, buffer (derived from observed outflow variance), cost of funds; unsecured prices the name; two books; refusal per name; tests
- [ ] Non-bank cash in the same session with the floor as its alternative (B5); test
- [ ] The corridor: floor as parking that destroys reserves, ceiling as a collateralised repo bounded by unencumbered eligible paper; the policy rate never a cleared rate; tests
- [ ] The reserve overdraft priced and collateralised through the decider; the kernel's recorded-unpriced path deleted; the money family updated; tests: a bank with no collateral cannot draw (C4.b) and fails for liquidity (D4)
- [ ] LOLR's four conditions (D6): freely, good collateral, penalty, solvent: the decider refuses an insolvent bank; test
- [ ] `banks.funding`: sell liquid assets, bid up deposits, stop originating (item 6's B2.b constraint now real), draw the facility, fail; tests for each branch as a state reached
- [ ] The run: wholesale depositors move on public observables; the loop shows in a scenario test (D5.b, E3.a); insurance breaks it for retail (E4); tests
- [ ] Interbank exposure as contagion: a failed bank's interbank rows land losses on lenders by name (E3); test with item 7's resolution
- [ ] Capital: requirements against risk weights, leverage backstop, which binds as a read; buffer as choice; distributions restricted near the line; tests
- [ ] Raising: equity issuance (item 9) and subordinated debt (`bank.subordinated` kind) into markets that can refuse; test: a failed raise leaves the bank where it was
- [ ] Reports: deposits by class, reserves as one row, liquidity metric published with a lag; observer
- [ ] Central bank: policy rate as a decision on its mandate (a stated rule reading its own outlook; the mandate text and target are parliament's at 14); test: a change moves the market rate through the corridor, never by assignment (B4)
- [ ] Money B3.b, B3.c and Banks Lending B2.b, C1.a re-marked MET; item 3's buffer placeholder and item 6's cost-of-funds placeholder deleted
- [ ] Audit contributions; observer: corridor, session prints per book, refusals, facility draws, runs
- [ ] Year-long run green with a scenario seed of a funding squeeze; determinism
- [ ] Coverage re-marked; record entry
- [ ] Delete this file; worklist row 11 → done
- [ ] Commit and push

## Exit criteria

The policy rate reaches every loan quote only through the session and the corridor; a bank can fail
for liquidity separately from solvency; a run is reachable and self-reinforcing; two placeholders
deleted; a year green.

## Guard

Money Market C5, D6; Banks Funding B1.c, D6.a, B2.b; Central Bank B3.a, D3.a; Banks Capital A1.a.
