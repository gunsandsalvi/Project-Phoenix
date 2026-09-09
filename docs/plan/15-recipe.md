# Item 15 — The recipe

**Objective.** The full input-output structure of the real economy — every good's inputs per unit,
plant per unit, labour per unit, energy per unit, with build lags — as versioned technology data,
replacing the foundation seed's reduced recipe. It is last, deliberately (Part XIII 15): changing
input-output relationships moves **every** quantity in the model, so it must land against a stable,
recorded measurement, and the comparison across that change is the point of it. Nothing here is a
mechanism; it is data, its adoption by firms through the mechanisms that already exist, and the
recorded comparison.

**Read first.** Goods A2, A2.a, A2.b, A2.c (the recipe as technology), B1–B5; Capital Programme A4,
F1; Law 2 (a technology primitive), Law 17 (no forecast without a falsification test); Part XII,
the run ladder and the fixed-point run; Appendix C (recount rather than adjust). Code: item 4's
`goods` recipe registry, item 10's capital kinds and build lags, item 13c's commodity grades.

**Clauses this item meets.** Goods A2 (fully, with the full recipe); Part XIII step 15.

---

## Design

- **The recipe as data** (Goods A2): `registry/recipe.ts` gains a second version: for every good in
  the seed, `inputs: [{ good, unitsPerUnit }]`, `plant: [{ capitalKind, unitsPerUnit }]`, `labour:
[{ occupation, hoursPerUnit }]`, `energy: [{ grade, unitsPerUnit }]`, `buildLag`, each declared
  `technology`; every good named appears as an output of some recipe or as a commodity produced
  from plant (Commodities Spot F1: no consumption without production or inventory: assembly throws on a dangling
  input).
- **Adoption is a mechanism that exists**: a recipe version carries an `effective` period; a firm on
  the old version adopts the new one only by investing in the plant the new recipe needs (item 10's
  decision reads the new recipe's expected return against its cost of capital); there is no instant
  switch: during the transition both versions are live for different firms, and the units family
  checks each firm against its own version.
- **The comparison** (Law 17): before the change, a **baseline** is frozen: the working-season run
  and the fixed-point run with the reduced recipe, their audit reports and every VERIFY read,
  stored under `docs/measure/baseline-reduced-recipe/` by a tool that serialises the observer's
  reads (`tools/snapshot-reads.ts`); after the change, the same runs with the full recipe; the tool
  diffs read by read and the record lists what moved, by how much, and which reads it names as
  findings (a read that moved in a direction no mechanism explains is a missing mechanism, inserted
  as an item, never a tuning).

### Parameters

The full recipe rows (technology). Deleted: the foundation's reduced-recipe rows.

### Files

```
packages/engine/src/registry/recipe.ts (version 2), seeds/foundation.ts (full seed)
tools/snapshot-reads.ts, tools/diff-reads.ts
docs/measure/baseline-reduced-recipe/, docs/measure/full-recipe/
```

---

## Steps

- [ ] Baseline frozen: season run and fixed-point run with the reduced recipe; observer reads and audit reports serialised by `tools/snapshot-reads.ts`; committed under `docs/measure/`
- [ ] The full recipe as versioned technology data with an effective period; assembly validation of every input's producer; tests
- [ ] Adoption only through investment in the plant the new recipe needs; both versions live per firm during transition; the units family per version; tests: no instant switch exists
- [ ] The same runs with the full recipe; `tools/diff-reads.ts` records what moved and how much; every unexplained move recorded as a finding naming its mechanism and inserted as an item at its dependency position
- [ ] Coverage re-marked; record entry with the comparison
- [ ] Delete this file; worklist row 15 → done; commit and push

## Exit criteria

The full recipe is live, adopted through investment, and the difference it made is recorded read by
read against a frozen baseline.

## Guard

Commodities Spot F1; Law 13 (a moved number is a finding, not a regression); Law 11 (no tuning to the
baseline).
