# Item 14 — The polity

**Objective.** The fiscal and regulatory POLICY primitives get their subject: a parliament of a fixed
number of seats, a few parties each with a platform that is data, an electorate of household cells
each casting its weight from its own state and its own outlook for the platform that leaves it best
off, seats by one stated rule, a government by one stated coalition rule, and a mandate that is a
read of the parliament and the only writer of every parliament-owned policy in the register. Until
now those policies had a standing mandate declared at the seed; from here the register prints their
owner beside their value and nothing else can set them.

**Read first.** §47 Polity (all); XI-17; §30 Treasury B1, B3, C1, D4.b; §31 A3, A4; §46 (the
cell's outlook); §45 A5, B1, B2.a; XI-15 (weighted sums); Law 2 (the parameter register); Part XII
(the mean-preserving-spread test; the downturn → transfers → government → spending chain). Code:
`registry/params.ts` (the parameter register, kinds and owners), item 3's `treasury` programme,
item 4's `households` and `expectations`, 13d's cohorts and employment states.

**Clauses this item meets.** Polity A1, A2, A2.a, A2.b, A3, A4, B1, B1.a, B2, B2.a, B2.b, B3, B4,
C1, C2, C2.a, C3, C3.a, C3.b, C4, D1–D5, D3.a, E1–E4, F1–F5; XI-17; Treasury B3 (the third cause),
D4.b (the buffer's owner); Central Bank A3 (the target's owner), A4 (operational independence,
fully).

---

## The finding this item carries

### A levy that fails is recorded and then forgotten: no arrears (`12d-5`)

Measured in `estate.test.ts` at 14 periods of the rig world: **855 failed tax legs** from household
cells, and nothing else a cell posted ever failed. `treasury/index.ts:728` settles the levy and, when
it fails, adds it to `unpaid` on `treasury.receipts` — which is right (`Money E1`, `D3`: a payer that
cannot pay has not paid). But `unpaid` is a number in an event and nothing carries it: the cell does
not owe it next period, the treasury does not chase it, and the receipt is not short by it in any
account. So a tax that failed is a hole between two balance sheets that only the journal knows about.

The mechanism is **arrears** — a levy that is not paid becomes a claim the treasury holds on the
payer, ranking where the law says — and the law is this item: what a tax is, what happens when it is
not paid, and where the claim ranks in `XI-8`'s waterfall are all fiscal policy with an owner
(`Polity D1`, `D3`). It is a claim like any other: an instrument with a named creditor and a named
debtor, carried until it is paid, written off, or ranked in an estate.

---

## Design

### Sub-item 14.1 Kernel: owners and the mandate door

- Every `policy` parameter in the register already declares an `owner` (item 2's parameter
  register: `parliament | centralBank | standardSetter | constitution`); this sub-item makes the
  owner **load-bearing**: a policy owned by `parliament` can be written only through
  `params.setByMandate(values, effective: Period)` on the `MechanismContext` of the module that
  declares itself the polity (assembly grants the door to exactly one module: D5, C3.a); every other
  write path throws `Forbidden`; the seed's standing mandate is the initial value with `setBy:
'seed.standingMandate'` recorded (XI-17: "the register says so"); the observer prints the owner and
  the setter beside every policy value (D5).
- The central bank's **rate** stays the central bank's (D4, Central Bank A4); its **target** and
  mandate text are parliament-owned policies (D4, Central Bank A3): `centralBank.target.*` moves to
  owner `parliament` here.

### Module `polity`

- `requires: ['households', 'treasury', 'expectations', 'labour']`.
- **The constitution** (A1, A4, C1): three policy primitives with owner `constitution` (declared
  once; no mechanism produces them): `polity.seats` (a count), `polity.termPeriods` (placed on the
  calendar by date: the election date is a calendar cycle event), `polity.allotmentRule` (data:
  the one rule: largest remainder, or highest averages: a named rule in a dispatch table, chosen by
  the constitution row).
- **Platforms** (A2): a data file `registry/platforms.ts`: one row per party with a value for **every**
  parliament-owned policy (assembly throws `InvalidRegistry` if a platform misses one or names one
  the parliament does not own); platforms differ (A2.a: assembly refuses two identical rows); a party
  is not a ledger party (A2.b: it has no account; it is a name with a seat count).
- **The vote** (A3, B): phase `polity.election` (cycle 4, order 3) on its date: for each household
  cell, the module evaluates every platform **applied to the cell's own state at the cell's own
  outlook** (B1, B2): its expected income under that platform's tax and transfer rates (its own wage
  outlook and employment state from 13d, its own transfer eligibility), its expected prices paid
  (its own basket at its own outlook: a consumption tax rate is in the platform), what it owns and
  owes (a regulatory ratio changes its mortgage quote in its outlook only through what it has
  experienced: B1.a: it never reads the published unemployment or inflation rate: the module's
  evaluation function takes a `ParticipantView` with **no** aggregate reads: the `withoutAggregates`
  view variant from item 12's `withoutPrints` family, structurally), and picks the platform with the
  highest expected position; a cell for which every platform gives the same position **abstains**
  (B2.b: the only abstention; B2.a: no turnout, swing or drawn share); the cell casts `weight` votes
  (A3, B3: `Σ vote(xᵢ)·wᵢ` through `integrate`); turnout is a read.
- **Seats and government** (C): seats by the allotment rule (C1); the government by the coalition
  rule (C2: the largest party adds the party whose platform is nearest its own (distance over the
  policy vector in each policy's own unit, stated) until it holds a majority); a **hung parliament**
  when no such coalition exists within a stated distance (a constitution datum: the maximum
  platform distance a coalition tolerates) continues the standing mandate and is reported (C2.a).
- **The mandate** (C3, C4): the seat-weighted position of the coalition's platforms per policy; a
  journaled event `polity.mandate` with date, parties, seats and what changed (C4, E4: an event that
  causes nothing itself); written through the door with `effective = election + polity.mandateLag`
  (a constitution datum); C3.b: an audit contribution: every parliament-owned policy's value equals
  the standing mandate's, every period, exactly.
- **What it controls** (D): tax rates on named bases (D1: item 3's `tax.<base>`), transfer rates
  (D1), the treasury's buffer (D1: Treasury D4.b), the outlay programme's size and composition (D2:
  item 3's programme reads `policy.outlay.<line>`), regulatory ratios and floors (D3: item 11's
  `regulation.*`, the deposit-insurance limit, 13a's haircut floor if any), the central bank's
  target (D4); D3.a: no price, quantity or outcome is a parliament-owned policy: assembly throws if
  a platform names a parameter whose kind is not `policy` or whose owner is not `parliament`.
- **Consequences** (E): E1: the treasury programme reads the register (it already does); E2: the
  chain runs through the mechanisms; E3: measured at 16; F4: the **approval rating** is the read
  "how would the cells vote today", computed by the observer with a lag (Observer A5) and read by
  nothing.
- **B4** (mean-preserving spread changes seats): a test with two seeds of equal weighted-mean
  income and different dispersion.

### Parameters

`polity.seats`, `polity.termPeriods`, `polity.allotmentRule`, `polity.coalitionMaxDistance`,
`polity.mandateLag` (policy, owner `constitution`); `platforms` (data). No `turnout`, no `swing`,
no `loyalty`, no `bloc`, no `approval` input exist.

### Audit contributions

- `names`/`flows`: C3.b: register values equal the standing mandate (a check of one writer).
- `units`: votes cast + abstentions = Σ weights of the electorate, exactly (F5).

### Files

```
packages/engine/src/registry/params.ts (14.1: owner load-bearing; setByMandate), registry/platforms.ts
packages/engine/src/world/context.ts (the door granted to one module), world/assemble.ts (platform validation)
packages/engine/src/mechanisms/polity/{index.ts,vote.ts,seats.ts,coalition.ts,mandate.ts,approval.ts}
packages/engine/test/{param-owners,platforms,vote,abstention,seats,coalition,hung,mandate,mandate-writer,spread-changes-seats,election-chain}.test.ts
```

---

## Steps

- [ ] 14.1 Kernel: policy owners load-bearing; parliament-owned policies writable only by the mandate door granted to one module; the seed's standing mandate recorded as the setter; owner and setter printed beside every value; the central bank's target moved to the parliament; tests (D4, D5, C3.a)
- [ ] The constitution's primitives: seats, term on the calendar, allotment rule from a dispatch table, coalition distance, mandate lag; tests (A1, A4, C1)
- [ ] Platforms as data rows covering every parliament-owned policy and nothing else, differing; a party holds no account; assembly validation; tests (A2, A2.a, A2.b, D3.a)
- [ ] The vote: each cell applies each platform to its own state at its own outlook through a view with no aggregate reads; votes weight for the best; abstains when indifferent; turnout as a read; tests (A3, B1, B1.a, B2, B2.a, B2.b, B3)
- [ ] Seats by the rule; the government by the coalition rule; a hung parliament continues the standing mandate and is reported; tests (C1, C2, C2.a)
- [ ] The mandate as the seat-weighted coalition platform, journaled with subjects, written through the door at the lag; C3.b contribution; tests (C3, C3.b, C4, E4)
- [ ] What it controls: fiscal rates, transfers, buffer, outlay programme, regulatory ratios, the target; never the rate, a price, a quantity or an outcome; tests (D1–D4, D3.a)
- [ ] **Arrears** (`12d-5`): a levy that fails becomes a claim the treasury holds on the payer — an instrument with two named sides, carried until paid, written off or ranked in an estate at the place the law states; the receipt is short by it in the accounts and not only in the journal; tests (Money E1, D3; XI-8; Polity D1, D3)
- [ ] Consequences through the mechanisms only: the treasury programme reads the new numbers; a multi-year scenario shows the deficit changing through named outlays and receipts after an election; tests (E1–E3)
- [ ] Approval rating as a lagged observer read that nothing reads; tests (F4)
- [ ] B4: two seeds with equal weighted-mean income and different dispersion give different seat counts; F5: seats and mandate computed from members; tests
- [ ] Observer: parliament, platforms, votes by cohort and region, the mandate with owner beside every policy; a multi-year run with two elections green; determinism; coverage re-marked; record entry
- [ ] **`13b.1-10`: the central bank is the marginal price-setter in the sovereign book, and that is a price the polity is setting by another route.** Its open-market desk closes a gap towards a 25%-of-line target by posting a MARKET order — a quantity with no level — so in every sovereign session where it has a gap it bids at the top of the book for a quarter of the line. An order with no level is a price-taker of a price the mechanism has not yet produced (Clearing A4), and a big enough one is the marginal order that sets it. The quantity limit is real policy (Central Bank C1) and the module correctly refuses to stand in any other market, so this is NOT Appendix B's buyer of last resort — but it is more aggressive than any real open-market operation, and it belongs here because what a central bank may and may not do to a price is this item's subject, alongside its administered rate. What it needs is the level at which its own reason stops: a schedule (Clearing A2), not a quantity. Test: the sovereign session's clearing price does not move when the desk's gap is doubled at an unchanged book
- [ ] Delete this file; worklist row 14 → done; commit and push

## Exit criteria

No parliament-owned policy can be set by anything but a mandate; a cell votes from what it
experienced and can abstain; a hung parliament is a reported outcome; a change of government
reaches the deficit only through the treasury's programme.

## Guard

Polity B1.a, B2.a, C3.a, D3.a, F1–F4; Central Bank A4; XI-17 ("no policy set directly").
