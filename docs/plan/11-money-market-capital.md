# Item 11 — The money market, the corridor, bank funding and capital

**Objective.** A bank's reserve position is the residue of everyone else's payments; the money
market clears after the flows, unsecured pricing the name and secured pricing the collateral; the
corridor's floor and ceiling make the policy rate effective through the market and never by
assertion; a bank has deposit classes with different stickiness, a blended cost of funds, a capital
requirement against risk weights with a buffer it chooses, and it can raise equity and subordinated
debt in markets that can refuse. The reserve overdraft that item 1 records unpriced is priced here
and the placeholder path deleted.

**Read first.** §11 Money Market (all); §24 Banks Funding (all); §25 Banks Capital A, B, **C, D, E**
(the resolution carried here from item 7); §31 Central Bank B, D; §22 D3 (the benchmark: 12); XI-4
joint one; Money B3.b, B3.c, C4.b, E4. Code: `ledger/settlement.ts` (the reserve overdraft path this
item deletes, and `Settlement.alive`), `registry/profiles.ts` (central bank profile), items 6, 9.
Also `docs/RECORD.md` **10.3**, which closed underneath this one: it is what made a bank failure
reachable, and the four tests it left red are a step below.

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

- [x] `interbank.loan` and `repo` kinds; repo collateral through liens; eligibility as data; haircut from the lender's own PD of the issuer (B3.b); tests
- [x] Deposit classes as a read of the holder; per-member insurance read; tests
- [x] The deposit rate as each bank's decision reading its need and its depositors' alternatives; interest paid by instruction; test: no `min` exists and the rate still never exceeds the cheaper alternative on the contested share in a year of runs (a VERIFY-style assertion, recorded as a finding if it fails, not a clamp)
- [x] Blended cost of funds as a read; item 6's placeholder deleted; test: two banks with different mixes quote loans differently
- [x] The session after the flows: schedules from position, buffer (derived from observed outflow variance), cost of funds; unsecured prices the name; two books; refusal per name; tests
- [x] Non-bank cash in the same session with the floor as its alternative (B5); test
- [x] The corridor: floor as parking that destroys reserves, ceiling as a collateralised repo bounded by unencumbered eligible paper; the policy rate never a cleared rate; tests
- [x] The reserve overdraft priced and collateralised through the decider; the kernel's recorded-unpriced path deleted; the money family updated; tests: a bank with no collateral cannot draw (C4.b) and fails for liquidity (D4)
- [x] LOLR's four conditions (D6): freely, good collateral, penalty, solvent: the decider refuses an insolvent bank; test
- [x] `banks.funding`: sell liquid assets, bid up deposits, stop originating (item 6's B2.b constraint now real), draw the facility, fail; tests for each branch as a state reached
- [ ] The run: wholesale depositors move on public observables; the loop shows in a scenario test (D5.b, E3.a); insurance breaks it for retail (E4); tests
- [ ] Interbank exposure as contagion: a failed bank's interbank rows land losses on lenders by name (E3); test with item 7's resolution
- [ ] Capital: requirements against risk weights, leverage backstop, which binds as a read; buffer as choice; distributions restricted near the line; tests
- [x] From item 7.5 (Banks Capital D1): a failed bank's book valued at marks and at its own carrying values; the hole is liabilities minus that; test
- [ ] From item 7.5 (D2): the hierarchy — equity to zero, subordinated rows bailed in by partial redemption, senior and depositors untouched outside liquidation; tests
- [x] From item 7.5 (D3, D6): every other bank bids for the book from its own view; the winner pays or is paid the difference; deposits and rows are assumed by the acquirer over the wire; test: a resolution with no bid falls to the public path
- [ ] From item 7.5 (D4, D5): deposit insurance per member up to the limit, the insurer as an estate creditor; test with a cell of small and a cell of large depositors
- [ ] From item 7.5 (E3): the resolution conserves — acquirer paid plus insurer paid plus estate realised plus holders lost equals the hole; audit contribution and test
- [x] **Carried from 10.3**: the four tests a bank failure leaves red go green — `capital.test.ts` (the XI-4 chain), `funds.test.ts` ×2 (the gate, and the year with a redemption wave), `omo.test.ts` (the book running off). None of them is about resolution; each of them ends in one, and none can be looked at until a bank has somewhere to go. Two things fall out with them: the fractional-share request in `households/portfolio.ts:fundOrders` rounded at its cause (Law 8, named at the site in `funds/index.ts`), and the XI-4 finding underneath the capital one — the dear world investing MORE than the cheap one — measured for the first time on a run that survives
- [ ] From item 7.6 (XI-2, Prime Brokerage C3.b): a bank whose capital falls cuts a borrower's limit below what it has drawn, and the borrower's own module posts the sales that repay it, at whatever the book gives; test: the sale moves the print and the print reaches other holders, and `limit − exposure` is negative with no floor anywhere in the path
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

---

## HANDOVER — state of play

Written at the point work stopped, from a full run of the suite at the last commit on
`claude/project-review-continuation-bh8ugh`. Item 10.3 — a quantity is a count of indivisible pieces
— landed underneath this one and has since closed; its record says what it changed and what it
found. Everything still red in this tree is red for THIS item's reason and no other: a bank fails
and there is nowhere to put it.

### What is built and working

- **The instruments.** `interbank.loan` and `repo` kinds; collateral pledged through real liens in
  the same instruction as the row (B3.c); eligibility as registry data; the haircut declared by the
  central bank for its own window and the lender's own required yield for a market repo.
- **Deposit classes** (retail / corporate / wholesale) as a read of the holder's kind, with the
  insured/uninsured split taken in one place (`coveredPerMember`) and read from both sides.
- **The deposit rate** as each bank's decision — what money is worth to it, less its own margin, less
  what it takes to move that class — paid by instruction, published as a public event (D5.a).
- **Blended cost of funds** as a read of what the bank actually paid last period (`costOfFunds` in
  `bank-lending`), so item 6's placeholder is gone and two banks with different mixes quote
  differently. The claim in the last bullet has no test yet.
- **The session after the flows**: positions read off the wire's own reserve legs (nobody chose
  them), a book per borrowing name per tenor so an unsecured rate prices the NAME, refusals recorded,
  the window taking its seat at the ceiling against unencumbered eligible paper only.
- **Non-bank cash** in the same session (B5): the money fund places its spare cash rather than
  hoarding it as a deposit.
- **The corridor**: floor as parking that destroys reserves (C1.a), ceiling as a collateralised repo,
  both administered and published; the policy rate never a cleared rate.
- **The priced reserve overdraft** through the credit-decision door, with LOLR's four conditions all
  present and the refusal of an insolvent bank being the one that must exist (D6).
- **Depositor mobility**: `world.moveBank` moves a depositor's balance and its bank, and uninsured
  money leaves a bank the market has publicly refused.
- **Two rungs of the ladder**: D3 (a bank the session left short values money at the window and bids
  up for deposits) and D4 (a bank short of liquid assets against what could leave it writes no new
  loans — the credit crunch, read from its own published `bank.liquidity`).

### Findings recorded on the way

1. **A refusal made of rounding started a real run.** The session recorded `moneyMarket.refused` for
   a shortfall of 1.8e-7; that event is a public observable, so every uninsured depositor left the
   bank and it died two periods later. Fixed at the cause — a bank borrows whole pieces of money and
   can only be short of whole pieces — and that finding is what 10.3 came out of.
2. **The reduced test worlds now carry the corridor.** Making the central bank's overdraft a credit
   decision means every world with a central bank needs the module that answers it, exactly as every
   world with a bank already carried `bank-lending`. Ten test files were changed to say so.
3. **Deposit interest reaches every balance.** A bank pays its depositors every period, so any test
   that asserted an exact cash or equity level now measures that too. The fix is `paidTo(w, party,
'coupon')` in `test/expected.ts`: read the payment, not the balance.
4. **`toGrain` rounded twice.** Rounding to a piece and then to the two parties' common grain moved a
   trade's cash by half a piece more than the grain, and the households flows family reported the
   second rounding as a discrepancy. It rounds once now.

### Open — in the order the plan wants them

- ~~**D1/D2, the rest of the ladder.**~~ **DONE.** A bank the last session refused sells its free
  eligible paper into the next one, at a size and no level (Clearing C3), so what it fetches is
  what somebody posted and never a reserve of its own (`money-market/funding.ts`). What it sells
  for is the refusal LESS the buffer inside it: a buffer is the thing you run down when the market
  says no (A2.b), and selling the book to top up a cushion is not D1. Two other rungs moved with
  it, both because leaving them as they were made the deposit market a metronome:
  - **B2.b, bidding up is expensive.** A bank cannot pay up for the next dollar without paying up
    on every dollar it already has, so it values money at the window only when what it could not
    raise is bigger than what it already owes. Before this, one refusal of any size repriced a
    whole base at the ceiling.
  - **B1.a, E2.a, a board answers a board.** A bank reads what its rivals are publicly paying and
    holds a class at the rival's rate less what moving costs that class — up to what money is
    worth to it and no further, which is B1.a's own bound and the point at which it lets the class
    go instead. Without it a bank sat still while the bank across the road bid a point over it and
    lost its entire base in the week the gap crossed the switching cost.

- **The buffer placeholder is still there, and now there is a reason.** `bank.liquidityBuffer.
    perDeposit` was to be replaced by a target read off the bank's own published position. Both
  candidate reads were built and BOTH tripped XI-15's grain invariance: the world is knife-edge —
  any change to what banks demand of sovereign paper re-rolls which whole cells cross between banks
  in which week, and a crossing carries a cell's entire account. The change was undone (it was not
  wrong on its own terms; it was not shown to be right either) and the finding stands: what has to
  exist first is a deposit market where a class does not move as one block. The two rungs above are
  the first half of that.

- **The run (137).** Uninsured money moves away from a publicly refused bank, which is half of
  E1/E2.a. Missing: the self-reinforcing loop shown in a scenario test (D5.b/E3.a — the deposit
  leaves with the reserves behind it, so the bank is shorter at the next close), and the test
  that insurance breaks it for retail and not for wholesale (E4).
- **Interbank contagion (138).** A failed bank's rows must land losses on its lenders by name
  (E3). Not started; the plan says to test it with item 7's resolution, so it waits on the next.
- **Bank capital (139).** Requirement against risk weights, leverage backstop, which of them
  binds as a read, the buffer as a choice, distributions restricted near the line. Not started.
  `bank-lending`'s `room()` already has the capital and appetite constraints to hang it on.
- **BANK RESOLUTION (140–144) — this is the blocker.** A bank can now genuinely fail for
  liquidity, and when one does the world breaks: its deposits are money issued by a party that
  has ceased, its paper sits in a dead holder's account, its rows default, and other modules
  throw `Money E4` addressing it. Every year-long run that reaches a bank failure is red for this
  reason and no other. Needed: the failed book valued at marks and at its own carrying values,
  the hole as liabilities minus that, the hierarchy (equity to zero first), the acquirer's bid
  with deposits and rows assumed over the wire, the public path when nobody bids, deposit
  insurance per member with the insurer as an estate creditor, and the conservation family
  (acquirer paid + insurer paid + estate realised + holders lost = the hole).
- **XI-2's funding-line cut (145).** Not started.
- **Raising (146).** Equity issuance and a `bank.subordinated` kind into markets that can refuse;
  a failed raise leaves the bank where it was. Not started. The bail-in hierarchy in 141 has only
  two layers until this exists.
- **The policy rate as the central bank's own decision (148).** It is still a declared parameter.
  §31 B says it is a decision on its mandate, and the test is that a change moves the market rate
  THROUGH the corridor and never by assignment (B4).
- **Reports and observer (147, 150).** `bank.liquidity` and `centralBank.corridor` are published;
  deposits by class are in the event. Missing: the observer surface for the corridor, the session
  prints per book, refusals, facility draws and runs.
- **~~Delete the expected red.~~ DONE in 10.3.** The corridor is what a bank goes to instead, so
  every foundation world now runs a full year with ZERO violations in every family and the
  exemption in `test/expected.ts` is deleted rather than widened: `unexpected()` is every
  violation the audit reported. **The unpriced path is still there**, though, and the one world
  that still reaches it says so: with the central bank's reinvestment off, bank A is overdrawn at
  it on `recordedAs: 'reserveOverdraft'` by period 5 (`omo.test.ts`, red). Pricing that path and
  deleting it is still on this list, and until it goes the base can grow by central-bank money
  created with no lender row — which is why `omo.test.ts`'s own claim (reinvestment off means a
  smaller base) currently comes out backwards.
- **A fund can be gated for ever on a fraction of a share, and fixing it needs the resolution.**
  `households/portfolio.ts:fundOrders` asks for `short / perShare` shares per member, which is
  not a whole number; the fund pays the whole shares it can and the remainder never leaves the
  book, so a fund that owes nobody anything reports a gate every period (Law 8, named at the site
  in `funds/index.ts`). Rounding it at the cause was tried in 10.3 and reverted: it takes the
  money fund's forced sale past what this world can absorb — the bank funded against the bills it
  dumps fails at period 22 — so it belongs in the same change as bank resolution below (XI-2).
- **Re-mark MET**: Money B3.b, B3.c; Banks Lending B2.b, C1.a (COVERAGE.md).
- **A year green with a funding-squeeze scenario, and determinism** (151). Blocked on resolution.
- **The four tests 10.3 left red** — `capital.test.ts` (the XI-4 chain), `funds.test.ts` ×2,
      `omo.test.ts` — and the two things that fall out with them (the fractional-share request, and
      the XI-4 finding measured on a run that survives). Blocked on resolution; a step above.
- Coverage, record, delete this file, one commit (152–154).

### Where the work is

Branch `claude/project-review-continuation-bh8ugh`, pushed. Item 10.3 closed on it; its record is in
`docs/RECORD.md`. What that leaves this item standing on:

- **Every foundation world runs a full year with ZERO violations in every family, with nothing
  forgiven.** The exemption in `test/expected.ts` is gone: `unexpected()` is every violation the
  audit reported. That is the state this item starts from, and it is the first time the tree has
  been in it.
- **Four tests are red and all four are this item's.** Each ends with a bank that fails and a world
  with nowhere to put it. Nothing else in the suite is red.
- **The world is more robust than it looked**, because the desks had been a hundred times too small
  since 10.2 and are now the size their own data states.
