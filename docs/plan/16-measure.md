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
- [ ] Coverage final pass: every VERIFY row carries its measured status; record entry
- [ ] Delete this file; worklist row 16 → done; commit and push

## Exit criteria

Every family holds at dust on the long run and each defect lights one; every VERIFY has a bound read
and a recorded result; every chain has a recorded verdict; every finding is an inserted item.

## Guard

Part XII ("a VERIFY that fails is a finding, never a licence to adjust a number"); Law 7 (never
widen a tolerance); Law 13 (never roll back a number); Observer E3 (measuring must not change the
model).
