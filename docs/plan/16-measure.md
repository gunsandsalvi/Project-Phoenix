# Item 16 — Measure

**Objective.** Part XII in full, and only now: confirm that every invariant family holds, with
tolerances that are arithmetic dust, and that one defect lights one family; read every VERIFY node
as a standing measurement in its group; run the sixteen causal chains as scenarios and record
which transmit; keep the standing observations; climb the run ladder including the fixed-point run.
A VERIFY that fails is a finding about a mechanism, inserted as a worklist item at its dependency
position. Nothing is tuned. Numbers taken before this item describe an economy that did not exist,
which is why none were taken.

**Read first.** Part XII (all); Law 7 (dust), Law 11, Law 13, Law 17; Appendix C; §45 D3 (every
number reproducible from the state); XI-15 (population resolution). Code: `audit/` (every family),
the observer surface, `tools/snapshot-reads.ts` and `tools/diff-reads.ts` from item 15, every
module's `families` contributions.

**Clauses this item meets.** Every VERIFY clause in `docs/COVERAGE.md` moves from "met as a read"
to "measured, with its finding recorded"; Part XII's invariant families, VERIFY groups, chains,
standing observations and run ladder; Observer D3, E1.

---

## The starting number this item inherits

Item 12 measured a LEVEL and did not chase it (Law 11: measuring is what comes after the recipe —
Part XIII 15 — and this is the item that measures). It is here so the first measurement has a number
to start from rather than a fresh look at an economy nobody has read yet.

### The world produces a hundred and fiftieth of the scale its seed states (item 12's finding **12-15**; `docs/RECORD.md`)

**Measured** in a rig world, seed `capital`, 120,000 people, 12 firms, period 40, from `firms.plan`:

```
firm.1  batch 1,322,910   runRate 2,417,363    capacity 376,000,000   bound: demand
firm.2  batch 34,823,497  runRate 63,646,945   capacity 324,000,000   bound: demand
firm.3  batch 3,023,152   runRate 5,447,483    capacity 388,000,000   bound: demand
```

A firm's plant lets it start 376 tonnes a period and it starts 1.3. The seed sized that plant from
what the population takes off the end of the chain — 0.0175 units a member a period, walked up
through each recipe — so the seed and the mechanisms disagree about the size of this economy by two
orders of magnitude, and what they disagree about is DEMAND: every firm is bound by what it expects
to sell, at a hundred and fiftieth of what the seed assumed it would.

**What it is not.** Not a Law 8 defect (every quantity in it is a whole count), not an audit finding
(green in every family), and not arithmetic: capacity comes to `starts × plantHeadroom` exactly, as
intended.

**What it cost when it was written.** Investment never happened: capacity is 150× the run rate, so no
firm ever had a gap, so `firms.invest` and `capital.commissioned` were empty in every seed over forty
periods. Part of that has since been placed elsewhere — worklist **11.5** found that no bank had
funding room, so no firm had a quoted rate, so `costOfCapital` was Missing and `project()` was never
reached at all. How much of the gap 11.5 closes is unknown until it is worked, and that is the first
thing to re-measure here.

**Where to look, if it survives 11.5.** The circuit, not the seed: what households are paid, what
they spend, and what that buys at the cleared price. A world whose people earn a hundred and
fiftieth of what its firms are equipped to sell them is short of wages or short of employment, and
the audit cannot see it because every flow in it has two sides and balances. It is a LEVEL, and a
level is this item's business.

### Seven more, all of them the same level seen from a different system

Each was measured while another item was being worked, each was left alone (Law 11), and each is a
LEVEL rather than a mechanism. They are here so the first measurement starts from what is already
known rather than from a fresh look.

- **No firm in this world ever wants plant, so nothing is ever built** (`12d-1`). Twelve periods with
  the plant requirement turned up four times: `capital.commissioned 0`, `capital.retired 0`, 48
  machine sessions and 0 cleared, `programme 0` at every firm — and 288 credit quotes in the same
  twelve periods, so it is NOT the funding. `project()` returns `none` at `wanted.length === 0`, and
  instrumenting every gate says it is the capacity gate, first try, every firm, every period:
  `gap=-20877525277.3 cautious=4474722.7 capacityNext=20882000000` — a farm's plant lets it run at
  **4,670 times** the rate it is sure enough of to build for. **So the mechanism is right**: a firm
  with four thousand times the plant it needs declining to buy more is Capital Programme B3 working
  as written. The test's lever cannot reach it either: the seed sizes plant from the same recipe
  figure the test turns up, so raising the requirement raises the endowment (measured at 64x, 1024x,
  4096x and 16384x: `capital.commissioned 0` at every one). Ten of the eleven reds in
  `capital.test.ts` are this one thing, they assert a mechanism that is correct and never fires, and
  they may not be edited into agreement with a world that does not invest.
- **Twelve firms, eighteen periods, one loan** (`12d-6`). Counted off `w.instruments.all()`:
  `loan 1`, against 34 interbank, 21 repo and 10 subordinated. Firms here do not borrow, so nothing
  about a firm's leverage, its cost of debt or a bank's corporate book is exercised by any test that
  builds this world. Probably the same cause; recorded separately because it is a different
  measurement and may have a different one.
- **No listed firm is ever short, so Equity D1 never fires** (`12d-9`). `decideEquity` sells shares
  only when the firm's own funding read says it is short of what it is about to have to pay, and over
  forty periods no listed firm ever is: every `equity.plan` reads `issue: 0`, and the last of them
  are BUYBACKS. The mechanism declines correctly.
- **A saver has no reason to hold a share until the issuer's first published quarter** (`12d-7`). 12c
  anchored the equity book on published accounts, and a company publishes on a fiscal calendar whose
  first report is for the first quarter opening on or after the epoch — measured across five rig
  seeds, **period 25 to 29**, with one seed publishing nothing in forty. So for the first half-year
  of every run no saver names a price for any share and the equity books have one side. The mechanism
  is not wrong (a saver that invented a figure would be holding a second set of the issuer's books),
  but a saver with NO read is not the only honest answer either: what a holder can see before the
  first report is its own basis, the market's own print and the prospectus. Whether that is a reason
  is `§46`'s question.
- **A share in this world is worth a fraction of a cent** (`12b.1-2`). At period 52 of a four-bank,
  forty-firm year: `equity.firm.11` at 0.00026 USD/share on 23,991,042,300 shares; `firm.17` at
  0.00465 on 48,375,963,805. With the price grid the same lines sit on the smallest thing that
  exists, one cent, because there is nothing below it to drift to. The grid did not break the share
  market; it made the break visible. **Two numbers are wrong together — the float and the level** —
  and the tick is NOT loosened to accommodate them: a grid widened to fit a broken price would be the
  price deciding the resolution.
- **Every issuer misses payments in every window, so every grade is the worst one** (`12a-2`).
  Measured through the assessor's own blind view at period 30 with a seven-period window:
  `treasury.us missed 1129`, `bank.a missed 3`, `firm.8 missed 24`, `firm.18 missed 24`. 12a fixed
  the measure (coverage against what an issuer TAKES IN, and a horizon rather than a count), and the
  grades are still one grade because the branch that binds is not the ratio: an issuer that cannot
  pay what falls due is what the worst grade is FOR (`§44 A2`). The grade distribution is a
  MEASUREMENT of this world and never a target; forcing a spread would be tuning the assessor to make
  the world look solvent.
- **The cell grain moves the money by more than whole people are worth** (`12d-16`).
  `resolution/cells.test.ts` runs one world at three cell grains and asks that the aggregates differ
  only by what whole people do: the cash differs by **60,119,516** against a bar of **51,247,510** —
  seventeen per cent over, on a bar that is the hours a whole person supplies over the run at the most
  any venue pays. `XI-15`'s own instruction is not to widen it: *"if the aggregates move, the
  resolution is too coarse and the finding is the resolution."* What moves more than people do is what
  an INSTITUTION does — which whole cells crossed which bank in which week — so the answer is a finer
  opening grain or a mechanism that stops a cell's whole account moving at once, and both are
  decisions with an owner.
- **The cell partition refines every period and never coarsens** (`12c.1-1`). Household cells: 16 at
  the seed, 260 at p15, 546 at p30, 844 at p45 — nineteen new a period, and `mergeCells` has never
  once been called. **And merging would reclaim nothing**: at period 45, grouped by cell key plus
  exact per-member state, 844 cells make **843 distinct groups**. They are genuinely different — a
  member who was hired has been paid and one who was not has not — so every partial event partitions
  the population a little finer and nothing ever makes two groups identical again. The representation
  degenerates towards one cell per person, which is the one thing a cell exists to avoid. `XI-15`
  gives five events that change a weight and `merge` is one of them, but a merge needs two cells that
  are the SAME. Either the state a cell carries is coarser than the register's (members of a cell
  share a bank and a cohort, so why not a balance?), or a cell needs a rule for when two nearly
  identical groups become one — and both are modelling decisions with an owner, which is why this is a
  measurement and not a performance question.

---

## Design

- **The families** (Part XII): all nine built by now (money, ownership, prices, cross-market,
  accounts, names, flows, zero-sum, units); this item adds **fault-injection tests**: a harness
  that applies one deliberate defect to a sealed world (a one-sided leg, a holding without an
  issuer, a stored value, a lost instruction, a mark that does not negate, a unit created from
  nothing) through a test-only kernel back door that exists in `test/support/` only, and asserts
  that **exactly one** family reports it with the right owner and size. A defect lighting two
  families is a finding about the families (they overlap) and is fixed by narrowing one.
- **The VERIFY registry**: `tools/spec-index.ts` already lists every VERIFY clause; each is bound
  to a **read** on the observer surface (`packages/engine/src/observe/verify.ts`: one entry per
  clause with its group — conservation, not-stored, behavioural, chain — its derivation (Observer
  D3: the reads it is computed from, shown) and its result for the run); a VERIFY with no bound
  read is reported "not measured", never green. Conservation reads are pass/fail at dust every
  period; behavioural reads are directions over whole years (Part XII: "judged on whole years");
  not-stored reads are a static check (a lint over the engine for a store named like the quantity)
  plus a runtime read that recomputes from the register and compares to what the surface showed.
- **The chains**: sixteen scenario runs (`packages/engine/scenarios/*.ts`): each applies one
  named shock through a mechanism that exists (a policy-rate decision, a commodity disruption, a
  bank's failure, an election) and asserts the direction and order of the transmission the table
  names, with the lag; a chain that does not transmit records the finding in the table's "what its
  failure means" column as the mechanism to build.
- **Standing observations**: unowned money (any non-zero is a defect at its site: a family
  finding), population resolution (the same seed at 1×, 2×, 4× cell counts: the aggregates' move is
  the recorded error bar on every represented number: `tools/resolution.ts`), cost growth (per-period
  step time over the long run: a growing series is a finding about rows never retired), refusals
  (13a's `derivatives.refused` and 13c's `freight.refused` counts by party and period), banks paying
  depositors above the policy rate (a count by bank and period, never banded), concentration and
  dispersion per sector (the dispersion of member states over time), thin markets (depth per market
  as an outcome). Each is a read on the surface with its history.
- **The run ladder**: the profile run (cost only), the season run (the working run), the long run
  with shocks (accumulation, a full set of rolls, the slow loops: housing, capital, ratings, an
  election), and the **fixed-point run** (nothing exogenous changes; where it settles exposes rules
  wrong on their own terms). All four are scripts under `packages/engine/runs/`, deterministic,
  with their reads snapshotted under `docs/measure/`.
- **Findings** become items: each is inserted in `docs/WORKLIST.md` at its dependency position with
  the mechanism it names; `docs/plan/manifest.json` gains the item and this file's count is
  unchanged (the new items have their own files).

### Files

```
packages/engine/src/observe/verify.ts, observe/observations.ts
packages/engine/test/support/faults.ts, test/families-independence.test.ts
packages/engine/scenarios/*.ts (sixteen chains), runs/{profile,season,long,fixedPoint}.ts
tools/resolution.ts, tools/verify-report.ts
docs/measure/<run>/
```

---

## Steps

- [ ] Fault-injection harness and the independence test: each of the injected defects lights exactly one family with the right owner and size; overlaps narrowed; tests
- [ ] The VERIFY registry: every VERIFY clause bound to a read with its group and derivation; unbound clauses report "not measured"; `tools/verify-report.ts` prints the table; tests
- [ ] Conservation reads at dust every period over the long run; findings recorded
- [ ] Not-stored reads: the static lint for stores named like a read quantity and the runtime recomputation check; findings recorded
- [ ] Behavioural reads over whole years; findings recorded, nothing tuned
- [ ] The sixteen chain scenarios with direction, order and lag assertions; each failure recorded with the mechanism it names
- [ ] Standing observations as surface reads with history: unowned money, resolution at 1×/2×/4× with the error bar recorded, cost growth, refusals, deposit rates above policy, concentration, thin markets
- [ ] The run ladder: profile, season, long with shocks, fixed point; all four deterministic with reads snapshotted under `docs/measure/`
- [ ] Findings inserted as worklist items at their dependency positions with their plan files and manifest rows
- [ ] The measurement surface: families, VERIFY groups, chains, observations, each with its derivation and no display-only number (Observer D3, E1)
- [ ] The level above re-measured against the world 11.5 leaves: what a member earns, what it spends and what that buys at the cleared price, party by party, with the finding it names inserted as an item rather than adjusted — and with it the seven that came from it: plant nobody wants, a world with one loan in it, a firm that is never short, a saver with no read before the first report, a share worth a hundredth of a cent, a grade distribution of one grade
- [ ] The REPRESENTATION measured and decided (`12d-16`, `12c.1-1`): the aggregates at 1×/2×/4× against the bar of what whole people do, and the cell partition that refines every period and never coarsens — each reported as a resolution finding with the two admissible answers named (a coarser per-member state, or a stated rule for when two nearly identical groups become one) and the owner's decision recorded, never widened
- [ ] Coverage final pass: every VERIFY row carries its measured status; record entry
- [ ] **`13b-3`: five equity tests moved when the world got richer, and nobody knows which change moved them.** Measured at 13b: `sells new shares when it is short and the market is dear`, `buys its own back only when the market is below its own book`, `prints near the residual the company itself published, a year in`, `has a party with a view in it once the company has published anything`, `gives the room to the higher earner first`. Three things changed underneath them in one item — banks borrow for what their own contracts will take, a desk is charged the cost of the money a line is IN rather than its own, and a desk quotes only the lines the listing drew it as a maker of. Each is right on its own terms (Law 13), and a print that moves because the world it is made in moved is a finding about the world. What is NOT established is that all five are that rather than one of them being a defect. It is a measurement and not a mechanism, which is why it is here: turn each of the three changes on alone and say which test each one moves
- [ ] Delete this file; worklist row 16 → done; commit and push

## Exit criteria

Every family holds at dust on the long run and each defect lights one; every VERIFY has a bound read
and a recorded result; every chain has a recorded verdict; every finding is an inserted item.

## Guard

Part XII ("a VERIFY that fails is a finding, never a licence to adjust a number"); Law 7 (never
widen a tolerance); Law 13 (never roll back a number); Observer E3 (measuring must not change the
model).
